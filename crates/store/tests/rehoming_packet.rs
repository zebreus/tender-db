//! Issue 317 Unit A: the review packet. `fusion-census` measures the fusion;
//! a verdict needs an ADDRESS and a DESTINATION, and this is what supplies
//! both. It writes nothing, so its whole contract is what it shows, what it
//! correctly refuses to show, and whether its numbers mean what they say.

use std::sync::atomic::{AtomicUsize, Ordering};
use store::turso::Value;
use store::RehomingVerdict;

/// The real `match_norm`: alphanumerics, Unicode-lowercased, everything else
/// a gap. The packet groups by it and `org_match_keys` is keyed by it, so a
/// fixture that normalizes differently tests nothing.
fn norm(name: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in name.chars() {
        if c.is_alphanumeric() {
            if gap && !out.is_empty() {
                out.push(' ');
            }
            gap = false;
            out.extend(c.to_lowercase());
        } else {
            gap = true;
        }
    }
    out
}

/// The genericness wall the app threads in (`SCAN_STOPLIST_CAP`).
const WALL: usize = 20;

async fn org(conn: &store::turso::Connection, id: i64, country: &str, name: &str) {
    org_with(conn, id, country, name, None).await;
}

async fn org_with(
    conn: &store::turso::Connection,
    id: i64,
    country: &str,
    name: &str,
    identifier: Option<&str>,
) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (?, ?, ?, ?, ?, ?, 0, 0)",
        (
            Value::Integer(id),
            Value::Text(country.into()),
            match identifier {
                Some(_) => Value::Text("national".into()),
                None => Value::Null,
            },
            match identifier {
                Some(v) => Value::Text(v.into()),
                None => Value::Null,
            },
            Value::Text(name.into()),
            Value::Text(name.to_lowercase()),
        ),
    )
    .await
    .unwrap();
    key(conn, id, name).await;
}

async fn key(conn: &store::turso::Connection, id: i64, name: &str) {
    conn.execute(
        "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', ?)",
        (Value::Integer(id), Value::Text(norm(name))),
    )
    .await
    .unwrap();
}

async fn mention(conn: &store::turso::Connection, notice: i64, section: &str, on: i64, name: &str) {
    mention_id(conn, notice, section, on, name, None).await;
}

async fn mention_id(
    conn: &store::turso::Connection,
    notice: i64,
    section: &str,
    on: i64,
    name: &str,
    raw: Option<&str>,
) {
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
         VALUES (?, ?, ?, ?, 'DE', ?)",
        (
            Value::Integer(notice),
            Value::Text(section.into()),
            Value::Integer(on),
            Value::Text(name.into()),
            match raw {
                Some(v) => Value::Text(v.into()),
                None => Value::Null,
            },
        ),
    )
    .await
    .unwrap();
}

async fn reviewed(conn: &store::turso::Connection, org_id: i64, cohort: &str) {
    conn.execute(
        "INSERT INTO org_case_reviews
           (case_org_id, cohort, verdict, diagnosis, handling, rationale, confidence,
            reviewed_at, applied_at, applied_action, job_id)
         VALUES (?, ?, 'consortium-vehicle-wrong-identifier', 'd', 'h', 'r', 'high', 1, 1, 'a', 1)",
        (Value::Integer(org_id), Value::Text(cohort.into())),
    )
    .await
    .unwrap();
}

async fn seed(path: &str) -> (store::Db, store::turso::Connection) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (db, conn)
}

