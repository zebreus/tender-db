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

    let p = db.repair_renormalised_identifiers(before, live, true, None, None, &never).await.unwrap();
    assert_eq!(p.walked, 6);
    assert_eq!(p.witnessed, 4, "1, 2, 3, 4 are explained by their own mention");
    assert_eq!(p.unexplained, 2, "5 (merged-in only) and 6 (no string)");
    assert_eq!(p.already_clean, 2, "2 and 4 read the same under both rules");
    assert_eq!(p.rows, 2, "1 and 3 move");
    assert_eq!(p.reunions, 2, "both land on a standing twin");
    let to: Vec<(i64, &str)> = p.plan.iter().map(|f| (f.org, f.to_identifier.as_str())).collect();
    assert_eq!(to, vec![(1, "1000E009610001"), (3, "16054368")]);
    assert_eq!(p.applied, 0, "a dry run writes nothing");

    let w = db.repair_renormalised_identifiers(before, live, false, Some(2), None, &never).await.unwrap();
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
    let again = db.repair_renormalised_identifiers(before, live, true, None, None, &never).await.unwrap();
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
    let err = db.repair_renormalised_identifiers(before, live, false, Some(40), None, &never).await.unwrap_err();
    assert!(err.to_string().contains("ABORTED"), "{err}");
    assert_eq!(read(&conn, 1).await.1, "1000009610001", "nothing moved");
}

// ---- Issue 514 unit 2: the register-spelling re-key (decision §2.2, §6) ----

/// `live` with issue 514's rewrite in miniature: under GB, a UKPRN behind repeated
/// `UKPRN` labels, and Companies House's `IP` + five digits + `R` society form, lose
/// their label. Everything else reads as `before` does (`ip6243r` is four digits, so it
/// stays; `RS008376` is never touched).
fn live_514(value: &str, country: Option<&str>) -> Option<(String, Option<String>, String)> {
    let (kind, cc, v) = classify(value, country)?;
    if country != Some("GB") {
        return Some((kind, cc, v));
    }
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let rewritten = if let Some(mut rest) = v.strip_prefix("GBUKPRN") {
        let mut labels = 0;
        while let Some(r) = rest.strip_prefix("UKPRN") {
            rest = r;
            labels += 1;
        }
        (labels > 0 && rest.len() == 8 && digits(rest)).then(|| format!("GBUKPRN{rest}"))
    } else {
        v.strip_prefix("GBMPRIP")
            .and_then(|n| n.strip_suffix('R'))
            .filter(|d| d.len() == 5 && digits(d) && !d.starts_with('0'))
            .map(|d| format!("GBMPR{d}R"))
    };
    Some((kind, cc, rewritten.unwrap_or(v)))
}

fn plan_set(p: &store::RenormaliseRepairReport) -> std::collections::HashSet<store::LabelFix> {
    p.plan.iter().cloned().collect()
}

async fn count(conn: &turso::Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let Value::Integer(n) = row.get_value(0).unwrap() else { panic!("count") };
    n
}

