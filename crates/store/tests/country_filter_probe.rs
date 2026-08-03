//! Issue 117 Class B / task 5: costing the two candidate fixes for
//! `/v1/tenders?country=`, which the existence short-circuit deliberately does not
//! cover.
//!
//! The filter is an `EXISTS` over `tender_version_classifications` evaluated PER ROW
//! across 4.26M Tenders. The short-circuit answers *matches-nothing* with one seek;
//! *matches-late* — a real country whose Tenders sit at high ids — still walks
//! (measured 0.5000s against 0.4954s for the absent case, so the guard buys nothing
//! there).
//!
//! Two candidates, costed here rather than argued:
//!
//! **B — restructure the EXISTS.** Drive from `tender_version_classifications`
//! instead of probing it per row, seeking the existing
//! `tender_version_classifications_code(scheme, code)`. No schema change.
//!
//! **A — denormalise.** A junction table keyed `(country, tender_id)`, so the filter
//! becomes an equality whose index yields `tender_id` order directly — the same
//! `(filter, id)` shape that fixed 117 Class A. Costs a schema change plus a backfill
//! over the 254 GB prod database.
//!
//! The suspicion worth testing, and the reason this is measured rather than reasoned:
//! a country filter is a PREFIX over codes, so a range covers MANY distinct codes
//! (`DE1`, `DE11`, `DE300`…). Scanning that range yields rows grouped by code, not by
//! `tender_id` — so candidate B may still need a sorter over the whole matched set
//! before `LIMIT`, which is exactly the defect 117 Class A turned out to be. If so, B
//! is not a fix at dense selectivities and the schema change is the only real option.

use std::time::Instant;
use store::turso::{self, Value};

