//! Issue 300 Stage 2: `match_org_identifiers_r2` — same-country canonical-key
//! groups merge through the full denial stack, with the recorded-plan parity
//! guard and restart-safe re-runs. The injected rules mirror the production
//! wiring's SHAPE (fn pointers) with test-local implementations for the name
//! vetoes, so this test pins the store machinery; the real rule content is
//! pinned by `ingest::crosswalk`'s own tests.

use store::turso::Value;

fn key(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool)> {
    // A miniature crosswalk: FI vat/national 8-digit unify; SK vat/national
    // 10-digit unify (dic); SK 8-digit is its own scheme (ico); FR 14-digit
    // truncates to 9. Everything else: no key.
    let norm: String =
        value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    let (cc, body) = if kind == "vat" {
        (norm.get(..2)?.to_owned(), norm.get(2..)?.to_owned())
    } else {
        (country?.to_owned(), norm)
    };
    let digits = body.bytes().all(|b| b.is_ascii_digit());
    match (cc.as_str(), body.len()) {
        ("FI", 8) if digits => Some(("FI:ytunnus", body, true)),
        ("SK", 10) if digits => Some(("SK:dic", body, true)),
        ("SK", 8) if digits && kind != "vat" => Some(("SK:ico", body, true)),
        ("FR", 9) if digits => Some(("FR:siren", body, true)),
        ("FR", 14) if digits => Some(("FR:siren", body[..9].to_owned(), true)),
        _ => None,
    }
}

fn condemns(_c: Option<&str>, _k: &str, v: &str) -> bool {
    v.contains("11111111")
}

fn consortium(name: &str) -> bool {
    name.to_lowercase().contains("groupement")
}

fn legal_form(name: &str) -> Option<&'static str> {
    let l = name.to_lowercase();
    if l.ends_with("gmbh") {
        Some("gmbh")
    } else if l.ends_with(" ag") {
        Some("ag")
    } else {
        None
    }
}

async fn seed(path: &str) -> (store::Db, store::turso::Connection) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn.execute("BEGIN", ()).await.unwrap();
    // (id, country, kind, identifier, name, provisional)
    let orgs: Vec<(i64, &str, &str, &str, &str, i64)> = vec![
        // The clean twin: non-provisional survivor.
        (10, "FI", "national", "01003158", "Telinekataja Oy", 0),
        (11, "FI", "vat", "FI01003158", "Telinekataja", 1),
        // Both provisional: min id survives.
        (12, "FI", "national", "01011975", "Ramboll Oy", 1),
        (13, "FI", "vat", "FI01011975", "Ramboll", 1),
        // Member-scoped consortium veto: the groupement (21) is EXCLUDED and
        // the two establishment rows (20, 22) merge without it.
        (20, "FR", "national", "18001404501577", "CNFPT Etablissement", 0),
        (21, "FR", "national", "180014045", "groupement CNFPT / X", 1),
        (22, "FR", "national", "18001404502245", "CNFPT Etablissement 2", 1),
        // …and a pair where exclusion leaves fewer than two: whole-group deny.
        (24, "FR", "national", "999888777", "groupement solo / x", 1),
        (25, "FR", "national", "99988877700011", "Solo Est", 1),
        // Legal-form veto (names carry conflicting families).
        (30, "FI", "national", "10773381", "Alpha GmbH", 1),
        (31, "FI", "vat", "FI10773381", "Alpha AG", 1),
        // Gate-poison: the pair keys cleanly but the injected gate condemns
        // the identifier — the whole group is disqualified.
        (35, "FI", "national", "11111111", "Poisoned", 1),
        (36, "FI", "vat", "FI11111111", "Poisoned Twin", 1),
        // VAT-group wall: two members share the group DIČ, mention evidence
        // carries their own DIFFERENT IČOs.
        (50, "SK", "vat", "SK2021005448", "Member One", 1),
        (51, "SK", "national", "2021005448", "Member Two", 1),
    ];
    for (id, c, k, v, n, p) in &orgs {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, 0)",
            (
                Value::Integer(*id),
                Value::Text((*c).into()),
                Value::Text((*k).into()),
                Value::Text((*v).into()),
                Value::Text((*n).into()),
                Value::Text(n.to_lowercase()),
                Value::Integer(*p),
            ),
        )
        .await
        .unwrap();
    }
    // The literal-cap group: nine FI orgs all publishing ONE literal id.
    for i in 0..9i64 {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, 'FI', 'national', '20445111', ?, ?, 1, 0)",
            (
                Value::Integer(40 + i),
                Value::Text(format!("Stranger {i}")),
                Value::Text(format!("stranger {i}")),
            ),
        )
        .await
        .unwrap();
    }
    // Mention evidence for the wall — the MASK shape a verifier caught: both
    // members' mentions carry the SHARED group IČ-DPH (the very value the
    // group formed on), which must be stripped as evidence, plus each
    // member's OWN DIČ in the SAME scheme. Without the strip the shared
    // value makes the sets intersect and the conflict never denies.
    for (nid, org, raw_id) in [
        (900i64, 50i64, "SK2021005448"),
        (901, 50, "2020000001"),
        (902, 51, "SK2021005448"),
        (903, 51, "2020000002"),
    ] {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
             VALUES (?, 'S-1', ?, 'x', 'SK', ?)",
            (Value::Integer(nid), Value::Integer(org), Value::Text(raw_id.into())),
        )
        .await
        .unwrap();
    }
    // Rows to repoint: a party on loser 11, a winner row on loser 13.
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, publication_id, published_at)
         VALUES (1, 1, 100, 'pub-1-1', 0)",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (1, 1, 'buyer', 11, 100, 'S-1')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id)
         VALUES (1, 1, 77, 13)",
        (),
    )
    .await
    .unwrap();
    conn.execute("COMMIT", ()).await.unwrap();
    (db, conn)
}

