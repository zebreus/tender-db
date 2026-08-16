# 218 — a notice's own parsed content (and any quarantined notice's payload) is reachable only via /v1/sql, not REST

Status: PART A (quarantine) DONE — committed `ec456a3`, awaiting deploy. `/v1/notices/{id}` now carries a
`quarantine` object (current + original cause, attempts, and the reclaimed/skipped terminal stamps) or
`null` when the notice parsed, via `read::notice_quarantine` (a bounded `(notice_id)` seek). This closes
the SHARP half — a quarantined notice has no parsed satellites and no tender, so this was its only
unreachable-via-REST content. PART B (the parsed satellites of a cleanly-parsed notice, the MILD half) is
still open — see below.
Kind: completeness (a whole content class has SQL access but no REST surface)
Blocked by: —
Relates to: 50 (sql-analyst-surface), 87 (quarantine reasons), the ledger/quarantine dashboard work

## PART B (open) — the parsed satellites of a cleanly-parsed notice

For a notice that PROJECTED, its parsed fields live in the `notice_*` satellites and are reachable via
the tender it projects to (`/v1/tenders/{its tender}`) or `/v1/sql`, but not as the notice's OWN content.
This is the "mild" half the issue flags: the canonical tender detail is the intended path, so it is a
convenience/completeness gap, not an unreachable-content gap. A `/v1/notices/{id}/content` sub-resource
returning the section-grouped `notice_sections`/`texts`/`codes`/`classifications`/`amounts`/`dates`/
`integers`/`numbers`/`ids` (all bounded `(notice_id)` slices) would close it. Deferred as its own slice —
it is a large response shape (8 satellite tables) with lower per-byte value than PART A, and warrants its
own firing. Raw quarantine PAYLOAD bytes remain a deliberate NON-goal on the unauthenticated surface
(size + unvalidated-bytes footgun; the source notice is already public via TED/DÖE, and `/v1/sql` remains
for the token-holder who needs the raw member).

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
