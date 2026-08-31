//! Issue 323: moving the re-queue's predicates onto a plan turso can seek must
//! not move its SEMANTICS.
//!
//! The PLAN half of this lives in `lib.rs`'s unit tests
//! (`the_requeue_statements_seek_notices_by_rowid`), because it asserts against
//! the `pub(crate)` SQL builders themselves — a plan checked against a copied
//! literal is a plan checked against a copy, and a panel showed a reordered
//! spelling of the poison that a copy-based guard would have missed.
//!
//! What is left here is the behaviour: which rows re-queue, and what the count
//! means. The count is not decoration — `refold-notices` prints it beside the
//! ids it was asked for, and the difference is how a typo'd id is caught.

use store::turso;

async fn seed(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-requeue-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn.execute(
        "INSERT INTO fetches (id, source, kind, period, url, sha256, bytes, fetched_at, path)
         VALUES (1, 'ted', 'daily', 'p', 'u', 'aa', 1, 0, 'p')",
        (),
    )
    .await
    .unwrap();
    (db, conn)
}

async fn notice(conn: &turso::Connection, id: i64, state: &str, projected: i64) {
    conn.execute(
        "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                              member_path, ingested_at, parse_state, projected)
         VALUES (?, 'ted', 'pub-' || ?, 'h', 'eforms', 1, 'm', 0, ?, ?)",
        (
            turso::Value::Integer(id),
            turso::Value::Integer(id),
            turso::Value::Text(state.into()),
            turso::Value::Integer(projected),
        ),
    )
    .await
    .unwrap();
}

async fn projected_of(conn: &turso::Connection) -> Vec<(i64, i64)> {
    let mut rows = conn.query("SELECT id, projected FROM notices ORDER BY id", ()).await.unwrap();
    let mut got = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        let (turso::Value::Integer(id), turso::Value::Integer(p)) =
            (row.get_value(0).unwrap(), row.get_value(1).unwrap())
        else {
            panic!("ints")
        };
        got.push((id, p));
    }
    got
}

/// Only parsed, currently-projected rows re-queue, and the returned count is
/// those rows and nothing else.
#[tokio::test]
async fn only_parsed_and_still_projected_rows_are_requeued() {
    let (db, conn) = seed("semantics").await;
    // 1 re-queues; 2 is already queued; 3 and 4 were never folded and must not
    // be touched; 999 does not exist.
    for (id, state, projected) in
        [(1i64, "parsed", 1i64), (2, "parsed", 0), (3, "quarantined", 1), (4, "pending", 1)]
    {
        notice(&conn, id, state, projected).await;
    }

    assert_eq!(db.unmark_projected_by_ids(&[1, 2, 3, 4, 999]).await.unwrap(), 1);
    assert_eq!(
        projected_of(&conn).await,
        vec![(1, 0), (2, 0), (3, 1), (4, 1)],
        "the quarantined and pending rows keep their watermark untouched"
    );
    // Idempotent, which is what makes the count a real delta rather than a
    // cohort size: a second pass finds nothing left to do.
    assert_eq!(db.unmark_projected_by_ids(&[1, 2, 3, 4]).await.unwrap(), 0);
}

/// A duplicate id must not inflate the count — including one that straddles a
/// chunk boundary, where the reads all happen before any write and an
/// unguarded UPDATE would report the same row twice. The panel caught exactly
/// this in a two-statement draft of the helper; the guard and the entry dedup
/// are what close it, and this is the case that would notice their removal.
#[tokio::test]
async fn a_duplicate_id_is_counted_once_even_across_a_chunk_boundary() {
    let (db, conn) = seed("dupes").await;
    // REQUEUE_CHUNK is 500, so 501 ids put the repeat in the second chunk.
    for id in 1i64..=500 {
        notice(&conn, id, "parsed", 1).await;
    }
    let mut ids: Vec<i64> = (1..=500).collect();
    ids.push(1);
    assert_eq!(
        db.unmark_projected_by_ids(&ids).await.unwrap(),
        500,
        "501 ids, 500 distinct notices, 500 re-queued"
    );
    assert!(projected_of(&conn).await.iter().all(|(_, p)| *p == 0));

    // And the shape an operator actually produces: two overlapping lists
    // concatenated, still under the job's 1,000-id cap.
    let (db2, conn2) = seed("dupes2").await;
    for id in 1i64..=500 {
        notice(&conn2, id, "parsed", 1).await;
    }
    let both: Vec<i64> = (1..=500).chain(1..=500).collect();
    assert_eq!(db2.unmark_projected_by_ids(&both).await.unwrap(), 500);
}
