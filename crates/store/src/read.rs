//! The read side: N parallel reader connections, and the filtered queries the
//! public API (`/v1`) serves over the canonical layer.
//!
//! Every collection query is built once, from one [`Filter`], and evaluated in
//! two [`Scope`]s: the current state of everything matching (REST lists and SSE
//! snapshots) or one specific `(entity, seq)` (the SSE diff loop's
//! "did this version match?" question). That is why filtered SSE cannot drift
//! from filtered REST — there is no second copy of the predicate.

use crate::{Change, int, max_cursor, opt_int_of, opt_text_of, t, text};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{OwnedRwLockReadGuard, OwnedSemaphorePermit, RwLock, Semaphore};
use turso::{Connection, Value};

/// A process-wide gate that lets a bulk-load checkpoint briefly EXCLUDE readers
/// so a WAL TRUNCATE gets a reader-free instant to wrap on a live box (issue 63).
/// Every pooled read holds the SHARED side for its borrow; a gated checkpoint
/// takes the EXCLUSIVE side. `None` unless `TENDER_WAL_READ_GATE` is set at open,
/// so the default path constructs and acquires nothing — zero overhead when off.
pub type WalGate = Option<Arc<RwLock<()>>>;

/// The `changes.entity_kind` values the projection ever emits. `changes_since`
/// short-circuits an unknown kind to an empty result (issue 61 finding 2) — a kind
/// with no rows must never trigger a table walk to discover it has none.
const ENTITY_KINDS: [&str; 6] = ["tender", "lot", "organization", "lot_result", "bid", "contract"];

// --------------------------------------------------------------- connections

/// A fixed set of reader connections over one database file. Turso readers
/// parallelise with each other and with the writer under WAL; the semaphore
/// bounds how many queries are in flight, and connections are reused so the
/// per-connection pragmas are paid once.
pub struct Readers {
    database: turso::Database,
    idle: Mutex<Vec<Connection>>,
    permits: Arc<Semaphore>,
    gate: WalGate,
}

impl Readers {
    pub(crate) fn open(database: turso::Database, n: usize, gate: WalGate) -> turso::Result<Arc<Readers>> {
        Ok(Arc::new(Readers {
            database,
            idle: Mutex::new(Vec::with_capacity(n)),
            permits: Arc::new(Semaphore::new(n)),
            gate,
        }))
    }

    /// Borrow a reader, waiting if all of them are busy.
    pub async fn get(self: &Arc<Self>) -> turso::Result<Reader> {
        let permit = self.permits.clone().acquire_owned().await.expect("semaphore is never closed");
        // Hold the WAL gate's SHARED side for the borrow's lifetime so a gated
        // checkpoint (issue 63) can take the EXCLUSIVE side and get a reader-free
        // instant to wrap the WAL. Acquired BEFORE taking a connection, so a
        // pending exclusive holder is never made to wait on a live read snapshot.
        // `None` (gate disabled) acquires nothing. Deadlock-free: readers never
        // take the writer lock, so the exclusive holder only ever waits on reads
        // that are draining, never on itself.
        let gate = match &self.gate {
            Some(g) => Some(g.clone().read_owned().await),
            None => None,
        };
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
        Ok(Reader { pool: self.clone(), conn: Some(conn), _permit: permit, _gate: gate })
    }
}

