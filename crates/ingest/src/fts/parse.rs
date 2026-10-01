//! Find a Tender (UK) OCDS 1.1 releases → the notice-parsed layer.
//!
//! The member handed here is a single-release OCDS *package* (D1): the header
//! the publisher attributes the data with, plus one release. Everything the
//! projection reads is inside that release.
//!
//! **Named paths, never a generic JSON document.** `serde_json::Value` cannot
//! hold this source: release `083529-2026` published `1e9999` in a number
//! slot, which `serde_json` refuses as "number out of range", and a document
//! model would have failed the whole day rather than the one release. So every
//! number arrives as a `RawValue` and is converted from its literal text —
//! the same rule [`crate::fts::Page`] follows for the page envelope.
//!
//! The mapping is deliberately LENIENT about structure and STRICT about
//! values. The UK extension moved four times in a year, so an unknown key is
//! ignored rather than quarantined; but an amount that cannot be represented
//! exactly quarantines the notice, because a wrong number is worse than a
//! missing source (ADR-0004). Three things quarantine: unparsable JSON, a
//! release with no `ocid` (nothing to key a Tender on), and an unrepresentable
//! amount.
//!
//! Field ids are the eForms ones the projection already reads. That is not
//! cosmetic: it is what lets a UK notice fold into the same `Tender` shape as
//! a TED one without the projection learning a dialect.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use serde_json::value::RawValue;
use store::{NoticeValue, Parsed, Section, ValueRow};

use crate::eforms::value as eforms;

/// The root section every profile in the corpus uses.
const ROOT: &str = "PROCEDURE";

/// A settled contract's own published value (issue 386 unit 2). Named in the
/// source's vocabulary because eForms has no contract-value BT to reuse — see
/// the emission site for why that is a departure worth making.
pub(crate) const CONTRACT_VALUE: &str = "OCDS-ContractValue";

/// Why a release was refused.
pub struct Rejected {
    pub reason: &'static str,
    pub detail: String,
}

/// Parse an FTS member, mapping every outcome onto what the store records.
pub fn parse_payload(profile: &str, bytes: &[u8]) -> store::Parse {
    if !profile.starts_with("fts:") {
        return store::Parse::Pending;
    }
    match parse(bytes) {
        Ok(parsed) => store::Parse::Parsed(parsed),
        Err(Rejected { reason, detail }) => {
            store::Parse::Quarantined { reason: reason.into(), detail: Some(detail) }
        }
    }
}

