//! Webhook delivery (issue 08): sign, guard, and push change batches.
//!
//! Every registered endpoint is a consumer slot over the change log
//! (docs/architecture.md, docs/research/api-layer.md §4). A single background
//! **sweeper** — woken by the same cursor doorbell the SSE uses, plus a slow
//! timer so backed-off endpoints get retried — walks the due endpoints, POSTs
//! each its unsent batch, and advances its slot only on a 2xx. The change log is
//! the queue, so a recovered endpoint automatically receives everything it
//! missed; there is no outbox.
//!
//! Three concerns live here: **signing** (Standard Webhooks HMAC), the **SSRF
//! guard** (https + publicly-routable resolved address), and the **sweeper**.
//! Registration/management is thin CRUD over the store, shared by `/v1/webhooks`
//! and the dashboard.

use std::net::IpAddr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use hmac::{Hmac, Mac};
use model::{NewWebhook, Webhook, WebhookDelivery};
use sha2::Sha256;
use store::read;
use store::{Db, Delivery, Endpoint, Readers};
use url::Url;

/// Standard-Webhooks secret prefix.
pub const SECRET_PREFIX: &str = "whsec_";

/// Per-delivery HTTP timeout — GitHub's documented limit, the safe convention.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Change rows per POST. A slow endpoint just reads bigger ranges later; the log
/// is the buffer.
const BATCH: i64 = 500;

/// Most batches to push to one endpoint in a single sweep before moving on, so a
/// huge backlog on one endpoint cannot stall the others.
const MAX_BATCHES_PER_SWEEP: usize = 20;

/// How many recent attempts the per-endpoint delivery ring keeps.
const LOG_RING: i64 = 20;

/// Continuous-failure window after which an endpoint auto-disables (~3 days,
/// per Stripe's retry horizon).
const DISABLE_AFTER: i64 = 3 * 86_400;

/// Retry backoff by streak length: 30 s, 2 m, 10 m, 1 h, 4 h, 12 h, then daily.
fn backoff_seconds(consecutive_failures: i64) -> i64 {
    match consecutive_failures {
        ..=1 => 30,
        2 => 120,
        3 => 600,
        4 => 3_600,
        5 => 4 * 3_600,
        6 => 12 * 3_600,
        _ => 86_400,
    }
}

/// `TENDER_WEBHOOK_ALLOW_INSECURE=1` relaxes the SSRF guard for local
/// development — it permits `http` and loopback/private targets. Never set in
/// production.
pub fn allow_insecure() -> bool {
    std::env::var("TENDER_WEBHOOK_ALLOW_INSECURE").is_ok_and(|v| v == "1")
}

// ------------------------------------------------------------------ signing

/// A fresh signing secret: `whsec_` + base64 of 24 OS-random bytes (Standard
/// Webhooks' recommended 24–64). Drawn from the store's CSPRNG so there is one
/// randomness source in the project.
pub fn generate_secret() -> String {
    format!("{SECRET_PREFIX}{}", B64.encode(store::accounts::random_bytes(24)))
}

/// The Standard-Webhooks signature for one message: base64 HMAC-SHA256 over
/// `{id}.{timestamp}.{payload}`, prefixed `v1,`. The timestamp is inside the MAC,
/// which is what gives replay protection.
///
/// The secret's key bytes are the base64 body after `whsec_`; a secret that is
/// not in that shape is used as raw UTF-8 bytes (so a hand-set secret still
/// signs deterministically).
pub fn sign(secret: &str, msg_id: &str, timestamp: i64, payload: &str) -> String {
    let key = secret
        .strip_prefix(SECRET_PREFIX)
        .and_then(|body| B64.decode(body).ok())
        .unwrap_or_else(|| secret.as_bytes().to_vec());
    let mut mac = <Hmac<Sha256>>::new_from_slice(&key).expect("HMAC takes a key of any length");
    mac.update(format!("{msg_id}.{timestamp}.{payload}").as_bytes());
    format!("v1,{}", B64.encode(mac.finalize().into_bytes()))
}

// --------------------------------------------------------------- SSRF guard

