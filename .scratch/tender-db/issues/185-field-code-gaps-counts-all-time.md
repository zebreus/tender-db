# 185 — the field-code gap panel counts all-time rows, so a 99.998%-reclaimed bucket still reads as a 577K gap

Status: RESOLVED-VERIFIED on prod (2026-08-11 00:3x CEST, deploy e66967c) — field_code_gaps shows OC=10, not 577K
Kind: dashboard correctness (small code fix)
Blocked by: —
Relates to: 137 (#29 criterion 6 — outstanding-only presentation), 30 (introduced the panel), 35/72 (the OC reclaim that exposed it)

## Why

`Db::quarantine_field_code_gaps` (crates/store/src/lib.rs:1415) groups ALL
`reason='unknown-field-code'` rows with no `reprocessed_at IS NULL AND skipped_at IS NULL`
filter. Live effect (public `/api/dashboard`, 2026-08-10): the panel shows `OC 576,753` while the
still-held reason bucket is **10**. Issue 137 called this exact misread out — "576,753 rows and
10 outstanding — 99.998% done, and on the dashboard it reads as a 577K gap" — and the fix that
followed (`by_reason` counts outstanding only, per #29 criterion 6: *a reason whose rows were all
reclaimed is not a gap and must not be presented as one*) skipped this sibling query.

## What

1. Add the still-held predicate to `quarantine_field_code_gaps`, matching `by_reason`.
2. Update the doc comments that bake in the pre-reclaim world: `model/src/dashboard.rs`'s
   `field_code_gaps` field ("~entirely the one legacy OC code") and `quarantine_class`'s
   narrative ("unknown-field-code (577k)… unparsable-xml (628k)") describe 2026-07-21 numbers as
   if current — fine as history, wrong as description; date them or restate over outstanding.
3. The store-level unit test (lib.rs:3621) should pin the behavior: a reclaimed row must not
   count.
