# 567 — a legacy TED award block resolves to the wrong lot when a section's ITEM ordinal equals another section's LOT_NO

Status: ready-for-agent — MEASURED 2026-10-17 (wf_3865d965-f11, 304 multi-lot legacy CANs read from the parsed layer, every misattribution re-traced by a skeptic): 0/100 in 2012–2015 (R2.0.8 has no lot sections), 4/100 in 2016–2019, 11/104 in 2020–2023 (10.6 %, 95 % 5.4–18.1 %; 6.1 % of award blocks), 12 of 15 verified cases put a winner or value on the wrong lot. DECIDED: fix read_legacy_results to match by published lot number, order-independent; refold the legacy R2.0.9 CANs. NEXT: build (wf running), gate, deploy, sized refold.
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

