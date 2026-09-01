//! Issue 328: re-parse the rows whose identifier carries a publisher label.
//!
//! What these pin is the judgement around the write, not the stripping — the
//! vocabulary and the strip itself are unit-tested in `crates/ingest`. Here:
//! which rows qualify, that a value the strip cannot rescue is left exactly as
//! published, that the reunions are counted because they are the point, and that
//! the identifier VALUE moving does not lose the published string.

use store::turso::{self, Value};

/// The strip, as the job injects it (`ingest` depends on `store`, so importing
/// the real one here would be a cycle).
fn strip(value: &str) -> Option<&str> {
    for p in ["UMSATZSTEUERIDENTIFIKATIONSNUMMER", "USTIDNR", "USTID", "STNR"] {
        if let Some(rest) = value.strip_prefix(p) {
            return (!rest.is_empty()).then_some(rest);
        }
    }
    None
}

/// A stand-in classifier with the real one's shape: `DE` + 9 digits is a German
/// VAT id; anything else keeps the mention's country and stays national. A value
/// with no digits at all classifies as nothing, like the v2 gate's floor.
fn reclassify(value: &str, country: Option<&str>) -> Option<(String, Option<String>, String)> {
    let bare = strip(value).unwrap_or(value);
    if !bare.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    if let Some(rest) = bare.strip_prefix("DE") {
        if rest.len() == 9 && rest.chars().all(|c| c.is_ascii_digit()) {
            return Some(("vat".to_owned(), Some("DE".to_owned()), bare.to_owned()));
        }
    }
    Some(("national".to_owned(), country.map(str::to_owned), bare.to_owned()))
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

async fn org(conn: &turso::Connection, id: i64, cc: &str, kind: &str, ident: &str, name: &str) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
         VALUES (?, ?, ?, ?, ?, 0, 0)",
        (
            Value::Integer(id),
            Value::Text(cc.into()),
            Value::Text(kind.into()),
            Value::Text(ident.into()),
            Value::Text(name.into()),
        ),
    )
    .await
    .unwrap();
}

/// A mention of `org` that keeps the PUBLISHED string in `raw_identifier`.
async fn mention(conn: &turso::Connection, org_id: i64, raw: &str, notice: i64) {
    conn.execute(
        "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                              member_path, ingested_at, parse_state, projected)
         VALUES (?, 'ted', 'pub-' || ?, 'h' || ?, 'eforms', 1, 'm', 0, 'parsed', 1)",
        (Value::Integer(notice), Value::Integer(notice), Value::Integer(notice)),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
         VALUES (?, 'ORG-1', ?, 'x', 'DE', ?)",
        (Value::Integer(notice), Value::Integer(org_id), Value::Text(raw.into())),
    )
    .await
    .unwrap();
}

fn never() -> bool {
    false
}

async fn plan(db: &store::Db) -> store::LabelRepairReport {
    db.repair_label_prefixes(strip, reclassify, true, None, &never).await.unwrap()
}

async fn read(conn: &turso::Connection, id: i64) -> (String, String, String) {
    let mut rows = conn
        .query(
            "SELECT country, identifier_kind, identifier FROM organizations WHERE id = ?",
            (Value::Integer(id),),
        )
        .await
        .unwrap();
    let r = rows.next().await.unwrap().unwrap();
    let g = |i| match r.get_value(i).unwrap() {
        Value::Text(s) => s,
        Value::Null => String::new(),
        other => panic!("{other:?}"),
    };
    (g(0), g(1), g(2))
}

/// The core: the label comes off, all three published fields move together, and
/// the string the notice actually carried is still in the mention.
#[tokio::test]
async fn the_label_comes_off_and_the_published_string_survives_in_the_mention() {
    let (db, conn) = open("test-label-core").await;
    org(&conn, 1, "DE", "national", "USTIDDE329214156", "Die Autobahn GmbH").await;
    mention(&conn, 1, "USt-IdNr. DE329214156", 100).await;
    db.build_organization_indexes().await.unwrap();

    let p = plan(&db).await;
    assert_eq!(p.labelled, 1);
    assert_eq!(p.rows, 1);
    let f = &p.plan[0];
    assert_eq!(f.from_identifier, "USTIDDE329214156");
    assert_eq!(f.to_identifier, "DE329214156");
    assert_eq!((f.from_kind.as_str(), f.to_kind.as_str()), ("national", "vat"));
    assert_eq!(p.applied, 0, "a dry run writes nothing");

    let w = db.repair_label_prefixes(strip, reclassify, false, Some(1), &never).await.unwrap();
    assert_eq!(w.applied, 1);
    assert_eq!(read(&conn, 1).await, ("DE".into(), "vat".into(), "DE329214156".into()));

    // The publisher's own string is untouched — the canonical row carries the
    // cleaned value, the mention keeps what the notice said. Same division
    // issue 311's strips relied on.
    let mut rows = conn
        .query("SELECT raw_identifier FROM organization_mentions WHERE organization_id = 1", ())
        .await
        .unwrap();
    assert_eq!(
        rows.next().await.unwrap().unwrap().get_value(0).unwrap(),
        Value::Text("USt-IdNr. DE329214156".into())
    );
}

