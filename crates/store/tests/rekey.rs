//! Issue 453: the re-key arm. Issue 452's reviewers found organizations keyed by
//! a wrong company number and, for many, the right one. The arm merges such an
//! org into the standing org that carries the right number (any spelling), or —
//! when nothing does — moves it onto the right number; it stamps the verdict,
//! and the resolver aliases the wrong number to the entity from then on. The
//! injected rules are `altid_merge.rs`'s GB miniatures.

use store::turso::{Connection, Value};
use store::{Identifier, IdentifierVerdict, Mention};

fn gb_key(value: &str) -> Option<(&'static str, String, bool)> {
    let norm: String =
        value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    let coh = norm.strip_prefix("GBCOH").or_else(|| norm.strip_prefix("GB")).unwrap_or(&norm);
    let digits = !coh.is_empty() && coh.bytes().all(|b| b.is_ascii_digit());
    match coh.len() {
        8 if digits
            || (coh[..2].bytes().all(|b| b.is_ascii_alphabetic())
                && coh[2..].bytes().all(|b| b.is_ascii_digit())) =>
        {
            Some(("GB:coh", coh.to_owned(), true))
        }
        _ => None,
    }
}

fn key(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool)> {
    if kind != "national" || country != Some("GB") {
        return None;
    }
    gb_key(value)
}

fn norm(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn name_key(name: &str) -> String {
    norm(name)
        .split(' ')
        .filter(|t| !t.is_empty() && *t != "the" && *t != "and")
        .map(|t| match t {
            "ltd" | "limited" => "§ltd",
            "plc" => "§plc",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn names_agree(a: &str, b: &str) -> bool {
    a == b
}

fn legal_family(name: &str) -> Option<&'static str> {
    norm(name)
        .split(' ')
        .filter_map(|t| match t {
            "ltd" | "limited" => Some("ltd"),
            "plc" => Some("plc"),
            _ => None,
        })
        .last()
}

fn consortium(name: &str) -> bool {
    norm(name).split(' ').any(|t| t == "consortium")
}

fn args<'a>(dry_run: bool, expect: Option<Vec<String>>, cap: Option<u64>) -> store::RekeyArgs<'a> {
    store::RekeyArgs {
        key,
        consortium,
        legal_family,
        name_key,
        names_agree,
        dry_run,
        max_rekeys: cap,
        expect,
        job_id: Some(453),
        stop: &|| false,
    }
}

// (id, identifier, name)
const ORGS: &[(i64, &str, &str)] = &[
    // merge: the transposed Rolls-Royce number, beside the real one.
    (1, "01006142", "Rolls-Royce plc"),
    (2, "01003142", "Rolls-Royce Plc"),
    // move: nothing carries the right number; the FTS spelling is kept.
    (3, "GBCOH04173398", "Amthal Fire & Security Ltd"),
    // merge across spellings: the owner is the FTS-minted row.
    (4, "05837803", "RSK Environment Ltd"),
    (5, "GBCOHSC115530", "RSK Environment Limited"),
    // names: the right number's owner is somebody else.
    (6, "02202746", "Harvey Nash Ltd"),
    (7, "02202476", "Digimune Ltd"),
    // multi-target: two rows already share the right number (R2's to fold).
    (8, "11111111", "Acme Ltd"),
    (9, "22222222", "Acme Ltd"),
    (10, "GBCOH22222222", "Acme Ltd"),
    // not high.
    (11, "33333333", "Beta Ltd"),
    // same key: the "right" number is the wrong one in another spelling.
    (12, "44444444", "Gamma Ltd"),
    // legal form: plc against Ltd.
    (13, "55555555", "Delta plc"),
    (14, "66666666", "Delta Ltd"),
    // withheld target: the right number's owner is itself under a wrong verdict.
    (15, "77777777", "Epsilon Ltd"),
    (16, "88888888", "Epsilon Ltd"),
];

fn verdict(org: i64, identifier: &str, correct: Option<&str>, confidence: &str) -> IdentifierVerdict {
    IdentifierVerdict {
        org_id: org,
        identifier: identifier.into(),
        verdict: "wrong".into(),
        correct_identifier: correct.map(str::to_owned),
        rationale: "fixture".into(),
        confidence: confidence.into(),
    }
}

fn verdicts() -> Vec<IdentifierVerdict> {
    vec![
        verdict(1, "01006142", Some("01003142"), "high"),
        verdict(3, "GBCOH04173398", Some("08004712"), "high"),
        verdict(4, "05837803", Some("SC115530"), "high"),
        verdict(6, "02202746", Some("02202476"), "high"),
        verdict(8, "11111111", Some("22222222"), "high"),
        verdict(11, "33333333", Some("33333339"), "medium"),
        verdict(12, "44444444", Some("GB44444444"), "high"),
        verdict(13, "55555555", Some("66666666"), "high"),
        verdict(15, "77777777", Some("88888888"), "high"),
        verdict(16, "88888888", None, "high"),
    ]
}

async fn bed(path: &str) -> (store::Db, Connection) {
    bed_with(path, ORGS, &verdicts()).await
}

async fn bed_with(
    path: &str,
    orgs: &[(i64, &str, &str)],
    verdicts: &[IdentifierVerdict],
) -> (store::Db, Connection) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let conn = store::turso::Builder::new_local(path).build().await.unwrap().connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    for (id, identifier, name) in orgs {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, 'GB', 'national', ?, ?, ?, 0, 0)",
            (Value::Integer(*id), Value::Text((*identifier).into()), Value::Text((*name).into()), Value::Text(name.to_lowercase())),
        )
        .await
        .unwrap();
    }
    for n in 1..=8i64 {
        conn.execute(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                                  member_path, ingested_at, parse_state, projected)
             VALUES (?, 'fts', ?, ?, 'fts:ocds-1.1', 1, 'p', 0, 'parsed', 0)",
            (Value::Integer(n), Value::Text(format!("pub-{n}")), Value::Text(format!("h{n}"))),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO notice_sections (notice_id, section_id, kind, parent_section_id)
             VALUES (?, 'S-1', 'Organization', NULL)",
            (Value::Integer(n),),
        )
        .await
        .unwrap();
    }
    // Notice 1 already names org 1 under its wrong number: a merge must carry
    // it to the right number's org, and its party row with it.
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
         VALUES (1, 'S-1', 1, ?, 'GB', ?)",
        (Value::Text(orgs[0].2.into()), Value::Text(orgs[0].1.into())),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_parties (tender_id, seq, role, organization_id, mention_notice_id, mention_section_id)
         VALUES (9, 1, 'winner', 1, 1, 'S-1')",
        (),
    )
    .await
    .unwrap();
    let r = db.record_identifier_verdicts("452", verdicts, 0).await.unwrap();
    assert_eq!(r.stale, 0);
    (db, conn)
}

