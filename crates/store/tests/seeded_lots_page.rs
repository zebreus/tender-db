//! Issue 388, the last clause: an organization-seeded lots read pages in the
//! order its participation index serves, so a page's cost scales with the page
//! and not with the org. The properties that make that safe are pinned here on
//! a fixture small enough to reason about by hand: the walk returns exactly the
//! set the id-ordered stream returns (same head, same predicates — a different
//! ORDER), in `(tender, lot)` order, once each, and terminates — with windows so
//! small that most pages come back short with a cursor, and with the production
//! window where one page holds everything. Membership is decided at the HEAD
//! version: a stale-version win and a subcontractor row admit nothing.

use store::read::{self, Filter, LotCursor, Scope};
use store::turso::{self, Value};

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

const ORG: i64 = 7;
const OTHER: i64 = 8;

/// Nine tenders. Versions per tender vary (1–3) so the seed window, which is in
/// index ROWS, ends inside a tender more than once. Lot ids are handed out so that
/// `(tender, lot)` order is visibly NOT lot-id order: tender 5's lots get the
/// lowest ids, tender 1's the next.
///
/// Winners (`tender_version_result_winners`), org 7:
///   * at the head version of tenders 1, 2, 4, 5, 8, 9 — members;
///   * only at a STALE version of tender 3 — not a member;
///   * tender 7: a member with no lots at all — contributes nothing;
///   * tender 6: org 8 only.
/// Bidders (`tender_version_bid_parties`), org 7:
///   * tenderer at the head of tenders 2, 5, 9 — members;
///   * subcontractor only, tender 4 — not a member;
///   * tenderer only at a stale version, tender 8 — not a member.
async fn seed(conn: &turso::Connection) {
    let versions = [1, 3, 2, 1, 2, 1, 1, 3, 1]; // tenders 1..=9
    for (i, &v) in versions.iter().enumerate() {
        let id = i as i64 + 1;
        conn.execute(
            "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
             VALUES (?, 'ted', ?, 'procedure', ?, 1700000000, 1700000000)",
            (Value::Integer(id), Value::Text(format!("pk-{id}")), Value::Integer(v)),
        )
        .await
        .unwrap();
        for seq in 1..=v {
            conn.execute(
                "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
                 VALUES (?, ?, 1700000000, ?, ?)",
                (
                    Value::Integer(id),
                    Value::Integer(seq),
                    Value::Text(format!("pub-{id}-{seq}")),
                    Value::Integer(id * 10 + seq),
                ),
            )
            .await
            .unwrap();
        }
    }
    // Lots: (lot id, tender). Tender 5 first, then 1, then the rest; tender 7 has none.
    let lots: [(i64, i64); 12] =
        [(1, 5), (2, 5), (3, 1), (4, 1), (5, 2), (6, 3), (7, 4), (8, 4), (9, 6), (10, 8), (11, 9), (12, 9)];
    for (lot, tender) in lots {
        conn.execute(
            "INSERT INTO lots (id, tender_id, lot_key) VALUES (?, ?, ?)",
            (Value::Integer(lot), Value::Integer(tender), Value::Text(format!("LOT-{lot}"))),
        )
        .await
        .unwrap();
        // The lot exists at every version of its tender (the head included).
        let v = versions[(tender - 1) as usize];
        for seq in 1..=v {
            conn.execute(
                "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES (?, ?, ?, 'Lot')",
                (Value::Integer(tender), Value::Integer(seq), Value::Integer(lot)),
            )
            .await
            .unwrap();
        }
    }
    // Winners: (tender, seq, org). Head versions: 1→1, 2→3, 3→2, 4→1, 5→2, 6→1, 7→1, 8→3, 9→1.
    let winners: [(i64, i64, i64); 12] = [
        (1, 1, ORG),
        (2, 1, ORG),
        (2, 3, ORG), // also at the head
        (3, 1, ORG), // STALE only (head is 2)
        (3, 2, OTHER),
        (4, 1, ORG),
        (5, 2, ORG),
        (6, 1, OTHER),
        (7, 1, ORG), // no lots
        (8, 2, ORG),
        (8, 3, ORG),
        (9, 1, ORG),
    ];
    for (n, (tender, seq, org)) in winners.into_iter().enumerate() {
        conn.execute(
            "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id)
             VALUES (?, ?, ?, ?)",
            (Value::Integer(tender), Value::Integer(seq), Value::Integer(100 + n as i64), Value::Integer(org)),
        )
        .await
        .unwrap();
    }
    // Bidders: (tender, seq, role, org).
    let bidders: [(i64, i64, &str, i64); 7] = [
        (2, 3, "tenderer", ORG),
        (4, 1, "subcontractor", ORG), // the role the seed does not admit
        (5, 1, "tenderer", ORG),
        (5, 2, "tenderer", ORG),
        (8, 1, "tenderer", ORG), // STALE only (head is 3)
        (8, 3, "tenderer", OTHER),
        (9, 1, "tenderer", ORG),
    ];
    for (n, (tender, seq, role, org)) in bidders.into_iter().enumerate() {
        conn.execute(
            "INSERT INTO tender_version_bid_parties (tender_id, seq, bid_id, role, organization_id, mention_notice_id, mention_section_id)
             VALUES (?, ?, ?, ?, ?, 1, 'S')",
            (
                Value::Integer(tender),
                Value::Integer(seq),
                Value::Integer(200 + n as i64),
                Value::Text(role.into()),
                Value::Integer(org),
            ),
        )
        .await
        .unwrap();
    }
    // The deferred covering indexes the walk's order comes from (the 225 recipe,
    // built by the issue-111 builder on prod; created here by hand).
    for (name, on) in [
        ("tender_version_result_winners_org_tender", "tender_version_result_winners(organization_id, tender_id)"),
        ("tender_version_bid_parties_org_tender", "tender_version_bid_parties(organization_id, tender_id)"),
        ("tender_version_result_winners_org", "tender_version_result_winners(organization_id)"),
        ("tender_version_bid_parties_org", "tender_version_bid_parties(organization_id)"),
    ] {
        conn.execute(&format!("CREATE INDEX IF NOT EXISTS {name} ON {on}"), ()).await.unwrap();
    }
}

