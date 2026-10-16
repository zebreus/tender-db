# 547 — SSE's per-version probe reads the HEAD columns for deadline, value, published and status, so a version's entering or leaving a filter on those is misclassified

Status: ready-for-agent — units 1-2 COMMITTED `323fd37` and GATED green 2026-10-16 (151 suites; three review rounds: wf_9679cf84-369, wf_c32a3269-1ec, wf_ea99545a-23c). On main, with 563 (`3a1845d`, gated green) on top. NEXT: deploy main (547 + 563 in one deploy) once the queue is idle AFTER 524 v11's canary wet run 2565 has run (a deploy changes the rev, so it must not land between plan 2562 and 2565), then both Verifies (the date-bounded stream answers 200; a `now`-bounded stream answers the refusal).
of writing. Next: gate (`ops/check.sh`), commit, deploy — read path only, no backfill — then the Verify. Follow-ups
filed: 560, 561, 562, 563. Was: needs-triage — filed 2026-10-15 from issue 533's unit-1 decision (accepted risk "Other
head-relative SSE predicates").
Kind: correctness (SSE `added`/`removed` classification)
Relates to:
- 533 (adds `StatusBy::Version` for `status` only, under `Scope::At`);
- 164 / 287 (SSE removal over-delivery);
- 490 (lot values stored per version because `Scope::At` serves non-head versions);
- 216 / 273 (the head columns).

## Verify

    grep -c 'ValueBy::Version { head_seq' crates/store/src/read.rs; curl -sN -m 3 -D - -o /dev/null -H 'Accept: text/event-stream' 'https://tenders.zebreus.click/v1/tenders?deadline_before=2026-01-24' | head -1

- **done**: the grep prints 3 (the `version_predicates` arm and the `tenders_query` and `lots_query_previous`
  callers), and the deployed server answers the date-bounded stream with `HTTP/2 200`. The api
  tests `a_version_that_moves_the_value_across_a_bound_is_removed_and_added`, `…_deadline_across_a_bound_…`,
  `a_version_published_across_a_bound_…`, `a_version_that_brings_the_first_value_or_deadline_is_added`,
  `a_resumed_subscription_reads_a_superseded_version_at_its_own_value_and_deadline` and
  `a_version_inside_a_bound_on_both_sides_is_changed` pass.
- **open (2026-10-16, before deploy)**: the grep prints 0 on `main`, and prod answers `HTTP/2 400` ("sort does not
  apply to event streams"). (The old Verify grep for `StatusBy::Head("t.current_deadline"), "t.current_value_eur_cents"`
  read 0 hits since 533 unit 2 replaced it with `STATUS_BY_HEAD`, so it read "done" without the fix.)

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

## Units 1-2 (2026-10-16): decided, built, review-fixed

**Decision.** Under `Scope::At` (the SSE probe only; pages keep the head columns):
- **dates, exact:** `published_*` reads `v.published_at`; `deadline_*` reads `version_deadline_sql` — the MAX of the
  version's admitted `submission_deadline` rows through canonical's `admitted_deadline_rows_sql`, the one selector
  533's `StatusBy::Version` uses (the `head_election_agreement` count moved to name it), and what `head_deadline`
  writes into `t.current_deadline` at the head.
- **value, exact at the head, a superset before it:** `ValueBy::Version` reads `t.current_value_eur_cents` when
  `seq = t.current_seq`, else `EXISTS` an amount row of that version inside the bound. No elected value is stored per
  version (the election is not transcribed into SQL: 366, 375), so the error is one-sided: an earlier version with an
  un-elected figure inside the bound matches. Under `max_value` that is ANY smaller figure — every lot figure of a
  Tender with several lots — so `added` usually arrives as `changed`. The served docs now say to apply `changed` as an
  upsert by id. Exact needs the value stored per version: **561**.
- **lots:** the value bound reads the Tender's value at the probed version, the same CASE. A lots stream hears it
  only when the version writes the lot's own change row (facts, kind, closure); a version that moves only the
  Tender's value probes no lot: **560**.
- **`source`, `kind`:** left on `t.*`. The issue's "they do not change between versions" was false — the fold
  rewrites both for the whole Tender (DÖE → TED when a TED twin joins; 462's kind), so no row keeps the earlier
  label: **562**.

**Review fixes (2026-10-16).**
- The lots At probe's lot row is bound `vl.seq = ?` (the At seq), not `vl.seq = v.seq`: with the value CASE on `v`,
  the planner put `vl` first on `(tender_id=?)` and walked every version's lot rows (0.003 s → 0.35 s a probe at
  2,400 lots × 31 versions; a flip of that Tender is 4,800 probes per subscriber). The plan test now faults any lots
  `vl` leg that does not seek `(tender_id=? AND seq=? AND lot_id=?)`, with a control that rejoins it.
- The amount EXISTS carries `seq IS NOT head_seq`, like `closed_at_version_sql`'s `cb_v.seq <> head_seq`: turso ran
  the ELSE loop on head probes too (0.046 s against 0.004 s on 2,400 amount rows with none inside the bound).
- A date-bounded stream opens: `/v1/tenders` implied `sort=deadline`/`published_at` from a lone date bound before it
  checked for a stream, then refused the stream for that sort. One stream rule now (re-review round 2, option A): on
  a stream, `sort` unset or `id` and `order` unset or `asc` are accepted and mean id order whatever the date bounds
  (both columns included); `sort=published_at`, `sort=deadline` and `order=desc` get the "sort does not apply to
  event streams" 400, checked before the both-bounds 400 (whose "pass sort=…" advice was a dead end on a stream) and
  the descending-id 400. The first fix round accepted `order=asc` without a date bound and refused it beside one.
  Pages answer byte-identically. The tests subscribe without `sort=id`;
  `a_tenders_stream_takes_id_order_whatever_its_date_bounds` pins the rule and the page controls.
- Tests added for the NULL side (the first value or deadline appearing), the ELSE branch on the new side (a resumed
  subscription past two versions), and a both-inside `changed`; `next_ops` reads the entity kind.
- CHANGELOG, `/docs`, openapi: qualified as above; "apply `changed` as an upsert by id" is pinned by
  `the_docs_say_the_sse_snapshot_is_at_least_once`.

**Re-review fixes, round 2 (2026-10-16).**
- `lots_query_previous`'s comment claimed `v` and `vl` both seek their full key whatever the filter; only `vl` does.
  Under `source`, `kind` or `tender` the planner still seeks `v` by `(tender_id=?)` (O(versions) a probe, as before
  547). The comment says so, and `at_probe_plan_tests` now plans `source`, `kind`, `tender`, `kind` + value band +
  `open`, and all three with `min_value`, so the `vl` full-key fault covers the filters that reorder the join.
- The stream sort/order rule above (one rule, both-bounds dead end gone).
- Served docs: the four openapi date-bound descriptions, the two `/docs` table rows and the both-bounds sentence say
  "(pages only; a stream snapshots in id order)"; the `/docs` ordering paragraph states the stream rule. The
  `changed`-for-unreceived clause names `source`/`kind` beside the value bounds, and "the diff stream then makes
  your final state exact" names its known exceptions (560, 562).
- 561's text: under `min_value` the superset also gives `changed` where `added` was due.
- A `now` date bound is re-resolved on every (re)connect, so a resumed stream is not exact: documented on `/docs`
  and in openapi (pass an absolute timestamp), filed as **563**.
  - Superseded 2026-10-16 by 563 (decision B): that caveat is retired from `/docs`, openapi and this issue's
    CHANGELOG bullet, replaced by a 400 for a `now` date bound on a `/v1/tenders` stream. The two ship in one deploy.

