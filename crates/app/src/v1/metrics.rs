//! `/metrics` — the in-process Prometheus text endpoint (issue 53).
//!
//! Every number here is either already computed for another surface or an O(1)
//! read: the dashboard's 60-second cache (quarantine, canonical row counts,
//! import lag), the reader-pooled job-log window `/health/deep` also reads, the
//! disk stats, `/proc/self/status` RSS, the in-memory change cursor, and the
//! SSE stream count. The scrape must never become the load it exists to
//! observe: no table-proportional scan runs on this path — a gauge whose
//! source would need one reads the dashboard cache instead, and is simply
//! absent while that cache is still measuring (a scraper sees the gauge appear
//! when the first measurement lands; absence is honest, a made-up zero is not).
//!
//! The exposition format is hand-rolled: it is `name{label="v"} value` lines
//! plus `# HELP`/`# TYPE` headers, and a metrics crate for that would be
//! weight without payoff — the issue-53 decision already rejected OTel on the
//! same grounds. Standing up a Prometheus server stays a later, reversible
//! choice; this endpoint is what makes it one.

use std::collections::HashSet;

use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use model::ingestion::JobRun;

use super::{AppState, health};

/// The scrape. Gauges only — every value is a level re-read from its source,
/// not an in-process accumulation, so a restart cannot silently reset a
/// counter mid-series.
pub async fn metrics(State(state): State<AppState>) -> Response {
    let mut out = String::with_capacity(4096);

    // Process + in-memory signals, all O(1).
    header(&mut out, "tender_db_change_cursor", "Newest change-log id (the SSE doorbell).");
    sample(&mut out, "tender_db_change_cursor", &[], state.db.current_cursor() as f64);
    if let Some(rss) = rss_bytes() {
        header(&mut out, "tender_db_rss_bytes", "Resident set size of the server process.");
        sample(&mut out, "tender_db_rss_bytes", &[], rss as f64);
    }
    header(&mut out, "tender_db_sse_streams", "Live SSE subscriptions.");
    sample(&mut out, "tender_db_sse_streams", &[], state.live_streams() as f64);

    // Disk on the DB volume — the same statvfs `/health/deep` folds into its
    // verdict, plus the WAL sidecar size (the issue-42 runaway signal).
    if let Some(d) = health::disk_usage() {
        header(&mut out, "tender_db_disk_used_fraction", "Used fraction of the DB volume.");
        sample(&mut out, "tender_db_disk_used_fraction", &[], d.used_fraction);
        header(&mut out, "tender_db_disk_free_bytes", "Free bytes on the DB volume.");
        sample(&mut out, "tender_db_disk_free_bytes", &[], d.free_bytes as f64);
        header(&mut out, "tender_db_disk_total_bytes", "Total bytes on the DB volume.");
        sample(&mut out, "tender_db_disk_total_bytes", &[], d.total_bytes as f64);
        if let Some(wal) = d.wal_bytes {
            header(&mut out, "tender_db_wal_bytes", "Size of the database's -wal sidecar.");
            sample(&mut out, "tender_db_wal_bytes", &[], wal as f64);
        }
    }

    // The legacy-adjacency coverage watermark (issue 58 v2). A one-row point read
    // on the reader pool, and the only external view of the claim the incremental
    // projection's closure walk gates on: > 0 means it may scope a legacy fold to
    // the touched component, 0 means every legacy delta takes the full-projection
    // fallback. The adjacency tables are deliberately not in `/v1/sql`'s public
    // allow-list (they are projection machinery, not corpus data), so without this
    // gauge the gate's input is unobservable from outside the process — which is
    // exactly the position the step-2 backfill's verification found itself in.
    if let Ok(watermark) = state.db.legacy_adjacency_watermark_observed().await {
        header(
            &mut out,
            "tender_db_legacy_adjacency_watermark",
            "Notice id up to which legacy OJS adjacency coverage is attested; 0 = never established.",
        );
        sample(&mut out, "tender_db_legacy_adjacency_watermark", &[], watermark as f64);
    }

    // The job log — the same bounded reader-pool window `/health/deep` reads
    // (newest `JOB_SCAN` runs), reduced to the newest run per kind. Durations
    // and finish stamps per kind are the "watch a number trend" series the
    // issue was opened for (a slowing daily `process` shows up here long
    // before it misses the freshness threshold).
    if let Ok(runs) = state.db.recent_job_runs(health::JOB_SCAN).await {
        if let Some(at) = health::ingest_last_success(&runs) {
            header(
                &mut out,
                "tender_db_ingest_last_success_timestamp_seconds",
                "When the newest successful daily-pipeline run (probe/process/project) finished.",
            );
            sample(&mut out, "tender_db_ingest_last_success_timestamp_seconds", &[], at as f64);
        }
        emit_job_gauges(&mut out, &runs);
    }

    // The dashboard's cached sections — measured by its background refresher on
    // its own cadence and gates (never by this scrape). Sections still `None`
    // (booting, or gated behind a heavy write) are omitted, not zeroed.
    let dash = crate::coverage::latest();
    if let Some(system) = &dash.system {
        header(
            &mut out,
            "tender_db_dashboard_measured_timestamp_seconds",
            "When the dashboard cache last measured its system section.",
        );
        sample(&mut out, "tender_db_dashboard_measured_timestamp_seconds", &[], system.measured_at as f64);
        if let Some(age) = system.lag.fetch_age {
            header(&mut out, "tender_db_ingest_fetch_age_seconds", "Age of the newest fetched package.");
            sample(&mut out, "tender_db_ingest_fetch_age_seconds", &[], age as f64);
        }
        if let Some(age) = system.lag.notice_age {
            header(&mut out, "tender_db_ingest_notice_age_seconds", "Age of the newest stored notice.");
            sample(&mut out, "tender_db_ingest_notice_age_seconds", &[], age as f64);
        }
    }
    if let Some(counts) = &dash.counts {
        header(&mut out, "tender_db_canonical_rows", "Rows per canonical table (dashboard cache).");
        for c in counts {
            sample(&mut out, "tender_db_canonical_rows", &[("table", &c.label)], c.value as f64);
        }
    }
    if let Some(q) = &dash.quarantine {
        for (name, help, value) in [
            ("tender_db_quarantine_total", "Quarantined members, all time (dashboard cache).", q.total),
            ("tender_db_quarantine_outstanding", "Quarantined members not yet resolved.", q.outstanding),
            ("tender_db_quarantine_reclaimed", "Quarantined members reclaimed into the corpus.", q.reclaimed),
            ("tender_db_quarantine_skipped", "Quarantined members resolved as policy skips.", q.skipped),
            ("tender_db_quarantine_actionable", "Outstanding members with a known fix path.", q.actionable),
            ("tender_db_quarantine_suspected", "Outstanding members under a suspected cause.", q.suspected),
        ] {
            header(&mut out, name, help);
            sample(&mut out, name, &[], value as f64);
        }
        header(&mut out, "tender_db_quarantine_reason_members", "Quarantined members by reason (dashboard cache).");
        for c in &q.by_reason {
            sample(&mut out, "tender_db_quarantine_reason_members", &[("reason", &c.label)], c.value as f64);
        }
    }

    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        out,
    )
        .into_response()
}

