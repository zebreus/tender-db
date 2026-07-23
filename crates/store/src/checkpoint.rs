//! WAL maintenance — bounding the write-ahead log during bulk loads (issue 42).
//!
//! Turso *does* autocheckpoint PASSIVE at a WAL size threshold, but two things
//! defeat it during a heavy load (live prod evidence, deploy 546189d): (1) a
//! long-lived reader **snapshot** pins the WAL so the autocheckpoint cannot fold
//! past it, and the WAL balloons for the life of that snapshot — the ~13 GB
//! spikes, prime suspect the dashboard coverage refresher's multi-minute scan
//! over millions of rows every 60 s; and (2) PASSIVE reuses the WAL file in
//! place — it never shrinks it on disk, so a high-water mark persists. The fix
//! is to TRUNCATE at the natural writer-idle points (package boundaries in the
//! process loop, batch boundaries in the projection): TRUNCATE returns the file
//! space PASSIVE leaves, and forces the reclaim the autocheckpoint may be blocked
//! from doing.
//!
//! A checkpoint can only reclaim frames older than the oldest **live reader
//! snapshot**: in WAL mode a reader mid-transaction pins every frame its snapshot
//! needs, and the checkpoint stops there (`busy = 1`). An *idle* pooled reader
//! (between queries, not in a transaction) holds no snapshot, so it does not pin
//! the WAL — this is verified in the tests below, and is why no reader-side
//! recycling is needed (only a legitimately-running scan pins it, and that cannot
//! be recycled away).

use crate::{Db, int};

/// How aggressively to checkpoint (`PRAGMA wal_checkpoint(<mode>)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointMode {
    /// Never blocks: fold back as many frames as no reader needs, then return.
    /// Once every frame is folded and no reader pins the tail, turso restarts the
    /// WAL at the front, so it stops growing — the cheap per-package cadence.
    Passive,
    /// Like passive, then force the next write to restart the WAL at the front.
    /// Waits (up to `busy_timeout`) for readers of the current frames.
    Restart,
    /// Restart plus truncate the `-wal` file to zero bytes — the only mode that
    /// shrinks the file on disk. Waits for readers like restart.
    Truncate,
}

impl CheckpointMode {
    fn pragma(self) -> &'static str {
        match self {
            CheckpointMode::Passive => "PRAGMA wal_checkpoint(PASSIVE)",
            CheckpointMode::Restart => "PRAGMA wal_checkpoint(RESTART)",
            CheckpointMode::Truncate => "PRAGMA wal_checkpoint(TRUNCATE)",
        }
    }
}

/// The result of one checkpoint — `PRAGMA wal_checkpoint` returns one row of
/// `(busy, log, checkpointed)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpointed {
    /// `true` when a reader (or writer) held frames the checkpoint could not pass
    /// — the reclaim was partial. Not an error: the next checkpoint retries.
    pub busy: bool,
    /// Frames in the WAL the checkpoint considered.
    pub wal_frames: i64,
    /// Frames actually written back into the main database file.
    pub checkpointed: i64,
}

/// Run a checkpoint on an already-held writer connection. `PRAGMA wal_checkpoint`
/// returns rows, so it goes through `query` and the single row is drained.
pub(crate) async fn checkpoint_on(
    conn: &turso::Connection,
    mode: CheckpointMode,
) -> turso::Result<Checkpointed> {
    let mut rows = conn.query(mode.pragma(), ()).await?;
    let out = match rows.next().await? {
        Some(row) => Checkpointed {
            busy: int(&row, 0) != 0,
            wal_frames: int(&row, 1),
            checkpointed: int(&row, 2),
        },
        None => Checkpointed { busy: false, wal_frames: 0, checkpointed: 0 },
    };
    while rows.next().await?.is_some() {}
    Ok(out)
}

