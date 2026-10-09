//! One way to run a write transaction on the shared writer (issue 498).
//!
//! The writer is ONE connection, so a transaction its holder leaves open is the next
//! holder's problem: its `BEGIN` fails, and its autocommit writes join the leak. turso
//! often aborts only the failing statement and leaves the transaction open (a constraint
//! error, any error in a journaled write); other errors (a generic one on a read or an
//! unjournaled write, a `RAISE(ROLLBACK)`) end the transaction itself. The caller cannot
//! tell which, so every error between a `BEGIN` and its `COMMIT` must be followed by a
//! `ROLLBACK` unless the connection is already back in autocommit. The 2026-10-09 census found
//! 22 sites where some `?` skipped it. [`Db::immediate`] makes that impossible to forget:
//! the body's error and the `COMMIT`'s error both roll back, and the original error is
//! what the caller sees. (`Db::conn_for`'s handover check is the net under the cases no
//! helper can reach: a panic or a cancelled future mid-transaction.)

use std::future::Future;

use turso::Connection;

use crate::Db;

impl Db {
    /// `BEGIN IMMEDIATE`, run `body`, `COMMIT`. On ANY error from the body or the
    /// `COMMIT`: a best-effort `ROLLBACK`, and the ORIGINAL error is returned.
    ///
    /// `body` must be lazy (an `async {}` block or an `async fn` call), so it first runs
    /// AFTER the `BEGIN` succeeds.
    pub(crate) fn immediate<'a, T: 'a>(
        conn: &'a Connection,
        body: impl Future<Output = turso::Result<T>> + 'a,
    ) -> impl Future<Output = turso::Result<T>> + 'a {
        Self::within(conn, "BEGIN IMMEDIATE", body)
    }

    /// [`Db::immediate`] with an explicit `BEGIN` form, for the sites that open a plain
    /// (deferred) `BEGIN` and must convert byte-identically.
    ///
    /// A plain fn that boxes `body` before its async block, on purpose. As an `async fn`
    /// the body was held about three times over (the argument, the awaitee, and again in
    /// `immediate`'s state), so every caller's future, and its O0 poll frame, grew by
    /// several bodies. That is the growth issue 467's size budgets and CLAUDE.md's
    /// `run_spec` stack overflow warn about. Boxed, the caller holds one pointer, at the
    /// cost of one allocation per transaction. Boxing does not poll, so the body still
    /// first runs after the `BEGIN`.
    pub(crate) fn within<'a, T: 'a>(
        conn: &'a Connection,
        begin: &'static str,
        body: impl Future<Output = turso::Result<T>> + 'a,
    ) -> impl Future<Output = turso::Result<T>> + 'a {
        let body = Box::pin(body);
        async move {
            // A failed BEGIN opened nothing, so there is nothing to roll back — and a
            // transaction this call did not open is never this call's to end.
            conn.execute(begin, ()).await?;
            finish(conn, body.await).await
        }
    }
}

/// `COMMIT` `conn`'s open transaction when `result` is `Ok`; otherwise, or when the
/// `COMMIT` fails, roll it back. Hands back `result`, or the `COMMIT`'s error.
pub(crate) async fn finish<T>(conn: &Connection, result: turso::Result<T>) -> turso::Result<T> {
    let err = match result {
        Ok(value) => match conn.execute("COMMIT", ()).await {
            Ok(_) => return Ok(value),
            Err(e) => e,
        },
        Err(e) => e,
    };
    rollback_best_effort(conn).await;
    Err(err)
}

async fn rollback_best_effort(conn: &Connection) {
    // turso sometimes ends the transaction itself (a `RAISE(ROLLBACK)`, a poisoned
    // transaction at COMMIT): then there is nothing left to roll back.
    if conn.is_autocommit().unwrap_or(false) {
        return;
    }
    // The ROLLBACK's own error never replaces the original one.
    let _ = conn.execute("ROLLBACK", ()).await;
    if !conn.is_autocommit().unwrap_or(true) {
        eprintln!("[store] ROLLBACK left the shared writer inside its transaction (issue 498)");
    }
}

