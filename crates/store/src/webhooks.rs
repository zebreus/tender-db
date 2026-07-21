//! Webhook endpoints and their delivery state (issue 08).
//!
//! Each registered endpoint is a **consumer slot** over the one change log
//! (docs/architecture.md, docs/research/api-layer.md §4): its
//! `last_delivered_cursor` advances only after a 2xx, so delivery is
//! at-least-once and ordered, and the log itself is the queue — there is no
//! outbox table. A failing endpoint simply stops advancing; when it recovers it
//! receives the whole backlog in the next batch, never a stale single payload.
//!
//! The secret is stored in plaintext by decision (one-box threat model,
//! CONTEXT.md): unlike a password or an API token, the server must present it on
//! every delivery to compute the HMAC, so a one-way hash is not an option.

use crate::{Db, Value, int, opt_int_of, opt_text, opt_text_of, t, text};

/// Applied idempotently at startup beside the other schemas.
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS webhook_endpoints (
        id                    INTEGER PRIMARY KEY AUTOINCREMENT,
        user_id               INTEGER NOT NULL REFERENCES users(id),
        url                   TEXT NOT NULL,
        -- Standard-Webhooks signing secret, `whsec_…`, plaintext (see module doc).
        secret                TEXT NOT NULL,
        created_at            INTEGER NOT NULL, -- unix seconds
        -- Set when N days of continuous failure disable the endpoint, or by the
        -- owner. NULL = active.
        disabled_at           INTEGER,
        -- The slot position: the newest cursor this endpoint has been given a
        -- 2xx for. A fresh endpoint starts at the log's head, so it receives
        -- future changes, not the whole history.
        last_delivered_cursor INTEGER NOT NULL DEFAULT 0,
        -- Unix seconds of the first failure in the current streak (NULL when
        -- healthy) — the clock the auto-disable window is measured against.
        failing_since         INTEGER,
        -- Earliest time the sweeper may try again (backoff); 0 = immediately.
        next_attempt_at       INTEGER NOT NULL DEFAULT 0,
        consecutive_failures  INTEGER NOT NULL DEFAULT 0
    ) STRICT;
    CREATE INDEX IF NOT EXISTS webhook_endpoints_user ON webhook_endpoints(user_id);
    CREATE INDEX IF NOT EXISTS webhook_endpoints_due
        ON webhook_endpoints(next_attempt_at) WHERE disabled_at IS NULL;

    -- A short ring of recent delivery attempts, for the dashboard's debugging
    -- view. Pruned per endpoint; never the source of truth for what was
    -- delivered (that is `last_delivered_cursor`).
    CREATE TABLE IF NOT EXISTS webhook_delivery_log (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        endpoint_id  INTEGER NOT NULL REFERENCES webhook_endpoints(id),
        attempted_at INTEGER NOT NULL, -- unix seconds
        cursor_from  INTEGER NOT NULL,
        cursor_to    INTEGER NOT NULL,
        events       INTEGER NOT NULL, -- how many change events in the batch
        status       INTEGER,          -- HTTP status, or NULL for a transport error
        duration_ms  INTEGER NOT NULL,
        ok           INTEGER NOT NULL, -- 0/1
        error        TEXT
    ) STRICT;
    CREATE INDEX IF NOT EXISTS webhook_delivery_log_endpoint
        ON webhook_delivery_log(endpoint_id, id DESC);
";

/// A registered endpoint and its full delivery state.
#[derive(Clone, Debug, PartialEq)]
pub struct Endpoint {
    pub id: i64,
    pub user_id: i64,
    pub url: String,
    pub secret: String,
    pub created_at: i64,
    pub disabled_at: Option<i64>,
    pub last_delivered_cursor: i64,
    pub failing_since: Option<i64>,
    pub next_attempt_at: i64,
    pub consecutive_failures: i64,
}

/// One row of the delivery log.
#[derive(Clone, Debug, PartialEq)]
pub struct Delivery {
    pub attempted_at: i64,
    pub cursor_from: i64,
    pub cursor_to: i64,
    pub events: i64,
    pub status: Option<i64>,
    pub duration_ms: i64,
    pub ok: bool,
    pub error: Option<String>,
}

const COLUMNS: &str = "id, user_id, url, secret, created_at, disabled_at,
    last_delivered_cursor, failing_since, next_attempt_at, consecutive_failures";

