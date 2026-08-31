//! Issue 325 step 4: re-parse the standing rows and write what the parser now
//! says.
//!
//! The repair's whole design is that it does NOT reimplement "which rows have a
//! country taken out of a word" — it injects the classifier and compares. So
//! these tests inject a fake one, and what they pin is the surrounding
//! judgement: whose country wins, when to refuse to act, that BOTH fields move
//! together, and that a wet pass will not write over a row that moved under it.

use store::turso::{self, Value};

/// A fake identifier parser with the shape of the real one after issue 325: a
/// two-letter VAT prefix followed by the number keeps `vat` and is scoped by
/// its own (canonicalised) prefix; anything else is `national` and takes the
/// mention's country.
fn reclassify(value: &str, mention: Option<&str>) -> Option<(String, Option<String>)> {
    if value.len() < 4 {
        return None; // the v2 gate's territory: refused outright
    }
    let prefix: String = value.chars().take(2).collect();
    let body = &value[2..];
    let vat_country = matches!(prefix.as_str(), "BE" | "DE" | "FR" | "EL" | "NO");
    let looks_like_a_number =
        body.chars().any(|c| c.is_ascii_digit()) && body.len() <= 14 && !has_letter_run(body, 3);
    if vat_country && looks_like_a_number {
        // `EL` canonicalises to `GR`, as the real one does.
        let cc = if prefix == "EL" { "GR".to_owned() } else { prefix };
        return Some(("vat".to_owned(), Some(cc)));
    }
    Some(("national".to_owned(), mention.map(str::to_owned)))
}

fn has_letter_run(s: &str, n: usize) -> bool {
    let mut run = 0usize;
    for b in s.bytes() {
        run = if b.is_ascii_alphabetic() { run + 1 } else { 0 };
        if run >= n {
            return true;
        }
    }
    false
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

async fn org(conn: &turso::Connection, id: i64, cc: &str, kind: &str, ident: &str) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
         VALUES (?, ?, ?, ?, 'Some Body', 0, 0)",
        (
            Value::Integer(id),
            Value::Text(cc.into()),
            Value::Text(kind.into()),
            Value::Text(ident.into()),
        ),
    )
    .await
    .unwrap();
}

/// Attach `n` mentions to `org`, each stating `cc`.
async fn mentions(conn: &turso::Connection, org_id: i64, cc: &str, n: i64, base: i64) {
    for i in 0..n {
        let notice = base + i;
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
             VALUES (?, 'ORG-' || ?, ?, 'Some Body', ?, NULL)",
            (
                Value::Integer(notice),
                Value::Integer(notice),
                Value::Integer(org_id),
                Value::Text(cc.into()),
            ),
        )
        .await
        .unwrap();
    }
}

fn never() -> bool {
    false
}

