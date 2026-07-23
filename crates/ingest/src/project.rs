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

use std::collections::{BTreeMap, BTreeSet, HashMap};
use store::{
    BidParty, BidState, ContractState, Db, Fact, Identifier, LotResultState, LotState, Mention,
    NoticeValue, Parsed, Round, TenderProjection, TenderVersion,
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
const TEXTS: &[(&str, &str)] = &[
    ("BT-21", "title"),
    ("BT-24", "description"),
    // legacy R2.0.7–R2.0.9 (title 100% fill, research §5.1)
    ("TED-TITLE", "title"),
    ("TED-TITLE_CONTRACT", "title"),
    ("TED-CONTRACT_TITLE", "title"),
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
];
const AMOUNTS: &[(&str, &str)] = &[
    ("BT-27", "estimated_value"),
    ("BT-271", "framework_maximum"),
    ("BT-161", "result_value"),
    // legacy
    ("TED-VAL_ESTIMATED_TOTAL", "estimated_value"),
    ("TED-VAL_TOTAL", "result_value"),
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
    // legacy: the submission deadline (100% fill on F02) and its openings
    ("TED-DATE_RECEIPT_TENDERS", "submission_deadline"),
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

/// Sections that are Lots in the canonical sense — Parts and LotsGroups are
/// Lots with a kind flag (CONTEXT.md).
const LOT_KINDS: &[&str] = &["Lot", "LotsGroup", "Part"];

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
const PUBLICATION_DATE_FIELDS: &[&str] = &[
    "OPP-012-notice",                 // eForms efbc:PublicationDate (TED; DÖE when stamped)
    "TED-DATE_PUB",                   // legacy r208/r209 REF_OJS publication date
    "TXT-PD",                         // text-era PD: publication date
    "BT-738-notice",                  // eForms RequestedPublicationDate — the DÖE portal date
    "SDK01-RequestedPublicationDate", // DÖE sdk-0.1 requested publication date
];

/// Dispatch-date fields, best-first (issue 18): when the notice left the
/// sender. Kept as its own axis because ordering within a publication day, and
/// the dispatch-vs-publication skew itself, are real questions consumers ask.
const DISPATCH_DATE_FIELDS: &[&str] = &[
    "BT-05(a)-notice",          // eForms cbc:IssueDate (TED + DÖE eforms-de)
    "SDK01-IssueDate",          // DÖE sdk-0.1 issue date
    "TED-DS_DATE_DISPATCH",     // legacy dispatch (CODIF_DATA)
    "TED-DATE_DISPATCH_NOTICE", // legacy dispatch (form body)
    "TED-DATE_DISP",            // INTERNAL_OJS 2008 dispatch (BIB_DOC_S)
    "TXT-DS",                   // text-era DS: dispatch
];

/// The notice's own OJS publication number, in-form (r209 emits it as a plain
/// `NO_DOC_OJS`; the text era's own id is `ND:`). Only used as a corroborating
/// node source — the publication id is the authoritative one.
const LEGACY_OWN_NUMBER_FIELDS: &[&str] = &["TED-NO_DOC_OJS", "TXT-ND"];

/// Received-bid count fields (research §5.1) — one legacy statistic, mapped to
/// the eForms `tenders` received-submission kind.
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
const SDK01_PARTY_COUNTRY_FIELDS: &[&str] = &[
    "SDK01-ContractingParty-Party-PostalAddress-Country-IdentificationCode",
    "SDK01-TenderResult-WinningParty-Party-PostalAddress-Country-IdentificationCode",
];
/// sdk-0.1's award-decision code, on the `TenderResult` section.
const SDK01_RESULT_CODE_FIELD: &str = "SDK01-TenderResult-TenderResultCode";
/// sdk-0.1's procedure folder id (its BT-04 analogue). Only a genuine uuid is a
/// strong-enough cross-reference to key a Tender on (issue 34); the numeric
/// channel's non-uuid folder ids are notice-local and stay islands.
const SDK01_FOLDER_FIELD: &str = "SDK01-ContractFolderID";

const PROCEDURE_KEY_FIELD: &str = "BT-04-notice";
const LOGICAL_NOTICE_FIELD: &str = "BT-701-notice";
const SUBTYPE_FIELD: &str = "OPP-070-notice";
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
    // The projection writes a self-consistent graph by construction, so it runs
    // with FK enforcement off (issue 19) — the per-row FK check on millions of
    // satellite inserts is the projection's super-linear cost at scale — and
    // restores it unconditionally, so no other write path loses the guard.
    db.set_foreign_keys(false).await?;
    let result = project_inner(db, rebuild).await;
    let restored = db.set_foreign_keys(true).await;
    let report = result?;
    restored?;
    Ok(report)
}

async fn project_inner(db: &Db, rebuild: bool) -> turso::Result<Report> {
    if rebuild {
        db.clear_canonical().await?;
    }
    let now = store::now_unix();
    let mut report = Report::default();

    // Phase 1 — read the notice-parsed layer in id-ordered chunks, each a
    // handful of scans rather than a per-notice query storm (issue 19). Chunking
    // bounds memory: only the compact per-notice states (needed whole for
    // grouping) stay in RAM, never the whole raw layer at once. Each notice's
    // mentions go into one flat list; its slice is remembered so the resolved
    // organization ids bind back.
    const READ_CHUNK: i64 = 10_000;
    let t0 = std::time::Instant::now();
    let mut states = Vec::new();
    let mut all_mentions: Vec<Mention> = Vec::new();
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    let mut after_id = 0i64;
    loop {
        let chunk = db.parsed_chunk(after_id, READ_CHUNK).await?;
        let Some((last, _)) = chunk.last() else { break };
        after_id = last.id;
        for (notice, parsed) in chunk {
            let mut state = NoticeState::read(&notice, &parsed);
            let mentions = state.take_mentions(notice.id, &parsed);
            let start = all_mentions.len();
            all_mentions.extend(mentions);
            ranges.push(start..all_mentions.len());
            states.push(state);
        }
    }
    report.notices = states.len() as u64;
    report.mentions = all_mentions.len() as u64;
    eprintln!(
        "[project] read: {} notices, {} mentions in {:.1}s",
        report.notices,
        report.mentions,
        t0.elapsed().as_secs_f64()
    );

    // Phase 2 — resolve every mention in bounded batches, then bind per notice.
    let t1 = std::time::Instant::now();
    let ids = db.resolve_mentions(&all_mentions, now).await?;
    for (state, range) in states.iter_mut().zip(&ranges) {
        state.bind_organizations(&all_mentions[range.clone()], &ids[range.clone()]);
    }
    eprintln!("[project] mentions: {} resolved in {:.1}s", ids.len(), t1.elapsed().as_secs_f64());

    // Phase 3 — group notices into Tenders (in memory).
    let t2 = std::time::Instant::now();
    let projections = group(states);
    let legacy_keys: BTreeSet<String> = projections
        .iter()
        .filter_map(|p| p.procedure_key.clone())
        .filter(|k| k.starts_with("ojs:"))
        .collect();
    for projection in &projections {
        report.tenders += 1;
        report.islands += u64::from(projection.island_notice_id.is_some());
    }
    eprintln!("[project] group: {} tenders in {:.1}s", report.tenders, t2.elapsed().as_secs_f64());

    // Phase 4 — reconcile all Tenders in batched write transactions, then retire
    // any legacy Tender a late component-merge absorbed (its rows migrated to the
    // surviving key; here it gets `removed` change events).
    let t3 = std::time::Instant::now();
    report.applied = db.apply_tenders(&projections, now).await?;
    report.absorbed = db.retire_absorbed_legacy_tenders(&legacy_keys, now).await?;
    eprintln!("[project] apply: {} tenders in {:.1}s", report.tenders, t3.elapsed().as_secs_f64());
    Ok(report)
}

/// One notice read in canonical terms, before it is folded into a chain.
struct NoticeState {
    notice_id: i64,
    source: String,
    publication_id: String,
    /// True for the legacy TED profiles (text / ted-export-r208 / r209): these
    /// chain by transitive OJS-number closure, not by BT-04 (research §3).
    legacy: bool,
    /// True for the DÖE sdk-0.1 dialect: a distinct node vocabulary
    /// (`ContractingParty`/`TenderResult`/`WinningParty` in place of eForms'
    /// `Organization`/`LotResult`), read by its own party- and results-binding
    /// paths (issue 29).
    sdk01: bool,
    procedure_key: Option<String>,
    /// The notice's own OJS publication number `(year, number)` — the node in
    /// the legacy chain graph. `None` when the id does not parse (falls back to
    /// an island). eForms notices have none.
    ojs_self: Option<OjsKey>,
    /// Transitive chain edges: every `is_ref` OJS-scheme id the notice carries
    /// (`REF_NOTICE/NO_DOC_OJS`, `NOTICE_NUMBER_OJ`, text-era `RN`). A missed
    /// edge splits a Tender, never wrongly merges (research §3).
    ojs_edges: Vec<OjsKey>,
    published_at: i64,
    dispatched_at: Option<i64>,
    subtype: Option<String>,
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
        let mut facts = BTreeSet::new();
        let mut raw_roles = Vec::new();
        let mut ojs_edges = Vec::new();
        // sdk-0.1 names its buyer by the `ContractingParty` section itself, with
        // no OPT-300 role reference — synthesise the buyer role directly at it.
        if sdk01 {
            for s in &parsed.sections {
                if s.kind == SDK01_BUYER_KIND {
                    raw_roles.push((Scope::Tender, s.id.clone(), "buyer".to_owned(), s.id.clone()));
                }
            }
        }
        for value in &parsed.values {
            let scope = scope_of(&sections, &value.section_id);
            let field_id = value.field_id.as_str();
            let fact = match &value.value {
                NoticeValue::Text { lang, value: v } => canonical_name(TEXTS, field_id)
                    .map(|field| Fact::Text { field, lang: lang.clone(), value: v.clone() }),
                NoticeValue::Amount { cents, currency } => canonical_name(AMOUNTS, field_id)
                    .map(|field| Fact::Amount { field, cents: *cents, currency: currency.clone() }),
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
                    // An OJS-scheme reference is a chain edge; anything else is
                    // an inline organization role reference (legacy synthesises
                    // `ORG-n` refs, mirroring eForms' OPT-300 pattern).
                    if scheme.as_deref() == Some("ojs") {
                        if let Some(key) = ojs_key(target) {
                            ojs_edges.push(key);
                        }
                    } else if let Some(role) = role_name(&value.field_id) {
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

        // The node key: the notice's own publication number. The publication id
        // is authoritative (`000001-2019` DOC form / text-era `ND:`); the
        // in-form `NO_DOC_OJS` is the OJS-display corroboration.
        let ojs_self = legacy
            .then(|| {
                ojs_key(&notice.publication_id).or_else(|| {
                    LEGACY_OWN_NUMBER_FIELDS.iter().find_map(|f| first_id(parsed, f).and_then(|v| ojs_key(&v)))
                })
            })
            .flatten();
        ojs_edges.sort_unstable();
        ojs_edges.dedup();

        let (published_at, dispatched_at) = notice_instants(parsed);

        NoticeState {
            notice_id: notice.id,
            source: notice.source.clone(),
            publication_id: notice.publication_id.clone(),
            legacy,
            sdk01,
            procedure_key: procedure_key(parsed, sdk01),
            ojs_self,
            ojs_edges,
            published_at,
            dispatched_at,
            subtype: first_code(parsed, SUBTYPE_FIELD),
            logical_id: first_id(parsed, LOGICAL_NOTICE_FIELD),
            is_correction: parsed.sections.iter().any(|s| s.kind == "Change"),
            facts,
            lots: lots.into_values().collect(),
            roles,
            raw_results,
            round: None,
        }
    }

    /// Every Organization section of the notice, normalised for merging.
    ///
    /// An Organization's own values are spread over its subtree rather than
    /// sitting on the section itself: the name is on the Organization, but the
    /// official identifier hangs off its `CompanyLegalEntity` child (14 813 of
    /// them on the 2026-136 daily). So a mention collects from the whole
    /// subtree, keyed by the enclosing Organization.
    fn take_mentions(&mut self, notice_id: i64, parsed: &Parsed) -> Vec<Mention> {
        let sections: HashMap<&str, &store::Section> =
            parsed.sections.iter().map(|s| (s.id.as_str(), s)).collect();

        // sdk-0.1 has no eForms `Organization` sections: its buyer and winners
        // are the inline `ContractingParty`/`WinningParty` sections themselves,
        // each carrying its name on its direct Party subtree (issue 29).
        let mention_kinds: &[&str] = if self.sdk01 { SDK01_PARTY_KINDS } else { &[ORGANIZATION_KIND] };

        let mut mentions: BTreeMap<&str, Mention> = parsed
            .sections
            .iter()
            .filter(|s| mention_kinds.contains(&s.kind.as_str()))
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
                    },
                )
            })
            .collect();

        for value in &parsed.values {
            let Some(owner) = enclosing(&sections, &value.section_id, mention_kinds) else {
                continue;
            };
            let Some(mention) = mentions.get_mut(owner) else { continue };
            let field = value.field_id.as_str();
            // Three vocabularies read here: eForms hangs BT-501 off a
            // `CompanyLegalEntity` child; legacy uses inline `OFFICIALNAME` /
            // `COUNTRY` / `NATIONALID` address blocks (research §6); sdk-0.1 reads
            // the party section's *direct* name/country (never the nested
            // `ServiceProviderParty` eSender), and carries no official id there.
            let (is_name, is_country, is_id) = if self.sdk01 {
                (SDK01_PARTY_NAME_FIELDS.contains(&field), SDK01_PARTY_COUNTRY_FIELDS.contains(&field), false)
            } else {
                (
                    field == ORG_NAME_FIELD || ORG_NAME_FIELDS.contains(&field),
                    field == ORG_COUNTRY_FIELD || ORG_COUNTRY_FIELDS.contains(&field),
                    field == ORG_IDENTIFIER_FIELD || field == ORG_NATIONALID_FIELD,
                )
            };
            match &value.value {
                NoticeValue::Text { value, .. } if is_name && mention.name.is_empty() => {
                    mention.name.clone_from(value);
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
                m.identifier = m
                    .raw_identifier
                    .as_deref()
                    .and_then(|raw| normalise_identifier(raw, m.country.as_deref()));
                m
            })
            .collect()
    }

    /// Turn the notice-local role references into party facts — and the
    /// results graph into a bound Round — now that each mention has a
    /// canonical Organization.
    fn bind_organizations(&mut self, mentions: &[Mention], ids: &[i64]) {
        let by_section: HashMap<&str, i64> = mentions
            .iter()
            .map(|m| m.section_id.as_str())
            .zip(ids.iter().copied())
            .collect();
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

    fn kind(&self) -> &'static str {
        match self.subtype.as_deref() {
            Some(REGISTRATION_SUBTYPE) => "registration",
            _ => "procedure",
        }
    }
}