/// Parse one single-release OCDS package.
pub fn parse(bytes: &[u8]) -> Result<Parsed, Rejected> {
    let package: Package = serde_json::from_slice(bytes)
        .map_err(|e| Rejected { reason: "unparsable-json", detail: e.to_string() })?;
    let Some(release) = package.releases.into_iter().next() else {
        return Err(Rejected {
            reason: "ocds-release-count",
            detail: "no release in the package".to_owned(),
        });
    };
    let Some(ocid) = release.ocid.as_deref().filter(|s| !s.trim().is_empty()) else {
        return Err(Rejected {
            reason: "missing-ocid",
            detail: "a release with no ocid cannot key a Tender".to_owned(),
        });
    };

    let mut w = Walk {
        parsed: Parsed {
            sections: vec![Section { id: ROOT.into(), kind: "Notice".into(), parent: None }],
            values: Vec::new(),
        },
        ordinals: HashMap::new(),
    };
    // `language` governs every Text this notice publishes. FTS is English-only
    // in practice, but the value is read rather than assumed so a future
    // Welsh-language notice lands with its own tag.
    let lang = release.language.clone().unwrap_or_else(|| "en".to_owned());

    w.push(ROOT, "BT-04-notice", NoticeValue::Id { scheme: None, value: ocid.to_owned(), is_ref: false });
    w.push(ROOT, "BT-702(a)-notice", NoticeValue::Code { list: None, code: lang.clone() });
    if let Some(date) = release.date.as_deref() {
        w.push(ROOT, "OPP-012-notice", instant(date, "release date")?);
    }
    // The subtype decides the Tender kind. FTS publishes it as a UK form code
    // on exactly one document, which hangs off whichever branch the notice is
    // about; when no document carries one, the release `tag` is the only thing
    // left that says what this notice IS.
    let subtype = release.notice_type().unwrap_or_else(|| release.tag.join("+"));
    if !subtype.is_empty() {
        w.push(ROOT, "OPP-070-notice", NoticeValue::Code { list: None, code: subtype });
    }

    let tender = release.tender.unwrap_or_default();
    if let Some(basis) = &tender.legal_basis
        && let Some(id) = basis.id.as_deref()
    {
        w.push(ROOT, "BT-01-notice", NoticeValue::Code { list: basis.scheme.clone(), code: id.to_owned() });
    }
    if let Some(title) = tender.title.as_deref() {
        w.text(ROOT, "BT-21-Procedure", &lang, title);
    }
    // The release's own `description` is the fallback the OCDS profile allows
    // when the tender carries none.
    match tender.description.as_deref().or(release.description.as_deref()) {
        Some(d) => w.text(ROOT, "BT-24-Procedure", &lang, d),
        None => {}
    }
    w.money(ROOT, "BT-27-Procedure", tender.value.as_ref())?;
    if let Some(end) = tender.tender_period.as_ref().and_then(|p| p.end_date.as_deref()) {
        w.push(ROOT, "BT-131(d)-Procedure", instant(end, "tenderPeriod.endDate")?);
    }
    if let Some(end) = tender.enquiry_period.as_ref().and_then(|p| p.end_date.as_deref()) {
        w.push(ROOT, "BT-13(d)-Procedure", instant(end, "enquiryPeriod.endDate")?);
    }
    // `tender.classification` is the procedure-level CPV when the publisher
    // sets one; the items carry the rest.
    if let Some(c) = &tender.classification {
        w.classification(ROOT, "BT-262-Procedure", c);
    }
    // The procedure type (issue 465), off the publisher's own label through a
    // closed table — never off `procurementMethod`, whose `selective` is restricted,
    // competitive-with-negotiation and competitive dialogue alike. A label the
    // table does not know emits nothing: a miss costs coverage, never a wrong code.
    if let Some(code) = tender.procurement_method_details.as_deref().and_then(procedure_type) {
        w.push(
            ROOT,
            "BT-105-Procedure",
            NoticeValue::Code { list: Some("procurement-procedure-type".into()), code: code.into() },
        );
    }

    // Lots BEFORE items, so an item's `relatedLot` has a section to hang on.
    let inherited = inherited_periods(&release.awards, &release.contracts);
    for lot in &tender.lots {
        let Some(id) = lot.id.as_deref().filter(|s| !s.is_empty()) else { continue };
        w.section(id, "Lot", ROOT);
        if let Some(t) = lot.title.as_deref() {
            w.text(id, "BT-21-Lot", &lang, t);
        }
        if let Some(d) = lot.description.as_deref() {
            w.text(id, "BT-24-Lot", &lang, d);
        }
        w.money(id, "BT-27-Lot", lot.value.as_ref())?;
        // The lot's own period, else the one its award or that award's
        // contract publishes for it (issue 386 unit 2b; `inherited_periods`).
        let period = match &lot.contract_period {
            Some(p) => Some((p, "lot contractPeriod")),
            None => inherited.get(id).copied(),
        };
        if let Some((p, what)) = period {
            if let Some(s) = p.start_date.as_deref() {
                w.push(id, "BT-536-Lot", instant(s, &format!("{what}.startDate"))?);
            }
            if let Some(e) = p.end_date.as_deref() {
                w.push(id, "BT-537-Lot", instant(e, &format!("{what}.endDate"))?);
            }
        }
    }
    // The items: the tender's, then every non-delta award's (issue 437). A UK5/UK6/
    // UK7 award release publishes NO `tender.items` — its CPV and delivery region
    // sit on `awards[].items[]` and nowhere else — so a walk of the tender alone
    // served those releases with neither. The tender's come first and are emitted
    // as they always were; an award item states only what is new at its scope.
    //
    // The contract nature (issue 465) follows the same walk and the same rule. Some
    // award releases publish `mainProcurementCategory` on the award alone
    // (028961-2025, 029664-2025), so it is the tender's on the procedure, then each
    // award's on the one lot it names (a multi-lot or lot-less award's on the
    // procedure).
    let mut stated = HashSet::new();
    w.nature(tender.main_procurement_category.as_deref(), None, &mut stated);
    for item in &tender.items {
        w.item(item, None, &mut stated, false);
    }
    for award in release.awards.iter().filter(|a| !a.is_delta()) {
        // The one lot the award names, for an item that names none — the
        // narrowness `inherited_periods` applies: a multi-lot award's item is the
        // procedure's, not any one lot's.
        let only_lot = match award.related_lots.as_slice() {
            [lot] => Some(lot.as_str()),
            _ => None,
        };
        w.nature(award.main_procurement_category.as_deref(), only_lot, &mut stated);
        for item in &award.items {
            w.item(item, only_lot, &mut stated, true);
        }
    }

    // Parties, then the roles that point at them.
    for party in &release.parties {
        let Some(pid) = party.id.as_deref().filter(|s| !s.is_empty()) else { continue };
        let sid = format!("ORG-{pid}");
        w.section(&sid, "Organization", ROOT);
        if let Some(name) = party.name.as_deref() {
            w.text(&sid, "BT-500-Organization-Company", &lang, name);
        }
        // The country the register sits in. `address.country` is the alpha-2
        // when present; FTS omits it only on a handful of foreign suppliers,
        // and GB is the right default for a UK register.
        let country = party
            .address
            .as_ref()
            .and_then(|a| a.country.clone())
            .unwrap_or_else(|| "GB".to_owned());
        w.push(&sid, "BT-514-Organization-Company", NoticeValue::Code { list: None, code: country });
        // `<scheme>-<id>` is the form FTS itself uses for `party.id`, and the
        // form the crosswalk's GB arm reads (`GB-PPON-…`, `GB-COH-…`).
        for (n, ident) in party.identifier.iter().chain(party.additional_identifiers.iter()).enumerate() {
            let (Some(scheme), Some(id)) = (ident.scheme.as_deref(), ident.id.as_deref()) else {
                continue;
            };
            if id.is_empty() {
                continue;
            }
            let _ = n;
            w.push(
                &sid,
                "BT-501-Organization-Company",
                NoticeValue::Id {
                    scheme: Some(scheme.to_owned()),
                    value: format!("{scheme}-{id}"),
                    is_ref: false,
                },
            );
        }
        for role in &party.roles {
            let field = match role.as_str() {
                "buyer" | "procuringEntity" => "OPT-300-Procedure-Buyer",
                "centralPurchasingBody" => "OPT-300-Procedure-CPB",
                "reviewBody" => "OPT-301-Lot-ReviewOrg",
                "mediationBody" => "OPT-301-Lot-Mediator",
                // Suppliers and tenderers are reached through the results
                // graph below, never by a procedure-level role reference.
                _ => continue,
            };
            w.push(ROOT, field, NoticeValue::Id { scheme: None, value: sid.clone(), is_ref: true });
        }
    }

    // The results graph. A UK15 dynamic-market modification publishes dozens of
    // `{id, amendments}` skeletons that say nothing about who won what — those
    // emit nothing at all rather than a phantom empty result.
    //
    // Every result section opened here, with its lot, in the order it opened —
    // the bid statistics below hang under the first one for their lot.
    let mut results: Vec<(String, Option<String>)> = Vec::new();
    for award in &release.awards {
        let Some(aid) = award.id.as_deref().filter(|s| !s.is_empty()) else { continue };
        if award.is_delta() {
            continue;
        }
        // One LotResult per related lot, or one Tender-scoped result when the
        // award names none.
        let lots: Vec<Option<&str>> = if award.related_lots.is_empty() {
            vec![None]
        } else {
            award.related_lots.iter().map(|l| Some(l.as_str())).collect()
        };
        for lot in lots {
            let rid = match lot {
                Some(l) => format!("RES-{aid}-{l}"),
                None => format!("RES-{aid}"),
            };
            w.section(&rid, "LotResult", ROOT);
            results.push((rid.clone(), lot.map(str::to_owned)));
            // The contracts this award settled (issue 386 unit 2b): OPT-315 on
            // the result names each `CON-<id>` whose `awardID` is this award, so
            // the fold reaches a contract from its result — and, through the
            // contract's BT-3202 below, the result's bids from the contract. The
            // same refs sit on every lot-result of a multi-lot award: OCDS links
            // contracts to awards, never to lots.
            for contract in release.contracts.iter().filter(|c| c.award_id.as_deref() == Some(aid)) {
                if let Some(cid) = contract.id.as_deref().filter(|s| !s.is_empty()) {
                    w.push(
                        &rid,
                        "OPT-315-LotResult",
                        NoticeValue::Id { scheme: None, value: format!("CON-{cid}"), is_ref: true },
                    );
                }
            }
            if let Some(status) = award.status.as_deref() {
                // `selec-w` is "a winner was chosen"; `clos-nw` is "closed with
                // none". Anything else the publisher invents is left unmapped
                // rather than guessed into one of the two.
                let code = match status {
                    "active" | "pending" => Some("selec-w"),
                    "unsuccessful" | "cancelled" => Some("clos-nw"),
                    _ => None,
                };
                if let Some(code) = code {
                    w.push(&rid, "BT-142-LotResult", NoticeValue::Code { list: None, code: code.into() });
                }
            }
            if let Some(l) = lot {
                w.push(&rid, "BT-13713-LotResult", NoticeValue::Id { scheme: None, value: l.to_owned(), is_ref: false });
            }
            // When the buyer decided. eForms scopes BT-1451 to the settled
            // contract, and it is emitted there too — but the UK6/UK5 shapes
            // publish an award with NO `contracts[]` at all, and inside the
            // contract loop is the only place this date used to be read, so for
            // those releases it was dropped whole. Issue 255 already settled where
            // a contract-less award date lives: on the result. Same BT, scoped
            // where this source publishes it (issue 386 unit 2).
            if let Some(date) = award.date.as_deref() {
                w.push(&rid, "BT-1451-LotResult", instant(date, "award date")?);
            }
            for (n, supplier) in award.suppliers.iter().enumerate() {
                let Some(sup_id) = supplier.id.as_deref().filter(|s| !s.is_empty()) else { continue };
                let ten = format!("TEN-{aid}-{n}");
                let tpa = format!("TPA-{aid}-{n}");
                w.section(&ten, "LotTender", ROOT);
                w.section(&tpa, "TenderingParty", ROOT);
                w.push(&rid, "OPT-320-LotResult", NoticeValue::Id { scheme: None, value: ten.clone(), is_ref: true });
                // The fold SUMS winning bids, so the award's value is carried
                // by the first supplier only: a two-supplier consortium award
                // is one amount, not two.
                if n == 0 {
                    w.money(&ten, "BT-720-Tender", award.value.as_ref())?;
                }
                if let Some(l) = lot {
                    w.push(&ten, "BT-13714-Tender", NoticeValue::Id { scheme: None, value: l.to_owned(), is_ref: false });
                }
                w.push(&ten, "OPT-310-Tender", NoticeValue::Id { scheme: None, value: tpa.clone(), is_ref: true });
                w.push(
                    &tpa,
                    "OPT-300-Tenderer",
                    NoticeValue::Id { scheme: None, value: format!("ORG-{sup_id}"), is_ref: true },
                );
            }
        }
    }

    for contract in &release.contracts {
        let Some(cid) = contract.id.as_deref().filter(|s| !s.is_empty()) else { continue };
        let sid = format!("CON-{cid}");
        w.section(&sid, "SettledContract", ROOT);
        w.push(&sid, "BT-150-Contract", NoticeValue::Id { scheme: None, value: cid.to_owned(), is_ref: false });
        if let Some(signed) = contract.date_signed.as_deref() {
            w.push(&sid, "BT-145-Contract", instant(signed, "contract dateSigned")?);
        }
        // The one field id here that is NOT an eForms one, and deliberately so:
        // eForms contracts carry no value of their own (their money is the value
        // of the Bid they settled, reached through BT-3202), so there is no BT to
        // borrow. OCDS publishes the contract's own amount, and dropping it —
        // which is what happened until now — served every FTS contract in the
        // corpus as `value: null` against a published figure. The projection
        // learns this one id and prefers a bid-derived total wherever one exists,
        // so no eForms notice changes shape (issue 386 unit 2).
        w.money(&sid, CONTRACT_VALUE, contract.value.as_ref())?;
        let award = release
            .awards
            .iter()
            .find(|a| a.id.as_deref().is_some_and(|id| !id.is_empty()) && a.id.as_deref() == contract.award_id.as_deref());
        if let Some(award) = award {
            if let Some(date) = award.date.as_deref() {
                w.push(&sid, "BT-1451-Contract", instant(date, "award date")?);
            }
            // The winning tenders this contract settled (issue 386 unit 2b):
            // BT-3202 names the award's `TEN-<award>-<n>` sections — the same
            // ids the awards loop minted, one per supplier WITH an id, so the
            // index `n` must be the enumerate index there, gaps included. A
            // delta award (`{id, amendments}`) minted nothing, so it gets no
            // reference either: a dangling ref is worse than an absent one.
            if !award.is_delta() {
                let aid = award.id.as_deref().unwrap_or_default();
                for (n, supplier) in award.suppliers.iter().enumerate() {
                    if supplier.id.as_deref().is_some_and(|s| !s.is_empty()) {
                        w.push(
                            &sid,
                            "BT-3202-Contract",
                            NoticeValue::Id { scheme: None, value: format!("TEN-{aid}-{n}"), is_ref: true },
                        );
                    }
                }
            }
        }
    }

    // The bid statistics (issue 342, the 2026-09-15 finding). Each one used to
    // open a `LotResult` section of its own, so the fold served every statistic
    // as a phantom award result with no decision: `STAT-<id>` rows in
    // `lot_results`, 128 of the 205 on tenders 7954610–7954620. A statistic
    // describes the result for its lot, as eForms' ReceivedSubmissionsStatistics
    // block does, so it now sits in a `ReceivedSubmissions` section under the
    // FIRST result for its `relatedLot`. A statistic with no `relatedLot` goes
    // under the lot-less result, and the fold's `enclosing()` walks up from
    // there. With no such result, the section stays at ROOT. The parsed layer
    // keeps it, no result encloses it, and the fold invents none.
    //
    // ONE BLOCK PER (lot, measure), whatever the publisher repeats. FTS publishes
    // a lot's statistics either once (029664-2025: five awards, one `bids`) or
    // once PER AWARD. When the repeats agree (083468-2026: two awards, `bids:2`
    // twice; 007621-2025: `bids:19` sixteen times), only the first-published
    // one hangs under the result, so the fold does not write the same row again
    // for every repeat. When they disagree (052408-2025: six lot-less awards
    // with `bids` 4,4,4,2,1,1), nothing says which award each belongs to, so the
    // whole group stays at ROOT. A wrong count on a result is worse than none,
    // and the fold would otherwise keep whichever row came last.
    if let Some(bids) = &release.bids {
        let key = |s: &Statistic| (s.related_lot.clone(), s.measure.clone());
        let mut groups: HashMap<(Option<String>, Option<String>), Vec<&Statistic>> = HashMap::new();
        for stat in bids.statistics.iter().filter(|s| s.id.as_deref().is_some_and(|id| !id.is_empty())) {
            groups.entry(key(stat)).or_default().push(stat);
        }
        for stat in &bids.statistics {
            let Some(sid) = stat.id.as_deref().filter(|s| !s.is_empty()) else { continue };
            let section = format!("STAT-{sid}");
            let group = &groups[&key(stat)];
            let agreed = group.iter().all(|s| s.same_figure(group[0]));
            let parent = if agreed && std::ptr::eq(group[0], stat) {
                results
                    .iter()
                    .find(|(_, lot)| lot.as_deref() == stat.related_lot.as_deref())
                    .map_or(ROOT, |(rid, _)| rid.as_str())
            } else {
                ROOT
            };
            w.section(&section, "ReceivedSubmissions", parent);
            match stat.measure.as_deref().and_then(measure) {
                Some(Measure::Count(code)) => {
                    if let Some(raw) = &stat.value
                        && let Some(n) = number(raw)
                    {
                        w.push(&section, "BT-759-LotResult", NoticeValue::Number { value: n, unit: None });
                    }
                    w.push(&section, "BT-760-LotResult", NoticeValue::Code { list: None, code: code.to_owned() });
                }
                // Money, not a count: the same exactness rule as every amount here,
                // and no currency means no amount, as in `Walk::money`.
                Some(Measure::Value(field)) => {
                    if let (Some(raw), Some(currency)) =
                        (&stat.value, stat.currency.as_deref().filter(|c| !c.is_empty()))
                    {
                        let cents = cents(raw)
                            .map_err(|detail| Rejected { reason: "unrepresentable-value", detail })?;
                        w.push(&section, field, NoticeValue::Amount { cents, currency: currency.to_owned() });
                    }
                }
                None => {}
            }
        }
    }

    Ok(w.parsed)
}

