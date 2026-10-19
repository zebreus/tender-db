# 551 B6 U5 fresh pre-flip read, round a, part 2: ranks 6–10

2026-10-19, reader `read_a_2`. These are ranks 6–10 of the fresh read's order (`b6/u5/order.json` sha256 3033ec31…,
`sha256('551-b6-fresh-' + id)` ascending over P minus every read id): **7763513, 5733, 6570707, 6660753, 5663715**.
All five are v2+both ships in P (`b6/u2/P.json` 5f8b7f7c…, guard set `v2+both`) and none is in `excluded_from_P`, so none
has a verdict from an earlier recorded read. All five are new reads.

**The question for each id:** P ships this id (v2 + G-single + G-cover). Under M2's rubric, is serving S (the summed lot
awards, vouched by the G-filtered stated total) RIGHT, NEUTRAL or WRONG as the tender's value?

**Rubric (M2, wf_dc85fa7e-c00's, as U4 applied it):** right/ok means S is the procedure's value: distinct lots (TED
`LOT-k` checked against the printed LOT_NO), each lot's latest award, no lot counted twice, no ×10ᵏ slip, no VAT mix, and S
is no farther from the procedure's stated total than the head it replaces. Wrong classes: `not_additive` (ceilings or
maxima sharing one envelope), `double_count`, `farther`, `slip`, `other` (for example, the vouching total belongs to
something else, as in 6465510). Neutral means S is not wrong but not clearly better than the head without W.

**Inputs:** port-predicted S, currency, n, summed keys ← notice ids, and the G-filtered `vouched_by` come from
`b6/u2/decisions.jsonl.gz` (0add92e3…, record `decisions["v2+both"]`, equal to `P.json`'s ship row and `order.json`'s
entry for each id). Live heads come from `GET /v1/tenders/{id}`. Path W is OFF, so the live head is the max-elected figure
(`value_derivation: null` on all five), not S. **Every live head is JSON-equal to its U2 fresh-layer copy**
(`scratchpad/b6/tenders`, fetched 06:15Z–07:37Z), and **every one of the 21 version notices was fetched live and is
JSON-equal to its U2-layer copy** (`scratchpad/b6/notices`), so the port's facts are still current. Estimate-twin flags
(decision point 7): none on these five (`flags: []`). The two estimate vouches (7763513, 6570707) come from CNs with one
Lot section per summed lot, not from a single-lot-section CN.

## Result

**5 right / 0 neutral / 0 wrong**, all class `ok`. In every case S equals a buyer-stated total to the cent (or the
forint/grosz), every summed key is a distinct printed lot with its own award, and the head without W is a single lot's
figure (5–80 % of the procedure).

