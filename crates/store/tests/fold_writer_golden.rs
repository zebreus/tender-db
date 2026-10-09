//! Issue 495 unit 2, commit A, anchor (b): a cross-commit golden of the fold WRITER
//! itself (`Db::apply_tenders` and everything under it in canonical.rs), captured on
//! the commit before unit 2's byte-identical refactor touched it.
//!
//! Unit 2 rewrites the writer's plumbing and nothing else: a leaf-table descriptor
//! generates the INSERT prefixes and the per-version DELETEs, `Pending` becomes
//! leaf-indexed, `flush_rows` moves values instead of cloning them, `stored_chain`,
//! the DELETEs and the change INSERT become prepared statements, and a per-Tender
//! identity cache replaces most `lot_identity`/`result_identity` probes. Every one of
//! those can move bytes without failing a single same-run test, because the other fold
//! tests compare two runs that SHARE `apply_tenders`: a wrong `Leaf` at a push site of
//! equal arity, a flush in the wrong table order, a short `keep` from a mis-indexed
//! `stored_chain` column, a stale cache hit after a delete. This test is the guard
//! against that. It pins the writer's output to a golden file, byte for byte.
//!
//! The ingest-level goldens (`crates/ingest/tests/project_golden.rs`) fold real
//! notices, which is their strength and their limit: the corpus never reaches a second
//! flush chunk, never carries lot-group members, an `is_buyer` winner, a withheld
//! statistic or a converted non-EUR amount, and its chains are four versions long.
//! Here the `TenderProjection`s are built by hand, so the content can be exactly what
//! the refactor has to survive:
//!
//! * a 30-version keyed chain whose lots are dropped and re-added, with a lots group
//!   and its members, result rounds that accumulate version over version (one of them
//!   replaced by a correction), a lot result naming a lot no section declares, a winner
//!   flagged `is_buyer`, withheld amounts, bids and statistics, and amounts in GBP,
//!   PLN and SEK converted through a loaded rates table;
//! * one version (seq 30) heavy enough that its own rows split the batched leaf INSERT
//!   into more than one statement: 168 texts rows (`flush_rows` caps a statement at
//!   `MAX_BATCH_BIND / 6` = 150) and a round of 75 contracts (`900 / 13` = 69);
//! * an island Tender and a second keyed Tender (a `Part`, an unconvertible currency
//!   code, a bid without a value) that the middle phases must leave untouched.
//!
//! Five phases, each followed by a full digest of everything the writer owns:
//!
//! 1. a fresh apply (`rebuild = false`, so the identity probe and `stored_chain` run);
//! 2. an append: a 31st version arrives, keep = 30;
//! 3. a mid-chain insert: a late notice belongs between seq 14 and 15, so the tail is
//!    deleted and rewritten (keep = 14) and the orphan sweep runs and finds nothing;
//! 4. a shrink: the result notice at seq 23 goes, so its round leaves every later
//!    version, the tail is rewritten (keep = 22) and the sweep deletes its lot result,
//!    bid, contract and the one lot only it published, writing four 'removed' rows;
//! 5. an epoch-stale refold: every stored epoch aged to -1 and the same projections
//!    applied again, so keep = 0 rewrites every version of every Tender.
//!
//! The digest is the one `project_golden.rs` uses: every column of every one of the 14
//! version-keyed tables, rowid first, column lists read from `pragma_table_info` rather
//! than written here, in (tender_id, seq, rowid) order; `tenders` without `created_at`;
//! the four entity tables by id; the currency present-set; the change log by cursor
//! without `changed_at`; and the `PROJECTION_EPOCH` header. Each phase also records the
//! writer's own counters, so a count that drifts from the rows it reports shows too.
//!
//! The golden file `fixtures/golden/fold_writer.snapshot` is read at RUN TIME and
//! written only with `GOLDEN_CAPTURE_495=1`, so a broad `GOLDEN_CAPTURE` can never
//! rewrite it. It MUST NOT be regenerated to make a change pass: a diff here means the
//! derived layer moved, which is the ADR-0001 byte-identical violation unit 2 promised
//! not to commit. Regenerate it only for a DELIBERATE, reviewed change to the derived
//! layer's content, and say so in that commit.
//!
//! Issue 495 unit 4 makes a stale refold compare before it writes (ADR-0017), which
//! deliberately changes phase 5's rowids and change rows. Unit 4 must run this test with
//! `TENDER_REFOLD_COMPARE=off`, the kill switch that restores keep = 0 exactly; with
//! compare on, phase 5 is expected to differ and this golden is not its oracle.

