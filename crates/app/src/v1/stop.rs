//! Issue 438: how the API stops a read at its time limit — one mechanism for the REST
//! pools (issue 120) and `/v1/sql` (issue 425).
//!
//! **A timer and `interrupt()`, not turso's statement deadline.** Issue 425 found both in
//! turso 0.7.2 and used the deadline (`set_query_timeout`). But while a deadline is set,
//! turso evaluates `io.current_time_monotonic() >= deadline` before EVERY VDBE
//! instruction (`turso_core` `vdbe/mod.rs` `maybe_request_interrupt`, a `clock_gettime`
//! per call): a local A/B changing nothing but the timeout (0 vs 600 s) measured
//! covering-index GROUP BYs 2.9× slower, scans 1.7–2.7×, sorter group-bys 1.7–2.1× —
//! every read on the bounded pools, not just the slow ones. The interrupt FLAG is read
//! in the same check whether or not anyone ever sets it, so stopping a read through it
//! costs nothing until it fires. A borrow here is therefore:
//!
//! 1. registered — a clone of its connection (a shared `Arc`, no `Drop` of its own)
//!    published in a [`Running`] slot, with the instant it is due;
//! 2. timed — a task on a runtime that never runs a query sleeps until that instant,
//!    then calls `interrupt()` on the registered connection, and again every [`TICK`]
//!    while the read stays registered. turso ignores an interrupt when no statement is
//!    active, so the repeats are what catch a read that was between two statements
//!    when its time ran out: the next statement it starts is stopped within a tick;
//! 3. unregistered — when the borrow ends ([`Held`]'s `Drop`), under the lock every
//!    interrupt is sent under, BEFORE the reader goes back to the pool. So an interrupt
//!    can only ever land on the read that registered it, never on the next borrower.
//!
//! The same slot is what the isolated pool's `Abandon` interrupts when a caller stops
//! waiting (issue 120) — one way to stop a read, whichever reason fires.
//!
//! **Why a connection that was interrupted is never pooled again.** turso's
//! `interrupt()` checks "is a root statement active" and then sets the flag, two
//! separate steps; a statement ending between them clears the flag first and the
//! store lands after, leaving it set with nothing running — and the NEXT statement on
//! that connection fails at its first instruction. The window is nanoseconds and there
//! is no public call that clears the flag, so a borrow that sent any interrupt
//! discards its connection ([`store::Reader::discard`]) instead of returning it. Reads
//! stopped at their limit are rare (none on prod through 2026-09-27), and a fresh
//! connection costs its pragmas and a cold page cache — not a failed request.
//!
//! **What it bounds.** A BORROW, from the moment the reader is handed out: every
//! statement it runs, and the time between them. The engine deadline was per
//! statement, so a read of several statements could run several deadlines; this is
//! the "no read runs past the limit" the 503's wording always promised. A borrow that
//! is a batch of independent reads (the SSE diff) restarts the clock per item
//! ([`Held::restart_clock`]). Like the deadline, it cannot stop a single VDBE
//! instruction that runs long by itself — the flag is read between instructions.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use store::turso::Connection;

/// How often an expired deadline re-sends its interrupt while the read is still
/// registered. It bounds how long a read past its limit can go on by starting a new
/// statement after the previous interrupt was ignored (nothing was running).
pub const TICK: Duration = Duration::from_millis(50);

/// The connection a read is running on, published for exactly as long as the read
/// holds it, with the instant it is due — so whoever interrupts it (the deadline
/// timer, or a handler that stopped waiting) interrupts THAT read and never one a
/// later request borrowed the same pooled connection for.
#[derive(Default)]
pub struct Running(Mutex<Slot>);

#[derive(Default)]
struct Slot {
    conn: Option<Connection>,
    due: Option<Instant>,
    /// Whether `interrupt()` was sent to `conn` during this registration — so the
    /// borrow knows to discard it (see the module docs).
    interrupted: bool,
}

/// What the deadline timer found when it woke.
enum Fire {
    /// The read is over (unregistered): nothing left to stop.
    Gone,
    /// Not due yet — the clock was restarted, or the timer woke early.
    NotYet(Instant),
    /// Past due: an interrupt was sent.
    Sent,
}

impl Running {
    fn lock(&self) -> MutexGuard<'_, Slot> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn register(&self, conn: Connection, due: Instant) {
        *self.lock() = Slot { conn: Some(conn), due: Some(due), interrupted: false };
    }

    /// End the registration; whether any interrupt was sent during it.
    fn unregister(&self) -> bool {
        std::mem::take(&mut *self.lock()).interrupted
    }

    fn restart(&self, limit: Duration) {
        let mut slot = self.lock();
        if slot.conn.is_some() {
            slot.due = Some(Instant::now() + limit);
        }
    }

    /// Interrupt the registered read now, whatever its deadline — how a handler that
    /// stopped waiting ends the read it left behind. `false` when no read is
    /// registered (it already ended), in which case nothing was sent.
    pub fn interrupt(&self) -> bool {
        let mut slot = self.lock();
        let Some(conn) = slot.conn.as_ref() else { return false };
        let _ = conn.interrupt();
        slot.interrupted = true;
        true
    }

    /// The timer's step: under the one lock, "still registered AND past due" and the
    /// interrupt are a single decision, so a restarted clock or an ended read can
    /// never be interrupted by a timer that woke for an earlier instant.
    fn fire(&self) -> Fire {
        let mut slot = self.lock();
        let (Some(conn), Some(due)) = (slot.conn.as_ref(), slot.due) else { return Fire::Gone };
        if Instant::now() < due {
            return Fire::NotYet(due);
        }
        let _ = conn.interrupt();
        slot.interrupted = true;
        Fire::Sent
    }
}

