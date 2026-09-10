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
                         THEN 1 ELSE 0 END) AS no_award_content, \
                SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_result_winners w \
                                      WHERE w.tender_id = tv.tender_id AND w.seq = tv.seq) \
                         THEN 1 ELSE 0 END) AS with_winner, \
                SUM(CASE WHEN EXISTS(SELECT 1 FROM tender_version_lot_results r \
                                      WHERE r.tender_id = tv.tender_id AND r.seq = tv.seq \
                                        AND NOT (r.decision IN ('no-rece', 'clos-nw', 'open-nw') \
                                                 AND NOT EXISTS(SELECT 1 FROM tender_version_result_winners w \
                                                                 WHERE w.tender_id = r.tender_id \
                                                                   AND w.seq = r.seq))) \
                         THEN 1 ELSE 0 END) AS with_awardable \
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

/// The content-presence probe (issue 109): versions with NO rows in ANY
/// version-keyed satellite — a shell has a `tender_versions` row and nothing
/// else, which is why every row-counting gate stayed green through the 218,635
/// factless eForms-DE 1.x versions of issue 85. Cause-agnostic by design: it
/// fires on an epoch skip, a narrowed mapping, a bad alias table, a vendored
/// inventory regression, and on causes nobody has thought of yet.
///
/// The `NOT EXISTS` chain is ordered by fill rate (texts first: title is
/// ~96–100 % in every era), so for a normal version the FIRST probe finds a row
/// and the whole predicate short-circuits false — the per-version cost is one
/// indexed seek, the same as a single field probe. Only genuinely sparse
/// versions probe deeper.
fn factless_template(scope: &str) -> String {
    let absent = [
        "tender_version_texts",
        "tender_version_classifications",
        "tender_version_dates",
        "tender_version_parties",
        "tender_version_amounts",
        "tender_version_lots",
    ]
    .map(|t| format!("NOT EXISTS (SELECT 1 FROM {t} s WHERE s.tender_id = v.tender_id AND s.seq = v.seq)"))
    .join(" AND ");
    format!(
        "SELECT n.profile, COUNT(*) AS factless \
           FROM tender_versions v JOIN notices n ON n.id = v.caused_by_notice_id \
          WHERE {scope}{absent} \
          GROUP BY n.profile"
    )
}

/// The amount-plausibility probe (issue 267): the standing form of the
/// negative-amount analyses (131/132/134/136), which established that negatives
/// are overwhelmingly SOURCE-published and then stopped measuring. The honest
/// invariant is the per-era RATE (issue 134's form) — a parser regression that
/// starts fabricating negatives, or a source shipping garbage at scale, moves
/// the rate; individual negatives do not. `over_1e12` is a deliberately crude
/// tripwire for the unrepresentable-value class escaping quarantine into the
/// layer, not a correctness claim about any single amount.
fn amount_plausibility_template(scope: &str) -> String {
    format!(
        "SELECT n.profile, COUNT(*) AS amounts, \
                SUM(CASE WHEN a.cents < 0 THEN 1 ELSE 0 END) AS negative, \
                SUM(CASE WHEN a.cents = 0 THEN 1 ELSE 0 END) AS zero, \
                SUM(CASE WHEN a.cents > 100000000000000 THEN 1 ELSE 0 END) AS over_1e12, \
                SUM(CASE WHEN a.eur_cents IS NOT NULL THEN 1 ELSE 0 END) AS convertible \
           FROM tender_versions v JOIN notices n ON n.id = v.caused_by_notice_id \
           JOIN tender_version_amounts a ON a.tender_id = v.tender_id AND a.seq = v.seq \
          {scope}GROUP BY n.profile"
    )
}

/// Amounts at or above this many cents — 1,000,000,000 major units — are the region
/// [`sentinel_amounts_sql`] looks in. The threshold is on the PUBLISHED figure rather
/// than the EUR conversion, and that is the whole point: `tenders.current_value_eur_cents`
/// is converted, so a PLN or HUF sentinel is smeared into a non-round EUR figure and
/// cannot cluster there at all. Measured 2026-09-08 — a frequency sweep of the converted
/// column returned nothing but genuine round budgets (€2 M ×10,114, €1.2 M ×8,908,
/// €1.5 M ×7,996). The instrument was the reason it found nothing, not the corpus.
const SENTINEL_AMOUNT_FLOOR: i64 = 100_000_000_000;

/// How many satellite rows a value must repeat on to be listed. A sentinel's signature
/// is REPETITION: one implausible figure is a typo in one notice, which section 8's
/// per-era rates already count. Filtered in SQL rather than rendered and skipped, so
/// the listing cap is spent on candidates instead of on noise.
const SENTINEL_MIN_REPEATS: u64 = 10;

/// The date sweep's own repeat threshold, far lower than [`SENTINEL_MIN_REPEATS`] because
/// the populations differ by three orders of magnitude: negatives alone are ~15,650 amount
/// rows, while the ENTIRE far-future deadline tail is on the order of a hundred tenders
/// (measured 2026-09-08 over the indexed `tenders.current_deadline`, both tails). At 10 the
/// date listing would report a clean corpus while 2040-12-31, 2038-12-01 and 2038-07-01 sat
/// in it. A missed sentinel costs more here than an extra candidate row, which the cap and
/// the ordering already bound.
const SENTINEL_DATE_MIN_REPEATS: u64 = 3;

/// At most this many values per sentinel listing. Both queries rank by repetition, so
/// the cap keeps the values worth a rule — and a full listing SAYS it is full (see
/// [`render_sentinels`]) rather than presenting a truncated tail as the whole of it.
const SENTINEL_LISTING_CAP: usize = 40;

/// The far edge of the plausible date range: ten years past the run. Mirrors
/// `store::canonical::DEADLINE_HORIZON_SECS`, which bounds the head-deadline election,
/// so the detector looks exactly where the election now refuses. Relative to the run
/// rather than to each notice's `published_at` (which is what the election uses) —
/// coarser, and enough for a detector whose job is to rank candidates.
const SENTINEL_DATE_HORIZON_SECS: i64 = 10 * 365 * 86_400;

/// The near edge: 1990-01-01. TED's own record starts in the 1990s, so an earlier date
/// is a placeholder, an epoch-zero default or a century typo rather than a procurement
/// date.
const SENTINEL_DATE_FLOOR: i64 = 631_152_000;

/// Published amounts that REPEAT inside the implausible tail (issue 366).
///
/// The standing sentinel list — negatives, all-nines maxima, figures past €100 bn — was
/// derived by CONFIRMING four guesses, so by construction it holds only shapes somebody
/// already imagined. This is the discovery instrument instead: group the tail by its
/// published `(currency, cents)` and rank by how often each value occurs. A value
/// carried by thousands of rows at a magnitude no procurement reaches is a sentinel
/// whether or not anyone predicted its shape.
///
/// **Why the tail rather than the whole column.** The hash state of `GROUP BY currency,
/// cents` is one entry per DISTINCT value, and over the full corpus that is millions of
/// them — the state blow-up issue 278 met, and issue 337's leaked temp database. The
/// floor bounds the state to the few thousand distinct values above it. The cost of the
/// bound is stated rather than hidden: this cannot see a LOW-magnitude sentinel. In that
/// region a frequency ranking is dominated by genuine round budgets anyway (measured —
/// see [`SENTINEL_AMOUNT_FLOOR`]), so frequency alone would not identify one there; it
/// would need a different discriminator, which is left as the open half of issue 366.
///
/// **The floor is applied to `eur_cents`; the GROUPING stays on published `(currency, cents)`.**
/// Measured on the first corpus run (job 816, 2026-09-08): a currency-blind floor of 1,000,000,000
/// major units is ~€40 M in CZK and ~€2.5 M in HUF, so the top-40 filled with entirely ordinary
/// Czech, Hungarian and Swedish contracts (CZK 2,300,000,000 ×1,960 rows, SEK 1,200,000,000 ×1,110,
/// HUF 1,000,000,000 ×383) while the one genuine sentinel in the listing — PLN 22,222,222,222 — sat
/// at rank ~35. The cap was being spent on noise.
///
/// Converting the THRESHOLD reintroduces none of the blind spot that made this query necessary: the
/// blind spot is about CLUSTERING (a converted value is smeared off its round published figure and
/// cannot group), and the grouping key is still the published pair. The threshold only decides scope.
///
/// `eur_cents IS NULL` is the honest case rather than an edge case — ADR-0014 D4 leaves it NULL when
/// no rate resolves instead of guessing — so an unconverted row falls back to the raw floor. Without
/// that arm every era awaiting its conversion backfill would silently leave the sweep's scope, which
/// is the same class of quiet narrowing as the exact-instant date grouping.
///
/// Negatives are in scope at EVERY magnitude: the class is ~15,650 rows corpus-wide, small
/// enough to group without a floor, and `-1.00` alone accounts for 15,529 of them.
///
/// Whole-corpus by necessity rather than by nature. The population is window-sliceable
/// (an amount row belongs to exactly one version), but `HAVING COUNT(*) >= n` is NOT: a
/// value repeating nine times in each of 25 windows passes the corpus test and fails
/// every window's. A windowed form would silently under-report precisely the values it
/// exists to find, which is why this sits here and not in [`windowed_queries`].
pub fn sentinel_amounts_sql() -> String {
    format!(
        "SELECT a.currency AS currency, a.cents AS cents, COUNT(*) AS hits, \
                COUNT(DISTINCT a.tender_id) AS tenders \
           FROM tender_version_amounts a \
          WHERE a.cents < 0 \
             OR a.eur_cents >= {SENTINEL_AMOUNT_FLOOR} \
             OR (a.eur_cents IS NULL AND a.cents >= {SENTINEL_AMOUNT_FLOOR}) \
          GROUP BY a.currency, a.cents \
         HAVING COUNT(*) >= {SENTINEL_MIN_REPEATS} \
          ORDER BY hits DESC \
          LIMIT {SENTINEL_LISTING_CAP}"
    )
}

/// A tender is flagged as a WELD CANDIDATE at this many distinct buyer
/// organizations (issue 364 unit 3).
///
/// Three, and the floor is measured rather than chosen: two is the organization
/// layer's own duplication noise (82803 carries "Stadt Osnabrück - FD Öffentliche
/// Aufträge" beside "… - Fachdienst Öffentliche Aufträge"; 82806 carries the same
/// Rostock company as two org rows), so an alarm at two would fire on the org
/// layer rather than on the grouping.
///
/// **Three is an UPPER bound on welds, not a count of them** — joint procurement
/// is legal and real. Measured corpus-wide 2026-09-10: 102,840 tenders at ≥3,
/// 32,497 at ≥5, 13,297 at ≥10, **1,326 at ≥50**. Two instruments agree on those
/// four numbers exactly — a windowed `/v1/sql` census and this whole-corpus query —
/// which is worth more than either alone.
///
/// **NO THRESHOLD SEPARATES A WELD FROM A JOINT PROCUREMENT, and the first run of
/// this section proved it.** The sentence that stood here said the listing opens at
/// ≥50 "because no joint procurement has fifty buyers". That is false. Tender 331647
/// carries **505 buyers on ONE version** and is titled `Skupno javno naročilo` —
/// Slovenian for joint public procurement — and several more of the top 40 are the
/// same country's joint fuel-purchasing notices. The claim was an assertion dressed
/// as a calibration, it shipped, and the first data it met refuted it.
///
/// What separates them is not the count but the SHAPE: a joint procurement names all
/// its buyers in one notice, a weld accumulates them across versions. The report
/// renders buyers-per-version as a hint (331647 scores 505.0; tender 2816628, the weld
/// issue 364 was filed about, scores 0.04 over 2,983 versions). The honest test is the
/// widest SINGLE version, which needs a second grouped pass nobody has paid for.
pub const WELD_MIN_BUYERS: i64 = 3;

/// Bands the summary reports, so one number cannot hide the shape.
pub const WELD_BANDS: [i64; 4] = [3, 5, 10, 50];

/// How many candidates the listing names, worst first.
pub const WELD_LISTING_CAP: usize = 40;

/// Tenders carrying many DISTINCT buyer organizations — the detector for issue
/// 364's legacy-OJS weld and for `procedure-key-accepted-unchecked` (369), built
/// once because both defects present identically: unrelated procurements fused
/// into one Tender.
///
/// **Both buyer roles, and that is the whole trick.** The corpus carries two
/// vocabularies — `Procedure-Buyer` from eForms and `buyer` from the legacy
/// r208/r209 era. Issue 364's calibration recorded `Procedure-Buyer` only,
/// derived from an eForms-only sample, and following it would have made this
/// blind to the legacy era: tender 2816628, the 2,983-version 127-buyer weld the
/// issue was FILED about, has 2,983 `buyer` rows and zero `Procedure-Buyer`. The
/// gauge would have reported nought for it and read green.
///
/// Counted across ALL versions rather than the head, because a weld shows along
/// the version chain — 2816628 accumulated its buyers over 2,983 of them.
///
/// `longest_chain` cannot substitute: it reads 212 tenders at ≥200 versions while
/// 1,326 carry ≥50 distinct buyers, and a two-version Tender welding two
/// unrelated procurements is invisible to a chain-length gauge by construction.
///
/// Whole-corpus rather than windowed for the same reason as the sentinel sweeps:
/// the `HAVING` is per tender, and although a tender never straddles a window (so
/// this one COULD be windowed correctly), the top-N ordering could not be merged
/// without keeping every window's tail. Registered beside them and paid for out
/// of the same phase.
///
/// **MEASURED 2026-09-10 (job 1912): the pair costs ~518 s (8.6 min).** The
/// whole-corpus phase went from ~128 s without them (job 816) to 646 s with them,
/// and the difference is these two.
///
/// The number is worth keeping WITH its prior, because the prior is what makes it
/// interpretable. Two plans were possible. The `ORDER BY` sorts only what survives
/// the `HAVING` (102,840 rows), not the corpus, so the top-N is cheap and the GROUP
/// BY is the whole cost; windowed at 100,000 `tender_id`s this shape answered inside
/// `/v1/sql`'s 10 s cap riding `tender_version_parties(tender_id, seq)` as a range
/// scan, which extrapolated to ~15 min for one pass. But **no index carries
/// `role`**, so turso could instead have taken a full table scan with a hash group
/// over ~8M keys — a different cost with a memory profile behind it, and one that
/// would have shown as far above ~30 min for the two together.
///
/// It came in under the range-scan estimate, so **it took the indexed plan and the
/// `(role, tender_id)` index this would otherwise need is not warranted.** Re-check
/// that if the figure ever jumps: the planner's choice is what the number is really
/// reporting.
pub fn weld_candidates_sql() -> String {
    format!(
        "SELECT p.tender_id AS tender_id, \
                COUNT(DISTINCT p.organization_id) AS buyers, \
                COUNT(DISTINCT p.seq) AS versions \
           FROM tender_version_parties p \
          WHERE p.role IN ('buyer', 'Procedure-Buyer') \
          GROUP BY p.tender_id \
         HAVING COUNT(DISTINCT p.organization_id) >= {WELD_MIN_BUYERS} \
          ORDER BY buyers DESC \
          LIMIT {WELD_LISTING_CAP}"
    )
}

