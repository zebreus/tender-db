//! Issue 483 unit 1: the buyer-role census.
//!
//! Award notices sometimes put the contractor, a review body or a platform vendor in the
//! buyer role (`Procedure-Buyer` in eForms, `buyer` in the legacy and sdk-0.1 dialects:
//! [`super::BUYER_ROLES`]). The 482 two-cluster read found 16 of its 45 false splits were such
//! mis-tags (16698's CAN swaps the roles: its contractor Ratio Web as the buyer, its real
//! buyer Instytut Adama Mickiewicza as the tenderer; 533381 the Tribunal Catalán de
//! Contratos; 438807 the UZP appeals department, with the real buyer only receiving tenders
//! and paying; 198229 European Dynamics). Every buyer-based guard trusts that
//! role, so this census measures how often it is wrong before a fix is chosen: demote the
//! role at projection, or ignore it in the guards only.
//!
//! It walks every parsed notice by id in windows (optionally one window in every
//! `stride`, a systematic sample), reads each notice's organization roles from its full
//! parse (DE-1.x folded first, as the projection does) and the resolved organization of
//! each mention (`organization_mentions`), and flags each buyer mention by class
//! ([`ROLE_CENSUS_CLASSES`]). A buyer mention is **clean** when no decisive class flags
//! it; a notice whose every buyer mention is flagged has **no clean buyer left**: its only
//! buyer is wrong.
//!
//! Read-only: the job stores its report and writes nothing else.

use super::{
    NoticeState, ORGANIZATION_KIND, SDK01_BUYER_KIND, SDK01_PARTY_KINDS, SDK01_WINNER_KIND, SUBTYPE_FIELD,
    first_code, is_legacy_profile, is_sdk01_profile, match_norm, nested_org_aliases, normalise_de1, role_name,
};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use store::{NoticeValue, Parsed};

/// Notice ids per window. A window is the unit of `stride` sampling and of the stop
/// check; it is read [`ROLE_CENSUS_CHUNK`] notices at a time.
pub const ROLE_CENSUS_WINDOW: i64 = 20_000;

/// Parsed notices per read ([`store::Db::parsed_window`]: every satellite of the chunk's
/// id range in one scan per table).
pub const ROLE_CENSUS_CHUNK: i64 = 1_000;

/// The default sample: one window in every 10. A full walk parses every notice (~3 ms
/// each with the mention read, issue 482's measure), which is the better part of a day
/// on prod; a tenth of the windows is evenly spread over the id space, so over every era
/// and Source, and `stride: 1` walks them all.
pub const ROLE_CENSUS_DEFAULT_STRIDE: u64 = 10;

/// Samples kept per class.
pub const ROLE_CENSUS_SAMPLES: usize = 30;

/// Other buyers listed per sample (`other_buyers_total` counts them all).
const SAMPLE_OTHER_BUYERS: usize = 10;

/// What a role reference says about the organization it names, for this census.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoleKind {
    Buyer,
    /// A winner, tenderer or (sub)contractor: an economic operator of this notice.
    Contractor,
    /// The review body proper (the organization that rules on appeals), not the one
    /// that gives information about them: the latter is routinely the buyer itself.
    ReviewBody,
    /// The eSender or procurement service provider (eForms `Procedure-SProvider`).
    Esender,
    /// The organization providing the procurement documents.
    DocsProvider,
    /// Next to the review body but not it: where to get information about appeals, the
    /// mediator (eForms `ReviewInfo` / `Mediator`, legacy `ADDRESS_REVIEW_INFO`). Routinely
    /// the buyer itself, so never a mis-tag alone; it corroborates a review-body name.
    ReviewAdjacent,
    /// A role only the real buyer (or its agent) holds: receives or evaluates tenders,
    /// gives additional information, pays or finances the contract, signs it. 438807
    /// names its real buyer POLREGIO only here, with the appeals office in the buyer slot.
    BuyerShaped,
}

impl RoleKind {
    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// The census's role vocabulary: the role a reference names ([`role_name`]: the eForms
/// OPT-300/301 suffix, or the legacy element folded onto the canonical name) to what it
/// means here. Data, not branches: a role the census should read is a row. Every eForms
/// row is a role of `crates/ingest/sdk/fields-*.json` (a test holds them to it).
const ROLE_KINDS: &[(&str, RoleKind)] = &[
    ("Procedure-Buyer", RoleKind::Buyer),
    ("buyer", RoleKind::Buyer),
    ("Tenderer", RoleKind::Contractor),
    ("Tenderer-MainCont", RoleKind::Contractor),
    ("Tenderer-SubCont", RoleKind::Contractor),
    ("winner", RoleKind::Contractor),
    ("Lot-ReviewOrg", RoleKind::ReviewBody),
    ("Part-ReviewOrg", RoleKind::ReviewBody),
    ("ReviewBody", RoleKind::ReviewBody),
    ("Lot-ReviewInfo", RoleKind::ReviewAdjacent),
    ("Part-ReviewInfo", RoleKind::ReviewAdjacent),
    ("Lot-Mediator", RoleKind::ReviewAdjacent),
    ("Part-Mediator", RoleKind::ReviewAdjacent),
    ("mediation-body", RoleKind::ReviewAdjacent),
    ("appeal-information", RoleKind::ReviewAdjacent),
    ("Procedure-SProvider", RoleKind::Esender),
    ("Lot-DocProvider", RoleKind::DocsProvider),
    ("Part-DocProvider", RoleKind::DocsProvider),
    ("specifications-provider", RoleKind::DocsProvider),
    ("Lot-TenderReceipt", RoleKind::BuyerShaped),
    ("Part-TenderReceipt", RoleKind::BuyerShaped),
    ("Lot-TenderEval", RoleKind::BuyerShaped),
    ("Part-TenderEval", RoleKind::BuyerShaped),
    ("Lot-AddInfo", RoleKind::BuyerShaped),
    ("Part-AddInfo", RoleKind::BuyerShaped),
    ("LotResult-Paying", RoleKind::BuyerShaped),
    ("LotResult-Financing", RoleKind::BuyerShaped),
    ("Contract-Signatory", RoleKind::BuyerShaped),
    ("tender-receipt", RoleKind::BuyerShaped),
    ("further-information", RoleKind::BuyerShaped),
];

/// Legacy address blocks read by their own element name, before [`role_name`]'s fold:
/// the fold puts `ADDRESS_REVIEW_INFO` (where to get information about appeals — very
/// often the buyer itself) under `review-body` beside the review body proper, and the
/// census must keep the two apart.
const LEGACY_ELEMENT_KINDS: &[(&str, RoleKind)] = &[
    ("ADDRESS_REVIEW_BODY", RoleKind::ReviewBody),
    ("APPEAL_PROCEDURE_BODY_RESPONSIBLE", RoleKind::ReviewBody),
    ("RESPONSIBLE_FOR_APPEAL_PROCEDURES", RoleKind::ReviewBody),
    ("ADDRESS_REVIEW_INFO", RoleKind::ReviewAdjacent),
];

/// The census kind of a role reference's field id.
fn role_kind(field_id: &str) -> Option<RoleKind> {
    if let Some(element) = field_id.strip_prefix("TED-")
        && let Some((_, kind)) = LEGACY_ELEMENT_KINDS.iter().find(|(e, _)| *e == element)
    {
        return Some(*kind);
    }
    let role = role_name(field_id)?;
    ROLE_KINDS.iter().find(|(r, _)| *r == role).map(|(_, k)| *k)
}

/// Which curated list a name pattern belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NameList {
    ReviewBody,
    Platform,
}