use std::collections::BTreeSet;

use store::Db;
use store::canonical::{
    BidParty, BidState, ContractState, Fact, LotResultState, LotState, QUALITY_WITHHELD, Round, TenderProjection,
    TenderVersion,
};
use store::turso::{Connection, Value};

const DAY: i64 = 86_400;
/// 2026-01-01T00:00:00Z, so the instants below read as dates.
const BASE: i64 = 1_767_225_600;
/// The mega chain's notices are `1000 + k`; the late mid-chain notice is this one.
const LATE_NOTICE: i64 = 2014;

fn text(field: &str, lang: Option<&str>, value: impl Into<String>) -> Fact {
    Fact::Text { field: field.into(), lang: lang.map(Into::into), value: value.into() }
}

fn amount(field: &str, cents: i64, currency: &str, tax_basis: Option<&str>, quality: Option<&str>) -> Fact {
    Fact::Amount {
        field: field.into(),
        cents,
        currency: currency.into(),
        tax_basis: tax_basis.map(Into::into),
        quality: quality.map(Into::into),
    }
}

fn class(field: &str, scheme: &str, code: &str) -> Fact {
    Fact::Classification { field: field.into(), scheme: scheme.into(), code: code.into() }
}

fn date(field: &str, utc_seconds: i64, offset_minutes: i64, has_time: bool) -> Fact {
    Fact::Date { field: field.into(), utc_seconds, offset_minutes, has_time }
}

fn lot(key: &str, kind: &str, facts: Vec<Fact>) -> LotState {
    LotState { key: key.into(), kind: kind.into(), facts: facts.into_iter().collect() }
}

fn party(role: &str, organization_id: i64, section_id: &str) -> BidParty {
    BidParty { role: role.into(), organization_id, section_id: section_id.into() }
}

#[allow(clippy::too_many_arguments)]
fn result(
    key: &str,
    lot_key: Option<&str>,
    decision: Option<&str>,
    reason: Option<&str>,
    awarded: Option<(i64, &str)>,
    decided: Option<(i64, i64, bool)>,
    winners: &[i64],
    buyer_winners: &[i64],
    statistics: &[(&str, i64, Option<&str>)],
) -> LotResultState {
    LotResultState {
        key: key.into(),
        lot_key: lot_key.map(Into::into),
        decision: decision.map(Into::into),
        reason: reason.map(Into::into),
        awarded_cents: awarded.map(|(c, _)| c),
        awarded_currency: awarded.map(|(_, cur)| cur.into()),
        decided,
        winners: winners.to_vec(),
        buyer_winners: buyer_winners.to_vec(),
        statistics: statistics.iter().map(|(k, n, q)| ((*k).into(), *n, q.map(Into::into))).collect(),
    }
}

fn bid(key: &str, lot_key: Option<&str>, value: Option<(i64, &str)>, quality: Option<&str>, parties: Vec<BidParty>) -> BidState {
    BidState {
        key: key.into(),
        lot_key: lot_key.map(Into::into),
        cents: value.map(|(c, _)| c),
        currency: value.map(|(_, cur)| cur.into()),
        quality: quality.map(Into::into),
        parties,
    }
}

fn contract(
    key: &str,
    buyer_contract_id: Option<&str>,
    concluded: Option<(i64, i64, bool)>,
    decided: Option<(i64, i64, bool)>,
    value: Option<(i64, &str)>,
) -> ContractState {
    ContractState {
        key: key.into(),
        buyer_contract_id: buyer_contract_id.map(Into::into),
        concluded,
        decided,
        cents: value.map(|(c, _)| c),
        currency: value.map(|(_, cur)| cur.into()),
    }
}

/// One position of the mega chain: base notice `k` (`1000 + k`, published on day 3k),
/// or the late notice that belongs between k = 14 and k = 15.
#[derive(Clone, Copy)]
enum Step {
    Base(i64),
    Late,
}

