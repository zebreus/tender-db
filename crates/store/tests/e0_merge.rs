//! Issue 329: the E0 fold — exact `(country, kind, identifier)` groups that no
//! cross-walk arm keys, merged through the R2 machinery under one extra rule:
//! the named members must agree on ONE non-generic N3 key. What this pins is
//! the store side — grouping by the injected E0 key, the name rule's denial,
//! the ledger's rule column — with test-local rule implementations.

use store::turso::Value;

/// A miniature cross-walk: FI 8-digit keys (so an FI pair is R2's, never E0's).
fn arm(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool, bool)> {
    let norm: String =
        value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    let (cc, body) = if kind == "vat" {
        (norm.get(..2)?.to_owned(), norm.get(2..)?.to_owned())
    } else {
        (country?.to_owned(), norm)
    };
    match (cc.as_str(), body.len()) {
        ("FI", 8) if body.bytes().all(|b| b.is_ascii_digit()) => Some(("FI:ytunnus", body, true, false)),
        _ => None,
    }
}

/// The E0 key, as `ingest::crosswalk::e0_key_flat` shapes it.
fn e0_key(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool, bool)> {
    if arm(country, kind, value).is_some() {
        return None;
    }
    let norm: String =
        value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    (!norm.is_empty()).then(|| ("E0", format!("{kind}:{norm}"), true, false))
}

fn never_condemns(_c: Option<&str>, _k: &str, _v: &str) -> bool {
    false
}
fn no_consortium(_name: &str) -> bool {
    false
}
fn no_legal_form(_name: &str) -> Option<&'static str> {
    None
}
fn n3(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

async fn seed(path: &str) -> (store::Db, store::turso::Connection) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let raw = store::turso::Builder::new_local(path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    // (id, country, kind, identifier, name, provisional)
    let orgs: Vec<(i64, &str, &str, &str, &str, i64)> = vec![
        // The Greek authority (311/1079): same code, one name modulo case — merges.
        (60, "GR", "national", "1000E009610001", "Single Authority", 0),
        (61, "GR", "national", "1000E009610001", "SINGLE   AUTHORITY", 1),
        // Same code, different names (a ministry and its directorate) — denied.
        (62, "GR", "national", "X7Y8Z9", "Alpha", 1),
        (63, "GR", "national", "X7Y8Z9", "Beta", 1),
        // An FI pair the arm keys — R2's group, invisible to E0.
        (64, "FI", "national", "01003158", "Kataja", 1),
        (65, "FI", "national", "01003158", "Kataja", 1),
        // A VAT row and a national row sharing a literal — never one E0 group.
        (66, "DE", "vat", "DEQ1", "Same literal", 1),
        (67, "DE", "national", "DEQ1", "Same literal", 1),
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
    // One mention on the row that will lose, so the repoint is observable.
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
         VALUES (900, 'S-1', 61, 'x', 'GR', '1000.E00961.0001')",
        (),
    )
    .await
    .unwrap();
    (db, conn)
}

fn args(dry_run: bool, expect_groups: Option<u64>) -> store::R2MergeArgs<'static> {
    store::R2MergeArgs {
        key: e0_key,
        name_key: |n| n.to_lowercase(),
        names_agree: |a, b| a == b,
        trim: |n| n.to_owned(),
        norm: |n| n.to_lowercase(),
        condemns: never_condemns,
        consortium: no_consortium,
        legal_form: no_legal_form,
        rule: "e0",
        n3,
        stoplist_cap: 20,
        dry_run,
        max_groups: None,
        expect_groups,
        register_identity: None,
        expect_listing: None,
        job_id: Some(9),
        stop: &|| false,
    }
}

async fn count(conn: &store::turso::Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let Value::Integer(n) = row.get_value(0).unwrap() else { panic!("count") };
    n
}

