//! Issue 407: the calibrated rate guard. One row per package walk, and a floor
//! taken from the walk's OWN (source, kind) history — the median of the previous
//! writing walks over `RATE_FLOOR_DIVISOR` — so the number is measured on this
//! box, never guessed. All dormant relative to the serving path.

use store::jobs::{
    rate_verdict, PackageRate, RateVerdict, RATE_FLOOR_DIVISOR, RATE_FLOOR_MIN_HISTORY, RATE_HISTORY_WINDOW,
    RATE_MIN_MEMBERS,
};

async fn open(name: &str) -> (store::Db, String) {
    let path = format!("/tmp/tender-db-package-rates-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    (store::Db::open(&path).await.expect("open"), path)
}

fn walk(period: &str, members: u64, notices: u64, seconds: f64, walked_at: i64) -> PackageRate {
    PackageRate {
        source: "ted".into(),
        kind: "daily".into(),
        period: period.into(),
        members,
        notices,
        duplicates: members - notices,
        seconds,
        walked_at,
    }
}

/// The eleven writing TED daily walks the divisor was calibrated against
/// (journal, 2026-09-11..18): members/s as the `[process]` line printed them.
const CALIBRATION: [f64; 11] = [155.7, 79.3, 34.0, 81.8, 156.3, 179.5, 136.2, 135.7, 87.7, 86.9, 75.2];

#[tokio::test]
async fn the_history_holds_only_writing_walks_of_judgeable_size_newest_first() {
    let (db, path) = open("history").await;
    // A writing walk, a pure-dedup re-walk of the same package, a tiny writing
    // walk, and a second writing walk a day later.
    db.record_package_rate(&walk("2026-00101", 3000, 3000, 30.0, 1_000)).await.expect("w1");
    db.record_package_rate(&walk("2026-00101", 3000, 0, 1.5, 1_100)).await.expect("dedup");
    db.record_package_rate(&walk("2026-00102", 50, 50, 0.1, 1_200)).await.expect("tiny");
    db.record_package_rate(&walk("2026-00103", 4000, 4000, 20.0, 2_000)).await.expect("w2");
    // Another source's walk never enters this history.
    let mut doe = walk("2026-09-18", 1000, 1000, 4.0, 3_000);
    doe.source = "doe".into();
    db.record_package_rate(&doe).await.expect("doe");

    let rates = db.recent_writing_rates("ted", "daily", RATE_HISTORY_WINDOW).await.expect("rates");
    assert_eq!(rates, vec![200.0, 100.0], "newest first, writing walks of ≥ {RATE_MIN_MEMBERS} members only");
    let one = db.recent_writing_rates("ted", "daily", 1).await.expect("rates");
    assert_eq!(one, vec![200.0], "the limit keeps the newest");
    let none = db.recent_writing_rates("ted", "monthly", RATE_HISTORY_WINDOW).await.expect("rates");
    assert!(none.is_empty(), "a (source, kind) with no walks has no history");
    drop(db);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}

#[test]
fn the_floor_is_calibrated_from_the_walks_own_history() {
    // 2026-00180 (136.4 members/s, the walk that priced 404's regression) judged
    // against the eleven before it: median 87.7, floor 8.77.
    let judged = walk("2026-00180", 3424, 3424, 25.1, 0);
    match rate_verdict(&judged, &CALIBRATION) {
        RateVerdict::Clear { floor, median, history } => {
            assert_eq!(history, 11);
            assert!((median - 87.7).abs() < 1e-9, "median {median}");
            assert!((floor - 87.7 / RATE_FLOOR_DIVISOR).abs() < 1e-9, "floor {floor}");
        }
        other => panic!("a normal writing walk is clear, got {other:?}"),
    }
    // Every real walk, judged against the other ten, clears — the natural 5.3×
    // spread within one (source, kind) never fires the guard. The slowest
    // (2026-00126, 34.0) is the one that would if the divisor were guessed low.
    for (i, rate) in CALIBRATION.iter().enumerate() {
        let others: Vec<f64> = CALIBRATION.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, r)| *r).collect();
        let w = walk("real", 3500, 3500, 3500.0 / rate, 0);
        assert!(
            matches!(rate_verdict(&w, &others), RateVerdict::Clear { .. }),
            "real walk at {rate} members/s must clear: {:?}",
            rate_verdict(&w, &others)
        );
    }
    // Issue 404's regression — 0.12 members/s on 2026-09-16 — is the alarm.
    let regression = walk("2026-00135", 3550, 3550, 3550.0 / 0.12, 0);
    match rate_verdict(&regression, &CALIBRATION) {
        RateVerdict::Alarm { floor, .. } => assert!(floor > 0.12, "floor {floor} names the collapse"),
        other => panic!("404's rate alarms, got {other:?}"),
    }
}

#[test]
fn pending_until_the_history_is_deep_enough_and_dedup_or_tiny_walks_are_never_judged() {
    let w = walk("2026-00180", 3424, 3424, 25.1, 0);
    let four: Vec<f64> = CALIBRATION[..RATE_FLOOR_MIN_HISTORY - 1].to_vec();
    assert_eq!(
        rate_verdict(&w, &four),
        RateVerdict::Pending { have: RATE_FLOOR_MIN_HISTORY - 1, need: RATE_FLOOR_MIN_HISTORY }
    );
    let five: Vec<f64> = CALIBRATION[..RATE_FLOOR_MIN_HISTORY].to_vec();
    assert!(matches!(rate_verdict(&w, &five), RateVerdict::Clear { history: 5, .. }), "five walks make a floor");
    // A pure-dedup walk is another regime; a tiny walk is fixed cost, not throughput.
    assert_eq!(rate_verdict(&walk("2026-00179", 3534, 0, 2.1, 0), &CALIBRATION), RateVerdict::NotJudged);
    assert_eq!(rate_verdict(&walk("2026-07-27", 93, 93, 0.01, 0), &CALIBRATION), RateVerdict::NotJudged);
    // A collapsed dedup walk is still not judged: the guard is for the write path.
    assert_eq!(rate_verdict(&walk("2026-00179", 3534, 0, 30_000.0, 0), &CALIBRATION), RateVerdict::NotJudged);
}

#[test]
fn the_threshold_is_the_even_count_median_over_the_divisor_and_at_the_floor_is_clear() {
    let history = [10.0, 60.0, 20.0, 50.0, 30.0, 40.0]; // median (30 + 40) / 2 = 35
    let floor = 35.0 / RATE_FLOOR_DIVISOR;
    let at = walk("at", 1000, 1000, 1000.0 / floor, 0);
    assert!(matches!(rate_verdict(&at, &history), RateVerdict::Clear { .. }), "at the floor is clear");
    let under = walk("under", 1000, 1000, 1000.0 / (floor * 0.99), 0);
    match rate_verdict(&under, &history) {
        RateVerdict::Alarm { floor: f, median, history: n } => {
            assert!((f - floor).abs() < 1e-9);
            assert!((median - 35.0).abs() < 1e-9);
            assert_eq!(n, 6);
        }
        other => panic!("just under the floor alarms, got {other:?}"),
    }
}
