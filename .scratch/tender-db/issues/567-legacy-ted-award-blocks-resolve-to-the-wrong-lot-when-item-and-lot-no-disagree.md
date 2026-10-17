# 567 — a legacy TED award block resolves to the wrong lot when a section's ITEM ordinal equals another section's LOT_NO

Status: ready-for-agent — MEASURED 2026-10-17 (wf_3865d965-f11, 304 multi-lot legacy CANs read from the parsed layer, every misattribution re-traced by a skeptic): 0/100 in 2012–2015 (R2.0.8 has no lot sections), 4/100 in 2016–2019, 11/104 in 2020–2023 (10.6 %, 95 % 5.4–18.1 %; 6.1 % of award blocks), 12 of 15 verified cases put a winner or value on the wrong lot. DECIDED: fix read_legacy_results to match by published lot number, order-independent; refold the legacy R2.0.9 CANs. BUILT 2026-10-17 (unit 3, commit b957661, gate green 1411 s): `LegacyLotNumbers` + `normalise_lot_number`; one deliberate widening of unit 2 recorded below (a no-number award block keeps its ordinal where every lot's number is its ordinal). DEPLOYED b957661 2026-10-17 16:22Z. SIZED: job 2587 (dry, expect 1) — 1,381,903 notices carry TED-LOT_NO (estimate ~1.4 M, 95 % 0.96–2.0 M); its paired project 2588 ran over 0 notices. WET 2026-10-17 17:12Z: `refold-fields` TED-LOT_NO `"expect":1381903` enqueued as job 2589 with its project 2590 (3cd94c0 deployed first; 551's fallback walk and read had finished on the pre-567 projection). NEXT: read 2589's `stamped N` and 2590's compare counts (D7: verified + corrected ≥ N), then the 15 read back and a fresh uniform 30.
by a code read. Next: measure the reach — a bounded census of R2.0.9/R2.0.8 notices whose lot sections carry LOT_NO values
that are not their ITEM ordinal in document order (e.g. lots published 2, 1 or 3, 5 with sections numbered 1, 2), and of
those how many award blocks resolved to a lot whose LOT_NO differs from the block's own LOT_NO.
Kind: correctness (legacy TED lot results: winners and award values attached to the wrong Lot)
Relates to:
- 551 (path W's S3a–d shapes: TED lots keyed by ordinal `LOT-k`; unit 4b's R2 needs the published lot number);
- 534 / 490 (legacy result reading);
- research §2.2 (legacy links a result to its lot positionally by LOT_NO).

## Verify

    grep -n '"TED-LOT_NO" | "TED-LOT_NUMBER" | "TED-ITEM"' crates/ingest/src/project.rs

- **done**: `read_legacy_results` keys the lot map by the published LOT_NO / LOT_NUMBER only (ITEM only as a fallback when
  no lot section of the notice carries a LOT_NO), and an award block resolves by its own LOT_NO first; a test with lot
  sections published in reverse order (section 1 = lot 2, section 2 = lot 1) attaches each award to its own lot.
- **open (2026-10-17)**: both lookups mix the two keys.

## What was found

`crates/ingest/src/project.rs` `read_legacy_results` (~7114–7151):

- `lot_by_no` is filled from every lot section's `TED-LOT_NO`, `TED-LOT_NUMBER` AND `TED-ITEM` values into ONE map with
  `entry(..).or_insert_with(..)`, so the FIRST section to claim a string wins. `TED-ITEM` is the r209 walker's ordinal
  attribute that every section carries (crates/ingest/src/r209/parse.rs ~344), not a published lot number.
- An award block then sets `r.lot_key` from each of its own `TED-LOT_NO` / `TED-LOT_NUMBER` / `TED-ITEM` values in
  turn, so the LAST of them in the block's value order decides.

Example: lot sections in reverse order — section A (ITEM 1, LOT_NO 2), section B (ITEM 2, LOT_NO 1). The map becomes
"1"→A (A's ITEM), "2"→A (A's LOT_NO); B's ITEM "2" and LOT_NO "1" are both ignored by `or_insert`. Both award blocks
(LOT_NO 1 and LOT_NO 2) resolve to A: lot 1's winner and value land on lot 2, and lot B gets no result. A notice that
awards a subset (lots 3 and 5 described in sections 1 and 2) resolves correctly only while ITEM and LOT_NO collide in a
harmless order.

Unmeasured. Reach matters for winners on lots (`/v1/lots?winner=`), lot award values, and 551's lot sums.

## Unit 1 — measured (2026-10-17, workflow wf_3865d965-f11)

