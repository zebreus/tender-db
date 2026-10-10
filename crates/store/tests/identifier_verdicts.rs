//! Issue 452: per-identifier verdicts. A reviewer found some organizations are
//! keyed by a registration number that is not theirs (a transposed digit, a
//! dissolved stranger's number, one never issued). The org keeps its row; a
//! `wrong` verdict stops the number being used as evidence of identity with
//! anyone else — the R2/E0 groups, the resolver's canonical bind — and the API
//! flags `wrong` and `related`. A verdict stands only while the org still carries
//! the number. The injected rules are the sibling tests' FI miniatures
//! (`r2_name_gate.rs`, `resolver_prevention.rs`); the altid arm and its alias are
//! pinned in `altid_merge.rs`, R3 in `r3_merge.rs`.

use store::read::{self, Filter, Scope};
use store::turso::{Connection, Value};
use store::{Identifier, IdentifierVerdict, Mention};

fn key(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool, bool)> {
    let norm: String =
        value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    let (cc, body) = if kind == "vat" {
        (norm.get(..2)?.to_owned(), norm.get(2..)?.to_owned())
    } else {
        (country?.to_owned(), norm)
    };
    if cc == "GB" && kind != "vat" {
        return gb_coh(&body);
    }
    match (cc.as_str(), body.len()) {
        ("FI", 8) if body.bytes().all(|b| b.is_ascii_digit()) => Some(("FI:ytunnus", body, true, false)),
        _ => None,
    }
}

/// `crosswalk::canonical_key`'s GB company-number arm in miniature, with issue
/// 470's O/0 fold and its flag (`r2_merge.rs` carries the same one).
fn gb_coh(body: &str) -> Option<(&'static str, String, bool, bool)> {
    let coh = body.strip_prefix("GBCOH").unwrap_or(body);
    if coh.len() != 8 {
        return None;
    }
    let orig = coh.as_bytes();
    let mut out = orig.to_vec();
    for c in &mut out[2..] {
        if *c == b'O' {
            *c = b'0';
        }
    }
    let zero_like = |c: u8| c.is_ascii_digit() || c == b'O';
    if orig[0] == b'O' && zero_like(orig[1]) {
        out[0] = b'0';
    }
    if orig[1] == b'O' && zero_like(orig[0]) {
        out[1] = b'0';
    }
    if orig[0] == b'0' && orig[1] == b'C' {
        out[0] = b'O';
    }
    let folded = out != orig;
    let k = String::from_utf8(out).unwrap();
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    let shaped = digits(&k)
        || (k[..2].bytes().all(|b| b.is_ascii_alphabetic()) && digits(&k[2..]))
        || (k.starts_with("R0") && digits(&k[2..]));
    shaped.then(|| ("GB:coh", k, true, folded))
}

fn condemns(_c: Option<&str>, _k: &str, _v: &str) -> bool {
    false
}

fn consortium(name: &str) -> bool {
    name.to_lowercase().contains("groupement")
}

fn legal_form(_name: &str) -> Option<&'static str> {
    None
}

fn r2_args(dry_run: bool, expect_groups: Option<u64>) -> store::R2MergeArgs<'static> {
    store::R2MergeArgs {
        key,
        name_key: |n| n.to_lowercase(),
        names_agree: |a, b| a == b,
        trim: |n| n.to_owned(),
        norm: |n| n.to_lowercase(),
        condemns,
        consortium,
        legal_form,
        rule: "r2",
        n3: |n| n.to_lowercase(),
        stoplist_cap: 20,
        dry_run,
        max_groups: None,
        expect_groups,
        job_id: Some(9),
        stop: &|| false,
    }
}

fn verdict(org_id: i64, identifier: &str, verdict: &str) -> IdentifierVerdict {
    IdentifierVerdict {
        org_id,
        identifier: identifier.into(),
        verdict: verdict.into(),
        correct_identifier: None,
        rationale: "fixture".into(),
        confidence: "high".into(),
    }
}

