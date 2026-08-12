//! Issue 139 diagnostic: feed one archived member through the real dispatch
//! and print what the pipeline would do with it today.
//!
//!     cargo run -p ingest --example diag139 -- <member_path> <file>

use ingest::profile::{Disposition, Record};

fn main() {
    let mut args = std::env::args().skip(1);
    let member_path = args.next().expect("member_path arg");
    let file = args.next().expect("file arg");
    let bytes = std::fs::read(&file).expect("read file");
    match ingest::profile::dispatch(&member_path, &bytes) {
        Disposition::Skipped(why) => println!("SKIPPED: {why}"),
        Disposition::Records(records) => {
            for r in records {
                match r {
                    Record::Notice(n) => println!(
                        "NOTICE: publication_id={} profile={} declared_version={:?}",
                        n.publication_id, n.profile, n.declared_version
                    ),
                    Record::Quarantine(q) => println!(
                        "QUARANTINE: reason={} detail={:?} profile={:?}",
                        q.reason, q.detail, q.profile
                    ),
                }
            }
        }
    }
}
