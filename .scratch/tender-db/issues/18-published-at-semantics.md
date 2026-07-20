# 18 — published_at should be the publication date, not dispatch

Status: resolved
Blocked by: —

Finding (verify-15): tender_versions.published_at is populated from the
notice's dispatch date (BT-05 / legacy DS), but the name — and the API field —
imply the OJ publication date. TED issue 136 dispatches 07-16, publishes
07-17; the acceptance harness had to switch to id-set-membership because of
the skew.

Fix: populate published_at from the actual publication date where derivable —
for TED the daily package's issue date (fetch period) is authoritative; DÖE
pubDay likewise; fall back to dispatch when no package date exists (direct
notice re-fetches). Keep dispatch as its own column (dispatched_at) since
ordering within a day uses it. API: tender versions + notices expose both,
documented. Re-projection (rebuild) refreshes historical rows — schedule one
after deploy.

Acceptance: verify-15's Search-API day-check can use published_at date
equality again on a sample day; API docs updated; rebuild executed in prod.

## Comments

2026-07-20 — Resolved (commit "Issue 18: published_at is the publication
date; add dispatched_at"). Investigation on the real corpus settled the
per-era sourcing:

- **TED eForms** carries the true OJEU publication date in
  `efac:Publication/efbc:PublicationDate` (**OPP-012-notice**, on 100% of the
  corpus), one day after `cbc:IssueDate` dispatch (**BT-05(a)-notice**). The
  old code already preferred OPP-012, so TED was already correct — the real
  gaps were (a) no separate dispatch axis and (b) DÖE.
- **DÖE** publishes on the national portal, with **no** `efac:Publication`
  block: eforms-de carries OPP-012 on only ~2%, so publication falls to
  `RequestedPublicationDate` (**BT-738-notice**); sdk-0.1 carries only
  **SDK01-RequestedPublicationDate** (100%) and **SDK01-IssueDate** (~29%
  dispatch). Previously sdk-0.1 `published_at` was 0.
- **Legacy** `TED-DATE_PUB`/`TXT-PD` (publication) vs
  `TED-DS_DATE_DISPATCH`/`TXT-DS` (dispatch) — unchanged, now split cleanly.

One resolver, `ingest::project::notice_instants`, returns `(published_at,
dispatched_at)` and feeds both the notice row (stamped at process time) and
the tender version, so they always agree. `dispatched_at` is a new nullable
column on `notices` and `tender_versions`; both are exposed on the /v1
tender, version and notice payloads. verify.rs's day-check note updated (the
set-membership check is kept as the stronger assertion). The package period
was investigated and rejected as a source (TED daily periods are OJS *issue
numbers*, not dates, and monthly packages span many publication days), so the
date is always taken from the notice content. Tests: a per-era resolver unit
test and a real-fixture round-trip. A prod re-projection refreshes historical
rows and runs as part of the full backfill (these fixes land before it, so no
extra reprocessing is needed).
