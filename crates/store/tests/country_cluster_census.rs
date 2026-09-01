//! Issue 326 step 1 re-cut: the same-identifier country class, grouped by
//! IDENTIFIER instead of by pair.
//!
//! The pair census measured the class and then showed its own unit to be wrong
//! in three ways. These tests pin the three corrections, because each one is a
//! premise a repair built on the pair view would have silently violated:
//! the true code lives in the CLUSTER and not in any one pair; BOTH sides of a
//! pair can be wrong; and a legitimate multi-country class (embassies,
//! development agencies) is indistinguishable by checksum or by mention spread
//! and separable only by name.

use store::turso::{self, Value};

// STAND-INS, injected exactly as the job injects the real ones. `ingest` depends
// on `store`, so importing the real predicates here would be a dependency cycle
// — and it would also be the wrong test: this method takes them as parameters,
// so what belongs here is the surrounding judgement, not their internals. The
// real `one_letter_apart` and `is_operational_footprint` are unit-tested in
// `crates/ingest/src/countries.rs`.
fn one_letter(a: &str, b: &str) -> bool {
    a.chars().count() == 2
        && b.chars().count() == 2
        && a != b
        && a.chars().zip(b.chars()).filter(|(x, y)| x != y).count() == 1
}

fn footprint(name: &str) -> bool {
    let n = name.to_lowercase();
    ["embassy", "ambassade", "regeringskansliet", "agence belge de développement"]
        .iter()
        .any(|m| n.contains(m))
}

fn anchors(value: &str) -> Vec<(&'static str, String)> {
    // A stand-in with the real one's shape. Eight digits validate as BOTH a
    // Slovak and a Slovenian register number — genuinely ambiguous arithmetic,
    // and `SK`/`SI` are one letter apart, which is the combination that makes
    // `anchor-names-several` reachable. Nine digits name Bulgaria alone.
    match value.len() {
        8 if value.chars().all(|c| c.is_ascii_digit()) => {
            vec![("SK:ico", value.to_owned()), ("SI:maticna", value.to_owned())]
        }
        9 if value.chars().all(|c| c.is_ascii_digit()) => {
            vec![("BG:eik", value.to_owned())]
        }
        _ => Vec::new(),
    }
}

fn vocabulary(value: &str) -> Vec<&'static str> {
    match value.len() {
        8 => vec!["SK:ico", "SI:maticna", "CZ:ico"],
        9 => vec!["BG:eik", "GR:afm"],
        _ => Vec::new(),
    }
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