#[tokio::test]
async fn the_e0_fold_merges_agreeing_distinctive_names_and_denies_the_rest() {
    let (db, conn) = seed("/tmp/tender-db-e0-merge.db").await;

    let dry = db.match_org_identifiers_r2(args(true, None)).await.expect("dry");
    assert_eq!(dry.groups, 2, "the two GR triples; the FI pair is the arm's, the DE literal splits by kind");
    assert_eq!(dry.denied_names, 1, "Alpha / Beta disagree");
    assert_eq!(dry.plan_groups, 1);
    assert_eq!(dry.merged_groups, 0, "a dry run writes nothing");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 8);

    assert!(db.foreign_keys_enabled().await.unwrap(), "issue 352: the writer enforces foreign keys before the wet run");

    let wet = db.match_org_identifiers_r2(args(false, Some(1))).await.expect("wet");

    assert!(db.foreign_keys_enabled().await.unwrap(), "and after it: the wet loop never turns them off (issue 442 retired 352's bracket)");
    assert_eq!(wet.merged_groups, 1);
    assert_eq!(wet.removed, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 61").await, 0, "the provisional twin lost");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 60").await, 1);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 60").await,
        1,
        "the loser's mention now points at the keep"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE loser = 61 AND keep = 60 AND rule = 'e0'").await,
        1,
        "the ledger names the rule"
    );
    // The disagreeing pair still stands.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE identifier = 'X7Y8Z9'").await, 2);
}

/// Issue 349: the name wall's carrier count is not one thing. `Stadt Echo`
/// carries its key on 27 org rows, 25 of them the city's own identifier-less
/// provisional rows — an echo, admitted; `Gemeinde Shared` carries it on 27
/// rows that each hold their own VAT — a shared name, denied.
#[tokio::test]
async fn an_echo_generic_name_is_admitted_and_a_shared_one_denied() {
    let path = "/tmp/tender-db-e0-echo.db";
    let (db, conn) = seed(path).await;
    async fn key_row(conn: &store::turso::Connection, id: i64, key: &str) {
        conn.execute(
            "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', ?)",
            (Value::Integer(id), Value::Text(key.into())),
        )
        .await
        .unwrap();
    }
    async fn org(conn: &store::turso::Connection, id: i64, cc: Option<&str>, vat: Option<&str>, name: &str) {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, 0)",
            (
                Value::Integer(id),
                cc.map(|c| Value::Text(c.into())).unwrap_or(Value::Null),
                vat.map(|_| Value::Text("vat".into())).unwrap_or(Value::Null),
                vat.map(|v| Value::Text(v.into())).unwrap_or(Value::Null),
                Value::Text(name.into()),
                Value::Text(name.to_lowercase()),
                Value::Integer(if vat.is_some() { 0 } else { 1 }),
            ),
        )
        .await
        .unwrap();
    }
    // The echo: the pair, plus 25 NULL/NULL provisional rows of the same city.
    org(&conn, 68, Some("DE"), Some("DEECHO1"), "Stadt Echo").await;
    org(&conn, 69, Some("DE"), Some("DEECHO1"), "Stadt Echo").await;
    for id in [68, 69] {
        key_row(&conn, id, "stadt echo").await;
    }
    for i in 0..25i64 {
        org(&conn, 1000 + i, None, None, "Stadt Echo").await;
        key_row(&conn, 1000 + i, "stadt echo").await;
    }
    // The shared name: the pair, plus 25 other companies with their own VATs.
    org(&conn, 70, Some("DE"), Some("DESHARED1"), "Gemeinde Shared").await;
    org(&conn, 71, Some("DE"), Some("DESHARED1"), "Gemeinde Shared").await;
    for id in [70, 71] {
        key_row(&conn, id, "gemeinde shared").await;
    }
    for i in 0..25i64 {
        org(&conn, 2000 + i, Some("DE"), Some(&format!("DEOTHER{i:03}")), "Gemeinde Shared").await;
        key_row(&conn, 2000 + i, "gemeinde shared").await;
    }
    let dry = db.match_org_identifiers_r2(args(true, None)).await.expect("dry");
    assert_eq!(dry.groups, 4, "the seed's two GR triples plus the echo and the shared pair");
    assert_eq!(dry.admitted_echo, 1, "Stadt Echo: over the wall by rows, under it by identified rows");
    assert_eq!(dry.denied_names, 2, "Alpha / Beta disagree; Gemeinde Shared is a shared name");
    assert_eq!(dry.plan_groups, 2);
    let wet = db.match_org_identifiers_r2(args(false, Some(2))).await.expect("wet");
    assert_eq!(wet.merged_groups, 2);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE identifier = 'DEECHO1'").await, 1, "the echo pair folded");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE identifier = 'DESHARED1'").await, 2, "the shared pair stands");
}


