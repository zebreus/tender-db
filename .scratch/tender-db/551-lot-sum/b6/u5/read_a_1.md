# 551 B6 U5 fresh pre-flip read, round a, part 1: P ranks 1–5

2026-10-19, reader `read_a_1`. These are the first five ids of the fresh read's order over P minus every read id
(`b6/u5/order.json`, salt `551-b6-fresh-`, `sha256('551-b6-fresh-'+id)` ascending; `first_60[:5]` and ranks 1–5 checked,
order keys recomputed): **77882, 6471017, 285956, 7021235, 7429588**. None is in `excluded_from_P` (every read id), so
all five are new reads.

**The question for each id:** P = v2+both (3,467 ships, `b6/u2/P.json`). Path W is OFF, so the live head is the
max-elected figure, not the sum. If the flip ships, the tender serves S, the summed lot awards, vouched by the
G-filtered `vouched_by`. Under M2's rubric, is serving S as the tender's value RIGHT, NEUTRAL or WRONG?

**Rubric (M2, wf_dc85fa7e-c00's):** right/ok means S is the procedure's value: distinct lots (TED `LOT-k` checked against
the printed LOT_NO), each lot's latest award, no lot counted twice, no ×10ᵏ slip, no VAT mix, and S no farther from the
procedure's stated total than the head it replaces. Wrong classes: `not_additive` (ceilings or maxima sharing one
envelope), `double_count`, `farther`, `slip`, `other` (for example, the vouching total belongs to something else, as in
6465510). Neutral means S is not wrong but not clearly better than the head without W. **Precedent applied:** read18 note 1
counts a partial sum as right when S equals a buyer-stated total exactly, its lots are distinct real awards, and it is
much closer to the whole-procedure figure than the head (7398935 was "the weakest right": 2 of 3 lots). read18 note 5:
per-lot single-operator framework ceilings whose sum is the buyer's own total are not `not_additive`.

**Inputs:** `b6/u5/order.json` (P sha256 5f8b7f7c…, ids_sha256 a2619fd9…, decisions.jsonl.gz 0add92e3…, as
`order.json` records). The port-predicted S, currency, n, summed keys and notices, and the G-filtered `vouched_by` come
from `b6/u2/decisions.jsonl.gz` (v2+both). On all five, v2+both equals v2: `g_removed` is empty, `vouched_by_moved_from_v2`
is false, and the flags are empty (no estimate-twin flag). Live heads come from `GET /v1/tenders/{id}`. **All five live
heads are JSON-equal to their U2 fresh-layer copies** (`scratchpad/b6/tenders`, fetched 06:17Z–07:31Z), so the port's
facts are current. **All 10 version notices** were fetched live and are JSON-equal to their U2 notice-layer copies.

## Result

**5 right / 0 neutral / 0 wrong**, all class `ok`. In four heads S equals a buyer-stated total that covers every lot
of the procedure. The fifth, 7429588, is a partial sum in read18's 7398935 shape: S equals the first award notice's
procedure total for lots 1 and 3, and lot 2's later re-award sits on a lot-null row that is not summed. S is 78 % of the
whole procedure, against 22 % for the head. It is the weakest of the five.

