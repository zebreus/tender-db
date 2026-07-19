//! The era checklist made executable: one recorded decision per TED_EXPORT
//! element name.
//!
//! The universe is the vendored `sdk/r209-inventory.json`, extracted from the
//! Publications Office XSDs mirrored on the VPS (`/opt/tender-db/ted-xsd/`):
//! the union of the R2.0.9 S01 and S05 revisions (F01–F25 + MOVE + the
//! TED_EXPORT container) plus everything reachable from the four defence form
//! roots of the R2.0.8.S05 set — defence notices never migrated to R2.0.9 and
//! ride inside R2.0.9-era packages through 2024. The completeness test walks
//! that inventory and fails on any element without a rule here, and on any
//! rule naming an element the XSDs do not declare (ADR-0002, era-scoped).
//!
//! Unlike eForms, R2.0.9 element names are essentially globally unique, so the
//! registry is keyed by local name with a handful of parent-context overrides
//! ([`CONTEXT`]) where one name means two things.

use std::collections::HashMap;
use std::sync::OnceLock;

/// What the walker does with one element (and, transitively, how its
/// attributes and text are claimed). Every rule except [`Rule::Text`],
/// [`Rule::Ignore`] and [`Rule::DateParts`] recurses into element children.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rule {
    /// Structural container: no value of its own, children carry the data.
    Group,
    /// Presence is the datum (`PT_OPEN`, `NO_LOT_DIVISION`): Integer 1.
    /// Children (e.g. negotiated-procedure justifications) are still walked.
    Marker,
    /// Claimed, deliberately not stored; the reason is the documentation
    /// ADR-0004 demands. Covers the whole subtree.
    Ignore(&'static str),
    /// Opens a synthesized section (legacy notices publish no section ids).
    Section(Kind),
    /// An inline party address block: opens an `ORG-<n>` section and records
    /// the role as an id-ref on the enclosing section (the eForms
    /// OPT-300-style pattern).
    Org,
    /// Free text (`<P>` paragraphs, mixed btx content, plain strings) in the
    /// current form language. Consumes the whole subtree, formatting markup
    /// (`FT`, lists, tables) included.
    Text,
    /// Code carried by the first of these attributes that is present; the
    /// element text is the redundant human-readable label in the form
    /// language (measured in ted-legacy-mapping.md §2.1) and is consumed.
    CodeAttr(&'static [&'static str]),
    /// Code carried by the element text (`LG_ORIG`, `HEADING`).
    CodeText,
    /// CPV classification, code from `@CODE` or the element text.
    Cpv,
    /// NUTS classification, code from `@CODE` or the element text.
    Nuts,
    /// Money: `@FMTVAL` if present (defence), else the element text; currency
    /// from `@CURRENCY` here or on the nearest ancestor that declared one.
    Amount,
    /// Decimal with a unit: from `@TYPE` (`DURATION TYPE="MONTH"`) or fixed.
    Number(Unit),
    Integer,
    /// `2019-02-01` (forms) or `20190102` (coded section); merges the paired
    /// sibling time element (`DATE_RECEIPT_TENDERS`/`TIME_RECEIPT_TENDERS`).
    Date,
    /// Wall-clock `HH:MM` whose paired date sibling stores the instant; a
    /// time without a paired date is stored on the epoch day.
    Time,
    /// `20190207 11:00` in one value (`DT_DATE_FOR_SUBMISSION`).
    DateTime,
    /// Defence-style date from `DAY`/`MONTH`/`YEAR` (+ optional `TIME`)
    /// children, which it consumes.
    DateParts,
    /// `DAY`/`MONTH`/`YEAR`: only valid inside [`Rule::DateParts`], which
    /// consumes them. Standalone occurrences are unclaimed content.
    DatePart,
    Id(IdKind),
    /// A form copy root (`F02_2014`, `CONTRACT_AWARD_DEFENCE`): dispatched by
    /// the FORM_SECTION handler (original/translation policy), never by the
    /// generic walker.
    FormRoot,
}

/// Synthesized-section kinds. Ids are `<prefix>-<n>` in document order per
/// notice (`LOT-1`, `RES-2`, …) — legacy notices have no eForms-style section
/// registry, so these are the deterministic addresses the projection joins on
/// (`LOT_NO`/`CONTRACT_NO` values live *inside* the sections as ids).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// `OBJECT_DESCR` and the defence lot annexes.
    Lot,
    /// `AWARD_CONTRACT` / `AWARD_OF_CONTRACT_DEFENCE` / design-contest
    /// `RESULTS` — one award block, the eForms LotResult analogue.
    LotResult,
    /// One F14 `CHANGE` block: WHERE + typed OLD_VALUE/NEW_VALUE.
    Change,
    /// The F20 `MODIFICATIONS_CONTRACT` block.
    Modification,
}

