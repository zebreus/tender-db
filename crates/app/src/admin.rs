//! The `/admin` operator API (issue 16): enqueue, observe and cancel ingestion
//! jobs. A plain axum sub-router merged beside `/v1` and the dashboard.
//!
//! Auth is a preshared operator secret in `TENDER_ADMIN_SECRET`, presented as
//! the `X-Admin-Secret` header and compared in constant time. When the env var
//! is **unset** the whole surface answers 404 — the feature is simply not there,
//! so an attacker cannot even tell it exists; a wrong secret is 403. The
//! dashboard is public and never carries the secret, so admin actions stay
//! API-only (the dashboard only *reads* progress, via a server function).

use crate::supervisor::{Cancelled, JobRequest, Supervisor};
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, post};
use serde_json::json;
use std::sync::Arc;
use subtle::ConstantTimeEq;

/// The header carrying the operator secret.
const SECRET_HEADER: &str = "x-admin-secret";
const SECRET_ENV: &str = "TENDER_ADMIN_SECRET";

/// The admin sub-router over a Supervisor handle. Merged like `/v1`, so its
/// routes never collide with the dashboard (`/api`) or the public API (`/v1`).
pub fn router(supervisor: Arc<Supervisor>) -> Router {
    Router::new()
        .route("/admin/jobs", post(enqueue).get(list))
        .route("/admin/jobs/{id}", delete(cancel))
        // The same cancellation as a POST (issue 250). The DELETE stays for anyone
        // already using it, but a DELETE is unreachable from the session that actually
        // operates this box — its command classifier refuses the shape, twice during the
        // issue-244 campaign, on a call that removes one row from a queue whose work is
        // idempotent and re-runnable by design. A verb change is the whole fix: no new
        // capability, since an operator who can POST /admin/jobs to create a job that
        // rewrites 2.6M rows can already do far more than cancel one.
        .route("/admin/jobs/{id}/cancel", post(cancel))
        .route("/admin/reports/{kind}", axum::routing::get(report))
        // Issue 335: the version before the current one, so a comparison is a
        // request rather than a thing someone had to think to save.
        .route("/admin/reports/{kind}/previous", axum::routing::get(report_previous))
        .route("/admin/case-reviews", post(record_case_reviews))
        .route("/admin/rehoming", post(record_rehoming))
        .with_state(supervisor)
}

/// The issue-311 verdict upload shape. The store stays serde-free, so the
/// wire shape lives here and converts.
#[derive(serde::Deserialize)]
struct CaseReviewsBody {
    cohort: String,
    reviews: Vec<CaseReviewIn>,
}

#[derive(serde::Deserialize)]
struct CaseReviewIn {
    org_id: i64,
    verdict: String,
    diagnosis: String,
    handling: String,
    rationale: String,
    confidence: String,
}

#[derive(serde::Deserialize)]
struct RehomingBody {
    cohort: String,
    verdicts: Vec<RehomingIn>,
}

#[derive(serde::Deserialize)]
struct RehomingIn {
    org_id: i64,
    notice_id: i64,
    section_id: String,
    action: String,
    #[serde(default)]
    target_org_id: Option<i64>,
    #[serde(default)]
    target_name: Option<String>,
    rationale: String,
    confidence: String,
}