// ------------------------------------------------------------------ the walk

struct Walk {
    parsed: Parsed,
    ordinals: HashMap<(String, String), i64>,
}

impl Walk {
    fn has_section(&self, id: &str) -> bool {
        self.parsed.sections.iter().any(|s| s.id == id)
    }

    /// Open a section once. Section ids are unique per notice (the parsed
    /// layer's primary key), and FTS can name the same lot from several
    /// places, so a repeat is a no-op rather than a duplicate row.
    fn section(&mut self, id: &str, kind: &str, parent: &str) {
        if self.has_section(id) {
            return;
        }
        self.parsed.sections.push(Section {
            id: id.to_owned(),
            kind: kind.to_owned(),
            parent: Some(parent.to_owned()),
        });
    }

    fn push(&mut self, section: &str, field: &str, value: NoticeValue) {
        let ordinal = self.ordinals.entry((section.to_owned(), field.to_owned())).or_insert(-1);
        *ordinal += 1;
        self.parsed.values.push(ValueRow {
            section_id: section.to_owned(),
            field_id: field.to_owned(),
            ordinal: *ordinal,
            value,
        });
    }

    fn text(&mut self, section: &str, field: &str, lang: &str, value: &str) {
        if value.trim().is_empty() {
            return;
        }
        self.push(
            section,
            field,
            NoticeValue::Text { lang: Some(lang.to_owned()), value: value.to_owned() },
        );
    }

    fn classification(&mut self, section: &str, field: &str, c: &Classification) {
        let Some(code) = c.id.as_deref().filter(|s| !s.is_empty()) else { return };
        // FTS writes the scheme as `CPV`; the corpus stores it lowercase.
        let scheme = c.scheme.as_deref().unwrap_or("CPV").to_ascii_lowercase();
        self.push(section, field, NoticeValue::Classification { scheme, code: code.to_owned() });
    }

    /// The section a value about `lot` lands on, and the field-id suffix that goes
    /// with it: the lot when it has a section, else the procedure.
    fn scope(&self, lot: Option<&str>) -> (String, &'static str) {
        match lot.filter(|l| self.has_section(l)) {
            Some(lot) => (lot.to_owned(), "Lot"),
            None => (ROOT.to_owned(), "Procedure"),
        }
    }

    /// An OCDS `mainProcurementCategory` as the contract nature, BT-23 (issue
    /// 465), on `lot`'s scope. It is translated here, at the profile boundary,
    /// the way `awards[].status` becomes BT-142 codes, so the fold's
    /// `contract_nature` stays in eForms' one vocabulary. A nature already in
    /// `stated` at that scope is not stated again: issue 437's rule for award items.
    /// A different one is, though BT-23 is single-valued in eForms: the release
    /// published both for that scope, and keeping one would be a guess. Two items'
    /// main CPVs (BT-262) on one scope go the same way.
    fn nature(
        &mut self,
        category: Option<&str>,
        lot: Option<&str>,
        stated: &mut HashSet<(String, String, String)>,
    ) {
        let Some(code) = category.and_then(contract_nature) else { return };
        let (scope, suffix) = self.scope(lot);
        if !stated.insert((scope.clone(), "nature".to_owned(), code.to_owned())) {
            return;
        }
        self.push(
            &scope,
            &format!("BT-23-{suffix}"),
            NoticeValue::Code { list: Some("contract-nature".into()), code: code.into() },
        );
    }

    /// One item's classifications and delivery places, on its scope.
    ///
    /// The scope is the lot the item names when that lot has a section, else
    /// `award_lot` — the one lot an award item's award names — else the procedure.
    /// An item naming a lot is never moved to its award's: the item said which.
    /// The first classification is BT-262 and the rest BT-263, per item.
    ///
    /// `stated` holds every (scope, scheme, code) emitted so far. A tender item
    /// only records into it, so a release without award items walks exactly as it
    /// always did. An award item (`restating`) skips what is already there, in
    /// EITHER role: an award's items commonly restate the tender's, and the fold's
    /// fact set keys on the role, so a code the tender made additional and the
    /// award made main would otherwise be served twice.
    fn item(
        &mut self,
        item: &Item,
        award_lot: Option<&str>,
        stated: &mut HashSet<(String, String, String)>,
        restating: bool,
    ) {
        let (scope, suffix) = self.scope(item.related_lot.as_deref().or(award_lot));
        let cpvs = item.classification.iter().chain(item.additional_classifications.iter());
        for (n, c) in cpvs.enumerate() {
            let Some(code) = c.id.as_deref().filter(|s| !s.is_empty()) else { continue };
            let scheme = c.scheme.as_deref().unwrap_or("CPV").to_ascii_lowercase();
            if !stated.insert((scope.clone(), scheme, code.to_owned())) && restating {
                continue;
            }
            let field = if n == 0 { "BT-262" } else { "BT-263" };
            self.classification(&scope, &format!("{field}-{suffix}"), c);
        }
        for addr in &item.delivery_addresses {
            let Some(region) = addr.region.as_deref().filter(|r| !r.is_empty()) else { continue };
            if !stated.insert((scope.clone(), "nuts".to_owned(), region.to_owned())) && restating {
                continue;
            }
            self.push(
                &scope,
                &format!("BT-5071-{suffix}"),
                NoticeValue::Classification { scheme: "nuts".into(), code: region.to_owned() },
            );
        }
    }

    /// An OCDS `value` object → an `Amount` plus the tax basis it is quoted on.
    /// `amount` is net of VAT and `amountGross` includes it; a release that
    /// publishes only the gross figure still carries a real number, so it is
    /// taken and LABELLED rather than dropped.
    fn money(&mut self, section: &str, field: &str, value: Option<&Money>) -> Result<(), Rejected> {
        let Some(money) = value else { return Ok(()) };
        let Some(currency) = money.currency.as_deref().filter(|c| !c.is_empty()) else {
            return Ok(());
        };
        let (raw, basis) = match (&money.amount, &money.amount_gross) {
            (Some(net), _) => (net, "excl"),
            (None, Some(gross)) => (gross, "incl"),
            (None, None) => return Ok(()),
        };
        let cents = cents(raw).map_err(|detail| Rejected { reason: "unrepresentable-value", detail })?;
        self.push(section, field, NoticeValue::Amount { cents, currency: currency.to_owned() });
        self.push(
            section,
            "TED-VAL_TOTAL_TAX_BASIS",
            NoticeValue::Code { list: None, code: basis.to_owned() },
        );
        Ok(())
    }
}

// ------------------------------------------------------------------- values

/// An OCDS date-time (`2026-09-03T15:18:45+01:00`) → a stored instant.
///
/// [`eforms::timestamp`] takes the date and the clock as SEPARATE lexicals,
/// each carrying its own zone offset — the shape eForms publishes. OCDS writes
/// one combined string, so the offset is lifted off the tail and given to both
/// halves before the call.
fn instant(text: &str, what: &str) -> Result<NoticeValue, Rejected> {
    let bad = |detail: String| Rejected { reason: "unrepresentable-value", detail };
    let text = text.trim();
    // FTS publishes SOME values with no zone at all — `"2025-07-07"` as a
    // tenderPeriod end, measured on 13 of June 2025's 7,243 releases. That is a
    // whole day in UK civil time, not a defect, so the publisher's zone is
    // supplied here rather than the release being refused. Demanding an offset
    // on every value quarantined all 13.
    let (body, offset): (&str, String) = match split_offset(text) {
        Some((body, offset)) => (body, offset.to_owned()),
        None => (text, uk_zone(text).ok_or_else(|| bad(format!("{what}: not a date: {text:?}")))?),
    };
    let offset = offset.as_str();
    let value = match body.split_once('T') {
        Some((date, clock)) => {
            eforms::timestamp(&format!("{date}{offset}"), Some(&format!("{clock}{offset}")))
        }
        // A date with no clock is a whole day, and `has_time: false` is how the
        // corpus records that.
        None => eforms::timestamp(&format!("{body}{offset}"), None),
    };
    value.map_err(|e| bad(format!("{what}: {e}")))
}

/// The UK civil offset that applies on the date this string names, as an
/// ISO-8601 designator. FTS is a UK service publishing UK local dates, so a
/// zone-less value is read in the zone the publisher was standing in.
///
/// The offset is looked up at UTC midnight of the named day, which differs from
/// the true local midnight only inside the one-hour DST transition window — and
/// only for a value that names no clock time anyway.
fn uk_zone(text: &str) -> Option<String> {
    let date = text.split_once('T').map_or(text, |(d, _)| d);
    let mut parts = date.split('-');
    let y: u16 = parts.next()?.parse().ok()?;
    let m: u8 = parts.next()?.parse().ok()?;
    let d: u8 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let seconds = crate::fetch::days_from_civil(y, m, d) * 86_400;
    let minutes = super::uk_offset(seconds) / 60;
    Some(format!("+{:02}:{:02}", minutes / 60, minutes % 60))
}

/// Split a trailing zone designator off an ISO-8601 string.
fn split_offset(text: &str) -> Option<(&str, &str)> {
    if let Some(body) = text.strip_suffix(['Z', 'z']) {
        return Some((body, "Z"));
    }
    let (body, offset) = text.split_at(text.len().checked_sub(6)?);
    let bytes = offset.as_bytes();
    let shaped = matches!(bytes[0], b'+' | b'-')
        && bytes[3] == b':'
        && bytes[1..3].iter().chain(&bytes[4..6]).all(u8::is_ascii_digit);
    shaped.then_some((body, offset))
}

