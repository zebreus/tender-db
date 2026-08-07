//! The match index: one SDK version's node and field xpaths, folded into a
//! tree that mirrors the XML.
//!
//! The index is what makes exhaustive consumption (ADR-0004) cheap. Walking a
//! notice is a simultaneous descent of the document and this tree: an element
//! that has no branch here is unclaimed content, and so is an attribute with no
//! attribute-field and text under a branch that is not a field. There is no
//! separate "ignore list" to drift out of sync — the SDK's own inventory is
//! both the mapping and the ignore rule.

use std::collections::HashMap;

use super::sdk::{self, Decision, Sdk};
use super::xpath::{self, Path, Step};

/// A repeatable SDK node — one instance becomes one row in `notice_sections`,
/// and the values below it hang off that section.
#[derive(Debug)]
pub struct NodeInfo {
    pub id: String,
    /// The section's `kind`: the SDK node id without its `ND-` prefix (Lot,
    /// LotsGroup, Part, Organization, LotResult, …).
    pub kind: String,
    /// Where the instance's published identifier (`LOT-0001`) sits, relative to
    /// the node element.
    pub identifier: Option<Path>,
}

#[derive(Debug)]
pub struct FieldInfo {
    pub id: String,
    pub decision: Decision,
    /// SDK type, kept for the value parser (`indicator` vs `integer`, …).
    pub kind: String,
    pub code_list: Option<String>,
}

/// Elements published TED notices carry that the SDK's own inventory does not
/// describe. ADR-0004 forbids a lenient default, so each one is an explicit,
/// reasoned rule; the subtree below an ignored element is claimed whole.
pub const IGNORED: &[(&str, &str)] = &[(
    "/*/cbc:ProfileID",
    "UBL profile marker restating cbc:CustomizationID (OPT-002-notice); no field id in any SDK version",
)];

/// Content-bearing elements published TED notices carry that the SDK's field
/// inventory does not describe at all — UBL elements the eForms schema permits
/// but defines no business term for.
///
/// ADR-0004 allows only two dispositions, mapped or explicitly ignored, and
/// "everything the source era publishes, nothing silently dropped" argues for
/// mapping: a contact's job title and a buyer's PO box are real data. They are
/// therefore given synthetic `UBL-` field ids and stored like any other value,
/// distinguishable from business terms by their prefix. Each entry is
/// (xpath, field id, SDK type).
/// Contexts where publishers mount an SDK subtree the SDK anchors elsewhere.
///
/// eForms' xpaths are context-specific — the same block is defined once under
/// `ProcurementProjectLot[@schemeName='Lot']` and, where the Regulation allows
/// it, again at procedure level. Real notices routinely use a mounting point
/// the inventory of a given SDK minor does not enumerate: procedure-level
/// `cac:ProcurementProject`/`cac:TenderingTerms`/`cac:TenderingProcess`, and
/// result-layer blocks nested inside `efac:LotResult` rather than beside it.
///
/// Each entry grafts the source subtree onto the target, *filling gaps only* —
/// a field the SDK already defines at the target always wins. The value's true
/// context is carried by its section, not by the SDK field id's suffix.
/// Each entry is (source prefix, target prefix).
pub const ALIASES: &[(&str, &str)] = &[
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:ProcurementProject",
        "/*/cac:ProcurementProject",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms",
        "/*/cac:TenderingTerms",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess",
        "/*/cac:TenderingProcess",
    ),
    // Fields the Regulation allows on a Part or a lots group but that a given
    // SDK minor only enumerates for a Lot.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']",
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Part']",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']",
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='LotsGroup']",
    ),
    // ...and the mirror case: a procedure-level block published per lot.
    ("/*/cac:ProcurementProject", "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:ProcurementProject"),
    ("/*/cac:TenderingTerms", "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms"),
    ("/*/cac:TenderingProcess", "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess"),
    // --- DÖE JAXB serializer quirks (issue 78). The DÖE OpenData export emits
    // eForms with a few structures the SDK models elsewhere; each is grafted
    // gap-fill (standard TED notices never carry these shapes, so they are
    // untouched).
    //
    // The whole UltimateBeneficialOwner (with efac:Nationality/BT-706) is nested
    // under efac:Organization; the SDK models the full UBO directly under
    // efac:Organizations, keeping only a reference cbc:ID under the Organization.
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:UltimateBeneficialOwner",
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:UltimateBeneficialOwner",
    ),
    // DÖE emits a UBL cac:AppealTerms block with the review body inlined as a full
    // party (WebsiteURI, PartyName, PostalAddress, Contact) — the SDK models the
    // review organisation via the efac register instead. Graft the Company org
    // subtree onto the inline party so its name/address/website/contact are claimed.
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company",
        "/*/cac:TenderingTerms/cac:AppealTerms/cac:AppealReceiverParty",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company",
        "/*/cac:TenderingTerms/cac:AppealTerms/cac:AppealInformationParty",
    ),
    // DÖE inlines the tender-recipient (submission) body as a full party under
    // cac:TenderingTerms/cac:TenderRecipientParty — at Lot level, and at procedure
    // level for the appeal bodies. Same Company graft.
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company",
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:TenderRecipientParty",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company",
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:AppealTerms/cac:AppealReceiverParty",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company",
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:AppealTerms/cac:AppealInformationParty",
    ),
    // Result-layer blocks nested one level deeper than the SDK models them.
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:NoticeResult/efac:LotTender",
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:NoticeResult/efac:LotResult/efac:LotTender",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:NoticeResult/efac:SettledContract",
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:NoticeResult/efac:LotResult/efac:SettledContract",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:StrategicProcurement",
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:NoticeResult/efac:LotResult/efac:StrategicProcurement",
    ),
    // Publishers restate award-criterion fields (type code, weight) on the
    // parent `cac:AwardingCriterion`, which the SDK models only under
    // `cac:SubordinateAwardingCriterion` (107 notices in the TED monthly
    // 2026-06). Gap-filling keeps the parent's own BT-543/BT-541 exact.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:AwardingTerms/cac:AwardingCriterion/cac:SubordinateAwardingCriterion",
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:AwardingTerms/cac:AwardingCriterion",
    ),
    // Withheld discriminators (seen on DÖE eforms-de notices, 96+20 in
    // 2026-06 alone): the SDK anchors a FieldsPrivacy block under the very
    // element variant whose discriminator the privacy block suppresses — a
    // legislation reference whose `cbc:ID` ('CrossBorderLaw') is withheld
    // matches the not(...) sibling variant instead, and a justification whose
    // `cbc:ProcessReasonCode` is withheld matches only the predicate-free
    // branch. The privacy subtree is grafted onto those landing branches;
    // inside it, the FieldsPrivacy step's own `efbc:FieldIdentifierCode`
    // predicate (which *is* published) keeps the BT-195 ids exact.
    (
        "/*/cac:TenderingTerms/cac:ProcurementLegislationDocumentReference[cbc:ID/text()='CrossBorderLaw']/ext:UBLExtensions",
        "/*/cac:TenderingTerms/cac:ProcurementLegislationDocumentReference[not(cbc:ID/text()=('CrossBorderLaw','LocalLegalBasis'))]/ext:UBLExtensions",
    ),
    (
        "/*/cac:TenderingProcess/cac:ProcessJustification[cbc:ProcessReasonCode/@listName='accelerated-procedure']/ext:UBLExtensions",
        "/*/cac:TenderingProcess/cac:ProcessJustification/ext:UBLExtensions",
    ),
    (
        "/*/cac:TenderingProcess/cac:ProcessJustification[cbc:ProcessReasonCode/@listName='direct-award-justification']/ext:UBLExtensions",
        "/*/cac:TenderingProcess/cac:ProcessJustification/ext:UBLExtensions",
    ),
    // --- TED eForms long tail (issue 18 mop-up): SDK subtrees publishers mount
    // one level deeper than the inventory models them.
    //
    // A doubly-nested subordinate award criterion (a criterion under a
    // criterion): graft the subordinate subtree one level deeper onto itself.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:AwardingTerms/cac:AwardingCriterion/cac:SubordinateAwardingCriterion",
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:AwardingTerms/cac:AwardingCriterion/cac:SubordinateAwardingCriterion/cac:SubordinateAwardingCriterion",
    ),
    // A UBLExtensions block published directly under the lot rather than under
    // its TenderingTerms — graft the TenderingTerms/UBLExtensions subtree onto it.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/ext:UBLExtensions",
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/ext:UBLExtensions",
    ),
    // A LotTender inlined under a SettledContract rather than beside it under
    // NoticeResult — graft the *whole* LotTender subtree (its TenderLot, its
    // TenderingParty ref, its amounts …) onto the nested position.
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:NoticeResult/efac:LotTender",
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:NoticeResult/efac:SettledContract/efac:LotTender",
    ),
    // A lot's TenderingProcess-extension content (AccessToolName BT-632,
    // ProcedureRelaunchIndicator BT-634 …) published under the lot's *direct*
    // UBLExtensions rather than under its TenderingProcess.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/ext:UBLExtensions",
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/ext:UBLExtensions",
    ),
];

