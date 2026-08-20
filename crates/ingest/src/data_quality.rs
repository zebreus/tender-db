//! `data-quality` — the first *semantic* measurement of the tender DB (issue 27).
//!
//! Coverage (per-year counts, `bin/verify`) answers "how much did we import?";
//! quarantine answers "what failed to parse?". Neither says anything about how
//! *complete* the data we did import actually is. This module measures that: per
//! era, what share of the canonical layer carries a title, a buyer, a value, a
//! CPV code, a deadline, a winner; how well awards chain back to their contract
//! notice; whether contract-award notices actually materialised their results;
//! and how often a procedure's TED and DÖE readings merged into one Tender
//! (ADR-0003, issue 12's promise).
//!
//! Unlike `verify`, this is **descriptive, not pass/fail** — there is no
//! external ground truth for "the right completeness rate", the numbers *are*
//! the finding. It is the discovery tool that turns "we think results aren't
//! projecting" into a number; the fixes it points at become their own issues, and
//! the first full-corpus run produced four (issues 231–234) plus two defects in the
//! report itself.
//!
//! This paragraph used to cite "eForms CANs materialise results at 0.3 %" as the
//! example, then recorded that the number was no longer reproducible because
//! section 3 read 100 % for every era. **Both readings were artefacts of the same
//! definition defect, now fixed** (issue 235): section 3's denominator was
//! "notices that already carry a result SECTION", so it measured the projection
//! against its own parse layer and an award notice that parsed with zero results
//! — the actual failure, and what 0.3 % was probably describing — fell out of
//! both halves.
//!
//! The denominator now comes from what the notice IS: its published document type
//! ([`DOC_TYPE_MARKERS`] — the eForms subtype, the TED `TD` code, the 2008
//! export's form). The old pair survives as section 3b under an honest name, the
//! projection-writes-what-it-parsed invariant, which is a real invariant that has
//! caught a real defect. Neither number above is restated here: the next
//! full-corpus run measures the question properly and that reading, not this
//! comment, is the record.
//!
//! Design notes it is worth being explicit about:
//!
//! - **Unit = tender-version.** A field is a property of a *version* (satellites
//!   are keyed `(tender_id, seq)`), and each stored version already holds the
//!   *resolved* state at that point (superseded fields carry forward), so
//!   version-level presence is neither under- nor over-counted by corrigenda.
//!   This is exactly the issue's "fraction of notices per era". A multi-notice
//!   Tender is therefore weighted by its version count — the honest reading of
//!   "per notice".
//! - **Era = the mapping profile of the version's causing notice** — the same
//!   split the dashboard and `award_linkage` already use, so the rows line up.
//! - **Every query is bounded to stay under the endpoint's 10 s limit at archive
//!   scale.** Completeness is satellite-driven (`FROM <satellite> … JOIN
//!   tender_versions`) — one sequential scan probing only primary keys, safe even
//!   where a satellite lacks a `(tender_id, seq)` index. Linkage/density/merge
//!   drive from their small sets (award Tenders, award-typed notices, DÖE
//!   Tenders) and filter with indexed `EXISTS` — the document-type probe seeks
//!   `notice_codes(notice_id, …)`, the prefix of its primary key — never an inline
//!   `(SELECT DISTINCT …) AS x JOIN` — turso re-evaluates such a derived table
//!   per outer row, timing the endpoint out even at a few thousand rows.
//!
//! The module is transport-agnostic: it defines *what* to measure (the SQL) and
//! *how* to present it (assembly + rendering) over a plain [`Rows`] matrix. The
//! `bin/data-quality` wrapper feeds it rows from the live `/v1/sql` endpoint;
//! the tests feed it rows straight from a scratch [`store::Db`]. Both exercise
//! the identical SQL.

use serde_json::{Value, json};

/// A result set as the `/v1/sql` endpoint returns it: rows of JSON cells. The
/// store's own reader is adapted to the same shape in tests, so one assembler
/// serves both.
pub type Rows = Vec<Vec<Value>>;

// ------------------------------------------------------------------- the SQL

/// One measured field and where its presence lives. `predicate` narrows the
/// satellite to the rows that count as "the field is present"; `None` means any
/// row of the satellite counts.
struct FieldSpec {
    key: &'static str,
    satellite: &'static str,
    predicate: Option<&'static str>,
}

/// The six completeness fields the issue names, each resolved to its canonical
/// satellite and the projection's own field/role vocabulary (`project.rs`).
/// `buyer` matches both the legacy `buyer` role and the eForms `Procedure-Buyer`
/// via the shared `uyer` infix; `deadline` spans the submission/participation/
/// info deadlines; `value` and `winner` need no narrowing (any amount, any named
/// winner).
const FIELDS: [FieldSpec; 6] = [
    FieldSpec { key: "title", satellite: "tender_version_texts", predicate: Some("field = 'title'") },
    FieldSpec { key: "buyer", satellite: "tender_version_parties", predicate: Some("role LIKE '%uyer%'") },
    FieldSpec { key: "value", satellite: "tender_version_amounts", predicate: None },
    FieldSpec { key: "cpv", satellite: "tender_version_classifications", predicate: Some("scheme = 'cpv'") },
    FieldSpec { key: "deadline", satellite: "tender_version_dates", predicate: Some("field LIKE '%deadline%'") },
    FieldSpec { key: "winner", satellite: "tender_version_result_winners", predicate: None },
];

/// Per-profile count of versions carrying one field. Driven by the satellite so
/// it is a single scan: distinct `(tender_id, seq)` that have the field, joined
/// up to the version's notice for its profile.
fn field_sql(spec: &FieldSpec) -> String {
    let filter = spec.predicate.map(|p| format!(" AND {p}")).unwrap_or_default();
    // Indexed EXISTS, driven from `tender_versions` — NOT an inline
    // `(SELECT DISTINCT …) d JOIN`. That derived-table shape is what this
    // function used to build, and it is the exact pattern the linkage query below
    // documents as pathological: turso re-evaluates such a derived table per
    // outer row. At full-corpus scale it cost >50 minutes at 100% CPU on a single
    // field (issue 230) — the warning was written for `lot_results` and applied
    // here all along.
    //
    // Driving from the versions side makes this one pass over `tender_versions`
    // with a probe per row into the satellite's by-version index (`(tender_id,
    // seq)`, which texts/amounts/dates/classifications all carry), so the cost is
    // the same order as the DENOMINATOR_SQL pass beside it.
    format!(
        "SELECT n.profile, COUNT(*) AS present \
         FROM tender_versions v JOIN notices n ON n.id = v.caused_by_notice_id \
         WHERE EXISTS (SELECT 1 FROM {satellite} s \
                        WHERE s.tender_id = v.tender_id AND s.seq = v.seq{filter}) \
         GROUP BY n.profile",
        satellite = spec.satellite,
    )
}

/// Per-profile denominator: how many tender-versions each era holds.
pub const DENOMINATOR_SQL: &str = "SELECT n.profile, COUNT(*) AS versions \
     FROM tender_versions v JOIN notices n ON n.id = v.caused_by_notice_id \
     GROUP BY n.profile";

/// Award→notice linkage per era (docs/research/ted-legacy-mapping.md §3): of the
/// Tenders that carry an award, how many are a single-notice island — an award
/// that never chained to its contract notice. A deliberate mirror of
/// `store::Db::award_linkage` (the dashboard's typed copy of this same query):
/// two homes because this catalog holds raw `(label, sql)` strings while that is
/// a typed reader method. Both use the identical indexed-`EXISTS` shape below —
/// keep the two in sync.
///
/// Driven from the first version of each Tender (`seq = 1` — one row per Tender,
/// a small set) and filtered to award-bearing Tenders with an indexed `EXISTS`
/// on `lot_results`; "unchained" is an indexed `seq > 1` existence check.
/// Critically it does *not* wrap `lot_results` in an inline
/// `(SELECT DISTINCT …) AS a JOIN` — turso re-evaluates such a derived table per
/// outer row, which times the endpoint out even at a few thousand rows (verified
/// against prod). Bounded by the Tender count, every lookup a primary-key probe.
pub const LINKAGE_SQL: &str = "SELECT n.profile, \
            COUNT(*) AS awards, \
            SUM(CASE WHEN NOT EXISTS( \
                  SELECT 1 FROM tender_versions tv WHERE tv.tender_id = v1.tender_id AND tv.seq > 1 \
                ) THEN 1 ELSE 0 END) AS unchained \
       FROM tender_versions v1 \
       JOIN notices n ON n.id = v1.caused_by_notice_id \
      WHERE v1.seq = 1 \
        AND EXISTS(SELECT 1 FROM lot_results lr WHERE lr.tender_id = v1.tender_id) \
      GROUP BY n.profile";

/// Results materialisation per era, denominator: contract-award notices that
/// have been **projected** (have a `tender_version`) and whose raw layer holds a
/// `LotResult` section (era-agnostic — legacy award blocks synthesise the same
/// section kind, `r209/rules.rs`). Scoping to *projected* notices is deliberate:
/// a notice not yet projected (a mid-backfill state) is not a results-projection
/// bug, and including the whole notice-layer backlog both muddies the metric and
/// does not scale.
///
/// Kept as its own query (not a `lot_results` join) because `lot_results` has no
/// index on `notice_id`: joining it by that column is a per-row table scan that
/// times the endpoint out. The numerator is measured separately and the two are
/// combined by profile.
///
/// **Both result section kinds, and that is a fix, not breadth** (issue 230). This
/// query previously matched `kind = 'LotResult'` alone and its doc claimed to be
/// "era-agnostic" because legacy award blocks synthesise that kind. True for
/// legacy, false for sdk-0.1: `project.rs`'s `SDK01_RESULT_KIND` is `TenderResult`,
/// so the whole DÖE island fell out of the DENOMINATOR while the numerator counted
/// it — the first full run reported `0` award notices against `139,961` with
/// results, a state that cannot exist. A hardcoded vocabulary in a query that
/// claims to span eras is exactly the trap issue 174 named.
pub const SECTIONS_CAN_SQL: &str = "SELECT n.profile, COUNT(*) AS can_notices \
       FROM tender_versions tv JOIN notices n ON n.id = tv.caused_by_notice_id \
      WHERE EXISTS(SELECT 1 FROM notice_sections s \
                    WHERE s.notice_id = tv.caused_by_notice_id \
                      AND s.kind IN ('LotResult', 'TenderResult')) \
      GROUP BY n.profile";

/// Results materialisation per era, numerator: award-notice versions whose own
/// notice actually produced a canonical `lot_results` row.
///
/// Version-driven, matching [`SECTIONS_CAN_SQL`]'s unit — and that is a FIX, not a
/// windowing convenience (issue 230). The previous form drove from `lot_results`
/// and counted `COUNT(DISTINCT lr.notice_id)`, so the denominator counted versions
/// while the numerator counted notices: a notice causing two versions contributed
/// 2 below the line and 1 above it, quietly depressing the density of exactly the
/// eras where corrigenda are common. Both halves now count versions, so the ratio
/// is a ratio.
///
/// The `EXISTS` probes `lot_results` on `(tender_id, notice_id)` — the prefix of
/// its `UNIQUE(tender_id, notice_id, result_key)` index — so it stays an indexed
/// seek. Driving from `lot_results` by `notice_id` would not: that column has no
/// index, which is why the two halves are measured separately at all.
pub const SECTIONS_WITH_SQL: &str = "SELECT n.profile, COUNT(*) AS with_results \
       FROM tender_versions tv JOIN notices n ON n.id = tv.caused_by_notice_id \
      WHERE EXISTS(SELECT 1 FROM lot_results lr \
                    WHERE lr.tender_id = tv.tender_id AND lr.notice_id = tv.caused_by_notice_id) \
      GROUP BY n.profile";

// ------------------------------------------- the published document type