impl Db {
    /// Checkpoint the WAL, acquiring the single writer. Callers that already hold
    /// the writer (the projection batch loop) use [`checkpoint_on`] directly.
    /// Cheap when the WAL is small; the process loop calls it at package
    /// boundaries where the writer is otherwise idle, so it never makes a read
    /// queue behind a longer-held writer (issue 20 stays true).
    pub async fn checkpoint(&self, mode: CheckpointMode) -> turso::Result<Checkpointed> {
        let conn = self.conn().await;
        checkpoint_on(&conn, mode).await
    }

    /// Size of the `-wal` sidecar in bytes, or `None` if it is absent (a freshly
    /// checkpointed/closed database has no WAL). Surfaced to monitoring so a
    /// runaway WAL is visible (issue 42).
    pub fn wal_bytes(&self) -> Option<u64> {
        std::fs::metadata(format!("{}-wal", self.path)).ok().map(|m| m.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Db, Fetch, Notice, Parse};

    /// [DEBUG-wal01] Grow the WAL the way the PROCESS job does: N notices, each
    /// its own explicit `BEGIN IMMEDIATE … COMMIT` (record_notice), NOT autocommit
    /// like `grow_wal`/`record_fetch`. This is the exact transaction shape the
    /// backfill drives through the single writer.
    async fn grow_wal_notices(db: &Db, n: i64) {
        db.record_fetch(&Fetch {
            source: "ted".into(),
            kind: "daily".into(),
            period: "2026-00001".into(),
            url: "u".repeat(64),
            sha256: "a".repeat(64),
            bytes: 1,
            fetched_at: 1,
            path: "ted/daily/2026-00001.tar.gz".into(),
        })
        .await
        .unwrap();
        for i in 0..n {
            db.record_notice(
                &Notice {
                    source: "ted".into(),
                    publication_id: format!("pub-{i:07}"),
                    content_hash: format!("{i:064}"),
                    profile: "eforms".into(),
                    declared_version: None,
                    fetch_id: 1,
                    member_path: format!("m/{i}"),
                    ingested_at: i,
                    published_at: None,
                    dispatched_at: None,
                },
                &Parse::Pending,
            )
            .await
            .unwrap();
        }
    }

    /// [DEBUG-wal01] Insert notices with ids in `[start, start+n)` — unique across
    /// chunks, so repeated calls actually grow (record_notice dedups on identity).
    async fn insert_notices(db: &Db, start: i64, n: i64) {
        for i in start..start + n {
            db.record_notice(
                &Notice {
                    source: "ted".into(),
                    publication_id: format!("pub-{i:09}"),
                    content_hash: format!("{i:064}"),
                    profile: "eforms".into(),
                    declared_version: None,
                    fetch_id: 1,
                    member_path: format!("m/{i}"),
                    ingested_at: i,
                    published_at: None,
                    dispatched_at: None,
                },
                &Parse::Pending,
            )
            .await
            .unwrap();
        }
    }

    /// [DEBUG-wal01] The faithful prod analog, no deploy needed: the single writer
    /// walks "packages" (chunks of notices) with a per-package TRUNCATE at each
    /// boundary — turso's DEFAULT autocheckpoint left ON, exactly like prod — WHILE
    /// a background task periodically borrows a pooled reader and runs a brief
    /// drained query (the `measure_system` cadence). If turso's reader/writer/
    /// checkpoint interaction leaks a read mark under this concurrency, the WAL
    /// climbs monotonically across boundaries instead of reclaiming. Asserts the
    /// WAL stays bounded near one package's worth.
    #[tokio::test]
    async fn concurrent_periodic_reader_does_not_defeat_per_package_reclaim() {
        use std::sync::Arc;
        let (path, db) = scratch("concurrent-prod-analog").await;
        let db = Arc::new(db);
        db.record_fetch(&Fetch {
            source: "ted".into(),
            kind: "daily".into(),
            period: "2026-00001".into(),
            url: "u".repeat(64),
            sha256: "a".repeat(64),
            bytes: 1,
            fetched_at: 1,
            path: "ted/daily/2026-00001.tar.gz".into(),
        })
        .await
        .unwrap();

        let pool = db.readers(4, "probe").unwrap();
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));

