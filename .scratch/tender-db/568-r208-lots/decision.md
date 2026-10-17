# 568: decision. BUILD the cross-notice lot match for legacy award blocks (2026-10-17)

**Decision: BUILD.** A legacy award block that resolved to no lot in its own notice may take the lot that an
earlier notice of the same Tender printed under the same number. The match is made in `fold`, against the lots
that the award notice's version inherits. It applies only when exactly one inherited lot prints the number, and
it carries two guards. On ambiguity, no match or a guard firing, the award stays on the Tender, as today.

Evidence: `measurement.json` holds the per-block, skeptic-final table and the judge's extra strata.
`cross_notice_port.py` and `port_result.json` hold the rule replayed on the cached parsed values; it agrees on
**679 of 679** blocks. `extra_port.jsonl` holds the extra strata.

## The rule (unit 3 builds exactly this)

Where: `crates/ingest/src/project.rs`, `fold`. The rule runs when a notice's `round` is pushed, before
`lots_awarded` reads the rounds. It has to live in the fold: `read_legacy_results` sees one notice, and the
Tender's lots, with their printed numbers (`LotState::published_lot_no`), exist only in the carried chain state.
Both fold paths, streaming and `BucketRow::to_notice_state`, go through `fold`.

1. **Which results.** A result with `lot_key == None` and `published_lot_no == Some(p)`, where `p` is the block's
   own number (first `TED-LOT_NO`, else first `TED-LOT_NUMBER`, as `award_block_lot_number` already keeps it).
   Only a legacy award block has that pair: eForms and FTS set `lot_key` and `published_lot_no` together from
   BT-13713. A block that printed no number is never matched.
2. **Which notices.** Only an award notice that published no lot section of its own (`state.lots` is empty). A
   notice with lot sections keeps issue 567's same-notice resolution, including its `None`. Example: the F20
   whose section prints `3.` while its block prints `3` (6094289; `a_ted_award_block_keeps_its_published_lot_number`
   stays green).
3. **Which lots count.** Every lot the version inherits from the notices folded before it: the contract notice,
   any corrigendum or later notice that published lot sections, a PIN's lots. Only lots of kind `Lot` count, each
   with its carried number. That number is the latest section's `TED-LOT_NO`, else `TED-LOT_NUMBER`. It is never
   `TED-ITEM` and never a title. A lot with no number, or a blank one, takes no part.
4. **Number match.** An exact match on the trimmed number first. Only where no inherited lot prints `p` exactly,
   compare `normalise_lot_number` forms (trimmed, lower-cased, a leading `Lot`/`Los`/`Lote`/`Lotto` and a
   following `nr`/`no`/`n°` dropped, leading zeros dropped). This is 567's `LegacyLotNumbers::lookup` semantics,
   over the inherited lots. Exactly one lot means `lot_key = that key`. Two or more lots means no lot
   (ambiguous). None means no lot.
5. **G1: one number on every block.** Take an award notice with two or more numbered blocks that all normalise to
   one number. If its inherited lots print two or more distinct numbers, **none** of its blocks is matched. A
   single-lot Tender is exempt, because there several awards of the one lot are the expected shape.
6. **G2: renumbered lots.** Suppose a notice folded before the award republished an inherited lot key under a
   different normalised number, or dropped its number. Then no block of the award notice is matched. The fold
   tracks this with one flag, set in the lot carry loop.
7. **Frozen at the award's version.** The key is set once, on the version the award notice causes, and later
   versions carry the round as it was resolved. A later renumbering never re-keys an earlier round.
8. **Kept as published.** `published_lot_no` stays the block's own number. Path W's R2 (551) reads the lot
   section's number for the key first, and by construction it normalises to the block's.
9. **Never used as a lot key:**
   - the key's ordinal (`LOT-k` read as k). It is wrong for 5 of 8 blocks in 4748635, where the CN skips
     lot 5.
   - the award notice's `TED-ITEM`.
   - a title, description or "Lot k" designator. 5071935's titles say `Lot 06` where the lot number is 3,
     and 5086044 RES-6 carries a copied title.
   - a corrigendum's free text.
   - `CONTRACT_NUMBER`, which is sequential and not a lot number in 4964788, 5227380 and 4996449.
   - the block's position.

