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
//! `sentinel_amount` refuses, and refuses nothing else. Since issue 490 the pick
//! is the fold's (`canonical::elect_lot_value`, stored on `tender_version_lots`),
//! so every fixture here is written by `apply_tenders`.

use store::canonical::{Fact, LotState, TenderProjection, TenderVersion};
use store::read::{self, Filter, Scope};
use store::turso;

const TENDER: i64 = 1;

/// One Tender, one version, the given lots -- written by the FOLD (`apply_tenders`).
/// Since issue 490 the lot row serves the value the fold stored, so a fixture that
/// hand-inserted amounts would test nothing the reader still does.
async fn fold(name: &str, lots: Vec<(&str, Vec<Fact>)>) -> turso::Connection {
    // One file per test: these run concurrently in one process, and a shared path
    // is `Busy("database is locked")`, not a failed assertion.
    let path = format!("/tmp/tender-db-lotvalue-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    db.set_foreign_keys(false).await.unwrap();
    let version = TenderVersion {
        caused_by_notice_id: 1,
        published_at: 1_700_000_000,
        dispatched_at: None,
        notice_subtype: None,
        original_lang: None,
        publication_id: "pub".into(),
        facts: Default::default(),
        lots: lots
            .into_iter()
            .map(|(key, facts)| LotState { key: key.into(), kind: "Lot".into(), facts: facts.into_iter().collect() })
            .collect(),
        rounds: Vec::new(),
        group_members: Vec::new(),
    };
    let p = TenderProjection {
        source: "ted".into(),
        procedure_key: Some("pk".into()),
        island_notice_id: None,
        kind: "procedure".into(),
        versions: vec![version],
    };
    db.apply_tenders(&[p], 1_700_000_000, false).await.unwrap();
    turso::Builder::new_local(&path).build().await.unwrap().connect().unwrap()
}

/// An amount as a notice publishes it: `cents` and `currency`, and `quality` set
/// only when the notice declared the field withheld (issue 372). The EUR sibling
/// is the fold's to derive (EUR converts 1:1 under the test's empty rates lookup;
/// other codes do not convert).
fn amount(cents: i64, currency: &str, quality: Option<&str>) -> Fact {
    Fact::Amount {
        field: "estimated_value".into(),
        cents,
        currency: currency.into(),
        tax_basis: None,
        quality: quality.map(Into::into),
    }
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
    let conn = fold(
        "zero",
        vec![
            // 1: the issue's case. One unflagged amount, exactly 0, under a NULL head.
            ("LOT-0000", vec![amount(0, "GBP", None)]),
            // 2: a zero AND a real figure. The zero must not win, and must not take
            //    the real one down with it -- the regression the naive fix makes.
            ("LOT-0001", vec![amount(0, "GBP", None), amount(50_000, "GBP", None)]),
            // 3: the control. An ordinary figure is untouched.
            ("LOT-0002", vec![amount(1_234, "EUR", None)]),
            // 4: a negative -- `sentinel_amount`'s oldest leg, and the eForms SDK's -1
            //    placeholder when a publisher writes it without the withheld marker.
            ("LOT-0003", vec![amount(-1, "EUR", None)]),
            // 5: over the ceiling. EUR 100 bn is not a procurement (issue 366); the row
            //    is refused only because its EUR conversion EXISTS to measure.
            ("LOT-0004", vec![amount(99_999_999_999_999_999, "EUR", None)]),
            // 6: an unconvertible figure of ordinary size. No conversion, so the
            //    ceiling has nothing to say -- and the published figure is still
            //    served, because the lot row carries what the publisher wrote.
            ("LOT-0005", vec![amount(900_000, "XXX", None)]),
            // 7: withheld (issue 372).
            ("LOT-0006", vec![amount(-1, "EUR", Some("withheld"))]),
        ],
    )
    .await;

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
/// say; the point is that the payload agrees with that instead of offering a
/// `0` the filter will not match.
#[tokio::test]
async fn the_lot_payload_and_the_value_filter_agree_about_a_zero() {
    let conn = fold("filter", vec![("LOT-0000", vec![amount(0, "GBP", None)])]).await;

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
