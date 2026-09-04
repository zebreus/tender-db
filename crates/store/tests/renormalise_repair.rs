//! Issue 345: re-parse every standing row's PUBLISHED identifier string through
//! the live normaliser and move what it now says.
//!
//! What these pin is the judgement around the write, not the folds — those are
//! unit-tested in `crates/ingest`. Here: that a row moves only when a sampled
//! mention's published string reproduces the stored triple under the OLD rules
//! (the witness) and the NEW rules disagree; that a row explained by no mention
//! (only merged-in ones) stays put; that the published string survives in the
//! mention; that a re-run plans nothing.

use store::turso::{self, Value};

/// Stand-ins with the real functions' shape. `before` is the ASCII filter as it
/// stood; `live` adds the two v2.1 folds (Greek Ε → E, RO `_n` suffix off).
fn classify(value: &str, country: Option<&str>) -> Option<(String, Option<String>, String)> {
    let v: String = value.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_uppercase()).collect();
    if v.len() < 4 || !v.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(("national".to_owned(), country.map(str::to_owned), v))
}
fn before(value: &str, country: Option<&str>) -> Option<(String, Option<String>, String)> {
    classify(value, country)
}
fn live(value: &str, country: Option<&str>) -> Option<(String, Option<String>, String)> {
    let folded: String = value.chars().map(|c| if c == '\u{0395}' { 'E' } else { c }).collect();
    let folded = match country {
        Some("RO") => match folded.rsplit_once('_') {
            Some((head, tail)) if tail.len() == 1 && tail.chars().all(|c| c.is_ascii_digit()) => head.to_owned(),
            _ => folded,
        },
        _ => folded,
    };
    classify(&folded, country)
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

async fn org(conn: &turso::Connection, id: i64, cc: &str, ident: &str, name: &str) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
         VALUES (?, ?, 'national', ?, ?, 0, 0)",
        (Value::Integer(id), Value::Text(cc.into()), Value::Text(ident.into()), Value::Text(name.into())),
    )
    .await
    .unwrap();
}

async fn mention(conn: &turso::Connection, org_id: i64, cc: &str, raw: &str, notice: i64) {
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
         VALUES (?, 'ORG-1', ?, 'x', ?, ?)",
        (Value::Integer(notice), Value::Integer(org_id), Value::Text(cc.into()), Value::Text(raw.into())),
    )
    .await
    .unwrap();
}

fn never() -> bool {
    false
}

async fn read(conn: &turso::Connection, id: i64) -> (String, String) {
    let mut rows = conn
        .query("SELECT identifier_kind, identifier FROM organizations WHERE id = ?", (Value::Integer(id),))
        .await
        .unwrap();
    let r = rows.next().await.unwrap().unwrap();
    let g = |i| match r.get_value(i).unwrap() {
        Value::Text(s) => s,
        other => panic!("{other:?}"),
    };
    (g(0), g(1))
}

/// The core: the Greek-letter row and the suffixed RO row move to what the
/// live rules say; both land on a standing twin (reunions); a clean row and a
/// merged-in-only row stay; a re-run plans nothing.
#[tokio::test]
async fn witnessed_rows_move_to_the_live_reading_and_the_rest_stand() {
    let (db, conn) = open("test-renorm-core").await;
    // 1: the Greek authority's second row — stored under the letter-dropped
    //    form, published with the Greek Ε.
    org(&conn, 1, "GR", "1000009610001", "EADHSY").await;
    mention(&conn, 1, "GR", "1000.\u{0395}00961.0001", 100).await;
    // 2: its Latin twin, already standing where 1 will land.
    org(&conn, 2, "GR", "1000E009610001", "EADHSY").await;
    mention(&conn, 2, "GR", "1000.E00961.0001", 101).await;
    // 3: a Romanian directorate under the suffixed CUI; 4: the parent.
    org(&conn, 3, "RO", "160543683", "DRDP Cluj").await;
    mention(&conn, 3, "RO", "16054368_3", 102).await;
    org(&conn, 4, "RO", "16054368", "CNAIR").await;
    mention(&conn, 4, "RO", "16054368", 103).await;
    // 5: a row whose only mention came in through a merge — its published
    //    string does not reproduce the stored value under the old rules.
    org(&conn, 5, "FR", "180014045", "CNFPT").await;
    mention(&conn, 5, "FR", "18001404501577", 104).await;
    // 6: a row with no published string at all.
    org(&conn, 6, "DE", "DE123456789", "X GmbH").await;
    db.build_organization_indexes().await.unwrap();

    let p = db.repair_renormalised_identifiers(before, live, true, None, &never).await.unwrap();
    assert_eq!(p.walked, 6);
    assert_eq!(p.witnessed, 4, "1, 2, 3, 4 are explained by their own mention");
    assert_eq!(p.unexplained, 2, "5 (merged-in only) and 6 (no string)");
    assert_eq!(p.already_clean, 2, "2 and 4 read the same under both rules");
    assert_eq!(p.rows, 2, "1 and 3 move");
    assert_eq!(p.reunions, 2, "both land on a standing twin");
    let to: Vec<(i64, &str)> = p.plan.iter().map(|f| (f.org, f.to_identifier.as_str())).collect();
    assert_eq!(to, vec![(1, "1000E009610001"), (3, "16054368")]);
    assert_eq!(p.applied, 0, "a dry run writes nothing");

    let w = db.repair_renormalised_identifiers(before, live, false, Some(2), &never).await.unwrap();
    assert_eq!(w.applied, 2);
    assert_eq!(read(&conn, 1).await.1, "1000E009610001");
    assert_eq!(read(&conn, 3).await.1, "16054368");
    assert_eq!(read(&conn, 5).await.1, "180014045", "the unexplained row stands");

    // The published strings are untouched.
    let mut rows = conn
        .query("SELECT raw_identifier FROM organization_mentions WHERE organization_id = 1", ())
        .await
        .unwrap();
    let r = rows.next().await.unwrap().unwrap();
    assert_eq!(r.get_value(0).unwrap(), Value::Text("1000.\u{0395}00961.0001".into()));

    // Idempotent: the moved rows now carry the live reading, which the OLD
    // rules do not reproduce from their published string — so they become
    // "unexplained" rather than re-planned, and nothing moves twice.
    let again = db.repair_renormalised_identifiers(before, live, true, None, &never).await.unwrap();
    assert_eq!(again.rows, 0);
    assert_eq!(again.unexplained, 4);
}

/// The wet arm refuses a plan that no longer matches the corpus (328 parity).
#[tokio::test]
async fn the_wet_arm_aborts_when_the_reviewed_count_has_moved() {
    let (db, conn) = open("test-renorm-parity").await;
    org(&conn, 1, "GR", "1000009610001", "EADHSY").await;
    mention(&conn, 1, "GR", "1000.\u{0395}00961.0001", 100).await;
    db.build_organization_indexes().await.unwrap();
    let err = db.repair_renormalised_identifiers(before, live, false, Some(40), &never).await.unwrap_err();
    assert!(err.to_string().contains("ABORTED"), "{err}");
    assert_eq!(read(&conn, 1).await.1, "1000009610001", "nothing moved");
}
