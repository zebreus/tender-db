//! Accounts: users, dashboard sessions, and API tokens.
//!
//! Username + password only — there is no email anywhere in this module, by
//! product decision (CONTEXT.md), which is also why a lost password is a lost
//! account: nothing here can prove a claim to one.
//!
//! Two credentials, deliberately separate (docs/research/api-layer.md §5): the
//! browser dashboard carries a **session** cookie, programmatic clients send a
//! **`tdb_` bearer token**. Both are high-entropy random secrets stored only as
//! their SHA-256 — a database leak leaks nothing usable — and both are looked
//! up by exact hash match, which also settles any timing concern. Passwords are
//! the one low-entropy secret, so they get argon2id (OWASP defaults) and a PHC
//! string instead.

use crate::{Db, int, opt_int_of, t, text};
use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use sha2::{Digest, Sha256};
use turso::{Connection, Value};

/// Applied idempotently at startup beside the notice and canonical schemas.
pub(crate) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS users (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        username      TEXT NOT NULL UNIQUE,
        -- The argon2id PHC string, parameters and salt included, so a later
        -- parameter bump can rehash on next login without a schema change.
        password_hash TEXT NOT NULL,
        created_at    INTEGER NOT NULL -- unix seconds
    ) STRICT;

    -- `prefix_hint` is the visible head of the token (`tdb_` plus a few
    -- characters): enough for the owner to tell two tokens apart in the
    -- revoke list, not enough to be a credential.
    CREATE TABLE IF NOT EXISTS api_tokens (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        user_id      INTEGER NOT NULL REFERENCES users(id),
        token_hash   TEXT NOT NULL UNIQUE,
        prefix_hint  TEXT NOT NULL,
        name         TEXT NOT NULL,
        created_at   INTEGER NOT NULL,
        last_used_at INTEGER,
        revoked_at   INTEGER
    ) STRICT;
    CREATE INDEX IF NOT EXISTS api_tokens_user ON api_tokens(user_id);

    -- Sessions live in the database rather than in a signed stateless cookie so
    -- that logout and account deletion revoke instantly.
    CREATE TABLE IF NOT EXISTS sessions (
        id_hash    TEXT PRIMARY KEY,
        user_id    INTEGER NOT NULL REFERENCES users(id),
        created_at INTEGER NOT NULL,
        expires_at INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX IF NOT EXISTS sessions_user ON sessions(user_id);
";

/// How long a dashboard session stays valid without re-login.
pub const SESSION_LIFETIME: i64 = 30 * 24 * 60 * 60;

/// The recognizable prefix every API token carries, GitHub-style: it makes
/// leaked tokens findable by secret scanners and obvious in a support request.
pub const TOKEN_PREFIX: &str = "tdb_";

/// An account, as everything outside this module sees it. The password hash
/// never leaves.
#[derive(Clone, Debug, PartialEq)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub created_at: i64,
}

