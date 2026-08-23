//! The kill-9 crash probe (issue 271) — the in-repo successor to the lost
//! `/opt/tender-db/turso-bench/crash-loop.sh` leg of the D1 protocol
//! (docs/research/turso-scale.md). The bench lived only on the box and died with
//! it (the issue-224 loss class); this binary and `ops/turso-crash-loop.sh`
//! restore the durability gate in the repository, where a box rebuild cannot
//! take it.
//!
//! Two modes, driven by the loop script:
//!
//!   crash_probe write  <db-path>   — open the REAL schema via `store::Db`, then
//!     append batches forever: one `tenders` row, its `tender_versions` head row
//!     and five `tender_version_texts` rows per `BEGIN IMMEDIATE … COMMIT`, a
//!     TRUNCATE checkpoint every 32 batches, and `committed <n>` on stdout after
//!     every commit. The shape is the fold's: parent + FK satellites in one
//!     transaction, exactly what must never tear.
//!
//!   crash_probe verify <db-path> <last-committed>  — reopen after a kill -9 and
//!     hold turso to the WAL contract: every tender id ≤ MAX(id) present exactly
//!     once with exactly one version and exactly five texts (no torn batch), and
//!     MAX(id) ≥ <last-committed> (nothing acknowledged was lost). Exit 0 on
//!     pass; panics name what broke.
//!
//! The probe is durability-only on purpose: planner behaviour is pinned by the
//! EXPLAIN-QUERY-PLAN gates and `view_pushdown_probe`, and the old bench's
//! throughput numbers stand as historical measurements in turso-scale.md.

use turso::Value;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("write") => write(&args[2]).await,
        Some("verify") => verify(&args[2], args[3].parse().expect("last-committed count")).await,
        _ => {
            eprintln!("usage: crash_probe write <db> | crash_probe verify <db> <last-committed>");
            std::process::exit(2);
        }
    }
}

async fn connect(path: &str) -> turso::Connection {
    // Db::open installs (or migrates) the real schema; the raw connection then
    // drives the engine directly — the probe tests turso under our schema, not
    // the store API.
    let _db = store::Db::open(path).await.expect("open store db");
    let raw = turso::Builder::new_local(path).build().await.expect("build");
    raw.connect().expect("connect")
}

async fn write(path: &str) {
    let conn = connect(path).await;
    conn.execute("PRAGMA foreign_keys = ON", ()).await.expect("fk on");
    // The FK spine every version hangs off: one fetch, one notice, inserted
    // idempotently so a restart over the surviving database reuses them.
    conn.execute(
        "INSERT INTO fetches (id, source, kind, period, url, sha256, bytes, fetched_at, path)
         VALUES (1, 'ted', 'daily', 'crash', 'probe://crash', 'aa', 1, 0, 'crash')
         ON CONFLICT DO NOTHING",
        (),
    )
    .await
    .expect("insert fetch");
    conn.execute(
        "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                              member_path, ingested_at, parse_state)
         VALUES (1, 'ted', 'crash-1', 'aa', 'text', 1, 'crash#1', 0, 'parsed')
         ON CONFLICT DO NOTHING",
        (),
    )
    .await
    .expect("insert notice");
    let mut rows = conn.query("SELECT COALESCE(MAX(id), 0) FROM tenders", ()).await.expect("max");
    let mut next = match rows.next().await.expect("row") {
        Some(row) => match row.get_value(0) {
            Ok(Value::Integer(i)) => i + 1,
            _ => 1,
        },
        None => 1,
    };
    drop(rows);
    loop {
        conn.execute("BEGIN IMMEDIATE", ()).await.expect("begin");
        conn.execute(
            "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
             VALUES (?, 'ted', ?, 'procedure', 1, 100, 0)",
            (Value::Integer(next), Value::Text(format!("crash-{next}"))),
        )
        .await
        .expect("insert tender");
        conn.execute(
            "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
             VALUES (?, 1, 1, 100, ?)",
            (Value::Integer(next), Value::Text(format!("crash-{next}"))),
        )
        .await
        .expect("insert version");
        for t in 0..5 {
            conn.execute(
                "INSERT INTO tender_version_texts (tender_id, seq, lot_id, field, lang, value)
                 VALUES (?, 1, NULL, 'title', 'ENG', ?)",
                (Value::Integer(next), Value::Text(format!("text {next}/{t}"))),
            )
            .await
            .expect("insert text");
        }
        conn.execute("COMMIT", ()).await.expect("commit");
        // The ack the verifier holds turso to. Printed only AFTER the commit
        // returned, like a caller that told its user "saved".
        println!("committed {next}");
        if next % 32 == 0 {
            let _ = conn.execute("PRAGMA wal_checkpoint(TRUNCATE)", ()).await;
        }
        next += 1;
    }
}

async fn verify(path: &str, last_committed: i64) {
    let conn = connect(path).await;
    let count = |sql: &'static str| {
        let conn = conn.clone();
        async move {
            let mut rows = conn.query(sql, ()).await.expect(sql);
            match rows.next().await.expect(sql) {
                Some(row) => match row.get_value(0) {
                    Ok(Value::Integer(i)) => i,
                    other => panic!("{sql}: not an integer: {other:?}"),
                },
                None => panic!("{sql}: no row"),
            }
        }
    };
    let max_id = count("SELECT COALESCE(MAX(id), 0) FROM tenders").await;
    assert!(
        max_id >= last_committed,
        "DURABILITY: writer was acked through {last_committed} but the store holds {max_id}"
    );
    let tenders = count("SELECT COUNT(*) FROM tenders").await;
    assert_eq!(tenders, max_id, "id gaps: {tenders} tenders under MAX(id) {max_id}");
    let versions = count("SELECT COUNT(*) FROM tender_versions").await;
    assert_eq!(versions, tenders, "ATOMICITY: {tenders} tenders but {versions} versions");
    let texts = count("SELECT COUNT(*) FROM tender_version_texts").await;
    assert_eq!(texts, tenders * 5, "ATOMICITY: {tenders} tenders but {texts} texts (want 5 each)");
    let orphans = count(
        "SELECT COUNT(*) FROM tender_version_texts x \
          WHERE NOT EXISTS (SELECT 1 FROM tender_versions v \
                             WHERE v.tender_id = x.tender_id AND v.seq = x.seq)",
    )
    .await;
    assert_eq!(orphans, 0, "FK: {orphans} orphaned text rows");
    println!("ok {tenders} tenders whole through kill -9 (acked {last_committed})");
}
