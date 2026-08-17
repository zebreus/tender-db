//! Processing stage: walk archived packages, dispatch every member to a
//! mapping profile, and write Notice identity rows or quarantine rows.
//!
//! Processing never downloads (CONTEXT.md: fetching and processing are separate
//! stages) and is idempotent — re-running a package inserts nothing new,
//! because Notice identity is (source, publication_id, content_hash).

use crate::package::{self, Member};
use crate::profile::{self, Disposition, Record};
use crate::{eforms, internal_ojs, r209};
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
    } else if profile == internal_ojs::PROFILE {
        internal_ojs::parse_payload(profile, bytes)
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
        let report = process_package_resilient(
            db,
            &archive_root.join(&pkg.path),
            source,
            pkg.fetch_id,
            |_, _, _| {},
        )
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

/// Process one package, treating **archive corruption as non-fatal** and only
/// letting **systemic** failures stop the run. A package the walker cannot read
/// at all (a truncated outer container, a corrupt nested tar) is recorded as a
/// package-level quarantine and reported as `quarantined: 1`, so the caller
/// moves on to the next package — the job must never die on one bad file
/// (ADR-0004). A database error is systemic and propagates: it means stop.
pub async fn process_package_resilient(
    db: &store::Db,
    archive: &Path,
    source: &str,
    fetch_id: i64,
    on_progress: impl FnMut(u64, u64, &Report),
) -> Result<Report, turso::Error> {
    match process_package(db, archive, source, fetch_id, on_progress).await {
        Ok(report) => Ok(report),
        Err(Error::Db(e)) => Err(e),
        Err(Error::Package(e)) => {
            let member_path = archive.to_string_lossy().into_owned();
            db.insert_quarantine(&store::Quarantined {
                fetch_id,
                content_hash: crate::sha256_hex(member_path.as_bytes()),
                member_path,
                profile: Some("corrupt-package".into()),
                reason: format!("unreadable package: {e}"),
                detail: None,
                first_seen: store::now_unix(),
            })
            .await?;
            Ok(Report { quarantined: 1, ..Default::default() })
        }
    }
}

/// Process one archived package file.
///
/// `on_progress(done, total, report)` fires after each of the package's
/// records is written, where `total` is the best current estimate of the
/// package's record count (the payload-entry count, corrected upward if
/// records outnumber it — text-era members yield several), `done` counts up
/// to it, and `report` is the running tally — the Supervisor turns this into
/// the live progress bar (issue 16). It is a pure UI hook; passing
/// `|_, _, _| {}` is the plain processing path.
type WalkerHandle = std::thread::JoinHandle<Result<WalkTally, package::Error>>;
type RecordRx = std::sync::mpsc::Receiver<(Record, store::Parse)>;

/// Spawn the walker: it reads `archive`, dispatches every member to its profile,
/// and field-maps each record, all on a dedicated CPU thread that streams
/// `(Record, Parse)` items over a bounded channel to the async writer. The bound
/// keeps memory flat: a monthly-scale package (66k notices) held whole as parsed
/// values is gigabytes, which is how the first TED monthly run died.
///
/// Shared by [`process_package`] (fresh ingest) and [`reclaim_package`]
/// (reprocess) so a reclaimed member sees a byte-identical parse to a first
/// ingest. Returns the receiver, the walker handle (yielding its
/// `(members, ingested, skipped)` tally), and the entry-count progress estimate.
///
/// `only` restricts parsing to a set of member files: a member whose path is not
/// in it is read but neither dispatched nor parsed (issue 77 — the reprocess of a
/// sparse bucket skips the members it would only no-op on). `None` parses every
/// member, the plain ingest path.
fn spawn_record_producer(
    archive: &Path,
    only: Option<std::collections::HashSet<String>>,
    unreadable: &std::collections::HashSet<String>,
) -> Result<(RecordRx, WalkerHandle, u64), Error> {
    // Cheap name-only pre-scan: dispatch policy that spans members (the text
    // era's ISO-vs-UTF8 variant selection) needs the package's shape up front;
    // the entry count doubles as the progress total.
    let names = package::entry_names(archive)?;
    let estimated = names.len() as u64;
    // A bundle the ledger holds as UNREADABLE never yielded records, so it
    // must not supersede its readable twin (issue 202: the corrupt 2005-04-09
    // EN UTF8 suppressed a good ISO and lost the whole day). Both the plain
    // ingest and the reclaim consult the same ledger-derived set.
    let ctx = profile::PackageContext::from_entry_names_excluding(&names, unreadable);

    let (tx, rx) = std::sync::mpsc::sync_channel::<(Record, store::Parse)>(64);
    let archive = archive.to_owned();
    // Held CONTAINERS (issue 196): a row whose member_path is a nested archive
    // the walker descends — a monthly's inner `<daily>.tar.gz`, recorded whole
    // by the pre-recursion walker — never arrives as a member itself, so the
    // exact-path check below can never dispatch it and the row is unreachable
    // by any reprocess. A member INSIDE a held container counts as held; its
    // records resolve the container row via the container stamp address.
    let containers: Vec<String> = only
        .as_ref()
        .map(|set| {
            set.iter()
                .filter(|p| !p.contains('!') && !p.contains('#'))
                .filter(|p| {
                    let lower = p.to_ascii_lowercase();
                    lower.ends_with(".tar.gz") || lower.ends_with(".zip")
                })
                .map(|p| format!("{p}/"))
                .collect()
        })
        .unwrap_or_default();
    let walker = std::thread::spawn(move || -> Result<WalkTally, package::Error> {
        let (mut members, mut ingested, mut skipped) = (0u64, 0u64, 0u64);
        // Which held members a dispatch policy declined, and which policy. Only
        // collected in reprocess mode (`only` set), where it is bounded by the
        // held set the caller already holds in RAM; a plain ingest leaves it empty
        // rather than accumulating every skipped sibling of a whole archive.
        let mut declined: Vec<(String, &'static str)> = Vec::new();
        let collect_declined = only.is_some();
        // Set once the receiver is gone (writer failed): keep walking cheaply
        // to finish the archive read, but stop parsing.
        let mut dead = false;
        package::walk(&archive, |Member { path, bytes, corruption }| {
            members += 1;
            // Issue 77: in a targeted reprocess, only the bucket's held members
            // are worth parsing — every other member would just no-op. Skip them
            // before the expensive dispatch+parse (the tar is still read).
            if only.as_ref().is_some_and(|set| !set.contains(&path))
                && !containers.iter().any(|c| path.starts_with(c.as_str()))
            {
                return;
            }
            // A member the walker recovered from archive corruption (a
            // truncated/unreadable inner bundle or entry) is quarantined with
            // its reason (ADR-0004) — a visible data-quality metric, never a
            // policy-skip.
            if let Some(reason) = corruption {
                ingested += 1;
                if !dead {
                    let record = Record::Quarantine(profile::QuarantineRecord {
                        content_hash: crate::sha256_hex(bytes),
                        profile: None,
                        reason,
                        detail: None,
                        member_path: path,
                    });
                    dead = tx.send((record, store::Parse::Pending)).is_err();
                }
                return;
            }
            match profile::dispatch_with(&path, bytes, &ctx) {
                Disposition::Records(records) => {
                    ingested += 1;
                    for record in records {
                        if dead {
                            continue;
                        }
                        // Field mapping happens here, while the payload is in
                        // hand: an XML member is exactly one notice, so the
                        // member's bytes are that notice's payload; a text-era
                        // record's payload is its span of the member.
                        let parse = match &record {
                            Record::Notice(n) => match n.span {
                                Some((start, end)) => {
                                    crate::text::parse_payload(&n.member_path, &bytes[start..end])
                                }
                                None => parse_payload(&n.profile, bytes),
                            },
                            Record::Quarantine(_) => store::Parse::Pending,
                        };
                        dead = tx.send((record, parse)).is_err();
                    }
                }
                Disposition::Skipped(policy) => {
                    skipped += 1;
                    if collect_declined {
                        declined.push((path.clone(), policy));
                    }
                }
            }
        })?;
        Ok(WalkTally { members, ingested, skipped, declined })
    });
    Ok((rx, walker, estimated))
}

/// Pull the next streamed record. `recv` blocks, so it hops to the blocking
/// pool; the receiver rides along because `spawn_blocking` needs `'static`.
async fn recv_next(slot: &mut Option<RecordRx>) -> Option<(Record, store::Parse)> {
    let rx = slot.take().expect("receiver in flight");
    let (msg, rx) = tokio::task::spawn_blocking(move || {
        let msg = rx.recv();
        (msg, rx)
    })
    .await
    .expect("record receiver panicked");
    *slot = Some(rx);
    msg.ok()
}

/// Build the store Notice for one parsed record, resolving its own
/// publication/dispatch dates now while the payload is in hand (issue 18) — the
/// same resolution the projection uses, so the notice row and its versions agree.
fn resolved_notice(
    source: &str,
    fetch_id: i64,
    ingested_at: i64,
    n: profile::NoticeRecord,
    parse: &store::Parse,
) -> store::Notice {
    let (published_at, dispatched_at) = match parse {
        store::Parse::Parsed(parsed) => {
            let (published, dispatched) = crate::project::notice_instants(parsed);
            (Some(published), dispatched)
        }
        _ => (None, None),
    };
    store::Notice {
        source: source.into(),
        publication_id: n.publication_id,
        content_hash: n.content_hash,
        profile: n.profile,
        declared_version: n.declared_version,
        fetch_id,
        member_path: n.member_path,
        ingested_at,
        published_at,
        dispatched_at,
    }
}

pub async fn process_package(
    db: &store::Db,
    archive: &Path,
    source: &str,
    fetch_id: i64,
    mut on_progress: impl FnMut(u64, u64, &Report),
) -> Result<Report, Error> {
    let (rx, walker, estimated) = spawn_record_producer(archive, None, &db.unreadable_bundle_members(fetch_id).await?)?;
    let mut report = Report::default();
    let now = store::now_unix();
    let mut done = 0u64;
    let mut slot = Some(rx);
    while let Some((record, parse)) = recv_next(&mut slot).await {
        match record {
            Record::Notice(n) => {
                let inserted =
                    db.record_notice(&resolved_notice(source, fetch_id, now, n, &parse), &parse).await?;
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
        on_progress(done, estimated.max(done), &report);
    }
    drop(slot);

    let tally = walker.join().expect("package walker panicked")?;
    report.members = tally.members;
    report.ingested = tally.ingested;
    report.skipped = tally.skipped;
    Ok(report)
}

/// What one archive walk observed. `declined` is populated only in reprocess
/// mode — see [`spawn_record_producer`].
struct WalkTally {
    members: u64,
    ingested: u64,
    skipped: u64,
    declined: Vec<(String, &'static str)>,
}

/// Per-package outcome of a reprocess pass over a held quarantine bucket.
#[derive(Debug, Default, PartialEq)]
pub struct ReclaimReport {
    /// Members the walker read.
    pub members: u64,
    /// Held members that now parse and were written in place (or freshly).
    pub reclaimed: u64,
    /// Members that still do not parse: left held.
    pub still_held: u64,
    /// Members already parsed (a prior reclaim, or one that never failed).
    pub already: u64,
    /// Held members a documented dispatch policy skips rather than ingests —
    /// the per-language duplicate siblings ([`profile::Disposition::Skipped`]).
    /// They yield no record, so nothing is written and their quarantine row is
    /// left exactly as it was; without this counter the reprocess walks past
    /// them reporting nothing, and the four outcomes stop summing to the held
    /// set (issue 84 — 593k stale 2008 rows that no reclaim can ever move).
    pub skipped_by_policy: u64,
    /// The CURRENT still-held reasons seen this pass, with counts — the bounded
    /// sample the job result surfaces so an operator sees the shape of the
    /// residual without querying (issue 87). At most [`REASON_SAMPLE_CAP`]
    /// distinct reasons; later arrivals fold into `(other)` so the counts stay
    /// honest without the map growing with the data.
    pub still_held_reasons: std::collections::BTreeMap<String, u64>,
}

/// The bound on distinct reasons in [`ReclaimReport::still_held_reasons`].
const REASON_SAMPLE_CAP: usize = 8;

/// Count `reason` in the bounded sample: an already-seen reason always counts,
/// a novel one takes a free slot or folds into `(other)`.
fn sample_reason(map: &mut std::collections::BTreeMap<String, u64>, reason: &str) {
    if map.contains_key(reason) || map.len() < REASON_SAMPLE_CAP {
        *map.entry(reason.to_owned()).or_insert(0) += 1;
    } else {
        *map.entry("(other)".to_owned()).or_insert(0) += 1;
    }
}

/// How often the reprocess checkpoints inside one package to bound WAL/RAM on a
/// huge package (issue 80). Every N members walked, not reclaimed, so a sparse
/// bucket still checkpoints on schedule.
const CHECKPOINT_EVERY: u64 = 5_000;

/// How many declined members are flagged per statement. Bounded for the same
/// reason the reclaim checkpoints: turso writes a WAL frame per row.
const FLAG_BATCH: usize = 500;

/// Re-parse a package's HELD members and reclaim every one that now parses,
/// writing its parsed layer in place ([`store::Db::reclaim_notice`]). `held` is
/// the bucket's still-held member files for this package
/// ([`store::Db::quarantine_held_member_files`]); only those are dispatched +
/// parsed (issue 77), so a sparse bucket skips the members it would only no-op on.
/// The walk/dispatch/parse is [`spawn_record_producer`] — identical to a fresh
/// ingest — so a reclaimed member's parsed layer matches one. Members that still
/// fail (or are still unrecognised profile-level quarantines / corruption) stay
/// held. Memory stays flat: the producer streams one record at a time.
pub async fn reclaim_package(
    db: &store::Db,
    archive: &Path,
    source: &str,
    fetch_id: i64,
    held: std::collections::HashSet<String>,
    mut on_progress: impl FnMut(u64, u64, &ReclaimReport),
) -> Result<ReclaimReport, Error> {
    let (rx, walker, estimated) =
        spawn_record_producer(archive, Some(held), &db.unreadable_bundle_members(fetch_id).await?)?;
    let mut report = ReclaimReport::default();
    let now = store::now_unix();
    let mut done = 0u64;
    let mut slot = Some(rx);
    while let Some((record, parse)) = recv_next(&mut slot).await {
        match record {
            Record::Notice(n) => {
                match db.reclaim_notice(&resolved_notice(source, fetch_id, now, n, &parse), &parse).await? {
                    store::Reclaim::Reclaimed => report.reclaimed += 1,
                    store::Reclaim::StillHeld => {
                        report.still_held += 1;
                        match &parse {
                            store::Parse::Quarantined { reason, .. } => {
                                sample_reason(&mut report.still_held_reasons, reason)
                            }
                            _ => sample_reason(&mut report.still_held_reasons, "pending"),
                        }
                    }
                    store::Reclaim::AlreadyParsed => report.already += 1,
                }
            }
            // A held member that STILL fails before an identity exists (an
            // unrecognised profile, or corruption) re-arrives as a quarantine
            // record, never reaching `reclaim_notice`. It is still-held work all
            // the same: count it, and record the attempt ON the row (issue 87) —
            // before this it was walked past silently, its row keeping a stale
            // first-ingest reason and the outcomes not summing to the held set.
            Record::Quarantine(q) => {
                db.record_reclaim_attempt(
                    fetch_id,
                    &q.member_path,
                    &q.content_hash,
                    &q.reason,
                    q.detail.as_deref(),
                    now,
                )
                .await?;
                report.still_held += 1;
                sample_reason(&mut report.still_held_reasons, &q.reason);
            }
        }
        done += 1;
        // Bound the WAL/RAM WITHIN a huge package (issue 80): reclaim writes each
        // commit but only checkpoint per-package by default, so a 70k-member
        // package would grow the WAL its whole length. Truncate periodically —
        // best-effort (a busy result reclaims on the next tick), writer-idle here.
        if done % CHECKPOINT_EVERY == 0 {
            let _ = db.checkpoint(store::CheckpointMode::Truncate).await;
        }
        on_progress(done, estimated.max(done), &report);
    }
    drop(slot);

    // In reprocess mode the producer dispatches ONLY the held members, so its
    // skipped tally is exactly the held members a dispatch policy declines.
    let tally = walker.join().expect("package walker panicked")?;
    report.members = tally.members;
    report.skipped_by_policy = tally.skipped;
    // Record the outcome ON THE ROW, not merely in this report (issue 84). A
    // declined member yields no record, so without this its quarantine row stays
    // indistinguishable from one nobody ever examined — held forever, counted as
    // outstanding work that no reprocess can ever move. Batched: a dense package
    // can decline tens of thousands of members, and turso writes a WAL frame per
    // row.
    for chunk in tally.declined.chunks(FLAG_BATCH) {
        db.flag_skipped_members(fetch_id, chunk, now).await?;
    }
    Ok(report)
}

/// What a [`reparse_package`] pass did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReparseReport {
    /// Members the walker read.
    pub members: u64,
    /// Notices whose parsed layer was REPLACED from the archive.
    pub reparsed: u64,
    /// Records that re-parsed fine but matched no notice row — nothing to replace.
    /// Not an error: the selected members come from `notices`, but a package walk
    /// can yield records the selection did not name (a text-era member file holds
    /// several records, and only some may be in the cohort).
    pub unmatched: u64,
    /// Records the CURRENT parser quarantines. Their existing parsed layer is left
    /// untouched — a re-parse must never trade a good layer for a failure.
    pub now_failing: u64,
}

/// Re-parse a package's already-parsed members IN PLACE against the current
/// parser (issue 100), replacing each notice's parsed layer and re-opening it for
/// folding. The reclaim path's twin, for the case where the notices are fine and
/// the PARSER changed.
///
/// `members` is the set to walk, from [`store::Db::parsed_member_files`], so a
/// cohort sparse in a big package costs its members and not the package (issue
/// 77's discipline).
///
/// A record the current parser QUARANTINES leaves the stored layer alone and is
/// counted in `now_failing`. That is the conservative reading and the reason this
/// cannot simply route through the reclaim path: swapping a good parsed layer for
/// a quarantine would turn a parser regression into data loss, and the counter
/// makes such a regression visible in the job summary instead of silent.
pub async fn reparse_package(
    db: &store::Db,
    archive: &Path,
    source: &str,
    fetch_id: i64,
    members: std::collections::HashSet<String>,
    mut on_progress: impl FnMut(u64, u64, &ReparseReport),
) -> Result<ReparseReport, Error> {
    let (rx, walker, estimated) =
        spawn_record_producer(archive, Some(members), &db.unreadable_bundle_members(fetch_id).await?)?;
    let mut report = ReparseReport::default();
    let now = store::now_unix();
    let mut done = 0u64;
    let mut slot = Some(rx);
    while let Some((record, parse)) = recv_next(&mut slot).await {
        if let Record::Notice(n) = record {
            match &parse {
                store::Parse::Parsed(parsed) => {
                    let resolved = resolved_notice(source, fetch_id, now, n, &parse);
                    if db.reparse_notice(&resolved, parsed).await? {
                        report.reparsed += 1;
                    } else {
                        report.unmatched += 1;
                    }
                }
                // Quarantined now: keep what the corpus already has.
                _ => report.now_failing += 1,
            }
        }
        done += 1;
        // Same WAL bound as the reclaim walk (issue 80): each notice commits, so a
        // dense package would grow the log its whole length without this.
        if done % CHECKPOINT_EVERY == 0 {
            let _ = db.checkpoint(store::CheckpointMode::Truncate).await;
        }
        on_progress(done, estimated.max(done), &report);
    }
    drop(slot);
    let tally = walker.join().expect("package walker panicked")?;
    report.members = tally.members;
    Ok(report)
}
