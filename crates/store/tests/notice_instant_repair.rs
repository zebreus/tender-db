//! Issue 367: re-derive each parsed notice's publication/dispatch instants from
//! its own stored parse.
//!
//! What these pin is the judgement around the write, not the resolution — the
//! resolver and its two field lists are unit-tested in `crates/ingest`. Here:
//! which rows qualify, that the three measured populations are counted apart
//! (the epoch stamp, the NULL prefix, the dateless payload whose stored value is
//! REMOVED), that an unparsed notice is never touched, that a second run plans
//! nothing, and that a wet run whose corpus has moved aborts instead of writing.

use store::turso::{self, Value};
use store::{NoticeValue, Parsed};

/// The two date axes, as the job injects them (`ingest` depends on `store`, so
/// importing the real lists here would be a cycle). Same shape as the real
/// ones — best-first, both vocabularies named, each dialect id beside its
/// eForms target.
const PUBLICATION: &[&str] =
    &["OPP-012-notice", "DE1-Publication-PublicationDate", "BT-738-notice", "DE1-RequestedPublicationDate"];
const DISPATCH: &[&str] = &["BT-05(a)-notice", "DE1-IssueDate"];

fn fields() -> Vec<&'static str> {
    PUBLICATION.iter().chain(DISPATCH).copied().collect()
}

/// A stand-in with `ingest::project::notice_instants`' exact contract: the first
/// publication field the parse carries, falling back to dispatch; `None` on
/// either axis when the parse states nothing.
fn resolve(parsed: &Parsed) -> (Option<i64>, Option<i64>) {
    let first = |field: &str| {
        parsed.values.iter().find(|v| v.field_id == field).and_then(|v| match &v.value {
            NoticeValue::Date { utc_seconds, .. } => Some(*utc_seconds),
            _ => None,
        })
    };
    let dispatched = DISPATCH.iter().find_map(|f| first(f));
    (PUBLICATION.iter().find_map(|f| first(f)).or(dispatched), dispatched)
}

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

/// A notice row with the instants it was STAMPED with — which is the whole
/// point: the stamp is what disagrees with the parse below it.
async fn notice(
    conn: &turso::Connection,
    id: i64,
    profile: &str,
    state: &str,
    published: Option<i64>,
    dispatched: Option<i64>,
) {
    conn.execute(
        "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                              member_path, ingested_at, published_at, dispatched_at,
                              parse_state, projected)
         VALUES (?, 'doe', 'pub-' || ?, 'h' || ?, ?, 1, 'm', 0, ?, ?, ?, 1)",
        (
            Value::Integer(id),
            Value::Integer(id),
            Value::Integer(id),
            Value::Text(profile.into()),
            published.map_or(Value::Null, Value::Integer),
            dispatched.map_or(Value::Null, Value::Integer),
            Value::Text(state.into()),
        ),
    )
    .await
    .unwrap();
}

async fn date(conn: &turso::Connection, notice_id: i64, field: &str, utc: i64) {
    conn.execute(
        "INSERT INTO notice_dates (notice_id, section_id, field_id, ordinal,
                                   utc_seconds, offset_minutes, has_time)
         VALUES (?, 'PROCEDURE', ?, 0, ?, 60, 0)",
        (Value::Integer(notice_id), Value::Text(field.into()), Value::Integer(utc)),
    )
    .await
    .unwrap();
}

fn never() -> bool {
    false
}

async fn plan(db: &store::Db) -> store::NoticeInstantRepairReport {
    db.repair_notice_instants(&fields(), resolve, true, None, &never).await.unwrap()
}

async fn stored(conn: &turso::Connection, id: i64) -> (Option<i64>, Option<i64>) {
    let mut rows = conn
        .query("SELECT published_at, dispatched_at FROM notices WHERE id = ?", (Value::Integer(id),))
        .await
        .unwrap();
    let r = rows.next().await.unwrap().unwrap();
    let g = |i| match r.get_value(i).unwrap() {
        Value::Integer(v) => Some(v),
        Value::Null => None,
        other => panic!("{other:?}"),
    };
    (g(0), g(1))
}

/// The measured defect, end to end: an `eforms-de-1.1` notice whose parse states
/// 2024-01-10 +01:00 on both axes, stamped `published_at = 0` and
/// `dispatched_at = NULL` because the process-time resolver saw the raw `DE1-*`
/// ids and matched neither list. Specimen 26244735's own two values.
#[tokio::test]
async fn the_epoch_stamp_is_replaced_by_the_date_the_parse_states() {
    let (db, conn) = open("test-instants-epoch").await;
    notice(&conn, 1, "eforms:eforms-de-1.1", "parsed", Some(0), None).await;
    date(&conn, 1, "DE1-RequestedPublicationDate", 1_704_841_200).await;
    date(&conn, 1, "DE1-IssueDate", 1_704_841_285).await;

    let p = plan(&db).await;
    assert_eq!(p.walked, 1);
    assert_eq!(p.rows, 1);
    assert_eq!(p.epoch_published, 1);
    assert_eq!(p.null_published, 0);
    assert_eq!(p.resolver_silent, 0);
    assert_eq!(p.by_profile, vec![("eforms:eforms-de-1.1".to_owned(), 1)]);
    let f = &p.plan[0];
    assert_eq!((f.from_published, f.from_dispatched), (Some(0), None));
    assert_eq!((f.to_published, f.to_dispatched), (Some(1_704_841_200), Some(1_704_841_285)));
    assert_eq!(p.applied, 0, "a dry run writes nothing");
    assert_eq!(stored(&conn, 1).await, (Some(0), None));

    let w = db.repair_notice_instants(&fields(), resolve, false, Some(1), &never).await.unwrap();
    assert_eq!((w.applied, w.skipped_moved), (1, 0));
    assert_eq!(stored(&conn, 1).await, (Some(1_704_841_200), Some(1_704_841_285)));

    // Idempotent: the repaired row now agrees with the resolver.
    let again = plan(&db).await;
    assert_eq!((again.walked, again.rows, again.agree), (1, 0, 1));
}

