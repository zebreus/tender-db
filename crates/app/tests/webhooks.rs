//! Issue 08 — webhook delivery end to end.
//!
//! A real sweeper delivering signed batches to a real local HTTP receiver over a
//! real socket, against a fixture-ingested change log. The signing unit tests
//! prove the MAC; this proves the whole loop: the receiver gets a correctly
//! signed batch, a 2xx advances the slot, a failure holds it and backs off, a
//! recovered endpoint gets its backlog, and a stale failure disables the
//! endpoint.
#![cfg(feature = "server")]

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use hmac::{Hmac, Mac};
use ingest::{eforms, profile, project};
use serde_json::Value;
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Mutex;
use store::{Db, Notice};
use tender_db::webhooks::{self, Sweeper};

const SOURCE: &str = "ted";
const FIXTURES: &str = "../ingest/tests/fixtures";
const CHAIN: [&str; 4] = [
    "eforms-chain/1-cn-16-831374-2025.xml",
    "eforms-chain/2-change-16-6281-2026.xml",
    "eforms-chain/3-change-16-18902-2026.xml",
    "eforms-chain/4-can-29-380868-2026.xml",
];

// ------------------------------------------------------------- local receiver

/// One captured POST.
#[derive(Clone)]
struct Hit {
    headers: HashMap<String, String>,
    body: String,
}

/// A local webhook receiver whose response status is switchable, so a test can
/// make it fail and then recover.
#[derive(Clone)]
struct Receiver {
    hits: Arc<Mutex<Vec<Hit>>>,
    status: Arc<AtomicU16>,
}

impl Receiver {
    async fn start() -> (Receiver, String) {
        let recv = Receiver { hits: Arc::new(Mutex::new(Vec::new())), status: Arc::new(AtomicU16::new(200)) };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().unwrap().port();
        let state = recv.clone();
        tokio::spawn(async move {
            let app = axum::Router::new()
                .route("/hook", axum::routing::post(handle))
                .with_state(state);
            let _ = axum::serve(listener, app).await;
        });
        (recv, format!("http://127.0.0.1:{port}/hook"))
    }

    fn set_status(&self, status: u16) {
        self.status.store(status, Ordering::SeqCst);
    }

    fn hits(&self) -> Vec<Hit> {
        self.hits.lock().unwrap().clone()
    }
}

async fn handle(
    axum::extract::State(recv): axum::extract::State<Receiver>,
    headers: axum::http::HeaderMap,
    body: String,
) -> axum::http::StatusCode {
    let map = headers
        .iter()
        .map(|(k, v)| (k.as_str().to_owned(), v.to_str().unwrap_or("").to_owned()))
        .collect();
    recv.hits.lock().unwrap().push(Hit { headers: map, body });
    axum::http::StatusCode::from_u16(recv.status.load(Ordering::SeqCst)).unwrap()
}

// -------------------------------------------------------------------- harness

