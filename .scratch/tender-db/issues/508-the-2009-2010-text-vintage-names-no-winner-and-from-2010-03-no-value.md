# 508 — the 2009-12 → 2010 text vintage names no winner, and from 2010-03 claims no value

Status: ready-for-agent — UNIT 1 MEASURED 2026-10-10 (hourly firing, found while working issue 491 unit 3).
Kind: parse coverage (text era, `crates/ingest/src/text/parse.rs`)
Relates to: 244 (the era's winner/value campaign; its "2010 tail" open question is this), 491 (same parser, same
`awarded_value`)

## What is wrong

The text era's last 13 months print section V in two shapes the parser does not read.

**From 2009-11-28 (notice id ≈ 3,930,000): a new winner heading.** The new standard forms say

    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT
    AWARD DECISION HAS BEEN TAKEN: Mike Priwitzer, Friedensstr. 39, 17179
    Gnoien, DEUTSCHLAND.

`AWARD_LABELS` knows `HAS BEEN AWARDED:` (2006 onward), not `HAS BEEN TAKEN:`. The winner is never claimed. Values
are still claimed, because the value lines keep their colons (`Total final value of the contract:` / `Value: 59 752
EUR.`).

**From 2010-02-25 (notice id ≈ 4,020,000) to the era's end (4,352,355, 2010-12-31): the colons go.**

    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT
    AWARD DECISION HAS BEEN TAKEN
    Clarke Machinery Ltd.
    New Inn, Ballyjamesduff, Co. Cavan
    IRELAND
    V.4)  INFORMATION ON VALUE OF CONTRACT
    Total final value of the contract
    Value 80 515 EUR
    Including VAT. VAT rate (%) 21

The heading ends its line and the name is the next line, with the address on the lines after it. The value is
`Value <figure> <currency>` with no colon. `read_value_item`'s sub-label retry splits on the last `:`, so it finds
nothing. The value is lost along with the winner.

Exhibits, served 2026-10-10:
- 8245313 (notice 3990107, 2009-12) serves `value` 59,752 EUR and only its buyer. The winner Mike Priwitzer is missing.
- 8255697 (notice 4200000, 2010-08) serves `value: null` and only its buyer. Clarke Machinery Ltd. and 80,515 EUR
  are both missing.

## Unit 1 — measurement (2026-10-10)

Bounded `/v1/sql` windows over `notice_texts` (`TXT-TX` bodies), `notice_amounts` and the parse layer's
`TED-OFFICIALNAME`. Scripts and outputs are in `../508-2010-text-vintage/` (`shape.sh`, `stored.sh`,
`stored-3900000-4359999.txt`).

| window (notice ids) | award bodies | winners stored | values stored |
|---|---:|---:|---:|
| 3,900,000–3,929,999 (before 2009-11-28) | 12,141 | 11,698 (96 %) | 7,765 |
| 3,930,000–4,019,999 (colon, new heading) | 39,866 | 512 (1.3 %) | 25,082 |
| 4,020,000–4,352,355 (colonless) | 121,734 | 851 (0.7 %) | 1,698 (98,295 bodies state `Total final value`) |

- **Winners.** 161,600 award notices from 2009-11-28 on; 156,853 print the new heading; **1,363 carry a stored
  winner**.
- **Values.** From 2010-02-25, 98,295 bodies state a total final value; **1,698 carry a stored figure**. Before the
  switch, about 63 % of award bodies carry one.
- **The colonless name line.** Sampled 10,339 heading occurrences (windows 4.04 M, 4.10 M, 4.30 M):
  - 2 have no line after the heading;
  - 713 name lines carry a comma (e.g. `Ghenova Civil, S.L.`);
  - about 3 % run to the wrap width (≥ 66 characters) and continue on the next line (consortia, long Greek
    municipal names).
- **The heading is regular.** Every one of the 10,341 `HAS BEEN TAKEN` occurrences in the colonless sample ends its
  line.

## Units

1. **Measure.** Done (above).
2. **Build.**
   - (a) `HAS BEEN TAKEN:` joins `AWARD_LABELS`.
   - (b) The colonless winner: a line ending in `HAS BEEN TAKEN` names the winner on its next non-empty line. That
     line goes through the same segment rules as the flat path: `;` splits, contract references are hopped, lot
     prefixes stripped, the first comma ends the name, and the authority's contact entry is dropped. A line that is
     itself a section marker (`V.4)`, `SECTION`, `CONTRACT NO`, `LOT NO`) names nothing. A wrapped name is cut at the
     wrap, not joined: joining would append the street line to a 70-character name that happens to fill the line.
   - (c) The colonless value: when the colon retry finds nothing, retry after the last whole word `Value`, with the
     same guard (the skipped part holds no digit).
   - Tests use the real bodies above, a multi-contract body (4101747), a comma name (4040925), and an
     initial-plus-final V.4 (4301811). Then measure on the sampled bodies with the real functions.
3. **Drain.** Re-parse the text packages for 2009-11 → 2010-12 (fetches 186–199, one package per job,
   `reclaim_only: true`), then one `project`. That is about 422 k notices. Check it stays under the 500 k
   full-fallback line, or split the fold.
4. **Verify** below, plus the exhibits and data-quality section 3 (text-era winner-named density).

## Verify

    echo "SELECT COUNT(DISTINCT notice_id) FROM notice_texts WHERE notice_id BETWEEN 4100000 AND 4109999 AND field_id='TED-OFFICIALNAME'" | ssh root@zebreus.click /root/sq.sh

- **open** (2026-10-10): `[[11]]`. 3,363 award bodies in that window, 3,269 with the new heading.
- **done:** about 3,000 or more (the window's named winners), after the unit 3 re-parse.
