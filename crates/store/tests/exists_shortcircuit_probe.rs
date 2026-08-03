//! Issue 117 Class B probe: can a nothing-matching `EXISTS` filter be answered by
//! one index seek instead of a per-row walk?
//!
//! `/v1/tenders?country=ZZ` exceeded 380s on prod. The filter is
//! `EXISTS (SELECT 1 FROM tender_version_classifications c WHERE c.tender_id = t.id
//! AND c.seq = v.seq AND c.scheme = 'nuts' AND c.code LIKE ?)`, evaluated per Tender
//! across 4.26M rows. No cursor shape helps: the filter is not a column of the
//! driven table.
//!
//! But `tender_version_classifications_code(scheme, code)` already exists. If NO row
//! anywhere carries the requested prefix, then no Tender can satisfy the `EXISTS`,
//! so an empty page is the CORRECT answer — reachable with a single seek rather than
//! a walk. That is the move `read::changes_since` already makes for an unknown
//! `entity_kind` (issue 61 finding 2): "a kind with no rows must never trigger a
//! table walk to discover it has none."
//!
//! This measures whether the guard is actually index-served, and what it saves.
//! It also measures the LIKE form against an explicit prefix range, because turso's
//! `LIKE` prefix optimisation is not something to assume.
//!
//! Limitation this CANNOT close, and the probe says so with a third case: the guard
//! answers "matches nothing". A prefix that exists but only on high `tender_id`s
//! still walks. So this is not a DoS defence, only a fix for the worst case.

use std::time::Instant;
use store::turso::{self, Value};

const TENDERS: i64 = 200_000;

async fn drain(conn: &turso::Connection, sql: &str) {
    let mut rows = conn.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

async fn time(conn: &turso::Connection, sql: &str, p: Vec<Value>) -> (f64, usize) {
    let mut best = f64::MAX;
    let mut n = 0;
    for run in 0..3 {
        let t = Instant::now();
        let mut rows = conn.query(sql, p.clone()).await.unwrap();
        n = 0;
        while rows.next().await.unwrap().is_some() {
            n += 1;
        }
        if run > 0 {
            best = best.min(t.elapsed().as_secs_f64());
        }
    }
    (best, n)
}

async fn plan(conn: &turso::Connection, label: &str, sql: &str, p: Vec<Value>) {
    let mut rows = conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), p).await.unwrap();
    println!("\n{label}:");
    while let Some(r) = rows.next().await.unwrap() {
        println!("  {}", r.get_value(3).unwrap().as_text().cloned().unwrap_or_default());
    }
}

#[tokio::test]
#[ignore = "probe: issue 117 Class B existence short-circuit; run with --ignored"]
async fn an_absent_prefix_can_be_answered_by_one_seek() {
    let path = format!("/tmp/tender-db-exists-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    let seeded = Instant::now();
    conn.execute("BEGIN", ()).await.unwrap();
    for i in 1..=TENDERS {
        if i % 100_000 == 0 {
            conn.execute("COMMIT", ()).await.unwrap();
            conn.execute("BEGIN", ()).await.unwrap();
        }
        conn.execute(
            "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
             VALUES (?, 'ted', ?, 'procedure', 1, 1700000000, 1700000000)",
            (Value::Integer(i), Value::Text(format!("pk-{i}"))),
        ).await.unwrap();
        conn.execute(
            "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
             VALUES (?, 1, 1700000000, ?, ?)",
            (Value::Integer(i), Value::Text(format!("pub-{i}")), Value::Integer(i)),
        ).await.unwrap();
        // Every Tender is DE except the last 20, which are MT — so 'MT' EXISTS but
        // only very late in rowid order: the "matches late" case the guard cannot fix.
        let code = if i > TENDERS - 20 { "MT001" } else { "DE300" };
        conn.execute(
            "INSERT INTO tender_version_classifications (tender_id, seq, lot_id, field, scheme, code)
             VALUES (?, 1, NULL, 'place', 'nuts', ?)",
            (Value::Integer(i), Value::Text(code.to_owned())),
        ).await.unwrap();
    }
    conn.execute("COMMIT", ()).await.unwrap();
    println!("\n{TENDERS} tenders seeded in {:.1}s", seeded.elapsed().as_secs_f64());

    // The filter as `read::tenders` builds it, reduced to the identity columns so the
    // EXISTS is what is being timed rather than the nine summary subqueries.
    const FILTERED: &str = "SELECT t.id FROM tenders t
          JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq
         WHERE 1 = 1
           AND EXISTS (SELECT 1 FROM tender_version_classifications c
                        WHERE c.tender_id = t.id AND c.seq = v.seq
                          AND c.scheme = 'nuts' AND c.code LIKE ?)
           AND t.id > ? ORDER BY t.id LIMIT 1000";

    // The proposed guard, in both forms.
    const GUARD_LIKE: &str = "SELECT 1 FROM tender_version_classifications
         WHERE scheme = 'nuts' AND code LIKE ? LIMIT 1";
    const GUARD_RANGE: &str = "SELECT 1 FROM tender_version_classifications
         WHERE scheme = 'nuts' AND code >= ? AND code < ? LIMIT 1";

    println!("\n{:<22} {:>11}  {:>6}", "case", "time", "rows");
    for (label, prefix) in
        [("dense (DE)", "DE"), ("late (MT)", "MT"), ("absent (ZZ)", "ZZ")]
    {
        let (t, n) = time(
            &conn,
            FILTERED,
            vec![Value::Text(format!("{prefix}%")), Value::Integer(0)],
        )
        .await;
        println!("filter {label:<15} {t:>10.4}s  {n:>6}");
    }
    for (label, prefix) in [("dense (DE)", "DE"), ("late (MT)", "MT"), ("absent (ZZ)", "ZZ")] {
        let (tl, nl) =
            time(&conn, GUARD_LIKE, vec![Value::Text(format!("{prefix}%"))]).await;
        let succ = {
            let mut b = prefix.as_bytes().to_vec();
            *b.last_mut().unwrap() += 1;
            String::from_utf8(b).unwrap()
        };
        let (tr, nr) = time(
            &conn,
            GUARD_RANGE,
            vec![Value::Text(prefix.to_owned()), Value::Text(succ)],
        )
        .await;
        println!("guard  {label:<15} {tl:>10.4}s  {nl:>6}   (LIKE)");
        println!("guard  {label:<15} {tr:>10.4}s  {nr:>6}   (range)");
        assert_eq!(nl, nr, "{label}: LIKE and range guards must agree");
    }

    plan(&conn, "filter, absent prefix", FILTERED, vec![Value::Text("ZZ%".into()), Value::Integer(0)])
        .await;
    plan(&conn, "guard, LIKE", GUARD_LIKE, vec![Value::Text("ZZ%".into())]).await;
    plan(
        &conn,
        "guard, range",
        GUARD_RANGE,
        vec![Value::Text("ZZ".into()), Value::Text("Z[".into())],
    )
    .await;

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
