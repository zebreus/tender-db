# 493 — a lot whose elected value moves emits no lot change

Status: ready-for-agent — filed 2026-10-08 from issue 490's deploy-B review (`wf_a689b740-7c8`, verified
against the code). This predates issue 490: the read-time pick had the same dependence. NEXT: unit 1,
measure how often it happens before changing the change feed.
Kind: change feed / SSE coherence
Relates to: 490 (the stored lot value), 471 (the exact-10ᵏ rule), 389

## What is wrong

`append_version_changes` (`crates/store/src/canonical.rs`, the lot arm) emits a lot `changed` row only
when `before.facts != lot.facts || before.kind != lot.kind`. A lot's served `value` also depends on
things outside its own facts, through `elect_lot_value`:

- **The running scale rule's partners:** the Tender's tender-level amounts, the other lots' amounts and
  the lot awards, from every version so far.
- **Corroboration by the head version's other fields:** `ScalePartners::set_head`.
- **The version's rate date:** the EUR conversion behind the €1 bn gate and the €100 bn ceiling.

So version N+1 can republish a lot unchanged while adding a tender-level figure exactly 1/1000 of the
lot's €3 bn figure. The scale rule then refuses the lot figure at N+1, and the REST lot `value` drops,
but only a TENDER `changed` row is written. `/v1/changes` pollers and SSE `?include_data` lot
subscribers keep the stale lot value.

## Direction

The fold already elects both versions' lot values in its write loop. It should also emit a lot `changed`
when the elected `(value_cents, value_currency)` differs from the previous version's. Check the cost
first: every all-profile refold re-emits history anyway, but a daily append must not start emitting a
change per lot.

## Units

1. **Measure.** Count, over a bounded window, consecutive versions whose lot facts are equal but whose
   stored `value_*` differ (after issue 490's backfill, both sides are stored).
2. **Decide and build.** Emit a lot change on a value move, with a fold test (v1 lot €3 bn; v2 adds a
   1/1000 tender figure; expect a lot `changed` at v2). Or document the gap, if unit 1 finds it
   vanishingly rare.
