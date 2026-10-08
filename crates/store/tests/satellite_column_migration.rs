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
    // The view is created by the schema batch BEFORE the ALTERs run; its value columns
    // must still resolve once the migration has added them.
    db.scalar("SELECT value_cents, value_currency, value_eur_cents, title FROM v_lots LIMIT 1")
        .await
        .expect("v_lots and its value columns answer on the migrated table");

    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

/// Issue 495 unit 2: the fold's leaf descriptor (`store::canonical::LEAF_TABLES`) names
/// exactly the columns each version-keyed table has, on a FRESH database and on an
/// EXISTING one whose tables predate the 20 ALTER-added columns. The fold's INSERT, DELETE
/// and compare SELECT are generated from the descriptor, so a column added to `SCHEMA` and
/// the descriptor but forgotten in `MIGRATIONS` (issue 372's miss) fails here, and so does
/// a 15th leaf table the descriptor does not know. Every generated statement must prepare
/// on both arms.
#[tokio::test]
async fn leaf_tables_match_the_schema_fresh_and_migrated() {
    let fresh = format!("/tmp/tender-db-leafcols-fresh-{}.db", std::process::id());
    let migrated = format!("/tmp/tender-db-leafcols-migrated-{}.db", std::process::id());
    for path in [&fresh, &migrated] {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
    }

    let db = Db::open(&fresh).await.unwrap();
    assert_leaf_tables_match(&db, "fresh", true).await;
    drop(db);

    {
        let raw = turso::Builder::new_local(&migrated).build().await.unwrap();
        let c = raw.connect().unwrap();
        // Today's leaf tables WITHOUT the 20 ALTER-added columns (lib.rs MIGRATIONS and the
        // add_column calls), frozen here on 2026-10-08. Foreign keys are omitted, as in the
        // tests above: the parent tables do not exist yet, and what is under test is the
        // ALTER. Do NOT update these when a column is added: the point is that it arrives
        // through MIGRATIONS.
        for ddl in [
            "CREATE TABLE tender_version_result_winners (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_result_id INTEGER NOT NULL, organization_id INTEGER NOT NULL, PRIMARY KEY (tender_id, seq, lot_result_id, organization_id)) STRICT",
            "CREATE TABLE tender_version_result_stats (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_result_id INTEGER NOT NULL, kind TEXT NOT NULL, count INTEGER NOT NULL) STRICT",
            "CREATE TABLE tender_version_lot_results (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_result_id INTEGER NOT NULL, lot_id INTEGER, decision TEXT, reason TEXT, awarded_cents INTEGER, awarded_currency TEXT, PRIMARY KEY (tender_id, seq, lot_result_id)) STRICT",
            "CREATE TABLE tender_version_bid_parties (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, bid_id INTEGER NOT NULL, role TEXT NOT NULL, organization_id INTEGER NOT NULL, mention_notice_id INTEGER NOT NULL, mention_section_id TEXT NOT NULL) STRICT",
            "CREATE TABLE tender_version_bids (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, bid_id INTEGER NOT NULL, lot_id INTEGER, cents INTEGER, currency TEXT, PRIMARY KEY (tender_id, seq, bid_id)) STRICT",
            "CREATE TABLE tender_version_contracts (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, contract_id INTEGER NOT NULL, buyer_contract_id TEXT, concluded_utc INTEGER, concluded_offset INTEGER, concluded_has_time INTEGER, cents INTEGER, currency TEXT, PRIMARY KEY (tender_id, seq, contract_id)) STRICT",
            "CREATE TABLE tender_version_parties (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_id INTEGER, role TEXT NOT NULL, organization_id INTEGER NOT NULL, mention_notice_id INTEGER NOT NULL, mention_section_id TEXT NOT NULL) STRICT",
            "CREATE TABLE tender_version_texts (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_id INTEGER, field TEXT NOT NULL, lang TEXT, value TEXT NOT NULL) STRICT",
            "CREATE TABLE tender_version_dates (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_id INTEGER, field TEXT NOT NULL, utc_seconds INTEGER NOT NULL, offset_minutes INTEGER NOT NULL, has_time INTEGER NOT NULL) STRICT",
            "CREATE TABLE tender_version_amounts (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_id INTEGER, field TEXT NOT NULL, cents INTEGER NOT NULL, currency TEXT NOT NULL) STRICT",
            "CREATE TABLE tender_version_classifications (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_id INTEGER, field TEXT NOT NULL, scheme TEXT NOT NULL, code TEXT NOT NULL) STRICT",
            "CREATE TABLE tender_version_lot_group_members (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, group_lot_id INTEGER NOT NULL, member_lot_id INTEGER NOT NULL, PRIMARY KEY (tender_id, seq, group_lot_id, member_lot_id)) STRICT",
            "CREATE TABLE tender_version_lots (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, lot_id INTEGER NOT NULL, kind TEXT NOT NULL, PRIMARY KEY (tender_id, seq, lot_id)) STRICT",
            "CREATE TABLE tender_versions (tender_id INTEGER NOT NULL, seq INTEGER NOT NULL, caused_by_notice_id INTEGER NOT NULL, published_at INTEGER NOT NULL, notice_subtype TEXT, publication_id TEXT NOT NULL, PRIMARY KEY (tender_id, seq), UNIQUE (tender_id, caused_by_notice_id)) STRICT",
        ] {
            c.execute(ddl, ()).await.unwrap_or_else(|e| panic!("{ddl}: {e}"));
        }
    }
    let db = Db::open(&migrated).await.unwrap();
    assert_leaf_tables_match(&db, "migrated", false).await;
    drop(db);

    for path in [&fresh, &migrated] {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
    }
}