// ---- Issue 514 unit 2: register-spelling reunions under E0 (decision §2.3, §6) ----

/// A miniature `crosswalk::registry_mention_key`: under GB, a UKPRN literal — bare or
/// behind repeated `UKPRN` / `GBUKPRN` labels, as the 514 rewrite reads it — keys to its
/// identity; a PPON, or anything outside GB, keys nothing.
fn registry_identity(country: Option<&str>, raw: &str) -> Option<(&'static str, String)> {
    if country != Some("GB") {
        return None;
    }
    let norm: String = raw.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    let mut rest = norm.strip_prefix("GBUKPRN")?;
    while let Some(r) = rest.strip_prefix("GBUKPRN").or_else(|| rest.strip_prefix("UKPRN")) {
        rest = r;
    }
    (rest.len() == 8 && rest.starts_with('1') && rest.bytes().all(|b| b.is_ascii_digit()))
        .then(|| ("GB:ukprn", format!("GBUKPRN{rest}")))
}

fn args_514(dry_run: bool, expect_groups: Option<u64>) -> store::R2MergeArgs<'static> {
    store::R2MergeArgs { register_identity: Some(registry_identity), ..args(dry_run, expect_groups) }
}

async fn fresh(name: &str) -> (store::Db, store::turso::Connection) {
    let path = format!("/tmp/tender-db-e0-514-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = store::turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (db, conn)
}

/// One org under `country`, `national`, with one mention per published raw.
async fn reg_org(conn: &store::turso::Connection, id: i64, country: &str, identifier: &str, name: &str, raws: &[&str]) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (?, ?, 'national', ?, ?, ?, 0, 0)",
        (
            Value::Integer(id),
            Value::Text(country.into()),
            Value::Text(identifier.into()),
            Value::Text(name.into()),
            Value::Text(name.to_lowercase()),
        ),
    )
    .await
    .unwrap();
    for (i, raw) in raws.iter().enumerate() {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
             VALUES (?, 'S-1', ?, ?, ?, ?)",
            (
                Value::Integer(id * 100 + i as i64),
                Value::Integer(id),
                Value::Text(name.into()),
                Value::Text(country.into()),
                Value::Text((*raw).into()),
            ),
        )
        .await
        .unwrap();
    }
}

/// The 514 reunion after the 345 re-key: the twin publishes `GB-UKPRN-10007798`, the
/// moved row still carries its `GB-UKPRN-UKPRN 10007798` mentions.
async fn reunion(conn: &store::turso::Connection, keep: i64, mover: i64, name: &str) {
    reg_org(conn, keep, "GB", "GBUKPRN10007798", name, &["GB-UKPRN-10007798"]).await;
    reg_org(conn, mover, "GB", "GBUKPRN10007798", name, &["GB-UKPRN-UKPRN 10007798", "GB-UKPRN-UKPRN10007798"]).await;
}

fn verdict(key: &str, members: Vec<i64>, action: &str, confidence: &str) -> store::MergeVerdict {
    store::MergeVerdict {
        country: "GB".into(),
        scheme: "E0".into(),
        key: key.into(),
        members,
        action: action.into(),
        rationale: "issue 514 test".into(),
        confidence: confidence.into(),
        reviewed_holders: None,
    }
}

