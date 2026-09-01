//! Issue 335: reports keep a bounded history, so a measurement can be diffed
//! against its own past.
//!
//! `reports` holds one row per kind, so every re-run destroyed the measurement it
//! replaced. That cost a real thing: issue 311's review cohort dropped 589 → 487
//! and the 102 cases that left cannot be enumerated, because only the count had
//! been read before the re-run. Issue 333's fix was verifiable only because
//! someone copied a baseline out by hand first — diligence, not design.
//!
//! What these tests pin: that the existing latest-wins read is untouched (forty
//! callers depend on it, cursors among them), that "previous" means previous to
//! what a reader currently SEES, that the depth bound actually holds, and that a
//! first run reports honestly rather than inventing a predecessor.

async fn open(name: &str) -> store::Db {
    let path = format!("/tmp/tender-db-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    store::Db::open(&path).await.unwrap()
}

#[tokio::test]
async fn a_first_report_has_no_previous_and_says_so() {
    let db = open("rh-first").await;
    db.put_report("census", "one", 1000).await.unwrap();
    assert_eq!(db.latest_report("census").await.unwrap().unwrap().0, "one");
    // Not an error and not an empty string — genuinely nothing yet.
    assert!(db.previous_report("census").await.unwrap().is_none());
    assert_eq!(db.report_versions("census").await.unwrap(), vec![1000]);
}

#[tokio::test]
async fn a_re_run_leaves_the_version_it_replaced_readable() {
    let db = open("rh-replaced").await;
    db.put_report("census", "before", 1000).await.unwrap();
    db.put_report("census", "after", 2000).await.unwrap();
    // THE WHOLE POINT. The old behaviour destroyed "before" here.
    assert_eq!(db.latest_report("census").await.unwrap().unwrap(), ("after".into(), 2000));
    assert_eq!(db.previous_report("census").await.unwrap().unwrap(), ("before".into(), 1000));
}

#[tokio::test]
async fn latest_wins_is_untouched_for_the_callers_that_depend_on_it() {
    let db = open("rh-latest").await;
    // Cursors are stored as reports (rehash-cursor, reveal-cursor) and a resumed
    // job reading a stale one would redo work or skip it. Forty call sites go
    // through `latest_report`; none of them may start seeing history.
    db.put_report("reveal-cursor", "{\"after\":10}", 1000).await.unwrap();
    db.put_report("reveal-cursor", "{\"after\":20}", 2000).await.unwrap();
    db.put_report("reveal-cursor", "{\"after\":30}", 3000).await.unwrap();
    assert_eq!(
        db.latest_report("reveal-cursor").await.unwrap().unwrap(),
        ("{\"after\":30}".into(), 3000)
    );
}

#[tokio::test]
async fn previous_is_relative_to_what_a_reader_currently_sees() {
    let db = open("rh-relative").await;
    for at in [1000i64, 2000, 3000] {
        db.put_report("census", &format!("v{at}"), at).await.unwrap();
    }
    // Not "second newest in history" but "newest strictly older than current",
    // which is the same thing here and would not be if a history row ever ran
    // ahead of `reports`.
    assert_eq!(db.previous_report("census").await.unwrap().unwrap(), ("v2000".into(), 2000));
}

#[tokio::test]
async fn two_runs_in_one_second_are_one_version() {
    let db = open("rh-samesecond").await;
    db.put_report("census", "first try", 1000).await.unwrap();
    db.put_report("census", "corrected", 1000).await.unwrap();
    assert_eq!(db.report_versions("census").await.unwrap(), vec![1000]);
    assert_eq!(db.latest_report("census").await.unwrap().unwrap().0, "corrected");
    // The stamp is the key, so the second run corrects the first rather than
    // becoming a predecessor of itself.
    assert!(db.previous_report("census").await.unwrap().is_none());
}

#[tokio::test]
async fn the_history_is_bounded_and_prunes_the_oldest() {
    let db = open("rh-depth").await;
    let depth = store::REPORT_HISTORY_DEPTH as i64;
    for at in 1..=(depth + 5) {
        db.put_report("census", &format!("v{at}"), at * 1000).await.unwrap();
    }
    let versions = db.report_versions("census").await.unwrap();
    assert_eq!(versions.len(), depth as usize, "pruned in the same write, no sweeper");
    assert_eq!(versions[0], (depth + 5) * 1000, "newest kept");
    assert_eq!(
        *versions.last().unwrap(),
        6 * 1000,
        "and the oldest five dropped, not the newest"
    );
}

#[tokio::test]
async fn kinds_keep_their_own_histories() {
    let db = open("rh-kinds").await;
    // The prune is per kind; a busy census must not evict a quiet one.
    for at in 1..=(store::REPORT_HISTORY_DEPTH as i64 + 5) {
        db.put_report("busy", &format!("v{at}"), at * 1000).await.unwrap();
    }
    db.put_report("quiet", "only", 500).await.unwrap();
    assert_eq!(db.report_versions("quiet").await.unwrap(), vec![500]);
    assert_eq!(db.latest_report("quiet").await.unwrap().unwrap().0, "only");
}

#[tokio::test]
async fn an_unknown_kind_is_empty_rather_than_an_error() {
    let db = open("rh-unknown").await;
    assert!(db.previous_report("never-run").await.unwrap().is_none());
    assert!(db.report_versions("never-run").await.unwrap().is_empty());
}

/// The stamps surface (`/metrics` report freshness) must keep reading one row per
/// kind — the risk the issue-335 plan called out before the additive design
/// removed it.
#[tokio::test]
async fn report_stamps_still_reports_one_row_per_kind() {
    let db = open("rh-stamps").await;
    for at in [1000i64, 2000, 3000] {
        db.put_report("census", &format!("v{at}"), at).await.unwrap();
    }
    db.put_report("other", "x", 1500).await.unwrap();
    let stamps = db.report_stamps().await.unwrap();
    assert_eq!(stamps.len(), 2, "two kinds, not four versions");
    let census = stamps.iter().find(|(k, _)| k == "census").unwrap();
    assert_eq!(census.1, 3000, "and it is the current stamp");
}