/// A JSON number → exact minor units.
///
/// The literal text is converted, never an `f64`: `32800.0` and `32800` are the
/// same amount and both appear in the same corpus, while an exponent form is
/// refused outright — `1e9999` is a real published value here, and no amount
/// written that way can be trusted to mean what it says.
fn cents(raw: &RawValue) -> Result<i64, String> {
    let text = raw.get().trim();
    if text.contains(['e', 'E']) {
        return Err(format!("exponent notation in an amount: {text}"));
    }
    eforms::cents(text)
}

/// A JSON number → `f64`, for the bid statistics, which are counts rather than
/// money and carry no exactness promise.
fn number(raw: &RawValue) -> Option<f64> {
    raw.get().trim().parse::<f64>().ok().filter(|n| n.is_finite())
}

/// What an OCDS `bids.statistics[].measure` is in eForms terms.
enum Measure {
    /// A count: BT-759 with this BT-760 received-submission-type code.
    Count(&'static str),
    /// A bid value: this field, as an amount.
    Value(&'static str),
}

/// The counts are the OCDS-for-eForms profile's received-submission-type table
/// read backwards
/// (standard.open-contracting.org/profiles/eforms/latest/en/codelists/received-submission-type/),
/// and the two value measures are that profile's BT-710/BT-711. The Procurement
/// Act's `finalStageBids`, `smeFinalStageBids` and `vcseFinalStageBids` have no
/// eForms code. Like any measure a publisher invents, they map to nothing
/// rather than being guessed into one (fts::checklist says so).
fn measure(name: &str) -> Option<Measure> {
    Some(match name {
        "bids" => Measure::Count("tenders"),
        "requests" => Measure::Count("part-req"),
        "electronicBids" => Measure::Count("t-esubm"),
        "smeBids" => Measure::Count("t-sme"),
        "microBids" => Measure::Count("t-micro"),
        "smallBids" => Measure::Count("t-small"),
        "mediumBids" => Measure::Count("t-med"),
        "foreignBidsFromEU" => Measure::Count("t-oth-eea"),
        "foreignBidsFromNonEU" => Measure::Count("t-no-eea"),
        "disqualifiedBids" => Measure::Count("t-verif-inad"),
        "tendersAbnormallyLow" => Measure::Count("t-verif-inad-low"),
        "lowestValidBidValue" => Measure::Value("BT-710-LotResult"),
        "highestValidBidValue" => Measure::Value("BT-711-LotResult"),
        _ => return None,
    })
}

/// OCDS's procurement-category codelist → eForms' contract-nature one (BT-23).
/// OCDS says `goods` where eForms says `supplies`; the other two are spelled
/// alike, and anything else is unmapped rather than guessed (issue 465).
fn contract_nature(category: &str) -> Option<&'static str> {
    Some(match category.trim() {
        "goods" => "supplies",
        "works" => "works",
        "services" => "services",
        _ => return None,
    })
}

/// `tender.procurementMethodDetails` → eForms' procurement-procedure-type code
/// (BT-105, issue 465). A closed table of the labels FTS publishes, measured on
/// the fixtures, the 2026-09-03 recorded pages and the public API's 2023-06-01
/// page. Its codes are the EU-era procedures and their Procurement Act namesake,
/// the open procedure. `neg-w-call` is both the utilities' negotiated procedure
/// with a call and the public sector's competitive procedure with negotiation: on
/// notice type 16, a 2014/24 contract notice, the SDK holds `neg-w-call` to that
/// procedure's minimum of three candidates. `Negotiated without publication of a
/// contract notice` (published under 2009/81, 2014/24 and 2014/25) is a
/// negotiated procedure without a call, like the PCR's own label for it.
///
/// No eForms code: the Procurement Act's `Competitive flexible procedure` and
/// `Direct award`, `Award under framework`, and the below-threshold routes
/// (`Below threshold - open competition`, `- limited competition`, `- without
/// competition`, `- award under framework`, `- unknown`). `oth-single` and
/// `oth-mult` would assert a stage count the label does not state, so these emit
/// nothing — the rule `awards[].status` and the final-stage bid measures follow.
fn procedure_type(details: &str) -> Option<&'static str> {
    Some(match details.trim() {
        "Open procedure" => "open",
        "Restricted procedure" => "restricted",
        "Negotiated procedure with prior call for competition" | "Competitive procedure with negotiation" => {
            "neg-w-call"
        }
        "Competitive dialogue" => "comp-dial",
        "Award procedure without prior publication of a call for competition"
        | "Negotiated without publication of a contract notice" => "neg-wo-call",
        _ => return None,
    })
}

// -------------------------------------------------------------------- shapes