/// THE REUNIONS ARE THE POINT, so they are counted. 3,253 of the 5,766 rows have
/// a partner already standing under the bare value; the repair makes the merge
/// possible and R2 performs it.
#[tokio::test]
async fn a_row_landing_on_a_standing_identity_is_counted_as_a_reunion() {
    let (db, conn) = open("test-label-reunion").await;
    // The labelled row…
    org(&conn, 1, "DE", "national", "USTIDDE329214156", "Die Autobahn GmbH").await;
    // …and the correctly-formed twin it is split from.
    org(&conn, 2, "DE", "vat", "DE329214156", "Die Autobahn GmbH des Bundes").await;
    // A labelled row with no twin: repaired, but no reunion.
    org(&conn, 3, "DE", "national", "USTIDNRDE811335517", "Regierung von Oberbayern").await;
    db.build_organization_indexes().await.unwrap();

    let p = plan(&db).await;
    assert_eq!(p.rows, 2);
    assert_eq!(p.reunions, 1, "only org 1 lands on a standing identity");

    let w = db.repair_label_prefixes(strip, reclassify, false, Some(2), &never).await.unwrap();
    assert_eq!(w.applied, 2);
    // Both rows now hold the same identity — the duplicate R2 exists to fold.
    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM organizations WHERE country='DE' AND identifier_kind='vat' \
               AND identifier='DE329214156'",
            (),
        )
        .await
        .unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get_value(0).unwrap(), Value::Integer(2));
}

/// A value the strip cannot rescue is left EXACTLY as published. These are the
/// prod shapes: the bare field name, and a remainder that classifies as nothing.
#[tokio::test]
async fn a_value_the_strip_cannot_rescue_stands_as_published() {
    let (db, conn) = open("test-label-refused").await;
    // The field name alone — three prod rows carry this and no number.
    org(&conn, 1, "DE", "national", "UMSATZSTEUERIDENTIFIKATIONSNUMMER", "Kreisklinik").await;
    // A label followed by letters: nothing to recover.
    org(&conn, 2, "DE", "national", "USTIDABCDEF", "Irgendwas GmbH").await;
    db.build_organization_indexes().await.unwrap();

    let p = plan(&db).await;
    assert_eq!(p.labelled, 1, "the bare field name strips to nothing, so it is not even labelled");
    assert_eq!(p.now_refused, 1, "and USTIDABCDEF's remainder classifies as nothing");
    assert_eq!(p.rows, 0);

    let w = db.repair_label_prefixes(strip, reclassify, false, None, &never).await.unwrap();
    assert_eq!(w.applied, 0);
    assert_eq!(read(&conn, 1).await.2, "UMSATZSTEUERIDENTIFIKATIONSNUMMER");
    assert_eq!(read(&conn, 2).await.2, "USTIDABCDEF");
}

/// Idempotent by a DIFFERENT route than the issue-325 repair. That one relies on
/// its `kind = 'vat'` scope; a repaired row here simply no longer carries a
/// label, so the strip stops selecting it.
#[tokio::test]
async fn a_repaired_row_is_no_longer_selected() {
    let (db, conn) = open("test-label-idem").await;
    org(&conn, 1, "DE", "national", "USTIDDE329214156", "Die Autobahn GmbH").await;
    db.build_organization_indexes().await.unwrap();

    assert_eq!(
        db.repair_label_prefixes(strip, reclassify, false, Some(1), &never)
            .await
            .unwrap()
            .applied,
        1
    );
    let again = plan(&db).await;
    assert_eq!(again.labelled, 0, "the label is gone, so the row is out of scope");
    assert_eq!(again.rows, 0);
}

/// A row already carrying the clean value is not a change, even though it has no
/// label — it never enters the population at all.
#[tokio::test]
async fn an_unlabelled_row_is_never_walked_into_the_plan() {
    let (db, conn) = open("test-label-clean").await;
    org(&conn, 1, "DE", "vat", "DE329214156", "Die Autobahn GmbH").await;
    org(&conn, 2, "PL", "national", "5261040828", "Krajowa Izba").await;
    db.build_organization_indexes().await.unwrap();

    let p = plan(&db).await;
    assert_eq!(p.labelled, 0);
    assert_eq!(p.rows, 0);
    assert!(p.plan.is_empty());
}

/// The parity gate and an honest cancel, the same ladder every other write path
/// in this campaign follows.
#[tokio::test]
async fn a_drifted_plan_aborts_and_a_cancel_reports_nothing() {
    let (db, conn) = open("test-label-parity").await;
    for n in 0..12i64 {
        org(
            &conn,
            n + 1,
            "DE",
            "national",
            &format!("USTIDDE1{n:08}"),
            "Irgendein Betrieb",
        )
        .await;
    }
    db.build_organization_indexes().await.unwrap();

    let p = plan(&db).await;
    assert_eq!(p.rows, 12);

    let err = db
        .repair_label_prefixes(strip, reclassify, false, Some(3), &never)
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("ABORTED") && msg.contains("12"), "{msg}");
    assert_eq!(read(&conn, 1).await.1, "national", "nothing was written");

    let always = || true;
    let c = db.repair_label_prefixes(strip, reclassify, true, None, &always).await.unwrap();
    assert!(c.stopped);
    assert_eq!(c.rows, 0);
    assert!(c.plan.is_empty());
}
