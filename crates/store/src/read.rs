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
                for pragma in crate::PRAGMAS.iter().map(|p| p.to_string()).chain([crate::cache_pragma()]) {
                    let mut rows = conn.query(&pragma, ()).await?;
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
            // store::checkpoint). This happens when a future holding an explicit
            // `BEGIN … COMMIT` is cancelled before its `COMMIT`: the future
            // unwinds and drops this Reader with the transaction still open. (The
            // original offender, the SSE snapshot's whole-collection read, is gone
            // — issue 55 made it autocommit pages — but the net stays: any future
            // multi-statement read that is cancelled lands here.) Discard such a
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
    /// Canonical Organization id that submitted a bid (role `tenderer`) on the
    /// Tender — a superset of `winner` (issue 217).
    pub bidder: Option<i64>,
    pub status: Option<Status>,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
    /// Exact-match a PUBLISHED currency code (ISO-4217 uppercase, e.g. `EUR`) on
    /// any amount row of the current version (ADR-0014 D5). Published, not
    /// normalized: `HRK` finds the tenders that were published in kuna, however
    /// they convert. Tenders/Lots; the other collections name it ignored.
    pub currency: Option<String>,
    /// Preferred language for the PICKED text values (ADR-0013 D3): ISO
    /// 639-2/T uppercase (`DEU`), normalized by the caller. A PROJECTION
    /// selector, not a predicate — it changes which title a row serves, never
    /// which rows match — so it is not in `honoured_params`, never appears in
    /// `ignored_filters`, and never affects isolation routing. The chain the
    /// picks implement: requested → ENG → any labelled → unlabelled.
    pub lang: Option<String>,
    /// `procedure` | `registration` for Tenders; `Lot` | `LotsGroup` | `Part`
    /// for Lots; the mapping profile for Notices.
    pub kind: Option<String>,
    /// Restrict Lots to one parent Tender.
    pub tender: Option<i64>,
    /// Exact-match a Notice's official publication number (`publication_id`) on
    /// `/v1/notices` (issue 217). Not a Tender/Lot/Org predicate — those collections
    /// name it ignored rather than applying it.
    pub publication_id: Option<String>,
    /// Exact-match an Organization's official identifier VALUE (e.g. a VAT number) on
    /// `/v1/organizations` (issue 217). Pair with `kind` (the identifier scheme) to
    /// disambiguate a value reused across schemes. An Organizations-only predicate —
    /// the other collections name it ignored rather than applying it.
    pub identifier: Option<String>,
    /// Publication-date bounds on the CURRENT version (`t.current_published_at`,
    /// inclusive after / exclusive before — issue 216). Tenders-only; the other
    /// collections name them ignored. Applied by `tenders_query` in id order (which
    /// walks for a narrow range, so `walks()` isolates it) and by
    /// [`tenders_by_published`] as the index range it rides (the fast path).
    pub published_after: Option<i64>,
    pub published_before: Option<i64>,
    /// Submission-deadline bounds on the current version (`t.current_deadline`,
    /// inclusive after / exclusive before — issue 216, deadline half). Same
    /// contract as the published pair: Tenders-only, id-ordered application
    /// isolates, [`tenders_ordered`] rides `tenders_current_deadline`.
    pub deadline_after: Option<i64>,
    pub deadline_before: Option<i64>,
    /// Case-insensitive organization name prefix (issue 217-B), ALREADY
    /// Unicode-lowercased by the caller — matched against `o.name_norm`.
    /// Organizations-only; the other collections name it ignored. The id-ordered
    /// application here isolates (a sparse prefix filters the PK walk);
    /// [`organizations_by_name`] is the fast name-ordered path.
    pub name_prefix: Option<String>,
    pub now: i64,
    /// Set by the async entry points (never by callers) when the cardinality
    /// probe says the country prefix is sparse enough to DRIVE the read from
    /// `tender_version_classifications` instead of testing an EXISTS per
    /// deadline-range candidate (issue 273 step 2). The seed is a superset —
    /// any version of the tender matched — and the untouched head-version
    /// EXISTS still decides membership, exactly like the issue-223
    /// participation seeds.
    pub country_seed: bool,
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
    /// The version's original language (ADR-0013 D3), `None` where the era never said.
    pub original_lang: Option<String>,
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
    /// The lot's own submission deadline where it publishes one, otherwise the
    /// PROCEDURE's (issue 389 unit 2). Lot-scoped wins even when the procedure's
    /// is later, because scope is the question and recency is not.
    ///
    /// Inherited rather than lot-only because `?status=open` is decided by the
    /// union of both scopes (`version_predicates`' EXISTS carries no `lot_id`
    /// term — issue 275 pins that form), so a lot-only field returned rows
    /// asserted to be open whose only visible deadline was `null`. Which scope a
    /// served date came from is not on the row yet; the tender detail's `dates`
    /// array names each date's `lot`, and a provenance marker for the rows
    /// themselves is issue 370's open unit, to land on Tenders and Lots together.
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

/// Why a notice's payload is held out of the canonical layer (issue 218). A
/// quarantined notice has NO parsed satellites — unrecognised content is held
/// whole, never partially imported — so the quarantine row IS its only content,
/// reachable until now only through `/v1/sql`. `reason`/`detail` are the CURRENT
/// hold cause; `first_reason`/`first_detail` the original, preserved when a failed
/// re-attempt overwrites the pair (issue 87). The terminal stamps distinguish the
/// three outcomes a held member can reach: still outstanding, reclaimed, or
/// skipped-by-policy (issue 84).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuarantineRow {
    pub reason: String,
    pub detail: Option<String>,
    pub profile: Option<String>,
    pub first_seen: i64,
    pub attempts: Option<i64>,
    pub last_attempt_at: Option<i64>,
    pub reprocessed_at: Option<i64>,
    pub skipped_at: Option<i64>,
    pub skipped_reason: Option<String>,
    pub first_reason: Option<String>,
    pub first_detail: Option<String>,
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
    /// Issue 372: `'withheld'` when the notice declared this field suppressed
    /// under BT-195/`FieldsPrivacy`, so `cents` is the eForms SDK's -1
    /// placeholder and not a figure. Amounts only, so far.
    pub quality: Option<String>,
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
    /// The causing notice's original language (ADR-0013 D3), or `None` where the
    /// era did not publish one.
    pub original_lang: Option<String>,
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
    /// When the buyer decided — the legacy eras' award-block date (issue 255).
    pub decided: Option<Stamp>,
    pub winners: Vec<ResultOrgRow>,
    /// `(kind, count, quality)` — a `quality` of `'withheld'` means the notice
    /// declared BT-759/BT-760 suppressed, so neither value is a reading (372 u4).
    pub statistics: Vec<(String, i64, Option<String>)>,
}

/// One Bid (eForms LotTender) in the Tender's current state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BidRow {
    pub notice_id: i64,
    pub key: String,
    pub lot_key: Option<String>,
    pub cents: Option<i64>,
    pub currency: Option<String>,
    /// Issue 372: `'withheld'` when the notice declared BT-720 suppressed for
    /// this bid — the value buyers withhold most often.
    pub quality: Option<String>,
    pub parties: Vec<ResultOrgRow>,
}

/// One settled Contract in the Tender's current state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractRow {
    pub notice_id: i64,
    pub key: String,
    pub buyer_contract_id: Option<String>,
    pub concluded: Option<Stamp>,
    /// BT-1451: when the buyer decided, as distinct from when the contract was
    /// signed (issue 255).
    pub decided: Option<Stamp>,
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
/// Which collection a [`Filter`] is being applied to.
///
/// Isolation routing cannot be a property of the `Filter` alone, because the same
/// field means different things per collection and whether an index serves it differs
/// with the meaning: `kind` is `t.kind` for Tenders, `vl.kind` — a JOINED table — for
/// Lots, `identifier_kind` for Organizations and `profile` for Notices. Three of those
/// are index-served and one is not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Collection {
    Tenders,
    Lots,
    Organizations,
    Notices,
}

impl Collection {
    /// The query parameters this collection's list read actually applies to the rows
    /// it returns.
    ///
    /// The filter vocabulary is shared across all four collections by design (a
    /// subscription is a collection query plus its filters), so `Params` accepts every
    /// parameter on every path. But each builder reads only the fields meaningful to
    /// it: `organizations_query` has no CPV to match against, `notices_query` narrows
    /// only by source and profile. A parameter outside this set is accepted and then
    /// changes nothing — an unfiltered answer that looks filtered, the "confidently
    /// wrong count" this project cares about most. The list handler diffs the request's
    /// parameters against this set and echoes the leftovers as `ignored_filters` so the
    /// response says what it did (issue 118).
    ///
    /// This is the ONE authoritative list. `honoured_params_match_the_emitted_sql`
    /// checks it against the SQL each builder actually emits — byte-comparing the
    /// statement with each parameter set against the statement without it — so if a
    /// builder starts or stops reading a field this set is wrong until it is updated,
    /// and `ignored_filters` cannot drift into lying about what applied. The names are
    /// the client-facing spelling (`min_value`, not the `Filter` field), because they
    /// are echoed to the client verbatim.
    pub fn honoured_params(self) -> &'static [&'static str] {
        match self {
            // `tenders_query` reads `source`, `kind` and the published bounds directly,
            // `publication_id` as the seeded FROM (issue 217-A, `tender_from`) and the
            // rest through `version_predicates`; only the Lot-containment `tender` has
            // no meaning.
            Collection::Tenders => &[
                "source", "country", "cpv", "buyer", "winner", "bidder", "status",
                "min_value", "max_value", "currency", "kind", "publication_id",
                "published_after", "published_before", "deadline_after", "deadline_before",
            ],
            // `lots_query` adds the `tender` containment shape (issue 115) to the same
            // version predicates, so the whole vocabulary applies here.
            Collection::Lots => &[
                "source", "country", "cpv", "buyer", "winner", "bidder", "status",
                "min_value", "max_value", "currency", "kind", "tender",
            ],
            // `organizations_query`: the identity-shaped predicates. `identifier` is
            // the official id value (issue 217), paired with `kind` for the scheme. The
            // value/CPV/status filters are Tender-shaped and have no meaning here.
            Collection::Organizations => &["country", "kind", "buyer", "identifier", "name_prefix"],
            // `notices_query` narrows by the notice layer's own vocabulary — `source`
            // and `kind` (the mapping profile) — and nothing else. `tender` is honoured
            // on `/v1/notices` too, but by an app-layer dispatch (the store has no
            // notice->tender predicate) that intercepts it before this read path is
            // reached, so it is not part of this set.
            // `publication_id` (issue 217) is the official notice number, applied by
            // `notices_query` directly.
            Collection::Notices => &["source", "kind", "publication_id"],
        }
    }
}

/// Can answering this request WALK — i.e. does it use a predicate no index serves?
///
/// Issue 117 established that the reads have no way to predict a query's *cost*: there
/// are no selectivity statistics, and the one probe available (the existence
/// short-circuit) answers only the matches-nothing case. But cost is not what routing
/// needs. **Which query SHAPES are capable of walking is statically decidable**, before
/// a row is read, from the filter alone — and that is enough to send them somewhere
/// they cannot starve the main reader pool (issue 120).
///
/// The classification is exhaustive BY CONSTRUCTION. `Filter` is destructured field by
/// field in [`isolation_routed`], so adding a field to it **fails to compile there**
/// until someone classifies it. That is deliberate and non-negotiable: a hand-maintained
/// list of "expensive filters" would silently route a newly-added unserved filter to the
/// fast pool and reintroduce the defect — the same staleness that put
/// `notices(source, id)` in the schema batch on the strength of a comment written when
/// the table was eight times smaller.
/// Every field of [`Filter`], and why it can or cannot walk — the classification
/// [`walks`] implements, written out so a test can check none has been missed.
///
/// The destructuring in `isolation_routed` makes adding a field a COMPILE error, which
/// forces a decision. It does not force a CORRECT one: the compiler helpfully suggests
/// `..` to ignore the new field, and taking that suggestion silently routes it to the
/// fast pool. This list is the belt to that brace — `filter_classification_is_exhaustive`
/// enumerates the real fields off `Filter`'s own `Debug` output and fails if any is
/// absent here, so a field added with `..` is caught by a test even though it compiled.
#[cfg(test)]
pub(crate) const FILTER_CLASSIFICATION: [(&str, &str); 22] = [
    ("source", "Tenders/Notices: index-served. Lots: t.source, a JOINED table -> isolates"),
    ("country", "EXISTS per row on Tenders/Lots -> isolates. Organizations: index-served"),
    ("cpv", "EXISTS per row -> isolates. Ignored by Organizations/Notices"),
    ("buyer", "EXISTS per row -> isolates. Organizations: o.id, the primary key"),
    ("winner", "EXISTS per row -> isolates"),
    ("bidder", "EXISTS per row -> isolates. Issue 223 seeds the driver from the org index \
                for speed, but a present org still routes to isolation"),
    ("status", "EXISTS over tender_version_dates per row -> isolates"),
    ("min_value", "head column current_value_eur_cents (EUR cents, ADR-0014 D5); still \
                    isolates pending a prod measurement (88d876a rule) — \
                    tenders_current_value_eur is the precondition, not the verdict"),
    ("max_value", "same as min_value"),
    ("currency", "EXISTS over tender_version_amounts per row -> isolates (ADR-0014 D5). \
                  Guarded by the projection-maintained present-set tender_currency_presence \
                  rather than by an index on the column (issue 371)"),
    ("kind", "Tenders: t.kind, NO index -> isolates. Lots: vl.kind, JOINED -> isolates. \
              Organizations/Notices: index-served"),
    ("tender", "the containment shape (issue 115), index-served -> never isolates"),
    ("publication_id", "Notices: index-served by notices_publication_id_id (publication_id, id); \
                        companions post-filter in Rust so the planner cannot flatten onto the wrong \
                        index (issue 217-A, measured). Tenders: seeds the FROM off \
                        tender_versions_publication and SUPPRESSES isolation (issue-212 pattern) — \
                        every companion is bounded by the seed; de-isolated on prod measurement \
                        (bare 3.4 ms, companioned 1.5-1.9 ms, rev 3a658ec). Ignored by \
                        Lots/Organizations"),
    ("identifier", "Organizations: index-served by organizations_identifier_id (identifier, id) \
                    (issue 217). Ignored by Tenders/Lots/Notices"),
    ("published_after", "Tenders, id-ordered shape: a narrow range filters the PK walk -> isolates. \
                         The REST published-ordered path rides tenders_current_published instead \
                         (issue 216). Ignored by Lots/Organizations/Notices"),
    ("published_before", "same as published_after"),
    ("deadline_after", "Tenders: same contract as published_after, over current_deadline / \
                        tenders_current_deadline (issue 216 deadline half). Ignored elsewhere"),
    ("deadline_before", "same as deadline_after"),
    ("name_prefix", "Organizations, id-ordered shape: a sparse prefix filters the PK walk -> \
                     isolates. The REST name-ordered path seeks organizations_name_norm_id \
                     (issue 217-B). Ignored by Tenders/Lots/Notices"),
    ("lang", "not a predicate: a projection selector (ADR-0013 D3) — changes which title a \
              row serves, never which rows match, so it never isolates and is not in \
              honoured_params"),
    ("now", "not a predicate: the reference instant `status` compares against"),
    ("country_seed", "not a request parameter: the async entries' drive-side decision \
                      (issue 273 step 2), set AFTER isolation routing consults `walks`, \
                      so it can never change where a read runs — only how fast it is there"),
];