| rank | id | S (port) | n | summed: key ← notice | P vouched_by (G-filtered) | head without W (live) | buyer-stated total(s) matching S | S/T | head/S | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| 6 | 7763513 | BGN 200,000.00 | 2 | LOT-1 ← 23601088 170,000.00; LOT-2 ← 23701706 30,000.00 | TED-VAL_ESTIMATED_TOTAL BGN 200,000 on CN 23214436 (F02, 2 Lot sections, both summed) | BGN 30,000.00 (lot 2) | CN 23214436 VAL_ESTIMATED_TOTAL = lot estimates 170,000 + 30,000; each CAN's lot award = its lot estimate | 0.00 % | 15.0 % | **right / ok** |
| 7 | 5733 | PLN 785,837.16 | 21 | LOT-0001…0017, 0019…0021 ← 24561226 (20 lots, 563,765.36); LOT-0018 ← 24758276 222,071.80 | BT-161 PLN 785,837.16 on 24561226 (20 results; names 20 of the 21 summed keys) | PLN 222,071.80 (lot 18) | BT-161 on 24561226 = all 21 lots, lot 18 included | 0.00 % | 28.3 % | **right / ok** |
| 8 | 6570707 | HUF 459,995,328.00 | 2 | LOT-1 ← 22484360 365,752,608; LOT-2 ← 22521344 94,242,720 (both F20 restatements of CAN 20808119's awards) | TED-VAL_ESTIMATED_TOTAL HUF 459,995,328 on CN 20595292 (then TED-VAL_TOTAL on CAN 20808119, same figure) | HUF 365,752,608.00 (lot 1) | CN estimate total; CAN 20808119 procedure VAL_TOTAL | 0.00 % | 79.5 % | **right / ok** |
| 9 | 6660753 | HUF 1,381,739,164.00 | 20 | 16 lots ← CAN 21475284; LOT-4/5/6/7 ← F20s 22510683 / 22984840 / 22975660 / 23029439 (restating the CAN's awards) | TED-VAL_TOTAL HUF 1,381,739,164 on CAN 21475284 (F03, 26 results, names all 20 summed keys) | HUF 65,132,339.00 (lot 7) | CAN 21475284 procedure VAL_TOTAL = the 20 awarded lots | 0.00 % | 4.7 % | **right / ok** |
| 10 | 5663715 | EUR 10,300,000.00 | 3 | LOT-1 4,500,000; LOT-2 4,000,000; LOT-3 1,800,000, all ← CAN 19055486 | TED-HIGH EUR 10,300,000 on CAN 19055486 (F03, 3 results, names all 3 keys; Range kind) | EUR 4,500,000.00 (lot 1) | CAN procedure range LOW 3,500,000 / HIGH 10,300,000 (PROCUREMENT_TOTAL); CN lot VAL_OBJECTs 4.5 m + 4.0 m + 1.8 m | 0.00 % (= HIGH) | 43.7 % | **right / ok** |

S_eur (port): 7763513 EUR 102,259.94; 5733 EUR 184,824.58; 6570707 EUR 1,139,363.75; 6660753 EUR 3,489,857.21;
5663715 EUR 10,300,000.00. None is in the band (≥ EUR 100 m) or within 2 % of it. Under every guard set (v2, +single,
+cover, +both) the port's S, currency, n and vouched_by are the same for all five (`g_removed` empty), and every vouch
candidate's notice names summed keys, so neither guard touches them.

## Per head

### 7763513: right / ok (Stolichna obshtina, bearings and bearing components for OP SPTO; vouch: CN estimate)

- **Procedure:** an open procedure, TED F02 23214436 (2023/S 119-374339), two Lot sections: LOT_NO 1 "Доставка на
  лагери" (bearings: 41 bearing kits, 15 bearing types, 18 housings, 102 bearing types) VAL_OBJECT BGN 170,000; LOT_NO 2
  "Доставка на комплектоващи елементи за лагерни възли" (93 oil seals, 8 O-rings/profile seals, 31 belts) VAL_OBJECT BGN
  30,000. Procedure **VAL_ESTIMATED_TOTAL BGN 200,000**. Each lot is a 24-month supply contract that ends at 24 months
  "or when its forecast value is reached" (INFO_ADD 1), so the contract value is the lot's forecast value by design.
- **Awards:** F03 23601088 (2023-12-26) awards LOT_NO 1 to ПроЕксТийм ЕООД (Plovdiv), contract 125482, 5 tenders,
  VAL_TOTAL BGN 170,000 (= its VAL_ESTIMATED_TOTAL; procedure VAL_TOTAL 170,000, its own lot). F03 23701706
  (2024-01-12) awards LOT_NO 2 (ITEM 1, LOT_NO 2, served on LOT-2) to ХЕНДИ ЕООД (Gabrovo), contract 127141, 1 tender,
  VAL_TOTAL BGN 30,000 (procedure VAL_TOTAL 30,000, its own lot).
- **Why right:** two distinct product lots, two contractors, two contracts, each awarded once. S = 170,000 + 30,000 =
  the buyer's estimated total. Each lot figure is that lot's own ceiling, so no envelope is shared and the sum is the
  procedure's ceiling, the value the buyer itself stated. The head without W is lot 2 alone (15 %). The vouch is an
  estimate, but here the awards equal the lot estimates exactly, so the estimate total and the award sum coincide.

### 5733: right / ok (szpital, Opatrunki, środki higieniczne (pieluchomajtki); 21 packages; vouch: BT-161)

- **Procedure:** eForms CN 24209331 (subtype 16, 2024-09-10) and its correction 24325390 (`cor-pub`, 2024-10-11): an
  open procedure in 21 lots ("pakiety"), LOT-0001…LOT-0021, no lot or procedure estimate stated.
- **First CAN 24561226** (subtype 29, can-standard, 2025-02-26): 20 LotResults (all `selec-w`) for every lot except
  LOT-0018, each with its own LotTender BT-720 and contract (BT-145 2024-12-09). The 20 BT-720s sum to **PLN
  563,765.36**, but the notice's **BT-161-NoticeResult is PLN 785,837.16** = 563,765.36 + 222,071.80, i.e. it already
  includes lot 18.
- **Second CAN 24758276** (subtype 29, 2025-05-13): LOT-0018 "Pieluchomajtki dla potrzeb Zakładu
  Opiekuńczo-Leczniczego" to Konsorcjum Citonet Śląski / TZMO, BT-720 **PLN 222,071.80**, 2 tenders, contract 2/ZP/2025
  signed **2025-01-20**, before the first CAN was dispatched (2025-02-25). Its BT-161 is its own lot (222,071.80).
- **Why right:** 21 distinct packages (compresses, gauze, plasters, bandages, plaster casts, gelatine sponges, lignin,
  specialist dressings, washing foam, wipes, two diaper packages for two different units, NPWT dressings, suture kits),
  each awarded once. The buyer's BT-161 on the first CAN equals S to the grosz, including the lot whose contract was
  already signed but published later. So the vouch is genuine: it is not the vouching notice's own lot award (6465510
  shape) and it is not contradicted by a later procedure total (the later CAN states only its own lot). The head without
  W is lot 18 alone (28 %). Lots 18 and 19 are both diapers but for different units (ZOL vs hospital wards), with
  different contracts (2/ZP/2025 vs 180/ZP/2024) and values (222,071.80 vs 96,365.81), and both are in the CN's lot list
  and in the BT-161: not a double count. Several packages share one contract number per supplier (e.g. Zarys
  179/ZP/2024 for packages 2, 5, 6, 7, 12, 15, 21), but each package has its own BT-720, which partitions that contract.

### 6570707: right / ok (MÁV Szolgáltató Központ, uniform denim; frameworks; vouch: CN estimate, then CAN total)

- **Procedure:** TED F02 20595292 (2019-08-01, 2019/S 147-361288), an open procedure for "adásvételi keretszerződés"
  (sale-purchase framework contracts) in two lots: LOT_NO 1 women's and men's denim jackets and trousers, framework amount
  ("keretösszeg") net HUF 365,752,608 (281,348,160 committed + 84,404,448 option); LOT_NO 2 women's and men's denim knee
  shorts, framework amount net HUF 94,242,720 (72,494,400 + 21,748,320 option). Procedure **VAL_ESTIMATED_TOTAL HUF
  459,995,328** = 365,752,608 + 94,242,720. (The CN's LOT-1 VAL_OBJECT prints 365,725,608, a digit transposition of the
  text's 365,752,608; the CN total, the CAN and the award all use 365,752,608. No effect on S.)
- **CAN 20808119** (F03, 2019-12-17): RES-1 LOT_NO 1 and RES-2 LOT_NO 2, both to "Pro-Team" Rehabilitációs Közhasznú
  Nonprofit Kft., contracts "1." and "2." concluded 2019-12-05, 1 tender each, VAL_TOTAL 365,752,608 and 94,242,720
  (each = its VAL_ESTIMATED_TOTAL). Procedure **VAL_TOTAL HUF 459,995,328**.
- **2022 F20s:** 22484360 (lot 1) and 22521344 (lot 2) are contract modifications (Kbt. 141 § (2); the 2022 Russia
  sanctions clause). Their LotResult VAL_TOTAL restates each lot's original award (365,752,608 / 94,242,720), which the
  port sums; the post-modification values (VAL_TOTAL_AFTER 400,620,122 / 103,477,481) are modification kinds and are not
  used.
- **Why right:** two distinct product lots, each a single-supplier framework with its own ceiling, summed at their
  original awards with no pre/post-modification mix. S equals the buyer's procedure award total on the CAN and its
  estimated total on the CN. A per-lot ceiling in separate lots does not share an envelope, so it is additive (the
  `not_additive` class covers ceilings copied across suppliers of one envelope, which is not the case here). The head
  without W is lot 1 alone (79.5 %).

### 6660753: right / ok (ÁEEK, EFOP-2.2.0 psychiatric and addiction care network, construction works in 26 towns)

- **Procedure:** TED F02 20741897 (2019-11-20, 2019/S 224-548608), an open procedure for general contractor works in 26
  lots, one per town (LOT_NO 1 Kaposvár … 26 Karcag), no estimate stated; F14s 20774806 and 20838348 only change payment
  terms and the deadlines.
- **CAN 21475284** (F03, 2021-01-15): 26 LotResults. Six lots (1, 2, 3, 8, 13, 21) are NO_AWARDED_CONTRACT /
  PROCUREMENT_DISCONTINUED (served `clos-nw`, no value). The other 20 are awarded on 2020-11-30 to 12 different
  contractors (2–4 tenders each), contract numbers = lot numbers, VAL_TOTAL per lot. Procedure **VAL_TOTAL HUF
  1,381,739,164**, which equals the sum of the 20 awarded lots exactly (recomputed).
- **F20s** 22510683 (lot 4), 22984840 (lot 5), 22975660 (lot 6), 23029439 (lot 7): contract modifications whose LotResult
  VAL_TOTAL restates the original award (74,766,064 / 62,817,630 / 199,950,783 / 65,132,339), and the port sums those
  restatements. Lot 4 (to 96,214,895) and lot 7 (to 71,689,361) also have value increases, which are modification kinds
  and are not used. Lots 5 and 6 changed only the deadline.
- **Why right:** 20 distinct town works, each awarded once at its original award value. The six discontinued lots add
  nothing. S equals the buyer's procedure total to the forint. The head without W is lot 7 (Szekszárd) alone, 4.7 % of
  the procedure.

### 5663715: right / ok (Ville de Paris, ornamental fountains, 3 framework lots; vouch: TED-HIGH, Range kind)

- **Procedure:** TED F02 18772080 (2016-05-03, 2016/S 086-151805), FRAMEWORK, "accords-cadres à bons de commande" in 3
  lots, each with a single operator, running 24 months and renewable once (48 months): lot 1 (DPA, fountains on public
  space) min EUR 750,000 / max EUR 2,250,000 per 24 months, VAL_OBJECT EUR 4,500,000; lot 2 (DEVE, maintenance in parks)
  min 700,000 / max 2,000,000, VAL_OBJECT 4,000,000; lot 3 (DEVE, repairs in parks) min 300,000 / max 900,000,
  VAL_OBJECT 1,800,000. So each VAL_OBJECT is the lot's 48-month maximum. Lots 1+2 and 2+3 cannot go to one bidder.
- **CAN 19055486** (F03, 2016-12-15): RES-1 LOT_NO 1 to EMTE/COMSA Services, EUR 4,500,000; RES-2 LOT_NO 2 to Veolia Eau
  Île-de-France, EUR 4,000,000; RES-3 LOT_NO 3 to Segex Energies, EUR 1,800,000. Three contracts
  (20161120001397/98/99), 3 tenders each. Procedure VALUE_RANGE (PROCUREMENT_TOTAL) **LOW EUR 3,500,000 / HIGH EUR
  10,300,000**. LOW = 2 × (750k + 700k + 300k), the 48-month minima, and HIGH = 4.5 m + 4.0 m + 1.8 m, the 48-month
  maxima. (Both are printed twice, ordinals 0 and 1, hence the duplicate vouch.)
- **Why right:** three distinct lots, three operators and three separate frameworks, each with its own maximum. The lot
  awards are each lot's maximum on one basis (48 months, renewal included), and S, their sum, is exactly the buyer's
  stated procedure maximum (HIGH). No envelope is shared across lots, so the maxima add up (the sum is the procedure's
  maximum, which the buyer itself states). S is not farther than the head: the stated total is the range [3.5 m, 10.3 m],
  S sits on its top, and the head (EUR 4.5 m, lot 1 alone) is one lot's maximum, not a procedure figure. A framework's
  value is conventionally its maximum, so S is the procedure's value on the buyer's own basis. Range vouches are allowed
  (the 8426318 pin's candidates are Range), and this one names all three summed keys (G-cover keeps it).

## Adversarial checks applied to every head (each found nothing)

- **Double count:** every summed key is a distinct printed LOT_NO / BT-137 with its own title. Where a lot has two rows
  (6570707 LOT-1/LOT-2; 6660753 LOT-4…7: CAN row + F20 restatement), the rows carry the same value and S takes one.
  7763513's second CAN prints ITEM 1 / LOT_NO 2 and is served on LOT-2, which is correct. 5733's two diaper packages are
  separate lots for separate units (see above).
- **Slip, VAT, currency:** every S equals a buyer-stated total, in the notices' own currency (7763513 CN estimate total;
  5733 BT-161; 6570707 CN estimate total and CAN VAL_TOTAL; 6660753 CAN VAL_TOTAL; 5663715 CAN HIGH). No lot estimate
  contradicts a lot award (7763513, 6570707 and 5663715 awards equal their lot estimates; the other two print none).
- **Shared envelope / not_additive:** 6570707 and 5663715 are frameworks and 7763513's contracts run "until the forecast
  value is reached". In all three, each lot has a single supplier and its own ceiling, the lots are different scopes, and
  the buyer's total is the sum of the lot ceilings. There is no multi-supplier lot and no ceiling copied across suppliers.
- **Vouch belongs to something else (6465510 shape):** in no case does the vouching total equal the vouching notice's own
  lot award. Each equals the sum of every summed lot, and every vouching notice names the summed keys (5733: 20 of 21, and
  its BT-161 includes the 21st exactly).
- **Superseded vouch (5816379 shape):** no later notice re-awards and states a different procedure total. The later
  notices state their own lot only (5733's 24758276; 7763513's 23701706) or are contract modifications with per-lot
  before/after values (6570707's and 6660753's F20s), which are modification kinds, not re-awards.
- **Pre/post-modification mix (1091419 shape):** none. 6570707 and 6660753 sum every awarded lot at its original award;
  the raised values of the modified lots (6570707 lots 1–2; 6660753 lots 4 and 7) are not used.
- **Farther:** S/T = 0.00 % on all five (5663715 against the stated HIGH, and S lies inside the stated range). The head
  without W covers 4.7–79.5 % of the procedure and is in each case one lot's figure.

## Requests (public API only; log `b6/u5/api_requests.log`, entries tagged `reader=read_a_2`)

26 GETs, all 200, 0 × 408, 0 retries, paced at least 2 s apart under a shared lock (`b6/u5/.http.lock` +
`.http.stamp`; across all readers in the log no two requests are less than 2 s apart):
- 5 live heads: `/v1/tenders/{7763513, 5733, 6570707, 6660753, 5663715}`;
- 21 notices, every version notice of the five chains (`/v1/notices/{id}/content`): 23214436, 23601088, 23701706;
  24209331, 24325390, 24561226, 24758276; 20595292, 20808119, 22484360, 22521344; 20741897, 20774806, 20838348,
  21475284, 22510683, 22984840, 22975660, 23029439; 18772080, 19055486.

Bodies are under `b6/u5/bodies/` (`tenders_{id}.json`, `notices_{id}_content.json`). No `/v1/sql`, ssh, POST, git or
cargo was used, and nothing was written outside `b6/u5/` except the reader's own throwaway helper scripts in the
session scratchpad.

**Protocol note:** this file is one reader's read under M2. M2 pairs it with an independent adversarial skeptic per head
and sends a disagreement to a third reader. The checks above are this reader's own adversarial pass, not a substitute for
that skeptic.

```json
{"reader": "read_a_2", "round": "a", "ranks": [6, 10],
 "verdicts": [
  {"id": 7763513, "verdict": "right", "class": "ok"},
  {"id": 5733, "verdict": "right", "class": "ok"},
  {"id": 6570707, "verdict": "right", "class": "ok"},
  {"id": 6660753, "verdict": "right", "class": "ok"},
  {"id": 5663715, "verdict": "right", "class": "ok"}
 ],
 "tally": {"right": 5, "neutral": 0, "wrong": 0}}
```