/// Vet a user-supplied endpoint URL: parse it, require `https`, and require its
/// host to resolve to at least one address, all of them publicly routable. The
/// dev flag relaxes both the scheme and the address check.
///
/// Note: this resolves at check time; between the check and reqwest's own
/// resolution a DNS-rebinding attacker could point the name at a private
/// address (a TOCTOU window). Registration accepts that window — the delivery
/// path closes it by POSTing through a client PINNED to the addresses this
/// check vetted ([`vet_url_addrs`] + [`pinned_client`], issue 214's follow-up),
/// so reqwest never re-resolves the name at all.
pub async fn vet_url(raw: &str) -> Result<(), String> {
    vet_url_addrs(raw).await.map(|_| ())
}

/// [`vet_url`], returning what it vetted: the URL's host and the resolved,
/// publicly-routable addresses. The delivery path pins its connection to
/// exactly these, which is what makes the vet meaningful — a check whose
/// result is thrown away leaves reqwest to resolve the name AGAIN, and that
/// second resolution is the DNS-rebinding TOCTOU (issue 214).
pub async fn vet_url_addrs(raw: &str) -> Result<(String, Vec<std::net::SocketAddr>), String> {
    let url = Url::parse(raw).map_err(|e| format!("invalid URL: {e}"))?;
    let insecure_ok = allow_insecure();
    match url.scheme() {
        "https" => {}
        "http" if insecure_ok => {}
        other => return Err(format!("scheme {other:?} is not allowed; use https")),
    }
    let host = url.host_str().ok_or("the URL has no host")?;
    let port = url.port_or_known_default().unwrap_or(443);

    let addrs = resolve(host, port).await?;
    if addrs.is_empty() {
        return Err(format!("{host} does not resolve"));
    }
    if !insecure_ok {
        for ip in &addrs {
            if !is_public_ip(*ip) {
                return Err(format!("{host} resolves to the non-public address {ip}"));
            }
        }
    }
    Ok((host.to_owned(), addrs.into_iter().map(|ip| std::net::SocketAddr::new(ip, port)).collect()))
}

/// A one-delivery HTTP client whose connection is PINNED to `addrs` for
/// `host`: `resolve_to_addrs` replaces DNS for that name, so the socket goes
/// where the vet looked — TLS still validates against the HOSTNAME (SNI and
/// certificate checks are unchanged; only address resolution is overridden).
/// Per-delivery construction costs the connection pool, which is the right
/// trade: deliveries are sparse and a pooled connection to a formerly-vetted
/// address would itself be a stale pin.
fn pinned_client(host: &str, addrs: &[std::net::SocketAddr]) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .resolve_to_addrs(host, addrs)
        .build()
        .map_err(|e| format!("could not build the pinned delivery client: {e}"))
}

async fn resolve(host: &str, port: u16) -> Result<Vec<IpAddr>, String> {
    // lookup_host handles both DNS names and IP literals.
    match tokio::net::lookup_host((host, port)).await {
        Ok(iter) => Ok(iter.map(|s| s.ip()).collect()),
        Err(e) => Err(format!("could not resolve {host}: {e}")),
    }
}

/// Whether an address is safe to send a user-triggered request to: not
/// loopback, private, link-local, unspecified, CGNAT-shared, or otherwise
/// non-global. Conservative — anything not clearly public is rejected.
fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => {
            let [b0, b1, ..] = a.octets();
            !(a.is_private()
                || a.is_loopback()
                || a.is_link_local()
                || a.is_broadcast()
                || a.is_documentation()
                || a.is_unspecified()
                || b0 == 0
                // 100.64.0.0/10 CGNAT shared address space.
                || (b0 == 100 && (64..=127).contains(&b1))
                // 192.0.0.0/24 IETF protocol assignments.
                || (b0 == 192 && b1 == 0 && a.octets()[2] == 0)
                // 240.0.0.0/4 reserved.
                || b0 >= 240)
        }
        IpAddr::V6(a) => {
            if let Some(v4) = a.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(v4));
            }
            let seg0 = a.segments()[0];
            !(a.is_loopback()
                || a.is_unspecified()
                // fc00::/7 unique local.
                || (seg0 & 0xfe00) == 0xfc00
                // fe80::/10 link-local.
                || (seg0 & 0xffc0) == 0xfe80)
        }
    }
}

// --------------------------------------------------------------------- CRUD