/// One isolation-routed filter: a filter that, on this collection, sends the request
/// to the shed-only isolated reader pool (issue 120).
///
/// **This enumeration is the coupling between [`walks`] and [`reachable`], and it is
/// the whole point of issue 371.** Issue 219 fixed the same class for
/// country/cpv/buyer/winner/kind and stated the invariant in PROSE — "fold it into one
/// probe so the guard set and the `walks()` set cannot drift again". Prose is what
/// failed: `currency` was later added to `walks()` (ADR-0014 D5), `reachable()` never
/// grew a leg, and `?currency=XXX` — a code present nowhere — was admitted, paid the
/// full density-bounded walk (29.78 s measured on prod, rev `d80a4bd`) and held one of
/// `SLOTS = 4` global isolation slots to answer empty. Four such requests shed every
/// other isolated request on an unauthenticated surface.
///
/// So the two sets are now ONE set. [`isolation_routed`] is the only thing that decides
/// isolation — `walks()` is `!isolation_routed(..).is_empty()` — and [`reachable`]
/// consumes its output through an EXHAUSTIVE `match`. Routing a new filter to isolation
/// therefore means adding a variant here, and a new variant does not compile until
/// `reachable` gives it either a guard leg or an explicit, commented decline:
///
/// ```text
/// error[E0004]: non-exhaustive patterns: `Isolated::Whatever` not covered
///     --> crates/store/src/read.rs:1090:30
///      |
/// 1090 |         let admitted = match routed {
///      |                              ^^^^^^ pattern `Isolated::Whatever` not covered
///      |
/// note: `Isolated` defined here
///  670 |     Whatever,
///      |     -------- not covered
/// ```
///
/// (Reproduced 2026-09-07 by adding a variant and running `cargo check -p store`, so
/// the shape above is the real message rather than a remembered one.)
///
/// A future reader cannot run that negative case (a compile error is not observable
/// from a passing test suite), which is why the error is written out here — and why
/// `the_isolated_set_and_the_guard_set_are_one_set` in
/// `crates/store/tests/isolation_routing.rs` is the belt to this brace: it enumerates
/// every variant, asserts each is reachable from a real filter, and pins which ones
/// carry a probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Isolated {
    Country,
    Cpv,
    Buyer,
    Winner,
    Bidder,
    Currency,
    Status,
    MinValue,
    MaxValue,
    Source,
    PublishedAfter,
    PublishedBefore,
    DeadlineAfter,
    DeadlineBefore,
    NamePrefix,
    Kind,
}

/// Can answering this request WALK? — the boolean face of [`isolation_routed`], and the
/// routing decision the API layer reads. Kept as the public predicate because that is
/// what every caller wants; the LIST is what the guard needs.
pub fn walks(collection: Collection, f: &Filter) -> bool {
    !isolation_routed(collection, f).is_empty()
}

/// Every filter of `f` that routes THIS collection's read to the isolated pool, in the
/// order [`reachable`] should probe them: the cheap index seeks first, the bare table
/// scans last, so a cheaper absent leg short-circuits before an expensive one is paid
/// for. Empty ⇒ the read cannot walk and stays on the main pool.
pub fn isolation_routed(collection: Collection, f: &Filter) -> Vec<Isolated> {
    let Filter {
        source,
        country,
        cpv,
        buyer,
        winner,
        bidder,
        status,
        min_value,
        max_value,
        currency,
        kind,
        tender,
        // A projection selector, not a predicate (ADR-0013 D3): it changes
        // which title a matching row serves, never which rows match — and the
        // pick's per-row subquery runs identically with or without it, so it
        // cannot change a query's cost class either.
        lang: _,
        publication_id,
        identifier,
        published_after,
        published_before,
        deadline_after,
        deadline_before,
        name_prefix,
        now: _,
        // Not a caller predicate: the async entries decide it AFTER isolation
        // routing has already run, so it cannot change where a read executes —
        // a seeded country read still runs isolated, it is just fast there.
        country_seed: _,
    } = f;

    // The `version_predicates` set: `EXISTS` subqueries evaluated PER ROW over the
    // satellites. No cursor shape and no index on the driven table helps, because the
    // filter is not a column of it — issue 117 Class B. Applied by `tenders` and `lots`
    // only; the other collections ignore these fields entirely, so passing one there
    // cannot walk.
    //
    // Ordered guard-first: the six legs `reachable` can PROBE come before the three it
    // can only admit, so an absent country/cpv/org/currency short-circuits without the
    // rest of the list being considered.
    let version_predicate_set = [
        (Isolated::Country, country.is_some()),
        (Isolated::Cpv, cpv.is_some()),
        (Isolated::Buyer, buyer.is_some()),
        (Isolated::Winner, winner.is_some()),
        (Isolated::Bidder, bidder.is_some()),
        (Isolated::Currency, currency.is_some()),
        (Isolated::Status, status.is_some()),
        (Isolated::MinValue, min_value.is_some()),
        (Isolated::MaxValue, max_value.is_some()),
    ];

    // `identifier` (issue 217) is served by `organizations_identifier_id (identifier,
    // id)` on the one collection that reads it, so it seeks on the main pool exactly
    // like `country`/`kind` do — and the other collections ignore it. It therefore
    // isolates nowhere; bound here only so adding the field forced this decision.
    let _ = identifier;
    // `publication_id` (issue 217-A) is served by `notices_publication_id_id` on
    // Notices (de-isolated on measurement, see that arm) and consulted in the
    // Tenders arm below, where its seed suppresses isolation.

    // `tender` is the containment shape (issues 115/116): a `tender=X` read drives
    // from that one Tender's `tender_version_lots` slice (whole-corpus max ~2,604
    // lots) and is index-served, so it bounds every companion predicate and cannot
    // walk. It therefore SUPPRESSES isolation, not merely abstains — consulted in the
    // Lots arm below (issue 212). Before, this line dropped it (`let _ = tender`), so
    // a companion `kind`/`source` still isolated the bounded read and it could 503.

    // Each arm names the filters this collection routes to isolation, paired with
    // whether the request actually set one; the tail filters that pair down to the
    // routed list. Same decisions, same order of reasoning as the boolean form this
    // replaced — a LIST rather than an OR so `reachable` can be handed the members.
    let candidates: Vec<(Isolated, bool)> = match collection {
        // `source` is served by `tenders_source_id`. `kind` is `t.kind`, which NO index
        // covers — `tenders_procedure_key`, `tenders_island`, `tenders_current_published`
        // and `tenders_source_id` are the whole set — so a value matching nothing walks
        // 4.26M rows exactly as the filters issue 117 fixed did. It was not in 117's
        // audit; routing it here is what stops it being a silent survivor.
        //
        // A published range (issue 216) isolates HERE because this classifies the
        // id-ordered `tenders_query` shape (SSE snapshots, `sort=id`), where a narrow
        // range filters the PK walk. The REST published-ordered path never consults
        // this arm for the range — `tenders_by_published` rides the
        // `tenders_current_published` index by construction, and the handler strips
        // the range before asking `walks()` about the REMAINING filters.
        // issue 217-A: `publication_id` is seeded from `tender_versions_publication`
        // and is the ONLY `t.`-column predicate in its SQL (companions post-filter
        // in Rust — the flatten measured 35 s before that). De-isolated on
        // MEASUREMENT (88d876a rule), rev 3a658ec: bare 3.4 ms, source-paired
        // 1.5 ms, kind/bound/sorted 1.5-1.6 ms, absent 0.9 ms, main pool at
        // 14-20 ms throughout. And like `tender` on Lots (issue 212) the seed
        // SUPPRESSES isolation, not merely abstains: every companion runs over
        // the seed's ≤handful of rows — post-filtered or EXISTS-probed — so no
        // predicate can walk while the number is present. Same accepted window
        // as notices: a box whose index is not yet built walks until the boot
        // detector's reindex lands.
        Collection::Tenders if publication_id.is_some() => Vec::new(),
        Collection::Tenders => version_predicate_set
            .into_iter()
            .chain([
                (Isolated::PublishedAfter, published_after.is_some()),
                (Isolated::PublishedBefore, published_before.is_some()),
                (Isolated::DeadlineAfter, deadline_after.is_some()),
                (Isolated::DeadlineBefore, deadline_before.is_some()),
                // LAST: its guard leg is a bare table scan, so every cheaper leg
                // above gets to short-circuit before it is paid for.
                (Isolated::Kind, kind.is_some()),
            ])
            .collect(),
        // Isolated because the COST is unbounded for sparse and absent values — NOT
        // because the filter is unserved. That distinction became load-bearing when
        // issue 16 restructured this read: `lots` now drives and the probe is a
        // three-column primary-key seek, so the old justification ("no index on
        // `lots` can serve either") is simply false.
        //
        // The cost argument survives the change and any future index. `?kind=` on a
        // value with fewer rows than the page limit walks all 13.2M lots to collect a
        // page it can never fill — 132.1s measured at prod scale — because the work is
        // bounded by DENSITY, not by the filter being index-served. `?source=` is the
        // same shape.
        //
        // **A fast common case is not grounds for de-isolation.** `?kind=Lot` is now
        // 0.004s; `?kind=` on a sparse value is unchanged. Deleting this arm because
        // the default request got quick would return the sparse and absent cases to
        // the main reader pool, which is what issue 120 exists to prevent.
        //
        // But a `tender=X` bound makes the WHOLE read a containment lookup over one
        // Tender's lot slice, so no companion predicate can walk — `tender.is_none()`
        // gates the decision (issue 212). Without `tender`, the sparse/absent-density
        // cases above still isolate.
        Collection::Lots if tender.is_some() => Vec::new(),
        Collection::Lots => version_predicate_set
            .into_iter()
            .chain([
                // Both bare table scans on their guard side — after the seeks.
                (Isolated::Source, source.is_some()),
                (Isolated::Kind, kind.is_some()),
            ])
            .collect(),
        // `country` and `identifier_kind` are served by the issue-117 indexes, and
        // `buyer` is `o.id`, the primary key. `name_prefix` (issue 217-B) isolates
        // in THIS id-ordered shape — a sparse prefix filters the PK walk (SSE
        // snapshots take this path); the REST handler routes prefix searches
        // through `organizations_by_name`, which seeks `(name_norm, id)` by
        // construction and strips the prefix before consulting this arm.
        Collection::Organizations => vec![(Isolated::NamePrefix, name_prefix.is_some())],
        // `source` is served by `notices_source_id` and `kind` (`profile`) by
        // `notices_profile`. `publication_id` (issue 217-A) is served by
        // `notices_publication_id_id (publication_id, id)`: `notices_query` emits it
        // as the ONLY identity predicate (companions post-filter in Rust, or the
        // planner flattens onto the wrong index — 7.8 s, measured), so the read is a
        // seek + cursor ride. De-isolated on MEASUREMENT, not assumption (the 88d876a
        // rule): 1 ms present / 0.8 ms absent against prod's real file with the index
        // built. A box whose index is not yet built (fresh restore, pre-auto-reindex)
        // walks on the main pool for those minutes — accepted and bounded: the
        // missing-index detector enqueues the build at boot.
        Collection::Notices => Vec::new(),
    };
    candidates.into_iter().filter(|(_, set)| *set).map(|(which, _)| which).collect()
}

/// The version-scoped filters, emitted against caller-supplied expressions for the
/// Tender id and the version `seq`.
///
/// Parameterised rather than duplicated because two query shapes need the same
/// predicates against different scopes: the JOIN form has `t`/`v` in FROM and passes
/// `"t.id"`/`"v.seq"`, while [`lots_query`] has neither and correlates on
/// `l.tender_id` with `seq` recomputed. Writing them twice is the paraphrase hazard
/// that blocked issue 112's B1 — one edit to a predicate would silently apply to one
/// shape and not the other.
/// `deadline_col`: the head deadline column when the builder's FROM serves one
/// (`Some("t.current_deadline")` for the Tenders shapes). `status` then becomes a
/// RANGE predicate on that indexed column instead of a per-row EXISTS — the fix
/// for issue 273's walk DoS (`status=open&country=LU` walked all ~7.9M rows to
/// the 30s bound; the range bounds the scan to the open head, 0.13s validated on
/// prod). Provably equivalent: the projection's head UPDATE writes
/// `MAX(head submission_deadline)` into `current_deadline` (`head_deadline`,
/// canonical.rs), and MAX(d) > now ⟺ EXISTS(d > now); a Tender with no deadline
/// gets NULL, which both forms read as Closed. Lots builders pass `None` — no
/// head column there — and keep the EXISTS.
fn version_predicates(
    q: &mut Query,
    f: &Filter,
    tid: &str,
    seq: &str,
    deadline_col: Option<&str>,
    value_col: &str,
) {
    if let Some(country) = &f.country {
        q.push(
            &format!(" AND EXISTS (SELECT 1 FROM tender_version_classifications c
                           WHERE c.tender_id = {tid} AND c.seq = {seq}
                             AND c.scheme = 'nuts' AND c.code LIKE ?)"),
            [t(format!("{country}%"))],
        );
    }
    if let Some(cpv) = &f.cpv {
        q.push(
            &format!(" AND EXISTS (SELECT 1 FROM tender_version_classifications c
                           WHERE c.tender_id = {tid} AND c.seq = {seq}
                             AND c.scheme = 'cpv' AND c.code LIKE ?)"),
            [t(format!("{cpv}%"))],
        );
    }
    if let Some(buyer) = f.buyer {
        q.push(
            &format!(" AND EXISTS (SELECT 1 FROM tender_version_parties p
                           WHERE p.tender_id = {tid} AND p.seq = {seq}
                             AND p.organization_id = ? AND p.role LIKE '%Buyer%')"),
            [Value::Integer(buyer)],
        );
    }
    if let Some(winner) = f.winner {
        q.push(
            &format!(" AND EXISTS (SELECT 1 FROM tender_version_result_winners w
                           WHERE w.tender_id = {tid} AND w.seq = {seq}
                             AND w.organization_id = ?)"),
            [Value::Integer(winner)],
        );
    }
    if let Some(bidder) = f.bidder {
        // Any org that SUBMITTED a bid (role `tenderer`, verified against prod), won
        // or not — the competitor-history reverse-lookup (issue 217). Subcontractors
        // are named in a bid but did not submit it, so they are excluded.
        q.push(
            &format!(" AND EXISTS (SELECT 1 FROM tender_version_bid_parties bp
                           WHERE bp.tender_id = {tid} AND bp.seq = {seq}
                             AND bp.organization_id = ? AND bp.role = 'tenderer')"),
            [Value::Integer(bidder)],
        );
    }
    if let Some(status) = f.status {
        // "Open" is a submission deadline still in the future. A Tender that
        // never published one (award notices) is therefore Closed, which is the
        // useful reading: it cannot be bid on.
        if let Some(col) = deadline_col {
            match status {
                Status::Open => q.push(&format!(" AND {col} > ?"), [Value::Integer(f.now)]),
                Status::Closed => q.push(
                    &format!(" AND ({col} IS NULL OR {col} <= ?)"),
                    [Value::Integer(f.now)],
                ),
            }
        } else {
            let exists = format!("EXISTS (SELECT 1 FROM tender_version_dates d
                                   WHERE d.tender_id = {tid} AND d.seq = {seq}
                                     AND d.field = 'submission_deadline' AND d.utc_seconds > ?)");
            match status {
                Status::Open => q.push(&format!(" AND {exists}"), [Value::Integer(f.now)]),
                Status::Closed => q.push(&format!(" AND NOT {exists}"), [Value::Integer(f.now)]),
            }
        }
    }
    // The value bounds compare the fold-maintained head column (ADR-0014 D5):
    // EUR cents against the head version's MAX derived amount. NULL — no
    // amount, nothing converts, or the row predates the backfill — fails both
    // comparisons, so an unconvertible Tender never matches a value bound
    // (the same one-sided honesty as every filter guard). The raw-cents
    // comparison this replaces mixed currencies numerically; its retirement
    // is CHANGELOG.md's first entry.
    for (bound, op) in [(f.min_value, ">="), (f.max_value, "<=")] {
        if let Some(eur_cents) = bound {
            q.push(&format!(" AND {value_col} {op} ?"), [Value::Integer(eur_cents)]);
        }
    }
    if let Some(currency) = &f.currency {
        // The PUBLISHED currency (ADR-0014 D5): any amount row of the current
        // version in that currency, tender-level or lot-level — the same
        // population the value bounds aggregate over.
        q.push(
            &format!(" AND EXISTS (SELECT 1 FROM tender_version_amounts a
                           WHERE a.tender_id = {tid} AND a.seq = {seq}
                             AND a.currency = ?)"),
            [t(currency.clone())],
        );
    }
}

