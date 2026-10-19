# 551 B6 U5 fresh pre-flip read, round a, part 4: P ranks 16–20

2026-10-19, reader `read_a_4`. These are ranks 16–20 of the fresh read's order over P minus every read id
(`b6/u5/order.json` sha256 3033ec31…, salt `551-b6-fresh-`, `sha256('551-b6-fresh-'+id)` ascending; `first_60[15:20]`
and the entries' ranks checked, order keys recomputed: 015971bc…, 01649b30…, 0172c178…, 017ed358…, 019cc6b6…):
**6530219, 8441074, 712964, 7872311, 6580960**. None is in `excluded_from_P` (every read id), none is in
`excluded_sources.json`, R_single, R_excess or U4's `read_set.json`, so all five are new reads.

**The question for each id:** P = v2+both (3,467 ships, `b6/u2/P.json`). Path W is OFF, so the live head is the
max-elected figure, not the sum. If the flip ships, the tender serves S, the summed lot awards, vouched by the
G-filtered `vouched_by`. Under M2's rubric, is serving S as the tender's value RIGHT, NEUTRAL or WRONG?

**Rubric (M2, wf_dc85fa7e-c00's):** right/ok means S is the procedure's value: distinct lots (TED `LOT-k` checked against
the printed LOT_NO), each lot's latest award, no lot counted twice, no ×10ᵏ slip, no VAT mix, and S no farther from the
procedure's stated total than the head it replaces. Wrong classes: `not_additive` (ceilings or maxima sharing one
envelope), `double_count`, `farther`, `slip`, `other` (for example, the vouching total belongs to something else, as in
6465510). Neutral means S is not wrong but not clearly better than the head without W. **Precedents applied:** read18
note 5 (per-lot framework ceilings whose sum is the buyer's own total are not `not_additive`); S1a (a ceiling copied per
supplier and summed per supplier is a double count) is checked on 8441074, where it does not occur.

**Inputs:** `b6/u5/order.json` (P sha256 5f8b7f7c…, ids_sha256 a2619fd9…, decisions.jsonl.gz 0add92e3…, as `order.json`
records; P.json and decisions.jsonl.gz re-hashed here and equal). The port-predicted S, currency, n, summed keys and
notices, and the G-filtered `vouched_by` come from `b6/u2/decisions.jsonl.gz` (v2+both). On all five, v2+both equals v2:
`g_removed`, `g_single_removed` and `g_cover_removed` are empty, `vouched_by` is the same under all four guard sets, and
`flags` is empty (no estimate-twin flag). Every vouch candidate has `g_single: false, g_cover: false`. Live heads come
from `GET /v1/tenders/{id}`. **All five live heads are JSON-equal to their U2 fresh-layer copies**
(`scratchpad/b6/tenders`, fetched 06:31Z–07:39Z), so the port's facts are current. **All 16 version notices** were
fetched live and are JSON-equal to their U2 notice-layer copies (`scratchpad/b6/notices`).

## Result

**5 right / 0 neutral / 0 wrong**, all class `ok`. In every head S equals, to the cent, a buyer-stated total that
covers every awarded lot of the procedure, and the vouching notice awards (or, for 8441074, sets out) every summed lot.
There is no partial sum in this part. 8441074 carries the part's only real trap: cascade frameworks with two
contractors per lot sharing one ceiling. S counts each lot's ceiling once, which is how the buyer counts it.

| rank | id | S (port, v2+both) | n | summed: key ← notice, award | vouched_by (G-filtered = v2) | head without W (live) | buyer-stated total(s) matching S | S/T | head/S | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| 16 | 6530219 | HUF 464,332,787.00 | 5 | LOT-1 ← 21027842 64,485,109; LOT-2 ← 20816834 248,867,609; LOT-3 ← 20765679 87,999,569; LOT-4 ← 20765679 7,088,000; LOT-5 ← 20765679 55,892,500 | TED-VAL_TOTAL HUF 464,332,787 on 20765679 (F03, 5 results, names all 5 keys) | HUF 64,485,109.00 (lot 1) | 20765679 procedure VAL_TOTAL = TED-VALUE (PROCUREMENT_TOTAL) 464,332,787 | 0.00 % | 13.9 % | **right / ok** |
| 17 | 8441074 | EUR 25,689,400.00 | 3 | LOT-1 ← 21409607 12,621,400; LOT-2 ← 22175851 5,082,000; LOT-3 ← 21409607 7,986,000 | TED-VAL_ESTIMATED_TOTAL EUR 25,689,400 on CN 20567519 (F02, estimate kind, 3 Lot sections, all in S) | EUR 5,082,000.00 (lot 2) | CN 20567519 VAL_ESTIMATED_TOTAL 25,689,400; also 21409607 VAL_TOTAL 20,607,400 (lots 1+3) + 22175851 / 21818664 VAL_TOTAL 5,082,000 (lot 2) | 0.00 % | 19.8 % | **right / ok** |
| 18 | 712964 | EUR 1,140,120.00 | 5 | LOT-0001…0005 ← 24751505: 156,160; 226,336; 299,872; 244,512; 213,240 | BT-118-NoticeResult EUR 1,140,120 on 24751505 (5 results, names all 5 keys) | **null** | 24751505 BT-118 1,140,120.00 | 0.00 % | — (null) | **right / ok** |
| 19 | 7872311 | PLN 337,651.20 | 2 | LOT-1 ← 23660402 294,840.00; LOT-2 ← 23660402 42,811.20 | TED-HIGH PLN 337,651.20 on 23660402 (F03, Range kind; LOW = HIGH; 2 results, names both keys) | PLN 294,840.00 (lot 1) | 23660402 VALUE_RANGE (PROCUREMENT_TOTAL) LOW = HIGH 337,651.20 | 0.00 % | 87.3 % | **right / ok** |
| 20 | 6580960 | EUR 116,530.00 | 3 | LOT-1 ← 20835362 9,980.00; LOT-3 ← 20835362 37,500.00; LOT-4 ← 20835362 69,050.00 | TED-VAL_TOTAL EUR 116,530 on 20835362 (F03, 4 results incl. LOT-2 unsuccessful; names LOT-1, LOT-3, LOT-4) | EUR 9,980.00 (lot 1, lot-null F20 row of 22785582) | 20835362 procedure VAL_TOTAL = TED-VALUE (PROCUREMENT_TOTAL) 116,530.00 | 0.00 % | 8.6 % | **right / ok** |

S_eur (`order.json`): 6530219 EUR 1,313,232.61, 8441074 EUR 25,689,400.00, 712964 EUR 1,140,120.00, 7872311
EUR 77,746.07, 6580960 EUR 116,530.00. None is a band head (B_P, ≥ EUR 100 m). None is within 2 % of EUR 100 m
(`band_set.json` lists 899272 and 5842187 only), and none is in `band_set.json`. None is in the provisional L_changed
(`b6/u2/l_changed_provisional.json`: criterion (a) and (b) id lists are empty; these ids appear there only as
fetch-time keys) or in the 545/571 sets. So decision point 8 (B-band) gets nothing from this part. No live head serves a
derivation (`value_derivation: null` on all five). 8441074 is the only head in EUR 10 m–100 m.

## Per head

### 6530219: right / ok (Magyar Posta and subsidiaries, insurance services 2019, 5 lots)

- **Procedure:** CN F02 20471198 (2019/S 114-279929), a competitive negotiation (PR_PROC B), joint procurement by
  Magyar Posta and five subsidiaries, five lots, 24 months each, no value stated anywhere in the CN: LOT_NO 1 property
  and liability, 2 fleet compulsory motor liability (5,120 vehicles), 3 fleet CASCO (740 vehicles), 4 D&O, 5 group life
  and accident.
- **Award:** F03 20765679 (2019/S 238-584379). RES-1…RES-5 print LOT_NO 1…5 with CONTRACT_NO 1…5, all concluded
  2019-11-29: lot 1 UNIQA HUF 64,485,109 (4 tenders), lot 2 UNIQA 248,867,609 (4), lot 3 Groupama 87,999,569 (3), lot 4
  Allianz 7,088,000 (3), lot 5 Magyar Posta Életbiztosító 55,892,500 (7). Procedure **VAL_TOTAL HUF 464,332,787** = the
  sum. The INFO_ADD lists a different subset of joint buyers and tenderers per lot, so the five lots are separate
  contracts with separate parties.
- **Modifications (F20s), not used by S:** 20816834 (2020-01) changes lot 2's risk multiplier 0.79 → 0.62 but prints
  VAL_TOTAL_BEFORE = AFTER = 248,867,609; its LotResult restates lot 2 at 248,867,609. 21027842 (2020-05) adds
  graffiti/SRCC/environmental cover to lot 1 for HUF 200,000 a year: "a szerződés éves díja: 64 685 109 Ft-ra módosul"
  (BEFORE 64,485,109, AFTER 64,685,109); its LotResult restates lot 1's award at 64,485,109. The port takes each lot's
  latest award row (lot 1 ← 21027842, lot 2 ← 20816834, both restating the original value), so S has no
  pre/post-modification mix.
- **Why right:** five distinct insurance lines, five contracts, each lot taken once. S equals the buyer's procedure total
  to the forint. The head without W is lot 1 alone (13.9 %).
- **Note for the skeptic (no verdict impact):** the lot figures are **annual premiums**. Lot 1's F20 calls 64,685,109 the
  contract's "éves díja" (annual fee) and puts the original two-year contract value at about twice that; lots 2–5 were
  priced on "Éves díj" (annual fee). So S, the buyer's VAL_TOTAL and the head are all on one annual basis, and the
  24-month contracts are worth roughly 2 × S. No rubric class fits: the basis is the same on every lot and on the stated
  total, so nothing is mixed, and S is still the buyer's own procedure figure.

### 8441074: right / ok (European GNSS Agency / EUSPA, security monitoring services, 3 cascade framework lots)

- **Procedure:** CN F02 20567519 (2019/S 157-386434), open, framework. "The procurement shall be divided into 3 separate
  lots with the award of multiple framework contracts in cascade with maximum 2 contractors each lot", 1 year plus up to
  3 renewals (4 years max). Lot values (VAL_OBJECT): LOT_NO 1 Operational Support to GSMC EUR 12,621,400; 2 Operational
  Support to GSA Cyber Team 5,082,000; 3 Technical Support to GSMC 7,986,000. Procedure **VAL_ESTIMATED_TOTAL
  EUR 25,689,400** = the sum. Two F14 corrigenda (20689335, 20754164) only move the deadlines.
- **Awards:** F03 21409607 (2020/S 249-619204) awards lot 1 to Contractor No 1 RHEA and Contractor No 2 GMV + Thales
  (group), each at VAL_TOTAL = VAL_ESTIMATED_TOTAL 12,621,400, and lot 3 to the same pair, each at 7,986,000. Its procedure
  **VAL_TOTAL is EUR 20,607,400 = lot 1 + lot 3 once each**, not once per contractor. F03 21818664 (2021-07) awards lot 2
  to Contractor No 1 RHEA and F03 22175851 (2022-01) to Contractor No 2 CGI IT Czech Republic, each at 5,082,000 with
  procedure VAL_TOTAL 5,082,000.
- **Double-count trap, not taken (S1a):** the store keeps two rows per lot (one per cascade contractor), each carrying
  the lot's shared ceiling. Summing per contractor would give EUR 51,378,800. S takes one row per lot (lot 2 the latest,
  22175851; the earlier row is the same figure), so it counts each lot's ceiling once, exactly as the buyer's 2020
  VAL_TOTAL does.