/// Register an endpoint for a user: vet the URL, mint a secret, and start its
/// slot at the current log head so it receives future changes, not the backlog.
/// Returns the secret exactly once.
pub async fn register(db: &Db, user_id: i64, url: &str) -> Result<NewWebhook, String> {
    vet_url(url).await?;
    let secret = generate_secret();
    let head = db.latest_cursor().await.map_err(|e| e.to_string())?;
    let generation = db.feed_generation().await.map_err(|e| e.to_string())?;
    let endpoint = db
        .create_webhook(user_id, url, &secret, head, generation, store::now_unix())
        .await
        .map_err(|e| e.to_string())?;
    Ok(NewWebhook { secret, webhook: view(&endpoint) })
}

pub async fn list(db: &Db, user_id: i64) -> Result<Vec<Webhook>, String> {
    Ok(db
        .list_webhooks(user_id)
        .await
        .map_err(|e| e.to_string())?
        .iter()
        .map(view)
        .collect())
}

/// One endpoint plus its recent delivery attempts.
pub async fn detail(
    db: &Db,
    user_id: i64,
    id: i64,
) -> Result<Option<(Webhook, Vec<WebhookDelivery>)>, String> {
    let Some(endpoint) = db.webhook(user_id, id).await.map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let deliveries = db
        .recent_webhook_deliveries(id, LOG_RING)
        .await
        .map_err(|e| e.to_string())?
        .iter()
        .map(delivery_view)
        .collect();
    Ok(Some((view(&endpoint), deliveries)))
}

pub async fn delete(db: &Db, user_id: i64, id: i64) -> Result<bool, String> {
    db.delete_webhook(user_id, id).await.map_err(|e| e.to_string())
}

pub async fn disable(db: &Db, user_id: i64, id: i64) -> Result<bool, String> {
    db.disable_webhook(user_id, id, store::now_unix()).await.map_err(|e| e.to_string())
}

/// Re-enable an endpoint. `from_now` true resumes at the current log head
/// (skip the backlog accumulated while disabled); false keeps the stored slot.
pub async fn enable(db: &Db, user_id: i64, id: i64, from_now: bool) -> Result<bool, String> {
    let resume = if from_now {
        Some(db.latest_cursor().await.map_err(|e| e.to_string())?)
    } else {
        None
    };
    db.enable_webhook(user_id, id, resume).await.map_err(|e| e.to_string())
}

fn view(e: &Endpoint) -> Webhook {
    Webhook {
        id: e.id,
        url: e.url.clone(),
        created_at: e.created_at,
        disabled_at: e.disabled_at,
        last_delivered_cursor: e.last_delivered_cursor,
        failing_since: e.failing_since,
        consecutive_failures: e.consecutive_failures,
    }
}

fn delivery_view(d: &Delivery) -> WebhookDelivery {
    WebhookDelivery {
        attempted_at: d.attempted_at,
        cursor_from: d.cursor_from,
        cursor_to: d.cursor_to,
        events: d.events,
        status: d.status,
        duration_ms: d.duration_ms,
        ok: d.ok,
        error: d.error.clone(),
    }
}

// ------------------------------------------------------------------ sweeper

/// The delivery engine: one background task over the whole store.
pub struct Sweeper {
    db: Arc<Db>,
    readers: Arc<Readers>,
    http: reqwest::Client,
    /// Re-run the public-IP guard on every delivery, not just at registration
    /// (issue 214). On in production; off for the local delivery tests, whose
    /// receivers are on `127.0.0.1` and would otherwise be refused as non-public.
    revet_on_send: bool,
}

static SWEEPER: OnceLock<Arc<Sweeper>> = OnceLock::new();

/// Start the delivery sweeper and spawn its loop. Idempotent, like the
/// Supervisor — a dev hot-reload's second call returns the running instance.
pub fn init(db: Arc<Db>) -> Arc<Sweeper> {
    SWEEPER
        .get_or_init(|| {
            // A dedicated reader pool so reading the change log never queues
            // behind ingestion on the writer; redirects are failures (Standard
            // Webhooks), so the client must not follow them.
            let readers = db.readers(2).expect("webhook reader pool");
            let http = reqwest::Client::builder()
                .timeout(TIMEOUT)
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("a default reqwest client always builds");
            let sweeper = Arc::new(Sweeper { db, readers, http, revet_on_send: true });
            sweeper.clone().spawn();
            sweeper
        })
        .clone()
}

