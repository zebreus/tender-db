//! Issue 423: `status=open` with a `cpv` prefix seeds the LOTS stream from the
//! open head, as issue 275 did for an over-cap country. On prod 2026-09-26
//! `/v1/lots?status=open&cpv=45&limit=100` walked 500k lot ids per page paying the
//! head, cpv and status EXISTS on every one, and answered 503 at the 30 s bound
//! (`cpv=72&limit=10` took 14.4 s); the same page seeded from the 8,647 open
//! cpv-45 tenders filled in ~1–2 s. The seed is a candidate set — the per-lot
//! predicates at the lot's own version still decide membership.

use store::read::{lots_statement, Filter, Scope, Status};
use store::turso::{self, Value};

/// 2026-09-14 — real instants, since issue 171's floor refuses anything before 1990.
const NOW: i64 = 1_789_344_000;
const PUBLISHED: i64 = 1_780_000_000;
const DEADLINE: i64 = NOW + 30 * 86_400;

fn open(cpv: Option<&str>, country: Option<&str>) -> Filter {
    Filter {
        status: Some(Status::Open),
        cpv: cpv.map(str::to_owned),
        country: country.map(str::to_owned),
        now: NOW,
        ..Filter::default()
    }
}

#[test]
fn a_cpv_prefix_with_status_open_drives_the_lots_stream_from_the_open_head() {
    let (sql, params) = lots_statement(&open(Some("45"), None), Scope::Page { after: 0, limit: 25 });
    assert!(
        sql.contains("l.tender_id IN (SELECT t.id FROM tenders t") && sql.contains("t.current_deadline > ?"),
        "cpv + status=open must drive from the open head: {sql}"
    );
    assert!(
        sql.contains("c.seq = t.current_seq") && sql.contains("c.scheme = 'cpv'"),
        "the cpv prefix is tested per TENDER at its head version inside the seed: {sql}"
    );
    assert!(
        params.iter().filter(|p| matches!(p, Value::Text(s) if s == "45%")).count() >= 2,
        "the prefix binds twice — once in the seed, once in the per-lot predicate that still decides"
    );

    // Both prefixes at once (over-cap country): one seed carrying both tests.
    let (sql, _) = lots_statement(&open(Some("45"), Some("DE")), Scope::Page { after: 0, limit: 25 });
    assert_eq!(sql.matches("l.tender_id IN (SELECT t.id FROM tenders t").count(), 1, "one seed: {sql}");
    assert!(sql.contains("c.scheme = 'nuts'") && sql.contains("c.scheme = 'cpv'"), "{sql}");

    // What stays unseeded, deliberately: bare status (dense), cpv without status.
    for (label, filter) in [
        ("bare status=open", open(None, None)),
        ("cpv without status", Filter { cpv: Some("45".into()), now: NOW, ..Filter::default() }),
    ] {
        let (sql, _) = lots_statement(&filter, Scope::Page { after: 0, limit: 25 });
        assert!(!sql.contains("l.tender_id IN"), "{label} stays unseeded: {sql}");
    }
}

