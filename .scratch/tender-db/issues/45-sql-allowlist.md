# 45 — Flip the SQL gate from deny-list to positive allow-list

Status: needs-verification
Blocked by: 43 (immediate deny-list fix ships first)

The /v1/sql credential gate is a deny-list (FORBIDDEN). Issue 43 showed
the failure mode: a deny-list silently re-opens the moment a new private
table is added (webhook_endpoints leaked per-user signing secrets that
way). The durable fix is a positive allow-list — SQL may read only the
public surface, everything else is denied by default.

Scope:
- Define the allowed set explicitly: the public `v_*` views + the
  canonical/notice/org/bid/contract/lot tables + quarantine (raw notice
  payloads are public business data) — enumerate deliberately and
  comment why each is public.
- classify() rejects any identifier not in the allow-list (same lexer
  token stream it already walks); schema() lists only allowed tables.
- Tests: every private table denied (incl. any added later — add a test
  that fails if a non-allowlisted table becomes reachable); every
  advertised public table queryable; the existing string-literal /
  quoted / schema-qualified edge cases still hold.
- Consider surfacing the allowed set in /v1/sql/schema's notes so users
  know the queryable surface.

Acceptance: only allow-listed tables/views are queryable; a newly added
private table is denied by default (regression-proven); no legitimate
public query breaks.

## Resolution (api-polish, 2026-07-23)
ALLOWED is a 38-entry positive allow-list (v_* views + canonical/notice/org/
bid/contract/lot tables + quarantine + changes). `fetches` is DELIBERATELY
excluded (owner call): its `path` column is server filesystem layout —
ingestion/operator infra, not business data. Provenance (which package/period
a notice came from) is legitimately public and will be surfaced through a
path-free `v_fetches` view, deferred to the store lane with issue-50's analyst
views. classify() walks the parsed statement's table references (FROM/JOIN,
`x IN table`, every nested subquery/CTE) with an exhaustive no-wildcard `Expr`
match, so a `turso_parser` bump that adds a table-bearing variant fails to
compile rather than opening a hole.
