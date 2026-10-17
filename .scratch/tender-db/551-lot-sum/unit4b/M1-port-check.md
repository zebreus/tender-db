# 551 unit 4b, M1: the port check of `rule_v2.py` on the 279 labels (2026-10-17)

**Result: PASS. The port reproduces every outcome the judge stated, and no adopted amendment changes an outcome.**

| assert (task / decision) | expected | port | |
|---|---|---|---|
| wrongs refused | 105 / 105 | 105 / 105 | pass |
| EUR 100 m band (229) ships | the judge's 15 | the same 15 | pass |
| uniform 50 ships | 32 | 32, all labelled right | pass |
| out of sample ships | {294968, 467176, 585917, 5995030, 8426318} | the same set | pass |
| v1 fires on every labelled and out-of-sample id | 285 | 285 | pass |
| served head (path W off) = the port's max election (the G0 reference) | 285 | 285 | pass |
| rebuilt S, currency, n = the evidence's rebuilt sum | 279 | 279 | pass |
| decision-level differences from the judge's v2 (j7.py) and from the amended strict table (adv/a2.py) | 0 | 0 | pass |

All of these are IN-SAMPLE: the checks were chosen on these 279 labels, so a pass shows the port is faithful. It does
not measure v2. The fresh uniform 30 in M1's walk is the measure.

Inputs and outputs:
- `rule_v2.py`: sha256 `39be6d1ebc0ed2828ce3dd2a10b4db33f98bdf83ec5c5a7bb2d5fe4401a305ce`.
- `M1-port-check.json`: the row for each of the 285 ids. It holds the decision, the first refusing check, every
  refusing check, S / currency / n / S_eur, the head date and rate, vouched_by, flags, the judge's and the strict
  table's decision, and the differences at the sub-check level. sha256
  `8acb600b316416a5b9f172f4ae29c5e685a916d8a3a6143d50a9c49fe4cb9f30`.
- Run `python3 -I -B rule_v2.py --port-check` to reproduce. It reads the cache in the scratchpad, `m551/cache`. With
  an empty cache it re-reads the API at one GET per 1.5 s.

## What the port is

- **v1** is unit 3's `merge_rule.serve`, imported unchanged and sha256-pinned with `../scripts/elect.py`. The script
  refuses to run if either file changes.
  - It is fed the head version rebuilt from `/v1/tenders/{id}`: tender-scope and lot amounts with the `withheld`
    quality, `lot_details` kinds, and `lot_results` rounds by notice id.
  - It reports the first refusing guard among T', K, G0, G5, G5', G6, G7, G8, G8b, G1', G2, G3 and G4.
  - M1 adds one fail-closed check, **W/G0-served**: an id whose served head is not the port's max election is
    dropped. 0 of the 285 ids were dropped this way.
- **v2** is v1 plus refusals only. `v2_decision` ships only where `v1_fires`, so every v2 fire is a v1 fire. G8b is
  untouched and nothing is re-keyed.
  - The refusals are R1, R2a–d, R3, R4, R5.1–R5.15 (numbered as the rule lists the shape checks), R6, R7, R8, R9 and
    R10, in that order after G4.
  - R10 applies from **EUR 10,000,000.00**, per amendment (a).
- **The field-to-kind table is frozen** in the header comment of `rule_v2.py`, and B1 must carry the same table:
  - R3 award totals: BT-161, BT-118 and BT-1118 NoticeResult, plus TED-VAL_TOTAL in the Notice section of a CAN-type
    form.
  - CN-type TED forms (old-vintage numeric codes and F-codes alike) are 1 2 4 5 7 8 9 10 11 12 16 17 24. Their
    VAL_TOTAL counts as an estimate.
  - R10 vouches come from the rule's list exactly: BT-161, TED VAL_TOTAL, BT-118, BT-1118, BT-27-Procedure, TED
    VAL_ESTIMATED_TOTAL, and TED VAL_RANGE_TOTAL LOW/HIGH.
  - Never read: TED-VALUE, TED-VALUE_COST, BT-271-Procedure, the estimate ranges, and every field in a Modification
    section.
  - A correction notice (with a `Change` section and a logical id) that states a total replaces the totals of the
    earlier notices with the same logical id, as rounds do.
  - R4 and R5 read API-keyed lot estimates only.
  - A published lot number comes from LOT_NO / LOT_NUMBER / BT-137 in the lot section, then the award block's LOT_NO,
    then BT-13713, then the FTS OCDS id. It is **never TED-ITEM** and never a title.
  - R9 counts only bids with a resolved lot key.