/// Measure-first #1, kept as the first half: with no `register_identity` (today's wall)
/// the reunion is walled — `vat:GBUKPRN10007798` against `vat:GBUKPRNUKPRN10007798` in
/// the one E0 bucket — and nothing a verdict says can lift it. With the strip, the
/// group's own identity is no evidence, the group plans, and the wet run merges it.
#[tokio::test]
async fn a_register_spelling_reunion_is_walled_today_and_passes_on_its_own_identity() {
    let (db, conn) = fresh("reunion").await;
    reunion(&conn, 100, 101, "Leeds City College").await;

    let today = db.match_org_identifiers_r2(args(true, None)).await.expect("dry");
    assert_eq!((today.groups, today.denied_group_vat, today.plan_groups), (1, 1, 0), "{today:?}");
    assert_eq!((today.wall_strip_admitted, today.wall_strip_denied), (0, 0), "no strip, no footprint");

    let dry = db.match_org_identifiers_r2(args_514(true, None)).await.expect("dry");
    assert_eq!((dry.denied_group_vat, dry.plan_groups), (0, 1), "{dry:?}");
    assert_eq!((dry.wall_strip_admitted, dry.wall_strip_denied), (1, 0), "S9a: the strip admitted exactly this group");
    let listed: Vec<(&str, &str, &str, Vec<i64>)> =
        dry.merge_listing.iter().map(|(c, s, k, m)| (c.as_str(), *s, k.as_str(), m.clone())).collect();
    assert_eq!(listed, vec![("GB", "E0", "national:GBUKPRN10007798", vec![100, 101])]);
    assert!(!dry.merge_listing_truncated);

    let wet = db.match_org_identifiers_r2(args_514(false, Some(1))).await.expect("wet");
    assert_eq!((wet.merged_groups, wet.removed), (1, 1), "{wet:?}");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 100").await, 3);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE keep = 100 AND loser = 101 AND rule = 'e0'").await,
        1
    );
}

/// The strip removes only evidence that AGREES with the group. Members whose mentions
/// name other numbers still hit the wall: a second UKPRN on each, or two different
/// PPONs — which the shared register raw used to mask (the subsidiary-on-its-parent's-
/// number shape), so W6 tightens there and counts it. As before 514, a member with no
/// other number (an empty set) never denies: the reunion then reads exactly as a pair
/// publishing one spelling would.
#[tokio::test]
async fn a_reunion_member_naming_another_number_still_hits_the_wall() {
    let (db, conn) = fresh("other-number").await;
    // A different second UKPRN on each member: walled with the strip and without it.
    reg_org(&conn, 200, "GB", "GBUKPRN10007157", "University of Sheffield", &["GB-UKPRN-10007157", "GB-UKPRN-10007160"]).await;
    reg_org(&conn, 201, "GB", "GBUKPRN10007157", "University of Sheffield", &["GB-UKPRN-UKPRN 10007157", "GB-UKPRN-10007158"]).await;
    // Two PPONs, one per member, beside the shared register spelling: admitted today
    // (the shared raw masks them), walled once the strip drops the shared raw.
    reg_org(&conn, 210, "GB", "GBUKPRN10005790", "Lancaster College", &["GB-UKPRN-10005790", "GB-PPON-PVMW-4512-JZYY"]).await;
    reg_org(&conn, 211, "GB", "GBUKPRN10005790", "Lancaster College", &["GB-UKPRN-10005790", "GB-PPON-PAAA-1111-BBBB"]).await;
    // One PPON on one member only: an empty set never denies.
    reg_org(&conn, 220, "GB", "GBUKPRN10007788", "Tyne College", &["GB-UKPRN-10007788", "GB-PPON-PMCX-4817-NTNG"]).await;
    reg_org(&conn, 221, "GB", "GBUKPRN10007788", "Tyne College", &["GB-UKPRN-UKPRN: 10007788"]).await;

    let today = db.match_org_identifiers_r2(args(true, None)).await.expect("dry");
    assert_eq!((today.groups, today.denied_group_vat, today.plan_groups), (3, 2, 1), "{today:?}");
    let dry = db.match_org_identifiers_r2(args_514(true, None)).await.expect("dry");
    assert_eq!((dry.groups, dry.denied_group_vat, dry.plan_groups), (3, 2, 1), "{dry:?}");
    assert_eq!((dry.wall_strip_admitted, dry.wall_strip_denied), (1, 1), "220/221 admitted, 210/211 walled");
    let walled: Vec<&str> = dry.denied_wall_listing.iter().map(|(_, _, k, _)| k.as_str()).collect();
    assert_eq!(walled.len(), 2);
    assert!(walled.contains(&"national:GBUKPRN10007157") && walled.contains(&"national:GBUKPRN10005790"), "{walled:?}");
    let planned: Vec<&str> = dry.merge_listing.iter().map(|(_, _, k, _)| k.as_str()).collect();
    assert_eq!(planned, vec!["national:GBUKPRN10007788"]);
}

