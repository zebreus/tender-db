//! Issue 551 B6 U3: TEMPORARY port-parity harness for path W v2 + G (R10's candidate guards), removed after the run;
//! the record copy lives in `.scratch/tender-db/551-lot-sum/b6/u3/harness/`. Derived from B3's
//! `canonical_b3_harness.rs` (same input line format, same S / currency / n reading). Included from canonical.rs by
//! three lines after `mod lot_sum_551_v2_fixtures;`:
//!
//!     #[cfg(test)]
//!     #[path = "canonical_u3_harness.rs"]
//!     mod u3_harness;
//!
//! `U3_JOBS` is `IN=OUT;IN=OUT;...`: each IN is a file in the line format `b3/export.py` / `b3/records.py` /
//! `b6/u3/export_u3.py` write (one `T` block per Tender, its chain oldest first); each OUT gets FOUR tab-separated
//! lines per Tender, one per guard set in the port's `GUARD_SETS` order (`R10Guards::ALL`: none, single, cover,
//! both):
//!
//!     id  guard  rust_v1_fires  ship  first  S  currency  n  vouched_field  vouched_notice  vouched_cents
//!        derived_kind  derived_cents  derived_currency  derived_lots  today_eur
//!        hv_eur  hv_derivation  hv_cents  hv_currency  hv_lots  hv_vouched_field  hv_vouched_notice  hv_vouched_cents
//!        hv_equals_check  production_eur  production_derivation
//!
//! `ship` / `first` are `awards_path_check_guards(chain, rates, guards)` (path W switched on): `Ok` with an awards
//! derivation is a ship; `Ok` with a vouched derivation is path V (first `V`, ship 0, as the port's `v1_reason`);
//! `Err(name)` is the first refusing check (`R10` when a guard empties R10's candidates). The `hv_*` columns are
//! `head_value_with_awards_path_guards(chain, rates, guards)` on the same chain; `hv_equals_check` is 1 when its
//! derivation is exactly the check's `Ok` derivation (or, on an `Err`, when it derives no awards sum). S / currency /
//! n are the summed set path W reads (B3's reading: K's currency, today's figure's or, on a null head, the currency
//! with the most award lots; `LotSums::per_lot(Awards)` there), printed on every line so refusals compare too.
//! `rust_v1_fires` is 1 when the first check is an `R*` or a ship (every v1 guard T'..G4 passed). The production
//! columns are `head_value` (path W off, as deployed) for the record.
use super::{
    BidParty, BidState, DerivationPath, DerivedValue, Fact, HeadValue, LotResultState, LotState, LotSumKind, LotSums,
    R10Guards, Round, ScalePartners, StatedTotal, StatedTotalKind, TenderVersion, awards_path_check_guards,
    elect_head_figure, head_value, head_value_with_awards_path_guards,
};
use std::collections::BTreeSet;

const PUBLISHED_AT: i64 = 1_700_000_000;

fn dec(s: &str) -> Option<String> {
    if s == "-" {
        return None;
    }
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).expect("percent escape"));
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    Some(String::from_utf8(out).expect("utf-8"))
}

fn opt_i64(s: &str) -> Option<i64> {
    (s != "-").then(|| s.parse().expect("integer"))
}

fn amount(p: &[&str]) -> Fact {
    Fact::Amount {
        field: dec(p[1]).expect("field"),
        cents: p[2].parse().expect("cents"),
        currency: dec(p[3]).expect("currency"),
        tax_basis: None,
        quality: dec(p[4]),
    }
}

fn show<T: ToString>(x: Option<T>) -> String {
    x.map_or_else(|| "-".to_owned(), |v| v.to_string())
}

fn derivation_name(d: Option<&DerivedValue>) -> String {
    match d.map(|d| &d.path) {
        None => "-".to_owned(),
        Some(DerivationPath::Vouched { rule, .. }) => format!("vouched:{}", rule.issue()),
        Some(DerivationPath::Awards { .. }) => "awards".to_owned(),
    }
}

const GUARD_NAMES: [&str; 4] = ["none", "single", "cover", "both"];