No epoch bump. The parsed layer is right, so this is a refold under ADR-0017 R3, the same as 567.

## Why: the measurement

Sample: issue 567's part-0 draw. One random OJ S day per month, 2012–2015, and every multi-lot R2.0.8 CAN found
(at least 2 award blocks and at least 2 distinct award numbers). That gave 100 CANs out of **470 R2.0.8 CANs
probed**. 71 of them sit on a Tender with 2 or more lots. All 679 award blocks of those 71 were classified by a
measurer and re-read by a skeptic. The skeptics re-fetched the 71 Tenders and every CN and corrigendum.

| class | blocks | share |
|---|---|---|
| unique_right | 636 | 93.7 % |
| unique_unverifiable | 11 | 1.6 % (5018063, Winterdienst: every title and the winner are the same; EUR/t is consistent across all 11, but lots with equal tonnage could be swapped) |
| unique_wrong | **0** | 0 |
| ambiguous | **0** | 0 |
| no_match | 26 | 3.8 %. 4746422: CN prints A/B/C, CAN 1/2/3 (3). 5218711: the CAN renumbers sub-lots 3.B–3.D as 5–7 (16). 4840245 and 4996449: lots added only in corrigendum prose (2+2). 5214582: numbers no notice printed (3). |
| no_number | 6 | 4945774 (3), 5214582 (3) |

