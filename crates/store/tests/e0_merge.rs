//! Issue 329: the E0 fold — exact `(country, kind, identifier)` groups that no
//! cross-walk arm keys, merged through the R2 machinery under one extra rule:
//! the named members must agree on ONE non-generic N3 key. What this pins is
//! the store side — grouping by the injected E0 key, the name rule's denial,
//! the ledger's rule column — with test-local rule implementations.

use store::turso::Value;

/// A miniature cross-walk: FI 8-digit keys (so an FI pair is R2's, never E0's).
fn arm(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool)> {
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

/// The E0 key, as `ingest::crosswalk::e0_key_flat` shapes it.
fn e0_key(country: Option<&str>, kind: &str, value: &str) -> Option<(&'static str, String, bool)> {
    if arm(country, kind, value).is_some() {
        return None;
    }
    let norm: String =
        value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    (!norm.is_empty()).then(|| ("E0", format!("{kind}:{norm}"), true))
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
        condemns: never_condemns,
        consortium: no_consortium,
        legal_form: no_legal_form,
        rule: "e0",
        n3,
        stoplist_cap: 20,
        dry_run,
        max_groups: None,
        expect_groups,
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

    let wet = db.match_org_identifiers_r2(args(false, Some(1))).await.expect("wet");
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
