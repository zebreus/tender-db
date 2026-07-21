//! Consistent online snapshots of the store (issue 23).
//!
//! Turso exposes no online-backup API, and `VACUUM INTO` OOM-kills the box at
//! archive scale (docs/research/turso-scale.md §1). The only mechanism that
//! works at 10–40 GB is `wal_checkpoint(TRUNCATE)` + a file copy — but a file
//! copy is consistent only if nothing writes the file while it is copied. This
//! module makes that hold by taking the store's **single writer** for the
//! length of the copy: with the writer held and the WAL truncated into the main
//! file, the database is momentarily frozen, so the copied bytes are a
//! self-contained, crash-clean SQLite file — not "a copy of a live DB". Readers
//! keep serving over WAL throughout, so a snapshot has zero read downtime.
//!
//! The copy runs on a blocking thread, so copying a 100 GB file never stalls
//! the async runtime; the writer guard is held across the await. Only the
//! checkpoint + copy hold the writer — the copy is then verified offline
//! (integrity_check + a row-count comparison against the source, per
//! docs/research/turso-scale.md, which showed integrity_check alone passing on a
//! truncated backup), with the writer already released.

use crate::{Db, int, opt_int_of};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// The outcome of one snapshot — the numbers the runbook and the job log
/// report: how big, how long the writer was frozen, and that the copy verified
/// against the source.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotReport {
    pub path: PathBuf,
    pub bytes: u64,
    /// `COUNT(*)` of notices in the source at snapshot time…
    pub source_notices: i64,
    /// …and in the copy — equal exactly when the copy is complete.
    pub copy_notices: i64,
    /// `PRAGMA integrity_check` on the copy returned a single `ok`.
    pub integrity_ok: bool,
    /// Wall-clock the single writer was held (checkpoint + copy), seconds.
    pub frozen_secs: f64,
    /// Wall-clock verifying the copy offline (writer already released), seconds.
    pub verify_secs: f64,
}

/// Why a snapshot failed. `Verify` means the copy was produced but does not
/// match the source (a torn/short copy) or failed integrity_check — the file is
/// deleted before this is returned, so a bad snapshot never lingers.
#[derive(Debug)]
pub enum BackupError {
    Db(turso::Error),
    Io(std::io::Error),
    Verify(String),
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupError::Db(e) => write!(f, "db: {e}"),
            BackupError::Io(e) => write!(f, "io: {e}"),
            BackupError::Verify(m) => write!(f, "verify: {m}"),
        }
    }
}

impl std::error::Error for BackupError {}

impl From<turso::Error> for BackupError {
    fn from(e: turso::Error) -> Self {
        BackupError::Db(e)
    }
}

impl From<std::io::Error> for BackupError {
    fn from(e: std::io::Error) -> Self {
        BackupError::Io(e)
    }
}

impl Db {
    /// Take a consistent snapshot of the database into `dest`, verify the copy
    /// offline against the source, and return what it did. The single writer is
    /// held only for the checkpoint + copy; verification runs on the copy
    /// afterwards with the writer released. A copy that fails to verify is
    /// removed and [`BackupError::Verify`] returned.
    pub async fn snapshot(&self, dest: &Path) -> Result<SnapshotReport, BackupError> {
        let frozen = Instant::now();
        let source_notices = {
            let conn = self.conn().await;
            // Fold the WAL into the main file and truncate it to zero bytes, so
            // the single `.db` file is self-contained — no `-wal` sibling to
            // copy and no un-checkpointed frames left behind.
            let mut rows = conn.query("PRAGMA wal_checkpoint(TRUNCATE)", ()).await?;
            while rows.next().await?.is_some() {}
            let source_notices = count_notices(&conn).await?;
            // Copy the now-quiescent file on a blocking thread. The writer guard
            // is held across the await, so no write can touch the bytes and
            // turso cannot auto-checkpoint fresh frames into the main file.
            copy_blocking(PathBuf::from(&self.path), dest.to_path_buf()).await?;
            source_notices
            // guard dropped here → writer released before the (slow) verify
        };
        let frozen_secs = frozen.elapsed().as_secs_f64();

        let verify = Instant::now();
        let (copy_notices, integrity_ok) = verify_copy(dest).await?;
        let verify_secs = verify.elapsed().as_secs_f64();

        if copy_notices != source_notices {
            let _ = std::fs::remove_file(dest);
            return Err(BackupError::Verify(format!(
                "notice count mismatch: source {source_notices}, copy {copy_notices}"
            )));
        }
        if !integrity_ok {
            let _ = std::fs::remove_file(dest);
            return Err(BackupError::Verify("integrity_check did not return ok".into()));
        }

        Ok(SnapshotReport {
            path: dest.to_path_buf(),
            bytes: std::fs::metadata(dest)?.len(),
            source_notices,
            copy_notices,
            integrity_ok,
            frozen_secs,
            verify_secs,
        })
    }

