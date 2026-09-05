//! Issue 357: the identifier-under-several-codes residue, carried with member
//! rows and per-member evidence for review through the country-verdict path.

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

async fn packet(db: &store::Db, cap: usize) -> store::ClusterPacket {
    db.country_cluster_packet(anchors, vocabulary, one_letter, footprint, cap, 2, &never)
        .await
        .unwrap()
}

/// The census decides the class; the packet adds what a country verdict needs
/// — the ORG IDS — and the per-member evidence, heaviest cluster first.
#[tokio::test]
async fn a_cluster_is_carried_with_its_member_rows_and_their_evidence() {
    let (db, conn) = open("cluster-packet").await;
    // 12345678 under SK (heavy), CZ and LV: eight digits validate as SK:ico
    // and SI:maticna, the vocabulary asks SK, SI and CZ.
    org(&conn, 10, "SK", "12345678", "Tamtron s.r.o.", 5).await;
    org(&conn, 11, "CZ", "12345678", "Tamtron s.r.o.", 1).await;
    org(&conn, 12, "LV", "12345678", "Tamtron s.r.o.", 1).await;
    // A lighter cluster, so the order is observable.
    org(&conn, 20, "BG", "123456789", "Софарма АД", 2).await;
    org(&conn, 21, "VU", "123456789", "Софарма АД", 1).await;
    // Too short to be an identity — excluded.
    org(&conn, 30, "DE", "12", "Kurz GmbH", 3).await;
    org(&conn, 31, "AT", "12", "Kurz GmbH", 3).await;
    // An operational footprint — excluded, never corrected.
    org(&conn, 40, "CH", "40871200", "Embassy of Switzerland", 9).await;
    org(&conn, 41, "FR", "40871200", "Embassy of Switzerland", 4).await;

    let p = packet(&db, 100).await;
    assert!(!p.stopped && !p.truncated);
    assert_eq!(p.clusters, 4, "the census sees all four clusters");
    assert_eq!(p.eligible, 2, "too-short and footprint clusters are left out");
    assert_eq!(p.cases.len(), 2);
    let first = &p.cases[0];
    assert_eq!((first.identifier.as_str(), first.total_mentions), ("12345678", 7), "heaviest first");
    assert_eq!(first.members.len(), 3, "one member per row, with its org id");
    let by_org: std::collections::BTreeMap<i64, &store::XbMember> =
        first.members.iter().map(|m| (m.org, m)).collect();
    let sk = by_org[&10];
    assert_eq!((sk.country.as_deref(), sk.mentions, sk.country_probed, sk.country_agrees), (Some("SK"), 5, true, true));
    assert!(sk.anchors.contains(&"SK:ico".to_owned()));
    let cz = by_org[&11];
    assert_eq!((cz.country_probed, cz.country_agrees), (true, false), "CZ was asked and refused");
    let lv = by_org[&12];
    assert_eq!((lv.country_probed, lv.country_agrees), (false, false), "LV has no scheme of this shape");
    assert_eq!(lv.notices.len(), 1, "a few publication ids travel with the member");
    assert_eq!(p.cases[1].identifier, "123456789");
    assert_eq!(p.by_verdict.values().sum::<u64>(), 2);
}

#[tokio::test]
async fn the_cap_truncates_after_the_heaviest_and_says_so() {
    let (db, conn) = open("cluster-packet-cap").await;
    org(&conn, 10, "SK", "12345678", "Tamtron s.r.o.", 5).await;
    org(&conn, 11, "CZ", "12345678", "Tamtron s.r.o.", 1).await;
    org(&conn, 20, "BG", "123456789", "Софарма АД", 2).await;
    org(&conn, 21, "VU", "123456789", "Софарма АД", 1).await;
    let p = packet(&db, 1).await;
    assert!(p.truncated);
    assert_eq!(p.eligible, 2, "the tally is not capped, only the listing");
    assert_eq!(p.cases.len(), 1);
    assert_eq!(p.cases[0].identifier, "12345678");
}

/// A cluster a campaign has already read stays out, whatever the verdict said
/// — the standing verdict on any member is the record of that reading, and
/// the cap carries new work (slice 3 of the 357 campaign found 381 of 600
/// carried clusters already reviewed).
#[tokio::test]
async fn a_cluster_with_a_standing_verdict_on_a_member_is_left_out_and_counted() {
    let (db, conn) = open("cluster-packet-reviewed").await;
    org(&conn, 10, "SK", "12345678", "Tamtron s.r.o.", 5).await;
    org(&conn, 11, "CZ", "12345678", "Tamtron s.r.o.", 1).await;
    org(&conn, 20, "BG", "123456789", "Софарма АД", 2).await;
    org(&conn, 21, "VU", "123456789", "Софарма АД", 1).await;
    // A parked (medium) verdict on the VU stranger: reviewed, kept on purpose.
    db.record_country_verdicts(
        "c",
        &[store::CountryVerdict {
            org_id: 21,
            action: "move".into(),
            from_country: Some("VU".into()),
            to_country: Some("BG".into()),
            rationale: "parked".into(),
            confidence: "medium".into(),
        }],
        1,
    )
    .await
    .unwrap();
    let p = packet(&db, 100).await;
    assert_eq!((p.eligible, p.already_reviewed, p.cases.len()), (2, 1, 1));
    assert_eq!(p.cases[0].identifier, "12345678");
    assert!(!p.truncated);
    // The cap counts CARRIED cases, so a reviewed cluster does not use it up.
    let p = packet(&db, 1).await;
    assert_eq!((p.cases.len(), p.already_reviewed, p.truncated), (1, 0, true), "the heaviest is carried; the cap stops before the reviewed one is even looked at");
}
