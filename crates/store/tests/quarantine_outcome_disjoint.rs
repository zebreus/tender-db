//! Issue 288: the three quarantine outcomes (outstanding / reclaimed / skipped)
//! are DISJOINT, and reclaimed wins — a skip is a policy statement, a reclaim is
//! the fact that the content now lives in the parsed layer. Pins all three edges:
//! the skip path never stamps a reclaimed row, a genuine reclaim flips a skipped
//! row (clearing the skip), and the resolution card counts a historical
//! both-stamped row exactly once (reprocessed wins — the same rule as the
//! dashboard header's split counter, so the two surfaces cannot disagree).

use store::turso::{self, Value};
use store::{Notice, Parse, Parsed, Reclaim};

async fn open(name: &str) -> (store::Db, turso::Connection, String) {
    let path = format!("/tmp/tender-db-qdisjoint-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.expect("open");
    let raw = turso::Builder::new_local(&path).build().await.expect("raw");
    let conn = raw.connect().expect("connect");
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.expect("fk off");
    (db, conn, path)
}

/// A held quarantine row (profile-level: notice_id NULL) with chosen stamps.
async fn seed(
    conn: &turso::Connection,
    member_path: &str,
    reason: &str,
    reprocessed_at: Option<i64>,
    skipped_at: Option<i64>,
) {
    conn.execute(
        "INSERT INTO quarantine(fetch_id, member_path, content_hash, notice_id, profile,
             reason, detail, first_seen, attempts, last_attempt_at, reprocessed_at,
             skipped_at, skipped_reason, first_reason, first_detail)
         VALUES (1, ?, 'h', NULL, 'r209', ?, 'd', 100, 1, 100, ?, ?, ?, NULL, NULL)",
        (
            Value::Text(member_path.into()),
            Value::Text(reason.into()),
            reprocessed_at.map_or(Value::Null, Value::Integer),
            skipped_at.map_or(Value::Null, Value::Integer),
            skipped_at.map_or(Value::Null, |_| Value::Text("old-policy".into())),
        ),
    )
    .await
    .expect("seed quarantine row");
}

async fn row(conn: &turso::Connection, member_path: &str) -> (Option<i64>, Option<i64>, Option<String>) {
    let mut rows = conn
        .query(
            "SELECT reprocessed_at, skipped_at, skipped_reason FROM quarantine WHERE member_path = ?",
            (Value::Text(member_path.into()),),
        )
        .await
        .expect("query");
    let r = rows.next().await.expect("next").expect("row exists");
    let int_of = |v: &Value| match v {
        Value::Integer(i) => Some(*i),
        _ => None,
    };
    let text_of = |v: &Value| match v {
        Value::Text(s) => Some(s.clone()),
        _ => None,
    };
    (
        int_of(&r.get_value(0).unwrap()),
        int_of(&r.get_value(1).unwrap()),
        text_of(&r.get_value(2).unwrap()),
    )
}

#[tokio::test]
async fn a_policy_skip_never_stamps_a_reclaimed_row() {
    let (db, conn, path) = open("skip-guard").await;
    seed(&conn, "pkg/F1", "not-utf8", Some(500), None).await; // already RECLAIMED
    seed(&conn, "pkg/F2", "not-utf8", None, None).await; // outstanding

    let flagged = db
        .flag_skipped_members(1, &[("pkg/F1".to_owned(), "policy-x"), ("pkg/F2".to_owned(), "policy-x")], 900)
        .await
        .expect("flag");
    assert_eq!(flagged, 1, "only the outstanding row is skippable");

    let (rep, skip, _) = row(&conn, "pkg/F1").await;
    assert_eq!((rep, skip), (Some(500), None), "the reclaimed row keeps its single outcome");
    let (rep, skip, _) = row(&conn, "pkg/F2").await;
    assert_eq!((rep, skip), (None, Some(900)), "the outstanding row is skipped normally");

    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

#[tokio::test]
async fn a_genuine_reclaim_flips_a_skipped_row_to_reclaimed() {
    let (db, conn, path) = open("reclaim-wins").await;
    // A real fetch row: reclaim_notice runs on the Db's writer (foreign_keys ON),
    // so the notice's fetch_id must exist.
    db.record_fetch(&store::Fetch {
        source: "ted".into(),
        kind: "daily".into(),
        period: "2026-00001".into(),
        url: "https://example.invalid/pkg".into(),
        sha256: "aa".into(),
        bytes: 1,
        fetched_at: 0,
        path: "ted/daily/2026-00001.tar.gz".into(),
    })
    .await
    .expect("record fetch");
    let fetch_id =
        db.current_packages("ted", "daily", None).await.expect("packages")[0].fetch_id;
    // Re-point the seeded row at the real fetch.
    seed(&conn, "pkg/F3", "not-utf8", None, Some(400)).await; // policy-SKIPPED
    conn.execute(
        "UPDATE quarantine SET fetch_id = ? WHERE member_path = 'pkg/F3'",
        (Value::Integer(fetch_id),),
    )
    .await
    .expect("repoint fetch");

    // A later reprocess actually parses the member: the reclaim's None-arm (no
    // notice row exists) records the notice and stamps by (fetch_id, member_path).
    let outcome = db
        .reclaim_notice(
            &Notice {
                source: "ted".into(),
                publication_id: "P-F3".into(),
                content_hash: "h-f3".into(),
                profile: "r209".into(),
                declared_version: None,
                fetch_id,
                member_path: "pkg/F3".into(),
                ingested_at: 800,
                published_at: Some(store::Stamp::utc(0)),
                dispatched_at: None,
            },
            &Parse::Parsed(Parsed::default()),
        )
        .await
        .expect("reclaim");
    assert!(matches!(outcome, Reclaim::Reclaimed), "the member genuinely reclaimed");

    let (rep, skip, skip_reason) = row(&conn, "pkg/F3").await;
    assert_eq!(rep, Some(800), "reclaimed is the recorded outcome");
    assert_eq!((skip, skip_reason), (None, None), "the stale skip is cleared — reclaimed wins (issue 288)");

    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

#[tokio::test]
async fn resolution_counts_a_both_stamped_row_once_reprocessed_wins() {
    let (db, conn, path) = open("resolution").await;
    seed(&conn, "pkg/A", "mixed-r", Some(500), None).await; // reclaimed
    seed(&conn, "pkg/B", "mixed-r", None, Some(400)).await; // skipped
    seed(&conn, "pkg/C", "mixed-r", Some(500), Some(400)).await; // historical BOTH
    seed(&conn, "pkg/D", "mixed-r", None, None).await; // outstanding

    let (reclaimed, skipped, outstanding) =
        db.quarantine_resolution("mixed-r", None, None, None, None).await.expect("resolution");
    assert_eq!(
        (reclaimed, skipped, outstanding),
        (2, 1, 1),
        "the both-stamped row counts once, as reclaimed — reclaimed+skipped+outstanding == rows"
    );

    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