#[cfg(test)]
mod tests {
    use turso::Value;

    use super::*;

    async fn scratch(case: &str) -> (Db, String) {
        let path = format!("/tmp/tender-db-tx498-{case}-{}.db", std::process::id());
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
        (Db::open(&path).await.expect("open scratch db"), path)
    }

    fn remove(path: &str) {
        for s in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{s}"));
        }
    }

    async fn count(conn: &Connection, table: &str) -> i64 {
        let mut rows = conn.query(&format!("SELECT COUNT(*) FROM {table}"), ()).await.unwrap();
        match rows.next().await.unwrap().unwrap().get_value(0).unwrap() {
            Value::Integer(n) => n,
            other => panic!("COUNT(*) gave {other:?}"),
        }
    }

    /// The connection is usable afterwards: a fresh transaction opens and commits.
    async fn assert_reusable(conn: &Connection) {
        assert!(conn.is_autocommit().unwrap(), "the connection is back in autocommit");
        conn.execute("BEGIN IMMEDIATE", ()).await.expect("a fresh BEGIN works");
        conn.execute("COMMIT", ()).await.expect("and commits");
    }

    #[tokio::test]
    async fn a_failing_body_rolls_back_and_returns_its_own_error() {
        let (db, path) = scratch("body").await;
        let conn = db.conn().await;
        conn.execute("CREATE TABLE t498(x INTEGER PRIMARY KEY)", ()).await.unwrap();

        // Control: by hand, turso leaves the transaction open after a failed statement —
        // so the assertions below are not vacuous.
        conn.execute("BEGIN IMMEDIATE", ()).await.unwrap();
        conn.execute("INSERT INTO t498 VALUES (1)", ()).await.unwrap();
        assert!(conn.execute("INSERT INTO t498 VALUES (1)", ()).await.is_err());
        assert!(!conn.is_autocommit().unwrap(), "control: the failed statement left the transaction open");
        conn.execute("ROLLBACK", ()).await.unwrap();

        let err = Db::immediate(&conn, async {
            conn.execute("INSERT INTO t498 VALUES (1)", ()).await?;
            conn.execute("INSERT INTO t498 VALUES (1)", ()).await?;
            Ok(())
        })
        .await
        .expect_err("the duplicate key fails the body");
        assert!(err.to_string().to_uppercase().contains("UNIQUE"), "the body's own error comes back: {err}");
        assert_reusable(&conn).await;
        assert_eq!(count(&conn, "t498").await, 0, "none of the failed body's rows survive");

        // A body that fails by returning an error of its own, after a successful write.
        let err = Db::immediate(&conn, async {
            conn.execute("INSERT INTO t498 VALUES (7)", ()).await?;
            Err::<(), _>(turso::Error::Error("boom".into()))
        })
        .await
        .expect_err("the body's Err");
        assert_eq!(err.to_string(), turso::Error::Error("boom".into()).to_string());
        assert_reusable(&conn).await;
        assert_eq!(count(&conn, "t498").await, 0);
        drop(conn);
        remove(&path);
    }

    #[tokio::test]
    async fn a_failing_commit_rolls_back_and_returns_the_commit_error() {
        let (db, path) = scratch("commit").await;
        let conn = db.conn().await;
        conn.execute_batch(
            "CREATE TABLE p498(id INTEGER PRIMARY KEY);
             CREATE TABLE c498(id INTEGER PRIMARY KEY,
                               p INTEGER REFERENCES p498(id) DEFERRABLE INITIALLY DEFERRED);",
        )
        .await
        .unwrap();

        // Control: a deferred FK violation fails the COMMIT and leaves the transaction open.
        conn.execute("BEGIN IMMEDIATE", ()).await.unwrap();
        conn.execute("INSERT INTO c498 VALUES (1, 42)", ()).await.unwrap();
        assert!(conn.execute("COMMIT", ()).await.is_err(), "control: the deferred FK fails the COMMIT");
        assert!(!conn.is_autocommit().unwrap(), "control: the failed COMMIT left the transaction open");
        conn.execute("ROLLBACK", ()).await.unwrap();

        let err = Db::immediate(&conn, async {
            conn.execute("INSERT INTO c498 VALUES (1, 42)", ()).await?;
            Ok(())
        })
        .await
        .expect_err("the COMMIT fails");
        assert!(err.to_string().contains("foreign key"), "the COMMIT's error comes back: {err}");
        // This also proves the ROLLBACK cleared the deferred-violation count, or this
        // COMMIT would fail again.
        assert_reusable(&conn).await;
        assert_eq!(count(&conn, "c498").await, 0);
        drop(conn);
        remove(&path);
    }

    #[tokio::test]
    async fn a_transaction_the_engine_already_ended_is_not_rolled_back_twice() {
        let (db, path) = scratch("raise").await;
        let conn = db.conn().await;
        conn.execute_batch(
            "CREATE TABLE t498(x INTEGER PRIMARY KEY);
             CREATE TRIGGER r498 BEFORE INSERT ON t498 WHEN NEW.x = 2
             BEGIN SELECT RAISE(ROLLBACK, 'r498 refused'); END;",
        )
        .await
        .unwrap();
        let err = Db::immediate(&conn, async {
            conn.execute("INSERT INTO t498 VALUES (1)", ()).await?;
            conn.execute("INSERT INTO t498 VALUES (2)", ()).await?;
            Ok(())
        })
        .await
        .expect_err("the trigger refuses");
        assert!(err.to_string().contains("r498 refused"), "the trigger's error comes back: {err}");
        assert_reusable(&conn).await;
        assert_eq!(count(&conn, "t498").await, 0);
        drop(conn);
        remove(&path);
    }

    #[tokio::test]
    async fn a_failing_begin_runs_nothing_and_ends_nothing() {
        let (db, path) = scratch("begin").await;
        let conn = db.conn().await;
        conn.execute("CREATE TABLE t498(x INTEGER PRIMARY KEY)", ()).await.unwrap();
        conn.execute("BEGIN IMMEDIATE", ()).await.unwrap();
        conn.execute("INSERT INTO t498 VALUES (1)", ()).await.unwrap();
        let mut ran = false;
        let result = Db::immediate(&conn, async {
            ran = true;
            Ok(())
        })
        .await;
        assert!(result.is_err(), "a BEGIN inside a transaction fails");
        assert!(!ran, "the body never ran");
        assert!(!conn.is_autocommit().unwrap(), "the outer transaction is not the helper's to end");
        conn.execute("COMMIT", ()).await.unwrap();
        assert_eq!(count(&conn, "t498").await, 1, "the outer transaction's write is intact");
        drop(conn);
        remove(&path);
    }

    #[tokio::test]
    async fn a_successful_body_commits_and_returns_its_value() {
        let (db, path) = scratch("ok").await;
        let conn = db.conn().await;
        conn.execute("CREATE TABLE t498(x INTEGER PRIMARY KEY)", ()).await.unwrap();
        let n = Db::immediate(&conn, async {
            let a = conn.execute("INSERT INTO t498 VALUES (1)", ()).await?;
            let b = conn.execute("INSERT INTO t498 VALUES (2)", ()).await?;
            Ok(a + b)
        })
        .await
        .unwrap();
        assert_eq!(n, 2);
        assert!(conn.is_autocommit().unwrap());
        assert_eq!(count(&conn, "t498").await, 2);
        let n = Db::within(&conn, "BEGIN", async { conn.execute("INSERT INTO t498 VALUES (3)", ()).await })
            .await
            .unwrap();
        assert_eq!(n, 1);
        assert_eq!(count(&conn, "t498").await, 3);
        drop(conn);
        remove(&path);
    }
}