    /// Unix seconds of the most recent successful snapshot, read from the job
    /// log — what the dashboard renders as "last snapshot age". `None` if none
    /// has run. Goes through the reader pool, never the writer (issue 20).
    pub async fn last_snapshot_at(&self) -> turso::Result<Option<i64>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT MAX(finished_at) FROM job_log WHERE kind = 'snapshot' AND outcome = 'ok'",
                (),
            )
            .await?;
        Ok(rows.next().await?.and_then(|row| opt_int_of(&row, 0)))
    }
}

/// `COUNT(*)` of the notices table — the biggest table and the one the row-count
/// comparison guards the copy with.
async fn count_notices(conn: &turso::Connection) -> turso::Result<i64> {
    let mut rows = conn.query("SELECT COUNT(*) FROM notices", ()).await?;
    Ok(rows.next().await?.map_or(0, |row| int(&row, 0)))
}

/// Copy `src` to `dst` on a blocking thread, creating the destination's parent
/// directory. Blocking because a 100 GB copy must not run on an async worker.
async fn copy_blocking(src: PathBuf, dst: PathBuf) -> std::io::Result<()> {
    tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&src, &dst)?;
        Ok(())
    })
    .await
    .expect("snapshot copy task panicked")
}

/// Open the copy and check it independently of the source: `integrity_check`
/// must return a single `ok`, and its notice count is returned for the caller
/// to compare against the source. A clean `integrity_check` yields exactly one
/// row (`ok`); any problem yields one row per fault, so more than one row — or a
/// first row that is not `ok` — fails the check.
async fn verify_copy(path: &Path) -> Result<(i64, bool), BackupError> {
    let path = path.to_str().ok_or_else(|| BackupError::Verify("non-utf8 snapshot path".into()))?;
    let database = turso::Builder::new_local(path).build().await?;
    let conn = database.connect()?;

    let mut rows = conn.query("PRAGMA integrity_check", ()).await?;
    let mut integrity_ok = false;
    let mut seen = 0;
    while let Some(row) = rows.next().await? {
        seen += 1;
        integrity_ok = seen == 1 && matches!(row.get_value(0), Ok(turso::Value::Text(s)) if s == "ok");
    }

    let copy_notices = count_notices(&conn).await?;
    Ok((copy_notices, integrity_ok))
}

#[cfg(test)]
mod tests {
    use crate::{Db, Fetch, Notice, Parse};

    async fn scratch(tag: &str) -> (String, Db) {
        let path = format!("/tmp/tender-db-backup-{}-{}.db", tag, std::process::id());
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
        let db = Db::open(&path).await.unwrap();
        (path, db)
    }

    /// Seed a couple of notices so the row-count verification has something to
    /// compare, then snapshot into a fresh path and assert the copy verifies and
    /// carries the same notice count.
    #[tokio::test]
    async fn snapshot_copies_and_verifies_against_the_source() {
        let (path, db) = scratch("ok").await;
        db.record_fetch(&Fetch {
            source: "ted".into(),
            kind: "daily".into(),
            period: "2026-00137".into(),
            url: "u".into(),
            sha256: "aa".into(),
            bytes: 1,
            fetched_at: 1,
            path: "p".into(),
        })
        .await
        .unwrap();
        for i in 0..3 {
            db.record_notice(
                &Notice {
                    source: "ted".into(),
                    publication_id: format!("pub-{i}"),
                    content_hash: format!("h{i}"),
                    profile: "eforms".into(),
                    declared_version: None,
                    fetch_id: 1,
                    member_path: format!("m{i}"),
                    ingested_at: 1,
                    published_at: None,
                    dispatched_at: None,
                },
                &Parse::Pending,
            )
            .await
            .unwrap();
        }

        let dest = format!("/tmp/tender-db-backup-ok-snap-{}.db", std::process::id());
        let _ = std::fs::remove_file(&dest);
        let report = db.snapshot(std::path::Path::new(&dest)).await.unwrap();

        assert!(report.integrity_ok, "the copy must pass integrity_check");
        assert_eq!(report.source_notices, 3);
        assert_eq!(report.copy_notices, report.source_notices, "the copy holds every notice");
        assert!(report.bytes > 0);

        // The snapshot opens on its own and reads the same data — the "open it
        // read-only as the test" acceptance check, in miniature.
        let restored = Db::open(&dest).await.unwrap();
        assert_eq!(restored.notice_counts_by_profile().await.unwrap(), vec![("eforms".to_owned(), 3)]);

        for p in [&path, &dest] {
            let _ = std::fs::remove_file(p);
        }
    }
}
