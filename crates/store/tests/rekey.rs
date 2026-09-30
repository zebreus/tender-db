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

async fn bed(path: &str) -> (store::Db, Connection) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let conn = store::turso::Builder::new_local(path).build().await.unwrap().connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    for (id, identifier, name) in ORGS {
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
    // Notice 1 already names Rolls-Royce under the wrong number: the merge must
    // carry it to org 2, and its party row with it.
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
         VALUES (1, 'S-1', 1, 'Rolls-Royce plc', 'GB', '01006142')",
        (),
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
    db.record_identifier_verdicts(
        "452",
        &[
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
        ],
        0,
    )
    .await
    .unwrap();
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
    assert_eq!(
        dry.keys,
        vec!["GB/national/01006142", "GB/national/05837803", "GB/national/GBCOH04173398"]
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
    assert_eq!(first.residual, vec!["GB/national/05837803", "GB/national/GBCOH04173398"]);
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
