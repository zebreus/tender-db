//! Issue 326: same identifier, two country codes one letter apart.
//!
//! The census exists because the issue-314 review campaign was deciding this
//! class by hand — 18 of 18 such cases came back `wrong-country`, unanimously —
//! and a predicate with that agreement rate is the thing the reviewers are
//! computing, not a heuristic about it.
//!
//! What these tests pin is the part that is easy to get subtly wrong: WHICH
//! pairs qualify, and how honestly the report describes what the checksum
//! evidence can and cannot settle. The query PLAN is pinned separately, in
//! `lib.rs`'s `the_country_typo_probe_seeks_the_far_country`, against the SQL
//! constant itself rather than a copy of it.

use store::turso::{self, Value};

/// A fake checksum probe whose arms are DISJOINT BY LENGTH, like the real one.
/// That property is what makes an unprobed country different from a refused
/// one, and this class lives almost entirely in the unprobed half.
fn anchors(value: &str) -> Vec<(&'static str, String)> {
    let digits: String = value.chars().filter(char::is_ascii_digit).collect();
    if value.bytes().any(|b| b.is_ascii_alphabetic()) {
        return Vec::new();
    }
    match digits.len() {
        9 => vec![("NO:orgnr", digits)],
        8 => vec![("CZ:ico", digits)],
        _ => Vec::new(),
    }
}

