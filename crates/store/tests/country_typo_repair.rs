//! Issue 326 step 2: the survivor rule.
//!
//! What these pin is the surrounding judgement, not the arithmetic: which rows
//! qualify, which are deliberately left alone, that weight does not vote, and
//! that the duplicate identities the repair creates are created ON PURPOSE for
//! the R2 merge arm to fold.

use store::turso::{self, Value};

// Stand-ins, injected as the job injects the real ones (`ingest` depends on
// `store`, so importing them here would be a cycle).
fn anchors(value: &str) -> Vec<(&'static str, String)> {
    match value.len() {
        // Eight digits name Slovakia alone.
        8 if value.chars().all(|c| c.is_ascii_digit()) => vec![("SK:ico", value.to_owned())],
        // Nine name Bulgaria alone.
        9 if value.chars().all(|c| c.is_ascii_digit()) => vec![("BG:eik", value.to_owned())],
        _ => Vec::new(),
    }
}

fn vocabulary(value: &str) -> Vec<&'static str> {
    match value.len() {
        8 => vec!["SK:ico", "SG:none"],
        9 => vec!["BG:eik"],
        _ => Vec::new(),
    }
}

fn one_letter(a: &str, b: &str) -> bool {
    a.chars().count() == 2
        && b.chars().count() == 2
        && a != b
        && a.chars().zip(b.chars()).filter(|(x, y)| x != y).count() == 1
}

fn footprint(name: &str) -> bool {
    let n = name.to_lowercase();
    ["embassy", "ambassade", "regeringskansliet"].iter().any(|m| n.contains(m))
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

async fn org(conn: &turso::Connection, id: i64, cc: &str, ident: &str, name: &str, mentions: i64) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
         VALUES (?, ?, 'national', ?, ?, 0, 0)",
        (
            Value::Integer(id),
            Value::Text(cc.into()),
            Value::Text(ident.into()),
            Value::Text(name.into()),
        ),
    )
    .await
    .unwrap();
    for i in 0..mentions {
        let n = id * 1000 + i;
        conn.execute(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                                  member_path, ingested_at, parse_state, projected)
             VALUES (?, 'ted', 'pub-' || ?, 'h' || ?, 'eforms', 1, 'm', 0, 'parsed', 1)",
            (Value::Integer(n), Value::Integer(n), Value::Integer(n)),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
             VALUES (?, 'ORG-' || ?, ?, ?, ?, NULL)",
            (
                Value::Integer(n),
                Value::Integer(n),
                Value::Integer(id),
                Value::Text(name.into()),
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

async fn plan(db: &store::Db) -> store::CountryTypoRepairReport {
    db.repair_country_typos(anchors, vocabulary, one_letter, footprint, true, None, &never)
        .await
        .unwrap()
}

async fn apply(db: &store::Db, expect: u64) -> store::CountryTypoRepairReport {
    db.repair_country_typos(anchors, vocabulary, one_letter, footprint, false, Some(expect), &never)
        .await
        .unwrap()
}

async fn country_of(conn: &turso::Connection, id: i64) -> String {
    let mut rows = conn
        .query("SELECT country FROM organizations WHERE id = ?", (Value::Integer(id),))
        .await
        .unwrap();
    match rows.next().await.unwrap().unwrap().get_value(0).unwrap() {
        Value::Text(s) => s,
        other => panic!("{other:?}"),
    }
}

/// THE ANCHOR DECIDES THE SURVIVOR, NOT THE WEIGHT — with realistic weights,
/// because the first draft of this test used invented ones.
///
/// It gave the wrong side 90 mentions to make the point vividly, and the weight
/// veto then correctly refused the move, failing the test. The fabrication was
/// the problem: measured over the first prod plan, 256 of 343 moved rows carry
/// exactly ONE mention and p95 is 4. A mis-countried row is light, because a
/// transcription slip happens once.
///
/// **The limitation this leaves is real and deliberate.** A heavily-published
/// mis-countried row — one that really does appear under the wrong country
/// dozens of times — is NOT auto-repaired; it is refused as
/// `refused_row_has_standing` and belongs in the review queue. That is the
/// correct trade for a published field: the same weight that would make such a
/// row worth fixing is what makes it indistinguishable from a real registration.
#[tokio::test]
async fn the_anchor_decides_the_survivor_not_the_weight() {
    let (db, conn) = open("test-typo-weight").await;
    // SG is the heavier side, and under the veto threshold — so the anchor,
    // which names SK, still wins.
    org(&conn, 1, "SG", "41734602", "Nejaka Firma", 4).await;
    org(&conn, 2, "SK", "41734602", "Nejaka Firma", 1).await;
    db.build_organization_indexes().await.unwrap();

    let p = plan(&db).await;
    assert_eq!(p.decisive, 1);
    assert_eq!(p.rows, 1);
    assert_eq!((p.moves[0].from.as_str(), p.moves[0].to.as_str()), ("SG", "SK"));
    assert_eq!(p.moves[0].mentions, 4, "the heavier side is the one that moves");
    assert_eq!(p.applied, 0, "a dry run writes nothing");

    let w = apply(&db, 1).await;
    assert_eq!(w.applied, 1);
    assert_eq!(country_of(&conn, 1).await, "SK");
    assert_eq!(country_of(&conn, 2).await, "SK");

    // Both fields of the identity now match, which is the duplicate the R2
    // merge arm exists to fold — created on purpose.
    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM organizations WHERE country='SK' AND identifier='41734602'",
            (),
        )
        .await
        .unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get_value(0).unwrap(), Value::Integer(2));
}