impl Sweeper {
    /// Construct without spawning — the integration test drives `sweep` directly.
    pub fn new(db: Arc<Db>, http: reqwest::Client) -> Sweeper {
        let readers = db.readers(2).expect("webhook reader pool");
        // Delivery re-vetting OFF by default: the delivery tests POST to a
        // `127.0.0.1` receiver, which the public-IP guard would refuse. Production
        // uses `init`, which turns it on; a test wanting the guard opts in with
        // [`Sweeper::recheck_ssrf_on_send`].
        Sweeper { db, readers, http, revet_on_send: false }
    }

    /// Turn on the delivery-time SSRF re-check (issue 214) for a test-built
    /// sweeper — production gets it from [`init`].
    #[doc(hidden)]
    pub fn recheck_ssrf_on_send(mut self) -> Self {
        self.revet_on_send = true;
        self
    }

    fn spawn(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut doorbell = self.db.cursor_watch();
            let mut tick = tokio::time::interval(Duration::from_secs(15));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                // A new change rings the doorbell; the timer catches endpoints
                // whose backoff has elapsed without any new change.
                tokio::select! {
                    changed = doorbell.changed() => {
                        if changed.is_err() {
                            return; // database gone → shutting down
                        }
                    }
                    _ = tick.tick() => {}
                }
                if let Err(e) = self.sweep().await {
                    eprintln!("webhooks: sweep error: {e}");
                }
            }
        });
    }

    /// One pass over every due endpoint.
    pub async fn sweep(&self) -> Result<(), String> {
        let due = self.db.due_webhooks(store::now_unix()).await.map_err(|e| e.to_string())?;
        let generation = {
            let reader = self.readers.get().await.map_err(|e| e.to_string())?;
            read::feed_generation(&reader).await.map_err(|e| e.to_string())?
        };
        for endpoint in due {
            self.deliver(endpoint, generation).await;
        }
        Ok(())
    }

    /// Push an endpoint's backlog in batches until it is drained, a batch fails,
    /// or the per-sweep cap is hit.
    async fn deliver(&self, mut endpoint: Endpoint, generation: i64) {
        // Generation gate (issue 178): if the slot's stamped generation is not the
        // current one — a rebuild re-issued the feed, or this is the first sweep of
        // a pre-upgrade endpoint (NULL) — the endpoint's mirrored state is from a
        // world that no longer exists. Its cursor may even sit BEYOND the new head
        // (clear_changes restarted the log), so no ordinary batch would ever arrive
        // to carry the signal. Send a reset notice and jump the slot to head; normal
        // delivery resumes next sweep. This is the webhook transport's counterpart
        // to the SSE `reset`/poll `generation` signal issue 46 gave the other two.
        if endpoint.last_generation != Some(generation) {
            self.deliver_reset(&mut endpoint, generation).await;
            return;
        }
        for _ in 0..MAX_BATCHES_PER_SWEEP {
            let from = endpoint.last_delivered_cursor;
            let changes = match self.read_batch(from).await {
                Ok(changes) => changes,
                Err(e) => {
                    eprintln!("webhooks: read log for endpoint {}: {e}", endpoint.id);
                    return;
                }
            };
            if changes.is_empty() {
                return; // caught up
            }
            let to = changes.last().map(|c| c.cursor).unwrap_or(from);
            let outcome = self.post(&endpoint, from, to, generation, &changes).await;
            self.record(&endpoint, from, to, changes.len() as i64, generation, &outcome).await;
            match outcome {
                Outcome::Ok => {
                    endpoint.last_delivered_cursor = to;
                    if (changes.len() as i64) < BATCH {
                        return; // last batch was partial → drained
                    }
                }
                // A failure stops this endpoint; its backoff was just set.
                Outcome::Failed { .. } => return,
            }
        }
    }

    /// Deliver the feed-rebuilt reset notice (issue 178): a batch-shaped body
    /// carrying `{"reset":"feed_rebuilt", "generation":N, "events":[]}` so the
    /// consumer hears the rebuild even when no events are flowing, then jump the
    /// slot to the current head under the new generation. On failure the endpoint
    /// backs off and retries the reset — the slot is never advanced past a
    /// rebuild the consumer has not acknowledged.
    async fn deliver_reset(&self, endpoint: &Endpoint, generation: i64) {
        let head = match self.db.latest_cursor().await {
            Ok(head) => head,
            Err(e) => {
                eprintln!("webhooks: read head for reset of endpoint {}: {e}", endpoint.id);
                return;
            }
        };
        let outcome = self.post_reset(endpoint, head, generation).await;
        // Reuse the normal accounting: on 2xx `record` advances the slot to head
        // AND stamps the new generation (`webhook_delivered`), which IS the reset;
        // on failure it backs off, leaving the stranded slot to retry the reset.
        self.record(endpoint, endpoint.last_delivered_cursor, head, 0, generation, &outcome).await;
    }

    async fn read_batch(&self, from: i64) -> Result<Vec<store::Change>, String> {
        let reader = self.readers.get().await.map_err(|e| e.to_string())?;
        read::changes_since(&reader, from, BATCH, None).await.map_err(|e| e.to_string())
    }

    /// Build, sign and POST one batch; classify the result.
    async fn post(
        &self,
        endpoint: &Endpoint,
        from: i64,
        to: i64,
        generation: i64,
        changes: &[store::Change],
    ) -> Outcome {
        // Deliver only the public-feed kinds (issue 211): the webhook transport is
        // one of the three feeds that must carry the same documented, resolvable
        // events SSE does — lot_result/bid/contract change rows are filtered out.
        // The cursor still advances over the full window (`to`/`changes.len()` in
        // `deliver`), so a batch of only result-graph rows acks and moves on.
        let events: Vec<serde_json::Value> = changes
            .iter()
            .filter(|c| crate::v1::sse::is_public_change_kind(&c.entity_kind))
            .map(crate::v1::sse::change_event)
            .collect();
        let body = serde_json::json!({
            "cursor_from": from.to_string(),
            "cursor": to.to_string(),
            // The feed generation (issue 46): when this moves between batches,
            // the consumer's mirrored state is from a world that was rebuilt —
            // drop it and re-fetch the collections. Same contract as the poll
            // envelope's `generation`.
            "generation": generation,
            "events": events,
        })
        .to_string();
        self.sign_and_send(endpoint, from, to, body).await
    }

    /// Build, sign and POST the feed-rebuilt reset notice (issue 178) — the same
    /// batch envelope and signing as a normal delivery, with an empty `events`
    /// and a `reset` marker, so a consumer verifies and routes it identically.
    async fn post_reset(&self, endpoint: &Endpoint, head: i64, generation: i64) -> Outcome {
        let body = serde_json::json!({
            "cursor_from": endpoint.last_delivered_cursor.to_string(),
            "cursor": head.to_string(),
            "generation": generation,
            // The consumer's mirrored state predates a rebuild and does not
            // compose — drop it and re-snapshot the collections via REST, then
            // resume from this cursor. Same reasons as SSE's `reset` event.
            "reset": "feed_rebuilt",
            "events": [],
        })
        .to_string();
        self.sign_and_send(endpoint, endpoint.last_delivered_cursor, head, body).await
    }

    /// Sign a prepared body with the Standard-Webhooks HMAC and POST it, classifying
    /// the HTTP result — the tail shared by a normal batch and a reset notice.
    async fn sign_and_send(&self, endpoint: &Endpoint, from: i64, to: i64, body: String) -> Outcome {
        // Re-vet at delivery, not just at registration (issue 214). `register`'s
        // `vet_url` ran once; every later POST re-resolves the stored host through
        // reqwest with no IP pin, so a DNS-rebinding attacker who showed a public
        // address at registration can repoint the name at a link-local / cloud-
        // metadata address (169.254.169.254 on this Hetzner box) afterwards. Re-
        // running the public-IP guard on each send refuses a host that now resolves
        // non-public — the persistent rebind — before any bytes leave the box.
        // (The residual sub-millisecond TOCTOU between this resolve and reqwest's own
        // is closed only by pinning the connection to the vetted IP; that needs a
        // live delivery to verify and is tracked as the issue-214 follow-up.)
        // …and the POST is PINNED to what the re-check vetted (the issue-214
        // follow-up): `resolve_to_addrs` hands reqwest the vetted sockets, so
        // the sub-millisecond TOCTOU between our resolve and reqwest's own is
        // gone — there is no second resolution. TLS still validates the
        // hostname. Tests (revet off) keep the shared client and today's path.
        let http = if self.revet_on_send {
            match vet_url_addrs(&endpoint.url).await {
                Ok((host, addrs)) => match pinned_client(&host, &addrs) {
                    Ok(client) => client,
                    Err(error) => {
                        return Outcome::Failed { status: None, error: Some(error), duration_ms: 0 };
                    }
                },
                Err(reason) => {
                    return Outcome::Failed {
                        status: None,
                        error: Some(format!(
                            "delivery refused: endpoint host failed the SSRF re-check ({reason})"
                        )),
                        duration_ms: 0,
                    };
                }
            }
        } else {
            self.http.clone()
        };

        let msg_id = format!("evt_{}_{}_{}", endpoint.id, from, to);
        let timestamp = store::now_unix();
        let signature = sign(&endpoint.secret, &msg_id, timestamp, &body);

        let started = std::time::Instant::now();
        let result = http
            .post(&endpoint.url)
            .header("content-type", "application/json")
            .header("webhook-id", &msg_id)
            .header("webhook-timestamp", timestamp.to_string())
            .header("webhook-signature", signature)
            .body(body)
            .send()
            .await;
        let duration_ms = started.elapsed().as_millis() as i64;

        match result {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    Outcome::Ok
                } else {
                    Outcome::Failed { status: Some(status.as_u16() as i64), error: None, duration_ms }
                }
            }
            Err(e) => Outcome::Failed { status: None, error: Some(e.to_string()), duration_ms },
        }
    }

    /// Persist the outcome: advance and clear on success (stamping the delivered
    /// generation, issue 178), or set the backoff and maybe disable on failure,
    /// and append the log row either way.
    async fn record(
        &self,
        endpoint: &Endpoint,
        from: i64,
        to: i64,
        events: i64,
        generation: i64,
        outcome: &Outcome,
    ) {
        let now = store::now_unix();
        let (ok, status, error, duration_ms) = match outcome {
            Outcome::Ok => (true, None, None, 0),
            Outcome::Failed { status, error, duration_ms } => {
                (false, *status, error.clone(), *duration_ms)
            }
        };

        let write = async {
            match outcome {
                Outcome::Ok => self.db.webhook_delivered(endpoint.id, to, generation).await,
                Outcome::Failed { .. } => {
                    let next_failures = endpoint.consecutive_failures + 1;
                    let next_attempt_at = now + backoff_seconds(next_failures);
                    // The streak began at failing_since, or now for the first failure.
                    let streak_start = endpoint.failing_since.unwrap_or(now);
                    let disable = now - streak_start >= DISABLE_AFTER;
                    self.db.webhook_failed(endpoint.id, now, next_attempt_at, disable).await
                }
            }
        };
        if let Err(e) = write.await {
            eprintln!("webhooks: persist outcome for endpoint {}: {e}", endpoint.id);
        }

        let record = Delivery {
            attempted_at: now,
            cursor_from: from,
            cursor_to: to,
            events,
            status,
            duration_ms,
            ok,
            error,
        };
        if let Err(e) = self.db.log_webhook_delivery(endpoint.id, &record, LOG_RING).await {
            eprintln!("webhooks: log delivery for endpoint {}: {e}", endpoint.id);
        }
    }
}

