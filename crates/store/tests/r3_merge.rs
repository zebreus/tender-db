//! Issue 300 Stage 3: `match_org_null_country_r3` — NULL-country identifier
//! orgs rescued by unique checksum anchor + standing-target lookup + exact
//! cross-language N2 corroboration, hardened with the R2 denial stack. The
//! injected rules mirror the production wiring's SHAPE (fn pointers) with
//! test-local implementations, so this test pins the store machinery; the
//! real rule content is pinned by `ingest::crosswalk`/`idgate`'s own tests.

use store::turso::Value;

fn key(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool)> {
    // A miniature crosswalk: FR 9-digit siren, SK 10-digit dic (vat and
    // national unify). Everything else: no key.
    let norm: String =
        value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    let (cc, body) = if kind == "vat" {
        (norm.get(..2)?.to_owned(), norm.get(2..)?.to_owned())
    } else {
        (country?.to_owned(), norm)
    };
    let digits = body.bytes().all(|b| b.is_ascii_digit());
    match (cc.as_str(), body.len()) {
        ("FR", 9) if digits => Some(("FR:siren", body, true)),
        ("SK", 10) if digits => Some(("SK:dic", body, true)),
        _ => None,
    }
}

fn anchors(v: &str) -> Vec<(&'static str, String)> {
    // A miniature checksum probe: length decides. 8-digit always carries the
    // ambiguity-by-construction marker (the DK|SI rule), 11-digit satisfies
    // TWO real schemes — both must classify as unanchored.
    if v.bytes().any(|b| b.is_ascii_alphabetic()) || v.is_empty() {
        return Vec::new();
    }
    match v.len() {
        8 => vec![("FI:ytunnus", v.to_owned()), ("DK|SI:8-digit", v.to_owned())],
        9 => vec![("FR:siren", v.to_owned())],
        10 => vec![("SK:dic", v.to_owned())],
        11 => vec![("FR:siren", v[..9].to_owned()), ("SK:dic", v[..10].to_owned())],
        _ => Vec::new(),
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

fn norm(s: &str) -> String {
    s.to_lowercase().chars().filter(char::is_ascii_alphanumeric).collect()
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
    // Standing (country-ful) targets: (id, country, kind, identifier, name).
    let targets: Vec<(i64, &str, &str, &str, &str)> = vec![
        // TWO standing rows share one SK:dic key — a candidate anchoring
        // there is ambiguous (multi_target), exactly the family R2 declined.
        (12, "SK", "national", "2021005448", "Duo One"),
        (13, "SK", "vat", "SK2021005448", "Duo One B"),
        // Target whose candidate's name will NOT corroborate.
        (14, "FR", "national", "552100554", "Colas Nord"),
        // The clean rescue target.
        (15, "FR", "national", "732829320", "Renault SA"),
        // Gate-poisoned identifier (contains 11111111).
        (16, "FR", "national", "111111119", "Poison Target"),
        // Consortium-named target.
        (17, "FR", "national", "444555666", "Groupement Alpha"),
        // The wall pair's keep side.
        (18, "FR", "national", "777888999", "Wall Corp"),
        // Legal-form trap: the head is a GmbH, but a SATELLITE name (seeded
        // below) matches the candidate's AG name exactly — corroboration
        // rides the satellite across the family conflict.
        (19, "FR", "national", "888999000", "Beta GmbH"),
        // The stamped-literal keep: nine candidates share its key.
        (23, "FR", "national", "555666777", "Stamp Co"),
        // The co-anchored pairwise-wall keep: no mentions of its own.
        (26, "FR", "national", "111222333", "Twin Hold"),
    ];
    for (id, c, k, v, n) in &targets {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, ?, ?, ?, ?, ?, 0, 0)",
            (
                Value::Integer(*id),
                Value::Text((*c).into()),
                Value::Text((*k).into()),
                Value::Text((*v).into()),
                Value::Text((*n).into()),
                Value::Text(n.to_lowercase()),
            ),
        )
        .await
        .unwrap();
    }
    // The NULL-country pool: one candidate per classification rung.
    let pool: Vec<(i64, &str, &str)> = vec![
        (100, "HRB123456", "Reg GmbH"),          // register-prefixed
        (101, "01003158", "Eight Digit Oy"),     // 8-digit: marker => never unique
        (102, "12345678901", "Two Anchor Corp"), // two real anchors: ambiguous
        (103, "999000111", "No Target SA"),      // unique anchor, no standing row
        (104, "2021005448", "Duo One"),          // unique anchor, TWO standing rows
        (105, "552100554", "Different Name"),    // target stands, name mismatch
        (106, "111111119", "Poison Target"),     // corroborates, gate condemns
        (107, "444555666", "Groupement Alpha"),  // corroborates, consortium veto
        (108, "777888999", "Wall Corp"),         // corroborates, wall denies
        (109, "732829320", "RENAULT SA"),        // the plan: merges into 15
        (110, "888999000", "Beta AG"),           // satellite corroborates, family differs
        (130, "111222333", "Twin Hold"),         // co-anchored pair with conflicting
        (131, "111222333", "Twin Hold"),         // evidence: pairwise wall drops both
    ];
    for (id, v, n) in &pool {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, NULL, 'national', ?, ?, ?, 1, 0)",
            (
                Value::Integer(*id),
                Value::Text((*v).into()),
                Value::Text((*n).into()),
                Value::Text(n.to_lowercase()),
            ),
        )
        .await
        .unwrap();
    }
    // The stamped-literal class: NINE candidates all publishing target 23's
    // key with corroborating names — the co-anchor cap must drop the group.
    for i in 0..9i64 {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, NULL, 'national', '555666777', 'Stamp Co', 'stamp co', 1, 0)",
            (Value::Integer(120 + i),),
        )
        .await
        .unwrap();
    }
    // Target 19's satellite: the exact name the candidate carries, so
    // corroboration passes while the HEAD families conflict.
    conn.execute(
        "INSERT INTO organization_names (org_id, lang, name, name_norm)
         VALUES (19, 'de', 'Beta AG', 'beta ag')",
        (),
    )
    .await
    .unwrap();
    // Wall evidence — the MASK shape (the R2 verifier's F1 catch, pinned
    // here too): BOTH sides' mentions carry the pair-forming key itself,
    // which must be stripped as evidence, plus each side's OWN different
    // register number in the SAME scheme. Without the strip the shared value
    // makes the sets intersect and the conflict never denies. The mention
    // COUNTRY is NULL on every row: keying rides the fallback (the
    // verification round's candidate-side disarm fix — the candidate falls
    // back to the TARGET's country, the target to its own).
    for (nid, org, raw_id) in [
        (904i64, 108i64, "777888999"),
        (905, 108, "123456782"),
        (906, 18, "777888999"),
        (907, 18, "987654329"),
        // The co-anchored pair's conflicting evidence (clean vs their
        // mention-less target, dirty against each other).
        (908, 130, "444000111"),
        (909, 131, "444000222"),
    ] {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
             VALUES (?, 'S-1', ?, 'x', NULL, ?)",
            (Value::Integer(nid), Value::Integer(org), Value::Text(raw_id.into())),
        )
        .await
        .unwrap();
    }
    // The rescue candidate's blast radius: a mention, a party, a winner row.
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
         VALUES (910, 'S-1', 109, 'RENAULT SA', NULL, NULL)",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, caused_by_notice_id, publication_id, published_at)
         VALUES (2, 1, 910, 'pub-2-1', 0)",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (2, 1, 'winner', 109, 910, 'S-1')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_result_winners (tender_id, seq, lot_result_id, organization_id)
         VALUES (2, 1, 88, 109)",
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

