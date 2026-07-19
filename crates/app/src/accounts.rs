//! The account lifecycle, as plain async functions over the store.
//!
//! Deliberately not server functions: the Dioxus server functions in the binary
//! wrap these and do nothing but move cookies around, so the lifecycle itself
//! stays callable from an integration test without a browser bundle in sight.
//!
//! Cookie policy lives here too (one place, one set of flags), because the
//! session cookie *is* half of the login contract.

use model::{Account, NewToken, Token};
use store::Db;
use store::accounts::{
    SESSION_LIFETIME, digest, generate_session_id, generate_token, hash_password, verify_password,
};

/// The session cookie's name. The `__Host-` prefix would be stricter still, but
/// it forbids `Domain` and demands `Secure` — which would break `dx serve` over
/// plain http on localhost, so the flags below carry the weight instead.
pub const SESSION_COOKIE: &str = "tdb_session";

/// Everything that can go wrong in the account lifecycle, kept small on
/// purpose: a login failure never says *which* half was wrong.
#[derive(Debug)]
pub enum AuthError {
    /// The username is already registered.
    Taken,
    /// Wrong username or wrong password — indistinguishable by design.
    InvalidCredentials,
    /// The request was not a well-formed account operation.
    Invalid(String),
    /// Not signed in, or signed in as someone the database no longer has.
    NotSignedIn,
    Db(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Taken => f.write_str("that username is taken"),
            AuthError::InvalidCredentials => f.write_str("wrong username or password"),
            AuthError::Invalid(why) => f.write_str(why),
            AuthError::NotSignedIn => f.write_str("sign in first"),
            AuthError::Db(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<store::turso::Error> for AuthError {
    fn from(e: store::turso::Error) -> AuthError {
        AuthError::Db(e.to_string())
    }
}

type Result<T> = std::result::Result<T, AuthError>;

// ------------------------------------------------------------------ policies

const MIN_PASSWORD: usize = 10;
const MAX_USERNAME: usize = 32;
const MIN_USERNAME: usize = 3;

/// Usernames are the only public handle an account has, so they are restricted
/// to characters that cannot be confused for one another or for markup.
fn check_username(username: &str) -> Result<()> {
    let ok = username.len() >= MIN_USERNAME
        && username.len() <= MAX_USERNAME
        && username.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(AuthError::Invalid(format!(
            "username must be {MIN_USERNAME}–{MAX_USERNAME} characters of a–z, 0–9, - or _"
        )))
    }
}

/// A length floor and nothing else: composition rules push users towards
/// memorable-but-weak passwords, and there is no reset here to fall back on.
fn check_password(password: &str) -> Result<()> {
    if password.chars().count() >= MIN_PASSWORD {
        Ok(())
    } else {
        Err(AuthError::Invalid(format!("password must be at least {MIN_PASSWORD} characters")))
    }
}

// ------------------------------------------------------------------- cookies

/// The `Set-Cookie` value for a fresh session.
///
/// `HttpOnly` keeps it away from any script, `SameSite=Lax` blocks cross-site
/// use while surviving ordinary top-level navigation, and `Secure` confines it
/// to https — with an escape hatch for local development, where the dev server
/// speaks plain http and a `Secure` cookie would simply never come back.
pub fn session_cookie(session_id: &str) -> String {
    let secure = if insecure_cookies() { "" } else { " Secure;" };
    format!(
        "{SESSION_COOKIE}={session_id}; Path=/; HttpOnly;{secure} SameSite=Lax; Max-Age={SESSION_LIFETIME}"
    )
}

/// The `Set-Cookie` value that clears the session cookie on logout.
pub fn cleared_cookie() -> String {
    let secure = if insecure_cookies() { "" } else { " Secure;" };
    format!("{SESSION_COOKIE}=; Path=/; HttpOnly;{secure} SameSite=Lax; Max-Age=0")
}

/// `TENDER_DB_INSECURE_COOKIES=1` drops the `Secure` flag for local http
/// development. Production sets nothing and gets the strict cookie.
fn insecure_cookies() -> bool {
    std::env::var("TENDER_DB_INSECURE_COOKIES").is_ok_and(|v| v == "1")
}

/// Pull our session id out of a `Cookie:` header value.
pub fn session_from_cookies(header: &str) -> Option<String> {
    header.split(';').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name.trim() == SESSION_COOKIE).then(|| value.trim().to_owned())
    })
}

// ---------------------------------------------------------------- lifecycle

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

fn account(user: store::User) -> Account {
    Account { id: user.id, username: user.username, created_at: user.created_at }
}

fn token(record: store::TokenRecord) -> Token {
    Token {
        id: record.id,
        name: record.name,
        prefix_hint: record.prefix_hint,
        created_at: record.created_at,
        last_used_at: record.last_used_at,
        revoked_at: record.revoked_at,
    }
}

