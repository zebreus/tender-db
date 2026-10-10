# 508 — the 2009-12 → 2010 text vintage names no winner, and from 2010-03 claims no value

Status: ready-for-agent — UNIT 2 BUILT AND REVIEWED 2026-10-10 (WIP `0ae3fbf`; review `wf_20e02b2c-6bb`, 7 confirmed, all fixed). NEXT: gate, push, deploy, then unit 3's drain in two rounds (below).
Was: ready-for-agent — UNIT 1 MEASURED 2026-10-10 (hourly firing, found while working issue 491 unit 3).
Kind: parse coverage (text era, `crates/ingest/src/text/parse.rs`)
Relates to: 244 (the era's winner/value campaign; its "2010 tail" open question is this), 491 (same parser, same
`awarded_value`)

## What is wrong

The text era's last 13 months print section V in two shapes the parser does not read.

**From 2009-12-02 (first body notice 3,931,555, fetch 198): a new winner heading.** The new standard forms say

    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT
    AWARD DECISION HAS BEEN TAKEN: Mike Priwitzer, Friedensstr. 39, 17179
    Gnoien, DEUTSCHLAND.

`AWARD_LABELS` knows `HAS BEEN AWARDED:` (2006 onward), not `HAS BEEN TAKEN:`. The winner is never claimed. Values
are still claimed, because the value lines keep their colons (`Total final value of the contract:` / `Value: 59 752
EUR.`).

**From the 2010-03 package (fetch 195; first body notice 4,023,765) to the era's end (4,352,355, 2010-12-31): the colons go.**

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
| 3,900,000–3,929,999 (before 2009-12-02) | 12,141 | 11,698 (96 %) | 7,765 |
| 3,930,000–4,019,999 (colon, new heading) | 39,866 | 512 (1.3 %) | 25,082 |
| 4,020,000–4,352,355 (colonless) | 121,734 | 851 (0.7 %) | 1,698 (98,295 bodies state `Total final value`) |

- **Winners.** 161,600 award notices from notice 3,930,000 on; 156,853 print the new heading; **1,363 carry a stored
  winner**.
- **Values.** From notice 4,020,000 on, 98,295 bodies state a total final value; **1,698 carry a stored figure**. Before the
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
3. **Drain.** Fetches 186–198 (2010-12 … 2009-12; 422,001 notices, ids 3,930,355–4,352,355). Fetch 199 (2009-11)
   carries no new heading and is left out. Two rounds, because the fold's legacy closure counts the seeds' Tenders'
   member notices too (about +35 %, measured by the review), so 422 k seeds would close at ~530–570 k, over the 500 k
   line, and fall back to a whole-corpus fold. That matches issue 244's own record: fetches 186–197 blew the cap, and
   its rule v2 was ≤ 8 packages (~260 k) for this range.
   - Round 1: `{"kind":"reparse","profiles":["text"],"after":185,"packages":7,"reclaim_only":true}` (fetches
     186–192, 234,469 notices), then one `{"kind":"project"}`.
   - Round 2: `{"kind":"reparse","profiles":["text"],"after":192,"packages":6,"reclaim_only":true}` (193–198,
     187,532), then one `project`.
   - Before each project: confirm the reparse log names the expected fetches and no other legacy re-parse is
     pending. Then read `[project] incremental: N changed notices` and `[project] legacy closure: … → N` in its log.
4. **Verify** below, plus the exhibits and data-quality section 3 (text-era winner-named density).

## Verify

    echo "SELECT COUNT(DISTINCT notice_id) FROM notice_texts WHERE notice_id BETWEEN 4100000 AND 4109999 AND field_id='TED-OFFICIALNAME'" | ssh root@zebreus.click /root/sq.sh

- **open** (2026-10-10): `[[11]]`. 3,363 award bodies in that window, 3,269 with the new heading.
- **done:** about 3,000 or more (the window's named winners), after the unit 3 re-parse.

## Unit 2 — built and reviewed (2026-10-10)

The build is (a) `HAS BEEN TAKEN:` in `AWARD_LABELS`, (b) `line_awarded_names`, (c) `colonless_total` under the
`TOTAL FINAL VALUE` label only, and (d) the multi-award guard extended. Tests use the real bodies (3990107, 4200000,
4101747, 4040506, 4040925, 4101508).

**Measured before the review** with the real functions over the sampled bodies (a temporary ignored harness, not
committed):
- **Colonless bodies.** Names in 99.2 %, a value in 80.7 % (was 0.7 % and 1.4 % stored). The remaining
  refusals are ranges (`Lowest offer … and highest offer …`), zeros, three-decimal figures and unit prices, which
  the colon print refuses too.
- **Colon-era regression.** 12,412 stored values reproduced exactly, none lost or changed. The 17 "gained" are
  non-TD:7 bodies the harness reads past `claim_awarded_value`'s gate.
- **`HAS BEEN TAKEN`.** It occurs in 0 of 216,974 pre-switch bodies sampled (1.0 M–3.9 M), so (a) and (b) reach no
  older vintage.

**Review** (`wf_20e02b2c-6bb`, three lenses, one refuter per finding). 7 confirmed, all fixed:
1. **(major) A bare country line in an empty slot was taken as the winner.** 198 awards across the colonless windows
   would have folded into one organization per country (80 `GERMANY`). Fixed: a line that is only a country
   (`NAME_TO_ALPHA2`) names nobody. Exhibit 4201806.
2. **A name the wrapper broke was cut at the wrap.** About 0.9 % of names, roughly 3,400 across the era, would
   have minted second spellings (issue 234). Fixed: `wrapped_continuation` joins the next line only when the body is
   wrapped (widest line ≤ 80), the next line's first word could not have fitted, and the line holds no digit,
   country, item or street opener. On the sample 158 names changed, almost all to their full form. The known
   leaks are `routes des Gatines` and some junk-name tails (`… a été déclaré infructueux`).
3. **Several award blocks with no `CONTRACT NO` / `LOT NO` heading claimed the last parseable block as the
   total** (4300188). The colon print has the same defect (3990360 and others), but (d) is this guard, so the V.3
   heading (`NAME AND ADDRESS OF ECONOMIC OPERATOR`) now counts as an award block in sectioned bodies.
4. **`\nCONTRACT NO` matched the 2010 print's own-line `Contract notice`** (IV.3.2), which dropped single-contract
   figures (4041078). Fixed: `count_heading` skips a marker that a letter continues.
5. **The drain plan** named one fetch too many and one round too few. Fixed above.
6. Comment dates were wrong (2009-11-28, 2010-02-25, the sample windows). Fixed.
7. One reparse job per package re-stamps the profile once per job. That is harmless (issue 244 measured it), but the
   plan now uses one job per round.

**Effect of the fixes on the sample.** 35 distinct colonless bodies changed value: 28 multi-award parts dropped, 7
single-contract figures gained by (4). 5 colon-era values changed: 2 gained (3602477, 3300270) and 3 multi-award parts dropped
(2902041, 3303486, 4002443). These reach older vintages only at their next re-parse.

