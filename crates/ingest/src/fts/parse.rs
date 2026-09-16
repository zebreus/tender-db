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

use std::collections::HashMap;

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

    // Lots BEFORE items, so an item's `relatedLot` has a section to hang on.
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
        if let Some(p) = &lot.contract_period {
            if let Some(s) = p.start_date.as_deref() {
                w.push(id, "BT-536-Lot", instant(s, "lot contractPeriod.startDate")?);
            }
            if let Some(e) = p.end_date.as_deref() {
                w.push(id, "BT-537-Lot", instant(e, "lot contractPeriod.endDate")?);
            }
        }
    }
    for item in &tender.items {
        // An item's classifications and delivery place belong to its lot when
        // it names one; otherwise they are procedure-wide.
        let scope = item
            .related_lot
            .as_deref()
            .filter(|l| w.has_section(l))
            .unwrap_or(ROOT)
            .to_owned();
        let suffix = if scope == ROOT { "Procedure" } else { "Lot" };
        let mut cpvs = item.classification.iter().chain(item.additional_classifications.iter());
        if let Some(first) = cpvs.next() {
            w.classification(&scope, &format!("BT-262-{suffix}"), first);
        }
        for extra in cpvs {
            w.classification(&scope, &format!("BT-263-{suffix}"), extra);
        }
        for addr in &item.delivery_addresses {
            if let Some(region) = addr.region.as_deref().filter(|r| !r.is_empty()) {
                w.push(
                    &scope,
                    &format!("BT-5071-{suffix}"),
                    NoticeValue::Classification { scheme: "nuts".into(), code: region.to_owned() },
                );
            }
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
        if let Some(award) = release.awards.iter().find(|a| a.id.as_deref() == contract.award_id.as_deref())
            && let Some(date) = award.date.as_deref()
        {
            w.push(&sid, "BT-1451-Contract", instant(date, "award date")?);
        }
    }

    if let Some(bids) = &release.bids {
        for stat in &bids.statistics {
            let Some(sid) = stat.id.as_deref().filter(|s| !s.is_empty()) else { continue };
            let section = format!("STAT-{sid}");
            w.section(&section, "LotResult", ROOT);
            if let Some(raw) = &stat.value
                && let Some(n) = number(raw)
            {
                w.push(&section, "BT-759", NoticeValue::Number { value: n, unit: None });
            }
            if let Some(measure) = stat.measure.as_deref() {
                w.push(&section, "BT-760", NoticeValue::Code { list: None, code: measure.to_owned() });
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

#[derive(Deserialize)]
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
    #[serde(default)]
    suppliers: Vec<PartyRef>,
    #[serde(default)]
    documents: Vec<Document>,
}

impl Award {
    /// A UK15 modification republishes every award as `{id, amendments}` and
    /// nothing else. Such an entry asserts no result — emitting a LotResult for
    /// it would invent an outcome the publisher did not state.
    fn is_delta(&self) -> bool {
        self.status.is_none() && self.value.is_none() && self.suppliers.is_empty()
    }
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

    /// Every fixture parses. This is the cheap standing guard: the UK extension
    /// has moved four times in a year, and the first symptom of the fifth move
    /// is a fixture that stops parsing.
    #[test]
    fn every_fts_fixture_parses() {
        for name in
            ["083563-2026", "083645-2026", "083650-2026", "083685-2026", "_noid-2026-09-03-p001-000"]
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
}
