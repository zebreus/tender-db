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

fn key(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool)> {
    let norm: String =
        value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    let (cc, body) = if kind == "vat" {
        (norm.get(..2)?.to_owned(), norm.get(2..)?.to_owned())
    } else {
        (country?.to_owned(), norm)
    };
    match (cc.as_str(), body.len()) {
        ("FI", 8) if body.bytes().all(|b| b.is_ascii_digit()) => Some(("FI:ytunnus", body, true)),
        _ => None,
    }
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

/// Validation refuses the whole upload; an upsert replaces the standing verdict
/// whatever cohort it came from; `live` counts the verdicts whose org still
/// carries the checked number; and the readback lists the store.
#[tokio::test]
async fn verdicts_are_validated_replaced_on_re_review_and_counted_live() {
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
                // An org that is gone (merged away): recorded, not live.
                verdict(99, "01234567", "wrong"),
            ],
            10,
        )
        .await
        .unwrap();
    assert_eq!((r.recorded, r.live), (2, 1));

    let again = db.record_identifier_verdicts("c2", &[verdict(1, "02202746", "related")], 20).await.unwrap();
    assert_eq!((again.recorded, again.live), (1, 1));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_identifier_verdicts").await, 2, "one row per (org, identifier)");
    let (cols, rows) = db.verdict_rows("identifier", Some("c2"), 10).await.unwrap();
    assert_eq!(cols[..4], ["org_id", "identifier", "cohort", "verdict"]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][3], Value::Text("related".into()), "the re-review replaced the verdict");
    assert_eq!(rows[0][4], Value::Null, "and its correct_identifier with it");
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
    assert_eq!((dry.scanned, dry.withheld), (5, 2), "only `wrong` withholds");
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
    let mut resolver = db.mention_resolver(Some(key), Some(consortium), None, None, None, None, 0).await.unwrap();
    let ids = db.resolve_mentions(&mut resolver, mentions, 0).await.unwrap();
    db.finish_mention_resolver(resolver).await.unwrap();
    ids
}

/// The resolver's canonical bind. Key 01003158 stands on two rows — a council
/// under a wrong number and the supplier it belongs to — so it is poisoned and a
/// third spelling mints. With the council's number withheld, the supplier owns
/// the key and the third spelling binds to it; the council's exact literal
/// still binds to the council. Key 20445111 has one owner, withheld: a
/// different spelling of it no longer binds there and mints. (Two stores: a
/// mint in the unreviewed half would itself become a second owner of the key.)
#[tokio::test]
async fn the_canonical_bind_skips_a_withheld_org_and_finds_the_rightful_owner() {
    let orgs = [
        (1, "FI", "vat", "FI01003158", "Cheltenham Borough Council"),
        (2, "FI", "national", "01003158", "Telinekataja Oy"),
        (3, "FI", "national", "20445111", "Digimune Ltd"),
    ];
    let third_spellings =
        [mention(1, "Telinekataja", "national", "0100315-8"), mention(2, "Silver Energy", "vat", "FI-20445111")];

    let (db, _conn) = bed("test-identifier-verdicts-resolver-before.db", &orgs, 2).await;
    let unreviewed = resolve(&db, &third_spellings).await;
    assert!(unreviewed[0] > 3, "two owners poison the key: the third spelling mints");
    assert_eq!(unreviewed[1], 3, "one owner: the other spelling binds to it");

    let (db, conn) = bed("test-identifier-verdicts-resolver-after.db", &orgs, 3).await;
    db.record_identifier_verdicts("452", &[verdict(1, "FI01003158", "wrong"), verdict(3, "20445111", "wrong")], 0)
        .await
        .unwrap();
    let mut mentions = third_spellings.to_vec();
    mentions.push(mention(3, "Cheltenham Borough Council", "vat", "FI01003158"));
    let ids = resolve(&db, &mentions).await;
    assert_eq!(ids[0], 2, "the rightful owner takes the key");
    assert!(ids[1] > 3, "the withheld org is no owner: a fresh mint");
    assert_eq!(ids[2], 1, "the exact literal still binds to the org that published it");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 4);
}

/// `/v1/organizations` reads the verdict beside the row: `wrong` and `related`
/// are served, `right` and no verdict are not, and a verdict on a number the org
/// no longer carries is not either.
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
}