/// The round a base notice publishes, if it is a result notice. Seq 18 opens the
/// results; 22 adds a round the shrink later removes; 25 adds one that 27 corrects
/// (same logical notice, so it REPLACES it); 30 is the heavy round of 75 contracts.
fn round_of(k: i64) -> Option<Round> {
    let notice_id = 1000 + k;
    let round = |logical: Option<&str>, lot_results, bids, contracts| Round {
        notice_id,
        logical_notice_id: logical.map(Into::into),
        lot_results,
        bids,
        contracts,
    };
    Some(match k {
        18 => round(
            None,
            vec![
                // Org 502 is the notice's own buyer: written with is_buyer = 1.
                result(
                    "RES-0001",
                    Some("LOT-0001"),
                    Some("selec-w"),
                    None,
                    Some((12_500_000, "GBP")),
                    Some((BASE + 50 * DAY + 36_000, 60, true)),
                    &[501, 502],
                    &[502],
                    &[("t-sme", 2, None), ("tenders", 4, None)],
                ),
                // LOT-0099 is declared by no section of any version: result_lot mints it.
                result(
                    "RES-0002",
                    Some("LOT-0099"),
                    Some("clos-nw"),
                    Some("no-rece"),
                    None,
                    Some((BASE + 51 * DAY, 0, false)),
                    &[],
                    &[],
                    &[("unpublished", -1, Some(QUALITY_WITHHELD))],
                ),
            ],
            vec![
                bid(
                    "TEN-0001",
                    Some("LOT-0001"),
                    Some((12_500_000, "GBP")),
                    None,
                    vec![party("tenderer", 501, "ORG-0005"), party("subcontractor", 503, "ORG-0006")],
                ),
                bid("TEN-0002", Some("LOT-0099"), Some((-1, "EUR")), Some(QUALITY_WITHHELD), vec![party("tenderer", 504, "ORG-0007")]),
                bid("TEN-0003", Some("GLO-0001"), Some((20_000_000, "EUR")), None, vec![party("tenderer", 505, "ORG-0008")]),
            ],
            vec![
                contract(
                    "CON-0001",
                    Some("RV-2026/118"),
                    Some((BASE + 55 * DAY + 36_000, 60, true)),
                    Some((BASE + 50 * DAY, 60, false)),
                    Some((12_500_000, "GBP")),
                ),
                contract("CON-0002", None, None, None, None),
            ],
        ),
        22 => round(
            None,
            vec![result(
                "RES-0001",
                Some("LOT-0007"),
                Some("selec-w"),
                None,
                Some((2_900_000, "SEK")),
                None,
                &[506],
                &[],
                &[("tenders", 1, None)],
            )],
            vec![bid("TEN-0001", Some("LOT-0007"), Some((2_900_000, "SEK")), None, vec![party("tenderer", 506, "ORG-0004")])],
            vec![contract("CON-0001", Some("EM-7"), Some((BASE + 68 * DAY, 60, false)), None, Some((2_900_000, "SEK")))],
        ),
        25 => round(
            Some("LOG-0025"),
            vec![result(
                "RES-0001",
                Some("LOT-0002"),
                Some("selec-w"),
                None,
                Some((199_000_000, "PLN")),
                None,
                &[507],
                &[],
                &[("tenders", 1, None)],
            )],
            Vec::new(),
            vec![contract("CON-0001", None, Some((BASE + 76 * DAY, 60, true)), None, Some((199_000_000, "PLN")))],
        ),
        27 => round(
            Some("LOG-0025"),
            vec![result(
                "RES-0001",
                Some("LOT-0002"),
                Some("selec-w"),
                None,
                Some((198_500_000, "PLN")),
                None,
                &[507, 900],
                &[900],
                &[("tenders", 2, None)],
            )],
            Vec::new(),
            vec![contract("CON-0001", None, Some((BASE + 76 * DAY, 60, true)), None, Some((198_500_000, "PLN")))],
        ),
        30 => round(
            None,
            vec![result(
                "RES-0001",
                Some("LOT-0003"),
                Some("selec-w"),
                None,
                Some((7_400_000, "GBP")),
                None,
                &[508, 509],
                &[],
                &[],
            )],
            Vec::new(),
            (1..=75)
                .map(|i: i64| {
                    let cur = ["EUR", "PLN", "GBP"][(i % 3) as usize];
                    contract(
                        &format!("CON-{i:04}"),
                        Some(format!("CALL-{i:03}").as_str()),
                        Some((BASE + (95 + i % 5) * DAY + 36_000, 60, i % 2 == 0)),
                        (i % 3 != 0).then_some((BASE + 92 * DAY, 60, false)),
                        Some((100_000 + i * 1_000, cur)),
                    )
                })
                .collect(),
        ),
        _ => return None,
    })
}