/// A row whose code is NOT one letter from the survivor shares the identifier
/// and nothing else. "The number checksums somewhere else" is not a reason to
/// rewrite a published country, so those are counted and left standing.
#[tokio::test]
async fn only_one_letter_neighbours_move() {
    let (db, conn) = open("test-typo-neighbours").await;
    // BG survives. BI and BW are one letter out; GA, VA and VU are not.
    org(&conn, 1, "BG", "831496285", "Petrol AD", 400).await;
    org(&conn, 2, "BI", "831496285", "Petrol AD", 1).await;
    org(&conn, 3, "BW", "831496285", "Petrol AD", 1).await;
    org(&conn, 4, "GA", "831496285", "Petrol AD", 1).await;
    org(&conn, 5, "VA", "831496285", "Petrol AD", 1).await;
    org(&conn, 6, "VU", "831496285", "Petrol AD", 1).await;
    db.build_organization_indexes().await.unwrap();

    let p = plan(&db).await;
    assert_eq!(p.rows, 2, "BI and BW only: {:?}", p.moves);
    assert_eq!(p.left_unmoved, 3, "GA, VA and VU are counted, not guessed at");
    let moved: Vec<&str> = p.moves.iter().map(|m| m.from.as_str()).collect();
    assert!(moved.contains(&"BI") && moved.contains(&"BW"), "{moved:?}");

    let w = apply(&db, 2).await;
    assert_eq!(w.applied, 2);
    assert_eq!(country_of(&conn, 2).await, "BG");
    assert_eq!(country_of(&conn, 3).await, "BG");
    assert_eq!(country_of(&conn, 4).await, "GA", "left exactly as it stood");
    assert_eq!(country_of(&conn, 5).await, "VA");
    assert_eq!(country_of(&conn, 6).await, "VU");
}

/// The census's exclusions carry through unchanged: a footprint cluster, a
/// too-short identifier, and a cluster with no one-letter pair are all outside
/// this repair because they never reach the `anchor-names-one` verdict.
#[tokio::test]
async fn the_census_exclusions_are_the_repairs_exclusions() {
    let (db, conn) = open("test-typo-excl").await;
    // An embassy: one entity, one register number, filed from everywhere.
    org(&conn, 1, "SK", "41734602", "Embassy of Slovakia", 100).await;
    org(&conn, 2, "SG", "41734602", "Embassy of Slovakia", 2).await;
    // Too short.
    org(&conn, 3, "SK", "4173", "Mala Firma", 10).await;
    org(&conn, 4, "SG", "4173", "Mala Firma", 2).await;
    // No one-letter pair anywhere in the cluster.
    org(&conn, 5, "SK", "417346021", "Ina Firma", 10).await;
    org(&conn, 6, "IT", "417346021", "Ina Firma", 2).await;
    db.build_organization_indexes().await.unwrap();

    let p = plan(&db).await;
    assert_eq!(p.decisive, 0, "none of the three is decisive: {:?}", p.moves);
    assert_eq!(p.rows, 0);

    let w = db
        .repair_country_typos(anchors, vocabulary, one_letter, footprint, false, None, &never)
        .await
        .unwrap();
    assert_eq!(w.applied, 0);
    for (id, cc) in [(2i64, "SG"), (4, "SG"), (6, "IT")] {
        assert_eq!(country_of(&conn, id).await, cc, "org {id} untouched");
    }
}

