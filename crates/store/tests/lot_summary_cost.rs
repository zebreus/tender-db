//! Issue 115 falsifier: is a tender-scoped lots read QUADRATIC in the Tender's
//! lot count?
//!
//! It was, twice over, for the same underlying reason: turso would seek a
//! `(tender_id, seq)` index prefix and then WALK the whole slice it landed in,
//! rather than use the remaining key columns.
//!
//!  * the six correlated summary subqueries — the satellites carry
//!    `(tender_id, seq)` indexes and nothing on `lot_id`, so each subquery walked
//!    that version's entire slice to find one lot's rows;
//!  * the `tender_version_lots` join — its PRIMARY KEY covers the join predicate
//!    `(tender_id, seq, lot_id)` exactly, and turso still walked the slice.
//!
//! One walk per lot, and the slice grows with the lot count, so both are
//! O(lots x slice) = O(lots^2). The fix drives the containment question from
//! `tender_version_lots` and reads each satellite once per version.
//!
//! The regression test states that as a scaling RATIO, which is robust to machine
//! speed in a way a wall-clock threshold is not: 4x the lots must cost about 4x,
//! not 16x. Measured 16.1x before the fix and 4.6x after.
//!
//! Run: `cargo test -p store --test lot_summary_cost -- --ignored --nocapture`

use std::time::Instant;
use store::read::{self, Filter, Scope};
use store::turso::{self, Value};

const SMALL: i64 = 600;
const LARGE: i64 = 2_400; // 4x SMALL; prod's worst Tender carries 2,604 lots.

/// Ratio the per-lot cost may grow by when the lot count grows 4x. Linear work
/// gives ~4 (four times the rows). Quadratic gives ~16. Anything under this is
/// unambiguously not quadratic, with room for noise.
const MAX_RATIO: f64 = 8.0;

async fn drain(conn: &turso::Connection, sql: &str) {
    let mut rows = conn.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

/// One Tender at seq 1 carrying `lots` Lots, each with the satellite rows a real
/// notice writes: a title in two languages, a description, a value, a deadline
/// and one other date. `base` offsets the lot ids so two Tenders can share a db.
async fn seed(conn: &turso::Connection, tender: i64, lots: i64) {
    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
         VALUES (?, 'ted', ?, 'procedure', 1, 1700000000, 1700000000)",
        (Value::Integer(tender), Value::Text(format!("pk-{tender}"))),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
         VALUES (?, 1, 1700000000, ?, ?)",
        (Value::Integer(tender), Value::Text(format!("pub-{tender}")), Value::Integer(tender)),
    )
    .await
    .unwrap();

    conn.execute("BEGIN", ()).await.unwrap();
    for i in 0..lots {
        let key = format!("LOT-{i:04}");
        conn.execute(
            "INSERT INTO lots (tender_id, lot_key) VALUES (?, ?)",
            (Value::Integer(tender), Value::Text(key.clone())),
        )
        .await
        .unwrap();
        let mut rows = conn
            .query(
                "SELECT id FROM lots WHERE tender_id = ? AND lot_key = ?",
                (Value::Integer(tender), Value::Text(key)),
            )
            .await
            .unwrap();
        let lot: i64 = rows.next().await.unwrap().unwrap().get_value(0).unwrap().as_integer().copied().unwrap();

        let t = Value::Integer(tender);
        let l = Value::Integer(lot);
        conn.execute(
            "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES (?, 1, ?, 'Lot')",
            (t.clone(), l.clone()),
        )
        .await
        .unwrap();
        for (field, lang) in [("title", "DEU"), ("title", "ENG"), ("description", "DEU")] {
            conn.execute(
                "INSERT INTO tender_version_texts (tender_id, seq, lot_id, field, lang, value)
                 VALUES (?, 1, ?, ?, ?, ?)",
                (
                    t.clone(),
                    l.clone(),
                    Value::Text(field.to_owned()),
                    Value::Text(lang.to_owned()),
                    Value::Text(format!("{field} {lang} {lot}")),
                ),
            )
            .await
            .unwrap();
        }
        conn.execute(
            "INSERT INTO tender_version_amounts (tender_id, seq, lot_id, field, cents, currency)
             VALUES (?, 1, ?, 'value', ?, 'EUR')",
            (t.clone(), l.clone(), Value::Integer(1000 + lot)),
        )
        .await
        .unwrap();
        for (field, secs) in [("submission_deadline", 1_800_000_000i64), ("planned_start", 1_810_000_000)] {
            conn.execute(
                "INSERT INTO tender_version_dates
                     (tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time)
                 VALUES (?, 1, ?, ?, ?, 120, 1)",
                (
                    t.clone(),
                    l.clone(),
                    Value::Text(field.to_owned()),
                    Value::Integer(secs + lot),
                ),
            )
            .await
            .unwrap();
        }
    }
    conn.execute("COMMIT", ()).await.unwrap();
}