/// The label spellings move onto the identity their bare twins hold (reunions, the E0
/// fold's input); the typo `ip6243r` and the deferred `RS008376` stand; a second pass
/// plans nothing, because the old rules no longer reproduce what the moved rows store.
#[tokio::test]
async fn a_register_label_spelling_moves_onto_its_bare_twin() {
    let (db, conn) = open("test-renorm-514-core").await;
    org(&conn, 10, "GB", "GBUKPRN10007798", "Leeds City College").await;
    mention(&conn, 10, "GB", "GB-UKPRN-10007798", 200).await;
    org(&conn, 11, "GB", "GBUKPRNUKPRN10007798", "Leeds City College").await;
    mention(&conn, 11, "GB", "GB-UKPRN-UKPRN 10007798", 201).await;
    org(&conn, 12, "GB", "GBMPRIP16243R", "Hastoe Housing").await;
    mention(&conn, 12, "GB", "GB-MPR-IP16243R", 202).await;
    org(&conn, 13, "GB", "GBMPRIP6243R", "Hastoe Housing Association").await;
    mention(&conn, 13, "GB", "GB-MPR-ip6243r", 203).await;
    org(&conn, 14, "GB", "GBMPRRS008376", "Connexus").await;
    mention(&conn, 14, "GB", "GB-MPR-RS008376", 204).await;
    // The same label spelling outside GB: no rewrite, no move.
    org(&conn, 15, "IE", "GBUKPRNUKPRN10007798", "Leeds City College").await;
    mention(&conn, 15, "IE", "GB-UKPRN-UKPRN 10007798", 205).await;
    db.build_organization_indexes().await.unwrap();

    let p = db.repair_renormalised_identifiers(before, live_514, true, None, None, &never).await.unwrap();
    let to: Vec<(i64, &str, &str)> =
        p.plan.iter().map(|f| (f.org, f.from_identifier.as_str(), f.to_identifier.as_str())).collect();
    assert_eq!(to, vec![(11, "GBUKPRNUKPRN10007798", "GBUKPRN10007798"), (12, "GBMPRIP16243R", "GBMPR16243R")]);
    assert_eq!((p.rows, p.reunions, p.already_clean), (2, 1, 4), "{p:?}");

    let w = db
        .repair_renormalised_identifiers(before, live_514, false, Some(p.rows), Some(&plan_set(&p)), &never)
        .await
        .unwrap();
    assert_eq!((w.applied, w.held_unplanned, w.preimages), (2, 0, 2), "{w:?}");
    assert_eq!(read(&conn, 11).await.1, "GBUKPRN10007798");
    assert_eq!(read(&conn, 12).await.1, "GBMPR16243R");
    assert_eq!(read(&conn, 13).await.1, "GBMPRIP6243R", "the typo stands");
    assert_eq!(read(&conn, 14).await.1, "GBMPRRS008376", "RS is deferred to unit 3");
    assert_eq!(read(&conn, 15).await.1, "GBUKPRNUKPRN10007798", "GB only");

    let again = db.repair_renormalised_identifiers(before, live_514, true, None, None, &never).await.unwrap();
    assert_eq!(again.rows, 0, "{again:?}");
}

/// G2: the wet pass moves only the `(org, from, to)` rows of the stored plan. A row the
/// recompute plans but the plan lacks (drift since the review), and a row whose target
/// changed, are held and left standing.
#[tokio::test]
async fn a_wet_run_moves_only_stored_plan_rows() {
    let (db, conn) = open("test-renorm-514-bound").await;
    org(&conn, 20, "GB", "GBUKPRNUKPRN10007798", "Leeds City College").await;
    mention(&conn, 20, "GB", "GB-UKPRN-UKPRN 10007798", 300).await;
    org(&conn, 21, "GB", "GBUKPRNUKPRN10007339", "Warrington College").await;
    mention(&conn, 21, "GB", "GB-UKPRN-UKPRN 10007339", 301).await;
    org(&conn, 22, "GB", "GBMPRIP16337R", "Metropolitan").await;
    mention(&conn, 22, "GB", "GB-MPR-IP16337R", 302).await;
    db.build_organization_indexes().await.unwrap();

    let p = db.repair_renormalised_identifiers(before, live_514, true, None, None, &never).await.unwrap();
    assert_eq!(p.rows, 3);
    // The reviewed plan: 20 as planned, 22 with another target, 21 missing.
    let mut stored = std::collections::HashSet::new();
    for f in &p.plan {
        match f.org {
            20 => {
                stored.insert(f.clone());
            }
            22 => {
                stored.insert(store::LabelFix { to_identifier: "GBMPR16377R".into(), ..f.clone() });
            }
            _ => {}
        }
    }
    let w = db.repair_renormalised_identifiers(before, live_514, false, Some(3), Some(&stored), &never).await.unwrap();
    assert_eq!((w.applied, w.held_unplanned), (1, 2), "{w:?}");
    assert_eq!(read(&conn, 20).await.1, "GBUKPRN10007798");
    assert_eq!(read(&conn, 21).await.1, "GBUKPRNUKPRN10007339", "unplanned: held");
    assert_eq!(read(&conn, 22).await.1, "GBMPRIP16337R", "a changed target: held");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organization_merged_identifiers").await, 1, "only the move records");
}

