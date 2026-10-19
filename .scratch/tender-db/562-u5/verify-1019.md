# 562 unit 5, first half (2026-10-19 daily fold, job 2645): adversarial verification

**Verdict: PASS.**
- flips_announced == flips == 102
- ted_first_unflipped == 0
- co-minted rows == 0, so there is nothing to explain
- the read reconciles with the job line's 102 relabelled

One caveat, stated below: the relabelled = 102 figure comes from the task-supplied job line and was not re-read.

## What was checked in the measurer's work

1. **The flip criterion matches q1's notice-id form exactly.** `sql/ted_*.sql` against `d56x/i562/q1.sql`:
   - `pre` is `t.created_at < n.ingested_at`.
   - `doe_before` and `ted_before` are the same EXISTS checks over `tender_versions` ⨝ `notices`, with `n2.id < n.id` and the source test.
   - `analyze.py` sets flip = `pre AND doe_before AND NOT ted_before` on TED-caused seq rows, which is q1's `src_flips`.
   - The differences do not change which rows match: `CROSS JOIN` in place of `JOIN`; unary `+` on `source`, which only steers the planner; and the extra filter "chain holds some DÖE notice", which `doe_before` already implies.
   - Prepends (seq 1) would be counted. There were 0.
2. **Pairing.** In `ted_*.sql`, `seqless_cursor` looks for a row with the same entity, `version_seq IS NULL`, `op = 'changed'`, the same `changed_at`, a cursor above the seq row's, and a cursor ≤ HI. That is the amendment's pairing. In `seqless_*.sql`, `paired_seq_rows` checks the same pairing in the reverse direction. All 102 rows read `(1, seq 2, current_seq 2)`.
3. **Cursor bounds.** `gen.py` uses the chunks (LO, LO+5000] … (…, HI] with LO = 958810037 and HI = 958848408.
   - The `sum_*` row counts add to 5000×7 + 3371 = 38,371 = HI − LO, so the range is dense.
   - Every row has `changed_at` 1792395355 (07:35:55Z) and `COUNT(DISTINCT changed_at)` is 1 per chunk.
   - `edge_lo`: cursor 958810037 is the last row of an earlier writer, at 05:53:39Z.
   - `edge_hi`: nothing exists after HI.
   - No query hit its LIMIT (seqless ≤ 19 against 1000, ted ≤ 123 against 2000, sum ≤ 12 against 50). No response was truncated.
4. **Plans.** All 26 statements are byte-identical to the statements in `plan_statements.sql` / `plan_edges.sql`, and `plans.out` / `plans_edges.out` contain 0 `SCAN`.
5. **Totals from `sum_*`.**
   - Seq-less tender `changed`: 15+19+10+15+15+12+0+16 = **102**.
   - Seq-less lot `changed`: 30+27+11+30+15+18+25 = **156**.
   - The seq-less tender `removed` (11) and lot `removed` (29) rows are the regroup sweep. They are not `changed`, so they are out of scope.

## Confirmation reads (2 of the 3 allowed; files in `verify/`)

I read both plans locally with `plan-probe plan` on a copy of v551/plan.db; the output is in `verify/plans_v.out`.
- **v1:** a 102-arm `UNION ALL`. Each arm does `SEARCH c USING INTEGER PRIMARY KEY` and `SEARCH l USING INTEGER PRIMARY KEY`.
- **v2:** `SEARCH n` on the PK range, then `tender_versions_notice`, then the t PK, then `tender_versions_2` and the n2 PK, then `changes_entity`. Its only `SCAN` is `SCAN w`, the derived table built from those seeks.

Both responses were valid JSON with `row_count` and `columns`, `truncated:false`, no error and no 408. They took 1.7 s and 2.1 s.

Earlier I also made one stray `/root/sq.sh < /dev/null` call that sent no SQL, and its output was discarded. It read no data, but it counts as one request against the token's hourly budget.

