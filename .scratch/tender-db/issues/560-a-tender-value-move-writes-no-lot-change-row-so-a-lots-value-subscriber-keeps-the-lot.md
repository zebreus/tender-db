# 560 — a version that moves only the Tender's value writes no lot change row, so a `/v1/lots` value-bound subscriber keeps a lot its bound now excludes

Status: ready-for-agent — filed 2026-10-16 from issue 547's build review (findings "lots value move", reproduced end to end
over a real SSE socket). Next: decide between emitting lot `changed` rows on a Tender head-value move (533's closure-flip
shape) and documenting the gap as permanent; measure the extra rows first.
Kind: correctness (SSE `added`/`removed` on `/v1/lots`)
Relates to:
- 547: the SSE `Scope::At` probe now reads the Tender's value at the probed version (`ValueBy::Version`), on
  `/v1/tenders` and `/v1/lots` alike. The lots half only runs when a lot change row exists at that version.
- 533: the same gap for `status`, closed by adding `closed_before != closed_now` to the lot `changed` condition.
- 493: a lot's OWN served value can move without a lot row too (measured 0 of 14,480 on prod, so left documented).
- 490 decision 6: a lot's `min_value`/`max_value` bound reads its Tender's value.
- 561: the earlier side of the value probe is a superset; independent of this one.

## Verify

    grep -c 'fn a_tender_value_move_alone_reaches_a_lots_value_subscriber' crates/app/tests/api.rs

- **done**: 1 (the test exists and passes: a version that changes only the procedure total moves the lot in or out of
  a `/v1/lots?min_value=`/`max_value=` subscription), OR a `## Decision` below keeps the gap, with the CHANGELOG
  547 bullet still naming it.
- **open (2026-10-16)**: 0. CHANGELOG 547's `/v1/lots` bullet and the `ValueBy::Version` doc name the gap.

## What was found

A lots stream reads only `lot` change rows (`sse.rs` `diff`, `changes_since(.., Some(kind))`). The fold writes a lot
`changed` row only when `before.facts != lot.facts || before.kind != lot.kind || closed_before != closed_now`
(`canonical.rs` `append_version_changes`). The Tender's value is elected over procedure and lot figures
(`head_value_eur_cents_with`), so it can move while no lot fact does:
- a corrigendum of the procedure total only (TED R2.0.9 F14 correcting II.1.5);
- a sibling lot's figure moving the election in a multi-lot Tender (every other lot keeps its facts);
- a CAN's result value beside lot facts carried over unchanged ("a lot's facts seldom move on a CAN", 533).

Reproduced twice in the 547 review (scratch logs, 2026-10-16):
- synthetic R2.09 CN (procedure EUR 1,000,000, LOT-1 EUR 300,000) then an F14 correcting II.1.5 to EUR 400,000:
  Tender value 100000000 → 40000000 cents, change rows `tender:1@1:added lot:1@1:added tender:1@2:changed` (no lot row
  at seq 2). `/v1/tenders?min_value=50000000` got `removed`; `/v1/lots?min_value=`, `?max_value=` and the unfiltered
  lots stream stayed silent, while the pages flipped.
- the eForms chain with lot-scope parties stripped and only `RequestedTenderTotal` changed: tenders min/max got
  `added`/`removed`, every lots tape `None`.

eForms lots that name a lot-scope party carry a per-notice `Fact::Party`, so they get a lot row on every version and
hide the gap (the api.rs chain fixtures do).

## Units

1. **Measure (S).** On prod, a bounded windowed read: consecutive versions whose `current_value` would differ (or a
   proxy: tender `changed` rows with no lot row at the same seq, for Tenders with lots). Size the extra lot rows that
   option A would write per daily append.
2. **Decide (S).** A: add a Tender head-value move to the lot `changed` condition — needs the elected value of the
   previous and this version in `append_version_changes` (the fold carries a running `ScalePartners`; the previous
   version's value is the previous iteration's). B: keep the gap documented.
3. **Build (M)** if A: the condition, an api test (the Verify's name) that changes only the procedure figure, and
   retire the CHANGELOG 547 lots caveat.
