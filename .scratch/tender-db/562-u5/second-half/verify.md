# 562 unit 5, second half (2026-10-20 daily fold, project 2669): adversarial verification

**Verdict: PASS.**
- flips 57 = announced 57 = relabelled 57
- TED-first rows: 0
- co-minted rows: 0
- kind moves: 0
- Every seq-less tender row in the project's sub-range is accounted for: 954 = 57 from B (issue 562) + 897 from ADR-0017 D3 (`corrected` 897).

As in the first half, the job-line figures are the orchestrator's text and were not re-read.

## 1. Sub-range (independent recount of `out/sum_*`, `out/edge_*`; `verify/check.py` → `verify/check.out`)

- **Whole range.** The 12 `sum_*` chunks add to 55,419 = HI − LO, so (958848784, 958904203] is dense. No group straddles cursor 958853608.
- **D5 rows.** 958848785..958853608 holds 913 seq-less tender `changed` + 3,911 seq-less lot `changed` = 4,824, at changed_at 1792482078..1792482086. This matches job 2668's "4824 correction row(s)".
- **Project rows.** (958853608, 958904203] holds 50,595 rows, every one at changed_at 1792482087. That is inside project 2669's window [1792482086, 1792482458], one second after the job started.
- **The boundary.** `edge_mid` shows D5's last lot row at 958853608 (changed_at …086), then `organization added` at 958853609 (…087).
- **After the project.** `edge_hi` shows the last row at 958904203, which is the task's post-chain cursor, so reveal-recheck 2670 wrote 0 rows.
- **The 1792482086 second.** Rows at …086 are all seq-less tender/lot `changed` below PLO. Since D5's count matches its job line exactly, none of them is a project row.

## 2. Criterion and SQL shape

- **`ted` matches the first half.** Its template is byte-identical to the first half's `TED` (already checked against q1.sql by verify-1019.md) except for two changes:
  - the added `d5_rows` column;
  - the seq-less pairing bound HI renamed PHI, which has the same value here.
- **`seqless` matches the first half.** It equals the first half's `SEQLESS`, with the pairing lower bound set to PLO and the same `d5_rows` column.
- **Flip test.** `analyze.py`'s flip test is q1's notice-id form: `created_at < n.ingested_at`, `doe_before`, `NOT ted_before`, on TED-caused seq rows.
- **Pairing.** The pairing is the amendment's: same entity, `version_seq IS NULL`, `op = 'changed'`, same changed_at, higher cursor, at most PHI.
- **Sent = planned.** All 41 statements in `out/requests.log` (42 attempts, one an ssh 255) are byte-identical to the statements in `plan_*.sql`.
- **Plans.** There are 344 plan lines and 0 `SCAN`.
- **Responses.** Every `out/*.out` is valid JSON with `columns`, `rows`, `row_count` and `truncated:false`, and none has an error key.
- **LIMITs.** No LIMIT bound: sum ≤ 19 rows against 400, seqless < 1000, ted < 2000.
- **Chunk size.** Each chunk covers at most 5,000 change rows. The `x_*` reads are PK or `changes_entity` seeks, plus one 5,000-cursor rowid range.

## 3. Recount from the raw outputs (`verify/check.out`)

- **Seq-less rows.** There are 954 seq-less tender `changed` rows on 954 distinct Tenders, all at 1792482087, which is the `sum` total.
- **Seq-carrying TED rows.** There are 411 TED-caused seq rows on chains that hold a DÖE notice.
- **(1) Flips: 57 rows on 57 Tenders.**
  - All are seq 2 (current_seq). There are 0 prepends.
  - All have d5_rows 0, so none is a D5-stamped Tender.
- **(2) Announced: 57/57.**
  - Every `seqless_cursor` is one of the 954 rows read, with a cursor gap of 2..15.
  - All 57 B rows pair back as (1 paired seq row, min seq 2, current_seq 2).
- **(3) Not-flip rows.**
  - TED-first: 353 rows on 353 Tenders, 0 with a seq-less row.
  - Co-minted: 1 row (8853588, doe_before 1, ted_before 0), with no seq-less row.
  - No TED-caused row on a D5-stamped Tender. No non-flip TED row is paired with a seq-less row.
- **(4) Split by D5 membership.**
  - d5_rows = 0 gives 57 Tenders, exactly the flip set.
  - d5_rows = 1 gives 897 Tenders, which matches `corrected` 897. 889 of them have no paired seq row and 8 have one.
  - **Lots.** The B head lots are 101. **The D3 Tenders' head lots, read from `tender_version_lots`, sum to 3,712.** That equals 4609 − 897, which the measurer took from the job line and did not observe. 101 + 3,712 = 3,813 = every seq-less lot `changed` row in the sub-range.
  - So the job line's "4609 correction rows" equals D3's rule T plus its head lots, read from the data independently.
