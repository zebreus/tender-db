//! The read side: N parallel reader connections, and the filtered queries the
//! public API (`/v1`) serves over the canonical layer.
//!
//! Every collection query is built once, from one [`Filter`], and evaluated in
//! two [`Scope`]s: the current state of everything matching (REST lists and SSE
//! snapshots) or one specific `(entity, seq)` (the SSE diff loop's
//! "did this version match?" question). That is why filtered SSE cannot drift
//! from filtered REST — there is no second copy of the predicate.

use crate::{Change, int, max_cursor, opt_int_of, opt_text, opt_text_of, t, text};
use std::sync::{Arc, Mutex};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use turso::{Connection, Value};

// --------------------------------------------------------------- connections

/// A fixed set of reader connections over one database file. Turso readers
/// parallelise with each other and with the writer under WAL; the semaphore
/// bounds how many queries are in flight, and connections are reused so the
/// per-connection pragmas are paid once.
pub struct Readers {
    database: turso::Database,
    idle: Mutex<Vec<Connection>>,
    permits: Arc<Semaphore>,
}

impl Readers {
    pub(crate) fn open(database: turso::Database, n: usize) -> turso::Result<Arc<Readers>> {
        Ok(Arc::new(Readers {
            database,
            idle: Mutex::new(Vec::with_capacity(n)),
            permits: Arc::new(Semaphore::new(n)),
        }))
    }

    /// Borrow a reader, waiting if all of them are busy.
    pub async fn get(self: &Arc<Self>) -> turso::Result<Reader> {
        let permit = self.permits.clone().acquire_owned().await.expect("semaphore is never closed");
        let idle = self.idle.lock().expect("reader pool lock").pop();
        let conn = match idle {
            Some(conn) => conn,
            None => {
                let conn = self.database.connect()?;
                for pragma in crate::PRAGMAS {
                    let mut rows = conn.query(pragma, ()).await?;
                    while rows.next().await?.is_some() {}
                }
                conn
            }
        };
        Ok(Reader { pool: self.clone(), conn: Some(conn), _permit: permit })
    }
}

/// A borrowed reader connection, returned to the pool on drop.
pub struct Reader {
    pool: Arc<Readers>,
    conn: Option<Connection>,
    _permit: OwnedSemaphorePermit,
}

impl std::ops::Deref for Reader {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        self.conn.as_ref().expect("connection is taken only in Drop")
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            // A connection returned mid-transaction must NOT re-enter the pool: an
            // open read transaction freezes a WAL snapshot that blocks every
            // checkpoint, so the WAL grows without bound (issue 53, reproduced in
            // store::checkpoint). This happens when a multi-statement snapshot
            // future — the SSE initial read's `BEGIN … COMMIT` — is cancelled
            // (client disconnects) before its `COMMIT`: the future unwinds and
            // drops this Reader with the transaction still open. Discard such a
            // connection — dropping it releases the snapshot at once — and let the
            // pool open a fresh one on the next `get`. On the (unexpected) error
            // path, discard too, conservatively. A drained autocommit read holds
            // no snapshot, so the common case returns to the pool as before.
            //
            // A std mutex, not tokio's: the lock is held only for a push, and Drop
            // cannot await.
            if conn.is_autocommit().unwrap_or(false) {
                self.pool.idle.lock().expect("reader pool lock").push(conn);
            }
        }
    }
}

// ------------------------------------------------------------------- filters

/// Whether a Tender is still open for submissions at a reference instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// A submission deadline in the future.
    Open,
    /// No submission deadline in the future — closed, awarded, or never
    /// carrying a deadline at all (award notices do not).
    Closed,
}

/// The indexed predicates shared by REST lists, SSE snapshots and SSE diffs
/// (docs/architecture.md, "SSE"). `now` is the reference instant `status`
/// compares against — passed in rather than read from the clock so a
/// subscription evaluates the same way for its whole lifetime.
#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub source: Option<String>,
    /// NUTS country prefix, e.g. `DE` — matched against the version's places.
    pub country: Option<String>,
    /// CPV prefix, e.g. `4521` — matched against the version's classifications.
    pub cpv: Option<String>,
    /// Canonical Organization id in a buyer role.
    pub buyer: Option<i64>,
    /// Canonical Organization id that won at least one Lot of the Tender.
    pub winner: Option<i64>,
    pub status: Option<Status>,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
    /// `procedure` | `registration` for Tenders; `Lot` | `LotsGroup` | `Part`
    /// for Lots; the mapping profile for Notices.
    pub kind: Option<String>,
    /// Restrict Lots to one parent Tender.
    pub tender: Option<i64>,
    pub now: i64,
}

