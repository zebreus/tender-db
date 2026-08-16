//! Issue 218: `read::notice_quarantine` surfaces a held notice's quarantine row —
//! the only content a quarantined notice has, since unrecognised payloads are held
//! whole and never parsed into satellites.
//!
//! Seeded through a raw connection (not the `Db` write API, which does not set
//! `notice_id` on insert — the reclaim path stamps it later), so the row links to a
//! notice id the way a reclaim-addressed hold does.

use store::turso::{self};

async fn open(name: &str) -> turso::Connection {
    let path = format!("/tmp/tender-db-quarantine-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    // `Db::open` builds the schema (the quarantine table + its notice_id index);
    // a raw connection then seeds rows the store write API cannot express.
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn
}

#[tokio::test]
async fn a_held_notice_reports_its_quarantine_hold() {
    let conn = open("held").await;
    conn.execute(
        "INSERT INTO quarantine(fetch_id, member_path, content_hash, notice_id, profile,
             reason, detail, first_seen, attempts, last_attempt_at, reprocessed_at,
             skipped_at, skipped_reason, first_reason, first_detail)
         VALUES (1, 'ted/2026-00136/12.xml', 'h1', 42, 'r209',
             'unparsable-xml', 'XML with DTD detected', 100, 3, 900, NULL,
             NULL, NULL, 'unrecognised-profile', 'no matching mapper')",
        (),
    )
    .await
    .unwrap();

    let held = store::read::notice_quarantine(&conn, 42)
        .await
        .unwrap()
        .expect("notice 42 is held");
    assert_eq!(held.reason, "unparsable-xml");
    assert_eq!(held.detail.as_deref(), Some("XML with DTD detected"));
    assert_eq!(held.profile.as_deref(), Some("r209"));
    assert_eq!(held.first_seen, 100);
    assert_eq!(held.attempts, Some(3));
    assert_eq!(held.last_attempt_at, Some(900));
    assert_eq!(held.reprocessed_at, None);
    // The original cause is preserved distinctly from the current one (issue 87).
    assert_eq!(held.first_reason.as_deref(), Some("unrecognised-profile"));
    assert_eq!(held.first_detail.as_deref(), Some("no matching mapper"));

    // A notice with no quarantine row is not held — one seek, an empty answer.
    assert!(store::read::notice_quarantine(&conn, 999).await.unwrap().is_none());
}

#[tokio::test]
async fn the_newest_hold_wins_when_a_notice_has_more_than_one() {
    let conn = open("newest").await;
    // A re-ingest under a new member path can leave two rows for one notice id; the
    // response shows the current (newest by first_seen) hold.
    conn.execute(
        "INSERT INTO quarantine(fetch_id, member_path, content_hash, notice_id, reason, detail, first_seen)
         VALUES (1, 'a', 'h1', 7, 'not-utf8', 'old', 100),
                (2, 'b', 'h2', 7, 'unparsable-xml', 'new', 500)",
        (),
    )
    .await
    .unwrap();
    let held = store::read::notice_quarantine(&conn, 7).await.unwrap().expect("held");
    assert_eq!(held.reason, "unparsable-xml");
    assert_eq!(held.detail.as_deref(), Some("new"));
}
