//! Issue 120 / task 5: run the reads that *can* walk somewhere they cannot starve
//! the ones that can't — and, since issue 120's REST half of issue 425, stop them.
//!
//! **What stops a walk, and what does not.** A `tokio` timeout does not: measured on
//! the deployed engine, cold and warm, a 0.5 s budget over a 3.5 s streaming read
//! never fires, because `Statement::step` only yields on IO and a synchronous VFS
//! blocks inside the poll. turso's OWN per-statement deadline does — it is checked
//! before every VDBE instruction, so it stops a step that never yields
//! (`crates/store/tests/query_timeout_probe.rs`: a series aggregate, a nested-loop
//! join, a full sort and a GROUP BY all end in `Error::Interrupt` near the deadline,
//! and the connection is usable after). The published SDK hid it; the vendored one
//! (`crates/vendor/turso`, issue 425) passes it through, with `interrupt()`. So this
//! pool's connections carry [`super::STATEMENT_DEADLINE`] (set in [`IsolatedReads::new`]),
//! and a read the handler stops waiting for is interrupted, not left to run (see
//! [`Abandon`]).
//!
//! **Why the isolation stays.** A request whose filter shape can walk
//! ([`store::read::walks`]) is executed on a **dedicated runtime with its own reader
//! pool**, behind a small semaphore. The deadline bounds how LONG a walk holds a
//! slot; the isolation bounds WHO it holds it from — a walk still pins one of this
//! runtime's threads and one of this pool's connections for up to the deadline, and
//! the main API's readers keep serving. It is `/v1/sql`'s isolation (issue 17)
//! applied to the public collections.
//!
//! **What it does not do.** It does not reduce the work below the deadline; a walk
//! that fits under it is served in full, however slow. And it sheds rather than
//! queues: the (N+1)th concurrent walk-capable request gets a 503, which is a
//! deliberate trade for an unauthenticated endpoint.
//!
//! **Before the deadline, duration was unbounded — the measurements that drove it.**
//! A walk is finite (`/v1/lots?kind=Lot` over 13.2M real lots on a dedicated bed
//! completed in 231.6 s), but nothing bounded HOW long: on 2026-08-04 a 14-minute
//! burst of real traffic left a slot busy for ~85 minutes with every client gone, and
//! prod carried ~2.6 cores of abandoned-walk residue ~50 minutes after its last
//! request. Every abandoned request cost its FULL runtime, so a retrying client
//! accumulated load rather than replacing it (issue 120). Now an abandoned request
//! costs nothing past the moment it is abandoned, and an admitted one at most the
//! deadline per statement — so [`SLOTS`] is sized against `arrival rate × deadline`,
//! a number that exists.
//!
//! **The confinement guarantee is unverified until measured on this endpoint.** That
//! the burn really lands on these threads and not on the main API workers has never
//! been confirmed for `/v1/sql` either; it was assumed from the construction. The
//! worker threads are named [`ISOLATED_THREAD_NAME`] precisely so
//! `/proc/<pid>/task/*/stat` deltas can attribute CPU to them and settle it.

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::v1::{Collection, Item, PageOut, read_items, read_page};
use store::read::{Filter, Scope};
use store::turso::Connection;

/// Worker threads on the isolated read runtime. **Must equal [`SLOTS`]** — see the
/// assertion below.
///
/// This is what bounds the CPU a runaway walk can consume: at most this many threads,
/// and never the main API/SSE runtime's.
const ISOLATED_RUNTIME_THREADS: usize = SLOTS;

/// Reader connections dedicated to walk-capable reads, separate from the API's own.
/// A walk holds one of these until it ends or reaches the statement deadline, so the
/// point is that the connection it holds is never one the fast path needs.
const ISOLATED_READERS: usize = 4;

/// Concurrent walk-capable reads served before shedding. Tunable rather than a
/// literal: the right value is an observed 503 rate, not an arithmetic constant, and
/// the walk-capable shapes are rare enough that expected legitimate concurrency is
/// ~0–1. Note the semaphore is GLOBAL — these endpoints are unauthenticated, so there
/// is no token to key on, and per-IP is spoofable and belongs at the ingress. One
/// heavy client can therefore consume all the slots and shed a second legitimate one;
/// if the 503 rate ever shows that happening, keying is the lever.
const SLOTS: usize = 4;

// Slots and threads must agree, and it is a correctness property rather than tuning.
//
// An admitted walk occupies a thread until its statement ends — naturally, or at the
// engine deadline; nothing in between can take the thread back, because the step does
// not yield. With more slots than threads, the surplus admitted requests wait on the
// runtime behind walks nobody can pre-empt — which is precisely the queueing that
// `try_acquire`-to-shed exists to prevent, relocated inside the sandbox where it is
// harder to see. With more threads than slots the extra threads are simply idle.
//
// So one admitted request maps to one thread that can actually run it.
const _: () = assert!(
    SLOTS == ISOLATED_RUNTIME_THREADS,
    "every admitted walk must have a thread to run on: a surplus slot queues behind a \
     running statement, which is the starvation this module exists to prevent"
);