/// Add a published round to the running set the way the fold accumulates them:
/// additive, except that a correction of the same logical notice replaces its round.
fn publish(rounds: &mut Vec<Round>, round: Round) {
    let corrected = rounds
        .iter()
        .position(|r| r.logical_notice_id.is_some() && r.logical_notice_id == round.logical_notice_id);
    match corrected {
        Some(i) => rounds[i] = round,
        None => rounds.push(round),
    }
}

/// The mega chain's version at position `k` (the late notice reads as k = 14 with its
/// own changes). Tender facts change every third version, so some versions are silent
/// on the change feed; lots come and go by the rules in the body.
fn mega_version(k: i64, late: bool, rounds: Vec<Round>) -> TenderVersion {
    let notice = if late { LATE_NOTICE } else { 1000 + k };
    let published_at = if late { BASE + 43 * DAY + 36_000 } else { BASE + 3 * k * DAY + 36_000 };
    let era = k / 3;
    let deadline = BASE + (40 + 9 * era + i64::from(late) * 7) * DAY + 43_200;

    let mut facts = vec![
        text("title", Some("ENG"), format!("Road maintenance framework, revision {era}")),
        text("title", Some("DEU"), format!("Straßeninstandhaltung – Rahmenvertrag 'Los {era}'")),
        text("description", None, "Unlabelled description, kept verbatim"),
        amount("estimated_value", 50_000_000 + era * 100_000, "EUR", Some("excl"), None),
        date("submission_deadline", deadline, 60, true),
        class("main", "cpv", "45233141"),
        class("place", "nuts", "DE212"),
        Fact::Party { role: "buyer".into(), organization_id: 900, notice_id: (1000 + 3 * era).max(1001), section_id: "ORG-0001".into() },
    ];
    if k >= 12 {
        facts.push(amount("max_value", -1, "EUR", None, Some(QUALITY_WITHHELD)));
    }
    if k >= 6 {
        facts.push(date("contract_start", BASE + 200 * DAY, 0, false));
    }
    if era % 2 == 0 {
        facts.push(class("additional", "cpv", "71311000"));
    }
    if k == 30 && !late {
        // The heavy version: 160 more texts rows, on top of its 8 ordinary ones.
        const LANGS: [&str; 8] = ["BUL", "CES", "DAN", "DEU", "ELL", "ENG", "EST", "FIN"];
        for i in 0..160usize {
            facts.push(text("description", Some(LANGS[i % 8]), format!("Heavy description part {i:03}: Leistungsbeschreibung é ü")));
        }
    }

    let lot3 = k <= 8 || k >= 16 || late;
    let lot4 = (5..=12).contains(&k) || ((24..=30).contains(&k) && !late);
    let mut lots = vec![
        lot(
            "LOT-0001",
            "Lot",
            vec![
                text("title", Some("ENG"), format!("Lot 1 carriageway, revision {}", k / 5)),
                amount("estimated_value", 10_000_000, "EUR", Some("excl"), None),
                date("submission_deadline", deadline, 60, true),
            ],
        ),
        lot(
            "LOT-0002",
            "Lot",
            vec![
                text("title", Some("ENG"), "Lot 2 bridges"),
                amount("estimated_value", 200_000_000 + (k / 4) * 1_000, "PLN", Some("incl"), None),
            ],
        ),
    ];
    if lot3 {
        lots.push(lot(
            "LOT-0003",
            "Lot",
            vec![
                text("title", Some("ENG"), "Lot 3 winter service"),
                amount("estimated_value", 7_500_000, "GBP", None, None),
                amount("max_value", -1, "EUR", None, Some(QUALITY_WITHHELD)),
            ],
        ));
    }
    if lot4 {
        lots.push(lot(
            "LOT-0004",
            "Lot",
            vec![
                text("title", Some("ENG"), "Lot 4 signage"),
                amount("estimated_value", 90_000_000, "SEK", None, None),
                class("main", "cpv", "34928471"),
            ],
        ));
    }
    if k == 22 && !late {
        lots.push(lot(
            "LOT-0007",
            "Lot",
            vec![text("title", Some("ENG"), "Lot 7 emergency repairs"), amount("estimated_value", 3_000_000, "SEK", None, None)],
        ));
    }
    let mut group_members = Vec::new();
    if k >= 10 {
        lots.push(lot("GLO-0001", "LotsGroup", vec![text("title", Some("ENG"), "Lots 1-3 combined")]));
        group_members.push(("GLO-0001".to_owned(), "LOT-0001".to_owned()));
        group_members.push(("GLO-0001".to_owned(), "LOT-0002".to_owned()));
        if lot3 {
            group_members.push(("GLO-0001".to_owned(), "LOT-0003".to_owned()));
        }
    }

    let result_notice = matches!(k, 18 | 22 | 25 | 27 | 30) && !late;
    TenderVersion {
        caused_by_notice_id: notice,
        published_at,
        dispatched_at: (k % 2 == 1).then_some(published_at - DAY),
        notice_subtype: if k == 13 { None } else if result_notice { Some("29".into()) } else { Some("16".into()) },
        original_lang: if k % 7 == 0 { None } else { Some("DEU".into()) },
        publication_id: format!("{notice}-2026"),
        facts: facts.into_iter().collect::<BTreeSet<_>>(),
        lots,
        rounds,
        group_members,
    }
}

