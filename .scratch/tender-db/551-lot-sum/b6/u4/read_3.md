# 551 B6 U4 guard-cost read, part 3: R_single ranks 13, 14, 17–20

2026-10-19, reader `read_3`. These six ids come from R_single's read set, in `sha256('551-b6-gcost-'+id)` ascending
order (`read_set.json`, sha256 d77efbc6…): 6815094 (rank 13), 6403567 (14), 7651470 (17), 6937514 (18), 6356382 (19),
7166738 (20). Ranks 15 and 16 are 6595330 (read18: right / ok) and 6465510 (B5 61: wrong / other). They keep their
recorded verdicts and are not re-read here. None of the six has a verdict from an earlier recorded read: read18, the B5
61, the post-drain 279, M2, the M2 fallback, the unit-2 panel and `results-slim.jsonl` were each grepped for every id,
with 0 hits. So all six are new reads. **7651470, 6356382 and 7166738 are also among G-cover's 12 R_single-overlap
ids**, so their verdicts also feed the G-cover gap case.

**The question for each id:** G-single refuses this v2 ship (the summed lot awards S, served as the tender's value and
vouched by a stated total). Under M2's rubric, is serving S RIGHT, NEUTRAL or WRONG as the tender's value?

**Rubric (M2, wf_dc85fa7e-c00's):** right/ok means S is the procedure's value. The lots must be distinct (TED `LOT-k`
checked against the printed LOT_NO), each lot must carry its latest award, and no lot may be counted twice. There must
be no ×10ᵏ slip and no VAT mix, and S must be no farther from the procedure's stated total than the head it replaces.
The wrong classes are:
- `not_additive`: ceilings or maxima that share one envelope;
- `double_count`;
- `farther`;
- `slip`;
- `other`: for example, the vouching total belongs to something else, as in 6465510.

Neutral means S is not wrong but is not clearly better than the head without W.

**Inputs:** `b6/u2/R_single.json` c1b413e0…, `b6/u2/R_excess.json` 3d134390…, `b6/u2/decisions.jsonl.gz` 0add92e3…
(the hashes `read_set.json` records, re-checked). The port-predicted S, n, summed keys and notices, the v2 `vouched_by`,
the vouch candidates and the refusing guard all come from `decisions.jsonl.gz`.
- **Live heads:** from `GET /v1/tenders/{id}`. Every live head is JSON-equal to its U2 fresh-layer copy
  (`scratchpad/b6/tenders`, fetched 06:55Z–07:36Z), so the port's facts are still current.
- **Notices:** all 28 notices named below were fetched live, and each is JSON-equal to its U2-layer copy
  (`scratchpad/b6/notices`).
- **CNs:** 21103713, 20215452, 21364792, 20114683 and 21889789 were read from the U2 notice layer only.
- **Flags:** U2 raised no estimate-twin flag on any of the six (`flags: []`).

## Result

**6 right / 0 neutral / 0 wrong**, all class `ok`. G-single refuses all six. G-cover also refuses 7651470, 6356382
and 7166738. In every case the vouching notice holds exactly one LotResult (`round_results = content_lot_results = 1`),
but its notice-level VAL_TOTAL is the **whole procedure's** award total, not that notice's own lot award. So R10's vouch
is genuine, and the guard removes a correct vouch. The shape is the same as all six of read_1's ids.

| rank | id | S (port) | n | summed: key ← notice, award | v2 vouched_by (removed by G) | refusing guard | head without W (live) | buyer-stated total(s) matching S | S/T | head/S | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 13 | 6815094 | EUR 677,400.00 | 4 | LOT-1 ← 21812830 213,180.00; LOT-2 ← 22251256 191,400.00; LOT-3 ← 21571535 174,900.00; LOT-4 ← 21786612 97,920.00 | TED-VAL_TOTAL EUR 677,400 on 21285538 (F21, 1 result: LOT_NO 2) | G-single (cover keeps it: names LOT-2) | EUR 191,400.00 | 21285538 procedure VAL_TOTAL; VAL_TOTAL_AFTER on all four F20s | 0.00 % | 28.3 % | **right / ok** |
| 14 | 6403567 | EUR 280,000.00 | 2 | LOT-1 ← 20520516 142,000.00; LOT-2 ← 20520354 138,000.00 | TED-VAL_TOTAL EUR 280,000 on 20520516 (F03, 1 result: LOT_NO 1) | G-single (cover keeps it: names LOT-1) | EUR 138,000.00 | 20520516 procedure VAL_TOTAL | 0.00 % | 49.3 % | **right / ok** |
| 17 | 7651470 | EUR 5,667,313.10 | 2 | LOT-1 ← 23740527 2,860,863.10; LOT-2 ← 23438905 2,806,450.00 | TED-VAL_TOTAL EUR 5,667,313.10 on 23006164 (F03, 1 result, keyless "Lotes 1 y2") | G-single **and G-cover** (names no summed key) | EUR 2,860,863.10 | 23006164 procedure VAL_TOTAL and its single RES-1 VAL_TOTAL | 0.00 % | 50.5 % | **right / ok** |
| 18 | 6937514 | EUR 88,732.00 | 4 | LOT-1 ← 21465935 11,752.00; LOT-2 ← 21536830 25,200.00; LOT-3 ← 21947679 34,080.00; LOT-4 ← 21530641 17,700.00 | TED-VAL_TOTAL EUR 88,732 on 21530641 (F21, 1 result: LOT_NO 4) | G-single (cover keeps it: names LOT-4) | EUR 34,080.00 | 21530641 procedure VAL_TOTAL; VAL_TOTAL_AFTER on F20s 21536830 and 21947679 | 0.00 % | 38.4 % | **right / ok** |
| 19 | 6356382 | EUR 285,379.52 | 2 | LOT-1 ← 23402733 203,335.42; LOT-2 ← 23402794 82,044.10 | TED-VAL_TOTAL EUR 285,379.53 on 20317908 (F03, 1 result, keyless "Los 1 und Los 2") | G-single **and G-cover** (names no summed key) | EUR 82,044.10 | 20317908 procedure VAL_TOTAL and its single RES-1 VAL_TOTAL (EUR 0.01 above S) | 0.000004 % | 28.7 % | **right / ok** |
| 20 | 7166738 | EUR 898,804.96 | 2 | LOT-1 ← 23664577 268,804.96; LOT-2 ← 23664044 630,000.00 | TED-VAL_TOTAL EUR 898,804.96 on 22450615 (F03, 1 result: LOT_NO 2, stored lot = null) | G-single **and G-cover** (its row is unkeyed) | EUR 630,000.00 | 22450615 procedure VAL_TOTAL | 0.00 % | 70.1 % | **right / ok** |

Under every guard set, the port's S, currency and n equal v2's. The ship flips to an R10 refusal only where a guard
removes the only vouch candidate:
- {single} and {single, cover}: all six;
- {cover}: 7651470, 6356382 and 7166738.

None of the six live heads serves a derivation (`value_derivation: null`). Each serves its max-elected `result_value`,
which the table shows.

## Per head

### 6815094: right / ok (Eesti Töötukassa, Estonian-language training, four lots)

- **Procedure:** a negotiated procedure with a call. CN 21103713 has four lots ("Hanke osa 1–4") and states no value.
- **The awards:**
  - F21 21254550 (2020-09) awards LOT_NO 4 (Reiting PR, 97,920.00), LOT_NO 3 (Atlasnet, 174,900.00) and LOT_NO 1
    (Mitteldorf, 213,180.00). Each lot has its own contract (7-6.3/20/0060, /0059, /0058). The notice's procedure
    VAL_TOTAL is 486,000.00, which is exactly these three.
  - F21 21285538 (2020-10) awards LOT_NO 2 (Atlasnet, 191,400.00, contract /0067). Its procedure VAL_TOTAL
    (PROCUREMENT_TOTAL) is **677,400.00**, which is 486,000 + 191,400.
- **The four F20s** (21571535, 21786612, 21812830, 22251256) only extend each contract's term because of COVID-19
  ("Lepingu kehtivuse … muutmine"). Each restates its lot's original VAL_TOTAL. Each carries VAL_TOTAL_AFTER 677,400;
  21571535 carries BEFORE 486,000, and the other three carry BEFORE 677,400.
- **Why right:** four distinct lots with four contracts. S takes each lot's latest row, and each latest row equals the
  lot's award. S equals the buyer's procedure total, stated in one award notice and four modification notices. The head
  without W is lot 2 alone (28 %).

### 6403567: right / ok (Große Kreisstadt Kitzingen, Friedrich-Bernbeck-Schule, building-services design)

- **Procedure:** a competitive negotiation. F02 CN 20215452 has 2 lots (HLS and Elektro/Aufzug) and no value.
- **The awards:** two F03s were dispatched on the same day (2019-07-11).
  - 20520516 awards LOT_NO 1 (Heizung, Lüftung, Sanitär) to Rennert Ingenieure: 142,000.00 from 2 tenders. Its
    procedure **VAL_TOTAL is 280,000.00**.
  - 20520354 awards LOT_NO 2 (Elektro und Aufzugsanlage) to Ingenieurbüro Pfister: 138,000.00 from 6 tenders. Its
    VAL_TOTAL is 138,000, its own lot.
- **Why right:** two distinct trades with two contractors. S = 142,000 + 138,000 is the buyer's stated total. It is not
  lot 1's own figure, and it is no coincidence. Both lots are HOAI fees commissioned in stages (LP 1–4 first). That is
  the same basis for both lots, so it is not a reason to doubt the sum. The head without W is lot 2 alone (49 %).

### 7651470: right / ok (Osakidetza, RALTEGRAVIR + SUGAMMADEX; G-single and G-cover)

- **Procedure:** a negotiated award without a call (exclusive marketing), F03 23006164 (2023/S 046-134404). It has two
  Lot sections (LOT_NO 1 RALTEGRAVIR, 2 SUGAMMADEX) and **one** LotResult, RES-1. That result has LOT_NO "Lotes 1 y2",
  CONTRACT_NO 1, MSD and 1 tender, with VAL_TOTAL EUR 5,667,313.10. The procedure VAL_TOTAL is the same figure. The
  store keeps the result on a lot = null row, and S does not sum it. This is the same shape as read_1's 7380025
  (Osakidetza too).
- **The F20s** restate each lot's original award in their LotResult. They do not change the award; S uses the restated
  value, not the modification values:
  - 23438905 (lot 2, 2023-09) restates LOT_NO 2 at 2,806,450.00 (CONTRACT_NO 2). The modification resolves lot 2 early
    because generics appeared, with VAL_TOTAL_BEFORE 2,806,450.00 and AFTER 1,368,315.50.
  - 23465553 (lot 1, 2023-10) restates LOT_NO 1 at 2,860,863.10. The modification is a 15 % price-deduction rule, with
    BEFORE = AFTER 2,860,863.10.
  - 23740527 (lot 1, 2024-01) restates 2,860,863.10. The modification is a one-year extension, with
    DESCRIPTION_PROCUREMENT.VAL_TOTAL and AFTER 5,721,726.20.
- **The figures:** 2,860,863.10 + 2,806,450.00 = 5,667,313.10, the 2023 award total to the cent. "Importe de
  adjudicación según precios unitarios" means the award is priced from unit prices, but each figure is a contract total
  (EUR 2.8 m per drug-year), not a unit rate. That is unlike M2's 6759797.
- **Why right:** two distinct medicines, each at its original award, with no pre- and post-modification mix. S equals the
  buyer's award total. The head without W is lot 1 alone (50.5 %).
- **Both guards misfire.** The one-result notice's total is a two-lot contract's total. Its result names no summed key
  only because it prints "Lotes 1 y2".

### 6937514: right / ok (Eesti Töötukassa, Tööharjutus, four lots)

- **Procedure:** a negotiated procedure with a call. CN 21364792 has 4 lots (Tallinn; Pärnu; Võru noored; Võru, missing
  or outdated qualification) and no value.
- **The awards:** four F21s, one per lot. Each states a **cumulative** procedure VAL_TOTAL:

  | notice | date | lot and winner | award | cumulative procedure VAL_TOTAL |
  |---|---|---|---|---|
  | 21465935 | 2021-01-05 | LOT_NO 1, Eesti Vaimse Tervise Ühing | 11,752.00 | 11,752.00 |
  | 21471905 | 2021-01-08 | LOT_NO 2, Febrec | 25,200.00 | 36,952.00 |
  | 21525584 | 2021-02-04 | LOT_NO 3, Projektiekspert | 34,080.00 | 71,032.00 |
  | 21530641 | 2021-02-08 | LOT_NO 4, Jako Koolitus | 17,700.00 | **88,732.00** |

- **The F20s:** 21536830 (lot 2, a correction of the contract data) and 21947679 (lot 3, a term change) restate their
  lots' awards. Both carry VAL_TOTAL_AFTER 88,732; their BEFORE values are 36,952 and 88,732.
- **Why right:** four distinct lots with four contracts (7-6.4/20/0046, /0047, 7-6.4/21/0005, /0006). The buyer's running
  total ends at exactly S. The vouching total is the last step of that running sum, not lot 4's own 17,700. The head
  without W is lot 3 alone (38 %).

### 6356382: right / ok (Stadt Rosenheim, Karolinen-Gymnasium earthworks and drainage; G-single and G-cover)

- **Procedure:** an open procedure. F02 CN 20114683 has LOT_NO 1 (Erdarbeiten DIN 18300) and LOT_NO 2
  (Abwasserleitungen). Its VAL_ESTIMATED_TOTAL is EUR 0.01, a placeholder; it is the head's 1-cent `estimated_value`
  amount, which the head does not elect.
- **The award:** F03 20317908 (2019-03) has one LotResult, RES-1. It has LOT_NO "Los 1 und Los 2", CONTRACT_NO 0294,
  A. Zehentbauer and 6 tenders, with VAL_TOTAL EUR 285,379.53. The procedure VAL_TOTAL is the same figure. One
  contract covers both lots, and the store keeps it on a lot = null row that S does not sum.
- **The 2023 F20s** are Nachträge (supplements) to contract 0294. Each LotResult restates that lot's share of the
  original award: lot 1 is 203,335.42 (23402733); lot 2 is 82,044.10 (23402794 and 23401003). The two shares add to
  285,379.52, one cent below the award total (rounding in the split). The supplements' own figures are modification
  kinds, and S does not use them:
  - lot 1, NA03: +9,373.77, BEFORE 232,757.22, AFTER 242,130.99;
  - lot 2, NA02: +7,595.37, 90,064.00 → 97,659.37;
  - lot 2, NA03: +1,520.92, 97,659.37 → 99,180.29.
- **Why right:** two distinct trades. Each lot's latest row is its original award share, and the shares partition the
  buyer's award total to within EUR 0.01. Nothing is double counted: the keyless 2019 row is not summed, and lot 2's
  two F20 rows restate one value. The head without W is lot 2 alone (29 %).
- **Both guards misfire,** for the same reason as 7380025 and 7651470.

### 7166738: right / ok (Hospital 12 de Octubre, Madrid, repair and maintenance services; G-single and G-cover)

- **Procedure:** an open procedure, 2021-0-111. F02 CN 21889789 has 2 lots of 8 months each, renewable by 16 more months.
  The VAL_OBJECTs are 1,013,848.23 and 1,890,000.00, and the VAL_ESTIMATED_TOTAL is 2,903,848.23. These estimates are
  net and include the renewals: lot 2's is 3 × 630,000.
- **The awards:**

  | notice | date | lot | contract | award VAL_TOTAL | other figures |
  |---|---|---|---|---|---|
  | 22306669 (F03) | 2022-04 | LOT_NO 1, UTE Lantania–Vareser | CR-WEB/2022/0000001243 | 325,254.00 | VAL_ESTIMATED_TOTAL 408,918.78 |
  | 22450615 (F03) | 2022-06 | LOT_NO 2, UTE "Conservación 12 de Octubre" | CR-WEB/2022/0000001870 | 630,000.00 | estimate 630,000; procedure **VAL_TOTAL 898,804.96** |
  | 22925424 (F20) | 2023-01 | lot 1, first 8-month extension | 0000001243 | restates 325,254.00 | BEFORE = AFTER 268,804.96 |
  | 22971695 (F20) | 2023-02 | lot 2, first 8-month extension | 0000001870 | restates 630,000.00 | BEFORE = AFTER 630,000 |
  | 23664577 (F03) | 2023-12 | LOT_NO 1, "2ª PRÓRROGA LOTE 1" | PRÓRROGA, concluded 2023-07-17 | 268,804.96 | |
  | 23664044 (F03) | 2023-12 | LOT_NO 2, "2ª PRÓRROGA LOTE 2" | PRÓRROGA | 630,000.00 | |

  The first-extension F20s say "en las mismas condiciones técnicas y económicas".
- **VAT basis:**
  - 325,254.00 = 268,804.96 × 1.21. Its estimate is 408,918.78 = (1,013,848.23 / 3) × 1.21. So the 2022 lot-1 award notice
    alone printed gross (21 % IVA) figures.
  - Every other notice, including the buyer's procedure total in 22450615, carries lot 1 at the net 268,804.96.
  - Lot 2's 630,000 is net: the CN's net 1,890,000 is 3 × 630,000.
  - So S is net throughout. It is not a VAT mix; it would be one only if it used the 2022 lot-1 row.
- **Why right:** two distinct lots, each at its latest award. The 2023 "prórroga" award values equal each lot's net
  per-period contract value, as the extensions run "in the same economic conditions". So S equals the buyer's own award
  total for the procedure, 268,804.96 + 630,000. The head without W is lot 2 alone (70 %).
- **Not the 6465510 shape:** T exceeds the vouching notice's own lot award (630,000) by exactly lot 1.
- **Weakest of the six, for the skeptic:**
  - The summed rows come from the second-extension notices, not the original awards. They coincide in value, so S does
    not change either way.
  - The full value with both extensions taken is up to 3 × S (EUR 2.70 m), near the CN's EUR 2.90 m estimate. S is the
    award total, as for every other head here: extension and modification values are excluded, as in read_1's 7380025.
- **G-cover misfire on a keying gap:** 22450615's result prints LOT_NO 2, which really is a summed lot. But both of that
  notice's Lot sections print LOT_NO 2 (section LOT-1 prints "2" too), so the store left the row unkeyed. That is why
  G-cover sees no summed key.

## Adversarial checks applied to every head (each found nothing)

- **Double count:** every summed key is a distinct printed LOT_NO with its own title and contract. Several lots have
  more than one row: 6815094 (each lot has 2 rows), 6937514 (LOT-2 and LOT-3), 7651470 (LOT-1), 6356382 (LOT-2) and
  7166738 (both lots). Those rows restate one value, except 7166738's lot-1 gross and net rows, and S takes the latest
  row for each lot. The keyless whole-contract rows of 7651470 and 6356382, and 7166738's unkeyed lot-2 row, are not
  summed.
- **Slip, VAT, currency:**
  - Every S equals a buyer-stated total to the cent (6356382: EUR 0.01 off, from the split), in EUR.
  - 7166738's one gross notice is not used, and S is net throughout.
  - No CN estimate contradicts a lot value. Four of the CNs state none; 6356382's EUR 0.01 is a placeholder; 7166738's
    estimates include the renewals.
- **Shared envelope / not_additive:**
  - There is no framework, DPS or multi-supplier lot.
  - 7651470's and 6356382's lots are each one contract, but the per-lot figures partition the contract total.
  - 7166738's lots are separate contracts with separate budgets.
  - 7651470's "precios unitarios" figure is a contract total, not a rate.
- **Vouch belongs to something else (6465510 shape):** in no case does T equal the vouching notice's own lot award
  (191,400 / 142,000 / 17,700 / 630,000; or, for the keyless rows, the whole contract, which is S). In each case T is the
  sum of every lot of the procedure.
- **Superseded vouch (5816379 shape):** no later notice re-awards a scope and states a different procedure total. The
  later notices state one of three things:
  - their own lot's value: 6403567 20520354; 7166738 23664577 and 23664044;
  - the same procedure total as VAL_TOTAL_AFTER: 6815094, 6937514;
  - modification values, which are excluded: 7651470, 6356382.
- **Farther:** S/T is ≤ 0.000004 % on all six, and the head without W covers 28–70 % of the procedure.

## What this says about the guards (recorded only; no rewording, per decision point 2)

- **G-single:** the six repeat read_1's shape in its two variants.
  - (a) A lot-per-notice award series in which one single-lot notice states the procedure's total: 6815094, 6403567,
    6937514 (the last as a running total) and 7166738.
  - (b) One multi-lot contract printed as a single keyless or multi-number LotResult, with per-lot F20 restatements
    summing to it: 7651470 and 6356382, like 7380025.

  In none does T equal the vouching notice's own lot award, which is what made 6465510 wrong. On these six reads,
  G-single costs 6 rights and removes no non-right.
- **G-cover:** it costs 3 rights here (7651470, 6356382, 7166738). Two are keyless multi-lot results ("Lotes 1 y2",
  "Los 1 und Los 2"). One is a result whose printed LOT_NO 2 is ambiguous across the notice's own sections, so it stays
  unkeyed.

## Requests (public API only; log `b6/u4/api_requests.log`, entries tagged `reader=read_3`)

34 GETs, all 200, with 0 × 408 and 0 retries. They were paced at least 2 s apart under the shared lock
(`b6/u4/.http.lock` + `.http.stamp`); the smallest gap across the whole log is 2.05 s. The fetcher is
`scratchpad/u4r3/get.py`, read_1's with the reader tag changed.
- 6 live heads: `/v1/tenders/{6815094, 6403567, 7651470, 6937514, 6356382, 7166738}`.
- 28 notices carrying the vouching totals, the summed awards and the other award or modification rows of each chain
  (`/v1/notices/{id}/content`):
  - 6815094: 21285538, 21254550, 21812830, 22251256, 21571535, 21786612;
  - 6403567: 20520516, 20520354;
  - 7651470: 23006164, 23740527, 23438905, 23465553;
  - 6937514: 21530641, 21465935, 21536830, 21947679, 21471905, 21525584;
  - 6356382: 20317908, 23402733, 23402794, 23401003;
  - 7166738: 22450615, 23664577, 23664044, 22306669, 22925424, 22971695.

Bodies are under `b6/u4/bodies/` (`tenders_{id}.json`, `notices_{id}_content.json`). No `/v1/sql`, POST, ssh, git or
cargo was used.

**Protocol note:** this file is one reader's read under M2. M2 pairs it with an independent adversarial skeptic per head
and sends any disagreement to a third reader. The checks above are this reader's own adversarial pass, not a substitute
for that skeptic. 7166738 is the head most worth the skeptic's time (see its last bullets).