        // Background reader: the measure_system cadence, tightened so it overlaps
        // the writer heavily (borrow, brief drained COUNT, return, repeat).
        let reader_db = db.clone();
        let reader_pool = pool.clone();
        let reader_stop = stop.clone();
        let reader = tokio::spawn(async move {
            let _ = &reader_db;
            while !reader_stop.load(std::sync::atomic::Ordering::Relaxed) {
                let conn = reader_pool.get().await.unwrap();
                let mut rows = conn.query("SELECT COUNT(*) FROM notices", ()).await.unwrap();
                while rows.next().await.unwrap().is_some() {}
                drop(conn);
                tokio::task::yield_now().await;
            }
        });

        // Writer: 20 "packages" of 1000 notices, TRUNCATE at each boundary.
        let mut peak = 0u64;
        for chunk in 0..10i64 {
            insert_notices(&db, chunk * 1000, 1000).await;
            let r = db.checkpoint(CheckpointMode::Truncate).await.unwrap();
            let wal = db.wal_bytes().unwrap_or(0);
            peak = peak.max(wal);
            if chunk % 5 == 0 || r.busy {
                eprintln!("[DEBUG-wal01] concurrent chunk {chunk}: busy={} wal={} probe_borrowed={}", r.busy, wal, pool.borrowed());
            }
        }
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = reader.await;

        eprintln!("[DEBUG-wal01] concurrent peak WAL = {peak}");
        // One 1000-notice package is well under 2 MB; a monotonic leak would reach
        // ~20× that. Bounded means reclaim survives the concurrent reader.
        assert!(peak < 8_000_000, "[DEBUG-wal01] WAL not bounded under concurrent reader, peak {peak}");

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// [DEBUG-wal01] Does turso 0.7 autocheckpoint AT ALL with our prod PRAGMAs
    /// (WAL, synchronous=NORMAL, no explicit wal_autocheckpoint)? Write 6k notices
    /// with NO explicit checkpoint and sample the -wal size. If it plateaus, turso
    /// folds frames on its own (a mid-package climb that then reclaims is normal);
    /// if it grows ~linearly, turso does NOT autocheckpoint and the ONLY reclaim
    /// signal is the boundary TRUNCATE. Prints the trajectory; not an assertion
    /// about the value, a measurement of turso's default behaviour.
    #[tokio::test]
    async fn does_turso_autocheckpoint_by_default() {
        let (path, db) = scratch("autockpt-default").await;
        db.record_fetch(&Fetch {
            source: "ted".into(), kind: "daily".into(), period: "2026-00001".into(),
            url: "u".repeat(64), sha256: "a".repeat(64), bytes: 1, fetched_at: 1,
            path: "ted/daily/2026-00001.tar.gz".into(),
        }).await.unwrap();
        for k in 0..6i64 {
            insert_notices(&db, k * 1000, 1000).await;
            eprintln!("[DEBUG-wal01] autockpt-default: after {} notices, wal = {} bytes (NO explicit checkpoint)",
                (k + 1) * 1000, db.wal_bytes().unwrap_or(0));
        }
        for s in ["", "-wal", "-shm"] { let _ = std::fs::remove_file(format!("{path}{s}")); }
    }

    /// [DEBUG-wal01] The writer's OWN read-mark: the process job's per-notice txn is
    /// `BEGIN IMMEDIATE → INSERT → SELECT notice_id → COMMIT` (record_notice_tx,
    /// lib.rs:607) — the SELECT takes a read snapshot INSIDE the write txn on the
    /// writer connection. Does that mark linger after COMMIT and pin the WAL when
    /// the SAME writer runs the boundary TRUNCATE? Autocheckpoint OFF so frames
    /// accumulate and any self-pin is visible. (record_notice already runs the
    /// internal SELECT, so grow_wal_notices exercises the exact sequence.)
    #[tokio::test]
    async fn the_writers_internal_select_does_not_pin_its_own_truncate() {
        let (path, db) = scratch("writer-select-pin").await;
        no_autocheckpoint(&db).await;
        // record_notice = BEGIN IMMEDIATE; INSERT notice; SELECT notice_id; COMMIT.
        grow_wal_notices(&db, 3_000).await;
        let before = db.wal_bytes().unwrap_or(0);
        let r = db.checkpoint(CheckpointMode::Truncate).await.unwrap();
        let after = db.wal_bytes().unwrap_or(0);
        eprintln!("[DEBUG-wal01] writer-select-pin: busy={} wal {before} -> {after}", r.busy);
        assert!(!r.busy, "[DEBUG-wal01] the writer's internal SELECT must not pin its own TRUNCATE");
        assert!(after < 65_536, "[DEBUG-wal01] writer self-reclaims to ~0, got {after} (was {before})");
        for s in ["", "-wal", "-shm"] { let _ = std::fs::remove_file(format!("{path}{s}")); }
    }

