//! Issue 503: a `source` companion on the ordered Tender list must not sort the source's
//! whole slice. `t.source = ?` beside `ORDER BY current_published_at DESC, id DESC LIMIT ?`
//! seeked `tenders_source_id (source, id)` on prod and sorted every Tender of the source
//! before the LIMIT: 17.5 s for `source=ted&sort=published_at`, 11.4 s for the deadline twin.
//! `read::tenders_ordered` now pins `tenders_source_published` / `tenders_source_deadline`,
//! which serve the equality, the order and the keyset cursor as one range at every density,
//! and only when the index exists (`INDEXED BY` fails on a missing one, and a deploy does not
//! build deferred indexes, issue 111).

use store::read::{Filter, HeadOrder, tenders_ordered, tenders_ordered_statement_pinned};
use store::turso::{self, Value};

const N: i64 = 6_000;

fn source_of(i: i64) -> &'static str {
    if i % 10 == 0 {
        "doe"
    } else if i % 13 == 0 {
        "fts"
    } else {
        "ted"
    }
}
fn published_of(i: i64) -> i64 {
    // Ties every third Tender, so the cursor's id tie-break is exercised.
    1_700_000_000 + (i / 3) * 60
}
fn deadline_of(i: i64) -> Option<i64> {
    (i % 7 != 0).then_some(1_710_000_000 + (i * 7919) % 100_000)
}

async fn plan(conn: &turso::Connection, sql: &str, params: Vec<Value>) -> Vec<String> {
    let mut rows = conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), params).await.expect("explain");
    let mut plan = Vec::new();
    while let Some(row) = rows.next().await.expect("plan row") {
        if let Ok(Value::Text(detail)) = row.get_value(3) {
            plan.push(detail);
        }
    }
    assert!(!plan.is_empty(), "no plan came back for: {sql}");
    plan
}

/// Every page of `filter` in `order`, through the real `read::tenders_ordered`, following
/// the keyset cursor to the end.
async fn every_page_of(conn: &turso::Connection, filter: &Filter, order: HeadOrder, desc: bool) -> Vec<i64> {
    let key = |id: i64| match order {
        HeadOrder::PublishedAt => published_of(id),
        HeadOrder::Deadline => deadline_of(id).expect("listed rows carry the key"),
    };
    let mut out = Vec::new();
    let mut cursor = None;
    loop {
        let page = tenders_ordered(conn, filter, order, desc, cursor, 97).await.expect("page");
        let Some(last) = page.last() else { break };
        cursor = Some((key(last.id), last.id));
        out.extend(page.iter().map(|r| r.id));
    }
    out
}

async fn every_page(conn: &turso::Connection, source: &str, order: HeadOrder, desc: bool) -> Vec<i64> {
    every_page_of(conn, &Filter { source: Some(source.into()), ..Filter::default() }, order, desc).await
}

/// The ids `keep` admits from `source`, in `order`, as the list must serve them.
fn expected_where(source: &str, order: HeadOrder, desc: bool, keep: impl Fn(i64) -> bool) -> Vec<i64> {
    let mut rows: Vec<(i64, i64)> = (1..=N)
        .filter(|&i| source_of(i) == source && keep(i))
        .filter_map(|i| match order {
            HeadOrder::PublishedAt => Some((published_of(i), i)),
            HeadOrder::Deadline => deadline_of(i).map(|d| (d, i)),
        })
        .collect();
    rows.sort_unstable();
    if desc {
        rows.reverse();
    }
    rows.into_iter().map(|(_, id)| id).collect()
}

fn expected(source: &str, order: HeadOrder, desc: bool) -> Vec<i64> {
    expected_where(source, order, desc, |_| true)
}

