# Owner verification matrix (goal: every CONTEXT.md behaviour verified first-hand)

Status legend: [x] verified by owner, [~] partially / indirect evidence, [ ] pending.
Each check names HOW it was/will be verified. Date-stamped on completion.

## Product faces

- [~] REST API around Tenders/Lots — /v1/tenders 200 with items incl.
  dispatched_at (2026-07-21, curl). Full-surface pass (filters, lots,
  notices, organizations endpoints) pending post-backfill.
- [ ] SSE on every endpoint: snapshot event then added/changed/removed
  diffs, resumable via Last-Event-ID. Verify: curl a stream during live
  ingestion, kill, resume with Last-Event-ID, assert no gap.
- [x] Change-cursor poll endpoint — /v1/changes?since=193000 returns
  cursored added/changed events (2026-07-21, curl). SSE cross-check pending.
- [~] SQL endpoint layered gate — verified 2026-07-21 (owner, live):
  raw-SQL body; valid SELECT 200 (COUNT over 4.01M notices); INSERT →
  400 "only SELECT statements are allowed"; multi-statement → 400
  "only a single statement is allowed"; PRAGMA → 400; no token → 401;
  /v1/sql/schema self-documents incl. hidden auth tables. Result caps +
  rate limit probes pending (gentle, later).
- [ ] Webhooks: register URL, receive POSTed change events, retries,
  https-only-public enforcement. Verify: registration + reject-http
  probe now; delivery needs a receiver (no external resources — use the
  integration tests as evidence + document the gap honestly).
- [x] Dashboard shows coverage + data quality — 2026-07-21 visual pass in
  Chrome: live job progress, queue, quarantine breakdown, award linkage,
  coverage per era, snapshot age; no console errors; loads in ~0.16s.
- [~] Account lifecycle — register (account id 4 "owner-verify") + token
  create verified live 2026-07-21; login/revoke/delete exercised at the
  end of verification (delete kills the working token).
- [x] Unauthenticated basic queries — /v1/tenders without token, 2026-07-21.
- [x] AGPL §13: API root advertises license + source_offer
  (/_source → 200) and dashboard footer links source. 2026-07-21.
- [ ] Rate posture sanity: SSE per-IP cap, ~10 rps/IP. Light probe only
  (production box) — a handful of parallel streams, not a load test.

## Data model & semantics

- [~] Era profile dispatch (text/r208/r209/eforms + eforms-DE + DÖE
  sdk-0.1) — all eras present in the coverage grid with plausible volumes
  (2026-07-21). Era-ladder API check (one real notice per era) runs in
  `verify` post-backfill.
- [~] Two-layer model traceability (ADR-0001) — SQL spot-walk verified
  2026-07-21: tender_versions.caused_by_notice_id → notices.publication_id
  → fetches(source, period) resolves for sampled rows. Cross-era samples
  post-backfill.
- [ ] Organization merge rules: auto-merge ONLY on exact official ids;
  name-only stays provisional. Verify: SQL — no canonical merge without a
  registration/VAT id; provisional count plausible (dashboard shows
  2 292 provisional / 13 224 canonical, 2026-07-21).
- [x] Island notices → single-notice Tenders — SQL-verified 2026-07-21:
  180 tenders with island_notice_id set, matching the dashboard count.
  (DÖE volume grows post-backfill; mechanism confirmed.)
- [~] Money = INTEGER cents + currency — verified 2026-07-21 via SQL:
  tender_version_amounts(cents INTEGER, currency) e.g. 750000000 cents
  PLN estimated_value. Timestamp original-offset check pending (needs a
  notice-layer sample vs source XML).
- [ ] BRIN notices become minimal Tenders of distinct kind. Verify: SQL.
- [ ] Legacy chains via OJ references; break at eForms boundary; missed
  link splits, never wrongly merges. Verify: issue-27 report + spot-check
  a known chain.
- [~] Strict quarantine (ADR-0004): unmapped content → whole notice held,
  reason recorded, reprocessable — mechanism observed live (1.21M entries
  with reasons mid-backfill). Triage of the composition = issue 15/27
  acceptance; "reprocessable" pending an actual reprocess drill.
- [x] Fetch/process separation + hash idempotency — observed directly:
  re-walks fast-forward as dup (identity dedup) without refetching;
  archive fetched once (200GB), processed repeatedly. 2026-07-21.
- [ ] TED↔DÖE merge on strong cross-reference only (ADR-0003). Verify:
  issue-27 merge-rate + spot-check one merged Tender's provenance.

## Full data (the big gate)

- [ ] Backfill complete: TED 1993→ + DÖE 2022-12→ processed, projected.
  In progress: job 1/5 at ~pkg 164/401 re-walk (2026-07-21 12:45Z).
- [ ] `verify` acceptance run green vs external ground truth (per-year
  counts ±2%, Search-API id-membership, era ladder). Owner-run.
- [ ] Quarantine fully triaged: every reason-bucket classified as
  benign-by-design (documented) or bug (fixed + reprocessed). Current
  leads: unparsable-xml 628k, unknown-field-code 577k (top-N histogram).
- [ ] Award linkage plausible post-projection (currently 96-99% unchained
  = suspected mid-backfill artifact; research predicts ≈17% for R2.0.9).
- [ ] lot_results density sane (12,600 present 2026-07-21 — issue 22).
- [ ] Data-quality baseline (issue 27) committed to docs/research/.
- [ ] Live updates observed 3 consecutive days (scheduler tick day 1 =
  2026-07-21, observed; day 2, day 3 pending).

## Infrastructure & operations

- [x] Monolith serving during heavy ingestion: /health <120ms, /admin/jobs
  <5ms, / ~2ms under load (2026-07-21, measured).
- [x] Durable job queue: survives restart by design; tests green; live
  restart drill = next deploy (kill -9 equivalence).
- [ ] Snapshot job: runs (queued #6), lands in /data/snapshots, verified
  offline (integrity + row count). Then: restore drill per runbook with
  measured duration.
- [x] Writer-poison trap (turso 0.7 dropped-write-future): unconditional
  ROLLBACK on error paths confirmed in store (accounts.rs:216,
  lib.rs:583, canonical.rs ×3). Code-read 2026-07-21.
- [x] Raw archive on filesystem, DB stores references not blobs —
  confirmed by design + 200GB archive / DB separation on /data. 2026-07-21.
- [x] Deploy pipeline: flake build on VPS, atomic switch, rev stamping,
  regression guard, health gate — exercised twice today first-hand.
- [x] Off-box alerting without external accounts: /health/deep +
  4-hourly cloud routine (notify-on-fail) — created + test-fired
  2026-07-21.
- [ ] Continuous mode: TED daily 09:35 CET + DÖE T+1 + finality re-fetch —
  verify the scheduler pipeline contents in code + observe 3 days.

## Architecture & codebase (goal: clear and minimal)

- [ ] Owner architecture review once the current agent commits land
  (25/26/27): crate boundaries (model/ingest/store/app), no dead code,
  no compat shims, docs/adr/ current. Method: read-through + cargo-udeps
  -style dead-dep check + ADR cross-check.
- [ ] "Easy data inspection" — decide the bar: /v1/sql + docs + dashboard
  exist; verify the SQL endpoint is genuinely pleasant (schema
  discoverability, error messages) by using it for the spot-checks above.
