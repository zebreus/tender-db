//! Issue 328 follow-on: the standing duplicate-identity census.
//!
//! The label-prefix repair left 3,215 exact duplicate `(DE, vat, DEnnnnnnnnn)`
//! triples standing, and `crosswalk::canonical_key` has no German arm — so
//! `match-org-identifiers --r2` will never see them. Adding an arm is only safe
//! if those duplicates are one organization fragmented; a German VAT number can
//! legitimately be shared across an *Organschaft* (a fiscal unity of legally
//! distinct companies), which is the same false-merge shape the CZ699 group-VAT
//! negative already guards against. So the census counts disagreeing names
//! instead of anyone deciding from the armchair.
//!
//! What these tests pin is the JUDGEMENT, not the arithmetic: that a keyed group
//! is left to the merge arm, that agreement on a name nobody chose to make
//! unique is bucketed apart from real agreement, that a branch suffix is not
//! read as conflict, that a nameless group abstains — and that `cap` bounds the
//! LISTING and never the TALLY.

use store::turso::{self, Value};

// STAND-INS, injected exactly as the job injects the real ones. `ingest` depends
// on `store`, so importing `canonical_key_flat` and `n3_key` here would be a
// dependency cycle — and it would be the wrong test besides: this method takes
// them as parameters, so what belongs here is the surrounding judgement. Their
// internals are unit-tested in `crates/ingest/src/crosswalk.rs`.

/// The real one's shape, and the real one's pinned German negative: `DE:vat`
/// keys to nothing, `FR:national` keys. That asymmetry is the whole premise of
/// the census, so the stand-in reproduces it rather than keying everything.
fn key(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool)> {
    match (country, kind) {
        (Some("FR"), "national") => Some(("FR:siren", format!("FR:siren:{value}"), true)),
        _ => None,
    }
}

/// Abstracts the legal-form family the way `n3_key` does, so `AG` and
/// `Aktiengesellschaft` collapse to one key and a spelling difference is not
/// mistaken for a different company.
fn n3(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| match t {
            "ag" | "aktiengesellschaft" => "§ag".to_owned(),
            "gmbh" | "gesellschaft" => "§gmbh".to_owned(),
            other => other.to_owned(),
        })
        .collect::<Vec<String>>()
        .join(" ")
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

/// One org row plus `mentions` mentions of it. `kind` is a parameter because the
/// class under measurement is `vat`, not the `national` the sibling censuses
/// happen to fixture.
async fn org(
    conn: &turso::Connection,
    id: i64,
    cc: &str,
    kind: &str,
    ident: &str,
    name: &str,
    mentions: i64,
) {
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
    for i in 0..mentions {
        let notice = id * 1000 + i;
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
             VALUES (?, 'ORG-' || ?, ?, ?, ?, NULL)",
            (
                Value::Integer(notice),
                Value::Integer(notice),
                Value::Integer(id),
                Value::Text(name.into()),
                Value::Text(cc.into()),
            ),
        )
        .await
        .unwrap();
    }
}

/// Give a name key `carriers` distinct orgs in `org_match_keys` under `kind`,
/// which is what the genericness probe reads.
///
/// `kind` is a parameter because the real build stores the same N3 key under
/// EITHER kind: it writes an `n3` row only when the key differs from the `n2`
/// one, so a name with no legal-form token lives under `n2` alone.
async fn carriers(conn: &turso::Connection, kind: &str, key: &str, carriers: i64) {
    for i in 0..carriers {
        conn.execute(
            "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, ?, ?)",
            (
                Value::Integer(900_000 + i),
                Value::Text(kind.into()),
                Value::Text(key.into()),
            ),
        )
        .await
        .unwrap();
    }
}

fn never() -> bool {
    false
}

const STOPLIST_CAP: usize = 8;

fn nowhere(_done: u64, _detail: &str) {}

async fn run(db: &store::Db, cap: usize) -> store::DuplicateIdentityReport {
    db.duplicate_identity_census(key, n3, STOPLIST_CAP, cap, &never, &nowhere).await.unwrap()
}

