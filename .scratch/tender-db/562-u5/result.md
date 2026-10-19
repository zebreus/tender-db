# 562 unit 5: paired read, 2026-10-19 daily fold (project job 2645)

Read on 2026-10-19 around 08:40Z. 26 bounded `/v1/sql` SELECTs: 2 edge reads plus 8 chunks x 3 queries. Every response carried `row_count` and `columns`, `truncated` was false on all of them, there were 0 errors and 0 408s, and each took 1.5-1.7 s.

## Range

The cursor range is (958810037, 958848408], 38,371 change rows, read in chunks of at most 5,000 rows (`gen.py`).
- `edge_lo`: cursor 958810037 and the rows below it are an earlier writer's seq-less rows at 05:53:39Z. The range starts at 958810038 with `organization added` at 1792395355 (07:35:55Z).
- `edge_hi`: no rows after 958848408 at read time.
- `sum_*`: every one of the 38,371 rows has the same `changed_at`, 1792395355 (the fold's `now`).

Totals over the range: tender seq rows added 2,023 and changed 2,648; tender seq-less changed **102** and removed 11; lot seq-less changed **156** and removed 29; organization added 1,109; plus bid, contract, lot and lot_result rows.

## Plans

The plans were read locally with `target/debug/plan-probe plan` (turso) on a copy of scratchpad/v551/plan.db. See `plans.out` and `plans_edges.out`; there is 0 `SCAN` in 216 plan lines.
- `sum`: `SEARCH c USING INTEGER PRIMARY KEY`, with a sorter for the GROUP BY (`GROUP BY +c.entity_kind`).
- `seqless`: rowid range on c, PK seek on t, `changes_entity (entity_kind=? AND entity_id=?)` for the paired subqueries, and `sqlite_autoindex_tender_version_lots_1 (tender_id=? AND seq=?)`.
- `ted`: rowid range on c, `sqlite_autoindex_tender_versions_2 (tender_id=?)` on v, v2 and v3, PK seeks on n, n2, n3 and t, and `changes_entity` for the paired seq-less lookup. `+n.source` and `+n2.source` keep `notices_member_identity` from driving the plan.

## Result (`result.txt`, `analyze.py`)

1. **Flips:** 102 rows on 102 distinct Tenders. A flip is q1's notice-id form: a TED-caused seq row, `created_at < n.ingested_at`, a DÖE notice of lower id, and no TED notice of lower id.
   - All are seq 2 (= current_seq) with op `changed`.
   - Prepends (seq 1): 0.
   - Every Tender was created on 10-18, and every TED notice was ingested on 10-19.
   - TED nids run 47410187..47414207; the DÖE first nids run 47409988..47410170.
2. **Flips with a seq-less tender `changed` row:** all 102 of 102 have a row for the same entity, with the same `changed_at` and a higher cursor. The cursor gap is 2..15.
3. **Seq-less tender rows that are not flips: 0** out of 102.
   - TED-first: 0. There are 305 TED-first Tenders in the fold and none carries a row.
   - Co-minted: 0 rows. Two co-minted Tenders with a lower-id DÖE notice exist, and neither carries a row, correctly:
     - 8851169 was re-minted in this fold (TED nid 27700183 from 08-10 over DÖE nid 27642151).
     - 8851990 is a DÖE (47409990) and TED (47412945) pair, both in this fold.
   - A further 293 co-minted Tenders whose TED notice has the lower id carry no row either.
   - Because co-minted holds 0 rows, the "TED notice fetched before, parsed after" explanation was not needed this time.
4. **Cross-check:** the job log's relabelled count is 102, and the read finds 102 seq-less tender rows, all 102 of them flips. They pair 1:1, and there are 0 seq-less rows without a paired seq row, so the unchanged-chain early return wrote nothing.
   - Head lots of those 102 Tenders at current_seq: 156, which equals the 156 seq-less lot `changed` rows.
   - Kind moves: 0. No seq-less row is unexplained by a source flip, there were no prepends (so no fold-first notice changed, and issue 462 derives kind from it), and all 102 Tenders are `ted`/`procedure`.

## Caveats

- The relabelled = 102 figure comes from the job line the orchestrator supplied (job 2645). It was not re-read here: the bounded journal window 07:00-09:00Z holds only the `group step links relabel` lines, not the durable counts line (`out/journal_relabel_0700_0900Z.out`), and the admin job API was not used.
- `tender_versions` and `tenders` were read as of 08:40Z, not as of the fold. A later fold rewriting these chains could shift a version's notice. None was observed: every flip's seq equals current_seq.
