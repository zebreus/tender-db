//! Processing stage: walk archived packages, dispatch every member to a
//! mapping profile, and write Notice identity rows or quarantine rows.
//!
//! Processing never downloads (CONTEXT.md: fetching and processing are separate
//! stages) and is idempotent — re-running a package inserts nothing new,
//! because Notice identity is (source, publication_id, content_hash).

use crate::package::{self, Member};
use crate::profile::{self, Disposition, Record};
use crate::{eforms, r209};
use std::path::Path;

/// Field mapping for one notice payload, dispatched per profile. Profiles
/// without a parser yet stay `Pending` — identity only, never quarantined.
/// Text-era records never arrive here: their payload is a span of the member
/// and their parser needs the member name for the declared encoding, so the
/// processor routes them through [`crate::text::parse_payload`] directly.
pub fn parse_payload(profile: &str, bytes: &[u8]) -> store::Parse {
    if profile.starts_with("eforms:") {
        eforms::parse_payload(profile, bytes)
    } else if profile.starts_with("ted-export-") {
        r209::parse_payload(profile, bytes)
    } else {
        store::Parse::Pending
    }
}

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
    /// Records quarantined before a Notice identity existed — unrecognised
    /// payloads (ADR-0004).
    pub quarantined: u64,
    /// Notices whose profile parser consumed the payload exhaustively.
    pub parsed: u64,
    /// Notices quarantined by their profile parser: unmapped content, or a
    /// value the schema cannot hold without loss.
    pub parse_quarantined: u64,
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
        let report =
            process_package(db, &archive_root.join(&pkg.path), source, pkg.fetch_id, |_, _, _| {})
                .await?;
        on_package(&pkg, &report);
        total.members += report.members;
        total.ingested += report.ingested;
        total.skipped += report.skipped;
        total.notices += report.notices;
        total.duplicates += report.duplicates;
        total.quarantined += report.quarantined;
        total.parsed += report.parsed;
        total.parse_quarantined += report.parse_quarantined;
    }
    Ok(total)
}

/// Process one archived package file.
///
/// `on_progress(done, total, report)` fires after each of the package's members
/// is written, where `total` is the number of records the package yielded,
/// `done` counts up to it, and `report` is the running tally — the Supervisor
/// turns this into the live progress bar (issue 16). It is a pure UI hook;
/// passing `|_, _, _| {}` is the plain processing path.
pub async fn process_package(
    db: &store::Db,
    archive: &Path,
    source: &str,
    fetch_id: i64,
    mut on_progress: impl FnMut(u64, u64, &Report),
) -> Result<Report, Error> {
    // The walker is synchronous and streams one member at a time; dispatch is
    // pure CPU. Collect the records per member, then write them — the store's
    // single writer connection serialises the inserts anyway.
    let mut report = Report::default();
    let mut pending = Vec::new();
    // Cheap name-only pre-scan: dispatch policy that spans members (the text
    // era's ISO-vs-UTF8 variant selection) needs the package's shape up front.
    let ctx = profile::PackageContext::from_entry_names(&package::entry_names(archive)?);
    package::walk(archive, |Member { path, bytes }| {
        report.members += 1;
        match profile::dispatch_with(&path, bytes, &ctx) {
            Disposition::Records(records) => {
                report.ingested += 1;
                // Field mapping happens here, while the payload is in hand: an
                // XML member is exactly one notice, so the member's bytes are
                // that notice's payload; a text-era record's payload is its
                // span of the member.
                pending.extend(records.into_iter().map(|record| {
                    let parse = match &record {
                        Record::Notice(n) => match n.span {
                            Some((start, end)) => {
                                crate::text::parse_payload(&n.member_path, &bytes[start..end])
                            }
                            None => parse_payload(&n.profile, bytes),
                        },
                        Record::Quarantine(_) => store::Parse::Pending,
                    };
                    (record, parse)
                }));
            }
            Disposition::Skipped(_) => report.skipped += 1,
        }
    })?;

    let now = unix_now();
    let total = pending.len() as u64;
    let mut done = 0u64;
    for (record, parse) in pending {
        match record {
            Record::Notice(n) => {
                let inserted = db
                    .record_notice(
                        &store::Notice {
                            source: source.into(),
                            publication_id: n.publication_id,
                            content_hash: n.content_hash,
                            profile: n.profile,
                            declared_version: n.declared_version,
                            fetch_id,
                            member_path: n.member_path,
                            ingested_at: now,
                        },
                        &parse,
                    )
                    .await?;
                if inserted {
                    report.notices += 1;
                    match parse {
                        store::Parse::Parsed(_) => report.parsed += 1,
                        store::Parse::Quarantined { .. } => report.parse_quarantined += 1,
                        store::Parse::Pending => {}
                    }
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
        done += 1;
        on_progress(done, total, &report);
    }
    Ok(report)
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