| rank | id | S (port, v2+both) | n | summed: key ← notice, award | vouched_by (G-filtered = v2) | head without W (live) | buyer-stated total(s) matching S | S/T | head/S | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 77882 | EUR 4,521,000.00 | 9 | LOT-0001…0009 ← 25504477: 90,000; 95,000; 250,000; 2,500,000; 100,000; 200,000; 480,000; 780,000; 26,000 | BT-118-NoticeResult EUR 4,521,000 on 25504477 (9 results, names all 9 keys) | **null** | 25504477 BT-118 4,521,000.00 | 0.00 % | — (null) | **right / ok** |
| 2 | 6471017 | EUR 1,078,485.00 | 2 | LOT-1 ← 21256163 689,108.00; LOT-2 ← 21256163 389,377.00 | TED-VAL_ESTIMATED_TOTAL EUR 1,078,485 on CN 20349868 (F05, estimate kind) | EUR 689,108.00 | CN 20349868 VAL_ESTIMATED_TOTAL = TED-VALUE (ESTIMATED_TOTAL) 1,078,485.00 | 0.00 % | 63.9 % | **right / ok** |
| 3 | 285956 | EUR 4,477,729.39 | 7 | LOT-0001…0007 ← 25517196: 2,297,601.90; 82,684.00; 253,794.39; 389,612.85; 985,697.65; 250,304.60; 218,034.00 | BT-118-NoticeResult EUR 4,477,729.39 on 25517196 (7 results, names all 7 keys) | **null** | 25517196 BT-118 4,477,729.39 | 0.00 % | — (null) | **right / ok** |
| 4 | 7021235 | HUF 119,444,475.00 | 2 | LOT-1 ← 21978620 58,136,250; LOT-2 ← 22266151 61,308,225 (21978620 states the same) | TED-VAL_TOTAL HUF 119,444,475 on 21978620 (F03, 2 results, names both keys) | HUF 61,308,225.00 | 21978620 procedure VAL_TOTAL = TED-VALUE (PROCUREMENT_TOTAL) 119,444,475 | 0.00 % | 51.3 % | **right / ok** |
| 5 | 7429588 | EUR 266,078.95 | 2 | LOT-1 ← 22628724 191,280.95; LOT-3 ← 22628724 74,798.00 | TED-VAL_TOTAL EUR 266,078.95 on 22628724 (F03, 3 results incl. LOT-2 unsuccessful; names LOT-1, LOT-3) | EUR 73,188.24 (lot 2's re-award, lot-null row of 22628772) | 22628724 procedure VAL_TOTAL = TED-VALUE (PROCUREMENT_TOTAL) 266,078.95 | 0.00 % | 27.5 % | **right / ok** (partial sum) |

S_eur: 77882 EUR 4.52 m, 6471017 EUR 1.08 m, 285956 EUR 4.48 m, 7021235 EUR 319,472.76, 7429588 EUR 266,078.95. None of
the five is a band head (B_P, ≥ EUR 100 m). None is within 2 % of EUR 100 m, none is in `band_set.json`, and none is in
the provisional L_changed (239 ids; ∩ 3,502 = 0) or in the 545/571 sets. So decision point 8 (B-band) gets nothing from
this part. No live head serves a derivation (`value_derivation: null` on all five).

## Per head

### 77882: right / ok (Golfe du Morbihan – Vannes agglomération, déchèterie waste treatment, 9 lots)

- **Procedure:** an eForms CAN (subtype 29, `00027219-2026`, the only notice in the chain), open procedure 2025-056.
  It has nine Lot sections with distinct internal ids 2025-056/01…/09: inert waste for geographic sectors 1, 2 and 3
  (RHUYS, CENTRE, EXTERIEURS), non-recoverable waste, asbestos, wood, cardboard, metals and batteries. Each LotResult
  RES-0001…0009 names its own lot (BT-13713). Its BT-709 framework maximum equals the winning tender's BT-720. Each
  result points to its own SettledContract, with distinct buyer contract ids 2026S00152–157 and 2026S00321–323.
  Notice-level **BT-118 = EUR 4,521,000.00**.
- **Why right:** nine distinct lots, one award each, all in one notice. S equals the notice's own framework total to the
  cent. The head without W is **null**: the BT-709 partner-only rows and BT-118 are never elected (506). So S replaces no
  figure, cannot be farther, and gives value filters the buyer's own total.
- **Note for the skeptic (no verdict impact):** lots 7, 8 and 9 are titled "(recettes)" (cardboard, metals, batteries).
  These are resale lots, where the material is taken against a price. Their 480,000 / 780,000 / 26,000 may be revenue to
  the buyer rather than spend. The buyer's own BT-118 includes them, and S reproduces that published total exactly. No
  rubric class fits: there is no shared envelope (each lot has its own contract and maximum), no double count, no slip,
  and the vouch belongs to this procedure. Without the three resale lots the figure would be EUR 3,235,000. I record it
  as a shape, not a wrong.

### 6471017: right / ok (Acquedotto Pugliese, maintenance of water and sewage lifting plants, STO AV/FG)

- **Procedure:** CN F05 20349868 (2019/S 068-161026), open procedure, an **accordo quadro con un unico operatore** per
  lot, in two lots with distinct CIGs. LOT_NO 1 covers AET Foggia (CIG 78570968BA, VAL_OBJECT 689,108.00, "importo
  complessivo presunto … posto a base d'asta"). LOT_NO 2 covers AET San Severo (CIG 785709798D, VAL_OBJECT 389,377.00).
  Procedure **VAL_ESTIMATED_TOTAL EUR 1,078,485.00**.
- **Award:** F06 21256163 (2020/S 173-419079). RES-1 (LOT_NO 1) and RES-2 (LOT_NO 2) each have VAL_TOTAL =
  VAL_ESTIMATED_TOTAL at the lot's base amount: 689,108.00 and 389,377.00. Both were awarded to the same group (RTI Cagno
  srl, RDR srl and PFT Palladino Fabrizio; AWARDED_TO_GROUP), which is one operator per lot, as the CN requires, not
  parallel suppliers. The F06 states no procedure total.
- **Why right:** two distinct territorial lots, one award each. S is the sum of two per-lot single-operator framework
  ceilings and equals the buyer's own procedure estimate to the cent (read18 note 5 basis: each ceiling belongs to one lot
  only). The vouch is estimate-kind, from a two-section CN whose lots are both in S, so the estimate-twin flag does not
  apply. The head without W is lot 1 alone (63.9 %).

