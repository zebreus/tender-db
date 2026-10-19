# 551 B6 U4 guard-cost read, part 5: R_single ranks 27–30, R_excess's 1, and the cover-overlap extra

2026-10-19, reader `read_5`. This part covers six ids from `read_set.json` (sha256 d77efbc6…). All are ordered by
`sha256('551-b6-gcost-'+id)` ascending, and every order key was recomputed and matches:
- **R_single, ranks 27–30** (the last four of G-single's 30): 6784258, 7605188, 7163120, 5884304. These count toward
  G-single.
- **R_excess, rank 1 of 1**: 7202436, the only ship G-cover refuses beyond G-single. It counts toward G-cover.
- **cover_overlap_extra**: 7600895. It is rank 34 of R_single, so it is outside the 30. It is one of G-cover's 12
  R_single-overlap ids. It is recorded **separately** and **not counted for G-single**. It is used only in the G-cover gap
  case.

None of the six has a verdict from an earlier recorded read. I grepped read18, the B5 61, the post-drain 279, and
M2/fallback for each id and found nothing, so all six are new reads. 7605188 and 7163120 are also in G-cover's 12-id
overlap, so their verdicts feed the gap case as well.

**The question for each id:** the guard REFUSES this v2 ship. The ship serves the summed lot awards S as the tender's
value, vouched by a stated total. Under M2's rubric, is serving S RIGHT, NEUTRAL or WRONG as the tender's value?

**Rubric (M2, wf_dc85fa7e-c00's, as read_1 states it):**
- **right/ok:** S is the procedure's value. That means distinct lots (TED `LOT-k` checked against the printed LOT_NO),
  each lot's latest award, no lot counted twice, no ×10ᵏ slip and no VAT mix. S must also be no farther from the
  procedure's stated total than the head it replaces.
- **Wrong classes:** `not_additive`, `double_count`, `farther`, `slip`, and `other`. `other` covers the case where the
  vouching total belongs to something else, as in 6465510.
- **Neutral:** S is not wrong, but it is not clearly better than the head without W.

**Inputs:**
- Files: `b6/u2/R_single.json` c1b413e0…, `b6/u2/R_excess.json` 3d134390…, `b6/u2/decisions.jsonl.gz` 0add92e3…. These
  are the hashes `read_set.json` records, re-checked.
- Port predictions come from `decisions.jsonl.gz`: S, currency, n, summed keys and notices, the v2 `vouched_by`, the
  removed vouch, and the refusing check under each guard set.
- Live heads come from `GET /v1/tenders/{id}`. All six are JSON-equal to their U2 fresh-layer copies
  (`scratchpad/b6/tenders`, fetched 06:46Z–07:35Z), so the port's facts are still current.
- All 23 notices named below were fetched live and are JSON-equal to their U2-layer copies (`scratchpad/b6/notices`).
- No live head serves a derivation (`value_derivation: null`). Each serves its elected figure, which is the latest
  version's `result_value`. That figure is the "head without W" below.

## Result

| set | ids read here | right | neutral | wrong |
|---|---|---|---|---|
| R_single (ranks 27–30; counts for G-single) | 4 | **4** | 0 | 0 |
| R_excess (rank 1 of 1; counts for G-cover) | 1 | 0 | 0 | **1** (7202436, class `other`) |
| cover_overlap_extra (separate; gap case only) | 1 | **1** | 0 | 0 |

**The five rights.** In each, the notice carrying the vouching total states the **whole procedure's** awarded total,
and S equals it to the cent. There are two ways this happens:
- In a lot-per-notice award series (6784258, 5884304), one single-lot award notice states the procedure total.
- In three cases (7605188, 7163120, 7600895), one keyless multi-lot LotResult ("1,2" or "1,2,3") carries the total, and
  later per-lot notices split it exactly.

So in all five the guard removes a genuine vouch.

**The wrong, 7202436.** The vouching total is the vouching notice's **own single award**, for a lot that is not in S. It
lands 0.89 % from S by coincidence. This is the 6465510 shape. G-single misses it because that notice also carries two
unsuccessful LotResults. G-cover catches it.

| set / rank | id | S (port) | n | summed: key ← notice, award | v2 vouched_by (removed by the guard) | refusing guard(s) | head without W (live) | buyer-stated total(s) = S | S/T | head/S | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|
| R_single 27 | 6784258 | PLN 46,569,105.69 | 2 | LOT-1 ← 23284790 23,479,674.80; LOT-2 ← 21353722 23,089,430.89 | TED-VAL_TOTAL PLN 46,569,105.69 on 21353722 (F06, 1 result: LOT-2) and on 21366937 (F06, 1 result: LOT-1) | G-single only (cover keeps: each names a summed key) | PLN 23,479,674.80 | procedure VAL_TOTAL on 21353722 and 21366937 | 0.00 % | 50.4 % | **right / ok** |
| R_single 28 | 7605188 | EUR 3,816,100.00 | 2 | LOT-1 ← 23596947 3,368,100.00; LOT-2 ← 23597536 448,000.00 | TED-VAL_TOTAL EUR 3,816,100.00 on 22923984 (F03, 1 result, keyless "1,2") | G-single **and G-cover** | EUR 448,000.00 | 22923984 procedure VAL_TOTAL and RES-1 ("1,2") VAL_TOTAL | 0.00 % | 11.7 % | **right / ok** |
| R_single 29 | 7163120 | EUR 4,204,400.00 | 2 | LOT-1 ← 22182794 1,909,200.00; LOT-2 ← 22281094 2,295,200.00 | TED-VAL_TOTAL EUR 4,204,400.00 on 22136632 (F03 framework, 1 result, keyless "1,2") | G-single **and G-cover** | EUR 2,295,200.00 | 22136632 procedure VAL_TOTAL and RES-1 ("1,2") VAL_TOTAL | 0.00 % | 54.6 % | **right / ok** |
| R_single 30 | 5884304 | EUR 120,222.44 | 2 | LOT-1 ← 19740228 78,250.88; LOT-2 ← 19344705 41,971.56 | TED-VAL_TOTAL EUR 120,222.44 on 19344705 (F03, 1 result: LOT-2) | G-single only (cover keeps: names LOT-2) | EUR 78,250.88 | 19344705 procedure VAL_TOTAL | 0.00 % | 65.1 % | **right / ok** |
| R_excess 1 | 7202436 | EUR 27,222.00 | 2 | LOT-6 ← 22139899 15,850.00; LOT-18 ← 22139899 11,372.00 | TED-VAL_TOTAL EUR 26,981.20 on 22179136 (F03, 3 results: 1 award keyless + 2 unsuccessful on LOT-2/LOT-3) | **G-cover only** (single keeps: 3 results) | EUR 26,981.20 | none; 26,981.20 is 22179136's own award for former lot 4 | +0.89 % | 99.1 % | **WRONG / other** |
| cover extra (R_single 34) | 7600895 | EUR 22,046,027.00 | 3 | LOT-1 ← 23600770 13,387,725.50; LOT-2 ← 23600177 263,304.00; LOT-3 ← 23597734 8,394,997.50 | TED-VAL_TOTAL EUR 22,046,027.00 on 22875211 (F03, 1 result, keyless "1,2,3") | G-single **and G-cover** | EUR 13,387,725.50 | 22875211 procedure VAL_TOTAL and RES-1 ("1,2,3") VAL_TOTAL | 0.00 % | 60.7 % | **right / ok** |

Under every guard set the port's S, currency and n are the same as v2's. The ship flips to an R10 refusal only where a
guard removes the vouch (`first: R10`), and no second candidate is left in any of the six:
- 6784258 and 5884304: {single} and {single, cover}.
- 7605188, 7163120 and 7600895: all three guard sets.
- 7202436: {cover} and {single, cover}. {single} keeps the ship.

## Per head

### 6784258: right / ok (Enea Operator, MV line cabling in Lubuskie; G-single only)

- **Procedure:** an open utilities procedure, CN 21022946, with two lots and no value stated. Zadanie 1 is OD Gorzów
  Wlkp. (about 75.14 km of MV lines). Zadanie 2 is OD Zielona Góra (about 82.32 km).
- **The two F06 awards:**
  - 21353722 (2020-11-11) awards LOT_NO 2 to the Elektro-Silver / „Eldor" consortium: PLN 23,089,430.89, 5 tenders.
  - 21366937 (2020-11-18) awards LOT_NO 1 to ZUE EON s.c.: PLN 23,479,674.80, 6 tenders.
  - **Both** notices state procedure **VAL_TOTAL PLN 46,569,105.69**, which is 23,479,674.80 + 23,089,430.89 exactly.
- **The 2023 F20 23284790** (Aneks nr 2, price indexation citing Covid-19 and the war) restates lot 1's award,
  23,479,674.80, in its LotResult. It raises lot 1 to VAL_TOTAL_AFTER 31,429,674.80. That is a modification kind, and S
  does not use it.
- **Why right:** two distinct works lots under two contracts, each awarded once. S equals the buyer's procedure total,
  stated on both award notices, to the grosz. The head without W is lot 1 alone (50.4 %). Against the post-indexation
  total (54,519,105.69), S is still far closer than the head.
- **Guards:** G-single removes both vouches, since each notice has one LotResult. G-cover keeps them, since 21353722 names
  LOT-2 and 21366937 names LOT-1.

### 7605188: right / ok (Osakidetza, CEMIPLIMAB + ALIROCUMAB; G-single and G-cover)

- **Procedure:** F03 22923984 (2023/S 011-027574), a negotiated award without a call on exclusivity grounds. It has two
  Lot sections (1 CEMIPLIMAB, 2 ALIROCUMAB) and **one** LotResult, RES-1, with LOT_NO "1,2", contract 1, Sanofi Aventis,
  1 tender and VAL_TOTAL EUR 3,816,100.00. The procedure VAL_TOTAL is the same figure, with the note "Importes de
  adjudicación según precios unitarios". The store keeps that result on a lot = null row, and S does not sum it.
- **The 2023-12-26 F20s** restate each lot's award in its LotResult:
  - 23596947: LOT_NO 1, contract 1, 3,368,100.00.
  - 23597536: LOT_NO 2, contract 2, 448,000.00.
  - VAL_TOTAL_BEFORE equals each of these. VAL_TOTAL_AFTER doubles each (6,736,200 / 896,000) for a one-year extension.
  - Their ADDITIONAL_NEED text names the annual amounts: "3.368.100,00 y 448.000,00 euros IVA excluido".
- **Why right:** this is the same shape as read_1's 7380025. Two distinct medicines, each lot's latest award equal to its
  original award, and the sum equals the buyer's own award total to the cent. The extension values are not used. The head
  without W (lot 2's 448,000) is 11.7 % of the award.
- **Guards:** both misfire. The one-result notice's total belongs to a two-lot result, not to a single lot's own award
  (G-single). Its result names no summed key only because it prints "1,2" and stays keyless (G-cover).

### 7163120: right / ok (PAGD, aerial fire-rescue ladders, Lithuania; G-single and G-cover)

- **Procedure:** an open procedure, CN 21882899, with two lots and no value stated. Lot 1 is ladders of at least 20 m,
  4 units. Lot 2 is ladders of at least 35 m, 4 units.
- **The framework award:** F03 22136632 (2022/S 002-002671, `TED-FRAMEWORK` set) has **one** LotResult, RES-1, with
  LOT_NO "1,2", contract 35-587 (2021-12-21), UAB „Iksados" GTC, 2 tenders and VAL_TOTAL EUR 4,204,400.00. The procedure
  VAL_TOTAL is the same figure. It is stored lot = null and is not summed.
- **Two later F03s** each name 22136632 as the prior notice:
  - 22182794 (2022-01-28): LOT_NO 1, contract 35-598 (2021-12-23), 1,909,200.00.
  - 22281094 (2022-03-21): LOT_NO 2, contract 35-65 (2022-03-09), 2,295,200.00.
  - 1,909,200 + 2,295,200 = 4,204,400 exactly.
- **Why right:** two distinct lots (different ladder heights), one supplier per lot. The later lot-level contracts split
  the framework's award total exactly. Each lot is counted once, and S equals the buyer-stated award total. Whether the two
  later contracts are the per-lot contracts or call-offs under 35-587, the sum is the same figure. This is not a
  `not_additive` case: no envelope is shared across parallel suppliers. The head without W (lot 2 alone) is 54.6 %.
- **Guards:** same misfire as 7605188. The total is a multi-lot result's total, and that result is keyless because it
  prints "1,2".

### 5884304: right / ok (Mesto Malacky, two cultural events; G-single only)

- **The chain:**
  - **F15 (VEAT) 19197976** (2017-03-28): a direct negotiation after a design contest, written in the future tense
    ("bude rokovať s jedným… účastníkom"). It has one keyless RES-1 with VAL_TOTAL 174,337.32, covering both parts for two
    years.
  - **F14 19223468** corrects it: lot 1's V.2.4 changes from 174,337.32 to 111,473.36, and a lot 2 award of 62,863.96 is
    added. The total is the same.
  - **F03 19285258** (2017-05-06) awards LOT_NO 1 "Prvomájové Malacky" to Media Invest Company: 78,250.88, contract
    2017-04-11, 1 tender, VAL_TOTAL 78,250.88. INFO_ADD says this notice covers only part 1 and part 2 will follow.
  - **F03 19344705** (2017-07-08) awards LOT_NO 2 "Vianočné trhy Malacky" (2017–2018) to the same firm: 41,971.56,
    contract 2017-07-03, 1 tender, with **procedure VAL_TOTAL 120,222.44**. Its SHORT_DESCR says "Zmluva bola uzatvorená
    na obidve časti" (contracts have been concluded for both parts).
  - **F20 19740228** (2018-03-06) restates lot 1 at 78,250.88 and raises it to 79,690.88 for the 2018 programme. That is a
    modification kind, and S does not use it.
- **Why right:** S = 78,250.88 + 41,971.56 = 120,222.44, the buyer's procedure total on the closing award notice. The head
  without W is lot 1 alone (65.1 %). The VEAT's 174,337.32 is an ex-ante intended value; the negotiated contracts came in
  lower. S is also closer to that figure than the head is (69.0 % against 44.9 %).
- **Note (no verdict impact):** the live head stores the VEAT's keyless result on LOT-1 (174,337.32, decided
  2017-03-16). LOT-1's latest award is 78,250.88 (19285258, restated by 19740228), and that is what S takes.

### 7202436: WRONG / other (VšĮ Respublikinė Vilniaus universitetinė ligoninė, food products; R_excess, G-cover only)

- **The open procedure Nr.5610-1:** CN 21970781 has 18 lots and no value stated. Its F03 22139899 (2022-01-07) has six
  LotResults:

  | result | LOT_NO | contractor | award | stored as |
  |---|---|---|---|---|
  | RES-1 | 6 | Daivida | 15,850.00 (7 tenders) | LOT-6 |
  | RES-2 | "8,9" | Riamona | 5,804.00 | keyless |
  | RES-3 | "12,15,16" | Officeday | 15,116.40 | keyless |
  | RES-4 | 18 | Mineraliniai vandenys ir alus | 11,372.00 (1 tender) | LOT-18 |
  | RES-5 | 17 | discontinued | none | |
  | RES-6 | "1-5,7,10,14,11,13" | unsuccessful | none | |

  INFO_ADD explains RES-6: lots 1–5, 7, 10 and 14 were not signed because the prices were too high, and the bids for 11
  and 13 did not meet the conditions. The **procedure VAL_TOTAL is EUR 48,142.40**, exactly the sum of the four awards.
- **The follow-on negotiated procedure Nr.6506:** F03 22179136 (2022-01-26) runs under VPĮ 63(1)(2), after the open
  procedure's bids were rejected as too expensive.
  - It renumbers three of the failed lots: "1 (buv. 4 p.d.)" frozen pastry, "2 (buv. 5 p.d.)" ice cream, "3 (buv.7 p.d.)"
    beef.
  - RES-1 awards **former lot 4** to UAB „Mantinga": **EUR 26,981.20**, 2 tenders, contract S-6.19E-21/2022.
  - RES-2 and RES-3 are unsuccessful.
  - The notice's **VAL_TOTAL is 26,981.20, which is RES-1's own award**.
- **S and its vouch:** S = LOT-6 15,850.00 + LOT-18 11,372.00 = 27,222.00, both from 22139899. v2's vouch is
  TED-VAL_TOTAL 26,981.20 on 22179136, which is 0.89 % from S, inside R10's 1 %.
- **Why wrong:**
  - The vouching total is the 22179136 notice's own single award, for former lot 4 (frozen pastry), and that lot is not
    in S. The figure says nothing about pork (LOT-6) plus water and soft drinks (LOT-18). It falls within 1 % of their sum
    by coincidence.
  - The rubric names this wrong class explicitly (`other`: the vouching total belongs to something else). It is exactly
    6465510's shape: a single-lot award notice's own total, for a lot outside S.
  - No total of this procedure vouches S. The open procedure's own award total is 48,142.40, so S covers 56.5 %. The
    chain's whole awarded money is 48,142.40 + 26,981.20 = 75,123.60, so S covers 36.2 %. The head without W (26,981.20)
    is 56.0 % and 35.9 % of those totals, so S is not meaningfully closer either.
  - Left out of S: the keyless multi-lot awards "8,9" (5,804.00) and "12,15,16" (15,116.40), and the negotiated lot 4
    (26,981.20, on a lot = null row).
- **Why G-single misses it and G-cover catches it:** 22179136 has three LotResults, one awarded and two unsuccessful
  (`round_results` 3), so "exactly ONE LotResult" fails and the vouch survives. G-cover refuses it because the notice's
  only rows held on a lot (RES-2 on LOT-2, RES-3 on LOT-3, both unsuccessful) are not summed keys, and its awarded result
  is keyless (`names_summed_keys: []`).
- **Recorded observation (no verdict impact):** the negotiated notice's renumbering conflates lot keys. Its LOT_NO 2 and
  3 (former 5 and 7) are served on this tender's LOT-2 and LOT-3, which in the open CN are "Kiti pieno gaminiai" and
  "Pyrago gaminiai". The live head's LOT-1/2/3 titles come from the negotiated notice. RES-1 "1 (buv.4 p.d.)" stays
  keyless. If a future lot-key change keyed it by its leading "1", it would land on the open procedure's LOT-1 (kefir and
  yoghurt) and join S as the wrong lot. This is worth a pin beside 6908248's, before any such drain.

### 7600895: right / ok (Osakidetza, DARATUMUMAB / ETRAVIRINA / IBRUTINIB; cover_overlap_extra, not counted for G-single)

- **Procedure:** F03 22875211 (2023/S 006-013929), a negotiated award without a call. It has three Lot sections and
  **one** LotResult, RES-1, with LOT_NO "1,2,3", contract 1, Janssen-Cilag, 1 tender and VAL_TOTAL EUR 22,046,027.00. The
  procedure VAL_TOTAL is the same figure ("importes de adjudicación según precios unitarios"). It is stored lot = null and
  is not summed.
- **The 2023-12-26 F20s** restate each lot's original award:
  - 23600770: LOT_NO 1, contract 1, 13,387,725.50.
  - 23600177: LOT_NO 2, contract 2, 263,304.00.
  - 23597734: LOT_NO 3, contract 3, 8,394,997.50.
  - VAL_TOTAL_BEFORE equals each of these. VAL_TOTAL_AFTER doubles each for a one-year extension.
  - The ADDITIONAL_NEED text names the annual amounts: "13.387.725,50, 263.304,00 y 8.394.997,50 euros IVA excluido".
- **Why right:** 13,387,725.50 + 263,304.00 + 8,394,997.50 = 22,046,027.00, the buyer's award total to the cent. These
  are three distinct medicines, and the extension values are not used. The head without W (lot 1) is 60.7 %. Both guards
  misfire, for the same reasons as on 7605188 and read_1's 7380025.

## Adversarial checks applied to every head

- **Double count:**
  - Every summed key is a distinct printed LOT_NO with its own subject.
  - The keyless multi-lot rows ("1,2" on 7605188 and 7163120, "1,2,3" on 7600895) are not summed. On 7202436 the
    keyless rows are left out of S (the opposite fault).
  - 5884304's LOT-1 carries three rows (the VEAT's 174,337.32, then 78,250.88 twice), and S takes the latest award.
  - 6784258's LOT-1 has two rows, both 23,479,674.80.
