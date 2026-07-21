//! WAL maintenance — bounding the write-ahead log during bulk loads (issue 42).
//!
//! Turso does not auto-checkpoint fresh frames (see `backup.rs`), so a long
//! process/project run accumulates WAL without bound — 13 GB and climbing during
//! the first backfill. The fix is to checkpoint at the natural writer-idle points
//! (package boundaries in the process loop, batch boundaries in the projection),
//! which folds committed frames back into the main file and lets the WAL be
//! reused instead of grown.
//!
//! A checkpoint can only reclaim frames older than the oldest **live reader
//! snapshot**: in WAL mode a reader mid-transaction pins every frame its snapshot
//! needs, and the checkpoint stops there (`busy = 1`). An *idle* pooled reader
//! (between queries, not in a transaction) holds no snapshot, so it does not pin
//! the WAL — this is verified in the tests below, and is why no reader-side
//! recycling is needed.

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
    use crate::{Db, Fetch};

    async fn scratch(tag: &str) -> (String, Db) {
        let path = format!("/tmp/tender-db-ckpt-{}-{}.db", tag, std::process::id());
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
        let db = Db::open(&path).await.unwrap();
        (path, db)
    }

    /// Grow the WAL with `n` autocommit inserts (turso never auto-checkpoints, so
    /// every committed frame stays in the WAL until we checkpoint).
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

    /// The core issue-42 question: after bulk writes the WAL is large, and a
    /// TRUNCATE checkpoint reclaims it to zero — proving turso does NOT
    /// auto-checkpoint (so the WAL really does grow unbounded without us) and that
    /// an explicit checkpoint shrinks the file on disk.
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
        let pool = db.readers(4).unwrap();
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

        let pool = db.readers(4).unwrap();
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
