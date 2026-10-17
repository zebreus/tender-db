# 568 — legacy R2.0.8 contract award notices never attach their awards to a lot, even on a multi-lot Tender

Status: ready-for-agent — DECIDED 2026-10-17 (wf_4d23aefc-0d0): BUILD the cross-notice match (rule steps 1–9 + G1/G2,
in `fold`). Measured 0 unique_wrong / 647 matched (95 % ≤ 0.57 % of blocks, ≤ 5.1 % of CANs); the one wrong shape
outside the sample (5476747, one LOT_NUMBER on every block) is G1's. Reach ≈ 16 % of R2.0.8 CANs (12.8–19.6 %), 1.39
blocks per CAN (0.80–2.16). NEXT: unit 3 build (project.rs `fold`; land with 569, after 551 B2 is gated), then drain
`refold-fields` TED-LOT_NUMBER BEFORE 551's B3 parity. Decision: `.scratch/tender-db/568-r208-lots/decision.md`.
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
