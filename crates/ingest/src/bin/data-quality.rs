//! `data-quality` — run the semantic data-quality report (issue 27) against a
//! deployed tender-db and print it.
//!
//! It is the descriptive sibling of `bin/verify`: where `verify` asserts an
//! instance meets *external* ground truth and exits non-zero on any failure,
//! this tool has no pass/fail — it *measures* how complete the imported data is
//! (per-era field completeness, award linkage, results materialisation, TED↔DÖE
//! merge) and reports the numbers. The measurement itself lives in
//! [`ingest::data_quality`], transport-free and unit-tested; this binary only
//! moves rows over the account-gated `/v1/sql` endpoint.
//!
//! Every query is a bounded `GROUP BY` aggregate (issue 27's prod constraint):
//! no full-row dumps, safe to run against a live instance whose SQL endpoint is
//! rate-limited and isolated by design (issue 17). All of it needs an API token
//! (`--token`, or `TENDER_API_TOKEN`); without one there is nothing to report,
//! so the tool says so and exits non-zero rather than printing an empty run.
//!
//! Human-readable by default; `--json` for machines.

use ingest::data_quality::{self, Raw, Rows};
use serde_json::Value;
use std::process::ExitCode;

/// The production instance, and the default target.
const DEFAULT_BASE_URL: &str = "https://tenders.zebreus.click";

/// A thin client bound to one instance and its token.
struct Instance {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl Instance {
    /// Run one read-only SELECT via `/v1/sql` and return its rows. The endpoint
    /// takes the SQL as the raw request body and authenticates a Bearer token.
    async fn sql(&self, query: &str) -> Result<Rows, String> {
        let response = self
            .http
            .post(format!("{}/v1/sql", self.base_url))
            .header("authorization", format!("Bearer {}", self.token))
            .header("content-type", "text/plain")
            .body(query.to_owned())
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;
        let status = response.status();
        let text = response.text().await.map_err(|e| format!("read body: {e}"))?;
        if !status.is_success() {
            return Err(format!("HTTP {}: {}", status.as_u16(), api_error(&text)));
        }
        let body: Value = serde_json::from_str(&text).map_err(|e| format!("bad JSON: {e}"))?;
        let rows = body
            .get("rows")
            .and_then(Value::as_array)
            .ok_or("response had no `rows`")?
            .iter()
            .filter_map(|r| r.as_array().cloned())
            .collect();
        Ok(rows)
    }
}

/// Pull `error.message` out of an API error body, or fall back to the raw text.
fn api_error(text: &str) -> String {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).map(str::to_owned))
        .unwrap_or_else(|| text.chars().take(200).collect())
}

// --------------------------------------------------------------------- CLI

struct Args {
    base_url: String,
    token: Option<String>,
    json: bool,
}

fn usage() -> ! {
    eprintln!(
        "usage: data-quality [--base-url URL] [--token TDB…] [--json]\n\
         \n\
         Measures the semantic completeness of a deployed tender-db (issue 27):\n\
         per-era field completeness, award linkage, results materialisation and\n\
         the TED↔DÖE merge rate. Descriptive, not pass/fail.\n\
         --base-url  instance to measure (default {DEFAULT_BASE_URL})\n\
         --token     API token for /v1/sql (or env TENDER_API_TOKEN) — required\n\
         --json      emit the report as JSON"
    );
    std::process::exit(2);
}

fn parse_args() -> Args {
    let mut args =
        Args { base_url: DEFAULT_BASE_URL.to_owned(), token: std::env::var("TENDER_API_TOKEN").ok(), json: false };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = || it.next().unwrap_or_else(|| usage());
        match arg.as_str() {
            "--base-url" | "--base" => args.base_url = value().trim_end_matches('/').to_owned(),
            "--token" => args.token = Some(value()),
            "--json" => args.json = true,
            "-h" | "--help" => usage(),
            _ => usage(),
        }
    }
    args
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args = parse_args();
    let Some(token) = args.token.filter(|t| !t.is_empty()) else {
        eprintln!(
            "no API token: the data-quality report reads via /v1/sql, which is account-gated.\n\
             Pass --token TDB… or set TENDER_API_TOKEN (the acceptance-verify account)."
        );
        return ExitCode::FAILURE;
    };
    let instance = Instance { http: reqwest::Client::new(), base_url: args.base_url.clone(), token };

    // Run every query in order, keeping its label; one failing query means the
    // report cannot be trusted, so fail loudly rather than render a partial one.
    let mut results: Vec<(String, Rows)> = Vec::new();
    for (label, query) in data_quality::queries() {
        match instance.sql(&query).await {
            Ok(rows) => results.push((label, rows)),
            Err(e) => {
                eprintln!("query `{label}` failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    let raw = match Raw::from_labelled(results) {
        Ok(raw) => raw,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let report = data_quality::assemble(&args.base_url, &raw);

    let output = if args.json { data_quality::render_json(&report) } else { data_quality::render_text(&report) };
    println!("{output}");
    ExitCode::SUCCESS
}
