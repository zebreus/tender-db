//! Issue 278: the ghost census must count every notice claimed by two Tenders,
//! and must not lose one in a slice seam.
//!
//! The census exists because its predecessor — one unbounded
//! `GROUP BY caused_by_notice_id HAVING COUNT(DISTINCT tender_id) > 1` over
//! ~12.4M rows — stalled production for 40+ minutes, uncancellable, with the job
//! queue behind it. Slicing the walk is what makes it safe, so the properties
//! worth pinning are the ones slicing could plausibly break: a ghost in the first
//! slice, a ghost in a later slice, a ghost in the final partial slice, and the
//! separation of what is COUNTED from what is REPORTED.

use store::turso;

async fn fixture(tag: &str) -> (String, store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-ghost-{tag}-{}.db", std::process::id());
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
    let db = store::Db::open(&path).await.expect("open");
    // A raw connection, deliberately: it does not carry the store's
    // `foreign_keys = ON` pragma, so the fixture can name notice ids without
    // building a whole fetch/notice provenance chain behind each one.
    let raw = turso::Builder::new_local(&path).build().await.expect("raw");
    let conn = raw.connect().expect("connect");
    (path, db, conn)
}

async fn exec(conn: &turso::Connection, sql: String) {
    conn.execute(&sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
}

/// One notice row, so the census has an upper bound to walk to.
async fn notice(conn: &turso::Connection, id: i64) {
    exec(
        conn,
        format!(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at)
             VALUES ({id}, 'ted', 'pub-{id}', 'hash-{id}', 'eforms-sdk-1.13', 1, 'm/{id}.xml', 0)"
        ),
    )
    .await;
}

/// A Tender claiming `notice_id` at `seq`.
async fn claim(conn: &turso::Connection, tender_id: i64, seq: i64, notice_id: i64) {
    exec(
        conn,
        format!(
            "INSERT OR IGNORE INTO tenders (id, source, kind, current_seq, created_at)
             VALUES ({tender_id}, 'ted', 'procedure', {seq}, 0)"
        ),
    )
    .await;
    exec(
        conn,
        format!(
            "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
             VALUES ({tender_id}, {seq}, 100, 'pub-{tender_id}-{seq}', {notice_id})"
        ),
    )
    .await;
}

fn silent() -> impl Fn(u64, &str) + Sync {
    |_, _| {}
}