/// W6 acts only under rule `e0` and only on a GB group: the same reunion under rule
/// `r2`, and under an IE group whose mentions are GB's, is walled as before — and the
/// strip reads `national:<identity>`, so a group keyed `vat:` is never stripped.
#[tokio::test]
async fn the_own_identity_strip_is_scoped_to_gb_registers_and_rule_e0() {
    let (db, conn) = fresh("scope").await;
    reunion(&conn, 300, 301, "Leeds City College").await;
    let r2 = db.match_org_identifiers_r2(store::R2MergeArgs { rule: "r2", ..args_514(true, None) }).await.expect("dry");
    assert_eq!((r2.denied_group_vat, r2.plan_groups, r2.wall_strip_admitted), (1, 0, 0), "rule r2: {r2:?}");

    let (db, conn) = fresh("scope-ie").await;
    // An IE pair on the same literal, with GB-country mentions the mimic would key.
    reg_org(&conn, 310, "IE", "GBUKPRN10007798", "Leeds City College", &[]).await;
    reg_org(&conn, 311, "IE", "GBUKPRN10007798", "Leeds City College", &[]).await;
    for (org, raw) in [(310, "GB-UKPRN-10007798"), (311, "GB-UKPRN-UKPRN 10007798")] {
        conn.execute(
            "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
             VALUES (?, 'S-1', ?, 'Leeds City College', 'GB', ?)",
            (Value::Integer(org * 10), Value::Integer(org), Value::Text(raw.into())),
        )
        .await
        .unwrap();
    }
    let ie = db.match_org_identifiers_r2(args_514(true, None)).await.expect("dry");
    assert_eq!((ie.denied_group_vat, ie.plan_groups, ie.wall_strip_admitted), (1, 0, 0), "an IE group: {ie:?}");

    // Two label spellings of one UKPRN on a GB pair: stored `national`, the strip reads
    // both as the group's identity and admits it; stored as kind `vat`, the key is
    // `vat:…`, never a register identity, and the pair stays walled.
    for (kind, walled) in [("national", 0u64), ("vat", 1)] {
        let (db, conn) = fresh(&format!("scope-{kind}")).await;
        for (id, raw) in [(320i64, "GB-UKPRN-UKPRN 10007798"), (321, "GB-UKPRN-GB-UKPRN10007798")] {
            reg_org(&conn, id, "GB", "GBUKPRN10007798", "Leeds City College", &[raw]).await;
            conn.execute("UPDATE organizations SET identifier_kind = ? WHERE id = ?", (Value::Text(kind.into()), Value::Integer(id)))
                .await
                .unwrap();
        }
        let r = db.match_org_identifiers_r2(args_514(true, None)).await.expect("dry");
        assert_eq!((r.denied_group_vat, r.plan_groups, r.wall_strip_admitted), (walled, 1 - walled, 1 - walled), "{kind}: {r:?}");
    }
}

