//! Issue 359: the R2 arm takes a name gate in the DENY direction. A same-key
//! group with two named members whose folded core tokens share NOTHING is the
//! signature of a wrong identifier on one row
//! (the buyer's NIP in the winner's field), so it is left standing and listed
//! for review. Agreeing, contained and unnamed groups merge as before. The
//! injected rules mirror `r2_merge.rs`.

use store::turso::Value;

fn key(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool)> {
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
        // agree: one key.
        (10, "FI", "national", "01003158", "Telinekataja Oy", 0),
        (11, "FI", "vat", "FI01003158", "Telinekataja Oy", 1),
        // contained: one name is the other's with a qualifier — a branch.
        (20, "FI", "national", "01011975", "Ramboll Finland Oy", 0),
        (21, "FI", "vat", "FI01011975", "Ramboll Finland Oy Oulun toimisto", 1),
        // disagree: the buyer's number on the winner's row (real shape, slice
        // 359: Powiat Wadowicki / REKORD SI under one NIP).
        (30, "FI", "national", "10773381", "Powiat Wadowicki Starostwo Powiatowe", 0),
        (31, "FI", "vat", "FI10773381", "Rekord SI", 1),
        // disagree by one stranger: two agreeing rows plus a third that shares
        // no core — the whole group stands (deny is group-atomic).
        (40, "FI", "national", "20445111", "System Data", 0),
        (41, "FI", "vat", "FI20445111", "System Data", 1),
        (42, "FI", "national", "20445111", "Sano Centrum Medycyny", 1),
        // unnamed: no name evidence either way — merges.
        (50, "FI", "national", "30303030", "", 0),
        (51, "FI", "vat", "FI30303030", "", 1),
        // spelling, not identity: a Polish Ł that never decomposes, all caps,
        // and a consortium-role tail — the shapes the first gate denied by the
        // thousand (2026-09-06). Core tokens agree after folding: merges.
        (60, "FI", "national", "40404040", "Gmina Melgiew", 0),
        (61, "FI", "vat", "FI40404040", "GMINA MEŁGIEW (Lider konsorcjum)", 1),
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
        job_id: Some(9),
        stop: &|| false,
    }
}

#[tokio::test]
async fn r2_denies_disagreeing_names_lists_them_and_merges_the_rest() {
    let (db, conn) = seed("test-r2-name-gate.db").await;

    let dry = db.match_org_identifiers_r2(args(true, None)).await.expect("dry");
    assert_eq!(dry.groups, 6, "six FI keys hold two or more rows");
    assert_eq!(dry.denied_names, 2, "Powiat/Rekord and the System Data group with its stranger");
    assert_eq!(dry.plan_groups, 4, "agree, contained, unnamed and the spelling pair merge");
    assert_eq!(
        (dry.denied_cap, dry.denied_gate, dry.denied_consortium, dry.denied_legal_form, dry.denied_group_vat),
        (0, 0, 0, 0, 0),
        "the name rule is the only denial firing"
    );
    // The planner walks a hash map, so the listing's order is not a contract.
    let mut denied_keys: Vec<&str> = dry.denied_listing.iter().map(|g| g.2.as_str()).collect();
    denied_keys.sort_unstable();
    assert_eq!(denied_keys, vec!["10773381", "20445111"], "the denied groups are listed");
    assert!(!dry.denied_listing_truncated);
    let stranger_group =
        dry.denied_listing.iter().find(|g| g.2 == "20445111").expect("the System Data group is listed");
    assert_eq!(stranger_group.3.len(), 3, "every member of a denied group is listed, names included");
    assert!(stranger_group.3.iter().any(|m| m.3 == "Sano Centrum Medycyny"));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 13, "dry run wrote nothing");

    let wet = db.match_org_identifiers_r2(args(false, Some(4))).await.expect("wet");
    assert_eq!((wet.plan_groups, wet.merged_groups, wet.removed, wet.denied_names), (4, 4, 4, 2));
    assert_eq!(wet.denied_listing.len(), 0, "the listing is dry-run review material");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id IN (11, 21, 51, 61)").await, 0, "losers gone");
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM organizations WHERE id IN (30, 31, 40, 41, 42)").await,
        5,
        "a denied group is never fused, not even its agreeing pair"
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE rule = 'r2'").await, 4);
}