- **Every version notice is read.** That is every notice a version, lot result, bid or contract names: 3,106 notices
  for the 285 ids, and up to 919 for a single Tender (8259278). There is no bounded-chain shortcut.
- **EUR uses `RatesLookup` at the head's date**: the UTC civil date of `published_at`, as `civil_date` computes it.
  - The rate table is rebuilt from the public series that the production `currency_rates` table is loaded from:
    ECB eurofxref-hist from 1999, the Eurostat ECU series before 1999, and the irrevocable euro rates. The lookup is
    the nearest row on or before the date, within 7 days for daily rows.
  - `/v1/sql` was not used.
  - Check: on 21 non-EUR heads, `round(H / rate)` matches the **stored** head EUR value to the cent (21 of 21 exact).
    The bracket probe is `/v1/lots?tender=X&min_value=E` returning lots and `min_value=E+1` returning none.
  - The 21 heads include 1062828 (CHF, near EUR 100 m), 8426318, 5704713 (published on a Saturday), and all the
    non-EUR ships with a non-null head.

## How the 279 were read (the "cached features where identical to a fresh read" clause)

- **Tender details:** all 285 were read fresh from `/v1/tenders/{id}` between 13:59 and 14:07 UTC.
  - Each was compared with the detail in the evidence that the scorer used, with `value` and `value_derivation` set
    aside (unit 4a moved those on purpose).
  - **285 of 285 are identical.**
- **Notices:** 1,333 of the 3,106 notices came from the evidence's earlier copies. The other 1,773 were read fresh.
  - The fresh 1,773 are exactly the notices that the scorer's bounded chains skipped: 327861, 489596, 511903, 531641,
    794186, 5571139, 8259278 and 8401934.
  - A seeded random 25 of the copies were re-read: **25 of 25 identical.**
- **Facts:** every fact was recomputed from these raw reads by the port's own `facts()`. The evidence's
  `features_all.json` was not used as input.
  - The recomputed facts were then compared with the evidence's cached facts, sub-check by sub-check, on all 279.

| sub-check (port vs evidence) | differing ids |
|---|---|
| R1 (Bm_g) | 1: 511903 |
| R2a, R2b, R2c, R2d, R3, R4, R5.1–R5.15, R6, R7, R8 | 0 |
| R9 (no_own_bid) | 1: 8597433 |
| R10 vouch found or not | 0 |

**Both differences come from one evidence artifact and change no outcome.** The evidence's extractor kept only the
first 60 bids of each lot (`pxscripts/extract.py`, `'bids': bids_by_lot.get(key, [])[:60]`). The port reads every bid.
- **8597433** (band, wrong, double_count): lot `1` has 186 bids. The first 60 are all from notice 46849342, so the
  summed round 47159234 looked as if it published no bid for lot `1`, and R9 fired.
  - With every bid read, R9 does not fire. The id is still refused, by R1, R5.3, R5.4, R8 and R10.
  - The decision's line "R9 refuses 27915 and 8597433" holds only for 27915. On a full read, R9 refuses 27915 and 0
    rights in-sample.
- **511903** (band, wrong, double_count): LOT-0001 has 151 bids, and none of the first 60 is from the summed notice
  46964667. With every bid read, one LotTender in that notice names all three winners, a consortium, so R1 passes.
  - The id is still refused by R10: CZK 4,247,807,917.00, no stated total, S_eur EUR 173,670,547.32.
  - It becomes a wrong that only one check refuses.

The port also departs from the scorer by design. Each change follows the rule text or an adopted amendment, and
**each changes 0 in-sample outcomes or sub-checks**:
- R1 follows the rule text: TED AWARDED_TO_GROUP counts as one tender with or without LotTender rows. The scorer
  honoured the flag only when there were no LotTender rows.
