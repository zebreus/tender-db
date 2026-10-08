//! Issue 366 unit 3: the read layer and the fold elect the SAME amount and the
//! SAME deadline.
//!
//! Until this pinned it they did not, and the divergence was realised rather
//! than theoretical. `head_value_eur_cents`/`head_deadline` filter sentinels,
//! the EUR 100bn ceiling and the ten-year deadline horizon (aa732c5), and the
//! standing rows were drained to match (2026-09-09/10) — but
//! `tender_select_head`, shared by every list shape AND the detail payload,
//! kept computing a raw `MAX(a.cents)` and a raw latest deadline. So prod
//! served tender 4490098 a `value` of EUR 4.97e16 in a response whose own
//! `amounts` array carried the EUR 50,000 the fold had elected, and 3323836 a
//! `submission_deadline` of 3005-07-06 while `status` and `sort=deadline` used
//! the real 2005-06-15.
//!
//! Everything below goes through `apply_tenders` — the fold's own path — so the
//! head columns are written by `head_value_eur_cents`/`head_deadline`
//! themselves and the satellites come from the same facts. Nothing is asserted
//! against a literal that a stale read pick could match by coincidence: the
//! expectations are read back off the stored head columns. Revert either side
//! and these fail.

use store::canonical::{
    DEADLINE_FLOOR_SECS, DEADLINE_HORIZON_SECS, Fact, TenderProjection, TenderVersion, deadline_admitted,
    deadline_admitted_sql,
};
use store::read::{self, Filter, Scope};
use store::turso::{self, Value};

/// A real 2005 instant, so the deadline arithmetic below reads as dates.
const PUBLISHED_AT: i64 = 1_118_000_000;

fn path(name: &str) -> String {
    format!("/tmp/tender-db-hea-{name}-{}.db", std::process::id())
}

async fn open(name: &str) -> (store::Db, turso::Connection) {
    let path = path(name);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = store::Db::open(&path).await.unwrap();
    db.set_foreign_keys(false).await.unwrap();
    let raw = turso::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    (db, conn)
}

fn amount(field: &str, cents: i64) -> Fact {
    Fact::Amount { field: field.into(), cents, currency: "EUR".into(), tax_basis: None, quality: None }
}

fn deadline(utc_seconds: i64) -> Fact {
    Fact::Date {
        field: "submission_deadline".into(),
        utc_seconds,
        offset_minutes: 0,
        has_time: true,
    }
}

fn projection(notice: i64, facts: Vec<Fact>) -> TenderProjection {
    TenderProjection {
        source: "ted".into(),
        procedure_key: Some(format!("pk-{notice}")),
        island_notice_id: None,
        kind: "procedure".into(),
        versions: vec![TenderVersion {
            caused_by_notice_id: notice,
            published_at: PUBLISHED_AT,
            dispatched_at: None,
            notice_subtype: None,
            original_lang: None,
            publication_id: format!("{notice}-2005"),
            facts: facts.into_iter().collect(),
            lots: Vec::new(),
            rounds: Vec::new(),
            group_members: Vec::new(),
        }],
    }
}