impl Kind {
    pub fn prefix(self) -> &'static str {
        match self {
            Kind::Lot => "LOT",
            Kind::LotResult => "RES",
            Kind::Change => "CHG",
            Kind::Modification => "MOD",
        }
    }
    pub fn kind(self) -> &'static str {
        match self {
            Kind::Lot => "Lot",
            Kind::LotResult => "LotResult",
            Kind::Change => "Change",
            Kind::Modification => "Modification",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unit {
    /// Unit named by the element's `@TYPE` attribute (MONTH | DAY).
    FromTypeAttr,
    Fixed(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IdKind {
    Plain,
    /// A reference to another OJS publication — a Tender chain edge.
    Ref,
    /// A NATIONALID: kept raw here, normalized by `crate::orgid` at
    /// projection time (16% of filled values are junk — research §6).
    National,
}

/// Parent-context overrides for the few names that mean different things in
/// different places.
const CONTEXT: &[(&str, &str, Rule)] = &[
    // NO_DOC_OJS is the notice's own OJS number everywhere except inside
    // REF_NOTICE, where it is the chain edge to a previous publication.
    ("REF_NOTICE", "NO_DOC_OJS", Rule::Id(IdKind::Ref)),
    // MIN/MAX are money range bounds except in the subcontracting share
    // percentage range.
    ("PCT_RANGE_SHARE_SUBCONTRACTING", "MIN", Rule::Number(Unit::Fixed("PCT"))),
    ("PCT_RANGE_SHARE_SUBCONTRACTING", "MAX", Rule::Number(Unit::Fixed("PCT"))),
    // The S01/S02 revisions wrap an award's amounts in `<VALUE PUBLICATION>`
    // where S03+ writes `<VALUES>` — measured on the 2017/2018 dailies.
    ("AWARDED_CONTRACT", "VALUE", Rule::Group),
];

/// The rule for an element, given its parent's local name.
pub fn rule(parent: &str, name: &str) -> Option<Rule> {
    if let Some(&(_, _, rule)) = CONTEXT.iter().find(|&&(p, n, _)| p == parent && n == name) {
        return Some(rule);
    }
    table().get(name).copied()
}

/// Every element name the registry decides — for the completeness test.
pub fn decided_names() -> impl Iterator<Item = &'static str> {
    table().keys().copied()
}

fn table() -> &'static HashMap<&'static str, Rule> {
    static TABLE: OnceLock<HashMap<&'static str, Rule>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut map = HashMap::new();
        for &(rule, names) in GROUPED {
            for &name in names {
                let clash = map.insert(name, rule);
                assert!(clash.is_none(), "duplicate rule for {name}");
            }
        }
        map
    })
}

