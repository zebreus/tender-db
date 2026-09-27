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
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

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

impl Subscriber for Capture {
    fn enabled(&self, meta: &Metadata<'_>) -> bool {
        meta.is_event() && *meta.level() == Level::DEBUG && meta.target() == "turso_core::connection"
    }
    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }
    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, event: &Event<'_>) {
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
    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
}

/// Install the capture if `TENDER_PLAN_CAPTURE` names a file; otherwise do
/// nothing. Idempotent. Returns whether it is installed, so the caller can log it.
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
        let state = State { seen: HashSet::new(), file, full: false };
        if tracing::subscriber::set_global_default(Capture { out: Mutex::new(state) }).is_ok() {
            INSTALLED.store(true, Ordering::Release);
        }
    });
    INSTALLED.load(Ordering::Acquire)
}