/// A fresh store with `orgs` (id, country, kind, identifier, name) and `notices`
/// notices of one Organization section each, for the resolver's mention rows.
async fn bed(path: &str, orgs: &[(i64, &str, &str, &str, &str)], notices: i64) -> (store::Db, Connection) {
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(path).await.unwrap();
    let conn = store::turso::Builder::new_local(path).build().await.unwrap().connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    for (id, c, k, v, n) in orgs {
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
    for n in 1..=notices {
        conn.execute(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id,
                                  member_path, ingested_at, parse_state, projected)
             VALUES (?, 'ted', ?, ?, 'eforms:test', 1, 'p', 0, 'parsed', 0)",
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
    (db, conn)
}

async fn count(conn: &Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let Value::Integer(n) = row.get_value(0).unwrap() else { panic!("{sql}: not an integer") };
    n
}

/// Validation refuses the whole upload; a verdict naming an org that no longer
/// carries the checked number is skipped as stale; an upsert replaces the
/// standing verdict whatever cohort it came from; the served status moving is a
/// change event; and the readback lists the store, keyed by the triple.
#[tokio::test]
async fn verdicts_are_validated_keyed_by_the_triple_and_replaced_on_re_review() {
    let (db, conn) =
        bed("test-identifier-verdicts-record.db", &[(1, "GB", "national", "02202746", "Harvey Nash Ltd")], 0).await;

    let bad = db
        .record_identifier_verdicts("c1", &[verdict(1, "02202746", "wrong"), verdict(1, "x", "maybe")], 0)
        .await;
    assert!(bad.is_err(), "an unknown verdict refuses the upload");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_identifier_verdicts").await, 0, "and records nothing");

    let r = db
        .record_identifier_verdicts(
            "c1",
            &[
                IdentifierVerdict { correct_identifier: Some("02202476".into()), ..verdict(1, "02202746", "wrong") },
                // An org that is gone (merged away): nothing to key it by.
                verdict(99, "01234567", "wrong"),
            ],
            10,
        )
        .await
        .unwrap();
    assert_eq!((r.recorded, r.stale, r.changed), (1, 1, 1));
    let events = "SELECT COUNT(*) FROM changes WHERE entity_kind = 'organization' AND entity_id = 1 AND op = 'changed'";
    assert_eq!(count(&conn, events).await, 1, "the served identifier_status moved: one event");

    let same = db.record_identifier_verdicts("c1b", &[verdict(1, "02202746", "wrong")], 15).await.unwrap();
    assert_eq!(same.changed, 0, "re-recording what is served is no change");
    let again = db.record_identifier_verdicts("c2", &[verdict(1, "02202746", "related")], 20).await.unwrap();
    assert_eq!((again.recorded, again.stale, again.changed), (1, 0, 1));
    assert_eq!(count(&conn, events).await, 2);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_identifier_verdicts").await, 1, "one row per triple");
    let (cols, rows) = db.verdict_rows("identifier", Some("c2"), 10).await.unwrap();
    assert_eq!(cols[..6], ["identifier", "identifier_kind", "country", "org_id", "cohort", "verdict"]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][..4], [
        Value::Text("02202746".into()),
        Value::Text("national".into()),
        Value::Text("GB".into()),
        Value::Integer(1),
    ]);
    assert_eq!(rows[0][5], Value::Text("related".into()), "the re-review replaced the verdict");
    assert_eq!(rows[0][6], Value::Null, "and its correct_identifier with it");
}

/// Issue 466: a `related` number withholds its org from R2 too — it is a related
/// company's number, and grouping by it folds the org into that company — but only
/// until the re-key arm stamps the verdict: after that, whoever carries the number is
/// the related company itself, which the verdict is not about.
#[tokio::test]
async fn a_related_verdict_withholds_only_until_it_is_applied() {
    let (db, conn) = bed(
        "test-identifier-verdicts-related.db",
        &[(30, "FI", "national", "30303030", "Annodata Oy"), (31, "FI", "vat", "FI30303030", "Annodata Oy")],
        0,
    )
    .await;
    let plan = || async { db.match_org_identifiers_r2(r2_args(true, None)).await.unwrap() };
    assert_eq!((plan().await.withheld, plan().await.plan_groups), (0, 1), "the control");
    db.record_identifier_verdicts("466", &[verdict(31, "FI30303030", "related")], 0).await.unwrap();
    let r = plan().await;
    assert_eq!((r.withheld, r.plan_groups), (1, 0), "withheld while unapplied");
    conn.execute("UPDATE org_identifier_verdicts SET applied_at = 1 WHERE identifier = 'FI30303030'", ())
        .await
        .unwrap();
    let r = plan().await;
    assert_eq!((r.withheld, r.plan_groups), (0, 1), "applied: no longer withheld");
}

/// R2 and a wrong number. Group 01003158: two rows of one supplier plus a
/// council keyed by the same number in another spelling — names disagree, so
/// the whole group stands (issue 359). Withholding the council's number takes
/// it out, and the supplier's two rows merge. Group 20445111: a lone withheld
/// row and a stranger — no group is left at all. The withheld rows are
/// untouched by the wet run.
#[tokio::test]
async fn r2_leaves_a_withheld_member_out_and_merges_the_remainder() {
    let (db, conn) = bed(
        "test-identifier-verdicts-r2.db",
        &[
            (10, "FI", "national", "01003158", "Telinekataja Oy"),
            (11, "FI", "vat", "FI01003158", "Telinekataja Oy"),
            (12, "FI", "national", "0100315-8", "Cheltenham Borough Council"),
            (20, "FI", "national", "20445111", "Digimune Ltd"),
            (21, "FI", "vat", "FI20445111", "Silver Energy Management"),
        ],
        0,
    )
    .await;

    let before = db.match_org_identifiers_r2(r2_args(true, None)).await.unwrap();
    assert_eq!((before.groups, before.denied_names, before.plan_groups, before.withheld), (2, 2, 0, 0));

    db.record_identifier_verdicts(
        "452",
        &[verdict(12, "0100315-8", "wrong"), verdict(20, "20445111", "wrong"), verdict(21, "FI20445111", "right")],
        0,
    )
    .await
    .unwrap();
    let dry = db.match_org_identifiers_r2(r2_args(true, None)).await.unwrap();
    assert_eq!((dry.scanned, dry.withheld), (5, 2), "`right` does not withhold");
    assert_eq!((dry.groups, dry.denied_names, dry.plan_groups), (1, 0, 1), "{dry:#?}");

    let wet = db.match_org_identifiers_r2(r2_args(false, Some(1))).await.unwrap();
    assert_eq!((wet.merged_groups, wet.removed), (1, 1));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id IN (10, 11)").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id IN (12, 20, 21)").await, 3);
}

