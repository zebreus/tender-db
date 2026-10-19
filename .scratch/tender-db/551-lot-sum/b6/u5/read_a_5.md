# 551 B6 U5 fresh pre-flip read, round a, part 5: P ranks 21–25

2026-10-19, reader `read_a_5`. These are ranks 21–25 of the fresh read's order over P minus every read id
(`b6/u5/order.json` sha256 3033ec31…, salt `551-b6-fresh-`, `sha256('551-b6-fresh-'+id)` ascending; `first_60[20:25]`
and the entries' ranks checked, order keys recomputed: 01b6c3b1…, 01ccbaa6…, 01d69884…, 01f005c8…, 01fb69d7…):
**64472, 7442464, 5861802, 7085538, 23846**. None is in `excluded_from_P` (every read id, 132), `excluded_sources.json`,
R_single, R_excess or U4's `read_set.json`, so all five are new reads.

**The question for each id:** P = v2+both (3,467 ships, `b6/u2/P.json`). Path W is OFF, so the live head is the
max-elected figure, not the sum. If the flip ships, the tender serves S, the summed lot awards, vouched by the
G-filtered `vouched_by`. Under M2's rubric, is serving S as the tender's value RIGHT, NEUTRAL or WRONG?

**Rubric (M2, wf_dc85fa7e-c00's):** right/ok means S is the procedure's value: distinct lots (TED `LOT-k` checked against
the printed LOT_NO), each lot's latest award, no lot counted twice, no ×10ᵏ slip, no VAT mix, and S no farther from the
procedure's stated total than the head it replaces. Wrong classes: `not_additive` (ceilings or maxima sharing one
envelope), `double_count`, `farther`, `slip`, `other` (for example, the vouching total belongs to something else, as in
6465510). Neutral means S is not wrong but not clearly better than the head without W. **Precedents applied:** read18
note 5 (per-lot single-operator framework ceilings whose sum is the buyer's own total are not `not_additive`), applied to
23846, 64472 and 5861802. Contract modifications are a modification kind that S does not use (U4 read_1, 7380025), which
applies to 7442464 and 7085538.

**Inputs:** `b6/u5/order.json` (P sha256 5f8b7f7c…, ids_sha256 a2619fd9…, decisions.jsonl.gz 0add92e3…, as `order.json`
records; P.json and decisions.jsonl.gz re-hashed here and equal). The port-predicted S, currency, n, summed keys and
notices, and the G-filtered `vouched_by` come from `b6/u2/decisions.jsonl.gz` (v2+both). On all five, v2+both equals v2:
`g_removed`, `g_single_removed` and `g_cover_removed` are empty, `vouched_by` is the same under all four guard sets,
`vouched_by_moved_from_v2` is false, and `flags` is empty (no estimate-twin flag). Every vouch candidate has
`g_single: false, g_cover: false`. Live heads come from `GET /v1/tenders/{id}`. **All five live heads are JSON-equal to
their U2 fresh-layer copies** (`scratchpad/b6/tenders`, fetched 06:15Z–07:31Z), so the port's facts are current. **All 27
version notices** were fetched live and are JSON-equal to their U2 notice-layer copies (`scratchpad/b6/notices`).

## Result

**5 right / 0 neutral / 0 wrong**, all class `ok`. Four heads have S equal, to the cent, to a buyer-stated total that
covers every awarded lot of the procedure. The vouching notice awards (or, for 5861802, also sets out) every summed lot.
The fifth, 64472, has S within 0.56 % of the buyer's BT-118. That BT-118 is the sum of the per-lot framework maxima,
while S sums the per-lot winning tenders, and the live head is null. No part of this read is a partial sum. Two heads
(7442464, 7085538) have post-award modifications that raise single lots, and S leaves them out, as the convention says.

| rank | id | S (port, v2+both) | n | summed: key ← notice, award | vouched_by (G-filtered = v2) | head without W (live) | buyer-stated total(s) matching S | S/T | head/S | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| 21 | 64472 | EUR 334,400.49 | 3 | LOT-0001…0003 ← 24822363: 205,154.67; 19,635.51; 109,610.31 | BT-118-NoticeResult EUR 332,553.00 on 24822363 (3 results, names all 3 keys) | **null** | 24822363 BT-118 332,553.00 (= Σ BT-709 210,967.50 + 18,742.50 + 102,843.00) | +0.56 % | — (null) | **right / ok** |
| 22 | 7442464 | EUR 4,442,428.69 | 7 | LOT-1…7 ← 22708527: 473,967.30; 3,321,138.43; 72,025.20; 147,040.76; 286,315.00; 101,565.00; 40,377.00 | TED-VAL_TOTAL EUR 4,442,428.69 on 22708527 (F06, 7 results, names all 7 keys) | EUR 473,967.30 (lot 1) | 22708527 procedure VAL_TOTAL = TED-VALUE (PROCUREMENT_TOTAL); VAL_TOTAL_BEFORE in all seven F20s | 0.00 % | 10.7 % | **right / ok** |
| 23 | 5861802 | BGN 342,000.00 | 2 | LOT-1 ← 21027845 218,000.00; LOT-2 ← 21027846 124,000.00 (19294684 states the same) | TED-VAL_ESTIMATED_TOTAL BGN 342,000 on CN 19145047 (F02, estimate kind, 2 Lot sections, both in S); also TED-VAL_TOTAL 342,000 on F03 19294684 | BGN 218,000.00 (lot 1) | CN 19145047 VAL_ESTIMATED_TOTAL 342,000; F03 19294684 VAL_TOTAL = TED-VALUE (PROCUREMENT_TOTAL) 342,000 | 0.00 % | 63.7 % | **right / ok** |
| 24 | 7085538 | HUF 1,630,930,456.00 | 5 | LOT-1 ← 23103681 488,400,150; LOT-2 ← 23197701 351,428,477; LOT-3 ← 23106548 470,697,443; LOT-5 ← 23195436 142,861,192; LOT-6 ← 23029966 177,543,194 (all = F03 22173860) | TED-VAL_TOTAL HUF 1,630,930,456 on 22173860 (F03, 6 results incl. LOT-4 unsuccessful; names all 5 keys) | HUF 351,428,477.00 (lot 2) | 22173860 procedure VAL_TOTAL = TED-VALUE (PROCUREMENT_TOTAL) 1,630,930,456 | 0.00 % | 21.5 % | **right / ok** |
| 25 | 23846 | EUR 1,069,830.00 | 3 | LOT-0001…0003 ← 24991969: 833,055.00; 112,389.00; 124,386.00 | BT-118-NoticeResult EUR 1,069,830.00 on 24991969 (3 results, names all 3 keys) | **null** | 24991969 BT-118 1,069,830.00 | 0.00 % | — (null) | **right / ok** |

S_eur: 64472 EUR 334,400.49, 7442464 EUR 4.44 m, 5861802 EUR 174,864.51, 7085538 EUR 4.39 m, 23846 EUR 1.07 m. **B-band
(decision point 8): nothing from this part.** None of the five is a band head (B_P, ≥ EUR 100 m) or within 2 % of
EUR 100 m. None is in `band_set.json`, in the provisional L_changed (239 ids; ∩ 3,502 = 0) or in the 541/545/571 sets.
No live head serves a derivation (`value_derivation: null` on all five).

## Per head

### 64472: right / ok (CPAS de Mouscron, bread, viennoiserie and pastry for four care homes and a crèche)

- **Procedure:** eForms CN 24625630 (subtype 16, `00193178-2025`, open procedure PPP0T6-350/5030/2025/088) states no value.
  It has three Lot sections: LOT-0001 PAINS & DIVERS, LOT-0002 VIENNOISERIES and LOT-0003 PATISSERIES (internal ids
  2025/088_1, _2, _3). Each is a framework without reopening of competition (BT-765 `fa-wo-rc`).
- **Award:** CAN 24822363 (subtype 29, `00432464-2025`). Each RES-0001…0003 names its own lot (BT-13713). All three were
  won by the same bakery, RANSON-CANNIERE BVBA, with 2 tenders per lot. Each points to its own LotTender and its own
  SettledContract (buyer contract ids 1556-3199, 1557-3195 and 1558-3197, all concluded 2025-07-01). Per-lot BT-720
  (winning tender = BT-710, the lowest of the two): 205,154.67 / 19,635.51 / 109,610.31. Per-lot BT-709 (framework
  maximum): 210,967.50 / 18,742.50 / 102,843.00. The notice-level **BT-118 = EUR 332,553.00** is exactly Σ BT-709.
- **Why right:** three distinct product lots, one winner and one contract each. S is the sum of the three awarded tender
  values, +0.56 % from the buyer's own framework total. The head without W is **null**, since BT-709 rows are
  partner-only and BT-118 is never elected (506). So S replaces no figure, cannot be farther, and is the buyer's
  measure within 0.56 %. read18 note 5 applies: each maximum belongs to one lot, which has one operator.
- **Note for the skeptic (no verdict impact):** two notes on basis. (1) S uses the tender values and T the framework
  maxima. For lots 2 and 3 the tender exceeds the maximum by 4.8 % and 6.6 %, and for lot 1 it is 2.8 % below. The
  difference is small and goes both ways, so this is no slip. (2) The lot text says "Les quantités mentionnées dans les
  inventaires correspondent à une période d'un an", with one year renewable twice. The values may therefore be annual,
  not for the full term. S and BT-118 rest on the same basis, and no figure on any other basis exists to compare.

### 7442464: right / ok (DB Netz AG, ABS/NBS Karlsruhe–Basel PfA 8.3+8.4, advance environmental measures, Bad Krozingen)

- **Procedure:** CN F05 22530364 (2022/S 133-380937; utilities, works, open), seven lots and no value stated: LOT_NO 1
  tree and shrub planting; 2 protective fences, deterrence and replacement habitats for reptiles; 3 field-bird habitat;
  4 clearing; 5 reptile capture and relocation; 6 amphibian capture and relocation; 7 cavity-tree checks and nest boxes.
  F14 22586819 only changes contacts, deadlines and the award criteria (price only), with no values.
- **Award:** F06 22708527 (2022/S 207-592382). RES-1…7 each print their own LOT_NO 1…7, with contracts concluded
  2022-10-21. Lots 1–3 went to Toriello GmbH, lot 4 to Baumschule Simon Heiss, and lots 5–7 to S.R.T. Garten- und
  Landschaftsbau. Each had 2–3 tenders and none was awarded to a group. The per-lot values are listed in the table.
  Procedure **VAL_TOTAL EUR 4,442,428.69** = Σ of the seven, exactly.
- **Seven F20s (2023-04-28 … 2023-10-10)** modify lots 2 (×4), 3 and 1 (×2): higher reptile fences, extra field
  areas and extra watering. Each states **VAL_TOTAL_BEFORE = EUR 4,442,428.69** (= S) and a VAL_TOTAL_AFTER of
  4,449,366.19 to 4,787,868.19. Each LotResult restates the lot's **original** award (3,321,138.43 / 72,025.20 /
  473,967.30), and each F20's own `DESCRIPTION_PROCUREMENT.VAL_TOTAL` is an EUR 1.00 placeholder. The store keeps these
  results on lot-null rows, so S does not sum them. Even if they were keyed, S would not move, because they restate the
  original values. The placeholders are not vouch candidates (the port lists one candidate only).
- **Why right:** seven distinct work packages and three contractors, each lot awarded once. S equals the buyer's award
  total, stated in the F06 and again as the "before" value in all seven modifications. The increases are modification
  kinds, which S does not use (7380025 convention). Adding them all would give about EUR 5.51 m if they are independent
  increments. S is 80.7 % of that, and today's head (lot 1 alone, EUR 473,967.30) is 8.6 % of it. So S is clearly
  better either way, and it is not `farther`.

### 5861802: right / ok (TP DLS Vitinya, servicing and repair of the service vehicles, 36 months)

- **Procedure:** CN F02 19145047 (2017/S 040-073078, open, LOT_ALL), two lots. LOT_NO 1 covers servicing and repair of
  the service vehicles, with parts and consumables, by an attached list (VAL_OBJECT BGN 218,000). LOT_NO 2 covers vehicles
  and motorcycles of Soviet and Russian make (VAL_OBJECT BGN 124,000). Procedure **VAL_ESTIMATED_TOTAL BGN 342,000.00**
  (TED-VALUE ESTIMATED_TOTAL).
- **Award:** F03 19294684 (2017/S 105-210427). RES-1 (LOT_NO 1) is contract 39 and RES-2 (LOT_NO 2) is contract 40, both
  concluded 2017-05-15. Both went to „Геотрейдинг" АД, with 2 tenders each and no group. Their VAL_TOTAL = VAL_ESTIMATED_TOTAL
  is 218,000.00 / 124,000.00. Procedure **VAL_TOTAL BGN 342,000.00** (TED-VALUE PROCUREMENT_TOTAL).
- **F20s 21027845 / 21027846 (2020/S 093-222966 / 222967)** extend each contract's term "до изчерпване на финансовия
  ресурс от 218 000 / 124 000 BGN без ДДС" (until the financial resource of BGN 218,000 / 124,000 net of VAT is used up)
  or a new contract. **VAL_TOTAL_BEFORE = AFTER** on each, and each LotResult restates its lot at the same value. Each F20's
  single Lot section carries section id LOT-1 but prints LOT_NO 1 and LOT_NO 2 respectively. The store keys them LOT-1 and
  LOT-2 correctly, and S takes each lot once from its latest row (the F20), at the unchanged award value.
- **Why right:** two distinct lots with two contracts. The lots differ in scope: general fleet against Soviet- and
  Russian-made vehicles and motorcycles. S equals the buyer's procedure total in both the CN and the F03. Each lot's
  figure is a single-operator financial ceiling for that lot's contract, net of VAT on both sides, so read18 note 5
  applies and S is not `not_additive`. The estimate vouch comes from a two-section CN whose lots are both in S, so the
  estimate-twin flag does not apply. The head without W is lot 1 alone (63.7 %).

### 7085538: right / ok (Széchenyi István Egyetem, energy-efficiency renovation of six university buildings)

- **Procedure:** CN F02 21726400 (2021/S 095-248766, open, works, KEHOP-5), six lots, one building each: 1 Hotel Famulus,
  2 Lucsony "C", 3 Petz Iskola, 4 Posta, 5 Tankonyha and 6 Várkapitányi épület. No value is stated. F14 21730684 only
  moves the deadline.
- **Award:** F03 22173860 (2022/S 016-037116), contracts concluded 2022-01-10. RES-1 lot 1 went to Fodor Építőipari
  Kft. for 488,400,150. RES-2 lot 2 went to Szabó és Társa for 351,428,477. RES-3 lot 3 went to Euro Generál Zrt. and
  Fodor as **joint tenderers** (AWARDED_TO_GROUP; INFO_ADD "közös ajánlattevők", one tender) for 470,697,443. **RES-4
  lot 4 (Posta) is PROCUREMENT_UNSUCCESSFUL** ("kizárólag érvénytelen ajánlatot nyújtottak be", only invalid tenders;
  served `clos-nw`). RES-5 lot 5 went to Szabó és Társa for 142,861,192, and RES-6 lot 6 to Szabó és Társa for
  177,543,194. Procedure **VAL_TOTAL HUF 1,630,930,456** = Σ of the five awarded lots, exactly.
- **Six F20s (2022-06-20 … 2023-06-02)**, one per awarded lot (lot 6 ×2), each print their LOT_NO and are keyed to the
  right lot. Each LotResult restates the **original** award, which is what S sums. The lot 2, 3 and 6 notices are
  product substitutions, extra works charged to the contract reserve, or billing changes, with VAL_TOTAL_BEFORE = AFTER.
  Lot 1 rises to 542,263,323 (extra façade windows, LED lighting and lettering, air conditioning). Lot 5 rises to
  173,882,009 (greenhouse demolition and rebuild, substrate repairs). These are modification values, and S does not use
  them. There is no pre/post mix, because every summed figure is an original award.
- **Why right:** five distinct buildings and five contracts. Lot 3's two winners form one consortium award, not two
  tenders, so it is counted once. S equals the buyer's procedure total to the forint. The unsuccessful lot 4 is
  correctly absent: the buyer's own total leaves it out too. The head without W is lot 2 alone (21.5 %). With the two
  increases, the post-modification figure would be HUF 1,715,814,446. S is 95.1 % of it and the head 20.5 %, so S is
  clearly better either way.

### 23846: right / ok (CARSAT Alsace-Moselle, office cleaning and window cleaning, 3 territorial lots)

- **Procedure:** eForms CN 24672914 (subtype 16, `00148355-2025`, open, services, 2025/M002 à M004), three Lot sections, each a
  48-month framework without reopening of competition (`fa-wo-rc`): Lot 1 Strasbourg, Lot 2 Bas-Rhin and Haut-Rhin
  outside Strasbourg, Lot 3 Moselle (57). No value is stated.
- **Award:** CAN 24991969 (subtype 29, `00577413-2025`). RES-0001 → LOT-0001 Caronet (11 tenders), RES-0002 → LOT-0002
  Lustral (6), RES-0003 → LOT-0003 Lustral (6). Each has its own LotTender (BT-13714 its lot) and its own SettledContract
  ("LOT 1", "LOT 2", "OFFRE LUSTRAL - LOT 3", concluded 2025-07-23). BT-720 = BT-709 per lot: 833,055.00 / 112,389.00 /
  124,386.00. Notice-level **BT-118 = EUR 1,069,830.00** = their sum, exactly.
- **Why right:** three distinct territories with one winner and one contract each. Lustral holds two lots, but with
  separate contracts for separate areas. S equals the buyer's framework total to the cent. read18 note 5 applies:
  per-lot single-operator maxima, no shared envelope. The head without W is **null** (BT-709 partner-only rows and
  BT-118 are never elected, 506), so S cannot be farther.

## Adversarial checks applied to every head (each found nothing)

- **Double count:** every summed key is a distinct printed LOT_NO (TED) or BT-13713 lot (eForms) with its own title.
  Where a lot has several rows, they restate the same value and S takes one (5861802 LOT-1/LOT-2 in the F03 and F20;
  7085538 LOT-6 ×3 and the other lots ×2). 7442464's seven F20 restatements sit on lot-null rows and are not summed.
  No eForms BT-709 is copied across lots: each lot's BT-709 is its own and differs (unlike 285956's shape in read_a_1).