fn mega_chain(steps: impl IntoIterator<Item = Step>) -> TenderProjection {
    let mut rounds = Vec::new();
    let versions = steps
        .into_iter()
        .map(|step| match step {
            Step::Base(k) => {
                if let Some(round) = round_of(k) {
                    publish(&mut rounds, round);
                }
                mega_version(k, false, rounds.clone())
            }
            Step::Late => mega_version(14, true, rounds.clone()),
        })
        .collect();
    TenderProjection {
        source: "ted".into(),
        procedure_key: Some("golden-mega-chain".into()),
        island_notice_id: None,
        kind: "procedure".into(),
        versions,
    }
}

/// An island Tender: one notice, no procedure key, no lots.
fn island() -> TenderProjection {
    TenderProjection {
        source: "ted".into(),
        procedure_key: None,
        island_notice_id: Some(3001),
        kind: "registration".into(),
        versions: vec![TenderVersion {
            caused_by_notice_id: 3001,
            published_at: BASE + 20 * DAY,
            dispatched_at: None,
            notice_subtype: None,
            original_lang: None,
            publication_id: "3001-2026".into(),
            facts: [
                text("title", Some("ENG"), "Supplier registration system"),
                amount("estimated_value", 5_000_000, "SEK", None, None),
                class("main", "cpv", "79000000"),
            ]
            .into_iter()
            .collect(),
            lots: Vec::new(),
            rounds: Vec::new(),
            group_members: Vec::new(),
        }],
    }
}

/// A second keyed Tender: a `Part`, an unconvertible currency code (`GPB`, the
/// observed typo, so eur_cents is honestly NULL) and a bid that published no value.
fn keyed_parts() -> TenderProjection {
    let version = |notice: i64, day: i64, rounds: Vec<Round>| TenderVersion {
        caused_by_notice_id: notice,
        published_at: BASE + day * DAY,
        dispatched_at: Some(BASE + (day - 2) * DAY),
        notice_subtype: Some(if rounds.is_empty() { "16" } else { "29" }.into()),
        original_lang: Some("DEU".into()),
        publication_id: format!("{notice}-2026"),
        facts: [text("title", Some("DEU"), "Schulbau Teil 'A'"), amount("estimated_value", 1_000_000, "GPB", None, None)]
            .into_iter()
            .collect(),
        lots: vec![lot("PAR-0001", "Part", vec![text("title", None, "Teil 1"), amount("estimated_value", 400_000, "GPB", None, None)])],
        rounds,
        group_members: Vec::new(),
    };
    let round = Round {
        notice_id: 4002,
        logical_notice_id: None,
        lot_results: vec![result("RES-0001", Some("PAR-0001"), Some("selec-w"), None, None, None, &[601], &[], &[])],
        bids: vec![bid("TEN-0001", Some("PAR-0001"), None, None, vec![party("tenderer", 601, "ORG-0002")])],
        contracts: vec![contract("CON-0001", None, None, None, Some((390_000, "EUR")))],
    };
    TenderProjection {
        source: "doe".into(),
        procedure_key: Some("golden-keyed-parts".into()),
        island_notice_id: None,
        kind: "procedure".into(),
        versions: vec![version(4001, 5, Vec::new()), version(4002, 40, vec![round])],
    }
}