fn endpoint(row: &turso::Row) -> Endpoint {
    Endpoint {
        id: int(row, 0),
        user_id: int(row, 1),
        url: text(row, 2),
        secret: text(row, 3),
        created_at: int(row, 4),
        disabled_at: opt_int_of(row, 5),
        last_delivered_cursor: int(row, 6),
        failing_since: opt_int_of(row, 7),
        next_attempt_at: int(row, 8),
        consecutive_failures: int(row, 9),
    }
}

impl Db {
    /// Register an endpoint. `start_cursor` is the log head at creation, so the
    /// endpoint's slot begins after everything already published — it receives
    /// what happens *next*, not the entire backlog.
    pub async fn create_webhook(
        &self,
        user_id: i64,
        url: &str,
        secret: &str,
        start_cursor: i64,
        now: i64,
    ) -> turso::Result<Endpoint> {
        let conn = self.conn().await;
        conn.execute(
            "INSERT INTO webhook_endpoints(user_id, url, secret, created_at, last_delivered_cursor,
                 next_attempt_at)
             VALUES(?, ?, ?, ?, ?, 0)",
            (
                Value::Integer(user_id),
                t(url),
                t(secret),
                Value::Integer(now),
                Value::Integer(start_cursor),
            ),
        )
        .await?;
        let mut rows = conn
            .query(
                &format!("SELECT {COLUMNS} FROM webhook_endpoints WHERE id = last_insert_rowid()"),
                (),
            )
            .await?;
        Ok(endpoint(&rows.next().await?.expect("the row just inserted")))
    }

    pub async fn list_webhooks(&self, user_id: i64) -> turso::Result<Vec<Endpoint>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                &format!("SELECT {COLUMNS} FROM webhook_endpoints WHERE user_id = ? ORDER BY id DESC"),
                (Value::Integer(user_id),),
            )
            .await?;
        collect(&mut rows).await
    }

    /// One of a user's own endpoints — scoping by `user_id` is what makes "a
    /// user only sees their own" a property of the query.
    pub async fn webhook(&self, user_id: i64, id: i64) -> turso::Result<Option<Endpoint>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                &format!("SELECT {COLUMNS} FROM webhook_endpoints WHERE id = ? AND user_id = ?"),
                (Value::Integer(id), Value::Integer(user_id)),
            )
            .await?;
        Ok(rows.next().await?.as_ref().map(endpoint))
    }

    /// Delete an endpoint and its delivery log, scoped to its owner.
    pub async fn delete_webhook(&self, user_id: i64, id: i64) -> turso::Result<bool> {
        let conn = self.conn().await;
        // Only owned endpoints can be targeted, so check ownership first.
        let owns = conn
            .query(
                "SELECT 1 FROM webhook_endpoints WHERE id = ? AND user_id = ?",
                (Value::Integer(id), Value::Integer(user_id)),
            )
            .await?
            .next()
            .await?
            .is_some();
        if !owns {
            return Ok(false);
        }
        conn.execute("DELETE FROM webhook_delivery_log WHERE endpoint_id = ?", (Value::Integer(id),))
            .await?;
        conn.execute("DELETE FROM webhook_endpoints WHERE id = ?", (Value::Integer(id),)).await?;
        Ok(true)
    }

    /// Disable an endpoint by the owner's choice. Idempotent.
    pub async fn disable_webhook(&self, user_id: i64, id: i64, now: i64) -> turso::Result<bool> {
        let conn = self.conn().await;
        let changed = conn
            .execute(
                "UPDATE webhook_endpoints SET disabled_at = ?
                  WHERE id = ? AND user_id = ? AND disabled_at IS NULL",
                (Value::Integer(now), Value::Integer(id), Value::Integer(user_id)),
            )
            .await?;
        Ok(changed > 0)
    }

    /// Re-enable an endpoint, clearing the failure streak. `resume_cursor`
    /// chooses where it resumes: `None` keeps the stored position (deliver the
    /// backlog it missed), `Some(c)` skips ahead (e.g. to the log head, "only
    /// new events from now").
    pub async fn enable_webhook(
        &self,
        user_id: i64,
        id: i64,
        resume_cursor: Option<i64>,
    ) -> turso::Result<bool> {
        let conn = self.conn().await;
        let cursor_set = match resume_cursor {
            Some(c) => format!(", last_delivered_cursor = {c}"),
            None => String::new(),
        };
        let changed = conn
            .execute(
                &format!(
                    "UPDATE webhook_endpoints
                        SET disabled_at = NULL, failing_since = NULL,
                            consecutive_failures = 0, next_attempt_at = 0{cursor_set}
                      WHERE id = ? AND user_id = ?"
                ),
                (Value::Integer(id), Value::Integer(user_id)),
            )
            .await?;
        Ok(changed > 0)
    }

    /// Active endpoints whose backoff has elapsed — the sweeper's work list.
    pub async fn due_webhooks(&self, now: i64) -> turso::Result<Vec<Endpoint>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {COLUMNS} FROM webhook_endpoints
                      WHERE disabled_at IS NULL AND next_attempt_at <= ? ORDER BY id"
                ),
                (Value::Integer(now),),
            )
            .await?;
        collect(&mut rows).await
    }

    /// Record a delivered batch: advance the slot and clear the failure streak.
    pub async fn webhook_delivered(&self, id: i64, cursor_to: i64) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "UPDATE webhook_endpoints
                SET last_delivered_cursor = ?, failing_since = NULL,
                    consecutive_failures = 0, next_attempt_at = 0
              WHERE id = ?",
            (Value::Integer(cursor_to), Value::Integer(id)),
        )
        .await?;
        Ok(())
    }

    /// Record a failed attempt: bump the streak, set the backoff, start the
    /// failure clock on the first failure, and disable if the caller decided the
    /// window is exhausted. The cursor is untouched — retrying re-sends the same
    /// backlog.
    pub async fn webhook_failed(
        &self,
        id: i64,
        now: i64,
        next_attempt_at: i64,
        disable: bool,
    ) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "UPDATE webhook_endpoints
                SET consecutive_failures = consecutive_failures + 1,
                    failing_since = COALESCE(failing_since, ?),
                    next_attempt_at = ?,
                    disabled_at = CASE WHEN ? = 1 THEN ? ELSE disabled_at END
              WHERE id = ?",
            (
                Value::Integer(now),
                Value::Integer(next_attempt_at),
                Value::Integer(i64::from(disable)),
                Value::Integer(now),
                Value::Integer(id),
            ),
        )
        .await?;
        Ok(())
    }

    /// Append a delivery-log row and prune the endpoint's ring to `keep` rows.
    pub async fn log_webhook_delivery(&self, id: i64, d: &Delivery, keep: i64) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "INSERT INTO webhook_delivery_log(endpoint_id, attempted_at, cursor_from, cursor_to,
                 events, status, duration_ms, ok, error)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                Value::Integer(id),
                Value::Integer(d.attempted_at),
                Value::Integer(d.cursor_from),
                Value::Integer(d.cursor_to),
                Value::Integer(d.events),
                d.status.map_or(Value::Null, Value::Integer),
                Value::Integer(d.duration_ms),
                Value::Integer(i64::from(d.ok)),
                opt_text(d.error.as_deref()),
            ),
        )
        .await?;
        conn.execute(
            "DELETE FROM webhook_delivery_log WHERE endpoint_id = ? AND id NOT IN (
                 SELECT id FROM webhook_delivery_log WHERE endpoint_id = ? ORDER BY id DESC LIMIT ?)",
            (Value::Integer(id), Value::Integer(id), Value::Integer(keep)),
        )
        .await?;
        Ok(())
    }

    pub async fn recent_webhook_deliveries(
        &self,
        id: i64,
        limit: i64,
    ) -> turso::Result<Vec<Delivery>> {
        let conn = self.reader().await?;
        let mut rows = conn
            .query(
                "SELECT attempted_at, cursor_from, cursor_to, events, status, duration_ms, ok, error
                   FROM webhook_delivery_log WHERE endpoint_id = ? ORDER BY id DESC LIMIT ?",
                (Value::Integer(id), Value::Integer(limit)),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(Delivery {
                attempted_at: int(&row, 0),
                cursor_from: int(&row, 1),
                cursor_to: int(&row, 2),
                events: int(&row, 3),
                status: opt_int_of(&row, 4),
                duration_ms: int(&row, 5),
                ok: int(&row, 6) != 0,
                error: opt_text_of(&row, 7),
            });
        }
        Ok(out)
    }
}

