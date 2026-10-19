# 551 B6 U4 guard-cost read, part 1: R_single ranks 1–6

2026-10-19, reader `read_1`. These are the first six ids of R_single's read set, in `sha256('551-b6-gcost-'+id)` ascending
order (`read_set.json`, sha256 d77efbc6…): 7380025, 472292, 7785319, 7014656, 7675062, 6234738. None of the six has a
verdict from an earlier recorded read (read18, the B5 61, the post-drain 279, M1/M2/fallback): all six are new reads.
7380025 is also one of G-cover's 12 R_single-overlap ids, so its verdict also feeds the G-cover gap case.

**The question for each id:** G-single refuses this v2 ship (the summed lot awards S, served as the tender's value and
vouched by a stated total). Under M2's rubric, is serving S RIGHT, NEUTRAL or WRONG as the tender's value?

**Rubric (M2, wf_dc85fa7e-c00's):** right/ok means S is the procedure's value: distinct lots (TED `LOT-k` checked against
the printed LOT_NO), each lot's latest award, no lot counted twice, no ×10ᵏ slip, no VAT mix, and S is no farther from
the procedure's stated total than the head it replaces. Wrong classes: `not_additive` (ceilings or maxima sharing one
envelope), `double_count`, `farther`, `slip`, `other` (for example, the vouching total belongs to something else, as in
6465510). Neutral means S is not wrong but not clearly better than the head without W.

**Inputs:** `b6/u2/R_single.json` c1b413e0…, `b6/u2/R_excess.json` 3d134390…, `b6/u2/decisions.jsonl.gz` 0add92e3…
(the same hashes `read_set.json` records). Port-predicted S, n, summed keys and notices, v2 `vouched_by` and the refusing
guard come from `decisions.jsonl.gz`. Live heads come from `GET /v1/tenders/{id}`. Every live head is JSON-equal to its
U2 fresh-layer copy (`scratchpad/b6/tenders`, fetched 06:26Z–07:37Z), so the port's facts are still current. Every notice
named below was fetched live and is JSON-equal to its U2-layer copy. One notice outside the six chains, 23018953 (the CAN
on twin tender 7499938), was fetched only live.

## Result

**6 right / 0 neutral / 0 wrong**, all class `ok`. All six are refused by G-single only; 7380025 is also refused by
G-cover. In every case the notice that carries the vouching total has exactly one LotResult, but its stated total is the
**whole procedure's** awarded total (the sum of every lot), not that notice's own lot award. So R10's vouch is genuine,
and the guard removes a correct vouch.

| rank | id | S (port) | n | summed: key ← notice, award | v2 vouched_by (all removed by G) | refusing guard | head without W (live) | buyer-stated total(s) matching S | S/T | head/S | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 7380025 | EUR 2,784,373.10 | 3 | LOT-1 ← 23081023 771,204.00; LOT-2 ← 23081641 311,301.90; LOT-3 ← 23081236 1,701,867.20 | TED-VAL_TOTAL EUR 2,784,373.10 on 22369006 (F03, 1 result, keyless) | G-single **and G-cover** (names no summed key) | EUR 311,301.90 | 22369006: procedure VAL_TOTAL and the single RES-1 (LOT_NO "1 - 2 - 3") VAL_TOTAL, both 2,784,373.10 | 0.00 % | 11.2 % | **right / ok** |
| 2 | 472292 | HUF 513,615,371.00 | 4 | LOT-0001 ← 23951676 389,741,363; LOT-0002 ← 23730496 25,957,272; LOT-0003 ← 23758321 72,997,645; LOT-0004 ← 23795731 24,919,091 | BT-161 HUF 513,615,371 on 23758321 and on 23795731 (each 1 result: LOT-0003 / LOT-0004) | G-single | HUF 389,741,363.00 | BT-161 on 23758321 and 23795731; the twin's CAN 23018953 VAL_TOTAL 513,615,371 | 0.00 % | 75.9 % | **right / ok** |
| 3 | 7785319 | PLN 41,500.00 | 2 | LOT-1 ← 23564346 7,000.00; LOT-2 ← 23613614 34,500.00 | TED-VAL_TOTAL PLN 41,500 on 23564346 (F03, 1 result: LOT-1) | G-single | PLN 34,500.00 | 23564346 procedure VAL_TOTAL 41,500 | 0.00 % | 83.1 % | **right / ok** |
| 4 | 7014656 | EUR 163,160.00 | 2 | LOT-1 ← 21635083 87,480.00; LOT-2 ← 22164062 75,680.00 | TED-VAL_TOTAL EUR 163,160 on 21635083 (F21, 1 result: LOT-1) | G-single | EUR 75,680.00 | 21635083 VAL_TOTAL; F20 22164062 VAL_TOTAL_BEFORE = AFTER 163,160 | 0.00 % | 46.4 % | **right / ok** |
| 5 | 7675062 | EUR 182,106.53 | 3 | LOT-1 ← 23316732 68,959.25; LOT-2 ← 23312560 83,308.39; LOT-3 ← 23314405 29,838.89 | TED-VAL_TOTAL EUR 182,106.53 on 23312560 and on 23314405 (each 1 result: LOT-2 / LOT-3) | G-single | EUR 68,959.25 | VAL_TOTAL on 23312560 and 23314405 | 0.00 % | 37.9 % | **right / ok** |
| 6 | 6234738 | EUR 68,497.20 | 2 | LOT-1 ← 20033617 24,930.00; LOT-2 ← 20033615 43,567.20 | TED-VAL_TOTAL EUR 68,497.20 on 20033615 (F03, 1 result: LOT-2) | G-single | EUR 24,930.00 | 20033615 procedure VAL_TOTAL | 0.00 % | 36.4 % | **right / ok** |

