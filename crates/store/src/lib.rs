//! Turso (pure-Rust SQLite) persistence — server-only by construction (the app
//! crate pulls this in behind its `server` feature; it never reaches wasm).
//!
//! One embedded database file (`TENDER_DB`, default `tender-db.db` in the working
//! directory — the systemd state dir in production). Accessors live on [`Db`];
//! the process-wide instance owns a single connection behind a mutex and
//! serialises access.

pub mod accounts;
pub mod backup;
pub mod canonical;
pub mod checkpoint;
pub mod jobs;
pub mod read;
pub mod webhooks;

/// Re-exported so callers can name `Error`/`Connection`/`Value` without taking
/// their own pin on the engine — the store owns which Turso this is.
pub use turso;

pub use accounts::{TokenRecord, User};
pub use backup::{BackupError, SnapshotReport};
pub use checkpoint::{Checkpointed, CheckpointMode};
pub use canonical::{
    Applied, BidParty, BidState, Change, ContractState, Fact, Identifier, LotResultState, LotState,
    Mention, MentionResolver, NoticeRef, PlanGroup, PlanRow, Round, TenderProjection, TenderVersion,
};
pub use jobs::QueuedJobRow;
pub use read::{Filter, Reader, Readers, Status};
pub use webhooks::{Delivery, Endpoint};

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard, OnceCell, watch};
use turso::{Connection, Value};

/// Sane connection defaults, per <https://mort.coffee/home/sqlite-editions/>:
/// enforce foreign keys, retry on lock contention instead of failing with
/// SQLITE_BUSY, WAL for concurrent reads during writes, and NORMAL sync (safe
/// under WAL, much faster than FULL).
///
/// `cache_size = -131072` is 128 MiB of page cache per connection (negative =
/// KiB), up from turso's ~2 MB default (issue 60). It is a per-connection *cap*
/// that fills lazily, but it is charged PER CONNECTION, and the process opens
/// many: the writer + the store read pool (8) + the API/SQL/webhook pools (~14) +
/// the Phase-2 parallel pre-pass's K shard readers. At the old 512 MiB the
/// aggregate CEILING was ~11.5 GiB on the 8 GB box, and during a full-corpus
/// rebuild the ACTIVE fillers (writer + K pre-pass readers, each sweeping the
/// corpus) grew toward 512 MiB apiece and tipped the box into swap-thrash
/// (issue 61 incident: RSS 4.1 G + 4.2 G swapped). 128 MiB drops the aggregate
/// CEILING to ~23 conns × 128 MiB ≈ 2.9 GiB and bounds the ACTIVE set well under
/// ~2 GB (writer + K≈3 ≈ ~0.5 GB) while still dwarfing turso's default, so point
/// queries and the sequential scans (which ride the kernel's per-fd readahead, not
/// this cache) are unaffected on our hot paths — the resume and the incremental
/// both SKIP Phase-1, the only pass the larger cache measurably helped. CAVEAT
/// (not blocking, tracked in issue 68): a from-scratch rebuild's Phase-1 does ~11
/// indexed range scans per chunk and benefited from the bigger cache; if a fresh
/// rebuild's Phase-1 regresses, promote to the targeted split (projection writer +
/// pre-pass readers → 256 MiB, API/SQL/webhook pools → 64 MiB) rather than raising
/// this shared default back up.
pub(crate) const PRAGMAS: [&str; 5] = [
    "PRAGMA foreign_keys = ON",
    "PRAGMA busy_timeout = 5000",
    "PRAGMA journal_mode = WAL",
    "PRAGMA synchronous = NORMAL",
    "PRAGMA cache_size = -131072",
];

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
    CREATE INDEX IF NOT EXISTS notice_sections_kind ON notice_sections(kind);

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
";