/// Which slice of the versioned layer a collection query reads.
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    /// Current state, paginated: rows with `id > after`, ascending.
    Page { after: i64, limit: i64 },
    /// One entity at one version — the SSE diff loop's predicate probe.
    At { id: i64, seq: i64 },
}

/// Incrementally assembled SQL plus its bind parameters, kept in step.
#[derive(Default)]
struct Query {
    sql: String,
    params: Vec<Value>,
}

impl Query {
    fn push(&mut self, sql: &str, params: impl IntoIterator<Item = Value>) {
        self.sql.push_str(sql);
        self.params.extend(params);
    }

    async fn rows<T>(
        &self,
        conn: &Connection,
        build: impl Fn(&turso::Row) -> T,
    ) -> turso::Result<Vec<T>> {
        let mut rows = conn.query(&self.sql, self.params.clone()).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(build(&row));
        }
        Ok(out)
    }
}

// --------------------------------------------------------------------- rows

/// A timestamp as the canonical layer stores it: the UTC instant plus the
/// offset the source published it in, because the buyer's local wall-clock
/// deadline is the meaningful one (CONTEXT.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub utc_seconds: i64,
    pub offset_minutes: i64,
    pub has_time: bool,
}

/// A Tender in its current (or a specific) version — the `/v1/tenders` item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TenderRow {
    pub id: i64,
    pub source: String,
    pub procedure_key: Option<String>,
    pub kind: String,
    pub seq: i64,
    pub published_at: i64,
    pub dispatched_at: Option<i64>,
    pub publication_id: String,
    pub notice_subtype: Option<String>,
    pub title: Option<String>,
    pub value_cents: Option<i64>,
    pub currency: Option<String>,
    pub deadline: Option<Stamp>,
    pub lots: i64,
    /// The version's CPV codes and NUTS (place) codes — echoed so a list row
    /// shows why it matched a `cpv`/`country` filter (issue 49).
    pub cpv: Vec<String>,
    pub country: Vec<String>,
}

/// A Lot in its current (or a specific) version — the `/v1/lots` item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LotRow {
    pub id: i64,
    pub tender_id: i64,
    pub lot_key: String,
    pub kind: String,
    pub seq: i64,
    pub title: Option<String>,
    pub value_cents: Option<i64>,
    pub currency: Option<String>,
    pub deadline: Option<Stamp>,
}

/// A canonical Organization profile — the `/v1/organizations` item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationRow {
    pub id: i64,
    pub name: String,
    pub country: Option<String>,
    pub identifier_kind: Option<String>,
    pub identifier: Option<String>,
    pub provisional: bool,
    pub mentions: i64,
}

/// A Notice identity row — the `/v1/notices` item. The parsed payload lives in
/// the notice layer; this is the publication event and its import state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoticeRow {
    pub id: i64,
    pub source: String,
    pub publication_id: String,
    pub content_hash: String,
    pub profile: String,
    pub declared_version: Option<String>,
    pub member_path: String,
    pub ingested_at: i64,
    pub published_at: Option<i64>,
    pub dispatched_at: Option<i64>,
    pub parse_state: String,
}

/// One satellite value of a Tender version, flattened for the detail endpoint.
/// `lot_key` is `None` when the value is the Tender's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactRow {
    pub lot_key: Option<String>,
    pub field: String,
    pub lang: Option<String>,
    pub text: Option<String>,
    pub cents: Option<i64>,
    pub currency: Option<String>,
    pub scheme: Option<String>,
    pub code: Option<String>,
    pub stamp: Option<Stamp>,
}

/// One participation of an Organization in a Tender version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyRow {
    pub lot_key: Option<String>,
    pub role: String,
    pub organization_id: i64,
    pub organization_name: String,
}

/// One version of a Tender, traceable to the Notice that caused it (ADR-0001).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionRow {
    pub seq: i64,
    pub published_at: i64,
    pub dispatched_at: Option<i64>,
    pub publication_id: String,
    pub notice_subtype: Option<String>,
    pub caused_by_notice_id: i64,
}

/// An Organization linked from the results layer, with the linking role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultOrgRow {
    pub role: String, // winner | tenderer | subcontractor
    pub organization_id: i64,
    pub organization_name: String,
}