/// The happy path and the distinctions the packet exists to draw: the fusion
/// proper with a ranked destination; a case whose destination does not exist;
/// a mention already decided; a reviewed case with nothing off-name.
#[tokio::test]
async fn the_packet_carries_addresses_destinations_and_only_the_undecided() {
    let (db, conn) = seed("test-rehoming-packet.db").await;

    // 1 = the vehicle. 2 = the member it should re-home to, established.
    // 3 = a same-named foreign STUB, so the ranking has something to get
    // wrong. 4 = a second vehicle whose member has no standing row at all.
    // 5 = a clean reviewed org: mentions, none off-name.
    org(&conn, 1, "DE", "Bietergemeinschaft Dobler / Oberall").await;
    org(&conn, 2, "DE", "Dobler GmbH").await;
    org(&conn, 3, "AT", "DOBLER GMBH").await;
    org(&conn, 4, "DE", "Bietergemeinschaft MIV / Nord").await;
    org(&conn, 5, "DE", "Stadt Musterhausen").await;
    for id in [1i64, 4, 5] {
        reviewed(&conn, id, "biege").await;
    }

    // Vehicle 1: four solo-member mentions (one already decided), one that
    // names the vehicle itself, and one that normalizes to nothing.
    mention(&conn, 900, "ORG-1", 1, "Dobler GmbH").await;
    mention(&conn, 901, "ORG-1", 1, "DOBLER GMBH").await;
    mention(&conn, 902, "ORG-1", 1, "Dobler GmbH").await;
    mention(&conn, 903, "ORG-1", 1, "Bietergemeinschaft Dobler / Oberall").await;
    mention(&conn, 904, "ORG-1", 1, "!!!").await;
    mention(&conn, 907, "ORG-1", 1, "Dobler GmbH").await;
    // Vehicle 4: a member nobody has a row for.
    mention(&conn, 905, "ORG-1", 4, "MIV Ingenieurbüro GmbH").await;
    // Org 5: on-name only, so it is not a case at all.
    mention(&conn, 906, "ORG-1", 5, "Stadt Musterhausen").await;
    // Give the established member a history the stub does not have.
    for n in 910i64..920 {
        mention(&conn, n, "ORG-1", 2, "Dobler GmbH").await;
    }

    // 902 is decided — a high-confidence rehome onto a row that stands, which
    // apply-rehoming will execute on its next run — and must not come back.
    db.record_rehoming(
        "biege",
        &[RehomingVerdict {
            case_org_id: 1,
            notice_id: 902,
            section_id: "ORG-1".into(),
            action: "rehome".into(),
            target_org_id: Some(2),
            target_name: Some("Dobler GmbH".into()),
            rationale: "decided earlier".into(),
            confidence: "high".into(),
        }],
        1,
    )
    .await
    .unwrap();

    let never = || false;
    let p = db.rehoming_packet(norm, 150, 80, 5, WALL, &never).await.unwrap();

    assert_eq!(p.cases, 3, "three applied case ORGS were reviewed");
    assert_eq!(p.open, 2, "org 5 is on-name only, so it is not open work");
    assert_eq!(p.off_name_mentions, 4, "900, 901, 907 and 905 — not 902, 903 or 904");
    assert_eq!(p.already_reviewed, 1, "902 is decided: counted, not listed");
    assert_eq!(p.parked_total, 0, "nothing recorded here is unappliable");
    assert!(!p.truncated);

    // Sorted by OPEN work descending: the vehicle with three open mentions.
    let one = &p.rows[0];
    assert_eq!(one.org, 1);
    assert_eq!(one.mentions, 5, "judgeable mentions exclude the punctuation-only name");
    assert_eq!(one.off_name, 4, "the ratio counts every off-name mention, decided included");
    assert_eq!(one.open, 3, "and `open` is the part still to review");
    assert_eq!(one.off_mentions.len(), 3);
    assert_eq!(
        one.off_mentions.iter().map(|m| m.notice_id).collect::<Vec<_>>(),
        vec![900, 901, 907],
        "addressed and in notice order, so a reviewer works a stable list"
    );
    assert!(one.off_mentions.iter().all(|m| m.group_shown));
    assert!(!one.mentions_truncated);
    assert_eq!(one.groups_elided, 0);

    // Case folds away, so the two spellings are one key and one group.
    assert_eq!(one.groups.len(), 1);
    let g = &one.groups[0];
    assert_eq!(g.mentions, 3);
    assert_eq!(g.name, "Dobler GmbH", "the MODAL spelling leads, not the first seen");
    assert!(!g.generic_key, "two carriers is nowhere near the wall");
    assert_eq!(g.target_total, 2, "orgs 2 and 3 carry the key; the case org never counts");
    assert_eq!(
        g.targets.iter().map(|t| t.org).collect::<Vec<_>>(),
        vec![2, 3],
        "the same-country row the corpus knows outranks the foreign stub"
    );
    assert_eq!(g.targets[0].mentions, 10);
    assert!(!g.targets[0].saturated);
    assert!(!g.targets[0].via_alias, "org 2 matched on its own head name");
    assert_eq!(g.targets[1].country.as_deref(), Some("AT"), "so a homonym reads as one");

    // Vehicle 4's member has no row: no destination, which is the count that
    // sizes the minting question v1 deliberately refuses to answer.
    let four = p.rows.iter().find(|c| c.org == 4).expect("vehicle 4 is open");
    assert_eq!(four.groups.len(), 1);
    assert!(four.groups[0].targets.is_empty());
    assert_eq!(four.groups[0].target_total, 0);
    assert_eq!(p.groups_total, 2, "one distinct off-name key on each open case");
    assert_eq!(p.probed_groups, 2);
    assert_eq!(p.probed_groups_with_target, 1, "one of the two has somewhere to go");
    assert_eq!(p.groups_generic, 0);
}

