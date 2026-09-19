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
use store::{NoticeValue, Parsed, Stamp};

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

/// A stand-in with `ingest::project::notice_stamps`' exact contract: the first
/// publication field the parse carries, falling back to dispatch; `None` on
/// either axis when the parse states nothing; a date-only value anchored at its
/// civil day's UTC midnight (issue 418 — the stored value is local midnight).
fn resolve(parsed: &Parsed) -> (Option<Stamp>, Option<Stamp>) {
    let first = |field: &str| {
        parsed.values.iter().find(|v| v.field_id == field).and_then(|v| match &v.value {
            NoticeValue::Date { utc_seconds, offset_minutes, has_time } => Some(Stamp {
                utc_seconds: if *has_time { *utc_seconds } else { *utc_seconds + *offset_minutes * 60 },
                offset_minutes: *offset_minutes,
                has_time: *has_time,
            }),
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

/// A TIMED date at +01:00 — exact, so the resolver hands it back unchanged.
async fn date(conn: &turso::Connection, notice_id: i64, field: &str, utc: i64) {
    conn.execute(
        "INSERT INTO notice_dates (notice_id, section_id, field_id, ordinal,
                                   utc_seconds, offset_minutes, has_time)
         VALUES (?, 'PROCEDURE', ?, 0, ?, 60, 1)",
        (Value::Integer(notice_id), Value::Text(field.into()), Value::Integer(utc)),
    )
    .await
    .unwrap();
}

/// A DATE-ONLY value at +01:00, stored as the parse layer stores it: the local
/// midnight in UTC. The resolver anchors it one hour later, at the civil midnight.
async fn date_only(conn: &turso::Connection, notice_id: i64, field: &str, local_midnight_utc: i64) {
    conn.execute(
        "INSERT INTO notice_dates (notice_id, section_id, field_id, ordinal,
                                   utc_seconds, offset_minutes, has_time)
         VALUES (?, 'PROCEDURE', ?, 0, ?, 60, 0)",
        (Value::Integer(notice_id), Value::Text(field.into()), Value::Integer(local_midnight_utc)),
    )
    .await
    .unwrap();
}

/// The same row WITH the unit-3 pair the `date` helper's values carry (+01:00,
/// timed) — a row stamped since the pair's columns existed.
async fn stamped_notice(
    conn: &turso::Connection,
    id: i64,
    profile: &str,
    published: Option<i64>,
    dispatched: Option<i64>,
) {
    notice(conn, id, profile, "parsed", published, dispatched).await;
    conn.execute(
        "UPDATE notices SET published_offset = CASE WHEN published_at IS NULL THEN NULL ELSE 60 END,
                            published_has_time = CASE WHEN published_at IS NULL THEN NULL ELSE 1 END,
                            dispatched_offset = CASE WHEN dispatched_at IS NULL THEN NULL ELSE 60 END,
                            dispatched_has_time = CASE WHEN dispatched_at IS NULL THEN NULL ELSE 1 END
          WHERE id = ?",
        (Value::Integer(id),),
    )
    .await
    .unwrap();
}

fn never() -> bool {
    false
}

/// The six instant columns as stored: `(published, dispatched)`, each `Some`
/// only when its UTC value is; the pair inside is `None` where the row carries
/// no offset/precision yet.
async fn stored_pairs(
    conn: &turso::Connection,
    id: i64,
) -> ((Option<i64>, Option<(i64, bool)>), (Option<i64>, Option<(i64, bool)>)) {
    let mut rows = conn
        .query(
            "SELECT published_at, published_offset, published_has_time,
                    dispatched_at, dispatched_offset, dispatched_has_time
               FROM notices WHERE id = ?",
            (Value::Integer(id),),
        )
        .await
        .unwrap();
    let r = rows.next().await.unwrap().unwrap();
    let g = |i| match r.get_value(i).unwrap() {
        Value::Integer(v) => Some(v),
        Value::Null => None,
        other => panic!("{other:?}"),
    };
    let pair = |o: usize| Some((g(o)?, g(o + 1)? != 0));
    ((g(0), pair(1)), (g(3), pair(4)))
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
    let at = |utc| Some(Stamp { utc_seconds: utc, offset_minutes: 60, has_time: true });
    assert_eq!((f.to_published, f.to_dispatched), (at(1_704_841_200), at(1_704_841_285)));
    assert_eq!(p.applied, 0, "a dry run writes nothing");
    assert_eq!(stored(&conn, 1).await, (Some(0), None));

    let w = db.repair_notice_instants(&fields(), resolve, false, Some(1), &never).await.unwrap();
    assert_eq!((w.applied, w.skipped_moved), (1, 0));
    assert_eq!(stored(&conn, 1).await, (Some(1_704_841_200), Some(1_704_841_285)));
    // …and the pair rides along with the instants (issue 367 unit 3).
    assert_eq!(
        stored_pairs(&conn, 1).await,
        ((Some(1_704_841_200), Some((60, true))), (Some(1_704_841_285), Some((60, true))))
    );

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
    stamped_notice(&conn, 1, "eforms:eforms-sdk-1.13", Some(200), Some(100)).await;
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

/// Issue 367 unit 3: a row whose instants are right but which predates the
/// offset/precision pair — every parsed notice on the box the day the columns
/// arrived — is stamped as the walk goes, and never planned: nothing about it
/// needs a reviewer, since the instant does not move. The dry run counts it
/// apart from `agree`; the wet run writes the pair and the next walk agrees.
#[tokio::test]
async fn a_row_with_the_right_instants_and_no_pair_is_stamped_without_a_plan() {
    let (db, conn) = open("test-instants-unstamped").await;
    notice(&conn, 1, "eforms:eforms-de-2.0", "parsed", Some(200), Some(100)).await;
    date(&conn, 1, "BT-738-notice", 200).await;
    date(&conn, 1, "BT-05(a)-notice", 100).await;
    // A dispatch-less row: its dispatched pair stays NULL like its instant.
    notice(&conn, 2, "text", "parsed", Some(300), None).await;
    date(&conn, 2, "OPP-012-notice", 300).await;

    let p = plan(&db).await;
    assert_eq!((p.walked, p.agree, p.unstamped, p.rows), (2, 0, 2, 0));
    assert!(p.plan.is_empty(), "the pair is not a plan item");
    assert_eq!(p.stamped, 0, "a dry run writes nothing");
    assert_eq!(stored_pairs(&conn, 1).await, ((Some(200), None), (Some(100), None)));

    // The wet gate reads the reviewed plan's row count — zero here — and the
    // unstamped rows are written regardless of it.
    let w = db.repair_notice_instants(&fields(), resolve, false, Some(0), &never).await.unwrap();
    assert_eq!((w.stamped, w.applied, w.rows), (2, 0, 0));
    assert_eq!(
        stored_pairs(&conn, 1).await,
        ((Some(200), Some((60, true))), (Some(100), Some((60, true))))
    );
    assert_eq!(stored_pairs(&conn, 2).await, ((Some(300), Some((60, true))), (None, None)));

    let again = plan(&db).await;
    assert_eq!((again.agree, again.unstamped, again.rows), (2, 0, 0));
}

/// A row whose instant moved away from the resolver's is a PLANNED disagreement,
/// never an unstamped row: the pair is written only where the UTC values already
/// agree, so a re-parse that re-stamped the row keeps its own values until a
/// reviewed plan says otherwise — and then it is the plan's rewrite, with the
/// pair, that lands.
#[tokio::test]
async fn a_row_whose_instant_moved_is_planned_not_stamped() {
    let (db, conn) = open("test-instants-moved").await;
    notice(&conn, 1, "eforms:eforms-de-2.0", "parsed", Some(200), None).await;
    date(&conn, 1, "BT-738-notice", 200).await;
    assert_eq!(plan(&db).await.unstamped, 1);
    // The row now says 250 (a re-parse's own stamp): the resolver's 200 no
    // longer matches, so the next walk plans it rather than stamping it.
    conn.execute("UPDATE notices SET published_at = 250 WHERE id = 1", ()).await.unwrap();
    let p = plan(&db).await;
    assert_eq!((p.unstamped, p.rows), (0, 1));
    assert_eq!(stored_pairs(&conn, 1).await, ((Some(250), None), (None, None)), "a dry run writes nothing");
    let w = db.repair_notice_instants(&fields(), resolve, false, Some(1), &never).await.unwrap();
    assert_eq!((w.unstamped, w.stamped, w.rows, w.applied), (0, 0, 1, 1));
    assert_eq!(stored_pairs(&conn, 1).await, ((Some(200), Some((60, true))), (None, None)));
}

/// The write path stores the pair the processor resolved (issue 367 unit 3):
/// a Notice recorded with a date-only +01:00 instant reads back as exactly that.
#[tokio::test]
async fn a_recorded_notice_keeps_the_offset_and_precision_of_its_instants() {
    let (db, conn) = open("test-instants-record").await;
    db.record_notice(
        &store::Notice {
            source: "doe".into(),
            publication_id: "p-1".into(),
            content_hash: "h-1".into(),
            profile: "eforms:eforms-de-2.0".into(),
            declared_version: None,
            fetch_id: 1,
            member_path: "m".into(),
            ingested_at: 0,
            published_at: Some(Stamp { utc_seconds: 1_704_841_200, offset_minutes: 60, has_time: false }),
            dispatched_at: Some(Stamp { utc_seconds: 1_704_841_285, offset_minutes: 60, has_time: true }),
        },
        &store::Parse::Pending,
    )
    .await
    .unwrap();
    let mut rows = conn.query("SELECT id FROM notices WHERE publication_id = 'p-1'", ()).await.unwrap();
    let id = match rows.next().await.unwrap().unwrap().get_value(0).unwrap() {
        Value::Integer(id) => id,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        stored_pairs(&conn, id).await,
        ((Some(1_704_841_200), Some((60, false))), (Some(1_704_841_285), Some((60, true))))
    );
}

/// Issue 418: a date-only instant stored at the publisher's local midnight —
/// every eForms/DÖE row stamped before 2026-09-19 — moves to its civil day's UTC
/// midnight as a MECHANICAL class: counted as `shifted` on the dry run, written
/// as walked on the wet run, never planned, and the next walk agrees. Specimen
/// 26244735's values: 2024-01-10+01:00 stored as 1_704_841_200 (09 Jan 23:00Z),
/// anchored at 1_704_844_800 (10 Jan 00:00Z); its timed dispatch is untouched.
#[tokio::test]
async fn a_local_midnight_date_only_instant_is_shifted_to_its_civil_midnight_without_a_plan() {
    let (db, conn) = open("test-instants-shifted").await;
    notice(&conn, 1, "eforms:eforms-de-1.1", "parsed", Some(1_704_841_200), Some(1_704_841_285)).await;
    date_only(&conn, 1, "DE1-RequestedPublicationDate", 1_704_841_200).await;
    date(&conn, 1, "DE1-IssueDate", 1_704_841_285).await;

    let p = plan(&db).await;
    assert_eq!((p.walked, p.agree, p.unstamped, p.shifted, p.rows), (1, 0, 0, 1, 0));
    assert!(p.plan.is_empty(), "the shift is not a plan item");
    assert_eq!(stored(&conn, 1).await, (Some(1_704_841_200), Some(1_704_841_285)), "a dry run writes nothing");

    let w = db.repair_notice_instants(&fields(), resolve, false, Some(0), &never).await.unwrap();
    assert_eq!((w.shifted, w.stamped, w.rows), (1, 1, 0));
    assert_eq!(
        stored_pairs(&conn, 1).await,
        ((Some(1_704_844_800), Some((60, false))), (Some(1_704_841_285), Some((60, true))))
    );

    let again = plan(&db).await;
    assert_eq!((again.agree, again.shifted, again.unstamped, again.rows), (1, 0, 0, 0));
}