pub const EXTRA: &[(&str, &str, &str)] = &[
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:PostAwardProcess/cbc:ElectronicCatalogueUsageIndicator",
        "UBL-ElectronicCatalogueUsage",
        "indicator",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:PostAwardProcess/cbc:ElectronicOrderUsageIndicator",
        "UBL-ElectronicOrderUsage",
        "indicator",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:PostAwardProcess/cbc:ElectronicInvoiceUsageIndicator",
        "UBL-ElectronicInvoiceUsage",
        "indicator",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:PostAwardProcess/cbc:ElectronicPaymentUsageIndicator",
        "UBL-ElectronicPaymentUsage",
        "indicator",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:CallForTendersDocumentReference/cac:Attachment/cac:ExternalReference/cbc:FileName",
        "UBL-DocumentFileName",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/cac:AdditionalDocumentReference/cbc:ID",
        "UBL-AdditionalDocumentID",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/cac:AdditionalDocumentReference/cac:Attachment/cac:ExternalReference/cbc:URI",
        "UBL-AdditionalDocumentURI",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:ProcurementProject/cac:ProcurementAdditionalType/cbc:ProcurementType",
        "UBL-ProcurementTypeLabel",
        "text",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company/cac:Contact/cbc:ID",
        "UBL-ContactID",
        "text",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:TouchPoint/cac:Contact/cbc:ID",
        "UBL-ContactID",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:CallForTendersDocumentReference/cac:Attachment/cac:ExternalReference/cbc:DocumentHash",
        "UBL-DocumentHash",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cbc:FundingProgram",
        "UBL-FundingProgram",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:ProcurementProject/cac:PlannedPeriod/cbc:StartTime",
        "UBL-PlannedPeriodStartTime",
        "time",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:ProcurementProject/cac:PlannedPeriod/cbc:EndTime",
        "UBL-PlannedPeriodEndTime",
        "time",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/cbc:TerminatedIndicator",
        "UBL-TerminatedIndicator",
        "indicator",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efbc:FrameworkMaximumAmount",
        "UBL-FrameworkMaximumAmount",
        "amount",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:SelectionCriteria/cbc:Name",
        "UBL-SelectionCriterionName",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:SelectionCriteria/cbc:CalculationExpressionCode",
        "UBL-SelectionCriterionUsage",
        "code",
    ),
    // A raw UBL weight on an awarding criterion; the SDK models weights only
    // as extension parameters (BT-5421..5423). Declared at the subordinate
    // criterion, the alias above mirrors it onto the parent.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:AwardingTerms/cac:AwardingCriterion/cac:SubordinateAwardingCriterion/cbc:WeightNumeric",
        "UBL-AwardCriterionWeightNumeric",
        "number",
    ),
    // UBL 2.3 forces a `cac:TenderResult` on every CAN; the SDK models only
    // its dummy AwardDate (OPT-999). Some eSenders fill the block in for
    // real — the result code beside the dummy date.
    ("/*/cac:TenderResult/cbc:TenderResultCode", "UBL-TenderResultCode", "code"),
    // DÖE eforms-de publishers restate the selection-criterion type in a
    // `cbc:CriterionTypeCode` the SDK's inventory does not model (it models
    // only `cbc:TendererRequirementTypeCode` there).
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:SelectionCriteria/cbc:CriterionTypeCode",
        "UBL-SelectionCriterionType",
        "code",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company/cac:PostalAddress/cac:Country/cbc:Name",
        "UBL-CountryName",
        "text",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:TouchPoint/cac:PostalAddress/cac:Country/cbc:Name",
        "UBL-CountryName",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:TendererQualificationRequest/cac:SpecificTendererRequirement/cbc:TendererRequirementTypeCode",
        "UBL-TendererRequirementTypeCode",
        "code",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company/cac:Contact/cbc:JobTitle",
        "UBL-JobTitle",
        "text",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:TouchPoint/cac:Contact/cbc:JobTitle",
        "UBL-JobTitle",
        "text",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company/cac:Contact/cbc:Department",
        "UBL-ContactDepartment",
        "text",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company/cac:PostalAddress/cbc:Postbox",
        "UBL-Postbox",
        "text",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:TouchPoint/cac:PostalAddress/cbc:Postbox",
        "UBL-Postbox",
        "text",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company/cac:PostalAddress/cbc:AddressFormatCode",
        "UBL-AddressFormatCode",
        "code",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:TouchPoint/cac:PostalAddress/cbc:AddressFormatCode",
        "UBL-AddressFormatCode",
        "code",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company/cac:PostalAddress/cbc:CountrySubentity",
        "UBL-CountrySubentity",
        "text",
    ),
    (
        "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:TouchPoint/cac:PostalAddress/cbc:CountrySubentity",
        "UBL-CountrySubentity",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:CallForTendersDocumentReference/cbc:LanguageID",
        "UBL-DocumentLanguageID",
        "code",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:CallForTendersDocumentReference/cbc:DocumentStatusCode",
        "UBL-DocumentStatusCode",
        "code",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:TendererQualificationRequest/cac:SpecificTendererRequirement/cbc:Description",
        "UBL-TendererRequirementDescription",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:TendererQualificationRequest/cbc:CompanyLegalFormCode",
        "UBL-CompanyLegalFormCode",
        "code",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:TendererQualificationRequest/cbc:CompanyLegalForm",
        "UBL-CompanyLegalForm",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:AppealTerms/cbc:Description",
        "UBL-AppealTermsDescription",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:AwardingTerms/cac:AwardingCriterion/cac:SubordinateAwardingCriterion/cbc:CalculationExpression",
        "UBL-CalculationExpression",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/cac:ProcessJustification/cbc:ProcessReason",
        "UBL-ProcessReason",
        "text",
    ),
    // --- TED eForms long tail (issue 18 mop-up): one-off UBL elements the SDK
    // inventory omits at the position publishers actually mount them. All are
    // fields=0 at these exact leaves across SDK 1.12–1.15 (verified), so each
    // fills a genuine gap rather than shadowing a business term.
    //
    // Free-text place-of-performance description (procedure level; the
    // procedure→lot alias mirrors it onto the lot's ProcurementProject too).
    (
        "/*/cac:ProcurementProject/cac:RealizedLocation/cac:Address/cbc:Description",
        "UBL-AddressDescription",
        "text",
    ),
    // Framework estimated maximum value published on a Lot — the SDK enumerates
    // this amount only under LotsGroup.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/cac:FrameworkAgreement/cbc:EstimatedMaximumValueAmount",
        "UBL-FrameworkEstimatedMaximumValue",
        "amount",
    ),
    // Invitation-to-submit deadline — the SDK enumerates only its StartDate; the
    // EndDate/EndTime pair reunites by UBL's Date/Time naming like any deadline.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/cac:InvitationSubmissionPeriod/cbc:EndDate",
        "UBL-InvitationSubmissionDeadline",
        "date",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/cac:InvitationSubmissionPeriod/cbc:EndTime",
        "UBL-InvitationSubmissionDeadline",
        "time",
    ),
    // A raw award-criterion weight (non-numeric form) beside the numeric one;
    // the Subordinate→AwardingCriterion alias mirrors it onto the parent.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:AwardingTerms/cac:AwardingCriterion/cac:SubordinateAwardingCriterion/cbc:Weight",
        "UBL-AwardCriterionWeight",
        "text",
    ),
    // A free-text contract-execution requirement whose ExecutionRequirementCode
    // listName the SDK does not enumerate a Description for (fsr / einvoicing /
    // esignature-submission / ecatalog-submission). Scoped by the exact
    // listName — NOT a bare `cac:ContractExecutionRequirement` step — because
    // that element is the one SDK block defined *only* with predicates, so a
    // predicate-free branch here would always match and suppress the walker's
    // relaxed by-name fallback for every unlisted-listName requirement (it did:
    // ~1.4k TED notices regressed in the dress rehearsal). A predicated branch
    // only joins its own listName and leaves the fallback intact.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:ContractExecutionRequirement[cbc:ExecutionRequirementCode/@listName='fsr']/cbc:Description",
        "UBL-ContractExecutionDescription",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:ContractExecutionRequirement[cbc:ExecutionRequirementCode/@listName='einvoicing']/cbc:Description",
        "UBL-ContractExecutionDescription",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:ContractExecutionRequirement[cbc:ExecutionRequirementCode/@listName='esignature-submission']/cbc:Description",
        "UBL-ContractExecutionDescription",
        "text",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:ContractExecutionRequirement[cbc:ExecutionRequirementCode/@listName='ecatalog-submission']/cbc:Description",
        "UBL-ContractExecutionDescription",
        "text",
    ),
    // A Spanish platform emits `cbc:ExecutionRequirementCode` with
    // listName='permission' — BT-63/BT-769's codelist, which no SDK minor
    // 1.0–1.15 defines for any contract-execution requirement (issues
    // 141/143). Same parent-predicate shape as the Description carve-outs
    // above: the predicated branch only ever joins its own listName, so it
    // exact-matches ahead of the relaxed by-name fallback (which dies
    // `ambiguous-field` across BT-736/743/744/764/OPT-060) and leaves that
    // fallback intact for genuinely unknown listNames.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:ContractExecutionRequirement[cbc:ExecutionRequirementCode/@listName='permission']/cbc:ExecutionRequirementCode",
        "UBL-ContractExecutionPermissionCode",
        "code",
    ),
    // Tender validity published as a deadline (issue 143): BT-98 is a
    // `cbc:DurationMeasure` in every minor, but a French publisher writes the
    // validity end as a date instead. Same class as
    // UBL-InvitationSubmissionDeadline above — the SDK enumerates one
    // temporal shape, publishers send the other — and the same Date/Time
    // pairing applies.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:TenderValidityPeriod/cbc:EndDate",
        "UBL-TenderValidityDeadline",
        "date",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:TenderValidityPeriod/cbc:EndTime",
        "UBL-TenderValidityDeadline",
        "time",
    ),
    // eInvoicing acceptance indicator — the SDK models only the usage indicators
    // (ElectronicInvoiceUsageIndicator etc.), not the accepted one.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:PostAwardProcess/cbc:ElectronicInvoiceAcceptedIndicator",
        "UBL-ElectronicInvoiceAccepted",
        "indicator",
    ),
    // A technical-committee member's given name — the SDK enumerates only the
    // FamilyName (BT-46) of the same person.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:AwardingTerms/cac:TechnicalCommitteePerson/cbc:FirstName",
        "UBL-CommitteePersonFirstName",
        "text",
    ),
    // A framework agreement's own duration period — the SDK models framework
    // fields but not this UBL DurationPeriod block; claim its period children.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/cac:FrameworkAgreement/cac:DurationPeriod/cbc:StartDate",
        "UBL-FrameworkDurationStart",
        "date",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/cac:FrameworkAgreement/cac:DurationPeriod/cbc:EndDate",
        "UBL-FrameworkDurationEnd",
        "date",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/cac:FrameworkAgreement/cac:DurationPeriod/cbc:DurationMeasure",
        "UBL-FrameworkDurationMeasure",
        "number",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/cac:FrameworkAgreement/cac:DurationPeriod/cbc:DescriptionCode",
        "UBL-FrameworkDurationDescriptionCode",
        "code",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/cac:FrameworkAgreement/cac:DurationPeriod/cbc:Description",
        "UBL-FrameworkDurationDescription",
        "text",
    ),
    // BT-531 under a mutated discriminator (issue 144, cause L): Austrian
    // (vemap) notices write the additional contract nature with
    // `listName='eforms-contract-nature'` — the TED genericode *file* name —
    // where every minor's predicate requires `listName='contract-nature'`.
    // The mutated block exact-matches only the predicate-free branch the
    // UBL-ProcurementTypeLabel entry above plants, which has no code leaf, so
    // the code died `unclaimed-content`. The values are legitimate BT-531
    // codes and no other business term shares this element+listName reading,
    // so the claim keeps the BT id rather than a synthetic one; the predicated
    // branch only ever joins the mutated listName and leaves genuinely
    // unknown listNames quarantining.
    (
        "/*/cac:ProcurementProject/cac:ProcurementAdditionalType[cbc:ProcurementTypeCode/@listName='eforms-contract-nature']/cbc:ProcurementTypeCode",
        "BT-531-Procedure",
        "code",
    ),
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:ProcurementProject/cac:ProcurementAdditionalType[cbc:ProcurementTypeCode/@listName='eforms-contract-nature']/cbc:ProcurementTypeCode",
        "BT-531-Lot",
        "code",
    ),
    // BT-76 legal-form text published in the wrong element (issue 144, cause
    // M): an Italian notice puts the company-legal-form free text in
    // `TendererQualificationRequest/cbc:Description` beside its (claimed)
    // `cbc:CompanyLegalFormCode` — every minor spells that text
    // `cbc:CompanyLegalForm`. One text leaf on the same predicate-free TQR
    // branch the UBL-CompanyLegalForm* entries above plant; no SDK version
    // declares any field at this path.
    (
        "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:TendererQualificationRequest/cbc:Description",
        "UBL-CompanyLegalFormDescription",
        "text",
    ),
];