struct Fixture {
    db: Arc<Db>,
    user_id: i64,
    fetch_id: i64,
    path: String,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl Fixture {
    async fn start(name: &str) -> Fixture {
        let path = format!("/tmp/tender-db-whi-{name}-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Arc::new(Db::open(&path).await.expect("open"));
        db.record_fetch(&store::Fetch {
            source: SOURCE.into(),
            kind: "daily".into(),
            period: "2026-00136".into(),
            url: "https://example.invalid/pkg".into(),
            sha256: "aa".into(),
            bytes: 1,
            fetched_at: 0,
            path: "ted/daily/2026-00136.tar.gz".into(),
        })
        .await
        .expect("fetch");
        let fetch_id = db.current_packages(SOURCE, "daily", None).await.unwrap()[0].fetch_id;
        let user = tender_db::accounts::register(&db, "hooker", "a long password")
            .await
            .expect("register")
            .0;
        Fixture { db, user_id: user.id, fetch_id, path }
    }

    async fn ingest_chain(&self) {
        for relative in CHAIN {
            let bytes = std::fs::read(format!("{FIXTURES}/{relative}")).unwrap();
            let profile::Disposition::Records(records) = profile::dispatch(relative, &bytes) else {
                panic!("dispatch skipped {relative}");
            };
            let [profile::Record::Notice(n)] = &records[..] else { panic!("one notice") };
            let parse = eforms::parse_payload(&n.profile, &bytes);
            self.db
                .record_notice(
                    &Notice {
                        source: SOURCE.into(),
                        publication_id: n.publication_id.clone(),
                        content_hash: n.content_hash.clone(),
                        profile: n.profile.clone(),
                        declared_version: n.declared_version.clone(),
                        fetch_id: self.fetch_id,
                        member_path: n.member_path.clone(),
                        ingested_at: 0,
                        published_at: None,
                        dispatched_at: None,
                    },
                    &parse,
                )
                .await
                .unwrap();
        }
        project::project(&self.db, false).await.expect("project");
    }
}

/// Independently verify a Standard-Webhooks signature header, the way a consumer
/// would — not via the code that produced it.
fn signature_valid(secret: &str, hit: &Hit) -> bool {
    let id = &hit.headers["webhook-id"];
    let ts = &hit.headers["webhook-timestamp"];
    let key = B64.decode(secret.strip_prefix("whsec_").unwrap()).unwrap();
    let mut mac = <Hmac<Sha256>>::new_from_slice(&key).unwrap();
    mac.update(format!("{id}.{ts}.{}", hit.body).as_bytes());
    let expected = format!("v1,{}", B64.encode(mac.finalize().into_bytes()));
    hit.headers["webhook-signature"] == expected
}

// ---------------------------------------------------------------------- tests

#[tokio::test(flavor = "multi_thread")]
async fn a_batch_is_delivered_signed_and_the_slot_advances() {
    let fx = Fixture::start("deliver").await;
    fx.ingest_chain().await;
    let head = fx.db.latest_cursor().await.unwrap();
    assert!(head > 0, "the chain produced changes");

    let (recv, url) = Receiver::start().await;
    let secret = webhooks::generate_secret();
    // Register directly (the SSRF guard would reject a loopback URL), starting
    // at 0 so the whole backlog is due.
    let ep = fx.db.create_webhook(fx.user_id, &url, &secret, 0, 1, store::now_unix()).await.unwrap();

    let sweeper = Sweeper::new(fx.db.clone(), reqwest::Client::new());
    sweeper.sweep().await.expect("sweep");

    let hits = recv.hits();
    assert!(!hits.is_empty(), "the receiver got a batch");
    let hit = &hits[0];
    // Standard Webhooks headers present and independently verifiable.
    assert!(hit.headers.contains_key("webhook-id"));
    assert!(hit.headers.contains_key("webhook-timestamp"));
    assert!(signature_valid(&secret, hit), "the signature verifies against the secret");
    // The body carries the shared change-event JSON.
    let body: Value = serde_json::from_str(&hit.body).unwrap();
    assert!(body["events"].as_array().is_some_and(|e| !e.is_empty()));
    assert_eq!(body["cursor"], head.to_string());

    // The slot advanced to the head and the failure state is clean.
    let after = fx.db.webhook(fx.user_id, ep.id).await.unwrap().unwrap();
    assert_eq!(after.last_delivered_cursor, head);
    assert_eq!(after.consecutive_failures, 0);
    // A second sweep with nothing new delivers nothing.
    sweeper.sweep().await.unwrap();
    assert_eq!(recv.hits().len(), hits.len(), "no re-delivery of an already-acked batch");
}

/// Issue 214: the public-IP guard runs at DELIVERY, not only at registration. An
/// endpoint whose host now resolves to a non-public address — a DNS rebind to the
/// Hetzner cloud-metadata endpoint after a public-looking `register` — is refused
/// before any bytes are sent. The IP literal `169.254.169.254` stands in for "the
/// name now resolves here", so no DNS control is needed. The slot must hold and the
/// log must NAME the SSRF re-check, which is what distinguishes a guard refusal from
/// an ordinary connection failure (a plain unreachable host would also fail, but
/// with reqwest's text, and only after trying to connect).
#[tokio::test(flavor = "multi_thread")]
async fn a_rebound_endpoint_is_refused_at_delivery() {
    let fx = Fixture::start("rebind").await;
    fx.ingest_chain().await;
    let head = fx.db.latest_cursor().await.unwrap();
    assert!(head > 0, "the chain produced changes to deliver");

    let secret = webhooks::generate_secret();
    let ep = fx
        .db
        .create_webhook(fx.user_id, "https://169.254.169.254/hook", &secret, 0, 1, store::now_unix())
        .await
        .unwrap();

    // Enforcement ON, exactly as production's `init` builds the sweeper.
    let sweeper = Sweeper::new(fx.db.clone(), reqwest::Client::new()).recheck_ssrf_on_send();
    sweeper.sweep().await.expect("sweep");

    // Nothing was POSTed: the slot never advanced and a failure was recorded.
    let after = fx.db.webhook(fx.user_id, ep.id).await.unwrap().unwrap();
    assert_eq!(after.last_delivered_cursor, 0, "a refused delivery must not advance the slot");
    assert_eq!(after.consecutive_failures, 1, "the refusal is recorded as a failure and backed off");

    // The log names the SSRF re-check — proof the guard refused it rather than a
    // network error that would also have failed but carried reqwest's message.
    let log = fx.db.recent_webhook_deliveries(ep.id, 10).await.unwrap();
    assert!(
        log.iter()
            .any(|d| !d.ok && d.error.as_deref().unwrap_or_default().contains("SSRF re-check")),
        "a delivery-log row must name the SSRF re-check, got {log:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failure_holds_the_cursor_then_a_recovery_delivers_the_backlog() {
    let fx = Fixture::start("retry").await;
    fx.ingest_chain().await;
    let head = fx.db.latest_cursor().await.unwrap();

    let (recv, url) = Receiver::start().await;
    recv.set_status(500);
    let secret = webhooks::generate_secret();
    let ep = fx.db.create_webhook(fx.user_id, &url, &secret, 0, 1, store::now_unix()).await.unwrap();

    let sweeper = Sweeper::new(fx.db.clone(), reqwest::Client::new());
    sweeper.sweep().await.unwrap();

    // The 5xx was recorded as a failure: cursor untouched, streak started,
    // backoff scheduled into the future, a failed row logged.
    let failed = fx.db.webhook(fx.user_id, ep.id).await.unwrap().unwrap();
    assert_eq!(failed.last_delivered_cursor, 0, "a failure never advances the slot");
    assert_eq!(failed.consecutive_failures, 1);
    assert!(failed.failing_since.is_some());
    assert!(failed.next_attempt_at > store::now_unix(), "backed off into the future");
    let log = fx.db.recent_webhook_deliveries(ep.id, 10).await.unwrap();
    assert_eq!(log.len(), 1);
    assert!(!log[0].ok);
    assert_eq!(log[0].status, Some(500));

    // Because it is backed off, it is not due yet — a sweep does nothing.
    let hits_before = recv.hits().len();
    sweeper.sweep().await.unwrap();
    assert_eq!(recv.hits().len(), hits_before, "not retried until the backoff elapses");

    // The receiver recovers; the owner re-enables (which clears the backoff).
    recv.set_status(200);
    assert!(fx.db.enable_webhook(fx.user_id, ep.id, None).await.unwrap());
    sweeper.sweep().await.unwrap();

    // The recovered endpoint received the whole backlog and caught up — retrying
    // delivers what was missed, not a stale single payload.
    let healed = fx.db.webhook(fx.user_id, ep.id).await.unwrap().unwrap();
    assert_eq!(healed.last_delivered_cursor, head);
    assert_eq!(healed.consecutive_failures, 0);
    assert!(signature_valid(&secret, recv.hits().last().unwrap()));
}

/// Issue 178: a feed rebuild bumps the generation, so an endpoint whose slot
/// was stamped under the old one must receive a `feed_rebuilt` reset notice —
/// even when its cursor is now stranded beyond the new head and no ordinary
/// batch would ever arrive to carry the signal. After the reset its slot sits
/// at the new head and normal delivery resumes.
#[tokio::test(flavor = "multi_thread")]
async fn a_rebuild_sends_a_reset_notice_and_the_slot_resumes() {
    let fx = Fixture::start("rebuild-reset").await;
    fx.ingest_chain().await;

    let (recv, url) = Receiver::start().await;
    let secret = webhooks::generate_secret();
    // Registered at generation 1, slot at 0 so the backlog delivers first.
    let ep = fx.db.create_webhook(fx.user_id, &url, &secret, 0, 1, store::now_unix()).await.unwrap();
    let sweeper = Sweeper::new(fx.db.clone(), reqwest::Client::new());
    sweeper.sweep().await.unwrap();
    let delivered = fx.db.webhook(fx.user_id, ep.id).await.unwrap().unwrap();
    assert_eq!(delivered.last_generation, Some(1), "delivered under generation 1");
    let hits_before = recv.hits().len();

    // A full rebuild re-derives the canonical layer and bumps the generation —
    // every prior entity id and cursor is now incomposable (issue 46).
    project::project(&fx.db, true).await.expect("rebuild");
    let generation = fx.db.feed_generation().await.unwrap();
    assert!(generation > 1, "the rebuild bumped the generation: {generation}");

    // The next sweep sees the slot's stale generation and sends the reset.
    sweeper.sweep().await.unwrap();
    let hits = recv.hits();
    assert!(hits.len() > hits_before, "the endpoint received the reset notice");
    let reset: Value = serde_json::from_str(&hits.last().unwrap().body).unwrap();
    assert_eq!(reset["reset"], "feed_rebuilt", "the body marks the rebuilt feed");
    assert_eq!(reset["generation"], generation);
    assert_eq!(reset["events"].as_array().map(|e| e.len()), Some(0), "a reset carries no events");
    assert!(signature_valid(&secret, hits.last().unwrap()), "the reset notice is signed");

    // The slot is stamped to the current generation and head, so it is coherent
    // again and a following sweep does not re-send the reset.
    let after = fx.db.webhook(fx.user_id, ep.id).await.unwrap().unwrap();
    assert_eq!(after.last_generation, Some(generation), "slot now on the current generation");
    assert_eq!(after.last_delivered_cursor, fx.db.latest_cursor().await.unwrap(), "slot at head");
    let n = recv.hits().len();
    sweeper.sweep().await.unwrap();
    assert_eq!(recv.hits().len(), n, "no second reset once the generation matches");
}

#[tokio::test(flavor = "multi_thread")]
async fn sustained_failure_disables_the_endpoint() {
    let fx = Fixture::start("disable").await;
    fx.ingest_chain().await;

    let (recv, url) = Receiver::start().await;
    recv.set_status(503);
    let secret = webhooks::generate_secret();
    let ep = fx.db.create_webhook(fx.user_id, &url, &secret, 0, 1, store::now_unix()).await.unwrap();

    // Pre-age the failure streak to just over the auto-disable window, so the
    // next failed delivery crosses it — the real clock does the rest.
    let four_days_ago = store::now_unix() - 4 * 86_400;
    fx.db.webhook_failed(ep.id, four_days_ago, four_days_ago, false).await.unwrap();
    assert!(fx.db.webhook(fx.user_id, ep.id).await.unwrap().unwrap().disabled_at.is_none());

    let sweeper = Sweeper::new(fx.db.clone(), reqwest::Client::new());
    sweeper.sweep().await.unwrap();

    // The endpoint whose streak exceeded the window is now disabled and off the
    // sweeper's list.
    let after = fx.db.webhook(fx.user_id, ep.id).await.unwrap().unwrap();
    assert!(after.disabled_at.is_some(), "a >3-day failure streak disables the endpoint");
    assert!(fx.db.due_webhooks(store::now_unix()).await.unwrap().is_empty());
}