/// One award decision (lot result) in the Tender's current state. Results are
/// keyed by their origin notice — framework/DPS rounds accumulate, so several
/// results can name the same (round-local) lot key without being the same
/// decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LotResultRow {
    pub notice_id: i64,
    pub key: String,
    pub lot_key: Option<String>,
    pub decision: Option<String>,
    pub reason: Option<String>,
    pub awarded_cents: Option<i64>,
    pub awarded_currency: Option<String>,
    pub winners: Vec<ResultOrgRow>,
    pub statistics: Vec<(String, i64)>,
}

/// One Bid (eForms LotTender) in the Tender's current state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BidRow {
    pub notice_id: i64,
    pub key: String,
    pub lot_key: Option<String>,
    pub cents: Option<i64>,
    pub currency: Option<String>,
    pub parties: Vec<ResultOrgRow>,
}

/// One settled Contract in the Tender's current state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractRow {
    pub notice_id: i64,
    pub key: String,
    pub buyer_contract_id: Option<String>,
    pub concluded: Option<Stamp>,
    pub cents: Option<i64>,
    pub currency: Option<String>,
}

/// Everything `/v1/tenders/{id}` answers: the current state plus the evidence
/// trail behind it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TenderDetail {
    pub tender: TenderRow,
    pub texts: Vec<FactRow>,
    pub amounts: Vec<FactRow>,
    pub dates: Vec<FactRow>,
    pub classifications: Vec<FactRow>,
    pub parties: Vec<PartyRow>,
    pub lots: Vec<LotRow>,
    pub lot_results: Vec<LotResultRow>,
    pub bids: Vec<BidRow>,
    pub contracts: Vec<ContractRow>,
    pub versions: Vec<VersionRow>,
}

// ---------------------------------------------------------------- predicates

/// The version a scoped query reads: the newest one, or a named one.
fn seq_expr(scope: Scope, alias: &str, params: &mut Vec<Value>) -> String {
    match scope {
        Scope::Page { .. } => {
            format!("(SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = {alias}.id)")
        }
        Scope::At { seq, .. } => {
            params.push(Value::Integer(seq));
            "?".to_owned()
        }
    }
}

/// The version-scoped predicates, identical for Tenders and Lots — everything
/// here is evaluated against `(tender_id, seq)`, which is exactly what makes
/// one filter serve both the list and the diff.
fn version_predicates(q: &mut Query, f: &Filter) {
    if let Some(country) = &f.country {
        q.push(
            " AND EXISTS (SELECT 1 FROM tender_version_classifications c
                           WHERE c.tender_id = t.id AND c.seq = v.seq
                             AND c.scheme = 'nuts' AND c.code LIKE ?)",
            [t(format!("{country}%"))],
        );
    }
    if let Some(cpv) = &f.cpv {
        q.push(
            " AND EXISTS (SELECT 1 FROM tender_version_classifications c
                           WHERE c.tender_id = t.id AND c.seq = v.seq
                             AND c.scheme = 'cpv' AND c.code LIKE ?)",
            [t(format!("{cpv}%"))],
        );
    }
    if let Some(buyer) = f.buyer {
        q.push(
            " AND EXISTS (SELECT 1 FROM tender_version_parties p
                           WHERE p.tender_id = t.id AND p.seq = v.seq
                             AND p.organization_id = ? AND p.role LIKE '%Buyer%')",
            [Value::Integer(buyer)],
        );
    }
    if let Some(winner) = f.winner {
        q.push(
            " AND EXISTS (SELECT 1 FROM tender_version_result_winners w
                           WHERE w.tender_id = t.id AND w.seq = v.seq
                             AND w.organization_id = ?)",
            [Value::Integer(winner)],
        );
    }
    if let Some(status) = f.status {
        // "Open" is a submission deadline still in the future. A Tender that
        // never published one (award notices) is therefore Closed, which is the
        // useful reading: it cannot be bid on.
        let exists = "EXISTS (SELECT 1 FROM tender_version_dates d
                               WHERE d.tender_id = t.id AND d.seq = v.seq
                                 AND d.field = 'submission_deadline' AND d.utc_seconds > ?)";
        match status {
            Status::Open => q.push(&format!(" AND {exists}"), [Value::Integer(f.now)]),
            Status::Closed => q.push(&format!(" AND NOT {exists}"), [Value::Integer(f.now)]),
        }
    }
    for (bound, op) in [(f.min_value, ">="), (f.max_value, "<=")] {
        if let Some(cents) = bound {
            q.push(
                &format!(
                    " AND (SELECT MAX(a.cents) FROM tender_version_amounts a
                            WHERE a.tender_id = t.id AND a.seq = v.seq) {op} ?"
                ),
                [Value::Integer(cents)],
            );
        }
    }
}