/// An API token's metadata — everything about a token except the token.
#[derive(Clone, Debug, PartialEq)]
pub struct TokenRecord {
    pub id: i64,
    pub name: String,
    pub prefix_hint: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

// ------------------------------------------------------------- secret making

/// Lowercase hex SHA-256 — how every bearer secret in this module is stored.
///
/// A fast hash is the right choice here (and what GitHub does): the input is
/// 256 bits of OS randomness, so there is nothing to brute-force. Passwords,
/// which are guessable, go through [`hash_password`] instead.
pub fn digest(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn random_hex(bytes: usize) -> String {
    random_bytes(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// `n` bytes from the OS CSPRNG — the raw material for every secret in the
/// project. Exposed so the webhook layer can draw its signing key from the same
/// source without its own RNG dependency.
pub fn random_bytes(n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    OsRng.fill_bytes(&mut buf);
    buf
}

/// A fresh API token: `tdb_` + 256 bits of OS randomness. Shown to its owner
/// exactly once — only its digest is ever stored.
pub fn generate_token() -> String {
    format!("{TOKEN_PREFIX}{}", random_hex(32))
}

/// A fresh session id (128 bits, per api-layer.md §5).
pub fn generate_session_id() -> String {
    random_hex(16)
}

/// The head of a token, kept in the clear so the owner can identify it later.
pub fn prefix_hint(token: &str) -> String {
    token.chars().take(TOKEN_PREFIX.len() + 6).collect()
}

/// Argon2id with the crate's defaults, which are OWASP's recommended
/// parameters (m=19 MiB, t=2, p=1). Deliberately ~50–100 ms of CPU, so callers
/// run it on a blocking thread.
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default().hash_password(password.as_bytes(), &salt)?.to_string())
}

/// Verify against a stored PHC string. A malformed hash verifies as false
/// rather than erroring — a corrupt row must not become an authentication
/// bypass or a 500.
pub fn verify_password(password: &str, phc: &str) -> bool {
    PasswordHash::new(phc)
        .map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
        .unwrap_or(false)
}

// ------------------------------------------------------------------ accessors

impl Db {
    /// Create an account. Returns `None` when the username is taken — the
    /// UNIQUE index is the arbiter, so two simultaneous registrations of one
    /// name cannot both win.
    pub async fn create_user(
        &self,
        username: &str,
        password_hash: &str,
        now: i64,
    ) -> turso::Result<Option<User>> {
        let conn = self.conn().await;
        let changed = conn
            .execute(
                "INSERT OR IGNORE INTO users(username, password_hash, created_at) VALUES(?, ?, ?)",
                (t(username), t(password_hash), Value::Integer(now)),
            )
            .await?;
        if changed == 0 {
            return Ok(None);
        }
        user_row(&conn, "SELECT id, username, created_at FROM users WHERE username = ?", t(username))
            .await
    }

    pub async fn user(&self, id: i64) -> turso::Result<Option<User>> {
        let conn = self.conn().await;
        user_row(&conn, "SELECT id, username, created_at FROM users WHERE id = ?", Value::Integer(id)).await
    }

    /// The account and its stored password hash, for a login attempt.
    pub async fn user_credentials(&self, username: &str) -> turso::Result<Option<(User, String)>> {
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT id, username, created_at, password_hash FROM users WHERE username = ?",
                (t(username),),
            )
            .await?;
        Ok(rows.next().await?.map(|row| {
            (
                User { id: int(&row, 0), username: text(&row, 1), created_at: int(&row, 2) },
                text(&row, 3),
            )
        }))
    }

    /// Delete an account and everything that authenticates as it. Explicit
    /// child deletes in one transaction rather than `ON DELETE CASCADE`: the
    /// order is then ours, not the engine's, and it holds regardless of what
    /// the storage layer implements.
    pub async fn delete_user(&self, id: i64) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute("BEGIN IMMEDIATE", ()).await?;
        let result = async {
            for sql in [
                "DELETE FROM api_tokens WHERE user_id = ?",
                "DELETE FROM sessions WHERE user_id = ?",
                "DELETE FROM users WHERE id = ?",
            ] {
                conn.execute(sql, (Value::Integer(id),)).await?;
            }
            turso::Result::Ok(())
        }
        .await;
        match result {
            Ok(()) => conn.execute("COMMIT", ()).await.map(|_| ()),
            Err(e) => {
                // turso 0.7.0 poisons the transaction if a write future is
                // abandoned, so the rollback is unconditional on the error path.
                let _ = conn.execute("ROLLBACK", ()).await;
                Err(e)
            }
        }
    }

    // ------------------------------------------------------------- sessions