/// The isolated runtime's worker-thread name. **Load-bearing for verification, not
/// cosmetic**: the confinement guarantee is measured by attributing per-thread CPU
/// from `/proc/<pid>/task/*/stat`, which needs these threads distinguishable from the
/// main runtime's `tokio-runtime-worker`. Changing it breaks that measurement.
pub const ISOLATED_THREAD_NAME: &str = "slow-read-exec";

/// A walk-capable read could not be admitted: [`SLOTS`] are already in flight.
pub struct Shed;

/// The connection a spawned read is running on, published for exactly as long as the
/// read holds it — so a handler that gives up interrupts THAT read, never one a later
/// request borrowed the same pooled connection for.
#[derive(Default)]
struct Running(Mutex<Option<Connection>>);

/// A borrowed isolated reader, registered in [`Running`] while it is held. It
/// unregisters in `drop` BEFORE its fields drop — i.e. before the `Reader` hands the
/// connection back to the pool — and under the same lock [`Abandon`] interrupts with,
/// so an interrupt can only ever land on the read that registered it.
pub struct Held {
    running: Arc<Running>,
    reader: store::Reader,
}

impl Held {
    fn register(reader: store::Reader, running: Arc<Running>) -> Held {
        *running.0.lock().unwrap_or_else(|p| p.into_inner()) = Some((*reader).clone());
        Held { running, reader }
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
        *self.running.0.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }
}

/// Fires when the handler stops waiting — a client disconnect, the whole-request
/// deadline, or this future being dropped for any other reason. Before issue 120's
/// REST half it could only `abort()` the task, which took effect at an await point a
/// non-yielding walk never reaches, so an abandoned walk ran to its natural end with
/// nobody waiting (~85 min once on prod). Now it first INTERRUPTS the statement the
/// read is running (turso's `interrupt()`, safe from another thread), which fails the
/// step with `Error::Interrupt` at its next instruction; the task then ends and its
/// permit frees. Between two statements of one read nothing is running, the
/// interrupt is ignored (turso's rule), and `abort()` cancels the task at its next
/// await — or, if the next statement starts first, the engine deadline bounds it.
struct Abandon {
    task: tokio::task::AbortHandle,
    running: Arc<Running>,
    /// [`IsolatedReads::abandoned_total`]'s counter (issue 430).
    abandoned: Arc<AtomicU64>,
}

impl Drop for Abandon {
    fn drop(&mut self) {
        if let Some(conn) = self.running.0.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
            let _ = conn.interrupt();
            self.abandoned.fetch_add(1, Ordering::Relaxed);
        }
        self.task.abort();
    }
}

/// The isolated runtime, its pool and its slot count. Lives in `AppState`.
pub struct IsolatedReads {
    readers: Arc<store::Readers>,
    slots: Arc<Semaphore>,
    runtime: tokio::runtime::Handle,
    /// Reads refused because every slot was busy (issue 430).
    shed: AtomicU64,
    /// Reads whose caller gave up while they were still running (issue 430).
    abandoned: Arc<AtomicU64>,
}