/// The parity gate: a wet run whose plan drifted from the reviewed one aborts
/// rather than applying a different plan than the one that was cleared.
#[tokio::test]
async fn a_drifted_plan_aborts_before_writing() {
    let (db, conn) = open("test-typo-parity").await;
    for n in 0..12i64 {
        // Fixed NINE digits: `format!("83149628{n}")` was the first draft and
        // silently produced ten-digit values for n >= 10, which the stand-in
        // anchor does not name, so two clusters quietly fell out of the plan.
        let ident = format!("8314{n:05}");
        org(&conn, n * 2 + 1, "BG", &ident, "Petrol AD", 30).await;
        org(&conn, n * 2 + 2, "BI", &ident, "Petrol AD", 1).await;
    }
    db.build_organization_indexes().await.unwrap();

    let p = plan(&db).await;
    assert_eq!(p.rows, 12);

    let err = db
        .repair_country_typos(anchors, vocabulary, one_letter, footprint, false, Some(3), &never)
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("ABORTED") && msg.contains("12"), "{msg}");
    assert_eq!(country_of(&conn, 2).await, "BI", "nothing was written");
}

/// A cancelled run reports nothing rather than a partial plan a reader would
/// take for a whole one.
#[tokio::test]
async fn a_cancelled_repair_reports_nothing() {
    let (db, conn) = open("test-typo-cancel").await;
    org(&conn, 1, "BG", "831496285", "Petrol AD", 30).await;
    org(&conn, 2, "BI", "831496285", "Petrol AD", 1).await;
    db.build_organization_indexes().await.unwrap();

    let always = || true;
    let r = db
        .repair_country_typos(anchors, vocabulary, one_letter, footprint, true, None, &always)
        .await
        .unwrap();
    assert!(r.stopped);
    assert_eq!(r.rows, 0);
    assert!(r.moves.is_empty());
}

/// Idempotence: once the rows are moved the cluster has one code, so it is no
/// longer a cluster and a second run plans nothing.
#[tokio::test]
async fn a_second_run_finds_nothing_to_do() {
    let (db, conn) = open("test-typo-idem").await;
    org(&conn, 1, "BG", "831496285", "Petrol AD", 40).await;
    org(&conn, 2, "BI", "831496285", "Petrol AD", 2).await;
    db.build_organization_indexes().await.unwrap();

    assert_eq!(apply(&db, 1).await.applied, 1);
    let again = plan(&db).await;
    assert_eq!(again.clusters_considered, 0, "one code left — not a cluster");
    assert_eq!(again.rows, 0);
}

/// THE PRECISION SPLIT, which the first prod dry run is what surfaced.
///
/// A move that abandons a country the vocabulary actually TESTED carries real
/// evidence against it. A move that abandons a country no scheme covers at this
/// shape rests on the survivor's anchor alone — issue 314's `country_probed`
/// gap. Both halves contain true positives, so this is a split for a reviewer
/// and NOT a filter: `LV -> LT` on a `UAB` company is never-asked and plainly
/// right, while the two false positives the first plan turned up
/// (`SI -> SE` on a Slovenian sole proprietor passing the Swedish Luhn, and
/// `IE -> IT` on an insurer registered in three countries) are also never-asked.
#[tokio::test]
async fn the_plan_separates_a_refusal_from_a_missing_scheme() {
    let (db, conn) = open("test-typo-asked").await;
    // Eight digits: the stand-in vocabulary covers SK and SG, so moving away
    // from SG means SG was tested and refused.
    org(&conn, 1, "SK", "41734602", "Nejaka Firma", 30).await;
    org(&conn, 2, "SG", "41734602", "Nejaka Firma", 2).await;
    // Nine digits: the vocabulary covers BG only, so moving away from BI rests
    // on Bulgaria's anchor alone — nobody asked about Burundi.
    org(&conn, 3, "BG", "831496285", "Petrol AD", 40).await;
    org(&conn, 4, "BI", "831496285", "Petrol AD", 1).await;
    db.build_organization_indexes().await.unwrap();

    let p = plan(&db).await;
    assert_eq!(p.rows, 2);
    assert_eq!(p.from_asked_and_refused, 1);
    assert_eq!(p.from_never_asked, 1);
    let by = |from: &str| -> bool {
        p.moves.iter().find(|m| m.from == from).expect(from).from_asked_and_refused
    };
    assert!(by("SG"), "SG is in the 8-digit vocabulary — tested and refused");
    assert!(!by("BI"), "no Burundian scheme exists at 9 digits — silence, not refusal");

    // And BOTH still apply: the split informs review, it does not gate the write.
    let w = apply(&db, 2).await;
    assert_eq!(w.applied, 2);
}

