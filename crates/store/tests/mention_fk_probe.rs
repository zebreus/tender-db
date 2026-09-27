//! Issue 441: deleting a mention proves no party row still references it — by a seek.
//!
//! `tender_version_parties` and `tender_version_bid_parties` carry `FOREIGN KEY
//! (mention_notice_id, mention_section_id) REFERENCES organization_mentions`, so every
//! mention DELETE on the writer (foreign keys ON) runs a child probe per party table.
//! `turso_core` 0.7.2 serves that probe from a child index only when the index's
//! columns EQUAL the FK's child columns (`translate/fkeys.rs`
//! `emit_fk_parent_key_probe`); anything else — including the one-column
//! `(mention_notice_id)` prefix the tables carried — falls back to `Rewind` over the
//! whole table. On prod that was ~2.2 s per deleted mention over 78M rows, and a
//! re-parse that drops a mentioned section (job 1596, issue 393's
//! `TRANSLITERATED_ADDR` twins) crawled at ~1.3 notices a second on it.
//!
//! These read the program turso compiles, not a timing: a scan is a `Rewind` opcode
//! over the party table, a seek is a `Found` probe on the `_mention_key` index.

use store::turso;

async fn open(name: &str) -> (store::Db, String) {
    let path = format!("/tmp/tender-db-mentionfk-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    (store::Db::open(&path).await.unwrap(), path)
}

/// A connection like the writer's: foreign keys ON (a fresh turso connection has
/// them off, and then the probe is not compiled at all).
async fn writer_like(path: &str) -> turso::Connection {
    let conn = turso::Builder::new_local(path).build().await.unwrap().connect().unwrap();
    let mut q = conn.query("PRAGMA foreign_keys=ON", ()).await.unwrap();
    while q.next().await.unwrap().is_some() {}
    conn
}

/// `(opcode, comment)` per instruction of `sql`'s compiled program.
async fn program(conn: &turso::Connection, sql: &str) -> Vec<(String, String)> {
    let mut rows = conn.query(&format!("EXPLAIN {sql}"), ()).await.unwrap();
    let mut out = Vec::new();
    while let Some(r) = rows.next().await.unwrap() {
        let text = |i| r.get_value(i).ok().and_then(|v| v.as_text().cloned()).unwrap_or_default();
        out.push((text(1), text(7)));
    }
    out
}

async fn indexes(conn: &turso::Connection) -> Vec<String> {
    let mut rows = conn
        .query("SELECT name FROM sqlite_schema WHERE type = 'index' ORDER BY name", ())
        .await
        .unwrap();
    let mut out = Vec::new();
    while let Some(r) = rows.next().await.unwrap() {
        out.push(r.get_value(0).unwrap().as_text().cloned().unwrap());
    }
    out
}

#[tokio::test]
async fn a_mention_delete_proves_its_foreign_keys_by_index() {
    let (db, path) = open("probe").await;
    db.build_tender_indexes().await.unwrap();
    let conn = writer_like(&path).await;

    let ops = program(&conn, "DELETE FROM organization_mentions WHERE notice_id = 1 AND section_id = 'ORG-1'").await;
    for table in ["tender_version_parties", "tender_version_bid_parties"] {
        let probe = format!("={table}_mention_key,");
        assert!(
            ops.iter().any(|(op, c)| op == "OpenRead" && c.contains(&probe)),
            "the FK probe into {table} opens its `_mention_key` index: {ops:#?}"
        );
        assert!(
            !ops.iter().any(|(op, c)| op == "Rewind" && c.trim_end().ends_with(table)),
            "no full walk of {table} to prove one mention unreferenced: {ops:#?}"
        );
    }
    assert!(
        ops.iter().filter(|(op, _)| op == "FkCounter").count() >= 2,
        "the proof is still compiled for both party tables — enforcement is on, not off: {ops:#?}"
    );
}

/// The re-parse clears a notice's party rows by `mention_notice_id` alone, and the
/// wider key must keep serving that prefix — the narrow index it replaces existed
/// for exactly this DELETE (issue 100).
#[tokio::test]
async fn the_notice_wide_party_delete_still_seeks() {
    let (db, path) = open("prefix").await;
    db.build_tender_indexes().await.unwrap();
    let conn = writer_like(&path).await;
    for table in ["tender_version_parties", "tender_version_bid_parties"] {
        let ops = program(&conn, &format!("DELETE FROM {table} WHERE mention_notice_id = 1")).await;
        assert!(
            ops.iter().any(|(op, _)| op == "SeekGE") && !ops.iter().any(|(op, _)| op == "Rewind"),
            "DELETE FROM {table} by notice seeks the key's prefix: {ops:#?}"
        );
    }
}

/// A box that built the narrow pair (prod) loses it once the replacement exists —
/// never before, so the notice-wide DELETE is never left without an index.
#[tokio::test]
async fn the_narrow_mention_indexes_are_retired_once_replaced() {
    let (db, path) = open("retire").await;
    let conn = writer_like(&path).await;
    for (name, cols) in [
        ("tender_version_parties_mention", "tender_version_parties(mention_notice_id)"),
        ("tender_version_bid_parties_mention", "tender_version_bid_parties(mention_notice_id)"),
    ] {
        conn.execute(&format!("CREATE INDEX {name} ON {cols}"), ()).await.unwrap();
    }
    drop(conn);

    db.build_tender_indexes().await.unwrap();

    let conn = writer_like(&path).await;
    let names = indexes(&conn).await;
    for table in ["tender_version_parties", "tender_version_bid_parties"] {
        assert!(names.contains(&format!("{table}_mention_key")), "{table}'s full-key index is built: {names:?}");
        assert!(!names.contains(&format!("{table}_mention")), "{table}'s narrow index is retired: {names:?}");
    }

    // And a rebuild's strip takes the retired names too, so a box that still had
    // them never maintains them through a from-scratch fold.
    conn.execute("CREATE INDEX tender_version_parties_mention ON tender_version_parties(mention_notice_id)", ())
        .await
        .unwrap();
    drop(conn);
    db.strip_tender_indexes().await.unwrap();
    let names = indexes(&writer_like(&path).await).await;
    assert!(
        !names.iter().any(|n| n.starts_with("tender_version_parties_mention")),
        "the strip drops the retired index with the deferred ones: {names:?}"
    );
}
