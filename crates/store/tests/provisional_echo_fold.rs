//! Issue 351 unit 3: the provisional-echo fold. Identical-name NULL-country
//! provisional rows collapse to one per name — unless the name is over the
//! wall, where the 234 exclusion still holds. What is pinned: the class
//! boundary, the wall gate, the survivor rule, the references moving with
//! the ledger and the parity guard, and that a dry run writes nothing.

use store::turso::{self, Value};

const STOPLIST_CAP: usize = 8;

fn n2(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (db, conn)
}

async fn org(conn: &turso::Connection, id: i64, cc: Option<&str>, name: &str) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (?, ?, NULL, NULL, ?, ?, 1, 0)",
        (
            Value::Integer(id),
            cc.map(|c| Value::Text(c.into())).unwrap_or(Value::Null),
            Value::Text(name.into()),
            Value::Text(name.to_lowercase()),
        ),
    )
    .await
    .unwrap();
}

async fn mention(conn: &turso::Connection, notice: i64, org: i64, name: &str) {
    conn.execute(
        "INSERT INTO organization_mentions (notice_id, section_id, organization_id, name, country, raw_identifier)
         VALUES (?, 'S-1', ?, ?, NULL, NULL)",
        (Value::Integer(notice), Value::Integer(org), Value::Text(name.into())),
    )
    .await
    .unwrap();
}

