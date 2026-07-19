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
            .map(|loc| Path { up: 0, steps: loc.steps[steps.len()..].to_vec() });
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
    for &(xpath, field_id, kind) in EXTRA {
        insert_extra(&mut root, xpath, field_id, kind, false)?;
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

    for &(xpath, reason) in IGNORED {
        root.descend(&locate(xpath)?.steps).ignored = Some(reason);
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