- **Why right:** three distinct service lots; each lot's figure is that lot's own framework ceiling, shared by its two
  cascade contractors and counted once. S equals the buyer's CN estimate to the cent and also equals the buyer's two award
  totals added (20,607,400 + 5,082,000). The ceilings partition the procedure total rather than sharing one envelope, so
  this is read18 note 5's basis, not `not_additive`. The vouch is estimate-kind from a three-section CN whose lots are all
  in S, so the estimate-twin flag does not apply. The head without W is lot 2 alone (19.8 %).

### 712964: right / ok (Haven van Antwerpen-Brugge, conservation grazing in the port area, 5 lots)

- **Procedure:** an eForms CAN (subtype 30, `00319890-2025`, can-standard, utilities directive 2014/25, negotiated with a
  call), the only notice in the chain. "Rasterbegrazing met schapen en geiten" in five geographic clusters, one lot each
  (BT-22 B11072bis_1…_5), each a framework without reopening of competition (BT-765 `fa-wo-rc`), 2024-01-05 to
  2032-01-04.
- **Results:** RES-0001…0005 each name their own lot (BT-13713). Johan De Meyst wins lots 1–4 and THV Collectief
  Schapenbegrazing Havengebied wins lot 5. Each result has its own tender (BT-720) and its own SettledContract
  (17682-31194 … 17686-31208, concluded 2024-01-29). Per lot BT-720 = BT-709 = BT-710 (the winner was the lowest tender):
  156,160 / 226,336 / 299,872 / 244,512 / 213,240. Notice-level **BT-118 = EUR 1,140,120.00**, the sum.
