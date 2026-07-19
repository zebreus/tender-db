//! Turso (pure-Rust SQLite) persistence — server-only by construction (the app
//! crate pulls this in behind its `server` feature; it never reaches wasm).
//!
//! One embedded database file (`TENDER_DB`, default `tender-db.db` in the working
//! directory — the systemd state dir in production). Accessors live on [`Db`];
//! the process-wide instance owns a single connection behind a mutex and
//! serialises access.

use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard, OnceCell};
use turso::{Connection, Value};

/// Sane connection defaults, per <https://mort.coffee/home/sqlite-editions/>:
/// enforce foreign keys, retry on lock contention instead of failing with
/// SQLITE_BUSY, WAL for concurrent reads during writes, and NORMAL sync (safe
/// under WAL, much faster than FULL).
const PRAGMAS: [&str; 4] = [
    "PRAGMA foreign_keys = ON",
    "PRAGMA busy_timeout = 5000",
    "PRAGMA journal_mode = WAL",
    "PRAGMA synchronous = NORMAL",
];

/// Schema, applied idempotently at startup. STRICT so columns actually enforce
/// their declared types.
const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS tenders (
        id    INTEGER PRIMARY KEY,
        title TEXT NOT NULL
    ) STRICT;

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
        UNIQUE(source, publication_id, content_hash)
    ) STRICT;
    CREATE INDEX IF NOT EXISTS notices_profile ON notices(profile);

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
";

pub struct Db {
    conn: Mutex<Connection>,
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
        Ok(Db { conn: Mutex::new(conn) })
    }

    async fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().await
    }

    /// All tenders, newest first.
    pub async fn list_tenders(&self) -> turso::Result<Vec<model::Tender>> {
        let conn = self.conn().await;
        let mut rows = conn.query("SELECT id, title FROM tenders ORDER BY id DESC", ()).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            if let (Ok(Value::Integer(id)), Ok(Value::Text(title))) = (row.get_value(0), row.get_value(1)) {
                out.push(model::Tender { id, title });
            }
        }
        Ok(out)
    }

    pub async fn insert_tender(&self, title: &str) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute("INSERT INTO tenders(title) VALUES(?)", (Value::Text(title.into()),)).await?;
        Ok(())
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

    /// Record a Notice. Returns false when this identity is already known —
    /// which is what makes re-processing a package idempotent.
    pub async fn insert_notice(&self, n: &Notice) -> turso::Result<bool> {
        let conn = self.conn().await;
        let changed = conn
            .execute(
                "INSERT OR IGNORE INTO notices(source, publication_id, content_hash, profile,
                     declared_version, fetch_id, member_path, ingested_at)
                 VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    t(&n.source),
                    t(&n.publication_id),
                    t(&n.content_hash),
                    t(&n.profile),
                    opt_text(n.declared_version.as_deref()),
                    Value::Integer(n.fetch_id),
                    t(&n.member_path),
                    Value::Integer(n.ingested_at),
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

fn t(s: impl Into<String>) -> Value {
    Value::Text(s.into())
}

fn opt_text(s: Option<&str>) -> Value {
    s.map_or(Value::Null, |s| Value::Text(s.into()))
}

fn text(row: &turso::Row, idx: usize) -> String {
    match row.get_value(idx) {
        Ok(Value::Text(s)) => s,
        _ => String::new(),
    }
}

fn int(row: &turso::Row, idx: usize) -> i64 {
    match row.get_value(idx) {
        Ok(Value::Integer(i)) => i,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Exercises the pragmas, the STRICT schema, and a tender round-trip against a
    // real Turso db file.
    #[tokio::test]
    async fn round_trips() {
        let path = format!("/tmp/tender-db-test-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path).await.unwrap();
        assert!(db.list_tenders().await.unwrap().is_empty());

        db.insert_tender("Road resurfacing, district 4").await.unwrap();
        db.insert_tender("School canteen catering 2027").await.unwrap();

        let tenders = db.list_tenders().await.unwrap();
        assert_eq!(tenders.len(), 2);
        // Newest first.
        assert_eq!(tenders[0].title, "School canteen catering 2027");

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