- **B Tenders' current labels.** All 57 are `ted`/`procedure`.

## 4. Confirmation read (1 of the 3 allowed; `verify/sql/v2.sql`, `verify/plans_v.out`, `verify/out/v2.out`)

This is the first-half verifier's q1 version-driven form, counted from the notice side and independent of the change-row chunks:
- every TED notice with id in (47414700, 47419700] ingested on 2026-10-20;
- each version it caused, classified;
- each version checked for the fold's seq row and seq-less row (changed_at 1792482087, cursor (958853608, 958904203]).

**Plan and response.**
- The plan was read first on a copy of v551/plan.db (scratchpad/u5bv/plan.db). Every step is a seek: n PK, `tender_versions_notice`, t PK, `tender_versions_2`, n2 PK, `changes_entity` ×2. The only `SCAN` is `SCAN w`, the derived table built from those seeks.
- It was sent once and answered in 2 s, with `truncated:false`, no 408 and no error.
- The observed nids run 47414825..47418559 and every `ingested_at` is 1792481702, so the window does not bind.

| pre | doe< | ted< | seq1 | fold seq row | fold seq-less | versions / Tenders | class |
|---|---|---|---|---|---|---|---|
| 1 | 1 | 0 | 0 | 1 | **1** | **57 / 57** | flips: all announced, 0 prepends |
| 1 | 1 | 1 | 0 | 1 | 0 | 353 / 353 | TED-first: none announced |
| 0 | 1 | 0 | 0 | 1 | 0 | 1 / 1 (nid 47416851 = 8853588) | co-minted: no row |
| 1 | 0 | 1 | 0 | 1 | 1 | 9 / 8 | TED-only chains with a seq-less row (see below) |
| 1 | 0 | 1 | 0 | 1 | 0 | 1,719 / 1,606 | TED-only, unchanged label |
| 0 | 0 | 1 | 0 | 1 | 0 | 5 / 5 | co-minted TED-only |
| 0 | 0 | 0 | 1 | 1 | 0 | 1,591 / 1,591 | TED mints |

- **The 8 TED-only Tenders with a seq-less row.** They have no DÖE notice below the TED one, so they are not label moves.
  - All 954 seq-less rows are either the 57 flips or d5_rows > 0, so these 8 must be D5-stamped D3 Tenders.
  - They match the measurer's "8 D3 rows with a paired seq row (versions appended past the prefix)".
- **Coverage.** Every 10-20 TED version has the fold's seq row, so no other writer folded these notices.
- **Agreement.** The notice-side recount gives the same 57 / 353 / 1 as the change-row read.

## 5. Points examined adversarially

- **Literal criterion (4), "seq-less tender rows of the project == relabelled 57".** Read literally it gives 954 ≠ 57. It does not hold as worded because this chain, unlike 10-19's, carried 913 D5-stamped Tenders that the fold refolded under compare On.
  - The issue's own docs/operations.md note, and the D3 objection in the Decision, say an `on` refold's set counts under `corrected`, not `relabelled`.
  - The split by D5 membership is exact on every axis: 57 = flip set = relabelled, 897 = corrected, 16 + 897 = 913 = D5 stamped. The lots match too, 3,712 + 101 = 3,813, with the 3,712 observed independently.
  - So the deviation is a correct refinement, not a stop condition.
- **The co-minted Tender 8853588.** It has 0 rows, so nothing needed explaining. The measurer's re-mint account fits the data: an island minted in this fold, v1 DÖE 47414783 and v2 TED 47416851, and its feed is `added` v1 then `changed` v2. 8852015 was swept by a seq-less `removed` in this fold.
  - The 8852015 link is inferred and labelled as such. It does not bear on the verdict.
  - A re-mint leaves no `source=doe` ghost: the old id is `removed`.
- **The seqless_3 retry.** The first attempt was an ssh transport close (rc 255) about 13 s into the attempt, not a 408 body, so resending it was not a retry of a 408.
  - At worst the endpoint ran it twice: 42 of the measurer's 60.
  - Total including mine: 43.
- **Rows read after the fold.** `tenders` and `tender_versions` were read about 08:00Z, after the fold. Every flip has seq = current_seq, and every 10-20 TED version has the fold's seq row, so no later rewrite is visible.

## Caveats

- **Job-line figures not re-read.** Relabelled 57, corrected 897, 4609, 4824 and 913 come from the task text, because the jobs table is not on `/v1/sql`. The data reproduces every one of them.
- **Kind moves.** Kind moves are 0 by construction: there are no prepends, and B's Tenders are all `procedure`. A kind move on a D5 Tender would sit inside D3's `corrected`, which the decision accepts.