pub struct Db {
    database: turso::Database,
    conn: Mutex<Connection>,
    /// The database file path, kept so [`Db::snapshot`] (issue 23) knows which
    /// file to copy — turso exposes no path accessor.
    path: String,
    /// The pool backing `Db`'s own read-only accessors. Reads run over WAL in
    /// parallel with the writer, so a dashboard/admin query never queues behind
    /// an ingestion job that is holding the writer for the length of its
    /// transaction (issue 20). The writer (`conn`) is reserved for writes and
    /// schema; this is separate from the public API's own pool (`readers`).
    read_pool: Arc<Readers>,
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
const MIGRATIONS: [&str; 4] = [
    "ALTER TABLE notices ADD COLUMN published_at INTEGER",
    "ALTER TABLE notices ADD COLUMN dispatched_at INTEGER",
    "ALTER TABLE tender_versions ADD COLUMN dispatched_at INTEGER",
    // The resume cursor (issue 32). job_queue shipped in bad8dda (issue 21), so
    // the prod table predates this column — CREATE TABLE IF NOT EXISTS never adds
    // it, and recover()'s `SELECT … progress` would fail on the first boot of the
    // new binary. NULL for every existing row, which is correct (a fresh cursor).
    "ALTER TABLE job_queue ADD COLUMN progress TEXT",
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
    let added = add_column(conn, "ALTER TABLE tenders ADD COLUMN current_seq INTEGER").await?;
    let added = add_column(conn, "ALTER TABLE tenders ADD COLUMN current_published_at INTEGER").await? || added;
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

impl Db {
    pub async fn open(path: &str) -> turso::Result<Db> {
        let database = turso::Builder::new_local(path).build().await?;
        let conn = database.connect()?;
        // Some pragmas report their new value as a row, so go through `query`
        // (execute rejects statements that return rows) and drain the result.
        for pragma in PRAGMAS {
            let mut rows = conn.query(pragma, ()).await?;
            while rows.next().await?.is_some() {}
        }
        conn.execute_batch(SCHEMA).await?;
        conn.execute_batch(canonical::SCHEMA).await?;
        conn.execute_batch(accounts::SCHEMA).await?;
        conn.execute_batch(jobs::SCHEMA).await?;
        conn.execute_batch(webhooks::SCHEMA).await?;
        migrate(&conn).await?;
        let cursor = watch::Sender::new(max_cursor(&conn).await?);
        let read_pool = Readers::open(database.clone(), READ_POOL)?;
        Ok(Db { database, conn: Mutex::new(conn), path: path.to_owned(), read_pool, cursor })
    }

    async fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().await
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
        Readers::open(self.database.clone(), n)
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

    async fn reclaim_notice_tx(&self, conn: &Connection, n: &Notice, parse: &Parse) -> turso::Result<Reclaim> {
        match self.notice_state(conn, n).await? {
            // Already good — never re-touch a parsed notice (guards double-writes
            // and makes a re-run a no-op).
            Some((_, state)) if state == "parsed" => Ok(Reclaim::AlreadyParsed),
            // A held parse-level quarantine: the notice row exists (empty of parsed
            // values — it was quarantined before `insert_parsed` ran), so write the
            // parsed layer in place when it now parses. This is the case a plain
            // re-run of `process` can never reach (its `INSERT OR IGNORE` short-
            // circuits) — OC (issue 72) and the SDK cohort (issue 71).
            Some((id, _)) => match parse {
                Parse::Parsed(parsed) => {
                    self.insert_parsed(conn, id, parsed).await?;
                    conn.execute(
                        "UPDATE notices SET parse_state = 'parsed', projected = 0,
                             published_at = ?, dispatched_at = ? WHERE id = ?",
                        (opt_int(n.published_at), opt_int(n.dispatched_at), Value::Integer(id)),
                    )
                    .await?;
                    conn.execute(
                        "UPDATE quarantine SET reprocessed_at = ?
                          WHERE notice_id = ? AND reprocessed_at IS NULL",
                        (Value::Integer(n.ingested_at), Value::Integer(id)),
                    )
                    .await?;
                    Ok(Reclaim::Reclaimed)
                }
                _ => Ok(Reclaim::StillHeld),
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
                    conn.execute(
                        "UPDATE quarantine SET reprocessed_at = ?
                          WHERE fetch_id = ? AND member_path = ? AND reprocessed_at IS NULL",
                        (Value::Integer(n.ingested_at), Value::Integer(n.fetch_id), t(&n.member_path)),
                    )
                    .await?;
                    Ok(Reclaim::Reclaimed)
                } else {
                    Ok(Reclaim::StillHeld)
                }
            }
        }
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
            conn.execute(
                "INSERT INTO notice_sections(notice_id, section_id, kind, parent_section_id)
                 VALUES(?, ?, ?, ?)",
                (Value::Integer(id), t(&s.id), t(&s.kind), opt_text(s.parent.as_deref())),
            )
            .await?;
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
    /// a prior reprocess has not yet reclaimed (`reprocessed_at IS NULL`), whose
    /// `fetch_id` exceeds `after`. The reprocess job's resumable work list: it
    /// returns distinct packages (not rows), so the result is bounded by package
    /// count regardless of how large the bucket is, and reclaimed packages fall
    /// out of a re-query automatically.
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
                  WHERE q.reason = ?1 AND q.reprocessed_at IS NULL
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
    /// reclaimed. Lets the reprocess parse ONLY these members and skip the rest —
    /// a sparse bucket re-parses `held/total` of the package instead of all of it.
    /// Seeks by `fetch_id` (the leading column of the quarantine unique index), so
    /// it is a bounded per-package lookup.
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
                  WHERE fetch_id = ?1 AND reason = ?2 AND reprocessed_at IS NULL
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
    /// code drives the bucket — it does: `OC` on the ISO-era text records.
    pub async fn quarantine_field_code_gaps(&self, limit: i64) -> turso::Result<Vec<(String, i64)>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT substr(detail, instr(detail, ': ') + 2) AS code, COUNT(*) c
                   FROM quarantine WHERE reason = 'unknown-field-code'
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
    pub async fn quarantine_resolution(
        &self,
        reason: &str,
        profile: Option<&str>,
        detail_like: Option<&str>,
    ) -> turso::Result<(i64, i64)> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT SUM(CASE WHEN reprocessed_at IS NOT NULL THEN 1 ELSE 0 END),
                        SUM(CASE WHEN reprocessed_at IS NULL     THEN 1 ELSE 0 END)
                   FROM quarantine
                  WHERE reason = ?
                    AND (? IS NULL OR profile = ?)
                    AND (? IS NULL OR detail LIKE ?)",
                (
                    reason.to_owned(),
                    opt_text(profile),
                    opt_text(profile),
                    opt_text(detail_like),
                    opt_text(detail_like),
                ),
            )
            .await?;
        // SUM over no matching rows is NULL — an unreprocessed, un-held category
        // is simply (0, 0).
        let row = rows.next().await?;
        Ok(row
            .map(|row| (opt_int_of(&row, 0).unwrap_or(0), opt_int_of(&row, 1).unwrap_or(0)))
            .unwrap_or((0, 0)))
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
                    AND (? IS NULL OR detail LIKE ?)",
                (
                    "unclaimed-content".to_owned(),
                    opt_text(Some("text")),
                    opt_text(Some("text")),
                    opt_text(Some("%scalar field RP")),
                    opt_text(Some("%scalar field RP")),
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
    /// `line <n>: <code>` detail, so one code across different line numbers sums
    /// into a single row — and only the unknown-field-code bucket is counted.
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
                   (1,'m4','h4','unclaimed-content','line 5: whatever',0);",
            )
            .await
            .unwrap();
        }
        db.set_foreign_keys(true).await.unwrap();