/// A satellite value picked from the version, newest-deadline / highest-amount
/// first. Correlated subqueries rather than joins: a version has many of each,
/// and a join would multiply the result rows.
fn pick(table: &str, column: &str, field: Option<&str>, order: &str, lot: &str) -> String {
    let field = field.map_or(String::new(), |f| format!(" AND s.field = '{f}'"));
    format!(
        "(SELECT s.{column} FROM {table} s
           WHERE s.tender_id = t.id AND s.seq = v.seq AND {lot}{field}
           ORDER BY {order} LIMIT 1)"
    )
}

// ------------------------------------------------------------------- tenders

/// Tenders matching `filter`, in the given scope. `Scope::Page` yields the
/// current state of every match after a cursor; `Scope::At` answers whether one
/// specific version matched, which is how the SSE diff loop classifies a change.
pub async fn tenders(
    conn: &Connection,
    filter: &Filter,
    scope: Scope,
) -> turso::Result<Vec<TenderRow>> {
    let mut q = Query::default();
    let title = pick(
        "tender_version_texts",
        "value",
        Some("title"),
        // The Tender's own title wins; a lot-only title stands in for the many
        // notices that title their lots and not the procedure (v_tenders).
        "(s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC",
        "1 = 1",
    );
    let deadline = |column| {
        pick(
            "tender_version_dates",
            column,
            Some("submission_deadline"),
            "s.utc_seconds DESC",
            "1 = 1",
        )
    };
    q.push(
        &format!(
            "SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                    v.publication_id, v.notice_subtype,
                    {title},
                    (SELECT MAX(a.cents) FROM tender_version_amounts a
                      WHERE a.tender_id = t.id AND a.seq = v.seq),
                    {currency},
                    {utc}, {offset}, {has_time},
                    (SELECT COUNT(*) FROM tender_version_lots l
                      WHERE l.tender_id = t.id AND l.seq = v.seq),
                    v.dispatched_at,
                    -- The version's CPV and NUTS codes, echoed so a list row
                    -- shows why it matched a cpv/country filter (issue 49). Both
                    -- seek by (tender_id, seq) on the classifications index.
                    (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                      WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'cpv'),
                    (SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                      WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts')
               FROM tenders t
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq = ",
            currency = pick("tender_version_amounts", "currency", None, "s.cents DESC", "1 = 1"),
            utc = deadline("utc_seconds"),
            offset = deadline("offset_minutes"),
            has_time = deadline("has_time"),
        ),
        [],
    );
    let seq = seq_expr(scope, "t", &mut q.params);
    q.push(&format!("{seq} WHERE 1 = 1"), []);

    if let Some(source) = &filter.source {
        q.push(" AND t.source = ?", [t(source)]);
    }
    if let Some(kind) = &filter.kind {
        q.push(" AND t.kind = ?", [t(kind)]);
    }
    version_predicates(&mut q, filter);
    match scope {
        Scope::Page { after, limit } => q.push(
            " AND t.id > ? ORDER BY t.id LIMIT ?",
            [Value::Integer(after), Value::Integer(limit)],
        ),
        Scope::At { id, .. } => q.push(" AND t.id = ?", [Value::Integer(id)]),
    }

    q.rows(conn, |row| TenderRow {
        id: int(row, 0),
        source: text(row, 1),
        procedure_key: opt_text_of(row, 2),
        kind: text(row, 3),
        seq: int(row, 4),
        published_at: int(row, 5),
        dispatched_at: opt_int_of(row, 15),
        publication_id: text(row, 6),
        notice_subtype: opt_text_of(row, 7),
        title: opt_text_of(row, 8),
        value_cents: opt_int_of(row, 9),
        currency: opt_text_of(row, 10),
        deadline: stamp(row, 11),
        lots: int(row, 14),
        cpv: split_codes(opt_text_of(row, 16)),
        country: split_codes(opt_text_of(row, 17)),
    })
    .await
}

/// A `group_concat` result — a comma-joined code list, or `None` when the
/// version has no codes of that scheme — as a (possibly empty) `Vec`.
fn split_codes(concat: Option<String>) -> Vec<String> {
    concat
        .map(|s| s.split(',').filter(|code| !code.is_empty()).map(str::to_owned).collect())
        .unwrap_or_default()
}

/// One Tender's full current state: the version chain, the satellites, the
/// parties — everything `/v1/tenders/{id}` answers.
pub async fn tender_detail(conn: &Connection, id: i64) -> turso::Result<Option<TenderDetail>> {
    let filter = Filter::default();
    let Some(tender) = tenders(conn, &filter, Scope::Page { after: id - 1, limit: 1 })
        .await?
        .into_iter()
        .find(|t| t.id == id)
    else {
        return Ok(None);
    };
    let seq = Value::Integer(tender.seq);
    let key = (Value::Integer(id), seq.clone());

    // Every satellite reads (tender_id, seq) and resolves lot_id back to the
    // published lot key, so the response never leaks internal row ids.
    let lot_key = "(SELECT l.lot_key FROM lots l WHERE l.id = s.lot_id)";
    let fact = |columns: &str, table: &str| {
        format!(
            "SELECT {lot_key}, s.field, {columns} FROM {table} s
              WHERE s.tender_id = ? AND s.seq = ?"
        )
    };
    let mut rows = conn.query(&fact("s.lang, s.value", "tender_version_texts"), key.clone()).await?;
    let mut texts = Vec::new();
    while let Some(row) = rows.next().await? {
        texts.push(FactRow {
            lot_key: opt_text_of(&row, 0),
            field: text(&row, 1),
            lang: opt_text_of(&row, 2),
            text: opt_text_of(&row, 3),
            ..blank()
        });
    }

    let mut rows =
        conn.query(&fact("s.cents, s.currency", "tender_version_amounts"), key.clone()).await?;
    let mut amounts = Vec::new();
    while let Some(row) = rows.next().await? {
        amounts.push(FactRow {
            lot_key: opt_text_of(&row, 0),
            field: text(&row, 1),
            cents: opt_int_of(&row, 2),
            currency: opt_text_of(&row, 3),
            ..blank()
        });
    }

    let mut rows = conn
        .query(
            &fact("s.utc_seconds, s.offset_minutes, s.has_time", "tender_version_dates"),
            key.clone(),
        )
        .await?;
    let mut dates = Vec::new();
    while let Some(row) = rows.next().await? {
        dates.push(FactRow {
            lot_key: opt_text_of(&row, 0),
            field: text(&row, 1),
            stamp: stamp(&row, 2),
            ..blank()
        });
    }

    let mut rows = conn
        .query(&fact("s.scheme, s.code", "tender_version_classifications"), key.clone())
        .await?;
    let mut classifications = Vec::new();
    while let Some(row) = rows.next().await? {
        classifications.push(FactRow {
            lot_key: opt_text_of(&row, 0),
            field: text(&row, 1),
            scheme: opt_text_of(&row, 2),
            code: opt_text_of(&row, 3),
            ..blank()
        });
    }

    let mut rows = conn
        .query(
            &format!(
                "SELECT {lot_key}, s.role, s.organization_id, o.name
                   FROM tender_version_parties s
                   JOIN organizations o ON o.id = s.organization_id
                  WHERE s.tender_id = ? AND s.seq = ?"
            ),
            key.clone(),
        )
        .await?;
    let mut parties = Vec::new();
    while let Some(row) = rows.next().await? {
        parties.push(PartyRow {
            lot_key: opt_text_of(&row, 0),
            role: text(&row, 1),
            organization_id: int(&row, 2),
            organization_name: text(&row, 3),
        });
    }

    let mut rows = conn
        .query(
            "SELECT seq, published_at, dispatched_at, publication_id, notice_subtype, caused_by_notice_id
               FROM tender_versions WHERE tender_id = ? ORDER BY seq",
            (Value::Integer(id),),
        )
        .await?;
    let mut versions = Vec::new();
    while let Some(row) = rows.next().await? {
        versions.push(VersionRow {
            seq: int(&row, 0),
            published_at: int(&row, 1),
            dispatched_at: opt_int_of(&row, 2),
            publication_id: text(&row, 3),
            notice_subtype: opt_text_of(&row, 4),
            caused_by_notice_id: int(&row, 5),
        });
    }

    let lots = lots_of(conn, id).await?;
    let (lot_results, bids, contracts) = results_of(conn, id, tender.seq).await?;
    Ok(Some(TenderDetail {
        tender,
        texts,
        amounts,
        dates,
        classifications,
        parties,
        lots,
        lot_results,
        bids,
        contracts,
        versions,
    }))
}

/// The results layer at one version: every accumulated round's lot results
/// (with winners and statistics), Bids (with their consortium), and Contracts.
async fn results_of(
    conn: &Connection,
    tender_id: i64,
    seq: i64,
) -> turso::Result<(Vec<LotResultRow>, Vec<BidRow>, Vec<ContractRow>)> {
    let key = (Value::Integer(tender_id), Value::Integer(seq));
    let lot_key = "(SELECT l.lot_key FROM lots l WHERE l.id = s.lot_id)";

    let mut rows = conn
        .query(
            &format!(
                "SELECT s.lot_result_id, r.notice_id, r.result_key, {lot_key},
                        s.decision, s.reason, s.awarded_cents, s.awarded_currency
                   FROM tender_version_lot_results s
                   JOIN lot_results r ON r.id = s.lot_result_id
                  WHERE s.tender_id = ? AND s.seq = ? ORDER BY s.lot_result_id"
            ),
            key.clone(),
        )
        .await?;
    let mut lot_results = Vec::new();
    let mut result_index = std::collections::HashMap::new();
    while let Some(row) = rows.next().await? {
        result_index.insert(int(&row, 0), lot_results.len());
        lot_results.push(LotResultRow {
            notice_id: int(&row, 1),
            key: text(&row, 2),
            lot_key: opt_text_of(&row, 3),
            decision: opt_text_of(&row, 4),
            reason: opt_text_of(&row, 5),
            awarded_cents: opt_int_of(&row, 6),
            awarded_currency: opt_text_of(&row, 7),
            winners: Vec::new(),
            statistics: Vec::new(),
        });
    }
    let mut rows = conn
        .query(
            "SELECT w.lot_result_id, w.organization_id, o.name
               FROM tender_version_result_winners w
               JOIN organizations o ON o.id = w.organization_id
              WHERE w.tender_id = ? AND w.seq = ?",
            key.clone(),
        )
        .await?;
    while let Some(row) = rows.next().await? {
        if let Some(&i) = result_index.get(&int(&row, 0)) {
            lot_results[i].winners.push(ResultOrgRow {
                role: "winner".to_owned(),
                organization_id: int(&row, 1),
                organization_name: text(&row, 2),
            });
        }
    }
    let mut rows = conn
        .query(
            "SELECT lot_result_id, kind, count FROM tender_version_result_stats
              WHERE tender_id = ? AND seq = ?",
            key.clone(),
        )
        .await?;
    while let Some(row) = rows.next().await? {
        if let Some(&i) = result_index.get(&int(&row, 0)) {
            lot_results[i].statistics.push((text(&row, 1), int(&row, 2)));
        }
    }

    let mut rows = conn
        .query(
            &format!(
                "SELECT s.bid_id, b.notice_id, b.bid_key, {lot_key}, s.cents, s.currency
                   FROM tender_version_bids s
                   JOIN bids b ON b.id = s.bid_id
                  WHERE s.tender_id = ? AND s.seq = ? ORDER BY s.bid_id"
            ),
            key.clone(),
        )
        .await?;
    let mut bids = Vec::new();
    let mut bid_index = std::collections::HashMap::new();
    while let Some(row) = rows.next().await? {
        bid_index.insert(int(&row, 0), bids.len());
        bids.push(BidRow {
            notice_id: int(&row, 1),
            key: text(&row, 2),
            lot_key: opt_text_of(&row, 3),
            cents: opt_int_of(&row, 4),
            currency: opt_text_of(&row, 5),
            parties: Vec::new(),
        });
    }
    let mut rows = conn
        .query(
            "SELECT p.bid_id, p.role, p.organization_id, o.name
               FROM tender_version_bid_parties p
               JOIN organizations o ON o.id = p.organization_id
              WHERE p.tender_id = ? AND p.seq = ?",
            key.clone(),
        )
        .await?;
    while let Some(row) = rows.next().await? {
        if let Some(&i) = bid_index.get(&int(&row, 0)) {
            bids[i].parties.push(ResultOrgRow {
                role: text(&row, 1),
                organization_id: int(&row, 2),
                organization_name: text(&row, 3),
            });
        }
    }

    let mut rows = conn
        .query(
            "SELECT c.notice_id, c.contract_key, s.buyer_contract_id,
                    s.concluded_utc, s.concluded_offset, s.concluded_has_time,
                    s.cents, s.currency
               FROM tender_version_contracts s
               JOIN contracts c ON c.id = s.contract_id
              WHERE s.tender_id = ? AND s.seq = ? ORDER BY s.contract_id",
            key,
        )
        .await?;
    let mut contracts = Vec::new();
    while let Some(row) = rows.next().await? {
        contracts.push(ContractRow {
            notice_id: int(&row, 0),
            key: text(&row, 1),
            buyer_contract_id: opt_text_of(&row, 2),
            concluded: stamp(&row, 3),
            cents: opt_int_of(&row, 6),
            currency: opt_text_of(&row, 7),
        });
    }
    Ok((lot_results, bids, contracts))
}

fn blank() -> FactRow {
    FactRow {
        lot_key: None,
        field: String::new(),
        lang: None,
        text: None,
        cents: None,
        currency: None,
        scheme: None,
        code: None,
        stamp: None,
    }
}

// ---------------------------------------------------------------------- lots

/// Lots matching `filter`. The tender-level predicates apply through the
/// parent Tender's version, so `/v1/lots?country=DE` means the same thing it
/// does on `/v1/tenders`.
pub async fn lots(conn: &Connection, filter: &Filter, scope: Scope) -> turso::Result<Vec<LotRow>> {
    let mut q = Query::default();
    let lot_scope = "s.lot_id = l.id";
    let deadline = |column| {
        pick(
            "tender_version_dates",
            column,
            Some("submission_deadline"),
            "s.utc_seconds DESC",
            lot_scope,
        )
    };
    q.push(
        &format!(
            "SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq,
                    {title},
                    (SELECT MAX(a.cents) FROM tender_version_amounts a
                      WHERE a.tender_id = t.id AND a.seq = v.seq AND a.lot_id = l.id),
                    {currency},
                    {utc}, {offset}, {has_time}
               FROM lots l
               JOIN tenders t ON t.id = l.tender_id
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq = ",
            title = pick(
                "tender_version_texts",
                "value",
                Some("title"),
                "(s.lang = 'ENG') DESC",
                lot_scope
            ),
            currency = pick("tender_version_amounts", "currency", None, "s.cents DESC", lot_scope),
            utc = deadline("utc_seconds"),
            offset = deadline("offset_minutes"),
            has_time = deadline("has_time"),
        ),
        [],
    );
    let seq = match scope {
        Scope::Page { .. } => {
            "(SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)".to_owned()
        }
        Scope::At { seq, .. } => {
            q.params.push(Value::Integer(seq));
            "?".to_owned()
        }
    };
    q.push(
        &format!(
            "{seq}
               JOIN tender_version_lots vl
                 ON vl.tender_id = t.id AND vl.seq = v.seq AND vl.lot_id = l.id
              WHERE 1 = 1"
        ),
        [],
    );
    if let Some(source) = &filter.source {
        q.push(" AND t.source = ?", [t(source)]);
    }
    if let Some(kind) = &filter.kind {
        q.push(" AND vl.kind = ?", [t(kind)]);
    }
    if let Some(tender) = filter.tender {
        q.push(" AND l.tender_id = ?", [Value::Integer(tender)]);
    }
    version_predicates(&mut q, filter);
    match scope {
        Scope::Page { after, limit } => q.push(
            " AND l.id > ? ORDER BY l.id LIMIT ?",
            [Value::Integer(after), Value::Integer(limit)],
        ),
        Scope::At { id, .. } => q.push(" AND l.id = ?", [Value::Integer(id)]),
    }

    q.rows(conn, |row| LotRow {
        id: int(row, 0),
        tender_id: int(row, 1),
        lot_key: text(row, 2),
        kind: text(row, 3),
        seq: int(row, 4),
        title: opt_text_of(row, 5),
        value_cents: opt_int_of(row, 6),
        currency: opt_text_of(row, 7),
        deadline: stamp(row, 8),
    })
    .await
}

async fn lots_of(conn: &Connection, tender_id: i64) -> turso::Result<Vec<LotRow>> {
    let filter = Filter { tender: Some(tender_id), ..Filter::default() };
    lots(conn, &filter, Scope::Page { after: 0, limit: MAX_PAGE }).await
}

/// The hard ceiling on any one page — a client asking for more gets this.
pub const MAX_PAGE: i64 = 1000;

// ------------------------------------------------------------- organizations

/// Canonical Organization profiles. Only `country` and `kind` (the identifier
/// kind) narrow them — the value/CPV/status predicates are Tender-shaped and
/// have no meaning here.
pub async fn organizations(
    conn: &Connection,
    filter: &Filter,
    scope: Scope,
) -> turso::Result<Vec<OrganizationRow>> {
    let mut q = Query::default();
    q.push(
        "SELECT o.id, o.name, o.country, o.identifier_kind, o.identifier, o.provisional,
                (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id)
           FROM organizations o WHERE 1 = 1",
        [],
    );
    if let Some(country) = &filter.country {
        q.push(" AND o.country = ?", [t(country)]);
    }
    if let Some(kind) = &filter.kind {
        q.push(" AND o.identifier_kind = ?", [t(kind)]);
    }
    if let Some(buyer) = filter.buyer {
        q.push(" AND o.id = ?", [Value::Integer(buyer)]);
    }
    match scope {
        Scope::Page { after, limit } => q.push(
            " AND o.id > ? ORDER BY o.id LIMIT ?",
            [Value::Integer(after), Value::Integer(limit)],
        ),
        Scope::At { id, .. } => q.push(" AND o.id = ?", [Value::Integer(id)]),
    }
    q.rows(conn, |row| OrganizationRow {
        id: int(row, 0),
        name: text(row, 1),
        country: opt_text_of(row, 2),
        identifier_kind: opt_text_of(row, 3),
        identifier: opt_text_of(row, 4),
        provisional: int(row, 5) != 0,
        mentions: int(row, 6),
    })
    .await
}

// ------------------------------------------------------------------- notices

/// Notice identity rows. `kind` selects the mapping profile, `status` the
/// parse state — the notice layer's own vocabulary rather than the Tender's.
pub async fn notices(
    conn: &Connection,
    filter: &Filter,
    scope: Scope,
) -> turso::Result<Vec<NoticeRow>> {
    let mut q = Query::default();
    q.push(
        "SELECT id, source, publication_id, content_hash, profile, declared_version,
                member_path, ingested_at, parse_state, published_at, dispatched_at
           FROM notices WHERE 1 = 1",
        [],
    );
    if let Some(source) = &filter.source {
        q.push(" AND source = ?", [t(source)]);
    }
    if let Some(kind) = &filter.kind {
        q.push(" AND profile = ?", [t(kind)]);
    }
    match scope {
        Scope::Page { after, limit } => q.push(
            " AND id > ? ORDER BY id LIMIT ?",
            [Value::Integer(after), Value::Integer(limit)],
        ),
        Scope::At { id, .. } => q.push(" AND id = ?", [Value::Integer(id)]),
    }
    q.rows(conn, |row| NoticeRow {
        id: int(row, 0),
        source: text(row, 1),
        publication_id: text(row, 2),
        content_hash: text(row, 3),
        profile: text(row, 4),
        declared_version: opt_text_of(row, 5),
        member_path: text(row, 6),
        ingested_at: int(row, 7),
        parse_state: text(row, 8),
        published_at: opt_int_of(row, 9),
        dispatched_at: opt_int_of(row, 10),
    })
    .await
}

// ------------------------------------------------------------------- changes

/// The change log from a cursor position, optionally narrowed to one entity
/// kind — the shared query behind `/v1/changes`, the SSE diff loop and (later)
/// webhook delivery.
pub async fn changes_since(
    conn: &Connection,
    cursor: i64,
    limit: i64,
    entity: Option<&str>,
) -> turso::Result<Vec<Change>> {
    let mut rows = conn
        .query(
            "SELECT cursor, entity_kind, entity_id, version_seq, op, changed_at FROM changes
              WHERE cursor > ? AND (? IS NULL OR entity_kind = ?)
              ORDER BY cursor LIMIT ?",
            (
                Value::Integer(cursor),
                opt_text(entity),
                opt_text(entity),
                Value::Integer(limit),
            ),
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(Change {
            cursor: int(&row, 0),
            entity_kind: text(&row, 1),
            entity_id: int(&row, 2),
            version_seq: opt_int_of(&row, 3),
            op: text(&row, 4),
            changed_at: int(&row, 5),
        });
    }
    Ok(out)
}

/// The oldest cursor still in the log. A resume request below it cannot be
/// served incrementally and gets an SSE `reset` instead (Firestore's expired-
/// token semantics, docs/research/api-layer.md §3).
pub async fn oldest_cursor(conn: &Connection) -> turso::Result<i64> {
    let mut rows = conn.query("SELECT COALESCE(MIN(cursor), 0) FROM changes", ()).await?;
    Ok(rows.next().await?.map_or(0, |row| int(&row, 0)))
}

/// The newest cursor — the snapshot boundary `N` and the `/health` liveness
/// probe in one query.
pub async fn latest_cursor(conn: &Connection) -> turso::Result<i64> {
    max_cursor(conn).await
}

fn stamp(row: &turso::Row, idx: usize) -> Option<Stamp> {
    Some(Stamp {
        utc_seconds: opt_int_of(row, idx)?,
        offset_minutes: opt_int_of(row, idx + 1).unwrap_or(0),
        has_time: opt_int_of(row, idx + 2).unwrap_or(0) != 0,
    })
}