fn vocabulary(value: &str) -> Vec<&'static str> {
    let digits: String = value.chars().filter(char::is_ascii_digit).collect();
    if value.bytes().any(|b| b.is_ascii_alphabetic()) {
        return Vec::new();
    }
    match digits.len() {
        9 => vec!["NO:orgnr", "PT:nif"],
        8 => vec!["CZ:ico", "SI:davcna"],
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

fn never() -> bool {
    false
}

/// The shape the census is for, and the three shapes it must NOT mistake for
/// it: a two-letter difference, a same-country pair, and the same countries
/// carrying DIFFERENT identifiers.
#[tokio::test]
async fn only_one_letter_apart_and_identifier_identical_qualifies() {
    let (db, conn) = open("test-typo-shape").await;
    // The hit: a Slovak IČO also standing under SG. One letter, same digits.
    org(&conn, 1, "SK", "national", "41734602", "Tatra Trading s.r.o.").await;
    org(&conn, 2, "SG", "national", "41734602", "Tatra Trading s.r.o.").await;
    // TWO letters apart — a different country, not a slip.
    org(&conn, 3, "SK", "national", "36382914", "Dunaj a.s.").await;
    org(&conn, 4, "PL", "national", "36382914", "Dunaj a.s.").await;
    // One letter apart, but the identifiers differ: two registrants.
    org(&conn, 5, "LT", "national", "120229395", "Baltic UAB").await;
    org(&conn, 6, "LV", "national", "120229396", "Baltic UAB").await;
    // Same identifier, SAME country: a duplicate, and somebody else's problem.
    org(&conn, 7, "CZ", "national", "04095316", "Vltava s.r.o.").await;
    org(&conn, 8, "CZ", "national", "04095316", "Vltava s.r.o.").await;
    db.build_organization_indexes().await.unwrap();

    let r = db.country_typo_census(anchors, vocabulary, 100, &never).await.unwrap();
    assert_eq!(r.hits, 1, "exactly one pair qualifies");
    assert_eq!(r.rows.len(), 1);
    let hit = &r.rows[0];
    assert_eq!(hit.identifier, "41734602");
    let mut cc = [hit.country_a.as_str(), hit.country_b.as_str()];
    cc.sort_unstable();
    assert_eq!(cc, ["SG", "SK"]);
    assert!(!r.truncated);
    assert!(!r.stopped);
}

/// The report has to be honest about what the arithmetic settles. For this
/// class it usually settles nothing: SK has no scheme in the probe at all, so
/// an 8-digit Slovak IČO anchors CZ by shared arithmetic and that is evidence
/// about Czechia's algorithm, not about Slovakia.
#[tokio::test]
async fn the_census_separates_what_the_checksum_decides_from_what_it_cannot() {
    let (db, conn) = open("test-typo-decide").await;
    // DECIDABLE: a 9-digit NO-passing value under NO and its one-letter
    // neighbour NL. NO is in the 9-digit arm and agrees; NL is not in it at
    // all, so exactly one side is probed-and-agreeing.
    org(&conn, 1, "NO", "national", "980921565", "Mercell Holding ASA").await;
    org(&conn, 2, "NL", "national", "980921565", "Mercell Holding ASA").await;
    // UNDECIDABLE: an 8-digit value under SK and SG. Neither country has an
    // arm; the CZ anchor that fires says nothing about either.
    org(&conn, 3, "SK", "national", "41734602", "Tatra Trading s.r.o.").await;
    org(&conn, 4, "SG", "national", "41734602", "Tatra Trading s.r.o.").await;
    db.build_organization_indexes().await.unwrap();

    let r = db.country_typo_census(anchors, vocabulary, 100, &never).await.unwrap();
    assert_eq!(r.hits, 2);
    assert_eq!(r.decided, 1, "only the NO/NL pair has a side the arithmetic names");
    assert_eq!(r.neither_probed, 1, "and the SK/SG pair has neither side asked");

    let sk = r.rows.iter().find(|p| p.identifier == "41734602").unwrap();
    assert_eq!(sk.anchors, vec!["CZ:ico".to_owned()], "an anchor DID fire");
    assert!(!sk.a_probed && !sk.b_probed, "…on a country that is neither side's");
    assert!(
        !sk.a_agrees && !sk.b_agrees,
        "so both `agrees` are false, and that false is the uninformative kind — \
         the pair still needs a discriminator the checksum cannot give"
    );

    let no = r.rows.iter().find(|p| p.identifier == "980921565").unwrap();
    let (agrees_side, other) = if no.country_a == "NO" {
        ((no.a_probed, no.a_agrees), (no.b_probed, no.b_agrees))
    } else {
        ((no.b_probed, no.b_agrees), (no.a_probed, no.a_agrees))
    };
    assert_eq!(agrees_side, (true, true), "NO was asked and agreed");
    assert_eq!(other, (false, false), "NL was never asked — not refused");
}

/// A cap must be reported, and it must not corrupt the counts: an operator
/// reading `hits` is asking how big the class is, not how many rows fit.
#[tokio::test]
async fn the_cap_truncates_the_rows_and_never_the_counts() {
    let (db, conn) = open("test-typo-cap").await;
    for i in 0..5i64 {
        let ident = format!("4173460{i}");
        org(&conn, 100 + i, "SK", "national", &ident, "Tatra").await;
        org(&conn, 200 + i, "SG", "national", &ident, "Tatra").await;
    }
    db.build_organization_indexes().await.unwrap();

    let full = db.country_typo_census(anchors, vocabulary, 100, &never).await.unwrap();
    assert_eq!((full.hits, full.rows.len(), full.truncated), (5, 5, false));

    let capped = db.country_typo_census(anchors, vocabulary, 2, &never).await.unwrap();
    assert_eq!(capped.hits, 5, "the class is the whole class, not the page");
    assert_eq!(capped.rows.len(), 2);
    assert!(capped.truncated, "and the cap is reported, not silently applied");
    assert_eq!(capped.neither_probed, 5, "counts cover every hit, capped or not");
}

/// A cancelled census carries nothing. A partial one read as a whole one would
/// under-report the class, and the number is going to be quoted.
#[tokio::test]
async fn a_cancel_carries_no_partial_census() {
    let (db, conn) = open("test-typo-cancel").await;
    org(&conn, 1, "SK", "national", "41734602", "Tatra").await;
    org(&conn, 2, "SG", "national", "41734602", "Tatra").await;
    db.build_organization_indexes().await.unwrap();

    let always = || true;
    let r = db.country_typo_census(anchors, vocabulary, 100, &always).await.unwrap();
    assert!(r.stopped);
    assert_eq!(r.hits, 0);
    assert!(r.rows.is_empty());
}

/// The mention spread rides along as evidence for a reviewer. It is
/// deliberately NOT a tie-breaker the census applies: measured over the
/// issue-314 cohort it inverts on the pairs where Bhutan outweighs Bulgaria,
/// so a rule built on it would confidently pick the typo.
#[tokio::test]
async fn the_mention_spread_is_reported_but_decides_nothing() {
    let (db, conn) = open("test-typo-spread").await;
    org(&conn, 1, "SK", "national", "41734602", "Tatra").await;
    org(&conn, 2, "SG", "national", "41734602", "Tatra").await;
    conn.execute(
        "INSERT INTO fetches (id, source, kind, period, url, sha256, bytes, fetched_at, path)
         VALUES (1, 'ted', 'daily', 'p', 'u', 'aa', 1, 0, 'p')",
        (),
    )
    .await
    .unwrap();
    for n in 1..=4i64 {
        conn.execute(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                                  member_path, ingested_at, parse_state, projected)
             VALUES (?, 'ted', 'pub-' || ?, 'h' || ?, 'eforms', 1, 'm', 0, 'parsed', 1)",
            (Value::Integer(n), Value::Integer(n), Value::Integer(n)),
        )
        .await
        .unwrap();
    }
    // The SG side heavy, the SK side light — deliberately the "wrong" way
    // round, so a census that quietly preferred the heavy side would show it.
    for (n, o) in [(1i64, 2i64), (2, 2), (3, 2), (4, 1)] {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
             VALUES (?, 'ORG-' || ?, ?, 'Tatra', 'SK', NULL)",
            (Value::Integer(n), Value::Integer(n), Value::Integer(o)),
        )
        .await
        .unwrap();
    }
    db.build_organization_indexes().await.unwrap();

    let r = db.country_typo_census(anchors, vocabulary, 100, &never).await.unwrap();
    assert_eq!(r.hits, 1);
    let p = &r.rows[0];
    let (sk, sg) = if p.country_a == "SK" {
        (p.mentions_a, p.mentions_b)
    } else {
        (p.mentions_b, p.mentions_a)
    };
    assert_eq!((sk, sg), (1, 3), "both counts are reported as they stand");
    assert_eq!(r.decided, 0, "and the spread did not decide anything");
}

