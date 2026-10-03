//! Issue 483 unit 1: the buyer-role census.
//!
//! Award notices sometimes put the contractor, a review body or a platform vendor in the
//! buyer role (`Procedure-Buyer` in eForms, `buyer` in the legacy and sdk-0.1 dialects:
//! [`super::BUYER_ROLES`]). The 482 two-cluster read found 16 of its 45 false splits were such
//! mis-tags (16698's CAN names its contractor Ratio Web as the buyer; 533381 the Tribunal
//! Catalán de Contratos; 198229 European Dynamics). Every buyer-based guard trusts that
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
}

impl RoleKind {
    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// The census's role vocabulary: the role a reference names ([`role_name`]: the eForms
/// OPT-300/301 suffix, or the legacy element folded onto the canonical name) to what it
/// means here. Data, not branches: a role the census should read is a row.
const ROLE_KINDS: &[(&str, RoleKind)] = &[
    ("Procedure-Buyer", RoleKind::Buyer),
    ("buyer", RoleKind::Buyer),
    ("Tenderer", RoleKind::Contractor),
    ("Tenderer-MainCont", RoleKind::Contractor),
    ("Tenderer-SubCont", RoleKind::Contractor),
    ("winner", RoleKind::Contractor),
    ("Lot-ReviewOrg", RoleKind::ReviewBody),
    ("ReviewOrg", RoleKind::ReviewBody),
    ("Procedure-SProvider", RoleKind::Esender),
    ("Lot-DocProvider", RoleKind::DocsProvider),
    ("specifications-provider", RoleKind::DocsProvider),
];

/// Legacy address blocks read by their own element name, before [`role_name`]'s fold:
/// the fold puts `ADDRESS_REVIEW_INFO` (where to get information about appeals — very
/// often the buyer itself) under `review-body` beside the review body proper, and the
/// census must keep the two apart. `None`: not a role this census reads.
const LEGACY_ELEMENT_KINDS: &[(&str, Option<RoleKind>)] = &[
    ("ADDRESS_REVIEW_BODY", Some(RoleKind::ReviewBody)),
    ("APPEAL_PROCEDURE_BODY_RESPONSIBLE", Some(RoleKind::ReviewBody)),
    ("RESPONSIBLE_FOR_APPEAL_PROCEDURES", Some(RoleKind::ReviewBody)),
    ("ADDRESS_REVIEW_INFO", None),
];

/// The census kind of a role reference's field id.
fn role_kind(field_id: &str) -> Option<RoleKind> {
    if let Some(element) = field_id.strip_prefix("TED-")
        && let Some((_, kind)) = LEGACY_ELEMENT_KINDS.iter().find(|(e, _)| *e == element)
    {
        return *kind;
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
/// Deliberately left out: `Commissione` (the European Commission is a real buyer), the
/// Polish Urząd Zamówień Publicznych (it runs e-Zamówienia but also buys for itself; the
/// eSender role catches it where it was the platform), `Lot-ReviewInfo` bodies.
const NAME_PATTERNS: &[(&str, &str, NameList)] = &[
    ("PL KIO", "krajowa izba odwolawcza", NameList::ReviewBody),
    ("PL KIO", "kio", NameList::ReviewBody),
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

/// The flag classes, in report order: `(name, decisive)`. A decisive class makes the
/// mention not a clean buyer; a non-decisive one is counted and sampled only.
///
/// - `contractor-org`: the buyer's resolved organization (or its own section) is also a
///   winner, tenderer or contractor on this notice — in ANY lot. Decided, not missed: a
///   procedure's buyer is never its own supplier, so a buyer that is a tenderer of another
///   lot is still the contractor in the buyer slot; the shape where the notice ALSO names
///   its real buyer is told apart by `no_clean_buyer`, not by dropping the flag.
/// - `contractor-name`: no organization match, but the folded name equals a contractor's.
/// - `review-body-name`: the name is a known review body ([`NAME_PATTERNS`]).
/// - `review-body-role`: the notice's own review-body role names the same organization or
///   name. NOT decisive: a buyer filling its own name into the review-body block is a
///   mis-tag of THAT role, and it is common (UK and IE notices list themselves); the name
///   list is what says the buyer slot holds a court.
/// - `esender`: the buyer is the notice's eSender / procurement service provider.
/// - `docs-provider`: the buyer is the documents provider. NOT decisive: a buyer handing
///   out its own documents is the normal case; counted to size it, never a mis-tag alone.
/// - `platform-name`: the name is a known platform vendor ([`NAME_PATTERNS`]).
pub const ROLE_CENSUS_CLASSES: [(&str, bool); 7] = [
    ("contractor-org", true),
    ("contractor-name", true),
    ("review-body-name", true),
    ("review-body-role", false),
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
    /// `by_org` is the organization half alone.
    fn same(&self, other: &Party, by_org: bool) -> bool {
        match (self.org, other.org) {
            (Some(a), Some(b)) if a == b => true,
            _ if by_org => false,
            _ => !self.folded.is_empty() && self.folded == other.folded,
        }
    }
}

/// A buyer mention's verdict: its classes (one bit per [`ROLE_CENSUS_CLASSES`] index) and
/// what each matched.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Verdict {
    classes: u8,
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
    let (contractor_org, contractor_name) = (class("contractor-org"), class("contractor-name"));
    let (review_name, review_role) = (class("review-body-name"), class("review-body-role"));
    let (esender, docs, platform) = (class("esender"), class("docs-provider"), class("platform-name"));
    let label = |p: &Party| if p.name.trim().is_empty() { "(no name)".to_owned() } else { p.name.trim().to_owned() };
    let mut out = Vec::new();
    for (i, buyer) in parties.iter().enumerate().filter(|(_, p)| p.is(RoleKind::Buyer)) {
        let mut v = Verdict::default();
        // The first party in `kind` that is the buyer itself (its own section carrying
        // both roles), then one with the same organization, then one with the same name.
        let find = |kind: RoleKind, by_org: bool| -> Option<&Party> {
            if buyer.is(kind) {
                return Some(buyer);
            }
            parties.iter().enumerate().find(|(j, p)| *j != i && p.is(kind) && buyer.same(p, by_org)).map(|(_, p)| p)
        };
        if let Some(p) = find(RoleKind::Contractor, true) {
            v.flag(contractor_org, format!("{} (org {})", label(p), p.org.map_or("-".to_owned(), |o| o.to_string())));
        } else if let Some(p) = find(RoleKind::Contractor, false) {
            v.flag(contractor_name, label(p));
        }
        if let Some(pattern) = name_pattern(&buyer.folded, NameList::ReviewBody) {
            v.flag(review_name, pattern.to_owned());
        }
        if let Some(p) = find(RoleKind::ReviewBody, false) {
            v.flag(review_role, label(p));
        }
        if let Some(p) = find(RoleKind::Esender, false) {
            v.flag(esender, label(p));
        }
        if let Some(p) = find(RoleKind::DocsProvider, false) {
            v.flag(docs, label(p));
        }
        if let Some(pattern) = name_pattern(&buyer.folded, NameList::Platform) {
            v.flag(platform, pattern.to_owned());
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
    /// The flagged buyer's name.
    pub flagged: String,
    /// What each of its classes matched (`contractor-org: Ratio Web … (org 123)`).
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
    fn classify(&mut self, notice: &store::NoticeRef, subtype: &str, parties: &[Party]) {
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
        report.classify(notice, &subtype_of(notice, parsed), &parties);
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

    /// The 482 read's shapes, each as the notice publishes it.
    #[test]
    fn the_482_mis_tags_are_flagged_and_a_real_buyer_stays_clean() {
        // 16698: the CAN's buyer slot names the contractor — the SAME section is both.
        let ratio = "Ratio Web Spółka z ograniczoną odpowiedzialnością";
        let v = verdicts(&[(BUYER, "ORG-1", ratio), (TENDERER, "ORG-1", ratio)], &[("ORG-1", 7)]);
        assert_eq!(v, vec![(ratio.to_owned(), vec!["contractor-org"], false)]);
        // 299165's shape with two sections the resolver bound to one organization.
        let v = verdicts(
            &[(BUYER, "ORG-1", "Naprzód Catering Sp. z o.o."), (TENDERER, "ORG-2", "NAPRZÓD CATERING sp. z o.o.")],
            &[("ORG-1", 9), ("ORG-2", 9)],
        );
        assert_eq!(v[0].1, vec!["contractor-org"]);
        // No resolved organization (a mention the resolver has not bound): the name.
        let v = verdicts(&[(BUYER, "ORG-1", "DOL-TRANS-TOUR"), (TENDERER, "ORG-2", "Dol-Trans-Tour")], &[]);
        assert_eq!(v[0].1, vec!["contractor-name"]);
        // 533381 / 159306: a review body in the buyer slot, by name alone.
        for court in [
            "Tribunal Català de Contractes del Sector Públic",
            "Úřad pro ochranu hospodářské soutěže",
            "Krajowa Izba Odwoławcza",
            "Vergabekammer des Bundes",
            "Förvaltningsrätten i Stockholm",
            "Tribunal Administrativo Central de Recursos Contractuales",
        ] {
            let v = verdicts(&[(BUYER, "ORG-1", court)], &[]);
            assert_eq!(v, vec![(court.to_owned(), vec!["review-body-name"], false)], "{court}");
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

    /// The decided case: a buyer that is also a tenderer — of another lot — is flagged,
    /// and a notice that also names its real buyer keeps a clean one.
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
        report.classify(&n, "29", &parties);
        assert_eq!((report.notices_with_buyers, report.buyer_mentions, report.no_clean_buyer), (1, 2, 0));
        let c = &report.classes["contractor-org"];
        assert_eq!((c.mentions, c.notices, c.no_clean_buyer, c.every_buyer), (1, 1, 0, 0));
        assert_eq!(c.samples[0].flagged, "Ratio Web Sp. z o.o.");
        assert_eq!(c.samples[0].other_buyers, vec!["Instytut Adama Mickiewicza"]);
        assert!(c.samples[0].clean_buyer_left);
        assert_eq!(report.cells.get("ted/29/contractor-org"), Some(&1));
        assert_eq!(report.cells.get("ted/29/no-clean-buyer"), None);
        // The contractor alone in the slot: no clean buyer left.
        let parties = notice_parties(
            false,
            2,
            &notice(&[(BUYER, "ORG-2", "Ratio Web Sp. z o.o."), (TENDERER, "ORG-2", "Ratio Web Sp. z o.o.")]),
            None,
        );
        report.classify(&store::NoticeRef { id: 2, ..n }, "29", &parties);
        assert_eq!(report.no_clean_buyer, 1);
        assert_eq!(report.cells.get("ted/29/no-clean-buyer"), Some(&1));
        assert_eq!(report.classes["contractor-org"].every_buyer, 1);
    }

    /// Legacy: the review body proper flags, the appeal-information block (very often the
    /// buyer itself) does not; and the legacy winner block is a contractor.
    #[test]
    fn legacy_review_info_is_not_a_review_body() {
        assert_eq!(role_kind("TED-ADDRESS_REVIEW_BODY"), Some(RoleKind::ReviewBody));
        assert_eq!(role_kind("TED-ADDRESS_REVIEW_INFO"), None);
        assert_eq!(role_kind("TED-ADDRESS_CONTRACTING_BODY"), Some(RoleKind::Buyer));
        assert_eq!(role_kind("TED-ADDRESS_CONTRACTOR"), Some(RoleKind::Contractor));
        assert_eq!(role_kind("OPT-301-Lot-ReviewInfo"), None);
        assert_eq!(role_kind("OPT-300-Contract-Signatory"), None);
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
    }

    #[test]
    fn a_class_keeps_the_smallest_ranks_whatever_the_order() {
        let sample = |id: i64| RoleCensusSample {
            notice_id: id,
            publication: String::new(),
            subtype: String::new(),
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
