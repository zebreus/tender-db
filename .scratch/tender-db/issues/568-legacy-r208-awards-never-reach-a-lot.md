# 568 — legacy R2.0.8 contract award notices never attach their awards to a lot, even on a multi-lot Tender

Status: done — 2026-10-18. 568's cross-notice R2.0.8 match (86f2745, drained 2593/2594) plus unit 8's refusal guards
G3–G6 as amended (R1/R2, per-language, reader fixes; gated c5977da, deployed fe64be6), drained in one refold with 570
(2616/2617: 46,879 corrected, D7 holds) and READ BACK (wf_f6e14299-ff8 + O1 wf_26ae3991-bdd,
`568-r208-lots/readback-u8/readback.md`): 8327340's 7 multi-lot blocks on the Tender, RES-3/4 and 17840150 RES-1 kept;
the s150's 57 refused blocks null, 1190/1190 = expected; the 77 692/692 unchanged; OOS 100/100 refused; band / path-V 0
movers; fresh 30 (salt "568u8-fresh30-", 8 of 8 G2 after the 3-Tender extension): parity 417/417, 0/30 Tenders keep a
judged-wrong block (6 wrong blocks, all refused by G3/G5). Recorded residual and shapes in `u7/decision-u7.md`
(false refusals 2/2,253: 5457789 RES-8 G4 on a copied title, 4836560 17518149 RES-3 G5 on an inherited carried title;
5602216 RES-128 unclear).
Kind: completeness (legacy lot results: R2.0.8 awards served tender-scoped)
Relates to:
- 567 (same-notice ITEM/LOT_NO mapping in read_legacy_results);
- 534 / 490 (legacy result reading).

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/4748635 | python3 -c "import json,sys; d=json.load(sys.stdin); print([r.get('lot') for r in d.get('lot_results', [])])"

- **done**: an R2.0.8 award block with LOT_NUMBER n on a Tender whose contract notice published lot n serves its result on
  that lot (or the issue records why a cross-notice match is refused).
- **open (2026-10-17)**: every R2.0.8 award result serves `lot: null`.

## What was found

R2.0.8 award forms repeat `AWARD_OF_CONTRACT` with `LOT_NUMBER` and carry no `OBJECT_DESCR` lot section, so
`read_legacy_results`' same-notice lot map is empty and every award resolves to `lot_key = None`. In 72 of 100 sampled
R2.0.8 multi-lot CANs (2012–2015) the Tender has 2 or more lots from its contract notice's annex B; their awards stay
tender-scoped. Spot-checked on the API: 4748635, 4730243, 4945774, 4840245, 5128122, 5086044, 5340243, 5274468 — every
result row from the CAN serves `lot: null`.

## Unit 1 — measured and decided (2026-10-17, workflow wf_4d23aefc-0d0)

Sample: issue 567's part-0 draw (one random OJ S day per month, 2012–2015): 470 R2.0.8 CANs probed, 100 multi-lot
CANs, 71 of them on a Tender with 2+ lots. All 679 award blocks of the 71 were classified by a measurer and re-read by
an adversarial skeptic (19 classifications changed); the judge's port replay (`cross_notice_port.py`) agrees 679/679.

| class | blocks |
|---|---|
| unique_right | 636 (about 275 confirmed by figures or bare labels rather than distinct text; none contradicts) |
| unique_unverifiable | 11 (5018063: identical titles and winner) |
| unique_wrong | 0 |
| ambiguous | 0 |
| no_match | 26 (letters vs numbers, CAN-only renumbering, lots added in corrigendum prose) |
| no_number | 6 |

Decision (full text, rule, tests, drain and read-back: `.scratch/tender-db/568-r208-lots/decision.md`): **BUILD**. A
legacy award block that resolved to no lot in its own notice (`lot_key None`, its own printed number p) takes the lot
an earlier notice of the same Tender printed under p — exact first, then 567's `normalise_lot_number`, only when
exactly one inherited `Lot` prints it; only for an award notice with no lot sections of its own; frozen at the award's
version. G1: a notice whose numbered blocks all print one number on a multi-lot Tender matches none (5476747, from the
judge's extra strata — NOT skeptic-verified — is the shape). G2: an inherited lot republished under another number
disables the match. Never the key ordinal, TED-ITEM, titles, corrigendum prose or CONTRACT_NUMBER. No epoch bump
(ADR-0017 R3 refold). Drain: `refold-fields` over `TED-LOT_NUMBER` (likely a FULL fallback, ~4.3 h), BEFORE 551's B3
parity, with a band / path-V baseline first. Read-back bar: a fresh uniform 30 corrected Tenders (≥ 10 outside
2012–2015), 30/30 with no wrong block.

## Drain result and read-back pre-draw (2026-10-18 ~03:00Z)

- Job 2593 stamped 537,878 Tenders (719,732 TED-LOT_NUMBER carriers); project 2594 (FULL fallback, ok, ~02:55Z):
  537,878 written, compare 381,755 verified + 156,123 corrected = 537,878 (D7 complete; the decision expected
  ~130–160 k corrected); 1,613,712 correction rows; `/v1/changes` cursor 956239230 → 957852942. Mid-drain spot check:
  4748635 serves [LOT-1, LOT-2, LOT-3, LOT-5, LOT-6, LOT-7, LOT-8, LOT-9] (the issue's done line), 4748733 RES-k on
  LOT-k, 5476747's three on none (G1).
- **Fresh-30 pre-draw (fixed before any id is read):** population = every `entity=tender`, `op=changed`,
  `version: null` row with cursor in (956239230, 957852942]; order by `sha256("568-fresh30-" + id)` ascending; walk the
  order reading each Tender's head `published_at` year and take ids until 30 are held with at least 10 published
  outside 2012–2015 (an id from 2012–2015 is skipped once 20 such are held). Bar: 30 of 30 with no wrong block.

## Read-back — FAILED 29 / 30 (2026-10-18, wf_48592647-c5a); decision reopened

Evidence: `568-r208-lots/readback/` (population.json: the 156,123 corrected; fresh30.json: the salted, stratified
draw, 20 from 2012–2015 + 10 outside; measured77.json; band_diff.json; readback_result.json).
- **The 77 measured Tenders:** 692 / 692 blocks exactly where the port put them (nulls included).
- **Band / path V:** one mover — 5617904 left the >= EUR 1 bn band: PLN 6,351,557,614.20 → PLN 63,515,576.14
  `lot-sum-vouched` (537, x100). Read RIGHT: the buyer's own corrigendum 18810459 corrects the total to PLN
  63,515,576.14 to the cent, and the 85 award blocks sum to it exactly. The 16 path-V pins unchanged.
- **Fresh 30:** 29 right, **1 wrong — 8327340** (reader and skeptic independently). Every block is where the rule puts
  it; the fault is a shape the decision did not foresee: 7 of 10 award blocks in CANs 17821481 / 17830774 are ONE
  contract for SEVERAL lots ("обособени позиции 5, 6, 15, 16"; "33, 34, 35" printing the middle number 34), and
  LOT_NUMBER prints only one of them, so the multi-lot award and its value now sit on that one lot (709,431.63 BGN on
  LOT-5 against its CN value 189,067.00; 17830774 RES-1 equals the exact 3-lot sum). Before 568 they stayed on the
  Tender, which was the honest scope. Per the decision ("one wrong reopens this decision") → unit 7.
