//! `GET /v1/openapi.json` — the machine-readable twin of `/docs`.
//!
//! The document is vendored beside the code as data (`data/openapi.json`), the
//! same discipline as the quarantine ledger and the coverage ground truth:
//! updating the API description is an edit to a table, not a patch. It is kept
//! in step with the router by the end-to-end completeness test in
//! `tests/api.rs`, which fires a request at every path the spec declares and
//! rejects any that the router does not actually serve.
//!
//! Served with `Access-Control-Allow-Origin: *` — deliberately, and safely: the
//! document is public, static and secret-free, and the header is what lets
//! browser-based viewers (the hosted Swagger UI, Redoc) load it straight from
//! this origin. No other `/v1` response carries CORS headers.

use axum::http::header;
use axum::response::{IntoResponse, Response};

/// The vendored OpenAPI document.
pub const SPEC: &str = include_str!("../../data/openapi.json");

/// Serve the spec. No state, no arguments: the content is a constant.
pub async fn spec() -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
        ],
        SPEC,
    )
        .into_response()
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
                path.starts_with("/v1") || ["/health", "/health/deep", "/_source", "/docs"].contains(&path.as_str()),
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
}
