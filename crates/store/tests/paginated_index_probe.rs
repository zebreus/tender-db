//! Issue 117: does a `(filter, id)` index fix the paginated reads, at both
//! densities, for the two cases nobody has measured yet?
//!
//!  * `read::tenders` filtered by `source` — run-driver noted its plan ALREADY
//!    carries a sorter today, which may make it behave unlike `organizations`.
//!  * `read::organizations` filtered by `identifier_kind` alone — the 99.08s
//!    case. Issue 117 first concluded no index could serve it (second column of
//!    `organizations_identity`); sdk-vendor overturned that, arguing
//!    `organizations(identifier_kind, id)` gives seek + id order. Unverified.
//!
//! Both call the REAL read functions rather than a restatement of their SQL, so
//! what is timed is the artifact — `read::tenders` carries nine correlated
//! subqueries that a paraphrase would omit, and omitting them is exactly how a
//! probe agrees with itself (issues 110, 102, 114).
//!
//! Timings only, no plans: the `*_statement` seams are `pub(crate)` so an
//! integration test cannot reach them — and the clock is the load-bearing
//! instrument here regardless. A plan would ENDORSE the regression this issue is
//! about (walk -> index seek), which is the whole reason 117 is gated on a clock.
//!
//! Densities matter in opposite directions and a fix must hold for both: a plain
//! cursor walks in rowid order and stops at `LIMIT` matches, so it is fast when the
//! filter is dense and a full walk when it matches nothing; a bare index seek is
//! the reverse. Only an index whose trailing column is `id` is fast for both.

use std::time::Instant;
use store::read::{self, Filter, Scope};
use store::turso::{self, Value};

const TENDERS: i64 = 400_000;
const ORGS: i64 = 400_000;

