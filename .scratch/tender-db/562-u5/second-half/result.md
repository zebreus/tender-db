# 562 unit 5, second half: paired read of the 2026-10-20 daily fold (project job 2669)

**Verdict: PASS.**
- flips 57 = announced 57 = relabelled 57
- TED-first rows: 0
- co-minted rows: 0 (one co-minted Tender exists; it correctly got no row, explained below)
- kind moves: 0

The read used the first half's method and SQL shape (`../gen.py`, `../analyze.py`). It added one step at the front: finding the project's own sub-range, because this chain's cursor range also holds the D5 rows written by rederive-eur-recent 2668.

It ran on 2026-10-20 between 07:53Z and 08:03Z:
- 41 bounded `/v1/sql` SELECTs, sent one at a time (`out/requests.log`).
- Every response carried `row_count` and `columns`, with `truncated: false`. There were 0 error bodies and 0 408s, and each took 2.2-5.9 s.
- One extra transport failure: on the first attempt at `seqless_3`, ssh exited 255 ("Connection closed by UNKNOWN port 65535"). That is not an endpoint answer and not a 408. The box answered a plain `echo` straight away, and the statement was sent once more. If the failed attempt did reach the endpoint, the total is 42, within the budget of 60.

## The project's sub-range

The chain's range is (958848784, 958904203]: 55,419 rows, read as 12 chunks of at most 5,000 by `sum_*`, grouped by `changed_at`, kind, seq-less and op. The rows add up to 55,419, so the range is dense.

| writer | cursors | rows | changed_at |
| --- | --- | --- | --- |
| rederive-eur-recent 2668 (D5) | 958848785 … 958853608 | 4,824 = 913 seq-less tender `changed` + 3,911 seq-less lot `changed` | 1792482078 … 1792482086, one `now` per window |
| **project 2669** | **958853609 … 958904203** | **50,595** | **1792482087** for every row |
| reveal-recheck 2670 | none | 0 | — |

- **D5:** its 4,824 rows match the job line's "4824 correction row(s)" exactly. Its 913 distinct Tenders match "913 tender(s) stamped epoch-stale".
- **Project:** its first row is `organization added` at 958853609 (`edge_mid`). The fold's `now` is one second after the job started at 1792482086, and every row in its sub-range carries it.
- **reveal-recheck:** it wrote no change rows. `edge_hi` shows nothing after 958904203, which is the project's last row (`contract added`).
- `edge_lo`: cursor 958848784 is the last row of an earlier writer (organization `changed` rows at 1792410397).

The paired read therefore ran over **(PLO, PHI] = (958853608, 958904203]** only: 11 chunks × 2 queries (`seqless_*`, `ted_*`).

## Plans

Every statement's plan was read locally with `target/debug/plan-probe plan` (turso) on a copy of scratchpad/v551/plan.db (scratchpad/u5b/plan.db). The plan files are `plans_p1.out`, `plans_edge.out`, `plans_p2.out` and `plans_x.out`.
- 344 plan lines, 0 `SCAN`.
- All 41 statements sent are byte-identical to the planned ones.

| statement | plan |
| --- | --- |
| `sum` | rowid range on c, then sorters |
| `seqless` | rowid range on c, PK seek on t, `changes_entity (entity_kind=? AND entity_id=?)` for both pairing subqueries and for the new `d5_rows` subquery, `sqlite_autoindex_tender_version_lots_1 (tender_id=? AND seq=?)` |
| `ted` | as in the first half (rowid range on c, `tender_versions_2 (tender_id=?)` on v/v2/v3, PK seeks on n/n2/n3/t, `changes_entity`), plus `changes_entity` for `d5_rows` |
| `x_*` | PK and `changes_entity` seeks |

## One change from the first half: telling D3 from B

The fold refolded the 913 stale Tenders under compare On. The job line reads "compare: 16 verified, 897 corrected; 4609 correction rows". Those 897 corrected Tenders get ADR-0017 D3's acting set, which is a seq-less tender `changed` plus lots. That set counts under `corrected`, never under `relabelled`.

So both `seqless` and `ted` carry `d5_rows`: the number of seq-less tender rows that Tender has in D5's sub-range (LO, PLO]. A count above 0 means the Tender is one of the 913 stamped stale. A Tender with a row at 0 is B's (issue 562).