/// Wall time of one whole-Tender lots read, best of `runs` (after a warm-up).
async fn time_read(conn: &turso::Connection, tender: i64, expect: i64) -> f64 {
    let filter = Filter { tender: Some(tender), ..Filter::default() };
    let scope = Scope::Page { after: 0, limit: 100_000 };
    let mut best = f64::MAX;
    for run in 0..4 {
        let t = Instant::now();
        let rows = read::lots(conn, &filter, scope).await.unwrap();
        let secs = t.elapsed().as_secs_f64();
        assert_eq!(rows.len() as i64, expect, "tender {tender} must return all its lots");
        assert!(rows.iter().all(|r| r.title.is_some()), "every lot decorated with a title");
        assert!(rows.iter().all(|r| r.value_cents.is_some()), "every lot decorated with a value");
        assert!(rows.iter().all(|r| r.deadline.is_some()), "every lot decorated with a deadline");
        if run > 0 {
            best = best.min(secs);
        }
    }
    best
}

#[tokio::test]
#[ignore = "heavy: builds a 3k-lot database and times two reads; run with --ignored"]
async fn tender_scoped_lots_read_is_not_quadratic_in_lot_count() {
    let path = format!("/tmp/tender-db-lotcost-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    // Db::open lays down the real canonical schema (and its indexes); the test then
    // drives its own connection so it can write the satellites directly.
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    seed(&conn, 1, SMALL).await;
    seed(&conn, 2, LARGE).await;

    let small = time_read(&conn, 1, SMALL).await;
    let large = time_read(&conn, 2, LARGE).await;
    let ratio = large / small;
    println!(
        "lots read: {SMALL} lots {small:.4}s, {LARGE} lots {large:.4}s -> {ratio:.1}x for 4x the lots"
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    assert!(
        ratio < MAX_RATIO,
        "4x the lots cost {ratio:.1}x the time ({small:.4}s -> {large:.4}s): the read is \
         superlinear in lot count. Expected ~4x (linear); ~16x means the per-lot correlated \
         subqueries are re-walking the version's whole satellite slice (issue 115)."
    );
}

/// Why the fix changes the DRIVING TABLE instead of adding an index — the obvious
/// answer, tried and measured.
///
/// `tender_version_lots` is `PRIMARY KEY (tender_id, seq, lot_id)`, which covers the
/// old join predicate `vl.tender_id = t.id AND vl.seq = v.seq AND vl.lot_id = l.id`
/// exactly. It should be a three-column seek. It is not: turso seeks the
/// `(tender_id, seq)` prefix and walks the slice, once per lot.
///
/// This probe adds an explicit index over precisely those three columns and shows
/// the cost does not move — so no index buys the old shape its way out, and the
/// `(tender_id, seq, lot_id)` index that issue 115 would otherwise have to
/// materialise on three prod-scale tables (with the deferred-builder obligation of
/// issue 111) would have bought nothing.
#[tokio::test]
#[ignore = "probe: records why an index is not the fix; run with --ignored"]
async fn an_index_does_not_rescue_the_lots_driven_join() {
    let path = format!("/tmp/tender-db-lotindex-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    seed(&conn, 1, SMALL).await;
    seed(&conn, 2, LARGE).await;

    // The pre-fix join order: drive from `lots`, probe `tender_version_lots` per row.
    // Identity columns only, so the satellite subqueries are not what is being timed.
    const LOTS_DRIVEN: &str = "SELECT l.id, vl.kind, v.seq
          FROM lots l
          JOIN tenders t ON t.id = l.tender_id
          JOIN tender_versions v ON v.tender_id = t.id
           AND v.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
          JOIN tender_version_lots vl
            ON vl.tender_id = t.id AND vl.seq = v.seq AND vl.lot_id = l.id
         WHERE l.tender_id = ? ORDER BY l.id LIMIT 100000";

    async fn run(conn: &turso::Connection, tender: i64) -> f64 {
        let mut best = f64::MAX;
        for run in 0..3 {
            let t = Instant::now();
            let mut rows = conn.query(LOTS_DRIVEN, (Value::Integer(tender),)).await.unwrap();
            while rows.next().await.unwrap().is_some() {}
            if run > 0 {
                best = best.min(t.elapsed().as_secs_f64());
            }
        }
        best
    }

    let before = run(&conn, 2).await;
    conn.execute(
        "CREATE INDEX tender_version_lots_probe ON tender_version_lots(tender_id, seq, lot_id)",
        (),
    )
    .await
    .unwrap();
    let after = run(&conn, 2).await;

    // And the shape the fix actually uses, for the contrast.
    let driven_from_vl = {
        let t = Instant::now();
        let mut rows = read::lots(
            &conn,
            &Filter { tender: Some(2), ..Filter::default() },
            Scope::Page { after: 0, limit: 100_000 },
        )
        .await
        .unwrap();
        rows.truncate(0);
        t.elapsed().as_secs_f64()
    };

    println!(
        "{LARGE} lots — lots-driven join: {before:.4}s, with an explicit \
         (tender_id, seq, lot_id) index: {after:.4}s, driven from tender_version_lots: \
         {driven_from_vl:.4}s"
    );

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    assert!(
        after > before / 2.0,
        "the explicit index made the lots-driven join {:.1}x faster ({before:.4}s -> \
         {after:.4}s). If turso has learned to use the third key column, the driving-table \
         split in read::lots may no longer be necessary — re-open issue 115's design.",
        before / after
    );
}