const TENDERS: i64 = 300_000;
/// Distinct NUTS codes under one country, so a country prefix spans many of them —
/// the property that decides whether candidate B can preserve `tender_id` order.
const CODES_PER_COUNTRY: i64 = 40;

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
#[ignore = "probe: task 5 country-filter candidates; run with --ignored"]
async fn cost_the_two_country_filter_candidates() {
    let path = format!("/tmp/tender-db-country-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    // DE dominates and sits early in id order; MT is rare and sits LATE — the
    // matches-late case the short-circuit cannot help. ZZ is absent.
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
        let late = i > TENDERS - 200;
        let code = if late {
            format!("MT{:03}", i % CODES_PER_COUNTRY)
        } else {
            format!("DE{:03}", i % CODES_PER_COUNTRY)
        };
        conn.execute(
            "INSERT INTO tender_version_classifications (tender_id, seq, lot_id, field, scheme, code)
             VALUES (?, 1, NULL, 'place', 'nuts', ?)",
            (Value::Integer(i), Value::Text(code)),
        ).await.unwrap();
    }
    conn.execute("COMMIT", ()).await.unwrap();
    conn.execute(
        "CREATE INDEX IF NOT EXISTS tender_version_classifications_code
             ON tender_version_classifications(scheme, code)",
        (),
    ).await.unwrap();
    println!("\n{TENDERS} tenders, {CODES_PER_COUNTRY} codes/country, seeded in {:.1}s",
             seeded.elapsed().as_secs_f64());

    // ---- current shape: EXISTS evaluated per Tender row
    const CURRENT: &str = "SELECT t.id FROM tenders t
          JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq
         WHERE EXISTS (SELECT 1 FROM tender_version_classifications c
                        WHERE c.tender_id = t.id AND c.seq = v.seq
                          AND c.scheme = 'nuts' AND c.code LIKE ?)
           AND t.id > ? ORDER BY t.id LIMIT 50";

    // ---- candidate B: drive FROM classifications, no schema change
    const CANDIDATE_B: &str = "SELECT DISTINCT t.id FROM tender_version_classifications c
          JOIN tenders t ON t.id = c.tender_id AND c.seq = t.current_seq
         WHERE c.scheme = 'nuts' AND c.code >= ? AND c.code < ?
           AND t.id > ? ORDER BY t.id LIMIT 50";

    println!("\n{:<34} {:>11}  {:>5}", "case", "time", "rows");
    for (label, prefix) in [("dense (DE)", "DE"), ("late (MT)", "MT"), ("absent (ZZ)", "ZZ")] {
        let (t, n) = time(
            &conn,
            CURRENT,
            vec![Value::Text(format!("{prefix}%")), Value::Integer(0)],
        )
        .await;
        println!("current   {label:<22} {t:>10.4}s  {n:>5}");
    }
    for (label, prefix) in [("dense (DE)", "DE"), ("late (MT)", "MT"), ("absent (ZZ)", "ZZ")] {
        let hi = {
            let mut b = prefix.as_bytes().to_vec();
            *b.last_mut().unwrap() += 1;
            String::from_utf8(b).unwrap()
        };
        let (t, n) = time(
            &conn,
            CANDIDATE_B,
            vec![Value::Text(prefix.into()), Value::Text(hi), Value::Integer(0)],
        )
        .await;
        println!("cand B    {label:<22} {t:>10.4}s  {n:>5}");
    }

    // ---- candidate A: the denormalised junction, (country, tender_id)
    let built = Instant::now();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tender_countries (
             tender_id INTEGER NOT NULL, country TEXT NOT NULL,
             PRIMARY KEY (country, tender_id)) STRICT",
        (),
    ).await.unwrap();
    conn.execute(
        "INSERT INTO tender_countries (tender_id, country)
         SELECT DISTINCT c.tender_id, substr(c.code, 1, 2)
           FROM tender_version_classifications c
           JOIN tenders t ON t.id = c.tender_id AND c.seq = t.current_seq
          WHERE c.scheme = 'nuts'",
        (),
    ).await.unwrap();
    println!("\ntender_countries backfilled in {:.1}s", built.elapsed().as_secs_f64());

    const CANDIDATE_A: &str = "SELECT t.id FROM tenders t
          JOIN tender_countries tc ON tc.tender_id = t.id
         WHERE tc.country = ? AND t.id > ? ORDER BY t.id LIMIT 50";
    for (label, prefix) in [("dense (DE)", "DE"), ("late (MT)", "MT"), ("absent (ZZ)", "ZZ")] {
        let (t, n) =
            time(&conn, CANDIDATE_A, vec![Value::Text(prefix.into()), Value::Integer(0)]).await;
        println!("cand A    {label:<22} {t:>10.4}s  {n:>5}");
    }

    // Candidate A still sorts, which it should not have to: its PK is
    // `(country, tender_id)`, so seeking `country = ?` already yields tender_id order.
    // The planner cannot see that `t.id` and `tc.tender_id` are the same value —
    // the join condition says so, but the ORDER BY names the joined table's column.
    // Spell the ordering on the JUNCTION column and the sort should disappear.
    const CANDIDATE_A2: &str = "SELECT t.id FROM tender_countries tc
          JOIN tenders t ON t.id = tc.tender_id
         WHERE tc.country = ? AND tc.tender_id > ? ORDER BY tc.tender_id LIMIT 50";
    println!();
    for (label, prefix) in [("dense (DE)", "DE"), ("late (MT)", "MT"), ("absent (ZZ)", "ZZ")] {
        let (t, n) =
            time(&conn, CANDIDATE_A2, vec![Value::Text(prefix.into()), Value::Integer(0)]).await;
        println!("cand A2   {label:<22} {t:>10.4}s  {n:>5}");
    }
    plan(
        &conn,
        "candidate A2 (dense)",
        CANDIDATE_A2,
        vec![Value::Text("DE".into()), Value::Integer(0)],
    )
    .await;

    plan(&conn, "current (dense)", CURRENT, vec![Value::Text("DE%".into()), Value::Integer(0)]).await;
    plan(
        &conn,
        "candidate B (dense)",
        CANDIDATE_B,
        vec![Value::Text("DE".into()), Value::Text("DF".into()), Value::Integer(0)],
    )
    .await;
    plan(&conn, "candidate A (dense)", CANDIDATE_A, vec![Value::Text("DE".into()), Value::Integer(0)])
        .await;

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