#[tokio::test]
async fn a_lone_identity_is_not_a_duplicate() {
    let (db, conn) = open("dupid-lone").await;
    org(&conn, 1, "DE", "vat", "DE136695976", "Siemens AG", 3).await;
    org(&conn, 2, "DE", "vat", "DE811907980", "Bosch GmbH", 2).await;
    let r = run(&db, 100).await;
    assert_eq!(r.rows_walked, 2);
    assert_eq!(r.triples, 2);
    assert_eq!(r.duplicate_groups, 0, "two different identifiers are not a duplicate of anything");
    assert_eq!(r.unkeyed_groups, 0);
    assert!(r.rows.is_empty());
}

#[tokio::test]
async fn a_keyed_duplicate_is_left_to_the_merge_arm() {
    let (db, conn) = open("dupid-keyed").await;
    // FR:national keys, so R2 already sees this pair. Whatever stands there is
    // its denial stack talking, and this census must not double-count it as an
    // unreachable class.
    org(&conn, 1, "FR", "national", "552081317", "Renault SA", 4).await;
    org(&conn, 2, "FR", "national", "552081317", "Renault SA", 1).await;
    let r = run(&db, 100).await;
    assert_eq!(r.duplicate_groups, 1);
    assert_eq!(r.keyed_groups, 1);
    assert_eq!(r.unkeyed_groups, 0);
    assert!(r.verdicts.is_empty(), "a keyed group gets no name verdict — it is not this census's");
    assert!(r.unkeyed_by_scope.is_empty());
}

#[tokio::test]
async fn matching_names_under_one_unkeyed_vat_agree() {
    let (db, conn) = open("dupid-agree").await;
    // The live specimen's shape: one German VAT number, three rows, one name.
    // The spellings differ in the legal-form family only, which is exactly what
    // the N3 key abstracts — two keys here would have read as disagreement.
    org(&conn, 1, "DE", "vat", "DE329214156", "Die Autobahn GmbH des Bundes", 6).await;
    org(&conn, 2, "DE", "vat", "DE329214156", "Die Autobahn Gesellschaft des Bundes", 2).await;
    org(&conn, 3, "DE", "vat", "DE329214156", "Die Autobahn GmbH des Bundes", 1).await;
    let r = run(&db, 100).await;
    assert_eq!(r.duplicate_groups, 1);
    assert_eq!(r.unkeyed_groups, 1);
    assert_eq!(r.unkeyed_by_scope.get("DE:vat"), Some(&1));
    assert_eq!(r.verdicts.get("agree-distinctive"), Some(&1));
    assert_eq!(r.verdicts_by_scope.get("DE:vat/agree-distinctive"), Some(&1));
    let row = &r.rows[0];
    assert_eq!(row.members, 3);
    assert_eq!(row.mentions, 9, "a fold here would move nine mentions, and that is the stake");
    assert_eq!(row.name_keys.len(), 1);
}

#[tokio::test]
async fn two_different_companies_under_one_vat_disagree() {
    let (db, conn) = open("dupid-disagree").await;
    // THE ORGANSCHAFT SHAPE, and the reason this census exists: a fiscal unity
    // files under the parent's VAT number, so legally distinct companies share
    // it. Merging them would destroy two real entities.
    org(&conn, 1, "DE", "vat", "DE811907980", "Robert Bosch GmbH", 5).await;
    org(&conn, 2, "DE", "vat", "DE811907980", "BSH Hausgeraete GmbH", 4).await;
    let r = run(&db, 100).await;
    assert_eq!(r.verdicts.get("disagree"), Some(&1));
    assert_eq!(r.verdicts_by_scope.get("DE:vat/disagree"), Some(&1));
    assert_eq!(r.rows[0].name_keys.len(), 2);
}

#[tokio::test]
async fn a_qualified_name_is_containment_whichever_end_the_qualifier_sits_on() {
    let (db, conn) = open("dupid-contained").await;
    // German publishers write the qualifier on either end, and both forms turn
    // up under one identifier. The first implementation tested a CONTIGUOUS
    // TOKEN WINDOW and this fixture killed it: a rotation is never a contiguous
    // run of its counterpart, so half the class read as outright conflict.
    org(&conn, 1, "DE", "vat", "DE329214156", "Die Autobahn GmbH", 3).await;
    org(&conn, 2, "DE", "vat", "DE329214156", "Die Autobahn GmbH Niederlassung Nordbayern", 2)
        .await;
    org(&conn, 3, "DE", "vat", "DE329214156", "Niederlassung Nordbayern Die Autobahn GmbH", 1)
        .await;
    let r = run(&db, 100).await;
    assert_eq!(r.verdicts.get("contained"), Some(&1), "{:?}", r.rows[0].name_keys);
    assert_eq!(r.verdicts.get("disagree"), None);
}