fn evaluate(id: i64, chain: &[TenderVersion], rates: &crate::rates::RatesLookup) -> String {
    let head = chain.last().expect("a chain");
    let partners = ScalePartners::of_chain(chain);
    let date = crate::rates::civil_date(head.published_at);
    let today = elect_head_figure(head, &partners, rates, &date);
    let sums = LotSums::by_currency(head);
    let currency: Option<&str> = match &today {
        Some(t) => Some(t.currency),
        None => sums.iter().filter(|(_, s)| s.award_lots() > 0).map(|(c, s)| (s.award_lots(), *c)).max().map(|x| x.1),
    };
    let awards = currency.and_then(|c| sums.get(c)).map(|ls| ls.per_lot(LotSumKind::Awards)).unwrap_or_default();
    let n = awards.len();
    let s = (n > 0).then(|| awards.iter().fold(0i64, |a, &(_, c)| a.saturating_add(c)));
    let production = head_value(chain, rates);
    let mut out = String::new();
    for (name, guards) in GUARD_NAMES.iter().zip(R10Guards::ALL) {
        let decision = awards_path_check_guards(chain, rates, guards);
        let hv: HeadValue = head_value_with_awards_path_guards(chain, rates, guards);
        let (ship, first, vouched, derived) = match &decision {
            Ok(d) => match &d.path {
                DerivationPath::Awards { vouched_by } => (true, None, Some(vouched_by.clone()), Some(d.clone())),
                DerivationPath::Vouched { .. } => (false, Some("V"), None, Some(d.clone())),
            },
            Err(name) => (false, Some(*name), None, None),
        };
        let v1_fires = ship || first.is_some_and(|f| f.starts_with('R'));
        let hv_vouched = match hv.derived.as_ref().map(|d| &d.path) {
            Some(DerivationPath::Awards { vouched_by }) => Some(vouched_by.clone()),
            _ => None,
        };
        let hv_equals_check = match &decision {
            Ok(d) => hv.derived.as_ref() == Some(d),
            Err(_) => hv_vouched.is_none(),
        };
        let cols = [
            id.to_string(),
            (*name).to_owned(),
            (v1_fires as u8).to_string(),
            (ship as u8).to_string(),
            first.unwrap_or("-").to_owned(),
            show(s),
            currency.unwrap_or("-").to_owned(),
            n.to_string(),
            show(vouched.as_ref().map(|v| v.field.clone())),
            show(vouched.as_ref().map(|v| v.notice_id)),
            show(vouched.as_ref().map(|v| v.cents)),
            show(derived.as_ref().map(|d| d.kind.label())),
            show(derived.as_ref().map(|d| d.cents)),
            show(derived.as_ref().map(|d| d.currency.clone())),
            show(derived.as_ref().map(|d| d.lots)),
            show(today.as_ref().map(|t| t.eur_cents)),
            show(hv.eur_cents),
            derivation_name(hv.derived.as_ref()),
            show(hv.derived.as_ref().map(|d| d.cents)),
            show(hv.derived.as_ref().map(|d| d.currency.clone())),
            show(hv.derived.as_ref().map(|d| d.lots)),
            show(hv_vouched.as_ref().map(|v| v.field.clone())),
            show(hv_vouched.as_ref().map(|v| v.notice_id)),
            show(hv_vouched.as_ref().map(|v| v.cents)),
            (hv_equals_check as u8).to_string(),
            show(production.eur_cents),
            derivation_name(production.derived.as_ref()),
        ];
        out.push_str(&(cols.join("\t") + "\n"));
    }
    out
}