/// One era family's published document-type marker: where the type is recorded
/// in the parse layer, which of its codes name a **result-bearing** publication,
/// and which are known not to.
///
/// This table exists because section 3's denominator used to be "notices that
/// already carry a result SECTION" — the projection measured against itself, so
/// an award notice that parsed with zero results was excluded from BOTH halves
/// and the rate read exactly 100.0 % on all twenty eras that measure (issue
/// 235). A denominator has to come from what the notice *is*, which every era
/// publishes and no era leaves to the projection.
///
/// **Both lists are spelled out on purpose.** A code in NEITHER is counted per
/// era by [`doc_type_sql`] as unclassified, so a vocabulary this table does not
/// know shows up as a number instead of silently becoming "not an award" — the
/// same defect one layer up, where `kind = 'LotResult'` dropped sdk-0.1's
/// `TenderResult` sections and the report read 0 award notices against 139,961
/// with results (issue 230).
///
/// ## Where each list comes from (measured on prod, 2026-08-19)
///
/// Sampled notices per profile, cross-tabbed against "carries a result section",
/// and — for the legacy TED codes — against `TED-FORM`, whose 2014 form set names
/// the document type outright. Every code below was observed; nothing is inferred
/// from a codelist we do not vendor.
///
/// | marker | award codes | evidence |
/// |---|---|---|
/// | `OPP-070-notice` | 25–40, `E4`, `E5` | ≤24 never carried results (800 notices); ≥25 always did; `E4` 21/21, `E5` 1/1 |
/// | `DE1-NoticeSubType-SubTypeCode` | same numeric scheme | DE 1.x reaches `SUBTYPE_FIELD` through this alias (project.rs) |
/// | `TED-TD_DOCUMENT_TYPE` | `7`, `J`, `K`, `R`, `V` | 7↔F03/F06 award (776/776), J↔F25 concession award, K↔F20 modification, R↔F13 design-contest result, V↔F15 VEAT — each 100 % result-bearing |
/// | `TXT-TD` | `7` | the text era publishes its own label: `TD: 7 - Contract awards` (1993 daily fixture), `TD: 3 - Invitation to tender` |
/// | `TED-FORM` (internal-ojs only) | `3`, `3_SUM` | the 2008 export carries no `TD_DOCUMENT_TYPE` at all; form 3 is the award form, 256/256 result-bearing |
///
/// The letter codes' meanings are the fixtures' own element text, not a guess:
/// `TD_DOCUMENT_TYPE CODE="V">Voluntary ex ante transparency notice`,
/// `CODE="K">Modification of a contract/concession during its term`,
/// `CODE="7">Contract award`, `CODE="3">Contract notice`.
pub struct DocTypeMarker {
    /// `notices.profile` values this marker speaks for, as SQL `LIKE` patterns —
    /// the eForms families are versioned, so `'eforms:eforms-sdk-1%'` is the
    /// honest scope and an exact name is a pattern without a wildcard.
    ///
    /// Scoping matters: `TED-FORM` exists in r2.0.8 and r2.0.9 too, with a
    /// DIFFERENT vocabulary (`F03`, `15`, `13`), and those eras are classified by
    /// their `TD_DOCUMENT_TYPE` instead. An unscoped field id would apply the
    /// 2008 form list to them and report every r2.0.9 form as unclassified.
    pub profiles: &'static [&'static str],
    /// The parse-layer field carrying the published type (`notice_codes.field_id`).
    pub field_id: &'static str,
    /// Codes naming a result-bearing publication: an award, a concession award, a
    /// design-contest result, a direct-award prenotification (VEAT), or a
    /// contract modification. All five publish a decision the projection is
    /// expected to materialise, which is the population section 3 asks about.
    pub award: &'static [&'static str],
    /// Codes observed and known NOT to be result-bearing. Only measured codes are
    /// listed — an unlisted one is unclassified, which is visible, rather than
    /// silently non-award, which is not.
    pub other: &'static [&'static str],
}

/// The eForms result-notice subtypes: 25–40 (direct-award prenotification,
/// result, modification) plus the `E`-family award subtypes. Shared by the EU
/// SDKs and the DE 1.x alias, which use the same numbering.
const EFORMS_AWARD_SUBTYPES: &[&str] = &[
    "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40",
    "E4", "E5",
];

/// The eForms planning and competition subtypes — measured never to carry a
/// result section (800 notices across sdk-1.7, sdk-1.14, de-1.1, de-2.1).
const EFORMS_OTHER_SUBTYPES: &[&str] = &[
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17",
    "18", "19", "20", "21", "22", "23", "24", "E1", "E3",
];

/// Per-era document-type markers, in the order the report's diagnostic renders
/// them. See [`DocTypeMarker`] for how each list was measured.
/// The TED `TD` ("type of document") codes that announce a result, shared by every
/// TED-lineage era: `7` contract award, `J` concession award, `K` modification,
/// `R` design-contest result, `V` voluntary ex-ante transparency.
const TED_TD_AWARD: &[&str] = &["7", "J", "K", "R", "V"];

/// The `TD` codes that are known NOT to announce a result. A code in neither list
/// is reported as `unclassified` rather than assumed non-award — the 2008 era's
/// tail (`E`, `G`, `I`, `S`, `B`, `4`, `6`) sits there deliberately: ~60 notices in
/// 27k, no cross-tab evidence either way, and inventing an answer for them would be
/// the same guess this issue exists to remove.
const TED_TD_OTHER: &[&str] =
    &["0", "1", "2", "3", "A", "C", "D", "H", "M", "O", "P", "Q", "Y"];

pub const DOC_TYPE_MARKERS: &[DocTypeMarker] = &[
    // eForms EU (every SDK 1.x) and eForms-DE 2.x, which publishes the EU field.
    DocTypeMarker {
        profiles: &["eforms:eforms-sdk-1%", "eforms:eforms-de-2%"],
        field_id: "OPP-070-notice",
        award: EFORMS_AWARD_SUBTYPES,
        other: EFORMS_OTHER_SUBTYPES,
    },
    // eForms-DE 1.x names the same subtype through its own id — the alias
    // `DE1-NoticeSubType-SubTypeCode` → `SUBTYPE_FIELD` in project.rs.
    DocTypeMarker {
        profiles: &["eforms:eforms-de-1%"],
        field_id: "DE1-NoticeSubType-SubTypeCode",
        award: EFORMS_AWARD_SUBTYPES,
        other: EFORMS_OTHER_SUBTYPES,
    },
    // The DÖE island spells its type in words rather than subtype numbers.
    DocTypeMarker {
        profiles: &["eforms:eforms-sdk-0.1"],
        field_id: "SDK01-NoticeTypeCode",
        award: &["can-standard"],
        other: &["cn-standard", "pin-only"],
    },
    // R2.0.8 / R2.0.9 / defence: the TED-wide `TD` vocabulary.
    DocTypeMarker {
        profiles: &["ted-export-r20%"],
        field_id: "TED-TD_DOCUMENT_TYPE",
        award: TED_TD_AWARD,
        other: TED_TD_OTHER,
    },
    // The text era publishes the same `TD` concept as a labelled line.
    DocTypeMarker {
        profiles: &["text"],
        field_id: "TXT-TD",
        award: &["7"],
        other: &["0", "2", "3", "C"],
    },
    // Why the 2008 export is read through `NAT_NOTICE` and not through its form.
    //
    // Measured on prod (27k internal-ojs notices, cross-tabbing the two fields on the
    // 6k that carry both):
    //
    // | `TED-FORM`      | `TED-NAT_NOTICE` | what the form is        |
    // |-----------------|------------------|-------------------------|
    // | `2` / `2_SUM`   | `3`              | contract notice         |
    // | `3` / `3_SUM`   | `7`              | contract award          |
    // | `6` / `6_SUM`   | `7`              | utilities award         |
    // | `1` / `1_SUM`   | `0`              | prior information       |
    // | `13_SUM`        | `R`              | design-contest result   |
    // | `12_SUM`        | `D`              | —                       |
    //
    // Two facts follow, and both favour `NAT_NOTICE`:
    //
    // 1. **It IS the `TD` vocabulary.** Award forms map to `7`, contract notices to
    //    `3`, design-contest results to `R` — the same codes [`TED_TD_AWARD`] carries
    //    for r2.0.8/r2.0.9. So the era needs no vocabulary of its own.
    // 2. **It is on every notice: 26,955 of 26,955.** `TED-FORM` is on 88 % — 3.2k
    //    notices have no form at all — and a form-keyed marker would have reported
    //    those as untyped.
    //
    // The form-keyed version of this marker also undercounted: it listed `3`/`3_SUM`
    // as the award forms and missed `6`/`6_SUM`, the utilities awards, which
    // `NAT_NOTICE` files under the same `7`. Awards in the era: 9,731 by `NAT_NOTICE`
    // (36 %) against roughly 30 % by form.
    DocTypeMarker {
        profiles: &["internal-ojs"],
        field_id: "TED-NAT_NOTICE",
        award: TED_TD_AWARD,
        other: TED_TD_OTHER,
    },
];

/// The section every era files its document type under.
///
/// Measured, and it is the difference between a 1.3 s query and an 11 s timeout:
/// `notice_codes` is keyed `(notice_id, section_id, field_id, ordinal)`, so a probe
/// that gives only `notice_id` scans every code row of the notice — ~100 rows for a
/// legacy form, each tested against six OR'd vocabularies. Pinning `section_id`
/// makes it a three-column primary-key prefix seek.
///
/// All six markers agree on `PROCEDURE` (checked per era on prod: `TXT-TD`,
/// `TED-TD_DOCUMENT_TYPE`, `TED-FORM`, `OPP-070-notice`, `SDK01-NoticeTypeCode`
/// and `DE1-NoticeSubType-SubTypeCode` all land there), which is why it is one
/// constant rather than a per-marker field. A parser that filed the type elsewhere
/// would show up as `untyped` in [`doc_type_sql`], not as a silent zero.
const DOC_TYPE_SECTION: &str = "PROCEDURE";

/// One marker's test, as a version-level predicate:
/// `(<this era's profiles> AND EXISTS(<seek this era's code>))`.
///
/// The profile test sits OUTSIDE the `EXISTS`, and that placement is the whole
/// cost of the query. With the six profile `LIKE`s inside the subquery they were
/// re-evaluated for every code row the probe visited — a legacy form files ~30
/// codes under `PROCEDURE` — and one 83k-version window took over 10 s. Outside,
/// each version pays six cheap string tests and then runs exactly ONE subquery
/// against a single `field_id`. Same answer, measured 1.4 s.
fn marker_term(m: &DocTypeMarker, code_pred: &str) -> String {
    format!(
        "({scope} AND EXISTS(SELECT 1 FROM notice_codes c \
                              WHERE c.notice_id = tv.caused_by_notice_id \
                                AND c.section_id = '{DOC_TYPE_SECTION}' \
                                AND c.field_id = '{field}'{code_pred}))",
        scope = profile_scope(m),
        field = m.field_id,
    )
}

/// A quoted SQL list of codes.
fn code_list(codes: &[&str]) -> String {
    codes.iter().map(|c| format!("'{c}'")).collect::<Vec<_>>().join(", ")
}

/// The marker's profile scope as SQL. Correlated on the outer query's `n`, so it
/// costs nothing beyond the row already joined.
fn profile_scope(m: &DocTypeMarker) -> String {
    let ors: Vec<String> = m.profiles.iter().map(|p| format!("n.profile LIKE '{p}'")).collect();
    format!("({})", ors.join(" OR "))
}

/// "This version's notice publishes a result-announcing document type", per era,
/// OR'd across the markers.
fn award_predicate() -> String {
    let terms: Vec<String> = DOC_TYPE_MARKERS
        .iter()
        .map(|m| marker_term(m, &format!(" AND c.code IN ({})", code_list(m.award))))
        .collect();
    terms.join(" OR ")
}

/// Results materialisation per era, all three of section 3's numbers in ONE pass
/// (issue 243).
///
/// The denominator is read off `notice_codes` — the parse layer's record of what the
/// publisher said — never off `notice_sections`, whose result sections are the very
/// thing the numerator checks the projection produced. That independence is the whole
/// point: this denominator counts an award notice that parsed with zero result
/// sections, which the old one could not (issue 235).
///
/// The numerator and the barren count are `CASE` sums over the SAME rows, so:
///
/// - the numerator is a subset of the denominator BY CONSTRUCTION, and the ratio
///   cannot exceed 1 — the `IMPOSSIBLE` state the old pair could reach when it
///   counted versions below the line and notices above it;
/// - the document-type probe runs once per row instead of three times. Measured on
///   prod: the three separate queries cost 1,346 s + 495 s + 813 s across a full run;
///   an A/B on one window put the merged form 30 % cheaper in query time and three
///   round trips fewer, or roughly 800 s off a 5,566 s run.
///
/// `no_award_content` is the explanatory third number (issue 242): award-typed
/// versions whose notice parsed with no result block at all. Two measured shapes in
/// r2.0.8, which together are its ENTIRE shortfall:
///
/// - the whole body is `OTH_NOT` free-text prose, no structured form at all
///   (~425 per 200k notices; `339168-2017` is one, with 24 language versions of
///   paragraphs and a `TD` code saying "Contract award notice");
/// - the F06 utilities award container is published EMPTY,
///   `<AWARD_CONTRACT_CONTRACT_AWARD_UTILITIES/>` (~465 per 200k; `017037-2017`,
///   which the r209 suite pins as a fixture).
///
/// So `award_notices - with_results - no_award_content` is the number that means "a
/// result block was parsed and the fold still did not write a row". Printing the rate
/// without it invites exactly the wrong conclusion, which is the mistake issue 242
/// opened with.
///
/// **What that column does NOT say** (issue 244, from the first full-corpus run): it
/// is a statement about the PARSE, not about the publisher. The r2.0.8 rows really did
/// publish nothing extractable. The text era's 1,306,514 did publish their awards —
/// winner and value, under numbered headings inside the `TXT-TX` prose body — and
/// until issue 244's parser slice nothing turned that into a result block. Same
/// column, opposite causes, so the rendered line names the split instead of asserting
/// one cause for all of it.
///
/// Reading `notice_sections` for the barren count is deliberate: the point is to
/// compare the published type against the parse, and the comparison is the finding.
/// What the DENOMINATOR must never do is derive itself from the parse.
pub fn awards_sql() -> String {
    awards_template("")
}

