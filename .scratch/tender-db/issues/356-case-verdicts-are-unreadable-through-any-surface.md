# 356 — Recorded case verdicts are unreadable through any surface

Status: ready-for-agent (filed 2026-09-05 from the issue-355 campaign)
Kind: capability / operability (organization layer, review loop)
Relates to: 311 (the review loop), 314, 355 (the campaign that needed the 124 prior verdicts
as a calibration set and could not get them), 45 (why `/v1/sql` is an allow-list)

## The gap

`org_case_reviews`, `org_mention_rehoming`, `org_name_verdicts` and now `org_country_verdicts`
are written through `POST /admin/*` and read by nothing but their apply jobs. `/v1/sql` is a
positive table allow-list (issue 45) and these are not public tables, so a bounded
`SELECT ... FROM org_case_reviews` is a 400. The only read path is a `sqlite3` on the serving
DB, which `docs/agents/prod-box-reads.md` reserves for a snapshot under the team lead's word.

Measured on 2026-09-05: the 355 campaign wanted slice 1's 124 `xb-same-name-2026-08-31`
verdicts as a reference set for its blind sample and had to run without them.

## Proposal

`GET /admin/case-reviews?cohort=<c>&table=<case|rehoming|name|country>&limit=<n>` returning
the rows (verdict, confidence, diagnosis/handling/rationale, applied_at, applied_action,
job_id), bounded by `limit` (default 500, cap 5,000), newest first. Read-only, admin-secret
gated like the POSTs. One handler with a table switch keeps the four verdict stores behind
one door; the `cohort` filter is what every campaign asks for.

## Not in scope

Exposing any verdict table on `/v1/sql`: the allow-list stays positive (issue 45).
