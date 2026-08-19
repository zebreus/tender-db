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

/// Results materialisation per era, DENOMINATOR: versions whose notice's own
/// published document type says it announces a result (issue 235).
///
/// Read off `notice_codes` — the parse layer's record of what the publisher said
/// — never off `notice_sections`, whose result sections are the very thing the
/// numerator checks the projection produced. That independence is the whole
/// point: this denominator counts an award notice that parsed with zero result
/// sections, which the old one could not.
pub fn awards_can_sql() -> String {
    awards_can_template("")
}

/// [`awards_can_sql`] with `win` spliced into its `WHERE` — one builder for both
/// the catalog and the windowed form, so the two cannot drift into measuring
/// different populations (the drift issue 230 hit when it spliced by text).
fn awards_can_template(win: &str) -> String {
    format!(
        "SELECT n.profile, COUNT(*) AS award_notices \
           FROM tender_versions tv JOIN notices n ON n.id = tv.caused_by_notice_id \
          WHERE {win}({award}) \
          GROUP BY n.profile",
        award = award_predicate(),
    )
}

/// Results materialisation per era, NUMERATOR: award-typed versions whose notice
/// actually produced a canonical `lot_results` row.
///
/// The award predicate is repeated verbatim from [`awards_can_sql`] rather than
/// joined against it, so the numerator is a subset of the denominator by
/// construction and the ratio cannot exceed 1 — the `IMPOSSIBLE` state the old
/// pair could reach (it counted versions below the line and notices above it).
pub fn awards_with_sql() -> String {
    awards_with_template("")
}

fn awards_with_template(win: &str) -> String {
    format!(
        "SELECT n.profile, COUNT(*) AS with_results \
           FROM tender_versions tv JOIN notices n ON n.id = tv.caused_by_notice_id \
          WHERE {win}({award}) \
            AND EXISTS(SELECT 1 FROM lot_results lr \
                        WHERE lr.tender_id = tv.tender_id AND lr.notice_id = tv.caused_by_notice_id) \
          GROUP BY n.profile",
        award = award_predicate(),
    )
}

/// Section 3's third number, EXPLANATORY: award-typed versions whose notice parsed
/// with no result block at all (issue 242).
///
/// A rate needs this to be readable. An award notice can announce a result and
/// publish no machine-readable award content whatsoever, and then no projection
/// can materialise it — the gap is upstream, in what was published. Two measured
/// shapes in r2.0.8, which together are its ENTIRE shortfall:
///
/// - the whole body is `OTH_NOT` free-text prose, no structured form at all
///   (~425 per 200k notices; `339168-2017` is one, with 24 language versions of
///   paragraphs and a `TD` code saying "Contract award notice");
/// - the F06 utilities award container is published EMPTY,
///   `<AWARD_CONTRACT_CONTRACT_AWARD_UTILITIES/>` (~465 per 200k; `017037-2017`,
///   which the r209 suite pins as a fixture).
///
/// So `award_notices - with_results - no_award_content` is the number that means
/// "we failed to project an award somebody actually published" — the only one of
/// the three that is ours to fix. Printing the rate without this column invites
/// exactly the wrong conclusion, which is the mistake issue 242 opened with.
///
/// This DOES read `notice_sections`, deliberately: the point is to compare the
/// published type against the parse, and the comparison is the finding. What
/// section 3's denominator must never do is DERIVE itself from the parse — see
/// [`awards_can_sql`].
pub fn awards_barren_sql() -> String {
    awards_barren_template("")
}