/// [`awards_sql`] with `win` spliced into its `WHERE` — one builder for both the
/// catalog and the windowed form, so the two cannot drift into measuring different
/// populations (the drift issue 230 hit when it spliced by text).
///
/// The cheap predicates sit inside the `CASE` sums and the expensive document-type
/// probe in the `WHERE`, which is the order the prod timings argued for: `awards_with`
/// was three times FASTER than `awards_can` despite doing strictly more work, because
/// the planner ran its `lot_results` seek first and the code probe only on survivors.
fn awards_template(win: &str) -> String {
    format!(
        "SELECT n.profile, COUNT(*) AS award_notices, \
                SUM(CASE WHEN EXISTS(SELECT 1 FROM lot_results lr \
                                      WHERE lr.tender_id = tv.tender_id \
                                        AND lr.notice_id = tv.caused_by_notice_id) \
                         THEN 1 ELSE 0 END) AS with_results, \
                SUM(CASE WHEN NOT EXISTS(SELECT 1 FROM notice_sections s \
                                          WHERE s.notice_id = tv.caused_by_notice_id \
                                            AND s.kind IN ('LotResult', 'TenderResult')) \
                         THEN 1 ELSE 0 END) AS no_award_content \
           FROM tender_versions tv JOIN notices n ON n.id = tv.caused_by_notice_id \
          WHERE {win}({award}) \
          GROUP BY n.profile",
        award = award_predicate(),
    )
}

/// Document-type COVERAGE per era: how much of the denominator's input this
/// vocabulary cannot read (issue 235).
///
/// Two counts, both of them ways for an award notice to go missing from section 3
/// without anyone noticing:
///
/// - `unclassified` — the notice carries its era's marker field with a code in
///   neither [`DocTypeMarker::award`] nor [`DocTypeMarker::other`]. A new subtype
///   (`X02` appeared in sdk-1.1x during the measurement) lands here.
/// - `untyped` — the notice carries no marker field at all, so its award-hood is
///   unknown rather than negative. 44 % of the 2008 internal-ojs sample had no
///   `TED-FORM`, and a silent "not an award" would have hidden them.
///
/// A rate is only as good as the share of its population it can classify, so this
/// renders next to section 3 rather than in a separate report.
pub fn doc_type_sql() -> String {
    doc_type_template("")
}

fn doc_type_template(win: &str) -> String {
    let unknown: Vec<String> = DOC_TYPE_MARKERS
        .iter()
        .map(|m| {
            let known: Vec<&str> = m.award.iter().chain(m.other).copied().collect();
            marker_term(m, &format!(" AND c.code NOT IN ({})", code_list(&known)))
        })
        .collect();
    let any: Vec<String> = DOC_TYPE_MARKERS.iter().map(|m| marker_term(m, "")).collect();
    format!(
        "SELECT n.profile, \
                SUM(CASE WHEN {unknown} THEN 1 ELSE 0 END) AS unclassified, \
                SUM(CASE WHEN NOT ({any}) THEN 1 ELSE 0 END) AS untyped \
           FROM tender_versions tv JOIN notices n ON n.id = tv.caused_by_notice_id \
          WHERE {win}1 = 1 \
          GROUP BY n.profile",
        unknown = unknown.join(" OR "),
        any = any.join(" OR "),
    )
}

/// TED↔DÖE merge (ADR-0003), single row. A keyed Tender whose versions include
/// notices from *both* sources is a merge: one procedure, two sources, one
/// Tender. Measured as "of the Tenders DÖE contributes to, how many also carry a
/// TED notice". DÖE only exists 2022-12→, so the overlap window is implicit.
///
/// Driven by the DÖE-touching Tenders alone (a merged Tender's own `source`
/// flips to `ted`, so we must find them through `notices.source = 'doe'`, not
/// `tenders.source`), each then tested for a TED notice with an indexed
/// existence check — bounded by DÖE volume, not the whole canonical layer.
///
/// The leading constant `'all'` column exists so this single-row result has the
/// same `[label, counts…]` shape every other query has, and therefore sums across
/// windows through the same [`sum_profile_counts`] as the rest (issue 230). There
/// is no era split here — a merge is a property of a Tender, not of one notice's
/// profile — so the label is a constant rather than a profile.
pub const MERGE_SQL: &str = "SELECT 'all' AS scope, COUNT(*) AS doe_tenders, \
            SUM(CASE WHEN EXISTS( \
                  SELECT 1 FROM tender_versions v2 JOIN notices n2 ON n2.id = v2.caused_by_notice_id \
                   WHERE v2.tender_id = d.tender_id AND n2.source = 'ted' \
                ) THEN 1 ELSE 0 END) AS merged \
       FROM (SELECT DISTINCT v.tender_id \
               FROM notices n JOIN tender_versions v ON v.caused_by_notice_id = n.id \
              WHERE n.source = 'doe') d";

/// How far back [`fresh_holds_sql`] counts arrivals: 30 days, long enough that a
/// quiet week does not read as a settled bucket and short enough to be a RATE rather
/// than a history.
const FRESH_HOLD_WINDOW_SECS: i64 = 30 * 86_400;

/// Quarantine ARRIVALS per reason over the last [`FRESH_HOLD_WINDOW_SECS`] — which
/// bucket is still being fed, and which is settled residue (issue 246).
///
/// `first_reason IS NULL` is what makes this a rate at all. Issue 87's relabel
/// machinery moves a row's original reason there the first time a failed reclaim
/// rewrites `reason`, so a row that has never been relabelled is a genuine
/// first-time hold, and one that has is a row that already existed under another
/// name. Without that filter the count is dominated by relabel passes: on 2026-08-19
/// the `unrepresentable-value` bucket held 5,185 rows, of which 1,899 had merely
/// been renamed from `unknown-customization` by issue 184's drain and 2,884 arrived
/// on ONE day when the DE-1.x reprocess met sub-cent amounts at scale. The bucket's
/// size answered no question anybody was asking.
///
/// Why it exists: ADR-0010 keeps sub-cent amounts quarantined and says to reopen
/// claim-and-store "if cause F ever grows past a nuisance", nominating issue 171 as
/// the watch — but 171 is a one-off study, so the trigger had no instrument. This is
/// the instrument. `newest` is the last arrival, so a dormant bucket (`not-utf8`, last
/// fed 2026-07-19) is visibly distinct from a live one (`unrepresentable-value`, fed
/// daily).
///
/// Deliberately no denominator: counting the notices ingested in the same window
/// costs a full scan of `notices` (measured: >10 s, over the public endpoint's limit),
/// and the trigger is an arrival RATE, which stands on its own. The day's ingest
/// counts are in the job log beside it.
///
/// Whole-corpus by nature: quarantine rows have no `tender_id`, so no window can
/// slice this — see [`whole_corpus_queries`]. `newest` is a MAX rather than a count
/// for the same reason; nothing sums these rows.
pub fn fresh_holds_sql() -> String {
    format!(
        "SELECT q.reason AS reason, COUNT(*) AS fresh_holds, MAX(q.first_seen) AS newest \
           FROM quarantine q \
          WHERE q.first_reason IS NULL \
            AND q.first_seen > strftime('%s','now') - {FRESH_HOLD_WINDOW_SECS} \
          GROUP BY q.reason \
          ORDER BY fresh_holds DESC"
    )
}

/// The VAT basis of the amounts each era projects (issue 251, option 2).
///
/// `tender_version_amounts.tax_basis` is `'incl'`, `'excl'`, or NULL when the source did
/// not say, and NULL is the honest answer rather than a default — so the useful reading is
/// three-way and the report prints all three. Two questions it answers that nothing else
/// does: whether a re-parse actually populated the column for an era (the r208/r209
/// re-parse this issue still owes), and whether an era's mix is BIASED, which it is until
/// that re-parse lands — `EXCLUDING_VAT` was mapped before `INCLUDING_VAT` was, so the
/// form eras will read excl-heavy for reasons that have nothing to do with their notices.
///
/// Counts amount ROWS across every version, like section 1 counts every version rather
/// than only the current one. An amount row belongs to exactly one version, so the count
/// is additive across windows.
pub fn amount_basis_sql() -> String {
    amount_basis_template("")
}

/// [`amount_basis_sql`] with `win` as its whole `WHERE`, or no `WHERE` at all when `win`
/// is empty — one builder for the catalog and the windowed form, the same reason
/// [`awards_template`] has one.
fn amount_basis_template(win: &str) -> String {
    let scope = if win.is_empty() { String::new() } else { format!("WHERE {win} ") };
    format!(
        "SELECT n.profile, COUNT(*) AS amounts, \
                SUM(CASE WHEN a.tax_basis = 'excl' THEN 1 ELSE 0 END) AS excl, \
                SUM(CASE WHEN a.tax_basis = 'incl' THEN 1 ELSE 0 END) AS incl \
           FROM tender_versions v JOIN notices n ON n.id = v.caused_by_notice_id \
           JOIN tender_version_amounts a ON a.tender_id = v.tender_id AND a.seq = v.seq \
          {scope}GROUP BY n.profile"
    )
}

/// The queries that measure a population no `tender_id` window can slice, so the
/// in-process job runs them ONCE against the whole corpus instead of per window
/// (issue 246). Distinct from [`unwindowed_labels`], which is for a query that
/// cannot be measured at all.
pub fn whole_corpus_queries() -> Vec<(String, String)> {
    vec![("fresh_holds".to_owned(), fresh_holds_sql())]
}

/// Every query the report runs, as `(label, sql)` — the order the bin executes
/// them and the order the JSON records them. The six field labels match
/// [`FIELDS`]; `versions`, `linkage`, `density_can`/`density_with` and `merge`
/// are the fixed rest.
pub fn queries() -> Vec<(String, String)> {
    let mut out = vec![("versions".to_owned(), DENOMINATOR_SQL.to_owned())];
    for spec in &FIELDS {
        out.push((spec.key.to_owned(), field_sql(spec)));
    }
    out.push(("linkage".to_owned(), LINKAGE_SQL.to_owned()));
    // Section 3 proper: the denominator the notice publishes about itself, with its
    // numerator and the barren count in the same pass (issue 243).
    out.push(("awards".to_owned(), awards_sql()));
    out.push(("doc_types".to_owned(), doc_type_sql()));
    // The section→row invariant, under its own name (issue 235): worth keeping,
    // just not a density.
    out.push(("sections_can".to_owned(), SECTIONS_CAN_SQL.to_owned()));
    out.push(("sections_with".to_owned(), SECTIONS_WITH_SQL.to_owned()));
    out.push(("merge".to_owned(), MERGE_SQL.to_owned()));
    // The VAT basis of what the projection wrote (issue 251).
    out.push(("amount_basis".to_owned(), amount_basis_sql()));
    // Whole-corpus queries last: the bin runs every label in this list, and these
    // are the ones the in-process job runs once rather than per window.
    out.extend(whole_corpus_queries());
    out
}

/// A query that can be run over a bounded slice of `tender_versions` and summed
/// (issue 230). Windowing is what makes the measurement affordable at full-corpus
/// scale, and it is affordable for a measured reason rather than an estimated one:
/// each window is small enough to time honestly, and the total is the sum of
/// timed parts instead of a line extrapolated past the page cache.
///
/// Only the version-driven queries qualify — the denominator and the six field
/// probes, which all shape as `FROM tender_versions v … GROUP BY n.profile` and so
/// take an extra `v.tender_id` range predicate without changing meaning. Summing
/// their per-profile counts across disjoint windows gives exactly the unwindowed
/// result, because every version belongs to exactly one window.
///
/// ALL eleven are here now (issue 230, step 3). Each of the four late ones drives
/// from `tender_versions` and so takes the same `v.tender_id` range predicate:
///
/// - `linkage` drives from `seq = 1` versions — one row per Tender, but reached by
///   scanning the version layer, so it is windowed like the rest. Both of its counts
///   (awards, unchained) are additive over disjoint Tender sets.
/// - `density_can` counts award-notice versions; `density_with` counts the subset
///   whose notice materialised `lot_results`. Both are version counts (see
///   [`SECTIONS_WITH_SQL`] — making them share a unit is what let the numerator be
///   windowed at all).
/// - `merge` counts DÖE-touching Tenders and the merged subset. Its inner `SELECT
///   DISTINCT v.tender_id` is windowed, and since Tender ids are disjoint across
///   windows, a `DISTINCT` inside each window sums exactly. This one is the reason
///   the label column exists: the result has no era split, so it carries a constant
///   scope label to share the `[label, counts…]` shape.
///
/// The `DISTINCT` deserves the explicit argument, because a `DISTINCT` summed across
/// windows is normally WRONG: it is only safe here because the distinct key IS the
/// window key. Distinct-on-anything-else — say notices rather than Tenders — would
/// need proof that the key cannot straddle two windows, and `tender_versions` only
/// declares `UNIQUE (tender_id, caused_by_notice_id)`, which does not give it.
pub struct WindowedQuery {
    pub label: String,
    /// SQL carrying a single `{window}` placeholder inside its WHERE clause.
    template: String,
    /// The qualified Tender-id column the window ranges over, e.g. `v.tender_id`.
    /// Per-query because the eleven statements alias `tender_versions` differently
    /// (`v`, `v1`, `tv`) and a hardcoded alias silently becomes "no such table" —
    /// which is how the equivalence test caught this rather than a reviewer.
    column: String,
}

impl WindowedQuery {
    /// The SQL for one half-open window `(lo, hi]` of `tender_versions.tender_id`.
    pub fn sql(&self, lo: i64, hi: i64) -> String {
        let col = &self.column;
        self.template.replace("{window}", &format!("{col} > {lo} AND {col} <= {hi}"))
    }
}

