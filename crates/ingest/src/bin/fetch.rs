//! Standalone fetcher CLI (issue 01). Examples:
//!
//! ```sh
//! fetch ted --daily 2026-137            # one OJ S issue
//! fetch ted --month 2026-06             # one monthly package
//! fetch ted --probe-latest              # walk forward from the newest known issue
//! fetch doe --day 2026-07-18            # one completed day (T+1)
//! fetch doe --month 2026-06             # one monthly export
//! fetch doe --backfill                  # every month from 2022-12 to now
//! fetch fts --day 2026-09-03            # one UK civil day, paged + assembled
//! fetch fts --month 2025-06             # one month as 1-day windows
//! fetch fts --backfill                  # every month from 2021-01 to now
//! ```
//!
//! `--archive` / `--db` override the TENDER_ARCHIVE / TENDER_DB env vars
//! (defaults: ./archive, tender-db.db). `--refetch` re-downloads and
//! compares by hash (finality window); default skips known periods.
//! `--page-pause SECS` overrides the FTS pause between page requests
//! (default `fts::PAGE_PAUSE_SECS`); an interrupted FTS walk resumes from its
//! `<period>.pages/` staging dir on the next run.

use ingest::{doe, fetch, fts, ted};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

enum Source {
    Ted,
    Doe,
    Fts,
}

struct Args {
    source: Source,
    daily: Option<(u16, u32)>,
    day: Option<(u16, u8, u8)>,
    month: Option<(u16, u8)>,
    probe_latest: bool,
    backfill: bool,
    refetch: bool,
    archive: PathBuf,
    db: String,
    base_url: String,
    page_pause: Duration,
}

fn usage() -> ! {
    eprintln!(
        "usage: fetch ted (--daily YYYY-NNN | --month YYYY-MM | --probe-latest) \
         [--refetch] [--archive DIR] [--db PATH]\n       \
         fetch doe (--day YYYY-MM-DD | --month YYYY-MM | --backfill) \
         [--refetch] [--archive DIR] [--db PATH]\n       \
         fetch fts (--day YYYY-MM-DD | --month YYYY-MM | --backfill) \
         [--refetch] [--page-pause SECS] [--archive DIR] [--db PATH]"
    );
    std::process::exit(2);
}

fn parse_args() -> Args {
    let mut args = std::env::args().skip(1);
    let (source, base_url) = match args.next().as_deref() {
        Some("ted") => (Source::Ted, ted::BASE),
        Some("doe") => (Source::Doe, doe::BASE),
        Some("fts") => (Source::Fts, fts::BASE),
        _ => usage(),
    };
    let mut out = Args {
        source,
        daily: None,
        day: None,
        month: None,
        probe_latest: false,
        backfill: false,
        refetch: false,
        archive: std::env::var("TENDER_ARCHIVE").unwrap_or_else(|_| "archive".into()).into(),
        db: std::env::var("TENDER_DB").unwrap_or_else(|_| "tender-db.db".into()),
        base_url: base_url.into(),
        page_pause: Duration::from_secs(fts::PAGE_PAUSE_SECS),
    };
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| usage());
        match arg.as_str() {
            "--daily" => {
                let v = value();
                let (y, n) = v.split_once('-').unwrap_or_else(|| usage());
                out.daily = Some((y.parse().unwrap_or_else(|_| usage()), n.parse().unwrap_or_else(|_| usage())));
            }
            "--day" => out.day = Some(parse_date(&value())),
            "--month" => {
                let v = value();
                let (y, m) = v.split_once('-').unwrap_or_else(|| usage());
                out.month = Some((y.parse().unwrap_or_else(|_| usage()), m.parse().unwrap_or_else(|_| usage())));
            }
            "--probe-latest" => out.probe_latest = true,
            "--backfill" => out.backfill = true,
            "--refetch" => out.refetch = true,
            "--archive" => out.archive = value().into(),
            "--db" => out.db = value(),
            "--base-url" => out.base_url = value(),
            "--page-pause" => {
                out.page_pause = Duration::from_secs(value().parse().unwrap_or_else(|_| usage()));
            }
            _ => usage(),
        }
    }
    let any = out.daily.is_some() || out.day.is_some() || out.month.is_some() || out.probe_latest || out.backfill;
    if !any {
        usage();
    }
    out
}