fn args(dry_run: bool, expect_groups: Option<u64>) -> store::R3MergeArgs<'static> {
    store::R3MergeArgs {
        key,
        anchors,
        condemns,
        consortium,
        legal_form,
        norm,
        dry_run,
        max_groups: None,
        expect_groups,
        job_id: Some(9),
        stop: &|| false,
    }
}

#[tokio::test]
async fn the_r3_merge_classifies_the_pool_and_merges_the_anchored_survivors() {
    let (db, conn) = seed("test-r3-merge.db").await;

    // Dry-run: the census ladder recomputed, nothing written.
    let dry = db.match_org_null_country_r3(args(true, None)).await.expect("dry");
    assert_eq!(dry.pool, 22);
    assert_eq!(
        (dry.register_prefixed, dry.unanchored, dry.no_target, dry.multi_target, dry.uncorroborated),
        (1, 2, 1, 1, 1),
        "each census rung fires once (unanchored: the 8-digit marker AND the two-anchor value)"
    );
    assert_eq!(
        (dry.denied_gate, dry.denied_consortium, dry.denied_legal_form),
        (1, 1, 1),
        "gate, consortium, and the satellite-corroborated family conflict each fire once"
    );
    assert_eq!(
        dry.denied_group_vat, 3,
        "the candidate-vs-target wall (key stripped, NULL mention country keyed via \
         the target-country fallback) plus the co-anchored pair's pairwise conflict"
    );
    assert_eq!(dry.denied_cap, 9, "the stamped-literal group falls to the co-anchor cap");
    assert_eq!(dry.plan_groups, 1, "only the corroborated clean candidate survives");
    assert_eq!(dry.merged_groups, 0);
    assert_eq!(
        (dry.mentions, dry.parties, dry.bid_parties, dry.winners),
        (1, 1, 0, 1),
        "the preview reports the FINAL plan's blast radius only"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 32, "dry run wrote nothing");

    // The parity guard: a wet run whose recorded plan disagrees aborts.
    let err = db.match_org_null_country_r3(args(false, Some(400))).await;
    assert!(err.is_err(), "divergence beyond tolerance must abort");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 32, "abort wrote nothing");

    // Wet run under the recorded plan.
    let wet = db.match_org_null_country_r3(args(false, Some(1))).await.expect("wet");
    assert_eq!((wet.plan_groups, wet.merged_groups, wet.removed), (1, 1, 1));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 109").await, 0);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 15").await, 1);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE rule = 'r3' AND keep = 15 AND loser = 109").await,
        1
    );
    // The rows followed the org.
    assert_eq!(
        count(&conn, "SELECT organization_id FROM tender_version_parties WHERE tender_id = 2").await,
        15
    );
    assert_eq!(
        count(&conn, "SELECT organization_id FROM tender_version_result_winners WHERE tender_id = 2").await,
        15
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 15").await,
        1
    );
    assert_eq!(wet.tender_changes, 1);
    // Change feed: loser removed, keep changed, the tender changed.
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 109 AND op = 'removed'").await,
        1
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 15 AND op = 'changed'").await,
        1
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'tender' AND entity_id = 2 AND op = 'changed'").await,
        1
    );
    // Every skipped candidate stands untouched — the census rungs, the
    // denials, the family conflict, the stamped nine, the co-anchored pair.
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organizations WHERE id >= 100 AND id <= 131 AND id <> 109").await,
        21
    );

    // Restart safety: the merged candidate is gone; a re-run plans nothing.
    let again = db.match_org_null_country_r3(args(false, None)).await.expect("rerun");
    assert_eq!((again.plan_groups, again.merged_groups, again.removed), (0, 0, 0));
    assert_eq!(again.pool, 21);
}