/// A borrowed reader connection, returned to the pool on drop.
pub struct Reader {
    pool: Arc<Readers>,
    conn: Option<Connection>,
    _permit: OwnedSemaphorePermit,
    /// The WAL-gate shared lease (issue 63), held for the borrow's lifetime so a
    /// gated checkpoint's exclusive acquire waits out this read. `None` when the
    /// gate is disabled. Dropped with the Reader, releasing the lease.
    _gate: Option<OwnedRwLockReadGuard<()>>,
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
    let q = tenders_query(filter, scope);
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

/// The statement [`tenders`] builds, without running it — the seam 112's plan gate
/// reads so it asserts the artifact rather than a paraphrase of it (issue 114).
///
/// That closes the DRIFT gap only. Asserting the artifact's PLAN still cannot see
/// the cost class: a plan names the access path, never the number of rows on it
/// (112 rule 6). Measured — 117's row-value fix turns a walk into a seek, the
/// transition a plan gate rewards, while getting 151,648x slower on prod's DE
/// slice. This seam narrows what the gate can be wrong about; it does not widen
/// what the gate can see.
#[cfg(test)]
pub(crate) fn tenders_statement(filter: &Filter, scope: Scope) -> (String, Vec<Value>) {
    let q = tenders_query(filter, scope);
    (q.sql, q.params)
}

/// The identity half of [`tenders`], built but not run.
fn tenders_query(filter: &Filter, scope: Scope) -> Query {
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
    q
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
    let mut rows = lots_query(filter, scope)
        .rows(conn, |row| LotRow {
            id: int(row, 0),
            tender_id: int(row, 1),
            lot_key: text(row, 2),
            kind: text(row, 3),
            seq: int(row, 4),
            title: None,
            value_cents: None,
            currency: None,
            deadline: None,
        })
        .await?;
    summarise(conn, &mut rows).await?;
    Ok(rows)
}

/// The SQL and bind parameters [`lots`] would run, without running them — the seam
/// a plan test needs to assert the access path of the statement the builder ACTUALLY
/// emits.
///
/// A plan test that plans its own hand-written string keeps passing while the builder
/// drifts underneath it: the same artifact-versus-proxy failure as issues 110 and
/// 102, and one this read has already been bitten by. The test guarding `1830d50`'s
/// row-value cursor spelled that cursor out in its own SQL, so it went on passing
/// after issue 115 removed the form from the builder — certifying a statement nothing
/// emitted.
///
/// Test-only, because it is a window onto the builder rather than a way to use it:
/// nothing in production wants the statement without running it. What it exposes is
/// [`lots_query`], the same production code [`lots`] itself runs — a view, not a
/// second path.
#[cfg(test)]
pub(crate) fn lots_statement(filter: &Filter, scope: Scope) -> (String, Vec<Value>) {
    let q = lots_query(filter, scope);
    (q.sql, q.params)
}

/// The identity half of [`lots`], built but not run.
fn lots_query(filter: &Filter, scope: Scope) -> Query {
    let mut q = Query::default();
    // Two questions, two driving tables — and that is the point, not an
    // optimisation. "Which Lots does THIS Tender's current version publish?" is a
    // containment question, and the set it asks for is literally the rows of
    // `tender_version_lots` under one `(tender_id, seq)`; it drives from there.
    // "The next page of the lot stream after this cursor" is a stream question, and
    // it drives from `lots` in id order. Answering the first with the second's
    // machinery is what made `/v1/tenders/{id}` cost minutes.
    //
    // Only `Scope::Page` splits: `Scope::At` probes one lot by id for the SSE diff
    // loop, where the id is already the whole answer.
    let scoped = match scope {
        Scope::Page { .. } => filter.tender,
        Scope::At { .. } => None,
    };
    match scoped {
        // The containment shape. `vl.tender_id = ?` seeks the PK's leading column
        // and the MAX(seq) subquery is uncorrelated, so it is evaluated once.
        //
        // Driving this from `lots` instead — a per-lot `vl.lot_id = l.id` probe —
        // is the second half of issue 115's blow-up. turso resolves that probe by
        // seeking the `(tender_id, seq)` PK prefix and then WALKING that version's
        // whole slice: it does not use the third PK column, and adding an explicit
        // `(tender_id, seq, lot_id)` index does not change that (measured —
        // identical cost with and without). One walk per lot is quadratic, so the
        // join alone cost 0.96s at 2,400 lots against 0.005s here (178x).
        //
        // `l.tender_id = vl.tender_id` is not redundant: it is the containment
        // guard the old shape got from driving off `lots`, and it keeps a
        // cross-Tender `vl.lot_id` (issue 103's orphaned rows) out of the answer.
        Some(tender) => {
            q.push(
                "SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq
                   FROM tender_version_lots vl
                   JOIN lots l ON l.id = vl.lot_id AND l.tender_id = vl.tender_id
                   JOIN tenders t ON t.id = vl.tender_id
                   JOIN tender_versions v ON v.tender_id = vl.tender_id AND v.seq = vl.seq
                  WHERE vl.tender_id = ?
                    AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x
                                   WHERE x.tender_id = ?)",
                [Value::Integer(tender), Value::Integer(tender)],
            );
        }
        // The stream shape, unchanged: driving from `lots` in rowid order is the
        // right plan for a global id-ordered page.
        None => {
            q.push(
                "SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq
                   FROM lots l
                   JOIN tenders t ON t.id = l.tender_id
                   JOIN tender_versions v ON v.tender_id = t.id AND v.seq = ",
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
        }
    }
    if let Some(source) = &filter.source {
        q.push(" AND t.source = ?", [t(source)]);
    }
    if let Some(kind) = &filter.kind {
        q.push(" AND vl.kind = ?", [t(kind)]);
    }
    if scoped.is_none()
        && let Some(tender) = filter.tender
    {
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
    q
}

/// Fill in each Lot's summary fields from the version satellites: the title, the
/// value and its currency, the submission deadline.
///
/// These used to be six correlated scalar subqueries on the row query above — one
/// set per lot. The satellites are indexed on `(tender_id, seq)` and nothing else;
/// `lot_id` appears in no index. So each subquery seeked the version and then
/// walked that version's WHOLE satellite slice to find one lot's rows. One walk per
/// lot per subquery is O(lots × slice), i.e. QUADRATIC in the Tender's lot count —
/// measured at 16.1× the time for 4× the lots (issue 115). The 4.26M Tenders with a
/// handful of lots never noticed; the 16 with more than a thousand took minutes per
/// request.
///
/// So read each satellite ONCE per version instead — the same
/// `WHERE tender_id = ? AND seq = ?` slice `tender_detail` already reads for the
/// response's own `texts`/`amounts`/`dates` — and do the per-lot pick in memory.
/// Cost is O(slice) per distinct version, independent of how many lots were asked
/// for: a whole-Tender read is three queries whether it has 2 lots or 2,604. A page
/// of the global list, whose lots span many versions, does three queries per version
/// against the old six per lot, so it gets cheaper too.
///
/// The picks are the SQL's, exactly:
///   * title — `ORDER BY (lang = 'ENG') DESC LIMIT 1`. SQLite sorts NULL below both
///     0 and 1 under DESC, so the preference is ENG, then any other language, then
///     an unlabelled row; ties keep the first in scan order.
///   * value and currency — `MAX(cents)` and `ORDER BY cents DESC LIMIT 1` resolve
///     to the SAME row, so one max-cents row serves both.
///   * deadline — the three columns were three subqueries sharing
///     `ORDER BY utc_seconds DESC LIMIT 1`, so one max-utc row serves all three.
async fn summarise(conn: &Connection, rows: &mut [LotRow]) -> turso::Result<()> {
    // Lot ids are unique across the result, so one map resolves a satellite row's
    // `lot_id` to the row it decorates — and drops any lot outside this page.
    // Keyed on the WHOLE of what the correlated subquery matched on —
    // `s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id` — not on `lot_id`
    // alone. A satellite row belongs to a lot only if it also belongs to that lot's
    // Tender AND version. Keying on `lot_id` by itself lets one Tender's slice
    // decorate another Tender's lot whenever a satellite row carries a foreign
    // `lot_id` (the issue-103 orphan shape, in the satellites rather than in
    // `tender_version_lots`) — a leak the old per-lot subqueries could not produce,
    // because their `s.tender_id = t.id` never matched.
    let at: HashMap<(i64, i64, i64), usize> =
        rows.iter().enumerate().map(|(i, r)| ((r.tender_id, r.seq, r.id), i)).collect();
    let mut versions: Vec<(i64, i64)> = rows.iter().map(|r| (r.tender_id, r.seq)).collect();
    versions.sort_unstable();
    versions.dedup();

    // Best key seen so far per row, `None` until the first candidate — kept beside
    // the rows rather than in them because it is the ORDER BY's key, not output.
    let mut best_title: Vec<Option<u8>> = vec![None; rows.len()];
    let mut best_value: Vec<Option<i64>> = vec![None; rows.len()];
    let mut best_deadline: Vec<Option<i64>> = vec![None; rows.len()];

    for (tender_id, seq) in versions {
        let key = (Value::Integer(tender_id), Value::Integer(seq));

        let mut got = conn
            .query(
                "SELECT s.lot_id, s.lang, s.value FROM tender_version_texts s
                  WHERE s.tender_id = ? AND s.seq = ? AND s.field = 'title'
                    AND s.lot_id IS NOT NULL",
                key.clone(),
            )
            .await?;
        while let Some(row) = got.next().await? {
            let Some(&i) = opt_int_of(&row, 0).and_then(|id| at.get(&(tender_id, seq, id))) else { continue };
            let rank = match opt_text_of(&row, 1).as_deref() {
                Some("ENG") => 2,
                Some(_) => 1,
                None => 0,
            };
            if best_title[i].is_none_or(|best| rank > best) {
                best_title[i] = Some(rank);
                rows[i].title = opt_text_of(&row, 2);
            }
        }

        let mut got = conn
            .query(
                "SELECT s.lot_id, s.cents, s.currency FROM tender_version_amounts s
                  WHERE s.tender_id = ? AND s.seq = ? AND s.lot_id IS NOT NULL",
                key.clone(),
            )
            .await?;
        while let Some(row) = got.next().await? {
            let Some(&i) = opt_int_of(&row, 0).and_then(|id| at.get(&(tender_id, seq, id))) else { continue };
            let cents = opt_int_of(&row, 1);
            // A NULL sorts last under `cents DESC` and is ignored by `MAX`, so it
            // ranks below every real amount rather than above them.
            let rank = cents.unwrap_or(i64::MIN);
            if best_value[i].is_none_or(|best| rank > best) {
                best_value[i] = Some(rank);
                rows[i].value_cents = cents;
                rows[i].currency = opt_text_of(&row, 2);
            }
        }

        let mut got = conn
            .query(
                "SELECT s.lot_id, s.utc_seconds, s.offset_minutes, s.has_time
                   FROM tender_version_dates s
                  WHERE s.tender_id = ? AND s.seq = ? AND s.field = 'submission_deadline'
                    AND s.lot_id IS NOT NULL",
                key,
            )
            .await?;
        while let Some(row) = got.next().await? {
            let Some(&i) = opt_int_of(&row, 0).and_then(|id| at.get(&(tender_id, seq, id))) else { continue };
            let rank = opt_int_of(&row, 1).unwrap_or(i64::MIN);
            if best_deadline[i].is_none_or(|best| rank > best) {
                best_deadline[i] = Some(rank);
                rows[i].deadline = stamp(&row, 1);
            }
        }
    }
    Ok(())
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
    let q = organizations_query(filter, scope);
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

/// The statement [`organizations`] builds, without running it — the seam 112's plan gate
/// reads so it asserts the artifact rather than a paraphrase of it (issue 114).
///
/// That closes the DRIFT gap only. Asserting the artifact's PLAN still cannot see
/// the cost class: a plan names the access path, never the number of rows on it
/// (112 rule 6). Measured — 117's row-value fix turns a walk into a seek, the
/// transition a plan gate rewards, while getting 151,648x slower on prod's DE
/// slice. This seam narrows what the gate can be wrong about; it does not widen
/// what the gate can see.
#[cfg(test)]
pub(crate) fn organizations_statement(filter: &Filter, scope: Scope) -> (String, Vec<Value>) {
    let q = organizations_query(filter, scope);
    (q.sql, q.params)
}

/// The identity half of [`organizations`], built but not run.
fn organizations_query(filter: &Filter, scope: Scope) -> Query {
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
    }    q
}

// ------------------------------------------------------------------- notices

/// Notice identity rows. `kind` selects the mapping profile, `status` the
/// parse state — the notice layer's own vocabulary rather than the Tender's.
pub async fn notices(
    conn: &Connection,
    filter: &Filter,
    scope: Scope,
) -> turso::Result<Vec<NoticeRow>> {
    let q = notices_query(filter, scope);
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

/// The statement [`notices`] builds, without running it — the seam 112's plan gate
/// reads so it asserts the artifact rather than a paraphrase of it (issue 114).
///
/// That closes the DRIFT gap only. Asserting the artifact's PLAN still cannot see
/// the cost class: a plan names the access path, never the number of rows on it
/// (112 rule 6). Measured — 117's row-value fix turns a walk into a seek, the
/// transition a plan gate rewards, while getting 151,648x slower on prod's DE
/// slice. This seam narrows what the gate can be wrong about; it does not widen
/// what the gate can see.
#[cfg(test)]
pub(crate) fn notices_statement(filter: &Filter, scope: Scope) -> (String, Vec<Value>) {
    let q = notices_query(filter, scope);
    (q.sql, q.params)
}

/// The identity half of [`notices`], built but not run.
fn notices_query(filter: &Filter, scope: Scope) -> Query {
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
    }    q
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
    // Two query shapes, each planner-clean (issue 61 finding 2):
    //  * no filter → `cursor > ? ORDER BY cursor` seeks the cursor PK and walks
    //    forward `limit` rows — bounded.
    //  * entity filter → `entity_kind = ? AND cursor > ? ORDER BY cursor` seeks the
    //    `changes_entity_cursor(entity_kind, cursor)` index directly and walks that
    //    kind's rows in cursor order. WITHOUT that index (or the old
    //    `(? IS NULL OR entity_kind = ?)` disjunction that defeats it) a rare or
    //    NONEXISTENT kind with `since=0` walked the whole 80M-row table to collect
    //    `limit` matches — a public-endpoint wedge (`/v1/changes?entity=x&since=0`).
    // Guard: an entity_kind the schema never emits has zero rows by definition, so
    // return empty WITHOUT touching the table — closes the wedge even before the
    // (lazily built) index exists. Behaviour-identical: a nonexistent kind always
    // yielded empty, just after a full scan.
    if let Some(kind) = entity
        && !ENTITY_KINDS.contains(&kind)
    {
        return Ok(Vec::new());
    }
    let mut rows = match entity {
        Some(kind) => {
            conn.query(
                "SELECT cursor, entity_kind, entity_id, version_seq, op, changed_at FROM changes
                  WHERE entity_kind = ? AND cursor > ? ORDER BY cursor LIMIT ?",
                (Value::Text(kind.to_owned()), Value::Integer(cursor), Value::Integer(limit)),
            )
            .await?
        }
        None => {
            conn.query(
                "SELECT cursor, entity_kind, entity_id, version_seq, op, changed_at FROM changes
                  WHERE cursor > ? ORDER BY cursor LIMIT ?",
                (Value::Integer(cursor), Value::Integer(limit)),
            )
            .await?
        }
    };
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
    // O(1). This runs on the SSE RESUME path (every reconnect-with-Last-Event-ID),
    // so a cold scan here starves the HTTP runtime exactly like the other issue-61
    // instances. turso 0.7 short-circuits NEITHER `MIN(cursor)` NOR `... ORDER BY
    // cursor LIMIT 1` — both FULL-SCAN the 80M-row changes table (verified: identical
    // timing over 60k rows). The only O(1) answer comes from the log's invariant: the
    // change log is APPEND-ONLY and never trimmed (ADR-0001; there is no
    // `DELETE FROM changes` anywhere), and `cursor` is AUTOINCREMENT from 1, so the
    // oldest surviving cursor is 1 the instant the log is non-empty, else 0.
    // Non-empty is the O(1) sqlite_sequence high-water (`max_cursor`).
    //
    // ⚠️ If a future cursor-expiry / changes-trim path is added (CONTEXT.md hints at
    // one), it MUST maintain a real oldest watermark here — this returns 1 for any
    // non-empty log and would otherwise under-report a trimmed floor.
    Ok(i64::from(max_cursor(conn).await? > 0))
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