Under every guard set the port's S, currency and n are the same as v2's. The ship flips to an R10 refusal only where a
guard removes the vouch: {single} and {single, cover} for all six; {cover} for 7380025 alone. None of the six live heads
serves a derivation (`value_derivation: null`), so each serves its max-elected figure, as the table shows.

## Per head

### 7380025: right / ok (Osakidetza, three exclusive-supply medicines; G-single and G-cover)

- **Procedure:** a negotiated award without a call (exclusive marketing), TED F03 22369006 (2022/S 089-241567). There
  are three Lot sections (LOT_NO 1 INTERFERON BETA 1A, 2 FAMPRIDINA, 3 NUSINERSEN) and **one** LotResult, RES-1, with
  LOT_NO "1 - 2 - 3", CONTRACT_NO 1, Biogen Spain, 1 tender and VAL_TOTAL EUR 2,784,373.10. The procedure VAL_TOTAL is
  the same figure. One contract covers all three lots. The store keeps that result on a lot = null row, and S does not sum
  it.
- **The 2023 F20s** each restate one lot's original award in their LotResult: 23081023 LOT_NO 1 771,204.00, 23081641
  LOT_NO 2 311,301.90, 23081236 LOT_NO 3 1,701,867.20 (23014672 also restates lot 3 at 1,701,867.20). These three
  values add to exactly the 2022 award total, 2,784,373.10. The modifications themselves (lot 3 +255,280.08 for extra
  units; then a one-year extension that doubles each lot, VAL_TOTAL_AFTER 1,542,408.00 / 622,603.80 / 3,914,294.56) are
  modification kinds, and S does not use them.
- **Why right:** S is three distinct medicines, each lot's latest award equal to its original award, and it equals the
  buyer's own award total to the cent. The head without W (lot 2's EUR 311,301.90) is 11 % of the award. Nothing is
  double counted: the keyless 2022 row is not summed. Both guards misfire here. The one-result notice's total is the
  multi-lot contract's total, not a single lot's own award (G-single). Its result names no summed key only because it
  prints "1 - 2 - 3" (G-cover).

### 472292: right / ok (Miskolc, AVASI KILÁTÓ ÉS KÖRNYEZETE; modification-only twin)

- **This tender's chain** holds four eForms contract-modification notices (subtype 38), one per lot. Each modifies the
  VAT clause (reverse charge) and the named personnel, "a nettó vállalkozói díj módosítása nélkül" (the net fee is not
  changed), and each carries its lot's original BT-720: LOT-0001 389,741,363 (net 349,730,592 + reserve + option),
  LOT-0002 25,957,272, LOT-0003 72,997,645, LOT-0004 24,919,091. BT-161 on the LOT-0003 and LOT-0004 notices is
  513,615,371, the sum of all four. BT-161 on the LOT-0001 and LOT-0002 notices is each notice's own lot value.