async fn count(conn: &Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    match row.get_value(0).unwrap() {
        Value::Integer(n) => n,
        other => panic!("{sql}: {other:?}"),
    }
}

async fn text(conn: &Connection, sql: &str) -> Option<String> {
    let mut rows = conn.query(sql, ()).await.unwrap();
    match rows.next().await.unwrap().map(|r| r.get_value(0).unwrap()) {
        Some(Value::Text(s)) => Some(s),
        _ => None,
    }
}

/// The plan names every class once, the wet run executes only the stored plan
/// (a cap leaves a residual that a continuation finishes), and each shape lands:
/// the merge moves the wrong-number org's mention and party onto the right
/// number's org and writes the ledger; the move rewrites the identifier in the
/// wrong literal's spelling; every executed verdict is stamped; and the moved
/// org no longer reads as a register mismatch.
#[tokio::test]
async fn the_arm_plans_every_class_and_executes_the_reviewed_plan() {
    let (db, conn) = bed("test-rekey-plan.db").await;

    let dry = db.match_org_rekey(args(true, None, None)).await.unwrap();
    assert_eq!(dry.verdicts, 9, "the verdict without a right number is not a candidate");
    assert_eq!(
        (dry.not_high, dry.same_key, dry.gone, dry.several, dry.unkeyed),
        (1, 1, 0, 0, 0)
    );
    assert_eq!(
        (dry.multi_target, dry.withheld_target, dry.denied_legal_form, dry.denied_names, dry.denied_consortium),
        (1, 1, 1, 1, 0),
        "{:#?}",
        dry.denied
    );
    assert_eq!((dry.plan_merge, dry.plan_move), (2, 1));
    // A planned key pins what was reviewed: the shape and what the entity will
    // carry, so a re-POSTed right number or a changed target is another key.
    assert_eq!(
        dry.keys,
        vec![
            "GB/national/01006142>merge:01003142",
            "GB/national/05837803>merge:GBCOHSC115530",
            "GB/national/GBCOH04173398>move:GBCOH08004712",
        ]
    );
    let rsk = dry.plan.iter().find(|l| l.org == 4).unwrap();
    assert_eq!((rsk.shape.as_str(), rsk.target.as_ref().map(|t| t.0)), ("merge", Some(5)));
    let amthal = dry.plan.iter().find(|l| l.org == 3).unwrap();
    assert_eq!((amthal.shape.as_str(), amthal.new_literal.as_str()), ("move", "GBCOH08004712"));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 16, "dry wrote nothing");

    assert!(db.match_org_rekey(args(false, None, None)).await.is_err(), "no stored plan, no wet run");

    // Capped at one: the first key only, and the residual names the rest.
    let first = db.match_org_rekey(args(false, Some(dry.keys.clone()), Some(1))).await.unwrap();
    assert_eq!((first.merged, first.moved), (1, 0));
    assert_eq!(
        first.residual,
        vec!["GB/national/05837803>merge:GBCOHSC115530", "GB/national/GBCOH04173398>move:GBCOH08004712"]
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 1").await, 0);
    assert_eq!(count(&conn, "SELECT organization_id FROM organization_mentions WHERE notice_id = 1").await, 2);
    assert_eq!(count(&conn, "SELECT organization_id FROM tender_version_parties WHERE tender_id = 9").await, 2);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE rule = 'rekey' AND keep = 2 AND loser = 1").await,
        1
    );

    // The continuation holds against the residual.
    let rest = db.match_org_rekey(args(false, Some(first.residual.clone()), None)).await.unwrap();
    assert_eq!((rest.merged, rest.moved, rest.deferred_unreviewed), (1, 1, 0), "{rest:#?}");
    assert!(rest.residual.is_empty());
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 4").await, 0);
    assert_eq!(
        text(&conn, "SELECT identifier FROM organizations WHERE id = 3").await.as_deref(),
        Some("GBCOH08004712")
    );
    for (wrong, literal) in
        [("01006142", "01003142"), ("05837803", "GBCOHSC115530"), ("GBCOH04173398", "GBCOH08004712")]
    {
        assert_eq!(
            text(&conn, &format!("SELECT applied_literal FROM org_identifier_verdicts WHERE identifier = '{wrong}'"))
                .await
                .as_deref(),
            Some(literal),
            "{wrong} is stamped with what the entity carries now"
        );
    }
    let moved = store::read::organizations(
        &conn,
        &store::read::Filter::default(),
        store::read::Scope::At { id: 3, seq: 0 },
    )
    .await
    .unwrap();
    assert_eq!(moved[0].identifier_verdict, None, "the moved org carries its right number now");
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 3").await,
        2,
        "one when the verdict flagged it, one when the move cleared it"
    );

    // Everything done: a re-plan finds nothing left, and the denied stay denied.
    let again = db.match_org_rekey(args(true, None, None)).await.unwrap();
    assert_eq!((again.plan_merge, again.plan_move, again.gone), (0, 0, 3));
}

