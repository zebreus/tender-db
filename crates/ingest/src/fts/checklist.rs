//! ADR-0004's per-profile mapped-or-ignored checklist for `fts:ocds-1.1` — the
//! thing that would have caught issue 386's dropped contract values before a
//! consumer did (unit 2b).
//!
//! The eForms profiles get this for free: the SDK's own field inventory is both
//! the mapping and the ignore rule, so an element with no branch is unclaimed
//! content and the notice quarantines (`eforms::index`). OCDS has no such
//! inventory to walk, `serde` skips unknown keys in silence, and so every field
//! the publisher adds — or every field the crosswalk never reached — was dropped
//! without a trace. This table is the inventory instead: every release path Find
//! a Tender publishes is either MAPPED, to the field id the walk emits it under,
//! or IGNORED, with the reason written down. The census test in `tests/fts.rs`
//! walks every fixture release and fails on a path that is neither, so a new
//! publisher field is a red test, not a silent drop.
//!
//! Paths are in census form — `awards[].value.amount` — and an entry covers its
//! subtree: `awards[].amendments` disposes of every key under it. A path with two
//! candidate entries takes the longest, so a leaf can be mapped inside an ignored
//! container and vice versa.

/// What the profile does with one published path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// Emitted by the walk — the field id(s) it lands under, or the structural
    /// role it plays (a section id, a scope pointer, the notice identity).
    Mapped(&'static str),
    /// Read and deliberately not stored, for this reason. An entry whose reason
    /// says "owed" is a mapping this profile does not have YET: listed so the
    /// gap is a decision on record, not an absence nobody can see.
    Ignored(&'static str),
}

use Disposition::{Ignored, Mapped};

/// The inventory. Order is irrelevant; [`disposition`] takes the longest match.
pub const CHECKLIST: &[(&str, Disposition)] = &[
    // ---------------------------------------------------------------- release
    ("ocid", Mapped("BT-04-notice — the procedure key (one Tender per ocid; issue 386 unit 1 splits a register ocid per buyer)")),
    ("id", Mapped("the notice identity — `publication_id`, read by the dispatcher (`fts::release_id`)")),
    ("date", Mapped("OPP-012-notice — the release instant")),
    ("language", Mapped("BT-702(a)-notice — governs every Text this notice publishes")),
    ("description", Mapped("BT-24-Procedure — the fallback when `tender.description` is absent")),
    ("tag", Mapped("OPP-070-notice — the fallback subtype when no document carries a UK form code")),
    ("initiationType", Ignored("always `tender` on FTS; carries no information")),
    ("buyer", Ignored("a reference to the party carrying the `buyer` role, which is read from `parties[]`")),
    ("buyerID", Ignored("the older spelling of `buyer.id`; the party is read from `parties[]`")),
    // ---------------------------------------------------------------- parties
    ("parties", Mapped("one Organization section per party — see the leaves")),
    ("parties[].id", Mapped("the Organization section id (`ORG-<id>`), which every role and supplier reference points at")),
    ("parties[].name", Mapped("BT-500-Organization-Company")),
    ("parties[].roles", Mapped("OPT-300-Procedure-Buyer — from both `buyer` and `procuringEntity`, so an agent acting for the buyer is served as a second buyer (issue 572) / OPT-300-Procedure-CPB / OPT-301-Lot-ReviewOrg / OPT-301-Lot-Mediator; suppliers and tenderers are reached through the results graph instead")),
    ("parties[].roleDetails", Ignored("owed: free-text 'on behalf of' prose beside a procuringEntity or buyer role; not read — the agent relation it states is not modelled yet (issue 572)")),
    ("parties[].identifier", Mapped("BT-501-Organization-Company as `<scheme>-<id>`, the form the crosswalk's GB arm reads")),
    ("parties[].identifier.legalName", Ignored("restates `parties[].name`")),
    ("parties[].identifier.noIdentifierRationale", Ignored("prose explaining a missing identifier; the absence itself is what the resolver sees")),
    ("parties[].additionalIdentifiers", Mapped("BT-501-Organization-Company, one per identifier, after the primary")),
    ("parties[].address.country", Mapped("BT-514-Organization-Company (GB when absent — a UK register)")),
    ("parties[].address.region", Ignored("the party's own region; the delivery place is read off the items (BT-5071)")),
    ("parties[].address.countryName", Ignored("the label of `address.country`")),
    ("parties[].address.locality", Ignored("owed: address lines (BT-513 city) are not folded for FTS; the org resolver binds on identifier and name")),
    ("parties[].address.postalCode", Ignored("owed: address lines (BT-512 postcode) are not folded for FTS")),
    ("parties[].address.streetAddress", Ignored("owed: address lines (BT-510 street) are not folded for FTS")),
    ("parties[].contactPoint", Ignored("owed: contact details (BT-502/503/506) are not folded for FTS; the eForms profile stores them as UBL- values")),
    ("parties[].details", Ignored("party metadata — scale, vcse, url — outside the canonical model; its classifications have an entry of their own")),
    ("parties[].details.classifications", Ignored("owed: the party's classifications, whatever the scheme — on a Procurement Act release UK_CA_TYPE (the buyer's contracting-authority type, the BT-11 analogue the eForms profile stores) and UK_CA_DEVOLVED_REGULATIONS; under the 2014 directives TED_CA_TYPE with COFOG, or TED_CE_ACTIVITY (the buyer's type and main activity); other schemes too, such as a qualification system's UVDB activity codes (issue 386); not folded for FTS (issue 572)")),
    // ----------------------------------------------------------------- tender
    ("tender", Mapped("the procedure block — see the leaves")),
    ("tender.id", Ignored("the release-local tender id; identity is the ocid")),
    ("tender.title", Mapped("BT-21-Procedure")),
    ("tender.description", Mapped("BT-24-Procedure")),
    ("tender.legalBasis", Mapped("BT-01-notice — `<scheme>` list, `<id>` code")),
    ("tender.legalBasis.uri", Ignored("the URL of the legal basis; the code carries it")),
    ("tender.value", Mapped("BT-27-Procedure — `amount`, or `amountGross` when the net is absent (342-fts-plan §3b item 5)")),
    ("tender.tenderPeriod", Mapped("BT-131(d)-Procedure — `endDate`; a start date is not published")),
    ("tender.enquiryPeriod", Mapped("BT-13(d)-Procedure — `endDate`")),
    ("tender.classification", Mapped("BT-262-Procedure — `scheme` and `id`")),
    ("tender.classification.description", Ignored("the code's label")),
    ("tender.items", Mapped("classifications and delivery places, scoped to the item's `relatedLot` or the procedure")),
    ("tender.items[].id", Ignored("the item id; items are not modelled, their classifications and places are")),
    ("tender.items[].relatedLot", Mapped("the scope of the item's classifications and delivery places")),
    ("tender.items[].additionalClassifications", Mapped("BT-262-{Procedure|Lot} for the first, BT-263-{Procedure|Lot} for the rest (342-fts-plan §3b item 3)")),
    ("tender.items[].additionalClassifications[].description", Ignored("the code's label")),
    ("tender.items[].deliveryAddresses", Mapped("BT-5071-{Procedure|Lot} from `region` (a NUTS code)")),
    ("tender.items[].deliveryAddresses[].country", Ignored("the region's country; the NUTS code implies it")),
    ("tender.items[].deliveryAddresses[].countryName", Ignored("the label of `country`")),
    ("tender.lots", Mapped("one Lot section per lot — see the leaves")),
    ("tender.lots[].id", Mapped("the Lot section id")),
    ("tender.lots[].title", Mapped("BT-21-Lot")),
    ("tender.lots[].description", Mapped("BT-24-Lot")),
    ("tender.lots[].value", Mapped("BT-27-Lot")),
    ("tender.lots[].contractPeriod", Mapped("BT-536-Lot / BT-537-Lot — start and end; a lot publishing none inherits its single-lot award's (issue 386 unit 2b)")),
    ("tender.lots[].contractPeriod.maxExtentDate", Ignored("owed: the maximum extension date has no destination (issue 386 unit 2b's schema question)")),
    ("tender.lots[].status", Ignored("the lot's OCDS status (`active`/`complete`); the Tender's status is derived from its notices")),
    ("tender.lots[].awardCriteria", Ignored("award criteria are not folded for any source's lots")),
    ("tender.lots[].suitability", Ignored("SME / VCSE suitability flags — outside the canonical model")),
    ("tender.lots[].hasOptions", Ignored("owed: options are not folded for FTS")),
    ("tender.lots[].options", Ignored("owed: options are not folded for FTS")),
    ("tender.lots[].hasRenewal", Ignored("owed: renewals (BT-58) are not folded for FTS")),
    ("tender.documents", Mapped("OPP-070-notice from the one document carrying `noticeType` — see the leaves")),
    ("tender.documents[].noticeType", Mapped("OPP-070-notice — the UK form code (UK1…UK15) that decides the Tender kind")),
    ("tender.documents[].id", Ignored("document metadata")),
    ("tender.documents[].description", Ignored("document metadata")),
    ("tender.documents[].documentType", Ignored("deliberately not keyed: `awardNotice` maps to both UK6 and UK15, `contractNotice` here means a change (342-fts-plan §3b item 4)")),
    ("tender.documents[].format", Ignored("document metadata")),
    ("tender.documents[].url", Ignored("document metadata")),
    ("tender.documents[].datePublished", Ignored("document metadata; the release `date` is the instant")),
    ("tender.aboveThreshold", Ignored("a UK-regime flag with no eForms counterpart")),
    ("tender.amendments", Ignored("amendment skeletons — a delta release restates nothing (plan D4)")),
    ("tender.awardPeriod", Ignored("owed: the award period has no destination")),
    ("tender.communication", Ignored("`futureNoticeDate` — a planning-notice promise, outside the model")),
    ("tender.competitive", Ignored("a UK-regime flag with no eForms counterpart")),
    ("tender.mainProcurementCategory", Mapped("BT-23-Procedure, list `contract-nature`: OCDS `goods` → eForms `supplies`, `works` and `services` as published, anything else unmapped rather than guessed (issue 465)")),
    ("tender.procurementMethod", Ignored("coarser than BT-105 and no code for it: `selective` is restricted, competitive-with-negotiation and competitive dialogue alike, and `open` also starts a competitive flexible procedure; the type is read off `procurementMethodDetails` (issue 465)")),
    ("tender.procurementMethodDetails", Mapped("BT-105-Procedure, list `procurement-procedure-type`, through the closed label table in `fts::parse::procedure_type`; the Procurement Act's `Open procedure` is eForms' `open`. The act's `Competitive flexible procedure` and `Direct award`, the below-threshold routes and `Award under framework` have no eForms code and emit nothing, rather than an `oth-single`/`oth-mult` guessing a stage count; so does a label the table does not know (issue 465)")),
    ("tender.procurementMethodRationale", Ignored("owed: the direct-award justification (BT-136) is not folded for FTS")),
    ("tender.procurementMethodRationaleClassifications", Ignored("owed: the direct-award justification codes (BT-136) are not folded for FTS")),
    ("tender.specialRegime", Ignored("a UK-regime flag with no eForms counterpart")),
    ("tender.status", Ignored("the OCDS tender status is not read: the Tender's `status` is derived from its submission deadline and its awards (issue 533: once its newest notice has published awards of its own, every award the procedure has published `cancelled`/`unsuccessful` and together covering every lot of kind `Lot` (lots groups and parts need no result; a Tender-scoped result, an award with no related lot, decides no lot, so it closes only a Tender with no `Lot`-kind lot), a UK12's included, closes it outside an open-ended scheme); a cancellation published only here is not seen (issue 533 unit 3)")),
    ("tender.submissionMethodDetails", Ignored("submission instructions, prose")),
    ("tender.submissionTerms", Ignored("submission terms (electronic policy, languages) — outside the canonical model")),
    // ----------------------------------------------------------------- awards
    ("awards", Mapped("one LotResult per related lot (or one Tender-scoped result) per non-delta award — see the leaves")),
    ("awards[].id", Mapped("the LotResult section id (`RES-<id>[-<lot>]`) and the LotTender/TenderingParty ids under it")),
    ("awards[].status", Mapped("BT-142-LotResult — `active`/`pending` → selec-w, `unsuccessful`/`cancelled` → clos-nw; anything else stays unmapped rather than guessed")),
    ("awards[].date", Mapped("BT-1451-LotResult, and BT-1451-Contract on the contract that names this award (issue 386 unit 2a)")),
    ("awards[].value", Mapped("BT-720-Tender on the first supplier's LotTender (the fold sums winning bids, so one amount per award)")),
    ("awards[].relatedLots", Mapped("BT-13713-LotResult / BT-13714-Tender — the lot each result belongs to")),
    ("awards[].suppliers", Mapped("OPT-320-LotResult → LotTender → OPT-310-Tender → TenderingParty → OPT-300-Tenderer, one chain per supplier")),
    ("awards[].suppliers[].name", Ignored("restates the party's name; the party is read from `parties[]`")),
    ("awards[].documents", Mapped("OPP-070-notice from the one document carrying `noticeType`")),
    ("awards[].documents[].noticeType", Mapped("OPP-070-notice")),
    ("awards[].documents[].id", Ignored("document metadata")),
    ("awards[].documents[].description", Ignored("document metadata")),
    ("awards[].documents[].documentType", Ignored("deliberately not keyed (342-fts-plan §3b item 4)")),
    ("awards[].documents[].format", Ignored("document metadata")),
    ("awards[].documents[].url", Ignored("document metadata")),
    ("awards[].documents[].datePublished", Ignored("document metadata")),
    ("awards[].amendments", Ignored("a delta award (`{id, amendments}` only) emits nothing at all, rather than a phantom empty result")),
    ("awards[].aboveThreshold", Ignored("a UK-regime flag with no eForms counterpart")),
    ("awards[].contractPeriod", Mapped("BT-536-Lot / BT-537-Lot on the one lot the award names, when that lot publishes no period of its own (issue 386 unit 2b; a multi-lot award's period is nobody's, and two awards that disagree fill nothing)")),
    ("awards[].contractPeriod.maxExtentDate", Ignored("owed: the maximum extension date has no destination (issue 386 unit 2b's schema question)")),
    ("awards[].finalStatusDate", Ignored("when the award's status became final — no destination")),
    ("awards[].hasOptions", Ignored("owed: options are not folded for FTS")),
    ("awards[].options", Ignored("owed: options are not folded for FTS")),
    // Issue 437: this was Ignored as "the award's items restate the tender's" — a
    // false premise. A UK5/UK6/UK7 publishes NO `tender.items` (028961-2025,
    // 083650-2026), so its CPV and region were on this path and nowhere else.
    ("awards[].items", Mapped("classifications and delivery places, walked like `tender.items` after them for every non-delta award; a code or region the release already stated at that scope is not stated again (issue 437)")),
    ("awards[].items[].id", Ignored("the item id; items are not modelled, their classifications and places are")),
    ("awards[].items[].relatedLot", Mapped("the scope of the item's classifications and delivery places; naming none, the one lot its award names (a multi-lot award's item is procedure-wide)")),
    ("awards[].items[].additionalClassifications", Mapped("BT-262-{Procedure|Lot} for the first, BT-263-{Procedure|Lot} for the rest, as on `tender.items`")),
    ("awards[].items[].additionalClassifications[].description", Ignored("the code's label")),
    ("awards[].items[].deliveryAddresses", Mapped("BT-5071-{Procedure|Lot} from `region` (a NUTS code)")),
    ("awards[].items[].deliveryAddresses[].country", Ignored("the region's country; the NUTS code implies it")),
    ("awards[].items[].deliveryAddresses[].countryName", Ignored("the label of `country`")),
    ("awards[].mainProcurementCategory", Mapped("BT-23-Lot on the one lot the award names, else BT-23-Procedure, translated as `tender.mainProcurementCategory`; a nature the release already stated at that scope is not stated again (issue 465, by issue 437's rule for award items)")),
    ("awards[].milestones", Ignored("award milestones — outside the canonical model")),
    ("awards[].title", Ignored("the award's own title — no destination; the lot and procedure titles are the served ones")),
    // -------------------------------------------------------------- contracts
    ("contracts", Mapped("one SettledContract per contract — see the leaves")),
    ("contracts[].id", Mapped("BT-150-Contract and the SettledContract section id (`CON-<id>`)")),
    ("contracts[].awardID", Mapped("BT-3202-Contract (the award's winning tenders, `TEN-<award>-<n>`) and OPT-315-LotResult on the award's results (`CON-<id>`) — the results graph linked both ways (issue 386 unit 2b) — and the award's date as BT-1451-Contract")),
    ("contracts[].dateSigned", Mapped("BT-145-Contract")),
    ("contracts[].value", Mapped("the contract's own value (`fts::parse::CONTRACT_VALUE`), preferred over a bid-derived total only when no bid carries one (issue 386 unit 2a)")),
    ("contracts[].documents", Mapped("OPP-070-notice from the one document carrying `noticeType`")),
    ("contracts[].documents[].noticeType", Mapped("OPP-070-notice")),
    ("contracts[].documents[].id", Ignored("document metadata")),
    ("contracts[].documents[].description", Ignored("document metadata")),
    ("contracts[].documents[].documentType", Ignored("deliberately not keyed (342-fts-plan §3b item 4)")),
    ("contracts[].documents[].format", Ignored("document metadata")),
    ("contracts[].documents[].url", Ignored("document metadata")),
    ("contracts[].documents[].datePublished", Ignored("document metadata")),
    ("contracts[].period", Mapped("BT-536-Lot / BT-537-Lot on the one lot the contract's award names, when neither the lot nor the award publishes one (issue 386 unit 2b)")),
    ("contracts[].period.maxExtentDate", Ignored("owed: the maximum extension date has no destination (issue 386 unit 2b's schema question)")),
    ("contracts[].status", Ignored("the contract's OCDS status — no destination")),
    ("contracts[].statusDetails", Ignored("prose beside `status`")),
    ("contracts[].title", Ignored("the contract's own title — no destination")),
    ("contracts[].aboveThreshold", Ignored("a UK-regime flag with no eForms counterpart")),
    ("contracts[].amendments", Ignored("amendment skeletons (plan D4)")),
    ("contracts[].hasRenewal", Ignored("owed: renewals are not folded for FTS")),
    // --------------------------------------------------------------- planning
    ("planning", Mapped("OPP-070-notice from the one document carrying `noticeType` — see the leaves")),
    ("planning.documents", Mapped("OPP-070-notice from the one document carrying `noticeType`")),
    ("planning.documents[].noticeType", Mapped("OPP-070-notice — UK1/UK2 planning notices carry the form code here")),
    ("planning.documents[].id", Ignored("document metadata")),
    ("planning.documents[].description", Ignored("document metadata")),
    ("planning.documents[].documentType", Ignored("deliberately not keyed (342-fts-plan §3b item 4)")),
    ("planning.documents[].format", Ignored("document metadata")),
    ("planning.documents[].url", Ignored("document metadata")),
    ("planning.documents[].datePublished", Ignored("document metadata")),
    ("planning.milestones", Ignored("planning milestones (a future notice's expected date) — outside the canonical model")),
    // ------------------------------------------------------------------- bids
    ("bids", Mapped("bid statistics — see the leaves")),
    ("bids.statistics", Mapped("one `ReceivedSubmissions` section per statistic (`STAT-<id>`), under the first LotResult for its lot — never a result of its own (issue 342)")),
    ("bids.statistics[].id", Mapped("the statistic's section id")),
    ("bids.statistics[].measure", Mapped("BT-760-LotResult by the OCDS-for-eForms received-submission-type table, or BT-710/BT-711-LotResult for the lowest/highest bid value; the Procurement Act's final-stage measures have no eForms code and emit nothing")),
    ("bids.statistics[].value", Mapped("BT-759-LotResult (a count), or the BT-710/BT-711 amount")),
    ("bids.statistics[].relatedLot", Mapped("which result the statistic hangs under (ROOT when none is for its lot)")),
    ("bids.statistics[].currency", Mapped("the BT-710/BT-711 amount's currency")),
];

/// The OCDS arrays Find a Tender was seen to publish as a map keyed by ordinal,
/// which the parser reads as the list they stand for (issue 478:
/// `"deliveryAddresses": {"1": {"region": "UK"}}`, `parse::list_or_ordinal_map`).
/// The census walks such a map as that list, so its leaves take the array
/// spelling's dispositions — `deliveryAddresses.1.countryName` is
/// `deliveryAddresses[].countryName`, ignored, not the container's mapped entry.
/// Keep it in step with the `deserialize_with` attributes in `parse.rs`.
pub const ORDINAL_MAPS: &[&str] = &["tender.items[].deliveryAddresses", "awards[].items[].deliveryAddresses"];

/// The disposition of one published path, by the longest entry that names it
/// or a container above it. `None` is the finding: a path this profile has never
/// decided about.
pub fn disposition(path: &str) -> Option<Disposition> {
    CHECKLIST
        .iter()
        .filter(|(key, _)| {
            path == *key
                || path
                    .strip_prefix(key)
                    .is_some_and(|rest| rest.starts_with('.') || rest.starts_with("[]"))
        })
        .max_by_key(|(key, _)| key.len())
        .map(|(_, d)| *d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_longest_entry_wins_and_containers_cover_their_subtrees() {
        assert_eq!(disposition("awards[].amendments[].id"), Some(Ignored("a delta award (`{id, amendments}` only) emits nothing at all, rather than a phantom empty result")));
        assert!(matches!(disposition("tender.documents[].noticeType"), Some(Mapped(_))));
        assert!(matches!(disposition("tender.documents[].url"), Some(Ignored(_))));
        assert!(matches!(disposition("tender.lots[].contractPeriod.startDate"), Some(Mapped(_))));
        assert!(matches!(disposition("tender.lots[].contractPeriod.maxExtentDate"), Some(Ignored(_))));
        assert!(matches!(disposition("contracts[].period.startDate"), Some(Mapped(_))));
        assert!(matches!(disposition("contracts[].period.maxExtentDate"), Some(Ignored(_))));
        // Issue 437: the award's items are read, leaf for leaf like the tender's.
        assert!(matches!(disposition("awards[].items[].additionalClassifications[].id"), Some(Mapped(_))));
        assert!(matches!(disposition("awards[].items[].deliveryAddresses[].region"), Some(Mapped(_))));
        assert!(matches!(disposition("awards[].items[].deliveryAddresses[].countryName"), Some(Ignored(_))));
        // A prefix that is not a path boundary does not match: `id` must not cover `identifier`,
        // and `tender.lots` must not cover `tender.lotsGroup` — the `tender` container does.
        assert_eq!(disposition("identifier"), None);
        assert_eq!(disposition("tender.lotsGroup"), disposition("tender"));
        assert_ne!(disposition("tender.lotsGroup"), disposition("tender.lots"));
        assert_eq!(disposition("somethingNew"), None);
        assert_eq!(disposition("tenderers"), None);
    }

    /// Issue 572 (U2b): FTS's free-text `roleDetails` ("on behalf of …") and the
    /// buyer's classifications are dropped on record, as owed, and the roles entry
    /// says that `procuringEntity` is served as Procedure-Buyer. No fixture publishes
    /// `roleDetails`, so the census in `tests/fts.rs` never reaches that entry: this
    /// is its pin.
    #[test]
    fn the_agent_relation_and_the_buyer_classifications_are_owed_on_record() {
        assert!(matches!(disposition("parties[].roleDetails"), Some(Ignored(r)) if r.starts_with("owed")));
        let Some(Mapped(roles)) = disposition("parties[].roles") else { panic!("parties[].roles is mapped") };
        assert!(roles.contains("procuringEntity"), "{roles}");
        // The longer path wins over `parties[].details`, leaf for leaf; the rest of
        // `details` keeps its "outside the canonical model" reason. The reason covers
        // every scheme that leaf can carry, not only the fixtures' (UK_CA_DEVOLVED_REGULATIONS
        // is in the 572 exemplars, UVDB on issue 386's utilities release).
        let schemes = ["UK_CA_TYPE", "UK_CA_DEVOLVED_REGULATIONS", "TED_CA_TYPE", "COFOG", "TED_CE_ACTIVITY", "UVDB"];
        for path in [
            "parties[].details.classifications",
            "parties[].details.classifications[].scheme",
            "parties[].details.classifications[].id",
        ] {
            assert!(
                matches!(disposition(path), Some(Ignored(r)) if r.starts_with("owed") && schemes.iter().all(|s| r.contains(s))),
                "{path}: {:?}",
                disposition(path)
            );
        }
        assert_ne!(disposition("parties[].details.classifications"), disposition("parties[].details"));
        assert_eq!(disposition("parties[].details.scale"), disposition("parties[].details"));
        // Every classification leaf now falls to its own entry, so the `details` reason
        // must not still claim them.
        let Some(Ignored(details)) = disposition("parties[].details") else { panic!("parties[].details is ignored") };
        assert!(!details.contains("UVDB") && !details.contains("activity classification"), "{details}");
    }

    /// Every ordinal-map field is a checklist entry of its own, mapped: the census
    /// folds its map into `[]` under it, so it must exist to be folded under.
    #[test]
    fn every_ordinal_map_is_a_mapped_entry() {
        for path in ORDINAL_MAPS {
            assert!(CHECKLIST.iter().any(|(key, d)| key == path && matches!(d, Mapped(_))), "{path}");
        }
    }
}