- **Slip, VAT, currency:** four S equal a buyer-stated total to the cent, and 64472 is +0.56 % from its BT-118, in each
  notice's own currency (EUR, BGN, HUF). Where a CN gives lot values (5861802), they equal the awards. No figure is 10ᵏ
  off a sibling figure. 5861802's values are net of VAT on both lots. 7085538's lot values add exactly to the procedure
  total.
- **Shared envelope / not_additive:** 23846, 64472 and 5861802 sum per-lot single-operator maxima or ceilings, one per
  lot and contract (read18 note 5). No lot has two parallel suppliers. 7085538 lot 3 is a joint tender, which is one
  award. 7442464's lots are separate works with separate contractors or contracts.
- **Vouch belongs to something else (6465510 shape):** every vouching total comes from a notice that awards every summed
  lot (5861802 also has its CN estimate, from a two-section CN with both lots in S). No vouching total equals one
  result's own value for a lot outside S. All five are kept by G-single and G-cover alike (every candidate has
  `round_results` ≥ 2 and names the summed keys, or is estimate kind).
- **Superseded vouch (5816379 shape):** no later award notice re-awards a summed lot at a different value or states a
  different procedure total. The later notices are F20 modifications. They restate the original lot award in their
  LotResult and give the original procedure total as VAL_TOTAL_BEFORE (7442464), or the lot's own value
  (7085538, 5861802). Their VAL_TOTAL_AFTER figures are modification values.
