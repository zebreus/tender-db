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

/// Rows to seed. Overridable via `TDB_LOTS` because two DIFFERENT questions are asked
/// of this fixture and they need different sizes: *is a shape index-served* is
/// structural and answerable small, while *what does it cost* needs scale and belongs
/// to the prod-scale clock anyway. Seeding 400k takes ~18 min on a loaded box, which
/// is a poor price for a structural answer.
fn lots() -> i64 {
    std::env::var("TDB_LOTS").ok().and_then(|v| v.parse().ok()).unwrap_or(400_000)
}

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

/// Branch A — the PK-correlated probe. Correlating on `l.tender_id` (which `lots`
/// already carries) hands the probe the PK's LEADING column, so no new index is
/// needed. But `vl` is probed before `t`, so `t.current_seq` is not yet available and
/// only that leading column binds — each probe then walks the tender's whole slice.
const PK_EXISTS: &str = "SELECT l.id FROM lots l
     WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                    JOIN tenders t ON t.id = l.tender_id
                    WHERE vl.tender_id = l.tender_id AND vl.seq = t.current_seq
                      AND vl.lot_id = l.id AND vl.kind = ?)
       AND l.id > ? ORDER BY l.id LIMIT 50";

/// Branch A' — the same answer with `current_seq` resolved independently, so ALL
/// THREE primary-key columns bind at once and the probe is a single descent instead
/// of a slice walk. One clause moved; the amplification is a join-order artifact.
const PK_EXISTS_SCALAR: &str = "SELECT l.id FROM lots l
     WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                    WHERE vl.tender_id = l.tender_id
                      AND vl.seq = (SELECT current_seq FROM tenders WHERE id = l.tender_id)
                      AND vl.lot_id = l.id AND vl.kind = ?)
       AND l.id > ? ORDER BY l.id LIMIT 50";


/// **The shape that actually ships** — today's `read::lots_query` stream form, verbatim
/// in structure: `vl.kind` and `v.seq` are OUTPUT columns, not just filter terms, and
/// `seq` is obtained by JOINING `tender_versions`.
///
/// This is the arm that matters. The `EXISTS` probes above establish that the *filter*
/// can be satisfied without driving from `vl` — necessary, but not sufficient for "the
/// whole row can be produced that way", because production must RETURN data from `vl`.
const SHIP_TODAY: &str = "SELECT l.id, l.tender_id, l.lot_key, vl.kind, v.seq
     FROM lots l
     JOIN tenders t ON t.id = l.tender_id
     JOIN tender_versions v ON v.tender_id = t.id
       AND v.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)
     JOIN tender_version_lots vl ON vl.tender_id = t.id AND vl.seq = v.seq AND vl.lot_id = l.id
    WHERE vl.kind = ? AND l.id > ? ORDER BY l.id LIMIT 50";

/// The candidate: same output columns, but `seq` resolved by a scalar subquery so all
/// three primary-key columns bind at once. `vl.seq` then supplies the output column the
/// `tender_versions` join used to provide, so that join drops out entirely.
///
/// Structurally A' with the join RETAINED for output — which is exactly why it needs
/// its own measurement. A' was an `EXISTS`; this is a JOIN, and the planner may still
/// choose to drive from `vl`, which is the whole failure mode.
const SHIP_CANDIDATE: &str = "SELECT l.id, l.tender_id, l.lot_key, vl.kind, vl.seq
     FROM lots l
     JOIN tenders t ON t.id = l.tender_id
     JOIN tender_version_lots vl
       ON vl.tender_id = l.tender_id
      AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
      AND vl.lot_id = l.id
    WHERE vl.kind = ? AND l.id > ? ORDER BY l.id LIMIT 50";


