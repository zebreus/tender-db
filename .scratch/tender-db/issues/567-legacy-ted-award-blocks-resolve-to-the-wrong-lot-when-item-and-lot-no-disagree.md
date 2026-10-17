# 567 — a legacy TED award block resolves to the wrong lot when a section's ITEM ordinal equals another section's LOT_NO

Status: needs-triage — filed 2026-10-17 from issue 551 unit 4b's adversarial check (wf_efa005fa-6a8), mechanism confirmed
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