    pub async fn create_session(&self, id_hash: &str, user_id: i64, now: i64) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute(
            "INSERT INTO sessions(id_hash, user_id, created_at, expires_at) VALUES(?, ?, ?, ?)",
            (
                t(id_hash),
                Value::Integer(user_id),
                Value::Integer(now),
                Value::Integer(now + SESSION_LIFETIME),
            ),
        )
        .await?;
        Ok(())
    }

    /// Whose session this is, or `None` if it is unknown or expired. Expiry is
    /// evaluated in SQL so a clock-skewed cookie cannot outlive its row.
    pub async fn session_user(&self, id_hash: &str, now: i64) -> turso::Result<Option<User>> {
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT u.id, u.username, u.created_at FROM sessions s
                   JOIN users u ON u.id = s.user_id
                  WHERE s.id_hash = ? AND s.expires_at > ?",
                (t(id_hash), Value::Integer(now)),
            )
            .await?;
        Ok(rows.next().await?.map(|row| User {
            id: int(&row, 0),
            username: text(&row, 1),
            created_at: int(&row, 2),
        }))
    }

    pub async fn delete_session(&self, id_hash: &str) -> turso::Result<()> {
        let conn = self.conn().await;
        conn.execute("DELETE FROM sessions WHERE id_hash = ?", (t(id_hash),)).await?;
        Ok(())
    }

    // --------------------------------------------------------------- tokens

    /// Store a freshly generated token's digest. The token itself is the
    /// caller's to show once and then forget.
    pub async fn create_token(
        &self,
        user_id: i64,
        token: &str,
        name: &str,
        now: i64,
    ) -> turso::Result<TokenRecord> {
        let conn = self.conn().await;
        let hint = prefix_hint(token);
        conn.execute(
            "INSERT INTO api_tokens(user_id, token_hash, prefix_hint, name, created_at)
             VALUES(?, ?, ?, ?, ?)",
            (
                Value::Integer(user_id),
                t(digest(token)),
                t(hint.clone()),
                t(name),
                Value::Integer(now),
            ),
        )
        .await?;
        let mut rows = conn
            .query("SELECT id FROM api_tokens WHERE token_hash = ?", (t(digest(token)),))
            .await?;
        let id = rows.next().await?.map_or(0, |row| int(&row, 0));
        Ok(TokenRecord {
            id,
            name: name.to_owned(),
            prefix_hint: hint,
            created_at: now,
            last_used_at: None,
            revoked_at: None,
        })
    }

    /// An account's tokens, revoked ones included — the revoke list is also the
    /// audit trail of what was ever issued.
    pub async fn list_tokens(&self, user_id: i64) -> turso::Result<Vec<TokenRecord>> {
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT id, name, prefix_hint, created_at, last_used_at, revoked_at
                   FROM api_tokens WHERE user_id = ? ORDER BY id DESC",
                (Value::Integer(user_id),),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(TokenRecord {
                id: int(&row, 0),
                name: text(&row, 1),
                prefix_hint: text(&row, 2),
                created_at: int(&row, 3),
                last_used_at: opt_int_of(&row, 4),
                revoked_at: opt_int_of(&row, 5),
            });
        }
        Ok(out)
    }

    /// Revoke one of an account's own tokens. Scoping the UPDATE by `user_id`
    /// is what makes "one user cannot revoke another's token" a property of the
    /// statement rather than of a caller's check.
    pub async fn revoke_token(&self, user_id: i64, token_id: i64, now: i64) -> turso::Result<bool> {
        let conn = self.conn().await;
        let changed = conn
            .execute(
                "UPDATE api_tokens SET revoked_at = ?
                  WHERE id = ? AND user_id = ? AND revoked_at IS NULL",
                (Value::Integer(now), Value::Integer(token_id), Value::Integer(user_id)),
            )
            .await?;
        Ok(changed > 0)
    }

    /// Resolve a presented bearer token to its account, recording the use.
    ///
    /// The `last_used_at` touch is best-effort: it is what makes a stale token
    /// visible in the dashboard, and losing one write is preferable to failing
    /// an otherwise valid request.
    pub async fn authenticate_token(&self, token: &str, now: i64) -> turso::Result<Option<User>> {
        let hash = digest(token);
        let conn = self.conn().await;
        let mut rows = conn
            .query(
                "SELECT u.id, u.username, u.created_at FROM api_tokens k
                   JOIN users u ON u.id = k.user_id
                  WHERE k.token_hash = ? AND k.revoked_at IS NULL",
                (t(hash.clone()),),
            )
            .await?;
        let Some(row) = rows.next().await? else { return Ok(None) };
        let user =
            User { id: int(&row, 0), username: text(&row, 1), created_at: int(&row, 2) };
        drop(rows);
        let _ = conn
            .execute(
                "UPDATE api_tokens SET last_used_at = ? WHERE token_hash = ?",
                (Value::Integer(now), t(hash)),
            )
            .await;
        Ok(Some(user))
    }
}

