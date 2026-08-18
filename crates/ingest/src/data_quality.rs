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
//! projecting" into "eForms CANs materialise results at 0.3 %"; the fixes it
//! points at become their own issues.
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
//!   drive from their small sets (award Tenders, projected award notices, DÖE
//!   Tenders) and filter with indexed `EXISTS`, never an inline
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
/// times the endpoint out. The numerator is measured separately, driven *from*
/// `lot_results`, and the two are combined by profile.
pub const DENSITY_CAN_SQL: &str = "SELECT n.profile, COUNT(*) AS can_notices \
       FROM tender_versions tv JOIN notices n ON n.id = tv.caused_by_notice_id \
      WHERE EXISTS(SELECT 1 FROM notice_sections s \
                    WHERE s.notice_id = tv.caused_by_notice_id AND s.kind = 'LotResult') \
      GROUP BY n.profile";

/// Results materialisation per era, numerator: award-notice versions whose own
/// notice actually produced a canonical `lot_results` row.
///
/// Version-driven, matching [`DENSITY_CAN_SQL`]'s unit — and that is a FIX, not a
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
pub const DENSITY_WITH_SQL: &str = "SELECT n.profile, COUNT(*) AS with_results \
       FROM tender_versions tv JOIN notices n ON n.id = tv.caused_by_notice_id \
      WHERE EXISTS(SELECT 1 FROM lot_results lr \
                    WHERE lr.tender_id = tv.tender_id AND lr.notice_id = tv.caused_by_notice_id) \
      GROUP BY n.profile";

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
    out.push(("density_can".to_owned(), DENSITY_CAN_SQL.to_owned()));
    out.push(("density_with".to_owned(), DENSITY_WITH_SQL.to_owned()));
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
///   [`DENSITY_WITH_SQL`] — making them share a unit is what let the numerator be
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
        label: "density_can".to_owned(),
        template: DENSITY_CAN_SQL.replace("WHERE EXISTS(", "WHERE {window} AND EXISTS("),
        column: "tv.tender_id".to_owned(),
    });
    out.push(WindowedQuery {
        label: "density_with".to_owned(),
        template: DENSITY_WITH_SQL.replace("WHERE EXISTS(", "WHERE {window} AND EXISTS("),
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

/// One era's results materialisation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DensityRow {
    pub profile: String,
    pub can_notices: u64,
    pub with_results: u64,
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
    pub density_can: Rows,
    pub density_with: Rows,
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
            density_can: take("density_can", &mut unmeasured)?,
            density_with: take("density_with", &mut unmeasured)?,
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

    // Density is two separately-measured halves (denominator from the projected
    // award notices, numerator from `lot_results`) combined by profile — every
    // profile in either half becomes one row, a missing numerator being 0 (the
    // gap the metric exists to show).
    let can = count_by_profile(&raw.density_can);
    let with = count_by_profile(&raw.density_with);
    let mut profiles: std::collections::BTreeSet<String> = can.keys().cloned().collect();
    profiles.extend(with.keys().cloned());
    let density: Vec<DensityRow> = profiles
        .into_iter()
        .map(|profile| DensityRow {
            can_notices: can.get(&profile).copied().unwrap_or(0),
            with_results: with.get(&profile).copied().unwrap_or(0),
            profile,
        })
        .collect();

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
            report.unmeasured.len() + 11 - report.unmeasured.len(),
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

    let _ = writeln!(out, "\n== 3. Results materialisation (award notices → lot_results) ==");
    let _ = writeln!(out, "  {:<30} {:>10} {:>14} {:>8}", "era", "award-notices", "with lot_results", "density");
    for row in &report.density {
        let _ = writeln!(
            out,
            "  {:<30} {:>10} {:>14} {:>8}",
            display_era(&row.profile), group(row.can_notices), group(row.with_results),
            pct(row.with_results, row.can_notices)
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
            "award_notices": r.can_notices,
            "with_results": r.with_results,
            "density": rate(r.with_results, r.can_notices),
        }))
        .collect();
    let value = json!({
        "base_url": report.base_url,
        "unit": "tender-version",
        "completeness": completeness,
        "award_linkage": linkage,
        "results_density": density,
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
            ["versions", "title", "buyer", "value", "cpv", "deadline", "winner", "linkage", "density_can", "density_with", "merge"]
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
            // Density: 2 projected award notices for the eForms era, 0 materialised.
            ("density_can".to_owned(), Some(vec![vec![json!("eforms:eforms-sdk-1.13"), json!(2)]])),
            ("density_with".to_owned(), Some(vec![])),
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
        assert_eq!(report.density[0].can_notices, 2);
        assert_eq!(report.density[0].with_results, 0);
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
        assert_eq!(json["completeness"][0]["rate"]["winner"], json!(0.2));
    }

    /// Issue 230: a query that FAILED and a query that ran and matched nothing
    /// are different claims, and the rendered report must not merge them. This is
    /// the exact shape prod produced — all 11 queries 408'd — where section 4
    /// printed "DÖE procedure Tenders: 0; merged with TED: 0 (—)" for numbers
    /// nobody had measured.
    #[test]
    fn a_failed_query_renders_as_unmeasured_not_as_zero() {
        let labels = [
            "versions", "title", "buyer", "value", "cpv", "deadline", "winner", "linkage",
            "density_can", "density_with", "merge",
        ];
        // Every query failed: None, not an empty result set.
        let all_failed: Vec<(String, Option<Rows>)> =
            labels.iter().map(|l| ((*l).to_owned(), None)).collect();
        let raw = Raw::from_labelled(all_failed).expect("labelled");
        assert_eq!(raw.unmeasured.len(), 11, "every label recorded as unmeasured");
        let text = render_text(&assemble("http://x", &raw));
        assert!(text.contains("INCOMPLETE: 11 of 11 queries did not run"), "{text}");
        assert!(text.contains("UNMEASURED — the `merge` query did not run."), "{text}");
        assert!(
            !text.contains("merged with TED: 0"),
            "a number nobody measured must not be printed: {text}"
        );

        // The complement: a query that RAN and matched nothing still reports its
        // real zero, and the report is not marked incomplete.
        let mut ran: Vec<(String, Option<Rows>)> =
            labels.iter().map(|l| ((*l).to_owned(), Some(Vec::new()))).collect();
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