#[tokio::test]
async fn a_clean_corpus_reports_no_ghosts() {
    let (path, db, conn) = fixture("clean").await;
    for id in [1, 2, 3] {
        notice(&conn, id).await;
        claim(&conn, id, 1, id).await;
    }
    let r = db.ghost_census(10, 100, &|| false, &silent()).await.expect("census");
    assert_eq!(r.ghost_notices, 0, "every notice keys to exactly one Tender");
    assert_eq!(r.ghost_tender_refs, 0);
    assert!(r.sample.is_empty());
    assert!(!r.truncated);
    assert!(!r.stopped);
    assert_eq!(r.max_notice_id, 3, "the walk is bounded by the newest notice");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

#[tokio::test]
async fn a_ghost_pair_is_counted_and_named() {
    let (path, db, conn) = fixture("pair").await;
    notice(&conn, 1).await;
    notice(&conn, 2).await;
    claim(&conn, 10, 1, 1).await;
    claim(&conn, 10, 2, 2).await;
    // Notice 2 claimed by a SECOND Tender — the ghost signature.
    claim(&conn, 11, 1, 2).await;

    let r = db.ghost_census(100, 100, &|| false, &silent()).await.expect("census");
    assert_eq!(r.ghost_notices, 1, "one notice is claimed twice");
    assert_eq!(r.ghost_tender_refs, 2, "by two Tenders");
    assert_eq!(
        r.ghost_tender_refs - r.ghost_notices,
        1,
        "so exactly one of the two is surplus — the ghost"
    );
    assert_eq!(r.sample.len(), 1);
    assert_eq!(r.sample[0].notice_id, 2);
    assert_eq!(r.sample[0].tenders, 2);
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

/// THE slicing property. The walk is exact because the slice key IS the
/// `GROUP BY` key — no group can straddle a boundary — but "exact by argument"
/// is worth an experiment, so: ghosts in the first slice, a middle slice, and
/// the final PARTIAL slice, with a window that makes all three separate reads.
#[tokio::test]
async fn ghosts_are_found_in_every_slice_including_the_last_partial_one() {
    let (path, db, conn) = fixture("slices").await;
    // Window 10 over ids 1..25 → slices 1-10, 11-20, 21-25 (the last partial).
    for id in [3, 5, 14, 23, 25] {
        notice(&conn, id).await;
    }
    let mut tender = 100;
    for id in [3, 14, 23, 25] {
        claim(&conn, tender, 1, id).await;
        claim(&conn, tender + 1, 1, id).await;
        tender += 2;
    }
    // A lone claim in the first slice, to prove a single claim never counts.
    claim(&conn, 999, 1, 5).await;

    let r = db.ghost_census(10, 100, &|| false, &silent()).await.expect("census");
    assert_eq!(r.max_notice_id, 25);
    assert_eq!(r.slices, 3, "1-10, 11-20, 21-25");
    assert_eq!(r.ghost_notices, 4, "one in the first slice, one in the middle, two in the last");
    assert_eq!(r.ghost_tender_refs, 8);
    let found: Vec<i64> = r.sample.iter().map(|g| g.notice_id).collect();
    assert_eq!(found, vec![3, 14, 23, 25], "and each is named, in id order");

    // The same corpus under a single slice must agree exactly — the window is a
    // performance choice, never a semantic one.
    let whole = db.ghost_census(1_000_000, 100, &|| false, &silent()).await.expect("census");
    assert_eq!(whole.slices, 1);
    assert_eq!(whole.ghost_notices, r.ghost_notices, "window width must not change the answer");
    assert_eq!(whole.ghost_tender_refs, r.ghost_tender_refs);
    assert_eq!(
        whole.sample.iter().map(|g| g.notice_id).collect::<Vec<_>>(),
        found,
        "nor which notices are named"
    );
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

/// `cap` bounds what is REPORTED, never what is COUNTED. Issue 326 inverted a
/// whole set of conclusions by letting a cap reach the totals; the census must
/// not repeat it.
#[tokio::test]
async fn the_cap_bounds_the_sample_and_never_the_counts() {
    let (path, db, conn) = fixture("cap").await;
    let mut tender = 100;
    for id in 1..=6 {
        notice(&conn, id).await;
        claim(&conn, tender, 1, id).await;
        claim(&conn, tender + 1, 1, id).await;
        tender += 2;
    }
    let r = db.ghost_census(2, 2, &|| false, &silent()).await.expect("census");
    assert_eq!(r.ghost_notices, 6, "all six are counted");
    assert_eq!(r.ghost_tender_refs, 12);
    assert_eq!(r.sample.len(), 2, "but only two are reported");
    assert!(r.truncated, "and the report says so");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

#[tokio::test]
async fn a_stop_flag_aborts_the_walk_and_says_so() {
    let (path, db, conn) = fixture("stop").await;
    for id in 1..=30 {
        notice(&conn, id).await;
    }
    let r = db.ghost_census(1, 100, &|| true, &silent()).await.expect("census");
    assert!(r.stopped, "a cancelled census reports stopped rather than a clean zero");
    assert_eq!(r.slices, 0, "and it stops before the first slice, not after the last");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

/// A zero or negative window would loop forever. Clamp, do not trust the caller.
#[tokio::test]
async fn a_degenerate_window_is_clamped_rather_than_looping() {
    let (path, db, conn) = fixture("window").await;
    for id in [1, 2] {
        notice(&conn, id).await;
        claim(&conn, id, 1, id).await;
    }
    let r = db.ghost_census(0, 10, &|| false, &silent()).await.expect("census");
    assert_eq!(r.window, 1, "clamped to one id per slice");
    assert_eq!(r.slices, 2, "and it terminates");
    assert_eq!(r.ghost_notices, 0);
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}

/// An empty corpus has no upper bound to walk to. It must return zero rather
/// than walking to i64::MAX.
#[tokio::test]
async fn an_empty_corpus_walks_nothing() {
    let (path, db, _conn) = fixture("empty").await;
    let r = db.ghost_census(10, 10, &|| false, &silent()).await.expect("census");
    assert_eq!(r.max_notice_id, 0);
    assert_eq!(r.slices, 0);
    assert_eq!(r.ghost_notices, 0);
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
}
