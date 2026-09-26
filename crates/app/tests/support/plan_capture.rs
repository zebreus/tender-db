//! Issue 428's statement capture: with `TENDER_PLAN_CAPTURE=<file>` set, every SQL
//! statement turso prepares while the suite runs is appended to `<file>` once, in
//! the `-- name:` block format `plan-probe plan` reads. Unset, this does nothing.
//!
//! The source is turso's own `Preparing: <sql>` debug event
//! (`turso_core::connection`), so the capture is the statement set the app
//! ACTUALLY issues — every read path, probe, job and fold step the suite drives —
//! not a list someone remembered (issue 114's rule: assert the artifact). Params
//! are not in the event; a plan does not depend on them, because turso compiles a
//! statement before anything is bound.

use std::collections::HashSet;
use std::io::Write;
use std::sync::{Mutex, Once};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

struct Capture {
    out: Mutex<(HashSet<String>, std::fs::File)>,
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
        let sql = sql.trim().to_owned();
        let Ok(mut guard) = self.out.lock() else { return };
        let (seen, file) = &mut *guard;
        if seen.insert(sql.clone()) {
            let _ = writeln!(file, "-- name: {}.{}\n{sql}\n", std::process::id(), seen.len());
        }
    }
    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
}

/// Install the capture if `TENDER_PLAN_CAPTURE` names a file. Idempotent.
pub fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let Ok(path) = std::env::var("TENDER_PLAN_CAPTURE") else { return };
        let file = std::fs::OpenOptions::new().create(true).append(true).open(&path).expect("open capture file");
        let _ = tracing::subscriber::set_global_default(Capture { out: Mutex::new((HashSet::new(), file)) });
    });
}