/// A key the dry run did not plan never executes, whatever the live plan says.
#[tokio::test]
async fn an_unreviewed_key_is_deferred_not_executed() {
    let (db, conn) = bed("test-rekey-deferred.db").await;
    let r = db.match_org_rekey(args(false, Some(Vec::new()), None)).await.unwrap();
    assert_eq!((r.merged, r.moved, r.deferred_unreviewed), (0, 0, 3));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 16);
}

fn mention(notice: i64, name: &str, value: &str) -> Mention {
    Mention {
        notice_id: notice,
        section_id: "S-1".into(),
        name: name.into(),
        country: Some("GB".into()),
        raw_identifier: Some(value.into()),
        scheme: None,
        identifier: Some(Identifier { country: Some("GB".into()), kind: "national".into(), value: value.into() }),
        variants: Vec::new(),
    }
}

/// After the re-key, nothing carries the wrong literals, so without the alias a
/// publisher's next use of one would mint a fresh row under it. The exact wrong
/// literal binds to the entity; another spelling of it binds only under a
/// matching name (the number may be somebody else's real one) and mints
/// otherwise; the moved org's old literal finds it too.
#[tokio::test]
async fn the_resolver_aliases_a_re_keyed_wrong_number_to_its_entity() {
    let (db, conn) = bed("test-rekey-alias.db").await;
    let dry = db.match_org_rekey(args(true, None, None)).await.unwrap();
    db.match_org_rekey(args(false, Some(dry.keys), None)).await.unwrap();
    let orgs = count(&conn, "SELECT COUNT(*) FROM organizations").await;

    let mut resolver =
        db.mention_resolver(Some(key), Some(consortium), None, Some(norm), None, None, 0).await.unwrap();
    let ids = db
        .resolve_mentions(
            &mut resolver,
            &[
                mention(2, "Rolls-Royce plc", "01006142"),
                mention(3, "Rolls-Royce Plc", "GBCOH01006142"),
                mention(4, "Someone Else Ltd", "GBCOH01006142"),
                mention(5, "Amthal Fire & Security Ltd", "GBCOH04173398"),
            ],
            0,
        )
        .await
        .unwrap();
    db.finish_mention_resolver(resolver).await.unwrap();
    assert_eq!(ids[0], 2, "the exact wrong literal reaches the entity");
    assert_eq!(ids[1], 2, "another spelling of it, under the entity's name, too");
    assert!(ids[2] > 16, "a stranger under that number mints");
    assert_eq!(ids[3], 3, "the moved org's old literal finds it");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, orgs + 1);
}