/// The band counts behind [`weld_candidates_sql`]'s listing, so the listing's cap
/// cannot be mistaken for the population's size.
pub fn weld_bands_sql() -> String {
    let cases: Vec<String> = WELD_BANDS
        .iter()
        .map(|n| format!("SUM(CASE WHEN buyers >= {n} THEN 1 ELSE 0 END)"))
        .collect();
    format!(
        "SELECT {} FROM (SELECT COUNT(DISTINCT p.organization_id) AS buyers \
           FROM tender_version_parties p \
          WHERE p.role IN ('buyer', 'Procedure-Buyer') \
          GROUP BY p.tender_id \
         HAVING COUNT(DISTINCT p.organization_id) >= {WELD_MIN_BUYERS})",
        cases.join(", ")
    )
}

/// Published dates that REPEAT outside the plausible range (issue 366) — the same
/// instrument as [`sentinel_amounts_sql`], pointed at the other column family whose
/// head election takes a `.max()`.
///
/// This half was never swept at all. `3005-07-06` reached issue 366 by being handed over
/// in a bug report, not by being found, and a `2099-12-31` or `9999-12-31` cluster is the
/// same defect class with nothing looking for it. The floor catches the other tail, where
/// an epoch-zero default (`1970-01-01`) or a century typo lands.
///
/// **Grouped by DAY, not by instant** — and that distinction is the whole difference
/// between a working detector and a silent one. Measured 2026-09-08 against the indexed
/// `tenders.current_deadline`: the strongest cluster in the corpus is 2099-12-31 with 14
/// tenders, but they are spread over several times of day (00:00, 10:00, 11:59, 23:59…)
/// and the largest single SECOND holds only 4. An exact-instant grouping with any useful
/// threshold returns nothing from the very cluster it exists to find. A date sentinel is a
/// DAY that a system emits; the time of day is whatever the source's formatter appended.
///
/// The two shapes that grouping revealed, neither of which anyone had guessed:
/// **far-year 31 December** (2099, 2040, 2038, 2050, 2036, 2999 — a "no real deadline"
/// convention) and a **2037–2038 concentration**, which is the 32-bit epoch ceiling:
/// 2^31 seconds lands on 2038-01-19, so a system capping at its maximum representable
/// date emits late 2037 and 2038.
///
/// `MIN(utc_seconds)` is the group's representative instant. Every member shares the
/// group's `date(…)`, so the minimum renders to exactly that day — correct by
/// construction, and it keeps the row shape an `i64` like the amount half.
///
/// `strftime('%s','now')` returns TEXT, and SQLite orders every number below every
/// string — so the arithmetic is load-bearing, not cosmetic: `+ SECS` forces numeric
/// affinity and makes the comparison mean what it reads as. Without it the predicate is
/// silently always-false. [`fresh_holds_sql`] is correct for the same reason.
pub fn sentinel_dates_sql() -> String {
    format!(
        "SELECT d.field AS field, MIN(d.utc_seconds) AS instant, COUNT(*) AS hits, \
                COUNT(DISTINCT d.tender_id) AS tenders \
           FROM tender_version_dates d \
          WHERE d.utc_seconds < {SENTINEL_DATE_FLOOR} \
             OR d.utc_seconds > strftime('%s','now') + {SENTINEL_DATE_HORIZON_SECS} \
          GROUP BY d.field, date(d.utc_seconds, 'unixepoch') \
         HAVING COUNT(*) >= {SENTINEL_DATE_MIN_REPEATS} \
          ORDER BY hits DESC \
          LIMIT {SENTINEL_LISTING_CAP}"
    )
}

/// The `-1.00` amount rows, and how many of them sit in a notice that DECLARED a
/// withholding (issue 372).
///
/// Issue 366's sweep found `-1.00` recurring across EUR, PLN, DKK and NOK beside a currency
/// literally spelled `unpublished`, and a sampled census traced it: under BT-195/`FieldsPrivacy`
/// the eForms SDK writes the code `unpublished` and the number **−1** when a buyer withholds a
/// field. So `-1.00` is not a publisher convention — it is the SDK's withheld-value marker, and
/// the notice says which field, why, and when it becomes publishable.
///
/// **The number this exists to produce is the RESIDUE**, `hits - in_withholding_notice`: a
/// declared withholding can be marked precisely, whereas an UNDECLARED `-1` is a guess about
/// somebody's intent and may deserve to stay quarantined instead. Issue 372 unit 2's disposition
/// decision turns on which of those dominates, and 4-of-4 on a hand sample is not an answer.
///
/// **Deliberately weaker than the exact test, and the gap is stated.** This asks whether the
/// notice declared ANY withholding, not whether it declared THIS field's. The exact form needs the
/// `AMOUNTS` source→canonical mapping (`BT-161` → `result_value`, `project.rs:135`), which lives in
/// Rust rather than in SQL — and unit 2 has it to hand at the point of the fix. Reading this as
/// "declared" would overstate it; read it as "the notice was withholding something".
///
/// **Covers BOTH satellites (issue 372's scope correction, 2026-09-08).** The two withheld codes the
/// notices actually declare do not share a destination: `BT-195(BT-161)` (`not-val`) reaches
/// `result_value` in `tender_version_amounts` through the `AMOUNTS` table, but `BT-195(BT-720)`
/// (`win-ten-val`, the winning tender value) is not in `AMOUNTS` at all — `project.rs:3875` routes it
/// into `raw.bids`, so it lands as a BID's own `cents` in `tender_version_bids`. An amounts-only query
/// therefore cannot see the second population, and the fix scoped to amounts alone would leave those
/// rows asserting a −0.01 bid.
///
/// **A third arm for the statistics satellite (issue 372 unit 4).** BT-759 (the received-submission
/// count) and BT-760 (its type) are withheld the same way, and the SDK writes -1 into the count and
/// the literal `unpublished` into the code — so an unmarked row asserts that -1 submissions of type
/// `unpublished` were received. Its predicate is `count < 0 OR kind = 'unpublished'` rather than a
/// cents test, because BOTH halves of the pair are placeholders and either can appear alone.
///
/// Denser than the amount case: a bounded probe of `tender_id <= 100000` alone returned **2,752
/// rows over 164 tenders** (~17 per tender — one statistics block per lot result), and 92 of 93 in a
/// narrower window sat in a notice that declared a withholding, matching the 99.6 % the amount side
/// showed. The corpus total is what this arm produces; the unbounded form of that probe hit the 10 s
/// cap and was not retried.
///
/// NOT included: the award-criterion markers from the same fixture (`BT-539`/`BT-541`/`BT-734`).
/// Those field ids reach no canonical satellite at all — verified 2026-09-09, they appear in no
/// field map and no routing arm — so their `-1`/`unpublished` values stay in the parsed layer where
/// ADR-0004 says they belong. There is nothing to mark and nothing to count.
///
/// Windowed probes put that population at **≥184 rows over ≥136 tenders** — a floor, not a total,
/// because the densest `tender_id` window hit the 10 s cap and was not retried. The `UNION ALL` arm
/// below replaces the floor with a corpus number on the next run. `ORDER BY 2` (the ordinal) rather
/// than the alias, because the ordering applies to the whole compound select.
///
/// **Split by SOURCE as well as field (issue 372 unit 5's follow-up).** The undeclared residue looks
/// like a publisher convention rather than the SDK marker, and a hand sample came back 3-of-4 `doe` —
/// suggestive, not a finding. Establishing it needed a `GROUP BY source` over the residue, which is
/// exactly what a bounded `/v1/sql` read cannot do: a 2,000,000-wide `tender_id` window carrying this
/// join plus the `EXISTS` hit the 10 s cap (2026-09-08, not retried). In-process the join is already
/// paid for, so the split is free here and the question answers itself on the next run instead of
/// staying a guess.
///
/// Affordable for a measured reason: the `WHERE a.cents = -100` scan is the same shape
/// `sentinel_amounts_sql` already pays for (the whole whole-corpus phase measured ~128 s of job
/// 816's 6,396 s), and the `EXISTS` is a two-column seek on `notice_sections(kind, notice_id)`
/// per matched row — ~19,000 seeks, not a second scan.
pub fn withheld_markers_sql() -> String {
    "SELECT a.field || ' · ' || n.source AS field, COUNT(*) AS hits, \
            SUM(CASE WHEN EXISTS ( \
                  SELECT 1 FROM notice_sections s \
                   WHERE s.kind = 'FieldsPrivacy' AND s.notice_id = v.caused_by_notice_id \
                ) THEN 1 ELSE 0 END) AS in_withholding_notice, \
            SUM(CASE WHEN a.quality IS NOT NULL THEN 1 ELSE 0 END) AS marked, \
            COUNT(DISTINCT a.tender_id) AS tenders \
       FROM tender_version_amounts a \
       JOIN tender_versions v ON v.tender_id = a.tender_id AND v.seq = a.seq \
       JOIN notices n ON n.id = v.caused_by_notice_id \
      WHERE a.cents = -100 \
      GROUP BY a.field, n.source \
     UNION ALL \
     SELECT 'bid_value · ' || n.source AS field, COUNT(*) AS hits, \
            SUM(CASE WHEN EXISTS ( \
                  SELECT 1 FROM notice_sections s \
                   WHERE s.kind = 'FieldsPrivacy' AND s.notice_id = v.caused_by_notice_id \
                ) THEN 1 ELSE 0 END) AS in_withholding_notice, \
            SUM(CASE WHEN b.quality IS NOT NULL THEN 1 ELSE 0 END) AS marked, \
            COUNT(DISTINCT b.tender_id) AS tenders \
       FROM tender_version_bids b \
       JOIN tender_versions v ON v.tender_id = b.tender_id AND v.seq = b.seq \
       JOIN notices n ON n.id = v.caused_by_notice_id \
      WHERE b.cents = -100 \
      GROUP BY n.source \
     UNION ALL \
     SELECT 'submission_stat · ' || n.source AS field, COUNT(*) AS hits, \
            SUM(CASE WHEN EXISTS ( \
                  SELECT 1 FROM notice_sections s \
                   WHERE s.kind = 'FieldsPrivacy' AND s.notice_id = v.caused_by_notice_id \
                ) THEN 1 ELSE 0 END) AS in_withholding_notice, \
            SUM(CASE WHEN t.quality IS NOT NULL THEN 1 ELSE 0 END) AS marked, \
            COUNT(DISTINCT t.tender_id) AS tenders \
       FROM tender_version_result_stats t \
       JOIN tender_versions v ON v.tender_id = t.tender_id AND v.seq = t.seq \
       JOIN notices n ON n.id = v.caused_by_notice_id \
      WHERE t.count < 0 OR t.kind = 'unpublished' \
      GROUP BY n.source \
      ORDER BY 2 DESC"
        .to_owned()
}

/// The queries that measure a population no `tender_id` window can slice, so the
/// in-process job runs them ONCE against the whole corpus instead of per window
/// (issue 246). Distinct from [`unwindowed_labels`], which is for a query that
/// cannot be measured at all.
/// How far back [`unmapped_fields_sql`] looks, in notice ids rather than time:
/// the newest slice of the corpus, which is where a vocabulary going stale shows
/// up first. Ids are dense enough at the head that this is ~100k notices.
pub const UNMAPPED_FIELD_WINDOW_IDS: i64 = 1_000_000;

/// How many unmodelled field ids the report lists.
///
/// Deliberately SMALL, and the reason is a measurement that arrived after this
/// section was first built. Run against prod 2026-09-09 over the window below,
/// **109 of 164 distinct published field ids have no destination, and they carry
/// 61.7 % of all published field rows** — because the canonical model is a
/// narrow subset by design. The head of that list is postal addresses
/// (`BT-513/512/510(a)-Organization-Company`), exclusion grounds (`BT-67(a/b)`),
/// award-criterion detail (`BT-539/540/5421-Lot`) and main nature (`BT-23-Lot`):
/// all correctly out of scope, none of them a defect.
///
/// So this is NOT issue 368's vocabulary diagnostic, and listing 60 of them
/// would have put a weekly section in front of a reader where 9 in 10 entries
/// are working-as-intended — the shape a diagnostic gets ignored for. What it
/// IS: a ranked "largest thing the model does not hold", useful for scope
/// decisions. Small cap, honest name.
///
/// 368's actual failure — a MODELLED concept missing because the closed
/// vocabulary did not know this publisher's spelling — needs the completeness
/// section as its entry point (a profile with a title gap), then this list
/// restricted to that profile. Filed as the follow-up rather than guessed at
/// here, because `any_channel_reads` is profile-BLIND: a field is either always
/// read or never, so no per-profile asymmetry is detectable with it.
pub const UNMAPPED_FIELD_LISTING_CAP: usize = 15;

/// Issue 368 unit 4b: which published field ids does the projection DROP, on how
/// many notices, under which profile.
///
/// The vocabulary is closed and hand-maintained, and an unlisted spelling
/// defaults silently instead of flagging — that is the whole of issue 368. This
/// turns "which spellings are we dropping" from a guess into a count.
///
/// ONE query over a UNION of the eight parsed value tables rather than eight
/// registered labels: the plumbing (a `Raw` slot, a `take`, a label-order entry)
/// is per-label, and the information is the same. `profile` rides along because
/// the same field id can be read on one profile and dropped on another, which is
/// exactly the DE-1.x alias case.
///
/// WINDOWED IN THE SQL, the [`fresh_holds_sql`] pattern, and this is the cost
/// decision unit 4a left open. Measured on prod 2026-09-09: over the newest
/// ~105k notice ids the eight tables hold **~1.0M rows** together
/// (`notice_texts` 257k, `notice_codes` 287k, `notice_ids` 230k, then dates,
/// amounts, integers, numbers, classifications) — cheap, with hash state bounded
/// by the small `(profile, field_id)` output. Extrapolated whole-corpus that is
/// ~300M rows across eight GROUP BYs, which is both a long scan and the exact
/// shape the issue-278 turso lesson warns about (see `longest_chain`, kept
/// GROUP-BY-free for that reason). There is no index to help: `field_id` is the
/// 4th column of each table's PK.
///
/// So this deliberately measures the HEAD of the corpus, not all of it — which
/// is also the question worth asking, since a vocabulary goes stale as new
/// spellings arrive. The report's convention is to say what it does not measure
/// rather than let a reader assume completeness, and the window is in the label.
pub fn unmapped_fields_sql() -> String {
    let arm = |table: &str| {
        format!(
            "SELECT n.profile AS profile, x.field_id AS field_id, COUNT(*) AS hits \
               FROM {table} x JOIN notices n ON n.id = x.notice_id \
              WHERE x.notice_id > (SELECT MAX(id) FROM notices) - {UNMAPPED_FIELD_WINDOW_IDS} \
              GROUP BY n.profile, x.field_id"
        )
    };
    let tables = [
        "notice_texts",
        "notice_codes",
        "notice_classifications",
        "notice_amounts",
        "notice_dates",
        "notice_integers",
        "notice_numbers",
        "notice_ids",
    ];
    let arms: Vec<String> = tables.iter().map(|t| arm(t)).collect();
    format!(
        "SELECT profile, field_id, SUM(hits) AS hits FROM ({}) \
          GROUP BY profile, field_id ORDER BY hits DESC",
        arms.join(" UNION ALL ")
    )
}

