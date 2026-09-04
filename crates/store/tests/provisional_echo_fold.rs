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
    let never = || false;
    let quiet = |_: u64, _: &str| {};

    let dry = db.fold_provisional_echoes(args(true, None, &never, &quiet)).await.unwrap();
    assert_eq!(dry.rows_walked, 11, "5 + 2 + 3 + the singleton; the DE pair is outside the class");
    assert_eq!(dry.groups, 3);
    assert_eq!(dry.over_wall, 1, "Gemeinde Generic stands");
    assert_eq!((dry.plan_groups, dry.plan_rows), (2, 5));
    assert_eq!(dry.listing[0], ("stadt echo".to_owned(), "Stadt Echo".to_owned(), 5, 100));
    assert_eq!(dry.merged_groups, 0);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 13, "a dry run writes nothing");

    // Parity: a wet run against a plan that is not the one recorded aborts.
    let off = db.fold_provisional_echoes(args(false, Some(200), &never, &quiet)).await;
    assert!(off.is_err(), "plan 2 vs recorded 200 is outside max(2%, 50)");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations").await, 13);

    let wet = db.fold_provisional_echoes(args(false, Some(2), &never, &quiet)).await.unwrap();
    assert_eq!((wet.merged_groups, wet.removed, wet.mentions), (2, 5, 2), "two mentions moved off 101 and 104");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'stadt echo' AND country IS NULL").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE id = 100 AND provisional = 1").await, 1, "the keep stays provisional");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 100").await, 3);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'kreis zwei'").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE name_norm = 'gemeinde generic'").await, 3, "over the wall: untouched");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM organizations WHERE country = 'DE'").await, 2, "outside the class: untouched");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE rule = 'p0' AND keep = 100").await, 4);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM org_merge_log WHERE rule = 'p0'").await, 5);

    // Idempotent: the class is now one row per name, nothing to plan.
    let again = db.fold_provisional_echoes(args(true, None, &never, &quiet)).await.unwrap();
    assert_eq!((again.plan_groups, again.over_wall), (0, 1));
}