/// What the FOLD decided, read back off the head columns it wrote.
async fn head(conn: &turso::Connection, id: i64) -> (Option<i64>, Option<i64>) {
    let mut rows = conn
        .query(
            "SELECT current_value_eur_cents, current_deadline FROM tenders WHERE id = ?",
            [Value::Integer(id)],
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    (
        row.get_value(0).unwrap().as_integer().copied(),
        row.get_value(1).unwrap().as_integer().copied(),
    )
}

async fn only_row(conn: &turso::Connection) -> read::TenderRow {
    let rows = read::tenders(conn, &Filter::default(), Scope::Page { after: 0, limit: 10 })
        .await
        .unwrap();
    assert_eq!(rows.len(), 1, "one fixture tender");
    rows.into_iter().next().unwrap()
}

#[tokio::test]
async fn the_list_row_serves_the_amount_and_deadline_the_fold_elected() {
    let (db, conn) = open("agree").await;

    // 4490098's shape: a real EUR 50,000 estimate beside a 10^10-scaled result
    // value the ceiling refuses. 3323836's shape: one notice publishing two
    // deadlines, the later one a typo a thousand years out.
    let real = 5_000_000;
    let junk = 4_970_000_000_000_000_000;
    let plausible = PUBLISHED_AT + 10 * 86_400;
    let millennium = PUBLISHED_AT + DEADLINE_HORIZON_SECS + 86_400;

    db.apply_tenders(
        &[projection(
            1,
            vec![
                amount("estimated_value", real),
                amount("result_value", junk),
                deadline(plausible),
                deadline(millennium),
            ],
        )],
        PUBLISHED_AT,
        false,
    )
    .await
    .unwrap();

    let (want_value, want_deadline) = head(&conn, 1).await;
    // The fold's side of the contract, so a regression there is named here
    // rather than showing up as a confusing read-layer failure.
    assert_eq!(want_value, Some(real), "the fold refuses the over-ceiling amount");
    assert_eq!(want_deadline, Some(plausible), "the fold refuses the beyond-horizon date");

    let row = only_row(&conn).await;
    assert_eq!(
        row.value_cents, want_value,
        "the list row must serve the elected amount, not MAX(a.cents) — this is the \
         4490098 divergence, where the payload showed 4.97e16 and the fold had chosen 50,000"
    );
    assert_eq!(row.currency.as_deref(), Some("EUR"));
    assert_eq!(
        row.deadline.as_ref().map(|d| d.utc_seconds),
        want_deadline,
        "the list row must serve the elected deadline, not the latest published one — \
         the 3323836 divergence"
    );
}

/// A Tender whose ONLY amount is refused has no known value, and the read layer
/// must say so rather than falling back to the published figure. Issue 366
/// recorded this as a consequence rather than a bug: such a Tender is returned
/// by NEITHER `min_value` nor `max_value`, so the payload agreeing is what keeps
/// a caller who filters and a caller who reads from seeing different corpora.
#[tokio::test]
async fn a_tender_whose_only_amount_is_refused_serves_no_value() {
    let (db, conn) = open("null").await;

    // The SDK's withheld marker under BT-195/FieldsPrivacy: 15,529 tenders in
    // prod carried exactly this and served it as their value.
    db.apply_tenders(&[projection(1, vec![amount("estimated_value", -100)])], PUBLISHED_AT, false)
        .await
        .unwrap();

    let (want_value, _) = head(&conn, 1).await;
    assert_eq!(want_value, None, "electing over an empty admitted set yields None");

    let row = only_row(&conn).await;
    assert_eq!(row.value_cents, None, "-1.00 must not be served as the row's value");
    assert_eq!(row.currency, None, "and no currency comes with a value that is not there");
}

/// Issue 471 unit 4(a), through the FOLD's path: the exact-10^k rule reads the
/// whole chain at the one call site (`write_tender`'s head update), so a
/// partner that only an EARLIER version carries (6721266's shape) still
/// refuses the head's figure. The read layer has no rule of its own for the
/// tender value: it finds the fold's row by `eur_cents`, so the served
/// `value`, `?min_value` and the stored column must all name the same row,
/// with the refused figure BIGGER than the elected one.
#[tokio::test]
async fn a_scale_slip_partnered_only_by_an_earlier_version_is_refused_by_the_fold_and_the_row() {
    let (db, conn) = open("scale-chain").await;
    // EUR 12.3 bn in the head, exactly 10^3 over the earlier version's
    // estimate, and published in one field only; a real EUR 5 m beside it.
    let partner = 1_234_567_891;
    let slip = partner * 1_000;
    let real = 500_000_000;
    let mut p = projection(1, vec![amount("estimated_value", partner)]);
    let mut head_version = p.versions[0].clone();
    head_version.caused_by_notice_id = 2;
    head_version.publication_id = "2-2005".into();
    head_version.published_at = PUBLISHED_AT + 86_400;
    head_version.facts = [amount("estimated_value", slip), amount("result_value", real)].into_iter().collect();
    p.versions.push(head_version);
    db.apply_tenders(&[p], PUBLISHED_AT + 86_400, false).await.unwrap();

    let (want_value, _) = head(&conn, 1).await;
    assert_eq!(want_value, Some(real), "the fold must read the earlier version's partner");

    let row = only_row(&conn).await;
    assert_eq!(row.value_cents, want_value, "the row serves the fold's row, not the refused bigger one");
    assert_eq!(row.currency.as_deref(), Some("EUR"));
    let detail = read::tender_detail(&conn, 1, None).await.unwrap().expect("the tender");
    assert_eq!(detail.tender.value_cents, want_value, "the detail payload serves the same row");

    let at_least = |min: i64| Filter { min_value: Some(min), ..Filter::default() };
    let n = |f: Filter| {
        let conn = &conn;
        async move { read::tenders(conn, &f, Scope::Page { after: 0, limit: 10 }).await.unwrap().len() }
    };
    assert_eq!(n(at_least(real)).await, 1, "?min_value at the elected figure finds it");
    assert_eq!(n(at_least(real + 1)).await, 0, "?min_value above it does not, whatever the refused figure says");
}

/// Issue 471 unit 4(a), the lot half (224156's shape): a lot's
/// `framework_maximum` 10^3 over a SIBLING lot's is refused by the head
/// election, and the per-lot pick in `summarise` calls the same predicate, so
/// the refused figure is not served as that lot's value while `/v1/lots`'
/// value filter reads the head column that refused it.
#[tokio::test]
async fn a_refused_lot_figure_is_not_served_as_the_lots_value() {
    use store::canonical::LotState;
    let (db, conn) = open("scale-lot").await;
    let small = 3_000_000_000; // EUR 30 m
    let slip = small * 1_000; // EUR 30 bn, in the band
    let lot = |key: &str, cents: i64| LotState {
        key: key.into(),
        kind: "Lot".into(),
        facts: [amount("framework_maximum", cents)].into_iter().collect(),
    };
    let mut p = projection(1, Vec::new());
    p.versions[0].lots = vec![lot("LOT-0001", small), lot("LOT-0002", slip)];
    db.apply_tenders(&[p], PUBLISHED_AT, false).await.unwrap();

    assert_eq!(head(&conn, 1).await.0, Some(small), "the head election refuses the sibling-lot slip");

    let lots = read::lots(&conn, &Filter { tender: Some(1), ..Filter::default() }, Scope::Page { after: 0, limit: 10 })
        .await
        .unwrap();
    let value = |key: &str| lots.iter().find(|l| l.lot_key == key).map(|l| l.value_cents).expect(key);
    assert_eq!(value("LOT-0001"), Some(small));
    assert_eq!(value("LOT-0002"), None, "the refused ceiling is not served as LOT-0002's value");

    // Below the EUR 1 bn gate nothing changes: the same shape at EUR 30 m over 30,000
    // keeps both lots' figures (the rule is gated to the decades it was measured on).
    let (db2, conn2) = open("scale-lot-below").await;
    let mut p = projection(1, Vec::new());
    p.versions[0].lots = vec![lot("LOT-0001", 3_000_000), lot("LOT-0002", 3_000_000_000)];
    db2.apply_tenders(&[p], PUBLISHED_AT, false).await.unwrap();
    assert_eq!(head(&conn2, 1).await.0, Some(3_000_000_000));
    let lots = read::lots(&conn2, &Filter { tender: Some(1), ..Filter::default() }, Scope::Page { after: 0, limit: 10 })
        .await
        .unwrap();
    let value = |key: &str| lots.iter().find(|l| l.lot_key == key).map(|l| l.value_cents).expect(key);
    assert_eq!(value("LOT-0002"), Some(3_000_000_000));
}

/// Issue 490: the fold STORES each lot's elected value in `tender_version_lots`,
/// and it is the value the REST lot row serves -- one function
/// (`canonical::elect_lot_value`) decides both. Every skip is exercised, plus
/// the two things a stored per-version value must get right: a scale partner
/// that sits in an EARLIER version, and the tie order.
#[tokio::test]
async fn the_stored_lot_value_is_the_value_the_lot_row_serves() {
    use store::canonical::LotState;
    let (db, conn) = open("stored-lot-value").await;
    let fig = |field: &str, cents: i64, currency: &str, quality: Option<&str>| Fact::Amount {
        field: field.into(),
        cents,
        currency: currency.into(),
        tax_basis: None,
        quality: quality.map(Into::into),
    };
    let lot = |key: &str, facts: Vec<Fact>| LotState { key: key.into(), kind: "Lot".into(), facts: facts.into_iter().collect() };

    let small = 3_000_000_000; // EUR 30 m, published in version 1 only
    let mut p = projection(1, Vec::new());
    p.versions[0].lots = vec![lot("LOT-A", vec![fig("estimated_value", small, "EUR", None)])];
    let mut v2 = p.versions[0].clone();
    v2.caused_by_notice_id = 2;
    v2.published_at = PUBLISHED_AT + 86_400;
    v2.publication_id = "2-2005".into();
    v2.lots = vec![
        // x1000 the EARLIER version's figure: refused, the next figure wins.
        lot("LOT-A", vec![fig("framework_maximum", small * 1_000, "EUR", None), fig("estimated_value", 2_000_000, "EUR", None)]),
        lot("LOT-B", vec![fig("estimated_value", 100, "EUR", None)]), // one unit: a token
        lot("LOT-C", vec![fig("estimated_value", 0, "EUR", None)]),   // zero: an absence
        lot("LOT-D", vec![fig("estimated_value", -100, "EUR", Some("withheld"))]),
        // Over the EUR 100 bn ceiling, beside a real figure.
        lot("LOT-E", vec![fig("result_value", 20_000_000_000_000_000, "EUR", None), fig("estimated_value", 5_000_000, "EUR", None)]),
        // No rate for the currency: still served, as published, with no EUR.
        lot("LOT-F", vec![fig("estimated_value", 7_000_000, "XYZ", None)]),
        // A tie on published cents: the first in Fact order (field, then cents, then currency) keeps it.
        lot("LOT-G", vec![fig("framework_maximum", 4_000_000, "XYZ", None), fig("estimated_value", 4_000_000, "EUR", None)]),
    ];
    p.versions.push(v2);
    db.apply_tenders(&[p], PUBLISHED_AT + 86_400, false).await.unwrap();

    let eur = |c: i64| (Some(c), Some("EUR".to_owned()), Some(c));
    let none = (None, None, None);
    let want: Vec<(&str, (Option<i64>, Option<String>, Option<i64>))> = vec![
        ("LOT-A", eur(2_000_000)),
        ("LOT-B", none.clone()),
        ("LOT-C", none.clone()),
        ("LOT-D", none.clone()),
        ("LOT-E", eur(5_000_000)),
        ("LOT-F", (Some(7_000_000), Some("XYZ".to_owned()), None)),
        ("LOT-G", eur(4_000_000)),
    ];
    let got = stored_lots(&conn, 2).await;
    assert_eq!(
        got.iter().map(|(k, c, cur, e)| (k.as_str(), (*c, cur.clone(), *e))).collect::<Vec<_>>(),
        want,
        "the fold's stored lot values"
    );
    assert_eq!(stored_lots(&conn, 1).await, vec![("LOT-A".to_owned(), Some(small), Some("EUR".to_owned()), Some(small))], "version 1 keeps its own");

    // The REST lot rows serve exactly what is stored.
    let lots = read::lots(&conn, &Filter { tender: Some(1), ..Filter::default() }, Scope::Page { after: 0, limit: 100 })
        .await
        .unwrap();
    assert_eq!(lots.len(), want.len());
    for row in &lots {
        let (_, c, cur, _) = got.iter().find(|(k, ..)| *k == row.lot_key).expect("stored row");
        assert_eq!((&row.value_cents, &row.currency), (c, cur), "lot {}: REST serves the stored value", row.lot_key);
    }
}

/// A chain of versions for tender `pk-1`, one per facts list, notices 1.., a day apart.
fn chain(versions: Vec<(Vec<Fact>, Vec<store::canonical::LotState>)>) -> TenderProjection {
    let mut p = projection(1, Vec::new());
    let first = p.versions.remove(0);
    p.versions = versions
        .into_iter()
        .enumerate()
        .map(|(i, (facts, lots))| TenderVersion {
            caused_by_notice_id: i as i64 + 1,
            published_at: PUBLISHED_AT + i as i64 * 86_400,
            publication_id: format!("{}-2005", i + 1),
            facts: facts.into_iter().collect(),
            lots,
            ..first.clone()
        })
        .collect();
    p
}

/// The stored lot values of one version, by lot key.
async fn stored_lots(conn: &turso::Connection, seq: i64) -> Vec<(String, Option<i64>, Option<String>, Option<i64>)> {
    let mut rows = conn
        .query(
            "SELECT l.lot_key, vl.value_cents, vl.value_currency, vl.value_eur_cents
               FROM tender_version_lots vl JOIN lots l ON l.id = vl.lot_id
              WHERE vl.tender_id = 1 AND vl.seq = ? ORDER BY l.lot_key",
            [Value::Integer(seq)],
        )
        .await
        .unwrap();
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        out.push((
            row.get_value(0).unwrap().as_text().unwrap().to_owned(),
            row.get_value(1).unwrap().as_integer().copied(),
            row.get_value(2).unwrap().as_text().map(|s| s.to_owned()),
            row.get_value(3).unwrap().as_integer().copied(),
        ));
    }
    out
}