pub fn whole_corpus_queries() -> Vec<(String, String)> {
    vec![
        ("fresh_holds".to_owned(), fresh_holds_sql()),
        // The fold-cost tripwire (issue 92): fold() is O(chain²), the worst real
        // chain was 3,282 on 2026-08-26 and a DPS grows without bound, so the
        // approach to the fatal zone must be VISIBLE weekly rather than
        // discovered inside a slow fold. One streaming MAX over `tenders` —
        // no GROUP BY, no hash state (the 278 turso lesson), and per-window
        // maxima don't SUM so this cannot ride the windowed machinery.
        ("longest_chain".to_owned(), "SELECT MAX(current_seq) FROM tenders".to_owned()),
        // The sentinel discovery sweep (issue 366). Here rather than in
        // `windowed_queries` because a `HAVING` cannot be windowed — the builders'
        // docs carry the argument.
        ("sentinel_amounts".to_owned(), sentinel_amounts_sql()),
        ("sentinel_dates".to_owned(), sentinel_dates_sql()),
        // Issue 372: the withheld-marker residue.
        ("withheld_markers".to_owned(), withheld_markers_sql()),
        // Issue 364 unit 3: the weld detector, and the detector for 369 too —
        // both defects look identical from outside, so it is built once.
        ("weld_candidates".to_owned(), weld_candidates_sql()),
        ("weld_bands".to_owned(), weld_bands_sql()),
        // Issue 368 unit 4b: the dropped-vocabulary diagnostic. Whole-corpus
        // registration (not `windowed_queries`) because the `notice_*` tables key
        // on `notice_id` while the windowing machinery walks `tender_id` — there
        // is nothing for `{window}` to bind to — and because `sum_profile_counts`
        // would `as_i64` the `field_id` in column 1 to 0. Its own SQL window
        // bounds the cost instead.
        ("unmapped_fields".to_owned(), unmapped_fields_sql()),
    ]
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
    // The content-presence probe (issue 109).
    out.push(("factless".to_owned(), factless_template("")));
    // Amount plausibility (issue 267).
    out.push(("amount_plausibility".to_owned(), amount_plausibility_template("")));
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
    // Content presence (issue 109): drives from `tender_versions v` like the six
    // field probes, so the same range predicate windows it and per-profile counts
    // sum exactly across disjoint windows.
    out.push(WindowedQuery {
        label: "factless".to_owned(),
        template: factless_template("{window} AND "),
        column: "v.tender_id".to_owned(),
    });
    // Amount plausibility (issue 267): drives from `tender_versions v` like
    // `amount_basis`, so the same range predicate windows it and the counts sum.
    out.push(WindowedQuery {
        label: "amount_plausibility".to_owned(),
        template: amount_plausibility_template("WHERE {window} "),
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
        // UK Find a Tender: OCDS releases, not notices in the TED sense, and a
        // different publisher entirely — its own era whatever OCDS version the
        // package declares (issue 342).
        p if p.starts_with("fts:") => "FTS OCDS",
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

/// One era's content presence (issue 109): how many of its versions are
/// SHELLS — a `tender_versions` row with no rows in any version-keyed
/// satellite. The rate every row-counting gate is structurally blind to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresenceRow {
    pub profile: String,
    pub versions: u64,
    pub factless: u64,
}

/// One era's amount plausibility (issue 267): the standing per-era rates the
/// negative-amount analyses (131/134) established as the honest invariant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlausibilityRow {
    pub profile: String,
    pub amounts: u64,
    pub negative: u64,
    pub zero: u64,
    pub over_1e12: u64,
    /// Amounts whose `eur_cents` derivation resolved (ADR-0014 D4): the honest
    /// convertibility gauge — NULL is the policy for an unresolvable rate, so
    /// the RATE per era is the number that says how much of the corpus the
    /// EUR-normalized read surface actually covers.
    pub convertible: u64,
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
    /// Of the denominator, the versions that resolved a WINNER (issue 101). A
    /// result block with no winner in it is the shape issue 100 describes for
    /// eForms-DE 1.x, and reading it used to take two sections and an inference:
    /// section 1's `winner` column is a share of ALL versions, so 1.4 % there is
    /// only a gap once you know what fraction of the era's versions are awards.
    /// Beside the density it is one line — 100 % of DE-1.1's award notices
    /// materialise a result and 3 % of them name who won.
    pub with_winner: u64,
    /// Of the materialised results, the ones a winner could be expected FOR — the
    /// denominator `named` divides by (issue 258). Defined by what it excludes: a
    /// result that positively denies an award (`no-rece`/`clos-nw`/`open-nw`) AND
    /// names nobody. Everything else counts, including an UNSTATED decision —
    /// silence is not a denial (issues 100, 257) — and including the publisher's
    /// own contradiction of a denial that still names a winner, which occurs (5 of
    /// 2,895 in the sdk-0.1 2023-01 cross-tab). Anything with a winner is in here
    /// by construction, so `with_winner` is a subset and the rate cannot exceed
    /// 100 %; a rate above 1 is not a number, it is a bug report.
    pub with_awardable: u64,
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

/// One value the sentinel sweep found repeating in an implausible tail (issue 366).
///
/// Two kinds share the shape: an amount (`scope` = currency, `raw` = published cents)
/// and a date (`scope` = the date field, `raw` = utc seconds). `raw` is SIGNED because
/// both tails run below zero — a negative amount and a pre-epoch date are each a live
/// class, and [`as_u64`] would clamp them to 0 and hide exactly the rows that matter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentinelRow {
    pub scope: String,
    pub raw: i64,
    /// Satellite rows carrying the value, across every version.
    pub hits: u64,
    /// Distinct Tenders among them — what separates ONE Tender revised 90 times from
    /// 9,000 Tenders each publishing the same placeholder. The two want different
    /// fixes, and `hits` alone cannot tell them apart.
    pub tenders: u64,
}

/// One weld candidate (issue 364 unit 3): a Tender carrying many distinct buyer
/// organizations, which is what a fused component looks like from the outside.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WeldRow {
    pub tender_id: i64,
    /// Distinct buyer organizations across every version, BOTH role vocabularies.
    pub buyers: u64,
    /// Versions carrying a buyer — context for whether the buyers accumulated
    /// along a long chain or arrived on a short one.
    pub versions: u64,
}

/// One published field id the projection has no destination for (issue 368
/// unit 4b) — a spelling the closed vocabulary silently defaults on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnmappedFieldRow {
    pub profile: String,
    pub field_id: String,
    /// Rows carrying it in the measured window — NOT a corpus total; see
    /// [`unmapped_fields_sql`] for the window and why it is windowed.
    pub hits: u64,
}

/// One canonical amount field's `-1.00` rows, and how many sat in a notice that declared a
/// withholding (issue 372). The interesting number is the difference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WithheldMarkerRow {
    pub field: String,
    pub hits: u64,
    /// Rows whose notice declared SOME `FieldsPrivacy` withholding — not necessarily this
    /// field's. See [`withheld_markers_sql`] for why the exact test lives in unit 2 instead.
    pub in_withholding_notice: u64,
    /// Rows the projection actually marked `withheld` (issue 372 unit 2). The
    /// per-row test is exact where `in_withholding_notice` is notice-wide, so the
    /// gap between the two is what the section-anchored rule does NOT reach —
    /// a publisher who hoisted the `FieldsPrivacy` block away from the value it
    /// suppresses. That was a stated risk of the fix; this makes it a number.
    ///
    /// Reads low against the standing corpus until rows are re-folded, so read it
    /// on the recent-notice rows, not as a corpus share.
    pub marked: u64,
    pub tenders: u64,
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
    /// Factless (shell) versions per era (issue 109).
    pub presence: Vec<PresenceRow>,
    /// Amount plausibility per era (issue 267).
    pub plausibility: Vec<PlausibilityRow>,
    /// Published amounts repeating in the implausible tail (issue 366), most-repeated
    /// first.
    pub weld_candidates: Vec<WeldRow>,
    /// Tenders at each of [`WELD_BANDS`], so the listing cap cannot be read as the population.
    pub weld_bands: Vec<u64>,
    pub sentinel_amounts: Vec<SentinelRow>,
    /// Published dates repeating outside the plausible range (issue 366).
    pub sentinel_dates: Vec<SentinelRow>,
    /// The `-1.00` withheld-marker rows per field, with their declared share (issue 372).
    pub withheld_markers: Vec<WithheldMarkerRow>,
    /// Issue 368 unit 4b, ranked by hits and capped: the biggest field volumes
    /// the canonical model does not hold. Mostly out of scope BY DESIGN — see
    /// [`UNMAPPED_FIELD_LISTING_CAP`] for the 61.7 % measurement that says so,
    /// and why this is not yet the vocabulary diagnostic 368 asked for.
    pub unmapped_fields: Vec<UnmappedFieldRow>,
    /// The longest version chain in the corpus (`MAX(tenders.current_seq)`) —
    /// the fold-cost tripwire (issue 92). 0 when unmeasured or the layer is
    /// empty; the render distinguishes the two via [`Report::unmeasured`].
    pub longest_chain: u64,
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

