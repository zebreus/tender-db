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
const APPLY_NOTICE_BATCH: usize = 20_000;

/// How many Phase-2 batches (and Phase-1 plan chunks) between WAL truncations.
/// Both the plan build and the apply burst grow the WAL; truncating at the clean
/// point between batches returns the space (issue 42) without checkpointing so
/// often the cost shows.
const CHECKPOINT_EVERY_BATCHES: usize = 4;

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
    /// Phase 1 → 2 transition: the plan grouped into `tenders` (`islands` of them
    /// single-notice).
    Grouped { tenders: u64, islands: u64 },
    /// Phase 2: `tenders` of `total` folded and applied so far.
    Applying { tenders: u64, total: u64 },
}

/// The projection with an explicit Phase-2 batch size (notices per fold+apply
/// batch). [`project`] uses [`APPLY_NOTICE_BATCH`]; tests drive tiny batches to
/// prove the output is invariant under batching — i.e. that folding whole Tenders
/// a batch at a time never splits a Tender's notices across a boundary (issue 57).
/// Progress is logged to stderr; use [`project_with_progress`] to observe it.
pub async fn project_with_batch(db: &Db, rebuild: bool, notice_batch: usize) -> turso::Result<Report> {
    // Default sink: log heartbeats to stderr, Phase-1 throttled to PLAN_HEARTBEAT.
    let mut last_plan_log = 0u64;
    project_with_progress(db, rebuild, notice_batch, |p| match p {
        Progress::Planning { notices, total } => {
            if notices - last_plan_log >= PLAN_HEARTBEAT || notices == total {
                eprintln!("[project] phase 1: {notices}/{total} notices planned");
                last_plan_log = notices;
            }
        }
        Progress::Grouped { tenders, islands } => {
            eprintln!("[project] phase 2: folding {tenders} tenders ({islands} islands)");
        }
        Progress::Applying { tenders, total } => {
            eprintln!("[project] phase 2: {tenders}/{total} tenders applied");
        }
    })
    .await
}

