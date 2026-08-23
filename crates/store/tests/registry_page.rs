//! Issue 173 (D4): `registry_page` is the re-hash probe's sampling cursor over
//! the fetch registry. What must hold: one row per DISTINCT package however many
//! re-fetch versions it accumulated, a stable MAX(id) order a stored cursor can
//! walk, and a strict `> after` bound so successive pages never overlap.

async fn open(name: &str) -> store::Db {
    let path = format!("/tmp/tender-db-regpage-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.expect("open")
}

async fn record(db: &store::Db, kind: &str, period: &str) {
    db.record_fetch(&store::Fetch {
        source: "ted".into(),
        kind: kind.into(),
        period: period.into(),
        url: format!("https://example.invalid/{period}"),
        sha256: "aa".into(),
        bytes: 1,
        fetched_at: 0,
        path: format!("ted/{kind}/{period}.tar.gz"),
    })
    .await
    .expect("record fetch");
}

#[tokio::test]
async fn pages_are_distinct_ordered_and_strictly_after() {
    let db = open("cycle").await;
    record(&db, "daily", "1993-00001").await; // id 1
    record(&db, "daily", "1993-00002").await; // id 2
    record(&db, "monthly", "2008-01").await; // id 3
    // A re-fetch version of the FIRST package: its group id becomes 4, moving it
    // to the cycle's end — the probe sees each package once, at its newest row.
    record(&db, "daily", "1993-00001").await; // id 4

    let all = db.registry_page(0, 10).await.expect("page");
    let keys: Vec<(i64, &str)> = all.iter().map(|(id, _, _, p)| (*id, p.as_str())).collect();
    assert_eq!(keys, vec![(2, "1993-00002"), (3, "2008-01"), (4, "1993-00001")]);

    // The cursor bound is strict and the limit caps the page.
    let after2 = db.registry_page(2, 1).await.expect("page");
    assert_eq!(after2.len(), 1);
    assert_eq!(after2[0].3, "2008-01");

    // Past the end: empty, which is the caller's wrap signal.
    assert!(db.registry_page(4, 10).await.expect("page").is_empty());
}