/// **S2** — the shippable candidate. `lots` alone in FROM (plus a PK join to `tenders`
/// for `current_seq`), output columns from a SELECT-list correlated subquery that runs
/// per OUTPUT row rather than per candidate. Keeps `lots` the sole driver, which the
/// gate established is the sufficient condition.
const SHIP_S2: &str = "SELECT l.id, l.tender_id, l.lot_key,
         (SELECT vl.kind FROM tender_version_lots vl
           WHERE vl.tender_id = l.tender_id AND vl.seq = t.current_seq AND vl.lot_id = l.id),
         t.current_seq
       FROM lots l JOIN tenders t ON t.id = l.tender_id
      WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                     WHERE vl.tender_id = l.tender_id AND vl.seq = t.current_seq
                       AND vl.lot_id = l.id AND vl.kind = ?)
        AND l.id > ? ORDER BY l.id LIMIT 50";

/// **S2b** — as S2 but recomputing `MAX(seq)` instead of trusting `tenders.current_seq`.
/// Costs extra seeks; keeps today's redundancy rather than converting it into a trust
/// relationship on a projection-maintained invariant.
const SHIP_S2B: &str = "SELECT l.id, l.tender_id, l.lot_key,
         (SELECT vl.kind FROM tender_version_lots vl
           WHERE vl.tender_id = l.tender_id
             AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
             AND vl.lot_id = l.id),
         (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
       FROM lots l
      WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                     WHERE vl.tender_id = l.tender_id
                       AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                       AND vl.lot_id = l.id AND vl.kind = ?)
        AND l.id > ? ORDER BY l.id LIMIT 50";

/// Whole rows as text, so equivalence covers the `kind` and `seq` VALUES rather than
/// ids alone. A shape that returns the right lots with the wrong version's `kind` is
/// the defect an id-only comparison cannot see.
async fn full_rows(conn: &turso::Connection, sql: &str, p: Vec<Value>) -> Vec<String> {
    let mut rows = conn.query(sql, p).await.unwrap();
    let mut out = Vec::new();
    while let Some(r) = rows.next().await.unwrap() {
        let mut cells = Vec::new();
        for i in 0..5 {
            cells.push(format!("{:?}", r.get_value(i).unwrap()));
        }
        out.push(cells.join("|"));
    }
    out
}

