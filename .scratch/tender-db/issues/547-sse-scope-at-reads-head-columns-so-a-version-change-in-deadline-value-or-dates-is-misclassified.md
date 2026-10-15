# 547 — SSE's per-version probe reads the HEAD columns for deadline, value, published and status, so a version's entering or leaving a filter on those is misclassified

Status: needs-triage — filed 2026-10-15 from issue 533's unit-1 decision (accepted risk "Other head-relative SSE
predicates"). Next: write a failing api test for one predicate (e.g. `value_min` on a change that lowers the
value), then decide the per-version shape.
Kind: correctness (SSE `added`/`removed` classification)
Relates to:
- 533 (adds `StatusBy::Version` for `status` only, under `Scope::At`);
- 164 / 287 (SSE removal over-delivery);
- 490 (lot values stored per version because `Scope::At` serves non-head versions);
- 216 / 273 (the head columns).

## Verify

    grep -n 'StatusBy::Head("t.current_deadline"), "t.current_value_eur_cents"' crates/store/src/read.rs

- **done**: no hit in `tenders_query`, or the hit is limited to `Scope::Page`. A test proves that a change row whose
  version leaves a `value_min`, `deadline_after` or `published_after` filter is delivered as `removed`.
- **open (2026-10-15)**: one hit, in `tenders_query` (read.rs ~2831). It applies to both scopes.

## What was found

The SSE diff (crates/app/src/v1/sse.rs ~420-470) probes each change row twice:
- the new side, `Scope::At { id, seq }`;
- the old side, `seq - 1`.

It classifies the row as added, removed, changed or skipped by which side matches the subscriber's `Filter`.

`tenders_query` builds both probes with `version_predicates(..., StatusBy::Head("t.current_deadline"),
"t.current_value_eur_cents")` and appends `t.current_published_at` and `t.current_deadline` bounds (read.rs
~2818-2831). Those are HEAD columns. Version facts (cpv, nuts, text, …) are read at `v.seq`, but the
deadline/status, value and published predicates read the same head row on both sides.

So a change that moves a tender across one of those bounds reads the same on both probes. Both sides match, which
gives `changed` where `added` or `removed` was due. Or neither matches, which gives `(false, false) => continue`
(sse.rs:465), and the event is dropped. Example: a corrigendum lowers the value below a subscriber's `value_min`. Both
probes read the new head value, and the subscriber never gets `removed`, so the client keeps a tender its filter
excludes.

533's decision fixes `status` only (`StatusBy::Version`). This issue covers the rest:
- `value_min`/`value_max`;
- `deadline_after`/`deadline_before`;
- `published_after`/`published_before`;
- `kind`;
- `source`.

Source and kind do not change between versions, so they are probably harmless.

## Units

1. **Reproduce (S).** Write an api.rs SSE test: subscribe with `value_min`, fold a version that drops the value
   below it, and expect `removed`. Do the same for `deadline_before`.
2. **Decide and build (M).** Under `Scope::At`, read each predicate from the version's own rows:
   - value: `tender_version_*` value at `v.seq`, if stored per version (490 stores lot values per version; check the
     tender-level value);
   - dates: `tender_version_dates` at `v.seq`, through the 474 `deadline_admitted_sql` helper.

   Share the helpers with 533's `StatusBy::Version`, so the deadline window keeps one home.
