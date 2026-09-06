//! Issue 330: organization names carrying a line break.
//!
//! Noticed in the issue-329 census listing: `Vergabekammer Rheinland-Pfalz` with
//! a street and postcode appended, newlines and all, on an org carrying 12,249
//! mentions. The name is not cosmetic — `n2_key` and `n3_key` are computed from
//! it, so a polluted name produces a key that matches nothing and the org drops
//! silently out of every name-corroborated arm.
//!
//! What these tests pin is the judgement the census exists to support: that
//! PUBLISHED and DERIVED-ONLY are distinguished from the mentions rather than
//! guessed, that an org with no mentions abstains instead of being filed under
//! either, and that `cap` bounds the listing and never the tally.

use store::turso::{self, Value};

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

/// An org whose stored name is `name`, plus one mention per entry in
/// `mention_names` — which is what the notices actually said.
async fn org(
    conn: &turso::Connection,
    id: i64,
    cc: &str,
    name: &str,
    mention_names: &[&str],
) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
         VALUES (?, ?, 'vat', 'DE' || ?, ?, 0, 0)",
        (
            Value::Integer(id),
            Value::Text(cc.into()),
            Value::Integer(id),
            Value::Text(name.into()),
        ),
    )
    .await
    .unwrap();
    for (i, mn) in mention_names.iter().enumerate() {
        let notice = id * 1000 + i as i64;
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
                Value::Text((*mn).into()),
                Value::Text(cc.into()),
            ),
        )
        .await
        .unwrap();
    }
}

/// An org with an explicit triple (or none of it) and its mentions, for the
/// address-shaped measurement, which is about same-triple twins.
async fn org_with(
    conn: &turso::Connection,
    id: i64,
    cc: Option<&str>,
    triple: Option<(&str, &str)>,
    name: &str,
    mention_names: &[&str],
) {
    let cc_v = cc.map_or(Value::Null, |c| Value::Text(c.into()));
    let (kind, identifier) = match triple {
        Some((k, i)) => (Value::Text(k.into()), Value::Text(i.into())),
        None => (Value::Null, Value::Null),
    };
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
         VALUES (?, ?, ?, ?, ?, ?, 0)",
        (
            Value::Integer(id),
            cc_v.clone(),
            kind,
            identifier,
            Value::Text(name.into()),
            Value::Integer(i64::from(triple.is_none())),
        ),
    )
    .await
    .unwrap();
    for (i, mn) in mention_names.iter().enumerate() {
        let notice = id * 1000 + i as i64;
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
                Value::Text((*mn).into()),
                cc_v.clone(),
            ),
        )
        .await
        .unwrap();
    }
}

fn never() -> bool {
    false
}

fn nowhere(_done: u64, _detail: &str) {}

