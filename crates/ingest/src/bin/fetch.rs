//! Standalone fetcher CLI (issue 01). Examples:
//!
//! ```sh
//! fetch ted --daily 2026-137            # one OJ S issue
//! fetch ted --month 2026-06             # one monthly package
//! fetch ted --probe-latest              # walk forward from the newest known issue
//! ```
//!
//! `--archive` / `--db` override the TENDER_ARCHIVE / TENDER_DB env vars
//! (defaults: ./archive, tender-db.db). `--refetch` re-downloads and
//! compares by hash (finality window); default skips known periods.

use ingest::{fetch, ted};
use std::path::PathBuf;
use std::process::ExitCode;

struct Args {
    daily: Option<(u16, u32)>,
    month: Option<(u16, u8)>,
    probe_latest: bool,
    refetch: bool,
    archive: PathBuf,
    db: String,
    base_url: String,
}

fn usage() -> ! {
    eprintln!(
        "usage: fetch ted (--daily YYYY-NNN | --month YYYY-MM | --probe-latest) \
         [--refetch] [--archive DIR] [--db PATH]"
    );
    std::process::exit(2);
}

fn parse_args() -> Args {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("ted") {
        usage();
    }
    let mut out = Args {
        daily: None,
        month: None,
        probe_latest: false,
        refetch: false,
        archive: std::env::var("TENDER_ARCHIVE").unwrap_or_else(|_| "archive".into()).into(),
        db: std::env::var("TENDER_DB").unwrap_or_else(|_| "tender-db.db".into()),
        base_url: ted::BASE.into(),
    };
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| usage());
        match arg.as_str() {
            "--daily" => {
                let v = value();
                let (y, n) = v.split_once('-').unwrap_or_else(|| usage());
                out.daily = Some((y.parse().unwrap_or_else(|_| usage()), n.parse().unwrap_or_else(|_| usage())));
            }
            "--month" => {
                let v = value();
                let (y, m) = v.split_once('-').unwrap_or_else(|| usage());
                out.month = Some((y.parse().unwrap_or_else(|_| usage()), m.parse().unwrap_or_else(|_| usage())));
            }
            "--probe-latest" => out.probe_latest = true,
            "--refetch" => out.refetch = true,
            "--archive" => out.archive = value().into(),
            "--db" => out.db = value(),
            "--base-url" => out.base_url = value(),
            _ => usage(),
        }
    }
    if out.daily.is_none() && out.month.is_none() && !out.probe_latest {
        usage();
    }
    out
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
    let run = |target: fetch::Target| {
        let db = &db;
        let client = &client;
        let archive = &args.archive;
        let refetch = args.refetch;
        async move {
            match fetch::fetch(db, client, archive, &target, refetch).await {
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
        && run(ted::daily(&args.base_url, year, issue)).await.is_err()
    {
        failures += 1;
    }
    if let Some((year, month)) = args.month
        && run(ted::monthly(&args.base_url, year, month)).await.is_err()
    {
        failures += 1;
    }
    if args.probe_latest {
        // Walk forward from the newest registered issue of the current year
        // (or issue 1) until the server says 404.
        let year = current_year();
        let mut issue = match latest_issue(&db, year).await {
            Ok(i) => i.unwrap_or(0) + 1,
            Err(e) => {
                eprintln!("probe: read registry: {e}");
                return ExitCode::FAILURE;
            }
        };
        loop {
            match run(ted::daily(&args.base_url, year, issue)).await {
                Ok(fetch::Outcome::NotFound) => break,
                Ok(_) => issue += 1,
                Err(()) => {
                    failures += 1;
                    break;
                }
            }
        }
    }

    if failures > 0 { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}

/// Newest daily issue number already in the registry for `year`.
async fn latest_issue(db: &store::Db, year: u16) -> turso::Result<Option<u32>> {
    // Periods sort lexicographically (`YYYY-NNNNN`), so max(period) works.
    let latest = db.latest_fetch_period_max("ted", "daily", &format!("{year}-")).await?;
    Ok(latest.and_then(|p| p.split_once('-').and_then(|(_, n)| n.parse().ok())))
}

fn current_year() -> u16 {
    // Days since epoch → civil year (Howard Hinnant's algorithm, no deps).
    let days = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        / 86_400) as i64;
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    (y + if mp >= 10 { 1 } else { 0 }) as u16
}
