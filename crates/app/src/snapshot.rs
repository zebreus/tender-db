//! Scheduled and on-demand database snapshots (issue 23).
//!
//! A snapshot is just another Supervisor job, which buys three things for free:
//! it serialises with ingestion (so it never runs concurrently with a write
//! job), it lands in the `job_log` (so `/admin/jobs` and the dashboard show it),
//! and cancellation/queueing work like any other job. This module owns only
//! what is snapshot-specific — where snapshots are staged, how many to keep
//! locally, and the one-line summary the job log shows. The consistency
//! mechanism lives in the store ([`store::Db::snapshot`]); off-box shipping and
//! the long retention ring are a systemd timer (docs/operations.md).

use std::path::{Path, PathBuf};
use store::Db;

/// Where snapshots are staged and how many to keep locally.
///
/// The staging dir is on the data volume, which must hold archive + DB + one
/// snapshot in flight (CONTEXT.md disk budget). Off-box shipping and the long
/// retention ring (e.g. 7 daily + 4 weekly) live in a systemd timer, so the
/// **local** ring is intentionally small — just enough that a fresh snapshot
/// never deletes the previous good one before it has verified and shipped.
pub struct Config {
    pub dir: PathBuf,
    pub keep: usize,
}

impl Config {
    /// From the environment: `TENDER_SNAPSHOT_DIR` (default `snapshots`,
    /// workdir-relative — production sets `/data/snapshots`) and
    /// `TENDER_SNAPSHOT_KEEP` (default 2, floored at 1).
    pub fn from_env() -> Config {
        let dir = std::env::var("TENDER_SNAPSHOT_DIR").unwrap_or_else(|_| "snapshots".into()).into();
        let keep = std::env::var("TENDER_SNAPSHOT_KEEP")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2usize)
            .max(1);
        Config { dir, keep }
    }
}

/// The prefix + suffix every snapshot file carries, so the ring can recognise
/// its own files and never touch anything else in the directory.
const PREFIX: &str = "tender-db-";
const SUFFIX: &str = ".db";

/// File name for a snapshot taken at `unix`. Zero-padded so a lexical sort is a
/// chronological sort.
fn snapshot_name(unix: i64) -> String {
    format!("{PREFIX}{unix:010}{SUFFIX}")
}

/// Take one snapshot into the staging dir, prune the local ring, and return the
/// one-line summary for the job log. `now` is the Supervisor's clock, passed in
/// rather than read here (the crate's clock discipline) and used as the file's
/// timestamp.
pub async fn run(db: &Db, config: &Config, now: i64) -> Result<String, String> {
    let dest = config.dir.join(snapshot_name(now));
    let report = db.snapshot(&dest).await.map_err(|e| e.to_string())?;
    let pruned =
        prune(&config.dir, config.keep).map_err(|e| format!("snapshot ok but prune failed: {e}"))?;
    Ok(format!(
        "{:.2} GB, {} notices, integrity ok · frozen {:.0}s, verify {:.0}s · kept {}, pruned {}",
        report.bytes as f64 / 1e9,
        report.copy_notices,
        report.frozen_secs,
        report.verify_secs,
        config.keep,
        pruned,
    ))
}

/// Keep the newest `keep` snapshots in `dir`, delete the rest; returns how many
/// were deleted. Only files matching the snapshot naming are considered, so a
/// stray file in the directory is never removed.
fn prune(dir: &Path, keep: usize) -> std::io::Result<usize> {
    let mut snaps: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(PREFIX) && n.ends_with(SUFFIX))
        })
        .collect();
    snaps.sort(); // names sort chronologically
    let remove = snaps.len().saturating_sub(keep);
    let mut deleted = 0;
    for p in snaps.into_iter().take(remove) {
        std::fs::remove_file(&p)?;
        deleted += 1;
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ring keeps the newest `keep` snapshots and deletes older ones,
    /// leaving unrelated files alone.
    #[test]
    fn prune_keeps_the_newest_and_ignores_strangers() {
        let dir = std::env::temp_dir().join(format!("tender-db-prune-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        for unix in [100i64, 200, 300, 400] {
            std::fs::write(dir.join(snapshot_name(unix)), b"x").unwrap();
        }
        // A file that is not a snapshot must survive pruning untouched.
        std::fs::write(dir.join("README.txt"), b"keep me").unwrap();

        let deleted = prune(&dir, 2).unwrap();
        assert_eq!(deleted, 2, "four snapshots, keep two → two deleted");

        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        assert_eq!(
            left,
            vec![
                "README.txt".to_owned(),
                snapshot_name(300),
                snapshot_name(400),
            ],
            "the two newest snapshots and the stranger remain"
        );

        // Fewer snapshots than `keep` deletes nothing.
        assert_eq!(prune(&dir, 5).unwrap(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
