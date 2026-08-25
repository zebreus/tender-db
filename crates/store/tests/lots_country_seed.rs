//! Issue 275 pins: the sparse-country seed (issue 273 step 2) drives the LOTS
//! stream too. `status=open&country=LU` measured 30s→503 on prod 2026-08-25
//! because `/v1/lots` kept walking `lots` with per-row EXISTS predicates after
//! the tenders fix — the seed bounds the candidates, the untouched predicates
//! still decide membership.

use store::read::{lots_statement, Filter, Scope, Status};
use store::turso::{self, Value};

fn f() -> Filter {
    Filter { status: Some(Status::Open), country: Some("LU".into()), now: 1_756_000_000, ..Filter::default() }
}

#[test]
fn a_viable_country_drives_the_lots_stream_from_the_classifications_seed() {
    let seeded = Filter { country_seed: true, ..f() };
    let (sql, _) = lots_statement(&seeded, Scope::Page { after: 0, limit: 25 });
    assert!(
        sql.contains("l.tender_id IN (SELECT DISTINCT tender_id")
            && sql.contains("FROM tender_version_classifications"),
        "a viable country must seed as an IN semi-join (the JOIN form inverts, measured 2.1s vs 0.32s): {sql}"
    );
    // status=open + an over-cap country (country_seed false): the open-head arm.
    let (sql, _) = lots_statement(&f(), Scope::Page { after: 0, limit: 25 });
    assert!(
        sql.contains("l.tender_id IN (SELECT t.id FROM tenders t") && sql.contains("t.current_deadline > ?"),
        "over-cap country with status=open must drive from the open head: {sql}"
    );
    // no country at all: no seed of either kind.
    let bare = Filter { status: Some(Status::Open), now: 1_756_000_000, ..Filter::default() };
    let (sql, _) = lots_statement(&bare, Scope::Page { after: 0, limit: 25 });
    assert!(!sql.contains("l.tender_id IN"), "bare status stays unseeded (fills from the dense walk): {sql}");
}

/// The superset trap, lots edition (the tenders twin lives in
/// status_head_range.rs): the seed enumerates tenders where ANY version matched
/// the prefix, so a lot whose tender's OLD version was CY but whose head moved
/// away MUST still be excluded — the head-version predicates decide membership.
/// On a corpus this small the async probe always says "viable", so `read::lots`
/// takes the seeded path; a lowercase `cy` must return the same rows (the
/// case-variant range union IS the LIKE set — the regression the first seed
/// shipment hit on tenders).
#[tokio::test]
async fn the_lots_country_seed_stays_a_candidate_set_not_an_answer() {
    let path = format!("/tmp/tender-db-lots-seed-{}.db", std::process::id());
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
    let db = store::Db::open(&path).await.expect("open");
    let raw = turso::Builder::new_local(&path).build().await.expect("raw");
    let conn = raw.connect().expect("connect");
    let exec = |sql: String| {
        let conn = conn.clone();
        async move { conn.execute(&sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}")) }
    };
    // Tender 1: head (seq 2) is CY — its lot must appear. Tender 2: seq 1 was
    // CY, head moved to DE — seeded in, filtered out. Tender 3: never CY.
    for id in [1, 2, 3] {
        exec(format!(
            "INSERT INTO tenders (id, source, kind, current_seq, current_published_at, created_at)
             VALUES ({id}, 'ted', 'procedure', 2, 100, 0)"
        ))
        .await;
        for seq in [1, 2] {
            exec(format!(
                "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
                 VALUES ({id}, {seq}, {}, 'pub-{id}-{seq}', {})",
                90 + seq, id * 10 + seq
            ))
            .await;
        }
        exec(format!("INSERT INTO lots (id, tender_id, lot_key) VALUES ({id}, {id}, 'LOT-1')")).await;
        for seq in [1, 2] {
            exec(format!(
                "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind)
                 VALUES ({id}, {seq}, {id}, 'Lot')"
            ))
            .await;
        }
    }
    for (tender, seq, code) in
        [(1, 1, "CY000"), (1, 2, "CY000"), (2, 1, "CY000"), (2, 2, "DE300"), (3, 1, "DE300"), (3, 2, "DE300")]
    {
        exec(format!(
            "INSERT INTO tender_version_classifications (tender_id, seq, lot_id, field, scheme, code)
             VALUES ({tender}, {seq}, NULL, 'place', 'nuts', '{code}')"
        ))
        .await;
    }
    let reader = raw.connect().expect("reader");
    for country in ["CY", "cy"] {
        let filter = Filter { country: Some(country.into()), now: 1_000, ..Filter::default() };
        let rows = store::read::lots(&reader, &filter, Scope::Page { after: 0, limit: 25 })
            .await
            .expect("seeded lots read");
        let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![1], "?country={country}: only the head-CY tender's lot; seeded-in-filtered-out must not leak");
    }
    // The issue-275 source guard, both ways: an absent source short-circuits to
    // the CORRECT empty page (no row carries it, so empty is the answer, not an
    // approximation); a present source changes nothing about the match.
    for (source, want) in [("nonexistent-source", vec![]), ("ted", vec![1i64])] {
        let filter = Filter {
            country: Some("CY".into()),
            source: Some(source.into()),
            now: 1_000,
            ..Filter::default()
        };
        let rows = store::read::lots(&reader, &filter, Scope::Page { after: 0, limit: 25 })
            .await
            .expect("source-filtered lots read");
        let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
        assert_eq!(ids, want, "?source={source}: the guard must drop nothing that matches");
    }
    // The open-head arm behaves: on a tiny corpus the viability probe always
    // arms the country seed, so exercise the over-cap path by running the
    // statement it would generate (country_seed forced false) directly. The
    // head pointers and per-lot deadline rows are set so tender 1 is open-CY,
    // tender 2 open-but-DE at head, tender 3 closed — only lot 1 may return.
    for id in [1, 2] {
        exec(format!("UPDATE tenders SET current_deadline = 2000 WHERE id = {id}")).await;
        exec(format!(
            "INSERT INTO tender_version_dates (tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time)
             VALUES ({id}, 2, NULL, 'submission_deadline', 2000, 0, 1)"
        ))
        .await;
    }
    let over_cap = Filter {
        status: Some(store::read::Status::Open),
        country: Some("CY".into()),
        country_seed: false,
        now: 1_000,
        ..Filter::default()
    };
    let (sql, params) = store::read::lots_statement(&over_cap, Scope::Page { after: 0, limit: 25 });
    assert!(sql.contains("t.current_deadline > ?"), "the open-head arm must be in play: {sql}");
    let mut rows = reader.query(&sql, params).await.expect("open-head statement runs");
    let mut ids = Vec::new();
    while let Some(row) = rows.next().await.expect("row") {
        ids.push(match row.get_value(0).expect("id") {
            Value::Integer(v) => v,
            other => panic!("unexpected id value {other:?}"),
        });
    }
    assert_eq!(ids, vec![1], "open-head seed: only the open, head-CY tender's lot");
    drop(db);
    let _ = Value::Integer(0);
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}
