# 551 unit 4b, M1: the salted shadow walk of v2 (2026-10-17)

**Status: COMPLETE. The pre-draw header below was written at 2026-10-17T15:02Z, before the order was computed and before any
walked id was read.**

## Pre-draw record (fixed before drawing)

- **Salt:** the integer `0x551B4` (= 348596).
- **Order rule:** sort the population by `((id XOR 0x551B4) * 2654435761) mod 2^32`, ascending, ties broken by `id`
  ascending.
- **Population:** the failed drain's 15,352 corrected ids (project 2575, `/v1/changes` cursor 955146761 -> 955254530),
  file `scratchpad/v551/changed_tenders.json`, sha256 `930422d4bd2d6f36ba15558a61946c534501d60294f575bf3e3808d133a02d44`.
- **Skipped:** every one of the 279 labelled ids (the keys of `postdrain/read_result.json` `final`, sha256
  `048a62c1981b5c6271a34b471807a8fbc754c6a38225cb32c91d5bdd471a4306`). All 279 are in the population. A skipped id is not
  walked, logged or counted.
- **Rule:** `unit4b/rule_v2.py`, sha256 `39be6d1ebc0ed2828ce3dd2a10b4db33f98bdf83ec5c5a7bb2d5fe4401a305ce` (the port
  check's file, unchanged). It pins `unit3/merge_rule.py` `111115a7e6500d3af78eed18bc45abf6f2ee0272098222f06ffc05ba825ba899`
  and `scripts/elect.py` `b270740e3c8e14b9a44660d7ca959d67b3ef6f8fe77c55974a29b715fe1ece63`. The walk script refuses to
  run if rule_v2.py's sha256 differs. R10 from EUR 10,000,000.00 (amendment (a)); published lot number never TED-ITEM;
  G8b unchanged; every v2 fire is a v1 fire.
- **Per id, in order:** `facts(id, refresh=True)` (a fresh `/v1/tenders/{id}` and every version notice's
  `/v1/notices/{id}/content`, notices cached by id; one GET per 1.5 s; a 408 retried once).
  - `v1_fires(f)` false: logged as **dropped** with v1's reason, not counted.
  - else `v2_decision(f)`: logged as **ship**, or **refuse** with the first refusing check (and every refusing check).
  - An id the API does not serve after the port's own retry policy (FetchError), or an id where the port raises, is
    logged as **unread** / **port_error** with the error, not counted, and listed in this file.
- **Stop:** when 30 v2 ships have been collected.
- **Outputs:** `unit4b/walk.jsonl` (one line per walked id), this file, and `unit4b/M1-walk-30.json`.

### Where this walk departs from issue 551's text (the task's decided choices)

- The issue's M1 text names the salt `sha256('551-4b-fresh30' + id)`. This walk uses the salt and order rule the task
  fixed instead (above).
- The issue requires the salt, and rule_v2.py's sha256, to be recorded **in the issue** before the draw, and rule_v2.py
  to be committed. This step may not edit the issue or commit, so both are recorded here, before drawing. Neither is in
  the issue yet.
- The amendment "prefer the ids corrected in BOTH 2575 and 2586" is not applied: the task fixes the population as the
  15,352 ids, and the 2586 correction set is not in hand.
- S_eur comes from the rate table rebuilt from the public series (the port check's method; `/v1/sql` is off limits),
  not from a bounded `/v1/sql` read of the stored rate.

## Result

**COMPLETE.** Walk started 2026-10-17T15:04:04Z, ended 2026-10-17T15:10:05Z. 54 ids walked (order positions 0..53; 0 labelled ids skipped on the way), 238 GETs.

| outcome | ids |
|---|---|
| v2 ship | 30 |
| v2 refuse | 24 |
| dropped (v1 does not fire) | 0 |
| unread / port error (not counted) | 0 |
| **walked** | **54** |

- **v1-fire rate among walked ids:** 54 of 54 counted ids (100.0 %).
- **v2 ship rate among v1 fires:** 30 of 54 (55.6 %); refused 24 (44.4 %).
- **Projected v2 fire count (the task's formula, 15,352 x ships / v1-fires-walked):** **8,529**; 95 % interval 6,356–10,605 (Clopper-Pearson on 30/54). The exact interval for a walk that stops at the 30th ship (inverse sampling) is 6,356–10,507.
- Stratified (the 15,073 unlabelled ids at the walked rate 30/54, plus the 47 labelled ids the port ships): 8,421; 95 % 6,287–10,459. The 279 include the value-selected band, so they are not a uniform part of the population; this line keeps them at their known outcomes.
- The decision expected about 9,800 (95 % 7,550–11,830; plan for 6,000–10,000), about 47 ids walked and about 36 % refused.

### v2 ships at or above EUR 100 m (the band read needs them)

**None.** No walked v2 ship has S_eur at or above EUR 100,000,000.00, so the band set for M2 stays the judge's 15.

### v2 ships in [EUR 10 m, EUR 100 m) (R10 territory under amendment (a))

None.

### Notes for M2 and B5

- **Largest ship:** 307725 at EUR 8,892,084.70.
- **Walked ids at or above EUR 10 m, any outcome:** 262408 (refuse, R1, EUR 40,400,000.00, R10 fires).
- **Vouched ships** (a stated total within 1 % of S, at any size): 5 of 30: 7399356 (TED-VAL_ESTIMATED_TOTAL), 6094289 (TED-VAL_TOTAL), 6527544 (TED-VAL_TOTAL), 307725 (BT-1118-NoticeResult), 1042997 (BT-1118-NoticeResult). The rest are below EUR 10 m, where R10 does not apply.
- **Served head null today:** 10 of 30 ships (108534, 633332, 181177, 81906, 1091419, 174041, 855538, 936207, 307725, 1042997).
- **S over today's served head** (same currency, 20 ships): 1.22x (7399356) to 37.01x (236597); median 2.33x.
- **Published lot numbers of the 147 summed lots:** lot section 147.
- **B5 stop band** (0.5x–2x of this projection): 4,264–17,058 corrections.

## The 30 shipped ids

| # | id | source | S | currency | n | S_eur | vouched by | served head today |
|---|---|---|---|---|---|---|---|---|
| 1 | 108534 | ted | EUR 439,877.57 | EUR | 2 | EUR 439,877.57 | - | null |
| 2 | 633332 | ted | EUR 687,163.82 | EUR | 3 | EUR 687,163.82 | - | null |
| 3 | 181177 | ted | EUR 223,090.00 | EUR | 2 | EUR 223,090.00 | - | null |
| 4 | 36928 | ted | EUR 144,208.08 | EUR | 5 | EUR 144,208.08 | - | EUR 14,984.08 |
| 5 | 193494 | ted | PLN 1,252,284.94 | PLN | 8 | EUR 298,141.79 | - | PLN 268,800.00 |
| 6 | 7399356 | ted | EUR 403,012.50 | EUR | 2 | EUR 403,012.50 | TED-VAL_ESTIMATED_TOTAL (22405135) | EUR 330,412.50 |
| 7 | 6787755 | ted | EUR 674,347.74 | EUR | 8 | EUR 674,347.74 | - | EUR 291,410.00 |
| 8 | 6094289 | ted | HUF 139,974,900.00 | HUF | 5 | EUR 406,313.21 | TED-VAL_TOTAL (19961452) | HUF 63,677,200.00 |
| 9 | 81906 | ted | EUR 1,050,000.00 | EUR | 2 | EUR 1,050,000.00 | - | null |
| 10 | 1091419 | ted | EUR 499,497.55 | EUR | 3 | EUR 499,497.55 | - | null |
| 11 | 6942121 | ted | SEK 951,900.00 | SEK | 2 | EUR 93,460.97 | - | SEK 492,000.00 |
| 12 | 6527544 | ted | EUR 2,534,623.69 | EUR | 12 | EUR 2,534,623.69 | TED-VAL_TOTAL (20738524) | EUR 699,128.76 |
| 13 | 174041 | ted | EUR 2,015,144.00 | EUR | 3 | EUR 2,015,144.00 | - | null |
| 14 | 5658786 | ted | EUR 24,400.00 | EUR | 4 | EUR 24,400.00 | - | EUR 13,200.00 |
| 15 | 855538 | ted | EUR 7,278,412.00 | EUR | 20 | EUR 7,278,412.00 | - | null |
| 16 | 1163033 | ted | EUR 778,184.80 | EUR | 5 | EUR 778,184.80 | - | EUR 24,750.00 |
| 17 | 478682 | ted | EUR 53,626.52 | EUR | 3 | EUR 53,626.52 | - | EUR 11,131.00 |
| 18 | 936207 | ted | EUR 3,078,098.30 | EUR | 8 | EUR 3,078,098.30 | - | null |
| 19 | 6697762 | ted | CHF 41,955.26 | CHF | 2 | EUR 39,013.63 | - | CHF 23,149.36 |
| 20 | 480181 | ted | PLN 4,954,732.67 | PLN | 7 | EUR 1,187,814.99 | - | PLN 498,461.19 |
| 21 | 236597 | ted | PLN 1,749,349.31 | PLN | 11 | EUR 409,511.05 | - | PLN 47,271.00 |
| 22 | 6513295 | ted | EUR 52,486.88 | EUR | 2 | EUR 52,486.88 | - | EUR 28,467.88 |
| 23 | 469403 | ted | EUR 842,207.83 | EUR | 11 | EUR 842,207.83 | - | EUR 38,556.97 |
| 24 | 533536 | ted | EUR 64,250.00 | EUR | 3 | EUR 64,250.00 | - | EUR 21,700.00 |
| 25 | 321859 | ted | CHF 3,369,583.40 | CHF | 2 | EUR 3,688,248.03 | - | CHF 1,448,972.40 |
| 26 | 6321416 | ted | EUR 171,540.19 | EUR | 3 | EUR 171,540.19 | - | EUR 135,055.86 |
| 27 | 307725 | ted | EUR 8,892,084.70 | EUR | 2 | EUR 8,892,084.70 | BT-1118-NoticeResult (28044262) | null |
| 28 | 225020 | ted | PLN 1,072,422.02 | PLN | 2 | EUR 252,073.62 | - | PLN 678,129.02 |
| 29 | 6759797 | ted | CHF 1,561,450.00 | CHF | 2 | EUR 1,454,134.85 | - | CHF 849,767.00 |
| 30 | 1042997 | ted | EUR 523,926.06 | EUR | 3 | EUR 523,926.06 | BT-1118-NoticeResult (31275820) | null |

Ids: 108534, 633332, 181177, 36928, 193494, 7399356, 6787755, 6094289, 81906, 1091419, 6942121, 6527544, 174041, 5658786, 855538, 1163033, 478682, 936207, 6697762, 480181, 236597, 6513295, 469403, 533536, 321859, 6321416, 307725, 225020, 6759797, 1042997.

`M1-walk-30.json` holds each one's facts summary (summed lots with notice, published lot number and winners; stated totals; vouches; served head; flags; notices).

## Refusals

**First refusing check** (24 refused):

| check | ids |
|---|---|
| R3 | 9 |
| R1 | 4 |
| R2c | 4 |
| R2d | 2 |
| R5.1 | 1 |
| R5.13 | 1 |
| R6 | 1 |
| R7 | 1 |
| R8 | 1 |

**Every refusing check** (an id counts once per check that refuses it):

| check | ids |
|---|---|
| R3 | 14 |
| R1 | 4 |
| R2c | 4 |
| R5.5 | 3 |
| R5.6 | 3 |
| R6 | 3 |
| R7 | 3 |
| R8 | 3 |
| R2d | 2 |
| R5.11 | 2 |
| R5.12 | 2 |
| R5.7 | 2 |
| R10 | 1 |
| R5.1 | 1 |
| R5.13 | 1 |
| R5.3 | 1 |
| R5.4 | 1 |
| R5.8 | 1 |
| R5.9 | 1 |

## Flags

- **`scale_rules_head_only`** (a figure of EUR 1 bn or more; ScalePartners reads the head only): none.
- **Within 2 % of EUR 10 m or EUR 100 m:** none.
- **Served head differs from the port's max election (fails closed as W/G0-served):** none.
- **Pins walked** (not labelled, so walked as any id): none.
- **Unread / port errors:** none.

## Every walked id, in salted order

`order_pos` is the position in the full salted order of the 15,352 (labelled ids included); labelled ids were skipped, not logged. The check column gives the first refusing check, then the others in brackets; for a dropped id it gives v1's reason.

| # | order_pos | id | outcome | check | S | n | S_eur | notices |
|---|---|---|---|---|---|---|---|---|
| 1 | 0 | 108534 | ship |  | EUR 439,877.57 | 2 | EUR 439,877.57 | 1 |
| 2 | 1 | 975665 | refuse | R3 (R6) | EUR 426,345.66 | 2 | EUR 426,345.66 | 3 |
| 3 | 2 | 145151 | refuse | R3 | PLN 14,795,619.00 | 3 | EUR 3,408,893.16 | 1 |
| 4 | 3 | 633332 | ship |  | EUR 687,163.82 | 3 | EUR 687,163.82 | 1 |
| 5 | 4 | 181177 | ship |  | EUR 223,090.00 | 2 | EUR 223,090.00 | 4 |
| 6 | 5 | 36928 | ship |  | EUR 144,208.08 | 5 | EUR 144,208.08 | 3 |
| 7 | 6 | 6674244 | refuse | R3 | HUF 154,505,140.00 | 15 | EUR 434,589.17 | 4 |
| 8 | 7 | 193494 | ship |  | PLN 1,252,284.94 | 8 | EUR 298,141.79 | 3 |
| 9 | 8 | 930442 | refuse | R1 (R3, R5.5) | CZK 3,260,246.08 | 2 | EUR 134,067.20 | 1 |
| 10 | 9 | 6656947 | refuse | R3 (R8) | PLN 17,393,200.84 | 2 | EUR 3,896,413.64 | 3 |
| 11 | 10 | 13292 | refuse | R3 (R6) | EUR 4,142,243.92 | 2 | EUR 4,142,243.92 | 2 |
| 12 | 11 | 7495977 | refuse | R2d | EUR 599,310.00 | 4 | EUR 599,310.00 | 5 |
| 13 | 12 | 7399356 | ship |  | EUR 403,012.50 | 2 | EUR 403,012.50 | 4 |
| 14 | 13 | 883025 | refuse | R3 | RON 2,283,650.40 | 3 | EUR 434,773.99 | 9 |
| 15 | 14 | 6787755 | ship |  | EUR 674,347.74 | 8 | EUR 674,347.74 | 2 |
| 16 | 15 | 6094289 | ship |  | HUF 139,974,900.00 | 5 | EUR 406,313.21 | 7 |
| 17 | 16 | 81906 | ship |  | EUR 1,050,000.00 | 2 | EUR 1,050,000.00 | 2 |
| 18 | 17 | 561571 | refuse | R7 | EUR 1,511,791.63 | 3 | EUR 1,511,791.63 | 8 |
| 19 | 18 | 7298633 | refuse | R2c (R7) | PLN 3,583,172.35 | 8 | EUR 781,413.66 | 3 |
| 20 | 19 | 1091419 | ship |  | EUR 499,497.55 | 3 | EUR 499,497.55 | 3 |
| 21 | 20 | 5917025 | refuse | R2c (R3) | EUR 63,500.00 | 3 | EUR 63,500.00 | 3 |
| 22 | 21 | 6942121 | ship |  | SEK 951,900.00 | 2 | EUR 93,460.97 | 2 |
| 23 | 22 | 6527544 | ship |  | EUR 2,534,623.69 | 12 | EUR 2,534,623.69 | 8 |
| 24 | 23 | 174041 | ship |  | EUR 2,015,144.00 | 3 | EUR 2,015,144.00 | 2 |
| 25 | 24 | 6623740 | refuse | R5.13 | SEK 895.00 | 2 | EUR 84.21 | 2 |
| 26 | 25 | 7276090 | refuse | R2c (R7) | HUF 122,147,486.00 | 16 | EUR 328,371.11 | 8 |
| 27 | 26 | 5658786 | ship |  | EUR 24,400.00 | 4 | EUR 24,400.00 | 1 |
| 28 | 27 | 855538 | ship |  | EUR 7,278,412.00 | 20 | EUR 7,278,412.00 | 1 |
| 29 | 28 | 1163033 | ship |  | EUR 778,184.80 | 5 | EUR 778,184.80 | 3 |
| 30 | 29 | 551343 | refuse | R1 (R3, R5.3, R5.4, R5.5, R5.6, R5.7, R5.8, R5.9, R5.12) | EUR 2,400,000.00 | 2 | EUR 2,400,000.00 | 3 |
| 31 | 30 | 262408 | refuse | R1 (R3, R5.5, R5.6, R5.7, R5.11, R10) | EUR 40,400,000.00 | 3 | EUR 40,400,000.00 | 4 |
| 32 | 31 | 6983647 | refuse | R3 | EUR 471,837.35 | 4 | EUR 471,837.35 | 3 |
| 33 | 32 | 5970462 | refuse | R3 (R8) | GBP 213,344.76 | 3 | EUR 240,551.09 | 2 |
| 34 | 33 | 871648 | refuse | R1 (R3, R5.6, R5.11, R5.12) | EUR 190,000.00 | 3 | EUR 190,000.00 | 1 |
| 35 | 34 | 5888594 | refuse | R2c | HUF 91,683,772.00 | 2 | EUR 283,490.84 | 6 |
| 36 | 35 | 7090888 | refuse | R2d | EUR 277,728.00 | 2 | EUR 277,728.00 | 3 |
| 37 | 36 | 478682 | ship |  | EUR 53,626.52 | 3 | EUR 53,626.52 | 4 |
| 38 | 37 | 936207 | ship |  | EUR 3,078,098.30 | 8 | EUR 3,078,098.30 | 2 |
| 39 | 38 | 6697762 | ship |  | CHF 41,955.26 | 2 | EUR 39,013.63 | 2 |
| 40 | 39 | 480181 | ship |  | PLN 4,954,732.67 | 7 | EUR 1,187,814.99 | 3 |
| 41 | 40 | 236597 | ship |  | PLN 1,749,349.31 | 11 | EUR 409,511.05 | 12 |
| 42 | 41 | 6513295 | ship |  | EUR 52,486.88 | 2 | EUR 52,486.88 | 2 |
| 43 | 42 | 1003480 | refuse | R3 | EUR 819,250.00 | 2 | EUR 819,250.00 | 2 |
| 44 | 43 | 6117990 | refuse | R8 | EUR 129,324.81 | 2 | EUR 129,324.81 | 2 |
| 45 | 44 | 6151596 | refuse | R5.1 | EUR 4,627,523.90 | 6 | EUR 4,627,523.90 | 2 |
| 46 | 45 | 469403 | ship |  | EUR 842,207.83 | 11 | EUR 842,207.83 | 2 |
| 47 | 46 | 533536 | ship |  | EUR 64,250.00 | 3 | EUR 64,250.00 | 7 |
| 48 | 47 | 88290 | refuse | R6 | HUF 41,013,000.00 | 2 | EUR 114,035.87 | 3 |
| 49 | 48 | 321859 | ship |  | CHF 3,369,583.40 | 2 | EUR 3,688,248.03 | 3 |
| 50 | 49 | 6321416 | ship |  | EUR 171,540.19 | 3 | EUR 171,540.19 | 2 |
| 51 | 50 | 307725 | ship |  | EUR 8,892,084.70 | 2 | EUR 8,892,084.70 | 4 |
| 52 | 51 | 225020 | ship |  | PLN 1,072,422.02 | 2 | EUR 252,073.62 | 4 |
| 53 | 52 | 6759797 | ship |  | CHF 1,561,450.00 | 2 | EUR 1,454,134.85 | 3 |
| 54 | 53 | 1042997 | ship |  | EUR 523,926.06 | 3 | EUR 523,926.06 | 4 |

Labelled ids skipped between order positions 0 and 53: none.

## Files

- `unit4b/walk.py` (this walk and report), sha256 `774fe39902cb1ec1ab112074a0a825ddf5f5db25308a3de8d91149769b5d95ac`.
- `unit4b/walk.jsonl` (one line per walked id), sha256 `761baee9c4a6abee8105f1e6507a7ca46313470f9076b22a73c7654b77bd505e`.
- `unit4b/M1-walk-30.json` (the 30 ships' facts summaries).
- Full facts of every walked id: `/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/m551/walk_facts/{id}.json` (scratch).
- Reproduce: `python3 -I -B walk.py --report` rewrites this results part from walk.jsonl.