/// Issue 490: on the daily path a Tender is APPENDED to (`keep > 0`), and the
/// fold seeds its running scale rule from the kept versions. A slip whose only
/// partner sits in a KEPT version must still be refused -- by the head election
/// and by the appended version's stored lot value -- exactly as a one-shot fold
/// of the same chain refuses it (6721266's shape, on the incremental path).
#[tokio::test]
async fn a_slip_partnered_only_by_a_kept_version_is_refused_on_the_append_path() {
    use store::canonical::LotState;
    let x = 3_000_000_000; // EUR 30 m, published in version 1 only
    let v1 = (vec![amount("estimated_value", x)], Vec::new());
    let v2 = (
        // The slip is published ONCE (the lot's framework_maximum): the same figure
        // under a second field would be corroborated and rightly kept.
        vec![amount("estimated_value", 5_000_000)],
        vec![LotState {
            key: "LOT-A".into(),
            kind: "Lot".into(),
            facts: [amount("framework_maximum", x * 1_000), amount("estimated_value", 2_000_000)].into_iter().collect(),
        }],
    );

    let (db, conn) = open("append").await;
    db.apply_tenders(&[chain(vec![v1.clone()])], PUBLISHED_AT, false).await.unwrap();
    let appended = db.apply_tenders(&[chain(vec![v1.clone(), v2.clone()])], PUBLISHED_AT + 86_400, false).await.unwrap();
    assert_eq!(appended.versions_written, 1, "an append, not a rewrite: version 1 was kept");

    let (fresh_db, fresh) = open("append-oneshot").await;
    fresh_db.apply_tenders(&[chain(vec![v1.clone(), v2.clone()])], PUBLISHED_AT + 86_400, false).await.unwrap();

    assert_eq!(head(&conn, 1).await.0, Some(5_000_000), "the kept version's partner refuses the head slip");
    assert_eq!(head(&conn, 1).await.0, head(&fresh, 1).await.0, "append == one-shot, head");
    assert_eq!(
        stored_lots(&conn, 2).await,
        vec![("LOT-A".to_owned(), Some(2_000_000), Some("EUR".to_owned()), Some(2_000_000))],
        "the kept version's partner refuses the lot slip"
    );
    assert_eq!(stored_lots(&conn, 2).await, stored_lots(&fresh, 2).await, "append == one-shot, lot values");

    let again = db.apply_tenders(&[chain(vec![v1, v2])], PUBLISHED_AT + 2 * 86_400, false).await.unwrap();
    assert_eq!(again.tenders_unchanged, 1, "the stored chain is still the state key");
}

