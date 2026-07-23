# 49 — REST: advertised per-id endpoints 404; filters silently ignored

Status: needs-verification (parts 1–4 complete)
Severity: MEDIUM (traceability + correctness footguns)

Found by usability audit, owner-confirmed (2026-07-21). The `/v1` root
advertises `/v1/organizations` and `/v1/notices`, but:

1. **`/v1/notices/{id}` and `/v1/organizations/{id}` return 404.**
   Confirmed: /v1/notices/14350 → 404, /v1/organizations/31033 → 404.
   Yet a tender detail hands you `versions[].caused_by_notice_id` and
   `parties[].organization_id` — ids you then cannot fetch. The
   ADR-0001 traceability chain dead-ends at the API. Add the per-id
   GET endpoints (and return JSON 404, not the HTML SPA page — see 51).
2. **`/v1/notices?tender={id}` silently ignores the filter** and returns
   unrelated notices. Honor it (list a tender's notices) or reject it.
3. **Unknown/mistyped query params are silently ignored** — `cvp=72`
   (typo for cpv) returns ALL tenders as if unfiltered, so an analyst
   wrongly concludes everything matched. 400 on unknown params.
4. **List rows omit `cpv` and `country`** — you can filter on them but
   not see them, so you can't tell why a row matched. Echo them in list
   items.

Acceptance: advertised entities are fetchable by id (JSON); `?tender=`
on notices works or 400s; unknown params 400; list rows carry the fields
you can filter on.

## Progress (api-polish, 2026-07-23)
Parts 1–3 landed (app-lane only):
- `/v1/notices/{id}` + `/v1/organizations/{id}` (JSON row, JSON 404).
- `/v1/notices?tender={id}` honored via the tender detail's
  `caused_by_notice_id` chain (404 on unknown tender, not an unfiltered dump).
- Unknown/mistyped query params → 400 (`serde(deny_unknown_fields)` on Params).
Tests: `advertised_entities_are_fetchable_by_id`,
`notices_can_be_scoped_to_a_tender`, `unknown_query_params_are_rejected`.

Part 4 (echo `cpv`+`country` in tender LIST rows) — DONE (store lane cleared by
lead). `TenderRow` gains `cpv`/`country`; `read::tenders()` echoes them via two
correlated `group_concat` subqueries keyed by (tender_id, seq); `json::tender`
emits them as arrays. Required a new index — `tender_version_classifications`
was the ONE version satellite lacking a `(tender_id, seq)` index (texts/amounts/
dates all had one), so the subqueries would have scanned; added
`tender_version_classifications_version` mirroring the siblings. An
EXPLAIN-QUERY-PLAN test (`classification_echo_seeks_the_version_index_not_a_scan`)
pins the O(page) guarantee. The index builds once on first prod open (a deploy-
timing consideration the lead owns).
