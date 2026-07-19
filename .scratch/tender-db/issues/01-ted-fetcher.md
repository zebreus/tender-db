# 01 — TED package fetcher + raw archive

Status: resolved

Goal: the `ingest` crate exists and can download TED daily/monthly packages
into `/data/archive/ted/` (or a configurable root) with hash-based
idempotency and a `fetches` registry in the store.

Scope:
- New `crates/ingest` in the workspace; `store` gains the `fetches` table
  (url, kind, period, sha256, bytes, fetched_at, path) via its migration.
- Fetch daily `https://ted.europa.eu/packages/daily/{yyyy}{nnnnn}` and
  monthly `…/monthly/{yyyy}-{m}` (reqwest/rustls, Range-resume, retries).
- Idempotency: skip when sha256 matches registry; re-fetch dailies younger
  than publication-day 09:30 CET finality; never mutate archived files —
  a changed upstream package lands as a new registry row + file version.
- CLI entry (`cargo run -p ingest --bin fetch -- ted --day 2026-07-18`,
  `--month 2026-06`, `--range`) usable standalone on the VPS.
- Unit tests with a local HTTP fixture server; no network in tests.

Acceptance: on the VPS, fetching one recent day + one month lands correct
files under /data/archive/ted/, registry rows exist, re-running is a no-op.

## Answer

Implemented in commit bde1731 (ingest crate: fetch.rs/ted.rs + fetch CLI,
store fetches registry). Verified on the VPS against real TED:
daily 2026-00136 (19.8 MB) and monthly 2026-06 (395 MB) fetched into
/data/archive/ted/, re-runs report Unchanged (no download), probe-latest
resumes from the registry (137 -> NotFound -> stop). Registry rows in
/data/db/tender-db.db. Note: TED monthly tars are much smaller than the
extracted sizes in the research docs (compressed members).
