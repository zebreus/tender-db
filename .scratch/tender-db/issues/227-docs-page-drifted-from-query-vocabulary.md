# 227 — /docs has drifted from the served query vocabulary; openapi.json is guarded, the prose page is not

Status: RESOLVED 2026-08-16 (commit `a3139e3`) — /docs caught up and now GUARDED against the spec.
Kind: documentation drift + a missing guard
Blocked by: —
Relates to: 216-A (the "newest matching first" lie — same class), 215 (contract drift cluster)

## Resolution

1. **Content.** /docs gained: `bidder`, `publication_id`, `identifier`, `name_prefix`, the
   `published_*`/`deadline_*` bounds, `sort`/`order` in the filter table; two new sections —
   **Ordering tenders** (the sort table, bound-implies-sort, both-bounds→400, per-sort cursors,
   no-SSE rule, worked examples) and **Lookups by real-world key** (notice number → notice, VAT →
   org → participation history, name search with the Unicode-case story); a **Notice content**
   section for `/v1/notices/{id}/content` with the envelope and value types; perf-table rows for
   the external-key lookups (1–8 ms) and ordered lists (2–13 ms), measured 2026-08-16.
   Also fixed while there: the applies-where table wrongly listed `tender` as ignored on
   `/v1/notices` (honoured since issue 49 via the app-layer dispatch) — the 216-A class again.

2. **Guard.** `the_docs_page_names_the_whole_spec_surface` (api.rs, beside the openapi guard):
   walks `openapi.json`'s `components.parameters` + `paths` and requires each to occur in the
   docs.rs source. Anchored byte-grep — a query param counts only as `>name<` (a code span/table
   cell naming exactly it) or `name=` (a usage example), a path param as `{name}` — so prose
   words like "sort"/"order" cannot vouch for an undocumented parameter. **Demonstrated red**:
   stripping every `name_prefix` mention fails the gate with the parameter named; note a single
   surviving mention (e.g. the perf-table `?name_prefix=…`) keeps it green by design — the gate
   is "documented at all", not "documented everywhere".

## Original defect

`crates/app/src/v1/docs.rs` — the human-readable reference at `/docs` — mentioned NONE of this
week's query vocabulary: `publication_id`, `identifier`, `name_prefix`, `sort`/`order`,
`published_after/_before`, `deadline_after/_before` had zero occurrences (only `bidder` appeared,
once). The machine spec (`openapi.json`) was current because `the_openapi_spec_matches_the_served_surface`
enforces it; the prose page had no equivalent guard, so it silently fell a week behind.

## Verification

- Every parameter name and path in openapi.json occurs in docs.rs — enforced by the new test,
  red-demonstrated once (name_prefix strip), green on the shipped page. Full api suite: 40/40.