/// Two owner-less wrong numbers naming ONE right number: moving both would mint
/// two carriers of it (a family the next R2 declines). Only the smallest key
/// moves; the other waits, and the next plan merges it into the moved org. A
/// reviewer's `GB-COH-` spelling of the right number never doubles the prefix.
#[tokio::test]
async fn one_right_number_moves_once_and_the_rest_merge_into_it() {
    let orgs: &[(i64, &str, &str)] = &[(1, "GBCOH09999991", "Zeta Ltd"), (2, "GBCOH09999992", "Zeta Limited")];
    let (db, conn) = bed_with(
        "test-rekey-pending.db",
        orgs,
        &[
            verdict(1, "GBCOH09999991", Some("GB-COH-09999999"), "high"),
            verdict(2, "GBCOH09999992", Some("09999999"), "high"),
        ],
    )
    .await;

    let dry = db.match_org_rekey(args(true, None, None)).await.unwrap();
    assert_eq!((dry.plan_move, dry.pending_move, dry.plan_merge), (1, 1, 0), "{:#?}", dry.denied);
    assert_eq!(dry.keys, vec!["GB/national/GBCOH09999991>move:GBCOH09999999"]);
    assert_eq!(dry.denied[0].shape, "pending-move");
    let wet = db.match_org_rekey(args(false, Some(dry.keys), None)).await.unwrap();
    assert_eq!((wet.moved, wet.merged), (1, 0));
    assert_eq!(
        text(&conn, "SELECT identifier FROM organizations WHERE id = 1").await.as_deref(),
        Some("GBCOH09999999")
    );

    let next = db.match_org_rekey(args(true, None, None)).await.unwrap();
    assert_eq!((next.plan_move, next.pending_move, next.plan_merge, next.gone), (0, 0, 1, 1));
    assert_eq!(next.keys, vec!["GB/national/GBCOH09999992>merge:GBCOH09999999"]);
    let wet = db.match_org_rekey(args(false, Some(next.keys), None)).await.unwrap();
    assert_eq!((wet.moved, wet.merged), (0, 1));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 1);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE rule = 'rekey' AND keep = 1 AND loser = 2").await,
        1
    );
}

