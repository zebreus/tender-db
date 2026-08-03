//! Issue 120 / task 5: run the reads that *can* walk somewhere they cannot starve
//! the ones that can't.
//!
//! turso has no `interrupt()`, and no timeout substitutes for one — measured on the
//! deployed engine, cold and warm: a 0.5 s budget over a 3.5 s streaming read never
//! fires, because `Statement::step` only yields on IO and a synchronous VFS blocks
//! inside the poll. So a walk **cannot be stopped once started**, and the only
//! available guarantee is about *who it hurts*, not *how much work it does*.
//!
//! That is what this module buys. A request whose filter shape can walk
//! ([`store::read::walks`]) is executed on a **dedicated runtime with its own reader
//! pool**, behind a small semaphore. A runaway then pins at most this runtime's
//! threads and this pool's connections; the main API's readers keep serving. It is
//! `/v1/sql`'s isolation (issue 17) applied to the public collections — and worth
//! being precise that it is the *isolation* being reused, not the timeout, which that
//! endpoint's own documentation used to claim worked and which does not.
//!
//! **What it does not do.** It does not reduce the work; the box still does it. It
//! does not bound how long any one request takes. And it sheds rather than queues: the
//! (N+1)th concurrent walk-capable request gets a 503, which is a deliberate trade for
//! an unauthenticated endpoint whose worst case is a multi-minute walk.
//!
//! **Recovery is bounded, and that is measured rather than assumed.** A walk is finite:
//! `/v1/lots?kind=Lot` over 13.2M real lots on a dedicated bed completed in **231.6 s**.
//! So a slot always frees on its own and no intervention is needed — [`SLOTS`] is a
//! tuning knob, not a countdown to permanent unavailability. This was worth measuring
//! rather than reasoning: "a finite scan must terminate" is plausible, but the cost
//! that could have hidden unboundedness was the top-level SORTER over the matched set,
//! not the scan.
//!
//! **But bounded is not small, and that is what [`SLOTS`] must be sized against.**
//! Nothing cancels on client disconnect, so **every abandoned request costs its FULL
//! runtime** of executor capacity with nobody waiting for the answer. A client that
//! retries and gives up — the natural behaviour of a browser or a retrying script —
//! ACCUMULATES load rather than replacing it; prod was measured carrying ~2.6 cores of
//! exactly this residue ~50 minutes after every client had gone. So size against
//! **arrival rate × full query runtime**, never against concurrent clients, and note
//! that an ingress rate limit bounds arrival and does nothing about in-flight
//! accumulation.
//!
//! The 231.6 s figure is a **lower bound on prod's per-query cost, not an estimate** —
//! the bed is 100% `kind='lot'`, deliberately the dense worst case, while prod's table
//! is larger and differently distributed. The termination conclusion transfers; the
//! number does not, and nothing should be sized off it.
//!
//! **The guarantee is unverified until measured on this endpoint.** Confinement — that
//! the burn really lands on these threads and not on the main API workers — has never
//! been confirmed for `/v1/sql` either; it was assumed from the construction. The
//! worker threads are named [`ISOLATED_THREAD_NAME`] precisely so
//! `/proc/<pid>/task/*/stat` deltas can attribute CPU to them and settle it.

use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::v1::{Collection, Item, read_items};
use store::read::{Filter, Scope};

/// Worker threads on the isolated read runtime. **Must equal [`SLOTS`]** — see the
/// assertion below.
///
/// This is what bounds the CPU a runaway walk can consume: at most this many threads,
/// and never the main API/SSE runtime's.
const ISOLATED_RUNTIME_THREADS: usize = SLOTS;

