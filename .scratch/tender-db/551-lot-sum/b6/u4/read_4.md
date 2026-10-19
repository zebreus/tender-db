# 551 B6 U4 guard-cost read, part 4: R_single ranks 21–26

2026-10-19, reader `read_4`. These are ranks 21–26 of R_single's read set, in `sha256('551-b6-gcost-'+id)` ascending
order (`read_set.json`, sha256 d77efbc6…): 7102361 (a5e6d8cc…), 6003114 (ad4e6954…), 7179291 (af0aef5f…), 402503
(b787f66c…), 7394272 (bd6e0e45…), 6852228 (bd73656f…). None of the six has a verdict from an earlier recorded read.
read18, the B5 61, the post-drain 279, M1/M2/fallback and the unit-2 panel were searched by id, and all six are new
reads. **7179291 and 7394272 are also among G-cover's 12 R_single-overlap ids**, so their verdicts also feed the G-cover
gap case.

**The question for each id:** G-single refuses this v2 ship (the summed lot awards S, served as the tender's value and
vouched by a stated total). Under M2, is serving S RIGHT, NEUTRAL or WRONG as the tender's value?

**Rubric (M2, as `read_1.md` states it):** right/ok means S is the procedure's value. That requires distinct lots
(`LOT-k` checked against the printed LOT_NO), each lot's latest award, no lot counted twice, no ×10ᵏ slip and no VAT
mix, and S must be no farther from the procedure's stated total than the head it replaces. The wrong classes are
`not_additive`, `double_count`, `farther`, `slip` and `other`. `other` includes a vouching total that belongs to
something else, as in 6465510. Neutral means S is not wrong but also not clearly better than the head without W.

**Inputs:** `b6/u2/R_single.json` c1b413e0…, `b6/u2/R_excess.json` 3d134390…, `b6/u2/decisions.jsonl.gz` 0add92e3…
(the same hashes `read_set.json` records). The port-predicted S, n, summed keys and notices, the v2 `vouched_by` and the
refusing guard come from `decisions.jsonl.gz`. Live heads come from `GET /v1/tenders/{id}`. All six live heads are
JSON-equal to their U2 fresh-layer copies (`scratchpad/b6/tenders`, fetched 06:24Z–07:30Z), so the port's facts are
still current. All 26 chain notices fetched live are JSON-equal to their U2-layer copies. Two objects outside the six
chains were fetched live only: twin tender 6896664 and its CAN 21777064, for 402503.

## Result

**5 right / 0 neutral / 1 wrong.** The wrong one is **7179291, wrong / other**: its vouching total is a different
call-off contract's own award, the 6465510 shape. All six are refused by G-single. 7179291 and 7394272 are also refused
by G-cover. For G-single's read set these six add **1 non-right**. For the G-cover gap case they give 7179291 wrong and
7394272 right.