Sample: one multi-lot legacy CAN per random OJ S day (≥ 2 own lot_results, ≥ 2 LotResult sections), its parsed values read
from `/v1/notices/{id}/content` — the layer `read_legacy_results` reads — and the code's mapping re-implemented exactly;
the copy predicted the API's served `lot_results[].lot` for every one of the 682 award blocks of 2020–2023. Evidence:
`.scratch/tender-db/567-lot-mapping/` (`measure_result.json`, the sampler and mapping scripts).

| period | multi-lot CANs | at risk | misattributed | rate (95 %) |
|---|---|---|---|---|
| 2012–2015 (all R2.0.8) | 100 | 0 | 0 | 0 % (0–3.6 %): R2.0.8 award forms carry no lot section, so the map is empty |
| 2016–2019 (78 R2.0.9, 22 R2.0.8) | 100 | 5 | 4 | 4.0 % (1.1–9.9 %); 5.1 % of the R2.0.9 |
| 2020–2023 (all R2.0.9) | 104 | 13 | 11 | 10.6 % (5.4–18.1 %); 41 of 672 award blocks (6.1 %), 34 with winners |

Two triggers, both confirmed on the API:
- **Published numbering differs from the section ordinal**: lots swapped (501322-2018: LOT-1 is LOT_NO 4, LOT-4 is LOT_NO
  1), skipped (305919-2017: lot 6 skipped, LOT-6..13 carry LOT_NO = ITEM + 1) or rotated (258495-2019: LOT_NO 3,4,5,1,2).
- **The values are read in the notice_ids primary-key order** (`section_id, field_id, ordinal`), so section ids sort as
  TEXT: `LOT-10` comes before `LOT-9`, and LOT-10's ITEM "10" claims the key "10" before LOT-9's LOT_NO "10". The result
  differs from the r209 walker's document order in notices with ten or more lot sections — 7 such notices in the 2020–23
  sample, where only the index order predicts production.

Each misattribution puts one award on another lot (that lot then carries two results) and leaves the right lot with
none: wrong winners on `/v1/lots?winner=`, wrong lot award values, and wrong inputs for 551's lot sums. 3 of the 15
verified wrong-lot blocks were no-award decisions (no winner, no value).

Not 567, filed as 568: in 72 of the 100 R2.0.8 multi-lot CANs (2012–2015) the Tender has 2 + lots from its contract
notice's annex B, yet the awards stay tender-scoped (no lot section to map to).

## Unit 2 — decided (2026-10-17)

The fix is mechanical and the reach is real (≈ 10 % of 2020–23 multi-lot legacy CANs): build it.
- `lot_by_no` takes a lot section's `TED-LOT_NO` / `TED-LOT_NUMBER` only; `TED-ITEM` only for a notice where NO lot
  section carries a LOT_NO / LOT_NUMBER. Two sections publishing the same number: neither is taken (ambiguous → None).
- An award block resolves by its own `TED-LOT_NO`, else `TED-LOT_NUMBER`, else (only in the ITEM-only notice) its
  `TED-ITEM` — never "last value wins". Exact match first, then a normalised match (trim, leading zeros, a `Lot`/`Los`/
  `Lote`/`Lotto` prefix) only when it is unique.
- The result is independent of value order (the primary-key order is not document order).
- Drain: refold the legacy R2.0.9 CANs — a `refold-fields` over the result-side lot-number field (`TED-LOT_NO` carriers),
  dry first for the size, wet with `expect`, its `project`; then a bounded read of the 15 misattributed notices above
  (each award on the lot its LOT_NO names) and a fresh uniform 30 of corrected Tenders.

## Unit 3 — built (2026-10-17)

`crates/ingest/src/project.rs`: `read_legacy_results` now collects each award block's own `(field, value)` lot
references while it reads the values and resolves them once the block is read, through `LegacyLotNumbers`:
- **The lot map** (`LegacyLotNumbers::read`) is keyed by a lot section's `TED-LOT_NO` / `TED-LOT_NUMBER` only;
  `TED-ITEM` keys it only where no lot section of the notice publishes a number. Each key keeps the SET of lot
  sections publishing it, so a number two sections publish is ambiguous and names none. Empty values are skipped.
- **An award block** (`resolve`) is read by the first field it carries of `TED-LOT_NO`, `TED-LOT_NUMBER`,
  `TED-ITEM` (an ITEM only where the ordinal is a key, below). Every value of that field must name the same lot; a
  value that names none or two that disagree leave the block on no lot. No cascade to a lower field on a miss: a
  block whose LOT_NO no lot publishes is lot-less, as it was under the old read (LOT_NO sorts after ITEM).