/// Names that are a review body, or a procurement platform vendor, whatever role a notice
/// gives them: `(label, phrase)`. A phrase matches a buyer name as whole words, after the
/// name is folded the way the buyer guard folds it ([`fold`]: [`match_norm`], then Latin
/// diacritics), so every phrase is written folded (a test holds each to its own fold).
///
/// A review-body name is decisive only when the notice corroborates it (`review-body-name`
/// against `review-body-name-alone`, [`ROLE_CENSUS_CLASSES`]): several of these bodies
/// (ÚOHS, the Raad van State, the Conseil d'État, any court) also buy in their own name.
///
/// Deliberately left out: `Commissione` (the European Commission is a real buyer), the
/// Polish Urząd Zamówień Publicznych as a whole (it also buys for itself; its appeals
/// department `Departament Odwołań`, 438807's buyer, is listed).
const NAME_PATTERNS: &[(&str, &str, NameList)] = &[
    ("PL KIO", "krajowa izba odwolawcza", NameList::ReviewBody),
    ("PL KIO", "kio", NameList::ReviewBody),
    ("PL UZP Departament Odwołań", "departament odwolan", NameList::ReviewBody),
    ("DE Vergabekammer", "vergabekammer", NameList::ReviewBody),
    ("DE Vergabekammer", "vergabekammern", NameList::ReviewBody),
    ("DE Vergabesenat", "vergabesenat", NameList::ReviewBody),
    ("AT Verwaltungsgericht", "verwaltungsgericht", NameList::ReviewBody),
    ("AT Verwaltungsgericht", "bundesverwaltungsgericht", NameList::ReviewBody),
    ("AT Verwaltungsgericht", "landesverwaltungsgericht", NameList::ReviewBody),
    ("AT Vergabekontrollsenat", "vergabekontrollsenat", NameList::ReviewBody),
    ("CZ ÚOHS", "uohs", NameList::ReviewBody),
    ("CZ ÚOHS", "urad pro ochranu hospodarske souteze", NameList::ReviewBody),
    ("SK ÚVO", "urad pre verejne obstaravanie", NameList::ReviewBody),
    ("SE Förvaltningsrätten", "forvaltningsratten", NameList::ReviewBody),
    ("SE Kammarrätten", "kammarratten", NameList::ReviewBody),
    ("ES Tribunal Catalán", "tribunal catala de contractes del sector public", NameList::ReviewBody),
    ("ES Tribunal Catalán", "tribunal catalan de contratos", NameList::ReviewBody),
    ("ES TACRC", "tribunal administrativo central de recursos contractuales", NameList::ReviewBody),
    ("ES recursos contractuales", "recursos contractuales", NameList::ReviewBody),
    ("ES tribunal de contratación", "tribunal administrativo de contratacion publica", NameList::ReviewBody),
    ("ES tribunal de contratos", "tribunal administrativo de contratos publicos", NameList::ReviewBody),
    ("ES tribunal de contratos", "tribunal de contratos publicos", NameList::ReviewBody),
    ("IT TAR", "tribunale amministrativo regionale", NameList::ReviewBody),
    ("IT TAR", "tar", NameList::ReviewBody),
    ("FR tribunal administratif", "tribunal administratif", NameList::ReviewBody),
    ("BE Raad van State", "raad van state", NameList::ReviewBody),
    ("BE Conseil d'État", "conseil d etat", NameList::ReviewBody),
    ("PT tribunal administrativo", "tribunal administrativo e fiscal", NameList::ReviewBody),
    ("PT tribunal administrativo", "tribunal administrativo de circulo", NameList::ReviewBody),
    ("HU Döntőbizottság", "kozbeszerzesi dontobizottsag", NameList::ReviewBody),
    ("RO CNSC", "consiliul national de solutionare a contestatiilor", NameList::ReviewBody),
    ("HR DKOM", "drzavna komisija za kontrolu postupaka javne nabave", NameList::ReviewBody),
    ("SI DKOM", "drzavna revizijska komisija", NameList::ReviewBody),
    ("DK Klagenævnet", "klagenaevnet for udbud", NameList::ReviewBody),
    ("DK Klagenævnet", "klagenavnet for udbud", NameList::ReviewBody),
    ("NO KOFA", "klagenemnda for offentlige anskaffelser", NameList::ReviewBody),
    ("NO KOFA", "kofa", NameList::ReviewBody),
    ("FI markkinaoikeus", "markkinaoikeus", NameList::ReviewBody),
    ("EE vaidlustuskomisjon", "vaidlustuskomisjon", NameList::ReviewBody),
    ("LV IUB", "iepirkumu uzraudzibas birojs", NameList::ReviewBody),
    ("BG KZK", "комисия за защита на конкуренцията", NameList::ReviewBody),
    ("GR AEPP", "αρχη εξετασησ προδικαστικων προσφυγων", NameList::ReviewBody),
    ("CY review authority", "αναθεωρητικη αρχη προσφορων", NameList::ReviewBody),
    ("MT review board", "public contracts review board", NameList::ReviewBody),
    ("IE/UK High Court", "high court", NameList::ReviewBody),
    ("European Dynamics", "european dynamics", NameList::Platform),
    ("EU-Supply", "eu supply", NameList::Platform),
    ("Mercell", "mercell", NameList::Platform),
    ("Vortal", "vortal", NameList::Platform),
    ("cosinex", "cosinex", NameList::Platform),
    ("subreport", "subreport", NameList::Platform),
    ("DTVP", "deutsches vergabeportal", NameList::Platform),
];

/// Commercial legal forms, as folded whole-word phrases: a name holding one is a company
/// (`Ratio Web Spółka z ograniczoną odpowiedzialnością`, `ISS HS Sp. z o.o.`, `S.A.`).
/// The swap classes read them: a commercial buyer awarding to a non-commercial tenderer.
const COMMERCIAL_FORMS: &[&str] = &[
    "sp z o o",
    "sp z oo",
    "spolka z ograniczona odpowiedzialnoscia",
    "spolka akcyjna",
    "spolka komandytowa",
    "spolka jawna",
    "sp k",
    "sp j",
    "s a",
    "sa",
    "gmbh",
    "ag",
    "kg",
    "s r o",
    "spol s r o",
    "a s",
    "as",
    "asa",
    "ltd",
    "limited",
    "plc",
    "llp",
    "llc",
    "inc",
    "b v",
    "bv",
    "n v",
    "nv",
    "s r l",
    "srl",
    "s p a",
    "spa",
    "s l",
    "sl",
    "s l u",
    "slu",
    "sas",
    "s a s",
    "sarl",
    "s a r l",
    "eurl",
    "oy",
    "oyj",
    "ab",
    "aps",
    "kft",
    "zrt",
    "nyrt",
    "d o o",
    "doo",
    "uab",
    "sia",
];

/// Public-law word stems: a folded name with a word starting with one is a public body
/// (`Instytut Adama Mickiewicza`, `Uniwersyteckie Centrum Kliniczne`, `Gmina`, `Stadt`).
/// Prefixes, not phrases, so `uniwersyte` holds `uniwersytet` and `uniwersyteckie`.
const PUBLIC_STEMS: &[&str] = &[
    "gmina",
    "miasto",
    "miejsk",
    "powiat",
    "wojewodztw",
    "urzad",
    "ministerstw",
    "ministry",
    "ministere",
    "ministerio",
    "ministero",
    "instytut",
    "institut",
    "instituto",
    "istituto",
    "uniwersyte",
    "universit",
    "politechni",
    "publiczn",
    "panstwow",
    "stadt",
    "gemeinde",
    "landkreis",
    "landratsamt",
    "kommun",
    "comune",
    "ajuntament",
    "ayuntamiento",
    "municipal",
    "municipio",
    "commune",
    "obec",
    "mesto",
];