- **Why right:** five distinct clusters, one winner and one contract each, all in one notice. S equals the buyer's own
  BT-118 to the cent. The head without W is **null**, because the BT-709 partner-only rows and BT-118 are never elected
  (506), so S replaces no figure and cannot be farther. No double count: S sums BT-720 once per lot, and BT-709 equals
  BT-720 per lot rather than copying the whole total.

### 7872311: right / ok (COBORU, nitrogen fertiliser deliveries, 2 lots)

- **Procedure:** CN F02 23459570 (2023/S 200-627652), open, LOT_ALL, two lots and no value stated: LOT_NO 1 150 t of
  ammonium nitrate (96 t / 48 t / 6 t to three stations), LOT_NO 2 16 t of granulated ammonium sulphate plus 8 t of urea
  for SDOO Wrócikowo.
- **Award:** F03 23660402 (2023/S 232-727408). RES-1 awards LOT_NO 1 to CHEMIROL for PLN 294,840.00 (1 tender, contract
  11/2023). RES-2 awards LOT_NO 2 to CHEMIROL for PLN 42,811.20 (1 tender, contract 12/2023). The procedure total is printed
  as a VALUE_RANGE (PROCUREMENT_TOTAL) with **LOW = HIGH = PLN 337,651.20** = the sum. The parse emits LOW and HIGH twice
  each, all at the same figure, so the four vouches are one total.