/// An org reviewed under two cohorts is ONE case. `org_case_reviews` is keyed
/// (case_org_id, cohort), and walking it per row doubles every total and
/// lists the same mention address twice.
#[tokio::test]
async fn an_org_reviewed_twice_is_counted_once() {
    let (db, conn) = seed("test-rehoming-packet-cohorts.db").await;
    org(&conn, 1, "DE", "Bietergemeinschaft Dobler / Oberall").await;
    org(&conn, 2, "DE", "Dobler GmbH").await;
    reviewed(&conn, 1, "biege-pilot").await;
    reviewed(&conn, 1, "biege-batch-2").await;
    mention(&conn, 900, "ORG-1", 1, "Dobler GmbH").await;

    let never = || false;
    let p = db.rehoming_packet(norm, 150, 80, 5, WALL, &never).await.unwrap();
    assert_eq!(p.cases, 1, "one ORG, two review rows");
    assert_eq!(p.open, 1);
    assert_eq!(p.off_name_mentions, 1);
    assert_eq!(p.rows.len(), 1);
    assert_eq!(p.rows[0].off_mentions.len(), 1);
}

/// A name carried by more standing rows than the genericness wall allows is a
/// shared literal, not a destination. Offering five of its rows would offer
/// five arbitrary ones — the probe has no ordering — so the group is listed
/// with the count and NO targets.
#[tokio::test]
async fn a_name_over_the_genericness_wall_is_offered_no_destination() {
    let (db, conn) = seed("test-rehoming-packet-generic.db").await;
    org(&conn, 1, "DE", "Bietergemeinschaft Gymnasium / Nord").await;
    reviewed(&conn, 1, "biege").await;
    mention(&conn, 900, "ORG-1", 1, "Gymnasium").await;
    // WALL + 1 standing rows share the name.
    for id in 100i64..(100 + WALL as i64 + 1) {
        org(&conn, id, "DE", "Gymnasium").await;
    }

    let never = || false;
    let p = db.rehoming_packet(norm, 150, 80, 5, WALL, &never).await.unwrap();
    let g = &p.rows[0].groups[0];
    assert!(g.generic_key, "{} carriers is over the wall", g.target_total);
    assert!(g.targets.is_empty(), "a shared literal has carriers, not destinations");
    assert_eq!(g.target_total, WALL as u64 + 1);
    assert_eq!(p.groups_generic, 1);
    assert_eq!(p.probed_groups_with_target, 0, "a wall is not a destination");
    // The mention is still listed: the group is work, and the address is the
    // thing the packet exists to supply.
    assert_eq!(p.rows[0].off_mentions.len(), 1);
}