/// A name as the patterns read it: [`match_norm`], Latin diacritics folded.
fn fold(name: &str) -> String {
    store::buyer_name_fold(&match_norm(name))
}

/// The first pattern of `list` the folded name holds as whole words.
fn name_pattern(folded: &str, list: NameList) -> Option<&'static str> {
    if folded.is_empty() {
        return None;
    }
    let padded = format!(" {folded} ");
    NAME_PATTERNS
        .iter()
        .find(|(_, phrase, l)| *l == list && padded.contains(&format!(" {phrase} ")))
        .map(|(label, _, _)| *label)
}

/// Whether the folded name holds a commercial legal form ([`COMMERCIAL_FORMS`]).
fn commercial(folded: &str) -> bool {
    let padded = format!(" {folded} ");
    !folded.is_empty() && COMMERCIAL_FORMS.iter().any(|f| padded.contains(&format!(" {f} ")))
}

/// Whether a word of the folded name starts with a public-law stem ([`PUBLIC_STEMS`]).
fn public(folded: &str) -> bool {
    folded.split(' ').any(|w| PUBLIC_STEMS.iter().any(|s| w.starts_with(s)))
}

/// The flag classes, in report order: `(name, decisive)`. A decisive class makes the
/// mention not a clean buyer; a non-decisive one is counted and sampled only.
///
/// - `contractor-same-section`: one Organization referenced as buyer AND as a winner,
///   tenderer or contractor of this notice — in ANY lot. Decided, not missed: a
///   procedure's buyer is never its own supplier, so a buyer that is a tenderer of another
///   lot is still the contractor in the buyer slot; the shape where the notice ALSO names
///   its real buyer is told apart by `no_clean_buyer`, not by dropping the flag.
/// - `contractor-org-same-name`: another section, the same resolved organization AND the
///   same folded name (299165's shape of two sections the resolver bound together).
/// - `contractor-org-other-name`: the same resolved organization under a DIFFERENT name.
///   NOT decisive: the org layer has known fusions (shared switchboard ids, the PL823 stub,
///   bare DE ids), and an in-house award to an Eigenbetrieb shares its authority's id.
/// - `contractor-name`: no organization match, but the folded name equals a contractor's.
/// - `buyer-tenderer-swap`: the buyer carries a commercial legal form and is not
///   public-shaped, while a tenderer (not itself a buyer) is public-shaped with no
///   commercial form — 16698 (Ratio Web Sp. z o.o. as buyer, Instytut Adama Mickiewicza
///   as tenderer) and 299165 (ISS HS Sp. z o.o. / Uniwersyteckie Centrum Kliniczne): the
///   two roles swapped, so no contractor class can see it.
/// - `swap-legal-form`: a commercial buyer and a tenderer with no commercial form that is
///   not public-shaped either (a natural person, an association, an unsuffixed name). NOT
///   decisive: the weak half of the swap signal, sized to see what it holds.
/// - `real-buyer-elsewhere`: the buyer holds no buyer-shaped role ([`RoleKind::BuyerShaped`]
///   or documents provider) while another organization, neither buyer nor contractor,
///   holds one — 438807 (the appeals office in the buyer slot, POLREGIO receiving tenders
///   and paying). NOT decisive (a central purchasing body leaves paying to its client);
///   its basis names the organization a demote could recover as the buyer.
/// - `review-body-name`: the name is a known review body ([`NAME_PATTERNS`]) AND the notice
///   agrees: the same organization holds a review(-adjacent) role, another buyer mention
///   is not a review-body name, or `real-buyer-elsewhere` holds.
/// - `review-body-name-alone`: the name alone. NOT decisive: ÚOHS, the Raad van State or a
///   court buying in its own name looks exactly like this.
/// - `review-body-role`: the notice's own review-body role names the same organization or
///   name. NOT decisive: a buyer filling its own name into the review-body block is a
///   mis-tag of THAT role, and it is common (UK and IE notices list themselves).
/// - `review-info-role`: the buyer is the appeals-information body or mediator. NOT
///   decisive: routinely the buyer itself.
/// - `esender`: the buyer is the notice's eSender / procurement service provider.
/// - `docs-provider`: the buyer is the documents provider. NOT decisive: a buyer handing
///   out its own documents is the normal case; counted to size it, never a mis-tag alone.
/// - `platform-name`: the name is a known platform vendor ([`NAME_PATTERNS`]).
pub const ROLE_CENSUS_CLASSES: [(&str, bool); 14] = [
    ("contractor-same-section", true),
    ("contractor-org-same-name", true),
    ("contractor-org-other-name", false),
    ("contractor-name", true),
    ("buyer-tenderer-swap", true),
    ("swap-legal-form", false),
    ("real-buyer-elsewhere", false),
    ("review-body-name", true),
    ("review-body-name-alone", false),
    ("review-body-role", false),
    ("review-info-role", false),
    ("esender", true),
    ("docs-provider", false),
    ("platform-name", true),
];

/// One organization of a notice: the roles referencing it, its name and its resolved
/// organization.
#[derive(Clone, Debug)]
struct Party {
    name: String,
    folded: String,
    org: Option<i64>,
    kinds: u8,
}

impl Party {
    fn is(&self, kind: RoleKind) -> bool {
        self.kinds & kind.bit() != 0
    }

    /// Whether `other` is the same organization: one resolved id, or one folded name.
    fn same(&self, other: &Party) -> bool {
        match (self.org, other.org) {
            (Some(a), Some(b)) if a == b => true,
            _ => self.same_name(other),
        }
    }

    fn same_org(&self, other: &Party) -> bool {
        matches!((self.org, other.org), (Some(a), Some(b)) if a == b)
    }

    fn same_name(&self, other: &Party) -> bool {
        !self.folded.is_empty() && self.folded == other.folded
    }
}

/// A buyer mention's verdict: its classes (one bit per [`ROLE_CENSUS_CLASSES`] index) and
/// what each matched.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Verdict {
    classes: u16,
    basis: Vec<String>,
}

impl Verdict {
    fn flag(&mut self, class: usize, basis: String) {
        self.classes |= 1 << class;
        self.basis.push(format!("{}: {basis}", ROLE_CENSUS_CLASSES[class].0));
    }

    fn has(&self, class: usize) -> bool {
        self.classes & (1 << class) != 0
    }

    /// No decisive class flags it.
    fn clean(&self) -> bool {
        ROLE_CENSUS_CLASSES.iter().enumerate().all(|(i, (_, decisive))| !decisive || !self.has(i))
    }
}

