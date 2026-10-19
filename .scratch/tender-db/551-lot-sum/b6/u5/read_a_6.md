# 551 B6 U5 fresh pre-flip read, round a, part 6: P ranks 26–30

2026-10-19, reader `read_a_6`. These are ranks 26–30 of the fresh read's order over P minus every read id
(`b6/u5/order.json`, salt `551-b6-fresh-`, `sha256('551-b6-fresh-'+id)` ascending; `first_60[25:30]` and the entries'
ranks 26–30 checked, order keys recomputed and equal): **8524598, 6757752, 6960026, 844584, 6763031**. None is in
`excluded_from_P` (every read id) or in `excluded_sources.json`, so all five are new reads. These five close round a's
first 30.

**The question for each id:** P = v2+both (3,467 ships, `b6/u2/P.json`). Path W is OFF, so the live head is the
max-elected figure, not the sum. If the flip ships, the tender serves S, the summed lot awards, vouched by the
G-filtered `vouched_by`. Under M2's rubric, is serving S as the tender's value RIGHT, NEUTRAL or WRONG?

**Rubric (M2, wf_dc85fa7e-c00's):** right/ok means S is the procedure's value: distinct lots (TED `LOT-k` checked against
the printed LOT_NO), each lot's latest award, no lot counted twice, no ×10ᵏ slip, no VAT mix, and S no farther from the
procedure's stated total than the head it replaces. Wrong classes: `not_additive` (ceilings or maxima sharing one
envelope), `double_count`, `farther`, `slip`, `other` (for example, the vouching total belongs to something else, as in
6465510). Neutral means S is not wrong but not clearly better than the head without W. **Precedents applied:** read18
note 5 (per-lot single-operator framework ceilings whose sum is the buyer's own total are not `not_additive`);
modification values (F20 / subtype 38 VAL_TOTAL_AFTER, extensions, indexation) are a modification kind, not a re-award,
and S's use of each lot's original award is right even when the contract later grew (read_1's 7380025; M2-fallback
"after modification the contract is worth more than S, but that is outside what this lot-sum rule decides"; read_a_3's
6570707 / 6660753).

**Inputs:** `b6/u5/order.json` (sha256 3033ec31…; it records P sha256 5f8b7f7c…, ids_sha256 a2619fd9…,
decisions.jsonl.gz 0add92e3…, and those two files hash to the recorded values). The port-predicted S, currency, n,
summed keys and notices, and the G-filtered `vouched_by` come from `b6/u2/decisions.jsonl.gz` (v2+both). On all five,
v2+both equals v2: `g_removed`, `g_single_removed` and `g_cover_removed` are empty, `vouched_by` is the same object under
v2 and v2+both, every vouch candidate has `g_single = g_cover = false`, and `flags` is empty (no estimate-twin flag).
Live heads come from `GET /v1/tenders/{id}`. **All five live heads are JSON-equal to their U2 fresh-layer copies**
(`scratchpad/b6/tenders`, fetched 06:35Z–07:40Z), so the port's facts are current. **All 13 version notices** were
fetched live and are JSON-equal to their U2 notice-layer copies (`scratchpad/b6/notices`). None of the five is in the
provisional L_changed (239 ids; ∩ 3,502 = 0), in the 545/571 sets, or in `band_set.json`.

## Result

**5 right / 0 neutral / 0 wrong**, all class `ok`. In every head S equals a buyer-stated total that covers every lot of
the procedure, to the cent (or lev / złoty), and each summed lot is a distinct real award with one winner. Two heads
need a note for the skeptic: **6763031**, where two 2020 F20s later raised the contract to EUR 2,425,760 (S is the
original award total, 10.9 % below the post-modification figure; the head is 76.2 % below it), and **8524598**, whose
call was preceded by a PIN (F04, same reference 2023-00675, the same three regional lots) stating EUR 21,555,456, which
is stored as a separate tender, 7711779.

| rank | id | S (port, v2+both) | n | summed: key ← notice, award | vouched_by (G-filtered = v2) | head without W (live) | buyer-stated total(s) matching S | S/T | head/S | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| 26 | 8524598 | EUR 5,849,968.70 | 3 | LOT-1 ← 23558847 1,787,724.86; LOT-2 ← 23558847 1,334,225.46; LOT-3 ← 23558847 2,728,018.38 | TED-VAL_ESTIMATED_TOTAL EUR 5,849,968.70 on CN 23230099 (F05, estimate kind, 3 Lot sections = the 3 summed lots) | EUR 2,728,018.38 (lot 3) | CN 23230099 VAL_ESTIMATED_TOTAL = TED-VALUE 5,849,968.70; the CN's three lot VAL_OBJECTs and the F06's three VAL_TOTAL = VAL_ESTIMATED_TOTAL add to it | 0.00 % | 46.6 % | **right / ok** |
| 27 | 6757752 | BGN 2,657,000.00 | 4 | LOT-1 ← 23241877 1,550,000 (21360130 states the same); LOT-2 ← 21360130 715,000; LOT-3 ← 21360130 107,000; LOT-4 ← 21360130 285,000 | TED-VAL_TOTAL BGN 2,657,000 on 21360130 (F06, 4 results, names all 4 keys) | BGN 1,550,000.00 (lot 1) | 21360130 procedure VAL_TOTAL = TED-VALUE (PROCUREMENT_TOTAL) 2,657,000; CN 20963427 lot VAL_OBJECTs add to it | 0.00 % | 58.3 % | **right / ok** |
| 28 | 6960026 | PLN 907,500.00 | 2 | LOT-1 ← 21615726 769,000; LOT-2 ← 21615726 138,500 | TED-HIGH PLN 907,500 on 21615726 (F03, range kind; LOW = HIGH, each printed twice; 2 results, names both keys) | PLN 769,000.00 (lot 1) | 21615726 VAL_RANGE_TOTAL LOW = HIGH 907,500 (VALUE_RANGE.TYPE PROCUREMENT_TOTAL) | 0.00 % | 84.7 % | **right / ok** |
| 29 | 844584 | EUR 2,023,000.00 | 3 | LOT-0001 ← 24887461 959,000; LOT-0002 ← 24887461 364,000; LOT-0003 ← 24887461 700,000 | BT-118-NoticeResult EUR 2,023,000 on 24887461 (subtype 29, 3 results, names all 3 keys) | **null** | 24887461 BT-118 2,023,000.00 | 0.00 % | — (null) | **right / ok** |
| 30 | 6763031 | EUR 2,160,200.00 | 2 | LOT-1 ← 21005314 576,200 (20974563 states the same); LOT-2 ← 21012872 1,584,000 (20974563 states the same) | TED-VAL_TOTAL EUR 2,160,200 on 20974563 (F21, 2 results, names both keys) | EUR 576,200.00 (lot 1) | 20974563 procedure VAL_TOTAL = TED-VALUE (PROCUREMENT_TOTAL) 2,160,200; both F20s' VAL_TOTAL_BEFORE 2,160,200 | 0.00 % (−10.9 % vs post-modification 2,425,760) | 26.7 % | **right / ok** |

S_eur (order.json): 8524598 EUR 5.85 m, 6757752 EUR 1,358,523.37, 6960026 EUR 195,190.67, 844584 EUR 2.02 m, 6763031
EUR 2.16 m. None is a band head (B_P, ≥ EUR 100 m) and none is within 2 % of EUR 100 m, so decision point 8 (B-band)
gets nothing from this part. No live head serves a derivation (`value_derivation: null` on all five).

## Per head

### 8524598: right / ok (Renfe Viajeros, excise-exempt Gasóleo B for rail traction, 3 regional lots)

- **Procedure:** CN F05 23230099 (2023/S 121-382502, ref. 2023-00675), open procedure, utilities directive, LOT_ALL,
  24 months, no options, no renewal. Three Lot sections with the same title but distinct regions (MAIN_SITE): LOT_NO 1
  Madrid and Andalucía, VAL_OBJECT 1,787,724.86; LOT_NO 2 Asturias, Cantabria, Castilla-León, Galicia and País Vasco,
  1,334,225.46; LOT_NO 3 Cataluña, Comunidad Valenciana and Murcia, 2,728,018.38. Procedure **VAL_ESTIMATED_TOTAL =
  TED-VALUE EUR 5,849,968.70**, exactly the three lots' sum.
- **Award:** F06 23558847 (2023/S 215-677806). RES-1 LOT_NO 1 to Pentagas Hidrocarburos (contract 4500075098); RES-2
  LOT_NO 2 (4500075097) and RES-3 LOT_NO 3 (4500075095) to UTE Esergui-Disteser Trenes TF. Each result's VAL_TOTAL equals
  its VAL_ESTIMATED_TOTAL and the CN's lot VAL_OBJECT. The F06 states no procedure total. Award = budget is the usual
  Spanish shape for a supply at unit prices up to a maximum budget.
- **Why right:** three distinct regions, three contracts, each lot awarded once, one supplier per lot (read18 note 5).
  S equals the buyer's own procedure estimate to the cent, and the award notice restates every lot at that value. The
  vouch is estimate kind from a three-section CN whose lots are all in S, so the estimate-twin flag does not apply. The
  head without W is lot 3 alone (46.6 %).
- **Note for the skeptic (no verdict impact):** the CN names a "previous publication concerning this procedure",
  2023/S 085-262739. It is an **F04 periodic indicative notice** used to shorten the time limit (23185304, "Este anuncio
  no es una convocatoria"; same reference 2023-00675, the same three regional lots), with VAL_ESTIMATED_TOTAL
  **EUR 21,555,456.00**. The store keeps it as a separate tender, **7711779** (`/v1/tenders?publication_id=262739-2023`,
  one version, serving that estimate; not in the 3,502). The CN then priced the call at 5,849,968.70 (factor 3.68, not a
  power of ten, not VAT 1.21, not 2 vs 4 years). The PIN is a forecast, the CN and the CAN are the procedure's own
  figures and agree, so S is the procedure's value. Even if 21.6 m were the truth, S would be nearer it than the head.
  The twin is older than path W and is a known pattern (472292 / 7499938 in read_1).

### 6757752: right / ok (Elektrorazpredelenie Sever, low-voltage twisted-cable fittings, 4 lots)

- **Procedure:** CN F05 20963427 (2020/S 061-146523, ref. 59/2020), negotiated with call, four lots, 48 months, no
  options: LOT_NO 1 insulated branch clamps, VAL_OBJECT BGN 1,550,000; 2 tension and suspension clamps, 715,000; 3
  insulated press lugs, 107,000; 4 fixing elements, 285,000. Each lot lists its forecast quantities, marked
  "ориентировъчно" (indicative). No procedure total is stated in the CN. F14 20997022 only moves the deadlines.
- **Award:** F06 21360130 (2020/S 222-546802). RES-1..4 award LOT_NO 1–4 under contracts 59-1 … 59-4 (lots 1 and 2 to
  ВАК-02 ООД, lots 3 and 4 to Тилком ООД; 5 / 4 / 3 / 2 tenders). Each VAL_TOTAL equals its VAL_ESTIMATED_TOTAL and the
  CN's VAL_OBJECT. Procedure **VAL_TOTAL = TED-VALUE (PROCUREMENT_TOTAL) BGN 2,657,000**, exactly the sum.
- **F20 23241877** (2023/S 117-367234) modifies contract 59-1 (lot 1): new unit prices after supplier cost rises, and
  "the changes do not alter the subject or the value of the contract". **VAL_TOTAL_BEFORE = AFTER = BGN 1,550,000.** Its
  LotResult restates lot 1 at 1,550,000. S takes lot 1 once, at that value, so nothing pre/post is mixed.
- **Why right:** four distinct product groups, four contracts, each lot awarded once to one supplier; the per-lot
  contract values are single-operator ceilings whose sum is the buyer's own procedure total (read18 note 5). S equals the
  award notice's procedure total to the lev. The head without W is lot 1 alone (58.3 %).

### 6960026: right / ok (Centrum Medyczne „Żelazna”, ultrasound scanners, 2 lots)

- **Procedure:** CN F02 21391452 (2020/S 244-603868), open procedure, "dostawa 3 szt. aparatów ultrasonograficznych",
  two lots ("zakresy"): LOT_NO 1 one USG scanner, LOT_NO 2 two USG scanners. Neither the CN nor its lots state a value.
- **Award:** F03 21615726 (2021/S 060-149792). RES-1 awards LOT_NO 1 to Intimex Sp. z o.o. Sp.k. for PLN 769,000.00
  (1 tender). RES-2 awards LOT_NO 2 to the same firm for PLN 138,500.00 (1 tender; concluded 2021-02-19). Procedure
  **VAL_RANGE_TOTAL LOW = HIGH = PLN 907,500.00** with VALUE_RANGE.TYPE PROCUREMENT_TOTAL. With one tender per lot, the
  lowest and highest offer taken into consideration coincide, so the range is the procedure total. The content prints
  the LOW/HIGH pair twice (ordinals 0 and 1), which is why the port lists four identical range vouches.
- **Why right:** two distinct lots, each awarded once, one supplier and one tender each. S equals the buyer's stated
  procedure total exactly. The range kind is kept by G-cover because 21615726 names both summed keys. The head without
  W is lot 1 alone (84.7 %).
- **Slip check (no verdict impact):** the unit prices differ about 11× (lot 1 PLN 769,000 for one scanner, lot 2
  PLN 69,250 per scanner). I looked for a ×10 slip on lot 2 and found none. The buyer's own procedure total (907,500)
  uses 138,500, while a ×10 lot 2 would give 2,154,000. A premium obstetric scanner for this maternity hospital next to
  two basic units is plausible. No CN figure contradicts either lot.

### 844584: right / ok (Tarn Habitat, central-store supplies 2025–2028, 3 lots)

- **Procedure:** CN subtype 16 24690547 (00252574-2025), open procedure, "accord-cadre à bons de commande sans minimum
  et avec maximum **mono-attributaire**", three lots (BT-765 `fa-wo-rc` each): LOT-0001 plumbing and taps, LOT-0002
  sanitary equipment, LOT-0003 hardware. No value is stated.
- **Award:** CAN subtype 29 24887461 (00445509-2025). RES-0001…0003 each name their own lot (BT-13713) and their own
  tender and contract: SIDER BT-720 **959,000** (contract 2025a0010-01, 3 tenders), RICHARDSON **364,000**
  (2025a0010-02, 4 tenders), FOUSSIER **700,000** (2025a0010-03, 1 tender). Notice-level **BT-118 = EUR 2,023,000.00**,
  the sum of the three BT-720s.
- **Double-count trap, not taken:** each LotResult carries **BT-709 = 2,023,000**, the whole total copied onto every lot.
  The store keeps these as `result_framework_maximum` per lot (3 × would be EUR 6.07 m). S sums the per-lot BT-720 values,
  not the copied BT-709, so it equals the total once. This is 285956's shape (read_a_1).
- **Why right:** three distinct trades, three single-supplier frameworks with distinct winners and contracts; the per-lot
  maxima sum to the buyer's own BT-118 (read18 note 5). The head without W is **null** (BT-709 partner-only rows and
  BT-118 are never elected, 506), so S replaces no figure and cannot be farther.

### 6763031: right / ok (Riigi Tugiteenuste Keskus for Terviseamet, COVID-19 sampling and testing, 2 lots)

- **Procedure:** F21 20974563 (2020/S 067-160206), negotiated without a call (RHS § 49 lg 1 p 3, the unforeseen
  SARS-CoV-2 outbreak), services, two lots: LOT_NO 1 taking samples at sampling points and homes, with logistics and a
  call centre; LOT_NO 2 testing the samples, and lab logistics where the authority takes the samples itself.
- **Award (same notice):** RES-1 (ITEM 1) prints **LOT_NO 2** and awards Synlab Eesti OÜ EUR 1,584,000; RES-2 (ITEM 2)
  prints **LOT_NO 1** and awards OÜ Medicum Eriarstiabi EUR 576,200. The store keys them by LOT_NO, so LOT-2 is Synlab
  (the lab, testing) and LOT-1 is Medicum (sampling), as the lot titles require. Procedure **VAL_TOTAL = TED-VALUE
  (PROCUREMENT_TOTAL) EUR 2,160,200**, exactly the sum. Both results print the same CONTRACT_NO 4.2-1/20/433 although the
  contractors differ, which is a buyer copy and does not make one envelope: each lot has its own scope, winner and value.
- **Modifications:** F20 **21012872** (lot 2, Synlab): extra tests "up to 27,000 tests × EUR 66 = up to EUR 1,782,000"
  (VII.1 VAL_TOTAL 1,782,000, so lot 2 goes from 24,000 to 27,000 tests). F20 **21005314** (lot 1, Medicum): one extra
  mobile team, 120 call-outs × EUR 563 = EUR 67,560 (VII.1 VAL_TOTAL 643,760). Both state **VAL_TOTAL_BEFORE 2,160,200
  and VAL_TOTAL_AFTER 2,425,760** (= 643,760 + 1,782,000, so the AFTER figure covers both modifications). Each F20's own
  LotResult restates its lot's original award (1,584,000 and 576,200), and those are the rows S takes as each lot's
  latest. The AFTER values are a modification kind and are not summed, so nothing pre/post is mixed.
- **Why right:** two distinct lots (sampling vs lab testing), two contractors, each awarded once. S equals the buyer's
  award total to the cent and both F20s' BEFORE total. Against the post-modification EUR 2,425,760, S is −10.9 % and the
  head (lot 1 alone, 26.7 % of S) is −76.2 %. So S is not `farther` on either total. The precedent is 7380025 (S = original
  award while extensions doubled the lots, read right) and M2-fallback's "after modification the contract is worth more
  than S, but that is outside what this lot-sum rule decides".
- **Not the 5816379 shape:** no later notice re-awards a summed lot with a different award. The two later notices are
  F20 modifications whose LotResults restate the original awards. Their different procedure figure is VAL_TOTAL_AFTER, a
  modification total, not a superseding award total.

## Adversarial checks applied to every head (each found nothing)

- **Double count:** every summed key is a distinct printed LOT_NO (TED) or BT-13713 lot (eForms), each with its own
  title or region. Where a lot has two rows (6757752 LOT-1, 6763031 LOT-1 and LOT-2), the rows restate the same value and
  S takes one. 844584's copied BT-709 totals are not summed. No head has a lot-null LotResult row (every served
  result is keyed), so there is nothing keyless that restates a keyed award.