- **Farther:** S/T = 0.00 % on four heads and +0.56 % on 64472. The heads without W are null (64472, 23846) or a single
  lot's figure (7442464 10.7 %, 7085538 21.5 %, 5861802 63.7 % of S). On the two heads with modifications, S stays far
  nearer the post-modification figure than the head does.

## Requests (public API only; log `b6/u5/api_requests.log`, entries tagged `reader=read_a_5`)

32 GETs, 16:04:02Z–16:05:50Z, all 200, 0 × 408, 0 retries. Request starts were paced at least 2.2 s apart under the
shared lock (`b6/u5/.http.lock` + `.http.stamp`, used alongside `read_a_6`). The smallest gap between any `read_a_5`
request and its neighbour in the log is 2.20 s.
- 5 live heads: `/v1/tenders/{64472, 7442464, 5861802, 7085538, 23846}`;
- 27 notices, every version notice of the five chains (`/v1/notices/{id}/content`): 24625630, 24822363; 22530364,
  22586819, 22708527, 23080444, 23083404, 23272636, 23418423, 23418650, 23444169, 23471105; 19145047, 19294684,
  21027845, 21027846; 21726400, 21730684, 22173860, 22453026, 23029966, 23103681, 23106548, 23195436, 23197701;
  24672914, 24991969.

Bodies are under `b6/u5/bodies/` (`tenders_{id}.json`, `notices_{id}_content.json`). Nothing outside `b6/u5/` was
written except the reader's own helper scripts in the session scratchpad. There was no /v1/sql, ssh, POST, git or cargo.

**Protocol note:** this file is one reader's read under M2. M2 pairs it with an independent adversarial skeptic per head
and sends any disagreement to a third reader. The checks above are this reader's own adversarial pass, not a substitute
for that skeptic. For the skeptic, the points most worth pressing are these. First, 64472's 0.56 % gap between S
(tender values) and BT-118 (framework maxima), and its possibly annual basis. Second, the modification exclusion on
7442464 and 7085538. Third, 7085538's unsuccessful lot 4 and the joint award on lot 3.
