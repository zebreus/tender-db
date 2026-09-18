//! Corpus-independent projection memory (issue 59), and bounded WAL through the
//! plan build (issue 42/59 caveat).
//!
//! Issue 57 bounded projection memory to one Phase-2 batch. Issue 59 removed the
//! last O(corpus) structure — the in-RAM grouping plan — by backing it with disk
//! (SQL grouping incl. the legacy union-find). This test proves the result two
//! ways, at a FIXED batch far below the corpus so batching is actually exercised:
//!
//!  1. **Flat peak** — projecting 2× the notices barely moves peak RSS (the plan
//!     is on disk; only a batch of states is ever resident), whereas the whole-RAM
//!     (`usize::MAX` batch) projection's peak scales with the corpus.
//!  2. **Bounded WAL** — a sampler watches the `-wal` file throughout the run;
//!     periodic checkpoints keep it bounded through the Phase-1 plan build too, so
//!     the plan's ~N inserts never re-balloon the WAL we just fixed (issue 42).
//!
//! Peak RSS is the kernel's `VmHWM` (turso owns the global allocator, so a tracking
//! allocator isn't possible). Corpus sizes are both above the 10k read chunk so the
//! Phase-1 transient is capped identically for both — isolating corpus dependence.
//!
//! `#[ignore]` because it builds tens of thousands of notices and projects them
//! three times (~15 min) — too slow for every `cargo test`. Run explicitly:
//!
//! ```sh
//! cargo test -p ingest --test project_memory -- --ignored --nocapture
//! ```

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

use ingest::project;
use store::{Db, Notice, NoticeValue, Parse, Parsed, Section, ValueRow};

// ---------------------------------------------------------------- peak RSS

fn status_field(name: &str) -> usize {
    let status = std::fs::read_to_string("/proc/self/status").expect("read /proc/self/status");
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix(name).filter(|_| line.as_bytes()[name.len()] == b':') {
            let kb: usize =
                rest.trim_start_matches(':').split_whitespace().next().unwrap().parse().unwrap();
            return kb * 1024;
        }
    }
    panic!("no {name} in /proc/self/status");
}

/// Reset the kernel's peak-RSS high-water mark to the current RSS.
fn reset_peak_rss() {
    std::fs::write("/proc/self/clear_refs", "5").expect("reset VmHWM via clear_refs");
}

fn mib(bytes: usize) -> f64 {
    bytes as f64 / 1_048_576.0
}

/// Project `db`, returning `(peak additional RSS bytes, peak observed -wal
/// bytes)` — the RSS high-water mark above entry, and the largest WAL the sampler
/// saw at any instant during the run.
async fn project_measured(db: &Arc<Db>, batch: usize) -> (usize, u64) {
    let stop = Arc::new(AtomicBool::new(false));
    let wal_max = Arc::new(AtomicU64::new(0));
    let sampler = tokio::spawn({
        let db = Arc::clone(db);
        let stop = Arc::clone(&stop);
        let wal_max = Arc::clone(&wal_max);
        async move {
            while !stop.load(Relaxed) {
                wal_max.fetch_max(db.wal_bytes().unwrap_or(0), Relaxed);
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        }
    });

    reset_peak_rss();
    let baseline = status_field("VmRSS");
    project::project_with_batch(db, true, batch).await.expect("project");
    let peak = status_field("VmHWM").saturating_sub(baseline);

    stop.store(true, Relaxed);
    sampler.await.expect("sampler");
    // A final direct read, in case the run ended between samples.
    wal_max.fetch_max(db.wal_bytes().unwrap_or(0), Relaxed);
    (peak, wal_max.load(Relaxed))
}

// ------------------------------------------------------------- corpus builder

const SOURCE: &str = "ted";

async fn scratch(name: &str) -> (Arc<Db>, i64, String) {
    let path = format!("/tmp/tender-db-projmem-{name}-{}.db", std::process::id());
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{suffix}"));
    }
    let db = Db::open(&path).await.expect("open scratch db");
    db.record_fetch(&store::Fetch {
        source: SOURCE.into(),
        kind: "daily".into(),
        period: "2026-00136".into(),
        url: "https://example.invalid/pkg".into(),
        sha256: "aa".into(),
        bytes: 1,
        fetched_at: 0,
        path: "ted/daily/2026-00136.tar.gz".into(),
    })
    .await
    .expect("record fetch");
    let fetch_id = db.current_packages(SOURCE, "daily", None).await.expect("packages")[0].fetch_id;
    (Arc::new(db), fetch_id, path)
}