#[derive(Debug, Default)]
pub struct Branch {
    /// The step that selects this branch from its parent. `None` at the root:
    /// the four eForms document types have four different root elements.
    pub step: Option<Step>,
    /// Set when a *repeatable* SDK node ends here.
    pub node: Option<NodeInfo>,
    pub field: Option<FieldInfo>,
    /// Set by an [`IGNORED`] rule: this element and everything under it is
    /// claimed, deliberately, and carries no value.
    pub ignored: Option<&'static str>,
    /// Attribute-leaf fields, by attribute name.
    pub attributes: HashMap<String, FieldInfo>,
    children: Vec<Branch>,
}

impl Branch {
    /// Every child branch whose step matches `node`, most specific (most
    /// predicates) first.
    ///
    /// Overlap is the norm, not the exception: the SDK describes one XML
    /// element with several node definitions whose predicates carve out
    /// different fields (`cac:TendererQualificationRequest` alone has five,
    /// mutually non-exclusive). A real element is therefore described by the
    /// *union* of the branches it matches, and the walker claims content
    /// against all of them.
    pub fn select(&self, node: roxmltree::Node<'_, '_>) -> Vec<&Branch> {
        self.matching(node, true)
    }

    /// Child branches selected by element name alone, ignoring predicates.
    ///
    /// Used only when [`select`](Self::select) finds nothing: TED publishes
    /// notices whose discriminator the SDK's predicates key on is missing or
    /// carries an unlisted value (an `efac:AppealRequestsStatistics` with no
    /// `efbc:StatisticsCode`, a `cac:ProcessJustification` with only a
    /// description). The element is known at this position, so it is claimed as
    /// a container — but the walker then requires the candidate branches to
    /// *agree* on a field before binding a value, so relaxing can never
    /// mis-attribute a value to the wrong business term.
    pub fn select_by_name(&self, node: roxmltree::Node<'_, '_>) -> Vec<&Branch> {
        self.matching(node, false)
    }