- **Port replay** (`cross_notice_port.py`, the rule above run on the cached `/v1/notices/{id}/content` of every
  notice in each chain):
  - It agrees with the skeptic-final class on 679 of 679 blocks: 643 exact matches, 4 normalised matches
    (4581969's `01`, `02`, `04`, `05`), 26 no_match and 6 no_number.
  - Neither guard fires on any of the 71: no CAN prints one number on every block, and no chain has two
    lot-publishing notices.
  - No CAN publishes its own lot section, and no block carries two `LOT_NUMBER` values.
  - 28 corrigenda appear in the chains. None carries a lot section.
- **Strength of confirmation.** About 275 of the 636 rest on figures or bare labels rather than distinct text:
  - figures (226 blocks): the estimate equals the CN lot value, wadium, area × rate or capacity table.
    Tenders 5284511 (139), 4945774 (45), 4840245 (15), 4961767 (11), 5527847 (8), 4964788 (6), 4910832 (2).
  - bare labels (49 blocks): `Circuit k`, `Pakiet <Roman>`, `Szerződés` + part table.

  All of them agree. None contradicts its number.
- **Judge's extra strata** (outside the 100, same 470 probes, read 2026-10-17, *not skeptic-verified*). The
  sampler's "≥ 2 distinct numbers" filter excluded every other in-scope CAN. There are 6 such CANs:
  - 4 single-award CANs with a `LOT_NUMBER` on 2-lot Tenders (8288447, 5154859, 5312343, 5302290): 3 right,
    1 unverifiable. The 4 further blocks of non-probed CANs in the same chains are all right.
  - 1 multi-lot CAN on a 1-lot Tender (4634188): 1 right, 1 no_match.
  - 1 multi-block CAN printing **`LOT_NUMBER 1` on all three blocks** (5476747, CAN 18543084; titles "Lot 1 /
    Lot 2 / Lot 3"). Without G1 the rule puts RES-2 and RES-3 on LOT-1, which makes **2 unique_wrong**. G1
    leaves all three on the Tender and gives up RES-1, which would have been right. This shape is why G1 exists.
    G1 itself fires on none of the 71.
- **G1's cost on the measured set: 0.** Repeated numbers inside one CAN are legitimate there. 55 blocks share a
  number with another block of their CAN: 4945774's partial positions, 5218711's framework awards and 5071935's
  groups. 39 of them match right. Guarding on any duplicate would cost those 39. G1 fires only when every block
  prints the same number.

## Expected reach (95 % intervals)

| unit | point | 95 % | basis |
|---|---|---|---|
| blocks moved per multi-lot R2.0.8 CAN | **6.48** | 3.9–10.0 | bootstrap over the 100 (heavy tail: 139, 60, 45, 26 …) |
| multi-lot CANs that gain at least one lot key | **71 / 100** | 61–80 % | Clopper–Pearson |
| blocks moved per 2+-lot Tender in the sample | 9.1 (647 / 71) | — | 95.3 % of their blocks |
| award blocks of the 100 multi-lot CANs that move | 648 / 906 | 71.5 % | — |
| R2.0.8 CANs whose Tender gains a lot key | **16.0 %** (75 / 470) | 12.8–19.6 % | 71 + 4634188 + 4 single-award; 5476747 refused by G1 |
| blocks moved per R2.0.8 CAN | **1.39** | 0.80–2.16 | bootstrap over the 470 |

Absolute numbers are an *assumption, not a measurement*: 0.8–1.0 M R2.0.8 award notices, i.e. 30–37 % of the
profile's 2,699,213 notices. On that assumption, about **130–160 k Tenders corrected** (100–195 k with the
interval) and about **1.1–1.4 M award blocks** moving from `lot: null` to a lot (0.64–2.2 M). The dry run's
carrier count and the wet run's `stamped N` replace this estimate.

Outside the frame, and expected to move at a similar rate (unmeasured):
- the R2.0.7 files filed under the profile (2010-03 → 2011-09);
- the R2.0.8 tail of 2016–2019 (22 of 567's 100 multi-lot legacy CANs there were R2.0.8);
- multi-CAN chains, which the sampler's head-version filter excluded. 3 of the 4 extra single-award Tenders were
  such chains, and all their blocks were right.

So the reach is more likely under-stated than over-stated. R2.0.9 is not in scope: all 182 sampled multi-lot
R2.0.9 CANs publish their own lot sections, so rule step 2 keeps 567's resolution there.

## Error-rate bound

- **Measured stratum, skeptic-verified:** 0 unique_wrong in 647 matched blocks. The 95 % upper bound is 0.57 % of
  blocks. Counted by CAN, 0 of 70 gives an upper bound of 5.1 %.
- **With the extra strata and G1:** 0 wrong in 652 matched blocks (upper bound 0.56 %), and 0 of 75 CANs (upper
  bound 4.8 %).
- **Without G1:** 2 wrong in 655 blocks (0.31 %, 95 % 0.04–1.1 %), and 1 of 76 CANs (1.3 %, 0.03–7.1 %). G1
  removes the one wrong shape found.
- **Unverifiable:** 12 blocks (5018063 × 11 and 5302290). No figure or text contradicts them.

## Units

1. **Measured (done).** The wf 568 measurer and skeptics, the judge's port replay (679 of 679), and the 6 extra
   CANs (`measurement.json`).
2. **Decided (this file).**
3. **Build.**
   - **Code:** the rule and G1/G2 in `fold`, with a helper beside `LegacyLotNumbers` that reuses
     `normalise_lot_number`. The `crates/ingest/src/project.rs` diff must be read before staging (CLAUDE.md
     "Committing"). Another agent is editing `crates/store/src/canonical.rs`, and this unit does not touch it.
   - **Named tests** in `project::tests`, built from the notices' parsed values (`/v1/notices/{id}/content`, the
     same way 567 did):
     - `an_r208_award_takes_the_lot_its_contract_notice_numbered_4748733`: CN 16929923 + CAN 17120114. RES-k
       lands on LOT-k, `published_lot_no` is unchanged, and the version's `stated_totals[].lots_awarded` names
       the keys.
     - `a_cross_notice_match_reads_the_printed_number_not_the_key_ordinal_4748635`: CN 16929463 (prints 1–4,
       6–10) + CAN 17187125. RES-4..8 land on LOT-5..9. A block printing 5 lands on no lot.
     - `a_normalised_number_matches_only_where_no_lot_prints_it_exactly_4581969`: `01`, `02`, `3`, `04`, `05`
       against 1–5. A lot printing `01` beside one printing `1` takes the exact `01`.
     - `a_number_no_inherited_lot_prints_stays_on_the_tender`. Three cases:
       - 4840245's pakiet 17/18, which exists only in corrigendum prose;
       - 5214582's 13–15;
       - 4746422's A/B/C against 1/2/3, where the CAN's `TED-ITEM` 1/2/3 is ignored.
     - `a_number_two_inherited_lots_print_names_neither`.
     - `a_block_with_no_printed_number_stays_on_the_tender_even_with_an_item`: 4945774 RES-41, and a CAN
       `TED-ITEM` alone.
     - `an_award_notice_with_its_own_lot_sections_keeps_its_same_notice_resolution`: the F20 `3.` / `3` shape.
       `a_ted_award_block_keeps_its_published_lot_number` must also stay green unchanged.
     - `one_number_on_every_block_matches_none_5476747` (G1): CN 18395736 + CAN 18543084 gives three `None`.
       Counter-cases that still match:
       - 4945774-style repeats beside other numbers (42 × 4, 47 × 4);
       - a 1-lot Tender whose blocks all print 1.
     - `a_renumbered_inherited_lot_disables_the_cross_notice_match` (G2): a CN prints 1/2, and a later notice
       republishes LOT-1 as 2 and LOT-2 as 1.
     - `a_cross_notice_match_is_frozen_at_the_award_version`: a renumbering after the CAN leaves its round's
       keys alone, and later versions carry them.
     - `several_awards_on_one_lot_all_take_it_5218711`: six `Lot 1` framework awards land on LOT-1, and 5/6/7
       stay `None`.
     - `an_eforms_result_is_never_matched_across_notices`: with no BT-13713 it stays `None`, and with BT-13713
       it is unchanged.
     - `the_cross_notice_match_is_identical_on_the_bucketed_path`: the `BucketRow::snapshot(..).to_notice_state()`
       chain gives the same keys.
   - **Port parity:** add G1/G2 to `cross_notice_port.py`. It must keep 679 of 679 on the 71 and give 5476747
     three `None`.
   - **Docs:** a `docs/operations.md` runbook block, "R2.0.8 awards by their contract notice's lot number (issue
     568)", beside 567's. Add one `/docs` caveat sentence pinned by `assert_says`: an r2.0.8 award takes the lot
     its contract notice printed under the same number; letters, numbers only the award prints, and lots added in
     corrigendum prose stay on the Tender.