## Result (`result.txt`, from `analyze.py`)

The project sub-range holds 954 seq-less tender `changed` rows on 954 distinct Tenders, all at 1792482087, and 3,813 seq-less lot `changed` rows. The `seqless_*` reads returned all 954.

1. **Flips (q1's notice-id form): 57 rows on 57 Tenders.** A flip is a TED-caused seq row with `created_at < n.ingested_at`, a DÖE notice of lower id, and no TED notice of lower id.
   - All 57 are seq 2 (= current_seq), op `changed`. There are 0 prepends (seq 1).
   - All 57 are now `ted`/`procedure`, and none is one of the D5-stamped stale Tenders.
   - Every flipped Tender was minted by the 10-19 fold (`created_at` 1792395355). Its DÖE notice has nid 47414726..47414807, and its TED twin has nid 47415000..47418535, all ingested at 1792481702 (10-20 07:35:02Z). This is the Monday/Tuesday weekend pattern again.
2. **Announced: 57 of 57.** Each flip has a seq-less tender `changed` row for the same Tender, with the same `changed_at` and a higher cursor, 2 to 15 cursors after the seq row.
3. **Seq-less rows that are not flips: 897, all D3, none B.**
   - **TED-first: 0 rows.** 353 TED-first Tenders were in the fold, and none got a row.
   - **Co-minted: 0 rows.** There is one co-minted Tender with a lower-id DÖE notice and no lower-id TED notice: **8853588**. It correctly got no row.
     - It is an island Tender minted *in this fold*: `created_at` 1792482087, `procedure_key` NULL, `island_notice_id` 47416851, which is its TED notice.
     - v1 is DÖE nid 47414783, ingested 1792395337 (10-19 07:35:37Z, 18 s before the 10-19 fold's `now`). v2 is TED nid 47416851, ingested 1792481702.
     - Its feed is `added` v1 and `changed` v2, with no seq-less row (`x_8853588_*`). It never held a stored label, so B's `moved` is false.
     - The DÖE notice's earlier Tender was most likely **8852015**: one version, `added` by the 10-19 fold at 958848344, and swept in this fold with a seq-less `removed` at 958854910 (`x_removed_tenders`, `x_8852015_changes`). That link is inferred, not proven: the swept Tender's rows are gone, and no indexed path leads from a notice to a deleted Tender.
     - Either way, this is a re-mint, not a label move. A `source=doe` subscriber hears the old Tender `removed`, and a `source=ted` subscriber hears the new one `added`. No ghost remains, and no announcement was owed.
   - **The 897 others are D3's acting set.** Every one sits on a D5-stamped stale Tender, and none is a TED join on a DÖE chain (class "no TED join in this fold"). 889 have no paired seq row (the prefix compared, nothing appended). 8 do have one: versions appended past the prefix, none of them TED-on-DÖE.
4. **Log cross-check (job line: 57 relabelled; 897 corrected; 4609 correction rows).**
   - B's rows: seq-less tender rows on Tenders that were not D5-stamped number **57 = relabelled 57**, and all 57 are the flips. Each pairs with the Tender's seq-2 row, so the unchanged-chain early return wrote 0 rows. Off/Shadow stale refolds wrote none either, because compare is On.
   - D3's rows: seq-less tender rows on D5-stamped Tenders number **897 = corrected 897**.
   - Lots: the 57 flipped Tenders have **101 head lots** at current_seq. D3's lot rows come to 4609 − 897 = 3712. 101 + 3712 = **3,813 = every seq-less lot `changed` row in the sub-range**. So nothing in the fold is left unexplained.
   - **Kind moves: 0.** No prepends (so the fold-first notice did not change), no seq-less row outside the flips and D3, and every B Tender is `procedure`.

## Caveats

- The job-line figures (57 relabelled, 897 corrected, 4609 correction rows, D5's 4824 and 913) are the orchestrator's text. They were not re-read: the jobs table is not on `/v1/sql`, and the admin API was not used.
- `tenders` and `tender_versions` were read at about 08:00Z, after the fold. Every flip's seq equals current_seq, so no later rewrite is visible.
- D3 and B are told apart by D5 membership: a seq-less row in D5's sub-range for the same Tender. That works because the stale set the fold compared (16 + 897 = 913) is exactly D5's 913. No other stale stamp ran in the chain.