/// The registry, grouped by decision. Ordering within a group is alphabetic;
/// the completeness test cross-checks every name against the XSD inventory.
#[rustfmt::skip]
static GROUPED: &[(Rule, &[&str])] = &[
    // ------------------------------------------------------------ sections
    (Rule::Section(Kind::Lot), &["F17_ANNEX_B", "LOT_PRIOR_INFORMATION", "OBJECT_DESCR"]),
    (Rule::Section(Kind::LotResult), &["AWARD_CONTRACT", "AWARD_OF_CONTRACT_DEFENCE", "RESULTS"]),
    (Rule::Section(Kind::Change), &["CHANGE"]),
    (Rule::Section(Kind::Modification), &["MODIFICATIONS_CONTRACT"]),
    (Rule::Org, &[
        "ADDRESS_CONTRACTING_BODY", "ADDRESS_CONTRACTING_BODY_ADDITIONAL", "ADDRESS_CONTRACTOR",
        "ADDRESS_FURTHER_INFO", "ADDRESS_MEDIATION_BODY", "ADDRESS_PARTICIPATION", "ADDRESS_PARTY",
        "ADDRESS_REVIEW_BODY", "ADDRESS_REVIEW_INFO", "ADDRESS_WINNER",
        "CA_CE_CONCESSIONAIRE_PROFILE", "CONTACT_DATA",
        "CONTACT_DATA_OTHER_BEHALF_CONTRACTING_AUTORITHY", "CONTACT_DATA_WITHOUT_RESPONSIBLE_NAME",
        "TRANSLITERATED_ADDR", "WINNER",
    ]),
    // ------------------------------------------------------------- ignores
    (Rule::Ignore("boilerplate xlink links; every value points at ted.europa.eu"), &[
        "FORMS_LABELS_LINK", "LINKS_SECTION", "OFFICIAL_FORMS_LINK", "ORIGINAL_CPV_LINK",
        "ORIGINAL_NUTS_LINK", "XML_SCHEMA_DEFINITION_LINK",
    ]),
    (Rule::Ignore("per-language notice URL, derivable from DOC_ID"), &["URI_DOC", "URI_LIST"]),
    (Rule::Ignore("eSender reception plumbing; absent from published packages"), &[
        "CONTACT", "IDENTIFICATION", "NOTIFICATION", "PUBLICATION", "SENDER", "TECHNICAL",
    ]),
    (Rule::Ignore("TED database housekeeping date, not notice content"), &["DELETION_DATE"]),
    (Rule::Ignore("restates the language copies present in FORM_SECTION"), &["FORM_LG_LIST"]),
    (Rule::Ignore("eSender conversion note ('From Convertor')"), &["COMMENTS"]),
    (Rule::Ignore("pre-2016 OJS heading restated by HEADING"), &["OLD_HEADING"]),
    // --------------------------------------------------- ids and references
    (Rule::Id(IdKind::Plain), &[
        "CONTRACT_NO", "CONTRACT_NUMBER", "CUSTOMER_LOGIN", "ESENDER_LOGIN", "LOT_NO",
        "LOT_NUMBER", "NOTICE_UUID", "NO_DOC_EXT", "NO_DOC_OJS", "RECEPTION_ID", "REFERENCE_NUMBER",
    ]),
    (Rule::Id(IdKind::Ref), &["NOTICE_NUMBER_OJ"]),
    (Rule::Id(IdKind::National), &["NATIONALID"]),
    // ------------------------------------------------------ classifications
    (Rule::Cpv, &["CPV_CODE", "CURRENT_CPV", "ORIGINAL_CPV"]),
    (Rule::Nuts, &["CA_CE_NUTS", "CURRENT_NUTS", "NUTS", "ORIGINAL_NUTS", "PERFORMANCE_NUTS", "TENDERER_NUTS"]),
    // ----------------------------------------------------------------- codes
    (Rule::CodeAttr(&["CODE"]), &[
        "AA_AUTHORITY_TYPE", "AC_AWARD_CRIT", "CPV_SUPPLEMENTARY_CODE", "MA_MAIN_ACTIVITIES",
        "NC_CONTRACT_NATURE", "PR_PROC", "RP_REGULATION", "TD_DOCUMENT_TYPE", "TY_TYPE_BID",
    ]),
    (Rule::CodeAttr(&["CTYPE", "VALUE"]), &["TYPE_CONTRACT"]),
    (Rule::CodeAttr(&["VALUE", "CTYPE", "TYPE"]), &[
        "CA_ACTIVITY", "CA_TYPE", "CE_ACTIVITY", "COUNTRY", "DIRECTIVE", "ISO_COUNTRY",
        "LANGUAGE", "LEGAL_BASIS", "NOTICE",
    ]),
    (Rule::CodeAttr(&["VALUE"]), &[
        "ACCEPTED_VARIANTS", "ACTIVITY_OF_CONTRACTING_ENTITY", "CNT_NOTICE_INFORMATION_S_F18",
        "EX_ANTE_NOTICE_INFORMATION_S", "F18_IS_ELECTRONIC_AUCTION_USABLE", "FRAMEWORK_AGREEMENT",
        "IDENTIFY_SUBCONTRACT", "INDICATE_ANY_CHANGE", "INDICATE_ANY_SHARE", "IS_CANDIDATE_SELECTED",
        "LANGUAGE_ANY_EC", "LANGUAGE_EC", "NON_COMMUNITY_ORIGIN", "NOTICE_INVOLVES_DEFENCE",
        "NOTICE_INVOLVES_DESC_DEFENCE", "NO_OPEN_RESTRICTED", "REDUCTION_OF_THE_NUMBER",
        "REQUESTS_NAMES_PROFESSIONAL_QUALIFICATIONS", "SUBCONTRACT_AWARD_PART", "TYPE_OF_ACTIVITY",
        "TYPE_OF_CONTRACTING_AUTHORITY", "TYPE_SUPPLIES_CONTRACT",
    ]),
    (Rule::CodeText, &[
        "COLL_OJ", "HEADING", "INITIATOR", "LG_ORIG", "SERVICE_CATEGORY_DEFENCE",
        "SERVICE_CATEGORY_PUB_DEFENCE",
    ]),
    // ------------------------------------------------------ dates and times
    (Rule::Date, &[
        "DATE", "DATE_AWARD_SCHEDULED", "DATE_CONCLUSION_CONTRACT", "DATE_DECISION_JURY",
        "DATE_DISPATCH_INVITATIONS", "DATE_DISPATCH_NOTICE", "DATE_DISPATCH_ORIGINAL", "DATE_END",
        "DATE_EXPECTED_PUBLICATION", "DATE_OPENING_TENDERS", "DATE_PUB", "DATE_PUBLICATION_NOTICE",
        "DATE_RECEIPT_TENDERS", "DATE_START", "DATE_TENDER_VALID", "DS_DATE_DISPATCH",
    ]),
    (Rule::Time, &["TIME", "TIME_OPENING_TENDERS", "TIME_RECEIPT_TENDERS"]),
    // DD_DATE_REQUEST_DOCUMENT is published both bare and with a wall clock
    // (`20190131 14:00`) — measured on the 2019-01-02 daily.
    (Rule::DateTime, &["DD_DATE_REQUEST_DOCUMENT", "DT_DATE_FOR_SUBMISSION"]),
    (Rule::DateParts, &[
        "CLEARING_LAST_DATE", "CONTRACT_AWARD_DATE", "DATE_OJ", "DISPATCH_INVITATIONS_DATE",
        "END_DATE", "NOTICE_DISPATCH_DATE", "PROCEDURE_DATE_STARTING", "RECEIPT_LIMIT_DATE",
        "START_DATE", "TIME_LIMIT",
    ]),
    (Rule::DatePart, &["DAY", "MONTH", "YEAR"]),
    // ------------------------------------------------------------------ money
    (Rule::Amount, &[
        "DOCUMENT_COST", "EXCLUDING_VAT_VALUE", "HIGH", "HIGH_VALUE", "LOW", "LOW_VALUE", "MAX",
        "MIN", "VALUE", "VALUE_COST", "VAL_BARGAIN_PURCHASE", "VAL_ESTIMATED_TOTAL", "VAL_OBJECT",
        "VAL_PRICE_PAYMENT", "VAL_PRIZE", "VAL_REVENUE", "VAL_SUBCONTRACTING", "VAL_TOTAL",
        "VAL_TOTAL_AFTER", "VAL_TOTAL_BEFORE",
    ]),
    // ---------------------------------------------------------------- numbers
    (Rule::Number(Unit::FromTypeAttr), &["DURATION", "DURATION_TENDER_VALID"]),
    (Rule::Number(Unit::Fixed("MONTH")), &[
        "DURATION_FRAMEWORK_MONTH", "MONTHS", "NUMBER_OF_MONTHS", "PROVISIONAL_TIMETABLE_MONTH",
        "TIME_FRAME_SUBSEQUENT_CONTRACTS_MONTH",
    ]),
    (Rule::Number(Unit::Fixed("YEAR")), &["DURATION_FRAMEWORK_YEAR", "NUMBER_OF_YEARS"]),
    (Rule::Number(Unit::Fixed("DAY")), &[
        "DAYS", "PROVISIONAL_TIMETABLE_DAY", "TIME_FRAME_SUBSEQUENT_CONTRACTS_DAY",
    ]),
    (Rule::Number(Unit::Fixed("PCT")), &[
        "EXCLUDING_VAT_PRCT", "MAX_PERCENTAGE", "MIN_PERCENTAGE", "PCT_ALLOCATED_OPERATOR",
        "PCT_SUBCONTRACTING", "VAT_PRCT",
    ]),
    (Rule::Number(Unit::Fixed("KM")), &["NB_KILOMETRES"]),
    // ----------------------------------------------------------------- counts
    (Rule::Integer, &[
        "LOT_MAX_NUMBER", "LOT_MAX_ONE_TENDERER", "MAX_NUMBER_PARTICIPANTS", "NB_CONTRACT_AWARDED",
        "NB_ENVISAGED_CANDIDATE", "NB_MAX_LIMIT_CANDIDATE", "NB_MAX_PARTICIPANTS",
        "NB_MIN_LIMIT_CANDIDATE", "NB_MIN_PARTICIPANTS", "NB_PARTICIPANTS",
        "NB_PARTICIPANTS_OTHER_EU", "NB_PARTICIPANTS_SME", "NB_TENDERS_RECEIVED",
        "NB_TENDERS_RECEIVED_EMEANS", "NB_TENDERS_RECEIVED_NON_EU", "NB_TENDERS_RECEIVED_OTHER_EU",
        "NB_TENDERS_RECEIVED_SME", "NO_OJ", "NUMBER_PARTICIPANTS", "NUMBER_POSSIBLE_RENEWALS",
        "NUMBER_POSSIBLE_RENEWALS_RANGE_MAX", "NUMBER_POSSIBLE_RENEWALS_RANGE_MIN",
        "OFFERS_RECEIVED_NUMBER", "OFFERS_RECEIVED_NUMBER_MEANING", "OPE_ENVISAGED_NUMBER",
        "OPE_MAXIMUM_NUMBER", "OPE_MINIMUM_NUMBER",
    ]),
    // ------------------------------------------------------------- form roots
    (Rule::FormRoot, &[
        "CONTRACT_AWARD_DEFENCE", "CONTRACT_CONCESSIONAIRE_DEFENCE", "CONTRACT_DEFENCE",
        "F01_2014", "F02_2014", "F03_2014", "F04_2014", "F05_2014", "F06_2014", "F07_2014",
        "F08_2014", "F12_2014", "F13_2014", "F14_2014", "F15_2014", "F20_2014", "F21_2014",
        "F22_2014", "F23_2014", "F24_2014", "F25_2014", "MOVE", "PRIOR_INFORMATION_DEFENCE",
    ]),
    // -------------------------------------------------------------- free text
    (Rule::Text, &[
        "AA_NAME", "ACCELERATED_PROC", "ACTIVITY_OF_CONTRACTING_ENTITY_OTHER", "AC_CRITERION",
        "AC_WEIGHTING", "ADDITIONAL_INFORMATION", "ADDITIONAL_INFORMATION_ABOUT_LOTS",
        "ADDITIONAL_NEED", "ADDRESS", "ASSIST_PERSONS_REDUCTED_MOB", "ATTENTION",
        "CALCULATION_METHOD", "CANCELLATIONS_SERVICES", "CATEGORY", "CA_ACTIVITY_OTHER",
        "CA_TYPE_OTHER", "CE_ACTIVITY_OTHER", "CLEANLINESS_ROLLING_STOCK", "COMPLAINT_HANDLING",
        "CONDITIONS", "CONTACT_POINT", "CONTRACT_TITLE", "COST_PARAMETERS", "CRITERIA",
        "CRITERIA_CANDIDATE", "CRITERIA_EVALUATION", "CRITERIA_SELECTION",
        "CUST_SATISFACTION_SURVEY", "DEPOSITS_GUARANTEES_REQUIRED", "DEPOSIT_GUARANTEE_REQUIRED",
        "DETAILS_PAYMENT", "DOCUMENT_METHOD_OF_PAYMENT", "D_JUSTIFICATION",
        "EAF_CAPACITY_INFORMATION", "EAF_CAPACITY_MIN_LEVEL", "ECONOMIC_FINANCIAL_INFO",
        "ECONOMIC_FINANCIAL_MIN_LEVEL", "ECONOMIC_OPERATORS_PERSONAL_SITUATION",
        "ECONOMIC_OPERATORS_PERSONAL_SITUATION_SUBCONTRACTORS",
        "EMPLOYMENT_PROTECTION_WORKING_CONDITIONS_VALUE",
        "ENVIRONMENTAL_PROTECTION_LEGISLATION_VALUE", "ESTIMATED_TIMING", "EU_PROGR_RELATED",
        "EXCLUSIVE_RIGHTS_GRANTED", "EXECUTION_SERVICE_RESERVED_PARTICULAR_PROFESSION",
        "EXISTENCE_OTHER_PARTICULAR_CONDITIONS", "E_MAIL", "FAX", "FILE_REFERENCE_NUMBER",
        "FREQUENCY_AWARDED_CONTRACTS", "FT", "IA_URL_ETENDERING", "IA_URL_GENERAL",
        "INFORMATION_TICKETS", "INFO_ADD", "INFO_ADD_EAUCTION", "INFO_ADD_SUBCONTRACTING",
        "INFO_ADD_VALUE", "JUSTIFICATION", "LABEL", "LANGUAGE_OTHER", "LEGAL_BASIS_OTHER",
        "LEGAL_FORM", "LIST", "LOCATION", "LODGING_OF_APPEALS_PRECISION",
        "LOT_COMBINING_CONTRACT_RIGHT", "LOT_DESCRIPTION", "LOT_TITLE", "MAIN_FEATURES_AWARD",
        "MAIN_FINANCING_CONDITION", "MAIN_FINANCING_CONDITIONS", "MAIN_SITE", "MEMBER_NAME",
        "METHODS", "NUMBER_VALUE_PRIZE", "OFFICIALNAME", "OPE_OBJECTIVE_CRITERIA",
        "OPTIONS_DESCR", "OPTION_DESCRIPTION", "ORDER_C", "ORIGINAL_OTHER_MEANS",
        "OTHER_PARTICULAR_CONDITIONS", "OTHER_QUALITY_TARGET", "OWNERSHIP", "P",
        "PARTICIPANT_NAME", "PARTICULAR_PROFESSION", "PERFORMANCE_CONDITIONS", "PHONE", "PLACE",
        "POSTAL_CODE", "PREDOMINANCE", "PROCUREMENT_LAW", "PTAN_JUSTIFICATION",
        "PTAR_JUSTIFICATION", "PUBLIC_SERVICE_OBLIGATIONS", "PUNCTUALITY_RELIABILITY",
        "REASON_CONTRACT_LAWFUL", "RECURRENT_PROCUREMENT", "REFERENCE_TO_LAW",
        "RELATES_TO_EU_PROJECT_YES", "RENEWAL_DESCR", "REVIEW_PROCEDURE", "REWARDS_PENALITIES",
        "RULES_CRITERIA", "SECTION", "SHORT_CONTRACT_DESCRIPTION", "SHORT_DESCR",
        "SHORT_DESCRIPTION_CONTRACT", "SIGNIFICANCE", "SOCIAL_STANDARDS", "SOFTWARE_VERSION",
        "SUITABILITY", "TAX_LEGISLATION_VALUE", "TECHNICAL_PROFESSIONAL_INFO",
        "TECHNICAL_PROFESSIONAL_MIN_LEVEL", "TEXT", "TITLE", "TITLE_CONTRACT", "TI_CY", "TI_TEXT",
        "TI_TOWN", "TOTAL_QUANTITY_OR_SCOPE", "TOWN", "TYPE_OF_ACTIVITY_OTHER",
        "TYPE_OF_CONTRACTING_AUTHORITY_OTHER", "T_CAPACITY_INFORMATION", "T_CAPACITY_MIN_LEVEL",
        "UNFORESEEN_CIRCUMSTANCE", "URL", "URL_BUYER", "URL_DOCUMENT", "URL_GENERAL",
        "URL_INFORMATION", "URL_NATIONAL_PROCEDURE", "URL_PARTICIPATE", "URL_PARTICIPATION",
        "URL_TOOL", "USE_ELECTRONIC_AUCTION", "WEIGHTING",
    ]),
    // ---------------------------------------------------------------- markers
    (Rule::Marker, &[
        "AC_PRICE", "AC_PROCUREMENT_DOC", "ADDITIONAL_WORKS", "ADDRESS_FURTHER_INFO_IDEM",
        "ADDRESS_PARTICIPATION_IDEM", "AIR_MARITIME_TRANSPORT_FOR_ARMED_FORCES_DEPLOYMENT",
        "AWARDED_CONTRACT", "AWARDED_SUBCONTRACTING", "AWARDED_TENDERER_VARIANT",
        "AWARDED_TO_GROUP", "CENTRAL_PURCHASING", "COMMUNITY_ORIGIN", "CONTRACT_COVERED_GPA",
        "CONTRACT_RESEARCH_DIRECTIVE", "CONTRACT_SERVICES_LISTED_IN_DIRECTIVE",
        "CONTRACT_SERVICES_OUTSIDE_DIRECTIVE", "CRITERIA_STATED_IN_OTHER_DOCUMENT",
        "DECISION_BINDING_CONTRACTING", "DESIGN_EXECUTION", "DIRECTIVE_2009_81_EC",
        "DIRECTIVE_2014_23_EU", "DIRECTIVE_2014_24_EU", "DIRECTIVE_2014_25_EU", "DIV_INTO_LOT_NO",
        "DOCUMENT_FULL", "DOCUMENT_RESTRICTED", "DPS", "DPS_ADDITIONAL_PURCHASERS",
        "D_ACCORDANCE_ARTICLE", "D_ADD_DELIVERIES_ORDERED", "D_ALL_TENDERS", "D_ARTISTIC",
        "D_BARGAIN_PURCHASE", "D_COMMODITY_MARKET", "D_CONTRACT_AWARDED_DESIGN_CONTEST",
        "D_EXCLUSIVE_RIGHT", "D_EXTREME_URGENCY", "D_FROM_LIQUIDATOR_CREDITOR",
        "D_FROM_WINDING_PROVIDER", "D_MANUF_FOR_RESEARCH", "D_MARITIME_SERVICES",
        "D_NO_TENDERS_REQUESTS", "D_OTHER_SERVICES", "D_OUTSIDE_SCOPE", "D_PERIODS_INCOMPATIBLE",
        "D_PROC_COMPETITIVE_DIALOGUE", "D_PROC_NEGOTIATED_PRIOR_CALL_COMPETITION", "D_PROC_OPEN",
        "D_PROC_RESTRICTED", "D_PROTECT_RIGHTS", "D_PURE_RESEARCH", "D_REPETITION_EXISTING",
        "D_SERVICES_LISTED", "D_TECHNICAL", "EAUCTION_USED", "ECATALOGUE_REQUIRED",
        "ECONOMIC_CRITERIA_DOC", "EINVOICING", "EORDERING", "EPAYMENT", "EXCLUDING_VAT",
        "EXECUTION", "EXTENDED_CONTRACT_DURATION", "EXTREME_URGENCY_EVENTS_UNFORESEEABLE",
        "FOLLOW_UP_CONTRACTS", "FRAMEWORK", "IDEM", "INDEFINITE_DURATION",
        "JOINT_PROCUREMENT_INVOLVED", "LIKELY_SUBCONTRACTED", "LOT_ALL", "LOT_DIVISION",
        "LOT_ONE_ONLY", "LOWEST_PRICE", "MANUFACTURED_BY_DIRECTIVE", "MODIFICATION_ORIGINAL",
        "NOTHING", "NO_ACCEPTED_VARIANTS", "NO_ADDITIONAL_WORKS",
        "NO_AIR_MARITIME_TRANSPORT_FOR_ARMED_FORCES_DEPLOYMENT", "NO_AWARDED_CONTRACT",
        "NO_AWARDED_PRIZE", "NO_AWARDED_TENDERER_VARIANT", "NO_AWARDED_TO_GROUP",
        "NO_CONTRACT_COVERED_GPA", "NO_CONTRACT_LIKELY_SUB_CONTRACTED",
        "NO_CONTRACT_RESEARCH_DIRECTIVE", "NO_DECISION_BINDING_CONTRACTING", "NO_EU_PROGR_RELATED",
        "NO_EXCLUSIVE_RIGHTS_GRANTED", "NO_EXEC_SERVICE_RESERVED_PARTICULAR_PROFESSION",
        "NO_EXISTENCE_OTHER_PARTICULAR_CONDITIONS", "NO_EXTENDED_CONTRACT_DURATION",
        "NO_EXTREME_URGENCY_EVENTS_UNFORESEEABLE", "NO_FOLLOW_UP_CONTRACTS", "NO_LOT_DIVISION",
        "NO_MANUFACTURED_BY_DIRECTIVE", "NO_ONLY_IRREGULAR_INACCEPTABLE_TENDERERS", "NO_OPTIONS",
        "NO_OTHER_PREVIOUS_PUBLICATION", "NO_PARTICULAR_PROFESSION", "NO_PAYABLE_DOCUMENTS",
        "NO_PERIOD_FOR_PROCEDURE_INCOMPATIBLE_WITH_CRISIS", "NO_PREVIOUS_PUBLICATION_EXISTS_F17",
        "NO_PREVIOUS_PUBLICATION_EXISTS_F18", "NO_PREVIOUS_PUBLICATION_EXISTS_F19",
        "NO_PRIZE_AWARDED", "NO_RECURRENT_CONTRACT", "NO_RECURRENT_PROCUREMENT", "NO_RENEWAL",
        "NO_SME", "NO_SUPPLIES_QUOTED_PURCHASED_COMMODITY_MARKET", "NO_TENDERS_EXCLUDED",
        "NO_USE_ELECTRONIC_AUCTION", "NO_WORKS_REPETITION_EXISTING_WORKS",
        "ONLY_IRREGULAR_INACCEPTABLE_TENDERERS", "OPTIONS", "ORIGINAL_ENOTICES",
        "ORIGINAL_TED_ESENDER", "PERFORMANCE_STAFF_QUALIFICATION",
        "PERIOD_FOR_PROCEDURE_INCOMPATIBLE_WITH_CRISIS", "PREVIOUS_NOTICE_BUYER_PROFILE_F18",
        "PREVIOUS_NOTICE_BUYER_PROFILE_F19", "PRIOR_INFORMATION_NOTICE_F17", "PRIZE_AWARDED",
        "PROCUREMENT_DISCONTINUED", "PROCUREMENT_UNSUCCESSFUL", "PT_ACCELERATED_NEGOTIATED",
        "PT_ACCELERATED_RESTRICTED", "PT_AWARD_CONTRACT_WITHOUT_CALL",
        "PT_AWARD_CONTRACT_WITHOUT_PUBLICATION", "PT_AWARD_CONTRACT_WITH_PRIOR_PUBLICATION",
        "PT_COMPETITIVE_DIALOGUE", "PT_COMPETITIVE_NEGOTIATION", "PT_COMPETITIVE_TENDERING",
        "PT_DA_EXCEPTIONAL_CIRCUMSTANCE_RAIL", "PT_DA_INTERNAL_OPERATOR",
        "PT_DA_MARKET_NETWORK_RAIL", "PT_DA_MEDIUM_ENTERPRISE", "PT_DA_OPERATOR_MANAGER_RAIL",
        "PT_DA_RAILWAY_TRANSPORT", "PT_DA_SMALL_CONTRACT", "PT_INNOVATION_PARTNERSHIP",
        "PT_INVOLVING_NEGOTIATION", "PT_NEGOTIATED_WITHOUT_PUBLICATION",
        "PT_NEGOTIATED_WITH_PRIOR_CALL", "PT_NEGOTIATED_WITH_PUBLICATION_CONTRACT_NOTICE",
        "PT_OPEN", "PT_REQUEST_EXPRESSION_INTEREST", "PT_RESTRICTED", "PUBLICATION_TED",
        "PURCHASING_ON_BEHALF_NO", "REALISATION_REQUIREMENTS_SPECIFIED_CONTRACTING_AUTHORITIES",
        "REASONS_PROVIDED_PARTICULAR_TENDERER_EXCLUSIVE_RIGHTS",
        "REASONS_PROVIDED_PARTICULAR_TENDERER_TECHNICAL", "RECEIVERS_ARRANGEMENT_CREDITORS",
        "REDUCTION_RECOURSE", "RELATES_TO_EU_PROJECT_NO", "RENEWAL",
        "RESERVED_ORGANISATIONS_SERVICE_MISSION", "RESTRICTED_SHELTERED_PROGRAM",
        "RESTRICTED_SHELTERED_WORKSHOP", "RESTRICTED_TO_FRAMEWORK",
        "RESTRICTED_TO_SHELTERED_WORKSHOPS", "RIGHT_CONTRACT_INITIAL_TENDERS", "SINGLE_OPERATOR",
        "SME", "SUPPLIER_WINDING_UP_BUSINESS", "SUPPLIES_QUOTED_PURCHASED_COMMODITY_MARKET",
        "TECHNICAL_CRITERIA_DOC", "TENDERS_EXCLUDED", "TERMINATION_DPS", "TERMINATION_PIN",
        "UNKNOWN_VALUE", "WORKS_REPETITION_EXISTING_WORKS",
    ]),
    // ------------------------------------------------- structural containers
    (Rule::Group, &[
        "AC", "ACTIVITIES_OF_CONTRACTING_ENTITY", "AC_COST", "AC_CRITERIA", "AC_QUALITY",
        "ADMINISTRATIVE_INFORMATION_CONTRACT_AWARD_DEFENCE",
        "ADMINISTRATIVE_INFORMATION_CONTRACT_NOTICE_DEFENCE",
        "ADMINISTRATIVE_INFORMATION_CONTRACT_SUB_NOTICE_DEFENCE", "ANNEX_D",
        "APPEAL_PROCEDURE_BODY_RESPONSIBLE", "AUTHORITY_PRIOR_INFORMATION_DEFENCE",
        "AWARDED_PRIZE", "AWARD_CRITERIA_CONTRACT_AWARD_NOTICE_INFORMATION_DEFENCE",
        "AWARD_CRITERIA_CONTRACT_NOTICE_INFORMATION",
        "AWARD_CRITERIA_CONTRACT_NOTICE_INFORMATION_DEFENCE", "AWARD_CRITERIA_DETAIL",
        "AWARD_CRITERIA_DETAIL_F18", "CHANGES", "CNT_NOTICE_INFORMATION_F18", "CODED_DATA_SECTION",
        "CODIF_DATA", "COMPLEMENTARY_INFO", "COMPLEMENTARY_INFORMATION_CONTRACT_AWARD",
        "COMPLEMENTARY_INFORMATION_CONTRACT_NOTICE",
        "COMPLEMENTARY_INFORMATION_CONTRACT_NOTICE_DEFENCE", "CONDITIONS_OBTAINING_SPECIFICATIONS",
        "CONTRACTING_AUTHORITY_INFORMATION_CONTRACT_AWARD_DEFENCE",
        "CONTRACTING_AUTHORITY_INFORMATION_CONTRACT_SUB_DEFENCE",
        "CONTRACTING_AUTHORITY_INFORMATION_DEFENCE", "CONTRACTING_BODY", "CONTRACTOR",
        "CONTRACTORS", "CONTRACT_LIKELY_SUB_CONTRACTED_WITH_DEFENCE", "CONTRACT_RELATING_CONDITIONS",
        "CONTRACT_VALUE_INFORMATION", "COSTS_RANGE_AND_CURRENCY",
        "COSTS_RANGE_AND_CURRENCY_WITH_VAT_RATE", "COUNTRY_ORIGIN", "CPV", "CPV_ADDITIONAL",
        "CPV_MAIN", "CRITERIA_DEFINITION", "CRITERIA_STATED_BELOW",
        "DESCRIPTION_AWARD_NOTICE_INFORMATION_DEFENCE", "DESCRIPTION_CONTRACT_INFORMATION_DEFENCE",
        "DESCRIPTION_CONTRACT_SUB_DEFENCE", "DESCRIPTION_PROCUREMENT",
        "ECONOMIC_OPERATOR_NAME_ADDRESS", "EMPLOYMENT_PROTECTION_WORKING_CONDITIONS",
        "ENVIRONMENTAL_PROTECTION_LEGISLATION", "ESSENTIAL_ASSETS", "EX_ANTE_NOTICE_INFORMATION",
        "E_MAILS", "F16_DIVISION_INTO_LOTS", "F16_DIV_INTO_LOT_YES",
        "F17_CONDITIONS_FOR_PARTICIPATION", "F17_DIVISION_INTO_LOTS", "F17_DIV_INTO_LOT_YES",
        "F17_ECONOMIC_FINANCIAL_CAPACITY", "F17_ECONOMIC_FINANCIAL_CAPACITY_SUBCONTRACTORS",
        "F17_FRAMEWORK", "F17_PT_ACCELERATED_NEGOTIATED",
        "F18_PT_NEGOTIATED_WITHOUT_PUBLICATION_CONTRACT_NOTICE", "F19_CONDITIONS_FOR_PARTICIPATION",
        "F19_FRAMEWORK", "FD_CONTRACT_AWARD_DEFENCE", "FD_CONTRACT_CONCESSIONAIRE_DEFENCE",
        "FD_CONTRACT_DEFENCE", "FD_PRIOR_INFORMATION_DEFENCE", "FORM_SECTION", "FURTHER_INFORMATION",
        "INCLUDING_VAT", "INFORMATION_REGULATORY_FRAMEWORK", "INFO_MODIFICATIONS",
        "INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT", "INTERNET_ADDRESSES_CONTRACT",
        "INTERNET_ADDRESSES_CONTRACT_AWARD", "INTERNET_ADDRESSES_CONTRACT_DEFENCE",
        "INTERNET_ADDRESSES_PRIOR_INFORMATION", "INTERVAL_DATE", "IS_ELECTRONIC_AUCTION_USABLE",
        "JUSTIFICATION_CHOICE_NEGOCIATED_PROCEDURE", "LANGUAGES", "LEFTI", "LEFTI_CONTRACT_DEFENCE",
        "LEFTI_CONTRACT_SUB_DEFENCE", "LEFTI_PRIOR_INFORMATION", "LOCATION_NUTS",
        "LODGING_INFORMATION_FOR_SERVICE", "LODGING_OF_APPEALS", "MAXIMUM_NUMBER_INVITED",
        "MEDIATION_PROCEDURE_BODY_RESPONSIBLE", "ML_AA_NAMES", "ML_TITLES", "ML_TI_DOC",
        "MORE_INFORMATION_IF_ANNUAL_MONTHLY", "MORE_INFORMATION_TO_SUB_CONTRACTED",
        "MOST_ECONOMICALLY_ADVANTAGEOUS_TENDER", "MOST_ECONOMICALLY_ADVANTAGEOUS_TENDER_SHORT",
        "NAME_ADDRESSES_CONTACT_CONTRACT", "NAME_ADDRESSES_CONTACT_CONTRACT_AWARD",
        "NAME_ADDRESSES_CONTACT_PRIOR_INFORMATION", "NATURE_QUANTITY_SCOPE", "NEW_VALUE",
        "NOTICE_DATA", "NUMBER_POSSIBLE_RENEWALS_RANGE", "OBJECT_CONTRACT",
        "OBJECT_CONTRACT_INFORMATION_CONTRACT_AWARD_NOTICE_DEFENCE",
        "OBJECT_CONTRACT_INFORMATION_DEFENCE", "OBJECT_CONTRACT_SUB_DEFENCE",
        "OBJECT_WORKS_SUPPLIES_SERVICES_PRIOR_INFORMATION", "OLD_VALUE", "OPENING_CONDITION",
        "ORGANISATION", "OTHER_JUSTIFICATION", "OTHER_PREVIOUS_PUBLICATION",
        "OTHER_PREVIOUS_PUBLICATIONS", "OTH_INFO_PRIOR_INFORMATION", "PARTICIPANTS",
        "PAYABLE_DOCUMENTS", "PCT_RANGE_SHARE_SUBCONTRACTING", "PERIOD_WORK_DATE_STARTING",
        "PREVIOUS_PUBLICATION_EXISTS_F17", "PREVIOUS_PUBLICATION_EXISTS_F18",
        "PREVIOUS_PUBLICATION_EXISTS_F19", "PREVIOUS_PUBLICATION_INFORMATION_NOTICE_F17",
        "PREVIOUS_PUBLICATION_INFORMATION_NOTICE_F18",
        "PREVIOUS_PUBLICATION_INFORMATION_NOTICE_F19", "PREVIOUS_PUBLICATION_NOTICE_F17",
        "PREVIOUS_PUBLICATION_NOTICE_F18", "PREVIOUS_PUBLICATION_NOTICE_F19", "PROCEDURE",
        "PROCEDURES_FOR_APPEAL", "PROCEDURE_DEFINITION_CONTRACT_AWARD_NOTICE_DEFENCE",
        "PROCEDURE_DEFINITION_CONTRACT_NOTICE_DEFENCE", "PROCEDURE_DEFINITION_CONTRACT_SUB_DEFENCE",
        "PT_ACCELERATED_RESTRICTED_CHOICE", "PT_NEGOTIATED_CHOICE",
        "PURCHASE_SUPPLIES_ADVANTAGEOUS_TERMS", "PURCHASING_ON_BEHALF", "PURCHASING_ON_BEHALF_YES",
        "QS", "QUALIFICATION", "QUANTITY_SCOPE", "QUANTITY_SCOPE_WORKS_DEFENCE", "RANGE_VALUE",
        "RANGE_VALUE_COST", "REASONS_PROVIDED_PARTICULAR_TENDERER", "RECURRENT_CONTRACT",
        "REF_NOTICE", "REF_OJS", "RESERVED_CONTRACTS", "SCHEDULED_DATE_PERIOD",
        "SERVICES_CONTRACTS_SPECIFIC_CONDITIONS", "SEVERAL_OPERATORS", "SINGLE_VALUE",
        "SITE_OR_LOCATION", "SPECIFICATIONS_AND_ADDITIONAL_DOCUMENTS", "SUBCONTRACTING",
        "SUBCONTRACT_DEFENCE", "SUBCONTRACT_SHARE", "TAX_LEGISLATION", "TECHNICAL_CAPACITY_LEFTI",
        "TECHNICAL_CAPACITY_LEFTI_SUBCONTRACTORS", "TECHNICAL_SECTION", "TED_EXPORT", "TENDERS",
        "TENDERS_REQUESTS_APPLICATIONS_MUST_BE_SENT_TO", "TOTAL_ESTIMATED", "TOTAL_FINAL_VALUE",
        "TRANSLATION_SECTION", "TRANSLITERATIONS", "TYPE_AND_ACTIVITIES",
        "TYPE_AND_ACTIVITIES_OR_CONTRACTING_ENTITY_AND_PURCHASING_ON_BEHALF",
        "TYPE_CONTRACT_DEFENCE", "TYPE_CONTRACT_PI_DEFENCE", "TYPE_CONTRACT_PLACE_DELIVERY_DEFENCE",
        "TYPE_CONTRACT_W_PUB_DEFENCE", "TYPE_OF_PROCEDURE_CONTRACT_AWARD_DEFENCE",
        "TYPE_OF_PROCEDURE_DEFENCE", "TYPE_OF_PROCEDURE_DETAIL_FOR_CONTRACT_NOTICE_DEFENCE",
        "TYPE_WORK_CONTRACT", "VALUES", "VALUES_LIST", "VALUE_RANGE", "VAL_RANGE_OBJECT",
        "VAL_RANGE_TOTAL", "WHERE", "WINNERS",
    ]),
];