/// A verdict `apply-rehoming` can never execute must not simply vanish from
/// the packet. Excluding it as "decided" makes the campaign read a mention
/// that never moved as one it had finished.
#[tokio::test]
async fn an_unappliable_verdict_comes_back_parked_with_its_reason() {
    let (db, conn) = seed("test-rehoming-packet-parked.db").await;
    org(&conn, 1, "DE", "Bietergemeinschaft Dobler / Oberall").await;
    org(&conn, 2, "DE", "Dobler GmbH").await;
    reviewed(&conn, 1, "biege").await;
    for n in [900i64, 901, 902, 903] {
        mention(&conn, n, "ORG-1", 1, "Dobler GmbH").await;
    }
    let v = |notice: i64, action: &str, target: Option<i64>, conf: &str| RehomingVerdict {
        case_org_id: 1,
        notice_id: notice,
        section_id: "ORG-1".into(),
        action: action.into(),
        target_org_id: target,
        target_name: Some("Dobler GmbH".into()),
        rationale: "r".into(),
        confidence: conf.into(),
    };
    db.record_rehoming(
        "biege",
        &[
            // Final: a keep is a decision, and nothing ever stamps one, so
            // testing its applied_at would park every keep forever.
            v(900, "keep", None, "high"),
            // Parked: below the apply bar.
            v(901, "rehome", Some(2), "medium"),
            // Parked: named a name, never a row (v1 mints nothing).
            v(902, "rehome", None, "high"),
            // Parked: the destination no longer stands.
            v(903, "rehome", Some(4242), "high"),
        ],
        1,
    )
    .await
    .unwrap();

    let never = || false;
    let p = db.rehoming_packet(norm, 150, 80, 5, WALL, &never).await.unwrap();
    assert_eq!(p.open, 0, "every mention carries a verdict, so nothing is open work");
    assert_eq!(p.already_reviewed, 1, "only the keep is decided");
    assert_eq!(p.parked_total, 3);
    let reasons: Vec<&str> = p.parked.iter().map(|x| x.reason.as_str()).collect();
    assert!(reasons.iter().any(|r| r.contains("confidence medium")), "{reasons:?}");
    assert!(reasons.iter().any(|r| r.contains("no target org id")), "{reasons:?}");
    assert!(reasons.iter().any(|r| r.contains("4242 no longer stands")), "{reasons:?}");
}

/// A cancel must not hand back a short packet. A reviewer cannot tell a
/// packet that stopped early from one whose missing cases were clean, and
/// acting on the difference is how a campaign silently skips work. The
/// checkpoint under test is the mid-case one, between destination probes.
#[tokio::test]
async fn a_cancelled_packet_is_empty_rather_than_partial() {
    let (db, conn) = seed("test-rehoming-packet-stop.db").await;
    org(&conn, 1, "DE", "Bietergemeinschaft Dobler / Oberall").await;
    org(&conn, 2, "DE", "Dobler GmbH").await;
    org(&conn, 3, "DE", "Oberall AG").await;
    reviewed(&conn, 1, "biege").await;
    mention(&conn, 900, "ORG-1", 1, "Dobler GmbH").await;
    mention(&conn, 901, "ORG-1", 1, "Oberall AG").await;

    // The uncancelled companion: without it, "empty" is indistinguishable
    // from "there was nothing to find".
    let never = || false;
    let whole = db.rehoming_packet(norm, 150, 80, 5, WALL, &never).await.unwrap();
    assert_eq!(whole.rows.len(), 1);
    assert_eq!(whole.groups_total, 2, "two distinct off-name keys on the one case");

    // Calls, in order: pass-1 case top, pass-2 case top, then one per group.
    // True from the third call lands inside the case, on the first group.
    let n = AtomicUsize::new(0);
    let mid = || n.fetch_add(1, Ordering::SeqCst) >= 2;
    let p = db.rehoming_packet(norm, 150, 80, 5, WALL, &mid).await.unwrap();
    assert!(p.stopped);
    assert!(p.rows.is_empty());
    assert_eq!(p.off_name_mentions, 0);
    assert_eq!(p.cases, 0, "even the totals are dropped — a partial count is a wrong count");
    assert!(n.load(Ordering::SeqCst) >= 3, "the mid-case checkpoint was the one that fired");
}