/// Which participation filter, if any, should SEED the driven set — and the org id
/// to seed it with (issue 223).
///
/// `winner`/`bidder`/`buyer` are per-row `EXISTS` predicates (see
/// [`version_predicates`]). Driven off the `tenders`/`lots` primary key by the
/// `ORDER BY id LIMIT` pagination, they are evaluated for every driven row, so a
/// PRESENT org walks the whole corpus to fill a page — 35 s+, a client timeout.
/// The reverse-lookup's natural driver is instead "the tenders this org touched",
/// which the participation table's `organization_id` index serves directly and
/// which is small (an org appears on a bounded slice of the corpus).
///
/// So the caller seeds the FROM clause with
/// `(SELECT DISTINCT tender_id FROM <table> WHERE organization_id = ?<extra>)` and
/// joins the driven table to it. The seed is a SUPERSET of the true matches — the
/// untouched `EXISTS` predicates still enforce the role narrowing and the
/// current-version constraint — so the result is byte-identical to the walk, only
/// bounded by the org's participation count instead of the corpus. Precedence is by
/// expected selectivity (winner rows ⊆ bidder rows ⊆ a frequent buyer's), but any
/// present filter is a correct seed because the `EXISTS` set, not the seed, decides
/// membership.
///
/// The `buyer` seed ALSO narrows by role (issue 225): `tender_version_parties`
/// holds every party role, and a ubiquitous NON-buyer org (org 3: 1.79M
/// review-body rows, 3 actual buyer rows — measured on prod) made the role-blind
/// seed haul millions of candidates the `EXISTS` then discarded (~17-19 s). The
/// role clause mirrors the EXISTS's own `%Buyer%` match, so the superset property
/// is preserved exactly; `tender_version_parties_org_role (organization_id, role,
/// tender_id)` serves the narrowed seed index-only. `winner`/`bidder` seeds need
/// no extra clause — their tables are participation-bounded already.
fn participation_seed(f: &Filter) -> Option<(&'static str, &'static str, i64)> {
    if let Some(org) = f.winner {
        Some(("tender_version_result_winners", "", org))
    } else if let Some(org) = f.bidder {
        Some(("tender_version_bid_parties", "", org))
    } else if let Some(org) = f.buyer {
        Some(("tender_version_parties", " AND role LIKE '%Buyer%'", org))
    } else {
        None
    }
}

/// The role clause the seeded org's PREDICATE carries (`version_predicates`,
/// verbatim, unqualified so it reads on whichever alias it sits under). The lots
/// seed (issue 388) decides membership by itself, so its head-version level must
/// carry exactly this — while its DISTINCT pre-level carries only what
/// `participation_seed` narrows by: the buyer role is covered there by
/// `tender_version_parties_org_role`, but the bidder role is not, and
/// `DISTINCT tender_id … AND role = 'tenderer'` cost a per-row table lookup for
/// each of org 357's 194k bid rows (4.2 s against 2.1 s covered, prod 2026-09-18).
fn participation_role(f: &Filter) -> &'static str {
    if f.winner.is_some() {
        ""
    } else if f.bidder.is_some() {
        " AND role = 'tenderer'"
    } else {
        " AND role LIKE '%Buyer%'"
    }
}