fn mention(notice: i64, name: &str, kind: &str, value: &str) -> Mention {
    Mention {
        notice_id: notice,
        section_id: "S-1".into(),
        name: name.into(),
        country: Some("FI".into()),
        raw_identifier: Some(value.into()),
        scheme: None,
        identifier: Some(Identifier { country: Some("FI".into()), kind: kind.into(), value: value.into() }),
        variants: Vec::new(),
    }
}

async fn resolve(db: &store::Db, mentions: &[Mention]) -> Vec<i64> {
    let norm: fn(&str) -> String = |n| n.to_lowercase();
    let mut resolver =
        db.mention_resolver(Some(key), Some(consortium), None, Some(norm), None, None, 0).await.unwrap();
    let ids = db.resolve_mentions(&mut resolver, mentions, 0).await.unwrap();
    db.finish_mention_resolver(resolver).await.unwrap();
    ids
}

/// The resolver's canonical bind. Unreviewed, key 01003158 stands on two rows
/// — a council under a wrong number and the supplier the number belongs to —
/// so it is poisoned and a third spelling mints; key 20445111 has one owner and
/// binds name-blind. With the council's and Digimune's numbers withheld, both
/// keys are GUARDED: a different spelling binds only to the owner its name
/// matches. So the supplier's own spelling finds the supplier, the council's
/// publisher's spelling finds the council (neither a twin under the wrong
/// number nor the supplier), and a stranger mints once — then is found again by
/// name. The council's exact literal still binds to it. (Two stores: a mint in
/// the unreviewed half would itself become an owner of the key.)
#[tokio::test]
async fn a_guarded_key_binds_only_to_the_owner_whose_name_matches() {
    let orgs = [
        (1, "FI", "vat", "FI01003158", "Cheltenham Borough Council"),
        (2, "FI", "national", "01003158", "Telinekataja Oy"),
        (3, "FI", "national", "20445111", "Digimune Ltd"),
    ];
    let (db, _conn) = bed("test-identifier-verdicts-resolver-before.db", &orgs, 2).await;
    let unreviewed = resolve(
        &db,
        &[mention(1, "TELINEKATAJA OY", "national", "0100315-8"), mention(2, "Silver Energy", "vat", "FI-20445111")],
    )
    .await;
    assert!(unreviewed[0] > 3, "two owners poison the key: the third spelling mints");
    assert_eq!(unreviewed[1], 3, "one owner: another company's spelling binds to it name-blind");

    let (db, conn) = bed("test-identifier-verdicts-resolver-after.db", &orgs, 7).await;
    db.record_identifier_verdicts("452", &[verdict(1, "FI01003158", "wrong"), verdict(3, "20445111", "wrong")], 0)
        .await
        .unwrap();
    let ids = resolve(
        &db,
        &[
            mention(1, "TELINEKATAJA OY", "national", "0100315-8"),
            mention(2, "Cheltenham Borough Council", "national", "010031-58"),
            mention(3, "Silver Energy", "vat", "FI-20445111"),
            mention(4, "SILVER ENERGY", "national", "2044-5111"),
            mention(5, "Nobody Ltd", "national", "01-003158"),
            mention(6, "Cheltenham Borough Council", "vat", "FI01003158"),
        ],
    )
    .await;
    assert_eq!(ids[0], 2, "the supplier's spelling finds the supplier");
    assert_eq!(ids[1], 1, "the council's publisher finds the council: no twin, not the supplier");
    assert!(ids[2] > 3, "a stranger on the withheld number mints…");
    assert_eq!(ids[3], ids[2], "…and joins the key's owners, so its next spelling finds it by name");
    assert!(ids[4] > 3 && ids[4] != ids[2], "a name no owner carries mints");
    assert_eq!(ids[5], 1, "the exact literal still binds to the org that published it");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 5);
}

