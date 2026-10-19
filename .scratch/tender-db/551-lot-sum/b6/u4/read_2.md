# 551 B6 U4 read 2: guard-cost read, R_single salted ranks 7–12

2026-10-19, issue 551 decision point 3 (adoption rule) and U2's adoption-rule gap. Read set `b6/u4/read_set.json`
(salt `551-b6-gcost-`, sha256 ascending). These six are R_single ranks 7–12 (re-derived here: the first 30 of
R_single's 34 in salted order equal `read_set.json`'s list). **None has a prior verdict.** Checked against b3/read18,
the B5 investigation 61, the post-drain 279, and M2 plus its fallback: no hit.

**Question per id:** G-single (and, where noted, G-cover) refuses this v2 ship, which serves the summed lot awards S as
the tender's value, vouched by a stated total. Is serving S RIGHT, NEUTRAL or WRONG as the tender's value, per M2?

**Rubric (M2, wf_dc85fa7e-c00's):** read each head against its notices: the procedure's stated total, each lot's latest
award, distinct lots (a TED `LOT-k` is an ordinal, so it is checked against LOT_NO), slips, and the figure S replaces.
The verdict is right / neutral / wrong; a wrong carries a class (not_additive, double_count, farther, slip, other).
As applied in the B5 read (6465510), a vouching total that belongs to something else (a non-summed lot's own award)
is wrong even when S is nominally closer to the truth.

**Inputs:** port-predicted decisions from `b6/u2/decisions.jsonl.gz` (sha256 0add92e3…). Tender facts come from the
fresh U2 layer (`scratchpad/b6/tenders`) and notices from `scratchpad/b6/notices`. The live head is a public GET
`/v1/tenders/{id}`, and all 6 are **byte-identical** to the U2 layer (sha256 equal to each sidecar). The vouching and
summed-award notices were re-opened live with `/v1/notices/{id}/content`: 18 notices, every one JSON-equal to the
U2 layer. Bodies are in `b6/u4/bodies/` (`tenders_{id}.json`, `notices_{id}_content.json`).

## Result

- **5 right, 0 neutral, 1 wrong** (6582856, class other).
- **G-single slice (counts toward its 30):** 6 read, 1 non-right (6582856).
- **G-cover overlap ids in this slice:** 6582856 (wrong) and 7423850 (right). Both keep these verdicts in the gap case
  (G-cover judged on all 13 refusals).
- None of the six is a band head: the largest S is EUR 52.75 m (7608791).
- **Shape of G-single's cost here.** The 5 rights all lose a genuine total, from one of two shapes:
  - In 4 of them (5581720, 6342738, 7348648, 7608791), the vouching notice has one LotResult, but its
    procedure-scope VAL_TOTAL is a running or all-lot total. The buyer restates the procedure's total on the notice
    that awards one lot. In 5581720 the notice says so in words.
  - In the 5th (7423850), the notice's single LotResult covers both lots ("1 y 2").

  G-single's premise ("a one-result notice's total is that result's own award") fails on all five. It holds only on
  the wrong (6582856), which G-cover also refuses.

| rank | id | S (n) | v2 vouched_by | G-filtered vouched_by | refusing | live head (= head without W) | stated total T (source) | S/T | S/truth | head/truth | reader | skeptic | final |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 7 | 6582856 | EUR 4,160.40 (7) | TED-VAL_TOTAL 4,200.00 on 20932731 | none (single and cover both remove it) | R10 under single, cover, both | EUR 945.00 | EUR 4,200.00 = **lot 6's own award** (F03 20932731; lot 6 not in S) | −0.94% | −50.24% (truth 8,360.40) | −88.70% | wrong/other | wrong/other | **wrong** |
| 8 | 7348648 | HRK 97,626,068.20 (17) | TED-VAL_TOTAL 97,626,068.20 on 22886555 | none under single; cover keeps 22886555 | R10 under single, both | HRK 9,709,000.00 | HRK 97,626,068.20 (F03 22886555, cumulative procedure total) | +0.00% | +0.00% (−0.91% vs post-modification) | −90.05% | right/ok | right/ok | **right** |
| 9 | 7423850 | EUR 1,176,865.00 (2) | TED-VAL_TOTAL 1,176,865.00 on 22432073 | none (single and cover both remove it) | R10 under single, cover, both | EUR 578,250.00 | EUR 1,176,865.00 (F03 22432073, its one result is "LOT_NO 1 y 2") | +0.00% | +0.00% (award basis) | −50.87% | right/ok | right/ok | **right** |
| 10 | 6342738 | EUR 180,736.00 (2) | TED-VAL_TOTAL 180,736.00 on 20223375 | none under single; cover keeps it | R10 under single, both | EUR 59,955.20 | EUR 180,736.00 (F21 20223375, cumulative; = F20s' VAL_TOTAL_AFTER) | +0.00% | +0.00% | −66.83% | right/ok | right/ok | **right** |
| 11 | 5581720 | EUR 1,380,000.00 (2) | TED-VAL_TOTAL 1,380,000.00 on 18983294 | none under single; cover keeps it | R10 under single, both | EUR 180,000.00 | EUR 1,380,000.00 (F03 18983294: "correspond à la somme des montants maxima des 2 lots") | +0.00% | +0.00% | −86.96% | right/ok | right/ok | **right** |
| 12 | 7608791 | EUR 52,750,000.00 (2) | TED-VAL_TOTAL 52,750,000.00 on 23061269 | none under single; cover keeps it | R10 under single, both | EUR 6,600,000.00 | EUR 52,750,000.00 (F03 23061269 = lot 1 + lot 2) | +0.00% | +0.00% | −87.49% | right/ok | right/ok | **right** |

In every case the port's v2 ships with that single candidate, and removing it leaves R10 with no candidate. No guard
shifts `vouched_by` to another total.

## Per id

### 6582856: wrong / other (G-single and G-cover both refuse)

- **Tender.** Rīgas Stradiņa universitāte, "Dažādu medicīnas aprīkojuma un ierīču iegāde — 8. kārta" (RSU 2019/49/SD-AK).
  CN 20568018 (F02, 2019-08-16) has 9 lots (LOT_NO 1–9) and states no value at all.
- **S** = EUR 4,160.40 over n = 7:

  | key | award (EUR) | notice |
  |---|---|---|
  | LOT-1 | 945.00 | 20989144 |
  | LOT-3 | 320.40 | 20722713 |
  | LOT-4 | 388.00 | 20781489 |
  | LOT-5 | 1,280.00 | 20738475 |
  | LOT-7 | 774.00 | 20722713 |
  | LOT-8 | 115.00 | 20738475 |
  | LOT-9 | 338.00 | 20781489 |

  Each award's printed LOT_NO matches its key.
- **Vouched by** TED-VAL_TOTAL EUR 4,200.00 on **20932731** (F03, 2020-03-16).
  - That notice has a single Lot section, LOT_NO **6** ("Ārējais automātiskais defibrilators"). Its one LotResult
    (contract 6, no LOT_NO printed) is VAL_TOTAL 4,200.00, and the notice's procedure VAL_TOTAL is the same 4,200.00.
  - The live `lot_results` store that award at **lot = null** (EUR 4,200.00), so lot 6 is outside S.
  - port: `round_results` 1, `names_summed_keys` [].
- **Every award notice states only its own total:**

  | notice | lots | stated total (EUR) |
  |---|---|---|
  | 20722713 | 3 + 7 | 1,094.40 |
  | 20738475 | 5 + 8 | 1,395.00 |
  | 20781489 | 4 + 9 | 726.00 |
  | 20932731 | 6 | 4,200.00 |
  | 20989144 | 1 | 945.00 |

  20957018 closes lot 2 with no award. No notice states a total for the whole procedure.
- **Truth.** 8 lots were awarded, for EUR 8,360.40. S covers 49.8 % of it. The head without W (EUR 945.00, lot 1) covers
  11.3 %.
- **Why wrong.** This is exactly 6465510's shape: a single-lot award notice's own figure lands within 1 % of S by
  coincidence (−0.94 %). The vouching total belongs to something else, and no total of this procedure vouches S. S is
  nominally closer to the truth, but the rubric's wrong criterion applies, as it did to 6465510.
- **Skeptic.**
  - Could 4,200.00 be a procedure total? No. Each of the five award notices' totals equals its own awards, and 20932731
    awards only lot 6.
  - No double count, no slip.
  - Upheld wrong/other.

### 7348648: right (G-single refuses; G-cover keeps)

- **Tender.** Ličko-senjska županija, "Usluge smještaja i prehrane … za zbrinjavanje raseljenog stanovništva iz Ukrajine".
  - CN 22311066 (F02, 2022-04-01): 36 lots, VAL_ESTIMATED_TOTAL HRK 430,389,750.00.
  - Each lot's VAL_OBJECT = persons per day × 365 × HRK 350.
- **S** = HRK 97,626,068.20 (EUR 12,957,205.95) over n = 17.
  - 16 lots come from F03 **22609100** (2022-08-23), in HRK:

    | key | award | key | award |
    |---|---|---|---|
    | LOT-1 | 5,913,000.00 | LOT-28 | 1,868,800.00 |
    | LOT-3 | 6,296,250.00 | LOT-29 | 3,403,260.00 |
    | LOT-6 | 3,164,915.00 | LOT-30 | 2,641,870.00 |
    | LOT-8 | 2,754,290.00 | LOT-31 | 7,933,275.00 |
    | LOT-14 | 6,643,000.00 | LOT-32 | 5,490,330.00 |
    | LOT-17 | 3,039,720.00 | LOT-33 | 2,261,058.20 |
    | LOT-23 | 1,401,600.00 | LOT-34 | 22,484,000.00 |
    | LOT-27 | 10,577,700.00 | LOT-35 | 9,709,000.00 |

    LOT-35's latest row comes from F20 23005980 and carries the same at-conclusion figure.
  - LOT-7 (HRK 2,044,000.00) comes from F03 **22886555** (2023-01-04).
- **Vouched by** TED-VAL_TOTAL HRK 97,626,068.20 on 22886555.
  - That notice has one LotResult (lot 7), so G-single removes the total. It names LOT-7, so G-cover keeps it.
  - 22609100's own procedure total, HRK 95,582,068.20, is the 16 lots exactly.
  - 22886555's total = 95,582,068.20 + 2,044,000.00. It is the buyer's running total of all 17 awarded lots.
  - The other 19 lots carry NO_AWARDED_CONTRACT in 22609100 and are never re-awarded (the chain has 4 versions).
- **Checks.**
  - Each award's VAL_ESTIMATED_TOTAL equals its LOT_NO's CN VAL_OBJECT (LOT-8 2,810,500; LOT-27 10,731,000; LOT-35
    9,709,000; …). So the RES-k / LOT_NO offset (RES-7 is LOT_NO 8) is keyed correctly.
  - No lot is counted twice. Every award is at or below its estimate. Each lot is a separate site with its own provider,
    so there is no shared envelope.
  - F20 23005980 (lot 35) states BEFORE 10,971,170.00 and AFTER 11,981,672.50. These are gross at 13 % VAT
    (9,709,000 × 1.13), a net increase of 894,250.00. S keeps the at-conclusion figure, as for every other lot. Against
    the post-modification truth (98,520,318.20) S is −0.91 %.
- **Head without W:** HRK 9,709,000.00 (lot 35), −90.1 %.
- **Final:** right/ok, reader and skeptic agree.

### 7423850: right (G-single and G-cover both refuse)

- **Tender.** Osakidetza: centralised supply of HERCEPTIN® (trastuzumab), negotiated procedure without publication, ref
  2022/00061. Two lots: LOT_NO 1 intravenous, LOT_NO 2 subcutaneous.
- **S** = EUR 1,176,865.00 over n = 2:
  - LOT-1 578,250.00 from F20 23566768 (2023-11-27);
  - LOT-2 598,615.00 from F20 23257062 (2023-06-13).

  Both are LotResult VAL_TOTALs (value at conclusion, contract 1, 2022-06-17, Roche Farma).
- **Vouched by** TED-VAL_TOTAL EUR 1,176,865.00 on F03 **22432073** (2022-06-24).
  - Its single LotResult prints **LOT_NO "1 y 2"** (title "LOTE 1 … Y LOTE 2 …"), with VAL_TOTAL 1,176,865.00, equal to
    the procedure VAL_TOTAL.
  - The live store holds that result at lot = null, so the port sees `names_summed_keys` [] and `round_results` 1.
    Both guards remove it.
  - The vouch is the award of exactly the two summed lots, and S splits it 578,250.00 + 598,615.00 = 1,176,865.00.
- **Modifications.** 23257221 (lot 1) and 23257062 (lot 2) extend each lot: BEFORE 578,250 / AFTER 1,156,500 and
  BEFORE 598,615 / AFTER 1,197,230. 23566768 (lot 1, reference-price update) states BEFORE 578,250 / AFTER 1,156,500.
- **Basis.** Both summed figures are at conclusion, so this is not 1091419's mixed pre/post-modification basis.
  - On the award basis: S = T. The head without W (EUR 578,250.00, lot 1) is −50.9 %.
  - On the post-extension basis (2,353,730): S is −50.0 % and the head −75.4 %.
- **Skeptic.** The lot-null "1 y 2" row equals S and is not added to it, so there is no double count. No slip, two
  distinct lots. Upheld right/ok.

### 6342738: right (G-single refuses; G-cover keeps)

- **Tender.** Eesti Töötukassa, "Tõstukijuhi koolitus" (forklift-driver training, ref 201908). Two lots, Tallinn: LOT_NO 1
  in Estonian, LOT_NO 2 in Russian. Contracts run 12 months. CN 20057409 (F21) states no value.
- **S** = EUR 180,736.00 over n = 2:
  - LOT-1 59,955.20 from F20 20909307;
  - LOT-2 120,780.80 from F20 20909303 (both 2020-02-04).
- **Vouched by** TED-VAL_TOTAL EUR 180,736.00 on F21 **20223375** (2019-01-17).
  - Its one LotResult is LOT_NO 2: 120,780.80, contract 7-6.3/19/0012.
  - The earlier F21 20239001 (2019-01-15) awards lot 1 at 59,955.20 (contract 0011) and states procedure total
    59,955.20.
  - 20223375's procedure total is the running total 59,955.20 + 120,780.80.
- **The three F20s** (20909302 and 20909307 for lot 1, 20909303 for lot 2) correct the winner. Their text says the first
  notice named the wrong company. Values are unchanged, and all three state VAL_TOTAL_AFTER 180,736.00.
- **Head without W:** EUR 59,955.20 (lot 1), −66.8 %.
- **Skeptic.** Lot 1 has two F20 rows, and S takes one per key. No double count, slip or shared envelope. Upheld
  right/ok.

### 5581720: right (G-single refuses; G-cover keeps)

- **Tender.** Ville de Paris, "2015 DFA 124", order-based contracts (bons de commande), 48 months. Lot 1 is bungalows,
  lot 2 mobile sanitary units.
  - CN 18579987 (2015-12-26): lot 1 at minimum 30 k / maximum 180 k EUR HT; lot 2 at minimum 300 k / maximum
    1.2 m EUR HT.
  - The CN's "montants cumulés": maximum 1,380,000 EUR HT.
- **S** = EUR 1,380,000.00 over n = 2:
  - LOT-1 180,000.00 (F03 19107853, 2017-01-12, Sermat);
  - LOT-2 1,200,000.00 (F03 18983294, 2016-10-25, PSV).
- **Vouched by** TED-VAL_TOTAL EUR 1,380,000.00 on 18983294.
  - That notice has one LotResult (lot 2), so G-single removes the total. It names LOT-2, so G-cover keeps it.
  - The notice's own text: *"La valeur totale finale de 1 380 000 EUR HT, mentionnée dans le présent avis, correspond
    à la somme des montants maxima des 2 lots."*
- **Head without W:** EUR 180,000.00 (lot 1), −87.0 %.
- **Skeptic.** Not additive? Each lot is its own order-based contract, with its own supplier and maximum. The buyer
  states the sum itself, and S is on the same basis as the head. No slip. Upheld right/ok.

### 7608791: right (G-single refuses; G-cover keeps)

- **Tender.** Senatsverwaltung für Bildung, Jugend und Familie (Berlin), "Rahmenvereinbarung je Los zur Überlassung von
  Personal-Zeitarbeitskräften" (2023-ZS_C_2-allg01_2023). Two lots, 48 months. CN 22881461 states no value; the
  maximum per lot ("Höchstwert … je Los") is in the tender documents.
- **S** = EUR 52,750,000.00 over n = 2:
  - LOT-1 46,150,000.00 (F03 23061269);
  - LOT-2 6,600,000.00 (F03 23063124).

  Both notices are dated 2023-03-31, both lots went to I.K. Hofmann, and they are separate contracts (allg01/2023;
  allg01/2023_Los 2).
- **Vouched by** TED-VAL_TOTAL EUR 52,750,000.00 on 23061269.
  - That notice has one LotResult (lot 1), so G-single removes the total. It names LOT-1, so G-cover keeps it.
  - 23061269's procedure total is 46,150,000 + 6,600,000 exactly. 23063124's procedure total (6,600,000) is lot 2's
    own.
- **Head without W:** EUR 6,600,000.00 (lot 2), −87.5 %.
- **Skeptic.**
  - Slip? Lot 1's 46.15 m appears in two fields (VAL_TOTAL, VAL_ESTIMATED_TOTAL) and in the buyer's procedure total,
    and the arithmetic is consistent. There is no ×10ᵏ evidence.
  - Shared envelope? No. There is one framework per lot ("je Los") with its own maximum.
  - Upheld right/ok.

## Protocol notes and deviations

- **Reader and skeptic were one agent.** M2 calls for a reader and an adversarial skeptic on every head, with a third
  reader on disagreement. Here a single agent ran both passes in sequence. The skeptic pass looked specifically for:
  - double counts;
  - shared envelopes and not-additive maxima;
  - ×10ᵏ slips;
  - superseded or mixed-basis values (the 5816379 and 1091419 shapes);
  - coincidence vouches (6465510's shape);
  - partial sums (7749755's shape).

  The two passes agreed on all 6, so no third reader was needed. They are not independent in the way two separate
  agents would be. If the adoption tally lands within one verdict of its threshold, an independent skeptic on these six
  would close that.
- **API use: 24 GETs, all 200, no 408, no retry.**
  - 6 × `/v1/tenders/{id}` and 18 × `/v1/notices/{id}/content`.
  - Logged in `b6/u4/api_requests.log` (`reader=read_2`).
  - Spaced at 2 s or more across all readers: the sibling's `.http.lock` / `.http.stamp` were shared, and the minimum
    gap in the log is 2.05 s.
- Nothing else was fetched or written: no `/v1/sql`, ssh, POST, git or cargo.