/// Read a JSON cell as a SIGNED value. [`as_u64`] clamps at zero, which is right for a
/// count and wrong for a published amount or instant — the negative tails are the point.
fn as_i64(cell: Option<&Value>) -> i64 {
    match cell {
        Some(v) => v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)).unwrap_or(0),
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
    /// Factless versions per era (issue 109).
    pub factless: Rows,
    /// Amount plausibility per era (issue 267).
    pub amount_plausibility: Rows,
    /// The single-row `MAX(current_seq)` fold-cost tripwire (issue 92).
    pub longest_chain: Rows,
    /// `[currency, cents, hits, tenders]` per repeated implausible amount (issue 366).
    pub weld_candidates: Rows,
    pub weld_bands: Rows,
    pub sentinel_amounts: Rows,
    /// `[field, instant, hits, tenders]` per repeated implausible DAY (issue 366) — the
    /// instant is the day's earliest member, standing for the whole day.
    pub sentinel_dates: Rows,
    /// `[field, hits, in_withholding_notice, tenders]` per amount field (issue 372).
    pub withheld_markers: Rows,
    /// Issue 368 unit 4b: `(profile, field_id, hits)` for every field id
    /// published in the newest slice — unfiltered. The projection's own
    /// predicate decides which of them are DROPPED, in Rust, because the match
    /// is full-id OR stem and the DE-1.x aliases resolve there (unit 4a).
    pub unmapped_fields: Rows,
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
            factless: take("factless", &mut unmeasured)?,
            amount_plausibility: take("amount_plausibility", &mut unmeasured)?,
            longest_chain: take("longest_chain", &mut unmeasured)?,
            weld_candidates: take("weld_candidates", &mut unmeasured)?,
            weld_bands: take("weld_bands", &mut unmeasured)?,
            sentinel_amounts: take("sentinel_amounts", &mut unmeasured)?,
            sentinel_dates: take("sentinel_dates", &mut unmeasured)?,
            withheld_markers: take("withheld_markers", &mut unmeasured)?,
            unmapped_fields: take("unmapped_fields", &mut unmeasured)?,
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
            with_winner: as_u64(r.get(4)),
            with_awardable: as_u64(r.get(5)),
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

    // Content presence (issue 109): denominator from the SAME `versions` result
    // the completeness table uses, numerator absent ⇒ zero shells — an era with
    // no factless row is the healthy case, not a gap.
    let shells = count_by_profile(&raw.factless);
    let mut presence: Vec<PresenceRow> = raw
        .versions
        .iter()
        .map(|r| {
            let profile = as_str(r.first());
            PresenceRow {
                versions: as_u64(r.get(1)),
                factless: shells.get(&profile).copied().unwrap_or(0),
                profile,
            }
        })
        .collect();
    presence.sort_by(|a, b| a.profile.cmp(&b.profile));

    let mut plausibility: Vec<PlausibilityRow> = raw
        .amount_plausibility
        .iter()
        .map(|r| PlausibilityRow {
            profile: as_str(r.first()),
            amounts: as_u64(r.get(1)),
            negative: as_u64(r.get(2)),
            zero: as_u64(r.get(3)),
            over_1e12: as_u64(r.get(4)),
            convertible: as_u64(r.get(5)),
        })
        .collect();
    plausibility.sort_by(|a, b| a.profile.cmp(&b.profile));

    let longest_chain = raw.longest_chain.first().map(|r| as_u64(r.first())).unwrap_or(0);

    // Both sentinel listings arrive ranked by the SQL (most-repeated first) and stay in
    // that order rather than being re-sorted: "which value has earned a rule" is the
    // question, and repetition is the answer to it.
    let sentinel = |rows: &Rows| -> Vec<SentinelRow> {
        rows.iter()
            .map(|r| SentinelRow {
                scope: as_str(r.first()),
                raw: as_i64(r.get(1)),
                hits: as_u64(r.get(2)),
                tenders: as_u64(r.get(3)),
            })
            .collect()
    };

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
        presence,
        plausibility,
        longest_chain,
        weld_candidates: raw
            .weld_candidates
            .iter()
            .map(|r| WeldRow {
                tender_id: as_i64(r.first()),
                buyers: as_u64(r.get(1)),
                versions: as_u64(r.get(2)),
            })
            .collect(),
        // One row of N sums; an absent query leaves it empty and the render says
        // UNMEASURED rather than printing zeros, which for a detector is the one
        // confusion worth spending a branch on.
        weld_bands: raw
            .weld_bands
            .first()
            .map(|r| (0..WELD_BANDS.len()).map(|i| as_u64(r.get(i))).collect())
            .unwrap_or_default(),
        sentinel_amounts: sentinel(&raw.sentinel_amounts),
        sentinel_dates: sentinel(&raw.sentinel_dates),
        withheld_markers: raw
            .withheld_markers
            .iter()
            .map(|r| WithheldMarkerRow {
                field: as_str(r.first()),
                hits: as_u64(r.get(1)),
                in_withholding_notice: as_u64(r.get(2)),
                marked: as_u64(r.get(3)),
                tenders: as_u64(r.get(4)),
            })
            .collect(),
        // Issue 368 unit 4b: the SQL returns EVERY field id published in the
        // window; the drop test happens here because it cannot be expressed in
        // SQL — the projection matches on full id OR two-segment stem, and the
        // DE-1.x aliases resolve to their eForms target in Rust first (unit 4a).
        // A channel-blind predicate would report the whole legacy era as read,
        // which is the one era this has to be honest about, so `any_channel_reads`
        // is the right question: is this id read on ANY channel at all.
        unmapped_fields: raw
            .unmapped_fields
            .iter()
            .map(|r| UnmappedFieldRow {
                profile: as_str(r.first()),
                field_id: as_str(r.get(1)),
                hits: as_u64(r.get(2)),
            })
            .filter(|row| !crate::project::any_channel_reads(&row.field_id))
            .take(UNMAPPED_FIELD_LISTING_CAP)
            .collect(),
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

    let _ = writeln!(
        out,
        "  `value` tracks what publishers STATE, not what we extract (issue 263, archive-sampled \
         2026-08-22): eforms-de-1.1 members carry a money element at ~16% at source (layer: 30% — \
         chains merge notices), DÖE sdk-0.1 at 0 of 50, EU-SDK members at ~50-70%. A low value \
         column is publisher behaviour; the extraction is acquitted for every sampled era."
    );

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
        "  A YOUNG era's rate is a maturation curve, not a quality claim (issue 264): an award \
         Tender is unchained BY CONSTRUCTION until its contract notice arrives and groups, so the \
         newest rows read low and climb for months. Measured inside one era (sdk-1.13, 2026-08-22): \
         65.6% linked among its oldest-minted Tenders vs 2.3% among the newest-minted. Compare an \
         era against its own last run, not against an older era."
    );

    let _ = writeln!(
        out,
        "\n== 3. Results materialisation (notices whose PUBLISHED type announces a result → lot_results) =="
    );
    let _ = writeln!(
        out,
        "  {:<30} {:>10} {:>14} {:>8} {:>16} {:>12} {:>9} {:>7}",
        "era", "award-notices", "with lot_results", "density", "no block parsed", "with winner",
        "closed n/a", "named"
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
            "  {:<30} {:>10} {:>14} {:>8} {:>16} {:>12} {:>9} {:>7}",
            display_era(&row.profile),
            group(row.award_notices),
            group(row.with_results),
            rate,
            group(row.no_award_content),
            group(row.with_winner),
            // Results the publisher CLOSED without naming anybody — `no-rece`, `clos-nw`,
            // `open-nw` with no winner. Not a gap of ours, and its own number rather than
            // a residue you have to subtract to find (issue 258).
            group(row.with_results.saturating_sub(row.with_awardable)),
            // Against the results a winner could be expected FOR, not against every
            // materialised result: a missing result block is already counted two columns
            // left, and a result the publisher closed with nobody is counted one column
            // left. Neither is the winner chain's failure, and dividing by either would
            // blame it for them (issues 101, 258).
            pct(row.with_winner, row.with_awardable),
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

    let _ = writeln!(
        out,
        "\n== 7. Content presence (versions with NO satellite rows — shells, issue 109) =="
    );
    if report.unmeasured.iter().any(|l| l == "factless") {
        let _ = writeln!(out, "  UNMEASURED — the `factless` query did not run.");
    } else {
        let _ = writeln!(out, "  {:<30} {:>11} {:>10} {:>9}", "era", "versions", "factless", "rate");
        for r in &report.presence {
            let _ = writeln!(
                out,
                "  {:<30} {:>11} {:>10} {:>9}",
                display_era(&r.profile),
                group(r.versions),
                group(r.factless),
                pct(r.factless, r.versions),
            );
        }
        let _ = writeln!(
            out,
            "  A shell HAS a `tender_versions` row, so every row-counting gate (G2, cohort \
             counts, the projected watermark) stays green while a cohort is 100% stale — \
             measured in issue 85, where 218,635 eForms-DE 1.x shells sat green for weeks. \
             The alert is the STEP CHANGE between runs, not an absolute floor: per-era fact \
             density legitimately varies."
        );
    }

    let _ = writeln!(out, "\n== 8. Amount plausibility (negative / zero / >1e12 rates — issue 267) ==");
    if report.unmeasured.iter().any(|l| l == "amount_plausibility") {
        let _ = writeln!(out, "  UNMEASURED — the `amount_plausibility` query did not run.");
    } else {
        let _ = writeln!(
            out,
            "  {:<30} {:>11} {:>9} {:>9} {:>9} {:>9}",
            "era", "amounts", "negative", "zero", ">1e12", "eur-conv"
        );
        for r in &report.plausibility {
            let _ = writeln!(
                out,
                "  {:<30} {:>11} {:>9} {:>9} {:>9} {:>9}",
                display_era(&r.profile),
                group(r.amounts),
                group(r.negative),
                group(r.zero),
                group(r.over_1e12),
                pct(r.convertible, r.amounts),
            );
        }
        let _ = writeln!(
            out,
            "  Negatives are overwhelmingly SOURCE-published (131/132: row-by-row diagnosed), so \
             their existence is not a defect — the RATE moving between runs is the signal, for the \
             fabricated-negative parser-regression class. >1e12 is a crude tripwire for \
             unrepresentable values escaping quarantine into the layer, not a claim about any \
             single amount. eur-conv is the share of amounts whose `eur_cents` derivation \
             resolved a rate (ADR-0014 D4: unresolvable is NULL, never a guess) — near-zero \
             per era until that era's backfill refold has run."
        );
    }

    let _ = writeln!(out, "\n== 9. Fold-cost tripwire (longest version chain — issue 92) ==");
    if report.unmeasured.iter().any(|l| l == "longest_chain") {
        let _ = writeln!(out, "  UNMEASURED — the `longest_chain` query did not run.");
    } else {
        let _ = writeln!(
            out,
            "  longest chain: {}{}",
            group(report.longest_chain),
            if report.longest_chain >= LONGEST_CHAIN_FLAG {
                " — FLAG: >= 4,000. fold() is O(chain^2) (issue 92); the measured decision \
                 point is here. Measure the real fold wall-time and decide the rewrite."
            } else {
                " (flag threshold 4,000; fold() is O(chain^2), issue 92)"
            },
        );
    }

    // Issue 366's discovery half. Sections 8 and 9 measure shapes we already named;
    // this one exists to surface the ones we did not. It is a CANDIDATE listing, not a
    // defect count: a repeated implausible value is a value that has earned a look,
    // and the judgement of whether it is a sentinel stays with the reader.
    let _ = writeln!(out, "\n== 10. Repeated implausible values (sentinel discovery — issue 366) ==");
    render_sentinels(
        &mut out,
        "sentinel_amounts",
        "amounts — negative at any size, or >= EUR 1,000,000,000 equivalent; grouped AS PUBLISHED",
        "currency",
        &report.unmeasured,
        &report.sentinel_amounts,
        major,
        SENTINEL_MIN_REPEATS,
    );
    render_sentinels(
        &mut out,
        "sentinel_dates",
        &format!(
            "dates — by DAY; before 1990-01-01, or more than {} years after this run",
            SENTINEL_DATE_HORIZON_SECS / (365 * 86_400)
        ),
        "field",
        &report.unmeasured,
        &report.sentinel_dates,
        day_utc_signed,
        SENTINEL_DATE_MIN_REPEATS,
    );
    let _ = writeln!(
        out,
        "  A value here is a CANDIDATE, not a verdict. The head-column election takes an \
         unconditional `.max()` over version facts (issue 366), so any sentinel a source \
         publishes wins the column outright — which is how -1.00 and the all-nines maxima \
         came to stand in roughly {} tenders before anyone looked. What makes a row \
         suspicious is repetition at a magnitude or on a day no procurement reaches: one \
         Tender is a typo, the {} carrying -1.00 are a publisher convention. Read `tenders` \
         next to `rows`, then either add the shape to `store::canonical::sentinel_amount` / \
         the deadline horizon, or record on issue 366 why the value is genuine.",
        group(16_000),
        group(15_529),
    );
    let _ = writeln!(
        out,
        "\n== 11. Withheld-marker amounts (`-1.00` rows and their declared share — issue 372) =="
    );
    if report.unmeasured.iter().any(|l| l == "withheld_markers") {
        let _ = writeln!(out, "  UNMEASURED — the `withheld_markers` query did not run.");
    } else if report.withheld_markers.is_empty() {
        let _ = writeln!(out, "  none — no amount row holds exactly -1.00.");
    } else {
        let _ = writeln!(
            out,
            "  {:<34}{:>12}{:>16}{:>10}{:>10}{:>12}",
            "field · source", "rows", "in-wh-notice", "residue", "marked", "tenders"
        );
        for r in &report.withheld_markers {
            let _ = writeln!(
                out,
                "  {:<34}{:>12}{:>16}{:>10}{:>10}{:>12}",
                r.field,
                group(r.hits),
                group(r.in_withholding_notice),
                group(r.hits.saturating_sub(r.in_withholding_notice)),
                group(r.marked),
                group(r.tenders),
            );
        }
        let _ = writeln!(
            out,
            "  `-1.00` is the eForms SDK's WITHHELD-value marker, not a publisher convention: under \
             BT-195/`FieldsPrivacy` a withheld field is published as the code `unpublished` and the \
             number -1, with the reason and the date it becomes publishable beside it (issue 372). \
             `residue` is the number to read — a DECLARED withholding can be marked precisely, \
             while an undeclared -1 is a guess about intent and may deserve quarantine instead. \
             `in-wh-notice` counts notices withholding SOMETHING, not necessarily this field: the \
             exact test needs the source→canonical mapping, which lives in Rust, so it belongs to \
             372 unit 2 rather than to this SQL. `marked` is unit 2's per-row verdict, which IS \
             exact — so `in-wh-notice` minus `marked` is what the section-anchored rule does not \
             reach, a block the publisher hoisted away from the value it suppresses. Read it on \
             recently folded rows: standing rows carry no marker until they are re-folded, so a \
             low corpus-wide `marked` says nothing about the rule. `submission_stat` rows are the \
             BT-759/BT-760 pair, where the marker means neither the count NOR the type is a \
             reading."
        );
    }
    let _ = writeln!(
        out,
        "\n== 12. Weld candidates (Tenders with many distinct buyers — issues 364 / 369) =="
    );
    if report.unmeasured.iter().any(|l| l == "weld_candidates" || l == "weld_bands") {
        let _ = writeln!(
            out,
            "  UNMEASURED — the weld query did not run. This is NOT 'no welds found'; \
             the two claims are opposite and a detector must never render them alike."
        );
    } else {
        let bands: Vec<String> = WELD_BANDS
            .iter()
            .zip(report.weld_bands.iter())
            .map(|(n, c)| format!(">= {n}: {}", group(*c)))
            .collect();
        let _ = writeln!(out, "  Tenders by DISTINCT buyer organizations — {}", bands.join("   "));
        let _ = writeln!(
            out,
            "  {:<12} {:>8} {:>10} {:>9}",
            "tender", "buyers", "versions", "per-ver"
        );
        for row in &report.weld_candidates {
            // Derived here rather than in SQL: both numbers are already on the row, so
            // the discriminator costs nothing. A zero version count cannot happen (a
            // buyer row implies a version) but is not worth a panic in a report.
            let per_version = if row.versions == 0 {
                "—".to_owned()
            } else {
                format!("{:.1}", row.buyers as f64 / row.versions as f64)
            };
            let _ = writeln!(
                out,
                "  {:<12} {:>8} {:>10} {:>9}",
                row.tender_id,
                group(row.buyers),
                group(row.versions),
                per_version
            );
        }
        if report.weld_candidates.len() >= WELD_LISTING_CAP {
            let _ = writeln!(
                out,
                "  LISTING FULL at {WELD_LISTING_CAP} — the tail is longer than what is shown."
            );
        }
        let _ = writeln!(
            out,
            "  Many buyers on one Tender means EITHER unrelated procurements were fused — the \
             legacy OJS closure (364) or an unchecked procedure key (369) — OR a genuine joint \
             procurement. **The buyer count alone does NOT tell them apart, at any threshold.** \
             The first run of this section (2026-09-10) refuted the claim that stood here: \
             tender 331647 carries 505 buyers on ONE version, and its title reads `Skupno javno \
             naročilo` — Slovenian for joint public procurement. Several more of the top 40 are \
             the same Slovenian joint fuel-purchasing notices. So >= 50 is NOT a safe reading, \
             and neither is >= 500.\n  \
             Read the `per-ver` column instead: it is buyers divided by versions, and it \
             separates the two shapes at a glance. A joint procurement names all its buyers in \
             ONE notice, so per-ver is close to the buyer count (331647: 505.0). A weld \
             accumulates buyers ACROSS versions, so per-ver is small — tender 2816628, the weld \
             issue 364 was filed about, is 127 buyers over 2,983 versions (0.04). It is a HINT, \
             not a test: a weld whose versions each named many buyers would score high too. The \
             honest discriminator is the widest SINGLE version, which is a second pass nobody \
             has paid for yet (issue 364).\n  \
             Counted over ALL versions and over BOTH buyer roles (`buyer` for the legacy era, \
             `Procedure-Buyer` for eForms): 2816628 carries its 127 under the former and none \
             under the latter, so a one-vocabulary gauge reads green on it. Bands open at >= {}."
            ,
            WELD_MIN_BUYERS
        );
    }

    // Issue 368 unit 4b. This section did not exist until 2026-09-10: the query was
    // registered, ran every week, and its rows were assembled into `Report` — where
    // nothing read them. `assemble` filtered and capped them; `render_text` never
    // mentioned the field. So the diagnostic cost its scan every run and produced
    // nothing anybody could see, which is the "deployed but inert" shape this
    // codebase keeps finding. Caught by walking `Report`'s fields against the
    // renderer's body; it was the only one.
    let _ = writeln!(
        out,
        "\n== 13. Unmodelled published fields (source vocabulary nothing reads — issue 368) =="
    );
    if report.unmeasured.iter().any(|l| l == "unmapped_fields") {
        let _ = writeln!(out, "  UNMEASURED — the `unmapped_fields` query did not run.");
    } else if report.unmapped_fields.is_empty() {
        let _ = writeln!(
            out,
            "  none in the window — every field id the newest notices publish has a \
             destination on some channel."
        );
    } else {
        let _ = writeln!(out, "  {:<26} {:<34} {:>12}", "profile", "field id", "rows");
        for r in &report.unmapped_fields {
            let _ = writeln!(out, "  {:<26} {:<34} {:>12}", r.profile, r.field_id, group(r.hits));
        }
        if report.unmapped_fields.len() >= UNMAPPED_FIELD_LISTING_CAP {
            let _ = writeln!(
                out,
                "  LISTING FULL at {UNMAPPED_FIELD_LISTING_CAP} — there are more; the cap is \
                 deliberate, see below."
            );
        }
        let _ = writeln!(
            out,
            "  **Being on this list is NOT a defect.** A field id here is published by the source \
             and read by no channel, and the canonical model is a narrow subset ON PURPOSE — when \
             these were hand-read (2026-09-09), every one of the top entries was correctly out of \
             scope: exclusion grounds, postal-address parts, award-criterion detail, main nature. \
             What the section is FOR is scope decisions: it names the largest volumes the model \
             does not hold, so choosing to hold one is an informed choice rather than a \
             discovery.\n  \
             **And it is NOT the detector for issue 368's own failures.** Those were a MODELLED \
             concept going missing because the closed vocabulary did not know one publisher's \
             spelling — 29,455 titleless tenders, r208's 100 %-null lot titles. \
             `any_channel_reads` is profile-BLIND: a field is either always read or never, so no \
             per-profile asymmetry can show through it. That entry point is the completeness \
             section (a profile with a gap), then this list restricted to that profile — issue \
             368 unit 4c.\n  \
             Two bounds on the numbers. WINDOWED to the newest {UNMAPPED_FIELD_WINDOW_IDS} notice \
             ids (~100k notices), because a vocabulary going stale shows at the head first, so \
             `rows` is a window count and NOT a corpus total. And capped at \
             {UNMAPPED_FIELD_LISTING_CAP}: 109 of 164 distinct published ids had no destination \
             when measured, and a page where nine entries in ten are working as intended is how a \
             diagnostic earns being ignored."
        );
    }
    out
}

