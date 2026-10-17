# 551 unit 4b, M1: independent fidelity check of rule_v2.py (2026-10-17)

**Verdict: 0 mismatches on 10 ids (6 ships, 4 refusals). 0 ship/refuse flips.** On all 4 refusals the first refusing
check is the same as the port's. Every fact that R1, R2, R3, R10 and v1's T', K and G0 read agrees with
`rule_v2.py`'s facts for these ids: 550 field comparisons, plus the by-hand reads below. Nothing here argues for a new
salt or a re-walk.

The rule is `rule_v2.py`, sha256 `39be6d1ebc0ed2828ce3dd2a10b4db33f98bdf83ec5c5a7bb2d5fe4401a305ce`, unchanged since the
walk. The port's facts are the walk's `walk_facts/{id}.json`. I re-ran `facts()` from the cache, and it returns them
byte for byte.

## The 10 ids

They were picked to cover different sources, currencies, chain shapes and refusal reasons.

| id | port | mine | first check (port / mine) | S | n | S_eur | shape |
|---|---|---|---|---|---|---|---|
| 6094289 | ship | ship | - / - | HUF 139,974,900.00 | 5 | EUR 406,313.21 | TED r209 F03, 7 notices incl. 3 F20 modifications, vouched by VAL_TOTAL |
| 236597 | ship | ship | - / - | PLN 1,749,349.31 | 11 | EUR 409,511.05 | eForms, 12 notices, 10 CANs, one closed lot |
| 307725 | ship | ship | - / - | EUR 8,892,084.70 | 2 | EUR 8,892,084.70 | eForms framework (BT-709/660/118/1118), the largest walked ship |
| 321859 | ship | ship | - / - | CHF 3,369,583.40 | 2 | EUR 3,688,248.03 | eForms, two single-lot CANs, same winner |
| 855538 | ship | ship | - / - | EUR 7,278,412.00 | 20 | EUR 7,278,412.00 | eForms, 20 of 21 lots in one CAN, no stated total anywhere |
| 533536 | ship | ship | - / - | EUR 64,250.00 | 3 | EUR 64,250.00 | eForms, 3 CANs + 3 can-modif restating lot 1 |
| 262408 | refuse | refuse | R1 / R1 | EUR 40,400,000.00 | 3 | EUR 40,400,000.00 | eForms multi-winner framework; the only walked id at EUR 10 m or more (R3, R10 also) |
| 7495977 | refuse | refuse | R2d / R2d | EUR 599,310.00 | 4 | EUR 599,310.00 | TED r209, a later single-lot CAN with ITEM but no LOT_NO |
| 7298633 | refuse | refuse | R2c / R2c | PLN 3,583,172.35 | 8 | EUR 781,413.66 | TED r209 ordinal keys; a later CAN's LOT_NO 4 keyed LOT-1 (R7 also) |
| 975665 | refuse | refuse | R3 / R3 | EUR 426,345.66 | 2 | EUR 426,345.66 | eForms modification re-keys the one lot as LOT-0001 (R6 also) |

The refusal lists agree too:
- 262408: R1, R3 and R10 (I did not derive the port's R5.5, R5.6, R5.7 and R5.11 independently, but they hold by eye; see below).
- 7298633: R2c and R7.
- 975665: R3 and R6.
- 7495977: R2d only.

## Method

- **Fresh reads.** I made 59 GETs to the public API (one every 1.6 s, no 408s, finished 15:14:13Z): `/v1/tenders/{id}` for the
  10 ids, and `/v1/notices/{id}/content` for each of the 49 notices they name in versions, lot results, bids and
  contracts. All 10 details and all 49 notices are identical, as parsed JSON, to the port's cache from the walk, so the
  walk's inputs had not moved.
- **Independent derivation** (`fidelity/derive.py`). I wrote it from issue 551's rule text and its adopted amendments (the
  frozen field-to-kind table). It does not import `rule_v2.py` or `merge_rule.py`, and it does several things the port
  does not:
  - It reads winners and the LotTender grouping from the award notice's own sections. For eForms that is LotResult ->
    OPT-320 (in its LotResultTenderReference children) -> LotTender -> OPT-310 -> TenderingParty -> Tenderer. For TED it
    is the award block's ADDRESS_CONTRACTOR, AWARDED_TO_GROUP and NO_AWARDED_TO_GROUP.
  - It reads the published lot number two ways and compares them: from the lot section with the key's id, and from the
    award block's own LOT_NO or BT-13713.
  - It takes every amount in every section of every notice and only then classifies it by the rule's table.
  - It checks every summed award against the notice's own figure: the referenced LotTenders' BT-720 for eForms, the
    award block's VAL_TOTAL for TED (`fidelity/tval.py`).
  - It recomputes v1's T', K, G0, G1', G2, G3 and G8b from the head rows.