/// Issue 490: a chain that SHRINKS can leave the write loop with nothing to
/// write (`keep == p.versions.len()`), and the head election must still see the
/// surviving head's fields. Version 2 publishes a EUR 30 bn figure under TWO
/// fields -- corroborated, so the exact-10^k rule keeps it although version 1
/// carries its 10^3 partner. If the running rule's head were left empty, the
/// figure would be refused.
#[tokio::test]
async fn a_shrunk_chain_still_elects_a_corroborated_figure_of_its_new_head() {
    let x = 3_000_000_000;
    let v1 = (vec![amount("estimated_value", x)], Vec::new());
    let v2 = (vec![amount("estimated_value", x * 1_000), amount("result_value", x * 1_000)], Vec::new());
    let v3 = (vec![amount("estimated_value", 7_000)], Vec::new());
    let (db, conn) = open("shrink").await;
    db.apply_tenders(&[chain(vec![v1.clone(), v2.clone(), v3])], PUBLISHED_AT + 2 * 86_400, false).await.unwrap();
    assert_eq!(head(&conn, 1).await.0, Some(7_000));
    let shrunk = db.apply_tenders(&[chain(vec![v1, v2])], PUBLISHED_AT + 3 * 86_400, false).await.unwrap();
    assert_eq!((shrunk.versions_written, shrunk.versions_removed), (0, 1), "nothing to write, one version removed");
    assert_eq!(head(&conn, 1).await.0, Some(x * 1_000), "the corroborated figure of the new head is elected");
}