#[tokio::test]
async fn containment_is_a_third_bucket_and_not_a_licence_to_fold() {
    let (db, conn) = open("dupid-subsidiary").await;
    // THE REASON `contained` IS NOT `agree`. This pair is exactly the shape of
    // an Organschaft: a fiscal unity is a parent plus companies named after the
    // parent with a qualifier, so a subsidiary and a branch office are named
    // identically and nothing in a name separates them. The census must not
    // report either answer for these — it reports the count and stops.
    org(&conn, 1, "DE", "vat", "DE777777777", "Muster Holding GmbH", 4).await;
    org(&conn, 2, "DE", "vat", "DE777777777", "Muster Holding Immobilien GmbH", 2).await;
    let r = run(&db, 100).await;
    assert_eq!(r.verdicts.get("contained"), Some(&1));
    assert_eq!(
        r.verdicts.get("agree-distinctive"),
        None,
        "a qualified subsidiary must never be counted as name agreement"
    );
    assert_eq!(r.verdicts_by_scope.get("DE:vat/contained"), Some(&1));
}

#[tokio::test]
async fn agreement_on_a_generic_name_is_bucketed_apart() {
    let (db, conn) = open("dupid-generic").await;
    // Issue 316's finding, transplanted: a generic name is agreement between
    // two names nobody chose to make unique, so it cannot carry a merge on its
    // own. It is still counted — just not counted as evidence.
    org(&conn, 1, "DE", "vat", "DE111111111", "Stadtverwaltung", 3).await;
    org(&conn, 2, "DE", "vat", "DE111111111", "Stadtverwaltung", 2).await;
    carriers(&conn, "n3", &n3("Stadtverwaltung"), STOPLIST_CAP as i64 + 1).await;
    let r = run(&db, 100).await;
    assert_eq!(r.verdicts.get("agree-generic"), Some(&1));
    assert_eq!(r.verdicts.get("agree-distinctive"), None);
    assert_eq!(r.name_keys_absent, 0, "the key IS in org_match_keys — it is generic, not missing");
}

#[tokio::test]
async fn a_key_the_build_stored_under_n2_is_still_found() {
    let (db, conn) = open("dupid-n2-fallback").await;
    // THE DEFECT THIS NEARLY SHIPPED WITH. `build-org-match-keys` writes an
    // `n3` row only `if k3 != k2`, so a name carrying no legal-form token has
    // its N3 key stored under kind `n2` and nothing under `n3`. A probe on
    // `key_kind = 'n3'` alone would report most of the corpus ABSENT and make
    // the whole agree/generic split unusable — silently, and in the safe-looking
    // direction.
    org(&conn, 1, "DE", "vat", "DE888888888", "Kreisverwaltung Ahrweiler", 2).await;
    org(&conn, 2, "DE", "vat", "DE888888888", "Kreisverwaltung Ahrweiler", 1).await;
    assert_eq!(n3("Kreisverwaltung Ahrweiler"), "kreisverwaltung ahrweiler", "no family token");
    carriers(&conn, "n2", "kreisverwaltung ahrweiler", STOPLIST_CAP as i64 + 1).await;
    let r = run(&db, 100).await;
    assert_eq!(r.verdicts.get("agree-generic"), Some(&1));
    assert_eq!(r.name_keys_absent, 0, "the key IS held — under n2, which the probe must span");
}

#[tokio::test]
async fn a_name_key_the_build_never_wrote_is_reported_not_read_as_distinctive() {
    let (db, conn) = open("dupid-absent").await;
    // The trap this field exists for: the genericness probe reads
    // `org_match_keys`, so on a stale or unrun key-build EVERY agreeing group
    // reads `distinctive` and the census overstates the fold signal. Nothing
    // was inserted into org_match_keys here, so the count has to say so.
    org(&conn, 1, "DE", "vat", "DE222222222", "Kreisverwaltung Ahrweiler", 2).await;
    org(&conn, 2, "DE", "vat", "DE222222222", "Kreisverwaltung Ahrweiler", 1).await;
    let r = run(&db, 100).await;
    assert_eq!(r.verdicts.get("agree-distinctive"), Some(&1));
    assert_eq!(r.name_keys_absent, 1, "an absent key must be visible, not silently distinctive");
}

