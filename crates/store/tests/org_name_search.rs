//! Issue 217-B: the name-ordered organization search. The load-bearing cases are
//! the ones ASCII shortcuts get wrong — an umlauted name must match either case
//! (Rust `to_lowercase`, not SQL `lower()`) — plus the keyset tie walk and the
//! per-row companion filters, mirroring the published_order suite's shape.

use store::read::{self, Filter};
use store::turso::{self, Value};

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = format!("/tmp/tender-db-orgname-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    (db, conn)
}

async fn org(conn: &turso::Connection, id: i64, name: &str, country: &str) {
    conn.execute(
        "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
         VALUES (?, ?, NULL, NULL, ?, ?, 1, 0)",
        (
            Value::Integer(id),
            Value::Text(country.into()),
            Value::Text(name.into()),
            Value::Text(name.to_lowercase()),
        ),
    )
    .await
    .unwrap();
}

async fn ids(
    conn: &turso::Connection,
    filter: &Filter,
    prefix: &str,
    cursor: Option<(String, i64)>,
    limit: i64,
) -> Vec<i64> {
    read::organizations_by_name(conn, filter, prefix, cursor, limit)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect()
}

#[tokio::test]
async fn the_prefix_search_is_unicode_case_insensitive_and_tie_safe() {
    let (_db, conn) = open("search").await;
    // Names chosen so lexicographic name_norm order ≠ id order, with a NORM TIE
    // (ids 3 and 4 normalise identically) and umlauts in both cases.
    org(&conn, 1, "MÜLLER Bau GmbH", "DE").await;
    org(&conn, 2, "müller & söhne", "DE").await;
    org(&conn, 3, "Müller AG", "DE").await;
    org(&conn, 4, "MÜLLER AG", "AT").await; // same norm as 3 — the tie
    org(&conn, 5, "Mühlenwerk Nord", "DE").await;
    org(&conn, 6, "Siemens AG", "DE").await;

    let f = Filter::default();
    // Lowercased-umlaut prefix finds every casing; name_norm order, id tiebreak.
    let all = ids(&conn, &f, "mü", None, 100).await;
    assert_eq!(all, vec![5, 2, 3, 4, 1], "norm order: mühlen… < müller &… < müller ag (id tie 3<4) < müller bau");

    // One-row pages reassemble the exact list — the boundary crosses the norm tie.
    let mut paged = Vec::new();
    let mut cursor: Option<(String, i64)> = None;
    loop {
        let rows = read::organizations_by_name(&conn, &f, "mü", cursor, 1).await.unwrap();
        let Some(last) = rows.last() else { break };
        cursor = Some((last.name.to_lowercase(), last.id));
        paged.extend(rows.iter().map(|r| r.id));
    }
    assert_eq!(paged, all, "1-row pages must reassemble the list across the tie");

    // Companion country filter narrows within the slice.
    let at = ids(&conn, &Filter { country: Some("AT".into()), ..f.clone() }, "müller", None, 100).await;
    assert_eq!(at, vec![4]);

    // An absent prefix is an empty page; an unrelated prefix misses the müllers.
    assert!(ids(&conn, &f, "zz", None, 100).await.is_empty());
    assert_eq!(ids(&conn, &f, "siemens", None, 100).await, vec![6]);
}

#[tokio::test]
async fn the_name_search_honours_identifier_and_buyer_filters() {
    // Issue 284: the name-ordered builder used to apply only country/kind, so a
    // `name_prefix=…&identifier=…` (or `&buyer=…`) silently returned the whole
    // prefix slice while the handler reported the filter as honoured.
    let (_db, conn) = open("idfilter").await;
    for (id, name, ident) in
        [(10, "Siemens AG", "DE811"), (11, "Siemens Energy", "DE999"), (12, "Siement Foods", "DE811")]
    {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, name_norm, provisional, created_at)
             VALUES (?, 'DE', 'vat', ?, ?, ?, 0, 0)",
            (
                Value::Integer(id),
                Value::Text(ident.into()),
                Value::Text(name.into()),
                Value::Text(name.to_lowercase()),
            ),
        )
        .await
        .unwrap();
    }
    let f = Filter::default();
    // Prefix alone: every siemens* (12 "siement" is inside the "sieme" slice but not "siemens").
    assert_eq!(ids(&conn, &f, "siemens", None, 100).await, vec![10, 11], "prefix alone");
    // identifier narrows to the one siemens* carrying DE811 — 12 shares DE811 but is not siemens*.
    let by_ident = Filter { identifier: Some("DE811".into()), ..f.clone() };
    assert_eq!(ids(&conn, &by_ident, "siemens", None, 100).await, vec![10], "identifier must be applied");
    // buyer (= o.id) narrows to exactly that org within the slice.
    let by_buyer = Filter { buyer: Some(11), ..f.clone() };
    assert_eq!(ids(&conn, &by_buyer, "siemens", None, 100).await, vec![11], "buyer id must be applied");
}

#[tokio::test]
async fn the_backfill_stamps_unicode_lowercase_in_batches() {
    let (db, conn) = open("backfill").await;
    // Rows WITHOUT name_norm — the pre-migration population.
    for (id, name) in [(1, "MÜLLER AG"), (2, "Siemens AG"), (3, "ærøskøbing havn")] {
        conn.execute(
            "INSERT INTO organizations (id, country, identifier_kind, identifier, name, provisional, created_at)
             VALUES (?, 'DE', NULL, NULL, ?, 1, 0)",
            (Value::Integer(id), Value::Text(name.into())),
        )
        .await
        .unwrap();
    }

    let mut after = 0;
    let mut total = 0;
    loop {
        let (rows, next) = db.backfill_org_name_norm(2, after).await.unwrap();
        if rows == 0 {
            break;
        }
        total += rows;
        after = next;
    }
    assert_eq!(total, 3);

    let norm = |id: i64| async move {
        let raw = turso::Builder::new_local(&format!(
            "/tmp/tender-db-orgname-backfill-{}.db",
            std::process::id()
        ))
        .build()
        .await
        .unwrap();
        let c = raw.connect().unwrap();
        let mut rows = c
            .query("SELECT name_norm FROM organizations WHERE id = ?", [Value::Integer(id)])
            .await
            .unwrap();
        rows.next().await.unwrap().unwrap().get_value(0).unwrap().as_text().cloned().unwrap()
    };
    assert_eq!(norm(1).await, "müller ag", "Unicode lowercase, not ASCII lower()");
    assert_eq!(norm(2).await, "siemens ag");
    assert_eq!(norm(3).await, "ærøskøbing havn");

    // Idempotent: everything stamped, a re-walk touches nothing.
    let (rows, _) = db.backfill_org_name_norm(10, 0).await.unwrap();
    assert_eq!(rows, 0);
}