/// Issue 171 (rule 12): the near side of the same window. Prod served five head
/// deadlines before 1990 on 2026-09-26 — 5671586's year 0016 (a two-digit
/// year), 1466977's `1970-01-01` in a 2024 notice — and every one of them was
/// its tender's ONLY deadline, so MAX had nothing better to pick and they sorted
/// first on `sort=deadline&order=asc`. Both shapes are pinned: the typo beside a
/// real date (MAX already coped; the floor must not break it) and the typo
/// alone, which must read as no deadline on the fold's column AND on the row.
#[tokio::test]
async fn a_deadline_before_the_floor_is_refused_by_the_fold_and_the_row_alike() {
    let (db, conn) = open("floor").await;

    // Year 0016 — what a two-digit `16` becomes when parsed as a full year.
    let year_16 = -61_648_419_600;
    let epoch_zero = 0;
    let plausible = PUBLISHED_AT + 10 * 86_400;
    assert!(year_16 < DEADLINE_FLOOR_SECS && epoch_zero < DEADLINE_FLOOR_SECS);

    db.apply_tenders(
        &[
            projection(1, vec![deadline(year_16), deadline(plausible)]),
            projection(2, vec![deadline(epoch_zero)]),
            // Exactly ON the floor is admitted: the boundary is where two
            // transcriptions of one rule drift apart.
            projection(3, vec![deadline(DEADLINE_FLOOR_SECS)]),
        ],
        PUBLISHED_AT,
        false,
    )
    .await
    .unwrap();

    assert_eq!(head(&conn, 1).await.1, Some(plausible), "the real date still wins");
    assert_eq!(head(&conn, 2).await.1, None, "an epoch-zero-only tender has no deadline");
    assert_eq!(head(&conn, 3).await.1, Some(DEADLINE_FLOOR_SECS), "the floor is inclusive");

    let rows = read::tenders(&conn, &Filter::default(), Scope::Page { after: 0, limit: 10 })
        .await
        .unwrap();
    assert_eq!(rows.len(), 3);
    for row in rows {
        assert_eq!(
            row.deadline.as_ref().map(|d| d.utc_seconds),
            head(&conn, row.id).await.1,
            "tender {}: the row must serve the deadline the fold elected",
            row.id
        );
    }
}

