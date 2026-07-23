//! Peak-memory reproduction for issue 57: the full-corpus projection accumulates
//! the whole corpus's per-notice `states` + `all_mentions` (and, inside
//! `resolve_mentions`, the whole `mention_of` map) in RAM before it does anything
//! with them, so peak heap grows with the corpus and OOMs the 8 GB VPS at 7.5M+
//! notices.
//!
//! This test drives the real projection over a synthetic corpus at two sizes and
//! measures the peak resident memory the projection holds, via the kernel's
//! peak-RSS high-water mark (`VmHWM`), reset to the current RSS just before each
//! run (`/proc/self/clear_refs`). The bug signature is that the peak scales with
//! the corpus (4× the notices ⇒ ~4× the peak). The fix bounds the peak to a
//! working set independent of corpus size, which is what this test asserts — so
//! the reproduction doubles as the regression test.
//!
//! turso installs its own `#[global_allocator]`, so a tracking allocator is not
//! an option; `VmHWM` needs no allocator hook and captures the true peak the OOM
//! killer sees.

use ingest::project;
use store::{Db, Notice, NoticeValue, Parse, Parsed, Section, ValueRow};

// ---------------------------------------------------------------- peak RSS

/// A `/proc/self/status` field, in bytes.
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

/// Reset the kernel's peak-RSS high-water mark to the current RSS, so `VmHWM`
/// afterwards reflects only the peak reached from here on.
fn reset_peak_rss() {
    std::fs::write("/proc/self/clear_refs", "5").expect("reset VmHWM via clear_refs");
}

/// Run `f`, returning the peak *additional* resident bytes above the baseline at
/// entry — the high-water mark of RSS over the region.
async fn peak_rss_of<F, Fut>(f: F) -> usize
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    reset_peak_rss();
    let baseline = status_field("VmRSS");
    f().await;
    status_field("VmHWM").saturating_sub(baseline)
}

// ------------------------------------------------------------- corpus builder

const SOURCE: &str = "ted";

async fn scratch(name: &str) -> (Db, i64, String) {
    let path = format!("/tmp/tender-db-projmem-{name}-{}.db", std::process::id());
    let _ = std::fs::remove_file(&path);
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
    (db, fetch_id, path)
}