/// `YYYY-MM-DD` → (year, month, day).
fn parse_date(s: &str) -> (u16, u8, u8) {
    let mut parts = s.split('-');
    let mut next = || parts.next().unwrap_or_else(|| usage());
    let (y, m, d) = (next(), next(), next());
    let date = (
        y.parse().unwrap_or_else(|_| usage()),
        m.parse().unwrap_or_else(|_| usage()),
        d.parse().unwrap_or_else(|_| usage()),
    );
    if parts.next().is_some() || !(1..=12).contains(&date.1) || !(1..=31).contains(&date.2) {
        usage();
    }
    date
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args = parse_args();
    let db = match store::Db::open(&args.db).await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("open db {}: {e}", args.db);
            return ExitCode::FAILURE;
        }
    };
    let client = reqwest::Client::new();

    let mut failures = 0u32;
    let run = |target: fetch::Target, refetch: bool| {
        let db = &db;
        let client = &client;
        let archive = &args.archive;
        let page_pause = args.page_pause;
        async move {
            let result = if target.source == "fts" {
                // Paged and self-assembled: page progress on stderr, since a
                // month is ~150 paced requests.
                fetch::fetch_fts(db, client, archive, &target, refetch, page_pause, |day, pages, releases| {
                    eprintln!("  {day}: page {pages}, {releases} releases so far");
                })
                .await
            } else {
                fetch::fetch(db, client, archive, &target, refetch).await
            };
            match result {
                Ok(outcome) => {
                    println!("{} {} {}: {outcome:?}", target.source, target.kind, target.period);
                    Ok(outcome)
                }
                Err(e) => {
                    eprintln!("{} {} {}: {e}", target.source, target.kind, target.period);
                    Err(())
                }
            }
        }
    };

    if let Some((year, issue)) = args.daily
        && run(ted::daily(&args.base_url, year, issue), args.refetch).await.is_err()
    {
        failures += 1;
    }
    if let Some(date) = args.day {
        let target = match args.source {
            Source::Ted => usage(), // TED dailies are OJ S issues (`--daily`), not calendar days
            Source::Doe => doe::day(&args.base_url, date),
            Source::Fts => fts::day(&args.base_url, date),
        };
        if run(target, args.refetch).await.is_err() {
            failures += 1;
        }
    }
    if let Some((year, month)) = args.month {
        let target = match args.source {
            Source::Ted => ted::monthly(&args.base_url, year, month),
            Source::Doe => doe::monthly(&args.base_url, year, month),
            Source::Fts => fts::monthly(&args.base_url, (year, month)),
        };
        if run(target, args.refetch).await.is_err() {
            failures += 1;
        }
    }
    if args.probe_latest {
        // Walk forward from the newest registered issue until the server 404s.
        // The library owns the probe so the in-app Supervisor runs the same one.
        match fetch::probe_ted_daily(
            &db,
            &client,
            &args.archive,
            &args.base_url,
            args.refetch,
            |period, outcome| println!("ted daily {period}: {outcome:?}"),
        )
        .await
        {
            Ok(_) => {}
            Err(e) => {
                eprintln!("probe: {e}");
                failures += 1;
            }
        }
    }
    if args.backfill {
        let (year, month, _) = fetch::current_date_utc();
        match args.source {
            Source::Doe => {
                // Every month since the archive starts. Closed months are immutable, so
                // the registry skips them without a download; the current month is
                // still accumulating (T+1 per day) and is always re-fetched.
                for (y, m) in doe::months_through((year, month)) {
                    let current = (y, m) == (year, month);
                    if run(doe::monthly(&args.base_url, y, m), args.refetch || current).await.is_err() {
                        failures += 1;
                    }
                }
            }
            Source::Fts => {
                // Every month since 2021-01, each walked as 1-day windows and
                // resumable from its staging dir. The current month is not
                // force-refetched: a re-walk is ~150 paced requests, and the
                // daily probe covers the days after this run. A failed month
                // (the limiter) is reported and the run continues; re-run to resume.
                for (y, m) in fts::months_through((year, month)) {
                    if run(fts::monthly(&args.base_url, (y, m)), args.refetch).await.is_err() {
                        failures += 1;
                    }
                }
            }
            Source::Ted => usage(), // TED backfills are monthly ranges (`--month`), not a walk
        }
    }

    if failures > 0 { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