### 285956: right / ok (Commune d'Épinal, Basilique Saint-Maurice, restoration of the choir, 7 trades)

- **Procedure:** an eForms CAN (subtype 29, `00003819-2026`, can-standard, open procedure 25VE022, works), the only
  notice in the chain. It has seven Lot sections, one per trade: LOT_NO 1 scaffolding and masonry, 2 sculpture, 3
  carpentry, 4 roofing, 5 stained glass, 6 stained-glass ironwork, 7 joinery. Each has one winner (PIANTANIDA, SOCRA,
  MADDALON FRERES, COANUS, ATELIER BAUDOIN-MONIQUE, HUGON METAL DESIGN, France-Lanord et Bichaton) and its own contract
  (2025-04001/1 … 2025-04007/7). BT-720 per lot: 2,297,601.90 / 82,684.00 / 253,794.39 / 389,612.85 / 985,697.65 /
  250,304.60 / 218,034.00. Their sum is exactly **BT-118 EUR 4,477,729.39**.
- **Double-count trap, not taken:** every RES-0001…0007 carries **BT-709 = 4,477,729.39**, the whole total copied onto each
  lot. The store keeps these as `result_framework_maximum` on each lot; 7 × would be EUR 31.3 m. S sums the per-lot
  awarded values (BT-720), not the copied BT-709, so it equals the total once.
- **Why right:** seven distinct trades and seven contracts, each awarded once. S equals the buyer's total to the cent.
  The lots carry BT-765 `fa-mix`, but each lot has a single winner, so no envelope is shared. The head without W is
  **null**, so S cannot be farther.

### 7021235: right / ok (Felső-Tisza-vidéki Vízügyi Igazgatóság, Tisza-Túr reservoir maintenance machines)

- **Procedure:** CN F02 21565417 (2021/S 045-110416), open procedure, two lots: LOT_NO 1 "Rézsűkasza és tartozékai"
  (2 slope mowers with adapters) and LOT_NO 2 "Mezőgazdasági vontató és tartozékai" (2 tractors with adapters and a
  trailer). Neither the CN nor its lots state a value. Price is evaluated as "egyösszegű nettó ajánlati ár (opció
  nélkül)": net, options excluded.
- **Award:** F03 21978620 (2021/S 201-523099). RES-1 awards LOT_NO 1 to Unikomtech Kft. for HUF 58,136,250 (3 tenders,
  contract 1). RES-2 awards LOT_NO 2 to Városkert Kft. for HUF 61,308,225 (4 tenders, contract 2). Procedure **VAL_TOTAL
  HUF 119,444,475** = the sum.
- **F20 22266151** (2022/S 051-132868) modifies the lot-2 contract: equivalent products replace discontinued ones (trailer,
  front disc mower) and acceptance is staged. **VAL_TOTAL_BEFORE = AFTER = HUF 61,308,225**; its LotResult restates lot 2
  at the same value. S takes lot 2 once, at that value. There is no pre/post-modification mix, because the value did not
  change.
- **Why right:** two distinct machine lots and two contracts. S equals the buyer's procedure total to the forint. Both
  the lot values and the total are net contract values without options, so the bases are consistent. The head without W
  is lot 2 alone (51.3 %).

### 7429588: right / ok, partial sum (Amt Schönberger Land, school IT for Selmsdorf and Schönberg)

- **Procedure:** CN F02 22531880 (2022/S 125-352431), open procedure, LOT_ALL, three lots and no value stated: LOT_NO 1
  interactive displays, 2 Netzwerk/WLAN, 3 other hardware and software.
- **First award:** F03 22628724 (2022/S 184-519172). RES-1 awards LOT_NO 1 to Hannelore Glagla Büro-Organisation GmbH
  for EUR 191,280.95 (1 tender). RES-2 for LOT_NO 2 is **PROCUREMENT_UNSUCCESSFUL** (no award; served `clos-nw`). RES-3
  awards LOT_NO 3 to Albacon Systemhaus GmbH for EUR 74,798.00 (3 tenders). Procedure **VAL_TOTAL EUR 266,078.95** =
  lot 1 + lot 3 exactly.
