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

    // Writer contention (issue 241). Issue 240 was a 25-minute outage of every
    // token-bearing endpoint, and every signal on this page stayed green through
    // it: requests were queued behind a fold that held the writer, and queueing
    // was the one thing nothing measured.
    //
    // A held writer is normal. `queue_depth` is the part that is not: sustained
    // non-zero depth means callers are waiting, whoever holds it. The two totals
    // give the mean wait per acquisition, and `longest_wait_seconds` is a
    // never-reset high-water mark, so a stall stays visible after it ends.
    // The deadline layer's cut count (issue 241 gap 2): the writer gauges below
    // say a stall is happening; this says one already turned into a 503.
    header(
        &mut out,
        "tender_db_request_deadline_hits_total",
        "Requests cut by the /v1 whole-request deadline since open.",
    );
    sample(
        &mut out,
        "tender_db_request_deadline_hits_total",
        &[],
        super::DEADLINE_HITS.load(std::sync::atomic::Ordering::Relaxed) as f64,
    );

    let writer = state.db.writer_stats();
    header(&mut out, "tender_db_writer_queue_depth", "Callers blocked waiting for the writer.");
    sample(&mut out, "tender_db_writer_queue_depth", &[], writer.depth as f64);
    header(&mut out, "tender_db_writer_acquisitions_total", "Writer acquisitions since open.");
    sample(&mut out, "tender_db_writer_acquisitions_total", &[], writer.acquisitions as f64);
    header(&mut out, "tender_db_writer_wait_seconds_total", "Seconds spent waiting for the writer.");
    sample(&mut out, "tender_db_writer_wait_seconds_total", &[], writer.waited_seconds);
    header(
        &mut out,
        "tender_db_writer_longest_wait_seconds",
        "Longest single wait for the writer since open (high-water mark).",
    );
    sample(&mut out, "tender_db_writer_longest_wait_seconds", &[], writer.longest_wait_seconds);

    // Where a re-parse's per-notice time goes (issue 247). Zero until one runs. The
    // four phases sum to the writer-side cost of a notice, so a crawling campaign says
    // which statement is paying rather than leaving an operator to profile the box.
    let reparse = state.db.reparse_stats();
    if reparse.notices > 0 {
        header(&mut out, "tender_db_reparse_notices_total", "Notices re-parsed since open.");
        sample(&mut out, "tender_db_reparse_notices_total", &[], reparse.notices as f64);
        // One HELP/TYPE pair for the family, then the four labelled samples — the
        // shape every other labelled series here uses, and the one a scrape can parse.
        header(
            &mut out,
            "tender_db_reparse_phase_seconds_total",
            "Seconds spent in each re-parse phase, on the writer connection.",
        );
        for (phase, seconds) in [
            ("lookup", reparse.lookup_seconds),
            ("clear", reparse.clear_seconds),
            ("insert", reparse.insert_seconds),
            ("commit", reparse.commit_seconds),
        ] {
            sample(&mut out, "tender_db_reparse_phase_seconds_total", &[("phase", phase)], seconds);
        }
        // The clear is nine statements and, measured, 99.6% of the writer-side cost, so
        // the phase total alone names the wrong thing. One series per statement says
        // which DELETE is paying.
        if !reparse.clear_statements.is_empty() {
            header(
                &mut out,
                "tender_db_reparse_clear_statement_seconds_total",
                "Seconds per statement inside a re-parse's clear.",
            );
            for (stmt, seconds) in &reparse.clear_statements {
                sample(
                    &mut out,
                    "tender_db_reparse_clear_statement_seconds_total",
                    &[("stmt", stmt)],
                    *seconds,
                );
            }
        }
    }

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

    // The running job's identity and phase (issue 65) — from the supervisor's
    // in-memory progress, a lock-and-clone, no DB. Absent when no job runs or
    // when this API serves without a supervisor (tests, embeddings).
    if let Some(job) = state.jobs.as_ref().and_then(|s| s.current_progress()) {
        render_running_job(&mut out, &job);
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

    // Stored report freshness (issue 230). The timestamp, not an age — Prometheus
    // convention is to expose the stamp and let the alert say `time() - stamp >
    // 10d`, and a stamp is also the series that survives a scrape gap honestly.
    //
    // This is the gauge that makes the original rot alertable. The data-quality
    // report had been failing every query for months and nothing noticed, because
    // "nobody ran it" and "it ran and passed" produce the same silence. A kind
    // that has never been computed emits NO series rather than a zero, so the
    // alert fires on absence instead of on a fabricated 1970 stamp.
    if let Ok(stamps) = state.db.report_stamps().await {
        if !stamps.is_empty() {
            header(
                &mut out,
                "tender_db_report_computed_timestamp_seconds",
                "When each stored report was last computed; absent = never computed.",
            );
            for (kind, at) in &stamps {
                sample(
                    &mut out,
                    "tender_db_report_computed_timestamp_seconds",
                    &[("kind", kind)],
                    *at as f64,
                );
            }
        }
    }

    // The job log — the same bounded reader-pool window `/health/deep` reads
    // (newest `JOB_SCAN` runs), reduced to the newest run per kind. Durations
    // and finish stamps per kind are the "watch a number trend" series the
    // issue was opened for (a slowing daily `process` shows up here long
    // before it misses the freshness threshold).
    // Per-era data-quality gauges (issue 266), from the stored headline
    // history (issue 265) — a point lookup on the reports table, never a
    // measurement: the weekly run pays the scan, the scrape reads its result.
    // Absent entirely until the first run stores a history (a gauge that
    // appears is honest; one that reads 0 before measuring is the issue-230
    // zero-lie), and a rate whose denominator is 0 is skipped, not emitted as
    // 0. `dq_report_age_seconds` is what makes a silently-stopped weekly run
    // alertable — the issue-161 class, where a dead observer reads as
    // permanently green.
    if let Ok(Some((body, computed_at))) = state.db.latest_report("data-quality-headlines").await {
        header(
            &mut out,
            "tender_db_dq_report_age_seconds",
            "Seconds since the newest data-quality headline run.",
        );
        sample(
            &mut out,
            "tender_db_dq_report_age_seconds",
            &[],
            (store::now_unix() - computed_at) as f64,
        );
        let runs: Vec<model::QualityRun> = serde_json::from_str(&body).unwrap_or_default();
        if let Some(latest) = runs.last() {
            let gauges: [(&str, &str, fn(&model::QualityEra) -> [u64; 2]); 7] = [
                ("tender_db_dq_factless_rate", "Shell versions / versions (issue 109).", |e| e.factless),
                ("tender_db_dq_value_completeness", "Versions carrying an amount / versions.", |e| e.value),
                ("tender_db_dq_winner_named_rate", "Results naming a winner / results a winner was possible for.", |e| e.named),
                ("tender_db_dq_award_linkage_rate", "Award Tenders chained to a contract notice / award Tenders.", |e| e.linkage),
                ("tender_db_dq_vat_stated_rate", "Amounts stating a VAT basis / amounts (issue 251).", |e| e.vat_stated),
                ("tender_db_dq_negative_amount_rate", "Negative amounts / amounts (issue 267; overwhelmingly source-published — the rate MOVING is the signal).", |e| e.negative),
                ("tender_db_dq_eur_convertible_rate", "Amounts with a derived eur_cents / amounts (ADR-0014 D4: unresolvable is NULL, never a guess).", |e| e.eur_convertible),
            ];
            for (name, help, pick) in gauges {
                header(&mut out, name, help);
                for era in &latest.eras {
                    let [num, den] = pick(era);
                    if den > 0 {
                        sample(&mut out, name, &[("era", &era.profile)], num as f64 / den as f64);
                    }
                }
            }
            // The fold-cost tripwire (issue 92): a whole-corpus scalar, not a
            // per-era rate. 0 means the run predates the measurement (serde
            // default) — absent beats a fake zero, same rule as the rates.
            if latest.longest_chain > 0 {
                header(
                    &mut out,
                    "tender_db_dq_longest_chain",
                    "Longest version chain in the corpus (issue 92; fold() is O(chain^2), flag threshold 4000).",
                );
                sample(&mut out, "tender_db_dq_longest_chain", &[], latest.longest_chain as f64);
            }
        }
    }

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
        // Issue 303: the terminal tripwire. 0 is the steady state; any reason
        // above its curated terminal policy counts here, so "quarantine is
        // done" stays an alertable fact instead of a memory — a new era
        // quarantining under a NEW reason trips this too (unknown reasons
        // default to a zero baseline by design).
        let exceeded = model::dashboard::quarantine_terminal_exceeded(&q.by_reason);
        header(
            &mut out,
            "tender_db_quarantine_terminal_exceeded",
            "Reasons whose outstanding count exceeds the curated terminal ledger (issue 303; 0 = terminal state holds).",
        );
        sample(&mut out, "tender_db_quarantine_terminal_exceeded", &[], exceeded.len() as f64);
    }

    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        out,
    )
        .into_response()
}