/// One sentinel listing, rendered. Shared by both halves of section 10 so the amount and
/// date tails cannot drift into different presentations of the same finding.
///
/// `value` formats the raw column for reading — cents as major units, seconds as a day —
/// while the JSON keeps `raw` beside it, so a value this cannot render (a year outside
/// `civil_date`'s `u16`) is still recoverable from the machine report.
fn render_sentinels(
    out: &mut String,
    label: &str,
    title: &str,
    scope_head: &str,
    unmeasured: &[String],
    rows: &[SentinelRow],
    value: fn(i64) -> String,
    min_repeats: u64,
) {
    use std::fmt::Write;
    let _ = writeln!(out, "  -- {title} --");
    if unmeasured.iter().any(|l| l == label) {
        let _ = writeln!(out, "  UNMEASURED — the `{label}` query did not run.");
        return;
    }
    if rows.is_empty() {
        let _ = writeln!(
            out,
            "  none — nothing in this tail repeats on {} rows or more. Not the same claim as \
             \"the tail is empty\": singletons are filtered in SQL.",
            group(min_repeats),
        );
        return;
    }
    // 22, not 10: at 10 every date field truncated to `duration…` on the first real run,
    // so the listing could not tell `duration_start` from `duration_end` — and that is
    // exactly what decides whether a far-future day is normal (an open-ended framework's
    // end) or wrong (its start). A diagnostic must not hide the discriminating half of
    // its own key.
    let _ = writeln!(out, "  {:<22}{:>26}{:>12}{:>12}", scope_head, "value", "rows", "tenders");
    for r in rows {
        let mut scope = r.scope.clone();
        if scope.chars().count() > 21 {
            scope = scope.chars().take(20).collect::<String>() + "…";
        }
        let _ = writeln!(
            out,
            "  {:<22}{:>26}{:>12}{:>12}",
            scope,
            value(r.raw),
            group(r.hits),
            group(r.tenders),
        );
    }
    if rows.len() >= SENTINEL_LISTING_CAP {
        let _ = writeln!(
            out,
            "  LISTING FULL at {SENTINEL_LISTING_CAP} — ranked by repetition, so what is cut \
             repeats least, but this is a truncated tail and not the whole of it."
        );
    }
}

/// Cents as major units, thousands-separated, sign kept — the reading a currency figure
/// wants. Not localised: one format for every currency in the corpus beats twenty-six
/// half-right ones in a diagnostic table.
fn major(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    format!("{sign}{}.{:02}", group(abs / 100), abs % 100)
}

