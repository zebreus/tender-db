//! Turso (pure-Rust SQLite) persistence — server feature only.
//!
//! One embedded database file (`TENDER_DB`, default `tender-db.db` in the working
//! directory — the systemd state dir in production). Accessors live on [`Db`];
//! the process-wide instance owns a single connection behind a mutex and
//! serialises access.
#![cfg(feature = "server")]

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
    pub async fn list_tenders(&self) -> turso::Result<Vec<crate::api::Tender>> {
        let conn = self.conn().await;
        let mut rows = conn.query("SELECT id, title FROM tenders ORDER BY id DESC", ()).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            if let (Ok(Value::Integer(id)), Ok(Value::Text(title))) = (row.get_value(0), row.get_value(1)) {
                out.push(crate::api::Tender { id, title });
            }
        }
        Ok(out)
    }

    pub async fn insert_tender(&self, title: &str) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute("INSERT INTO tenders(title) VALUES(?)", (Value::Text(title.into()),)).await?;
        Ok(())
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
}
