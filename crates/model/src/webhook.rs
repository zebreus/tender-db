//! Webhooks across the client/server boundary (issue 08).
//!
//! Report types the dashboard renders; the secret crosses the wire exactly once,
//! at creation, in [`NewWebhook`].

use serde::{Deserialize, Serialize};

/// A registered endpoint as the dashboard and API list it. The signing secret
/// is never in here — only [`NewWebhook`] carries it, once.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Webhook {
    pub id: i64,
    pub url: String,
    pub created_at: i64,
    /// Set when the endpoint is disabled (by the owner or after sustained
    /// failure); `None` means active.
    pub disabled_at: Option<i64>,
    /// Newest change cursor this endpoint has been given a 2xx for.
    pub last_delivered_cursor: i64,
    /// First failure of the current streak, if it is currently failing.
    pub failing_since: Option<i64>,
    pub consecutive_failures: i64,
}

/// The one time an endpoint's signing secret is shown — copy it now, it is
/// stored server-side but never returned again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewWebhook {
    pub secret: String,
    pub webhook: Webhook,
}

/// One recent delivery attempt, for the dashboard's per-endpoint history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub attempted_at: i64,
    pub cursor_from: i64,
    pub cursor_to: i64,
    pub events: i64,
    pub status: Option<i64>,
    pub duration_ms: i64,
    pub ok: bool,
    pub error: Option<String>,
}