/// [`day_utc`] for a SIGNED instant. Two differences, both needed here: the pre-epoch
/// tail is a sentinel class of its own, and `0` is a real date in this section
/// (1970-01-01, the epoch-zero default) rather than the "none" that `day_utc` reads it as.
fn day_utc_signed(unix: i64) -> String {
    let (y, m, d) = crate::fetch::civil_date(unix);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The chain length at which the weekly report flags the approach to the fold's
/// quadratic-cost fatal zone (issue 92's "on the clock" decision).
pub const LONGEST_CHAIN_FLAG: u64 = 4_000;

/// One run's per-era headline rates as a compact JSON history entry (issue
/// 265). Every rate is stored as `[numerator, denominator]` rather than a
/// float, so a consumer computing deltas divides ONCE, its own way, and two
/// surfaces can never disagree by rounding. The dashboard renders the last N
/// of these; the /metrics gauges (issue 266) read the newest.
pub fn headline_history_entry(report: &Report, computed_at: i64) -> serde_json::Value {
    let by_profile_linkage: std::collections::BTreeMap<&str, &LinkageRow> =
        report.linkage.iter().map(|r| (r.profile.as_str(), r)).collect();
    let by_profile_density: std::collections::BTreeMap<&str, &DensityRow> =
        report.density.iter().map(|r| (r.profile.as_str(), r)).collect();
    let by_profile_basis: std::collections::BTreeMap<&str, &BasisRow> =
        report.amount_basis.iter().map(|r| (r.profile.as_str(), r)).collect();
    let by_profile_plaus: std::collections::BTreeMap<&str, &PlausibilityRow> =
        report.plausibility.iter().map(|r| (r.profile.as_str(), r)).collect();
    // `value` is FIELDS[2] in the completeness present-array.
    let eras: Vec<serde_json::Value> = report
        .completeness
        .iter()
        .map(|c| {
            let p = c.profile.as_str();
            let factless =
                report.presence.iter().find(|r| r.profile == p).map_or(0, |r| r.factless);
            let named = by_profile_density
                .get(p)
                .map_or([0, 0], |d| [d.with_winner, d.with_awardable]);
            let linkage = by_profile_linkage
                .get(p)
                .map_or([0, 0], |l| [l.awards - l.unchained.min(l.awards), l.awards]);
            let vat = by_profile_basis.get(p).map_or([0, 0], |b| [b.excl + b.incl, b.amounts]);
            let neg = by_profile_plaus.get(p).map_or([0, 0], |x| [x.negative, x.amounts]);
            let eur = by_profile_plaus.get(p).map_or([0, 0], |x| [x.convertible, x.amounts]);
            json!({
                "profile": p,
                "versions": c.versions,
                "factless": [factless, c.versions],
                "value": [c.present[2], c.versions],
                "named": named,
                "linkage": linkage,
                "vat_stated": vat,
                "negative": neg,
                "eur_convertible": eur,
            })
        })
        .collect();
    json!({ "at": computed_at, "eras": eras, "longest_chain": report.longest_chain })
}

/// Append one entry to the stored headline history, keeping the newest
/// [`HEADLINE_HISTORY_KEEP`] (issue 265). Pure so it is testable without a
/// store: `existing` is the stored JSON array (or garbage/empty — a corrupt
/// history is dropped, never fatal, because losing a trend beats failing the
/// measurement that would extend it).
pub fn append_headline_history(existing: &str, entry: serde_json::Value) -> String {
    let mut history: Vec<serde_json::Value> =
        serde_json::from_str(existing).unwrap_or_default();
    history.push(entry);
    if history.len() > HEADLINE_HISTORY_KEEP {
        let drop = history.len() - HEADLINE_HISTORY_KEEP;
        history.drain(..drop);
    }
    serde_json::to_string(&history).unwrap_or_else(|_| "[]".into())
}

/// Weekly runs kept in the headline history — a quarter's trend, bounded.
pub const HEADLINE_HISTORY_KEEP: usize = 12;

/// The step-change alarm on the presence rates (issue 109): compare this run's
/// per-era factless rate to the previous run's and name every era that went
/// wholesale stale between them. A JUMP is what the incident actually looked
/// like (2% → 100% between two mapping changes), so the trigger is a rise of
/// ≥ 20 percentage points that at least doubled the rate, on a cohort big
/// enough to mean it (≥ 1,000 versions) — not an absolute floor, which would be
/// wrong per era on day one and rot after. First run (no previous rates): no
/// alarms, by construction.
pub fn presence_step_changes(previous: &[(String, u64, u64)], presence: &[PresenceRow]) -> Vec<String> {
    let prev: std::collections::BTreeMap<&str, (u64, u64)> =
        previous.iter().map(|(p, v, f)| (p.as_str(), (*v, *f))).collect();
    let mut alarms = Vec::new();
    for r in presence {
        if r.versions < 1_000 {
            continue;
        }
        let Some(&(pv, pf)) = prev.get(r.profile.as_str()) else { continue };
        if pv == 0 {
            continue;
        }
        let was = pf as f64 / pv as f64;
        let now = r.factless as f64 / r.versions as f64;
        if now - was >= 0.20 && now >= 2.0 * was.max(f64::EPSILON) {
            // The RAW profile, not `display_era` — an alarm names the exact
            // cohort to act on, and the era display collapses unmapped profiles
            // to \"other\".
            alarms.push(format!(
                "{}: factless {:.1}% -> {:.1}% ({} of {} versions) — a cohort went content-stale \
                 between runs (issue 109)",
                r.profile,
                100.0 * was,
                100.0 * now,
                r.factless,
                r.versions,
            ));
        }
    }
    alarms
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
            "with_winner": r.with_winner,
            // The denominator is the results a winner could be expected for, so this can
            // never exceed 1 (issue 258); `closed_no_winner` is the excluded population,
            // published rather than left to be derived by subtraction.
            "with_awardable": r.with_awardable,
            "closed_no_winner": r.with_results.saturating_sub(r.with_awardable),
            "winner_rate": rate(r.with_winner, r.with_awardable),
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
    let presence: Vec<Value> = report
        .presence
        .iter()
        .map(|r| json!({
            "profile": r.profile,
            "era": era_of(&r.profile),
            "versions": r.versions,
            "factless": r.factless,
            "factless_rate": rate(r.factless, r.versions),
        }))
        .collect();
    let plausibility: Vec<Value> = report
        .plausibility
        .iter()
        .map(|r| json!({
            "profile": r.profile,
            "era": era_of(&r.profile),
            "amounts": r.amounts,
            "negative": r.negative,
            "zero": r.zero,
            "over_1e12": r.over_1e12,
            "negative_rate": rate(r.negative, r.amounts),
            "eur_convertible": r.convertible,
            "eur_convertible_rate": rate(r.convertible, r.amounts),
        }))
        .collect();
    let sentinels = |rows: &[SentinelRow], shown: fn(i64) -> String| -> Vec<Value> {
        rows.iter()
            .map(|r| json!({
                "scope": r.scope,
                // Raw AND rendered: the raw column is the one a follow-up query can use,
                // and the rendered one is what a reader recognises a sentinel by.
                "raw": r.raw,
                "shown": shown(r.raw),
                "rows": r.hits,
                "tenders": r.tenders,
            }))
            .collect()
    };
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
        "content_presence": presence,
        "amount_plausibility": plausibility,
        "ted_doe_merge": {
            "doe_tenders": report.merge.doe_tenders,
            "merged": report.merge.merged,
            "rate": rate(report.merge.merged, report.merge.doe_tenders),
        },
        "longest_chain": report.longest_chain,
        // Issue 366's sentinel discovery sweep. The thresholds ride along because a
        // listing read six months from now must say what region it covered — a later
        // run with a different floor is not comparable to this one.
        "withheld_markers": report
            .withheld_markers
            .iter()
            .map(|r| json!({
                "field": r.field,
                "rows": r.hits,
                "in_withholding_notice": r.in_withholding_notice,
                // Precomputed: the residue is the number consumers want, and deriving it in
                // every consumer is how two of them come to disagree about it.
                "residue": r.hits.saturating_sub(r.in_withholding_notice),
                // Unit 2's exact per-row verdict, beside the notice-wide count it
                // refines. Their difference is the anchoring gap.
                "marked": r.marked,
                "tenders": r.tenders,
            }))
            .collect::<Vec<Value>>(),
        "repeated_implausible": {
            "min_repeats": SENTINEL_MIN_REPEATS,
            "date_min_repeats": SENTINEL_DATE_MIN_REPEATS,
            "listing_cap": SENTINEL_LISTING_CAP,
            "amount_floor_cents": SENTINEL_AMOUNT_FLOOR,
            "date_floor": SENTINEL_DATE_FLOOR,
            "date_horizon_secs": SENTINEL_DATE_HORIZON_SECS,
            "amounts": sentinels(&report.sentinel_amounts, major),
            "dates": sentinels(&report.sentinel_dates, day_utc_signed),
        },
        // Issue 364's weld gauge, and 368's unmodelled-field listing. Both were in
        // the text render and NOT here, which is the same inertia issue 368 unit 4b
        // had against `render_text` — a machine consumer could not see them at all.
        //
        // `per_version` is precomputed for the same reason `residue` is above: it is
        // the number that separates a weld from a joint procurement, and deriving it
        // in every consumer is how two of them come to disagree. `bands` pairs each
        // threshold with its count so the array cannot be read against the wrong
        // WELD_BANDS if the constant ever changes.
        "weld_candidates": {
            "min_buyers": WELD_MIN_BUYERS,
            "listing_cap": WELD_LISTING_CAP,
            "bands": WELD_BANDS
                .iter()
                .zip(report.weld_bands.iter())
                .map(|(n, c)| json!({ "at_least": n, "tenders": c }))
                .collect::<Vec<Value>>(),
            "listing": report
                .weld_candidates
                .iter()
                .map(|r| json!({
                    "tender_id": r.tender_id,
                    "buyers": r.buyers,
                    "versions": r.versions,
                    "per_version": if r.versions == 0 {
                        Value::Null
                    } else {
                        json!(r.buyers as f64 / r.versions as f64)
                    },
                }))
                .collect::<Vec<Value>>(),
        },
        "unmodelled_fields": {
            "window_notice_ids": UNMAPPED_FIELD_WINDOW_IDS,
            "listing_cap": UNMAPPED_FIELD_LISTING_CAP,
            "listing": report
                .unmapped_fields
                .iter()
                .map(|r| json!({
                    "profile": r.profile,
                    "field_id": r.field_id,
                    // Window rows, NOT a corpus total — see `window_notice_ids`.
                    "rows": r.hits,
                }))
                .collect::<Vec<Value>>(),
        },
        // THE field a machine consumer cannot do without, and it was missing. Without
        // it an absent or empty section is ambiguous between "measured, nothing found"
        // and "the query never ran" — opposite claims. The text render spends a branch
        // on exactly this distinction; the JSON had no way to express it.
        "unmeasured": report.unmeasured,
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
        // FTS is one era across OCDS versions (issue 342).
        assert_eq!(era_of("fts:ocds-1.1"), "FTS OCDS");
        assert_eq!(era_of("fts:ocds-1.2"), "FTS OCDS");
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

    /// Issue 109's alarm shape: a cohort going WHOLESALE stale between two runs
    /// fires; ordinary drift, small cohorts, and eras with no baseline stay
    /// quiet — the trigger is a step (≥ 20 points AND at least doubled, on
    /// ≥ 1,000 versions), not a floor, because per-era fact density
    /// legitimately varies and hand-tuned floors would be wrong on day one.
    #[test]
    fn the_step_change_alarm_fires_on_a_jump_and_stays_quiet_on_noise() {
        let row = |profile: &str, versions: u64, factless: u64| PresenceRow {
            profile: profile.to_owned(),
            versions,
            factless,
        };
        let previous = vec![
            ("stale-era".to_owned(), 100_000u64, 2_000u64),  // 2 %
            ("noisy-era".to_owned(), 100_000, 2_000),        // 2 %
            ("small-era".to_owned(), 500, 0),
        ];
        let presence = vec![
            row("stale-era", 100_000, 100_000), // 2 % → 100 %: the incident
            row("noisy-era", 100_000, 8_000),   // 2 % → 8 %: quadrupled but +6 points — drift
            row("small-era", 500, 500),         // wholesale stale but under the cohort floor
            row("new-era", 50_000, 50_000),     // no baseline — first sight, nothing to step from
        ];
        let alarms = presence_step_changes(&previous, &presence);
        assert_eq!(alarms.len(), 1, "exactly the wholesale-stale cohort: {alarms:?}");
        assert!(alarms[0].contains("stale-era") && alarms[0].contains("100.0%"), "{alarms:?}");

        // First run ever: no baseline at all, no alarms, by construction.
        assert!(presence_step_changes(&[], &presence).is_empty());
        // A run identical to its baseline: quiet.
        let steady: Vec<(String, u64, u64)> =
            presence.iter().map(|r| (r.profile.clone(), r.versions, r.factless)).collect();
        assert!(presence_step_changes(&steady, &presence).is_empty());
    }

    /// Issue 265: the headline entry carries every rate as [num, den] pulled
    /// from the section it belongs to, and the history stays bounded while a
    /// corrupt stored blob degrades to a fresh history rather than an error.
    #[test]
    fn the_headline_history_entry_carries_num_den_pairs_and_stays_bounded() {
        let report = Report {
            base_url: "x".into(),
            completeness: vec![CompletenessRow {
                profile: "eforms:eforms-sdk-1.13".into(),
                versions: 1_000,
                present: [900, 850, 700, 950, 800, 400],
            }],
            linkage: vec![LinkageRow {
                profile: "eforms:eforms-sdk-1.13".into(),
                awards: 200,
                unchained: 60,
            }],
            density: vec![DensityRow {
                profile: "eforms:eforms-sdk-1.13".into(),
                award_notices: 200,
                with_results: 199,
                no_award_content: 1,
                with_winner: 150,
                with_awardable: 170,
            }],
            invariant: vec![],
            doc_types: vec![],
            merge: Merge::default(),
            fresh_holds: vec![],
            amount_basis: vec![BasisRow {
                profile: "eforms:eforms-sdk-1.13".into(),
                amounts: 500,
                excl: 90,
                incl: 10,
            }],
            presence: vec![PresenceRow {
                profile: "eforms:eforms-sdk-1.13".into(),
                versions: 1_000,
                factless: 5,
            }],
            plausibility: vec![PlausibilityRow {
                profile: "eforms:eforms-sdk-1.13".into(),
                amounts: 500,
                negative: 2,
                zero: 9,
                over_1e12: 0,
                convertible: 480,
            }],
            longest_chain: 3_282,
            weld_candidates: vec![],
            weld_bands: vec![],
            sentinel_amounts: vec![],
            sentinel_dates: vec![],
            withheld_markers: vec![],
            unmapped_fields: Vec::new(),
            unmeasured: vec![],
        };
        let entry = headline_history_entry(&report, 1_700_000_000);
        assert_eq!(entry["at"], 1_700_000_000);
        assert_eq!(entry["longest_chain"], 3_282, "the fold-cost tripwire rides along (issue 92)");
        let era = &entry["eras"][0];
        assert_eq!(era["profile"], "eforms:eforms-sdk-1.13");
        assert_eq!(era["factless"], json!([5, 1_000]));
        assert_eq!(era["value"], json!([700, 1_000]), "value is FIELDS[2]");
        assert_eq!(era["named"], json!([150, 170]), "named divides by awardable (issue 258)");
        assert_eq!(era["linkage"], json!([140, 200]));
        assert_eq!(era["vat_stated"], json!([100, 500]));
        assert_eq!(era["negative"], json!([2, 500]));
        assert_eq!(era["eur_convertible"], json!([480, 500]), "ADR-0014's convertibility gauge");

        // Bounded: KEEP+3 appends leave exactly KEEP, newest last.
        let mut stored = "[]".to_owned();
        for i in 0..(HEADLINE_HISTORY_KEEP + 3) {
            stored = append_headline_history(&stored, json!({ "at": i }));
        }
        let history: Vec<serde_json::Value> = serde_json::from_str(&stored).unwrap();
        assert_eq!(history.len(), HEADLINE_HISTORY_KEEP);
        assert_eq!(history.last().unwrap()["at"], HEADLINE_HISTORY_KEEP + 2);

        // Corrupt storage degrades to a fresh history, never an error.
        let recovered = append_headline_history("not json{", json!({ "at": 7 }));
        let history: Vec<serde_json::Value> = serde_json::from_str(&recovered).unwrap();
        assert_eq!(history.len(), 1);
    }

    /// Every label present and empty — the shape [`Raw::from_labelled`] wants, so a test can
    /// fill in only the result set it is about and let the rest read as "ran, found nothing".
    fn sentinel_scaffold() -> Vec<(String, Option<Rows>)> {
        queries().into_iter().map(|(l, _)| (l, Some(Vec::new()))).collect()
    }

    fn put(results: &mut [(String, Option<Rows>)], label: &str, rows: Option<Rows>) {
        results.iter_mut().find(|(l, _)| l == label).expect("label").1 = rows;
    }

    /// Issue 366's discovery half. The standing sentinel list was derived by CONFIRMING four
    /// guesses, so the sweep that is meant to find the rest must not inherit the same blind
    /// spots: it reads the PUBLISHED figure, because `current_value_eur_cents` is converted
    /// and a PLN or HUF sentinel is smeared into a non-round EUR figure that cannot cluster
    /// at all; and it keeps negatives at every magnitude, because the floor exists to bound
    /// hash state rather than because small figures are innocent.
    #[test]
    fn the_amount_sweep_reads_published_cents_and_keeps_both_tails() {
        let sql = sentinel_amounts_sql();
        assert!(sql.contains("a.cents < 0"), "negatives at any size:\n{sql}");
        assert!(
            sql.contains(&format!("a.eur_cents >= {SENTINEL_AMOUNT_FLOOR}")),
            "the high tail is bounded by an EUR-EQUIVALENT floor — a currency-blind one spent \
             the listing cap on ordinary CZK and HUF contracts (job 816):\n{sql}"
        );
        assert!(
            sql.contains(&format!("a.eur_cents IS NULL AND a.cents >= {SENTINEL_AMOUNT_FLOOR}")),
            "an unconverted row falls back to the raw floor — ADR-0014 D4 leaves eur_cents NULL \
             when no rate resolves, and without this arm a whole era leaves scope silently:\n{sql}"
        );
        assert!(sql.contains("GROUP BY a.currency, a.cents"), "grouped per published pair:\n{sql}");
        assert!(
            sql.contains(&format!("HAVING COUNT(*) >= {SENTINEL_MIN_REPEATS}")),
            "singletons filtered in SQL, so the cap is spent on candidates:\n{sql}"
        );
        assert!(sql.contains(&format!("LIMIT {SENTINEL_LISTING_CAP}")), "{sql}");
        // The ban is on GROUPING by the converted column, not on referencing it. Reading
        // `eur_cents` as a THRESHOLD is what makes the floor currency-neutral (job 816);
        // grouping BY it is what reproduced the sweep that found nothing on 2026-09-08,
        // because conversion smears a sentinel off its round published figure so it
        // cannot cluster. An earlier form of this test banned the string outright and
        // failed the fix — the invariant is about the key, so it is stated about the key.
        assert!(
            sql.contains("GROUP BY a.currency, a.cents") && !sql.contains("GROUP BY a.eur_cents"),
            "the grouping key must be the PUBLISHED pair — a converted value cannot cluster:\n{sql}"
        );
    }

    /// The trap this test exists for, and it is silent: `strftime('%s','now')` returns TEXT,
    /// SQLite orders every number below every string, so `utc_seconds > strftime(…)` on its
    /// own is always-false and the sweep would report a clean corpus forever. The arithmetic
    /// forces numeric affinity and is load-bearing — `fresh_holds_sql` is correct by the same
    /// accident and this pins the reason.
    #[test]
    fn the_date_sweep_forces_numeric_affinity_on_its_horizon() {
        let sql = sentinel_dates_sql();
        assert!(
            sql.contains(&format!("strftime('%s','now') + {SENTINEL_DATE_HORIZON_SECS}")),
            "the `+` is what makes the comparison numeric rather than always-false:\n{sql}"
        );
        assert!(
            sql.contains(&format!("d.utc_seconds < {SENTINEL_DATE_FLOOR}")),
            "the pre-1990 tail is swept too — an epoch-zero default lands there:\n{sql}"
        );
        // Measured 2026-09-08: 2099-12-31 carries 14 tenders spread over several times of
        // day, and its largest single SECOND holds 4. Grouping by instant with any useful
        // threshold returns nothing from the strongest cluster in the corpus — a detector
        // that is silent exactly where it matters.
        assert!(
            sql.contains("GROUP BY d.field, date(d.utc_seconds, 'unixepoch')"),
            "dates group by DAY, never by instant:\n{sql}"
        );
        assert!(
            !sql.contains("GROUP BY d.field, d.utc_seconds"),
            "the exact-instant grouping is the silent-detector bug:\n{sql}"
        );
        assert!(
            sql.contains(&format!("HAVING COUNT(*) >= {SENTINEL_DATE_MIN_REPEATS}")),
            "dates use their own, lower threshold — the whole far tail is ~100 tenders:\n{sql}"
        );
    }

    /// Both sweeps must stay OUT of the windowed catalog. `HAVING COUNT(*) >= n` cannot be
    /// Issue 364 unit 3: the weld detector must name BOTH buyer vocabularies.
    ///
    /// This is the whole reason the unit exists in the shape it does. The issue's
    /// recorded calibration said `role='Procedure-Buyer'` only, derived from an
    /// eForms-only sample — and tender 2816628, the 2,983-version 127-buyer weld
    /// the issue was FILED about, carries 2,983 `buyer` rows and zero
    /// `Procedure-Buyer`. A one-vocabulary gauge reports nought for it and reads
    /// green, which is the worst failure available to a detector.
    #[test]
    fn the_weld_detector_counts_both_buyer_vocabularies() {
        for sql in [weld_candidates_sql(), weld_bands_sql()] {
            assert!(sql.contains("'buyer'"), "the legacy role must be counted: {sql}");
            assert!(sql.contains("'Procedure-Buyer'"), "the eForms role must be counted: {sql}");
            // Not a role-blind count: Tenderer and the review bodies would swamp
            // it (tender 2816628 carries 1,986 orgs under AWARD_AND_CONTRACT_VALUE
            // against its 127 buyers), which is what the calibration got RIGHT.
            assert!(sql.contains("p.role IN"), "roles must be restricted: {sql}");
            assert!(!sql.contains("Tenderer"), "{sql}");
        }
    }

    /// The source-reading guard proves a field is MENTIONED; this proves the JSON
    /// consumer can actually act on it. `unmeasured` is the one that matters: without
    /// it an empty listing is ambiguous between "measured, nothing found" and "the
    /// query never ran", which are opposite claims about the corpus.
    #[test]
    fn the_json_says_which_queries_did_not_run() {
        let mut ran = sentinel_scaffold();
        put(&mut ran, "weld_candidates", None);
        let report = assemble("x", &Raw::from_labelled(ran).expect("raw"));
        let v: serde_json::Value =
            serde_json::from_str(&render_json(&report)).expect("valid JSON");

        let unmeasured = v["unmeasured"].as_array().expect("unmeasured array");
        assert!(
            unmeasured.iter().any(|l| l == "weld_candidates"),
            "a query that did not run must be named in the JSON: {v:#}"
        );
        // And the section is still present rather than absent, so a consumer reads an
        // empty listing beside the reason it is empty, not a missing key.
        assert!(v["weld_candidates"]["listing"].is_array(), "{v:#}");
    }

    /// The weld listing's JSON must carry the discriminator precomputed, for the same
    /// reason `residue` is precomputed beside the withheld markers: two consumers
    /// deriving it separately is how they come to disagree about it.
    #[test]
    fn the_weld_json_precomputes_the_per_version_discriminator() {
        let mut ran = sentinel_scaffold();
        put(
            &mut ran,
            "weld_candidates",
            Some(vec![
                vec![json!(331_647), json!(505), json!(1)],
                vec![json!(2_816_628), json!(127), json!(2_983)],
            ]),
        );
        put(&mut ran, "weld_bands", Some(vec![vec![json!(2), json!(2), json!(2), json!(2)]]));
        let report = assemble("x", &Raw::from_labelled(ran).expect("raw"));
        let v: serde_json::Value =
            serde_json::from_str(&render_json(&report)).expect("valid JSON");
        let listing = v["weld_candidates"]["listing"].as_array().expect("listing");

        assert_eq!(listing[0]["per_version"].as_f64().expect("f64"), 505.0);
        let weld = listing[1]["per_version"].as_f64().expect("f64");
        assert!(weld < 0.1, "the weld's ratio must stay small, got {weld}");
        // The bands pair each threshold with its count, so an array cannot be read
        // against the wrong WELD_BANDS if the constant ever moves.
        let bands = v["weld_candidates"]["bands"].as_array().expect("bands");
        assert_eq!(bands.len(), WELD_BANDS.len());
        assert_eq!(bands[0]["at_least"], json!(WELD_BANDS[0]));
    }

    /// The guard that would have caught issue 368 unit 4b's inertia, generalised.
    ///
    /// `unmapped_fields` was registered, ran every week, and was assembled into
    /// `Report` — and `render_text` never mentioned it. The query paid its scan and
    /// produced nothing a reader could see, for as long as it took someone to walk
    /// the struct by hand. Reading the SOURCE is the only way to state the property
    /// "every measured field reaches the renderer", because a field that renders
    /// nothing is invisible to any assertion about output.
    ///
    /// Same shape as the head-column writer-count test in `store`, and for the same
    /// reason: the invariant is about the code, so the code is what it reads.
    #[test]
    fn every_report_field_is_read_by_the_renderer() {
        let src = include_str!("data_quality.rs");
        // Past the opening brace, so the declaration line itself is not read as a field.
        let start =
            src.find("pub struct Report {").expect("Report struct") + "pub struct Report {".len();
        let body = &src[start..];
        let fields: Vec<&str> = body[..body.find("\n}").expect("struct end")]
            .lines()
            .filter_map(|l| l.trim().strip_prefix("pub "))
            .filter_map(|l| l.split(':').next())
            .collect();
        assert!(fields.len() > 10, "parsed too few fields: {fields:?}");

        // BOTH renderers. The JSON one had four fields missing when this test was
        // written — including `unmeasured`, without which a machine consumer cannot
        // tell "measured, nothing found" from "the query never ran".
        for renderer in ["pub fn render_text(", "pub fn render_json("] {
            let r = src.find(renderer).expect(renderer);
            let render = &src[r..src[r..].find("\n}\n").expect("render end") + r];
            let missing: Vec<&&str> = fields.iter().filter(|f| !render.contains(**f)).collect();
            assert!(
                missing.is_empty(),
                "{renderer} never reads these Report fields, so their queries cost their scan \
                 every week and show that consumer nothing: {missing:?}"
            );
        }
    }

    /// Sections must print in their numbered order. They did not: adding section 12
    /// beside the query it belonged to put it AHEAD of section 11 in the output, so
    /// the first real report read 10, 12, 11. Harmless to a machine, confusing to a
    /// person, and free to hold.
    #[test]
    fn the_sections_render_in_numbered_order() {
        let text = render_text(&assemble("x", &Raw::from_labelled(sentinel_scaffold()).expect("raw")));
        let numbers: Vec<u32> = text
            .lines()
            .filter_map(|l| l.trim().strip_prefix("== "))
            .filter_map(|l| l.split('.').next())
            .filter_map(|n| n.parse().ok())
            .collect();
        assert!(numbers.len() >= 10, "too few sections parsed: {numbers:?}");
        let mut sorted = numbers.clone();
        sorted.sort_unstable();
        assert_eq!(numbers, sorted, "sections must print in order, got {numbers:?}");
        // And contiguous from 1, so a section cannot be dropped without notice.
        assert_eq!(
            numbers,
            (1..=numbers.len() as u32).collect::<Vec<_>>(),
            "section numbers must run 1..N with no gaps: {numbers:?}"
        );
    }

    /// Issue 368: the section must not read as a defect list.
    ///
    /// Every one of its top entries was hand-read and found correctly out of scope —
    /// the canonical model is a narrow subset on purpose. A section that presents
    /// them as silent drops asks a reader to act on fifteen things that are working
    /// as intended, which is how a diagnostic earns being ignored. It also is not the
    /// detector for this issue's own failures, since `any_channel_reads` is
    /// profile-blind and those failures are per-profile asymmetries.
    #[test]
    fn the_unmodelled_field_section_is_scope_not_defect() {
        let mut ran = sentinel_scaffold();
        put(
            &mut ran,
            "unmapped_fields",
            Some(vec![vec![json!("eforms:eforms-sdk-1.13"), json!("BT-67(a)-Procedure"), json!(63_814)]]),
        );
        let text = render_text(&assemble("x", &Raw::from_labelled(ran).expect("raw")));
        assert!(text.contains("NOT a defect"), "the section must say so outright:\n{text}");
        assert!(
            text.contains("profile-BLIND"),
            "it must say why it cannot detect this issue's own failures:\n{text}"
        );
        assert!(
            text.contains("window count and NOT a corpus"),
            "the window bound must stay attached to the number:\n{text}"
        );
    }

    /// Issue 364, from the first live run: the section must not tell a reader that any
    /// buyer count is a safe weld verdict.
    ///
    /// It used to. "No joint procurement has fifty buyers" shipped in the render, and
    /// the first report it produced listed tender 331647 with 505 buyers on ONE version
    /// under the title `Skupno javno naročilo` — Slovenian for joint public procurement.
    /// This pins the correction so the assertion cannot creep back as a tidier sentence.
    #[test]
    fn the_weld_section_claims_no_safe_threshold() {
        let mut ran = sentinel_scaffold();
        put(
            &mut ran,
            "weld_candidates",
            Some(vec![vec![json!(331_647), json!(505), json!(1)]]),
        );
        put(&mut ran, "weld_bands", Some(vec![vec![json!(2), json!(1), json!(1), json!(1)]]));
        let text = render_text(&assemble("x", &Raw::from_labelled(ran).expect("raw")));
        assert!(
            !text.contains("no joint procurement has fifty buyers"),
            "the refuted claim must not return:\n{text}"
        );
        assert!(
            text.contains("does NOT tell them apart"),
            "the section must say the count alone cannot decide:\n{text}"
        );
        // And the discriminator must be rendered, not merely described: 505 over one
        // version is 505.0, which is what marks it as joint rather than welded.
        assert!(text.contains("505.0"), "the per-version hint must render:\n{text}");
        assert!(text.contains("per-ver"), "the column must be labelled:\n{text}");
    }

    /// The lowest band and the query's floor are ONE number wearing two names, and
    /// nothing else makes them agree.
    ///
    /// `weld_bands_sql` sums `buyers >= n` over a subquery that has already applied
    /// `HAVING COUNT(...) >= WELD_MIN_BUYERS`. So if the floor were raised to 5 while
    /// `WELD_BANDS` still opened at 3, the ">= 3" column would report the >= 5 count
    /// and read as a fall in welds — a silent wrong number in the one direction that
    /// looks like good news. The render labels the column from `WELD_BANDS`, so
    /// nothing downstream could catch it either.
    #[test]
    fn the_lowest_band_is_the_querys_floor() {
        assert_eq!(
            WELD_BANDS[0], WELD_MIN_BUYERS,
            "the first band must be the HAVING floor, or its column reports a different \
             population than its label claims"
        );
        // And the bands ascend, so each column is a strict subset of the one before —
        // the reading "3,000 at >= 3 of which 40 at >= 50" depends on it.
        for pair in WELD_BANDS.windows(2) {
            assert!(pair[0] < pair[1], "bands must ascend: {WELD_BANDS:?}");
        }
    }

    /// The bands and the listing must come from the SAME predicate, or the
    /// summary line describes a different population from the rows under it.
    #[test]
    fn the_weld_bands_and_listing_share_their_predicate() {
        let listing = weld_candidates_sql();
        let bands = weld_bands_sql();
        for fragment in [
            "p.role IN ('buyer', 'Procedure-Buyer')",
            "COUNT(DISTINCT p.organization_id)",
            "GROUP BY p.tender_id",
        ] {
            assert!(listing.contains(fragment), "listing missing {fragment}: {listing}");
            assert!(bands.contains(fragment), "bands missing {fragment}: {bands}");
        }
        // The listing is capped; the bands are not, which is the point of having
        // both — a full listing must never be read as the population's size.
        assert!(listing.contains(&format!("LIMIT {WELD_LISTING_CAP}")), "{listing}");
        assert!(!bands.contains("LIMIT"), "the bands must count the whole population: {bands}");
    }

    #[test]
    fn the_weld_detector_is_whole_corpus() {
        let whole: Vec<String> = whole_corpus_queries().into_iter().map(|(l, _)| l).collect();
        assert!(whole.iter().any(|l| l == "weld_candidates"), "{whole:?}");
        assert!(whole.iter().any(|l| l == "weld_bands"), "{whole:?}");
        for q in windowed_queries() {
            assert!(!q.label.starts_with("weld"), "{} must not be windowed", q.label);
        }
    }

    /// A detector that renders "0" when it did not run is worse than one that
    /// fails loudly: the two claims are opposite and look identical.
    #[test]
    fn an_unmeasured_weld_query_renders_unmeasured_not_zero() {
        let mut ran = sentinel_scaffold();
        put(&mut ran, "weld_candidates", None);
        let text = render_text(&assemble("x", &Raw::from_labelled(ran).expect("raw")));
        assert!(text.contains("UNMEASURED — the weld query did not run"), "{text}");
        assert!(
            !text.contains(">= 3: 0"),
            "an unmeasured detector must not print a zero population: {text}"
        );
    }

    /// windowed: a value repeating nine times in each of 25 windows passes the corpus test
    /// and fails every window's, so a windowed form would under-report exactly the values the
    /// sweep exists to find — and it would do it quietly, as a shorter listing.
    #[test]
    fn the_sentinel_sweeps_are_whole_corpus_because_a_having_cannot_be_windowed() {
        let whole: Vec<String> = whole_corpus_queries().into_iter().map(|(l, _)| l).collect();
        assert!(whole.iter().any(|l| l == "sentinel_amounts"), "{whole:?}");
        assert!(whole.iter().any(|l| l == "sentinel_dates"), "{whole:?}");
        for q in windowed_queries() {
            assert!(
                !q.label.starts_with("sentinel"),
                "{} must not be windowed — its HAVING would be evaluated per window",
                q.label
            );
        }
    }

    /// [`as_u64`] clamps negatives to zero, which would erase the largest sentinel class in
    /// the corpus (-1.00, 15,529 tenders) on its way through assembly and render it as a
    /// harmless `0.00`. The signed reader is the fix; this pins it end to end.
    #[test]
    fn a_negative_amount_keeps_its_sign_through_assembly_and_render() {
        let mut ran = sentinel_scaffold();
        put(
            &mut ran,
            "sentinel_amounts",
            Some(vec![vec![json!("EUR"), json!(-100), json!(15_650), json!(15_529)]]),
        );
        let report = assemble("x", &Raw::from_labelled(ran).expect("raw"));
        assert_eq!(report.sentinel_amounts[0].raw, -100, "the sign survives assembly");
        let text = render_text(&report);
        assert!(text.contains("-1.00"), "and renders as a negative:\n{text}");
        assert!(text.contains("15,529"), "with its Tender count beside it:\n{text}");
    }

    /// The section is a CANDIDATE listing, and two things make it usable: the per-Tender count
    /// beside the row count (one Tender revised 90 times is not 9,000 Tenders sharing a
    /// placeholder — they want different fixes), and an honest statement when the listing is
    /// full, so a truncated tail is never read as the whole of one.
    #[test]
    fn section_10_ranks_the_repeats_and_admits_when_the_listing_is_full() {
        let mut ran = sentinel_scaffold();
        // 3005-07-06 — the date issue 366 was handed rather than found.
        put(
            &mut ran,
            "sentinel_dates",
            Some(vec![vec![json!("BT-131"), json!(32_677_516_800_i64), json!(4_120), json!(4_010)]]),
        );
        let text = render_text(&assemble("x", &Raw::from_labelled(ran).expect("raw")));
        assert!(text.contains("== 10. Repeated implausible values"), "{text}");
        assert!(text.contains("3005-07-06"), "the raw instant renders as a day:\n{text}");
        assert!(text.contains("4,010"), "tenders counted beside rows:\n{text}");
        assert!(!text.contains("LISTING FULL"), "one row is not a full listing:\n{text}");

        let full: Rows = (0..SENTINEL_LISTING_CAP)
            .map(|i| {
                vec![json!("EUR"), json!(SENTINEL_AMOUNT_FLOOR + i as i64), json!(11), json!(11)]
            })
            .collect();
        let mut ran = sentinel_scaffold();
        put(&mut ran, "sentinel_amounts", Some(full));
        let text = render_text(&assemble("x", &Raw::from_labelled(ran).expect("raw")));
        assert!(text.contains("LISTING FULL"), "a truncated tail says so:\n{text}");
    }

    /// Section 11 carries FOUR readings of one population and they mean different
    /// things: `rows` is every `-1.00`, `in-wh-notice` is notice-wide ("this notice
    /// withheld something"), `marked` is unit 2's exact per-row verdict, and
    /// `residue` is the undeclared remainder. The gap that matters is
    /// `in-wh-notice` minus `marked` — a withholding block the publisher hoisted
    /// away from the value it suppresses, which the section-anchored rule cannot
    /// reach. This pins that the two counts stay SEPARATE columns, because
    /// collapsing them would hide exactly the risk the marker was shipped with.
    #[test]
    fn section_11_keeps_the_notice_wide_count_apart_from_the_per_row_verdict() {
        let mut ran = sentinel_scaffold();
        put(
            &mut ran,
            "withheld_markers",
            Some(vec![
                // 100 rows, 90 in a withholding notice, only 80 marked: 10 hoisted
                // blocks the rule did not reach, and 10 undeclared as residue.
                vec![json!("result_value · ted"), json!(100), json!(90), json!(80), json!(97)],
            ]),
        );
        let report = assemble("x", &Raw::from_labelled(ran).expect("raw"));
        let text = render_text(&report);
        assert!(text.contains("== 11. Withheld-marker amounts"), "{text}");
        assert!(text.contains("marked"), "the per-row verdict has its own column:\n{text}");

        let row = &report.withheld_markers[0];
        assert_eq!(row.hits, 100);
        assert_eq!(row.in_withholding_notice, 90);
        assert_eq!(row.marked, 80, "column 4 is the marker, not the tender count");
        assert_eq!(row.tenders, 97);

        let json_text = render_json(&report);
        let v: Value = serde_json::from_str(&json_text).expect("valid json");
        let out = &v["withheld_markers"][0];
        assert_eq!(out["rows"], json!(100));
        assert_eq!(out["in_withholding_notice"], json!(90));
        assert_eq!(out["marked"], json!(80));
        assert_eq!(out["residue"], json!(10), "residue is rows - in_withholding_notice");
        assert_eq!(out["tenders"], json!(97));
    }

    /// Issue 230's distinction, at this section: "the query did not run" and "nothing repeats"
    /// are different claims. A failed sweep that rendered as a clean one would be the worst
    /// possible failure for a detector — silence meaning the opposite of what it reads as.
    #[test]
    fn an_unmeasured_sweep_does_not_read_as_a_clean_sweep() {
        let mut ran = sentinel_scaffold();
        put(&mut ran, "sentinel_amounts", None);
        let text = render_text(&assemble("x", &Raw::from_labelled(ran).expect("raw")));
        assert!(text.contains("UNMEASURED — the `sentinel_amounts` query did not run"), "{text}");
        // The date half DID run and found nothing, which must still read as none.
        assert!(text.contains("none — nothing in this tail repeats"), "{text}");
    }

    /// The two formatters the section reads through, at their edges: the sign, the sub-unit,
    /// and the epoch — where `day_utc` would print `—` for a real 1970-01-01 sentinel.
    #[test]
    fn the_sentinel_formatters_render_both_tails() {
        assert_eq!(major(-100), "-1.00");
        assert_eq!(major(-1), "-0.01");
        assert_eq!(major(0), "0.00");
        assert_eq!(major(999_999_999_900), "9,999,999,999.00");
        assert_eq!(day_utc_signed(0), "1970-01-01", "the epoch is a date here, not `none`");
        assert_eq!(day_utc_signed(-86_400), "1969-12-31");
        assert_eq!(day_utc_signed(32_677_516_800), "3005-07-06");
    }

    /// The machine report must carry the thresholds, not just the rows. A listing read six
    /// months from now has to say what region it covered — a later run with a different floor
    /// measures a different population and the two are not comparable without them.
    #[test]
    fn the_json_sentinel_section_carries_its_thresholds_and_both_readings() {
        let mut ran = sentinel_scaffold();
        put(
            &mut ran,
            "sentinel_amounts",
            Some(vec![vec![json!("PLN"), json!(-100), json!(90), json!(12)]]),
        );
        let json_text = render_json(&assemble("x", &Raw::from_labelled(ran).expect("raw")));
        let v: Value = serde_json::from_str(&json_text).expect("valid json");
        let sect = &v["repeated_implausible"];
        assert_eq!(sect["min_repeats"], SENTINEL_MIN_REPEATS);
        assert_eq!(sect["amount_floor_cents"], SENTINEL_AMOUNT_FLOOR);
        assert_eq!(sect["date_floor"], SENTINEL_DATE_FLOOR);
        let row = &sect["amounts"][0];
        assert_eq!(row["scope"], "PLN");
        assert_eq!(row["raw"], -100, "the raw column a follow-up query can use");
        assert_eq!(row["shown"], "-1.00", "and the reading a human recognises");
        assert_eq!(row["tenders"], 12);
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
                // Content presence — the shell detector (issue 109).
                "factless",
                // Amount plausibility (issue 267).
                "amount_plausibility",
                "fresh_holds",
                // The fold-cost tripwire (issue 92).
                "longest_chain",
                // The sentinel discovery sweep (issue 366).
                "sentinel_amounts",
                "sentinel_dates",
                // The withheld-marker residue (issue 372).
                "withheld_markers",
                // Issue 364 unit 3: the weld detector, registered beside the
                // other whole-corpus sweeps it shares its phase with.
                "weld_candidates",
                "weld_bands",
                "unmapped_fields",
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
                    // No winner among them, and every result awardable — so `named` is a
                    // real 0.0 % rather than the em-dash an absent denominator would give,
                    // which is what this row is here to keep out of the render.
                    json!(0),
                    json!(139_961),
                ]]),
            ),
            ("doc_types".to_owned(), Some(vec![])),
            ("sections_can".to_owned(), Some(vec![])),
            ("sections_with".to_owned(), Some(vec![])),
            ("merge".to_owned(), Some(vec![vec![json!("all"), json!(0), json!(0)]])),
            ("amount_basis".to_owned(), Some(vec![])),
            ("factless".to_owned(), Some(vec![])),
            ("amount_plausibility".to_owned(), Some(vec![])),
            ("fresh_holds".to_owned(), Some(vec![])),
            ("longest_chain".to_owned(), Some(vec![])),
            ("sentinel_amounts".to_owned(), Some(vec![])),
            ("sentinel_dates".to_owned(), Some(vec![])),
            ("withheld_markers".to_owned(), Some(vec![])),
            ("weld_candidates".to_owned(), Some(vec![])),
            ("weld_bands".to_owned(), Some(vec![])),
            ("unmapped_fields".to_owned(), Some(vec![])),
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
    /// Issue 258: `named` divides by the results a winner could be expected FOR.
    ///
    /// Two populations must not count against the winner chain. A result the publisher
    /// CLOSED with nobody (`no-rece`, `clos-nw`, `open-nw` and no winner) never had a
    /// winner to find. And — the trap that falsified this issue's first sketch — an
    /// UNSTATED decision is not a denial: eForms-DE 1.x publishes no `TenderResultCode`
    /// at all and DOES name winners (issue 100), sdk-0.1 the same (issue 257). A
    /// denominator defined as "carries a decision that expects a winner" would have put
    /// those in the numerator and out of the denominator, and read above 100 %.
    #[test]
    fn the_named_rate_divides_by_the_results_a_winner_was_possible_for() {
        let row = |era: &str, awards, results, barren, winner, awardable| {
            vec![json!(era), json!(awards), json!(results), json!(barren), json!(winner), json!(awardable)]
        };
        let mut rows: Vec<(String, Option<Rows>)> =
            queries().into_iter().map(|(l, _)| (l, Some(Vec::new()))).collect();
        rows.iter_mut().find(|(l, _)| l == "awards").expect("label").1 = Some(vec![
            // 1,000 results, 600 of which the publisher closed naming nobody. 320 of the
            // remaining 400 name a winner: 80 %, not the 32 % the old denominator read.
            row("text", 1_000, 1_000, 0, 320, 400),
            // Every result awardable and every one named — the ceiling still reads 100 %.
            row("eforms:eforms-sdk-1.13", 500, 500, 0, 500, 500),
        ]);
        let report = assemble("(t)", &Raw::from_labelled(rows).expect("labelled"));
        let text = render_text(&report);

        assert!(text.contains("closed n/a"), "the excluded population must be headed: {text}");
        assert!(text.contains("80.0%"), "320 of 400 awardable results name a winner: {text}");
        assert!(
            !text.contains("32.0%"),
            "dividing by every materialised result blames the winner chain for the \
             publisher's own closures: {text}"
        );
        assert!(text.contains("100.0%"), "and a fully-named era still reads 100 %: {text}");

        // The invariant the whole shape exists for: the rate cannot exceed 1, because a
        // result with a winner is in the denominator by construction. A rate above 1 is
        // not a number, it is a bug report.
        let json: Value = serde_json::from_str(&render_json(&report)).expect("valid json");
        for r in json["results_density"].as_array().expect("rows") {
            let (w, a) = (r["with_winner"].as_u64().expect("n"), r["with_awardable"].as_u64().expect("n"));
            assert!(w <= a, "winners must be a subset of awardable results: {r}");
            assert!(r["winner_rate"].as_f64().expect("rate") <= 1.0, "{r}");
        }
        let text_row = json["results_density"]
            .as_array()
            .expect("rows")
            .iter()
            .find(|r| r["profile"] == "text")
            .expect("row");
        assert_eq!(text_row["closed_no_winner"], json!(600), "published, not left to subtraction");
        assert_eq!(text_row["winner_rate"], json!(0.8));
    }

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

    /// Issue 101: a result block with nobody in it. Reading that used to take two
    /// sections and an inference — section 1's `winner` is a share of ALL versions, so
    /// eForms-DE 1.1's 1.4 % only becomes a gap once you know 41 % of its versions are
    /// awards. Beside the density it is one line, and it is a rate against the notices
    /// that DID materialise a result, so a missing result block is not counted twice.
    #[test]
    fn a_result_block_with_no_winner_is_its_own_column() {
        let mut rows: Vec<(String, Option<Rows>)> =
            queries().into_iter().map(|(l, _)| (l, Some(Vec::new()))).collect();
        rows.iter_mut().find(|(l, _)| l == "awards").expect("label").1 = Some(vec![
            // The DE-1.x shape: every award notice materialises a result, almost none
            // names a winner (issue 100).
            vec![json!("eforms:eforms-de-1.1"), json!(1_000), json!(1_000), json!(0), json!(34), json!(1_000)],
            // …against an era whose chain works.
            vec![json!("eforms:eforms-de-2.1"), json!(1_000), json!(1_000), json!(0), json!(800), json!(1_000)],
        ]);
        let report = assemble("(t)", &Raw::from_labelled(rows).expect("labelled"));

        let de11 = report.density.iter().find(|r| r.profile == "eforms:eforms-de-1.1").expect("row");
        assert_eq!((de11.with_results, de11.with_winner), (1_000, 34));

        let text = render_text(&report);
        assert!(text.contains("with winner"), "the column must be headed: {text}");
        assert!(text.contains("3.4%"), "34 of 1,000 materialised results name a winner: {text}");
        assert!(text.contains("80.0%"), "and the working era reads 80 %: {text}");

        // The rate divides by the materialised results, NOT by the award notices — a
        // notice with no result block is already counted one column left, and dividing by
        // it twice would blame the winner chain for a missing block.
        let mut half: Vec<(String, Option<Rows>)> =
            queries().into_iter().map(|(l, _)| (l, Some(Vec::new()))).collect();
        half.iter_mut().find(|(l, _)| l == "awards").expect("label").1 =
            Some(vec![vec![json!("text"), json!(1_000), json!(500), json!(500), json!(250), json!(500)]]);
        let text = render_text(&assemble("(t)", &Raw::from_labelled(half).expect("labelled")));
        assert!(
            text.contains("50.0%"),
            "250 winners against 500 materialised results is 50 %, not 25 %: {text}"
        );

        // And the JSON carries the count and the rate, so a consumer never recomputes it.
        let json: Value = serde_json::from_str(&render_json(&report)).expect("valid json");
        let de11 = json["results_density"]
            .as_array()
            .expect("rows")
            .iter()
            .find(|r| r["profile"] == "eforms:eforms-de-1.1")
            .expect("row")
            .clone();
        assert_eq!(de11["with_winner"], json!(34));
        assert_eq!(de11["winner_rate"], json!(0.034));
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
            ("factless".to_owned(), Some(vec![vec![json!("eforms:eforms-sdk-1.13"), json!(2)]])),
            (
                "amount_plausibility".to_owned(),
                Some(vec![vec![
                    json!("eforms:eforms-sdk-1.13"),
                    json!(120),
                    json!(3),
                    json!(7),
                    json!(1),
                    json!(96),
                ]]),
            ),
            // Issue 92: the single-cell whole-corpus MAX, as the runner delivers it.
            ("longest_chain".to_owned(), Some(vec![vec![json!(3_282)]])),
            // Issue 366: the two sentinel sweeps, empty — a corpus with nothing repeating in
            // either implausible tail, which is the state this report hopes to describe.
            ("sentinel_amounts".to_owned(), Some(vec![])),
            ("sentinel_dates".to_owned(), Some(vec![])),
            ("withheld_markers".to_owned(), Some(vec![])),
            // Issue 364 unit 3: one weld candidate and the bands behind it. The
            // listing is a subset of the >= 3 band by construction (the cap), so a
            // one-row listing under a band count of 2 is the ordinary shape, not a
            // contradiction.
            ("weld_candidates".to_owned(), Some(vec![vec![json!(2_816_628), json!(127), json!(2_983)]])),
            ("weld_bands".to_owned(), Some(vec![vec![json!(2), json!(1), json!(1), json!(1)]])),
            ("unmapped_fields".to_owned(), Some(vec![])),
        ];
        let raw = Raw::from_labelled(results).expect("labelled");
        let report = assemble("http://x", &raw);

        // Issue 267: the plausibility row assembles and renders with its caveat.
        assert_eq!(
            report.plausibility,
            vec![PlausibilityRow {
                profile: "eforms:eforms-sdk-1.13".into(),
                amounts: 120,
                negative: 3,
                zero: 7,
                over_1e12: 1,
                convertible: 96,
            }]
        );
        let text = render_text(&report);
        assert!(text.contains("== 8. Amount plausibility"), "section 8 must render:\n{text}");
        assert!(
            text.contains("RATE moving between runs is the signal"),
            "the source-published caveat must render — a rate table without it manufactures \
             a defect out of publisher behaviour:\n{text}"
        );
        assert!(
            text.contains("80.0%"),
            "the eur-conv column renders 96/120 as a rate:\n{text}"
        );

        // Issue 92: the tripwire line renders un-flagged below the threshold and
        // FLAGS at it — the whole point is that the approach is visible, so both
        // sides of the threshold are pinned here.
        assert_eq!(report.longest_chain, 3_282);
        assert!(
            text.contains("longest chain: 3,282") && !text.contains("FLAG: >= 4,000"),
            "3,282 is on the clock but below the flag threshold:\n{text}"
        );
        let flagged = render_text(&Report { longest_chain: 4_000, ..report.clone() });
        assert!(
            flagged.contains("FLAG: >= 4,000"),
            "at the threshold the line must flag, not murmur:\n{flagged}"
        );

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
    /// Issue 368 unit 4b: the diagnostic lists only field ids the projection
    /// actually DROPS, and reads its window from the head of the corpus.
    ///
    /// The filter runs in Rust, not SQL, because the projection matches on full
    /// id OR two-segment stem and resolves DE-1.x aliases first — so the SQL
    /// deliberately returns everything and the test's job is to prove the sieve.
    #[test]
    fn the_unmapped_field_listing_keeps_only_ids_nothing_reads() {
        // A field the projection reads, one it reads only via its stem, and two
        // it has no destination for at all.
        let read = "BT-501-Organization-Company";
        let dropped = "BT-99999-Invented-Field";
        assert!(
            crate::project::any_channel_reads(read),
            "fixture check: {read} must be a field the projection reads, or this \
             test proves nothing about the filter"
        );
        assert!(!crate::project::any_channel_reads(dropped), "fixture check: {dropped}");

        let row = |field: &str, hits: u64| {
            vec![
                Value::String("eforms:eforms-sdk-1.13".to_owned()),
                Value::String(field.to_owned()),
                Value::from(hits),
            ]
        };
        let raw: Rows = vec![row(read, 9_000), row(dropped, 42)];
        let listed: Vec<UnmappedFieldRow> = raw
            .iter()
            .map(|r| UnmappedFieldRow {
                profile: as_str(r.first()),
                field_id: as_str(r.get(1)),
                hits: as_u64(r.get(2)),
            })
            .filter(|row| !crate::project::any_channel_reads(&row.field_id))
            .take(UNMAPPED_FIELD_LISTING_CAP)
            .collect();

        assert_eq!(listed.len(), 1, "the read field must not be listed: {listed:?}");
        assert_eq!(listed[0].field_id, dropped);
        assert_eq!(listed[0].hits, 42);
        assert_eq!(listed[0].profile, "eforms:eforms-sdk-1.13");
    }

    /// The window is in the SQL, and it is the HEAD of the corpus — the property
    /// the cost decision rests on (a whole-corpus form is ~300M rows over eight
    /// GROUP BYs). A refactor that dropped the bound would still return correct
    /// rows, just slowly and against the issue-278 hash-state hazard, so it is
    /// worth pinning rather than trusting.
    #[test]
    fn the_unmapped_field_query_is_bounded_to_the_newest_notices() {
        let sql = unmapped_fields_sql();
        assert!(
            sql.contains("(SELECT MAX(id) FROM notices) -"),
            "the notice-id window must survive: {sql}"
        );
        assert_eq!(
            sql.matches("JOIN notices n ON n.id = x.notice_id").count(),
            8,
            "all eight parsed value tables ride the union"
        );
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
            assert!(sql.contains(table), "{table} missing from the union");
        }
    }

    fn from_labelled_reports_a_missing_result_set() {
        let err = Raw::from_labelled(vec![("versions".to_owned(), Some(vec![]))]).unwrap_err();
        assert!(err.contains("title"), "{err}");
    }
}
