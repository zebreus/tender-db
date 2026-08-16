# 227 — /docs has drifted from the served query vocabulary; openapi.json is guarded, the prose page is not

Status: needs-triage — MEDIUM (doc-vs-behavior, the issue-216-A class in reverse: missing rather than
wrong), CONFIRMED 2026-08-16 (owner audit while the org-names backfill ran).
Kind: documentation drift + a missing guard
Blocked by: —
Relates to: 216-A (the "newest matching first" lie — same class), 215 (contract drift cluster)

## Defect

`crates/app/src/v1/docs.rs` — the human-readable reference at `/docs` — mentions NONE of this week's
query vocabulary: `publication_id`, `identifier`, `name_prefix`, `sort`/`order`,
`published_after/_before`, `deadline_after/_before` all have zero occurrences (only `bidder` appears,
once). The machine spec (`openapi.json`) is current because `the_openapi_spec_matches_the_served_surface`
enforces it; the prose page has no equivalent guard, so it silently fell a week behind. A developer
reading `/docs` today would conclude the flagship queries ("closes soon", newest-first, find-by-VAT,
find-by-name) do not exist.

## Fix

1. Write the missing sections into docs.rs (the collections' filter tables + the tenders sort/order
   story + the org lookups + `/v1/notices/{id}/content`).
2. Add the GUARD so this cannot recur: a test that walks `openapi.json`'s `components.parameters` names
   + `paths` and asserts each appears at least once in the docs.rs source (byte-grep is enough — the
   goal is "documented at all", not prose quality). New vocabulary then fails the gate until `/docs`
   names it, exactly like honoured_params.

## Verification

- Every parameter name in openapi.json occurs in docs.rs; the new test stays green after removing any
  one mention (i.e., it actually fails when a param is dropped — demonstrate red once).