/// The windowable half of [`queries`] — the denominator and the field probes.
pub fn windowed_queries() -> Vec<WindowedQuery> {
    let mut out = vec![WindowedQuery {
        label: "versions".to_owned(),
        template: "SELECT n.profile, COUNT(*) AS versions \
                   FROM tender_versions v JOIN notices n ON n.id = v.caused_by_notice_id \
                   WHERE {window} GROUP BY n.profile"
            .to_owned(),
        column: "v.tender_id".to_owned(),
    }];
    for spec in &FIELDS {
        let filter = spec.predicate.map(|p| format!(" AND {p}")).unwrap_or_default();
        out.push(WindowedQuery {
            label: spec.key.to_owned(),
            template: format!(
                "SELECT n.profile, COUNT(*) AS present \
                 FROM tender_versions v JOIN notices n ON n.id = v.caused_by_notice_id \
                 WHERE {{window}} AND EXISTS (SELECT 1 FROM {satellite} s \
                        WHERE s.tender_id = v.tender_id AND s.seq = v.seq{filter}) \
                 GROUP BY n.profile",
                satellite = spec.satellite,
            ),
            column: "v.tender_id".to_owned(),
        });
    }
    // The four that used to be unmeasurable. Each is the catalog SQL with a range
    // predicate spliced into its existing WHERE, so the two forms cannot drift into
    // measuring different things.
    out.push(WindowedQuery {
        label: "linkage".to_owned(),
        template: LINKAGE_SQL.replace("WHERE v1.seq = 1", "WHERE {window} AND v1.seq = 1"),
        column: "v1.tender_id".to_owned(),
    });
    out.push(WindowedQuery {
        label: "sections_can".to_owned(),
        template: SECTIONS_CAN_SQL.replace("WHERE EXISTS(", "WHERE {window} AND EXISTS("),
        column: "tv.tender_id".to_owned(),
    });
    out.push(WindowedQuery {
        label: "sections_with".to_owned(),
        template: SECTIONS_WITH_SQL.replace("WHERE EXISTS(", "WHERE {window} AND EXISTS("),
        column: "tv.tender_id".to_owned(),
    });
    // The document-type queries build their windowed form from the SAME builder as the
    // catalog form, with the predicate passed in rather than spliced by text — one
    // source, so windowed and unwindowed cannot come to measure different populations.
    out.push(WindowedQuery {
        label: "awards".to_owned(),
        template: awards_template("{window} AND "),
        column: "tv.tender_id".to_owned(),
    });
    out.push(WindowedQuery {
        label: "doc_types".to_owned(),
        template: doc_type_template("{window} AND "),
        column: "tv.tender_id".to_owned(),
    });
    out.push(WindowedQuery {
        label: "merge".to_owned(),
        template: MERGE_SQL.replace("WHERE n.source = 'doe'", "WHERE {window} AND n.source = 'doe'"),
        // Spliced into the INNER `SELECT DISTINCT v.tender_id`, where `v` is in scope
        // — windowing the driver, which is what makes the DISTINCT sum exactly.
        column: "v.tender_id".to_owned(),
    });
    out.push(WindowedQuery {
        label: "amount_basis".to_owned(),
        template: amount_basis_template("{window}"),
        column: "v.tender_id".to_owned(),
    });
    out
}

/// The labels [`windowed_queries`] does NOT cover, so a caller can report them as
/// unmeasured rather than silently dropping them. Empty since step 3 — every query
/// is windowed — and kept rather than deleted for two reasons: the caller's
/// "measure these, declare those unmeasured" split is the right shape whether or
/// not the second half is currently empty, and a query added later that cannot be
/// windowed belongs here instead of being quietly left out of the report.
pub fn unwindowed_labels() -> Vec<String> {
    Vec::new()
}

/// Sum labelled count rows across windows into the single result set the assembler
/// expects: `[label, count…]` per label, label-sorted so the report is stable run
/// to run.
///
/// Column 0 is the label (an era profile, or `merge`'s constant scope) and EVERY
/// column after it is summed, so a query reporting two counts side by side —
/// `linkage`'s awards and unchained, `merge`'s DÖE Tenders and merged — folds
/// correctly without a second summing function to keep in step with this one.
///
/// A row narrower than the widest is padded with zeros rather than dropped: a
/// window matching no unchained awards may return fewer columns, and treating that
/// as "no data" instead of "no rows of that kind" is the confusion this module
/// spends its effort avoiding.
pub fn sum_profile_counts(windows: &[Rows]) -> Rows {
    let mut totals: std::collections::BTreeMap<String, Vec<i64>> = Default::default();
    for rows in windows {
        for row in rows {
            let Some(label) = row.first().and_then(|v| v.as_str()) else { continue };
            let counts = totals.entry(label.to_owned()).or_default();
            for (i, cell) in row.iter().skip(1).enumerate() {
                if counts.len() <= i {
                    counts.resize(i + 1, 0);
                }
                counts[i] += cell.as_i64().unwrap_or(0);
            }
        }
    }
    let width = totals.values().map(Vec::len).max().unwrap_or(0);
    totals
        .into_iter()
        .map(|(label, mut counts)| {
            counts.resize(width, 0);
            let mut row = vec![Value::String(label)];
            row.extend(counts.into_iter().map(|n| serde_json::json!(n)));
            row
        })
        .collect()
}

// ------------------------------------------------------------------- eras

/// A human era label for a mapping profile — the same era split the dashboard
/// shows, so a reader can line the two up. Unknown profiles pass through as-is
/// rather than being hidden.
pub fn era_of(profile: &str) -> &'static str {
    match profile {
        "text" => "text 1993–2010",
        "internal-ojs" => "INTERNAL_OJS 2008",
        "ted-export-r208" => "TED_EXPORT r2.0.8",
        "ted-export-r209" => "TED_EXPORT r2.0.9",
        "eforms:eforms-sdk-0.1" => "DÖE sdk-0.1 island",
        p if p.starts_with("eforms:eforms-de") => "eForms-DE",
        p if p.starts_with("eforms:") => "eForms EU",
        _ => "other",
    }
}

/// The text table's row label — the coarse era, but distinguishing the profiles
/// that share one (the several eForms SDK versions, DE dialects) by their own
/// suffix, so three "eForms EU" rows do not render identically. The JSON keeps
/// the coarse `era` and the raw `profile` separately.
fn display_era(profile: &str) -> String {
    match profile {
        "text" => "text 1993–2010".to_owned(),
        "internal-ojs" => "INTERNAL_OJS 2008".to_owned(),
        "ted-export-r208" => "TED_EXPORT r2.0.8".to_owned(),
        "ted-export-r209" => "TED_EXPORT r2.0.9".to_owned(),
        "eforms:eforms-sdk-0.1" => "DÖE sdk-0.1 island".to_owned(),
        p => p.strip_prefix("eforms:").map_or_else(|| era_of(p).to_owned(), str::to_owned),
    }
}

// ------------------------------------------------------------------- model

/// One era's field completeness: the denominator and the per-field present-count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletenessRow {
    pub profile: String,
    pub versions: u64,
    /// Present-count per field, in [`FIELDS`] order (title, buyer, value, cpv,
    /// deadline, winner).
    pub present: [u64; 6],
}

/// One era's award→notice linkage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkageRow {
    pub profile: String,
    pub awards: u64,
    pub unchained: u64,
}

/// One era's results materialisation: award notices by their own PUBLISHED type
/// (issue 235), and how many of them produced a canonical `lot_results` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DensityRow {
    pub profile: String,
    /// Versions whose notice's document type announces a result — see
    /// [`DOC_TYPE_MARKERS`]. Independent of what the projection wrote, which is
    /// what lets this rate fall below 100 %.
    pub award_notices: u64,
    pub with_results: u64,
    /// Of the denominator, the versions whose notice published no award block at
    /// all — see [`awards_sql`]. Not a failure of ours, and the difference
    /// between a readable rate and a misleading one.
    pub no_award_content: u64,
}

/// One era's projection invariant: of the versions whose notice parsed WITH a
/// result section, how many carry a `lot_results` row.
///
/// This is the pair that used to be called density (issue 235). It measures the
/// projection against the parse layer — "did we write what we parsed" — which is a
/// real invariant that caught a real defect (sdk-0.1's `TenderResult` sections
/// falling out of the denominator, issue 230), and is NOT the question section 3
/// asks. Kept under its own name rather than deleted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvariantRow {
    pub profile: String,
    pub with_sections: u64,
    pub with_rows: u64,
}

/// One era's document-type coverage: the share of the section-3 population whose
/// award-hood this vocabulary could not read. See [`doc_type_sql`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocTypeRow {
    pub profile: String,
    /// Carries its era's marker field, with a code in neither list.
    pub unclassified: u64,
    /// Carries no marker field at all — award-hood unknown, not negative.
    pub untyped: u64,
}

/// One era's amounts by VAT basis (issue 251, option 2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasisRow {
    pub profile: String,
    pub amounts: u64,
    pub excl: u64,
    pub incl: u64,
}

impl BasisRow {
    /// Amounts whose source stated no basis. Derived rather than measured, so it cannot
    /// disagree with the three counts it is derived from.
    pub fn unstated(&self) -> u64 {
        self.amounts.saturating_sub(self.excl).saturating_sub(self.incl)
    }
}

/// One quarantine reason's arrival count over the report's fresh-hold window
/// (issue 246).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreshHoldRow {
    pub reason: String,
    /// Rows first held under this reason inside the window — never relabelled, so
    /// this is arrivals rather than bucket size.
    pub fresh_holds: u64,
    /// The newest arrival's `first_seen`, which is what separates a live bucket from
    /// settled residue. Unix seconds; 0 when the row somehow carries none.
    pub newest: u64,
}

/// The TED↔DÖE merge tally.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Merge {
    pub doe_tenders: u64,
    pub merged: u64,
}

/// A whole measurement, ready to render.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub base_url: String,
    pub completeness: Vec<CompletenessRow>,
    pub linkage: Vec<LinkageRow>,
    pub density: Vec<DensityRow>,
    /// The section→row invariant, per era (issue 235).
    pub invariant: Vec<InvariantRow>,
    /// How much of each era section 3's vocabulary can classify (issue 235).
    pub doc_types: Vec<DocTypeRow>,
    pub merge: Merge,
    /// Quarantine arrivals per reason over the last 30 days (issue 246).
    pub fresh_holds: Vec<FreshHoldRow>,
    /// Amounts by VAT basis, per era (issue 251).
    pub amount_basis: Vec<BasisRow>,
    /// Query labels that did NOT run (issue 230). A failed query used to arrive
    /// as empty rows, indistinguishable from a query that legitimately returned
    /// none — so the render printed real-looking zeros ("DÖE procedure Tenders:
    /// 0") for numbers nobody measured. The exit code and stderr always told the
    /// truth; stdout did not, and stdout is what gets pasted into a comment.
    pub unmeasured: Vec<String>,
}

// ------------------------------------------------------------------- assembly

/// Read a JSON cell as a count, tolerating integer/float/null (a `SUM` over no
/// rows is JSON `null`).
fn as_u64(cell: Option<&Value>) -> u64 {
    match cell {
        Some(v) => v
            .as_u64()
            .or_else(|| v.as_i64().map(|i| i.max(0) as u64))
            .or_else(|| v.as_f64().map(|f| f.max(0.0) as u64))
            .unwrap_or(0),
        None => 0,
    }
}

/// Read a JSON cell as a profile string.
fn as_str(cell: Option<&Value>) -> String {
    cell.and_then(Value::as_str).unwrap_or_default().to_owned()
}

/// `(profile → count)` from a two-column `profile, count` result.
fn count_by_profile(rows: &Rows) -> std::collections::BTreeMap<String, u64> {
    rows.iter()
        .map(|r| (as_str(r.first()), as_u64(r.get(1))))
        .collect()
}

/// The named result sets the report needs, in [`queries`] order.
#[derive(Debug)]
pub struct Raw {
    pub versions: Rows,
    pub fields: [Rows; 6],
    pub linkage: Rows,
    /// Section 3's three counts per era, one row each (issue 243).
    pub awards: Rows,
    pub doc_types: Rows,
    pub sections_can: Rows,
    pub sections_with: Rows,
    pub merge: Rows,
    /// Quarantine arrivals per reason over the last 30 days (issue 246).
    pub fresh_holds: Rows,
    /// Amounts by VAT basis, per era (issue 251).
    pub amount_basis: Rows,
    /// Labels whose query never ran (issue 230), in the order collected.
    pub unmeasured: Vec<String>,
}

impl Raw {
    /// Collect the ordered `(label, rows)` results the bin gathered into the
    /// named slots the assembler expects, so the transport never has to know the
    /// query shapes.
    ///
    /// `None` rows mean the query FAILED; `Some(vec![])` means it ran and matched
    /// nothing. Those are different claims and the report must not merge them
    /// (issue 230) — a failed query is recorded in [`Raw::unmeasured`] and the
    /// slot is filled with no rows so assembly still proceeds.
    pub fn from_labelled(mut results: Vec<(String, Option<Rows>)>) -> Result<Raw, String> {
        let mut unmeasured = Vec::new();
        let mut take = |label: &str, unmeasured: &mut Vec<String>| {
            let pos = results
                .iter()
                .position(|(l, _)| l == label)
                .ok_or_else(|| format!("missing result set: {label}"))?;
            Ok::<Rows, String>(match results.remove(pos).1 {
                Some(rows) => rows,
                None => {
                    unmeasured.push(label.to_owned());
                    Vec::new()
                }
            })
        };
        Ok(Raw {
            versions: take("versions", &mut unmeasured)?,
            fields: [
                take("title", &mut unmeasured)?,
                take("buyer", &mut unmeasured)?,
                take("value", &mut unmeasured)?,
                take("cpv", &mut unmeasured)?,
                take("deadline", &mut unmeasured)?,
                take("winner", &mut unmeasured)?,
            ],
            linkage: take("linkage", &mut unmeasured)?,
            awards: take("awards", &mut unmeasured)?,
            doc_types: take("doc_types", &mut unmeasured)?,
            sections_can: take("sections_can", &mut unmeasured)?,
            sections_with: take("sections_with", &mut unmeasured)?,
            merge: take("merge", &mut unmeasured)?,
            fresh_holds: take("fresh_holds", &mut unmeasured)?,
            amount_basis: take("amount_basis", &mut unmeasured)?,
            unmeasured,
        })
    }
}