- **Comparison** (`fidelity/compare.py`). It compares field by field against `walk_facts/{id}.json`:
  - per id: v1 fires, currency, S, n, S_eur, served head, the summed keys, R1, R2a–d, R3, R10, R6, R7, R8, the stated
    totals and the vouches;
  - per summed lot (60 lots): award notice, result key, cents, published lot number, winner ids, and the winner count read
    from the notice.

  That is 19 x 10 + 6 x 60 = 550 comparisons. I seeded 7 mutations into a copy of my facts (a lot number, an R1 key, S, R2d,
  a stated total, a vouch and a winner id), and the harness reported all 7.
- **Shared inputs, disclosed.**
  - Both sides read the API detail's head rows, because the rule's v1 and its summed set are defined on them.
  - Both read the same notice contents.
  - The EUR rate comes from the same ECB `eurofxref-hist` file the port uses, so the S_eur agreement checks the date and
    the lookup, not the rate series itself.
  - The CN-form list (1 2 4 5 7 8 9 10 11 12 16 17 24) comes from the frozen table, which is the spec.
  - I read `rule_v2.py` before writing the derivation, to know what it outputs.

## Mismatches

**None.** No fact differs, so no outcome flips.

## Facts derived by hand, per id

Amounts are in the tender's currency unless marked. "Single-own" means a notice's total equals its own single valued lot
award, which the rule says does not count.

**6094289 (ship).** TED r209, HUF. The head date is 2020-08-05, and the ECB rate is HUF 344.50 per EUR.
- Chain: F02 19610083; F14 19661042 and 19678301 (changes, no totals); F03 19961452; F20 21175651, 21176110 and 21182144.
- Summed, all five from F03 19961452:
  - LOT-1 9,045,000.00 (B. Braun)
  - LOT-2 4,060,000.00 (TEVA)
  - LOT-3 35,578,000.00 (HARTMANN-RICO)
  - LOT-4 63,677,200.00 (HARTMANN-RICO)
  - LOT-5 27,614,700.00 (Pannon Diagnosztika)
- Each value equals its award block's VAL_TOTAL. The published lot numbers are 1–5, and the lot section's LOT_NO equals the
  block's LOT_NO. Each lot has one contractor and NO_AWARDED_TO_GROUP.
- S is 139,974,900.00, which is EUR 406,313.21.
- T': there are six lot-null figures (five estimated_value, one result_value). Each equals a lot award exactly, so they
  are award copies and there is no procedure figure.
- K: the served head HUF 63,677,200.00 is LOT-4's award.
- G0: EUR 406,313.21 is above today's EUR 184,839.48.
- Stated totals: TED-VAL_TOTAL in the Notice section of the F03, 139,974,900.00, equals S. So R3 does not fire, and it
  vouches for S (not needed below EUR 10 m). There is no estimate total.
- R1 and R2 do not fire.

**236597 (ship).** eForms, PLN. The head date is 2026-03-25, and the ECB rate is 4.2718.
- Chain: CN 25523868, change 25497863, ten CANs.
- Summed: 11 lots, each from its own CAN. LOT-0005 is clos-nw in 25663872 and is not summed.
- Each award equals its LotTender's BT-720. Each lot has one LotTender with one tenderer. The published lot number is the
  BT-137 value LOT-000N, which is also the key.
- S is 1,749,349.31, which is EUR 409,511.05.
- T': the lot-null result_value 47,271.00 is LOT-0008's award (a copy).
- K holds.
- G0: EUR 409,511.05 is above EUR 11,065.83.
- Stated totals: each CAN has a BT-161.
  - Seven of them are single-own.
  - 25619589's 111,622.40 equals LOT-0011 + LOT-0004 (78,750.00 + 32,872.40).
  - 25647921's 90,428.24 equals LOT-0009 + LOT-0002 (39,219.42 + 51,208.82).
- R3 does not fire. There is no estimate, and no total near S.

**307725 (ship).** eForms framework, EUR. The head date is 2026-08-14.
- Chain: CN 25495334 and two changes (no amounts), then CAN 28044262.
- Summed:
  - LOT-0001 2,902,095.90 (Selecta Belgium)
  - LOT-0002 5,989,988.80 (OKE KOFFIE)
- Each lot has one LotTender, and its BT-720 equals the award, which also equals the lot's BT-660. The BT-709 figures
  3,490,000.00 and 6,450,000.00 are partner-only and are not summed.
- S is EUR 8,892,084.70.
- T': there is no lot-null figure.
- K: the served head is null.
- G0: n is 2.
- Stated totals: BT-1118 is 8,892,084.70, which equals S. BT-118 is 9,940,000.00, which equals the sum of BT-709.
  - R3 does not fire, because S is at or below both totals.
  - BT-1118 vouches for S. R10 does not apply below EUR 10 m.