    fn matching(&self, node: roxmltree::Node<'_, '_>, predicates: bool) -> Vec<&Branch> {
        let mut matches: Vec<&Branch> = self
            .children
            .iter()
            .filter(|b| {
                b.step.as_ref().is_some_and(|s| {
                    if predicates { s.matches(node) } else { s.matches_name(node) }
                })
            })
            .collect();
        matches.sort_by_key(|b| std::cmp::Reverse(b.step.as_ref().map_or(0, |s| s.preds.len())));
        matches
    }

    fn child_mut(&mut self, step: &Step) -> &mut Branch {
        // Linear scan: branch fan-out is small (the widest node has ~40
        // children) and this runs once per SDK version.
        let existing = self.children.iter().position(|b| b.step.as_ref() == Some(step));
        match existing {
            Some(i) => &mut self.children[i],
            None => {
                self.children.push(Branch { step: Some(step.clone()), ..Branch::default() });
                self.children.last_mut().expect("just pushed")
            }
        }
    }

    fn descend(&mut self, steps: &[Step]) -> &mut Branch {
        let mut branch = self;
        for step in steps {
            branch = branch.child_mut(step);
        }
        branch
    }

    /// The branch at `steps` if every step is already present — unlike
    /// [`descend`](Self::descend), never creates one.
    fn existing(&self, steps: &[Step]) -> Option<&Branch> {
        let mut branch = self;
        for step in steps {
            branch = branch.children.iter().find(|b| b.step.as_ref() == Some(step))?;
        }
        Some(branch)
    }
}

