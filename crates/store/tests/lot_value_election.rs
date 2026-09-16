//! Issue 389 unit 1: the lot headline `value` must not elect a figure the tender
//! headline refuses.
//!
//! The fold decided (issue 366, `55d239d`) that a DERIVED value column must not
//! assert an exact zero — "you cannot award a contract for nothing, and you
//! cannot cap a framework at nothing" — and `/docs` says so to every reader.
//! `tender_select_head` was brought under that rule by looking up the row the
//! fold chose (`eur_cents = t.current_value_eur_cents`), but `summarise`, which
//! builds the lot row for `/v1/lots` AND for `lot_details`, kept its own
//! `MAX(cents)` over `quality IS NULL`. So tender 25773 served `value: null` and
//! priced its only lot at `{cents: 0, currency: "GBP"}` in one response, from the
//! same published figure — and `?tender=25773&max_value=0` returned nothing,
//! because the FILTER reads the head column while the payload re-derived.
//!
//! These are not oracle tests: `lot_summary_equivalence.rs` pins `summarise`
//! against the pre-115 SQL, and this is exactly where the two are MEANT to
//! diverge. What is pinned here is that the lot pick refuses what
//! `sentinel_amount` refuses, and refuses nothing else.

use store::read::{self, Filter, Scope};
use store::turso::{self, Value};

const TENDER: i64 = 1;