- **Slip, VAT, currency:**
  - Each of the five rights equals a buyer-stated total to the cent, in the notices' own currency.
  - The Osakidetza modification texts state their amounts "IVA excluido".
  - None of the CNs (21022946, 21882899, 21970781) states a value that could contradict a lot figure.
- **Shared envelope / not_additive:**
  - 7163120 carries the framework flag, but each lot is a distinct purchase with one supplier, and the lot values split
    the framework total exactly.
  - The Osakidetza "precios unitarios" awards are the buyer's own award amounts for distinct exclusive medicines, and
    they add up.
  - No multi-supplier lot appears anywhere.
- **Vouch belongs to something else (6465510 shape):**
  - In the five rights, T is at least the sum of every lot of the procedure, and it exceeds the vouching notice's own
    single-lot value wherever that notice has one (21353722, 21366937, 19344705).
  - In 7202436, T equals that notice's own award, for a lot outside S. That is the wrong.
- **Superseded vouch (5816379 shape):**
  - No later notice re-awards a summed lot and states a different procedure total.
  - The later notices are either the other lot's award (6784258, 5884304, 7163120) or F20s whose AFTER values are
    extensions or indexation, which are modification kinds.
  - 5884304's VEAT total is earlier, not later, and it is ex ante.
- **Farther:** S/T is 0.00 % on the five rights, and the head without W covers 11.7–65.1 % of them. 7202436 is not
  meaningfully closer than its head.