impl IsolatedReads {
    pub fn new(db: &store::Db) -> store::turso::Result<IsolatedReads> {
        let readers = db.readers(ISOLATED_READERS)?;
        readers.bound_statements(super::STATEMENT_DEADLINE);
        Ok(IsolatedReads {
            readers,
            slots: Arc::new(Semaphore::new(SLOTS)),
            runtime: spawn_isolated_runtime(),
            shed: AtomicU64::new(0),
            abandoned: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Re-bound this pool's statements — how [`super::AppState::bound_statements`]
    /// lets a test watch the deadline fire without a 25 s query.
    pub fn bound_statements(&self, deadline: std::time::Duration) {
        self.readers.bound_statements(deadline);
    }

    /// This pool's statement deadline — see [`store::Readers::statement_deadline`].
    pub fn statement_deadline(&self) -> Option<std::time::Duration> {
        self.readers.statement_deadline()
    }

    /// How many slots are free. `/metrics` reports the complement as
    /// `tender_db_isolated_slots_busy` (issue 430), so occupancy is observable rather
    /// than inferred from 503s in a log.
    pub fn available(&self) -> usize {
        self.slots.available_permits()
    }

    /// Slots held right now — by a running read, or by a test.
    pub fn busy(&self) -> usize {
        SLOTS.saturating_sub(self.available())
    }

    /// Walk-capable reads refused since open because every slot was busy — the 503
    /// "too many expensive filtered reads in flight".
    pub fn shed_total(&self) -> u64 {
        self.shed.load(Ordering::Relaxed)
    }

    /// Reads whose caller stopped waiting (client gone, the 30 s layer) while the read
    /// was still running, since open. Each was interrupted rather than left to run —
    /// before issue 120 every one of them ran to its natural end with nobody waiting.
    pub fn abandoned_total(&self) -> u64 {
        self.abandoned.load(Ordering::Relaxed)
    }

    /// Take and hold `n` slots — how a test saturates the pool to prove routing
    /// and admission control without needing a genuinely slow read.
    pub fn hold_slots_for_test(&self, n: usize) -> Vec<OwnedSemaphorePermit> {
        (0..n)
            .map(|_| self.slots.clone().try_acquire_owned().expect("a free slot to hold"))
            .collect()
    }

    /// Run `read` on the isolated runtime over one of this pool's readers, or shed.
    /// Every walk-capable read goes through here; the named methods below only say
    /// which query.
    ///
    /// `try_acquire` rather than `acquire`: queueing behind a walk is how a slow
    /// endpoint becomes an unavailable one, and a caller who waits N×25s for a slot
    /// has been served worse than one refused immediately.
    ///
    /// The permit is moved INTO the spawned task, so it lives exactly as long as the
    /// query does — which is the load-bearing detail, not a stylistic one. If it were
    /// released when the CALLER gave up, a new request would be admitted while the
    /// abandoned one still burned a thread and concurrent burns would exceed SLOTS
    /// (measured before cancellation existed: ~2.77 cores still burning from clients
    /// that had exited minutes earlier). The permit tracks the QUERY, never the
    /// caller; what changed is that the query now ends when the caller does
    /// ([`Abandon`]) or at the statement deadline, whichever is first.
    pub async fn run<T, F, Fut>(&self, read: F) -> Result<store::turso::Result<T>, Shed>
    where
        T: Send + 'static,
        F: FnOnce(Held) -> Fut + Send + 'static,
        Fut: Future<Output = store::turso::Result<T>> + Send + 'static,
    {
        let Ok(permit) = self.slots.clone().try_acquire_owned() else {
            self.shed.fetch_add(1, Ordering::Relaxed);
            return Err(Shed);
        };
        let readers = self.readers.clone();
        let running = Arc::new(Running::default());
        let registered = running.clone();
        let handle = self.runtime.spawn(async move {
            let _permit = permit;
            let reader = Held::register(readers.get().await?, registered);
            read(reader).await
        });
        let _abandon = Abandon { task: handle.abort_handle(), running, abandoned: self.abandoned.clone() };
        match handle.await {
            Ok(result) => Ok(result),
            // Aborted because we stopped waiting, or a panic on the isolated runtime.
            // Either way this request has no answer; report it as shed rather than
            // inventing a database error.
            Err(_) => Err(Shed),
        }
    }

    /// The full read of a walk-capable filter, on the isolated runtime — see [`Self::run`].
    pub async fn read(
        &self,
        collection: Collection,
        filter: Filter,
        scope: Scope,
    ) -> Result<store::turso::Result<Vec<Item>>, Shed> {
        self.run(move |reader| async move { read_items(collection, &reader, &filter, scope).await }).await
    }

    /// The REST list's id-ordered page with the fallback walk bounded (issue 408 (b)),
    /// over [`read_page`].
    pub async fn read_page(
        &self,
        collection: Collection,
        filter: Filter,
        cursor: String,
        limit: i64,
        band: i64,
    ) -> Result<store::turso::Result<PageOut>, Shed> {
        self.run(move |reader| async move { read_page(collection, &reader, &filter, &cursor, limit, band).await })
            .await
    }

    /// The name-ordered organization search when a companion filter rides along
    /// (issue 217-B): `country`/`kind` beside the name range makes the planner drive
    /// from the COMPANION's index and scan its whole slice (measured 4.9 s for
    /// country=DE over 3.85M rows) — correct, but a walk, so it must not hold a
    /// main-pool reader. Bare-prefix searches stay on the main pool (1.7 ms, measured).
    pub async fn read_org_named(
        &self,
        filter: Filter,
        prefix: String,
        cursor: Option<(String, i64)>,
        limit: i64,
    ) -> Result<store::turso::Result<Vec<store::read::OrganizationRow>>, Shed> {
        self.run(move |reader| async move {
            store::read::organizations_by_name(&reader, &filter, &prefix, cursor, limit).await
        })
        .await
    }

    /// The ordered Tender list (issue 216).
    pub async fn read_ordered(
        &self,
        filter: Filter,
        order: store::read::HeadOrder,
        desc: bool,
        cursor: Option<(i64, i64)>,
        limit: i64,
    ) -> Result<store::turso::Result<Vec<store::read::TenderRow>>, Shed> {
        self.run(move |reader| async move {
            store::read::tenders_ordered(&reader, &filter, order, desc, cursor, limit).await
        })
        .await
    }
}

/// A runtime dedicated to walk-capable reads, owned by a parked thread so it lives for
/// the process and is never dropped in an async context (which would panic).
///
/// One per `AppState` rather than a process global, matching `/v1/sql`: a server owns
/// its own isolation, which is what lets tests run on independent threads instead of
/// contending for a shared pool.
fn spawn_isolated_runtime() -> tokio::runtime::Handle {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("slow-read-runtime".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(ISOLATED_RUNTIME_THREADS)
                .thread_name(ISOLATED_THREAD_NAME)
                .enable_all()
                .build()
                .expect("build the isolated read runtime");
            tx.send(runtime.handle().clone()).expect("hand back the runtime handle");
            runtime.block_on(std::future::pending::<()>());
        })
        .expect("spawn the isolated read runtime thread");
    rx.recv().expect("receive the isolated read runtime handle")
}