- **The original award** sits on twin tender **7499938** (`/v1/tenders?publication_id=146509-2023`, the notice every
  modification names in BT-1501(n)). Its F03 CAN 23018953 awards LOT_NO 1–4 to the same contractor at exactly these four
  values, with procedure **VAL_TOTAL HUF 513,615,371**. So S is the CAN's award total to the forint. (The free text of
  the LOT-0004 modification prints 24,191,091; BT-720 and the CAN say 24,919,091, so the text has a digit transposition.)
- **Why right:** all 4 of 4 lots are summed at their original award values, with no pre/post-modification mix. Lot 1's
  later net change (347,039,720 net) is a modification kind and is not used. S equals two buyer-stated BT-161s and the
  CAN's VAL_TOTAL. The head without W is lot 1 alone (75.9 %). This differs from M2's wrong 1091419, a modification-only
  twin whose S covered 3 of 9 lots and mixed pre- and post-modification values. **Note (no verdict impact):** the procedure
  is represented twice, by 7499938 (currently serving its HUF 530,431,034 estimate; not in the 3,502) and by 472292. That
  twin is older than path W, and S makes 472292 agree with the CAN.

### 7785319: right / ok (Województwo Mazowieckie, leaflets)

- **Procedure:** an open procedure, CN 23279414 (2 lots, LOT_ALL, no value stated). F03 23564346 awards LOT_NO 1 (A5,
  100,000 leaflets) to Zapol Sobczyk: PLN 7,000.00, 12 tenders. Its procedure **VAL_TOTAL is PLN 41,500.00**. F03
  23613614 awards LOT_NO 2 (DL, 300,000 leaflets) to the same firm: PLN 34,500.00, 12 tenders, VAL_TOTAL 34,500 (its own
  lot).
- **Why right:** two distinct lots, each awarded once. The lot-1 notice's procedure total is 7,000 + 34,500 exactly, so
  the buyer stated the whole procedure's total there. It is not lot 1's own figure, and it is no coincidence. The head
  without W is lot 2 alone (83 %).

### 7014656: right / ok (Eesti Töötukassa, Kaitstud töö)

- **Procedure:** a negotiated procedure with a call, CN 21552283 (2 lots: Kohtla-Järve and Haapsalu, no value stated).
  Notice 21621407 awards LOT_NO 2 to SA Hea Hoog, EUR 75,680, 1 tender (VAL_TOTAL 75,680, its own lot). F21 21635083
  awards LOT_NO 1 to OÜ Keerub, EUR 87,480, 3 tenders, with procedure **VAL_TOTAL EUR 163,160**. F20 22164062 (lot 2,
  only the term corrected) restates lot 2 at 75,680 and carries **VAL_TOTAL_BEFORE = AFTER EUR 163,160**.
- **Why right:** two distinct lots and distinct contracts (7-6.15/21/002 and /003). S = 87,480 + 75,680 = 163,160, which
  the buyer states in two notices. The head without W is lot 2 alone (46 %).

### 7675062: right / ok (Emschergenossenschaft, waste disposal)

- **Procedure:** an open procedure, CN 23054547 (3 regional lots: West, Mitte, Ost; no value stated). Three F03s on
  2023-07-07: 23312560 awards LOT_NO 2 (Mitte) to Drekopf, EUR 83,308.39; 23314405 awards LOT_NO 3 (Ost) to Drekopf,
  EUR 29,838.89; 23316732 awards LOT_NO 1 (West) to Remondis, EUR 68,959.25. Each had 2 tenders. Both Drekopf notices state
  procedure **VAL_TOTAL EUR 182,106.53**. The Remondis notice states its own 68,959.25. In 23312560 the section ids are
  swapped (section LOT-1 prints LOT_NO 2), but its result prints LOT_NO 2 and is served on LOT-2 (Mitte), as it should be.