| rank | id | S (port) | n | summed: key ← notice, award | v2 vouched_by (all removed by G) | refusing guard | head without W (live) | buyer-stated total(s) | S/T | head/S | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 21 | 7102361 | EUR 1,000,000.00 | 2 | LOT-1 ← 22066668 450,000.00; LOT-2 ← 22048453 550,000.00 | TED-VAL_TOTAL EUR 1,000,000 on 22048453 (F03, 1 result: LOT_NO 2) | G-single | EUR 450,000.00 | 22048453 procedure VAL_TOTAL 1,000,000 | 0.00 % | 45.0 % | **right / ok** |
| 22 | 6003114 | PLN 38,200.00 | 3 | LOT-1 ← 19982269 11,400; LOT-2 ← 19973462 12,800; LOT-3 ← 19973463 14,000 | TED-VAL_TOTAL PLN 38,200 on 19427538 (F15) and 19437670 (F03), each with 1 result (no LOT_NO; stored LOT-1) | G-single | PLN 11,400.00 | procedure VAL_TOTAL 38,200 (net) on 19427538 and 19437670 | 0.00 % | 29.8 % | **right / ok** |
| 23 | 7179291 | EUR 83,349.90 | 3 | LOT-1 ← 22364146 26,372.00; LOT-2 ← 22364146 28,077.90; LOT-3 ← 22315443 28,900.00 | TED-VAL_TOTAL EUR 83,146.73 on 22780357 (F03, 1 result: LOT_NO "1,2", keyless, contract 35-372) | G-single **and G-cover** | EUR 83,146.73 | none equals S. Framework 22148426: 187,227.36; call-offs: 28,900.00 / 54,449.90 / 83,146.73 | +0.24 % (to a different contract) | 99.8 % | **wrong / other** |
| 24 | 402503 | HUF 35,298,888,888.00 | 2 | LOT-0001 ← 24982264 29,978,888,888; LOT-0002 ← 24416505 5,320,000,000 | BT-161 HUF 35,298,888,888 on 24416505 (can-modif, 1 result: LOT-0002) | G-single | HUF 29,978,888,888.00 | BT-161 on 24416505; twin CAN 21777064 VAL_TOTAL 35,298,888,888 | 0.00 % | 84.9 % | **right / ok** |
| 25 | 7394272 | EUR 3,692,965.50 | 5 | LOT-1 ← 23168277 201,187.50; LOT-2 ← 23169697 892,500.00; LOT-3 ← 23168435 253,557.00; LOT-4 ← 23168097 2,288,265.00; LOT-5 ← 23168536 57,456.00 | TED-VAL_TOTAL EUR 3,692,965.50 on 22388431 (F03, 1 result: LOT_NO "1,2,3,4,5", keyless) | G-single **and G-cover** | EUR 2,288,265.00 | 22388431 procedure VAL_TOTAL and its single RES-1, both 3,692,965.50 | 0.00 % | 62.0 % | **right / ok** |
| 26 | 6852228 | PLN 19,822,610.00 | 6 | LOT-1 ← 22470437 3,698,300; LOT-2 ← 23320304 5,999,000; LOT-3 ← 21584710 531,480; LOT-4 ← 22470439 735,500; LOT-5 ← 23318309 3,312,330; LOT-6 ← 23067905 5,546,000 | TED-VAL_TOTAL PLN 19,822,610 on 21600409 (F06, 1 result: LOT_NO 6) | G-single | PLN 3,312,330.00 | 21600409 procedure VAL_TOTAL 19,822,610 (cumulative); 21584710 14,276,610 = lots 1–5 | 0.00 % | 16.7 % | **right / ok** |

Under every guard set the port's S, currency and n equal v2's. The ship becomes an R10 refusal only where a guard
removes the vouch: {single} and {single, cover} for all six, and {cover} for 7179291 and 7394272. No live head serves a
derivation (`value_derivation: null`).

## Per head

### 7102361: right / ok (Région Bretagne, communication services framework; G-single)