/// Per-kind last-run gauges from the newest-first job log: duration, finish
/// stamp, and outcome (1 ok / 0 error) for the newest run of each kind seen in
/// the window.
fn emit_job_gauges(out: &mut String, runs: &[JobRun]) {
    let mut seen = HashSet::new();
    let newest: Vec<&JobRun> = runs.iter().filter(|r| seen.insert(r.kind.as_str())).collect();
    if newest.is_empty() {
        return;
    }
    header(out, "tender_db_job_last_duration_seconds", "Wall-clock duration of the newest run per job kind.");
    for r in &newest {
        sample(
            out,
            "tender_db_job_last_duration_seconds",
            &[("kind", &r.kind)],
            (r.finished_at - r.started_at).max(0) as f64,
        );
    }
    header(out, "tender_db_job_last_finished_timestamp_seconds", "When the newest run per job kind finished.");
    for r in &newest {
        sample(out, "tender_db_job_last_finished_timestamp_seconds", &[("kind", &r.kind)], r.finished_at as f64);
    }
    header(out, "tender_db_job_last_ok", "Whether the newest run per job kind succeeded (1) or errored (0).");
    for r in &newest {
        let ok = if r.outcome == "ok" { 1.0 } else { 0.0 };
        sample(out, "tender_db_job_last_ok", &[("kind", &r.kind)], ok);
    }
}