/// THE WEIGHT VETO, and it replaced a blunter rule that this test's own
/// counterpart exposed.
///
/// A transcription slip appears once or twice; a real registration accumulates
/// publications. Both false positives the first dry plan turned up sit far out
/// in the tail — "XL Insurance Company SE" at 65 mentions, the Slovenian
/// "Gorup … S.P." at 17 — against a distribution whose p95 is 4.
///
/// The first attempt refused any cluster holding a code neither the survivor nor
/// one letter from it. It caught XL Insurance and also refused
/// `Софарма Трейдинг АД`: one Bulgarian EIK under `BG` plus five junk codes with
/// a single mention each, which is precisely the corruption this repair exists
/// for. Cluster shape cannot separate a multi-country registration from one real
/// code plus wide spray. The moved row's own weight can.
#[tokio::test]
async fn a_row_with_real_standing_is_not_moved() {
    let (db, conn) = open("test-typo-standing").await;
    // The XL Insurance shape: the row being moved has real activity of its own.
    org(&conn, 1, "SK", "41734602", "XL Insurance Company SE", 40).await;
    org(&conn, 2, "SG", "41734602", "XL Insurance Company SE", 65).await;
    // …and a genuine slip alongside it: one mention under a neighbour code.
    org(&conn, 3, "SK", "41734603", "Nejaka Firma", 30).await;
    org(&conn, 4, "SG", "41734603", "Nejaka Firma", 1).await;
    db.build_organization_indexes().await.unwrap();

    let p = plan(&db).await;
    assert_eq!(p.decisive, 2, "both clusters are decidable");
    assert_eq!(p.refused_row_has_standing, 1);
    assert_eq!(p.rows, 1, "only the one-mention slip moves: {:?}", p.moves);
    assert_eq!(p.moves[0].identifier, "41734603");

    let w = apply(&db, 1).await;
    assert_eq!(w.applied, 1);
    assert_eq!(country_of(&conn, 2).await, "SG", "the 65-mention row keeps its country");
    assert_eq!(country_of(&conn, 4).await, "SK");
}

/// TIGHTENING (b): a survivor named only by a bare Luhn/// TIGHTENING (b): a survivor named only by a bare Luhn is not evidence about a
/// country. Luhn is a transcription checksum shared by `SE:orgnr` and
/// `FR:siren`, and it is what moved "Gorup – Audio Stojan Gorup S.P." — a
/// Slovenian sole proprietor — into Sweden.
///
/// The scheme has to be carried on the move for this to be expressible at all:
/// a length proxy fails because 261 of the first plan's 343 moves were nine
/// digits, where the namer is `BG:eik` or `LT:kodas` rather than `FR:siren`.
#[tokio::test]
async fn a_survivor_named_only_by_luhn_is_refused() {
    // A stand-in whose 10-digit arm is SE:orgnr — the Luhn-family case — and
    // whose 9-digit arm is LT:kodas, a national mod-11.
    fn luhnish(value: &str) -> Vec<(&'static str, String)> {
        match value.len() {
            10 => vec![("SE:orgnr", value.to_owned())],
            9 => vec![("LT:kodas", value.to_owned())],
            _ => Vec::new(),
        }
    }
    fn vocab(value: &str) -> Vec<&'static str> {
        match value.len() {
            10 => vec!["SE:orgnr"],
            9 => vec!["LT:kodas"],
            _ => Vec::new(),
        }
    }
    let (db, conn) = open("test-typo-luhn").await;
    // The Gorup shape: a Slovenian S.P. whose number closes the Swedish Luhn.
    org(&conn, 1, "SE", "5565140000", "Gorup - Audio Stojan Gorup S.P.", 3).await;
    org(&conn, 2, "SI", "5565140000", "Gorup - Audio Stojan Gorup S.P.", 17).await;
    // And a genuine Lithuanian company under Latvia, named by a NATIONAL scheme.
    org(&conn, 3, "LT", "302591590", "UAB \"AE Medical\"", 22).await;
    org(&conn, 4, "LV", "302591590", "UAB \"AE Medical\"", 4).await;
    db.build_organization_indexes().await.unwrap();

    let p = db
        .repair_country_typos(luhnish, vocab, one_letter, footprint, true, None, &never)
        .await
        .unwrap();
    assert_eq!(p.decisive, 2, "both clusters look decidable to the census");
    assert_eq!(p.refused_luhn_only, 1, "the Swedish Luhn one is refused");
    assert_eq!(p.rows, 1, "only the Lithuanian move survives: {:?}", p.moves);
    assert_eq!((p.moves[0].from.as_str(), p.moves[0].to.as_str()), ("LV", "LT"));
    assert_eq!(
        p.moves[0].by_scheme, "LT:kodas",
        "and the move records WHICH scheme decided it"
    );

    // The Slovenian row stands exactly as it was.
    let w = db
        .repair_country_typos(luhnish, vocab, one_letter, footprint, false, Some(1), &never)
        .await
        .unwrap();
    assert_eq!(w.applied, 1);
    assert_eq!(country_of(&conn, 2).await, "SI", "the Slovenian S.P. is untouched");
    assert_eq!(country_of(&conn, 4).await, "LT");
}
