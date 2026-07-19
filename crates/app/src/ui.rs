//! The dashboard: data coverage, data quality, and the account lifecycle.
//!
//! This is the product's face (CONTEXT.md, "Four faces"). It renders numbers the
//! server has already resolved — the client never computes a coverage ratio or
//! reads its own clock for an age, so what a user sees is what the server
//! measured.

use crate::api;
use dioxus::prelude::*;
use model::account::LOST_PASSWORD_NOTICE;
use model::dashboard::{Coverage, Quarantined};
use model::ingestion::{Ingestion, JobProgress, JobRun};
use model::{Account, NewToken, NewWebhook, Token, Webhook};

/// How often the dashboard re-measures. Deliberately a plain poll: the change
/// feed's SSE plumbing serves API clients, and a dashboard that refreshes twice
/// a minute needs none of it.
const REFRESH_SECONDS: u64 = 15;

// ---------------------------------------------------------------- dashboard

#[component]
pub fn DashboardPage() -> Element {
    let mut data = use_server_future(api::dashboard)?;

    // Poll rather than subscribe. `futures_timer::Delay` is the one timer that
    // works both in the wasm client and during a native server render.
    use_future(move || async move {
        loop {
            futures_timer::Delay::new(std::time::Duration::from_secs(REFRESH_SECONDS)).await;
            data.restart();
        }
    });

    let value = data.read();
    rsx! {
        main {
            Nav {}
            IngestionPanel {}
            match &*value {
                Some(Ok(d)) => rsx! {
                    section { class: "panel",
                        h2 { "Contents" }
                        dl { class: "counts",
                            for c in d.counts.clone() {
                                dt { key: "{c.label}", "{c.label}" }
                                dd { "{group(c.value)}" }
                            }
                            dt { "change cursor" }
                            dd { "{group(d.cursor)}" }
                        }
                    }

                    section { class: "panel",
                        h2 { "Import lag" }
                        p { class: "muted",
                            "Fetching and processing are separate stages, so they go stale separately."
                        }
                        dl { class: "counts",
                            dt { "newest fetched package" }
                            dd { "{age(d.lag.fetch_age)}" }
                            dt { "newest ingested notice" }
                            dd { "{age(d.lag.notice_age)}" }
                        }
                    }

                    QuarantinePanel {
                        total: d.quarantine_total,
                        reasons: d.quarantine_by_reason.iter().map(|c| (c.label.clone(), c.value)).collect::<Vec<_>>(),
                        recent: d.quarantine_recent.clone(),
                    }

                    CoveragePanel { rows: d.coverage.clone() }
                },
                Some(Err(e)) => rsx! { p { class: "error", "Could not measure: {e}" } },
                None => rsx! { p { class: "muted", "Measuring…" } },
            }
            Footer {}
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
    let mut state = use_server_future(api::ingestion)?;
    use_future(move || async move {
        loop {
            futures_timer::Delay::new(std::time::Duration::from_secs(INGESTION_REFRESH_SECONDS))
                .await;
            state.restart();
        }
    });

    let value = state.read();
    let Some(Ok(ingestion)) = &*value else {
        return rsx! {
            section { class: "panel",
                h2 { "Ingestion" }
                match &*value {
                    Some(Err(e)) => rsx! { p { class: "error", "Could not read the importer: {e}" } },
                    _ => rsx! { p { class: "muted", "Reading…" } },
                }
            }
        };
    };
    let Ingestion { current, queued, recent } = ingestion.clone();

    rsx! {
        section { class: "panel",
            h2 { "Ingestion" }
            p { class: "muted",
                "The importer runs inside the server (ADR-0005): one job at a time, the "
                "readers serving throughout. Operators drive it through the "
                code { "/admin" } " API."
            }

            match current {
                Some(job) => rsx! { RunningJob { job } },
                None => rsx! { p { class: "muted", "Idle — no job running." } },
            }

            if !queued.is_empty() {
                h3 { "Queued" }
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
fn RunningJob(job: JobProgress) -> Element {
    rsx! {
        div { class: "running",
            p { class: "headline", "{job.kind} — {job.params}" }
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
            p { class: "muted", "{group(job.notices as i64)} notices written" }
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

#[component]
fn CoveragePanel(rows: Vec<Coverage>) -> Element {
    rsx! {
        section { class: "panel",
            h2 { "Coverage" }
            p { class: "muted",
                "Notices held per source, mapping profile and publication year, against what that "
                "year is known to have published (docs/research/ted-access-channels.md §6)."
            }
            if rows.is_empty() {
                p { class: "muted", "Nothing ingested yet — every year is at 0 %." }
            } else {
                table {
                    thead {
                        tr {
                            th { "Year" }
                            th { "Source" }
                            th { "Profile" }
                            th { class: "num", "Held" }
                            th { class: "num", "Published" }
                            th { class: "num", "Coverage" }
                        }
                    }
                    tbody {
                        for row in rows {
                            tr { key: "{row.year}-{row.source}-{row.profile}",
                                td { "{row.year}" }
                                td { "{row.source}" }
                                td { "{row.profile}" }
                                td { class: "num", "{group(row.held)}" }
                                td { class: "num",
                                    match row.published {
                                        Some(n) => group(n),
                                        None => "—".to_owned(),
                                    }
                                }
                                td { class: "num",
                                    match row.ratio {
                                        Some(r) => format!("{:.2} %{}", r * 100.0, if row.partial { " *" } else { "" }),
                                        None => "—".to_owned(),
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

#[component]
fn QuarantinePanel(total: i64, reasons: Vec<(String, i64)>, recent: Vec<Quarantined>) -> Element {
    rsx! {
        section { class: "panel",
            h2 { "Quarantine" }
            p { class: "muted",
                "A notice with content no profile maps is held whole, never partly imported "
                "(ADR-0004). This count is the headline data-quality metric."
            }
            p { class: "headline", "{group(total)}" }
            if !reasons.is_empty() {
                dl { class: "counts",
                    for (reason, count) in reasons {
                        dt { key: "{reason}", "{reason}" }
                        dd { "{group(count)}" }
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
                                    td { title: entry.detail.clone().unwrap_or_default(), "{entry.reason}" }
                                    td { "{entry.profile.clone().unwrap_or_else(|| \"—\".into())}" }
                                    td { class: "path", "{entry.member_path}" }
                                }
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
            "tender-db is free software under the "
            a { href: "/_source", "AGPL-3.0-or-later — get the source of this running version" }
            "."
        }
    }
}

// ------------------------------------------------------------------ display

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
    }

    fn root_days(n: i64) -> i64 {
        n * 86_400
    }
}
