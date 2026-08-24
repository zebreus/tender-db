//! Issue 273 step 1 pin: `status` on the TENDERS shapes is a head-range on the
//! indexed `current_deadline` column, never the per-row submission-deadline
//! EXISTS. The EXISTS form combined with a sparse second filter walked the whole
//! deadline-ordered stream to the 30s service bound (`status=open&country=LU` →
//! 503, four of them brown out the walk pool); the range bounds the scan to the
//! open head (0.13s validated on prod, 2026-08-24). Lots shapes keep the EXISTS
//! — no head column there — so this also pins that the split stays split.

use store::read::{tenders_ordered_statement, lots_statement, Filter, HeadOrder, Scope, Status};
use store::turso::{self, Value};

fn f() -> Filter {
    Filter { status: Some(Status::Open), country: Some("LU".into()), now: 1_756_000_000, ..Filter::default() }
}

#[test]
fn tenders_status_is_a_current_deadline_range() {
    let (sql, _) = tenders_ordered_statement(&f(), HeadOrder::Deadline, false, None, 25);
    println!("TENDERS SQL:\n{sql}");
    assert!(sql.contains("t.current_deadline > ?"), "Open must be the head range: {sql}");
    assert!(
        !sql.contains("d.field = 'submission_deadline'"),
        "the walking EXISTS form must be gone from the Tenders shape: {sql}"
    );

    let closed = Filter { status: Some(Status::Closed), ..f() };
    let (sql, _) = tenders_ordered_statement(&closed, HeadOrder::Deadline, false, None, 25);
    assert!(
        sql.contains("(t.current_deadline IS NULL OR t.current_deadline <= ?)"),
        "Closed is NULL-or-past — a Tender that never published a deadline cannot be bid on: {sql}"
    );
}

#[test]
fn lots_status_keeps_the_exists_form() {
    let (sql, _) = lots_statement(&f(), Scope::Page { after: 0, limit: 25 });
    assert!(
        sql.contains("d.field = 'submission_deadline'"),
        "lots have no head deadline column; the EXISTS stays: {sql}"
    );
}

/// Issue 273 step 2 pin: a viable country prefix drives the read from the
/// classifications seed; without the flag the shape is unchanged.
#[test]
fn a_viable_country_drives_from_the_classifications_seed() {
    let seeded = Filter { country_seed: true, ..f() };
    let (sql, _) = tenders_ordered_statement(&seeded, HeadOrder::Deadline, false, None, 25);
    assert!(
        sql.contains("FROM tender_version_classifications") && sql.contains("hits"),
        "the seed must drive: {sql}"
    );
    let (sql, _) = tenders_ordered_statement(&f(), HeadOrder::Deadline, false, None, 25);
    assert!(!sql.contains("hits"), "unseeded stays unseeded: {sql}");
}

/// Issue 273 step 2, the superset trap: the seed enumerates tenders where ANY
/// version matched the prefix, so a tender whose OLD version was CY but whose
/// head no longer is MUST still be excluded — the head-version EXISTS decides
/// membership, the seed only narrows the candidates. On a corpus this small the
/// async probe always says "viable", so `read::tenders` takes the seeded path.
#[tokio::test]
async fn the_country_seed_stays_a_candidate_set_not_an_answer() {
    let path = format!("/tmp/tender-db-seed-{}.db", std::process::id());
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
    // Tender 1: head (seq 2) is CY — must appear. Tender 2: seq 1 was CY, head
    // (seq 2) moved to DE — seeded in, filtered out. Tender 3: never CY.
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
    let filter = Filter { country: Some("CY".into()), now: 1_000, ..Filter::default() };
    let rows = store::read::tenders(&reader, &filter, Scope::Page { after: 0, limit: 25 })
        .await
        .expect("seeded read");
    let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![1], "only the head-CY tender: seeded-in-filtered-out must not leak");
    let _ = Value::Integer(0);
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}