- R5.6 and R5.7 do not fire: each award is below 1.9 x its lot's BT-709, and S is below 1.5 x the sum of BT-709.

**321859 (ship).** eForms, CHF. The head date is 2026-02-11, and the ECB rate is 0.9136.
- Chain: CN 24907409, CAN 25550656 (LOT-0002), CAN 25551449 (LOT-0001).
- Summed: LOT-0001 1,920,611.00 and LOT-0002 1,448,972.40. DE_Berlinux wins both, each on one LotTender.
- S is 3,369,583.40, which is EUR 3,688,248.03.
- T': the lot-null 1,448,972.40 is LOT-0002's award (a copy).
- G0: EUR 3,688,248.03 is above EUR 1,586,003.06.
- Each CAN's BT-161 is single-own, so R3 does not fire and nothing vouches.
- R6 does not fire. The winner set is the same and the notices differ, but the titles do not match ("Los 1 - PUT ..." and
  "Los 2 - Patientenportal").

**855538 (ship).** eForms, EUR. The head date is 2024-04-08, and the chain is one CAN, 23861005.
- Summed: 20 of 21 lots (LOT-0015 is clos-nw). Each award equals its BT-720, with one LotTender each.
- S is EUR 7,278,412.00.
- T': there is no lot-null figure. K: the served head is null.
- No notice states any total or estimate, so R3 cannot fire on either branch.

**533536 (ship).** eForms, EUR. The head date is 2026-07-20.
- Chain:
  - CN 24515324
  - CAN 24714086 (LOT-0003 12,550.00)
  - CAN 24754549 (LOT-0002 30,000.00)
  - CAN 44557436 (LOT-0001 21,700.00, published 2025-06-11)
  - can-modif 25363512, 25425874 and 9514 (each LOT-0001 21,700.00)
- LOT-0001's latest award by notice id is 44557436, the highest id but the earliest published. All four LOT-0001 rows are
  21,700.00, so S is the same either way.
- S is EUR 64,250.00.
- T': the result_value 21,700.00 is a copy.
- G0: 64,250.00 is above 21,700.00.
- All six BT-161 totals are single-own, so R3 does not fire and nothing vouches.

**262408 (refuse, R1).** eForms framework, EUR. The head date is 2026-07-09.
- Two CANs, 26993722 and 45872022, award the same three lots. The summed rows come from 45872022, the higher id.
- Every summed lot has two or more winners, each on a separate LotTender, with no shared LotTender, so R1 fires on all three lots:
  - LOT-0001 17,800,000.00: 6 winners on 6 separate LotTenders
  - LOT-0002 12,800,000.00: 5 winners on 5 LotTenders
  - LOT-0003 9,800,000.00: 3 winners on 3 LotTenders
- S is EUR 40,400,000.00.
- BT-118 is EUR 3,400,000.00 in both notices, and S is above 1.01 x that, so R3 fires. No total vouches, so R10 fires.
- By eye the port's other refusals also hold:
  - R5.6: 17,800,000.00 is at least 1.9 x LOT-0001's BT-709 of 2,400,000.00.
  - R5.7: S is above 1.5 x 3,400,000.00.
  - R5.11: the equal bids of 2,400,000.00 equal the BT-709.
  - R5.5: 17,800,000.00 is above the sum of the bids, 14,400,000.00.

**7495977 (refuse, R2d).** TED r209, EUR. The head date is 2023-08-25.
- Three F03s award a key LOT-1:
  - 23355915: 10,000.00, LOT_NO 1
  - 23344948: 19,580.00, whose award block has ITEM 1 only (no LOT_NO), single-own
  - 23379689: 9,690.00, ITEM 1 only in both the lot section and the award block
- The latest by id is 23379689, so its published lot number is None (ITEM is never taken).
- The other summed lots come from 23355915: LOT-4 144,000.00, LOT-9 236,670.00 and LOT-10 208,950.00.
- The summed awards come from two notices and one has no lot number, so R2d fires.
- S is 599,310.00. VAL_TOTAL 23355915 is 599,620.00.
  - The keys that notice awards sum to 599,310.00, which is not above 1.01 x 599,620.00, so R3 does not fire.
  - That total vouches for S (0.05 % off).

**7298633 (refuse, R2c).** TED r209, PLN. The head date is 2022-05-30, and the ECB rate is 4.5855.
- F03 22385634 (2022-05-16) has lot sections ITEM 1–9 carrying LOT_NO 1, 2, 3, 5, …, 10. Section 8 (LOT_NO 9) was
  unsuccessful.
- F03 22375511 (2022-05-30) has one section, ITEM 1 / LOT_NO 4, for PLN 597.40. The API keys it LOT-1.
- LOT-1's latest by id is 22385634 (1,155,000.00, LOT_NO 1), not the later-published 22375511.
- R2c fires: LOT-1's award rounds name the lot numbers {1} and {4}.
- R7 fires: 1,155,000.00 is at least 10 x 597.40.
- S is 3,583,172.35, which equals VAL_TOTAL 22385634. So R3 does not fire, and the total vouches for S.