/// The case cap bounds the LISTING, never the workload — and it keeps the
/// biggest cases, because a cap that keeps the first ones in id order and
/// then sorts them by size presents an arbitrary subset as the worst.
#[tokio::test]
async fn the_case_cap_keeps_the_biggest_cases_and_never_shrinks_the_totals() {
    let (db, conn) = seed("test-rehoming-packet-cap.db").await;
    org(&conn, 1, "DE", "Bietergemeinschaft Small / One").await;
    org(&conn, 2, "DE", "Bietergemeinschaft Big / Two").await;
    org(&conn, 3, "DE", "Member Alpha").await;
    org(&conn, 4, "DE", "Member Beta").await;
    reviewed(&conn, 1, "biege").await;
    reviewed(&conn, 2, "biege").await;
    // Org 1 (the lower id) is the SMALLER case, so an id-ordered cap would
    // keep exactly the wrong one.
    mention(&conn, 900, "ORG-1", 1, "Member Alpha").await;
    for n in 910i64..914 {
        mention(&conn, n, "ORG-1", 2, "Member Beta").await;
    }

    let never = || false;
    let p = db.rehoming_packet(norm, 150, 80, 5, WALL, &never).await.unwrap();
    assert_eq!(p.open, 2);
    assert_eq!(p.off_name_mentions, 5);
    assert!(!p.truncated);

    let capped = db.rehoming_packet(norm, 1, 80, 5, WALL, &never).await.unwrap();
    assert!(capped.truncated, "the listing stopped and says so");
    assert_eq!(capped.rows.len(), 1);
    assert_eq!(capped.rows[0].org, 2, "the four-mention case, not the one-mention one");
    assert_eq!(capped.open, 2, "the workload still counts both");
    assert_eq!(capped.off_name_mentions, 5, "a cap on the listing must not shrink it");
    assert_eq!(capped.groups_total, 2, "groups are counted over the workload too");
    assert_eq!(capped.probed_groups, 1, "but only the listed case is probed");
}

/// A satellite row whose org was merged away must not be offered as a
/// destination: re-homing onto a row that no longer exists is exactly the
/// dangling reference the apply job's target check refuses, and the packet
/// should never propose it.
#[tokio::test]
async fn a_destination_merged_away_since_the_key_build_is_not_offered() {
    let (db, conn) = seed("test-rehoming-packet-ghost.db").await;
    org(&conn, 1, "DE", "Bietergemeinschaft Dobler / Oberall").await;
    org(&conn, 2, "DE", "Dobler GmbH").await;
    reviewed(&conn, 1, "biege").await;
    mention(&conn, 900, "ORG-1", 1, "Dobler GmbH").await;
    // org_match_keys is a rebuildable snapshot and deliberately has no FK: an
    // R2/R3 merge removes the org row and leaves the key behind until the
    // next build.
    conn.execute("DELETE FROM organizations WHERE id = 2", ()).await.unwrap();

    let never = || false;
    let p = db.rehoming_packet(norm, 150, 80, 5, WALL, &never).await.unwrap();
    assert_eq!(p.open, 1);
    let g = &p.rows[0].groups[0];
    assert!(g.targets.is_empty(), "the join to organizations drops the ghost");
    assert_eq!(g.target_total, 0, "and it is not counted as a destination either");
}

/// Two same-named companies, and the mention publishes an identifier. The
/// digit body is the evidence that tells them apart — the issue-317 peer
/// comparison, reused here for free on rows already in hand — and a match
/// outranks a bigger mention count.
#[tokio::test]
async fn a_matching_identifier_outranks_a_bigger_namesake() {
    let (db, conn) = seed("test-rehoming-packet-ident.db").await;
    org(&conn, 1, "DE", "Bietergemeinschaft Nord / Sued").await;
    // 2 is busy but unrelated; 3 is the one the notice's own number names.
    org_with(&conn, 2, "DE", "Nordbau GmbH", Some("DE999999999")).await;
    org_with(&conn, 3, "DE", "Nordbau GmbH", Some("DE811111111")).await;
    reviewed(&conn, 1, "biege").await;
    mention_id(&conn, 900, "ORG-1", 1, "Nordbau GmbH", Some("DE 811 111 111")).await;
    for n in 910i64..925 {
        mention(&conn, n, "ORG-1", 2, "Nordbau GmbH").await;
    }

    let never = || false;
    let p = db.rehoming_packet(norm, 150, 80, 5, WALL, &never).await.unwrap();
    let g = &p.rows[0].groups[0];
    assert_eq!(g.target_total, 2);
    assert_eq!(g.targets[0].org, 3, "the identifier match wins over fifteen mentions");
    assert!(g.targets[0].identifier_match);
    assert!(!g.targets[1].identifier_match);
    assert_eq!(
        p.rows[0].off_mentions[0].raw_identifier.as_deref(),
        Some("DE 811 111 111"),
        "and the raw value is published, separators and all"
    );
    assert_eq!(
        p.rows[0].off_mentions[0].notice_orgs, 1,
        "notice 900 names one organization — the solo mention that IS the fusion"
    );
}