/// Every statement read to its end, on a connection of its own (a partly-read statement can
/// leave a snapshot on a pooled reader: docs/research/turso-scale.md).
async fn assert_leaf_tables_match(db: &Db, arm: &str, fresh: bool) {
    use std::collections::BTreeSet;
    use store::canonical::{LEAF_COUNT, LEAF_TABLES};
    let pool = db.readers(1).unwrap();
    let conn = pool.get().await.unwrap();
    async fn texts(conn: &turso::Connection, sql: &str, col: usize) -> Vec<String> {
        let mut rows = conn.query(sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.unwrap() {
            match row.get_value(col) {
                Ok(turso::Value::Text(t)) => out.push(t),
                other => panic!("{sql}: column {col} is {other:?}"),
            }
        }
        out
    }
    // GLOB, not LIKE: LIKE's `_` is a wildcard.
    let tables: BTreeSet<String> = texts(
        &conn,
        "SELECT name FROM sqlite_master WHERE type = 'table' AND (name = 'tender_versions' OR name GLOB 'tender_version_*')",
        0,
    )
    .await
    .into_iter()
    .collect();
    let described: BTreeSet<String> = LEAF_TABLES.iter().map(|t| t.name.to_owned()).collect();
    assert_eq!(tables, described, "{arm}: the version-keyed tables are exactly the descriptor's {LEAF_COUNT}");
    for t in LEAF_TABLES {
        let ddl = texts(&conn, &format!("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = '{}'", t.name), 0).await;
        // The precondition: the migrated arm's tables are still the frozen ones above (no
        // FKs), not fresh ones `CREATE TABLE IF NOT EXISTS` made. Every fresh leaf table
        // carries a REFERENCES clause, so the check means something on both arms.
        assert_eq!(
            ddl[0].contains("REFERENCES"),
            fresh,
            "{arm}: {} is not the table this arm meant to test:\n{}",
            t.name,
            ddl[0]
        );
        let have: BTreeSet<String> = texts(&conn, &format!("PRAGMA table_info(\"{}\")", t.name), 1).await.into_iter().collect();
        let want: BTreeSet<String> = t.cols.iter().map(|c| (*c).to_owned()).collect();
        assert_eq!(have, want, "{arm}: {}'s columns against the descriptor", t.name);
        let row = vec!["?"; t.ncols()].join(", ");
        for sql in [format!("{}({row})", t.insert_prefix()), t.delete_version_sql(), t.compare_select_sql()] {
            conn.prepare(&sql).await.unwrap_or_else(|e| panic!("{arm}: {sql}: {e}"));
        }
    }
}