/// The deadline pick calls canonical's window (issue 474), and the amount pick reuses the fold's elected value instead of re-deriving it.
/// Both are the anti-drift property this issue exists for: a literal or a
/// transcribed digit walk would be a second copy free to disagree.
#[test]
fn the_read_layer_reuses_the_election_rather_than_repeating_it() {
    let (sql, _) = read::tenders_ordered_statement(
        &Filter::default(),
        read::HeadOrder::Deadline,
        false,
        None,
        25,
    );
    assert!(
        sql.contains(&deadline_admitted_sql("s.utc_seconds", "v.published_at")),
        "the deadline pick must apply canonical's window, not a transcription of it: {sql}"
    );
    assert!(
        sql.contains("s.eur_cents = t.current_value_eur_cents"),
        "the amount pick must reuse the fold's elected value rather than re-deriving it: {sql}"
    );
    assert!(
        !sql.contains("MAX(a.cents)"),
        "the raw published extremum must be gone from the display pick: {sql}"
    );
}

/// Issue 375's "Done when": **no code path writes the head columns with a rule
/// that differs from the fold's election.** That is a property of the whole
/// source, not of any one function, so it is checked the only way such a thing
/// can be — by counting the writers.
///
/// The history is why it is worth a test. `current_value_eur_cents` had three
/// writers (the fold, a backfill walk, and the read layer recomputing its own
/// display value) and `current_deadline` had the same three. Two of the six
/// diverged silently when the election grew filters in `aa732c5`, and one of
/// those two ran AUTOMATICALLY as the second half of `rederive-eur`. None of it
/// was catchable by a unit test of any single writer, because each was
/// self-consistent; the defect only existed between them.
///
/// If this test fails, the question to ask is not "is the new writer correct
/// today" but "what makes it stay correct when the election next changes".
#[test]
fn the_head_columns_have_exactly_one_writer_that_decides_them() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut writers: Vec<String> = Vec::new();
    for rel in ["src/lib.rs", "src/canonical.rs", "src/read.rs", "src/rates.rs"] {
        let text = std::fs::read_to_string(root.join(rel)).expect(rel);
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if code.contains("current_value_eur_cents =") || code.contains("current_deadline =") {
                writers.push(format!("{rel}:{}", n + 1));
            }
        }
    }
    assert_eq!(
        writers.len(),
        2,
        "expected exactly two writers of the head columns — the fold's own write, and the \
         deadline backfill, which applies the window through `canonical::deadline_admitted_sql`. \
         Found: {writers:?}. A third writer is how issue 375 happened: it will agree with the \
         fold on the day it is written and diverge the next time the election grows a filter."
    );
}