/// A destination matched through a SATELLITE name is not the name the
/// reviewer reads on it. Saying so costs nothing — both sides are in hand —
/// and not saying it is how a cross-language alias reads as an exact match.
#[tokio::test]
async fn a_destination_matched_through_an_alias_says_so() {
    let (db, conn) = seed("test-rehoming-packet-alias.db").await;
    org(&conn, 1, "DE", "Bietergemeinschaft Rijk / Nord").await;
    org(&conn, 2, "NL", "Rijkswaterstaat").await;
    // The Dutch ministry's German exonym, as the resolver would record it.
    key(&conn, 2, "Reichswasserstaat").await;
    reviewed(&conn, 1, "biege").await;
    mention(&conn, 900, "ORG-1", 1, "Reichswasserstaat").await;

    let never = || false;
    let p = db.rehoming_packet(norm, 150, 80, 5, WALL, &never).await.unwrap();
    let g = &p.rows[0].groups[0];
    assert_eq!(g.targets.len(), 1);
    assert_eq!(g.targets[0].org, 2);
    assert!(g.targets[0].via_alias, "the head name is not the name that matched");
}

/// Beyond the per-case group cap the mentions stay listed — dropping
/// addresses from a packet whose purpose is to supply them would be a silent
/// shrink — but they are flagged, so "no destination" and "not probed" do not
/// read alike.
#[tokio::test]
async fn an_elided_group_keeps_its_addresses_and_flags_them() {
    let (db, conn) = seed("test-rehoming-packet-elide.db").await;
    org(&conn, 1, "DE", "Bietergemeinschaft Shared / Literal").await;
    reviewed(&conn, 1, "biege").await;
    // Twenty distinct member names on one row: the shared-literal shape the
    // per-case cap exists for.
    for i in 0i64..20 {
        mention(&conn, 900 + i, "ORG-1", 1, &format!("Member Number {i}")).await;
    }

    let never = || false;
    let p = db.rehoming_packet(norm, 150, 80, 5, WALL, &never).await.unwrap();
    assert_eq!(p.off_name_mentions, 20);
    assert_eq!(p.groups_total, 20, "the workload's groups are counted whole");
    assert_eq!(p.probed_groups, 12, "the per-case cap bounds only what is probed");
    assert_eq!(p.groups_elided, 8);
    let c = &p.rows[0];
    assert_eq!(c.groups.len(), 12);
    assert_eq!(c.groups_elided, 8, "and the case says which case elided them");
    assert_eq!(c.off_mentions.len(), 20, "every address is still here");
    assert_eq!(c.off_mentions.iter().filter(|m| m.group_shown).count(), 12);
    assert_eq!(c.off_mentions.iter().filter(|m| !m.group_shown).count(), 8);
}

/// The discriminator a name and a count cannot supply: a notice that names
/// one organization and calls it by the member's name is the fusion proper;
/// one that names seven is a consortium listing, where the vehicle under a
/// longer spelling is the likelier reading and re-homing would be an error.
#[tokio::test]
async fn each_listed_mention_says_how_crowded_its_notice_is() {
    let (db, conn) = seed("test-rehoming-packet-crowd.db").await;
    org(&conn, 1, "DE", "Bietergemeinschaft Dobler / Oberall").await;
    org(&conn, 2, "DE", "Dobler GmbH").await;
    reviewed(&conn, 1, "biege").await;
    // Notice 900 names the vehicle alone, under the member's name.
    mention(&conn, 900, "ORG-1", 1, "Dobler GmbH").await;
    // Notice 901 is a full award listing: five organizations, one of which
    // happens to be this row.
    mention(&conn, 901, "ORG-1", 1, "Dobler GmbH").await;
    for section in ["ORG-2", "ORG-3", "ORG-4", "ORG-5"] {
        mention(&conn, 901, section, 2, "Somebody Else GmbH").await;
    }

    let never = || false;
    let p = db.rehoming_packet(norm, 150, 80, 5, WALL, &never).await.unwrap();
    let by = |n: i64| {
        p.rows[0]
            .off_mentions
            .iter()
            .find(|m| m.notice_id == n)
            .unwrap_or_else(|| panic!("notice {n} is listed"))
    };
    assert_eq!(by(900).notice_orgs, 1, "the solo mention");
    assert_eq!(by(901).notice_orgs, 5, "the crowded one, and the reviewer can see it");
}
