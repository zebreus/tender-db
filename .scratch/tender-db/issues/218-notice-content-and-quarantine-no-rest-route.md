# 218 — a notice's own parsed content (and any quarantined notice's payload) is reachable only via /v1/sql, not REST

Status: needs-triage — MEDIUM, CONFIRMED (code) 2026-08-15. Filed from the API completeness review (subagent).
Kind: completeness (a whole content class has SQL access but no REST surface)
Blocked by: —
Relates to: 50 (sql-analyst-surface), 87 (quarantine reasons), the ledger/quarantine dashboard work

## Gap

`/v1/notices/{id}` (`crates/app/src/v1/mod.rs:738-745`) serializes `NoticeRow` — publication identity plus
`parse_state` — and nothing from the notice's parsed payload. The parsed content tables
(`notice_sections`, `notice_texts`, `notice_amounts`, `notice_dates`, `notice_classifications`,
`notice_codes`, `notice_ids`, `notice_integers`, `notice_numbers`) and `quarantine` are all in the
`/v1/sql` `ALLOWED` list (`crates/app/src/v1/sql.rs:153-165`) as public business data — but appear in **no
REST response shape** (`json::notice`, json.rs:107-121, returns identity + parse state only).

For notices that project cleanly, the canonical tender detail is the intended content path, so this is
mild. But **quarantined / unparsed notices have no canonical projection at all** — their content exists
only in the `notice_*`/`quarantine` tables, so for them `/v1/sql` (token-gated) is the *only* way to see
anything, and the public `/v1/notices/{id}` returns a near-empty metadata stub.

## Consumer scenario

"Show me the parsed fields of notice 12" — if 12 projected, its data is under `/v1/tenders/{its tender}`,
not `/v1/notices/12`; if 12 quarantined, there is no tender and `/v1/notices/12` returns only
`parse_state`, so the payload is reachable only with a token and hand-written SQL.

## Fix direction

Either expand `/v1/notices/{id}` to embed the notice's parsed satellites (texts/amounts/dates/
classifications/ids…) and, for quarantined notices, the quarantine reason + payload; or add a
`/v1/notices/{id}/content` sub-resource. Keep it read-shaped and bounded (a single notice's satellites are
a small `(notice_id)` slice). Decide whether quarantine payloads belong on the unauthenticated surface or
should stay SQL-only — that is a data-exposure policy call, not just a plumbing one.

## Verification

- `GET /v1/notices/<projected id>` (or its `/content`) returns the notice's parsed fields.
- `GET /v1/notices/<quarantined id>` returns the quarantine reason and whatever payload policy allows,
  instead of a bare `parse_state` stub.