/// UKPRN 10007157's shape: the bare org's head name reads "Sheffield Hallam University"
/// from one stray mention, the mover's "University of Sheffield". The head-name rule
/// denies it (the safe direction) and lists it; a HIGH `merge` verdict on exactly the
/// live member set admits it; a `keep` denies it whatever the names say; a verdict on
/// another member set is stale and the name rule decides.
#[tokio::test]
async fn a_head_name_split_is_denied_and_a_high_verdict_admits_it() {
    const KEY: &str = "national:GBUKPRN10007157";
    let (db, conn) = fresh("names").await;
    reg_org(&conn, 400, "GB", "GBUKPRN10007157", "Sheffield Hallam University", &["GB-UKPRN-10007157"]).await;
    reg_org(&conn, 401, "GB", "GBUKPRN10007157", "University of Sheffield", &["GB-UKPRN-UKPRN 10007157"]).await;

    let dry = db.match_org_identifiers_r2(args_514(true, None)).await.expect("dry");
    assert_eq!((dry.denied_names, dry.plan_groups), (1, 0), "{dry:?}");
    let denied: Vec<(&str, Vec<i64>)> =
        dry.denied_listing.iter().map(|(_, _, k, m)| (k.as_str(), m.iter().map(|r| r.0).collect())).collect();
    assert_eq!(denied, vec![(KEY, vec![400, 401])], "an E0 name denial is listed");

    // Stale: reviewed on another member set.
    db.record_merge_verdicts("514-spellings-test-stale", &[verdict(KEY, vec![400, 402], "merge", "high")], 0).await.unwrap();
    let stale = db.match_org_identifiers_r2(args_514(true, None)).await.expect("dry");
    assert_eq!((stale.verdict_stale, stale.denied_names, stale.plan_groups), (1, 1, 0), "{stale:?}");

    // HIGH merge on exactly the live set: admitted past the name rule, still under the wall.
    let (db, conn) = fresh("names-merge").await;
    reg_org(&conn, 400, "GB", "GBUKPRN10007157", "Sheffield Hallam University", &["GB-UKPRN-10007157"]).await;
    reg_org(&conn, 401, "GB", "GBUKPRN10007157", "University of Sheffield", &["GB-UKPRN-UKPRN 10007157"]).await;
    db.record_merge_verdicts("514-spellings-test", &[verdict(KEY, vec![400, 401], "merge", "high")], 0).await.unwrap();
    let dry = db.match_org_identifiers_r2(args_514(true, None)).await.expect("dry");
    assert_eq!((dry.admitted_verdict, dry.denied_names, dry.plan_groups), (1, 0, 1), "{dry:?}");
    let wet = db.match_org_identifiers_r2(args_514(false, Some(1))).await.expect("wet");
    assert_eq!(wet.merged_groups, 1);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_merge_verdicts WHERE cohort = '514-spellings-test' AND applied_at IS NOT NULL").await,
        1,
        "the verdict is stamped with the merge it caused"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 1);

    // A keep denies it whatever the names say.
    let (db, conn) = fresh("names-keep").await;
    reunion(&conn, 410, 411, "Leeds City College").await;
    db.record_merge_verdicts(
        "514-spellings-test-keep",
        &[verdict("national:GBUKPRN10007798", vec![410, 411], "keep", "high")],
        0,
    )
    .await
    .unwrap();
    let kept = db.match_org_identifiers_r2(args_514(true, None)).await.expect("dry");
    assert_eq!((kept.denied_verdict, kept.plan_groups), (1, 0), "{kept:?}");
}

