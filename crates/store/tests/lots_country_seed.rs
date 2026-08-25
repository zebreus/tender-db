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
        sql.contains("FROM tender_version_classifications") && sql.contains("hits"),
        "the seed must drive the lots stream: {sql}"
    );
    assert!(sql.contains("JOIN lots l ON l.tender_id = hits.tender_id"), "seed joins lots on the tender id: {sql}");
    let (sql, _) = lots_statement(&f(), Scope::Page { after: 0, limit: 25 });
    assert!(!sql.contains("hits"), "unseeded stays unseeded: {sql}");
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
    drop(db);
    let _ = Value::Integer(0);
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}