/// The organizations of one notice, by mention section: the roles referencing each (a
/// nested Organization's inner half lands on its outer one, as the projection binds it),
/// the name, and the resolved organization from `orgs` (`section → organization`).
fn notice_parties(sdk01: bool, notice_id: i64, parsed: &Parsed, orgs: Option<&HashMap<String, i64>>) -> Vec<Party> {
    let sections: HashMap<&str, &store::Section> = parsed.sections.iter().map(|s| (s.id.as_str(), s)).collect();
    let kinds: &[&str] = if sdk01 { SDK01_PARTY_KINDS } else { &[ORGANIZATION_KIND] };
    let alias = nested_org_aliases(&sections, kinds);
    let outer = |id: &str| -> String { alias.get(id).cloned().unwrap_or_else(|| id.to_owned()) };
    let mut roles: HashMap<String, u8> = HashMap::new();
    for value in &parsed.values {
        if let NoticeValue::Id { value: target, is_ref: true, scheme } = &value.value
            && scheme.as_deref() != Some("ojs")
            && let Some(kind) = role_kind(&value.field_id)
        {
            *roles.entry(outer(target)).or_default() |= kind.bit();
        }
    }
    if sdk01 {
        for s in &parsed.sections {
            let kind = match s.kind.as_str() {
                SDK01_BUYER_KIND => RoleKind::Buyer,
                SDK01_WINNER_KIND => RoleKind::Contractor,
                _ => continue,
            };
            *roles.entry(outer(&s.id)).or_default() |= kind.bit();
        }
    }
    NoticeState::mentions(sdk01, notice_id, parsed)
        .into_iter()
        .filter_map(|m| {
            let kinds = *roles.get(&m.section_id)?;
            let org = orgs.and_then(|o| o.get(&m.section_id)).copied();
            Some(Party { folded: fold(&m.name), name: m.name, org, kinds })
        })
        .collect()
}

/// The verdict of each buyer among `parties` (index into `parties`, verdict), in order.
fn judge(parties: &[Party]) -> Vec<(usize, Verdict)> {
    let class = |name: &str| ROLE_CENSUS_CLASSES.iter().position(|(n, _)| *n == name).expect("a census class");
    let label = |p: &Party| if p.name.trim().is_empty() { "(no name)".to_owned() } else { p.name.trim().to_owned() };
    let org_label = |p: &Party| format!("{} (org {})", label(p), p.org.map_or("-".to_owned(), |o| o.to_string()));
    let buyer_shaped = |p: &Party| p.is(RoleKind::BuyerShaped) || p.is(RoleKind::DocsProvider);
    let mut out = Vec::new();
    for (i, buyer) in parties.iter().enumerate().filter(|(_, p)| p.is(RoleKind::Buyer)) {
        let mut v = Verdict::default();
        let others = || parties.iter().enumerate().filter(move |(j, _)| *j != i).map(|(_, p)| p);
        // The buyer itself when its own section carries `kind`, else another party in
        // `kind` that is the same organization (one resolved id, or one folded name).
        let find = |kind: RoleKind| -> Option<&Party> {
            if buyer.is(kind) {
                return Some(buyer);
            }
            others().find(|p| p.is(kind) && buyer.same(p))
        };
        // The contractor classes, strongest first; one per mention.
        let contractors = || others().filter(|p| p.is(RoleKind::Contractor));
        if buyer.is(RoleKind::Contractor) {
            v.flag(class("contractor-same-section"), org_label(buyer));
        } else if let Some(p) = contractors().find(|p| buyer.same_org(p) && buyer.same_name(p)) {
            v.flag(class("contractor-org-same-name"), org_label(p));
        } else if let Some(p) = contractors().find(|p| !buyer.same_org(p) && buyer.same_name(p)) {
            v.flag(class("contractor-name"), label(p));
        } else if let Some(p) = contractors().find(|p| buyer.same_org(p)) {
            v.flag(class("contractor-org-other-name"), org_label(p));
        }
        // The swap: the roles exchanged, so the buyer is the company and the tenderer the
        // public body. Only when no contractor class already holds.
        if v.classes == 0 && commercial(&buyer.folded) && !public(&buyer.folded) {
            let tenderers = || contractors().filter(|p| !p.is(RoleKind::Buyer) && !commercial(&p.folded));
            if let Some(p) = tenderers().find(|p| public(&p.folded)) {
                v.flag(class("buyer-tenderer-swap"), format!("tenderer {}", label(p)));
            } else if let Some(p) = tenderers().find(|p| !p.folded.is_empty()) {
                v.flag(class("swap-legal-form"), format!("tenderer {}", label(p)));
            }
        }
        let elsewhere = if buyer_shaped(buyer) {
            None
        } else {
            others().find(|p| {
                buyer_shaped(p) && !p.is(RoleKind::Buyer) && !p.is(RoleKind::Contractor) && !buyer.same(p)
            })
        };
        if let Some(p) = elsewhere {
            v.flag(class("real-buyer-elsewhere"), org_label(p));
        }
        let review_role = find(RoleKind::ReviewBody);
        let review_info = find(RoleKind::ReviewAdjacent);
        if let Some(pattern) = name_pattern(&buyer.folded, NameList::ReviewBody) {
            let another_buyer = others()
                .any(|p| p.is(RoleKind::Buyer) && !buyer.same(p) && name_pattern(&p.folded, NameList::ReviewBody).is_none());
            let why = if review_role.is_some() || review_info.is_some() {
                Some("its review role")
            } else if another_buyer {
                Some("another buyer")
            } else if elsewhere.is_some() {
                Some("real buyer elsewhere")
            } else {
                None
            };
            match why {
                Some(why) => v.flag(class("review-body-name"), format!("{pattern}; {why}")),
                None => v.flag(class("review-body-name-alone"), pattern.to_owned()),
            }
        }
        if let Some(p) = review_role {
            v.flag(class("review-body-role"), label(p));
        }
        if let Some(p) = review_info {
            v.flag(class("review-info-role"), label(p));
        }
        if let Some(p) = find(RoleKind::Esender) {
            v.flag(class("esender"), label(p));
        }
        if let Some(p) = find(RoleKind::DocsProvider) {
            v.flag(class("docs-provider"), label(p));
        }
        if let Some(pattern) = name_pattern(&buyer.folded, NameList::Platform) {
            v.flag(class("platform-name"), pattern.to_owned());
        }
        out.push((i, v));
    }
    out
}

/// One sampled flagged buyer mention.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RoleCensusSample {
    pub notice_id: i64,
    /// `source:publication_id`.
    pub publication: String,
    pub subtype: String,
    /// The procedure type (BT-105, `-` for none): an in-house or negotiated-without-call
    /// award reads differently from an open procedure.
    pub procedure_type: String,
    /// The flagged buyer's name.
    pub flagged: String,
    /// What each of its classes matched (`buyer-tenderer-swap: tenderer Instytut Adama Mickiewicza`).
    pub basis: Vec<String>,
    /// The notice's other buyer mentions, each with its decisive classes in brackets when
    /// flagged (at most [`SAMPLE_OTHER_BUYERS`] of `other_buyers_total`).
    pub other_buyers_total: usize,
    pub other_buyers: Vec<String>,
    /// Whether any buyer mention of the notice is clean.
    pub clean_buyer_left: bool,
}

/// One class: how many buyer mentions and notices it flags, and a sample of them.
#[derive(Clone, Debug, Default, Serialize)]
pub struct RoleCensusClass {
    /// Whether the class makes a mention not clean ([`ROLE_CENSUS_CLASSES`]).
    pub decisive: bool,
    /// Buyer mentions it flags, and notices with at least one.
    pub mentions: u64,
    pub notices: u64,
    /// Of `notices`, those with no clean buyer left (every buyer decisively flagged).
    pub no_clean_buyer: u64,
    /// Of `notices`, those whose every buyer mention carries THIS class: acting on the
    /// class alone would leave them no buyer (the measure for a non-decisive class).
    pub every_buyer: u64,
    pub samples: Vec<RoleCensusSample>,
    /// The sample's ranks, parallel to `samples`.
    #[serde(skip)]
    ranks: Vec<u64>,
}