/// Reader connections dedicated to walk-capable reads, separate from the API's own.
/// A walk holds one of these to completion — it cannot be cancelled — so the point is
/// that the connection it holds is never one the fast path needs.
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
// A walk cannot be cancelled, so an admitted request occupies a thread until the query
// finishes NATURALLY. With more slots than threads, the surplus admitted requests wait
// on the runtime behind walks nobody can stop — which is precisely the queueing that
// `try_acquire`-to-shed exists to prevent, relocated inside the sandbox where it is
// harder to see. With more threads than slots the extra threads are simply idle.
//
// So one admitted request maps to one thread that can actually run it.
const _: () = assert!(
    SLOTS == ISOLATED_RUNTIME_THREADS,
    "every admitted walk must have a thread to run on: a surplus slot queues behind an \
     uncancellable query, which is the starvation this module exists to prevent"
);

/// The isolated runtime's worker-thread name. **Load-bearing for verification, not
/// cosmetic**: the confinement guarantee is measured by attributing per-thread CPU
/// from `/proc/<pid>/task/*/stat`, which needs these threads distinguishable from the
/// main runtime's `tokio-runtime-worker`. Changing it breaks that measurement.
pub const ISOLATED_THREAD_NAME: &str = "slow-read-exec";

/// A walk-capable read could not be admitted: [`SLOTS`] are already in flight.
pub struct Shed;

/// Abandon the spawned read when the handler stops waiting — a client disconnect, or
/// this future being dropped. It cannot stop the turso work (nothing can), but it
/// releases everything this side holds instead of waiting on a walk nobody wants.
struct AbortOnDrop(tokio::task::AbortHandle);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// The isolated runtime, its pool and its slot count. Lives in `AppState`.
pub struct IsolatedReads {
    readers: Arc<store::Readers>,
    slots: Arc<Semaphore>,
    runtime: tokio::runtime::Handle,
}

impl IsolatedReads {
    pub fn new(db: &store::Db) -> store::turso::Result<IsolatedReads> {
        Ok(IsolatedReads {
            readers: db.readers(ISOLATED_READERS)?,
            slots: Arc::new(Semaphore::new(SLOTS)),
            runtime: spawn_isolated_runtime(),
        })
    }

    /// How many slots are free — for the readiness/debug surface, so the shed rate is
    /// observable rather than inferred from 503s in a log.
    pub fn available(&self) -> usize {
        self.slots.available_permits()
    }

    /// Run a walk-capable read on the isolated runtime, or shed.
    ///
    /// `try_acquire` rather than `acquire`: queueing behind a walk that cannot be
    /// cancelled is how a slow endpoint becomes an unavailable one, and a caller who
    /// waits N×10s for a slot has been served worse than one refused immediately.
    pub async fn read(
        &self,
        collection: Collection,
        filter: Filter,
        scope: Scope,
    ) -> Result<store::turso::Result<Vec<Item>>, Shed> {
        let permit: OwnedSemaphorePermit =
            self.slots.clone().try_acquire_owned().map_err(|_| Shed)?;
        let readers = self.readers.clone();
        // The permit is moved INTO the spawned task, so it lives exactly as long as the
        // query does — which is the load-bearing detail, not a stylistic one.
        //
        // `AbortOnDrop` below fires when this handler stops waiting (client disconnect,
        // or an outer timeout). It does NOT stop the query: turso has no interrupt and
        // `step` does not yield on a cache-resident scan, so `abort` only takes effect
        // at an await point the task will not reach until the query returns. The task's
        // future therefore cannot be dropped mid-query, so `_permit` cannot be released
        // mid-query either.
        //
        // That is the property that keeps the cap honest. If the permit were released
        // when the CALLER gave up, a new request would be admitted while the abandoned
        // one is still burning a thread, and concurrent burns would exceed SLOTS —
        // measured: ~2.77 cores still burning from clients that had exited minutes
        // earlier. The permit tracks the QUERY, never the caller.
        let handle = self.runtime.spawn(async move {
            let _permit = permit;
            let reader = readers.get().await?;
            read_items(collection, &reader, &filter, scope).await
        });
        let _abandon = AbortOnDrop(handle.abort_handle());
        match handle.await {
            Ok(result) => Ok(result),
            // Aborted because we stopped waiting, or a panic on the isolated runtime.
            // Either way this request has no answer; report it as shed rather than
            // inventing a database error.
            Err(_) => Err(Shed),
        }
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