fn gb_mention(notice: i64, name: &str, value: &str) -> Mention {
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

/// Issue 470: a mention whose literal needed the GB O/0 fold binds through the
/// guarded path — to the one owner its names match — never through the key
/// alone. So a new `GBCOHSCO55775` named Galliford Try finds the `SC055775`
/// org; the same literal under another name mints, and that mint claims
/// nothing (the next Galliford spelling still finds the register's org). A
/// folded row is no key-only owner either: the number a trailing-O rotation
/// proposes (`9694399O` → `96943990`), published as itself, mints rather than
/// binding to the rotation's org (the decision's refinement 2).
#[tokio::test]
async fn a_folded_mention_binds_only_to_the_owner_whose_name_matches() {
    let orgs = [
        (1, "GB", "national", "SC055775", "Galliford Try Infrastructure Ltd"),
        (2, "GB", "national", "9694399O", "Trailing O Ltd"),
    ];
    let (db, conn) = bed("test-identifier-verdicts-folded.db", &orgs, 4).await;
    let ids = resolve(
        &db,
        &[
            gb_mention(1, "Galliford Try Infrastructure Ltd", "GBCOHSCO55775"),
            gb_mention(2, "Somebody Else Ltd", "GBCOHSCO55775"),
            gb_mention(3, "GALLIFORD TRY INFRASTRUCTURE LTD", "SCO55775"),
            gb_mention(4, "Trailing O Ltd", "96943990"),
        ],
    )
    .await;
    assert_eq!(ids[0], 1, "the folded spelling finds the register's org by name");
    assert!(ids[1] > 2, "the same literal under another name mints…");
    assert_eq!(ids[2], 1, "…and claims nothing: the next Galliford spelling still finds org 1");
    assert!(ids[3] > 2 && ids[3] != ids[1], "a folded row owns no key a mention can match by key alone");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 4);
}