/// Group notices into Tenders and fold each group's chain. Three identity
/// regimes coexist (docs/research/ted-legacy-mapping.md §3, §8.3):
///
/// - **Keyed** — a notice publishing a procedure key (eForms BT-04) shares a
///   Tender with every notice under the same `(source, key)`.
/// - **Legacy OJS chain** — a legacy TED notice publishes no key; instead its
///   own OJS number is a graph node and each `is_ref` OJS id is an edge. The
///   transitive closure (union-find) is one Tender, identified by the *earliest*
///   OJS number in the component (including not-yet-ingested edge targets, so
///   the identity is stable as backfill deepens). eForms notices are excluded
///   by construction: they carry a BT-04 key and reference by UUID, so a legacy
///   → eForms procedure stays two Tenders (the accepted era-boundary split).
/// - **Island** — anything else (an eForms notice without BT-04, a DÖE numeric
///   island) is a single-notice Tender keyed by that notice.
///
/// A late edge that joins two existing components is an ADR-0003-style merge:
/// the component's earliest-OJS identity is deterministic, so the run simply
/// re-projects every member under the surviving key; the absorbed key's rows
/// are retired with `removed` change events in [`Db::retire_absorbed_legacy_tenders`].
fn group(states: Vec<NoticeState>) -> Vec<TenderProjection> {
    // Keyed Tenders group by procedure key *alone*, across Sources: a TED
    // eForms procedure and its DÖE twin publish one and the same BT-04 UUID
    // (ADR-0003, verified exact), so keying on the shared key is what merges
    // the two Sources into one Tender. Legacy OJS chains and islands stay
    // per-Source (ojs: keys are TED-only, an island is one notice).
    let mut keyed: BTreeMap<String, Vec<NoticeState>> = BTreeMap::new();
    let mut islands: Vec<NoticeState> = Vec::new();
    let mut legacy: Vec<NoticeState> = Vec::new();
    for state in states {
        match (&state.procedure_key, state.legacy, state.ojs_self) {
            (Some(key), _, _) => keyed.entry(key.clone()).or_default().push(state),
            (None, true, Some(_)) => legacy.push(state),
            (None, ..) => islands.push(state),
        }
    }

    let mut chains: Vec<Vec<NoticeState>> = keyed.into_values().collect();
    // Legacy: transitive closure over OJS edges, one bucket per component,
    // keyed by the component's earliest OJS number.
    let mut uf = UnionFind::default();
    for s in &legacy {
        let own = uf.node(s.ojs_self.expect("legacy states carry an own key"));
        for edge in &s.ojs_edges {
            let target = uf.node(*edge);
            uf.union(own, target);
        }
    }
    // One O(nodes) pass to fix each component's earliest-OJS representative,
    // then bucket every legacy notice under it — never a per-notice scan, so
    // this stays linear at 9M scale.
    let root_min = uf.root_minimums();
    let mut components: BTreeMap<OjsKey, Vec<NoticeState>> = BTreeMap::new();
    for s in legacy {
        let rep = root_min[&uf.find(uf.index[&s.ojs_self.unwrap()])];
        components.entry(rep).or_default().push(s);
    }
    for (rep, chain) in components {
        chains.push(chain.into_iter().map(|mut s| {
            s.procedure_key = Some(ojs_procedure_key(rep));
            s
        }).collect());
    }
    // Islands stay one Tender each.
    for island in islands {
        chains.push(vec![island]);
    }

    chains
        .into_iter()
        .map(|mut chain| {
            // Order by publication instant, then a fixed Source precedence, so a
            // cross-source procedure folds deterministically: on an equal
            // instant the TED reading folds *last* and thus supersedes the DÖE
            // one for shared eForms fields and publication identity (ADR-0003).
            chain.sort_by(|a, b| {
                (a.published_at, source_rank(&a.source), &a.publication_id, a.notice_id)
                    .cmp(&(b.published_at, source_rank(&b.source), &b.publication_id, b.notice_id))
            });
            let first = &chain[0];
            let projection_kind = first.kind().to_owned();
            let procedure_key = first.procedure_key.clone();
            let island_notice_id = procedure_key.is_none().then_some(first.notice_id);
            TenderProjection {
                source: primary_source(&chain),
                procedure_key,
                island_notice_id,
                kind: projection_kind,
                versions: fold(&chain),
            }
        })
        .collect()
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
/// Tender; a Source-only procedure keeps its own.
fn primary_source(chain: &[NoticeState]) -> String {
    if chain.iter().any(|s| s.source == "ted") {
        "ted".to_owned()
    } else {
        chain[0].source.clone()
    }
}

/// The synthetic procedure key of a legacy Tender: its component's earliest OJS
/// number. Namespaced so it can never collide with an eForms BT-04 folder id.
fn ojs_procedure_key((year, number): OjsKey) -> String {
    format!("ojs:{year}-{number:06}")
}

/// Union-find over OJS `(year, number)` nodes — the transitive-closure grouping
/// of legacy notices into Tenders. The representative of a component is its
/// *minimum* key (the earliest publication, the procedure's natural root), so
/// grouping is deterministic regardless of the order edges arrive.
#[derive(Default)]
struct UnionFind {
    index: HashMap<OjsKey, usize>,
    parent: Vec<usize>,
    key: Vec<OjsKey>,
}

impl UnionFind {
    fn node(&mut self, key: OjsKey) -> usize {
        if let Some(&i) = self.index.get(&key) {
            return i;
        }
        let i = self.parent.len();
        self.index.insert(key, i);
        self.parent.push(i);
        self.key.push(key);
        i
    }

    fn find(&mut self, mut i: usize) -> usize {
        while self.parent[i] != i {
            self.parent[i] = self.parent[self.parent[i]];
            i = self.parent[i];
        }
        i
    }

    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra] = rb;
        }
    }

    /// Each component's root → its minimum OJS key, in one linear pass.
    fn root_minimums(&mut self) -> HashMap<usize, OjsKey> {
        let mut min: HashMap<usize, OjsKey> = HashMap::with_capacity(self.parent.len());
        for i in 0..self.parent.len() {
            let root = self.find(i);
            let key = self.key[i];
            min.entry(root).and_modify(|m| *m = (*m).min(key)).or_insert(key);
        }
        min
    }
}

