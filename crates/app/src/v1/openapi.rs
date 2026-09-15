//! `GET /v1/openapi.json` — the machine-readable twin of `/docs`.
//!
//! The document is vendored beside the code as data (`data/openapi.json`), the
//! same discipline as the quarantine ledger and the coverage ground truth:
//! updating the API description is an edit to a table, not a patch. It is kept
//! in step with the router by the end-to-end completeness test in
//! `tests/api.rs`, which fires a request at every path the spec declares and
//! rejects any that the router does not actually serve.
//!
//! CORS comes from the shared unauthenticated-surface middleware
//! (`super::public_cors`), which is what lets browser-based viewers (the
//! hosted Swagger UI, Redoc) load the document straight from this origin.

use axum::http::header;
use axum::response::{IntoResponse, Response};

/// The vendored OpenAPI document.
pub const SPEC: &str = include_str!("../../data/openapi.json");

/// Serve the spec. No state, no arguments: the content is a constant.
pub async fn spec() -> Response {
    ([(header::CONTENT_TYPE, "application/json; charset=utf-8")], SPEC).into_response()
}

#[cfg(test)]
mod tests {
    use super::SPEC;
    use serde_json::Value;

    /// The vendored document is valid JSON with the OpenAPI skeleton in place —
    /// a parse failure is a build-time mistake, caught here rather than by the
    /// first external viewer that loads it.
    #[test]
    fn the_vendored_spec_parses_and_carries_the_skeleton() {
        let spec: Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        assert_eq!(spec["openapi"], "3.0.3", "the OpenAPI version is pinned");
        assert!(spec["info"]["title"].is_string());
        assert!(
            spec["info"]["description"].as_str().is_some_and(|d| d.contains("cents")),
            "the money convention is stated up front"
        );
        let paths = spec["paths"].as_object().expect("a paths object");
        assert!(paths.len() >= 20, "the surface is described, not sketched: {}", paths.len());
        // The licence travels with the machine-readable form too.
        assert_eq!(spec["info"]["license"]["name"], "AGPL-3.0-or-later");
    }

    /// Every operation carries an operationId (generated clients need them) and
    /// every path lives under a known prefix — a typo'd path would otherwise
    /// silently describe an endpoint nobody serves (the e2e harness in
    /// tests/api.rs then proves the served half).
    #[test]
    fn every_path_is_prefixed_and_every_operation_named() {
        let spec: Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        for (path, item) in spec["paths"].as_object().expect("paths") {
            assert!(
                path.starts_with("/v1")
                    || ["/health", "/health/deep", "/metrics", "/_source", "/docs"]
                        .contains(&path.as_str()),
                "unexpected path prefix: {path}"
            );
            for (method, op) in item.as_object().expect("path item") {
                assert!(
                    op["operationId"].is_string(),
                    "{method} {path} is missing an operationId"
                );
                assert!(
                    op["responses"].as_object().is_some_and(|r| !r.is_empty()),
                    "{method} {path} documents no responses"
                );
            }
        }
    }
    /// Issue 398: the spec must not describe `NoticeDetail.quarantine` as a
    /// held-today flag, and must name the field that IS one.
    ///
    /// A prose guard, and deliberately a narrow one — it cannot prove the
    /// sentence is right, only that the specific wrong reading does not come
    /// back. It is worth having because that reading was wrong for ~99.75 % of
    /// the notices it applied to and nothing noticed for months: the ledger keeps
    /// a quarantine row after the member is reclaimed, so a non-null
    /// `quarantine` is the hold HISTORY. The behavioural half is pinned in
    /// `store/tests/notice_quarantine.rs`
    /// (`a_reclaimed_hold_is_still_served_and_says_so_in_its_stamps`); this only
    /// keeps the document from drifting back out of step with it.
    #[test]
    fn the_quarantine_field_is_not_described_as_a_held_today_flag() {
        let spec: Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let detail = &spec["components"]["schemas"]["NoticeDetail"];
        let field = detail["allOf"]
            .as_array()
            .and_then(|a| a.iter().find_map(|s| s["properties"]["quarantine"].as_object()))
            .expect("NoticeDetail describes a quarantine property");
        let text = field["description"].as_str().expect("the property is described");
        assert!(
            text.contains("parse_state"),
            "the field must name the held-today predicate: {text}"
        );
        assert!(
            text.contains("reprocessed_at"),
            "and say what a reclaimed record looks like: {text}"
        );
        assert!(
            !text.contains("null when it parsed"),
            "the retired claim: most notices carrying this object DID parse: {text}"
        );
        let whole = detail["description"].as_str().expect("NoticeDetail is described");
        assert!(
            !whole.contains("A held notice has no parsed satellites and no canonical tender, so its"),
            "the description must not assert held-ness of every notice carrying the field: {whole}"
        );
    }
}