/// One org row plus `mentions` mentions of it.
async fn org(
    conn: &turso::Connection,
    id: i64,
    cc: &str,
    ident: &str,
    name: &str,
    mentions: i64,
) {
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

fn never() -> bool {
    false
}

async fn run(db: &store::Db) -> store::CountryClusterReport {
    db.country_cluster_census(
        anchors,
        vocabulary,
        one_letter,
        footprint,
        400,
        &never,
    )
    .await
    .unwrap()
}

/// THE CORRECTION THE RE-CUT EXISTS FOR: the true code is in the cluster, and
/// the pair view cannot see it. A Bulgarian EIK under `BG BW VA VE VG VU` is
/// `BG` plus spray; reported pairwise that becomes edges like `VA/VU`, which do
/// not contain the answer at all.
#[tokio::test]
async fn a_cluster_is_reported_once_with_its_whole_country_set() {
    let (db, conn) = open("test-cluster-set").await;
    let eik = "831496285";
    org(&conn, 1, "BG", eik, "Petrol AD", 400).await;
    for (i, cc) in ["BW", "VA", "VE", "VG", "VU"].iter().enumerate() {
        org(&conn, 2 + i as i64, cc, eik, "Petrol AD", 1).await;
    }
    db.build_organization_indexes().await.unwrap();

    let r = run(&db).await;
    assert_eq!(r.clusters, 1, "one identifier, not fifteen pairs");
    let c = &r.rows[0];
    assert_eq!(c.identifier, eik);
    assert_eq!(c.codes.len(), 6, "the whole set, in one row: {:?}", c.codes);
    assert!(c.codes.contains(&"BG".to_owned()));
    // The evidence is computed once per identifier rather than once per pair.
    assert_eq!(c.named, vec!["BG".to_owned()], "only BG is both in the cluster and named");
    assert_eq!(c.verdict, "anchor-names-one");
    // The heavy code leads the mention list.
    assert_eq!(c.mentions[0].0, "BG");
    assert_eq!(c.mentions[0].1, 400);
}

/// The sharper filter. `one_letter_pair` is satisfied by `VA`/`VE` — spray one
/// letter from spray — which says nothing about whether the code carrying the
/// entity was the one mistyped. `heavy_one_letter` asks the question that
/// matters.
#[tokio::test]
async fn spray_one_letter_from_spray_does_not_implicate_the_heavy_code() {
    let (db, conn) = open("test-cluster-heavy").await;
    // PL is not one letter from either IT or IS; IT/IS are one letter apart.
    org(&conn, 1, "PL", "5261040828", "Krajowa Izba", 300).await;
    org(&conn, 2, "IT", "5261040828", "Krajowa Izba", 1).await;
    org(&conn, 3, "IS", "5261040828", "Krajowa Izba", 1).await;
    db.build_organization_indexes().await.unwrap();

    let r = run(&db).await;
    let c = &r.rows[0];
    assert!(c.one_letter_pair, "IT/IS are one letter apart");
    assert!(
        !c.heavy_one_letter,
        "but PL is one letter from neither, so the heavy code is not implicated"
    );
    assert_eq!(r.with_one_letter_pair, 1);
    assert_eq!(r.with_heavy_one_letter, 0);
}

/// THE CLASS A SAME-IDENTIFIER RULE WOULD DESTROY. An embassy is one legal
/// entity with one register number, filing from everywhere it operates. Neither
/// the checksum nor the mention spread separates it — the spread shows the SAME
/// asymmetry the real typos do — so the name has to.
#[tokio::test]
async fn an_embassy_is_excluded_and_never_corrected() {
    let (db, conn) = open("test-cluster-embassy").await;
    let reg = "2021003831";
    org(&conn, 1, "SE", reg, "Regeringskansliet", 250).await;
    // The same 2-against-249 asymmetry a real typo shows.
    for (i, cc) in ["KE", "MD", "MZ", "UA", "UG"].iter().enumerate() {
        org(&conn, 2 + i as i64, cc, reg, "Embassy of Sweden", 2).await;
    }
    db.build_organization_indexes().await.unwrap();

    let r = run(&db).await;
    let c = &r.rows[0];
    assert!(c.footprint, "the NAME is the only discriminator that works here");
    assert_eq!(c.verdict, "footprint-excluded");
    assert_eq!(r.footprint, 1);
    // And it is excluded on its own merits, not because the evidence was thin:
    // the mention spread would have pointed at SE just as a real typo would.
    assert_eq!(c.mentions[0].0, "SE");
}

/// A short identifier collides by arithmetic rather than by identity. `9948` was
/// shared by a US and a Spanish company in the pair census's carried rows.
#[tokio::test]
async fn a_short_identifier_is_floored_out() {
    let (db, conn) = open("test-cluster-short").await;
    org(&conn, 1, "US", "9948", "Techno-Sciences, LLC", 5).await;
    org(&conn, 2, "ES", "9948", "VICENTE TARREGA PEREZ", 3).await;
    db.build_organization_indexes().await.unwrap();

    let r = run(&db).await;
    assert_eq!(r.too_short, 1);
    assert_eq!(r.rows[0].verdict, "too-short");
}

/// The abstain arms, and they are the point. `nobody-asked` is issue 314's
/// vocabulary gap rather than a hard case — the census must say which it is
/// rather than reporting one undifferentiated "unknown".
#[tokio::test]
async fn the_census_abstains_and_says_which_kind_of_abstention_it_is() {
    let (db, conn) = open("test-cluster-abstain").await;
    // Letters: the stand-in vocabulary has no arm at all for this shape.
    org(&conn, 1, "SK", "ABCDEFGHIJ", "Nejaka Firma", 40).await;
    org(&conn, 2, "SG", "ABCDEFGHIJ", "Nejaka Firma", 3).await;
    // Eight digits: the anchor names BOTH SK and SI, and they ARE one letter
    // apart — so the filter fires and the evidence still cannot choose.
    org(&conn, 3, "SK", "41734602", "Nejaka Firma", 40).await;
    org(&conn, 4, "SI", "41734602", "Nejaka Firma", 3).await;
    // Same identifier, codes NOT one letter apart: real, but no corruption
    // filter fires, so there is no reason to call it a typo.
    org(&conn, 5, "PL", "PL99887766", "Jakas Firma", 9).await;
    org(&conn, 6, "IT", "PL99887766", "Jakas Firma", 4).await;
    db.build_organization_indexes().await.unwrap();

    let r = run(&db).await;
    assert_eq!(r.clusters, 3);
    let v = |id: &str| -> &'static str {
        r.rows.iter().find(|c| c.identifier == id).expect(id).verdict
    };
    assert_eq!(v("ABCDEFGHIJ"), "nobody-asked", "a missing scheme, not a hard case");
    assert_eq!(v("41734602"), "anchor-names-several");
    assert_eq!(v("PL99887766"), "no-one-letter-pair");
    assert_eq!(r.verdicts.get("nobody-asked"), Some(&1));
}