- **Slip, VAT, currency:** every S equals a buyer-stated total in the notices' own currency (EUR, BGN, PLN). Where a CN
  gives lot values (8524598, 6757752), they equal the awards exactly. 6960026's 11× unit-price gap is corroborated by
  the buyer's own total (see above). Nothing suggests a gross/net mix: the lot values and the totals come from the same
  notice or from a CN and CAN that agree.
- **Shared envelope / not_additive:** every lot has one winner and one contract. 844584, 6757752 and 8524598 sum per-lot
  single-operator ceilings (frameworks or unit-price maxima), which read18 note 5 accepts. There is no multi-supplier
  lot, unit rate or nested lot.
- **Vouch belongs to something else (6465510 shape):** each vouching total comes from a notice that awards (or, for
  8524598, estimates) every summed lot: 2, 3 or 4 results naming every summed key, or a CN whose three sections are the
  three summed lots. None equals one result's own value for a lot outside S. G-single and G-cover keep all five, and
  `g_removed` is empty on each.
- **Superseded vouch (5816379 shape):** no later award notice restates a different total for the summed lots. The later
  notices are F20 modifications (6757752 at an unchanged value; 6763031's two, discussed above) or nothing (8524598,
  6960026, 844584 end in their award notice).
- **Farther:** S/T = 0.00 % on all five. The heads without W are null (844584) or one lot's figure (8524598 46.6 %,
  6757752 58.3 %, 6960026 84.7 %, 6763031 26.7 % of S).

