//! Issue 239: `current_title` must carry the SAME title the old `v_tenders` subquery
//! computed, because that subquery is what made the view unusable — a primary-key point
//! read measured >10 s on prod — and replacing it is only safe if the answer is
//! unchanged.
//!
//! Two writers must agree: the fold's `head_title` (in memory, for new heads) and
//! `backfill_current_title` (in SQL, for the 7.9M existing rows). Both encode the same
//! precedence, so the cases here are the ones where precedence actually decides:
//! Tender-title over lot-title, ENG over another language, the head version over a
//! newer non-head one, and a title-less tender staying NULL.

use store::turso::{self, Value};

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-titlebf-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (db, conn)
}

async fn tender(conn: &turso::Connection, id: i64, current_seq: i64) {
    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
         VALUES (?, 'ted', ?, 'procedure', ?, 100, 0)",
        (Value::Integer(id), Value::Text(format!("pk-{id}")), Value::Integer(current_seq)),
    )
    .await
    .unwrap();
}

async fn title(
    conn: &turso::Connection,
    tender: i64,
    seq: i64,
    lot: Option<i64>,
    lang: Option<&str>,
    value: &str,
) {
    conn.execute(
        "INSERT INTO tender_version_texts (tender_id, seq, lot_id, field, lang, value)
         VALUES (?, ?, ?, 'title', ?, ?)",
        (
            Value::Integer(tender),
            Value::Integer(seq),
            lot.map(Value::Integer).unwrap_or(Value::Null),
            lang.map(|l| Value::Text(l.into())).unwrap_or(Value::Null),
            Value::Text(value.into()),
        ),
    )
    .await
    .unwrap();
}

async fn current_title(conn: &turso::Connection, id: i64) -> Option<String> {
    let mut rows = conn
        .query("SELECT current_title FROM tenders WHERE id = ?", (Value::Integer(id),))
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("the tender row");
    row.get_value(0).unwrap().as_text().cloned()
}

#[tokio::test]
async fn the_backfill_reproduces_the_old_subquery_precedence() {
    let (db, conn) = open("precedence").await;

    // 1: a Tender-level title and a lot-level one. The Tender's own must win — a lot
    // title is a STAND-IN for notices that never title the procedure, not a preference.
    tender(&conn, 1, 2).await;
    title(&conn, 1, 2, Some(10), None, "lot title").await;
    title(&conn, 1, 2, None, None, "tender title").await;

    // 2: lot-level only. This is why the fallback exists at all.
    tender(&conn, 2, 1).await;
    title(&conn, 2, 1, Some(20), None, "only a lot title").await;

    // 3: two Tender-level languages. ENG wins.
    tender(&conn, 3, 1).await;
    title(&conn, 3, 1, None, Some("DEU"), "deutscher Titel").await;
    title(&conn, 3, 1, None, Some("ENG"), "english title").await;

    // 4: the title lives on a NON-head version. The head has none, so the tender is
    // untitled — taking the newer one would silently resurrect superseded content.
    tender(&conn, 4, 1).await;
    title(&conn, 4, 2, None, Some("ENG"), "a later version's title").await;

    // 5: no title anywhere.
    tender(&conn, 5, 1).await;

    let (rows, watermark) = db.backfill_current_title(100, 0).await.unwrap();
    assert_eq!(rows, 5, "all five tenders walked");
    assert_eq!(watermark, 5, "the watermark is the last id stamped");

    assert_eq!(current_title(&conn, 1).await.as_deref(), Some("tender title"));
    assert_eq!(current_title(&conn, 2).await.as_deref(), Some("only a lot title"));
    assert_eq!(current_title(&conn, 3).await.as_deref(), Some("english title"));
    assert_eq!(current_title(&conn, 4).await, None, "a non-head title must not win");
    assert_eq!(current_title(&conn, 5).await, None, "no title stays NULL, not empty string");

    // The view now READS that column, so it must agree with it — this is the assertion
    // that the replacement is faithful, not merely fast.
    let mut rows = conn
        .query("SELECT id, title FROM v_tenders ORDER BY id", ())
        .await
        .unwrap();
    let mut seen = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        seen.push((
            row.get_value(0).unwrap().as_integer().copied().unwrap(),
            row.get_value(1).unwrap().as_text().cloned(),
        ));
    }
    drop(rows);
    // Only tenders whose head version exists in tender_versions appear; none were
    // inserted here, so the view is empty and that is the honest answer.
    assert!(seen.is_empty(), "no versions were written, so the view has no rows: {seen:?}");
}

#[tokio::test]
async fn the_walk_is_restartable_and_idempotent() {
    let (db, conn) = open("restart").await;
    for id in 1..=5i64 {
        tender(&conn, id, 1).await;
        title(&conn, id, 1, None, Some("ENG"), &format!("title {id}")).await;
    }

    // Batch of two, twice, then the remainder — a crashed run resumes from its
    // watermark rather than starting over.
    let (rows, w1) = db.backfill_current_title(2, 0).await.unwrap();
    assert_eq!((rows, w1), (2, 2));
    let (rows, w2) = db.backfill_current_title(2, w1).await.unwrap();
    assert_eq!((rows, w2), (2, 4));
    let (rows, w3) = db.backfill_current_title(2, w2).await.unwrap();
    assert_eq!((rows, w3), (1, 5));
    let (rows, _) = db.backfill_current_title(2, w3).await.unwrap();
    assert_eq!(rows, 0, "zero rows is how the caller learns the walk is done");

    for id in 1..=5i64 {
        assert_eq!(current_title(&conn, id).await.as_deref(), Some(&*format!("title {id}")));
    }

    // Re-running a completed range rewrites the same values.
    db.backfill_current_title(100, 0).await.unwrap();
    assert_eq!(current_title(&conn, 3).await.as_deref(), Some("title 3"));
}