/// Turn the raw result sets into a [`Report`]. Every era that appears as a
/// denominator becomes one completeness row, with each field looked up by
/// profile (absent ⇒ zero present).
pub fn assemble(base_url: &str, raw: &Raw) -> Report {
    let by_field: Vec<_> = raw.fields.iter().map(count_by_profile).collect();

    let mut completeness: Vec<CompletenessRow> = raw
        .versions
        .iter()
        .map(|r| {
            let profile = as_str(r.first());
            let mut present = [0u64; 6];
            for (i, field) in by_field.iter().enumerate() {
                present[i] = field.get(&profile).copied().unwrap_or(0);
            }
            CompletenessRow { versions: as_u64(r.get(1)), present, profile }
        })
        .collect();
    completeness.sort_by(|a, b| a.profile.cmp(&b.profile));

    let mut linkage: Vec<LinkageRow> = raw
        .linkage
        .iter()
        .map(|r| LinkageRow { profile: as_str(r.first()), awards: as_u64(r.get(1)), unchained: as_u64(r.get(2)) })
        .collect();
    linkage.sort_by(|a, b| a.profile.cmp(&b.profile));

    // Density is two separately-measured halves (denominator from the notice's
    // published document type, numerator from `lot_results`) combined by profile —
    // every profile in either half becomes one row, a missing numerator being 0
    // (the gap the metric exists to show).
    let mut density: Vec<DensityRow> = raw
        .awards
        .iter()
        .map(|r| DensityRow {
            profile: as_str(r.first()),
            award_notices: as_u64(r.get(1)),
            with_results: as_u64(r.get(2)),
            no_award_content: as_u64(r.get(3)),
        })
        .collect();
    // Profile-sorted, because one query no longer imposes an order the way merging two
    // keyed maps did, and a report that reorders its rows run to run is unreadable.
    density.sort_by(|a, b| a.profile.cmp(&b.profile));

    // The section→row invariant, same two-halves shape under its own name.
    let sec = count_by_profile(&raw.sections_can);
    let sec_rows = count_by_profile(&raw.sections_with);
    let mut profiles: std::collections::BTreeSet<String> = sec.keys().cloned().collect();
    profiles.extend(sec_rows.keys().cloned());
    let invariant: Vec<InvariantRow> = profiles
        .into_iter()
        .map(|profile| InvariantRow {
            with_sections: sec.get(&profile).copied().unwrap_or(0),
            with_rows: sec_rows.get(&profile).copied().unwrap_or(0),
            profile,
        })
        .collect();

    let mut doc_types: Vec<DocTypeRow> = raw
        .doc_types
        .iter()
        .map(|r| DocTypeRow {
            profile: as_str(r.first()),
            unclassified: as_u64(r.get(1)),
            untyped: as_u64(r.get(2)),
        })
        .collect();
    doc_types.sort_by(|a, b| a.profile.cmp(&b.profile));

    // Column 0 is the constant `'all'` scope label that lets the merge row sum
    // across windows like every other result set, so the counts start at 1.
    let merge = raw.merge.first().map_or(Merge::default(), |r| Merge {
        doe_tenders: as_u64(r.get(1)),
        merged: as_u64(r.get(2)),
    });

    let mut amount_basis: Vec<BasisRow> = raw
        .amount_basis
        .iter()
        .map(|r| BasisRow {
            profile: as_str(r.first()),
            amounts: as_u64(r.get(1)),
            excl: as_u64(r.get(2)),
            incl: as_u64(r.get(3)),
        })
        .collect();
    amount_basis.sort_by(|a, b| a.profile.cmp(&b.profile));

    // Arrivals, already ordered by the query (busiest reason first) — kept in that
    // order rather than re-sorted, because "which bucket is being fed" is the
    // question and the SQL answers it.
    let fresh_holds: Vec<FreshHoldRow> = raw
        .fresh_holds
        .iter()
        .map(|r| FreshHoldRow {
            reason: as_str(r.first()),
            fresh_holds: as_u64(r.get(1)),
            newest: as_u64(r.get(2)),
        })
        .collect();

    Report {
        base_url: base_url.to_owned(),
        completeness,
        linkage,
        density,
        invariant,
        doc_types,
        merge,
        fresh_holds,
        amount_basis,
        unmeasured: raw.unmeasured.clone(),
    }
}

// ------------------------------------------------------------------- rendering

/// `num/den` as a percentage string, or `—` where the denominator is zero (no
/// rate to report, not zero percent).
fn pct(num: u64, den: u64) -> String {
    if den == 0 { "—".to_owned() } else { format!("{:.1}%", 100.0 * num as f64 / den as f64) }
}

/// The human report.
pub fn render_text(report: &Report) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "tender-db data-quality — {}", report.base_url);
    let _ = writeln!(
        out,
        "unit: tender-version (≈ one per notice); era = mapping profile of the version's notice"
    );
    // Say it in the report itself, not only on stderr (issue 230): stdout is what
    // gets pasted into a comment, and an unmeasured section that renders as zeros
    // is the reading that misleads.
    if !report.unmeasured.is_empty() {
        let _ = writeln!(
            out,
            "INCOMPLETE: {} of {} queries did not run ({}). Sections below that depend on them are \
             UNMEASURED, not zero.",
            report.unmeasured.len(),
            // The catalog's own length, not a literal: a query added to `queries()`
            // must not leave this banner quietly claiming a total that no longer
            // matches how many ran.
            queries().len(),
            report.unmeasured.join(", ")
        );
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "== 1. Field completeness (share of versions carrying each field) ==");
    let _ = writeln!(
        out,
        "  {:<30} {:>11}  {:>6} {:>6} {:>6} {:>6} {:>8} {:>6}",
        "era", "versions", "title", "buyer", "value", "cpv", "deadline", "winner"
    );
    for row in &report.completeness {
        let v = row.versions;
        let _ = writeln!(
            out,
            "  {:<30} {:>11}  {:>6} {:>6} {:>6} {:>6} {:>8} {:>6}",
            display_era(&row.profile),
            group(v),
            pct(row.present[0], v),
            pct(row.present[1], v),
            pct(row.present[2], v),
            pct(row.present[3], v),
            pct(row.present[4], v),
            pct(row.present[5], v),
        );
    }

    let _ = writeln!(out, "\n== 2. Award→notice linkage (award Tenders chained to a contract notice) ==");
    let _ = writeln!(out, "  {:<30} {:>10} {:>10} {:>8}", "era", "awards", "unchained", "linked");
    for row in &report.linkage {
        let linked = row.awards.saturating_sub(row.unchained);
        let _ = writeln!(
            out,
            "  {:<30} {:>10} {:>10} {:>8}",
            display_era(&row.profile), group(row.awards), group(row.unchained), pct(linked, row.awards)
        );
    }

    let _ = writeln!(
        out,
        "\n== 3. Results materialisation (notices whose PUBLISHED type announces a result → lot_results) =="
    );
    let _ = writeln!(
        out,
        "  {:<30} {:>10} {:>14} {:>8} {:>16}",
        "era", "award-notices", "with lot_results", "density", "no block parsed"
    );
    let mut impossible = 0usize;
    for row in &report.density {
        // A numerator above its denominator is not a high rate, it is a
        // CONTRADICTION: the two halves are measuring different populations. Neither
        // `—` (which reads as "nothing to report") nor a >100% figure (which reads
        // as a rate) says so, and this report has already been burned once by a
        // number that could not distinguish two states. Name it.
        let rate = if row.with_results > row.award_notices {
            impossible += 1;
            "IMPOSSIBLE".to_owned()
        } else {
            pct(row.with_results, row.award_notices)
        };
        let _ = writeln!(
            out,
            "  {:<30} {:>10} {:>14} {:>8} {:>16}",
            display_era(&row.profile),
            group(row.award_notices),
            group(row.with_results),
            rate,
            group(row.no_award_content)
        );
    }
    if impossible > 0 {
        let _ = writeln!(
            out,
            "  IMPOSSIBLE ({impossible} era(s)): more notices carry results than are counted as \
             award notices, so the two halves disagree about the population. Both halves share one \
             award predicate ({markers} document-type markers), so a contradiction here means the \
             predicate itself changed between the two queries.",
            markers = DOC_TYPE_MARKERS.len()
        );
    }
    // The column that makes the rate readable (issue 242), with the correction the
    // first full run forced (issue 244): this counts award notices with no result
    // block IN THE PARSE, which is a statement about our parse and NOT about what
    // the publisher shipped. The two are different, and both occur at scale:
    //
    // - r2.0.8's 14,532 really did publish nothing extractable — an `OTH_NOT` prose
    //   body, or an empty `<AWARD_CONTRACT_CONTRACT_AWARD_UTILITIES/>` container.
    // - the text era's 1,306,514 DID publish their awards, inside the prose body
    //   (`TXT-TX`) under TED's own numbered headings — `SECTION V: AWARD OF CONTRACT`
    //   / `V.3) …AWARDED: <name>` in the later shape, ` 6.  Supplier(s): …` in the
    //   1993 one. Nothing parses either into a result block yet.
    //
    // The first draft of this line claimed "nothing can project those" for the whole
    // column. That was false for 1.3M notices and exactly the kind of confident
    // wrong summary this report exists to prevent, so the wording now states the
    // measurement and points at the split rather than asserting a cause.
    if report.unmeasured.iter().any(|l| l == "awards") {
        let _ = writeln!(
            out,
            "  no result block parsed: UNMEASURED — the `awards` query did not run, so the \
             density above cannot be split by cause."
        );
    } else {
        let barren: u64 = report.density.iter().map(|r| r.no_award_content).sum();
        let missing: u64 = report
            .density
            .iter()
            .map(|r| r.award_notices.saturating_sub(r.with_results).saturating_sub(r.no_award_content))
            .sum();
        let _ = writeln!(
            out,
            "  no result block parsed: {} award notice(s) announce a result and have no result \
             block in the parse layer. That is one of two different things per era — the notice \
             published nothing extractable (r2.0.8: `OTH_NOT` prose bodies, empty F06 containers), \
             or its era's award block is not extracted yet (the text era publishes winner and value \
             inside its prose body — issue 244). Award notices whose result block IS parsed and still did not \
             materialise, i.e. the fold's own shortfall: {}.",
            group(barren),
            group(missing)
        );
    }
    // A rate is only worth as much as the share of its population the vocabulary
    // could classify — printed with the rate, not in an appendix (issue 235).
    let unread: Vec<&DocTypeRow> =
        report.doc_types.iter().filter(|r| r.unclassified > 0 || r.untyped > 0).collect();
    if report.unmeasured.iter().any(|l| l == "doc_types") {
        let _ = writeln!(out, "  document-type coverage: UNMEASURED — the `doc_types` query did not run.");
    } else if unread.is_empty() {
        let _ = writeln!(out, "  document-type coverage: every version's notice carries a classified type.");
    } else {
        let _ = writeln!(
            out,
            "  document-type coverage — versions section 3 CANNOT classify (not counted as awards):"
        );
        let _ = writeln!(out, "  {:<30} {:>14} {:>10}", "era", "unclassified", "untyped");
        for row in unread {
            let _ = writeln!(
                out,
                "  {:<30} {:>14} {:>10}",
                display_era(&row.profile), group(row.unclassified), group(row.untyped)
            );
        }
    }

    let _ = writeln!(
        out,
        "\n== 3b. Projection invariant (notices that PARSED a result section → lot_results) =="
    );
    let _ = writeln!(out, "  {:<30} {:>10} {:>14} {:>8}", "era", "w/ sections", "with lot_results", "written");
    for row in &report.invariant {
        let _ = writeln!(
            out,
            "  {:<30} {:>10} {:>14} {:>8}",
            display_era(&row.profile),
            group(row.with_sections),
            group(row.with_rows),
            pct(row.with_rows, row.with_sections)
        );
    }

    let _ = writeln!(out, "\n== 4. TED↔DÖE merge (of DÖE procedures, share also seen on TED) ==");
    if report.unmeasured.iter().any(|l| l == "merge") {
        // Printing "0; merged with TED: 0" for a query that never ran was the one
        // place this report stated a number it had not measured (issue 230).
        let _ = writeln!(out, "  UNMEASURED — the `merge` query did not run.");
    } else {
        let m = report.merge;
        let _ = writeln!(
            out,
            "  DÖE procedure Tenders: {}; merged with TED: {} ({})",
            group(m.doe_tenders), group(m.merged), pct(m.merged, m.doe_tenders)
        );
    }

    // Section 5 (issue 246): which quarantine buckets are still being FED. Bucket
    // size cannot answer that — relabel passes move rows between reasons, and one
    // campaign day can dominate a whole bucket — so this counts arrivals that were
    // never relabelled, and prints the newest one so a settled bucket is visibly
    // settled. ADR-0010's "reopen if cause F grows past a nuisance" is the trigger
    // this exists to make observable.
    let _ = writeln!(
        out,
        "\n== 5. Quarantine arrivals (first-time holds in the last {} days, per reason) ==",
        FRESH_HOLD_WINDOW_SECS / 86_400
    );
    if report.unmeasured.iter().any(|l| l == "fresh_holds") {
        let _ = writeln!(out, "  UNMEASURED — the `fresh_holds` query did not run.");
    } else if report.fresh_holds.is_empty() {
        let _ = writeln!(
            out,
            "  none — no member was held for the first time in the window. Every row in the \
             quarantine ledger predates it."
        );
    } else {
        let _ = writeln!(out, "  {:<52}{:>10}  {}", "reason", "arrivals", "newest");
        for r in &report.fresh_holds {
            let mut reason = r.reason.clone();
            if reason.chars().count() > 50 {
                reason = reason.chars().take(49).collect::<String>() + "…";
            }
            let _ = writeln!(
                out,
                "  {:<52}{:>10}  {}",
                reason,
                group(r.fresh_holds),
                day_utc(r.newest)
            );
        }
        let _ = writeln!(
            out,
            "  Arrivals, not bucket size: a row that a failed reclaim merely RENAMED is excluded \
             (it carries `first_reason`), so this is what is still coming in. A reason whose newest \
             arrival is weeks old is settled residue; one fed daily is a live cost — which is the \
             distinction ADR-0010's reopen trigger needs and the bucket totals cannot make."
        );
    }

    // Section 6 (issue 251): whether an amount's source said what its figure INCLUDES.
    // Three-way on purpose — `unstated` is the honest third answer, and reading it as
    // either basis is the mistake this section exists to prevent.
    let _ = writeln!(out, "\n== 6. Amount VAT basis (share of projected amounts stating one) ==");
    if report.unmeasured.iter().any(|l| l == "amount_basis") {
        let _ = writeln!(out, "  UNMEASURED — the `amount_basis` query did not run.");
    } else if report.amount_basis.is_empty() {
        let _ = writeln!(out, "  none — no era projects an amount, which would itself be a finding.");
    } else {
        let _ = writeln!(
            out,
            "  {:<30} {:>11} {:>10} {:>10} {:>10} {:>8}",
            "era", "amounts", "excl", "incl", "unstated", "stated"
        );
        for r in &report.amount_basis {
            let stated = r.excl + r.incl;
            let _ = writeln!(
                out,
                "  {:<30} {:>11} {:>10} {:>10} {:>10} {:>8}",
                display_era(&r.profile),
                group(r.amounts),
                group(r.excl),
                group(r.incl),
                group(r.unstated()),
                pct(stated, r.amounts),
            );
        }
        let _ = writeln!(
            out,
            "  An era's incl/excl MIX is only meaningful once both markers have been mapped and \
             the era re-parsed: the form eras recorded `EXCLUDING_VAT` before `INCLUDING_VAT` \
             existed, so an excl-heavy split there is a parser history, not a procurement fact \
             (issue 251)."
        );
    }
    out
}