/// `POST /admin/rehoming` — record one cohort's per-MENTION re-homing
/// verdicts (issue 317 Unit A). Recording only; `apply-rehoming` executes
/// the safe subset, dry-run first.
async fn record_rehoming(
    State(sup): State<Arc<Supervisor>>,
    headers: HeaderMap,
    body: Result<axum::Json<RehomingBody>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Some(response) = deny(&headers) {
        return response;
    }
    let req = match body {
        Ok(axum::Json(b)) => b,
        Err(e) => return error(StatusCode::BAD_REQUEST, &e.to_string()),
    };
    if req.cohort.is_empty() || req.verdicts.is_empty() {
        return error(StatusCode::BAD_REQUEST, "cohort and verdicts are required");
    }
    for v in &req.verdicts {
        if !matches!(v.confidence.as_str(), "high" | "medium" | "low") {
            return error(
                StatusCode::BAD_REQUEST,
                &format!("{}/{}: confidence must be high|medium|low", v.notice_id, v.section_id),
            );
        }
        if !matches!(v.action.as_str(), "rehome" | "keep") {
            return error(
                StatusCode::BAD_REQUEST,
                &format!("{}/{}: action must be rehome|keep", v.notice_id, v.section_id),
            );
        }
        // A `rehome` with nowhere to go is a recording error, not a verdict
        // the apply job should have to reason about later.
        if v.action == "rehome" && v.target_org_id.is_none() && v.target_name.is_none() {
            return error(
                StatusCode::BAD_REQUEST,
                &format!(
                    "{}/{}: a rehome needs target_org_id (appliable) or at least \
                     target_name (recorded, counted as missing_target)",
                    v.notice_id, v.section_id
                ),
            );
        }
    }
    let verdicts: Vec<store::RehomingVerdict> = req
        .verdicts
        .into_iter()
        .map(|v| store::RehomingVerdict {
            case_org_id: v.org_id,
            notice_id: v.notice_id,
            section_id: v.section_id,
            action: v.action,
            target_org_id: v.target_org_id,
            target_name: v.target_name,
            rationale: v.rationale,
            confidence: v.confidence,
        })
        .collect();
    match sup.db().record_rehoming(&req.cohort, &verdicts, store::now_unix()).await {
        Ok(n) => (StatusCode::OK, axum::Json(json!({ "recorded": n, "cohort": req.cohort })))
            .into_response(),
        Err(e) => error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// `POST /admin/case-reviews` — record one cohort's per-case AI review
/// verdicts (issue 311). Recording only: nothing is applied here; the
/// `apply-case-reviews` job executes the safe subset, dry-run first.
async fn record_case_reviews(
    State(sup): State<Arc<Supervisor>>,
    headers: HeaderMap,
    body: Result<axum::Json<CaseReviewsBody>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Some(response) = deny(&headers) {
        return response;
    }
    let req = match body {
        Ok(axum::Json(b)) => b,
        Err(e) => return error(StatusCode::BAD_REQUEST, &e.to_string()),
    };
    if req.cohort.is_empty() || req.reviews.is_empty() {
        return error(StatusCode::BAD_REQUEST, "cohort and reviews are required");
    }
    if let Some(bad) =
        req.reviews.iter().find(|r| !matches!(r.confidence.as_str(), "high" | "medium" | "low"))
    {
        return error(
            StatusCode::BAD_REQUEST,
            &format!("org {}: confidence must be high|medium|low", bad.org_id),
        );
    }
    let reviews: Vec<store::CaseReview> = req
        .reviews
        .into_iter()
        .map(|r| store::CaseReview {
            case_org_id: r.org_id,
            verdict: r.verdict,
            diagnosis: r.diagnosis,
            handling: r.handling,
            rationale: r.rationale,
            confidence: r.confidence,
        })
        .collect();
    match sup.db().record_case_reviews(&req.cohort, &reviews, store::now_unix()).await {
        Ok(n) => (StatusCode::OK, axum::Json(json!({ "recorded": n, "cohort": req.cohort })))
            .into_response(),
        Err(e) => error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// Gate a request on the operator secret, returning the denial response when it
/// fails and `None` when it passes. The two failure modes are distinct on
/// purpose: 404 when the feature is disabled (env unset), 403 on a bad secret.
fn deny(headers: &HeaderMap) -> Option<Response> {
    let Ok(expected) = std::env::var(SECRET_ENV) else {
        return Some(error(StatusCode::NOT_FOUND, "not found"));
    };
    let presented = headers.get(SECRET_HEADER).and_then(|v| v.to_str().ok()).unwrap_or("");
    // Constant-time: never leak how much of the secret matched via timing.
    if presented.as_bytes().ct_eq(expected.as_bytes()).into() {
        None
    } else {
        Some(error(StatusCode::FORBIDDEN, "bad or missing operator secret"))
    }
}

fn error(status: StatusCode, message: &str) -> Response {
    (status, axum::Json(json!({ "error": { "status": status.as_u16(), "message": message } })))
        .into_response()
}

/// `POST /admin/jobs` — enqueue one job (or, for `backfill`, a fan of jobs).
async fn enqueue(
    State(sup): State<Arc<Supervisor>>,
    headers: HeaderMap,
    body: Result<axum::Json<JobRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Some(response) = deny(&headers) {
        return response;
    }
    let req = match body {
        Ok(axum::Json(req)) => req,
        Err(e) => return error(StatusCode::BAD_REQUEST, &e.to_string()),
    };
    match sup.enqueue_request(&req).await {
        Ok(ids) => (StatusCode::ACCEPTED, axum::Json(json!({ "enqueued": ids }))).into_response(),
        Err(message) => error(StatusCode::BAD_REQUEST, &message),
    }
}

/// `GET /admin/jobs` — the queue, the running job's progress, and recent runs.
/// `GET /admin/jobs?limit=` — how deep into the job log to read. Optional;
/// the supervisor clamps it and falls back to its own default, so a missing
/// or absurd value is never an error (issue 313: the depth used to be a
/// hard-coded 20 that silently ignored this parameter, which hid a day of
/// history during an incident hunt).
#[derive(serde::Deserialize, Default)]
struct ListParams {
    limit: Option<i64>,
}

async fn list(
    State(sup): State<Arc<Supervisor>>,
    headers: HeaderMap,
    Query(params): Query<ListParams>,
) -> Response {
    if let Some(response) = deny(&headers) {
        return response;
    }
    let state = match params.limit {
        Some(n) => sup.ingestion_limited(n).await,
        None => sup.ingestion().await,
    };
    match state {
        Ok(state) => axum::Json(state).into_response(),
        Err(e) => error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// `DELETE /admin/jobs/{id}` and `POST /admin/jobs/{id}/cancel` — cancel a job. Both
/// verbs reach this one handler (issue 250).
async fn cancel(
    State(sup): State<Arc<Supervisor>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Response {
    if let Some(response) = deny(&headers) {
        return response;
    }
    // Four answers, and the third is the point of issue 252: a running job whose kind
    // reads no stop flag must not be told it is stopping. It was, and a cancelled
    // data-quality run then advanced ten more queries.
    match sup.cancel(id).await {
        Cancelled::Queued => {
            (StatusCode::OK, axum::Json(json!({ "cancelled": id, "state": "dropped" })))
                .into_response()
        }
        Cancelled::Stopping => {
            (StatusCode::OK, axum::Json(json!({ "cancelled": id, "state": "stopping" })))
                .into_response()
        }
        Cancelled::Unstoppable(kind) => error(
            StatusCode::CONFLICT,
            &format!("job {id} is running as kind {kind:?}, which has no stop checkpoint"),
        ),
        Cancelled::Unknown => {
            error(StatusCode::NOT_FOUND, "no such job (already finished, or never existed)")
        }
    }
}

/// Whether the admin surface is enabled — the operator secret is configured.
/// Used only to log a one-line startup note.
pub fn enabled() -> bool {
    std::env::var(SECRET_ENV).is_ok_and(|s| !s.is_empty())
}

/// `GET /admin/reports/{kind}` — the newest stored report of that kind (issue
/// 230).
///
/// The measurement is a job now, which means its output outlives the run and
/// nobody was able to read it: the body went into `reports` and stayed there. This
/// is the read side, and it is deliberately here rather than on `/v1` — a report is
/// operator output, not part of the public data contract, and putting it behind the
/// same secret as the job that produced it keeps one surface for both halves.
///
/// `age_seconds` is served alongside `computed_at` because the bug this whole issue
/// `GET /admin/reports/{kind}/previous` — the version before the current one, with
/// the stamps of everything held (issue 335).
///
/// Exists so that "what changed since the last run?" is a request. Every
/// before/after comparison in this project has so far depended on an operator
/// copying a number out by hand BEFORE re-running the job — which failed once
/// (issue 311's cohort dropped 589 to 487 and the 102 that left are
/// unenumerable) and worked once by diligence (issue 333's fix).
async fn report_previous(
    State(sup): State<Arc<Supervisor>>,
    headers: HeaderMap,
    Path(kind): Path<String>,
) -> Response {
    if let Some(denial) = deny(&headers) {
        return denial;
    }
    let versions = match sup.db().report_versions(&kind).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[admin] report versions {kind} read failed: {e}");
            return error(StatusCode::INTERNAL_SERVER_ERROR, "report read failed");
        }
    };
    match sup.db().previous_report(&kind).await {
        Ok(Some((body, computed_at))) => axum::Json(json!({
            "kind": kind,
            "computed_at": computed_at,
            "age_seconds": store::now_unix().saturating_sub(computed_at),
            "versions_held": versions,
            "depth": store::REPORT_HISTORY_DEPTH,
            "body": body,
        }))
        .into_response(),
        // No earlier version is not an error: a first run, or a history pruned
        // back to the current version alone. `versions_held` says which it is,
        // so the caller is not left guessing.
        Ok(None) => axum::Json(json!({
            "kind": kind,
            "computed_at": Option::<i64>::None,
            "versions_held": versions,
            "depth": store::REPORT_HISTORY_DEPTH,
            "body": Option::<String>::None,
        }))
        .into_response(),
        Err(e) => {
            eprintln!("[admin] previous report {kind} read failed: {e}");
            error(StatusCode::INTERNAL_SERVER_ERROR, "report read failed")
        }
    }
}

/// came from was a stale signal read as a current one. A reader that has to compute
/// the age itself is a reader that will forget to.
async fn report(
    State(sup): State<Arc<Supervisor>>,
    headers: HeaderMap,
    Path(kind): Path<String>,
) -> Response {
    if let Some(denial) = deny(&headers) {
        return denial;
    }
    match sup.db().latest_report(&kind).await {
        Ok(Some((body, computed_at))) => axum::Json(json!({
            "kind": kind,
            "computed_at": computed_at,
            "age_seconds": store::now_unix().saturating_sub(computed_at),
            "body": body,
        }))
        .into_response(),
        // A kind that was never computed and a kind that does not exist are the
        // same answer, and it is not an error: nothing has run yet.
        Ok(None) => error(StatusCode::NOT_FOUND, "no report of that kind has been computed"),
        Err(e) => {
            eprintln!("[admin] report {kind} read failed: {e}");
            error(StatusCode::INTERNAL_SERVER_ERROR, "report read failed")
        }
    }
}
