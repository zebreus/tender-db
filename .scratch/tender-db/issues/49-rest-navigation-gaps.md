# 49 — REST: advertised per-id endpoints 404; filters silently ignored

Status: ready-for-agent
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
