//! Task 16, design step: can a filter on the JOINED table be expressed so that `lots`
//! stays the driving table?
//!
//! The target property is measured, not theorised: `/v1/lots?limit=50` already runs at
//! **0.014 s** on prod because it drives from `lots` in rowid order, so `ORDER BY l.id`
//! is satisfied by the drive and `LIMIT` stops early. Adding `?kind=Lot` costs **>330 s**
//! — because the drive flips to a full `SCAN tender_version_lots` and `ORDER BY l.id` then
//! needs a top-level sorter over the whole matched set, so the DENSE value is the worst
//! case (bigger match set, bigger sort).
//!
//! run-driver already eliminated one option by measurement: an index on
//! `tender_version_lots(kind)` or `(kind, lot_id)` kills the scan and **leaves the sorter**,
//! which fixes the half that was already cheaper.
//!
//! So the question is not "how do we find the matching rows faster" but **"how do we keep
//! `lots` driving so the ordering comes from the drive"** — and that is a question about
//! what turso's planner does, which is measured here rather than assumed.
//!
//! Three shapes, and the reason each might fail:
//!   * **JOIN** — today's. Expected to flip the drive to `vl`.
//!   * **EXISTS** — keeps `lots` in the FROM clause alone, so the planner has no other
//!     table to drive from. But the per-row probe needs an index that serves
//!     `vl.lot_id = l.id`, and the PK is `(tender_id, seq, lot_id)` — `lot_id` is its
//!     THIRD column, so nothing serves that lookup today.
//!   * **EXISTS + `tender_version_lots(lot_id, kind)`** — the same shape with the index
//!     the probe needs. If this keeps `lots` driving AND the probe is index-served, the
//!     target property survives filtering.

use std::time::Instant;
use store::turso::{self, Value};

const LOTS: i64 = 400_000;

async fn drain(conn: &turso::Connection, sql: &str) {
    let mut rows = conn.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

async fn plan(conn: &turso::Connection, label: &str, sql: &str, p: Vec<Value>) {
    let mut rows = conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), p).await.unwrap();
    println!("\n{label}");
    while let Some(r) = rows.next().await.unwrap() {
        println!("    {}", r.get_value(3).unwrap().as_text().cloned().unwrap_or_default());
    }
}

async fn time(conn: &turso::Connection, sql: &str, p: Vec<Value>) -> (f64, usize) {
    let (mut best, mut n) = (f64::MAX, 0);
    for i in 0..3 {
        let t = Instant::now();
        let mut rows = conn.query(sql, p.clone()).await.unwrap();
        n = 0;
        while rows.next().await.unwrap().is_some() {
            n += 1;
        }
        if i > 0 {
            best = best.min(t.elapsed().as_secs_f64());
        }
    }
    (best, n)
}

