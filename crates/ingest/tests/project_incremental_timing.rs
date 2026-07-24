//! Reproduce-first timing curve for the incremental projection (issue 58) — the
//! time analogue of `project_memory`. The current full projection re-reads and
//! re-folds the WHOLE corpus every run, so its wall-clock grows with the corpus;
//! the incremental projection touches only the delta, so it stays flat as the
//! corpus grows. This builds established corpora of increasing size, applies a
//! FIXED small delta to each, and times both the full non-rebuild absorb and the
//! incremental absorb — asserting the full cost rises with the corpus while the
//! incremental cost stays flat and is far cheaper at scale.

use std::time::Instant;

use ingest::project;
use store::{Db, Notice, NoticeValue, Parse, Parsed, Section, ValueRow};

const SOURCE: &str = "ted";

async fn scratch(tag: &str) -> (Db, i64, String) {
    let path = format!("/tmp/tender-db-projincrtime-{tag}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let db = Db::open(&path).await.unwrap();
    db.record_fetch(&store::Fetch {
        source: SOURCE.into(),
        kind: "daily".into(),
        period: "p".into(),
        url: "u".into(),
        sha256: "aa".into(),
        bytes: 1,
        fetched_at: 0,
        path: "p".into(),
    })
    .await
    .unwrap();
    let fetch_id = db.current_packages(SOURCE, "daily", None).await.unwrap()[0].fetch_id;
    (db, fetch_id, path)
}

/// One minimal single-notice keyed Tender (distinct key → distinct Tender).
async fn record_keyed(db: &Db, fetch_id: i64, n: i64) {
    let key = format!("bt04-{n:08}");
    let parsed = Parsed {
        sections: vec![Section { id: "PROC".into(), kind: "Procedure".into(), parent: None }],
        values: vec![
            ValueRow {
                section_id: "PROC".into(),
                field_id: "BT-04-notice".into(),
                ordinal: 0,
                value: NoticeValue::Id { scheme: None, value: key, is_ref: false },
            },
            ValueRow {
                section_id: "PROC".into(),
                field_id: "BT-05(a)-notice".into(),
                ordinal: 0,
                value: NoticeValue::Date { utc_seconds: n, offset_minutes: 0, has_time: false },
            },
        ],
    };
    db.record_notice(
        &Notice {
            source: SOURCE.into(),
            publication_id: format!("pub-{n}"),
            content_hash: format!("h-{n}"),
            profile: "eforms:eforms-sdk-1.13".into(),
            declared_version: None,
            fetch_id,
            member_path: "m".into(),
            ingested_at: 0,
            published_at: Some(0),
            dispatched_at: None,
        },
        &Parse::Parsed(parsed),
    )
    .await
    .unwrap();
}

const DELTA: i64 = 50;

/// Establish a corpus of `n` Tenders, then time absorbing a fixed `DELTA` of new
/// Tenders both ways: incremental first (touches only the delta), then a full
/// non-rebuild projection (re-scans the whole corpus, idempotent). Returns
/// (incremental_secs, full_secs).
async fn time_absorb(n: i64) -> (f64, f64) {
    let (db, fetch_id, path) = scratch(&format!("n{n}")).await;
    for i in 0..n {
        record_keyed(&db, fetch_id, i).await;
    }
    project::project(&db, false).await.unwrap(); // establish
    for i in n..n + DELTA {
        record_keyed(&db, fetch_id, i).await;
    }
    let t = Instant::now();
    project::project_incremental(&db).await.unwrap();
    let incr = t.elapsed().as_secs_f64();
    // Everything is projected now; a full run re-scans the whole corpus and
    // changes nothing — measuring exactly the O(corpus) work incremental avoids.
    let t = Instant::now();
    project::project(&db, false).await.unwrap();
    let full = t.elapsed().as_secs_f64();
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    (incr, full)
}

#[tokio::test]
#[ignore = "heavy: builds corpora and times projections (~1-2 min); run with --ignored"]
async fn incremental_absorb_is_flat_while_full_scales_with_corpus() {
    let sizes = [800i64, 3_200, 8_000];
    let mut results = Vec::new();
    for &n in &sizes {
        let (incr, full) = time_absorb(n).await;
        eprintln!("[incrtime] corpus={n:>6}  incremental={incr:6.3}s  full={full:6.3}s  (full/incr {:.0}x)", full / incr.max(1e-9));
        results.push((n, incr, full));
    }

    let (_, incr_small, full_small) = results[0];
    let (_, incr_large, full_large) = results[results.len() - 1];

    // The full projection's cost rises with the corpus (10x corpus here).
    assert!(
        full_large > full_small * 2.0,
        "full projection should scale with corpus: {full_small:.3}s → {full_large:.3}s"
    );
    // The incremental cost stays roughly flat as the corpus grows.
    assert!(
        incr_large < incr_small * 3.0,
        "incremental should stay flat as the corpus grows: {incr_small:.3}s → {incr_large:.3}s"
    );
    // And at scale the incremental run is far cheaper than the full re-scan.
    assert!(
        full_large > incr_large * 4.0,
        "at scale incremental should be far cheaper than full: full={full_large:.3}s incr={incr_large:.3}s"
    );
}
