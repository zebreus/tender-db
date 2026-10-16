//! Issue 533: `status=open` served procedures their newest notice had closed. A no-winner
//! result (TED 29/30/32/33/E4, legacy TED, FTS UK12 and award+contract) publishes no deadline,
//! so the head CARRIES its contract notice's and stayed open until that date passed — 38279's
//! all-`clos-nw` CAN of 2026-06-30 sat under a 2026-10-29 deadline, served open throughout.
//!
//! The rule (`## Decision 1(a) re-made`):
//!
//!     closed_by_notice(C) = !exempt(C) && own(H) non-empty && every r in all(H) is clos-nw && covered(H)
//!     exempt(C)           = K1(C) || K2(H) || E2(C) || (W(C) && !D(H))
//!
//! What this file pins, every branch through the FOLD (`apply_tenders`), never a literal a
//! stale twin could match by coincidence:
//! - each rule branch, by the Rust form over the projected chain;
//! - the agreement of the twins: per version the SQL form (`closed_by_notice_sql`) equals the
//!   Rust form (`closed_by_notice`) of that prefix, and per Tender the stored
//!   `current_closed_by_notice` equals the SQL form at `current_seq` (the issue-474 pattern);
//! - the read path: `status` on every Tenders and Lots shape, and the SSE `Scope::At` probe
//!   deciding per version; a lot is closed exactly when its Tender is (the decided per-lot leg
//!   failed 1(c)'s perf bound and is unit 3's — `canonical::closed_by_notice`'s doc);
//! - the change feed: the version that closes or reopens a Tender is a `changed` row for the
//!   Tender and for each of its lots, even when nothing else moved.

use std::collections::{BTreeMap, BTreeSet};
use store::canonical::{
    DECISION_NO_AWARD, Fact, LotResultState, LotState, OPEN_WINDOW_SECS, REASON_DISCONTINUED, Round,
    TenderProjection, TenderVersion, closed_at_version_sql, closed_by_notice, closed_by_notice_sql, closure_exempt,
    closure_exempt_sql, head_deadline,
};
use store::read::{self, Filter, HeadOrder, Scope, Status};
use store::turso::{self, Value};

/// The reference instant every `status` read compares against (2026-09-21).
const NOW: i64 = 1_790_000_000;
const DAY: i64 = 86_400;
/// When the contract notices are published: ten days before NOW.
const CN_AT: i64 = NOW - 10 * DAY;
/// When the result notices are published: two days before NOW, before every deadline.
const CAN_AT: i64 = NOW - 2 * DAY;
/// A one-shot deadline: 40 days after the CN, a month after NOW.
const DEADLINE: i64 = CN_AT + 40 * DAY;