/// `# HELP` + `# TYPE` for one metric. Everything exposed here is a gauge —
/// see [`metrics`] on why no in-process counters exist.
fn header(out: &mut String, name: &str, help: &str) {
    out.push_str("# HELP ");
    out.push_str(name);
    out.push(' ');
    out.push_str(help);
    out.push_str("\n# TYPE ");
    out.push_str(name);
    out.push_str(" gauge\n");
}

/// One sample line: `name{label="value",...} 42`. Label values are escaped per
/// the exposition format (backslash, quote, newline); label names and metric
/// names are compile-time literals here, never data.
fn sample(out: &mut String, name: &str, labels: &[(&str, &str)], value: f64) {
    out.push_str(name);
    if !labels.is_empty() {
        out.push('{');
        for (i, (k, v)) in labels.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(k);
            out.push_str("=\"");
            out.push_str(&escape(v));
            out.push('"');
        }
        out.push('}');
    }
    out.push(' ');
    out.push_str(&format_value(value));
    out.push('\n');
}

/// Label-value escaping per the Prometheus text exposition format.
fn escape(v: &str) -> String {
    v.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

/// `f64` Display prints integral values without a trailing `.0`, which is what
/// the format wants; NaN/infinities cannot arise from the sources above but
/// would serialize as `NaN`/`inf`, both of which Prometheus parses.
fn format_value(value: f64) -> String {
    format!("{value}")
}

/// VmRSS from `/proc/self/status`, in bytes. `None` off Linux or if the file
/// is unreadable — the gauge is then absent rather than zero.
fn rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(kind: &str, started_at: i64, finished_at: i64, outcome: &str) -> JobRun {
        JobRun {
            id: 0,
            kind: kind.into(),
            params: String::new(),
            started_at,
            finished_at,
            outcome: outcome.into(),
            counts: String::new(),
        }
    }

    #[test]
    fn label_values_are_escaped() {
        let mut out = String::new();
        sample(&mut out, "m", &[("reason", "a \"quoted\\\" reason\nsecond line")], 1.0);
        assert_eq!(out, "m{reason=\"a \\\"quoted\\\\\\\" reason\\nsecond line\"} 1\n");
    }

    #[test]
    fn integral_values_print_without_a_decimal_point() {
        assert_eq!(format_value(123.0), "123");
        assert_eq!(format_value(0.913), "0.913");
    }

    #[test]
    fn job_gauges_take_the_newest_run_per_kind() {
        // Newest-first, as recent_job_runs returns them: the kind seen twice
        // must report its NEWER run (error, 5s), not the older success.
        let runs =
            vec![run("process", 100, 105, "error"), run("probe", 90, 91, "ok"), run("process", 10, 80, "ok")];
        let mut out = String::new();
        emit_job_gauges(&mut out, &runs);
        assert!(out.contains("tender_db_job_last_duration_seconds{kind=\"process\"} 5\n"), "{out}");
        assert!(out.contains("tender_db_job_last_ok{kind=\"process\"} 0\n"), "{out}");
        assert!(out.contains("tender_db_job_last_ok{kind=\"probe\"} 1\n"), "{out}");
        // One HELP/TYPE block per metric name, however many kinds sample it.
        assert_eq!(out.matches("# TYPE tender_db_job_last_ok gauge").count(), 1, "{out}");
    }

    #[test]
    fn no_runs_emit_no_job_headers() {
        let mut out = String::new();
        emit_job_gauges(&mut out, &[]);
        assert_eq!(out, "");
    }
}