**975665 (refuse, R3).** eForms, EUR. The head date is 2025-10-10.
- CN 24554865 has a single lot, LOT-0000, with BT-27-Lot 200,000.00.
- CAN 24697829 awards LOT-0000 207,854.00.
- can-modif 25245587 re-keys the same lot and contract as LOT-0001 at 218,491.66.
- Summed: 207,854.00 + 218,491.66 = 426,345.66, so one contract is counted twice.
- R3 fires. 25245587's BT-161 of 207,854.00 is not single-own, because its own award is 218,491.66. Its key LOT-0001 sums
  to 218,491.66, which is above 1.01 x 207,854.00.
- R6 fires: the winner and title are the same, in different notices.
- G8b does not fire (the keys spell 0 and 1).

**R4, R5 and R9 on the 6 ships (checked to confirm the ships).** None fires:
- No ship has an API-keyed lot estimate, so R4, R5.3, R5.8, R5.9, R5.10, R5.14 and R5.15 cannot fire.
- R5.1 and R5.2: every summed result has a winner and no later closed result.
- R5.4: no award is within 0.5 % of a round or kind total.
- R5.5: each eForms award equals its own BT-720.
- R5.6 and R5.7: see 307725.
- R5.11 and R5.12: one bid and one winner per lot.
- R5.13: every S is far above 1,000.00.
- R9: each summed eForms lot has its own LotTender, and 6094289 has no bids.
- R8: the largest award is under 10 x the second everywhere.
- G1', G2, G3 and G8b: none fires on any of the 10.

## Observations (not mismatches; none flips an outcome)

1. **Latest by notice id is not latest by publication date** in 2 of the 10 (shape S11).
   - In 533536 the values are equal, so there is no effect.
   - In 7298633 the summed LOT-1 is the older-id award. That id is refused anyway, by R2c and R7.
   - Both implementations follow v1's LotSums rule as the rule text states it, so this is not a port defect.
2. **6094289's three F20 modification award blocks are not keyed.** They carry LOT_NO 3, 5 and 4, but the API leaves them
   lot-null, so neither implementation sees them in LotSums, R2c or R7.
   - They restate the same values, so there is no effect here.
   - It is a shared projection input. B1 and B3's projection-parity check must key them the same way, or the parity will
     diverge.
3. **262408's stored per-lot award is not the lot's tenders.** Each LotTender's BT-720 is the lot's BT-709 copied per
   supplier: 6 x 2,400,000.00 = 14,400,000.00 for LOT-0001.
   - The stored 17,800,000.00 is the sum of the six settled contracts the result references (CON-0001 to CON-0006 =
     3.4 + 2.8 + 2.8 + 2.4 + 3.4 + 3.0 m).
   - Each of those contracts spans lots; for example, CON-0001 = TEN-0004 + TEN-0008 + TEN-0013.
   - This is shared input, and the id is refused by R1 first.
4. **eForms published lot numbers are the BT-137 identifiers** (LOT-000N), which are the keys themselves. For example,
   236597's LOT-0001 is the buyer's "Część nr 5" by title.
   - Because of the frozen table's no-title rule, R2 cannot see buyer part numbers on eForms.
   - There is no effect here: 11 distinct keys, one award each.
5. **R3's "keys that notice awards" is read loosely.** Both implementations count every key with a lot-result row in the
   notice, including closed or no-award rows. In these 10, no total-bearing notice has a non-award row for a summed key,
   so there is no effect. B2's R3 test should pin which reading is meant.
6. **Pending keying change (issue 567).** `crates/ingest/src/project.rs` is mid-edit in the worktree for issue 567 (TED
   ITEM/LOT_NO keying). These facts were read before that change. In the 10, every summed TED lot's section LOT_NO equals
   its award block's LOT_NO, so no 567-type misattribution is present.

## Files

`unit4b/fidelity/` (sha256 in `fidelity/SHA256SUMS`):
- `fetch.py`, the reads (log: `fetch.log`)
- `raw.tar.gz`, the 10 details and 49 notice contents as read
- `derive.py`, the independent derivation (output: `derived.json`)
- `compare.py`, the field-by-field comparison with `walk_facts` (output: `compare.txt`)
- `tval.py`, awards against the notices' own figures
- `show.py`, `dump.py` and `ndump.py`, readable dumps

To reproduce, unpack `raw.tar.gz` into `$M551/fid/` (the default `M551` is the session scratchpad's `m551`), copy the
walk's `walk_facts/` to `$M551/walk_facts/`, and run `python3 -I derive.py <ids> > $M551/fid/derived.json; python3 -I compare.py`.