/// G5: the moved row keeps its published spelling findable — a merged identifier of
/// its own (`loser` = the row, rule `renormalise`) — and the E0 merge that reunites it
/// with its twin carries that row to the survivor, so `?identifier=` on the polluted
/// spelling answers the survivor.
#[tokio::test]
async fn a_moved_row_keeps_its_old_spelling_findable() {
    let (db, conn) = open("test-renorm-514-preimage").await;
    org(&conn, 30, "GB", "GBUKPRN10007798", "Leeds City College").await;
    mention(&conn, 30, "GB", "GB-UKPRN-10007798", 400).await;
    org(&conn, 31, "GB", "GBUKPRNUKPRN10007798", "Leeds City College").await;
    mention(&conn, 31, "GB", "GB-UKPRN-UKPRN 10007798", 401).await;
    db.build_organization_indexes().await.unwrap();

    let p = db.repair_renormalised_identifiers(before, live_514, true, None, None, &never).await.unwrap();
    let w = db
        .repair_renormalised_identifiers(before, live_514, false, Some(p.rows), Some(&plan_set(&p)), &never)
        .await
        .unwrap();
    assert_eq!((w.applied, w.preimages), (1, 1));
    let row = "SELECT COUNT(*) FROM organization_merged_identifiers WHERE identifier = 'GBUKPRNUKPRN10007798' \
               AND identifier_kind = 'national' AND country = 'GB' AND rule = 'renormalise'";
    assert_eq!(count(&conn, &format!("{row} AND org_id = 31 AND loser = 31")).await, 1, "loser = the row itself");

    // The E0 fold reunites them; 30 survives (lower id) and the pre-image follows.
    fn e0_key(_c: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool, bool)> {
        let norm: String = value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
        Some(("E0", format!("{kind}:{norm}"), true, false))
    }
    fn own(country: Option<&str>, raw: &str) -> Option<(&'static str, String)> {
        live_514(raw, country).map(|(_, _, v)| ("GB:ukprn", v)).filter(|(_, v)| v == "GBUKPRN10007798")
    }
    let args = |dry_run: bool| store::R2MergeArgs {
        key: e0_key,
        name_key: |n| n.to_lowercase(),
        names_agree: |a, b| a == b,
        trim: |n| n.to_owned(),
        norm: |n| n.to_lowercase(),
        condemns: |_, _, _| false,
        consortium: |_| false,
        legal_form: |_| None,
        rule: "e0",
        n3: |n| n.to_lowercase(),
        stoplist_cap: 20,
        dry_run,
        max_groups: None,
        expect_groups: (!dry_run).then_some(1),
        register_identity: Some(own),
        expect_listing: None,
        job_id: None,
        stop: &|| false,
    };
    let dry = db.match_org_identifiers_r2(args(true)).await.unwrap();
    assert_eq!(dry.plan_groups, 1, "{dry:?}");
    let wet = db.match_org_identifiers_r2(args(false)).await.unwrap();
    assert_eq!(wet.removed, 1);
    assert_eq!(count(&conn, &format!("{row} AND org_id = 30 AND loser = 31")).await, 1, "carried to the survivor");
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organization_merged_identifiers WHERE identifier = 'GBUKPRN10007798'").await,
        0,
        "an E0 merge of identical triples writes nothing for the loser's own identifier"
    );
}