#[derive(Deserialize, Default)]
struct Package {
    #[serde(default)]
    releases: Vec<Release>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Release {
    ocid: Option<String>,
    date: Option<String>,
    language: Option<String>,
    description: Option<String>,
    #[serde(default)]
    tag: Vec<String>,
    #[serde(default)]
    parties: Vec<Party>,
    tender: Option<Tender>,
    #[serde(default)]
    awards: Vec<Award>,
    #[serde(default)]
    contracts: Vec<Contract>,
    planning: Option<Planning>,
    bids: Option<Bids>,
}

impl Release {
    /// The UK form code (`UK4`, `UK6`, …). Exactly one document per release
    /// carries it, and which branch that document hangs off depends on the
    /// archetype — a tender notice puts it under `tender`, an award under the
    /// award, a market-engagement notice under `planning`, a contract change
    /// under the contract. All four are searched because none is canonical.
    fn notice_type(&self) -> Option<String> {
        let tender = self.tender.iter().flat_map(|t| t.documents.iter());
        let planning = self.planning.iter().flat_map(|p| p.documents.iter());
        let awards = self.awards.iter().flat_map(|a| a.documents.iter());
        let contracts = self.contracts.iter().flat_map(|c| c.documents.iter());
        tender
            .chain(planning)
            .chain(awards)
            .chain(contracts)
            .find_map(|d| d.notice_type.clone())
            .filter(|s| !s.is_empty())
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Tender {
    title: Option<String>,
    description: Option<String>,
    legal_basis: Option<LegalBasis>,
    value: Option<Money>,
    tender_period: Option<Period>,
    enquiry_period: Option<Period>,
    classification: Option<Classification>,
    /// `goods` / `works` / `services`: the contract nature, BT-23 (issue 465).
    main_procurement_category: Option<String>,
    /// The publisher's label for the procedure — the one place its type is
    /// stated precisely enough for BT-105 (issue 465; [`procedure_type`]).
    procurement_method_details: Option<String>,
    #[serde(default)]
    items: Vec<Item>,
    #[serde(default)]
    lots: Vec<Lot>,
    #[serde(default)]
    documents: Vec<Document>,
}

#[derive(Deserialize, Default)]
struct Planning {
    #[serde(default)]
    documents: Vec<Document>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Document {
    notice_type: Option<String>,
}

#[derive(Deserialize)]
struct LegalBasis {
    scheme: Option<String>,
    id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Money {
    amount: Option<Box<RawValue>>,
    amount_gross: Option<Box<RawValue>>,
    currency: Option<String>,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Period {
    start_date: Option<String>,
    end_date: Option<String>,
}

#[derive(Deserialize)]
struct Classification {
    scheme: Option<String>,
    id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    related_lot: Option<String>,
    classification: Option<Classification>,
    #[serde(default)]
    additional_classifications: Vec<Classification>,
    #[serde(default)]
    delivery_addresses: Vec<Address>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Lot {
    id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    value: Option<Money>,
    contract_period: Option<Period>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Party {
    id: Option<String>,
    name: Option<String>,
    #[serde(default)]
    roles: Vec<String>,
    identifier: Option<Identifier>,
    #[serde(default)]
    additional_identifiers: Vec<Identifier>,
    address: Option<Address>,
}

#[derive(Deserialize)]
struct Identifier {
    scheme: Option<String>,
    id: Option<String>,
}

#[derive(Deserialize)]
struct Address {
    country: Option<String>,
    region: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Award {
    id: Option<String>,
    status: Option<String>,
    date: Option<String>,
    value: Option<Money>,
    #[serde(default)]
    related_lots: Vec<String>,
    /// The contract's duration, published on the AWARD by a UK6 (083650-2026
    /// carries it here and nowhere else). Inherited by the one lot the award
    /// names — see [`inherited_periods`].
    contract_period: Option<Period>,
    #[serde(default)]
    suppliers: Vec<PartyRef>,
    #[serde(default)]
    documents: Vec<Document>,
    /// What was awarded — and, on a UK5/UK6/UK7, the ONLY place the release
    /// publishes its CPV and delivery region: 028961-2025 and 083650-2026 carry
    /// no `tender.items` and one item here (issue 437). Walked like the tender's.
    #[serde(default)]
    items: Vec<Item>,
    /// The contract nature, which 028961-2025 and 029664-2025 publish here and
    /// not on the tender (issue 465). Scoped like the award's items.
    main_procurement_category: Option<String>,
}

impl Award {
    /// A UK15 modification republishes every award as `{id, amendments}` and
    /// nothing else. Such an entry asserts no result — emitting a LotResult for
    /// it would invent an outcome the publisher did not state.
    fn is_delta(&self) -> bool {
        self.status.is_none() && self.value.is_none() && self.suppliers.is_empty()
    }
}

/// The period a lot inherits when it publishes none of its own (issue 386
/// unit 2b). FTS puts a contract's duration on the award (083650-2026, a UK6
/// whose lot carries no period) or on the contract (028961-2025), where eForms
/// puts it on the lot as BT-536/537-Lot — so without this every FTS lot served
/// `duration_start: null` while its notice published a duration.
///
/// The inheritance is narrow on purpose. Only an award that names exactly one
/// lot says anything about THAT lot's duration; a multi-lot award's period is
/// the award's, not any one lot's. The award's own `contractPeriod` beats its
/// contracts' `period`. And candidates that disagree — two single-lot awards
/// on the same lot, or two contracts of one award — leave the lot without a
/// period rather than pick one. A lot's own period is never overridden: the
/// walk consults this map only in its absence. The label beside each period
/// names its source for the quarantine detail when a date is unreadable.
fn inherited_periods<'a>(
    awards: &'a [Award],
    contracts: &'a [Contract],
) -> HashMap<&'a str, (&'a Period, &'static str)> {
    let mut candidates: HashMap<&str, Vec<(&Period, &'static str)>> = HashMap::new();
    for award in awards {
        let Some(aid) = award.id.as_deref().filter(|s| !s.is_empty()) else { continue };
        if award.is_delta() {
            continue;
        }
        let [lot] = award.related_lots.as_slice() else { continue };
        let found = match &award.contract_period {
            Some(p) => Some((p, "award contractPeriod")),
            None => agreed(
                contracts
                    .iter()
                    .filter(|c| c.award_id.as_deref() == Some(aid))
                    .filter_map(|c| c.period.as_ref()),
            )
            .map(|p| (p, "contract period")),
        };
        if let Some(found) = found {
            candidates.entry(lot.as_str()).or_default().push(found);
        }
    }
    candidates
        .into_iter()
        .filter_map(|(lot, found)| {
            agreed(found.iter().map(|(p, _)| *p)).map(|p| (lot, (p, found[0].1)))
        })
        .collect()
}

/// The one period a set of candidates agrees on, or none when they differ.
fn agreed<'a>(mut periods: impl Iterator<Item = &'a Period>) -> Option<&'a Period> {
    let first = periods.next()?;
    periods.all(|p| p == first).then_some(first)
}

#[derive(Deserialize)]
struct PartyRef {
    id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Contract {
    id: Option<String>,
    #[serde(rename = "awardID")]
    award_id: Option<String>,
    date_signed: Option<String>,
    /// What the contract is worth. Published on the CONTRACT, not on the award
    /// it settles: 028961-2025 carries `awards[0].value = null` beside
    /// `contracts[0].value = 54393.6 GBP` (issue 386 unit 2).
    value: Option<Money>,
    /// The contract's duration, when the publisher put it on the contract
    /// rather than the award (028961-2025). Inherited by the one lot the
    /// contract's award names — see [`inherited_periods`].
    period: Option<Period>,
    #[serde(default)]
    documents: Vec<Document>,
}

#[derive(Deserialize, Default)]
struct Bids {
    #[serde(default)]
    statistics: Vec<Statistic>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Statistic {
    id: Option<String>,
    measure: Option<String>,
    value: Option<Box<RawValue>>,
    /// The lot the statistic is about; its result is where the block hangs.
    related_lot: Option<String>,
    /// Set on the two value measures only.
    currency: Option<String>,
}

impl Statistic {
    /// The same figure: the same literal value in the same currency. Compared
    /// as published text, so `4` and `4.0` count as different. That is the
    /// cautious side, since a disagreeing group is never served.
    fn same_figure(&self, other: &Statistic) -> bool {
        let text = |s: &Statistic| s.value.as_ref().map(|v| v.get().trim().to_owned());
        text(self) == text(other) && self.currency == other.currency
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let path = format!("{}/tests/fixtures/fts/members/{name}.json", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
    }

    fn parsed(name: &str) -> Parsed {
        match parse(&fixture(name)) {
            Ok(p) => p,
            Err(Rejected { reason, detail }) => panic!("{name} quarantined as {reason}: {detail}"),
        }
    }

    fn one(p: &Parsed, section: &str, field: &str) -> Option<NoticeValue> {
        p.values
            .iter()
            .find(|v| v.section_id == section && v.field_id == field)
            .map(|v| v.value.clone())
    }

    fn all(p: &Parsed, field: &str) -> Vec<NoticeValue> {
        p.values.iter().filter(|v| v.field_id == field).map(|v| v.value.clone()).collect()
    }

    fn text_of(v: Option<NoticeValue>) -> Option<String> {
        match v {
            Some(NoticeValue::Text { value, .. }) => Some(value),
            _ => None,
        }
    }

    fn section_of<'a>(p: &'a Parsed, id: &str) -> &'a Section {
        p.sections.iter().find(|s| s.id == id).unwrap_or_else(|| panic!("no section {id}"))
    }

    /// Issue 342 (the 2026-09-15 finding): a statistic is a block under its lot's
    /// result, in eForms vocabulary, never a result of its own. 029615-2025 has
    /// counts and both value measures on lot 1. It also has a procedure-level
    /// lowest/highest pair that no result can take, which stays at ROOT.
    #[test]
    fn bid_statistics_hang_under_their_lots_result_in_eforms_vocabulary() {
        let p = parsed("029615-2025");
        let res = "RES-029615-2025-1-1";
        assert!(
            p.sections.iter().all(|s| s.kind != "LotResult" || !s.id.starts_with("STAT-")),
            "no statistic opens a result of its own"
        );
        for (stat, code, count) in [("STAT-5", "tenders", 6.0), ("STAT-6", "t-sme", 6.0), ("STAT-7", "t-esubm", 6.0)] {
            let s = section_of(&p, stat);
            assert_eq!((s.kind.as_str(), s.parent.as_deref()), ("ReceivedSubmissions", Some(res)), "{stat}");
            assert_eq!(
                one(&p, stat, "BT-760-LotResult"),
                Some(NoticeValue::Code { list: None, code: code.into() }),
                "{stat}"
            );
            assert_eq!(one(&p, stat, "BT-759-LotResult"), Some(NoticeValue::Number { value: count, unit: None }));
        }
        // Money, exactly, in the currency the statistic names — not a count.
        let gbp = |cents| Some(NoticeValue::Amount { cents, currency: "GBP".into() });
        assert_eq!(one(&p, "STAT-3", "BT-710-LotResult"), gbp(28_608_400));
        assert_eq!(one(&p, "STAT-4", "BT-711-LotResult"), gbp(85_825_200));
        assert_eq!(one(&p, "STAT-3", "BT-759-LotResult"), None, "a value is not a count");
        assert_eq!(section_of(&p, "STAT-3").parent.as_deref(), Some(res));
        // No relatedLot and no lot-less result: kept, at ROOT, where no result encloses it.
        assert_eq!(section_of(&p, "STAT-1").parent.as_deref(), Some(ROOT));
        assert_eq!(one(&p, "STAT-1", "BT-710-LotResult"), gbp(27_055_900));
        assert_eq!(one(&p, "STAT-2", "BT-711-LotResult"), gbp(85_825_200));
        // The bare pre-fix ids are gone.
        assert!(all(&p, "BT-759").is_empty() && all(&p, "BT-760").is_empty());
    }

    /// 029664-2025: five awards on ONE lot. The lot's counts are published once
    /// and hang under the first result, not five times over. The three Procurement
    /// Act final-stage measures have no eForms code and emit nothing.
    #[test]
    fn a_lot_with_five_awards_gets_its_statistics_once_and_unmapped_measures_emit_nothing() {
        let p = parsed("029664-2025");
        for stat in ["STAT-1", "STAT-2", "STAT-3", "STAT-4"] {
            assert_eq!(section_of(&p, stat).parent.as_deref(), Some("RES-1-1"), "{stat}");
        }
        assert_eq!(all(&p, "BT-760-LotResult"), vec![NoticeValue::Code { list: None, code: "tenders".into() }]);
        assert_eq!(all(&p, "BT-759-LotResult"), vec![NoticeValue::Number { value: 8.0, unit: None }]);
        for stat in ["STAT-2", "STAT-3", "STAT-4"] {
            assert!(p.values.iter().all(|v| v.section_id != stat), "{stat} is a final-stage measure: unmapped");
        }
    }

    /// Issue 342 review: FTS repeats a lot's statistics once per award. Repeats
    /// that AGREE (083468-2026, two awards on lot 1, `bids:2` twice) hang ONE
    /// block under the result, and the copies stay at ROOT.
    #[test]
    fn statistics_repeated_per_award_hang_once_when_they_agree() {
        let p = parsed("083468-2026");
        let res = "RES-083468-2026-1-1";
        let under = |id: &str| section_of(&p, id).parent.clone();
        assert_eq!(under("STAT-69").as_deref(), Some(res), "the first `bids` is the lot's");
        assert_eq!(under("STAT-72").as_deref(), Some(ROOT), "its identical repeat is not");
        assert_eq!(under("STAT-67").as_deref(), Some(res));
        assert_eq!(under("STAT-70").as_deref(), Some(ROOT));
        let enclosed: Vec<&str> = p
            .sections
            .iter()
            .filter(|s| s.kind == "ReceivedSubmissions" && s.parent.as_deref() != Some(ROOT))
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(enclosed, vec!["STAT-69", "STAT-67", "STAT-68"], "one block per measure on lot 1");
    }

    /// And repeats that DISAGREE are not attributed at all: 052408-2025 has six
    /// lot-less awards and `bids` 4,4,4,2,1,1, with nothing saying which award
    /// each count belongs to. Every one stays at ROOT, parsed but unserved.
    #[test]
    fn statistics_repeated_per_award_that_disagree_are_attributed_to_no_result() {
        let p = parsed("052408-2025");
        let stats: Vec<&Section> = p.sections.iter().filter(|s| s.kind == "ReceivedSubmissions").collect();
        assert_eq!(stats.len(), 12, "every statistic is kept in the parsed layer");
        assert!(stats.iter().all(|s| s.parent.as_deref() == Some(ROOT)), "none is guessed onto a result");
        assert_eq!(all(&p, "BT-759-LotResult").len(), 12, "the counts themselves are not dropped");
    }

    /// Every fixture parses. This is the cheap standing guard: the UK extension
    /// has moved four times in a year, and the first symptom of the fifth move
    /// is a fixture that stops parsing.
    #[test]
    fn every_fts_fixture_parses() {
        for name in
            ["083563-2026", "083645-2026", "083650-2026", "083685-2026", "_noid-2026-09-03-p001-000",
             "029615-2025", "029664-2025", "052408-2025", "083468-2026"]
        {
            let p = parsed(name);
            assert!(
                matches!(one(&p, ROOT, "BT-04-notice"), Some(NoticeValue::Id { .. })),
                "{name} must key on its ocid"
            );
        }
    }

    /// UK4, an open tender: the notice's own identity, its title and deadline,
    /// its buyer, and five lots that keep their own titles and values.
    #[test]
    fn uk4_tender_maps_title_deadline_buyer_and_lots() {
        let p = parsed("083563-2026");
        assert_eq!(
            one(&p, ROOT, "BT-04-notice"),
            Some(NoticeValue::Id {
                scheme: None,
                value: "ocds-h6vhtk-06f17d".into(),
                is_ref: false
            })
        );
        // The subtype is the UK form code off the one document that carries it
        // — the other two documents in this release have none.
        assert_eq!(
            one(&p, ROOT, "OPP-070-notice"),
            Some(NoticeValue::Code { list: None, code: "UK4".into() })
        );
        assert_eq!(
            text_of(one(&p, ROOT, "BT-21-Procedure")).as_deref(),
            Some("Science and Industry Museum Wonderlab Group 2 Interactives Makers")
        );
        // The submission deadline, with its published +01:00 offset intact.
        let Some(NoticeValue::Date { offset_minutes, has_time, .. }) =
            one(&p, ROOT, "BT-131(d)-Procedure")
        else {
            panic!("the tenderPeriod end must land as a Date")
        };
        assert_eq!((offset_minutes, has_time), (60, true));
        // Procedure value: net of VAT, so the tax basis says so.
        assert_eq!(
            one(&p, ROOT, "BT-27-Procedure"),
            Some(NoticeValue::Amount { cents: 18_250_000, currency: "GBP".into() })
        );
        assert_eq!(
            one(&p, ROOT, "TED-VAL_TOTAL_TAX_BASIS"),
            Some(NoticeValue::Code { list: None, code: "excl".into() })
        );
        // Five lots, each its own section under the procedure.
        let lots: Vec<&Section> = p.sections.iter().filter(|s| s.kind == "Lot").collect();
        assert_eq!(lots.len(), 5);
        assert!(lots.iter().all(|s| s.parent.as_deref() == Some(ROOT)));
        assert_eq!(text_of(one(&p, "1", "BT-21-Lot")).as_deref(), Some("Lot 1"));
        assert_eq!(
            one(&p, "1", "BT-27-Lot"),
            Some(NoticeValue::Amount { cents: 5_000_000, currency: "GBP".into() })
        );
        // The buyer: one Organization section, and a role reference to it.
        let org = "ORG-GB-PPON-PDTR-3338-MNPG";
        assert!(p.sections.iter().any(|s| s.id == org && s.kind == "Organization"));
        assert_eq!(
            one(&p, ROOT, "OPT-300-Procedure-Buyer"),
            Some(NoticeValue::Id { scheme: None, value: org.into(), is_ref: true })
        );
        assert_eq!(
            one(&p, org, "BT-501-Organization-Company"),
            Some(NoticeValue::Id {
                scheme: Some("GB-PPON".into()),
                value: "GB-PPON-PDTR-3338-MNPG".into(),
                is_ref: false,
            }),
            "the identifier keeps the scheme prefix the crosswalk's GB arm reads"
        );
        assert_eq!(
            one(&p, org, "BT-514-Organization-Company"),
            Some(NoticeValue::Code { list: None, code: "GB".into() })
        );
        // The items' CPVs land on their lots, not on the procedure.
        assert!(
            !all(&p, "BT-262-Lot").is_empty(),
            "each item names a relatedLot, so its CPV is lot-scoped"
        );
    }

    /// Issue 386 unit 2: the money OCDS puts on the contract, and the award date
    /// a contract-less release would otherwise lose.
    ///
    /// 028961-2025 is the shape that made every FTS contract serve `value: null`:
    /// `awards[0].value` is null and `contracts[0].value` is 54,393.60 GBP, so an
    /// award-scoped read finds nothing. 083650-2026 is the mirror — a UK6 award
    /// with no `contracts[]` at all, whose `date` used to be emitted only inside
    /// the contracts loop.
    #[test]
    fn a_contracts_own_value_and_a_contractless_awards_date_are_both_emitted() {
        let p = parsed("028961-2025");
        assert_eq!(
            one(&p, "CON-1", CONTRACT_VALUE),
            Some(NoticeValue::Amount { cents: 5_439_360, currency: "GBP".into() }),
            "54393.6 is a JSON float and still exact in minor units"
        );
        assert_eq!(
            one(&p, "CON-1", "BT-145-Contract"),
            Some(NoticeValue::Date { utc_seconds: 1_743_724_800, offset_minutes: 0, has_time: true }),
            "dateSigned 2025-04-04Z, unchanged"
        );
        assert!(
            all(&p, "BT-720-Tender").is_empty(),
            "the award publishes no value, so nothing may appear as a bid"
        );

        // The contract-less award: its date is on the RESULT, because there is no
        // settled contract for eForms' contract-scoped BT-1451 to hang on.
        let q = parsed("083650-2026");
        assert!(q.sections.iter().all(|s| s.kind != "SettledContract"), "no contracts[]");
        assert_eq!(
            one(&q, "RES-1-1", "BT-1451-LotResult"),
            Some(NoticeValue::Date {
                utc_seconds: 1_788_390_000,
                offset_minutes: 60,
                has_time: true
            }),
            "2026-09-03T00:00:00+01:00"
        );
    }

    /// UK6, a contract award: the winner, the amount the fold will sum, and
    /// the graph that ties result → tender → tendering party → organization.
    #[test]
    fn uk6_award_maps_winner_value_and_result_graph() {
        let p = parsed("083650-2026");
        assert_eq!(
            one(&p, ROOT, "OPP-070-notice"),
            Some(NoticeValue::Code { list: None, code: "UK6".into() })
        );
        // The award names lot "1", so the result is scoped to it.
        let res = "RES-1-1";
        assert!(p.sections.iter().any(|s| s.id == res && s.kind == "LotResult"));
        assert_eq!(
            one(&p, res, "BT-142-LotResult"),
            Some(NoticeValue::Code { list: None, code: "selec-w".into() }),
            "an active award selected a winner"
        );
        // The winning bid, net, on the first (here only) supplier.
        assert_eq!(
            one(&p, "TEN-1-0", "BT-720-Tender"),
            Some(NoticeValue::Amount { cents: 3_280_000, currency: "GBP".into() }),
            "32800.0 is a JSON float and still exact in minor units"
        );
        assert_eq!(
            one(&p, res, "OPT-320-LotResult"),
            Some(NoticeValue::Id { scheme: None, value: "TEN-1-0".into(), is_ref: true })
        );
        assert_eq!(
            one(&p, "TEN-1-0", "OPT-310-Tender"),
            Some(NoticeValue::Id { scheme: None, value: "TPA-1-0".into(), is_ref: true })
        );
        assert_eq!(
            one(&p, "TPA-1-0", "OPT-300-Tenderer"),
            Some(NoticeValue::Id {
                scheme: None,
                value: "ORG-GB-PPON-PYDR-3797-LZLJ".into(),
                is_ref: true
            }),
            "Oxford Brookes Enterprises, reached through the results graph"
        );
    }

    /// UK15 republishes 34 awards as `{id, amendments}`. They assert no
    /// outcome, so they must produce no result — the alternative is inventing
    /// 34 empty awards the publisher never made.
    #[test]
    fn delta_only_awards_emit_no_result() {
        let p = parsed("083685-2026");
        assert_eq!(
            one(&p, ROOT, "OPP-070-notice"),
            Some(NoticeValue::Code { list: None, code: "UK15".into() })
        );
        assert!(
            p.sections.iter().all(|s| s.kind != "LotResult"),
            "a delta award states no result and must emit none"
        );
        assert!(all(&p, "BT-720-Tender").is_empty(), "and no winning bid either");
    }

    /// A planning notice carries its form code under `planning.documents`, and
    /// its value has only `amountGross` — which is a real number and is taken,
    /// labelled as VAT-inclusive rather than silently mixed with net figures.
    #[test]
    fn uk2_planning_reads_its_own_document_branch_and_a_gross_only_value() {
        let p = parsed("083645-2026");
        assert_eq!(
            one(&p, ROOT, "OPP-070-notice"),
            Some(NoticeValue::Code { list: None, code: "UK2".into() })
        );
        assert_eq!(
            one(&p, ROOT, "BT-27-Procedure"),
            Some(NoticeValue::Amount { cents: 0, currency: "GBP".into() })
        );
        assert_eq!(
            one(&p, ROOT, "TED-VAL_TOTAL_TAX_BASIS"),
            Some(NoticeValue::Code { list: None, code: "incl".into() })
        );
        // Its single lot publishes `"description": null` — an explicit null,
        // not an omission, and the shape that would panic a non-optional read.
        assert!(p.sections.iter().any(|s| s.id == "LOT-0000" && s.kind == "Lot"));
        assert_eq!(text_of(one(&p, "LOT-0000", "BT-24-Lot")), None);
    }

    /// A contract-change notice keys and parses even though the release has no
    /// `id` of its own — `ocid` is what a Tender is keyed on, and it is there.
    #[test]
    fn a_release_without_its_own_id_still_keys_on_the_ocid() {
        let p = parsed("_noid-2026-09-03-p001-000");
        assert_eq!(
            one(&p, ROOT, "BT-04-notice"),
            Some(NoticeValue::Id {
                scheme: None,
                value: "ocds-h6vhtk-065444".into(),
                is_ref: false
            })
        );
        assert_eq!(
            one(&p, ROOT, "OPP-070-notice"),
            Some(NoticeValue::Code { list: None, code: "UK10".into() })
        );
        assert!(p.sections.iter().any(|s| s.id == "CON-1" && s.kind == "SettledContract"));
    }

    /// Issue 386 unit 2b: the results graph is linked both ways. The result
    /// names the contracts its award settled (OPT-315 → `CON-<id>`), and the
    /// contract names the winning tenders it settled (BT-3202 → `TEN-<award>-<n>`),
    /// which is how the fold reaches a contract's bids — and its lot — from
    /// either end. An award with no contracts carries no OPT-315; a contract
    /// whose award is not in the release (an amendment skeleton) carries no
    /// BT-3202, rather than a reference to a section nobody minted.
    #[test]
    fn a_contract_and_its_award_reference_each_other_and_only_each_other() {
        let p = parsed("028961-2025");
        assert_eq!(
            one(&p, "RES-1-1", "OPT-315-LotResult"),
            Some(NoticeValue::Id { scheme: None, value: "CON-1".into(), is_ref: true }),
            "the award's result names the contract that settled it"
        );
        assert_eq!(
            one(&p, "CON-1", "BT-3202-Contract"),
            Some(NoticeValue::Id { scheme: None, value: "TEN-1-0".into(), is_ref: true }),
            "the contract names the winning tender it settled"
        );
        assert!(p.sections.iter().any(|s| s.id == "TEN-1-0"), "the referenced tender exists");

        // UK6 award, no contracts[] at all: the result stands alone.
        let q = parsed("083650-2026");
        assert_eq!(one(&q, "RES-1-1", "OPT-315-LotResult"), None);

        // A contract amendment with no awards[]: the contract has no bid to name.
        let r = parsed("_noid-2026-09-03-p001-000");
        assert!(r.sections.iter().any(|s| s.id == "CON-1"));
        assert_eq!(one(&r, "CON-1", "BT-3202-Contract"), None);
    }

    #[test]
    fn a_release_with_no_ocid_is_quarantined() {
        let payload = br#"{"version":"1.1","releases":[{"id":"x","date":"2026-01-01T00:00:00Z"}]}"#;
        let Err(Rejected { reason, .. }) = parse(payload) else {
            panic!("a release with no ocid must not key a Tender")
        };
        assert_eq!(reason, "missing-ocid");
    }

    /// The `1e9999` lesson, at the value layer this time: a number no decimal
    /// reader can hold quarantines the ONE release rather than being rounded.
    #[test]
    fn an_exponent_amount_quarantines_rather_than_rounding() {
        let payload = br#"{"version":"1.1","releases":[{"ocid":"ocds-x-1",
            "tender":{"value":{"amount":1e9999,"currency":"GBP"}}}]}"#;
        let Err(Rejected { reason, detail }) = parse(payload) else {
            panic!("an exponent amount must quarantine")
        };
        assert_eq!(reason, "unrepresentable-value");
        assert!(detail.contains("1e9999"), "the offending literal is named: {detail}");
    }

    #[test]
    fn a_combined_iso_instant_splits_into_the_pair_the_converter_wants() {
        // The offset lives on the tail of one combined string; both halves need
        // it, and the stored instant is the true UTC one.
        let Ok(NoticeValue::Date { utc_seconds, offset_minutes, has_time }) =
            instant("2026-09-03T15:18:45+01:00", "t").map_err(|e| e.detail)
        else {
            panic!("a combined instant must convert")
        };
        assert_eq!((offset_minutes, has_time), (60, true));
        // 14:18:45Z.
        assert_eq!(utc_seconds % 86_400, 14 * 3600 + 18 * 60 + 45);
        // A bare date is a whole day.
        let Ok(NoticeValue::Date { has_time, .. }) = instant("2026-09-03Z", "t").map_err(|e| e.detail)
        else {
            panic!("a date-only value must convert")
        };
        assert!(!has_time);
        // A zone-less value is the publisher's UK civil day, not an error: FTS
        // published `"2025-07-07"` as a tenderPeriod end on 13 of June 2025's
        // 7,243 releases, and demanding an offset quarantined every one of them.
        let Ok(NoticeValue::Date { offset_minutes, has_time, .. }) =
            instant("2025-07-07", "t").map_err(|e| e.detail)
        else {
            panic!("a zone-less date must convert")
        };
        assert_eq!((offset_minutes, has_time), (60, false), "July is BST");
        // January is GMT, so the same shape reads with no offset.
        let Ok(NoticeValue::Date { offset_minutes, .. }) =
            instant("2025-01-07", "t").map_err(|e| e.detail)
        else {
            panic!("a winter zone-less date must convert")
        };
        assert_eq!(offset_minutes, 0, "January is GMT");
        // A zone-less date-TIME gets the same treatment, and keeps its clock.
        let Ok(NoticeValue::Date { offset_minutes, has_time, .. }) =
            instant("2025-07-07T09:30:00", "t").map_err(|e| e.detail)
        else {
            panic!("a zone-less datetime must convert")
        };
        assert_eq!((offset_minutes, has_time), (60, true));
        // Still not a date at all: refused.
        assert!(instant("not-a-date", "t").is_err());
        assert!(instant("", "t").is_err());
    }

    /// The exact release that exposed the zone-less date — reconstructed from
    /// what the archive holds, so the regression has a named witness.
    #[test]
    fn the_release_that_published_a_zone_less_deadline_parses() {
        let payload = br#"{"version":"1.1","releases":[{"ocid":"ocds-h6vhtk-054e86",
            "id":"033064-2025","date":"2025-06-17T16:12:12+01:00","language":"en",
            "tag":["tender"],
            "tender":{"title":"A thing","value":{"amount":10000,"currency":"GBP"},
                      "tenderPeriod":{"endDate":"2025-07-07"}}}]}"#;
        let p = match parse(payload) {
            Ok(p) => p,
            Err(Rejected { reason, detail }) => panic!("quarantined as {reason}: {detail}"),
        };
        let Some(NoticeValue::Date { has_time, offset_minutes, .. }) =
            one(&p, ROOT, "BT-131(d)-Procedure")
        else {
            panic!("the deadline must land")
        };
        assert_eq!((has_time, offset_minutes), (false, 60));
    }