/// One synthetic eForms island notice with a title and one organization carrying
/// a distinct VAT id — representative of the heavy per-notice state (facts +
/// mentions) the projection folds. `n` notices produce `n` island Tenders.
fn island_notice(fetch_id: i64, i: u64) -> (Notice, Parse) {
    let pub_id = format!("{i:08}-2026");
    let sections = vec![
        Section { id: "PROC".into(), kind: "Procedure".into(), parent: None },
        Section { id: "ORG-0".into(), kind: "Organization".into(), parent: Some("PROC".into()) },
        Section { id: "ORG-0-legal".into(), kind: "CompanyLegalEntity".into(), parent: Some("ORG-0".into()) },
    ];
    let values = vec![
        ValueRow {
            section_id: "PROC".into(),
            field_id: "BT-21-Procedure".into(),
            ordinal: 0,
            value: NoticeValue::Text { lang: Some("ENG".into()), value: format!("Works contract {i}") },
        },
        ValueRow {
            section_id: "ORG-0".into(),
            field_id: "BT-500-Organization-Company".into(),
            ordinal: 0,
            value: NoticeValue::Text { lang: Some("ENG".into()), value: format!("Bidder {i}") },
        },
        ValueRow {
            section_id: "ORG-0-legal".into(),
            field_id: "BT-501-Organization-Company".into(),
            ordinal: 0,
            value: NoticeValue::Id { scheme: Some("VAT".into()), value: format!("NL{i:09}B01"), is_ref: false },
        },
        ValueRow {
            section_id: "PROC".into(),
            field_id: "OPT-300-Procedure-Buyer".into(),
            ordinal: 0,
            value: NoticeValue::Id { scheme: None, value: "ORG-0".into(), is_ref: true },
        },
    ];
    (
        Notice {
            source: SOURCE.into(),
            publication_id: pub_id.clone(),
            content_hash: format!("h{i}"),
            profile: "eforms:eforms-sdk-1.13".into(),
            declared_version: None,
            fetch_id,
            member_path: format!("{pub_id}.xml"),
            ingested_at: 0,
            published_at: Some(store::Stamp::utc(1_700_000_000 + i as i64)),
            dispatched_at: Some(store::Stamp::utc(1_700_000_000 + i as i64)),
        },
        Parse::Parsed(Parsed { sections, values }),
    )
}

async fn build_corpus(db: &Db, fetch_id: i64, n: u64) {
    for i in 0..n {
        let (notice, parse) = island_notice(fetch_id, i);
        db.record_notice(&notice, &parse).await.expect("record notice");
    }
}

/// Projection peak RSS is independent of corpus size, and the WAL stays bounded
/// through the whole run — the disk-backed plan (issue 59) means only a batch of
/// states is ever resident. Both corpus sizes are above the 10k read chunk so the
/// Phase-1 transient is identical, isolating corpus dependence to what the fix
/// removed.
#[tokio::test]
#[ignore = "heavy: builds ~36k notices and projects 3× (~15 min); run with --ignored"]
async fn projection_peak_memory_is_independent_of_corpus_size() {
    const BATCH: usize = 2_000;
    const SMALL: u64 = 12_000;
    const LARGE: u64 = 24_000; // 2× the corpus, both above the 10k read chunk

    let (db_small, fetch_small, path_small) = scratch("small").await;
    build_corpus(&db_small, fetch_small, SMALL).await;
    let (stream_small, _) = project_measured(&db_small, BATCH).await;

    let (db_large, fetch_large, path_large) = scratch("large").await;
    build_corpus(&db_large, fetch_large, LARGE).await;
    let (stream_large, wal_large) = project_measured(&db_large, BATCH).await;
    // The same larger corpus, folded whole (the pre-issue-59 whole-RAM behaviour).
    let (whole_large, _) = project_measured(&db_large, usize::MAX).await;

    eprintln!(
        "[projmem] streaming(batch={BATCH}): {SMALL} -> {:.1} MiB | {LARGE} -> {:.1} MiB \
         (ratio {:.2}x); whole-RAM {LARGE} -> {:.1} MiB; peak WAL during {LARGE} run {:.1} MiB",
        mib(stream_small),
        mib(stream_large),
        stream_large as f64 / stream_small.max(1) as f64,
        mib(whole_large),
        wal_large as f64 / 1_048_576.0,
    );

    // Corpus-independent: 2× the corpus, same batch → peak barely moves.
    assert!(
        stream_large < stream_small * 6 / 5,
        "streaming peak scaled with the corpus: {:.1} MiB at {LARGE} vs {:.1} MiB at {SMALL} notices",
        mib(stream_large),
        mib(stream_small),
    );
    // And materially below the whole-RAM peak on the same corpus.
    assert!(
        stream_large < whole_large * 3 / 4,
        "streaming peak ({:.1} MiB) is not below the whole-RAM peak ({:.1} MiB) on {LARGE} notices",
        mib(stream_large),
        mib(whole_large),
    );
    // Bounded WAL through the whole run (plan build + apply): periodic TRUNCATE
    // checkpoints keep the -wal file far below the corpus's on-disk footprint.
    assert!(
        wal_large < 192 * 1_048_576,
        "WAL ballooned during the run: peak {:.1} MiB — checkpointing did not bound the plan build",
        wal_large as f64 / 1_048_576.0,
    );

    let _ = std::fs::remove_file(&path_small);
    let _ = std::fs::remove_file(&path_large);
}