async fn collect(rows: &mut turso::Rows) -> turso::Result<Vec<Endpoint>> {
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(endpoint(&row));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{generate_token, hash_password};

    async fn db_with_user(name: &str) -> (Db, i64, String) {
        let path = format!("/tmp/tender-db-wh-{name}-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).await.unwrap();
        let user = db
            .create_user("hooker", &hash_password("pw").unwrap(), 0)
            .await
            .unwrap()
            .unwrap();
        (db, user.id, path)
    }

    #[tokio::test]
    async fn endpoints_round_trip_and_scope_to_owner() {
        let (db, user, path) = db_with_user("crud").await;

        let ep = db.create_webhook(user, "https://example.com/hook", "whsec_x", 5, 100).await.unwrap();
        assert_eq!(ep.url, "https://example.com/hook");
        assert_eq!(ep.last_delivered_cursor, 5, "starts at the log head");
        assert_eq!(ep.consecutive_failures, 0);

        assert_eq!(db.list_webhooks(user).await.unwrap().len(), 1);
        assert_eq!(db.webhook(user, ep.id).await.unwrap(), Some(ep.clone()));
        // A different user cannot see or delete it.
        let other = db.create_user("mal", &hash_password("pw").unwrap(), 0).await.unwrap().unwrap();
        assert!(db.webhook(other.id, ep.id).await.unwrap().is_none());
        assert!(!db.delete_webhook(other.id, ep.id).await.unwrap());

        assert!(db.delete_webhook(user, ep.id).await.unwrap());
        assert!(db.list_webhooks(user).await.unwrap().is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn success_advances_the_slot_and_clears_failures() {
        let (db, user, path) = db_with_user("advance").await;
        let ep = db.create_webhook(user, "https://e/h", generate_token().as_str(), 0, 0).await.unwrap();

        // A failure sets the backoff and starts the failure clock.
        db.webhook_failed(ep.id, 100, 130, false).await.unwrap();
        let after = db.webhook(user, ep.id).await.unwrap().unwrap();
        assert_eq!(after.consecutive_failures, 1);
        assert_eq!(after.failing_since, Some(100));
        assert_eq!(after.next_attempt_at, 130);
        assert_eq!(after.last_delivered_cursor, 0, "a failure never advances the cursor");

        // It is not due until the backoff elapses.
        assert!(db.due_webhooks(120).await.unwrap().is_empty());
        assert_eq!(db.due_webhooks(130).await.unwrap().len(), 1);

        // Success advances the slot and wipes the streak.
        db.webhook_delivered(ep.id, 42).await.unwrap();
        let healed = db.webhook(user, ep.id).await.unwrap().unwrap();
        assert_eq!(healed.last_delivered_cursor, 42);
        assert_eq!(healed.consecutive_failures, 0);
        assert_eq!(healed.failing_since, None);

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn disable_hides_from_the_sweeper_and_enable_restores() {
        let (db, user, path) = db_with_user("disable").await;
        let ep = db.create_webhook(user, "https://e/h", "whsec_x", 0, 0).await.unwrap();

        // Auto-disable via a failure with the disable flag set.
        db.webhook_failed(ep.id, 200, 200, true).await.unwrap();
        assert!(db.webhook(user, ep.id).await.unwrap().unwrap().disabled_at.is_some());
        assert!(db.due_webhooks(10_000).await.unwrap().is_empty(), "disabled endpoints are not due");

        // Re-enable keeping position clears the streak and makes it due again.
        assert!(db.enable_webhook(user, ep.id, None).await.unwrap());
        let back = db.webhook(user, ep.id).await.unwrap().unwrap();
        assert!(back.disabled_at.is_none());
        assert_eq!(back.consecutive_failures, 0);
        assert_eq!(db.due_webhooks(0).await.unwrap().len(), 1);

        // Re-enable to a new head skips the backlog.
        db.enable_webhook(user, ep.id, Some(99)).await.unwrap();
        assert_eq!(db.webhook(user, ep.id).await.unwrap().unwrap().last_delivered_cursor, 99);

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn delivery_log_is_a_bounded_ring() {
        let (db, user, path) = db_with_user("log").await;
        let ep = db.create_webhook(user, "https://e/h", "whsec_x", 0, 0).await.unwrap();
        for i in 0..5 {
            let d = Delivery {
                attempted_at: i,
                cursor_from: i,
                cursor_to: i + 1,
                events: 1,
                status: Some(200),
                duration_ms: 10,
                ok: true,
                error: None,
            };
            db.log_webhook_delivery(ep.id, &d, 3).await.unwrap();
        }
        let recent = db.recent_webhook_deliveries(ep.id, 10).await.unwrap();
        assert_eq!(recent.len(), 3, "pruned to the ring size");
        assert_eq!(recent[0].attempted_at, 4, "newest first");
        let _ = std::fs::remove_file(&path);
    }
}