/// G1: a wet run bound to the dry run's listing merges only a group listed with exactly
/// its live members. A group that appeared since, and one that gained a member, are
/// held; without a listing (a truncated one, in the supervisor) parity alone decides.
#[tokio::test]
async fn a_wet_run_merges_only_listed_groups() {
    let (db, conn) = fresh("listing").await;
    reunion(&conn, 500, 501, "Leeds City College").await;
    reg_org(&conn, 510, "GB", "GBUKPRN10004116", "Bury College", &["GB-UKPRN-10004116"]).await;
    reg_org(&conn, 511, "GB", "GBUKPRN10004116", "Bury College", &["GB-UKPRN-GB-UKPRN10004116"]).await;
    let dry = db.match_org_identifiers_r2(args_514(true, None)).await.expect("dry");
    assert_eq!(dry.plan_groups, 2);
    let listing: std::collections::HashMap<(String, String, String), Vec<i64>> = dry
        .merge_listing
        .iter()
        .map(|(c, s, k, m)| ((c.clone(), (*s).to_owned(), k.clone()), m.clone()))
        .collect();

    // Drift after the review: Bury gains a member, and a group nobody listed appears.
    reg_org(&conn, 512, "GB", "GBUKPRN10004116", "Bury College", &["GB-UKPRN-10004116"]).await;
    reg_org(&conn, 520, "GB", "GBUKPRN10007339", "Warrington College", &["GB-UKPRN-10007339"]).await;
    reg_org(&conn, 521, "GB", "GBUKPRN10007339", "Warrington College", &["GB-UKPRN-UKPRN 10007339"]).await;

    let bound = store::R2MergeArgs { expect_listing: Some(&listing), ..args_514(false, Some(2)) };
    let wet = db.match_org_identifiers_r2(bound).await.expect("wet");
    assert_eq!((wet.plan_groups, wet.held_unplanned, wet.merged_groups), (3, 2, 1), "{wet:?}");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE identifier = 'GBUKPRN10007798'").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE identifier = 'GBUKPRN10004116'").await, 3, "changed: held");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE identifier = 'GBUKPRN10007339'").await, 2, "unlisted: held");

    // The fallback: no listing, parity only — both remaining groups merge.
    let wet = db.match_org_identifiers_r2(args_514(false, Some(2))).await.expect("wet");
    assert_eq!((wet.held_unplanned, wet.merged_groups), (0, 2), "{wet:?}");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 3);
}

/// E0 dry runs list what they deny — the name rule's denials in `denied_listing`, the
/// wall's in `denied_wall_listing` — in R2's 4c shape, so a group nobody foresaw is
/// read rather than counted. Wet runs list nothing.
#[tokio::test]
async fn e0_dry_lists_name_and_wall_denials() {
    let (db, conn) = fresh("denials").await;
    reg_org(&conn, 600, "GB", "GBUKPRN10007157", "Sheffield Hallam University", &["GB-UKPRN-10007157"]).await;
    reg_org(&conn, 601, "GB", "GBUKPRN10007157", "University of Sheffield", &["GB-UKPRN-UKPRN 10007157"]).await;
    reg_org(&conn, 610, "GB", "GBUKPRN10005790", "Lancaster College", &["GB-UKPRN-10005790", "GB-PPON-PVMW-4512-JZYY"]).await;
    reg_org(&conn, 611, "GB", "GBUKPRN10005790", "Lancaster College", &["GB-PPON-PAAA-1111-BBBB"]).await;
    let dry = db.match_org_identifiers_r2(args_514(true, None)).await.expect("dry");
    assert_eq!((dry.denied_names, dry.denied_group_vat, dry.plan_groups), (1, 1, 0), "{dry:?}");
    let names: Vec<(&str, Vec<(i64, &str, &str)>)> = dry
        .denied_listing
        .iter()
        .map(|(_, _, k, m)| (k.as_str(), m.iter().map(|r| (r.0, r.2.as_str(), r.3.as_str())).collect()))
        .collect();
    assert_eq!(
        names,
        vec![(
            "national:GBUKPRN10007157",
            vec![(600, "GBUKPRN10007157", "Sheffield Hallam University"), (601, "GBUKPRN10007157", "University of Sheffield")]
        )]
    );
    let wall: Vec<(&str, &str, Vec<i64>)> =
        dry.denied_wall_listing.iter().map(|(c, _, k, m)| (c.as_str(), k.as_str(), m.iter().map(|r| r.0).collect())).collect();
    assert_eq!(wall, vec![("GB", "national:GBUKPRN10005790", vec![610, 611])]);
    assert!(!dry.denied_listing_truncated && !dry.denied_wall_listing_truncated);
    let wet = db.match_org_identifiers_r2(args_514(false, Some(0))).await.expect("wet");
    assert!(wet.denied_listing.is_empty() && wet.denied_wall_listing.is_empty() && wet.merge_listing.is_empty());
}
