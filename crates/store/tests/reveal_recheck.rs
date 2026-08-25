//! Issue 173 (D5): `reveal_recheck` measures whether BT-198 "publish later"
//! promises are kept. Fixture: one tender, three withheld fields —
//!   * `cri-num` on notice 1, due, and notice 2 (a later version) no longer
//!     withholds it → revealed at head;
//!   * `win-nam` on notice 1, due, but notice 2 STILL withholds it → reveal debt;
//!   * `oth-fld` on notice 2 with a reveal date in the future → dated, not due.

async fn raw(path: &str) -> (store::Db, turso::Connection) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.expect("open");
    let raw = turso::Builder::new_local(path).build().await.expect("raw open");
    let conn = raw.connect().expect("connect");
    (db, conn)
}

async fn exec(conn: &turso::Connection, sql: &str) {
    conn.execute(sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
}

/// A FieldsPrivacy section on `notice`, withholding `field` (NULL skips the
/// BT-195 code), revealing after `due` (NULL skips the BT-198 date).
async fn withhold(conn: &turso::Connection, notice: i64, section: &str, field: Option<&str>, due: Option<i64>) {
    exec(
        conn,
        &format!(
            "INSERT INTO notice_sections (notice_id, section_id, parent_section_id, kind)
             VALUES ({notice}, '{section}', 'root', 'FieldsPrivacy')"
        ),
    )
    .await;
    if let Some(f) = field {
        exec(
            conn,
            &format!(
                "INSERT INTO notice_codes (notice_id, section_id, field_id, ordinal, code)
                 VALUES ({notice}, '{section}', 'BT-195(BT-09)-Procedure', 0, '{f}')"
            ),
        )
        .await;
    }
    if let Some(d) = due {
        exec(
            conn,
            &format!(
                "INSERT INTO notice_dates (notice_id, section_id, field_id, ordinal, utc_seconds, offset_minutes, has_time)
                 VALUES ({notice}, '{section}', 'BT-198(BT-09)-Procedure', 0, {d}, 0, 0)"
            ),
        )
        .await;
    }
}

#[tokio::test]
async fn reveal_recheck_separates_kept_promises_from_debt() {
    let path = format!("/tmp/tender-db-reveal-{}.db", std::process::id());
    let (db, conn) = raw(&path).await;
    let now = 1_000_000;

    exec(&conn, "INSERT INTO fetches (id, source, kind, period, url, sha256, bytes, fetched_at, path) VALUES (1, 'ted', 'daily', 'p', 'u', 'aa', 1, 0, 'p')").await;
    for n in [1, 2] {
        exec(&conn, &format!(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at)
             VALUES ({n}, 'ted', 'pub-{n}', 'h{n}', 'eforms', 1, 'm{n}', 0)")).await;
    }
    exec(&conn, "INSERT INTO tenders (id, source, kind, created_at) VALUES (7, 'ted', 'procedure', 0)").await;
    for (seq, n) in [(1, 1), (2, 2)] {
        exec(&conn, &format!(
            "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, published_at, publication_id)
             VALUES (7, {seq}, {n}, {}, 'pub-{n}')", 100 + seq)).await;
    }

    withhold(&conn, 1, "s1", Some("cri-num"), Some(now - 50)).await; // due, revealed by notice 2
    withhold(&conn, 1, "s2", Some("win-nam"), Some(now - 50)).await; // due, still withheld below
    withhold(&conn, 2, "s1", Some("win-nam"), None).await; // notice 2 keeps withholding win-nam
    withhold(&conn, 2, "s2", Some("oth-fld"), Some(now + 500)).await; // dated, not yet due

    let sl = db.reveal_recheck(now, 0, 20_000).await.expect("recheck");
    assert_eq!(sl.withheld_total, 4, "every FieldsPrivacy section counts");
    assert_eq!(sl.sections, 4, "one big slice holds the whole cohort");
    assert_eq!(sl.dated, 3, "three carry a BT-198 date");
    assert_eq!(sl.due, 2, "two dates have passed");
    assert_eq!(sl.checked, 2);
    assert_eq!(sl.revealed, 1, "cri-num vanished from the later version");
    assert_eq!(sl.checked - sl.revealed, 1, "win-nam is the standing reveal debt");
    assert_eq!(sl.by_field.len(), 2, "the due breakdown names both fields");
    assert!(sl.by_field.iter().any(|(f, n)| f == "cri-num" && *n == 1));
    assert!(sl.by_field.iter().any(|(f, n)| f == "win-nam" && *n == 1));
    assert!(sl.wrapped, "an exhaustive slice reports the wrap");
    assert_eq!(sl.upto, 2, "the cursor stands on the last notice");
    // The failure split (campaign acceptance metric): win-nam's tender HAS a
    // later version that still withholds — the broken-promise bucket, not the
    // awaitable one.
    assert_eq!(sl.no_later, 0, "both due rows sit on a tender with a later version");
    assert_eq!(sl.checked - sl.revealed - sl.no_later, 1, "win-nam is BROKEN, not awaitable");
}

/// Issue 274: the walk is sliced. `slice = 1` picks a one-row boundary but the
/// range still processes the boundary NOTICE whole (both of notice 1's
/// sections), so the cursor can stand on it; the next run continues at notice 2
/// and wraps; a run starting past the cohort's end is empty and wrapped.
#[tokio::test]
async fn reveal_recheck_walks_the_cohort_in_notice_slices() {
    let path = format!("/tmp/tender-db-reveal-slices-{}.db", std::process::id());
    let (db, conn) = raw(&path).await;
    let now = 1_000_000;

    exec(&conn, "INSERT INTO fetches (id, source, kind, period, url, sha256, bytes, fetched_at, path) VALUES (1, 'ted', 'daily', 'p', 'u', 'aa', 1, 0, 'p')").await;
    for n in [1, 2] {
        exec(&conn, &format!(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at)
             VALUES ({n}, 'ted', 'pub-{n}', 'h{n}', 'eforms', 1, 'm{n}', 0)")).await;
    }
    withhold(&conn, 1, "s1", Some("cri-num"), Some(now - 50)).await;
    withhold(&conn, 1, "s2", Some("win-nam"), Some(now - 50)).await;
    withhold(&conn, 2, "s1", Some("oth-fld"), Some(now - 50)).await;

    let first = db.reveal_recheck(now, 0, 1).await.expect("first slice");
    assert_eq!(first.upto, 1, "boundary notice");
    assert_eq!(first.sections, 2, "the boundary notice is processed whole");
    assert_eq!(first.due, 2);
    assert!(!first.wrapped, "notice 2 is still ahead");
    assert_eq!(first.withheld_total, 3, "the cohort total is not slice-scoped");

    // Notice 1's tender has no versions at all in this fixture, so its two due
    // rows land in the awaitable bucket, not the broken one.
    assert_eq!(first.no_later, 2, "no later version exists: awaitable, not broken");
    assert_eq!(first.checked - first.revealed - first.no_later, 0);

    let second = db.reveal_recheck(now, first.upto, 1).await.expect("second slice");
    assert_eq!(second.upto, 2);
    assert_eq!(second.sections, 1);
    assert_eq!(second.due, 1);
    assert!(second.by_field.iter().any(|(f, n)| f == "oth-fld" && *n == 1));

    let third = db.reveal_recheck(now, second.upto, 1).await.expect("past the end");
    assert!(third.wrapped, "past the cohort's end the slice is empty and wrapped");
    assert_eq!(third.sections, 0);
    assert_eq!(third.upto, second.upto, "an empty slice does not move the cursor");
}