async fn count(conn: &store::turso::Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let store::turso::Value::Integer(n) = row.get_value(0).unwrap() else { panic!("count") };
    n
}

fn args(dry_run: bool, expect_groups: Option<u64>) -> store::R2MergeArgs<'static> {
    store::R2MergeArgs {
        key,
        condemns,
        consortium,
        legal_form,
        rule: "r2",
        n3: |n| n.to_lowercase(),
        stoplist_cap: 20,
        dry_run,
        max_groups: None,
        expect_groups,
        job_id: Some(7),
        stop: &|| false,
    }
}

#[tokio::test]
async fn the_r2_merge_applies_the_denial_stack_and_merges_the_plan() {
    let (db, conn) = seed("test-r2-merge.db").await;

    // Dry-run: the full plan, nothing written.
    let dry = db.match_org_identifiers_r2(args(true, None)).await.expect("dry");
    assert_eq!(
        dry.groups, 8,
        "01003158, 01011975, 180014045, 999888777, 10773381, 11111111, 20445111, 2021005448"
    );
    assert_eq!(
        (dry.denied_cap, dry.denied_gate, dry.denied_consortium, dry.denied_legal_form, dry.denied_group_vat),
        (1, 1, 1, 1, 1),
        "every denial class fires exactly once (consortium: the remainder-below-two pair)"
    );
    assert_eq!(dry.consortium_excluded, 2, "both groupement members were excluded");
    assert_eq!(
        dry.plan_groups, 3,
        "the two FI twins plus the CNFPT establishments minus their groupement"
    );
    assert_eq!(dry.merged_groups, 0);
    assert_eq!(
        (dry.mentions, dry.parties, dry.bid_parties, dry.winners),
        (0, 1, 0, 1),
        "the preview reports the losers' blast radius (the Stage-1 lesson)"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 24, "dry run wrote nothing");

    // The parity guard: a wet run whose recorded plan disagrees aborts.
    let err = db.match_org_identifiers_r2(args(false, Some(4000))).await;
    assert!(err.is_err(), "divergence beyond tolerance must abort");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 24, "abort wrote nothing");

    // Wet run under the recorded plan.
    let wet = db.match_org_identifiers_r2(args(false, Some(3))).await.expect("wet");
    assert_eq!((wet.plan_groups, wet.merged_groups, wet.removed), (3, 3, 3));
    // Survivors: 10 (non-provisional beats min id 10<11 anyway), 12 (min id),
    // and 20 (non-provisional) — with 22 its loser and 21 left STANDING.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id IN (11, 13, 22)").await, 0);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id IN (10, 12, 20, 21)").await, 4);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE keep = 20 AND loser = 22").await,
        1,
        "the establishments merged around their excluded groupement"
    );
    // The rows followed their orgs.
    assert_eq!(
        count(&conn, "SELECT organization_id FROM tender_version_parties WHERE tender_id = 1").await,
        10
    );
    assert_eq!(
        count(&conn, "SELECT organization_id FROM tender_version_result_winners WHERE tender_id = 1").await,
        12
    );
    assert_eq!(wet.tender_changes, 1, "tender 1 was touched by both repoints");
    // The merge log records every merge, auditable and targetable.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE rule = 'r2'").await, 3);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE keep = 10 AND loser = 11").await,
        1
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE keep = 12 AND loser = 13").await,
        1
    );
    // Change feed: losers removed, keeps changed, the tender changed.
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 11 AND op = 'removed'").await,
        1
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 10 AND op = 'changed'").await,
        1
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'tender' AND entity_id = 1 AND op = 'changed'").await,
        1
    );
    // The denied groups stand untouched: the strangers, the solo-groupement
    // pair, the legal-form pair, the poisoned pair, the SK wall pair.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id >= 40 AND id < 49").await, 9);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id IN (21, 24, 25, 30, 31, 35, 36, 50, 51)").await, 9);

    // Restart safety: merged groups are singletons; a re-run plans nothing.
    let again = db.match_org_identifiers_r2(args(false, None)).await.expect("rerun");
    assert_eq!((again.plan_groups, again.merged_groups, again.removed), (0, 0, 0));
}