- **v1, the correction sets** (`verify/sql/v1.sql`, `out/v1.out`, `an_v1.py`, `out/v1.result`). For each of the 102 seq-less tender rows X, it read rows X..X+head_lots+1. In all **102 of 102** sets:
  - X is the Tender's rule-T row.
  - It is followed by exactly `head_lots` seq-less lot `changed` rows, and every one of those lots has `lots.tender_id` equal to that Tender.
  - The row after the set is the next Tender's seq row.
  - The lot rows owned this way total **156**, which is all 156 seq-less lot rows in the range.

  So every seq-less row in the fold is accounted for, as B's rule T plus head lots for these 102 Tenders.
- **v2, q1's own version-driven form** (`verify/sql/v2.sql`, `out/v2.out`). It is an independent recount that starts from the notices rather than the change rows. It covers every TED notice ingested on 2026-10-19 with id in (47409900, 47414900]; the observed ids are 47410179..47414207, so neither edge binds. It classifies every version those notices caused, and for each one checks whether the fold wrote a seq row and a seq-less row (changed_at 1792395355, cursor in (LO, HI]).
  - `src_flips` (pre, doe_before, ¬ted_before) = **102 versions on 102 Tenders**, with seq1 = 0. All 102 have the fold's seq row and the fold's seq-less row.
  - Every other bucket has `fold_seqless = 0`:
    - TED-first, pre: 298 versions on 296 Tenders
    - pre with no lower DÖE notice: 1,667
    - co-minted with a lower DÖE notice and no lower TED notice: 1 (nid 47412945, Tender 8851990, a DÖE and TED pair both minted this fold)
    - co-minted with both lower: 1
    - co-minted TED-lowest or TED-only: 292 + 1,645 + 23
  - Every version in every bucket has `fold_seq_row = 1`, so no 10-19 TED notice was folded by a different writer.

## Reconciliation with the job line

The job line reads "4070 written, 357 verified unchanged, 102 relabelled". 4070 + 357 = 4427, the line's tender count.

B counts one announcement per Tender, from three paths:
- **(a)** An ordinary fold moved the label.
- **(b)** The unchanged-chain early return. This writes a seq-less set with no seq row. There are 0 seq-less rows without a paired seq row, so this path wrote nothing.
- **Off/Shadow stale refolds.** Their sets would be extra seq-less rows. There are none.

An `on` refold's label moves go into D3's set under `corrected`. Any D3 set, for a label move or any other diff, would show up as seq-less tender rows beyond the flips. There are none: all 102 seq-less rows are flips paired with their seq-2 row. So D3's `corrected` wrote 0 rows in this range, and every relabelled Tender is one of B's (a) announcements: 102 = 102 = 102.

**Kind moves: 0.**
- No prepends, so the fold-first notice, which issue 462 derives kind from, did not change.
- No seq-less row lacks a source flip.
- All 102 Tenders are now `ted`/`procedure`.

## Caveats (they do not change the verdict)

- **The relabelled = 102 figure was not re-read.** It is the task-supplied text of job 2645. The jobs table is not allow-listed on `/v1/sql`. The journal (07:00-09:00Z) carries only the `group step links` lines. `/root/jobs-now.json` is stale (2026-09-29). My attempt to look at the box's admin helper was refused, so I did not re-read the durable `counts` line.
- **The fold's compare tally was not seen.** The task's line elides it. But D3 wrote no seq-less rows in this range, so `corrected` cannot have absorbed a label move from this fold.
- **The rows were read after the fold.** `tenders` and `tender_versions` were read at about 08:40Z (the measurer's reads) and about 09:00Z (mine). For all 102 flips, seq equals current_seq, and no TED version in v2 lacks the fold's seq row, so no later rewrite is visible.
- **Co-minted held 0 rows today.** So the "fetched before, parsed after" explanation went unexercised. The one co-minted candidate with a lower DÖE notice (8851990) was minted in this same fold, carried no stored label to move, and correctly got no row.