- **Why right:** two distinct product lots and two contracts. With one tender per lot, the "lowest/highest offer" range
  collapses to the awarded total, and S equals it to the grosz. The Range vouch comes from a notice that names both summed
  keys, so G-cover keeps it. The prices are plausible (about PLN 1,966 per tonne of ammonium nitrate), so there is no
  slip. The head without W is lot 1 alone (87.3 %).

### 6580960: right / ok (CU Le Havre Seine Métropole, AMO for the Harfleur recycling centre, 4 lots)

- **Procedure:** CN F02 20576119 (2019/S 155-382728), open, four lots and no value stated: LOT_NO 1 contrôle technique
  (lump sum), 2 geotechnical studies (unit-price single-operator framework), 3 OPC and construction economics (lump
  sum), 4 environmental AMO/HQE (lump sum, firm plus optional tranche).
- **Award:** F03 20835362 (2020/S 007-011646). RES-1 awards LOT_NO 1 to Socotec, EUR 9,980.00 (2 tenders). RES-2 (LOT_NO
  2) is **PROCUREMENT_UNSUCCESSFUL** (served `clos-nw`, never awarded in this chain). RES-3 awards LOT_NO 3 to
  Architecture Sécurité Pilotage, EUR 37,500.00 (2 tenders). RES-4 awards LOT_NO 4 to Socotec Construction, EUR 69,050.00
  (1 tender); the INFO_ADD says lot 4's amount is the firm plus the optional tranche. Procedure **VAL_TOTAL EUR 116,530.00**
  = lot 1 + lot 3 + lot 4 exactly.