    /// Issue 386 unit 2b: FTS publishes a contract's duration on the award
    /// (083650-2026) or on the contract (028961-2025), never on the lot where
    /// eForms' BT-536/537-Lot live — so every FTS lot served a null duration.
    /// The one lot each award names inherits the period; a lot that publishes
    /// its own (083563-2026) keeps it.
    #[test]
    fn a_lot_without_a_period_inherits_its_single_lot_awards_or_that_awards_contracts() {
        // From the award's `contractPeriod` — a UK6 with no contracts[] at all.
        let p = parsed("083650-2026");
        assert_eq!(
            one(&p, "1", "BT-536-Lot"),
            Some(NoticeValue::Date { utc_seconds: 1_789_686_000, offset_minutes: 60, has_time: true }),
            "2026-09-18T00:00:00+01:00, awards[0].contractPeriod.startDate"
        );
        assert_eq!(
            one(&p, "1", "BT-537-Lot"),
            Some(NoticeValue::Date { utc_seconds: 1_806_533_999, offset_minutes: 60, has_time: true }),
            "2027-03-31T23:59:59+01:00"
        );
        // From the contract's `period`, the award publishing none.
        let q = parsed("028961-2025");
        assert_eq!(
            one(&q, "1", "BT-536-Lot"),
            Some(NoticeValue::Date { utc_seconds: 1_747_612_800, offset_minutes: 0, has_time: true }),
            "2025-05-19T00:00:00Z, contracts[0].period.startDate"
        );
        assert_eq!(
            one(&q, "1", "BT-537-Lot"),
            Some(NoticeValue::Date { utc_seconds: 1_842_307_199, offset_minutes: 0, has_time: true }),
            "2028-05-18T23:59:59Z"
        );
        // A lot with its own period keeps it.
        let r = parsed("083563-2026");
        assert_eq!(
            one(&r, "1", "BT-536-Lot"),
            Some(NoticeValue::Date { utc_seconds: 1_793_750_400, offset_minutes: 0, has_time: true }),
            "2026-11-04T00:00:00+00:00, tender.lots[0].contractPeriod.startDate"
        );
    }

