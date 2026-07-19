//! Processing stage: walk archived packages, dispatch every member to a
//! mapping profile, and write Notice identity rows or quarantine rows.
//!
//! Processing never downloads (CONTEXT.md: fetching and processing are separate
//! stages) and is idempotent — re-running a package inserts nothing new,
//! because Notice identity is (source, publication_id, content_hash).

use crate::package::{self, Member};
use crate::profile::{self, Disposition, Record};
use std::path::Path;

/// Per-package outcome. The no-silent-drops invariant is
/// `members == ingested + skipped`, with every ingested member accounted for by
/// its notice and quarantine records.
#[derive(Debug, Default, PartialEq)]
pub struct Report {
    /// Payload files seen in the package.
    pub members: u64,
    /// Members that produced at least one record.
    pub ingested: u64,
    /// Members deliberately not ingested, per profile policy.
    pub skipped: u64,
    /// Notice rows newly written.
    pub notices: u64,
    /// Records whose identity was already known — the idempotency signal.
    pub duplicates: u64,
    /// Records quarantined (ADR-0004).
    pub quarantined: u64,
}

#[derive(Debug)]
pub enum Error {
    Package(package::Error),
    Db(turso::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Package(e) => write!(f, "package: {e}"),
            Error::Db(e) => write!(f, "db: {e}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<package::Error> for Error {
    fn from(e: package::Error) -> Self {
        Error::Package(e)
    }
}
impl From<turso::Error> for Error {
    fn from(e: turso::Error) -> Self {
        Error::Db(e)
    }
}

/// Process every current package of `(source, kind)`, or just one `period`.
pub async fn process(
    db: &store::Db,
    archive_root: &Path,
    source: &str,
    kind: &str,
    period: Option<&str>,
    mut on_package: impl FnMut(&store::Package, &Report),
) -> Result<Report, Error> {
    let mut total = Report::default();
    for pkg in db.current_packages(source, kind, period).await? {
        let report = process_package(db, &archive_root.join(&pkg.path), source, pkg.fetch_id).await?;
        on_package(&pkg, &report);
        total.members += report.members;
        total.ingested += report.ingested;
        total.skipped += report.skipped;
        total.notices += report.notices;
        total.duplicates += report.duplicates;
        total.quarantined += report.quarantined;
    }
    Ok(total)
}

/// Process one archived package file.
pub async fn process_package(
    db: &store::Db,
    archive: &Path,
    source: &str,
    fetch_id: i64,
) -> Result<Report, Error> {
    // The walker is synchronous and streams one member at a time; dispatch is
    // pure CPU. Collect the records per member, then write them — the store's
    // single writer connection serialises the inserts anyway.
    let mut report = Report::default();
    let mut pending = Vec::new();
    package::walk(archive, |Member { path, bytes }| {
        report.members += 1;
        match profile::dispatch(&path, bytes) {
            Disposition::Records(records) => {
                report.ingested += 1;
                pending.extend(records);
            }
            Disposition::Skipped(_) => report.skipped += 1,
        }
    })?;

    let now = unix_now();
    for record in pending {
        match record {
            Record::Notice(n) => {
                let inserted = db
                    .insert_notice(&store::Notice {
                        source: source.into(),
                        publication_id: n.publication_id,
                        content_hash: n.content_hash,
                        profile: n.profile,
                        declared_version: n.declared_version,
                        fetch_id,
                        member_path: n.member_path,
                        ingested_at: now,
                    })
                    .await?;
                if inserted {
                    report.notices += 1;
                } else {
                    report.duplicates += 1;
                }
            }
            Record::Quarantine(q) => {
                db.insert_quarantine(&store::Quarantined {
                    fetch_id,
                    member_path: q.member_path,
                    content_hash: q.content_hash,
                    profile: q.profile,
                    reason: q.reason,
                    detail: q.detail,
                    first_seen: now,
                })
                .await?;
                report.quarantined += 1;
            }
        }
    }
    Ok(report)
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