- **F20 22785582 (2022-11), not used by S:** avenant no. 2 to lot 1 adds an air-permeability test (VAL_TOTAL_BEFORE 700.00,
  AFTER 11,660.00). Its single LotResult prints LOT_NO 1 and restates the original award, 9,980.00. The store keeps that
  row on **lot = null**, so S does not sum it, and today's head serves it.
- **Why right:** three distinct awarded trades, one award each. Lot 2 was declared unsuccessful and has no award, so
  this is not a partial sum: S is the whole awarded procedure and equals the buyer's stated total. Lot 4 includes its
  optional tranche on both sides (the award and the buyer's total). No pre/post-modification mix: lot 1 is taken at its
  award value, which the F20 restates. The head without W is lot 1 alone (8.6 %).

## Adversarial checks applied to every head (each found nothing)

- **Double count:** every summed key is a distinct printed LOT_NO (TED) or BT-13713 lot (eForms) with its own title.
  Where a lot has several rows (6530219 LOT-1 and LOT-2 restated in F20s; 8441074 two cascade contractors per lot), the
  rows carry the same value, and S takes one. 8441074's per-contractor copies are not summed. The buyer's own
  21409607 total confirms that the ceiling is counted once per lot. No lot-null row restates a keyed award that S
  also takes (6580960's lot-null F20 row is not summed).
- **Slip, VAT, currency:** every S equals a buyer-stated total to the cent, in the notices' own currency (HUF, EUR,
  PLN). Where a CN gives lot values (8441074), they equal the awards. No figure is 10ᵏ off any sibling. 6530219's lots
  are all annual premiums, so its basis is consistent; it is noted above, not a mix.
- **Shared envelope / not_additive:** 8441074's lot ceilings are per lot (each shared by that lot's two cascade
  contractors, counted once) and partition the buyer's total. 712964 and 7872311 have one winner and one contract per
  lot. 6530219 has five separately contracted insurance lines. There is no unit-rate sum and no nested lot.
- **Vouch belongs to something else (6465510 shape):** every vouching total comes from a notice that awards (or, for
  8441074, sets out the estimate for) every summed lot. None equals one result's own value for a lot outside S. All five
  are kept by G-single and G-cover alike.
- **Superseded vouch (5816379 shape):** no later award notice states a different total for the summed lots. The later
  notices are 6530219's two F20s (single-lot modifications restating the award), 8441074's lot-2 award to cascade
  contractor no. 2 (same ceiling, procedure total 5,082,000 for lot 2 only) and 6580960's F20 (a single-lot
  modification). None re-awards a summed lot at a new value.
- **Farther:** S/T = 0.00 % on all five. The heads without W are null (712964) or one lot's figure (6530219 13.9 %,
  8441074 19.8 %, 7872311 87.3 %, 6580960 8.6 % of S).

## Requests (public API only; log `b6/u5/api_requests.log`, entries tagged `reader=read_a_4`)

21 GETs, all 200, 0 × 408, 0 retries. Request starts were paced at least 2.2 s apart under the shared lock
(`b6/u5/.http.lock` + `.http.stamp`), interleaved with the other round-a readers:
- 5 live heads: `/v1/tenders/{6530219, 8441074, 712964, 7872311, 6580960}`;
- 16 notices, every version notice of the five chains: 20471198, 20765679, 20816834, 21027842; 20567519, 20689335,
  20754164, 21409607, 21818664, 22175851; 24751505; 23459570, 23660402; 20576119, 20835362, 22785582
  (`/v1/notices/{id}/content`).

Bodies are under `b6/u5/bodies/` (`tenders_{id}.json`, `notices_{id}_content.json`). Within the repo, only `b6/u5/`
(this file, the bodies and the log lines) was written. The fetch and dump helpers were session scratchpad files
(`scratchpad/u5a4/`) outside the repo. There was no /v1/sql, ssh, POST, git or cargo.

**Protocol note:** this file is one reader's read under M2. M2 pairs it with an independent adversarial skeptic per head
and sends any disagreement to a third reader. The checks above are this reader's own adversarial pass, not a substitute
for that skeptic. For the skeptic, the points most worth pressing are 8441074's cascade ceilings (that S counts each lot
once, as the buyer's 21409607 total does) and 6530219's annual-premium basis.