/// A read time limit and the runtime whose timers enforce it.
///
/// The runtime must be one whose threads a query never occupies — a non-yielding
/// statement pins the thread it runs on, and a timer queued behind it would fire only
/// when the statement is over, which is too late by definition. The REST pools use a
/// dedicated current-thread runtime ([`spawn_deadline_runtime`]); `/v1/sql` uses its
/// own coordination runtime, whose workers hand every computation to a blocking
/// thread (issue 417).
pub struct Deadline {
    limit_nanos: AtomicU64,
    runtime: tokio::runtime::Handle,
}

impl Deadline {
    pub fn new(limit: Duration, runtime: tokio::runtime::Handle) -> Deadline {
        let deadline = Deadline { limit_nanos: AtomicU64::new(0), runtime };
        deadline.set_limit(limit);
        deadline
    }

    /// The limit each new borrow gets.
    pub fn limit(&self) -> Duration {
        Duration::from_nanos(self.limit_nanos.load(Ordering::Relaxed))
    }

    /// Change the limit; takes effect at the next borrow. How a test watches a read be
    /// stopped without running one for the production limit.
    pub fn set_limit(&self, limit: Duration) {
        self.limit_nanos.store(u64::try_from(limit.as_nanos()).unwrap_or(u64::MAX), Ordering::Relaxed);
    }

    /// Bound `reader`'s borrow: register it and start its clock now.
    pub fn hold(&self, reader: store::Reader) -> Held {
        self.hold_in(reader, Arc::default())
    }

    /// As [`hold`](Deadline::hold), registered in a slot the caller keeps too — the
    /// isolated pool's, so the handler that stops waiting can interrupt the read.
    pub fn hold_in(&self, reader: store::Reader, running: Arc<Running>) -> Held {
        let limit = self.limit();
        running.register((*reader).clone(), Instant::now() + limit);
        let slot = running.clone();
        let timer = self
            .runtime
            .spawn(async move {
                loop {
                    match slot.fire() {
                        Fire::Gone => return,
                        Fire::NotYet(due) => tokio::time::sleep_until(due.into()).await,
                        Fire::Sent => tokio::time::sleep(TICK).await,
                    }
                }
            })
            .abort_handle();
        Held { running, limit, timer, reader }
    }
}

/// A borrowed reader under a [`Deadline`]. Derefs to the connection.
///
/// `Drop` unregisters it FIRST — under the lock every interrupt is sent under, and
/// before the fields drop, i.e. before the [`store::Reader`] hands the connection back
/// to the pool — then cancels the timer. A borrow that sent any interrupt discards the
/// connection rather than pooling it (module docs).
pub struct Held {
    running: Arc<Running>,
    limit: Duration,
    timer: tokio::task::AbortHandle,
    reader: store::Reader,
}

impl Held {
    /// Give the read a fresh limit from now. For a borrow that is a batch of
    /// independent reads — the SSE diff classifies up to 500 changes on one borrow,
    /// and each change is one read, as each statement was one under the engine
    /// deadline this replaced.
    pub fn restart_clock(&self) {
        self.running.restart(self.limit);
    }
}

impl std::ops::Deref for Held {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        &self.reader
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        if self.running.unregister() {
            self.reader.discard();
        }
        self.timer.abort();
    }
}

/// A reader pool whose every borrow is bounded by a [`Deadline`] — the REST surface's
/// main pool (`AppState::readers`).
pub struct BoundedReaders {
    readers: Arc<store::Readers>,
    deadline: Deadline,
}

impl BoundedReaders {
    pub fn new(readers: Arc<store::Readers>, deadline: Deadline) -> BoundedReaders {
        BoundedReaders { readers, deadline }
    }

    /// Borrow a reader, waiting if all of them are busy; its clock starts when it is
    /// handed out.
    pub async fn get(&self) -> store::turso::Result<Held> {
        Ok(self.deadline.hold(self.readers.get().await?))
    }

    /// The limit each borrow gets.
    pub fn deadline(&self) -> Duration {
        self.deadline.limit()
    }

    /// Change the limit for later borrows — a test seam, like [`Deadline::set_limit`].
    pub fn set_deadline(&self, limit: Duration) {
        self.deadline.set_limit(limit);
    }
}

/// The runtime the REST pools' deadline timers run on: current-thread, on its own
/// parked thread (`read-deadline`), running nothing but timers — so a timer fires on
/// time however many API or isolated-pool threads are pinned inside a statement that
/// never yields. One per `AppState`, like the isolated and SQL runtimes, so each test
/// server owns its own.
pub fn spawn_deadline_runtime() -> tokio::runtime::Handle {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("read-deadline".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .expect("build the read-deadline runtime");
            tx.send(runtime.handle().clone()).expect("hand back the runtime handle");
            runtime.block_on(std::future::pending::<()>());
        })
        .expect("spawn the read-deadline runtime thread");
    rx.recv().expect("receive the read-deadline runtime handle")
}