async fn drain(conn: &turso::Connection, sql: &str) {
    let mut rows = conn.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

/// A Tender with one version, no head value — the shape the issue measured: the
/// fold already refused every figure on it, so `current_value_eur_cents` is NULL.
async fn fixture(name: &str) -> turso::Connection {
    // One file per test: these run concurrently in one process, and a shared path
    // is `Busy("database is locked")`, not a failed assertion.
    let path = format!("/tmp/tender-db-lotvalue-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap();
    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    drain(&conn, "PRAGMA journal_mode = WAL").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    conn.execute(
        "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, created_at)
         VALUES (?, 'ted', 'pk', 'procedure', 1, 1700000000, 1700000000)",
        (Value::Integer(TENDER),),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id)
         VALUES (?, 1, 1700000000, 'pub', 1)",
        (Value::Integer(TENDER),),
    )
    .await
    .unwrap();
    conn
}

async fn lot(conn: &turso::Connection, id: i64, key: &str) {
    conn.execute(
        "INSERT INTO lots (id, tender_id, lot_key) VALUES (?, ?, ?)",
        (Value::Integer(id), Value::Integer(TENDER), Value::Text(key.to_owned())),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO tender_version_lots (tender_id, seq, lot_id, kind) VALUES (?, 1, ?, 'Lot')",
        (Value::Integer(TENDER), Value::Integer(id)),
    )
    .await
    .unwrap();
}

/// An amount as the projection stores it: published `cents` and `currency`, the
/// derived `eur_cents` beside them (ADR-0014), and `quality` NULL unless the
/// notice declared the field withheld (issue 372).
async fn amount(
    conn: &turso::Connection,
    lot: i64,
    cents: i64,
    currency: &str,
    eur_cents: Option<i64>,
    quality: Option<&str>,
) {
    conn.execute(
        "INSERT INTO tender_version_amounts (tender_id, seq, lot_id, field, cents, currency, eur_cents, quality)
         VALUES (?, 1, ?, 'estimated_value', ?, ?, ?, ?)",
        (
            Value::Integer(TENDER),
            Value::Integer(lot),
            Value::Integer(cents),
            Value::Text(currency.to_owned()),
            eur_cents.map_or(Value::Null, Value::Integer),
            quality.map_or(Value::Null, |q| Value::Text(q.to_owned())),
        ),
    )
    .await
    .unwrap();
}

async fn served(conn: &turso::Connection) -> Vec<(String, Option<i64>, Option<String>)> {
    serve(conn, Filter { tender: Some(TENDER), ..Filter::default() }).await
}

async fn serve(
    conn: &turso::Connection,
    filter: Filter,
) -> Vec<(String, Option<i64>, Option<String>)> {
    read::lots(conn, &filter, Scope::Page { after: 0, limit: 1000 })
        .await
        .unwrap()
        .into_iter()
        .map(|r| (r.lot_key, r.value_cents, r.currency))
        .collect()
}

/// The measured case (tender 25773) and its near neighbours, in one fixture so
/// the refusal and the non-refusals are read off the same page.
#[tokio::test]
async fn a_lot_whose_only_figure_is_an_exact_zero_serves_no_value() {
    let conn = fixture("zero").await;

    // 1: the issue's case. One unflagged amount, exactly 0, under a NULL head.
    lot(&conn, 1, "LOT-0000").await;
    amount(&conn, 1, 0, "GBP", Some(0), None).await;

    // 2: a zero AND a real figure. The zero must not win, and must not take the
    //    real one down with it — this is the regression the naive fix makes.
    lot(&conn, 2, "LOT-0001").await;
    amount(&conn, 2, 0, "GBP", Some(0), None).await;
    amount(&conn, 2, 50_000, "GBP", Some(58_000), None).await;

    // 3: the control. An ordinary figure is untouched.
    lot(&conn, 3, "LOT-0002").await;
    amount(&conn, 3, 1_234, "EUR", Some(1_234), None).await;

    // 4: a negative — `sentinel_amount`'s oldest leg, and the eForms SDK's -1
    //    placeholder when a publisher writes it without the withheld marker.
    lot(&conn, 4, "LOT-0003").await;
    amount(&conn, 4, -1, "EUR", None, None).await;

    // 5: over the ceiling. €100 bn is not a procurement (issue 366); the row is
    //    refused only because its EUR conversion EXISTS to measure.
    lot(&conn, 5, "LOT-0004").await;
    amount(&conn, 5, 99_999_999_999_999_999, "EUR", Some(99_999_999_999_999_999), None).await;

    // 6: an unconvertible figure of ordinary size. No `eur_cents`, so the
    //    ceiling has nothing to say — and the published figure is still served,
    //    because the lot row carries what the publisher wrote, not a conversion.
    lot(&conn, 6, "LOT-0005").await;
    amount(&conn, 6, 900_000, "XXX", None, None).await;

    // 7: withheld (issue 372). Already refused before this change; pinned here so
    //    the new `continue`s cannot be written in a way that drops the old one.
    lot(&conn, 7, "LOT-0006").await;
    amount(&conn, 7, -1, "EUR", None, Some("withheld")).await;

    let got = served(&conn).await;
    let value = |key: &str| {
        got.iter().find(|(k, _, _)| k == key).unwrap_or_else(|| panic!("{key} missing")).clone()
    };

    assert_eq!(
        value("LOT-0000"),
        ("LOT-0000".to_owned(), None, None),
        "an exact zero is not a figure: the tender headline refuses it and so must the lot"
    );
    assert_eq!(
        value("LOT-0001"),
        ("LOT-0001".to_owned(), Some(50_000), Some("GBP".to_owned())),
        "the zero is skipped, not the whole lot"
    );
    assert_eq!(value("LOT-0002"), ("LOT-0002".to_owned(), Some(1_234), Some("EUR".to_owned())));
    assert_eq!(value("LOT-0003"), ("LOT-0003".to_owned(), None, None), "a negative is a sentinel");
    assert_eq!(value("LOT-0004"), ("LOT-0004".to_owned(), None, None), "over the ceiling");
    assert_eq!(
        value("LOT-0005"),
        ("LOT-0005".to_owned(), Some(900_000), Some("XXX".to_owned())),
        "no conversion is not a reason to blank a published figure"
    );
    assert_eq!(value("LOT-0006"), ("LOT-0006".to_owned(), None, None), "withheld, since issue 372");
}

/// The payload and the filter are the same endpoint, and issue 389's sharpest
/// form is that they disagreed on one lot. `max_value` compares the TENDER's head
/// column, so with a NULL head the filter returns nothing whatever the lot rows
/// say; the point is that the payload now agrees with that instead of offering a
/// `0` the filter will not match.
#[tokio::test]
async fn the_lot_payload_and_the_value_filter_agree_about_a_zero() {
    let conn = fixture("filter").await;
    lot(&conn, 1, "LOT-0000").await;
    amount(&conn, 1, 0, "GBP", Some(0), None).await;

    let by_max =
        serve(&conn, Filter { tender: Some(TENDER), max_value: Some(0), ..Filter::default() })
            .await;
    assert!(by_max.is_empty(), "`max_value=0` returns nothing — it reads the head column");

    let plain = served(&conn).await;
    assert_eq!(
        plain,
        vec![("LOT-0000".to_owned(), None, None)],
        "and the payload no longer prices the same lot at 0, which is the contradiction"
    );
}