4. **Gate, review, deploy.** Run `ops/check.sh` and read `GATE-EXIT=`. Then an adversarial review (the rule's
   steps 1–9, G1/G2, and the frozen key), then deploy in a queue gap.
5. **Drain** with `refold-fields` over **`TED-LOT_NUMBER`**, `"tables":["notice_ids"]`.
   - **Why that field.** Every R2.0.8 award block that can move prints it. So does the R2.0.8 CN's lot annex
     that it matches. Those CN-only Tenders ride along and compare `verified`. `TED-LOT_NO` stays out: 567's
     refold covered it, and R2.0.9 CANs keep their same-notice resolution.
   - **Commands:**
     - dry: `{"kind":"refold-fields","profiles":["TED-LOT_NUMBER"],"tables":["notice_ids"],"expect":1}` gives N
     - wet: the same with `"expect":N`. Its handler queues its own `project`; do not queue a second.
   - **Cost.** Carriers are likely about 0.5–0.9 M, so the `project` probably falls back to FULL (issue 305's
     500 k closure cap). That is about 4.3 h, like 2531 and 2590.
   - **Order.** Run it after 567's drain is read back (2589/2590), clear of the 07:35 UTC daily chain, and
     **before 551's B3 parity**. If B3 runs first, it re-runs after this drain, for the same reason it waits for
     567: legacy award lot keys feed path W's sums.
   - **Baseline first.** Before the wet run, snapshot the ≥ €1 bn value band and the path V `lot-sum-vouched`
     heads.