#[derive(Debug)]
pub struct Error(String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for Error {}

/// Build the match tree for one SDK version. Fails only if the SDK ships an
/// xpath shape outside [`xpath`]'s grammar — a loud, test-visible failure
/// rather than a silent gap.
pub fn build(sdk: &Sdk) -> Result<Branch, Error> {
    let mut root = Branch::default();

    for node in &sdk.nodes {
        // Non-repeatable nodes merely group columns and open no section — with
        // one exception: the ~60 `efac:FieldsPrivacy` blocks (BT-195…BT-198).
        // Several of them can sit side by side under one owner, each describing
        // a different withheld field, so each needs its own section to keep
        // "which field, why, until when" together.
        if !node.repeatable && !node.xpath.contains("efac:FieldsPrivacy") {
            continue;
        }
        let steps = locate(&node.xpath)?.steps;
        let privacy = steps.last().is_some_and(|s| s.local == "FieldsPrivacy");
        // `identifierFieldId` sometimes names an `id-ref` rather than an `id`:
        // ND-ContractingParty's "identifier" is OPT-300-Procedure-Buyer, a
        // reference *to* an organization. Naming the buyer-role section after
        // the organization it points at would collide with that organization's
        // own section, so only a real `id` names a section.
        let identifier = node
            .identifier_field_id
            .as_deref()
            .and_then(|id| sdk.field(id))
            .filter(|f| f.kind == "id")
            .map(|f| locate(&f.xpath))
            .transpose()?
            .map(|loc| Path {
                origin: xpath::Origin::Context { up: 0 },
                steps: loc.steps[steps.len()..].to_vec(),
            });
        // The kind comes from the node id, not `businessEntityId`: node ids are
        // stable across SDK minors, while the entity names were recased and
        // reshaped (1.13 "lot" → 1.15 "Lot") and are absent from older SDKs
        // entirely (docs/research/eforms-data-model.md §1). A stored value the
        // API exposes must not change meaning with the publisher's SDK version.
        let kind = match privacy {
            true => "FieldsPrivacy".into(),
            false => node.id.strip_prefix("ND-").unwrap_or(&node.id).to_owned(),
        };
        root.descend(&steps).node = Some(NodeInfo { id: node.id.clone(), kind, identifier });
    }

    for field in &sdk.fields {
        insert_field(&mut root, field, &field.xpath, false)?;
    }

    // The TED-quirk patch tables ([`EXTRA`], [`ALIASES`]) correct *SDK-shaped*
    // inventories against what publishers really send. The sdk-0.1 inventory
    // is itself empirical — every observed path is already in it, and grafting
    // predicate branches over its predicate-free paths would shadow its field
    // ids — so the patches stay off there.
    if sdk.sdk_version != "eforms-sdk-0.1" {
        for &(xpath, field_id, kind) in EXTRA {
            insert_extra(&mut root, xpath, field_id, kind, false)?;
        }

        // BT-165 company size (issue 74). SDK 1.0–1.7 gate `efbc:CompanySizeCode`
        // behind a `//`+`or` join predicate the EU SDK dropped at 1.8 — and whose
        // subcontractor side never matches real data anyway (the SDK writes
        // `efac:Subcontractor`, the schema and every notice write `efac:SubContractor`).
        // So on those versions an economic operator's size code hangs under a bare
        // `efac:Company` and would go unclaimed. Bind it predicate-free, exactly as
        // 1.8+ declares it, so it is always consumed. Gap-fill (`true`): the 1.8+
        // inventories that already declare this leaf keep their own field untouched,
        // and the stored list comes from the element's `@listName` regardless.
        insert_extra(
            &mut root,
            "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension\
             /efac:Organizations/efac:Organization/efac:Company/efbc:CompanySizeCode",
            "BT-165-Organization-Company",
            "code",
            true,
        )?;

        // BT-803 transmission stamp (issue 141). TED's publication pipeline
        // stamps the eSender dispatch instant (`efbc:TransmissionDate` +
        // `efbc:TransmissionTime`) onto published notices since ~2023-05 —
        // envelope metadata written by the publisher, regardless of the minor
        // the notice declares. `fields-1.3.0.json` knows only the date half
        // (BT-803(t) enters the vendored line at 1.5.0) and `fields-1.0.0.json`
        // knows neither, so 4,827 sdk-1.3 notices quarantined on the stamp
        // alone. Claim both halves under their proper field ids on every
        // minor; gap-fill (`true`) keeps the declaration of inventories that
        // already carry them.
        for (leaf, id, kind) in [
            ("efbc:TransmissionDate", "BT-803(d)-notice", "date"),
            ("efbc:TransmissionTime", "BT-803(t)-notice", "time"),
        ] {
            insert_extra(
                &mut root,
                &format!(
                    "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent\
                     /efext:EformsExtension/{leaf}"
                ),
                id,
                kind,
                true,
            )?;
        }

        // OPT-060 contract-execution conditions code (issue 142). Estonian
        // eSender notices declaring eforms-sdk-1.3 publish the full BT-70
        // block — `cbc:ExecutionRequirementCode[@listName='conditions']`
        // beside the description — but the code element enters the vendored
        // line only at 1.7.0. On earlier minors it matches no exact leaf,
        // relaxes to five differing candidates (BT-736/743/744/764/801) and
        // the notice dies `ambiguous-field`. Claim it under its SDK-1.9+
        // shape: the parent's `conditions` predicate means the leaf only ever
        // joins a genuine BT-70 block (whose branch every minor has, via
        // BT-70 itself), so the relaxed fallback for unlisted listNames stays
        // intact — see the UBL-ContractExecutionDescription note on [`EXTRA`].
        // Gap-fill (`true`): 1.9+ declares this exact leaf and keeps its own.
        insert_extra(
            &mut root,
            "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms\
             /cac:ContractExecutionRequirement[cbc:ExecutionRequirementCode/@listName='conditions']\
             /cbc:ExecutionRequirementCode",
            "OPT-060-Lot",
            "code",
            true,
        )?;

        // Bare procedure-level ProcessJustification description (issue 142).
        // The same Estonian eSender emits `cac:ProcessJustification` with no
        // `cbc:ProcessReasonCode` at all, its Description merely repeating the
        // notice's own ContractFolderID UUID — publisher-invalid in every SDK
        // minor (BT-1252 requires the direct-award discriminator in each).
        // The privacy graft above creates a predicate-free PJ branch, so the
        // bare block exact-matches it, the relaxed fallback never runs, and
        // the Description goes unclaimed. Claim it under a synthetic UBL- id
        // like the other undeclared-but-published leaves; the predicated
        // direct-award branch still sorts first and keeps BT-1252 for real
        // justifications. Gap-fill (`true`): SDK-DE 1.x declares its own
        // field at this very path and keeps it.
        insert_extra(
            &mut root,
            "/*/cac:TenderingProcess/cac:ProcessJustification/cbc:Description",
            "UBL-ProcessJustificationDescription",
            "text",
            true,
        )?;

        // Bare Lot-level ProcessJustification description (issue 143, cause C).
        // French and Italian buyers publish a Lot `cac:ProcessJustification`
        // holding only free text — the exact shape SDK 1.12.0 itself adopted
        // when BT-745-Lot's xpath dropped its
        // `[cbc:ProcessReasonCode/@listName='no-esubmission-justification']`
        // predicate. On ≤1.11 the bare block exact-matches the predicate-free
        // PJ branch (planted at Lot level by the UBL-ProcessReason [`EXTRA`]
        // entry) and its Description dies unclaimed: the procedure-level
        // carve-out above is a direct call the [`ALIASES`] loop never
        // replicates onto the Lot — that loop rewrites only `sdk.fields` and
        // the [`EXTRA`] const. Claim the Lot description under BT-745-Lot,
        // the field later SDKs declare at this very path; moving the pair
        // into [`EXTRA`] instead would insert without gap-fill and displace
        // 1.12+'s (and eforms-de-2.x's) own BT-745-Lot declaration, and at
        // procedure level SDK-DE 1.x's DE1 field — so both stay direct,
        // gap-filled calls. The predicated ≤1.11 branch still sorts first
        // for real no-esubmission justifications.
        insert_extra(
            &mut root,
            "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess\
             /cac:ProcessJustification/cbc:Description",
            "BT-745-Lot",
            "text",
            true,
        )?;

        // Subcontracting term code without its discriminator (issue 143,
        // cause D). A Bulgarian eSender publishes
        // `efac:SubcontractingTerm/efbc:TermCode` with no `@listName` —
        // BT-773-Tender requires `[efbc:TermCode/@listName='applicability']`
        // in every minor 1.7–1.15, and the attr-less element exact-matches
        // the predicate-FREE ND-SubcontractedActivity branch instead (the
        // BT-64/65 home), which has no TermCode leaf, so the relaxed
        // fallback is suppressed and the code dies unclaimed. Claim it as
        // BT-773-Tender on that predicate-free branch, OPT-060-style: the
        // predicated ND-SubcontractedContract branch still sorts first and
        // keeps declared inventories' own leaf exact. Gap-fill (`true`):
        // SDK-DE 1.x declares its own DE1 field at this exact path and
        // keeps it.
        insert_extra(
            &mut root,
            "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension\
             /efac:NoticeResult/efac:LotTender/efac:SubcontractingTerm/efbc:TermCode",
            "BT-773-Tender",
            "code",
            true,
        )?;

        // Contract-execution description without its code (issue 143, cause
        // E). A German buyer publishes `cac:ContractExecutionRequirement`
        // blocks holding only a `cbc:Description` — BT-70's text with its
        // `conditions` discriminator dropped. On minors 1.0–1.8 the SDK's
        // own leaf-predicated code fields (BT-736/743/744/764 …) plant a
        // predicate-free CER branch; the bare block exact-matches it and the
        // description dies unclaimed. Add a Description leaf to that branch —
        // but only where the SDK itself already planted it: *creating* a
        // predicate-free CER step on the 1.9+ minors that define the block
        // only with parent predicates is the documented ~1.4k-notice
        // regression (see the UBL-ContractExecutionDescription note on
        // [`EXTRA`]). Adding a leaf to an existing branch claims only
        // currently-unclaimed descriptions and cannot change which branches
        // an element matches.
        let bare_cer = "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']\
                        /cac:TenderingTerms/cac:ContractExecutionRequirement";
        if root.existing(&locate(bare_cer)?.steps).is_some() {
            insert_extra(
                &mut root,
                &format!("{bare_cer}/cbc:Description"),
                "UBL-ContractExecutionDescription",
                "text",
                true,
            )?;
        }

        // Award-criterion parameter code without its discriminator (issue
        // 144, cause J). Austrian vemap notices publish
        // `efac:AwardCriterionParameter/efbc:ParameterCode` with no
        // `@listName` — every minor discriminates BT-5421/5422/5423 by that
        // attribute, so the bare code relaxes to three differing candidates
        // and dies `ambiguous-field`. With the discriminator genuinely
        // dropped, storing any one of the three BT ids would be a guess (the
        // number-weight/-fixed/-threshold codelists happen to be disjoint,
        // but a branch cannot see the value), so the code is claimed under a
        // synthetic UBL- id — code and any @listName stored as published.
        // Added only where the minor itself plants the predicate-free
        // parameter branch (1.0–1.7, whose predicate-free BT-541
        // ParameterNumeric leaf already claims the sibling); never *created*
        // on 1.8+, whose parameter blocks are parent-predicated only — the
        // documented predicate-free-branch regression class. On those minors
        // the proper listName'd codes keep their leaf-predicated BT-542x
        // branches, which sort first. Gap-fill for form's sake: no inventory
        // declares this exact leaf.
        for scheme in ["Lot", "LotsGroup"] {
            let parameter = format!(
                "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='{scheme}']/cac:TenderingTerms\
                 /cac:AwardingTerms/cac:AwardingCriterion/cac:SubordinateAwardingCriterion\
                 /ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension\
                 /efac:AwardCriterionParameter"
            );
            if root.existing(&locate(&parameter)?.steps).is_some() {
                insert_extra(
                    &mut root,
                    &format!("{parameter}/efbc:ParameterCode"),
                    "UBL-AwardCriterionParameterCode",
                    "code",
                    true,
                )?;
            }
        }

        // Lot-level procurement-legislation reference (issue 144, cause N).
        // German TED notices declaring plain eforms-sdk-1.10 carry
        // `cac:TenderingTerms/cac:ProcurementLegislationDocumentReference`
        // (`vob-a-eu` + optional description) on their lots and parts — a
        // home no EU minor declares (BT-01 is procedure-level only) but which
        // the vendored eForms-DE inventory declares verbatim
        // (DE1-ProcurementProjectLot-TenderingTerms-…-ID/-DocumentDescription):
        // the national toolchain emits its tailoring onto the EU
        // customization — the BT-803 "construct from another inventory in the
        // vendored line" class across *dialects* rather than minors. Claimed
        // as UBL- leaves mirroring the DE1 fields' xpaths and types, and
        // skipped wholesale when the inventory itself declares the
        // predicate-free construct (eforms-de-1.x), so the DE profiles' own
        // DE1 ids keep every match rather than being displaced by a
        // predicated twin branch.
        let plain_pldr = "/*/cac:ProcurementProjectLot/cac:TenderingTerms\
                          /cac:ProcurementLegislationDocumentReference";
        if root.existing(&locate(plain_pldr)?.steps).is_none() {
            for scheme in ["Lot", "Part"] {
                let pldr = format!(
                    "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='{scheme}']\
                     /cac:TenderingTerms/cac:ProcurementLegislationDocumentReference"
                );
                insert_extra(&mut root, &format!("{pldr}/cbc:ID"), "UBL-ProcurementLegislationID", "id", true)?;
                insert_extra(
                    &mut root,
                    &format!("{pldr}/cbc:DocumentDescription"),
                    "UBL-ProcurementLegislationDescription",
                    "text",
                    true,
                )?;
            }
        }

        // Gap-filling aliases, after every declared path is in place.
        for &(source, target) in ALIASES {
            for field in &sdk.fields {
                if let Some(rest) = field.xpath.strip_prefix(source) {
                    insert_field(&mut root, field, &format!("{target}{rest}"), true)?;
                }
            }
            for &(xpath, field_id, kind) in EXTRA {
                if let Some(rest) = xpath.strip_prefix(source) {
                    insert_extra(&mut root, &format!("{target}{rest}"), field_id, kind, true)?;
                }
            }
        }
    }

    for &(xpath, reason) in IGNORED {
        let branch = root.descend(&locate(xpath)?.steps);
        // An inventory that *does* declare a field here wins over the ignore
        // rule: SDK-DE defines `cbc:ProfileID` as OPT-002-notice-DET — for the
        // eforms-de profiles the declared EU base is content, not plumbing.
        if branch.field.is_none() {
            branch.ignored = Some(reason);
        }
    }

    Ok(root)
}

/// Place one SDK field at `xpath`. `gap_only` is set for aliased copies, which
/// must never displace a field the SDK declares at the target itself.
fn insert_field(root: &mut Branch, field: &sdk::Field, xpath: &str, gap_only: bool) -> Result<(), Error> {
    let decision = sdk::decide(field)
        .ok_or_else(|| Error(format!("field {} has unknown SDK type {}", field.id, field.kind)))?;
    // Two field ids can share one xpath (the OPA-* virtual views). The virtual
    // view is a documented exclusion and carries no row of its own.
    if decision == Decision::VirtualView {
        return Ok(());
    }
    let info = FieldInfo {
        id: field.id.clone(),
        decision,
        kind: field.kind.clone(),
        code_list: field.code_list.as_ref().map(|c| c.value.id.clone()),
    };
    let location = locate(xpath)?;
    let branch = root.descend(&location.steps);
    match location.attribute {
        Some(attr) => {
            if !(gap_only && branch.attributes.contains_key(&attr)) {
                branch.attributes.insert(attr, info);
            }
        }
        None if gap_only && branch.field.is_some() => {}
        None => branch.field = Some(info),
    }
    Ok(())
}

/// Place one [`EXTRA`] out-of-inventory field.
fn insert_extra(
    root: &mut Branch,
    xpath: &str,
    field_id: &str,
    kind: &str,
    gap_only: bool,
) -> Result<(), Error> {
    let decision = match kind {
        "text" => Decision::Texts,
        "code" => Decision::Codes,
        "date" | "time" => Decision::Dates,
        "indicator" => Decision::Integers,
        "amount" => Decision::Amounts,
        "number" => Decision::Numbers,
        "id" => Decision::Ids,
        other => return Err(Error(format!("EXTRA field {field_id} has unknown type {other}"))),
    };
    let branch = root.descend(&locate(xpath)?.steps);
    if !(gap_only && branch.field.is_some()) {
        branch.field = Some(FieldInfo {
            id: field_id.to_owned(),
            decision,
            kind: kind.to_owned(),
            code_list: None,
        });
    }
    Ok(())
}

fn locate(xpath: &str) -> Result<xpath::Location, Error> {
    xpath::parse_absolute(xpath).map_err(|e| Error(e.to_string()))
}

/// The index for a `CustomizationID`, built once per process.
pub fn for_customization(customization: &str) -> Option<&'static Branch> {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<String, &'static Branch>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut cache = cache.lock().expect("index cache poisoned");
    if let Some(branch) = cache.get(customization) {
        return Some(branch);
    }
    let sdk = sdk::load(customization)?;
    // Leaked deliberately: one index per accepted SDK version lives for the
    // whole process, and `&'static` keeps it out of every parse signature.
    let branch: &'static Branch = Box::leak(Box::new(
        build(sdk).unwrap_or_else(|e| panic!("vendored SDK {customization} has an unsupported xpath: {e}")),
    ));
    cache.insert(customization.to_owned(), branch);
    Some(branch)
}