impl RoleCensusClass {
    /// Keep `sample` when its rank is among the [`ROLE_CENSUS_SAMPLES`] smallest: a
    /// bottom-k sample by a hash of the notice id, the same whatever the window or stride.
    fn offer(&mut self, rank: u64, sample: &RoleCensusSample) {
        if self.samples.len() < ROLE_CENSUS_SAMPLES {
            self.ranks.push(rank);
            self.samples.push(sample.clone());
        } else if let Some((at, &worst)) = self.ranks.iter().enumerate().max_by_key(|(_, r)| **r)
            && rank < worst
        {
            self.ranks[at] = rank;
            self.samples[at] = sample.clone();
        }
    }

    fn sort(&mut self) {
        let mut both: Vec<(u64, RoleCensusSample)> = self.ranks.drain(..).zip(self.samples.drain(..)).collect();
        both.sort_by_key(|(_, s)| s.notice_id);
        (self.ranks, self.samples) = both.into_iter().unzip();
    }
}

/// What the buyer-role census walked and found, accumulated window by window.
#[derive(Clone, Debug, Default, Serialize)]
pub struct BuyerRoleCensus {
    /// One window in every `stride` was read (1: all of them).
    pub stride: u64,
    /// Parsed notices read, those naming at least one buyer, and their buyer mentions.
    pub notices: u64,
    pub notices_with_buyers: u64,
    pub buyer_mentions: u64,
    /// Notices with a buyer mention any class flags, and with one a decisive class flags.
    pub flagged_notices: u64,
    pub decisively_flagged_notices: u64,
    /// Notices with buyers of which none is clean: the notice's only buyer is wrong.
    pub no_clean_buyer: u64,
    /// Per class ([`ROLE_CENSUS_CLASSES`]).
    pub classes: BTreeMap<String, RoleCensusClass>,
    /// Notices read per `source/subtype` (`legacy` for the legacy TED profiles, `-` for a
    /// notice with no subtype): the denominators of `cells`.
    pub read: BTreeMap<String, u64>,
    /// Notices per `source/subtype/class`, and per `source/subtype/no-clean-buyer`.
    pub cells: BTreeMap<String, u64>,
    /// Notices per `procedure type/class` (BT-105, `-` for none): in-house and
    /// negotiated-without-call awards as their own cells.
    pub procedures: BTreeMap<String, u64>,
    /// Notices per `class/pattern label` for the name-list classes (`review-body-name`,
    /// `review-body-name-alone`, `platform-name`): which list entries dominate.
    pub patterns: BTreeMap<String, u64>,
    /// The notice id the walk reached, and the one it walks to (captured before it).
    pub cursor: i64,
    pub target: i64,
    /// Windows passed and windows read.
    pub windows: u64,
    pub windows_read: u64,
    /// A cancel ended the walk.
    pub stopped: bool,
}

impl BuyerRoleCensus {
    pub fn new(stride: u64) -> BuyerRoleCensus {
        BuyerRoleCensus {
            stride,
            classes: ROLE_CENSUS_CLASSES
                .iter()
                .map(|(name, decisive)| ((*name).to_owned(), RoleCensusClass { decisive: *decisive, ..Default::default() }))
                .collect(),
            ..Default::default()
        }
    }

    /// Count one notice from its parties.
    fn classify(&mut self, notice: &store::NoticeRef, subtype: &str, procedure_type: &str, parties: &[Party]) {
        self.notices += 1;
        let cell = format!("{}/{subtype}", notice.source);
        *self.read.entry(cell.clone()).or_default() += 1;
        let verdicts = judge(parties);
        if verdicts.is_empty() {
            return;
        }
        self.notices_with_buyers += 1;
        self.buyer_mentions += verdicts.len() as u64;
        let clean_left = verdicts.iter().any(|(_, v)| v.clean());
        if verdicts.iter().any(|(_, v)| v.classes != 0) {
            self.flagged_notices += 1;
        }
        if !clean_left {
            self.no_clean_buyer += 1;
            *self.cells.entry(format!("{cell}/no-clean-buyer")).or_default() += 1;
        }
        if verdicts.iter().any(|(_, v)| !v.clean()) {
            self.decisively_flagged_notices += 1;
        }
        let rank = sample_rank(notice.id);
        let label = |p: &Party| if p.name.trim().is_empty() { "(no name)".to_owned() } else { p.name.trim().to_owned() };
        for (class, (name, _)) in ROLE_CENSUS_CLASSES.iter().enumerate() {
            let flagged: Vec<&(usize, Verdict)> = verdicts.iter().filter(|(_, v)| v.has(class)).collect();
            let Some((first, verdict)) = flagged.first().map(|(i, v)| (*i, v)) else { continue };
            let tally = self.classes.get_mut(*name).expect("a census class");
            tally.mentions += flagged.len() as u64;
            tally.notices += 1;
            if !clean_left {
                tally.no_clean_buyer += 1;
            }
            if flagged.len() == verdicts.len() {
                tally.every_buyer += 1;
            }
            *self.cells.entry(format!("{cell}/{name}")).or_default() += 1;
            *self.procedures.entry(format!("{procedure_type}/{name}")).or_default() += 1;
            let list = match *name {
                "review-body-name" | "review-body-name-alone" => Some(NameList::ReviewBody),
                "platform-name" => Some(NameList::Platform),
                _ => None,
            };
            if let Some(list) = list {
                let labels: std::collections::BTreeSet<&str> =
                    flagged.iter().filter_map(|(i, _)| name_pattern(&parties[*i].folded, list)).collect();
                for l in labels {
                    *self.patterns.entry(format!("{name}/{l}")).or_default() += 1;
                }
            }
            let others: Vec<String> = verdicts
                .iter()
                .filter(|(i, _)| *i != first)
                .map(|(i, v)| {
                    let decisive: Vec<&str> = ROLE_CENSUS_CLASSES
                        .iter()
                        .enumerate()
                        .filter(|(c, (_, d))| *d && v.has(*c))
                        .map(|(_, (n, _))| *n)
                        .collect();
                    if decisive.is_empty() {
                        label(&parties[*i])
                    } else {
                        format!("{} [{}]", label(&parties[*i]), decisive.join(","))
                    }
                })
                .collect();
            let sample = RoleCensusSample {
                notice_id: notice.id,
                publication: format!("{}:{}", notice.source, notice.publication_id),
                subtype: subtype.to_owned(),
                procedure_type: procedure_type.to_owned(),
                flagged: label(&parties[first]),
                basis: verdict.basis.clone(),
                other_buyers_total: others.len(),
                other_buyers: others.into_iter().take(SAMPLE_OTHER_BUYERS).collect(),
                clean_buyer_left: clean_left,
            };
            tally.offer(rank, &sample);
        }
    }

    fn finish(&mut self) {
        for class in self.classes.values_mut() {
            class.sort();
        }
    }
}