async fn user_row(conn: &Connection, sql: &str, key: Value) -> turso::Result<Option<User>> {
    let mut rows = conn.query(sql, (key,)).await?;
    Ok(rows.next().await?.map(|row| User {
        id: int(&row, 0),
        username: text(&row, 1),
        created_at: int(&row, 2),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn db(name: &str) -> (Db, String) {
        let path = format!("/tmp/tender-db-{name}-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        (Db::open(&path).await.expect("open scratch db"), path)
    }

    #[tokio::test]
    async fn passwords_hash_and_verify() {
        let phc = hash_password("correct horse").expect("hash");
        assert!(phc.starts_with("$argon2id$"), "PHC string: {phc}");
        assert!(verify_password("correct horse", &phc));
        assert!(!verify_password("correct hors", &phc));
        // A corrupt hash denies access rather than erroring.
        assert!(!verify_password("correct horse", "not a PHC string"));
        // The salt is per-hash, so the same password hashes differently twice.
        assert_ne!(phc, hash_password("correct horse").expect("hash"));
    }

    #[tokio::test]
    async fn accounts_round_trip() {
        let (db, path) = db("accounts").await;

        let phc = hash_password("hunter2").expect("hash");
        let user = db.create_user("ada", &phc, 100).await.expect("create").expect("fresh username");
        assert_eq!(user.username, "ada");
        // Usernames are unique; the second registration loses.
        assert!(db.create_user("ada", &phc, 101).await.expect("create").is_none());

        let (found, stored) = db.user_credentials("ada").await.expect("lookup").expect("exists");
        assert_eq!(found, user);
        assert!(verify_password("hunter2", &stored));
        assert!(db.user_credentials("nobody").await.expect("lookup").is_none());

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn tokens_are_found_by_hash_and_revocable() {
        let (db, path) = db("tokens").await;
        let user = db
            .create_user("grace", &hash_password("pw").expect("hash"), 0)
            .await
            .expect("create")
            .expect("fresh");

        let token = generate_token();
        assert!(token.starts_with("tdb_") && token.len() == 68);
        let record = db.create_token(user.id, &token, "ci", 10).await.expect("create token");
        assert_eq!(record.prefix_hint, token[..10]);

        // The plaintext is nowhere in the database — only its digest is.
        assert_eq!(
            db.authenticate_token(&token, 20).await.expect("auth"),
            Some(user.clone())
        );
        assert!(db.authenticate_token(&generate_token(), 20).await.expect("auth").is_none());

        // Authenticating touched last_used_at.
        let listed = db.list_tokens(user.id).await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].last_used_at, Some(20));

        // Another account cannot revoke it; its owner can, once.
        let other = db
            .create_user("mallory", &hash_password("pw").expect("hash"), 0)
            .await
            .expect("create")
            .expect("fresh");
        assert!(!db.revoke_token(other.id, record.id, 30).await.expect("revoke"));
        assert!(db.revoke_token(user.id, record.id, 30).await.expect("revoke"));
        assert!(!db.revoke_token(user.id, record.id, 31).await.expect("revoke"));
        assert!(db.authenticate_token(&token, 40).await.expect("auth").is_none());

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn sessions_expire_and_are_revocable() {
        let (db, path) = db("sessions").await;
        let user = db
            .create_user("linus", &hash_password("pw").expect("hash"), 0)
            .await
            .expect("create")
            .expect("fresh");

        let id = generate_session_id();
        db.create_session(&digest(&id), user.id, 1_000).await.expect("create session");

        assert_eq!(db.session_user(&digest(&id), 1_001).await.expect("lookup"), Some(user.clone()));
        // One second past expiry it is gone, without anything having deleted it.
        let expired = 1_000 + SESSION_LIFETIME + 1;
        assert!(db.session_user(&digest(&id), expired).await.expect("lookup").is_none());
        // The raw id is not the key: only its digest is stored.
        assert!(db.session_user(&id, 1_001).await.expect("lookup").is_none());

        db.delete_session(&digest(&id)).await.expect("logout");
        assert!(db.session_user(&digest(&id), 1_001).await.expect("lookup").is_none());

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn deleting_an_account_takes_its_credentials_with_it() {
        let (db, path) = db("delete").await;
        let user = db
            .create_user("ken", &hash_password("pw").expect("hash"), 0)
            .await
            .expect("create")
            .expect("fresh");
        let token = generate_token();
        db.create_token(user.id, &token, "cli", 0).await.expect("token");
        let session = generate_session_id();
        db.create_session(&digest(&session), user.id, 0).await.expect("session");

        db.delete_user(user.id).await.expect("delete account");

        assert!(db.user(user.id).await.expect("lookup").is_none());
        assert!(db.authenticate_token(&token, 1).await.expect("auth").is_none());
        assert!(db.session_user(&digest(&session), 1).await.expect("lookup").is_none());
        assert!(db.list_tokens(user.id).await.expect("list").is_empty());
        // The username is free again — nothing of the account survives.
        assert!(db.create_user("ken", "x", 2).await.expect("create").is_some());

        let _ = std::fs::remove_file(&path);
    }
}
