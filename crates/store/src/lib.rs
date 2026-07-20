//! Turso (pure-Rust SQLite) persistence — server-only by construction (the app
//! crate pulls this in behind its `server` feature; it never reaches wasm).
//!
//! One embedded database file (`TENDER_DB`, default `tender-db.db` in the working
//! directory — the systemd state dir in production). Accessors live on [`Db`];
//! the process-wide instance owns a single connection behind a mutex and
//! serialises access.

pub mod accounts;
pub mod canonical;
pub mod jobs;
pub mod read;
pub mod webhooks;

/// Re-exported so callers can name `Error`/`Connection`/`Value` without taking
/// their own pin on the engine — the store owns which Turso this is.
pub use turso;

pub use accounts::{TokenRecord, User};
pub use canonical::{
    Applied, BidParty, BidState, Change, ContractState, Fact, Identifier, LotResultState, LotState,
    Mention, NoticeRef, Round, TenderProjection, TenderVersion,
};
pub use read::{Filter, Reader, Readers, Status};
pub use webhooks::{Delivery, Endpoint};

use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard, OnceCell, watch};
use turso::{Connection, Value};

/// Sane connection defaults, per <https://mort.coffee/home/sqlite-editions/>:
/// enforce foreign keys, retry on lock contention instead of failing with
/// SQLITE_BUSY, WAL for concurrent reads during writes, and NORMAL sync (safe
/// under WAL, much faster than FULL).
pub(crate) const PRAGMAS: [&str; 4] = [
    "PRAGMA foreign_keys = ON",
    "PRAGMA busy_timeout = 5000",
    "PRAGMA journal_mode = WAL",
    "PRAGMA synchronous = NORMAL",
];

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
        UNIQUE(source, publication_id, content_hash)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS notices_profile ON notices(profile);
    CREATE INDEX IF NOT EXISTS notices_parse_state ON notices(parse_state);

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
const MIGRATIONS: [&str; 3] = [
    "ALTER TABLE notices ADD COLUMN published_at INTEGER",
    "ALTER TABLE notices ADD COLUMN dispatched_at INTEGER",
    "ALTER TABLE tender_versions ADD COLUMN dispatched_at INTEGER",
];

async fn migrate(conn: &Connection) -> turso::Result<()> {
    for statement in MIGRATIONS {
        match conn.execute(statement, ()).await {
            Ok(_) => {}
            Err(e) if e.to_string().contains("duplicate column") => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
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
        Ok(Db { database, conn: Mutex::new(conn), cursor })
    }

    async fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().await
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
        max_cursor(&*self.conn().await).await
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
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT id, COALESCE(title, '(untitled)') FROM v_tenders
                 ORDER BY published_at DESC, id DESC LIMIT ?",
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
        let conn = self.conn().await;
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
        let conn = self.conn().await;
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
        let conn = self.conn().await;
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

    async fn set_parse_state(&self, conn: &Connection, id: i64, state: &str) -> turso::Result<()> {
        conn.execute("UPDATE notices SET parse_state = ? WHERE id = ?", (t(state), Value::Integer(id)))
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

    /// Notice counts per mapping profile — the era-split check and the
    /// dashboard's coverage breakdown.
    pub async fn notice_counts_by_profile(&self) -> turso::Result<Vec<(String, i64)>> {
        let conn = self.conn().await;
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
        let conn = self.conn().await;
        let mut rows = conn
            .query("SELECT reason, COUNT(*) FROM quarantine GROUP BY reason ORDER BY reason", ())
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
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT f.source, n.profile, substr(f.period, 1, 4) AS year, COUNT(*)
                   FROM notices n JOIN fetches f ON f.id = n.fetch_id
                  GROUP BY f.source, n.profile, year
                  ORDER BY year, f.source, n.profile",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(ProfileYear {
                source: text(&row, 0),
                profile: text(&row, 1),
                year: text(&row, 2),
                notices: int(&row, 3),
            });
        }
        Ok(out)
    }

    /// The newest quarantined payloads — the drill-down behind the headline
    /// count, newest first because a fresh reason is the one worth acting on.
    pub async fn recent_quarantine(&self, limit: i64) -> turso::Result<Vec<QuarantineEntry>> {
        let conn = self.conn().await;
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
        let conn = self.conn().await;
        Ok(ImportLag {
            newest_fetch_at: max_instant(&conn, "SELECT MAX(fetched_at) FROM fetches").await?,
            newest_notice_at: max_instant(&conn, "SELECT MAX(ingested_at) FROM notices").await?,
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

/// The newest cursor in the change log, 0 when it is empty. The log is
/// append-only and never renumbered, so this is a high-water mark.
pub(crate) async fn max_cursor(conn: &Connection) -> turso::Result<i64> {
    let mut rows = conn.query("SELECT COALESCE(MAX(cursor), 0) FROM changes", ()).await?;
    Ok(rows.next().await?.map_or(0, |row| int(&row, 0)))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