/// Resolve the chain: each version is the notice's own values laid over the
/// previous version's, per field — except results, which are *additive*.
fn fold(chain: &[NoticeState]) -> Vec<TenderVersion> {
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

        versions.push(TenderVersion {
            caused_by_notice_id: state.notice_id,
            published_at: state.published_at,
            dispatched_at: state.dispatched_at,
            notice_subtype: state.subtype.clone(),
            publication_id: state.publication_id.clone(),
            facts,
            lots,
            rounds,
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
    statistics: Vec<(String, i64)>, // BT-760 code, BT-759 count
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
}

#[derive(Default)]
struct RawContract {
    key: String,
    buyer_contract_id: Option<String>,  // BT-150
    concluded: Option<(i64, i64, bool)>, // BT-145
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
    for (code, count, owner) in stats.into_values() {
        if let (Some(code), Some(count)) = (code, count)
            && let Some(r) = raw.lot_results.iter_mut().find(|r| r.key == owner)
        {
            r.statistics.push((code.to_owned(), count));
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
            ("TED-NO_AWARDED_CONTRACT", _) => r.decision = Some("clos-nw".to_owned()),
            (f, NoticeValue::Integer(n)) if LEGACY_BID_COUNT_FIELDS.contains(&f) => {
                r.statistics.push(("tenders".to_owned(), *n));
            }
            (f, NoticeValue::Number { value: n, .. }) if LEGACY_BID_COUNT_FIELDS.contains(&f) => {
                r.statistics.push(("tenders".to_owned(), *n as i64));
            }
            _ => {}
        }
    }

    // Decide from the evidence: a named winner or an awarded value is a win.
    for r in &mut raw.lot_results {
        r.direct_winners.sort();
        r.direct_winners.dedup();
        if r.decision.is_none() {
            let awarded = !r.direct_winners.is_empty() || r.direct_cents.is_some();
            r.decision = Some(if awarded { "selec-w" } else { "clos-nw" }.to_owned());
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
    // The decision code hangs on the TenderResult section itself.
    for value in &parsed.values {
        if value.field_id == SDK01_RESULT_CODE_FIELD
            && let NoticeValue::Code { code, .. } = &value.value
            && let Some(r) = raw.lot_results.iter_mut().find(|r| r.key == value.section_id)
        {
            r.decision = Some(code.clone());
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
        if r.decision.is_none() {
            r.decision = Some(if r.direct_winners.is_empty() { "clos-nw" } else { "selec-w" }.to_owned());
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
                } else if r.decision.as_deref() == Some("selec-w") {
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
    profile == "text" || profile.starts_with("ted-export")
}

/// The DÖE sdk-0.1 dialect (issue 29): a permanent ~40%-of-German-volume channel
/// with its own `SDK01-*` node vocabulary, projected by the sdk-0.1 party and
/// results paths rather than the eForms `Organization`/`LotResult` ones.
fn is_sdk01_profile(profile: &str) -> bool {
    profile == "eforms:eforms-sdk-0.1"
}

/// The Tender's procedure key: BT-04 for eForms/eForms-DE, or — for the sdk-0.1
/// dialect — its `ContractFolderID` when that is a genuine uuid. A shared uuid is
/// the strong explicit cross-reference ADR-0003 merges on (a TED eForms
/// procedure and its DÖE twin publish the same BT-04 uuid), so keying sdk-0.1 on
/// it upgrades a uuid-bearing island into the merged Tender. Non-uuid folder ids
/// (the sdk-0.1 numeric channel) are notice-local and never key a Tender — a
/// missed link splits, it must never wrongly merge (issue 34).
fn procedure_key(parsed: &Parsed, sdk01: bool) -> Option<String> {
    if let Some(key) = first_id(parsed, PROCEDURE_KEY_FIELD).filter(|k| !k.trim().is_empty()) {
        return Some(key);
    }
    sdk01.then(|| first_id(parsed, SDK01_FOLDER_FIELD).filter(|k| is_uuid(k))).flatten()
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
/// A VAT id carries its country in its own prefix and is scoped by it; a
/// national registry number is only unique inside its country, so it is scoped
/// by the mention's country and stays separate when that is unknown.
pub fn normalise_identifier(raw: &str, country: Option<&str>) -> Option<Identifier> {
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

    let vat_prefix: String = value.chars().take(2).collect();
    let is_vat = vat_prefix.chars().all(|c| c.is_ascii_alphabetic())
        && value[2..].chars().any(|c| c.is_ascii_digit());
    if is_vat {
        Some(Identifier { country: Some(vat_prefix), kind: "vat".into(), value })
    } else {
        Some(Identifier {
            country: country.map(str::to_owned),
            kind: "national".into(),
            value,
        })
    }
}

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

/// The publication and dispatch instants of a parsed notice, resolved per era
/// (issue 18). `published_at` is the real publication date where the notice
/// records one (the OJEU stamp, the legacy OJ date, or DÖE's requested/portal
/// date), falling back to dispatch; `dispatched_at` is the send date, or `None`
/// when the notice carries none (e.g. a DÖE numeric island with only a
/// requested-publication date). Shared by the processor (which stamps the
/// notice row) and the projection (which stamps the version), so both agree.
pub fn notice_instants(parsed: &Parsed) -> (i64, Option<i64>) {
    let dispatched_at = DISPATCH_DATE_FIELDS.iter().find_map(|f| first_date(parsed, f));
    let published_at = PUBLICATION_DATE_FIELDS
        .iter()
        .find_map(|f| first_date(parsed, f))
        .or(dispatched_at)
        .unwrap_or(0);
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

    #[test]
    fn business_term_stems_survive_their_context_suffix() {
        assert_eq!(stem("BT-21-Lot"), "BT-21");
        assert_eq!(stem("BT-21-Procedure"), "BT-21");
        assert_eq!(stem("BT-131(d)-Lot"), "BT-131(d)");
        assert_eq!(stem("OPP-070-notice"), "OPP-070");
        assert_eq!(stem("BT-04"), "BT-04");
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