#[tokio::test]
#[ignore = "probe: task 16 driving-table question; run with --ignored"]
async fn can_a_joined_filter_keep_lots_driving() {
    let path = format!("/tmp/tender-db-lotsdrive-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    // One tender per 4 lots, every lot kind 'Lot' except a rare tail — mirroring prod,
    // where `Lot` is the overwhelming default and the dense case is the catastrophe.
    let seeded = Instant::now();
    conn.execute("BEGIN", ()).await.unwrap();
    for i in 1..=(LOTS / 4) {
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
    }
    for i in 1..=LOTS {
        if i % 100_000 == 0 {
            conn.execute("COMMIT", ()).await.unwrap();
            conn.execute("BEGIN", ()).await.unwrap();
        }
        let tender = (i - 1) / 4 + 1;
        conn.execute(
            "INSERT INTO lots (id, tender_id, lot_key) VALUES (?, ?, ?)",
            (Value::Integer(i), Value::Integer(tender), Value::Text(format!("LOT-{i}"))),
        ).await.unwrap();
        // Placement of the rare kind is the UNKNOWN this probe brackets. `TDB_RARE=late`
        // clusters every rare lot in the last 50 rows — matches-late by construction,
        // the worst case. `scattered` spreads the same count evenly. Which one prod
        // resembles decides whether the EXISTS trade is acceptable, and that is a
        // property of the corpus nobody can read off the schema.
        let late = std::env::var("TDB_RARE").as_deref() != Ok("scattered");
        let kind = if late {
            if i > LOTS - 50 { "Part" } else { "Lot" }
        } else if i % 20 == 0 {
            "Part"
        } else {
            "Lot"
        };
        conn.execute(
            "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES (?, 1, ?, ?)",
            (Value::Integer(tender), Value::Integer(i), Value::Text(kind.to_owned())),
        ).await.unwrap();
    }
    conn.execute("COMMIT", ()).await.unwrap();
    let placement = std::env::var("TDB_RARE").unwrap_or_else(|_| "late".into());
    println!("\n{LOTS} lots seeded in {:.1}s ('Lot' dense, 'Part' rare, placement={placement})",
             seeded.elapsed().as_secs_f64());

    // The target: unfiltered, driving from `lots`, ORDER BY satisfied by the drive.
    const UNFILTERED: &str = "SELECT l.id FROM lots l WHERE l.id > 0 ORDER BY l.id LIMIT 50";
    // Today's shape: the filter lives on the joined table.
    const JOINED: &str = "SELECT l.id FROM lots l
          JOIN tenders t ON t.id = l.tender_id
          JOIN tender_version_lots vl
            ON vl.tender_id = t.id AND vl.seq = t.current_seq AND vl.lot_id = l.id
         WHERE vl.kind = ? AND l.id > ? ORDER BY l.id LIMIT 50";
    // The candidate: the filter becomes a per-lot predicate, so `lots` is the only
    // table the planner can drive from.
    const EXISTS_SHAPE: &str = "SELECT l.id FROM lots l
         WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                        JOIN tenders t ON t.id = vl.tender_id
                        WHERE vl.lot_id = l.id AND vl.seq = t.current_seq AND vl.kind = ?)
           AND l.id > ? ORDER BY l.id LIMIT 50";

    let (t, n) = time(&conn, UNFILTERED, vec![]).await;
    println!("\n{:<44} {:>10}  {:>5}", "case", "time", "rows");
    println!("{:<44} {t:>9.4}s  {n:>5}", "unfiltered (the target property)");
    for (label, kind) in [
        ("joined, kind=Lot (dense)", "Lot"),
        ("joined, kind=Part (rare)", "Part"),
        ("joined, kind=zzz (nothing)", "zzz"),
    ] {
        let sql = JOINED;
        let (t, n) = time(&conn, sql, vec![Value::Text(kind.into()), Value::Integer(0)]).await;
        println!("{label:<44} {t:>9.4}s  {n:>5}");
    }
    // The DENSE EXISTS case only, un-indexed: it is fast because `lots` drives and the
    // first 50 rows all match, so the per-row probe barely runs. The RARE case is
    // deliberately NOT timed here — without a supporting index it must probe nearly
    // every lot with an unserved `vl.lot_id = l.id` lookup before finding 50 matches,
    // and it does not finish in any useful time. That un-indexed shape is not a
    // shipping candidate, so measuring it precisely buys nothing; knowing it is
    // pathological is the point, and it is why the index below is part of the design
    // rather than an optimisation of it.
    let (t, n) =
        time(&conn, EXISTS_SHAPE, vec![Value::Text("Lot".into()), Value::Integer(0)]).await;
    println!("{:<44} {t:>9.4}s  {n:>5}", "EXISTS, kind=Lot (dense), NO index");

    plan(&conn, "JOINED (dense) plan:", JOINED, vec![Value::Text("Lot".into()), Value::Integer(0)]).await;

    // The index the per-row probe needs: the PK is (tender_id, seq, lot_id), so
    // `lot_id` is its THIRD column and nothing serves a lookup by it alone.
    conn.execute(
        "CREATE INDEX IF NOT EXISTS tvl_lot_kind ON tender_version_lots(lot_id, kind)",
        (),
    )
    .await
    .unwrap();
    println!("\n--- with tender_version_lots(lot_id, kind) ---");
    for (label, kind) in [
        ("EXISTS, kind=Lot (dense)", "Lot"),
        ("EXISTS, kind=Part (rare)", "Part"),
        ("EXISTS, kind=zzz (nothing)", "zzz"),
    ] {
        let (t, n) =
            time(&conn, EXISTS_SHAPE, vec![Value::Text(kind.into()), Value::Integer(0)]).await;
        println!("{label:<44} {t:>9.4}s  {n:>5}");
    }
    plan(&conn, "EXISTS (dense) plan, WITH the index:", EXISTS_SHAPE,
         vec![Value::Text("Lot".into()), Value::Integer(0)]).await;

    // ---------------------------------------------------------------- column order
    //
    // The design so far assumed `(lot_id, kind)`, because the probe's *unserved*
    // lookup was `vl.lot_id = l.id`. But BOTH of the subquery's predicates on `vl`
    // are equalities — `vl.lot_id = l.id AND vl.kind = ?` — so a composite index
    // seeks on both columns whichever order they are in. The order is therefore free
    // to be chosen for what ELSE it can serve.
    //
    // `(kind, lot_id)` additionally serves `WHERE kind = ? LIMIT 1`, the issue-117
    // existence short-circuit. `(lot_id, kind)` cannot: `kind` is its second column,
    // so answering "does any row carry this kind" means walking every lot_id.
    //
    // That matters because matches-nothing is the ONLY class where the new shape
    // stays O(N) — it must probe every lot to prove nothing matches — and it is the
    // cell blocking acceptance. If the short-circuit serves it, that class collapses
    // to a single seek and stops being a full pass at all.
    //
    // Two things are being measured, and only one of them transfers across scale:
    // whether the planner SERVES both uses from one index is structural and testable
    // here; the COSTS are not, and belong to the prod-scale clock.
    conn.execute("DROP INDEX tvl_lot_kind", ()).await.unwrap();
    conn.execute(
        "CREATE INDEX tvl_kind_lot ON tender_version_lots(kind, lot_id)",
        (),
    )
    .await
    .unwrap();
    println!("\n--- with tender_version_lots(kind, lot_id) instead ---");
    for (label, kind) in [
        ("EXISTS, kind=Lot (dense)", "Lot"),
        ("EXISTS, kind=Part (rare)", "Part"),
        ("EXISTS, kind=zzz (nothing)", "zzz"),
    ] {
        let (t, n) =
            time(&conn, EXISTS_SHAPE, vec![Value::Text(kind.into()), Value::Integer(0)]).await;
        println!("{label:<44} {t:>9.4}s  {n:>5}");
    }

    // The short-circuit this column order unlocks: one seek, no walk. Timed by the
    // clock and not read off a plan — EQP calls both a `SEARCH ... USING INDEX` and
    // issue 112 rule 6 is that only the clock separates a seek from a walk.
    const REACHABLE: &str = "SELECT 1 FROM tender_version_lots WHERE kind = ? LIMIT 1";
    println!("\nexistence short-circuit, `WHERE kind = ? LIMIT 1`:");
    for (label, kind) in
        [("kind=Lot (present)", "Lot"), ("kind=Part (present)", "Part"), ("kind=zzz (absent)", "zzz")]
    {
        let (t, n) = time(&conn, REACHABLE, vec![Value::Text(kind.into())]).await;
        println!("    {label:<40} {t:>9.4}s  {n:>5}");
    }
    plan(&conn, "the short-circuit's plan:", REACHABLE, vec![Value::Text("zzz".into())]).await;

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