- R2 drops the title fallback and never uses TED-ITEM (amendment). It adds BT-13713 and the FTS OCDS id as sources.
- R3 and R10 use the frozen table: TED-VALUE, VALUE_COST and BT-271-Procedure are dropped, numeric CN forms count as
  estimates, and corrections replace totals.
- R4 and R5 use API-keyed lot estimates only.
- The summed set is v1's own LotSums, with elect.py's sentinel. It equals the evidence's rebuilt set on all 279.

**The judge's reference was reproduced** from the evidence copies (`m551/ev`, tarball sha256 verified against
`SHA256SUMS`) with the paths re-pointed away from the session's `d551w`:
- j7.py's v2 gives band 15 / sample 32 / 105 wrongs refused.
- adv/a2.py's strict table (totals + lotno + est) changes 0 decisions.

## Results on the 279

| group | verdict | refused | shipped |
|---|---|---|---|
| band 229 | wrong 96 | 96 | 0 |
| band 229 | right 120 | 106 | 14 |
| band 229 | neutral 13 | 12 | 1 |
| uniform 50 | wrong 9 | 9 | 0 |
| uniform 50 | right 40 | 8 | 32 |
| uniform 50 | neutral 1 | 1 | 0 |

**First refusing check, all 279:**

| check | ids |
|---|---|
| R1 | 71 |
| R10 | 63 |
| R3 | 38 |
| R2c | 16 |
| R2d | 12 |
| R2a | 10 |
| R8 | 9 |
| R6 | 3 |
| R5.1 | 2 |
| R2b | 2 |
| R9 | 1 |
| R5.13 | 1 |
| R5.10 | 1 |
| R5.5 | 1 |
| R7 | 1 |
| R5.3 | 1 |

**First refusing check, the 105 wrongs:**

| check | wrongs |
|---|---|
| R1 | 46 |
| R3 | 20 |
| R2a | 10 |
| R2d | 6 |
| R2c | 5 |
| R8 | 4 |
| R6 | 3 |
| R10 | 2 |
| R5.1 | 2 |
| R9 | 1 |
| R5.13 | 1 |
| R5.10 | 1 |
| R5.5 | 1 |
| R7 | 1 |
| R2b | 1 |
| R5.3 | 1 |

**Wrongs refused by one check only:**

| id | group | class | refused by |
|---|---|---|---|
| 51385 | mid | slip | R10 |
| 324582 | sample | not_additive | R5.13 |
| 511903 | mid | double_count | R10 (R1 too in the scorer, see above) |
| 6421819 | sample | farther | R2d |
| 6972400 | mid | slip | R8 |

6421819 is refused only because TED-ITEM is never a lot number (amendment). Its LOT-1 notice 21205884 has ITEM 1 and
no LOT_NO.

The uniform 50's wrongs: 249185 is refused by R1; 324582 by R5.13; 831972, 5779166 and 6330626 by R3; 5976305 and
7056700 by R2a; 6421819 by R2d; 6545090 by R2c.

### The band ships (exactly the judge's 15)

Every S_eur is at or above EUR 100 m at the exact head-date rate. No band id falls below EUR 100 m and no sample id
rises above it, so the label groups and the computed band agree.

