//! The vendored eForms SDK field metadata, and what tender-db decides to do
//! with each field.
//!
//! ADR-0002: `fields.json` is a *checklist*, never a schema generator. It is
//! vendored verbatim (`crates/ingest/sdk/`) because it is the authority the
//! completeness test walks — the test fails if any field id lacks a decision.
//! The ADR-0002 amendment makes that per SDK version, so one file is vendored
//! per accepted `CustomizationID` minor.
//!
//! Notices declaring a customization outside [`ACCEPTED`] are quarantined
//! rather than parsed against a neighbouring version's metadata: xpaths for the
//! same business term do move between minors (SDK 1.15 CHANGELOG), so guessing
//! would silently mis-file values.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

/// The profiles tender-db parses, each with its vendored field inventory.
/// Adding one is: vendor the file, add the line, run the completeness test.
///
/// Three families share the mechanism:
///
/// - `eforms-sdk-1.x` — the EU SDK's `fields.json` at that minor.
/// - `eforms-de-2.x` — **SDK-DE**'s `fields.json` (gitlab.opencode.de
///   `OC000008125155/SDK-eforms-de`), a patched fork of the EU SDK: same
///   layout, zero fields removed, plus the national delta (the `OPT-002`
///   ProfileID field everywhere; from SDK-DE 1.14 also the 4 DEX fields in
///   the `german-eforms-extension` namespace, and 14 national codelists whose
///   values ride the ordinary code channel). eForms-DE 2.1 tracks *two* EU
///   bases (1.13 and 1.14), so it vendors both SDK-DE lines — see [`resolve`].
/// - `eforms-sdk-0.1` — the DÖE below-threshold dialect. No SDK artifact
///   defines it anywhere, so its inventory is **empirical**: every element
///   path observed across the full DÖE sample history (2022-12 → 2026-07,
///   ~250k notices), committed as the era checklist in `fields.json` shape
///   (`SDK01-` field ids). A path outside it quarantines the notice
///   (ADR-0004); extending the inventory is a reviewed commit + reprocess,
///   exactly like the text-era profile.
pub const ACCEPTED: &[(&str, &str)] = &[
    // Not vendored, deliberately: `eforms-sdk-1.0` — a small permanent DÖE
    // stream of E2/E3 below-threshold notices (~82/month, e.g.
    // vergabe.bremen.de) declares the EU SDK 1.0, whose fields.json predicates
    // use descendant axes and boolean `or` that the [`super::xpath`] grammar
    // does not model. Those notices quarantine as unknown-customization until
    // a slice extends the grammar and vendors 1.0.
    ("eforms-sdk-1.8", include_str!("../../sdk/fields-1.8.0.json")),
    ("eforms-sdk-1.9", include_str!("../../sdk/fields-1.9.0.json")),
    ("eforms-sdk-1.10", include_str!("../../sdk/fields-1.10.0.json")),
    ("eforms-sdk-1.11", include_str!("../../sdk/fields-1.11.0.json")),
    ("eforms-sdk-1.12", include_str!("../../sdk/fields-1.12.0.json")),
    ("eforms-sdk-1.13", include_str!("../../sdk/fields-1.13.0.json")),
    ("eforms-sdk-1.14", include_str!("../../sdk/fields-1.14.0.json")),
    ("eforms-sdk-1.15", include_str!("../../sdk/fields-1.15.0.json")),
    ("eforms-de-2.0", include_str!("../../sdk/fields-de-2.0.0.json")),
    ("eforms-de-2.1@eforms-sdk-1.13", include_str!("../../sdk/fields-de-2.1.0-eu-1.13.json")),
    ("eforms-de-2.1@eforms-sdk-1.14", include_str!("../../sdk/fields-de-2.1.0-eu-1.14.json")),
    ("eforms-sdk-0.1", include_str!("../../sdk/fields-sdk-0.1.json")),
    // The DÖE eForms-DE 1.0/1.1/1.2 national dialect. No SDK-DE `fields.json`
    // artifact exists for the 1.x line (issue 75: the SDK-eForms-DE fork begins
    // at national 2.0; the 1.x line is spec+schematron only), so — like
    // `eforms-sdk-0.1` — this inventory is empirical: the full observed element
    // path set across the archived eforms-de-1.x corpus, one merged era file for
    // all three minors (686/773 paths are shared, and a superset only over-claims).
    ("eforms-de-1.x", include_str!("../../sdk/fields-de-1.x.json")),
];

/// Resolve a notice's `CustomizationID` (plus its `cbc:ProfileID`, when
/// declared) to the [`ACCEPTED`] key it parses under, or `None` = quarantine.
///
/// This is the DE→EU version map the research pinned from the
/// Bekanntmachungsservice OpenAPI: eForms-DE 2.1 → EU 1.14 *and* 1.13, told
/// apart by ProfileID; 2.0 → 1.12. ProfileID is sometimes absent even on 2.1
/// (1,858 of 11,917 in 2026-06), so absence falls back to the empirically
/// dominant base, 1.13 (9,260 declared 1.13 vs 799 × 1.14 that month).
pub fn resolve(customization: &str, profile_id: Option<&str>) -> Option<&'static str> {
    if let Some(&(key, _)) = ACCEPTED.iter().find(|&&(key, _)| key == customization) {
        return Some(key);
    }
    if customization == "eforms-de-2.1" {
        return Some(match profile_id {
            Some("eforms-sdk-1.14") => "eforms-de-2.1@eforms-sdk-1.14",
            _ => "eforms-de-2.1@eforms-sdk-1.13",
        });
    }
    // The three eForms-DE 1.x minors share one merged empirical inventory; no
    // ProfileID split (unlike 2.1, each minor is its own CustomizationID).
    if matches!(customization, "eforms-de-1.0" | "eforms-de-1.1" | "eforms-de-1.2") {
        return Some("eforms-de-1.x");
    }
    None
}