/// Issue 470 review: a folded mention's one name match must clear the
/// generic-name wall. A folded key is not merge-grade, so `Highways Department`
/// — a name over the cap (the test resolver's cap is 0, so one carrier is
/// over) — is no evidence that `SCO24680` is the `SC024680` org's number: the
/// mention mints. A rare name still binds.
#[tokio::test]
async fn a_folded_mention_matching_only_a_generic_name_mints() {
    let orgs = [
        (1, "GB", "national", "SC024680", "Highways Department"),
        (2, "GB", "national", "SC013579", "Acme Widgets Ltd"),
    ];
    let (db, conn) = bed("test-identifier-verdicts-folded-generic.db", &orgs, 2).await;
    conn.execute("INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (1, 'n2', 'highways department')", ())
        .await
        .unwrap();
    let ids = resolve(
        &db,
        &[gb_mention(1, "Highways Department", "SCO24680"), gb_mention(2, "Acme Widgets Ltd", "SCO13579")],
    )
    .await;
    assert!(ids[0] > 2, "a generic name is no evidence for a folded key: a mint");
    assert_eq!(ids[1], 2, "a rare name binds");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 3);
}

/// Issue 470 review: a WITHHELD lookalike guards nothing. 452 flagged
/// `GBCOHIPO30808` `wrong` (its challenger named `IP030808`, the number the fold
/// proposes), and guarding that key took the register spelling's org out of
/// `canon_of`: a bare `IP030808` under a name that org does not carry minted a
/// twin of the right number. The right number binds by key again, and the
/// withheld literal still binds to the org that published it.
#[tokio::test]
async fn a_withheld_lookalike_does_not_guard_the_number_its_fold_proposes() {
    let orgs = [
        (1, "GB", "national", "GBCOHIP030808", "Funeral Services Limited T/A Co-op Funeralcare"),
        (2, "GB", "national", "GBCOHIPO30808", "Co-Op Funeral Care"),
    ];
    let (db, conn) = bed("test-identifier-verdicts-withheld-fold.db", &orgs, 3).await;
    db.record_identifier_verdicts("452", &[verdict(2, "GBCOHIPO30808", "wrong")], 0).await.unwrap();
    let ids = resolve(
        &db,
        &[
            gb_mention(1, "Co-op Funeralcare", "IP030808"),
            gb_mention(2, "Co-Op Funeral Care", "GBCOHIPO30808"),
            gb_mention(3, "Funeral Services Ltd", "GB-COH-IP030808"),
        ],
    )
    .await;
    assert_eq!(ids[0], 1, "the right number binds to the register spelling's org by key");
    assert_eq!(ids[1], 2, "the withheld literal still binds to the org that published it");
    assert_eq!(ids[2], 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 2, "nothing minted");
}

/// The altid name key in miniature, as lenient as the fold gate's: `&` is
/// `and`, and a `Ltd` one side carries is no difference.
fn gate_key(name: &str) -> String {
    name.to_lowercase().replace('&', " and ").split_whitespace().filter(|t| *t != "ltd").collect::<Vec<_>>().join(" ")
}