/// The ids a query returns, in order — the unit of the equivalence assertions.
async fn ids(conn: &turso::Connection, sql: &str, p: Vec<Value>) -> Vec<i64> {
    let mut rows = conn.query(sql, p).await.unwrap();
    let mut out = Vec::new();
    while let Some(r) = rows.next().await.unwrap() {
        out.push(r.get_value(0).unwrap().as_integer().copied().unwrap_or(-1));
    }
    out
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
    let lots_n = lots();
    for i in 1..=(lots_n / 4) {
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
    for i in 1..=lots_n {
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
            if i > lots_n - 50 { "Part" } else { "Lot" }
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
    println!("\n{lots_n} lots seeded in {:.1}s ('Lot' dense, 'Part' rare, placement={placement})",
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

    // ------------------------------------------------- the probe may need NO index
    //
    // `EXISTS_SHAPE` correlates the subquery through `vl.tender_id = t.id`, so the
    // only thing tying `vl` to the outer row is `vl.lot_id = l.id` — and `lot_id` is
    // the THIRD column of the PK `(tender_id, seq, lot_id)`, hence unserved, hence the
    // new index looked mandatory.
    //
    // But `lots` already carries `tender_id`. Correlating on `l.tender_id` instead
    // hands the probe ALL THREE primary-key columns, so the PK itself should serve it
    // and no new index is needed at all. That matters well beyond tidiness: an index
    // on this table is 40.6M rows against a 41M auto-build cap, with a memory-linear
    // ~1.82 GB build peak — so "no index" dissolves a feasibility gate rather than
    // saving a little disk.
    //
    // Measured here BEFORE any index exists, so nothing else can be serving it.
    println!("\n--- PK-correlated EXISTS, NO new index ---");
    for (label, kind) in [
        ("PK EXISTS, kind=Lot (dense)", "Lot"),
        ("PK EXISTS, kind=Part (rare)", "Part"),
        ("PK EXISTS, kind=zzz (nothing)", "zzz"),
    ] {
        let (t, n) =
            time(&conn, PK_EXISTS, vec![Value::Text(kind.into()), Value::Integer(0)]).await;
        println!("{label:<44} {t:>9.4}s  {n:>5}");
    }
    plan(&conn, "PK-correlated EXISTS (dense) plan:", PK_EXISTS,
         vec![Value::Text("Lot".into()), Value::Integer(0)]).await;

    // Equivalence across EVERY class, asserted rather than printed.
    //
    // A faster shape that answers differently is not a candidate, and dense alone is
    // not enough: A' is a scalar-subquery REWRITE, so the risk is not that it is slow
    // but that it silently changes which rows come back — and the class most likely to
    // expose that is the one where few rows match, not the one where nearly all do.
    // Dense passing is close to uninformative here: the first 50 lots match under
    // almost any correct-ish predicate.
    //
    // Compared against `JOINED` — the shape actually in production — rather than
    // against each other, since agreement between two candidates says nothing about
    // whether either preserves today's answer.
    println!("\nequivalence against today's JOIN (all three classes):");
    for kind in ["Lot", "Part", "zzz"] {
        let p = vec![Value::Text(kind.into()), Value::Integer(0)];
        let today = ids(&conn, JOINED, p.clone()).await;
        let branch_a = ids(&conn, PK_EXISTS, p.clone()).await;
        let branch_a_prime = ids(&conn, PK_EXISTS_SCALAR, p).await;
        println!(
            "    kind={kind:<5} JOIN {:>3} ids · A {:>3} · A' {:>3}",
            today.len(),
            branch_a.len(),
            branch_a_prime.len()
        );
        assert_eq!(branch_a, today, "branch A changed the answer for kind={kind}");
        assert_eq!(branch_a_prime, today, "branch A' changed the answer for kind={kind}");
    }

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

/// **Does branch A's probe seek WITHIN a tender's slice, or walk all of it?**
///
/// This is the single number run-driver's 6.34-billion estimate hangs on. They measured
/// prod's real joint distribution and found branch A's probe costs **480 tvl-row visits
/// per lot** — but only *if* each probe walks the tender's whole `tender_version_lots`
/// slice. Their own caveat: if turso can use the `(tender_id, seq, lot_id)` PK to seek
/// further into the slice, the true figure is far lower and branch A survives.
///
/// The plan text says `SEARCH vl USING INDEX sqlite_autoindex_… (tender_id=?)` — only
/// the leading column bound. But issue 112 rule 6 is that EQP cannot tell a seek from a
/// walk, so the plan is exactly what must NOT be trusted here. The clock can.
///
/// **The design holds lots constant and varies only the slice depth.** Same lot count,
/// same query, same matched set — only `SEQS` (versions per tender, hence tvl rows per
/// tender) changes. `kind='zzz'` is used because it probes every lot and returns
/// nothing, so the measurement is pure probe cost with no result assembly.
///
///   * cost scaling ~linearly in `SEQS`  → each probe WALKS the slice → the 480x
///     amplification is real and branch A is in serious trouble on the `zzz` arm.
///   * cost flat in `SEQS`               → the probe SEEKS within the slice → the
///     estimate is a large overbound and branch A survives.
///
/// A ratio, not an absolute — so a loaded box cannot change the verdict, only the noise.
#[tokio::test]
#[ignore = "probe: does the PK probe seek within a tender's slice? run with --ignored"]
async fn does_the_pk_probe_seek_within_the_tender_slice() {
    const TENDERS: i64 = 2_000;
    const LOTS_PER: i64 = 5;

    println!("\n{:>6}  {:>12}  {:>12}  {:>10}  {:>12}  {:>10}  {:>12}  {:>10}  {:>12}  {:>10}",
             "seqs", "tvl rows", "A zzz", "A ratio", "A' zzz", "A' ratio",
             "SHIP today", "ratio", "SHIP cand", "ratio");
    let (mut baseline, mut baseline2) = (0.0, 0.0);
    let (mut baseline3, mut baseline4) = (0.0, 0.0);
    let (mut baseline5, mut baseline6) = (0.0, 0.0);
    for (run, seqs) in [1i64, 8, 32].into_iter().enumerate() {
        let path = format!("/tmp/tender-db-slice-{}-{seqs}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        store::Db::open(&path).await.unwrap();
        let db = turso::Builder::new_local(&path).build().await.unwrap();
        let conn = db.connect().unwrap();
        drain(&conn, "PRAGMA journal_mode = WAL").await;
        conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

        conn.execute("BEGIN", ()).await.unwrap();
        for tender in 1..=TENDERS {
            // `current_seq` is the LAST seq, so a walking probe pays the whole slice
            // before it finds the current row — the honest worst case for the shape.
            conn.execute(
                "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
                 VALUES (?, 'ted', ?, 'procedure', ?, 1700000000, 1700000000)",
                (Value::Integer(tender), Value::Text(format!("pk-{tender}")), Value::Integer(seqs)),
            ).await.unwrap();
            for seq in 1..=seqs {
                conn.execute(
                    "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
                     VALUES (?, ?, 1700000000, ?, ?)",
                    (Value::Integer(tender), Value::Integer(seq),
                     Value::Text(format!("pub-{tender}-{seq}")),
                     Value::Integer((tender - 1) * seqs + seq)),
                ).await.unwrap();
            }
            for k in 1..=LOTS_PER {
                let lot = (tender - 1) * LOTS_PER + k;
                conn.execute(
                    "INSERT INTO lots (id, tender_id, lot_key) VALUES (?, ?, ?)",
                    (Value::Integer(lot), Value::Integer(tender), Value::Text(format!("LOT-{lot}"))),
                ).await.unwrap();
                // Every (seq, lot) pair exists: the slice is LOTS_PER * seqs deep.
                //
                // The kind DIFFERS between historical and current versions, and that is
                // the point. Historical rows say 'Part', the current one says 'Lot'. So
                // `?kind=Part` must return NOTHING — every Part row sits on a
                // superseded version.
                //
                // This is the discriminating case for A', which rewrites how
                // `current_seq` is resolved. A shape that matched ANY seq rather than
                // the current one would return rows here and be silently wrong in
                // production, while looking correct on a single-version fixture. The
                // main probe seeds one seq per tender and therefore cannot see it.
                for seq in 1..=seqs {
                    let kind = if seq == seqs { "Lot" } else { "Part" };
                    conn.execute(
                        "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES (?, ?, ?, ?)",
                        (Value::Integer(tender), Value::Integer(seq), Value::Integer(lot),
                         Value::Text(kind.to_owned())),
                    ).await.unwrap();
                }
            }
        }
        conn.execute("COMMIT", ()).await.unwrap();

        // Branch A' — the same answer, with `current_seq` moved into a scalar subquery.
        //
        // A's plan probes `vl` BEFORE `t`, so `t.current_seq` is not yet available and
        // only the PK's leading column can be bound; the rest of the slice is filtered
        // row by row. That makes the amplification a JOIN-ORDER artifact rather than a
        // property of the shape. Resolving `current_seq` independently should let all
        // three PK columns bind at once and collapse the probe to one descent.

        let (t, _) = time(&conn, PK_EXISTS, vec![Value::Text("zzz".into()), Value::Integer(0)]).await;
        let (t2, _) =
            time(&conn, PK_EXISTS_SCALAR, vec![Value::Text("zzz".into()), Value::Integer(0)]).await;
        if run == 0 {
            baseline = t;
            baseline2 = t2;
        }
        // THE ARM THAT GATES THE CHANGE: the shapes that would actually ship, with
        // `vl.kind`/`seq` as OUTPUT columns. If SHIP_CANDIDATE scales with slice depth
        // like SHIP_TODAY does, the planner has driven from `vl` and the design does
        // not survive contact with the real column list.
        let (t3, _) = time(&conn, SHIP_TODAY, vec![Value::Text("zzz".into()), Value::Integer(0)]).await;
        let (t4, _) =
            time(&conn, SHIP_CANDIDATE, vec![Value::Text("zzz".into()), Value::Integer(0)]).await;
        let (t5, _) = time(&conn, SHIP_S2, vec![Value::Text("zzz".into()), Value::Integer(0)]).await;
        let (t6, _) = time(&conn, SHIP_S2B, vec![Value::Text("zzz".into()), Value::Integer(0)]).await;
        if run == 0 {
            baseline5 = t5;
            baseline6 = t6;
        }
        println!("        S2  {t5:>9.4}s  {:>6.2}x      S2b {t6:>9.4}s  {:>6.2}x",
                 t5 / baseline5, t6 / baseline6);
        if run == 0 {
            baseline3 = t3;
            baseline4 = t4;
        }
        println!("{seqs:>6}  {:>12}  {t:>11.4}s  {:>9.2}x  {t2:>11.4}s  {:>9.2}x  {t3:>11.4}s  {:>9.2}x  {t4:>11.4}s  {:>9.2}x",
                 TENDERS * LOTS_PER * seqs, t / baseline, t2 / baseline2,
                 t3 / baseline3, t4 / baseline4);
        if seqs == 32 {
            plan(&conn, "branch A plan (deep slice):", PK_EXISTS,
                 vec![Value::Text("zzz".into()), Value::Integer(0)]).await;
            plan(&conn, "SHIPPING today plan (deep slice):", SHIP_TODAY,
                 vec![Value::Text("Lot".into()), Value::Integer(0)]).await;
            plan(&conn, "SHIPPING candidate plan (deep slice):", SHIP_CANDIDATE,
                 vec![Value::Text("Lot".into()), Value::Integer(0)]).await;
            // Equivalence on WHOLE ROWS: the kind and seq values, not just the ids.
            // A shape returning the right lots with a superseded version's `kind` is
            // exactly what an id-only comparison cannot see.
            println!("\nSHIPPING equivalence on whole rows (id|tender|key|kind|seq):");
            for kind in ["Lot", "Part", "zzz"] {
                let p = vec![Value::Text(kind.into()), Value::Integer(0)];
                let p2 = p.clone();
                let today = full_rows(&conn, SHIP_TODAY, p.clone()).await;
                let cand = full_rows(&conn, SHIP_CANDIDATE, p).await;
                println!("    kind={kind:<5} today {:>3} rows · candidate {:>3}", today.len(), cand.len());
                let s2 = full_rows(&conn, SHIP_S2, p2.clone()).await;
                let s2b = full_rows(&conn, SHIP_S2B, p2).await;
                println!("             S2 {:>3} · S2b {:>3}", s2.len(), s2b.len());
                assert_eq!(cand, today, "SHIPPING candidate changed the answer for kind={kind}");
                assert_eq!(s2, today, "S2 changed the answer for kind={kind}");
                assert_eq!(s2b, today, "S2b changed the answer for kind={kind}");
            }
            plan(&conn, "S2 plan (deep slice):", SHIP_S2,
                 vec![Value::Text("Lot".into()), Value::Integer(0)]).await;
            plan(&conn, "S2b plan (deep slice):", SHIP_S2B,
                 vec![Value::Text("Lot".into()), Value::Integer(0)]).await;
            plan(&conn, "branch A' plan (deep slice):", PK_EXISTS_SCALAR,
                 vec![Value::Text("zzz".into()), Value::Integer(0)]).await;
            // Equivalence on a MULTI-VERSION fixture, which the main probe cannot
            // provide. `Part` exists only on superseded versions here, so a correct
            // shape returns nothing for it — and a shape that resolved `current_seq`
            // loosely would return every lot.
            const JOINED: &str = "SELECT l.id FROM lots l
                  JOIN tenders t ON t.id = l.tender_id
                  JOIN tender_version_lots vl
                    ON vl.tender_id = t.id AND vl.seq = t.current_seq AND vl.lot_id = l.id
                 WHERE vl.kind = ? AND l.id > ? ORDER BY l.id LIMIT 50";
            println!("\nequivalence at seqs={seqs} (Part exists ONLY on superseded versions):");
            for kind in ["Lot", "Part", "zzz"] {
                let p = vec![Value::Text(kind.into()), Value::Integer(0)];
                let today = ids(&conn, JOINED, p.clone()).await;
                let a = ids(&conn, PK_EXISTS, p.clone()).await;
                let a_prime = ids(&conn, PK_EXISTS_SCALAR, p).await;
                println!("    kind={kind:<5} JOIN {:>3} ids · A {:>3} · A' {:>3}",
                         today.len(), a.len(), a_prime.len());
                assert_eq!(a, today, "branch A changed the answer for kind={kind}");
                assert_eq!(a_prime, today, "branch A' changed the answer for kind={kind}");
            }
            // The fixture must actually EXERCISE the discriminator, or the assertions
            // above pass vacuously: `Part` has to be absent from the answer because it
            // is superseded, not because it was never seeded.
            assert!(
                ids(&conn, JOINED, vec![Value::Text("Lot".into()), Value::Integer(0)]).await.len() == 50
                    && ids(&conn, JOINED, vec![Value::Text("Part".into()), Value::Integer(0)]).await.is_empty(),
                "the multi-version fixture must return rows for the current kind and none \
                 for the superseded one, else the equivalence check proves nothing"
            );
        }
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }
    println!("\nlots held constant at {}; only slice depth varies.", TENDERS * LOTS_PER);
    println!("~linear in seqs => probe WALKS the slice (run-driver's 480x stands)");
    println!("flat in seqs    => probe SEEKS within it (the estimate is an overbound)");
}

/// **Fast shape search.** The shipping-shape gate failed: a JOIN that outputs `vl.kind`
/// reverts to `SCAN tender_version_lots` + top-level sorter, because once `vl` is a
/// FROM-clause table the planner drives from it. A' only worked because `lots` was the
/// ONLY candidate driver — the `EXISTS` is the mechanism, not a detail.
///
/// So the question is which *shippable* formulation keeps `lots` the sole driver while
/// still returning `vl.kind` and the version `seq`. That is a plan question, and plan
/// questions are answerable in seconds on a tiny fixture — turso has no `sqlite_stat1`
/// for these tables on either the bed or prod, so the planner chooses structurally and
/// the choice is largely size-independent. Timing comes after a shape survives this.
#[tokio::test]
#[ignore = "probe: fast EQP-only shape search for the shippable form; run with --ignored"]
async fn which_shippable_shape_keeps_lots_driving() {
    let path = format!("/tmp/tender-db-shape-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    // Tiny: the plan is what is being read, not the clock.
    conn.execute("BEGIN", ()).await.unwrap();
    for tender in 1..=500i64 {
        conn.execute(
            "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
             VALUES (?, 'ted', ?, 'procedure', 2, 1700000000, 1700000000)",
            (Value::Integer(tender), Value::Text(format!("pk-{tender}"))),
        ).await.unwrap();
        for seq in 1..=2i64 {
            conn.execute(
                "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
                 VALUES (?, ?, 1700000000, ?, ?)",
                (Value::Integer(tender), Value::Integer(seq),
                 Value::Text(format!("p-{tender}-{seq}")), Value::Integer((tender - 1) * 2 + seq)),
            ).await.unwrap();
        }
        for k in 1..=4i64 {
            let lot = (tender - 1) * 4 + k;
            conn.execute("INSERT INTO lots (id, tender_id, lot_key) VALUES (?, ?, ?)",
                (Value::Integer(lot), Value::Integer(tender), Value::Text(format!("L-{lot}")))).await.unwrap();
            for seq in 1..=2i64 {
                conn.execute(
                    "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES (?, ?, ?, ?)",
                    (Value::Integer(tender), Value::Integer(seq), Value::Integer(lot),
                     Value::Text(if seq == 2 { "Lot" } else { "Part" }.to_owned())),
                ).await.unwrap();
            }
        }
    }
    conn.execute("COMMIT", ()).await.unwrap();

    // Candidate shapes. Each must return the SAME five columns as production.
    let shapes: Vec<(&str, String)> = vec![
        ("S0 today (baseline)", SHIP_TODAY.to_owned()),
        ("S1 JOIN + scalar seq (FAILED gate)", SHIP_CANDIDATE.to_owned()),
        // `lots` alone in FROM; both output columns come from SELECT-list subqueries,
        // which evaluate per OUTPUT row (~50 per page) rather than per candidate.
        ("S2 EXISTS + select-list subqueries", "SELECT l.id, l.tender_id, l.lot_key,
             (SELECT vl.kind FROM tender_version_lots vl
               WHERE vl.tender_id = l.tender_id AND vl.seq = t.current_seq AND vl.lot_id = l.id),
             t.current_seq
           FROM lots l JOIN tenders t ON t.id = l.tender_id
          WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                         WHERE vl.tender_id = l.tender_id AND vl.seq = t.current_seq
                           AND vl.lot_id = l.id AND vl.kind = ?)
            AND l.id > ? ORDER BY l.id LIMIT 50".to_owned()),
        // S2 depends on `tenders.current_seq` being the max seq — true on prod today
        // (verified: 0 of 4,262,716 tenders diverge) but PROJECTION-MAINTAINED, not
        // enforced by the schema. This variant recomputes it instead, keeping today's
        // redundancy rather than converting it into a trust relationship.
        ("S2b EXISTS + select-list, MAX(seq) not current_seq", "SELECT l.id, l.tender_id, l.lot_key,
             (SELECT vl.kind FROM tender_version_lots vl
               WHERE vl.tender_id = l.tender_id
                 AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                 AND vl.lot_id = l.id),
             (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
           FROM lots l
          WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                         WHERE vl.tender_id = l.tender_id
                           AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                           AND vl.lot_id = l.id AND vl.kind = ?)
            AND l.id > ? ORDER BY l.id LIMIT 50".to_owned()),
        // THE FULL FILTER SURFACE. `version_predicates` references BOTH `t.id` and
        // `v.seq`, and `?source=` references `t.source` — none of which exist in S2b's
        // FROM clause. So the shape validated on `kind` alone does not yet cover the
        // query it has to become. Both predicates are rewritten to correlate on
        // `l.tender_id` directly; the question is whether adding them lets the planner
        // pick a different driver.
        ("S2b+country+source (full surface)", "SELECT l.id, l.tender_id, l.lot_key,
             (SELECT vl.kind FROM tender_version_lots vl
               WHERE vl.tender_id = l.tender_id
                 AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                 AND vl.lot_id = l.id),
             (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
           FROM lots l JOIN tenders t ON t.id = l.tender_id
          WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                         WHERE vl.tender_id = l.tender_id
                           AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                           AND vl.lot_id = l.id AND vl.kind = ?)
            AND t.source = 'ted'
            AND EXISTS (SELECT 1 FROM tender_version_classifications c
                         WHERE c.tender_id = l.tender_id
                           AND c.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                           AND c.scheme = 'nuts' AND c.code LIKE 'DE%')
            AND l.id > ? ORDER BY l.id LIMIT 50".to_owned()),
        // `lots` ALONE in FROM — even `tenders` becomes a scalar subquery. If the rule
        // is "any second FROM-clause table lets the planner pick a different driver",
        // then `?source=` must be expressed this way too, not as a join.
        ("S2c full surface, lots alone in FROM", "SELECT l.id, l.tender_id, l.lot_key,
             (SELECT vl.kind FROM tender_version_lots vl
               WHERE vl.tender_id = l.tender_id
                 AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                 AND vl.lot_id = l.id),
             (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
           FROM lots l
          WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                         WHERE vl.tender_id = l.tender_id
                           AND vl.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                           AND vl.lot_id = l.id AND vl.kind = ?)
            AND (SELECT tt.source FROM tenders tt WHERE tt.id = l.tender_id) = 'ted'
            AND EXISTS (SELECT 1 FROM tender_version_classifications c
                         WHERE c.tender_id = l.tender_id
                           AND c.seq = (SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = l.tender_id)
                           AND c.scheme = 'nuts' AND c.code LIKE 'DE%')
            AND l.id > ? ORDER BY l.id LIMIT 50".to_owned()),
        // S2d — the combination neither S2 nor S2c offered: `lots` alone in FROM (so the
        // full filter surface is safe) AND `current_seq` read by a cheap PK subquery
        // instead of recomputing MAX(seq) 13.2M times. S2's cost with S2c's structure.
        ("S2d full surface, current_seq by PK subquery", "SELECT l.id, l.tender_id, l.lot_key,
             (SELECT vl.kind FROM tender_version_lots vl
               WHERE vl.tender_id = l.tender_id
                 AND vl.seq = (SELECT tt.current_seq FROM tenders tt WHERE tt.id = l.tender_id)
                 AND vl.lot_id = l.id),
             (SELECT tt.current_seq FROM tenders tt WHERE tt.id = l.tender_id)
           FROM lots l
          WHERE EXISTS (SELECT 1 FROM tender_version_lots vl
                         WHERE vl.tender_id = l.tender_id
                           AND vl.seq = (SELECT tt.current_seq FROM tenders tt WHERE tt.id = l.tender_id)
                           AND vl.lot_id = l.id AND vl.kind = ?)
            AND (SELECT tt.source FROM tenders tt WHERE tt.id = l.tender_id) = 'ted'
            AND EXISTS (SELECT 1 FROM tender_version_classifications c
                         WHERE c.tender_id = l.tender_id
                           AND c.seq = (SELECT tt.current_seq FROM tenders tt WHERE tt.id = l.tender_id)
                           AND c.scheme = 'nuts' AND c.code LIKE 'DE%')
            AND l.id > ? ORDER BY l.id LIMIT 50".to_owned()),
        // Paginate FIRST, join after: the LIMIT is applied to a `lots`-driven subquery,
        // so at most 50 rows ever reach the join.
        ("S3 paginate-then-join", "SELECT l.id, l.tender_id, l.lot_key, vl.kind, vl.seq
           FROM (SELECT id, tender_id, lot_key FROM lots l2
                  WHERE l2.id > ?
                    AND EXISTS (SELECT 1 FROM tender_version_lots vl2
                                 JOIN tenders t2 ON t2.id = l2.tender_id
                                WHERE vl2.tender_id = l2.tender_id AND vl2.seq = t2.current_seq
                                  AND vl2.lot_id = l2.id AND vl2.kind = ?)
                  ORDER BY l2.id LIMIT 50) l
           JOIN tenders t ON t.id = l.tender_id
           JOIN tender_version_lots vl
             ON vl.tender_id = l.tender_id AND vl.seq = t.current_seq AND vl.lot_id = l.id
          ORDER BY l.id".to_owned()),
    ];

    for (label, sql) in &shapes {
        // S3 binds cursor-then-kind; the others bind kind-then-cursor.
        let p = if label.starts_with("S3") {
            vec![Value::Integer(0), Value::Text("Lot".into())]
        } else {
            vec![Value::Text("Lot".into()), Value::Integer(0)]
        };
        plan(&conn, &format!("--- {label}"), sql, p.clone()).await;
        let rows = full_rows(&conn, sql, p).await;
        println!("    rows={} first={}", rows.len(), rows.first().cloned().unwrap_or_default());
    }

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