/// A destination some verdict flags is not a place to put an entity: neither a
/// right-number owner under a `related` verdict (merge), nor a right number a
/// `wrong` verdict names while nothing carries it (move).
#[tokio::test]
async fn a_destination_under_a_verdict_is_refused() {
    let orgs: &[(i64, &str, &str)] = &[
        (1, "41414149", "Iota Ltd"),
        (2, "41414141", "Iota Ltd"),
        (3, "61616161", "Lambda Ltd"),
        (4, "61616169", "Lambda Ltd"),
    ];
    let mut related = verdict(2, "41414141", None, "high");
    related.verdict = "related".into();
    let (db, conn) = bed_with(
        "test-rekey-destination.db",
        orgs,
        &[
            verdict(1, "41414149", Some("41414141"), "high"),
            related,
            verdict(3, "61616161", Some("61616169"), "high"),
            verdict(4, "61616169", None, "high"),
        ],
    )
    .await;
    // The org the second verdict flagged is gone; its number stays flagged.
    conn.execute("DELETE FROM organizations WHERE id = 4", ()).await.unwrap();

    let dry = db.match_org_rekey(args(true, None, None)).await.unwrap();
    assert_eq!((dry.plan_merge, dry.plan_move, dry.destination_verdict), (0, 0, 2), "{:#?}", dry.denied);
    let mut shapes: Vec<(i64, &str)> = dry.denied.iter().map(|l| (l.org, l.shape.as_str())).collect();
    shapes.sort();
    assert_eq!(shapes, vec![(1, "destination-verdict"), (3, "destination-verdict")]);
    assert!(dry.denied.iter().all(|l| !l.key.contains('>')), "a denied key is the bare wrong triple");
}

/// The alias follows a CHAIN (a moved org's new number found wrong in its turn
/// and moved again) and resolves at bind time: in a rebuild the entity's row
/// does not stand when the resolver opens; it is minted by the fold, and the
/// wrong literal's later mention must reach that row, not mint another.
#[tokio::test]
async fn the_alias_follows_a_chain_and_binds_to_a_row_minted_during_the_fold() {
    let orgs: &[(i64, &str, &str)] = &[(1, "50505050", "Kappa Ltd")];
    let (db, conn) =
        bed_with("test-rekey-chain.db", orgs, &[verdict(1, "50505050", Some("50505051"), "high")]).await;
    let move_once = || async {
        let dry = db.match_org_rekey(args(true, None, None)).await.unwrap();
        assert_eq!(dry.plan_move, 1, "{:#?}", dry.denied);
        let wet = db.match_org_rekey(args(false, Some(dry.keys), None)).await.unwrap();
        assert_eq!(wet.moved, 1);
    };
    move_once().await;
    // The moved-onto number is found wrong in its turn.
    let r = db
        .record_identifier_verdicts("452b", &[verdict(1, "50505051", Some("50505052"), "high")], 0)
        .await
        .unwrap();
    assert_eq!(r.recorded, 1);
    move_once().await;
    assert_eq!(text(&conn, "SELECT identifier FROM organizations WHERE id = 1").await.as_deref(), Some("50505052"));

    let resolve = |ms: Vec<Mention>| {
        let db = &db;
        async move {
            let mut resolver =
                db.mention_resolver(Some(key), Some(consortium), None, Some(norm), None, None, 0).await.unwrap();
            let ids = db.resolve_mentions(&mut resolver, &ms, 0).await.unwrap();
            db.finish_mention_resolver(resolver).await.unwrap();
            ids
        }
    };
    let ids = resolve(vec![mention(2, "Kappa Ltd", "50505050"), mention(3, "Kappa Ltd", "50505051")]).await;
    assert_eq!(ids, vec![1, 1], "both earlier numbers reach the entity through the chain");

    // A rebuild's shape: the entity's row is not standing when the resolver
    // opens. The fold mints it under the right number, and a later mention of
    // the first wrong number binds to that row.
    conn.execute("DELETE FROM organization_mentions", ()).await.unwrap();
    conn.execute("DELETE FROM tender_version_parties", ()).await.unwrap();
    conn.execute("DELETE FROM organizations", ()).await.unwrap();
    let ids = resolve(vec![mention(4, "Kappa Ltd", "50505052"), mention(5, "Kappa Ltd", "50505050")]).await;
    assert_eq!(ids[1], ids[0], "the wrong number binds to the row the fold minted");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 1);
}