/// Issue 470 review: an R2 merge of a fold-joined pair HOLDS. The lookalike org
/// is gone and its literal is recorded on the survivor
/// (`organization_merged_identifiers`), so the publisher's next mention of that
/// literal binds there — although its name (`Morris and Spottiswood`) is none
/// the survivor carries under the resolver's exact name key, a stricter test
/// than the gate's agreement. Before, it re-minted the lookalike and the split
/// was back. A lookalike spelling no merge recorded still needs a name match.
#[tokio::test]
async fn a_merged_lookalike_literal_binds_to_its_survivor() {
    let orgs = [
        (1, "GB", "national", "SCO46129", "Morris and Spottiswood"),
        (2, "GB", "national", "SC046129", "Morris & Spottiswood Ltd"),
    ];
    let (db, conn) = bed("test-identifier-verdicts-merged-fold.db", &orgs, 3).await;
    let dry = db
        .match_org_identifiers_r2(store::R2MergeArgs { name_key: gate_key, ..r2_args(true, None) })
        .await
        .unwrap();
    assert_eq!((dry.keyed_folded, dry.fold_excluded, dry.plan_groups), (1, 0, 1), "{dry:#?}");
    let wet = db
        .match_org_identifiers_r2(store::R2MergeArgs { name_key: gate_key, ..r2_args(false, Some(1)) })
        .await
        .unwrap();
    assert_eq!((wet.merged_groups, wet.removed), (1, 1));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 2").await, 1, "the register spelling keeps");
    let ids = resolve(
        &db,
        &[
            gb_mention(1, "Morris and Spottiswood", "SCO46129"),
            gb_mention(2, "Morris and Spottiswood", "SCO46129"),
            gb_mention(3, "Morris and Spottiswood", "GBCOHSCO46129"),
        ],
    )
    .await;
    assert_eq!(ids[..2], [2, 2], "the merged literal binds to its survivor, and the bind is cached");
    assert!(ids[2] > 2, "a spelling no merge recorded is a folded mention like any other: no name match, a mint");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 2);
}

/// `/v1/organizations` reads the verdict beside the row: `wrong` and `related`
/// are served, `right` and no verdict are not, and a verdict on a number the org
/// no longer carries is not either. A rebuild re-mints org ids; the verdict is
/// keyed by the triple, so the re-minted org carries it again.
#[tokio::test]
async fn the_org_read_carries_the_verdict_only_while_the_org_carries_the_number() {
    let (db, conn) = bed(
        "test-identifier-verdicts-read.db",
        &[
            (1, "GB", "national", "02202746", "Harvey Nash Ltd"),
            (2, "GB", "national", "04958135", "Southern Gas Networks plc"),
            (3, "GB", "national", "01003142", "Rolls-Royce plc"),
            (4, "GB", "national", "07495895", "Acme Ltd"),
        ],
        0,
    )
    .await;
    db.record_identifier_verdicts(
        "452",
        &[verdict(1, "02202746", "wrong"), verdict(2, "04958135", "related"), verdict(3, "01003142", "right")],
        0,
    )
    .await
    .unwrap();
    let served = |rows: Vec<read::OrganizationRow>| -> Vec<(i64, Option<String>)> {
        rows.into_iter().map(|r| (r.id, r.identifier_verdict)).collect()
    };
    let page = read::organizations(&conn, &Filter::default(), Scope::Page { after: 0, limit: 10 }).await.unwrap();
    assert_eq!(
        served(page),
        vec![(1, Some("wrong".into())), (2, Some("related".into())), (3, None), (4, None)]
    );
    let by_name = read::organizations_by_name(&conn, &Filter::default(), "harvey", None, 10).await.unwrap();
    assert_eq!(served(by_name), vec![(1, Some("wrong".into()))], "the name-ordered path reads it too");

    conn.execute("UPDATE organizations SET identifier = '02202476' WHERE id = 1", ()).await.unwrap();
    let page = read::organizations(&conn, &Filter::default(), Scope::At { id: 1, seq: 0 }).await.unwrap();
    assert_eq!(served(page), vec![(1, None)], "re-keyed: the verdict was about the old number");

    // A rebuild's renumbering: Southern Gas Networks re-minted as org 50.
    conn.execute("DELETE FROM organizations WHERE id = 2", ()).await.unwrap();
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (50, 'GB', 'national', '04958135', 'Southern Gas Networks plc', 'southern gas networks plc', 0, 0)",
        (),
    )
    .await
    .unwrap();
    let page = read::organizations(&conn, &Filter::default(), Scope::At { id: 50, seq: 0 }).await.unwrap();
    assert_eq!(served(page), vec![(50, Some("related".into()))], "the triple carries the verdict to the new id");
}
