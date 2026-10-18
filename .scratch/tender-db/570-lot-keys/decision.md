# 570: decision. BUILD option B: in `fold`, a legacy notice's lot sections join the Tender lot an earlier notice printed under the same number (2026-10-18)

**Decision: BUILD option B, a fold-time remap.** The first notice of a chain that publishes lot sections keeps
today's keys (`LOT-k`, its own section ids). Each later legacy notice's lot sections are then mapped onto the
Tender's existing keys by printed number before the lot carry. A section joins the one carried lot that printed
the same number. The notice's award blocks follow their section, which issue 567 already resolves. A section with
nothing to match keeps its own key, as today. Four guards (K1–K4) leave a whole notice on its own keys, also as
today, wherever the match is not clean.

**Not A.** Re-keying every legacy lot by its printed number changes lot keys in 13 % of the sample and mints or
sweeps lot ids in 5 %. 19 of the 27 Tenders it re-keys gain nothing from it.

**Not C.** Refusing and recording leaves the measured harm in place: 7 of 204 Tenders, 9 results and 22 lot facts,
plus 8375276's 147 blocks. B's measured error is 0.

Evidence:
- `measurement.json`: the judge's skeptic-final table per Tender, the rates and the exhibits.
- `remap_port.py` and `port_result.json`: today's rule and B replayed on the cached parsed values of every chain (204
  sample Tenders + 7 issue exhibits).
- `assemble_measurement.py`: builds `measurement.json` and asserts the judge's class totals and the 1,596-result total.

## The rule (unit 3 builds exactly this)

Where: `crates/ingest/src/project.rs`, `fold`. The remap runs per notice, before the lot carry
(`lots.iter_mut().find(|l| l.key == published.key)`). It needs the carried chain state, which
`read_legacy_results` never sees. Both fold paths (streaming and `BucketRow::to_notice_state`) go through `fold`.

1. **Which notices.** A legacy notice (`is_legacy_profile`) with at least one lot section, folded after at least one
   earlier notice of the chain published lot sections.
   - `NoticeState` gains `legacy: bool`, set in `NoticeState::read`. `BucketRow` carries it. That is a postcard format
     change, safe under the bucket lifecycle note: the directory is removed at the start and the end of every sharded
     run.
2. **The anchor.** The first lot-publishing notice keys its lots by its own section ids, exactly as today. Its keys
   never change, and B never renames a key.
