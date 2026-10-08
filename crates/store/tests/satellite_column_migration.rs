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

/// Issue 484 unit 3, the same trap pinned for the winner flag: a prod-shaped
/// `tender_version_result_winners` (no `is_buyer`) gains the column at open, and the
/// two award views that read it (`v_lot_results.winner_is_buyer`, `v_awards`) answer —
/// the views are created by the schema batch BEFORE the migration runs, so this also
/// proves that order holds on an existing database.
#[tokio::test]
async fn an_existing_database_gains_the_winner_is_buyer_column() {
    let path = format!("/tmp/tender-db-satcol-isbuyer-{}.db", std::process::id());
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
    {
        let raw = turso::Builder::new_local(&path).build().await.unwrap();
        let c = raw.connect().unwrap();
        c.execute(
            "CREATE TABLE tender_version_result_winners (
                 tender_id INTEGER NOT NULL, seq INTEGER NOT NULL,
                 lot_result_id INTEGER NOT NULL, organization_id INTEGER NOT NULL,
                 PRIMARY KEY (tender_id, seq, lot_result_id, organization_id)
             ) STRICT",
            (),
        )
        .await
        .unwrap();
        c.execute("INSERT INTO tender_version_result_winners VALUES (1, 1, 10, 7)", ()).await.unwrap();
    }

    let db = Db::open(&path).await.unwrap();
    let sql = match db
        .scalar("SELECT sql FROM sqlite_master WHERE type='table' AND name='tender_version_result_winners'")
        .await
        .unwrap()
    {
        Some(turso::Value::Text(s)) => s,
        other => panic!("no table sql: {other:?}"),
    };
    assert!(!sql.contains("Issue 484"), "precondition: still the pre-column table:\n{sql}");
    // The pre-existing row reads NULL: "not judged", served and counted as before.
    assert_eq!(
        db.scalar("SELECT COUNT(*) FROM tender_version_result_winners WHERE is_buyer IS NULL").await.unwrap(),
        Some(turso::Value::Integer(1)),
    );
    for view in ["v_lot_results", "v_awards"] {
        db.scalar(&format!("SELECT winner_is_buyer FROM {view} LIMIT 1"))
            .await
            .unwrap_or_else(|e| panic!("{view}.winner_is_buyer unreachable on an existing database: {e}"));
    }

    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

/// Issue 490, the same trap for the stored lot value: a prod-shaped
/// `tender_version_lots` (no `value_*`) gains the three columns at open, the
/// pre-existing row reads NULL in all three (the epoch-4 refold fills it), and the
/// fold's seven-column INSERT then lands on the migrated table.
#[tokio::test]
async fn an_existing_database_gains_the_lot_value_columns() {
    let path = format!("/tmp/tender-db-satcol-lotvalue-{}.db", std::process::id());
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
    {
        let raw = turso::Builder::new_local(&path).build().await.unwrap();
        let c = raw.connect().unwrap();
        c.execute(
            "CREATE TABLE tender_version_lots (
                 tender_id INTEGER NOT NULL, seq INTEGER NOT NULL,
                 lot_id INTEGER NOT NULL, kind TEXT NOT NULL,
                 PRIMARY KEY (tender_id, seq, lot_id)
             ) STRICT",
            (),
        )
        .await
        .unwrap();
        c.execute("INSERT INTO tender_version_lots VALUES (1, 1, 10, 'Lot')", ()).await.unwrap();
    }

    let db = Db::open(&path).await.unwrap();
    let sql = match db
        .scalar("SELECT sql FROM sqlite_master WHERE type='table' AND name='tender_version_lots'")
        .await
        .unwrap()
    {
        Some(turso::Value::Text(s)) => s,
        other => panic!("no table sql: {other:?}"),
    };
    assert!(!sql.contains("Issue 490"), "precondition: still the pre-column table:\n{sql}");
    assert_eq!(
        db.scalar(
            "SELECT COUNT(*) FROM tender_version_lots
              WHERE value_cents IS NULL AND value_currency IS NULL AND value_eur_cents IS NULL"
        )
        .await
        .unwrap(),
        Some(turso::Value::Integer(1)),
        "the pre-existing row reads NULL until the refold reaches it"
    );
    db.scalar(
        "INSERT INTO tender_version_lots(tender_id, seq, lot_id, kind, value_cents, value_currency, value_eur_cents)
         VALUES (1, 2, 10, 'Lot', 500, 'EUR', 500) RETURNING value_eur_cents",
    )
    .await
    .expect("the fold's seven-column insert lands on the migrated table");
    db.scalar("SELECT title FROM v_lots LIMIT 1").await.expect("v_lots still answers on the migrated table");

    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}