    /// Issue 437: a UK6/UK7 award release publishes NO `tender.items` — its CPV and
    /// delivery region sit on `awards[].items[]` and nowhere else, and the walk read
    /// only the tender's items, so the release served neither. 028961-2025 (UK7) and
    /// 083650-2026 (UK6) each carry one award item naming lot `1`.
    #[test]
    fn an_award_releases_items_carry_its_cpv_and_delivery_region() {
        let cpv = |code: &str| NoticeValue::Classification { scheme: "cpv".into(), code: code.into() };
        let nuts = |code: &str| NoticeValue::Classification { scheme: "nuts".into(), code: code.into() };

        let p = parsed("028961-2025");
        assert_eq!(one(&p, "1", "BT-262-Lot"), Some(cpv("48000000")), "Software package and information systems");
        assert_eq!(one(&p, "1", "BT-5071-Lot"), Some(nuts("UK")));
        let q = parsed("083650-2026");
        assert_eq!(one(&q, "1", "BT-262-Lot"), Some(cpv("80500000")), "Training services");
        assert_eq!(one(&q, "1", "BT-5071-Lot"), Some(nuts("UKK15")));
        // Exactly what the item says, on the lot it names: one code, one region.
        for r in [&p, &q] {
            assert_eq!(all(r, "BT-262-Lot").len(), 1);
            assert!(all(r, "BT-263-Lot").is_empty());
            assert_eq!(all(r, "BT-5071-Lot").len(), 1);
            assert!(all(r, "BT-262-Procedure").is_empty() && all(r, "BT-5071-Procedure").is_empty());
        }
    }