- **Why right:** three distinct regional lots and three awards. S equals the buyer's procedure total, stated twice. The
  head without W is lot 1 alone (38 %).

### 6234738: right / ok (UDAP de la Marne, Reims cathedral terrace)

- **Procedure:** an open procedure, CN 19893804 (LOT_NO 1 Maçonnerie, 2 Étanchéité; no value stated). F03 20033615
  awards LOT_NO 2 to SEL-TECH, EUR 43,567.20, 2 tenders, with procedure **VAL_TOTAL EUR 68,497.20**. F03 20033617 awards
  LOT_NO 1 to Léon Noël, EUR 24,930.00, 2 tenders, VAL_TOTAL 24,930 (its own lot).
- **Why right:** two distinct trades. S = 43,567.20 + 24,930.00 = the buyer's stated total. The head without W is lot 1
  alone (36 %).

## Adversarial checks applied to every head (each found nothing)

- **Double count:** every summed key is a distinct printed LOT_NO with its own title. Where a lot has several rows
  (7380025 LOT-3, 7014656 LOT-2) the rows restate the same value, and S takes one. The keyless 7380025 2022 row is not
  summed.
- **Slip, VAT, currency:** every S equals a buyer-stated total to the cent, in the notices' own currency. No CN estimate
  contradicts any lot value. 472292's estimates (lots 2–4: 22.0 m / 57.0 m / 20.4 m against awards 26.0 m / 73.0 m /
  24.9 m) are consistent.
- **Shared envelope / not_additive:** no framework, DPS or multi-supplier lot. 7380025's lots are one contract, but its
  per-lot figures partition the contract total exactly.
- **Vouch belongs to something else (6465510 shape):** in no case does T equal the vouching notice's own lot award. In
  each, T is at least the sum of every lot of the procedure.
- **Superseded vouch (5816379 shape):** no later notice re-awards and states a different procedure total. The later
  notices state either their own lot's value (7785319 23613614, 6234738 20033617, 7675062 23316732, 472292 23951676 /
  23730496) or modification values, which are excluded.
- **Farther:** S/T = 0.00 % on all six, and the head without W covers 11–83 % of the procedure.

## What this says about G-single (recorded only; no rewording, per decision point 2)

The six share one shape. In a lot-per-notice award series, one of the single-lot award notices (or, in 7380025, a single
multi-lot result) states the **procedure's** total in its notice-level VAL_TOTAL or BT-161. G-single treats every
one-result notice's total as that notice's own award. 6465510's total was its lot's own award (T = lot 4's VAL_OBJECT).
In these six, T exceeds the notice's own LotResult value and equals the full sum. So the 6465510 shape is not
"one LotResult" but "T equals that one result's own value, for a lot outside S". On these six reads, G-single costs 6
rights and removes no non-right.

## Requests (public API only; log `b6/u4/api_requests.log`, entries tagged `reader=read_1`)

28 GETs, all 200, 0 × 408, 0 retries, paced at least 2 s apart under a lock (`b6/u4/.http.lock` + `.http.stamp`):
- 6 live heads: `/v1/tenders/{7380025, 472292, 7785319, 7014656, 7675062, 6234738}`;
- 19 notices carrying the vouching totals and the summed awards: 22369006, 23081023, 23081641, 23081236, 23014672,
  23758321, 23795731, 23951676, 23730496, 23564346, 23613614, 21635083, 22164062, 21621407, 23312560, 23314405,
  23316732, 20033615, 20033617 (`/v1/notices/{id}/content`);
- the 472292 twin: `/v1/tenders?publication_id=146509-2023` → 7499938, `/v1/tenders/7499938`,
  `/v1/notices/23018953/content`.

Bodies are under `b6/u4/bodies/` (`tenders_{id}.json`, `notices_{id}_content.json`,
`tenders_publication_id-146509-2023.json`). The CNs 23279414, 21552283, 23054547 and 19893804 were read from the U2
notice layer (`scratchpad/b6/notices`) only.

**Protocol note:** this file is one reader's read under M2. M2 pairs it with an independent adversarial skeptic per head
and sends a disagreement to a third reader. The checks above are this reader's own adversarial pass, not a substitute for
that skeptic.
