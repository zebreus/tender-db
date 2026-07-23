# 49 — REST: advertised per-id endpoints 404; filters silently ignored

Status: needs-verification (parts 1–3); part 4 deferred (store change)
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

Part 4 (echo `cpv`+`country` in tender LIST rows) is DEFERRED: it needs a
`crates/store` change — `TenderRow` carries neither field and `read::tenders()`
does not select them (they live in `tender_version_classifications`, scheme
`cpv`/`nuts`). Deriving them app-side would be an N+1 over `tender_detail`,
unacceptable for a list. Clean fix: two correlated subqueries in
`read::tenders()` (like the title/deadline picks) + two `TenderRow` fields,
echoed as arrays in `json::tender`. Pinged lead for a store-lane decision.
