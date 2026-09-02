//! Issue 337: what creates the `tursodb-temp.db` directories that accumulate
//! under `TMPDIR` on the production box, and what removes them.
//!
//! Read from turso 0.7.2's own source and then reproduced here:
//!
//! * `BEGIN IMMEDIATE` (and `EXCLUSIVE`) emit an `Insn::Transaction` for
//!   `TEMP_DB_ID` — `translate/transaction.rs` does this deliberately, to keep
//!   the opcode sequence identical to SQLite's. A **deferred** `BEGIN` emits no
//!   `Transaction` opcode at all, so it never touches the temp database.
//! * `op_transaction` calls `Connection::ensure_temp_database()` for that
//!   opcode (`vdbe/execute.rs`), which lazily calls `create_temp_database()`
//!   (`connection.rs`) — a `tempfile::tempdir()` under `TMPDIR` holding
//!   `tursodb-temp.db`. It is memoised per connection, so it happens once per
//!   connection, at that connection's first immediate transaction, whether or
//!   not the transaction goes on to write anything.
//! * The `TempDir` lives in the connection's `TempDatabase`, so **a graceful
//!   drop removes the directory**. This test pins that too.
//!
//! ## What that means for prod, and why this test exists
//!
//! Nothing leaks while the process is healthy: every directory is owned by a
//! live connection and removed when that connection closes. The accumulation on
//! the box comes from the *other* exit path — the service takes a default
//! SIGTERM on every restart and never unwinds, so `TempDir::drop` never runs and
//! whatever the writer connection was holding is orphaned. That is why the rate
//! tracks restarts-that-had-a-write (~1/day) rather than jobs or connections,
//! and why the sweep rule "anything older than `ActiveEnterTimestamp`" is exact
//! rather than a heuristic.
//!
//! It also rules out the cheap-looking fix: `PRAGMA temp_store = MEMORY` would
//! skip the directory, but `temp_store` is the same switch the sorter and hash
//! table read (`TempFile::with_temp_store`), so it would route every external
//! sort and hash spill into RAM — the exact failure issue 83's `TMPDIR` drop-in
//! exists to prevent, on a 490 GiB database. Not a fix; a trade of a few KB of
//! directory entries for an OOM.
//!
//! If a turso bump changes either half of the contract — the trigger or the
//! cleanup — this test fails, and the sweep unit's justification needs rereading.

use store::turso::{self, Value};

fn leaked(root: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(root)
        .expect("read TMPDIR")
        .map(|e| e.expect("dir entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(".tmp"))
        .collect();
    v.sort();
    v
}

/// Every file inside every leaked directory, as `dir/file` — the names matter:
/// the temp *database* is `tursodb-temp.db`, while a sorter spill would be
/// `tursodb_temp_file`. Only the former is what accumulates on the box.
fn inside(root: &std::path::Path) -> Vec<String> {
    let mut v = Vec::new();
    for d in leaked(root) {
        for e in std::fs::read_dir(root.join(&d)).expect("read leaked dir") {
            v.push(format!("{d}/{}", e.expect("entry").file_name().to_string_lossy()));
        }
    }
    v.sort();
    v
}

async fn drain(c: &turso::Connection, sql: &str) {
    let mut rows = c.query(sql, ()).await.unwrap();
    while rows.next().await.unwrap().is_some() {}
}

#[test]
fn begin_immediate_creates_one_temp_db_dir_per_connection_and_a_graceful_drop_removes_it() {
    // `TMPDIR` must be private to this probe, and `set_var` is only sound while
    // this thread is the only one — hence a hand-built current-thread runtime
    // rather than `#[tokio::test]`, whose runtime is constructed first.
    let root = std::path::PathBuf::from(format!("/tmp/tender-db-337-tmpdir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("TMPDIR", &root) };

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(probe(root.clone()));
    let _ = std::fs::remove_dir_all(&root);
}

async fn probe(root: std::path::PathBuf) {
    let path = format!("/tmp/tender-db-337-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    let count = |what: &str| {
        let n = leaked(&root).len();
        eprintln!("[337] after {what:<46} → {n} dir(s) {:?}", inside(&root));
        n
    };

    let db = turso::Builder::new_local(&path).build().await.unwrap();
    let c = db.connect().unwrap();
    drain(&c, "PRAGMA journal_mode = WAL").await;
    c.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, b INTEGER)", ()).await.unwrap();
    assert_eq!(count("open + WAL + CREATE TABLE"), 0, "opening a database must not create a temp database");

    // A deferred BEGIN emits no Transaction opcode, so a write under it is not
    // enough: it is the transaction *mode*, not the writing, that triggers this.
    c.execute("BEGIN", ()).await.unwrap();
    c.execute("INSERT INTO t(b) VALUES(1)", ()).await.unwrap();
    c.execute("COMMIT", ()).await.unwrap();
    assert_eq!(count("deferred BEGIN + INSERT + COMMIT"), 0, "a deferred transaction must not create a temp database");

    c.execute("BEGIN IMMEDIATE", ()).await.unwrap();
    assert_eq!(count("BEGIN IMMEDIATE, before any write"), 1, "BEGIN IMMEDIATE is the trigger, at the BEGIN itself");
    let first = inside(&root);
    assert!(
        first.iter().any(|f| f.ends_with("/tursodb-temp.db")),
        "the directory should hold the temp DATABASE, not a sorter spill: {first:?}"
    );

    c.execute("INSERT INTO t(b) VALUES(2)", ()).await.unwrap();
    c.execute("COMMIT", ()).await.unwrap();
    assert_eq!(count("its INSERT + COMMIT"), 1, "writing under it adds nothing");

    c.execute("BEGIN IMMEDIATE", ()).await.unwrap();
    c.execute("COMMIT", ()).await.unwrap();
    assert_eq!(count("a SECOND BEGIN IMMEDIATE, same connection"), 1, "memoised per connection — once, not once per transaction");

    let c2 = db.connect().unwrap();
    assert_eq!(count("a second connection, opened"), 1, "opening a connection is not enough; it has to transact");
    c2.execute("BEGIN IMMEDIATE", ()).await.unwrap();
    c2.execute("COMMIT", ()).await.unwrap();
    assert_eq!(count("BEGIN IMMEDIATE on the SECOND connection"), 2, "the temp database is per connection");

    // The cleanup half of the contract: this is not an unconditional library
    // leak. It only escapes when the process dies without unwinding.
    drop(c2);
    assert_eq!(count("dropping the second connection"), 1, "a graceful connection drop removes its temp directory");
    drop(c);
    assert_eq!(count("dropping the first connection"), 0, "and so does the writer's");
    drop(db);

    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
