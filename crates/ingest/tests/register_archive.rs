//! Issue 23 / the DR premise's load-bearing finding: `register_archive` rebuilds
//! the `fetches` registry from the on-disk archive, so a fresh DB with an intact
//! `/data/archive` can `process` everything with zero re-downloads. The registry
//! row must carry the real content identity (sha256, bytes), honest provenance
//! (`archive://` url, file-mtime `fetched_at`), and version files must resolve so
//! `latest_fetch` lands on the newest content — exactly where live history would
//! have left it.

use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn scratch(name: &str) -> (PathBuf, String) {
    let root = std::env::temp_dir().join(format!("tender-db-regarch-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let db_path = format!("/tmp/tender-db-regarch-{name}-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{db_path}{s}"));
    }
    (root, db_path)
}

#[tokio::test]
async fn a_fresh_registry_is_rebuilt_from_the_archive() {
    let (root, db_path) = scratch("fresh");
    std::fs::create_dir_all(root.join("ted/daily")).unwrap();
    std::fs::create_dir_all(root.join("ted/monthly")).unwrap();
    std::fs::create_dir_all(root.join("doe/daily")).unwrap();
    std::fs::write(root.join("ted/daily/2026-00137.tar.gz"), b"ted daily bytes").unwrap();
    std::fs::write(root.join("ted/monthly/2024-06.tar.gz"), b"monthly bytes").unwrap();
    std::fs::write(root.join("doe/daily/2025-08-16.zip"), b"doe bytes").unwrap();
    // A finality rewrite: base + -v2 under ONE period; v2 must win latest_fetch.
    std::fs::write(root.join("ted/daily/2026-00138.tar.gz"), b"first content").unwrap();
    std::fs::write(root.join("ted/daily/2026-00138-v2.tar.gz"), b"rewritten content").unwrap();
    // Debris and noise that must not register or alarm.
    std::fs::write(root.join("ted/daily/2026-00139.tar.gz.part"), b"partial").unwrap();

    let db = store::Db::open(&db_path).await.unwrap();
    let done = ingest::fetch::register_archive(&db, &root).await.unwrap();
    assert_eq!(done.registered, 5, "4 periods, one of them twice (base + v2)");
    assert_eq!(done.existing, 0);
    assert_eq!(done.unrecognised, 0);

    // Content identity + honest provenance on a plain row.
    let f = db.latest_fetch("ted", "daily", "2026-00137").await.unwrap().expect("registered");
    assert_eq!(f.sha256, hex(b"ted daily bytes"));
    assert_eq!(f.bytes, b"ted daily bytes".len() as i64);
    assert_eq!(f.path, "ted/daily/2026-00137.tar.gz");
    assert_eq!(f.url, "archive://ted/daily/2026-00137.tar.gz");

    // The versioned period resolves to the NEWEST content.
    let v = db.latest_fetch("ted", "daily", "2026-00138").await.unwrap().expect("registered");
    assert_eq!(v.sha256, hex(b"rewritten content"), "latest_fetch lands on -v2");
    assert_eq!(v.path, "ted/daily/2026-00138-v2.tar.gz");

    // The other sources/kinds registered under their own identities.
    assert!(db.latest_fetch("ted", "monthly", "2024-06").await.unwrap().is_some());
    assert!(db.latest_fetch("doe", "daily", "2025-08-16").await.unwrap().is_some());

    // Idempotent: a re-run hashes nothing and records nothing.
    let again = ingest::fetch::register_archive(&db, &root).await.unwrap();
    assert_eq!((again.registered, again.existing), (0, 4), "all 4 periods already known");

    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn known_periods_keep_their_real_provenance() {
    // A registry that SURVIVED (no DB loss) must never have its rows replaced by
    // archive:// reconstructions — the original URL is better provenance.
    let (root, db_path) = scratch("keep");
    std::fs::create_dir_all(root.join("ted/daily")).unwrap();
    std::fs::write(root.join("ted/daily/2026-00140.tar.gz"), b"content").unwrap();

    let db = store::Db::open(&db_path).await.unwrap();
    db.record_fetch(&store::Fetch {
        source: "ted".into(),
        kind: "daily".into(),
        period: "2026-00140".into(),
        url: "https://ted.europa.eu/packages/daily/2026-00140".into(),
        sha256: "original".into(),
        bytes: 7,
        fetched_at: 123,
        path: "ted/daily/2026-00140.tar.gz".into(),
    })
    .await
    .unwrap();

    let done = ingest::fetch::register_archive(&db, &root).await.unwrap();
    assert_eq!((done.registered, done.existing), (0, 1));
    let f = db.latest_fetch("ted", "daily", "2026-00140").await.unwrap().unwrap();
    assert_eq!(f.url, "https://ted.europa.eu/packages/daily/2026-00140", "provenance untouched");

    let _ = std::fs::remove_dir_all(&root);
}