/// Walk the seeded page to the end, returning every page's `(tender, lot)` pairs
/// in the order served, and the number of pages it took.
async fn walk(
    conn: &turso::Connection,
    filter: &Filter,
    limit: i64,
    window: i64,
    windows: usize,
) -> (Vec<(i64, i64)>, usize) {
    let mut cursor = LotCursor::default();
    let mut out = Vec::new();
    let mut pages = 0;
    loop {
        pages += 1;
        assert!(pages <= 60, "the walk must terminate: {out:?}");
        let page = read::lots_seeded_page(conn, filter, cursor, limit, window, windows).await.unwrap();
        assert!(page.rows.len() as i64 <= limit, "a page never exceeds its limit");
        out.extend(page.rows.iter().map(|r| (r.tender_id, r.id)));
        match page.next {
            Some(next) => {
                // A cursor only ever moves forward, so a client following it can
                // neither loop nor re-read.
                assert!((next.tender_id, next.lot_id) > (cursor.tender_id, cursor.lot_id), "{next:?} after {cursor:?}");
                cursor = next;
            }
            None => return (out, pages),
        }
    }
}

/// The id-ordered stream's answer for the same filter — the oracle. A different
/// ORDER BY over the same head and predicates; the SET must agree.
async fn oracle(conn: &turso::Connection, filter: &Filter) -> Vec<(i64, i64)> {
    let mut rows: Vec<(i64, i64)> = read::lots(conn, filter, Scope::Page { after: 0, limit: 1000 })
        .await
        .unwrap()
        .into_iter()
        .map(|r| (r.tender_id, r.id))
        .collect();
    rows.sort();
    rows
}

