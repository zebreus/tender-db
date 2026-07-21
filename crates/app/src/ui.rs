//! The dashboard: data coverage, data quality, and the account lifecycle.
//!
//! This is the product's face (CONTEXT.md, "Four faces"). It renders numbers the
//! server has already resolved — the client never computes a coverage ratio or
//! reads its own clock for an age, so what a user sees is what the server
//! measured.

use crate::api;
use dioxus::fullstack::Transportable;
use dioxus::prelude::*;
use model::account::LOST_PASSWORD_NOTICE;
use model::dashboard::{
    AwardLinkage, Coverage, Lag, PipelineStage, QuarantineClass, Quarantined, ResolvedCategory,
    quarantine_class,
};
use model::ingestion::{Ingestion, JobProgress, JobRun};
use model::{Account, NewToken, NewWebhook, Token, Webhook};
use std::time::Duration;

/// How often the dashboard re-measures. Deliberately a plain poll: the change
/// feed's SSE plumbing serves API clients, and a dashboard that refreshes twice
/// a minute needs none of it.
const REFRESH_SECONDS: u64 = 15;

/// Poll a server function forever without ever unmounting what is on screen.
///
/// The trap with `use_server_future(f)?` is that `?` re-suspends the whole
/// subtree the instant the resource turns `Pending` — and `.restart()` makes it
/// `Pending` on every tick. That is what made the dashboard flash: each poll
/// replaced the rendered panels with the suspense fallback for a frame (issue
/// 06). This keeps the value instead. The first load still comes through the
/// server render — so the page hydrates with data already in place, and that
/// first load is the only moment a loading state may show — and every refresh
/// after it runs in the background and swaps a signal in place. The last good
/// value stays mounted throughout, so a poll never blanks.
#[track_caller]
fn use_polled<T, F, Fut, M>(period: Duration, fetch: F) -> Result<T, RenderError>
where
    T: Clone + Transportable<M> + 'static,
    F: FnMut() -> Fut + Copy + 'static,
    Fut: std::future::Future<Output = T> + 'static,
    M: 'static,
{
    // First load only: transported through the server render, suspends once.
    let seed = use_server_future(fetch)?;
    // Every refresh after that lands here, disturbing nothing that is rendered.
    let mut latest = use_signal(|| None::<T>);
    use_future(move || async move {
        let mut fetch = fetch;
        loop {
            futures_timer::Delay::new(period).await;
            latest.set(Some(fetch().await));
        }
    });
    // Freshest wins: the newest completed poll, else the seed — which `?`
    // guarantees has resolved by the time control reaches this line.
    Ok(latest().unwrap_or_else(|| seed().unwrap()))
}

// ---------------------------------------------------------------- dashboard

#[component]
pub fn DashboardPage() -> Element {
    let data = use_polled(Duration::from_secs(REFRESH_SECONDS), api::dashboard)?;
    rsx! {
        main {
            Nav {}
            IngestionPanel {}
            match &data {
                Ok(d) => rsx! {
                    section { class: "panel",
                        h2 { "Contents" }
                        dl { class: "counts",
                            for c in d.counts.clone() {
                                dt { key: "{c.label}", "{c.label}" }
                                dd { "{group(c.value)}" }
                            }
                        }
                    }

                    SystemPanel { rev: d.service_rev.clone(), cursor: d.cursor, lag: d.lag, snapshot_age: d.snapshot_age }

                    QuarantinePanel {
                        total: d.quarantine_total,
                        actionable: d.quarantine_actionable,
                        suspected: d.quarantine_suspected,
                        reasons: d.quarantine_by_reason.iter().map(|c| (c.label.clone(), c.value)).collect::<Vec<_>>(),
                        field_code_gaps: d.quarantine_field_code_gaps.iter().map(|c| (c.label.clone(), c.value)).collect::<Vec<_>>(),
                        recent: d.quarantine_recent.clone(),
                        resolved: d.resolved_categories.clone(),
                    }

                    AwardLinkagePanel { rows: d.award_linkage.clone() }

                    PipelinePanel { rows: d.pipeline.clone() }

                    CoveragePanel { rows: d.coverage.clone() }
                },
                Err(e) => rsx! { p { class: "error", "Could not measure: {e}" } },
            }
            Footer {}
        }
    }
}

