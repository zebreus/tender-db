//! Issue 216: the published-ordered Tender list — order, range bounds, NULL
//! exclusion, and above all the KEYSET TIE: the bounded-OR cursor
//! (`published <= ?p AND (published < ?p OR id < ?id)` for DESC) must cross a run
//! of equal `current_published_at` values without duplicating or dropping a row,
//! because the tie trim is the one part of the shape a plain range test never
//! exercises. Seeded through a raw connection so the ties are constructed, not
//! hoped for.

use store::read::{self, Filter, HeadOrder};
use store::turso::{self, Value};

async fn open(name: &str) -> turso::Connection {
    let path = format!("/tmp/tender-db-pubord-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn
}

/// Ten tenders: ids 1..=10. Published: 1..=4 at T=100 (a four-way tie), 5..=8 at
/// descending distinct instants, 9 at T=900 (the newest), 10 NULL (must never
/// appear). Every one carries a current version so the list JOIN matches.
async fn seed(conn: &turso::Connection) {
    for id in 1..=10i64 {
        let published: Option<i64> = match id {
            1..=4 => Some(100),
            5 => Some(500),
            6 => Some(400),
            7 => Some(300),
            8 => Some(200),
            9 => Some(900),
            _ => None,
        };
        conn.execute(
            "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
             VALUES (?, 'ted', ?, 'procedure', 1, ?, 0)",
            (
                Value::Integer(id),
                Value::Text(format!("pk-{id}")),
                published.map(Value::Integer).unwrap_or(Value::Null),
            ),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
             VALUES (?, 1, ?, ?, ?)",
            (
                Value::Integer(id),
                Value::Integer(published.unwrap_or(0)),
                Value::Text(format!("pub-{id}")),
                Value::Integer(id),
            ),
        )
        .await
        .unwrap();
    }
}

async fn ids(
    conn: &turso::Connection,
    filter: &Filter,
    desc: bool,
    cursor: Option<(i64, i64)>,
    limit: i64,
) -> Vec<i64> {
    read::tenders_ordered(conn, filter, HeadOrder::PublishedAt, desc, cursor, limit)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect()
}

#[tokio::test]
async fn desc_pages_cross_a_publication_tie_without_dup_or_gap() {
    let conn = open("tie").await;
    seed(&conn).await;
    let f = Filter::default();

    // Full DESC order: newest first, ties broken by descending id, NULL absent.
    let all = ids(&conn, &f, true, None, 100).await;
    assert_eq!(all, vec![9, 5, 6, 7, 8, 4, 3, 2, 1], "desc order with tie by id desc, no NULL row");

    // Walk it in pages of 2 — boundaries land INSIDE the T=100 tie — and require
    // the concatenation to equal the one-shot list exactly.
    let mut paged = Vec::new();
    let mut cursor = None;
    loop {
        let rows = read::tenders_ordered(&conn, &f, HeadOrder::PublishedAt, true, cursor, 2).await.unwrap();
        if rows.is_empty() {
            break;
        }
        cursor = rows.last().map(|r| (r.published_at, r.id));
        paged.extend(rows.iter().map(|r| r.id));
        if rows.len() < 2 {
            break;
        }
    }
    assert_eq!(paged, all, "2-row pages must reassemble the exact list across the tie");

    // ASC is the exact reverse (tie by ascending id).
    let asc = ids(&conn, &f, false, None, 100).await;
    assert_eq!(asc, all.iter().rev().copied().collect::<Vec<_>>());
}

#[tokio::test]
async fn the_deadline_ordering_is_the_same_shape_over_its_own_column() {
    // Issue 216, deadline half: identical keyset mechanics over current_deadline.
    // Deadlines deliberately DISAGREE with publication order (tender 9 published
    // newest but closes last; 8 closes soonest), so a pass here proves the
    // ordering actually reads its own column. 10 (NULL both) never appears; 5
    // has a deadline but the tie tenders 1..=4 do not (award-style rows leave
    // the deadline ordering entirely).
    let conn = open("deadline").await;
    seed(&conn).await;
    conn.execute(
        "UPDATE tenders SET current_deadline = CASE id
             WHEN 5 THEN 3000 WHEN 6 THEN 1000 WHEN 7 THEN 2000
             WHEN 8 THEN 500 WHEN 9 THEN 9000 ELSE NULL END",
        (),
    )
    .await
    .unwrap();
    let f = Filter::default();

    // "Closes soon": ascending deadline.
    let soon: Vec<i64> = read::tenders_ordered(&conn, &f, HeadOrder::Deadline, false, None, 100)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect();
    assert_eq!(soon, vec![8, 6, 7, 5, 9], "soonest first; deadline-less rows absent");

    // A deadline window, while a PUBLISHED bound filters within the same read
    // (the other column's bound applies as a plain predicate).
    let f2 = Filter {
        deadline_after: Some(1000),
        deadline_before: Some(9000),
        published_after: Some(300), // drops 8 (published 200) — already out — and keeps 5,6,7
        ..Filter::default()
    };
    let windowed: Vec<i64> = read::tenders_ordered(&conn, &f2, HeadOrder::Deadline, false, None, 100)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect();
    assert_eq!(windowed, vec![6, 7, 5], "deadline window rides the index, published bound filters");
}

#[tokio::test]
async fn range_bounds_are_inclusive_after_exclusive_before() {
    let conn = open("range").await;
    seed(&conn).await;

    // [200, 500): keeps 8 (200), 7 (300), 6 (400); excludes 5 (500) and the tie (100).
    let f = Filter { published_after: Some(200), published_before: Some(500), ..Filter::default() };
    assert_eq!(ids(&conn, &f, true, None, 100).await, vec![6, 7, 8]);

    // A range matching nothing is an empty page, not an error.
    let none = Filter { published_after: Some(901), ..Filter::default() };
    assert!(ids(&conn, &none, true, None, 100).await.is_empty());
}