/// Issue 362: a reviewer's verdict on a group is the execution path for what
/// the name gate leaves standing. A HIGH `merge` whose member set is exactly
/// the live one is admitted past the gate and stamped by the wet merge; a
/// `keep` denies its group whatever the names say; a merge whose reviewed set
/// no longer matches the live group is stale and changes nothing.
#[tokio::test]
async fn a_reviewers_verdict_admits_or_denies_a_group_and_the_wet_merge_stamps_it() {
    let (db, conn) = seed("test-r2-name-gate-verdicts.db").await;
    let mv = |key: &str, members: Vec<i64>, action: &str, confidence: &str| store::MergeVerdict {
        country: "FI".into(),
        scheme: "FI:ytunnus".into(),
        key: key.into(),
        members,
        action: action.into(),
        rationale: "read by hand".into(),
        confidence: confidence.into(),
    };
    // Validation: members must be ascending and distinct; action is merge|keep.
    assert!(db.record_merge_verdicts("rev-1", &[mv("10773381", vec![31, 30], "merge", "high")], 1).await.is_err());
    assert!(db.record_merge_verdicts("rev-1", &[mv("10773381", vec![30, 31], "fuse", "high")], 1).await.is_err());
    let n = db
        .record_merge_verdicts(
            "rev-1",
            &[
                // The disagreeing pair, read as one entity (a rename): merge.
                mv("10773381", vec![30, 31], "merge", "high"),
                // The agreeing pair, read as two entities after all: keep.
                mv("01003158", vec![10, 11], "keep", "high"),
                // A merge reviewed on two members while the live group has three: stale.
                mv("20445111", vec![40, 41], "merge", "high"),
            ],
            1,
        )
        .await
        .expect("records");
    assert_eq!(n, 3);
    let (cols, rows) = db.verdict_rows("merge", Some("rev-1"), 10).await.expect("readable");
    assert_eq!((cols[0], rows.len()), ("country", 3), "the store reads back through the 356 surface");

    let dry = db.match_org_identifiers_r2(args(true, None)).await.expect("dry");
    assert_eq!(dry.denied_verdict, 1, "Telinekataja is kept by verdict");
    assert_eq!(dry.admitted_verdict, 1, "Powiat/Rekord is admitted by verdict");
    assert_eq!(dry.verdict_stale, 1, "the two-member verdict does not cover the three-member group");
    assert_eq!(dry.denied_names, 1, "…which the name rule then denies as before");
    assert_eq!(
        dry.plan_groups, 4,
        "Powiat/Rekord (verdict), Ramboll (contained), the spelling pair, the unnamed pair"
    );

    let wet = db.match_org_identifiers_r2(args(false, Some(4))).await.expect("wet");
    assert_eq!((wet.merged_groups, wet.removed), (4, 4));
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 31").await, 0, "Rekord folded into Powiat");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id IN (10, 11)").await, 2, "the kept pair stands");
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_merge_verdicts WHERE key = '10773381' AND applied_at IS NOT NULL AND job_id = 9 AND applied_action = 'merged 1 row(s) into 30'").await,
        1,
        "the admitting verdict is stamped with the merge it caused"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM org_merge_verdicts WHERE applied_at IS NULL").await,
        2,
        "the keep and the stale merge carry no stamp"
    );
    // A second run: the applied merge is stale now (its group is gone), the
    // keep still denies.
    let again = db.match_org_identifiers_r2(args(true, None)).await.expect("dry again");
    assert_eq!((again.denied_verdict, again.admitted_verdict), (1, 0));
}