/// Every `.rs` file under `dir`, recursively, in a stable order.
fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).expect("readable dir").map(|e| e.unwrap().path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// Issue 474: the submission-deadline window has ONE home, `canonical`'s
/// `deadline_admitted` and `deadline_admitted_sql`. It used to be written out by
/// hand at five sites, and a change to it (the horizon, `aa732c5`) reached one
/// copy and the others over 18 days and three fixes — one of the gaps served on
/// prod. The writer count above cannot see this: three of the five were readers.
///
/// (a) Outside `canonical.rs` no code line names the two constants or retypes
///     their values (the data-quality sentinel detector's two `SENTINEL_DATE_*`
///     definitions are the one named exception: it ranks candidates and elects
///     nothing). Inside it, only the two definitions and the two helpers' body
///     lines do — matched by exact text, each exactly once.
/// (b) The code lines in store and app that select `submission_deadline` facts
///     are counted, because the 366 drift was a raw `MAX` that named no constant
///     at all. A new reader fails this until it is moved onto a helper and the
///     count is raised in the same commit.
#[test]
fn the_deadline_window_is_written_once() {
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    // The window's names, and its numbers in every spelling a retyped copy would
    // plausibly take (the horizon is 315_360_000 s).
    let names = [
        "DEADLINE_FLOOR_SECS",
        "DEADLINE_HORIZON_SECS",
        "631_152_000",
        "631152000",
        "10 * 365 * 86_400",
        "315_360_000",
        "315360000",
    ];
    // Any SQL literal naming the field, so `IN ('submission_deadline', ..)` counts as
    // a reader too. Still literal: a `LIKE '%deadline%'` or a field bound from a
    // variable is not seen — the count below is a tripwire, not a proof.
    let selectors = ["'submission_deadline'", "Some(\"submission_deadline\")", "== \"submission_deadline\""];
    // The only code lines that may name the window, by exact text, each expected
    // exactly once: the two definitions, the two helper bodies, and the detector's
    // two reads (the spec's one named exception). Exact text rather than "inside a
    // fn named so", so a copy placed beside the helpers — under any header form —
    // is still a stray (issue 474 review).
    let allowed_lines: [(&str, &str); 6] = [
        ("store/src/canonical.rs", "pub const DEADLINE_HORIZON_SECS: i64 = 10 * 365 * 86_400;"),
        ("store/src/canonical.rs", "pub const DEADLINE_FLOOR_SECS: i64 = 631_152_000;"),
        ("store/src/canonical.rs", "utc >= DEADLINE_FLOOR_SECS && utc - published_at <= DEADLINE_HORIZON_SECS"),
        (
            "store/src/canonical.rs",
            "\"({utc} >= {DEADLINE_FLOOR_SECS} AND {utc} - {published_at} <= {DEADLINE_HORIZON_SECS})\"",
        ),
        (
            "ingest/src/data_quality.rs",
            "const SENTINEL_DATE_HORIZON_SECS: i64 = store::canonical::DEADLINE_HORIZON_SECS;",
        ),
        ("ingest/src/data_quality.rs", "const SENTINEL_DATE_FLOOR: i64 = store::canonical::DEADLINE_FLOOR_SECS;"),
    ];

    let mut files = Vec::new();
    for krate in std::fs::read_dir(&crates).unwrap().map(|e| e.unwrap().path()) {
        let src = krate.join("src");
        if krate.file_name().is_some_and(|n| n != "vendor") && src.is_dir() {
            rust_files(&src, &mut files);
        }
    }
    assert!(files.iter().any(|f| f.ends_with("store/src/canonical.rs")), "the scan must reach canonical.rs");

    let mut stray: Vec<String> = Vec::new();
    let mut readers: Vec<String> = Vec::new();
    let mut seen = [0usize; 6];
    for file in &files {
        let rel = file.strip_prefix(&crates).unwrap().to_string_lossy().into_owned();
        let text = std::fs::read_to_string(file).unwrap();
        let counted = rel.starts_with("store/src/") || rel.starts_with("app/src/");
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            let trimmed = code.trim();
            let at = format!("{rel}:{}", n + 1);
            if names.iter().any(|name| code.contains(name)) {
                match allowed_lines.iter().position(|&(f, l)| f == rel && l == trimmed) {
                    Some(i) => seen[i] += 1,
                    None => stray.push(at.clone()),
                }
            }
            if counted && selectors.iter().any(|s| code.contains(s)) {
                readers.push(at);
            }
        }
    }
    assert_eq!(
        seen,
        [1; 6],
        "each allowed line of the deadline window must appear exactly once ({allowed_lines:?}); \
         a second copy of a helper's body is a hand copy too, and a missing one means this \
         list is stale against canonical.rs / data_quality.rs (issue 474)."
    );
    assert!(
        stray.is_empty(),
        "the deadline window is written by hand outside canonical's helpers at {stray:?}. \
         Call `canonical::deadline_admitted` (Rust) or `canonical::deadline_admitted_sql` (SQL) \
         instead, so the next change to the window reaches every site (issue 474)."
    );
    assert_eq!(
        readers.len(),
        6,
        "the code lines selecting `submission_deadline` facts changed: {readers:?}. Expected six — \
         the fold's `head_deadline`, the read pick, the lots `status` EXISTS (two rows), the lot \
         row in `summarise`, and the backfill — each applying canonical's window. A new reader \
         must call `deadline_admitted`/`deadline_admitted_sql` and raise this count in the same \
         commit (issue 474; issue 366 was a reader that named no constant)."
    );
}