/// The second population: a TED notice ingested before issue 18 shipped the
/// resolver at all, so both columns arrived NULL and no re-parse or reclaim ever
/// revisits a cleanly-parsed notice. Its parse states the dates all along.
#[tokio::test]
async fn a_null_prefix_notice_gains_the_instants_its_parse_carries() {
    let (db, conn) = open("test-instants-null").await;
    notice(&conn, 1, "eforms:eforms-sdk-1.13", "parsed", None, None).await;
    date(&conn, 1, "OPP-012-notice", 1_784_239_200).await;
    date(&conn, 1, "BT-05(a)-notice", 1_784_100_000).await;

    let p = plan(&db).await;
    assert_eq!((p.rows, p.null_published, p.epoch_published), (1, 1, 0));

    db.repair_notice_instants(&fields(), resolve, false, Some(1), &never).await.unwrap();
    assert_eq!(stored(&conn, 1).await, (Some(1_784_239_200), Some(1_784_100_000)));
}

/// The class that REMOVES a value, counted on its own line because it is the
/// only one: a payload that states no date at all was stamped with the epoch,
/// and the honest answer is NULL. 1970-01-01 was never a publication date.
#[tokio::test]
async fn a_dateless_payload_loses_its_invented_epoch() {
    let (db, conn) = open("test-instants-silent").await;
    notice(&conn, 1, "text", "parsed", Some(0), None).await;

    let p = plan(&db).await;
    assert_eq!((p.rows, p.resolver_silent, p.epoch_published), (1, 1, 0));
    assert_eq!((p.plan[0].to_published, p.plan[0].to_dispatched), (None, None));

    db.repair_notice_instants(&fields(), resolve, false, Some(1), &never).await.unwrap();
    assert_eq!(stored(&conn, 1).await, (None, None));
}

/// A row that already says what the parse says is counted and left alone, and an
/// UNPARSED notice is never walked — its NULL instants are the schema's
/// documented "null until the payload is parsed", not a defect to repair.
#[tokio::test]
async fn correct_rows_agree_and_unparsed_rows_are_not_walked() {
    let (db, conn) = open("test-instants-agree").await;
    notice(&conn, 1, "eforms:eforms-sdk-1.13", "parsed", Some(200), Some(100)).await;
    date(&conn, 1, "OPP-012-notice", 200).await;
    date(&conn, 1, "BT-05(a)-notice", 100).await;
    // Identity-only: a profile with no parser yet. It carries no parsed layer,
    // so a walk that included it would "repair" its NULLs into NULLs at best
    // and plan a rewrite of a row nobody has read yet at worst.
    notice(&conn, 2, "eforms:eforms-sdk-9.9", "pending", None, None).await;
    // Quarantined at parse: same reasoning.
    notice(&conn, 3, "eforms:eforms-sdk-1.13", "quarantined", None, None).await;

    let p = plan(&db).await;
    assert_eq!(p.walked, 1, "only the parsed notice is walked");
    assert_eq!((p.agree, p.rows), (1, 0));
    assert!(p.by_profile.is_empty());
}

/// The wet arm's gate: the reviewed plan's row count is the contract, and a
/// corpus that has moved past it aborts rather than writing a plan nobody read.
#[tokio::test]
async fn a_wet_run_aborts_when_the_corpus_no_longer_matches_the_reviewed_plan() {
    let (db, conn) = open("test-instants-abort").await;
    for id in 1..=3 {
        notice(&conn, id, "eforms:eforms-de-1.2", "parsed", Some(0), None).await;
        date(&conn, id, "DE1-RequestedPublicationDate", 1_700_000_000 + id).await;
    }

    assert_eq!(plan(&db).await.rows, 3);
    let err = db
        .repair_notice_instants(&fields(), resolve, false, Some(40), &never)
        .await
        .expect_err("a plan of 40 rows against a corpus of 3 must abort");
    assert!(format!("{err}").contains("ABORTED"), "{err}");
    assert_eq!(stored(&conn, 1).await, (Some(0), None), "the abort wrote nothing");

    // Within the max(2%, 5) tolerance the same run proceeds.
    let w = db.repair_notice_instants(&fields(), resolve, false, Some(5), &never).await.unwrap();
    assert_eq!(w.applied, 3);
}

/// A cancel between read windows stores nothing — a partial plan would read as a
/// smaller defect than the corpus actually holds.
#[tokio::test]
async fn a_stopped_dry_run_plans_nothing() {
    let (db, conn) = open("test-instants-stop").await;
    notice(&conn, 1, "eforms:eforms-de-1.1", "parsed", Some(0), None).await;
    date(&conn, 1, "DE1-IssueDate", 1_704_841_285).await;

    let always = || true;
    let p = db.repair_notice_instants(&fields(), resolve, true, None, &always).await.unwrap();
    assert!(p.stopped);
    assert_eq!((p.walked, p.rows), (0, 0));
    assert!(p.plan.is_empty());
    assert_eq!(stored(&conn, 1).await, (Some(0), None));
}
