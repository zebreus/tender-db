//! Issue 351: the provisional-echo census. Country-less mentions mint one
//! provisional row each, so one name can stand in dozens of identical rows;
//! this census sizes that class and says which of its largest names the wall
//! already calls generic. What is pinned: the class boundary (provisional,
//! NULL country, no identifier, a name), the grouping by `name_norm`, the
//! mentions and the wall verdict on the listed groups — and that a singleton
//! is a name, not a group.

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

async fn org(conn: &turso::Connection, id: i64, cc: Option<&str>, ident: Option<&str>, name: &str, provisional: i64) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, 0)",
        (
            Value::Integer(id),
            cc.map(|c| Value::Text(c.into())).unwrap_or(Value::Null),
            ident.map(|_| Value::Text("vat".into())).unwrap_or(Value::Null),
            ident.map(|v| Value::Text(v.into())).unwrap_or(Value::Null),
            Value::Text(name.into()),
            Value::Text(name.to_lowercase()),
            Value::Integer(provisional),
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

async fn key_rows(conn: &turso::Connection, key: &str, from: i64, n: i64) {
    for i in 0..n {
        conn.execute(
            "INSERT INTO org_match_keys (org_id, key_kind, key) VALUES (?, 'n2', ?)",
            (Value::Integer(from + i), Value::Text(key.into())),
        )
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn the_echo_class_is_grouped_by_name_and_the_wall_is_read_for_the_listed() {
    let (db, conn) = open("echo-census").await;
    // Five identical country-less provisional rows: one group of five.
    for i in 0..5 {
        org(&conn, 100 + i, None, None, "Stadt Echo", 1).await;
    }
    // A pair with a different casing in the name — still one name_norm.
    org(&conn, 200, None, None, "Kreis Zwei", 1).await;
    org(&conn, 201, None, None, "KREIS ZWEI", 1).await;
    // A singleton: a name, not a group.
    org(&conn, 300, None, None, "Solo GmbH", 1).await;
    // Outside the class: a DE-country provisional, an identified NULL-country
    // row, a nameless provisional.
    org(&conn, 400, Some("DE"), None, "Stadt Echo", 1).await;
    org(&conn, 401, None, Some("DE123"), "Stadt Echo", 0).await;
    org(&conn, 402, None, None, "", 1).await;
    mention(&conn, 1, 100, "Stadt Echo").await;
    mention(&conn, 2, 101, "Stadt Echo").await;
    mention(&conn, 3, 104, "Stadt Echo").await;
    mention(&conn, 4, 200, "Kreis Zwei").await;
    // The wall: "stadt echo" is carried by 12 org rows (over the cap of 8),
    // "kreis zwei" by 2.
    key_rows(&conn, "stadt echo", 100, 12).await;
    key_rows(&conn, "kreis zwei", 200, 2).await;

    let r = db.provisional_echo_census(n2, STOPLIST_CAP, 10, &|| false, &|_, _| {}).await.unwrap();
    assert_eq!(r.rows_walked, 8, "five + two + one; the DE row, the identified row and the nameless row are outside");
    assert_eq!(r.names, 3);
    assert_eq!(r.groups, 2);
    assert_eq!(r.rows_in_groups, 7);
    assert_eq!(r.size_hist.get("2"), Some(&1));
    assert_eq!(r.size_hist.get("3-5"), Some(&1));
    assert_eq!(r.listed.len(), 2);
    let echo = &r.listed[0];
    assert_eq!((echo.name_norm.as_str(), echo.rows, echo.mentions, echo.generic), ("stadt echo", 5, 3, true));
    assert_eq!(echo.carriers, STOPLIST_CAP as u64 + 1, "counted up to cap + 1, the wall's own bound");
    let zwei = &r.listed[1];
    assert_eq!((zwei.name_norm.as_str(), zwei.rows, zwei.mentions, zwei.generic), ("kreis zwei", 2, 1, false));
    assert_eq!(r.listed_mentions, 4);
    assert_eq!(r.listed_over_wall, 1);
    assert_eq!((echo.tier.as_str(), zwei.tier.as_str()), ("over-wall", "under-wall"), "unit 4: the tier a fold would decide by");
    assert_eq!(r.listed_tiers.get("over-wall"), Some(&1));
    assert_eq!((echo.shape.as_str(), zwei.shape.as_str()), ("id0/c0", ""), "issue 353: a pure echo's shape; none under the wall");
    assert!(!r.stopped);
}

/// The listing keeps the LARGEST groups when there are more than `cap`.
#[tokio::test]
async fn the_listing_keeps_the_largest_groups_under_the_cap() {
    let (db, conn) = open("echo-census-cap").await;
    let mut id = 1000;
    for (name, n) in [("Alpha", 2), ("Beta", 6), ("Gamma", 3), ("Delta", 4)] {
        for _ in 0..n {
            org(&conn, id, None, None, name, 1).await;
            id += 1;
        }
    }
    let r = db.provisional_echo_census(n2, STOPLIST_CAP, 2, &|| false, &|_, _| {}).await.unwrap();
    assert_eq!(r.groups, 4, "the tally is never capped");
    let listed: Vec<(&str, u64)> = r.listed.iter().map(|g| (g.name_norm.as_str(), g.rows)).collect();
    assert_eq!(listed, vec![("beta", 6), ("delta", 4)]);
}