fn path(name: &str) -> String {
    format!("/tmp/tender-db-533-{name}-{}.db", std::process::id())
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

fn deadline(utc_seconds: i64) -> Fact {
    Fact::Date { field: "submission_deadline".into(), utc_seconds, offset_minutes: 0, has_time: true }
}

fn class(scheme: &str, code: &str) -> Fact {
    Fact::Classification { field: scheme.into(), scheme: scheme.into(), code: code.into() }
}

fn lot(key: &str, kind: &str, facts: &[Fact]) -> LotState {
    LotState { key: key.into(), kind: kind.into(), facts: facts.iter().cloned().collect() }
}

fn result(key: &str, lot_key: Option<&str>, decision: Option<&str>) -> LotResultState {
    LotResultState {
        key: key.into(),
        lot_key: lot_key.map(str::to_owned),
        decision: decision.map(str::to_owned),
        reason: None,
        awarded_cents: None,
        awarded_currency: None,
        decided: None,
        winners: Vec::new(),
        buyer_winners: Vec::new(),
        statistics: Vec::new(),
    }
}

fn reasoned(mut r: LotResultState, reason: &str) -> LotResultState {
    r.reason = Some(reason.into());
    r
}

fn round(notice: i64, results: Vec<LotResultState>) -> Round {
    Round { notice_id: notice, logical_notice_id: None, lot_results: results, bids: Vec::new(), contracts: Vec::new() }
}

const NO_AWARD: Option<&str> = Some(DECISION_NO_AWARD);
const WINNER: Option<&str> = Some("selec-w");

/// One version, its state already resolved the way the fold carries it: `facts` and `lots`
/// are the head's union, `rounds` every round published so far.
fn version(
    notice: i64,
    published_at: i64,
    subtype: Option<&str>,
    facts: &[Fact],
    lots: &[LotState],
    rounds: &[Round],
) -> TenderVersion {
    TenderVersion {
        caused_by_notice_id: notice,
        published_at,
        dispatched_at: None,
        notice_subtype: subtype.map(str::to_owned),
        original_lang: None,
        publication_id: format!("{notice}-2026"),
        facts: facts.iter().cloned().collect(),
        lots: lots.to_vec(),
        rounds: rounds.to_vec(),
        group_members: Vec::new(),
    }
}

/// One rule branch: a named chain and whether its full chain closes. A lot of the head is
/// closed exactly when its Tender is.
struct Case {
    name: &'static str,
    versions: Vec<TenderVersion>,
    closes: bool,
}

/// The notice ids of case `n` start at `n * 100`, so no two cases share a notice.
fn cases() -> Vec<Case> {
    let mut out = Vec::new();
    let mut add = |name, closes, build: &dyn Fn(i64) -> Vec<TenderVersion>| {
        let n = out.len() as i64 + 1;
        out.push(Case { name, versions: build(n * 100), closes });
    };
    let one = [lot("LOT-1", "Lot", &[])];
    let two = [lot("LOT-1", "Lot", &[]), lot("LOT-2", "Lot", &[])];
    let cn = |b: i64, lots: &[LotState], facts: &[Fact]| {
        let mut f = facts.to_vec();
        f.push(deadline(DEADLINE));
        version(b + 1, CN_AT, Some("16"), &f, lots, &[])
    };
    let can = |b: i64, subtype: &str, lots: &[LotState], facts: &[Fact], rounds: &[Round]| {
        let mut f = facts.to_vec();
        f.push(deadline(DEADLINE));
        version(b + 2, CAN_AT, Some(subtype), &f, lots, rounds)
    };

    // R1: 841082's shape (minus its DPS) — a CN, then a CAN whose every lot is `clos-nw`.
    add("r1-closes", true, &|b| {
        vec![
            cn(b, &two, &[]),
            can(b, "29", &two, &[], &[round(b + 2, vec![
                result("RES-1", Some("LOT-1"), NO_AWARD),
                result("RES-2", Some("LOT-2"), NO_AWARD),
            ])]),
        ]
    });
    // A re-launch: the CN after the failed round has no results of its own (the 88).
    add("relaunch-stays-open", false, &|b| {
        let failed = round(b + 2, vec![result("RES-1", Some("LOT-1"), NO_AWARD)]);
        vec![
            cn(b, &one, &[]),
            can(b, "29", &one, &[], std::slice::from_ref(&failed)),
            version(b + 3, NOW - DAY, Some("16"), &[deadline(DEADLINE + 30 * DAY)], &one, &[failed]),
        ]
    });
    // A winner in an earlier round keeps the Tender open, and the later round's `clos-nw` lot
    // with it (a lot closes only with its Tender).
    add("earlier-winner-stays-open", false, &|b| {
        let first = round(b + 2, vec![result("RES-1", Some("LOT-1"), WINNER)]);
        vec![
            cn(b, &two, &[]),
            can(b, "29", &two, &[], std::slice::from_ref(&first)),
            version(b + 3, NOW - DAY, Some("29"), &[deadline(DEADLINE)], &two, &[
                first,
                round(b + 3, vec![result("RES-2", Some("LOT-2"), NO_AWARD)]),
            ]),
        ]
    });
    // A lot with no result: not covered, so open, the decided lot with it.
    add("uncovered-lot-stays-open", false, &|b| {
        vec![cn(b, &two, &[]), can(b, "29", &two, &[], &[round(b + 2, vec![result("RES-1", Some("LOT-1"), NO_AWARD)])])]
    });
    // A NULL decision is no `clos-nw` (issue 257).
    add("null-decision-stays-open", false, &|b| {
        vec![cn(b, &one, &[]), can(b, "29", &one, &[], &[round(b + 2, vec![result("RES-1", Some("LOT-1"), None)])])]
    });
    // K1, 445631's shape: `dps-list` on the CN's lot, `none` on the CAN (which emits nothing),
    // so the head's lot still carries the declaration by key.
    add("k1-cn-dps-then-can-none-stays-open", false, &|b| {
        let dps = [lot("LOT-1", "Lot", &[class("dps", "dps-list")])];
        vec![cn(b, &dps, &[]), can(b, "29", &dps, &[], &[round(b + 2, vec![result("RES-1", Some("LOT-1"), NO_AWARD)])])]
    });
    // K1's legacy leg: r209's `DPS` marker as the Tender-scope code `dps`, on the CAN itself.
    add("k1-legacy-dps-on-the-can-stays-open", false, &|b| {
        vec![
            cn(b, &one, &[]),
            can(b, "29", &one, &[class("dps", "dps")], &[round(b + 2, vec![result("RES-1", Some("LOT-1"), NO_AWARD)])]),
        ]
    });
    // K2: 403644's shape, an `oth-single` authorisation system.
    add("k2-oth-single-stays-open", false, &|b| {
        let proc = [class("procedure", "oth-single")];
        vec![cn(b, &one, &proc), can(b, "29", &one, &proc, &[round(b + 2, vec![result("RES-1", Some("LOT-1"), NO_AWARD)])])]
    });
    // E2: a qualification-system notice (subtype 15) anywhere in the chain.
    add("e2-qualification-system-stays-open", false, &|b| {
        vec![
            version(b + 1, CN_AT, Some(" 15 "), &[deadline(DEADLINE)], &one, &[]),
            can(b, "29", &one, &[], &[round(b + 2, vec![result("RES-1", Some("LOT-1"), NO_AWARD)])]),
        ]
    });
    // A UK12 that cancels every lot closes through its own `cancelled` awards.
    add("uk12-whole-closes", true, &|b| {
        let lots = [lot("1", "Lot", &[])];
        vec![
            version(b + 1, CN_AT, Some("UK4"), &[deadline(DEADLINE)], &lots, &[]),
            version(b + 2, CAN_AT, Some("UK12"), &[deadline(DEADLINE)], &lots, &[round(b + 2, vec![
                result("RES-1-1", Some("1"), NO_AWARD),
            ])]),
        ]
    });
    // A UK12 that cancels some lots: open, its cancelled lot too (the per-lot leg is unit 3's).
    add("uk12-partial-stays-open", false, &|b| {
        let lots = [lot("1", "Lot", &[]), lot("2", "Lot", &[])];
        vec![
            version(b + 1, CN_AT, Some("UK4"), &[deadline(DEADLINE)], &lots, &[]),
            version(b + 2, CAN_AT, Some("UK12"), &[deadline(DEADLINE)], &lots, &[round(b + 2, vec![
                result("RES-1-1", Some("1"), NO_AWARD),
            ])]),
        ]
    });
    // A lotless Tender is covered by construction, so a Tender-scoped `clos-nw` closes it.
    add("lotless-closes", true, &|b| {
        vec![cn(b, &[], &[]), can(b, "29", &[], &[], &[round(b + 2, vec![result("RES-1", None, NO_AWARD)])])]
    });
    // A Tender-scoped result covers no lot: beside a Lot-kind lot it stays open.
    add("tender-scoped-result-beside-lots-stays-open", false, &|b| {
        vec![cn(b, &one, &[]), can(b, "29", &one, &[], &[round(b + 2, vec![result("RES-1", None, NO_AWARD)])])]
    });
    // A correction replaces the `clos-nw` round with a `selec-w` one: reopened.
    add("correction-to-a-winner-reopens", false, &|b| {
        vec![
            cn(b, &one, &[]),
            can(b, "29", &one, &[], &[round(b + 2, vec![result("RES-1", Some("LOT-1"), NO_AWARD)])]),
            version(b + 3, NOW - DAY, Some("29"), &[deadline(DEADLINE)], &one, &[round(b + 3, vec![
                result("RES-1", Some("LOT-1"), WINNER),
            ])]),
        ]
    });
    // W at 366 days from the first deadline-bearing version: an admission window, open.
    add("w-366-days-stays-open", false, &|b| {
        let d = [deadline(CN_AT + 366 * DAY)];
        vec![
            version(b + 1, CN_AT, Some("16"), &d, &one, &[]),
            version(b + 2, CAN_AT, Some("29"), &d, &one, &[round(b + 2, vec![result("RES-1", Some("LOT-1"), NO_AWARD)])]),
        ]
    });
    // …and at 364 days a one-shot, closed.
    add("w-364-days-closes", true, &|b| {
        let d = [deadline(CN_AT + 364 * DAY)];
        vec![
            version(b + 1, CN_AT, Some("16"), &d, &one, &[]),
            version(b + 2, CAN_AT, Some("29"), &d, &one, &[round(b + 2, vec![result("RES-1", Some("LOT-1"), NO_AWARD)])]),
        ]
    });
    // A deadline-less PIN (or UK2) a year earlier does not anchor W: the window runs from the
    // CN, 40 days, so it closes.
    add("deadline-less-pin-does-not-anchor-w", true, &|b| {
        vec![
            version(b, CN_AT - 400 * DAY, Some("UK2"), &[], &one, &[]),
            cn(b, &one, &[]),
            can(b, "29", &one, &[], &[round(b + 2, vec![result("RES-1", Some("LOT-1"), NO_AWARD)])]),
        ]
    });
    // D overrides W: 6739006's shape, an explicit legacy discontinuation of a long scheme.
    add("d-overrides-w-closes", true, &|b| {
        let d = [deadline(CN_AT + 900 * DAY)];
        vec![
            version(b + 1, CN_AT, None, &d, &one, &[]),
            version(b + 2, CAN_AT, None, &d, &one, &[round(b + 2, vec![reasoned(
                result("RES-1", Some("LOT-1"), NO_AWARD),
                REASON_DISCONTINUED,
            )])]),
        ]
    });
    // …but never K1: 7207004's shape, a legacy DPS.
    add("d-does-not-override-k1", false, &|b| {
        let d = [deadline(CN_AT + 900 * DAY), class("dps", "dps")];
        vec![
            version(b + 1, CN_AT, None, &d, &one, &[]),
            version(b + 2, CAN_AT, None, &d, &one, &[round(b + 2, vec![reasoned(
                result("RES-1", Some("LOT-1"), NO_AWARD),
                REASON_DISCONTINUED,
            )])]),
        ]
    });
    // D needs EVERY own result explained: one `discontinued` beside one with no legacy reason,
    // past a long window, stays open — W holds and D does not override it.
    add("d-needs-every-own-result-explained-stays-open", false, &|b| {
        let d = [deadline(CN_AT + 900 * DAY)];
        vec![
            version(b + 1, CN_AT, None, &d, &two, &[]),
            version(b + 2, CAN_AT, None, &d, &two, &[round(b + 2, vec![
                reasoned(result("RES-1", Some("LOT-1"), NO_AWARD), REASON_DISCONTINUED),
                result("RES-2", Some("LOT-2"), NO_AWARD),
            ])]),
        ]
    });
    // covered(H) asks the `Lot`-kind lots only: a lots group and a part with no result of their
    // own beside the one decided lot do not keep the Tender open.
    add("a-group-and-a-part-without-results-close", true, &|b| {
        let lots = [lot("LOT-1", "Lot", &[]), lot("GLO-1", "LotsGroup", &[]), lot("PAR-1", "Part", &[])];
        vec![cn(b, &lots, &[]), can(b, "29", &lots, &[], &[round(b + 2, vec![result("RES-1", Some("LOT-1"), NO_AWARD)])])]
    });
    // A later notice that publishes nothing new — no results of its own, and the closing CAN's
    // facts, lots and rounds — reopens the Tender (own(H) is empty), and the flip is the ONLY
    // thing that moved, so only the change feed's closure clause can announce it.
    add("a-flip-with-nothing-else-moving-reopens", false, &|b| {
        let closing = round(b + 2, vec![result("RES-1", Some("LOT-1"), NO_AWARD)]);
        vec![
            cn(b, &one, &[]),
            can(b, "29", &one, &[], std::slice::from_ref(&closing)),
            version(b + 3, NOW - DAY, Some("29"), &[deadline(DEADLINE)], &one, &[closing]),
        ]
    });
    // Coverage counts DISTINCT lots: two `clos-nw` results on one lot of two leave the other
    // uncovered, so open (a plain COUNT would read 2 = 2 and close it).
    add("two-results-on-one-lot-of-two-stay-open", false, &|b| {
        vec![
            cn(b, &two, &[]),
            can(b, "29", &two, &[], &[round(b + 2, vec![
                result("RES-1", Some("LOT-1"), NO_AWARD),
                result("RES-2", Some("LOT-1"), NO_AWARD),
            ])]),
        ]
    });
    // …and the other way: rounds accumulate, so a lot decided `clos-nw` twice is one covered
    // lot of one, and it closes.
    add("a-lot-decided-in-two-rounds-closes", true, &|b| {
        let first = round(b + 2, vec![result("RES-1", Some("LOT-1"), NO_AWARD)]);
        vec![
            cn(b, &one, &[]),
            can(b, "29", &one, &[], std::slice::from_ref(&first)),
            version(b + 3, NOW - DAY, Some("29"), &[deadline(DEADLINE)], &one, &[
                first,
                round(b + 3, vec![result("RES-2", Some("LOT-1"), NO_AWARD)]),
            ]),
        ]
    });
    // Coverage counts the Lot-kind lots the results name: a result naming only the lots group
    // covers no lot, so open…
    add("a-result-naming-only-a-group-stays-open", false, &|b| {
        let lots = [lot("LOT-1", "Lot", &[]), lot("GLO-1", "LotsGroup", &[])];
        vec![cn(b, &lots, &[]), can(b, "29", &lots, &[], &[round(b + 2, vec![result("RES-1", Some("GLO-1"), NO_AWARD)])])]
    });
    // …and results naming the group and the lot cover the one Lot-kind lot, so it closes.
    add("results-naming-the-group-and-the-lot-close", true, &|b| {
        let lots = [lot("LOT-1", "Lot", &[]), lot("GLO-1", "LotsGroup", &[])];
        vec![
            cn(b, &lots, &[]),
            can(b, "29", &lots, &[], &[round(b + 2, vec![
                result("RES-1", Some("GLO-1"), NO_AWARD),
                result("RES-2", Some("LOT-1"), NO_AWARD),
            ])]),
        ]
    });
    out
}

fn projection(case: &Case) -> TenderProjection {
    TenderProjection {
        source: "ted".into(),
        procedure_key: Some(format!("pk-533-{}", case.name)),
        island_notice_id: None,
        kind: "procedure".into(),
        versions: case.versions.clone(),
    }
}

async fn scalar(conn: &turso::Connection, sql: &str, params: Vec<Value>) -> Option<i64> {
    let mut rows = conn.query(sql, params).await.unwrap_or_else(|e| panic!("{e}: {sql}"));
    let row = rows.next().await.unwrap()?;
    row.get_value(0).unwrap().as_integer().copied()
}

async fn tender_id(conn: &turso::Connection, case: &Case) -> i64 {
    scalar(conn, "SELECT id FROM tenders WHERE procedure_key = ?", vec![Value::Text(format!("pk-533-{}", case.name))])
        .await
        .expect("the case's tender")
}

/// Every case folded through `apply_tenders`, with the deferred tender indexes built (the
/// state prod reads in), and `name -> tender id`.
async fn folded(name: &str) -> (store::Db, turso::Connection, Vec<Case>, BTreeMap<&'static str, i64>) {
    let (db, conn) = open(name).await;
    let cases = cases();
    let projections: Vec<TenderProjection> = cases.iter().map(projection).collect();
    db.apply_tenders(&projections, NOW, false).await.unwrap();
    db.build_tender_indexes().await.unwrap();
    let mut ids = BTreeMap::new();
    for case in &cases {
        ids.insert(case.name, tender_id(&conn, case).await);
    }
    (db, conn, cases, ids)
}

/// The rule branches, through the Rust form over each chain.
#[test]
fn every_rule_branch_reads_as_decided() {
    let cases = cases();
    for case in &cases {
        assert_eq!(closed_by_notice(&case.versions), case.closes, "{}: the Tender's closure", case.name);
    }
    // The three chains that reopen were closed at v2, so each is a real flip, not a chain that
    // never closed.
    for name in ["relaunch-stays-open", "correction-to-a-winner-reopens", "a-flip-with-nothing-else-moving-reopens"] {
        let case = cases.iter().find(|c| c.name == name).unwrap();
        assert!(closed_by_notice(&case.versions[..2]), "{name}: v2 closes");
    }
    // The window's edges are the constant's: 366 days is past it, 364 within it.
    assert!(366 * DAY > OPEN_WINDOW_SECS && 364 * DAY < OPEN_WINDOW_SECS);
}

/// The twins agree (the issue-474 pattern): per version the SQL form over the stored rows
/// equals the Rust form over the projected prefix, the exemption's too; per Tender the stored
/// flag equals the SQL form at `current_seq`.
#[tokio::test]
async fn closed_by_notice_agrees_with_its_sql_form() {
    let (_db, conn, cases, ids) = folded("agree").await;
    let tender_sql = format!("SELECT {}", closed_by_notice_sql("?1", "?2"));
    let exempt_sql = format!("SELECT {}", closure_exempt_sql("?1", "?2"));
    let mut fired = 0;
    for case in &cases {
        let id = ids[case.name];
        for n in 1..=case.versions.len() {
            let prefix = &case.versions[..n];
            let params = vec![Value::Integer(id), Value::Integer(n as i64)];
            let sql = scalar(&conn, &tender_sql, params.clone()).await;
            assert_eq!(sql, Some(closed_by_notice(prefix) as i64), "{} v{n}: the SQL form disagrees", case.name);
            fired += usize::from(closed_by_notice(prefix));
            let exempt = scalar(&conn, &exempt_sql, params).await;
            assert_eq!(exempt, Some(closure_exempt(prefix) as i64), "{} v{n}: the exemption's SQL form disagrees", case.name);
        }
        let stored = scalar(&conn, "SELECT current_closed_by_notice FROM tenders WHERE id = ?", vec![Value::Integer(id)]).await;
        assert_eq!(stored, case.closes.then_some(1), "{}: the stored flag is 1 or NULL, never 0", case.name);
        let at_head = scalar(
            &conn,
            &format!("SELECT {} FROM tenders t WHERE t.id = ?1", closed_by_notice_sql("t.id", "t.current_seq")),
            vec![Value::Integer(id)],
        )
        .await;
        assert_eq!(at_head, Some(case.closes as i64), "{}: the stored flag equals the SQL form at the head", case.name);
    }
    // The comparison is not vacuous: both answers occur, including at a non-head version
    // (the re-launch's, the correction's and the flip's v2).
    assert!(fired >= 10, "only {fired} closing versions across the cases");
}

/// `status` on every Tenders shape: the id-ordered page, the deadline-ordered list (both
/// directions), the published-ordered list, and the source pin. A closed Tender keeps its
/// served deadline (and `sort=deadline` without a status still lists it); only `status`
/// moves.
#[tokio::test]
async fn status_reads_the_flag_beside_the_deadline_range() {
    let (_db, conn, cases, ids) = folded("status").await;
    let open: BTreeSet<i64> = cases.iter().filter(|c| !c.closes).map(|c| ids[c.name]).collect();
    let closed: BTreeSet<i64> = cases.iter().filter(|c| c.closes).map(|c| ids[c.name]).collect();
    assert!(!open.is_empty() && !closed.is_empty());
    let filter = |status| Filter { status: Some(status), now: NOW, ..Filter::default() };

    for (status, want) in [(Status::Open, &open), (Status::Closed, &closed)] {
        let page: BTreeSet<i64> = read::tenders(&conn, &filter(status), Scope::Page { after: 0, limit: 1000 })
            .await
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(&page, want, "{status:?}: the id-ordered page");
        for order in [HeadOrder::Deadline, HeadOrder::PublishedAt] {
            for desc in [false, true] {
                for f in [filter(status), Filter { source: Some("ted".into()), ..filter(status) }] {
                    let rows = read::tenders_ordered(&conn, &f, order, desc, None, 1000).await.unwrap();
                    let got: BTreeSet<i64> = rows.iter().map(|r| r.id).collect();
                    assert_eq!(&got, want, "{status:?} {order:?} desc={desc} source={:?}", f.source);
                }
            }
        }
    }
    // The closed Tenders still serve their (future) deadline, and sort=deadline still lists
    // them: the flag closes `status`, never the date.
    let all = read::tenders_ordered(&conn, &Filter { now: NOW, ..Filter::default() }, HeadOrder::Deadline, false, None, 1000)
        .await
        .unwrap();
    for row in all.iter().filter(|r| closed.contains(&r.id)) {
        assert!(row.deadline.as_ref().is_some_and(|d| d.utc_seconds > NOW), "tender {}: keeps its deadline", row.id);
    }
    assert_eq!(all.len(), cases.len(), "every case has a deadline, so sort=deadline lists each");
}

/// `status` on Lots: the stream, the tender-scoped containment read and the bare open-head
/// seed all drop a lot whose Tender is closed and keep the rest — a lot its head notice alone
/// decided `clos-nw` included (a partial UK12's: the per-lot leg is unit 3's); `closed` is the
/// complement.
#[tokio::test]
async fn lots_status_drops_the_lots_of_closed_tenders_and_keeps_the_rest() {
    let (_db, conn, cases, ids) = folded("lots").await;
    let mut open_lots: BTreeSet<(i64, String)> = BTreeSet::new();
    let mut closed_lots: BTreeSet<(i64, String)> = BTreeSet::new();
    for case in &cases {
        for l in &case.versions.last().unwrap().lots {
            let key = (ids[case.name], l.key.clone());
            if case.closes {
                closed_lots.insert(key);
            } else {
                open_lots.insert(key);
            }
        }
    }
    let base = Filter { now: NOW, ..Filter::default() };
    for (status, want) in [(Status::Open, &open_lots), (Status::Closed, &closed_lots)] {
        let f = Filter { status: Some(status), ..base.clone() };
        let stream: BTreeSet<(i64, String)> = read::lots(&conn, &f, Scope::Page { after: 0, limit: 1000 })
            .await
            .unwrap()
            .into_iter()
            .map(|r| (r.tender_id, r.lot_key))
            .collect();
        assert_eq!(&stream, want, "{status:?}: the lots stream");
        let mut contained = BTreeSet::new();
        for id in ids.values() {
            let scoped = Filter { tender: Some(*id), ..f.clone() };
            for r in read::lots(&conn, &scoped, Scope::Page { after: 0, limit: 1000 }).await.unwrap() {
                contained.insert((r.tender_id, r.lot_key));
            }
        }
        assert_eq!(&contained, want, "{status:?}: the tender-scoped containment read");
    }
}

/// The SSE `Scope::At` probe decides status per VERSION (`StatusBy::Version`): the version
/// that closed a Tender reads closed while the one before it reads open, so the diff is
/// `removed`; a re-launch reads open again after a closed version, so it is `added`. With the
/// head columns both probes read the head, and a fold-time close was `(false, false)`.
#[tokio::test]
async fn the_sse_probe_decides_status_at_each_version() {
    let (_db, conn, cases, ids) = folded("sse").await;
    let open = Filter { status: Some(Status::Open), now: NOW, ..Filter::default() };
    for case in &cases {
        let id = ids[case.name];
        for n in 1..=case.versions.len() {
            let prefix = &case.versions[..n];
            // Every deadline here is Tender-scoped, so a version is open on its date exactly
            // when the fold would elect one after NOW (a deadline-less PIN is not).
            let dated = head_deadline(prefix.last().unwrap()).is_some_and(|d| d > NOW);
            let rows = read::tenders(&conn, &open, Scope::At { id, seq: n as i64 }).await.unwrap();
            assert_eq!(
                !rows.is_empty(),
                dated && !closed_by_notice(prefix),
                "{} v{n}: the At probe reads the version's own deadline and closure",
                case.name
            );
            // Lots at each version: open exactly when dated and their Tender not closed at that
            // version.
            for l in &prefix.last().unwrap().lots {
                let lot_id = scalar(
                    &conn,
                    "SELECT id FROM lots WHERE tender_id = ? AND lot_key = ?",
                    vec![Value::Integer(id), Value::Text(l.key.clone())],
                )
                .await
                .unwrap();
                let at = read::lots(&conn, &open, Scope::At { id: lot_id, seq: n as i64 }).await.unwrap();
                assert_eq!(!at.is_empty(), dated && !closed_by_notice(prefix), "{} v{n} {}: the lot At probe", case.name, l.key);
            }
        }
    }
    // The two transitions the feed must carry, by name.
    let relaunch = ids["relaunch-stays-open"];
    let at = |seq| read::tenders(&conn, &open, Scope::At { id: relaunch, seq });
    assert!(!at(1).await.unwrap().is_empty() && at(2).await.unwrap().is_empty(), "the CAN is a `removed`");
    assert!(!at(3).await.unwrap().is_empty(), "the re-launch CN is an `added`");
}

/// The change feed: the version that closes a Tender, or reopens it, is a `tender changed` row
/// at its seq, and each of its lots is a `lot changed` row there — even where nothing else
/// moved (the flip case), and where the lot's facts did not, as a CAN's seldom do.
#[tokio::test]
async fn a_closure_flip_is_a_change_row_for_the_tender_and_its_lots() {
    let (_db, conn, _cases, ids) = folded("changes").await;
    let rows = |kind: &'static str, entity: i64, seq: i64| {
        let conn = conn.clone();
        async move {
            scalar(
                &conn,
                "SELECT COUNT(*) FROM changes WHERE entity_kind = ? AND entity_id = ? AND version_seq = ? AND op = 'changed'",
                vec![Value::Text(kind.into()), Value::Integer(entity), Value::Integer(seq)],
            )
            .await
            .unwrap()
        }
    };
    let lot_id = |tender: i64, key: &'static str| {
        let conn = conn.clone();
        async move {
            scalar(&conn, "SELECT id FROM lots WHERE tender_id = ? AND lot_key = ?", vec![Value::Integer(tender), Value::Text(key.into())])
                .await
                .unwrap()
        }
    };
    let r1 = ids["r1-closes"];
    assert_eq!(rows("tender", r1, 2).await, 1, "the closing CAN");
    for key in ["LOT-1", "LOT-2"] {
        assert_eq!(rows("lot", lot_id(r1, key).await, 2).await, 1, "{key}: closed with its Tender, facts unmoved");
    }
    let relaunch = ids["relaunch-stays-open"];
    assert_eq!(rows("tender", relaunch, 3).await, 1, "the re-launch reopens");
    assert_eq!(rows("lot", lot_id(relaunch, "LOT-1").await, 3).await, 1, "and its lot with it");
    // No per-lot leg (unit 3): the partial UK12's cancelled lot does not move on its own.
    let partial = ids["uk12-partial-stays-open"];
    for key in ["1", "2"] {
        assert_eq!(rows("lot", lot_id(partial, key).await, 2).await, 0, "{key}: its Tender stayed open");
    }
    // The flip alone: v3 repeats v2's facts, lots and rounds, so only the closure clause can
    // announce it — for the Tender and for its lot.
    let flip = ids["a-flip-with-nothing-else-moving-reopens"];
    assert_eq!(rows("tender", flip, 2).await, 1, "the closing CAN");
    assert_eq!(rows("tender", flip, 3).await, 1, "the reopening notice, nothing else moving");
    assert_eq!(rows("lot", lot_id(flip, "LOT-1").await, 3).await, 1, "and its lot with it");
}

