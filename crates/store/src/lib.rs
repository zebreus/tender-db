//! Turso (pure-Rust SQLite) persistence — server-only by construction (the app
//! crate pulls this in behind its `server` feature; it never reaches wasm).
//!
//! One embedded database file (`TENDER_DB`, default `tender-db.db` in the working
//! directory — the systemd state dir in production). Accessors live on [`Db`];
//! the process-wide instance owns a single connection behind a mutex and
//! serialises access.

pub mod accounts;
pub mod canonical;
pub mod checkpoint;
pub mod jobs;
pub mod rates;
pub mod read;
pub mod webhooks;

/// Re-exported so callers can name `Error`/`Connection`/`Value` without taking
/// their own pin on the engine — the store owns which Turso this is.
pub use turso;

pub use accounts::{TokenRecord, User};
pub use checkpoint::{Checkpointed, CheckpointMode};
pub use canonical::{
    Applied, BidParty, BidState, Change, ContractState, Fact, Identifier, LayerPresence, LayerState,
    register_jurisdiction,
    LotResultState, LotState, Mention, MentionResolver, NestedOrgRepair, NoticeRef, OrgDissolve, OrgMergeBatch, OrgNameBackfill,
    CaseApplyReport, CaseBacklogReport, CaseBacklogRow, CaseReview, CaseUnapplyReport,
    ClusterCase, ClusterPacket, CountryFoldReport, CountryMove, CountryVerdict,
    CountryVerdictReport, FusionCandidate, MergeVerdict,
    FusionReport, RehomingReport, RehomingVerdict,
    RehomingCase, RehomingGroup, RehomingMention, RehomingPacket, RehomingParked, RehomingTarget,
    OrphanSatellite, SatelliteOrphanReport,
    SatelliteDropReport, SatelliteRestoreReport,
    AnchorWallReport, WallGapOwner,
    WallCounts,
    XbCase, XbMember, XbPacket,
    EdgeCensusReport, MatchKeyBuildWindow, OrgEdgeScanArgs,
    OrgEdgeScanReport,
    PlanGroup, PlanRow, QUALITY_WITHHELD, R2MergeArgs,
    R2MergeReport, R3MergeArgs, R3MergeReport, Round, TenderProjection, TenderVersion,
    CountryCluster, CountryClusterReport, CountryTypoMove, CountryTypoRepairReport,
    DuplicateIdentity, GenericKeyProbe, ProvisionalEchoGroup, ProvisionalEchoReport, ProvisionalFoldArgs, ProvisionalFoldReport, EchoTier, NameVerdict, DuplicateIdentityReport,
    LabelFix, LabelRepairReport, RenormaliseRepairReport,
    NoticeInstantFix, NoticeInstantRepairReport,
    GenericKeyShape, GenericStatisticReport,
    GenericWallReport, InflatedKey,
    GhostCensusReport, GhostNotice,
    NameAttribution, NameAttributionReport,
    AddressStrip, NamePollutionReport, PollutedName,
    MintedCountryFix, MintedCountryReport,
    EDGE_VOLUME_CEILING, LUHN_FAMILY, MIN_CLUSTER_IDENTIFIER, R2_PLAN_LISTING_CAP,
    REPORT_HISTORY_DEPTH,
    SCAN_KEY_WINDOW,
    TYPO_MOVE_MENTION_VETO,
};
pub use jobs::QueuedJobRow;
pub use read::{Filter, Reader, Readers, Status};
pub use webhooks::{Delivery, Endpoint};

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{Mutex, MutexGuard, OnceCell, watch};
use turso::{Connection, Value};

/// Sane connection defaults, per <https://mort.coffee/home/sqlite-editions/>:
/// enforce foreign keys, retry on lock contention instead of failing with
/// SQLITE_BUSY, WAL for concurrent reads during writes, and NORMAL sync (safe
/// under WAL, much faster than FULL).
///
pub(crate) const PRAGMAS: [&str; 4] = [
    "PRAGMA foreign_keys = ON",
    "PRAGMA busy_timeout = 5000",
    "PRAGMA journal_mode = WAL",
    "PRAGMA synchronous = NORMAL",
];

/// Default per-connection page cache: 128 MiB (the pragma value is negative KiB),
/// up from turso's ~2 MB default (issue 60). It is a per-connection *cap* that
/// fills lazily, but it is charged PER CONNECTION, and the process opens many:
/// the writer + the store read pool (8) + the API/SQL/webhook pools (~14) + the
/// Phase-2 parallel pre-pass's K shard readers. At the old 512 MiB the aggregate
/// CEILING was ~11.5 GiB on the 8 GB box, and during a full-corpus rebuild the
/// ACTIVE fillers (writer + K pre-pass readers, each sweeping the corpus) grew
/// toward 512 MiB apiece and tipped the box into swap-thrash (issue 61 incident:
/// RSS 4.1 G + 4.2 G swapped). 128 MiB bounds that while still dwarfing turso's
/// default: point queries and sequential scans (which ride the kernel's per-fd
/// readahead, not this cache) are unaffected on our hot paths. The pass that DOES
/// measurably want more is a projection Phase-1/pre-pass (~11 indexed range
/// scans per chunk, issue 68) — which is why the value is an ops valve now.
const CACHE_KIB_DEFAULT: u64 = 131_072;

/// The page-cache pragma, sized by the `TENDER_CACHE_KIB` env valve (issue 175):
/// the right cache is a property of the BOX, not the build — 128 MiB per
/// connection protects an 8 GB machine, and starves a 64 GB one. Positive KiB,
/// clamped to [1 MiB, 4 GiB]; unset/unparseable → the 128 MiB default. Sizing
/// rule: ceiling ≈ (23 + pre-pass workers) × value; the ACTIVE set during a fold
/// is writer + K pre-pass readers. On the 64 GB / 32-core prod, 524288 (512 MiB)
/// gives the fold ~16 GiB of active cache with ample headroom.
pub(crate) fn cache_pragma() -> String {
    format!("PRAGMA cache_size = -{}", cache_kib(std::env::var("TENDER_CACHE_KIB").ok().as_deref()))
}

/// Parse + clamp the valve (pure, so it is testable without process-global env).
fn cache_kib(env: Option<&str>) -> u64 {
    env.and_then(|v| v.trim().parse::<u64>().ok())
        .map_or(CACHE_KIB_DEFAULT, |k| k.clamp(1_024, 4_194_304))
}

/// Reader connections backing `Db`'s own read-only accessors — the dashboard,
/// admin, auth, webhook and projection reads. Kept apart from the public API's
/// pool (`Db::readers`) and from the single writer.
const READ_POOL: usize = 8;

/// Schema, applied idempotently at startup. STRICT so columns actually enforce
/// their declared types.
const SCHEMA: &str = "
    -- Raw-fetch registry (docs/architecture.md 'Storage layout'). One row per
    -- downloaded file version; the newest row per (source, kind, period) is
    -- current. Files themselves are immutable under the archive root — a
    -- changed upstream package lands as a NEW row + file, never a rewrite.
    CREATE TABLE IF NOT EXISTS fetches (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        source     TEXT NOT NULL,
        kind       TEXT NOT NULL,
        period     TEXT NOT NULL,
        url        TEXT NOT NULL,
        sha256     TEXT NOT NULL,
        bytes      INTEGER NOT NULL,
        fetched_at INTEGER NOT NULL, -- unix seconds
        path       TEXT NOT NULL
    ) STRICT;
    CREATE INDEX IF NOT EXISTS fetches_period ON fetches(source, kind, period);

    -- One row per Notice: a single publication event at a Source (CONTEXT.md).
    -- Identity is the source's publication identity plus a content hash;
    -- `declared_version` (BT-757 / TED_EXPORT VERSION) is advisory only, as real
    -- TED chains have gaps and cross-type sequences. The payload is NOT stored:
    -- (fetch_id, member_path) locates it inside the immutable raw archive.
    CREATE TABLE IF NOT EXISTS notices (
        id               INTEGER PRIMARY KEY AUTOINCREMENT,
        source           TEXT NOT NULL,
        publication_id   TEXT NOT NULL,
        content_hash     TEXT NOT NULL,
        profile          TEXT NOT NULL,
        declared_version TEXT,
        fetch_id         INTEGER NOT NULL REFERENCES fetches(id),
        member_path      TEXT NOT NULL,
        ingested_at      INTEGER NOT NULL, -- unix seconds
        -- The publication event's own dates, resolved per era at process time
        -- (issue 18): `published_at` the OJ/portal publication date, `dispatched_at`
        -- the send date. Null until the payload is parsed (identity-only rows).
        published_at     INTEGER,
        dispatched_at    INTEGER,
        -- Field mapping state (ADR-0004): 'parsed' once the profile's parser
        -- consumed the payload exhaustively, 'quarantined' when it could not,
        -- 'pending' for profiles whose parser does not exist yet.
        parse_state      TEXT NOT NULL DEFAULT 'pending',
        -- The incremental-projection watermark (issue 58): 0 = this notice's
        -- parsed layer has NOT been folded into the canonical layer since it was
        -- last (re)parsed; 1 = it has. New rows default 0; `set_parse_state`
        -- clears it to 0 on every transition to 'parsed' (the choke-point for
        -- (re)parsing, so a future in-place re-parse path stays covered); Phase 2
        -- sets it to 1 for every notice it applies; a full rebuild resets all to
        -- 0. The daily incremental change-set is exactly
        -- `parse_state='parsed' AND projected=0`.
        projected        INTEGER NOT NULL DEFAULT 0,
        UNIQUE(source, publication_id, content_hash)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS notices_profile ON notices(profile);
    -- The `notices_unprojected` partial index (issue 58) is created in `migrate`,
    -- not here: on an existing DB the `projected` column is added by ALTER after
    -- this schema batch runs, so the index — which references it — cannot exist
    -- until then.
    CREATE INDEX IF NOT EXISTS notices_parse_state ON notices(parse_state);
    -- notices.fetch_id is a foreign key that was unindexed (issue 20 reopened):
    -- any query seeking notices by their fetch — and the old join-based coverage
    -- query's inner side — had to full-scan. Idempotent CREATE INDEX (not the
    -- ALTER-only MIGRATIONS list); on an existing 3.5M-row prod table the first
    -- open after this change builds it once (tens of seconds; the process/query
    -- rewrite is the actual DoS fix, this is FK-hygiene insurance).
    CREATE INDEX IF NOT EXISTS notices_fetch_id ON notices(fetch_id);

    -- The only failure mode of ingestion (ADR-0004): a notice with unmapped or
    -- unrecognised content is quarantined whole, never partially imported. The
    -- raw payload stays reachable via (fetch_id, member_path), so a fixed
    -- importer reprocesses from the archive without re-downloading.
    CREATE TABLE IF NOT EXISTS quarantine (
        id             INTEGER PRIMARY KEY AUTOINCREMENT,
        notice_id      INTEGER REFERENCES notices(id),
        fetch_id       INTEGER NOT NULL REFERENCES fetches(id),
        member_path    TEXT NOT NULL,
        content_hash   TEXT NOT NULL,
        profile        TEXT,
        reason         TEXT NOT NULL,
        detail         TEXT,
        first_seen     INTEGER NOT NULL, -- unix seconds
        reprocessed_at INTEGER,
        -- A held member that was RE-EXAMINED and correctly not ingested, because a
        -- dispatch policy declines it — the 2008 per-language duplicate siblings
        -- (issue 84). Distinct from `reprocessed_at`, which means RECLAIMED: using
        -- that column here would claim ~593k notices entered the corpus that never
        -- did. Three outcomes exist (outstanding / reclaimed / skipped) so the
        -- schema carries three, rather than hiding one inside another.
        skipped_at     INTEGER,
        skipped_reason TEXT,
        -- A re-parse that FAILS AGAIN records its outcome here (issue 87):
        -- `reason`/`detail` above mean the member's CURRENT hold cause, and the
        -- first-ingest pair moves to `first_reason`/`first_detail` on the first
        -- overwrite. `last_attempt_at`/`attempts` make attempted-and-still-
        -- failing distinguishable from never-reached without the job logs.
        -- Deliberately separate from `reprocessed_at`: that column means
        -- RECLAIMED and the reprocess resume predicate keys on it, so a failed
        -- member must never set it — it stays in the backlog under its new,
        -- true reason.
        last_attempt_at INTEGER,
        attempts        INTEGER,
        first_reason    TEXT,
        first_detail    TEXT,
        UNIQUE(fetch_id, member_path, content_hash)
    ) STRICT;
    -- Every quarantine metric filters or groups by `reason` first — the reason
    -- breakdown, the field-code gaps, and issue 40's resolution split (which then
    -- narrows by profile + a `detail LIKE`). Without this index each is a full
    -- scan of the ~1.2M-row table; with it they seek to the (usually small) rows
    -- of one reason. Idempotent CREATE INDEX, built once on first open after
    -- deploy like `notices_fetch_id` (issues 37/40).
    CREATE INDEX IF NOT EXISTS quarantine_reason ON quarantine(reason);
    -- The reprocess flags a reclaimed member's row by its notice_id
    -- (`reclaim_notice`, issues 76/77). Without this index that per-member UPDATE
    -- full-scans the whole ~2.4M-row table — O(held × 2.4M) per package, which
    -- cliffed a 70k-member dense bucket to ~1 s/member (issue 80). With it the
    -- UPDATE seeks its row. Built once on first open like `quarantine_reason`.
    CREATE INDEX IF NOT EXISTS quarantine_notice_id ON quarantine(notice_id);

    -- ------------------------------------------------------------------
    -- Notice-parsed layer. The relational reading of one notice's payload,
    -- still in the source's own terms (raw section ids as published, source
    -- field ids); the canonical Tender/Lot/Organization layer is projected
    -- from here, never the other way round (ADR-0001).
    --
    -- Shape: eForms' *node tree* — not its field list — defines the relational
    -- structure (docs/research/eforms-data-model.md §2.1): every repeatable
    -- node instance is a section, every field value hangs off the nearest
    -- enclosing section. That is why a handful of typed value tables covers
    -- all 1256 fields without a column per business term, and why deep
    -- results-layer nodes (LotResult, LotTender, SettledContract, UBO) are
    -- already stored losslessly here before issue 13 models them
    -- first-class.
    -- ------------------------------------------------------------------

    -- One row per repeatable-node instance, plus 'PROCEDURE' for the notice
    -- root. `section_id` is the identifier the notice published (LOT-0001,
    -- ORG-0002, RES-0001 — the very strings change notices reference in
    -- BT-13716), or `<node-id>#<n>` where the node has no identifier field.
    CREATE TABLE IF NOT EXISTS notice_sections (
        notice_id         INTEGER NOT NULL REFERENCES notices(id),
        section_id        TEXT NOT NULL,
        kind              TEXT NOT NULL, -- Lot, LotsGroup, Part, Organisation, LotResult, …
        parent_section_id TEXT,
        PRIMARY KEY (notice_id, section_id)
    ) STRICT;
    -- (kind, notice_id), not bare (kind): the D5 reveal recheck walks one kind's
    -- cohort in cursor-resumable slices, and turso only serves
    -- `kind = ? AND notice_id > ? ORDER BY notice_id` as an index range seek off
    -- the composite — with the bare index it re-scans the cohort from the start
    -- every slice (measured in reveal_cursor_probe, issue 274). Kind-only scans
    -- read the same index by prefix, so the bare form has no remaining use and is
    -- dropped; both statements are no-ops after the first open.
    CREATE INDEX IF NOT EXISTS notice_sections_kind_notice ON notice_sections(kind, notice_id);
    DROP INDEX IF EXISTS notice_sections_kind;

    -- Free text, including url/phone/email. `lang` is the published
    -- @languageID (eForms notices carry their official language(s) only, so
    -- these rows are exactly the EN + original set CONTEXT.md asks for).
    CREATE TABLE IF NOT EXISTS notice_texts (
        notice_id  INTEGER NOT NULL REFERENCES notices(id),
        section_id TEXT NOT NULL,
        field_id   TEXT NOT NULL,
        ordinal    INTEGER NOT NULL,
        lang       TEXT,
        value      TEXT NOT NULL,
        -- `ordinal` already separates a text-multilingual field's language
        -- variants: it counts every repeat of the field within the section.
        PRIMARY KEY (notice_id, section_id, field_id, ordinal)
    ) STRICT;

    -- Controlled-vocabulary values; `list_name` is the published @listName.
    CREATE TABLE IF NOT EXISTS notice_codes (
        notice_id  INTEGER NOT NULL REFERENCES notices(id),
        section_id TEXT NOT NULL,
        field_id   TEXT NOT NULL,
        ordinal    INTEGER NOT NULL,
        list_name  TEXT,
        code       TEXT NOT NULL,
        PRIMARY KEY (notice_id, section_id, field_id, ordinal)
    ) STRICT;

    -- CPV and NUTS: the two vocabularies queried as classifications rather
    -- than as one field's value.
    CREATE TABLE IF NOT EXISTS notice_classifications (
        notice_id  INTEGER NOT NULL REFERENCES notices(id),
        section_id TEXT NOT NULL,
        field_id   TEXT NOT NULL,
        ordinal    INTEGER NOT NULL,
        scheme     TEXT NOT NULL, -- 'cpv' | 'nuts'
        code       TEXT NOT NULL,
        PRIMARY KEY (notice_id, section_id, field_id, ordinal)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS notice_classifications_code ON notice_classifications(scheme, code);

    -- Money as INTEGER cents + currency (CONTEXT.md). A notice whose amount
    -- carries more than two fraction digits is quarantined, never rounded.
    CREATE TABLE IF NOT EXISTS notice_amounts (
        notice_id  INTEGER NOT NULL REFERENCES notices(id),
        section_id TEXT NOT NULL,
        field_id   TEXT NOT NULL,
        ordinal    INTEGER NOT NULL,
        cents      INTEGER NOT NULL,
        currency   TEXT NOT NULL,
        PRIMARY KEY (notice_id, section_id, field_id, ordinal)
    ) STRICT;

    -- UTC instant + the published offset, because the offset is the buyer's
    -- local wall-clock deadline and normalising it away loses meaning.
    -- `has_time` distinguishes a date-only field (UTC midnight of the local
    -- day) from a date the SDK pairs with a time field.
    CREATE TABLE IF NOT EXISTS notice_dates (
        notice_id      INTEGER NOT NULL REFERENCES notices(id),
        section_id     TEXT NOT NULL,
        field_id       TEXT NOT NULL,
        ordinal        INTEGER NOT NULL,
        utc_seconds    INTEGER NOT NULL,
        offset_minutes INTEGER NOT NULL,
        has_time       INTEGER NOT NULL,
        PRIMARY KEY (notice_id, section_id, field_id, ordinal)
    ) STRICT;

    -- Counts and indicators (SQLite STRICT has no BOOLEAN; indicators are 0/1).
    CREATE TABLE IF NOT EXISTS notice_integers (
        notice_id  INTEGER NOT NULL REFERENCES notices(id),
        section_id TEXT NOT NULL,
        field_id   TEXT NOT NULL,
        ordinal    INTEGER NOT NULL,
        value      INTEGER NOT NULL,
        PRIMARY KEY (notice_id, section_id, field_id, ordinal)
    ) STRICT;

    -- Weights, percentages and measures; `unit` is the published @unitCode
    -- (durations in eForms are value+unit, not ISO 8601 strings).
    CREATE TABLE IF NOT EXISTS notice_numbers (
        notice_id  INTEGER NOT NULL REFERENCES notices(id),
        section_id TEXT NOT NULL,
        field_id   TEXT NOT NULL,
        ordinal    INTEGER NOT NULL,
        value      REAL NOT NULL,
        unit       TEXT,
        PRIMARY KEY (notice_id, section_id, field_id, ordinal)
    ) STRICT;

    -- Identifiers and the notice-local references between sections. `is_ref`
    -- marks an id-ref: with `scheme` it names the role a section plays for
    -- another (OPT-300-Procedure-Buyer → ORG-0001 is a buyer mention).
    CREATE TABLE IF NOT EXISTS notice_ids (
        notice_id  INTEGER NOT NULL REFERENCES notices(id),
        section_id TEXT NOT NULL,
        field_id   TEXT NOT NULL,
        ordinal    INTEGER NOT NULL,
        scheme     TEXT,
        value      TEXT NOT NULL,
        is_ref     INTEGER NOT NULL,
        PRIMARY KEY (notice_id, section_id, field_id, ordinal)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS notice_ids_target ON notice_ids(value) WHERE is_ref = 1;

    -- The withheld-field mechanism (BT-195/196/197/198): a publishable value
    -- may be suppressed, the notice then carrying only which field, why, and
    -- until when. Those live in their own FieldsPrivacy sections, so the
    -- satellite is a view over them, not a fourth copy of the data.
    DROP VIEW IF EXISTS notice_withheld_fields;
    CREATE VIEW notice_withheld_fields AS
    SELECT s.notice_id,
           s.parent_section_id AS section_id,
           MAX(CASE WHEN c.field_id LIKE 'BT-195%' THEN c.code END) AS withheld_field,
           MAX(CASE WHEN c.field_id LIKE 'BT-197%' THEN c.code END) AS reason_code,
           (SELECT t.value FROM notice_texts t
             WHERE t.notice_id = s.notice_id AND t.section_id = s.section_id
               AND t.field_id LIKE 'BT-196%' LIMIT 1) AS reason_text,
           (SELECT d.utc_seconds FROM notice_dates d
             WHERE d.notice_id = s.notice_id AND d.section_id = s.section_id
               AND d.field_id LIKE 'BT-198%' LIMIT 1) AS publish_after
      FROM notice_sections s
      LEFT JOIN notice_codes c ON c.notice_id = s.notice_id AND c.section_id = s.section_id
     WHERE s.kind = 'FieldsPrivacy'
     GROUP BY s.notice_id, s.section_id;

    -- The live-layer presence marker (issue 133 / task #38). One row per table
    -- the standing gate's `present_*` checks cover, carrying the only fact that
    -- cannot be recovered by looking at the layer itself: whether it was EVER
    -- populated.
    --
    -- Why this has to be stored rather than derived. An empty table is
    -- ambiguous — a fresh install and a wiped corpus look identical from
    -- inside the database — and the whole incremental-assertion scheme (#28)
    -- is blind here: an assertion scoped to the rows a batch writes cannot
    -- observe that a table has NO rows, and a wipe writes no batch at all.
    --
    -- Deliberately NOT derived from `changes` being non-empty, which was the
    -- tempting shortcut: `clear_changes` (canonical.rs) legitimately empties
    -- that table as a paired one-time reset, so it is not the never-cleared
    -- witness it looks like.
    -- Rendered operator reports, one row per kind (issue 230). A measurement too
    -- expensive to run per request — the semantic data-quality report is minutes
    -- of scanning — is computed by a JOB and left here for whoever asks, so the
    -- read is a point lookup instead of eleven full scans through a 10s-capped
    -- endpoint.
    CREATE TABLE IF NOT EXISTS reports (
        kind        TEXT PRIMARY KEY,
        -- When the job that produced this finished, unix seconds. A reader
        -- decides for itself whether that is too old to trust; the row never
        -- expires on its own, because a stale report that says WHEN it was made
        -- beats no report at all.
        computed_at INTEGER NOT NULL,
        -- The rendered body, exactly as the tool would have printed it.
        body        TEXT NOT NULL
    ) STRICT;

    -- Previous versions of the rows above (issue 335). `reports` keeps exactly one
    -- row per kind, so every re-run destroyed the measurement it replaced — and
    -- there is no way to ask what changed since the last run, which is the
    -- question most of those reports exist to support. It cost the enumeration of
    -- the 102 cases that left issue 311's review cohort: the count had been read,
    -- the membership had not, and the re-run overwrote it.
    --
    -- ADDITIVE ON PURPOSE. The first plan was to key `reports` on
    -- (kind, computed_at), but the MIGRATIONS list above says anything beyond ADD
    -- COLUMN is out of scope by policy. A separate table needs no migration at
    -- all, leaves `reports`, `latest_report` and `report_stamps` untouched, and so
    -- carries no risk to the forty existing readers. The latest body is stored
    -- twice, which at a few dozen kinds is not worth a schema change to avoid.
    CREATE TABLE IF NOT EXISTS report_history (
        kind        TEXT    NOT NULL,
        computed_at INTEGER NOT NULL,
        body        TEXT    NOT NULL,
        -- Two runs finishing in the same second are one version, not two: the
        -- second overwrites, which is what INSERT OR REPLACE relies on.
        PRIMARY KEY (kind, computed_at)
    ) STRICT;

    CREATE TABLE IF NOT EXISTS layer_presence (
        name           TEXT PRIMARY KEY,
        -- 1 once this table has been observed non-empty at least once. Never
        -- returns to 0: it records history, not current state.
        ever_populated INTEGER NOT NULL DEFAULT 0,
        -- When it was first observed empty HAVING been populated — the
        -- transition, and the age of the damage. NULL while populated.
        went_empty_at  INTEGER,
        observed_at    INTEGER NOT NULL
    ) STRICT;

    -- Daily EUR-pivot exchange rates (ADR-0014): one row per (currency, day),
    -- the ECU daily series 1993-1998 chained to the ECB reference rates 1999-,
    -- plus the irrevocable euro conversion rates as 'irrevocable' rows valid
    -- from each currency's adoption date FOREVER (the currency is frozen).
    -- rate_to_eur: units of `currency` per 1 EUR (the ECB quoting convention).
    -- A REFERENCE table, deliberately notice-layer: it must survive
    -- reset_tender_layer and every rebuild. Dates are TEXT 'YYYY-MM-DD' — the
    -- source datasets' own key, human-auditable, and lexicographically ordered.
    CREATE TABLE IF NOT EXISTS currency_rates (
        currency    TEXT NOT NULL,
        rate_date   TEXT NOT NULL,
        rate_to_eur REAL NOT NULL,
        source      TEXT NOT NULL, -- 'ecb' | 'ecu' | 'irrevocable'
        PRIMARY KEY (currency, rate_date)
    ) STRICT;
";

/// Live writer-contention counters (issue 241).
///
/// Issue 240 was a 25-minute outage in which every token-bearing request queued
/// behind a fold that held the writer — and NOTHING said so. `/health` reads the
/// in-memory cursor by design (issue 61), the read path uses the reader pool, and
/// the job record showed a healthy run. The outage was found by hand.
///
/// A held writer is normal: a fold holds it for its whole transaction. The
/// alertable state is a held writer with WORK QUEUED BEHIND IT, so that pair is
/// what these counters export. `depth` alone answers it — a sustained non-zero
/// queue depth is requests (or jobs) waiting, whatever is holding the lock.
///
/// Every writer acquisition in the crate funnels through [`Db::conn`], so this
/// counts all of them and costs two relaxed atomics per acquisition.
#[derive(Debug, Default)]
pub struct WriterContention {
    /// Callers currently blocked in [`Db::conn`], not counting the holder.
    depth: AtomicU64,
    /// Total acquisitions since open — the denominator for a mean wait.
    acquisitions: AtomicU64,
    /// Total nanoseconds spent waiting, summed across acquisitions.
    waited_nanos: AtomicU64,
    /// The longest single wait since open. A high-water mark, never reset: the
    /// point is to still be able to see yesterday's 25-minute stall.
    longest_wait_nanos: AtomicU64,
}

/// A single writer wait at or above this is journaled with its length, so the
/// never-reset `longest_wait_seconds` high-water mark has a timestamp to be
/// read against. Ten seconds is far above any ordinary acquisition (the mean
/// is microseconds) and below the shortest stall worth investigating.
pub const SLOW_WRITER_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// One caller's place in the writer queue (issue 241), released on drop.
///
/// The depth was a `fetch_add` before the lock and a `fetch_sub` after it — which
/// is one `fetch_sub` short whenever the waiting future is DROPPED instead of
/// resumed: a `tokio::time::timeout` around `Db::conn` (issue 256's queue-persist
/// gives up after 30 s behind a fold) abandons the wait, and the gauge stayed one
/// higher for the life of the process. On 2026-09-03 `/metrics` read
/// `writer_queue_depth 2` on an idle box after two such give-ups — exactly the
/// "sustained non-zero depth" the gauge exists to flag, with nobody waiting.
/// A drop guard is cancellation-safe by construction.
struct Queued<'a>(&'a AtomicU64);

impl<'a> Queued<'a> {
    fn new(depth: &'a AtomicU64) -> Self {
        depth.fetch_add(1, Ordering::Relaxed);
        Queued(depth)
    }
}

impl Drop for Queued<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

/// A snapshot of [`WriterContention`], so a scrape reads one consistent set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WriterStats {
    pub depth: u64,
    pub acquisitions: u64,
    pub waited_seconds: f64,
    pub longest_wait_seconds: f64,
}

/// Where a re-parse's per-notice time goes (issue 247).
///
/// Filed under measurement rather than debugging because the guessing cost hours: the
/// same package re-parsed at 410 members/min on one run and 3.5 on the next, and every
/// statement involved answers in about a millisecond through the READER pool. The one
/// structural difference is that `reparse_notice` runs its lookup, deletes and inserts
/// on the long-lived WRITER connection inside `BEGIN IMMEDIATE` — so the split has to
/// be measured there, where it happens, not inferred from a reader-pool timing.
#[derive(Default)]
struct ReparsePhases {
    notices: AtomicU64,
    lookup_nanos: AtomicU64,
    clear_nanos: AtomicU64,
    insert_nanos: AtomicU64,
    commit_nanos: AtomicU64,
    /// Per-STATEMENT totals inside the clear, keyed by the label `Db::step` already
    /// carries. `clear` turned out to be 99.6% of a re-parse's writer time (measured:
    /// 159 ms a notice against 57 µs for the lookup) and it runs nine statements, so
    /// the phase total names the wrong thing on its own. A mutex is free at this scale:
    /// every write here already holds the writer.
    clear_stmt_nanos: std::sync::Mutex<std::collections::BTreeMap<&'static str, u64>>,
}

/// A snapshot of [`ReparsePhases`], so a scrape reads one consistent set.
#[derive(Debug, Clone, PartialEq)]
pub struct ReparseStats {
    pub notices: u64,
    pub lookup_seconds: f64,
    pub clear_seconds: f64,
    pub insert_seconds: f64,
    pub commit_seconds: f64,
    /// `(statement label, seconds)` inside the clear, busiest first.
    pub clear_statements: Vec<(&'static str, f64)>,
}

pub struct Db {
    database: turso::Database,
    conn: Mutex<Connection>,
    /// Writer-contention counters (issue 241), read by `/metrics`.
    writer: WriterContention,
    /// Re-parse phase timings (issue 247), read by `/metrics`.
    reparse: ReparsePhases,
    /// The database file path, kept so [`Db::snapshot`] (issue 23) knows which
    /// file to copy — turso exposes no path accessor.
    path: String,
    /// The in-memory EUR-pivot rates lookup (ADR-0014), swapped whole on
    /// [`Db::reload_rates_lookup`]. Empty until first loaded — derivations are
    /// honestly absent, never stale.
    rates: std::sync::RwLock<std::sync::Arc<rates::RatesLookup>>,
    /// The pool backing `Db`'s own read-only accessors. Reads run over WAL in
    /// parallel with the writer, so a dashboard/admin query never queues behind
    /// an ingestion job that is holding the writer for the length of its
    /// transaction (issue 20). The writer (`conn`) is reserved for writes and
    /// schema; this is separate from the public API's own pool (`readers`).
    read_pool: Arc<Readers>,
    /// The WAL read-gate (issue 63), shared by every reader pool and the gated
    /// bulk-load checkpoint. `None` unless `TENDER_WAL_READ_GATE` is set at open,
    /// so the default build is byte-for-byte the old behaviour.
    wal_gate: read::WalGate,
    /// The change-cursor doorbell (docs/research/api-layer.md §2): the writer
    /// publishes the newest cursor after every committed change-append, and SSE
    /// streams wake on it and read the log themselves. It carries only the
    /// cursor — never a payload — so a slow subscriber cannot lose an event.
    cursor: watch::Sender<i64>,
}

static DB: OnceCell<Arc<Db>> = OnceCell::const_new();

/// The process-wide database, opened and migrated on first use.
pub async fn state() -> Arc<Db> {
    DB.get_or_init(|| async {
        let path = std::env::var("TENDER_DB").unwrap_or_else(|_| "tender-db.db".into());
        Arc::new(Db::open(&path).await.expect("open Turso database"))
    })
    .await
    .clone()
}

/// Additive migrations for databases created before a column existed.
/// `CREATE TABLE IF NOT EXISTS` never evolves an existing table, so every
/// later-added column needs its ALTER here; an already-migrated database
/// answers "duplicate column name" and the statement is skipped. Anything
/// beyond ADD COLUMN stays out of scope by policy — the canonical layer is
/// rebuildable, and destructive changes recreate from the archive instead.
const MIGRATIONS: [&str; 16] = [
    "ALTER TABLE notices ADD COLUMN published_at INTEGER",
    "ALTER TABLE notices ADD COLUMN dispatched_at INTEGER",
    "ALTER TABLE tender_versions ADD COLUMN dispatched_at INTEGER",
    // The resume cursor (issue 32). job_queue shipped in bad8dda (issue 21), so
    // the prod table predates this column — CREATE TABLE IF NOT EXISTS never adds
    // it, and recover()'s `SELECT … progress` would fail on the first boot of the
    // new binary. NULL for every existing row, which is correct (a fresh cursor).
    "ALTER TABLE job_queue ADD COLUMN progress TEXT",
    // The webhook slot's feed generation (issue 178). NULL for an endpoint that
    // predates this column, which the sweeper treats as "unknown generation" and
    // resets on first contact — the conservative choice, since a rebuild may have
    // stranded its cursor beyond the new head with no batch to carry the signal.
    "ALTER TABLE webhook_endpoints ADD COLUMN last_generation INTEGER",
    // The Supervisor's job id on a finished run. `job_log` shipped in issue 16
    // keyed only by its own append counter, so a finished run carried no link
    // back to the queue id it ran under — and the in-memory counter, seeded from
    // the *pending* queue only, restarted at 1 whenever a restart found no work
    // outstanding, re-using ids the log had already spent. NULL for every
    // pre-existing row, which recovery reads as "no floor from this row".
    "ALTER TABLE job_log ADD COLUMN job_id INTEGER",
    // The tax basis of an amount (issue 251). Every existing row answers NULL, which is
    // the honest reading: those figures were written without knowing whether the source
    // called them inclusive or exclusive, and the column has always held both.
    "ALTER TABLE tender_version_amounts ADD COLUMN tax_basis TEXT",
    // The winner-decision date beside the conclusion date (issue 255). BT-1451 has been
    // in the parse layer of every eForms CAN all along and had nowhere to land; existing
    // rows answer NULL until the era is re-folded, which is the honest reading.
    "ALTER TABLE tender_version_contracts ADD COLUMN decided_utc INTEGER",
    "ALTER TABLE tender_version_contracts ADD COLUMN decided_offset INTEGER",
    "ALTER TABLE tender_version_contracts ADD COLUMN decided_has_time INTEGER",
    // …and on the award block itself, for the eras with no contract graph (issue 255,
    // slice 2). Same three columns, one table over.
    "ALTER TABLE tender_version_lot_results ADD COLUMN decided_utc INTEGER",
    "ALTER TABLE tender_version_lot_results ADD COLUMN decided_offset INTEGER",
    "ALTER TABLE tender_version_lot_results ADD COLUMN decided_has_time INTEGER",
    // The withheld marker on both amount-bearing satellites (issue 372). Existing
    // rows answer NULL until re-folded, which is the honest reading: they were
    // written by a projection that never consulted the notice's BT-195 declaration,
    // so it does not know whether their figure is a value or a placeholder.
    //
    // These two are here because the fix SHIPPED WITHOUT THEM and was inert on prod
    // (2026-09-09): every test creates its database fresh, where the CREATE TABLE
    // carries the column, so the whole gate passed green while `SELECT quality FROM
    // tender_version_amounts` on the box answered "no such column". Caught by
    // probing the production database rather than by the suite.
    "ALTER TABLE tender_version_amounts ADD COLUMN quality TEXT",
    "ALTER TABLE tender_version_bids ADD COLUMN quality TEXT",
    // …and the statistics satellite (issue 372 unit 4). Same reading: NULL until
    // re-folded. Added WITH its SCHEMA column this time, not a firing later.
    "ALTER TABLE tender_version_result_stats ADD COLUMN quality TEXT",
];

async fn migrate(conn: &Connection) -> turso::Result<()> {
    for statement in MIGRATIONS {
        match conn.execute(statement, ()).await {
            Ok(_) => {}
            Err(e) if e.to_string().contains("duplicate column") => {}
            Err(e) => return Err(e),
        }
    }

    // The current-version head pointer (issue 25). On a database created before
    // it, the columns are added here and backfilled once from tender_versions;
    // thereafter the projection maintains them, so this is a no-op. `MAX(seq)` and
    // the head's `published_at` come straight off the PK index (tender_id, seq).
    // The projection-logic epoch (issue 99). Metadata-only on a STRICT table with a
    // constant default, so O(1) even on the 8.1M-row prod `tenders` — the same shape
    // `alter_add_column_cost.rs` proved for the issue-58 watermark. Existing rows read
    // the default 0, i.e. "folded under unknown/older logic", so the first fold that
    // touches each one rewrites it. That is the intended semantics, not a migration
    // cost: nothing is rewritten until a refold marks it.
    add_column(conn, "ALTER TABLE tenders ADD COLUMN projection_epoch INTEGER NOT NULL DEFAULT 0")
        .await?;

    let added = add_column(conn, "ALTER TABLE tenders ADD COLUMN current_seq INTEGER").await?;
    let added = add_column(conn, "ALTER TABLE tenders ADD COLUMN current_published_at INTEGER").await? || added;
    // The current version's submission deadline (issue 216, deadline half):
    // maintained by the fold's head update; existing rows are backfilled by the
    // batched `backfill-deadlines` admin job, NOT here — a one-shot UPDATE over
    // 7.9M rows would be a multi-minute blocking boot (the 82/83 regression) in
    // one giant WAL transaction (the issue-42 lesson). Metadata-only, O(1).
    add_column(conn, "ALTER TABLE tenders ADD COLUMN current_deadline INTEGER").await?;
    // Issue 239: `v_tenders.title` reads this instead of a correlated subquery. NULL on
    // every pre-239 row until the backfill job fills it, and the view honestly shows
    // NULL rather than a wrong title in the meantime — a title that is absent for an
    // hour beats a view nobody can query.
    add_column(conn, "ALTER TABLE tenders ADD COLUMN current_title TEXT").await?;
    // Issue 306 repair resumability: the rederive-eur walk's persisted
    // watermark (see the projection_state schema comment).
    add_column(
        conn,
        "ALTER TABLE projection_state ADD COLUMN rederive_eur_watermark INTEGER NOT NULL DEFAULT 0",
    )
    .await?;
    // Issue 300 Stage 4: the key-build resume watermark and the edge store's
    // monotone-growth baseline (see the projection_state schema comment).
    add_column(
        conn,
        "ALTER TABLE projection_state ADD COLUMN org_match_keys_watermark INTEGER NOT NULL DEFAULT 0",
    )
    .await?;
    add_column(
        conn,
        "ALTER TABLE projection_state ADD COLUMN org_edge_total INTEGER NOT NULL DEFAULT 0",
    )
    .await?;
    add_column(
        conn,
        "ALTER TABLE projection_state ADD COLUMN org_match_keys_epoch TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    // Issue 371: the currency present-set's coverage attestation (see the
    // projection_state schema comment). Metadata-only on a STRICT table with a
    // constant default, so O(1) even on prod. It defaults to 0 — NOT covered —
    // because a file whose amount rows predate `tender_currency_presence` holds a
    // SUBSET of the codes it carries, and a subset is the one state the read's
    // guard must never answer from. `backfill-currencies` establishes it there.
    add_column(
        conn,
        "ALTER TABLE projection_state ADD COLUMN currency_presence_complete INTEGER NOT NULL DEFAULT 0",
    )
    .await?;
    // …but a file with NO amount rows at all has a trivially complete present-set,
    // and that is every fresh database plus every one a rebuild has just emptied.
    // Attest it here rather than making a new box wait for a backfill job that has
    // no work to do. One `LIMIT 1` probe at open: on a populated file it finds a row
    // immediately and leaves the flag alone. This only ever sets the flag TO 1 — a
    // half-finished rebuild has an attested set covering what it has written so far,
    // and clearing that would be wrong as well as pointless.
    let amounts_present = {
        let mut rows = conn.query("SELECT 1 FROM tender_version_amounts LIMIT 1", ()).await?;
        rows.next().await?.is_some()
    };
    if !amounts_present {
        conn.execute("UPDATE projection_state SET currency_presence_complete = 1 WHERE id = 0", ())
            .await?;
    }
    // The Unicode-lowercased org name (issue 217-B): fold-written for new orgs,
    // backfilled by the batched `backfill-org-names` job (24.6M rows — never at
    // open; the 82/83 + issue-42 lessons, same as current_deadline above).
    add_column(conn, "ALTER TABLE organizations ADD COLUMN name_norm TEXT").await?;
    if added {
        conn.execute(
            "UPDATE tenders SET
                 current_seq = (SELECT MAX(v.seq) FROM tender_versions v WHERE v.tender_id = tenders.id),
                 current_published_at = (SELECT v.published_at FROM tender_versions v
                                          WHERE v.tender_id = tenders.id ORDER BY v.seq DESC LIMIT 1)",
            (),
        )
        .await?;
    }
    // Depends on the column above, so it lives here rather than in the schema
    // batch (which runs before this ALTER on a pre-issue-25 database).
    conn.execute(
        "CREATE INDEX IF NOT EXISTS tenders_current_published ON tenders(current_published_at, id)",
        (),
    )
    .await?;

    // The incremental-projection watermark (issue 58). On a database created
    // before it, every parsed notice is ALREADY folded into the canonical layer,
    // so backfill `projected = 1` for the notices that caused a version — else the
    // first incremental run would treat the whole existing corpus as unprojected.
    // (A parsed notice always causes exactly one version, so the join covers them
    // all.) Runs once, when the column is first added.
    let added_projected =
        add_column(conn, "ALTER TABLE notices ADD COLUMN projected INTEGER NOT NULL DEFAULT 0").await?;
    if added_projected {
        conn.execute(
            "UPDATE notices SET projected = 1
              WHERE parse_state = 'parsed'
                AND id IN (SELECT caused_by_notice_id FROM tender_versions)",
            (),
        )
        .await?;
    }
    // Issue 84's third quarantine outcome. Nullable and defaulted to absent, so an
    // existing row is untouched: nothing becomes "skipped" by migrating, only by a
    // deliberate run of the marker.
    add_column(conn, "ALTER TABLE quarantine ADD COLUMN skipped_at INTEGER").await?;
    add_column(conn, "ALTER TABLE quarantine ADD COLUMN skipped_reason TEXT").await?;
    // ADR-0014: the derived EUR value BESIDE each published amount — nullable,
    // populated by the projection from `currency_rates`; NULL = unconvertible
    // (honest absence) or not-yet-refolded. Never touches the published value.
    add_column(conn, "ALTER TABLE tender_version_amounts ADD COLUMN eur_cents INTEGER").await?;
    add_column(conn, "ALTER TABLE tender_version_lot_results ADD COLUMN awarded_eur_cents INTEGER").await?;
    add_column(conn, "ALTER TABLE tender_version_bids ADD COLUMN eur_cents INTEGER").await?;
    add_column(conn, "ALTER TABLE tender_version_contracts ADD COLUMN eur_cents INTEGER").await?;
    // ADR-0014 D5: the head version's MAX derived-EUR amount, fold-maintained
    // like current_deadline/current_title; backfilled by `backfill-values`.
    // ALSO in reset_tender_layer's hardcoded CREATE — a column added here
    // alone is missing for a whole rebuild (the current_deadline lesson).
    add_column(conn, "ALTER TABLE tenders ADD COLUMN current_value_eur_cents INTEGER").await?;
    // ADR-0013 D3's third leg (2026-09-02): the version's original language.
    // O(1) at boot like the eur_cents columns; the standing corpus is stamped by
    // the `backfill-original-lang` job rather than a refold.
    add_column(conn, "ALTER TABLE tender_versions ADD COLUMN original_lang TEXT").await?;
    // Issue 87: a failed re-parse stamps its attempt and rewrites the row's
    // reason/detail to the CURRENT failure (the first-ingest pair is preserved
    // once in first_reason/first_detail). Nullable and absent by default, so
    // nothing becomes "attempted" by migrating — and deliberately NOT
    // reprocessed_at, which the reclaim resume predicate keys on: overloading it
    // would silently drop a failed member from the next run's backlog.
    add_column(conn, "ALTER TABLE quarantine ADD COLUMN last_attempt_at INTEGER").await?;
    add_column(conn, "ALTER TABLE quarantine ADD COLUMN attempts INTEGER").await?;
    add_column(conn, "ALTER TABLE quarantine ADD COLUMN first_reason TEXT").await?;
    add_column(conn, "ALTER TABLE quarantine ADD COLUMN first_detail TEXT").await?;
    // The `notices_unprojected` partial index is built LAZILY at the end of a
    // projection ([`Db::ensure_unprojected_index`]), NOT here: on a large existing
    // DB upgraded to this schema, Phase-2 hasn't marked anything projected yet, so
    // building it at open would index all ~12M rows (minutes). Deferring it to
    // after the first projection — when nearly every parsed notice is projected=1,
    // so the partial index is near-empty — makes the (salvage) startup fast.
    Ok(())
}

/// Run an `ADD COLUMN`, reporting whether it actually added the column — a
/// `duplicate column` answer means an already-migrated database, not an error.
async fn add_column(conn: &Connection, statement: &str) -> turso::Result<bool> {
    match conn.execute(statement, ()).await {
        Ok(_) => Ok(true),
        Err(e) if e.to_string().contains("duplicate column") => Ok(false),
        Err(e) => Err(e),
    }
}

/// The sibling-lookup template halves, split at the one optional clause so
/// [`Db::sibling_exists`] can compose either variant from a single source.
const SIBLING_HEAD: &str = "EXISTS (
            SELECT 1 FROM notices n
             WHERE n.source = 'ted'
               -- The unary `+` makes `parse_state` a non-indexable term, and it
               -- is here for a CROSS-ENGINE reason worth stating exactly, because
               -- the two engines disagree (measured, 2026-08-05):
               --   * turso  — seeks `UNIQUE(source, publication_id, …)` with or
               --              without the `+`. Prod is fine either way.
               --   * sqlite3 — PREFERS `notices_parse_state`, which means seeking
               --              to every 'parsed' notice (~14M on prod) for each row
               --              examined. Without the `+` it is an unbounded probe.
               -- sqlite3 is not a hypothetical reader: the canonical-verify suite,
               -- run_light, the standing gate and issue 84's own falsifier all read
               -- snapshots with stock sqlite3, and reproducing this predicate there
               -- is the obvious way to check the marker's scope. So the `+` is what
               -- keeps the verification path from hanging the way section B did —
               -- same unbounded-probe shape, reached from the opposite direction
               -- (there a missing predicate, here an extra one offering a worse
               -- index).
";
const SIBLING_TAIL: &str = "               -- `…/115165_2008.fr` -> `115165-2008`. The trailing **3** is
               -- the length of a `.xx` suffix, correct ONLY because every code
               -- in this population is two letters — measured, not assumed: section
               -- A found exactly 23 languages, all 2-letter (bg cs da de el en es
               -- et fi fr ga hu it lt lv mt nl pl pt ro sk sl sv). It is a property
               -- of THIS corpus, not a general rule: a 3-letter code would silently
               -- mis-extract the id, the sibling lookup would miss, and the row
               -- would simply stay outstanding (fail-safe, but silently). Do not
               -- lift this expression into a general helper without replacing the
               -- constant with a real suffix split.
               AND n.publication_id = replace(
                     substr(replace(q.member_path,
                                    rtrim(q.member_path, replace(q.member_path, '/', '')), ''),
                            1,
                            length(replace(q.member_path,
                                           rtrim(q.member_path, replace(q.member_path, '/', '')), '')) - 3),
                     '_', '-'))";

/// The mount point and filesystem type governing `path` — the `/proc/mounts` entry
/// with the longest matching mount point.
fn mount_of<'a>(mounts: &'a str, path: &str) -> Option<(&'a str, &'a str)> {
    let mut best: Option<(&str, &str)> = None;
    for line in mounts.lines() {
        let mut f = line.split_whitespace();
        let (Some(_dev), Some(point), Some(fstype)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        let covers = path == point
            || point == "/"
            || path.starts_with(&format!("{}/", point.trim_end_matches('/')));
        if covers && best.is_none_or(|(p, _)| point.len() > p.len()) {
            best = Some((point, fstype));
        }
    }
    best
}

/// Warn at open when large temporary spill files would land in RAM (issue 83).
///
/// turso's external sort spills to `TMPDIR`. A systemd unit with `PrivateTmp` (or
/// any default `/tmp`) puts that on tmpfs — RAM — which on the production box is
/// ~3.8 GB against a 450 GB database. A full-corpus `CREATE INDEX` spills many GB,
/// fills the tmpfs, and dies with `I/O error (pwrite): no storage space` while the
/// data disk sits at 180 GB free. That killed the recovery rebuild's index build and
/// left `tenders_current_published` missing (issues 82/83).
///
/// The hazard is specifically a **mismatch**: a large database on disk whose spill
/// goes to RAM. A database that lives on the same RAM filesystem as its spill is a
/// coherent, deliberately small setup — every scratch and test DB is one — so that
/// case is silent. Without that distinction this would fire on every `Db::open` in
/// the test suite on any host whose `/tmp` is tmpfs, and a warning that cries wolf
/// in CI is a warning nobody reads in production.
///
/// A warning, not a hard failure: refusing to open would take the service down for a
/// condition that is only fatal at scale. The point is that the next occurrence names
/// itself in the log instead of surfacing hours later as an opaque ENOSPC.
fn warn_if_spill_dir_is_ram(db_path: &str) {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let Ok(mounts) = std::fs::read_to_string("/proc/mounts") else { return };
    let Some((spill_point, fstype)) = mount_of(&mounts, &dir) else { return };
    if !matches!(fstype, "tmpfs" | "ramfs") {
        return;
    }
    // Resolve the database to an absolute path without requiring it to exist yet.
    let db_abs = if db_path.starts_with('/') {
        db_path.to_owned()
    } else {
        std::env::current_dir()
            .map(|d| d.join(db_path).to_string_lossy().into_owned())
            .unwrap_or_else(|_| db_path.to_owned())
    };
    if let Some((db_point, _)) = mount_of(&mounts, &db_abs)
        && db_point == spill_point
    {
        return; // database and spill share the RAM filesystem — coherent, stay quiet
    }
    eprintln!(
        "[store] WARNING: TMPDIR={dir} is {fstype} (RAM) but the database {db_abs} is not. \
         turso spills large external sorts to TMPDIR, so a full-corpus CREATE INDEX or scan \
         can exhaust it and fail with \"no storage space\" while the data disk is nearly \
         empty (issue 83). Point TMPDIR at disk-backed storage on the database's filesystem."
    );
}

/// One completed slice of the D5 reveal recheck — see [`Db::reveal_recheck`].
/// `after`/`upto` are the processed notice-id range (cursor in / cursor out);
/// `wrapped` means the cohort is exhausted and the next run restarts at 0.
/// `withheld_total` is the whole cohort; every other number is slice-scoped.
#[derive(Debug, Clone)]
pub struct RevealSlice {
    pub withheld_total: i64,
    pub after: i64,
    pub upto: i64,
    pub wrapped: bool,
    pub sections: i64,
    pub dated: i64,
    pub due: i64,
    pub checked: i64,
    pub revealed: i64,
    /// Due rows whose tender has NO later version at all — the promise is
    /// unmet but still awaitable, unlike `checked - revealed - no_later`
    /// (a later version exists and STILL withholds: the promise broken).
    pub no_later: i64,
    pub by_field: Vec<(String, i64)>,
}

impl Db {
    pub async fn open(path: &str) -> turso::Result<Db> {
        Db::open_inner(path, std::env::var_os("TENDER_WAL_READ_GATE").is_some()).await
    }

    /// `gate_on` forces the issue-63 WAL read-gate on; production derives it from
    /// `TENDER_WAL_READ_GATE` in [`open`]. Split out so tests drive the gate
    /// without touching the process environment (a data race under parallel tests).
    async fn open_inner(path: &str, gate_on: bool) -> turso::Result<Db> {
        warn_if_spill_dir_is_ram(path);
        let database = turso::Builder::new_local(path).build().await?;
        let conn = database.connect()?;
        // Some pragmas report their new value as a row, so go through `query`
        // (execute rejects statements that return rows) and drain the result.
        for pragma in PRAGMAS.iter().map(|p| p.to_string()).chain([cache_pragma()]) {
            let mut rows = conn.query(&pragma, ()).await?;
            while rows.next().await?.is_some() {}
        }
        conn.execute_batch(SCHEMA).await?;
        conn.execute_batch(canonical::SCHEMA).await?;
        conn.execute_batch(accounts::SCHEMA).await?;
        conn.execute_batch(jobs::SCHEMA).await?;
        conn.execute_batch(webhooks::SCHEMA).await?;
        migrate(&conn).await?;
        let cursor = watch::Sender::new(max_cursor(&conn).await?);
        // The WAL read-gate is opt-in (issue 63): only a full-corpus rebuild whose
        // Phase-1 checkpoint is being pinned by live readers needs it, and it adds a
        // shared-lock acquire to every read, so the default leaves it off (None).
        let wal_gate: read::WalGate = gate_on.then(|| Arc::new(tokio::sync::RwLock::new(())));
        let read_pool = Readers::open(database.clone(), READ_POOL, wal_gate.clone())?;
        Ok(Db {
            database,
            conn: Mutex::new(conn),
            writer: WriterContention::default(),
            reparse: ReparsePhases::default(),
            path: path.to_owned(),
            rates: std::sync::RwLock::new(std::sync::Arc::new(rates::RatesLookup::default())),
            read_pool,
            wal_gate,
            cursor,
        })
    }

    /// The current rates lookup (cheap Arc clone; empty until first reloaded).
    pub fn rates_lookup(&self) -> std::sync::Arc<rates::RatesLookup> {
        self.rates.read().expect("rates lock poisoned").clone()
    }

    pub(crate) fn set_rates_lookup(&self, lookup: rates::RatesLookup) {
        *self.rates.write().expect("rates lock poisoned") = std::sync::Arc::new(lookup);
    }

    async fn conn(&self) -> MutexGuard<'_, Connection> {
        // Issue 241: the wait itself is the measurement. Nothing else in the
        // process can see "a request is queued behind the writer" — the fast path
        // (uncontended) adds one `try_lock` and two relaxed adds.
        if let Ok(guard) = self.conn.try_lock() {
            self.writer.acquisitions.fetch_add(1, Ordering::Relaxed);
            return guard;
        }
        // Held by a guard, not an add/sub pair, so a caller that gives up mid-wait
        // (a `timeout` around this future) is un-counted on drop — see [`Queued`].
        let queued = Queued::new(&self.writer.depth);
        let started = std::time::Instant::now();
        let guard = self.conn.lock().await;
        drop(queued);
        let elapsed = started.elapsed();
        let waited = elapsed.as_nanos().min(u64::MAX as u128) as u64;
        let still_queued = self.writer.depth.load(Ordering::Relaxed);
        self.writer.acquisitions.fetch_add(1, Ordering::Relaxed);
        self.writer.waited_nanos.fetch_add(waited, Ordering::Relaxed);
        self.writer.longest_wait_nanos.fetch_max(waited, Ordering::Relaxed);
        if elapsed >= SLOW_WRITER_WAIT {
            // The high-water mark above says HOW LONG the worst wait was but not
            // WHEN — on 2026-09-03 a 752 s stall sat in `/metrics` with nothing in
            // the journal to line it up against (one wait held 752 of the 763 s
            // waited across 7.2M acquisitions). A timestamped line is the
            // attribution: whichever job the journal shows around it held the
            // writer. Rare by construction — at the 7.2M-acquisition scale, waits
            // this long numbered one.
            eprintln!(
                "[store] writer acquired after a {:.1} s wait ({still_queued} caller(s) still queued)",
                elapsed.as_secs_f64()
            );
        }
        guard
    }

    /// Writer contention since open (issue 241) — the input to the `/metrics`
    /// gauges that make issue 240's outage shape visible without running a query
    /// by hand.
    pub fn writer_stats(&self) -> WriterStats {
        const NANOS: f64 = 1_000_000_000.0;
        WriterStats {
            depth: self.writer.depth.load(Ordering::Relaxed),
            acquisitions: self.writer.acquisitions.load(Ordering::Relaxed),
            waited_seconds: self.writer.waited_nanos.load(Ordering::Relaxed) as f64 / NANOS,
            longest_wait_seconds: self.writer.longest_wait_nanos.load(Ordering::Relaxed) as f64
                / NANOS,
        }
    }

    /// Where a re-parse's time goes, since open (issue 247). Zero until a re-parse
    /// runs, which is itself the reading an operator wants: these are per-phase totals
    /// over the WRITER connection, and `notices` is the denominator for a per-notice
    /// mean.
    pub fn reparse_stats(&self) -> ReparseStats {
        const NANOS: f64 = 1_000_000_000.0;
        ReparseStats {
            notices: self.reparse.notices.load(Ordering::Relaxed),
            lookup_seconds: self.reparse.lookup_nanos.load(Ordering::Relaxed) as f64 / NANOS,
            clear_seconds: self.reparse.clear_nanos.load(Ordering::Relaxed) as f64 / NANOS,
            insert_seconds: self.reparse.insert_nanos.load(Ordering::Relaxed) as f64 / NANOS,
            commit_seconds: self.reparse.commit_nanos.load(Ordering::Relaxed) as f64 / NANOS,
            clear_statements: {
                let by_stmt = self.reparse.clear_stmt_nanos.lock().expect("clear stmt lock");
                let mut out: Vec<(&'static str, f64)> =
                    by_stmt.iter().map(|(label, nanos)| (*label, *nanos as f64 / NANOS)).collect();
                out.sort_by(|a, b| b.1.total_cmp(&a.1));
                out
            },
        }
    }

    /// A scratch directory beside the database file — where a projection spills its
    /// transient on-disk working set (the Phase-2 fold buckets, issue 62). Placing it
    /// next to the db keeps it on the same volume as the durable file (so it inherits
    /// the db's disk headroom) without ever entering a snapshot. The projection
    /// creates and removes it around its run.
    ///
    /// The directory is namespaced by the db FILE NAME (`{file}.{name}`), not just
    /// `name`: several databases can live in one parent dir (parallel test scratch
    /// DBs all sit in `/tmp`), and a bare `name` would make concurrent projections
    /// over different db files clobber each other's buckets.
    pub fn scratch_dir(&self, name: &str) -> std::path::PathBuf {
        let path = std::path::Path::new(&self.path);
        let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let file = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        parent.join(format!("{file}.{name}"))
    }

    /// Borrow a pooled reader for a read-only accessor — never the writer, which
    /// an ingestion job holds for the length of its transaction (issue 20).
    async fn reader(&self) -> turso::Result<Reader> {
        self.read_pool.get().await
    }

    /// Toggle foreign-key enforcement on the writer connection. The projection
    /// turns it off for the duration of a run (issue 19): it writes a
    /// self-consistent graph by construction — every referenced id is resolved
    /// before it is referenced — so the per-row FK-check lookup on millions of
    /// satellite inserts is pure overhead there, and it grows with the
    /// referenced tables (the projection's super-linear slowdown at scale). It
    /// is restored to on afterwards, so every other write path keeps the guard.
    /// Whether the writer connection currently enforces foreign keys — the
    /// test probe for the paths that turn them off around a bulk write.
    pub async fn foreign_keys_enabled(&self) -> turso::Result<bool> {
        let conn = self.conn().await;
        let mut rows = conn.query("PRAGMA foreign_keys", ()).await?;
        let on = match rows.next().await? {
            Some(row) => matches!(row.get_value(0), Ok(turso::Value::Integer(1))),
            None => false,
        };
        while rows.next().await?.is_some() {}
        Ok(on)
    }

    pub async fn set_foreign_keys(&self, on: bool) -> turso::Result<()> {
        let conn = self.conn().await;
        let mut rows = conn.query(if on { "PRAGMA foreign_keys=ON" } else { "PRAGMA foreign_keys=OFF" }, ()).await?;
        while rows.next().await?.is_some() {}
        Ok(())
    }

    /// `n` reader connections over the same database file. Readers run in
    /// parallel with each other and with the writer (WAL), so the API's fan-out
    /// never queues behind ingestion.
    pub fn readers(&self, n: usize) -> turso::Result<Arc<Readers>> {
        Readers::open(self.database.clone(), n, self.wal_gate.clone())
    }

    /// Subscribe to the change-cursor doorbell. The current value is the newest
    /// cursor known to have been committed.
    pub fn cursor_watch(&self) -> watch::Receiver<i64> {
        self.cursor.subscribe()
    }

    /// The newest committed cursor, read from the log.
    pub async fn latest_cursor(&self) -> turso::Result<i64> {
        let conn = self.reader().await?;
        max_cursor(&conn).await
    }

    /// The feed generation — bumped by every rebuild that reissues entity ids
    /// (issue 46). A `Db` wrapper over [`crate::read::feed_generation`] for the
    /// callers that hold a `Db`, not a `Connection` (webhook registration).
    pub async fn feed_generation(&self) -> turso::Result<i64> {
        let conn = self.reader().await?;
        crate::read::feed_generation(&conn).await
    }

    /// The newest committed cursor from the IN-MEMORY doorbell — no DB access at all
    /// (issue 61). The watch is seeded at open and advanced on every change-append
    /// (`publish_cursor`), so it is the newest committed cursor without a reader or a
    /// query. `/health` uses this so a liveness probe never queues behind the writer
    /// or touches turso — it stays instant regardless of `changes`-table size.
    pub fn current_cursor(&self) -> i64 {
        *self.cursor.borrow()
    }

    /// Ring the doorbell for whatever the just-committed transaction appended.
    /// Called after COMMIT, so a subscriber that reads immediately can only see
    /// durable rows.
    async fn publish_cursor(&self, conn: &Connection) -> turso::Result<()> {
        self.cursor.send_replace(max_cursor(conn).await?);
        Ok(())
    }

    /// Current-state Tenders, newest first — the `v_tenders` view, which is
    /// `MAX(seq)` per Tender (ADR-0001).
    pub async fn list_tenders(&self, limit: i64) -> turso::Result<Vec<model::Tender>> {
        let conn = self.reader().await?;
        // Drive straight off `tenders`, ordered by the maintained head-version
        // date (issue 25): the `tenders_current_published` index turns this into a
        // range scan + LIMIT, and the title is one indexed lookup per returned
        // row — O(page), not the old `v_tenders` MAX(seq) aggregation over every
        // version followed by a full sort. `title` is the head version's, resolved
        // exactly as the `v_tenders` view does.
        let mut rows = conn
            .query(
                "SELECT t.id,
                        COALESCE((SELECT x.value FROM tender_version_texts x
                                   WHERE x.tender_id = t.id AND x.seq = t.current_seq AND x.field = 'title'
                                   ORDER BY (x.lot_id IS NULL) DESC, (x.lang = 'ENG') DESC
                                   LIMIT 1), '(untitled)')
                   FROM tenders t
                  WHERE t.current_published_at IS NOT NULL
                  ORDER BY t.current_published_at DESC, t.id DESC
                  LIMIT ?",
                (Value::Integer(limit),),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(model::Tender { id: int(&row, 0), title: text(&row, 1) });
        }
        Ok(out)
    }

    /// The current (newest) fetch of a package, if any.
    pub async fn latest_fetch(&self, source: &str, kind: &str, period: &str) -> turso::Result<Option<Fetch>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT source, kind, period, url, sha256, bytes, fetched_at, path
                 FROM fetches WHERE source = ? AND kind = ? AND period = ?
                 ORDER BY id DESC LIMIT 1",
                (t(source), t(kind), t(period)),
            )
            .await?;
        let Some(row) = rows.next().await? else { return Ok(None) };
        Ok(Some(Fetch {
            source: text(&row, 0),
            kind: text(&row, 1),
            period: text(&row, 2),
            url: text(&row, 3),
            sha256: text(&row, 4),
            bytes: int(&row, 5),
            fetched_at: int(&row, 6),
            path: text(&row, 7),
        }))
    }

    /// One page of DISTINCT registered packages, keyed and ordered by each
    /// package's newest fetch row id, strictly above `after_id` — the D4 re-hash
    /// probe's sampling cursor (issue 173 / dr-premise §6). Grouping collapses a
    /// package's re-fetch versions to one probe target; ordering by the group's
    /// MAX(id) gives a stable cycle that a stored cursor can walk and wrap. The
    /// registry is hundreds of rows, so the aggregate is trivially cheap.
    pub async fn registry_page(
        &self,
        after_id: i64,
        limit: usize,
    ) -> turso::Result<Vec<(i64, String, String, String)>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT MAX(id) AS newest, source, kind, period
                 FROM fetches GROUP BY source, kind, period
                 HAVING MAX(id) > ? ORDER BY newest LIMIT ?",
                (Value::Integer(after_id), Value::Integer(limit as i64)),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((int(&row, 0), text(&row, 1), text(&row, 2), text(&row, 3)));
        }
        Ok(out)
    }

    /// One cursor-resumable slice of the D5 reveal recheck (issue 173 /
    /// dr-premise §6): does the corpus keep BT-198's promise? A withheld field
    /// carries a "publish later" date; once that date passes, SOME later notice
    /// of the same tender should carry the value — visible here as a later
    /// version whose own privacy sections no longer name the same field.
    ///
    /// SLICED, not capped (issue 274): the first capped form bounded only the
    /// reveal-EXISTS pass, while its population aggregates (`dated`/`due`, the
    /// by-field group-by) still joined the ENTIRE FieldsPrivacy cohort against
    /// `notice_dates`/`notice_codes` — 18+ min at one saturated core on prod
    /// with no cancellation point; the service was restarted twice on
    /// 2026-08-24 to get rid of one run. Every query below is bounded to the
    /// `after < notice_id <= upto` range instead, where `upto` is picked so the
    /// range holds ~`slice` FieldsPrivacy sections (whole notices — the range
    /// may overshoot by the boundary notice's remaining sections). Consecutive
    /// runs walk the cohort behind the supervisor's persisted cursor and wrap,
    /// exactly like D4's re-hash probe. The range is an index seek off
    /// `notice_sections_kind_notice`; the bare (kind) index re-scans the cohort
    /// from the start every slice (measured: reveal_cursor_probe).
    ///
    /// Still deliberately hits the BASE tables, never the
    /// `notice_withheld_fields` VIEW: that view carries a correlated subquery
    /// per section, so any aggregate over it re-evaluates those per row (the
    /// first D5 form ran >12 min on prod exactly that way, same day).
    ///
    /// `withheld_total` is the one whole-cohort number kept per run: a bare
    /// index-entry count with no joins, cheap at any population.
    pub async fn reveal_recheck(
        &self,
        now: i64,
        after: i64,
        slice: usize,
    ) -> turso::Result<RevealSlice> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM notice_sections WHERE kind = 'FieldsPrivacy'",
                (),
            )
            .await?;
        let withheld_total = match rows.next().await? {
            Some(row) => int(&row, 0),
            None => 0,
        };
        drop(rows);

        // The slice boundary: the highest notice_id among the next `slice`
        // cohort rows. The aggregates below use `<= upto`, so the boundary
        // notice is always processed WHOLE and the cursor can stand on it.
        let mut rows = conn
            .query(
                "SELECT MAX(notice_id), COUNT(*) FROM (
                   SELECT notice_id FROM notice_sections
                    WHERE kind = 'FieldsPrivacy' AND notice_id > ?
                    ORDER BY notice_id LIMIT ?)",
                (Value::Integer(after), Value::Integer(slice as i64)),
            )
            .await?;
        let (upto, picked) = match rows.next().await? {
            Some(row) => (opt_int_of(&row, 0), int(&row, 1)),
            None => (None, 0),
        };
        drop(rows);
        let wrapped = (picked as usize) < slice;
        let Some(upto) = upto else {
            // Cursor already past the cohort's end: an empty, wrapped slice.
            return Ok(RevealSlice {
                withheld_total,
                after,
                upto: after,
                wrapped: true,
                sections: 0,
                dated: 0,
                due: 0,
                checked: 0,
                revealed: 0,
                no_later: 0,
                by_field: Vec::new(),
            });
        };
        let range = [Value::Integer(after), Value::Integer(upto)];

        // Slice counts: `sections` = FieldsPrivacy sections in range; `dated`/
        // `due` = their BT-198 date rows, respectively any and already-passed.
        let mut rows = conn
            .query(
                "SELECT
                   (SELECT COUNT(*) FROM notice_sections
                     WHERE kind = 'FieldsPrivacy' AND notice_id > ? AND notice_id <= ?),
                   COUNT(*),
                   COALESCE(SUM(CASE WHEN d.utc_seconds <= ? THEN 1 ELSE 0 END), 0)
                 FROM notice_sections s
                 JOIN notice_dates d
                   ON d.notice_id = s.notice_id AND d.section_id = s.section_id
                      AND d.field_id LIKE 'BT-198%'
                 WHERE s.kind = 'FieldsPrivacy' AND s.notice_id > ? AND s.notice_id <= ?",
                (
                    range[0].clone(),
                    range[1].clone(),
                    Value::Integer(now),
                    range[0].clone(),
                    range[1].clone(),
                ),
            )
            .await?;
        let (sections, dated, due) = match rows.next().await? {
            Some(row) => (int(&row, 0), int(&row, 1), int(&row, 2)),
            None => (0, 0, 0),
        };
        drop(rows);

        // Reveal check over every due row in the slice. `revealed` = a later
        // version of the same tender exists whose notice no longer withholds
        // the same BT-195 field. `has_later` splits the failures honestly (the
        // campaign's deferred acceptance metric): a due row with NO later
        // version yet is a promise still awaitable, while a later version that
        // STILL withholds is the promise broken. Every lookup is an index
        // seek: due rows by the (kind, notice_id) range, the version hop by
        // `tender_versions_notice`, the "still withheld?" test by the
        // section/code PK prefixes.
        let mut rows = conn
            .query(
                "SELECT COALESCE(SUM(revealed), 0), COALESCE(SUM(has_later), 0), COUNT(*) FROM (
                   SELECT CASE WHEN EXISTS (
                     SELECT 1 FROM tender_versions tv1
                     JOIN tender_versions tv2
                       ON tv2.tender_id = tv1.tender_id AND tv2.seq > tv1.seq
                     WHERE tv1.caused_by_notice_id = due.notice_id
                       AND NOT EXISTS (
                         SELECT 1 FROM notice_sections s2
                         JOIN notice_codes c2
                           ON c2.notice_id = s2.notice_id AND c2.section_id = s2.section_id
                              AND c2.field_id LIKE 'BT-195%'
                         WHERE s2.notice_id = tv2.caused_by_notice_id
                           AND s2.kind = 'FieldsPrivacy'
                           AND c2.code = due.field)
                   ) THEN 1 ELSE 0 END AS revealed,
                   CASE WHEN EXISTS (
                     SELECT 1 FROM tender_versions tv1
                     JOIN tender_versions tv2
                       ON tv2.tender_id = tv1.tender_id AND tv2.seq > tv1.seq
                     WHERE tv1.caused_by_notice_id = due.notice_id
                   ) THEN 1 ELSE 0 END AS has_later
                   FROM (
                     SELECT s.notice_id AS notice_id, c.code AS field
                     FROM notice_sections s
                     JOIN notice_codes c
                       ON c.notice_id = s.notice_id AND c.section_id = s.section_id
                          AND c.field_id LIKE 'BT-195%'
                     JOIN notice_dates d
                       ON d.notice_id = s.notice_id AND d.section_id = s.section_id
                          AND d.field_id LIKE 'BT-198%'
                     WHERE s.kind = 'FieldsPrivacy' AND d.utc_seconds <= ?
                       AND s.notice_id > ? AND s.notice_id <= ?) AS due)",
                (Value::Integer(now), range[0].clone(), range[1].clone()),
            )
            .await?;
        let (revealed, with_later, checked) = match rows.next().await? {
            Some(row) => (int(&row, 0), int(&row, 1), int(&row, 2)),
            None => (0, 0, 0),
        };
        drop(rows);

        let mut rows = conn
            .query(
                "SELECT c.code, COUNT(*)
                 FROM notice_sections s
                 JOIN notice_dates d
                   ON d.notice_id = s.notice_id AND d.section_id = s.section_id
                      AND d.field_id LIKE 'BT-198%' AND d.utc_seconds <= ?
                 JOIN notice_codes c
                   ON c.notice_id = s.notice_id AND c.section_id = s.section_id
                      AND c.field_id LIKE 'BT-195%'
                 WHERE s.kind = 'FieldsPrivacy' AND s.notice_id > ? AND s.notice_id <= ?
                 GROUP BY c.code ORDER BY 2 DESC LIMIT 12",
                (Value::Integer(now), range[0].clone(), range[1].clone()),
            )
            .await?;
        let mut by_field = Vec::new();
        while let Some(row) = rows.next().await? {
            by_field.push((text(&row, 0), int(&row, 1)));
        }
        Ok(RevealSlice {
            withheld_total,
            after,
            upto,
            wrapped,
            sections,
            dated,
            due,
            checked,
            revealed,
            no_later: checked - with_later,
            by_field,
        })
    }

    /// Highest period key with the given prefix, e.g. prefix `2026-` over
    /// zero-padded daily periods yields the newest issue. Periods are
    /// zero-padded exactly so that MAX() is the newest.
    pub async fn latest_fetch_period_max(
        &self,
        source: &str,
        kind: &str,
        period_prefix: &str,
    ) -> turso::Result<Option<String>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT MAX(period) FROM fetches WHERE source = ? AND kind = ? AND period LIKE ?",
                (t(source), t(kind), t(format!("{period_prefix}%"))),
            )
            .await?;
        let Some(row) = rows.next().await? else { return Ok(None) };
        Ok(match row.get_value(0) {
            Ok(Value::Text(s)) => Some(s),
            _ => None,
        })
    }

    pub async fn record_fetch(&self, f: &Fetch) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "INSERT INTO fetches(source, kind, period, url, sha256, bytes, fetched_at, path)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
            (
                t(&f.source),
                t(&f.kind),
                t(&f.period),
                t(&f.url),
                t(&f.sha256),
                Value::Integer(f.bytes),
                Value::Integer(f.fetched_at),
                t(&f.path),
            ),
        )
        .await?;
        Ok(())
    }

    /// The current file version of each archived package, newest row per
    /// period — what the processor walks. `period` narrows to a single one.
    pub async fn current_packages(
        &self,
        source: &str,
        kind: &str,
        period: Option<&str>,
    ) -> turso::Result<Vec<Package>> {
        let conn = self.reader().await?;
        // A re-fetched package lands as a new row (fetch.rs), so the highest id
        // per period is the current version.
        let mut rows = conn
            .query(
                "SELECT id, period, path FROM fetches
                 WHERE id IN (SELECT MAX(id) FROM fetches
                              WHERE source = ? AND kind = ? AND (? IS NULL OR period = ?)
                              GROUP BY period)
                 ORDER BY period",
                (t(source), t(kind), opt_text(period), opt_text(period)),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(Package { fetch_id: int(&row, 0), period: text(&row, 1), path: text(&row, 2) });
        }
        Ok(out)
    }

    /// Record a Notice together with whatever its profile parser made of the
    /// payload, atomically: a notice is either absent, or present with its
    /// complete parsed form — never half-imported (ADR-0004).
    ///
    /// Returns false when this identity is already known, which is what makes
    /// re-processing a package idempotent.
    pub async fn record_notice(&self, n: &Notice, parse: &Parse) -> turso::Result<bool> {
        let conn = self.conn().await;
        conn.execute("BEGIN IMMEDIATE", ()).await?;
        let result = self.record_notice_tx(&conn, n, parse).await;
        // turso 0.7.0 poisons the open transaction if a write future is
        // abandoned, so the rollback is unconditional on the error path.
        match result {
            Ok(inserted) => {
                conn.execute("COMMIT", ()).await?;
                Ok(inserted)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                Err(e)
            }
        }
    }

    async fn record_notice_tx(&self, conn: &Connection, n: &Notice, parse: &Parse) -> turso::Result<bool> {
        if !self.insert_notice_row(conn, n).await? {
            return Ok(false);
        }
        let Some(id) = self.notice_id(conn, n).await? else {
            return Ok(false);
        };
        match parse {
            Parse::Pending => {}
            Parse::Parsed(parsed) => {
                self.insert_parsed(conn, id, parsed).await?;
                self.set_parse_state(conn, id, "parsed").await?;
            }
            Parse::Quarantined { reason, detail } => {
                self.set_parse_state(conn, id, "quarantined").await?;
                conn.execute(
                    "INSERT OR IGNORE INTO quarantine(notice_id, fetch_id, member_path, content_hash,
                         profile, reason, detail, first_seen)
                     VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
                    (
                        Value::Integer(id),
                        Value::Integer(n.fetch_id),
                        t(&n.member_path),
                        t(&n.content_hash),
                        t(&n.profile),
                        t(reason),
                        opt_text(detail.as_deref()),
                        Value::Integer(n.ingested_at),
                    ),
                )
                .await?;
            }
        }
        Ok(true)
    }

    async fn notice_id(&self, conn: &Connection, n: &Notice) -> turso::Result<Option<i64>> {
        let mut rows = conn
            .query(
                "SELECT id FROM notices WHERE source = ? AND publication_id = ? AND content_hash = ?",
                (t(&n.source), t(&n.publication_id), t(&n.content_hash)),
            )
            .await?;
        Ok(rows.next().await?.map(|row| int(&row, 0)))
    }

    /// The id and parse state of a notice by identity, if it exists.
    async fn notice_state(&self, conn: &Connection, n: &Notice) -> turso::Result<Option<(i64, String)>> {
        let mut rows = conn
            .query(
                "SELECT id, parse_state FROM notices
                  WHERE source = ? AND publication_id = ? AND content_hash = ?",
                (t(&n.source), t(&n.publication_id), t(&n.content_hash)),
            )
            .await?;
        Ok(rows.next().await?.map(|row| (int(&row, 0), text(&row, 1))))
    }

    /// Re-attempt one member whose earlier ingest quarantined it, writing its
    /// parsed layer IN PLACE when it now parses — the reprocess mechanism the
    /// reclaim program was missing (issues 72/73). Transactional and idempotent:
    /// the parsed-layer insert, the `parsed` transition (which clears `projected`
    /// so the trailing projection re-folds it) and the `reprocessed_at` flag
    /// commit together, so a crash leaves the member held and a re-run redoes it.
    ///
    /// `n.ingested_at` is the reprocess wall-clock, recorded as `reprocessed_at`.
    /// An already-parsed notice keeps its original `ingested_at`; only its parse
    /// state, `projected` watermark and resolved instants change — the tender
    /// layer orders by `published_at`, never `ingested_at`, so a reclaimed fold is
    /// byte-identical to a fresh ingest of the member.
    pub async fn reclaim_notice(&self, n: &Notice, parse: &Parse) -> turso::Result<Reclaim> {
        let conn = self.conn().await;
        conn.execute("BEGIN IMMEDIATE", ()).await?;
        let result = self.reclaim_notice_tx(&conn, n, parse).await;
        match result {
            Ok(outcome) => {
                conn.execute("COMMIT", ()).await?;
                Ok(outcome)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                Err(e)
            }
        }
    }

    /// Every table [`Db::insert_parsed`] writes — the notice's whole parsed layer.
    /// A re-parse must clear exactly this set, so the two lists are maintained
    /// together; `a_reparse_clears_every_parsed_table` fails if the schema grows a
    /// `notice_*` table that this misses, which is the drift that would otherwise
    /// leave orphan rows behind a re-parse.
    /// ORDER MATTERS: `notice_sections` is last. The value tables reference
    /// `notices(id)` only, so their order among themselves is free, but
    /// `organization_mentions` carries `FOREIGN KEY (notice_id, section_id)
    /// REFERENCES notice_sections` — the one FK pointing INTO the parsed layer —
    /// so sections cannot go before their referrers are gone.
    const PARSED_TABLES: [&'static str; 9] = [
        "notice_texts",
        "notice_codes",
        "notice_classifications",
        "notice_dates",
        "notice_amounts",
        "notice_numbers",
        "notice_integers",
        "notice_ids",
        "notice_sections",
    ];

    /// Drop one notice's parsed layer, leaving the `notices` row itself.
    ///
    /// Also drops the notice's `organization_mentions`, and that is not an
    /// overreach — it is forced, and it is recoverable. Forced: those rows carry
    /// an FK onto `notice_sections`, so sections cannot be replaced while they
    /// exist (prod taught this — the first re-parse run died on `immediate foreign
    /// key constraint failed`). Recoverable: mentions are written by the
    /// PROJECTION's Phase 1, not by parse, and `resolve_mentions` preloads
    /// existing rows per notice for idempotency — so a re-parsed notice, which
    /// leaves here with `projected = 0`, has its mentions re-derived against the
    /// NEW section ids by the fold that follows. Keeping the stale rows was never
    /// an option anyway: they point at section ids the re-parse has deleted.
    ///
    /// An organization whose last mention this removes survives with a zero
    /// mention count until the re-fold re-mentions it (the resolver dedupes by
    /// identity, so it is the same organization row, not a new one).
    async fn clear_parsed(
        &self,
        conn: &Connection,
        id: i64,
        keep: &std::collections::HashSet<&str>,
    ) -> turso::Result<()> {
        // Each step names itself on failure. Two prod re-parse runs died with a bare
        // "immediate foreign key constraint failed" (jobs 721 and 733), which says
        // an FK broke but not WHICH statement broke it — and the obvious suspect,
        // the mentions/sections ordering below, is demonstrably not it: a store test
        // re-parses a mentioned notice successfully. Without the statement, the next
        // occurrence is another round of guessing.
        // The canonical party rows come first, and finding out WHY took two prod
        // failures and a statement-level log (jobs 721/733/735): both
        // `tender_version_parties` and `tender_version_bid_parties` carry
        // `FOREIGN KEY (mention_notice_id, mention_section_id) REFERENCES
        // organization_mentions`, so a notice that has been FOLDED has canonical rows
        // pinning the very mentions this clear must remove. The mentions-before-
        // sections ordering fixed earlier was necessary and not sufficient; this is
        // the layer above it.
        //
        // Deleting them is coherent rather than collateral: `projected = 0`, set at
        // the end of this same transaction, declares exactly that everything derived
        // from this notice's parse is stale, and these rows are derived from it. The
        // re-fold rebuilds them from the new parse. It does leave a window in which
        // the Tender carries texts and amounts but no parties — inherent to
        // re-parsing in place, and the bulk path (`reclaim_only` then one rebuild)
        // spends that window with the canonical layer under reconstruction anyway.
        //
        // Only what the FK forces is deleted. `lot_results`, `bids` and `contracts`
        // also reference this notice, but by `notice_id` onto `notices`, so nothing
        // breaks by leaving them for the fold to replace — and a re-parse that
        // quietly widened into the results layer would be much harder to reason about.
        for table in ["tender_version_parties", "tender_version_bid_parties"] {
            self.step(
                conn,
                table,
                &format!("DELETE FROM {table} WHERE mention_notice_id = ?"),
                id,
            )
            .await?;
        }
        // Seek before deleting, because this one statement was the whole cost of a
        // re-parse (issue 247): 153 ms a notice, measured, scanning all 41.78M mention
        // rows even for a notice that has none. Most notices have none — the era being
        // re-parsed has mentions only where a party was extracted — so a 0.8 ms seek
        // that skips the DELETE is the difference between 3.5 and 400 members a minute.
        //
        // This is not a substitute for `organization_mentions_notice` (queued by the same
        // issue) and not made redundant by it: the index fixes the DELETE for notices
        // that DO have mentions, and this skips the statement entirely for those that do
        // not. Both are cheap; the ordering means a box whose index has not been built
        // yet is not stuck waiting for a Reindex that is queued behind the very job the
        // index would speed up.
        // Only the mentions whose SECTION is going away (issue 248).
        //
        // A mention references `(notice_id, section_id)`, so it has to go before its
        // section does — but a section the new parse re-creates under the same id does not
        // go anywhere, and neither does its mention. That distinction is worth a great deal:
        // deleting one mention row costs ~2.2 s on prod (proving that no row of
        // `tender_version_parties`' 78M references it is not index-served on the write
        // path), while keeping it costs nothing. The text era re-creates every section id it
        // had — `PROCEDURE` and `ORG-1` — and merely ADDS the award sections, so an era
        // re-parse that used to be 2,300 hours of foreign-key proving becomes none at all.
        //
        // Read the ids first, then delete each survivor-to-be by its FULL primary key: a
        // prefix `WHERE notice_id = ?` cost 10.2 s where the full key costs 2.2 s, and
        // neither the single-column index nor `defer_foreign_keys` moved the prefix form.
        let sections: Vec<String> = {
            let mut rows = conn
                .query("SELECT section_id FROM organization_mentions WHERE notice_id = ?", (Value::Integer(id),))
                .await?;
            let mut out = Vec::new();
            while let Some(row) = rows.next().await? {
                let section = text(&row, 0);
                if !keep.contains(section.as_str()) {
                    out.push(section);
                }
            }
            out
        };
        for section in &sections {
            let started = std::time::Instant::now();
            if let Err(e) = conn
                .execute(
                    "DELETE FROM organization_mentions WHERE notice_id = ? AND section_id = ?",
                    (Value::Integer(id), t(section)),
                )
                .await
            {
                eprintln!("[store] notice {id}: clear mention {section} failed: {e}");
                return Err(e);
            }
            *self
                .reparse
                .clear_stmt_nanos
                .lock()
                .expect("clear stmt lock")
                .entry("organization_mentions")
                .or_insert(0) += started.elapsed().as_nanos() as u64;
        }
        // The value tables go wholesale — nothing references them, and they are replaced
        // in full by the new parse.
        for table in Self::PARSED_TABLES {
            if table == "notice_sections" {
                continue;
            }
            self.step(conn, table, &format!("DELETE FROM {table} WHERE notice_id = ?"), id).await?;
        }
        // Sections, however, are referenced (by mentions), and a section the new parse
        // re-creates under the same id is not going anywhere — see the note above. So
        // delete only the ones that are, by full primary key, and let `insert_parsed`
        // UPSERT the survivors.
        let vanishing: Vec<String> = {
            let mut rows = conn
                .query("SELECT section_id FROM notice_sections WHERE notice_id = ?", (Value::Integer(id),))
                .await?;
            let mut out = Vec::new();
            while let Some(row) = rows.next().await? {
                let section = text(&row, 0);
                if !keep.contains(section.as_str()) {
                    out.push(section);
                }
            }
            out
        };
        for section in &vanishing {
            let started = std::time::Instant::now();
            if let Err(e) = conn
                .execute(
                    "DELETE FROM notice_sections WHERE notice_id = ? AND section_id = ?",
                    (Value::Integer(id), t(section)),
                )
                .await
            {
                eprintln!("[store] notice {id}: clear section {section} failed: {e}");
                return Err(e);
            }
            *self
                .reparse
                .clear_stmt_nanos
                .lock()
                .expect("clear stmt lock")
                .entry("notice_sections")
                .or_insert(0) += started.elapsed().as_nanos() as u64;
        }
        Ok(())
    }

    /// One notice-scoped statement, logging which one it was if it fails and timing it
    /// so the clear's cost can be attributed to a statement rather than to the phase
    /// (issue 247).
    async fn step(&self, conn: &Connection, label: &'static str, sql: &str, id: i64) -> turso::Result<()> {
        let started = std::time::Instant::now();
        let outcome = conn.execute(sql, (Value::Integer(id),)).await;
        *self
            .reparse
            .clear_stmt_nanos
            .lock()
            .expect("clear stmt lock")
            .entry(label)
            .or_insert(0) += started.elapsed().as_nanos() as u64;
        if let Err(e) = outcome {
            eprintln!("[store] notice {id}: {label} failed: {e}");
            return Err(e);
        }
        Ok(())
    }

    /// RE-PARSE an already-parsed notice in place: replace its parsed layer with
    /// `parsed` and clear its `projected` watermark so the next projection re-folds
    /// it (issue 100).
    ///
    /// This is deliberately NOT a flag on [`Db::reclaim_notice`], whose
    /// already-parsed arm exists to make a re-run a no-op and is load-bearing for
    /// the reclaim campaigns. Re-parsing is the opposite intent — overwrite what is
    /// already good, because the PARSER changed — so it gets its own entry point
    /// that says so at the call site.
    ///
    /// The clear is the whole point and the reason this cannot be done by calling
    /// `insert_parsed` on a parsed notice: that function is pure `INSERT`, so
    /// without the clear a re-parse would DOUBLE every section, text, code and id
    /// row rather than replace them — silent corruption, and worse than the missing
    /// feature it would look like it implemented. Clear + insert + the state update
    /// commit as one transaction, so a crash leaves the notice with its old parsed
    /// layer intact rather than a half-replaced one.
    ///
    /// Returns `false` when there is no notice row for `n` (nothing to re-parse);
    /// the caller counts that rather than treating it as an error, since a cohort
    /// walk can legitimately meet a member whose notice was never ingested.
    pub async fn reparse_notice(&self, n: &Notice, parsed: &Parsed) -> turso::Result<bool> {
        let conn = self.conn().await;
        conn.execute("BEGIN IMMEDIATE", ()).await?;
        // Defer the foreign-key checks to COMMIT (issue 247). Measured on prod: deleting
        // ONE mention row cost ~10 s, because `tender_version_parties` and
        // `tender_version_bid_parties` reference `organization_mentions`, and verifying
        // that no child row does so is not served by an index here — all 78,033,566 party
        // rows get walked, per notice. Reads of the same shape seek in 1 ms, so it is the
        // enforcement path rather than a missing index.
        //
        // Deferring does not weaken anything: the constraints are still checked, once, at
        // COMMIT, where a violation still aborts the whole transaction. And this clear
        // deletes the children BEFORE their parents anyway, so nothing is ever actually
        // in violation — only the proving of it was expensive.
        //
        // Best-effort: an engine without the pragma must not turn a re-parse into an
        // error, so the failure is logged once and the transaction proceeds with immediate
        // checks (correct, just slow).
        if let Err(e) = conn.execute("PRAGMA defer_foreign_keys = ON", ()).await {
            eprintln!("[store] re-parse: defer_foreign_keys unavailable ({e}); FK checks stay immediate");
        }
        let result = async {
            // Timed per phase (issue 247): the same statements are milliseconds through
            // the reader pool, so if a re-parse crawls the cost has to be attributed on
            // the connection it actually runs on.
            let t0 = std::time::Instant::now();
            let Some((id, _)) = self.notice_state(&conn, n).await? else { return Ok(false) };
            let t1 = std::time::Instant::now();
            // The sections the new parse re-creates: kept rather than deleted, so their
            // mentions survive and the expensive FK proof never runs (issue 248).
            let keep: std::collections::HashSet<&str> =
                parsed.sections.iter().map(|s| s.id.as_str()).collect();
            self.clear_parsed(&conn, id, &keep).await?;
            let t2 = std::time::Instant::now();
            self.insert_parsed(&conn, id, parsed).await?;
            let t3 = std::time::Instant::now();
            let phases = &self.reparse;
            phases.notices.fetch_add(1, Ordering::Relaxed);
            phases.lookup_nanos.fetch_add((t1 - t0).as_nanos() as u64, Ordering::Relaxed);
            phases.clear_nanos.fetch_add((t2 - t1).as_nanos() as u64, Ordering::Relaxed);
            phases.insert_nanos.fetch_add((t3 - t2).as_nanos() as u64, Ordering::Relaxed);
            // `projected = 0` is what makes the re-parse reach the canonical layer:
            // the next incremental projection takes unprojected parsed notices as
            // its change-set (issue 58). Without it the new parse rows would sit
            // there and every reader would keep seeing the old fold.
            conn.execute(
                "UPDATE notices SET parse_state = 'parsed', projected = 0,
                     published_at = ?, dispatched_at = ? WHERE id = ?",
                (opt_int(n.published_at), opt_int(n.dispatched_at), Value::Integer(id)),
            )
            .await?;
            Ok(true)
        }
        .await;
        match result {
            Ok(outcome) => {
                // Attributed separately from the statements above, and that
                // distinction is the point: prod's failure reads "IMMEDIATE foreign
                // key constraint failed" where a row-level violation in this build
                // reads "FOREIGN KEY constraint failed", so the check that fails may
                // be the transaction's rather than any single statement's. A log
                // showing this line and NO statement line means the violation is only
                // visible once the whole transaction is considered — a different bug
                // with a different fix, and worth knowing before writing either.
                let commit_started = std::time::Instant::now();
                let committed = conn.execute("COMMIT", ()).await;
                self.reparse
                    .commit_nanos
                    .fetch_add(commit_started.elapsed().as_nanos() as u64, Ordering::Relaxed);
                if let Err(e) = committed {
                    eprintln!(
                        "[store] notice re-parse COMMIT failed (no single statement did): {e}"
                    );
                    let _ = conn.execute("ROLLBACK", ()).await;
                    return Err(e);
                }
                Ok(outcome)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                Err(e)
            }
        }
    }

    async fn reclaim_notice_tx(&self, conn: &Connection, n: &Notice, parse: &Parse) -> turso::Result<Reclaim> {
        match self.notice_state(conn, n).await? {
            // Already good — never re-touch a parsed notice (guards double-writes
            // and makes a re-run a no-op). The MEMBER's ledger rows are still
            // resolved: this very member produced an identity that is parsed, so
            // its held rows document content the corpus already carries — without
            // this stamp a member whose reclaim succeeded but whose ledger write
            // missed (issue 139's second act) could never converge, because every
            // re-run would stop here and stamp nothing.
            Some((id, state)) if state == "parsed" => {
                // Opportunistic: resolves a stranded ledger row when one exists
                // (issue 139's convergence path); zero stamped is the ordinary
                // case here — most already-parsed records were never quarantined.
                self.stamp_reclaimed(conn, n, id).await?;
                Ok(Reclaim::AlreadyParsed)
            }
            // A held parse-level quarantine: the notice row exists (empty of parsed
            // values — it was quarantined before `insert_parsed` ran), so write the
            // parsed layer in place when it now parses. This is the case a plain
            // re-run of `process` can never reach (its `INSERT OR IGNORE` short-
            // circuits) — OC (issue 72) and the SDK cohort (issue 71).
            // Either way the member's held rows may live under TWO addresses: a
            // parse-level quarantine carries this notice_id, while a profile-level
            // one (quarantined at dispatch, before an identity existed) has
            // notice_id NULL and is only reachable by (fetch_id, member_path) —
            // and a prior failed reclaim creates exactly that split, because its
            // record_notice_tx inserts the notice row while the original
            // profile-level quarantine row keeps its NULL notice_id (issue 139:
            // 1,905 members stamped "notice_id = ?" that matched nothing). Both
            // UPDATEs run, disjoint by construction (`notice_id = ?` vs
            // `notice_id IS NULL`), so no row is ever stamped twice.
            Some((id, _)) => match parse {
                Parse::Parsed(parsed) => {
                    self.insert_parsed(conn, id, parsed).await?;
                    conn.execute(
                        "UPDATE notices SET parse_state = 'parsed', projected = 0,
                             published_at = ?, dispatched_at = ? WHERE id = ?",
                        (opt_int(n.published_at), opt_int(n.dispatched_at), Value::Integer(id)),
                    )
                    .await?;
                    // Zero here IS anomalous — the member came off the held
                    // list, so a ledger row must exist (issue 139's exact
                    // failure shape) — and must be loud in the journal. One
                    // benign zero exists: a multi-record FILE whose row was
                    // already resolved by an earlier record of the same run
                    // (the C01 drain fired 17 such lines), so a zero is only
                    // reported when the file has no resolved row either.
                    if self.stamp_reclaimed(conn, n, id).await? == 0 {
                        self.log_zero_stamp(conn, n, "parsed arm").await?;
                    }
                    Ok(Reclaim::Reclaimed)
                }
                _ => {
                    self.stamp_still_held(conn, parse, n.ingested_at, "notice_id = ?", vec![Value::Integer(id)])
                        .await?;
                    self.stamp_still_held(
                        conn,
                        parse,
                        n.ingested_at,
                        "fetch_id = ? AND member_path = ? AND notice_id IS NULL",
                        vec![Value::Integer(n.fetch_id), t(&n.member_path)],
                    )
                    .await?;
                    Ok(Reclaim::StillHeld)
                }
            },
            // No notice row — a profile-level quarantine (failed before an identity
            // existed). The ordinary ingest write now records it; flag the held
            // member's row by (fetch_id, member_path) so a content-hash difference
            // between the raw-bytes quarantine and the notice can't leave it stuck.
            None => {
                if !self.record_notice_tx(conn, n, parse).await? {
                    return Ok(Reclaim::AlreadyParsed);
                }
                if matches!(parse, Parse::Parsed(_)) {
                    let mut stamped = conn
                        .execute(
                            "UPDATE quarantine SET reprocessed_at = ?, skipped_at = NULL, skipped_reason = NULL
                              WHERE fetch_id = ? AND member_path = ? AND reprocessed_at IS NULL",
                            (Value::Integer(n.ingested_at), Value::Integer(n.fetch_id), t(&n.member_path)),
                        )
                        .await?;
                    // The member FILE's whole-file rejection row, when this is a
                    // `#<ordinal>` text record — see stamp_reclaimed's third
                    // address (issue 181).
                    let file = member_file(n.member_path.clone());
                    if file != n.member_path {
                        stamped += conn
                            .execute(
                                "UPDATE quarantine SET reprocessed_at = ?, skipped_at = NULL, skipped_reason = NULL
                                  WHERE fetch_id = ? AND member_path = ? AND notice_id IS NULL
                                    AND reprocessed_at IS NULL",
                                (Value::Integer(n.ingested_at), Value::Integer(n.fetch_id), t(&file)),
                            )
                            .await?;
                    }
                    // And the whole-CONTAINER rejection row — stamp_reclaimed's
                    // fourth address (issue 196).
                    if let Some(container) = member_container(&n.member_path) {
                        stamped += conn
                            .execute(
                                "UPDATE quarantine SET reprocessed_at = ?, skipped_at = NULL, skipped_reason = NULL
                                  WHERE fetch_id = ? AND member_path = ? AND notice_id IS NULL
                                    AND reprocessed_at IS NULL",
                                (Value::Integer(n.ingested_at), Value::Integer(n.fetch_id), t(&container)),
                            )
                            .await?;
                    }
                    // Zero is benign when the file's row is ALREADY resolved —
                    // the same guard the parsed arm applies above. The 1999 COR
                    // drain hit exactly this shape: a correction file's records
                    // mostly duplicate an earlier daily, so the first duplicate
                    // resolves the whole-file rejection row through the
                    // already-parsed arm, and the few genuinely-corrected
                    // records that follow (fresh identities, this path) find
                    // nothing left to stamp — four false alarms over a fully
                    // consistent ledger.
                    if stamped == 0 {
                        self.log_zero_stamp(conn, n, "fresh record path").await?;
                    }
                    Ok(Reclaim::Reclaimed)
                } else {
                    self.stamp_still_held(
                        conn,
                        parse,
                        n.ingested_at,
                        "fetch_id = ? AND member_path = ?",
                        vec![Value::Integer(n.fetch_id), t(&n.member_path)],
                    )
                    .await?;
                    Ok(Reclaim::StillHeld)
                }
            }
        }
    }

    /// Whether the member's FILE-level ledger row is already resolved
    /// (reclaimed or skipped) — the benign explanation for a zero-stamp
    /// reclaim in a multi-record file, where an earlier record of the same
    /// run resolved the row.
    async fn member_file_resolved(&self, conn: &Connection, n: &Notice) -> turso::Result<bool> {
        let file = member_file(n.member_path.clone());
        // The container address rides along (issue 196): once the first record
        // out of a whole-container rejection resolves the container row, its
        // thousands of sibling records must read as benign zeros, not alarms.
        let container = member_container(&n.member_path).unwrap_or_else(|| file.clone());
        // And the file's RESOLVED record-level siblings (issue 200's drain): a
        // text record's `#<ordinal>` names TODAY'S segmentation, so after a
        // segmentation change the file's old rows resolve under ordinals that
        // no longer align with the records stamping them — 106 false alarms in
        // the 2010 merge pass, every row of the bucket in fact resolved. A
        // resolved sibling is evidence the file's bookkeeping is live; a truly
        // stranded file (no row resolved anywhere) still alarms.
        let siblings = format!("{}#%", like_escape(&file));
        let mut rows = conn
            .query(
                "SELECT 1 FROM quarantine
                  WHERE fetch_id = ?
                    AND (member_path IN (?, ?) OR member_path LIKE ? ESCAPE '\\')
                    AND (reprocessed_at IS NOT NULL OR skipped_at IS NOT NULL)
                  LIMIT 1",
                (Value::Integer(n.fetch_id), t(&file), t(&container), t(&siblings)),
            )
            .await?;
        Ok(rows.next().await?.is_some())
    }

    /// The complement (issue 289): whether any of the member's family rows — its
    /// file, container, or a `#<ordinal>` record sibling — is still HELD (neither
    /// outcome stamp). [`Self::member_file_resolved`] alone blinds the zero-stamp
    /// alarm for a whole file the moment its FIRST record resolves, so a later
    /// record whose reclaim genuinely strands its row (an issue-139 address miss)
    /// went unlogged. The property that separates a real stranding from the three
    /// documented benign zero shapes (181's resolved file row, 196's container,
    /// 200's shifted ordinals) is exactly this: a stranding leaves unresolved
    /// family residue behind; the benign shapes leave none.
    async fn member_family_still_held(&self, conn: &Connection, n: &Notice) -> turso::Result<bool> {
        let file = member_file(n.member_path.clone());
        let container = member_container(&n.member_path).unwrap_or_else(|| file.clone());
        let siblings = format!("{}#%", like_escape(&file));
        let mut rows = conn
            .query(
                "SELECT 1 FROM quarantine
                  WHERE fetch_id = ?
                    AND (member_path IN (?, ?) OR member_path LIKE ? ESCAPE '\\')
                    AND reprocessed_at IS NULL AND skipped_at IS NULL
                  LIMIT 1",
                (Value::Integer(n.fetch_id), t(&file), t(&container), t(&siblings)),
            )
            .await?;
        Ok(rows.next().await?.is_some())
    }

    /// The zero-stamp verdict shared by both reclaim arms (issue 289): loud when
    /// the family has no resolved row at all (issue 139's original shape), loud
    /// WITH a distinguishing marker when the family is partially resolved but
    /// held residue remains (the shape `member_file_resolved` alone silenced —
    /// the residue may be exactly the row this reclaim failed to address), and
    /// silent only when the family is fully resolved (181/196/200's benign
    /// zeros). The marker keeps the irreducible ambiguity honest: a sibling
    /// legitimately awaiting its own reclaim also leaves residue, so the second
    /// line is a "look here", not a verdict. Both lines share the
    /// `reclaim stamped NO ledger rows` prefix the OPERATE journal grep watches.
    async fn log_zero_stamp(
        &self,
        conn: &Connection,
        n: &Notice,
        context: &str,
    ) -> turso::Result<()> {
        if !self.member_file_resolved(conn, n).await? {
            eprintln!(
                "[store] reclaim stamped NO ledger rows for fetch {} member {:?} \
                 ({context}) — the member reclaimed but its quarantine rows were \
                 not addressable (issue 139)",
                n.fetch_id, n.member_path
            );
        } else if self.member_family_still_held(conn, n).await? {
            eprintln!(
                "[store] reclaim stamped NO ledger rows for fetch {} member {:?} \
                 ({context}) — zero-stamp under a PARTIALLY-resolved file: held \
                 sibling rows remain, one may be this member's stranded row \
                 (issue 289)",
                n.fetch_id, n.member_path
            );
        }
        Ok(())
    }

    /// Flag a member's held ledger rows reclaimed, under BOTH addresses a row can
    /// live at: parse-level (`notice_id = ?`) and profile-level
    /// (`fetch_id`/`member_path` with `notice_id IS NULL`) — disjoint by
    /// construction, so no row is stamped twice.
    ///
    /// Reclaimed WINS over skipped (issue 288): a skip is a policy statement, a
    /// reclaim is the fact that the content now lives in the parsed layer — so
    /// every stamp here (and the None-arm's three) clears `skipped_at`/
    /// `skipped_reason` as it sets `reprocessed_at`, keeping the three ledger
    /// outcomes (outstanding / reclaimed / skipped) disjoint in the data. The
    /// mirror guard lives in `flag_skipped_members`, which never stamps a
    /// reclaimed row. Returns rows stamped: on the
    /// RECLAIMED path zero is issue 139's failure shape (the member came off the
    /// held list, so a row must exist) and the caller logs it loudly; on the
    /// ALREADY-PARSED path zero is the ordinary case (a text member's co-resident
    /// records were never quarantined — 1.06M of them in issue 183's pass — so
    /// logging there would bury the signal under half a million lines).
    async fn stamp_reclaimed(&self, conn: &Connection, n: &Notice, id: i64) -> turso::Result<u64> {
        let by_notice = conn
            .execute(
                "UPDATE quarantine SET reprocessed_at = ?, skipped_at = NULL, skipped_reason = NULL
                  WHERE notice_id = ? AND reprocessed_at IS NULL",
                (Value::Integer(n.ingested_at), Value::Integer(id)),
            )
            .await?;
        let by_member = conn
            .execute(
                "UPDATE quarantine SET reprocessed_at = ?, skipped_at = NULL, skipped_reason = NULL
                  WHERE fetch_id = ? AND member_path = ? AND notice_id IS NULL
                    AND reprocessed_at IS NULL",
                (Value::Integer(n.ingested_at), Value::Integer(n.fetch_id), t(&n.member_path)),
            )
            .await?;
        // Third address (issue 181's CF drain, caught by the zero-stamp journal
        // line): a text RECORD's path carries `#<ordinal>` while the member
        // FILE's quarantine row does not — a whole-file rejection (not-utf8,
        // unparsable-xml) holds ONE row for a file of ~1,000 records. The first
        // record reclaimed from that file resolves the file row: its content is
        // demonstrably readable and in the corpus.
        let file = member_file(n.member_path.clone());
        let by_file = if file != n.member_path {
            conn.execute(
                "UPDATE quarantine SET reprocessed_at = ?, skipped_at = NULL, skipped_reason = NULL
                  WHERE fetch_id = ? AND member_path = ? AND notice_id IS NULL
                    AND reprocessed_at IS NULL",
                (Value::Integer(n.ingested_at), Value::Integer(n.fetch_id), t(&file)),
            )
            .await?
        } else {
            0
        };
        // Fourth address (issue 196): the member's whole-CONTAINER rejection
        // row, when its path shows a nested archive. See [`member_container`].
        let by_container = if let Some(container) = member_container(&n.member_path) {
            conn.execute(
                "UPDATE quarantine SET reprocessed_at = ?, skipped_at = NULL, skipped_reason = NULL
                  WHERE fetch_id = ? AND member_path = ? AND notice_id IS NULL
                    AND reprocessed_at IS NULL",
                (Value::Integer(n.ingested_at), Value::Integer(n.fetch_id), t(&container)),
            )
            .await?
        } else {
            0
        };
        Ok(by_notice + by_member + by_file + by_container)
    }

    /// Record a re-parse attempt that left the member held (issue 87). A
    /// [`Parse::Quarantined`] re-parse rewrites the row's `reason`/`detail` to the
    /// CURRENT failure — before this, a failed reclaim left the first-ingest pair
    /// in place, so the 241 DE-1.x residuals still claimed "no vendored SDK
    /// metadata" AFTER the metadata was vendored, and the real cause was written
    /// nowhere. The first-ingest pair is preserved once in `first_reason` /
    /// `first_detail`: `first_reason IS NULL` marks a row never overwritten, and
    /// every SET expression reads the pre-update row, so both move together on the
    /// first overwrite and never again. A [`Parse::Pending`] re-parse has no
    /// failure payload — the member is still unrecognised for the same recorded
    /// reason — so only the attempt is stamped.
    ///
    /// Resolved rows (`reprocessed_at`/`skipped_at`) are terminal ledger outcomes
    /// and are never rewritten. `reprocessed_at` itself is never set here: a
    /// failed member must stay in the reclaim backlog, now under its true reason.
    async fn stamp_still_held(
        &self,
        conn: &Connection,
        parse: &Parse,
        now: i64,
        where_sql: &str,
        params: Vec<Value>,
    ) -> turso::Result<u64> {
        let (set, mut bound) = match parse {
            Parse::Quarantined { reason, detail } => (
                "first_reason = CASE WHEN first_reason IS NULL THEN reason ELSE first_reason END,
                 first_detail = CASE WHEN first_reason IS NULL THEN detail ELSE first_detail END,
                 reason = ?, detail = ?,
                 last_attempt_at = ?, attempts = COALESCE(attempts, 0) + 1",
                vec![t(reason), opt_text(detail.as_deref()), Value::Integer(now)],
            ),
            _ => (
                "last_attempt_at = ?, attempts = COALESCE(attempts, 0) + 1",
                vec![Value::Integer(now)],
            ),
        };
        bound.extend(params);
        conn.execute(
            &format!(
                "UPDATE quarantine SET {set}
                  WHERE {where_sql} AND reprocessed_at IS NULL AND skipped_at IS NULL"
            ),
            bound,
        )
        .await
    }

    /// The profile-level half of issue 87: a held member that STILL cannot
    /// produce an identity re-arrives from the walker as a fresh quarantine
    /// record, not a notice, so [`Db::reclaim_notice`] never sees it. The
    /// reprocess records the attempt on the held row through here instead.
    ///
    /// Addressed by exact `(fetch_id, member_path)` first, then by
    /// `(fetch_id, content_hash)` when that matches nothing (issue 193): a
    /// text-era record's path carries a `#<ordinal>` that is its index in
    /// TODAY'S segmentation — a parser change since the original ingest shifts
    /// later ordinals, so the exact path misses rows whose bytes are unchanged.
    /// The record's own hash still identifies them precisely; identical bytes
    /// appearing under several rows get the same (true) current failure. A
    /// record whose bytes ALSO changed is unreachable by construction and stays
    /// under its stale reason — logged, because silently it looks like issue
    /// 87 working when it is not.
    pub async fn record_reclaim_attempt(
        &self,
        fetch_id: i64,
        member_path: &str,
        content_hash: &str,
        reason: &str,
        detail: Option<&str>,
        now: i64,
    ) -> turso::Result<()> {
        let conn = self.conn().await;
        let parse =
            Parse::Quarantined { reason: reason.to_owned(), detail: detail.map(str::to_owned) };
        let by_path = self
            .stamp_still_held(
                &conn,
                &parse,
                now,
                "fetch_id = ? AND member_path = ?",
                vec![Value::Integer(fetch_id), t(member_path)],
            )
            .await?;
        if by_path > 0 {
            return Ok(());
        }
        let by_hash = self
            .stamp_still_held(
                &conn,
                &parse,
                now,
                "fetch_id = ? AND content_hash = ?",
                vec![Value::Integer(fetch_id), t(content_hash)],
            )
            .await?;
        if by_hash == 0 {
            eprintln!(
                "[store] reclaim attempt stamped NO ledger rows for fetch {fetch_id} \
                 member {member_path:?} — record bytes and path both drifted from the \
                 held row (issue 193)"
            );
        }
        Ok(())
    }

    /// Set a notice's parse state. A transition to 'parsed' also clears the
    /// incremental-projection watermark (`projected = 0`, issue 58): this is the
    /// single choke-point through which a notice's parsed layer becomes current,
    /// so clearing here keeps the change-set correct even for a future in-place
    /// re-parse path (today every parse is a fresh row, already 0).
    async fn set_parse_state(&self, conn: &Connection, id: i64, state: &str) -> turso::Result<()> {
        conn.execute(
            "UPDATE notices SET parse_state = ?1, projected = projected AND (?1 <> 'parsed')
              WHERE id = ?2",
            (t(state), Value::Integer(id)),
        )
        .await?;
        Ok(())
    }

    /// Fan one notice's parsed form out into the value tables.
    async fn insert_parsed(&self, conn: &Connection, id: i64, parsed: &Parsed) -> turso::Result<()> {
        for s in &parsed.sections {
            // UPSERT, not INSERT: a re-parse now KEEPS the sections it is about to
            // re-create (issue 248), so the row may already be there — with its mentions
            // still attached, which is the whole point. `ON CONFLICT DO UPDATE` refreshes
            // the kind and parent without a delete, so no foreign key is ever momentarily
            // violated and none has to be proven satisfied. A fresh ingest takes the
            // INSERT path exactly as before.
            if let Err(e) = conn
                .execute(
                    "INSERT INTO notice_sections(notice_id, section_id, kind, parent_section_id)
                     VALUES(?, ?, ?, ?)
                     ON CONFLICT(notice_id, section_id) DO UPDATE SET
                         kind = excluded.kind, parent_section_id = excluded.parent_section_id",
                    (Value::Integer(id), t(&s.id), t(&s.kind), opt_text(s.parent.as_deref())),
                )
                .await
            {
                // Includes the parent id, because a section whose parent is not in
                // this same batch is the one shape that can fail here on an FK.
                eprintln!(
                    "[store] notice {id}: insert section {} (kind {}, parent {:?}) failed: {e}",
                    s.id, s.kind, s.parent
                );
                return Err(e);
            }
        }
        for v in &parsed.values {
            let key = (Value::Integer(id), t(&v.section_id), t(&v.field_id), Value::Integer(v.ordinal));
            match &v.value {
                NoticeValue::Text { lang, value } => {
                    conn.execute(
                        "INSERT INTO notice_texts(notice_id, section_id, field_id, ordinal, lang, value)
                         VALUES(?, ?, ?, ?, ?, ?)",
                        (key.0, key.1, key.2, key.3, opt_text(lang.as_deref()), t(value)),
                    )
                    .await?;
                }
                NoticeValue::Code { list, code } => {
                    conn.execute(
                        "INSERT INTO notice_codes(notice_id, section_id, field_id, ordinal, list_name, code)
                         VALUES(?, ?, ?, ?, ?, ?)",
                        (key.0, key.1, key.2, key.3, opt_text(list.as_deref()), t(code)),
                    )
                    .await?;
                }
                NoticeValue::Classification { scheme, code } => {
                    conn.execute(
                        "INSERT INTO notice_classifications(notice_id, section_id, field_id, ordinal, scheme, code)
                         VALUES(?, ?, ?, ?, ?, ?)",
                        (key.0, key.1, key.2, key.3, t(scheme), t(code)),
                    )
                    .await?;
                }
                NoticeValue::Amount { cents, currency } => {
                    conn.execute(
                        "INSERT INTO notice_amounts(notice_id, section_id, field_id, ordinal, cents, currency)
                         VALUES(?, ?, ?, ?, ?, ?)",
                        (key.0, key.1, key.2, key.3, Value::Integer(*cents), t(currency)),
                    )
                    .await?;
                }
                NoticeValue::Date { utc_seconds, offset_minutes, has_time } => {
                    conn.execute(
                        "INSERT INTO notice_dates(notice_id, section_id, field_id, ordinal,
                             utc_seconds, offset_minutes, has_time)
                         VALUES(?, ?, ?, ?, ?, ?, ?)",
                        (
                            key.0,
                            key.1,
                            key.2,
                            key.3,
                            Value::Integer(*utc_seconds),
                            Value::Integer(*offset_minutes),
                            Value::Integer(i64::from(*has_time)),
                        ),
                    )
                    .await?;
                }
                NoticeValue::Integer(value) => {
                    conn.execute(
                        "INSERT INTO notice_integers(notice_id, section_id, field_id, ordinal, value)
                         VALUES(?, ?, ?, ?, ?)",
                        (key.0, key.1, key.2, key.3, Value::Integer(*value)),
                    )
                    .await?;
                }
                NoticeValue::Number { value, unit } => {
                    conn.execute(
                        "INSERT INTO notice_numbers(notice_id, section_id, field_id, ordinal, value, unit)
                         VALUES(?, ?, ?, ?, ?, ?)",
                        (key.0, key.1, key.2, key.3, Value::Real(*value), opt_text(unit.as_deref())),
                    )
                    .await?;
                }
                NoticeValue::Id { scheme, value, is_ref } => {
                    conn.execute(
                        "INSERT INTO notice_ids(notice_id, section_id, field_id, ordinal, scheme, value, is_ref)
                         VALUES(?, ?, ?, ?, ?, ?, ?)",
                        (
                            key.0,
                            key.1,
                            key.2,
                            key.3,
                            opt_text(scheme.as_deref()),
                            t(value),
                            Value::Integer(i64::from(*is_ref)),
                        ),
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }

    async fn insert_notice_row(&self, conn: &Connection, n: &Notice) -> turso::Result<bool> {
        let changed = conn
            .execute(
                "INSERT OR IGNORE INTO notices(source, publication_id, content_hash, profile,
                     declared_version, fetch_id, member_path, ingested_at, published_at, dispatched_at)
                 VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    t(&n.source),
                    t(&n.publication_id),
                    t(&n.content_hash),
                    t(&n.profile),
                    opt_text(n.declared_version.as_deref()),
                    Value::Integer(n.fetch_id),
                    t(&n.member_path),
                    Value::Integer(n.ingested_at),
                    opt_int(n.published_at),
                    opt_int(n.dispatched_at),
                ),
            )
            .await?;
        Ok(changed > 0)
    }

    /// Quarantine a payload we could not turn into a Notice. Returns false if
    /// this exact payload is already quarantined.
    pub async fn insert_quarantine(&self, q: &Quarantined) -> turso::Result<bool> {
        let conn = self.conn().await;
        let changed = conn
            .execute(
                "INSERT OR IGNORE INTO quarantine(fetch_id, member_path, content_hash, profile,
                     reason, detail, first_seen)
                 VALUES(?, ?, ?, ?, ?, ?, ?)",
                (
                    Value::Integer(q.fetch_id),
                    t(&q.member_path),
                    t(&q.content_hash),
                    opt_text(q.profile.as_deref()),
                    t(&q.reason),
                    opt_text(q.detail.as_deref()),
                    Value::Integer(q.first_seen),
                ),
            )
            .await?;
        Ok(changed > 0)
    }

    /// The archived packages still holding quarantined members of a bucket —
    /// `reason`, plus an optional `detail LIKE` pattern and exact `profile` — that
    /// no prior pass has resolved, whose `fetch_id` exceeds `after`. The reprocess
    /// job's resumable work list: it returns distinct packages (not rows), so the
    /// result is bounded by package count regardless of how large the bucket is,
    /// and resolved packages fall out of a re-query automatically. BOTH terminal
    /// stamps disqualify a row: `reprocessed_at` (reclaimed) and `skipped_at`
    /// (documented policy skip, issue 84) — before the latter was filtered, the
    /// 2008 monthlies sat on every unparsable-xml work list forever, their 593k
    /// already-skipped DTD siblings re-walked and re-declined each run (job 653's
    /// "599262 skipped by dispatch policy" over a 4,441-row bucket, issue 197).
    pub async fn quarantine_reclaim_packages(
        &self,
        reason: &str,
        detail_like: Option<&str>,
        profile: Option<&str>,
        after: i64,
    ) -> turso::Result<Vec<(i64, String, String)>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT DISTINCT q.fetch_id, f.source, f.path
                   FROM quarantine q JOIN fetches f ON f.id = q.fetch_id
                  WHERE q.reason = ?1 AND q.reprocessed_at IS NULL AND q.skipped_at IS NULL
                    AND (?2 IS NULL OR q.detail LIKE ?2)
                    AND (?3 IS NULL OR q.profile = ?3)
                    AND q.fetch_id > ?4
                  ORDER BY q.fetch_id",
                (t(reason), opt_text(detail_like), opt_text(profile), Value::Integer(after)),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((int(&row, 0), text(&row, 1), text(&row, 2)));
        }
        Ok(out)
    }

    /// The still-held member FILES of one package for a bucket (issue 77): the
    /// distinct `member_path`s (text-era `#<ordinal>` suffix stripped to the
    /// member file the walker yields) matching the bucket in `fetch_id`, not yet
    /// resolved (neither reclaimed nor policy-skipped — both stamps are terminal,
    /// issue 197). Lets the reprocess parse ONLY these members and skip the rest —
    /// a sparse bucket re-parses `held/total` of the package instead of all of it.
    /// Seeks by `fetch_id` (the leading column of the quarantine unique index), so
    /// it is a bounded per-package lookup.
    /// The fetch's held WHOLE-BUNDLE rows under an unreadable-at-source reason
    /// (issue 202): paths with no record (`#`) or inner-file (`!`) component,
    /// reason `unreadable …`. These members never yielded records, so the
    /// dispatch policy must not let them supersede a readable twin.
    pub async fn unreadable_bundle_members(
        &self,
        fetch_id: i64,
    ) -> turso::Result<std::collections::HashSet<String>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT member_path FROM quarantine
                  WHERE fetch_id = ?1 AND reason LIKE 'unreadable %'
                    AND reprocessed_at IS NULL AND skipped_at IS NULL
                    AND member_path NOT LIKE '%!%' AND member_path NOT LIKE '%#%'",
                (Value::Integer(fetch_id),),
            )
            .await?;
        let mut out = std::collections::HashSet::new();
        while let Some(row) = rows.next().await? {
            out.insert(text(&row, 0));
        }
        Ok(out)
    }

    pub async fn quarantine_held_member_files(
        &self,
        fetch_id: i64,
        reason: &str,
        detail_like: Option<&str>,
        profile: Option<&str>,
    ) -> turso::Result<std::collections::HashSet<String>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT DISTINCT member_path FROM quarantine
                  WHERE fetch_id = ?1 AND reason = ?2
                    AND reprocessed_at IS NULL AND skipped_at IS NULL
                    AND (?3 IS NULL OR detail LIKE ?3)
                    AND (?4 IS NULL OR profile = ?4)",
                (Value::Integer(fetch_id), t(reason), opt_text(detail_like), opt_text(profile)),
            )
            .await?;
        let mut out = std::collections::HashSet::new();
        while let Some(row) = rows.next().await? {
            out.insert(member_file(text(&row, 0)));
        }
        Ok(out)
    }

    /// The archived packages holding READABLE notices of these profiles, whose
    /// `fetch_id` exceeds `after` — the re-parse job's resumable work list (issue
    /// 100), the profile-scoped twin of [`Db::quarantine_reclaim_packages`].
    ///
    /// Readable is `parsed` OR **`pending`**, and the second is the whole point of
    /// this being a set rather than an equality. A source can land on the
    /// dry-first rung — recorded, archived, identity only, `parse_state='pending'`
    /// — before its parser exists, which is how issue 342 brought UK FTS in: 7,243
    /// releases sat pending for a day while the mapping was written. Selecting
    /// only `parsed` left that rung with NO WAY BACK: the notices are not
    /// quarantined so `reprocess` does not see them, and `process` dedupes them as
    /// duplicates, so nothing in the system would ever read a payload it had
    /// already stored. A pending notice is precisely one whose payload has never
    /// been read, which makes it the clearest possible candidate for a re-read.
    ///
    /// Widening it costs nothing when a parser is absent: the payload is offered,
    /// the profile returns `Pending` again, and the row is unchanged.
    ///
    /// The two differ in a way worth stating, because it decides how the job knows
    /// it is done. A reclaim's work list SHRINKS as it runs: a reclaimed row gets
    /// `reprocessed_at` and its package drops out of a re-query. A re-parse has no
    /// such stamp — a re-parsed notice is still a parsed notice of the same
    /// profile, so this list is IDEMPOTENT and re-querying returns the same
    /// packages. Progress therefore lives entirely in the `after` cursor, and a
    /// resumed run must carry it or redo work it already did.
    pub async fn reparse_packages(
        &self,
        profiles: &[&str],
        after: i64,
    ) -> turso::Result<Vec<(i64, String, String)>> {
        if profiles.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.reader().await?;
        let list = crate::canonical::placeholders(profiles.len());
        let sql = format!(
            "SELECT DISTINCT n.fetch_id, f.source, f.path
               FROM notices n JOIN fetches f ON f.id = n.fetch_id
              WHERE n.parse_state IN ('parsed', 'pending')
                AND n.profile IN ({list}) AND n.fetch_id > ?
              ORDER BY n.fetch_id"
        );
        let mut params: Vec<Value> = profiles.iter().map(|p| t(*p)).collect();
        params.push(Value::Integer(after));
        let mut rows = conn.query(&sql, params).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((int(&row, 0), text(&row, 1), text(&row, 2)));
        }
        Ok(out)
    }

    /// The member FILES of one package holding readable notices of these profiles —
    /// what a re-parse must walk in that package, and nothing else (the issue-77
    /// sparse-bucket discipline). `member_file` strips the text-era `#<ordinal>`
    /// suffix so the set matches what the archive walker yields.
    ///
    /// `pending` counts as readable alongside `parsed`, for the reason given on
    /// [`Db::reparse_packages`].
    pub async fn parsed_member_files(
        &self,
        fetch_id: i64,
        profiles: &[&str],
    ) -> turso::Result<std::collections::HashSet<String>> {
        if profiles.is_empty() {
            return Ok(Default::default());
        }
        let conn = self.reader().await?;
        let list = crate::canonical::placeholders(profiles.len());
        let sql = format!(
            "SELECT DISTINCT member_path FROM notices
              WHERE fetch_id = ? AND parse_state IN ('parsed', 'pending')
                AND profile IN ({list})"
        );
        let mut params: Vec<Value> = vec![Value::Integer(fetch_id)];
        params.extend(profiles.iter().map(|p| t(*p)));
        let mut rows = conn.query(&sql, params).await?;
        let mut out = std::collections::HashSet::new();
        while let Some(row) = rows.next().await? {
            out.insert(member_file(text(&row, 0)));
        }
        Ok(out)
    }

    /// Notice counts per mapping profile — the era-split check and the
    /// dashboard's coverage breakdown.
    pub async fn notice_counts_by_profile(&self) -> turso::Result<Vec<(String, i64)>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query("SELECT profile, COUNT(*) FROM notices GROUP BY profile ORDER BY profile", ())
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((text(&row, 0), int(&row, 1)));
        }
        Ok(out)
    }

    /// Quarantine counts per reason — the headline data-quality metric
    /// (ADR-0004), broken down.
    /// The reason breakdown split three ways: still-held, reclaimed, and
    /// skipped-as-duplicate (issue 137 / #29 criterion 6).
    ///
    /// [`Self::quarantine_counts_by_reason`] counts every row a reason ever
    /// held, which was honest when nothing had been reclaimed and became a
    /// misstatement the moment reprocessing worked: 1,734,594 of 2,419,410 rows
    /// were already resolved while the dashboard still presented the whole
    /// figure as "quarantined". `unknown-field-code` showed as a 577K gap with
    /// 10 rows actually left.
    ///
    /// The three states are disjoint and cover the table, so the row sums back
    /// to the all-time count — nothing is hidden by splitting it, which is the
    /// property that makes the split safe to show a user.
    pub async fn quarantine_counts_by_reason_split(
        &self,
    ) -> turso::Result<Vec<(String, i64, i64, i64)>> {
        let conn = self.reader().await?;
        // `reprocessed_at` wins over `skipped_at` in the CASE so a row that
        // somehow carried both is counted as RECLAIMED — the stronger claim,
        // and the one that would be visible as an over-count rather than
        // hiding rows in the quieter bucket.
        let mut rows = conn
            .query(
                "SELECT reason,
                        SUM(CASE WHEN reprocessed_at IS NULL AND skipped_at IS NULL THEN 1 ELSE 0 END),
                        SUM(CASE WHEN reprocessed_at IS NOT NULL THEN 1 ELSE 0 END),
                        SUM(CASE WHEN reprocessed_at IS NULL AND skipped_at IS NOT NULL THEN 1 ELSE 0 END)
                   FROM quarantine
                  GROUP BY reason
                  ORDER BY reason",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((text(&row, 0), int(&row, 1), int(&row, 2), int(&row, 3)));
        }
        Ok(out)
    }

    pub async fn quarantine_counts_by_reason(&self) -> turso::Result<Vec<(String, i64)>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query("SELECT reason, COUNT(*) FROM quarantine GROUP BY reason ORDER BY reason", ())
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((text(&row, 0), int(&row, 1)));
        }
        Ok(out)
    }

    /// The field codes behind the `unknown-field-code` bucket, biggest first
    /// (issue 30). Each detail is `line <n>: <code>`; grouping by the code (not
    /// the whole detail, which carries the line number) shows whether one legacy
    /// code drives the bucket — it did: `OC` on the ISO-era text records.
    /// STILL-HELD rows only (issue 185): a quarantine row is retained as a
    /// historical record after reclaim, so counting all rows counts work that is
    /// already done — the OC bucket read as a 577K gap while 10 rows were held
    /// (#29 criterion 6, the same rule `by_reason` follows).
    pub async fn quarantine_field_code_gaps(&self, limit: i64) -> turso::Result<Vec<(String, i64)>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT substr(detail, instr(detail, ': ') + 2) AS code, COUNT(*) c
                   FROM quarantine WHERE reason = 'unknown-field-code'
                    AND reprocessed_at IS NULL AND skipped_at IS NULL
                  GROUP BY code ORDER BY c DESC LIMIT ?",
                (Value::Integer(limit),),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((text(&row, 0), int(&row, 1)));
        }
        Ok(out)
    }

    /// Live reclaimed/outstanding split for one resolution-ledger key (issue 40):
    /// of the quarantined payloads matching `(reason, profile?, detail LIKE?)`,
    /// how many have been reprocessed back in (`reprocessed_at` set) versus are
    /// still held. `profile`/`detail_like` are optional narrowers — `detail_like`
    /// is a SQL `LIKE` pattern (e.g. `%@REASON`) that pins a sub-bucket within a
    /// reason. The ledger's narrative is source-controlled in the app; this is its
    /// live half, and like the rest of the quarantine metrics it scans the table,
    /// so it belongs on the background refresher, never the request path.
    /// The 2008 per-language duplicate siblings, as ONE self-verifying predicate
    /// (issue 84). Every clause is load-bearing and the last one is the guard:
    ///
    /// - the stale pre-issue-36 bucket (`reason`/`detail`), still held
    ///   (`reprocessed_at IS NULL`), not already marked (`skipped_at IS NULL`);
    /// - from a 2008 TED monthly fetch — the only era that fans one notice out
    ///   across ~23 language files;
    /// - whose member file is **not** the English one (`… .en`);
    /// - **and whose English sibling is present and `parsed`.**
    ///
    /// That last clause makes the operation self-verifying ROW BY ROW rather than
    /// trusting the aggregate: a row whose original is missing is precisely the row
    /// that must NOT be marked, because marking real data loss as a duplicate is
    /// the one outcome worse than an overstated count. Such rows stay outstanding.
    ///
    /// `n.source = 'ted'` is required, not decorative: the only index covering
    /// `publication_id` is `UNIQUE(source, publication_id, content_hash)`, whose
    /// leftmost column is `source`. Unbound, this lookup scans 14.2M notices PER
    /// ROW — the shape that hung issue 84's own falsifier for 90 minutes.
    /// Conditions 1-4: which held rows are even CANDIDATES — the stale 2008 bucket,
    /// still held, not already marked, and not the English member itself. Selecting
    /// a row here is NOT sufficient to mark it; see [`Self::sibling_exists(true)`].
    const SKIPPED_SIBLING_SCOPE: &'static str = "
          q.reason = 'unparsable-xml'
      AND q.detail = 'XML with DTD detected'
      AND q.reprocessed_at IS NULL
      AND q.skipped_at IS NULL
      AND EXISTS (SELECT 1 FROM fetches f
                   WHERE f.id = q.fetch_id
                     AND f.source = 'ted' AND f.kind = 'monthly'
                     AND f.period LIKE '2008%')
      AND lower(replace(q.member_path,
                        rtrim(q.member_path, replace(q.member_path, '.', '')), '')) <> 'en'";

    /// Condition 5, the anti-overreach guard, kept as its own constant so the two
    /// dry-run numbers are COMPOSED from the same text rather than recovered by
    /// slicing it. An earlier draft split the combined predicate on a literal
    /// fragment; a whitespace change there would have silently yielded "no gaps",
    /// which is precisely the reassuring-but-wrong answer this pair exists to
    /// prevent.
    /// The sibling lookup, built from ONE template so the two variants cannot
    /// drift. `require_parsed` is the only difference between them:
    ///
    /// - `true`  — [`Self::sibling_exists(true)`]: the original is present AND
    ///   parsed, i.e. the evidence that earns the duplicate label.
    /// - `false` — the original merely EXISTS, which separates the two findings a
    ///   rejected row can represent (run-driver, 2026-08-05): *no original at all*
    ///   is a fetch/ingest gap; *original present but unparsed* is a parse failure
    ///   on a notice we hold. Both must stay outstanding, so one number is right
    ///   for go/no-go — but they are different investigations, and handing the
    ///   operator an unsplit number costs them the first hour working out which
    ///   population they are looking at.
    ///
    /// Duplicating the publication-id extraction across two constants would be the
    /// divergence risk that splitting SCOPE/GUARD was meant to remove, so it lives
    /// here once.
    fn sibling_exists(require_parsed: bool) -> String {
        let parsed = if require_parsed { "               AND +n.parse_state = 'parsed'\n" } else { "" };
        format!("{}{}{}", SIBLING_HEAD, parsed, SIBLING_TAIL)
    }



    /// Record that a held member was RE-EXAMINED and declined by a dispatch
    /// policy (issue 84) — the permanent half of the fix, which the one-time
    /// backfill exists to catch up.
    ///
    /// A declined member produces no record, so without this its quarantine row is
    /// indistinguishable from one nobody ever looked at: held forever, counted as
    /// outstanding work that no reprocess can move. `skipped_reason` carries WHICH
    /// policy declined it (`internal-ojs-non-english`, …) rather than a generic
    /// mark, so the row says why and not merely that.
    ///
    /// Scoped `AND skipped_at IS NULL` so a re-walk is a no-op, and keyed by
    /// `(fetch_id, member_path)` — the same identity the profile-level reclaim
    /// uses, and unique per the table's own constraint.
    pub async fn flag_skipped_members(
        &self,
        fetch_id: i64,
        members: &[(String, &'static str)],
        now: i64,
    ) -> turso::Result<u64> {
        if members.is_empty() {
            return Ok(0);
        }
        // Grouped by policy so the reason travels with its members without
        // interpolating paths into SQL: every path is a bound parameter. The
        // policy set is a handful of `&'static str` labels, so in practice this is
        // one statement per batch.
        let mut by_policy: std::collections::BTreeMap<&'static str, Vec<&String>> =
            std::collections::BTreeMap::new();
        for (path, policy) in members {
            by_policy.entry(policy).or_default().push(path);
        }
        let conn = self.conn().await;
        let mut flagged = 0u64;
        for (policy, paths) in by_policy {
            let places = vec!["?"; paths.len()].join(",");
            // The 2008 sibling decline carries issue 84's parsed-original guard
            // here too (issue 190): a sibling whose English original is held but
            // UNPARSED is potentially the only readable copy, so the policy alone
            // must not resolve it — the one-time marker refused these rows by
            // construction, and without the same guard this reprocess-time path
            // swept the 154 protected rows into skipped-by-policy. The text-era
            // selection policies stay unguarded: their chosen twin is selected on
            // byte-level equality of the SAME member, not a different document.
            let guard = if policy == "internal-ojs-non-english" {
                format!(" AND {}", Self::sibling_exists(true))
            } else {
                String::new()
            };
            // `reprocessed_at IS NULL` keeps the three outcomes disjoint (issue
            // 288): a row a reclaim already restored is RECLAIMED — the stronger,
            // factual outcome — and a later policy skip of its file must not
            // stamp over it (the bulk backfill's SKIPPED_SIBLING_SCOPE already
            // guards this; the reprocess-time path did not).
            let sql = format!(
                "UPDATE quarantine SET skipped_at = ?, skipped_reason = ?
                  WHERE id IN (SELECT q.id FROM quarantine q
                                WHERE q.fetch_id = ?
                                  AND q.skipped_at IS NULL
                                  AND q.reprocessed_at IS NULL
                                  AND q.member_path IN ({places}){guard})"
            );
            let mut params = vec![
                Value::Integer(now),
                Value::Text(policy.to_owned()),
                Value::Integer(fetch_id),
            ];
            params.extend(paths.into_iter().map(|p| Value::Text(p.clone())));
            conn.execute(&sql, params).await?;
            let mut rows = conn.query("SELECT changes()", ()).await?;
            flagged += rows.next().await?.map_or(0, |row| int(&row, 0) as u64);
        }
        Ok(flagged)
    }

    /// Held 2008 non-English siblings whose English original is **missing or
    /// unparsed** — i.e. everything the scope selects that the guard then declines.
    ///
    /// This is the number that makes a dry-run self-explaining. The marked count
    /// alone says whether the population matches; this says WHY when it does not.
    /// Expected 0 against the verified set — and a non-zero answer is a **data-loss
    /// finding to investigate, never a reason to widen the predicate**: these are
    /// held rows whose original is not in the corpus, so marking them as duplicates
    /// would record real loss as a duplicate, the one outcome worse than an
    /// overstated count.
    pub async fn count_skipped_sibling_gaps(&self) -> turso::Result<(i64, i64)> {
        let conn = self.reader().await?;
        // Two findings, not one: no original at all (a fetch/ingest gap) versus an
        // original we hold that did not parse (a parse failure). Both must stay
        // outstanding — so their SUM is the go/no-go number — but they are
        // different investigations, and the operator reading an abort at 2am should
        // not have to run a second query to learn which they are looking at.
        let sql = format!(
            "SELECT SUM(CASE WHEN NOT {any} THEN 1 ELSE 0 END),
                    SUM(CASE WHEN {any} AND NOT {parsed} THEN 1 ELSE 0 END)
               FROM quarantine q WHERE {scope}",
            any = Self::sibling_exists(false),
            parsed = Self::sibling_exists(true),
            scope = Self::SKIPPED_SIBLING_SCOPE,
        );
        let mut rows = conn.query(&sql, ()).await?;
        Ok(rows.next().await?.map_or((0, 0), |row| {
            (opt_int_of(&row, 0).unwrap_or(0), opt_int_of(&row, 1).unwrap_or(0))
        }))
    }

    /// How many rows the marker WOULD flag, without writing anything (issue 84).
    ///
    /// This is the decision input, not a formality: the run is authorised against
    /// an expected population, and a count that disagrees means the predicate and
    /// the verified set have diverged — so the caller aborts rather than writes.
    pub async fn count_skipped_siblings(&self) -> turso::Result<i64> {
        let conn = self.reader().await?;
        let sql = format!(
            "SELECT COUNT(*) FROM quarantine q WHERE {} AND {}",
            Self::SKIPPED_SIBLING_SCOPE,
            Self::sibling_exists(true)
        );
        let mut rows = conn.query(&sql, ()).await?;
        Ok(rows.next().await?.map_or(0, |row| int(&row, 0)))
    }

    /// Mark one batch of skipped siblings, newest-id-first bounded by `batch`.
    /// Returns how many rows this call marked; 0 means the work list is empty.
    ///
    /// Batched deliberately: turso writes a WAL frame per row and cannot checkpoint
    /// mid-statement, so a single ~593k-row UPDATE is the mechanism that produced a
    /// 127 GB WAL and an OOM during the recovery. The caller checkpoints between
    /// batches. Idempotent — `skipped_at IS NULL` is in the predicate, so a re-run
    /// does the remainder and a crash costs one batch.
    pub async fn mark_skipped_siblings(&self, batch: i64, now: i64, reason: &str) -> turso::Result<i64> {
        let conn = self.conn().await;
        let sql = format!(
            "UPDATE quarantine SET skipped_at = ?, skipped_reason = ?
              WHERE id IN (SELECT q.id FROM quarantine q WHERE {} AND {} LIMIT ?)",
            Self::SKIPPED_SIBLING_SCOPE,
            Self::sibling_exists(true)
        );
        conn.execute(&sql, (Value::Integer(now), reason.to_owned(), Value::Integer(batch))).await?;
        let mut rows = conn.query("SELECT changes()", ()).await?;
        Ok(rows.next().await?.map_or(0, |row| int(&row, 0)))
    }

    /// One batch of the `current_deadline` backfill (issue 216, deadline half):
    /// stamp the next `batch` tenders past the `after` watermark with their head
    /// version's submission deadline, straight from `tender_version_dates`.
    /// Returns `(rows, watermark)`; `rows == 0` means the walk is complete.
    ///
    /// The `MAX` excludes deadlines beyond `DEADLINE_HORIZON_SECS` past the
    /// version's publication, which is what `head_deadline` does in memory.
    ///
    /// **That filter was missing until issue 375**, and its absence was a live
    /// corpus regression rather than an untidiness: `head_deadline` grew the
    /// horizon in `aa732c5` and this walk did not, so running it re-stamped
    /// every row issue 366's drain had corrected — tender 3323836's head
    /// deadline back to 3005-07-06, and back into `status=open`. Worse, the doc
    /// here asserted the two "compute the same thing", so a reader checking
    /// whether a backfill was safe found a promise that it was.
    ///
    /// It is transcribed rather than looked up because it CAN be: one comparison
    /// against one constant, interpolated from `canonical` so the number cannot
    /// drift. Its twin for the value column had no such luck — `sentinel_amount`
    /// is a digit walk — which is why that one was deleted rather than repaired,
    /// leaving the fold as the only thing that elects a head value.
    ///
    /// Batched for the same reason as [`Self::mark_skipped_siblings`]: turso writes
    /// a WAL frame per row and cannot checkpoint mid-statement, so the caller
    /// checkpoints between batches (issue 42). Idempotent — recomputing a stamped
    /// row writes the same value — so a crashed run restarts from zero at worst.
    pub async fn backfill_current_deadline(
        &self,
        batch: i64,
        after: i64,
    ) -> turso::Result<(i64, i64)> {
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT COUNT(*), MAX(id) FROM
                   (SELECT id FROM tenders WHERE id > ? ORDER BY id LIMIT ?)",
                (Value::Integer(after), Value::Integer(batch)),
            )
            .await?;
        let (count, watermark) = match rows.next().await? {
            Some(row) => (int(&row, 0), opt_int_of(&row, 1).unwrap_or(after)),
            None => (0, after),
        };
        drop(rows);
        if count == 0 {
            return Ok((0, after));
        }
        conn.execute(
            &format!(
                "UPDATE tenders SET current_deadline =
                     (SELECT MAX(d.utc_seconds) FROM tender_version_dates d
                       WHERE d.tender_id = tenders.id AND d.seq = tenders.current_seq
                         AND d.field = 'submission_deadline'
                         AND d.utc_seconds - tenders.current_published_at <= {})
                  WHERE id > ? AND id <= ?",
                crate::canonical::DEADLINE_HORIZON_SECS
            ),
            (Value::Integer(after), Value::Integer(watermark)),
        )
        .await?;
        Ok((count, watermark))
    }

    /// One batch of the `current_title` backfill (issue 239): stamp the next `batch`
    /// tenders past the `after` watermark with their head version's title, straight
    /// from `tender_version_texts`.
    ///
    /// The ORDER BY reproduces `head_title`'s precedence — the Tender's own title over a
    /// lot's, `ENG` over another language — and it is the same expression the OLD
    /// `v_tenders` ran per row. That is the point: the cost was never the precedence, it
    /// was paying for it on every read instead of once per fold.
    ///
    /// Batched and idempotent for the same reasons as
    /// [`Self::backfill_current_deadline`]: turso writes a WAL frame per row and cannot
    /// checkpoint mid-statement, so the caller checkpoints between batches (issue 42),
    /// and recomputing a stamped row writes the same value.
    pub async fn backfill_current_title(
        &self,
        batch: i64,
        after: i64,
    ) -> turso::Result<(i64, i64)> {
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT COUNT(*), MAX(id) FROM
                   (SELECT id FROM tenders WHERE id > ? ORDER BY id LIMIT ?)",
                (Value::Integer(after), Value::Integer(batch)),
            )
            .await?;
        let (count, watermark) = match rows.next().await? {
            Some(row) => (int(&row, 0), opt_int_of(&row, 1).unwrap_or(after)),
            None => (0, after),
        };
        drop(rows);
        if count == 0 {
            return Ok((0, after));
        }
        conn.execute(
            "UPDATE tenders SET current_title =
                 (SELECT x.value FROM tender_version_texts x
                   WHERE x.tender_id = tenders.id AND x.seq = tenders.current_seq
                     AND x.field = 'title'
                   ORDER BY (x.lot_id IS NULL) DESC, (x.lang = 'ENG') DESC
                   LIMIT 1)
              WHERE id > ? AND id <= ?",
            (Value::Integer(after), Value::Integer(watermark)),
        )
        .await?;
        Ok((count, watermark))
    }

    // `backfill_current_value_eur` was here, and it is DELETED (issue 375).
    //
    // It stamped `current_value_eur_cents` with a plain `MAX(a.eur_cents)`, which
    // stopped being the head-value election in `aa732c5` — that skips withheld
    // amounts, the EUR 100bn ceiling and `sentinel_amount`'s repdigit field
    // maxima. Its doc claimed the two "can never disagree", so a reader checking
    // whether it was safe to run found a promise that it was.
    //
    // Deleted rather than left refused, because the two reasons for keeping it
    // both expired: `rederive-eur` no longer needs it (it now stamps the affected
    // tenders epoch-stale and lets the FOLD re-elect), and it cannot be repaired
    // in place the way its deadline twin could — `sentinel_amount` is a digit
    // walk, and transcribing that into SQL would be the second implementation
    // this whole class of bug is made of.
    //
    // `current_value_eur_cents` now has exactly ONE writer that decides it: the
    // fold. `refold-notices` aims it at a cohort. A test pins the writer count.

    /// One batch of the currency present-set backfill (issue 371): fold the DISTINCT
    /// `tender_version_amounts.currency` of the next `batch` tenders past the watermark
    /// into `tender_currency_presence`. Windowed on `tender_id` so each batch rides
    /// `tender_version_amounts_version (tender_id, seq)` and costs its own rows, never
    /// the corpus — the [`Self::backfill_current_deadline`] contract, batched and
    /// checkpointed by the caller.
    ///
    /// Returns `(tenders in the window, watermark)`; `0` ends the walk. Idempotent
    /// (`INSERT OR IGNORE` into a primary key), so a crash-restart may redo windows
    /// harmlessly — the set only grows, and growing it is the safe direction.
    ///
    /// This exists for files whose amount rows predate the table. The fold maintains
    /// the set for everything it writes, and a rebuild clears and repopulates it, so
    /// this is the one-time sweep over what neither has touched. It deliberately does
    /// NOT set the coverage flag: only a walk that reached the END of the corpus may
    /// attest coverage, and that is the caller's business
    /// ([`Self::set_currency_presence_complete`]).
    pub async fn backfill_currency_presence(
        &self,
        batch: i64,
        after: i64,
    ) -> turso::Result<(i64, i64)> {
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT COUNT(*), MAX(id) FROM
                   (SELECT id FROM tenders WHERE id > ? ORDER BY id LIMIT ?)",
                (Value::Integer(after), Value::Integer(batch)),
            )
            .await?;
        let (count, watermark) = match rows.next().await? {
            Some(row) => (int(&row, 0), opt_int_of(&row, 1).unwrap_or(after)),
            None => (0, after),
        };
        drop(rows);
        if count == 0 {
            return Ok((0, after));
        }
        conn.execute(
            "INSERT OR IGNORE INTO tender_currency_presence(currency)
             SELECT DISTINCT currency FROM tender_version_amounts
              WHERE tender_id > ? AND tender_id <= ?",
            (Value::Integer(after), Value::Integer(watermark)),
        )
        .await?;
        Ok((count, watermark))
    }

    /// Attest (or withdraw) that `tender_currency_presence` covers the whole standing
    /// corpus — the flag `read::reachable`'s currency leg refuses to guard without.
    /// Set by the `backfill-currencies` job when its walk reaches the end; the layer
    /// wipes and `Db::open`'s empty-layer check set it directly in SQL where they
    /// already hold the connection.
    pub async fn set_currency_presence_complete(&self, complete: bool) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "UPDATE projection_state SET currency_presence_complete = ? WHERE id = 0",
            (Value::Integer(i64::from(complete)),),
        )
        .await?;
        Ok(())
    }

    /// Whether the currency present-set is attested to cover the standing corpus.
    pub async fn currency_presence_complete(&self) -> turso::Result<bool> {
        let conn = self.conn().await;
        let mut rows = conn
            .query("SELECT currency_presence_complete FROM projection_state WHERE id = 0", ())
            .await?;
        Ok(match rows.next().await? {
            Some(row) => int(&row, 0) != 0,
            None => false,
        })
    }

    /// One batch of the `tender_versions.original_lang` backfill (ADR-0013 D3's
    /// third leg, 2026-09-02): the next `batch` tenders past the watermark, every
    /// version of theirs still `NULL`, each resolved to the causing notice's
    /// PROCEDURE-level language code — `BT-702(a)-notice`, `TED-LG_ORIG` or
    /// `TXT-OL`, whichever the era published — and stamped through `normalize`,
    /// the fold's own 639-2/T map, injected as a `fn` because it lives in
    /// `ingest` and `store` cannot depend on it (the same seam the census jobs
    /// use). Every read here is a PK or unique-index seek: versions by
    /// `tender_id`, codes by `(notice_id, section_id, field_id)`, so a batch's cost
    /// is proportional to its rows and never to the corpus. Returns
    /// `(versions stamped, watermark)`; `rows == 0` ends the walk. Idempotent —
    /// stamped rows are skipped — so a crash-restart does only the remainder.
    ///
    /// A version whose notice carries none of the three codes stays `NULL`, which
    /// is the honest value (the 1990s text notices predate the `OL:` line): the
    /// read-time rank treats it as "leg absent", never as a wrong guess.
    pub async fn backfill_original_lang(
        &self,
        batch: i64,
        after: i64,
        normalize: fn(&str) -> Option<String>,
    ) -> turso::Result<(i64, i64)> {
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT COUNT(*), MAX(id) FROM
                   (SELECT id FROM tenders WHERE id > ? ORDER BY id LIMIT ?)",
                (Value::Integer(after), Value::Integer(batch)),
            )
            .await?;
        let (count, watermark) = match rows.next().await? {
            Some(row) => (int(&row, 0), opt_int_of(&row, 1).unwrap_or(after)),
            None => (0, after),
        };
        drop(rows);
        if count == 0 {
            return Ok((0, after));
        }
        // The window's unstamped versions, each with its notice's code if any.
        // The field list is an `IN` on a NON-leading column of the codes PK, so it
        // filters inside the (notice_id, section_id) seek rather than steering the
        // plan — the turso trap is an `IN` on the leading column.
        let mut rows = conn
            .query(
                "SELECT v.tender_id, v.seq,
                        (SELECT c.code FROM notice_codes c
                          WHERE c.notice_id = v.caused_by_notice_id
                            AND c.section_id = 'PROCEDURE'
                            AND c.field_id IN ('BT-702(a)-notice', 'TED-LG_ORIG', 'TXT-OL',
                                               'DE1-NoticeLanguageCode', 'SDK01-NoticeLanguageCode')
                          ORDER BY c.field_id, c.ordinal LIMIT 1)
                   FROM tender_versions v
                  WHERE v.tender_id > ? AND v.tender_id <= ? AND v.original_lang IS NULL",
                (Value::Integer(after), Value::Integer(watermark)),
            )
            .await?;
        let mut stamps: Vec<(i64, i64, String)> = Vec::new();
        while let Some(row) = rows.next().await? {
            if let Some(lang) = opt_text_of(&row, 2).as_deref().and_then(normalize) {
                stamps.push((int(&row, 0), int(&row, 1), lang));
            }
        }
        drop(rows);
        // One transaction per batch. The first cut issued each UPDATE on its own —
        // ~9,200 autocommits (a WAL append and fsync each) per 10,000-tender window —
        // and measured 398 tenders/s on prod with the writer held 20–40 s per window
        // (2026-09-03, job 632: 5.5 h projected for 7.9M tenders). The sibling
        // backfills are one statement per batch for the same reason; this one cannot
        // be (the 639-2/T map is Rust), so the rows share a commit instead.
        if !stamps.is_empty() {
            conn.execute("BEGIN IMMEDIATE", ()).await?;
            for (tender_id, seq, lang) in stamps {
                if let Err(e) = conn
                    .execute(
                        "UPDATE tender_versions SET original_lang = ? WHERE tender_id = ? AND seq = ?",
                        (Value::Text(lang), Value::Integer(tender_id), Value::Integer(seq)),
                    )
                    .await
                {
                    let _ = conn.execute("ROLLBACK", ()).await;
                    return Err(e);
                }
            }
            conn.execute("COMMIT", ()).await?;
        }
        // Like `backfill_current_deadline`: the count is the WINDOW's tenders, so
        // a window whose versions were all stamped already (or all NULL-by-era)
        // still advances the walk rather than ending it.
        Ok((count, watermark))
    }

    /// One batch of the org `name_norm` backfill (issue 217-B): Unicode-lowercase
    /// the next `batch` names past the watermark, in Rust — SQL `lower()` is
    /// ASCII-only and would leave every umlauted name unfindable by the
    /// case-insensitive search the column exists for. Returns `(rows, watermark)`;
    /// `rows == 0` ends the walk. Rows already stamped are skipped, so re-runs and
    /// crash-restarts do only the remainder. The caller checkpoints between
    /// batches (issue 42), exactly like [`Self::backfill_current_deadline`].
    pub async fn backfill_org_name_norm(
        &self,
        batch: i64,
        after: i64,
    ) -> turso::Result<(i64, i64)> {
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT id, name FROM organizations
                  WHERE id > ? AND name_norm IS NULL ORDER BY id LIMIT ?",
                (Value::Integer(after), Value::Integer(batch)),
            )
            .await?;
        let mut pending: Vec<(i64, String)> = Vec::new();
        while let Some(row) = rows.next().await? {
            pending.push((int(&row, 0), text(&row, 1)));
        }
        drop(rows);
        let Some(&(last, _)) = pending.last() else { return Ok((0, after)) };
        let count = pending.len() as i64;
        conn.execute("BEGIN", ()).await?;
        for (id, name) in pending {
            conn.execute(
                "UPDATE organizations SET name_norm = ? WHERE id = ?",
                (Value::Text(name.to_lowercase()), Value::Integer(id)),
            )
            .await?;
        }
        conn.execute("COMMIT", ()).await?;
        Ok((count, last))
    }

    /// The sibling-scope predicate with its skipped-state condition FLIPPED: rows
    /// already marked skipped. Built from the one constant so the repair can never
    /// drift from the marker's own scope; the assert guards against the constant
    /// being reworded in a way that silently makes the flip a no-op.
    fn swept_sibling_scope() -> String {
        let flipped =
            Self::SKIPPED_SIBLING_SCOPE.replace("q.skipped_at IS NULL", "q.skipped_at IS NOT NULL");
        assert_ne!(flipped, Self::SKIPPED_SIBLING_SCOPE, "the skipped-state flip must apply");
        flipped
    }

    /// How many sibling rows are marked skipped although the parsed-original
    /// guard REJECTS them — i.e. rows a guard-free pass swept (issue 190). The
    /// dry-run number for [`Self::repair_swept_siblings`].
    pub async fn count_swept_siblings(&self) -> turso::Result<i64> {
        let conn = self.reader().await?;
        let sql = format!(
            "SELECT COUNT(*) FROM quarantine q WHERE {} AND NOT {}",
            Self::swept_sibling_scope(),
            Self::sibling_exists(true)
        );
        let mut rows = conn.query(&sql, ()).await?;
        Ok(rows.next().await?.map_or(0, |row| int(&row, 0)))
    }

    /// Restore to OUTSTANDING every sibling row that is marked skipped but whose
    /// English original is missing or unparsed (issue 190): the duplicate argument
    /// does not hold for them, so skipped-by-policy overstates what was resolved.
    /// Idempotent, and self-limiting: once the originals parse, the guard accepts
    /// the rows and this predicate matches nothing. Returns rows restored.
    pub async fn repair_swept_siblings(&self) -> turso::Result<i64> {
        let conn = self.conn().await;
        let sql = format!(
            "UPDATE quarantine SET skipped_at = NULL, skipped_reason = NULL
              WHERE id IN (SELECT q.id FROM quarantine q
                            WHERE {} AND NOT {})",
            Self::swept_sibling_scope(),
            Self::sibling_exists(true)
        );
        conn.execute(&sql, ()).await?;
        let mut rows = conn.query("SELECT changes()", ()).await?;
        Ok(rows.next().await?.map_or(0, |row| int(&row, 0)))
    }

    /// Clear every mark this operation made — the reversal, scoped by the marker's
    /// own reason so it can never clear a mark something else wrote. Returns the
    /// number of rows restored to outstanding.
    pub async fn unmark_skipped_siblings(&self, reason: &str) -> turso::Result<i64> {
        let conn = self.conn().await;
        conn.execute(
            "UPDATE quarantine SET skipped_at = NULL, skipped_reason = NULL
              WHERE skipped_reason = ?",
            (reason.to_owned(),),
        )
        .await?;
        let mut rows = conn.query("SELECT changes()", ()).await?;
        Ok(rows.next().await?.map_or(0, |row| int(&row, 0)))
    }

    /// A category's three outcomes: `(reclaimed, skipped, outstanding)`.
    ///
    /// `skipped` is issue 84's third state — re-examined and correctly not
    /// ingested, because the original is already in the corpus. It is reported
    /// separately rather than folded into either neighbour: counting it as
    /// reclaimed would claim notices entered that never did, and counting it as
    /// outstanding is the overstatement this exists to end.
    /// `member_path_like`/`member_path_unlike` (issue 186) narrow by the member's
    /// path shape — the only column that separates populations sharing one
    /// (reason, detail): the 2008 language siblings end in a 2-letter code
    /// (`%.__` minus `%.en`), the English originals in `.en`, the 2010-03
    /// non-siblings in `.xml`. The negative form exists because "2-letter suffix
    /// that is not `.en`" has no single positive LIKE.
    pub async fn quarantine_resolution(
        &self,
        reason: &str,
        profile: Option<&str>,
        detail_like: Option<&str>,
        member_path_like: Option<&str>,
        member_path_unlike: Option<&str>,
    ) -> turso::Result<(i64, i64, i64)> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                // Disjoint arms, reprocessed wins (issue 288) — the same rule
                // `quarantine_counts_by_reason_split` applies, so this card and
                // the dashboard header can never disagree about a row that
                // historically carried both stamps.
                "SELECT SUM(CASE WHEN reprocessed_at IS NOT NULL THEN 1 ELSE 0 END),
                        SUM(CASE WHEN reprocessed_at IS NULL
                                  AND skipped_at     IS NOT NULL THEN 1 ELSE 0 END),
                        SUM(CASE WHEN reprocessed_at IS NULL
                                  AND skipped_at     IS NULL     THEN 1 ELSE 0 END)
                   FROM quarantine
                  WHERE reason = ?
                    AND (? IS NULL OR profile = ?)
                    AND (? IS NULL OR detail LIKE ?)
                    AND (? IS NULL OR member_path LIKE ?)
                    AND (? IS NULL OR member_path NOT LIKE ?)",
                (
                    reason.to_owned(),
                    opt_text(profile),
                    opt_text(profile),
                    opt_text(detail_like),
                    opt_text(detail_like),
                    opt_text(member_path_like),
                    opt_text(member_path_like),
                    opt_text(member_path_unlike),
                    opt_text(member_path_unlike),
                ),
            )
            .await?;
        // SUM over no matching rows is NULL — an untouched category is (0, 0, 0).
        let row = rows.next().await?;
        Ok(row
            .map(|row| {
                (
                    opt_int_of(&row, 0).unwrap_or(0),
                    opt_int_of(&row, 1).unwrap_or(0),
                    opt_int_of(&row, 2).unwrap_or(0),
                )
            })
            .unwrap_or((0, 0, 0)))
    }

    /// The fetch stage per source (issue 33): how many distinct package periods
    /// are on disk and the range they span. Small — one row per source over the
    /// tiny fetch registry.
    pub async fn fetch_registry_summary(&self) -> turso::Result<Vec<(String, i64, String, String)>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT source, COUNT(DISTINCT period), MIN(period), MAX(period)
                   FROM fetches GROUP BY source ORDER BY source",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((text(&row, 0), int(&row, 1), text(&row, 2), text(&row, 3)));
        }
        Ok(out)
    }

    /// Projected Tenders per source (issue 33) — the pipeline's last stage.
    pub async fn tenders_by_source(&self) -> turso::Result<Vec<(String, i64)>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query("SELECT source, COUNT(*) FROM tenders GROUP BY source ORDER BY source", ())
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((text(&row, 0), int(&row, 1)));
        }
        Ok(out)
    }

    /// Notice counts per (mapping profile, publication year) — the dashboard's
    /// coverage grid. The year comes from the package the notice was found in
    /// (periods are `YYYY-NNNNN`, zero-padded and sortable by construction),
    /// not from a parsed date: coverage asks "how much of what TED published
    /// that year do we hold", which is a question about packages.
    pub async fn notice_counts_by_profile_year(&self) -> turso::Result<Vec<ProfileYear>> {
        let conn = self.reader().await?;
        // Single-pass, join-free (issue 20 reopened). The previous form —
        // `notices n JOIN fetches f ON f.id = n.fetch_id GROUP BY f.source, …` —
        // let the planner drive the join from `fetches` and full-scan the 3.5M-row
        // `notices` table once per fetch (no index on notices.fetch_id):
        // O(notices × fetches) ≈ billions of visits, hours per query, one core
        // pinned per unauthenticated `/` hit. Instead, aggregate `notices` by its
        // OWN columns in one scan, then fold the result up against the tiny
        // `fetches` table in process — the planner has no join to get wrong.
        let mut per_fetch = Vec::new();
        let mut rows = conn
            .query("SELECT fetch_id, profile, COUNT(*) FROM notices GROUP BY fetch_id, profile", ())
            .await?;
        while let Some(row) = rows.next().await? {
            per_fetch.push((int(&row, 0), text(&row, 1), int(&row, 2)));
        }

        // One row per downloaded package — thousands, not millions: id → (source, year).
        let mut meta: HashMap<i64, (String, String)> = HashMap::new();
        let mut frows = conn.query("SELECT id, source, substr(period, 1, 4) FROM fetches", ()).await?;
        while let Some(row) = frows.next().await? {
            meta.insert(int(&row, 0), (text(&row, 1), text(&row, 2)));
        }

        // Fold per-(fetch, profile) counts up to (source, profile, year). The
        // BTreeMap key is (year, source, profile), so iteration reproduces the
        // old `ORDER BY year, f.source, n.profile` exactly.
        let mut agg: BTreeMap<(String, String, String), i64> = BTreeMap::new();
        for (fetch_id, profile, count) in per_fetch {
            if let Some((source, year)) = meta.get(&fetch_id) {
                *agg.entry((year.clone(), source.clone(), profile)).or_insert(0) += count;
            }
        }
        Ok(agg
            .into_iter()
            .map(|((year, source, profile), notices)| ProfileYear { source, profile, year, notices })
            .collect())
    }

    /// The newest quarantined payloads — the drill-down behind the headline
    /// count, newest first because a fresh reason is the one worth acting on.
    pub async fn recent_quarantine(&self, limit: i64) -> turso::Result<Vec<QuarantineEntry>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT reason, profile, member_path, detail, first_seen FROM quarantine
                  WHERE reprocessed_at IS NULL ORDER BY id DESC LIMIT ?",
                (Value::Integer(limit),),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(QuarantineEntry {
                reason: text(&row, 0),
                profile: opt_text_of(&row, 1),
                member_path: text(&row, 2),
                detail: opt_text_of(&row, 3),
                first_seen: int(&row, 4),
            });
        }
        Ok(out)
    }

    /// How far behind the source we are, as two independent instants: when we
    /// last downloaded anything, and when we last turned anything into a
    /// Notice. They differ whenever fetching runs ahead of processing, which is
    /// exactly the stall the dashboard needs to make visible (the two stages are
    /// deliberately decoupled — CONTEXT.md).
    pub async fn import_lag(&self) -> turso::Result<ImportLag> {
        let conn = self.reader().await?;
        Ok(ImportLag {
            newest_fetch_at: max_instant(&conn, "SELECT MAX(fetched_at) FROM fetches").await?,
            // The newest notice via the id PK, NOT `MAX(ingested_at)`. There is no
            // index on ingested_at, so `MAX(ingested_at)` is a full table scan of
            // notices (~80 ms at 40k rows → ~15 s at prod's 7.5M) — and this runs
            // ungated every 60 s from the dashboard's `measure_system` while a
            // write-heavy job holds the WAL. A multi-second scan holds a live WAL
            // read snapshot for its duration, which pins the WAL and defeats the
            // per-package TRUNCATE — the store-pool reader behind the 70 GB runaway
            // (issue 42/53). `ingested_at` is assigned at insert time in id order,
            // so it is monotonic with the autoincrement id: the id-newest row's
            // `ingested_at` IS `MAX(ingested_at)`, but this reads exactly one row
            // via the primary key (O(1), microseconds) — no scan, no long snapshot.
            newest_notice_at: max_instant(
                &conn,
                "SELECT ingested_at FROM notices ORDER BY id DESC LIMIT 1",
            )
            .await?,
        })
    }
}

/// `MAX(<timestamp column>)`, `None` when the table is empty.
async fn max_instant(conn: &Connection, sql: &str) -> turso::Result<Option<i64>> {
    let mut rows = conn.query(sql, ()).await?;
    Ok(rows.next().await?.and_then(|row| opt_int_of(&row, 0)))
}

/// One cell of the coverage grid.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileYear {
    pub source: String,
    pub profile: String,
    pub year: String,
    pub notices: i64,
}

/// One quarantined payload, as the dashboard drill-down shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct QuarantineEntry {
    pub reason: String,
    pub profile: Option<String>,
    pub member_path: String,
    pub detail: Option<String>,
    pub first_seen: i64,
}

/// The two ends of the import pipeline, in unix seconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImportLag {
    pub newest_fetch_at: Option<i64>,
    pub newest_notice_at: Option<i64>,
}

/// A current package version in the archive, as the processor addresses it.
#[derive(Clone, Debug, PartialEq)]
pub struct Package {
    pub fetch_id: i64,
    pub period: String,
    /// Archive-relative path, e.g. `ted/daily/2026-00137.tar.gz`.
    pub path: String,
}

/// A Notice identity row. `member_path` is the file inside the package the
/// payload came from (`outer.zip!inner` for nested members, plus `#<n>` for one
/// record of a text-era bundle).
#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub source: String,
    pub publication_id: String,
    pub content_hash: String,
    pub profile: String,
    pub declared_version: Option<String>,
    pub fetch_id: i64,
    pub member_path: String,
    pub ingested_at: i64,
    /// The publication date resolved from the payload at process time (issue
    /// 18), or `None` for an identity-only (unparsed) notice.
    pub published_at: Option<i64>,
    /// The dispatch date resolved from the payload, where the era records one.
    pub dispatched_at: Option<i64>,
}

/// One repeatable-node instance of a notice — see `notice_sections`.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub id: String,
    pub kind: String,
    pub parent: Option<String>,
}

/// A typed value extracted from a notice. The variants are exactly the value
/// tables of the notice-parsed layer.
#[derive(Clone, Debug, PartialEq)]
pub enum NoticeValue {
    Text { lang: Option<String>, value: String },
    Code { list: Option<String>, code: String },
    Classification { scheme: String, code: String },
    Amount { cents: i64, currency: String },
    Date { utc_seconds: i64, offset_minutes: i64, has_time: bool },
    Integer(i64),
    Number { value: f64, unit: Option<String> },
    Id { scheme: Option<String>, value: String, is_ref: bool },
}

/// One value in its place: which section of the notice, which source field.
#[derive(Clone, Debug, PartialEq)]
pub struct ValueRow {
    pub section_id: String,
    pub field_id: String,
    /// Distinguishes repeats of one field within one section (document order).
    pub ordinal: i64,
    pub value: NoticeValue,
}

/// The relational reading of one notice — written atomically with the notice's
/// identity row, so a notice is never half-parsed (ADR-0004).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Parsed {
    pub sections: Vec<Section>,
    pub values: Vec<ValueRow>,
}

/// What the profile parser made of a notice's payload.
#[derive(Clone, Debug, PartialEq)]
pub enum Parse {
    /// No parser for this profile yet — identity only.
    Pending,
    Parsed(Parsed),
    /// Unmapped content or an unrepresentable value: the notice is recorded,
    /// its payload stays in the archive, and nothing of it is imported.
    Quarantined { reason: String, detail: Option<String> },
}

/// The outcome of re-attempting one quarantined member ([`Db::reclaim_notice`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reclaim {
    /// A held member now parses: its parsed layer was written and the quarantine
    /// row flagged `reprocessed_at`, ready for the trailing projection to fold.
    Reclaimed,
    /// The member still does not parse (or is still unrecognised): left held.
    StillHeld,
    /// The notice is already parsed — a prior reclaim, or a member that never
    /// failed. Nothing to do; this makes a re-run a no-op.
    AlreadyParsed,
}

/// A payload that could not be turned into a Notice (ADR-0004).
#[derive(Clone, Debug, PartialEq)]
pub struct Quarantined {
    pub fetch_id: i64,
    pub member_path: String,
    pub content_hash: String,
    pub profile: Option<String>,
    pub reason: String,
    pub detail: Option<String>,
    pub first_seen: i64,
}

/// One downloaded file version in the raw archive.
#[derive(Clone, Debug, PartialEq)]
pub struct Fetch {
    pub source: String,
    pub kind: String,
    pub period: String,
    pub url: String,
    pub sha256: String,
    pub bytes: i64,
    pub fetched_at: i64,
    pub path: String,
}

/// The current wall-clock instant in unix seconds — the one epoch helper the
/// ingestion and server runtimes share (issue 38), replacing five identical
/// `unix_now`/`now_unix` copies across `ingest` and `app`. `0` if the clock is
/// somehow before the epoch (a value the callers only ever store or diff).
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

pub(crate) fn t(s: impl Into<String>) -> Value {
    Value::Text(s.into())
}

pub(crate) fn opt_text(s: Option<&str>) -> Value {
    s.map_or(Value::Null, |s| Value::Text(s.into()))
}

pub(crate) fn opt_int(i: Option<i64>) -> Value {
    i.map_or(Value::Null, Value::Integer)
}

pub(crate) fn text(row: &turso::Row, idx: usize) -> String {
    match row.get_value(idx) {
        Ok(Value::Text(s)) => s,
        _ => String::new(),
    }
}

pub(crate) fn int(row: &turso::Row, idx: usize) -> i64 {
    match row.get_value(idx) {
        Ok(Value::Integer(i)) => i,
        _ => 0,
    }
}

/// The archive member file a quarantine `member_path` names: text-era records
/// carry a `#<ordinal>` suffix (`…ISO_ORG.zip#3`), but the package walker yields
/// the member file, so strip a trailing `#<digits>` to key on it (issue 77).
fn member_file(member_path: String) -> String {
    match member_path.rsplit_once('#') {
        Some((base, ord)) if !ord.is_empty() && ord.bytes().all(|b| b.is_ascii_digit()) => {
            base.to_owned()
        }
        _ => member_path,
    }
}

/// The nested ARCHIVE a member came out of, when its path shows one: the
/// `!`-separated bundle for `outer.zip!inner`, or the `.tar.gz` prefix for
/// `daily.tar.gz/inner`. A whole-container rejection — issue 196: the
/// pre-recursion walker recorded a monthly's 21 inner dailies as raw members —
/// holds ONE row at the container path, which no record- or file-level address
/// can ever reach; the first record reclaimed from inside the container
/// resolves it, exactly as [`member_file`] does for `#<ordinal>` rows.
/// Escape `%`/`_`/`\` for a literal prefix inside a `LIKE … ESCAPE '\'`.
fn like_escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn member_container(member_path: &str) -> Option<String> {
    let file = member_file(member_path.to_owned());
    if let Some((bundle, _)) = file.rsplit_once('!') {
        return Some(bundle.to_owned());
    }
    // The deepest `.tar.gz` prefix at a `/` boundary — a daily may nest
    // subdirectories below the container, so the container is not always the
    // member's immediate dirname.
    let lower = file.to_ascii_lowercase();
    let mut best = None;
    for (i, _) in lower.match_indices('/') {
        if lower[..i].ends_with(".tar.gz") {
            best = Some(i);
        }
    }
    best.map(|i| file[..i].to_owned())
}

pub(crate) fn opt_text_of(row: &turso::Row, idx: usize) -> Option<String> {
    match row.get_value(idx) {
        Ok(Value::Text(s)) => Some(s),
        _ => None,
    }
}

pub(crate) fn opt_int_of(row: &turso::Row, idx: usize) -> Option<i64> {
    match row.get_value(idx) {
        Ok(Value::Integer(i)) => Some(i),
        _ => None,
    }
}

/// The newest cursor in the change log, 0 when it is empty — a high-water mark.
///
/// O(1): reads the AUTOINCREMENT high-water from `sqlite_sequence`, NOT
/// `MAX(cursor)`. turso 0.7 does not lower `MAX()` over an INTEGER PRIMARY KEY to a
/// b-tree extremum seek — it FULL-SCANS `changes`, which at prod (80M rows ≈ 8 GB)
/// is the ~8-minute boot (this runs once in `Db::open`) and the `/health` timeout
/// (issue 61), and it also fired on every `publish_cursor` after a change-append.
/// `changes.cursor` is `INTEGER PRIMARY KEY AUTOINCREMENT` and the table is
/// strictly append-only (never deleted or renumbered — ADR-0001; not in
/// `clear_canonical`/`reset_tender_layer`), so `sqlite_sequence.seq` equals
/// `MAX(cursor)` exactly. Even a future one-time changes-clean would leave the
/// high-water, which is still a safe (≥ any existing cursor) doorbell init. No row
/// exists until the first append, hence the COALESCE to 0.
pub(crate) async fn max_cursor(conn: &Connection) -> turso::Result<i64> {
    let mut rows = conn
        .query("SELECT COALESCE((SELECT seq FROM sqlite_sequence WHERE name = 'changes'), 0)", ())
        .await?;
    Ok(rows.next().await?.map_or(0, |row| int(&row, 0)))
}

#[cfg(test)]
mod tests {
    /// Issue 368: the unmapped-field sweep's window must plan as a RANGE SCAN of
    /// the satellite, and that holds only while its floor is a CONSTANT.
    ///
    /// Asserted as a plan rather than a stopwatch, which is this repo's own rule
    /// (see the batch-apply guarantee below: "a laptop-scale clock cannot tell a
    /// seek from a scan"). I broke that rule twice on this query and paid two
    /// three-hour production runs for it; the answer took 0.4 s once asked here.
    ///
    /// **What the two plans showed.** With a constant floor the driver is the
    /// satellite and the predicate is a range:
    ///
    ///     SEARCH x USING INDEX … (notice_id>?)
    ///     SEARCH n USING INTEGER PRIMARY KEY (rowid=?)
    ///
    /// Join to a per-profile maximum instead and the planner INVERTS it — it
    /// drives from `notices`, all 31 M of them, and seeks the satellite by
    /// EQUALITY once per notice:
    ///
    ///     SCAN notices AS n USING COVERING INDEX notices_profile
    ///     SEARCH x USING INDEX … (notice_id=?)
    ///
    /// The window then bounds nothing at all, which is why the per-profile arm
    /// cost ~60 minutes in both its one-sided and two-sided forms. Reordering the
    /// FROM clause to put the heads first does not help; the planner reorders
    /// anyway. **A per-profile window therefore needs literal per-profile ranges
    /// computed in a prior pass, not a join** — see issue 368.
    #[tokio::test]
    async fn the_unmapped_field_window_plans_as_a_range_scan() {
        let dir = std::env::temp_dir().join(format!("plan-368-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::open(dir.join("t.db").to_str().unwrap()).await.unwrap();
        let conn = db.reader().await.unwrap();
        let mut rows = conn
            .query(
                "EXPLAIN QUERY PLAN \
                 SELECT n.profile, 'notice_texts' AS channel, x.field_id, COUNT(*) \
                   FROM notice_texts x JOIN notices n ON n.id = x.notice_id \
                  WHERE x.notice_id > (SELECT MAX(id) FROM notices) - 1000000 \
                  GROUP BY n.profile, x.field_id",
                (),
            )
            .await
            .unwrap();
        let mut plan = String::new();
        while let Some(row) = rows.next().await.unwrap() {
            if let Ok(turso::Value::Text(d)) = row.get_value(3) {
                plan.push_str(&d);
                plan.push('\n');
            }
        }
        assert!(
            plan.contains("SEARCH x USING INDEX") && plan.contains("notice_id>?"),
            "the satellite must be driven by a RANGE on notice_id:\n{plan}"
        );
        assert!(
            !plan.contains("SCAN notices AS n"),
            "and `notices` must NOT be the outer loop — that inversion is what made the \
             per-profile variant scan 31 M rows per arm:\n{plan}"
        );

        // The probe's shape (`unmapped_fields_for_profile`): CONSTANT bounds on
        // the satellite and the profile as a rowid-seek filter. This is the form
        // that replaced the reverted report arm, and it must plan like the
        // control — satellite driven by a range, notices seeked by rowid — or it
        // is the same 60-minute query wearing a different hat.
        //
        // The CROSS JOIN is what makes it hold. The first draft used a plain JOIN
        // and THIS assertion failed on it: the planner took the profile
        // equality as the driver (`SEARCH n USING INDEX notices_profile
        // (profile=?)`), which is the 2.7 M-row walk again. That is the guard
        // paying for itself on the first change it was written to catch.
        let mut rows = conn
            .query(
                "EXPLAIN QUERY PLAN \
                 SELECT x.field_id, COUNT(*) FROM notice_texts x \
                   CROSS JOIN notices n ON n.id = x.notice_id AND n.profile = 'ted-export-r208' \
                  WHERE x.notice_id > 27061439 AND x.notice_id <= 27161439 \
                  GROUP BY x.field_id",
                (),
            )
            .await
            .unwrap();
        let mut plan = String::new();
        while let Some(row) = rows.next().await.unwrap() {
            if let Ok(turso::Value::Text(d)) = row.get_value(3) {
                plan.push_str(&d);
                plan.push('\n');
            }
        }
        assert!(
            plan.contains("SEARCH x USING INDEX") && plan.contains("notice_id>?"),
            "the probe's satellite scan must be a bounded RANGE:\n{plan}"
        );
        assert!(
            plan.contains("SEARCH n USING INTEGER PRIMARY KEY"),
            "and the profile filter must be a rowid SEEK into notices, not a scan:\n{plan}"
        );
        assert!(!plan.contains("SCAN notices AS n"), "{plan}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Issue 175: the page-cache valve parses positive KiB, clamps the absurd at
    /// both ends (a 100 KiB cache thrashes, an unbounded one re-creates the
    /// issue-61 swap incident as a typo), and falls back to the 128 MiB default
    /// on anything unparseable — a bad valve value must degrade to the safe
    /// default, never fail an open.
    #[test]
    fn the_cache_valve_parses_clamps_and_defaults() {
        use super::{cache_kib, CACHE_KIB_DEFAULT};
        assert_eq!(cache_kib(None), CACHE_KIB_DEFAULT);
        assert_eq!(cache_kib(Some("garbage")), CACHE_KIB_DEFAULT);
        assert_eq!(cache_kib(Some("-524288")), CACHE_KIB_DEFAULT, "negative is unparseable as u64");
        assert_eq!(cache_kib(Some("")), CACHE_KIB_DEFAULT);
        assert_eq!(cache_kib(Some("524288")), 524_288, "512 MiB — the 64 GB prod setting");
        assert_eq!(cache_kib(Some(" 524288 ")), 524_288, "whitespace tolerated");
        assert_eq!(cache_kib(Some("100")), 1_024, "clamped up to 1 MiB");
        assert_eq!(cache_kib(Some("99999999999")), 4_194_304, "clamped down to 4 GiB");
    }

    /// Issue 83: the spill-directory warning must resolve the mount governing a path
    /// by LONGEST matching mount point, and must distinguish the real hazard (a
    /// database on disk spilling to RAM) from the coherent case (a small scratch
    /// database that lives on the same tmpfs as its spill). Without the second half
    /// this fires on every `Db::open` in the suite on any host whose `/tmp` is
    /// tmpfs, and a warning that cries wolf in CI is one nobody reads in production.
    #[test]
    fn spill_mount_resolution_finds_the_longest_prefix() {
        use super::mount_of;
        const MOUNTS: &str = "\
/dev/root / ext4 rw 0 0
tmpfs /tmp tmpfs rw 0 0
/dev/sdb /data ext4 rw 0 0
tmpfs /data/ramcache tmpfs rw 0 0
";
        // Longest prefix wins over `/`, and over a shorter real mount.
        assert_eq!(mount_of(MOUNTS, "/tmp/sort.tmp"), Some(("/tmp", "tmpfs")));
        assert_eq!(mount_of(MOUNTS, "/data/db/tender-db.db"), Some(("/data", "ext4")));
        assert_eq!(mount_of(MOUNTS, "/data/ramcache/x"), Some(("/data/ramcache", "tmpfs")));
        assert_eq!(mount_of(MOUNTS, "/home/someone/db"), Some(("/", "ext4")));
        // An exact mount point resolves to itself, not to its parent.
        assert_eq!(mount_of(MOUNTS, "/data"), Some(("/data", "ext4")));

        // The hazard: spill in RAM, database on disk — different mount points.
        let spill = mount_of(MOUNTS, "/tmp").expect("spill mount");
        let prod_db = mount_of(MOUNTS, "/data/db/tender-db.db").expect("db mount");
        assert_eq!(spill.1, "tmpfs");
        assert_ne!(spill.0, prod_db.0, "prod shape must be reported as a mismatch");

        // The coherent case: a scratch database on the same tmpfs as the spill.
        let scratch_db = mount_of(MOUNTS, "/tmp/scratch-1234.db").expect("scratch mount");
        assert_eq!(spill.0, scratch_db.0, "a tmpfs-resident scratch DB must stay silent");
    }

    use super::*;

    /// [`Db::reset_tender_layer`] must leave every tender-side AUTOINCREMENT table's
    /// sqlite_sequence high-water cleared, so a from-scratch fold (fresh OR resume)
    /// re-inserts ids from 1 in fold order — the invariant that makes a resumed
    /// rebuild byte-identical to a fresh one, and the projection's surrogate ids
    /// deterministic. Proves the turso behavior the projection depends on
    /// (DROP TABLE clears the sequence) AND the explicit sqlite_sequence DELETE, so
    /// the cutover has no unproven unknown.
    #[tokio::test]
    async fn reset_tender_layer_restarts_autoincrement_at_one() {
        let path = format!("/tmp/tender-db-resettender-{}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        let db = Db::open(&path).await.unwrap();
        // reset_tender_layer runs FK-off in prod (the projection disables FK).
        db.set_foreign_keys(false).await.unwrap();

        // Push every tender-side sequence high-water above 1 (two rows each), so a
        // reset that did NOT clear the sequence would hand out ids 3+, not 1.
        {
            let conn = db.conn().await;
            for i in 1..=2i64 {
                conn.execute(
                    "INSERT INTO tenders(source, procedure_key, island_notice_id, kind, created_at)
                     VALUES('ted', ?, NULL, 'procedure', 0)",
                    (t(&format!("bt04-{i}")),),
                )
                .await
                .unwrap();
                conn.execute("INSERT INTO lots(tender_id, lot_key) VALUES(?, ?)", (Value::Integer(i), t(&format!("L{i}")))).await.unwrap();
                conn.execute("INSERT INTO bids(tender_id, notice_id, bid_key) VALUES(?, 1, ?)", (Value::Integer(i), t(&format!("TEN-{i}")))).await.unwrap();
                conn.execute("INSERT INTO contracts(tender_id, notice_id, contract_key) VALUES(?, 1, ?)", (Value::Integer(i), t(&format!("CON-{i}")))).await.unwrap();
                conn.execute("INSERT INTO lot_results(tender_id, notice_id, result_key) VALUES(?, 1, ?)", (Value::Integer(i), t(&format!("RES-{i}")))).await.unwrap();
            }
        }

        db.reset_tender_layer().await.unwrap();

        // Every fresh insert restarts at id 1.
        let conn = db.conn().await;
        conn.execute(
            "INSERT INTO tenders(source, procedure_key, island_notice_id, kind, created_at)
             VALUES('ted', 'bt04-fresh', NULL, 'procedure', 0)",
            (),
        )
        .await
        .unwrap();
        assert_eq!(conn.last_insert_rowid(), 1, "tenders id restarts at 1 after reset_tender_layer");
        conn.execute("INSERT INTO lots(tender_id, lot_key) VALUES(1, 'x')", ()).await.unwrap();
        assert_eq!(conn.last_insert_rowid(), 1, "lots id restarts at 1 after sqlite_sequence reset");
        conn.execute("INSERT INTO bids(tender_id, notice_id, bid_key) VALUES(1, 1, 'x')", ()).await.unwrap();
        assert_eq!(conn.last_insert_rowid(), 1, "bids id restarts at 1");
        conn.execute("INSERT INTO contracts(tender_id, notice_id, contract_key) VALUES(1, 1, 'x')", ()).await.unwrap();
        assert_eq!(conn.last_insert_rowid(), 1, "contracts id restarts at 1");
        conn.execute("INSERT INTO lot_results(tender_id, notice_id, result_key) VALUES(1, 1, 'x')", ()).await.unwrap();
        assert_eq!(conn.last_insert_rowid(), 1, "lot_results id restarts at 1");
        drop(conn);

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// Regression (issue 42/53, the store-pool WAL pin): `import_lag`'s newest-notice
    /// read must be O(1), not a full `notices` scan. `MAX(ingested_at)` full-scanned
    /// the table (no index on ingested_at) — running ungated every 60 s from the
    /// dashboard's `measure_system` during a write-heavy job, it held a live WAL read
    /// snapshot for its multi-second duration and pinned the WAL (70 GB in the field).
    /// The fix reads the newest notice via the id PK. This asserts BOTH correctness
    /// (id-newest == max ingested_at, which holds because ingested_at is monotonic
    /// with the autoincrement id) AND that `import_lag` is dramatically cheaper than
    /// the scan it replaced — self-calibrating against the same machine, so it is not
    /// a brittle absolute-time threshold.
    #[tokio::test]
    async fn import_lag_reads_the_newest_notice_in_o1_not_a_full_scan() {
        let path = format!("/tmp/tender-db-importlag-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        db.record_fetch(&Fetch {
            source: "ted".into(),
            kind: "daily".into(),
            period: "2026-00001".into(),
            url: "u".into(),
            sha256: "a".into(),
            bytes: 1,
            fetched_at: 7,
            path: "p".into(),
        })
        .await
        .unwrap();

        // Enough rows that a full scan is clearly measurable; ingested_at monotonic
        // with the insert order (the production invariant — it is set to now_unix()
        // per package). One transaction for speed.
        const N: i64 = 40_000;
        {
            let conn = db.conn().await;
            conn.execute("BEGIN IMMEDIATE", ()).await.unwrap();
            for i in 0..N {
                conn.execute(
                    "INSERT INTO notices(source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at)
                     VALUES('ted', ?, ?, 'eforms', 1, 'm', ?)",
                    (t(&format!("p{i}")), t(&format!("{i:064}")), Value::Integer(i)),
                )
                .await
                .unwrap();
            }
            conn.execute("COMMIT", ()).await.unwrap();
        }

        // Correctness: the newest notice's instant is the max ingested_at.
        let lag = db.import_lag().await.unwrap();
        assert_eq!(lag.newest_notice_at, Some(N - 1), "newest notice == max ingested_at");
        assert_eq!(lag.newest_fetch_at, Some(7));

        // Performance: import_lag must NOT scan. Compare it, on THIS machine, to the
        // full `MAX(ingested_at)` scan it replaced — the fix must be at least 10x
        // cheaper (in practice ~250x). Self-calibrating, so a slow CI box scales both.
        let time = |sql: &'static str| {
            let db = &db;
            async move {
                let conn = db.reader().await.unwrap();
                let t = std::time::Instant::now();
                for _ in 0..10 {
                    let mut rows = conn.query(sql, ()).await.unwrap();
                    while rows.next().await.unwrap().is_some() {}
                }
                t.elapsed()
            }
        };
        let scan = time("SELECT MAX(ingested_at) FROM notices").await;
        let fixed = time("SELECT ingested_at FROM notices ORDER BY id DESC LIMIT 1").await;
        assert!(
            fixed * 10 < scan,
            "import_lag's newest-notice read must be O(1), not the O(n) scan: fixed={fixed:?} scan={scan:?}"
        );

        let _ = std::fs::remove_file(&path);
    }

    // Exercises the pragmas and the full STRICT schema — both layers — against a
    // real Turso db file.
    #[tokio::test]
    async fn opens_and_applies_the_schema() {
        let path = format!("/tmp/tender-db-test-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path).await.unwrap();
        assert!(db.list_tenders(10).await.unwrap().is_empty());
        // Re-opening applies the schema again; every statement is IF NOT EXISTS.
        drop(db);
        let db = Db::open(&path).await.unwrap();
        assert!(db.canonical_counts().await.unwrap().iter().all(|(_, n)| *n == 0));

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 61 root cause: `max_cursor` must be O(1) (sqlite_sequence high-water),
    /// NOT a `MAX(cursor)` full scan of the 80M-row `changes` table. Proves the O(1)
    /// form equals the old MAX after appends and survives a reopen, and that it reads
    /// 0 on an empty log (no sqlite_sequence row yet).
    #[tokio::test]
    async fn max_cursor_is_o1_and_matches_the_scan() {
        let path = format!("/tmp/tender-db-maxcursor-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path).await.unwrap();
        {
            let conn = db.conn().await;
            // Empty log: no sqlite_sequence row for `changes` yet → 0.
            assert_eq!(max_cursor(&conn).await.unwrap(), 0, "empty change log reads 0");
            for i in 0..5 {
                conn.execute(
                    &format!(
                        "INSERT INTO changes(entity_kind, entity_id, version_seq, op, changed_at) \
                         VALUES ('tender', {i}, 1, 'added', 0)"
                    ),
                    (),
                )
                .await
                .unwrap();
            }
            // The O(1) form must equal the authoritative MAX(cursor) scan.
            let scan = {
                let mut r = conn.query("SELECT COALESCE(MAX(cursor), 0) FROM changes", ()).await.unwrap();
                int(&r.next().await.unwrap().unwrap(), 0)
            };
            assert_eq!(scan, 5, "5 AUTOINCREMENT appends → MAX(cursor) = 5");
            assert_eq!(max_cursor(&conn).await.unwrap(), scan, "O(1) max_cursor == MAX(cursor) scan");
        }

        // Survives reopen (sqlite_sequence is durable; this is exactly the Db::open
        // init path that stalled for 8 minutes at prod).
        drop(db);
        let db = Db::open(&path).await.unwrap();
        let conn = db.conn().await;
        assert_eq!(max_cursor(&conn).await.unwrap(), 5, "high-water survives reopen");

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 61 finding: `oldest_cursor` (SSE resume path) must be O(1). turso
    /// short-circuits neither MIN(cursor) nor ORDER BY cursor LIMIT 1 — both
    /// full-scan the 80M-row changes table (verified) — so it derives from the
    /// append-only invariant: 1 when the log is non-empty (via the O(1)
    /// sqlite_sequence high-water, whose O(1)-ness `max_cursor_is_o1` already
    /// proves), else 0. Here we prove it returns the CORRECT value — equal to the
    /// authoritative MIN(cursor) scan — on both empty and non-empty logs.
    #[tokio::test]
    async fn oldest_cursor_is_o1_for_the_append_only_log() {
        let path = format!("/tmp/tender-db-oldest-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let conn = db.conn().await;

        // Empty log → 0, matching MIN(cursor)'s COALESCE.
        assert_eq!(crate::read::oldest_cursor(&conn).await.unwrap(), 0, "empty log → 0");

        for i in 0..5 {
            conn.execute(
                &format!(
                    "INSERT INTO changes(entity_kind, entity_id, version_seq, op, changed_at) \
                     VALUES ('tender', {i}, 1, 'added', 0)"
                ),
                (),
            )
            .await
            .unwrap();
        }

        // The authoritative MIN scan says 1 (AUTOINCREMENT from 1, never trimmed);
        // the O(1) form must agree.
        let min_scan = {
            let mut r = conn.query("SELECT COALESCE(MIN(cursor), 0) FROM changes", ()).await.unwrap();
            int(&r.next().await.unwrap().unwrap(), 0)
        };
        assert_eq!(min_scan, 1, "the append-only log's true oldest cursor is 1");
        assert_eq!(
            crate::read::oldest_cursor(&conn).await.unwrap(),
            min_scan,
            "O(1) oldest_cursor == the MIN(cursor) scan"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 61 finding: `changes_since` with an entity filter uses the
    /// (entity_kind, cursor) index shape and short-circuits an UNKNOWN kind to empty
    /// without a table walk (the `/v1/changes?entity=x&since=0` wedge). Correctness:
    /// filters to the kind, returns all with no filter, empty for a nonexistent kind.
    #[tokio::test]
    async fn changes_since_filters_by_kind_and_guards_unknown() {
        let path = format!("/tmp/tender-db-changessince-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let conn = db.conn().await;
        for (kind, id) in [("tender", 100), ("organization", 200), ("tender", 101)] {
            conn.execute(
                &format!(
                    "INSERT INTO changes(entity_kind, entity_id, version_seq, op, changed_at) \
                     VALUES ('{kind}', {id}, 1, 'added', 0)"
                ),
                (),
            )
            .await
            .unwrap();
        }
        let kinds = |rows: &[crate::Change]| rows.iter().map(|c| c.entity_kind.clone()).collect::<Vec<_>>();

        let all = crate::read::changes_since(&conn, 0, 100, None).await.unwrap();
        assert_eq!(all.len(), 3, "no filter returns every change");

        let tenders = crate::read::changes_since(&conn, 0, 100, Some("tender")).await.unwrap();
        assert_eq!(kinds(&tenders), vec!["tender", "tender"], "filters to the tender rows in cursor order");

        let orgs = crate::read::changes_since(&conn, 0, 100, Some("organization")).await.unwrap();
        assert_eq!(orgs.len(), 1, "filters to the single organization row");

        let bogus = crate::read::changes_since(&conn, 0, 100, Some("nonexistent_kind")).await.unwrap();
        assert!(bogus.is_empty(), "an unknown entity_kind short-circuits to empty (no table walk)");

        let _ = std::fs::remove_file(&path);
    }

    /// Salvage-loop fix: the resume signal is the durable `rebuild_in_progress`
    /// flag, decoupled from "a plan is on disk". A fresh DB is not rebuilding; a
    /// rebuild sets it; `clear_plan` (clean completion) clears it — so a finished
    /// build's leftover complete plan can never re-trigger a layer-nuking resume.
    #[tokio::test]
    async fn rebuild_in_progress_flag_lifecycle() {
        let path = format!("/tmp/tender-db-rebuildflag-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path).await.unwrap();
        assert!(!db.rebuild_in_progress().await.unwrap(), "a fresh DB is not mid-rebuild");

        db.set_rebuild_in_progress().await.unwrap();
        assert!(db.rebuild_in_progress().await.unwrap(), "set marks the rebuild in-flight");

        // reset_plan (start of a run, via clear_plan_on) must NOT touch the flag —
        // a fresh rebuild sets the flag and then builds its plan.
        db.reset_plan().await.unwrap();
        assert!(db.rebuild_in_progress().await.unwrap(), "reset_plan leaves the flag set");

        // clear_plan is the clean-completion path; it must also clear the flag so a
        // restart does not re-salvage a fully-built layer.
        db.clear_plan().await.unwrap();
        assert!(!db.rebuild_in_progress().await.unwrap(), "clear_plan retires the flag");

        // Survives a reopen (durable, not in-memory) and defaults off.
        drop(db);
        let db = Db::open(&path).await.unwrap();
        assert!(!db.rebuild_in_progress().await.unwrap(), "flag is durable and defaults off");

        let _ = std::fs::remove_file(&path);
    }

    /// A database created before issue 18 lacks the published_at/dispatched_at
    /// columns; opening it must migrate rather than fail on the first write —
    /// the production incident of 2026-07-21.
    #[tokio::test]
    async fn opening_a_pre_issue18_database_adds_the_missing_columns() {
        let path = format!("/tmp/tender-db-migrate-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);

        // Simulate the old schema: same table names, without the new columns.
        let database = turso::Builder::new_local(&path).build().await.unwrap();
        let conn = database.connect().unwrap();
        conn.execute_batch(
            "CREATE TABLE notices (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL, publication_id TEXT NOT NULL,
                content_hash TEXT NOT NULL, profile TEXT NOT NULL,
                declared_version TEXT, fetch_id INTEGER NOT NULL,
                member_path TEXT NOT NULL, ingested_at INTEGER NOT NULL,
                parse_state TEXT NOT NULL DEFAULT 'pending',
                UNIQUE(source, publication_id, content_hash)
            ) STRICT;
             CREATE TABLE tender_versions (
                tender_id INTEGER NOT NULL, seq INTEGER NOT NULL,
                caused_by_notice_id INTEGER NOT NULL, published_at INTEGER,
                PRIMARY KEY (tender_id, seq)
            ) STRICT;",
        )
        .await
        .unwrap();
        drop(conn);
        drop(database);

        let db = Db::open(&path).await.expect("open must migrate the old schema");
        let conn = db.conn().await;
        conn.execute(
            "INSERT INTO notices(source, publication_id, content_hash, profile,
                                 fetch_id, member_path, ingested_at, published_at, dispatched_at)
             VALUES('ted', 'p', 'h', 'eforms', 1, 'm', 0, 1, 2)",
            (),
        )
        .await
        .expect("the migrated columns must be writable");

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn fetch_registry_round_trips() {
        let path = format!("/tmp/tender-db-fetchtest-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();

        assert!(db.latest_fetch("ted", "daily", "2026-00137").await.unwrap().is_none());

        let first = Fetch {
            source: "ted".into(),
            kind: "daily".into(),
            period: "2026-00137".into(),
            url: "https://ted.europa.eu/packages/daily/202600137".into(),
            sha256: "aa".into(),
            bytes: 10,
            fetched_at: 1,
            path: "ted/daily/2026-00137.tar.gz".into(),
        };
        db.record_fetch(&first).await.unwrap();
        assert_eq!(db.latest_fetch("ted", "daily", "2026-00137").await.unwrap(), Some(first.clone()));

        // A re-fetch with different content becomes the new current version.
        let second = Fetch { sha256: "bb".into(), fetched_at: 2, ..first };
        db.record_fetch(&second).await.unwrap();
        assert_eq!(db.latest_fetch("ted", "daily", "2026-00137").await.unwrap(), Some(second));

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 20 regression on the user-visible path: the dashboard root `/`
    /// (`coverage::measure` plus `list_tenders`) must not queue behind an
    /// ingestion job holding the writer. These read-only accessors go through
    /// `reader()` (the WAL pool), so they return while an open write transaction
    /// is in flight; routed through the writer mutex (the old code) they would
    /// deadlock against the guard held below — the `/` timeout in production.
    #[tokio::test]
    async fn dashboard_reads_do_not_block_on_a_held_writer() {
        let path = format!("/tmp/tender-db-dash-busy-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();

        // Hold the writer in a live transaction — what a heavy process/project
        // batch does for the length of its commit.
        let writer = db.conn().await;
        writer.execute("BEGIN IMMEDIATE", ()).await.unwrap();

        let started = std::time::Instant::now();
        // The heaviest of the seven reads `/` issues, plus the tender list.
        db.notice_counts_by_profile_year().await.unwrap();
        db.quarantine_counts_by_reason().await.unwrap();
        db.recent_quarantine(20).await.unwrap();
        db.list_tenders(200).await.unwrap();
        assert!(started.elapsed().as_secs() < 1, "dashboard reads must not queue behind the writer");

        writer.execute("COMMIT", ()).await.unwrap();
        let _ = std::fs::remove_file(&path);
    }

    /// Issue 241: a writer held while callers queue behind it is the shape of
    /// issue 240's 25-minute outage, and nothing in the process could see it. Now
    /// the queue itself is measured.
    ///
    /// The test holds the writer, parks three callers behind it, and checks that
    /// the depth gauge reads three WHILE they wait — a gauge that only moved after
    /// the fact would be useless for the alert it exists to raise.
    #[tokio::test]
    async fn a_held_writer_with_callers_behind_it_is_visible_while_it_happens() {
        let path = format!("/tmp/tender-db-writer-wait-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Arc::new(Db::open(&path).await.unwrap());

        // An uncontended acquisition counts but never waits: the fast path must
        // not invent contention out of ordinary writes.
        {
            let _ = db.conn().await;
        }
        let quiet = db.writer_stats();
        assert!(quiet.acquisitions > 0, "acquisitions are counted");
        assert_eq!(quiet.depth, 0, "nobody is waiting");
        assert_eq!(quiet.waited_seconds, 0.0, "an uncontended acquire waits for nothing");

        // Hold it, the way a fold holds it for its whole transaction.
        let held = db.conn().await;

        let mut waiters = Vec::new();
        for _ in 0..3 {
            let db = Arc::clone(&db);
            waiters.push(tokio::spawn(async move {
                let _guard = db.conn().await;
            }));
        }
        // Let them reach the lock. Depth is observed WHILE the writer is held —
        // this is the assertion that matters.
        let mut depth = 0;
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            depth = db.writer_stats().depth;
            if depth == 3 {
                break;
            }
        }
        assert_eq!(depth, 3, "three callers are queued behind the held writer");

        drop(held);
        for w in waiters {
            w.await.unwrap();
        }

        let after = db.writer_stats();
        assert_eq!(after.depth, 0, "the queue drains");
        assert!(after.waited_seconds > 0.0, "the wait is accumulated: {after:?}");
        assert!(
            after.longest_wait_seconds > 0.0 && after.longest_wait_seconds <= after.waited_seconds,
            "the high-water mark is one wait, not the sum: {after:?}"
        );
        assert!(after.acquisitions >= 5, "one uncontended + one held + three queued: {after:?}");

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 241, the gauge's one leak: a waiter that gives up (a `timeout` around
    /// `conn`, as issue 256's queue-persist does behind a fold) must not leave a
    /// ghost in `queue_depth`. Before the drop guard this read 1 forever — on prod,
    /// 2 after two give-ups, on an idle box.
    #[tokio::test]
    async fn a_waiter_that_gives_up_leaves_no_ghost_in_the_queue_depth() {
        let path = format!("/tmp/tender-db-writer-giveup-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Arc::new(Db::open(&path).await.unwrap());

        let held = db.conn().await;
        let waiter = {
            let db = Arc::clone(&db);
            tokio::spawn(async move {
                tokio::time::timeout(std::time::Duration::from_millis(50), async {
                    let _guard = db.conn().await;
                })
                .await
            })
        };
        let gave_up = waiter.await.unwrap();
        assert!(gave_up.is_err(), "the waiter timed out behind the held writer");
        assert_eq!(
            db.writer_stats().depth,
            0,
            "an abandoned wait is un-counted the moment it is dropped, while the writer is still held"
        );
        drop(held);

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 20 (reopened): the coverage query must aggregate `notices` in a
    /// single scan, not a `notices × fetches` nested loop. Builds a dataset with
    /// MANY fetches (which is what made the old join superlinear) and asserts the
    /// counts are correct and the call stays well under a wall-clock bound a
    /// quadratic plan over this size would blow past.
    #[tokio::test]
    async fn coverage_query_is_single_pass_over_notices() {
        let path = format!("/tmp/tender-db-coverage-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();

        // Many fetches amplify a nested loop's fetches×notices term; the row total
        // is kept modest because turso's debug-build insert path is what bounds
        // this test's runtime, not the query. 600 fetches × 25 notices = 15k. The
        // definitive perf check is in prod (the issue's acceptance), on 3.5M rows.
        const FETCHES: i64 = 600;
        const PER_FETCH: i64 = 25;
        {
            let conn = db.conn().await;
            conn.execute("BEGIN IMMEDIATE", ()).await.unwrap();
            for f in 0..FETCHES {
                // ted↔eforms, doe↔text; years cycle 2024/2025/2026 via f % 3.
                let source = if f % 2 == 0 { "ted" } else { "doe" };
                let profile = if f % 2 == 0 { "eforms" } else { "text" };
                let year = 2024 + (f % 3);
                let id = f + 1;
                conn.execute(
                    &format!(
                        "INSERT INTO fetches(id, source, kind, period, url, sha256, bytes, fetched_at, path)
                         VALUES({id}, '{source}', 'daily', '{year}-{f:05}', 'u', 'h', 1, 0, 'p')"
                    ),
                    (),
                )
                .await
                .unwrap();
                let mut sql = String::from(
                    "INSERT INTO notices(source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at) VALUES ",
                );
                for n in 0..PER_FETCH {
                    if n > 0 {
                        sql.push(',');
                    }
                    sql.push_str(&format!("('{source}','p{f}_{n}','h{f}_{n}','{profile}',{id},'m',0)"));
                }
                conn.execute(&sql, ()).await.unwrap();
            }
            conn.execute("COMMIT", ()).await.unwrap();
        }

        let started = std::time::Instant::now();
        let rows = db.notice_counts_by_profile_year().await.unwrap();
        let elapsed = started.elapsed();

        // Every notice is counted exactly once.
        assert_eq!(rows.iter().map(|r| r.notices).sum::<i64>(), FETCHES * PER_FETCH);
        // ted holds only eforms, doe only text — the join folded profile correctly.
        assert!(rows.iter().all(|r| (r.source == "ted") == (r.profile == "eforms")));
        // Exactly the (year, source) × its one profile cells: 3 years × 2 sources.
        assert_eq!(rows.len(), 6);
        // Ordered by (year, source, profile), reproducing the old ORDER BY.
        let keys: Vec<_> = rows.iter().map(|r| (r.year.clone(), r.source.clone(), r.profile.clone())).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);

        // The single scan is milliseconds; a reverted fetches×notices nested loop
        // (600 × 15k = 9e6 visits in a debug build) is seconds. Generous bound so
        // slow CI stays green while a gross quadratic regression still trips it.
        assert!(elapsed.as_secs() < 3, "coverage query looks superlinear (took {elapsed:?})");

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 25: opening a pre-issue-25 canonical layer adds the head-pointer
    /// columns and backfills them once from `tender_versions` — the deploy path,
    /// since the prod DB predates the pointer.
    #[tokio::test]
    async fn migration_backfills_the_current_version_pointer() {
        let path = format!("/tmp/tender-db-curptr-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);

        // A pre-issue-25 schema: tenders + a two-version chain, no pointer columns.
        let database = turso::Builder::new_local(&path).build().await.unwrap();
        let conn = database.connect().unwrap();
        conn.execute_batch(
            "CREATE TABLE tenders (
                id INTEGER PRIMARY KEY AUTOINCREMENT, source TEXT NOT NULL,
                procedure_key TEXT, island_notice_id INTEGER, kind TEXT NOT NULL,
                created_at INTEGER NOT NULL, UNIQUE(procedure_key), UNIQUE(island_notice_id)
            ) STRICT;
             CREATE TABLE tender_versions (
                tender_id INTEGER NOT NULL, seq INTEGER NOT NULL,
                caused_by_notice_id INTEGER NOT NULL, published_at INTEGER NOT NULL,
                publication_id TEXT NOT NULL, PRIMARY KEY (tender_id, seq)
            ) STRICT;
             INSERT INTO tenders(id, source, kind, created_at) VALUES(1, 'ted', 'procedure', 0);
             INSERT INTO tender_versions(tender_id, seq, caused_by_notice_id, published_at, publication_id)
                VALUES(1, 1, 10, 100, 'a'), (1, 2, 11, 200, 'b');",
        )
        .await
        .unwrap();
        drop(conn);
        drop(database);

        let db = Db::open(&path).await.expect("open migrates and backfills the pointer");
        let conn = db.reader().await.unwrap();
        let mut rows = conn
            .query("SELECT current_seq, current_published_at FROM tenders WHERE id = 1", ())
            .await
            .unwrap();
        let row = rows.next().await.unwrap().expect("the tender row");
        assert_eq!(int(&row, 0), 2, "current_seq backfilled to MAX(seq)");
        assert_eq!(int(&row, 1), 200, "current_published_at backfilled to the head version's date");

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 25: `list_tenders` is ordered by the maintained head date (newest
    /// first), honours the limit, and falls back to '(untitled)'. Rows are
    /// inserted with the pointer set, as the projection would leave them.
    #[tokio::test]
    async fn list_tenders_orders_by_the_current_head() {
        let path = format!("/tmp/tender-db-listorder-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        // Insert canonical rows directly without the full notice/fetch graph, as
        // the projection does behind its own FK-off window (issue 19).
        db.set_foreign_keys(false).await.unwrap();
        {
            let conn = db.conn().await;
            // Three tenders whose head publication dates are 300 / 100 / 200.
            conn.execute_batch(
                "INSERT INTO tenders(id, source, kind, created_at, current_seq, current_published_at)
                   VALUES (1,'ted','procedure',0,1,300),
                          (2,'ted','procedure',0,1,100),
                          (3,'ted','procedure',0,1,200);
                 INSERT INTO tender_versions(tender_id, seq, caused_by_notice_id, published_at, publication_id)
                   VALUES (1,1,10,300,'a'),(2,1,11,100,'b'),(3,1,12,200,'c');
                 INSERT INTO tender_version_texts(tender_id, seq, lot_id, field, lang, value)
                   VALUES (1,1,NULL,'title','ENG','Newest'),(3,1,NULL,'title','ENG','Middle');",
            )
            .await
            .unwrap();
        }
        db.set_foreign_keys(true).await.unwrap();

        let all = db.list_tenders(10).await.unwrap();
        assert_eq!(
            all.iter().map(|t| t.id).collect::<Vec<_>>(),
            vec![1, 3, 2],
            "ordered by current head date, newest first"
        );
        assert_eq!(all[0].title, "Newest");
        assert_eq!(all[2].title, "(untitled)", "a tender with no title row falls back");

        // The limit is a top-N over the ordering, not a slice of insertion order.
        let top = db.list_tenders(2).await.unwrap();
        assert_eq!(top.iter().map(|t| t.id).collect::<Vec<_>>(), vec![1, 3]);

        let _ = std::fs::remove_file(&path);
    }

    /// Task #27: the projection may not commit a Tender whose head pointer is not
    /// its last version.
    ///
    /// The reachable way to produce one: a Tender whose chain goes to EMPTY. The
    /// reconcile deletes the stored versions, then skips the head update because
    /// there is no last version to point at (`p.versions.last()` is `None`), so
    /// `current_seq` keeps pointing into versions that no longer exist. Every
    /// satellite view joins `t.seq = t.current_seq`, so the Tender would serve a
    /// reading assembled from a version that is gone — no error, no log line.
    ///
    /// This drives the real `apply_tenders`, not a hand-written UPDATE: the value
    /// of the assertion is that it fires on what the code actually does. The first
    /// apply establishes a two-version chain; the second re-projects the same
    /// Tender with no versions at all, and must be REFUSED before it commits.
    #[tokio::test]
    async fn a_head_that_is_not_the_last_version_is_refused_before_commit() {
        let path = format!("/tmp/tender-db-headmax-{}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();

        let version = |notice_id: i64, published_at: i64| canonical::TenderVersion {
            caused_by_notice_id: notice_id,
            published_at,
            dispatched_at: None,
            notice_subtype: None,
            original_lang: None,
            publication_id: format!("{notice_id}-2024"),
            facts: Default::default(),
            lots: Vec::new(),
            rounds: Vec::new(),
            group_members: Vec::new(),
        };
        let projection = |versions: Vec<canonical::TenderVersion>| canonical::TenderProjection {
            source: "ted".into(),
            procedure_key: Some("key:1".into()),
            island_notice_id: None,
            kind: "procedure".into(),
            versions,
        };

        // A healthy two-version chain commits, and its head points at seq 2.
        db.apply_tenders(&[projection(vec![version(10, 100), version(11, 200)])], 0, false)
            .await
            .expect("a well-formed chain applies");
        async fn head(db: &Db) -> Option<i64> {
            match db.scalar("SELECT current_seq FROM tenders WHERE id = 1").await.unwrap() {
                Some(turso::Value::Integer(n)) => Some(n),
                _ => None,
            }
        }
        assert_eq!(head(&db).await, Some(2), "the head points at the last version");

        // The poison: the same Tender re-projected with an EMPTY chain. Both
        // stored versions are deleted and the head update is skipped, so the
        // pointer would be left at 2 with no version 2 to point at.
        let err = db
            .apply_tenders(&[projection(Vec::new())], 0, false)
            .await
            .expect_err("a head left pointing at a deleted version must be refused");
        let message = err.to_string();
        assert!(
            message.contains("head") && message.contains("tender 1"),
            "the error names the invariant and the Tender: {message}"
        );

        // Refused BEFORE COMMIT, so the batch rolled back whole: the versions the
        // poisoned apply deleted are still there, and the head still agrees with
        // them. A check that let the delete land and only complained afterwards
        // would leave exactly the corruption it was added to prevent.
        assert_eq!(head(&db).await, Some(2), "the rejected batch rolled back");
        assert_eq!(
            db.scalar("SELECT COUNT(*) FROM tender_versions WHERE tender_id = 1").await.unwrap(),
            Some(turso::Value::Integer(2)),
            "both versions survive the refused apply"
        );

        // The scale guarantee, asserted as a PLAN rather than a stopwatch (the
        // issue-80 lesson: a per-row probe inside a full-corpus loop has to SEEK,
        // and a laptop-scale clock cannot tell a seek from a scan). This runs once
        // per write batch on every projection, including the 8.1M-Tender rebuild,
        // so a full scan of `tender_versions` here would be catastrophic and
        // invisible. Both sides must seek: `tenders` by primary key, the MAX(seq)
        // subquery by the (tender_id, seq) primary key.
        let conn = db.reader().await.unwrap();
        async fn plan_of(conn: &turso::Connection, sql: &str) -> String {
            let mut rows = conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), ()).await.unwrap();
            let mut plan = String::new();
            while let Some(row) = rows.next().await.unwrap() {
                if let Ok(turso::Value::Text(detail)) = row.get_value(3) {
                    plan.push_str(&detail);
                    plan.push('\n');
                }
            }
            assert!(!plan.is_empty(), "no plan came back for: {sql}");
            plan
        }
        let check = "SELECT t.id, t.current_seq,
                            (SELECT MAX(v.seq) FROM tender_versions v WHERE v.tender_id = t.id)
                       FROM tenders t
                      WHERE t.id IN (1, 2, 3)
                        AND t.current_seq IS NOT
                            (SELECT MAX(v.seq) FROM tender_versions v WHERE v.tender_id = t.id)";
        let plan = plan_of(&conn, check).await;
        assert!(!plan.contains("SCAN"), "the integrity check must seek both sides: {plan}");

        // And the predicate above must be capable of saying NO — the plan prints
        // ALIASES (`SCAN t`), so a check written against table NAMES would pass
        // whatever the planner did. Same statement with the primary-key seek
        // defeated: if this does not scan, the assertion above proves nothing.
        let defeated = check.replace("t.id IN (1, 2, 3)", "t.id + 0 IN (1, 2, 3)");
        assert!(
            plan_of(&conn, &defeated).await.contains("SCAN"),
            "the no-SCAN predicate cannot distinguish a seek from a scan"
        );

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// Issue 234: an identifier-less mention REUSES the Organization its
    /// `(name_norm, country)` already resolved to, instead of minting a fresh
    /// provisional row per mention — the defect that left 95.30 % of a 24.6M-row
    /// table provisional and made every legacy buyer rollup mostly fragments.
    ///
    /// The acceptance's three-way split, plus the two conservatism edges the
    /// issue's over-merge evidence demanded: nameless mentions never merge (2,411
    /// of a window's nameless rows were awarded contractors — distinct unknown
    /// parties, not one party), and a country-less name never merges (that is
    /// where platform strings like `tendsign` concentrate). Case-insensitive by
    /// construction, since the probe key is `name_norm`.
    /// ADR-0013 D4: a newly recorded mention's labelled name variants land in
    /// the `organization_names` satellite (name_norm Unicode-lowercased); the
    /// idempotent re-resolve path writes nothing more.
    #[tokio::test]
    async fn name_variants_land_in_the_satellite_once() {
        let path = format!("/tmp/tender-db-orgnames-{}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();
        let m = Mention {
            notice_id: 5,
            section_id: "ORG-1".into(),
            name: "Stadt Brüssel".into(),
            country: Some("BE".into()),
            raw_identifier: None,
            scheme: None,
            identifier: None,
            variants: vec![
                ("DEU".into(), "Stadt Brüssel".into()),
                ("FRA".into(), "Ville de Bruxelles".into()),
            ],
        };
        let mut resolver = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
        let org = db.resolve_mentions(&mut resolver, &[m.clone()], 100).await.unwrap()[0];
        let count = int_of(&db, "SELECT COUNT(*) FROM organization_names").await;
        assert_eq!(count, Some(2), "both labelled variants recorded");
        assert_eq!(
            text_of(
                &db,
                "SELECT name_norm FROM organization_names WHERE lang = 'DEU'"
            )
            .await
            .as_deref(),
            Some("stadt brüssel"),
            "Unicode-lowercased in Rust, umlaut intact"
        );
        assert_eq!(
            int_of(&db, "SELECT org_id FROM organization_names WHERE lang = 'FRA'").await,
            Some(org),
            "rows key to the resolved org"
        );
        // Re-resolving the same mention hits the idempotency map: no new rows.
        let again = db.resolve_mentions(&mut resolver, &[m], 100).await.unwrap()[0];
        assert_eq!(again, org);
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM organization_names").await, Some(2));
        drop(db);
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// Issue 358: the table IS the decision — every code whose register is
    /// the parent's folds, every code with a register of its own stays.
    #[test]
    fn register_jurisdiction_folds_only_codes_whose_register_is_the_parents() {
        for (code, register) in [
            ("GP", "FR"), ("MQ", "FR"), ("GF", "FR"), ("RE", "FR"), ("YT", "FR"),
            ("PM", "FR"), ("BL", "FR"), ("MF", "FR"), ("WF", "FR"),
            ("AX", "FI"), ("GL", "DK"), ("SJ", "NO"),
        ] {
            assert_eq!(register_jurisdiction(code), register, "{code} → {register}");
        }
        // A register of their own: RIDET, numéro Tahiti, Skráseting Føroya,
        // the Aruban/Curaçao/Sint Maarten/BES chambers.
        for code in ["NC", "PF", "FO", "AW", "CW", "SX", "BQ"] {
            assert_eq!(register_jurisdiction(code), code, "{code} keeps its own register");
        }
        // Parents and ordinary codes pass through; so does junk.
        for code in ["FR", "FI", "DK", "NO", "NL", "DE", "GB", "EL", "1A", ""] {
            assert_eq!(register_jurisdiction(code), code);
        }
    }

    /// Issue 358, the store's half: a mention tagged with a regional code
    /// mints its org row under the register's jurisdiction, keeps the
    /// regional code on its own mention row, and a later mention under the
    /// parent code REUSES that row. (The identifier half — `Identifier.country`
    /// arriving already mapped — is the ingest normaliser's, pinned there.)
    #[tokio::test]
    async fn a_regional_code_mention_mints_under_its_register_jurisdiction() {
        let path = format!("/tmp/tender-db-register-jurisdiction-{}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();

        let mention = |notice: i64, name: &str, country: &str| Mention {
            notice_id: notice,
            section_id: "ORG-1".into(),
            name: name.into(),
            country: Some(country.into()),
            raw_identifier: None,
            scheme: None,
            identifier: None,
            variants: Vec::new(),
        };
        let mut resolver = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
        let ids = db
            .resolve_mentions(
                &mut resolver,
                &[
                    mention(1, "SDIS de la Réunion", "RE"),
                    mention(2, "SDIS de la Réunion", "FR"),
                    mention(3, "Ålands landskapsregering", "AX"),
                    // A register of its own: New Caledonia stays NC, so the
                    // same name under FR is another body.
                    mention(4, "Province Sud", "NC"),
                    mention(5, "Province Sud", "FR"),
                ],
                0,
            )
            .await
            .unwrap();
        db.finish_mention_resolver(resolver).await.unwrap();

        assert_eq!(ids[0], ids[1], "RE and FR name the same register: one row");
        assert_ne!(ids[3], ids[4], "NC keeps its own register: two rows");
        let country_of = |id: i64| format!("SELECT country FROM organizations WHERE id = {id}");
        for (id, want) in [(ids[0], "FR"), (ids[2], "FI"), (ids[3], "NC"), (ids[4], "FR")] {
            match db.scalar(&country_of(id)).await.unwrap() {
                Some(turso::Value::Text(c)) => assert_eq!(c, want, "org {id}"),
                other => panic!("org {id}: {other:?}"),
            }
        }
        // The mention row keeps what the notice published.
        match db
            .scalar("SELECT country FROM organization_mentions WHERE notice_id = 1")
            .await
            .unwrap()
        {
            Some(turso::Value::Text(c)) => assert_eq!(c, "RE", "the mention keeps the regional code"),
            other => panic!("mention 1: {other:?}"),
        }
    }

    #[tokio::test]
    async fn an_identifierless_mention_reuses_its_named_organization() {
        let path = format!("/tmp/tender-db-namemerge-{}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();

        let mention = |notice: i64, name: &str, country: Option<&str>| Mention {
            notice_id: notice,
            section_id: "ORG-1".into(),
            name: name.into(),
            country: country.map(Into::into),
            raw_identifier: None,
            scheme: None,
            identifier: None,
            variants: Vec::new(),
        };
        let mut resolver = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
        let ids = db
            .resolve_mentions(
                &mut resolver,
                &[
                    // The same authority, twice — once with the case the probe must
                    // fold away. ONE Organization.
                    mention(1, "Mairie de Paris", Some("FR")),
                    mention(2, "MAIRIE DE PARIS", Some("FR")),
                    // The same name in another country: a DIFFERENT body.
                    mention(3, "Mairie de Paris", Some("US")),
                    // No country: never merged, even with itself.
                    mention(4, "Mairie de Paris", None),
                    mention(5, "Mairie de Paris", None),
                    // Nameless: never merged, even in one country.
                    mention(6, "", Some("FR")),
                    mention(7, "", Some("FR")),
                ],
            0,
            )
            .await
            .unwrap();
        db.finish_mention_resolver(resolver).await.unwrap();

        assert_eq!(ids[0], ids[1], "same name + country resolves to ONE organization");
        assert_ne!(ids[0], ids[2], "the same name in another country stays separate");
        assert_ne!(ids[3], ids[4], "a country-less name never merges");
        assert_ne!(ids[5], ids[6], "nameless mentions never merge");
        match db.scalar("SELECT COUNT(*) FROM organizations").await.unwrap() {
            Some(turso::Value::Integer(6)) => {}
            other => panic!("7 mentions must yield exactly 6 organizations, got {other:?}"),
        }

        // The reuse is durable, not only in-run: a FRESH resolver (new run, empty
        // caches) probes the table itself and still reuses.
        let mut resolver = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
        let again = db
            .resolve_mentions(&mut resolver, &[mention(8, "mairie de paris", Some("FR"))], 0)
            .await
            .unwrap();
        db.finish_mention_resolver(resolver).await.unwrap();
        assert_eq!(again[0], ids[0], "a later run reuses through the (name_norm, country) probe");

        // And a name matching a CANONICAL organization does NOT capture it: the
        // probe is scoped `identifier IS NULL`, because promoting by bare name is
        // exactly the over-merge the issue declines (two bodies can share a name
        // with only one of them registered).
        let mut resolver = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
        let with_id = db
            .resolve_mentions(
                &mut resolver,
                &[Mention {
                    notice_id: 9,
                    section_id: "ORG-1".into(),
                    name: "Mairie de Paris".into(),
                    country: Some("FR".into()),
                    raw_identifier: Some("123".into()),
                    scheme: None,
                    identifier: Some(canonical::Identifier {
                        country: Some("FR".into()),
                        kind: "national".into(),
                        value: "123".into(),
                    }),
                    variants: Vec::new(),
                }],
                0,
            )
            .await
            .unwrap();
        db.finish_mention_resolver(resolver).await.unwrap();
        assert_ne!(with_id[0], ids[0], "an identifier-bearing mention keeps its own row");
        let mut resolver = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
        let nameless_probe = db
            .resolve_mentions(&mut resolver, &[mention(10, "Mairie de Paris", Some("FR"))], 0)
            .await
            .unwrap();
        db.finish_mention_resolver(resolver).await.unwrap();
        assert_eq!(
            nameless_probe[0], ids[0],
            "…and the name probe still finds the PROVISIONAL row, not the canonical one"
        );

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// Issue 234's backfill half. The resolver merge only PREVENTS new duplicate
    /// identifier-less Organizations — no fold can retro-collapse the stock,
    /// because the mention idempotency preload never re-resolves a recorded
    /// mention. This walk is what collapses it: each `(name_norm, country)`
    /// group keeps its minimum id, every referencing row is repointed, losers
    /// leave a `removed` change row and the survivor an `updated` one. The
    /// out-of-scope rows the issue's over-merge evidence protects —
    /// identifier-bearing, nameless, country-less, other-country — must not
    /// move. The winners PK ends in `organization_id`, so a lot_result already
    /// naming the survivor must DROP the loser's row rather than collide. Dry
    /// run counts and writes nothing; a second run finds nothing.
    #[tokio::test]
    async fn the_org_merge_backfill_collapses_and_repoints_every_reference() {
        let path = format!("/tmp/tender-db-orgmerge-{}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();
        {
            let conn = db.conn().await;
            for (id, country, kind, ident, name) in [
                (1, Some("DE"), None, None, "Stadt Musterstadt"),
                (2, Some("DE"), None, None, "STADT MUSTERSTADT"),
                (3, Some("DE"), None, None, "Stadt Musterstadt"),
                // The same name in another country: a different body.
                (4, Some("FR"), None, None, "Stadt Musterstadt"),
                // Nameless: a distinct unknown party, never merged.
                (5, Some("DE"), None, None, ""),
                // Country-less: no scope to err inside, never merged.
                (6, None, None, None, "Stadt Musterstadt"),
                // Identifier-bearing: its own canonical row, untouched.
                (7, Some("DE"), Some("national"), Some("X1"), "Stadt Musterstadt"),
            ] {
                conn.execute(
                    "INSERT INTO organizations(id, country, identifier_kind, identifier, name, \
                                               name_norm, provisional, created_at) \
                     VALUES(?, ?, ?, ?, ?, ?, ?, 0)",
                    (
                        Value::Integer(id),
                        opt_text(country),
                        opt_text(kind),
                        opt_text(ident),
                        t(name),
                        Value::Text(name.to_lowercase()),
                        Value::Integer(if ident.is_some() { 0 } else { 1 }),
                    ),
                )
                .await
                .unwrap();
            }
            for (notice, org) in [(1, 1), (2, 2), (3, 3)] {
                conn.execute(
                    "INSERT INTO organization_mentions(notice_id, section_id, organization_id) \
                     VALUES(?, 'ORG-1', ?)",
                    (Value::Integer(notice), Value::Integer(org)),
                )
                .await
                .unwrap();
            }
            conn.execute(
                "INSERT INTO tender_version_parties(tender_id, seq, lot_id, role, organization_id, \
                                                    mention_notice_id, mention_section_id) \
                 VALUES(100, 1, NULL, 'buyer', 2, 2, 'ORG-1')",
                (),
            )
            .await
            .unwrap();
            conn.execute(
                "INSERT INTO tender_version_bid_parties(tender_id, seq, bid_id, role, \
                                                        organization_id, mention_notice_id, \
                                                        mention_section_id) \
                 VALUES(100, 1, 55, 'tenderer', 3, 3, 'ORG-1')",
                (),
            )
            .await
            .unwrap();
            for (lot_result, org) in [(77, 1), (77, 2), (88, 3)] {
                conn.execute(
                    "INSERT INTO tender_version_result_winners(tender_id, seq, lot_result_id, \
                                                               organization_id) \
                     VALUES(100, 1, ?, ?)",
                    (Value::Integer(lot_result), Value::Integer(org)),
                )
                .await
                .unwrap();
            }
        }
        db.set_foreign_keys(true).await.unwrap();

        let count = |sql: &'static str| {
            let db = &db;
            async move {
                let conn = db.conn().await;
                let mut rows = conn.query(sql, ()).await.unwrap();
                rows.next().await.unwrap().map_or(-1, |row| int(&row, 0))
            }
        };

        let dry = db.merge_provisional_organizations_batch(1_000, "", true).await.unwrap();
        assert_eq!((dry.groups, dry.removed), (1, 2), "one group of ids 1+2+3, two losers");
        assert!(dry.done);
        assert_eq!(count("SELECT COUNT(*) FROM organizations").await, 7, "dry run writes nothing");

        let r = db.merge_provisional_organizations_batch(1_000, "", false).await.unwrap();
        assert_eq!((r.groups, r.removed), (1, 2));
        assert_eq!(r.mentions, 2, "notices 2 and 3's mentions repoint to org 1");
        assert_eq!(r.parties, 1);
        assert_eq!(r.bid_parties, 1);
        assert_eq!(r.winner_dups, 1, "lot_result 77 already names the survivor — loser row dropped");
        assert_eq!(r.winners, 1, "lot_result 88's row repoints");
        assert!(r.done);

        assert_eq!(count("SELECT COUNT(*) FROM organizations").await, 5, "losers 2 and 3 deleted");
        assert_eq!(
            count("SELECT COUNT(*) FROM organizations WHERE id IN (1, 4, 5, 6, 7)").await,
            5,
            "survivor and every out-of-scope row remain"
        );
        assert_eq!(
            count("SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 1").await,
            3
        );
        assert_eq!(
            count("SELECT COUNT(*) FROM tender_version_parties WHERE organization_id = 1").await,
            1
        );
        assert_eq!(
            count("SELECT COUNT(*) FROM tender_version_bid_parties WHERE organization_id = 1").await,
            1
        );
        assert_eq!(count("SELECT COUNT(*) FROM tender_version_result_winners").await, 2);
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM tender_version_result_winners \
                  WHERE organization_id = 1 AND lot_result_id IN (77, 88)"
            )
            .await,
            2,
            "one winner row per lot_result, both naming the survivor"
        );
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM changes \
                  WHERE entity_kind = 'organization' AND op = 'removed' AND entity_id IN (2, 3)"
            )
            .await,
            2
        );
        // Issue 285: the survivor's change is `changed` (documented enum), not the
        // former out-of-enum `updated`.
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM changes \
                  WHERE entity_kind = 'organization' AND op = 'changed' AND entity_id = 1"
            )
            .await,
            1
        );

        let again = db.merge_provisional_organizations_batch(1_000, "", false).await.unwrap();
        assert_eq!((again.groups, again.removed), (0, 0), "the merge is idempotent");

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// The merge walk's batch boundary is NAME-aligned: a batch cut mid-name
    /// drops that whole trailing name and resumes there, so no `(name, country)`
    /// group is ever split across batches — and a name too large for any batch
    /// takes the unbounded-refetch path instead of stalling the walk.
    #[tokio::test]
    async fn the_org_merge_walk_advances_name_aligned_batches() {
        let path = format!("/tmp/tender-db-orgmergebatch-{}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        let db = Db::open(&path).await.unwrap();
        {
            let conn = db.conn().await;
            for name in ["aaa", "aaa", "bbb", "bbb", "bbb", "ccc", "ccc"] {
                conn.execute(
                    "INSERT INTO organizations(country, identifier_kind, identifier, name, \
                                               name_norm, provisional, created_at) \
                     VALUES('DE', NULL, NULL, ?, ?, 1, 0)",
                    (t(name), t(name)),
                )
                .await
                .unwrap();
            }
        }

        let b1 = db.merge_provisional_organizations_batch(4, "", false).await.unwrap();
        assert_eq!(b1.cursor, "aaa", "the cut-mid-name trailing 'bbb' rows were dropped");
        assert_eq!((b1.groups, b1.removed, b1.done), (1, 1, false));
        let b2 = db.merge_provisional_organizations_batch(4, &b1.cursor, false).await.unwrap();
        assert_eq!(b2.cursor, "bbb");
        assert_eq!((b2.groups, b2.removed, b2.done), (1, 2, false));
        let b3 = db.merge_provisional_organizations_batch(4, &b2.cursor, false).await.unwrap();
        assert_eq!((b3.groups, b3.removed), (1, 1));
        assert!(b3.done, "a short scan ends the walk");
        {
            let conn = db.conn().await;
            let mut rows =
                conn.query("SELECT COUNT(*) FROM organizations", ()).await.unwrap();
            assert_eq!(rows.next().await.unwrap().map_or(-1, |row| int(&row, 0)), 3);
        }

        // One name larger than the whole batch: the refetch path.
        {
            let conn = db.conn().await;
            for _ in 0..5 {
                conn.execute(
                    "INSERT INTO organizations(country, identifier_kind, identifier, name, \
                                               name_norm, provisional, created_at) \
                     VALUES('DE', NULL, NULL, 'zzz', 'zzz', 1, 0)",
                    (),
                )
                .await
                .unwrap();
            }
        }
        let g = db.merge_provisional_organizations_batch(2, "ccc", false).await.unwrap();
        assert_eq!((g.groups, g.removed, g.done), (1, 4, false), "refetched complete and merged");
        assert_eq!(g.cursor, "zzz");
        assert_eq!(g.scanned, 5, "all five rows, though the batch holds two");
        let end = db.merge_provisional_organizations_batch(2, &g.cursor, false).await.unwrap();
        assert!(end.done);
        assert_eq!(end.scanned, 0);
        {
            let conn = db.conn().await;
            let mut rows = conn
                .query("SELECT COUNT(*) FROM organizations WHERE name_norm = 'zzz'", ())
                .await
                .unwrap();
            assert_eq!(rows.next().await.unwrap().map_or(-1, |row| int(&row, 0)), 1);
        }

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// The merge walk's whole cost model is "one ordered pass over
    /// `organizations_name_country`" — grouping in Rust on index-ordered rows.
    /// This engine has declined composite-index access before (239: no pushdown
    /// into views; 248: DELETE ignoring a composite-PK index; 256: a LIST
    /// SUBQUERY re-scanned per outer row for six hours), so the assumption is a
    /// gate, asserted against the constant the walk actually runs.
    #[tokio::test]
    async fn the_org_merge_scan_walks_the_name_index_in_order() {
        let path = format!("/tmp/tender-db-orgmergeeqp-{}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        let db = Db::open(&path).await.unwrap();
        db.build_organization_indexes().await.unwrap();

        let conn = db.conn().await;
        let mut rows = conn
            .query(
                &format!("EXPLAIN QUERY PLAN {}", crate::canonical::ORG_MERGE_SCAN_SQL),
                (Value::Text(String::new()), Value::Integer(10)),
            )
            .await
            .unwrap();
        let mut plan = String::new();
        while let Some(row) = rows.next().await.unwrap() {
            plan.push_str(&text(&row, 3));
            plan.push('\n');
        }
        drop(rows);
        assert!(
            plan.contains("organizations_name_country"),
            "the scan must walk the name index — plan was:\n{plan}"
        );
        assert!(
            !plan.to_uppercase().contains("TEMP B-TREE"),
            "ordering must come from the index, not a sorter — plan was:\n{plan}"
        );

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// Issue 103, both gates in one place: a forced rewrite that produces FEWER
    /// entities than the stored chain had must sweep the strays — and one that
    /// produces the SAME entities must delete nothing, because issue 99's
    /// epoch-rewrite byte-identity depends on the entity rows (and their
    /// surrogate ids) surviving an unchanged rewrite untouched.
    ///
    /// This became reachable on 2026-08-20: issue 100's fix stopped three DE-1.x
    /// reference carriers minting phantom bids/contracts, so the very next refold
    /// was the first shrinking rewrite. `delete_version` deliberately clears only
    /// the version-keyed satellites, so without the sweep the carriers' rows
    /// survive with nothing referencing them — invisible to every read path
    /// (which all join through the satellites) but wrong in `COUNT(*)` and
    /// permanent.
    ///
    /// Driven through the real `apply_tenders`, with the rewrite forced the same
    /// way production forces it (`projection_epoch` stamped stale), not by a
    /// hand-run DELETE.
    #[tokio::test]
    async fn a_shrinking_rewrite_sweeps_the_entities_no_version_references() {
        let path = format!("/tmp/tender-db-sweep-{}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();

        let bid = |key: &str| canonical::BidState {
            key: key.into(),
            lot_key: Some("LOT-0001".into()),
            cents: Some(1_000),
            currency: Some("EUR".into()),
            quality: None,
            parties: Vec::new(),
        };
        let round = |bids: Vec<canonical::BidState>| canonical::Round {
            notice_id: 10,
            logical_notice_id: None,
            lot_results: vec![canonical::LotResultState {
                key: "RES-0001".into(),
                lot_key: Some("LOT-0001".into()),
                decision: None,
                reason: None,
                awarded_cents: None,
                awarded_currency: None,
                decided: None,
                winners: Vec::new(),
                statistics: Vec::new(),
            }],
            bids,
            contracts: Vec::new(),
        };
        let projection = |bids: Vec<canonical::BidState>| canonical::TenderProjection {
            source: "ted".into(),
            procedure_key: Some("key:sweep".into()),
            island_notice_id: None,
            kind: "procedure".into(),
            versions: vec![canonical::TenderVersion {
                caused_by_notice_id: 10,
                published_at: 100,
                dispatched_at: None,
                notice_subtype: None,
                original_lang: None,
                publication_id: "10-2024".into(),
                facts: Default::default(),
                lots: Vec::new(),
                rounds: vec![round(bids)],
                group_members: Vec::new(),
            }],
        };
        async fn count(db: &Db, sql: &str) -> i64 {
            match db.scalar(sql).await.unwrap() {
                Some(turso::Value::Integer(n)) => n,
                other => panic!("expected a count, got {other:?}"),
            }
        }

        // The old projection logic: one result, TWO bids (one of them the phantom
        // reference carrier this shape stands in for).
        db.apply_tenders(&[projection(vec![bid("TEN-0001"), bid("TEN-9999")])], 0, false)
            .await
            .expect("the wide chain applies");
        assert_eq!(count(&db, "SELECT COUNT(*) FROM bids").await, 2);

        // Gate 2 first — the UNCHANGED forced rewrite. Same content, epoch stamped
        // stale exactly as a deploy with a bumped PROJECTION_EPOCH finds it: the
        // rewrite must reuse both bid rows and sweep nothing.
        db.set_projection_epoch_for_test(0).await.unwrap();
        let ids_before: Vec<i64> = {
            let conn = db.reader().await.unwrap();
            let mut rows = conn.query("SELECT id FROM bids ORDER BY id", ()).await.unwrap();
            let mut out = Vec::new();
            while let Some(row) = rows.next().await.unwrap() {
                if let turso::Value::Integer(n) = row.get_value(0).unwrap() {
                    out.push(n);
                }
            }
            out
        };
        let applied = db
            .apply_tenders(&[projection(vec![bid("TEN-0001"), bid("TEN-9999")])], 0, false)
            .await
            .expect("the unchanged forced rewrite applies");
        assert_eq!(applied.entities_swept, 0, "an unchanged rewrite must sweep NOTHING");
        assert_eq!(count(&db, "SELECT COUNT(*) FROM bids").await, 2);
        let ids_after: Vec<i64> = {
            let conn = db.reader().await.unwrap();
            let mut rows = conn.query("SELECT id FROM bids ORDER BY id", ()).await.unwrap();
            let mut out = Vec::new();
            while let Some(row) = rows.next().await.unwrap() {
                if let turso::Value::Integer(n) = row.get_value(0).unwrap() {
                    out.push(n);
                }
            }
            out
        };
        assert_eq!(ids_before, ids_after, "issue 99: same surrogate ids, byte-identical rewrite");

        // Gate 1 — the SHRINKING forced rewrite. The narrowed logic produces one
        // bid; the stray must be swept and announced on the change feed.
        db.set_projection_epoch_for_test(0).await.unwrap();
        let applied = db
            .apply_tenders(&[projection(vec![bid("TEN-0001")])], 0, false)
            .await
            .expect("the shrinking rewrite applies");
        assert_eq!(applied.entities_swept, 1, "exactly the stray bid is swept");
        assert_eq!(count(&db, "SELECT COUNT(*) FROM bids").await, 1);
        assert_eq!(
            count(&db, "SELECT COUNT(*) FROM tender_version_bids").await,
            1,
            "the surviving reference set matches"
        );
        // The survivor is the SAME row it always was — the sweep deletes strays,
        // it never renumbers what stays.
        assert_eq!(count(&db, "SELECT MIN(id) FROM bids").await, ids_before[0]);
        assert_eq!(
            count(
                &db,
                "SELECT COUNT(*) FROM changes WHERE entity_kind = 'bid' AND op = 'removed'"
            )
            .await,
            1,
            "issue 164's discipline: a removal a subscriber cannot see leaves them holding ghosts"
        );

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// Issue 25, the O(page) guarantee: the newest-Tenders list must read the
    /// `tenders_current_published` index in order and stop at the limit, never
    /// materialise-and-sort every tender. Asserting the query plan proves this at
    /// any scale without needing a giant dataset — turso plans it as
    /// `SCAN tenders USING COVERING INDEX tenders_current_published`, no temp
    /// b-tree. (The old `v_tenders` form sorted all current rows on every call.)
    #[tokio::test]
    async fn list_tenders_orders_from_the_index_not_a_sort() {
        let path = format!("/tmp/tender-db-eqp-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let conn = db.reader().await.unwrap();
        let mut rows = conn
            .query(
                "EXPLAIN QUERY PLAN
                 SELECT t.id FROM tenders t WHERE t.current_published_at IS NOT NULL
                  ORDER BY t.current_published_at DESC, t.id DESC LIMIT 200",
                (),
            )
            .await
            .unwrap();
        let mut plan = String::new();
        while let Some(row) = rows.next().await.unwrap() {
            plan.push_str(&text(&row, 3));
            plan.push('\n');
        }
        assert!(
            plan.contains("tenders_current_published"),
            "the list must read the head-date index — plan was:\n{plan}"
        );
        assert!(
            !plan.to_uppercase().contains("TEMP B-TREE"),
            "ordering must come from the index, not a full sort — plan was:\n{plan}"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// ADR-0011's previous-notice pass joins `notices` on `(source, publication_id)`,
    /// and `clear_plan_on`'s own comment states the intent as fact — `plan_prev_edge`
    /// carries `a_source` "so the lookup hits `notices`' UNIQUE(source, publication_id, …)
    /// index by its leading columns". Nothing checked it. It is also the step that pins
    /// one core for the better part of an hour on a full re-projection: 245,955 edges
    /// measured on job 288 (issue 256 part 2), every neighbouring step in the tens of
    /// seconds. A prefix of a composite UNIQUE is exactly the kind of access this engine
    /// has failed to select before (239: no pushdown into views; 248: DELETE ignoring a
    /// composite-PK index), so the assumption is worth a gate rather than a comment.
    ///
    /// Issue 323: the re-queue statement's PLAN, asserted against the SQL the
    /// code actually runs — the `PREV_EDGE_JOIN_SQL` discipline, because the
    /// first version of this probe pinned hand-copied literals and a panel
    /// showed a reordered spelling of the poison that such a copy would miss.
    ///
    /// `notices` carries `notices_parse_state`, and turso prefers it to the
    /// rowid, so the obvious spelling of this statement plans as a walk of
    /// every parsed notice in the corpus — 3.4M rows on prod — per statement.
    /// Issue 317's re-homing measured 1,185 seconds to move 416 rows, holding
    /// the single writer. The unary `+` is what demotes the term.
    ///
    /// Plans, not stopwatches, on purpose: at fixture scale a clock cannot
    /// tell a seek from a scan (the issue-80 lesson). The POISONED spellings
    /// are pinned too, so the day turso stops preferring that index this test
    /// fails and says the `+` can go.
    /// Issues 316/318/321: the two statements that ask `org_match_keys` about
    /// a key must SEEK, and the one on the ingest hot path especially.
    ///
    /// `org_match_keys` carries exactly one index, `(key_kind, key, org_id)`.
    /// Leading with `key_kind` is what makes these seeks, and the negative
    /// case is not hypothetical — issue 321's stale-key count shipped as
    /// `org_id = ? AND key = ?`, which turso answers with a full SCAN, run
    /// once per dropped row on the held writer connection. This pins the two
    /// live statements against their own builders and pins that the poisoned
    /// ordering still scans, so the day that stops being true the test says so
    /// rather than the wall quietly getting slower.
    #[tokio::test]
    async fn the_key_store_probes_seek_and_the_poisoned_ordering_still_does_not() {
        let path = format!("/tmp/tender-db-keyplan-eqp-{}.db", std::process::id());
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
        let db = Db::open(&path).await.unwrap();
        let conn = db.reader().await.unwrap();
        // The index is created by the key BUILD, not by `Db::open`, so a
        // fixture without it is not the schema the planner sees on the box.
        conn.execute(
            "CREATE INDEX IF NOT EXISTS org_match_keys_kk \
                 ON org_match_keys(key_kind, key, org_id)",
            (),
        )
        .await
        .unwrap();

        let plan_of = async |sql: &str| -> String {
            let mut rows = conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), ()).await.unwrap();
            let mut plan = String::new();
            while let Some(row) = rows.next().await.unwrap() {
                plan.push_str(&text(&row, 3));
                plan.push('\n');
            }
            assert!(!plan.is_empty(), "no plan came back for: {sql}");
            plan
        };

        // Issue 354: the generic probe now aliases the table (`k`) and joins
        // `organizations` by primary key, so the seek line names the alias;
        // the predicate is what proves the index leads on (key_kind, key).
        for sql in [
            crate::canonical::GENERIC_KEY_SQL,
            crate::canonical::NAME_KEY_CARRIERS_SQL,
            crate::canonical::STALE_KEY_COUNT_SQL,
        ] {
            let plan = plan_of(sql).await;
            assert!(
                plan.contains("USING INDEX org_match_keys_kk (key_kind=? AND key=?"),
                "must seek on (key_kind, key, …) — plan was:\n{plan}"
            );
            assert!(
                !plan.contains("SCAN org_match_keys") && !plan.contains("SCAN k\n") && !plan.contains("SCAN k "),
                "a scan here is the issue-321 shape: the whole key store, per \
                 call, and the generic probe runs on the INGEST hot path:\n{plan}"
            );
            if sql != crate::canonical::STALE_KEY_COUNT_SQL {
                assert!(
                    plan.contains("SEARCH o USING INTEGER PRIMARY KEY"),
                    "the issue-354 join must be a PK lookup per carrier, not a scan of organizations:\n{plan}"
                );
            }
        }

        // The shape that shipped, kept as the negative control. If turso ever
        // learns to seek this, the guard above is no longer load-bearing and
        // whoever reads this should know it.
        let poisoned = plan_of(
            "SELECT COUNT(*) FROM org_match_keys WHERE org_id = 1 AND key = 'x'",
        )
        .await;
        assert!(
            poisoned.contains("SCAN org_match_keys"),
            "the org_id-led ordering is expected to SCAN — if it now seeks, \
             turso changed and this whole probe wants re-reading:\n{poisoned}"
        );
    }

    /// Issue 326: the typo census must SEEK the far side, not scan it.
    ///
    /// The whole design rests on it. There is no index leading with
    /// `identifier`, so the alternative shape — group the corpus by identifier
    /// and look for country disagreement — sorts 1.1M rows (issue 117 measured
    /// that class at 15.16 s). Driving from the rarer country and seeking the
    /// other is only cheap if the seek is real, and "is it real" is a question
    /// about the planner, not about the SQL's appearance. Asserted against
    /// [`crate::canonical::COUNTRY_TYPO_JOIN_SQL`] itself, so a rewrite that
    /// loses the seek fails here rather than on prod.
    #[tokio::test]
    async fn the_country_typo_probe_seeks_the_far_country() {
        let path = format!("/tmp/tender-db-typo-eqp-{}.db", std::process::id());
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
        let db = Db::open(&path).await.unwrap();
        // `organizations_identity` is built by the reindex path, not by open —
        // a fixture without it is not the schema the box plans against.
        db.build_organization_indexes().await.unwrap();
        let conn = db.reader().await.unwrap();
        let mut rows = conn
            .query(
                &format!("EXPLAIN QUERY PLAN {}", crate::canonical::COUNTRY_TYPO_JOIN_SQL),
                (),
            )
            .await
            .unwrap();
        let mut plan = String::new();
        while let Some(row) = rows.next().await.unwrap() {
            plan.push_str(&text(&row, 3));
            plan.push('\n');
        }
        assert!(!plan.is_empty(), "no plan came back");
        // The far side, seeking on ALL THREE columns. Two would be a country
        // slice per row of the driving side — DE's is 148,355 rows — so the
        // assertion names the whole key, not just the index.
        assert!(
            plan.contains(
                "SEARCH b USING INDEX organizations_identity \
                 (country=? AND identifier_kind=? AND identifier=?)"
            ),
            "the far side must be a three-column seek on organizations_identity. \
             Anything less turns 2,275 pair probes into 2,275 corpus walks, which \
             is the whole cost argument inverted:\n{plan}"
        );
        // The driving side is a country range, not a table scan.
        assert!(
            plan.contains("SEARCH a USING INDEX") && plan.contains("(country=?)"),
            "the driving side must be an indexed country range:\n{plan}"
        );
        assert!(!plan.contains("SCAN"), "something scans:\n{plan}");
        assert!(
            !plan.contains("USE TEMP B-TREE"),
            "a sorter appeared — the seek was not used:\n{plan}"
        );
    }

    #[tokio::test]
    async fn the_requeue_statements_seek_notices_by_rowid() {
        let path = format!("/tmp/tender-db-requeue-eqp-{}.db", std::process::id());
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
        let db = Db::open(&path).await.unwrap();
        // The partial index prod carries is built lazily at the end of a
        // projection, never by `Db::open`, so a fixture that skips it is not
        // the schema the planner sees on the box.
        db.ensure_unprojected_index().await.unwrap();
        let conn = db.reader().await.unwrap();

        let plan_of = async |sql: &str| -> String {
            let mut rows =
                conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), ()).await.unwrap();
            let mut plan = String::new();
            while let Some(row) = rows.next().await.unwrap() {
                plan.push_str(&text(&row, 3));
                plan.push('\n');
            }
            assert!(!plan.is_empty(), "no plan came back for: {sql}");
            plan
        };

        // The three statements the re-queue path runs, from their own builders.
        for sql in [
            crate::canonical::requeue_update_sql(500),
            crate::canonical::requeue_count_sql(500),
        ] {
            let plan = plan_of(&sql).await;
            assert!(
                plan.contains("SEARCH notices USING INTEGER PRIMARY KEY"),
                "the re-queue must seek by rowid; the `+` on parse_state is what \
                 makes it — plan was:\n{plan}"
            );
            assert!(
                !plan.contains("notices_parse_state"),
                "the re-queue reached for the parse_state index — that is the \
                 3.4M-row corpus walk issue 323 measured:\n{plan}"
            );
        }
        let plan = plan_of(&crate::canonical::notice_ids_of_tenders_sql(500)).await;
        assert!(
            plan.contains("SEARCH tender_versions") && plan.contains("(tender_id=?)"),
            "the tender-side resolve must seek on tender_versions' \
             (tender_id, caused_by_notice_id) unique index. A renumbered \
             sqlite_autoindex name is fine; a SCAN is not:\n{plan}"
        );

        // The poison, in both spellings the repair paths used and in the
        // reordered one a source-grep guard would have missed.
        let list = crate::canonical::placeholders(500);
        for sql in [
            format!(
                "UPDATE notices SET projected = 0 \
                   WHERE parse_state = 'parsed' AND projected <> 0 AND id IN ({list})"
            ),
            format!(
                "UPDATE notices SET projected = 0 \
                   WHERE id IN ({list}) AND parse_state = 'parsed' AND projected <> 0"
            ),
            format!(
                "UPDATE notices SET projected = 0 \
                   WHERE parse_state = 'parsed' AND projected <> 0 AND id IN (\
                     SELECT caused_by_notice_id FROM tender_versions \
                      WHERE tender_id IN ({list}))"
            ),
        ] {
            let plan = plan_of(&sql).await;
            assert!(
                plan.contains("notices_parse_state"),
                "turso no longer prefers the parse_state index — re-measure issue \
                 323 and the unary `+` in requeue_update_sql may go:\n{plan}"
            );
        }

        // Issue 323's open residue, settled 2026-09-02. `parsed_id_stripes` carried
        // the SAME poisoned shape at a fifth site, behind a doc block asserting the
        // opposite — that the planner "picks the rowid range seek, not
        // notices_parse_state". It did not. Measured at 200k notices, the scoped
        // call the incremental pre-pass makes went 0.39s → 0.0011s.
        for sql in [crate::canonical::STRIPE_COUNT_SQL, crate::canonical::STRIPE_WALK_SQL] {
            let plan = plan_of(sql).await;
            assert!(
                plan.contains("SEARCH notices USING INTEGER PRIMARY KEY"),
                "the stripe partition must seek the id RANGE; the `+` on parse_state \
                 is what makes it:\n{plan}"
            );
            assert!(
                !plan.contains("notices_parse_state"),
                "the stripe partition reached for the parse_state index — that walks \
                 every parsed notice in the corpus and applies no id range, which is \
                 the whole of issue 323 at a fifth site:\n{plan}"
            );
        }
        // …and the same statements WITHOUT the `+` must still mis-plan, or the
        // guard above is passing for a reason that has nothing to do with it.
        for sql in [
            crate::canonical::STRIPE_COUNT_SQL.replace("+parse_state", "parse_state"),
            crate::canonical::STRIPE_WALK_SQL.replace("+parse_state", "parse_state"),
        ] {
            let plan = plan_of(&sql).await;
            assert!(
                plan.contains("notices_parse_state"),
                "without the `+` turso no longer prefers the parse_state index here \
                 — re-measure before dropping it:\n{plan}"
            );
        }
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
    }

    /// Asserted against `PREV_EDGE_JOIN_SQL` itself, so the plan can never be checked
    /// against a copy that has drifted from the statement the fold runs.
    #[tokio::test]
    async fn the_previous_notice_join_seeks_notices_rather_than_scanning_it() {
        let path = format!("/tmp/tender-db-preveqp-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        db.reset_plan().await.unwrap();

        let conn = db.reader().await.unwrap();
        let mut rows = conn
            .query(&format!("EXPLAIN QUERY PLAN {}", crate::canonical::PREV_EDGE_JOIN_SQL), ())
            .await
            .unwrap();
        let mut plan = String::new();
        while let Some(row) = rows.next().await.unwrap() {
            plan.push_str(&text(&row, 3));
            plan.push('\n');
        }

        // The `notices` side must be a seek, not a scan. Whatever the engine calls the
        // access path, the one thing it must not say is that it walks the table: at 14.3M
        // rows and 245,955 edges a scan per edge is the observed single-core hour.
        let notices_line = plan
            .lines()
            .find(|l| l.contains("notices") && !l.contains("plan_notice"))
            .unwrap_or_else(|| panic!("no `notices` access in the plan:\n{plan}"));
        assert!(
            !notices_line.to_uppercase().contains("SCAN NOTICES"),
            "the previous-notice join must SEEK notices by (source, publication_id), not scan \
             14.3M rows per edge — plan was:\n{plan}"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 82 regression: a rebuild empties the tender layer via
    /// `reset_tender_layer`, which DROPs the `tenders` table — losing every index on
    /// it, including `tenders_current_published` (created only by `migrate()` at
    /// process open). Only `DEFERRED_TENDER_INDEXES` is rebuilt at fold end, so that
    /// index MUST be in the set or the newest-Tenders list silently falls back to a
    /// full scan of all 8M+ tenders until the next restart. Reproduce the rebuild's
    /// index lifecycle and assert the list plan still reads the covering index.
    #[tokio::test]
    async fn rebuild_preserves_the_current_published_covering_index() {
        let path = format!("/tmp/tender-db-eqp-rebuild-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        // The from-scratch rebuild's tender-index lifecycle: strip the deferred
        // indexes, empty the layer (DROPs `tenders`), then rebuild the deferred set.
        db.strip_tender_indexes().await.unwrap();
        db.reset_tender_layer().await.unwrap();
        db.build_tender_indexes().await.unwrap();
        let conn = db.reader().await.unwrap();
        let mut rows = conn
            .query(
                "EXPLAIN QUERY PLAN
                 SELECT t.id FROM tenders t WHERE t.current_published_at IS NOT NULL
                  ORDER BY t.current_published_at DESC, t.id DESC LIMIT 200",
                (),
            )
            .await
            .unwrap();
        let mut plan = String::new();
        while let Some(row) = rows.next().await.unwrap() {
            plan.push_str(&text(&row, 3));
            plan.push('\n');
        }
        assert!(
            plan.contains("tenders_current_published"),
            "after a rebuild the list must still read the covering index (issue 82) — plan was:\n{plan}"
        );
        assert!(
            !plan.to_uppercase().contains("TEMP B-TREE"),
            "after a rebuild the list must not materialise-and-sort — plan was:\n{plan}"
        );
        let _ = std::fs::remove_file(&path);
    }

    /// Issue 49 part 4: the /v1/tenders list echoes each row's CPV/NUTS codes
    /// with a correlated subquery keyed by (tender_id, seq). It must seek the
    /// `tender_version_classifications_version` index, never scan the table —
    /// otherwise, over a page of rows, it reintroduces the issue-25 scan
    /// pathology. Asserting the plan pins O(page) at any scale.
    #[tokio::test]
    async fn classification_echo_seeks_the_version_index_not_a_scan() {
        let path = format!("/tmp/tender-db-eqp-cls-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let conn = db.reader().await.unwrap();
        let mut rows = conn
            .query(
                "EXPLAIN QUERY PLAN
                 SELECT group_concat(DISTINCT c.code) FROM tender_version_classifications c
                  WHERE c.tender_id = 1 AND c.seq = 1 AND c.scheme = 'cpv'",
                (),
            )
            .await
            .unwrap();
        let mut plan = String::new();
        while let Some(row) = rows.next().await.unwrap() {
            plan.push_str(&text(&row, 3));
            plan.push('\n');
        }
        assert!(
            plan.contains("tender_version_classifications_version"),
            "the echo subquery must seek the by-version index — plan was:\n{plan}"
        );
        assert!(
            !plan.to_uppercase().contains("SCAN"),
            "it must SEARCH by index, never SCAN the table — plan was:\n{plan}"
        );

        let _ = std::fs::remove_file(&path);
    }


    /// Issue 50: the analyst views take the current version from the maintained
    /// `current_seq` pointer, never a MAX(seq) GROUP BY over every version — the
    /// issue-25 pathology. (turso does not push a predicate through a view, so
    /// like the existing v_tenders/v_lots a filtered query materialises the view;
    /// the guarantee that matters here is "no version aggregation/sort", which
    /// holds regardless of the planner's join order or table stats.)
    #[tokio::test]
    async fn analyst_views_read_the_current_pointer_not_a_max_aggregation() {
        let path = format!("/tmp/tender-db-eqp-buyers-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let conn = db.reader().await.unwrap();
        for view in ["v_tender_buyers", "v_tender_classifications", "v_awards"] {
            let mut rows = conn
                .query(&format!("EXPLAIN QUERY PLAN SELECT * FROM {view} WHERE tender_id = 1"), ())
                .await
                .unwrap();
            let mut plan = String::new();
            while let Some(row) = rows.next().await.unwrap() {
                plan.push_str(&text(&row, 3));
                plan.push('\n');
            }
            // A MAX(seq) GROUP BY would show as an aggregation over a sorted temp
            // b-tree; the current_seq pointer never does.
            assert!(
                !plan.to_uppercase().contains("TEMP B-TREE"),
                "{view} must not sort/aggregate over versions — plan was:\n{plan}"
            );
        }
        let _ = std::fs::remove_file(&path);
    }

    /// A tender-scoped lots read must be driven by a `tender_id` key, never by a walk
    /// of `lots` (13.2M rows on prod) or of `tender_version_lots` in rowid order.
    ///
    /// This started as the `/v1/tenders/{id}` ~2.2s regression, where `read::lots`
    /// carried the cursor as a plain `l.id > ?` and turso drove from `lots` to satisfy
    /// `ORDER BY l.id`, filtering `tender_id` per row. Issue 115 then moved the read
    /// off `lots` entirely: the containment question is answered by seeking
    /// `tender_version_lots` on its `(tender_id, seq)` PK prefix.
    ///
    /// Covers `after > 0` — a REAL cursor page — not just the first page. A first-page
    /// -only fix leaves `/v1/lots?tender=&after=N` on the bad plan (measured 2237ms),
    /// so testing only `after = 0` would certify a half-fix as whole.
    ///
    /// Asserting the PLAN, not rows or latency: the rows were correct throughout the
    /// outage and every functional test passed — what was wrong was how they were
    /// reached. Note a presence-of-index gate would ALSO have passed here: the index
    /// existed the whole time. Only the plan shows it.
    ///
    /// It plans the statement `read::lots` ACTUALLY builds, via `lots_statement`,
    /// rather than a restatement of it. The earlier version of this test planned its
    /// own hand-written `(l.tender_id, l.id) > (?, ?)` string — and went on passing
    /// after issue 115 removed that cursor form from the builder, certifying SQL no
    /// code emitted. A plan test decoupled from its artifact is worse than none: it
    /// reports green about a shape nothing runs.
    ///
    /// A plan is a sound instrument for THIS question (which key drives the read) and
    /// an unsound one for cost — issue 115's quadratic read had a fully optimal plan
    /// while it took 248.8s. Hence the separate timing tests; see `lot_summary_cost`.
    /// EXTRACTION SEAM for 112's gate — a printer, not an assertion.
    ///
    /// Prints the statement the deployed `lots` builder ACTUALLY emits, with each `?`
    /// replaced by the value the builder ITSELF bound in the same call, so the gate
    /// plans the artifact rather than a hand-written paraphrase of it (issue 114
    /// part 2). No token is rewritten: the SQL comes from `lots_statement`, the
    /// values come from the params it returned alongside it.
    #[test]
    fn dump_lots_statement_for_the_plan_gate() {
        use super::read::{self, Filter, Scope};
        use turso::Value;
        /// Strip `-- …` line comments BEFORE any newline is collapsed. The builders
        /// carry explanatory comments inside their SQL, and flattening a statement to
        /// one line turns the first of them into a comment over EVERYTHING after it —
        /// turso then rejects the result as "incomplete input". A one-line fixture
        /// format makes this mandatory, not optional. Quote-aware, so a `--` inside a
        /// string literal survives.
        fn decomment(sql: &str) -> String {
            let mut out = String::new();
            for line in sql.lines() {
                let mut qd = false;
                let b: Vec<char> = line.chars().collect();
                let mut i = 0;
                while i < b.len() {
                    if b[i] == '\'' {
                        qd = !qd;
                    } else if !qd && b[i] == '-' && i + 1 < b.len() && b[i + 1] == '-' {
                        break;
                    }
                    out.push(b[i]);
                    i += 1;
                }
                out.push('\n');
            }
            out
        }
        fn inline(sql: &str, params: &[Value]) -> String {
            let sql = &decomment(sql);
            let mut out = String::new();
            let mut p = params.iter();
            for ch in sql.chars() {
                if ch == '?' {
                    match p.next() {
                        Some(Value::Integer(i)) => out.push_str(&i.to_string()),
                        Some(Value::Text(t)) => {
                            out.push('\'');
                            out.push_str(&t.replace('\'', "''"));
                            out.push('\'');
                        }
                        Some(other) => out.push_str(&format!("{other:?}")),
                        None => out.push('?'),
                    }
                } else if ch == '\n' {
                    out.push(' ');
                } else {
                    out.push(ch);
                }
            }
            // collapse the builder's indentation to one line, as the gate's table needs
            let mut flat = String::with_capacity(out.len());
            let mut space = false;
            for ch in out.chars() {
                if ch == ' ' {
                    if !space {
                        flat.push(ch);
                    }
                    space = true;
                } else {
                    space = false;
                    flat.push(ch);
                }
            }
            flat.trim().to_owned()
        }
        for (label, after) in [("B1", 0i64), ("B1b", 49_377)] {
            let filter = Filter { tender: Some(424_242), ..Filter::default() };
            let (sql, params) = read::lots_statement(&filter, Scope::Page { after, limit: 1000 });
            println!("GATE-SQL {label}: {}", inline(&sql, &params));
        }
    }

    /// 114 PART 2 PROTOTYPE — enumerate the paginated read set MECHANICALLY.
    ///
    /// The set is (collection x filter x cursor position), not "the reads someone
    /// remembered": `read_items`' match on `Collection` is the app's own exhaustive
    /// statement of which reads paginate, and one builder emits a DIFFERENT statement
    /// per filter — `organizations` was 22.0s fixable with `country` and 99.08s
    /// unservable with `kind`, from the same function.
    #[test]
    fn enumerate_paginated_read_statements() {
        use super::read::{self, Filter, Scope, Status};
        use turso::Value;
        /// Strip `-- …` line comments BEFORE any newline is collapsed. The builders
        /// carry explanatory comments inside their SQL, and flattening a statement to
        /// one line turns the first of them into a comment over EVERYTHING after it —
        /// turso then rejects the result as "incomplete input". A one-line fixture
        /// format makes this mandatory, not optional. Quote-aware, so a `--` inside a
        /// string literal survives.
        fn decomment(sql: &str) -> String {
            let mut out = String::new();
            for line in sql.lines() {
                let mut qd = false;
                let b: Vec<char> = line.chars().collect();
                let mut i = 0;
                while i < b.len() {
                    if b[i] == '\'' {
                        qd = !qd;
                    } else if !qd && b[i] == '-' && i + 1 < b.len() && b[i + 1] == '-' {
                        break;
                    }
                    out.push(b[i]);
                    i += 1;
                }
                out.push('\n');
            }
            out
        }
        fn inline(sql: &str, params: &[Value]) -> String {
            let sql = &decomment(sql);
            let mut out = String::new();
            let mut p = params.iter();
            for ch in sql.chars() {
                match ch {
                    '?' => match p.next() {
                        Some(Value::Integer(i)) => out.push_str(&i.to_string()),
                        Some(Value::Text(t)) => out.push_str(&format!("'{}'", t.replace('\'', "''"))),
                        Some(other) => out.push_str(&format!("{other:?}")),
                        None => out.push('?'),
                    },
                    '\n' => out.push(' '),
                    c => out.push(c),
                }
            }
            let mut flat = String::new();
            let mut sp = false;
            for c in out.chars() {
                if c == ' ' {
                    if !sp { flat.push(c) }
                    sp = true;
                } else { sp = false; flat.push(c) }
            }
            flat.trim().to_owned()
        }
        let base = Filter::default();
        let filters: Vec<(&str, Filter)> = vec![
            ("none", base.clone()),
            ("source", Filter { source: Some("ted".into()), ..base.clone() }),
            ("country", Filter { country: Some("DE".into()), ..base.clone() }),
            ("cpv", Filter { cpv: Some("4521".into()), ..base.clone() }),
            ("buyer", Filter { buyer: Some(7), ..base.clone() }),
            ("winner", Filter { winner: Some(7), ..base.clone() }),
            ("status", Filter { status: Some(Status::Open), ..base.clone() }),
            ("min_value", Filter { min_value: Some(1000), ..base.clone() }),
            ("max_value", Filter { max_value: Some(9000), ..base.clone() }),
            ("kind", Filter { kind: Some("Lot".into()), ..base.clone() }),
            ("tender", Filter { tender: Some(424_242), ..base.clone() }),
            ("identifier", Filter { identifier: Some("DE811907980".into()), ..base.clone() }),
        ];
        for (fname, f) in &filters {
            for after in [0i64, 49_377] {
                let scope = Scope::Page { after, limit: 1000 };
                for (coll, (sql, params)) in [
                    ("tenders", read::tenders_statement(f, scope)),
                    ("lots", read::lots_statement(f, scope)),
                    ("organizations", read::organizations_statement(f, scope)),
                    ("notices", read::notices_statement(f, scope)),
                ] {
                    println!("ENUM\t{coll}\t{fname}\tafter={after}\t{}", inline(&sql, &params));
                }
            }
        }
    }

    /// `Collection::honoured_params` is the list handler's source of truth for which
    /// filters actually applied — the set it diffs a request against to build the
    /// `ignored_filters` it echoes back (issue 118). It is a hand-written list, so it
    /// can lie. This is the belt to that brace, in the spirit of
    /// `filter_classification_is_exhaustive`: for every (collection × parameter) pair it
    /// compares the statement the builder emits with only that parameter set against the
    /// statement with none — the parameter is honoured IFF the SQL changes — and asserts
    /// that mechanical split equals what `honoured_params` claims. A builder that starts
    /// or stops reading a field fails this test until the set is corrected, so
    /// `ignored_filters` can never certify a filter that changed nothing.
    #[test]
    fn honoured_params_match_the_emitted_sql() {
        use super::read::{self, Collection, Filter, Scope, Status};

        // The client-facing parameter name paired with a Filter that sets ONLY it, to a
        // non-default value. `now` is the reference instant, not a filter; `cursor` and
        // `limit` are pagination — none is in the shared filter vocabulary this echoes.
        let one = |p: &str| -> Filter {
            let base = Filter::default();
            match p {
                "source" => Filter { source: Some("ted".into()), ..base },
                "country" => Filter { country: Some("DE".into()), ..base },
                "cpv" => Filter { cpv: Some("4521".into()), ..base },
                "buyer" => Filter { buyer: Some(7), ..base },
                "winner" => Filter { winner: Some(7), ..base },
                "bidder" => Filter { bidder: Some(7), ..base },
                "status" => Filter { status: Some(Status::Open), ..base },
                "min_value" => Filter { min_value: Some(1000), ..base },
                "max_value" => Filter { max_value: Some(9000), ..base },
                "currency" => Filter { currency: Some("EUR".into()), ..base },
                "kind" => Filter { kind: Some("Lot".into()), ..base },
                "tender" => Filter { tender: Some(424_242), ..base },
                "publication_id" => Filter { publication_id: Some("00018218-2024".into()), ..base },
                "identifier" => Filter { identifier: Some("DE811907980".into()), ..base },
                "published_after" => Filter { published_after: Some(1_754_000_000), ..base },
                "published_before" => Filter { published_before: Some(1_786_000_000), ..base },
                "deadline_after" => Filter { deadline_after: Some(1_754_000_000), ..base },
                "deadline_before" => Filter { deadline_before: Some(1_786_000_000), ..base },
                "name_prefix" => Filter { name_prefix: Some("siemens".into()), ..base },
                other => panic!("unknown parameter {other}"),
            }
        };
        let sql_of = |c: Collection, f: &Filter| -> String {
            let scope = Scope::Page { after: 0, limit: 1000 };
            match c {
                Collection::Tenders => read::tenders_statement(f, scope),
                Collection::Lots => read::lots_statement(f, scope),
                Collection::Organizations => read::organizations_statement(f, scope),
                Collection::Notices => read::notices_statement(f, scope),
            }
            .0
        };

        const ALL: [&str; 19] = [
            "source", "country", "cpv", "buyer", "winner", "bidder", "status", "min_value",
            "max_value", "currency", "kind", "tender", "publication_id", "identifier",
            "published_after", "published_before", "deadline_after", "deadline_before",
            "name_prefix",
        ];
        for c in
            [Collection::Tenders, Collection::Lots, Collection::Organizations, Collection::Notices]
        {
            let base = sql_of(c, &Filter::default());
            for p in ALL {
                let changed = sql_of(c, &one(p)) != base;
                let claimed = c.honoured_params().contains(&p);
                assert_eq!(
                    changed, claimed,
                    "{c:?}: parameter `{p}` changes the emitted SQL = {changed}, but \
                     honoured_params() says {claimed}. Update Collection::honoured_params \
                     (crates/store/src/read.rs) so ignored_filters stays honest."
                );
            }
        }
    }

    /// LOCAL PLAN PROBE for 112's gate — plans whatever statements `TDB_PLAN_SQL`
    /// names (one per line), against a schema-only DB built by `Db::open`, through
    /// the workspace's pinned turso (`=0.7.0`, the same version the on-box probe
    /// pins). Prints the plan in the on-box probe's column form so the gate's own
    /// parser can consume it unchanged.
    #[tokio::test]
    async fn local_eqp_probe() {
        let Ok(sqlfile) = std::env::var("TDB_PLAN_SQL") else { return };
        // Plan against the DB the caller names, so the gate's stats precondition
        // inspects the SAME file these plans come from.
        let path = std::env::var("TDB_PLAN_DB")
            .unwrap_or_else(|_| format!("/tmp/tender-db-eqp-probe-{}.db", std::process::id()));
        // TDB_PLAN_RAW opens the file AS IT IS, with no migration — the only way to
        // plan against a deliberately MUTILATED catalogue (an index removed) and see
        // a check go red. `Db::open` would repair the schema it is meant to be missing.
        // TDB_PLAN_DDL builds the catalogue from DDL executed BY TURSO, which is how a
        // deliberately mutilated schema (a PK or index removed) becomes a DB this engine
        // can actually open — one built by sqlite3 is not loadable here.
        if let Ok(ddl) = std::env::var("TDB_PLAN_DDL") {
            let _ = std::fs::remove_file(&path);
            let db = turso::Builder::new_local(&path).build().await.unwrap();
            let c = db.connect().unwrap();
            for stmt in std::fs::read_to_string(&ddl).unwrap().split(";\n") {
                if !stmt.trim().is_empty() {
                    c.execute(stmt, ()).await.unwrap();
                }
            }
        }
        let held;
        let conn = if std::env::var("TDB_PLAN_RAW").is_ok() {
            let db = turso::Builder::new_local(&path).build().await.unwrap();
            db.connect().unwrap()
        } else {
            held = Db::open(&path).await.unwrap();
            (*held.reader().await.unwrap()).clone()
        };
        for line in std::fs::read_to_string(&sqlfile).unwrap().lines() {
            if line.trim().is_empty() {
                continue;
            }
            println!("PLAN-BEGIN");
            let mut rows = conn.query(&format!("EXPLAIN QUERY PLAN {line}"), ()).await.unwrap();
            while let Some(row) = rows.next().await.unwrap() {
                println!("PLAN 1 | 0 | 0 | {}", text(&row, 3));
            }
            println!("PLAN-END");
        }
    }

    #[tokio::test]
    async fn a_tender_scoped_lots_read_seeks_the_index_instead_of_walking_rowids() {
        let path = format!("/tmp/tender-db-lots-eqp-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let conn = db.reader().await.unwrap();
        // Both the first page and a real cursor page must seek a tender_id key.
        for after in [0i64, 49_377] {
            let filter = read::Filter { tender: Some(1), ..read::Filter::default() };
            let (sql, params) =
                read::lots_statement(&filter, read::Scope::Page { after, limit: 1000 });
            let mut rows = conn
                .query(&format!("EXPLAIN QUERY PLAN {sql}"), params)
                .await
                .unwrap();
            let mut plan = String::new();
            while let Some(row) = rows.next().await.unwrap() {
                plan.push_str(&text(&row, 3));
                plan.push('\n');
            }
            assert!(
                plan.to_uppercase().contains("TENDER_ID="),
                "after={after}: a tender_id key must drive the read — plan was:\n{plan}"
            );
            assert_eq!(
                driver(&plan),
                Driver::VersionLots,
                "after={after}: `tender_version_lots` must be the OUTER loop — plan was:\n{plan}"
            );
        }
        let _ = std::fs::remove_file(&path);
    }

    /// A tender-scoped lots read returns the Tender's COMPLETE set, and still honours
    /// the cursor (issue 116).
    ///
    /// The truncation lived in `lots_of`'s limit: it asked for `MAX_PAGE` (1000), so
    /// the 16 Tenders (of 4.26M) carrying more than 1,000 lots shipped a detail
    /// response that contradicted itself — `"lots": 2604` beside 1,000 `lot_details`,
    /// with no marker that the array was cut and no cursor to reach the rest.
    ///
    /// The completeness half runs through the REAL detail path (`tender_detail` →
    /// `lots_of`): asking `lots()` directly with a big limit would pass against the
    /// unfixed code and prove nothing.
    #[tokio::test]
    async fn a_tender_scoped_lots_read_is_complete_and_still_honours_the_cursor() {
        use turso::Value;
        let path = format!("/tmp/tender-db-lots-bounded-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        // Seed the canonical layer directly, as the projection does (FK off).
        db.set_foreign_keys(false).await.unwrap();
        let w = db.conn().await;
        w.execute("INSERT INTO tenders (id, source, kind, created_at) VALUES (1,'ted','procedure',0)", ())
            .await
            .unwrap();
        w.execute(
            "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
             VALUES (1, 1, 1, 0, 'p')",
            (),
        )
        .await
        .unwrap();
        // More lots than MAX_PAGE, so a page-sized read would truncate.
        for i in 1..=1200i64 {
            w.execute(
                "INSERT INTO lots (id, tender_id, lot_key) VALUES (?, 1, ?)",
                (Value::Integer(i), Value::Text(format!("LOT-{i:05}"))),
            )
            .await
            .unwrap();
            w.execute(
                "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES (1, 1, ?, 'Lot')",
                (Value::Integer(i),),
            )
            .await
            .unwrap();
        }
        drop(w);
        let conn = db.reader().await.unwrap();

        let detail = read::tender_detail(&conn, 1, None).await.unwrap().expect("tender 1");
        assert_eq!(
            detail.lots.len(),
            1200,
            "the detail response must carry EVERY lot — it reported a count of 1200 while \
             shipping only MAX_PAGE of them"
        );

        // The cursor still selects strictly-later ids: post-115 the containment shape
        // serves `l.id > ?` from the Tender's own slice, so it stays in SQL.
        let f = read::Filter { tender: Some(1), ..read::Filter::default() };
        let after = read::lots(&conn, &f, read::Scope::Page { after: 1000, limit: 20_000 })
            .await
            .unwrap();
        assert_eq!(after.len(), 200, "after=1000 must skip the first 1000 lots");
        assert!(after.iter().all(|r| r.id > 1000), "no row at or before the cursor may be re-served");
        let _ = std::fs::remove_file(&path);
    }

    /// Which table a lots plan enters first — the only thing that separates the
    /// linear read from the quadratic one.
    ///
    /// Neither `SCAN`-vs-`SEARCH` nor the presence of `INTEGER PRIMARY KEY` can tell
    /// them apart, and both mislead in opposite directions. turso renders a full
    /// rowid traversal as `SEARCH l USING INTEGER PRIMARY KEY (rowid=?)`, which reads
    /// like a point lookup, so `!contains("SCAN")` passes for the DEFECTIVE shape.
    /// And the fixed shape contains that same string for a genuine one-row lookup of
    /// `l.id = vl.lot_id`, so asserting its absence rejects the CORRECT shape. The
    /// string is identical either way; only its position in the join order differs.
    #[derive(Debug, PartialEq)]
    enum Driver {
        VersionLots,
        Lots,
        Other,
    }

    fn driver(plan: &str) -> Driver {
        let upper = plan.to_uppercase();
        // Ignore lines belonging to the uncorrelated MAX(seq) subquery: it is
        // evaluated once, before the join, and names neither `l` nor `vl`.
        let order = |needle: &str| upper.find(needle).unwrap_or(usize::MAX);
        let vl = order("SEARCH VL ");
        let l = order("SEARCH L ").min(order("SCAN L "));
        match (vl, l) {
            (usize::MAX, usize::MAX) => Driver::Other,
            (vl, l) if vl < l => Driver::VersionLots,
            _ => Driver::Lots,
        }
    }

    /// Every `Filter` field must be explicitly classified as walk-capable or not.
    ///
    /// `read::walks` destructures `Filter` field by field, so adding a field is a
    /// compile error — but the compiler suggests `..` to ignore it, and taking that
    /// suggestion routes the new filter to the fast pool silently. A compile error
    /// forces a decision, not a correct one.
    ///
    /// So the field names are recovered from `Filter`'s own `Debug` output rather than
    /// written down twice, and checked against the classification list. A field added
    /// with `..` compiles and then fails HERE. If the derive's format ever changes this
    /// test breaks loudly rather than passing vacuously, which is the right direction
    /// for a check whose whole job is noticing an omission.
    #[test]
    fn filter_classification_is_exhaustive() {
        let debug = format!("{:?}", read::Filter::default());
        let body = debug
            .split_once('{')
            .and_then(|(_, rest)| rest.rsplit_once('}'))
            .map(|(inner, _)| inner.to_owned())
            .expect("derived Debug renders as `Filter { field: value, .. }`");
        let fields: Vec<String> = body
            .split(',')
            .filter_map(|part| part.split_once(':'))
            .map(|(name, _)| name.trim().to_owned())
            .filter(|name| !name.is_empty())
            .collect();
        assert!(
            fields.len() >= 5,
            "recovered too few fields from Debug ({fields:?}) — the derive's format \
             has probably changed and this check has stopped checking"
        );

        for field in &fields {
            assert!(
                read::FILTER_CLASSIFICATION.iter().any(|(name, _)| name == field),
                "Filter::{field} is not classified in read::FILTER_CLASSIFICATION. \
                 Every field must be recorded as index-served or walk-capable, because \
                 `walks` decides from it whether a request may starve the main reader \
                 pool (issue 120). If it compiled, the destructuring was bypassed with \
                 `..` — classify it rather than ignoring it."
            );
        }
        for (name, _) in read::FILTER_CLASSIFICATION {
            assert!(
                fields.iter().any(|f| f == name),
                "read::FILTER_CLASSIFICATION lists `{name}`, which is no longer a \
                 Filter field — a stale entry hides the absence of a real one"
            );
        }
    }

    /// The short-circuit guard's BOUNDARY, asserted on the function itself.
    ///
    /// The end-to-end cases in `tenders_shortcircuit.rs` cannot establish this: the
    /// guard changes SPEED and never RESULTS, so a guarded and a declined query return
    /// the same rows and no assertion on the answer can tell which path ran. Asserting
    /// the result and calling it verification of the boundary would be a check that
    /// cannot fail for the reason it claims to test.
    ///
    /// What actually decides the size of issue 117's remaining Class B hole is whether
    /// the decline counts CHARACTERS or ASCII LETTERS. Digits do not case-fold, so they
    /// do not branch: a NUTS-shaped prefix (two letters then digits) is guarded at any
    /// length, and only 5-or-more LETTERS declines — which is not a NUTS shape at all,
    /// so the hole is adversarial-only rather than reachable by ordinary use.
    #[test]
    fn the_guard_declines_on_letter_count_not_length() {
        use read::prefix_ranges_for_test as ranges;

        // Two letters -> 4 variants, whatever follows them.
        assert_eq!(ranges("DE").map(|r| r.len()), Some(4));
        assert_eq!(ranges("DE300").map(|r| r.len()), Some(4), "digits do not branch");
        assert_eq!(ranges("ZZ999").map(|r| r.len()), Some(4), "a 5-CHAR prefix is still guarded");
        assert_eq!(ranges("45210000").map(|r| r.len()), Some(1), "an all-digit CPV prefix: one range");

        // Four letters is the last guarded width; five declines.
        assert_eq!(ranges("ABCD").map(|r| r.len()), Some(16), "16 variants is the cap, inclusive");
        assert_eq!(ranges("ABCDE"), None, "5 LETTERS = 32 variants, past the cap");
        assert_eq!(ranges("DE30A").map(|r| r.len()), Some(8), "3 letters among digits -> 8");

        // The declines that exist for correctness rather than cost.
        assert_eq!(ranges(""), None, "empty binds LIKE '%' — matches everything");
        assert_eq!(ranges("%"), None, "LIKE metacharacter");
        assert_eq!(ranges("_E"), None, "LIKE metacharacter");
        assert_eq!(ranges("d\\"), None, "backslash, declined so a later ESCAPE cannot break it");
        assert_eq!(ranges("Ä"), None, "non-ASCII: LIKE does not fold it, and we do not guess");

        // The union really is the case-variant set, not just the right count.
        let mut got: Vec<String> = ranges("de").unwrap().into_iter().map(|(lo, _)| lo).collect();
        got.sort();
        assert_eq!(got, ["DE", "De", "dE", "de"]);
    }

    /// The negative control for the test above, and the reason a green there is
    /// attributable to the fix rather than to the statement having been reworded.
    ///
    /// It plans the PRE-115 shape — `lots` in the outer loop, `tender_version_lots`
    /// probed per row — through the SAME discriminator, and requires it to still come
    /// out `Driver::Lots`. That is what makes the pair meaningful: one statement, one
    /// measure, opposite verdicts. Without it, the test above could go green merely
    /// because the query was reworded.
    ///
    /// If this ever fails, turso has learned to reorder that join itself, and the
    /// containment/stream split in `read::lots` may have stopped earning its keep.
    #[tokio::test]
    async fn the_pre_115_lots_shape_still_plans_as_the_walk_we_left() {
        let path = format!("/tmp/tender-db-lots-eqp-ctl-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let conn = db.reader().await.unwrap();
        // Verbatim pre-115 identity half: driven from `lots`, vl probed per row.
        let mut rows = conn
            .query(
                "EXPLAIN QUERY PLAN
                 SELECT l.id, vl.kind, v.seq
                   FROM lots l
                   JOIN tenders t ON t.id = l.tender_id
                   JOIN tender_versions v ON v.tender_id = t.id
                    AND v.seq = (SELECT MAX(x.seq) FROM tender_versions x
                                  WHERE x.tender_id = t.id)
                   JOIN tender_version_lots vl
                     ON vl.tender_id = t.id AND vl.seq = v.seq AND vl.lot_id = l.id
                  WHERE 1 = 1 AND l.tender_id = ? AND l.id > ?
                  ORDER BY l.id LIMIT 1000",
                (Value::Integer(1), Value::Integer(0)),
            )
            .await
            .unwrap();
        let mut plan = String::new();
        while let Some(row) = rows.next().await.unwrap() {
            plan.push_str(&text(&row, 3));
            plan.push('\n');
        }
        assert_eq!(
            driver(&plan),
            Driver::Lots,
            "the pre-115 shape no longer drives from `lots` — the control has stopped \
             controlling, so re-check whether read::lots still needs its \
             containment/stream split. Plan was:\n{plan}"
        );
        let _ = std::fs::remove_file(&path);
    }

    /// Issue 37: the resolution-ledger counts must seek the `quarantine_reason`
    /// index — `WHERE reason = ?` narrows to one (usually small) bucket before the
    /// `detail LIKE` filter runs, instead of scanning the whole ~1.2M-row table on
    /// the background refresher. Asserting the plan proves the audit at any scale.
    #[tokio::test]
    async fn quarantine_resolution_seeks_the_reason_index() {
        let path = format!("/tmp/tender-db-qres-eqp-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let conn = db.reader().await.unwrap();
        let mut rows = conn
            .query(
                "EXPLAIN QUERY PLAN
                 SELECT SUM(CASE WHEN reprocessed_at IS NOT NULL THEN 1 ELSE 0 END),
                        SUM(CASE WHEN reprocessed_at IS NULL     THEN 1 ELSE 0 END)
                   FROM quarantine
                  WHERE reason = ?
                    AND (? IS NULL OR profile = ?)
                    AND (? IS NULL OR detail LIKE ?)
                    AND (? IS NULL OR member_path LIKE ?)
                    AND (? IS NULL OR member_path NOT LIKE ?)",
                (
                    "unclaimed-content".to_owned(),
                    opt_text(Some("text")),
                    opt_text(Some("text")),
                    opt_text(Some("%scalar field RP")),
                    opt_text(Some("%scalar field RP")),
                    opt_text(Some("%.__")),
                    opt_text(Some("%.__")),
                    opt_text(Some("%.en")),
                    opt_text(Some("%.en")),
                ),
            )
            .await
            .unwrap();
        let mut plan = String::new();
        while let Some(row) = rows.next().await.unwrap() {
            plan.push_str(&text(&row, 3));
            plan.push('\n');
        }
        assert!(
            plan.contains("quarantine_reason"),
            "resolution must seek the reason index, not scan the whole table — plan was:\n{plan}"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 30: the field-code breakdown groups by the code, not the whole
    /// The reason breakdown splits into still-held / reclaimed / skipped, and the
    /// three are disjoint and total (issue 137 / #29 criterion 6).
    ///
    /// The old count returned every row a reason EVER held, which read as a live
    /// gap long after the gap was closed — 1,734,594 of 2,419,410 rows were
    /// already resolved while the dashboard presented the whole figure as
    /// quarantined. The property that makes the split safe to show a user is
    /// that nothing disappears in it: the three buckets must sum back to the
    /// all-time count for every reason.
    #[tokio::test]
    async fn the_reason_breakdown_splits_held_from_reclaimed_and_skipped() {
        let path = format!("/tmp/tender-db-qsplit-{}.db", std::process::id());
        for x in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{x}"));
        }
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();
        {
            let conn = db.conn().await;
            conn.execute_batch(
                "INSERT INTO quarantine(fetch_id, member_path, content_hash, reason, detail, first_seen,
                                        reprocessed_at, skipped_at) VALUES
                   (1,'a1','h1','unparsable-xml','XML with DTD detected',0, NULL, NULL),
                   (1,'a2','h2','unparsable-xml','XML with DTD detected',0, 100,  NULL),
                   (1,'a3','h3','unparsable-xml','XML with DTD detected',0, NULL, 200),
                   (1,'a4','h4','unparsable-xml','XML with DTD detected',0, NULL, 200),
                   -- a row carrying BOTH must count once, as reclaimed: the
                   -- stronger claim, so a bookkeeping slip shows up as an
                   -- over-count rather than hiding rows in the quieter bucket.
                   (1,'a5','h5','unparsable-xml','XML with DTD detected',0, 300,  300),
                   -- a wholly resolved reason must report 0 still-held, not its
                   -- historical size: this is the unknown-field-code case.
                   (1,'b1','h6','unknown-field-code','line 20: OC',0, 400, NULL);",
            )
            .await
            .unwrap();
        }

        let split = db.quarantine_counts_by_reason_split().await.unwrap();
        let of = |name: &str| {
            split.iter().find(|(r, ..)| r == name).map(|(_, o, r, s)| (*o, *r, *s)).unwrap()
        };

        assert_eq!(of("unparsable-xml"), (1, 2, 2), "held / reclaimed / skipped");
        assert_eq!(
            of("unknown-field-code"),
            (0, 1, 0),
            "a fully reclaimed reason reports zero still-held, not its historical size"
        );

        // Disjoint AND total: nothing is double-counted and nothing vanishes.
        for (reason, held, reclaimed, skipped) in &split {
            let all_time = {
                let conn = db.conn().await;
                let mut rows = conn
                    .query(
                        "SELECT COUNT(*) FROM quarantine WHERE reason = ?1",
                        turso::params::Params::Positional(vec![Value::Text(reason.clone())]),
                    )
                    .await
                    .unwrap();
                int(&rows.next().await.unwrap().unwrap(), 0)
            };
            assert_eq!(
                held + reclaimed + skipped,
                all_time,
                "the split for `{reason}` does not sum to its all-time count — rows are \
                 being double-counted or dropped, and a user-facing total that does not \
                 add up is worse than the overstated one it replaced"
            );
        }

        let _ = std::fs::remove_file(&path);
    }

    /// `line <n>: <code>` detail, so one code across different line numbers sums
    /// into a single row — and only the unknown-field-code bucket is counted,
    /// over STILL-HELD rows only (issue 185): a reclaimed or skipped row is a
    /// historical record, not a gap.
    #[tokio::test]
    async fn field_code_gaps_group_by_code_across_line_numbers() {
        let path = format!("/tmp/tender-db-fcgaps-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();
        {
            let conn = db.conn().await;
            conn.execute_batch(
                "INSERT INTO quarantine(fetch_id, member_path, content_hash, reason, detail, first_seen) VALUES
                   (1,'m1','h1','unknown-field-code','line 20: OC',0),
                   (1,'m2','h2','unknown-field-code','line 25: OC',0),
                   (1,'m3','h3','unknown-field-code','line 9: XY',0),
                   (1,'m4','h4','unclaimed-content','line 5: whatever',0);
                 -- reclaimed and skipped OC rows: history, not gaps (issue 185)
                 INSERT INTO quarantine(fetch_id, member_path, content_hash, reason, detail, first_seen, reprocessed_at) VALUES
                   (1,'m5','h5','unknown-field-code','line 3: OC',0,7);
                 INSERT INTO quarantine(fetch_id, member_path, content_hash, reason, detail, first_seen, skipped_at) VALUES
                   (1,'m6','h6','unknown-field-code','line 4: OC',0,7);",
            )
            .await
            .unwrap();
        }
        db.set_foreign_keys(true).await.unwrap();

        let gaps = db.quarantine_field_code_gaps(5).await.unwrap();
        assert_eq!(
            gaps,
            vec![("OC".to_owned(), 2), ("XY".to_owned(), 1)],
            "OC sums across its two line numbers and counts only still-held rows; \
             unclaimed-content and the reclaimed/skipped rows are excluded",
        );

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 84's marker. The failure to fear is **over-scope**: flagging a row
    /// whose English original is NOT present would record real data loss as a
    /// duplicate, which is worse than the overstated count it fixes. So the
    /// fixture plants one of each trap and asserts the marker declines all of them.
    #[tokio::test]
    async fn the_skipped_sibling_marker_flags_only_the_confirmed_set() {
        let path = format!("/tmp/tender-db-skipmark-{}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();
        {
            let conn = db.conn().await;
            conn.execute_batch(
                "INSERT INTO fetches(id, source, kind, period, url, sha256, bytes, fetched_at, path)
                   VALUES (1,'ted','monthly','2008-05','u','s',1,0,'p'),
                          (2,'ted','daily','2019-001','u','s',1,0,'p');
                 -- the English original, present and parsed
                 INSERT INTO notices(id, source, publication_id, content_hash, profile, fetch_id,
                                     member_path, ingested_at, parse_state)
                   VALUES (1,'ted','115165-2008','h','internal-ojs',1,'m',0,'parsed'),
                 -- an original that exists but did NOT parse: its siblings are NOT duplicates
                          (2,'ted','999001-2008','h2','internal-ojs',1,'m2',0,'quarantined');
                 INSERT INTO quarantine(fetch_id, member_path, content_hash, reason, detail, first_seen) VALUES
                   -- (a) the real thing: non-EN sibling of a parsed original
                   (1,'115165/opoce-input/115165_2008.fr','c1','unparsable-xml','XML with DTD detected',0),
                   (1,'115165/opoce-input/115165_2008.de','c2','unparsable-xml','XML with DTD detected',0),
                   -- (b) the ENGLISH member: never a duplicate of itself
                   (1,'115165/opoce-input/115165_2008.en','c3','unparsable-xml','XML with DTD detected',0),
                   -- (c) sibling whose original exists but is NOT parsed -> real gap
                   (1,'999001/opoce-input/999001_2008.fr','c4','unparsable-xml','XML with DTD detected',0),
                   -- (d) sibling whose original does not exist at all -> real gap
                   (1,'999999/opoce-input/999999_2008.es','c5','unparsable-xml','XML with DTD detected',0),
                   -- (e) right shape, WRONG ERA (a 2019 daily) -> out of scope
                   (2,'115165/opoce-input/115165_2008.it','c6','unparsable-xml','XML with DTD detected',0),
                   -- (f) right era, WRONG REASON -> out of scope
                   (1,'115165/opoce-input/115165_2008.nl','c7','unclaimed-content','something else',0);",
            )
            .await
            .unwrap();
        }
        db.set_foreign_keys(true).await.unwrap();

        // The dry-run count is the decision input, so it must equal the write.
        assert_eq!(db.count_skipped_siblings().await.unwrap(), 2, "only (a): the two real siblings");

        // And the SECOND dry-run number: rows in scope that the guard declines.
        // These are (c) and (d) — a sibling whose original did not parse, and one
        // with no original at all. On prod this is expected to be 0; a non-zero
        // answer is a data-loss FINDING, never a reason to widen the predicate.
        // Asserted as 2 here precisely so the number is known to COUNT something —
        // a gap count that could only ever report 0 would be the reassuring-but-
        // empty answer this pair exists to prevent.
        // Reported as TWO findings, not one number: the fixture plants exactly one
        // of each, and they are different investigations. (d) has no original at
        // all — a fetch/ingest gap. (c) has one we hold that did not parse — a parse
        // failure. Asserting the split (not just the sum) is what proves the
        // classification works rather than merely totalling.
        assert_eq!(
            db.count_skipped_sibling_gaps().await.unwrap(),
            (1, 1),
            "(no original at all, original held but unparsed)"
        );
        // Note the two numbers are INDEPENDENT — 2 markable AND 2 rejected, in the
        // same population. That is why the execute path must check BOTH: a marked
        // count matching expectation says nothing about whether data-loss rows sit
        // beside them, so an operator reading only the first number would proceed
        // past rows whose originals are not in the corpus.

        let marked = db.mark_skipped_siblings(1000, 999, "internal-ojs-non-english").await.unwrap();
        assert_eq!(marked, 2);

        let flagged: i64 = match db
            .scalar("SELECT COUNT(*) FROM quarantine WHERE skipped_at IS NOT NULL")
            .await
            .unwrap()
        {
            Some(turso::Value::Integer(n)) => n,
            other => panic!("count: {other:?}"),
        };
        assert_eq!(flagged, 2, "no row outside the confirmed set was touched");

        // Named individually, so a future predicate change that sweeps one of these
        // in fails here rather than in production.
        for (path, why) in [
            ("115165/opoce-input/115165_2008.en", "the English member is not its own duplicate"),
            ("999001/opoce-input/999001_2008.fr", "original exists but is not parsed — a real gap"),
            ("999999/opoce-input/999999_2008.es", "no original at all — a real gap"),
            ("115165/opoce-input/115165_2008.it", "wrong era"),
            ("115165/opoce-input/115165_2008.nl", "wrong reason"),
        ] {
            let sql = format!(
                "SELECT skipped_at FROM quarantine WHERE member_path = '{path}'"
            );
            assert!(
                matches!(db.scalar(&sql).await.unwrap(), None | Some(turso::Value::Null)),
                "must stay outstanding: {why}"
            );
        }

        // The scale guarantee: over ~593k rows the per-row sibling lookup must
        // SEEK the identity index. Checked as a plan, not timed — a laptop clock
        // cannot tell a seek from a scan.
        //
        // Honest about what this covers: it asserts TURSO's plan, and turso seeks
        // correctly with or without the unary `+`, so this assertion does NOT
        // discriminate that. It guards against a future turso regression. The `+`
        // is there for stock sqlite3, which prefers `notices_parse_state` — and
        // that is not assertable from here, because this test drives turso. The
        // engines disagree, so a green here is not a statement about sqlite3.
        let conn = db.reader().await.unwrap();
        let mut rows = conn
            .query(
                "EXPLAIN QUERY PLAN
                 SELECT 1 FROM notices n
                  WHERE n.source = 'ted' AND +n.parse_state = 'parsed'
                    AND n.publication_id = '115165-2008'",
                (),
            )
            .await
            .unwrap();
        let mut plan = String::new();
        while let Some(row) = rows.next().await.unwrap() {
            if let Ok(turso::Value::Text(detail)) = row.get_value(3) {
                plan.push_str(&detail);
                plan.push('\n');
            }
        }
        assert!(!plan.is_empty(), "no plan came back");
        assert!(
            !plan.contains("notices_parse_state") && !plan.contains("SCAN"),
            "the sibling lookup must seek the identity index, not parse_state: {plan}"
        );

        // After a full run the data-loss rows are STILL THERE and still reported:
        // marking never consumes them, so they cannot be quietly retired by a
        // successful pass. The pair (0 markable, 2 rejected) is what a completed
        // run over a population containing data loss looks like.
        assert_eq!(db.count_skipped_siblings().await.unwrap(), 0, "work list drained");
        assert_eq!(
            db.count_skipped_sibling_gaps().await.unwrap(),
            (1, 1),
            "the guard-rejected rows survive the run and keep being reported, still split"
        );

        // Idempotent: the work list is empty, so a re-run writes nothing.
        assert_eq!(db.count_skipped_siblings().await.unwrap(), 0);
        assert_eq!(db.mark_skipped_siblings(1000, 999, "internal-ojs-non-english").await.unwrap(), 0);

        // The three buckets are now distinct, and `skipped` is NOT counted as
        // reclaimed (which would claim notices entered that never did) nor as
        // outstanding (the overstatement being fixed).
        assert_eq!(
            db.quarantine_resolution("unparsable-xml", None, Some("XML with DTD detected"), None, None)
                .await
                .unwrap(),
            (0, 2, 4),
            "(reclaimed, skipped, outstanding)"
        );

        // Reversible, and scoped by the marker's own reason.
        assert_eq!(db.unmark_skipped_siblings("internal-ojs-non-english").await.unwrap(), 2);
        assert_eq!(db.count_skipped_siblings().await.unwrap(), 2, "back on the work list");

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// Issue 190: the REPROCESS-TIME flag pass carries issue 84's parsed-original
    /// guard (it swept the 154 protected siblings without it), and the repair
    /// restores exactly the guard-rejected rows a guard-free pass marked.
    #[tokio::test]
    async fn the_reprocess_flag_pass_is_guarded_and_the_repair_restores_swept_rows() {
        let path = format!("/tmp/tender-db-sweptfix-{}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();
        {
            let conn = db.conn().await;
            conn.execute_batch(
                "INSERT INTO fetches(id, source, kind, period, url, sha256, bytes, fetched_at, path)
                   VALUES (1,'ted','monthly','2008-05','u','s',1,0,'p');
                 INSERT INTO notices(id, source, publication_id, content_hash, profile, fetch_id,
                                     member_path, ingested_at, parse_state)
                   VALUES (1,'ted','115165-2008','h','internal-ojs',1,'m',0,'parsed'),
                          (2,'ted','999001-2008','h2','internal-ojs',1,'m2',0,'quarantined');
                 INSERT INTO quarantine(fetch_id, member_path, content_hash, reason, detail, first_seen) VALUES
                   -- sibling of a PARSED original: the guard accepts it
                   (1,'115165/opoce-input/115165_2008.fr','c1','unparsable-xml','XML with DTD detected',0),
                   -- sibling of an UNPARSED original: protected, must stay outstanding
                   (1,'999001/opoce-input/999001_2008.fr','c4','unparsable-xml','XML with DTD detected',0),
                   -- a text-era member declined by a policy the guard does not gate
                   -- (its own failure detail — it is not part of the sibling scope)
                   (1,'EN_19990601_104_ISO_ORG.zip','c8','unparsable-xml','unreadable member',0);",
            )
            .await
            .unwrap();
        }
        db.set_foreign_keys(true).await.unwrap();

        // The reprocess walk declines both siblings; the guard admits only the
        // one whose English original is parsed.
        let flagged = db
            .flag_skipped_members(
                1,
                &[
                    ("115165/opoce-input/115165_2008.fr".to_string(), "internal-ojs-non-english"),
                    ("999001/opoce-input/999001_2008.fr".to_string(), "internal-ojs-non-english"),
                ],
                999,
            )
            .await
            .unwrap();
        assert_eq!(flagged, 1, "the guard admits only the parsed-original sibling");
        assert!(
            matches!(
                db.scalar("SELECT skipped_at FROM quarantine WHERE member_path = '999001/opoce-input/999001_2008.fr'")
                    .await
                    .unwrap(),
                None | Some(turso::Value::Null)
            ),
            "the protected sibling stays outstanding"
        );

        // Non-sibling policies stay unguarded: their decline defers to the SAME
        // member's chosen twin, not a different document.
        let flagged = db
            .flag_skipped_members(1, &[("EN_19990601_104_ISO_ORG.zip".to_string(), "text-era-iso-superseded-by-utf8")], 999)
            .await
            .unwrap();
        assert_eq!(flagged, 1);

        // The repair: simulate the pre-guard sweep on the protected row, then
        // restore it — and ONLY it.
        db.conn()
            .await
            .execute(
                "UPDATE quarantine SET skipped_at = 5, skipped_reason = 'internal-ojs-non-english'
                  WHERE member_path = '999001/opoce-input/999001_2008.fr'",
                (),
            )
            .await
            .unwrap();
        assert_eq!(db.count_swept_siblings().await.unwrap(), 1, "the dry-run count sees the swept row");
        assert_eq!(db.repair_swept_siblings().await.unwrap(), 1);
        assert!(
            matches!(
                db.scalar("SELECT skipped_at FROM quarantine WHERE member_path = '999001/opoce-input/999001_2008.fr'")
                    .await
                    .unwrap(),
                None | Some(turso::Value::Null)
            ),
            "the swept row is outstanding again"
        );
        assert!(
            matches!(
                db.scalar("SELECT skipped_at FROM quarantine WHERE member_path = '115165/opoce-input/115165_2008.fr'")
                    .await
                    .unwrap(),
                Some(turso::Value::Integer(999))
            ),
            "the legitimately flagged sibling keeps its mark"
        );

        // Self-limiting: nothing left to repair.
        assert_eq!(db.repair_swept_siblings().await.unwrap(), 0);

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// Issue 40: a resolution-ledger key splits its matching quarantine rows into
    /// reclaimed, skipped and outstanding, and the profile + detail narrowers keep
    /// one ledger entry from counting a sibling bucket's rows.
    #[tokio::test]
    async fn quarantine_resolution_splits_reclaimed_from_outstanding() {
        let path = format!("/tmp/tender-db-qres-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();
        {
            let conn = db.conn().await;
            conn.execute_batch(
                "INSERT INTO quarantine(fetch_id, member_path, content_hash, profile, reason, detail, first_seen, reprocessed_at) VALUES
                   (1,'m1','h1','ted-export-r208','unclaimed-content','unclaimed attribute at /TED_EXPORT/.../PROCEDURE/@REASON',0,100),
                   (1,'m2','h2','ted-export-r208','unclaimed-content','unclaimed attribute at /TED_EXPORT/.../PROCEDURE/@REASON',0,NULL),
                   (1,'m3','h3','ted-export-r208','unclaimed-content','unclaimed attribute at /TED_EXPORT/.../OBJECT/@CATEGORY',0,NULL),
                   (1,'m4','h4','text','unclaimed-content','line 5: continuation under scalar field RP',0,100),
                   (1,'m5','h5','text','unclaimed-content','line 8: continuation under scalar field XY',0,NULL);
                 -- the DTD populations of issue 186, distinguishable only by path
                 -- shape: an .en original (reclaimed), a non-English sibling
                 -- (skipped), a sibling whose original never parsed (held), and a
                 -- 2010-03 non-sibling ending .xml (held).
                 INSERT INTO quarantine(fetch_id, member_path, content_hash, reason, detail, first_seen, reprocessed_at, skipped_at) VALUES
                   (1,'115165/opoce-input/115165_2008.en','d1','unparsable-xml','XML with DTD detected',0,100,NULL),
                   (1,'115165/opoce-input/115165_2008.fr','d2','unparsable-xml','XML with DTD detected',0,NULL,100),
                   (1,'999001/opoce-input/999001_2008.de','d3','unparsable-xml','XML with DTD detected',0,NULL,NULL),
                   (1,'201003/member-4711.xml','d4','unparsable-xml','XML with DTD detected',0,NULL,NULL);",
            )
            .await
            .unwrap();
        }
        db.set_foreign_keys(true).await.unwrap();

        // r208 @REASON: one reprocessed, one still held; the sibling @CATEGORY row
        // is excluded by the detail pattern.
        assert_eq!(
            db.quarantine_resolution("unclaimed-content", Some("ted-export-r208"), Some("%@REASON"), None, None)
                .await
                .unwrap(),
            (1, 0, 1),
        );
        // text RP: reclaimed, with the sibling XY continuation excluded.
        assert_eq!(
            db.quarantine_resolution("unclaimed-content", Some("text"), Some("%scalar field RP"), None, None)
                .await
                .unwrap(),
            (1, 0, 0),
        );
        // A key that matches nothing yet is simply (0, 0, 0), never an error.
        assert_eq!(
            db.quarantine_resolution("unknown-field-code", None, None, None, None).await.unwrap(),
            (0, 0, 0),
        );

        // Issue 186: the member-path narrowers split populations one (reason,
        // detail) key blends. Unkeyed, the DTD bucket shows everything at once…
        let dtd = Some("XML with DTD detected");
        assert_eq!(
            db.quarantine_resolution("unparsable-xml", None, dtd, None, None).await.unwrap(),
            (1, 1, 2),
            "the blended key: every population in one row"
        );
        // …the English originals key shows the reclaim story (and its 0 held)…
        assert_eq!(
            db.quarantine_resolution("unparsable-xml", None, dtd, Some("%.en"), None).await.unwrap(),
            (1, 0, 0),
        );
        // …the non-English siblings key shows the skip + the unparsed-original
        // holdout, with .en and .xml rows excluded…
        assert_eq!(
            db.quarantine_resolution("unparsable-xml", None, dtd, Some("%.__"), Some("%.en"))
                .await
                .unwrap(),
            (0, 1, 1),
        );
        // …and the 2010-03 non-siblings key isolates the .xml population.
        assert_eq!(
            db.quarantine_resolution("unparsable-xml", None, dtd, Some("%.xml"), None).await.unwrap(),
            (0, 0, 1),
        );

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 33: the pipeline's fetch and projection stages — distinct package
    /// periods with their range per source, and projected Tenders per source.
    #[tokio::test]
    async fn pipeline_stage_queries_summarise_per_source() {
        let path = format!("/tmp/tender-db-pipeline-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();
        {
            let conn = db.conn().await;
            conn.execute_batch(
                "INSERT INTO fetches(source, kind, period, url, sha256, bytes, fetched_at, path) VALUES
                   ('ted','monthly','1993-01','u','a',1,0,'p'),
                   ('ted','monthly','1993-01','u','b',1,1,'p'),
                   ('ted','monthly','2026-07','u','c',1,0,'p'),
                   ('doe','daily','2026-07-18','u','d',1,0,'p');
                 INSERT INTO tenders(source, kind, created_at) VALUES
                   ('ted','procedure',0),('ted','procedure',0),('doe','procedure',0);",
            )
            .await
            .unwrap();
        }
        db.set_foreign_keys(true).await.unwrap();

        let fetch = db.fetch_registry_summary().await.unwrap();
        assert_eq!(
            fetch,
            vec![
                ("doe".to_owned(), 1, "2026-07-18".to_owned(), "2026-07-18".to_owned()),
                // ted: two distinct periods (the 1993-01 re-fetch counts once), range 1993→2026.
                ("ted".to_owned(), 2, "1993-01".to_owned(), "2026-07".to_owned()),
            ],
        );
        assert_eq!(db.tenders_by_source().await.unwrap(), vec![("doe".to_owned(), 1), ("ted".to_owned(), 2)]);

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 32 deploy fix: `job_queue` shipped in bad8dda without `progress`, so
    /// opening a pre-32 database must ALTER the column in — otherwise recover()'s
    /// `SELECT … progress` crashes the new binary on boot. Opens a database with
    /// the old job_queue and a live job, then asserts the read path works and the
    /// column is writable.
    #[tokio::test]
    async fn migration_adds_the_job_queue_progress_column() {
        let path = format!("/tmp/tender-db-jqmigrate-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);

        // The pre-issue-32 schema: job_queue without `progress`, holding a job.
        let database = turso::Builder::new_local(&path).build().await.unwrap();
        let conn = database.connect().unwrap();
        conn.execute_batch(
            "CREATE TABLE job_queue (
                id INTEGER PRIMARY KEY, kind TEXT NOT NULL, params TEXT NOT NULL, spec TEXT NOT NULL
            ) STRICT;
             INSERT INTO job_queue(id, kind, params, spec) VALUES(3, 'process', 'ted (all)', 'x');",
        )
        .await
        .unwrap();
        drop(conn);
        drop(database);

        let db = Db::open(&path).await.expect("open must migrate the pre-32 job_queue");
        let pending = db.pending_jobs().await.expect("the migrated column is readable");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].progress, None, "an existing job gets a NULL cursor");
        // And it is writable — the resume cursor works on the migrated table.
        db.record_job_progress(3, "2008-06").await.unwrap();
        assert_eq!(db.pending_jobs().await.unwrap()[0].progress.as_deref(), Some("2008-06"));

        let _ = std::fs::remove_file(&path);
    }

    /// The `job_log.job_id` migration. The log shipped in issue 16 keyed only by
    /// its own append counter, so a prod database has a `job_log` without the
    /// column — and `recent_job_runs`' `SELECT … job_id` would crash the new
    /// binary on the first dashboard read. Opens a pre-column log holding a run,
    /// then asserts the read path works, the old row reads NULL, and a new row
    /// carries and reports its Supervisor id.
    #[tokio::test]
    async fn migration_adds_the_job_log_job_id_column() {
        let path = format!("/tmp/tender-db-joblogmigrate-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);

        // The pre-column schema, holding one finished run.
        let database = turso::Builder::new_local(&path).build().await.unwrap();
        let conn = database.connect().unwrap();
        conn.execute_batch(
            "CREATE TABLE job_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT, kind TEXT NOT NULL, params TEXT NOT NULL,
                started_at INTEGER NOT NULL, finished_at INTEGER NOT NULL,
                outcome TEXT NOT NULL, counts_json TEXT NOT NULL
             ) STRICT;
             INSERT INTO job_log(id, kind, params, started_at, finished_at, outcome, counts_json)
             VALUES(919, 'reparse', 'reparse text', 10, 20, 'ok', '12385 notices');",
        )
        .await
        .unwrap();
        drop(conn);
        drop(database);

        let db = Db::open(&path).await.expect("open must migrate the pre-job_id job_log");
        let runs = db.recent_job_runs(10).await.expect("the migrated column is readable");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, 919, "the log's own counter is untouched");
        assert_eq!(runs[0].job_id, None, "a row written before the column has no Supervisor id");
        // A NULL-only log offers recovery no floor, which is the honest answer —
        // those rows were numbered under the old scheme.
        assert_eq!(db.max_logged_job_id().await.unwrap(), None);

        // And the column is writable: a new run carries its Supervisor id, which
        // is a different number from the log's append counter.
        db.record_job_run(7, "project", "rebuild=false", 30, 40, "ok", "0 tenders").await.unwrap();
        let runs = db.recent_job_runs(10).await.unwrap();
        assert_eq!(runs[0].job_id, Some(7));
        assert_ne!(runs[0].id, 7, "the two ids are independent namespaces");
        assert_eq!(db.max_logged_job_id().await.unwrap(), Some(7), "now recovery has a floor");

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 255: the decision-date columns arrive on a `tender_version_contracts`
    /// that already holds rows, and an existing row must read NULL rather than
    /// inheriting the conclusion date — "we did not record it" and "it is the same
    /// day as the signature" are different claims.
    #[tokio::test]
    async fn migration_adds_the_contract_decision_date_columns() {
        let path = format!("/tmp/tender-db-decidedmigrate-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);

        // The pre-column schema, holding one folded contract row.
        let database = turso::Builder::new_local(&path).build().await.unwrap();
        let conn = database.connect().unwrap();
        conn.execute_batch(
            "CREATE TABLE tender_version_contracts (
                tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, contract_id INTEGER NOT NULL,
                buyer_contract_id TEXT, concluded_utc INTEGER, concluded_offset INTEGER,
                concluded_has_time INTEGER, cents INTEGER, currency TEXT,
                PRIMARY KEY (tender_id, seq, contract_id)
             ) STRICT;
             INSERT INTO tender_version_contracts(tender_id, seq, contract_id, buyer_contract_id,
                        concluded_utc, concluded_offset, concluded_has_time, cents, currency)
             VALUES(5, 1, 9, 'BC-1', 1679608800, 120, 0, 100, 'EUR');",
        )
        .await
        .unwrap();
        drop(conn);
        drop(database);

        let db = Db::open(&path).await.expect("open must migrate the pre-decided contracts table");
        let conn = db.conn().await;
        let mut rows = conn
            .query(
                "SELECT decided_utc, decided_offset, decided_has_time, concluded_utc
                   FROM tender_version_contracts WHERE tender_id = 5",
                (),
            )
            .await
            .expect("the migrated columns are readable");
        let row = rows.next().await.unwrap().expect("the pre-existing row survives");
        assert!(
            matches!(row.get_value(0).unwrap(), turso::Value::Null),
            "a row folded before the column knows no decision date"
        );
        assert!(matches!(row.get_value(1).unwrap(), turso::Value::Null));
        assert!(matches!(row.get_value(2).unwrap(), turso::Value::Null));
        assert_eq!(
            row.get_value(3).unwrap(),
            turso::Value::Integer(1_679_608_800),
            "and its conclusion date is untouched"
        );

        let _ = std::fs::remove_file(&path);
    }

    // ---------------------------------------------------------- reclaim (issue 72/73)

    async fn seed_fetch(db: &Db) {
        db.conn()
            .await
            .execute(
                "INSERT INTO fetches(id, source, kind, period, url, sha256, bytes, fetched_at, path)
                 VALUES(1,'ted','daily','2001-1','u','h',1,0,'pkg')",
                (),
            )
            .await
            .unwrap();
    }

    fn tiny_parsed() -> Parsed {
        Parsed {
            sections: vec![Section { id: "PROCEDURE".into(), kind: "Notice".into(), parent: None }],
            values: vec![ValueRow {
                section_id: "PROCEDURE".into(),
                field_id: "TITLE".into(),
                ordinal: 0,
                value: NoticeValue::Text { lang: Some("EN".into()), value: "hello".into() },
            }],
        }
    }

    /// Issue 230: a report round-trips, and a re-run REPLACES rather than
    /// accumulating — the latest measurement is the only one worth keeping.
    #[tokio::test]
    async fn a_report_round_trips_and_the_newest_run_wins() {
        let path = format!("/tmp/tender-db-reports-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();

        assert!(db.latest_report("data-quality").await.unwrap().is_none(), "nothing measured yet");

        db.put_report("data-quality", "first body", 1_000).await.unwrap();
        assert_eq!(
            db.latest_report("data-quality").await.unwrap(),
            Some(("first body".to_owned(), 1_000))
        );

        db.put_report("data-quality", "second body", 2_000).await.unwrap();
        assert_eq!(
            db.latest_report("data-quality").await.unwrap(),
            Some(("second body".to_owned(), 2_000)),
            "the newest run replaces the previous body and stamp"
        );
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM reports").await, Some(1), "one row per kind");

        // A different kind is independent.
        db.put_report("other", "x", 3_000).await.unwrap();
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM reports").await, Some(2));
        assert_eq!(
            db.latest_report("data-quality").await.unwrap().map(|(b, _)| b),
            Some("second body".to_owned()),
            "kinds do not overwrite each other"
        );

        // measure_rows runs an aggregate on the reader pool and shapes it as rows.
        let rows = db.measure_rows("SELECT COUNT(*), 'lit' FROM reports").await.unwrap();
        assert_eq!(rows.len(), 1);
        assert!(matches!(rows[0][0], Value::Integer(2)), "{:?}", rows[0][0]);
        assert!(matches!(&rows[0][1], Value::Text(s) if s == "lit"), "{:?}", rows[0][1]);

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 100: a re-parse REPLACES the parsed layer and re-opens the notice for
    /// folding. The hazard it guards is duplication, not absence — `insert_parsed`
    /// is pure INSERT, so a re-parse that forgot to clear would double every row
    /// Issue 248: a section the new parse RE-CREATES keeps its mention, and that is what
    /// makes an era-scale re-parse possible at all.
    ///
    /// Deleting one mention costs ~2.2 s on prod — proving that none of
    /// `tender_version_parties`' 78M rows references it is not index-served on the write
    /// path — so a 3.79M-notice era spent 2,300 hours on foreign-key proving. The text
    /// era's parse re-creates every section id it had (`PROCEDURE`, `ORG-1`) and merely
    /// ADDS the award sections, so with this the proof never runs.
    #[tokio::test]
    async fn a_reparse_keeps_the_mentions_of_sections_it_recreates() {
        let path = format!("/tmp/tender-db-reparse-keep-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;
        let notice = held_notice();
        assert!(db.record_notice(&notice, &Parse::Parsed(tiny_parsed())).await.unwrap());
        let id = int_of(&db, "SELECT id FROM notices").await.expect("the notice row");

        // A mention on `PROCEDURE`, the section `tiny_parsed` declares — and the section
        // the re-parse below declares again.
        let mut resolver = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
        db.resolve_mentions(
            &mut resolver,
            &[Mention {
                notice_id: id,
                section_id: "PROCEDURE".into(),
                name: "Behoerde".into(),
                country: Some("DE".into()),
                raw_identifier: None,
                scheme: None,
                identifier: None,
                variants: Vec::new(),
            }],
            100,
        )
        .await
        .unwrap();
        db.finish_mention_resolver(resolver).await.unwrap();
        let org = int_of(&db, "SELECT organization_id FROM organization_mentions").await;
        assert!(org.is_some(), "the mention resolved to an organization");

        // The new parse keeps `PROCEDURE` and adds an award section beside it — the exact
        // shape issue 244's text-era extraction produces.
        let reparsed = Parsed {
            sections: vec![
                Section { id: "PROCEDURE".into(), kind: "Notice".into(), parent: None },
                Section {
                    id: "RES-1".into(),
                    kind: "LotResult".into(),
                    parent: Some("PROCEDURE".into()),
                },
            ],
            values: vec![ValueRow {
                section_id: "PROCEDURE".into(),
                field_id: "TITLE".into(),
                ordinal: 0,
                value: NoticeValue::Text { lang: None, value: "rewritten".into() },
            }],
        };
        assert!(db.reparse_notice(&notice, &reparsed).await.unwrap(), "the notice exists");

        // The mention SURVIVED, still pointing at the same organization: no delete, so no
        // foreign-key proof, so no 2.2 seconds.
        assert_eq!(
            int_of(&db, "SELECT COUNT(*) FROM organization_mentions").await,
            Some(1),
            "a re-created section keeps its mention"
        );
        assert_eq!(int_of(&db, "SELECT organization_id FROM organization_mentions").await, org);

        // And the parse layer is still REPLACED, not appended: the kept section is upserted
        // (its kind refreshed), the new one added, and the values are the new ones only.
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM notice_sections").await, Some(2));
        assert_eq!(
            text_of(&db, "SELECT kind FROM notice_sections WHERE section_id = 'PROCEDURE'")
                .await
                .as_deref(),
            Some("Notice"),
            "the kept section's kind is refreshed by the upsert"
        );
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM notice_texts").await, Some(1));
        assert_eq!(
            text_of(&db, "SELECT value FROM notice_texts").await.as_deref(),
            Some("rewritten")
        );

        let _ = std::fs::remove_file(&path);
    }

    /// and still look like it worked.
    #[tokio::test]
    async fn a_reparse_replaces_the_parsed_layer_and_reopens_the_fold() {
        let path = format!("/tmp/tender-db-reparse-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;
        let notice = held_notice();
        assert!(db.record_notice(&notice, &Parse::Parsed(tiny_parsed())).await.unwrap());
        let id = int_of(&db, "SELECT id FROM notices").await.expect("the notice row");
        db.mark_projected(&[id]).await.unwrap();
        assert!(db.unprojected_parsed_notice_ids().await.unwrap().is_empty(), "starts folded");

        // A DIFFERENT parse of the same notice — the shape a parser fix produces:
        // the section keeps its identity, the value changes.
        let reparsed = Parsed {
            sections: vec![Section { id: "RES-0001".into(), kind: "LotResult".into(), parent: None }],
            values: vec![ValueRow {
                section_id: "RES-0001".into(),
                field_id: "TITLE".into(),
                ordinal: 0,
                value: NoticeValue::Text { lang: Some("EN".into()), value: "rewritten".into() },
            }],
        };
        // A mention on the OLD section id — the shape prod actually had, and the
        // FK that made the first real re-parse run fail. The projection writes
        // these, so they reference sections the re-parse is about to delete.
        let mut resolver = db.mention_resolver(None, None, None, None, None, None, 0).await.unwrap();
        let mentions = vec![Mention {
            notice_id: id,
            section_id: "PROCEDURE".into(),
            name: "Alte Behoerde".into(),
            country: Some("DE".into()),
            raw_identifier: None,
            scheme: None,
            identifier: None,
            variants: Vec::new(),
        }];
        db.resolve_mentions(&mut resolver, &mentions, 100).await.unwrap();
        db.finish_mention_resolver(resolver).await.unwrap();
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM organization_mentions").await, Some(1));

        assert!(db.reparse_notice(&notice, &reparsed).await.unwrap(), "the notice exists");

        // The stale mention is gone with the section it named — it could not
        // survive (its FK points at a deleted section) and the following fold
        // re-derives it against the new ids.
        assert_eq!(
            int_of(&db, "SELECT COUNT(*) FROM organization_mentions").await,
            Some(0),
            "a re-parse clears the notice's mentions; Phase 1 re-records them"
        );

        // Replaced, not appended — one section and one text row, carrying the NEW
        // content. A missing clear would show 2 of each here.
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM notice_sections").await, Some(1));
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM notice_texts").await, Some(1));
        assert_eq!(
            text_of(&db, "SELECT section_id FROM notice_sections").await.as_deref(),
            Some("RES-0001"),
            "the new section id is what the references will resolve against"
        );
        assert_eq!(text_of(&db, "SELECT value FROM notice_texts").await.as_deref(), Some("rewritten"));

        // And it is folding work again: without this the new rows would sit unread
        // behind the projected watermark.
        assert_eq!(
            db.unprojected_parsed_notice_ids().await.unwrap(),
            vec![id],
            "a re-parsed notice re-enters the incremental change-set"
        );

        // Issue 247: re-parse AGAIN, now that the notice has no mentions. This is the
        // path the mention delete is skipped on — the common one in a bulk campaign, and
        // the one whose unconditional DELETE cost 153 ms a notice scanning 41.78M rows —
        // so it has to reach the same end state as the first pass rather than quietly
        // leaving the parse layer half-replaced.
        assert!(db.reparse_notice(&notice, &reparsed).await.unwrap(), "a second re-parse works");
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM organization_mentions").await, Some(0));
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM notice_sections").await, Some(1));
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM notice_texts").await, Some(1));
        assert_eq!(
            text_of(&db, "SELECT parse_state FROM notices").await.as_deref(),
            Some("parsed"),
            "it stays parsed throughout — a re-parse must never open a window where \
             the notice is absent from the corpus"
        );

        // An unknown notice is reported, not an error: a cohort walk can meet a
        // member that was never ingested.
        let stranger = Notice { publication_id: "999-2099".into(), ..held_notice() };
        assert!(!db.reparse_notice(&stranger, &tiny_parsed()).await.unwrap());

        let _ = std::fs::remove_file(&path);
    }

    /// The clear must cover every table the parsed layer occupies. This reads the
    /// SCHEMA rather than trusting the constant, so adding a tenth `notice_*` table
    /// to `insert_parsed` without adding it to `PARSED_TABLES` fails here instead
    /// of silently orphaning rows on every future re-parse.
    #[tokio::test]
    async fn a_reparse_clears_every_parsed_table() {
        let path = format!("/tmp/tender-db-reparse-tables-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let conn = db.conn().await;
        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name LIKE 'notice\\_%' ESCAPE '\\'",
                (),
            )
            .await
            .unwrap();
        let mut schema = Vec::new();
        while let Some(row) = rows.next().await.unwrap() {
            schema.push(text(&row, 0));
        }
        drop(rows);
        drop(conn);
        schema.sort();
        let mut covered: Vec<String> = Db::PARSED_TABLES.iter().map(|t| t.to_string()).collect();
        covered.sort();
        assert_eq!(
            schema, covered,
            "PARSED_TABLES must equal the notice_* tables in the schema — a table in one list \
             and not the other is either an orphan-row leak (missing from the clear) or a \
             DELETE against nothing"
        );

        let _ = std::fs::remove_file(&path);
    }

    fn held_notice() -> Notice {
        Notice {
            source: "ted".into(),
            publication_id: "123-2001".into(),
            content_hash: "hash1".into(),
            profile: "text".into(),
            declared_version: None,
            fetch_id: 1,
            member_path: "pkg/m1".into(),
            ingested_at: 100,
            published_at: None,
            dispatched_at: None,
        }
    }

    /// Issue 305: the incremental pre-check's cheap upper bound counts exactly
    /// the LEGACY profiles (text / internal-ojs / ted-export*) still in the
    /// un-projected set — the SQL predicate must mirror ingest's
    /// `is_legacy_profile`.
    #[tokio::test]
    async fn unprojected_legacy_count_matches_the_profile_predicate() {
        let path = format!("/tmp/tender-db-legacy-count-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;
        let mut n = held_notice();
        let profiles =
            ["text", "internal-ojs", "ted-export-r208", "ted-export-r209", "eforms:eforms-sdk-1.13", "doe"];
        for (i, profile) in profiles.iter().enumerate() {
            n.publication_id = format!("p{i}");
            n.content_hash = format!("h{i}");
            n.profile = (*profile).into();
            assert!(db.record_notice(&n, &Parse::Parsed(tiny_parsed())).await.unwrap());
        }
        assert_eq!(db.unprojected_legacy_notice_count().await.unwrap(), 4);
        // Projecting a legacy notice removes it from the pre-check's count.
        let id = int_of(&db, "SELECT id FROM notices WHERE profile = 'text'").await.unwrap();
        db.mark_projected(&[id]).await.unwrap();
        assert_eq!(db.unprojected_legacy_notice_count().await.unwrap(), 3);
        let _ = std::fs::remove_file(&path);
    }

    async fn int_of(db: &Db, sql: &str) -> Option<i64> {
        match db.scalar(sql).await.unwrap() {
            Some(Value::Integer(i)) => Some(i),
            _ => None,
        }
    }
    async fn text_of(db: &Db, sql: &str) -> Option<String> {
        match db.scalar(sql).await.unwrap() {
            Some(Value::Text(s)) => Some(s),
            _ => None,
        }
    }

    /// Issue 85: a cohort the projection MIS-READ is re-queued for the incremental
    /// fold by clearing its `projected` watermark alone — the parsed layer is already
    /// correct, so nothing else may move. Asserts the scope is exactly the named
    /// profiles (siblings keep their watermark and their values), the returned count
    /// is the number actually re-queued, and the cohort lands in the change-set.
    #[tokio::test]
    async fn unmark_projected_re_queues_only_the_named_profile_cohort() {
        let path = format!("/tmp/tender-db-unmark-cohort-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;

        // Three profiles: two in the cohort, one sibling that must not be touched.
        for (i, profile) in
            ["eforms:eforms-de-1.1", "eforms:eforms-de-1.2", "eforms:eforms-sdk-1.7"].iter().enumerate()
        {
            let notice = Notice {
                publication_id: format!("{i}-2026"),
                content_hash: format!("hash-{i}"),
                profile: (*profile).into(),
                member_path: format!("pkg/m{i}"),
                ..held_notice()
            };
            assert!(db.record_notice(&notice, &Parse::Parsed(tiny_parsed())).await.unwrap());
        }
        // Fold them: every notice is projected, so the change-set is empty.
        db.mark_projected(&[1, 2, 3]).await.unwrap();
        assert!(db.unprojected_parsed_notice_ids().await.unwrap().is_empty(), "all three start folded");

        // The guard's pre-count sees exactly the cohort, never the sibling.
        let cohort = ["eforms:eforms-de-1.1", "eforms:eforms-de-1.2"];
        assert_eq!(db.projected_notice_count_for_profiles(&cohort).await.unwrap(), 2);
        assert_eq!(
            db.projected_notice_count_for_profiles(&["eforms:eforms-de-9.9"]).await.unwrap(),
            0,
            "an unmatched profile counts zero — the mistyped-string case the guard catches"
        );

        // Re-queue: the two DE notices come back, the sdk-1.7 sibling does not.
        assert_eq!(db.unmark_projected_for_profiles(&cohort).await.unwrap(), 2, "returns what it re-queued");
        assert_eq!(db.unprojected_parsed_notice_ids().await.unwrap(), vec![1, 2]);
        assert_eq!(int_of(&db, "SELECT projected FROM notices WHERE id=3").await, Some(1), "sibling untouched");

        // Byte-safe: clearing the watermark re-derives, it never edits the parse layer.
        assert_eq!(
            text_of(&db, "SELECT parse_state FROM notices WHERE id=1").await.as_deref(),
            Some("parsed"),
            "the cohort stays parsed — a re-fold is projection-only, never a re-parse"
        );
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM notice_texts WHERE notice_id=1").await, Some(1));

        // Idempotent: nothing left projected in the cohort, so a second pass is a no-op.
        assert_eq!(db.unmark_projected_for_profiles(&cohort).await.unwrap(), 0);
        assert_eq!(db.unmark_projected_for_profiles(&[]).await.unwrap(), 0, "an empty profile list is a no-op");

        let _ = std::fs::remove_file(&path);
    }

    /// A parse-level held notice (the OC/SDK class: a `notices` row exists in
    /// state `quarantined`, empty of parsed values) is written IN PLACE when it
    /// now parses — the case a plain `process` re-run can never reach.
    #[tokio::test]
    async fn reclaim_writes_a_held_parse_level_notice_in_place() {
        let path = format!("/tmp/tender-db-reclaim-parse-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;

        // Ingest quarantined: a notice row (state quarantined) + a held quarantine
        // row keyed to it, no parsed values.
        assert!(db
            .record_notice(
                &held_notice(),
                &Parse::Quarantined { reason: "unknown-field-code".into(), detail: Some("line 3: OC".into()) },
            )
            .await
            .unwrap());
        assert_eq!(text_of(&db, "SELECT parse_state FROM notices WHERE id=1").await.as_deref(), Some("quarantined"));
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM notice_texts WHERE notice_id=1").await, Some(0));

        // A re-attempt that STILL fails leaves the notice held, untouched.
        assert_eq!(
            db.reclaim_notice(&held_notice(), &Parse::Quarantined { reason: "still".into(), detail: None }).await.unwrap(),
            Reclaim::StillHeld
        );
        assert_eq!(text_of(&db, "SELECT parse_state FROM notices WHERE id=1").await.as_deref(), Some("quarantined"));
        assert_eq!(int_of(&db, "SELECT reprocessed_at FROM quarantine WHERE notice_id=1").await, None);

        // Reprocess: the same identity now parses. The reclaim carries the fresh
        // ingest's wall-clock (999) and resolved instants (published 1_000_000).
        let mut fresh = held_notice();
        fresh.ingested_at = 999;
        fresh.published_at = Some(1_000_000);
        assert_eq!(db.reclaim_notice(&fresh, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::Reclaimed);

        // Parsed in place: state flipped, value written, watermark cleared so the
        // projection re-folds it, instants filled — and the quarantine row flagged.
        assert_eq!(text_of(&db, "SELECT parse_state FROM notices WHERE id=1").await.as_deref(), Some("parsed"));
        assert_eq!(int_of(&db, "SELECT projected FROM notices WHERE id=1").await, Some(0));
        assert_eq!(int_of(&db, "SELECT published_at FROM notices WHERE id=1").await, Some(1_000_000));
        assert_eq!(int_of(&db, "SELECT ingested_at FROM notices WHERE id=1").await, Some(100), "original ingest time is preserved");
        assert_eq!(text_of(&db, "SELECT value FROM notice_texts WHERE notice_id=1").await.as_deref(), Some("hello"));
        assert_eq!(int_of(&db, "SELECT reprocessed_at FROM quarantine WHERE notice_id=1").await, Some(999));
        assert!(db.unprojected_parsed_notice_ids().await.unwrap().contains(&1), "reclaimed notice is in the projection change-set");

        // Idempotent: a second pass is a no-op — never double-writes the values.
        assert_eq!(db.reclaim_notice(&fresh, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::AlreadyParsed);
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM notice_texts WHERE notice_id=1").await, Some(1));

        let _ = std::fs::remove_file(&path);
    }

    /// A profile-level held member (the DTD class: it failed before an identity
    /// existed, so there is NO `notices` row) is recorded fresh and its held
    /// quarantine row flagged — matched by (fetch_id, member_path), so a content
    /// hash difference between the raw-bytes quarantine and the notice can't strand
    /// it.
    #[tokio::test]
    async fn reclaim_records_a_held_profile_level_member() {
        let path = format!("/tmp/tender-db-reclaim-profile-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;

        // Profile-level quarantine: raw-bytes hash, no notice row.
        assert!(db
            .insert_quarantine(&Quarantined {
                fetch_id: 1,
                member_path: "pkg/m1".into(),
                content_hash: "raw-bytes-hash".into(),
                profile: None,
                reason: "unparsable-xml".into(),
                detail: Some("XML with DTD detected".into()),
                first_seen: 0,
            })
            .await
            .unwrap());
        assert!(int_of(&db, "SELECT id FROM notices WHERE publication_id='123-2001'").await.is_none(), "no notice row before reclaim");

        // Now it parses: the notice is recorded and the held row (with a DIFFERENT
        // content hash) flagged by (fetch_id, member_path).
        let mut fresh = held_notice();
        fresh.ingested_at = 999;
        fresh.published_at = Some(1_000_000);
        assert_eq!(db.reclaim_notice(&fresh, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::Reclaimed);
        assert_eq!(text_of(&db, "SELECT parse_state FROM notices WHERE publication_id='123-2001'").await.as_deref(), Some("parsed"));
        assert_eq!(int_of(&db, "SELECT reprocessed_at FROM quarantine WHERE member_path='pkg/m1'").await, Some(999));

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 139: a FAILED reclaim of a profile-level member splits its state —
    /// `record_notice_tx` inserts the notice row (state quarantined) while the
    /// original quarantine row keeps its NULL `notice_id` (the dispatch hash and
    /// the raw-bytes hash are the same, so the parse-level insert is ignored by
    /// UNIQUE(fetch_id, member_path, content_hash)). Every LATER reclaim then
    /// takes the notice-exists branch, whose `notice_id = ?` addressing alone
    /// matches nothing: 1,905 members were stamped into the void, and even a
    /// successful re-parse would have left their ledger rows outstanding
    /// forever. Both branch paths must also reach the
    /// (fetch_id, member_path, notice_id IS NULL) address.
    #[tokio::test]
    async fn reclaim_reaches_profile_level_rows_after_a_failed_attempt_created_the_notice() {
        let path = format!("/tmp/tender-db-reclaim-split-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;

        // Original ingest: profile-level quarantine, no notice row. The hash is
        // the raw member bytes' — the same one dispatch later gives the notice.
        assert!(db
            .insert_quarantine(&Quarantined {
                fetch_id: 1,
                member_path: "pkg/m1".into(),
                content_hash: "hash1".into(),
                profile: None,
                reason: "unparsable-xml".into(),
                detail: Some("XML with DTD detected".into()),
                first_seen: 0,
            })
            .await
            .unwrap());

        // First reprocess: dispatch now yields an identity but the deep parse
        // still fails. The no-notice branch records the notice row and stamps
        // the held row by (fetch_id, member_path).
        assert_eq!(
            db.reclaim_notice(
                &held_notice(),
                &Parse::Quarantined { reason: "unparsable-xml".into(), detail: Some("XML with DTD detected".into()) },
            )
            .await
            .unwrap(),
            Reclaim::StillHeld
        );
        let id = int_of(&db, "SELECT id FROM notices WHERE publication_id='123-2001'").await.expect("failed attempt records the notice row");
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM quarantine").await, Some(1), "same-hash insert is ignored: one ledger row");
        assert_eq!(int_of(&db, "SELECT attempts FROM quarantine WHERE notice_id IS NULL").await, Some(1));

        // Second reprocess, STILL failing: the notice row exists now, so the
        // notice-exists branch runs — it must stamp the NULL-notice_id row too,
        // recording the CURRENT failure (issue 87's contract).
        let mut again = held_notice();
        again.ingested_at = 200;
        assert_eq!(
            db.reclaim_notice(&again, &Parse::Quarantined { reason: "unclaimed-content".into(), detail: Some("7 rows".into()) })
                .await
                .unwrap(),
            Reclaim::StillHeld
        );
        assert_eq!(
            text_of(&db, "SELECT reason FROM quarantine WHERE notice_id IS NULL").await.as_deref(),
            Some("unclaimed-content"),
            "held profile-level row carries the current failure"
        );
        assert_eq!(int_of(&db, "SELECT attempts FROM quarantine WHERE notice_id IS NULL").await, Some(2), "exactly one stamp per attempt — the two addresses are disjoint");
        assert_eq!(text_of(&db, "SELECT first_reason FROM quarantine WHERE notice_id IS NULL").await.as_deref(), Some("unparsable-xml"));

        // Third reprocess, now parsing: the success path must flag the
        // NULL-notice_id ledger row reclaimed, or it reads outstanding forever.
        let mut fresh = held_notice();
        fresh.ingested_at = 999;
        fresh.published_at = Some(1_000_000);
        assert_eq!(db.reclaim_notice(&fresh, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::Reclaimed);
        assert_eq!(text_of(&db, &format!("SELECT parse_state FROM notices WHERE id={id}")).await.as_deref(), Some("parsed"));
        assert_eq!(int_of(&db, "SELECT reprocessed_at FROM quarantine WHERE notice_id IS NULL").await, Some(999));

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 181's stamping act: a whole-FILE rejection (not-utf8 on a CF
    /// member) holds one row for a file of ~1,000 `#<ordinal>` records. The
    /// first record reclaimed from the file must resolve the file row — on both
    /// the fresh-record path and the already-parsed arm (a re-run after the
    /// records entered the corpus).
    #[tokio::test]
    async fn reclaiming_a_record_resolves_its_files_whole_file_rejection_row() {
        let path = format!("/tmp/tender-db-filestamp-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;
        assert!(db
            .insert_quarantine(&Quarantined {
                fetch_id: 1,
                member_path: "pkg/EN_CF1.ZIP!EN_CF1".into(),
                content_hash: "file-hash".into(),
                profile: None,
                reason: "not-utf8".into(),
                detail: None,
                first_seen: 0,
            })
            .await
            .unwrap());

        // A record from inside the file reclaims (fresh path): the file row resolves.
        let mut n = held_notice();
        n.member_path = "pkg/EN_CF1.ZIP!EN_CF1#7".into();
        n.ingested_at = 500;
        assert_eq!(db.reclaim_notice(&n, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::Reclaimed);
        assert_eq!(
            int_of(&db, "SELECT reprocessed_at FROM quarantine WHERE member_path = 'pkg/EN_CF1.ZIP!EN_CF1'").await,
            Some(500),
            "the whole-file rejection row is resolved by its first reclaimed record"
        );

        // And the already-parsed arm does the same for a row left behind.
        db.conn()
            .await
            .execute("UPDATE quarantine SET reprocessed_at = NULL WHERE member_path = 'pkg/EN_CF1.ZIP!EN_CF1'", ())
            .await
            .unwrap();
        let mut again = n.clone();
        again.ingested_at = 900;
        assert_eq!(db.reclaim_notice(&again, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::AlreadyParsed);
        assert_eq!(
            int_of(&db, "SELECT reprocessed_at FROM quarantine WHERE member_path = 'pkg/EN_CF1.ZIP!EN_CF1'").await,
            Some(900)
        );

        let _ = std::fs::remove_file(&path);
    }

    /// The 1999 COR shape (four false issue-139 alarms in the job-654 drain): a
    /// correction FILE's records mostly duplicate an earlier daily, so the first
    /// record to come off the file resolves the whole-file rejection row — and
    /// the genuinely-corrected records that follow are FRESH identities whose
    /// stamps then find nothing left. That zero is benign, and the fresh-record
    /// path must see it as such via `member_file_resolved` (the guard the parsed
    /// arm already had) instead of sounding the stranded-ledger alarm.
    #[tokio::test]
    async fn corrected_records_after_a_resolved_file_row_are_a_benign_zero_stamp() {
        let path = format!("/tmp/tender-db-corstamp-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;
        assert!(db
            .insert_quarantine(&Quarantined {
                fetch_id: 1,
                member_path: "pkg/EN_COR.ZIP!EN_COR".into(),
                content_hash: "file-hash".into(),
                profile: None,
                reason: "not-utf8".into(),
                detail: None,
                first_seen: 0,
            })
            .await
            .unwrap());

        // The first record off the file resolves the file row (issue 181).
        let mut dup = held_notice();
        dup.member_path = "pkg/EN_COR.ZIP!EN_COR#3".into();
        dup.ingested_at = 500;
        assert_eq!(db.reclaim_notice(&dup, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::Reclaimed);

        // A LATER record of the same file carries a fresh identity — the
        // corrected re-issue. Its reclaim stamps zero rows, and the guard must
        // read that as resolved-already, not as issue 139's stranded ledger.
        let mut corrected = held_notice();
        corrected.publication_id = "999-2001".into();
        corrected.content_hash = "hash-corrected".into();
        corrected.member_path = "pkg/EN_COR.ZIP!EN_COR#7".into();
        corrected.ingested_at = 800;
        assert_eq!(db.reclaim_notice(&corrected, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::Reclaimed);
        let conn = db.conn().await;
        assert!(
            db.member_file_resolved(&conn, &corrected).await.unwrap(),
            "the corrected record's zero-stamp is benign: its file row is already resolved"
        );
        assert_eq!(
            int_of(&db, "SELECT reprocessed_at FROM quarantine WHERE member_path = 'pkg/EN_COR.ZIP!EN_COR'").await,
            Some(500),
            "the file row keeps its first resolution instant — later records never re-stamp it"
        );
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM quarantine").await, Some(1), "no new ledger rows appear");

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 200's drain shape: after a segmentation change, a file's old
    /// record-level rows resolve under ordinals that no longer align with the
    /// records stamping them. A fresh record whose own ordinal has no row must
    /// read the file's RESOLVED SIBLING as a benign zero, not an alarm.
    #[tokio::test]
    async fn resolved_record_siblings_are_a_benign_zero_stamp() {
        let path = format!("/tmp/tender-db-siblingstamp-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;
        assert!(db
            .insert_quarantine(&Quarantined {
                fetch_id: 1,
                member_path: "pkg/EN_ORG.ZIP!EN_ORG#5".into(),
                content_hash: "old-tail-hash".into(),
                profile: Some("text".into()),
                reason: "missing-publication-id".into(),
                detail: None,
                first_seen: 0,
            })
            .await
            .unwrap());
        // Some record's stamp resolved the old row (the by_member coincidence).
        db.conn()
            .await
            .execute("UPDATE quarantine SET reprocessed_at = 400 WHERE member_path = 'pkg/EN_ORG.ZIP!EN_ORG#5'", ())
            .await
            .unwrap();

        // A fresh merged record at a different ordinal: zero rows to stamp,
        // but the file's bookkeeping is demonstrably live.
        let mut merged = held_notice();
        merged.publication_id = "777-2010".into();
        merged.content_hash = "merged-hash".into();
        merged.member_path = "pkg/EN_ORG.ZIP!EN_ORG#7".into();
        merged.ingested_at = 500;
        assert_eq!(db.reclaim_notice(&merged, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::Reclaimed);
        let conn = db.conn().await;
        assert!(
            db.member_file_resolved(&conn, &merged).await.unwrap(),
            "a resolved record sibling reads as benign"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 289: `member_file_resolved` alone blinds the zero-stamp alarm for a
    /// whole file the moment its FIRST record resolves — a later record whose
    /// reclaim genuinely strands its row went unlogged. The discriminator that
    /// separates a real stranding from the benign 181/196/200 zeros: a stranding
    /// leaves an UNRESOLVED family row behind. `member_family_still_held` is that
    /// probe, and the alarm now fires (with a distinguishing marker) when a
    /// partially-resolved family still holds residue.
    #[tokio::test]
    async fn a_zero_stamp_under_a_partially_resolved_file_is_not_silenced() {
        let path = format!("/tmp/tender-db-partialfam-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;
        // Sibling #5: resolved (an earlier record's stamp). Sibling #9: still HELD
        // under a path/hash no current reclaim addresses — the stranded residue.
        for (ordinal, hash) in [("5", "old-hash-5"), ("9", "drifted-hash-9")] {
            assert!(db
                .insert_quarantine(&Quarantined {
                    fetch_id: 1,
                    member_path: format!("pkg/EN_FAM.ZIP!EN_FAM#{ordinal}"),
                    content_hash: hash.to_string(),
                    profile: Some("text".into()),
                    reason: "missing-publication-id".into(),
                    detail: None,
                    first_seen: 0,
                })
                .await
                .unwrap());
        }
        db.conn()
            .await
            .execute("UPDATE quarantine SET reprocessed_at = 400 WHERE member_path = 'pkg/EN_FAM.ZIP!EN_FAM#5'", ())
            .await
            .unwrap();

        // A fresh record at another ordinal reclaims with zero stamps.
        let mut fresh = held_notice();
        fresh.publication_id = "888-2010".into();
        fresh.content_hash = "fresh-hash".into();
        fresh.member_path = "pkg/EN_FAM.ZIP!EN_FAM#7".into();
        fresh.ingested_at = 500;
        assert_eq!(db.reclaim_notice(&fresh, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::Reclaimed);

        let conn = db.conn().await;
        assert!(
            db.member_file_resolved(&conn, &fresh).await.unwrap(),
            "the old gate alone reads this as benign — the suppression issue 289 diagnosed"
        );
        assert!(
            db.member_family_still_held(&conn, &fresh).await.unwrap(),
            "but held residue remains (#9) — the marker line must fire (issue 289)"
        );

        // Once the residue resolves, the family is fully benign and the probe
        // goes quiet — the 181/196/200 shapes stay silent. (Reuse the held
        // writer handle: a second `conn()` while it lives would deadlock the
        // single-writer pool.)
        conn.execute("UPDATE quarantine SET skipped_at = 600, skipped_reason = 'policy' WHERE member_path = 'pkg/EN_FAM.ZIP!EN_FAM#9'", ())
            .await
            .unwrap();
        assert!(
            !db.member_family_still_held(&conn, &fresh).await.unwrap(),
            "a fully-resolved family leaves nothing to report"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn member_container_names_the_nested_archive() {
        assert_eq!(member_container("06/20260601_2026103.tar.gz/00364561_2026.xml").as_deref(), Some("06/20260601_2026103.tar.gz"));
        assert_eq!(
            member_container("20260601.tar.gz/20240102_1/notice.xml").as_deref(),
            Some("20260601.tar.gz"),
            "a subdirectory below the container does not hide it"
        );
        assert_eq!(member_container("pkg.tar.gz/EN_CF1.ZIP!EN_CF1#7").as_deref(), Some("pkg.tar.gz/EN_CF1.ZIP"));
        assert_eq!(member_container("pkg/EN_X.ZIP!EN_X").as_deref(), Some("pkg/EN_X.ZIP"));
        assert_eq!(member_container("pkg/m1"), None, "a plain directory prefix is not a container");
        assert_eq!(member_container("m1.xml"), None);
    }

    /// Issue 196: the pre-recursion walker recorded a monthly's inner dailies
    /// as raw members — ONE row at the container path (`06/<daily>.tar.gz`)
    /// that no record- or file-level address reaches. The first record
    /// reclaimed from inside the container resolves the row (the fourth stamp
    /// address), and the records that follow are benign zeros, not alarms.
    #[tokio::test]
    async fn reclaiming_a_record_resolves_its_containers_rejection_row() {
        let path = format!("/tmp/tender-db-containerstamp-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;
        assert!(db
            .insert_quarantine(&Quarantined {
                fetch_id: 1,
                member_path: "06/20260601_2026103.tar.gz".into(),
                content_hash: "container-hash".into(),
                profile: None,
                reason: "not-utf8".into(),
                detail: None,
                first_seen: 0,
            })
            .await
            .unwrap());

        let mut n = held_notice();
        n.member_path = "06/20260601_2026103.tar.gz/00364561_2026.xml".into();
        n.ingested_at = 600;
        assert_eq!(db.reclaim_notice(&n, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::Reclaimed);
        assert_eq!(
            int_of(&db, "SELECT reprocessed_at FROM quarantine WHERE member_path = '06/20260601_2026103.tar.gz'").await,
            Some(600),
            "the whole-container rejection row is resolved by its first reclaimed record"
        );

        // A later record out of the same container: benign zero, first stamp kept.
        let mut sibling = held_notice();
        sibling.publication_id = "888-2026".into();
        sibling.content_hash = "hash-sibling".into();
        sibling.member_path = "06/20260601_2026103.tar.gz/00364777_2026.xml".into();
        sibling.ingested_at = 700;
        assert_eq!(db.reclaim_notice(&sibling, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::Reclaimed);
        let conn = db.conn().await;
        assert!(
            db.member_file_resolved(&conn, &sibling).await.unwrap(),
            "sibling records read the resolved container, not a stranded ledger"
        );
        assert_eq!(
            int_of(&db, "SELECT reprocessed_at FROM quarantine WHERE member_path = '06/20260601_2026103.tar.gz'").await,
            Some(600)
        );

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 193: a text-era record's `#<ordinal>` is its index in TODAY'S
    /// segmentation, so a parser change shifts it and the reclaim attempt's
    /// exact-path stamp misses rows whose bytes never changed. The attempt must
    /// fall back to the record's content hash — and prefer the exact path when
    /// it does match, so a hash-twin row elsewhere in the fetch is not touched
    /// when the path already identifies the row.
    #[tokio::test]
    async fn reclaim_attempt_reaches_rows_whose_ordinal_drifted() {
        let path = format!("/tmp/tender-db-attempt-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;
        for (m, h) in [("z.zip!E#95", "rec-hash"), ("z.zip!E#12", "other-hash")] {
            assert!(db
                .insert_quarantine(&Quarantined {
                    fetch_id: 1,
                    member_path: m.into(),
                    content_hash: h.into(),
                    profile: Some("text".into()),
                    reason: "unclaimed-content".into(),
                    detail: Some("line 3: )".into()),
                    first_seen: 0,
                })
                .await
                .unwrap());
        }

        // Ordinal drifted (#95 → #93), bytes unchanged: the hash fallback stamps.
        db.record_reclaim_attempt(1, "z.zip!E#93", "rec-hash", "missing-publication-id", None, 500)
            .await
            .unwrap();
        assert_eq!(
            text_of(&db, "SELECT reason FROM quarantine WHERE member_path = 'z.zip!E#95'").await.as_deref(),
            Some("missing-publication-id"),
            "hash fallback reaches the drifted row"
        );
        assert_eq!(
            text_of(&db, "SELECT reason FROM quarantine WHERE member_path = 'z.zip!E#12'").await.as_deref(),
            Some("unclaimed-content"),
            "the other row is untouched"
        );

        // Exact path match wins: no hash spillover onto the already-stamped row.
        db.record_reclaim_attempt(1, "z.zip!E#12", "other-hash", "text-no-records", None, 600)
            .await
            .unwrap();
        assert_eq!(
            text_of(&db, "SELECT reason FROM quarantine WHERE member_path = 'z.zip!E#12'").await.as_deref(),
            Some("text-no-records")
        );
        assert_eq!(
            int_of(&db, "SELECT attempts FROM quarantine WHERE member_path = 'z.zip!E#95'").await,
            Some(1),
            "exact-path success never re-stamps via the hash"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 139's second act: the notice reclaimed but the run that reclaimed it
    /// failed to stamp its ledger row (whatever the cause), leaving a parsed
    /// notice with an outstanding quarantine row. Every later reprocess hits the
    /// already-parsed guard — which used to stamp nothing, so the panel claimed
    /// the member outstanding forever and NO re-run could converge it. The guard
    /// now resolves the member's ledger rows too, and stays idempotent: a
    /// stamped row is terminal and is never re-stamped with a later wall-clock.
    #[tokio::test]
    async fn already_parsed_reclaim_still_resolves_the_ledger_row() {
        let path = format!("/tmp/tender-db-reclaim-already-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        seed_fetch(&db).await;

        // The stranded shape: a parse-level quarantine whose notice was later
        // parsed without the ledger stamp landing.
        assert!(db
            .record_notice(
                &held_notice(),
                &Parse::Quarantined { reason: "unparsable-xml".into(), detail: Some("XML with DTD detected".into()) },
            )
            .await
            .unwrap());
        db.conn().await.execute("UPDATE notices SET parse_state = 'parsed' WHERE id = 1", ()).await.unwrap();
        assert_eq!(int_of(&db, "SELECT reprocessed_at FROM quarantine WHERE notice_id = 1").await, None);

        // The re-run: already parsed — and the ledger row resolves now.
        let mut fresh = held_notice();
        fresh.ingested_at = 500;
        assert_eq!(db.reclaim_notice(&fresh, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::AlreadyParsed);
        assert_eq!(int_of(&db, "SELECT reprocessed_at FROM quarantine WHERE notice_id = 1").await, Some(500));
        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM notice_texts WHERE notice_id = 1").await, Some(0), "already-parsed never rewrites the parsed layer");

        // Terminal: a later pass never moves the stamp.
        let mut later = held_notice();
        later.ingested_at = 900;
        assert_eq!(db.reclaim_notice(&later, &Parse::Parsed(tiny_parsed())).await.unwrap(), Reclaim::AlreadyParsed);
        assert_eq!(int_of(&db, "SELECT reprocessed_at FROM quarantine WHERE notice_id = 1").await, Some(500));

        let _ = std::fs::remove_file(&path);
    }

    /// The work list is package-granular, held-only, and resumable: it returns the
    /// distinct packages of a bucket whose `fetch_id` exceeds the cursor, and
    /// resolved rows fall out — reclaimed (`reprocessed_at`) and policy-skipped
    /// (`skipped_at`, issue 197) alike, or a package whose bucket is all skipped
    /// sits on every work list forever and gets re-walked for nothing.
    #[tokio::test]
    async fn reclaim_packages_lists_held_buckets_resumably() {
        let path = format!("/tmp/tender-db-reclaim-list-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();
        db.conn()
            .await
            .execute_batch(
                "INSERT INTO fetches(id, source, kind, period, url, sha256, bytes, fetched_at, path) VALUES
                   (1,'ted','daily','a','u','h',1,0,'p1'),(2,'ted','daily','b','u','h',1,0,'p2'),(3,'doe','daily','c','u','h',1,0,'p3'),(4,'ted','monthly','d','u','h',1,0,'p4');
                 INSERT INTO quarantine(fetch_id, member_path, content_hash, reason, detail, first_seen, reprocessed_at, skipped_at) VALUES
                   (1,'m1','h1','unknown-field-code','line 3: OC',0,NULL,NULL),
                   (1,'m2','h2','unknown-field-code','line 9: OC',0,NULL,NULL),
                   (2,'m3','h3','unknown-field-code','line 3: OC',0,NULL,NULL),
                   (3,'m4','h4','unknown-field-code','line 3: OC',0,123,NULL),
                   (4,'m6','h6','unknown-field-code','line 3: OC',0,NULL,456),
                   (2,'m5','h5','unparsable-xml','other',0,NULL,NULL);",
            )
            .await
            .unwrap();
        db.set_foreign_keys(true).await.unwrap();

        // The OC bucket: two held packages (1 and 2), deduped; pkg 3 is already
        // reclaimed (reprocessed_at set), pkg 4's only row is policy-skipped
        // (skipped_at set — the 2008-monthly shape of issue 197), and pkg 2's
        // row m5 is a different reason.
        let all = db.quarantine_reclaim_packages("unknown-field-code", Some("%: OC"), None, 0).await.unwrap();
        assert_eq!(all.iter().map(|(id, ..)| *id).collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(all[0].2, "p1");

        // Resume past fetch 1: only package 2 remains.
        let rest = db.quarantine_reclaim_packages("unknown-field-code", Some("%: OC"), None, 1).await.unwrap();
        assert_eq!(rest.iter().map(|(id, ..)| *id).collect::<Vec<_>>(), vec![2]);

        let _ = std::fs::remove_file(&path);
    }

    /// The per-package held-member work list (issue 77): distinct member FILES of
    /// the bucket for one package, text-era `#<ordinal>` stripped, excluding other
    /// reasons and resolved rows — reclaimed and policy-skipped alike (issue 197:
    /// before `skipped_at` was filtered, 593k skipped DTD siblings were re-walked
    /// and re-declined on every unparsable-xml reprocess).
    #[tokio::test]
    async fn held_member_files_are_the_bucket_of_one_package() {
        let path = format!("/tmp/tender-db-held-files-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();
        db.conn()
            .await
            .execute_batch(
                "INSERT INTO quarantine(fetch_id, member_path, content_hash, reason, detail, first_seen, reprocessed_at, skipped_at) VALUES
                   (1,'pkg/a.xml','h1','unknown-field-code','line 3: OC',0,NULL,NULL),
                   (1,'pkg/bundle.zip#0','h2','unknown-field-code','line 3: OC',0,NULL,NULL),
                   (1,'pkg/bundle.zip#1','h3','unknown-field-code','line 9: OC',0,NULL,NULL),
                   (1,'pkg/done.xml','h4','unknown-field-code','line 3: OC',0,555,NULL),
                   (1,'pkg/dupe.xml','h7','unknown-field-code','line 3: OC',0,NULL,777),
                   (1,'pkg/other.xml','h5','unclaimed-content','x',0,NULL,NULL),
                   (2,'pkg2/z.xml','h6','unknown-field-code','line 3: OC',0,NULL,NULL);",
            )
            .await
            .unwrap();
        db.set_foreign_keys(true).await.unwrap();

        let held = db.quarantine_held_member_files(1, "unknown-field-code", Some("%: OC"), None).await.unwrap();
        // a.xml + the two bundle records collapsed to the one member file; NOT the
        // reclaimed done.xml, the skipped dupe.xml, the other-reason row, or fetch 2.
        let mut got: Vec<_> = held.into_iter().collect();
        got.sort();
        assert_eq!(got, vec!["pkg/a.xml".to_string(), "pkg/bundle.zip".to_string()]);

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 81: the one-time CDC clean drops the feed + its high-water, resets the
    /// in-memory cursor watch, keeps both change indexes, and restarts the cursor at
    /// 1 — so a paired rebuild re-emits one clean generation.
    #[tokio::test]
    async fn clear_changes_resets_the_feed_and_cursor() {
        let path = format!("/tmp/tender-db-clearchanges-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();

        // Seed the feed and advance the in-memory cursor watch to the high-water.
        {
            let conn = db.conn().await;
            conn.execute(
                "INSERT INTO changes(entity_kind, entity_id, version_seq, op, changed_at)
                 VALUES('tender',1,1,'added',0),('tender',2,1,'added',0)",
                (),
            )
            .await
            .unwrap();
            db.publish_cursor(&conn).await.unwrap();
        }
        assert_eq!(db.current_cursor(), 2, "watch advanced to the high-water");

        db.clear_changes().await.unwrap();

        assert_eq!(int_of(&db, "SELECT COUNT(*) FROM changes").await, Some(0), "feed emptied");
        assert_eq!(db.current_cursor(), 0, "the in-memory cursor watch is reset");
        assert_eq!(
            int_of(&db, "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name IN ('changes_entity','changes_entity_cursor')").await,
            Some(2),
            "both change indexes are recreated"
        );
        // A fresh append restarts the cursor at 1 (the autoincrement high-water dropped).
        db.conn()
            .await
            .execute("INSERT INTO changes(entity_kind, entity_id, op, changed_at) VALUES('tender',9,'added',0)", ())
            .await
            .unwrap();
        assert_eq!(int_of(&db, "SELECT MIN(cursor) FROM changes").await, Some(1), "cursor restarts at 1");

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 80: the reprocess flags a reclaimed member by notice_id every member,
    /// so that lookup must SEEK the `quarantine_notice_id` index — a SCAN of the
    /// ~2.4M-row table per member cliffs a dense bucket. Asserting the plan proves
    /// the seek at any scale.
    #[tokio::test]
    async fn reclaim_flag_seeks_the_notice_id_index() {
        let path = format!("/tmp/tender-db-qnid-eqp-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let conn = db.reader().await.unwrap();
        let mut rows = conn
            .query(
                "EXPLAIN QUERY PLAN
                 SELECT id FROM quarantine WHERE notice_id = ? AND reprocessed_at IS NULL",
                (Value::Integer(1),),
            )
            .await
            .unwrap();
        let mut plan = String::new();
        while let Some(row) = rows.next().await.unwrap() {
            plan.push_str(&text(&row, 3));
            plan.push('\n');
        }
        assert!(
            plan.contains("quarantine_notice_id"),
            "the reclaim flag must seek the notice_id index — plan was:\n{plan}"
        );
        assert!(
            !plan.to_uppercase().contains("SCAN"),
            "it must SEARCH by index, never SCAN the ~2.4M-row table — plan was:\n{plan}"
        );

        let _ = std::fs::remove_file(&path);
    }
}

#[cfg(test)]
mod fk_enforcement {
    use super::*;

    /// The declared `REFERENCES` on the canonical tables are ENFORCED by turso,
    /// in both directions — and the whole incremental-verification mapping
    /// (issue 133 / task #28) rests on that being true.
    ///
    /// The `orphan_*` family of standing checks has no incremental assertion
    /// form of its own outside the projection, because it does not need one:
    /// with `foreign_keys=ON` an orphan is *unrepresentable*. That argument is
    /// only worth as much as this test. turso is a young engine and a declared
    /// constraint it does not enforce reads exactly like one it does — so this
    /// asserts the enforcement rather than trusting the declaration.
    ///
    /// Both arms matter and they fail independently:
    ///   * child-insert  — the fold's shape (a version for a missing tender);
    ///   * parent-delete — the LATER breakage sdk-vendor raised, where the rows
    ///     are written correctly and the invariant is broken afterwards by a
    ///     delete. An engine could enforce the first and not the second.
    ///
    /// This says nothing about the projection itself, which runs
    /// `set_foreign_keys(false)` deliberately (issue 19) — inside a projection
    /// run these guarantees are OFF by design, which is precisely why the
    /// orphan checks still need batch-scoped assertions THERE and only there.
    #[tokio::test]
    async fn declared_foreign_keys_are_enforced_on_insert_and_on_delete() {
        let path = format!("/tmp/tender-db-fkenforce-{}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        let db = Db::open(&path).await.unwrap();

        // A child pointing at a parent that does not exist must be refused.
        let orphan_insert = {
            let conn = db.conn().await;
            conn.execute_batch(
                "INSERT INTO tender_versions(tender_id,seq,caused_by_notice_id,published_at,publication_id)
                   VALUES (999,1,1,1,'x');",
            )
            .await
        };
        assert!(
            matches!(orphan_insert, Err(turso::Error::Constraint(_))),
            "turso accepted a tender_versions row for a nonexistent tender — the \
             orphan_* checks lose their structural guarantee: {orphan_insert:?}"
        );

        // Build a VALID parent/child pair with enforcement off (the projection's
        // regime), restore enforcement, then delete the parent.
        db.set_foreign_keys(false).await.unwrap();
        {
            let conn = db.conn().await;
            conn.execute_batch(
                "INSERT INTO notices(id,source,publication_id,content_hash,parse_state,profile,fetch_id,member_path,ingested_at)
                   VALUES (1,'ted','p1','h1','parsed','eforms',1,'m',0);
                 INSERT INTO tenders(id,source,kind,created_at,current_seq,current_published_at)
                   VALUES (1,'ted','procedure',0,1,1);
                 INSERT INTO tender_versions(tender_id,seq,caused_by_notice_id,published_at,publication_id)
                   VALUES (1,1,1,1,'x');",
            )
            .await
            .unwrap();
        }
        db.set_foreign_keys(true).await.unwrap();

        let parent_delete = {
            let conn = db.conn().await;
            conn.execute("DELETE FROM tenders WHERE id=1", ()).await
        };
        assert!(
            matches!(parent_delete, Err(turso::Error::Constraint(_))),
            "turso deleted a tender that still has versions — orphans CAN appear \
             after a correct write, and orphan_versions needs a real detector: {parent_delete:?}"
        );

        // And the child is still there, with its parent: the delete was refused,
        // not silently cascaded (which would be a different, also-wrong outcome).
        let (children, parents) = {
            let conn = db.conn().await;
            let mut c = conn
                .query("SELECT (SELECT COUNT(*) FROM tender_versions WHERE tender_id=1), (SELECT COUNT(*) FROM tenders WHERE id=1)", ())
                .await
                .unwrap();
            let row = c.next().await.unwrap().unwrap();
            (
                row.get_value(0).unwrap().as_integer().copied().unwrap_or(-1),
                row.get_value(1).unwrap().as_integer().copied().unwrap_or(-1),
            )
        };
        assert_eq!((children, parents), (1, 1), "the refused DELETE must leave the pair intact, not cascade");
    }
}
