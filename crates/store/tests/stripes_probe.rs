//! Issue 323's open residue: which plan does turso pick for `parsed_id_stripes`,
//! and does it matter?
//!
//! The doc block above that function said the planner "picks the rowid range seek,
//! not `notices_parse_state`", and argued from there that forcing the compact index
//! would be a pessimisation. A panel's fixture EQP disagreed, and the issue left it
//! open on purpose: "correcting a measured record deserves its own measurement, not
//! a fixture plan quoted second-hand."
//!
//! This is that measurement. Both statements picked `notices_parse_state` — walking
//! every parsed notice with no id range applied, the pessimisation the paragraph
//! warned about — and the unary `+` restores the rowid seek. At 200,000 notices
//! (180,000 parsed), best of three:
//!
//! ```text
//!                     as shipped   with `+`
//! scoped 100 ids  COUNT  0.3898s    0.0011s   (354x)
//! scoped 100 ids  WALK   0.3849s    0.0012s   (321x)
//! whole range     COUNT  0.4703s    0.4841s   (a wash)
//! whole range     WALK   0.6947s    0.6537s   (a wash)
//! ```
//!
//! The scoped row is the one that matters: the pre-pass's range comes from
//! `plan_notice_id_range`, so the incremental path always calls this scoped. The
//! whole-range row says the `+` costs nothing on a full rebuild, which is what makes
//! it unconditional rather than a judgement call.
//!
//! **The control is not decoration.** Issue 323's poisoned statement reproduces only
//! at a wide `IN` list — at two ids it seeks by rowid even poisoned — so a probe
//! written with a short list would appear to acquit the shape 323 measured. The
//! plan turns on the list width, not just the predicate.
//!
//! Ignored by default: seeding 200k notices takes ~4 minutes. Run with
//! `cargo test -p store --test stripes_probe -- --ignored --nocapture`.

use std::time::Instant;
use store::turso::{self, Value};

#[tokio::test]
#[ignore = "probe: issue 323 residue, stripe partition query plans; run with --ignored"]
async fn measure_the_stripe_plans() {
    let path = format!("/tmp/tender-db-stripes-eqp-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    db.ensure_unprojected_index().await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    const N: i64 = 200_000;
    let t = Instant::now();
    conn.execute(
        "INSERT INTO fetches (id, source, kind, period, url, sha256, bytes, fetched_at, path)
         VALUES (1, 'ted', 'daily', 'p', 'u', 'aa', 1, 0, 'p')",
        (),
    )
    .await
    .unwrap();
    // Seed through generate_series rather than 200k round trips. 90% parsed, so
    // the parse_state index is selective in exactly the unhelpful direction.
    conn.execute(
        &format!(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                                  member_path, ingested_at, parse_state)
             SELECT value, 'ted', 'pub-' || value, 'h' || value, 'eforms', 1, 'm', 0,
                    CASE WHEN value % 10 = 0 THEN 'pending' ELSE 'parsed' END
               FROM generate_series(1, {N})"
        ),
        (),
    )
    .await
    .unwrap();
    println!("seeded {N} notices in {:.1}s", t.elapsed().as_secs_f64());

    let plan_of = |sql: String| {
        let conn = conn.clone();
        async move {
            let mut rows = conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), ()).await.unwrap();
            let mut plan = String::new();
            while let Some(row) = rows.next().await.unwrap() {
                let turso::Value::Text(t) = row.get_value(3).unwrap() else { panic!("plan text") };
                plan.push_str(&t);
                plan.push(' ');
            }
            plan.trim_end().to_owned()
        }
    };
    let time_of = |sql: String, lo: i64, hi: i64| {
        let conn = conn.clone();
        async move {
            let mut best = f64::MAX;
            let mut rows_seen = 0usize;
            for run in 0..3 {
                let t = Instant::now();
                let mut rows = conn
                    .query(&sql, (Value::Integer(lo), Value::Integer(hi)))
                    .await
                    .unwrap();
                rows_seen = 0;
                while rows.next().await.unwrap().is_some() {
                    rows_seen += 1;
                }
                if run > 0 {
                    best = best.min(t.elapsed().as_secs_f64());
                }
            }
            (best, rows_seen)
        }
    };

    const COUNT_AS_IS: &str =
        "SELECT COUNT(*) FROM notices WHERE parse_state = 'parsed' AND id > ? AND id <= ?";
    const COUNT_PLUS: &str =
        "SELECT COUNT(*) FROM notices WHERE +parse_state = 'parsed' AND id > ? AND id <= ?";
    const WALK_AS_IS: &str =
        "SELECT id FROM notices WHERE parse_state = 'parsed' AND id > ? AND id <= ? ORDER BY id";
    const WALK_PLUS: &str =
        "SELECT id FROM notices WHERE +parse_state = 'parsed' AND id > ? AND id <= ? ORDER BY id";

    for (label, sql) in [
        ("COUNT as-is", COUNT_AS_IS),
        ("COUNT  plus", COUNT_PLUS),
        ("WALK  as-is", WALK_AS_IS),
        ("WALK   plus", WALK_PLUS),
    ] {
        println!("plan {label}: {}", plan_of(sql.to_string()).await);
    }

    // The instrument's control: issue 323's poison must reproduce at the list
    // size it was measured with. It does NOT at a small list — the plan depends
    // on the IN-list width — so a casual probe could "disprove" 323.
    for n in [2usize, 100, 500] {
        let list = std::iter::repeat("?").take(n).collect::<Vec<_>>().join(",");
        let poison = format!(
            "UPDATE notices SET projected = 0 WHERE parse_state = 'parsed' AND projected <> 0 AND id IN ({list})"
        );
        println!("control n={n} poison: {}", plan_of(poison).await);
    }

    // Scoped window (100 ids of 200k) — the shape the incremental pre-pass calls
    // with — and then the whole range, which is the full-rebuild shape.
    for (what, lo, hi) in [("scoped 100 ids", 100_000i64, 100_100i64), ("whole range", 0, N)] {
        for (label, sql) in [
            ("COUNT as-is", COUNT_AS_IS),
            ("COUNT  plus", COUNT_PLUS),
            ("WALK  as-is", WALK_AS_IS),
            ("WALK   plus", WALK_PLUS),
        ] {
            let (secs, rows) = time_of(sql.to_string(), lo, hi).await;
            println!("{what:15} {label}: {secs:.4}s ({rows} row(s))");
        }
    }

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