- **Procedure:** an open procedure, DIRCOM-AOO-2021-005. The CN 21758156 sets up a framework agreement with subsequent
  contracts in 2 lots (12 months plus 3 tacit renewals), with no value stated. F03 22048453 (2021-11-19) awards LOT_NO 2
  ("services publics et marketing") to Notchup: contract 2021-90539, 4 tenders, VAL_TOTAL = VAL_ESTIMATED_TOTAL
  EUR 550,000. Its procedure **VAL_TOTAL is EUR 1,000,000**. F03 22066668 (2021-11-29) awards LOT_NO 1 ("politiques
  publiques et participation") to Scopic: contract 2021-90538, 3 tenders, 450,000. Its procedure VAL_TOTAL is its own
  450,000.
- **Why right:** two distinct lots, two framework contracts and two holders. Each lot has its own framework value, so
  this is not one envelope repeated across lots (`not_additive` does not apply). S = 450,000 + 550,000 is the buyer's
  stated procedure total. The head without W is lot 1 alone (45 %).

### 6003114: right / ok (Gmina Piła, works-supervision inspector for LED street lighting; G-single)

- **Procedure:** the open procedure of 4.7.2017 failed, and the contract went by negotiation without publication
  (art. 67(1)(4) Pzp), ref. 271.31.2017. The F15 VEAT 19427538 (2017-09-01) and the F03 19437670 (2017-09-09) each have
  three Lot sections (Części 1–3, with their streets) and **one** LotResult RES-1. That result prints no LOT_NO (ITEM 1)
  and goes to Budconsult (1 tender), with RES VAL_TOTAL **PLN 46,986.00**. The procedure **VAL_TOTAL is PLN 38,200.00**.
  46,986 = 38,200 × 1.23 (Polish 23 % VAT), so the result row is gross and the procedure total net. The store keeps
  RES-1 on LOT-1, which is why the port names LOT-1 for both notices and why G-cover keeps them.
- **The 2018 F20s** came in three rounds: 19628831/33/37 in January, 19787805 and 19795352/53 in April, and
  19982269 / 19973462 / 19973463 in August. All are term-only extensions with VAL_TOTAL_BEFORE = AFTER, and each states
  its part's own value in its LotResult: Część 1 11,400, Część 2 12,800, Część 3 14,000. These add to exactly 38,200.
  The served texts also print "38 200.00".
- **Why right:** three distinct parts, each value stated identically in three rounds, and the sum equals the buyer's
  net total to the grosz. The gross 46,986 row on LOT-1 is superseded by the 2018 LOT-1 rows and is not summed, so there
  is no VAT mix. The head without W is part 1 alone (30 %).

### 7179291: wrong / other (Lithuanian Fire and Rescue Department, fire-hose nozzles; G-single and G-cover)

- **Procedure:** an open procedure. The CN 21936672 (3 lots, no value) gives quantities "during the framework agreement
  term" (288 / 288 / 108 units over 24 months). The chain holds one framework award and three call-off awards. Every
  later notice is marked FRAMEWORK and refers to the framework award 2022/S 007-013596:
  - **22148426** (2022-01-11): RES-1 with LOT_NO "1, 2, 3", contract **35-1**, UAB "Saugus namas", 4 tenders,
    VAL_TOTAL **EUR 187,227.36**, the same as the procedure VAL_TOTAL. This is the framework agreement. It is stored
    lot = null and is not summed.
  - **22315443** (2022-04-29): contract **35-103**, LOT_NO 3, 28,900.00. Notice total 28,900.00.
  - **22364146** (2022-05-24): contract **35-165**, with LOT_NO 1 at 26,372.00 and LOT_NO 2 at 28,077.90. Notice total
    54,449.90.
  - **22780357** (2022-11-03): contract **35-372**, concluded 2022-09-30, one RES-1 with LOT_NO "1,2" at
    **83,146.73**, the same as the notice total. This is a second call-off for lots 1 and 2. It is stored lot = null.
- **S** = 26,372.00 + 28,077.90 + 28,900.00 = **EUR 83,349.90**, which is call-offs 35-165 and 35-103. **T** =
  83,146.73 is the vouching notice's own single result: call-off 35-372, a contract S does not contain. S lands 0.24 %
  from T only because two different call-off batches happen to be about the same size. No notice of this procedure
  states 83,349.90. The four notice totals are 187,227.36, 28,900.00, 54,449.90 and 83,146.73.
- **Truth:** the framework award is EUR 187,227.36, and the call-offs published so far add to 166,496.63. S covers
  44.5 % of the framework and 50.1 % of the call-offs. The head without W (83,146.73 = 35-372 alone) covers 44.4 %. So S
  is not farther than the head (by EUR 203), and it is not better either.
- **Why wrong / other:** the rubric's explicit wrong criterion applies. The vouching total belongs to something else:
  a one-result notice's own award of an object outside S, which is 6465510's shape. Served, the "lots: 3, vouched" label
  would claim a procedure total that no notice states. If the vouch criterion were discounted, the reading would be
  neutral (not farther, not better). Either way the head is **non-right**, so the adoption count does not depend on the
  class. Both guards refuse it for the right reason. G-single applies because 22780357 has one LotResult. G-cover applies
  because that result prints "1,2", is stored keyless and names no summed key.

### 402503: right / ok (Hungarian road bypass, 260. sz. Sajószentpéter–Berente; modification-only twin; G-single)

- **This tender's chain** holds three eForms contract-modification notices (can-modif, subtype 38). Each names the
  original award notice 275434-2021 in BT-1501(n):
  - 23627848 (2023-12) is LOT-0002 (STRABAG), deadline only (mod-cir), with BT-720 and BT-161 both 5,320,000,000.
  - **24416505** (2024-12) is LOT-0002, a value raise (mod-minv). BT-201 says "Módosítást megelőző szerződéses érték:
    5 320 000 000,-Ft Módosított szerződéses érték: 5.487.355.850,-Ft". BT-720 keeps the original 5,320,000,000, and
    **BT-161 is 35,298,888,888**.
  - 24982264 (2025-08) is LOT-0001 (Soltút + HE-D), data only (mod-cir: the contracting authority moved from NIF to
    ÉKM, plus a financing clause). BT-720 and BT-161 are both 29,978,888,888.
- **The original award** sits on twin tender **6896664** (`/v1/tenders?publication_id=275434-2021`). Its F03 CAN
  21777064 awards LOT_NO 1 HUF 29,978,888,888 (contract 1) and LOT_NO 2 HUF 5,320,000,000 (contract 2), each with
  4 tenders. Its procedure **VAL_TOTAL is HUF 35,298,888,888**, and S equals that to the forint.
- **Why right:** both lots are summed at their as-awarded values. Lot 2's later 5,487,355,850 is a modification kind
  and is not used, so pre- and post-modification values are not mixed. S equals the CAN's total and the buyer's BT-161.
  The head without W is lot 1 alone (84.9 %). This is the same shape as read_1's 472292.
- **Note (no verdict impact):** twin 6896664 is itself in the 3,502. It ships the **same S** HUF 35,298,888,888 under
  every guard set, vouched by its CAN's two-result VAL_TOTAL. If G-single is rejected, two Tenders serve this procedure's
  35.3 bn. Today, without W, both serve lot 1's 29.98 bn. The twin is older than path W. S_eur is about 89.2 m, which is
  outside the band and not within 2 % of EUR 100 m.

### 7394272: right / ok (Osakidetza, five exclusive-supply medicines; G-single and G-cover)

- **Procedure:** negotiation without publication, 2021/02452, Sanofi exclusive. F03 22388431 (2022/S 099-273772) has
  five Lot sections (LOT_NO 1 ALEMTUZUMAB, 2 ALGLUCOSIDASA ALFA, 3 PLERIXAFOR, 4 TERIFLUNOMIDA, 5 RASBURICASA, each priced
  "según precios unitarios"). It has **one** LotResult, RES-1, with LOT_NO "1,2,3,4,5", contract 1, 1 tender and
  VAL_TOTAL **EUR 3,692,965.50**, the same as the procedure VAL_TOTAL. That result is stored lot = null and is not
  summed.
- **The 2023-05-10 F20s** are a one-year extension that doubles each lot (VAL_TOTAL_AFTER = 2 × BEFORE). Each restates
  its lot's original award in its LotResult: 23168277 LOT_NO 1 201,187.50 (contract 1), 23169697 LOT_NO 2 892,500.00
  (2), 23168435 LOT_NO 3 253,557.00 (3), 23168097 LOT_NO 4 2,288,265.00 (4) and 23168536 LOT_NO 5 57,456.00 (5). These
  add to exactly **3,692,965.50**.
- **The earlier F20s** (22761239, 22864816, 22956107, 22963790, 22980688) are quantity increases. They are modification
  kinds and restate the same LotResult values, with one exception: lot 4's 22980688 (2023-02-20) copies the whole
  contract's 3,692,965.50 into its LotResult. The later 23168097 (2,288,265.00) supersedes it as LOT-4's latest award,
  so S does not take it.
- **Why right:** the same shape as read_1's 7380025. Five distinct medicines, as-awarded per-lot values, and they
  partition the single contract's total to the cent. The head without W is lot 4 alone (62 %). Both guards misfire. The
  one-result notice's total is the multi-lot contract's total (G-single), and its result names no summed key only
  because it prints "1,2,3,4,5" (G-cover). **Fragility (recorded only):** if 22980688 were LOT-4's latest row, S would
  be EUR 5,097,666.00 and R10 would refuse it at 38 % off.

### 6852228: right / ok (PGE Energia Ciepła, coal-mill grinding sets and spare parts; G-single)

- **Procedure:** an open sector procedure with 7 lots, one per branch. The CN 21174451 states no value. F06 21584710
  (2021-03-12) awards LOT_NO 1 at 3,698,300, 2 at 5,999,000, 3 at 531,480, 4 at 735,500 and 5 at 3,312,330. These are
  contracts 1–5 to the Metalodlew consortium, 1 tender each. LOT_NO 7 was cancelled because the price exceeded the budget.
  The procedure VAL_TOTAL is **PLN 14,276,610**, the sum of lots 1–5. F06 21600409 (2021-03-19) awards LOT_NO 6 at
  5,546,000 (contract 6), and its procedure **VAL_TOTAL is PLN 19,822,610** = 14,276,610 + 5,546,000, the cumulative
  procedure total.
- **The 2022–2023 F20s** (22470437/38/39/40/46, 23067905, 23320304, 23318309) restate each lot's original LotResult
  value. The raises sit in modification fields, and S does not use them: lot 1 +250,500; lot 2 +642,750, then +685,600;
  lot 6 +836,500, then +880,000; plus the 20 % / 35 % unit-price indexation of lots 4–5 with BEFORE = AFTER.
- **Why right:** six distinct branch lots, each awarded once and summed as awarded. S equals the buyer's cumulative
  total on the last award notice. The head without W is lot 5 alone (16.7 %). In 21584710, section LOT-6 prints LOT_NO 7;
  in 21600409, section LOT-1 prints LOT_NO 6. The store keys by the printed LOT_NO (LOT-7 no award, LOT-6 = 5,546,000),
  which is correct.

## Adversarial checks applied to every head

- **Double count:** every summed key is a distinct printed LOT_NO with its own title. Lots with several rows (6003114 ×3
  per part, 7394272, 6852228) restate one value, and S takes one row. Keyless whole-contract rows are not summed:
  7394272 22388431, 6003114 RES-1 (stored LOT-1, superseded), 7179291 22148426 and 22780357. 7394272's 22980688 copy is
  superseded.
- **Slip, VAT, currency:** each of the five rights equals a buyer-stated total to the cent, in the notices' own currency.
  6003114's net-against-gross was checked (38,200 × 1.23 = 46,986; S and T are both net). None of the six chains has a CN
  estimate that could contradict a lot value.
- **Shared envelope / not_additive:** 7102361 is a framework, but each lot has its own holder and value.
  6852228 ("sukcesywne dostawy") is one supply contract per lot. 7179291 is a framework whose S is wrong for a different
  reason (above).
- **Vouch belongs to something else (6465510 shape):** **7179291 only.** In the other five, T is the sum of every awarded
  lot of the procedure.
- **Superseded vouch (5816379 shape):** no later notice re-awards and states a different procedure total. Later notices
  state only their own lot's value (402503's 24982264 BT-161, 7102361's 22066668) or modification values, which are
  excluded.
- **Farther:** S/T = 0.00 % on the five rights. 7179291 is level with its head against the framework total
  (44.5 % / 44.4 %).

## What this says about G-single (recorded only; no rewording, per decision point 2)

Five of the six share read_1's shape. A one-result notice states the **procedure's** total in one of three ways:
- the last single-lot award states the cumulative total (7102361, 6852228);
- one result covers every lot (7394272: "1,2,3,4,5"; 6003114: no LOT_NO, gross);
- a modification notice carries the original CAN's total in BT-161 (402503).

7179291 is the target shape: T is the notice's own award, and it belongs to a contract outside S. On these six,
**G-single removes 1 non-right and 5 rights.** **G-cover removes 1 wrong (7179291) and 1 right (7394272).**

read_1's proposed line was "T equals that one result's own value, for a lot outside S". It does not separate these two
on stored facts alone:
- 7394272's single result also equals T, but it prints all five summed lots.
- 7179291's result prints "1,2", lots that **are** in S, but under a different contract (35-372, not 35-165).

What separates them is whether T equals the sum of S's own awards on the lots the result names: 3,692,965.50 = S for
7394272, but 83,146.73 ≠ 54,449.90 for 7179291.

## Requests (public API only; log `b6/u4/api_requests.log`, entries tagged `reader=read_4`)

36 GETs between 15:10:34Z and 15:16:24Z, paced at least 2 s apart under the shared lock (`b6/u4/.http.lock` +
`.http.stamp`). 35 returned 200. One got a connection reset with no HTTP status (`/v1/notices/21584710/content`,
rc=0); it was not a 408, and it was retried once and returned 200. There were 0 × 408.
- 6 live heads: `/v1/tenders/{7102361, 6003114, 7179291, 402503, 7394272, 6852228}`.
- 26 notices carrying the vouching totals and the summed awards (`/v1/notices/{id}/content`): 22048453, 22066668,
  19427538, 19437670, 19982269, 19973462, 19973463, 22780357, 22364146, 22315443, 22148426, 24416505, 24982264,
  22388431, 23168277, 23169697, 23168435, 23168097, 23168536, 21600409, 22470437, 23320304, 21584710, 22470439,
  23318309, 23067905.
- The 402503 twin: `/v1/tenders?publication_id=275434-2021` → 6896664, then `/v1/tenders/6896664` and
  `/v1/notices/21777064/content`.

Bodies are under `b6/u4/bodies/` (`tenders_{id}.json`, `notices_{id}_content.json`,
`tenders_publication_id-275434-2021.json`). Read from the U2 notice layer only: the CNs 21758156, 21936672 and 21174451;
6003114's 19628831/33/37, 19787805 and 19795352/53; 7394272's 22761239, 22864816, 22956107, 22963790 and 22980688; and
6852228's 22470438/40/46.

**Protocol note:** this file is one reader's read under M2. M2 pairs it with an independent adversarial skeptic per head
and sends any disagreement to a third reader. The checks above are this reader's own adversarial pass and do not replace
that skeptic. The skeptic should look first at 7179291's class (other or neutral). That choice does not move the
non-right count.
