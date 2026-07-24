//! Go/no-go for the issue-58 watermark migration on a 254GB prod table: prove
//! `ALTER TABLE … ADD COLUMN … INTEGER NOT NULL DEFAULT 0` on a STRICT table is
//! METADATA-ONLY (O(1)), not an O(rows) table rewrite — a rewrite at startup on
//! 12.375M rows / 254GB would be multiple hours, catastrophic and on every open.
//!
//! turso 0.7 source (translate/alter.rs): a plain constant-default column with no
//! type mismatch, no NULL default, no non-constant default, and no CHECK emits
//! ONLY `Insn::AddColumn` (a schema change) — existing rows read the default
//! lazily. This test confirms it empirically: the ADD COLUMN time must NOT scale
//! with row count.

use std::time::Instant;

use store::turso::{self, Value};

async fn add_column_time(rows: i64) -> f64 {
    let path = format!("/tmp/tender-db-altercost-{rows}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let c = db.connect().unwrap();
    {
        let mut r = c.query("PRAGMA journal_mode = WAL", ()).await.unwrap();
        while r.next().await.unwrap().is_some() {}
    }
    // A STRICT table like `notices`.
    c.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, a TEXT NOT NULL, b INTEGER) STRICT", ())
        .await
        .unwrap();
    for chunk_start in (0..rows).step_by(5_000) {
        c.execute("BEGIN", ()).await.unwrap();
        for i in chunk_start..(chunk_start + 5_000).min(rows) {
            c.execute(
                "INSERT INTO t(id, a, b) VALUES(?, ?, ?)",
                (Value::Integer(i), Value::Text(format!("row-{i}")), Value::Integer(i * 2)),
            )
            .await
            .unwrap();
        }
        c.execute("COMMIT", ()).await.unwrap();
    }

    // The one operation under test — the exact shape of the issue-58 migration.
    let t = Instant::now();
    c.execute("ALTER TABLE t ADD COLUMN projected INTEGER NOT NULL DEFAULT 0", ()).await.unwrap();
    let elapsed = t.elapsed().as_secs_f64();

    // Sanity: existing rows read the default.
    let mut r = c.query("SELECT COUNT(*) FROM t WHERE projected = 0", ()).await.unwrap();
    let zeros = match r.next().await.unwrap() {
        Some(row) => match row.get_value(0).unwrap() {
            Value::Integer(i) => i,
            _ => -1,
        },
        None => -1,
    };
    assert_eq!(zeros, rows, "every existing row must read the default 0");

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    elapsed
}

#[tokio::test]
#[ignore = "heavy: builds tables at two sizes to time ADD COLUMN (~1 min); run with --ignored"]
async fn add_column_is_metadata_only_not_a_row_rewrite() {
    let small = add_column_time(100_000).await;
    let large = add_column_time(500_000).await;
    eprintln!("[altercost] ADD COLUMN: 100k rows = {small:.4}s | 500k rows = {large:.4}s");

    // A metadata-only ADD COLUMN is ~constant regardless of row count. An O(rows)
    // rewrite would make 500k take ~5x the 100k time. Allow generous slack for
    // fixed overhead/noise but reject linear scaling.
    assert!(
        large < small + 0.5 && large < small * 3.0 + 0.05,
        "ADD COLUMN scales with rows → it REWRITES the table (100k={small:.4}s, 500k={large:.4}s); \
         a 254GB rewrite at startup would be catastrophic"
    );
    // Both must be fast in absolute terms — a metadata op is milliseconds.
    assert!(large < 0.5, "ADD COLUMN on 500k rows took {large:.4}s — not metadata-only");
}