    /// Issue 437's scoping and restatement rules, on one synthetic release. An award
    /// item lands where a tender item would — its own `relatedLot` — and, naming
    /// none, on the one lot its award names (a multi-lot award's item is the
    /// procedure's; the `inherited_periods` narrowness). A code or region already
    /// stated at that scope is not stated again, in either role, so an award that
    /// restates the tender's items adds nothing. A delta award's items are nobody's.
    #[test]
    fn award_items_scope_like_tender_items_and_restate_nothing() {
        let payload = br#"{"version":"1.1","releases":[{"ocid":"ocds-x-3",
            "tender":{"lots":[{"id":"L1"},{"id":"L2"},{"id":"L3"}],
                      "items":[{"id":"1","relatedLot":"L1",
                                "additionalClassifications":[{"scheme":"CPV","id":"48000000"},{"scheme":"CPV","id":"72000000"}],
                                "deliveryAddresses":[{"region":"UKK15"}]}]},
            "awards":[
                {"id":"1","status":"active","relatedLots":["L1"],
                 "items":[{"id":"1","relatedLot":"L1",
                           "additionalClassifications":[{"scheme":"CPV","id":"48000000"},{"scheme":"CPV","id":"72000000"}],
                           "deliveryAddresses":[{"region":"UKK15"}]}]},
                {"id":"2","status":"active","relatedLots":["L1"],
                 "items":[{"id":"1","relatedLot":"L1",
                           "additionalClassifications":[{"scheme":"CPV","id":"72000000"},{"scheme":"CPV","id":"30200000"}]}]},
                {"id":"3","status":"active","relatedLots":["L2"],
                 "items":[{"id":"1","additionalClassifications":[{"scheme":"CPV","id":"80500000"}],
                           "deliveryAddresses":[{"region":"UKI5"}]}]},
                {"id":"4","status":"active","relatedLots":["L2","L3"],
                 "items":[{"id":"1","additionalClassifications":[{"scheme":"CPV","id":"45000000"}]}]},
                {"id":"5","items":[{"id":"1","relatedLot":"L3",
                                    "additionalClassifications":[{"scheme":"CPV","id":"90000000"}]}]}]}]}"#;
        let p = match parse(payload) {
            Ok(p) => p,
            Err(Rejected { reason, detail }) => panic!("quarantined as {reason}: {detail}"),
        };
        let codes = |section: &str, field: &str| -> Vec<String> {
            p.values
                .iter()
                .filter(|v| v.section_id == section && v.field_id == field)
                .map(|v| match &v.value {
                    NoticeValue::Classification { code, .. } => code.clone(),
                    other => panic!("{field} is not a classification: {other:?}"),
                })
                .collect()
        };
        // L1: the tender's two codes and region once each; award 2's new code joins
        // in the role its own item gives it, and its restated code is dropped.
        assert_eq!(codes("L1", "BT-262-Lot"), ["48000000"]);
        assert_eq!(codes("L1", "BT-263-Lot"), ["72000000", "30200000"]);
        assert_eq!(codes("L1", "BT-5071-Lot"), ["UKK15"]);
        // L2: award 3's item names no lot; its award names exactly L2.
        assert_eq!(codes("L2", "BT-262-Lot"), ["80500000"]);
        assert_eq!(codes("L2", "BT-5071-Lot"), ["UKI5"]);
        // Award 4 spans L2 and L3, so its lot-less item is procedure-wide.
        assert_eq!(codes(ROOT, "BT-262-Procedure"), ["45000000"]);
        assert!(codes("L3", "BT-262-Lot").is_empty(), "award 5 is a delta: its item says nothing");
    }

    /// Issue 465: `mainProcurementCategory` is the contract nature (BT-23), in
    /// eForms' vocabulary — OCDS `goods` is eForms `supplies` — so the fold's
    /// `contract_nature` reads FTS like any eForms notice. The tender's is the
    /// procedure's. Some award releases publish it only on the award (028961-2025,
    /// 029664-2025), and there it is the one lot's the award names.
    #[test]
    fn the_main_procurement_category_is_the_contract_nature_where_it_was_published() {
        let nature = |code: &str| NoticeValue::Code { list: Some("contract-nature".into()), code: code.into() };
        let p = parsed("052408-2025");
        assert_eq!(all(&p, "BT-23-Procedure"), vec![nature("supplies")], "the tender's `goods`");
        assert!(all(&p, "BT-23-Lot").is_empty(), "its six awards name no category");
        let q = parsed("028961-2025");
        assert_eq!(one(&q, "1", "BT-23-Lot"), Some(nature("supplies")), "the UK7's award, on lot 1");
        assert!(all(&q, "BT-23-Procedure").is_empty(), "the tender names none");
        // Five awards on lot 1, each saying `services`: one statement, not five.
        assert_eq!(all(&parsed("029664-2025"), "BT-23-Lot"), vec![nature("services")]);
        assert_eq!(all(&parsed("029615-2025"), "BT-23-Procedure"), vec![nature("services")]);
        assert_eq!(all(&parsed("083645-2026"), "BT-23-Procedure"), vec![nature("supplies")], "a UK2 planning notice");
        for name in ["083685-2026", "_noid-2026-09-03-p001-000"] {
            let p = parsed(name);
            assert!(all(&p, "BT-23-Procedure").is_empty() && all(&p, "BT-23-Lot").is_empty(), "{name} names no category");
        }
    }

    /// Issue 465's scoping, on one synthetic release, by issue 437's rule for award
    /// items: an award's nature lands on the one lot it names, a multi-lot or
    /// lot-less award's on the procedure, and a nature already stated at that scope
    /// is not stated again. A category outside OCDS's three is not guessed at.
    #[test]
    fn an_awards_nature_scopes_like_its_items_and_restates_nothing() {
        let payload = br#"{"version":"1.1","releases":[{"ocid":"ocds-x-4",
            "tender":{"mainProcurementCategory":"services","lots":[{"id":"L1"},{"id":"L2"},{"id":"L3"}]},
            "awards":[
                {"id":"1","status":"active","relatedLots":["L1"],"mainProcurementCategory":"goods"},
                {"id":"2","status":"active","relatedLots":["L1"],"mainProcurementCategory":"goods"},
                {"id":"3","status":"active","relatedLots":["L2","L3"],"mainProcurementCategory":"works"},
                {"id":"4","status":"active","mainProcurementCategory":"services"},
                {"id":"5","status":"active","relatedLots":["L3"],"mainProcurementCategory":"consultingServices"}]}]}"#;
        let p = match parse(payload) {
            Ok(p) => p,
            Err(Rejected { reason, detail }) => panic!("quarantined as {reason}: {detail}"),
        };
        let codes = |section: &str, field: &str| -> Vec<String> {
            p.values
                .iter()
                .filter(|v| v.section_id == section && v.field_id == field)
                .map(|v| match &v.value {
                    NoticeValue::Code { list, code } if list.as_deref() == Some("contract-nature") => code.clone(),
                    other => panic!("{field} is not a contract-nature code: {other:?}"),
                })
                .collect()
        };
        // The tender's `services`, then award 3's `works` (two lots: the procedure's);
        // lot-less award 4's `services` is already stated there.
        assert_eq!(codes(ROOT, "BT-23-Procedure"), ["services", "works"]);
        assert_eq!(codes("L1", "BT-23-Lot"), ["supplies"], "awards 1 and 2 say it once between them");
        assert!(codes("L2", "BT-23-Lot").is_empty());
        assert!(codes("L3", "BT-23-Lot").is_empty(), "`consultingServices` is no eForms nature");
    }

    /// Issue 465: the procedure type is read off `procurementMethodDetails` through
    /// a closed table. A label with no eForms code, or one the table does not know,
    /// emits nothing, and `procurementMethod` alone never decides it: `open` is also
    /// the first stage of a competitive flexible procedure.
    #[test]
    fn the_procedure_type_is_a_closed_table_of_the_published_labels() {
        for (label, code) in [
            ("Open procedure", "open"),
            ("Restricted procedure", "restricted"),
            ("Negotiated procedure with prior call for competition", "neg-w-call"),
            ("Competitive procedure with negotiation", "neg-w-call"),
            ("Competitive dialogue", "comp-dial"),
            ("Award procedure without prior publication of a call for competition", "neg-wo-call"),
            ("Negotiated without publication of a contract notice", "neg-wo-call"),
        ] {
            assert_eq!(procedure_type(label), Some(code), "{label}");
        }
        for label in [
            "Competitive flexible procedure",
            "Direct award",
            "Award under framework",
            "Below threshold - open competition",
            "Below threshold - without competition",
            "Below threshold - unknown",
        ] {
            assert_eq!(procedure_type(label), None, "{label} has no eForms code");
        }
        assert_eq!(procedure_type("Open procedure (accelerated)"), None, "an unknown label");

        let code = |p: &Parsed| all(p, "BT-105-Procedure");
        let typed = |c: &str| vec![NoticeValue::Code { list: Some("procurement-procedure-type".into()), code: c.into() }];
        assert_eq!(code(&parsed("052408-2025")), typed("neg-w-call"));
        assert_eq!(code(&parsed("083563-2026")), typed("open"));
        assert!(code(&parsed("028961-2025")).is_empty(), "Below threshold - without competition");
        assert!(code(&parsed("083645-2026")).is_empty(), "a planning notice states no procedure");
        let payload = br#"{"version":"1.1","releases":[{"ocid":"ocds-x-5",
            "tender":{"procurementMethod":"open","mainProcurementCategory":"works"}}]}"#;
        let Ok(p) = parse(payload) else { panic!("a bare method parses") };
        assert!(code(&p).is_empty(), "`procurementMethod` alone is no procedure type");
    }

    /// The inheritance's refusals, on one synthetic release: a lot's own period
    /// is never overridden, a multi-lot award says nothing about any one lot,
    /// two single-lot awards that disagree leave the lot bare, and an award and
    /// a contract that agree fill it.
    #[test]
    fn the_inherited_period_is_narrow() {
        let payload = br#"{"version":"1.1","releases":[{"ocid":"ocds-x-2",
            "tender":{"lots":[{"id":"own","contractPeriod":{"startDate":"2026-01-01Z"}},
                              {"id":"multi-a"},{"id":"multi-b"},{"id":"split"},{"id":"fed"}]},
            "awards":[
                {"id":"1","status":"active","relatedLots":["own"],"contractPeriod":{"startDate":"2030-01-01Z"}},
                {"id":"2","status":"active","relatedLots":["multi-a","multi-b"],"contractPeriod":{"startDate":"2030-01-01Z"}},
                {"id":"3","status":"active","relatedLots":["split"],"contractPeriod":{"startDate":"2030-01-01Z"}},
                {"id":"4","status":"active","relatedLots":["split"],"contractPeriod":{"startDate":"2031-01-01Z"}},
                {"id":"5","status":"active","relatedLots":["fed"],"contractPeriod":{"startDate":"2030-01-01Z"}},
                {"id":"6","status":"active","relatedLots":["fed"]}],
            "contracts":[{"id":"c6","awardID":"6","period":{"startDate":"2030-01-01Z"}}]}]}"#;
        let p = match parse(payload) {
            Ok(p) => p,
            Err(Rejected { reason, detail }) => panic!("quarantined as {reason}: {detail}"),
        };
        let start = |lot: &str| match one(&p, lot, "BT-536-Lot") {
            Some(NoticeValue::Date { utc_seconds, .. }) => Some(utc_seconds),
            _ => None,
        };
        assert_eq!(start("own"), Some(1_767_225_600), "the lot's own 2026-01-01, not the award's 2030");
        assert_eq!(start("multi-a"), None, "a two-lot award's period is nobody's");
        assert_eq!(start("multi-b"), None);
        assert_eq!(start("split"), None, "two awards on one lot disagree: nothing");
        assert_eq!(start("fed"), Some(1_893_456_000), "an award and a contract that agree fill the lot");
    }
}