/// A stable pseudo-random rank of a notice id (splitmix64), for the bottom-k samples.
fn sample_rank(id: i64) -> u64 {
    let mut z = (id as u64).wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The census's subtype label of a notice: the eForms subtype (OPP-070), `legacy` for the
/// legacy TED profiles, `-` for none.
fn subtype_of(notice: &store::NoticeRef, parsed: &Parsed) -> String {
    if is_legacy_profile(&notice.profile) {
        return "legacy".to_owned();
    }
    first_code(parsed, SUBTYPE_FIELD).unwrap_or_else(|| "-".to_owned())
}

/// The eForms procedure type (BT-105).
const PROCEDURE_TYPE_FIELD: &str = "BT-105-Procedure";

/// Count one chunk of parsed notices (DE-1.x already folded).
async fn census_chunk(
    db: &store::Db,
    notices: &[(store::NoticeRef, Parsed)],
    report: &mut BuyerRoleCensus,
) -> turso::Result<()> {
    let ids: Vec<i64> = notices.iter().map(|(n, _)| n.id).collect();
    let orgs = Box::pin(db.mentions_by_ids(&ids)).await?;
    for (notice, parsed) in notices {
        let parties = notice_parties(is_sdk01_profile(&notice.profile), notice.id, parsed, orgs.get(&notice.id));
        let procedure_type = first_code(parsed, PROCEDURE_TYPE_FIELD).unwrap_or_else(|| "-".to_owned());
        report.classify(notice, &subtype_of(notice, parsed), &procedure_type, &parties);
    }
    Ok(())
}

/// Issue 483 unit 1: the buyer-role census over the parsed notices, one
/// [`ROLE_CENSUS_WINDOW`]-id window in every `stride` (0 reads as 1). Read-only. `stop`
/// is read before every window and every chunk, and a stopped census is a prefix
/// (`stopped`), which the caller does not store.
pub async fn buyer_role_census(
    db: &store::Db,
    stride: u64,
    stop: &(dyn Fn() -> bool + Sync),
    progress: impl FnMut(&BuyerRoleCensus),
) -> turso::Result<BuyerRoleCensus> {
    buyer_role_census_windowed(db, ROLE_CENSUS_WINDOW, ROLE_CENSUS_CHUNK, stride, stop, progress).await
}

/// [`buyer_role_census`] with explicit window and chunk sizes, for the tests.
pub async fn buyer_role_census_windowed(
    db: &store::Db,
    window: i64,
    chunk: i64,
    stride: u64,
    stop: &(dyn Fn() -> bool + Sync),
    mut progress: impl FnMut(&BuyerRoleCensus),
) -> turso::Result<BuyerRoleCensus> {
    debug_assert!(window > 0 && chunk > 0, "a non-positive window could not advance the cursor");
    let stride = stride.max(1);
    let mut report = BuyerRoleCensus::new(stride);
    report.target = db.max_parsed_notice_id().await?;
    'walk: while report.cursor < report.target {
        if stop() {
            report.stopped = true;
            break;
        }
        let hi = report.cursor.saturating_add(window).min(report.target);
        if report.windows % stride == 0 {
            let mut after = report.cursor;
            loop {
                // Boxed (issue 467's stack budgets): the parse stays off the walk's frame.
                let mut notices = Box::pin(db.parsed_window(after, hi, chunk)).await?;
                let Some(last) = notices.last().map(|(n, _)| n.id) else { break };
                normalise_de1(&mut notices);
                Box::pin(census_chunk(db, &notices, &mut report)).await?;
                after = last;
                if (notices.len() as i64) < chunk {
                    break;
                }
                if stop() {
                    report.stopped = true;
                    break 'walk;
                }
            }
            report.windows_read += 1;
        }
        report.windows += 1;
        report.cursor = hi;
        progress(&report);
    }
    report.finish();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{ORG_COUNTRY_FIELD, ORG_NAME_FIELD};

    /// `(OPT-300/301 role field, organization section, name)` → a parsed eForms notice
    /// naming each organization once and referencing it from each role.
    fn notice(roles: &[(&str, &str, &str)]) -> Parsed {
        let mut parsed = Parsed {
            sections: vec![store::Section { id: "PROC".into(), kind: "Notice".into(), parent: None }],
            values: Vec::new(),
        };
        for (field, section, name) in roles {
            if !parsed.sections.iter().any(|s| s.id == *section) {
                parsed.sections.push(store::Section {
                    id: (*section).into(),
                    kind: ORGANIZATION_KIND.into(),
                    parent: None,
                });
                for (f, v) in [
                    (ORG_NAME_FIELD, NoticeValue::Text { value: (*name).into(), lang: None }),
                    (ORG_COUNTRY_FIELD, NoticeValue::Code { list: None, code: "POL".into() }),
                ] {
                    parsed.values.push(store::ValueRow { section_id: (*section).into(), field_id: f.into(), ordinal: 0, value: v });
                }
            }
            parsed.values.push(store::ValueRow {
                section_id: "PROC".into(),
                field_id: (*field).into(),
                ordinal: 0,
                value: NoticeValue::Id { scheme: None, value: (*section).into(), is_ref: true },
            });
        }
        parsed
    }

    fn verdicts(roles: &[(&str, &str, &str)], orgs: &[(&str, i64)]) -> Vec<(String, Vec<&'static str>, bool)> {
        let orgs: HashMap<String, i64> = orgs.iter().map(|(s, o)| ((*s).to_owned(), *o)).collect();
        let parties = notice_parties(false, 1, &notice(roles), Some(&orgs));
        judge(&parties)
            .into_iter()
            .map(|(i, v)| {
                let classes =
                    ROLE_CENSUS_CLASSES.iter().enumerate().filter(|(c, _)| v.has(*c)).map(|(_, (n, _))| *n).collect();
                (parties[i].name.clone(), classes, v.clean())
            })
            .collect()
    }

    const BUYER: &str = "OPT-300-Procedure-Buyer";
    const TENDERER: &str = "OPT-300-Tenderer";

    /// The 482 read's shapes, each as the notice publishes it (16698, 299165 and 438807
    /// read live from `/v1/tenders/<id>` on 2026-10-03).
    #[test]
    fn the_482_mis_tags_are_flagged_and_a_real_buyer_stays_clean() {
        // 16698: the roles SWAPPED — Ratio Web (buyer and signatory) is the company, the
        // Instytut Adama Mickiewicza the tenderer. No contractor class can see it.
        let ratio = "Ratio Web Spółka z ograniczoną odpowiedzialnością";
        let v = verdicts(
            &[
                (BUYER, "ORG-1", ratio),
                ("OPT-300-Contract-Signatory", "ORG-1", ratio),
                ("OPT-300-Procedure-SProvider", "ORG-4", "Publications Office of the European Union"),
                ("OPT-301-Lot-ReviewInfo", "ORG-3", "Krajowa Izba Odwoławcza"),
                ("OPT-301-Lot-ReviewOrg", "ORG-3", "Krajowa Izba Odwoławcza"),
                (TENDERER, "ORG-2", "Instytut Adama Mickiewicza"),
            ],
            &[("ORG-1", 23_472_098), ("ORG-2", 23_327_423), ("ORG-3", 36), ("ORG-4", 12)],
        );
        assert_eq!(v, vec![(ratio.to_owned(), vec!["buyer-tenderer-swap"], false)]);
        // 299165: the same swap (ISS HS as buyer, the university hospital as tenderer).
        let v = verdicts(
            &[
                (BUYER, "ORG-1", "ISS HS Sp. z o.o."),
                ("OPT-301-Lot-ReviewOrg", "ORG-3", "Krajowa Izba Odwoławcza"),
                (TENDERER, "ORG-2", "Uniwersyteckie Centrum Kliniczne Warszawskiego Uniwersytetu Medycznego"),
            ],
            &[("ORG-1", 15_556_355), ("ORG-2", 6_284), ("ORG-3", 36)],
        );
        assert_eq!(v[0].1, vec!["buyer-tenderer-swap"]);
        assert!(!v[0].2);
        // 438807: the UZP appeals department in the buyer slot (also mediator and appeals
        // information); the real buyer POLREGIO only receives tenders and pays.
        let uzp = "Urząd Zamówień Publicznych Departament Odwołań";
        let polregio = "POLREGIO S.A ul. Kolejowa 1 , 01-217 Warszawa";
        let v = verdicts(
            &[
                (BUYER, "ORG-1", uzp),
                ("OPT-301-Lot-Mediator", "ORG-1", uzp),
                ("OPT-301-Lot-ReviewInfo", "ORG-1", uzp),
                ("OPT-301-Lot-AddInfo", "ORG-2", polregio),
                ("OPT-301-Lot-TenderReceipt", "ORG-2", polregio),
                ("OPT-301-LotResult-Financing", "ORG-2", polregio),
                ("OPT-301-LotResult-Paying", "ORG-2", polregio),
                ("OPT-301-Lot-ReviewOrg", "ORG-3", "Krajowa Izba Odwoławcza"),
                (TENDERER, "ORG-4", "Serwis Pojazdów Szynowych sp. z o.o. spółka komandytowa"),
            ],
            &[("ORG-1", 791), ("ORG-2", 6_465), ("ORG-3", 36), ("ORG-4", 13_279_016)],
        );
        assert_eq!(v, vec![(uzp.to_owned(), vec!["real-buyer-elsewhere", "review-body-name", "review-info-role"], false)]);
        let parties = notice_parties(
            false,
            1,
            &notice(&[(BUYER, "ORG-1", uzp), ("OPT-301-LotResult-Paying", "ORG-2", polregio)]),
            Some(&[("ORG-2".to_owned(), 6_465)].into_iter().collect()),
        );
        assert_eq!(judge(&parties)[0].1.basis[0], format!("real-buyer-elsewhere: {polregio} (org 6465)"), "names the buyer");
        // One Organization referenced as buyer and as tenderer.
        let v = verdicts(&[(BUYER, "ORG-1", ratio), (TENDERER, "ORG-1", ratio)], &[("ORG-1", 7)]);
        assert_eq!(v, vec![(ratio.to_owned(), vec!["contractor-same-section"], false)]);
        // Two sections the resolver bound to one organization, one name.
        let v = verdicts(
            &[(BUYER, "ORG-1", "Naprzód Catering Sp. z o.o."), (TENDERER, "ORG-2", "NAPRZÓD CATERING sp. z o.o.")],
            &[("ORG-1", 9), ("ORG-2", 9)],
        );
        assert_eq!(v[0].1, vec!["contractor-org-same-name"]);
        // One organization under two names (a resolver fusion, an Eigenbetrieb): counted,
        // not decisive.
        let v = verdicts(
            &[(BUYER, "ORG-1", "Stadt Musterstadt"), (TENDERER, "ORG-2", "Stadtentwässerung Musterstadt")],
            &[("ORG-1", 9), ("ORG-2", 9)],
        );
        assert_eq!(v, vec![("Stadt Musterstadt".to_owned(), vec!["contractor-org-other-name"], true)]);
        // No resolved organization (a mention the resolver has not bound): the name.
        let v = verdicts(&[(BUYER, "ORG-1", "DOL-TRANS-TOUR"), (TENDERER, "ORG-2", "Dol-Trans-Tour")], &[]);
        assert_eq!(v[0].1, vec!["contractor-name"]);
        // 533381 / 159306: a review body in the buyer slot that is also the notice's review
        // body; the name alone (a court buying for itself) is counted, not decisive.
        for court in [
            "Tribunal Català de Contractes del Sector Públic",
            "Úřad pro ochranu hospodářské soutěže",
            "Krajowa Izba Odwoławcza",
            "Vergabekammer des Bundes",
            "Förvaltningsrätten i Stockholm",
            "Tribunal Administrativo Central de Recursos Contractuales",
        ] {
            let v = verdicts(&[(BUYER, "ORG-1", court), ("OPT-301-Lot-ReviewOrg", "ORG-1", court)], &[]);
            assert_eq!(v, vec![(court.to_owned(), vec!["review-body-name", "review-body-role"], false)], "{court}");
            let v = verdicts(&[(BUYER, "ORG-1", court)], &[]);
            assert_eq!(v, vec![(court.to_owned(), vec!["review-body-name-alone"], true)], "{court}");
            // Beside a real buyer, the court mention is decisive and the buyer stays clean.
            let v = verdicts(&[(BUYER, "ORG-1", court), (BUYER, "ORG-2", "Gmina Olkusz")], &[]);
            assert_eq!(v[0].1, vec!["review-body-name"], "{court}");
            assert_eq!(v[1], ("Gmina Olkusz".to_owned(), vec![], true));
        }
        // 198229: the eSender in the buyer slot, and European Dynamics by name too.
        let v = verdicts(
            &[(BUYER, "ORG-1", "European Dynamics S.A."), ("OPT-300-Procedure-SProvider", "ORG-1", "European Dynamics S.A.")],
            &[],
        );
        assert_eq!(v[0].1, vec!["esender", "platform-name"]);
        // A real buyer that is its own review body and documents provider stays clean.
        let v = verdicts(
            &[
                (BUYER, "ORG-1", "Gmina Olkusz"),
                ("OPT-301-Lot-ReviewOrg", "ORG-1", "Gmina Olkusz"),
                ("OPT-301-Lot-DocProvider", "ORG-1", "Gmina Olkusz"),
                (TENDERER, "ORG-2", "Budimex S.A."),
            ],
            &[("ORG-1", 1), ("ORG-2", 2)],
        );
        assert_eq!(v, vec![("Gmina Olkusz".to_owned(), vec!["review-body-role", "docs-provider"], true)]);
    }

    /// The swap reads legal forms: a municipal company buying from a research institute is
    /// no swap, a company awarding to a natural person is the weak class only, and two
    /// companies are neither.
    #[test]
    fn the_swap_needs_a_company_buyer_and_a_public_tenderer() {
        let v = verdicts(
            &[(BUYER, "ORG-1", "Miejskie Przedsiębiorstwo Komunikacji Sp. z o.o."), (TENDERER, "ORG-2", "Instytut Kolejnictwa")],
            &[],
        );
        assert_eq!(v[0].1, Vec::<&str>::new(), "a public-owned company is public-shaped");
        let v = verdicts(&[(BUYER, "ORG-1", "POLREGIO S.A."), (TENDERER, "ORG-2", "Jan Kowalski")], &[]);
        assert_eq!(v, vec![("POLREGIO S.A.".to_owned(), vec!["swap-legal-form"], true)]);
        let v = verdicts(&[(BUYER, "ORG-1", "POLREGIO S.A."), (TENDERER, "ORG-2", "Budimex S.A.")], &[]);
        assert_eq!(v[0].1, Vec::<&str>::new());
        assert!(commercial(&fold("Ratio Web Sp. z o.o.")) && commercial(&fold("Krajská zdravotní, a.s.")));
        assert!(!commercial(&fold("Instytut Adama Mickiewicza")) && !commercial(&fold("Sagan")));
        assert!(public(&fold("Uniwersyteckie Centrum Kliniczne")) && !public(&fold("Ratio Web")));
    }

    /// The decided case: a buyer that is also a tenderer — of another lot — is flagged,
    /// and a notice that also names its real buyer keeps a clean one. A synthetic shape
    /// (16698 itself is the swap above).
    #[test]
    fn a_buyer_tendering_in_another_lot_is_flagged_but_a_real_buyer_is_left() {
        let mut parsed = notice(&[
            (BUYER, "ORG-1", "Instytut Adama Mickiewicza"),
            (BUYER, "ORG-2", "Ratio Web Sp. z o.o."),
            (TENDERER, "ORG-2", "Ratio Web Sp. z o.o."),
        ]);
        // The tenderer reference sits in lot 2's result graph; scope does not matter.
        parsed.sections.push(store::Section { id: "LOT-2".into(), kind: "Lot".into(), parent: Some("PROC".into()) });
        parsed.values.last_mut().expect("the tenderer ref").section_id = "LOT-2".into();
        let parties = notice_parties(false, 1, &parsed, None);
        let mut report = BuyerRoleCensus::new(1);
        let n = store::NoticeRef { id: 16_698, source: "ted".into(), publication_id: "00016698-2024".into(), profile: "eforms:eforms-sdk-1.10".into() };
        report.classify(&n, "29", "open", &parties);
        assert_eq!((report.notices_with_buyers, report.buyer_mentions, report.no_clean_buyer), (1, 2, 0));
        let c = &report.classes["contractor-same-section"];
        assert_eq!((c.mentions, c.notices, c.no_clean_buyer, c.every_buyer), (1, 1, 0, 0));
        assert_eq!(c.samples[0].flagged, "Ratio Web Sp. z o.o.");
        assert_eq!(c.samples[0].procedure_type, "open");
        assert_eq!(c.samples[0].other_buyers, vec!["Instytut Adama Mickiewicza"]);
        assert!(c.samples[0].clean_buyer_left);
        assert_eq!(report.cells.get("ted/29/contractor-same-section"), Some(&1));
        assert_eq!(report.procedures.get("open/contractor-same-section"), Some(&1));
        assert_eq!(report.cells.get("ted/29/no-clean-buyer"), None);
        // The contractor alone in the slot: no clean buyer left.
        let parties = notice_parties(
            false,
            2,
            &notice(&[(BUYER, "ORG-2", "Ratio Web Sp. z o.o."), (TENDERER, "ORG-2", "Ratio Web Sp. z o.o.")]),
            None,
        );
        report.classify(&store::NoticeRef { id: 2, ..n.clone() }, "29", "-", &parties);
        assert_eq!(report.no_clean_buyer, 1);
        assert_eq!(report.cells.get("ted/29/no-clean-buyer"), Some(&1));
        assert_eq!(report.classes["contractor-same-section"].every_buyer, 1);
        // The name-list classes count by pattern label.
        let parties = notice_parties(false, 3, &notice(&[(BUYER, "ORG-1", "Vergabekammer Südbayern")]), None);
        report.classify(&store::NoticeRef { id: 3, ..n }, "29", "-", &parties);
        assert_eq!(report.patterns.get("review-body-name-alone/DE Vergabekammer"), Some(&1));
    }

    /// Legacy: the review body proper is a review body, the appeal-information block (very
    /// often the buyer itself) is review-adjacent; and the legacy winner block is a
    /// contractor. eForms: the Part- and ReviewBody roles count, `ReviewOrg` is no role.
    #[test]
    fn the_role_vocabulary_keeps_review_info_apart_from_the_review_body() {
        assert_eq!(role_kind("TED-ADDRESS_REVIEW_BODY"), Some(RoleKind::ReviewBody));
        assert_eq!(role_kind("TED-ADDRESS_REVIEW_INFO"), Some(RoleKind::ReviewAdjacent));
        assert_eq!(role_kind("TED-ADDRESS_CONTRACTING_BODY"), Some(RoleKind::Buyer));
        assert_eq!(role_kind("TED-ADDRESS_CONTRACTOR"), Some(RoleKind::Contractor));
        assert_eq!(role_kind("TED-ADDRESS_PARTICIPATION"), Some(RoleKind::BuyerShaped));
        assert_eq!(role_kind("OPT-301-Lot-ReviewInfo"), Some(RoleKind::ReviewAdjacent));
        assert_eq!(role_kind("OPT-301-Lot-Mediator"), Some(RoleKind::ReviewAdjacent));
        assert_eq!(role_kind("OPT-301-Part-ReviewOrg"), Some(RoleKind::ReviewBody));
        assert_eq!(role_kind("OPT-301-ReviewBody"), Some(RoleKind::ReviewBody));
        assert_eq!(role_kind("OPT-301-Part-DocProvider"), Some(RoleKind::DocsProvider));
        assert_eq!(role_kind("OPT-300-Contract-Signatory"), Some(RoleKind::BuyerShaped));
        assert_eq!(role_kind("OPT-301-ReviewOrg"), None);
    }

    /// Every eForms row of [`ROLE_KINDS`] (an upper-case first letter; the lower-case rows
    /// are [`role_name`]'s legacy names) is an OPT-300/301 role some SDK defines.
    #[test]
    fn every_eforms_role_row_is_a_role_of_the_sdk() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/sdk");
        let mut sdk = String::new();
        for entry in std::fs::read_dir(dir).expect("the sdk dir") {
            let path = entry.expect("an sdk entry").path();
            if path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("fields-")) {
                sdk.push_str(&std::fs::read_to_string(&path).expect("an sdk fields file"));
            }
        }
        for (role, _) in ROLE_KINDS.iter().filter(|(r, _)| r.starts_with(|c: char| c.is_ascii_uppercase())) {
            assert!(
                sdk.contains(&format!("\"OPT-300-{role}\"")) || sdk.contains(&format!("\"OPT-301-{role}\"")),
                "{role} is no OPT-300/301 role of any SDK"
            );
        }
    }

    /// The patterns are data written folded, and match whole words only.
    #[test]
    fn the_name_patterns_are_folded_whole_word_phrases() {
        for (label, phrase, _) in NAME_PATTERNS {
            assert_eq!(fold(phrase), *phrase, "{label}: the phrase must be written as its own fold");
        }
        assert_eq!(name_pattern(&fold("KIO"), NameList::ReviewBody), Some("PL KIO"));
        assert_eq!(name_pattern(&fold("Kiosk Miejski"), NameList::ReviewBody), None, "whole words only");
        assert_eq!(name_pattern(&fold("Stadtwerke Tarp"), NameList::ReviewBody), None);
        assert_eq!(name_pattern(&fold("T.A.R. Lazio"), NameList::ReviewBody), None, "dotted initials are not a word");
        assert_eq!(name_pattern(&fold("TAR Lazio - Roma"), NameList::ReviewBody), Some("IT TAR"));
        assert_eq!(name_pattern(&fold("Commissione Europea"), NameList::ReviewBody), None);
        assert_eq!(
            name_pattern(&fold("Urząd Zamówień Publicznych Departament Odwołań"), NameList::ReviewBody),
            Some("PL UZP Departament Odwołań")
        );
        assert_eq!(name_pattern(&fold("Urząd Zamówień Publicznych"), NameList::ReviewBody), None);
        for form in COMMERCIAL_FORMS {
            assert_eq!(fold(form), *form, "{form}: written as its own fold");
        }
        for stem in PUBLIC_STEMS {
            assert_eq!(fold(stem), *stem, "{stem}: written as its own fold");
        }
    }

    #[test]
    fn a_class_keeps_the_smallest_ranks_whatever_the_order() {
        let sample = |id: i64| RoleCensusSample {
            notice_id: id,
            publication: String::new(),
            subtype: String::new(),
            procedure_type: String::new(),
            flagged: String::new(),
            basis: Vec::new(),
            other_buyers_total: 0,
            other_buyers: Vec::new(),
            clean_buyer_left: false,
        };
        let (mut forward, mut backward) = (RoleCensusClass::default(), RoleCensusClass::default());
        for id in 0..200 {
            forward.offer(sample_rank(id), &sample(id));
            backward.offer(sample_rank(199 - id), &sample(199 - id));
        }
        forward.sort();
        backward.sort();
        assert_eq!(forward.samples.len(), ROLE_CENSUS_SAMPLES);
        assert_eq!(forward.samples, backward.samples);
    }
}
