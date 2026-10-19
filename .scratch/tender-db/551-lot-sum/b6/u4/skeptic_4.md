# 551 B6 U4 guard-cost read, skeptic 4: the non-right from read 4 (7179291)

2026-10-19, skeptic `skeptic_4`. This is the adversarial skeptic on the one non-right verdict in `read_4.md` (R_single
salted ranks 21–26). The brief: try to show that the v2 ship is in fact RIGHT under M2, meaning the summed awards are
the tender's value and the stated total vouches for them. Keep neutral or wrong only where the documents show that the
sum misstates the value.

**Inputs (hashes as `read_set.json` d77efbc6… records):** `b6/u2/R_single.json` c1b413e0…, `b6/u2/R_excess.json`
3d134390…, `b6/u2/decisions.jsonl.gz` 0add92e3…. The tender and notices come from the fresh U2 layer
(`scratchpad/b6/tenders`, `scratchpad/b6/notices`, including CN 21936672) and from the bodies `read_4` saved in
`b6/u4/bodies/`. The saved `tenders_7179291.json` and the four award notices (22148426, 22315443, 22364146, 22780357)
are each JSON-equal to their U2-layer copies.

**Prior read:** none. 7179291 shows up in b5 only as a member of population and order lists (`confirm_result.predraw.json`
order, `p2575_ids.json`, `served_lsa_rows.json`, `post_switchoff_rows.json`, `corpus_changes.json`). It has no verdict in
b3/read18, the B5 investigation 61, the post-drain read, M1, M2 or the M2 fallback. Its salted rank is re-derived as
**23 of 34** (`sha256('551-b6-gcost-7179291')` = af0aef5f…), inside the first 30. It is also one of G-cover's 12
R_single-overlap ids.

**New GETs: 2 of the 4 allowed.** Both are logged in `api_requests.log` as `reader=skeptic_4`. Each returned 200 on its
first attempt, spaced at least 2 s apart.
- `/v1/tenders/7179291` → `bodies/skeptic4_tenders_7179291.json`. JSON-equal to the U2 copy: version 5, head
  EUR 83,146.73, `value_derivation` null, five versions (CN 21936672, then 22148426, 22315443, 22364146 and 22780357).
- `/v1/tenders?buyer=1950314&cpv=35111000&limit=100` → `bodies/skeptic4_tenders_buyer-1950314_cpv-35111000.json`. This
  looks for a twin tender of procedure `ojs:2021-460838` that could hold a later call-off or a closing total. The
  result has 11 tenders, `more` false and no ignored filters. Only 7179291 carries `ojs:2021-460838`, so the procedure
  has no twin.

## Verdict

| id | set | prior | v2 S (n) | v2 vouched_by | reader | skeptic 4 |
|---|---|---|---|---|---|---|
| 7179291 | R_single (also G-cover overlap) | no | EUR 83,349.90 (3) | TED-VAL_TOTAL EUR 83,146.73 on 22780357 | wrong / other | **wrong / other (upheld)** |

## The case for RIGHT, and why it fails

The port's v2 decision ships S = 8,334,990 cents EUR (n = 3) on one candidate: TED-VAL_TOTAL 8,314,673 on 22780357.
That candidate is F03, kind award, with `round_results` 1 and `names_summed_keys` []. G-single removes it (one result),
and so does G-cover (no summed key named). R10 then refuses under {single}, {cover} and {single, cover}.

**The chain.** Every award notice is F03 with FRAMEWORK = 1. CN 21936672 (3 lots, no amount of any kind, 24 months)
gives each lot's quantity as "preliminarus kiekis preliminariosios sutarties galiojimo laikotarpiu", the provisional
quantity over the framework agreement's term (288 / 288 / 108 units). There are four award notices:

| notice | published | RES | printed LOT_NO | CONTRACT_NO | concluded | VAL_TOTAL (RES = procedure) | stored lot | in S? |
|---|---|---|---|---|---|---|---|---|
| 22148426 | 2022-01-11 | RES-1 | "1, 2, 3" | 35-1 (framework) | 2022-01-03 | 187,227.36 | null | no |
| 22315443 | 2022-04-29 | RES-1 | 3 | 35-103 | 2022-03-28 | 28,900.00 | LOT-3 | yes |
| 22364146 | 2022-05-24 | RES-1, RES-2 | 1, 2 | 35-165 (both) | 2022-05-12 | 26,372.00 + 28,077.90 = 54,449.90 | LOT-1, LOT-2 | yes |
| 22780357 | 2022-11-03 | RES-1 | "1,2" | 35-372 | 2022-09-30 | 83,146.73 | null | no (vouch) |

The three later notices name 22148426's OJ number (2022/S 007-013596) as this procedure's previous notice. They are
call-off contracts under framework 35-1.

