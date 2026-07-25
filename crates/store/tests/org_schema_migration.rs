//! Issue 62: the Organization identity uniqueness moved from an inline
//! `UNIQUE(country, identifier_kind, identifier)` constraint (which SQLite backs
//! with an implicit auto-index) to a NAMED `organizations_identity` index, so a
//! full-rebuild projection can drop it, bulk-load bare, and rebuild it once. This
//! pins down the two end-states the migration must reach:
//!
//!   * a FRESH DB gets the named index and NO inline UNIQUE, and
//!   * an EXISTING prod DB — created with the old inline UNIQUE — is reconciled
//!     to the same bare-table + named-index shape by one rebuild cycle
//!     (strip_organization_indexes → build_organization_indexes), dropping the
//!     stale auto-index the inline UNIQUE left behind.

use store::turso::{self};
use store::Db;

/// The `sql` text SQLite recorded for a table (its original DDL) — we check it
/// for the inline `UNIQUE`, which is exactly what leaves a stale auto-index.
async fn table_sql(db: &Db, table: &str) -> String {
    match db
        .scalar(&format!("SELECT sql FROM sqlite_master WHERE type='table' AND name='{table}'"))
        .await
        .unwrap()
    {
        Some(turso::Value::Text(s)) => s,
        other => panic!("no table sql for {table}: {other:?}"),
    }
}

/// The `sql` DDL SQLite recorded for an index — we check it for `UNIQUE`.
async fn index_sql(db: &Db, name: &str) -> String {
    match db
        .scalar(&format!("SELECT sql FROM sqlite_master WHERE type='index' AND name='{name}'"))
        .await
        .unwrap()
    {
        Some(turso::Value::Text(s)) => s,
        other => panic!("no index sql for {name}: {other:?}"),
    }
}

async fn index_exists(db: &Db, name: &str) -> bool {
    matches!(
        db.scalar(&format!("SELECT 1 FROM sqlite_master WHERE type='index' AND name='{name}'"))
            .await
            .unwrap(),
        Some(turso::Value::Integer(1))
    )
}

/// A fresh DB is created straight into the bare-table + named-index shape.
#[tokio::test]
async fn fresh_db_has_named_index_and_no_inline_unique() {
    let path = format!("/tmp/tender-db-orgschema-fresh-{}.db", std::process::id());
    let _ = std::fs::remove_file(&path);
    let db = Db::open(&path).await.unwrap();

    assert!(
        !table_sql(&db, "organizations").await.to_uppercase().contains("UNIQUE"),
        "fresh organizations table must carry no inline UNIQUE constraint"
    );
    // organizations_identity is NOT built at open (that would be a CREATE-INDEX-at-
    // scale on an existing prod DB — it hung startup); it is built by the first
    // projection's build_organization_indexes. On a fresh (bare, no inline UNIQUE)
    // table that build is cheap and produces the named index.
    assert!(!index_exists(&db, "organizations_identity").await, "identity index not built at open");
    db.build_organization_indexes().await.unwrap();
    assert!(index_exists(&db, "organizations_identity").await, "identity index built by a projection");
    assert!(index_exists(&db, "organization_mentions_org").await, "mentions-org index present");
    // Issue 62: the identity index is PLAIN (non-unique). Org identity is
    // deduplicated in RAM (org_of), so nothing needs a DB UNIQUE constraint — and a
    // UNIQUE build over the NULLable identity columns HANGS at ~30M orgs (the resume
    // cutover reaches this over a bare-but-full org table). A plain index serves the
    // lookup without that pathology; guard against a regression back to UNIQUE.
    assert!(
        !index_sql(&db, "organizations_identity").await.to_uppercase().contains("UNIQUE"),
        "organizations_identity must be a PLAIN index (a UNIQUE build over 30M orgs hangs prod)"
    );

    let _ = std::fs::remove_file(&path);
}

/// An existing prod DB created with the old inline UNIQUE is reconciled by one
/// strip → build cycle (what a rebuild:true projection runs) to the same shape as
/// a fresh DB — the stale inline UNIQUE (and its auto-index) gone.
#[tokio::test]
async fn existing_inline_unique_db_is_reconciled_by_a_rebuild() {
    let path = format!("/tmp/tender-db-orgschema-existing-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }

    // Pre-create the OLD-schema organizations table (inline UNIQUE → auto-index)
    // via a raw connection, so the subsequent Db::open's `CREATE TABLE IF NOT
    // EXISTS` leaves it in place — exactly an existing prod DB's shape.
    {
        let raw = turso::Builder::new_local(&path).build().await.unwrap();
        let c = raw.connect().unwrap();
        c.execute(
            "CREATE TABLE organizations (
                 id INTEGER PRIMARY KEY AUTOINCREMENT, country TEXT, identifier_kind TEXT,
                 identifier TEXT, name TEXT NOT NULL, provisional INTEGER NOT NULL,
                 created_at INTEGER NOT NULL,
                 UNIQUE(country, identifier_kind, identifier)
             ) STRICT",
            (),
        )
        .await
        .unwrap();
    }

    let db = Db::open(&path).await.unwrap();
    // Sanity: it really is the legacy shape — inline UNIQUE still on the table.
    assert!(
        table_sql(&db, "organizations").await.to_uppercase().contains("UNIQUE"),
        "precondition: the simulated existing DB carries the old inline UNIQUE"
    );

    // One rebuild cycle: recreate bare, then build the indexes once.
    db.strip_organization_indexes().await.unwrap();
    db.build_organization_indexes().await.unwrap();

    assert!(
        !table_sql(&db, "organizations").await.to_uppercase().contains("UNIQUE"),
        "after a rebuild the inline UNIQUE (and its auto-index) must be gone"
    );
    assert!(index_exists(&db, "organizations_identity").await, "named identity index rebuilt");
    assert!(index_exists(&db, "organization_mentions_org").await, "mentions-org index rebuilt");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