    /// [DEBUG-wal01] THE DECISIVE EXPERIMENT (issue 54/55, second pin). No reader
    /// pool is touched at all: a single writer records N notices — each an explicit
    /// `BEGIN IMMEDIATE … COMMIT`, the process job's exact shape — and then the
    /// SAME writer connection runs a TRUNCATE checkpoint at the writer-idle package
    /// boundary. If turso leaves the writer connection holding a WAL read mark after
    /// an explicit-transaction COMMIT, the checkpoint sees a pinned frame and
    /// returns busy=1 with the WAL un-truncated — the field's "busy (reader
    /// pinned)" with ZERO other connections. Contrast: the autocommit
    /// `truncate_reclaims_the_wal_file` test above is NOT busy.
    #[tokio::test]
    async fn explicit_txn_writer_pins_its_own_wal() {
        let (path, db) = scratch("explicit-txn-pin").await;
        grow_wal_notices(&db, 2_000).await;

        let before = db.wal_bytes().expect("a wal exists after writes");
        assert!(before > 200_000, "2k notices build a non-trivial WAL, got {before}");

        let r = db.checkpoint(CheckpointMode::Truncate).await.unwrap();
        let after = db.wal_bytes().unwrap_or(0);
        eprintln!("[DEBUG-wal01] explicit-txn: busy={} wal {before} -> {after} checkpointed={} frames={}", r.busy, r.checkpointed, r.wal_frames);
        assert!(!r.busy, "[DEBUG-wal01] a lone writer must not pin its own WAL, busy={}", r.busy);
        assert!(after < 65_536, "[DEBUG-wal01] truncate should shrink the -wal, got {after} (was {before})");

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// [DEBUG-wal01] THE SECOND PIN, sharpened. The passing
    /// `an_idle_pooled_reader_does_not_pin_the_wal` checkpoints IMMEDIATELY after
    /// the idle reader's query — no writes intervene, so the reader's mark is at the
    /// WAL head and a checkpoint past it is trivially fine. Production is the other
    /// order: a reader takes a snapshot at frame X, returns to the pool IDLE, then
    /// the writer appends thousands more frames, THEN the package-boundary
    /// checkpoint runs. If turso freezes the idle connection's read mark at X, the
    /// checkpoint cannot pass X → busy, and the WAL grows past X forever (until that
    /// pooled connection is reused and re-reads, advancing its mark). This is the
    /// "frozen idle-reader marks" of issue 15 and the field's second pin.
    #[tokio::test]
    async fn an_idle_reader_that_took_an_early_mark_pins_later_writes() {
        let (path, db) = scratch("frozen-idle-mark").await;
        grow_wal_notices(&db, 500).await;

        // A reader borrows, runs a query (taking a snapshot/mark at the current
        // head), and returns to the pool IDLE — the drained-autocommit case the
        // Reader::drop pools back.
        let pool = db.readers(4, "test").unwrap();
        {
            let reader = pool.get().await.unwrap();
            let mut rows = reader.query("SELECT COUNT(*) FROM notices", ()).await.unwrap();
            while rows.next().await.unwrap().is_some() {}
            // dropped here → back in the pool, idle, no open statement/txn
        }

        // The writer now appends thousands of frames PAST the idle reader's mark.
        grow_wal_notices(&db, 5_000).await;

        let r = db.checkpoint(CheckpointMode::Truncate).await.unwrap();
        let after = db.wal_bytes().unwrap_or(0);
        eprintln!("[DEBUG-wal01] frozen-idle-mark: busy={} wal_after={after} checkpointed={} frames={}", r.busy, r.checkpointed, r.wal_frames);
        assert!(!r.busy, "[DEBUG-wal01] an idle pooled reader froze its mark and pinned later writes (busy)");
        assert!(after < 65_536, "[DEBUG-wal01] WAL not reclaimed past the frozen idle mark: {after}");

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    async fn scratch(tag: &str) -> (String, Db) {
        let path = format!("/tmp/tender-db-ckpt-{}-{}.db", tag, std::process::id());
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
        let db = Db::open(&path).await.unwrap();
        (path, db)
    }

    /// Grow the WAL with `n` autocommit inserts. At this small scale the WAL
    /// stays under turso's autocheckpoint size threshold, so frames accumulate
    /// until we checkpoint — which lets these tests observe checkpoint behaviour
    /// (reader pinning, PASSIVE-vs-TRUNCATE reclaim) directly.
    async fn grow_wal(db: &Db, n: i64) {
        for i in 0..n {
            db.record_fetch(&Fetch {
                source: "ted".into(),
                kind: "daily".into(),
                period: format!("2026-{i:05}"),
                url: "u".repeat(64),
                sha256: "a".repeat(64),
                bytes: i,
                fetched_at: i,
                path: format!("ted/daily/2026-{i:05}.tar.gz"),
            })
            .await
            .unwrap();
        }
    }

    /// [DEBUG-wal01] THE PRIME SUSPECT for the second pin: an UNDRAINED `Rows`.
    /// Many store accessors run a `LIMIT 1` query and take exactly one row via a
    /// single `rows.next()` — then drop `Rows` WITHOUT stepping to `None`
    /// (`latest_fetch`, `max_cursor`, `oldest_cursor`, …). If turso holds the read
    /// snapshot open until the statement is fully stepped/finalized, such a reader
    /// returns to the pool still pinning a frozen snapshot — yet `is_autocommit()`
    /// is true (a bare SELECT starts no txn), so the issue-53 discard fix does NOT
    /// catch it. That exactly fits the field: monotonic climb (snapshot frozen),
    /// re-running a query on that pooled conn advances it (intermittent reclaim on
    /// partial gating), full gating freezes it (monotonic). Reproduce it here.
    #[tokio::test]
    async fn an_undrained_limit1_reader_pins_the_wal() {
        let (path, db) = scratch("undrained-rows").await;
        grow_wal_notices(&db, 500).await;

        let pool = db.readers(4, "test").unwrap();
        {
            let reader = pool.get().await.unwrap();
            // The single-row accessor pattern: take ONE row, never step to None.
            let mut rows = reader.query("SELECT COUNT(*) FROM notices", ()).await.unwrap();
            let _one = rows.next().await.unwrap();
            drop(rows); // Rows dropped un-stepped-to-None
            eprintln!("[DEBUG-wal01] undrained reader autocommit={:?}", reader.is_autocommit());
            // reader returns to pool here (autocommit==true → pooled, not discarded)
        }

        grow_wal_notices(&db, 5_000).await;

        let r = db.checkpoint(CheckpointMode::Truncate).await.unwrap();
        let after = db.wal_bytes().unwrap_or(0);
        eprintln!("[DEBUG-wal01] undrained-rows: busy={} wal_after={after} checkpointed={} frames={}", r.busy, r.checkpointed, r.wal_frames);
        assert!(!r.busy, "[DEBUG-wal01] an undrained LIMIT-1 reader pinned the WAL (busy)");
        assert!(after < 65_536, "[DEBUG-wal01] WAL not reclaimed past an undrained reader: {after}");

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// [DEBUG-wal01] Disable turso's size-triggered autocheckpoint on the writer,
    /// so frames ACCUMULATE and only an explicit checkpoint folds them — removing
    /// the confound that made the tests above report `frames=0` (turso auto-folded
    /// my writes before the explicit TRUNCATE ran). With this, a genuinely pinned
    /// state is observable.
    async fn no_autocheckpoint(db: &Db) {
        let conn = db.conn().await;
        let mut rows = conn.query("PRAGMA wal_autocheckpoint = 0", ()).await.unwrap();
        while rows.next().await.unwrap().is_some() {}
    }

    /// [DEBUG-wal01] With autocheckpoint OFF, cleanly separate the two reader
    /// states against a real accumulated WAL:
    ///   A. a RETURNED (pooled, idle) reader — drained or undrained — must NOT pin;
    ///   B. a still-BORROWED reader holding an open snapshot MUST pin (control).
    #[tokio::test]
    async fn returned_readers_never_pin_only_a_held_snapshot_does() {
        let (path, db) = scratch("accumulate").await;
        no_autocheckpoint(&db).await;
        grow_wal_notices(&db, 500).await;

        let pool = db.readers(4, "test").unwrap();
        // (A1) drained-and-returned
        {
            let reader = pool.get().await.unwrap();
            let mut rows = reader.query("SELECT COUNT(*) FROM notices", ()).await.unwrap();
            while rows.next().await.unwrap().is_some() {}
        }
        // (A2) undrained-and-returned (LIMIT-1, one row, drop)
        {
            let reader = pool.get().await.unwrap();
            let mut rows = reader.query("SELECT id FROM notices LIMIT 1", ()).await.unwrap();
            let _ = rows.next().await.unwrap();
        }
        grow_wal_notices(&db, 2_000).await;
        let a = db.checkpoint(CheckpointMode::Truncate).await.unwrap();
        let a_wal = db.wal_bytes().unwrap_or(0);
        eprintln!("[DEBUG-wal01] accumulate/returned: busy={} frames={} checkpointed={} wal_after={a_wal}", a.busy, a.wal_frames, a.checkpointed);
        assert!(a.wal_frames > 0 || a_wal == 0, "[DEBUG-wal01] autocheckpoint-off should have accumulated real frames");
        assert!(!a.busy, "[DEBUG-wal01] returned readers (drained OR undrained) must not pin — busy={}", a.busy);
        assert!(a_wal < 65_536, "[DEBUG-wal01] returned readers must not block reclaim, wal={a_wal}");

        // (B) still-BORROWED reader with an OPEN transaction — the known real pin.
        grow_wal_notices(&db, 500).await;
        let held = pool.get().await.unwrap();
        held.execute("BEGIN", ()).await.unwrap();
        let mut rows = held.query("SELECT COUNT(*) FROM notices", ()).await.unwrap();
        while rows.next().await.unwrap().is_some() {}
        grow_wal_notices(&db, 2_000).await;
        let b = db.checkpoint(CheckpointMode::Truncate).await.unwrap();
        eprintln!("[DEBUG-wal01] accumulate/held-open-txn: busy={} frames={} wal={}", b.busy, b.wal_frames, db.wal_bytes().unwrap_or(0));
        assert!(b.busy, "[DEBUG-wal01] a held OPEN-txn reader must pin (control)");
        held.execute("COMMIT", ()).await.unwrap();
        drop(held);
        let c = db.checkpoint(CheckpointMode::Truncate).await.unwrap();
        assert!(!c.busy && db.wal_bytes().unwrap_or(0) < 65_536, "[DEBUG-wal01] reclaims once the snapshot ends");

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// The core issue-42 question: after bulk writes the WAL is large, and a
    /// TRUNCATE checkpoint reclaims it to zero. At this small scale turso's
    /// size-triggered autocheckpoint has not fired, so the explicit checkpoint is
    /// what folds the frames back and shrinks the file on disk.
    #[tokio::test]
    async fn truncate_reclaims_the_wal_file() {
        let (path, db) = scratch("truncate").await;
        grow_wal(&db, 4_000).await;

        let before = db.wal_bytes().expect("a wal exists after writes");
        assert!(before > 200_000, "4k inserts should build a non-trivial WAL, got {before}");

        let r = db.checkpoint(CheckpointMode::Truncate).await.unwrap();
        assert!(!r.busy, "no reader is open, so truncate completes");
        assert!(r.checkpointed >= r.wal_frames.min(r.checkpointed), "frames were folded back");

        let after = db.wal_bytes().unwrap_or(0);
        assert!(after < 65_536, "truncate shrinks the -wal file to ~0, got {after} (was {before})");

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// Issue 53 regression (the prod root cause): a pooled connection returned
    /// mid-transaction — the SSE initial-snapshot's `BEGIN` when the request
    /// future is cancelled before its `COMMIT` — must NOT re-enter the pool, or
    /// its frozen snapshot pins the WAL and blocks every checkpoint, so the WAL
    /// grows without bound (70 GB in the field, surviving with zero live
    /// connections). Without the fix this reproduces as `busy=1` and no reclaim
    /// (the leaked open transaction also blocks turso's own autocheckpoint, so
    /// the WAL is far larger here than the idle-reader case below). The pool
    /// discards a non-autocommit connection on drop, releasing the snapshot at
    /// once, so the WAL reclaims and the pool stays usable.
    #[tokio::test]
    async fn a_reader_returned_mid_transaction_is_discarded_not_pooled() {
        let (path, db) = scratch("leaked-txn").await;
        grow_wal(&db, 1_000).await;

        let pool = db.readers(4, "test").unwrap();
        {
            let reader = pool.get().await.unwrap();
            // The SSE snapshot path, cancelled mid-read: BEGIN, then the future is
            // dropped before COMMIT. The Reader drops with the transaction open.
            reader.execute("BEGIN", ()).await.unwrap();
            let mut rows = reader.query("SELECT COUNT(*) FROM fetches", ()).await.unwrap();
            while rows.next().await.unwrap().is_some() {}
            // <-- no COMMIT
        }
        // The discarded connection released its snapshot, so the writer's frames
        // fold and a TRUNCATE reclaims to zero.
        grow_wal(&db, 3_000).await;
        let r = db.checkpoint(CheckpointMode::Truncate).await.unwrap();
        assert!(!r.busy, "a discarded mid-transaction connection must not pin the WAL");
        assert!(
            db.wal_bytes().unwrap_or(0) < 65_536,
            "the WAL reclaims once the leaked snapshot is discarded, got {:?}",
            db.wal_bytes()
        );

        // The pool stays usable: a fresh borrow opens a new connection and works.
        let reader = pool.get().await.unwrap();
        let mut rows = reader.query("SELECT COUNT(*) FROM fetches", ()).await.unwrap();
        assert!(rows.next().await.unwrap().is_some(), "the pool reopened a healthy connection");

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// The CRITICAL reader-pool question (issue 42): an *idle* pooled reader — one
    /// that ran a query and was returned to the pool — must NOT pin the WAL, or a
    /// checkpoint could never reclaim past it under a live serving workload. We
    /// borrow a reader, fully drain a query, drop it back to the pool, then
    /// checkpoint and confirm the WAL still reclaims to zero.
    #[tokio::test]
    async fn an_idle_pooled_reader_does_not_pin_the_wal() {
        let (path, db) = scratch("idle-reader").await;
        grow_wal(&db, 4_000).await;

        // Warm a pooled reader: borrow, run + fully drain a query, return to pool.
        let pool = db.readers(4, "test").unwrap();
        {
            let reader = pool.get().await.unwrap();
            let mut rows = reader.query("SELECT COUNT(*) FROM fetches", ()).await.unwrap();
            while rows.next().await.unwrap().is_some() {}
            // reader dropped here → back in the pool, idle, no open statement
        }

        let r = db.checkpoint(CheckpointMode::Truncate).await.unwrap();
        assert!(!r.busy, "an idle pooled reader must not make the checkpoint busy");
        let after = db.wal_bytes().unwrap_or(0);
        assert!(after < 65_536, "the WAL reclaims past an idle pooled reader, got {after}");

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// The contrast that proves the mechanism is real: a reader holding a LIVE
    /// snapshot (an open read transaction) DOES pin the WAL, so a concurrent
    /// truncate cannot reclaim it (`busy = 1`, file not shrunk). Once the reader
    /// ends its transaction, the next checkpoint reclaims fully.
    #[tokio::test]
    async fn a_live_reader_snapshot_blocks_reclaim_until_it_ends() {
        let (path, db) = scratch("live-reader").await;
        grow_wal(&db, 4_000).await;
        let before = db.wal_bytes().unwrap();

        let pool = db.readers(4, "test").unwrap();
        let reader = pool.get().await.unwrap();
        // Open an explicit read transaction and touch a row: this takes a WAL read
        // mark and holds the snapshot for the life of the transaction.
        reader.execute("BEGIN", ()).await.unwrap();
        let mut rows = reader.query("SELECT COUNT(*) FROM fetches", ()).await.unwrap();
        while rows.next().await.unwrap().is_some() {}

        // A busy checkpoint must return PROMPTLY, not stall the writer for
        // busy_timeout: the process loop calls this per package, so a multi-second
        // wait whenever the 60s coverage scan overlaps would throttle the backfill.
        let t = std::time::Instant::now();
        let held = db.checkpoint(CheckpointMode::Truncate).await.unwrap();
        assert!(t.elapsed().as_millis() < 1_000, "a busy checkpoint returns promptly, no stall");
        assert!(held.busy, "a live reader snapshot must block a truncate (busy=1)");
        assert!(db.wal_bytes().unwrap() >= before / 2, "the file is not reclaimed while pinned");

        // End the reader's transaction; now a checkpoint reclaims fully.
        reader.execute("COMMIT", ()).await.unwrap();
        drop(reader);
        let freed = db.checkpoint(CheckpointMode::Truncate).await.unwrap();
        assert!(!freed.busy, "once the snapshot ends the checkpoint completes");
        assert!(db.wal_bytes().unwrap_or(0) < 65_536, "the WAL reclaims once unpinned");

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// Why the process loop uses TRUNCATE, not PASSIVE, to fix an *already large*
    /// WAL (the live incident: 13 GB): PASSIVE folds the frames back and lets the
    /// WAL be reused, but does NOT shrink the file on disk — only TRUNCATE returns
    /// the space. This asserts that difference so the mode choice is grounded.
    #[tokio::test]
    async fn passive_folds_but_only_truncate_shrinks_the_file() {
        let (path, db) = scratch("passive-vs-truncate").await;
        grow_wal(&db, 4_000).await;
        let grown = db.wal_bytes().unwrap();
        assert!(grown > 200_000, "the WAL grew, got {grown}");

        // PASSIVE folds every frame back (no reader pins anything) but leaves the
        // file at its high-water mark — reused in place, not returned to the FS.
        let p = db.checkpoint(CheckpointMode::Passive).await.unwrap();
        assert!(!p.busy);
        assert!(db.wal_bytes().unwrap() >= grown / 2, "PASSIVE does not shrink the file on disk");

        // TRUNCATE returns the space.
        db.checkpoint(CheckpointMode::Truncate).await.unwrap();
        assert!(db.wal_bytes().unwrap_or(0) < 65_536, "TRUNCATE shrinks the file");

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    /// Passive per-package is the loop's cadence: it never blocks, and repeated
    /// passive checkpoints keep the WAL from growing without bound across many
    /// write bursts (it is reused in place rather than extended).
    #[tokio::test]
    async fn passive_checkpoints_keep_the_wal_bounded_across_bursts() {
        let (path, db) = scratch("passive").await;

        let mut peak = 0u64;
        for _ in 0..6 {
            grow_wal(&db, 1_000).await;
            db.checkpoint(CheckpointMode::Passive).await.unwrap();
            peak = peak.max(db.wal_bytes().unwrap_or(0));
        }
        // Six 1k bursts with a passive checkpoint between each stays near one
        // burst's worth, nowhere near the ~6× an un-checkpointed run would reach.
        let one_burst_ceiling = 4_000_000; // generous: one 1k-insert burst « this
        assert!(peak < one_burst_ceiling, "passive keeps the WAL bounded, peak {peak}");

        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }
}
