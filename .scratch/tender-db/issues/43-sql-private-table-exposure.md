# 43 — SQL endpoint exposed webhook secrets + private tables (HIGH)

Status: RESOLVED-SUBSUMED (2026-08-17, owner). The permanent fix — issue 45's positive allow-list —
was prod-verified 2026-08-16 (see 45's status: AST walk, 45 ALLOWED entries, denied by default,
`sqlite_schema` refused live), and it subsumes this issue's interim deny-list entirely: a
default-deny surface cannot re-expose a private table by omission, which was this issue's risk. And
independently re-confirmed today from the other side: the issue-58-v2 verification found the NEW
private tables (`legacy_adjacency`, `legacy_ojs_keys`) unreachable through `/v1/sql` — the
default-deny doing exactly what a deny-list could not have promised for tables that did not exist
when the list was written.
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
