# Issue 551 — B6 decision record (2026-10-19, workflow wf_834aaac1-932)

Panel: three judges (risk-first, value-first, process-integrity), all chose **g-cover-first**; a synthesis; an adversarial critic. The adopted decision is the synthesis **as amended by every critic amendment** (issue 551 "B6 — decision" lists the binding points).

## Judges

### judge:risk-first

**Option:** g-cover-first

Judged risk-first: a wrong derived value served under our own `lot-sum-awards` label is the cost I weigh most.

1. Re-enabling as-is is out. It would knowingly serve again two heads that panels have already read as wrong. Both are in served_lsa_rows.json (the 3,502).
   - 6465510 (wrong/other; G-cover refuses it).
   - 5816379 (M2 fallback, wrong/farther: S EUR 472,520 vouched by the 2017 award VAL_TOTAL on 19334482, superseded by the 2018 re-award that serves 322,400). G-cover does NOT catch it, because the vouching notice does award summed keys.
   - Uniform reads of v2 ships so far: M2 fallback 1 wrong of 30, confirm30 1 wrong + 1 neutral of 30. Pooled that is 2 of 60, about 3.3 % (exact 95 % 0.4–11.5 %), or about 115 wrong heads in the 3,502 (between 14 and 400).

2. Keeping it off is also a served-wrong-value choice. On the 61 heads just read, the head without path W was read as the worse value on 59, typically 1–80 % of the true sum. Turning the path off re-serves about 3,400 understated values, about 97 % of which read right as sums.
   - The wrongs found are small. 6465510's S (162k) is still closer to the true ~860k than today's head (70k). 5816379 is 1.47x over.
   - The multiples-scale danger sits in the €100 m band, and all 29 band heads have read right (read18's 20 plus band9).

3. The stop rule tripped on a stale projection, not on bad reads.
   - Inside 2575's population: 15,300 × 41/277 ≈ 2,265 predicted against 2,348 seen.
   - Outside it, the 1,154 fires were never sampled before. They have now read 29/30 (outside30) and 31/31 (outside22 + band9).
   - So the excess is explained; the window needs re-basing, not the rule withdrawing.

4. G-cover is cheap and safe in shape, but not yet evidence.
   - It only adds refusals, so v2+G ⊆ v2 on the same facts. That keeps the id-list drain complete and every wrong already refused stays refused.
   - Its evidence is in-sample: it was designed after reading 6465510, on the same 61. The confirm30's 0 wrong after G-cover therefore does not count toward a go.
   - Missing before a re-enable:
     - (a) the size of the G-cover refusals on the 3,502;
     - (b) Rust and port agreeing on it, especially whether `stated_totals.lots_awarded` is empty for a notice whose results are stored lot = null, as on 20772908;
     - (c) a FRESH uniform read of what still ships after G-cover.
   - None of these is in hand.

5. Drain route: an id list is enough; no corpus walk.
   - 2627's plan walked all 14.9 M notices, no daily chain ran during it, and the fold is deterministic on stored facts. So on unchanged facts the v2 fire set is exactly those 3,502, and v2+G-cover's is a subset of it.
   - The only new candidates are Tenders whose stored facts changed since 2627 ended (cursor 958787061):
     - 541's drain 2632 (126 corrected; winners feed R1);
     - the daily chains of 10-19 onward, including rederive-eur-recent's D5 rows;
     - 571's drain (67) if it lands.
   - Every one of those writes a `tender` change row, so `/v1/changes` from 958787061 to the cursor at the flip deploy lists them. A Tender that compared identical has unchanged stored facts and cannot newly fire.
   - The corpus walk is needed only if a change relaxes a guard or re-keys lots, which G-cover does not. Missing a fire only loses a fix; it never serves a wrong value.

**Units:**

- U1 (S, read-only). Specify G-cover and port it.
- G-cover filters R10's vouch candidates: a total of kind Award or Range (BT-161, BT-118, BT-1118, TED-VAL_TOTAL on CAN forms, TED-LOW/HIGH) counts only if its notice awards at least one summed key (the notice's lots_awarded ∩ the summed keys ≠ ∅). With no remaining vouch, R10 refuses.
- Write b5/rule_v2_b5.py from b3/rule_v2_b3.py; record its sha256 in issue 551 before any use.
- Re-run the B3 asserts on the 279 / oos-6 / 277 / 4,745. Every change must be ship → refuse. List the movers and say which read-right heads are lost (that cost goes on record, not on a bar).
- Must hold: 6465510 refuses; 7749755, 6779279, 6850401 and 7698260 (identity vouches) still ship.
- U2 (S, read-only, can start today). Size G-cover on the 3,502.
- Proxy pass: bounded /v1/sql reads of the served lot_results of the 3,502 ids, by tender_id in batches of ≤100, plan-probed, never retrying a 408. Combine them with served_lsa_rows.json's vouched_by. Candidate set C = ids whose first award/range vouch comes from a notice that awards no summed key.
- Exact pass on C: fetch tenders and notices (≤1 GET per 1.5 s) and decide with the port, using ALL vouches. Result: the exact G-cover refusal set Rg.
- In the same pass, size the 5816379 shape: an award-type vouch from a notice superseded by a later award notice in the chain.
- Record |C|, |Rg| and Rg's shape mix. If |Rg| > 15 % of 3,502, stop and re-decide: G-cover is then not the narrow guard the 61 suggested.
- U3 (M, code, W stays off).
- Rust: add G-cover to canonical.rs R10 as a named pure function. Refusal name: R10 (or R10c, if the port uses it too).
- Tests through head_value_with_awards_path:
  - a_vouch_from_a_notice_that_awards_no_summed_lot_does_not_count, on a 6465510 real-row fixture from gen_fixtures.py;
  - the four identity-vouch rights still ship;
  - 585917, 5995030 and 8426318 still serve their pins.
- Temporary B3-style harness: Rust against rule_v2_b5.py on the 562 cached ids, the 61 and C. Bar: 0 disagreements on ship/refuse, S, currency, n, first refusing check and vouched_by. Add projection parity of stated_totals.lots_awarded against the served lot_results notice→key rows on the same ids, so the lot = null case is pinned. Remove the harness afterwards.
- Run ops/check.sh and read GATE-EXIT=. Deploying with LOT_SUM_AWARDS_PATH=false is harmless and may go in any queue gap.
- U4 (S, read). Fresh confirmation, pre-registered.
- Record a new salt in issue 551 before the draw, e.g. sha256('551-gcover-fresh-'+id) ascending.
- Population: (3,502 − Rg), minus every id read before (the 279, the 277, the M2 reads, read18, the 61).
- Take the first 30. Each gets a reader and an adversarial skeptic, a third reader on disagreement, wf_dc85fa7e-c00's rubric.
- Sequential bar, fixed in advance:
  - 0 wrong of any class → go;
  - exactly 1 wrong → continue the same salted order for 30 more, which must have 0 wrong (≤1 of 60, upper 95 % ≈ 8.9 %);
  - 2 or more wrong in total → keep off and re-decide (unit-X shape).
- Band: the post-G-cover band set must be a subset of the 29 already read. Any band ship not among them is read under 0 wrong of any class.
- 5816379: if a guard candidate refuses it and refuses 0 of the roughly 170 read-right ships, adopt it the U1–U3 way. Otherwise keep it as a counted residual named in the decision.
- U5 (S, ops). Flip and drain by id list.
- Prediction: P = 3,502 − Rg, as exact ids.
- Id list: L = the 3,502 ∪ every tender id with a change row in /v1/changes from 958787061 to the cursor at the flip deploy (covers 2632, the daily chains, 571). Deduplicate and split into runs of ≤5,000.
- Before the wet run, snapshot the served value and value_derivation of every id in L.
- Order: LOT_SUM_AWARDS_PATH=true, gate, deploy, then refold-lot-sum by tender_ids dry → wet with expect → project, all in one queue gap with no daily chain in between.
- Afterwards: bounded PK reads of L, the pins, and the band seek (bars below).
- U6 (S, ops, 14 days after the flip). After each 07:35Z chain:
- Band seek for new lot-sum-awards rows of €100 m or more: each one is read at 0 wrong of any class, otherwise switch off.
- Count of new lot-sum-awards heads per day against the expected ≈1.4 % of the admitted Tenders it touched: more than 3x is a stop-and-read.
- Before any lot-key drain (the 567/570 single-section placement, the 571 follow-on): re-run the port diff on the served lot-sum-awards set first. Pin 6908248 against becoming a double count (R3 must refuse it once RES-2..10 are keyed), and pin the partial sums 6679375, 7153588, 7398935 and 7722197.

**Bars / stop rules:** GO BARS (all must hold before the flip):
- U3 parity: 0 disagreements and a green gate.
- U2: |Rg| ≤ 15 % of 3,502, with Rg exact (not the proxy).
- U4: the fresh 30 has 0 wrong; or 1 wrong followed by a further 30 with 0 wrong. Every band ship is among the 29 read heads, or is read at 0 wrong of any class.
- The 0.5x–2x window is retired. The prediction is now an exact id set.