- **Lot 2 re-award:** F03 22628772 (2022/S 184-519220, same dispatch date) refers to the same CN as THIS_PROCEDURE. It
  is a competitive negotiation for "Netzwerk/WLAN" (NO_LOT_DIVISION, one Lot section ITEM 1 with no LOT_NO). Its single
  LotResult prints LOT_NO 2 and awards IT Point MV GmbH & Co. KG EUR 73,188.24 (2 tenders), with procedure VAL_TOTAL
  73,188.24. The store keeps this result on a **lot = null** row, so S does not sum it, and today's head serves it.
- **Why right (read18 note 1 precedent):** S equals a buyer-stated procedure total exactly. Its two lots are distinct
  real awards, each the lot's only award. The whole procedure is 191,280.95 + 74,798.00 + 73,188.24 = **EUR 339,267.19**.
  S covers 78.4 % of it and the head 21.6 %, so S is clearly nearer the procedure's value. It is the same shape as
  7398935 ("S covers 2 of 3 lots and today's head covers the third"), with a wider margin (S −22 % against head −78 %;
  7398935 was −44 % against −56 %). Not neutral, because S is clearly better than the head. Not `farther`. Not
  `double_count`: LOT-2's only keyed row is the unsuccessful RES-2.
- **Fragility, for the standing guard (decision point 12; no verdict impact):** 7429588 is a partial-sum head that is
  **not** on the pin list (6679375, 7153588, 7398935, 7722197, 6779279, 6850401, 7698260). A single-Lot-section
  placement drain that keys 22628772's result (printed LOT_NO 2) onto LOT-2 would make S the full EUR 339,267.19. No stated
  total equals that (only 266,078.95 and 73,188.24, a disjoint pair R10 does not add), so R10 would refuse and the head
  would fall back to one figure. It would not become a double count.

## Adversarial checks applied to every head (each found nothing)

- **Double count:** every summed key is a distinct printed LOT_NO (TED) or BT-13713 lot (eForms), each with its own
  title. 7021235's LOT-2 has two rows (CAN and F20) at the same value, and S takes one. 285956's copied BT-709 totals are
  not summed. No lot-null row restates a keyed award.
- **Slip, VAT, currency:** every S equals a buyer-stated total to the cent, in the notice's own currency (EUR or HUF).
  Where a CN gives lot values (6471017), they equal the awards. No figure is 10ᵏ off any sibling figure. 7021235's
  values are net, without options, on both sides.
- **Shared envelope / not_additive:** 6471017 sums per-lot single-operator framework ceilings (one per CIG and area),
  which read18 note 5 accepts. 77882 and 285956 have one winner and one contract per lot. There is no multi-supplier lot,
  unit rate or nested lot.
- **Vouch belongs to something else (6465510 shape):** every vouching total comes from a notice that awards (or, for
  6471017, estimates) every summed lot. None equals one result's own value for a lot outside S. All five are kept by
  G-single and G-cover alike.
- **Superseded vouch (5816379 shape):** no later award notice restates a different total for the summed lots. The later
  notices are 7021235's F20, at the same lot-2 value with no procedure total, and 7429588's 22628772, which awards a lot
  outside S. Neither re-awards a summed lot.
- **Farther:** S/T = 0.00 % on all five. The heads without W are null (77882, 285956) or a single lot's figure (6471017
  63.9 %, 7021235 51.3 %, 7429588 27.5 % of S).

## Requests (public API only; log `b6/u5/api_requests.log`, entries tagged `reader=read_a_1`)

15 GETs, all 200, 0 × 408, 0 retries. Request starts were paced at least 2.2 s apart under the shared lock
(`b6/u5/.http.lock` + `.http.stamp`, used alongside `read_a_2`):
- 5 live heads: `/v1/tenders/{77882, 6471017, 285956, 7021235, 7429588}`;
- 10 notices, every version notice of the five chains: 25504477; 20349868, 21256163; 25517196; 21565417, 21978620,
  22266151; 22531880, 22628724, 22628772 (`/v1/notices/{id}/content`).

Bodies are under `b6/u5/bodies/` (`tenders_{id}.json`, `notices_{id}_content.json`). Nothing outside `b6/u5/` was written.
There was no /v1/sql, ssh, POST, git or cargo.

**Protocol note:** this file is one reader's read under M2. M2 pairs it with an independent adversarial skeptic per head
and sends any disagreement to a third reader. The checks above are this reader's own adversarial pass, not a substitute
for that skeptic. For the skeptic, the two points most worth pressing are 77882's three "(recettes)" lots and 7429588's
partial sum.
