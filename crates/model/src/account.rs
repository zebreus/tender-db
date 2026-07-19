//! The account lifecycle across the client/server boundary.
//!
//! Username and password only — no email exists anywhere in tender-db, which is
//! why [`LOST_PASSWORD_NOTICE`] is shown at registration: it is the whole
//! recovery story, and the user has to see it before it applies to them.

use serde::{Deserialize, Serialize};

/// Shown at registration, per CONTEXT.md. There is no email, so there is no
/// reset link, no support recovery, and nothing to appeal to.
pub const LOST_PASSWORD_NOTICE: &str = "There is no email address on your account and no password \
    reset. If you lose your password you lose the account and every API token on it. Store it in a \
    password manager now.";

/// Who you are signed in as.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub username: String,
    pub created_at: i64,
}

/// An API token's metadata — the revoke list's row. The token itself appears
/// only once, in [`NewToken`], and is never retrievable again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub id: i64,
    pub name: String,
    /// The visible head of the token, enough to tell two apart.
    pub prefix_hint: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

/// The one and only time the plaintext of a token crosses the wire.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewToken {
    pub token: String,
    pub record: Token,
}