/// Daily GBP, PLN and SEK rates over the whole chain, each a little different every
/// day, so a version converted at the wrong date reads differently in the digest.
fn rates() -> Vec<(String, String, f64, String)> {
    let mut rows = Vec::new();
    for d in 0..140i64 {
        let day = store::rates::civil_date(BASE + (d - 10) * DAY);
        let d = d as f64;
        for (currency, rate) in [("GBP", 0.84 + 0.0001 * d), ("PLN", 4.25 + 0.001 * d), ("SEK", 11.0 + 0.002 * d)] {
            rows.push((currency.to_owned(), day.clone(), rate, "ecb".to_owned()));
        }
    }
    rows
}

/// The first column of the first row, with the statement DRAINED to its end.
///
/// Not `Db::scalar`, which reads one row and drops the statement on a POOLED reader. For
/// the `pragma_table_info()` table-valued function the digest uses, turso 0.7.2 then leaves
/// the read snapshot open, and the connection goes back to the pool holding it
/// (`is_autocommit()` still reads true). Reproduced 2026-10-08: only `pragma_*` functions
/// do it; a drained one, the `PRAGMA` statement, `generate_series`, `json_each` and a
/// plain partial read do not (docs/research/turso-scale.md). Observed here first: with
/// every digest query through `Db::scalar`, all five digests showed phase 1's rows (33
/// versions, 146 change rows) while the writer reported, for example, 18 versions written
/// and 17 removed in phase 3. Each digest therefore opens a fresh connection (see
/// [`full_digest`]) and every statement here runs to completion.
async fn text_of(conn: &Connection, sql: &str) -> String {
    let mut rows = conn.query(sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
    let first = rows.next().await.unwrap_or_else(|e| panic!("{sql}: {e}")).map(|row| row.get_value(0).expect("column 0"));
    while rows.next().await.unwrap_or_else(|e| panic!("{sql}: {e}")).is_some() {}
    match first {
        Some(Value::Text(s)) => s,
        Some(Value::Null) | None => String::new(),
        other => panic!("{sql}: expected text, got {other:?}"),
    }
}

/// One table's rows, every column `quote()`d (NULL stays distinguishable from '' and 0), rowid
/// first, in `order`. `skip` names columns that hold the wall clock. The same shape as
/// `project_golden.rs`'s, on purpose: the two goldens read alike.
async fn table_digest(db: &Connection, table: &str, order: &str, skip: &[&str]) -> String {
    let cols = text_of(db, &format!("SELECT group_concat(name, ',') FROM pragma_table_info('{table}')")).await;
    assert!(!cols.is_empty(), "{table}: pragma_table_info named no columns");
    let mut expr = String::from("quote(rowid)");
    for col in cols.split(',').filter(|c| !skip.contains(c)) {
        expr.push_str(&format!("||'|'||quote(\"{col}\")"));
    }
    let rows = text_of(db, &format!("SELECT group_concat(r, x'0a') FROM (SELECT {expr} AS r FROM \"{table}\" ORDER BY {order})")).await;
    let count = text_of(db, &format!("SELECT CAST(COUNT(*) AS TEXT) FROM \"{table}\"")).await;
    format!("--- {table} ({count} rows; {cols}) ---\n{rows}\n")
}

async fn full_digest(db: &Db) -> String {
    // A new one-connection pool per digest: a connection that has never read cannot be
    // holding an older snapshot (see `text_of`).
    let pool = db.readers(1).expect("digest reader pool");
    let reader = pool.get().await.expect("digest reader");
    let db: &Connection = &reader;
    let leaves = text_of(
        db,
        "SELECT group_concat(name, ',') FROM (SELECT name FROM sqlite_master WHERE type = 'table' \
           AND (name = 'tender_versions' OR name GLOB 'tender_version_*') ORDER BY name)",
    )
    .await;
    assert_eq!(leaves.split(',').count(), 14, "the fold's version-keyed tables: {leaves}");
    let mut out = format!("--- projection epoch ---\n{}\n", store::canonical::PROJECTION_EPOCH);
    for table in leaves.split(',') {
        out.push_str(&table_digest(db, table, "tender_id, seq, rowid", &[]).await);
    }
    out.push_str(&table_digest(db, "tenders", "id", &["created_at"]).await);
    for table in ["lots", "lot_results", "bids", "contracts"] {
        out.push_str(&table_digest(db, table, "id", &[]).await);
    }
    out.push_str(&table_digest(db, "tender_currency_presence", "rowid", &[]).await);
    out.push_str(&table_digest(db, "changes", "cursor", &["changed_at"]).await);
    out
}

/// Apply one phase through the real writer and append its counters and digest. `now`
/// differs per phase; it lands only in `created_at` and `changed_at`, which the digest
/// leaves out.
async fn phase(db: &Db, out: &mut String, title: &str, projections: &[TenderProjection], now: i64) -> store::canonical::Applied {
    let a = db.apply_tenders(projections, now, false).await.unwrap_or_else(|e| panic!("{title}: apply_tenders: {e}"));
    out.push_str(&format!(
        "=== {title} ===\napplied: tenders_created={} tenders_written={} tenders_unchanged={} versions_written={} \
         versions_removed={} entities_swept={} changes={} leaf_rows={}\n",
        a.tenders_created,
        a.tenders_written,
        a.tenders_unchanged,
        a.versions_written,
        a.versions_removed,
        a.entities_swept,
        a.changes,
        a.leaf_rows,
    ));
    out.push_str(&full_digest(db).await);
    a
}

/// The fold writer's output across fresh / append / mid-chain insert / shrink /
/// epoch-stale refold is byte-for-byte the committed golden. See the module header:
/// the golden is a cross-commit anchor and must not be regenerated to pass.
///
/// Run on an explicit large-stack thread, as `project_golden.rs` does: turso's
/// debug-build query execution (the wide `group_concat` digests in particular) can
/// overflow libtest's default worker stack.
#[test]
fn fold_writer_output_matches_the_committed_golden() {
    on_a_big_stack(store::RefoldCompare::Off);
}

/// Issue 495 unit 3: the same five phases with the shadow compare on write the same golden,
/// and the epoch-stale refold of the mega chain (lot groups, result rounds, `is_buyer`
/// winners, bids, converted amounts, a 168-row version) compares identical, Tender by Tender.
#[test]
fn fold_writer_output_holds_in_shadow_and_its_stale_refold_verifies() {
    on_a_big_stack(store::RefoldCompare::Shadow);
}

fn on_a_big_stack(compare: store::RefoldCompare) {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime")
                .block_on(run(compare))
        })
        .expect("spawn")
        .join()
        .expect("join");
}