| id | label | S | lots | S_eur | vouched by |
|---|---|---|---|---|---|
| 683951 | right | EUR 807,552,606.00 | 149 | EUR 807,552,606.00 | BT-1118-NoticeResult of notice 24772240: EUR 807,552,606.00 |
| 707424 | right | HUF 47,686,558,000.00 | 3 | EUR 131,259,449.49 | BT-118-NoticeResult of notice 47360663: HUF 47,686,558,000.00 |
| 726354 | right | EUR 106,287,074.72 | 3 | EUR 106,287,074.72 | BT-118-NoticeResult of notice 26858945: EUR 106,287,074.73 |
| 1102571 | right | EUR 225,000,000.00 | 2 | EUR 225,000,000.00 | BT-1118-NoticeResult of notice 25848303: EUR 225,000,000.00 |
| 1147190 | right | EUR 199,287,951.00 | 2 | EUR 199,287,951.00 | BT-1118-NoticeResult of notice 25033715: EUR 199,287,951.00 |
| 1148297 | right | HUF 83,827,102,943.40 | 10 | EUR 231,259,939.70 | BT-118-NoticeResult of notice 25724682: HUF 83,827,102,943.40 |
| 5278856 | neutral | EUR 548,000,000.00 | 2 | EUR 548,000,000.00 | TED-VAL_TOTAL of notice 19045609: EUR 548,000,000.00 |
| 5842187 | right | EUR 101,031,420.60 | 3 | EUR 101,031,420.60 | TED-VAL_TOTAL of notice 19548423: EUR 101,031,420.60 |
| 6017838 | right | GBP 93,658,000.00 | 6 | EUR 105,649,182.18 | TED-HIGH of notice 19744792: GBP 93,378,000.00 |
| 6057925 | right | EUR 230,000,000.00 | 3 | EUR 230,000,000.00 | TED-VAL_ESTIMATED_TOTAL of notice 19536156: EUR 230,000,000.00 |
| 6195903 | right | EUR 133,290,220.00 | 2 | EUR 133,290,220.00 | TED-VAL_TOTAL of notice 20027347: EUR 133,290,220.00 |
| 6370901 | right | HUF 113,611,196,000.00 | 2 | EUR 317,900,263.03 | TED-VAL_TOTAL of notice 20317379: HUF 113,611,196,000.00 |
| 7001924 | right | EUR 424,000,000.00 | 3 | EUR 424,000,000.00 | TED-VAL_ESTIMATED_TOTAL of notice 21525333: EUR 424,000,000.00 |
| 7011414 | right | PLN 1,144,774,260.00 | 2 | EUR 250,212,943.70 | TED-VAL_TOTAL of notice 21543078: PLN 1,144,774,260.00 |
| 8426318 | right | GBP 3,000,000,000.00 | 7 | EUR 3,382,949,932.34 | TED-HIGH of notice 20083570: GBP 3,000,000,000.00 |

### The uniform 50: 32 ships, all labelled right

Ships: 37368, 50454, 90837, 107998, 124839, 129259, 140308, 204547, 207243, 327515, 406646, 493668, 509321, 521716,
613940, 661269, 780184, 1043115, 5565555, 5698996, 5704713, 5944534, 6241092, 6444915, 6483572, 6512557, 6862672,
7269740, 7448359, 7647263, 7700151 and 8697820.

**Amendment (a) changes nothing here.** The only one at or above EUR 10 m is 7647263 (EUR 12,043,056.71), and
TED-VAL_TOTAL of notice 23229224 vouches for it to the cent.

The uniform 50's 8 refused rights are lost fixes:

| id | refused by |
|---|---|
| 312399 | R8 |
| 467379 | R3 |
| 5740501 | R3 |
| 6378980 | R8 |
| 6558417 | R2c |
| 6595330 | R2c |
| 7177766 | R2c |
| 7450228 | R2c |

The neutral 401929 is refused by R3.

### Out of sample and the pins

| id | v2 | first refusing check | S | as the decision expects |
|---|---|---|---|---|
| 585917 | ship | - | EUR 250,000.00, vouched by BT-118 of notice 25416405 | yes |
| 5995030 | ship | - | PLN 1,549,511.60, 35 lots; TED-VAL_TOTAL PLN 1,546,390.00 vouches (0.2 % off) | yes |
| 8426318 | ship | - | GBP 3,000,000,000.00, vouched by TED-HIGH | yes |
| 467176 | ship | - | CHF 3,161,818.00 | yes |
| 294968 | ship | - | CHF 4,440,992.42 | yes |
| 7256284 | refuse | R2d | EUR 574,873.28 | yes (R2/R7) |
| 5977060 | refuse | R2d (R7 too) | EUR 4,143,243.00 | yes (R2/R7) |
| 6450467/69/73 | refuse | R10 | EUR 3,574,764,979.28, no stated total | yes (keep EUR 1,660,661,682.94) |
| 8831876 | refuse | R1 (R10 too) | GBP 25,999,999,986.00 | yes |

## v1 on API facts beyond the labels (parity)

The adaptation was also checked against unit 3's own input, so that its refusals are faithful too. That matters for
the walk, where an id that v1 does not fire on is dropped.
- `merge_rule.serve` ran on unit 1's **stored** rows (`unit3/data/ctx.json.gz`, rates from the stored `eur_cents`).
- `rule_v2.v1_eval` ran on today's `/v1/tenders/{id}`.
- The ids were 59 of the 4,745: 9 pins plus a seeded random 50 (seed 5514), all outside the 15,352 walk ids except the
  pins.