fn awards_barren_template(win: &str) -> String {
    format!(
        "SELECT n.profile, COUNT(*) AS no_award_content \
           FROM tender_versions tv JOIN notices n ON n.id = tv.caused_by_notice_id \
          WHERE {win}({award}) \
            AND NOT EXISTS(SELECT 1 FROM notice_sections s \
                            WHERE s.notice_id = tv.caused_by_notice_id \
                              AND s.kind IN ('LotResult', 'TenderResult')) \
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
    // Section 3 proper: the denominator the notice publishes about itself.
    out.push(("awards_can".to_owned(), awards_can_sql()));
    out.push(("awards_with".to_owned(), awards_with_sql()));
    out.push(("awards_barren".to_owned(), awards_barren_sql()));
    out.push(("doc_types".to_owned(), doc_type_sql()));
    // The section→row invariant, under its own name (issue 235): worth keeping,
    // just not a density.
    out.push(("sections_can".to_owned(), SECTIONS_CAN_SQL.to_owned()));
    out.push(("sections_with".to_owned(), SECTIONS_WITH_SQL.to_owned()));
    out.push(("merge".to_owned(), MERGE_SQL.to_owned()));
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
    // The three document-type queries build their windowed form from the SAME
    // builder as the catalog form, with the predicate passed in rather than
    // spliced by text — one source, so windowed and unwindowed cannot come to
    // measure different populations.
    out.push(WindowedQuery {
        label: "awards_can".to_owned(),
        template: awards_can_template("{window} AND "),
        column: "tv.tender_id".to_owned(),
    });
    out.push(WindowedQuery {
        label: "awards_with".to_owned(),
        template: awards_with_template("{window} AND "),
        column: "tv.tender_id".to_owned(),
    });
    out.push(WindowedQuery {
        label: "awards_barren".to_owned(),
        template: awards_barren_template("{window} AND "),
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
    /// all — see [`awards_barren_sql`]. Not a failure of ours, and the difference
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
    pub awards_can: Rows,
    pub awards_with: Rows,
    pub awards_barren: Rows,
    pub doc_types: Rows,
    pub sections_can: Rows,
    pub sections_with: Rows,
    pub merge: Rows,
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
            awards_can: take("awards_can", &mut unmeasured)?,
            awards_with: take("awards_with", &mut unmeasured)?,
            awards_barren: take("awards_barren", &mut unmeasured)?,
            doc_types: take("doc_types", &mut unmeasured)?,
            sections_can: take("sections_can", &mut unmeasured)?,
            sections_with: take("sections_with", &mut unmeasured)?,
            merge: take("merge", &mut unmeasured)?,
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
    let can = count_by_profile(&raw.awards_can);
    let with = count_by_profile(&raw.awards_with);
    let barren = count_by_profile(&raw.awards_barren);
    let mut profiles: std::collections::BTreeSet<String> = can.keys().cloned().collect();
    profiles.extend(with.keys().cloned());
    let density: Vec<DensityRow> = profiles
        .into_iter()
        .map(|profile| DensityRow {
            award_notices: can.get(&profile).copied().unwrap_or(0),
            with_results: with.get(&profile).copied().unwrap_or(0),
            no_award_content: barren.get(&profile).copied().unwrap_or(0),
            profile,
        })
        .collect();

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

    Report {
        base_url: base_url.to_owned(),
        completeness,
        linkage,
        density,
        invariant,
        doc_types,
        merge,
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
        "era", "award-notices", "with lot_results", "density", "no content pub."
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
    // The column that makes the rate readable (issue 242): of the notices that
    // announce a result, how many published no award block for anyone to project.
    // What is left after subtracting those is the part that is ours to fix, so say
    // that number out loud rather than leaving the reader to compute it.
    if report.unmeasured.iter().any(|l| l == "awards_barren") {
        let _ = writeln!(
            out,
            "  no content published: UNMEASURED — the `awards_barren` query did not run, so the \
             density above cannot be split into published-nothing and failed-to-project."
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
            "  no content published: {} award notice(s) parsed with no result block at all — an \
             empty or free-text award (measured shapes: an `OTH_NOT` prose body, an empty F06 \
             container). Nothing can project those. Unmaterialised award notices that DID publish \
             a result block, i.e. the projection's own shortfall: {}.",
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
    out
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
    let value = json!({
        "base_url": report.base_url,
        "unit": "tender-version",
        "completeness": completeness,
        "award_linkage": linkage,
        "results_density": density,
        "sections_to_rows": invariant,
        "doc_type_coverage": doc_types,
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
                // Section 3: the notice's own published type (issue 235) …
                "awards_can",
                "awards_with",
                // …split into "published nothing" and "we missed it" (issue 242) …
                "awards_barren",
                "doc_types",
                // … and the section→row invariant that used to wear its name.
                "sections_can",
                "sections_with",
                "merge",
            ]
        );
    }

    /// Issue 230: every catalog query must be either windowed or explicitly named
    /// unmeasurable. A query that is in neither list is one the report silently does
    /// not measure — the failure mode this whole issue is about, reintroduced by a
    /// future addition rather than by a timeout.
    #[test]
    fn every_query_is_either_windowed_or_declared_unmeasured() {
        let catalog: Vec<String> = queries().into_iter().map(|(l, _)| l).collect();
        let mut covered: Vec<String> =
            windowed_queries().into_iter().map(|q| q.label).chain(unwindowed_labels()).collect();
        covered.sort();
        let mut expected = catalog.clone();
        expected.sort();
        assert_eq!(covered, expected, "windowed ∪ unmeasured must be exactly the catalog");

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
            // The prod shape: a zero denominator under a large numerator.
            ("awards_can".to_owned(), Some(vec![])),
            ("awards_with".to_owned(), Some(vec![vec![json!("eforms:eforms-sdk-0.1"), json!(139_961)]])),
            ("awards_barren".to_owned(), Some(vec![])),
            ("doc_types".to_owned(), Some(vec![])),
            ("sections_can".to_owned(), Some(vec![])),
            ("sections_with".to_owned(), Some(vec![])),
            ("merge".to_owned(), Some(vec![vec![json!("all"), json!(0), json!(0)]])),
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
    /// So: no `notice_sections` anywhere in either half of section 3. If a future
    /// edit reaches for it again for convenience, this fails.
    #[test]
    fn the_award_denominator_never_reads_the_projections_own_output() {
        for (label, sql) in [("awards_can", awards_can_sql()), ("awards_with", awards_with_sql())] {
            assert!(
                !sql.contains("notice_sections"),
                "{label} must read the notice's PUBLISHED type, not what the projection wrote: {sql}"
            );
            assert!(sql.contains("notice_codes"), "{label} reads the doc type from notice_codes: {sql}");
        }
        // And each half seeks by the primary-key prefix rather than scanning the
        // notice's codes: measured, this is 1.3 s vs an 11 s timeout on one window.
        for (label, sql) in [
            ("awards_can", awards_can_sql()),
            ("awards_with", awards_with_sql()),
            ("doc_types", doc_type_sql()),
        ] {
            assert!(
                sql.contains("c.section_id = 'PROCEDURE'"),
                "{label} must pin the section so the probe is a (notice_id, section_id, field_id) \
                 prefix seek: {sql}"
            );
        }

        // The numerator carries the denominator's predicate verbatim, so it is a
        // subset by construction and the rate cannot exceed 1.
        let award = award_predicate();
        assert!(awards_can_sql().contains(&award), "the denominator IS the award predicate");
        assert!(awards_with_sql().contains(&award), "the numerator repeats it verbatim");
        // And the numerator adds exactly one thing: the canonical row.
        assert!(awards_with_sql().contains("FROM lot_results lr"), "{}", awards_with_sql());
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
        let labels = |barren: Rows| -> Vec<(String, Option<Rows>)> {
            let mut out: Vec<(String, Option<Rows>)> =
                queries().into_iter().map(|(l, _)| (l, Some(Vec::new()))).collect();
            for (label, rows) in [
                ("awards_can", vec![
                    vec![json!("ted-export-r208"), json!(10)],
                    vec![json!("eforms:eforms-sdk-1.13"), json!(10)],
                ]),
                ("awards_with", vec![
                    vec![json!("ted-export-r208"), json!(4)],
                    vec![json!("eforms:eforms-sdk-1.13"), json!(4)],
                ]),
                ("awards_barren", barren.clone()),
            ] {
                let slot = out.iter_mut().find(|(l, _)| l == label).expect("label");
                slot.1 = Some(rows);
            }
            out
        };

        let raw = Raw::from_labelled(labels(vec![
            vec![json!("ted-export-r208"), json!(6)],
            vec![json!("eforms:eforms-sdk-1.13"), json!(0)],
        ]))
        .expect("labelled");
        let report = assemble("http://x", &raw);
        let r208 = report.density.iter().find(|r| r.profile == "ted-export-r208").expect("r208");
        assert_eq!((r208.award_notices, r208.with_results, r208.no_award_content), (10, 4, 6));

        let text = render_text(&report);
        // The rate is still reported honestly …
        assert!(text.contains("40.0%"), "{text}");
        // … and so is the split: 6 published nothing, 6 are ours (the sdk-1.13 era).
        assert!(
            text.contains("6 award notice(s) parsed with no result block at all"),
            "the publication gap must be named: {text}"
        );
        assert!(
            text.contains("the projection's own shortfall: 6"),
            "and so must the part that is ours: {text}"
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
        let mut failed = labels(Vec::new());
        failed.iter_mut().find(|(l, _)| l == "awards_barren").expect("label").1 = None;
        let raw = Raw::from_labelled(failed).expect("labelled");
        let text = render_text(&assemble("http://x", &raw));
        assert!(text.contains("no content published: UNMEASURED"), "{text}");
        assert!(!text.contains("the projection's own shortfall"), "no arithmetic on a missing input: {text}");
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
        let sql = awards_can_sql();
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
            // Density: 2 award-TYPED notices for the eForms era, 0 materialised.
            ("awards_can".to_owned(), Some(vec![vec![json!("eforms:eforms-sdk-1.13"), json!(2)]])),
            ("awards_with".to_owned(), Some(vec![])),
            // Both eForms award notices published a result block, so the 0 % density
            // is entirely the projection's own gap — nothing is explained away.
            ("awards_barren".to_owned(), Some(vec![])),
            // One version of the r209 era carries a type this vocabulary cannot read.
            ("doc_types".to_owned(), Some(vec![vec![json!("ted-export-r209"), json!(1), json!(0)]])),
            // The section→row invariant: 3 parsed a result section, all 3 written.
            ("sections_can".to_owned(), Some(vec![vec![json!("ted-export-r209"), json!(3)]])),
            ("sections_with".to_owned(), Some(vec![vec![json!("ted-export-r209"), json!(3)]])),
            // Column 0 is merge's constant scope label (issue 230) — the shape that
            // lets a single-row result sum across windows like every other one.
            ("merge".to_owned(), Some(vec![vec![json!("all"), json!(3), json!(1)]])),
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