/// Argon2id is ~50–100 ms of deliberate CPU, which would stall the async
/// runtime's worker; every password operation goes through here.
async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| AuthError::Db(format!("password worker: {e}")))
}

/// Register and sign in, in one step: a fresh account with no session would
/// only ever be followed by a login with the same credentials.
///
/// Returns the account and the raw session id for the caller to set as a cookie.
pub async fn register(db: &Db, username: &str, password: &str) -> Result<(Account, String)> {
    check_username(username)?;
    check_password(password)?;

    let owned = password.to_owned();
    let phc = blocking(move || hash_password(&owned))
        .await?
        .map_err(|e| AuthError::Db(format!("hashing failed: {e}")))?;

    let Some(user) = db.create_user(username, &phc, now()).await? else {
        return Err(AuthError::Taken);
    };
    let session = start_session(db, user.id).await?;
    Ok((account(user), session))
}

/// Verify credentials and open a session.
///
/// An unknown username still pays for a hash verification, so the response time
/// does not reveal which accounts exist.
pub async fn login(db: &Db, username: &str, password: &str) -> Result<(Account, String)> {
    let found = db.user_credentials(username).await?;
    let stored = found.as_ref().map(|(_, phc)| phc.clone());
    let owned = password.to_owned();
    // Both the dummy hash and the verification happen on the blocking thread —
    // the whole point is that the two paths cost the same.
    let ok = blocking(move || verify_password(&owned, &stored.unwrap_or_else(dummy_hash))).await?;

    match found {
        Some((user, _)) if ok => {
            let session = start_session(db, user.id).await?;
            Ok((account(user), session))
        }
        _ => Err(AuthError::InvalidCredentials),
    }
}

/// A real argon2id hash of a value nobody can present, verified against when
/// the username does not exist — the timing-equalising half of [`login`].
fn dummy_hash() -> String {
    // Hashed once per process, not per attempt: the cost that matters is the
    // *verification*, which happens either way.
    use std::sync::OnceLock;
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| hash_password(&store::accounts::generate_token()).unwrap_or_default())
        .clone()
}

async fn start_session(db: &Db, user_id: i64) -> Result<String> {
    let session_id = generate_session_id();
    db.create_session(&digest(&session_id), user_id, now()).await?;
    Ok(session_id)
}

/// End one session. Other sessions of the same account are untouched — logging
/// out of one browser is not logging out everywhere.
pub async fn logout(db: &Db, session_id: &str) -> Result<()> {
    db.delete_session(&digest(session_id)).await?;
    Ok(())
}

/// Who a session cookie belongs to, if it is still valid.
pub async fn session_account(db: &Db, session_id: &str) -> Result<Option<Account>> {
    Ok(db.session_user(&digest(session_id), now()).await?.map(account))
}

/// Mint a token. This is the only moment its plaintext exists outside the
/// client — nothing stores it, so nothing can show it again.
pub async fn create_token(db: &Db, user_id: i64, name: &str) -> Result<NewToken> {
    let name = name.trim();
    if name.is_empty() || name.len() > 60 {
        return Err(AuthError::Invalid("give the token a name (1–60 characters)".into()));
    }
    let secret = generate_token();
    let record = db.create_token(user_id, &secret, name, now()).await?;
    Ok(NewToken { token: secret, record: token(record) })
}

pub async fn list_tokens(db: &Db, user_id: i64) -> Result<Vec<Token>> {
    Ok(db.list_tokens(user_id).await?.into_iter().map(token).collect())
}

pub async fn revoke_token(db: &Db, user_id: i64, token_id: i64) -> Result<bool> {
    Ok(db.revoke_token(user_id, token_id, now()).await?)
}

/// Delete the account, its tokens and its sessions. There is no undo and no
/// grace period — matching the product's own "no recovery" posture.
pub async fn delete_account(db: &Db, user_id: i64) -> Result<()> {
    db.delete_user(user_id).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookies_carry_the_flags_and_parse_back() {
        let cookie = session_cookie("abc123");
        for flag in ["Path=/", "HttpOnly", "Secure", "SameSite=Lax"] {
            assert!(cookie.contains(flag), "{flag} missing from {cookie}");
        }
        assert!(cleared_cookie().contains("Max-Age=0"));

        assert_eq!(session_from_cookies("tdb_session=abc123"), Some("abc123".into()));
        assert_eq!(
            session_from_cookies("other=1; tdb_session=abc123; third=2"),
            Some("abc123".into())
        );
        assert_eq!(session_from_cookies("other=1"), None);
        assert_eq!(session_from_cookies(""), None);
    }

    #[test]
    fn credentials_are_validated_before_they_reach_the_database() {
        assert!(check_username("ada").is_ok());
        assert!(check_username("ad").is_err());
        assert!(check_username("Ada").is_err());
        assert!(check_username("ada lovelace").is_err());
        assert!(check_username(&"a".repeat(33)).is_err());
        assert!(check_password("0123456789").is_ok());
        assert!(check_password("012345678").is_err());
    }
}