- **59 of 59 agree** on the reason and on the derived S. The outcomes were W 6, T' 38, G0 7, K 3, G1' 3, G5 1 and G3
  1.
- Script: `m551/v1parity.py` in the scratchpad; output: `m551/v1parity.json`.

## Flags, and what the walk inherits

- **`scale_rules_head_only`** (14 ids with a figure of EUR 1 bn or more): 116385, 176280, 524007, 982370, 993254,
  1074798, the Lyon-Turin trio, 8386982, 8426318, 8676784, 8822688 and 8831876.
  - The API serves the head version only, so ScalePartners reads only the head (elect.py SIMPLIFICATION 4). Every
    scale rule is gated at EUR 1 bn, so nothing below it can change.
  - On the labels, v1 fires on all 14, as it did in production.
  - A walked id with this flag should be read with that caveat in mind.
- **Within 2 % of EUR 100 m** (band membership for M2's read; S_eur at the exact rate):

  | id | S_eur | v2 |
  |---|---|---|
  | 240143 | EUR 100,269,573.24 | refused |
  | 360576 | EUR 100,904,315.14 | refused |
  | 742200 | EUR 100,863,323.30 | refused |
  | 820038 | EUR 101,847,373.43 | refused |
  | 1062828 | EUR 100,474,491.15 | refused |
  | 5842187 | EUR 101,031,420.60 | **ships** |
  | 7139761 | EUR 101,147,983.94 | refused |
  | 7388206 | EUR 100,900,600.40 | refused |
  | 7606577 | EUR 100,785,875.07 | refused |
  | 8648668 | EUR 101,743,045.90 | refused |

  - 1062828 (CHF 93,401,086.97 at 0.9296) is in the band. The probe checked its rate against the stored value.
- **Within 2 % of EUR 10 m** (R10's threshold): none of the 285.
- `facts(id, refresh=True)` is the walk's call, so each walked Tender is a fresh read. Notices are cached by id under
  `m551/cache` in the scratchpad.

## Frozen inputs (sha256)

| file | sha256 |
|---|---|
| `unit4b/rule_v2.py` | `39be6d1ebc0ed2828ce3dd2a10b4db33f98bdf83ec5c5a7bb2d5fe4401a305ce` |
| `unit3/merge_rule.py` (pinned inside rule_v2.py) | `111115a7e6500d3af78eed18bc45abf6f2ee0272098222f06ffc05ba825ba899` |
| `scripts/elect.py` (pinned inside rule_v2.py) | `b270740e3c8e14b9a44660d7ca959d67b3ef6f8fe77c55974a29b715fe1ece63` |
| `postdrain/read_result.json` (labels) | `048a62c1981b5c6271a34b471807a8fbc754c6a38225cb32c91d5bdd471a4306` |
| `unit4b/evidence.tar.gz` (matches SHA256SUMS) | `10bdea545944d1d27c634158f3a35946d33d0299e75a2604fcde3d67e0b3eeb5` |
| rates: `m551/rates/eurofxref-hist.zip` (ECB, newest row 2026-10-16) | `8e925cef1572f8df5c8743229a761b029180f3b5cd4aa2213592192964ebd793` |
| rates: `m551/rates/ecu-ert_h_eur_d.csv` (Eurostat, 1993–1998) | `47e941270ac4a0f0f22416bd4b41c9b427d43c3a4f430996f8457dca8f134e5c` |
| rates: `m551/rates/ecu-ert_bil_eur_d.csv` (Eurostat, 1993–1998) | `48e2c9f17ae7219b2b145a66ace399aba7c306c88721b7ce8f7c76f4758e131b` |

## Not done here: the rest of M1

The salted walk has not started, and no walk id was scored. The amendments require two things before the first
walked id, and both belong in issue 551, which this step may not edit:
1. Record the salt `sha256('551-4b-fresh30' + id)` in the issue.
2. Record this `rule_v2.py` sha256 next to it, and commit `rule_v2.py`.

Any edit to `rule_v2.py` after the first walked id voids the draw.