async fn count(conn: &turso::Connection, sql: &str) -> i64 {
    let mut rows = conn.query(sql, ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let Value::Integer(n) = row.get_value(0).unwrap() else { panic!("count") };
    n
}

fn args<'a>(dry_run: bool, expect_groups: Option<u64>, stop: &'a (dyn Fn() -> bool + Sync), progress: &'a (dyn Fn(u64, &str) + Sync)) -> store::ProvisionalFoldArgs<'a> {
    store::ProvisionalFoldArgs {
        n2,
        stoplist_cap: STOPLIST_CAP,
        dry_run,
        max_groups: None,
        expect_groups,
        job_id: Some(7),
        stop,
        progress,
    }
}

#[tokio::test]
async fn identical_null_country_provisionals_fold_under_the_wall_and_stand_over_it() {
    let (db, conn) = open("echo-fold").await;
    // Five identical rows of one city, mentions on three of them.
    for i in 0..5 {
        org(&conn, 100 + i, None, "Stadt Echo").await;
    }
    mention(&conn, 1, 100, "Stadt Echo").await;
    mention(&conn, 2, 101, "Stadt Echo").await;
    mention(&conn, 3, 104, "Stadt Echo").await;
    // A pair.
    org(&conn, 200, None, "Kreis Zwei").await;
    org(&conn, 201, None, "KREIS ZWEI").await;
    // A name over the wall: three rows, twelve carriers of its key.
    for i in 0..3 {
        org(&conn, 300 + i, None, "Gemeinde Generic").await;
    }
    for i in 0..12 {
        conn.execute(
            "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'gemeinde generic')",
            (Value::Integer(300 + i),),
        )
        .await
        .unwrap();
    }
    // Outside the class: a DE-country pair, and a singleton.
    org(&conn, 400, Some("DE"), "Stadt Echo").await;
    org(&conn, 401, Some("DE"), "Stadt Echo").await;
    org(&conn, 500, None, "Solo").await;
    // Unit 4's tiers. "Stadt Big": 30 rows of one city, over the wall by its
    // own echo — but two identified DE rows carry the key: an echo of one.
    for i in 0..30 {
        org(&conn, 600 + i, None, "Stadt Big").await;
        conn.execute("INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'stadt big')", (Value::Integer(600 + i),)).await.unwrap();
    }
    for (id, vat) in [(700, "DE700"), (701, "DE701")] {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, 'DE', 'vat', ?, 'Stadt Big', 'stadt big', 0, 0)",
            (Value::Integer(id), Value::Text(vat.into())),
        )
        .await
        .unwrap();
        conn.execute("INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'stadt big')", (Value::Integer(id),)).await.unwrap();
    }
    // "Tribunal Verdict": 30 rows, nothing identified — over the wall, but a
    // recorded `single` verdict folds it. "Kreis Zwei" gets a `generic`
    // verdict, which refuses its under-wall pair.
    for i in 0..30 {
        org(&conn, 800 + i, None, "Tribunal Verdict").await;
        conn.execute("INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', 'tribunal verdict')", (Value::Integer(800 + i),)).await.unwrap();
    }
    let recorded = db
        .record_name_verdicts(
            "t",
            &[
                store::NameVerdict { name_norm: "tribunal verdict".into(), verdict: "single".into(), rationale: "one court".into() },
                store::NameVerdict { name_norm: "kreis zwei".into(), verdict: "generic".into(), rationale: "a class name".into() },
                store::NameVerdict { name_norm: "kreis zwei".into(), verdict: "generic".into(), rationale: "re-posted".into() },
            ],
            0,
        )
        .await
        .unwrap();
    assert_eq!(recorded, 3, "an upsert counts every row it wrote");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_name_verdicts").await, 2, "one row per name");
    let never = || false;
    let quiet = |_: u64, _: &str| {};

    let dry = db.fold_provisional_echoes(args(true, None, &never, &quiet)).await.unwrap();
    assert_eq!(dry.rows_walked, 71, "5 + 2 + 3 + 30 + 30 + the singleton; the DE rows are outside the class");
    assert_eq!(dry.groups, 5);
    assert_eq!(dry.over_wall, 2, "Gemeinde Generic (over the wall, nothing identified) and Kreis Zwei (verdict) stand");
    assert_eq!(dry.tiers.get("under-wall"), Some(&1), "Stadt Echo");
    assert_eq!(dry.tiers.get("echo-of-one"), Some(&1), "Stadt Big");
    assert_eq!(dry.tiers.get("verdict-single"), Some(&1), "Tribunal Verdict");
    assert_eq!(dry.tiers.get("verdict-refused"), Some(&1), "Kreis Zwei");
    assert_eq!(dry.tiers.get("over-wall"), Some(&1), "Gemeinde Generic");
    assert_eq!((dry.plan_groups, dry.plan_rows), (3, 4 + 29 + 29));
    assert_eq!(dry.listing[0], ("stadt big".to_owned(), "Stadt Big".to_owned(), 30, 600));
    assert_eq!(dry.merged_groups, 0);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 75, "a dry run writes nothing");

    // Parity: a wet run against a plan that is not the one recorded aborts.
    let off = db.fold_provisional_echoes(args(false, Some(200), &never, &quiet)).await;
    assert!(off.is_err(), "plan 3 vs recorded 200 is outside max(2%, 50)");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 75);

    assert!(db.foreign_keys_enabled().await.unwrap(), "the writer enforces foreign keys before the fold");
    let wet = db.fold_provisional_echoes(args(false, Some(3), &never, &quiet)).await.unwrap();
    assert_eq!((wet.merged_groups, wet.removed, wet.mentions), (3, 62, 2), "two mentions moved off 101 and 104");
    assert!(db.foreign_keys_enabled().await.unwrap(), "and again after it: the wet loop's OFF is bracketed");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'stadt big' AND country IS NULL").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'stadt big' AND country = 'DE'").await, 2, "the identified rows are not the fold's");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'tribunal verdict'").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'stadt echo' AND country IS NULL").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 100 AND provisional = 1").await, 1, "the keep stays provisional");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 100").await, 3);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'kreis zwei'").await, 2, "a refusing verdict: untouched");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'gemeinde generic'").await, 3, "over the wall: untouched");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'stadt echo' AND country = 'DE'").await, 2, "outside the class: untouched");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE rule = 'p0' AND keep = 100").await, 4);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE rule = 'p0'").await, 62);

    // Idempotent: the class is now one row per admitted name, nothing to plan.
    let again = db.fold_provisional_echoes(args(true, None, &never, &quiet)).await.unwrap();
    assert_eq!((again.plan_groups, again.over_wall), (0, 2));
}