/// An identifier under ONE country is not a cluster, and the walk must not
/// report it. The corpus is overwhelmingly this case, so a census that leaked
/// them would be unreadable.
#[tokio::test]
async fn a_single_country_identifier_is_not_a_cluster() {
    let (db, conn) = open("test-cluster-single").await;
    org(&conn, 1, "DE", "DE136695976", "Ein Betrieb", 12).await;
    org(&conn, 2, "DE", "DE136695976", "Ein Betrieb GmbH", 3).await; // same code twice
    org(&conn, 3, "FR", "FR12345678901", "Une Societe", 4).await;
    db.build_organization_indexes().await.unwrap();

    let r = run(&db).await;
    assert_eq!(r.identifiers, 2);
    assert_eq!(r.clusters, 0, "two rows under one code is not two countries");
    assert!(r.rows.is_empty());
}

/// The majority share is REPORTED, never acted on. The pair census found it
/// inverting — `BT` heavy over `BG` light, three times — so the distribution is
/// carried for the next unit to pick a threshold from, and no verdict reads it.
#[tokio::test]
async fn the_majority_share_is_recorded_but_decides_nothing() {
    let (db, conn) = open("test-cluster-share").await;
    // A cluster where the heavy side is the WRONG one: the 8-digit value is a
    // Slovak register number, but SG carries more mentions.
    org(&conn, 1, "SG", "41734602", "Nejaka Firma", 90).await;
    org(&conn, 2, "SK", "41734602", "Nejaka Firma", 10).await;
    db.build_organization_indexes().await.unwrap();

    let r = run(&db).await;
    let c = &r.rows[0];
    assert_eq!(c.mentions[0].0, "SG", "the heavier side");
    assert_eq!(
        c.verdict, "anchor-names-one",
        "and the verdict follows the ANCHOR, which names SK — not the weight"
    );
    assert_eq!(c.named, vec!["SK".to_owned()]);
    assert_eq!(r.majority_share, vec![90], "the share is recorded for the next unit");
}

/// Cancel is honoured, and a stopped run reports nothing rather than a partial
/// census a reader would take for a whole one.
#[tokio::test]
async fn a_cancelled_census_reports_nothing() {
    let (db, conn) = open("test-cluster-cancel").await;
    org(&conn, 1, "BG", "831496285", "Petrol AD", 5).await;
    org(&conn, 2, "BW", "831496285", "Petrol AD", 1).await;
    db.build_organization_indexes().await.unwrap();

    let always = || true;
    let r = db
        .country_cluster_census(
            anchors,
            vocabulary,
            one_letter,
            footprint,
            400,
            &always,
        )
        .await
        .unwrap();
    assert!(r.stopped);
    assert_eq!(r.clusters, 0);
    assert!(r.rows.is_empty());
}

/// THE CAP BOUNDS WHAT IS REPORTED, NOT WHAT IS COUNTED — and the first draft
/// got this wrong in the way that mattered most.
///
/// It computed verdicts only for the carried rows, which are sorted
/// widest-first. So the single number this census exists to produce — how much
/// of the class is decidable — was being read off the most code-spread tail
/// instead of the corpus. Same split the issue-325 repair got right: `rows`
/// counts everything, the carried list is capped.
#[tokio::test]
async fn the_cap_limits_the_carried_rows_but_never_the_tally() {
    let (db, conn) = open("test-cluster-cap").await;
    let mut id = 1i64;
    // Twelve clusters, all identical in shape, so nothing about WHICH ones are
    // carried can change the tally.
    for n in 0..12 {
        let ident = format!("4173460{n:03}"); // ten chars, over the floor
        org(&conn, id, "SK", &ident, "Nejaka Firma", 30).await;
        org(&conn, id + 1, "SG", &ident, "Nejaka Firma", 2).await;
        id += 2;
    }
    db.build_organization_indexes().await.unwrap();

    let capped = db
        .country_cluster_census(anchors, vocabulary, one_letter, footprint, 3, &never)
        .await
        .unwrap();
    assert_eq!(capped.clusters, 12);
    assert_eq!(capped.rows.len(), 3, "three carried");
    assert!(capped.truncated);
    assert_eq!(
        capped.verdicts.values().sum::<u64>(),
        12,
        "but all twelve counted — the tally is corpus-wide or it is worthless"
    );
    assert_eq!(capped.majority_share.len(), 12, "and so is the share distribution");
    assert_eq!(capped.with_heavy_one_letter, 12, "and the heavy-code filter");

    // An uncapped run agrees with it on every count, differing only in what it
    // carries.
    let full = db
        .country_cluster_census(anchors, vocabulary, one_letter, footprint, 400, &never)
        .await
        .unwrap();
    assert!(!full.truncated);
    assert_eq!(full.rows.len(), 12);
    assert_eq!(full.verdicts, capped.verdicts);
    assert_eq!(full.with_heavy_one_letter, capped.with_heavy_one_letter);
    assert_eq!(full.majority_share, capped.majority_share);
}