- **A lookup** (`lookup`) takes the exact trimmed key first; only where no lot publishes it exactly, the
  `normalise_lot_number` key (trimmed, lower-cased, a leading `Lot`/`Los`/`Lote`/`Lotto` word and a `nr`/`no`/`n°`
  after it dropped, leading zeros of a number dropped; `Lots 2` stays itself), and it must name one lot section.
- **Order**: sets throughout and a field choice by presence, so no value order matters. The tests read every
  fixture in the store's primary-key order, its reverse and document order, and require one answer.

**One widening of unit 2, deliberate.** Unit 2 limits an award block's ITEM to the ITEM-only notice. Taken
literally, that unplaces award blocks that print no LOT_NO in a notice whose lots do: 27 blocks in 6 of the 694
cached unit-1 notices (19645087, 20792981, 21414166, 20822492, 23049879, 21272117), all placed today on the lot of
their ordinal; the 19 whose award title names its lot match it (20792981's eleven `Delaftale` lots, 21414166's five
laptops, 23049879's two food lots, 21272117's `VL Linz`), the other 8 print one title for every award (19645087,
20822492). The build keeps that placement
where it cannot be 567's collision: `ordinal_is_key` is true in the ITEM-only notice AND in a notice where every lot
section publishes exactly its ordinal as its number (normalised), and an ITEM is looked up only then. In a notice
whose numbers are not its ordinals (swapped, skipped, rotated) a no-number block names no lot. The port finds 0
such blocks in the 694. If the decision should stand literally, delete the `|| { ... }` arm of `ordinal_is_key`
and the second half of `a_well_numbered_legacy_notice_maps_as_before`.

**Port and replay.** `.scratch/tender-db/567-lot-mapping/new_mapping.py` mirrors the Rust; over the 694 cached
legacy award notices of unit 1 against `mapping.py` (production, primary-key order): 3,413 award blocks, 3,362
unchanged, 51 changed: 45 from the wrong lot to the lot their LOT_NO names (all 15 verified notices, every
verified block) and 6 from no lot to it through the normalised number (20004819: awards `01`, `06`…`09` against
lots `1`…`72`; 20356321: `01`), each right by its title. Nothing moved elsewhere and nothing dropped to the Tender
(`new_mapping_result.json`).

**Tests** (`project::tests`, beside issue 512's): `a_reverse_numbered_legacy_notice_puts_each_award_on_its_own_lot`
(the issue's example), `a_swapped_lot_pair_resolves_by_the_published_number_501322_2018`,
`a_skipped_lot_number_resolves_by_the_published_number_305919_2017` (13 sections, LOT-10 before LOT-9 asserted),
`a_rotated_numbering_resolves_by_the_published_number_258495_2019` (the three from the notices' parsed values,
`/v1/notices/{20128178,19411290,20451472}/content` read 2026-10-17, identical to unit 1's cache),
`an_item_only_legacy_notice_keeps_its_ordinal_mapping` (and R2.0.8's no-lot-section case),
`a_lot_number_published_twice_is_ambiguous`, `a_lot_number_matches_its_normalised_spelling_when_unique` (and
`normalise_lot_number`'s cases), `a_well_numbered_legacy_notice_maps_as_before` (12 sections, numbered and
no-number awards; a misnumbered no-number pair names no lot). Focused runs green (filters legacy, lot_result,
read_legacy, project, golden, r209; and the r208, r209, project, text, internal_ojs integration binaries in full);
no golden moved.

**Drain (designed, docs/operations.md "Legacy award blocks by their published lot number").** `refold-fields` over
`TED-LOT_NO` on `notice_ids`: dry with `"expect":1`, wet with the count (it queues its `project`). The job sweeps
field ids, not section kinds, so the award-side LOT_NO cannot be selected alone; every notice whose mapping can move
carries a LOT_NO (lot sections or award blocks), R2.0.8 award forms carry no lot section (nothing moves there), so
`TED-LOT_NUMBER` stays out. Size, estimated from a uniform 60 `ted-export-r209` notices drawn from the public API
(`size_sample.py`, `size_sample.jsonl`): 19 carry `TED-LOT_NO` → about 1.4 M carriers (95 % ~0.96–2.0 M) of 4,490,549;
the `project` about 550's (4.3 h), so 3–4 h in a queue gap. Expected: tens of thousands of Tenders corrected
(6 of the 60 are multi-lot CANs with award LOT_NOs, ~450 k, at unit 1's 4–10.6 %). Then the 15 notices read back
(their tenders and the 45 expected moves are listed in the runbook) and a fresh uniform 30 corrected Tenders
from the `project`'s `version: null` `tender changed` rows, bar 30 of 30.