/// `rows_walked` is a COST figure and has to stay one. It reported the join's
/// output for one revision of this census, which made it a second name for
/// `hits` — the join can only ever return rows that matched, so the unmatched
/// rows a probe actually ranged over were invisible. An operator reading
/// "walked 1, found 1" would conclude the probe was free when it was not.
#[tokio::test]
async fn rows_walked_counts_the_population_probed_not_the_pairs_found() {
    let (db, conn) = open("test-typo-cost").await;
    // SK: three identifier-bearing rows. SG: five, exactly one of which shares
    // an identifier with SK. So the driving side is SK (the rarer one), the
    // probe ranges over its three rows, and it finds one pair.
    for (i, ident) in ["41734602", "36382914", "31359825"].iter().enumerate() {
        org(&conn, 10 + i as i64, "SK", "national", ident, "Tatra").await;
    }
    for (i, ident) in ["41734602", "70000001", "70000002", "70000003", "70000004"]
        .iter()
        .enumerate()
    {
        org(&conn, 20 + i as i64, "SG", "national", ident, "Tatra").await;
    }
    db.build_organization_indexes().await.unwrap();

    let r = db.country_typo_census(anchors, vocabulary, 100, &never).await.unwrap();
    assert_eq!(r.hits, 1, "one shared identifier");
    assert_eq!(
        r.rows_walked, 3,
        "and three rows were ranged over to find it — the SK side's whole \
         identifier-bearing population, not the one row that matched"
    );
    assert_eq!(r.pairs_considered, 1, "SK/SG is the only one-letter pairing present");
    assert_eq!(r.countries, 2);
}