DRAIN STOP RULES (each one means switch off and drain back by id list):
1. Any id serving lot-sum-awards that is outside P ∪ L_changed.
2. An id in P not serving it, unless its facts changed after 2627 (it is in L_changed) and the port explains the move.
3. Count window: corrected ≥ 0.99 × |P| and ≤ |P| + 0.03 × |L_changed| + 5. Ids in L_changed that newly fire are expected at about 1.4 % of them.
4. A pin off:
   - 585917 must serve EUR 250,000.00, 5995030 PLN 1,549,511.60 and 8426318 GBP 3,000,000,000.00, each lot-sum-awards with its vouched_by;
   - the Lyon-Turin trio 6450467/69/73 keeps EUR 1,660,661,682.94 with no derivation;
   - the 16 path-V pins stay byte-identical;
   - the 9 must-stay heads serve no derivation;
   - 6465510 does not serve lot-sum-awards (and 5816379 does not either, if its guard was adopted).
5. The plan-probed band seek finds a row outside the read 29. Stop and read it; switch off only if it reads wrong (any class).

AFTER THE DRAIN: every lot-sum-awards fire outside the 3,502 (the new ones from L_changed) is read: all of them if there are 30 or fewer, otherwise a salted 30 recorded before the draw. Bar: 0 wrong of any class, otherwise switch off.

**Drain route:** Id list, not a corpus walk.
- List: the 3,502 (b5/investigate/served_lsa_rows.json minus 179732) ∪ every tender id with a change row in /v1/changes from 958787061 (end of 2627) to the cursor read right after the flip deploy. That range covers 541's 2632, the 10-19 chain onward with its rederive D5 rows, and 571's drain if it lands first.
- Split into refold-lot-sum tender_ids runs of ≤5,000: dry, then wet with expect, then project. Minutes per run.
- Switching off again uses the same route: the served lot-sum-awards ids, which stay under 5,000.
- A corpus walk (about 5 h, including the 14.9 M-notice plan) is needed only if the code ever relaxes a guard or re-keys lots. G-cover only adds refusals, so it is not needed. If wanted, an optional reconciliation walk can run later in a quiet window, expecting about 0 corrections.

Timing:
- U1, U2 and U4 are read-only, so they can run any time, today included (bounded SQL reads and ≤1 GET per 1.5 s). The U3 deploy with W off may go into any queue gap.
- The FLIP and its drain must wait for 560's drain-free D5 mornings. Do them only after the 2026-10-21 07:35Z chain, including its rederive-eur-recent, has finished, roughly from 10-21 ~10:00Z, once /health shows the chain done. The one exception: if 560 records U-d done on the 10-20 morning, the flip may go after the 10-20 chain.
- Do flip, deploy and drain in one gap, with no daily chain between the flip deploy and the end of the drain, and finish well before the next 07:35Z. A flip deploy that has to sit through a chain means adding that chain's changed ids to the list.
- Land 571's 67-Tender drain before the flip, or include its ids in the list.
- No lot-key drain while W is on until the port diff has been re-run on the served set.

**Risks:** - G-cover is fitted in-sample: built after reading 6465510 on the same 61. That is why U4 has to be a fresh draw.
- The lots_awarded semantics may differ between Rust and port, or may not be empty for lot = null results. If so, G-cover is a no-op in production while the port says it refuses. U3's projection parity on 20772908-type notices is the check.
- G-cover can refuse rights: a procedure-wide CAN whose awards are stored lot = null but whose total really covers S. That is a lost fix, not a wrong value, but its size is unknown until U2.
- The 5816379 shape (a vouch superseded by a later award notice) stays unguarded unless a clean guard turns up. Expect it to recur among the remaining ships at about 1 in 60. Its wrongs so far are about 1.5x, not orders of magnitude.
- The estimated residual wrong rate after G-cover is about 1–3 %, i.e. about 35–100 wrong heads among about 3,400. Against that, about 3,300 understatements get fixed. A fresh 30 at 0 wrong still allows up to about 10 % (95 % upper bound), hence the sequential 60 and the 14-day monitoring.
- Partial sums (6679375 in the band, 7153588, 7398935, 7722197, 6779279, 6850401, 7698260) are right only while their lot = null rows stay unplaced. A future placement drain could push their S off its vouch, or turn 6908248 into a double count. Hence the rule to re-run the port diff before any lot-key drain.
- The id list depends on every changed Tender having a change row in the feed. A fold that alters stored facts without writing a change row would break completeness. That only loses fixes, so the risk is low.
- Flipping during 560's U-d mornings would also change what the daily fold writes, and would make 560's D5 coverage read meaningless. Hence the hard timing gate.
- The fires from the new daily notices after the flip are never sampled before they are served. U6's band seek and rate check are the only guard on them.

### judge:value-first

**Option:** g-cover-first

Value lens: about 3,500 heads stay understated by 1.5x to 569x while path W is off. On everything read so far they are about 97-98 % right: M2-fallback 29/30, read18 18/18, confirm30 28 right / 1 neutral / 1 wrong, band9 9/9, outside22 22/22. That is roughly 2 wrong in about 110 reads, and even 6465510's S (18.8 % of the truth) is nearer the truth than the head without W (8.2 %). So keeping path W off for good has no case on value.

The stop rule tripped on count, not on correctness, and the count is now explained:
- 2,348 fires inside 2575's population against B3's post-drain rate. 15,300 x 41/277 ≈ 2,265, so that half matches within 4 %.
- 1,154 fires in a population M1/M2 never sampled. It has now been read 29/30 right, and the band rows in it 9/9.
- The pre-registered confirm bar passed, with no margin.

So on the evidence, re-enabling as-is is allowed. But the one wrong is not noise. It is a systematic, explained hole: R10's "single-lot notice's own award" exclusion (canonical.rs usable_totals) only looks at award rows with a lot key and a value. 19 of 20 of 6465510's award rows are stored lot=null/awarded=null, so a total from a notice that awards no summed key can vouch by coincidence. G-cover closes that hole and can only add refusals.

The deciding fact is calendar time. Path W must not be ON during the 10-20 or 10-21 07:35Z chains. rederive_eur_window stamps the Tenders whose EUR moved and re-queues them into a FOLD, and the fold elects through head_value, which reads LOT_SUM_AWARDS_PATH. With W on, the D5 refold of 10-19's and 10-20's mostly non-EUR TED packages would move heads for path-W reasons, not rate reasons. That would confound 560 U-d's D5 coverage read. The fires are rate-heavy: HUF 677, BGN 385, PLN 247. The current 10-19/10-20 daily folds also ran with W off, so a later re-elect would add exactly the moves U-d cannot tell apart.

So the earliest safe flip is 10-21, after that morning's chain and its rederive refold finish (or 10-20 post-chain if 560 records that U-d was done on the 10-20 morning alone). That leaves about 45 hours of slack that the flip cannot use anyway. Building and sizing G-cover in that slack costs zero days of the 3,500 fixes.

It is time-boxed. If G-cover is not gated, parity-clean and sized by the 10-21 post-chain gap, flip as-is: that version passed its pre-registered bar. G-cover then follows. Because it only adds refusals, only currently served lot-sum-awards heads can move, so its later drain is an id list of the served lot-sum-awards rows (minutes) and is complete.

Drain route: an id list, not the corpus walk. 2627 decided every cohort Tender under v2. Facts for a Tender only change through a fold. stated_totals are projection-only and move only with new notices. So the only Tenders that can fire beyond the 3,502 are those folded since 2627 ended (cursor 958787061). That covers the 10-19/20/21 daily chains, the rederive refolds and any other drain such as 541's 2631/2632. Every such fold that moves served output leaves a row in /v1/changes, so an id list from the feed finds them in minutes, with no 14.9M-notice plan and no 5-hour collision with the daily chain.

**Units:**

