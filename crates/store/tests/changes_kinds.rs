//! Issue 211, the reopened half: the public poll/webhook feeds read `limit` events of
//! the PUBLIC kinds, not `limit` raw rows filtered afterwards. Pinned on a hand-built
//! change log with the hidden kinds interleaved between the public ones.

use store::read::{changes_since, changes_since_kinds};
use store::turso::{self};

const PUBLIC: [&str; 3] = ["tender", "lot", "organization"];

#[tokio::test]
async fn the_public_kinds_fill_a_page_across_interleaved_hidden_rows() {
    let path = format!("/tmp/tender-db-changes-kinds-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let _db = store::Db::open(&path).await.expect("open");
    let raw = turso::Builder::new_local(&path).build().await.expect("raw");
    let conn = raw.connect().expect("connect");
    // cursor: kind — the hidden kinds sit first and between every public row.
    for (cursor, kind) in [
        (1, "lot_result"),
        (2, "tender"),
        (3, "bid"),
        (4, "lot"),
        (5, "contract"),
        (6, "organization"),
        (7, "tender"),
        (8, "lot_result"),
    ] {
        conn.execute(
            &format!(
                "INSERT INTO changes (cursor, entity_kind, entity_id, version_seq, op, changed_at)
                 VALUES ({cursor}, '{kind}', {cursor}, NULL, 'added', 100)"
            ),
            (),
        )
        .await
        .expect("insert");
    }
    let cursors = |v: &[store::Change]| v.iter().map(|c| c.cursor).collect::<Vec<_>>();

    // A page of three public events from the start: exactly the three public rows,
    // in cursor order, whatever sat between them.
    let page = changes_since_kinds(&conn, 0, 3, &PUBLIC).await.expect("page");
    assert_eq!(cursors(&page), vec![2, 4, 6]);
    assert!(page.iter().all(|c| PUBLIC.contains(&c.entity_kind.as_str())));
    // Continuing from the page's last cursor picks up the rest, and no hidden row.
    let rest = changes_since_kinds(&conn, 6, 10, &PUBLIC).await.expect("rest");
    assert_eq!(cursors(&rest), vec![7]);
    // One kind is the same seek `changes_since` makes with an `entity` filter.
    assert_eq!(cursors(&changes_since_kinds(&conn, 0, 10, &["tender"]).await.unwrap()), vec![2, 7]);
    // A kind the log never emits contributes nothing and costs no scan.
    assert_eq!(cursors(&changes_since_kinds(&conn, 0, 10, &["nope"]).await.unwrap()), Vec::<i64>::new());
    // The raw read is unchanged: every row, every kind — the projection's own view.
    assert_eq!(cursors(&changes_since(&conn, 0, 100, None).await.unwrap()), (1..=8).collect::<Vec<_>>());

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