/// Issue 326: the plan must be READABLE, not merely sampled.
///
/// `plan_sample` is a fixed 1-in-199 content-stable acceptance. That is the
/// right shape for estimating precision over a huge plan and the wrong shape for
/// reading a small one: on prod's 200-group plan it yielded exactly ONE row, so
/// a 200-group merge could not be reviewed at all — which is why the 297
/// duplicate identities the issue-326 repair created on purpose sat unfolded.
///
/// So a capped LISTING rides alongside the sample. Complete whenever the plan
/// fits the cap, with a flag when it does not, and the sample keeps its own
/// unbiased-over-large-plans guarantee untouched.
#[tokio::test]
async fn the_dry_plan_is_listed_in_full_not_just_sampled() {
    let (db, conn) = seed("/tmp/tender-db-r2-listing").await;
    let r = db.match_org_identifiers_r2(args(true, None)).await.unwrap();

    assert!(r.plan_groups > 0, "the fixture plans something");
    assert_eq!(
        r.plan_listing.len() as u64,
        r.plan_groups,
        "every planned group is listed: {} groups, {} listed",
        r.plan_groups,
        r.plan_listing.len()
    );
    assert!(!r.plan_listing_truncated, "and the listing is complete");

    // The listing carries what a reviewer needs to judge a merge: the identity
    // being merged onto, and every member with its own literal and name.
    let (country, scheme, key, members) = &r.plan_listing[0];
    assert!(!country.is_empty() && !scheme.is_empty() && !key.is_empty());
    assert!(members.len() >= 2, "a group is two or more rows");
    for (org_id, kind, literal, _name) in members {
        assert!(*org_id > 0);
        assert!(!kind.is_empty());
        assert!(!literal.is_empty());
    }

    // The 1-in-199 sample is a DIFFERENT thing and stays as it was: on a plan
    // this small it is expected to be empty, and that is precisely the gap the
    // listing closes rather than a defect in the sample.
    assert!(
        r.plan_sample.len() <= r.plan_listing.len(),
        "the sample never exceeds the listing"
    );

    // A WET run records no listing — it is a review artifact for a plan that has
    // not been applied yet, and a wet re-record must not masquerade as one.
    let w = db.match_org_identifiers_r2(args(false, None)).await.unwrap();
    assert!(w.plan_listing.is_empty(), "a wet run lists nothing");
    assert!(!w.plan_listing_truncated);
    let _ = count(&conn, "SELECT COUNT(*) FROM organizations").await;
}
