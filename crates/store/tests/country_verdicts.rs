//! Issue 355: the execution path for a reviewed `wrong-country` verdict.
//! The issue-314 campaign recorded "the SK row is the contaminated one" in
//! free text nothing could execute; this is the structured verdict and the
//! move, and — as with every apply path here — the guards matter more than
//! the happy path: it rewrites a published column.

use store::turso::Value;
use store::CountryVerdict;

async fn count(conn: &store::turso::Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let Value::Integer(n) = row.get_value(0).unwrap() else { panic!("count") };
    n
}

async fn text1(conn: &store::turso::Connection, sql: &str) -> Option<String> {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    match row.get_value(0).unwrap() {
        Value::Text(s) => Some(s),
        _ => None,
    }
}

fn verdict(org: i64, action: &str, from: Option<&str>, to: Option<&str>, conf: &str) -> CountryVerdict {
    CountryVerdict {
        org_id: org,
        action: action.into(),
        from_country: from.map(str::to_owned),
        to_country: to.map(str::to_owned),
        rationale: "the identifier validates under the other register".into(),
        confidence: conf.into(),
    }
}

async fn open(name: &str) -> (store::Db, store::turso::Connection) {
    let path = format!("test-country-verdicts-{name}.db");
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = store::turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (db, conn)
}

async fn org(conn: &store::turso::Connection, id: i64, cc: Option<&str>, kind: Option<&str>, ident: Option<&str>, name: &str) {
    let v = |o: Option<&str>| match o {
        Some(s) => Value::Text(s.into()),
        None => Value::Null,
    };
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (?, ?, ?, ?, ?, ?, 0, 0)",
        (Value::Integer(id), v(cc), v(kind), v(ident), Value::Text(name.into()), Value::Text(name.to_lowercase())),
    )
    .await
    .unwrap();
}

/// The fixture every test starts from: eight verdicts of which exactly three
/// are executable moves.
///
/// * 10 `SG` national 12345678 → `SK`, high: MOVES, and lands on org 11's
///   triple — the duplicate identity the R2 arm folds afterwards.
/// * 20 `NL` vat A82473349 → `ES`, high: MOVES (a Spanish CIF on a Dutch row).
/// * 80 country-less → `DE`, high: MOVES (a from of NULL is a real pre-image).
/// * 30 `DE` → `AT`, MEDIUM: parked, never eligible.
/// * 40 `FR` keep: a verdict, not a move.
/// * 50 `IT` → `CH`, high, but the row now stands under `DE`: somebody moved
///   it since the review — a stamped no-op.
/// * 60 `PT` → `ES`, high, row deleted: a stamped no-op.
/// * 90 `BE` → `NL`, high, but the row is ALREADY under `NL`: a no-op.
async fn seed(name: &str) -> (store::Db, store::turso::Connection) {
    let (db, conn) = open(name).await;
    org(&conn, 10, Some("SG"), Some("national"), Some("12345678"), "Slovenská firma s.r.o.").await;
    org(&conn, 11, Some("SK"), Some("national"), Some("12345678"), "Slovenská firma s.r.o.").await;
    org(&conn, 20, Some("NL"), Some("vat"), Some("A82473349"), "Empresa Española SA").await;
    org(&conn, 30, Some("DE"), Some("vat"), Some("DE123456789"), "Firma GmbH").await;
    org(&conn, 40, Some("FR"), Some("vat"), Some("FR12345678901"), "Société SA").await;
    org(&conn, 50, Some("DE"), None, None, "Moved Since Review").await;
    org(&conn, 80, None, None, None, "Stadt Ohne Land").await;
    org(&conn, 90, Some("NL"), Some("vat"), Some("NL000000001B01"), "Already There BV").await;
    for notice in [1i64, 2] {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
             VALUES (?, 'ORG-1', 10, 'Slovenská firma', 'SG', '12345678')",
            (Value::Integer(notice),),
        )
        .await
        .unwrap();
    }
    let verdicts = [
        verdict(10, "move", Some("SG"), Some("SK"), "high"),
        verdict(20, "move", Some("NL"), Some("ES"), "high"),
        verdict(30, "move", Some("DE"), Some("AT"), "medium"),
        verdict(40, "keep", Some("FR"), None, "high"),
        verdict(50, "move", Some("IT"), Some("CH"), "high"),
        verdict(60, "move", Some("PT"), Some("ES"), "high"),
        verdict(80, "move", None, Some("DE"), "high"),
        verdict(90, "move", Some("BE"), Some("NL"), "high"),
    ];
    assert_eq!(db.record_country_verdicts("xb-test", &verdicts, 1_000).await.unwrap(), 8);
    (db, conn)
}