async fn plan_lines(conn: &turso::Connection, sql: &str, params: Vec<Value>) -> Vec<String> {
    let mut rows = conn.query(&format!("EXPLAIN QUERY PLAN {sql}"), params).await.expect("explain");
    let mut plan = Vec::new();
    while let Some(row) = rows.next().await.expect("plan row") {
        if let Ok(Value::Text(detail)) = row.get_value(3) {
            plan.push(detail.trim().to_owned());
        }
    }
    assert!(!plan.is_empty(), "no plan came back for: {sql}");
    plan
}

/// The lines of a plan that SCAN a table, the shapes a regression would add.
fn scans(plan: &[String]) -> BTreeSet<String> {
    plan.iter().filter(|l| l.starts_with("SCAN ")).cloned().collect()
}

/// Issue 533 plan pin, with the deferred tender indexes built (prod's state): every changed
/// `status` read still seeks `tenders_current_deadline` (or `tenders_source_deadline` under the
/// issue-503 source pin) and adds no SCAN. The flag is a residual on the `tenders` row each
/// window already reads, so a Tenders plan is IDENTICAL to the pre-533 statement's (the same
/// statement with the flag clause cut out); a lots page gains at most the flag's primary-key
/// seek, no SCAN, and never the closing rule itself. 273's DoS was a range turned walk; this is
/// what would show it returning.
///
/// And where the rule's SQL does run — the SSE `Scope::At` probes, Tenders and Lots — every leg
/// seeks by the version: a `(tender_id, …)` prefix or a rowid, never a `MULTI-INDEX OR` or a
/// `(scheme=?)` seek of `tender_version_classifications_code`. The stats-free planner chose
/// exactly that for the K1/K2 leg's OR (a walk of every `procedure` and `dps` row in the corpus
/// per probe; review 2026-10-16) until the leg was pinned to the version index. A SCAN-only
/// comparison could not see it: a MULTI-INDEX OR is no SCAN line. The legs keyed by ONE version's
/// rows (its classifications, dates, results and lots) must seek `(tender_id=? AND seq=…)`: a
/// `(tender_id=?)` prefix alone reads every version's slice of the Tender, which is what the
/// coverage count's results did until its lot join was keyed on the version row (review
/// 2026-10-16). `cb_e`/`cb_a` walk the chain prefix, a prefix by design; the version rows
/// `cb_v`/`cb_g` are sought on `(tender_id=?)` like the statements' own `v`, a walk of the
/// Tender's versions, not of their rows.
#[tokio::test]
async fn the_status_reads_keep_their_deadline_index_and_add_no_scan() {
    const N: i64 = 3_000;
    let (db, conn) = open("plans").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    for chunk in (1..=N).collect::<Vec<_>>().chunks(500) {
        let tenders: Vec<String> = chunk
            .iter()
            .map(|&i| {
                let source = if i % 10 == 0 { "doe" } else { "ted" };
                let deadline = if i % 7 == 0 { "NULL".to_owned() } else { (NOW - 50 * DAY + i * 3_600).to_string() };
                let flag = if i % 11 == 0 { "1" } else { "NULL" };
                format!("({i}, '{source}', 'p{i}', 'procedure', 1, {}, {deadline}, {flag}, 0)", CN_AT + i)
            })
            .collect();
        conn.execute(
            &format!(
                "INSERT INTO tenders (id, source, procedure_key, kind, current_seq, current_published_at, \
                 current_deadline, current_closed_by_notice, created_at) VALUES {}",
                tenders.join(", ")
            ),
            (),
        )
        .await
        .unwrap();
        let versions: Vec<String> =
            chunk.iter().map(|&i| format!("({i}, 1, {}, 'OJ-{i}', {i})", CN_AT + i)).collect();
        conn.execute(
            &format!(
                "INSERT INTO tender_versions (tender_id, seq, published_at, publication_id, caused_by_notice_id) VALUES {}",
                versions.join(", ")
            ),
            (),
        )
        .await
        .unwrap();
    }
    db.build_tender_indexes().await.unwrap();
    const FLAG_OPEN: &str = " AND t.current_closed_by_notice IS NOT 1";
    const FLAG_CLOSED: &str = " OR t.current_closed_by_notice IS 1";
    let open = Filter { status: Some(Status::Open), now: NOW, ..Filter::default() };
    let closed = Filter { status: Some(Status::Closed), ..open.clone() };
    let ted = |f: &Filter| Filter { source: Some("ted".into()), ..f.clone() };

    // The id-ordered page: no index is demanded of it (on this unanalysed file the planner
    // walks the PK range for both shapes), only that the flag leaves its plan as it was.
    let mut statements: Vec<(String, (String, Vec<Value>), &'static str)> = Vec::new();
    for after in [0, 1_500] {
        statements.push((format!("page open after={after}"), read::tenders_page_statement(&open, after, 100), ""));
        statements.push((format!("page closed after={after}"), read::tenders_page_statement(&closed, after, 100), ""));
    }
    for desc in [false, true] {
        for cursor in [None, Some((NOW + 10 * DAY, 1_234))] {
            let what = format!("desc={desc} cursor={cursor:?}");
            statements.push((
                format!("sort=deadline open {what}"),
                read::tenders_ordered_statement(&open, HeadOrder::Deadline, desc, cursor, 100),
                "tenders_current_deadline",
            ));
            for order in [HeadOrder::Deadline, HeadOrder::PublishedAt] {
                statements.push((
                    format!("source-pinned {order:?} open {what}"),
                    read::tenders_ordered_statement_pinned(&ted(&open), order, desc, cursor, 100),
                    "tenders_source_deadline",
                ));
            }
            statements.push((
                format!("source-pinned sort=deadline closed {what}"),
                read::tenders_ordered_statement_pinned(&ted(&closed), HeadOrder::Deadline, desc, cursor, 100),
                "tenders_source_deadline",
            ));
        }
    }
    for (label, (sql, params), index) in &statements {
        assert!(sql.contains(FLAG_OPEN) || sql.contains(FLAG_CLOSED), "{label}: the flag must be in the statement: {sql}");
        let plan = plan_lines(&conn, sql, params.clone()).await;
        let before = sql.replace(FLAG_OPEN, "").replace(FLAG_CLOSED, "");
        let old = plan_lines(&conn, &before, params.clone()).await;
        assert_eq!(plan, old, "{label}: the flag must not move the plan (pre-533 shape on the right)");
        if !index.is_empty() {
            assert!(
                plan.iter().any(|l| l.contains(&format!("USING INDEX {index}")) || l.contains(&format!("USING COVERING INDEX {index}"))),
                "{label}: the window must seek {index}:\n{}",
                plan.join("\n")
            );
        }
    }

    // Lots: the stream, the bare open-head seed and the containment read gain the Tender's
    // flag (a residual on the stream's `tenders` PK seek), no SCAN, and no leg of the rule.
    let stream_cut = " AND NOT ((SELECT tt.current_closed_by_notice FROM tenders tt WHERE tt.id = l.tender_id) IS 1)".to_owned();
    let scoped_cut = " AND NOT (t.current_closed_by_notice IS 1)".to_owned();
    for (label, f, cut) in [
        ("lots stream open", open.clone(), &stream_cut),
        ("lots stream closed", closed.clone(), &stream_cut),
        ("lots stream open+cpv", Filter { cpv: Some("45".into()), ..open.clone() }, &stream_cut),
        ("lots tender-scoped open", Filter { tender: Some(42), ..open.clone() }, &scoped_cut),
    ] {
        let (sql, params) = read::lots_statement(&f, Scope::Page { after: 0, limit: 100 });
        assert!(sql.contains(cut.as_str()), "{label}: the Tender's flag must be in the statement: {sql}");
        assert!(!sql.contains("cb_"), "{label}: the closing rule never runs in a Page read: {sql}");
        let plan = plan_lines(&conn, &sql, params.clone()).await;
        let old = plan_lines(&conn, &sql.replace(cut.as_str(), ""), params).await;
        assert_eq!(scans(&plan), scans(&old), "{label}: no new SCAN:\n{}\n--- before:\n{}", plan.join("\n"), old.join("\n"));
        if label == "lots stream open" {
            assert!(
                plan.iter().any(|l| l.contains("tenders_current_deadline")),
                "{label}: the open-head seed still reads the deadline index:\n{}",
                plan.join("\n")
            );
        }
    }

    // The closing rule where it runs: alone (the agreement test's form) and inside the SSE
    // At probes, where turso flattens its subqueries into the statement's join.
    let at = vec![Value::Integer(1), Value::Integer(1)];
    let mut rule: Vec<(String, (String, Vec<Value>))> = vec![
        ("closed_by_notice_sql".into(), (format!("SELECT {}", closed_by_notice_sql("?1", "?2")), at.clone())),
        ("closure_exempt_sql".into(), (format!("SELECT {}", closure_exempt_sql("?1", "?2")), at)),
    ];
    for (label, f) in [("open", &open), ("closed", &closed)] {
        rule.push((format!("tenders At {label}"), read::tenders_at_statement(f, 1, 1)));
        rule.push((format!("lots At {label}"), read::lots_statement(f, Scope::At { id: 1, seq: 1 })));
    }
    for (label, (sql, params)) in &rule {
        let plan = plan_lines(&conn, sql, params.clone()).await;
        let legs: Vec<&String> = plan.iter().filter(|l| l.contains(" cb_")).collect();
        assert!(
            legs.iter().any(|l| l.contains(" cb_c ")),
            "{label}: the K1/K2 classifications leg must be in the plan:\n{}",
            plan.join("\n")
        );
        let off: Vec<&&String> = legs
            .iter()
            .filter(|l| {
                l.contains("MULTI-INDEX OR") || l.contains("(scheme=") || !(l.contains("(tender_id=") || l.contains("(rowid="))
            })
            .collect();
        assert!(off.is_empty(), "{label}: a leg of the closing rule does not seek by the version: {off:?}\n{}", plan.join("\n"));
        const VERSION_KEYED: [&str; 5] = ["cb_c", "cb_d", "cb_f", "cb_r", "cb_l"];
        let loose: Vec<&&String> = legs
            .iter()
            .filter(|l| {
                let alias = l.split_whitespace().nth(1).unwrap_or_default();
                VERSION_KEYED.contains(&alias) && !l.contains("(tender_id=? AND seq=")
            })
            .collect();
        assert!(
            loose.is_empty(),
            "{label}: a version-keyed leg seeks the Tender's rows, not the version's: {loose:?}\n{}",
            plan.join("\n")
        );
    }
}

/// Issue 533's perf bound (`1(c)`): on one Tender with 2,400 lots and 2,400 head results,
/// `lots?tender=X&status=open` must stay under 2x its pre-533 time — and so must the lots stream
/// page (`lots?status=open&limit=100`), which the bound did not name and the decided per-lot leg
/// broke worst. Measured 2026-10-16 with that leg, on lots that carry no deadline of their own
/// (so the baseline is not the 424 EXISTS's own quadratic walk, which had hidden it at 1.02x):
/// 2.71x on the tender-scoped page at 2,400 lots, and 88x on the stream at 300 (the leg
/// correlated per lot, its result-slice set re-read for every lot). Past the bound, so the leg
/// shipped out under the decision's fallback (unit 3). What is left is the Tender's flag, a
/// residual on the `tenders` row each page reads (the stream: one primary-key seek per lot),
/// timed against the same statement with the flag cut out, best of three each, as a ratio —
/// robust to machine speed in the way issue 115's `lot_summary_cost` is.
///
/// The SSE At probes, timed the same way against the same statement with the closed leg cut
/// out, under the same 2x: one lots At probe at the head (where it reads the stored flag) and
/// at the contract notice before it, and one Tenders At probe at the head; and under 3x, a lots
/// At probe at a non-head award version whose last result names a winner (the rule's own pass
/// over the results, see the bound's comment). Each probe ran the rule's coverage counts
/// eagerly before (review 2026-10-16, 2,400 lots: 12.4x at a closing head, 3.2x at the CN,
/// 10-17x at winner heads, 23x for the Tenders probe), and a flip of a 2,400-lot Tender is
/// 4,800 lots probes per subscription. What still pays the rule in full is an earlier version
/// that passes both of its cheap legs (one that closed, or an exempt all-`clos-nw` one): linear
/// in its lots, the scaling bound below.
///
/// And the rule's SQL where it does run, the SSE At probe, must grow neither with the corpus
/// nor with the square of a Tender's lots. One probe is timed before and after 100k
/// `procedure`/`oth-single` and `dps`/`dps-list` rows of OTHER Tenders land — rows the
/// planner's `(scheme=?)` MULTI-INDEX OR read on every probe (6 ms → 1.85 s in the review's
/// measurement) and the pinned version index never touches. And the probe of the 2,400-lot
/// Tender is timed against the same shape at an eighth of the lots: under 16x, where linear is
/// about 8x and the first coverage leg (NOT EXISTS a Lot-kind lot without a result, the result
/// slice walked per lot) was 64x — 120 of its probes had not finished in 15 minutes.
///
/// Run: the focused command with `-- --ignored the_status_terms_stay_within_twice`
/// (`TDB_533_LOTS=600` for a quick read; the bound is stated at 2,400).
#[tokio::test]
#[ignore]
async fn the_status_terms_stay_within_twice_the_pre_533_reads() {
    let lots_n: usize = std::env::var("TDB_533_LOTS").ok().and_then(|n| n.parse().ok()).unwrap_or(2_400);
    let (db, conn) = open("status-term-cost").await;
    conn.execute("PRAGMA foreign_keys = OFF", ()).await.unwrap();
    // 403644's shape at scale: every lot decided `clos-nw` by the head notice, the Tender an
    // `oth-single` scheme, so it stays open and every lot is served.
    let lots: Vec<LotState> = (0..lots_n).map(|i| lot(&format!("LOT-{i:04}"), "Lot", &[])).collect();
    let proc = [class("procedure", "oth-single"), deadline(DEADLINE)];
    let results: Vec<LotResultState> =
        (0..lots_n).map(|i| result(&format!("RES-{i:04}"), Some(&format!("LOT-{i:04}")), NO_AWARD)).collect();
    let chain = |n: usize, notice: i64| {
        vec![
            version(notice, CN_AT, Some("16"), &proc, &lots[..n], &[]),
            version(notice + 1, CAN_AT, Some("29"), &proc, &lots[..n], &[round(notice + 1, results[..n].to_vec())]),
        ]
    };
    let small_n = lots_n / 8;
    let tender = |key: &str, versions: Vec<TenderVersion>| TenderProjection {
        source: "ted".into(),
        procedure_key: Some(key.into()),
        island_notice_id: None,
        kind: "procedure".into(),
        versions,
    };
    let mut projections: Vec<TenderProjection> = [("pk-533-cost", lots_n, 1), ("pk-533-cost-small", small_n, 11)]
        .into_iter()
        .map(|(key, n, notice)| {
            let versions = chain(n, notice);
            assert!(!closed_by_notice(&versions), "exempt (K2): open");
            tender(key, versions)
        })
        .collect();
    // A one-shot (not exempt) whose award version's LAST result names a winner, then a later
    // notice with no results of its own: v2 is the probed earlier version, open on its winner.
    let mut awards = results.clone();
    awards.last_mut().unwrap().decision = WINNER.map(str::to_owned);
    let one_shot = [deadline(DEADLINE)];
    let awarded = round(22, awards);
    let winner_chain = vec![
        version(21, CN_AT, Some("16"), &one_shot, &lots, &[]),
        version(22, CAN_AT, Some("29"), &one_shot, &lots, std::slice::from_ref(&awarded)),
        version(23, NOW - DAY, Some("29"), &one_shot, &lots, &[awarded.clone()]),
    ];
    assert!(!closed_by_notice(&winner_chain[..2]), "a winner: open");
    projections.push(tender("pk-533-cost-winner", winner_chain));
    db.apply_tenders(&projections, NOW, false).await.unwrap();
    db.build_tender_indexes().await.unwrap();
    let id = scalar(&conn, "SELECT id FROM tenders WHERE procedure_key = 'pk-533-cost'", vec![]).await.unwrap();
    let small = scalar(&conn, "SELECT id FROM tenders WHERE procedure_key = 'pk-533-cost-small'", vec![]).await.unwrap();
    let winner = scalar(&conn, "SELECT id FROM tenders WHERE procedure_key = 'pk-533-cost-winner'", vec![]).await.unwrap();

    // Best of three samples, each `repeat` runs of the statement; and the rows one run returned.
    let time = |sql: String, params: Vec<Value>, repeat: usize| {
        let conn = conn.clone();
        async move {
            let mut best = std::time::Duration::MAX;
            let mut n = 0;
            for _ in 0..3 {
                let started = std::time::Instant::now();
                for _ in 0..repeat {
                    let mut rows = conn.query(&sql, params.clone()).await.unwrap();
                    n = 0;
                    while rows.next().await.unwrap().is_some() {
                        n += 1;
                    }
                }
                best = best.min(started.elapsed());
            }
            (best, n)
        }
    };
    let mut ratios = Vec::new();
    for (label, filter, limit, cut) in [
        (
            "tender-scoped page",
            Filter { tender: Some(id), status: Some(Status::Open), now: NOW, ..Filter::default() },
            lots_n,
            " AND NOT (t.current_closed_by_notice IS 1)",
        ),
        (
            "stream page",
            Filter { status: Some(Status::Open), now: NOW, ..Filter::default() },
            100,
            " AND NOT ((SELECT tt.current_closed_by_notice FROM tenders tt WHERE tt.id = l.tender_id) IS 1)",
        ),
    ] {
        let (sql, params) = read::lots_statement(&filter, Scope::Page { after: 0, limit: limit as i64 });
        assert!(sql.contains(cut), "{label}: carries the Tender's flag: {sql}");
        let (old, n_old) = time(sql.replace(cut, ""), params.clone(), 1).await;
        let (new, n_new) = time(sql, params, 1).await;
        assert_eq!((n_old, n_new), (limit, limit), "{label}: every lot is open either way");
        let ratio = new.as_secs_f64() / old.as_secs_f64();
        println!("issue 533 {label} ({lots_n} lots): pre-533 {old:?}, with the flag {new:?}, ratio {ratio:.2}");
        ratios.push((label, ratio, 2.0));
    }

    // The At probes, each against itself with the closed leg cut out (the pre-533 statement).
    let open = Filter { status: Some(Status::Open), now: NOW, ..Filter::default() };
    let closed_leg = format!(
        " AND NOT ({})",
        closed_at_version_sql("t.id", "v.seq", "t.current_seq", "t.current_closed_by_notice")
    );
    let tenders_leg = format!(
        " AND NOT {}",
        closed_at_version_sql("t.id", "v.seq", "t.current_seq", "t.current_closed_by_notice")
    );
    let first_lot = |tender: i64| {
        let conn = conn.clone();
        async move {
            scalar(&conn, "SELECT MIN(id) FROM lots WHERE tender_id = ?", vec![Value::Integer(tender)]).await.unwrap()
        }
    };
    let (cost_lot, winner_lot) = (first_lot(id).await, first_lot(winner).await);
    // The earlier award version is bounded at 3x, not 2x: it passes the rule's first cheap leg
    // (an own result) and its second reads every result to find the winner, last in key order
    // here — one pass over the version's results, which the pre-533 probe never read (2.34x
    // measured 2026-10-16; 12.2x while the coverage counts ran eagerly).
    for (label, (sql, params), cut, bound) in [
        ("lots At probe, head", read::lots_statement(&open, Scope::At { id: cost_lot, seq: 2 }), &closed_leg, 2.0),
        ("lots At probe, the CN before it", read::lots_statement(&open, Scope::At { id: cost_lot, seq: 1 }), &closed_leg, 2.0),
        (
            "lots At probe, an earlier award version whose last result names a winner",
            read::lots_statement(&open, Scope::At { id: winner_lot, seq: 2 }),
            &closed_leg,
            3.0,
        ),
        ("tenders At probe, head", read::tenders_at_statement(&open, id, 2), &tenders_leg, 2.0),
    ] {
        assert!(sql.contains(cut.as_str()), "{label}: carries the closed leg: {sql}");
        let (old, n_old) = time(sql.replace(cut.as_str(), ""), params.clone(), 10).await;
        let (new, n_new) = time(sql, params, 10).await;
        assert_eq!((n_old, n_new), (1, 1), "{label}: open either way");
        let ratio = new.as_secs_f64() / old.as_secs_f64();
        println!("issue 533 {label} x10 ({lots_n} lots): pre-533 {old:?}, with the closed leg {new:?}, ratio {ratio:.2}");
        ratios.push((label, ratio, bound));
    }

    let probe = format!("SELECT {}", closed_by_notice_sql("?1", "?2"));
    let at_head = vec![Value::Integer(id), Value::Integer(2)];
    let (quiet, _) = time(probe.clone(), at_head.clone(), 20).await;
    let (eighth, _) = time(probe.clone(), vec![Value::Integer(small), Value::Integer(2)], 20).await;
    let scale = quiet.as_secs_f64() / eighth.as_secs_f64();
    println!("issue 533 At probe x20: {lots_n} lots {quiet:?}, {small_n} lots {eighth:?}, ratio {scale:.2} (linear ~8, quadratic ~64)");
    assert!(scale < 16.0, "the At probe grows with the square of the Tender's lots: {scale:.2}x for 8x the lots");
    for chunk in (0..100_000_i64).collect::<Vec<_>>().chunks(1_000) {
        let rows: Vec<String> = chunk
            .iter()
            .map(|&i| {
                let (scheme, code) = if i % 2 == 0 { ("procedure", "oth-single") } else { ("dps", "dps-list") };
                format!("({}, 1, NULL, '{scheme}', '{scheme}', '{code}')", 10_000_000 + i)
            })
            .collect();
        conn.execute(
            &format!(
                "INSERT INTO tender_version_classifications (tender_id, seq, lot_id, field, scheme, code) VALUES {}",
                rows.join(", ")
            ),
            (),
        )
        .await
        .unwrap();
    }
    let (crowded, _) = time(probe, at_head, 20).await;
    let ratio = crowded.as_secs_f64() / quiet.as_secs_f64();
    println!("issue 533 At probe x20: {quiet:?} alone, {crowded:?} beside 100k other classification rows, ratio {ratio:.2}");
    ratios.push(("At probe beside 100k other rows", ratio, 2.0));
    for (label, ratio, bound) in ratios {
        assert!(ratio < bound, "issue 533 {label}: {ratio:.2}x the pre-533 / uncrowded cost (bound {bound}x)");
    }
}