## What this says about the guards (recorded only; no rewording, per decision point 2)

- **G-single:** on these four R_single reads it removes four genuine whole-procedure vouches and no non-right. It does
  the same on 7600895 (separate). The shapes match read_1's:
  - In 6784258 and 5884304, a single-lot award notice states the procedure total, and T is greater than that notice's own
    LotResult value.
  - In 7605188, 7163120 and 7600895, one keyless multi-lot result carries the total.
  - It also **misses** the 6465510 shape on 7202436, because the coincidental vouch's notice lists two unsuccessful
    results beside its single award.
- **G-cover:** it refuses 7202436 correctly, and that vouch is the one the R_excess read exists to test. On 7605188,
  7163120 and 7600895 (all also G-single refusals) it misfires on keyless multi-lot results whose totals equal S exactly.
  The same is true of read_1's 7380025 and read_3's 7651470, 6356382 and 7166738.
- **Arithmetic under point 3 and the gap rule** (for the adoption step; it holds only if M2's skeptic confirms 7202436):
  - G-cover's own read set is R_excess = {7202436}. That is 1 of 1 non-right (100 % ≥ 10 %), so G-cover meets its bar.
  - In the gap case, where U4 rejects G-single, G-cover's 13-read set already holds at least 2 non-right: 6465510
    (recorded wrong) and 7202436. That is ≥ 2 of 13 = 15.4 % ≥ 10 %, so G-cover meets its bar there too, whatever the
    other overlap verdicts are.
  - 7600895 (right) is recorded here for that 13-read set.
  - The adoption call itself is not this reader's.

## Requests (public API only; log `b6/u4/api_requests.log`, entries tagged `reader=read_5`)

29 GETs between 15:20:55Z and 15:22:32Z. All returned 200, with 0 × 408 and 0 retries. They were paced at least 2 s apart
under the shared lock (`b6/u4/.http.lock` + `.http.stamp`). The smallest gap before any read_5 entry is 2.45 s, and the
smallest across the whole log is 2.05 s. The fetcher is `scratchpad/u4r5/get.py`, which is read_1's with the reader tag
changed.
- 6 live heads: `/v1/tenders/{6784258, 7605188, 7163120, 5884304, 7202436, 7600895}`.
- 23 notices carrying the vouching totals, the summed awards and the other award, modification and CN rows of each chain
  (`/v1/notices/{id}/content`):
  - 6784258: 21353722, 21366937, 23284790, 21022946;
  - 5884304: 19344705, 19740228, 19285258, 19197976, 19223468;
  - 7605188: 22923984, 23596947, 23597536;
  - 7163120: 22136632, 22182794, 22281094, 21882899;
  - 7202436: 22179136, 22139899, 21970781;
  - 7600895: 22875211, 23600177, 23597734, 23600770.

Bodies are under `b6/u4/bodies/` (`tenders_{id}.json`, `notices_{id}_content.json`). No `/v1/sql`, POST, ssh, git or
cargo was used.

**Protocol note:** this file is one reader's read under M2. M2 pairs it with an independent adversarial skeptic per head
and sends any disagreement to a third reader. The checks above are this reader's own adversarial pass, not a substitute
for that skeptic. The wrong on 7202436 in particular should get its skeptic before the adoption arithmetic is applied.
