//! Standalone processor CLI (issue 02). Examples:
//!
//! ```sh
//! process ted --package 2026-00137     # one registered daily package
//! process ted --all                    # every registered daily package
//! ```
//!
//! `--archive` / `--db` override the TENDER_ARCHIVE / TENDER_DB env vars
//! (defaults: ./archive, tender-db.db). Re-runs are idempotent: notices whose
//! (source, publication_id, content_hash) is already known count as duplicates
//! and write nothing.

use ingest::process;
use std::path::PathBuf;
use std::process::ExitCode;

struct Args {
    package: Option<String>,
    all: bool,
    kind: String,
    archive: PathBuf,
    db: String,
}

fn usage() -> ! {
    eprintln!(
        "usage: process ted (--package PERIOD | --all) \
         [--kind daily|monthly] [--archive DIR] [--db PATH]"
    );
    std::process::exit(2);
}

fn parse_args() -> Args {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("ted") {
        usage();
    }
    let mut out = Args {
        package: None,
        all: false,
        kind: "daily".into(),
        archive: std::env::var("TENDER_ARCHIVE").unwrap_or_else(|_| "archive".into()).into(),
        db: std::env::var("TENDER_DB").unwrap_or_else(|_| "tender-db.db".into()),
    };
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| usage());
        match arg.as_str() {
            "--package" => out.package = Some(value()),
            "--all" => out.all = true,
            "--kind" => out.kind = value(),
            "--archive" => out.archive = value().into(),
            "--db" => out.db = value(),
            _ => usage(),
        }
    }
    if out.package.is_none() && !out.all {
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

    let total = process::process(
        &db,
        &args.archive,
        "ted",
        &args.kind,
        args.package.as_deref(),
        |pkg, r| {
            println!(
                "ted {} {}: {} members → {} notices, {} duplicates, {} quarantined, {} skipped",
                args.kind, pkg.period, r.members, r.notices, r.duplicates, r.quarantined, r.skipped
            );
        },
    )
    .await;

    let total = match total {
        Ok(t) => t,
        Err(e) => {
            eprintln!("process: {e}");
            return ExitCode::FAILURE;
        }
    };

    // The no-silent-drops check (ADR-0004): every member either produced
    // records or was explicitly skipped by a documented policy.
    let accounted = total.ingested + total.skipped;
    println!(
        "\ntotal: {} members = {} ingested + {} skipped\n       \
         {} notices, {} duplicates, {} quarantined",
        total.members, total.ingested, total.skipped, total.notices, total.duplicates,
        total.quarantined
    );
    if accounted != total.members {
        eprintln!("SILENT DROP: {} members unaccounted for", total.members - accounted);
        return ExitCode::FAILURE;
    }

    match db.notice_counts_by_profile().await {
        Ok(counts) => {
            println!("\nnotices by profile:");
            for (profile, n) in counts {
                println!("  {n:>8}  {profile}");
            }
        }
        Err(e) => eprintln!("profile counts: {e}"),
    }
    match db.quarantine_counts_by_reason().await {
        Ok(counts) if !counts.is_empty() => {
            println!("\nquarantine by reason:");
            for (reason, n) in counts {
                println!("  {n:>8}  {reason}");
            }
        }
        Ok(_) => {}
        Err(e) => eprintln!("quarantine counts: {e}"),
    }

    ExitCode::SUCCESS
}
