//! Issue 117 probe: does the row-value cursor fix the pathological filter at the
//! cost of the ordinary one?
//!
//! `organizations_identity` is `(country, identifier_kind, identifier)` — it does
//! NOT contain `id`. So `AND (country, id) > (?, ?) ORDER BY id LIMIT ?` seeks the
//! country slice, but that slice arrives in `(identifier_kind, identifier)` order,
//! so `ORDER BY id` needs a sorter over the WHOLE slice and `LIMIT` applies after
//! it. The plain cursor has the opposite profile: it walks the table in rowid order
//! and stops as soon as it has `LIMIT` matches — instant for a dense filter, a full
//! table walk for one that matches nothing.
//!
//! So the two shapes trade places, and timing only a nothing-matching filter (as
//! issue 117's verification section specifies) sees the win and is structurally
//! blind to the regression.
//!
//! Both filters, both cursor shapes, both page sizes, with and without a
//! `(country, id)` index — the four-way table that decides the fix.
//!
//! `TDB_ORG_ROWS=3854017` reproduces prod's measured `country='DE'` slice
//! (run-driver, on the snapshot). Default 400k for a quick run.
//!
//! Run: `cargo test -p store --test org_cursor_probe -- --ignored --nocapture`

use std::time::Instant;
use store::turso::{self, Value};

const DENSE: &str = "DE";
const ABSENT: &str = "ZZ";

/// The current shape: the cursor is a plain `id > ?`.
const PLAIN: &str = "SELECT o.id, o.name FROM organizations o
                      WHERE 1 = 1 AND o.country = ? AND o.id > ?
                      ORDER BY o.id LIMIT ?";
/// Issue 117's proposed fix: the filter column joins the cursor comparison.
const ROW_VALUE: &str = "SELECT o.id, o.name FROM organizations o
                          WHERE 1 = 1 AND o.country = ? AND (o.country, o.id) > (?, ?)
                          ORDER BY o.id LIMIT ?";

fn rows_n() -> i64 {
    std::env::var("TDB_ORG_ROWS").ok().and_then(|v| v.parse().ok()).unwrap_or(400_000)
}

async fn drain(conn: &turso::Connection, sql: &str) {
    let mut rows = conn.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

fn params(sql: &str, country: &str, limit: i64) -> Vec<Value> {
    let c = || Value::Text(country.to_owned());
    if std::ptr::eq(sql, PLAIN) {
        vec![c(), Value::Integer(0), Value::Integer(limit)]
    } else {
        vec![c(), c(), Value::Integer(0), Value::Integer(limit)]
    }
}

async fn time(conn: &turso::Connection, sql: &str, country: &str, limit: i64) -> (f64, usize) {
    let p = params(sql, country, limit);
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

async fn sweep(conn: &turso::Connection, heading: &str) {
    println!("\n--- {heading} ---");
    println!("{:<12} {:>6}  {:>11}  {:>11}   {:>5}", "filter", "limit", "plain", "row value", "rows");
    for (label, country) in [("dense (DE)", DENSE), ("absent (ZZ)", ABSENT)] {
        for limit in [50i64, 1000] {
            let (plain, np) = time(conn, PLAIN, country, limit).await;
            let (rowval, nr) = time(conn, ROW_VALUE, country, limit).await;
            assert_eq!(np, nr, "{label} limit={limit}: the two shapes must return the same rows");
            println!("{label:<12} {limit:>6}  {plain:>10.4}s  {rowval:>10.4}s   {np:>5}");
        }
    }
}

async fn plans(conn: &turso::Connection, heading: &str) {
    for (label, sql) in [("plain", PLAIN), ("row value", ROW_VALUE)] {
        let mut rows = conn
            .query(&format!("EXPLAIN QUERY PLAN {sql}"), params(sql, DENSE, 1000))
            .await
            .unwrap();
        println!("\n{heading} / {label} plan:");
        while let Some(r) = rows.next().await.unwrap() {
            println!("  {}", r.get_value(3).unwrap().as_text().cloned().unwrap_or_default());
        }
    }
}

#[tokio::test]
#[ignore = "probe: issue 117 cursor-shape trade-off; run with --ignored"]
async fn row_value_cursor_trades_the_dense_filter_for_the_absent_one() {
    let path = format!("/tmp/tender-db-orgcursor-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();

    let n = rows_n();
    let seeded = Instant::now();
    conn.execute("BEGIN", ()).await.unwrap();
    for i in 0..n {
        if i > 0 && i % 500_000 == 0 {
            conn.execute("COMMIT", ()).await.unwrap();
            conn.execute("BEGIN", ()).await.unwrap();
        }
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
             VALUES (?, ?, ?, ?, ?, 0, 1700000000)",
            (
                Value::Integer(i),
                Value::Text(DENSE.to_owned()),
                Value::Text(if i % 2 == 0 { "vat" } else { "national" }.to_owned()),
                // Scattered, so index order is nothing like id order — the condition
                // that forces the sorter. Prod's identifiers are equally unrelated
                // to insertion order.
                Value::Text(format!("{:012}", (i.wrapping_mul(2_654_435_761)) & 0xFFFF_FFFF)),
                Value::Text(format!("org {i}")),
            ),
        )
        .await
        .unwrap();
    }
    conn.execute("COMMIT", ()).await.unwrap();
    conn.execute(
        "CREATE INDEX IF NOT EXISTS organizations_identity
             ON organizations(country, identifier_kind, identifier)",
        (),
    )
    .await
    .unwrap();
    println!("\n{n} organizations, all country='{DENSE}', seeded in {:.1}s", seeded.elapsed().as_secs_f64());

    sweep(&conn, "today's indexes (organizations_identity only)").await;
    plans(&conn, "today").await;

    // The other route to issue 117's own rule ("the cursor column must participate in
    // the index being sought"): rather than bend the query to the index, give the
    // index a trailing `id`, so the seek yields id order and `LIMIT` truncates with no
    // sorter. `changes_entity_cursor(entity_kind, cursor)` already does exactly this
    // for `changes_since`; 117 names it as the counter-example proving the rule
    // without drawing the conclusion that it is also the fix.
    let built = Instant::now();
    conn.execute("CREATE INDEX IF NOT EXISTS organizations_country_id ON organizations(country, id)", ())
        .await
        .unwrap();
    println!("\norganizations(country, id) built in {:.1}s", built.elapsed().as_secs_f64());

    sweep(&conn, "with organizations(country, id)").await;
    plans(&conn, "with (country,id)").await;

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
