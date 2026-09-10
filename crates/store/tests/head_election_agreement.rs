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

use store::canonical::{DEADLINE_HORIZON_SECS, Fact, TenderProjection, TenderVersion};
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

/// The horizon in the SQL is interpolated from the constant, not retyped, and
/// the amount pick reuses the fold's elected value instead of re-deriving it.
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
        sql.contains(&format!("<= {DEADLINE_HORIZON_SECS}")),
        "the horizon must come from DEADLINE_HORIZON_SECS: {sql}"
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
         deadline backfill which transcribes the horizon faithfully from DEADLINE_HORIZON_SECS. \
         Found: {writers:?}. A third writer is how issue 375 happened: it will agree with the \
         fold on the day it is written and diverge the next time the election grows a filter."
    );
}
