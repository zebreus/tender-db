//! Issue 217-B (org name search) — design probe: can turso 0.7 serve a
//! case-insensitive name-prefix search from an index, and via WHICH shape?
//!
//! The candidates, cheapest first:
//!   1. `CREATE INDEX … ON organizations(name COLLATE NOCASE, id)` + a NOCASE
//!      range — no schema change, one deferred index.
//!   2. The same index driven by `LIKE 'prefix%'` (SQLite's LIKE is
//!      ASCII-case-insensitive by default and can seek a NOCASE index).
//!   3. A normalised `name_norm` column + plain index — always works, but costs a
//!      column, fold changes and a ~30M-row backfill; only worth it if 1/2 fail.
//!
//! Run with: cargo test -p store --test name_prefix_probe -- --ignored --nocapture

use store::turso::{self, Value};

async fn plan(conn: &turso::Connection, label: &str, sql: &str, p: Vec<Value>) {
    match conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), p).await {
        Ok(mut rows) => {
            println!("\n{label}:");
            while let Some(r) = rows.next().await.unwrap() {
                println!("  {}", r.get_value(3).unwrap().as_text().cloned().unwrap_or_default());
            }
        }
        Err(e) => println!("\n{label}: ERROR {e}"),
    }
}

async fn timed(conn: &turso::Connection, label: &str, sql: &str, p: Vec<Value>) {
    let t = std::time::Instant::now();
    match conn.query(sql, p).await {
        Ok(mut rows) => {
            let mut n = 0;
            let mut first = String::new();
            while let Some(r) = rows.next().await.unwrap() {
                if n == 0 {
                    first = r.get_value(1).ok().and_then(|v| v.as_text().cloned()).unwrap_or_default();
                }
                n += 1;
            }
            println!("{label:<28} {:>9.4}s rows={n} first={first:?}", t.elapsed().as_secs_f64());
        }
        Err(e) => println!("{label:<28} ERROR {e}"),
    }
}

#[tokio::test]
#[ignore = "design probe for issue 217-B; run with --ignored --nocapture"]
async fn nocase_index_prefix_shapes() {
    let path = format!("/tmp/tender-db-nameprobe-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    // 200k orgs, mixed-case names; a handful of "Siemens…" rows scattered in.
    conn.execute("BEGIN", ()).await.unwrap();
    for i in 1..=200_000i64 {
        let name = match i % 40_000 {
            7 => format!("Siemens AG Niederlassung {i}"),
            13 => format!("SIEMENS Mobility {i}"),
            21 => format!("siemens healthineers {i}"),
            _ => format!("Musterfirma {} GmbH", i * 7919 % 999_983),
        };
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
             VALUES (?, 'DE', NULL, NULL, ?, 1, 0)",
            (Value::Integer(i), Value::Text(name)),
        )
        .await
        .unwrap();
    }
    conn.execute("COMMIT", ()).await.unwrap();

    println!("== candidate 1/2: the NOCASE index ==");
    match conn
        .execute("CREATE INDEX orgs_name_nocase ON organizations(name COLLATE NOCASE, id)", ())
        .await
    {
        Ok(_) => println!("NOCASE index: CREATED"),
        Err(e) => println!("NOCASE index: REFUSED — {e}"),
    }

    const RANGE: &str = "SELECT id, name FROM organizations
        WHERE name >= ? COLLATE NOCASE AND name < ? COLLATE NOCASE
        ORDER BY name COLLATE NOCASE LIMIT 50";
    const LIKE: &str = "SELECT id, name FROM organizations WHERE name LIKE ? LIMIT 50";

    timed(&conn, "range NOCASE 'siemens'", RANGE, vec![Value::Text("siemens".into()), Value::Text("siement".into())]).await;
    timed(&conn, "LIKE 'siemens%'", LIKE, vec![Value::Text("siemens%".into())]).await;
    timed(&conn, "LIKE absent 'zzzz%'", LIKE, vec![Value::Text("zzzz%".into())]).await;

    plan(&conn, "plan: range NOCASE", RANGE, vec![Value::Text("siemens".into()), Value::Text("siement".into())]).await;
    plan(&conn, "plan: LIKE prefix", LIKE, vec![Value::Text("siemens%".into())]).await;

    println!("\n== candidate 3 baseline: plain index + lower() range ==");
    match conn
        .execute("CREATE INDEX orgs_name_plain ON organizations(name, id)", ())
        .await
    {
        Ok(_) => println!("plain index: CREATED"),
        Err(e) => println!("plain index: REFUSED — {e}"),
    }
    const PLAIN: &str = "SELECT id, name FROM organizations
        WHERE name >= ? AND name < ? ORDER BY name LIMIT 50";
    timed(&conn, "plain range 'Siemens'", PLAIN, vec![Value::Text("Siemens".into()), Value::Text("Siement".into())]).await;
    plan(&conn, "plan: plain range", PLAIN, vec![Value::Text("Siemens".into()), Value::Text("Siement".into())]).await;

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