## Requests (public API only; log `b6/u5/api_requests.log`, entries tagged `reader=read_a_6`)

21 GETs (16:04:22Z–16:09:40Z), all 200, 0 × 408, 0 retries. Request starts were paced at least 2.2 s apart under the
shared lock (`b6/u5/.http.lock` + `.http.stamp`):
- 5 live heads: `/v1/tenders/{8524598, 6757752, 6960026, 844584, 6763031}`;
- 13 notices, every version notice of the five chains (`/v1/notices/{id}/content`): 23230099, 23558847; 20963427,
  20997022, 21360130, 23241877; 21391452, 21615726; 24690547, 24887461; 20974563, 21012872, 21005314;
- 3 for 8524598's PIN: `/v1/tenders?publication_id=262739-2023`, `/v1/tenders/7711779`, `/v1/notices/23185304/content`.

Bodies are under `b6/u5/bodies/` (`tenders_{id}.json`, `notices_{id}_content.json`; the publication lookup is saved as
`tenders_by_publication_262739-2023.json`). Within the repo, only `b6/u5/` (this file, the bodies and the log lines) was
written. The fetch and dump helpers were session scratchpad files (`scratchpad/u5a6/`, the fetcher copied from
`u5a4` with this reader's tag). There was no /v1/sql, ssh, POST, git or cargo.

**Protocol note:** this file is one reader's read under M2. M2 pairs it with an independent adversarial skeptic per head
and sends any disagreement to a third reader. The checks above are this reader's own adversarial pass, not a substitute
for that skeptic. The two points most worth pressing are 6763031 (S is the original award; the contract grew 12.3 % by
modification) and 8524598 (the PIN's EUR 21.6 m against the CN's EUR 5.85 m, and award = budget on every lot).