/// System status in one compact panel: the running server's revision, the change
/// cursor, and how stale each end of the import pipeline is.
#[component]
fn SystemPanel(rev: String, cursor: i64, lag: Lag, snapshot_age: Option<i64>) -> Element {
    rsx! {
        section { class: "panel",
            h2 { "System" }
            p { class: "muted",
                "Fetching and processing are separate stages, so they go stale separately."
            }
            dl { class: "counts",
                dt { "service revision" }
                dd { class: "path", "{rev}" }
                dt { "change cursor" }
                dd { "{group(cursor)}" }
                dt { "newest fetched package" }
                dd { "{age(lag.fetch_age)}" }
                dt { "newest ingested notice" }
                dd { "{age(lag.notice_age)}" }
                dt { "last DB snapshot" }
                dd { "{age(snapshot_age)}" }
            }
        }
    }
}

/// How often the Ingestion panel re-reads the supervisor — faster than the
/// coverage poll, because a running job's progress bar should feel live.
const INGESTION_REFRESH_SECONDS: u64 = 3;

/// The live view of the ingestion Supervisor (issue 16): the running job with a
/// progress bar, the queue, and the recent-run log. Read-only — admin actions
/// are API-only (the dashboard is public and carries no operator secret).
#[component]
fn IngestionPanel() -> Element {
    let ingestion = use_polled(Duration::from_secs(INGESTION_REFRESH_SECONDS), api::ingestion)?;
    let Ingestion { current, queued, recent, measured_at } = match &ingestion {
        Ok(i) => i.clone(),
        Err(e) => {
            return rsx! {
                section { class: "panel",
                    h2 { "Ingestion" }
                    p { class: "error", "Could not read the importer: {e}" }
                }
            };
        }
    };

    rsx! {
        section { class: "panel",
            h2 { "Ingestion" }
            p { class: "muted",
                "The importer runs inside the server (ADR-0005): one job at a time, the "
                "readers serving throughout. Operators drive it through the "
                code { "/admin" } " API."
            }

            match current {
                Some(job) => rsx! { RunningJob { job, measured_at } },
                None => rsx! { p { class: "muted", "Idle — no job running." } },
            }

            if !queued.is_empty() {
                h3 { "Queued ({queued.len()})" }
                ol { class: "queue",
                    for job in queued {
                        li { key: "{job.id}", class: "path",
                            "#{job.id} {job.kind} — {job.params}"
                        }
                    }
                }
            }

            if !recent.is_empty() {
                details {
                    summary { "Recent runs ({recent.len()})" }
                    table {
                        thead {
                            tr {
                                th { "Job" }
                                th { "Params" }
                                th { "Outcome" }
                                th { "Result" }
                            }
                        }
                        tbody {
                            for run in recent {
                                RunRow { run }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RunningJob(job: JobProgress, measured_at: i64) -> Element {
    // Elapsed and throughput are the server's clock minus the job's start — the
    // client never times the job itself.
    let elapsed = (measured_at - job.started_at).max(0);
    let rate = (elapsed > 0).then(|| job.notices as f64 / elapsed as f64);
    // A backfill re-walk writes mostly dedups, not new notices: members climb
    // while notices stay flat. Name it, so "0.0 notices/s" is never mistaken for
    // a hang (issue 33) — the member bar above shows it is very much alive.
    let rewalking = job.duplicates > job.notices;
    rsx! {
        div { class: "running",
            p { class: "job-title", "{job.kind} — {job.params}" }
            if job.packages_total > 0 {
                label { class: "muted",
                    "Package {group(job.packages_done as i64)} / {group(job.packages_total as i64)}"
                    if let Some(pkg) = job.package.clone() {
                        " — {pkg}"
                    }
                }
                progress { max: "{job.packages_total}", value: "{job.packages_done}" }
            }
            if job.members_total > 0 {
                label { class: "muted",
                    "Members {group(job.members_done as i64)} / {group(job.members_total as i64)}"
                }
                progress { max: "{job.members_total}", value: "{job.members_done}" }
            }
            if rewalking {
                p { class: "muted",
                    "Re-walking already-ingested packages — {group(job.duplicates as i64)} dup, "
                    "{group(job.notices as i64)} new · {duration(elapsed)} elapsed"
                }
            } else {
                p { class: "muted",
                    "{group(job.notices as i64)} notices written · {duration(elapsed)} elapsed"
                    if let Some(r) = rate {
                        " · {r:.1} notices/s"
                    }
                }
            }
        }
    }
}

#[component]
fn RunRow(run: JobRun) -> Element {
    let class = if run.outcome == "ok" { "" } else { "error" };
    rsx! {
        tr { key: "{run.id}", class,
            td { "{run.kind}" }
            td { class: "path", "{run.params}" }
            td { "{run.outcome}" }
            td { "{run.counts}" }
        }
    }
}

/// Award-chaining health per era: how many award Tenders never linked to a
/// contract notice and so stand alone (docs/research/ted-legacy-mapping.md §3 —
/// ~17% predicted for R2.0.9). A data-quality signal beside quarantine, not an
/// error: some awards are legitimately reference-free (direct awards).
#[component]
fn AwardLinkagePanel(rows: Vec<AwardLinkage>) -> Element {
    rsx! {
        section { class: "panel",
            h2 { "Award linkage" }
            p { class: "muted",
                "Legacy Tenders chain by transitive OJS references; a missed link strands an award "
                "as a single-notice Tender. Research predicts ≈17% unchained for the R2.0.9 era."
            }
            if rows.is_empty() {
                p { class: "muted", "No award Tenders yet." }
            } else {
                table {
                    thead {
                        tr {
                            th { "era" }
                            th { class: "num", "award tenders" }
                            th { class: "num", "unchained" }
                            th { class: "num", "rate" }
                        }
                    }
                    tbody {
                        for r in rows {
                            tr { key: "{r.era}",
                                td { "{r.era}" }
                                td { class: "num", "{group(r.awards)}" }
                                td { class: "num", "{group(r.unchained)}" }
                                td { class: "num", "{r.ratio * 100.0:.1} %" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The import pipeline per source (issue 33): fetched → processed → projected,
/// so "is downloading done? are we just processing?" is answered at a glance,
/// without ssh. The per-year detail stays in the Coverage panel below.
#[component]
fn PipelinePanel(rows: Vec<PipelineStage>) -> Element {
    if rows.is_empty() {
        return rsx! {};
    }
    rsx! {
        section { class: "panel",
            h2 { "Pipeline" }
            p { class: "muted",
                "Where each source is in the import — fetched packages, then notices "
                "processed out of them, then Tenders projected."
            }
            table {
                thead {
                    tr {
                        th { "Source" }
                        th { class: "num", "Published" }
                        th { "Fetched" }
                        th { class: "num", "Processed" }
                        th { class: "num", "Projected" }
                    }
                }
                tbody {
                    for s in rows {
                        tr { key: "{s.source}",
                            td { class: "path", "{s.source}" }
                            td { class: "num",
                                if let Some(p) = s.published { "{group(p)}" } else { "—" }
                            }
                            td {
                                "{group(s.fetched_packages)} pkgs"
                                if let (Some(from), Some(to)) = (s.fetched_from.clone(), s.fetched_to.clone()) {
                                    span { class: "muted", " ({from} … {to})" }
                                }
                                if s.fetch_complete {
                                    span { " · fetch complete ✓" }
                                }
                            }
                            td { class: "num", "{group(s.processed_notices)}" }
                            td { class: "num", "{group(s.projected_tenders)}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CoveragePanel(rows: Vec<Coverage>) -> Element {
    let eras = coverage_by_era(rows);
    rsx! {
        section { class: "panel",
            h2 { "Coverage" }
            p { class: "muted",
                "Coverage is notices held ÷ notices that year is known to have published "
                "(docs/research/ted-access-channels.md §6): 100 % means we hold the whole year. "
                "One collapsible row per source and profile era — expand for the per-year breakdown. "
                "A dash means no ground-truth denominator exists (any source but TED, or a year "
                "outside the reference counts)."
            }
            p { class: "muted",
                "During the historical backfill these ratios climb: the importer works through the "
                "eras in the background and in learn-order, not calendar-order, so an old year can "
                "sit well below 100 % simply because its packages have not been processed yet. A low "
                "ratio here is work still in progress, not a permanent gap."
            }
            if eras.is_empty() {
                p { class: "muted", "Nothing ingested yet — every year is at 0 %." }
            } else {
                for era in eras {
                    details { key: "{era.source}-{era.profile}", class: "era",
                        summary {
                            span { "{era.source} · {era.profile}" }
                            span { class: "era-cover",
                                "{group(era.held)} / {published_cell(era.published)} · {coverage_pct(era.ratio, era.partial)}"
                            }
                        }
                        table {
                            thead {
                                tr {
                                    th { "Year" }
                                    th { class: "num", "Held" }
                                    th { class: "num", "Published" }
                                    th { class: "num", "Coverage" }
                                }
                            }
                            tbody {
                                for row in era.years {
                                    tr { key: "{row.year}",
                                        td { "{row.year}" }
                                        td { class: "num", "{group(row.held)}" }
                                        td { class: "num", "{published_cell(row.published)}" }
                                        td { class: "num", "{coverage_pct(row.ratio, row.partial)}" }
                                    }
                                }
                            }
                        }
                    }
                }
                p { class: "muted", "* the year is not over; a shortfall there is the calendar, not a gap." }
            }
        }
    }
}

/// One (source, profile) era: the per-year rows plus the totals that let a
/// collapsed row still tell the whole story.
struct CoverageEra {
    source: String,
    profile: String,
    held: i64,
    /// Sum of the years with a known denominator; `None` if none has one.
    published: Option<i64>,
    ratio: Option<f64>,
    /// Any year in the era is still open, so the aggregate ratio is a floor.
    partial: bool,
    years: Vec<Coverage>,
}

/// Fold the flat coverage cells into one era per (source, profile), newest year
/// first within each — so 34 years collapse behind a single scannable summary.
fn coverage_by_era(rows: Vec<Coverage>) -> Vec<CoverageEra> {
    let mut eras: Vec<CoverageEra> = Vec::new();
    for row in rows {
        let era = match eras.iter_mut().find(|e| e.source == row.source && e.profile == row.profile)
        {
            Some(e) => e,
            None => {
                eras.push(CoverageEra {
                    source: row.source.clone(),
                    profile: row.profile.clone(),
                    held: 0,
                    published: None,
                    ratio: None,
                    partial: false,
                    years: Vec::new(),
                });
                eras.last_mut().expect("just pushed")
            }
        };
        era.held += row.held;
        if let Some(p) = row.published {
            era.published = Some(era.published.unwrap_or(0) + p);
        }
        era.partial |= row.partial;
        era.years.push(row);
    }
    for era in &mut eras {
        era.ratio = era.published.map(|p| era.held as f64 / p as f64);
        era.years.sort_by(|a, b| b.year.cmp(&a.year));
    }
    eras
}

/// A published count as a cell — a known denominator, or an em dash where none
/// exists (any source but TED, or a year outside the ground truth).
fn published_cell(published: Option<i64>) -> String {
    match published {
        Some(n) => group(n),
        None => "—".to_owned(),
    }
}

/// A coverage ratio as a percent, dashed where there is no denominator and
/// starred where the year is not over.
fn coverage_pct(ratio: Option<f64>, partial: bool) -> String {
    match ratio {
        Some(r) => format!("{:.2} %{}", r * 100.0, if partial { " *" } else { "" }),
        None => "—".to_owned(),
    }
}

#[component]
fn QuarantinePanel(
    total: i64,
    actionable: i64,
    suspected: i64,
    reasons: Vec<(String, i64)>,
    field_code_gaps: Vec<(String, i64)>,
    recent: Vec<Quarantined>,
    resolved: Vec<ResolvedCategory>,
) -> Element {
    let benign = total - actionable - suspected;
    rsx! {
        section { class: "panel",
            h2 { "Quarantine" }
            p { class: "muted",
                "A notice with content no profile maps is held whole, never partly imported "
                "(ADR-0004). The headline counts only confirmed real-notice loss; two large "
                "buckets are suspected parser gaps under investigation, and a small remainder "
                "is benign non-notice members (issue 30)."
            }
            // The honest headline (issue 30): confirmed real-notice loss only.
            p { class: "headline", "{group(actionable)}" }
            p { class: "muted", "actionable — real notices held whole" }
            p { class: "muted",
                "+ {group(suspected)} suspected parser gap (investigate-then-fix, issues 35/36)"
            }
            if !field_code_gaps.is_empty() {
                p { class: "muted",
                    "— unknown-field-code is one legacy code: "
                    {field_code_gaps.iter().map(|(code, n)| format!("{code} ({})", group(*n))).collect::<Vec<_>>().join(", ")}
                }
            }
            p { class: "muted", "+ {group(benign)} benign (non-notice members) — of {group(total)} total held" }
            if !reasons.is_empty() {
                table {
                    thead {
                        tr {
                            th { "Reason" }
                            th { "Class" }
                            th { "What it means" }
                            th { class: "num", "Count" }
                        }
                    }
                    tbody {
                        for (reason, count) in reasons {
                            tr { key: "{reason}",
                                td { class: "path", "{reason}" }
                                td { class: "muted", "{quarantine_class_label(&reason)}" }
                                td { class: "muted", "{quarantine_reason_explained(&reason)}" }
                                td { class: "num", "{group(count)}" }
                            }
                        }
                    }
                }
            }
            if !recent.is_empty() {
                details {
                    summary { "Most recent {recent.len()}" }
                    table {
                        thead {
                            tr {
                                th { "Reason" }
                                th { "Profile" }
                                th { "Member" }
                            }
                        }
                        tbody {
                            for entry in recent {
                                tr { key: "{entry.member_path}",
                                    td {
                                        title: entry.detail.clone().unwrap_or_else(|| quarantine_reason_explained(&entry.reason).to_owned()),
                                        "{entry.reason}"
                                    }
                                    td { "{entry.profile.clone().unwrap_or_else(|| \"—\".into())}" }
                                    td { class: "path", "{entry.member_path}" }
                                }
                            }
                        }
                    }
                }
            }
            if !resolved.is_empty() {
                h3 { "Resolved categories" }
                p { class: "muted",
                    "Gaps we diagnosed and fixed. As the archive reprocesses, the held "
                    "count falls to zero — the record stays here: what the gap was, the "
                    "change that closed it, and how many notices have come back."
                }
                table {
                    thead {
                        tr {
                            th { "Category" }
                            th { "Diagnosis" }
                            th { "Fix" }
                            th { class: "num", "Reclaimed" }
                            th { "Resolved" }
                        }
                    }
                    tbody {
                        for entry in resolved {
                            tr { key: "{entry.category}",
                                td { "{entry.category}" }
                                td { class: "muted", "{entry.diagnosis}" }
                                td { class: "path", "{entry.fix}" }
                                td { class: "num",
                                    "{group(entry.reclaimed)}"
                                    if entry.outstanding > 0 {
                                        span { class: "muted", " · {group(entry.outstanding)} still held" }
                                    }
                                }
                                td { "{entry.resolved}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ account

#[component]
pub fn AccountPage() -> Element {
    // A resource, not a server future: the answer depends on the session
    // cookie, so it is the client's question to ask.
    let mut account = use_resource(api::me);
    let signed_in = account.read().clone().and_then(std::result::Result::ok).flatten();

    rsx! {
        main {
            Nav {}
            match signed_in {
                Some(user) => rsx! { SignedIn { account: user, on_change: move |()| account.restart() } },
                None => rsx! { SignedOut { on_change: move |()| account.restart() } },
            }
            Footer {}
        }
    }
}

#[component]
fn SignedOut(on_change: EventHandler<()>) -> Element {
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut registering = use_signal(|| false);

    let submit = move |_| async move {
        let (name, secret) = (username(), password());
        let result = if registering() {
            api::register(name, secret).await.map(|_| ())
        } else {
            api::login(name, secret).await.map(|_| ())
        };
        match result {
            Ok(()) => {
                password.set(String::new());
                error.set(None);
                on_change.call(());
            }
            Err(e) => error.set(Some(e.to_string())),
        }
    };

    rsx! {
        section { class: "panel",
            h2 { if registering() { "Create an account" } else { "Sign in" } }
            if registering() {
                p { class: "warning", "{LOST_PASSWORD_NOTICE}" }
            }
            form { onsubmit: submit, autocomplete: "on",
                label {
                    "Username"
                    input {
                        name: "username",
                        autocomplete: "username",
                        value: "{username}",
                        oninput: move |e| username.set(e.value()),
                    }
                }
                label {
                    "Password"
                    input {
                        r#type: "password",
                        name: "password",
                        autocomplete: if registering() { "new-password" } else { "current-password" },
                        value: "{password}",
                        oninput: move |e| password.set(e.value()),
                    }
                }
                button { r#type: "submit", if registering() { "Register" } else { "Sign in" } }
            }
            if let Some(message) = error() {
                p { class: "error", "{message}" }
            }
            button {
                class: "link",
                onclick: move |_| {
                    error.set(None);
                    registering.toggle();
                },
                if registering() { "I already have an account" } else { "Create an account" }
            }
        }
    }
}

#[component]
fn SignedIn(account: Account, on_change: EventHandler<()>) -> Element {
    let mut tokens = use_resource(api::list_tokens);
    let mut fresh = use_signal(|| Option::<NewToken>::None);
    let mut name = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut confirm_delete = use_signal(|| false);

    let create = move |_| async move {
        match api::create_token(name()).await {
            Ok(minted) => {
                name.set(String::new());
                error.set(None);
                fresh.set(Some(minted));
                tokens.restart();
            }
            Err(e) => error.set(Some(e.to_string())),
        }
    };

    let username = account.username.clone();
    rsx! {
        section { class: "panel",
            h2 { "Signed in as {username}" }
            button {
                onclick: move |_| async move {
                    let _ = api::logout().await;
                    on_change.call(());
                },
                "Sign out"
            }
        }

        section { class: "panel",
            h2 { "API tokens" }
            p { class: "muted",
                "Tokens authenticate the account-gated API. Send one as "
                code { "Authorization: Bearer tdb_…" } "."
            }
            form {
                onsubmit: create,
                label {
                    "Name"
                    input {
                        value: "{name}",
                        placeholder: "laptop, ci, …",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                button { r#type: "submit", "Create token" }
            }
            if let Some(minted) = fresh() {
                div { class: "warning",
                    p { "Copy this now — it is shown once and never again." }
                    code { class: "token", "{minted.token}" }
                }
            }
            if let Some(message) = error() {
                p { class: "error", "{message}" }
            }
            match &*tokens.read() {
                Some(Ok(list)) if list.is_empty() => rsx! { p { class: "muted", "No tokens yet." } },
                Some(Ok(list)) => rsx! {
                    TokenTable { tokens: list.clone(), on_revoke: move |()| tokens.restart() }
                },
                Some(Err(e)) => rsx! { p { class: "error", "Could not list tokens: {e}" } },
                None => rsx! { p { class: "muted", "Loading…" } },
            }
        }

        Webhooks {}

        section { class: "panel",
            h2 { "Delete account" }
            p { class: "muted", "Deletes the account, every token on it, and every session. No undo." }
            if confirm_delete() {
                button {
                    class: "danger",
                    onclick: move |_| async move {
                        let _ = api::delete_account().await;
                        on_change.call(());
                    },
                    "Yes, delete {username} permanently"
                }
                button { class: "link", onclick: move |_| confirm_delete.set(false), "Cancel" }
            } else {
                button { class: "danger", onclick: move |_| confirm_delete.set(true), "Delete account" }
            }
        }
    }
}

#[component]
fn TokenTable(tokens: Vec<Token>, on_revoke: EventHandler<()>) -> Element {
    rsx! {
        table {
            thead {
                tr {
                    th { "Name" }
                    th { "Token" }
                    th { "Last used" }
                    th { "" }
                }
            }
            tbody {
                for token in tokens {
                    tr { key: "{token.id}", class: if token.revoked_at.is_some() { "revoked" } else { "" },
                        td { "{token.name}" }
                        td { class: "path", "{token.prefix_hint}…" }
                        td { if token.last_used_at.is_some() { "yes" } else { "never" } }
                        td {
                            if token.revoked_at.is_some() {
                                span { class: "muted", "revoked" }
                            } else {
                                button {
                                    class: "link",
                                    onclick: move |_| async move {
                                        let _ = api::revoke_token(token.id).await;
                                        on_revoke.call(());
                                    },
                                    "Revoke"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ webhooks

#[component]
fn Webhooks() -> Element {
    let mut hooks = use_resource(api::list_webhooks);
    let mut fresh = use_signal(|| Option::<NewWebhook>::None);
    let mut url = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);

    let create = move |_| async move {
        match api::create_webhook(url()).await {
            Ok(created) => {
                url.set(String::new());
                error.set(None);
                fresh.set(Some(created));
                hooks.restart();
            }
            Err(e) => error.set(Some(e.to_string())),
        }
    };

    rsx! {
        section { class: "panel",
            h2 { "Webhooks" }
            p { class: "muted",
                "Register an https URL to receive change events as signed POST batches "
                "(Standard Webhooks: verify the "
                code { "webhook-signature" } " header). Delivery is at-least-once and resumes "
                "from where it left off after a failure."
            }
            form {
                onsubmit: create,
                label {
                    "Endpoint URL"
                    input {
                        r#type: "url",
                        value: "{url}",
                        placeholder: "https://example.com/hooks/tenders",
                        oninput: move |e| url.set(e.value()),
                    }
                }
                button { r#type: "submit", "Add webhook" }
            }
            if let Some(created) = fresh() {
                div { class: "warning",
                    p { "Signing secret — copy it now, it is shown once:" }
                    code { class: "token", "{created.secret}" }
                }
            }
            if let Some(message) = error() {
                p { class: "error", "{message}" }
            }
            match &*hooks.read() {
                Some(Ok(list)) if list.is_empty() => rsx! { p { class: "muted", "No webhooks yet." } },
                Some(Ok(list)) => rsx! {
                    table {
                        thead {
                            tr {
                                th { "URL" }
                                th { "State" }
                                th { "Delivered" }
                                th { "" }
                            }
                        }
                        tbody {
                            for hook in list.clone() {
                                WebhookRow { hook, on_change: move |()| hooks.restart() }
                            }
                        }
                    }
                },
                Some(Err(e)) => rsx! { p { class: "error", "Could not list webhooks: {e}" } },
                None => rsx! { p { class: "muted", "Loading…" } },
            }
        }
    }
}

#[component]
fn WebhookRow(hook: Webhook, on_change: EventHandler<()>) -> Element {
    let disabled = hook.disabled_at.is_some();
    let state = if disabled {
        "disabled".to_string()
    } else if hook.consecutive_failures > 0 {
        format!("failing ×{}", hook.consecutive_failures)
    } else {
        "active".to_string()
    };
    let id = hook.id;
    rsx! {
        tr { key: "{hook.id}", class: if disabled { "revoked" } else { "" },
            td { class: "path", "{hook.url}" }
            td { "{state}" }
            td { "cursor {group(hook.last_delivered_cursor)}" }
            td {
                if disabled {
                    button {
                        class: "link",
                        onclick: move |_| async move {
                            let _ = api::enable_webhook(id).await;
                            on_change.call(());
                        },
                        "Enable"
                    }
                } else {
                    button {
                        class: "link",
                        onclick: move |_| async move {
                            let _ = api::disable_webhook(id).await;
                            on_change.call(());
                        },
                        "Disable"
                    }
                }
                " "
                button {
                    class: "link",
                    onclick: move |_| async move {
                        let _ = api::delete_webhook(id).await;
                        on_change.call(());
                    },
                    "Delete"
                }
            }
        }
    }
}

// ------------------------------------------------------------------- chrome

#[component]
fn Nav() -> Element {
    rsx! {
        header {
            h1 { "tender-db" }
            nav {
                a { href: "/", "Dashboard" }
                a { href: "/tenders", "Tenders" }
                a { href: "/account", "Account" }
                a { href: "/v1", "API" }
                a { href: "/docs", "Docs" }
            }
        }
    }
}

/// AGPL §13: a user interacting with this server over a network must be offered
/// the source of the exact version running. `/_source` is that offer.
#[component]
fn Footer() -> Element {
    rsx! {
        footer {
            a { href: "/docs", "API docs" }
            " · "
            a { href: "/_source", "Source (AGPL-3.0-or-later)" }
            " · "
            span { class: "muted",
                "Free software; the running version offers its own source, as the licence requires."
            }
        }
    }
}

// ------------------------------------------------------------------ display

/// A plain-English gloss for each quarantine reason code the importers emit
/// (the `reason` strings in `crates/ingest`). Quarantine is
/// content-no-profile-maps held whole (ADR-0004), so most reasons mean "we saw
/// something this notice's profile has no rule for yet", differing by where.
fn quarantine_reason_explained(reason: &str) -> &'static str {
    match reason {
        "unclaimed-content" => "Content no mapping profile claims — held whole rather than partly imported.",
        "unknown-customization" => "An eForms CustomizationID (SDK / national profile) with no mapping profile yet.",
        "unknown-field-code" => "A legacy text-era field code with no rule — ~all the one OC code on real EN notices (issue 35).",
        "unknown-root" => "The XML root was not a notice format we ingest — a non-notice sibling member.",
        "missing-publication-id" => "The member carried no publication id of its own, so it is not a distinct notice.",
        "unparsable-xml" => "Malformed XML — ~all 'XML with DTD detected', a whole DTD-bearing era rejected (issue 36).",
        "not-utf8" => "The notice file was not valid UTF-8 text and could not be read.",
        "unrepresentable-value" => "A value did not fit its expected type (a malformed amount, date, or code).",
        "ambiguous-field" => "A field matched more than one mapping, so its meaning was unclear.",
        "duplicate-section-id" => "Two sections shared an identifier that must be unique.",
        "unexpected-root" => "The XML root element was not the one this profile expects.",
        "empty-record" => "The record carried no header fields to parse.",
        "form-outside-form-section" => "A form element appeared outside its expected FORM_SECTION.",
        "form-without-category" => "A form gave no category to classify it by.",
        "no-original-form" => "The notice had no original-language form to key on.",
        "translation-structure-mismatch" => "A translation's structure did not line up with the original.",
        _ => "Content this notice's profile has no mapping for — held whole (ADR-0004).",
    }
}

/// The one-word coverage class for a quarantine reason (issue 30).
fn quarantine_class_label(reason: &str) -> &'static str {
    match quarantine_class(reason) {
        QuarantineClass::Actionable => "actionable",
        QuarantineClass::SuspectedGap => "suspected gap",
        QuarantineClass::Benign => "benign",
    }
}

/// Thin-space digit grouping, so six-figure notice counts stay readable.
fn group(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// A running duration in seconds, compact — for a job that is happening now, not
/// an age relative to the present.
fn duration(seconds: i64) -> String {
    let s = seconds.max(0);
    match s {
        0..60 => format!("{s} s"),
        60..3600 => format!("{} min {} s", s / 60, s % 60),
        _ => format!("{} h {} min", s / 3600, (s % 3600) / 60),
    }
}

/// An age in seconds, as the coarsest unit that still says something.
fn age(seconds: Option<i64>) -> String {
    let Some(s) = seconds else { return "never".to_owned() };
    match s {
        ..0 => "in the future".to_owned(),
        0..120 => format!("{s} s ago"),
        120..7200 => format!("{} min ago", s / 60),
        7200..172_800 => format!("{} h ago", s / 3600),
        _ => format!("{} days ago", s / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_group_and_ages_coarsen() {
        assert_eq!(group(0), "0");
        assert_eq!(group(999), "999");
        assert_eq!(group(1_234), "1\u{202f}234");
        assert_eq!(group(871_149), "871\u{202f}149");
        assert_eq!(group(-1_234), "-1\u{202f}234");

        assert_eq!(age(None), "never");
        assert_eq!(age(Some(5)), "5 s ago");
        assert_eq!(age(Some(600)), "10 min ago");
        assert_eq!(age(Some(10_800)), "3 h ago");
        assert_eq!(age(Some(root_days(3))), "3 days ago");

        assert_eq!(duration(-5), "0 s");
        assert_eq!(duration(5), "5 s");
        assert_eq!(duration(90), "1 min 30 s");
        assert_eq!(duration(3_725), "1 h 2 min");
    }

    fn root_days(n: i64) -> i64 {
        n * 86_400
    }

    #[test]
    fn coverage_folds_into_eras_newest_year_first() {
        let cell = |profile: &str, year: &str, held, published: Option<i64>| Coverage {
            source: "ted".into(),
            profile: profile.into(),
            year: year.into(),
            held,
            published,
            ratio: published.map(|p| held as f64 / p as f64),
            partial: false,
        };
        let eras = coverage_by_era(vec![
            cell("eforms", "2024", 50, Some(200)),
            cell("eforms", "2025", 30, Some(100)),
            cell("standard", "2019", 10, Some(40)),
        ]);
        assert_eq!(eras.len(), 2, "one era per (source, profile)");

        let eforms = &eras[0];
        assert_eq!(eforms.profile, "eforms");
        assert_eq!(eforms.held, 80);
        assert_eq!(eforms.published, Some(300));
        assert_eq!(eforms.years.first().map(|y| y.year.as_str()), Some("2025"));
        assert!((eforms.ratio.unwrap() - 80.0 / 300.0).abs() < 1e-9);
    }
}
