//! Issue 372, and the class of bug it exposed: a column added to `SCHEMA` reaches
//! a FRESH database through `CREATE TABLE`, and an EXISTING one only through
//! `MIGRATIONS`. Every test in this workspace builds its database fresh, so the
//! whole suite passes green on a schema change that is completely inert in
//! production — which is what happened on 2026-09-09: `quality` shipped on both
//! amount-bearing satellites, 113 suites went green, and `SELECT quality FROM
//! tender_version_amounts` on the box answered "no such column".
//!
//! So this test does the one thing the rest of the suite structurally cannot: it
//! pre-creates the satellites in their PRE-column shape through a raw connection,
//! exactly as prod carries them, and then opens the database the way the binary
//! does. `CREATE TABLE IF NOT EXISTS` leaves the old table alone, so the column
//! can only arrive via the ALTER — and if someone adds a column to `SCHEMA` and
//! forgets `MIGRATIONS` again, this is where it fails instead of on the box.

use store::Db;
use store::turso;

/// Pre-create both amount-bearing satellites WITHOUT `quality`, then open through
/// `Db::open` and assert the column is queryable on each. The prod shape, reproduced.
#[tokio::test]
async fn an_existing_database_gains_the_withheld_marker_column() {
    let path = format!("/tmp/tender-db-satcol-{}.db", std::process::id());
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }

    {
        let raw = turso::Builder::new_local(&path).build().await.unwrap();
        let c = raw.connect().unwrap();
        // The shape prod carries: every column the satellite had before issue 372,
        // and no `quality`. Foreign keys are omitted deliberately — the parent
        // tables do not exist yet at this point, and what is under test is the
        // ALTER, not referential integrity.
        c.execute(
            "CREATE TABLE tender_version_amounts (
                 tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_id INTEGER,
                 field TEXT NOT NULL, cents INTEGER NOT NULL, currency TEXT NOT NULL,
                 tax_basis TEXT, eur_cents INTEGER
             ) STRICT",
            (),
        )
        .await
        .unwrap();
        c.execute(
            "CREATE TABLE tender_version_bids (
                 tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, bid_id INTEGER NOT NULL,
                 lot_id INTEGER, cents INTEGER, currency TEXT, eur_cents INTEGER,
                 PRIMARY KEY (tender_id, seq, bid_id)
             ) STRICT",
            (),
        )
        .await
        .unwrap();
        // The statistics satellite (issue 372 unit 4), same treatment.
        c.execute(
            "CREATE TABLE tender_version_result_stats (
                 tender_id INTEGER NOT NULL, seq INTEGER NOT NULL,
                 lot_result_id INTEGER NOT NULL, kind TEXT NOT NULL, count INTEGER NOT NULL
             ) STRICT",
            (),
        )
        .await
        .unwrap();
    }

    let db = Db::open(&path).await.unwrap();

    for table in ["tender_version_amounts", "tender_version_bids", "tender_version_result_stats"] {
        // Sanity that the precondition held: an old-shaped table really did survive
        // the open. If `CREATE TABLE IF NOT EXISTS` had replaced it, this test would
        // be asserting nothing at all — it would pass on a fresh table every time.
        let sql = match db
            .scalar(&format!(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='{table}'"
            ))
            .await
            .unwrap()
        {
            Some(turso::Value::Text(s)) => s,
            other => panic!("no table sql for {table}: {other:?}"),
        };
        assert!(
            !sql.contains("Issue 372"),
            "precondition: {table} must still be the pre-column table, not a fresh one:\n{sql}",
        );

        // The ALTER ran: the column answers rather than raising "no such column".
        db.scalar(&format!("SELECT quality FROM {table} LIMIT 1"))
            .await
            .unwrap_or_else(|e| panic!("{table}.quality unreachable on an existing database: {e}"));
    }

    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}