/// Stand-in for the app's `crosswalk::n3_key`: lower-cased alphanumeric
/// tokens, which is all the census's equality checks need.
fn n3(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Stand-in for `ingest::address::strip_trailing_address` (that function has
/// its own tests in the ingest crate): drop the trailing lines from the first
/// one that starts with a digit, keeping at least the first line.
fn strip(s: &str) -> Option<String> {
    let lines: Vec<&str> = s.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let keep = lines.iter().position(|l| l.starts_with(|c: char| c.is_ascii_digit()))?;
    (keep > 0).then(|| lines[..keep].join(" "))
}

async fn run(db: &store::Db, cap: usize) -> store::NamePollutionReport {
    db.name_pollution_census(n3, strip, cap, &never, &nowhere).await.unwrap()
}

#[tokio::test]
async fn a_clean_name_is_not_a_finding() {
    let (db, conn) = open("np-clean").await;
    org(&conn, 1, "DE", "Siemens AG", &["Siemens AG"]).await;
    org(&conn, 2, "DE", "Robert Bosch GmbH", &["Robert Bosch GmbH"]).await;
    let r = run(&db, 100).await;
    assert_eq!(r.rows_walked, 2);
    assert_eq!(r.polluted, 0);
    assert!(r.rows.is_empty());
    // Every row contributes a length, polluted or not — the distribution is
    // corpus-wide because a threshold has to be chosen against the whole corpus.
    assert_eq!(r.name_lengths.len(), 2);
}

#[tokio::test]
async fn a_break_the_notice_itself_carries_is_published() {
    let (db, conn) = open("np-published").await;
    // The live specimen's shape. If the mention carries the break too, no parser
    // change on our side would have prevented it, and calling this a defect
    // would send the next agent looking for a bug that is not there.
    org(
        &conn,
        1,
        "DE",
        "Vergabekammer Rheinland-Pfalz\nStiftsstraße 9\n55116 Mainz",
        &["Vergabekammer Rheinland-Pfalz\nStiftsstraße 9\n55116 Mainz", "Vergabekammer Rheinland-Pfalz"],
    )
    .await;
    let r = run(&db, 100).await;
    assert_eq!(r.polluted, 1);
    assert_eq!(r.published, 1);
    assert_eq!(r.derived_only, 0);
    assert_eq!(r.polluted_mentions, 2, "both mentions hang off the broken name");
    assert_eq!(r.by_country.get("DE"), Some(&1));
    // Escaped, because a literal break inside the JSON report is legal and
    // unreadable, and the listing exists to be read.
    assert_eq!(
        r.rows[0].name,
        "Vergabekammer Rheinland-Pfalz\\nStiftsstraße 9\\n55116 Mainz"
    );
    assert_eq!(r.rows[0].verdict, "published");
}

#[tokio::test]
async fn a_break_no_notice_carries_is_ours() {
    let (db, conn) = open("np-derived").await;
    // Every mention clean, the stored name broken: something downstream of the
    // notice introduced it. THIS is the bucket that would be a parser defect,
    // and separating it from the published class is the whole point of pass 2.
    org(&conn, 1, "DE", "Stadt Mainz\n55116 Mainz", &["Stadt Mainz", "Stadt Mainz"]).await;
    let r = run(&db, 100).await;
    assert_eq!(r.polluted, 1);
    assert_eq!(r.published, 0);
    assert_eq!(r.derived_only, 1);
    assert_eq!(r.rows[0].verdict, "derived-only");
}

#[tokio::test]
async fn a_carriage_return_counts_as_a_break() {
    let (db, conn) = open("np-cr").await;
    org(&conn, 1, "AT", "Amt der Tiroler Landesregierung\r\nInnsbruck", &["Amt der Tiroler Landesregierung\r\nInnsbruck"]).await;
    let r = run(&db, 100).await;
    assert_eq!(r.polluted, 1);
    assert_eq!(r.published, 1);
    assert_eq!(r.by_country.get("AT"), Some(&1));
}

#[tokio::test]
async fn an_org_with_no_mentions_abstains() {
    let (db, conn) = open("np-orphan").await;
    // Neither published nor ours — there is nothing to compare against, and
    // filing it under either would be inventing evidence.
    org(&conn, 1, "DE", "Irgendein Amt\nSomewhere", &[]).await;
    let r = run(&db, 100).await;
    assert_eq!(r.polluted, 1);
    assert_eq!(r.no_mentions, 1);
    assert_eq!(r.published, 0);
    assert_eq!(r.derived_only, 0);
    assert_eq!(r.rows[0].verdict, "no-mentions");
}

#[tokio::test]
async fn the_cap_bounds_the_listing_and_never_the_tally() {
    let (db, conn) = open("np-cap").await;
    // The lesson inherited from issue 326, where verdicts computed over the
    // carried slice inverted two published conclusions.
    for n in 1..=9i64 {
        org(&conn, n, "DE", &format!("Amt {n}\nStraße {n}"), &[&format!("Amt {n}\nStraße {n}")]).await;
    }
    let r = run(&db, 3).await;
    assert_eq!(r.polluted, 9);
    assert_eq!(r.published, 9, "all nine got a verdict");
    assert_eq!(r.rows.len(), 3, "only three are listed");
    assert!(r.truncated);
}

#[tokio::test]
async fn a_stop_request_returns_stopped_and_no_half_report() {
    let (db, conn) = open("np-stop").await;
    org(&conn, 1, "DE", "Amt\nStraße", &["Amt\nStraße"]).await;
    fn always() -> bool {
        true
    }
    let r = db.name_pollution_census(n3, strip, 100, &always, &nowhere).await.unwrap();
    assert!(r.stopped);
    // Issue 252's honest cancel: nothing, rather than a partial tally a reader
    // would take for a corpus-wide one.
    assert_eq!(r.polluted, 0);
    assert!(r.rows.is_empty());
}

/// Issue 330's second measurement. The strip is a key-builder candidate, not a
/// repair, and before it is built the census says what it would change: which
/// address-shaped rows would GAIN agreement with a same-triple twin (the E0
/// `contained` bucket moving to `agree`), and whose stripped key another
/// identity in the same country already holds.
#[tokio::test]
async fn an_address_shaped_name_is_measured_against_its_twin_and_the_key_table() {
    let (db, conn) = open("np-address").await;
    // The issue's specimen: same triple, one clean row, one with the postal
    // block. The polluted key is a superset; the stripped key is the clean one.
    org_with(&conn, 1, Some("DE"), Some(("national", "DE355604198")), "Vergabekammer Rheinland-Pfalz", &["Vergabekammer Rheinland-Pfalz"]).await;
    org_with(&conn, 2, Some("DE"), Some(("national", "DE355604198")), "Vergabekammer Rheinland-Pfalz\n55116 Mainz", &["Vergabekammer Rheinland-Pfalz\n55116 Mainz"]).await;
    // A twin whose key already equals the polluted one: nothing to gain.
    org_with(&conn, 3, Some("DE"), Some(("vat", "DE111")), "Stadt Mainz 55116 Mainz", &["Stadt Mainz 55116 Mainz"]).await;
    org_with(&conn, 4, Some("DE"), Some(("vat", "DE111")), "Stadt Mainz\n55116 Mainz", &["Stadt Mainz\n55116 Mainz"]).await;
    // No twin, but the stripped key is carried by a row under ANOTHER
    // identifier in the same country: the strip would hand it a shared key.
    org_with(&conn, 5, Some("DE"), Some(("vat", "DE555")), "Landkreis Saalekreis\n06217 Merseburg", &["Landkreis Saalekreis\n06217 Merseburg"]).await;
    org_with(&conn, 6, Some("DE"), Some(("national", "06-1-99")), "Landkreis Saalekreis", &["Landkreis Saalekreis"]).await;
    conn.execute("INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (6, 'n3', 'landkreis saalekreis')", ()).await.unwrap();
    // A carrier under a DIFFERENT country is not a collision: R2 keys on
    // country, and a Dutch row cannot meet a German one there.
    org_with(&conn, 7, Some("NL"), Some(("vat", "NL777")), "Landkreis Saalekreis", &[]).await;
    conn.execute("INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (7, 'n3', 'landkreis saalekreis')", ()).await.unwrap();
    // A wrapped name: polluted, not address-shaped. The bulk of the class.
    org_with(&conn, 8, Some("DE"), Some(("vat", "DE888")), "Landeshauptstadt\nDresden, Zentrales Vergabebüro", &["Landeshauptstadt\nDresden, Zentrales Vergabebüro"]).await;
    // A NULL-country provisional row: counted as address-shaped, no seeks.
    org_with(&conn, 9, None, None, "Stadt Burghausen\n84489 Burghausen", &["Stadt Burghausen\n84489 Burghausen"]).await;
    // A twin that STILL differs after the strip: a department line survives
    // above the postal block, and an address strip is the wrong tool for it.
    org_with(&conn, 10, Some("DE"), Some(("vat", "DE1010")), "Landratsamt Kelheim", &["Landratsamt Kelheim"]).await;
    org_with(&conn, 11, Some("DE"), Some(("vat", "DE1010")), "Landratsamt Kelheim\nKreisfinanzverwaltung\n93309 Kelheim", &["Landratsamt Kelheim\nKreisfinanzverwaltung\n93309 Kelheim"]).await;

    let r = run(&db, 100).await;
    assert_eq!(r.polluted, 6, "rows 2, 4, 5, 8, 9, 11 carry a break");
    assert_eq!(r.address_shaped, 5, "rows 2, 4, 5, 9, 11 end in a postal block");
    assert_eq!(r.address_with_country, 4);
    assert_eq!(r.address_by_country.get("DE"), Some(&4));
    assert_eq!(r.twin_rows, 3, "rows 2, 4 and 11 have a same-triple twin");
    assert_eq!(r.gains_agreement, 1);
    assert_eq!(r.already_agree, 1);
    assert_eq!(r.still_differs, 1);
    assert_eq!(r.collides_other_identifier, 1, "row 5 alone: row 7's carrier is Dutch");
    assert!(!r.address_truncated);
    assert!(!r.address_seeks_truncated, "four rows are nowhere near the seek ceiling");

    let by_id = |id: i64| r.address_rows.iter().find(|a| a.org_id == id).unwrap();
    let a = by_id(2);
    assert_eq!(a.verdict, "gains-agreement");
    assert_eq!(a.twins, vec![1]);
    assert_eq!(a.stripped, "Vergabekammer Rheinland-Pfalz");
    assert_eq!(a.name, "Vergabekammer Rheinland-Pfalz\\n55116 Mainz", "escaped like the main listing");
    assert_eq!(a.mentions, 1);
    assert_eq!(a.key_carriers, 0);
    assert!(!a.collides_other_identifier);
    assert_eq!(by_id(4).verdict, "already-agree");
    let s5 = by_id(5);
    assert_eq!(s5.verdict, "no-twin");
    assert_eq!(s5.key_carriers, 2, "both carriers are counted; only the German one collides");
    assert!(s5.collides_other_identifier);
    let s9 = by_id(9);
    assert_eq!(s9.country, "");
    assert_eq!(s9.verdict, "no-twin");
    assert_eq!(s9.key_carriers, 0, "no seeks for a country-less row");
    let s11 = by_id(11);
    assert_eq!(s11.verdict, "still-differs");
    assert_eq!(s11.twins, vec![10]);
    assert_eq!(s11.stripped, "Landratsamt Kelheim Kreisfinanzverwaltung");
    assert!(r.address_rows.iter().all(|a| a.org_id != 8), "a wrapped name is not address-shaped");
    // The main listing and tally are untouched by the second measurement.
    assert_eq!(r.published, 6);
    assert_eq!(r.rows.len(), 6);
}

#[tokio::test]
async fn the_address_listing_has_its_own_cap() {
    let (db, conn) = open("np-address-cap").await;
    for n in 1..=6i64 {
        org_with(&conn, n, Some("DE"), Some(("vat", &format!("DE{n}"))), &format!("Amt {n}\n1234{n} Ort"), &[]).await;
    }
    let r = run(&db, 4).await;
    assert_eq!(r.address_shaped, 6, "the tally is corpus-wide");
    assert_eq!(r.address_rows.len(), 4);
    assert!(r.address_truncated);
}