- U1 G-cover build (S, code; 10-19/20; path W stays off). In canonical.rs R10 (r10_vouching_total, through usable_totals): a stated total of kind Award (BT-161, BT-118, BT-1118, TED-VAL_TOTAL on a CAN-type form) or Range (TED-LOW/HIGH) vouches only if its notice's StatedTotal.lots_awarded shares at least one key with the summed keys. Estimate kinds (BT-27-Procedure, TED-VAL_ESTIMATED_TOTAL, a CN-type form's VAL_TOTAL) are unchanged. A refusal still names R10, so the vocabulary does not change. Mirror the rule in a copy of b3/rule_v2_b3.py and record the new port of record's sha256 in issue 551 before any sizing read.
- U1 tests: a real-row fixture of 6465510 built from b5/investigate/served/6465510.json by gen_fixtures must refuse at R10. The partial-sum and identity-vouch heads must still ship with the same S: 7749755, 6779279, 6850401, 7698260, 6679375, 7153588, 7398935, 7722197. 585917, 5995030 and 8426318 keep their pins through head_value_with_awards_path; the Lyon-Turin trio and path V stay byte-identical. LOT_SUM_AWARDS_PATH stays false. Verify with CLAUDE.md's focused command, then ops/check.sh, and read the GATE-EXIT= value.
- U2 parity (S, measure). Re-add B3's temporary ignored harness (b3/harness/) and run Rust v2+G against the port v2+G on B3's 5,307 rows plus B5's 61 served snapshots plus the 563 cohort. Bar: 0 disagreements on ship/refuse, S, currency, n, first refusing check and vouched_by. Then remove the harness and confirm crates/ is byte-identical to the U1 commit. List every already-read head that G-cover refuses (cohort 73, read18, the band 29, M2's 30, B5's 61): each one is a lost right fix, recorded and counted.
- U2 sizing on the 3,502 (S, read-only). For the roughly 2,900 heads whose vouched_by is Award or Range kind (served_lsa_rows.json), GET the served Tender at ≤1 GET per 1.5 s (about 75 min, outside the 07:35Z chain) and compute k, the number G-cover refuses. Use the port's rule on served lot_results; U2 parity makes it Rust-equivalent. Fix the salt '551-gcover-' in the issue before reading. Read the k refused heads, all of them if k ≤ 30, otherwise a salted 30, with reader + adversarial skeptic. Adopt G-cover if k ≤ 175 (≤5 % of fires) or if ≥1/3 of the refusals read wrong or neutral. Otherwise drop G-cover and flip as-is.
- U3 flip (S, ops). Window: 10-21, after the 07:35Z chain AND its rederive-triggered refold have finished, with no other drain queued (serialize 571's refold-fields with it, never at the same time). Set LOT_SUM_AWARDS_PATH = true with G-cover, gate, and deploy one rev. Then run refold-lot-sum with tender_ids = served_lsa_rows' 3,502 ids (dry, then wet with expect, then project; minutes).
- U3 complement drain. Collect the distinct tender ids from /v1/changes after cursor 958787061 (2627's end) up to the flip deploy's cursor, minus the 3,502 and minus 2630's own correction rows. Run refold-lot-sum with them in batches of ≤5,000: dry first (it reports how many the cohort admits; those it does not admit cannot move), then wet with expect. Before each wet run, snapshot the served value of every listed id.
- U4 confirmation and follow-up (S, read). Read every new complement fire if there are ≤30, otherwise a salted 30, at ≤1 wrong. Read every new band row in full, at 0 wrong of any class. After a week of daily folds with W on, read a salted 30 of the lot-sum-awards heads that the daily folds derived (new notices are a population never measured), same bar. File follow-ups that are not blockers: placement of single-Lot-section LotResults plus vouching by disjoint notice totals (the partial-sum family); pin 6908248 before any lot-key drain; the 5816379 superseded-vouch shape.
- Fallback if U1/U2 miss the 10-21 gap: run U3 with path W v2 as-is, using the same windows minus k. G-cover then lands later as a refusal-only change, drained by an id list of the then-served lot-sum-awards rows.

**Bars / stop rules:** Retire the 835-3,338 window: it was based on M1's pre-drain rate, and it has served its purpose by forcing the never-measured population to be read. Pre-register these in issue 551 before the flip deploy.

(1) Id-list drain of the 3,502. The drain replays decisions already measured, on unchanged facts, so the expected count is exact: E = 3,502 − k (k from U2, 0 if as-is). Stop (switch off, refold back by the same id list) if |corrected − E| > max(35, 1 % of E). In parallel, the 2627-to-flip /v1/changes ids that are also among the 3,502 are the only ones allowed to deviate; list them in advance.

(2) Pins. 585917 EUR 250,000.00, 5995030 PLN 1,549,511.60 and 8426318 GBP 3,000,000,000.00 must be lot-sum-awards with their vouched_by. The Lyon-Turin trio stays EUR 1,660,661,682.94 unmarked. The 16 path-V pins and the 9 must-stay heads stay byte-identical. 6465510 must not serve lot-sum-awards. Any pin off: switch off.

(3) Band seek (≥ EUR 100 m, range seek, LIMIT 200). The band must be exactly the 29 read band heads (B3's 20 plus B5's 9) minus any that U2 shows G-cover refusing. One of them missing: switch off. Any extra row: stop and read, 0 wrong of any class allowed. Switch off only if it reads wrong.

(4) Complement drain. New fires above max(50, 2 × admitted × 1.39 %), where 1.39 % = 3,502/251,526 is the corpus fire rate: stop and read before going on. The read bar is ≤1 wrong in all or a salted 30, and 0 wrong in the band.

(5) U2 gate: parity must be 0 disagreements. Any disagreement blocks G-cover, and you fall back to as-is for the flip.

(6) Timing stops. Never let path W be ON, and never run any refold-lot-sum, during the 10-20 or 10-21 07:35Z chain or its rederive refold. If the U3 work could overrun into the 10-22 07:35Z chain, do not start it that day.

**Drain route:** Use the id list, never the corpus walk.

(a) refold-lot-sum with tender_ids = the 3,502 served lot-sum-awards ids from b5/investigate/served_lsa_rows.json: one run under the 5,000 cap, dry → wet (expect) → project, which took 2 minutes in 2630.

(b) The complement: the distinct ids in /v1/changes from cursor 958787061 (2627's end) up to the flip deploy's /health cursor, minus (a) and minus 2630's corrections. These are the 10-19/20/21 daily folds, the rederive refolds, and 541's 2631/2632 or any other drain since. Batches of ≤5,000, dry first.

This route is complete. 2627 evaluated all 251,526 cohort Tenders under v2, and a Tender's v2 decision can only change when a fold changes its facts. Stated totals are projection-only and move only with new notices, and every such fold that moves served output writes a change row. The corpus route would plan the 14.9M notices again (about 5 h, 2627 took 4 h 48 m) for no extra coverage.

Timing: the flip deploy and both drains go in the 10-21 queue gap after the 07:35Z chain and its rederive refold have finished. If 560 U-d records that it finished on the 10-20 morning, the same can run in the 10-20 post-chain gap. Everything finishes in under an hour, long before 10-22 07:35Z. U1/U2 (build, gate, parity, about 75 min of sizing GETs) run from 10-19 to 10-21 with path W still off, outside the chain windows.

**Risks:** - G-cover vs Rust semantics. lots_awarded counts every LotResult with a lot key, closed and no-award results included. So a vouching notice whose only summed-key result is closed passes G-cover, while the served-lot_results proxy might refuse it. U2 parity on 5,307 + 61 + 563 rows is what makes k trustworthy.
- G-cover may refuse more read rights than expected, most likely partial-sum heads vouched by a notice whose own results are stored lot=null. That is why the adopt-or-drop rule in U2 exists.
- Estimate-kind vouches (604 VAL_ESTIMATED_TOTAL, 79+53 LOW/HIGH ranges if they are reclassified) have the same coincidence hole in principle. About 20 of them have read right; G-cover leaves them alone.
- The 5816379 shape (an earlier award notice's total, superseded by a later re-award) is not addressed. The partial-sum family stays fragile against any future lot-key drain: 6679375 would leave the band, and 6908248 would become a double count only R3 catches. No lot-key drain should land without re-running the port diff.
- The complement id list depends on change rows. A fold that changed facts v2 reads without changing served output would be missed. Only stated totals qualify, and those come only with new notices, which always write a version row. Residual risk is small; the next corpus-scale refold for any other reason would sweep it.
- With W on, every later daily fold derives v2 heads on new notices, a population never measured. U4's one-week read covers it.
- The confirm bar passed with no margin (1/30, 95 % CI 0.1-17.2 %). The pooled about 2/110 is the better estimate, but it pools populations read under slightly different port versions.
- Running the flip before 10-21 for speed would confound 560 U-d, because the rederive re-queues into a W-on fold, and could cost 560 several more days.

### judge:process-integrity

**Option:** g-cover-first

Process-integrity view. The pre-registered parts did their job and must stay on the record as they are.

1. The stop rule tripped, and the switch-off and drain back were the pre-registered response. Both were done and verified (2630: 3,502 corrected; PK reads show 0 lot-sum-awards; the band shows 0).

2. Both pre-registered quality bars passed:
- confirm30: 28 right / 1 neutral / 1 wrong (PASS, no margin).
- band9: 9/9 right.

3. The count window was stale before B5 began, and that is a process error of our own. B3 had already recorded 41 of 277 walked shipping after the 567/568/570 drains, against M1's 30. The window should have been re-based on that rate at B3, before any B5 data. 15,300 × 41/277 ≈ 2,265 predicts the 2,348 fires inside the 2575 population. That prediction comes from a number written down before the drain, so explaining the inside excess this way is not post-hoc. The 1,154 fires outside 2575 were a population no read had ever sampled. It has now been measured at 29/30 right, but only as an investigation read: its salt was fixed before reading, its bar was not. An honest record says: count rule tripped on a stale projection; quality bars passed; one third of the population was measured late.

4. What would count as moving the goalposts:
- re-scoring confirm30 as "0 wrong once G-cover refuses 6465510";
- widening the old window after the fact to take in 3,504;
- treating the outside30 as a pre-registered pass;
- adjusting G-cover after seeing its size.
None of these may happen.

5. Does adding G-cover after seeing the wrong count as fitting the rule to the sample? Yes. By construction it was chosen on the 61 and refuses exactly the one wrong. It gets no credit on those 61. Three facts make adding it legitimate anyway:
- It only adds refusals, so v2+G ⊆ v2 ⊆ v1. It cannot create a wrong, and every earlier uniform draw stays a valid draw of the smaller rule's fires.
- It has a reason fixed before the data. R10's pre-registered text already excludes "a single-lot notice's own award". 6465510 got through only because the implementation compares T with the stored award row, which is null when results are stored with lot = null and awarded = null. So G-cover repairs the implementation to match a pre-registered clause; it does not add a new idea.
- Its only real risk is lost fixes. A procedure-total award notice whose results are stored lot = null would also "award no summed key", so it would be refused even where the sum is right. That cost has to be measured before adopting it, with the adoption rule written down first.

6. Why not re-enable as-is: nothing in the bars forbids it, but it would knowingly ship a gap between the code and a pre-registered clause of unmeasured frequency, on a result with no margin (exact 95 % upper bound 17 %).

7. Why not keep off: the evidence on v2's vouched fires is about 2 wrong in 82 non-band uniform reads (M2 fallback 1/30, confirm 1/30, outside22 0/22) and 0 in 29 band heads. That is inside the bar the decision set.

8. Timing costs nothing extra. Issue 560 needs 10-20 and 10-21 mornings with no drain or flip. The measurement units below fit inside that hold, and the re-enable lands after it.

9. Drain route. The id list is enough; no corpus walk is needed:
- 2627 was an exhaustive Rust pass over the whole cohort (251,526, D7 complete), and no other fold ran before the switch-off (2630 changed no data).
- On the data as of 2627, v2's fires are exactly the 3,502, and v2+G's are a subset of them.
- The only Tenders that can newly fire are those whose data changed since: the daily chains of 10-19, 10-20 and 10-21 that ran with path W off. They are enumerable from the change feed after the 2630 range, cross-checked against notices ingested since 2026-10-18 20:30Z.
- The ~5 h corpus walk (with its 14.9M-notice plan) becomes an optional later audit, expected to correct 0.

**Units:**

- R0 (record first, before any new fetch or read). Add a 'B5 re-registration' block to issue 551. It records: (a) the count rule tripped because the window was not re-based on B3's 41/277; this is a process error, and B3-type rate changes re-register the window from now on; (b) confirm30 stands as 1 wrong / 1 neutral / 28 right, PASS with no margin, never re-scored; (c) outside30 is an investigation measure only; (d) G-cover is post-hoc and gets no credit on the 61; (e) every id already read (279 labelled, 277 walked, read18, the 61) is excluded from new bar samples. Fix the salts now: sha256('551-b6-gcost-'+id) for the cost read, sha256('551-b6-fresh30-'+id) for the pre-enable read, sha256('551-b6-delta-'+id) for the post-drain delta read. Fix G-cover's wording now, with no variants: 'an award-type vouching total (TED-VAL_TOTAL, BT-161, BT-118, BT-1118, TED-LOW/HIGH) counts for R10 only if its notice's stated_totals.lots_awarded contains at least one summed key; estimate kinds unchanged'. Write the decision rules of G3, G4 and B6 below into the block.
- G1 (build, W stays off). Add G-cover in canonical.rs as a named R10 sub-check. Its refusal name is chosen now, e.g. R10c. The same edit goes into a new port copy rule_v2_g.py, made from b3/rule_v2_b3.py, with its sha256 recorded in 551 before any run. Real-row fixtures: 6465510 refuses at R10c; 7749755, 6779279, 6850401 and 7698260 still ship; re-derive the pins 585917, 5995030 and 8426318 under G. 8426318 is vouched by TED-HIGH on 20083570, so check whether that notice awards a summed key, and record the pin outcome before any drain. Check that lots_awarded is EMPTY, not absent, for lot=null results, as on 6465510's notice 20772908. Run the focused test with the gate's flags, then ops/check.sh, and read GATE-EXIT. A deploy with W off is allowed (it writes nothing) but not required.
- G2 (parity and sizing on the actual population, during the 560 hold). Fetch public-API facts for all 3,502 ids, about 19k GETs at no more than 1 per 1.5 s, roughly 8 h across 10-19 and 10-20. Run a temporary B3-style harness, Rust v2+G against rule_v2_g.py, over the 3,502 plus the 563-id cohort plus the 61. Bar: 0 disagreements on ship/refuse, S, currency, n, first refusing check and vouched_by. Any disagreement stops the re-enable until it is explained, as with 179732. Outputs: P, the predicted v2+G ship set (sha256 of the id list recorded in 551), and g = |3,502 \ P|. Also check that P's S, currency and n equal served_lsa_rows.json for every id whose data has not changed. Remove the harness and confirm crates/ is byte-identical.
- G3 (lost-fix cost read, rule fixed in R0). If g ≤ 350 (10 % of 3,502), adopt G-cover without a read. If g > 350, read a salted 30 of G's refusals ('551-b6-gcost-', excluding read ids) under M2's protocol. Adopt G-cover only if at most 20 of the 30 read right (lost fixes). Otherwise G-cover in this form is rejected, and the re-enable goes ahead as v2 as-is, which already passed its pre-registered bars. No rewording of G-cover after seeing g.
- G4 (pre-enable fresh read of the rule that will ship). Draw a salted 30 from P minus every read id ('551-b6-fresh30-'), with reader and adversarial skeptic on every head and a third reader on disagreement. Bar unchanged: at most 1 wrong of 30. Band: every P head at or above EUR 100 m outside the 29 already read is read in full, 0 wrong of any class. Failure means path W is withdrawn for good (unit X), as the original decision tree already says after the fallback. There is no third variant.
- B6 (flip and id-list drain, after 560's window). Earliest: 2026-10-21, after the 07:35Z chain and its rederive have finished AND after 560's U-d bounded read has run (or its inputs are snapshotted); otherwise 2026-10-22 after its chain. It must finish well before the next 07:35Z chain. Set LOT_SUM_AWARDS_PATH = true with G, run ops/check.sh, read GATE-EXIT, deploy. Then refold-lot-sum by tender_ids over L = the 3,502 ∪ D. D is every Tender with a change row after the 2630 drain-back range (the daily chains of 10-19..10-21), cross-checked against notices ingested since 2026-10-18 20:30Z. Run in chunks of at most 5,000: dry, then wet with expect, then project. Before the wet run, snapshot the served rows of D, as was done for the 3,502.
- B6v (verify and read). PK-read every id in L. Check the stop rules (next field). Read every D fire if there are at most 30, otherwise a salted 30 ('551-b6-delta-'); bar at most 1 wrong. Read any band row outside the predicted band set; bar 0 wrong of any class. Re-read the pins. Record everything in 551 and in the operations.md runbook.
- Standing guards after re-enable. No lot-key drain (567/570 follow-ons, single-section placement) until the port diff has re-run on P and any band change or newly double-counted head is read; pin 6908248 and the partial sums 6679375, 6779279, 6850401 and 7698260. Optional: one corpus refold-lot-sum audit in a later gap away from D5-sensitive mornings. Expected corrections: 0. Any correction is an enumeration miss and gets read.

**Bars / stop rules:** Bars that stay unchanged:
- At most 1 wrong in a fresh salted 30.
- 0 wrong of any class among band heads; neutral allowed.
- Reader and adversarial skeptic on every head; a third reader on disagreement.

Registered before data (in R0):
- G2 parity: 0 Rust/port disagreements on ship/refuse, S, currency, n, first check and vouched_by over the 3,502 plus the 563 cohort plus the 61.
- G3 adoption rule: g ≤ 350 means adopt. Above 350, a salted 30 of G's refusals with more than 20 rights means reject G-cover and re-enable as v2 as-is.
- G4: the fresh 30 of P (excluding every id already read) has at most 1 wrong, and band heads in P outside the 29 already read have 0 wrong of any class. Failure means unit X.

The new count window is a predicted set, not an extrapolated rate, because the data is known:
1. The corrected ids among the 3,502 must be a subset of P ∪ D. Any corrected id outside both means switch off.
2. That count must be within |P| ± 1 %, the tolerance only for ids also in D. Outside it means switch off.
3. For every P id not in D, the served S, currency and n must equal G2's prediction (and served_lsa_rows.json). Any mismatch means switch off.
4. D's fires are projected as |D admitted by the dry run| × 3,502/251,526. More than max(2× that projection, projection + 10) means stop and read before anything goes on. The D fires read: at most 1 wrong.
5. Pins: 585917, 5995030, 8426318 and the Lyon-Turin trio must equal the values re-derived in G1. The 16 path-V pins and the 9 must-stay heads are unchanged. Any one off means switch off.
6. Band: the bounded seek must equal the predicted band set, i.e. (the 20 + the 9) ∩ P plus any D head already read. A missing head means switch off. An extra row means stop and read; switch off only if it reads wrong.

Switching off uses the route already proven: LOT_SUM_AWARDS_PATH = false, gate, deploy, then refold-lot-sum by tender_ids over the corrected ids that serve lot-sum-awards.

**Drain route:** Use the id list; no corpus walk is needed.
- 2627 decided every cohort Tender exhaustively (251,526, D7 complete). No other fold changed data before the switch-off; 2630 was a drain back with no data change. So on 2627-era data, v2's fires are exactly the 3,502, and v2+G's are a subset of them.
- The only Tenders that can newly fire are those whose data changed since then. Those are the ones touched by the daily chains of 10-19, 10-20 and 10-21 (path W off), plus any other refold, which none should be.
- Enumerate them as D: every Tender with a change row after the 2630 range, cross-checked against the tender ids of notices ingested since 2026-10-18 20:30Z.
- Drain L = 3,502 ∪ D with refold-lot-sum tender_ids: chunks of at most 5,000, dry, then wet with expect, then project. This takes minutes per chunk, not the ~5 h corpus plan over 14.9M notices.
- G-refused ids in the list simply compare identical.

Timing:
- Nothing flips and nothing wet drains before 560's window ends.
- Until then, run only off-box and read-only work: G1 build and gate, G2's ~19k GETs at 1 per 1.5 s, the G3 and G4 reads.
- Do the deploy and drain on 2026-10-21 after the 07:35Z chain and its rederive have finished and after 560's U-d read has taken its data. Otherwise do them on 2026-10-22 after the chain, finishing well before the next 07:35Z chain.
- From the deploy on, daily folds derive v2+G heads themselves.

Optional later: one corpus refold-lot-sum audit in a quiet gap that is not a D5-sensitive morning, expected corrected = 0. Any correction is a miss of the D enumeration and gets read.

**Risks:** 1. G-cover's lost-fix cost is unmeasured beyond the 61. With results stored lot = null, a legitimate procedure-total award notice also "awards no summed key". G3's pre-registered read decides it, so the guard must not be reworded after sizing.

2. lots_awarded semantics may differ between Rust and the port on lot=null results (empty versus absent). G1's fixture and G2's parity must cover 6465510 and 7749755 explicitly.

3. A G-cover pin can flip: 8426318 is vouched by TED-HIGH. Its outcome must be re-derived and recorded before the drain, or the pin stop rule fires spuriously.

4. Completeness of D. If a touched Tender can appear without a change row, the id list misses it. The notices-ingested cross-check and the optional corpus audit cover this.

5. The partial-sum family (lot = null awards outside S) is right only while those rows stay lot-null. A future lot-key drain can turn heads into double counts (6908248) or push them off their vouch (6679375 would leave the band). Every lot-key drain must re-run the port diff on P first.

6. The bars have wide intervals: 1 of 30 has an exact 95 % upper bound of 17 %. Pooled evidence (about 2 wrong in 82, 0 in 29 band heads) is better, but includes post-hoc samples that must not be presented as pre-registered.

7. If the deploy or drain slips into 560's D5 mornings, 560's U-d coverage read is spoiled: W-driven seq-less corrections would look like coverage or residue. Hold the flip until 560's read has its data.

8. Process risk: the temptation to report confirm30 as '0 wrong under G' or to re-base the old window. The R0 record must prevent both in writing.

## Synthesis

# Issue 551: decision after the B5 stop rule. Path W v2 comes back with G-cover, flipped from an exact predicted set

## Decision

**Option: g-cover-first.** All three judges chose it. G-cover is adopted only through a decision rule written down now (U4). If it fails that rule, the fallback is v2 as it stands, which passed its pre-registered bars. Running out of time is never a reason to fall back.

- **Re-enabling v2 as it stands now is out.** It would knowingly serve again two heads already read as wrong. Both are among the 3,502:
  - 6465510 (G-cover refuses it).
  - 5816379 (G-cover does not refuse it; it stays a named residual).
- **Keeping path W off is also out.** It keeps about 3,400 understated heads served. Their reads so far are about 97–98 % right: M2 fallback 29/30, read18 18/18, confirm30 28/1/1, band9 9/9, outside22 22/22. All 29 band heads read right.
- **Why the stop rule tripped.** The window was never re-based, and that is our own process error.
  - B3 had already recorded 41 ships of 277 walked after the 567/568/570 drains, against M1's 30.
  - 15,300 × 41/277 ≈ 2,265 predicts the 2,348 fires seen inside the projection's population. That figure was on record before the drain.
  - The 1,154 fires outside it were never sampled before B5.
  - The quality bars passed. The count rule was set on a stale base.
- **Why G-cover is legitimate even though it was fitted after the data.**
  - It only adds refusals, so v2+G ⊆ v2 ⊆ v1.
  - It repairs the implementation of a clause R10 already had: "not a single-lot notice's own award". `usable_totals` (crates/store/src/canonical.rs:4626) can only spot that case through a stored award row that has a lot key and a value. 6465510's notice 20772908 has neither.
  - It gets no credit on the 61 heads it was designed on.
- **Checked in the code: Rust and the port can agree on the hard case.** `lots_awarded` (crates/ingest/src/project.rs:6908) collects `lot_key` with `filter_map`, so a result stored with lot = null adds no key. A notice with no round gets an empty set through `unwrap_or_default`. So 20772908's `lots_awarded` is empty and G-cover refuses 6465510 in Rust too.
  - Caveat: closed and no-award results that have a key DO count. The port must read it the same way ("any result row on the key"), not only rows with an award.
- **Proxy check of the pins on B3's cached served tenders.** Each pin's vouching notice names its summed keys in served `lot_results`, so G-cover keeps all three:
  - 8426318: TED-HIGH on 20083570, which names LOT-1..7.
  - 585917: BT-118 on 25416405, which names LOT-0001/0002.
  - 5995030: TED-VAL_TOTAL on 19612533, which names LOT-1..42 except 36.
  - U3 confirms this on the real facts.

### How G-cover's effect is measured, and on which facts

Both sides are used, with parity checks.

1. **Port on public-API facts.** `rule_v2_g.py` runs over all 3,502 ids, fetched fresh. This gives P, the exact predicted set.
2. **Rust on store, under v2.** That result already exists: `served_lsa_rows.json` is what Rust v2 served on the store. Before any G result is trusted, the port's v2 must reproduce it id for id.
3. **Rust v2+G on fixtures built from the same public facts.** This is the B3-style harness. It must agree with the port's v2+G with 0 disagreements. A projection-parity check pins `stated_totals.lots_awarded`, including the lot = null case.
4. **Rust on store, under v2+G.** This is only observable in the drain itself, so the drain's stop rules compare it against P id by id.

### Re-based window

The 835–3,338 window is retired. The new prediction is an exact set:

- **Base count:** |P|, where P is the port's v2+G ships among the 3,502, re-run on refetched facts for any id whose facts changed. Each id in P carries its predicted S, currency, n and `vouched_by`.
- **Tolerance on the 3,502:** 0.
- **The only rate-based part:** new fires among Tenders whose facts changed since project 2627.
  - Projection: admitted × 3,502/251,526 (1.39 %).
  - Stop and read if the count exceeds max(2 × projection, projection + 10).

### Is a corpus walk needed?

No. The id list is enough.

- 2627 decided v2 for every cohort Tender (251,526, D7 complete), and no other fold ran before the switch-off. So on 2627-era facts, v2's fires are exactly the 3,502, and v2+G's are a subset of them.
- Only Tenders whose facts changed after 2627 can newly fire. These are 541's 2631/2632, the daily chains from 10-19 onward including their rederive refolds, and 571's drain if it lands first.
- Enumerate them as the union of:
  - every tender id with a change row in `/v1/changes` from 958787061 up to the cursor read right after the flip deploy;
  - the tender ids of every notice ingested since 2026-10-18 20:30Z. This covers a rate move on a Tender whose head is null, or a fold that wrote no change row.
- Missing a fire only loses a fix; it never serves a wrong value.
- The corpus walk (~5 h, including the 14.9M-notice plan) is needed only if a future change relaxes a guard or re-keys lots. An optional reconciliation walk can run later in a quiet gap; it should correct about 0.

---

## Units

**U0 — Record first (S, doc; before any fetch or read).** Add a "B5 re-registration" block to issue 551 that fixes:

- (a) **The process error.** The count rule tripped because the window was not re-based on B3's 41/277. From now on, any B3-type rate change re-registers the window.
- (b) **confirm30 stands as recorded:** 28 right / 1 neutral / 1 wrong, PASS with no margin. It is never re-scored as "0 wrong under G".
- (c) **outside30 is an investigation measure only.**
- (d) **G-cover gets no credit on the 61.**
- (e) **Excluded from every new bar sample:** the 279 labelled, 277 walked, 6 px2 ids, 7511890, M1's and M2's reads (including M2 fallback's 30), read18, and the 61.
- (f) **Salts:**
  - `sha256('551-b6-gcost-'+id)` for the refusal read;
  - `sha256('551-b6-fresh-'+id)` for the pre-flip read;
  - `sha256('551-b6-delta-'+id)` for new fires after the drain;
  - `sha256('551-b6-daily-'+id)` for the day-7 read of daily-derived heads.
- (g) **G-cover's single wording, with no variants:**
  > A stated total of kind Award (BT-161, BT-118, BT-1118, TED-VAL_TOTAL on a CAN-type form) or Range (TED-LOW, TED-HIGH) is a candidate for R10's vouch only if its notice's `lots_awarded` (every lot key one of that notice's LotResults names in this version's rounds, closed and no-award results included, keyless results excluded) shares at least one key with the summed keys. Estimate-kind totals are unchanged. R3 is unchanged.
  - It filters R10's candidates. With no candidate left, the refusal name stays **R10**, so the vocabulary does not change.
  - Both sides also emit a harness-only diagnostic, `g_removed`.
- (h) **Every bar and stop rule below, verbatim.**

**U1 — Port of record (S, read-only).**

- Write `b6/rule_v2_g.py` from `b3/rule_v2_b3.py` (sha256 5400bc55…), and a matching `gen_b6.py` from `gen_b3.py`. Record both sha256 values in 551 before any run.
- Self-tests must hold:
  - 6465510 refuses at R10, from `b5/investigate/served/6465510.json`;
  - 7749755, 6779279, 6850401 and 7698260 (identity vouches) still ship with the same S;
  - so do the partial sums 6679375, 7153588, 7398935 and 7722197;
  - every other head in the 61 is unchanged.
- Re-run B3's populations (279 / 6 / 277 / 4,745) through the port. Every move must be ship → refuse. List the movers, including any read-right heads lost, as a recorded cost.

**U2 — Fetch and size on the actual population (S, read-only; from today).**

- **Fetch.** Public-API facts for the 3,502 (`served_lsa_rows.json` minus 179732):
  - tenders first (about 3,502 GETs, about 1.5 h), then the notice contents the M1/B3/B5 caches lack;
  - at most 1 GET per 1.5 s, a 408 retried once, no ssh;
  - paused from 07:30Z until the 10-20 chain shows done on `/health`;
  - about 19k GETs in total, roughly 8 h.
- **v2 parity against the store (gate on the method).** Port v2 must equal `served_lsa_rows.json` on ship, S, currency, n and `vouched_by`, for every id not in L_changed. Any disagreement blocks the flip until it is explained and fixed, as with 179732.
- **Outputs, recorded in 551:**
  - P, as an id list with its sha256;
  - g = |3,502 \ P|;
  - Rg, the refused set, with its shape mix (vouch field and year);
  - how many ids in P have the 5816379 shape (an award-type vouch from a notice superseded by a later award notice in the chain). Recorded only; not a bar.
  - the predicted band set B_P (ships in P at EUR 100 m or more).
- **If the fetch cannot finish before 10-20 07:30Z,** fall back to judge 1's two-pass method:
  - proxy candidate set C: ids whose first award/range vouch notice names no summed key in served `lot_results`;
  - exact port run only on C plus every id whose first vouch is award/range;
  - the parity in the method bullet above then covers only the ids actually fetched, and that coverage goes on record.

**U3 — Rust build and parity (M, code; LOT_SUM_AWARDS_PATH stays false).**

- **Build.** G-cover goes into canonical.rs R10 as a named pure function, used by `r10_vouching_total` through `usable_totals`.
- **Tests,** run through `head_value_with_awards_path`:
  - `a_vouch_from_a_notice_that_awards_no_summed_lot_does_not_count`, on a real-row fixture of 6465510 built by `gen_fixtures.py`;
  - the four identity vouches and four partial sums still ship;
  - 585917, 5995030 and 8426318 serve their pins (G's outcome recorded explicitly);
  - the Lyon-Turin trio and the path-V pins are byte-identical;
  - from real contents, a unit test that 20772908's `lots_awarded` is empty.
- **Temporary harness** (B3's `harness/`): Rust v2+G against `rule_v2_g.py` over:
  - B3's 5,307 rows, the 563 cohort, the 61, and fixtures of all 3,502 from U2's facts.
  - **Bar: 0 disagreements** on ship/refuse, S, currency, n, first refusing check and `vouched_by`.
  - Plus projection parity (`NoticeState::read` → `fold`) of `stated_totals` including `lots_awarded`, on the 562 + 61 + 3,502.
  - Then remove the harness and confirm `crates/` is byte-identical to the U3 commit.
- **Gate.**
  - Focused test with CLAUDE.md's full flags and package set.
  - Then `ops/check.sh` redirected to a file. Read the `GATE-EXIT=` value. Never pipe it through a filter.
- **Deploy** with W=false in any queue gap outside the 07:35Z chain. It writes nothing.
  - **This rev is the pre-gated switch-off rev from now on.** A later switch-off is a redeploy of it (minutes), not a fresh gate.
- **Flip commit.** Prepare it separately: the same code plus `LOT_SUM_AWARDS_PATH = true`, tests re-pinned through the test-only twin, docs/CHANGELOG/operations.md updated. Gate it in advance and record its marker.

**U4 — G-cover adoption (S, decision by the rule fixed in U0).**

- **Precondition:** U2's v2 parity and U3's parity are both clean. Any disagreement blocks G-cover. If it cannot be explained in time, the flip waits; it does not fall back to v2 as it stands.
- **g ≤ 350 (10 % of 3,502):** adopt G-cover. The refused heads are still read for the record in U5 (all if 30 or fewer, otherwise a salted 30 using `551-b6-gcost-`). This is not a bar.
- **g > 350:** read a salted 30 of Rg (`551-b6-gcost-`, read ids excluded) under M2's protocol. Adopt G-cover only if **20 or fewer of the 30 read right**, meaning at least a third are lost fixes of wrong or neutral heads. Otherwise reject G-cover in this form and flip v2 as it stands (P := the 3,502), with 6465510 and 5816379 as named residuals.
- **No rewording of G-cover after seeing g.**

**U5 — Fresh read of the rule that will ship (S, read).**

- **Draw:** P minus every read id, in `551-b6-fresh-` order, first 30.
- **Reading:** a reader and an adversarial skeptic on each head, a third reader on disagreement, wf_dc85fa7e-c00's rubric.
- **Band:** every head in B_P that is not among the 29 already read is read in full, at 0 wrong of any class.

**U6 — Freeze the prediction (S; in the flip gap, before the deploy).**

- **Timing:** after the 10-21 07:35Z chain and its rederive refold show done on `/health`, and after 560's U-d bounded read has taken its data or snapshotted its inputs.
- **Enumerate L_changed** with the union defined above. Read the cursor again right after the flip deploy and add any ids in between. There should be none, because no chain runs.
- **Refetch** the facts of L_changed ∩ 3,502 and re-run the port. The result replaces those ids' entries in P.
- **Record:** the final P (sha256), the predicted band set B_P, the pins' expected values, and L = 3,502 ∪ L_changed split into runs of at most 5,000.
- **Snapshot** the served value and `value_derivation` of every id in L before the wet run.

**U7 — Flip and drain by id list (S, ops; one gap, no chain in between).**

- **Order:**
  1. Deploy the pre-gated flip rev.
  2. Run `refold-lot-sum` with the 3,502 `tender_ids`: dry, then wet with `expect`, then project.
  3. Check stop rules D1–D5.
  4. Only then run the L_changed runs (dry, wet with `expect`, project), and check D6.
- **Start rule:** do not start unless everything can finish at least 2 h before the next 07:35Z.
- **571:** its 67-Tender drain runs either before U6's enumeration in the same gap, in which case its ids are in L_changed, or after U8 ends. Never at the same time, and never on 560's mornings.

**U8 — Verify and read the delta (S, read).**

- Bounded primary-key reads of every id in L, the pins and the band seek, checked against D1–D7.
- Read every new fire in L_changed that is not among the 3,502: all of them if 30 or fewer, otherwise a salted 30 using `551-b6-delta-`, with U5's bar.
- Record everything in 551 and in operations.md as "Path W v2+G" and its runbook.

**U9 — 14-day watch and standing guards (S, ops).** After each 07:35Z chain:

- Band seek for new `lot-sum-awards` rows of EUR 100 m or more. Each is read at 0 wrong of any class, otherwise switch off.
- Count of new `lot-sum-awards` heads against 1.39 % of the Tenders the chain admitted. More than 3x means stop and read.
- On day 7, read a salted 30 (`551-b6-daily-`) of heads the daily folds derived, a population never measured before. Same sequential bar as U5.
- **Standing rule:** no lot-key drain while W is on (567/570 single-section placement, 571 follow-ons) until the port diff has been re-run on the served `lot-sum-awards` set.
  - Pin 6908248 against becoming a double count.
  - Pin the partial sums 6679375, 7153588, 7398935, 7722197, 6779279, 6850401 and 7698260.
  - Read any band change.

**U10 — Follow-ups to file (not blockers).**

- Placement of single-Lot-section LotResults, plus vouching by the sum of disjoint notice totals (the partial-sum family).
- The 5816379 superseded-vouch shape. Any guard for it must be sized and freshly read the same way before adoption; it does not ride on this flip.
- An optional corpus reconciliation walk in a quiet gap that is not a D5-sensitive morning. Expected corrections: 0; any correction is read as an enumeration miss.

---

## Bars (fixed now; all must hold before the flip deploy)

- **B-parity:** U2's port-v2 equals `served_lsa_rows.json` on every id not in L_changed, and U3's Rust-vs-port v2+G harness shows 0 disagreements, with `lots_awarded` projection parity clean. Gate green, read from `GATE-EXIT=`.
- **B-adopt:** U4's rule applied as written.
- **B-fresh (sequential):**
  - 0 wrong of 30 → go.
  - Exactly 1 wrong → continue the same salted order for 30 more, which must have 0 wrong (1 of 60 or fewer).
  - 2 or more wrong → keep path W off; it is withdrawn pending a new decision (unit-X shape), with no third variant.
  - Neutral heads outside the band are allowed and counted.
- **B-band:** every head in B_P outside the 29 already read reads 0 wrong of any class.
- **B-time:** 560's 10-20 and 10-21 mornings are clean. The flip happens no earlier than the 10-21 post-chain gap, or the 10-20 post-chain gap if 560 records U-d done on the 10-20 morning alone. If any bar is not met, wait for the next post-chain gap. There is no clock-driven flip of v2 as it stands.

## Drain stop rules

**Switch off** means: redeploy U3's pre-gated W=false rev, then `refold-lot-sum` with `tender_ids` = the ids serving `lot-sum-awards` (runs of at most 5,000), then verify that the band shows 0 and the primary-key reads show 0.

- **D1 — exact set, 3,502 run.** The ids corrected to `lot-sum-awards` must equal P exactly.
  - An unpredicted fire, or a predicted id with a different S, currency, n or `vouched_by`: **switch off**.
  - A predicted id that does not fire: stop the drain and explain it within the gap; if it is unexplained by the end of the gap, **switch off**. It means Rust on the store ≠ the port, and the parity premise is broken.
  - Ids in Rg must compare identical.
- **D2 — count.** corrected = |P| exactly. Tolerance 0, because L_changed ∩ 3,502 was refetched and re-predicted in U6.
- **D3 — pins.**
  - 585917 serves EUR 250,000.00, 5995030 PLN 1,549,511.60 and 8426318 GBP 3,000,000,000.00, each `lot-sum-awards` with its recorded `vouched_by` (or U3's recorded G outcome).
  - The Lyon-Turin trio 6450467/69/73 keeps EUR 1,660,661,682.94 with no derivation.
  - The 16 path-V pins are byte-identical.
  - The 9 must-stay heads serve no derivation.
  - 6465510 does not serve `lot-sum-awards`.
  - Any one off: **switch off**.
- **D4 — band.** The plan-probed seek (≥ EUR 100 m) must equal B_P.
  - A head missing: **switch off**.
  - An extra row: stop and read it at 0 wrong of any class; switch off only if it reads wrong.
- **D5 — scope.** Any id serving `lot-sum-awards` outside P ∪ L_changed: **switch off**.
- **D6 — L_changed runs.** Projection = dry-admitted × 3,502/251,526.
  - New fires above max(2 × projection, projection + 10): stop and read before the next run.
  - U8's delta read uses U5's sequential bar; failing it means **switch off**.
  - Any delta band row is read at 0 wrong of any class.
- **D7 — time.** If the drain could overrun into the next 07:35Z chain, do not start it. If it is already running, switch off before the chain.

## After the drain, what switches path W off again

Any of the following:

- a failed delta read (U8);
- a U9 band row that reads wrong (any class);
- a U9 day-7 read that fails the sequential bar;
- an unexplained daily count above 3x;
- a lot-key drain landing without the port diff re-run first.

## Risks (accepted, named)

- G-cover is fitted in-sample; U5's fresh draw is the answer.
- The 5816379 shape stays unguarded, expected at about 1 in 60, with errors of about 1.5x.
- Partial sums are fragile against placement drains; U9's standing rule covers this.
- The id list depends on change rows plus the ingest cross-check. A miss only loses fixes, and the optional walk catches it.
- With W on, daily folds derive heads on new notices before anyone reads them. U9 is the only guard on those.
- B-fresh at 0/30 still allows up to about 10 % (95 % upper bound); the sequential 60 and U9 narrow it.

Files consulted:

- /home/user/tender-db/.scratch/tender-db/issues/551-a-refused-head-falls-to-one-lots-figure-or-null-never-the-lot-sum.md
- /home/user/tender-db/.scratch/tender-db/551-lot-sum/b5/investigate/read/read.md
- /home/user/tender-db/.scratch/tender-db/551-lot-sum/b5/investigate/served_lsa_rows.json
- /home/user/tender-db/crates/store/src/canonical.rs (lines 4626, 4908, 7106–7148)
- /home/user/tender-db/crates/ingest/src/project.rs (line 6908)
- /home/user/tender-db/.scratch/tender-db/issues/560-a-tender-value-move-writes-no-lot-change-row-so-a-lots-value-subscriber-keeps-the-lot.md

## Critic (amendments, all adopted)

# Critic: amendments to the issue 551 "g-cover-first" plan

## Verdict

The direction holds up: v2+G, flipped from an exact predicted set, with no corpus walk. But five parts of the plan can't be carried out as written, or would trip falsely:

- **G-cover is wired to a field no guard reads.** The plan builds it on `lots_awarded`, which nothing in canonical.rs consumes.
- **The port can't run as written.** `rule_v2_b3.py` is offline-only and reads stale tender data.
- **`vouched_by` equality would switch path W off on a known naming difference.**
- **Code that changed after 2627 is not accounted for.** 571 unit 2 is committed and changes result winners.
- **Disk is at 4.9 GB free (88 % used).**

The amendments come in four groups: mechanism, statistics and adoption, drain and stop rules, then timing, ops and risks.

---

## A. Mechanism: what the code and scripts actually support

**A1. Build G-cover on R3's accessor, not on `StatedTotal.lots_awarded`.**
- **What:** In Rust, G-cover's predicate should be `view.summed.iter().any(|s| view.on_lot(s.key).iter().any(|(n, _)| *n == t.notice_id))`. That is exactly the test R3 uses for "keys that notice awards" (canonical.rs:4664–4668). In the port, use the same `results_by_lot` test R3 uses (rule_v2_b3.py:827). Drop U3's "`lots_awarded` of 20772908 is empty" test as a decision test. Replace it with an end-to-end test: real contents → `NoticeState::read` → `fold` → `head_value_with_awards_path` refuses 6465510 at R10.
- **Why:**
  - `lots_awarded` is read by no guard. The only hits are the struct (canonical.rs:7148), the fixtures (canonical_551_v2_fixtures.rs:176) and tests.
  - The fixtures set it from generator data. That is the same kind of input-encoding gap that hid 179732 from B3.
  - It is recomputed only when the version's notice has a round or totals (project.rs:6878), so it can go stale.
  - `filter_map(as_deref)` keeps `""` keys, while `AwardsView` filters them out (canonical.rs:4268).
  - `on_lot` / `results_by_lot` already carry B3's 0-disagreement R3 parity. The plan's check of project.rs:6908 is correct, but it verifies a fact the guard should not depend on.

**A2. The port has no `lots_awarded` and no "Range" kind.**
- **What:** In `rule_v2_g.py`, identify Range by `field in ('TED-LOW', 'TED-HIGH')`. In the port these have `r3_kind = None, vouches = True` (rule_v2_b3.py:466). Identify Award by `r3_kind == 'award'`. Exclude `None` and `''` lot keys.
- **Why:** U0(g)'s wording uses Rust's kind names. A port that filters on `r3_kind in ('award', 'range')` would silently never apply G to TED-LOW/HIGH, which covers 132 of the 3,502, including pin 8426318.

**A3. "A repair of R10's clause" is the wrong framing. Pre-register the narrower repair beside G-cover.**
- **What:** G-cover refuses any award or range total from a notice whose results name no summed key. That includes multi-block CANs whose blocks are all stored lot = null and whose total restates the whole procedure. That is a new refusal family, not the repair of "not a single-lot notice's own award".
- The real repair is read.md's own "equivalent fix", **G-single**: a total from a notice whose round has exactly one LotResult (keyed or not) does not vouch. 20772908 has exactly one result row (lot = null, checked in `served/6465510.json`), so G-single also refuses 6465510.
- U0 should fix, before g is seen:
  - U1 and U2 compute both sets.
  - G-single is adopted under U4's rule.
  - The extra refusals of G-cover (G-cover \ G-single) are adopted only if a read of them (all if 30 or fewer, otherwise a salted 30) finds at least a third non-right.
- **Why:** Only the part of G-cover beyond the minimal repair was fitted after the data. Without this, "no rewording after seeing g" locks in the broader rule untested.

**A4. Compare `vouched_by` by membership, not equality (U2 parity, D1, D3).**
- **What:** A Rust `vouched_by` counts as matching when it is in the port's G-filtered `vouches` list (rule_v2_b3.py:885) and its kind passes G. Also record that G can move `vouched_by` on ids that still ship: when an earlier award or range candidate is removed, the first vouch shifts to a later total.
- **Why:**
  - Rust names the first vouching total in B1's chain and read order. The port groups each notice's values by field (`fids` dict, :485). This is B2's recorded name-level difference.
  - `cohort_parity.py` already allows it.
  - As written, D1 would switch path W off over a naming difference on any id where one notice states two vouching totals.

**A5. The port cannot fetch, and its tender layer is stale.**
- **What:** U1 must also produce a fetcher for an id list. Adapt `b5/cohort_parity.py fetch`.
  - `rule_v2_g.py`'s `_cached` must read tenders only from the new fresh U2 layer.
  - Notices are read from U2's layer, then the M1, B3 and bundle layers (notice contents are immutable).
  - Refresh the rates files and record their sha256 before U6.
- **Why:**
  - `rule_v2_b3.py` raises "B3 never fetches here" on a cache miss. Its fetch code is dead after the `raise` (:241).
  - For tenders it reads only `B3_RAW`, which holds B3-era reads. Any of the 562 inside the 3,502 would be decided on pre-B5 tender data without any error.
  - The rates are dated 10-17. They are good for head dates only up to 10-24 through the 7-day carry, and at the 10-16 fixing, not the store's.

**A6. U1's 6465510 self-test needs notice contents.**
- **What:** Confirm that all 20+ version-notice contents of 6465510 are cached (beyond `read/` rdr_*), or fetch them, before U1 runs.
- **Why:** `facts_from` builds its totals from notice contents. `served/6465510.json` alone cannot produce the TED-VAL_TOTAL candidate on 20772908.

## B. Statistics and the adoption rule

**B1. U4's rule is not monotone.**
- **What:** Replace it with one rule for any g. Read min(g, 30) of Rg with the salt `551-b6-gcost-`. Adopt G-single, plus the G-cover excess per A3, if at least 3 of the 30 are non-right. That is at least 3× v2's background non-right rate of 2/61 seen on the investigation's 61.
- **Why:** As written:
  - g = 340 with 0 of 30 non-right adopts: about 340 lost fixes and no measured benefit.
  - g = 360 with 9 of 30 non-right rejects.
  - Neutral counts as "worth refusing" in U4 but is allowed in B-fresh. Pick one valuation.

**B2. Reframe the decision on rates, not named heads.**
- **What:** Strike "v2 as it stands is out because it knowingly serves two read-wrong heads".
- **Why:** v2+G knowingly serves 5816379, and U4's fallback is v2 as it stands. The real argument is the rates: about 1/61 wrong, about 98 % right.

**B3. B-band is vacuous as written.**
- **What:** B_P is a subset of the 3,502's band, which is exactly the 29 already read (20 + 9). So B-band should require a re-read of:
  - any of the 29 whose predicted S, n or `vouched_by` changed (U6 refetch);
  - every band row from L_changed or the delta.
- D4 also needs a near-threshold allowance. An id flagged within 2 % of EUR 100 m (port rates vs the stored rate) that differs from B_P should mean stop and explain, not switch off.

**B4. Specify the B-fresh reading inputs.**
- **What:** Readers get the port-predicted S, the summed keys and notice ids, n and the G-filtered `vouched_by`, plus the live served head ("head without W").
- **Why:** The heads are drained back. Unlike the 61, there is no served snapshot of S.

**B5. Fix U2's parity exemption set before the comparison.**
- **What:** The exemption is the provisional L_changed: ids with a change row from 958787061 up to that id's own fetch time, or with a notice folded since 10-18 20:30Z, recorded before the comparison runs. Do not use U6's L_changed, which only exists later.
- **Why:** Otherwise the set that excuses disagreements is defined after seeing them.

## C. Drain, enumeration and stop rules

**C1. Code drift after 2627 breaks the id-list premise unless it is handled.**
- **What:** Add a precondition, recorded in 551:
  - List every commit touching `crates/ingest` or `crates/store` that sits between the deployed 9529345 and the flip rev.
  - Example: **571 unit 2, 21b2537**, which removes ADDRESS_PARTY winners. That is an input to R1, R5.1, R5.12 and R6, so it can create new fires.
  - Each such commit's drain must land before U6, and its **whole refolded id set** (named or admitted notice→Tender ids, not only the corrected ones) goes into L_changed.
  - A commit that changes projection-only facts (`stated_totals`, `published_lot_no`, `group`, rounds that are not served) writes no change row. If one lands, the id list is incomplete and a corpus walk is required.
- **Why:** "On 2627-era facts the fires are exactly the 3,502" holds only for 2627's code. Change rows cover served diffs only.

**C2. Add a zero-drift probe after the U3 deploy.**
- **What:** Run a `refold-lot-sum` with the 3,502 `tender_ids` at W=false, wet. Expect `corrected = 0`. Do it in a gap that is not one of 560's mornings. Any correction is drift, to be explained before U6.
- **Why:** It establishes that the store sits at the current code's fixed point. Only then do D1 and D2 measure path W and nothing else. 2630 folded under 9529345, and anything deployed since is unmeasured on these ids.

**C3. Complete L_changed.**
- **What:** Add to the union:
  - the full refolded sets of 541's 2631/2632 and of 571's drain;
  - the moved ids of `rederive-eur-recent` for 10-19 to 10-21;
  - for the ingest cross-check, the ids of notices **folded** since 2627, not `ingested_at` (notices fetched before the cutoff but parsed later fall through).

**C4. Measure D1 and D2 on a snapshot diff, scoped to the named ids.**
- **What:**
  - D1 and D2 compare U6's PK snapshot with post-run PK reads of the named ids: lot-sum-awards transitions only.
  - The job's `corrected` count must equal |P| plus the drift found by C2 (expected 0).
  - A fire in the touched expansion (the 2630 run folded 644 un-named Tenders) is judged under D5 and D6, not as a D1 "unpredicted fire".
- **Why:** The job counts any diff on any folded Tender. That makes D1 and D2 trip on drift or on a legitimate expansion Tender from L_changed.

**C5. Define D5 so it can be measured.**
- **What:** D5 is measured over the project's change-row cursor range: PK-read every changed id and flag any that serve lot-sum-awards outside P ∪ L_changed.
- **Why:** There is no index on `current_value_derivation` (operations.md says "never run the predicate without the value range"), and the band seek covers only EUR 100 m and up. Read literally, D5 needs a scan that has no on-box path.

**C6. Stage the 3,502 run.**
- **What:**
  - Stage 1: the 61, the pins, 7511890, every id in Rg, and a salted 300 of P (one run). Check D1–D4 on them.
  - Stage 2: the rest.
  - Optional, cost M: a read-only "shadow decide" job that folds named Tenders in memory with the on-twin and reports `lot_sum_decision`. It would move Rust-on-store parity ahead of the flip. If built, wrap the Spec arm in `Box::pin`, per CLAUDE.md.
- **Why:** The fixture harness never exercises the store's projection. 179732 was found only in the cohort drain on the store. Staging costs minutes and limits the churn of a switch-off.

**C7. Update the switch-off route for after the first daily chain.**
- **What:**
  - Once W is on, the id list is: drain ids ∪ every id with a change row since the flip cursor, PK-read for lot-sum-awards, in runs of at most 5,000. The alternative is the corpus refold (about 5 h, needs a gap away from 07:35Z).
  - "Redeploy U3's pre-gated W=false rev" rolls back any other agent's code deployed since. So before each gap, rebuild and re-gate both twins on the currently deployed base. Gates are bound to the rev (issue 459); budget 22–35 min.
- **Why:** B5's completeness argument ("no other writer") stops holding once daily folds derive heads.

**C8. Define U9's count rule properly.**
- **What:**
  - Daily chains don't run the cohort predicate, so "1.39 % of Tenders the chain admitted" has no denominator. Use the fire rate observed in the L_changed runs (daily-fold population, recorded before the first U9 check) on the chain's folded Tenders.
  - Count new lot-sum-awards heads via the chain's change-row range plus PK reads.
  - Day-7 read: the population is heads whose first lot-sum-awards write came from a daily fold (not in P). If there are fewer than 30, read all at 0 wrong.
- **Why:** As written, the 3× rule can never trip or can always trip.

**C9. Broaden U9's standing rule.**
- **What:** Cover any drain or deploy that changes path-W inputs while W is on, not only lot-key drains: winners (571; org merges, which feed R1 and R6), rounds, stated totals and published lot numbers. 571's drain goes before U6 or not at all while W is on.
- **Why:** 571 after U8 would be a W-on refold of 67 Tenders with no port diff.

**C10. Drop U2's two-pass fallback.**
- **What:** Remove it.
- **Why:**
  - D1 at tolerance 0 needs an exact P for every id.
  - B-time already forbids a clock-driven flip.
  - A fetch that spans a chain is already handled by L_changed.

## D. Timing, ops, risks

**D1. Disk.**
- **What:** `/` has 4.9 GB free (252 G, 88 % used). `target/` is 15 G and the scratchpad 3.9 G.
  - U2 adds about 0.6–0.8 GB (about 39 KB per notice × about 11–15k new contents). Store it gzipped.
  - Prune the scratchpad (old .db or xml artifacts).
  - Check `df` before the U3 harness and gate, aiming for at least about 15 GB free. The gate auto-cleans and rebuilds from scratch when build inputs change.
  - Generate the harness's 3,502 fixtures at run time; don't commit them.
- **Why:** CLAUDE.md's ENOSPC trap shows up as GATE-EXIT=101 with no FAILED line.

**D2. B-time on 10-20.**
- **What:** Allow the 10-20 post-chain flip only if 560 records U-d as **PASS and closed** from the 10-20 morning. If its read fails, it needs the 10-21 morning clean, and a 10-20 afternoon drain over L_changed (daily-fold Tenders) can land inside the window that morning's read counts. 562 unit 5's paired read must have taken its daily-fold cursor ranges first.

**D3. U3 must show that nothing stored moves at W=false.**
- **What:** fold_writer and project_refold snapshots byte-identical, recorded.
- **Why:** This rev deploys before 560's mornings, when daily folds must be the only writers with unchanged effect.

**D4. Measure an unexamined risk class: estimate-kind coincidence vouches.**
- **What:** 604 of the 3,502 are vouched by TED-VAL_ESTIMATED_TOTAL, and G doesn't touch them. Chains hold single-lot relaunch CNs (7698260's lot 3). In U2, record how many ids in P are vouched by an estimate from a CN with exactly one lot section, where that lot is not in S. Record only; it is the estimate twin of 6465510. Flag such heads to readers in B-fresh.

**D5. Note the subscriber churn.**
- **What:** Note in CHANGELOG and operations.md that the |P| heads take their third correction round (on, off, on).
- **Why:** Lot and SSE subscribers see each of them again.