/// One synthetic eForms island notice: a BT-04-less procedure with a title, a
/// dispatch date, and two organizations each carrying a distinct VAT id. This is
/// deliberately representative of the heavy per-notice state (facts + mentions)
/// that the projection accumulates — every notice is its own Tender, so `n`
/// notices produce `n` states and `2n` mentions.
fn island_notice(fetch_id: i64, i: u64) -> (Notice, Parse) {
    let pub_id = format!("{i:08}-2026");
    let mut sections = vec![Section { id: "PROC".into(), kind: "Procedure".into(), parent: None }];
    let mut values = vec![
        ValueRow {
            section_id: "PROC".into(),
            field_id: "BT-21-Procedure".into(),
            ordinal: 0,
            value: NoticeValue::Text { lang: Some("ENG".into()), value: format!("Works contract {i}") },
        },
        ValueRow {
            section_id: "PROC".into(),
            field_id: "BT-05(a)-notice".into(),
            ordinal: 0,
            value: NoticeValue::Date {
                utc_seconds: 1_700_000_000 + i as i64,
                offset_minutes: 0,
                has_time: false,
            },
        },
    ];
    for org in 0..2u64 {
        let sid = format!("ORG-{org}");
        let legal = format!("ORG-{org}-legal");
        sections.push(Section { id: sid.clone(), kind: "Organization".into(), parent: Some("PROC".into()) });
        sections.push(Section { id: legal.clone(), kind: "CompanyLegalEntity".into(), parent: Some(sid.clone()) });
        values.push(ValueRow {
            section_id: sid.clone(),
            field_id: "BT-500-Organization-Company".into(),
            ordinal: 0,
            value: NoticeValue::Text { lang: Some("ENG".into()), value: format!("Bidder {i}-{org}") },
        });
        // A distinct, plausible VAT id per (notice, org): each seeds one new
        // canonical Organization, so the org dedup maps grow with the corpus too.
        values.push(ValueRow {
            section_id: legal,
            field_id: "BT-501-Organization-Company".into(),
            ordinal: 0,
            value: NoticeValue::Id {
                scheme: Some("VAT".into()),
                value: format!("NL{:09}B{:02}", i, org),
                is_ref: false,
            },
        });
        values.push(ValueRow {
            section_id: "PROC".into(),
            field_id: "OPT-300-Procedure-Buyer".into(),
            ordinal: org as i64,
            value: NoticeValue::Id { scheme: None, value: sid, is_ref: true },
        });
    }
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
            published_at: Some(1_700_000_000 + i as i64),
            dispatched_at: Some(1_700_000_000 + i as i64),
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

fn mib(bytes: usize) -> f64 {
    bytes as f64 / 1_048_576.0
}

/// The bounded projection's peak resident memory is a fixed working set — a batch
/// of notices, not the whole corpus. Two facts prove the fix:
///
///  1. **Bounded** — folding a fixed batch size over 2× the corpus barely moves
///     the peak, whereas the whole-corpus (`usize::MAX` batch) projection's peak
///     grows with the corpus, because it materialises every notice's state at
///     once (the OOM mechanism of issue 57).
///  2. **Lower** — on the same corpus, the streaming projection's peak sits well
///     below the whole-RAM projection's.
///
/// Peak is the kernel's `VmHWM`; the batch here (2 000) is deliberately far below
/// the corpus so Phase 2 never holds more than a batch of states at once.
///
/// `#[ignore]` because it builds tens of thousands of notices and projects them
/// three times (~10 min) — too slow for every `cargo test`. It is the committed
/// reproduction/regression for issue 57; run it explicitly:
///
/// ```sh
/// cargo test -p ingest --test project_memory -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore = "heavy: builds ~18k notices and projects 3× (~10 min); run with --ignored"]
async fn projection_peak_memory_is_bounded_by_the_batch_not_the_corpus() {
    const BATCH: usize = 2_000;
    const SMALL: u64 = 6_000;
    const LARGE: u64 = 12_000; // 2× the corpus

    let (db_small, fetch_small, path_small) = scratch("small").await;
    build_corpus(&db_small, fetch_small, SMALL).await;
    let stream_small = peak_rss_of(|| async {
        let report = project::project_with_batch(&db_small, true, BATCH).await.expect("stream small");
        assert_eq!(report.tenders, SMALL);
    })
    .await;

    let (db_large, fetch_large, path_large) = scratch("large").await;
    build_corpus(&db_large, fetch_large, LARGE).await;
    let stream_large = peak_rss_of(|| async {
        let report = project::project_with_batch(&db_large, true, BATCH).await.expect("stream large");
        assert_eq!(report.tenders, LARGE);
    })
    .await;
    // The same larger corpus, folded whole (the pre-fix, whole-RAM behaviour).
    let whole_large = peak_rss_of(|| async {
        let report =
            project::project_with_batch(&db_large, true, usize::MAX).await.expect("whole large");
        assert_eq!(report.tenders, LARGE);
    })
    .await;

    eprintln!(
        "[projmem] streaming(batch={BATCH}): {SMALL} notices -> {:.1} MiB | {LARGE} notices -> {:.1} MiB \
         (ratio {:.2}x); whole-RAM {LARGE} notices -> {:.1} MiB",
        mib(stream_small),
        mib(stream_large),
        stream_large as f64 / stream_small.max(1) as f64,
        mib(whole_large),
    );

    // Bounded: 2× the corpus, same batch → the peak barely moves. Unbounded
    // accumulation would roughly double it.
    assert!(
        stream_large < stream_small * 3 / 2,
        "streaming peak grew with the corpus: {:.1} MiB at {LARGE} vs {:.1} MiB at {SMALL} notices",
        mib(stream_large),
        mib(stream_small),
    );
    // Lower: on the larger corpus, streaming holds a batch where whole-RAM holds
    // the corpus, so its peak is materially smaller.
    assert!(
        stream_large < whole_large * 3 / 4,
        "streaming peak ({:.1} MiB) is not below the whole-RAM peak ({:.1} MiB) on {LARGE} notices",
        mib(stream_large),
        mib(whole_large),
    );

    let _ = std::fs::remove_file(&path_small);
    let _ = std::fs::remove_file(&path_large);
}