async fn run(compare: store::RefoldCompare) {
    let path = format!("/tmp/tender-db-foldwriter-golden-{compare:?}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = Db::open(&path).await.expect("open scratch db");
    db.set_refold_compare(compare);
    if compare != store::RefoldCompare::Off {
        // The compare refuses to run without `tender_version_bid_parties_version`.
        db.build_tender_indexes().await.expect("the by-version indexes");
    }
    // The projection runs FK-off; the parties, winners and versions here reference
    // organizations and notices this file never creates.
    db.set_foreign_keys(false).await.expect("foreign keys off");
    let rates = rates();
    db.upsert_currency_rates(&rates).await.expect("load rates");
    assert_eq!(db.reload_rates_lookup().await.expect("reload rates"), rates.len(), "every rate row is in the lookup");

    let base = |range: std::ops::RangeInclusive<i64>| range.map(Step::Base);
    let others = [island(), keyed_parts()];
    let with = |chain: TenderProjection| {
        let mut all = vec![chain];
        all.extend(others.iter().cloned());
        all
    };
    let mut got = String::new();

    let a = phase(&db, &mut got, "1: fresh apply", &with(mega_chain(base(1..=30))), 1_800_000_001).await;
    assert_eq!((a.tenders_created, a.versions_written, a.versions_removed), (3, 33, 0), "fresh: {a:?}");

    let a = phase(&db, &mut got, "2: append (keep = 30)", &with(mega_chain(base(1..=31))), 1_800_000_002).await;
    assert_eq!(
        (a.tenders_written, a.tenders_unchanged, a.versions_written, a.versions_removed, a.entities_swept),
        (1, 2, 1, 0, 0),
        "append writes the new version only: {a:?}"
    );

    let late = || base(1..=14).chain([Step::Late]).chain(base(15..=31));
    let a = phase(&db, &mut got, "3: mid-chain insert (keep = 14)", &with(mega_chain(late())), 1_800_000_003).await;
    assert_eq!(
        (a.tenders_written, a.tenders_unchanged, a.versions_written, a.versions_removed, a.entities_swept),
        (1, 2, 18, 17, 0),
        "the late notice deletes and rewrites the tail, and the sweep finds nothing: {a:?}"
    );

    let shrunk = || base(1..=14).chain([Step::Late]).chain(base(15..=21)).chain(base(23..=31));
    let a = phase(&db, &mut got, "4: shrink (keep = 22)", &with(mega_chain(shrunk())), 1_800_000_004).await;
    assert_eq!(
        (a.tenders_written, a.tenders_unchanged, a.versions_written, a.versions_removed, a.entities_swept),
        (1, 2, 9, 10, 4),
        "dropping the seq-23 result notice sweeps its lot result, bid, contract and lot: {a:?}"
    );

    db.set_projection_epoch_for_test(-1).await.expect("age every Tender");
    let a = phase(&db, &mut got, "5: epoch-stale refold (keep = 0)", &with(mega_chain(shrunk())), 1_800_000_005).await;
    assert!(
        a.tenders_written == 3 && a.versions_written == 34 && a.versions_removed == a.versions_written && a.entities_swept == 0,
        "every stale Tender is rewritten from keep = 0 and an unchanged rewrite sweeps nothing: {a:?}"
    );
    if compare == store::RefoldCompare::Shadow {
        assert_eq!(
            (a.tenders_verified, a.tenders_corrected, a.tables_rewritten, a.correction_rows_planned),
            (3, 0, 0, 0),
            "the stale refold of unchanged projections compares identical: {a:?}"
        );
        assert_eq!((a.compare_rows, a.compare_versions), (a.rows_skipped, 34), "{a:?}");
    }

    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/golden/fold_writer.snapshot");
    if std::env::var_os("GOLDEN_CAPTURE_495").is_some() {
        std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/golden")).expect("golden dir");
        std::fs::write(file, &got).expect("write golden");
    }
    let golden = std::fs::read_to_string(file).expect(
        "tests/fixtures/golden/fold_writer.snapshot is missing — it is captured once, with \
         GOLDEN_CAPTURE_495=1, on a commit that changes no production code",
    );
    // Not assert_eq!: on a mismatch that would print two ~10k-line strings. Name the
    // first differing line instead; `diff` the files for the rest.
    if got != golden {
        let line = got.lines().zip(golden.lines()).position(|(g, w)| g != w).unwrap_or_else(|| got.lines().count().min(golden.lines().count()));
        let dump = format!("/tmp/fold_writer.snapshot.got-{compare:?}-{}", std::process::id());
        let _ = std::fs::write(&dump, &got);
        panic!(
            "the fold writer's output diverged from the committed golden \
             (tests/fixtures/golden/fold_writer.snapshot) at line {}: got {:?}, golden {:?}. \
             This run's output is in {dump}. Do NOT regenerate the golden to make this pass — \
             a diff means the derived layer moved (ADR-0001), which issue 495 unit 2 promised not to do.",
            line + 1,
            got.lines().nth(line),
            golden.lines().nth(line),
        );
    }

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
