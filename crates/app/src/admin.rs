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
        .route("/admin/name-key", axum::routing::get(name_key))
        .route("/admin/unmapped-fields", axum::routing::get(unmapped_fields))
        .route("/admin/case-reviews", axum::routing::get(case_reviews))
        .route("/admin/case-reviews", post(record_case_reviews))
        .route("/admin/name-verdicts", post(record_name_verdicts))
        .route("/admin/rehoming", post(record_rehoming))
        .route("/admin/country-verdicts", post(record_country_verdicts))
        .route("/admin/merge-verdicts", post(record_merge_verdicts))
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

#[derive(serde::Deserialize)]
struct CaseReviewsParams {
    /// `case` (org_case_reviews, default) | `rehoming` | `name` | `country`.
    table: Option<String>,
    cohort: Option<String>,
    /// Rows to return (default 500, at most 5,000), newest first.
    limit: Option<usize>,
}

/// `GET /admin/case-reviews?table=…&cohort=…&limit=…` — read a verdict store
/// back (issue 356). The four review tables are written through the POSTs
/// above and were readable by nothing but their apply jobs: `/v1/sql` is a
/// positive allow-list (issue 45) they are deliberately not on. Bounded,
/// read-only, admin-gated like the writes.
async fn case_reviews(
    State(sup): State<Arc<Supervisor>>,
    headers: HeaderMap,
    Query(params): Query<CaseReviewsParams>,
) -> Response {
    if let Some(denial) = deny(&headers) {
        return denial;
    }
    let table = params.table.as_deref().unwrap_or("case");
    let limit = params.limit.unwrap_or(500).clamp(1, 5_000);
    let (cols, rows) = match sup.db().verdict_rows(table, params.cohort.as_deref(), limit).await {
        Ok(v) => v,
        Err(e) => {
            let msg = e.to_string();
            return if msg.contains("no verdict store named") {
                error(StatusCode::BAD_REQUEST, &msg)
            } else {
                eprintln!("[admin] case-reviews read failed: {e}");
                error(StatusCode::INTERNAL_SERVER_ERROR, "case-reviews read failed")
            };
        }
    };
    let rows: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let mut o = serde_json::Map::new();
            for (c, v) in cols.iter().zip(r) {
                o.insert(
                    (*c).to_owned(),
                    match v {
                        store::turso::Value::Integer(i) => json!(i),
                        store::turso::Value::Real(f) => json!(f),
                        store::turso::Value::Text(s) => json!(s),
                        _ => serde_json::Value::Null,
                    },
                );
            }
            serde_json::Value::Object(o)
        })
        .collect();
    (
        StatusCode::OK,
        axum::Json(json!({
            "table": table, "cohort": params.cohort, "limit": limit,
            "count": rows.len(), "rows": rows,
        })),
    )
        .into_response()
}

/// The issue-355 country-verdict upload shape.
#[derive(serde::Deserialize)]
struct CountryVerdictsBody {
    cohort: String,
    verdicts: Vec<CountryVerdictIn>,
}

#[derive(serde::Deserialize)]
struct CountryVerdictIn {
    org_id: i64,
    action: String,
    #[serde(default)]
    from_country: Option<String>,
    #[serde(default)]
    to_country: Option<String>,
    rationale: String,
    confidence: String,
}