#[derive(Debug, Deserialize)]
pub struct Sdk {
    #[serde(rename = "sdkVersion")]
    pub sdk_version: String,
    pub fields: Vec<Field>,
    #[serde(rename = "xmlStructure")]
    pub nodes: Vec<XmlNode>,
    #[serde(skip)]
    index: OnceLock<HashMap<String, usize>>,
}

#[derive(Debug, Deserialize)]
pub struct Field {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "xpathAbsolute")]
    pub xpath: String,
    #[serde(rename = "attributeOf")]
    pub attribute_of: Option<String>,
    #[serde(rename = "codeList")]
    pub code_list: Option<CodeList>,
}

#[derive(Debug, Deserialize)]
pub struct CodeList {
    pub value: CodeListValue,
}

#[derive(Debug, Deserialize)]
pub struct CodeListValue {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct XmlNode {
    pub id: String,
    #[serde(rename = "xpathAbsolute")]
    pub xpath: String,
    #[serde(default)]
    pub repeatable: bool,
    #[serde(rename = "identifierFieldId")]
    pub identifier_field_id: Option<String>,
}

/// Where a field's value goes in the notice-parsed layer, or why it does not
/// need a value row of its own. Every field id resolves to exactly one of
/// these — that totality *is* the ADR-0002 guarantee, and the completeness
/// test asserts it per vendored version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Texts,
    Codes,
    /// CPV and NUTS — the two code lists that are classifications rather than
    /// controlled vocabulary for a single field.
    Classifications,
    Amounts,
    Dates,
    Integers,
    Numbers,
    Ids,
    /// An XML attribute of another field (`attributeOf`). 480 of 1256 fields in
    /// SDK 1.15 are these: `@listName`, `@languageID`, `@schemeName`,
    /// `@currencyID`, `@unitCode`. They are consumed by their owning field's
    /// handler, which is what makes them claimed without a row of their own.
    Attribute,
    /// An SDK "virtual" field: a second field id over an xpath another field
    /// already owns (the three `OPA-*` numeric views of duration measures).
    VirtualView,
}

impl Decision {
    /// The value tables a decision writes into, for the completeness report.
    pub fn label(self) -> &'static str {
        match self {
            Decision::Texts => "texts",
            Decision::Codes => "codes",
            Decision::Classifications => "classifications",
            Decision::Amounts => "amounts",
            Decision::Dates => "dates",
            Decision::Integers => "integers",
            Decision::Numbers => "numbers",
            Decision::Ids => "ids",
            Decision::Attribute => "excluded: attribute of another field",
            Decision::VirtualView => "excluded: virtual view of another field's xpath",
        }
    }
}

/// The mapping registry, as data: field *type* plus xpath context decides the
/// target, so the 1256 fields need no 1256 hand-written arms — but every field
/// id still gets a recorded decision (see [`Sdk::decisions`]).
pub fn decide(field: &Field) -> Option<Decision> {
    if field.attribute_of.is_some() {
        return Some(Decision::Attribute);
    }
    if field.id.starts_with("OPA-") {
        return Some(Decision::VirtualView);
    }
    Some(match field.kind.as_str() {
        "text" | "text-multilingual" | "url" | "phone" | "email" => Decision::Texts,
        "code" => {
            // CPV and NUTS are the classification vocabularies; every other
            // code list is a controlled value of its own field.
            let list = field.code_list.as_ref().map_or("", |c| c.value.id.as_str());
            if list == "cpv" || list.starts_with("nuts") {
                Decision::Classifications
            } else {
                Decision::Codes
            }
        }
        "amount" => Decision::Amounts,
        "date" | "time" => Decision::Dates,
        // eForms indicators are booleans; SQLite has no BOOLEAN under STRICT,
        // so they land as 0/1 integers alongside the counts.
        "indicator" | "integer" => Decision::Integers,
        "number" | "measure" => Decision::Numbers,
        "id" | "id-ref" => Decision::Ids,
        _ => return None,
    })
}

impl Sdk {
    /// The recorded decision per field id — what the completeness test walks.
    pub fn decisions(&self) -> Vec<(&str, Option<Decision>)> {
        self.fields.iter().map(|f| (f.id.as_str(), decide(f))).collect()
    }

    pub fn field(&self, id: &str) -> Option<&Field> {
        self.by_id().get(id).map(|&i| &self.fields[i])
    }

    fn by_id(&self) -> &HashMap<String, usize> {
        self.index.get_or_init(|| {
            self.fields.iter().enumerate().map(|(i, f)| (f.id.clone(), i)).collect()
        })
    }
}

/// Load (once per process) the vendored metadata for a `CustomizationID`.
pub fn load(customization: &str) -> Option<&'static Sdk> {
    static LOADED: OnceLock<HashMap<&'static str, Sdk>> = OnceLock::new();
    LOADED
        .get_or_init(|| {
            ACCEPTED
                .iter()
                .map(|&(id, json)| {
                    let sdk: Sdk = serde_json::from_str(json)
                        .unwrap_or_else(|e| panic!("vendored fields.json for {id} is malformed: {e}"));
                    (id, sdk)
                })
                .collect()
        })
        .get(customization)
}