#[tokio::test]
async fn the_dry_plan_lists_exactly_the_executable_moves_and_writes_nothing() {
    let (db, conn) = seed("dry").await;
    let r = db.apply_country_verdicts(true, None, None, 2_000).await.unwrap();
    assert_eq!((r.pending, r.eligible, r.moved, r.noop, r.collisions), (8, 6, 3, 3, 1));
    let plan: Vec<(i64, Option<&str>, &str, Option<i64>)> =
        r.plan.iter().map(|m| (m.org, m.from.as_deref(), m.to.as_str(), m.collides_with)).collect();
    assert_eq!(
        plan,
        vec![(10, Some("SG"), "SK", Some(11)), (20, Some("NL"), "ES", None), (80, None, "DE", None)],
        "the concrete move list, with the duplicate identity org 10 lands on named"
    );
    assert_eq!(r.plan[0].mentions, 2, "the reviewer's first read travels with the plan");
    assert_eq!(r.plan[0].identifier.as_deref(), Some("12345678"));
    // Dry: nothing moved, nothing stamped.
    assert_eq!(text1(&conn, "SELECT country FROM organizations WHERE id = 10").await.as_deref(), Some("SG"));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_country_verdicts WHERE applied_at IS NOT NULL").await, 0);
}

#[tokio::test]
async fn a_wet_run_executes_the_reviewed_plan_and_stamps_the_pre_images() {
    let (db, conn) = seed("wet").await;
    let plan = db.apply_country_verdicts(true, None, None, 2_000).await.unwrap().plan;
    let expect: Vec<(i64, Option<String>, String)> =
        plan.iter().map(|m| (m.org, m.from.clone(), m.to.clone())).collect();
    let r = db.apply_country_verdicts(false, Some(&expect), Some(77), 3_000).await.unwrap();
    assert_eq!((r.moved, r.noop, r.collisions), (3, 3, 1));
    assert!(r.plan.is_empty(), "the plan is a dry-run artefact");

    for (id, cc) in [(10i64, "SK"), (20, "ES"), (80, "DE")] {
        assert_eq!(
            text1(&conn, &format!("SELECT country FROM organizations WHERE id = {id}")).await.as_deref(),
            Some(cc)
        );
    }
    // The rows that must not move did not.
    for (id, cc) in [(11i64, "SK"), (30, "DE"), (40, "FR"), (50, "DE"), (90, "NL")] {
        assert_eq!(
            text1(&conn, &format!("SELECT country FROM organizations WHERE id = {id}")).await.as_deref(),
            Some(cc)
        );
    }
    // Pre-images on the moved rows; no-ops stamped with their reason; the
    // medium and the keep stay pending.
    assert_eq!(
        text1(&conn, "SELECT applied_action FROM org_country_verdicts WHERE org_id = 10").await.as_deref(),
        Some("moved from SG")
    );
    assert_eq!(
        text1(&conn, "SELECT applied_action FROM org_country_verdicts WHERE org_id = 80").await.as_deref(),
        Some("moved from NULL")
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_country_verdicts WHERE job_id = 77").await, 6);
    assert_eq!(
        text1(&conn, "SELECT applied_action FROM org_country_verdicts WHERE org_id = 50").await.as_deref(),
        Some("no-op: row stands under DE, the verdict saw IT")
    );
    assert_eq!(
        text1(&conn, "SELECT applied_action FROM org_country_verdicts WHERE org_id = 60").await.as_deref(),
        Some("no-op: org 60 no longer exists")
    );
    assert_eq!(
        text1(&conn, "SELECT applied_action FROM org_country_verdicts WHERE org_id = 90").await.as_deref(),
        Some("no-op: already under NL")
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_country_verdicts WHERE applied_at IS NULL").await, 2);
    // A second wet run finds only the parked pair and moves nothing.
    let again = db.apply_country_verdicts(false, Some(&[]), Some(78), 4_000).await.unwrap();
    assert_eq!((again.pending, again.eligible, again.moved, again.noop), (2, 0, 0, 0));
}

#[tokio::test]
async fn a_wet_run_against_a_stale_plan_refuses_and_writes_nothing() {
    let (db, conn) = seed("stale").await;
    // The reviewed plan named only org 20; the live computation names three.
    let stale = [(20i64, Some("NL".to_owned()), "ES".to_owned())];
    let err = db.apply_country_verdicts(false, Some(&stale), Some(1), 3_000).await.unwrap_err();
    assert!(err.to_string().contains("ABORTED"), "{err}");
    assert_eq!(text1(&conn, "SELECT country FROM organizations WHERE id = 20").await.as_deref(), Some("NL"));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_country_verdicts WHERE applied_at IS NOT NULL").await, 0);
}

#[tokio::test]
async fn re_recording_replaces_the_verdict_but_never_an_applied_stamp() {
    let (db, conn) = seed("rerecord").await;
    let plan = db.apply_country_verdicts(true, None, None, 2_000).await.unwrap().plan;
    let expect: Vec<(i64, Option<String>, String)> =
        plan.iter().map(|m| (m.org, m.from.clone(), m.to.clone())).collect();
    db.apply_country_verdicts(false, Some(&expect), Some(5), 3_000).await.unwrap();
    // A fresh review of org 10 says keep — the verdict changes, the record of
    // where the row came from does not.
    let again = [verdict(10, "keep", Some("SK"), None, "medium")];
    assert_eq!(db.record_country_verdicts("xb-test", &again, 9_000).await.unwrap(), 1);
    assert_eq!(text1(&conn, "SELECT action FROM org_country_verdicts WHERE org_id = 10").await.as_deref(), Some("keep"));
    assert_eq!(
        text1(&conn, "SELECT applied_action FROM org_country_verdicts WHERE org_id = 10").await.as_deref(),
        Some("moved from SG")
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_country_verdicts WHERE org_id = 10 AND applied_at = 3000").await, 1);
    // Another cohort's verdict on the same row is a separate record.
    let other = [verdict(10, "move", Some("SK"), Some("CZ"), "low")];
    assert_eq!(db.record_country_verdicts("second-look", &other, 9_500).await.unwrap(), 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_country_verdicts WHERE org_id = 10").await, 2);
}

#[tokio::test]
async fn a_move_that_moves_nothing_is_refused_at_record_time() {
    let (db, _conn) = open("invalid").await;
    for bad in [
        verdict(1, "move", Some("BE"), Some("BE"), "high"),
        verdict(1, "move", Some("BE"), None, "high"),
        verdict(1, "move", Some("BE"), Some("nl"), "high"),
        verdict(1, "move", Some(""), Some("NL"), "high"),
        verdict(1, "split", Some("BE"), Some("NL"), "high"),
    ] {
        let err = db.record_country_verdicts("c", &[bad.clone()], 1).await.unwrap_err();
        assert!(!err.to_string().is_empty(), "{bad:?}");
    }
    assert_eq!(db.record_country_verdicts("c", &[verdict(1, "keep", None, None, "low")], 1).await.unwrap(), 1);
    // The pre-image is whatever the row carries — a junk code like `1A` is
    // exactly what a contamination looks like (the 355 campaign's first POST
    // was refused on one), so it must be recordable and, once the row still
    // reads `1A`, movable.
    assert_eq!(
        db.record_country_verdicts("c", &[verdict(2, "move", Some("1A"), Some("IL"), "high")], 1).await.unwrap(),
        1
    );
}

#[tokio::test]
async fn a_junk_pre_image_code_is_a_real_pre_image_and_the_row_moves() {
    let (db, conn) = open("junk").await;
    org(&conn, 2, Some("1A"), Some("national"), Some("520045678"), "Elbit Systems Land Ltd").await;
    db.record_country_verdicts("c", &[verdict(2, "move", Some("1A"), Some("IL"), "high")], 1).await.unwrap();
    let plan = db.apply_country_verdicts(true, None, None, 2).await.unwrap();
    assert_eq!((plan.eligible, plan.moved), (1, 1));
    let expect = [(2i64, Some("1A".to_owned()), "IL".to_owned())];
    db.apply_country_verdicts(false, Some(&expect), Some(9), 3).await.unwrap();
    assert_eq!(text1(&conn, "SELECT country FROM organizations WHERE id = 2").await.as_deref(), Some("IL"));
    assert_eq!(
        text1(&conn, "SELECT applied_action FROM org_country_verdicts WHERE org_id = 2").await.as_deref(),
        Some("moved from 1A")
    );
}

#[tokio::test]
async fn the_verdict_stores_read_back_bounded_and_by_cohort() {
    let (db, _conn) = seed("readback").await;
    let (cols, rows) = db.verdict_rows("country", Some("xb-test"), 1_000).await.unwrap();
    assert_eq!(cols[0], "org_id");
    assert_eq!(rows.len(), 8);
    let (_, rows) = db.verdict_rows("country", Some("other-cohort"), 1_000).await.unwrap();
    assert!(rows.is_empty());
    let (_, rows) = db.verdict_rows("country", None, 3).await.unwrap();
    assert_eq!(rows.len(), 3, "`limit` bounds the read");
    for table in ["case", "rehoming", "name"] {
        let (cols, rows) = db.verdict_rows(table, None, 10).await.unwrap();
        assert!(!cols.is_empty() && rows.is_empty(), "{table}: readable, empty here");
    }
    assert!(db.verdict_rows("secrets", None, 10).await.is_err(), "only the four stores, by fixed name");
}