#[tokio::test]
async fn the_seeded_walk_returns_the_stream_set_in_tender_order_once_and_terminates() {
    let (conn, path) = scratch("seeded-lots").await;
    seed(&conn).await;

    let winner = Filter { winner: Some(ORG), now: 1_756_000_000, ..Filter::default() };
    let bidder = Filter { bidder: Some(ORG), now: 1_756_000_000, ..Filter::default() };
    assert!(read::seeded_lots(&winner) && read::seeded_lots(&bidder));

    // By hand: org 7 wins at the head of 1, 2, 4, 5, 8, 9 (7 has no lots; 3 only stale).
    let expected_winner: Vec<(i64, i64)> =
        vec![(1, 3), (1, 4), (2, 5), (4, 7), (4, 8), (5, 1), (5, 2), (8, 10), (9, 11), (9, 12)];
    // Org 7 is a head-version tenderer on 2, 5, 9 (4 only as subcontractor; 8 only stale).
    let expected_bidder: Vec<(i64, i64)> = vec![(2, 5), (5, 1), (5, 2), (9, 11), (9, 12)];
    assert_eq!(oracle(&conn, &winner).await, expected_winner, "the oracle agrees with the hand count");
    assert_eq!(oracle(&conn, &bidder).await, expected_bidder);

    for (label, filter, expected) in [("winner", &winner, &expected_winner), ("bidder", &bidder, &expected_bidder)] {
        // Tiny windows (2 index rows — less than one deep tender), one window per
        // page, limit 2: most pages are short, several are empty, every cursor is
        // a position the next page resumes from exactly.
        let (small, pages_small) = walk(&conn, filter, 2, 2, 1).await;
        assert_eq!(&small, expected, "{label}: tiny windows return the set in (tender, lot) order");

        // The production window and cap, limit 3: full pages with the cursor at the
        // last row returned, then the last page short with `next: None`.
        let (prod, pages_prod) = walk(&conn, filter, 3, read::DEFAULT_SEED_WINDOW, read::DEFAULT_SEED_WINDOWS_PER_PAGE).await;
        assert_eq!(&prod, expected, "{label}: production windows return the same set in the same order");
        assert_eq!(pages_prod, expected.len().div_ceil(3), "{label}: pages of 3, the last one short");
        assert!(pages_small > pages_prod, "{label}: tiny windows take more pages ({pages_small} > {pages_prod})");

        // One big page holds everything and says so.
        let one = read::lots_seeded_page(&conn, filter, LotCursor::default(), 100, read::DEFAULT_SEED_WINDOW, 8).await.unwrap();
        assert_eq!(one.rows.iter().map(|r| (r.tender_id, r.id)).collect::<Vec<_>>(), *expected);
        assert_eq!(one.next, None, "{label}: the seed is exhausted inside one page");
    }

    // (tender, lot) is not lot-id order here — that is the point of the fixture, and
    // of the compound cursor: the first page's lots are 3 and 4 (tender 1), not 1 and 2.
    let first = read::lots_seeded_page(&conn, &winner, LotCursor::default(), 2, 256, 8).await.unwrap();
    assert_eq!(first.rows.iter().map(|r| r.id).collect::<Vec<_>>(), vec![3, 4]);
    assert_eq!(first.next, Some(LotCursor { tender_id: 1, lot_id: 4 }));

    // A companion filter still narrows inside the walk's head: `kind=Part` is in the
    // vocabulary (reachable, issue 415) but no fixture lot carries it.
    let parts = Filter { winner: Some(ORG), kind: Some("Part".into()), now: 1_756_000_000, ..Filter::default() };
    let none = read::lots_seeded_page(&conn, &parts, LotCursor::default(), 10, 2, 1).await.unwrap();
    assert!(none.rows.is_empty());
    // …and the window cap makes that page short-with-cursor, not a corpus pass.
    assert!(none.next.is_some(), "one window read, the rest of the seed waits behind a cursor");
    // Followed to the end it is still empty, and it still terminates.
    let (parts_all, parts_pages) = walk(&conn, &parts, 10, 2, 1).await;
    assert!(parts_all.is_empty() && parts_pages >= 5, "{parts_pages} short pages, nothing admitted");
    // A companion the guard can answer alone answers alone: no tender of this org is `doe`.
    let elsewhere = Filter { winner: Some(ORG), source: Some("doe".into()), now: 1_756_000_000, ..Filter::default() };
    let unreachable = read::lots_seeded_page(&conn, &elsewhere, LotCursor::default(), 10, 2, 1).await.unwrap();
    assert!(unreachable.rows.is_empty() && unreachable.next.is_none());

    // An organization nobody recorded: unreachable, one guard seek, no walk.
    let nobody = Filter { winner: Some(999), now: 1_756_000_000, ..Filter::default() };
    let empty = read::lots_seeded_page(&conn, &nobody, LotCursor::default(), 10, 256, 8).await.unwrap();
    assert!(empty.rows.is_empty() && empty.next.is_none());

    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

/// The cursor grammar: round-trips, and is not the bare-id grammar.
#[test]
fn the_compound_cursor_round_trips_and_rejects_a_bare_id() {
    let c = LotCursor { tender_id: 7954584, lot_id: 13_200_001 };
    assert_eq!(c.render(), "7954584:13200001");
    assert_eq!(LotCursor::parse(&c.render()), Some(c));
    assert_eq!(LotCursor::parse("13200001"), None, "a bare lot id is another shape's cursor");
    assert_eq!(LotCursor::parse("a:b"), None);
    assert_eq!(LotCursor::parse(""), None);
}

/// The seed window is served by the covering index, in order: the statement the
/// walk's bound and order both come from must not scan or sort the org's rows.
#[tokio::test]
async fn the_seed_window_is_an_index_range_read_in_tender_order() {
    let (conn, path) = scratch("seeded-lots-plan").await;
    seed(&conn).await;
    for (filter, index) in [
        (Filter { winner: Some(ORG), ..Filter::default() }, "tender_version_result_winners_org_tender"),
        (Filter { bidder: Some(ORG), ..Filter::default() }, "tender_version_bid_parties_org_tender"),
    ] {
        let sql = read::seeded_window_sql(&filter).expect("a seeded shape");
        let mut rows = conn
            .query(
                &format!("EXPLAIN QUERY PLAN {sql}"),
                vec![Value::Integer(ORG), Value::Integer(0), Value::Integer(256)],
            )
            .await
            .unwrap();
        let mut plan = Vec::new();
        while let Some(r) = rows.next().await.unwrap() {
            plan.push(r.get_value(3).unwrap().as_text().cloned().unwrap_or_default());
        }
        let text = plan.join("\n");
        eprintln!("seed window plan ({index}):\n{text}");
        assert!(text.contains(index), "the window seeks the covering index: {text}");
        assert!(!text.contains("SCAN"), "no table scan: {text}");
        assert!(!text.to_uppercase().contains("SORT") && !text.contains("TEMP B-TREE"), "the index serves the order: {text}");
    }
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}