/// Two rows under the SAME country code is an ordinary duplicate — the R2 merge
/// arm's business — and must not be read as a cluster. Pass 1 collects one entry
/// per ROW now (it carries the org id), so the distinct-country test has to
/// dedupe rather than count entries.
#[tokio::test]
async fn two_rows_under_one_code_is_a_duplicate_not_a_cluster() {
    let (db, conn) = open("test-cluster-dupe").await;
    org(&conn, 1, "SK", "41734602", "Nejaka Firma", 10).await;
    org(&conn, 2, "SK", "41734602", "Nejaka Firma s.r.o.", 4).await;
    org(&conn, 3, "SK", "41734602", "NEJAKA FIRMA", 1).await;
    db.build_organization_indexes().await.unwrap();

    let r = run(&db).await;
    assert_eq!(r.identifiers, 1);
    assert_eq!(r.clusters, 0, "three rows, one country — not this census's problem");

    // And when a second country does appear, all three rows' mentions count
    // toward the Slovak side rather than only the first row's.
    org(&conn, 4, "SG", "41734602", "Nejaka Firma", 2).await;
    let r = run(&db).await;
    assert_eq!(r.clusters, 1);
    let c = &r.rows[0];
    assert_eq!(c.mentions[0], ("SK".to_owned(), 15), "10 + 4 + 1, not 10");
    assert_eq!(c.names.len(), 3, "all three spellings are carried");
}

/// The per-country cut of the `nobody-asked` bucket, which after the
/// corpus-wide run is the census's most load-bearing output: 80% of candidates
/// fail only for want of a checksum arm, and this names which arms to write.
///
/// EVERY code in the cluster is counted, not just the heavy one — an arm for
/// either side of an `SK`/`SG` cluster makes it decidable — and only clusters
/// whose verdict is actually `nobody-asked` contribute, so a country that is
/// merely present alongside a decided cluster does not inflate its case.
#[tokio::test]
async fn the_nobody_asked_bucket_is_cut_by_country() {
    let (db, conn) = open("test-cluster-bycountry").await;
    // Two clusters nobody was asked about (letters: the stand-in has no arm).
    org(&conn, 1, "SK", "ABCDEFGHIJ", "Nejaka Firma", 30).await;
    org(&conn, 2, "SG", "ABCDEFGHIJ", "Nejaka Firma", 2).await;
    org(&conn, 3, "SK", "KLMNOPQRST", "Ina Firma", 20).await;
    org(&conn, 4, "SO", "KLMNOPQRST", "Ina Firma", 1).await;
    // And one the anchor DECIDES, whose countries must not be counted here.
    org(&conn, 5, "SK", "41734602", "Tretia Firma", 40).await;
    org(&conn, 6, "SG", "41734602", "Tretia Firma", 3).await;
    db.build_organization_indexes().await.unwrap();

    let r = run(&db).await;
    assert_eq!(r.verdicts.get("nobody-asked"), Some(&2));
    let by = &r.nobody_asked_by_country;
    assert_eq!(by.get("SK"), Some(&2), "in both undecidable clusters");
    assert_eq!(by.get("SG"), Some(&1), "once — the decided cluster does not count");
    assert_eq!(by.get("SO"), Some(&1));
    assert_eq!(by.len(), 3, "and nothing else: {by:?}");

    // And the cut that chooses the arms: the HEAVY side only. SK carries both
    // undecidable clusters; SG and SO are the typo targets and get an arm for
    // nobody. A census that ranked SO alongside SK would send the next unit off
    // to write a Somali register checksum.
    let heavy = &r.nobody_asked_heavy_country;
    assert_eq!(heavy.get("SK"), Some(&2));
    assert_eq!(heavy.len(), 1, "the light sides do not appear at all: {heavy:?}");
}