async fn drain(conn: &turso::Connection, sql: &str) {
    let mut rows = conn.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

async fn open(tag: &str) -> (String, turso::Connection) {
    let path = format!("/tmp/tender-db-pgidx-{tag}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (path, conn)
}

/// Best of three, after a warm-up. Concrete per read rather than generic: the
/// closure-plus-async-future form fights the borrow checker for no benefit here.
async fn time_tenders(conn: &turso::Connection, source: &str, limit: i64) -> (f64, usize) {
    let f = Filter { source: Some(source.to_owned()), ..Filter::default() };
    let (mut b, mut n) = (f64::MAX, 0);
    for i in 0..3 {
        let t = Instant::now();
        n = read::tenders(conn, &f, Scope::Page { after: 0, limit }).await.unwrap().len();
        if i > 0 {
            b = b.min(t.elapsed().as_secs_f64());
        }
    }
    (b, n)
}

async fn time_orgs(conn: &turso::Connection, kind: &str, limit: i64) -> (f64, usize) {
    let f = Filter { kind: Some(kind.to_owned()), ..Filter::default() };
    let (mut b, mut n) = (f64::MAX, 0);
    for i in 0..3 {
        let t = Instant::now();
        n = read::organizations(conn, &f, Scope::Page { after: 0, limit }).await.unwrap().len();
        if i > 0 {
            b = b.min(t.elapsed().as_secs_f64());
        }
    }
    (b, n)
}

#[tokio::test]
#[ignore = "probe: issue 117 tenders(source, id); run with --ignored"]
async fn a_source_id_index_fixes_the_tenders_list_at_both_densities() {
    let (path, conn) = open("tenders").await;

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
    }
    conn.execute("COMMIT", ()).await.unwrap();
    conn.execute("CREATE INDEX IF NOT EXISTS tenders_island ON tenders(source, island_notice_id)", ())
        .await
        .unwrap();
    println!("\n{TENDERS} tenders ('ted'), seeded in {:.1}s", seeded.elapsed().as_secs_f64());


    println!("\n{:<26} {:>11}  {:>6}", "case", "time", "rows");
    for (label, source) in [("dense (ted)", "ted"), ("absent (zz)", "zz")] {
        for limit in [50i64, 1000] {
            let (t, n) = time_tenders(&conn, source, limit).await;
            println!("before  {label:<12} limit {limit:<5} {t:>9.4}s  {n:>6}");
        }
    }

    let built = Instant::now();
    conn.execute("CREATE INDEX IF NOT EXISTS tenders_source_id ON tenders(source, id)", ())
        .await
        .unwrap();
    println!("\ntenders(source, id) built in {:.1}s", built.elapsed().as_secs_f64());

    for (label, source) in [("dense (ted)", "ted"), ("absent (zz)", "zz")] {
        for limit in [50i64, 1000] {
            let (t, n) = time_tenders(&conn, source, limit).await;
            println!("after   {label:<12} limit {limit:<5} {t:>9.4}s  {n:>6}");
        }
    }


    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

#[tokio::test]
#[ignore = "probe: issue 117 organizations(identifier_kind, id) — the 99s kind-only case; run with --ignored"]
async fn an_identifier_kind_id_index_fixes_the_kind_only_listing() {
    let (path, conn) = open("orgkind").await;

    let seeded = Instant::now();
    conn.execute("BEGIN", ()).await.unwrap();
    for i in 0..ORGS {
        if i > 0 && i % 100_000 == 0 {
            conn.execute("COMMIT", ()).await.unwrap();
            conn.execute("BEGIN", ()).await.unwrap();
        }
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
             VALUES (?, ?, ?, ?, ?, 0, 1700000000)",
            (
                Value::Integer(i),
                Value::Text("DE".to_owned()),
                Value::Text(if i % 2 == 0 { "vat" } else { "national" }.to_owned()),
                Value::Text(format!("{:012}", (i.wrapping_mul(2_654_435_761)) & 0xFFFF_FFFF)),
                Value::Text(format!("org {i}")),
            ),
        ).await.unwrap();
    }
    conn.execute("COMMIT", ()).await.unwrap();
    conn.execute(
        "CREATE INDEX IF NOT EXISTS organizations_identity
             ON organizations(country, identifier_kind, identifier)",
        (),
    )
    .await
    .unwrap();
    println!("\n{ORGS} organizations, seeded in {:.1}s", seeded.elapsed().as_secs_f64());

    // kind ONLY — no country — which is what makes `organizations_identity` useless:
    // `identifier_kind` is its SECOND column.

    println!("\n{:<26} {:>11}  {:>6}", "case", "time", "rows");
    for (label, kind) in [("dense (vat)", "vat"), ("absent (zzz)", "zzz")] {
        for limit in [50i64, 1000] {
            let (t, n) = time_orgs(&conn, kind, limit).await;
            println!("before  {label:<13} limit {limit:<5} {t:>9.4}s  {n:>6}");
        }
    }

    let built = Instant::now();
    conn.execute(
        "CREATE INDEX IF NOT EXISTS organizations_kind_id ON organizations(identifier_kind, id)",
        (),
    )
    .await
    .unwrap();
    println!("\norganizations(identifier_kind, id) built in {:.1}s", built.elapsed().as_secs_f64());

    for (label, kind) in [("dense (vat)", "vat"), ("absent (zzz)", "zzz")] {
        for limit in [50i64, 1000] {
            let (t, n) = time_orgs(&conn, kind, limit).await;
            println!("after   {label:<13} limit {limit:<5} {t:>9.4}s  {n:>6}");
        }
    }


    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// `notices(source, id)` — the 226s+ case. Shape-identical to `tenders(source, id)`
/// above, and measured anyway: "same shape, therefore same result" is the reasoning
/// that produced issue 117's original fix table, which was wrong by 151,648x.
#[tokio::test]
#[ignore = "probe: issue 117 notices(source, id); run with --ignored"]
async fn a_source_id_index_fixes_the_notices_list_at_both_densities() {
    let (path, conn) = open("notices").await;

    let seeded = Instant::now();
    conn.execute("BEGIN", ()).await.unwrap();
    for i in 1..=TENDERS {
        if i % 100_000 == 0 {
            conn.execute("COMMIT", ()).await.unwrap();
            conn.execute("BEGIN", ()).await.unwrap();
        }
        conn.execute(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile,
                                  member_path, ingested_at, parse_state, fetch_id)
             VALUES (?, 'ted', ?, ?, 'eforms', ?, 1700000000, 'parsed', 1)",
            (
                Value::Integer(i),
                Value::Text(format!("pub-{i}")),
                Value::Text(format!("{:064x}", i)),
                Value::Text(format!("m/{i}.xml")),
            ),
        )
        .await
        .unwrap();
    }
    conn.execute("COMMIT", ()).await.unwrap();
    println!("\n{TENDERS} notices ('ted'), seeded in {:.1}s", seeded.elapsed().as_secs_f64());

    async fn time_notices(conn: &turso::Connection, source: &str, limit: i64) -> (f64, usize) {
        let f = Filter { source: Some(source.to_owned()), ..Filter::default() };
        let (mut b, mut n) = (f64::MAX, 0);
        for i in 0..3 {
            let t = Instant::now();
            n = read::notices(conn, &f, Scope::Page { after: 0, limit }).await.unwrap().len();
            if i > 0 {
                b = b.min(t.elapsed().as_secs_f64());
            }
        }
        (b, n)
    }

    println!("\n{:<26} {:>11}  {:>6}", "case", "time", "rows");
    for (label, source) in [("dense (ted)", "ted"), ("absent (zz)", "zz")] {
        for limit in [50i64, 1000] {
            let (t, n) = time_notices(&conn, source, limit).await;
            println!("before  {label:<12} limit {limit:<5} {t:>9.4}s  {n:>6}");
        }
    }

    let built = Instant::now();
    conn.execute("CREATE INDEX IF NOT EXISTS notices_source_id ON notices(source, id)", ())
        .await
        .unwrap();
    println!("\nnotices(source, id) built in {:.1}s", built.elapsed().as_secs_f64());

    for (label, source) in [("dense (ted)", "ted"), ("absent (zz)", "zz")] {
        for limit in [50i64, 1000] {
            let (t, n) = time_notices(&conn, source, limit).await;
            println!("after   {label:<12} limit {limit:<5} {t:>9.4}s  {n:>6}");
        }
    }

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

/// Is `/v1/tenders?kind=` actually a walk, or only shaped like one?
///
/// It was NOT in issue 117's audit. The routing predicate isolates it because no index
/// covers `t.kind` — `tenders_procedure_key`, `tenders_island`,
/// `tenders_current_published` and `tenders_source_id` are the whole set. But that is
/// reading the schema, and reading is not measuring; a claim that a filter walks needs
/// a clock like every other one today.
#[tokio::test]
#[ignore = "probe: is tenders?kind= an unaudited member of the 117 class?"]
async fn tenders_kind_is_an_unaudited_walk() {
    let (path, conn) = open("tenderkind").await;
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
    }
    conn.execute("COMMIT", ()).await.unwrap();
    conn.execute("CREATE INDEX IF NOT EXISTS tenders_source_id ON tenders(source, id)", ())
        .await
        .unwrap();
    println!("\n{TENDERS} tenders ('procedure'), seeded in {:.1}s", seeded.elapsed().as_secs_f64());

    async fn time_kind(conn: &turso::Connection, kind: &str, limit: i64) -> (f64, usize) {
        let f = Filter { kind: Some(kind.to_owned()), ..Filter::default() };
        let (mut b, mut n) = (f64::MAX, 0);
        for i in 0..3 {
            let t = Instant::now();
            n = read::tenders(conn, &f, Scope::Page { after: 0, limit }).await.unwrap().len();
            if i > 0 {
                b = b.min(t.elapsed().as_secs_f64());
            }
        }
        (b, n)
    }

    println!("\n{:<28} {:>11}  {:>6}", "case", "time", "rows");
    for (label, kind) in [("dense (procedure)", "procedure"), ("absent (zzz)", "zzz")] {
        for limit in [50i64, 1000] {
            let (t, n) = time_kind(&conn, kind, limit).await;
            println!("tenders?kind {label:<14} limit {limit:<5} {t:>9.4}s  {n:>6}");
        }
    }

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