3. **Targets.** The carried lots of kind `Lot` with a non-blank carried number (`LotState::published_lot_no`: the
   latest section's `TED-LOT_NO`, else `TED-LOT_NUMBER`). The lookup is `LegacyLotNumbers::inherited` + `lookup`,
   568's semantics exactly:
   - an exact trimmed match first;
   - only where no carried lot prints the number exactly, a `normalise_lot_number` match;
   - exactly one lot, or no target.

   Never `TED-ITEM`, a title, or a key's ordinal.
4. **Each lot section.**
   - A `Lot` section with a non-blank printed number p whose lookup names one carried lot goes to that lot's key.
   - Every other section keeps its own key: no number, no match, ambiguous, or not of kind `Lot`.
5. **Guards.** When one fires, the whole notice keeps its own keys (today's fold):
   - **K1, injective.** Two sections of the notice would land on one key. This covers duplicate numbers, and a
     matched section landing on another section's own key.
   - **K2, no silent merge.** A numbered section that matched nothing may keep its own key only where no carried lot
     sits there, or the one there prints no number or the same number (normalised). Otherwise B would merge a
     different number into it. Examples: 8433088's `1` against the carried `1-4`, and 6016286's `2.` against `2`.
   - **K3, legacy chain only.** Once a non-legacy notice has published a lot section in the chain, no later notice is
     remapped. A fold-level flag tracks this, like 568's G2. It keeps eForms `LOT-0001` and TED `LOT-1` apart, so
     551's G8b pair is unchanged.
   - **K4, something to match.** If no carried `Lot` prints a number, nothing is remapped (7025760's unnumbered CN).
6. **What follows the section.**
   - The `LotState` is carried under its target key: facts superseded, kind, and `published_lot_no` set as today.
   - `place_country_lots` is still read by the section's own key and applied to the target lot.
   - The round's `lot_results`, `bids` and `contracts` `lot_key` values are mapped through the same map. 567 resolved
     them to the notice's own section ids. Legacy rounds carry only results, but the map is applied generically.
   - `published_lot_no` is never touched, on the lot or on the result.
7. **G2 reads the remapped carry.** `lot_renumbered` compares at the target key. A remapped section therefore never
   renumbers its lot, while a refused notice still does, exactly as today. `match_inherited_lots` itself is unchanged:
   it runs only for a notice with no lot sections, so it never meets a remap.
8. **Frozen.** A notice's map reads only the notices folded before it. A later notice never re-keys an earlier
   version: version N depends on 1..=N only.
9. **Never:**
   - rename an anchor key;
   - invent a key (every key is a carried key or the section's own id);
   - remap across a non-legacy notice;
   - use `TED-ITEM`, a title, `CONTRACT_NO` or a position as a number.

No epoch bump. The parsed layer is right, and the fold output changes for a cohort a field sweep can enumerate. This
is an ADR-0017 R3 scoped-stale refold, like 567 and 568.

## Why B, not A or C

| | A: key by printed number | **B: remap onto existing keys** | C: refuse and record |
|---|---|---|---|
| fixes the measured harm | yes (same matching) | **5 of 7 harmful Tenders, 7 of 9 results, 14 of 22 facts**; the other 2 cannot be fixed by numbers | no |
| Tenders whose lot keys change | 27 / 204 (13.2 %, 8.9–18.7 %) | **0** (keys are the anchor's) | 0 |
| lot ids minted or removed | 30 minted, 23 swept, in 11 / 204 (5.4 %, 2.7–9.4 %) | **0 in 211 replayed chains** (≤ 1.8 %) | 0 |
| Tenders changed for no gain | 19 of its 27 (consistent or single-notice: 7006083, 7153808, 6997164, 6397695 …) | 0 (a notice that agrees maps to itself) | — |
| key strings to design | yes: `S1`, `4a`, `5.2`, `2021-21162-1`, `19ac005-L1` in 8 / 204 | none | — |
| guards needed | B's K1–K4 as well | K1–K4 | — |

A would honour the schema line "lot identity inside a Tender: the published id" more literally. It is also B plus a
rename of every non-ordinal key. If that rename is ever wanted, it can be its own decision with its own churn
budget, and it would build on B's matching.

Anchoring on the latest notice instead of the first was rejected. Keys would then move every time a notice arrives,
and a lot id's meaning would drift with the chain. The first notice is stable and frozen, as 568's award-version
freeze is.

## The measurement

Sample: issue 567's part-1/2 draws, one multi-lot legacy CAN per random OJ S day, 2016–2023: 204 CANs (182 R2.0.9,
22 R2.0.8). For each, the Tender (`/v1/tenders/{id}`) and every version notice's `/v1/notices/{id}/content` were
read. A measurer, an adversarial skeptic per group of 17 and a judge produced the skeptic-final classes:

| class | Tenders | results | misattributed results | lot facts on another lot |
|---|---|---|---|---|
| single_lot_notice | 47 | — | 0 | 0 |
| consistent | 143 | — | 0 | 0 |
| disagree_harmless | 6 | — | 0 | 0 |
| disagree_harmful | 7 | — | **9** | **22** |
| unnumbered | 1 | — | 0 | 0 |
| **total** | **204** | **1,596** | 9 (0.56 %) | 22 |

The 14 Tenders that are not routine, and what B does with each (port replay):

| Tender | judge | B | after B |
|---|---|---|---|
| 7274985 | harmful 1 / 2 | CAN 22223285 LOT-3 (S4) → LOT-4 | RES-3 on LOT-4 (S4); LOT-3 keeps S3's text |
| 7511890 | harmful 4 / 9 | 22907340 LOT-3 (5) → LOT-5; 22980479 LOT-1/2/3 (3/4/6) → LOT-3/4/6 | all 6 awards on Pakiet k = LOT-k |
| 7697322 | harmful 1 / 0 | 23276289 LOT-1 (02) → LOT-2; 23413021's 13 shifted sections → their CN keys | Mounard TP on lot 02; lots 02/07/09 back; no lot served twice |
| 5787834 | harmful 1 / 2 | 19154619 LOT-1 (18) → LOT-18; 19167246 LOT-18 (19) → LOT-19 | RES-1 on lot 18; LOT-18 keeps lot 18's title |
| 5827131 | harmful 0 / 1 | 19259840 LOT-2 (3) → LOT-3 | RES-2 on lot 3; lot 2 back |
| 6633011 | harmless | 20957186 LOT-3..6 (4/5/6/8) → LOT-4/5/6/8 | 4 awards follow; lot 3 back, lot 8 single |
| 7693726 | harmless | 23256169 LOT-7..12 (8/11/13..16) → their CN keys | 6 awards follow; lots 7/9/10/12 back |
| 6980424 | harmless | 21769762's 7 permuted sections → the CN's keys | RES-2 (5) LOT-8 → LOT-9; identity stable from v1 to v2 |
| 8433088 | harmless | **refused (K2)**: CAN `1` against the CN's single `1-4` | unchanged (a re-lotting, results already right) |
| 6016286 | consistent | **refused (K2)**: CAN `2.`..`7.` against `2`..`7` | unchanged (positions agree) |
| 5317587 | harmful 0 / 7 | identity: the numbers agree | unchanged: a weld (364), 2014 and 2019 procedures |
| 5540787 | harmless | identity | unchanged: a weld (364) |
| 7465548 | harmful 2 / 0 | no numbers (K4 / unnumbered) | unchanged: framework call-offs each restart at an unnumbered LOT-1 |
| 7112657, 5939039 | harmless, unnumbered | the unnumbered section keeps its key | unchanged |

**Port replay** (`remap_port.py`, 204 + 7 chains, 0 notices missing):
- **Today's port matches production.** It equals the served lot on all 1,316 blocks the API had placed. The other
  280 blocks are R2.0.8 awards that were served `lot: null` at cache time; 568's rule places them, and 568's drain was
  running.
- **Lot-publishing notices in the sample: 384.** Of these, 204 are anchors, 164 map to themselves, 11 are remapped,
  2 are refused (K2) and 3 have nothing to match (K4).
- **Sample moves:** B moves 39 sections and 32 of 1,596 blocks, in 8 Tenders. 7 of the blocks are the judge's
  misattributions. The other 25 are right by today's served title, but they sit on a key whose chain identity is
  another lot, and they follow their lot back to its key.
- **Exhibits** (14 remapped notices across 5 of the issue's 7 examples):
  - **8375276**: 147 blocks move. The 120 blocks printing LOT_NO 1 go LOT-2 → LOT-1 and the 27 printing 2 go
    LOT-1 → LOT-2, exactly 567's skeptic-confirmed counts. lots[] stay LOT-1 "Lahve…" and LOT-2 "Kojenecka voda".
  - **8387180**: 30 sections of 19043870 and 19152735 are remapped onto CN 18927620's keys, and 23 awards move.
  - **8480304**: the 7 lots of 21927890 go to LOT-3/5/8/9/10/16/30, putting the "Zad.3" award on Zad.3. 21 sections
    of 22001080 are remapped too. 28 awards move.
  - **7415397**: 22583013's single section `2` → LOT-2.
  - **7025548**: 10 sections of 21742060 → the CN's keys, and 9 awards move.
  - **6120045, 7025760**: unchanged.
- **Keys minted or swept by B: 0 in all 211 chains.** Conflated keys (one Tender key holding sections that print
  different numbers) fall from 10 sample Tenders to 2. The remaining 2 are the refusals 8433088 and 6016286, a range
  and a format variant.
- **Titles: 124 of 124 moved sections** (39 in the sample, 85 in the exhibits) have a title closer to the reference
  title at their target key than at their own key. 0 are closer to their own key, and 0 tie. The reference is the
  anchor's title, or, for 8375276's untitled CN, its first titled notice that agrees with the anchor. The judge
  separately confirmed every sample Tender by figures or descriptions: 7274985's S4 estimate, 7511890's "zwanej
  Pakietem k", 7697322's CPVs, and 5827131's verbatim descriptions.

## Expected reach (95 % intervals)

| unit | point | 95 % |
|---|---|---|
| multi-lot legacy CANs on a Tender whose notices' maps disagree | 13 / 204 = 6.4 % | 3.4–10.6 % |
| … with a result or lot fact on another lot (harmful) | 7 / 204 = 3.4 % | 1.4–6.9 % |
| … that B changes | **8 / 204 = 3.9 %** | 1.7–7.6 % (2016–19: 2 / 100; 2020–23: 6 / 104 = 5.8 %, 2.2–12.1 %) |
| … whose harm B removes | 5 / 204 = 2.5 % | 0.8–5.6 % |
| … whose harm printed numbers cannot fix (weld 364, unnumbered call-offs) | 2 / 204 = 1.0 % | 0.1–3.5 % |
| served results misattributed | 9 / 1,596 = 0.56 % | 0.26–1.07 %; per CAN 0.044 (0.005–0.098, bootstrap) |
| served results B fixes | 7 / 1,596 = 0.44 % | 0.18–0.90 %; per CAN 0.034 (0.005–0.083) |
| lot facts B fixes | 14 of 22 | per CAN 0.069 (0.005–0.18); 5317587's 7 are 364's |
| served results B moves (fixes + re-keyed rights) | 32 / 1,596 = 2.0 % | 1.4–2.8 %; per CAN 0.157 (0.034–0.34), heavy tail (8375276: 147) |

**Absolute numbers (an estimate, not a measurement).** The 204 draws are the same draws issue 567 measured: 15 of
them had a 567 misattribution, and 567's drain corrected 40,708 Tenders. Scaled by 8 / 15, B corrects about
**22 k Tenders** (roughly 9–42 k). The dry run's carrier count and the wet run's `corrected` replace this estimate.

**2012–2015 is ≈ 0.** R2.0.8 award forms have no lot section, and 568 found no chain with two lot-publishing notices
among its 71.

## Error risk

- **Wrong remap** (one printed number naming different lots in two notices of one procedure):
  - 0 of 124 moved sections contradicted by title (95 % ≤ 2.9 %);
  - 0 of 25 remapped notices (≤ 13.7 %);
  - 0 of 13 changed Tenders (≤ 24.7 %).

  The shapes that could produce one are below, and none was seen.
  - **A corrigendum that genuinely renumbers lots** (lot 4 renamed 3 after lot 3 is withdrawn). B merges by the new
    number. Today merges by position, which is the same merge whenever the corrigendum lists its lots in order.
  - **A wrong anchor**: the first lot-publishing notice belongs to another procedure (a weld) or numbers lots
    differently (a PIN). B pulls later notices onto it by number; today pulls them by position.
  - **Welds whose numbers agree** (5317587, 5540787): B is a no-op, as is today. These belong to 364.
- **Refusals cost nothing measured.** 2 of 204 Tenders (1.0 %) keep today's keys on one notice. Both are harmless
  today: a range re-lotting, and `2.` against `2`.
- **Lot ids are not minted or removed**: 0 of 211 chains (≤ 1.8 %). B can sweep a key only when a remapped notice
  has more sections than every earlier notice and its extra sections all match. ADR-0017 announces any such case as
  `removed` plus a minted lot.
- **G2 narrows.** A swap that B resolves no longer disables 568's R2.0.8 match. Measured 0 R2.0.8 awards after a
  remapped notice in the sample.

## Identity and migration cost (ADR-0017)

- **Schema and keys.** No schema change and no epoch bump. Lot key strings and lot ids are unchanged on every
  replayed chain.
- **What changes.** On the corrected Tenders (≈ 4 % of multi-lot legacy CANs), the content under some existing lot
  ids changes: titles, facts, results and lot-scoped winner parties. This happens at the versions caused by remapped
  notices and at the head. Each such id returns to the identity its first notice gave it. Today's fold renamed it at
  a later version (7697322's v3 renamed LOT-2..14; 6633011's v2 renamed LOT-3..6).
  - **Caveat (read-back O4, 2026-10-18).** The return fails where K1-refused notices re-keyed the carry first. A
    refused notice stays on today's positional merge (step 5), so a later section joins the carried lot that prints
    its number, not the anchor's lot at that key. 6876932, yearly re-tenders by one buyer 2020–2023 with two
    interleaved numberings, has K1-refused 21923843, 22231117 and 22219592 at v4, v6 and v7. Its 34 moved sections
    each join the carried lot printing the same number and title, so no number names two lots, but not the anchor
    CN's titles. No award moved, and the head equals the remap-off head. No 570 fix: the chain is a weld, filed on
    364.
- **Announced as corrections.** D3 writes one `tender changed` per corrected Tender, plus `lot changed`
  (`version: null`) for every lot of the head version only when something at the head differs, and for every lot
  whose own rows differ at any seq. That is ADR-0017 D3 rule L, which is what is built (corrected 2026-10-18,
  read-back O5; this sentence said "every lot of the head version"). The fresh 30 shows it: 270 of 274 lots had one
  event, and the 4 without are 6876932's LOT-1/19/22/23, whose rows are the same with the remap on and off. The
  estimate counted every head lot, 10.5 on average across the 8 changed Tenders, so its ≈ 250 k rows (100–500 k) is
  high where the head does not differ. Transition rows are not re-emitted (D4).
- **Subscriptions.** Subscriptions are filters, not lot ids, so nothing migrates. SSE on `/v1/lots` can see
  `added` / `removed` where a moved award changes `winner` or `min_value` membership (D4; issue 565's caveat).
  Mirrors re-read by id.
- **Rollback below the deploy is safe.** The old binary re-derives today's keys on the next refold of a stale
  Tender. Nothing stored depends on B.

## Interaction with 567, 568, 551 and 364

- **567.**
  - B rests on 567: a block resolves to its own section (567), and the section maps to its Tender key (B). Without
    567 the map would carry the wrong section.
  - B uses 567's `normalise_lot_number` unchanged. Its trailing-separator gap (`2.` ≠ `2`) shows up here as K2's
    refusal on 6016286, which is harmless because the positions agree. If 567 ever widens normalisation, B gains it.
  - B unblocks 567's deferred refinement, which is not part of this build. With the remap in place, a no-number
    block in a notice with exactly one lot section can take that section: 7415397's single section `2` now sits on
    Tender LOT-2. The shapes are 6120045 and 7025760. That refinement is measured and decided on its own, and may
    share unit 5's drain if ready.
- **568.**
  - One matching semantics: sections (570) and R2.0.8 blocks (568) both match the carried lots through
    `LegacyLotNumbers::inherited` / `lookup`.
  - G2 is computed on the remapped carry, so it now fires only for renumberings B refuses: a dropped or blank number,
    a range, or a format variant. Two assertions in 568's tests are re-specified on purpose, because the swap is now
    resolved by number:
    - `a_renumbered_inherited_lot_disables_the_cross_notice_match`: the swapped case gives `[LOT-1, LOT-2]`. The
      dropped and blank cases keep `[None, None]`.
    - `a_cross_notice_match_is_frozen_at_the_award_version`: the second chain's later CAN gives `[LOT-1, LOT-3]`.
      The first CAN still keeps its round.
  - Order: 568's drain (2593/2594) is read back before 570's wet run, so each correction set stays attributable.
- **551.**
  - **Rust R2.** Path W's R2 reads `LotState.published_lot_no` at the round notice's version. After B, a summed
    award's key prints the block's own number, so R2(a) (two keys sharing a token, as in 7697322's doubled 15–17) and
    R2(c) (one key with rounds naming two numbers, as in 8375276's LOT-1/LOT-2) stop firing on 570's shapes. No guard
    is relaxed, but path W's inputs change.
  - **The port needs the remap before B3's parity is read after 570's drain.** `unit4b/gen_fixtures.py` carries lot
    numbers by each notice's own section id, and `rule_v2.py`'s `pub_no(nid, key, result_key)` reads the round
    notice's section at the served key. Both assume that the served key is the notice's own section id. For a
    remapped notice it is not: 7511890's 22980479 RES-1 is served on LOT-3, but that notice's own LOT-3 prints 6.
    Both must apply `remap_port.remap`.
  - **Order.** B3 need not wait for 570's build. If B3 runs first, it re-runs after 570's drain. **B5's flip waits
    for 570's drain and that B3 re-run**, so that path W goes live on the final legacy keys once. Path V, which is
    live, can ride along: lot sums move with the awards. The band / path-V baseline is snapshotted before the wet run.
- **364.** The two welded Tenders (5317587, 5540787; 1.0 %, 0.1–3.5 %) are recorded there as evidence: the numbers
  agree but the lots differ. Printed-number keying cannot separate them.

## Units

1. **Measured (done).** The workflow measurer, skeptics and judge: 204 Tenders, `measurement.json`. The port replay:
   `remap_port.py`, `port_result.json`.
2. **Decided (this file).**
3. **Build.**
   - **Code.** The rule above in `fold`, with a helper `legacy_lot_remap` beside `match_inherited_lots` that reuses
     `LegacyLotNumbers::inherited`. Add `NoticeState.legacy` and `BucketRow.legacy` (lifecycle note) and the K3 flag.
     Read the `crates/ingest/src/project.rs` diff before staging (CLAUDE.md "Committing").
   - **Named tests** in `project::tests`, from the notices' parsed values (`/v1/notices/{id}/content`, cached in this
     folder), the way 567 and 568 built theirs:
     - `a_later_notice_listing_a_subset_by_position_joins_the_lots_it_prints_7274985`: RES-3 on LOT-4; LOT-3 keeps
       S3's title.
     - `two_award_notices_with_disjoint_packages_join_their_own_lots_7511890`: awards on LOT-1/2/5 and LOT-3/4/6;
       head lots are Pakiet k at LOT-k.
     - `swapped_award_notices_follow_the_printed_number_8375276`: CN 18679338 + 19786640 + 21337435. Every block lands
       on the key whose head number is its own.
     - `a_notice_omitting_lots_does_not_shift_the_rest_7697322`: `02` on LOT-2, 23413021's 13 shifted sections on
       their CN keys, and no number at two keys.
     - `a_one_section_award_joins_the_lot_it_numbers_5787834`: 19154619's 18 lands on LOT-18; 19167246's 19 on LOT-19.
     - `a_consistent_permuted_chain_is_unchanged_7006083`: versions identical to a fold with the remap off.
     - `the_first_lot_publishing_notice_keys_the_lots_and_never_moves`: anchor keys survive a disagreeing notice, and
       no key is invented.
     - `two_sections_naming_one_lot_keep_the_notice_on_its_own_keys` (K1).
     - `a_number_no_lot_prints_at_an_occupied_key_keeps_the_notice_on_its_own_keys_8433088` (K2), with 6016286's `2.`.
     - `a_new_number_on_a_free_key_keeps_its_own_key`: CN 1..3; CAN LOT-4 printing 4 stays at LOT-4.
     - `an_unnumbered_section_keeps_its_own_key_7112657` and `an_unnumbered_chain_is_not_remapped_7465548_7025760`
       (K4).
     - `a_non_legacy_notice_is_never_remapped` (K3): an eForms CAN's LOT-0001 after a TED CN's LOT-1 stays LOT-0001.
     - `place_country_follows_the_section_to_its_target`.
     - `g2_reads_the_remapped_carry`: a remapped R2.0.9 notice before an R2.0.8 CAN leaves the match on; a refused one
       turns it off. This goes with the two 568 re-specifications above.
     - `stated_totals_name_the_remapped_keys`.
     - `the_remap_is_identical_on_the_bucketed_path`.
     - 567's and 568's other tests stay green unchanged, and no golden moves unless its chain disagrees. A golden that
       moves is read and named.
   - **Port parity.** The Rust tests over the cached chains (7274985, 7511890, 7697322, 5787834, 8433088, 6016286,
     7006083, reduced 8375276) reproduce `port_result.json`'s keys block for block.
   - **Docs.**
     - A `docs/operations.md` block, "Legacy lots joined by printed number across a chain (issue 570)", beside 567's
       and 568's.
     - One `/docs` coverage sentence, pinned by a new `the_docs_say_legacy_lots_join_by_printed_number_across_notices`
       (`assert_says`): a legacy notice's lot section joins the Tender lot an earlier notice printed under the same
       number, and its awards follow. A section with no number, a number no earlier lot prints at a position another
       lot holds, a range or list, or two sections naming one lot keep their positions.
     - Re-word 568's "where an earlier notice renumbered the lots" to the refused renumberings, and re-pin its test.
     - A CHANGELOG entry.
4. **Gate, review, deploy.** Run `ops/check.sh` and read `GATE-EXIT=`, not a pipe's exit. Then an adversarial review
   (steps 1–9, K1–K4, the frozen anchor, the 568 re-specifications), then deploy in a queue gap.
5. **Drain.** Preconditions: 568's read-back has passed, there is a queue gap clear of the 07:35 UTC daily chain, and
   no refold is running.
   - **Baseline first:** the ≥ €1 bn band, the path-V heads, and the 211 port Tenders re-fetched after 568
     (`drain/baseline-pre570.json`).
   - **Commands:**
     - dry: `{"kind":"refold-fields","profiles":["TED-LOT_NO","TED-LOT_NUMBER"],"tables":["notice_ids"],"expect":1}`
       gives N. Expect ≈ 2.1 M: ≈ 1.4 M `TED-LOT_NO` (567's estimate) plus 719,732 `TED-LOT_NUMBER` (568's dry 2591).
     - wet: the same with `"expect":N`. It queues its own `project`; do not queue a second.
   - **Why both fields.** Every remapped section prints one, and so does every anchor it can match. `TED-LOT_NUMBER`
     adds only R2.0.8-only chains with two lot-publishing notices (0 of 71 in 568).
   - **Cost.** The extra Tenders cost an ADR-0017 verify each, minutes. The `project` falls back to FULL (issue 305),
     ≈ 4.3 h like 2590.
6. **Read back.**
   - (a) **D7.** `verified + corrected == stamped`, and no `projection_epoch = 0` residue in 100k windows.
   - (b) **The issue's Verify.** 8375276 serves LOT-1 "Lahve…" and LOT-2 "Kojenecka voda", with every block whose
     notice printed LOT_NO 1 on LOT-1 and every block printing 2 on LOT-2 (147 moved: 120 + 27).
   - (c) **The 8 changed sample Tenders and 5 changed exhibits.** Every block is on `port_result.json`'s B lot (32 +
     207), and lot key sets are unchanged. 8433088, 6016286, 7465548, 7112657, 5939039, 5317587, 5540787, 6120045 and
     7025760 equal the baseline.
   - (d) **The other 196 sample Tenders** equal the baseline. Read at 1 GET per 1.5 s.
   - (e) **A fresh uniform 30.** Draw corrected Tenders from the `project`'s `version: null` `tender changed` rows in
     its cursor window. The draw is fixed before any id is read: `sha256("570-fresh30-" + id)`. Each moved block must
     land on the key whose head number equals its own, each moved section's title must agree with its target (or be
     untitled), and no lot may be added or removed. **Bar: 30 of 30.** One wrong reopens this decision.
   - (f) **Band and path-V diff** against the baseline. Every head that moves must be a corrected Tender, and each is
     read as 570's.
   - (g) **`lot removed` rows** in the window attributable to 570: expected 0.
7. **Follow-on, not this build.** 567's single-section no-number placement (6120045, 7025760), measured and decided
   on its own.
   - **Exhibit (read-back O6, 2026-10-18):** 8371908 CAN 21510251 RES-1, a EUR 2.97 bn award. The block prints "Lot
     No. 3 - Lot 1C …", which 567's same-notice normalisation does not read, so it serves `lot: null` before the
     drain and after. The notice's only lot section, LOT_NO 3, is now remapped from its own LOT-1 to LOT-3. Neither
     568-u8 nor 570 moved the block.

## What the issue records

Status line:

> Status: ready-for-agent — DECIDED 2026-10-18: BUILD option B (fold-time remap of a legacy notice's lot sections onto
> the Tender's existing keys by printed number; anchor = first lot-publishing notice; K1 injective, K2 no silent
> merge, K3 legacy chain only, K4 something to match). Measured (204 multi-lot legacy CANs 2016–23, judge-verified):
> maps disagree 6.4 % (3.4–10.6 %), harmful 3.4 % (1.4–6.9 %; 9 / 1,596 results, 22 lot facts); B changes 3.9 %
> (1.7–7.6 %), fixes 5 of 7 harmful Tenders, 7 of 9 results, 14 of 22 facts; 0 / 124 moved sections contradicted by
> title, 0 lot ids minted or swept in 211 chains; ≈ 22 k Tenders (9–42 k). NEXT: unit 3 build; drain after 568's
> read-back, before 551 B5.

The `done` line holds: an award whose notice printed LOT_NO n serves on the Tender lot the chain identifies as lot n.
The recorded exceptions, which keep today's positional key by design:
- a section that prints no number (7465548's framework call-offs, 7112657's NO_LOT_DIVISION republication);
- a number no earlier lot prints, at a position another lot holds (8433088's `1` against `1-4`; a format variant such
  as 6016286's `2.`);
- two sections of one notice naming one lot;
- a chain in which a non-legacy notice published lots;
- a first lot-publishing notice whose numbers belong to another procedure. Welds whose numbers agree (5317587,
  5540787) are issue 364's.