#[tokio::test]
async fn a_source_companion_on_the_ordered_list_reads_an_index_range_not_a_sort() {
    let path = format!("/tmp/tender-db-503-plan-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.expect("open");
    let raw = turso::Builder::new_local(&path).build().await.expect("raw");
    let conn = raw.connect().expect("connect");
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    // A dense source and two sparse ones.
    for chunk in (1..=N).collect::<Vec<_>>().chunks(500) {
        let tenders: Vec<String> = chunk
            .iter()
            .map(|&i| {
                let deadline = deadline_of(i).map_or("NULL".to_owned(), |d| d.to_string());
                format!("({i}, '{}', 'p{i}', 'procedure', 1, {}, {deadline}, 0)", source_of(i), published_of(i))
            })
            .collect();
        conn.execute(
            &format!(
                "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, \
                 current_deadline, created_at) VALUES {}",
                tenders.join(", ")
            ),
            (),
        )
        .await
        .unwrap();
        let versions: Vec<String> =
            chunk.iter().map(|&i| format!("({i}, 1, {}, 'OJ-{i}', {i})", published_of(i))).collect();
        conn.execute(
            &format!(
                "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id) \
                 VALUES {}",
                versions.join(", ")
            ),
            (),
        )
        .await
        .unwrap();
    }
    db.build_tender_indexes().await.expect("the deferred tender indexes");

    // The plan: the window seeks the pinned index and runs no sorter, for a dense, a sparse
    // and an absent source, both directions, first page and a cursor page.
    for order in [HeadOrder::PublishedAt, HeadOrder::Deadline] {
        for desc in [true, false] {
            for source in ["ted", "doe", "nope"] {
                for cursor in [None, Some((1_700_050_000, 4_000))] {
                    let filter = Filter { source: Some(source.into()), ..Filter::default() };
                    let (sql, params) = tenders_ordered_statement_pinned(&filter, order, desc, cursor, 51);
                    let plan = plan(&conn, &sql, params).await;
                    // The window's own lines: everything before the first per-row subquery.
                    let window: Vec<&String> =
                        plan.iter().take_while(|l| !l.starts_with("CORRELATED SCALAR SUBQUERY")).collect();
                    let what = format!("{order:?} desc={desc} source={source} cursor={cursor:?}");
                    let index = order.source_index();
                    assert!(
                        window.iter().any(|l| l.contains(&format!("USING INDEX {index} (source=?"))),
                        "{what}: the window must seek {index} on source:\n{}",
                        plan.join("\n")
                    );
                    assert!(
                        !window.iter().any(|l| l.contains("SORTER") || l.contains("TEMP B-TREE")),
                        "{what}: no sorter may run over the source's slice:\n{}",
                        plan.join("\n")
                    );
                }
            }
        }
    }

    // The answer: every page, through the real read, is exactly the source's Tenders in
    // (key, id) order, tie-broken by id across the cursor.
    for order in [HeadOrder::PublishedAt, HeadOrder::Deadline] {
        for desc in [true, false] {
            for source in ["ted", "doe", "fts", "nope"] {
                assert_eq!(
                    every_page(&conn, source, order, desc).await,
                    expected(source, order, desc),
                    "{order:?} desc={desc} source={source}"
                );
            }
        }
    }

    // A seeded read keeps driving from its `hits` set: no pin.
    let seeded = Filter { source: Some("ted".into()), buyer: Some(7), ..Filter::default() };
    let (sql, _) = tenders_ordered_statement_pinned(&seeded, HeadOrder::PublishedAt, true, None, 51);
    assert!(!sql.contains("INDEXED BY"), "a seeded read is not pinned: {sql}");

    // Which index each shape pins (unit 3): a closed range on the other date column pins
    // that column's index; a one-sided other bound or a value band stays with the planner;
    // `status=open` pins the deadline index; otherwise the ordering column's own index.
    const NOW: i64 = 1_710_050_000;
    let ted = || Filter { source: Some("ted".into()), now: NOW, ..Filter::default() };
    let open = || Filter { status: Some(store::read::Status::Open), ..ted() };
    let closed = || Filter { status: Some(store::read::Status::Closed), ..ted() };
    let (published, deadline) = (Some("tenders_source_published"), Some("tenders_source_deadline"));
    for (order, filter, pin, what) in [
        (HeadOrder::Deadline, Filter { published_after: Some(1), published_before: Some(2), ..ted() }, published, "closed published range under sort=deadline"),
        (HeadOrder::PublishedAt, Filter { deadline_after: Some(1), deadline_before: Some(2), ..ted() }, deadline, "closed deadline range under sort=published_at"),
        (HeadOrder::PublishedAt, Filter { deadline_after: Some(1), ..ted() }, None, "one-sided deadline bound under sort=published_at"),
        (HeadOrder::Deadline, Filter { published_after: Some(1), ..ted() }, None, "one-sided published bound under sort=deadline"),
        (HeadOrder::PublishedAt, Filter { min_value: Some(1), ..ted() }, None, "value bound"),
        (HeadOrder::PublishedAt, open(), deadline, "status=open under sort=published_at"),
        (HeadOrder::Deadline, open(), deadline, "status=open under sort=deadline"),
        (HeadOrder::Deadline, Filter { published_after: Some(1), published_before: Some(2), ..open() }, deadline, "status=open beats a closed published range"),
        (HeadOrder::PublishedAt, Filter { deadline_after: Some(1), ..open() }, deadline, "status=open beats a one-sided deadline bound"),
        (HeadOrder::PublishedAt, Filter { min_value: Some(1), ..open() }, deadline, "status=open beats a value band"),
        (HeadOrder::PublishedAt, closed(), published, "status=closed under sort=published_at"),
        (HeadOrder::Deadline, closed(), deadline, "status=closed under sort=deadline"),
        (HeadOrder::PublishedAt, Filter { published_after: Some(1), published_before: Some(2), ..ted() }, published, "own range under sort=published_at"),
        (HeadOrder::Deadline, Filter { deadline_before: Some(2), kind: Some("procedure".into()), ..ted() }, deadline, "own bound and kind under sort=deadline"),
    ] {
        let (sql, params) = tenders_ordered_statement_pinned(&filter, order, true, None, 51);
        match pin {
            Some(index) => {
                assert!(sql.contains(&format!("INDEXED BY {index}")), "{what}: pins {index}: {sql}");
                // The window seeks it on the source; a pin on the other column sorts its range.
                let plan = plan(&conn, &sql, params).await;
                assert!(
                    plan.iter().any(|l| l.contains(&format!("USING INDEX {index} (source=?"))),
                    "{what}: the window must seek {index}:\n{}",
                    plan.join("\n")
                );
            }
            None => assert!(!sql.contains("INDEXED BY"), "{what}: unpinned: {sql}"),
        }
    }

    // And every pinned or unpinned shape answers exactly the rows the filter admits, in
    // order, page after page, for a dense and a sparse source.
    let is_open = |i: i64| deadline_of(i).is_some_and(|d| d > NOW);
    for source in ["ted", "doe"] {
        let with = |f: Filter| Filter { source: Some(source.into()), ..f };
        let cases: [(HeadOrder, Filter, Box<dyn Fn(i64) -> bool>, &str); 7] = [
            (HeadOrder::PublishedAt, with(open()), Box::new(is_open), "status=open by published"),
            (HeadOrder::PublishedAt, with(closed()), Box::new(move |i| !is_open(i)), "status=closed by published"),
            (HeadOrder::Deadline, with(closed()), Box::new(move |i| !is_open(i)), "status=closed by deadline"),
            (
                HeadOrder::PublishedAt,
                with(Filter { deadline_after: Some(1_710_020_000), deadline_before: Some(1_710_030_000), ..ted() }),
                Box::new(|i| deadline_of(i).is_some_and(|d| (1_710_020_000..1_710_030_000).contains(&d))),
                "closed deadline range by published",
            ),
            (
                HeadOrder::Deadline,
                with(Filter { published_after: Some(1_700_030_000), published_before: Some(1_700_060_000), ..ted() }),
                Box::new(|i| (1_700_030_000..1_700_060_000).contains(&published_of(i))),
                "closed published range by deadline",
            ),
            (
                HeadOrder::Deadline,
                with(Filter { published_after: Some(1_700_030_000), published_before: Some(1_700_060_000), ..open() }),
                Box::new(move |i| is_open(i) && (1_700_030_000..1_700_060_000).contains(&published_of(i))),
                "status=open with a closed published range by deadline",
            ),
            (
                HeadOrder::PublishedAt,
                with(Filter { deadline_after: Some(1_710_090_000), ..ted() }),
                Box::new(|i| deadline_of(i).is_some_and(|d| d >= 1_710_090_000)),
                "one-sided deadline bound by published (unpinned)",
            ),
        ];
        for (order, filter, keep, what) in cases {
            for desc in [true, false] {
                assert_eq!(
                    every_page_of(&conn, &filter, order, desc).await,
                    expected_where(source, order, desc, &keep),
                    "{source} {what} desc={desc}"
                );
            }
        }
    }

    // Before the Reindex job builds the pinned index (issue 111), the read is unpinned and
    // still answers, identically.
    conn.execute("DROP INDEX tenders_source_published", ()).await.unwrap();
    assert_eq!(
        every_page(&conn, "ted", HeadOrder::PublishedAt, true).await,
        expected("ted", HeadOrder::PublishedAt, true),
        "the missing index falls back to the planner's choice"
    );

    drop(conn);
    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