/// Issue 474: the SQL fragment and the Rust predicate are one window, checked
/// at both edges ±1 by evaluating the fragment on a store connection.
#[tokio::test]
async fn the_sql_window_and_the_rust_window_agree_at_every_edge() {
    let (_db, conn) = open("window-edges").await;
    let published = PUBLISHED_AT;
    let mut cases = Vec::new();
    for d in [-1, 0, 1] {
        cases.push((DEADLINE_FLOOR_SECS + d, published));
        cases.push((published + DEADLINE_HORIZON_SECS + d, published));
    }
    // Both edges must actually be exercised, in both directions.
    assert!(cases.iter().any(|&(u, p)| deadline_admitted(u, p)));
    assert!(cases.iter().filter(|&&(u, p)| !deadline_admitted(u, p)).count() == 2);
    for (utc, published_at) in cases {
        let sql = format!("SELECT CASE WHEN {} THEN 1 ELSE 0 END", deadline_admitted_sql("?1", "?2"));
        let mut rows = conn
            .query(&sql, [Value::Integer(utc), Value::Integer(published_at)])
            .await
            .unwrap();
        let got = rows.next().await.unwrap().unwrap().get_value(0).unwrap().as_integer().copied();
        assert_eq!(
            got,
            Some(deadline_admitted(utc, published_at) as i64),
            "utc {utc}, published {published_at}: the SQL window disagrees with the Rust one ({sql})"
        );
    }
}
