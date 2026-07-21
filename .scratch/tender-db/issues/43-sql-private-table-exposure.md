# 43 — SQL endpoint exposed webhook secrets + private tables (HIGH)

Status: needs-verification (immediate deny-list fix landed; allow-list = issue 45)
Severity: HIGH (cross-account credential leak)

Found by a fresh-eyes security review, code-verified by the owner
(2026-07-21). The /v1/sql gate's deny-list was `FORBIDDEN =
["users","api_tokens","sessions"]` (crates/app/src/v1/sql.rs) with the
false premise "everything else is public business data". But
`webhook_endpoints` stores, per user, `secret TEXT` (the plaintext
Standard-Webhooks HMAC signing key) and `url` (private endpoint) —
neither in FORBIDDEN, and `execute()` applies no user-scoping. So any
account holder (open registration) could:

    POST /v1/sql   SELECT user_id, url, secret FROM webhook_endpoints

and dump every user's signing secret → forge valid v1-signed payloads to
their endpoints, defeating the HMAC authenticity guarantee; plus leak all
private URLs. `/v1/sql/schema` actively advertised the table and its
`secret` column. Same class: `webhook_delivery_log` (cross-account
history), `job_queue`/`job_log` (operator job params).

Plaintext-secret-at-rest is an accepted risk for the operator's own DB
(one-box threat model) — but "any registered user reads all of them via
the public SQL endpoint" was never the intent.

Live exposure at discovery: ~nil (pre-launch, no real webhook rows), so
no secrets were actually leaked — fixed before it mattered.

## Immediate fix (landed)
Expanded FORBIDDEN to the complete private set: users, api_tokens,
sessions, webhook_endpoints, webhook_delivery_log, job_queue, job_log —
enumerated against the full schema (all other tables are notice/canonical
public business data). Regression test `account_private_tables_are_denied`.
Closes the hole (both `classify()` and `schema()` read the same const).

## Durable fix
Issue 45 — flip the gate to a positive allow-list. A deny-list re-opens
silently the next time a private table is added; this incident is exactly
that failure.

Acceptance: SELECT from any private table → 400; /v1/sql/schema omits
them; verified against prod after deploy.