/// The projection core, reporting progress through `on_progress` (called between
/// awaits, so a cheap closure). See [`Progress`]; [`project_with_batch`] wraps
/// this with a stderr-logging sink.
pub async fn project_with_progress(
    db: &Db,
    rebuild: bool,
    notice_batch: usize,
    mut on_progress: impl FnMut(Progress),
) -> turso::Result<Report> {
    // Resume-from-plan salvage (issue 60): if an interrupted rebuild already left a
    // COMPLETE grouping plan on disk (Phase-1 finished — the expensive part), skip
    // the clear + strip + the whole of Phase-1 and re-run only grouping (path-B) →
    // Phase-2 from the immutable on-disk plan. A normal rebuild (its prior run
    // cleared the plan) sees an empty/absent plan and rebuilds from scratch.
    let resume = rebuild && db.plan_is_complete().await?;
    if rebuild && !resume {
        db.clear_canonical().await?;
        // Bulk-load the Organization tables index-free, then rebuild the indexes
        // once at the end (issue 60): the per-row uniqueness probe into the org
        // identity index was a random-seek storm once it outgrew the page cache.
        db.strip_organization_indexes().await?;
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
        let (notices, mentions) = build_plan(db, now, total, &mut on_progress).await?;
        report.notices = notices;
        report.mentions = mentions;
    }

    // Group the plan into Tenders — keyed chains, the legacy OJS transitive-closure
    // union-find, islands — entirely in SQL over the on-disk plan (issue 59), so no
    // whole-corpus structure ever enters RAM.
    let t1 = std::time::Instant::now();
    db.build_plan_groups().await?;
    let legacy_keys = db.plan_legacy_keys().await?;
    let (tenders, islands) = db.plan_counts().await?;
    report.tenders = tenders;
    report.islands = islands;
    let _ = db.checkpoint(store::CheckpointMode::Truncate).await;
    on_progress(Progress::Grouped { tenders, islands });
    eprintln!("[project] group: {} tenders in {:.1}s", report.tenders, t1.elapsed().as_secs_f64());

    // Phase 2 — stream whole-Tender batches out of the plan (in fold order) and
    // apply them a bounded batch of notices at a time. Each batch reads only its
    // notices' parsed layer and their already-resolved Organizations, folds each
    // group, and reconciles — so peak RAM is one batch, independent of corpus.
    let t2 = std::time::Instant::now();
    let mut after = String::new();
    let mut batches_done = 0usize;
    let mut tenders_done = 0u64;
    loop {
        let groups = db.next_plan_batch(&after, notice_batch).await?;
        let Some(last) = groups.last() else { break };
        after = last.group_key.clone();
        tenders_done += groups.len() as u64;
        report.applied.add(apply_plan_batch(db, &groups, now).await?);
        // Heartbeat per batch so Phase 2 reports how far along it is (issue 59).
        on_progress(Progress::Applying { tenders: tenders_done, total: report.tenders });
        batches_done += 1;
        if batches_done.is_multiple_of(CHECKPOINT_EVERY_BATCHES)
            && let Err(e) = db.checkpoint(store::CheckpointMode::Truncate).await
        {
            eprintln!("[project] checkpoint after batch {batches_done}: {e}");
        }
    }
    // Retire any legacy Tender a late component-merge absorbed (its rows migrated
    // to the surviving key; here it gets `removed` change events).
    report.absorbed = db.retire_absorbed_legacy_tenders(&legacy_keys, now).await?;
    // Don't leave the transient plan in the durable DB between runs (issue 59).
    db.clear_plan().await?;
    // Rebuild the Organization indexes the bulk load ran without (issue 60) — one
    // sorted build each, now that every org and mention is in.
    if rebuild {
        let ti = std::time::Instant::now();
        db.build_organization_indexes().await?;
        let _ = db.checkpoint(store::CheckpointMode::Truncate).await;
        eprintln!("[project] org indexes rebuilt in {:.1}s", ti.elapsed().as_secs_f64());
    }
    // Build the incremental change-set index now that (nearly) every parsed notice
    // is projected=1, so the partial index is near-empty and instant (issue 58).
    db.ensure_unprojected_index().await?;
    eprintln!("[project] apply: {} tenders in {:.1}s", report.tenders, t2.elapsed().as_secs_f64());
    eprintln!(
        "[project] done: {} notices → {} tenders ({} islands), {} versions, {} change rows in {:.1}s",
        report.notices,
        report.tenders,
        report.islands,
        report.applied.versions_written,
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
/// disk (issue 59), so peak RAM is one read chunk, independent of corpus. Returns
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
) -> turso::Result<(u64, u64)> {
    const READ_CHUNK: i64 = 10_000;
    let t0 = std::time::Instant::now();
    db.reset_plan().await?;
    let mut resolver = db.mention_resolver().await?;
    let (mut notices, mut mentions_total) = (0u64, 0u64);
    let mut after_id = 0i64;
    let mut chunks = 0usize;
    loop {
        let chunk = db.parsed_chunk(after_id, READ_CHUNK).await?;
        let Some((last, _)) = chunk.last() else { break };
        after_id = last.id;
        let mut mentions: Vec<Mention> = Vec::new();
        let mut rows: Vec<store::PlanRow> = Vec::with_capacity(chunk.len());
        for (notice, parsed) in &chunk {
            let ident = Ident::read(notice, parsed);
            mentions.extend(NoticeState::mentions(ident.sdk01, notice.id, parsed));
            rows.push(ident.into_plan_row());
        }
        notices += rows.len() as u64;
        mentions_total += db.resolve_mentions(&mut resolver, &mentions, now).await?.len() as u64;
        db.insert_plan(&rows).await?;
        // Heartbeat so a many-minute plan build is visibly alive (issue 59).
        on_progress(Progress::Planning { notices, total });
        // Keep the WAL bounded through the plan build too (issue 42/59): the
        // plan-row inserts are a burst; truncate at the clean point between chunks.
        chunks += 1;
        if chunks.is_multiple_of(CHECKPOINT_EVERY_BATCHES)
            && let Err(e) = db.checkpoint(store::CheckpointMode::Truncate).await
        {
            eprintln!("[project] plan checkpoint after chunk {chunks}: {e}");
        }
    }
    db.finish_mention_resolver(resolver).await?;
    eprintln!(
        "[project] plan: {notices} notices, {mentions_total} mentions resolved in {:.1}s",
        t0.elapsed().as_secs_f64()
    );
    Ok((notices, mentions_total))
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
        let (notices, mentions) = build_plan(db, store::now_unix(), total, |_| {}).await?;
        Ok::<Report, turso::Error>(Report { notices, mentions, ..Default::default() })
    }
    .await;
    let restored = db.set_foreign_keys(true).await;
    let report = result?;
    restored?;
    Ok(report)
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
/// LEGACY FALLBACK: the transitive OJS union-find needs the whole existing edge
/// graph, which is not persisted between runs. Legacy notices are the pre-2024
/// historical era, loaded by bulk backfill (`rebuild:true`), never in the daily
/// near-real-time feed — so if the delta contains ANY legacy notice this falls
/// back to a full non-rebuild projection (correct, and effectively never fires
/// daily). It logs loudly, so if legacy ever starts arriving incrementally we
/// notice and build the durable-adjacency path (issue 58 v2).
pub async fn project_incremental(db: &Db) -> turso::Result<Report> {
    db.set_foreign_keys(false).await?;
    let result = project_incremental_inner(db).await;
    let restored = db.set_foreign_keys(true).await;
    let report = result?;
    restored?;
    Ok(report)
}

async fn project_incremental_inner(db: &Db) -> turso::Result<Report> {
    let changed = db.unprojected_parsed_notice_ids().await?;
    if changed.is_empty() {
        return Ok(Report::default());
    }
    let now = store::now_unix();
    let t0 = std::time::Instant::now();

    // Read the changed notices' grouping identity: detect legacy (→ fallback) and
    // collect their new keyed keys (a notice joining an existing keyed Tender).
    // Keep the parsed layer — we reuse it to build the plan without re-reading.
    let mut changed_parsed = db.parsed_by_ids(&changed).await?;
    changed_parsed.sort_by_key(|(n, _)| n.id);
    let changed_set: std::collections::HashSet<i64> = changed.iter().copied().collect();
    let mut new_keyed_keys: Vec<String> = Vec::new();
    for (notice, parsed) in &changed_parsed {
        let ident = Ident::read(notice, parsed);
        if ident.legacy {
            eprintln!(
                "[project] INCREMENTAL → FULL fallback: delta contains legacy notice(s) \
                 (issue 58 v1); re-projecting the whole corpus"
            );
            return project_with_batch(db, false, APPLY_NOTICE_BATCH).await;
        }
        if let Some(key) = &ident.procedure_key {
            new_keyed_keys.push(key.clone());
        }
    }
    new_keyed_keys.sort_unstable();
    new_keyed_keys.dedup();

    // Expand to the touched EXISTING Tenders and load their full notice sets, so
    // each is re-derived in full alongside the changed notices.
    let touched_tenders = db.touched_existing_tender_ids(&changed, &new_keyed_keys).await?;
    let existing_ids = db.notice_ids_for_tenders(&touched_tenders).await?;

    // Phase 1 (scoped): plan rows for the WHOLE touched notice set, in id order so
    // the plan/org bulk-load stays sequential; mentions resolved only for the
    // changed notices (the existing notices' mentions are already recorded and are
    // bound in Phase 2 by `mentions_by_ids`).
    db.reset_plan().await?;
    let extra_ids: Vec<i64> = existing_ids.iter().copied().filter(|id| !changed_set.contains(id)).collect();
    let mut extra_parsed = db.parsed_by_ids(&extra_ids).await?;
    extra_parsed.sort_by_key(|(n, _)| n.id);

    let mut resolver = db.mention_resolver().await?;
    let mut rows: Vec<store::PlanRow> = Vec::with_capacity(changed_parsed.len() + extra_parsed.len());
    let mut mentions: Vec<Mention> = Vec::new();
    // Both slices are id-sorted; merge them so the plan appends in id order and the
    // mentions resolve in id order (org ids stay identical to a full projection).
    let mut ci = changed_parsed.iter();
    let mut ei = extra_parsed.iter();
    let (mut cn, mut en) = (ci.next(), ei.next());
    loop {
        let take_changed = match (cn, en) {
            (Some((c, _)), Some((e, _))) => c.id <= e.id,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        let (notice, parsed) = if take_changed {
            let v = cn.unwrap();
            cn = ci.next();
            v
        } else {
            let v = en.unwrap();
            en = ei.next();
            v
        };
        let ident = Ident::read(notice, parsed);
        if changed_set.contains(&notice.id) {
            mentions.extend(NoticeState::mentions(ident.sdk01, notice.id, parsed));
        }
        rows.push(ident.into_plan_row());
    }
    let mut report = Report::default();
    report.notices = changed.len() as u64;
    db.insert_plan(&rows).await?;
    report.mentions = db.resolve_mentions(&mut resolver, &mentions, now).await?.len() as u64;
    db.finish_mention_resolver(resolver).await?;

    // Group the scoped plan (same SQL as a full run — over the touched set only).
    db.build_plan_groups().await?;
    let (tenders, islands) = db.plan_counts().await?;
    report.tenders = tenders;
    report.islands = islands;

    // Retire any touched Tender the new plan did not reproduce (island→keyed
    // upgrade, etc.) BEFORE applying, so a regrouped notice's old Tender is gone.
    report.absorbed = db.retire_regrouped_tenders(&touched_tenders, now).await?;

    // Phase 2 (unchanged): fold + apply the touched Tenders a bounded batch at a
    // time; `apply_tenders` upserts each by natural key and marks its notices
    // projected — untouched Tenders are never read or written.
    let mut after = String::new();
    loop {
        let groups = db.next_plan_batch(&after, APPLY_NOTICE_BATCH).await?;
        let Some(last) = groups.last() else { break };
        after = last.group_key.clone();
        report.applied.add(apply_plan_batch(db, &groups, now).await?);
    }
    db.clear_plan().await?;
    // Keep the change-set index present for the next daily run (issue 58); cheap —
    // it exists after the first projection and this is a no-op thereafter.
    db.ensure_unprojected_index().await?;
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
) -> turso::Result<store::Applied> {
    let ids: Vec<i64> = groups.iter().flat_map(|g| g.notice_ids.iter().copied()).collect();
    let parsed = db.parsed_by_ids(&ids).await?;
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
    let applied = db.apply_tenders(&projections, now).await?;
    // Mark every applied notice as folded into the canonical layer (issue 58) —
    // whether or not its Tender changed — so the next incremental run skips it.
    db.mark_projected(&ids).await?;
    Ok(applied)
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

        let (published_at, dispatched_at) = notice_instants(parsed);

        NoticeState {
            notice_id: notice.id,
            publication_id: notice.publication_id.clone(),
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
    fn mentions(sdk01: bool, notice_id: i64, parsed: &Parsed) -> Vec<Mention> {
        let sections: HashMap<&str, &store::Section> =
            parsed.sections.iter().map(|s| (s.id.as_str(), s)).collect();

        // sdk-0.1 has no eForms `Organization` sections: its buyer and winners
        // are the inline `ContractingParty`/`WinningParty` sections themselves,
        // each carrying its name on its direct Party subtree (issue 29).
        let mention_kinds: &[&str] = if sdk01 { SDK01_PARTY_KINDS } else { &[ORGANIZATION_KIND] };

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
    /// results graph into a bound Round — now that each mention has a canonical
    /// Organization. `by_section` maps this notice's Organization section ids to
    /// the canonical Organization ids Phase 1 resolved and recorded.
    fn bind_organizations(&mut self, by_section: &HashMap<String, i64>) {
        let by_section: HashMap<&str, i64> =
            by_section.iter().map(|(k, &v)| (k.as_str(), v)).collect();
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
///   number is a graph node and each `is_ref` OJS id an edge. The transitive
///   closure is one Tender, identified by the *earliest* OJS number in the
///   component (including not-yet-ingested edge targets, so identity is stable as
///   backfill deepens). A late edge merging two components is an ADR-0003-style
///   merge; the absorbed key's rows are retired with `removed` events.
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
    subtype: Option<String>,
}

/// Encode an OJS key `(year, number)` as `year*1e9 + number` — a single sortable
/// integer whose `MIN` over a component is the earliest publication (the legacy
/// Tender's representative). `number` is well under 1e9, `year ≤ 2100`, so this
/// fits `i64` and never collides across keys.
fn encode_ojs((year, number): OjsKey) -> i64 {
    year * 1_000_000_000 + number
}

impl Ident {
    fn read(notice: &store::NoticeRef, parsed: &Parsed) -> Ident {
        let legacy = is_legacy_profile(&notice.profile);
        let sdk01 = is_sdk01_profile(&notice.profile);
        // The chain edges: every `is_ref` OJS-scheme id the notice carries. Same
        // reading as [`NoticeState::read`], so the grouping is identical.
        let mut ojs_edges: Vec<OjsKey> = parsed
            .values
            .iter()
            .filter_map(|v| match &v.value {
                NoticeValue::Id { value, is_ref: true, scheme } if scheme.as_deref() == Some("ojs") => {
                    ojs_key(value)
                }
                _ => None,
            })
            .collect();
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
            published_at: notice_instants(parsed).0,
            legacy,
            sdk01,
            procedure_key: procedure_key(parsed, sdk01),
            ojs_self,
            ojs_edges,
            subtype: first_code(parsed, SUBTYPE_FIELD),
        }
    }

    /// This notice's row for the on-disk grouping plan — OJS keys encoded, Source
    /// precedence precomputed (issue 59).
    fn into_plan_row(self) -> store::PlanRow {
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