6. **Read back.**
   - (a) D7 completeness: the `project`'s `verified + corrected` is at least the wet run's `stamped N`, and no
     `projection_epoch = 0` residue remains.
   - (b) The issue's Verify: 4748635 serves `[LOT-1, LOT-2, LOT-3, LOT-5, LOT-6, LOT-7, LOT-8, LOT-9]`.
   - (c) The 77 measured Tenders (71 + 6), read through `/v1/tenders/{id}` at 1 GET per 1.5 s:
     - every block on `port_result.json`'s or `extra_port.jsonl`'s lot: 647 + 8;
     - 5476747's three, the 26 no_match, the 6 no_number and 4634188 RES-2 stay `null`;
     - nothing else moved.
   - (d) A fresh uniform 30 corrected Tenders from the `project`'s `tender changed` rows, with at least 10 from
     outside 2012–2015. Each moved block is adjudicated by title or figure. **Bar: 30 of 30 with no wrong
     block.** One wrong reopens this decision.
   - (e) Band and path V diff against the baseline. Every head that moves must be a Tender whose R2.0.8 awards
     gained keys, and each one is read as 568's.
   - (f) Status: R2.0.8 Tenders whose served `status` changed should be about none (see risk 6).

## Risks

1. **Copy-pasted numbers that G1 does not catch.** A CAN printing `1, 1, 3` where it meant `1, 2, 3` is not
   caught. It is bounded by 0 wrong of 647 in the stratum with at least two distinct numbers.
2. **A welded or mis-chained Tender (issue 364).** If an unrelated CN's lots sit in the chain, a number match
   is unique and wrong. Measured 0 of 77: each CAN's `REF_NOTICE` names the Tender's CN, checked by the skeptics
   for the 71 and by the judge for the 6. Upper bound about 4 %. There is no guard here; weld correctness belongs to 364.
3. **Several awards on one lot.** Partial positions, framework multi-awards and groups all land on the lot,
   which is correct. But `LotSums` keeps one award per lot (the latest), so award-side sums of such Tenders
   understate. 567's R2.0.9 keys already feed the same shape, and 551 B2/B3 must account for it.
4. **The head value can ride along.** 537's award-side scale refusal and 551's live path V now see R2.0.8 award
   lot sums for the first time. A ≥ €1 bn head can flip between kept and refused, or between vouched and not.
   Expect a handful at most. Read each one (unit 6e).
5. **Change-feed burst.** Each corrected Tender writes a `tender changed` plus a `lot changed` per touched lot;
   5284511 alone writes about 140. In total that is on the order of 10⁶ rows, and lot subscribers (SSE,
   webhooks) see the burst.
6. **Status (533).** A Tender whose every result is a no-award now decides its lots, so `closed_by_notice` can
   flip in the projection. 2010–2019 deadlines are past, so the served status should not move (unit 6f).
7. **First value only.** `published_lot_no` is the block's first number. A block that prints two disagreeing
   numbers would be matched on its first. Measured 0 of 692.
8. **G1 and G2 refuse some right matches.** A CAN that awards one lot of a multi-lot Tender to several framework
   suppliers stays tender-scoped under G1. A corrigendum that correctly renumbers lots disables the match under
   G2. Both outcomes are today's behaviour, never a wrong lot. Neither guard fires on the 71.
9. **Outside the frame.** 2010–2011, 2016–2019 and multi-CAN chains are unmeasured (see reach). Unit 6(d)'s
   stratified 30 covers them.

## What the issue records

The issue's status line should read:

> Status: ready-for-agent — DECIDED 2026-10-17: BUILD the cross-notice match (rule steps 1–9 + G1/G2, in
> `fold`). Measured 0 unique_wrong / 647 matched (95 % ≤ 0.57 % of blocks, ≤ 5.1 % of CANs); the one wrong shape
> outside the sample (5476747, one LOT_NUMBER on every block) is G1's. Reach ≈ 16 % of R2.0.8 CANs (12.8–19.6 %),
> 1.39 blocks per CAN (0.80–2.16). NEXT: unit 3 build.

The `done` line holds: an award block printing n on a Tender whose contract notice printed lot n serves that
lot. The recorded exceptions, which stay `lot: null` by design:
- no lot prints n (letters, a CAN-only number, a lot added in corrigendum prose);
- two lots print n;
- the CAN prints one number on every block (G1);
- the inherited lots were renumbered (G2);
- the award notice has its own lot sections (567's resolution);
- the block prints no number.

Fallbacks considered and refused, with the evidence above: the lot key's ordinal, the CAN's `TED-ITEM`, title
designators, corrigendum prose and `CONTRACT_NUMBER`.
