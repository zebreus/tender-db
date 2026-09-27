//! Statement capture (issues 428/429): with `TENDER_PLAN_CAPTURE=<file>` set, every
//! distinct SQL statement turso prepares in this process is appended to `<file>`
//! once, in the `-- name:` block format `plan-probe plan` reads. Unset — the
//! default, and production's normal state — nothing is installed and nothing costs.
//!
//! The source is turso's own `Preparing: <sql>` debug event
//! (`turso_core::connection`), so the capture is the statement set the process
//! ACTUALLY issues — every read path, job, census and fold step, and on the box the
//! analysts' `/v1/sql` queries too — not a list someone remembered (issue 114's
//! rule: assert the artifact). Params are not in the event; a plan does not depend
//! on them, because turso compiles a statement before anything is bound.
//!
//! Why the server carries it and not only the tests: issue 428 measured ANALYZE
//! against the statements the app's integration suites prepare, and those never
//! run the supervisor's weekly jobs (the data-quality run, the censuses, the org
//! merges). A week of production capture is the population those jobs actually
//! issue.
//!
//! Bounded on purpose: a statement that inlines a value would make every call
//! "distinct", so after [`MAX_STATEMENTS`] the capture stops recording and says so
//! once, and a statement longer than [`MAX_SQL_BYTES`] is skipped. Only the one
//! callsite is enabled, so every other turso event stays disabled at its callsite.

use std::collections::HashSet;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, Once};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::{EnvFilter, Layer, Registry};

/// Distinct statements recorded before the capture stops (and says so once).
pub const MAX_STATEMENTS: usize = 20_000;
/// Longer statements are skipped — a generated `IN (…)` list of that size is a
/// shape, not a new plan.
pub const MAX_SQL_BYTES: usize = 64 * 1024;

struct Capture {
    out: Mutex<State>,
}

struct State {
    seen: HashSet<String>,
    file: std::fs::File,
    full: bool,
}

struct Message(Option<String>);

impl Visit for Message {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = Some(format!("{value:?}"));
        }
    }
}

impl<S: Subscriber> Layer<S> for Capture {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let mut message = Message(None);
        event.record(&mut message);
        let Some(sql) = message.0.as_deref().and_then(|m| m.strip_prefix("Preparing: ")) else {
            return;
        };
        let sql = sql.trim();
        if sql.len() > MAX_SQL_BYTES {
            return;
        }
        let Ok(mut state) = self.out.lock() else { return };
        if state.full || state.seen.contains(sql) {
            return;
        }
        if state.seen.len() >= MAX_STATEMENTS {
            state.full = true;
            let _ = writeln!(state.file, "-- capture stopped: {MAX_STATEMENTS} distinct statements recorded\n");
            return;
        }
        state.seen.insert(sql.to_owned());
        let n = state.seen.len();
        let _ = writeln!(state.file, "-- name: {}.{n}\n{sql}\n", std::process::id());
    }
}

/// Install the capture if `TENDER_PLAN_CAPTURE` names a file; otherwise do
/// nothing, and the process keeps whatever logger it would have had. Idempotent.
/// Returns whether it is installed, so the caller can log it.
///
/// MUST run before anything else sets the global subscriber: tracing has exactly
/// one, and `dioxus::server::serve` installs its fmt logger first thing — the first
/// deploy of this (bba7804, 2026-09-27) called it inside the serve closure, lost
/// that race, and recorded nothing. So the capture brings the logger with it: a
/// registry with the same fmt layer dioxus-logger would build (INFO in release,
/// DEBUG in debug, `RUST_LOG` honoured, `hyper_util` at warn) beside the capture
/// layer, each with its own filter — dioxus then sees a subscriber is set and
/// skips its own, and no log line is lost for the week the capture runs.
pub fn install_from_env() -> bool {
    static ONCE: Once = Once::new();
    static INSTALLED: AtomicBool = AtomicBool::new(false);
    ONCE.call_once(|| {
        let Ok(path) = std::env::var("TENDER_PLAN_CAPTURE") else { return };
        if path.is_empty() {
            return;
        }
        let mut options = std::fs::OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let file = match options.open(&path) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("[plan-capture] cannot open {path}: {e} — capture off");
                return;
            }
        };
        let level = if cfg!(debug_assertions) { Level::DEBUG } else { Level::INFO };
        let mut logs = EnvFilter::builder().with_default_directive(level.into()).from_env_lossy();
        if let Ok(quiet) = "hyper_util=warn".parse() {
            logs = logs.add_directive(quiet);
        }
        // Only turso's prepare event reaches the capture; every other callsite it
        // leaves disabled.
        let prepares = tracing_subscriber::filter::filter_fn(|meta| {
            meta.is_event() && *meta.level() == Level::DEBUG && meta.target() == "turso_core::connection"
        });
        let capture = Capture { out: Mutex::new(State { seen: HashSet::new(), file, full: false }) };
        let subscriber = Registry::default()
            .with(tracing_subscriber::fmt::layer().with_filter(logs))
            .with(capture.with_filter(prepares));
        match tracing::subscriber::set_global_default(subscriber) {
            Ok(()) => INSTALLED.store(true, Ordering::Release),
            Err(e) => eprintln!("[plan-capture] a global subscriber is already set ({e}) — capture off"),
        }
    });
    INSTALLED.load(Ordering::Acquire)
}