        let gaps = db.quarantine_field_code_gaps(5).await.unwrap();
        assert_eq!(
            gaps,
            vec![("OC".to_owned(), 2), ("XY".to_owned(), 1)],
            "OC sums across its two line numbers; unclaimed-content is excluded",
        );

        let _ = std::fs::remove_file(&path);
    }

    /// Issue 40: a resolution-ledger key splits its matching quarantine rows into
    /// reclaimed (reprocessed) and outstanding, and the profile + detail narrowers
    /// keep one ledger entry from counting a sibling bucket's rows.
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
                   (1,'m5','h5','text','unclaimed-content','line 8: continuation under scalar field XY',0,NULL);",
            )
            .await
            .unwrap();
        }
        db.set_foreign_keys(true).await.unwrap();

        // r208 @REASON: one reprocessed, one still held; the sibling @CATEGORY row
        // is excluded by the detail pattern.
        assert_eq!(
            db.quarantine_resolution("unclaimed-content", Some("ted-export-r208"), Some("%@REASON")).await.unwrap(),
            (1, 1),
        );
        // text RP: reclaimed, with the sibling XY continuation excluded.
        assert_eq!(
            db.quarantine_resolution("unclaimed-content", Some("text"), Some("%scalar field RP")).await.unwrap(),
            (1, 0),
        );
        // A key that matches nothing yet is simply (0, 0), never an error.
        assert_eq!(
            db.quarantine_resolution("unknown-field-code", None, None).await.unwrap(),
            (0, 0),
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

    /// The work list is package-granular, held-only, and resumable: it returns the
    /// distinct packages of a bucket whose `fetch_id` exceeds the cursor, and
    /// reclaimed rows (`reprocessed_at` set) fall out.
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
                   (1,'ted','daily','a','u','h',1,0,'p1'),(2,'ted','daily','b','u','h',1,0,'p2'),(3,'doe','daily','c','u','h',1,0,'p3');
                 INSERT INTO quarantine(fetch_id, member_path, content_hash, reason, detail, first_seen, reprocessed_at) VALUES
                   (1,'m1','h1','unknown-field-code','line 3: OC',0,NULL),
                   (1,'m2','h2','unknown-field-code','line 9: OC',0,NULL),
                   (2,'m3','h3','unknown-field-code','line 3: OC',0,NULL),
                   (3,'m4','h4','unknown-field-code','line 3: OC',0,123),
                   (2,'m5','h5','unparsable-xml','other',0,NULL);",
            )
            .await
            .unwrap();
        db.set_foreign_keys(true).await.unwrap();

        // The OC bucket: two held packages (1 and 2), deduped; pkg 3 is already
        // reclaimed (reprocessed_at set) and pkg 2's row m5 is a different reason.
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
    /// reasons and already-reclaimed rows.
    #[tokio::test]
    async fn held_member_files_are_the_bucket_of_one_package() {
        let path = format!("/tmp/tender-db-held-files-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        db.set_foreign_keys(false).await.unwrap();
        db.conn()
            .await
            .execute_batch(
                "INSERT INTO quarantine(fetch_id, member_path, content_hash, reason, detail, first_seen, reprocessed_at) VALUES
                   (1,'pkg/a.xml','h1','unknown-field-code','line 3: OC',0,NULL),
                   (1,'pkg/bundle.zip#0','h2','unknown-field-code','line 3: OC',0,NULL),
                   (1,'pkg/bundle.zip#1','h3','unknown-field-code','line 9: OC',0,NULL),
                   (1,'pkg/done.xml','h4','unknown-field-code','line 3: OC',0,555),
                   (1,'pkg/other.xml','h5','unclaimed-content','x',0,NULL),
                   (2,'pkg2/z.xml','h6','unknown-field-code','line 3: OC',0,NULL);",
            )
            .await
            .unwrap();
        db.set_foreign_keys(true).await.unwrap();

        let held = db.quarantine_held_member_files(1, "unknown-field-code", Some("%: OC"), None).await.unwrap();
        // a.xml + the two bundle records collapsed to the one member file; NOT the
        // reclaimed done.xml, the other-reason row, or fetch 2.
        let mut got: Vec<_> = held.into_iter().collect();
        got.sort();
        assert_eq!(got, vec!["pkg/a.xml".to_string(), "pkg/bundle.zip".to_string()]);

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