/// The running job's gauges (issue 65): that it runs, when it started, and its
/// phase counts. Labels carry only the small closed vocabularies — job `kind`
/// and phase `name` — never the free-text detail, which is unbounded and would
/// mint a new series per sweep position. The phase's `updated_at` is exported
/// so an alert can express "a job is running but its reporter went silent",
/// the dead-vs-slow distinction, as `time() - updated > threshold`.
fn render_running_job(out: &mut String, job: &model::ingestion::JobProgress) {
    header(out, "tender_db_job_running", "1 while a job runs; the series is absent when idle.");
    sample(out, "tender_db_job_running", &[("kind", &job.kind)], 1.0);
    header(out, "tender_db_job_started_timestamp_seconds", "When the running job started.");
    sample(out, "tender_db_job_started_timestamp_seconds", &[("kind", &job.kind)], job.started_at as f64);
    let Some(phase) = &job.phase else { return };
    let labels = [("kind", job.kind.as_str()), ("phase", phase.name.as_str())];
    if let Some(done) = phase.done {
        header(out, "tender_db_job_phase_done", "Units completed in the running job's phase.");
        sample(out, "tender_db_job_phase_done", &labels, done as f64);
    }
    if let Some(total) = phase.total {
        header(out, "tender_db_job_phase_total", "The phase's end, when it is known up front.");
        sample(out, "tender_db_job_phase_total", &labels, total as f64);
    }
    header(
        out,
        "tender_db_job_phase_updated_timestamp_seconds",
        "When the phase was last reported — a stale stamp under a running job is a silent reporter.",
    );
    sample(out, "tender_db_job_phase_updated_timestamp_seconds", &labels, phase.updated_at as f64);
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
            job_id: None,
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

    #[test]
    fn a_running_job_renders_its_phase_without_the_free_text() {
        let job = model::ingestion::JobProgress {
            id: 1,
            kind: "backfill-legacy-adjacency".into(),
            params: String::new(),
            started_at: 1_000,
            package: None,
            packages_done: 0,
            packages_total: 0,
            members_done: 5,
            members_total: 0,
            notices: 0,
            duplicates: 0,
            phase: Some(model::ingestion::Phase {
                name: "sweeping".into(),
                done: Some(11_400_000),
                total: Some(28_251_412),
                detail: "notice id 11,400,000 of 28,251,412".into(),
                updated_at: 2_000,
            }),
        };
        let mut out = String::new();
        render_running_job(&mut out, &job);
        assert!(out.contains("tender_db_job_running{kind=\"backfill-legacy-adjacency\"} 1\n"), "{out}");
        assert!(
            out.contains("tender_db_job_phase_done{kind=\"backfill-legacy-adjacency\",phase=\"sweeping\"} 11400000\n"),
            "{out}"
        );
        assert!(
            out.contains("tender_db_job_phase_updated_timestamp_seconds{kind=\"backfill-legacy-adjacency\",phase=\"sweeping\"} 2000\n"),
            "{out}"
        );
        // The unbounded detail string must never become a label or a series.
        assert!(!out.contains("11,400,000 of"), "detail text stays out of the exposition: {out}");

        // A phase with no total (the pre-pass shape) renders done alone; a job
        // with no phase renders only the running pair.
        let mut bare = job.clone();
        bare.phase = None;
        let mut out = String::new();
        render_running_job(&mut out, &bare);
        assert!(out.contains("tender_db_job_running"), "{out}");
        assert!(!out.contains("tender_db_job_phase"), "no phase, no phase series: {out}");
    }
}
