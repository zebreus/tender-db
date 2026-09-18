//! Issue 408, option (b): the fallback walk is BOUNDED. An isolated, unseeded
//! id-ordered read examines one band of ids per page and hands back the band's
//! end as the cursor, so a sparse value costs one band per page instead of a
//! corpus pass — and the property that makes that safe is pinned here: pages
//! never examine overlapping ranges, nothing is skipped, and the walk terminates.

use store::read::{self, Filter};
use store::turso::{self, Value};

/// Six tenders, each with one version. Only 5 and 6 are `planning`; `kind` is a
/// `t.kind` predicate with no index, so `?kind=` isolates and — with no seed to
/// drive it — is exactly the walk this issue bounds.
async fn seed(conn: &turso::Connection) {
    for id in 1..=6i64 {
        let kind = if id >= 5 { "planning" } else { "procedure" };
        conn.execute(
            "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
             VALUES (?, 'ted', ?, ?, 1, 1700000000, 1700000000)",
            (Value::Integer(id), Value::Text(format!("pk-{id}")), Value::Text(kind.into())),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
             VALUES (?, 1, 1700000000, ?, ?)",
            (Value::Integer(id), Value::Text(format!("pub-{id}")), Value::Integer(id)),
        )
        .await
        .unwrap();
    }
}

/// The schema through the store, then a raw connection for seeding and reading —
/// the pattern the neighbouring read-path suites use.
async fn scratch(name: &str) -> (turso::Connection, String) {
    let path = format!("/tmp/tender-db-{name}-{}.db", std::process::id());
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    (db.connect().unwrap(), path)
}

#[tokio::test]
async fn a_bounded_walk_pages_without_overlap_skips_nothing_and_terminates() {
    let (conn, path) = scratch("fallback-band").await;
    seed(&conn).await;
    let filter = Filter { kind: Some("planning".into()), now: 1_756_000_000, ..Filter::default() };

    // Band of 2 ids, page limit far above the row count: every page but the last is
    // an examined-but-empty band, and the walk must still reach 5 and 6 exactly once.
    let mut after = 0i64;
    let mut walked: Vec<i64> = Vec::new();
    let mut examined: Vec<(i64, i64)> = Vec::new();
    let mut pages = 0;
    loop {
        pages += 1;
        assert!(pages <= 10, "the walk must terminate: {walked:?} {examined:?}");
        let page = read::tenders_page(&conn, &filter, after, 10, 2).await.unwrap();
        let ids: Vec<i64> = page.rows.iter().map(|r| r.id).collect();
        walked.extend(ids.iter().copied());
        match page.examined_to {
            Some(end) => {
                assert!(end > after, "the cursor must advance past the last examined id: {after} -> {end}");
                assert!(ids.iter().all(|id| *id <= end), "a returned row can never sit beyond the band: {ids:?} > {end}");
                examined.push((after + 1, end));
                after = end;
            }
            None => {
                examined.push((after + 1, i64::MAX));
                break;
            }
        }
    }
    assert_eq!(walked, vec![5, 6], "the bounded walk returns every match exactly once, in id order");
    // Bands 1..2, 3..4, then 5..6 — the last is the table's end, so no cursor follows it.
    assert_eq!(examined.len(), 3, "six ids at a band of two are three pages: {examined:?}");
    for pair in examined.windows(2) {
        assert!(pair[0].1 < pair[1].0, "successive pages must not examine overlapping ranges: {examined:?}");
    }

    // A full page is never bounded: with a band of 1000 and a page of 1, the overflow
    // row decides `more` exactly as before and no band cursor is issued.
    let page = read::tenders_page(&conn, &filter, 0, 2, 1000).await.unwrap();
    assert_eq!(page.rows.len(), 2, "limit+1 rows fetched: the page is full");
    assert!(page.examined_to.is_none(), "a full page resumes from its last row, not the band");

    // The last band of a walk that ends on the table's last id issues no cursor either.
    let page = read::tenders_page(&conn, &filter, 4, 10, 2).await.unwrap();
    assert_eq!(page.rows.iter().map(|r| r.id).collect::<Vec<_>>(), vec![5, 6]);
    assert!(page.examined_to.is_none(), "band end 6 is the table's last id: the walk is over");

    // An unseeded, non-walking read is never banded: no `kind`, the main pool's
    // ordinary page, `examined_to` absent whatever the band.
    let plain = Filter { now: 1_756_000_000, ..Filter::default() };
    let page = read::tenders_page(&conn, &plain, 0, 10, 1).await.unwrap();
    assert_eq!(page.rows.len(), 6);
    assert!(page.examined_to.is_none(), "a read that cannot walk is not bounded");

    drop(conn);
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}
