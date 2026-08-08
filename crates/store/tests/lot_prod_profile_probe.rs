//! Scratch probe: issue 115's fix against run-driver's PROD slice profile (tender
//! 7161565 @ seq 9), not the synthetic seed in `lot_summary_cost.rs`.
//!
//! Prod's profile differs in ways that matter:
//!   * texts are lang `FR` only, ~2 rows/lot — so the old title subquery's
//!     `ORDER BY (lang = 'ENG') DESC` never discriminated and always walked;
//!   * every `tender_version_amounts` row has `lot_id IS NULL` (299 rows) — so both
//!     old amount subqueries walked all 299 rows per lot and found nothing;
//!   * `tender_version_dates` is 2 rows TOTAL.
//! So the 5,516-row texts slice is essentially the whole cost.
//!
//! Answers two questions: does the curve flatten on this profile, and does `LIMIT`
//! now truncate before the work (run-driver's sorter question)?

use std::time::Instant;
use store::read::{self, Filter, Scope};
use store::turso::{self, Value};

async fn drain(conn: &turso::Connection, sql: &str) {
    let mut rows = conn.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

/// One Tender with `lots` Lots, satellite slices scaled to prod's measured ratios.
async fn seed(conn: &turso::Connection, tender: i64, lots: i64) {
    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
         VALUES (?, 'ted', ?, 'procedure', 9, 1700000000, 1700000000)",
        (Value::Integer(tender), Value::Text(format!("pk-{tender}"))),
    ).await.unwrap();
    for seq in 1..=9 {
        conn.execute(
            "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
             VALUES (?, ?, 1700000000, ?, ?)",
            (Value::Integer(tender), Value::Integer(seq),
             Value::Text(format!("pub-{tender}-{seq}")), Value::Integer(tender * 100 + seq)),
        ).await.unwrap();
    }

    conn.execute("BEGIN", ()).await.unwrap();
    for i in 0..lots {
        conn.execute(
            "INSERT INTO lots (tender_id, lot_key) VALUES (?, ?)",
            (Value::Integer(tender), Value::Text(format!("LOT-{i:04}"))),
        )
        .await
        .unwrap();
    }
    conn.execute(
        "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind)
         SELECT tender_id, 9, id, 'Lot' FROM lots WHERE tender_id = ?",
        (Value::Integer(tender),),
    ).await.unwrap();
    // texts: one FR title + one FR description per lot (prod: 2,904 + 2,612 for
    // 2,604 lots), plus the lot-NULL Tender-level rows (300 + 8).
    for (field, extra) in [("title", 300i64), ("description", 8)] {
        conn.execute(
            "INSERT INTO tender_version_texts (tender_id, seq, lot_id, field, lang, value)
             SELECT tender_id, 9, id, ?, 'FR', ? || id FROM lots WHERE tender_id = ?",
            (Value::Text(field.to_owned()), Value::Text(format!("{field} ")), Value::Integer(tender)),
        ).await.unwrap();
        for k in 0..(extra * lots / 2604).max(1) {
            conn.execute(
                "INSERT INTO tender_version_texts (tender_id, seq, lot_id, field, lang, value)
                 VALUES (?, 9, NULL, ?, 'FR', ?)",
                (Value::Integer(tender), Value::Text(field.to_owned()),
                 Value::Text(format!("tender-level {field} {k}"))),
            ).await.unwrap();
        }
    }
    // amounts: 299 rows, EVERY ONE lot_id IS NULL.
    for k in 0..(299 * lots / 2604).max(1) {
        conn.execute(
            "INSERT INTO tender_version_amounts (tender_id, seq, lot_id, field, cents, currency)
             VALUES (?, 9, NULL, ?, ?, 'EUR')",
            (Value::Integer(tender),
             Value::Text(if k == 0 { "estimated_value" } else { "result_value" }.to_owned()),
             Value::Integer(1000 + k)),
        ).await.unwrap();
    }
    // dates: 2 rows total, Tender-level.
    for (field, secs) in [("opening_date", 1_800_000_000i64), ("submission_deadline", 1_800_009_000)] {
        conn.execute(
            "INSERT INTO tender_version_dates
                 (tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time)
             VALUES (?, 9, NULL, ?, ?, 120, 1)",
            (Value::Integer(tender), Value::Text(field.to_owned()), Value::Integer(secs)),
        ).await.unwrap();
    }
    conn.execute("COMMIT", ()).await.unwrap();
}

async fn time_read(conn: &turso::Connection, tender: i64, limit: i64) -> (f64, usize) {
    let filter = Filter { tender: Some(tender), ..Filter::default() };
    let mut best = f64::MAX;
    let mut n = 0;
    for run in 0..4 {
        let t = Instant::now();
        let rows = read::lots(conn, &filter, Scope::Page { after: 0, limit }).await.unwrap();
        let secs = t.elapsed().as_secs_f64();
        n = rows.len();
        if run > 0 {
            best = best.min(secs);
        }
    }
    (best, n)
}

#[tokio::test]
#[ignore = "probe: prod-profile curve and LIMIT sweep; run with --ignored"]
async fn prod_profile_curve_and_limit_sweep() {
    let path = format!("/tmp/tender-db-lotprod-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    seed(&conn, 1, 651).await;
    seed(&conn, 2, 1302).await;
    seed(&conn, 3, 2604).await;

    println!("\n--- curve (LIMIT 1000, as the endpoint serves) ---");
    let mut prev = 0.0;
    let mut worst = 0.0f64;
    for (tender, lots) in [(1i64, 651), (2, 1302), (3, 2604)] {
        let (secs, n) = time_read(&conn, tender, 1000).await;
        let ratio = if prev > 0.0 { secs / prev } else { 0.0 };
        println!("{lots:>5} lots: {secs:.4}s ({n} rows){}",
            if ratio > 0.0 { format!("  {ratio:.1}x vs previous (2x the lots)") } else { String::new() });
        worst = worst.max(ratio);
        prev = secs;
    }

    println!("\n--- LIMIT sweep at 2604 lots (does LIMIT truncate before the work?) ---");
    let mut sweep = Vec::new();
    for limit in [125i64, 250, 500, 1000, 3000] {
        let (secs, n) = time_read(&conn, 3, limit).await;
        println!("LIMIT {limit:>4}: {secs:.4}s ({n} rows)");
        sweep.push((limit, secs, n));
    }

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }

    // Doubling the lots must roughly double the time. Pre-fix this step was ~4x.
    assert!(
        worst < 3.0,
        "2x the lots cost {worst:.1}x the time on prod's slice profile — the curve has \
         not flattened (issue 115)."
    );

    // The sorter still consumes the whole set before LIMIT — that did not change, and
    // does not need to: it now sorts identity-only rows. What matters is the
    // CONSEQUENCE, which is that serving a Tender's every lot is no more expensive
    // than serving a page of it. That is the measurement issue 116 rests on, so assert
    // it here rather than leave it as an observation.
    let smallest = sweep.first().expect("swept").1;
    let (_, whole_set, rows) = *sweep.last().expect("swept");
    assert_eq!(rows, 2604, "the last sweep step must return the Tender's whole set");
    assert!(
        whole_set < smallest * 2.0,
        "serving all 2,604 lots cost {whole_set:.4}s against {smallest:.4}s for 125 — \
         returning the whole set is supposed to be nearly free after issue 115, which \
         is what unblocks issue 116."
    );
}