/// The core claim: a row whose country was taken out of a word gets the
/// publisher's country AND the parser's kind, in one move.
#[tokio::test]
async fn the_repair_moves_the_country_and_the_kind_together() {
    let (db, conn) = open("test-mint-core").await;
    // The defect: a German reporting-unit id filed as a Belgian VAT number.
    org(&conn, 1, "BE", "vat", "BERICHTSEINHEITID00002636").await;
    mentions(&conn, 1, "DE", 4, 100).await;
    db.build_organization_indexes().await.unwrap();

    let r = db.repair_minted_countries(reclassify, true, None, &never).await.unwrap();
    assert_eq!(r.walked, 1);
    assert_eq!(r.rows, 1);
    let fix = &r.plan[0];
    assert_eq!((fix.from_kind.as_str(), fix.from_country.as_deref()), ("vat", Some("BE")));
    assert_eq!(
        (fix.to_kind.as_str(), fix.to_country.as_deref()),
        ("national", Some("DE")),
        "BOTH fields move: a country-only repair would leave `vat` on a value the \
         parser calls national, and a fresh mention would still key differently"
    );
    assert_eq!(fix.mentions, 4);
    assert_eq!(r.applied, 0, "a dry run writes nothing");

    // Wet, with the reviewed count.
    let w = db.repair_minted_countries(reclassify, false, Some(1), &never).await.unwrap();
    assert_eq!(w.applied, 1);
    let mut rows = conn
        .query("SELECT country, identifier_kind FROM organizations WHERE id = 1", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(
        (row.get_value(0).unwrap(), row.get_value(1).unwrap()),
        (Value::Text("DE".into()), Value::Text("national".into()))
    );
    // And a change event, because both are published fields.
    let mut ev = conn
        .query(
            "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 1",
            (),
        )
        .await
        .unwrap();
    let n = ev.next().await.unwrap().unwrap();
    assert!(
        matches!(n.get_value(0).unwrap(), Value::Integer(k) if k >= 1),
        "a consumer filtering on country has to see the correction go by"
    );
}

/// Where the publisher's own mentions disagree, the repair does nothing. Two
/// rows on prod are in this state, and a coin-flip on a published field is worse
/// than a value that is visibly wrong.
#[tokio::test]
async fn a_row_whose_mentions_disagree_is_counted_and_left_alone() {
    let (db, conn) = open("test-mint-ambig").await;
    org(&conn, 1, "BE", "vat", "BERICHTSEINHEITID00002636").await;
    mentions(&conn, 1, "DE", 3, 100).await;
    mentions(&conn, 1, "AT", 2, 200).await;
    // A row with no stated country anywhere is its own bucket, not the same one.
    org(&conn, 2, "BE", "vat", "CHARITYNUMBER1040303").await;
    db.build_organization_indexes().await.unwrap();

    let r = db.repair_minted_countries(reclassify, true, None, &never).await.unwrap();
    assert_eq!(r.walked, 2);
    assert_eq!(r.ambiguous, 1);
    assert_eq!(r.no_mention_country, 1);
    assert_eq!(r.rows, 0, "neither row is repairable, and neither is guessed at");
    assert!(r.plan.is_empty());
}

/// An accidentally-correct row is not a change. `UKCOMPANYREGISTER…` really is
/// British and `EL…` really is Greek — the first stays put, the second moves
/// only because `EL` is not the alpha-2 spelling (issue 319).
#[tokio::test]
async fn a_row_the_parser_still_agrees_with_is_not_touched() {
    let (db, conn) = open("test-mint-agree").await;
    // A real VAT id, already canonical: nothing to do.
    org(&conn, 1, "DE", "vat", "DE136695976").await;
    mentions(&conn, 1, "DE", 2, 100).await;
    // A real Greek VAT id stored under the non-ISO `EL`: kind stays, code moves.
    org(&conn, 2, "EL", "vat", "EL094019245").await;
    mentions(&conn, 2, "GR", 2, 200).await;
    db.build_organization_indexes().await.unwrap();

    let r = db.repair_minted_countries(reclassify, true, None, &never).await.unwrap();
    assert_eq!(r.walked, 2);
    assert_eq!(r.rows, 1, "only the non-canonical code is a change");
    let fix = &r.plan[0];
    assert_eq!(fix.org, 2);
    assert_eq!((fix.from_country.as_deref(), fix.to_country.as_deref()), (Some("EL"), Some("GR")));
    assert_eq!(
        (fix.from_kind.as_str(), fix.to_kind.as_str()),
        ("vat", "vat"),
        "a real VAT id stays a VAT id — this half of the repair is issue 319's \
         vocabulary, not a reclassification"
    );
}

/// A value the parser now REFUSES outright is counted, never acted on.
/// Stripping a published identifier is what issue 312 had to undo.
#[tokio::test]
async fn a_value_the_gate_now_refuses_is_counted_and_left_standing() {
    let (db, conn) = open("test-mint-refused").await;
    org(&conn, 1, "BE", "vat", "BE1").await; // shorter than the fake gate's floor
    mentions(&conn, 1, "DE", 2, 100).await;
    db.build_organization_indexes().await.unwrap();

    let r = db.repair_minted_countries(reclassify, true, None, &never).await.unwrap();
    assert_eq!(r.now_refused, 1);
    assert_eq!(r.rows, 0);

    let w = db.repair_minted_countries(reclassify, false, None, &never).await.unwrap();
    assert_eq!(w.applied, 0);
    let mut rows = conn
        .query("SELECT identifier FROM organizations WHERE id = 1", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get_value(0).unwrap(), Value::Text("BE1".into()), "the value stands");
}

/// WHAT THE REPAIR DOES TO A HAND CORRECTION — and the answer turns on the
/// walk's SCOPE, which two drafts of this test got wrong before measuring it.
///
/// The walk covers `identifier_kind = 'vat'` only, because that is the only
/// class the parser change can move (a value it called `national` before still
/// does). Two consequences fall straight out of that, and both matter:
///
/// * **The job is idempotent.** A row it repairs becomes `national` and is out
///   of scope from then on. Running it twice is a no-op, and a correction it
///   made is never re-litigated.
/// * **Which hand corrections survive depends on whether they change the KIND.**
///   Setting a row to `(AT, national)` takes it out of scope and it stands.
///   Setting it to `(AT, vat)` leaves it in scope, and the publisher's stated
///   country wins on the next run.
///
/// That asymmetry is not a design anyone would choose deliberately, so it is
/// pinned here rather than left to be rediscovered. An override that must
/// survive regardless needs a marker of its own, the way `org_case_reviews`
/// stamps an applied verdict — this job has no such stamp and should not be
/// used as one.
#[tokio::test]
async fn what_survives_a_hand_correction_depends_on_the_kind() {
    let (db, conn) = open("test-mint-moved").await;
    org(&conn, 1, "BE", "vat", "BERICHTSEINHEITID00002636").await;
    mentions(&conn, 1, "DE", 3, 100).await;
    org(&conn, 2, "BE", "vat", "BERLINCHARLOTTENBURG93627").await;
    mentions(&conn, 2, "DE", 3, 200).await;
    org(&conn, 3, "BE", "vat", "FINANZAMTBIELEFELD34959").await;
    mentions(&conn, 3, "DE", 3, 300).await;
    db.build_organization_indexes().await.unwrap();

    let dry = db.repair_minted_countries(reclassify, true, None, &never).await.unwrap();
    assert_eq!(dry.rows, 3);

    // Org 1: corrected by hand AND reclassified — out of the walk's scope.
    conn.execute(
        "UPDATE organizations SET country = 'AT', identifier_kind = 'national' WHERE id = 1",
        (),
    )
    .await
    .unwrap();
    // Org 3: the country changed by hand, the kind left alone — still in scope.
    conn.execute("UPDATE organizations SET country = 'AT' WHERE id = 3", ()).await.unwrap();

    let w = db.repair_minted_countries(reclassify, false, Some(3), &never).await.unwrap();
    assert_eq!(w.walked, 2, "org 1 left the population when its kind changed");
    assert_eq!(w.applied, 2, "orgs 2 and 3");
    assert_eq!(w.skipped_moved, 0, "nothing drifted WITHIN the run");

    let read = async |id: i64| -> String {
        let mut rows = conn
            .query("SELECT country FROM organizations WHERE id = ?", (Value::Integer(id),))
            .await
            .unwrap();
        match rows.next().await.unwrap().unwrap().get_value(0).unwrap() {
            Value::Text(s) => s,
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(read(1).await, "AT", "the reclassified hand correction stands");
    assert_eq!(read(3).await, "DE", "the country-only one does not — the publisher wins");
    assert_eq!(read(2).await, "DE");

    // …and a second run changes nothing, because everything it touched is now
    // `national`.
    let again = db.repair_minted_countries(reclassify, true, None, &never).await.unwrap();
    assert_eq!(again.rows, 0, "idempotent: a repaired row is out of scope");
}

/// The parity gate: a wet run whose plan has drifted from the reviewed one
/// aborts instead of applying a different plan than the one that was cleared.
#[tokio::test]
async fn a_plan_that_drifted_since_the_review_aborts() {
    let (db, conn) = open("test-mint-parity").await;
    for id in 1..=12i64 {
        org(&conn, id, "BE", "vat", &format!("BERICHTSEINHEITID0000{id:04}")).await;
        mentions(&conn, id, "DE", 1, 1000 + id * 10).await;
    }
    db.build_organization_indexes().await.unwrap();

    let dry = db.repair_minted_countries(reclassify, true, None, &never).await.unwrap();
    assert_eq!(dry.rows, 12);

    // The reviewed plan said 3 rows; this run computes 12. Beyond max(2%, 5).
    let err = db.repair_minted_countries(reclassify, false, Some(3), &never).await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("ABORTED"), "{msg}");
    assert!(msg.contains("3") && msg.contains("12"), "the message names both counts: {msg}");

    // Nothing was written.
    let mut rows = conn
        .query("SELECT COUNT(*) FROM organizations WHERE identifier_kind = 'national'", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get_value(0).unwrap(), Value::Integer(0));
}

/// A collision is reported, not refused: two rows sharing
/// `(country, kind, identifier)` is what the R2 merge arm folds, and reaching it
/// is the repair's whole argument. A reviewer should see the downstream merge
/// work in advance rather than discover it.
#[tokio::test]
async fn a_repair_that_lands_on_an_existing_identity_reports_a_collision() {
    let (db, conn) = open("test-mint-collide").await;
    // The contaminated row…
    org(&conn, 1, "BE", "vat", "BERICHTSEINHEITID00002636").await;
    mentions(&conn, 1, "DE", 3, 100).await;
    // …and the row it will land on top of, already correct.
    org(&conn, 2, "DE", "national", "BERICHTSEINHEITID00002636").await;
    db.build_organization_indexes().await.unwrap();

    let r = db.repair_minted_countries(reclassify, true, None, &never).await.unwrap();
    assert_eq!(r.rows, 1);
    assert_eq!(r.collisions, 1, "one identity would be held by two rows");

    // And it still applies — the merge arm is the next step, not a blocker.
    let w = db.repair_minted_countries(reclassify, false, Some(1), &never).await.unwrap();
    assert_eq!(w.applied, 1);
}

/// A cancelled wet pass is honest: the committed prefix stands and `stopped`
/// says so, rather than reporting a clean finish over a partial write.
#[tokio::test]
async fn a_cancelled_repair_says_it_stopped() {
    let (db, conn) = open("test-mint-cancel").await;
    org(&conn, 1, "BE", "vat", "BERICHTSEINHEITID00002636").await;
    mentions(&conn, 1, "DE", 2, 100).await;
    db.build_organization_indexes().await.unwrap();

    let always = || true;
    let r = db.repair_minted_countries(reclassify, true, None, &always).await.unwrap();
    assert!(r.stopped);
    assert_eq!(r.rows, 0);
    assert!(r.plan.is_empty(), "a partial plan reviewed as a whole one would under-run");
}