enum Outcome {
    Ok,
    Failed { status: Option<i64>, error: Option<String>, duration_ms: i64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue 214's follow-up, proven by construction: the POST connects to
    /// where the PIN points, not where DNS does. `pinned.invalid` can never
    /// resolve (RFC 2606), so the request reaching the local receiver at all
    /// is possible only through `resolve_to_addrs` — the same mechanism the
    /// delivery path hands its vetted sockets. If reqwest re-resolved, this
    /// request could not even start, let alone land.
    #[tokio::test]
    async fn the_pin_routes_the_connection_where_the_vet_looked() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = axum::Router::new()
            .route("/hook", axum::routing::post(|| async { "ok" }));
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let client = pinned_client("pinned.invalid", &[addr]).expect("pinned client builds");
        let response = client
            .post(format!("http://pinned.invalid:{}/hook", addr.port()))
            .body("x")
            .send()
            .await
            .expect("the pin must route the connection to the vetted socket");
        assert_eq!(response.status().as_u16(), 200);

        // And WITHOUT the pin the same URL is unreachable — the control that
        // proves the success above came from the pin, not from ambient DNS.
        let unpinned = reqwest::Client::builder().timeout(TIMEOUT).build().unwrap();
        assert!(
            unpinned
                .post(format!("http://pinned.invalid:{}/hook", addr.port()))
                .body("x")
                .send()
                .await
                .is_err(),
            ".invalid must not resolve without the pin"
        );
    }

