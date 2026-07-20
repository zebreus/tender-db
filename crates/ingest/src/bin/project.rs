//! Projection CLI (issue 04): the notice-parsed layer → the canonical layer.
//!
//! ```sh
//! project              # project whatever is not projected yet
//! project --rebuild    # the reproject path: drop the canonical layer's
//!                      # content and re-derive all of it from the notices
//! ```
//!
//! Both are safe to re-run. Without `--rebuild`, an unchanged notice layer
//! produces no versions and no change rows at all. With it, the canonical
//! content is identical to what a fresh run would build, and the change log —
//! which is never renumbered — simply gains the rows describing the rebuild.

use ingest::project;
use std::process::ExitCode;

fn usage() -> ! {
    eprintln!("usage: project [--rebuild] [--db PATH]");
    std::process::exit(2);
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let mut rebuild = false;
    let mut db_path = std::env::var("TENDER_DB").unwrap_or_else(|_| "tender-db.db".into());
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--rebuild" => rebuild = true,
            "--db" => db_path = args.next().unwrap_or_else(|| usage()),
            _ => usage(),
        }
    }

    let db = match store::Db::open(&db_path).await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("open {db_path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let started = std::time::Instant::now();
    let report = match project::project(&db, rebuild).await {
        Ok(report) => report,
        Err(e) => {
            eprintln!("project: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!(
        "projected {} notices into {} tenders ({} islands) in {:.1}s",
        report.notices,
        report.tenders,
        report.islands,
        started.elapsed().as_secs_f64()
    );
    println!(
        "  versions written {}, removed {}; change rows {}; organization mentions {}; \
         legacy tenders absorbed {}",
        report.applied.versions_written,
        report.applied.versions_removed,
        report.applied.changes,
        report.mentions,
        report.absorbed
    );
    match db.canonical_counts().await {
        Ok(counts) => {
            for (label, count) in counts {
                println!("  {label:<28} {count}");
            }
        }
        Err(e) => {
            eprintln!("counts: {e}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