/// The title pick's ORDER BY rank for an optional requested language
/// (ADR-0013 D3). `lang` is inlined as a literal, which is safe ONLY because
/// the guard here re-verifies the shape the API layer already validated —
/// exactly three ASCII uppercase letters (the fold's ISO 639-2/T vocabulary,
/// `ingest::project::normalize_lang`); anything else falls back to the
/// default rank rather than reaching the SQL.
fn title_rank(lang: Option<&str>) -> String {
    // ADR-0013 D3, all four legs: requested → ENG → the version's ORIGINAL
    // language (`v.original_lang`, in scope because `pick` correlates on
    // `v.seq`) → any labelled → unlabelled. A NULL `original_lang` makes the
    // third term NULL, which sorts below both 0 and 1 under DESC — so a
    // version whose era never said its language ranks as if the leg were
    // absent, exactly the chain before the column existed.
    //
    // The `s.value` tail is the tie rule, stated (issue 343): a notice can publish
    // several tender-level titles in one language, and "first in scan order" was
    // an accident of insertion — the fold's `head_title` broke the same tie the
    // other way. Both now take the smallest value among equals.
    match lang {
        Some(l) if l.len() == 3 && l.bytes().all(|b| b.is_ascii_uppercase()) => {
            format!(
                "(s.lot_id IS NULL) DESC, (s.lang = '{l}') DESC, (s.lang = 'ENG') DESC, \
                 (s.lang = v.original_lang) DESC, s.value"
            )
        }
        _ => "(s.lot_id IS NULL) DESC, (s.lang = 'ENG') DESC, (s.lang = v.original_lang) DESC, s.value"
            .to_owned(),
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

/// Can any row satisfy `filter` at all? The absent-value short-circuit that both
/// [`tenders`] and [`lots`] run before their `Scope::Page` walk. The tender-level
/// value predicates mean the same thing on both endpoints (`/v1/lots?country=DE` is
/// `/v1/tenders?country=DE` scoped to lots), so a single existence probe serves both;
/// `collection` only selects which table the `kind` leg probes.
///
/// The value-shaped predicates in [`version_predicates`] are `EXISTS` subqueries
/// evaluated PER ROW. When the value matches nothing the query still walks every
/// Tender to discover that — measured on prod at over 380 seconds for
/// `/v1/tenders?country=ZZ`, unauthenticated, on a documented filter. No cursor
/// shape helps, because the filter is not a column of the driven table (issue 117
/// Class B).
///
/// But each of those subqueries is satisfiable only if SOME row exists carrying the
/// value at all, and that is one index seek. If the seek finds nothing, no Tender can
/// match and the empty page is the CORRECT answer rather than an approximation —
/// reached without the walk. Same move [`changes_since`] already makes for an
/// unknown `entity_kind` (issue 61 finding 2): a kind with no rows must never trigger
/// a table walk to discover it has none.
///
/// Deliberately CONSERVATIVE in the safe direction. The probes ignore the
/// `c.seq = v.seq` correlation and the buyer's `role LIKE '%Buyer%'`, so a value
/// present only in a superseded version, or an organization present only in a
/// non-buyer role, fails to short-circuit and falls through to the full query. That
/// costs a walk we could have avoided; the reverse — short-circuiting something that
/// does match — would be a wrong answer, so the asymmetry is the right way round.
///
/// **This is a performance fix that reduces the DoS surface; it is NOT a defence.**
/// It answers *matches-nothing* only. A prefix that exists but sits on high
/// `tender_id`s still walks — measured at 0.5s against 0.4954s for the absent case on
/// a 200k-Tender fixture, i.e. the guard buys nothing there. `MT` is a real country,
/// so that case is reachable without adversarial intent. Bounding worst-case work
/// needs the real fix (restructuring the per-row `EXISTS`), not this.
async fn reachable(conn: &Connection, filter: &Filter, collection: Collection) -> turso::Result<bool> {
    // ONE list, walked in order. `isolation_routed` is the same call `walks()` makes to
    // decide the pool, so the guard set cannot be a different set from the routed set —
    // that drift is issue 371, and the `match` below is what now prevents it: a new
    // `Isolated` variant does not compile until it is given a leg or an explicit
    // decline. Ordered cheap-seek-first by `isolation_routed`, so an absent
    // country/cpv/org/currency short-circuits before a bare scan is paid for.
    for routed in isolation_routed(collection, filter) {
        let admitted = match routed {
            Isolated::Country => prefix_reachable(conn, "nuts", filter.country.as_deref()).await?,
            Isolated::Cpv => prefix_reachable(conn, "cpv", filter.cpv.as_deref()).await?,
            // All three seek `..._org(organization_id)`, the indexes issue 62 deferred.
            Isolated::Buyer => org_reachable(conn, "tender_version_parties", filter.buyer).await?,
            Isolated::Winner => {
                org_reachable(conn, "tender_version_result_winners", filter.winner).await?
            }
            Isolated::Bidder => {
                org_reachable(conn, "tender_version_bid_parties", filter.bidder).await?
            }
            Isolated::Currency => currency_reachable(conn, filter.currency.as_deref()).await?,
            // Issue 275 (measured 2026-08-25): `?source=<absent>` on LOTS ran 33.4s to
            // the shed — the lots shape tests source through a correlated seek into
            // `tenders` PER CANDIDATE ROW, 13.2M seeks for a value no row carries. The
            // same absent value on TENDERS is a cheap in-row compare along its PK walk
            // (measured 0.69s), which is why `isolation_routed` routes source on Lots
            // and not on Tenders — so this leg is reached for Lots alone, and the old
            // `matches!(collection, Collection::Lots)` gate is now the routing itself.
            // A bare `WHERE source = ? LIMIT 1` over `tenders` — unindexed, so the
            // absent case pays one plain table pass (about what the tenders read itself
            // pays) instead of the correlated walk, and a real source hits its first row
            // immediately. Same one-sided hazard as every leg here: it answers
            // matches-nothing only; a present-but-rare source still walks, dense in
            // practice (ted/doe).
            Isolated::Source => match &filter.source {
                Some(source) => {
                    exists(conn, "SELECT 1 FROM tenders WHERE source = ? LIMIT 1", vec![t(source)])
                        .await?
                }
                None => true,
            },
            // NO PROBE, and each of these is a decision rather than an omission.
            //
            // `status` is not a value that can be absent: both members of its
            // two-valued vocabulary are always "present" in the sense a probe could
            // test, and the head-column range form (issue 273) already bounds the scan
            // to the open head. There is nothing to seek.
            Isolated::Status => true,
            // The value bounds compare `current_value_eur_cents`, a head column. A
            // bound outside the corpus range is not a matches-nothing VALUE but an
            // empty RANGE, and answering that needs MIN/MAX statistics this layer does
            // not keep (issue 117: no selectivity statistics). `tenders_current_value_eur`
            // is the precondition for the real fix, not this guard.
            Isolated::MinValue | Isolated::MaxValue => true,
            // Same shape as the value bounds, over the publication/deadline head
            // columns: a range, not a value, and a narrow-but-nonempty range is the
            // costly case a presence probe could not detect anyway.
            Isolated::PublishedAfter
            | Isolated::PublishedBefore
            | Isolated::DeadlineAfter
            | Isolated::DeadlineBefore => true,
            // Organizations-only (issue 217-B), and `reachable` is called from
            // `tenders`/`lots` alone — so this arm is unreachable today. It exists
            // because the match must be exhaustive, and it declines rather than
            // guessing: if `organizations()` ever grows the guard, the leg belongs
            // here, seeking `organizations_name_norm_id` over the prefix range.
            Isolated::NamePrefix => true,
            // `kind` is `t.kind` (tenders) / `vl.kind` (lots), and NO index covers
            // either — it is precisely why it routes to the isolated pool. So an absent
            // value walks the whole driven table INSIDE that pool, holding a reader slot
            // for the duration: issue 219's unauthenticated saturation hole. `kind` was
            // that issue's survivor, guarded on `lots` but not `tenders`, so a handful
            // of `?kind=<absent>` requests wedged the pool while every legitimate
            // filtered read shed 503.
            //
            // Probe the single driven table with a bare `WHERE kind = ? LIMIT 1`. This
            // is NOT free for an absent value — no index, so it scans the table — but it
            // is a single-column scan with no joins, `EXISTS` subqueries or sort, orders
            // of magnitude short of the full filtered-and-ordered walk it stands in for
            // (`/v1/tenders?kind=` measured at 130-230s; the bare scan is a table pass),
            // and ~1ms when the value is present (prod's first `Lot`/`procedure` row
            // sits at the head of its table). Same one-sided hazard as the prefix
            // probes: it answers *matches-nothing* only, so it can cost a walk it need
            // not have but never drops a row that matches. Routed LAST so a cheaper
            // absent leg short-circuits before this scan is paid for.
            //
            // What it still does NOT cover: a `kind` that EXISTS but has fewer rows than
            // the page limit walks everything to fill a page it never can
            // (`T(K) = T_full*50/K`). No corpus value is near that today; latent,
            // confined by the isolation of issue 120, and recorded rather than fixed.
            Isolated::Kind => match (&filter.kind, collection) {
                (Some(kind), Collection::Tenders) => kind_reachable(conn, "tenders", kind).await?,
                (Some(kind), Collection::Lots) => {
                    kind_reachable(conn, "tender_version_lots", kind).await?
                }
                // Unreachable: `isolation_routed` routes `kind` on Tenders and Lots only
                // and `reachable` is called from nowhere else. Decline to short-circuit
                // rather than probe a table whose `kind` column means something
                // different (`profile`, `identifier_kind`).
                (Some(_), Collection::Organizations | Collection::Notices) => true,
                (None, _) => true,
            },
        };
        if !admitted {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The `country`/`cpv` leg: does any classification carry this prefix?
///
/// A prefix range, NOT `LIKE`. Measured on prod: `code LIKE 'ZZ%'` takes 41.05s against
/// 0.01s for the range, because turso keeps the index but drops the second bound —
/// `(scheme=?)` alone — and then filters the whole scheme's partition row by row. Both
/// forms plan as `SEARCH ... USING INDEX`, so a plan cannot tell them apart; only the
/// clock can (issue 112 rule 6).
async fn prefix_reachable(
    conn: &Connection,
    scheme: &str,
    prefix: Option<&str>,
) -> turso::Result<bool> {
    let Some(prefix) = prefix else { return Ok(true) };
    // `LIKE` is ASCII-case-INSENSITIVE and a range comparison is not, so one range over
    // the prefix as given is NARROWER than the predicate it stands in for. Verified:
    // with `DE300` stored, `LIKE 'de%'` matches and `code >= 'de' AND code < 'df'` does
    // not — so a lowercase `?country=de` would have short-circuited to an empty page
    // while the real query returns rows. A guard that is narrower than what it guards
    // does not make the read faster, it makes it WRONG.
    //
    // So probe every case variant of the prefix and treat the value as reachable if ANY
    // of them hits: their union is exactly the set `LIKE` would match. `None` means too
    // many variants to be worth it — skip the guard and let the full query answer, which
    // is slow but correct.
    let Some(ranges) = prefix_ranges(prefix) else { return Ok(true) };
    for (low, high) in ranges {
        if exists(
            conn,
            "SELECT 1 FROM tender_version_classifications
              WHERE scheme = ? AND code >= ? AND code < ? LIMIT 1",
            vec![t(scheme), t(&low), t(&high)],
        )
        .await?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The `buyer`/`winner`/`bidder` leg: does this organization appear in the
/// participation table at all? Seeks `..._org(organization_id)`.
async fn org_reachable(conn: &Connection, table: &str, org: Option<i64>) -> turso::Result<bool> {
    let Some(org) = org else { return Ok(true) };
    let sql = format!("SELECT 1 FROM {table} WHERE organization_id = ? LIMIT 1");
    exists(conn, &sql, vec![Value::Integer(org)]).await
}

/// The `kind` leg: a bare single-column scan of the one driven table.
async fn kind_reachable(conn: &Connection, table: &str, kind: &str) -> turso::Result<bool> {
    let sql = format!("SELECT 1 FROM {table} WHERE kind = ? LIMIT 1");
    exists(conn, &sql, vec![t(kind)]).await
}

/// The `currency` leg (issue 371): does any amount row anywhere carry this code?
///
/// `currency` is a per-row `EXISTS` over `tender_version_amounts`, whose only index is
/// `tender_version_amounts_version (tender_id, seq)` — so the column is unindexed and an
/// absent code walked the whole corpus inside the isolated pool to answer empty: 29.78 s
/// measured on prod for `?currency=XXX`, just inside `REQUEST_DEADLINE`, holding one of
/// four global slots. Four such requests shed everything else on those endpoints.
///
/// It does NOT seek an index on `tender_version_amounts(currency)`, deliberately: that
/// would pay write amplification on every fold across tens of millions of rows to store
/// a key with a few dozen distinct values. It seeks the PRESENT-SET the projection
/// maintains instead (`tender_currency_presence`, canonical.rs) — one primary-key probe
/// over ~26 rows.
///
/// **The set may be a SUPERSET and must never be a subset.** Entries are only ever
/// added: a delete (a retired Tender, a shrinking rewrite, a dropped version) never
/// removes one, so an entry can outlive the last row carrying it. That makes this probe
/// ADMIT a code no row carries any more, and the read then degrades to exactly the walk
/// this guard exists to avoid — slow, and correct. The reverse, a MISSING entry for a
/// code rows do carry, would short-circuit to an empty page and hide real rows: fast,
/// WRONG and silent. That is why the fold writes the entry in the SAME transaction as
/// the amount rows (`Pending::flush`), and why this declines entirely until
/// `projection_state.currency_presence_complete` attests the standing corpus is covered
/// — on a file whose amounts predate the table the set is a subset until
/// `backfill-currencies` has run, and a subset is the one thing this must not read.
///
/// Case: the predicate it stands in for is `a.currency = ?`, an exact comparison, and so
/// is this — the set stores codes exactly as the fold wrote them. The guard is therefore
/// exactly as narrow as the predicate, not narrower.
async fn currency_reachable(conn: &Connection, currency: Option<&str>) -> turso::Result<bool> {
    let Some(currency) = currency else { return Ok(true) };
    if !exists(
        conn,
        "SELECT 1 FROM projection_state WHERE id = 0 AND currency_presence_complete <> 0",
        Vec::new(),
    )
    .await?
    {
        return Ok(true);
    }
    exists(
        conn,
        "SELECT 1 FROM tender_currency_presence WHERE currency = ? LIMIT 1",
        vec![t(currency)],
    )
    .await
}

/// [`reachable`] under a test-visible name (the `_for_test` pattern of
/// `Db::set_projection_epoch_for_test`). The guard changes SPEED and never RESULTS, so
/// no assertion on a returned page can tell a short-circuit from a walk that happened to
/// find nothing — the integration tests need the verdict itself to assert on the
/// MECHANISM rather than on a clock.
pub async fn reachable_for_test(
    conn: &Connection,
    filter: &Filter,
    collection: Collection,
) -> turso::Result<bool> {
    reachable(conn, filter, collection).await
}

async fn exists(conn: &Connection, sql: &str, params: Vec<Value>) -> turso::Result<bool> {
    Ok(conn.query(sql, params).await?.next().await?.is_some())
}

/// The index ranges whose union is exactly `code LIKE '<prefix>%'`.
///
/// One range per ASCII-case variant of the prefix, because `LIKE` folds ASCII case
/// and `>=`/`<` do not. `de` yields the four ranges `de..df`, `dE..dF`, `De..Df`,
/// `DE..DF`; a prefix of digits (CPV) yields one. Non-ASCII bytes are not folded by
/// `LIKE` either, so they do not branch.
///
/// `None` when the guard is not worth applying — an empty prefix, more than
/// [`MAX_CASE_VARIANTS`] variants, or a prefix with no representable upper bound.
/// The caller then skips the short-circuit and lets the full query answer: slower,
/// and still correct. Every `None` path must stay on that side, because a guard that
/// matches less than the predicate it stands in for returns wrong rows rather than
/// slow ones.
#[cfg(test)]
pub(crate) fn prefix_ranges_for_test(prefix: &str) -> Option<Vec<(String, String)>> {
    prefix_ranges(prefix)
}

#[cfg(test)]
pub(crate) fn successor_for_test(prefix: &str) -> Option<String> {
    successor(prefix)
}

fn prefix_ranges(prefix: &str) -> Option<Vec<(String, String)>> {
    /// 2^5 = 32 seeks at ~0.02s is still four orders of magnitude under the walk it
    /// avoids; beyond that the guard stops paying for itself. Five, not four,
    /// because the API admits a `country` of up to five characters (a NUTS code is
    /// `DEB35` at its longest — issue 117) and every value it admits must be one
    /// this guard can bound: a declined guard is the 30 s walk the bound exists to
    /// prevent, and `?country=Germany` (seven letters, declined at 16) reached it.
    const MAX_CASE_VARIANTS: usize = 32;

    // A `LIKE` METACHARACTER makes the prefix a pattern, and a range is not one.
    // `version_predicates` binds `format!("{prefix}%")`, so `?country=%` becomes
    // `LIKE '%%'` — which matches EVERY code — while the range `['%', '&')` matches
    // none, and the guard would return an empty page for a filter that matches
    // everything. `_` is the same trap one character at a time: `?country=_E` matches
    // `DE300` and the range does not. Nothing upstream validates these — `Params`
    // passes `country` and `cpv` through verbatim — so this is the only place it can
    // be caught, and the answer is to decline the guard rather than to interpret the
    // pattern. `\` is literal in SQLite's `LIKE` without an `ESCAPE` clause, but it is
    // declined too so that adding one later cannot silently make this wrong.
    if prefix.is_empty() || !prefix.is_ascii() || prefix.contains(['%', '_', '\\']) {
        return None;
    }
    let letters = prefix.chars().filter(char::is_ascii_alphabetic).count();
    if 1usize.checked_shl(letters as u32)? > MAX_CASE_VARIANTS {
        return None;
    }

    let mut variants = vec![String::new()];
    for c in prefix.chars() {
        variants = variants
            .into_iter()
            .flat_map(|v| {
                if c.is_ascii_alphabetic() {
                    vec![format!("{v}{}", c.to_ascii_lowercase()), format!("{v}{}", c.to_ascii_uppercase())]
                } else {
                    vec![format!("{v}{c}")]
                }
            })
            .collect();
    }
    variants.into_iter().map(|v| successor(&v).map(|hi| (v, hi))).collect()
}

/// The least string greater than every string starting with `prefix`, so
/// `code >= prefix AND code < successor` is exactly "has this prefix".
///
/// Increments the last CHARACTER, not the last byte (issue 387). The byte
/// version looked equivalent — UTF-8 sorts byte-wise in code-point order, which
/// is what makes this range trick work at all — but incrementing a byte can
/// leave a string that is not UTF-8 at all, and the `String::from_utf8(…).ok()`
/// that noticed turned into a `None`, which both call sites read as "no upper
/// bound". The search then walked from the prefix to the end of the table and
/// served whatever the page limit cut it off at.
///
/// It fires on any prefix whose last character ends in byte `0xBF` — the last
/// code point of each UTF-8 block, so roughly 1 letter in 64 across the alphabets
/// that need more than ASCII: Cyrillic `п` (`d0 bf`), Greek `ο` (`ce bf`), `¿`
/// (`c2 bf`). Measured on prod: `name_prefix=яп` returned 1 match and 4
/// non-matches on the first page, and `name_prefix=δήμο&country=FR` returned five
/// rows of which none started with the prefix, both with `ignored_filters: []`.
///
/// `None` now means what the call sites already assume: there IS no upper bound,
/// because the prefix is empty (everything matches) or is entirely `char::MAX`
/// (nothing sorts above it). Neither widens a result.
fn successor(prefix: &str) -> Option<String> {
    let mut out = prefix.to_owned();
    while let Some(last) = out.pop() {
        // The next scalar value, stepping over the surrogate gap D800..=DFFF,
        // which `char::from_u32` rejects. `char::MAX + 1` is rejected too, and
        // that is the carry: drop this character and increment the one before.
        let next = u32::from(last) + 1;
        let next = if next == 0xD800 { 0xE000 } else { next };
        if let Some(next) = char::from_u32(next) {
            out.push(next);
            return Some(out);
        }
    }
    None
}

pub async fn tenders(
    conn: &Connection,
    filter: &Filter,
    scope: Scope,
) -> turso::Result<Vec<TenderRow>> {
    if matches!(scope, Scope::Page { .. }) && !reachable(conn, filter, Collection::Tenders).await? {
        return Ok(Vec::new());
    }
    let filter = &with_country_seed(conn, filter).await?;
    let q = tenders_query(filter, scope);
    let mut rows = q.rows(conn, tender_row).await?;
    retain_publication_companions(&mut rows, filter);
    Ok(rows)
}

/// issue 217-A: a publication seed leaves every `t.`-column companion out of the
/// SQL (see `tenders_query` — the planner otherwise flattens onto the companion's
/// index and walks its slice, measured 35 s). They apply here, over the ≤handful
/// of rows one publication number maps to. A page can only UNDER-fill from this
/// (the number is nearly unique, its whole result is one page), never
/// mis-paginate. The bounds read the returned row's own version values, which at
/// the head equal the `current_*` pointer columns the SQL form reads.
fn retain_publication_companions(rows: &mut Vec<TenderRow>, f: &Filter) {
    if f.publication_id.is_none() {
        return;
    }
    rows.retain(|r| {
        f.source.as_ref().is_none_or(|s| r.source == *s)
            && f.kind.as_ref().is_none_or(|k| r.kind == *k)
            && f.published_after.is_none_or(|a| r.published_at >= a)
            && f.published_before.is_none_or(|b| r.published_at < b)
            && f.deadline_after
                .is_none_or(|a| r.deadline.as_ref().is_some_and(|d| d.utc_seconds >= a))
            && f.deadline_before
                .is_none_or(|b| r.deadline.as_ref().is_some_and(|d| d.utc_seconds < b))
    });
}

/// One list row off the [`tender_select_head`] column order — shared by every
/// tender list shape so the mapping cannot drift from the SELECT.
fn tender_row(row: &turso::Row) -> TenderRow {
    TenderRow {
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
        original_lang: opt_text_of(row, 18),
    }
}

/// Which materialised head column an ordered Tender list rides (issue 216): the
/// publication date or the submission deadline. Each is fold-maintained on
/// `tenders` and covered by its own `(column, id)` index, so the range bound, the
/// keyset cursor and the ORDER BY are all one index — no sorter, no walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadOrder {
    PublishedAt,
    Deadline,
}

impl HeadOrder {
    fn column(self) -> &'static str {
        match self {
            HeadOrder::PublishedAt => "t.current_published_at",
            HeadOrder::Deadline => "t.current_deadline",
        }
    }

    /// The bounds this ordering SERVES on its own index — the caller strips
    /// exactly these from the filter before consulting `walks()`, because the
    /// OTHER column's bounds (and every version predicate) still filter the
    /// ordered stream per row and can walk it for sparse values.
    pub fn strip_served(self, filter: &Filter) -> Filter {
        let mut stripped = filter.clone();
        match self {
            HeadOrder::PublishedAt => {
                stripped.published_after = None;
                stripped.published_before = None;
            }
            HeadOrder::Deadline => {
                stripped.deadline_after = None;
                stripped.deadline_before = None;
            }
        }
        stripped
    }
}

/// The ordered Tender list (issue 216): by publication date (newest-first
/// flagship) or by submission deadline ("closes soon"), riding the ordering
/// column's `(column, id)` index end to end. First page 5 ms, deep keyset pages
/// 1-2 ms, measured on prod's real file for the published twin; the deadline twin
/// is the identical shape over `tenders_current_deadline`.
///
/// The keyset cursor is `(key, id)` of the last row, applied in the BOUNDED-OR
/// form — `key <= ?k AND (key < ?k OR id < ?id)` for DESC — because the planner
/// seeks the redundant outer bound and the OR only trims the tie edge. Measured
/// against the alternatives on prod: naive OR 527 ms (no seek), row-value 94 ms;
/// bounded-OR 1-2 ms at any depth.
///
/// Rows whose head lacks the ordering value (`IS NULL`) do not appear — there is
/// nothing truthful to sort them by (award-only tenders have no deadline, say).
/// Companion filters apply exactly as on the id-ordered list: `source`/`kind` and
/// the OTHER column's bounds directly, the org reverse-lookups via the issue-223
/// seed, the rest per row through `version_predicates` — WHICH CAN WALK the
/// ordered stream for sparse values, so the caller must route through `walks()`
/// with [`HeadOrder::strip_served`] applied (the served bounds decide nothing;
/// the REMAINING filters decide isolation).
pub async fn tenders_ordered(
    conn: &Connection,
    filter: &Filter,
    order: HeadOrder,
    desc: bool,
    cursor: Option<(i64, i64)>,
    limit: i64,
) -> turso::Result<Vec<TenderRow>> {
    if !reachable(conn, filter, Collection::Tenders).await? {
        return Ok(Vec::new());
    }
    let filter = &with_country_seed(conn, filter).await?;
    let q = tenders_ordered_query(filter, order, desc, cursor, limit);
    let mut rows = q.rows(conn, tender_row).await?;
    retain_publication_companions(&mut rows, filter);
    Ok(rows)
}

/// The statement [`tenders_ordered`] builds — the same test seam every other
/// list shape exposes (issue 114: assert the artifact, not a paraphrase).
#[doc(hidden)]
pub fn tenders_ordered_statement(
    filter: &Filter,
    order: HeadOrder,
    desc: bool,
    cursor: Option<(i64, i64)>,
    limit: i64,
) -> (String, Vec<Value>) {
    let q = tenders_ordered_query(filter, order, desc, cursor, limit);
    (q.sql, q.params)
}

fn tenders_ordered_query(
    filter: &Filter,
    order: HeadOrder,
    desc: bool,
    cursor: Option<(i64, i64)>,
    limit: i64,
) -> Query {
    let key = order.column();
    let dir = if desc { "DESC" } else { "ASC" };
    // The WINDOW is ids-only: candidates enter the sorter as three integers, and
    // the satellite SELECT list joins back onto the LIMITed page below. With the
    // satellites inline they are evaluated for every WHERE-passing row BEFORE
    // the limit (the sorter materialises full rows), which is where
    // `status=open&country=LU` spent 3 of its 3.6s on prod — wrapped, the same
    // page reads in 0.46s (issue 273, measured 2026-08-24).
    let mut inner = Query::default();
    let (from, seed_param) = tender_from(filter);
    inner.push(
        &format!(
            "SELECT t.id AS wid, v.seq AS wseq, {key} AS wkey
               FROM {from}
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE {key} IS NOT NULL"
        ),
        seed_param,
    );
    // Same flatten hazard as `tenders_query`: a publication seed must be the only
    // `t.`-column predicate — the companions post-filter in Rust.
    if filter.publication_id.is_none() {
        if let Some(after) = filter.published_after {
            inner.push(" AND t.current_published_at >= ?", [Value::Integer(after)]);
        }
        if let Some(before) = filter.published_before {
            inner.push(" AND t.current_published_at < ?", [Value::Integer(before)]);
        }
        if let Some(after) = filter.deadline_after {
            inner.push(" AND t.current_deadline >= ?", [Value::Integer(after)]);
        }
        if let Some(before) = filter.deadline_before {
            inner.push(" AND t.current_deadline < ?", [Value::Integer(before)]);
        }
        if let Some(source) = &filter.source {
            inner.push(" AND t.source = ?", [t(source)]);
        }
        if let Some(kind) = &filter.kind {
            inner.push(" AND t.kind = ?", [t(kind)]);
        }
    }
    version_predicates(&mut inner, filter, "t.id", "v.seq", Some("t.current_deadline"), "t.current_value_eur_cents");
    if let Some((value, id)) = cursor {
        let (outer, tie) = if desc { ("<=", "<") } else { (">=", ">") };
        inner.push(
            &format!(" AND {key} {outer} ? AND ({key} {tie} ? OR t.id {tie} ?)"),
            [Value::Integer(value), Value::Integer(value), Value::Integer(id)],
        );
    }
    inner.push(&format!(" ORDER BY {key} {dir}, t.id {dir} LIMIT ?"), [Value::Integer(limit)]);

    // The outer re-joins `t`/`v` by primary key so `tender_select_head` — the one
    // string that defines WHAT a list row is — applies verbatim; only WHICH rows
    // changed hands. Ordering re-applies the window's own key: the page is ≤limit
    // rows, so this sort is trivial.
    let mut q = Query::default();
    q.push(
        &tender_select_head(&format!("({}) w JOIN tenders t ON t.id = w.wid", inner.sql), filter.lang.as_deref()),
        inner.params,
    );
    q.push(&format!("w.wseq ORDER BY w.wkey {dir}, t.id {dir}"), []);
    q
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

/// The Tender list row's SELECT list + FROM/JOIN, up to (and including) the
/// `v.seq = ` the caller completes with its seq expression. One string shared by
/// the id-ordered [`tenders_query`] and the published-ordered
/// [`tenders_by_published_query`], so the two shapes cannot drift in WHAT a row is
/// — they may only differ in which rows and in what order.
fn tender_select_head(from: &str, lang: Option<&str>) -> String {
    let title = pick(
        "tender_version_texts",
        "value",
        Some("title"),
        // The Tender's own title wins; a lot-only title stands in for the many
        // notices that title their lots and not the procedure (v_tenders).
        // A requested language (ADR-0013 D3) outranks the ENG default; both
        // legs below keep the deterministic tail, so the chain is
        // requested → ENG → any labelled → unlabelled (SQLite sorts NULL
        // below 0 and 1 under DESC).
        &title_rank(lang),
        "1 = 1",
    );
    // Issue 366 unit 3: both picks below now read the SAME ladder the fold's
    // election reads, because until they did, this query contradicted the
    // filters on the very rows the election exists for.
    //
    // `head_deadline`'s doc calls `current_value_eur_cents` "the eur_cents twin
    // of the read layer's OLD `MAX(a.cents)`" — and the word "old" was wishful:
    // the head column replaced the aggregate for the BOUNDS, while this SELECT
    // (shared by every list shape AND the detail payload) kept running the raw
    // extremum for display. So tender 4490098 served `value` €4.97×10¹⁶ from a
    // response whose own `amounts` array carried the €50,000 the fold elected,
    // and 3323836 served `submission_deadline` 3005-07-06 while `status` and
    // `sort=deadline` used the real 2005-06-15.
    //
    // Two different techniques, and the difference is about DRIFT rather than
    // taste — this issue and 343 are both "two places computed one election and
    // disagreed", so a second implementation is the thing to avoid:
    //
    // - The deadline horizon is one arithmetic comparison against one constant,
    //   so it transcribes faithfully and the constant itself is interpolated
    //   from `canonical` rather than retyped.
    // - The amount rule is a DIGIT WALK (`sentinel_amount`) plus a ceiling, and
    //   transcribing that into SQL would be exactly the second implementation.
    //   So the amount pick does not re-derive anything: it looks up the row the
    //   fold already chose, by matching `eur_cents` against the head column.
    //   Zero drift by construction — if the election changes, this follows with
    //   no edit here.
    let deadline = |column| {
        pick(
            "tender_version_dates",
            column,
            Some("submission_deadline"),
            "s.utc_seconds DESC",
            &format!(
                "s.utc_seconds - v.published_at <= {}",
                crate::canonical::DEADLINE_HORIZON_SECS
            ),
        )
    };
    // The published figure the fold elected, in ITS OWN currency. This also
    // repairs an incoherence the old pair had independently of this issue:
    // `MAX(a.cents)` compared raw numbers across currencies, so 1,000,000 HUF
    // outranked 500,000 EUR and the two columns could describe different rows.
    // Matching on `eur_cents` ranks by value, which is what a reader assumes.
    let elected = |column| {
        pick(
            "tender_version_amounts",
            column,
            None,
            // Deterministic among ties: several rows can share one eur_cents.
            "s.cents DESC, s.currency",
            "t.current_value_eur_cents IS NOT NULL
               AND s.eur_cents = t.current_value_eur_cents",
        )
    };
    format!(
        "SELECT t.id, t.source, t.procedure_key, t.kind, v.seq, v.published_at,
                v.publication_id, v.notice_subtype,
                {title},
                {cents},
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
                  WHERE c.tender_id = t.id AND c.seq = v.seq AND c.scheme = 'nuts'),
                v.original_lang
           FROM {from}
           JOIN tender_versions v ON v.tender_id = t.id AND v.seq = ",
        cents = elected("cents"),
        currency = elected("currency"),
        utc = deadline("utc_seconds"),
        offset = deadline("offset_minutes"),
        has_time = deadline("has_time"),
    )
}

/// The seeded FROM clause both tender list shapes share (issue 223): the plain
/// driven table, or the most selective present seed joined to it.
fn tender_from(filter: &Filter) -> (String, Vec<Value>) {
    // issue 217-A: the official notice number drives from `tender_versions.
    // publication_id` (the deferred `tender_versions_publication` index). Unlike
    // the participation seeds this one is EXACT, not a superset — `hits` is
    // precisely "tenders one of whose versions the number caused" — so it doubles
    // as the predicate and no companion EXISTS is emitted for it. That is why it
    // MUST take precedence here: seeded any other way, publication_id would
    // silently stop filtering. A companion winner/bidder/buyer keeps its own
    // EXISTS predicate and narrows as usual.
    if let Some(pub_id) = &filter.publication_id {
        return (
            "(SELECT DISTINCT tender_id FROM tender_versions WHERE publication_id = ?) hits
               JOIN tenders t ON t.id = hits.tender_id"
                .to_owned(),
            vec![t(pub_id)],
        );
    }
    match participation_seed(filter) {
        Some((table, extra, org)) => (
            format!(
                "(SELECT DISTINCT tender_id FROM {table} WHERE organization_id = ?{extra}) hits
                   JOIN tenders t ON t.id = hits.tender_id"
            ),
            vec![Value::Integer(org)],
        ),
        None => match (&filter.country, filter.country_seed) {
            // The sparse-country seed (issue 273 step 2): enumerate the prefix's
            // tenders off the classifications (scheme, code) index — thousands of
            // rows for a sparse country — instead of testing every open-head row.
            // The range form mirrors the 117 guard; NUTS codes are uppercase
            // alphanumeric, so every LIKE-prefix match sorts inside
            // `[prefix, prefix~)` and the seed stays a superset. Measured on prod
            // 2026-08-24: status=open&country=CY 1.8s → 0.05–0.11s, both orders.
            // The seed set must equal what `LIKE prefix%` would admit, and LIKE
            // folds ASCII case while a range does not (the reachability guard's
            // lesson, re-caught by tenders_shortcircuit when this seed first
            // shipped as one case-sensitive range and returned [] for
            // `?country=cy`). Same remedy: the union of every case variant's
            // range IS the LIKE set. `with_country_seed` only sets the flag when
            // `prefix_ranges` accepts the prefix; a None here still falls back
            // to the unseeded FROM rather than seeding wrongly.
            (Some(prefix), true) => match prefix_ranges(prefix) {
                Some(ranges) => {
                    let (hits, params) = country_seed_hits(ranges);
                    (format!("{hits} hits\n                               JOIN tenders t ON t.id = hits.tender_id"), params)
                }
                None => ("tenders t".to_owned(), vec![]),
            },
            _ => ("tenders t".to_owned(), vec![]),
        },
    }
}

/// The sparse-country `hits` set the seeded FROM builders share (issue 273
/// step 2 on tenders; issue 275 ported it to lots — one construction so the
/// two cannot drift). UNION ALL of one range branch per case variant, NEVER
/// one OR'd WHERE: turso serves each branch as an index seek (CY 0.09–0.11s,
/// measured 2026-08-24) but drops the bounds on the OR form and row-filters
/// the whole nuts partition — which resurrected the 30s→503 this seed exists
/// to kill, live, for the ~20 minutes before the rollback.
fn country_seed_hits(ranges: Vec<(String, String)>) -> (String, Vec<Value>) {
    let branches = ranges
        .iter()
        .map(|_| {
            "SELECT tender_id FROM tender_version_classifications
              WHERE scheme = 'nuts' AND code >= ? AND code < ?"
        })
        .collect::<Vec<_>>()
        .join(" UNION ALL ");
    let params = ranges.into_iter().flat_map(|(low, high)| [t(&low), t(&high)]).collect();
    (format!("(SELECT DISTINCT tender_id FROM ({branches}))"), params)
}

/// Should this country prefix drive the read (issue 273 step 2)? A capped count
/// over the classifications (scheme, code) index — ~10 ms warm, bounded at
/// [`COUNTRY_SEED_CAP`] entries for dense prefixes. Under the cap ⇒ the seed
/// enumerates quickly and the read stops paying an EXISTS per deadline-range
/// candidate; at the cap ⇒ dense country, the range shape is already the right
/// drive side. Only consulted when no publication/participation seed outranks it.
///
/// **Raised from 60,000 to 200,000 on 2026-09-17 (issue 408), and the second half
/// of the sentence above is why it had to be.** "At the cap ⇒ dense country ⇒ the
/// range shape is right" reads density over ALL HISTORY off an index that carries
/// no `tender_id`, and uses it as a proxy for density in the id order the fallback
/// walks. For a CURRENT codelist spelling those agree. For a RETIRED one they are
/// opposites, and `?country=GR` — Greece's pre-2013 NUTS spelling — was the proof:
/// 503 after 30.7 s on an idle box, against `EL` at 0.69 s the same minute.
///
/// Measured on prod the same day, uncapped:
///
/// | prefix | entries | at 60,000 | at 200,000 |
/// | --- | --- | --- | --- |
/// | `GR` (pre-2013 Greece) | **87,026** | declined ⇒ walked ⇒ 503 | seeds |
/// | `EL` (current Greece) | **656,330** | declined ⇒ walked ⇒ 0.69 s | declined, unchanged |
///
/// So GR sat just 1.45× over the old cap while EL is 7.5× over any cap in this
/// range: one number admits the value that needs seeding without disturbing the
/// dense one that does not. That is the whole justification — it is NOT a claim
/// that the cap now measures the right thing. It still measures history, and a
/// retired spelling with more than 200,000 entries would fail exactly as GR did.
///
/// **The crossover is unmeasured.** Nobody has established where enumerating
/// costs more than walking; 60,000 was not derived from one either. 200,000 is
/// chosen to clear GR with room and stay far below EL, and the honest test is the
/// served latency of `?country=GR` after this deploys — a single ordinary request,
/// not a characterisation run. If it is still slow, the seed does not pay at 87k
/// and issue 408's option (b), a bounded fallback walk, is required rather than
/// merely preferable.
const COUNTRY_SEED_CAP: i64 = 200_000;
pub async fn country_seed_viable(conn: &Connection, prefix: &str) -> turso::Result<bool> {
    // The same case-variant union the seed itself enumerates; a prefix the
    // range machinery declines cannot be seeded at all.
    let Some(ranges) = prefix_ranges(prefix) else { return Ok(false) };
    let mut total = 0i64;
    for (low, high) in ranges {
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM (
                   SELECT 1 FROM tender_version_classifications
                    WHERE scheme = 'nuts' AND code >= ? AND code < ? LIMIT ?)",
                (t(&low), t(&high), Value::Integer(COUNTRY_SEED_CAP - total)),
            )
            .await?;
        if let Some(row) = rows.next().await? {
            total += int(&row, 0);
        }
        if total >= COUNTRY_SEED_CAP {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Probe-and-set for the async entries: a filter that qualifies for the country
/// seed (country present, no higher-precedence seed) comes back with
/// `country_seed` decided; every other filter passes through unchanged.
async fn with_country_seed(conn: &Connection, filter: &Filter) -> turso::Result<Filter> {
    let mut f = filter.clone();
    f.country_seed = false;
    if let Some(prefix) = &filter.country {
        if filter.publication_id.is_none() && participation_seed(filter).is_none() {
            f.country_seed = country_seed_viable(conn, prefix).await?;
        }
    }
    Ok(f)
}

/// The identity half of [`tenders`], built but not run.
fn tenders_query(filter: &Filter, scope: Scope) -> Query {
    // The paged shape wraps like the ordered list (issue 273 step 1b): an
    // ids-only inner query walks/seeks with the predicates and LIMIT, and the
    // satellite SELECT list joins back onto the ≤limit page rows. Beyond the
    // sorter argument (which does not apply here — id order streams), the wrap
    // lets the planner serve a status head-range off `tenders_current_deadline`
    // and sort the ids, instead of walking the PK testing the range per row.
    // `Scope::At` stays inline: one row by primary key, nothing to bound.
    if matches!(scope, Scope::Page { .. }) {
        return tenders_page_query(filter, scope);
    }
    let mut q = Query::default();
    // issue 223: an org reverse-lookup (winner/buyer/bidder) drives from the
    // participation table's `organization_id` index instead of walking `tenders`.
    // The `hits` set is the org's tenders (a superset of the matches); the untouched
    // `EXISTS` predicates below still decide membership, so the result is identical.
    let (from, seed_param) = tender_from(filter);
    q.push(&tender_select_head(&from, filter.lang.as_deref()), seed_param);
    let seq = seq_expr(scope, "t", &mut q.params);
    q.push(&format!("{seq} WHERE 1 = 1"), []);

    // issue 217-A: with a publication seed, NO `t.`-column predicate may ride
    // along — the planner flattens the compound WHERE onto the companion's index
    // and walks its slice instead of seeking the seed (`source=ted` measured at
    // 35 s vs 3 ms on prod; the notices lesson, repeated on tenders). Those
    // companions post-filter in Rust over the seed's ≤handful of rows
    // (`retain_publication_companions`); the `EXISTS` predicates stay — they have
    // no `tenders` index to flatten onto (winner companion measured 1.9 ms).
    if filter.publication_id.is_none() {
        if let Some(source) = &filter.source {
            q.push(" AND t.source = ?", [t(source)]);
        }
        if let Some(kind) = &filter.kind {
            q.push(" AND t.kind = ?", [t(kind)]);
        }
        // Publication-date bounds (issue 216). In THIS id-ordered shape a narrow range
        // walks the PK to fill its page, so `walks()` isolates it; the REST handler
        // routes range/sorted reads through `tenders_by_published` instead, which rides
        // `tenders_current_published`. This application exists so the filter also means
        // something on the id-ordered paths (SSE snapshots, an explicit `sort=id`).
        if let Some(after) = filter.published_after {
            q.push(" AND t.current_published_at >= ?", [Value::Integer(after)]);
        }
        if let Some(before) = filter.published_before {
            q.push(" AND t.current_published_at < ?", [Value::Integer(before)]);
        }
        if let Some(after) = filter.deadline_after {
            q.push(" AND t.current_deadline >= ?", [Value::Integer(after)]);
        }
        if let Some(before) = filter.deadline_before {
            q.push(" AND t.current_deadline < ?", [Value::Integer(before)]);
        }
    }
    version_predicates(&mut q, filter, "t.id", "v.seq", Some("t.current_deadline"), "t.current_value_eur_cents");
    match scope {
        Scope::Page { after, limit } => q.push(
            " AND t.id > ? ORDER BY t.id LIMIT ?",
            [Value::Integer(after), Value::Integer(limit)],
        ),
        Scope::At { id, .. } => q.push(" AND t.id = ?", [Value::Integer(id)]),
    }
    q
}

/// The `Scope::Page` half of [`tenders_query`], wrapped: predicates and the id
/// cursor bound an ids-only window; `tender_select_head` joins the page by
/// primary key, so WHAT a row is still comes from the one shared string.
fn tenders_page_query(filter: &Filter, scope: Scope) -> Query {
    let Scope::Page { after, limit } = scope else { unreachable!("guarded by the caller") };
    let mut inner = Query::default();
    let (from, seed_param) = tender_from(filter);
    inner.push(
        &format!(
            "SELECT t.id AS wid, v.seq AS wseq FROM {from}
               JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
                    (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
              WHERE 1 = 1"
        ),
        seed_param,
    );
    // Same predicate set, same order, same hazards as the inline shape above —
    // see the issue 217-A comment there for why a publication seed rides alone.
    if filter.publication_id.is_none() {
        if let Some(source) = &filter.source {
            inner.push(" AND t.source = ?", [t(source)]);
        }
        if let Some(kind) = &filter.kind {
            inner.push(" AND t.kind = ?", [t(kind)]);
        }
        if let Some(after) = filter.published_after {
            inner.push(" AND t.current_published_at >= ?", [Value::Integer(after)]);
        }
        if let Some(before) = filter.published_before {
            inner.push(" AND t.current_published_at < ?", [Value::Integer(before)]);
        }
        if let Some(after) = filter.deadline_after {
            inner.push(" AND t.current_deadline >= ?", [Value::Integer(after)]);
        }
        if let Some(before) = filter.deadline_before {
            inner.push(" AND t.current_deadline < ?", [Value::Integer(before)]);
        }
    }
    version_predicates(&mut inner, filter, "t.id", "v.seq", Some("t.current_deadline"), "t.current_value_eur_cents");
    inner.push(
        " AND t.id > ? ORDER BY t.id LIMIT ?",
        [Value::Integer(after), Value::Integer(limit)],
    );

    let mut q = Query::default();
    q.push(
        &tender_select_head(&format!("({}) w JOIN tenders t ON t.id = w.wid", inner.sql), filter.lang.as_deref()),
        inner.params,
    );
    q.push("w.wseq ORDER BY t.id", []);
    q
}

/// A `group_concat` result — a comma-joined code list, or `None` when the
/// version has no codes of that scheme — as a (possibly empty) `Vec`.
fn split_codes(concat: Option<String>) -> Vec<String> {
    concat
        .map(|s| s.split(',').filter(|code| !code.is_empty()).map(str::to_owned).collect())
        .unwrap_or_default()
}

/// The `caused_by_notice_id` of each of a Tender's versions, in `seq` order — the
/// version→notice mapping behind `/v1/notices?tender=X`, and nothing else.
///
/// This is the slice [`tender_detail`] builds from the same `tender_versions` read
/// (see its version block), separated so the notice-list handler stops paying for
/// the ~17 satellite queries — lots, `summarise`, results, parties, amounts — that
/// `tender_detail` also runs and the handler then discards (issue 220). The result
/// is empty exactly when the Tender has no versions; for a projected corpus every
/// real Tender has ≥1, so the caller reads emptiness as its `404`.
pub async fn tender_version_notice_ids(conn: &Connection, tender_id: i64) -> turso::Result<Vec<i64>> {
    let mut rows = conn
        .query(
            "SELECT caused_by_notice_id FROM tender_versions WHERE tender_id = ? ORDER BY seq",
            (Value::Integer(tender_id),),
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(int(&row, 0));
    }
    Ok(out)
}

/// One Tender's full current state: the version chain, the satellites, the
/// parties — everything `/v1/tenders/{id}` answers.
pub async fn tender_detail(
    conn: &Connection,
    id: i64,
    lang: Option<&str>,
) -> turso::Result<Option<TenderDetail>> {
    let filter = Filter { lang: lang.map(str::to_owned), ..Filter::default() };
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

    let mut rows = conn
        .query(&fact("s.cents, s.currency, s.quality", "tender_version_amounts"), key.clone())
        .await?;
    let mut amounts = Vec::new();
    while let Some(row) = rows.next().await? {
        amounts.push(FactRow {
            lot_key: opt_text_of(&row, 0),
            field: text(&row, 1),
            cents: opt_int_of(&row, 2),
            currency: opt_text_of(&row, 3),
            quality: opt_text_of(&row, 4),
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
            "SELECT seq, published_at, dispatched_at, publication_id, notice_subtype, caused_by_notice_id,
                    original_lang
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
            original_lang: opt_text_of(&row, 6),
        });
    }

    let lots = lots_of(conn, id, lang).await?;
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
                        s.decision, s.reason, s.awarded_cents, s.awarded_currency,
                        s.decided_utc, s.decided_offset, s.decided_has_time
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
            decided: stamp(&row, 8),
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
            "SELECT lot_result_id, kind, count, quality FROM tender_version_result_stats
              WHERE tender_id = ? AND seq = ?",
            key.clone(),
        )
        .await?;
    while let Some(row) = rows.next().await? {
        if let Some(&i) = result_index.get(&int(&row, 0)) {
            lot_results[i].statistics.push((text(&row, 1), int(&row, 2), opt_text_of(&row, 3)));
        }
    }

    let mut rows = conn
        .query(
            &format!(
                "SELECT s.bid_id, b.notice_id, b.bid_key, {lot_key}, s.cents, s.currency, s.quality
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
            quality: opt_text_of(&row, 6),
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
                    s.decided_utc, s.decided_offset, s.decided_has_time,
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
            decided: stamp(&row, 6),
            cents: opt_int_of(&row, 9),
            currency: opt_text_of(&row, 10),
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
        quality: None,
    }
}

// ---------------------------------------------------------------------- lots

/// Lots matching `filter`. The tender-level predicates apply through the
/// parent Tender's version, so `/v1/lots?country=DE` means the same thing it
/// does on `/v1/tenders`.
pub async fn lots(conn: &Connection, filter: &Filter, scope: Scope) -> turso::Result<Vec<LotRow>> {
    let mut rows = lots_identity(conn, filter, scope).await?;
    summarise(conn, &mut rows, filter.lang.as_deref()).await?;
    Ok(rows)
}

/// The identity half of [`lots`] — the same matching rows, WITHOUT the per-row
/// summary decoration (title/value/deadline). The SSE diff classifies a change by
/// whether a matching row exists on each side of it and only ever emits the display
/// fields under `?include_data=true` (issue 221), so running [`summarise`] there —
/// which reads the version's WHOLE lot-satellite slice to pick one lot — is pure
/// waste, and quadratic on a fat tender's version bump (one such read per lot
/// change × the whole slice each). This is that classification read.
pub async fn lots_identity(
    conn: &Connection,
    filter: &Filter,
    scope: Scope,
) -> turso::Result<Vec<LotRow>> {
    // Same absent-value short-circuit `tenders()` runs, and for the same reason: every
    // isolation-routed filter on this endpoint (`country`/`cpv`/`buyer`/`winner` via
    // the tender's version, and `kind` as `vl.kind`) walks the isolated pool when its
    // value matches nothing. Before issue 219 `lots` guarded only `kind` and skipped
    // `reachable()` entirely, so `?country=<absent>`/`?buyer=<absent>` were the lots
    // half of the saturation hole (215-D). Folding both entry points through the one
    // probe keeps the guard set and the `walks()` set from drifting apart again — the
    // per-collection `kind` table lives inside `reachable()` now.
    //
    // That last sentence was PROSE until issue 371, and prose is what failed: `currency`
    // was added to `walks()` and never to the probe, so a code present nowhere walked the
    // isolated pool for 29.8 s. The two sets are now the ONE `isolation_routed` list,
    // matched exhaustively in `reachable`, so the next drift is a compile error.
    if matches!(scope, Scope::Page { .. }) && !reachable(conn, filter, Collection::Lots).await? {
        return Ok(Vec::new());
    }
    // Issue 275: decide the sparse-country drive side (issue 273 step 2's
    // probe), Page-scoped only — the At and tender-containment paths route to
    // `lots_query_previous`, which never consults the flag, so probing there
    // would bill the SSE diff ~10ms per classification for nothing.
    let seeded;
    let filter = if matches!(scope, Scope::Page { .. }) && filter.tender.is_none() {
        seeded = with_country_seed(conn, filter).await?;
        &seeded
    } else {
        filter
    };
    lots_query(filter, scope)
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
        .await
}

/// The PREVIOUS stream shape, kept so the equivalence test can compare the shipped
/// read against what it replaced on data where every filter provably discriminates.
///
/// Not dead code: it is the oracle. Deleting it would leave the equivalence assertion
/// with nothing to compare against, and a rewrite of this size wants its predecessor
/// available to answer "did the answer change" for as long as anyone might ask.
#[doc(hidden)]
pub async fn lots_previous_shape(conn: &Connection, filter: &Filter, scope: Scope) -> turso::Result<Vec<LotRow>> {
    let mut rows = lots_query_previous(filter, scope)
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
    summarise(conn, &mut rows, filter.lang.as_deref()).await?;
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
#[doc(hidden)]
pub fn lots_statement(filter: &Filter, scope: Scope) -> (String, Vec<Value>) {
    let q = lots_query(filter, scope);
    (q.sql, q.params)
}

/// The identity half of [`lots`], built but not run.
fn lots_query_previous(filter: &Filter, scope: Scope) -> Query {
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
    version_predicates(&mut q, filter, "t.id", "v.seq", None, "t.current_value_eur_cents");
    match scope {
        Scope::Page { after, limit } => q.push(
            " AND l.id > ? ORDER BY l.id LIMIT ?",
            [Value::Integer(after), Value::Integer(limit)],
        ),
        Scope::At { id, .. } => q.push(" AND l.id = ?", [Value::Integer(id)]),
    }
    q
}

/// **Issue 16 candidate — wired to nothing yet.** The stream-shape `lots` query
/// rebuilt so that `lots` is the ONLY table in the FROM clause.
///
/// Today's shape puts `tender_version_lots` in FROM, and the planner therefore drives
/// from it: `SCAN tender_version_lots` plus a top-level sorter over the whole matched
/// set, which is why `?kind=Lot` — the DEFAULT value — costs 164.6s at prod scale
/// against 0.004s here.
///
/// **The sufficient condition is being the only candidate driver**, not the index
/// available. Measured: an `EXISTS` whose FROM holds only `lots` keeps `lots` driving;
/// the same predicate as a JOIN reverts to the scan even when all three primary-key
/// columns are bindable. Adding `JOIN tenders t` back for `?source=` reverts it too —
/// `SCAN tenders AS t` — so **every** other table access here is a correlated
/// subquery, including the one for `source`.
///
/// **Anyone adding a table to this FROM clause silently undoes the fix.** The symptom
/// is not a wrong answer; it is the old plan returning, which only a plan assertion or
/// a clock will show.
///
/// `seq` is recomputed as `MAX(seq)` rather than read from `tenders.current_seq`.
/// `current_seq` equals it for all 4,262,716 production Tenders today, but that is a
/// projection-MAINTAINED property with no schema constraint behind it. Trusting it
/// would make this read serve a superseded version's `kind` in exactly the state where
/// the data is already wrong — removing a cross-check at the moment it is most needed.
/// Recomputing costs ~1.9x on the sparse band and nothing on the dense path.
/// See issue 27 for enforcing the invariant, after which this may be revisited.
/// Issue 275: the lots-stream seeds, as `l.tender_id IN (…)` predicates — the
/// form turso executes as a semi-join. The JOIN form inverts on lots: measured
/// on prod 2026-08-25, turso drives a `hits JOIN lots … ORDER BY l.id` shape from
/// `lots` to serve the ORDER BY and probes `hits` per row (CY 2.1s via JOIN vs
/// 0.32s via IN; LU+open 7.8s vs 0.48s). The seed set is a candidate SUPERSET
/// (or an exact restatement); the untouched EXISTS/version predicates still
/// decide membership, so results match the unseeded walk exactly.
///
/// Three arms, mutually exclusive, the org seed outranking the country ones:
/// * An org reverse-lookup (issues 223 and 388, the LOTS half): the org's
///   tenders, DECIDED AT THE HEAD VERSION, as `l.tender_id IN (…)`. Two levels —
///   the DISTINCT tenders off the `(organization_id)` index, then one
///   `MAX(seq)` probe per TENDER — and that order is the whole cost story: the
///   same probe written per participation ROW ran 8–10 s on prod for org 357's
///   194k bid rows, and written per LOT (the predicate's own form, which
///   `version_predicates` would add) it visited hundreds of index rows for
///   each of the org's 137k–334k lots: `?bidder=357` walked to the 30 s
///   deadline (503) on prod 2026-09-18 even after the seed had become an IN
///   semi-join, while `?buyer=357` answered in 0.8 s off `(tender_id, seq)`.
///   The org predicates never name the lot (they are per tender), so this seed
///   IS the predicate, exactly — role clause included, at the head level only
///   (`participation_role`) — and `lots_query` drops the per-lot copy for the
///   seeded org. Measured on prod 2026-09-18 through `/v1/sql`: `?bidder=357`'s
///   page 30.6 s → 4.8 s. The remaining cost is enumerating the org's lots and
///   sorting them by id (~2–4 s warm for 334k, I/O-bound cold) plus, for
///   winners, an UNCOVERED pre-seed until `tender_version_result_winners_org_tender`
///   exists on prod — which only a page order the seed can serve, and that
///   index, would remove.
/// * A VIABLE sparse country (`country_seed`, issue 273 step 2's probe):
///   the case-variant UNION ALL enumeration off the classifications index.
///   Measured on prod: `?country=CY` 0.32s against 30s-class walks.
/// * An over-cap country WITH `status=open`: drive from the open head —
///   `t.current_deadline > now` is EXACTLY status-open (273's proven
///   equivalence, `tenders_current_deadline`-indexed, ~38k tenders), with the
///   country prefix tested per TENDER at `t.current_seq` so only matching
///   tenders' lots are ever enumerated. Measured on prod:
///   `?status=open&country=LU` 0.48s against the 30.5s→503 walk this fixes.
///   (`current_seq`/`current_deadline` are the same head pointers the whole
///   tenders endpoint reads; the per-lot predicates still re-decide.)
///
/// Not seeded (recorded in issue 275's residuals): bare `status=open` (fills
/// from the dense walk, 1.1s), over-cap country without status (1.7s), and
/// `cpv`+`status` (no cpv seed anywhere yet).
fn lot_seed_predicates(q: &mut Query, filter: &Filter) {
    if let Some((table, extra, org)) = participation_seed(filter) {
        let role = participation_role(filter);
        q.push(
            &format!(
                " AND l.tender_id IN (SELECT s.tender_id
                                        FROM (SELECT DISTINCT tender_id FROM {table}
                                               WHERE organization_id = ?{extra}) s
                                       WHERE EXISTS (SELECT 1 FROM {table} p
                                                      WHERE p.tender_id = s.tender_id
                                                        AND p.seq = (SELECT MAX(x.seq) FROM tender_versions x
                                                                      WHERE x.tender_id = s.tender_id)
                                                        AND p.organization_id = ?{role}))"
            ),
            [Value::Integer(org), Value::Integer(org)],
        );
        return; // the org seed is the tighter one; the country arms stay out
    }
    match (&filter.country, filter.country_seed) {
        (Some(prefix), true) => {
            if let Some(ranges) = prefix_ranges(prefix) {
                let (hits, params) = country_seed_hits(ranges);
                q.push(&format!(" AND l.tender_id IN {hits}"), params);
            }
        }
        (Some(country), false) if filter.status == Some(Status::Open) => {
            q.push(
                " AND l.tender_id IN (SELECT t.id FROM tenders t
                       WHERE t.current_deadline > ?
                         AND EXISTS (SELECT 1 FROM tender_version_classifications c
                                      WHERE c.tender_id = t.id AND c.seq = t.current_seq
                                        AND c.scheme = 'nuts' AND c.code LIKE ?))",
                vec![Value::Integer(filter.now), t(format!("{country}%"))],
            );
        }
        _ => {}
    }
}

fn lots_query(filter: &Filter, scope: Scope) -> Query {
    let scoped = match scope {
        Scope::Page { .. } => filter.tender,
        Scope::At { .. } => None,
    };
    // The containment shape (issue 115) and the single-lot probe already drive from
    // the right table; only the stream shape is rebuilt here.
    if scoped.is_some() || matches!(scope, Scope::At { .. }) {
        return lots_query_previous(filter, scope);
    }

    // The current version, correlated on the outer lot. Used in the SELECT list, the
    // EXISTS probe and every version predicate, so they cannot disagree about which
    // version they are reading.
    const SEQ: &str = "(SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)";

    // Always `FROM lots l`: every seed is an `l.tender_id IN (…)` predicate
    // (`lot_seed_predicates`), never a JOIN in the FROM clause.
    let mut q = Query::default();
    q.push(
        &format!(
            "SELECT l.id, l.tender_id, l.lot_key,
                    (SELECT vl.kind FROM tender_version_lots vl
                      WHERE vl.tender_id = l.tender_id AND vl.seq = {SEQ}
                        AND vl.lot_id = l.id),
                    {SEQ}
               FROM lots l
              WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                             WHERE vl.tender_id = l.tender_id AND vl.seq = {SEQ}
                               AND vl.lot_id = l.id"
        ),
        [],
    );
    // `kind` filters INSIDE the existence probe: it is a property of the version's lot
    // row, so a lot whose current version does not carry the kind must not match.
    if let Some(kind) = &filter.kind {
        q.push(" AND vl.kind = ?", [t(kind)]);
    }
    q.push(")", []);
    if let Some(source) = &filter.source {
        q.push(" AND (SELECT tt.source FROM tenders tt WHERE tt.id = l.tender_id) = ?", [t(source)]);
    }
    lot_seed_predicates(&mut q, filter);
    // Issue 388: the seeded org filter is decided by the seed, at the head
    // version, exactly — so its per-lot EXISTS (which never named the lot and
    // cost hundreds of index rows per lot on a prolific org) is not added again.
    // Only the seeded one: a second org filter alongside it still decides per lot.
    let mut predicates = filter.clone();
    match participation_seed(filter) {
        Some(("tender_version_result_winners", ..)) => predicates.winner = None,
        Some(("tender_version_bid_parties", ..)) => predicates.bidder = None,
        Some(("tender_version_parties", ..)) => predicates.buyer = None,
        _ => {}
    }
    version_predicates(&mut q, &predicates, "l.tender_id", SEQ, None,
        "(SELECT tt.current_value_eur_cents FROM tenders tt WHERE tt.id = l.tender_id)");
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
///   * title — the `title_rank` ladder: a requested language, then ENG, then the
///     version's original language (ADR-0013 D3), then any other language, then an
///     unlabelled row; ties keep the first in scan order. SQLite sorts NULL below
///     both 0 and 1 under DESC, which is what puts unlabelled last.
///   * value and currency — `MAX(cents)` and `ORDER BY cents DESC LIMIT 1` resolve
///     to the SAME row, so one max-cents row serves both. Over the CANDIDATES,
///     which since issue 389 are the rows the fold would also have accepted: not
///     withheld (issue 372), not a sentinel, and under the ceiling where a EUR
///     conversion exists to measure it against.
///   * deadline — the three columns were three subqueries sharing
///     `ORDER BY utc_seconds DESC LIMIT 1`, so one max-utc row serves all three.
async fn summarise(conn: &Connection, rows: &mut [LotRow], lang: Option<&str>) -> turso::Result<()> {
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

        // ADR-0013 D3's third leg: the version's original language, one PK seek,
        // so the in-memory rank below can honour it exactly as the SQL
        // `title_rank` does.
        let original: Option<String> = {
            let mut got = conn
                .query(
                    "SELECT original_lang FROM tender_versions WHERE tender_id = ? AND seq = ?",
                    key.clone(),
                )
                .await?;
            match got.next().await? {
                Some(row) => opt_text_of(&row, 0),
                None => None,
            }
        };

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
            // ADR-0013 D3, all four legs: requested → ENG → the version's
            // original language → any labelled → unlabelled. Strict `>` keeps
            // first-seen winning ties — the same stability the SQL `LIMIT 1`
            // oracle pins. A version with no recorded original never matches the
            // third leg, so it ranks exactly as before the column existed.
            let rank = match opt_text_of(&row, 1).as_deref() {
                l if lang.is_some() && l == lang => 4,
                Some("ENG") => 3,
                l if l.is_some() && l == original.as_deref() => 2,
                Some(_) => 1,
                None => 0,
            };
            let value = opt_text_of(&row, 2);
            // Among equal ranks the smallest value wins — the SQL's `s.value`
            // tail and the fold's `head_title`, spelled the same way (issue 343)
            // — rather than whichever row the scan produced first.
            let better = match best_title[i] {
                None => true,
                Some(best) => rank > best || (rank == best && value < rows[i].title),
            };
            if better {
                best_title[i] = Some(rank);
                rows[i].title = value;
            }
        }

        let mut got = conn
            .query(
                // Issue 372: a withheld figure is not a candidate. Without this a
                // lot whose ONLY amount is withheld would show -0.01 as its
                // headline value: -100 outranks the NULL default below, so it
                // wins by being the only row rather than by being a figure.
                "SELECT s.lot_id, s.cents, s.currency, s.eur_cents FROM tender_version_amounts s
                  WHERE s.tender_id = ? AND s.seq = ? AND s.lot_id IS NOT NULL
                    AND s.quality IS NULL",
                key.clone(),
            )
            .await?;
        while let Some(row) = got.next().await? {
            let Some(&i) = opt_int_of(&row, 0).and_then(|id| at.get(&(tender_id, seq, id))) else { continue };
            let cents = opt_int_of(&row, 1);
            // Issue 389 unit 1: the fold's own predicate, CALLED rather than
            // transcribed. `tender_select_head` warns that walking the digits in
            // SQL would be the second implementation this whole class of bug is
            // made of, and solves it by looking up the row the fold chose via
            // `eur_cents = t.current_value_eur_cents`. That trick is unavailable
            // here — the head column is tender-scoped and no per-LOT twin exists
            // — but `summarise` is Rust, so the function itself is in reach and
            // there is still only one rule.
            //
            // What it fixes: tender 25773 served `value: null` (the head refuses
            // an exact zero since issue 366's `55d239d`) in the same response
            // that priced its only lot at `{cents: 0, currency: "GBP"}`, from the
            // same published figure — and `?tender=25773&max_value=0` returned
            // nothing, because the FILTER reads the head column while this pick
            // re-derived. 1,280 lots over ids 1–100,000 were served that way.
            if cents.is_some_and(crate::canonical::sentinel_amount) {
                continue;
            }
            // The ceiling is defined on the EUR conversion, so it is applied
            // where that conversion exists and nowhere else. An unconvertible
            // amount is NOT refused: unlike the tender head column, which is a
            // EUR figure and so has nothing to say without a rate, the lot row
            // serves the PUBLISHED figure, and blanking a published amount for
            // want of a rate would be a new defect rather than this one's fix.
            if opt_int_of(&row, 3).is_some_and(|eur| eur > crate::canonical::IMPLAUSIBLE_EUR_CENTS) {
                continue;
            }
            // A NULL sorts last under `cents DESC` and is ignored by `MAX`, so it
            // ranks below every real amount rather than above them.
            let rank = cents.unwrap_or(i64::MIN);
            if best_value[i].is_none_or(|best| rank > best) {
                best_value[i] = Some(rank);
                rows[i].value_cents = cents;
                rows[i].currency = opt_text_of(&row, 2);
            }
        }

        // Issue 389 unit 2: a LOT-scoped deadline still wins, and a tender-scoped
        // one is now the FALLBACK rather than nothing — so `AND s.lot_id IS NOT
        // NULL` is gone from the WHERE and decided per row below.
        //
        // The `status=open` filter on `/v1/lots` runs `version_predicates`, whose
        // EXISTS carries NO `lot_id` term: a procedure-scoped deadline opens every
        // lot of the procedure. Issue 275 pins that form deliberately, on 273's
        // status ≡ head-range equivalence, so the filter is not the thing to
        // change. The row field, though, read lot-scoped rows only — so 73 of 73
        // open-with-lots tenders over ids 8,436,069–8,536,069 were RETURNED as open
        // while serving `submission_deadline: null`, a page a bidder-facing client
        // can neither sort by nor display a date for.
        //
        // There is no lot-level date to find in those: they are r209-era, whose
        // form-section `DATE_RECEIPT_TENDERS` is procedure-level BY DESIGN, and the
        // projection stored it faithfully with `lot_id NULL`. The deadline that
        // decided `open` IS the tender's, and a bidder submitting for LOT-1 of
        // 8436333 submits by it. Inheritance in this direction is not new — it is
        // the mirror of what already happens upward, where `head_deadline` takes
        // MAX over the version with lot rows INCLUDED and
        // `crates/store/tests/deadline_backfill.rs` asserts "a lot-level deadline
        // counts". What was missing was the other half.
        let mut inherited: Option<Stamp> = None;
        let mut got = conn
            .query(
                "SELECT s.lot_id, s.utc_seconds, s.offset_minutes, s.has_time
                   FROM tender_version_dates s
                  WHERE s.tender_id = ? AND s.seq = ? AND s.field = 'submission_deadline'",
                key,
            )
            .await?;
        while let Some(row) = got.next().await? {
            match opt_int_of(&row, 0) {
                // Lot-scoped: this lot's own date, latest wins — unchanged.
                Some(lot_id) => {
                    let Some(&i) = at.get(&(tender_id, seq, lot_id)) else { continue };
                    let rank = opt_int_of(&row, 1).unwrap_or(i64::MIN);
                    if best_deadline[i].is_none_or(|best| rank > best) {
                        best_deadline[i] = Some(rank);
                        rows[i].deadline = stamp(&row, 1);
                    }
                }
                // Tender-scoped: it belongs to EVERY lot of this version, so it is
                // held here and applied after the scan to the lots that published
                // none of their own. MAX, the extremum `head_deadline` takes.
                None => {
                    if let Some(c) = stamp(&row, 1) {
                        if inherited.is_none_or(|held| c.utc_seconds > held.utc_seconds) {
                            inherited = Some(c);
                        }
                    }
                }
            }
        }
        if let Some(fallback) = inherited {
            for (i, row) in rows.iter_mut().enumerate() {
                // `best_deadline` is set only by the lot-scoped arm, so `None` here
                // means exactly "this lot published no deadline of its own".
                if row.tender_id == tender_id && row.seq == seq && best_deadline[i].is_none() {
                    row.deadline = Some(fallback);
                }
            }
        }
    }
    Ok(())
}

/// Every Lot of one Tender. A Tender's lots are a BOUNDED set, so this asks for the
/// whole of it rather than a page: at `MAX_PAGE` the detail response silently
/// truncated the 16 Tenders (of 4.26M) that carry more than 1,000 lots — reporting
/// `"lots": 2604` while shipping 1,000 `lot_details`, a response that contradicted
/// itself about its own data (issue 116).
async fn lots_of(conn: &Connection, tender_id: i64, lang: Option<&str>) -> turso::Result<Vec<LotRow>> {
    let filter =
        Filter { tender: Some(tender_id), lang: lang.map(str::to_owned), ..Filter::default() };
    lots(conn, &filter, Scope::Page { after: 0, limit: TENDER_LOTS_CAP }).await
}

/// The hard ceiling on any one page — a client asking for more gets this.
pub const MAX_PAGE: i64 = 1000;

/// The ceiling on a tender-scoped lots read. Not a page size: a Tender's lot count
/// is a bounded real-world quantity (whole-corpus maximum 2,604, measured on the
/// post-refold snapshot), so this is a sanity bound that must sit comfortably above
/// the true maximum — never a value the data is expected to reach. It exists so a
/// corrupt `tender_id` cannot turn one read into an unbounded scan, not to paginate.
/// Serving the whole set costs 0.4% more than serving a truncated page (issue 116's
/// measurement): the containment shape reads only the Tender's own slice, so the
/// limit bounds nothing on the happy path.
const TENDER_LOTS_CAP: i64 = 20_000;

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
    // The official identifier value (issue 217), served by `organizations_identifier_id
    // (identifier, id)` so `WHERE identifier=? AND id>? ORDER BY id LIMIT` seeks and the
    // cursor rides the same index — present OR absent, both O(log n), no sorter. Pair
    // with `kind` above to pin the scheme; alone it returns every scheme's match.
    if let Some(identifier) = &filter.identifier {
        q.push(" AND o.identifier = ?", [t(identifier)]);
    }
    // The id-ordered application of the name prefix (issue 217-B): correct but
    // walk-shaped (walks() isolates it); the REST handler uses the name-ordered
    // `organizations_by_name` instead. Range over the normalised column; the
    // upper bound is the prefix's successor, or unbounded for a 0xFF-tail edge.
    if let Some(prefix) = &filter.name_prefix {
        q.push(" AND o.name_norm >= ?", [t(prefix)]);
        if let Some(hi) = successor(prefix) {
            q.push(" AND o.name_norm < ?", [t(&hi)]);
        }
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

/// The name-ordered organization search (issue 217-B): every org whose
/// Unicode-lowercased name starts with `prefix` (itself already lowercased), in
/// `(name_norm, id)` order, riding `organizations_name_norm_id` end to end —
/// prefix range, keyset cursor and ORDER BY on one index (the issue-216 shape;
/// the probe measured the plain-index seek at 3.3 ms where every NOCASE shape
/// scanned). `country`/`kind` filter per row within the prefix slice. The cursor
/// is the last row's `(name_norm, id)`, applied bounded-OR.
pub async fn organizations_by_name(
    conn: &Connection,
    filter: &Filter,
    prefix: &str,
    cursor: Option<(String, i64)>,
    limit: i64,
) -> turso::Result<Vec<OrganizationRow>> {
    let mut q = Query::default();
    q.push(
        "SELECT o.id, o.name, o.country, o.identifier_kind, o.identifier, o.provisional,
                (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id)
           FROM organizations o WHERE o.name_norm >= ?",
        [t(prefix)],
    );
    if let Some(hi) = successor(prefix) {
        q.push(" AND o.name_norm < ?", [t(&hi)]);
    }
    if let Some(country) = &filter.country {
        q.push(" AND o.country = ?", [t(country)]);
    }
    if let Some(kind) = &filter.kind {
        q.push(" AND o.identifier_kind = ?", [t(kind)]);
    }
    // Issue 284: `identifier` and `buyer` are both in Organizations.honoured_params,
    // so the handler reports them as applied — but the name-ordered builder used to
    // apply only country/kind, silently widening the result while `ignored_filters`
    // stayed empty. Apply them exactly as `organizations_query` does, so the two org
    // paths agree on what the honoured set means (pinned by the equivalence test).
    if let Some(identifier) = &filter.identifier {
        q.push(" AND o.identifier = ?", [t(identifier)]);
    }
    if let Some(buyer) = filter.buyer {
        q.push(" AND o.id = ?", [Value::Integer(buyer)]);
    }
    if let Some((norm, id)) = cursor {
        q.push(
            " AND o.name_norm >= ? AND (o.name_norm > ? OR o.id > ?)",
            [t(&norm), t(&norm), Value::Integer(id)],
        );
    }
    q.push(" ORDER BY o.name_norm, o.id LIMIT ?", [Value::Integer(limit)]);
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
    let q = notices_query(filter, scope);
    let mut rows = q
        .rows(conn, |row| NoticeRow {
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
        .await?;
    // A publication_id read seeks its own index and leaves `source`/`kind` out of
    // the SQL (see notices_query) — apply them here, over the ≤handful of rows one
    // publication number maps to. A page can only UNDER-fill from this (the number
    // is nearly unique), never mis-paginate: the cursor advances on row ids the
    // seek actually returned.
    if filter.publication_id.is_some() {
        if let Some(source) = &filter.source {
            rows.retain(|r| r.source == *source);
        }
        if let Some(kind) = &filter.kind {
            rows.retain(|r| r.profile == *kind);
        }
    }
    Ok(rows)
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
    // The official notice number (issue 217-A). When present it is the ONLY
    // identity predicate emitted in SQL: `notices_publication_id_id (publication_id,
    // id)` then serves the whole read as a seek + cursor ride (measured 1 ms against
    // prod, absent 0.8 ms). Emitting `source`/`profile` ALONGSIDE it lets the planner
    // flatten and drive from THEIR indexes instead, filtering publication_id per row —
    // a 7.8 s walk over the source's slice, measured; the FROM-subquery and IN-seed
    // shapes that fix this on two-table reads (issue 223) get flattened here because
    // both sides are `notices`. So companion filters are applied by [`notices`] in
    // Rust, over the ≤handful of rows one publication number maps to — semantically
    // identical, deterministically fast, no planner statistics involved.
    if let Some(publication_id) = &filter.publication_id {
        q.push(" AND publication_id = ?", [t(publication_id)]);
    } else {
        if let Some(source) = &filter.source {
            q.push(" AND source = ?", [t(source)]);
        }
        if let Some(kind) = &filter.kind {
            q.push(" AND profile = ?", [t(kind)]);
        }
    }
    match scope {
        Scope::Page { after, limit } => q.push(
            " AND id > ? ORDER BY id LIMIT ?",
            [Value::Integer(after), Value::Integer(limit)],
        ),
        Scope::At { id, .. } => q.push(" AND id = ?", [Value::Integer(id)]),
    }    q
}

/// The quarantine row for one notice — its hold HISTORY, whatever outcome that
/// hold reached (issue 218; meaning settled by issue 398). Seeks
/// `quarantine_notice_id` — a bounded `(notice_id)` lookup, never a scan. A notice
/// maps to at most one member, so at most one row; the newest by `first_seen` wins
/// if a re-ingest ever produced more.
///
/// **`Some` does NOT mean the notice is held today**, and reading it that way is
/// wrong for the large majority of the rows it returns. The ledger retains the row
/// after the member is reclaimed — deliberately, so the reclaim campaign stays
/// auditable (issues 40/76/84/137) — and reclaimed is the DOMINANT outcome:
/// 1,734,594 rows, 71.7 %, on 2026-08-05, and 11,737 of 11,766 in a spot-checked
/// id band, every one of them joined to a notice whose `parse_state` is `parsed`.
/// Read the terminal stamps: `reprocessed_at` set means reclaimed and its content
/// is served; `skipped_at` set means resolved as a policy skip and still out of the
/// corpus; both null means outstanding. `None` means the notice was NEVER held.
///
/// The filter that would make `Some` mean "held today" is deliberately absent: it
/// would leave no REST path to a reclaim record at all, and the record is the point.
/// `parse_state` is the held-today predicate.
pub async fn notice_quarantine(
    conn: &Connection,
    notice_id: i64,
) -> turso::Result<Option<QuarantineRow>> {
    let mut rows = conn
        .query(
            "SELECT reason, detail, profile, first_seen, attempts, last_attempt_at,
                    reprocessed_at, skipped_at, skipped_reason, first_reason, first_detail
               FROM quarantine WHERE notice_id = ? ORDER BY first_seen DESC LIMIT 1",
            [Value::Integer(notice_id)],
        )
        .await?;
    let Some(row) = rows.next().await? else { return Ok(None) };
    Ok(Some(QuarantineRow {
        reason: text(&row, 0),
        detail: opt_text_of(&row, 1),
        profile: opt_text_of(&row, 2),
        first_seen: int(&row, 3),
        attempts: opt_int_of(&row, 4),
        last_attempt_at: opt_int_of(&row, 5),
        reprocessed_at: opt_int_of(&row, 6),
        skipped_at: opt_int_of(&row, 7),
        skipped_reason: opt_text_of(&row, 8),
        first_reason: opt_text_of(&row, 9),
        first_detail: opt_text_of(&row, 10),
    }))
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

/// The change feed's generation (issue 46): bumped by every wipe of the
/// canonical layer or the change log. Events from different generations do not
/// compose — a client that sees this move must drop state and re-snapshot.
/// A database created before the table existed reads as generation 1, the
/// schema's seed value.
pub async fn feed_generation(conn: &Connection) -> turso::Result<i64> {
    let mut rows = conn.query("SELECT generation FROM feed_generation WHERE id = 0", ()).await?;
    Ok(match rows.next().await? {
        Some(row) => int(&row, 0),
        None => 1,
    })
}

/// The head (newest) version seq of one Tender, or `None` when it has no
/// versions (retired/absent). The SSE diff's probe point for a seq-less
/// in-place `changed` row (issue 287): the write repointed CURRENT rows, so
/// current state is the only side left to evaluate. Bounded: the PK is
/// `(tender_id, seq)`, and even a scan of the slice is one Tender's chain —
/// a handful of rows (deliberately NOT `MAX(seq)`, which turso 0.7 does not
/// short-circuit; see `oldest_cursor`).
pub async fn tender_head_seq(conn: &Connection, tender_id: i64) -> turso::Result<Option<i64>> {
    let mut rows = conn
        .query(
            "SELECT seq FROM tender_versions WHERE tender_id = ? ORDER BY seq DESC LIMIT 1",
            (Value::Integer(tender_id),),
        )
        .await?;
    Ok(rows.next().await?.map(|row| int(&row, 0)))
}

/// The head version seq of the Tender owning one lot, or `None` when the lot
/// (or its Tender's chain) is gone. Companion to [`tender_head_seq`] for lot
/// rows on the same seq-less diff arm.
pub async fn lot_head_seq(conn: &Connection, lot_id: i64) -> turso::Result<Option<i64>> {
    let mut rows = conn
        .query(
            "SELECT v.seq FROM lots l JOIN tender_versions v ON v.tender_id = l.tender_id \
              WHERE l.id = ? ORDER BY v.seq DESC LIMIT 1",
            (Value::Integer(lot_id),),
        )
        .await?;
    Ok(rows.next().await?.map(|row| int(&row, 0)))
}

fn stamp(row: &turso::Row, idx: usize) -> Option<Stamp> {
    Some(Stamp {
        utc_seconds: opt_int_of(row, idx)?,
        offset_minutes: opt_int_of(row, idx + 1).unwrap_or(0),
        has_time: opt_int_of(row, idx + 2).unwrap_or(0) != 0,
    })
}

#[cfg(test)]
mod country_seed_cap_tests {
    use super::COUNTRY_SEED_CAP;

    /// Issue 408: the cap must admit the value that needs seeding and still
    /// decline the dense one that does not.
    ///
    /// Both numbers are MEASURED on prod (2026-09-17), uncapped, and they are what
    /// makes this constant a decision rather than a guess:
    ///
    /// * `GR` — Greece's pre-2013 NUTS spelling — carries **87,026** classification
    ///   entries. Under the old 60,000 cap it declined the seed, fell back to the
    ///   id-ordered walk, and `?country=GR` returned **503 after 30.7 s** on an idle
    ///   box. It is the retired spelling of a member state, not an exotic probe
    ///   value, and every pre-2013 notice in the corpus carries it.
    /// * `EL` — current Greece — carries **656,330**, is dense at the head of the id
    ///   order, and answers in **0.69 s** through the plain range walk. It must keep
    ///   declining: seeding it is the cost this cap exists to avoid.
    ///
    /// So the constant has to sit strictly between them. This test is the thing
    /// that fails if someone restores 60,000 — the old value looks defensible right
    /// up until you know GR sits 1.45x above it.
    ///
    /// It deliberately does NOT assert the cap is the right KIND of measurement. It
    /// is not: it counts entries over all history off an index carrying no
    /// `tender_id`, and a retired spelling above 200,000 would fail exactly as GR
    /// did. That is issue 408's option (b), a bounded fallback walk, and this test
    /// is not a substitute for it.
    #[test]
    fn the_cap_admits_a_retired_spelling_and_still_declines_a_dense_one() {
        const GR_ENTRIES: i64 = 87_026;
        const EL_ENTRIES: i64 = 656_330;
        assert!(
            COUNTRY_SEED_CAP > GR_ENTRIES,
            "GR ({GR_ENTRIES}) must SEED — under a cap of {COUNTRY_SEED_CAP} it falls back to \
             the id walk, which is the 30.7 s / 503 this issue is about"
        );
        assert!(
            COUNTRY_SEED_CAP < EL_ENTRIES,
            "EL ({EL_ENTRIES}) must keep DECLINING — it is dense at the head and the range walk \
             already answers it in 0.69 s; seeding it is the cost the cap exists to avoid"
        );
    }
}
