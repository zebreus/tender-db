# 551 B6 U5 fresh pre-flip read, round a, part 3: ranks 11–15

2026-10-19, reader `read_a_3`. These are ranks 11–15 of the fresh read's order over P minus every read id
(`b6/u5/order.json`, salt `551-b6-fresh-`, `sha256('551-b6-fresh-'+id)` ascending; `first_60[10:15]` checked and the five
order keys recomputed: 00f629dc…, 0111d777…, 011e866f…, 01221422…, 01323c1d…): **6826783, 6611306, 8488091, 118880,
805752**. All five are v2+both ships in P (`b6/u2/P.json`, guard set `v2+both`), and none is in `excluded_from_P` or in any
source of `excluded_sources.json`. So none has an earlier recorded verdict, and all five are new reads.

**The question for each id:** P = v2+both (3,467 ships). Path W is OFF, so the live head is the max-elected figure, not the
sum. If the flip ships, the tender serves S, the summed lot awards, vouched by the G-filtered `vouched_by`. Under M2's
rubric, is serving S as the tender's value RIGHT, NEUTRAL or WRONG?

**Rubric (M2, wf_dc85fa7e-c00's, as U4 and read_a_1/read_a_2 applied it):** right/ok means S is the procedure's value:
distinct lots (TED `LOT-k` checked against the printed LOT_NO), each lot's latest award, no lot counted twice, no ×10ᵏ
slip, no VAT mix, and S no farther from the procedure's stated total than the head it replaces. Wrong classes:
`not_additive` (ceilings or maxima sharing one envelope), `double_count`, `farther`, `slip`, `other` (for example, the
vouching total belongs to something else, as in 6465510). Neutral means S is not wrong but not clearly better than the head
without W. **Precedents applied:** read18 note 5 (per-lot single-operator framework ceilings whose sum is the buyer's own
total are not `not_additive`). From the B5 investigation read, point 4, and U4 read_1/3/5: on a modified head, S is the
as-awarded sum, and it must also be no farther than the head under the post-modification total. From read_a_2's 7763513:
a CAN's lot award that equals its lot estimate is the published award.

**Inputs:** `b6/u5/order.json`, which records P.json 5f8b7f7c…, ids_sha256 a2619fd9… and decisions.jsonl.gz 0add92e3….
The port-predicted S, currency, n, summed keys ← notices and the G-filtered `vouched_by` come from
`b6/u2/decisions.jsonl.gz` (record `decisions["v2+both"]`). On all five, v2+both equals v2: `g_removed`,
`g_single_removed` and `g_cover_removed` are empty, and the one vouch candidate has `g_single: false, g_cover: false`.
The flags are empty, so there is no estimate-twin flag. Live heads come from `GET /v1/tenders/{id}`. **All five live heads
are JSON-equal to their U2 fresh-layer copies** (`scratchpad/b6/tenders`, fetched 06:18Z–07:40Z), and **all 14 version
notices of the five chains were fetched live and are JSON-equal to their U2 notice-layer copies** (`scratchpad/b6/notices`).
So the port's facts are current. No live head serves a derivation (`value_derivation: null` on all five).

## Result

**5 right / 0 neutral / 0 wrong**, all class `ok`. In every case S equals a buyer-stated total that covers every lot of
the procedure, to the cent or forint (6611306: to 13 cents, the CAN's own rounding). Every summed key is a distinct
printed lot with one award. The head without W is either one lot's figure (14–58 % of S) or null. Two heads (6826783,
6611306) carry an F20 that raises one lot after award. On both, S is the as-awarded total, and it is still far nearer the
post-modification procedure value than the head is (S −5.4 % against −45.6 % and −86.7 %).

| rank | id | S (port, v2+both) | n | summed: key ← notice, award | vouched_by (G-filtered = v2) | head without W (live) | buyer-stated total(s) matching S | S/T | head/S | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| 11 | 6826783 | HUF 476,573,000.00 | 3 | LOT-1 ← 21404947 120,053,000; LOT-2 ← 21404947 82,445,000; LOT-3 ← 22064682 274,075,000 (F20 restating the CAN's lot-3 award) | TED-VAL_TOTAL HUF 476,573,000 on 21404947 (F06, 3 results, names all 3 keys) | HUF 274,075,000.00 (lot 3) | F06 21404947 procedure VAL_TOTAL = TED-VALUE (PROCUREMENT_TOTAL); its short description lists the 3 net frame amounts | 0.00 % | 57.5 % | **right / ok** |
| 12 | 6611306 | EUR 7,267,960.13 | 7 | LOT-1, 2, 4, 5, 6, 7 ← 21302219: 365,220.82; 3,209,700.00; 1,072,835.00; 1,392,234.22; 29,890.00; 175,416.00. LOT-3 ← 23552871 1,022,664.09 (F20 restating the CAN's lot-3 award) | TED-VAL_TOTAL EUR 7,267,960.00 on 21302219 (F03, 7 results, names all 7 keys) | EUR 1,022,664.09 (lot 3) | F03 21302219 procedure VAL_TOTAL = TED-VALUE (PROCUREMENT_TOTAL) 7,267,960.00 | +0.000002 % (13 cents) | 14.1 % | **right / ok** |
| 13 | 8488091 | EUR 164,676.75 | 3 | LOT-1 ← 22272896 63,205.75; LOT-2 ← 22344730 26,271.00; LOT-3 ← 22277915 75,200.00 | TED-VAL_ESTIMATED_TOTAL EUR 164,676.75 on CN 22059514 (F02, 3 Lot sections, all summed; estimate kind) | EUR 26,271.00 (lot 2) | CN 22059514 VAL_ESTIMATED_TOTAL = TED-VALUE (ESTIMATED_TOTAL) = lot VAL_OBJECTs 63,205.75 + 26,271.00 + 75,200.00; each CAN's award = its lot estimate | 0.00 % | 16.0 % | **right / ok** |
| 14 | 118880 | EUR 290,383.00 | 3 | LOT-0001 ← 24501593 166,736.00; LOT-0003 ← 24501593 113,811.00; LOT-0004 ← 24501593 9,836.00 | BT-1118-NoticeResult EUR 290,383 on 24501593 (CAN 29, 3 results, names all 3 keys) | **null** | 24501593 BT-1118 290,383.00 = Σ BT-660 = Σ BT-720 | 0.00 % | — (null) | **right / ok** |
| 15 | 805752 | HUF 15,060,500,000.00 | 4 | LOT-0001…0004 ← 24457168: 7,000,000,000; 3,135,000,000; 4,318,000,000; 607,500,000 | BT-1118-NoticeResult HUF 15,060,500,000 on 24457168 (CAN 29, 4 results, names all 4 keys) | **null** | 24457168 BT-1118 15,060,500,000 = Σ BT-660 = Σ BT-720 | 0.00 % | — (null) | **right / ok** |

S_eur (port): 6826783 EUR 1,293,067.61; 6611306 EUR 7,267,960.13; 8488091 EUR 164,676.75; 118880 EUR 290,383.00; 805752
EUR 36,294,734.31. None of the five is a band head (B_P, ≥ EUR 100 m) or within 2 % of EUR 100 m. None is in
`band_set.json`, in the provisional L_changed (239 ids; ∩ 3,502 = 0) or in the 545/571 sets. So decision point 8 (B-band)
gets nothing from this part.

## Per head

### 6826783: right / ok (Tiszamenti Regionális Vízművek Zrt., sewage sludge removal and treatment, 3 regions)

- **Procedure:** CN F05 21069082 (2020/S 124-304955), utilities directive (32014L0025), open procedure. It has three
  regional lots. LOT_NO 1 "Keleti régió" covers about 8,384,000 kg of sludge, LOT_NO 2 "Középső régió" about 8,940,000 kg,
  and LOT_NO 3 "Nyugati régió és Tápióság" about 17,190,000 kg. Each lot runs 12 months, renewable once for up to 4 months
  if the frame is not exhausted. The CN states no value.
- **Award:** F06 21404947 (2020/S 240-594710).
  - RES-1 awards LOT_NO 1 to A.K.S.D. Városgazdálkodási Kft. for HUF 120,053,000 (1 tender, contract 1).
  - RES-2 awards LOT_NO 2 to A.K.S.D. for HUF 82,445,000 (1 tender, contract 2).
  - RES-3 awards LOT_NO 3 to Vértes és Vidéke Kommunális Szolgáltató Kft. for HUF 274,075,000 (1 tender, contract 3).
  - Each result has VAL_ESTIMATED_TOTAL = VAL_TOTAL. Procedure **VAL_TOTAL HUF 476,573,000** = the sum.
  - The short description spells it out: "1. rész: nettó 120 053 000 HUF / 2. rész: nettó 82 445 000 HUF / 3. rész: nettó
    274 075 000 HUF". The buyer commits to call off up to 90 % of each part's frame amount (keretösszeg). These are per-lot
    single-operator framework contracts, all net.
- **F20 22064682** (2021/S 230-606592) modifies the lot-3 framework contract under Kbt. 141. § (2). Net 274,074,938 of the
  274,075,000 frame had been used, so the frame was raised: **VAL_TOTAL_BEFORE 274,075,000 → AFTER 301,481,938** (+10 %).
  Its LotResult (LOT_NO 3, contract 3) restates the award at 274,075,000. S takes LOT-3 once, from this row, at the award
  value.
- **Why right:** three distinct regional lots, three contracts, one award each. S equals the buyer's procedure award
  total to the forint. S sums per-lot single-operator framework ceilings, each belonging to one lot only (read18 note 5),
  so it is not `not_additive`. Lots 1 and 2 share a winner, but they are different regions with different quantities and
  values, under contracts 1 and 2, so there is no double count. The head without W is lot 3 alone (57.5 % of S).
- **Modified-head check:** the post-modification procedure value is 120,053,000 + 82,445,000 + 301,481,938 =
  HUF 503,979,938. S is −5.4 % from it and the head −45.6 %, so S is not `farther`. Every summed figure is an as-awarded
  value, and the modification's AFTER value is not summed, so pre- and post-modification values are not mixed.

### 6611306: right / ok (Département de la Seine-Saint-Denis, collège Lenain de Tillemont, Montreuil, 7 macro-lots)

- **Procedure:** CN F02 20615458 (2019/S 184-446871), open procedure, works: the energy renovation and restructuring of a
  2,292 m² collège. It has seven macro-lots, LOT_NO 1–7: temporary buildings; extended structural work; exterior joinery
  and locksmithing; finishing; heating, ventilation, plumbing and electrical; lifts; warming kitchen. Each runs 26 months,
  with no options. The CN states no value.
- **Award:** CAN F03 21302219 (2020/S 202-487433). RES-1…7 each award their own LOT_NO to one winner, concluded
  2020-06-09:
  - LOT_NO 1 Altempo 365,220.82 (2 tenders)
  - LOT_NO 2 Fayolle et Fils 3,209,700.00 (1 tender)
  - LOT_NO 3 SMA 1,022,664.09 (2 tenders)
  - LOT_NO 4 L'Union 1,072,835.00 (2 tenders)
  - LOT_NO 5 STEPC 1,392,234.22 (3 tenders)
  - LOT_NO 6 Thyssenkrupp Ascenseurs 29,890.00 (3 tenders)
  - LOT_NO 7 Medinox 175,416.00 (4 tenders)

  Procedure **VAL_TOTAL EUR 7,267,960.00**. The exact sum is 7,267,960.13, so the CAN dropped the cents.
- **F20 23552871** (2023/S 222-696703) is avenant n°3 on lot 3 (SMA): supplementary works under R.2194-2 CCP (OS n°22 and
  23). Its LotResult (LOT_NO 3) restates **VAL_TOTAL 1,022,664.09**, the original award. The modification block gives
  VAL_TOTAL_BEFORE 1,236,933.46 (after avenants 1–2, which were not published on TED) → **AFTER 1,435,222.03**. S takes
  LOT-3 once, from this row, at the award value.
- **Why right:** seven distinct trades, seven winners, one award each, all at conclusion values. S equals the buyer's
  procedure total within the CAN's 13-cent rounding. The port's R10 accepts the vouch at that tolerance. The head without
  W is lot 3 alone (14.1 %).
- **Modified-head check:** with lot 3 at 1,435,222.03, the procedure is EUR 7,680,518.07. S is −5.4 % from it and the head
  −86.7 %. The F20 restates one summed lot at its original value, so there is no pre- and post-modification mix.

### 8488091: right / ok (Ministerio de Asuntos Económicos y Transformación Digital, office furniture, Complejo Cuzco)

- **Procedure:** CN F02 22059514 (2021/S 228-599622), open procedure, LOT_MAX_NUMBER 3, 2 months, no options or renewal.
  It has three lots:
  - LOT_NO 1 "Muebles de oficina general", VAL_OBJECT 63,205.75
  - LOT_NO 2 "Sillería", VAL_OBJECT 26,271.00
  - LOT_NO 3 "Muebles de dirección", VAL_OBJECT 75,200.00

  Procedure **VAL_ESTIMATED_TOTAL EUR 164,676.75** = the lot sum.
- **Awards:** three F03s, each with one LotResult and each referring to the CN as THIS_PROCEDURE:
  - 22272896 (2022/S 053-137572): LOT_NO 1 to Imasoto S.A. for 63,205.75 (4 tenders, contract 1, 2022-03-10).
  - 22277915 (2022/S 055-141309): LOT_NO 3 to Dynamobel S.A. for 75,200.00 (2 tenders, contract 3, 2022-03-10).
  - 22344730 (2022/S 070-183649): LOT_NO 2 to Dynamobel for 26,271.00 (5 tenders, contract 2, 2022-04-01).

  In each CAN the lot's VAL_TOTAL equals its VAL_ESTIMATED_TOTAL and the CN's VAL_OBJECT. Each CAN's procedure VAL_TOTAL
  is its own lot only.
- **Why right:** three distinct lots, three CANs, one award each, two winners. S equals the CN's estimate total to the
  cent. The vouch is estimate kind, from a three-section CN whose lots are all in S, so the estimate-twin flag does not
  apply. The single-lot CANs' own totals are not candidates: none equals S, and G-single would refuse them anyway. The head
  without W is lot 2 alone (16.0 %), from the last CAN.
- **Note for the skeptic (no verdict impact):** every award equals its lot budget exactly, even though 2–5 tenders were
  received. The buyer may have published the budget as the contract value, as with a unit-price contract up to the budget.
  It did so in every CAN, as the lots' VAL_TOTAL, and the head without W has the same basis. TED's lot and award values are
  both net of VAT, so there is no VAT mix and no slip, and no rubric class fits. This is the same shape as read_a_2's
  7763513 (right).

### 118880: right / ok (Mesto Rožňava, municipal insurance for 4 years, 3 parts)

- **Procedure:** eForms CN (subtype 16) 24320689 (00597229-2024), open procedure, lowest price, no BT-27. It has three Lot
  sections:
  - LOT-0001 "Časť 1: Poistenie majetku" (property and liability; fa-wo-rc)
  - LOT-0003 "Časť 2: Havarijné poistenie motorových vozidiel a PZP" (motor hull and MTPL; fa-mix)
  - LOT-0004 "Časť 3: Úrazové poistenie" (accident; fa-wo-rc)

  No LOT-0002 section exists in either notice. It is a key gap only: the parts are numbered 1, 2 and 3, and all three are
  summed.
- **Award:** eForms CAN (subtype 29) 24501593 (00014252-2025).
  - RES-0001 → LOT-0001: BT-660 166,736.00 = TEN-0001 BT-720; contract CON-0001 "803/2024"; 1 tender.
  - RES-0002 → LOT-0003: 113,811.00 = TEN-0002; contract CON-0002 "800/2024, 801/2024"; 1 tender.
  - RES-0003 → LOT-0004: 9,836.00 = TEN-0003; contract CON-0004 "802/2024"; 3 tenders (BT-710 lowest 9,836 / BT-711
    highest 26,812).
  - All three lots went to Kooperativa poisťovňa, a.s. VIG (one tendering party). Notice-level **BT-1118 EUR 290,383.00** =
    the sum.
- **Why right:** three distinct insurance lines, three contracts, one winner per lot. S equals the buyer's BT-1118 to the
  cent. LOT-0003 is `fa-mix`, but it has a single winner, so no envelope is shared. The live head without W is **null**:
  none of the result-level or notice-level framework values is elected. So S replaces no figure and cannot be farther.

### 805752: right / ok (Nemzeti Egészségbiztosítási Alapkezelő, factor VIII and von Willebrand products, 4 lots)

- **Procedure:** eForms CN (subtype 16) 24206725 (00581899-2024). It is an accelerated negotiated procedure with a call
  (Kbt. 85. § (2) c)) for framework agreements with **one operator per lot** (Kbt. 105. § (1) a)). It has four lots, each
  a different product and each with its own framework quantity:
  - LOT-0001: full-length FVIII with a PK dosing device, 35,000,000 IU plus a mandatory 17,500,000 IU rebate in kind.
  - LOT-0002: B-domain-truncated, tyrosine-sulphated FVIII, 30,000,000 IU.
  - LOT-0003: 4th-generation Fc-fusion extended-half-life FVIII, 20,000,000 IU.
  - LOT-0004: balanced VWF/FVIII, 4,500,000 IU, ending 2025-12-31 (the others end 2026-12-31).

  The award criterion is the "Nettó ajánlati összár (teljes Keretmennyiségre, HUF)": the net total price for the whole
  framework quantity. There is no BT-27.
- **Award:** eForms CAN (subtype 29) 24457168 (00731786-2024). RES-0001…0004 each have one tender, with BT-660 = BT-720:
  - LOT-0001: 7,000,000,000 (Hungaropharma)
  - LOT-0002: 3,135,000,000 (Atlas Pharma)
  - LOT-0003: 4,318,000,000 (Hungaropharma)
  - LOT-0004: 607,500,000 (Hungaropharma)

  There are four separate framework contracts ("Keretmegállapodás", ids 1–4, concluded 2024-11-27). Notice-level
  **BT-1118 HUF 15,060,500,000** = the sum.
- **Why right:** four distinct products and four single-operator frameworks, each awarded once. S equals the buyer's
  BT-1118 to the forint, and every figure is net for the full framework quantity, so the basis is consistent. read18 note
  5 applies: each ceiling belongs to one lot only. The BT-553 subcontracting values (61,819,245 on lots 1 and 3, 1,567,500
  on lot 2, 10,313,999 on lot 4) are not summed. The head without W is **null**, so S cannot be farther.

## Adversarial checks applied to every head (each found nothing)

- **Double count:** every summed key is a distinct printed LOT_NO (TED) or BT-13713 lot (eForms), each with its own title
  and contract.
  - 6826783's and 6611306's LOT-3 each has two rows (CAN and F20) at the same value, and S takes one.
  - 8488091's three lots come from three separate CANs, with no overlap.
  - No lot-null row restates a keyed award.
- **Slip, VAT, currency:** every S equals a buyer-stated total in the notice's own currency (HUF or EUR). 6611306 matches
  within the CAN's 13-cent rounding; the other four match exactly. No figure is 10ᵏ off a sibling figure. 6826783's values
  are stated "nettó", 805752's criterion is net, and the TED lot and award values are net, so no lot mixes bases.
- **Shared envelope / not_additive:**
  - 6826783 and 805752 sum per-lot single-operator framework ceilings, which read18 note 5 accepts.
  - 118880's lots each have one winner and one contract.
  - There is no multi-supplier lot, unit-rate cap or nested lot.
- **Vouch belongs to something else (6465510 shape):** every vouching total comes from a notice that awards or estimates
  every summed lot. The four award-kind vouches come from notices with 3–7 results that name every summed key on R3's
  accessor. 8488091's vouch is estimate kind, which G-cover leaves unchanged: it has 0 results, and its CN's three Lot
  sections are exactly the three summed lots. None equals one result's own value for a lot outside S. G-single and G-cover
  both keep all five.
- **Superseded vouch (5816379 shape):** no later award notice restates a different procedure total for the summed lots.
  - The two F20s restate their lot's original award. Their VAL_TOTAL_AFTER is a modification kind, not an award total.
  - 8488091's later CANs each total only their own lot.
- **Farther:** S/T = 0.00 % on four heads and +0.000002 % on 6611306. The heads without W are null (118880, 805752) or one
  lot's figure (6826783 57.5 %, 6611306 14.1 %, 8488091 16.0 % of S). On the two modified heads, S stays far nearer the
  post-modification total than the head (−5.4 % against −45.6 % and −86.7 %).

## Requests (public API only; log `b6/u5/api_requests.log`, entries tagged `reader=read_a_3`)

There were 19 GETs (15:55:30Z–15:56:32Z), all 200, with 0 × 408 and 0 retries. Request starts were paced at least 2.2 s
apart under the shared lock (`b6/u5/.http.lock` + `.http.stamp`), interleaved with `read_a_4`'s requests.
- 5 live heads: `/v1/tenders/{6826783, 6611306, 8488091, 118880, 805752}`.
- 14 notices, every version notice of the five chains (`/v1/notices/{id}/content`):
  - 6826783: 21069082, 21404947, 22064682
  - 6611306: 20615458, 21302219, 23552871
  - 8488091: 22059514, 22272896, 22277915, 22344730
  - 118880: 24320689, 24501593
  - 805752: 24206725, 24457168

Bodies are under `b6/u5/bodies/` (`tenders_{id}.json`, `notices_{id}_content.json`). Nothing was written outside `b6/u5/`
except this reader's throwaway helper scripts in the session scratchpad. There was no /v1/sql, ssh, POST, git or cargo.

**Protocol note:** this file is one reader's read under M2. M2 pairs it with an independent adversarial skeptic per head
and sends any disagreement to a third reader. The checks above are this reader's own adversarial pass, not a substitute
for that skeptic. For the skeptic, the points most worth pressing are these three:
- 8488091: every award equals its lot budget.
- 6826783 and 6611306: S is as-awarded while an F20 raises one lot.
- 6611306: the vouch matches within 13 cents.

```json
{"reader": "read_a_3", "round": "a", "ranks": [11, 15],
 "verdicts": [
  {"id": 6826783, "verdict": "right", "class": "ok"},
  {"id": 6611306, "verdict": "right", "class": "ok"},
  {"id": 8488091, "verdict": "right", "class": "ok"},
  {"id": 118880, "verdict": "right", "class": "ok"},
  {"id": 805752, "verdict": "right", "class": "ok"}
 ],
 "tally": {"right": 5, "neutral": 0, "wrong": 0}}
```
