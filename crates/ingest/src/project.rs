//! Projection: the notice-parsed layer → the versioned canonical layer.
//!
//! Deterministic and rebuildable (ADR-0001): canonical state is a pure function
//! of the parsed notices plus the rules below, so it can always be thrown away
//! and re-derived. Nothing here reads XML — that happened in [`crate::process`].
//!
//! The rules, in the order they apply:
//!
//! 1. **Identity.** Notices sharing a procedure key (BT-04 `ContractFolderID`
//!    for TED eForms) are one Tender. A notice publishing no key becomes a
//!    single-notice *island* Tender rather than being dropped or guessed into
//!    someone else's procedure (CONTEXT.md); it upgrades by re-projection if
//!    linkage ever appears.
//! 2. **Order.** A Tender's notices are ordered by publication date (BT-05
//!    dispatch date where no publication date exists), then by publication id.
//!    Declared version numbers are advisory and are not used — real chains have
//!    gaps and cross-type sequences.
//! 3. **Supersession.** Each notice yields one version, resolved as *this
//!    notice's values over the previous version's*: a field the notice
//!    republishes replaces the earlier one wholesale (all languages of a title
//!    together, all CPV codes of one role together); a field it is silent about
//!    carries forward. That is what makes an award notice a complete Tender
//!    state rather than a fragment.
//! 4. **Lots.** Lots are resolved per version by the id the notice published.
//!    Two versions share a Lot exactly when they publish the same id — nothing
//!    is matched across versions by position, title or order, because
//!    framework/DPS call-off rounds relabel lots per round.
//! 5. **Organizations.** Every Organization section becomes a mention. Mentions
//!    merge into one canonical profile only on an exact normalised official
//!    identifier that passes the plausibility gate below; everything else is a
//!    provisional profile of that one mention.
//!
//! Change scoping is diff-based (ADR-0001 amendment) and lives in `store`,
//! which has both the old and the new version in hand.

use crate::r209::rules;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use store::{
    BidParty, BidState, ContractState, Db, Fact, Identifier, LotResultState, LotState, Mention,
    NoticeValue, Parsed, QUALITY_WITHHELD, Round, TenderProjection, TenderVersion,
};

/// What a projection run did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub notices: u64,
    pub tenders: u64,
    pub islands: u64,
    pub mentions: u64,
    /// Legacy Tenders retired because a late edge merged their members into
    /// another component (ADR-0003-style merge — their rows got removed events).
    pub absorbed: u64,
    pub applied: store::Applied,
    /// The run was asked to stop and ended at a checkpoint instead of finishing
    /// (issue 256). Everything counted above was really done and really committed;
    /// what the run did NOT do is everything after the checkpoint. A stopped
    /// rebuild leaves `rebuild_in_progress` set, so the next project job salvages
    /// it; a stopped incremental/full-fallback run leaves the layer intact and the
    /// unfolded notices still `projected = 0`, so the next run picks them up.
    pub stopped: bool,
    /// Issue 318: what the resolver's genericness wall did — (asked, denied,
    /// errored). It rides the durable Report rather than a log line because
    /// this runtime's stderr does not reach journald (issues 61/63), and
    /// because the batch arm's twin count is already durable in the
    /// r3-merge-plan report. `errored` non-zero means the wall was
    /// unavailable and binds went through at the pre-318 bar.
    pub wall: store::WallCounts,
    /// Issue 364: what the legacy previous-publication kind gate admitted and
    /// refused this run, per declared kind. Counted where the plan is built, so
    /// it covers every notice the run planned — a full run's whole corpus, an
    /// incremental run's delta plus its touched expansion.
    pub citations: CitationGate,
}

/// The canonical fields this layer carries, as data. Source field ids are
/// matched by their business-term stem (`BT-21-Lot`, `BT-21-Procedure` and
/// `BT-21-Part` are all the same canonical `title`), so a term keeps one
/// canonical name wherever the SDK mounts it.
///
/// The legacy profiles emit the source's own terms with a `TED-`/`TXT-` prefix
/// (docs/research/ted-legacy-mapping.md, r209/mod.rs), and those stems have no
/// context suffix, so the whole field id is the stem. The projection maps them
/// onto the same canonical fields as the eForms `BT-*` ids — one canonical
/// shape, earlier eras simply populating fewer columns (research §8.3). Only
/// the high-fill core is wired (title, values, CPV/NUTS, key dates, winners);
/// the ~23 no-eForms-equivalent legacy elements stay in the notice layer under
/// their prefixed ids, retrievable but not surfaced as canonical facts.
/// The Official Journal heading, used as a last-resort title — see the fallback in
/// `state_of`. Deliberately NOT in [`TEXTS`]: as a plain mapping it would add a
/// second, competing title to every legacy notice (and half the time that title
/// would be the publication reference).
const OJ_HEADING_FIELD: &str = "TED-TI_DOC";

const TEXTS: &[(&str, &str)] = &[
    ("BT-21", "title"),
    ("BT-24", "description"),
    // legacy R2.0.7–R2.0.9 (research §5.1 measured 100% title fill on its
    // window; the corpus held 29,763 titleless r208 tenders — issue 368)
    ("TED-TITLE", "title"),
    ("TED-TITLE_CONTRACT", "title"),
    ("TED-CONTRACT_TITLE", "title"),
    // Issue 368 unit 2: the legacy forms that name their subject in a
    // form-specific element rather than TITLE_CONTRACT — F07 qualification
    // system, F12 design contest, F13 result of a design contest, F08 notice on
    // a buyer profile. Half of the titleless r208 cohort carried one of these
    // and nothing read it. Each was read before being mapped (2026-09-12,
    // fixtures f07-185353-2013 / f12-185289-2013 / f13-187010-2013 /
    // f08-198630-2013): every one sits in the root PROCEDURE section and names
    // the procurement itself — "Sistema de Clasificación Proveedores Endesa
    // Local", "GLA Helicopter Services 2015" — and in a bounded band of 91
    // carriers none also published a TITLE_CONTRACT, so this adds no second,
    // competing title. (`TED-TI_TEXT` beside them stays out: it is the OJ
    // heading's CPV label in 23 languages, see the 2026-09-08 note on 368.)
    ("TED-TITLE_QUALIFICATION_SYSTEM", "title"),
    ("TED-TITLE_DESIGN_CONTACT_NOTICE", "title"),
    ("TED-TITLE_RESULT_DESIGN_CONTEST", "title"),
    ("TED-TITLE_NOTICE_BUYER_PROFILE", "title"),
    // Issue 368 unit 2, the lot half: the legacy Annex B lot's own title and
    // description. r208's lots were 100 %-null on title (lots 6,000,001–
    // 6,000,100: 100 of 100) because nothing read these, while the notice
    // layer held them all along; scope comes from the enclosing Lot section,
    // as for every text. The r208 probe put them at 1,816 and 3,244 rows in
    // the era's own head window (2026-09-12).
    ("TED-LOT_TITLE", "title"),
    ("TED-LOT_DESCRIPTION", "description"),
    ("TED-SHORT_DESCR", "description"),
    ("TED-SHORT_CONTRACT_DESCRIPTION", "description"),
    ("TED-SHORT_DESCRIPTION_CONTRACT", "description"),
    // an F14 corrigendum's new text is a prose version event on the Tender
    // (research §2.3: TEXT changes at minimum version the affected section).
    ("TED-NEW_VALUE.TEXT", "description"),
    // text era (1993–2010): TI title, TX/AB prose bodies
    ("TXT-TI", "title"),
    ("TXT-TX", "description"),
    ("TXT-AB", "description"),
    // DÖE sdk-0.1: ProcurementProject Name/Description, at Tender and Lot scope.
    // Keyed by full id — the stem (`SDK01-ProcurementProject`) cannot tell Name
    // from Description apart.
    ("SDK01-ProcurementProject-Name", "title"),
    ("SDK01-ProcurementProjectLot-ProcurementProject-Name", "title"),
    ("SDK01-ProcurementProject-Description", "description"),
    ("SDK01-ProcurementProjectLot-ProcurementProject-Description", "description"),
    // The grafted UBL-* inventory (issue 88): the tender-scoped prose facts with
    // unambiguous meaning map; everything else is in [`UBL_PARSE_ONLY`] with its
    // reason. Full-id keyed like SDK01-*; scope (Tender/Lot) comes from the
    // value's section, as for every fact.
    ("UBL-FundingProgram", "funding_program"),
    ("UBL-SelectionCriterionName", "selection_criterion"),
    ("UBL-TendererRequirementDescription", "tenderer_requirement"),
    ("UBL-AppealTermsDescription", "appeal_terms"),
];
const AMOUNTS: &[(&str, &str)] = &[
    ("BT-27", "estimated_value"),
    ("BT-271", "framework_maximum"),
    ("BT-161", "result_value"),
    // legacy
    ("TED-VAL_ESTIMATED_TOTAL", "estimated_value"),
    ("TED-VAL_TOTAL", "result_value"),
    // r208's plain `VALUE_COST` is deliberately NOT here: it is three facts in
    // one field id, routed by context in [`amount_target`] (issue 177). The
    // framework block's estimate (F02 II.1.4, `F02_FRAMEWORK/TOTAL_ESTIMATED`)
    // IS mapped: for a framework CN it is the notice's headline estimate, not a
    // restated copy — many carry no QUANTITY_SCOPE value at all (the committed
    // 2014 F02 is one).
    ("TED-TOTAL_ESTIMATED.VALUE_COST", "estimated_value"),
    // The grafted framework ceilings (issue 88): the UBL spelling of BT-271, at
    // whatever scope the section gives them — same fact the DE1 alias table
    // routes to BT-271. "Estimated maximum" is sdk-0.1-era phrasing of the same
    // ceiling.
    ("UBL-FrameworkMaximumAmount", "framework_maximum"),
    ("UBL-FrameworkEstimatedMaximumValue", "framework_maximum"),
    // DÖE sdk-0.1 (issue 231's value half). The era measured `value 0.0 %` over 666,671
    // versions and the issue's recorded diagnosis was that `notice_amounts` holds no rows
    // for it — so there would be nothing for a mapping to catch. That diagnosis is wrong:
    // the parse layer claims the era's money as `Amount` under `SDK01-*` ids, verified by
    // probing every committed DÖE fixture. It was a missing canonical destination, exactly
    // like the CPV half.
    //
    // Both spellings inside `cac:RequestedTenderTotal` map to the estimate, because the
    // draft-era publishers used them interchangeably: `doe-sdk01-ple-addinfo` carries
    // `EstimatedOverallContractAmount` and no `TotalAmount`, `doe-sdk01-subcontract`
    // carries `TotalAmount` and no `EstimatedOverallContractAmount`. Same container, same
    // fact — the DE-1.x alias table already routes its `EstimatedOverallContractAmount`
    // to BT-27, and this is that fact under the older element name.
    ("SDK01-ProcurementProject-RequestedTenderTotal-EstimatedOverallContractAmount", "estimated_value"),
    (
        "SDK01-ProcurementProjectLot-ProcurementProject-RequestedTenderTotal-EstimatedOverallContractAmount",
        "estimated_value",
    ),
    ("SDK01-ProcurementProject-RequestedTenderTotal-TotalAmount", "estimated_value"),
    // NOT mapped, deliberately, and both are recorded in issue 231:
    // `SDK01-TenderResult-AwardedTenderedProject-LegalMonetaryTotal-PayableAmount` is the
    // awarded value per tender — a results-graph fact (BT-720's shape), not a tender-scope
    // amount, and routing it here would file every award value as an estimate;
    // `SDK01-TenderResult-SubcontractTerms-Amount` is the subcontracted share, which has
    // no canonical home in any era yet.
];
const CLASSIFICATIONS: &[(&str, &str)] = &[
    ("BT-262", "main"),
    ("BT-263", "additional"),
    ("BT-5071", "place"),
    // legacy CPV (`@CODE` on CPV_MAIN/CPV_CODE/ORIGINAL_CPV) and NUTS
    ("TED-CPV_CODE", "main"),
    ("TED-ORIGINAL_CPV", "main"),
    ("TED-CURRENT_CPV", "main"),
    ("TED-CPV_ADDITIONAL", "additional"),
    ("TED-NUTS", "place"),
    ("TED-PERFORMANCE_NUTS", "place"),
    ("TED-ORIGINAL_NUTS", "place"),
    ("TED-CA_CE_NUTS", "place"),
    ("TED-CURRENT_NUTS", "place"),
    ("TED-TENDERER_NUTS", "place"),
    // text era
    ("TXT-PC", "main"),
    ("TXT-RC", "place"),
    ("TXT-CC", "main"),
    // DÖE sdk-0.1: the realized-location NUTS subentity, at Tender and Lot scope
    // (already carried as a `nuts`-scheme classification by the parser).
    ("SDK01-ProcurementProject-RealizedLocation-Address-CountrySubentityCode", "place"),
    ("SDK01-ProcurementProjectLot-ProcurementProject-RealizedLocation-Address-CountrySubentityCode", "place"),
    // DÖE sdk-0.1 CPV, at Tender and Lot scope, main and additional (issue 231).
    //
    // The report measured this era at 0.0 % CPV over 666,671 versions and the issue asked
    // the right question first — does the era publish CPV at all? Answered from prod: yes.
    // 175 sampled `can-standard` notices carry 1,328 `cpv`-scheme classification rows,
    // 7.6 apiece, under these four field ids. So the parse layer had them all along and
    // only the canonical destination was missing — the same shape as issue 177 one era
    // over, and the 0 % was a mapping gap rather than an absence.
    ("SDK01-ProcurementProject-MainCommodityClassification-ItemClassificationCode", "main"),
    ("SDK01-ProcurementProjectLot-ProcurementProject-MainCommodityClassification-ItemClassificationCode", "main"),
    (
        "SDK01-ProcurementProject-AdditionalCommodityClassification-ItemClassificationCode",
        "additional",
    ),
    (
        "SDK01-ProcurementProjectLot-ProcurementProject-AdditionalCommodityClassification-ItemClassificationCode",
        "additional",
    ),
];
/// The date/time pairs issue 03 stores as one instant, so `(d)` is the whole
/// deadline and there is no `(t)` row to reunite here.
const DATES: &[(&str, &str)] = &[
    ("BT-131(d)", "submission_deadline"),
    ("BT-1311(d)", "participation_deadline"),
    ("BT-132(d)", "opening_date"),
    ("BT-13(d)", "additional_information_deadline"),
    ("BT-536", "duration_start"),
    ("BT-537", "duration_end"),
    // legacy: the submission deadline (100% fill on F02) and its openings.
    // r209 names the form element DATE_RECEIPT_TENDERS; r208 called the same
    // IV.3.4 field RECEIPT_LIMIT_DATE — both map, or the whole 2011–2016 era
    // projects deadline-less (issue 174). The coded section's
    // DT_DATE_FOR_SUBMISSION stays unprojected in BOTH eras: the form value
    // is the published instant, the coded one a derived copy that can
    // disagree with it, and facts only dedupe when byte-identical.
    ("TED-DATE_RECEIPT_TENDERS", "submission_deadline"),
    ("TED-RECEIPT_LIMIT_DATE", "submission_deadline"),
    ("TED-DATE_OPENING_TENDERS", "opening_date"),
    ("TED-DATE_START", "duration_start"),
    ("TED-DATE_END", "duration_end"),
    // an F14 corrigendum's new deadline is the canonical delta ADR-0001's
    // motivating question reads ("how did the deadline move?"). Section-aware
    // mapping of every F14 WHERE target is deferred; the deadline is the
    // dominant, highest-value case (research §2.3: 210 DATE changes / package).
    ("TED-NEW_VALUE.DATE", "submission_deadline"),
    // text era deadline codes (DT/DD)
    ("TXT-DT", "submission_deadline"),
    ("TXT-DD", "submission_deadline"),
    // DÖE sdk-0.1: the lot's tender-submission deadline (an EndDate period).
    ("SDK01-ProcurementProjectLot-TenderingProcess-TenderSubmissionDeadlinePeriod-EndDate", "submission_deadline"),
];

/// The grafted `UBL-*` ids that stay PARSE-LAYER-ONLY, each with its reason
/// (issue 88). ADR-0004 allows two dispositions — mapped, or explicitly
/// ignored — and until this ledger existed the grafts were neither: captured by
/// the parser, then silently dropped by the fold. Every entry here is still
/// served verbatim by `/v1/notices/{id}/content`, so "ignored" means "not a
/// canonical fact", not "invisible". `ubl_grafts_are_all_mapped_or_ignored`
/// enforces the two-disposition rule: a new graft fails the gate until it is
/// mapped above or entered here with a reason. Revisit any entry when a
/// consumer asks for it — that is what the reason strings are for.
#[cfg(test)]
const UBL_PARSE_ONLY: &[(&str, &str)] = &[
    // -- no canonical channel for the value type (code/integer/number/plain id).
    ("UBL-AddressFormatCode", "code; org/address satellite, no code fact channel"),
    ("UBL-AwardCriterionParameterCode", "code; no code fact channel"),
    ("UBL-CompanyLegalFormCode", "code; org satellite, no code fact channel"),
    ("UBL-ContractExecutionPermissionCode", "code; no code fact channel"),
    ("UBL-ContractExecutionReservedCode", "code; no code fact channel"),
    ("UBL-ContractingPartyTypeCode", "code; org satellite, no code fact channel"),
    ("UBL-DocumentLanguageID", "code; document plumbing"),
    ("UBL-DocumentStatusCode", "code; document plumbing"),
    ("UBL-ProcurementAdditionalTypeCode", "code; no code fact channel"),
    ("UBL-SelectionCriterionParameterCode", "code; no code fact channel"),
    ("UBL-SelectionCriterionType", "code; no code fact channel"),
    ("UBL-SelectionCriterionUsage", "code; no code fact channel"),
    ("UBL-TenderResultCode", "code; results-layer state, no code fact channel"),
    ("UBL-TendererRequirementTypeCode", "code; no code fact channel"),
    ("UBL-AwardCriterionWeightNumeric", "number; no number fact channel"),
    ("UBL-FrameworkDurationMeasure", "number; no number fact channel"),
    ("UBL-ExpectedOperatorQuantity", "integer; no integer fact channel"),
    (
        "UBL-ReceivedTenderQuantity",
        "integer; the sdk-0.1 received-bids stat — belongs to the results binder's \
         statistics channel (LEGACY_BID_COUNT_FIELDS class), not a fact-table row",
    ),
    (
        "UBL-WinningPartyReference",
        "id; sdk-0.1 winner identity rides the WinningParty section mentions \
         (SDK01_WINNER_KIND) — the bare reference adds no edge the section lacks",
    ),
    ("UBL-ProcurementLegislationID", "id; legislation citation, provenance not content"),
    // -- org/contact/address satellite prose: the canonical Organization layer
    //    owns identity; contact-person PII deliberately stays out of canonical
    //    facts (the issue-173 posture).
    ("UBL-JobTitle", "contact-person PII; parse-layer only by posture"),
    ("UBL-PersonFirstName", "contact-person PII; parse-layer only by posture"),
    ("UBL-PersonFamilyName", "contact-person PII; parse-layer only by posture"),
    ("UBL-CommitteePersonFirstName", "contact-person PII; parse-layer only by posture"),
    ("UBL-ContactDepartment", "org contact satellite"),
    ("UBL-ContactID", "org contact plumbing"),
    ("UBL-Postbox", "org address satellite"),
    ("UBL-AddressDescription", "org address satellite"),
    ("UBL-CountryName", "org address satellite (country rides mentions already)"),
    ("UBL-CountrySubentity", "org address satellite (NUTS rides classifications)"),
    ("UBL-CompanyLegalForm", "org satellite prose"),
    ("UBL-CompanyLegalFormDescription", "org satellite prose"),
    // -- document plumbing / provenance.
    ("UBL-DocumentFileName", "document plumbing"),
    ("UBL-DocumentHash", "document plumbing"),
    ("UBL-AdditionalDocumentID", "document plumbing"),
    ("UBL-AdditionalDocumentURI", "document plumbing"),
    // -- semantics not settled; mapping wrongly is worse than parse-only.
    (
        "UBL-FrameworkDurationStart",
        "framework VALIDITY period ≠ contract duration_start (BT-536); needs its \
         own canonical names before mapping",
    ),
    (
        "UBL-FrameworkDurationEnd",
        "framework VALIDITY period ≠ contract duration_end (BT-537); see Start",
    ),
    ("UBL-FrameworkDurationDescription", "prose twin of the unsettled framework period"),
    ("UBL-FrameworkDurationDescriptionCode", "code twin of the unsettled framework period"),
    (
        "UBL-InvitationSubmissionDeadline",
        "invitation-to-tender deadline ≠ submission_deadline; conflating would \
         corrupt status open/closed",
    ),
    ("UBL-TenderValidityDeadline", "offer-validity end ≠ any mapped deadline"),
    ("UBL-TenderResultStartDate", "results-layer date; the results binder owns result facts"),
    (
        "UBL-LowerTenderAmount",
        "result statistic (lowest offer); the results binder owns result amounts — \
         a fact-table row would misfile it as a tender value",
    ),
    ("UBL-HigherTenderAmount", "result statistic (highest offer); see Lower"),
    ("UBL-AwardCriterionWeight", "prose twin of the weight numeric; neither has a channel"),
    ("UBL-CalculationExpression", "award-formula prose; too free-form for a named fact"),
    (
        "UBL-ContractExecutionDescription",
        "contract-execution prose; candidate canonical name pending demand",
    ),
    ("UBL-ProcessReason", "procedure-justification prose; candidate canonical name pending demand"),
    ("UBL-ProcessJustificationDescription", "procedure-justification prose; see ProcessReason"),
    ("UBL-ProcurementLegislationDescription", "legislation citation prose"),
    ("UBL-ProcurementTypeLabel", "free-text type label; kind rides the section machinery"),
    ("UBL-SubTypeDescription", "notice-subtype prose; subtype rides notice metadata"),
    // -- boolean/indicator codes: no indicator fact channel; the eForms BT
    //    equivalents (BT-743/-92/-93 etc.) are not canonical facts either, so
    //    mapping the UBL spellings first would invert completeness.
    ("UBL-ElectronicCatalogueUsage", "indicator code; no indicator channel (BT-764 class)"),
    ("UBL-ElectronicInvoiceAccepted", "indicator code; no indicator channel (BT-743 class)"),
    ("UBL-ElectronicInvoiceUsage", "indicator code; no indicator channel (BT-743 class)"),
    ("UBL-ElectronicOrderUsage", "indicator code; no indicator channel (BT-92 class)"),
    ("UBL-ElectronicPaymentUsage", "indicator code; no indicator channel (BT-93 class)"),
    ("UBL-RenewalsIndicator", "indicator; no indicator channel (BT-58 class)"),
    ("UBL-TerminatedIndicator", "indicator; results-layer state, no indicator channel"),
    // -- planned-period instants: which period (contract? framework? lot
    //    delivery?) depends on the mount; settle semantics before mapping, as
    //    with the framework validity pair above.
    ("UBL-PlannedPeriodStartTime", "planned-period semantics unsettled; see FrameworkDurationStart"),
    ("UBL-PlannedPeriodEndTime", "planned-period semantics unsettled; see FrameworkDurationStart"),
];

/// Sections that are Lots in the canonical sense — Parts and LotsGroups are
/// Lots with a kind flag (CONTEXT.md).
const LOT_KINDS: &[&str] = &["Lot", "LotsGroup", "Part"];

/// The section a lots-group composition lives in, and the two fields naming its ends
/// (issue 237). Named constants rather than literals in the reader because both ids are
/// `-Procedure`-suffixed and read as procedure-level facts, which is exactly the
/// confusion that left them unmapped.
const GROUP_COMPOSITION_KIND: &str = "GroupComposition";
/// BT-330: the LotsGroup this composition composes.
const GROUP_ID_FIELD: &str = "BT-330-Procedure";
/// BT-1375: one repeat per member lot of that group.
const GROUP_MEMBER_FIELD: &str = "BT-1375-Procedure";

/// The results-layer entity sections (docs/research/eforms-data-model.md §2):
/// LotResult = the award decision, LotTender = a Bid, TenderingParty = the
/// consortium behind a Bid, SettledContract = a Contract.
const RESULT_KINDS: &[&str] = &["LotResult", "LotTender", "TenderingParty", "SettledContract"];

/// Legacy Organization mention fields (inline address blocks — research §6):
/// the party's name, its country, and its raw national id (normalised and
/// plausibility-gated exactly like eForms BT-501).
const ORG_NAME_FIELDS: &[&str] = &["TED-OFFICIALNAME", "TXT-AU"];
const ORG_COUNTRY_FIELDS: &[&str] = &["TED-COUNTRY", "TED-ISO_COUNTRY", "TXT-CY"];
const ORG_NATIONALID_FIELD: &str = "TED-NATIONALID";

/// Publication-date fields, best-first (issue 18): the true OJEU / OJ S
/// publication date where the era stamps one, then the requested/portal
/// publication date DÖE carries in place of an OJEU stamp — DÖE notices are
/// published on the national portal and have no `efac:Publication` block, so
/// their requested date is the only publication signal they carry.
///
/// **Each dialect id sits immediately after the eForms id it aliases to** (issue
/// 367). These lists are read at BOTH layers: the projection calls
/// [`notice_instants`] after [`normalise_de1`] has folded the vocabulary, the
/// processor calls it on the RAW parse ([`crate::process`]'s `resolved_notice`),
/// because the stored notice layer keeps the publisher's own `DE1-*` ids on
/// purpose (see [`DE1_FIELD_ALIASES`]) — so normalising there would either
/// rewrite what the parse layer records or cost a clone of every parse. Naming
/// both vocabularies here instead is the shape issue 18 already used for
/// `SDK01-*`; adjacency is what makes the two layers agree, since a dialect id
/// placed out of order would out- or under-rank its own alias target and the
/// notice row would resolve a different date than its version. Pinned by
/// `every_de1_date_alias_sits_beside_its_target`.
const PUBLICATION_DATE_FIELDS: &[&str] = &[
    "OPP-012-notice",                  // eForms efbc:PublicationDate (TED; DÖE when stamped)
    "DE1-Publication-PublicationDate", // ↑ its eForms-DE 1.x spelling (issue 367)
    "TED-DATE_PUB",                    // legacy r208/r209 REF_OJS publication date
    "TXT-PD",                          // text-era PD: publication date
    "BT-738-notice",                   // eForms RequestedPublicationDate — the DÖE portal date
    "DE1-RequestedPublicationDate",    // ↑ its eForms-DE 1.x spelling (issue 367)
    "SDK01-RequestedPublicationDate",  // DÖE sdk-0.1 requested publication date
];

/// Dispatch-date fields, best-first (issue 18): when the notice left the
/// sender. Kept as its own axis because ordering within a publication day, and
/// the dispatch-vs-publication skew itself, are real questions consumers ask.
///
/// The `DE1-*` entry is here for the reason [`PUBLICATION_DATE_FIELDS`] gives.
const DISPATCH_DATE_FIELDS: &[&str] = &[
    "BT-05(a)-notice",          // eForms cbc:IssueDate (TED + DÖE eforms-de)
    "DE1-IssueDate",            // ↑ its eForms-DE 1.x spelling (issue 367)
    "SDK01-IssueDate",          // DÖE sdk-0.1 issue date
    "TED-DS_DATE_DISPATCH",     // legacy dispatch (CODIF_DATA)
    "TED-DATE_DISPATCH_NOTICE", // legacy dispatch (form body)
    "TED-DATE_DISP",            // INTERNAL_OJS 2008 dispatch (BIB_DOC_S)
    "TXT-DS",                   // text-era DS: dispatch
];

/// Every field id either date axis can resolve — the repair job's read filter
/// (issue 367). `notices.published_at` / `dispatched_at` are re-derived from
/// `notice_dates` rows restricted to these ids, so the store never has to read
/// the whole parsed layer back to check one notice's instants; filtering to the
/// candidate ids cannot change which candidate [`first_date`] picks.
pub const INSTANT_DATE_FIELDS: &[&str] = &{
    let mut all = [""; PUBLICATION_DATE_FIELDS.len() + DISPATCH_DATE_FIELDS.len()];
    let mut i = 0;
    while i < PUBLICATION_DATE_FIELDS.len() {
        all[i] = PUBLICATION_DATE_FIELDS[i];
        i += 1;
    }
    let mut j = 0;
    while j < DISPATCH_DATE_FIELDS.len() {
        all[i + j] = DISPATCH_DATE_FIELDS[j];
        j += 1;
    }
    all
};

/// The notice's own OJS publication number, in-form (r209 emits it as a plain
/// `NO_DOC_OJS`; the text era's own id is `ND:`). Only used as a corroborating
/// node source — the publication id is the authoritative one.
const LEGACY_OWN_NUMBER_FIELDS: &[&str] = &["TED-NO_DOC_OJS", "TXT-ND"];

/// Received-bid count fields (research §5.1) — one legacy statistic, mapped to
/// the eForms `tenders` received-submission kind.
/// Field ids whose value states the tax basis of an amount in the same section
/// (issue 251). One so far: the text era's companion code, emitted beside the price it
/// qualifies.
///
/// The r208/r209 eras also publish a VAT indicator — `EXCLUDING_VAT` as a presence flag
/// and `INCLUDING_VAT` as a container — and both already reach the parse layer. They are
/// NOT here yet because neither is a code carrying `incl`/`excl`: pairing them needs
/// knowing how those elements sit relative to the value element, which is a payload read
/// of its own. Issue 251 names it as the follow-up.
const TAX_BASIS_FIELDS: &[&str] = &["TED-VAL_TOTAL_TAX_BASIS"];

/// The form eras state the basis as a bare marker element beside the value, inside a
/// container that groups the two (issue 251):
///
/// ```text
///     <COSTS_RANGE_AND_CURRENCY_WITH_VAT_RATE CURRENCY="RON">
///       <VALUE_COST FMTVAL="1681100">1 681 100</VALUE_COST>
///       <EXCLUDING_VAT/>
///     </COSTS_RANGE_AND_CURRENCY_WITH_VAT_RATE>
/// ```
///
/// Pairing them by SECTION would be wrong, and measurably so: the committed defence
/// award holds an `INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT` amount with no marker in the
/// same section as a `COSTS_RANGE` amount that has one, so a section-keyed lookup labels
/// the initial estimate from the final value's marker.
///
/// The parse layer already separates them, because
/// `INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT` is a field-id prefix wrapper and `COSTS_RANGE`
/// is not — so **the prefix is the container identity**:
///
/// ```text
///     TED-VALUE_COST                            ↔  TED-EXCLUDING_VAT
///     TED-INITIAL_…_CONTRACT.VALUE_COST         ↔  TED-INITIAL_…_CONTRACT.EXCLUDING_VAT
/// ```
///
/// Hence the marker id is derived from the amount id by swapping its trailing element,
/// which is exact rather than heuristic.
const AMOUNT_ELEMENT: &str = "VALUE_COST";
const BASIS_MARKERS: [(&str, &str); 2] = [("EXCLUDING_VAT", "excl"), ("INCLUDING_VAT", "incl")];

const LEGACY_BID_COUNT_FIELDS: &[&str] =
    &["TED-NB_TENDERS_RECEIVED", "TED-OFFERS_RECEIVED_NUMBER"];

/// DÖE sdk-0.1 party sections (issue 29): the buyer is an inline
/// `ContractingParty`, the winner an inline `WinningParty` under a
/// `TenderResult` — neither is an eForms `Organization` section, so they seed
/// mentions of their own. Each carries its name/country on its *direct* Party
/// subtree; the nested `ServiceProviderParty` (the eSender) is deliberately not
/// read as the buyer/winner.
const SDK01_BUYER_KIND: &str = "ContractingParty";
const SDK01_WINNER_KIND: &str = "WinningParty";
const SDK01_RESULT_KIND: &str = "TenderResult";
const SDK01_PARTY_KINDS: &[&str] = &[SDK01_BUYER_KIND, SDK01_WINNER_KIND];
const SDK01_PARTY_NAME_FIELDS: &[&str] =
    &["SDK01-ContractingParty-Party-PartyName-Name", "SDK01-TenderResult-WinningParty-Party-PartyName-Name"];

/// Every field id that carries a party's NAME, across all vocabularies — the
/// issue-307 backfill walk's probe list (the union of what `mentions()`
/// treats as `is_name`). Pub because the walk lives in the store (dependency
/// direction) and the supervisor hands it this list.
pub const ORG_NAME_FIELD_IDS: &[&str] = &[
    "BT-500-Organization-Company",
    "TED-OFFICIALNAME",
    "TXT-AU",
    "SDK01-ContractingParty-Party-PartyName-Name",
    "SDK01-TenderResult-WinningParty-Party-PartyName-Name",
];
const SDK01_PARTY_COUNTRY_FIELDS: &[&str] = &[
    "SDK01-ContractingParty-Party-PostalAddress-Country-IdentificationCode",
    "SDK01-TenderResult-WinningParty-Party-PostalAddress-Country-IdentificationCode",
];
/// sdk-0.1's award-decision code, on the `TenderResult` section.
const SDK01_RESULT_CODE_FIELD: &str = "SDK01-TenderResult-TenderResultCode";
/// When the buyer awarded. On this dialect it is very often the ONLY thing the
/// result block says (issue 257): ~90 % of sdk-0.1 award notices publish a
/// `TenderResult` carrying an AwardDate and nothing else — no result code, no
/// winner, no value. `AwardTime` is published beside it and is not merged: the
/// day is the fact anyone reads, and a half-carried instant is worse than a date.
const SDK01_AWARD_DATE_FIELD: &str = "SDK01-TenderResult-AwardDate";
/// The legacy eras' award date, read on the award block by `read_legacy_results`
/// (issue 255). Named so the destination predicate and the reader agree: the
/// r208 probe listed 1,688 rows of it as unread on the date channel (2026-09-12)
/// while the reader consumed every one — a literal in the reader that the
/// predicate could not see.
const LEGACY_AWARD_DATE_FIELD: &str = "TED-CONTRACT_AWARD_DATE";
/// The same award-block date under its R2.0.7 spelling (issue 383): F06 awards of
/// 2010 publish `DATE_OF_CONTRACT_AWARD` as DAY/MONTH/YEAR, which the parse layer
/// has already made one instant. Read on one carrier (070248-2010, seventeen
/// award blocks, all 2009-06-01) before being matched: same section, same fact.
/// The r208 probe listed 79 rows of it as dropped in the era's head window; its
/// corpus size is whatever `refold-fields` counts.
const LEGACY_AWARD_DATE_FIELD_R207: &str = "TED-DATE_OF_CONTRACT_AWARD";
/// The legacy "no contract was awarded" marker on the award block, a `Rule::Marker`
/// the parser stores as `Integer(1)`; the results reader turns it into the
/// `clos-nw` decision. Named for the same reason as the award date (issue 384):
/// the reader matched it with a wildcard value pattern, so the predicate had
/// no channel to know it on and would have listed it as dropped.
const LEGACY_NO_AWARD_MARKER: &str = "TED-NO_AWARDED_CONTRACT";
/// sdk-0.1's procedure folder id (its BT-04 analogue). Only a genuine uuid is a
/// strong-enough cross-reference to key a Tender on (issue 34); the numeric
/// channel's non-uuid folder ids are notice-local and stay islands.
const SDK01_FOLDER_FIELD: &str = "SDK01-ContractFolderID";

/// eForms-DE 1.x speaks the eForms *structure* with its own field-id vocabulary.
/// The national 1.x line shipped no SDK `fields.json` (issue 75), so the vendored
/// inventory is empirical and names every leaf by its element path
/// (`DE1-ProcurementProject-Name`) instead of by business term (`BT-21-Lot`).
/// Every canonical mapping in this module keys on the eForms ids, so a DE-1.x
/// notice folded as-is matched nothing at all: 218k notices reclaimed into a rich
/// parse layer projected to Tender versions with no title, no CPV/NUTS, no
/// amounts, no lots and no parties (issue 85 — the same class of gap the DÖE
/// sdk-0.1 dialect hit in issue 29).
///
/// Rather than teach ~15 match sites a second vocabulary, the dialect is folded
/// onto the eForms one ONCE, as a chunk is read ([`normalise_de1`]): downstream
/// the whole projection sees standard `BT-*`/`OPT-*` ids and every existing rule
/// — stem matching, the results graph, org mentions, the instant resolution —
/// applies unchanged. The stored notice layer keeps its `DE1-*` ids: they are the
/// source's own names, and the parse layer records what the publisher sent.
///
/// Only the leaves with a genuine eForms equivalent are listed. The rest of the
/// 460-field inventory stays in the notice layer under its `DE1-*` id, retrievable
/// but not surfaced as a canonical fact — exactly how the legacy eras are handled.
const DE1_FIELD_ALIASES: &[(&str, &str)] = &[
    // Notice identity: subtype drives fold order, and the three instants resolve
    // published/dispatched (issue 18). The folder id is deliberately NOT aliased
    // onto BT-04 — see [`DE1_FOLDER_FIELD`], it is keyed through the gated path.
    ("DE1-ID", LOGICAL_NOTICE_FIELD),
    ("DE1-NoticeSubType-SubTypeCode", SUBTYPE_FIELD),
    ("DE1-Publication-PublicationDate", "OPP-012-notice"),
    ("DE1-RequestedPublicationDate", "BT-738-notice"),
    ("DE1-IssueDate", "BT-05(a)-notice"),
    // Title and description, at Tender and Lot scope.
    ("DE1-ProcurementProject-Name", "BT-21-Procedure"),
    ("DE1-ProcurementProjectLot-ProcurementProject-Name", "BT-21-Lot"),
    ("DE1-ProcurementProject-Description", "BT-24-Procedure"),
    ("DE1-ProcurementProjectLot-ProcurementProject-Description", "BT-24-Lot"),
    // Values: the estimate and its framework ceiling, at Tender and Lot scope.
    ("DE1-ProcurementProject-RequestedTenderTotal-EstimatedOverallContractAmount", "BT-27-Procedure"),
    (
        "DE1-ProcurementProjectLot-ProcurementProject-RequestedTenderTotal-EstimatedOverallContractAmount",
        "BT-27-Lot",
    ),
    ("DE1-ProcurementProject-RequestedTenderTotal-FrameworkMaximumAmount", "BT-271-Procedure"),
    (
        "DE1-ProcurementProjectLot-ProcurementProject-RequestedTenderTotal-FrameworkMaximumAmount",
        "BT-271-Lot",
    ),
    ("DE1-NoticeResult-TotalAmount", "BT-161-NoticeResult"),
    // CPV (main + additional) and the realized-location NUTS, at both scopes.
    ("DE1-ProcurementProject-MainCommodityClassification-ItemClassificationCode", "BT-262-Procedure"),
    (
        "DE1-ProcurementProjectLot-ProcurementProject-MainCommodityClassification-ItemClassificationCode",
        "BT-262-Lot",
    ),
    (
        "DE1-ProcurementProject-AdditionalCommodityClassification-ItemClassificationCode",
        "BT-263-Procedure",
    ),
    (
        "DE1-ProcurementProjectLot-ProcurementProject-AdditionalCommodityClassification-ItemClassificationCode",
        "BT-263-Lot",
    ),
    ("DE1-ProcurementProject-RealizedLocation-Address-CountrySubentityCode", "BT-5071-Procedure"),
    (
        "DE1-ProcurementProjectLot-ProcurementProject-RealizedLocation-Address-CountrySubentityCode",
        "BT-5071-Lot",
    ),
    // Dates. The parser has already reunited each `EndDate`/`EndTime` pair into one
    // instant (issue 03), so these are the whole deadline — the `(d)` ids.
    ("DE1-TenderingProcess-TenderSubmissionDeadlinePeriod-EndDate", "BT-131(d)-Procedure"),
    ("DE1-ProcurementProjectLot-TenderingProcess-TenderSubmissionDeadlinePeriod-EndDate", "BT-131(d)-Lot"),
    (
        "DE1-ProcurementProjectLot-TenderingProcess-ParticipationRequestReceptionPeriod-EndDate",
        "BT-1311(d)-Lot",
    ),
    ("DE1-TenderingProcess-OpenTenderEvent-OccurrenceDate", "BT-132(d)-Procedure"),
    ("DE1-ProcurementProjectLot-TenderingProcess-OpenTenderEvent-OccurrenceDate", "BT-132(d)-Lot"),
    (
        "DE1-ProcurementProjectLot-TenderingProcess-AdditionalInformationRequestPeriod-EndDate",
        "BT-13(d)-Lot",
    ),
    ("DE1-ProcurementProject-PlannedPeriod-StartDate", "BT-536-Procedure"),
    ("DE1-ProcurementProjectLot-ProcurementProject-PlannedPeriod-StartDate", "BT-536-Lot"),
    ("DE1-ProcurementProject-PlannedPeriod-EndDate", "BT-537-Procedure"),
    ("DE1-ProcurementProjectLot-ProcurementProject-PlannedPeriod-EndDate", "BT-537-Lot"),
    // Organization identity. DE-1.x carries the standard `efac:Organization`
    // sections, so only the leaf names differ.
    ("DE1-Organizations-Organization-Company-PartyName-Name", ORG_NAME_FIELD),
    ("DE1-Organizations-Organization-Company-PartyLegalEntity-CompanyID", ORG_IDENTIFIER_FIELD),
    ("DE1-Organizations-Organization-Company-PostalAddress-Country-IdentificationCode", ORG_COUNTRY_FIELD),
    // Organization role references (eForms' OPT-300/301 pattern).
    ("DE1-ContractingParty-Party-PartyIdentification-ID", "OPT-300-Procedure-Buyer"),
    ("DE1-ContractingParty-Party-ServiceProviderParty-Party-PartyIdentification-ID", "OPT-300-Procedure-SProvider"),
    ("DE1-NoticeResult-TenderingParty-Tenderer-ID", "OPT-300-Tenderer"),
    ("DE1-NoticeResult-TenderingParty-SubContractor-ID", "OPT-301-Tenderer-SubCont"),
    ("DE1-NoticeResult-TenderingParty-SubContractor-MainContractor-ID", "OPT-301-Tenderer-MainCont"),
    ("DE1-NoticeResult-SettledContract-SignatoryParty-PartyIdentification-ID", "OPT-300-Contract-Signatory"),
    ("DE1-NoticeResult-LotResult-FinancingParty-PartyIdentification-ID", "OPT-301-LotResult-Financing"),
    ("DE1-NoticeResult-LotResult-PayerParty-PartyIdentification-ID", "OPT-301-LotResult-Paying"),
    // The lot-level role parties (issue 98). eForms splits each of these into a
    // `Lot-`/`Part-` pair by the `schemeName` predicate DE-1.x does not carry
    // (issue 75), and the DE dialect publishes some at procedure scope and some
    // under the lot — both fold onto the `Lot-` id, because `role_name` uses the
    // suffix verbatim as the role string and the projection resolves the scope
    // separately. That yields exactly the role names TED twins already carry
    // onto these same Tenders (`Lot-ReviewOrg`, `Lot-AddInfo`, …), which is what
    // makes a DE version and its TED twin agree instead of inventing a dialect.
    ("DE1-TenderingTerms-AppealTerms-AppealReceiverParty-PartyIdentification-ID", "OPT-301-Lot-ReviewOrg"),
    ("DE1-ProcurementProjectLot-TenderingTerms-AppealTerms-AppealReceiverParty-PartyIdentification-ID", "OPT-301-Lot-ReviewOrg"),
    ("DE1-TenderingTerms-AppealTerms-AppealInformationParty-PartyIdentification-ID", "OPT-301-Lot-ReviewInfo"),
    ("DE1-ProcurementProjectLot-TenderingTerms-AppealTerms-AppealInformationParty-PartyIdentification-ID", "OPT-301-Lot-ReviewInfo"),
    ("DE1-TenderingTerms-AppealTerms-MediationParty-PartyIdentification-ID", "OPT-301-Lot-Mediator"),
    ("DE1-ProcurementProjectLot-TenderingTerms-AppealTerms-MediationParty-PartyIdentification-ID", "OPT-301-Lot-Mediator"),
    ("DE1-TenderingTerms-TenderRecipientParty-PartyIdentification-ID", "OPT-301-Lot-TenderReceipt"),
    ("DE1-ProcurementProjectLot-TenderingTerms-TenderRecipientParty-PartyIdentification-ID", "OPT-301-Lot-TenderReceipt"),
    ("DE1-ProcurementProjectLot-TenderingTerms-AdditionalInformationParty-PartyIdentification-ID", "OPT-301-Lot-AddInfo"),
    ("DE1-ProcurementProjectLot-TenderingTerms-DocumentProviderParty-PartyIdentification-ID", "OPT-301-Lot-DocProvider"),
    ("DE1-ProcurementProjectLot-TenderingTerms-TenderEvaluationParty-PartyIdentification-ID", "OPT-301-Lot-TenderEval"),
    ("DE1-TenderingTerms-FiscalLegislationDocumentReference-IssuerParty-PartyIdentification-ID", "OPT-301-Lot-FiscalLegis"),
    ("DE1-ProcurementProjectLot-TenderingTerms-FiscalLegislationDocumentReference-IssuerParty-PartyIdentification-ID", "OPT-301-Lot-FiscalLegis"),
    ("DE1-TenderingTerms-EmploymentLegislationDocumentReference-IssuerParty-PartyIdentification-ID", "OPT-301-Lot-EmployLegis"),
    ("DE1-ProcurementProjectLot-TenderingTerms-EmploymentLegislationDocumentReference-IssuerParty-PartyIdentification-ID", "OPT-301-Lot-EmployLegis"),
    ("DE1-TenderingTerms-EnvironmentalLegislationDocumentReference-IssuerParty-PartyIdentification-ID", "OPT-301-Lot-EnvironLegis"),
    ("DE1-ProcurementProjectLot-TenderingTerms-EnvironmentalLegislationDocumentReference-IssuerParty-PartyIdentification-ID", "OPT-301-Lot-EnvironLegis"),
    // The results graph: award decision, its lot, and the bid/contract edges that
    // resolve a winner.
    ("DE1-NoticeResult-LotResult-TenderResultCode", "BT-142-LotResult"),
    ("DE1-NoticeResult-LotResult-DecisionReason-DecisionReasonCode", "BT-144-LotResult"),
    ("DE1-NoticeResult-LotResult-TenderLot-ID", "BT-13713-LotResult"),
    ("DE1-NoticeResult-LotResult-LotTender-ID", "OPT-320-LotResult"),
    ("DE1-NoticeResult-LotResult-SettledContract-ID", "OPT-315-LotResult"),
    ("DE1-NoticeResult-LotResult-ReceivedSubmissionsStatistics-StatisticsNumeric", "BT-759-LotResult"),
    ("DE1-NoticeResult-LotResult-ReceivedSubmissionsStatistics-StatisticsCode", "BT-760-LotResult"),
    ("DE1-NoticeResult-LotTender-LegalMonetaryTotal-PayableAmount", "BT-720-Tender"),
    ("DE1-NoticeResult-LotResult-LotTender-LegalMonetaryTotal-PayableAmount", "BT-720-Tender"),
    ("DE1-NoticeResult-LotTender-TenderLot-ID", "BT-13714-Tender"),
    ("DE1-NoticeResult-LotTender-TenderingParty-ID", "OPT-310-Tender"),
    ("DE1-NoticeResult-SettledContract-ContractReference-ID", "BT-150-Contract"),
    ("DE1-NoticeResult-SettledContract-IssueDate", "BT-145-Contract"),
    ("DE1-NoticeResult-SettledContract-LotTender-ID", "BT-3202-Contract"),
];

/// eForms-DE 1.x's single, predicate-free lot node. The EU SDK splits
/// `cac:ProcurementProjectLot` into ND-Lot / ND-LotsGroup / ND-Part by an
/// `[cbc:ID/@schemeName='…']` predicate; the empirical DE-1.x inventory carries no
/// predicates (issue 75), so all three collapse onto one node whose kind is the
/// element name. The distinction is not lost — it is exactly the section id's own
/// prefix, which is where [`de1_lot_kind`] reads it back from.
const DE1_LOT_KIND: &str = "ProcurementProjectLot";

/// eForms-DE 1.x's procedure folder id — its BT-04 analogue, and the only alias
/// deliberately kept OUT of [`DE1_FIELD_ALIASES`], because keying a Tender is not
/// the same trust decision as mapping a fact.
///
/// `procedure_key` accepts any non-empty BT-04 unchecked, because on TED BT-04 is
/// a spec-guaranteed uuid. eForms-DE 1.x carries no such guarantee: the inventory
/// is empirical (issue 75) and the national spec is not the EU one, so a portal
/// -local reference number here would key a Tender on a string that is only
/// notice-local — and every notice sharing it would collapse into one Tender.
/// That is issue 34's failure exactly, which is why the sdk-0.1 folder id is
/// gated, and this one is gated the same way.
///
/// The gate cannot cost a real merge: a DÖE notice with a genuine TED twin shares
/// that twin's BT-04, which *is* a uuid, so it passes untouched. A folder id that
/// fails the gate leaves the notice an island — the state it is in today — so the
/// bad case is a missed link that splits and re-merges cleanly when a real key
/// appears, never an unrecoverable wrong merge across 218k notices (ADR-0003).
const DE1_FOLDER_FIELD: &str = "DE1-ContractFolderID";

const PROCEDURE_KEY_FIELD: &str = "BT-04-notice";
const LOGICAL_NOTICE_FIELD: &str = "BT-701-notice";
const SUBTYPE_FIELD: &str = "OPP-070-notice";
/// The notice's ORIGINAL language, as each era publishes it — ADR-0013 D3's third
/// leg, which the ADR's first amendment thought had no data source. It has three:
/// `BT-702(a)-notice` (every eForms SDK, DE included), `TED-LG_ORIG` (r208/r209 —
/// the notice-level element, distinct from the per-copy `LG` attribute the
/// amendment was looking at), and `TXT-OL` (the text era's `OL:` line, absent on
/// the early-1990s notices that predate it). All three are PROCEDURE-level codes
/// already in `notice_codes`, so this is a fold-time read, not a parser change.
const ORIGINAL_LANG_FIELDS: [&str; 5] = [
    "BT-702(a)-notice",
    "TED-LG_ORIG",
    "TXT-OL",
    // The national eForms generations before SDK-DE publish the same root element
    // (`/*/cbc:NoticeLanguageCode`) under their empirical inventories' ids — issue
    // 344: every eForms-DE 1.x and DÖE sdk-0.1 version had no original language
    // while its notice said DEU.
    "DE1-NoticeLanguageCode",
    "SDK01-NoticeLanguageCode",
];
/// eForms' explicit previous-publication reference (`ND-PreviousNoticeReference`):
/// the publisher's own statement that an earlier TED publication continues into
/// this notice. ADR-0011 makes it an identity edge, because EU eForms does NOT
/// keep BT-04 stable across a procedure's notices — measured on prod, 27–39 % of
/// EU award Tenders are single-notice islands whose contract notice sits in the
/// corpus under a different BT-04 (issue 236), and this field is how the source
/// says so.
const PREVIOUS_NOTICE_FIELD: &str = "OPP-090-Procedure";
const ORGANIZATION_KIND: &str = "Organization";
const ORG_NAME_FIELD: &str = "BT-500-Organization-Company";
const ORG_IDENTIFIER_FIELD: &str = "BT-501-Organization-Company";
const ORG_COUNTRY_FIELD: &str = "BT-514-Organization-Company";
/// Business Registration Information Notices carry no procurement procedure;
/// CONTEXT.md makes them minimal Tenders of their own kind.
const REGISTRATION_SUBTYPE: &str = "X01";

/// Run the projection over every parsed notice. With `rebuild`, the canonical
/// layer's content is dropped first and re-derived from scratch — the change
/// log is kept and appended to, never renumbered.
pub async fn project(db: &Db, rebuild: bool) -> turso::Result<Report> {
    // ADR-0014: one rates snapshot per run — every version's eur_cents derives
    // from the same table state, and an empty table means honest NULLs.
    db.reload_rates_lookup().await?;
    let pre_populated = wipe_guard_pre(db, rebuild).await?;
    // The projection writes a self-consistent graph by construction, so it runs
    // with FK enforcement off (issue 19) — the per-row FK check on millions of
    // satellite inserts is the projection's super-linear cost at scale — and
    // restores it unconditionally, so no other write path loses the guard.
    db.set_foreign_keys(false).await?;
    let result = project_inner(db, rebuild).await;
    let restored = db.set_foreign_keys(true).await;
    let report = result?;
    restored?;
    wipe_guard_post(db, pre_populated).await?;
    Ok(report)
}

/// Issue 133, the projection-side placements (the continuous one lives in
/// `/health/deep`). The 2026-07-30 incident: a killed rebuild's
/// `reset_tender_layer` is DDL, durable the instant it runs, so the layer sat
/// empty for hours while every later signal stayed green.
///
/// Start precondition — RELATIVE to the `layer_presence` witness, which
/// survives the wipe in its own table: an incremental fold onto a layer that
/// was populated once and is empty now would fold one day's delta onto a wiped
/// corpus and record success. Refuse, naming the repair path. A rebuild IS the
/// repair path, so it passes unconditionally.
pub async fn wipe_guard_pre(db: &Db, rebuild: bool) -> turso::Result<bool> {
    let (populated, ever_populated) = db.tender_layer_state().await?;
    if !rebuild && !populated && ever_populated {
        return Err(turso::Error::Corrupt(
            "the canonical layer was populated once and is empty now — an incremental fold \
             would compound the wipe and record it as success; run a rebuild (issue 133)"
                .into(),
        ));
    }
    Ok(populated)
}

/// End assertion — RELATIVE to what this run itself observed at entry, never
/// absolute (a first fold legitimately starts and can end empty): a run that
/// began with a populated layer and ends with an empty one emptied it,
/// whatever its own report claims. The chunked commits are already durable, so
/// this cannot un-wipe; what it refuses is *reporting success*, which turns
/// the wipe into a failed `project` job — the signal issue 32's jobwatch
/// surfaces — instead of a green run nobody questions.
pub async fn wipe_guard_post(db: &Db, pre_populated: bool) -> turso::Result<()> {
    if !pre_populated {
        return Ok(());
    }
    let (populated, _) = db.tender_layer_state().await?;
    if !populated {
        return Err(turso::Error::Corrupt(
            "this projection run emptied a populated canonical layer — refusing to report \
             success so the wipe surfaces as a failed job (issue 133)"
                .into(),
        ));
    }
    Ok(())
}

/// The process's peak resident set so far, in MB, from `/proc/self/status` `VmHWM`
/// (a monotonic high-water mark — so probing it at each stage boundary reports the
/// run's TRUE peak, not the instantaneous RSS). This is the number issue 57's
/// acceptance wants ("peak RSS well under the box"); it was never logged, so a
/// rebuild's real anonymous memory cost had to be inferred from cgroup peaks that
/// include reclaimable page cache. 0 when unreadable (non-Linux / sandboxed) — a
/// projection must never fail over a diagnostic.
fn peak_rss_mb() -> u64 {
    std::fs::read_to_string("/proc/self/status").map(|s| parse_vm_hwm_mb(&s)).unwrap_or(0)
}

/// Parse the `VmHWM:` line (`VmHWM:\t   12345 kB`) out of `/proc/self/status` and
/// return it in MB. Split out from the read so it is unit-testable without a live
/// `/proc`. Returns 0 when the line is absent or malformed.
fn parse_vm_hwm_mb(status: &str) -> u64 {
    status
        .lines()
        .find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|kb| kb.parse::<u64>().ok())
        .map(|kb| kb / 1024)
        .unwrap_or(0)
}

async fn project_inner(db: &Db, rebuild: bool) -> turso::Result<Report> {
    project_with_batch(db, rebuild, APPLY_NOTICE_BATCH).await
}

/// Notices per Phase-2 fold+apply batch. The projection groups a whole corpus's
/// notices into Tenders whose members are scattered across the id space by
/// publication history, so it cannot be windowed by period without splitting a
/// Tender (ADR-0001, issue 57). Instead it plans the grouping first (holding only
/// compact per-notice identity), then folds and applies **whole** Tenders a
/// bounded batch of notices at a time — never the whole corpus's states and
/// mentions at once, which is what OOM-crash-looped the 8 GB VPS. The batch is a
/// count of notices (not Tenders) so peak memory is bounded regardless of how
/// large individual Tenders are.
const APPLY_NOTICE_BATCH: usize = 50_000;

/// How many Phase-2 batches between WAL truncations. The apply burst grows the
/// WAL; truncating at the clean point between batches returns the space (issue
/// 42) without checkpointing so often the cost shows.
const CHECKPOINT_EVERY_BATCHES: usize = 4;

/// Phase-1 (`build_plan`) truncates the WAL EVERY chunk, tighter than the Phase-2
/// cadence: a full-corpus plan build writes ~40M rows (14.2M plan + ~28M
/// org/mention), and a sparse cadence lets the WAL — and turso's in-RAM WAL-index,
/// which holds an entry per un-checkpointed frame — grow to the OOM point (the
/// 2026-07-30 rebuild: WAL 1MB→3.5GB, +640MB/min, OOM). One TRUNCATE per
/// 10k-notice chunk keeps the WAL (and its index) tiny throughout, bounding the
/// RAM regardless of write volume; a small WAL also truncates fast, so the added
/// checkpoints are cheap (issue 63).
const PLAN_CHECKPOINT_EVERY: usize = 1;

/// How often Phase-1 records a diagnostic line (chunk, notices, WAL size, last
/// checkpoint busy/frames) to the DB-side `.diag.log` (issue 63) — a channel that
/// survives the worker-runtime stderr not reaching journald. Every 16 chunks
/// (~160k notices) traces the WAL trend without flooding.
const PLAN_DIAG_EVERY: usize = 16;

/// How often Phase 1 logs a heartbeat. A full-corpus plan build streams millions
/// of notices over many minutes; without a heartbeat the run looks dead from the
/// outside, which made the issue-57 incident far harder to diagnose (issue 59).
const PLAN_HEARTBEAT: u64 = 500_000;

/// A projection progress event, for operability. The projection runs for many
/// minutes on a full corpus, so it reports a live heartbeat in **both** phases —
/// an operator (and the logs) can see it moving and roughly how far along, and
/// tell "working" from "stuck". [`project`] logs these to stderr; a caller can
/// observe them directly via [`project_with_progress`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    /// Phase 1: `notices` of `total` parsed notices planned so far.
    Planning { notices: u64, total: u64 },
    /// The incremental fold's pass-1 identity scan: `notices` of `total`
    /// CHANGED notices scanned for keys/adjacency. Its own variant (issue 305):
    /// it used to borrow `Planning`, so a 98-minute identity pass read as
    /// "planning" and the 58-v2 fallback's real plan build then restarted the
    /// same-named counter from zero — indistinguishable from a crash-restart.
    Identity { notices: u64, total: u64 },
    /// Phase 1 → 2 transition: the plan grouped into `tenders` (`islands` of them
    /// single-notice).
    Grouped { tenders: u64, islands: u64 },
    /// Phase 2: `tenders` of `total` folded so far, and `versions` version rows
    /// actually WRITTEN.
    ///
    /// The two are deliberately separate. `tenders` counts groups the fold has
    /// processed, which climbs to completion even when every single one hits
    /// `apply_tender_tx`'s unchanged-chain early return and writes nothing — so a
    /// fold that is a total no-op looks identical to a healthy one on that counter
    /// alone. `versions` is what distinguishes them, and it is the first-heartbeat
    /// signal that a projection-logic re-fold (issue 99's epoch) is really
    /// rewriting rather than silently skipping.
    /// `leaf_rows` is the satellite rows those versions carried (issue 96).
    Applying { tenders: u64, total: u64, versions: u64, leaf_rows: u64 },
    /// Phase 2 pre-pass: `notices` read and spilled to buckets so far, summed
    /// across all shard workers (issue 65). No total: the sweep's bound is an id
    /// RANGE, not a row count, and counting the rows in it up front would cost a
    /// scan of exactly the shape the pre-pass exists to do once — so this reports
    /// movement without a destination rather than paying twice for one. Emitted
    /// by the parent thread on a coarse poll of the workers' shared counter, so
    /// ticks arrive every couple of seconds however many shards run; a final
    /// tick with the complete count always closes the phase, which is also what
    /// makes the variant deterministic for tests over corpora that finish before
    /// the first poll.
    PrePass { notices: u64 },
}

/// Which Phase-2 fold the projection runs. Byte-identical either way — both build
/// the same [`TenderProjection`]s and feed them to `apply_tenders` in the same
/// global fold order — so the choice is purely how the parsed layer is READ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase2 {
    /// Read each fold batch's parsed layer by an `IN(…)` over its scattered notice
    /// ids — random rowid seeks into the cold notice tables in group_key order,
    /// ~50ms/notice at scale (the ~7-day Phase-2, issue 62). Still used by
    /// [`project_incremental`] (its delta is small and already scoped) and kept as
    /// the fold-source-invariance baseline.
    ParsedFold,
    /// Read the WHOLE parsed layer ONCE, sequentially in notice_id order (a forward
    /// read-ahead-friendly sweep), spilling each resolved notice state to an
    /// order-preserving on-disk bucket; then fold each bucket sorted in RAM (issue
    /// 62). The default — minutes, not days — with peak RAM of one bucket.
    ///
    /// `shards` is how many parallel id-stripe workers the pre-pass runs (issue 66);
    /// `None` picks `cores − 1`, capped to the file-descriptor budget. The fold is
    /// always serial and the output is byte-identical for any worker count — sharding
    /// only changes which worker writes a notice, never which bucket it lands in.
    Buckets { shards: Option<usize> },
}

/// The projection with an explicit Phase-2 batch size (notices per fold+apply
/// batch). [`project`] uses [`APPLY_NOTICE_BATCH`]; tests drive tiny batches to
/// prove the output is invariant under batching — i.e. that folding whole Tenders
/// a batch at a time never splits a Tender's notices across a boundary (issue 57).
/// Progress is logged to stderr; use [`project_with_progress`] to observe it.
pub async fn project_with_batch(db: &Db, rebuild: bool, notice_batch: usize) -> turso::Result<Report> {
    project_with_progress(db, rebuild, notice_batch, stderr_progress_sink()).await
}

/// The default progress sink: heartbeats to stderr (the journal), the per-event
/// phases throttled so a multi-hour run logs steadily rather than floods.
/// Extracted (issue 65) so the supervisor can COMPOSE with it — journal lines
/// and the durable phase record come from one mapping, not two drifting copies.
pub fn stderr_progress_sink() -> impl FnMut(Progress) {
    let mut last_plan_log = 0u64;
    let mut last_prepass_log = 0u64;
    move |p| match p {
        Progress::Planning { notices, total } => {
            if notices - last_plan_log >= PLAN_HEARTBEAT || notices == total {
                eprintln!("[project] phase 1: {notices}/{total} notices planned");
                last_plan_log = notices;
            }
        }
        Progress::Identity { notices, total } => {
            if notices - last_plan_log >= PLAN_HEARTBEAT || notices == total {
                eprintln!("[project] pass-1 identity: {notices}/{total} changed notices scanned");
                last_plan_log = notices;
            }
        }
        Progress::Grouped { tenders, islands } => {
            eprintln!("[project] phase 2: folding {tenders} tenders ({islands} islands)");
        }
        Progress::Applying { tenders, total, versions, leaf_rows } => {
            eprintln!(
                "[project] phase 2: {tenders}/{total} tenders folded, {versions} versions written, {leaf_rows} leaf rows"
            );
        }
        // Aggregate line beside the per-shard heartbeats `write_shard` already
        // prints (issue 94) — same cadence bound, so the journal cost is one
        // extra line per PREPASS_HEARTBEAT notices, not one per poll tick.
        Progress::PrePass { notices } => {
            if notices - last_prepass_log >= PREPASS_HEARTBEAT {
                eprintln!("[project] phase 2 pre-pass: {notices} notices swept into buckets");
                last_prepass_log = notices;
            }
        }
    }
}

/// [`project`] with the caller observing progress on top of the default journal
/// logging (issue 65): every event reaches BOTH the stderr sink and `observe`.
/// This is the supervisor's entry — the journal keeps its heartbeats and the
/// durable job-phase record gets the same stream, so the two can never disagree
/// about what the projection was doing.
pub async fn project_observed(
    db: &Db,
    rebuild: bool,
    mut observe: impl FnMut(Progress),
) -> turso::Result<Report> {
    let mut log = stderr_progress_sink();
    project_with_progress(db, rebuild, APPLY_NOTICE_BATCH, move |p| {
        log(p);
        observe(p);
    })
    .await
}

/// The projection core with the default (bucketed) Phase-2 fold, reporting progress
/// through `on_progress`. See [`Phase2`] and [`project_with_progress_phase2`].
pub async fn project_with_progress(
    db: &Db,
    rebuild: bool,
    notice_batch: usize,
    on_progress: impl FnMut(Progress),
) -> turso::Result<Report> {
    project_with_progress_phase2(db, rebuild, notice_batch, Phase2::Buckets { shards: None }, on_progress).await
}

/// [`project_observed`] with a cooperative stop (issue 256): `stop` is polled at
/// the projection's checkpoints — between Phase-1 plan chunks, before grouping,
/// and between Phase-2 fold batches — and a `true` ends the run there with
/// [`Report::stopped`] set. This is what makes `project` a stoppable job kind:
/// before it, the only way off a grinding fold was a service restart, which
/// re-runs the job from the top (the TENDER_DROP_JOBS dance, twice in one day).
pub async fn project_observed_stoppable(
    db: &Db,
    rebuild: bool,
    mut observe: impl FnMut(Progress),
    stop: &(dyn Fn() -> bool + Sync),
) -> turso::Result<Report> {
    let mut log = stderr_progress_sink();
    project_with_progress_phase2_stoppable(
        db,
        rebuild,
        APPLY_NOTICE_BATCH,
        Phase2::Buckets { shards: None },
        move |p| {
            log(p);
            observe(p);
        },
        stop,
    )
    .await
}

/// The projection core, reporting progress through `on_progress` (called between
/// awaits, so a cheap closure), with an explicit Phase-2 fold selector ([`Phase2`]).
/// See [`Progress`]; [`project_with_batch`] wraps this with a stderr-logging sink
/// and the default (bucketed) fold.
pub async fn project_with_progress_phase2(
    db: &Db,
    rebuild: bool,
    notice_batch: usize,
    phase2: Phase2,
    on_progress: impl FnMut(Progress),
) -> turso::Result<Report> {
    project_with_progress_phase2_stoppable(db, rebuild, notice_batch, phase2, on_progress, &|| false)
        .await
}

/// The projection core with a cooperative stop; see [`project_observed_stoppable`]
/// for the checkpoint contract. Everything committed before the stop stays
/// committed — the flag never rolls anything back, it only declines to start the
/// next unit of work.
pub async fn project_with_progress_phase2_stoppable(
    db: &Db,
    rebuild: bool,
    notice_batch: usize,
    phase2: Phase2,
    mut on_progress: impl FnMut(Progress),
    stop: &(dyn Fn() -> bool + Sync),
) -> turso::Result<Report> {
    // Resume-from-plan salvage (issue 60): if an interrupted rebuild already left a
    // COMPLETE grouping plan on disk (Phase-1 finished — the expensive part), skip
    // the clear + strip + the whole of Phase-1 and re-run only grouping (path-B) →
    // Phase-2 from the immutable on-disk plan. A normal rebuild (its prior run
    // cleared the plan) sees an empty/absent plan and rebuilds from scratch.
    // Force a FRESH rebuild (rebuild the plan from the current parsed corpus) instead
    // of resuming an on-disk plan — set when the existing plan may not reflect the
    // current corpus (e.g. after a reclaim), so the fold cannot silently reuse a stale
    // plan. Drops the plan tables (O(1), via reset_plan) so plan_is_complete → false
    // and the from-scratch Phase-1 runs. Env valve, like TENDER_DISABLE_COVERAGE.
    if rebuild && std::env::var_os("TENDER_FORCE_FRESH_PLAN").is_some() {
        db.reset_plan().await?;
        db.log_diag("force-fresh: dropped the on-disk plan (TENDER_FORCE_FRESH_PLAN set)");
    }
    let resume = rebuild && db.plan_is_complete().await?;
    // Run-start marker on the .diag.log (issue 63) — confirms the channel works and
    // the run began, before the first stage probe (teardown) fires.
    db.log_diag(&format!("=== projection start: rebuild={rebuild} resume={resume} ==="));
    if rebuild && !resume {
        db.clear_canonical().await?;
        db.log_diag(&format!("WAL after clear_canonical: {} MB", db.wal_bytes().unwrap_or(0) / 1_048_576));
        // Bulk-load the Organization tables index-free, then rebuild the indexes
        // once at the end (issue 60): the per-row uniqueness probe into the org
        // identity index was a random-seek storm once it outgrew the page cache.
        db.strip_organization_indexes().await?;
    }
    if rebuild {
        // Defer the random-key tender satellite indexes for the from-scratch Phase-2
        // fold — this runs for BOTH a fresh rebuild and a resume (both fold from an
        // empty canonical layer). Maintaining organization_id / CPV / published_at /
        // notice_id / tenders-identity indexes live during the fold is the
        // issue-60/62 random-position write storm; they are rebuilt sorted at the end.
        //
        // Strip BEFORE reset (issue 64): a resume over a fully-indexed partial layer
        // (e.g. the rebuild=false fallback) would otherwise pay reset's per-row
        // content DELETEs against the live random-key indexes — a random-position
        // b-tree delete storm scaling with the partial's satellite rows. Dropping the
        // indexes first makes those DELETEs sequential page frees.
        // Mark the rebuild in-flight BEFORE emptying the layer, so an interruption
        // anywhere in Phase-2 is resumable (the supervisor's salvage keys on this
        // flag). Crucially it is set only for a rebuild that resets the layer — a
        // rebuild=false full-fallback never sets it, so an interrupted fallback over
        // an intact layer is NOT mistaken for a resumable rebuild. Cleared with the
        // plan on clean completion (`clear_plan`).
        db.set_rebuild_in_progress().await?;
        db.strip_tender_indexes().await?;
        db.log_diag(&format!("WAL before reset_tender_layer: {} MB", db.wal_bytes().unwrap_or(0) / 1_048_576));
        // Empty the tender-content layer and DROP+recreate `tenders` bare (fresh AND
        // resume both fold from empty): this strips the inline-UNIQUE auto-indexes
        // that steepened the fold and resets sqlite_sequence so ids restart at 1 in
        // fold order — resume becomes byte-identical to fresh. Preserves the Phase-1
        // Organizations the resume relies on (issue 60).
        db.reset_tender_layer().await?;
        db.log_diag(&format!("WAL after reset_tender_layer: {} MB", db.wal_bytes().unwrap_or(0) / 1_048_576));
    }
    // Stage-boundary WAL probe (issue 63): a full-corpus DELETE/UPDATE/CREATE INDEX
    // writes per-row WAL that no per-chunk checkpoint covers (it is one statement),
    // so a balloon here is invisible to build_plan's checkpoint log. Print the WAL
    // size at each stage boundary so a rebuild pinpoints exactly which stage balloons.
    // Emit each stage's WAL size to BOTH stderr and the DB-side .diag.log (issue 63):
    // the worker-runtime job's stderr did not reach journald, so the .diag.log is the
    // channel we can actually read (`cat {db}.diag.log`).
    let probe = |db: &Db, stage: &str| {
        let mb = db.wal_bytes().unwrap_or(0) / 1_048_576;
        let rss = peak_rss_mb();
        db.log_diag(&format!("WAL after {stage}: {mb} MB (peak RSS {rss} MB)"));
        eprintln!("[project] WAL after {stage}: {mb} MB (peak RSS {rss} MB)");
    };
    if rebuild {
        probe(db, "teardown (clear/strip/reset)");
    }
    let now = store::now_unix();
    let mut report = Report::default();

    // Phase 1 — build the whole grouping plan on disk (see [`build_plan`]). On a
    // RESUME the plan is already complete on disk, so skip it entirely.
    let t0 = std::time::Instant::now();
    let total = db.parsed_notice_count().await?;
    if resume {
        report.notices = total;
        eprintln!(
            "[project] RESUME: a complete on-disk plan ({total} notices) was found — \
             skipping Phase-1 and re-running grouping + Phase-2 (salvage)"
        );
    } else {
        let (notices, mentions, stopped, citations) =
            build_plan(db, now, total, &mut on_progress, stop).await?;
        report.stopped = stopped;
        report.notices = notices;
        report.mentions = mentions;
        report.citations = citations;
    }
    probe(db, "Phase-1 (build_plan)");
    if report.stopped {
        // Stopped mid-plan: the partial plan is NOT complete, so nothing downstream
        // may run on it — grouping would fold a truncated corpus and the salvage
        // machinery would rightly refuse it anyway (`plan_is_complete` is false).
        // Leave the plan for the next run's `reset_plan` to clear; a rebuild's
        // `rebuild_in_progress` flag stays set, so the next project job redoes the
        // build from scratch — a stop costs the redo, never correctness.
        return Ok(report);
    }

    // Group the plan into Tenders — keyed chains, the legacy OJS transitive-closure
    // union-find, islands — entirely in SQL over the on-disk plan (issue 59), so no
    // whole-corpus structure ever enters RAM.
    let t1 = std::time::Instant::now();
    db.build_plan_groups().await?;
    probe(db, "grouping (build_plan_groups)");
    let (tenders, islands, legacy_keys) = db.plan_summary().await?;
    report.tenders = tenders;
    report.islands = islands;
    let _ = db.checkpoint(store::CheckpointMode::Truncate).await;
    on_progress(Progress::Grouped { tenders, islands });
    eprintln!("[project] group: {} tenders in {:.1}s", report.tenders, t1.elapsed().as_secs_f64());

    // Phase 2 — fold whole Tenders out of the plan (in global fold order) and apply
    // them, holding at most one bounded working set at a time (issue 57/62). Both
    // folds produce byte-identical output; they differ only in how they READ the
    // parsed layer (see [`Phase2`]).
    let t2 = std::time::Instant::now();
    match phase2 {
        Phase2::ParsedFold => {
            // Stream whole-Tender batches from the plan in fold order and apply each,
            // reading only its notices' scattered parsed rows — the original path.
            let mut after = String::new();
            let mut batches_done = 0usize;
            let mut tenders_done = 0u64;
            loop {
                // Cooperative stop between apply batches (issue 256): each batch
                // committed whole, and every folded notice is already marked
                // projected, so the next run resumes with the unfolded remainder.
                if stop() {
                    report.stopped = true;
                    break;
                }
                let groups = db.next_plan_batch(&after, notice_batch).await?;
                let Some(last) = groups.last() else { break };
                after = last.group_key.clone();
                tenders_done += groups.len() as u64;
                report.applied.add(apply_plan_batch(db, &groups, now, rebuild).await?);
                // Heartbeat per batch so Phase 2 reports how far along it is (issue 59).
                on_progress(Progress::Applying {
                    tenders: tenders_done,
                    total: report.tenders,
                    versions: report.applied.versions_written,
                    leaf_rows: report.applied.leaf_rows,
                });
                batches_done += 1;
                if batches_done.is_multiple_of(CHECKPOINT_EVERY_BATCHES)
                    && let Err(e) = db.checkpoint(store::CheckpointMode::Truncate).await
                {
                    eprintln!("[project] checkpoint after batch {batches_done}: {e}");
                }
            }
        }
        Phase2::Buckets { shards } => {
            bucketed_fold(db, notice_batch, shards, now, rebuild, &mut report, &mut on_progress, stop)
                .await?;
        }
    }
    if report.stopped {
        // Stopped mid-fold. Everything applied is committed and marked projected;
        // retirement, plan teardown and the index builds belong to a COMPLETE fold
        // (retiring legacy keys against a partial fold would remove Tenders whose
        // members simply had not folded yet). A stopped rebuild keeps
        // `rebuild_in_progress` + the complete plan, which is exactly the
        // issue-60 salvage state — the next project job resumes Phase-2 from it.
        let _ = db.checkpoint(store::CheckpointMode::Truncate).await;
        eprintln!("[project] STOPPED at a checkpoint (issue 256) — partial tallies above are committed");
        return Ok(report);
    }
    // Retire any legacy Tender a late component-merge absorbed (its rows migrated
    // to the surviving key; here it gets `removed` change events).
    report.absorbed = db.retire_absorbed_legacy_tenders(&legacy_keys, now).await?;
    // Issue 278: the legacy retirement above scans only `ojs:%`, so a uuid-keyed or
    // island Tender a reparse regrouped away survived as a ghost on this full path
    // (the incremental path retires it via its touched set; the full fallback did
    // not). Retire those two shapes too — a no-op on a rebuild (fresh layer, every
    // key produced), corpus-wide on a non-rebuild. Runs before `clear_plan` so
    // `plan_notice` is still the authoritative produced set.
    report.absorbed += db.retire_regrouped_nonlegacy_tenders(now).await?;
    // Don't leave the transient plan in the durable DB between runs (issue 59).
    db.clear_plan().await?;
    // Rebuild the Organization indexes the bulk load ran without (issue 60) — one
    // sorted build each, now that every org and mention is in.
    if rebuild {
        let ti = std::time::Instant::now();
        db.build_organization_indexes().await?;
        db.build_tender_indexes().await?;
        let _ = db.checkpoint(store::CheckpointMode::Truncate).await;
        eprintln!("[project] org + tender indexes rebuilt in {:.1}s", ti.elapsed().as_secs_f64());
    }
    // Build the incremental change-set index now that (nearly) every parsed notice
    // is projected=1, so the partial index is near-empty and instant (issue 58).
    db.ensure_unprojected_index().await?;
    // Build the (entity_kind, cursor) change-feed index off the boot path (issue 61
    // finding 2): a no-op once present, and cheap on a fresh build's still-small
    // changes table vs a multi-minute CREATE INDEX at open on the full 80M rows.
    db.ensure_changes_entity_cursor_index().await?;
    // Reclaim the WAL left by the end-of-run index builds (issue 63): on a
    // rebuild+clear_changes the changes table was fully re-emitted during the fold,
    // so this (entity_kind, cursor) index is a big single-statement build, not the
    // "still-small" one the comment above assumes — its WAL must not sit as a tail.
    let _ = db.checkpoint(store::CheckpointMode::Truncate).await;
    probe(db, "end-of-run index builds");
    eprintln!("[project] apply: {} tenders in {:.1}s", report.tenders, t2.elapsed().as_secs_f64());
    eprintln!(
        "[project] done: {} notices → {} tenders ({} islands), {} versions, {} entities swept, \
         {} change rows in {:.1}s",
        report.notices,
        report.tenders,
        report.islands,
        report.applied.versions_written,
        report.applied.entities_swept,
        report.applied.changes,
        t0.elapsed().as_secs_f64()
    );
    Ok(report)
}

/// Phase 1 of the projection: stream the notice-parsed layer in id-ordered chunks
/// (issue 19) and, per notice, do the two things that need the whole corpus but
/// only a notice at a time — resolve its Organization mentions (in id order, so
/// canonical Organization identity is exactly what the whole-RAM projection
/// produced) and append its compact grouping *identity* to the on-disk plan.
/// Nothing per-notice heavy (facts, lots, results) is retained and the plan is on
/// disk (issue 59), so peak RAM is two read chunks — the one being swept and the
/// one being written (issue 175's pipeline) — independent of corpus. Returns
/// `(notices, mentions)`. Shared by the full projection and [`project_plan_only`].
///
/// Mentions are resolved BEFORE the chunk's plan rows are appended, so a full
/// `plan_notice` implies mentions are complete — the invariant the resume-from-plan
/// salvage relies on ([`store::Db::plan_is_complete`]).
async fn build_plan(
    db: &Db,
    now: i64,
    total: u64,
    mut on_progress: impl FnMut(Progress),
    stop: &(dyn Fn() -> bool + Sync),
) -> turso::Result<(u64, u64, bool, CitationGate)> {
    const READ_CHUNK: i64 = 10_000;
    let t0 = std::time::Instant::now();
    db.reset_plan().await?;
    let mut resolver = db.mention_resolver(Some(crate::crosswalk::canonical_key_flat), Some(crate::crosswalk::consortium_name), Some(crate::idgate::checksum_anchors), Some(crate::project::match_norm), Some(crate::crosswalk::legal_form_family), Some(crate::idgate::hard_scheme), crate::idgate::STOPLIST_CAP).await?;
    // The SWEEP half — the sequential parsed-layer read plus the pure-CPU
    // identity/mention extraction — runs on a prepare thread one chunk ahead of
    // the WRITER half (issue 175: phase 1 was measured pinned on one core while
    // the reader and writer each idled inside the same serial loop). The order
    // invariant lives in the writer half: `resolve_mentions` must see chunks in
    // notice-id order so canonical Organization identity is exactly what the
    // serial sweep produced — one producer + an in-order channel preserves that,
    // and the rendezvous handoff (capacity 0) bounds RAM at two chunks. Reading
    // one chunk ahead of the writes is safe: the sweep reads the notices/parsed
    // tables, the writer writes organizations/mentions/plan rows — disjoint.
    struct PlanChunk {
        rows: Vec<store::PlanRow>,
        mentions: Vec<Mention>,
        /// Issue 364's per-kind tally for this chunk's notices, summed by the
        /// writer half — the sweep is where `Ident::read` runs.
        citations: CitationGate,
    }
    let (tx, rx) = std::sync::mpsc::sync_channel::<turso::Result<PlanChunk>>(0);
    let readers = db.readers(1)?;
    let producer = {
        std::thread::spawn(move || {
            // Its own current-thread runtime and its own reader connection, the
            // pre-pass workers' pattern: the decode is CPU-bound, so a real
            // thread is what parallelises it. Between chunk queries the reader
            // holds no snapshot, so it never pins the WAL (the checkpoint.rs
            // idle-reader property).
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build plan-sweep runtime");
            rt.block_on(async move {
                let conn = match readers.get().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        return;
                    }
                };
                let mut after_id = 0i64;
                loop {
                    let mut chunk =
                        match Db::parsed_chunk_on(&conn, after_id, i64::MAX, READ_CHUNK).await {
                            Ok(chunk) => chunk,
                            Err(e) => {
                                let _ = tx.send(Err(e));
                                return;
                            }
                        };
                    normalise_de1(&mut chunk);
                    let Some((last, _)) = chunk.last() else { break };
                    after_id = last.id;
                    let mut mentions: Vec<Mention> = Vec::new();
                    let mut rows: Vec<store::PlanRow> = Vec::with_capacity(chunk.len());
                    let mut citations = CitationGate::default();
                    for (notice, parsed) in &chunk {
                        let ident = Ident::read(notice, parsed);
                        mentions.extend(NoticeState::mentions(ident.sdk01, notice.id, parsed));
                        citations.add(ident.citations);
                        rows.push(ident.into_plan_row());
                    }
                    if tx.send(Ok(PlanChunk { rows, mentions, citations })).is_err() {
                        return; // the writer half bailed on an error
                    }
                }
            });
        })
    };

    let (mut notices, mut mentions_total) = (0u64, 0u64);
    let mut citations = CitationGate::default();
    let mut chunks = 0usize;
    // The newest planned notice id — the legacy-adjacency attestation bound
    // (issue 58 v2). Chunks arrive id-ordered, but take the max rather than
    // trusting that.
    let mut max_planned = 0i64;
    let mut plan_err: Option<turso::Error> = None;
    let mut stopped = false;
    while let Ok(sent) = rx.recv() {
        // Cooperative stop (issue 256), between chunks — the same clean point the
        // WAL checkpoint uses. Everything inserted so far is committed; the reader
        // thread ends when its next send finds the receiver gone.
        if stop() {
            stopped = true;
            break;
        }
        let chunk = match sent {
            Ok(chunk) => chunk,
            Err(e) => {
                plan_err = Some(e);
                break;
            }
        };
        notices += chunk.rows.len() as u64;
        citations.add(chunk.citations);
        max_planned = chunk.rows.iter().map(|r| r.notice_id).fold(max_planned, i64::max);
        let resolved = match db.resolve_mentions(&mut resolver, &chunk.mentions, now).await {
            Ok(resolved) => resolved,
            Err(e) => {
                plan_err = Some(e);
                break;
            }
        };
        mentions_total += resolved.len() as u64;
        if let Err(e) = db.insert_plan(&chunk.rows).await {
            plan_err = Some(e);
            break;
        }
        // Heartbeat so a many-minute plan build is visibly alive (issue 59).
        on_progress(Progress::Planning { notices, total });
        // Keep the WAL bounded through the plan build too (issue 42/59): the
        // plan-row inserts are a burst; truncate at the clean point between chunks.
        chunks += 1;
        // Truncate every chunk to keep the WAL (and its in-RAM index) tiny, and
        // log the outcome when it does NOT fully reclaim — `busy` means a reader
        // pinned frames (a reader-pin), a large `wal_frames` residual over
        // `checkpointed` means the checkpoint could not keep up (throughput
        // divergence). Silence = healthy; the first line tells us which failure
        // mode a ballooning WAL is, in the first minute, not at OOM (issue 63).
        if chunks.is_multiple_of(PLAN_CHECKPOINT_EVERY) {
            match db.checkpoint_gated(store::CheckpointMode::Truncate).await {
                Ok(c) => {
                    let unreclaimed = c.busy || c.wal_frames > c.checkpointed + 20_000;
                    // Trace to the .diag.log every PLAN_DIAG_EVERY chunks (the WAL
                    // trend + busy flag), and ALWAYS when a checkpoint fails to fully
                    // reclaim — this is what tells us busy=true (reader-pin → the
                    // reader-gate) vs a climbing WAL at busy=false (throughput).
                    if unreclaimed || chunks.is_multiple_of(PLAN_DIAG_EVERY) {
                        db.log_diag(&format!(
                            "phase1 chunk={chunks} notices={notices} wal={}MB ckpt_busy={} wal_frames={} checkpointed={}",
                            db.wal_bytes().unwrap_or(0) / 1_048_576,
                            c.busy,
                            c.wal_frames,
                            c.checkpointed
                        ));
                    }
                    if unreclaimed {
                        eprintln!(
                            "[project] plan checkpoint after chunk {chunks} (notices={notices}): \
                             busy={} wal_frames={} checkpointed={} — WAL not fully reclaimed",
                            c.busy, c.wal_frames, c.checkpointed
                        );
                    }
                }
                Err(e) => {
                    db.log_diag(&format!("phase1 chunk={chunks} checkpoint ERROR: {e}"));
                    eprintln!("[project] plan checkpoint after chunk {chunks}: {e}");
                }
            }
        }
    }
    // Dropping the receiver unblocks a producer parked in `send` on the error
    // path; joining surfaces a producer panic instead of letting it read as a
    // short (silently truncated) plan.
    drop(rx);
    if let Err(panic) = producer.join() {
        std::panic::resume_unwind(panic);
    }
    if let Some(e) = plan_err {
        return Err(e);
    }
    db.finish_mention_resolver(resolver).await?;
    // issue 58 v2: this loop visited EVERY parsed notice, so the durable
    // adjacency rows insert_plan wrote are complete up to the newest planned
    // notice — attest it. (Reached only on a COMPLETE walk: an aborted plan
    // returned above, and a STOPPED one skips the attestation here — a stop is
    // precisely a walk that did not visit every notice, and attesting it would
    // be the issue-105 marked-without-a-row lie.)
    if !stopped {
        db.establish_legacy_adjacency(max_planned).await?;
    }
    eprintln!(
        "[project] plan: {notices} notices, {mentions_total} mentions resolved in {:.1}s{}",
        t0.elapsed().as_secs_f64(),
        if stopped { " — STOPPED at a checkpoint (issue 256)" } else { "" }
    );
    Ok((notices, mentions_total, stopped, citations))
}

/// Run only the interruptible PREFIX of a full rebuild — clear the canonical
/// layer, strip the Organization indexes, and build the whole grouping plan on
/// disk (Phase 1) — then STOP, leaving a complete plan on disk. A subsequent
/// `project(db, true)` detects that plan and RESUMES from it (grouping → Phase 2)
/// without redoing the expensive Phase 1 (issue 60 salvage). Runs with FK
/// enforcement off, like the full projection.
pub async fn project_plan_only(db: &Db) -> turso::Result<Report> {
    db.set_foreign_keys(false).await?;
    let result = async {
        db.clear_canonical().await?;
        db.strip_organization_indexes().await?;
        let total = db.parsed_notice_count().await?;
        let (notices, mentions, _, citations) =
            build_plan(db, store::now_unix(), total, |_| {}, &|| false).await?;
        Ok::<Report, turso::Error>(Report { notices, mentions, citations, ..Default::default() })
    }
    .await;
    let restored = db.set_foreign_keys(true).await;
    let report = result?;
    restored?;
    Ok(report)
}

/// Notices per adjacency-backfill chunk (issue 58 v2, step 2): the same size the
/// incremental fold streams at — full parsed layers for a chunk fit comfortably,
/// and the WAL is truncated between chunks.
const ADJACENCY_BACKFILL_CHUNK: i64 = 50_000;

/// How many notice IDS one chunk query may scan, however few of them match the
/// legacy pre-filter (issue 228). Without this bound the row LIMIT alone decides
/// when a query stops, so once the cursor passes the last legacy notice the
/// query cannot return early — it scans every remaining row to prove none is
/// left. On prod that tail was ~3.3M eForms notices swept in ONE query, ~40
/// minutes long, during which the sweep reported no progress at all and a
/// working job was indistinguishable from a wedged one. Windowing the id range
/// makes the sparse tail advance in visible steps and caps the I/O of any single
/// query; the row limit still bounds memory in the dense legacy eras, where a
/// window fills long before it is exhausted.
const ADJACENCY_BACKFILL_ID_WINDOW: i64 = 500_000;

/// What [`backfill_legacy_adjacency`] did: notices swept (legacy only), key rows
/// offered (self ∪ edges; pre-existing rows are ignored, not re-written), and the
/// coverage watermark it established.
pub struct LegacyAdjacencyBackfill {
    pub swept: u64,
    pub keys: u64,
    pub watermark: i64,
}

/// One progress tick from the sweep (issue 65, closing the field issue 228
/// deferred). Carries the count AND the position, kept as separate fields on
/// purpose: `swept` is a count of legacy notices and belongs where every other
/// job puts a count, while `cursor`/`target` are notice ids. Issue 228 declined
/// to merge them into `members_done` for exactly that reason — an id in a count's
/// field is the dishonest-signal shape the issue was filed about.
///
/// The pair is what makes a legacy-free tail legible: through the eForms era
/// `swept` is motionless while `cursor` climbs toward `target`, which is a
/// working job. Both motionless is a wedged one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepTick {
    /// Legacy notices found so far.
    pub swept: u64,
    /// Notice id the walk has reached.
    pub cursor: i64,
    /// Notice id it is walking to — captured before the walk.
    pub target: i64,
}

/// Issue 58 v2, step 2: populate `legacy_ojs_keys` for the STANDING corpus — the
/// notices folded before the choke-point writer existed — and establish the
/// coverage watermark. One bounded chunk at a time (the issue-42 shape), each
/// batch idempotent, so an interrupted run just re-runs.
///
/// Zero drift by construction: each notice's keys come from the SAME
/// [`Ident::read`] the plan build feeds the choke point, over the same parsed
/// layer. (The DE-1.x alias fold the plan build applies first is a no-op here:
/// its profiles are eForms, disjoint from the legacy set this sweeps.) The SQL
/// legacy pre-filter in [`Db::legacy_parsed_chunk`] only bounds the read;
/// `Ident::read`'s own `legacy` verdict decides what is written.
///
/// The sweep target is captured BEFORE the walk and established AFTER it
/// completes — jobs are queue-serialized, so nothing (re)parses into the swept
/// range mid-run; notices parsed after this job are covered by their own fold's
/// choke-point write + `advance_legacy_adjacency`.
///
/// The walk is bounded on BOTH axes (issue 228): each query reads at most
/// [`ADJACENCY_BACKFILL_CHUNK`] matching rows (memory, the issue-42 shape) and
/// scans at most [`ADJACENCY_BACKFILL_ID_WINDOW`] ids (I/O, and the reason a
/// legacy-free tail still advances). Termination is the cursor reaching the
/// captured target, not a chunk coming back empty — an empty chunk now means
/// only "no legacy notices in this window", which is the normal state of every
/// eForms-era window.
pub async fn backfill_legacy_adjacency(
    db: &Db,
    progress: impl FnMut(SweepTick),
) -> turso::Result<LegacyAdjacencyBackfill> {
    backfill_legacy_adjacency_windowed(db, ADJACENCY_BACKFILL_ID_WINDOW, progress).await
}

/// [`backfill_legacy_adjacency`] with the id window as a parameter. Production
/// always takes the default; a test passes a tiny window so a handful of fixture
/// notices span many windows — the only way to exercise the legacy-free tail
/// (issue 228) without half a million rows.
pub async fn backfill_legacy_adjacency_windowed(
    db: &Db,
    window: i64,
    mut progress: impl FnMut(SweepTick),
) -> turso::Result<LegacyAdjacencyBackfill> {
    debug_assert!(window > 0, "a non-positive window could not advance the cursor");
    let target = db.max_parsed_notice_id().await?;
    let mut cursor = 0i64;
    let mut swept = 0u64;
    let mut keys = 0u64;
    while cursor < target {
        let hi = cursor.saturating_add(window).min(target);
        let chunk = db.legacy_parsed_chunk(cursor, hi, ADJACENCY_BACKFILL_CHUNK).await?;
        let mut batch: Vec<(i64, i64)> = Vec::new();
        for (notice, parsed) in &chunk {
            let ident = Ident::read(notice, parsed);
            if !ident.legacy {
                continue;
            }
            swept += 1;
            for k in ident.ojs_self.into_iter().chain(ident.ojs_edges) {
                batch.push((encode_ojs(k), notice.id));
            }
        }
        keys += batch.len() as u64;
        db.insert_legacy_ojs_keys(&batch).await?;
        // A full chunk may have stopped short of the window's end on the row
        // limit, so resume from the last row read; otherwise the whole window is
        // swept and the cursor takes its end. Both advance strictly, so the walk
        // cannot stall — the bug this replaced could only stall by scanning
        // ahead invisibly, never by looping.
        cursor = match chunk.last() {
            Some((n, _)) if chunk.len() as i64 >= ADJACENCY_BACKFILL_CHUNK => n.id,
            _ => hi,
        };
        progress(SweepTick { swept, cursor, target });
        let _ = db.checkpoint(store::CheckpointMode::Truncate).await;
    }
    db.establish_legacy_adjacency(target).await?;
    Ok(LegacyAdjacencyBackfill { swept, keys, watermark: target })
}

/// The daily projection: re-derive only the Tenders TOUCHED by notices parsed
/// since the last run, so cost scales with the delta, not the corpus (issue 58).
///
/// The reconcile path (`apply_tenders`) is already a per-Tender natural-key
/// upsert with no global deletes, so feeding Phase 2 only the touched Tenders
/// leaves every untouched Tender byte-identical and produces the touched ones
/// exactly as a full non-rebuild projection would. Incremental is therefore only
/// about SCOPING: (1) the change-set is the unprojected parsed notices (the
/// `notices.projected` watermark); (2) the plan is seeded with just the touched
/// Tenders' full notice sets, so the same grouping SQL runs over a bounded set;
/// (3) retirement is scoped to the touched set.
///
/// LEGACY CLOSURE (issue 58 v2, step 3): the transitive OJS union-find needs the
/// whole existing edge graph, persisted in `legacy_ojs_keys` (written at the
/// plan-build choke point; the standing corpus swept by the backfill job). When
/// `legacy_adjacency.watermark` attests coverage, a legacy delta expands to its
/// OJS component via [`legacy_closure`] — keys → notices → tenders → member
/// notices → keys, to a fixpoint — and the run stays scoped. Without the
/// attestation (watermark 0, a coverage gap, or an over-cap component) it falls
/// back to a full non-rebuild projection exactly as v1 did, loudly; the full
/// pass re-establishes the watermark, so the fallback self-heals.
pub async fn project_incremental(db: &Db) -> turso::Result<Report> {
    project_incremental_stoppable(db, &|| false).await
}

/// [`project_incremental`] with the cooperative stop (issue 256): the same wipe
/// guards and FK toggling, the stop polled between fold batches — and forwarded
/// into the whole-corpus fallback, so a cancel reaches whichever path the delta
/// routed to.
pub async fn project_incremental_stoppable(
    db: &Db,
    stop: &(dyn Fn() -> bool + Sync),
) -> turso::Result<Report> {
    project_incremental_observed_stoppable(db, |_| {}, stop).await
}

/// [`project_incremental_stoppable`] with the Progress events surfaced (issue
/// 262): the plan build over a re-parse-scale delta runs for tens of minutes,
/// and without this the job's phase record read `None` the whole time — a
/// 2.7M-notice delta was watched as dead air, and a cancel took ~17 minutes to
/// find a checkpoint. The supervisor maps these to the same durable phase
/// record the full-projection path earns (issue 65).
pub async fn project_incremental_observed_stoppable(
    db: &Db,
    on_progress: impl FnMut(Progress),
    stop: &(dyn Fn() -> bool + Sync),
) -> turso::Result<Report> {
    db.reload_rates_lookup().await?; // ADR-0014: one rates snapshot per run
    let pre_populated = wipe_guard_pre(db, false).await?;
    db.set_foreign_keys(false).await?;
    let result =
        project_incremental_chunked_observed(db, INCREMENTAL_CHUNK, None, on_progress, stop).await;
    let restored = db.set_foreign_keys(true).await;
    let report = result?;
    restored?;
    wipe_guard_post(db, pre_populated).await?;
    Ok(report)
}

/// Notices per Phase-1 chunk of the incremental fold (issue 81). The incremental
/// path used to load the WHOLE delta's parsed layer (changed + touched-Tender
/// notices) into RAM before planning — O(delta), which OOM'd on a 300k+ reclaim.
/// It now streams the plan build in id-ordered chunks of this size; the daily
/// delta is far smaller than one chunk, so it stays effectively single-pass.
const INCREMENTAL_CHUNK: usize = 50_000;

/// Planned notices above which the incremental fold reads the parsed layer with the
/// BUCKETED sweep ([`Phase2::Buckets`]) instead of [`Phase2::ParsedFold`] (issue 91).
///
/// `ParsedFold` re-reads each fold batch's notices by an `IN (…)` over ids scattered
/// across the corpus — random rowid seeks into the cold multi-hundred-GB notice
/// tables (the issue-62 read storm). That is the right trade for a daily delta of a
/// few hundred notices, where a whole-corpus sweep would be absurd; it is the wrong
/// trade for a re-fold, where the scattered re-read dominates. Measured on prod: a
/// 7 211-notice delta took 8h21m through `ParsedFold`, of which 7h13m was
/// post-grouping, while the bucketed sweep resolved the whole 14.1M-notice parsed
/// layer in 7h13m and folded 8.1M Tenders. Crossover is therefore well below the
/// corpus size; 100k keeps a normal daily on the cheap path with a wide margin and
/// routes any re-fold onto the path that is proven at full scale.
///
/// Both folds are byte-identical by construction (see [`Phase2`]) — this only
/// chooses HOW the parsed layer is read — and `write_shard` skips notices absent
/// from the plan, so the bucketed sweep honours a SCOPED plan unchanged.
const INCREMENTAL_BUCKET_THRESHOLD: usize = 100_000;

/// The incremental projection with a bounded, streamed Phase 1 (issue 81). Grouping
/// and Phase 2 stay GLOBAL (one plan, one fold) so the output — surrogate ids
/// included — is byte-identical to the old whole-delta path; only the parsed-layer
/// read + plan build are chunked, so peak RAM is flat vs delta size instead of
/// O(delta). `chunk_size` is exposed for the batch-invariance test.
///
/// Phase 2 picks its fold by plan size (see [`INCREMENTAL_BUCKET_THRESHOLD`]);
/// [`project_incremental_chunked_phase2`] forces one, for the invariance test.
pub async fn project_incremental_chunked(db: &Db, chunk_size: usize) -> turso::Result<Report> {
    project_incremental_chunked_phase2(db, chunk_size, None).await
}

/// As [`project_incremental_chunked`], with an explicit Phase-2 fold — `None` picks
/// by plan size, `Some(_)` forces one. The forcing form exists so the fold-source
/// invariance test can prove BOTH folds produce a byte-identical canonical layer
/// over the same SCOPED plan, at a corpus size far below the routing threshold.
/// Closure notices above which the incremental fold gives up on scoping and takes
/// the full path (issue 58 v2). A pathological component (issue 68's class — a
/// hub key referenced by hundreds of thousands of notices) must degrade to
/// today's full-projection behavior, never to a wrong scope; and past this size
/// a full pass is not meaningfully more expensive anyway.
const LEGACY_CLOSURE_CAP: usize = 500_000;

/// Expand legacy seed keys to their full OJS component via the durable adjacency
/// (issue 58 v2, step 3): keys → notices (`legacy_ojs_keys`), notices → tenders
/// (`caused_by`), tenders → member notices, members → keys; repeat to a fixpoint.
/// Seeds must be the UNION of every legacy delta notice's keys (self ∪ edges), so
/// intra-delta joins are pre-joined and the delta's own rows need not be durable
/// yet (they are written later, at the pass-2 choke point).
///
/// Returns `Ok((notices, tenders))` — existing notice ids to add to the plan and
/// existing tender ids whose grouping may change (both sorted) — or `Err(reason)`
/// when the closure must not be trusted: watermark never established, a coverage
/// gap (see [`Db::projected_parsed_above`]), or an over-cap component. The caller
/// then takes the full path, which re-establishes coverage.
async fn legacy_closure(
    db: &Db,
    seeds: &[i64],
) -> turso::Result<Result<(Vec<i64>, Vec<i64>), String>> {
    legacy_closure_capped(db, seeds, LEGACY_CLOSURE_CAP).await
}

/// As [`legacy_closure`], with an explicit cap. Exposed so the cap test can
/// drive a tiny cap over a small component; production uses
/// [`LEGACY_CLOSURE_CAP`].
pub async fn legacy_closure_capped(
    db: &Db,
    seeds: &[i64],
    cap: usize,
) -> turso::Result<Result<(Vec<i64>, Vec<i64>), String>> {
    let watermark = db.legacy_adjacency_watermark().await?;
    if watermark == 0 {
        return Ok(Err("legacy adjacency never established".into()));
    }
    if let Some(id) = db.projected_parsed_above(watermark).await? {
        return Ok(Err(format!(
            "legacy adjacency coverage gap: projected parsed notice {id} above watermark {watermark}"
        )));
    }
    let mut seen_keys: std::collections::HashSet<i64> = seeds.iter().copied().collect();
    let mut frontier: Vec<i64> = seeds.to_vec();
    let mut notices = std::collections::BTreeSet::new();
    let mut tenders = std::collections::BTreeSet::new();
    let mut hops = 0usize;
    while !frontier.is_empty() {
        hops += 1;
        let mut new_notices: Vec<i64> = db
            .legacy_notices_for_keys(&frontier)
            .await?
            .into_iter()
            .filter(|id| notices.insert(*id))
            .collect();
        if new_notices.is_empty() {
            break;
        }
        // Membership expansion: a hop notice's Tender re-derives IN FULL, so its
        // other member notices join the closure (and contribute their keys) too.
        let new_tenders: Vec<i64> = db
            .tenders_for_notice_ids(&new_notices)
            .await?
            .into_iter()
            .filter(|t| tenders.insert(*t))
            .collect();
        new_notices.extend(
            db.notice_ids_for_tenders(&new_tenders)
                .await?
                .into_iter()
                .filter(|id| notices.insert(*id)),
        );
        if notices.len() > cap {
            return Ok(Err(format!(
                "legacy closure exceeds cap ({} notices > {cap})",
                notices.len()
            )));
        }
        frontier = db
            .legacy_keys_for_notices(&new_notices)
            .await?
            .into_iter()
            .filter(|k| seen_keys.insert(*k))
            .collect();
    }
    eprintln!(
        "[project] legacy closure: {} seed keys → {} notices, {} tenders in {hops} hops",
        seeds.len(),
        notices.len(),
        tenders.len()
    );
    Ok(Ok((notices.into_iter().collect(), tenders.into_iter().collect())))
}

pub async fn project_incremental_chunked_phase2(
    db: &Db,
    chunk_size: usize,
    phase2: Option<Phase2>,
) -> turso::Result<Report> {
    project_incremental_chunked_phase2_stoppable(db, chunk_size, phase2, &|| false).await
}

/// [`project_incremental_chunked_phase2`] with the cooperative stop (issue 256);
/// polled between fold batches, and forwarded into the whole-corpus fallback so a
/// cancel reaches whichever path the delta routed to.
pub async fn project_incremental_chunked_phase2_stoppable(
    db: &Db,
    chunk_size: usize,
    phase2: Option<Phase2>,
    stop: &(dyn Fn() -> bool + Sync),
) -> turso::Result<Report> {
    project_incremental_chunked_observed(db, chunk_size, phase2, |_| {}, stop).await
}

/// The chunked incremental with its Progress surfaced and its plan-build loops
/// stoppable (issue 262). Both passes emit [`Progress::Planning`] per chunk and
/// poll the stop flag per chunk, so a cancel lands within one chunk's work
/// (~[`INCREMENTAL_CHUNK`] notices) instead of only at the fold. A stop during
/// pass 2 abandons the partial plan (cleared; rebuilt from the same delta next
/// run — mention resolution is idempotent) and deliberately does NOT advance
/// the legacy-adjacency watermark: that attestation is only true of a COMPLETED
/// plan build.
pub async fn project_incremental_chunked_observed(
    db: &Db,
    chunk_size: usize,
    phase2: Option<Phase2>,
    mut on_progress: impl FnMut(Progress),
    stop: &(dyn Fn() -> bool + Sync),
) -> turso::Result<Report> {
    let changed = db.unprojected_parsed_notice_ids().await?;
    if changed.is_empty() {
        return Ok(Report::default());
    }
    // Issue 305: when the change-set's LEGACY portion alone already exceeds the
    // closure cap, the identity pass can only discover the fallback the COUNT
    // below implies (the epoch refold paid a 98-minute pass-1 for that
    // discovery). Not a strict theorem — a keyless legacy notice seeds no
    // closure — but the full path is always correct, and a >cap legacy delta
    // is whole-corpus-shaped work either way. One indexed COUNT, negligible on
    // the daily delta.
    let legacy_changed = db.unprojected_legacy_notice_count().await?;
    if legacy_changed as usize > LEGACY_CLOSURE_CAP {
        eprintln!(
            "[project] INCREMENTAL → FULL fallback BEFORE identity pass: {legacy_changed} \
             un-projected legacy notices exceed the closure cap ({LEGACY_CLOSURE_CAP}) \
             (issue 305); re-projecting the whole corpus"
        );
        let mut stderr = stderr_progress_sink();
        return project_with_progress_phase2_stoppable(
            db,
            false,
            APPLY_NOTICE_BATCH,
            Phase2::Buckets { shards: None },
            |p| {
                stderr(p);
                on_progress(p);
            },
            stop,
        )
        .await;
    }
    let chunk_size = chunk_size.max(1);
    let now = store::now_unix();
    let t0 = std::time::Instant::now();
    // Per-stage timings (issue 90). This path used to print NOTHING between its
    // start and its one-line summary, so a run that wedged for hours gave no clue
    // WHICH stage was wedged — the 2026-07-30 reclaim and the 2026-08-01 eForms-DE
    // re-fold both had to be diagnosed from `/proc` I/O counters and WAL size. Every
    // stage boundary now prints elapsed seconds, so the next stall names itself.
    let mut mark = std::time::Instant::now();
    let mut stage = |label: &str| {
        eprintln!("[project] incremental stage {label}: {:.1}s", mark.elapsed().as_secs_f64());
        mark = std::time::Instant::now();
    };
    eprintln!("[project] incremental: {} changed notices", changed.len());

    // Pass 1 (streamed): read the changed notices' grouping identity in id-ordered
    // chunks — collect keyed keys and legacy OJS seed keys — without holding the
    // whole delta's parsed layer.
    let mut new_keyed_keys: Vec<String> = Vec::new();
    let mut legacy_seed_keys: Vec<i64> = Vec::new();
    let mut legacy_delta = 0usize;
    let mut scanned = 0u64;
    for chunk in changed.chunks(chunk_size) {
        // Stop per chunk (issue 262): nothing durable is written in pass 1, so
        // an immediate return is the checkpoint.
        if stop() {
            return Ok(Report { stopped: true, ..Report::default() });
        }
        scanned += chunk.len() as u64;
        on_progress(Progress::Identity { notices: scanned, total: changed.len() as u64 });
        let mut batch = db.parsed_by_ids(chunk).await?;
        normalise_de1(&mut batch);
        for (notice, parsed) in &batch {
            let ident = Ident::read(notice, parsed);
            if ident.legacy {
                legacy_delta += 1;
                legacy_seed_keys.extend(
                    ident.ojs_self.iter().chain(ident.ojs_edges.iter()).copied().map(encode_ojs),
                );
            }
            if let Some(key) = &ident.procedure_key {
                new_keyed_keys.push(key.clone());
            }
        }
    }
    new_keyed_keys.sort_unstable();
    new_keyed_keys.dedup();
    legacy_seed_keys.sort_unstable();
    legacy_seed_keys.dedup();
    stage(&format!(
        "pass-1 identity ({} new keyed keys, {legacy_delta} legacy notices, {} seed keys)",
        new_keyed_keys.len(),
        legacy_seed_keys.len()
    ));

    // A legacy delta groups transitively with EXISTING notices through shared OJS
    // keys — expand to the whole component via the durable adjacency, or fall back
    // to the full path when the closure cannot be trusted (never established, a
    // coverage gap, an over-cap component). A keyless legacy notice (no self
    // number, no refs) shares nothing and needs no closure — the empty-seed case
    // is correct without the gate.
    let (closure_ids, closure_tenders) = if legacy_seed_keys.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        match legacy_closure(db, &legacy_seed_keys).await? {
            Ok(scoped) => scoped,
            Err(reason) => {
                eprintln!(
                    "[project] INCREMENTAL → FULL fallback: {reason} (issue 58 v2); \
                     re-projecting the whole corpus"
                );
                // The caller's sink rides along (issue 262): an era-scale reparse
                // delta routinely exceeds the closure cap — r208's pulled a 2.9M
                // closure — so the fallback IS the common path for the biggest
                // folds, and it must not shed the phase record on the way through.
                let mut stderr = stderr_progress_sink();
                return project_with_progress_phase2_stoppable(
                    db,
                    false,
                    APPLY_NOTICE_BATCH,
                    Phase2::Buckets { shards: None },
                    |p| {
                        stderr(p);
                        on_progress(p);
                    },
                    stop,
                )
                .await;
            }
        }
    };
    if !closure_ids.is_empty() || !closure_tenders.is_empty() {
        stage(&format!(
            "legacy closure ({} notices, {} tenders)",
            closure_ids.len(),
            closure_tenders.len()
        ));
    }

    // Expand to the touched EXISTING Tenders and their full notice sets; the plan
    // covers changed ∪ touched-existing ∪ legacy closure, in one global id order.
    let mut touched_tenders = db.touched_existing_tender_ids(&changed, &new_keyed_keys).await?;
    touched_tenders.extend(closure_tenders);
    touched_tenders.sort_unstable();
    touched_tenders.dedup();
    let existing_ids = db.notice_ids_for_tenders(&touched_tenders).await?;
    let changed_set: std::collections::HashSet<i64> = changed.iter().copied().collect();
    let mut all_ids: Vec<i64> = changed
        .iter()
        .copied()
        .chain(existing_ids.iter().copied().filter(|id| !changed_set.contains(id)))
        .chain(closure_ids.iter().copied().filter(|id| !changed_set.contains(id)))
        .collect();
    all_ids.sort_unstable();
    all_ids.dedup();
    stage(&format!(
        "touched expansion ({} touched Tenders → {} planned notices)",
        touched_tenders.len(),
        all_ids.len()
    ));

    // Pass 2 (streamed): build the ONE plan in id-ordered chunks so the org bulk-load
    // stays sequential and RAM bounded; mentions resolve only for the changed notices
    // (existing notices' mentions are already recorded, bound in Phase 2 by
    // `mentions_by_ids`), in global id order so org ids match a whole-delta pass.
    db.reset_plan().await?;
    let mut resolver = db.mention_resolver(Some(crate::crosswalk::canonical_key_flat), Some(crate::crosswalk::consortium_name), Some(crate::idgate::checksum_anchors), Some(crate::project::match_norm), Some(crate::crosswalk::legal_form_family), Some(crate::idgate::hard_scheme), crate::idgate::STOPLIST_CAP).await?;
    let mut report = Report::default();
    let mut planned = 0u64;
    for chunk in all_ids.chunks(chunk_size) {
        // Stop per chunk (issue 262). The partial plan is abandoned below —
        // cleared, and rebuilt from the same (unshrunk) delta next run; the
        // resolver's writes are idempotent, so finishing it loses nothing.
        if stop() {
            report.stopped = true;
            break;
        }
        planned += chunk.len() as u64;
        on_progress(Progress::Planning { notices: planned, total: all_ids.len() as u64 });
        let mut parsed = db.parsed_by_ids(chunk).await?;
        normalise_de1(&mut parsed);
        parsed.sort_by_key(|(n, _)| n.id);
        let mut rows: Vec<store::PlanRow> = Vec::with_capacity(parsed.len());
        let mut mentions: Vec<Mention> = Vec::new();
        for (notice, p) in &parsed {
            let ident = Ident::read(notice, p);
            if changed_set.contains(&notice.id) {
                mentions.extend(NoticeState::mentions(ident.sdk01, notice.id, p));
            }
            // Issue 364: counted in the PLAN build only — pass 1 above reads the
            // same notices' identity again, and counting there too would double
            // every citation in the delta.
            report.citations.add(ident.citations);
            rows.push(ident.into_plan_row());
        }
        db.insert_plan(&rows).await?;
        report.mentions += db.resolve_mentions(&mut resolver, &mentions, now).await?.len() as u64;
    }
    report.wall = store::Db::wall_counts(&resolver);
    db.finish_mention_resolver(resolver).await?;
    if report.stopped {
        // A stopped pass 2 wrote a PARTIAL plan: clear it, and do NOT advance
        // the adjacency watermark — its attestation ("every parsed notice above
        // the watermark has durable key rows") is only true of a completed
        // build (issue 262). Nothing was folded; the whole delta re-enters.
        db.clear_plan().await?;
        eprintln!("[project] incremental STOPPED at a checkpoint during plan build (issue 262)");
        return Ok(report);
    }
    // issue 58 v2: the delta is ALL unprojected parsed notices, so after this
    // plan build every parsed notice above the old adjacency watermark has its
    // durable key rows (insert_plan wrote them) — advance the attestation. A
    // never-established watermark (0) stays 0: only a full pass may set the base.
    if let Some(max) = all_ids.last() {
        db.advance_legacy_adjacency(*max).await?;
    }
    stage(&format!("pass-2 plan build ({} mentions resolved)", report.mentions));

    // Group the whole plan (same SQL as a full run — over the touched set only).
    db.build_plan_groups().await?;
    let (tenders, islands) = db.plan_counts().await?;
    report.tenders = tenders;
    report.islands = islands;
    on_progress(Progress::Grouped { tenders, islands });
    stage(&format!("grouping ({tenders} Tenders, {islands} islands)"));

    // Retire any touched Tender the new plan did not reproduce (island→keyed
    // upgrade, etc.) BEFORE applying, so a regrouped notice's old Tender is gone.
    report.absorbed = db.retire_regrouped_tenders(&touched_tenders, now).await?;
    stage(&format!("retire regrouped ({} retired)", report.absorbed));

    // Phase 2: fold + apply the touched Tenders, reading the parsed layer the way
    // that suits the plan's size (see [`INCREMENTAL_BUCKET_THRESHOLD`]). Either fold
    // upserts each Tender by natural key and marks its notices projected, so
    // untouched Tenders are never read or written — and both produce the identical
    // canonical layer, so this choice is purely about read cost.
    let phase2 = phase2.unwrap_or(if all_ids.len() >= INCREMENTAL_BUCKET_THRESHOLD {
        Phase2::Buckets { shards: None }
    } else {
        Phase2::ParsedFold
    });
    eprintln!("[project] incremental phase 2: {phase2:?} over {} planned notices", all_ids.len());
    match phase2 {
        Phase2::ParsedFold => {
            let mut after = String::new();
            let mut tenders_done = 0u64;
            loop {
                // Cooperative stop between batches (issue 256): folded notices are
                // already `projected = 1`, the rest re-enter the next delta.
                if stop() {
                    report.stopped = true;
                    break;
                }
                let groups = db.next_plan_batch(&after, APPLY_NOTICE_BATCH).await?;
                let Some(last) = groups.last() else { break };
                after = last.group_key.clone();
                tenders_done += groups.len() as u64;
                report.applied.add(apply_plan_batch(db, &groups, now, false).await?);
                on_progress(Progress::Applying {
                    tenders: tenders_done,
                    total: tenders,
                    versions: report.applied.versions_written,
                    leaf_rows: report.applied.leaf_rows,
                });
                // Heartbeat per batch: without it a wedged fold is indistinguishable
                // from a slow one (issue 90).
                eprintln!(
                    "[project] incremental fold: {tenders_done}/{tenders} Tenders folded, \
                     {} versions written",
                    report.applied.versions_written
                );
            }
        }
        Phase2::Buckets { shards } => {
            bucketed_fold(
                db,
                APPLY_NOTICE_BATCH,
                shards,
                now,
                false,
                &mut report,
                |p| {
                    if let Progress::Applying { tenders, total, versions, leaf_rows } = p {
                        // `versions` is the load-bearing number: `tenders` climbs to
                        // completion even if every fold early-returns (issue 99).
                        eprintln!(
                            "[project] incremental fold: {tenders}/{total} Tenders folded, \
                             {versions} versions written, {leaf_rows} leaf rows"
                        );
                    }
                    on_progress(p);
                },
                stop,
            )
            .await?;
        }
    }
    stage("phase 2 fold + apply");
    if report.stopped {
        // Stopped between batches: folded notices are marked, unfolded ones stay
        // `projected = 0` and re-enter the next delta whole. The plan is cleared —
        // the next incremental rebuilds it from the (smaller) remaining delta.
        db.clear_plan().await?;
        report.notices = changed.len() as u64;
        eprintln!("[project] incremental STOPPED at a checkpoint (issue 256)");
        return Ok(report);
    }
    db.clear_plan().await?;
    report.notices = changed.len() as u64;

    // Keep the change-set + change-feed indexes present for the next run (issues
    // 58/61) — no-ops once built.
    db.ensure_unprojected_index().await?;
    db.ensure_changes_entity_cursor_index().await?;
    let _ = db.checkpoint(store::CheckpointMode::Truncate).await;
    eprintln!(
        "[project] incremental: {} changed → {} touched Tenders ({} retired) in {:.1}s",
        report.notices,
        report.tenders,
        report.absorbed,
        t0.elapsed().as_secs_f64()
    );
    Ok(report)
}

/// Fold one streamed batch of whole Tenders and reconcile them. Reads only the
/// batch's notices — their parsed form and the Organizations Phase 1 resolved —
/// rebuilds each notice's canonical state, binds it, folds each group's chain,
/// and applies.
async fn apply_plan_batch(
    db: &Db,
    groups: &[store::PlanGroup],
    now: i64,
    rebuild: bool,
) -> turso::Result<store::Applied> {
    // Read the batch's parsed layer + mentions in ascending notice_id order, not
    // fold (group_key) order: the batch's notices are scattered across id space, and
    // notices/notice_sections/values are all keyed by notice_id, so an unsorted
    // (fold-order) read is ~random rowid seeks into the cold multi-hundred-GB notice
    // tables — the Phase-2 bottleneck at scale. A sorted read is a forward sweep
    // (read-ahead friendly). The fold below is unaffected: it indexes `states` by id
    // and iterates each group's own notice_ids.
    let mut ids: Vec<i64> = groups.iter().flat_map(|g| g.notice_ids.iter().copied()).collect();
    ids.sort_unstable();
    let mut parsed = db.parsed_by_ids(&ids).await?;
    normalise_de1(&mut parsed);
    let orgs = db.mentions_by_ids(&ids).await?;

    let mut states: HashMap<i64, NoticeState> = HashMap::with_capacity(parsed.len());
    for (notice, parsed) in &parsed {
        let mut state = NoticeState::read(notice, parsed);
        state.bind_organizations(orgs.get(&notice.id).unwrap_or(&HashMap::new()));
        states.insert(notice.id, state);
    }

    let projections: Vec<TenderProjection> = groups
        .iter()
        .map(|group| {
            let chain: Vec<&NoticeState> =
                group.notice_ids.iter().filter_map(|id| states.get(id)).collect();
            let island_notice_id = group
                .group_key
                .strip_prefix("island:")
                .and_then(|id| id.parse::<i64>().ok());
            let procedure_key = island_notice_id.is_none().then(|| group.group_key.clone());
            TenderProjection {
                source: primary_source(&group.sources),
                procedure_key,
                island_notice_id,
                kind: kind_of(group.first_subtype.as_deref()).to_owned(),
                versions: fold(&chain),
            }
        })
        .collect();
    let applied = db.apply_tenders(&projections, now, rebuild).await?;
    // Mark every applied notice as folded into the canonical layer (issue 58) —
    // whether or not its Tender changed — so the next incremental run skips it.
    db.mark_projected(&ids).await?;
    Ok(applied)
}

// ------------------------------------------------------- bucketed Phase-2 fold
//
// The read fix (issue 62). [`apply_plan_batch`] reads each fold batch's parsed
// layer in group_key (fold) order — random rowid seeks across ten notice-keyed
// tables, ~50ms/notice at 12.4M scale (~7 days). The bucketed fold instead reads
// the parsed layer ONCE, sequentially in notice_id order (a forward sweep), resolves
// each notice's state, and spills it to an order-preserving on-disk bucket; then it
// folds each bucket sorted in RAM. The buckets partition the plan by contiguous
// group_key ranges (whole groups, never split), taken in fold order — so the global
// fold order, and thus every surrogate id, is identical to [`Phase2::ParsedFold`].
// Peak RAM is two buckets: the one the writer is applying plus the one the prepare
// thread is folding (issue 175's pipeline; the rendezvous handoff stops it there).

/// Orchestrate the bucketed Phase-2 fold: compute the fold-order bucket boundaries,
/// stream the parsed layer once into buckets, then fold each bucket in order —
/// preparation pipelined one bucket ahead of the single-writer apply. See the
/// module note above.
async fn bucketed_fold(
    db: &Db,
    notice_batch: usize,
    shards: Option<usize>,
    now: i64,
    rebuild: bool,
    report: &mut Report,
    mut on_progress: impl FnMut(Progress),
    stop: &(dyn Fn() -> bool + Sync),
) -> turso::Result<()> {
    let dir = db.scratch_dir("proj_buckets");
    let _ = std::fs::remove_dir_all(&dir);

    // Contiguous group_key boundaries partitioning the plan into ~`notice_batch`
    // -notice buckets in fold order. A group never splits across a boundary
    // (`next_plan_batch` stops before opening a group past the budget).
    let boundaries = bucket_boundaries(db, notice_batch).await?;
    if boundaries.is_empty() {
        return Ok(()); // Empty plan — nothing to fold.
    }

    // Pre-pass (issue 66): shard the parsed read across `k` id-stripe workers, each
    // resolving state and spilling to its OWN `shard{s}_bucket{b}.bin` files. A group
    // still routes to ONE logical bucket `b` (routing is by group_key, not id), so a
    // group split across id stripes just lands in different physical files of the
    // same `b`; the fold re-concatenates and sorts them. All workers must finish
    // before the fold — a group can span any stripe (barrier is the `join` inside).
    let fd_budget = raise_fd_limit();
    let k = write_buckets_sharded(
        db,
        &boundaries,
        &dir,
        worker_count(shards, boundaries.len(), fd_budget),
        &mut on_progress,
    )
    .await?;

    // Fold pass: each bucket in order. The APPLY stays serial — `apply_tenders`
    // assigns surrogate ids in global fold order through the single writer
    // (byte-identity, ADR-0001) — but a bucket's PREPARATION (shard-file read +
    // sort + the pure-CPU fold, none of which touches the DB) overlaps it from a
    // prepare thread (issue 175): on the 12h prod fold the writer and the fold
    // CPU each idled while the other ran. The rendezvous channel (capacity 0)
    // hands bucket N+1 over exactly as the writer finishes N, so at most two
    // buckets are in RAM — the prepare thread cannot run ahead of the writer.
    let n_buckets = boundaries.len();
    let (tx, rx) = std::sync::mpsc::sync_channel::<Prepared>(0);
    let producer = {
        let dir = dir.clone();
        std::thread::spawn(move || {
            for b in 0..n_buckets {
                let mut rows = read_bucket_shards(&dir, b, k);
                rows.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
                if tx.send(fold_rows(&rows)).is_err() {
                    return; // the apply side bailed on an error — stop preparing
                }
            }
        })
    };

    // The fold announces itself at the pre-pass barrier (issue 339). The grouping
    // step set the phase to `folding 0/N`, but the pre-pass ticks overwrote it
    // with `pre-pass <count>`, and the first per-bucket tick below only lands
    // once a WHOLE bucket has been applied — the biggest chains first, so on
    // the 2026-09-02 campaign fold that was 17 minutes of a job row saying
    // "pre-pass" over a stalled count while the WAL grew by 26 GB: the exact
    // shape of the issue-42/53 runaway, on a healthy run. One zero-tick here
    // and the record names the stage the moment it begins. (A bucket is one
    // `apply_tenders` transaction by design, so this does not try to tick
    // inside it.)
    on_progress(Progress::Applying { tenders: 0, total: report.tenders, versions: report.applied.versions_written, leaf_rows: report.applied.leaf_rows });
    let mut tenders_done = 0u64;
    let mut fold_err: Option<turso::Error> = None;
    for b in 0..n_buckets {
        // Cooperative stop between buckets (issue 256): the bucket just applied is
        // committed and its notices marked projected; dropping the receiver below
        // unparks the prepare thread exactly as the error path does.
        if stop() {
            report.stopped = true;
            break;
        }
        // A recv error means the producer died mid-run; its panic is surfaced by
        // the join below rather than being swallowed into a short row count.
        let Ok(prepared) = rx.recv() else { break };
        let groups = prepared.projections.len() as u64;
        let applied = match db.apply_tenders(&prepared.projections, now, rebuild).await {
            Ok(applied) => applied,
            Err(e) => {
                fold_err = Some(e);
                break;
            }
        };
        // Mark every folded notice projected (issue 58), exactly as apply_plan_batch does.
        if let Err(e) = db.mark_projected(&prepared.applied_ids).await {
            fold_err = Some(e);
            break;
        }
        report.applied.add(applied);
        tenders_done += groups;
        on_progress(Progress::Applying {
            tenders: tenders_done,
            total: report.tenders,
            versions: report.applied.versions_written,
            leaf_rows: report.applied.leaf_rows,
        });
        if (b + 1).is_multiple_of(CHECKPOINT_EVERY_BATCHES)
            && let Err(e) = db.checkpoint(store::CheckpointMode::Truncate).await
        {
            eprintln!("[project] checkpoint after bucket {b}: {e}");
        }
    }
    // Dropping the receiver unblocks a producer parked in `send` on the error
    // path; joining surfaces a producer panic (corrupt bucket file) instead of
    // letting it read as a silently short fold.
    drop(rx);
    if let Err(panic) = producer.join() {
        std::panic::resume_unwind(panic);
    }
    if let Some(e) = fold_err {
        return Err(e);
    }
    // The buckets are transient scratch — never leave them behind (issue 59 spirit).
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

/// The fold-order group_key boundaries that partition the plan into buckets: the
/// LAST group_key of each successive [`Db::next_plan_batch`] window of `notice_batch`
/// notices. Ascending (fold order); the final entry is the global max group_key, so
/// every notice's group_key routes into some bucket.
async fn bucket_boundaries(db: &Db, notice_batch: usize) -> turso::Result<Vec<String>> {
    let mut boundaries = Vec::new();
    let mut after = String::new();
    loop {
        let groups = db.next_plan_batch(&after, notice_batch).await?;
        let Some(last) = groups.last() else { break };
        after = last.group_key.clone();
        boundaries.push(after.clone());
    }
    Ok(boundaries)
}

/// Headroom left below the file-descriptor budget when auto-sizing the worker
/// count — the writer connection, the WAL, stdio, and the K reader connections all
/// need descriptors alongside the `n_buckets × workers` open shard files (issue 66).
const FD_MARGIN: usize = 256;

/// Raise the process's soft `RLIMIT_NOFILE` toward its hard limit and return the
/// effective soft limit. The sharded pre-pass holds `n_buckets × workers` shard
/// files open at once (a route-by-content worker cannot close a bucket early — a
/// notice may route to any bucket at any point in its id sweep), which on a default
/// 1024-fd box would EMFILE. Best-effort: on any failure the current soft limit is
/// returned and [`worker_count`] caps the fan-out to fit it (FFI, issue 66 §4).
fn raise_fd_limit() -> usize {
    // SAFETY: plain getrlimit/setrlimit on RLIMIT_NOFILE with a well-formed struct.
    unsafe {
        let mut lim = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut lim) != 0 {
            return FD_MARGIN;
        }
        if lim.rlim_cur < lim.rlim_max {
            lim.rlim_cur = lim.rlim_max;
            libc::setrlimit(libc::RLIMIT_NOFILE, &lim); // denial is tolerated
        }
        let mut cur = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        let held = if libc::getrlimit(libc::RLIMIT_NOFILE, &mut cur) == 0 {
            cur.rlim_cur
        } else {
            lim.rlim_cur
        };
        usize::try_from(held).unwrap_or(usize::MAX)
    }
}

/// The parallel-pre-pass worker count (issue 66). A pinned `shards` (tests, and the
/// byte-identity gate at 1 vs 3+) is honoured verbatim; otherwise `cores − 1`,
/// clamped to at least 1 and capped so the `n_buckets × workers` open shard files
/// stay within `fd_budget` (§4).
fn worker_count(shards: Option<usize>, n_buckets: usize, fd_budget: usize) -> usize {
    if let Some(k) = shards {
        return k.max(1);
    }
    let fd_cap = (fd_budget.saturating_sub(FD_MARGIN) / n_buckets.max(1)).max(1);
    // Ops valve — the right number is a property of the DEVICE, not the build, and
    // this is the one knob worth turning without a redeploy while a many-hour sweep
    // is the critical path.
    if let Some(k) = std::env::var("TENDER_PREPASS_SHARDS").ok().and_then(|v| v.parse::<usize>().ok())
    {
        return k.clamp(1, fd_cap);
    }
    let cores = std::thread::available_parallelism().map_or(1, |c| c.get());
    cores.saturating_sub(1).max(PREPASS_MIN_WORKERS).clamp(1, fd_cap)
}

/// Floor on the pre-pass worker count, independent of core count (issue 94).
///
/// `cores − 1` sizes for a CPU-bound sweep. This one is not: a pre-pass worker sits
/// in uninterruptible disk wait with one read outstanding, and on prod the box was
/// measured 75% idle with 17.5% iowait while a single worker held ~16 MB/s. What
/// buys throughput here is DEVICE QUEUE DEPTH — more readers in flight — not more
/// cores, and the per-worker read chunk shrinks as workers are added
/// ([`PREPASS_CHUNK_BUDGET`]) so the memory bill does not follow.
///
/// 8 is deliberately conservative rather than optimal: it is a solid multiple of the
/// 3 the 4-core prod box was getting, stays sane on small dev machines, and the real
/// optimum is a device property — measure it with `TENDER_PREPASS_SHARDS` rather
/// than guess it here.
const PREPASS_MIN_WORKERS: usize = 8;

/// Pre-pass (issue 66): shard the parsed read into `k` contiguous notice-id stripes,
/// each swept by its own worker on its own reader connection, spilling resolved
/// states to its own `shard{s}_bucket{b}.bin` files. Routing is by group_key, so a
/// group always lands in one logical bucket `b` whichever stripe produced it; the
/// fold re-concatenates a bucket's K shard files and sorts (see [`read_bucket_shards`]).
/// Joins all workers before returning — the barrier the fold's global order relies on.
async fn write_buckets_sharded(
    db: &Db,
    boundaries: &[String],
    dir: &Path,
    k: usize,
    on_progress: &mut impl FnMut(Progress),
) -> turso::Result<usize> {
    std::fs::create_dir_all(dir).expect("create bucket dir");

    // Bound the sweep to the id range the PLAN covers (issue 94). `write_shard`
    // already skips notices absent from the plan, so ids outside this range can
    // never produce a bucket row — reading them is pure waste. On a rebuild the plan
    // covers the corpus and this degenerates to the whole id space; on a scoped
    // re-fold whose cohort is clustered (a late bulk reclaim is, by construction) it
    // removes most of the sweep. It can never change the OUTPUT, only which ids are
    // visited, so byte-identity is untouched.
    let max_id = db.max_parsed_notice_id().await?;
    let (lo, hi) = match db.plan_notice_id_range().await? {
        // `id > lo` is exclusive, so step one below the first planned notice.
        Some((plan_lo, plan_hi)) => (plan_lo - 1, plan_hi.min(max_id)),
        None => (0, max_id),
    };

    // Partition the swept range into contiguous stripes holding equally many PARSED
    // notices (issue 94) — NOT equal id widths, which put ~all the work in one
    // worker whenever the id space is unevenly dense (which it is: reclaims append
    // late). Falls back to one stripe when the range is too small to split.
    let stripes = db.parsed_id_stripes(lo, hi, k).await?;
    let k = stripes.len();
    eprintln!(
        "[project] phase 2 pre-pass: {k} shard(s) over notice ids ({lo}, {hi}] \
         ({} of the id space)",
        if max_id > 0 { format!("{}%", (hi - lo) * 100 / max_id.max(1)) } else { "100%".into() }
    );
    let readers = db.readers(k)?;

    // Each worker drives its stripe on its own thread (the decode/fold/encode is
    // CPU-bound, so real threads — not tokio tasks on the CLI's current-thread
    // runtime — are what parallelises it) with its own current-thread runtime and its
    // own reader connection. Scoped threads let the workers borrow `boundaries`/`dir`.
    // Keep peak RAM flat as `k` rises: each worker holds one read chunk of resolved
    // notices, so the per-worker chunk shrinks as workers are added (issue 94 /
    // the bounded-memory principle). Concurrency goes up, the working set does not.
    let chunk = (PREPASS_CHUNK_BUDGET / k).clamp(PREPASS_CHUNK_MIN, PREPASS_CHUNK_MAX) as i64;
    // Aggregate sweep counter the workers bump per chunk (never per notice — the
    // cadence bound is the chunk, issue 65's "keep it cheap" rule) and the parent
    // polls into `on_progress` while they run. The per-shard stderr heartbeats in
    // `write_shard` stay: they carry id positions the aggregate cannot.
    let swept = std::sync::atomic::AtomicU64::new(0);
    std::thread::scope(|scope| -> turso::Result<()> {
        let handles: Vec<_> = stripes
            .iter()
            .copied()
            .enumerate()
            .map(|(s, (lo, hi))| {
                let readers = readers.clone();
                let swept = &swept;
                scope.spawn(move || -> turso::Result<()> {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("build worker runtime");
                    rt.block_on(async move {
                        let conn = readers.get().await?;
                        write_shard(&conn, boundaries, dir, s, lo, hi, chunk, swept).await
                    })
                })
            })
            .collect();
        // The parent thread already blocks here for the pre-pass's whole
        // duration (the joins below) — polling first costs nothing extra and is
        // what turns the workers' shared counter into progress events. Coarse on
        // purpose: an in-memory read every 2 s against a phase measured in hours.
        while handles.iter().any(|h| !h.is_finished()) {
            std::thread::sleep(std::time::Duration::from_secs(2));
            on_progress(Progress::PrePass {
                notices: swept.load(std::sync::atomic::Ordering::Relaxed),
            });
        }
        for h in handles {
            h.join().expect("shard worker panicked")?;
        }
        Ok(())
    })?;
    // The closing tick, AFTER every worker joined: the complete count, exactly
    // once, however fast the corpus went. A test corpus finishes before the
    // first poll fires, so without this the variant would be untestable — and a
    // prod operator gets a final "the sweep read N" line either way.
    on_progress(Progress::PrePass {
        notices: swept.load(std::sync::atomic::Ordering::Relaxed),
    });
    Ok(k)
}

/// Total notices a sharded pre-pass holds in RAM at once, across ALL workers. Each
/// worker reads `PREPASS_CHUNK_BUDGET / k` notices per chunk, so raising the worker
/// count buys I/O concurrency without raising peak memory (issue 94). Clamped at
/// both ends: too small a chunk pays per-query overhead on every satellite scan, too
/// large a one puts the old un-sharded working set back on a single worker.
const PREPASS_CHUNK_BUDGET: usize = 30_000;
const PREPASS_CHUNK_MIN: usize = 1_000;
const PREPASS_CHUNK_MAX: usize = 10_000;

/// How many notices a pre-pass worker sweeps between heartbeats (issue 94). The
/// pre-pass used to print NOTHING for hours, and its one external proxy — the bucket
/// files — is actively misleading, because they are `BufWriter`-wrapped and flushed
/// only at the end, so their on-disk size stays near zero however far along the
/// sweep is. Diagnosing a live sweep meant reconstructing worker positions from
/// `/proc/<pid>/task/*/io`. Per shard, so with `k` workers the line rate is `k` per
/// this many notices swept.
const PREPASS_HEARTBEAT: u64 = 250_000;

/// One pre-pass worker: sweep notice ids in `(lo, hi]` on `conn`, resolve each
/// notice's [`NoticeState`] (binding the Organizations Phase-1 recorded), and append
/// it — postcard-framed `[u32 len][bytes]` — to `shard{shard}_bucket{b}.bin` for the
/// bucket its group_key routes to. Identical per-notice work to the old single-pass
/// `write_buckets`; only the id range and the shard-scoped file names differ.
async fn write_shard(
    conn: &store::Reader,
    boundaries: &[String],
    dir: &Path,
    shard: usize,
    lo: i64,
    hi: i64,
    read_chunk: i64,
    total_swept: &std::sync::atomic::AtomicU64,
) -> turso::Result<()> {
    // Every bucket file is created (even if it stays empty) so the fold's
    // `read_bucket_shards` can open `shard{s}_bucket{b}.bin` for every (s, b).
    let mut writers: Vec<BufWriter<File>> = (0..boundaries.len())
        .map(|b| {
            let p = dir.join(format!("shard{shard}_bucket{b}.bin"));
            BufWriter::new(File::create(&p).expect("create shard bucket file"))
        })
        .collect();
    let empty = HashMap::new();
    let mut after_id = lo;
    let t0 = std::time::Instant::now();
    let (mut swept, mut spilled, mut last_beat) = (0u64, 0u64, 0u64);
    loop {
        let mut chunk = Db::parsed_chunk_on(conn, after_id, hi, read_chunk).await?;
        normalise_de1(&mut chunk);
        let Some((last, _)) = chunk.last() else { break };
        let (clo, chi) = (chunk[0].0.id, last.id);
        after_id = last.id;
        // notice_id → group_key over the chunk's id window (one range scan).
        let group_keys = Db::plan_group_keys_on(conn, clo, chi).await?;
        let ids: Vec<i64> = chunk.iter().map(|(n, _)| n.id).collect();
        let orgs = Db::mentions_by_ids_on(conn, &ids).await?;
        for (notice, parsed) in &chunk {
            // A notice absent from the plan is a resume's post-Phase-1 suffix — not
            // grouped, so leave it unprojected for the incremental projection (it is
            // never folded and never marked projected).
            let Some(group_key) = group_keys.get(&notice.id) else { continue };
            let mut state = NoticeState::read(notice, parsed);
            state.bind_organizations(orgs.get(&notice.id).unwrap_or(&empty));
            let row = BucketRow::snapshot(group_key.clone(), notice, state);
            let bucket = boundaries.partition_point(|b| b.as_str() < group_key.as_str());
            let bytes = postcard::to_stdvec(&row).expect("serialize bucket row");
            let w = &mut writers[bucket];
            w.write_all(&u32::try_from(bytes.len()).expect("bucket row < 4GB").to_le_bytes())
                .expect("write bucket frame length");
            w.write_all(&bytes).expect("write bucket frame");
            spilled += 1;
        }
        // Heartbeat (issue 94): position and rate, per shard. `at` is how far the
        // stripe has been consumed, which is what makes an unbalanced partition or a
        // slow stripe visible while it is happening rather than afterwards.
        swept += chunk.len() as u64;
        // The aggregate the parent polls into Progress::PrePass (issue 65): once
        // per chunk, alongside the local counter — never per notice.
        total_swept.fetch_add(chunk.len() as u64, std::sync::atomic::Ordering::Relaxed);
        if swept - last_beat >= PREPASS_HEARTBEAT {
            last_beat = swept;
            eprintln!(
                "[project] pre-pass shard {shard}: {swept} swept, {spilled} spilled, \
                 at id {after_id} of ({lo}, {hi}], {:.0} notices/s",
                swept as f64 / t0.elapsed().as_secs_f64().max(1e-9)
            );
        }
    }
    eprintln!(
        "[project] pre-pass shard {shard} DONE: {swept} swept, {spilled} spilled from ({lo}, {hi}] in {:.1}s",
        t0.elapsed().as_secs_f64()
    );
    for w in &mut writers {
        w.flush().expect("flush shard bucket");
    }
    Ok(())
}

/// All of logical bucket `b`'s rows — the K `shard{s}_bucket{b}.bin` files a sharded
/// pre-pass wrote for it, concatenated (issue 66). Concatenation order across shards
/// is irrelevant: the caller re-establishes the total fold order by sorting on
/// `sort_key` before folding, so the merged bucket folds byte-identically to the
/// single-file bucket a serial pre-pass would have written.
fn read_bucket_shards(dir: &Path, bucket: usize, shards: usize) -> Vec<BucketRow> {
    let mut rows = Vec::new();
    for s in 0..shards {
        rows.extend(read_bucket(&dir.join(format!("shard{s}_bucket{bucket}.bin"))));
    }
    rows
}

/// Read back a bucket file written by a pre-pass worker — the `[u32 len][bytes]`
/// postcard frames, in append order.
fn read_bucket(path: &Path) -> Vec<BucketRow> {
    let mut reader = BufReader::new(File::open(path).expect("open bucket"));
    let mut rows = Vec::new();
    let mut len_buf = [0u8; 4];
    loop {
        match reader.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => panic!("read bucket frame length: {e}"),
        }
        let len = u32::from_le_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf).expect("read bucket frame");
        rows.push(postcard::from_bytes(&buf).expect("deserialize bucket row"));
    }
    rows
}

/// One bucket folded and ready for the single-writer apply: the pipeline's unit
/// of handoff from the prepare thread to the writer (issue 175). Plain data, so
/// it crosses the thread boundary; the projections are in fold order.
struct Prepared {
    projections: Vec<TenderProjection>,
    applied_ids: Vec<i64>,
}

/// Fold one bucket — already sorted by the fold key — into Tenders. Walks
/// adjacent equal-group_key runs (each a whole Tender, since a group never
/// spans a bucket and the bucket is sorted by `(group_key, …)`), rebuilds each
/// group's [`NoticeState`] chain, and folds it EXACTLY as [`apply_plan_batch`]
/// does. Pure CPU — no DB access — so the pipeline runs it on the prepare
/// thread while the writer applies the previous bucket; the caller feeds the
/// result to `apply_tenders` + `mark_projected` in bucket order, which is what
/// keeps every surrogate id byte-identical to the serial fold.
fn fold_rows(rows: &[BucketRow]) -> Prepared {
    let mut projections: Vec<TenderProjection> = Vec::new();
    let mut applied_ids: Vec<i64> = Vec::new();
    let mut i = 0;
    while i < rows.len() {
        let group_key = &rows[i].group_key;
        let mut j = i;
        while j < rows.len() && &rows[j].group_key == group_key {
            j += 1;
        }
        let group = &rows[i..j];
        let chain: Vec<NoticeState> = group.iter().map(BucketRow::to_notice_state).collect();
        let island_notice_id = group_key.strip_prefix("island:").and_then(|s| s.parse::<i64>().ok());
        let procedure_key = island_notice_id.is_none().then(|| group_key.clone());
        projections.push(TenderProjection {
            source: primary_source(&group.iter().map(|r| r.source.clone()).collect::<Vec<_>>()),
            procedure_key,
            island_notice_id,
            kind: kind_of(group[0].subtype.as_deref()).to_owned(),
            versions: fold(&chain.iter().collect::<Vec<_>>()),
        });
        applied_ids.extend(group.iter().map(|r| r.notice_id));
        i = j;
    }
    Prepared { projections, applied_ids }
}

/// One resolved notice spilled to an on-disk fold bucket (issue 62). Carries the
/// fold-order key fields (from the plan / the notice's Source) and the bound fold
/// payload (from the resolved [`NoticeState`]) — enough to rebuild a NoticeState for
/// [`fold`] without re-reading the parsed layer.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct BucketRow {
    group_key: String,
    source: String,
    source_rank: i64,
    notice_id: i64,
    publication_id: String,
    published_at: i64,
    dispatched_at: Option<i64>,
    subtype: Option<String>,
    original_lang: Option<String>,
    is_correction: bool,
    facts: BTreeSet<Fact>,
    lots: Vec<LotState>,
    round: Option<Round>,
    /// Issue 237's lots-group membership.
    ///
    /// Adding a field here changes an on-disk format, and `serde(default)` would NOT
    /// make that backward compatible: these rows are framed with **postcard**, which is
    /// not self-describing, so old bytes read under a new struct misparse rather than
    /// defaulting. What makes it safe is the lifecycle — the bucket directory is
    /// `remove_dir_all`'d at the START of every sharded run and again at the end, so no
    /// run ever reads bytes another binary wrote.
    group_members: Vec<(String, String)>,
}

impl BucketRow {
    /// Snapshot a fully-resolved notice (after `NoticeState::read` +
    /// `bind_organizations`). `group_key` is the grouping plan's; `source`/
    /// `source_rank` the notice's Source; everything else the bound state. The plan
    /// and the state agree on published_at/publication_id/subtype by construction
    /// (both read `notice_instants(parsed).0` / `notice.publication_id` /
    /// `first_code(parsed, SUBTYPE_FIELD)`), so one field serves both the fold-order
    /// sort key and the written version.
    fn snapshot(group_key: String, notice: &store::NoticeRef, state: NoticeState) -> BucketRow {
        BucketRow {
            group_key,
            source: notice.source.clone(),
            source_rank: i64::from(source_rank(&notice.source)),
            notice_id: state.notice_id,
            publication_id: state.publication_id,
            published_at: state.published_at,
            dispatched_at: state.dispatched_at,
            subtype: state.subtype,
            original_lang: state.original_lang,
            is_correction: state.is_correction,
            facts: state.facts,
            lots: state.lots,
            round: state.round,
            group_members: state.group_members,
        }
    }

    /// The fold-order key — IDENTICAL to `plan_notice_fold` / `next_plan_batch`'s
    /// `ORDER BY group_key, published_at, source_rank, publication_id, notice_id`.
    /// Sorting a bucket by this reproduces the exact per-group and cross-group order
    /// the streaming path folds in, so surrogate ids come out identical.
    fn sort_key(&self) -> (&str, i64, i64, &str, i64) {
        (&self.group_key, self.published_at, self.source_rank, &self.publication_id, self.notice_id)
    }

    /// Rebuild a [`NoticeState`] for [`fold`]. Roles and raw results were already
    /// consumed by `bind_organizations` (their output is in `facts`/`lots`/`round`),
    /// so they reconstruct empty; `logical_id` is only read pre-bind, so it is unused
    /// here. [`fold`] reads only the fields restored below.
    fn to_notice_state(&self) -> NoticeState {
        NoticeState {
            notice_id: self.notice_id,
            publication_id: self.publication_id.clone(),
            published_at: self.published_at,
            dispatched_at: self.dispatched_at,
            subtype: self.subtype.clone(),
            original_lang: self.original_lang.clone(),
            group_members: self.group_members.clone(),
            logical_id: None,
            is_correction: self.is_correction,
            facts: self.facts.clone(),
            lots: self.lots.clone(),
            roles: Vec::new(),
            raw_results: RawResults::default(),
            round: self.round.clone(),
            // Already bound: no reference is left to alias (issue 259).
            org_alias: HashMap::new(),
        }
    }
}

/// One notice read in canonical terms, before it is folded into a chain. Carries
/// only what folding a version needs; the grouping identity that assigns the
/// notice to a Tender lives in the far smaller [`Ident`] (issue 57).
struct NoticeState {
    notice_id: i64,
    publication_id: String,
    published_at: i64,
    dispatched_at: Option<i64>,
    subtype: Option<String>,
    /// ADR-0013 D3's third leg, from [`original_lang`]; rides into the version row.
    original_lang: Option<String>,
    /// BT-701, the source's logical notice id — corrections republish under it.
    logical_id: Option<String>,
    /// A change notice (it carries `efac:Changes` sections): what it publishes
    /// corrects an earlier notice rather than adding to the chain's results.
    is_correction: bool,
    /// Tender-scoped facts, and one bucket per lot the notice published.
    facts: BTreeSet<Fact>,
    lots: Vec<LotState>,
    /// Role references awaiting their canonical organization id: (scope,
    /// role, ORG section id).
    roles: Vec<(Scope, String, String)>,
    /// The notice's results graph, awaiting organization resolution.
    raw_results: RawResults,
    /// The bound results — `Some` exactly when the notice published any.
    round: Option<Round>,
    /// `(group lot key, member lot key)` pairs from this notice's `GroupComposition`
    /// sections (issue 237). Empty for the vast majority of notices.
    group_members: Vec<(String, String)>,
    /// Nested Organization section id -> the outermost Organization above it
    /// (issue 259). A role or winner reference may name either end of a nest; both
    /// must land on the one party. Empty except in the legacy eras.
    org_alias: HashMap<String, String>,
}

/// Where a value belongs: the Tender itself, or one of its Lots.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Scope {
    Tender,
    Lot(String),
}

/// A normalised OJS publication key `(year, number)`. The display form is not
/// stable across eras (`2011/S 1-000181` vs `2019/S 001-000001` vs the
/// `000001-2019` DOC form vs the text era's `154-2005`), so the join key is
/// always the parsed pair, never the raw string (research §1).
type OjsKey = (i64, i64);

impl NoticeState {
    fn read(notice: &store::NoticeRef, parsed: &Parsed) -> NoticeState {
        let sections: HashMap<&str, &store::Section> =
            parsed.sections.iter().map(|s| (s.id.as_str(), s)).collect();

        let mut lots: BTreeMap<String, LotState> = parsed
            .sections
            .iter()
            .filter(|s| LOT_KINDS.contains(&s.kind.as_str()))
            .map(|s| {
                (s.id.clone(), LotState { key: s.id.clone(), kind: s.kind.clone(), facts: BTreeSet::new() })
            })
            .collect();

        let legacy = is_legacy_profile(&notice.profile);
        let sdk01 = is_sdk01_profile(&notice.profile);
        // Award-family marker for `amount_target` (issue 177): only award forms
        // carry result sections, and only award forms publish `TOTAL_FINAL_VALUE`
        // — a result total — at object scope.
        let has_results =
            parsed.sections.iter().any(|s| RESULT_KINDS.contains(&s.kind.as_str()));
        let mut facts = BTreeSet::new();
        let mut raw_roles = Vec::new();
        // sdk-0.1 names its buyer by the `ContractingParty` section itself, with
        // no OPT-300 role reference — synthesise the buyer role directly at it.
        if sdk01 {
            for s in &parsed.sections {
                if s.kind == SDK01_BUYER_KIND {
                    raw_roles.push((Scope::Tender, s.id.clone(), "buyer".to_owned(), s.id.clone()));
                }
            }
        }
        // The tax basis a value states, keyed by the section that states it (issue 251).
        // It travels as a SIBLING code rather than a field on the amount, because the
        // parse layer's `NoticeValue::Amount` has no room for it and widening that enum
        // would touch every parser. Built before the loop so pairing is a lookup rather
        // than a rescan per amount.
        let tax_bases: std::collections::BTreeMap<&str, &str> = parsed
            .values
            .iter()
            .filter_map(|v| match &v.value {
                NoticeValue::Code { code, .. }
                    if TAX_BASIS_FIELDS.contains(&v.field_id.as_str())
                        && (code == "incl" || code == "excl") =>
                {
                    Some((v.section_id.as_str(), code.as_str()))
                }
                _ => None,
            })
            .collect();
        // Every (section, field_id) a basis marker sits at, and how many amounts each
        // (section, field_id) holds. The second is the one guard the derived-id rule still
        // needs: two unprefixed `COSTS_RANGE` containers in one section would both emit
        // `TED-VALUE_COST`, and a single marker could not say which it qualifies. None of
        // the committed fixtures does that, and if one exists the amount stays unlabelled
        // rather than guessed at.
        let mut markers: std::collections::BTreeSet<(&str, &str)> = Default::default();
        let mut amount_counts: std::collections::BTreeMap<(&str, &str), usize> = Default::default();
        for v in &parsed.values {
            let at = (v.section_id.as_str(), v.field_id.as_str());
            match &v.value {
                NoticeValue::Integer(_)
                    if BASIS_MARKERS.iter().any(|(el, _)| v.field_id.ends_with(el)) =>
                {
                    markers.insert(at);
                }
                NoticeValue::Amount { .. } => *amount_counts.entry(at).or_default() += 1,
                _ => {}
            }
        }
        // Issue 372: which (section, source field) pairs this notice declared
        // withheld, so an amount can be marked at the row it is emitted from
        // rather than by a rule over its number.
        let withheld = withheld_source_fields(parsed);

        for value in &parsed.values {
            let scope = scope_of(&sections, &value.section_id);
            let field_id = value.field_id.as_str();
            let fact = match &value.value {
                NoticeValue::Text { lang, value: v } => {
                    canonical_name(TEXTS, field_id).map(|field| Fact::Text {
                        field,
                        // issue 292: one language vocabulary across eras, or the
                        // read layer's 'ENG'-wins picks never fire for legacy tags.
                        lang: normalize_lang(lang.as_deref()),
                        value: v.clone(),
                    })
                }
                NoticeValue::Amount { cents, currency } => {
                    amount_target(field_id, &sections, &value.section_id, has_results).map(|field| {
                        Fact::Amount {
                            field,
                            cents: *cents,
                            currency: currency.clone(),
                            tax_basis: tax_bases
                                .get(value.section_id.as_str())
                                .map(|b| (*b).to_owned())
                                .or_else(|| {
                                    marker_basis(
                                        &value.section_id,
                                        field_id,
                                        &markers,
                                        &amount_counts,
                                    )
                                }),
                            // Issue 372: the notice's own declaration, not the
                            // shape of `cents`. `-1` marks nothing on its own --
                            // 116 of the corpus's 19,236 `-1.00` rows are
                            // publisher-invented sentinels with no withholding
                            // block at all (unit 5), and calling those withheld
                            // would assert something no notice ever said.
                            quality: withheld
                                .contains(&(value.section_id.as_str(), stem(field_id)))
                                .then(|| QUALITY_WITHHELD.to_owned()),
                        }
                    })
                }
                NoticeValue::Classification { scheme, code } => canonical_name(CLASSIFICATIONS, field_id)
                    .map(|field| Fact::Classification {
                        field,
                        scheme: scheme.clone(),
                        code: code.clone(),
                    }),
                NoticeValue::Date { utc_seconds, offset_minutes, has_time } => {
                    canonical_name(DATES, field_id).map(|field| Fact::Date {
                        field,
                        utc_seconds: *utc_seconds,
                        offset_minutes: *offset_minutes,
                        has_time: *has_time,
                    })
                }
                NoticeValue::Id { value: target, is_ref: true, scheme } => {
                    // An OJS-scheme reference is a chain edge (grouped via
                    // [`Ident`]); anything else is an inline organization role
                    // reference (legacy synthesises `ORG-n` refs, mirroring
                    // eForms' OPT-300 pattern).
                    if scheme.as_deref() != Some("ojs")
                        && let Some(role) = role_name(&value.field_id)
                    {
                        raw_roles.push((scope.clone(), value.section_id.clone(), role, target.clone()));
                    }
                    None
                }
                _ => None,
            };
            if let Some(fact) = fact {
                match &scope {
                    Scope::Tender => {
                        facts.insert(fact);
                    }
                    Scope::Lot(key) => {
                        if let Some(lot) = lots.get_mut(key) {
                            lot.facts.insert(fact);
                        }
                    }
                }
            }
        }

        // Issue 233: the OJ heading as a LAST-RESORT title.
        //
        // A notice that carries no title element still has a title: the heading
        // the Official Journal published it under, `TI_DOC`, e.g.
        // "NO-Bodø: miscellaneous vessels". The 2008 INTERNAL_OJS era needs this —
        // 56 % of its versions have no `TITLE_CONTRACT` because whole form families
        // (EEIG registrations and friends) do not have one, while every notice in
        // the era carries `TI_DOC` — and it measured 43.7 % title completeness
        // against ≥ 96 % everywhere else.
        //
        // Two rules make it a fallback rather than a competing title:
        //
        // - only when the notice mapped NO title of its own, so the 44 % that do
        //   publish one are untouched, as are r2.0.x and eForms;
        // - never the paragraph that merely restates the publication reference.
        //   `TI_DOC` is published as two paragraphs — the heading, then
        //   "2008/S 85-114238", which is `NO_DOC_OJS` again. A title of
        //   "2008/S 85-114238" would be worse than none, and it is the shape a
        //   positional "take the first paragraph" rule would eventually pick up.
        if !facts.iter().any(|f| matches!(f, Fact::Text { field, .. } if field == "title")) {
            let reference = first_id(parsed, "TED-NO_DOC_OJS");
            let heading = parsed.values.iter().find_map(|v| match &v.value {
                NoticeValue::Text { lang, value }
                    if v.field_id == OJ_HEADING_FIELD
                        && scope_of(&sections, &v.section_id) == Scope::Tender
                        && Some(value.trim()) != reference.as_deref() =>
                {
                    Some((lang.clone(), value.clone()))
                }
                _ => None,
            });
            if let Some((lang, value)) = heading {
                facts.insert(Fact::Text {
                    field: "title".to_owned(),
                    lang: normalize_lang(lang.as_deref()),
                    value,
                });
            }
        }

        let raw_results = read_results(&sections, parsed, legacy, sdk01);
        // Award-side roles sit under the results graph, which has no Lot
        // ancestor — resolve their Lot through the graph instead (issue 04's
        // noted limitation, closed here).
        let roles = raw_roles
            .into_iter()
            .map(|(scope, source, role, target)| {
                let scope = match scope {
                    Scope::Tender => match raw_results.lot_of(&sections, &source) {
                        Some(key) if lots.contains_key(&key) => Scope::Lot(key),
                        _ => Scope::Tender,
                    },
                    lot => lot,
                };
                (scope, role, target)
            })
            .collect();

        // `tender_versions.published_at` is NOT NULL and the fold orders
        // versions by it, so the version layer keeps the pre-367 epoch fallback
        // for a notice that states no date on either axis. The honest `None`
        // lands on the nullable NOTICE column (see [`notice_instants`]); the two
        // still AGREE wherever the resolver found anything, which is the
        // invariant `notices_and_their_versions_carry_the_same_instants` pins.
        let (published_at, dispatched_at) = notice_instants(parsed);
        let published_at = published_at.unwrap_or(0);

        NoticeState {
            notice_id: notice.id,
            publication_id: notice.publication_id.clone(),
            published_at,
            dispatched_at,
            subtype: first_code(parsed, SUBTYPE_FIELD),
            original_lang: original_lang(parsed),
            group_members: group_members(notice.id, parsed),
            logical_id: first_id(parsed, LOGICAL_NOTICE_FIELD),
            is_correction: parsed.sections.iter().any(|s| s.kind == "Change"),
            facts,
            lots: lots.into_values().collect(),
            roles,
            raw_results,
            round: None,
            // A role or winner reference may name the inner half of a nested party
            // (issue 259); both halves must bind to the one Organization.
            org_alias: nested_org_aliases(
                &sections,
                if sdk01 { SDK01_PARTY_KINDS } else { &[ORGANIZATION_KIND] },
            ),
        }
    }

    /// Every Organization section of the notice, normalised for merging.
    ///
    /// An Organization's own values are spread over its subtree rather than
    /// sitting on the section itself: the name is on the Organization, but the
    /// official identifier hangs off its `CompanyLegalEntity` child (14 813 of
    /// them on the 2026-136 daily). So a mention collects from the whole
    /// subtree, keyed by the enclosing Organization.
    fn mentions(sdk01: bool, notice_id: i64, parsed: &Parsed) -> Vec<Mention> {
        let sections: HashMap<&str, &store::Section> =
            parsed.sections.iter().map(|s| (s.id.as_str(), s)).collect();

        // sdk-0.1 has no eForms `Organization` sections: its buyer and winners
        // are the inline `ContractingParty`/`WinningParty` sections themselves,
        // each carrying its name on its direct Party subtree (issue 29).
        let mention_kinds: &[&str] = if sdk01 { SDK01_PARTY_KINDS } else { &[ORGANIZATION_KIND] };

        // One party, one mention, even when the era's vocabulary opens two nested
        // Organization sections for it (issue 259). The inner ones are aliases, not
        // parties of their own.
        let alias = nested_org_aliases(&sections, mention_kinds);

        let mut mentions: BTreeMap<&str, Mention> = parsed
            .sections
            .iter()
            .filter(|s| mention_kinds.contains(&s.kind.as_str()) && !alias.contains_key(&s.id))
            .map(|s| {
                (
                    s.id.as_str(),
                    Mention {
                        notice_id,
                        section_id: s.id.clone(),
                        name: String::new(),
                        country: None,
                        raw_identifier: None,
                        scheme: None,
                        identifier: None,
                        variants: Vec::new(),
                    },
                )
            })
            .collect();

        for value in &parsed.values {
            let Some(owner) = enclosing(&sections, &value.section_id, mention_kinds) else {
                continue;
            };
            // A value inside a nested Organization belongs to the party the nest is —
            // which is how `Opal Publicidade, S. A.` reaches the winner rather than
            // sitting on an unreferenced sibling.
            let owner = alias.get(owner).map_or(owner, |outer| outer.as_str());
            let Some(mention) = mentions.get_mut(owner) else { continue };
            let field = value.field_id.as_str();
            // Three vocabularies read here: eForms hangs BT-501 off a
            // `CompanyLegalEntity` child; legacy uses inline `OFFICIALNAME` /
            // `COUNTRY` / `NATIONALID` address blocks (research §6); sdk-0.1 reads
            // the party section's *direct* name/country (never the nested
            // `ServiceProviderParty` eSender), and carries no official id there.
            let (is_name, is_country, is_id) = if sdk01 {
                (SDK01_PARTY_NAME_FIELDS.contains(&field), SDK01_PARTY_COUNTRY_FIELDS.contains(&field), false)
            } else {
                (
                    field == ORG_NAME_FIELD || ORG_NAME_FIELDS.contains(&field),
                    field == ORG_COUNTRY_FIELD || ORG_COUNTRY_FIELDS.contains(&field),
                    field == ORG_IDENTIFIER_FIELD || field == ORG_NATIONALID_FIELD,
                )
            };
            match &value.value {
                NoticeValue::Text { value, lang } if is_name => {
                    // The designated single head keeps its first-seen semantics.
                    if mention.name.is_empty() {
                        mention.name.clone_from(value);
                    }
                    // ADR-0013 D4: every LABELLED variant feeds the
                    // organization_names satellite, first-seen per language
                    // (eForms multilingual notices publish BT-500 per language).
                    if let Some(lang) = normalize_lang(lang.as_deref()) {
                        if !mention.variants.iter().any(|(l, _)| *l == lang) {
                            mention.variants.push((lang, value.clone()));
                        }
                    }
                }
                NoticeValue::Code { code, .. } if is_country => {
                    mention.country.get_or_insert_with(|| code.clone());
                }
                NoticeValue::Id { value, scheme, .. } if is_id && mention.raw_identifier.is_none() => {
                    mention.raw_identifier = Some(value.clone());
                    mention.scheme.clone_from(scheme);
                }
                _ => {}
            }
        }

        mentions
            .into_values()
            .map(|mut m| {
                // Canonicalise the country to alpha-2 (issue 48) before it is
                // stored AND before it scopes a national id, so both agree and a
                // country filter no longer splits `DEU`/`DE`/`UK` apart.
                m.country = m.country.map(|c| canonical_country(&c));
                // Issue 365 unit 4: the publisher's DECLARED scheme decides
                // admission before the value's shape gets a say. The scheme was
                // already captured onto the mention and then never consulted, so
                // "this scheme is never a register" was inexpressible — and no
                // value-shape rule can substitute, because the values under such
                // a scheme are arbitrary.
                //
                // Deliberately gated HERE rather than inside
                // `normalise_identifier`. The issue proposed threading the scheme
                // into the normaliser, but that is a 78-call-site signature change
                // for a question the normaliser should not be asked: it converts a
                // STRING to an identifier, while this is a decision about whether
                // a MENTION's identifier may key a merge. The scheme is already in
                // scope on `m` at exactly this point.
                m.identifier = m
                    .raw_identifier
                    .as_deref()
                    .filter(|_| !scheme_never_keys(m.scheme.as_deref()))
                    .and_then(|raw| normalise_identifier(raw, m.country.as_deref()));
                m
            })
            .collect()
    }

    /// Turn the notice-local role references into party facts — and the
    /// results graph into a bound Round — now that each mention has a canonical
    /// Organization. `by_section` maps this notice's Organization section ids to
    /// the canonical Organization ids Phase 1 resolved and recorded.
    fn bind_organizations(&mut self, by_section: &HashMap<String, i64>) {
        let mut by_section: HashMap<&str, i64> =
            by_section.iter().map(|(k, &v)| (k.as_str(), v)).collect();
        // The inner half of a nested party resolves to the outer half's Organization
        // (issue 259). Without this a winner reference naming `ADDRESS_WINNER` would
        // find nothing now that only the outermost section mints a mention — and one
        // naming `WINNER` would still be the nameless wrapper. Both now bind to the
        // single party, and the caller's existing sort/dedup collapses an award that
        // references both ends of the same nest into one winner rather than two.
        for (inner, outer) in &self.org_alias {
            if let Some(&id) = by_section.get(outer.as_str()) {
                by_section.insert(inner.as_str(), id);
            }
        }
        self.round = (!self.raw_results.is_empty())
            .then(|| self.raw_results.bind(self.notice_id, self.logical_id.clone(), &by_section));
        for (scope, role, target) in std::mem::take(&mut self.roles) {
            // A reference to something that is not an Organization section (a
            // touchpoint, a lot, a result) is notice-layer detail, not a party.
            let Some(&organization_id) = by_section.get(target.as_str()) else { continue };
            let fact = Fact::Party {
                role,
                organization_id,
                notice_id: self.notice_id,
                section_id: target,
            };
            match scope {
                Scope::Tender => {
                    self.facts.insert(fact);
                }
                Scope::Lot(key) => {
                    if let Some(lot) = self.lots.iter_mut().find(|l| l.key == key) {
                        lot.facts.insert(fact);
                    }
                }
            }
        }
    }

}

/// A Tender's canonical kind, from a notice's subtype: a Business Registration
/// Information Notice is a Tender of its own kind (CONTEXT.md), everything else a
/// procurement procedure.
fn kind_of(subtype: Option<&str>) -> &'static str {
    match subtype {
        Some(REGISTRATION_SUBTYPE) => "registration",
        _ => "procedure",
    }
}

/// One notice's compact grouping identity — everything the disk-backed plan needs
/// to assign it to a Tender and order it, and nothing else. Read per notice in
/// Phase 1 and written straight to the plan via [`Ident::into_plan_row`]; never
/// accumulated (issue 59). Three identity regimes coexist
/// (docs/research/ted-legacy-mapping.md §3, §8.3), all resolved by
/// [`store::Db::build_plan_groups`] in SQL:
///
/// - **Keyed** — a notice publishing a procedure key (eForms BT-04) shares a
///   Tender with every notice under the same key, across Sources (a TED eForms
///   procedure and its DÖE twin publish one BT-04 UUID — ADR-0003).
/// - **Legacy OJS chain** — a legacy TED notice publishes no key; its own OJS
///   number is a graph node and an `is_ref` OJS id an edge — but only where the
///   payload declares the cited publication to be a predecessor of the SAME
///   procedure ([`ojs_chain_edges`], issue 364). The transitive closure is one
///   Tender, identified by the *earliest* OJS number in the component (including
///   not-yet-ingested edge targets, so identity is stable as backfill deepens).
///   A late edge merging two components is an ADR-0003-style merge; the absorbed
///   key's rows are retired with `removed` events.
/// - **Island** — anything else (an eForms notice without BT-04, a DÖE numeric
///   island) is a single-notice Tender keyed by that notice.
struct Ident {
    notice_id: i64,
    source: String,
    publication_id: String,
    published_at: i64,
    legacy: bool,
    sdk01: bool,
    procedure_key: Option<String>,
    ojs_self: Option<OjsKey>,
    ojs_edges: Vec<OjsKey>,
    /// Normalised `publication_id`s this notice names as its predecessors
    /// (ADR-0011). A set: 2 of 1,343 measured carriers named two publications and
    /// one named three, and a procedure republished in parts is still one procedure.
    prev_refs: Vec<String>,
    subtype: Option<String>,
    /// Issue 364: what the previous-publication kind gate did to this notice's
    /// citations. Not part of the plan row — a tally, summed into the run's
    /// [`Report`] so a re-projection can be read against it.
    citations: CitationGate,
    /// Issue 369 unit 2: the buyer SET this notice publishes, sorted and joined —
    /// see [`buyer_key`] for why a set and not one buyer.
    buyer_key: Option<String>,
}

/// Issue 364 — what the previous-publication kind gate did, per declared kind.
///
/// A legacy citation becomes a Tender chain edge only where the payload declares
/// it to be a predecessor of THIS procedure. Everything else is refused, and
/// refusing silently is how the defect stood for years: 212 Tenders each fusing
/// hundreds of unrelated procurements read green because nothing counted. So the
/// gate keeps a tally, it rides the durable [`Report`] (the `WallCounts`
/// precedent — this runtime's stderr does not reach journald, issues 61/63), and
/// the supervisor prints it on the job row.
///
/// The shares to expect, measured over four February-2013 archive days (6,327
/// notices, 3,037 citations): `CONTRACT_NOTICE` 72.9 %, `PRIOR_INFORMATION_NOTICE`
/// 15.8 %, undeclared 7.8 %, `NOTICE_BUYER_PROFILE` 1.7 %,
/// `PERIODIC_INDICATIVE_NOTICE` 0.9 %, `SIMPLIFIED_CONTRACT_NOTICE_DPS` 0.4 %,
/// `NOTICE_QUALIFICATION_SYSTEM` 0.4 %. A run whose `undeclared` dwarfs that share
/// is the signal that an era publishes a shape this gate has not been taught —
/// look at it before assuming the split is right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CitationGate {
    /// Citations admitted as chain edges: the payload declared a same-procedure
    /// predecessor (`CONTRACT_NOTICE`, or a corrigendum's original notice).
    pub admitted: u64,
    pub prior_information: u64,
    pub periodic_indicative: u64,
    pub buyer_profile: u64,
    pub qualification_system: u64,
    /// A simplified DPS notice: one system, not one procurement (see
    /// [`rules::SHARED_PUBLICATION_KINDS`]).
    pub dps: u64,
    /// No kind declared anywhere in reach — the forms' own "other previous
    /// publications" slot and anything else unnamed. Refused by default,
    /// because defaulting to "edge" is the mistake issue 364 is about.
    pub undeclared: u64,
    /// A declared kind neither table knows. Refused, and worth looking at: it
    /// means the corpus publishes a spelling this gate has never seen.
    pub unknown_kind: u64,
}

impl CitationGate {
    pub fn refused(&self) -> u64 {
        self.prior_information
            + self.periodic_indicative
            + self.buyer_profile
            + self.qualification_system
            + self.dps
            + self.undeclared
            + self.unknown_kind
    }

    fn add(&mut self, other: CitationGate) {
        self.admitted += other.admitted;
        self.prior_information += other.prior_information;
        self.periodic_indicative += other.periodic_indicative;
        self.buyer_profile += other.buyer_profile;
        self.qualification_system += other.qualification_system;
        self.dps += other.dps;
        self.undeclared += other.undeclared;
        self.unknown_kind += other.unknown_kind;
    }

    fn refuse(&mut self, kind: &str) {
        let slot = match kind {
            "PRIOR_INFORMATION_NOTICE" => &mut self.prior_information,
            "PERIODIC_INDICATIVE_NOTICE" => &mut self.periodic_indicative,
            "NOTICE_BUYER_PROFILE" => &mut self.buyer_profile,
            "NOTICE_QUALIFICATION_SYSTEM" => &mut self.qualification_system,
            "SIMPLIFIED_CONTRACT_NOTICE_DPS" => &mut self.dps,
            rules::KIND_UNDECLARED => &mut self.undeclared,
            _ => &mut self.unknown_kind,
        };
        *slot += 1;
    }
}

/// This notice's OJS chain edges, and the tally of what the kind gate refused
/// (issue 364).
///
/// Every `is_ref` OJS-scheme id is a candidate edge, as before. The gate applies
/// only to those the parse layer marked as a previous-publication CITATION — the
/// `<field>.PREV_KIND` code row the legacy walker records beside
/// `NOTICE_NUMBER_OJ`/`NOTICE_NUMBER`, paired by `(section, field, ordinal)`.
///
/// An id with no such row keeps its pre-364 meaning, and that is deliberate in
/// three ways. `REF_NOTICE/NO_DOC_OJS` — the coded-data-section predecessor — is
/// the reference TED itself picks per procedure, and in every fixture that has
/// both it names the SAME publication as the same-procedure citation (the 2011
/// F03's `CONTRACT_NOTICE`, the 2017 F06's, the 2019 F14's original notice) and
/// never the PIN; the text era's `TXT-RN` and the eForms edge have their own
/// warrants (ADR-0011). And a notice parsed BEFORE this gate existed carries no
/// kind rows at all, so it keeps today's grouping until it is re-parsed — the
/// change lands without a flag day, and takes effect era by era as the re-parse
/// deepens.
fn ojs_chain_edges(parsed: &Parsed) -> (Vec<OjsKey>, CitationGate) {
    let mut gate = CitationGate::default();
    let mut edges = Vec::new();
    for v in &parsed.values {
        let NoticeValue::Id { value, is_ref: true, scheme } = &v.value else { continue };
        if scheme.as_deref() != Some("ojs") {
            continue;
        }
        match citation_kind_of(parsed, v) {
            // Not a previous-publication citation: unchanged (see above).
            None => edges.extend(ojs_key(value)),
            Some(kind) if rules::kind_is_same_procedure(kind) => {
                gate.admitted += 1;
                edges.extend(ojs_key(value));
            }
            // A shared publication that many unrelated procurements cite, or a
            // slot that declares nothing. Recorded as notice detail — it is
            // still in the parse layer, and still served — but it joins nothing.
            Some(kind) => gate.refuse(kind),
        }
    }
    (edges, gate)
}

/// The kind the parse layer recorded beside one citation, if this id is one.
/// Paired by `(section, field, ordinal)`: the walker emits the code row
/// immediately after the citation, in the same section, once per citation, so
/// the ordinals of `TED-X` and `TED-X.PREV_KIND` advance together.
fn citation_kind_of<'a>(parsed: &'a Parsed, citation: &store::ValueRow) -> Option<&'a str> {
    let field = format!("{}.{}", citation.field_id, rules::CITATION_KIND_SUFFIX);
    parsed
        .values
        .iter()
        .find(|v| {
            v.ordinal == citation.ordinal
                && v.field_id == field
                && v.section_id == citation.section_id
        })
        .and_then(|v| match &v.value {
            NoticeValue::Code { code, .. } => Some(code.as_str()),
            _ => None,
        })
}

/// Encode an OJS key `(year, number)` as `year*1e9 + number` — a single sortable
/// integer whose `MIN` over a component is the earliest publication (the legacy
/// Tender's representative). `number` is well under 1e9, `year ≤ 2100`, so this
/// fits `i64` and never collides across keys.
fn encode_ojs((year, number): OjsKey) -> i64 {
    year * 1_000_000_000 + number
}

/// Every predecessor publication this notice names, normalised and deduped
/// (ADR-0011). Not filtered to `is_ref`: the field is published as a plain id
/// (`scheme` absent, `is_ref = 0`), so requiring a reference flag would find
/// nothing — checked against the bytes on prod before this was written.
fn previous_publications(parsed: &Parsed) -> Vec<String> {
    let mut out: Vec<String> = parsed
        .values
        .iter()
        .filter(|v| v.field_id == PREVIOUS_NOTICE_FIELD)
        .filter_map(|v| match &v.value {
            NoticeValue::Id { value, .. } => publication_ref(value),
            _ => None,
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// A previous-notice reference as a `notices.publication_id`, or `None` if it is
/// not a TED publication number.
///
/// eForms writes `615938-2024`; the archive holds `00615938-2024` — 8 digits,
/// zero-padded, then the year. Normalising here rather than at the join keeps the
/// shape in one place and makes an unparseable reference a `None` at read time
/// instead of a row that silently matches nothing later. Anything that is not
/// `<digits>-<4 digits>` is ignored rather than guessed at: a wrong publication id
/// would merge two unrelated procedures, which ADR-0011 rates worse than leaving
/// them apart.
fn publication_ref(value: &str) -> Option<String> {
    let (number, year) = value.trim().split_once('-')?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if year.len() != 4 || !digits(year) || !digits(number) {
        return None;
    }
    let number = number.trim_start_matches('0');
    // Longer than the archive's own width is not a publication number we hold.
    if number.is_empty() || number.len() > 8 {
        return None;
    }
    Some(format!("{number:0>8}-{year}"))
}

impl Ident {
    fn read(notice: &store::NoticeRef, parsed: &Parsed) -> Ident {
        let legacy = is_legacy_profile(&notice.profile);
        let sdk01 = is_sdk01_profile(&notice.profile);
        let (mut ojs_edges, citations) = ojs_chain_edges(parsed);
        ojs_edges.sort_unstable();
        ojs_edges.dedup();
        let ojs_self = legacy
            .then(|| {
                ojs_key(&notice.publication_id).or_else(|| {
                    LEGACY_OWN_NUMBER_FIELDS.iter().find_map(|f| first_id(parsed, f).and_then(|v| ojs_key(&v)))
                })
            })
            .flatten();
        Ident {
            notice_id: notice.id,
            source: notice.source.clone(),
            publication_id: notice.publication_id.clone(),
            // The plan's fold-order key. Same epoch fallback as `NoticeState`,
            // and it must be the SAME one — the two are documented to agree by
            // construction (see `BucketRow::snapshot`).
            published_at: notice_instants(parsed).0.unwrap_or(0),
            legacy,
            sdk01,
            procedure_key: procedure_key(parsed, sdk01, is_de1_profile(&notice.profile)),
            ojs_self,
            ojs_edges,
            prev_refs: previous_publications(parsed),
            subtype: first_code(parsed, SUBTYPE_FIELD),
            citations,
            // issue 369 unit 2: the buyer set this notice publishes, for the
            // key-election gate. Parsed-side, so no org-layer dependency.
            buyer_key: buyer_key(sdk01, notice.id, parsed),
        }
    }

    /// This notice's row for the on-disk grouping plan — OJS keys encoded, Source
    /// precedence precomputed (issue 59).
    fn into_plan_row(self) -> store::PlanRow {
        // issue 369 unit 2. Computed BEFORE the literal, which moves
        // `procedure_key` out of `self` — read off the key this row carries, so
        // the verdict cannot disagree with the key it describes.
        let key_shaped = self.procedure_key.as_deref().is_some_and(is_placeholder_key);
        store::PlanRow {
            notice_id: self.notice_id,
            procedure_key: self.procedure_key,
            legacy: self.legacy,
            ojs_self: self.ojs_self.map(encode_ojs),
            source_rank: i64::from(source_rank(&self.source)),
            source: self.source,
            publication_id: self.publication_id,
            published_at: self.published_at,
            subtype: self.subtype,
            ojs_edges: self.ojs_edges.into_iter().map(encode_ojs).collect(),
            prev_refs: self.prev_refs,
            key_shaped,
            buyer_key: self.buyer_key,
        }
    }
}

/// Fixed cross-source precedence for the supersession tiebreak (ADR-0003). On
/// an equal publication instant the higher rank folds last and so wins the
/// shared eForms fields and the publication identity — TED > DÖE, because the
/// OJEU gazette is the authoritative publication record. (German national
/// content — national-codelist codes and DEX satellites — is not projected as
/// canonical facts; it is retained in full in the notice layer, so the DÖE
/// side of the ADR precedence needs no fact-level override here.)
fn source_rank(source: &str) -> u8 {
    match source {
        "ted" => 1,
        _ => 0,
    }
}

/// The Source a merged Tender is labelled by. ADR-0003 puts publication
/// identity on the TED side, so a procedure present on both Sources is a TED
/// Tender; a Source-only procedure keeps its own. `sources` is the group's
/// notices in fold order.
fn primary_source(sources: &[String]) -> String {
    if sources.iter().any(|s| s == "ted") {
        "ted".to_owned()
    } else {
        sources[0].clone()
    }
}

/// Resolve the chain: each version is the notice's own values laid over the
/// previous version's, per field — except results, which are *additive*.
fn fold(chain: &[&NoticeState]) -> Vec<TenderVersion> {
    let mut versions: Vec<TenderVersion> = Vec::with_capacity(chain.len());
    for state in chain {
        let previous = versions.last();
        let mut facts = previous.map(|p| p.facts.clone()).unwrap_or_default();
        supersede(&mut facts, &state.facts);

        let mut lots: Vec<LotState> = previous.map(|p| p.lots.clone()).unwrap_or_default();
        for published in &state.lots {
            match lots.iter_mut().find(|l| l.key == published.key) {
                Some(carried) => {
                    carried.kind.clone_from(&published.kind);
                    supersede(&mut carried.facts, &published.facts);
                }
                None => lots.push(published.clone()),
            }
        }
        lots.sort_by(|a, b| a.key.cmp(&b.key));

        // Results accumulate: a framework/DPS round or a tranche CAN adds its
        // round and never deletes an earlier one (ted-empirical-checks.md §1:
        // v(n+1) does NOT contain v(n)'s content — the 24/24 and 37/37 union
        // pattern). The one exception is a correction — a change notice
        // republishing the same logical notice (BT-701 + efac:Changes) — which
        // replaces the round it corrects instead of duplicating it.
        let mut rounds = previous.map(|p| p.rounds.clone()).unwrap_or_default();
        if let Some(round) = &state.round {
            if state.is_correction && round.logical_notice_id.is_some() {
                rounds.retain(|r| r.logical_notice_id != round.logical_notice_id);
            }
            rounds.push(round.clone());
        }

        // Membership carries forward like lots and facts, and for the same reason: the
        // composition is published by the notice that DEFINES the groups — a contract
        // notice — while the bids that reference a group arrive with the award notice
        // several versions later. Read only from its own notice, membership would be
        // present on exactly the version that has no bids, which is the one shape this
        // table exists to serve (issue 237).
        //
        // Supersession is per GROUP, mirroring `supersede`'s per-field rule: a notice
        // that republishes a group's composition replaces that group's member list
        // entirely, and a group it is silent about keeps the one it had.
        let mut group_members: Vec<(String, String)> =
            previous.map(|p| p.group_members.clone()).unwrap_or_default();
        if !state.group_members.is_empty() {
            let republished: BTreeSet<&str> =
                state.group_members.iter().map(|(group, _)| group.as_str()).collect();
            group_members.retain(|(group, _)| !republished.contains(group.as_str()));
            group_members.extend(state.group_members.iter().cloned());
            group_members.sort_unstable();
            group_members.dedup();
        }

        versions.push(TenderVersion {
            caused_by_notice_id: state.notice_id,
            published_at: state.published_at,
            dispatched_at: state.dispatched_at,
            notice_subtype: state.subtype.clone(),
            original_lang: state.original_lang.clone(),
            publication_id: state.publication_id.clone(),
            facts,
            lots,
            rounds,
            group_members,
        });
    }
    versions
}

/// Supersession per field: a field the notice republishes replaces the carried
/// one entirely; a field it is silent about is left alone.
fn supersede(carried: &mut BTreeSet<Fact>, published: &BTreeSet<Fact>) {
    let republished: BTreeSet<(&str, &str)> = published.iter().map(Fact::key).collect();
    carried.retain(|fact| !republished.contains(&fact.key()));
    carried.extend(published.iter().cloned());
}

/// Which Lot a value belongs to — the nearest enclosing Lot section, or the
/// Tender when there is none.
fn scope_of(sections: &HashMap<&str, &store::Section>, section_id: &str) -> Scope {
    match enclosing(sections, section_id, LOT_KINDS) {
        Some(lot) => Scope::Lot(lot.to_owned()),
        None => Scope::Tender,
    }
}

/// The nearest section of one of `kinds`, starting at `section_id` itself and
/// walking up the parent chain. This is how a value finds the entity it
/// describes: eForms hangs values off the deepest node that carries them, and
/// the canonical scope is the nearest enclosing entity above it.
/// Map every Organization-kind section that is NESTED inside another one to the
/// OUTERMOST Organization above it (issue 259). Sections not nested are absent.
///
/// One real-world party can open two Organization sections. `r209/rules.rs` declares
/// both `WINNER` and `ADDRESS_WINNER` as `Rule::Org`, so an F13 prize block nests
/// `ADDRESS_WINNER` (which carries `OFFICIALNAME`) inside `WINNER` (which carries
/// nothing). Left alone that is two Organizations for one company: the award's winner
/// reference points at the empty wrapper, so the winner has no name, and the real party
/// sits on a sibling row nobody reads. Nesting is the signal that they are the same
/// party — an Organization is not a container for other Organizations in any era's
/// vocabulary — so the inner ones alias to the outer.
///
/// eForms is unaffected: `efac:Organization` sections are siblings under
/// `efac:Organizations`, which is not itself an Organization, so nothing nests and the
/// map comes back empty.
fn nested_org_aliases(
    sections: &HashMap<&str, &store::Section>,
    kinds: &[&str],
) -> HashMap<String, String> {
    let mut alias = HashMap::new();
    for section in sections.values() {
        if !kinds.contains(&section.kind.as_str()) {
            continue;
        }
        // Walk the whole ancestor chain, keeping the LAST Organization seen: with three
        // levels of nesting the innermost must land on the outermost, not on its parent.
        let mut outermost: Option<&str> = None;
        let mut current = section.parent.as_deref();
        for _ in 0..sections.len().max(1) {
            let Some(id) = current else { break };
            let Some(ancestor) = sections.get(id) else { break };
            if kinds.contains(&ancestor.kind.as_str()) {
                outermost = Some(ancestor.id.as_str());
            }
            current = ancestor.parent.as_deref();
        }
        if let Some(outer) = outermost {
            alias.insert(section.id.clone(), outer.to_owned());
        }
    }
    alias
}

fn enclosing<'a>(
    sections: &HashMap<&str, &'a store::Section>,
    section_id: &str,
    kinds: &[&str],
) -> Option<&'a str> {
    let mut current = section_id;
    // Bounded by the section count: the parent chain is a tree, but guard
    // anyway so malformed data cannot spin.
    for _ in 0..sections.len().max(1) {
        let section = sections.get(current)?;
        if kinds.contains(&section.kind.as_str()) {
            return Some(section.id.as_str());
        }
        current = section.parent.as_deref()?;
    }
    None
}

// ------------------------------------------------------------------- results

/// The results graph of one notice, read in the notice's own vocabulary
/// (section keys), before organizations are resolved. eForms links everything
/// by notice-local id-refs: LotResult → Lot/Bid/Contract, Bid (LotTender) →
/// Lot/TenderingParty, Contract → Bid, TenderingParty → Organizations.
#[derive(Default)]
struct RawResults {
    lot_results: Vec<RawLotResult>,
    bids: Vec<RawBid>,
    contracts: Vec<RawContract>,
    parties: Vec<RawParty>,
}

#[derive(Default)]
struct RawLotResult {
    key: String,
    lot_key: Option<String>,     // BT-13713
    decision: Option<String>,    // BT-142
    reason: Option<String>,      // BT-144
    bid_refs: Vec<String>,       // OPT-320
    contract_refs: Vec<String>,  // OPT-315
    statistics: Vec<(String, i64, Option<String>)>, // BT-760 code, BT-759 count, issue-372 quality
    /// The legacy eras' `CONTRACT_AWARD_DATE`, on the award block (issue 255).
    decided: Option<(i64, i64, bool)>,
    /// Legacy award blocks name their winner(s) directly (inline
    /// `ADDRESS_CONTRACTOR`/`WINNER` → `ORG-n`) and carry the awarded value on
    /// the block itself — there is no bid/contract graph to resolve through
    /// (research §2.2: legacy notices have no notice-internal entity ids).
    direct_winners: Vec<String>, // ORG-n section ids
    direct_cents: Option<i64>,
    direct_currency: Option<String>,
}

#[derive(Default)]
struct RawBid {
    key: String,
    lot_key: Option<String>,   // BT-13714
    party_ref: Option<String>, // OPT-310
    cents: Option<i64>,        // BT-720
    currency: Option<String>,
    /// Issue 372: the notice declared BT-720 withheld for this bid, so `cents`
    /// is the SDK's -1 placeholder and not an offer.
    quality: Option<String>,
}

#[derive(Default)]
struct RawContract {
    key: String,
    buyer_contract_id: Option<String>,  // BT-150
    concluded: Option<(i64, i64, bool)>, // BT-145
    decided: Option<(i64, i64, bool)>,  // BT-1451
    bid_refs: Vec<String>,              // BT-3202
}

#[derive(Default)]
struct RawParty {
    key: String,
    /// (role, ORG section): members via OPT-300-Tenderer, subcontractors via
    /// OPT-301-Tenderer-SubCont.
    members: Vec<(String, String)>,
}

fn read_results(
    sections: &HashMap<&str, &store::Section>,
    parsed: &Parsed,
    legacy: bool,
    sdk01: bool,
) -> RawResults {
    if legacy {
        return read_legacy_results(sections, parsed);
    }
    if sdk01 {
        return read_sdk01_results(parsed);
    }
    // Issue 372: BT-720 is the figure buyers withhold most, and it reaches the
    // canonical layer here rather than through the amount table, so the marker
    // has to be applied on this path too or the bids satellite keeps asserting
    // -0.01 offers. Same rule as the amounts side: the notice's declaration for
    // THIS section and field, never the shape of the number.
    let withheld = withheld_source_fields(parsed);

    let mut raw = RawResults::default();
    for s in &parsed.sections {
        match s.kind.as_str() {
            "LotResult" => raw.lot_results.push(RawLotResult { key: s.id.clone(), ..RawLotResult::default() }),
            "LotTender" => raw.bids.push(RawBid { key: s.id.clone(), ..RawBid::default() }),
            "SettledContract" => raw.contracts.push(RawContract { key: s.id.clone(), ..RawContract::default() }),
            "TenderingParty" => raw.parties.push(RawParty { key: s.id.clone(), ..RawParty::default() }),
            _ => {}
        }
    }

    // BT-759 (count) and BT-760 (type) pair inside one ReceivedSubmissions
    // block; pair by that block's section, then attach to the enclosing result.
    let mut stats: BTreeMap<&str, (Option<&str>, Option<i64>, &str)> = BTreeMap::new();
    for row in &parsed.values {
        let Some(owner) = enclosing(sections, &row.section_id, RESULT_KINDS) else { continue };
        match sections[owner].kind.as_str() {
            "LotResult" => {
                let Some(r) = raw.lot_results.iter_mut().find(|r| r.key == owner) else { continue };
                match (stem(&row.field_id), &row.value) {
                    ("BT-142", NoticeValue::Code { code, .. }) => r.decision = Some(code.clone()),
                    ("BT-144", NoticeValue::Code { code, .. }) => r.reason = Some(code.clone()),
                    ("BT-13713", NoticeValue::Id { value, .. }) => r.lot_key = Some(value.clone()),
                    ("OPT-320", NoticeValue::Id { value, .. }) => r.bid_refs.push(value.clone()),
                    ("OPT-315", NoticeValue::Id { value, .. }) => r.contract_refs.push(value.clone()),
                    ("BT-759", NoticeValue::Number { value, .. }) => {
                        stats.entry(row.section_id.as_str()).or_insert((None, None, owner)).1 =
                            Some(*value as i64);
                    }
                    ("BT-759", NoticeValue::Integer(value)) => {
                        stats.entry(row.section_id.as_str()).or_insert((None, None, owner)).1 =
                            Some(*value);
                    }
                    ("BT-760", NoticeValue::Code { code, .. }) => {
                        stats.entry(row.section_id.as_str()).or_insert((None, None, owner)).0 =
                            Some(code.as_str());
                    }
                    _ => {}
                }
            }
            "LotTender" => {
                let Some(b) = raw.bids.iter_mut().find(|b| b.key == owner) else { continue };
                match (stem(&row.field_id), &row.value) {
                    ("BT-720", NoticeValue::Amount { cents, currency }) => {
                        b.cents = Some(*cents);
                        b.currency = Some(currency.clone());
                        b.quality = withheld
                            .contains(&(row.section_id.as_str(), "BT-720"))
                            .then(|| QUALITY_WITHHELD.to_owned());
                    }
                    ("BT-13714", NoticeValue::Id { value, .. }) => b.lot_key = Some(value.clone()),
                    ("OPT-310", NoticeValue::Id { value, .. }) => b.party_ref = Some(value.clone()),
                    _ => {}
                }
            }
            "SettledContract" => {
                let Some(c) = raw.contracts.iter_mut().find(|c| c.key == owner) else { continue };
                match (stem(&row.field_id), &row.value) {
                    ("BT-150", NoticeValue::Id { value, .. }) => {
                        c.buyer_contract_id = Some(value.clone());
                    }
                    ("BT-145", NoticeValue::Date { utc_seconds, offset_minutes, has_time }) => {
                        c.concluded = Some((*utc_seconds, *offset_minutes, *has_time));
                    }
                    // The winner-DECISION date, which the SDK scopes to the settled
                    // contract rather than to the LotResult (issue 255). A different fact
                    // from BT-145's signature date and published beside it in every
                    // committed CAN fixture.
                    ("BT-1451", NoticeValue::Date { utc_seconds, offset_minutes, has_time }) => {
                        c.decided = Some((*utc_seconds, *offset_minutes, *has_time));
                    }
                    ("BT-3202", NoticeValue::Id { value, .. }) => c.bid_refs.push(value.clone()),
                    _ => {}
                }
            }
            "TenderingParty" => {
                let Some(p) = raw.parties.iter_mut().find(|p| p.key == owner) else { continue };
                match (row.field_id.as_str(), &row.value) {
                    ("OPT-300-Tenderer", NoticeValue::Id { value, .. }) => {
                        p.members.push(("tenderer".to_owned(), value.clone()));
                    }
                    ("OPT-301-Tenderer-SubCont", NoticeValue::Id { value, .. }) => {
                        p.members.push(("subcontractor".to_owned(), value.clone()));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    // `into_iter`, not `into_values`: the KEY is the ReceivedSubmissions section,
    // and issue 372 needs it to ask whether this notice declared BT-759 or BT-760
    // withheld for THIS block. Either declaration marks the row, because the row is
    // the pair — a withheld count with a published type is still not a statistic,
    // and the fixture shows publishers declaring both together (`rec-sub-cou` and
    // `rec-sub-typ` side by side).
    for (section, (code, count, owner)) in stats {
        if let (Some(code), Some(count)) = (code, count)
            && let Some(r) = raw.lot_results.iter_mut().find(|r| r.key == owner)
        {
            let suppressed = withheld.contains(&(section, "BT-759"))
                || withheld.contains(&(section, "BT-760"));
            r.statistics.push((
                code.to_owned(),
                count,
                suppressed.then(|| QUALITY_WITHHELD.to_owned()),
            ));
        }
    }
    raw
}

/// Legacy award blocks (`AWARD_CONTRACT`/`RESULTS` → `RES-n`) read as
/// LotResults. The winner is the inline contractor address block, the awarded
/// value sits on the block, and the received-bid count is the one statistic —
/// there is no bid/contract graph in the legacy schema (research §2.2), so
/// those stay empty and the winner/value resolve directly.
fn read_legacy_results(sections: &HashMap<&str, &store::Section>, parsed: &Parsed) -> RawResults {
    let mut raw = RawResults::default();
    for s in &parsed.sections {
        if s.kind == "LotResult" {
            raw.lot_results.push(RawLotResult { key: s.id.clone(), ..RawLotResult::default() });
        }
    }
    if raw.lot_results.is_empty() {
        return raw;
    }

    // Published lot number → the Lot section it labels: legacy links a result to
    // its lot positionally by LOT_NO (research §2.2), not by a section id-ref.
    let mut lot_by_no: HashMap<String, String> = HashMap::new();
    for value in &parsed.values {
        let is_lot = sections
            .get(value.section_id.as_str())
            .is_some_and(|s| LOT_KINDS.contains(&s.kind.as_str()));
        if is_lot
            && matches!(value.field_id.as_str(), "TED-LOT_NO" | "TED-LOT_NUMBER" | "TED-ITEM")
            && let NoticeValue::Id { value: no, .. } = &value.value
        {
            lot_by_no.entry(no.trim().to_owned()).or_insert_with(|| value.section_id.clone());
        }
    }

    for value in &parsed.values {
        let Some(owner) = enclosing(sections, &value.section_id, RESULT_KINDS) else { continue };
        let Some(r) = raw.lot_results.iter_mut().find(|r| r.key == owner) else { continue };
        match (value.field_id.as_str(), &value.value) {
            // Any inline organization reference in an award block is a winner
            // (contractors, joint AWARDED_TO_GROUP members); the OJS chain edges
            // are scheme "ojs" and never appear here.
            (_, NoticeValue::Id { value: org, is_ref: true, scheme })
                if scheme.as_deref() != Some("ojs") =>
            {
                r.direct_winners.push(org.clone());
            }
            ("TED-LOT_NO" | "TED-LOT_NUMBER" | "TED-ITEM", NoticeValue::Id { value: no, .. }) => {
                r.lot_key = lot_by_no.get(no.trim()).cloned();
            }
            // The awarded value. R2.0.9 writes `VAL_TOTAL`; R2.0.8/defence forms
            // write the locale-formatted `VALUE_COST` (research §2.5) — take it
            // only when VAL_TOTAL is absent, and never the prefixed
            // initial-estimate variant.
            ("TED-VAL_TOTAL", NoticeValue::Amount { cents, currency }) => {
                r.direct_cents = Some(*cents);
                r.direct_currency = Some(currency.clone());
            }
            ("TED-VALUE_COST", NoticeValue::Amount { cents, currency }) if r.direct_cents.is_none() => {
                r.direct_cents = Some(*cents);
                r.direct_currency = Some(currency.clone());
            }
            // When the buyer decided (issue 255). The legacy forms put it inside the
            // award block, which is this LotResult, and they publish no contract graph
            // for eForms' contract-scoped BT-1451 to land on. The r209 defence form
            // splits it into DAY/MONTH/YEAR elements; the parse layer has already made
            // that one instant.
            (
                LEGACY_AWARD_DATE_FIELD | LEGACY_AWARD_DATE_FIELD_R207,
                NoticeValue::Date { utc_seconds, offset_minutes, has_time },
            ) => {
                r.decided = Some((*utc_seconds, *offset_minutes, *has_time));
            }
            (LEGACY_NO_AWARD_MARKER, NoticeValue::Integer(_)) => r.decision = Some("clos-nw".to_owned()),
            (f, NoticeValue::Integer(n)) if LEGACY_BID_COUNT_FIELDS.contains(&f) => {
                r.statistics.push(("tenders".to_owned(), *n, None));
            }
            (f, NoticeValue::Number { value: n, .. }) if LEGACY_BID_COUNT_FIELDS.contains(&f) => {
                r.statistics.push(("tenders".to_owned(), *n as i64, None));
            }
            _ => {}
        }
    }

    // Decide from the evidence: a named winner or an awarded value is a win. A
    // result with NEITHER but with an award DATE stays NULL — the publisher
    // announced an award and withheld its outcome, and reading that silence as
    // `clos-nw` asserts a closure the source never published (the issue-257 rule,
    // and what issue 244's slice 9 mints for the era's winner-silent award bodies).
    // `clos-nw` remains the default only for a result block with no award evidence
    // at all.
    for r in &mut raw.lot_results {
        r.direct_winners.sort();
        r.direct_winners.dedup();
        if r.decision.is_none() {
            if !r.direct_winners.is_empty() || r.direct_cents.is_some() {
                r.decision = Some("selec-w".to_owned());
            } else if r.decided.is_none() {
                r.decision = Some("clos-nw".to_owned());
            }
        }
    }
    raw
}

/// DÖE sdk-0.1 results (issue 29): each `TenderResult` section is a LotResult
/// whose winner(s) are the inline `WinningParty` sections beneath it (resolved as
/// direct winners, exactly like the legacy inline award blocks), and whose
/// decision is the `TenderResultCode`. sdk-0.1 carries no notice-internal
/// bid/contract graph and no lot reference on the result, so those stay empty and
/// the result is Tender-scoped.
///
/// What this dialect mostly publishes is a date and nothing else (issue 257).
/// Across 2023-01, 2023-06 and 2024-06 of the DÖE archive, every award-type
/// notice carries a `TenderResult` — the density is exactly 100 %, which is the
/// serializer, not richness — but only 13.5 %, 15.1 % and 1.9 % of them carry a
/// `WinningParty`. The rest are `<TenderResult><AwardDate/><AwardTime/></>`: the
/// day of the award, no code, no winner, no value. So the winner shortfall this
/// era shows is the publisher's, not ours — where a `WinningParty` IS published
/// we resolve it, and every one of them carried a `PartyName` to resolve.
fn read_sdk01_results(parsed: &Parsed) -> RawResults {
    let mut raw = RawResults::default();
    for s in &parsed.sections {
        if s.kind == SDK01_RESULT_KIND {
            raw.lot_results.push(RawLotResult { key: s.id.clone(), ..RawLotResult::default() });
        }
    }
    if raw.lot_results.is_empty() {
        return raw;
    }
    // The decision code and the award date both hang on the TenderResult section.
    for value in &parsed.values {
        let Some(r) = raw.lot_results.iter_mut().find(|r| r.key == value.section_id) else {
            continue;
        };
        match &value.value {
            NoticeValue::Code { code, .. } if value.field_id == SDK01_RESULT_CODE_FIELD => {
                r.decision = Some(code.clone());
            }
            NoticeValue::Date { utc_seconds, offset_minutes, has_time }
                if value.field_id == SDK01_AWARD_DATE_FIELD =>
            {
                r.decided = Some((*utc_seconds, *offset_minutes, *has_time));
            }
            _ => {}
        }
    }
    // Each WinningParty section is a direct winner of its parent TenderResult.
    for s in &parsed.sections {
        if s.kind == SDK01_WINNER_KIND
            && let Some(parent) = &s.parent
            && let Some(r) = raw.lot_results.iter_mut().find(|r| &r.key == parent)
        {
            r.direct_winners.push(s.id.clone());
        }
    }
    for r in &mut raw.lot_results {
        r.direct_winners.sort();
        r.direct_winners.dedup();
        // A named winner IS a selection, so infer that much. The other half of this
        // fallback used to read the OPPOSITE out of silence — `clos-nw`, documented
        // to the SQL sandbox as "closed, no award" — and on this dialect silence is
        // the norm rather than the exception: measured across three months of the
        // DÖE archive, ~90 % of sdk-0.1 award notices publish a `TenderResult`
        // carrying an AwardDate and nothing else. That fabricated a positive claim
        // of "no award" on ~125k notices which state the day the award was made
        // (issue 257). Unstated is now NULL, and the date it does state is kept.
        if r.decision.is_none() && !r.direct_winners.is_empty() {
            r.decision = Some("selec-w".to_owned());
        }
    }
    raw
}

impl RawResults {
    fn is_empty(&self) -> bool {
        self.lot_results.is_empty() && self.bids.is_empty() && self.contracts.is_empty()
    }

    fn bid(&self, key: &str) -> Option<&RawBid> {
        self.bids.iter().find(|b| b.key == key)
    }

    /// The Lot a results entity is about, through the notice's own graph —
    /// `None` when it does not resolve to exactly one lot.
    fn entity_lot(&self, key: &str) -> Option<&str> {
        if let Some(r) = self.lot_results.iter().find(|r| r.key == key) {
            return r.lot_key.as_deref();
        }
        if let Some(b) = self.bid(key) {
            return b.lot_key.as_deref();
        }
        if self.parties.iter().any(|p| p.key == key) {
            return unique(
                self.bids
                    .iter()
                    .filter(|b| b.party_ref.as_deref() == Some(key))
                    .filter_map(|b| b.lot_key.as_deref()),
            );
        }
        if let Some(c) = self.contracts.iter().find(|c| c.key == key) {
            return unique(
                c.bid_refs.iter().filter_map(|r| self.bid(r)).filter_map(|b| b.lot_key.as_deref()),
            );
        }
        None
    }

    /// The Lot scope of an award-side role reference: the reference's nearest
    /// enclosing results entity, resolved to its lot.
    fn lot_of(&self, sections: &HashMap<&str, &store::Section>, source: &str) -> Option<String> {
        let entity = enclosing(sections, source, RESULT_KINDS)?;
        self.entity_lot(entity).map(str::to_owned)
    }

    /// Bind the graph onto canonical Organizations and resolve each result's
    /// winners and awarded value.
    fn bind(
        &self,
        notice_id: i64,
        logical_notice_id: Option<String>,
        orgs: &HashMap<&str, i64>,
    ) -> Round {
        let members_of = |party_ref: Option<&str>| -> Vec<BidParty> {
            let mut parties: Vec<BidParty> = party_ref
                .and_then(|k| self.parties.iter().find(|p| p.key == k))
                .map(|p| {
                    p.members
                        .iter()
                        .filter_map(|(role, section)| {
                            orgs.get(section.as_str()).map(|&organization_id| BidParty {
                                role: role.clone(),
                                organization_id,
                                section_id: section.clone(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            parties.sort();
            parties.dedup();
            parties
        };

        let bids = self
            .bids
            .iter()
            .map(|b| BidState {
                key: b.key.clone(),
                lot_key: b.lot_key.clone(),
                cents: b.cents,
                currency: b.currency.clone(),
                quality: b.quality.clone(),
                parties: members_of(b.party_ref.as_deref()),
            })
            .collect();

        let contracts = self
            .contracts
            .iter()
            .map(|c| {
                // A contract's value is the value of the Bid(s) it settled —
                // eForms contracts carry no value of their own.
                let (cents, currency) =
                    single_currency_total(c.bid_refs.iter().filter_map(|r| self.bid(r)));
                ContractState {
                    key: c.key.clone(),
                    buyer_contract_id: c.buyer_contract_id.clone(),
                    concluded: c.concluded,
                    decided: c.decided,
                    cents,
                    currency,
                }
            })
            .collect();

        let lot_results = self
            .lot_results
            .iter()
            .map(|r| {
                // The winning Bids: the ones this result's contracts settled —
                // real eSenders list *all* received tenders under OPT-320, so a
                // settled contract is the stronger winner signal — falling back
                // to the result's own tender references when no contract is
                // linked yet (framework awards publish winners without one).
                let contract_bids: Vec<&RawBid> = r
                    .contract_refs
                    .iter()
                    .filter_map(|cr| self.contracts.iter().find(|c| &c.key == cr))
                    .flat_map(|c| c.bid_refs.iter())
                    .filter_map(|br| self.bid(br))
                    .collect();
                let winning: Vec<&RawBid> = if contract_bids.is_empty() {
                    r.bid_refs.iter().filter_map(|br| self.bid(br)).collect()
                } else {
                    contract_bids
                };
                // Legacy blocks carry the awarded value and the winner(s)
                // directly; eForms resolves them through the bid/contract graph.
                let (cents, currency) = if r.direct_cents.is_some() {
                    (r.direct_cents, r.direct_currency.clone())
                } else {
                    single_currency_total(winning.iter().copied())
                };
                let mut winners: Vec<i64> = if !r.direct_winners.is_empty() {
                    r.direct_winners.iter().filter_map(|s| orgs.get(s.as_str()).copied()).collect()
                // An UNSTATED decision must not suppress a winner the notice names
                // (issue 100). BT-142 "Winner Chosen" is ERROR-severity and
                // non-repeatable in the SDK, so standard eForms always carries it and
                // this arm is unchanged there — but eForms-DE 1.x publishes no
                // `TenderResultCode` at all, and gating on it silently dropped ~60k
                // award notices' winners whose whole chain resolves. A LotResult that
                // REFERENCES a tender (OPT-320, the SDK's "Tender Identifier
                // Reference") is referring to the tender that won: there is no
                // mechanism for a result to reference the tenders that lost — those are
                // counted in ReceivedSubmissionsStatistics, never referenced. A decision
                // that positively says otherwise (`clos-nw`, `no-rece`, `open-nw`) still
                // suppresses, which is the case this gate was protecting.
                } else if matches!(r.decision.as_deref(), Some("selec-w") | None) {
                    winning
                        .iter()
                        .flat_map(|b| members_of(b.party_ref.as_deref()))
                        .filter(|p| p.role == "tenderer")
                        .map(|p| p.organization_id)
                        .collect()
                } else {
                    Vec::new()
                };
                winners.sort_unstable();
                winners.dedup();
                LotResultState {
                    key: r.key.clone(),
                    lot_key: r.lot_key.clone(),
                    decision: r.decision.clone(),
                    reason: r.reason.clone(),
                    awarded_cents: cents,
                    awarded_currency: currency,
                    decided: r.decided,
                    winners,
                    statistics: r.statistics.clone(),
                }
            })
            .collect();

        Round { notice_id, logical_notice_id, lot_results, bids, contracts }
    }
}

/// Sum bid values when they agree on one currency — anything mixed yields no
/// value rather than a wrong one.
fn single_currency_total<'a>(
    bids: impl Iterator<Item = &'a RawBid>,
) -> (Option<i64>, Option<String>) {
    let mut total = 0;
    let mut currency: Option<&str> = None;
    for bid in bids {
        let (Some(cents), Some(c)) = (bid.cents, bid.currency.as_deref()) else { continue };
        if currency.is_some_and(|have| have != c) {
            return (None, None);
        }
        currency = Some(c);
        total += cents;
    }
    (currency.map(|_| total), currency.map(str::to_owned))
}

/// The single distinct item of an iterator, or `None`.
fn unique<'a>(items: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let mut found = None;
    for item in items {
        match found {
            None => found = Some(item),
            Some(have) if have == item => {}
            Some(_) => return None,
        }
    }
    found
}

/// The business-term stem of a source field id: `BT-21-Lot` → `BT-21`,
/// `BT-131(d)-Lot` → `BT-131(d)`.
fn stem(field_id: &str) -> &str {
    let mut parts = field_id.match_indices('-');
    parts.next();
    match parts.next() {
        Some((i, _)) => &field_id[..i],
        None => field_id,
    }
}

/// `BT-195(BT-161)-NoticeResult` -> `BT-161`: the SOURCE field a withheld-field
/// declaration names. `None` for anything that is not a BT-195 declaration.
fn withheld_source(field_id: &str) -> Option<&str> {
    let rest = field_id.strip_prefix("BT-195(")?;
    let end = rest.find(')')?;
    Some(&rest[..end])
}

/// Every withheld-field declaration of one notice, as `(section, source field)`.
///
/// Under BT-195/`FieldsPrivacy` (ADR-0013 D5) a buyer may suppress a publishable
/// value. The notice then carries a `FieldsPrivacy` block anchored under the
/// section whose value is suppressed, and inside it a `BT-195(<source>)-<context>`
/// code naming WHICH field went unpublished -- while the SDK writes the literal
/// `-1` into the numeric slot it left empty and `unpublished` into the sibling
/// code slot. A projection that copies that number asserts a value the notice
/// took care to say it was NOT publishing, which is issue 372.
///
/// The pair is (the block's PARENT section, the parenthesised source id): the
/// parent because that is the section holding the suppressed value, and the bare
/// source id because a fact's own `field_id` carries it as its [`stem`]. So the
/// test at emission is one set lookup on `(value.section_id, stem(field_id))`,
/// needing no new vocabulary -- the correspondence is the identity, which is what
/// made unit 2 of 372 cheap. Verified against the committed withheld fixture in
/// `withheld_declarations_pair_with_the_section_holding_the_suppressed_value`:
/// all five of its blocks anchor exactly this way, across three value channels.
///
/// Deliberately EXACT on the section rather than notice-wide. Two consequences,
/// and both are the ones to want:
///
/// - a sibling field in the same section is NOT marked (the fixture's
///   `BT-5421-Lot` weight-type keeps its value while `BT-541` beside it is
///   marked), which is the per-row precision issue 372 unit 2 was decided on --
///   a blanket rule keyed on the number `-1` would re-commit this issue's own
///   mistake of treating a value as self-describing;
/// - a block a publisher hoisted elsewhere in the notice (issue 195 saw
///   `FieldsPrivacy` written under the root extension) marks nothing. That miss
///   is the SAFE direction -- an unmarked withheld row behaves exactly as it does
///   today, refused by issue 366's negative-sentinel rule -- and it is measured
///   rather than assumed: section 11 of the weekly report counts rows whose
///   notice declared SOME withholding, so the gap against the marked count is
///   readable.
fn withheld_source_fields(parsed: &Parsed) -> BTreeSet<(&str, &str)> {
    let privacy: HashMap<&str, &str> = parsed
        .sections
        .iter()
        .filter(|s| s.kind == "FieldsPrivacy")
        .filter_map(|s| s.parent.as_deref().map(|parent| (s.id.as_str(), parent)))
        .collect();
    parsed
        .values
        .iter()
        // A declaration is a CODE (`non-publication-identifier`). The field id is
        // what discriminates, so the list name is not required -- but the channel
        // is, or a publisher echoing the id in prose would read as a declaration.
        .filter(|v| matches!(v.value, NoticeValue::Code { .. }))
        .filter_map(|v| {
            let parent = privacy.get(v.section_id.as_str())?;
            Some((*parent, withheld_source(&v.field_id)?))
        })
        .collect()
}

/// Which value table a parsed value came from — the channel the projection
/// dispatches on (`NoticeState::read`'s `match &value.value`). The projection
/// reads a field id through ONE channel, so "is this id read?" is only a
/// meaningful question with the channel attached.
///
/// The distinction is load-bearing rather than tidy: [`role_name`] accepts ANY
/// `TED-`-prefixed id, so a channel-blind predicate would report every legacy
/// field as read — including the whole titleless-r208 cohort that issue 368 is
/// about, which is exactly the era such a predicate would have to be honest in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Text,
    Code,
    Classification,
    Amount,
    Date,
    Integer,
    Number,
    /// `is_ref` splits a cross-section pointer (a role, a result edge) from a
    /// published identifier; the projection reads the two through different code.
    Id { is_ref: bool },
}

/// The stems the results graph reads, by the channel each arrives on
/// (`read_results`, `read_legacy_results`, `read_sdk01_results`).
const RESULT_ID_STEMS: &[&str] =
    &["BT-13713", "OPT-320", "OPT-315", "BT-13714", "OPT-310", "BT-150", "BT-3202", "OPT-300"];
const RESULT_CODE_STEMS: &[&str] = &["BT-142", "BT-144", "BT-760"];
const RESULT_DATE_STEMS: &[&str] = &["BT-145", "BT-1451"];
const RESULT_AMOUNT_STEMS: &[&str] = &["BT-720", "BT-161"];
const RESULT_NUMBER_STEMS: &[&str] = &["BT-759"];

/// Does the projection read this field id on this channel — does the value have
/// anywhere to go?
///
/// **This is the union the codebase had only ever written inside two test
/// gates.** `every_de1_alias_target_is_a_field_the_projection_reads` and
/// `ubl_grafts_are_all_mapped_or_ignored` each open-coded their own version, and
/// neither was reachable from production, so nothing could COUNT what the fold
/// drops — which is how 29,455 titleless r208 Tenders and whole eras of lot
/// titles stood unnoticed until an external reviewer looked (issue 368). Both
/// gates now call this, so the answer they enforce and the answer a diagnostic
/// reports cannot drift apart.
///
/// DE-1.x alias sources resolve to their eForms target first: the notice layer
/// keeps the publisher's `DE1-*` spelling on purpose (see [`DE1_FIELD_ALIASES`]),
/// so a reader that skipped this step would call the entire eForms-DE 1.x
/// vocabulary unread.
///
/// Deliberately does NOT model scope. A value can satisfy this and still be
/// dropped because its Lot section is missing (`NoticeState::read`'s
/// `lots.get_mut(key)` miss), so a `true` here means "the vocabulary knows this
/// id", not "this particular row landed".
pub fn has_destination(field_id: &str, channel: Channel) -> bool {
    let field_id = DE1_FIELD_ALIASES
        .iter()
        .find(|(de1, _)| *de1 == field_id)
        .map_or(field_id, |(_, target)| *target);
    let stem = stem(field_id);
    match channel {
        Channel::Text => {
            canonical_name(TEXTS, field_id).is_some()
                || field_id == OJ_HEADING_FIELD
                || field_id == ORG_NAME_FIELD
                || ORG_NAME_FIELDS.contains(&field_id)
                || SDK01_PARTY_NAME_FIELDS.contains(&field_id)
        }
        Channel::Amount => {
            canonical_name(AMOUNTS, field_id).is_some()
                || RESULT_AMOUNT_STEMS.contains(&stem)
                || field_id.ends_with(AMOUNT_ELEMENT)
        }
        Channel::Classification => canonical_name(CLASSIFICATIONS, field_id).is_some(),
        Channel::Date => {
            canonical_name(DATES, field_id).is_some()
                || PUBLICATION_DATE_FIELDS.contains(&field_id)
                || DISPATCH_DATE_FIELDS.contains(&field_id)
                || RESULT_DATE_STEMS.contains(&stem)
                || field_id == SDK01_AWARD_DATE_FIELD
                || field_id == LEGACY_AWARD_DATE_FIELD
                || field_id == LEGACY_AWARD_DATE_FIELD_R207
        }
        Channel::Code => {
            field_id == SUBTYPE_FIELD
                || field_id == ORG_COUNTRY_FIELD
                || field_id == SDK01_RESULT_CODE_FIELD
                || ORG_COUNTRY_FIELDS.contains(&field_id)
                || SDK01_PARTY_COUNTRY_FIELDS.contains(&field_id)
                || TAX_BASIS_FIELDS.contains(&field_id)
                || ORIGINAL_LANG_FIELDS.contains(&field_id)
                || RESULT_CODE_STEMS.contains(&stem)
        }
        Channel::Integer => {
            LEGACY_BID_COUNT_FIELDS.contains(&field_id)
                || field_id.ends_with(AMOUNT_ELEMENT)
                || field_id == LEGACY_NO_AWARD_MARKER
        }
        // The legacy bid count arrives as an Integer OR a Number (the reader takes
        // both), so it has a destination on both channels.
        Channel::Number => {
            RESULT_NUMBER_STEMS.contains(&stem) || LEGACY_BID_COUNT_FIELDS.contains(&field_id)
        }
        // A pointer: roles and the result graph's edges.
        Channel::Id { is_ref: true } => {
            role_name(field_id).is_some() || RESULT_ID_STEMS.contains(&stem)
        }
        // A published identifier the projection keys or binds on.
        Channel::Id { is_ref: false } => {
            field_id == PROCEDURE_KEY_FIELD
                || field_id == LOGICAL_NOTICE_FIELD
                || field_id == PREVIOUS_NOTICE_FIELD
                || field_id == ORG_IDENTIFIER_FIELD
                || field_id == ORG_NATIONALID_FIELD
                || field_id == SDK01_FOLDER_FIELD
                || field_id == DE1_FOLDER_FIELD
                || field_id == GROUP_ID_FIELD
                || field_id == GROUP_MEMBER_FIELD
                || LEGACY_OWN_NUMBER_FIELDS.contains(&field_id)
                || RESULT_ID_STEMS.contains(&stem)
        }
    }
}

/// The channel(s) one notice-layer satellite table feeds, keyed by the table's
/// name, so a diagnostic that found a row in `notice_texts` asks the TEXT
/// question of its field id — [`table_reads`] — rather than whether some
/// channel somewhere reads that id.
///
/// The distinction decided a live result. The r208 probe's first run
/// (2026-09-12) returned **311 published field ids and 0 unmapped**, with four
/// title elements nothing reads among the 311, because its sieve was the
/// channel-blind [`any_channel_reads`]: `role_name` accepts any `TED-` id, so
/// the pointer channel alone answered "read" for the whole legacy vocabulary.
/// The weekly report's section 13 used the same sieve and was blind the same
/// way — invisibly, since the corpus head is eForms and only a `TED-` id trips
/// it. An unknown table has no channel and so reads nothing: a typo lists that
/// table's every row as dropped instead of hiding them.
pub fn table_channels(table: &str) -> &'static [Channel] {
    match table {
        "notice_texts" => &[Channel::Text],
        "notice_codes" => &[Channel::Code],
        "notice_classifications" => &[Channel::Classification],
        "notice_amounts" => &[Channel::Amount],
        "notice_dates" => &[Channel::Date],
        "notice_integers" => &[Channel::Integer],
        "notice_numbers" => &[Channel::Number],
        "notice_ids" => &[Channel::Id { is_ref: true }, Channel::Id { is_ref: false }],
        _ => &[],
    }
}

/// Whether the projection reads `field_id` on the channel(s) that `table`
/// feeds — the sieve for "published and dropped" diagnostics (issue 368: the
/// weekly report's section 13 and `GET /admin/unmapped-fields`).
pub fn table_reads(table: &str, field_id: &str) -> bool {
    table_channels(table).iter().any(|c| has_destination(field_id, *c))
}

/// Whether ANY channel reads this field id — the question the DE-1.x alias gate
/// asks, where the alias table names a target rather than a stored value.
///
/// **Not a sieve for stored rows, and private so it cannot become one again.**
/// A stored row arrived on ONE channel, and this asks about all of them: on the
/// pointer channel `role_name` accepts any `TED-` id, so through this predicate
/// the entire legacy vocabulary reads as read — the r208 probe reported 0
/// unmapped of 311 that way (2026-09-12). [`table_reads`] is the question a
/// diagnostic wants.
fn any_channel_reads(field_id: &str) -> bool {
    [
        Channel::Text,
        Channel::Code,
        Channel::Classification,
        Channel::Amount,
        Channel::Date,
        Channel::Integer,
        Channel::Number,
        Channel::Id { is_ref: true },
        Channel::Id { is_ref: false },
    ]
    .iter()
    .any(|c| has_destination(field_id, *c))
}

/// Map a source field to its canonical name, matching the **full field id**
/// first, then its [`stem`]. BT-/TED-/TXT- codes are keyed by stem (`BT-21` for
/// `BT-21-Lot`); the DÖE sdk-0.1 dialect's path-shaped ids
/// (`SDK01-ProcurementProject-Name` vs `-Description`) collide under the coarse
/// stem, so they are keyed by their full id instead — the full-id check wins for
/// them and is a harmless miss for everything else.
fn canonical_name(table: &[(&str, &str)], field_id: &str) -> Option<String> {
    table
        .iter()
        .find(|(source, _)| *source == field_id || *source == stem(field_id))
        .map(|(_, name)| (*name).to_owned())
}

/// The canonical language vocabulary for `Fact::Text.lang` (issue 292): ISO
/// 639-2/T three-letter uppercase — the form the eForms codelist publishes
/// (`ENG`, `DEU`, `FRA`, …). The other eras publish other dialects — r208/r209
/// the raw two-letter `LG` attribute (`EN`, `DE`), the text era `EN` — and
/// before this mapping every "English wins" pick in the read layer compared the
/// literal `'ENG'` and was inert for the whole pre-eForms corpus: title choice
/// fell to scan order, so a bilingual legacy tender could serve its non-English
/// title. Importers translate at the boundary (CONTEXT.md); this is that
/// boundary for language, applied once where parse-layer text becomes a fact,
/// so every era — and every future portal's dialect — funnels through one map.
/// An unknown tag passes through UPPERCASED: it fails visible (a tag the picks
/// simply ignore) instead of silently splitting one language across spellings.
pub fn normalize_lang(lang: Option<&str>) -> Option<String> {
    let up = lang?.to_ascii_uppercase();
    Some(
        match up.as_str() {
            // ISO 639-1 → 639-2/T for the languages the TED corpus publishes.
            "BG" => "BUL",
            "CS" => "CES",
            "DA" => "DAN",
            "DE" => "DEU",
            "EL" => "ELL",
            "EN" => "ENG",
            "ES" => "SPA",
            "ET" => "EST",
            "FI" => "FIN",
            "FR" => "FRA",
            "GA" => "GLE",
            "HR" => "HRV",
            "HU" => "HUN",
            "IS" => "ISL",
            "IT" => "ITA",
            "LT" => "LIT",
            "LV" => "LAV",
            "MK" => "MKD",
            "MT" => "MLT",
            "NL" => "NLD",
            "NO" => "NOR",
            "PL" => "POL",
            "PT" => "POR",
            "RO" => "RON",
            "RU" => "RUS",
            "SK" => "SLK",
            "SL" => "SLV",
            "SQ" => "SQI",
            "SR" => "SRP",
            "SV" => "SWE",
            "TR" => "TUR",
            "UK" => "UKR",
            _ => return Some(up),
        }
        .to_owned(),
    )
}

/// The matcher's N2 name key (issue 300 §2.3): Unicode lowercase, every
/// non-alphanumeric character folded to a space, runs collapsed. This is
/// deliberately NOT `organizations.name_norm` (the 234 reuse key, bare
/// `to_lowercase`) — changing that would silently re-key the provisional
/// probe. Folding through the alphanumeric filter subsumes the design's
/// punctuation/quote/dash/whitespace classes in one rule; NFKC is deferred
/// until something measured demands it (fullwidth/ligature forms — the same
/// nothing-measured-demands-it line the design draws for diacritics, which
/// are intentionally preserved: "gymnázium" must not collide with
/// "gymnazium" across languages).
///
/// One scripted exception to "diacritics preserved" (issue 346): Greek. Greek
/// orthography writes the tonos/dialytika in lower and mixed case and DROPS
/// them in ALL CAPS, so `Δήμος Αβδήρων` and `ΔΗΜΟΣ ΑΒΔΗΡΩΝ` — one municipality,
/// two casings — lower-cased to two keys differing on every accented vowel
/// (22 of the 210 Greek same-identifier duplicate groups on 2026-09-04, and
/// issue 329's specimen 311/1079). Latin-script upper-casing keeps its marks,
/// so the fold is scoped to the precomposed Greek letters and the final
/// sigma (`Σ` lower-cases to `σ`, the mixed-case spelling ends in `ς`); a
/// Latin `ü` still does not meet a `u`. Polytonic (U+1F00–U+1FFF) and
/// decomposed (NFD) Greek are not folded — nothing measured carries them.
pub fn match_norm(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut gap = false;
    for c in name.chars() {
        if c.is_alphanumeric() {
            if gap && !out.is_empty() {
                out.push(' ');
            }
            gap = false;
            out.extend(c.to_lowercase().map(fold_greek_tonos));
        } else {
            gap = true;
        }
    }
    out
}

/// The Greek half of [`match_norm`]'s case fold: a lower-case Greek vowel with
/// tonos and/or dialytika to its bare letter, and the final sigma to the
/// medial one. Everything else passes through unchanged.
fn fold_greek_tonos(c: char) -> char {
    match c {
        'ά' => 'α',
        'έ' => 'ε',
        'ή' => 'η',
        'ί' | 'ϊ' | 'ΐ' => 'ι',
        'ό' => 'ο',
        'ύ' | 'ϋ' | 'ΰ' => 'υ',
        'ώ' => 'ω',
        'ς' => 'σ',
        other => other,
    }
}

/// The canonical target of one parsed amount. Everything except r208's plain
/// `VALUE_COST` maps by field id alone ([`AMOUNTS`]). `VALUE_COST` is three
/// facts in one field id (issue 177), told apart only by context:
///
/// - inside an award block it is the awarded value, and the results binder owns
///   it — mapping it here would re-file every award value as a tender estimate;
/// - at object scope on an award-family notice it is the II.2 TOTAL FINAL value
///   (`TOTAL_FINAL_VALUE`), the r208 spelling of r209's `VAL_TOTAL`;
/// - at object scope on a contract notice it is the II.2.1 estimate
///   (`COSTS_RANGE_AND_CURRENCY`), the r208 spelling of `VAL_ESTIMATED_TOTAL`.
///
/// The prefixed variants fall through to the table:
/// `TED-TOTAL_ESTIMATED.VALUE_COST` (the framework block's estimate, a headline
/// value in its own right) maps there; the award block's
/// `TED-INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT.VALUE_COST` matches nothing and
/// stays unprojected — a restated copy loses to the form value, the issue-174
/// precedent. `RANGE_VALUE_COST` (low/high ranges) also stays unprojected: a
/// range is not one estimate (decision recorded on issue 177).
/// The tax basis stated beside one form-era amount, or `None` (issue 251).
///
/// `None` covers three cases that all mean the same thing downstream — the source stated
/// no basis, it stated both, or the section holds two amounts under this id so the marker
/// cannot be attributed. NULL means "not stated" in the column either way.
fn marker_basis(
    section: &str,
    amount_field: &str,
    markers: &std::collections::BTreeSet<(&str, &str)>,
    amount_counts: &std::collections::BTreeMap<(&str, &str), usize>,
) -> Option<String> {
    let stem = amount_field.strip_suffix(AMOUNT_ELEMENT)?;
    if amount_counts.get(&(section, amount_field)).copied().unwrap_or(0) > 1 {
        return None;
    }
    let mut found: Option<&str> = None;
    for (element, basis) in BASIS_MARKERS {
        let id = format!("{stem}{element}");
        if markers.contains(&(section, id.as_str())) {
            if found.replace(basis).is_some() {
                return None; // both bases marked: the notice states neither clearly
            }
        }
    }
    found.map(str::to_owned)
}

fn amount_target(
    field_id: &str,
    sections: &HashMap<&str, &store::Section>,
    section_id: &str,
    has_results: bool,
) -> Option<String> {
    if field_id != "TED-VALUE_COST" {
        return canonical_name(AMOUNTS, field_id);
    }
    if enclosing(sections, section_id, RESULT_KINDS).is_some() {
        return None;
    }
    Some(if has_results { "result_value" } else { "estimated_value" }.to_owned())
}

/// The role an id-ref names. The OPT-300/301 families are eForms' organization
/// references (`OPT-300-Procedure-Buyer` → `Procedure-Buyer`); the legacy
/// profiles name the role by the address-block element itself
/// (`TED-ADDRESS_CONTRACTOR`), which is folded onto the canonical role names.
/// OJS chain edges are handled before this is reached, so a `TED-` reference
/// here is always an organization role.
fn role_name(field_id: &str) -> Option<String> {
    for prefix in ["OPT-300-", "OPT-301-"] {
        if let Some(rest) = field_id.strip_prefix(prefix) {
            return Some(rest.to_owned());
        }
    }
    field_id.strip_prefix("TED-").map(legacy_role)
}

/// The roles that mean "this notice's buyer", across the dialects (issue 369 unit 2).
/// eForms names the role by its OPT-300 suffix (`Procedure-Buyer`); the legacy address
/// blocks and sdk-0.1's `ContractingParty` both arrive as `buyer`, through
/// [`legacy_role`] and the sdk01 synthesis in `NoticeState::read` respectively.
const BUYER_ROLES: &[&str] = &["Procedure-Buyer", "buyer"];

/// The buyer identity this notice publishes, as a stable key for issue 369's key
/// election — `None` when it names no buyer at all.
///
/// **A SET, sorted and joined, not a single buyer.** A notice can legitimately name
/// several: a joint procurement lists a central purchasing body beside its
/// participating authorities. Keying on the SET is what makes the gate's
/// `count(DISTINCT buyer_key) >= 3` mean the right thing — a joint procurement repeats
/// ONE set across its notices and is admitted, while the welds this gate exists for
/// carry buyers DISJOINT across versions (tender 1: seq 1–2 Klinikum Neumarkt, 3–4
/// Land BW, 5–7 BG Holz und Metall) and so present three distinct sets. Counting
/// individual buyers instead would refuse the joint procurement, which is the failure
/// the issue's "keep the shape pre-filter" argument is about; this encoding avoids it
/// without a second column.
///
/// The known gap, bounded rather than solved: notices naming overlapping but UNEQUAL
/// subsets (`{X,Y}`, then `{X,Y,Z}`, then `{X,Z}`) present three sets and would be
/// refused despite sharing buyers. Inside the shape pre-filter's reach — the 14
/// measured tenders, whose welded members are disjoint — that cannot arise. If the
/// pre-filter is ever dropped, the predicate must become pairwise-disjoint SETS rather
/// than a count, exactly as the issue already records.
///
/// **Parsed-side by decision.** The buyer's PUBLISHED identifier, through the same
/// normaliser the resolver binds on, never the resolved `organizations` row: the
/// planner must not take a dependency on the org layer to decide TENDER identity, and
/// the `>= 3` threshold already absorbs the duplication that layer would add (2 is the
/// measured org-duplicate floor). The census used resolved rows only because that is
/// all a read-only probe could reach.
fn buyer_key(sdk01: bool, notice_id: i64, parsed: &Parsed) -> Option<String> {
    let mut sections: BTreeSet<&str> = BTreeSet::new();
    if sdk01 {
        for section in &parsed.sections {
            if section.kind == SDK01_BUYER_KIND {
                sections.insert(section.id.as_str());
            }
        }
    }
    for value in &parsed.values {
        // An id-ref's VALUE is the organization section it points at — the same
        // `target` `bind_organizations` resolves through `by_section`. The `ojs`
        // scheme is a chain edge, not a role reference.
        if let NoticeValue::Id { value: target, is_ref: true, scheme } = &value.value
            && scheme.as_deref() != Some("ojs")
            && role_name(&value.field_id).is_some_and(|r| BUYER_ROLES.contains(&r.as_str()))
        {
            sections.insert(target.as_str());
        }
    }
    if sections.is_empty() {
        return None;
    }
    let mut keys: Vec<String> = NoticeState::mentions(sdk01, notice_id, parsed)
        .into_iter()
        .filter(|m| sections.contains(m.section_id.as_str()))
        .filter_map(|m| match &m.identifier {
            // The strong key: country-scoped and normalised by the function the
            // resolver itself binds on, so two notices publishing one buyer agree
            // here whatever they wrote in the name field.
            Some(id) => {
                Some(format!("{}:{}:{}", id.country.as_deref().unwrap_or(""), id.kind, id.value))
            }
            // No identifier that passed the plausibility gate: fall back to the N2
            // name key, which is what the org layer falls back to. An empty key
            // carries no identity, so it is dropped rather than colliding every
            // nameless buyer into one.
            None => {
                let norm = match_norm(&m.name);
                (!norm.is_empty())
                    .then(|| format!("n2:{}:{norm}", m.country.as_deref().unwrap_or("")))
            }
        })
        .collect();
    keys.sort();
    keys.dedup();
    (!keys.is_empty()).then(|| keys.join("|"))
}

/// Fold a legacy address-block element name onto a canonical party role.
fn legacy_role(element: &str) -> String {
    match element {
        "ADDRESS_CONTRACTING_BODY"
        | "ADDRESS_CONTRACTING_BODY_ADDITIONAL"
        | "CA_CE_CONCESSIONAIRE_PROFILE" => "buyer".to_owned(),
        "ADDRESS_CONTRACTOR" | "ADDRESS_WINNER" | "WINNER" => "winner".to_owned(),
        "ADDRESS_REVIEW_BODY" | "ADDRESS_REVIEW_INFO" => "review-body".to_owned(),
        other => other.to_owned(),
    }
}

/// The legacy TED profiles (text / ted-export-r208 / ted-export-r209) chain by
/// transitive OJS-number closure; eForms and DÖE key on their own identifiers.
fn is_legacy_profile(profile: &str) -> bool {
    // `internal-ojs` (the 2008 OPOCE export, issue 41) is parsed by the r209
    // legacy machinery and chains by OJS number exactly like the TED forms —
    // it publishes no BT-04 key, only `NO_DOC_OJS` self-numbers and `REF_NOTICE`
    // chain edges. Classifying it legacy is what gives it an `ojs_self` node and
    // marks its plan row so its edges enter the OJS union-find (canonical.rs's
    // `ojs_self.filter(|_| legacy)` gate); without it every 2008 award was an
    // island — 100% unchained (issue 187). Its award sections are the same
    // `LotResult` kind, so `read_legacy_results` reads them like every other
    // r209 profile.
    profile == "text" || profile == "internal-ojs" || profile.starts_with("ted-export")
}

/// The DÖE sdk-0.1 dialect (issue 29): a permanent ~40%-of-German-volume channel
/// with its own `SDK01-*` node vocabulary, projected by the sdk-0.1 party and
/// results paths rather than the eForms `Organization`/`LotResult` ones.
fn is_sdk01_profile(profile: &str) -> bool {
    profile == "eforms:eforms-sdk-0.1"
}

/// The eForms-DE 1.x national dialect (issue 85): eForms structure, path-shaped
/// `DE1-*` field ids. Covers `eforms-de-1.0`, `-1.1` and `-1.2`; the 2.x line is a
/// real SDK fork that emits ordinary `BT-*` ids and is deliberately not matched.
fn is_de1_profile(profile: &str) -> bool {
    profile.starts_with("eforms:eforms-de-1.")
}

/// Fold a DE-1.x chunk onto the eForms vocabulary in place, so every rule below
/// this point sees one vocabulary (see [`DE1_FIELD_ALIASES`]). Done on the chunk
/// the projection already owns, so no notice is cloned; non-DE-1.x notices are
/// skipped on the profile test and cost one string compare each.
fn normalise_de1(chunk: &mut [(store::NoticeRef, Parsed)]) {
    for (_, parsed) in chunk.iter_mut().filter(|(n, _)| is_de1_profile(&n.profile)) {
        for section in &mut parsed.sections {
            if section.kind == DE1_LOT_KIND {
                section.kind = de1_lot_kind(&section.id).to_owned();
            }
        }
        for value in &mut parsed.values {
            if let Some((_, eforms)) = DE1_FIELD_ALIASES.iter().find(|(de1, _)| *de1 == value.field_id) {
                value.field_id = (*eforms).to_owned();
                de1_mark_reference(value);
            }
        }
    }
}

/// Flag an aliased organization-role reference as a reference (issue 98).
///
/// The vendored DE-1.x inventory is empirical (issue 75) and types every
/// identifier `id`, never `id-ref` — a reference and an identifier are
/// indistinguishable by their lexical form, so the generator could not tell them
/// apart. `value::convert` derives `is_ref` from exactly that type
/// (`is_ref: field.kind == "id-ref"`), so every DE-1.x id reaches the projection
/// with `is_ref = false`, and the role arm — which matches only
/// `NoticeValue::Id { is_ref: true, .. }` — never sees one. The whole
/// organization layer of the cohort was therefore empty: no buyer, no review
/// body, no tenderer, and so no award winner (issue 98; measured at 0% of
/// 218,635 notices, the visible ~35% being parties carried forward from merged
/// TED twins, never DE's own).
///
/// The flag is set HERE rather than by fixing only the vendored json because
/// `is_ref` is written at parse time: correcting the metadata alone would
/// require re-parsing all 218,635 notices from the archive, where doing it in the
/// projection's existing in-memory pass keeps this a projection-only fix and a
/// scoped re-fold. The json is corrected too, so future ingests are right at the
/// source — the two are idempotent, since a value that already arrives `is_ref`
/// is simply set `is_ref` again.
///
/// Scoped to the `OPT-300-`/`OPT-301-` families deliberately, NOT to every id:
/// those two prefixes are exactly what [`role_name`] recognises, so this marks
/// the ids that become party roles and nothing else. A blanket flip would also
/// flag identifiers (`DE1-ProcurementProjectLot-ID`, the folder id, document
/// reference ids), and an identifier read as a reference emits a party pointing
/// at whatever section happens to share its value.
fn de1_mark_reference(value: &mut store::ValueRow) {
    if !(value.field_id.starts_with("OPT-300-") || value.field_id.starts_with("OPT-301-")) {
        return;
    }
    if let store::NoticeValue::Id { is_ref, .. } = &mut value.value {
        *is_ref = true;
    }
}

/// Which lots each `LotsGroup` contains, as `(group key, member key)` pairs (issue 237).
///
/// eForms publishes the composition in its OWN section — `GroupComposition`, a sibling of
/// the group hanging off the notice root, NOT a child of `GLO-nnnn`. The group is that
/// section's [`GROUP_ID_FIELD`] reference and each member one of its repeated
/// [`GROUP_MEMBER_FIELD`] references. Verified against the bytes in
/// `ingest/tests/eforms.rs`, because looking for membership under the group section finds
/// nothing and invites the conclusion that it is not published at all.
///
/// When a composition carries no [`GROUP_ID_FIELD`], the group is inferred ONLY if the
/// notice publishes exactly one `LotsGroup` — then there is nothing to be wrong about.
/// Measured on prod: BT-330 is absent from most compositions (13 values against
/// BT-1375's 59 in a sample), and **9,272 of ~9,694 carrier notices publish exactly one
/// group**, so this fallback is what makes the mapping cover the corpus rather than the
/// fixture. Before it, membership landed for 54 tenders out of 5,890 re-folded.
///
/// With SEVERAL groups and no BT-330 the composition is skipped and logged. Pairing
/// `ND-GroupComposition#0` with the first group in document order is the obvious guess and
/// is deliberately not made: nothing documents the two orders as corresponding, and a
/// wrong row silently reassigns which lots a bid covered, which is worse than a missing
/// one. ~406 notices are in that shape; the log names each so the residual is countable.
fn group_members(notice_id: i64, parsed: &Parsed) -> Vec<(String, String)> {
    let composed: Vec<&str> = parsed
        .sections
        .iter()
        .filter(|s| s.kind == GROUP_COMPOSITION_KIND)
        .map(|s| s.id.as_str())
        .collect();
    let refs_in = |section: &str, field: &str| -> Vec<String> {
        parsed
            .values
            .iter()
            .filter(|v| v.section_id == section && v.field_id == field)
            .filter_map(|v| match &v.value {
                NoticeValue::Id { value, is_ref: true, .. } => Some(value.clone()),
                _ => None,
            })
            .collect()
    };
    let groups: Vec<&str> = parsed
        .sections
        .iter()
        .filter(|s| s.kind == "LotsGroup")
        .map(|s| s.id.as_str())
        .collect();

    let mut out = Vec::new();
    for section in composed {
        let named = refs_in(section, GROUP_ID_FIELD).into_iter().next();
        let group = match (named, groups.as_slice()) {
            (Some(id), _) => id,
            // Exactly one group: unambiguous, so infer it.
            (None, [only]) => (*only).to_owned(),
            (None, several) => {
                eprintln!(
                    "[project] notice {notice_id} {section}: no {GROUP_ID_FIELD} and {} LotsGroup \
                     section(s) — membership skipped rather than guessed (issue 237)",
                    several.len()
                );
                continue;
            }
        };
        for member in refs_in(section, GROUP_MEMBER_FIELD) {
            out.push((group.clone(), member));
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Which kind of lot a DE-1.x `ProcurementProjectLot` section is, read back from
/// its own id: eForms numbers lots `LOT-nnnn`, lots groups `GLO-nnnn` and parts
/// `PAR-nnnn`, and the id is the `cbc:ID` the predicate would have tested. An
/// unrecognised prefix is a plain Lot — the overwhelmingly dominant case, and the
/// one that keeps a lot visible rather than dropping it.
fn de1_lot_kind(section_id: &str) -> &'static str {
    match section_id.split('-').next() {
        Some("GLO") => "LotsGroup",
        Some("PAR") => "Part",
        _ => "Lot",
    }
}

/// The Tender's procedure key: BT-04 for eForms / eForms-DE 2.x, or — for the
/// national dialects that carry no BT-04 — their own `ContractFolderID` when that
/// is a genuine uuid. A shared uuid is the strong explicit cross-reference
/// ADR-0003 merges on (a TED eForms procedure and its DÖE twin publish the same
/// BT-04 uuid), so keying a dialect on it upgrades a uuid-bearing island into the
/// merged Tender. Non-uuid folder ids — the sdk-0.1 numeric channel (issue 34),
/// and any eForms-DE 1.x portal-local reference (issue 85) — are notice-local and
/// never key a Tender: a missed link splits, it must never wrongly merge.
fn procedure_key(parsed: &Parsed, sdk01: bool, de1: bool) -> Option<String> {
    if let Some(key) = first_id(parsed, PROCEDURE_KEY_FIELD).filter(|k| !k.trim().is_empty()) {
        return Some(key);
    }
    // The national dialects' folder ids, each gated: only a genuine uuid is a
    // strong-enough cross-reference to key a Tender on (issue 34, and see
    // [`DE1_FOLDER_FIELD`]).
    let folder = if sdk01 {
        Some(SDK01_FOLDER_FIELD)
    } else if de1 {
        Some(DE1_FOLDER_FIELD)
    } else {
        None
    };
    folder.and_then(|field| first_id(parsed, field).filter(|k| is_uuid(k)))
}

/// A genuine uuid (`8-4-4-4-12` hex). Only these sdk-0.1 folder ids are strong
/// enough to merge Tenders across Sources.
fn is_uuid(s: &str) -> bool {
    let s = s.trim();
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

/// How many DISTINCT hex characters a uuid spends on the 30 nibbles it is free to
/// choose. The version nibble (block 3, character 1) and the variant nibble
/// (block 4, character 1) are dropped first: they are structurally fixed, so
/// counting them would credit even an all-zero placeholder with three distinct
/// characters and blunt the measure exactly where it has to be sharp.
///
/// `None` when `s` is not uuid-shaped at all — the caller decides what that means.
// Inert until issue 369 unit 2 wires the refusal into the plan's key election; the
// tests below are the only callers today, and they pin what it must keep doing.
fn uuid_free_nibbles(s: &str) -> Option<std::collections::BTreeSet<u8>> {
    let s = s.trim();
    if !is_uuid(s) {
        return None;
    }
    let b = s.as_bytes();
    Some(
        (0..36)
            // 8/13/18/23 are the dashes; 14 is the version nibble and 19 the variant.
            .filter(|i| !matches!(i, 8 | 13 | 18 | 23 | 14 | 19))
            .map(|i| b[i].to_ascii_lowercase())
            .collect(),
    )
}

/// The longest run of one repeated character among those free nibbles, dashes and
/// the two fixed nibbles skipped.
fn uuid_longest_run(s: &str) -> usize {
    let s = s.trim();
    let b = s.as_bytes();
    let free: Vec<u8> = (0..s.len())
        .filter(|i| !matches!(i, 8 | 13 | 18 | 23 | 14 | 19))
        .map(|i| b[i].to_ascii_lowercase())
        .collect();
    let mut best = 0;
    let mut run = 0;
    let mut prev = None;
    for c in free {
        run = if Some(c) == prev { run + 1 } else { 1 };
        prev = Some(c);
        best = best.max(run);
    }
    best
}

/// Whether a published procedure key was typed by a human rather than generated
/// (issue 369). BT-04 is a spec-guaranteed uuid, and TED does not check that the
/// publisher honoured the spec — so keys like `00000000-0000-4000-8000-000000000000`
/// and `11111111-1111-4111-9111-111111111111` reach us structurally valid and
/// utterly non-unique, and every notice publishing one collapses into a single
/// Tender.
///
/// **This is a pre-filter, not the gate.** It cannot tell a placeholder that
/// happens to be unique from one two buyers both typed: Rostock's procurement
/// office keys real, distinct procurements
/// `11111111-2222-4aaa-8333-444444444444` … `-444444444450`, one per procurement,
/// while `11111111-2222-4000-8111-123412341235` — one character from a correct
/// key on the same portal — welds eleven buyers into one record. Measured on prod
/// 2026-09-08: of the 14 keys this refuses, 3 are welds and 11 are correct. The
/// caller must combine it with buyer disagreement across the key's notices; see
/// the issue.
///
/// Two disjuncts, both calibrated against 796 real keys sampled off prod, where
/// together they flagged exactly the two genuine placeholders and nothing else:
///
/// - **≤6 distinct free nibbles.** A v4 uuid draws 30 nibbles at random, so the
///   chance of landing on six characters or fewer is about 1.4e-9 — under ten
///   million keys, a hundredth of one expected false positive. This is safe by
///   arithmetic, not by tuning.
/// - **a run of ≥8 identical nibbles**, which catches the hand-typed keys that
///   spend many distinct characters on a decorative prefix
///   (`abcdefab-1111-4111-9111-111111111111` uses seven). Its false-positive rate
///   is far higher — order one key corpus-wide — and that is affordable only
///   because the buyer test stands behind it: a real key has one buyer and is
///   admitted regardless.
fn is_placeholder_key(s: &str) -> bool {
    let Some(free) = uuid_free_nibbles(s) else {
        return false;
    };
    free.len() <= 6 || uuid_longest_run(s) >= 8
}

/// Parse an OJS publication reference into `(year, number)`. Handles the OJS
/// display form (`2019/S 001-000001`, `2011/S 1-000181`), the DOC/eForms form
/// (`000001-2019`), and the text-era form (`154-2005`). The raw string is never
/// the key — its shape is not stable across eras (research §1).
fn ojs_key(raw: &str) -> Option<OjsKey> {
    let s = raw.trim();
    let (year, number) = if let Some((head, tail)) = split_ci(s, "/S") {
        // Display form: `<year>/S <issue>-<number>`; the number is the tail
        // after the last '-'.
        (head.trim(), tail.rsplit('-').next()?.trim())
    } else {
        // DOC / text-era form: `<number>-<year>`.
        let (number, year) = s.rsplit_once('-')?;
        (year.trim(), number.trim())
    };
    let year: i64 = year.parse().ok()?;
    let number: i64 = number.parse().ok()?;
    ((1900..=2100).contains(&year) && number > 0).then_some((year, number))
}

/// `split_once`, case-insensitive on the delimiter — the OJS separator is
/// written `/S` but a stray lowercase `s` should not defeat the parse.
fn split_ci<'a>(s: &'a str, delim: &str) -> Option<(&'a str, &'a str)> {
    let lower = s.to_ascii_uppercase();
    let at = lower.find(&delim.to_ascii_uppercase())?;
    Some((&s[..at], &s[at + delim.len()..]))
}


/// Normalise an official identifier and gate it on plausibility before letting
/// it merge two mentions into one Organization
/// (docs/research/ted-legacy-mapping.md §6: 16% of real ids are junk).
///
/// National company-register prefixes (issue 86, data-profile §3 rule 1). A
/// register number like the German `HRB 22388` begins with an alphabetic scheme
/// tag whose first two letters coincide with an ISO country code — `HR` reads as
/// Croatia — so the naive VAT sniffer minted ~50,700 German companies as
/// Croatian. These are classified as national register ids BEFORE VAT sniffing,
/// so their prefix never mints a country. Longest-first is unnecessary (each is
/// matched with a following digit), but the list is the documented family:
/// German (HRB/HRA/VR/GNR/PR), Austrian (FN), Polish (KRS/NIP/REGON), Romanian
/// (CUI), Spanish (CIF/NIF), Croatian (OIB), Danish (CVR), Czech (ICO/DIC),
/// French (SIREN/SIRET), and the German VAT-word tag (UST).
///
/// Since issue 359 the label strip above runs FIRST for `NIP`, `KRS`, `REGON`,
/// `CIF` and `NIF` (as it already did for `UST…`): those tags name the scheme
/// the bare value classifies as by shape, so `NIP1070000916` becomes the same
/// `1070000916` its twin row already carries. This arm still catches them when
/// the strip is refused — a compound `NIP…REGON…` field, a bare field name.
const REGISTER_PREFIXES: &[&str] = &[
    "HRB", "HRA", "GNR", "VR", "PR", "FN", "KRS", "NIP", "REGON", "CUI", "CIF",
    "NIF", "OIB", "CVR", "ICO", "DIC", "SIRET", "SIREN", "UST",
];

/// The country prefixes a VAT id may legitimately carry — the EU-27 (with `EL`
/// for Greece), the EEA and the near-European VAT jurisdictions, plus the two
/// non-ISO forms TED emits (`UK`, and `XI` for Northern Ireland). A two-letter
/// prefix outside this set must never mint a country (data-profile §3 rule 1):
/// it is a national id whose country comes from the mention, not the string.
const VAT_COUNTRIES: &[&str] = &[
    "AT", "BE", "BG", "CY", "CZ", "DE", "DK", "EE", "EL", "ES", "FI", "FR", "GR",
    "HR", "HU", "IE", "IT", "LT", "LU", "LV", "MT", "NL", "PL", "PT", "RO", "SE",
    "SI", "SK", "GB", "UK", "XI", "CH", "IS", "LI", "NO",
];

/// Canonicalise a country code to ISO-3166 alpha-2 (issue 48, completed by
/// issue 319). Alpha-3 folds to its alpha-2 from the GENERATED ISO 3166-1
/// table in [`crate::countries`]; a country NAME folds too; TED's non-ISO
/// `UK` becomes `GB`, eurostat's `EL` (Greece) becomes `GR`, and the
/// user-assigned `XKX` (Kosovo, which ISO does not assign) becomes `XK`. A
/// value already alpha-2, or one nothing recognises, passes through
/// unchanged — an unmapped code is preserved rather than dropped.
///
/// Issue 319 is why the table is generated. The hand-picked version covered
/// the EU-27 plus "common third countries" and left 151 alpha-3 codes
/// (1,344 org rows) to pass through, so `GRL` sat beside `GL` and the merge
/// arm — which keys on country — could never close the pair.
pub fn canonical_country(raw: &str) -> String {
    // UNICODE uppercase, not ASCII (panel catch): eight ISO names carry
    // accents — CURAÇAO, CÔTE D'IVOIRE, TÜRKIYE, RÉUNION, ÅLAND ISLANDS —
    // and `to_ascii_uppercase` leaves their lowercase forms half-cased, so
    // they would miss the table AND be written back mangled, since this
    // function returns the cased value when nothing matches.
    let up = raw.trim().to_uppercase();
    match up.as_str() {
        "UK" => return "GB".into(),   // TED writes UK, ISO is GB
        "EL" => return "GR".into(),   // eurostat/NUTS Greece is EL, ISO is GR
        "XKX" => return "XK".into(),  // Kosovo: user-assigned, not in ISO 3166-1
        _ => {}
    }
    let table: &[(&str, &str)] = match up.len() {
        3 => crate::countries::ALPHA3_TO_ALPHA2,
        n if n > 3 => crate::countries::NAME_TO_ALPHA2,
        _ => return up,
    };
    match table.iter().find(|(from, _)| *from == up) {
        Some((_, to)) => (*to).to_owned(),
        None => up,
    }
}

/// Greek and Cyrillic capitals (and the Greek lowercase forms the uppercase
/// step would reach anyway) that render like Latin capitals, folded to the
/// Latin letter. Only letters that are visually identical are mapped — this
/// is a lookalike fold for identifier text, not a transliteration.
fn fold_confusable(c: char) -> char {
    match c {
        'Α' | 'α' => 'A',
        'Β' | 'β' => 'B',
        'Ε' | 'ε' => 'E',
        'Ζ' | 'ζ' => 'Z',
        'Η' | 'η' => 'H',
        'Ι' | 'ι' => 'I',
        'Κ' | 'κ' => 'K',
        'Μ' | 'μ' => 'M',
        'Ν' | 'ν' => 'N',
        'Ο' | 'ο' => 'O',
        'Ρ' | 'ρ' => 'P',
        'Τ' | 'τ' => 'T',
        'Υ' | 'υ' => 'Y',
        'Χ' | 'χ' => 'X',
        'А' | 'а' => 'A',
        'В' | 'в' => 'B',
        'Е' | 'е' => 'E',
        'К' | 'к' => 'K',
        'М' | 'м' => 'M',
        'Н' | 'н' => 'H',
        'О' | 'о' => 'O',
        'Р' | 'р' => 'P',
        'С' | 'с' => 'C',
        'Т' | 'т' => 'T',
        'Х' | 'х' => 'X',
        'У' | 'у' => 'Y',
        other => other,
    }
}

/// `16054368_3` → `16054368`: an all-digit head, an underscore, a one- or
/// two-digit sub-unit index. Anything else is returned unchanged.
fn strip_ro_subunit(raw: &str) -> &str {
    if let Some((head, tail)) = raw.rsplit_once('_')
        && !head.is_empty()
        && head.bytes().all(|b| b.is_ascii_digit())
        && (1..=2).contains(&tail.len())
        && tail.bytes().all(|b| b.is_ascii_digit())
    {
        return head;
    }
    raw
}

/// A VAT id carries its country in its own prefix and is scoped by it; a
/// national registry number is only unique inside its country, so it is scoped
/// by the mention's country and stays separate when that is unknown.
pub fn normalise_identifier(raw: &str, country: Option<&str>) -> Option<Identifier> {
    normalise_identifier_with(raw, country, true)
}

/// The normaliser as it stood BEFORE the v2.1 lookalike/suffix folds. Not a
/// live path: issue 345's repair re-parses each standing row's published
/// string through both — a row moves only when this one reproduces what is
/// stored (so the sampled mention is the one that minted the row, not one an
/// R2/R3 merge brought in) and [`normalise_identifier`] now says otherwise.
pub fn normalise_identifier_before_folds(raw: &str, country: Option<&str>) -> Option<Identifier> {
    normalise_identifier_with(raw, country, false)
}

fn normalise_identifier_with(raw: &str, country: Option<&str>, folds: bool) -> Option<Identifier> {
    // Issue 358: the identifier's country is the REGISTER's jurisdiction. A
    // SIREN published under `RE` is French, and the org row it mints or
    // binds has to say so, or Réunion's SDIS stands as two rows (one per
    // code the buyer chose that day). Only the identifier's scope maps; the
    // mention keeps the regional code the notice published. Mapped before
    // the country-specific folds and the gate below, so a Y-tunnus under
    // `AX` is checked as the Finnish number it is.
    let country = country.map(store::register_jurisdiction);
    // Issue 300 (top-100 read, 2026-09-03): a Romanian CUI with a sub-unit
    // suffix — `16054368_3` for a regional directorate of CNAIR — is the
    // parent's identifier; folding the suffix away is the entity-level
    // identity the model already promises. RO only: the underscore-digit
    // shape means nothing established elsewhere.
    let raw = match country {
        Some("RO") | Some("ROU") if folds => strip_ro_subunit(raw),
        _ => raw,
    };
    // Same read: the Greek procurement authority sat in two rows because its
    // id was published once with a Latin `E` and once with a Greek `Ε`
    // (U+0395), and the ASCII filter below kept one letter and dropped the
    // other. Homoglyph capitals fold to Latin BEFORE the filter, so a register
    // id spelled with a lookalike letter equals its ASCII twin.
    //
    // ONLY when the fold leaves nothing non-Latin behind. The first dry plan of
    // issue 345's repair showed why: a Cyrillic label in front of a Bulgarian
    // id — `ЕИК 121663601` — has two lookalike letters and one that is not, and
    // folding the two glued a bogus `EK` onto an id the filter used to leave
    // bare. A string that is still non-Latin after folding was written in that
    // script on purpose; its lookalikes are its own letters, not ours.
    let folded: Option<String> = folds.then(|| raw.chars().map(fold_confusable).collect::<String>());
    let raw = match &folded {
        Some(f) if !f.chars().any(|c| c.is_alphabetic() && !c.is_ascii()) => f.as_str(),
        _ => raw,
    };
    let value: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();

    if value.len() < 4 || !value.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    // All-zero and single-character fillers ("0000", "999999", "n/a" variants
    // that survived the digit test).
    if value.chars().filter(|c| c.is_ascii_digit()).all(|c| c == '0') {
        return None;
    }
    if value.chars().skip(1).all(|c| c == value.as_bytes()[0] as char) {
        return None;
    }

    // ISSUE 328: a publisher label written in FRONT of the identifier.
    // `USTIDDE329214156` is `USt-IdNr. DE329214156` — the number is fine, the
    // field name came along with it. 5,766 rows carry one, and 3,253 of them
    // have a partner row already standing under the bare value, so the label
    // splits an organization from its own correctly-formed twin.
    //
    // STRIP THEN RE-VALIDATE, and the recursion is the re-validation: the
    // stripped remainder goes through this whole function, and it is accepted
    // only if it classifies. That is what stops the class from inventing
    // identifiers — three prod rows carry `UMSATZSTEUERIDENTIFIKATIONSNUMMER`
    // and nothing else, `USTIDNRDEDE…` strips to a doubled country code, and
    // neither survives a second pass. A prefix strip that trusted its own
    // output would keep both.
    //
    // Issue 325's suffix work needed no such guard because a scheme label at
    // the BACK sits behind a value that already parsed; a label at the front
    // hides the value entirely until it is gone.
    if let Some(rest) = crate::countries::label_prefix_stripped(&value) {
        // THE GUARD HAS TO BE SHARPER THAN "IT PARSES", and the first version
        // was not. `normalise_identifier` almost never returns `None` for a
        // string containing a digit, because `national()` is a catch-all — so
        // "strip then re-validate" validated nothing, and the issue-328 dry plan
        // showed it: `UMSATZSTEUERIDENTIFIKATIONSNRENTEGAPLUSGMBHDE813810149`
        // became the identifier `ENTIFIKATIONSNRENTEGAPLUSGMBHDE813810149`.
        // The vocabulary has `UMSATZSTEUERIDENTIFIKATIONSNUMMER` and
        // `UMSATZSTEUERID` but not `…SNR`, so a SHORTER entry matched and left a
        // plausible-looking fragment. `HANDELSREGISTERNRHRB64128` → `NRHRB64128`
        // and `HANDELSREGISTERARNHEM09155985` → `ARNHEM09155985` are the same
        // shape.
        //
        // So the remainder must be RECOGNISABLE, not merely parseable: either it
        // classifies as a real scheme (a VAT id, a register form) or it is pure
        // digits — a bare registration number that lost its label, which is the
        // `STNR…`/`STEUERNUMMER…` class and a correct strip. Anything else is
        // leftover label text and the row keeps what the publisher wrote.
        //
        // This makes the vocabulary's gaps SAFE rather than harmful: an
        // unlisted label variant now leaves the row alone instead of mangling
        // it, which is the property that matters when the list is read off a
        // corpus that keeps growing.
        if let Some(id) = normalise_identifier_with(rest, country, folds) {
            // Issue 359 widened "recognisable" by one shape: a Spanish CIF/NIF
            // (one letter, seven digits, a check character; or eight digits
            // and a letter; or an NIE's X/Y/Z lead). `CIFA48283964` — the
            // label IS the scheme's name — strips to a value the national arm
            // classifies by shape, and a leftover label fragment never has that
            // shape (`NRHRB64128`, `ARNHEM09155985` both fail it).
            // Issue 374 widens "recognisable" by one more shape, the same way
            // 359 did for the CIF: a German register division followed by
            // digits. `HANDELSREGISTERHRB93017` strips to `HRB93017`, which is
            // neither pure digits nor a CIF, so the guard used to throw the
            // strip away and leave the labelled form standing as its own merge
            // key — the same company published bare as `HRB93017` got a second
            // org row. The ANCHORING is what keeps this safe: the two leftover
            // fragments the guard exists for, `NRHRB64128` and
            // `ARNHEM09155985`, do not START with a division and still fail.
            let recognisable = id.kind != "national"
                || rest.bytes().all(|b| b.is_ascii_digit())
                || es_cif_or_nif_shaped(rest)
                || de_register_division_shaped(rest);
            if recognisable {
                return Some(id);
            }
        }
    }

    let national = || Identifier {
        country: country.map(str::to_owned),
        kind: "national".into(),
        value: value.clone(),
    };

    // Issue 300 Stage 1 — the v2 gate: an identifier in a measured
    // false-merge class must never become a merge key. The predicate is
    // `idgate::condemns` (placeholder lexicon, suspicious digit runs, short
    // VAT stubs, HARD-scheme checksum failures per the standing enablement
    // decision), applied to the same (country, kind, value) shape the
    // census measured, so prevention and the census read one ruler. The
    // raw published value survives untouched on the mention
    // (`raw_identifier`); only merge-key status is refused — the mention
    // takes the provisional path, exactly like a value the v1 gate already
    // rejected. Rejection cannot create prevention-vs-stock splits; value
    // RESHAPING could, which is why none happens here (compounds and
    // labelled prefixes are Stage-2 match-time work).
    let condemned = |id: &Identifier| {
        crate::idgate::condemns(id.country.as_deref(), &id.kind, &id.value)
    };
    let gated = |id: Identifier| if condemned(&id) { None } else { Some(id) };

    // A known register scheme is national — its prefix is a scheme tag, not a
    // country, so it must not reach the VAT sniffer (issue 86). `HRB22388` and
    // `HRBDRESDEN4115` (court name inline) both start with `HRB`, not the country
    // `HR`. A 3+-letter tag (HRB, KRS, REGON…) is specific enough to match on the
    // prefix alone; a 2-letter tag (FN, VR, PR) also requires a following digit,
    // so it cannot swallow an unrelated word that merely begins with its letters.
    let starts_register = REGISTER_PREFIXES.iter().any(|p| match value.strip_prefix(p) {
        Some(rest) => p.len() >= 3 || rest.starts_with(|c: char| c.is_ascii_digit()),
        None => false,
    });
    if starts_register {
        return gated(national());
    }

    // Otherwise a leading two-letter VAT country (Austrian `ATU…` keeps its
    // `U`) followed by the registration number ITSELF is a VAT id scoped by
    // that country; a two-letter prefix outside the VAT set never mints a
    // country.
    //
    // ISSUE 325: the old test was "some digit ANYWHERE in the rest", which let
    // any word whose first two letters spell a VAT country mint one.
    // `CHARITYNO298028` became Swiss on a British charity, `BERICHTSEINHEITID…`
    // Belgian on 595 German public bodies, `FINANZAMT…` Finnish, and every
    // 32-char hex GUID starting `EE`/`DE`/`BE` became a VAT id of that country.
    // Measured: 4,206 rows, and 3,789 of the 4,059 with mentions (93.4%) carry
    // mentions that UNANIMOUSLY name a different country — the publisher's own
    // field, contradicting the prefix guess 34,111 times against 946.
    //
    // The discipline is the one the register-prefix arm above already applies,
    // one arm up: a two-letter tag must be followed by the thing it tags, not
    // by the middle of a word. A real VAT body is short and mostly digits — no
    // scheme puts three letters straight after the country code (Austria has
    // one `U`, a Spanish CIF one letter, France two check characters, GB's
    // `GD`/`HA` two) and none is longer than Sweden's twelve characters. So
    // both bounds apply, and the letter run is the sharper of the two: it is
    // what separates `ESX1234567X` from `ESTRADADOBAIRRO…`.
    let vat_prefix: String = value.chars().take(2).collect();
    let body = &value[2..];
    let is_vat = VAT_COUNTRIES.contains(&vat_prefix.as_str())
        && body.chars().any(|c| c.is_ascii_digit())
        && body.len() <= VAT_BODY_MAX
        && longest_letter_run(scheme_suffix_stripped(body)) < 3;
    if is_vat {
        // Canonicalise the minted code. The arm used to store the prefix RAW,
        // which is a live re-contamination path for issue 319's completed
        // fold: it re-mints `EL`, `UK` and `XI` — 262 org rows on prod carry
        // them today and every one is `kind = 'vat'`, i.e. minted here after
        // the fold ran. The published VALUE keeps its own prefix (`EL094…`
        // really is spelled that way); only the country column joins the
        // alpha-2 vocabulary every other writer uses.
        let country = canonical_country(&vat_prefix);
        gated(Identifier { country: Some(country), kind: "vat".into(), value })
    } else {
        gated(national())
    }
}

/// A Spanish CIF (`A48283964`: letter, seven digits, check digit or letter),
/// NIF/DNI (`12345678Z`) or NIE (`X1234567L`) shape — the one national form a
/// label strip may leave behind that is neither digits nor a VAT id (issue
/// 359). Shape only; the letter algebra is `idgate`'s business.
/// Schemes that are never a register, so nothing published under them may
/// become a merge key (issue 365 unit 4).
///
/// **CURRENTLY EMPTY, and that is the finding rather than a gap.** `OTROS` was
/// added here on 2026-09-09 and REVERTED the same day, because the measurement
/// that justified it was scoped wrongly. What it measured was "canonical orgs
/// that carry an `OTROS` mention" — 36.9 % of them held ≥2 distinct mention
/// names against a 14.7 % baseline, worst row 231. But an org reached by an
/// `OTROS` mention is usually reached by many others too, so that statistic
/// attributed a large buyer's whole name spread to whichever scheme happened to
/// appear among its mentions. It is guilt by association.
///
/// Scoped to the thing actually under suspicion — one `OTROS` VALUE, and the
/// distinct names published against it — the class declines flatly:
///
/// | distinct `OTROS` values (`notice_id > 25000000`) | 887 |
/// | spanning ≥2 distinct names | **16 (1.8 %)** |
/// | worst value | **11** names |
///
/// 1.8 % is the same territory as `SPRAWA` (1.8 %) and `ID_UTE_TEMP_PLATAFORMA`
/// (1.2 %), both of which were measured and DECLINED. And reading the 16: fifteen
/// are real VAT or CIF numbers carrying two or three NAME VARIANTS of one company
/// (`A95758389`, `NL862416000B01`, `IT03412740171`…) — a key doing its job, not
/// fusing. The sixteenth is the literal word `UTE` (Spanish for a temporary
/// business consortium) with 11 names, and `UTE` is already refused by the shape
/// filters: no digit, three characters. So the denial protected NOTHING and cost
/// the linking value of ~871 working keys — the issue-312 calculus exactly.
///
/// The mechanism stays: the predicate, the case-insensitive match, and unit 6's
/// cohort re-fold are all sound and tested, and a future denial needs them. What
/// changed is that no scheme currently earns one.
///
/// Two candidates never made it in for the same reason: `ID_PLATAFORMA` (41,579
/// mentions) and the raw `eu`/`EU` scheme (252k) are UNMEASURED — the read timed
/// out three times — and after the `OTROS` episode the standard for adding one is
/// a per-VALUE fusion measurement, not a per-org one.
pub const DENIED_SCHEMES: &[&str] = &[];

fn scheme_never_keys(scheme: Option<&str>) -> bool {
    scheme.is_some_and(|s| DENIED_SCHEMES.iter().any(|d| s.eq_ignore_ascii_case(d)))
}

/// A German commercial-register value: the register division then digits and
/// nothing else — `HRB93017`, `HRA2132`, `VR326`, `PR258`, `GNR22`.
///
/// The five divisions are the ones the corpus actually carries in bare form
/// (HRB 11,439 rows, HRA 2,132, VR 326, PR 258, GnR 22 — prod 2026-09-09), so
/// a labelled row stripping to one of them rejoins real company rows rather
/// than a shape someone guessed at. Anchored and digits-only on purpose: a
/// court suffix (`HRB93017B`) will not strip, which is the conservative
/// direction — the row keeps what the publisher wrote.
fn de_register_division_shaped(rest: &str) -> bool {
    ["HRB", "HRA", "GNR", "VR", "PR"].iter().any(|div| {
        rest.strip_prefix(div)
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    })
}

fn es_cif_or_nif_shaped(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 9 {
        return false;
    }
    let digits = |r: &[u8]| r.iter().all(u8::is_ascii_digit);
    let letter = |c: u8| c.is_ascii_uppercase();
    (letter(b[0]) && digits(&b[1..8]) && (b[8].is_ascii_digit() || letter(b[8])))
        || (digits(&b[..8]) && letter(b[8]))
}

/// `body` without a trailing scheme label, if it carries one.
///
/// **Several countries publish the local word for "VAT" inside the id.** Norway
/// writes `NO 999 665 624 MVA` (*merverdiavgift*), Switzerland
/// `CHE-106.094.419 MWST` (*Mehrwertsteuer*), Germany appends `USt-IdNr`, and
/// French, Italian and English forms appear too. Measuring the letter run over
/// the whole body rejects all of them.
///
/// **This is a VOCABULARY and not a length bound, and that was the lesson.**
/// The first attempt allowed "a trailing run of up to N letters" and the N
/// ratcheted on every corpus read — 3 for `MVA`, then 4 for `MWST`, then 5 for
/// `USTID` — until the actual distribution was pulled:
///
/// ```text
///   MVA 113   MWST 46   USTID 27   TVA 11   VAT 8   IVA 4   VATID 1   AVAT 1
///   BBERLIN 1  AGJENA 2  AGULM 1  ESSEN 1  BONN 1  BURG 1  KAMP 1  AGSL 1
/// ```
///
/// `ESSEN` and `USTID` are both five letters; `BONN` and `MWST` are both four.
/// The first list is scheme labels and the second is German towns and court
/// tags appended to a register number — so **no length bound can separate
/// them**, and ratcheting one was going to keep finding a longer counterexample
/// forever. The set below is read off the corpus rather than guessed; changing
/// it means re-running that query, not reasoning about which languages exist.
///
/// `AVAT` needs no entry: it is an Irish check letter followed by `VAT`, and
/// stripping `VAT` leaves `…A`, a one-letter run. `WWST` (a misspelling of
/// `MWST`, one row) is deliberately absent — a typo is not a scheme.
fn scheme_suffix_stripped(body: &str) -> &str {
    for suffix in VAT_SUFFIXES {
        if body.len() > suffix.len() && body.ends_with(suffix) {
            return &body[..body.len() - suffix.len()];
        }
    }
    body
}

/// The scheme labels published inside a VAT id, measured from prod's whole
/// `kind = 'vat'` population (issue 325). Longest first, so a greedy match
/// takes `VATID` before it could take `VAT`.
const VAT_SUFFIXES: &[&str] = &["USTID", "VATID", "MWST", "MVA", "TVA", "VAT", "IVA"];

/// The longest run of consecutive ASCII letters in `s`.
///
/// Distinct from [`crate::idgate::letter_run_after_prefix`], which first skips
/// a leading letter run of up to six so a register TAG (`HRB`, `REGON`) does
/// not count against its own value. Here the two-letter country prefix is
/// already removed by the caller and everything left is the body, so a plain
/// run is what the question wants.
fn longest_letter_run(s: &str) -> usize {
    let mut best = 0usize;
    let mut run = 0usize;
    for b in s.bytes() {
        if b.is_ascii_alphabetic() {
            run += 1;
            best = best.max(run);
        } else {
            run = 0;
        }
    }
    best
}

/// The longest real VAT body, plus headroom: Sweden's is twelve characters
/// after the country code and nothing published is longer. Fourteen rather
/// than twelve because the length is the coarse bound — `longest_letter_run`
/// is what actually separates an identifier from prose, and a 16-character
/// mostly-digit value that is NOT a VAT number simply becomes national, which
/// is where it belonged. A 32-character hex GUID has a 30-character body and
/// fails here whatever its letter runs look like.
const VAT_BODY_MAX: usize = 14;

fn first_id(parsed: &Parsed, field_id: &str) -> Option<String> {
    parsed.values.iter().find(|v| v.field_id == field_id).and_then(|v| match &v.value {
        NoticeValue::Id { value, .. } => Some(value.clone()),
        _ => None,
    })
}

fn first_code(parsed: &Parsed, field_id: &str) -> Option<String> {
    parsed.values.iter().find(|v| v.field_id == field_id).and_then(|v| match &v.value {
        NoticeValue::Code { code, .. } => Some(code.clone()),
        _ => None,
    })
}

/// The notice's original language in the fold's 639-2/T vocabulary — the first
/// of [`ORIGINAL_LANG_FIELDS`] the parse carries, through the same
/// [`normalize_lang`] every text tag goes through, so `EN`, `DEU` and `FR` all
/// land as the tag `tender_version_texts.lang` uses and the read-time rank can
/// compare them by equality.
pub fn original_lang(parsed: &Parsed) -> Option<String> {
    ORIGINAL_LANG_FIELDS
        .iter()
        .find_map(|field| first_code(parsed, field))
        .and_then(|code| normalize_lang(Some(&code)))
}

/// The publication and dispatch instants of a parsed notice, resolved per era
/// (issue 18). `published_at` is the real publication date where the notice
/// records one (the OJEU stamp, the legacy OJ date, or DÖE's requested/portal
/// date), falling back to dispatch; `dispatched_at` is the send date, or `None`
/// when the notice carries none (e.g. a DÖE numeric island with only a
/// requested-publication date). Shared by the processor (which stamps the
/// notice row) and the projection (which stamps the version), so both agree —
/// and since issue 367 both vocabularies are named in the two field lists, so
/// "agree" holds for the DE-1.x dialect too, whose ids the notice layer keeps.
///
/// **Both axes are `Option`, and a payload that states no date at all yields
/// `None`, never the epoch** (issue 367). The old `.unwrap_or(0)` stored
/// 1970-01-01 for every notice the resolver missed, which read as a real
/// publication date to every consumer: 218,876 DE-1.x rows carried it, and
/// `/v1/notices/…` served `"published_at":"1970-01-01T00:00:00Z"`. The notice
/// column is nullable and now says so. `tender_versions.published_at` is NOT
/// NULL, so the two projection call sites keep an explicit epoch fallback for
/// the (vanishing) dateless notice — the fold has to order versions somehow.
pub fn notice_instants(parsed: &Parsed) -> (Option<i64>, Option<i64>) {
    let dispatched_at = DISPATCH_DATE_FIELDS.iter().find_map(|f| first_date(parsed, f));
    let published_at =
        PUBLICATION_DATE_FIELDS.iter().find_map(|f| first_date(parsed, f)).or(dispatched_at);
    (published_at, dispatched_at)
}

fn first_date(parsed: &Parsed, field_id: &str) -> Option<i64> {
    parsed.values.iter().find(|v| v.field_id == field_id).and_then(|v| match &v.value {
        NoticeValue::Date { utc_seconds, .. } => Some(*utc_seconds),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue 372 unit 2, against real published XML rather than the schema: a
    /// `FieldsPrivacy` block's PARENT is the section holding the value it
    /// suppresses, and the id the declaration names is that value's [`stem`].
    ///
    /// That pairing is the whole mechanism -- it is what lets the marker be a set
    /// lookup with no new vocabulary -- so it is asserted against the committed
    /// withheld fixture instead of trusted. All five of its blocks anchor this
    /// way across three value channels (a number, a code and a text), which is
    /// also why the same lookup will serve unit 4's other satellites.
    #[test]
    fn withheld_declarations_pair_with_the_section_holding_the_suppressed_value() {
        let relative = "eforms/can-withheld-29-00495618-2026.xml";
        let bytes = std::fs::read(format!("tests/fixtures/{relative}")).expect("fixture");
        let crate::profile::Disposition::Records(records) =
            crate::profile::dispatch(relative, &bytes)
        else {
            panic!("dispatch skipped the withheld fixture");
        };
        let [crate::profile::Record::Notice(notice)] = &records[..] else {
            panic!("expected one notice record");
        };
        let store::Parse::Parsed(parsed) = crate::eforms::parse_payload(&notice.profile, &bytes)
        else {
            panic!("the withheld fixture must parse");
        };

        let withheld = withheld_source_fields(&parsed);
        assert_eq!(withheld.len(), 5, "the fixture declares five withheld fields: {withheld:?}");

        // Every declaration names a field that IS published in the paired section,
        // and it is reachable by the same `stem` a fact carries. If the SDK
        // anchored the block anywhere else, this loop is where it shows up.
        for (section, source) in &withheld {
            let found = parsed
                .values
                .iter()
                .find(|v| v.section_id == *section && stem(&v.field_id) == *source);
            assert!(
                found.is_some(),
                "declaration {source} names no value in its parent section {section}",
            );
        }

        // The three channels the marker lands on, which is unit 4's map: BT-759 is
        // a Number(-1), BT-760 a Code('unpublished'), BT-734 a Text('unpublished').
        let sources: BTreeSet<&str> = withheld.iter().map(|(_, f)| *f).collect();
        for expected in ["BT-759", "BT-760", "BT-539", "BT-541", "BT-734"] {
            assert!(sources.contains(expected), "{expected} not declared: {sources:?}");
        }
    }

    /// The per-row precision issue 372 unit 2 was decided on: a declaration marks
    /// the field it NAMES and leaves its siblings alone. The fixture's award
    /// criterion is the real case — `BT-541`'s weight is withheld while
    /// `BT-5421`'s weight TYPE beside it in the same section is published.
    #[test]
    fn a_declaration_marks_only_the_field_it_names() {
        let parsed = Parsed {
            sections: vec![
                store::Section { id: "ND-Crit#0".into(), kind: "LotAwardCriterion".into(), parent: None },
                store::Section {
                    id: "ND-CritWeightUnpublish#0".into(),
                    kind: "FieldsPrivacy".into(),
                    parent: Some("ND-Crit#0".into()),
                },
            ],
            values: vec![
                store::ValueRow {
                    section_id: "ND-CritWeightUnpublish#0".into(),
                    field_id: "BT-195(BT-541)-Lot-Weight".into(),
                    ordinal: 0,
                    value: NoticeValue::Code {
                        list: Some("non-publication-identifier".into()),
                        code: "awa-cri-num".into(),
                    },
                },
                store::ValueRow {
                    section_id: "ND-Crit#0".into(),
                    field_id: "BT-541-Lot-WeightNumber".into(),
                    ordinal: 0,
                    value: NoticeValue::Number { value: -1.0, unit: None },
                },
                store::ValueRow {
                    section_id: "ND-Crit#0".into(),
                    field_id: "BT-5421-Lot".into(),
                    ordinal: 0,
                    value: NoticeValue::Code { list: Some("number-weight".into()), code: "per-exa".into() },
                },
            ],
        };

        let withheld = withheld_source_fields(&parsed);
        assert!(withheld.contains(&("ND-Crit#0", "BT-541")), "the named field: {withheld:?}");
        assert!(
            !withheld.contains(&("ND-Crit#0", "BT-5421")),
            "a sibling in the same section must keep its value: {withheld:?}",
        );
        assert_eq!(withheld.len(), 1);
    }

    /// The claim in [`withheld_source_fields`]'s doc that the exact-section rule
    /// misses in the SAFE direction. A block a publisher hoisted away from the
    /// value it suppresses (issue 195 saw `FieldsPrivacy` under the root
    /// extension) marks NOTHING — the amount stays an unmarked negative, which is
    /// what issue 366's sentinel rule already refuses. Asserted so that if the
    /// rule is ever widened to notice-wide, this test is the thing that says so.
    #[test]
    fn a_hoisted_privacy_block_marks_nothing_rather_than_guessing() {
        let parsed = Parsed {
            sections: vec![
                store::Section { id: "ND-Root".into(), kind: "Root".into(), parent: None },
                store::Section { id: "ND-Result#0".into(), kind: "NoticeResult".into(), parent: Some("ND-Root".into()) },
                store::Section {
                    id: "ND-Hoisted#0".into(),
                    kind: "FieldsPrivacy".into(),
                    parent: Some("ND-Root".into()),
                },
            ],
            values: vec![
                store::ValueRow {
                    section_id: "ND-Hoisted#0".into(),
                    field_id: "BT-195(BT-161)-NoticeResult".into(),
                    ordinal: 0,
                    value: NoticeValue::Code {
                        list: Some("non-publication-identifier".into()),
                        code: "not-val".into(),
                    },
                },
                store::ValueRow {
                    section_id: "ND-Result#0".into(),
                    field_id: "BT-161-NoticeResult".into(),
                    ordinal: 0,
                    value: NoticeValue::Amount { cents: -100, currency: "EUR".into() },
                },
            ],
        };

        let withheld = withheld_source_fields(&parsed);
        assert!(
            !withheld.contains(&("ND-Result#0", "BT-161")),
            "a hoisted block must not reach the value's section: {withheld:?}",
        );
        // It is not discarded either: the pair exists against the root, so a later
        // widening has something to work from and section 11 can still count it.
        assert!(withheld.contains(&("ND-Root", "BT-161")), "{withheld:?}");
    }

    /// Issue 372 unit 5, as a rule rather than a census note: a `-1` with no
    /// declaration is NOT withheld. 116 of the corpus's 19,236 `-1.00` amount rows
    /// are publisher-invented sentinels — notice 24158422 publishes `BT-27` = −1
    /// with zero `FieldsPrivacy` blocks and its own lots at 0 — and marking those
    /// `withheld` would assert something no notice ever said, which is the exact
    /// error this issue is about, one layer along.
    #[test]
    fn an_undeclared_negative_amount_is_not_withheld() {
        let parsed = Parsed {
            sections: vec![store::Section {
                id: "ND-Procedure".into(),
                kind: "Procedure".into(),
                parent: None,
            }],
            values: vec![store::ValueRow {
                section_id: "ND-Procedure".into(),
                field_id: "BT-27-Procedure".into(),
                ordinal: 0,
                value: NoticeValue::Amount { cents: -100, currency: "EUR".into() },
            }],
        };
        assert!(withheld_source_fields(&parsed).is_empty());
    }

    /// `BT-195(BT-161)-NoticeResult` -> `BT-161`, and nothing else is a declaration.
    #[test]
    fn only_a_bt_195_field_id_yields_a_withheld_source() {
        assert_eq!(withheld_source("BT-195(BT-161)-NoticeResult"), Some("BT-161"));
        assert_eq!(withheld_source("BT-195(BT-541)-Lot-Weight"), Some("BT-541"));
        assert_eq!(withheld_source("BT-196(BT-161)-NoticeResult"), None, "the REASON is not a declaration");
        assert_eq!(withheld_source("BT-161-NoticeResult"), None);
        assert_eq!(withheld_source("BT-195"), None, "no parenthesised source");
        assert_eq!(withheld_source("BT-195(BT-161"), None, "unclosed");
    }

    /// Issue 300 §2.3: the N2 key folds case, punctuation, and spacing — and
    /// nothing else. Diacritics survive (cross-language collision safety);
    /// legal forms survive as tokens (N3's job, not N2's).
    #[test]
    fn the_n2_match_key_folds_punctuation_but_keeps_diacritics() {
        assert_eq!(match_norm("Ernst  & Young Advisory Services"), "ernst young advisory services");
        assert_eq!(match_norm("VERBRAEKEN INFRA n.v."), match_norm("Verbraeken Infra n v"));
        assert_eq!(match_norm("s.r.o."), match_norm("s. r. o."));
        assert_eq!(match_norm("„Sp. z o.o.”"), "sp z o o", "typographic quotes fold");
        assert_ne!(match_norm("Softronic AB"), match_norm("Softronic Aktiebolag"), "forms differ at N2");
        assert_eq!(match_norm("Gymnázium"), "gymnázium", "diacritics preserved");
        assert_ne!(match_norm("gymnázium"), match_norm("gymnazium"));
        assert_eq!(match_norm("  --  "), "", "all-punctuation collapses to empty");
    }

    /// Issue 346: Greek ALL CAPS drops the tonos, so the two casings of one
    /// name must meet at N2 — and the fold must stop at the Greek script:
    /// Latin diacritics still separate names (the 300 §2.3 line above).
    #[test]
    fn greek_casings_meet_at_n2_and_latin_diacritics_still_separate() {
        assert_eq!(match_norm("ΔΗΜΟΣ ΑΒΔΗΡΩΝ"), match_norm("Δήμος Αβδήρων"));
        assert_eq!(match_norm("Δήμος Αβδήρων"), "δημοσ αβδηρων");
        assert_eq!(
            match_norm("ΕΝΙΑΙΑ ΑΡΧΗ ΔΗΜΟΣΙΩΝ ΣΥΜΒΑΣΕΩΝ"),
            match_norm("Ενιαία Αρχή Δημοσίων Συμβάσεων")
        );
        // Dialytika, with and without tonos (Ϊ/ϊ/ΐ, Ϋ/ϋ/ΰ) all reach the bare vowel.
        assert_eq!(match_norm("ΠΡΟΪΟΝ"), match_norm("προϊόν"));
        assert_eq!(match_norm("ΓΑΪΔΟΥΡΟΝΗΣΙ"), match_norm("Γαϊδουρονήσι"));
        // Scope guard: the fold is Greek-only.
        assert_ne!(match_norm("MÜLLER"), match_norm("MULLER"));
        assert_eq!(match_norm("MÜLLER"), "müller");
        assert_ne!(match_norm("Ćwiek"), match_norm("Cwiek"));
    }

    /// Issue 292: one language vocabulary across eras. The legacy two-letter tags
    /// map to the eForms three-letter form the read layer's 'ENG'-wins picks
    /// compare against; already-canonical and unknown tags pass through
    /// uppercased; absent stays absent.
    #[test]
    fn lang_tags_normalize_to_one_vocabulary() {
        assert_eq!(normalize_lang(Some("EN")), Some("ENG".into()), "r209/text-era English");
        assert_eq!(normalize_lang(Some("DE")), Some("DEU".into()), "r209 German");
        assert_eq!(normalize_lang(Some("FR")), Some("FRA".into()));
        assert_eq!(normalize_lang(Some("CS")), Some("CES".into()), "T-form, not B-form CZE");
        assert_eq!(normalize_lang(Some("en")), Some("ENG".into()), "case-insensitive");
        assert_eq!(normalize_lang(Some("ENG")), Some("ENG".into()), "eForms passthrough");
        assert_eq!(normalize_lang(Some("DEU")), Some("DEU".into()));
        assert_eq!(normalize_lang(Some("XX")), Some("XX".into()), "unknown passes visible");
        assert_eq!(normalize_lang(None), None, "untagged stays untagged");
    }

    #[test]
    fn vm_hwm_is_parsed_from_a_proc_status_block() {
        // A realistic /proc/self/status excerpt — VmHWM is tab-padded and in kB.
        let status = "Name:\tserver\nVmPeak:\t 8000000 kB\nVmHWM:\t 6291456 kB\nVmRSS:\t 4000000 kB\n";
        assert_eq!(parse_vm_hwm_mb(status), 6144, "6291456 kB / 1024 = 6144 MB");
        // Absent or malformed lines yield 0, never a panic — a diagnostic must not
        // fail a projection.
        assert_eq!(parse_vm_hwm_mb("VmRSS:\t 100 kB\n"), 0, "no VmHWM line");
        assert_eq!(parse_vm_hwm_mb("VmHWM:\tnonsense kB\n"), 0, "unparseable value");
        assert_eq!(parse_vm_hwm_mb(""), 0, "empty");
    }

    /// The Phase-2 bucket codec must be lossless: a [`BucketRow`] carrying every
    /// fold-relevant shape (all Fact variants, a lot with facts, a full results
    /// Round) survives `postcard` serialize → `[u32 len][bytes]` framing →
    /// deserialize byte-for-byte. If it did not, the bucketed fold would not be
    /// byte-identical to the streaming path.
    #[test]
    fn bucket_row_survives_the_postcard_codec() {
        let mut facts = BTreeSet::new();
        facts.insert(Fact::Text { field: "BT-21".into(), lang: Some("ENG".into()), value: "Title".into() });
        facts.insert(Fact::Amount {
            field: "BT-27".into(),
            cents: 1_234_500,
            currency: "EUR".into(),
            tax_basis: None,
            quality: None,
        });
        // Both readings of issue 372's marker, or the codec is only tested on the
        // absent one -- and a bucketed fold that silently dropped `withheld` would
        // stop matching the streaming path it is required to be byte-identical to.
        facts.insert(Fact::Amount {
            field: "result_value".into(),
            cents: -100,
            currency: "EUR".into(),
            tax_basis: None,
            quality: Some(QUALITY_WITHHELD.into()),
        });
        facts.insert(Fact::Classification { field: "BT-262".into(), scheme: "CPV".into(), code: "45000000".into() });
        facts.insert(Fact::Date { field: "BT-131".into(), utc_seconds: 700_000_000, offset_minutes: 60, has_time: true });
        facts.insert(Fact::Party { role: "buyer".into(), organization_id: 7, notice_id: 3, section_id: "ORG-1".into() });

        let mut lot_facts = BTreeSet::new();
        lot_facts.insert(Fact::Text { field: "BT-21".into(), lang: None, value: "Lot title".into() });

        let round = Round {
            notice_id: 3,
            logical_notice_id: Some("PID-9".into()),
            lot_results: vec![LotResultState {
                key: "RES-1".into(),
                lot_key: Some("LOT-1".into()),
                decision: Some("selected".into()),
                reason: None,
                awarded_cents: Some(999),
                awarded_currency: Some("EUR".into()),
                decided: Some((700_050_000, -60, false)),
                winners: vec![7, 8],
                statistics: vec![("t1".into(), 4, None)],
            }],
            bids: vec![BidState {
                key: "TEN-1".into(),
                lot_key: Some("LOT-1".into()),
                cents: Some(500),
                currency: Some("EUR".into()),
                quality: None,
                parties: vec![BidParty { role: "tenderer".into(), organization_id: 8, section_id: "ORG-2".into() }],
            }],
            contracts: vec![ContractState {
                key: "CON-1".into(),
                buyer_contract_id: Some("BC-1".into()),
                concluded: Some((700_100_000, 0, false)),
                decided: Some((700_000_000, 60, false)),
                cents: Some(999),
                currency: Some("EUR".into()),
            }],
        };

        let row = BucketRow {
            group_key: "bt04-0001".into(),
            source: "ted".into(),
            source_rank: 1,
            notice_id: 3,
            publication_id: "k0001-w0".into(),
            published_at: 42,
            dispatched_at: Some(41),
            subtype: Some("cn-standard".into()),
            original_lang: Some("DEU".into()),
            is_correction: true,
            facts,
            lots: vec![LotState { key: "LOT-1".into(), kind: "Lot".into(), facts: lot_facts }],
            round: Some(round),
            // Issue 237: carried through the frame, so the round-trip covers it.
            group_members: vec![("GLO-1".into(), "LOT-1".into())],
        };

        // Frame exactly as write_buckets does, then read exactly as read_bucket does.
        let bytes = postcard::to_stdvec(&row).expect("serialize");
        let mut framed = (bytes.len() as u32).to_le_bytes().to_vec();
        framed.extend_from_slice(&bytes);
        let len = u32::from_le_bytes(framed[..4].try_into().unwrap()) as usize;
        let decoded: BucketRow = postcard::from_bytes(&framed[4..4 + len]).expect("deserialize");
        assert_eq!(row, decoded, "the bucket row must round-trip byte-identically");
    }

    /// Issue 237: which lots group a `GroupComposition` section attaches to.
    ///
    /// Three shapes, in the order they occur on prod: the composition names its group
    /// with BT-330; it names none but the notice publishes exactly one group (9,272 of
    /// ~9,694 carriers — inferred, since there is nothing to be wrong about); it names
    /// none and the notice publishes several (~406 — skipped, because nothing documents
    /// composition order as corresponding to group order, and a wrong row silently
    /// reassigns which lots a bid covered).
    #[test]
    fn a_group_composition_attaches_to_its_named_group_or_to_the_only_one() {
        const COMPOSITION: &str = "ND-GroupComposition";
        let compose = |groups: &[&str], named: Option<&str>, members: &[&str]| -> Parsed {
            let mut parsed = Parsed::default();
            for group in groups {
                parsed.sections.push(store::Section {
                    id: (*group).into(),
                    kind: "LotsGroup".into(),
                    parent: None,
                });
            }
            parsed.sections.push(store::Section {
                id: COMPOSITION.into(),
                kind: GROUP_COMPOSITION_KIND.into(),
                parent: None,
            });
            let mut push = |field: &str, value: &str, ordinal: i64| {
                parsed.values.push(store::ValueRow {
                    section_id: COMPOSITION.into(),
                    field_id: field.into(),
                    ordinal,
                    value: NoticeValue::Id { scheme: None, value: value.into(), is_ref: true },
                });
            };
            if let Some(id) = named {
                push(GROUP_ID_FIELD, id, 0);
            }
            for (i, member) in members.iter().enumerate() {
                push(GROUP_MEMBER_FIELD, member, i as i64);
            }
            parsed
        };
        let pair = |group: &str, member: &str| (group.to_owned(), member.to_owned());

        assert_eq!(
            group_members(1, &compose(&["GLO-1", "GLO-2"], Some("GLO-2"), &["LOT-3", "LOT-4"])),
            vec![pair("GLO-2", "LOT-3"), pair("GLO-2", "LOT-4")],
            "a named group wins, however many groups the notice publishes",
        );
        assert_eq!(
            group_members(2, &compose(&["GLO-1"], None, &["LOT-1", "LOT-2"])),
            vec![pair("GLO-1", "LOT-1"), pair("GLO-1", "LOT-2")],
            "the sole group is inferred — this is what covers the corpus (issue 237)",
        );
        assert_eq!(
            group_members(3, &compose(&["GLO-1", "GLO-2"], None, &["LOT-1"])),
            Vec::<(String, String)>::new(),
            "several groups and no BT-330: skipped, not paired by document order",
        );
        assert_eq!(
            group_members(4, &compose(&[], None, &["LOT-1"])),
            Vec::<(String, String)>::new(),
            "no group at all: the members have nothing to attach to",
        );
    }

    /// ADR-0011: a previous-notice reference becomes a `notices.publication_id`, or
    /// nothing. eForms writes `615938-2024`; the archive holds `00615938-2024`.
    /// A reference that does not parse is dropped rather than guessed at — a wrong
    /// publication id merges two unrelated procedures, which is worse than leaving
    /// them apart.
    #[test]
    fn a_previous_notice_reference_normalises_to_an_archive_publication_id() {
        assert_eq!(publication_ref("615938-2024").as_deref(), Some("00615938-2024"), "the published form");
        assert_eq!(publication_ref("00615938-2024").as_deref(), Some("00615938-2024"), "already padded");
        assert_eq!(publication_ref(" 1-2019 ").as_deref(), Some("00000001-2019"), "surrounding space, short number");
        assert_eq!(publication_ref("12345678-2024").as_deref(), Some("12345678-2024"), "the full width");

        for bad in [
            "615938",           // no year
            "615938-24",        // two-digit year
            "615938-20244",     // five-digit year
            "-2024",            // no number
            "0-2024",           // number zero is not a publication
            "123456789-2024",   // wider than the archive's own form
            "61x938-2024",      // not digits
            "615938-20a4",
            "",
        ] {
            assert_eq!(publication_ref(bad), None, "{bad:?} is not a publication id");
        }
    }

    /// Issue 237: membership must survive to the version that HAS the bids.
    ///
    /// The composition is published by the notice that defines the groups — a contract
    /// notice — and the bids referencing a group arrive with the award notice, versions
    /// later. Measured on prod: 1,408 bids across 505 notices reference a `LotsGroup`,
    /// and membership read only from its own notice sits on the one version with no bids
    /// at all. So it carries forward like `lots` and `facts`, superseded per group.
    /// Issue 259: nesting means "same party", and it means it all the way up.
    ///
    /// The corpus case is two levels (`WINNER` > `ADDRESS_WINNER`), which the r209
    /// fixture covers end to end. This pins the two things a fixture cannot: that a
    /// THREE-level nest lands the innermost on the OUTERMOST rather than on its parent —
    /// otherwise two aliases would chain and one would resolve to a section that mints no
    /// mention — and that a section which merely SITS under a LotResult is untouched,
    /// since only Organization-inside-Organization is the signal.
    #[test]
    fn a_nested_organization_aliases_to_the_outermost_one() {
        let sections = vec![
            store::Section { id: "PROCEDURE".into(), kind: "Notice".into(), parent: None },
            store::Section {
                id: "RES-1".into(),
                kind: "LotResult".into(),
                parent: Some("PROCEDURE".into()),
            },
            // Three deep: the wrapper, its address block, and a transliterated address
            // inside that — all three are `Rule::Org` in the legacy vocabulary.
            store::Section {
                id: "ORG-2".into(),
                kind: ORGANIZATION_KIND.into(),
                parent: Some("RES-1".into()),
            },
            store::Section {
                id: "ORG-3".into(),
                kind: ORGANIZATION_KIND.into(),
                parent: Some("ORG-2".into()),
            },
            store::Section {
                id: "ORG-4".into(),
                kind: ORGANIZATION_KIND.into(),
                parent: Some("ORG-3".into()),
            },
            // A sibling party of its own, under the same result. Not nested, not aliased.
            store::Section {
                id: "ORG-5".into(),
                kind: ORGANIZATION_KIND.into(),
                parent: Some("RES-1".into()),
            },
        ];
        let by_id: HashMap<&str, &store::Section> =
            sections.iter().map(|s| (s.id.as_str(), s)).collect();
        let alias = nested_org_aliases(&by_id, &[ORGANIZATION_KIND]);

        assert_eq!(alias.get("ORG-3").map(String::as_str), Some("ORG-2"));
        assert_eq!(
            alias.get("ORG-4").map(String::as_str),
            Some("ORG-2"),
            "the innermost must reach the OUTERMOST, not merely its parent — an alias \
             pointing at another alias resolves to a section that mints no mention"
        );
        assert_eq!(alias.get("ORG-2"), None, "the outermost is the party, not an alias");
        assert_eq!(alias.get("ORG-5"), None, "a sibling under the result is its own party");
        assert_eq!(alias.len(), 2);

        // eForms shape: Organizations are siblings under a non-Organization container,
        // so nothing aliases and this change is a no-op for the modern eras.
        let flat = vec![
            store::Section { id: "PROCEDURE".into(), kind: "Notice".into(), parent: None },
            store::Section {
                id: "ORG-0001".into(),
                kind: ORGANIZATION_KIND.into(),
                parent: Some("PROCEDURE".into()),
            },
            store::Section {
                id: "ORG-0002".into(),
                kind: ORGANIZATION_KIND.into(),
                parent: Some("PROCEDURE".into()),
            },
        ];
        let by_id: HashMap<&str, &store::Section> =
            flat.iter().map(|s| (s.id.as_str(), s)).collect();
        assert!(nested_org_aliases(&by_id, &[ORGANIZATION_KIND]).is_empty());
    }

    /// ADR-0013 D4: `mentions()` keeps every LABELLED language variant of the
    /// party's name for the `organization_names` satellite — while the single
    /// designated `name` keeps its first-seen semantics untouched.
    #[test]
    fn mention_capture_keeps_labelled_name_variants_per_language() {
        let parsed = Parsed {
            sections: vec![
                store::Section { id: "PROCEDURE".into(), kind: "Notice".into(), parent: None },
                store::Section {
                    id: "ORG-0001".into(),
                    kind: ORGANIZATION_KIND.into(),
                    parent: Some("PROCEDURE".into()),
                },
            ],
            values: vec![
                // eForms multilingual shape: BT-500 repeated per language.
                text_value("ORG-0001", ORG_NAME_FIELD, 0, Some("DE"), "Stadt Brüssel"),
                text_value("ORG-0001", ORG_NAME_FIELD, 1, Some("FR"), "Ville de Bruxelles"),
                // A repeat of an already-seen language: first wins.
                text_value("ORG-0001", ORG_NAME_FIELD, 2, Some("DEU"), "Stadt Bruessel (dupe)"),
                // Unlabelled: feeds the designated head only, never the satellite.
                text_value("ORG-0001", ORG_NAME_FIELD, 3, None, "City of Brussels"),
            ],
        };
        let mentions = NoticeState::mentions(false, 7, &parsed);
        assert_eq!(mentions.len(), 1);
        let m = &mentions[0];
        assert_eq!(m.name, "Stadt Brüssel", "the head keeps first-seen semantics");
        assert_eq!(
            m.variants,
            vec![
                ("DEU".to_owned(), "Stadt Brüssel".to_owned()),
                ("FRA".to_owned(), "Ville de Bruxelles".to_owned()),
            ],
            "labelled variants canonicalised and deduped per language; unlabelled excluded"
        );

        // The legacy unlabelled shape stays variant-free.
        let legacy = Parsed {
            sections: vec![store::Section {
                id: "ORG-1".into(),
                kind: ORGANIZATION_KIND.into(),
                parent: None,
            }],
            values: vec![text_value("ORG-1", "TED-OFFICIALNAME", 0, None, "Mairie de Paris")],
        };
        let m = &NoticeState::mentions(false, 8, &legacy)[0];
        assert_eq!((m.name.as_str(), m.variants.len()), ("Mairie de Paris", 0));
    }

    fn text_value(
        section: &str,
        field: &str,
        ordinal: i64,
        lang: Option<&str>,
        value: &str,
    ) -> store::ValueRow {
        store::ValueRow {
            section_id: section.into(),
            field_id: field.into(),
            ordinal,
            value: store::NoticeValue::Text {
                lang: lang.map(Into::into),
                value: value.into(),
            },
        }
    }

    #[test]
    fn lots_group_membership_carries_forward_and_supersedes_per_group() {
        let state = |notice_id: i64, members: &[(&str, &str)]| NoticeState {
            notice_id,
            publication_id: format!("k{notice_id:04}-w0"),
            published_at: notice_id * 1_000,
            dispatched_at: None,
            subtype: None,
            original_lang: None,
            logical_id: None,
            is_correction: false,
            facts: BTreeSet::new(),
            lots: Vec::new(),
            roles: Vec::new(),
            raw_results: RawResults::default(),
            round: None,
            org_alias: HashMap::new(), // no parties in this fixture (issue 259)
            group_members: members
                .iter()
                .map(|(g, m)| ((*g).to_owned(), (*m).to_owned()))
                .collect(),
        };
        let pair = |group: &str, member: &str| (group.to_owned(), member.to_owned());

        // A contract notice composes two groups; a corrigendum says nothing about either;
        // an award notice republishes GLO-1 with a different member list.
        let composed = state(1, &[("GLO-1", "LOT-1"), ("GLO-1", "LOT-2"), ("GLO-2", "LOT-9")]);
        let silent = state(2, &[]);
        let recomposed = state(3, &[("GLO-1", "LOT-3")]);
        let chain: Vec<&NoticeState> = vec![&composed, &silent, &recomposed];
        let versions = fold(&chain);

        assert_eq!(
            versions[0].group_members,
            vec![pair("GLO-1", "LOT-1"), pair("GLO-1", "LOT-2"), pair("GLO-2", "LOT-9")],
            "the composing notice's own pairs",
        );
        assert_eq!(
            versions[1].group_members, versions[0].group_members,
            "a notice silent about every group changes no membership — this is the case that \
             matters, because the award notice is usually the silent one",
        );
        assert_eq!(
            versions[2].group_members,
            vec![pair("GLO-1", "LOT-3"), pair("GLO-2", "LOT-9")],
            "republishing GLO-1 replaces ITS list entirely and leaves GLO-2 alone",
        );
    }

    #[test]
    fn business_term_stems_survive_their_context_suffix() {
        assert_eq!(stem("BT-21-Lot"), "BT-21");
        assert_eq!(stem("BT-21-Procedure"), "BT-21");
        assert_eq!(stem("BT-131(d)-Lot"), "BT-131(d)");
        assert_eq!(stem("OPP-070-notice"), "OPP-070");
        assert_eq!(stem("BT-04"), "BT-04");
    }

    /// Issue 187: the `internal-ojs` 2008 export chains by OJS number like the
    /// TED legacy forms, so it must classify legacy — otherwise its notices get
    /// no `ojs_self` node and the plan-row `legacy` gate keeps their `REF_NOTICE`
    /// edges out of the union-find, leaving every 2008 award a 100%-unchained
    /// island. Its `publication_id` (`<number>-2008`) must also parse as an OJS
    /// key, or the node would still be absent.
    #[test]
    fn internal_ojs_chains_by_ojs_like_the_legacy_forms() {
        assert!(is_legacy_profile("internal-ojs"), "internal-ojs chains by OJS");
        // The families it must not disturb.
        assert!(is_legacy_profile("text"));
        assert!(is_legacy_profile("ted-export-r208"));
        assert!(is_legacy_profile("ted-export-r209"));
        assert!(!is_legacy_profile("eforms:eforms-sdk-1.0"));
        assert!(!is_legacy_profile("eforms:eforms-sdk-0.1"));
        // Its publication_id shape is `<number>-<year>` — the DOC/text-era form
        // ojs_key parses, so ojs_self populates once the profile is legacy.
        assert_eq!(ojs_key("115165-2008"), Some((2008, 115165)));
    }

    /// The DE-1.x fold is only correct if every alias target is an id the
    /// projection actually recognises — a typo'd target is silent, exactly the
    /// failure mode issue 85 was. Each target must be reachable by one of the
    /// canonical tables, the instant lists, the org/identity constants, or the
    /// results-graph stems.
    /// The predicate the two gates below and the drop diagnostic all share.
    /// Every case here is one the union has to get right for a COUNT of dropped
    /// values to mean anything.
    #[test]
    fn has_destination_answers_per_channel_not_per_field() {
        // The case that motivated it. `TED-TI_TEXT` is the OJ heading's CPV
        // label in 23 languages (issue 368): the projection reads its sibling
        // TED-TI_DOC as a last-resort title and reads this one nowhere.
        assert!(!has_destination("TED-TI_TEXT", Channel::Text));
        assert!(has_destination("TED-TI_DOC", Channel::Text));
        // Stem matching: the tables key `BT-21`, the notices publish contexts.
        assert!(has_destination("BT-21-Lot", Channel::Text));
        assert!(has_destination("BT-21-Procedure", Channel::Text));
        // …and the channel is what makes the answer meaningful. `role_name`
        // accepts ANY `TED-` id, so on the pointer channel a legacy id reads,
        // while the very same id has no text destination. A channel-blind
        // predicate would call the whole titleless r208 era "read".
        assert!(has_destination("TED-TI_TEXT", Channel::Id { is_ref: true }));
        assert!(!has_destination("TED-TI_TEXT", Channel::Date));
        assert!(!has_destination("TED-TI_TEXT", Channel::Amount));
        // A DE-1.x alias resolves to its eForms target: the notice layer keeps
        // the publisher's spelling, so without this step the entire eForms-DE
        // vocabulary would read as dropped.
        assert!(has_destination("DE1-ProcurementProject-Name", Channel::Text));
        assert!(has_destination(DE1_FOLDER_FIELD, Channel::Id { is_ref: false }));
        // Full-id keyed dialects, which the coarse stem cannot separate.
        assert!(has_destination("SDK01-ProcurementProject-Name", Channel::Text));
        assert!(has_destination("SDK01-ProcurementProject-Description", Channel::Text));
        // Identity and the results graph, on their own channels.
        assert!(has_destination(PROCEDURE_KEY_FIELD, Channel::Id { is_ref: false }));
        assert!(has_destination("BT-720-Tender", Channel::Amount));
        assert!(has_destination("BT-142-LotResult", Channel::Code));
        assert!(has_destination("BT-759-LotResult", Channel::Number));
        assert!(has_destination("OPT-320-LotResult", Channel::Id { is_ref: true }));
        // Channels with no fact table at all: a code or a number that no
        // vocabulary claims really has nowhere to go, and the diagnostic should
        // say so rather than hide it (the UBL_PARSE_ONLY ledger's whole point).
        assert!(!has_destination("UBL-AddressFormatCode", Channel::Code));
        assert!(!has_destination("UBL-AwardCriterionWeightNumeric", Channel::Number));
        // Nonsense is not read on any channel.
        assert!(!any_channel_reads("NOT-A-FIELD-ID"));
        assert!(!any_channel_reads(""));

        // The sieve the diagnostics use asks the question of the row's OWN
        // channel. This is the case the channel-blind form got wrong on prod:
        // a legacy text element read by no text channel, yet "read" through
        // `any_channel_reads` because every `TED-` id is a role on the pointer
        // channel — 0 unmapped of 311 for r208 (2026-09-12). `TED-TI_TEXT` is
        // the standing example; the four form-specific title elements that
        // finding exposed are mapped now (unit 2) and read on the text channel.
        assert!(any_channel_reads("TED-TI_TEXT"), "the blind form says read, which is the trap");
        assert!(!table_reads("notice_texts", "TED-TI_TEXT"), "no text destination");
        for title in [
            "TED-TITLE_QUALIFICATION_SYSTEM",
            "TED-TITLE_RESULT_DESIGN_CONTEST",
            "TED-TITLE_DESIGN_CONTACT_NOTICE",
            "TED-TITLE_NOTICE_BUYER_PROFILE",
        ] {
            assert!(table_reads("notice_texts", title), "{title}: mapped to title in unit 2");
        }
        assert!(table_reads("notice_texts", "TED-TI_DOC"));
        assert!(table_reads("notice_texts", "TED-TITLE"));
        assert!(table_reads("notice_texts", "TED-LOT_TITLE"));
        // The legacy award date is consumed by the results reader; the predicate
        // must say so or the probe lists 1,688 rows of it as dropped (it did).
        assert!(table_reads("notice_dates", LEGACY_AWARD_DATE_FIELD));
        assert!(table_reads("notice_dates", LEGACY_AWARD_DATE_FIELD_R207));
        // Two more the issue-384 guard found on its first run: the no-award marker
        // (a wildcard arm in the reader, an Integer(1) in the store) and the bid
        // count on its Number spelling.
        assert!(table_reads("notice_integers", LEGACY_NO_AWARD_MARKER));
        assert!(table_reads("notice_numbers", LEGACY_BID_COUNT_FIELDS[0]));
        assert!(table_reads("notice_integers", LEGACY_BID_COUNT_FIELDS[0]));
        assert!(table_reads("notice_texts", "BT-21-Lot"));
        // A legacy address block IS read where it is stored — as a role.
        assert!(table_reads("notice_ids", "TED-ADDRESS_CONTRACTING_BODY"));
        // The same id on a channel that does not carry it is not read there.
        assert!(!table_reads("notice_amounts", "TED-TITLE"));
        // An unknown table reads nothing, so a misspelling shows as everything dropped.
        assert!(table_channels("notice_typo").is_empty());
        assert!(!table_reads("notice_typo", "BT-21-Lot"));
    }

    /// Issue 384: every field id the legacy results reader matches on has a
    /// destination on the channel it matches it on, so `has_destination` cannot
    /// drift from the reader again. The drift is what put 1,688 rows of a consumed
    /// award date on the r208 probe's dropped list (issue 368), and this test's
    /// first run found two more: the no-award marker behind a wildcard arm, and
    /// the bid count on its Number spelling.
    ///
    /// Source-read, like `every_report_field_is_read_by_the_renderer`: the
    /// invariant is about the reader's match arms, so the arms are what it reads.
    /// Literal arms are parsed; const-named arms are asserted by name below, and a
    /// wildcard value pattern on a `TED-` literal is refused outright, because it
    /// hides the channel the predicate would need.
    #[test]
    fn every_id_the_legacy_results_reader_matches_has_a_destination_on_its_channel() {
        let src = include_str!("project.rs");
        let start = src.find("\nfn read_legacy_results(").expect("reader");
        let body = &src[start..start + src[start..].find("\n}\n").expect("reader end")];

        let mut literal_arms = 0;
        let mut rest = body;
        while let Some(i) = rest.find("(\"TED-") {
            let arm = &rest[i + 1..];
            let end = arm.find(", ").expect("a value pattern after the field literal");
            let (ids, value) = (&arm[..end], &arm[end + 2..]);
            let channels: &[Channel] = if value.starts_with("NoticeValue::Amount") {
                &[Channel::Amount]
            } else if value.starts_with("NoticeValue::Date") {
                &[Channel::Date]
            } else if value.starts_with("NoticeValue::Integer") {
                &[Channel::Integer]
            } else if value.starts_with("NoticeValue::Number") {
                &[Channel::Number]
            } else if value.starts_with("NoticeValue::Text") {
                &[Channel::Text]
            } else if value.starts_with("NoticeValue::Code") {
                &[Channel::Code]
            } else if value.starts_with("NoticeValue::Id") {
                &[Channel::Id { is_ref: true }, Channel::Id { is_ref: false }]
            } else {
                panic!(
                    "read_legacy_results matches {ids} with a value pattern that names no \
                     channel ({}) — name the variant, so the predicate can be held to it",
                    value.split(')').next().unwrap_or(value)
                );
            };
            for id in ids.split('|').map(|s| s.trim().trim_matches('"')) {
                assert!(id.starts_with("TED-"), "unexpected pattern piece {id:?} in {ids}");
                assert!(
                    channels.iter().any(|c| has_destination(id, *c)),
                    "read_legacy_results consumes {id} on {channels:?} but has_destination says \
                     nothing reads it there — the diagnostics would list it as dropped"
                );
                literal_arms += 1;
            }
            rest = &arm[end..];
        }
        assert!(literal_arms >= 5, "parsed too few literal arms ({literal_arms}); the scan is broken");

        // The const-named arms and slices the reader consults, held by name.
        assert!(has_destination(LEGACY_AWARD_DATE_FIELD, Channel::Date));
        assert!(has_destination(LEGACY_AWARD_DATE_FIELD_R207, Channel::Date));
        assert!(has_destination(LEGACY_NO_AWARD_MARKER, Channel::Integer));
        for f in LEGACY_BID_COUNT_FIELDS {
            assert!(has_destination(f, Channel::Integer), "{f}: Integer");
            assert!(has_destination(f, Channel::Number), "{f}: Number");
        }
        // And every const the reader matches on is one this test names.
        for name in [
            "LEGACY_AWARD_DATE_FIELD",
            "LEGACY_AWARD_DATE_FIELD_R207",
            "LEGACY_NO_AWARD_MARKER",
            "LEGACY_BID_COUNT_FIELDS",
        ] {
            assert!(body.contains(name), "{name} is no longer used by the reader; update this test");
        }
    }

    /// Every satellite table a "published and dropped" diagnostic walks has a
    /// channel here, so a new satellite cannot be walked and then reported as
    /// wholly dropped for want of a mapping. The list is the one both
    /// diagnostics use (`data_quality::unmapped_fields_sql`, the store probe).
    #[test]
    fn every_walked_satellite_table_has_a_channel() {
        for table in [
            "notice_texts",
            "notice_codes",
            "notice_classifications",
            "notice_amounts",
            "notice_dates",
            "notice_integers",
            "notice_numbers",
            "notice_ids",
        ] {
            assert!(!table_channels(table).is_empty(), "{table} feeds no channel");
        }
    }

    /// The report's sieve must be the per-channel one. Source-read, because the
    /// blind form is private now and a same-crate caller is the one path left;
    /// the same guard shape as `every_report_field_is_read_by_the_renderer`.
    #[test]
    fn the_report_sieve_is_per_channel() {
        let src = include_str!("data_quality.rs");
        assert!(
            !src.contains("any_channel_reads("),
            "data_quality.rs must sieve with table_reads, not the channel-blind predicate"
        );
        assert!(src.contains("table_reads("), "data_quality.rs must use table_reads");
    }

    /// Every DE-1.x alias names an eForms field the projection actually reads,
    /// so the rewrite cannot point at a destination that does not exist.
    #[test]
    fn every_de1_alias_target_is_a_field_the_projection_reads() {
        for (de1, target) in DE1_FIELD_ALIASES {
            // One predicate, shared with the diagnostic that counts what the fold
            // drops: this gate used to open-code its own union, and an answer
            // enforced in a test but unavailable to production is how a whole era
            // of unread field ids went uncounted (issue 368).
            assert!(
                any_channel_reads(target),
                "{de1} → {target}: the projection reads no such field"
            );
            assert!(de1.starts_with("DE1-"), "{de1}: not a DE-1.x source id");
        }
    }

    /// Issue 88 / ADR-0004: every grafted `UBL-*` id has exactly one disposition
    /// — MAPPED (a fact table above) or EXPLICITLY IGNORED (`UBL_PARSE_ONLY`,
    /// with a reason). Before this gate the grafts were neither: the parser
    /// captured them and the fold silently dropped them, the same class that
    /// produced issue 85's 218K factless tenders. The inventory is read from
    /// index.rs's SOURCE, so adding a graft fails here until it is dispositioned
    /// — exactly how the docs page is held to openapi.json.
    #[test]
    fn ubl_grafts_are_all_mapped_or_ignored() {
        let index = include_str!("eforms/index.rs");
        let mut ids = std::collections::BTreeSet::new();
        for (pos, _) in index.match_indices("UBL-") {
            let body: String = index[pos + 4..].chars().take_while(char::is_ascii_alphabetic).collect();
            if !body.is_empty() {
                ids.insert(format!("UBL-{body}"));
            }
        }
        assert!(ids.len() >= 50, "the graft inventory extraction broke: {} ids", ids.len());

        for id in &ids {
            let mapped = any_channel_reads(id);
            let ignored = UBL_PARSE_ONLY.iter().any(|(i, _)| i == id);
            assert!(
                mapped || ignored,
                "{id}: no disposition — ADR-0004 allows mapped or explicitly ignored, \
                 not silently dropped; map it or enter it in UBL_PARSE_ONLY with a reason"
            );
            assert!(!(mapped && ignored), "{id}: contradictory disposition — mapped AND parse-only");
        }

        // The ledger cannot rot: every entry names a graft that still exists.
        for (id, reason) in UBL_PARSE_ONLY {
            assert!(ids.contains(*id), "{id}: stale UBL_PARSE_ONLY entry (no such graft)");
            assert!(!reason.is_empty(), "{id}: an ignore without a reason is not a disposition");
        }
    }

    /// One source id must not map two ways — a duplicate would make the fold
    /// depend on table order.
    /// Issues 94/98: an alias may never move a value INTO a field that decides the
    /// Tender key, its kind, or the fold order — the second lever, distinct from
    /// `is_ref`, by which a "mapping-only" change can silently re-group the corpus.
    ///
    /// `normalise_de1` rewrites `field_id`, so an alias targeting one of these
    /// injects a value the grouping reads. The worst case is `BT-04-notice`:
    /// `procedure_key` returns it **unchecked** (on TED it is a spec-guaranteed
    /// uuid), so an alias pointing there would key Tenders on a raw portal string
    /// with no gate at all — the issue-34 collapse, reached around the `is_uuid`
    /// guard that exists to prevent it. `published_at` is the subtler one: it
    /// orders versions inside a Tender via `plan_notice_fold`, so perturbing it
    /// renumbers every `seq`.
    ///
    /// An allowlist rather than a ban, because five identity aliases legitimately
    /// target this set and produced the current fold. Anything else must be a
    /// deliberate edit here, with a fold-impact review attached.
    #[test]
    fn no_de1_alias_reaches_the_grouping_or_the_fold_order() {
        let mut decides_the_fold: Vec<&str> = vec![
            PROCEDURE_KEY_FIELD,  // the Tender key, read UNCHECKED
            DE1_FOLDER_FIELD,     // the gated key fallback
            SUBTYPE_FIELD,        // tenders.kind + the plan group's first_subtype
            LOGICAL_NOTICE_FIELD, // correction dedup — moves results, not grouping
        ];
        decides_the_fold.extend(PUBLICATION_DATE_FIELDS); // published_at = fold order
        decides_the_fold.extend(DISPATCH_DATE_FIELDS); // dispatched_at, and published_at by fallback
        decides_the_fold.extend(LEGACY_OWN_NUMBER_FIELDS); // legacy OJS identity
        // Issue 369 unit 2: BUYER identity now decides TENDER identity. The key election
        // refuses a placeholder-shaped procedure key whose notices disagree on their buyer,
        // so the buyer role pointer and the organization fields `buyer_key` reads are
        // inputs to grouping for the first time. Declared here rather than discovered:
        // this list is HAND-MAINTAINED and the assertion below only checks aliases against
        // what it contains, so an undeclared widening is invisible — the gate cannot go red
        // on its own, which is the opposite of what issue 369's coupling note assumed.
        decides_the_fold.push("OPT-300-Procedure-Buyer"); // the eForms buyer role reference
        decides_the_fold.push(ORG_NAME_FIELD);
        decides_the_fold.push(ORG_IDENTIFIER_FIELD);
        decides_the_fold.push(ORG_COUNTRY_FIELD);
        decides_the_fold.push(ORG_NATIONALID_FIELD);
        decides_the_fold.extend(ORG_NAME_FIELDS);
        decides_the_fold.extend(ORG_COUNTRY_FIELDS);
        decides_the_fold.extend(SDK01_PARTY_NAME_FIELDS);
        decides_the_fold.extend(SDK01_PARTY_COUNTRY_FIELDS);

        // The identity aliases that deliberately target it (issue 85's DE-1.x line).
        const IDENTITY: &[(&str, &str)] = &[
            ("DE1-ID", LOGICAL_NOTICE_FIELD),
            ("DE1-NoticeSubType-SubTypeCode", SUBTYPE_FIELD),
            ("DE1-Publication-PublicationDate", "OPP-012-notice"),
            ("DE1-RequestedPublicationDate", "BT-738-notice"),
            ("DE1-IssueDate", "BT-05(a)-notice"),
            // Issue 369 unit 2, decided 2026-09-08. A DE-1.x notice's buyer reference is an
            // input to TENDER identity, and the fold-impact review is recorded on the issue.
            // In short: the blast radius does not change, because a buyer only reaches the
            // election through the refusal and the refusal is gated on `key_shaped = 1` — the
            // 14 measured tenders. And excluding this alias would be WORSE than including it:
            // the gate counts buyer disagreement, so dropping one dialect's buyers makes that
            // count dialect-dependent, and a key whose weld is visible only through its DE-1.x
            // notices would be UNDER-counted and silently admitted. Under-refusing is the
            // silent direction; over-refusing merely splits (CONTEXT.md:112-113).
            //
            // NOT justified by "it is needed for tender 1" — that was checked and is false:
            // tender 1's three buyers are each carried by >= 2 notices, so it is refused with
            // or without the DE-1.x one.
            ("DE1-ContractingParty-Party-PartyIdentification-ID", "OPT-300-Procedure-Buyer"),
            ("DE1-Organizations-Organization-Company-PartyName-Name", ORG_NAME_FIELD),
            ("DE1-Organizations-Organization-Company-PartyLegalEntity-CompanyID", ORG_IDENTIFIER_FIELD),
            ("DE1-Organizations-Organization-Company-PostalAddress-Country-IdentificationCode", ORG_COUNTRY_FIELD),
        ];

        for (de1, target) in DE1_FIELD_ALIASES {
            if decides_the_fold.contains(target) {
                assert!(
                    IDENTITY.contains(&(de1, target)),
                    "alias {de1} → {target} moves a value into the grouping/fold-order set. \
                     That re-groups or re-orders the corpus and needs a fold-impact review, \
                     not a mapping review — add it to IDENTITY here only once that is done."
                );
            }
            assert_ne!(
                *de1, DE1_FOLDER_FIELD,
                "aliasing the folder id AWAY would destroy the DE-1.x Tender key"
            );
        }
    }

    #[test]
    fn de1_aliases_are_unique() {
        let mut seen: Vec<&str> = DE1_FIELD_ALIASES.iter().map(|(de1, _)| *de1).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), before, "duplicate DE-1.x source id in the alias table");
    }

    /// Issue 367: the two date lists are read at two layers — on the RAW parse
    /// by the processor, on the NORMALISED parse by the projection — so every
    /// alias into either list must appear in it, **immediately after its own
    /// target**. Missing it stamps the notice row with a wrong date (the 218,876
    /// epoch rows). Present but mis-ordered is worse and quieter: the raw list
    /// would then prefer a different candidate than the folded one, and the
    /// notice row and its version would carry two different real dates.
    #[test]
    fn every_de1_date_alias_sits_beside_its_target() {
        for list in [PUBLICATION_DATE_FIELDS, DISPATCH_DATE_FIELDS] {
            for (de1, target) in DE1_FIELD_ALIASES {
                let Some(at) = list.iter().position(|f| f == target) else { continue };
                let found = list.iter().position(|f| f == de1);
                assert_eq!(
                    found,
                    Some(at + 1),
                    "{de1} → {target}: a date alias must sit immediately after its target in \
                     the same list, so the raw and the normalised parse resolve the same \
                     candidate (issue 367). Found at {found:?}, target at {at}."
                );
            }
        }
    }

    /// The repair job's read filter must cover both axes exactly — a missing id
    /// makes the job re-derive an instant from an incomplete parse and "repair"
    /// a correct row into a wrong one.
    #[test]
    fn the_instant_field_filter_is_both_lists() {
        for f in PUBLICATION_DATE_FIELDS.iter().chain(DISPATCH_DATE_FIELDS) {
            assert!(INSTANT_DATE_FIELDS.contains(f), "{f} missing from INSTANT_DATE_FIELDS");
        }
        assert_eq!(
            INSTANT_DATE_FIELDS.len(),
            PUBLICATION_DATE_FIELDS.len() + DISPATCH_DATE_FIELDS.len()
        );
    }

    #[test]
    fn the_de1_lot_node_splits_back_into_lot_lotsgroup_and_part() {
        assert_eq!(de1_lot_kind("LOT-0001"), "Lot");
        assert_eq!(de1_lot_kind("GLO-0001"), "LotsGroup");
        assert_eq!(de1_lot_kind("PAR-0001"), "Part");
        // Unknown shapes stay visible as plain Lots rather than being dropped.
        assert_eq!(de1_lot_kind("LOT-0002-extra"), "Lot");
        assert_eq!(de1_lot_kind(""), "Lot");
        // The kinds it produces are the ones the fold actually looks for.
        for id in ["LOT-0001", "GLO-0001", "PAR-0001"] {
            assert!(LOT_KINDS.contains(&de1_lot_kind(id)), "{id}");
        }
    }

    /// The DE-1.x folder id keys a Tender only when it is a genuine uuid, exactly
    /// as sdk-0.1's is. Ungated it would key on any non-empty string, and every
    /// notice sharing a portal-local reference number would collapse into one
    /// Tender — issue 34's wrong merge, at 218k scale (issue 85).
    #[test]
    fn a_de1_folder_id_keys_a_tender_only_when_it_is_a_uuid() {
        let folder = |value: &str| Parsed {
            sections: vec![store::Section { id: "PROCEDURE".into(), kind: "Notice".into(), parent: None }],
            values: vec![store::ValueRow {
                section_id: "PROCEDURE".into(),
                field_id: DE1_FOLDER_FIELD.into(),
                ordinal: 0,
                value: NoticeValue::Id { scheme: None, value: value.into(), is_ref: false },
            }],
        };

        let uuid = folder("3f2504e0-4f89-41d3-9a0c-0305e82c3301");
        assert_eq!(
            procedure_key(&uuid, false, true).as_deref(),
            Some("3f2504e0-4f89-41d3-9a0c-0305e82c3301"),
            "a genuine uuid keys the Tender, so a DÖE notice still merges with its TED twin"
        );

        // Portal-local shapes that must never key a Tender.
        for local in ["2023-001", "VG-2024-0815", "12345", "", "   ", "not-a-uuid"] {
            assert_eq!(
                procedure_key(&folder(local), false, true),
                None,
                "{local:?} must stay an island, not merge every notice that shares it"
            );
        }

        // The gate is scoped: a non-DE-1.x notice never reads this field at all.
        assert_eq!(procedure_key(&uuid, false, false), None);
    }

    /// Issue 369 unit 2: the gate counts DISTINCT buyer keys across a key's notices, so
    /// what `buyer_key` encodes decides whether the gate is right. Keying on the SET is
    /// what lets `>= 3` refuse a weld while admitting a joint procurement — the census's
    /// tender 1 carries three DISJOINT buyers across its versions (three sets), whereas a
    /// central purchasing body plus its participating authorities is one set repeated.
    /// Counting individual buyers instead would refuse the joint procurement.
    #[test]
    fn the_buyer_key_is_the_notices_buyer_set_so_a_joint_procurement_stays_one_key() {
        // One eForms notice: a Procedure section referencing `orgs` as its buyers, and one
        // Organization section per buyer carrying BT-500/501/514.
        let notice = |orgs: &[(&str, &str, &str)]| -> Parsed {
            let mut sections =
                vec![store::Section { id: "PROC".into(), kind: "Notice".into(), parent: None }];
            let mut values = Vec::new();
            for (id, name, nat) in orgs {
                sections.push(store::Section {
                    id: (*id).into(),
                    kind: ORGANIZATION_KIND.into(),
                    parent: None,
                });
                values.push(store::ValueRow {
                    section_id: "PROC".into(),
                    field_id: "OPT-300-Procedure-Buyer".into(),
                    ordinal: 0,
                    value: NoticeValue::Id { scheme: None, value: (*id).into(), is_ref: true },
                });
                values.push(store::ValueRow {
                    section_id: (*id).into(),
                    field_id: ORG_NAME_FIELD.into(),
                    ordinal: 0,
                    value: NoticeValue::Text { value: (*name).into(), lang: None },
                });
                values.push(store::ValueRow {
                    section_id: (*id).into(),
                    field_id: ORG_COUNTRY_FIELD.into(),
                    ordinal: 0,
                    value: NoticeValue::Code { list: None, code: "DEU".into() },
                });
                if !nat.is_empty() {
                    values.push(store::ValueRow {
                        section_id: (*id).into(),
                        field_id: ORG_IDENTIFIER_FIELD.into(),
                        ordinal: 0,
                        value: NoticeValue::Id { scheme: None, value: (*nat).into(), is_ref: false },
                    });
                }
            }
            Parsed { sections, values }
        };

        // A joint procurement: the SAME two buyers on both notices, listed in opposite
        // order. One key, because the set is sorted before it is joined — so `>= 3` never
        // fires on it however the source orders its parties.
        let a = buyer_key(false, 1, &notice(&[("ORG-1", "Stadt Aachen", "DE811907980"),
                                              ("ORG-2", "Kreis Düren", "DE121038462")]))
            .expect("two identified buyers");
        let b = buyer_key(false, 2, &notice(&[("ORG-2", "Kreis Düren", "DE121038462"),
                                              ("ORG-1", "Stadt Aachen", "DE811907980")]))
            .expect("same pair, other order");
        assert_eq!(a, b, "order must not create a second buyer set");

        // The weld's shape: three notices, three disjoint single buyers ⇒ three distinct
        // keys, which is exactly what the gate's `count(DISTINCT buyer_key) >= 3` reads.
        let welded: std::collections::BTreeSet<String> = [
            ("ORG-1", "Klinikum Neumarkt", "DE133517778"),
            ("ORG-1", "Land Baden-Württemberg", "DE811245646"),
            ("ORG-1", "Berufsgenossenschaft Holz und Metall", "DE811188162"),
        ]
        .iter()
        .filter_map(|o| buyer_key(false, 3, &notice(&[*o])))
        .collect();
        assert_eq!(welded.len(), 3, "disjoint buyers must present three distinct sets");

        // A buyer with no plausible identifier falls back to the N2 name key rather than
        // vanishing — otherwise a whole era of identifier-less buyers would read as
        // agreeing with each other, which is the silent direction.
        let nameless = buyer_key(false, 4, &notice(&[("ORG-1", "Gemeinde Alsdorf", "")]))
            .expect("a named buyer with no id still keys");
        assert!(nameless.starts_with("n2:"), "name-key fallback: {nameless}");
        assert_ne!(nameless, a);

        // No buyer named at all is None — distinct from naming an unidentifiable one.
        assert_eq!(buyer_key(false, 5, &notice(&[])), None);
    }

    #[test]
    fn the_de1_fold_is_scoped_to_the_1x_line() {
        assert!(is_de1_profile("eforms:eforms-de-1.0"));
        assert!(is_de1_profile("eforms:eforms-de-1.1"));
        assert!(is_de1_profile("eforms:eforms-de-1.2"));
        // 2.x is a real SDK fork emitting BT-* ids; sdk-* is the EU line.
        assert!(!is_de1_profile("eforms:eforms-de-2.0"));
        assert!(!is_de1_profile("eforms:eforms-de-2.1"));
        assert!(!is_de1_profile("eforms:eforms-sdk-1.7"));
        assert!(!is_de1_profile("eforms:eforms-sdk-0.1"));
    }

    #[test]
    fn only_genuine_uuids_key_an_sdk01_tender() {
        // The real sdk-0.1 CAN's ContractFolderID — a genuine uuid.
        assert!(is_uuid("3d2aac86-4286-4ae2-9bc1-08eb1cc61f80"));
        assert!(is_uuid("  427D4645-163C-419D-93A9-5F5CE05FF9B7  ")); // trimmed, upper hex
        // The numeric channel's local ids are not uuids and stay islands.
        assert!(!is_uuid("25599482"));
        assert!(!is_uuid("LOCAL-12345"));
        assert!(!is_uuid("3d2aac86-4286-4ae2-9bc1-08eb1cc61f8")); // 35 chars
        assert!(!is_uuid("3d2aac8664286-4ae2-9bc1-08eb1cc61f80")); // hyphen misplaced
        assert!(!is_uuid("g3d2aac8-4286-4ae2-9bc1-08eb1cc61f80")); // non-hex
        assert!(!is_uuid(""));
    }

    /// Every key in this test was read off production on 2026-09-08, and the
    /// verdicts are the measured ones — the point of the pair is that shape alone
    /// CANNOT separate a weld from a correct grouping, so this test pins the
    /// pre-filter's reach and nothing more. The buyer test decides.
    #[test]
    fn hand_typed_procedure_keys_are_refused_by_shape() {
        // The three welds. Tender 1 serves three buyers' procurements as one
        // record; 82802 seven; 82804 eleven.
        assert!(is_placeholder_key("00000000-0000-4000-8000-000000000000"));
        assert!(is_placeholder_key("11111111-1111-4111-9111-111111111111"));
        assert!(is_placeholder_key("11111111-2222-4000-8111-123412341235"));
        // Also refused, and CORRECTLY grouped — which is exactly why refusal by
        // shape may not end the decision. Tender 2 (Landkreis Göttingen) is the
        // one the first-block probe missed; 160170 and 778091 have one buyer each.
        assert!(is_placeholder_key("00000001-2023-4000-a000-000000000001"));
        assert!(is_placeholder_key("22222222-2222-4222-8222-222222222222"));
        assert!(is_placeholder_key("aaaaaaaa-aaaa-4aaa-8abc-aaaaaaaaaaaa"));
        // The long-run disjunct earns its place here: seven distinct characters,
        // so the entropy arm alone would let this through.
        assert!(is_placeholder_key("abcdefab-1111-4111-9111-111111111111"));
    }

    #[test]
    fn generated_uuids_are_not_refused() {
        // Real BT-04 keys sampled off prod. Of 796 sampled keys, the entropy arm
        // refused exactly the two genuine placeholders and nothing else.
        assert!(!is_placeholder_key("00003744-bf2c-4313-ab4c-d065bce1ca11"));
        assert!(!is_placeholder_key("00005c05-f443-4f25-a20d-d647f449d456"));
        assert!(!is_placeholder_key("00006a51-12a3-4d77-83b8-5d96d84f34ff"));
        assert!(!is_placeholder_key("3d2aac86-4286-4ae2-9bc1-08eb1cc61f80"));
        // Rostock's incremented family is the exception this test exists to name:
        // hand-typed, placeholder-LOOKING, and each one keys a real distinct
        // procurement. Shape refuses BOTH — the first on entropy (5 distinct
        // characters), the second only on the run arm (7 distinct, but eight `1`s
        // in the opening block). So on this family the shape filter is wrong every
        // time, and every one of them is admitted by the buyer test instead. That
        // is the division of labour, pinned: a later widening of the shape rule
        // cannot fix this by tuning, because there is nothing here to tune towards.
        assert!(is_placeholder_key("11111111-2222-4aaa-8333-444444444444"));
        assert!(is_placeholder_key("11111111-2222-4aaa-8333-444444444450"));
        // Not uuid-shaped at all: never this function's business.
        assert!(!is_placeholder_key("25599482"));
        assert!(!is_placeholder_key("LOCAL-12345"));
        assert!(!is_placeholder_key(""));
    }

    #[test]
    fn the_fixed_nibbles_do_not_pad_the_entropy_count() {
        // The all-zero key spends ONE character. Counting the version `4` and the
        // variant `8` would report three and start the threshold three characters
        // too high on every placeholder.
        let free = uuid_free_nibbles("00000000-0000-4000-8000-000000000000").unwrap();
        assert_eq!(free.len(), 1);
        assert_eq!(free.iter().copied().collect::<Vec<u8>>(), vec![b'0']);
        assert_eq!(uuid_free_nibbles("not-a-uuid"), None);
        // Case folds, so an upper-hex publisher is measured the same way.
        let upper = uuid_free_nibbles("AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA").unwrap();
        assert_eq!(upper.len(), 1);
    }

    #[test]
    fn roles_come_from_the_organization_reference_families() {
        assert_eq!(role_name("OPT-300-Procedure-Buyer").as_deref(), Some("Procedure-Buyer"));
        assert_eq!(role_name("OPT-301-Lot-Mediator").as_deref(), Some("Lot-Mediator"));
        assert_eq!(role_name("BT-137-Lot"), None);
    }

    /// The plausibility gate of docs/research/ted-legacy-mapping.md §6: merging
    /// is only allowed to happen on identifiers that can actually identify.
    #[test]
    fn only_plausible_identifiers_are_allowed_to_merge() {
        let vat = normalise_identifier("NL 8045.95859B01", Some("NLD")).expect("a VAT id");
        assert_eq!(vat.kind, "vat");
        assert_eq!(vat.value, "NL804595859B01");
        // The prefix, not the mention's country, scopes a VAT id.
        assert_eq!(vat.country.as_deref(), Some("NL"));
        // The same id written differently normalises to the same profile.
        assert_eq!(normalise_identifier("nl804595859b01", None), Some(vat));

        let national = normalise_identifier("65993390", Some("CZE")).expect("a registry number");
        assert_eq!(national.kind, "national");
        assert_eq!(national.country.as_deref(), Some("CZE"));

        // Junk from the real corpus, all of which must stay provisional.
        assert_eq!(normalise_identifier("Romania", None), None); // no digit
        assert_eq!(normalise_identifier("n/a", None), None); // too short, no digit
        assert_eq!(normalise_identifier("000000", None), None); // all zeros
        assert_eq!(normalise_identifier("111111", None), None); // filler
        assert_eq!(normalise_identifier("12", None), None); // too short

        // Issue 300 (2026-09-03 top-100 read): a Greek Ε in an identifier is
        // the Latin E — the Greek procurement authority's two rows (311/1079)
        // must key alike.
        assert_eq!(
            normalise_identifier("1000.\u{0395}00961.0001", Some("GR")),
            normalise_identifier("1000.E00961.0001", Some("GR"))
        );
        assert_eq!(normalise_identifier("1000.\u{0395}00961.0001", Some("GR")).unwrap().value, "1000E009610001");
        // A Romanian CUI with a sub-unit suffix is the parent's id; the same
        // string elsewhere is left as written.
        assert_eq!(normalise_identifier("16054368_3", Some("ROU")).unwrap().value, "16054368");
        assert_eq!(normalise_identifier("16054368_3", Some("RO")), normalise_identifier("16054368", Some("RO")));
        assert_eq!(normalise_identifier("16054368_3", Some("HU")).unwrap().value, "160543683");
        // The fold is all-or-nothing per string: a Cyrillic label with one
        // non-lookalike letter keeps the OLD reading (the bare id), a Cyrillic
        // С closing an Irish VAT id folds because nothing non-Latin remains.
        assert_eq!(normalise_identifier("\u{0415}\u{0418}\u{041a} 121663601", Some("BG")).unwrap().value, "121663601");
        assert_eq!(normalise_identifier("IE6609432\u{0421}", None).unwrap().value, "IE6609432C");
        // The before-folds twin reproduces the pre-v2.1 store: the Greek letter
        // dropped, the suffix swallowed — what issue 345's repair compares against.
        assert_eq!(normalise_identifier_before_folds("1000.\u{0395}00961.0001", Some("GR")).unwrap().value, "1000009610001");
        assert_eq!(normalise_identifier_before_folds("16054368_3", Some("RO")).unwrap().value, "160543683");
        assert_eq!(normalise_identifier_before_folds("NL 8045.95859B01", Some("NLD")), normalise_identifier("NL 8045.95859B01", Some("NLD")));
    }

    /// Issue 300 Stage 1 — the v2 gate flip: the MEASURED false-merge classes
    /// lose merge-key status (the mention goes provisional); soft-scheme
    /// checksum failures and the recoverable classes keep it. The exemplar
    /// panel is 300-exemplars.md, every id live-verified on prod.
    #[test]
    fn the_v2_gate_condemns_measured_false_merge_classes_only() {
        // Live: org 15176 (DE123456789, 144+ merged strangers), org 15566
        // (bare 123456789, 425), the NIMAT family (org 211, 794), PL823
        // (org 10583053, 418), eForms technical ids, zero-pad stubs.
        assert_eq!(normalise_identifier("DE123456789", Some("DE")), None);
        assert_eq!(normalise_identifier("123456789", Some("DE")), None);
        assert_eq!(normalise_identifier("NIMAT500", Some("SI")), None);
        assert_eq!(normalise_identifier("ORG-0003", Some("SE")), None);
        assert_eq!(normalise_identifier("BT501", None), None);
        assert_eq!(normalise_identifier("PL823", Some("PL")), None);
        assert_eq!(normalise_identifier("00001", None), None);
        // HARD checksum (standing enablement decision): a real CZ IČO
        // survives, its single-digit mutation is condemned; same for a pure
        // DE VAT mutation that is neither lexicon nor sequence.
        assert!(normalise_identifier("00006947", Some("CZ")).is_some());
        assert_eq!(normalise_identifier("00006948", Some("CZ")), None);
        assert_eq!(normalise_identifier("DE136695977", None), None);
        // SOFT schemes keep typo load merge-usable: PL:nip measured 95.6%,
        // below the HARD bar, so a NIP-checksum-failing value still merges
        // (its failure is evidence, not a rejection).
        assert!(normalise_identifier("5262239326", Some("PL")).is_some());
        // Recoverable classes stay census-only: labelled and compound forms
        // carry REAL ids for Stage 2's splitter; platform hex hashes are
        // merge-inert but not junk.
        assert!(normalise_identifier("REGON470850645", Some("PL")).is_some());
        assert!(normalise_identifier("NIP5262239325REGON010828091", Some("PL")).is_some());
    }

    /// Issue 86: a national register prefix must NOT mint a country. The German
    /// `HRB`/`HRA` numbers begin with letters that spell the ISO code `HR`
    /// (Croatia), so the naive sniffer flagged ~50,700 German companies Croatian.
    #[test]
    fn register_prefixes_are_national_not_a_minted_country() {
        // The canonical bug: a German Handelsregister number under a DE mention.
        let hrb = normalise_identifier("HRB Dresden 4115", Some("DEU")).expect("a register id");
        assert_eq!(hrb.kind, "national");
        assert_eq!(hrb.country.as_deref(), Some("DEU"), "the mention's country, not 'HR'");
        // The whole documented register family stays national, never VAT.
        for (raw, note) in [
            ("HRB 22388", "German HRB"),
            ("HRA2104", "German HRA"),
            ("FN 75109 p", "Austrian Firmenbuch — FN is not a country"),
            ("KRS 0000123456", "Polish KRS"),
            ("NIP 1234567890", "Polish NIP"),
            // A live-measured REGON — the old "12345678" sample is now a
            // condemned ascending run under the issue-300 v2 gate.
            ("REGON 470850645", "Polish REGON"),
            ("OIB 12345678901", "Croatian OIB — starts with 'OI', not a country"),
        ] {
            let id = normalise_identifier(raw, Some("XX")).unwrap_or_else(|| panic!("{note}: {raw}"));
            assert_eq!(id.kind, "national", "{note} ({raw}) must be national");
            assert_eq!(id.country.as_deref(), Some("XX"), "{note}: no minted country");
        }
    }

    /// Issue 86: real VAT ids still parse — including Austrian `ATU…`, whose
    /// third character is a letter. A two-letter prefix outside the VAT-country
    /// set never mints a country.
    ///
    /// ISSUE 325 CHANGED ONE EXPECTATION HERE. This test used to assert that
    /// `EL094019245` lands under country `EL`, "scoped by its prefix" — and
    /// that was right when it was written, before issue 319 established alpha-2
    /// as the country vocabulary and folded the column to it. Storing the raw
    /// prefix is now a re-contamination path for that completed fold: 262 org
    /// rows on prod carry `EL`/`UK`/`XI` and every one is `kind = 'vat'`, i.e.
    /// minted here AFTER the fold. The value keeps its own spelling; the
    /// country column joins the vocabulary.
    #[test]
    fn real_vat_ids_keep_their_country_prefix() {
        // Checksum-clean specimens: the v2 gate (issue 300) condemns
        // ascending-run and HARD-checksum-failing samples, so the fixtures
        // are real-shaped ids (DE is the canonical valid USt-IdNr, EL is
        // OTE's real AFM).
        for (raw, country) in [
            ("ATU37675002", "AT"),
            ("DE136695976", "DE"),
            ("EL094019245", "GR"),
            ("FR12345678901", "FR"),
        ] {
            let id = normalise_identifier(raw, Some("ignored")).expect("a VAT id");
            assert_eq!(id.kind, "vat", "{raw} is a VAT id");
            assert_eq!(id.country.as_deref(), Some(country), "{raw} scoped by its prefix");
        }
        // The VALUE is untouched by the canonicalisation — a published `EL…`
        // id really is spelled that way, and rewriting it would be the value
        // reshaping issue 300 Stage 1 deliberately keeps out of this function.
        let el = normalise_identifier("EL094019245", None).expect("a VAT id");
        assert_eq!(el.value, "EL094019245");
        // A two-letter prefix that is not a VAT country does not mint one — it is
        // a national id scoped by the mention.
        let zz = normalise_identifier("ZZ998877", Some("DEU")).expect("an id");
        assert_eq!(zz.kind, "national");
        assert_eq!(zz.country.as_deref(), Some("DEU"), "ZZ is not a VAT country");
    }

    /// The suffix allowance is a VOCABULARY, and this is what it buys: a German
    /// town appended to a register number is the same LENGTH as a scheme label
    /// and must still be rejected. `ESSEN` and `USTID` are both five letters;
    /// `BONN` and `MWST` are both four. Every value here is a real prod row,
    /// and every one names a German entity filed under someone else's country.
    #[test]
    fn a_place_name_is_not_a_scheme_suffix_however_long_it_is() {
        for raw in [
            "HR100586AGTOSTEDT", // Elbe Kliniken Stade
            "HR224817STUTTGART",
            "HR302325AGJENA",    // Kompaktreinigung Neuhöfer GmbH
            "GB007WIGN001",      // Wigan Council — the run is not even at the back
            "SI2002NUMBER2073",  // East Lancashire Hospitals NHS Trust
        ] {
            let id = normalise_identifier(raw, Some("DE")).expect("still an identifier");
            assert_eq!(id.kind, "national", "{raw} is a register string, not a VAT id");
            assert_eq!(id.country.as_deref(), Some("DE"), "{raw} takes the mention's country");
        }
    }

    /// Issue 328: a publisher label in front of the identifier comes off, and
    /// what is left has to stand on its own.
    ///
    /// Every value here is a real prod row. The point of the test is the pairing:
    /// the labelled form and the bare form must produce the SAME identifier, or
    /// the 3,253 rows fragmented from their twin stay fragmented.
    #[test]
    fn a_label_prefix_resolves_to_the_same_identifier_as_the_bare_value() {
        for (labelled, bare) in [
            ("USTIDDE329214156", "DE329214156"),   // Die Autobahn GmbH des Bundes
            ("USTIDNRDE811335517", "DE811335517"), // Regierung von Oberbayern
            ("UMSATZSTEUERIDDE188369991", "DE188369991"), // TU Dresden
            ("UMSATZSTEUERIDENTIFIKATIONSNUMMERDE198235088", "DE198235088"),
            ("USTIDNRATU37675002", "ATU37675002"), // Austrian, label and all
        ] {
            let l = normalise_identifier(labelled, Some("DE")).expect(labelled);
            let b = normalise_identifier(bare, Some("DE")).expect(bare);
            assert_eq!(
                (l.kind.as_str(), l.country.as_deref(), l.value.as_str()),
                (b.kind.as_str(), b.country.as_deref(), b.value.as_str()),
                "{labelled} must key exactly as {bare}"
            );
        }
    }

    /// THE GUARD: the strip is only allowed to win when its remainder
    /// classifies, and these are the prod shapes where it must not.
    ///
    /// **Every value here has realistic digits, and that is not decoration.**
    /// The first draft of this test used `…123456789` and `…12345678`, which the
    /// v2 gate condemns as ascending runs — so the assertions were reading the
    /// GATE's verdict rather than the strip's. Issue 325's own test carries a
    /// note about exactly this trap and I walked into it again one file over.
    #[test]
    fn a_label_whose_remainder_is_not_an_identifier_is_left_alone() {
        // Three prod rows carry the field name and nothing else. No number
        // anywhere, so there is nothing to recover and nothing to invent.
        assert_eq!(normalise_identifier("UMSATZSTEUERIDENTIFIKATIONSNUMMER", Some("DE")), None);
        // A label followed by letters is not a labelled identifier either.
        assert_eq!(normalise_identifier("USTIDXYZ", Some("DE")), None);
        // And a value with no label is untouched by any of this.
        let plain = normalise_identifier("DE811335517", Some("DE")).expect("a VAT id");
        assert_eq!((plain.kind.as_str(), plain.value.as_str()), ("vat", "DE811335517"));
    }

    /// THE GUARD MUST BE SHARPER THAN "IT PARSES", and this test exists because
    /// the first version was not.
    ///
    /// `normalise_identifier` almost never returns `None` for a string with a
    /// digit in it, because `national()` is a catch-all — so "strip then
    /// re-validate" validated nothing. The issue-328 DRY PLAN is what exposed
    /// it, on prod values, before any write: every value below was in that plan
    /// as a proposed change.
    ///
    /// The rule now is that the remainder must be RECOGNISABLE — it classifies
    /// as a real scheme, or it is pure digits (a registration number that lost
    /// its label). That makes the vocabulary's gaps SAFE: an unlisted variant
    /// leaves the row alone instead of mangling it, which is what matters for a
    /// list read off a corpus that keeps growing.
    #[test]
    fn a_leftover_label_fragment_never_becomes_an_identifier() {
        // The one that made the point. The vocabulary carries
        // `UMSATZSTEUERIDENTIFIKATIONSNUMMER` and `UMSATZSTEUERID`; this value
        // has `…SNR`, so a SHORTER entry matched and left a fragment that
        // classified happily as `national`.
        for raw in [
            "UMSATZSTEUERIDENTIFIKATIONSNRENTEGAPLUSGMBHDE813810149",
            // `HANDELSREGISTERNRHRB64128` used to be here. It is no longer a
            // fragment case and now strips correctly — see
            // `the_handelsregister_strip_reunites_the_labelled_and_bare_forms`.
            "HANDELSREGISTERARNHEM09155985",
            "HANDELSREGISTERAMTSGERICHTESSENHRB11082",
            "STNRDE811183963REGNRAMTSGERICHTKLNHRB2130",
            "STNR16227103384FINANZAMTGERA",
        ] {
            let id = normalise_identifier(raw, Some("DE")).expect("still an identifier");
            assert_eq!(
                id.value, raw,
                "{raw} must keep the value the publisher wrote — a leftover \
                 label fragment is not an identifier"
            );
        }
    }

    /// …and the strips that ARE right still happen. Two shapes qualify: a real
    /// scheme, and pure digits.
    #[test]
    fn a_recognisable_remainder_is_still_accepted() {
        // A real scheme.
        let v = normalise_identifier("USTIDDE329214156", Some("DE")).expect("a VAT id");
        assert_eq!((v.kind.as_str(), v.value.as_str()), ("vat", "DE329214156"));
        // Pure digits: a Steuernummer that lost its label. Prod values.
        for (raw, want) in [
            ("STNR1529086043", "1529086043"),
            ("STEUERNUMMER809033537", "809033537"),
            ("USTID308958755", "308958755"),
            ("USTIDNR194657063", "194657063"),
        ] {
            let id = normalise_identifier(raw, Some("DE")).expect(raw);
            assert_eq!(id.value, want, "{raw}");
            assert_eq!(id.kind, "national", "a bare number stays national");
        }
    }

    /// Issue 359: the Polish, Italian and Spanish field names key exactly as
    /// the bare value — the 357 campaign met these on rows whose twin already
    /// stood under the bare number. Every value is a real prod row.
    /// The UK's two registers survive normalisation as `national` identifiers
    /// with their scheme prefix intact, which is what the crosswalk's GB arm
    /// reads (issue 342). Both are checked here rather than assumed, because
    /// two gates could plausibly have eaten them and neither does: the VAT
    /// sniffer declines because `COHSC`/`PPONPBZB` exceed its letter-run
    /// bound, and `idgate::condemns` declines because its `letter_run` census
    /// flag is deliberately not one of the condemning conditions.
    #[test]
    fn the_uk_registers_normalise_to_national_identifiers_with_their_prefix() {
        for (raw, value) in [
            ("GB-COH-SC305103", "GBCOHSC305103"),
            ("GB-COH-07495895", "GBCOH07495895"),
            ("GB-PPON-PBZB-4962-TVLR", "GBPPONPBZB4962TVLR"),
        ] {
            let id = normalise_identifier(raw, Some("GB"))
                .unwrap_or_else(|| panic!("{raw} must survive as an identifier"));
            assert_eq!(
                (id.country.as_deref(), id.kind.as_str(), id.value.as_str()),
                (Some("GB"), "national", value),
                "{raw}"
            );
        }
        // A real GB VAT still reads as a vat, so the crosswalk's GB arm has
        // something to guard against.
        let vat = normalise_identifier("GB553298332", Some("GB")).expect("a GB vat");
        assert_eq!((vat.kind.as_str(), vat.value.as_str()), ("vat", "GB553298332"));
    }

    #[test]
    fn a_non_german_label_prefix_resolves_to_the_same_identifier_as_the_bare_value() {
        for (labelled, bare, country, kind) in [
            ("NIP1070000916", "1070000916", "PL", "national"),     // SAFEGE, Polish branch
            ("NIPNUMER1070000916", "1070000916", "PL", "national"),
            ("NUMERNIPDE312308370", "DE312308370", "DE", "vat"),   // Acandis GmbH
            ("NIPDE312308370", "DE312308370", "PL", "vat"),        // the label on a PL row, the id German
            ("PIVA10548370963", "10548370963", "IT", "national"),  // Lloyd's Insurance Company, IT branch
            ("CFEPIVA10548370963", "10548370963", "IT", "national"),
            ("CF97819940152", "97819940152", "IT", "national"),
            ("CIFA48283964", "A48283964", "ES", "national"),       // IDOM — the CIF shape, not digits
            ("NIPA41015322", "A41015322", "PL", "national"),       // Ayesa: a Spanish CIF under a Polish label
            ("VATIDGB287249363", "GB287249363", "GB", "vat"),      // Therakos EMEA
        ] {
            let l = normalise_identifier(labelled, Some(country)).expect(labelled);
            let b = normalise_identifier(bare, Some(country)).expect(bare);
            assert_eq!(
                (l.kind.as_str(), l.country.as_deref(), l.value.as_str()),
                (b.kind.as_str(), b.country.as_deref(), b.value.as_str()),
                "{labelled} must key exactly as {bare}"
            );
            assert_eq!(l.kind, kind, "{labelled}");
            assert_eq!(l.value, bare, "{labelled}");
        }
    }

    /// …and the guard still holds where it must: a compound field holding TWO
    /// Polish ids is not a labelled identifier (the strip leaves letters behind),
    /// a label followed by a word is nothing, and a bare field name is nothing.
    /// The compound is the splitter's (B8 rule 4), and it keeps the value the
    /// publisher wrote.
    #[test]
    fn the_polish_compound_field_and_the_bare_label_are_left_alone() {
        let compound = normalise_identifier("NIP1070000916REGON015259640", Some("PL"))
            .expect("still an identifier — census-only class");
        assert_eq!(compound.value, "NIP1070000916REGON015259640");
        assert_eq!(normalise_identifier("NIPXYZ", Some("PL")), None);
        assert_eq!(normalise_identifier("CIFEMPRESA", Some("ES")), None);
    }

    /// The shape test behind the CIF acceptance, pinned on both sides.
    #[test]
    fn the_spanish_cif_shape_is_nine_characters_and_nothing_else() {
        for ok in ["A48283964", "B82351800", "S2800568D", "12345678Z", "X1234567L", "L01280796"] {
            assert!(es_cif_or_nif_shaped(ok), "{ok}");
        }
        for no in ["NRHRB64128", "ARNHEM09155985", "DE312308370", "1070000916", "A4828396", "A482839644", "ABCDEFGHI"] {
            assert!(!es_cif_or_nif_shaped(no), "{no}");
        }
    }

    /// The `HANDELSREGISTER` entry is currently INERT, and that is worth pinning
    /// so nobody "fixes" it into working.
    ///
    /// The Handelsregister strip, and the fixture that made it look inert
    /// (issue 374, correcting this test's own earlier conclusion).
    ///
    /// This test used to assert that both `HANDELSREGISTERHRB12345` and bare
    /// `HRB12345` return `None`, and concluded from that pair that "the v2 gate
    /// condemns `HRB…` values on their own account… the class is simply out of
    /// reach". That conclusion was wrong, and the reason is the fixture: the
    /// digits `12345` are an ascending run, so `suspicious_digit_run` condemns
    /// BOTH forms whatever the strip does. The test was measuring the sequence
    /// rule and reading the answer as a fact about `HRB`.
    ///
    /// A realistic register number tells the real story: `HRB93017` is not
    /// condemned at all, and before issue 374 `HANDELSREGISTERHRB93017`
    /// normalised to ITSELF — the strip was computed, then thrown away by the
    /// `recognisable` guard, because `HRB93017` is neither pure digits nor a
    /// Spanish CIF. So the two forms were separate live merge keys and the same
    /// company published both ways got two org rows.
    #[test]
    fn the_handelsregister_strip_reunites_the_labelled_and_bare_forms() {
        assert_eq!(
            crate::countries::label_prefix_stripped("HANDELSREGISTERHRB93017"),
            Some("HRB93017"),
            "the field name comes off and the register division stays"
        );
        let labelled = normalise_identifier("HANDELSREGISTERHRB93017", Some("DE"));
        let bare = normalise_identifier("HRB93017", Some("DE"));
        assert_eq!(
            labelled.as_ref().map(|i| i.value.as_str()),
            Some("HRB93017"),
            "the labelled form must resolve to the register number, not to itself",
        );
        assert_eq!(labelled, bare, "both spellings are one identifier");

        // Every German register division that the corpus actually carries in
        // bare form (HRB 11,439 rows, HRA 2,132, VR 326, PR 258, GnR 22 on prod
        // 2026-09-09) — so a labelled row stripping to one of these is a real
        // reunion, not a guess.
        for div in ["HRB", "HRA", "GNR", "VR", "PR"] {
            let raw = format!("HANDELSREGISTER{div}93017");
            assert_eq!(
                normalise_identifier(&raw, Some("DE")).map(|i| i.value),
                Some(format!("{div}93017")),
                "{raw} strips to its register division",
            );
        }

        // `HANDELSREGISTERNRHRB64128` reads "Handelsregister-Nr. HRB 64128", so
        // `HRB64128` is the right answer — and this case moved HERE from
        // `a_leftover_label_fragment_never_becomes_an_identifier`, which is
        // worth explaining rather than quietly re-pointing.
        //
        // That test's comment cited this value as producing the fragment
        // `NRHRB64128`, which was true when the vocabulary's longest match was
        // `HANDELSREGISTER` (15). The list has since gained
        // `HANDELSREGISTERNR` (17), so longest-match consumes the label's `NR`
        // too and the remainder has been the clean register value for a while —
        // the row only kept its labelled form because the `recognisable` guard
        // was still rejecting `HRB…`. Issue 374 removes that last obstacle, so
        // the old assertion was protecting a fragment the code no longer makes.
        assert_eq!(
            normalise_identifier("HANDELSREGISTERNRHRB64128", Some("DE")).map(|i| i.value),
            Some("HRB64128".to_owned()),
        );

        // The guard's original job still holds: a LEFTOVER label fragment must
        // not pass. Neither of these starts with an anchored register division,
        // and the second is a real prod value from the issue-328 dry plan.
        for raw in ["NRHRB64128", "ARNHEM09155985"] {
            let id = normalise_identifier(raw, Some("DE")).expect("kept as written");
            assert_eq!(id.value, raw, "{raw} is leftover label text, not an id");
        }

        // And the ascending-digit case that misled the old test still refuses,
        // for the reason it always did — the sequence rule, not the strip.
        assert_eq!(normalise_identifier("HANDELSREGISTERHRB12345", Some("DE")), None);
        assert_eq!(normalise_identifier("HRB12345", Some("DE")), None);
    }

    /// Issue 374 unit 1: three label prefixes whose stripped remainder is pure
    /// digits, so the existing `recognisable` guard already admits them — they
    /// were simply missing from the vocabulary. 165 `CVRNR` rows, 540 `SIRET`,
    /// 31 `REGISTRIERUNGSNUMMER` on prod 2026-09-09.
    ///
    /// Digits chosen to be realistic: an ascending run is condemned by the
    /// `sequence` rule regardless of the strip, so it would assert nothing.
    #[test]
    fn a_registry_number_under_its_registers_name_resolves_to_the_bare_number() {
        for (raw, bare, country) in [
            ("CVRNR29189498", "29189498", "DK"),
            ("SIRET78467169500087", "78467169500087", "FR"),
            ("REGISTRIERUNGSNUMMER84067219", "84067219", "DE"),
        ] {
            let labelled = normalise_identifier(raw, Some(country));
            assert_eq!(
                labelled.as_ref().map(|i| i.value.as_str()),
                Some(bare),
                "{raw} must resolve to the number the publisher meant",
            );
            assert_eq!(labelled, normalise_identifier(bare, Some(country)), "{raw} == {bare}");
        }
    }

    /// Issue 325: a word whose first two letters spell a VAT country must not
    /// mint that country.
    ///
    /// Every specimen below is a REAL prod value, and the country beside it is
    /// the one the row actually carried because of it. The mechanism is the old
    /// guard's "a digit ANYWHERE in the rest", which the register-prefix arm
    /// eleven lines above had already learned not to do: a two-letter tag must
    /// be followed by the thing it tags. 4,206 rows, and the mentions on 93.4%
    /// of them unanimously name a different country.
    #[test]
    fn a_word_that_starts_with_a_country_code_does_not_mint_that_country() {
        for (raw, minted) in [
            // Digits are non-sequential throughout: the v2 gate (issue 300
            // Stage 1) condemns ascending runs, and a fixture it refuses
            // wholesale tests the gate rather than this arm. Two synthetic
            // values in the first draft of this test did exactly that.
            ("CHARITYNO298028", "CH"),          // a British charity
            ("CHARITYNUMBER1040303", "CH"),     // Citizens Advice Wandsworth
            // `BERICHTSEINHEITID00002636` (traffiQ, Frankfurt) used to sit here as
            // a third "BE" specimen. Issue 365 unit 3 measured the Berichtseinheit
            // class and refused it as a merge key outright — one reporting unit
            // carried 47 distinct mention names — so it is now gated away before
            // country election ever runs, which is a different question from the
            // one this test asks. `BERLINCHARLOTTENBURG93627` and the `BE2A…` GUID
            // below still cover the BE prefix twice over.
            ("BERLINCHARLOTTENBURG93627", "BE"), // the Amtsgericht
            ("FINANZAMTBIELEFELD34959", "FI"),  // a German tax office
            ("FIRMENBUCHNUMMER441612F", "FI"),  // an Austrian Firmenbuch number
            ("FRANKFURTHRB105754", "FR"),       // a German company
            ("DECRETODIRIGENZIALE1486762017", "DE"), // an Italian decree
            ("LIDERKONSORCJUM9661386113", "LI"), // a Polish consortium
            ("ESTRADADOBAIRROSN2600614", "ES"), // a Portuguese street address
            ("ATTOGE13295DEL28102022", "AT"),   // an Italian administrative act
            ("BGLFRZ76T09F712E", "BG"),         // an Italian codice fiscale
            ("FRRSFN75A45F839O", "FR"),         // another codice fiscale
            ("EEE9F40A5D2242C8825F273DE191CF69", "EE"), // a 32-char platform GUID
            ("BE2A168C8910492EB06E6644D5F75B0B", "BE"), // ditto, a Swiss canton's
        ] {
            let id = normalise_identifier(raw, Some("PL"))
                .unwrap_or_else(|| panic!("{raw} ({minted}) was gated away entirely"));
            assert_eq!(
                id.kind, "national",
                "{raw} is not a VAT number — it merely begins with {minted}"
            );
            assert_eq!(
                id.country.as_deref(),
                Some("PL"),
                "{raw} must take the MENTION's country, not {minted} out of its own letters"
            );
            assert_eq!(id.value, raw, "and the published value is never reshaped");
        }
    }

    /// The other half of issue 325's fix: the tightening must not cost a single
    /// real VAT id. Every European scheme that puts something other than a
    /// digit right after the country code is here, because those are the ones a
    /// careless "must be followed by a digit" rule would have broken.
    ///
    /// EVERY VALUE IS A REAL-SHAPED ONE, and that is not decoration. Five
    /// fixtures in this test's first draft were swallowed whole by the v2 gate
    /// (issue 300 Stage 1) — `ESX1234567X` and `BERLINCHARLOTTENBURG12345` for
    /// ascending digit runs, `FRXX999999999` and `GBGD001` as filler and
    /// short-VAT stubs, `SE556602998601` for a Luhn that does not close
    /// (SE:vat is a HARD scheme). Each returned `None`, which would have made
    /// this test pass or fail on the GATE's behaviour rather than on this arm's.
    /// A synthetic VAT id is nearly always a gate-refused one, so use a real
    /// registrant: the Swedish body here is Philips AB's orgnr, read off prod.
    #[test]
    fn the_tightened_vat_arm_still_admits_every_real_scheme_shape() {
        // Not here, deliberately: `GBGD001` and `GBHA599`, the UK departmental
        // and health-authority forms. Their two-letter tag is exactly what the
        // letter-run bound has to admit (2 < 3, and `FRAB404833048` exercises
        // that), but they carry three digits and the v2 gate's short-VAT-stub
        // rule refuses them as merge keys — a decision that predates issue 325
        // and belongs to `idgate::condemns`. A fixture the gate swallows would
        // test the gate, not this arm.
        for (raw, country) in [
            ("ATU37675002", "AT"),      // Austria's fixed `U`
            ("ESX3873152T", "ES"),      // a Spanish CIF's leading letter
            ("ESA58818501", "ES"),
            ("FRAB404833048", "FR"),    // two alphabetic check characters
            ("FRK7399859412", "FR"),
            ("NL804595859B01", "NL"),   // the `B` sub-number
            ("IE9825613N", "IE"),       // the Irish trailing letter
            ("IE8Z49289F", "IE"),       // and the older mid-string form
            ("GB553298332", "GB"),
            ("CY10259033P", "CY"),      // the Cypriot trailing letter
            ("SE556105261301", "SE"),   // the longest real body: twelve
            // Norway publishes the scheme name IN the id. Three letters, at
            // the back — the shape the first draft of the tightening rejected
            // on 331 real prod rows.
            ("NO999665624MVA", "NO"),
            ("NO966041056VAT", "NO"),
            ("CHE106094419MWST", "CH"), // Switzerland writes the word too
            ("CHE113202476MWST", "CH"),
            ("DE122624631USTID", "DE"), // and Germany appends `USt-IdNr`
            ("IE9Z17184AVAT", "IE"),    // an Irish check letter, then `VAT`
            ("XI553298332", "XI"),      // Northern Ireland
            ("DE136695976", "DE"),
        ] {
            let id = normalise_identifier(raw, Some("ZZ"))
                .unwrap_or_else(|| panic!("{raw} ({country}) was gated away entirely"));
            assert_eq!(id.kind, "vat", "{raw} is a real VAT shape and must stay one");
            assert_eq!(
                id.country.as_deref(),
                Some(canonical_country(country).as_str()),
                "{raw} is scoped by its own prefix, canonicalised"
            );
        }
    }

    /// Issue 363: the Finnish field names come off and the remainder is
    /// re-validated as the Y-tunnus it is — a HARD checksum, so a mistyped
    /// remainder keeps the row as published; and the label is country-agnostic
    /// while the bare `Y` shape never touches a Spanish NIE.
    #[test]
    fn finnish_labels_come_off_and_the_y_tunnus_is_checked() {
        for (raw, country) in [("YTUNNUS01274855", "FI"), ("Y01274855", "FI"), ("FONR01446821", "AX"),
                               ("FONUMMER01446821", "FI"), ("BUSINESSID01274855", "FI"), ("Y-tunnus: 0127485-5", "FI")] {
            let id = normalise_identifier(raw, Some(country)).unwrap();
            assert_eq!(id.country.as_deref(), Some("FI"), "{raw}");
            assert_eq!(id.kind, "national", "{raw}");
            assert!(id.value == "01274855" || id.value == "01446821", "{raw} → {}", id.value);
        }
        // A remainder that fails the FI checksum is refused, so the row keeps
        // what the publisher wrote rather than a mangled id.
        for raw in ["YTUNNUS01274856", "Y01274856"] {
            let id = normalise_identifier(raw, Some("FI")).unwrap();
            assert_eq!(id.value, raw, "a bad remainder leaves the value as published");
        }
        // An ES NIE keeps its Y; a Y-led value under FI that is not the shape too.
        assert_eq!(normalise_identifier("Y7395817K", Some("ES")).unwrap().value, "Y7395817K");
        assert_eq!(normalise_identifier("YT22493", Some("FI")).unwrap().value, "YT22493");
    }

    /// Issue 358: a national id published under an overseas-department or
    /// Åland code scopes to the register's jurisdiction, so the org row it
    /// mints joins the parent's series — and is gated as that series (the
    /// FI checksum applies to a Y-tunnus under `AX`). A code with a register
    /// of its own keeps its code.
    #[test]
    fn a_regional_code_scopes_its_identifier_to_the_register() {
        for code in ["GP", "MQ", "GF", "RE", "YT", "PM", "BL", "MF", "WF"] {
            let id = normalise_identifier("552081317", Some(code)).unwrap();
            assert_eq!(id.country.as_deref(), Some("FR"), "{code} → FR");
            assert_eq!(id.kind, "national", "{code}: a bare SIREN stays a national id");
            assert_eq!(id.value, "552081317");
        }
        let ax = normalise_identifier("0100315-8", Some("AX")).unwrap();
        assert_eq!((ax.country.as_deref(), ax.kind.as_str()), (Some("FI"), "national"));
        assert!(
            normalise_identifier("0100315-9", Some("AX")).is_none(),
            "a Y-tunnus under AX that fails the FI checksum is refused like one under FI"
        );
        assert_eq!(normalise_identifier("25313763", Some("GL")).unwrap().country.as_deref(), Some("DK"));
        assert_eq!(normalise_identifier("923609016", Some("SJ")).unwrap().country.as_deref(), Some("NO"));
        for code in ["NC", "PF", "FO", "AW", "CW", "SX", "BQ", "DE"] {
            let id = normalise_identifier("552081317", Some(code)).unwrap();
            assert_eq!(id.country.as_deref(), Some(code), "{code} keeps its own register");
        }
        // The VAT arm is unaffected: a prefixed VAT id scopes by its prefix.
        let vat = normalise_identifier("FR40303265045", Some("RE")).unwrap();
        assert_eq!((vat.country.as_deref(), vat.kind.as_str()), (Some("FR"), "vat"));
    }

    /// Issue 48: country codes converge to one canonical alpha-2 vocabulary, so a
    /// filter or aggregation no longer splits the same country across codings.
    #[test]
    fn country_codes_canonicalise_to_alpha2() {
        // The dominant alpha-3 class folds to alpha-2.
        for (raw, want) in
            [("DEU", "DE"), ("FRA", "FR"), ("ESP", "ES"), ("ROU", "RO"), ("GBR", "GB"), ("USA", "US")]
        {
            assert_eq!(canonical_country(raw), want, "{raw} → {want}");
        }
        // TED's non-ISO forms.
        assert_eq!(canonical_country("UK"), "GB", "TED UK is ISO GB");
        assert_eq!(canonical_country("EL"), "GR", "eurostat EL is ISO GR");
        // Already alpha-2, and case/whitespace tolerance.
        assert_eq!(canonical_country("DE"), "DE");
        assert_eq!(canonical_country(" fr "), "FR");
        // An unrecognised code is preserved, not dropped.
        assert_eq!(canonical_country("ZZ"), "ZZ");

        // Issue 319: the fold is the WHOLE ISO 3166-1 table now, not a
        // hand-picked EU-plus-favourites list. These are the codes that were
        // actually sitting unfolded in the corpus.
        for (raw, want) in [
            ("GRL", "GL"), // beside 23 live GL rows — the split that found this
            ("MCO", "MC"),
            ("ARE", "AE"),
            ("ZAF", "ZA"),
            ("SGP", "SG"),
            ("GEO", "GE"),
            ("HKG", "HK"),
            ("NZL", "NZ"),
        ] {
            assert_eq!(canonical_country(raw), want, "{raw} → {want}");
        }
        // THE TRAP, pinned so nobody ever "simplifies" this to a prefix cut:
        // an alpha-3's first two letters are a DIFFERENT country's alpha-2.
        assert_eq!(canonical_country("SEN"), "SN", "Senegal, not Sweden");
        assert_eq!(canonical_country("BEN"), "BJ", "Benin, not Belgium");
        assert_eq!(canonical_country("CHN"), "CN", "China, not Switzerland");
        assert_ne!(canonical_country("SEN"), "SE");
        assert_ne!(canonical_country("BEN"), "BE");
        // Kosovo is user-assigned — ISO 3166-1 does not list it, so the fold
        // keeps its own arm and the generated table cannot silently drop it.
        assert_eq!(canonical_country("XKX"), "XK");
        // A country NAME folds too: the corpus holds one row spelled this way.
        assert_eq!(canonical_country("LUXEMBOURG"), "LU");
        assert_eq!(canonical_country("luxembourg"), "LU");
        assert_eq!(canonical_country("Netherlands"), "NL");
        // Accented names fold, and — the part that matters for the backfill —
        // do not come back half-uppercased.
        assert_eq!(canonical_country("Curaçao"), "CW");
        assert_eq!(canonical_country("Côte d'Ivoire"), "CI");
        assert_eq!(canonical_country("Réunion"), "RE");
        assert_eq!(canonical_country("Åland Islands"), "AX");
        assert_eq!(canonical_country("Türkiye"), "TR");
        // Junk that is neither: preserved, so it stays visible to the census
        // rather than being laundered into a plausible-looking code. '1A0'
        // is a real value on 3 prod rows.
        assert_eq!(canonical_country("1A0"), "1A0");
        assert_eq!(canonical_country("NOTACOUNTRY"), "NOTACOUNTRY");
    }

    /// Issue 319: the generated table must stay a bijection-ish mapping —
    /// every alpha-3 distinct, every target a plausible alpha-2 — because it
    /// is generated and nobody reads 249 lines in review.
    #[test]
    fn the_generated_country_table_is_well_formed() {
        use crate::countries::{ALPHA3_TO_ALPHA2, NAME_TO_ALPHA2};
        assert_eq!(ALPHA3_TO_ALPHA2.len(), 249, "ISO 3166-1 has 249 assignments");
        let mut codes: Vec<&str> = ALPHA3_TO_ALPHA2.iter().map(|(a, _)| *a).collect();
        codes.sort_unstable();
        let before = codes.len();
        codes.dedup();
        assert_eq!(codes.len(), before, "no alpha-3 appears twice");
        for (a3, a2) in ALPHA3_TO_ALPHA2 {
            assert_eq!(a3.len(), 3, "{a3}");
            assert_eq!(a2.len(), 2, "{a3} → {a2}");
            assert!(a3.chars().all(|c| c.is_ascii_uppercase()), "{a3}");
            assert!(a2.chars().all(|c| c.is_ascii_uppercase()), "{a2}");
        }
        for (name, a2) in NAME_TO_ALPHA2 {
            assert!(name.len() > 3, "a name shorter than 4 would shadow a code: {name}");
            assert_eq!(a2.len(), 2, "{name} → {a2}");
            // UNICODE uppercase, because eight entries are accented and the
            // lookup upper-cases its input the same way. An ASCII-only
            // assertion here would have passed while CURAÇAO stayed
            // unreachable — which is exactly what happened (panel catch).
            assert_eq!(*name, name.to_uppercase(), "the table must be pre-uppercased: {name}");
        }
        assert!(
            NAME_TO_ALPHA2.iter().any(|(n, _)| !n.is_ascii()),
            "the accented entries are the ones that regress silently; if this ever \
             becomes false the casing assertion above has stopped testing anything"
        );
    }

    #[test]
    fn supersession_replaces_a_field_wholesale_and_leaves_others_alone() {
        let text = |field: &str, lang: &str, value: &str| Fact::Text {
            field: field.into(),
            lang: Some(lang.into()),
            value: value.into(),
        };
        let mut carried: BTreeSet<Fact> =
            [text("title", "ENG", "Roof works"), text("title", "DEU", "Dacharbeiten"),
             text("description", "ENG", "unchanged")]
                .into_iter()
                .collect();

        // A corrigendum republishing only the English title drops the stale
        // German one — they are one field — but not the description.
        supersede(&mut carried, &[text("title", "ENG", "Roof works, revised")].into_iter().collect());

        assert_eq!(carried.len(), 2);
        assert!(carried.contains(&text("title", "ENG", "Roof works, revised")));
        assert!(carried.contains(&text("description", "ENG", "unchanged")));
    }
}