1. **The summed lots are clean.** This holds. Each LOT-k is a printed LOT_NO k with its own title, and the three lots
   are distinct.
   - There is no double count: the keyless multi-lot rows (35-1 and 35-372) are not summed.
   - There is no ×10ᵏ slip and no VAT mix: every figure is a TED-VAL_TOTAL in EUR.
   - So the sum is arithmetically clean. It is exactly two call-off contracts, 35-103 + 35-165.
2. **S is the procedure's value.** This fails, under every reading of "value" that the documents support.
   - **The framework award reading.** 22148426's procedure total is 187,227.36 (TED-VAL_TOTAL and VALUE
     PROCUREMENT_TOTAL), and so is its single framework contract 35-1. S is **44.52 %** of it. The head without W
     (83,146.73) is 44.41 %.
   - **The call-off spend reading.** The three published call-offs add to 28,900.00 + 54,449.90 + 83,146.73 =
     **166,496.63**. S is **50.06 %** of that; the head is 49.94 %.
   - **The per-lot latest award reading.** Contract 35-372 (2022-09-30, lots "1,2") is later than 35-165 (2022-05-12)
     for lots 1 and 2. In a framework the call-offs add up rather than replace each other. Even if 35-372 superseded
     35-165, lots 1 and 2 plus lot 3 would give 83,146.73 + 28,900.00 = 112,046.73, which is not S either.
   - **No notice of the procedure states 83,349.90.** The four notice totals are 187,227.36, 28,900.00, 54,449.90 and
     83,146.73, and the CN states no amount.
3. **The total vouches S.** This fails. T = 83,146.73 is contract 35-372's own award, and three facts show it:
   - **The lots differ.** 22780357's one LotResult prints LOT_NO "1,2", so T covers lots 1 and 2 only. S includes lot 3
     (28,900.00), so T cannot be a statement of S's scope.
   - **The contract differs.** RES-1 is CONTRACT_NO 35-372, concluded 2022-09-30 with 4 tenders. S is 35-103 + 35-165,
     concluded 2022-03-28 and 2022-05-12.
   - **The figure is the notice's own result.** RES-1's VAL_TOTAL is 83,146.73, and so are the procedure VAL_TOTAL and
     VALUE (PROCUREMENT_TOTAL) of the same notice.
   - S / T = 1.00244 (+0.24 %). That is inside R10's 1 % only because two different call-off batches happen to be about
     the same size. This is the rubric's explicit wrong criterion (the vouching total belongs to something else), and
     it is the same class M2 recorded for 6465510, where reader and skeptic agreed.
4. **Could 83,146.73 be a cumulative or procedure-wide total?** No, and the arithmetic settles it.
   - When 22780357 was dispatched (2022-10-29), the call-offs already published added to 28,900.00 + 54,449.90 =
     **83,349.90**, exactly S. A notice restating the running total would print 83,349.90 before its own contract, or
     166,496.63 including it. It prints neither: it prints 83,146.73, 203.17 below S, in both its RES-1 and its procedure
     total.
   - The figure is not a net/gross pair with S either: 83,349.90 / 83,146.73 = 1.0024, not Lithuania's 1.21.
   - It is not the framework value (187,227.36), and it is not S plus the framework headroom.
   - The buyer's other call-off notices in this chain (22315443, 22364146) each state exactly their own contract's
     total, 28,900.00 and 54,449.90. So 22780357 following the same practice is the plain reading.
5. **Could it be neutral rather than wrong?** S is not farther than the head without W. It is EUR 203.17 closer under
   both the framework total and the call-off total. So the "farther" class does not apply.
   - 7749755 was neutral because its vouch was an identity vouch: a real buyer-stated total of exactly the summed awards.
     Here the vouch comes from a notice that awards none of the summed lots, which is 6465510's class (wrong / other).
   - 6465510 was also nominally closer than its head (18.8 % against 8.2 %) and was still recorded wrong. Served, S would
     carry the "lots: 3, vouched" label for a figure no notice states, with a vouch from another contract.
   - If the vouch criterion were set aside, S would be neutral (not farther, not better), as `read_4` says.
   - For the adoption count, neutral and wrong are both non-right, so the choice between them moves no tally.
6. **The guards read the right fact.** 22780357 has three Lot sections (LOT-1..LOT-3). Its single result, though, is a
   multi-lot "1,2" result stored keyless. G-cover's accessor (R3's `view.on_lot`, keyed results only) correctly finds
   that the notice names no summed key. G-single correctly counts one LotResult. A section-id reading would wrongly have
   kept the vouch.

**Side note (does not affect the verdict):** neither keyless row (35-1 "1, 2, 3" and 35-372 "1,2") is a single-lot
placement candidate, so a future 567/570-style placement drain would not change S here. The tender's head without W
(83,146.73, call-off 35-372 alone) is not the procedure's value either. That is a path-V matter outside this guard's
question.

## Effect on the adoption rule

- **G-single (R_single read set of 30):** 7179291 stays non-right. Read 4's slice tally is unchanged: 6 read, 1
  non-right.
- **G-cover gap case (13-read set):** 7179291 keeps wrong / other as one of the 12 overlap verdicts.