    #[test]
    fn signatures_are_standard_webhooks_shaped_and_stable() {
        // The Standard Webhooks example inputs (secret is base64 after the
        // `whsec_` hint). The expected signature is cross-checked against an
        // independent HMAC-SHA256 implementation over the exact
        // `{id}.{timestamp}.{payload}` bytes — the payload's space after the
        // colon is part of the signed content.
        let secret = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";
        let sig = sign(secret, "msg_p5jXN8AQM9LWM0D4loKWxJ", 1614265330, r#"{"test": 2432232314}"#);
        assert_eq!(sig, "v1,nZDXxaBq9gjtUTw4rmGoTP5WJxSmVfaXcrB9OIf7URE=");
    }

    #[test]
    fn a_generated_secret_round_trips_through_signing() {
        let secret = generate_secret();
        assert!(secret.starts_with("whsec_"));
        let a = sign(&secret, "evt_1_0_5", 100, "{}");
        let b = sign(&secret, "evt_1_0_5", 100, "{}");
        assert_eq!(a, b, "same inputs → same signature");
        assert_ne!(a, sign(&secret, "evt_1_0_6", 100, "{}"), "the id is signed");
        assert_ne!(a, sign(&secret, "evt_1_0_5", 101, "{}"), "the timestamp is signed");
    }

    #[test]
    fn private_and_loopback_addresses_are_rejected() {
        use std::net::{Ipv4Addr, Ipv6Addr};
        for ip in [
            "127.0.0.1", "10.0.0.1", "192.168.1.1", "172.16.0.1", "169.254.1.1",
            "0.0.0.0", "100.64.0.1", "192.0.0.1", "240.0.0.1", "255.255.255.255",
        ] {
            assert!(!is_public_ip(ip.parse::<Ipv4Addr>().unwrap().into()), "{ip} is not public");
        }
        for ip in ["8.8.8.8", "1.1.1.1", "93.184.216.34"] {
            assert!(is_public_ip(ip.parse::<Ipv4Addr>().unwrap().into()), "{ip} is public");
        }
        assert!(!is_public_ip(Ipv6Addr::LOCALHOST.into()));
        assert!(!is_public_ip("fc00::1".parse::<Ipv6Addr>().unwrap().into()), "ULA");
        assert!(!is_public_ip("fe80::1".parse::<Ipv6Addr>().unwrap().into()), "link-local");
        assert!(!is_public_ip("::ffff:127.0.0.1".parse::<Ipv6Addr>().unwrap().into()), "mapped loopback");
        assert!(is_public_ip("2606:4700:4700::1111".parse::<Ipv6Addr>().unwrap().into()));
    }

    #[tokio::test]
    async fn vet_url_enforces_scheme_and_public_host() {
        // These hold regardless of the dev flag's default-off state in CI.
        assert!(vet_url("not a url").await.is_err());
        assert!(vet_url("ftp://example.com").await.is_err(), "only https");
        // Loopback by literal — rejected without any DNS.
        assert!(vet_url("https://127.0.0.1/hook").await.is_err());
        assert!(vet_url("https://[::1]/hook").await.is_err());
    }

    #[test]
    fn backoff_grows_then_caps_at_a_day() {
        assert_eq!(backoff_seconds(1), 30);
        assert_eq!(backoff_seconds(4), 3_600);
        assert_eq!(backoff_seconds(6), 12 * 3_600);
        assert_eq!(backoff_seconds(7), 86_400);
        assert_eq!(backoff_seconds(50), 86_400);
    }
}