fn run_one(input: &str, output: &str) -> usize {
    let text = std::fs::read_to_string(input).expect("read U3 input");
    let date = crate::rates::civil_date(PUBLISHED_AT);
    let mut out = String::new();
    let mut id = 0i64;
    let mut rates = crate::rates::RatesLookup::default();
    let mut chain: Vec<TenderVersion> = Vec::new();
    let mut tenders = 0usize;
    for line in text.lines() {
        let p: Vec<&str> = line.split('\t').collect();
        match p[0] {
            "T" => {
                id = p[1].parse().expect("id");
                rates = crate::rates::RatesLookup::default();
                chain.clear();
            }
            "R" => {
                let currency = dec(p[1]).expect("currency");
                rates.insert_for_test(crate::rates::canonical_currency(&currency), &date, p[2].parse().expect("rate"));
            }
            "V" => {
                let n = chain.len() + 1;
                chain.push(TenderVersion {
                    caused_by_notice_id: p[1].parse().expect("notice"),
                    published_at: PUBLISHED_AT,
                    dispatched_at: None,
                    notice_subtype: None,
                    original_lang: None,
                    publication_id: format!("{id}-{n}"),
                    facts: BTreeSet::new(),
                    lots: Vec::new(),
                    rounds: Vec::new(),
                    group_members: Vec::new(),
                    stated_totals: Vec::new(),
                });
            }
            "F" => {
                chain.last_mut().expect("V").facts.insert(amount(&p));
            }
            "L" => {
                let mut facts = BTreeSet::new();
                if let Some(title) = dec(p[4]) {
                    facts.insert(Fact::Text { field: "title".into(), lang: None, value: title });
                }
                chain.last_mut().expect("V").lots.push(LotState {
                    key: dec(p[1]).expect("key"),
                    kind: dec(p[2]).expect("kind"),
                    facts,
                    published_lot_no: dec(p[3]),
                });
            }
            "LF" => {
                chain.last_mut().expect("V").lots.last_mut().expect("L").facts.insert(amount(&p));
            }
            "N" => chain.last_mut().expect("V").rounds.push(Round {
                notice_id: p[1].parse().expect("notice"),
                logical_notice_id: None,
                lot_results: Vec::new(),
                bids: Vec::new(),
                contracts: Vec::new(),
            }),
            "A" => {
                let winners: Vec<i64> =
                    if p[6] == "-" { Vec::new() } else { p[6].split(',').map(|w| w.parse().expect("winner")).collect() };
                chain.last_mut().expect("V").rounds.last_mut().expect("N").lot_results.push(LotResultState {
                    key: dec(p[1]).expect("result key"),
                    lot_key: dec(p[2]),
                    decision: dec(p[5]),
                    reason: None,
                    awarded_cents: opt_i64(p[3]),
                    awarded_currency: dec(p[4]),
                    decided: None,
                    winners,
                    buyer_winners: Vec::new(),
                    statistics: Vec::new(),
                    published_lot_no: dec(p[7]),
                    group: p[8] == "1",
                });
            }
            "B" => {
                let parties: Vec<BidParty> = if p[6] == "-" {
                    Vec::new()
                } else {
                    p[6].split(',')
                        .map(|x| {
                            let (role, org) = x.rsplit_once(':').expect("role:org");
                            BidParty { role: dec(role).expect("role"), organization_id: org.parse().expect("org"), section_id: String::new() }
                        })
                        .collect()
                };
                chain.last_mut().expect("V").rounds.last_mut().expect("N").bids.push(BidState {
                    key: dec(p[1]).expect("bid key"),
                    lot_key: dec(p[2]),
                    cents: opt_i64(p[3]),
                    currency: dec(p[4]),
                    quality: dec(p[5]),
                    parties,
                });
            }
            "S" => {
                let kind = match p[4] {
                    "Award" => StatedTotalKind::Award,
                    "Estimate" => StatedTotalKind::Estimate,
                    "Range" => StatedTotalKind::Range,
                    other => panic!("kind {other}"),
                };
                let lots: Vec<String> =
                    if p[7] == "-" { Vec::new() } else { p[7].split(',').map(|k| dec(k).expect("lot")).collect() };
                chain.last_mut().expect("V").stated_totals.push(StatedTotal {
                    notice_id: p[1].parse().expect("notice"),
                    logical_notice_id: dec(p[2]),
                    field: dec(p[3]).expect("field"),
                    kind,
                    cents: p[5].parse().expect("cents"),
                    currency: dec(p[6]).expect("currency"),
                    lots_awarded: lots,
                });
            }
            "E" => {
                out.push_str(&evaluate(id, &chain, &rates));
                tenders += 1;
            }
            other => panic!("unknown line kind {other:?}"),
        }
    }
    std::fs::write(output, out).expect("write U3 output");
    tenders
}

#[test]
#[ignore = "issue 551 B6 U3 parity harness: reads each IN of U3_JOBS, writes its OUT"]
fn u3_parity() {
    let Ok(jobs) = std::env::var("U3_JOBS") else {
        eprintln!("U3 PARITY HARNESS: U3_JOBS unset, nothing to do");
        return;
    };
    for job in jobs.split(';').filter(|j| !j.is_empty()) {
        let (input, output) = job.split_once('=').expect("IN=OUT");
        let tenders = run_one(input, output);
        eprintln!("U3 PARITY HARNESS: {tenders} Tenders x 4 guard sets evaluated from {input} -> {output}");
    }
}
