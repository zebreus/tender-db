//! Issue 418 unit 2b: every `tender_versions` row follows its notice's
//! publication/dispatch instants, and the touched tenders' head column follows.
//!
//! What these pin: only a notice that carries the pair is followed (the notice
//! repair must have run first, and the job says so rather than reporting
//! agreement); a version that already says what its notice says is counted and
//! left alone; a dry run writes nothing; a wet run writes the two columns and
//! re-derives `current_published_at`; the wet gate on the reviewed count; and
//! that a second run finds nothing to do.

use store::turso::{self, Value};

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-{name}-{}.db", std::process::id());
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

/// A notice row: `pair` = `Some((offset, has_time))` when it has been through
/// the notice repair (or was stamped since issue 367 unit 3), `None` otherwise.
async fn notice(
    conn: &turso::Connection,
    id: i64,
    published: Option<i64>,
    dispatched: Option<i64>,
    pair: Option<(i64, bool)>,
) {
    conn.execute(
        "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                              member_path, ingested_at, published_at, dispatched_at,
                              published_offset, published_has_time, parse_state, projected)
         VALUES (?, 'doe', 'pub-' || ?, 'h' || ?, 'eforms:eforms-de-2.0', 1, 'm', 0, ?, ?, ?, ?, 'parsed', 1)",
        (
            Value::Integer(id),
            Value::Integer(id),
            Value::Integer(id),
            published.map_or(Value::Null, Value::Integer),
            dispatched.map_or(Value::Null, Value::Integer),
            pair.map_or(Value::Null, |(o, _)| Value::Integer(o)),
            pair.map_or(Value::Null, |(_, t)| Value::Integer(i64::from(t))),
        ),
    )
    .await
    .unwrap();
}

/// A tender with one version caused by `notice_id`, the head column set as the
/// fold set it.
async fn tender(conn: &turso::Connection, id: i64, notice_id: i64, published: i64, dispatched: Option<i64>) {
    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
         VALUES (?, 'doe', 'pk-' || ?, 'procedure', 1, ?, 0)",
        (Value::Integer(id), Value::Integer(id), Value::Integer(published)),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, published_at, dispatched_at, publication_id, caused_by_notice_id)
         VALUES (?, 1, ?, ?, 'pub-' || ?, ?)",
        (
            Value::Integer(id),
            Value::Integer(published),
            dispatched.map_or(Value::Null, Value::Integer),
            Value::Integer(notice_id),
            Value::Integer(notice_id),
        ),
    )
    .await
    .unwrap();
}

fn never() -> bool {
    false
}

async fn version(conn: &turso::Connection, tender_id: i64) -> (i64, Option<i64>, i64) {
    let mut rows = conn
        .query(
            "SELECT v.published_at, v.dispatched_at, t.current_published_at
               FROM tender_versions v JOIN tenders t ON t.id = v.tender_id
              WHERE v.tender_id = ? AND v.seq = 1",
            (Value::Integer(tender_id),),
        )
        .await
        .unwrap();
    let r = rows.next().await.unwrap().unwrap();
    let g = |i| match r.get_value(i).unwrap() {
        Value::Integer(v) => Some(v),
        Value::Null => None,
        other => panic!("{other:?}"),
    };
    (g(0).unwrap(), g(1), g(2).unwrap())
}

const LOCAL_MIDNIGHT: i64 = 1_704_841_200; // 2024-01-10+01:00 as the parse layer stores it
const CIVIL_MIDNIGHT: i64 = 1_704_844_800; // 2024-01-10T00:00Z, where the notice repair anchors it
const DISPATCHED: i64 = 1_704_841_285;

/// The campaign's shape: the notice repair has moved notice 1's instant and
/// stamped its pair; its version still carries the local midnight. Notice 2 has
/// not been repaired (no pair) — its version is left alone and the count says so.
/// Notice 3's version already agrees.
#[tokio::test]
async fn a_version_follows_its_repaired_notice_and_the_head_column_follows_the_version() {
    let (db, conn) = open("test-vinst-follow").await;
    notice(&conn, 1, Some(CIVIL_MIDNIGHT), Some(DISPATCHED), Some((60, false))).await;
    tender(&conn, 1, 1, LOCAL_MIDNIGHT, Some(DISPATCHED)).await;
    notice(&conn, 2, Some(LOCAL_MIDNIGHT), None, None).await;
    tender(&conn, 2, 2, LOCAL_MIDNIGHT, None).await;
    notice(&conn, 3, Some(CIVIL_MIDNIGHT), None, Some((60, false))).await;
    tender(&conn, 3, 3, CIVIL_MIDNIGHT, None).await;

    let dry = db.repair_version_instants(true, None, &never).await.unwrap();
    assert_eq!((dry.walked, dry.notice_unstamped, dry.agree, dry.moved), (3, 1, 1, 1));
    assert_eq!((dry.applied, dry.heads_recomputed), (0, 0), "a dry run writes nothing");
    assert_eq!(version(&conn, 1).await, (LOCAL_MIDNIGHT, Some(DISPATCHED), LOCAL_MIDNIGHT));

    let wet = db.repair_version_instants(false, Some(1), &never).await.unwrap();
    assert_eq!((wet.moved, wet.applied, wet.skipped_moved, wet.heads_recomputed), (1, 1, 0, 1));
    assert_eq!(
        version(&conn, 1).await,
        (CIVIL_MIDNIGHT, Some(DISPATCHED), CIVIL_MIDNIGHT),
        "the version and the head column both say what the notice says"
    );
    assert_eq!(version(&conn, 2).await, (LOCAL_MIDNIGHT, None, LOCAL_MIDNIGHT), "an unrepaired notice is not followed");
    assert_eq!(version(&conn, 3).await, (CIVIL_MIDNIGHT, None, CIVIL_MIDNIGHT), "untouched");

    let again = db.repair_version_instants(true, None, &never).await.unwrap();
    assert_eq!((again.agree, again.moved, again.notice_unstamped), (2, 0, 1));
}

/// The wet gate: the reviewed count is the contract, and a corpus that has moved
/// past it aborts rather than writing.
#[tokio::test]
async fn a_wet_run_aborts_when_the_count_no_longer_matches_the_reviewed_one() {
    let (db, conn) = open("test-vinst-abort").await;
    for id in 1..=3 {
        notice(&conn, id, Some(CIVIL_MIDNIGHT), None, Some((60, false))).await;
        tender(&conn, id, id, LOCAL_MIDNIGHT, None).await;
    }
    assert_eq!(db.repair_version_instants(true, None, &never).await.unwrap().moved, 3);
    let err = db
        .repair_version_instants(false, Some(40), &never)
        .await
        .expect_err("a reviewed count of 40 against 3 must abort");
    assert!(format!("{err}").contains("ABORTED"), "{err}");
    assert_eq!(version(&conn, 1).await.0, LOCAL_MIDNIGHT, "the abort wrote nothing");
    let w = db.repair_version_instants(false, Some(5), &never).await.unwrap();
    assert_eq!((w.applied, w.heads_recomputed), (3, 3));
}

/// A cancel between bands counts what it walked and stores nothing partial as
/// "done": the report says stopped.
#[tokio::test]
async fn a_stopped_run_says_so() {
    let (db, conn) = open("test-vinst-stop").await;
    notice(&conn, 1, Some(CIVIL_MIDNIGHT), None, Some((60, false))).await;
    tender(&conn, 1, 1, LOCAL_MIDNIGHT, None).await;
    let always = || true;
    let r = db.repair_version_instants(true, None, &always).await.unwrap();
    assert!(r.stopped);
    assert_eq!((r.walked, r.moved), (0, 0));
}