#[tokio::test]
async fn a_nameless_group_abstains() {
    let (db, conn) = open("dupid-unnamed").await;
    // Issue 257's "name nobody" class. Neither agreement nor conflict, and an
    // abstention is the correct answer — a coin-flip on a merge is not.
    org(&conn, 1, "DE", "vat", "DE333333333", "", 2).await;
    org(&conn, 2, "DE", "vat", "DE333333333", "   ", 1).await;
    let r = run(&db, 100).await;
    assert_eq!(r.verdicts.get("unnamed"), Some(&1));
    assert!(r.rows[0].names.is_empty());
}

#[tokio::test]
async fn the_cap_bounds_the_listing_and_never_the_tally() {
    let (db, conn) = open("dupid-cap").await;
    // THE LESSON THIS CENSUS INHERITS. The issue-326 census computed verdicts
    // only over the carried widest-first slice, and it inverted two published
    // conclusions. `cap` bounds `rows`; every tally is corpus-wide.
    for n in 0..12i64 {
        let ident = format!("DE4{n:08}");
        org(&conn, n * 2 + 1, "DE", "vat", &ident, &format!("Firma {n} GmbH"), 1).await;
        org(&conn, n * 2 + 2, "DE", "vat", &ident, &format!("Firma {n} GmbH"), 1).await;
    }
    let r = run(&db, 4).await;
    assert_eq!(r.unkeyed_groups, 12);
    assert!(r.truncated);
    assert_eq!(r.rows.len(), 4, "the listing is capped");
    let tallied: u64 = r.verdicts.values().sum();
    assert_eq!(tallied, 12, "the tally is not — all twelve got a verdict");
    assert_eq!(r.verdicts.get("agree-distinctive"), Some(&12));
}

#[tokio::test]
async fn the_scope_cut_separates_the_countries_and_the_kinds() {
    let (db, conn) = open("dupid-scope").await;
    // The output that chooses the NEXT cross-walk arm. Reporting one lump
    // "unkeyed duplicates" number would have made this census about Germany
    // because Germany is what came up in conversation.
    org(&conn, 1, "DE", "vat", "DE555555555", "Alpha GmbH", 1).await;
    org(&conn, 2, "DE", "vat", "DE555555555", "Alpha GmbH", 1).await;
    org(&conn, 3, "DE", "national", "HRB64128", "Beta GmbH", 1).await;
    org(&conn, 4, "DE", "national", "HRB64128", "Gamma GmbH", 1).await;
    org(&conn, 5, "AT", "vat", "ATU12345678", "Delta GmbH", 1).await;
    org(&conn, 6, "AT", "vat", "ATU12345678", "Delta GmbH", 1).await;
    let r = run(&db, 100).await;
    assert_eq!(r.unkeyed_groups, 3);
    assert_eq!(r.unkeyed_by_scope.get("DE:vat"), Some(&1));
    assert_eq!(r.unkeyed_by_scope.get("DE:national"), Some(&1));
    assert_eq!(r.unkeyed_by_scope.get("AT:vat"), Some(&1));
    assert_eq!(r.verdicts_by_scope.get("DE:national/disagree"), Some(&1));
    assert_eq!(r.verdicts_by_scope.get("AT:vat/agree-distinctive"), Some(&1));
    assert_eq!(r.group_sizes, vec![2, 2, 2]);
}

#[tokio::test]
async fn a_stop_request_returns_stopped_and_no_half_report() {
    let (db, conn) = open("dupid-stop").await;
    org(&conn, 1, "DE", "vat", "DE666666666", "Epsilon GmbH", 1).await;
    org(&conn, 2, "DE", "vat", "DE666666666", "Epsilon GmbH", 1).await;
    fn always() -> bool {
        true
    }
    let r =
        db.duplicate_identity_census(key, n3, STOPLIST_CAP, 100, &always, &nowhere).await.unwrap();
    assert!(r.stopped);
    // Issue 252's honest cancel: a stopped run reports nothing rather than a
    // partial tally a reader would take for a corpus-wide one.
    assert_eq!(r.unkeyed_groups, 0);
    assert!(r.verdicts.is_empty());
}
