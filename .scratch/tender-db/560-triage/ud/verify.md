# 560 U-d: adversarial verification of the 10-19 coverage read (2026-10-20)

**Verdict: PASS. No stop.** The measurer's numbers, the criterion it applied, and its pass and no-stop calls all hold up when re-derived from the saved SQL and outputs. One extra bounded read (`v_chain.*`) confirms amendment (d) from the data rather than from the job list.

## Criterion and bounds, checked against the issue

- **The query.**
  - `q_ud_1019.sql` has the same shape as the baseline `d56x/adv/q_adv.sql` except for its timestamps. I checked this by diff: the baseline has 1791936000 once and 1792022400 three times, and the new query has 1792368000 once and 1792454400 three times, plus 1792540800 in place of 1792108800.
  - Publication day: [1792368000, 1792454400) = 2026-10-19 00:00Z (a Monday) to 10-20 00:00Z.
  - D5 window (cov15): [1792454400, 1792540800) = 10-20. covany counts from 10-20.
  - `plan_in_ud.sql` is the same text as `q_ud_1019.sql`, ignoring whitespace.
- **The first day folded entirely after the deploy.** The deploy was 10-17 10:50Z. 10-17 and 10-18 are a weekend, so there is no TED publication. 10-19 TED versions were folded at 07:35:55Z on 10-19 (the seq rows in `r_changes.out` carry 1792395355). The 10-19 FTS versions were folded by project 2669 on 10-20.
- **Amendment (a).** It credits only seq-less lot `changed` rows from the next morning's rederive. `q_silent.sql` narrows the window to rederive 2668 alone, [1792482078, 1792482086]. Decoding the `enc` values: 878700@360 is 100001000010, which is 10 silent lots, all 10 covered by the rederive and by the project compare. 1139127@3 is 10000100001, which is 1 lot covered by both. So D5 coverage does not depend on project 2669's same-day compare rows.
- **Amendment (b), the residue bar.**
  - Baseline from `q_adv.out`:

    | segment | me_lots | covered | residue |
    | --- | ---: | ---: | ---: |
    | TED EUR-only | 1,165 − 1,041 = 124 | 2 | **122** |
    | TED non-EUR | 1,041 | 1,009 | **32** |
    | FTS | 103 | 0 | **103** |

    That is 257 in total, matching the issue.
  - 10-19 from `q_ud_1019.out`:

    | segment | me_lots | covered (cov15) | residue |
    | --- | ---: | ---: | ---: |
    | TED | 11, all non-EUR | 11 | 0 |
    | FTS | 0 | 0 | 0 |

    **Residue 0 against 257.**
  - The original Verify bar (me_lots at or below 25% of 1,069 to 1,268) is also met: 11 is about 1%.
- **Seq-row coverage (`q_seqcov.out`).**
  - FTS: 216 of 216 lots covered.
  - TED EUR-only: 376 of 376.
  - TED non-EUR: 3,024 of 3,035, leaving 11 silent. That matches `me_lots` = 11 in the main query.
  - me_pairs: 126 + 137 + 623 = 886, which equals 126 + 760.
- **Amendment (d).** My read `v_chain.sql` / `v_chain.out` takes the rowid range of the chain's cursors, plan `SEARCH changes USING INTEGER PRIMARY KEY`, 6.2 s. It returns:
  - 55,419 rows, all stamped 07:41:18Z to 07:41:27Z;
  - 0 seq-less rows before the rederive;
  - exactly **4,824** seq-less rows inside the rederive's window, which matches job 2668's "4824 correction row(s) announced";
  - 6,070 seq-less rows after it, from the project compare and its stamping;
  - **0 rows past cursor 958904203.**

  There is no refold or drain signature: a drain writes millions of rows.
- **Plans.** Every plan file seeks. The only `SCAN` lines are `SCAN (subquery-0)` and `SCAN CONSTANT ROW`, and neither is a base table. `r_changes` uses CROSS JOIN, so lb drives and c seeks `changes_entity`.

## Amendment (c): the two silent versions

These are not residue: both are fully D5-covered. They are still the right versions to examine for the stop rule.

- **The election rule.** It is the max election (`canonical.rs` `elect_head_figure`): the largest admitted EUR figure among the head's procedure-scope and lot amounts, with Parts used only as a fallback. `current_value_derivation` is NULL for both tenders, so neither value is derived.
- **Units.** The `cents` column holds minor units: haléř, not CZK as the write-up says. The ratios are unaffected.

**878700, seq 360 (versus seq 359, published 10-16).**
- The largest figure is lot 2394899's estimate, 6,701,675,175 haléř, at both seqs. Nothing in the version is a 10^k partner of it.
- At the 10-16 fixing (24.323) it converts to 275,528,313.7, which rounds to 275,528,314. That is the stored value at seq 359.
- At the 10-19 fixing (24.343) it converts to 275,301,942.04. That is the stored value at seq 360 and equals `current_value_eur_cents`.
- The 10-19 fixing did not exist at 07:35Z on 10-19, so the fold saw 275,528,314 on both sides: no move at fold time.
- The rederive's 10-20 `changed` rows on every lot of this tender confirm that seq 360's EUR value moved only on 10-20.

**1139127, seq 3 (versus seq 2, published 10-16).**
- The procedure estimate is 3,140,495,800 haléř at both seqs.
- At 24.323 it converts to 129,116,301 (stored at seq 2). At 24.343 it converts to 129,010,220.6, which rounds to 129,010,221 (stored at seq 3, the head value).
- No move at fold time.

**Both are D5 cases under amendment (c), not stops.** This is exactly the misfire class in objection 3.

## Nits (they do not change the verdict)

- The write-up says "6,701,675,175 CZK" and "3,140,495,800 CZK". These are minor units (haléř).
- For 878700 the elected value is shown through the head (seq 361). The seq-360 election is inferred from the same unchanged maximum figure under the max rule, which is sound.
- `result.md` attributes the 4,609 correction rows to project 2669. My read finds 6,070 seq-less rows after the rederive. The difference is other seq-less stamping by the project and does not bear on coverage.

## Budget

1 `/v1/sql` request (`v_chain.sql`). It was plan-probed first on `d56x/adv/plan.db` and answered on the first attempt: no 408, no retry. No git, no admin jobs, no repo edits outside this directory.