/// The superset trap, cpv edition: the seed reads the tender's HEAD cpv and the
/// head deadline; the lot's own predicates re-decide at the lot's version. Tender
/// 1 is open and head-cpv 45 (its lot returns); tender 2 WAS cpv 45 at seq 1 but
/// its head moved to 72 (seeded out, and the per-lot predicate agrees); tender 3
/// is cpv 45 but closed; tender 4 is open cpv 45 with a year-3005 typo as its only
/// deadline — its head column is NULL, so the seed and issue 422's horizon agree.
#[tokio::test]
async fn the_cpv_open_seed_returns_exactly_the_unseeded_answer() {
    let path = format!("/tmp/tender-db-lots-open-seed-{}.db", std::process::id());
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
    let db = store::Db::open(&path).await.expect("open");
    let raw = turso::Builder::new_local(&path).build().await.expect("raw");
    let conn = raw.connect().expect("connect");
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.expect("fk off");
    let exec = |sql: String| {
        let conn = conn.clone();
        async move { conn.execute(&sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}")) }
    };
    let horizon = store::canonical::DEADLINE_HORIZON_SECS;
    // (tender, head deadline column, deadline row at seq 2, cpv at seq 1, cpv at seq 2)
    let cases: [(i64, Option<i64>, i64, &str, &str); 4] = [
        (1, Some(DEADLINE), DEADLINE, "45000000", "45000000"),
        (2, Some(DEADLINE), DEADLINE, "45000000", "72000000"),
        (3, Some(NOW - 86_400), NOW - 86_400, "45000000", "45000000"),
        (4, None, PUBLISHED + horizon + 86_400, "45000000", "45000000"),
    ];
    for (id, head_deadline, deadline, cpv1, cpv2) in cases {
        exec(format!(
            "INSERT INTO tenders (id, source, kind, current_seq, current_published_at, created_at, current_deadline)
             VALUES ({id}, 'ted', 'procedure', 2, {PUBLISHED}, 0, {})",
            head_deadline.map_or("NULL".to_owned(), |d| d.to_string())
        ))
        .await;
        for (seq, cpv) in [(1, cpv1), (2, cpv2)] {
            exec(format!(
                "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
                 VALUES ({id}, {seq}, {PUBLISHED}, 'pub-{id}-{seq}', {})",
                id * 10 + seq
            ))
            .await;
            exec(format!(
                "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES ({id}, {seq}, {id}, 'Lot')"
            ))
            .await;
            exec(format!(
                "INSERT INTO tender_version_classifications (tender_id, seq, lot_id, field, scheme, code)
                 VALUES ({id}, {seq}, NULL, 'main', 'cpv', '{cpv}')"
            ))
            .await;
        }
        exec(format!("INSERT INTO lots (id, tender_id, lot_key) VALUES ({id}, {id}, 'LOT-1')")).await;
        exec(format!(
            "INSERT INTO tender_version_dates (tender_id, seq, lot_id, field, utc_seconds, offset_minutes, has_time)
             VALUES ({id}, 2, NULL, 'submission_deadline', {deadline}, 0, 1)"
        ))
        .await;
    }
    let reader = raw.connect().expect("reader");
    let rows = store::read::lots(&reader, &open(Some("45"), None), Scope::Page { after: 0, limit: 25 })
        .await
        .expect("seeded lots read");
    let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![1], "only the open, head-cpv-45 tender's lot");
    assert_eq!(rows[0].deadline.map(|d| d.utc_seconds), Some(DEADLINE));

    // The same filter with the seed stripped — the per-lot predicates alone —
    // must agree, or the seed is deciding membership rather than bounding it.
    let (sql, params) = lots_statement(&open(Some("45"), None), Scope::Page { after: 0, limit: 25 });
    let start = sql.find(" AND l.tender_id IN (SELECT t.id FROM tenders t").expect("seed present");
    let open_paren = start + sql[start..].find('(').expect("seed paren");
    let mut depth = 0;
    let mut end = open_paren;
    for (i, ch) in sql[open_paren..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = open_paren + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    let unseeded = format!("{}{}", &sql[..start], &sql[end..]);
    assert!(!unseeded.contains("t.current_deadline"), "the whole seed is stripped: {unseeded}");
    let seed_params = 2; // `now` and the prefix
    let first_seed_param = sql[..start].matches('?').count();
    let mut unseeded_params = params.clone();
    unseeded_params.drain(first_seed_param..first_seed_param + seed_params);
    let mut got = reader.query(&unseeded, unseeded_params).await.expect("unseeded runs");
    let mut plain = Vec::new();
    while let Some(row) = got.next().await.expect("row") {
        plain.push(match row.get_value(0).expect("id") {
            Value::Integer(v) => v,
            other => panic!("unexpected id value {other:?}"),
        });
    }
    assert_eq!(plain, ids, "the unseeded walk and the seeded one return the same lots");
    drop(db);
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}