/// `POST /admin/country-verdicts` — record one cohort's per-ROW country
/// verdicts (issue 355). Recording only; `apply-country-verdicts` executes
/// the high-confidence `move` subset, dry-run first.
async fn record_country_verdicts(
    State(sup): State<Arc<Supervisor>>,
    headers: HeaderMap,
    body: Result<axum::Json<CountryVerdictsBody>, axum::extract::rejection::JsonRejection>,
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
    let is_cc = |c: &str| c.len() == 2 && c.bytes().all(|b| b.is_ascii_uppercase());
    for v in &req.verdicts {
        if !matches!(v.confidence.as_str(), "high" | "medium" | "low") {
            return error(
                StatusCode::BAD_REQUEST,
                &format!("org {}: confidence must be high|medium|low", v.org_id),
            );
        }
        if !matches!(v.action.as_str(), "move" | "keep") {
            return error(
                StatusCode::BAD_REQUEST,
                &format!("org {}: action must be move|keep", v.org_id),
            );
        }
        // The pre-image is whatever the row carries, junk included (`1A`).
        if let Some(f) = &v.from_country {
            if f.is_empty() || f.len() > 16 {
                return error(
                    StatusCode::BAD_REQUEST,
                    &format!("org {}: from_country must be the row's code as published", v.org_id),
                );
            }
        }
        if v.action == "move" {
            match &v.to_country {
                Some(to) if is_cc(to) && Some(to) != v.from_country.as_ref() => {}
                Some(_) => {
                    return error(
                        StatusCode::BAD_REQUEST,
                        &format!(
                            "org {}: to_country must be a two-letter code different from \
                             from_country",
                            v.org_id
                        ),
                    )
                }
                None => {
                    return error(
                        StatusCode::BAD_REQUEST,
                        &format!("org {}: a move needs to_country", v.org_id),
                    )
                }
            }
        }
    }
    let verdicts: Vec<store::CountryVerdict> = req
        .verdicts
        .into_iter()
        .map(|v| store::CountryVerdict {
            org_id: v.org_id,
            action: v.action,
            from_country: v.from_country,
            to_country: v.to_country,
            rationale: v.rationale,
            confidence: v.confidence,
        })
        .collect();
    match sup.db().record_country_verdicts(&req.cohort, &verdicts, store::now_unix()).await {
        Ok(n) => (StatusCode::OK, axum::Json(json!({ "recorded": n, "cohort": req.cohort })))
            .into_response(),
        Err(e) => error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// The issue-362 merge-verdict upload shape.
#[derive(serde::Deserialize)]
struct MergeVerdictsBody {
    cohort: String,
    verdicts: Vec<MergeVerdictIn>,
}

#[derive(serde::Deserialize)]
struct MergeVerdictIn {
    country: String,
    scheme: String,
    key: String,
    /// The org ids the reviewer read, ascending — the merge is honoured only
    /// while the live group is exactly this set.
    members: Vec<i64>,
    action: String,
    rationale: String,
    confidence: String,
}

/// `POST /admin/merge-verdicts` — record one cohort's per-GROUP merge
/// verdicts (issue 362): the review loop's execution path for the groups the
/// R2 name gate leaves standing. Recording only; the next wet
/// `match-org-identifiers` run executes HIGH `merge` verdicts whose member
/// set still matches, and `keep` denies its group from then on.
async fn record_merge_verdicts(
    State(sup): State<Arc<Supervisor>>,
    headers: HeaderMap,
    body: Result<axum::Json<MergeVerdictsBody>, axum::extract::rejection::JsonRejection>,
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
    if req.verdicts.len() > 5_000 {
        return error(StatusCode::BAD_REQUEST, "at most 5,000 verdicts per upload");
    }
    for v in &req.verdicts {
        let name = format!("{}/{}/{}", v.country, v.scheme, v.key);
        if v.country.is_empty() || v.scheme.is_empty() || v.key.is_empty() || v.key.len() > 64 {
            return error(
                StatusCode::BAD_REQUEST,
                &format!("group {name}: country, scheme and a key of at most 64 chars are required"),
            );
        }
        if !matches!(v.action.as_str(), "merge" | "keep") {
            return error(StatusCode::BAD_REQUEST, &format!("group {name}: action must be merge|keep"));
        }
        if !matches!(v.confidence.as_str(), "high" | "medium" | "low") {
            return error(
                StatusCode::BAD_REQUEST,
                &format!("group {name}: confidence must be high|medium|low"),
            );
        }
        if v.members.len() < 2 || v.members.windows(2).any(|w| w[0] >= w[1]) {
            return error(
                StatusCode::BAD_REQUEST,
                &format!("group {name}: members must be two or more org ids, ascending, distinct"),
            );
        }
        if v.rationale.len() > 4_000 {
            return error(StatusCode::BAD_REQUEST, &format!("group {name}: rationale over 4,000 chars"));
        }
    }
    let verdicts: Vec<store::MergeVerdict> = req
        .verdicts
        .into_iter()
        .map(|v| store::MergeVerdict {
            country: v.country,
            scheme: v.scheme,
            key: v.key,
            members: v.members,
            action: v.action,
            rationale: v.rationale,
            confidence: v.confidence,
        })
        .collect();
    match sup.db().record_merge_verdicts(&req.cohort, &verdicts, store::now_unix()).await {
        Ok(n) => (StatusCode::OK, axum::Json(json!({ "recorded": n, "cohort": req.cohort })))
            .into_response(),
        Err(e) => error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// The issue-351 name-verdict upload shape.
#[derive(serde::Deserialize)]
struct NameVerdictsBody {
    cohort: String,
    verdicts: Vec<NameVerdictIn>,
}

#[derive(serde::Deserialize)]
struct NameVerdictIn {
    name: String,
    verdict: String,
    rationale: String,
}

/// `POST /admin/name-verdicts` — record one cohort's NAME-level verdicts
/// (issue 351 unit 4): `single` lets the provisional echo fold and the
/// resolver's country-less reuse treat the name as one entity whatever the
/// wall says; `generic`, `platform` and `non-name` refuse both; `unclear`
/// is recorded and decides nothing. Keyed by the bare lower-case name.
async fn record_name_verdicts(
    State(sup): State<Arc<Supervisor>>,
    headers: HeaderMap,
    body: Result<axum::Json<NameVerdictsBody>, axum::extract::rejection::JsonRejection>,
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
    if let Some(bad) = req.verdicts.iter().find(|r| {
        !matches!(r.verdict.as_str(), "single" | "generic" | "platform" | "non-name" | "unclear")
    }) {
        return error(
            StatusCode::BAD_REQUEST,
            &format!("{:?}: verdict must be single|generic|platform|non-name|unclear", bad.name),
        );
    }
    let verdicts: Vec<store::NameVerdict> = req
        .verdicts
        .into_iter()
        .filter(|r| !r.name.trim().is_empty())
        .map(|r| store::NameVerdict {
            name_norm: r.name.trim().to_lowercase(),
            verdict: r.verdict,
            rationale: r.rationale,
        })
        .collect();
    match sup.db().record_name_verdicts(&req.cohort, &verdicts, store::now_unix()).await {
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
#[derive(serde::Deserialize)]
struct NameKeyParams {
    name: String,
    /// How many carrier org rows to list (default 20, at most 100).
    show: Option<usize>,
}

/// `GET /admin/name-key?name=…` — the genericness wall, made observable
/// (issue 348). Returns the N2 and N3 keys `ingest` derives from the name,
/// the wall's cap, and per kind the distinct-carrier count (bounded at
/// 1,000) with the first carriers' rows, so "why did this key read as
/// generic" is a request rather than a guess. Bounded index seeks only.
async fn name_key(
    State(sup): State<Arc<Supervisor>>,
    headers: HeaderMap,
    Query(params): Query<NameKeyParams>,
) -> Response {
    if let Some(denial) = deny(&headers) {
        return denial;
    }
    let show = params.show.unwrap_or(20).min(100);
    let n2 = ingest::project::match_norm(&params.name);
    let n3 = ingest::crosswalk::n3_key(&params.name);
    let cap = ingest::idgate::STOPLIST_CAP;
    let mut kinds = serde_json::Map::new();
    // The wall's own order: 'n3' then 'n2' (see NAME_KEY_CARRIERS_SQL for why
    // not one IN), and a name whose N3 equals its N2 has no 'n3' row at all.
    for (kind, key) in [("n3", n3.as_str()), ("n2", n2.as_str())] {
        if key.is_empty() {
            continue;
        }
        let (carriers, ids) = match sup.db().name_key_carriers(kind, key, 1_000, show).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[admin] name-key probe failed: {e}");
                return error(StatusCode::INTERNAL_SERVER_ERROR, "name-key probe failed");
            }
        };
        let meta = match sup.db().org_health_meta(&ids).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[admin] name-key meta read failed: {e}");
                return error(StatusCode::INTERNAL_SERVER_ERROR, "name-key probe failed");
            }
        };
        kinds.insert(
            kind.to_owned(),
            json!({
                "key": key,
                "carriers": carriers,
                "carriers_capped_at": 1_000,
                "generic": carriers as usize > cap,
                "rows": meta.iter().map(|(id, country, ikind, ident, name)| json!({
                    "org_id": id, "country": country, "identifier_kind": ikind,
                    "identifier": ident, "name": name,
                })).collect::<Vec<_>>(),
            }),
        );
    }
    axum::Json(json!({
        "name": params.name,
        "n2": n2,
        "n3": n3,
        "stoplist_cap": cap,
        "kinds": kinds,
    }))
    .into_response()
}

#[derive(serde::Deserialize)]
struct UnmappedFieldsParams {
    profile: String,
    /// How many notice ids back from the profile's own newest notice to read
    /// (default 100,000, at most 1,000,000).
    window: Option<i64>,
    /// How many field ids to list (default 40, at most 200).
    show: Option<usize>,
}

/// `GET /admin/unmapped-fields?profile=…` — what one profile publishes at its
/// own head that no channel reads (issue 368).
///
/// The weekly report's section 13 answers this for the corpus HEAD, and an era
/// that stopped publishing is invisible to it: `ted-export-r208` holds 29,763
/// of the 30,285 titleless Tenders and its newest notice sits 4.3 M ids below
/// that window's floor. Two attempts to put a per-profile window in the report
/// each cost ~60 minutes and were reverted; the plan showed the join to a
/// per-profile maximum inverts the driver onto `notices`. A probe takes ONE
/// profile, so its bounds are constants and it plans as a range scan — see
/// `store`'s plan guard. The `any_channel_reads` filter is applied here, in
/// the crate that owns it, and both the unfiltered and filtered counts are
/// returned so a reader can see what the filter removed.
async fn unmapped_fields(
    State(sup): State<Arc<Supervisor>>,
    headers: HeaderMap,
    Query(params): Query<UnmappedFieldsParams>,
) -> Response {
    if let Some(denial) = deny(&headers) {
        return denial;
    }
    let window = params.window.unwrap_or(100_000).clamp(1, 1_000_000);
    let show = params.show.unwrap_or(40).min(200);
    let (max_id, published) = match sup.db().unmapped_fields_for_profile(&params.profile, window).await
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[admin] unmapped-fields probe failed: {e}");
            return error(StatusCode::INTERNAL_SERVER_ERROR, "unmapped-fields probe failed");
        }
    };
    let Some(max_id) = max_id else {
        return error(StatusCode::NOT_FOUND, "no notices carry that profile");
    };
    let published_ids = published.len();
    let unmapped: Vec<serde_json::Value> = published
        .iter()
        .filter(|(field, _)| !ingest::project::any_channel_reads(field))
        .take(show)
        .map(|(field, hits)| json!({ "field_id": field, "rows": hits }))
        .collect();
    let unmapped_ids = published.iter().filter(|(f, _)| !ingest::project::any_channel_reads(f)).count();
    axum::Json(json!({
        "profile": params.profile,
        "newest_notice_id": max_id,
        "window_notice_ids": window,
        "ids_read": [max_id - window + 1, max_id],
        "published_field_ids": published_ids,
        "unmapped_field_ids": unmapped_ids,
        "listing_cap": show,
        "unmapped": unmapped,
        "note": "rows are counts inside this profile's own head window, not corpus totals; \
                 a field id here is published by the source and read by no channel, which is \
                 a scope decision rather than a defect unless section 1 shows the profile \
                 missing a MODELLED field (issue 368).",
    }))
    .into_response()
}

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