/// A unix timestamp as `YYYY-MM-DD` (UTC), or `—` for none. Days are the resolution
/// this report reads at; an exact instant would be noise in a table of rates.
fn day_utc(unix: u64) -> String {
    if unix == 0 {
        return "—".to_owned();
    }
    let (y, m, d) = crate::fetch::civil_date(unix as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The machine report.
pub fn render_json(report: &Report) -> String {
    let field_keys: Vec<&str> = FIELDS.iter().map(|f| f.key).collect();
    let completeness: Vec<Value> = report
        .completeness
        .iter()
        .map(|r| {
            let rates: serde_json::Map<String, Value> = field_keys
                .iter()
                .enumerate()
                .map(|(i, k)| ((*k).to_owned(), json!(rate(r.present[i], r.versions))))
                .collect();
            json!({
                "profile": r.profile,
                "era": era_of(&r.profile),
                "versions": r.versions,
                "present": field_keys.iter().enumerate()
                    .map(|(i, k)| ((*k).to_owned(), json!(r.present[i])))
                    .collect::<serde_json::Map<_, _>>(),
                "rate": rates,
            })
        })
        .collect();
    let linkage: Vec<Value> = report
        .linkage
        .iter()
        .map(|r| json!({
            "profile": r.profile,
            "era": era_of(&r.profile),
            "awards": r.awards,
            "unchained": r.unchained,
            "linked_rate": rate(r.awards.saturating_sub(r.unchained), r.awards),
        }))
        .collect();
    let density: Vec<Value> = report
        .density
        .iter()
        .map(|r| json!({
            "profile": r.profile,
            "era": era_of(&r.profile),
            "award_notices": r.award_notices,
            "with_results": r.with_results,
            "density": rate(r.with_results, r.award_notices),
            // Issue 242: the part of the denominator nobody could project, and the
            // part that is genuinely ours. A consumer plotting `density` alone would
            // read a publication-quality floor as our defect rate.
            "no_award_content": r.no_award_content,
            "unprojected": r.award_notices.saturating_sub(r.with_results).saturating_sub(r.no_award_content),
        }))
        .collect();
    let invariant: Vec<Value> = report
        .invariant
        .iter()
        .map(|r| json!({
            "profile": r.profile,
            "era": era_of(&r.profile),
            "with_sections": r.with_sections,
            "with_rows": r.with_rows,
            "written_rate": rate(r.with_rows, r.with_sections),
        }))
        .collect();
    let doc_types: Vec<Value> = report
        .doc_types
        .iter()
        .map(|r| json!({
            "profile": r.profile,
            "era": era_of(&r.profile),
            "unclassified": r.unclassified,
            "untyped": r.untyped,
        }))
        .collect();
    // Section 5 has been missing from this surface since issue 246 added it to the text
    // report — noticed while wiring section 6 into both. A consumer reading the JSON could
    // not see the quarantine arrival rate at all, which is the one number ADR-0010's
    // reopen trigger is watched by.
    let fresh_holds: Vec<Value> = report
        .fresh_holds
        .iter()
        .map(|r| json!({
            "reason": r.reason,
            "fresh_holds": r.fresh_holds,
            "newest": r.newest,
            "newest_day": day_utc(r.newest),
        }))
        .collect();
    let amount_basis: Vec<Value> = report
        .amount_basis
        .iter()
        .map(|r| json!({
            "profile": r.profile,
            "era": era_of(&r.profile),
            "amounts": r.amounts,
            "excl": r.excl,
            "incl": r.incl,
            // Derived from the three above, so a consumer cannot compute a different
            // third answer than the text report prints.
            "unstated": r.unstated(),
            "stated_rate": rate(r.excl + r.incl, r.amounts),
        }))
        .collect();
    let value = json!({
        "base_url": report.base_url,
        "unit": "tender-version",
        "completeness": completeness,
        "award_linkage": linkage,
        "results_density": density,
        "sections_to_rows": invariant,
        "doc_type_coverage": doc_types,
        "quarantine_arrivals": {
            "window_days": FRESH_HOLD_WINDOW_SECS / 86_400,
            "reasons": fresh_holds,
        },
        "amount_vat_basis": amount_basis,
        "ted_doe_merge": {
            "doe_tenders": report.merge.doe_tenders,
            "merged": report.merge.merged,
            "rate": rate(report.merge.merged, report.merge.doe_tenders),
        },
    });
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".into())
}

/// A ratio in `[0,1]`, or `null` where the denominator is zero — the JSON twin
/// of [`pct`].
fn rate(num: u64, den: u64) -> Option<f64> {
    (den != 0).then(|| num as f64 / den as f64)
}

/// Group an integer with thin thousands separators, so a 3.5 M count is
/// readable in the text table.
fn group(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

// ------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn era_labels_cover_every_profile_family() {
        assert_eq!(era_of("text"), "text 1993–2010");
        assert_eq!(era_of("internal-ojs"), "INTERNAL_OJS 2008");
        assert_eq!(era_of("ted-export-r208"), "TED_EXPORT r2.0.8");
        assert_eq!(era_of("ted-export-r209"), "TED_EXPORT r2.0.9");
        assert_eq!(era_of("eforms:eforms-sdk-1.13"), "eForms EU");
        assert_eq!(era_of("eforms:eforms-de-2.1@eforms-sdk-1.13"), "eForms-DE");
        // The DÖE numeric island dialect is its own era, not lumped with eForms EU.
        assert_eq!(era_of("eforms:eforms-sdk-0.1"), "DÖE sdk-0.1 island");
        assert_eq!(era_of("something-new"), "other");
    }

    #[test]
    fn field_sql_narrows_only_when_a_predicate_exists() {
        let title = field_sql(&FIELDS[0]);
        assert!(title.contains("tender_version_texts"));
        assert!(title.contains("AND field = 'title'"), "{title}");
        // `value` has no predicate — any amount row counts, so the EXISTS carries
        // only the version join.
        let value = field_sql(&FIELDS[2]);
        assert!(value.contains("tender_version_amounts"));
        assert!(!value.contains("AND field"), "{value}");

        // Issue 230: the derived-table shape must never come back. It is the
        // documented turso trap (re-evaluated per outer row) and it cost >50
        // minutes on one field at full-corpus scale.
        for spec in FIELDS.iter() {
            let sql = field_sql(spec);
            assert!(!sql.contains("SELECT DISTINCT"), "no derived DISTINCT table: {sql}");
            assert!(sql.contains("WHERE EXISTS (SELECT 1 FROM"), "indexed EXISTS: {sql}");
            assert!(
                sql.contains("s.tender_id = v.tender_id AND s.seq = v.seq"),
                "the probe must hit the satellite by-version index: {sql}"
            );
        }
    }

    #[test]
    fn queries_are_labelled_in_execution_order() {
        let q = queries();
        let labels: Vec<&str> = q.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(
            labels,
            [
                "versions",
                "title",
                "buyer",
                "value",
                "cpv",
                "deadline",
                "winner",
                "linkage",
                // Section 3: the notice's own published type (issue 235), its
                // materialisation, and the "published nothing" split (issue 242) — one
                // statement since issue 243 merged them.
                "awards",
                "doc_types",
                // … and the section→row invariant that used to wear its name.
                "sections_can",
                "sections_with",
                "merge",
                // The VAT basis of what the projection wrote (issue 251).
                "amount_basis",
                "fresh_holds",
            ]
        );
    }

    /// Issue 251 option 2: the VAT basis section must read three ways, and must say that
    /// a mix is not yet a fact.
    #[test]
    fn the_vat_basis_section_reads_three_ways_and_warns_about_the_bias() {
        let all = || -> Vec<(String, Option<Rows>)> {
            queries().into_iter().map(|(l, _)| (l, Some(Vec::new()))).collect()
        };
        let mut ran = all();
        ran.iter_mut().find(|(l, _)| l == "amount_basis").expect("label").1 = Some(vec![
            // 1,000 amounts, 600 excl, 150 incl — so 250 unstated, which is DERIVED and
            // must not be a fourth measured column that can disagree with the three.
            vec![json!("text"), json!(1_000), json!(600), json!(150)],
            // An era that states nothing at all: the section must show 0 % rather than
            // omitting the era, because "no basis anywhere" is the finding.
            vec![json!("ted-export-r208"), json!(400), json!(0), json!(0)],
        ]);
        let raw = Raw::from_labelled(ran).expect("labelled");
        let report = assemble("(t)", &raw);

        assert_eq!(
            report.amount_basis.iter().map(|r| r.unstated()).collect::<Vec<_>>(),
            [400, 250],
            "profile-sorted (r208 before text), and `unstated` is amounts - excl - incl"
        );

        let text = render_text(&report);
        assert!(text.contains("== 6. Amount VAT basis"), "the section must exist:\n{text}");
        // 750 of 1,000 stated.
        assert!(text.contains("75.0%"), "the stated share:\n{text}");
        assert!(text.contains("0.0%"), "an era stating none reads 0 %, not blank:\n{text}");
        assert!(
            text.contains("parser history, not a procurement fact"),
            "the mix caveat must travel with the numbers:\n{text}"
        );

        // And a failed query is UNMEASURED rather than an era-less zero table.
        let mut failed = all();
        failed.iter_mut().find(|(l, _)| l == "amount_basis").expect("label").1 = None;
        let text = render_text(&assemble("(t)", &Raw::from_labelled(failed).expect("labelled")));
        assert!(text.contains("UNMEASURED — the `amount_basis` query did not run."), "{text}");
    }

    /// Issue 246: the arrivals section must say ARRIVALS, and must distinguish a
    /// bucket still being fed from settled residue — the whole reason it exists.
    #[test]
    fn quarantine_arrivals_are_rendered_as_a_rate_with_the_newest_arrival() {
        // Every catalog label present and empty, so only the section under test speaks.
        let all = || -> Vec<(String, Option<Rows>)> {
            queries().into_iter().map(|(l, _)| (l, Some(Vec::new()))).collect()
        };
        let mut ran = all();
        ran.iter_mut().find(|(l, _)| l == "fresh_holds").expect("label").1 = Some(vec![
            vec![json!("unrepresentable-value"), json!(31), json!(1_787_124_972u64)],
            vec![json!("not-utf8"), json!(4), json!(1_784_619_647u64)],
        ]);
        let raw = Raw::from_labelled(ran).expect("labelled");
        let text = render_text(&assemble("(t)", &raw));

        assert!(text.contains("== 5. Quarantine arrivals"), "the section must exist: {text}");
        assert!(text.contains("unrepresentable-value"), "{text}");
        // The newest arrival as a date, which is what separates live from settled.
        assert!(text.contains("2026-08-19"), "the live bucket's newest arrival: {text}");
        assert!(text.contains("2026-07-21"), "the settled bucket's newest arrival: {text}");
        // And the same section on the JSON surface, which it was missing entirely until
        // section 6 went into both (issue 251's note).
        let js = render_json(&assemble("(t)", &raw));
        assert!(js.contains("\"quarantine_arrivals\""), "the JSON must carry section 5: {js}");
        assert!(js.contains("\"newest_day\": \"2026-08-19\""), "with the day a person reads: {js}");
        // And the caveat that keeps this from being read as a bucket total.
        assert!(text.contains("Arrivals, not bucket size"), "{text}");

        // A query that did not run must not read as "nothing arrived" — the same
        // distinction issue 230 drew for every other label.
        let mut failed = all();
        failed.iter_mut().find(|(l, _)| l == "fresh_holds").expect("label").1 = None;
        let text = render_text(&assemble("(t)", &Raw::from_labelled(failed).expect("labelled")));
        assert!(text.contains("UNMEASURED — the `fresh_holds` query did not run"), "{text}");
        assert!(!text.contains("Arrivals, not bucket size"), "no caveat for an absent table: {text}");
    }

    /// Issue 230: every catalog query must be windowed, run whole-corpus, or
    /// explicitly named unmeasurable. A query in NONE of those lists is one the
    /// report silently does not measure — the failure mode that issue is about,
    /// reintroduced by a future addition rather than by a timeout.
    ///
    /// Three categories since issue 246 added the whole-corpus one, and the
    /// partition must stay a partition: a label in two lists would be measured twice
    /// per run, which for a quarantine-arrival count would silently double it.
    #[test]
    fn every_query_is_either_windowed_whole_corpus_or_declared_unmeasured() {
        let catalog: Vec<String> = queries().into_iter().map(|(l, _)| l).collect();
        let windowed: Vec<String> = windowed_queries().into_iter().map(|q| q.label).collect();
        let whole: Vec<String> = whole_corpus_queries().into_iter().map(|(l, _)| l).collect();
        for label in &whole {
            assert!(!windowed.contains(label), "{label} is both windowed and whole-corpus");
            assert!(!unwindowed_labels().contains(label), "{label} is both measured and declared unmeasured");
        }
        let mut covered: Vec<String> = windowed
            .iter()
            .cloned()
            .chain(whole.iter().cloned())
            .chain(unwindowed_labels())
            .collect();
        covered.sort();
        let mut expected = catalog.clone();
        expected.sort();
        assert_eq!(
            covered, expected,
            "windowed ∪ whole-corpus ∪ unmeasured must be exactly the catalog"
        );

        // And each template must actually carry the placeholder, exactly once. The
        // four late queries build theirs by splicing a predicate into the catalog
        // SQL, so a text that stopped matching would leave an UNWINDOWED statement
        // wearing a windowed label — the original unbounded pass, back again.
        for q in windowed_queries() {
            let sql = q.sql(10, 20);
            assert!(!sql.contains("{window}"), "{}: placeholder unfilled", q.label);
            assert_eq!(
                sql.matches("tender_id > 10 AND ").count(),
                1,
                "{}: exactly one window predicate: {sql}",
                q.label
            );
        }
    }

    /// Issue 230: the denominator must know every era's result section kind, and a
    /// numerator above it must be named rather than dashed. Both come from the same
    /// real defect — sdk-0.1's `TenderResult` sections were invisible to a query
    /// hardcoded to `LotResult`, so the first full-corpus run reported 0 award
    /// notices against 139,961 with results and rendered it as `—`.
    #[test]
    fn the_section_invariant_spans_both_result_kinds_and_a_contradiction_is_named() {
        assert!(
            SECTIONS_CAN_SQL.contains("s.kind IN ('LotResult', 'TenderResult')"),
            "sdk-0.1 names its results `TenderResult` (project.rs SDK01_RESULT_KIND): {SECTIONS_CAN_SQL}"
        );

        let raw = Raw::from_labelled(vec![
            ("versions".to_owned(), Some(vec![vec![json!("eforms:eforms-sdk-0.1"), json!(10)]])),
            ("title".to_owned(), Some(vec![])),
            ("buyer".to_owned(), Some(vec![])),
            ("value".to_owned(), Some(vec![])),
            ("cpv".to_owned(), Some(vec![])),
            ("deadline".to_owned(), Some(vec![])),
            ("winner".to_owned(), Some(vec![])),
            ("linkage".to_owned(), Some(vec![])),
            // A zero denominator under a large numerator — the shape prod actually
            // produced when the two halves were separate queries. Issue 243's merge
            // makes it unreachable from the database (both counts now come from the
            // same rows, so the numerator cannot exceed the denominator), and this
            // fixture keeps the RENDER honest anyway: a defensive path that stops
            // being exercised is a defensive path that quietly rots, and the next edit
            // to split these counts would need it again.
            (
                "awards".to_owned(),
                Some(vec![vec![
                    json!("eforms:eforms-sdk-0.1"),
                    json!(0),
                    json!(139_961),
                    json!(0),
                ]]),
            ),
            ("doc_types".to_owned(), Some(vec![])),
            ("sections_can".to_owned(), Some(vec![])),
            ("sections_with".to_owned(), Some(vec![])),
            ("merge".to_owned(), Some(vec![vec![json!("all"), json!(0), json!(0)]])),
            ("amount_basis".to_owned(), Some(vec![])),
            ("fresh_holds".to_owned(), Some(vec![])),
        ])
        .expect("labelled");
        let text = render_text(&assemble("http://x", &raw));
        assert!(text.contains("IMPOSSIBLE"), "a contradiction must be named:\n{text}");
        // And specifically NOT dashed, which reads as "nothing to report".
        let density_line = text
            .lines()
            .find(|l| l.contains("DÖE sdk-0.1 island") && l.contains("139,961"))
            .expect("the density row is rendered");
        assert!(!density_line.contains('—'), "a contradiction is not an absent rate: {density_line}");
    }

    /// Issue 235, THE regression: the award denominator must not be read from the
    /// projection's own output.
    ///
    /// The old denominator was "notices carrying a result SECTION", and the
    /// numerator was "notices carrying a `lot_results` ROW" — the projection
    /// writes the second from the first, so the pair could only ever measure
    /// whether the fold ran. It read exactly 100.0 % on all twenty eras that
    /// measure, including 60,037/60,037 on eforms-de-1.1 while issue 100's DE-1.x
    /// winners were known unresolved. An award notice that parsed with ZERO result
    /// sections — the actual failure — was excluded from both halves.
    ///
    /// So: the DENOMINATOR — the `WHERE` — must not mention `notice_sections`. Since
    /// issue 243 merged the three counts into one statement the barren `CASE` does
    /// read it, deliberately (that column exists to compare the published type against
    /// the parse), so the check is placed on the clause that decides the population
    /// rather than on the text of the whole query.
    #[test]
    fn the_award_denominator_never_reads_the_projections_own_output() {
        let sql = awards_sql();
        // The OUTER `WHERE`, found by the join text that precedes it — `find("WHERE")`
        // lands inside the first `CASE`'s subquery, and `rfind` inside the last one.
        let marker = "FROM tender_versions tv JOIN notices n ON n.id = tv.caused_by_notice_id WHERE ";
        let at = sql.find(marker).expect("the one pass over versions");
        let (projection, population) = (&sql[..at], &sql[at + marker.len()..]);
        assert!(
            !population.contains("notice_sections"),
            "the population must come from the notice's PUBLISHED type, not from what the \
             projection wrote: {population}"
        );
        assert!(
            population.contains("notice_codes"),
            "the doc type is read from notice_codes: {population}"
        );
        // The barren CASE is the only place `notice_sections` may appear, and it sits
        // in the projection list rather than in the population.
        assert!(
            projection.contains("notice_sections"),
            "the barren column compares the parse against the published type: {projection}"
        );

        // And the probe seeks by the primary-key prefix rather than scanning the
        // notice's codes: measured, this is 1.3 s vs an 11 s timeout on one window.
        for (label, sql) in [("awards", awards_sql()), ("doc_types", doc_type_sql())] {
            assert!(
                sql.contains("c.section_id = 'PROCEDURE'"),
                "{label} must pin the section so the probe is a (notice_id, section_id, field_id) \
                 prefix seek: {sql}"
            );
        }

        // One pass means the numerator is a subset of the denominator by construction:
        // both counts run over the same rows, so the rate cannot exceed 1 — the
        // `IMPOSSIBLE` state the old two-query pair could reach.
        assert!(awards_sql().contains(&award_predicate()), "the population IS the award predicate");
        assert!(awards_sql().contains("FROM lot_results lr"), "{}", awards_sql());
        assert_eq!(
            awards_sql().matches("FROM tender_versions tv").count(),
            1,
            "one pass over the versions, not three: {}",
            awards_sql()
        );
    }

    /// Issue 242: a density below 100 % has two very different causes, and the
    /// report must not let a reader mistake one for the other.
    ///
    /// Both eras below read 40.0 %. In the first, every unmaterialised notice
    /// published no award block at all (an `OTH_NOT` prose body, an empty F06
    /// container) — nothing to project, our shortfall is zero. In the second, all
    /// six published a result block and we did not project it. Same rate, opposite
    /// meaning.
    #[test]
    fn the_barren_column_separates_a_publication_gap_from_a_projection_gap() {
        // `barren` is the third count per era, in the merged row: 10 award notices, 4
        // materialised, and N of the 6 remaining explained by publishing nothing.
        let labels = |r208_barren: u64, eforms_barren: u64| -> Vec<(String, Option<Rows>)> {
            let mut out: Vec<(String, Option<Rows>)> =
                queries().into_iter().map(|(l, _)| (l, Some(Vec::new()))).collect();
            let slot = out.iter_mut().find(|(l, _)| l == "awards").expect("label");
            slot.1 = Some(vec![
                vec![json!("ted-export-r208"), json!(10), json!(4), json!(r208_barren)],
                vec![json!("eforms:eforms-sdk-1.13"), json!(10), json!(4), json!(eforms_barren)],
            ]);
            out
        };

        let raw = Raw::from_labelled(labels(6, 0)).expect("labelled");
        let report = assemble("http://x", &raw);
        let r208 = report.density.iter().find(|r| r.profile == "ted-export-r208").expect("r208");
        assert_eq!((r208.award_notices, r208.with_results, r208.no_award_content), (10, 4, 6));

        let text = render_text(&report);
        // The rate is still reported honestly …
        assert!(text.contains("40.0%"), "{text}");
        // … and so is the split: 6 published nothing, 6 are ours (the sdk-1.13 era).
        assert!(
            text.contains("6 award notice(s) announce a result and have no result block in the parse"),
            "the unparsed-block count must be named: {text}"
        );
        assert!(
            text.contains("the fold's own shortfall: 6"),
            "and so must the part that is the fold's: {text}"
        );
        // Issue 244: and the line must NOT claim a cause it cannot know. The text
        // era publishes its awards under `CO:` and is counted in this column too.
        assert!(
            !text.contains("Nothing can project those"),
            "the column measures the parse, not the publisher: {text}"
        );

        // JSON carries both per era, so a consumer never has to subtract.
        let json: Value = serde_json::from_str(&render_json(&report)).expect("valid json");
        let rows = json["results_density"].as_array().expect("rows");
        let ted = rows.iter().find(|r| r["profile"] == "ted-export-r208").expect("r208 row");
        assert_eq!((ted["no_award_content"].as_u64(), ted["unprojected"].as_u64()), (Some(6), Some(0)));
        let eforms =
            rows.iter().find(|r| r["profile"] == "eforms:eforms-sdk-1.13").expect("eforms row");
        assert_eq!((eforms["no_award_content"].as_u64(), eforms["unprojected"].as_u64()), (Some(0), Some(6)));

        // And a barren count that did not run must not silently read as zero —
        // that would turn "unknown" into "all of it is our fault" (issue 230).
        let mut failed = labels(0, 0);
        failed.iter_mut().find(|(l, _)| l == "awards").expect("label").1 = None;
        let raw = Raw::from_labelled(failed).expect("labelled");
        let text = render_text(&assemble("http://x", &raw));
        assert!(text.contains("no result block parsed: UNMEASURED"), "{text}");
        assert!(!text.contains("the fold's own shortfall"), "no arithmetic on a missing input: {text}");
    }

    /// The vocabulary itself: every marker must classify codes into two disjoint
    /// lists, because a code in both makes "unclassified" meaningless and a code in
    /// neither is the visible gap [`doc_type_sql`] exists to count.
    #[test]
    fn every_doc_type_marker_lists_disjoint_award_and_other_codes() {
        for m in DOC_TYPE_MARKERS {
            assert!(!m.profiles.is_empty(), "{}: a marker with no profile scope reads no era", m.field_id);
            assert!(!m.award.is_empty(), "{}: a marker naming no award code cannot feed a denominator", m.field_id);
            for code in m.award {
                assert!(
                    !m.other.contains(code),
                    "{}: `{code}` is in both lists, so it is both an award and not one",
                    m.field_id
                );
            }
            let mut seen: Vec<&str> = m.award.iter().chain(m.other).copied().collect();
            let before = seen.len();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(before, seen.len(), "{}: a code is listed twice", m.field_id);
        }
        // Every era family the report labels must have a marker, or section 3
        // silently reports no award notices for a whole era. `other` is the
        // catch-all `era_of` bucket and has no corpus behind it.
        for profile in [
            "text",
            "internal-ojs",
            "ted-export-r208",
            "ted-export-r209",
            "eforms:eforms-sdk-0.1",
            "eforms:eforms-sdk-1.13",
            "eforms:eforms-de-1.1",
            "eforms:eforms-de-2.1@eforms-sdk-1.13",
        ] {
            assert!(
                DOC_TYPE_MARKERS.iter().any(|m| m.profiles.iter().any(|p| like_matches(p, profile))),
                "no document-type marker covers {profile}, so its awards cannot be counted"
            );
        }
    }

    /// A minimal SQL `LIKE` for the test above: `%` only, which is all
    /// [`DocTypeMarker::profiles`] uses.
    fn like_matches(pattern: &str, value: &str) -> bool {
        match pattern.strip_suffix('%') {
            Some(prefix) => value.starts_with(prefix),
            None => pattern == value,
        }
    }

    /// The unclassified diagnostic must actually be able to fire: it asks for a
    /// marker field whose code is in NEITHER list, per era. A measured example is
    /// `OPP-070-notice = 'X02'`, which appeared in sdk-1.1x and is in neither list
    /// on purpose — the vocabulary should report it, not absorb it.
    #[test]
    fn doc_type_coverage_counts_a_code_in_neither_list() {
        let sql = doc_type_sql();
        assert!(sql.contains("AS unclassified") && sql.contains("AS untyped"), "{sql}");
        assert!(sql.contains("c.code NOT IN ("), "an unclassified code is one outside the known set: {sql}");
        for m in DOC_TYPE_MARKERS {
            assert!(sql.contains(m.field_id), "{} must be probed: {sql}", m.field_id);
        }
        // A code we deliberately do NOT classify must not appear in the known set.
        assert!(!sql.contains("'X02'"), "X02 is unclassified by design, so the diagnostic can show it");
        // The eForms scheme's boundary, as measured: 24 is never a result notice,
        // 25 (direct-award prenotification) always is.
        assert!(EFORMS_OTHER_SUBTYPES.contains(&"24"), "24 is a competition subtype");
        assert!(EFORMS_AWARD_SUBTYPES.contains(&"25"), "25 announces a direct award");
    }

    /// The per-era scope is what keeps one field id from carrying another era's
    /// vocabulary: `TED-FORM` exists in r2.0.8 and r2.0.9 too, with different
    /// values, and those eras are classified by `TD_DOCUMENT_TYPE` instead.
    #[test]
    fn the_2008_export_is_read_through_the_shared_ted_vocabulary() {
        let ojs = DOC_TYPE_MARKERS
            .iter()
            .find(|m| m.profiles == ["internal-ojs"])
            .expect("the 2008 export is classified");
        // Not by its form: `TED-FORM` is missing from 12 % of the era, and its award
        // forms are 3/3_SUM AND 6/6_SUM (utilities), which the form list missed.
        // `NAT_NOTICE` is on 26,955 of 26,955 and files both under `7`.
        assert_eq!(ojs.field_id, "TED-NAT_NOTICE");
        assert!(std::ptr::eq(ojs.award, TED_TD_AWARD), "the era shares the TD award list");
        assert!(std::ptr::eq(ojs.other, TED_TD_OTHER), "and the TD non-award list");
        // Sharing the list is the point: no era-local vocabulary to drift.
        let r20x = DOC_TYPE_MARKERS
            .iter()
            .find(|m| m.field_id == "TED-TD_DOCUMENT_TYPE")
            .expect("the r2.0.x marker");
        assert_eq!((r20x.award, r20x.other), (ojs.award, ojs.other));
        // And the marker is still profile-scoped at version level: `TED-NAT_NOTICE`
        // exists in r2.0.x too, where TD_DOCUMENT_TYPE is the authority.
        let sql = awards_sql();
        assert!(
            sql.contains("(n.profile LIKE 'internal-ojs') AND EXISTS("),
            "the marker must be profile-scoped, at version level: {sql}"
        );
        assert!(sql.contains("AND c.field_id = 'TED-NAT_NOTICE' AND c.code IN ('7',"), "{sql}");
    }

    #[test]
    fn as_u64_tolerates_int_float_and_null() {
        assert_eq!(as_u64(Some(&json!(42))), 42);
        assert_eq!(as_u64(Some(&json!(42.9))), 42);
        assert_eq!(as_u64(Some(&json!(-3))), 0);
        assert_eq!(as_u64(Some(&Value::Null)), 0);
        assert_eq!(as_u64(None), 0);
    }

    #[test]
    fn pct_and_rate_guard_a_zero_denominator() {
        assert_eq!(pct(1, 4), "25.0%");
        assert_eq!(pct(0, 0), "—");
        assert_eq!(rate(1, 4), Some(0.25));
        assert_eq!(rate(1, 0), None);
    }

    #[test]
    fn group_inserts_thousands_separators() {
        assert_eq!(group(0), "0");
        assert_eq!(group(999), "999");
        assert_eq!(group(1_000), "1,000");
        assert_eq!(group(3_512_034), "3,512,034");
    }

    /// The whole assembly, from result matrices to a rendered report — the shape
    /// the bin and the tests both drive.
    #[test]
    fn assembles_a_report_from_labelled_rows() {
        let results = vec![
            ("versions".to_owned(), Some(vec![
                vec![json!("eforms:eforms-sdk-1.13"), json!(10)],
                vec![json!("ted-export-r209"), json!(4)],
            ])),
            ("title".to_owned(), Some(vec![vec![json!("eforms:eforms-sdk-1.13"), json!(10)], vec![json!("ted-export-r209"), json!(4)]])),
            ("buyer".to_owned(), Some(vec![vec![json!("eforms:eforms-sdk-1.13"), json!(9)]])),
            ("value".to_owned(), Some(vec![vec![json!("eforms:eforms-sdk-1.13"), json!(7)]])),
            ("cpv".to_owned(), Some(vec![vec![json!("eforms:eforms-sdk-1.13"), json!(10)]])),
            ("deadline".to_owned(), Some(vec![vec![json!("eforms:eforms-sdk-1.13"), json!(6)]])),
            ("winner".to_owned(), Some(vec![vec![json!("eforms:eforms-sdk-1.13"), json!(2)]])),
            ("linkage".to_owned(), Some(vec![vec![json!("ted-export-r209"), json!(2), json!(1)]])),
            // Density: 2 award-TYPED notices for the eForms era, 0 materialised, and
            // both published a result block — so the 0 % is entirely the projection's
            // own gap and nothing is explained away.
            (
                "awards".to_owned(),
                Some(vec![vec![json!("eforms:eforms-sdk-1.13"), json!(2), json!(0), json!(0)]]),
            ),
            // One version of the r209 era carries a type this vocabulary cannot read.
            ("doc_types".to_owned(), Some(vec![vec![json!("ted-export-r209"), json!(1), json!(0)]])),
            // The section→row invariant: 3 parsed a result section, all 3 written.
            ("sections_can".to_owned(), Some(vec![vec![json!("ted-export-r209"), json!(3)]])),
            ("sections_with".to_owned(), Some(vec![vec![json!("ted-export-r209"), json!(3)]])),
            // Column 0 is merge's constant scope label (issue 230) — the shape that
            // lets a single-row result sum across windows like every other one.
            ("merge".to_owned(), Some(vec![vec![json!("all"), json!(3), json!(1)]])),
            // Issue 251: three amounts, one of each basis and one the source left
            // unstated — so the derived third answer is exercised rather than assumed.
            (
                "amount_basis".to_owned(),
                Some(vec![vec![json!("ted-export-r209"), json!(3), json!(1), json!(1)]]),
            ),
            (
                "fresh_holds".to_owned(),
                Some(vec![
                    vec![json!("unrepresentable-value"), json!(31), json!(1_787_124_972u64)],
                    vec![json!("not-utf8"), json!(4), json!(1_784_619_647u64)],
                ]),
            ),
        ];
        let raw = Raw::from_labelled(results).expect("labelled");
        let report = assemble("http://x", &raw);

        // Two eras, sorted by profile.
        assert_eq!(report.completeness.len(), 2);
        let eforms = &report.completeness[0];
        assert_eq!(eforms.profile, "eforms:eforms-sdk-1.13");
        assert_eq!(eforms.versions, 10);
        // title present on all 10, winner on 2, r209 has no buyer row ⇒ 0.
        assert_eq!(eforms.present, [10, 9, 7, 10, 6, 2]);
        assert_eq!(report.completeness[1].present, [4, 0, 0, 0, 0, 0]);

        assert_eq!(report.linkage[0].awards, 2);
        assert_eq!(report.linkage[0].unchained, 1);
        // The issue-15 anomaly in miniature: 2 award notices, 0 materialised.
        assert_eq!(report.density[0].award_notices, 2);
        assert_eq!(report.density[0].with_results, 0);
        // The invariant is its own row set now (issue 235), and it reads 3/3 —
        // "the projection wrote what it parsed" is TRUE on the same era where the
        // award density is 0 %, which is exactly the pair of facts one metric
        // could not express.
        assert_eq!(report.invariant[0].with_sections, 3);
        assert_eq!(report.invariant[0].with_rows, 3);
        assert_eq!(report.doc_types[0].unclassified, 1);
        assert_eq!(report.merge, Merge { doe_tenders: 3, merged: 1 });

        // Rendered text carries the headline rates.
        let text = render_text(&report);
        assert!(text.contains("Field completeness"));
        // The text label distinguishes the eForms SDK version (JSON keeps the
        // coarse era separately).
        assert!(text.contains("eforms-sdk-1.13"));
        assert!(text.contains("merged with TED: 1 (33.3%)"));

        // JSON round-trips to the documented shape.
        let json: Value = serde_json::from_str(&render_json(&report)).expect("valid json");
        assert_eq!(json["ted_doe_merge"]["merged"], json!(1));
        assert_eq!(json["results_density"][0]["density"], json!(0.0));
        assert_eq!(json["sections_to_rows"][0]["written_rate"], json!(1.0));
        assert_eq!(json["doc_type_coverage"][0]["unclassified"], json!(1));
        assert_eq!(json["completeness"][0]["rate"]["winner"], json!(0.2));
        // And the two questions render as separate sections, so a reader cannot
        // mistake one for the other.
        assert!(text.contains("PUBLISHED type announces a result"), "{text}");
        assert!(text.contains("3b. Projection invariant"), "{text}");
        assert!(text.contains("CANNOT classify"), "{text}");
    }

    /// Issue 230: a query that FAILED and a query that ran and matched nothing
    /// are different claims, and the rendered report must not merge them. This is
    /// the exact shape prod produced — all 11 queries 408'd — where section 4
    /// printed "DÖE procedure Tenders: 0; merged with TED: 0 (—)" for numbers
    /// nobody had measured.
    #[test]
    fn a_failed_query_renders_as_unmeasured_not_as_zero() {
        let labels: Vec<String> = queries().into_iter().map(|(l, _)| l).collect();
        let total = labels.len();
        // Every query failed: None, not an empty result set.
        let all_failed: Vec<(String, Option<Rows>)> =
            labels.iter().map(|l| (l.clone(), None)).collect();
        let raw = Raw::from_labelled(all_failed).expect("labelled");
        assert_eq!(raw.unmeasured.len(), total, "every label recorded as unmeasured");
        let text = render_text(&assemble("http://x", &raw));
        assert!(text.contains(&format!("INCOMPLETE: {total} of {total} queries did not run")), "{text}");
        assert!(text.contains("UNMEASURED — the `merge` query did not run."), "{text}");
        assert!(
            !text.contains("merged with TED: 0"),
            "a number nobody measured must not be printed: {text}"
        );

        // The complement: a query that RAN and matched nothing still reports its
        // real zero, and the report is not marked incomplete.
        let mut ran: Vec<(String, Option<Rows>)> =
            labels.iter().map(|l| (l.clone(), Some(Vec::new()))).collect();
        ran.retain(|(l, _)| l != "merge");
        ran.push(("merge".to_owned(), Some(vec![vec![json!("all"), json!(0), json!(0)]])));
        ran.push(("fresh_holds".to_owned(), Some(vec![])));
        let raw = Raw::from_labelled(ran).expect("labelled");
        assert!(raw.unmeasured.is_empty());
        let text = render_text(&assemble("http://x", &raw));
        assert!(!text.contains("INCOMPLETE"), "{text}");
        assert!(text.contains("merged with TED: 0"), "a measured zero is still reported: {text}");
    }

    #[test]
    fn from_labelled_reports_a_missing_result_set() {
        let err = Raw::from_labelled(vec![("versions".to_owned(), Some(vec![]))]).unwrap_err();
        assert!(err.contains("title"), "{err}");
    }
}
