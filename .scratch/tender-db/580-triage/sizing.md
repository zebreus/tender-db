# 580 unit 1: how often a fold retires a Tender id (sized from local change-log archives)

Built 2026-10-24. Inputs are local archives of production `changes` rows only. No statements were run against the box, and there was no HTTP, git or cargo.
Per-archive and per-job counts: `580-triage/per-archive-counts.json`. Scripts: `580-triage/scripts/` (see "Reproduce").

## Headline

| scope | retired Tender ids | rate | evidence |
|---|---|---|---|
| daily chain (5 chains, 10-19 to 10-23) | 11, 14, 12, 13, 11 (61 in total) | **12.2 per chain** (min 11, max 14); 231 per 100k Tenders written; 0.56 % of the ids minted that day | u6-lchanged, u9-days12 |
| 539 run c (refold eForms-DE 2.x, 2740/2741) | 93 | **63 per 100k** re-folded (93 / 147,825); 93 new ids minted, one per retired id | u9 segment w539-c |
| 539 runs a+b+c together | 93 | 9.8 per 100k (93 / 951,573) | w539-a/b/c |
| every campaign fold in the covered windows | 93 | 3.8 per 100k (93 / 2,476,214) | all archives |
| all campaign folds except 2741 (value, field, lot-sum and party drains; 539 a/b) | **0** | 0 of 2,328,389 re-folded; the 95 % upper bound is ≤ 0.13 per 100k | all archives |
| monthly, steady state | about 370 a month from the daily chains (range 335–425), plus 0 to a few hundred from campaigns that move grouping inputs | **order 10^2–10^3 a month** | from the rows above |
| months with a regrouping campaign | 10^4–10^5 | issue 93 alone: **216,450** in one refold (2026-08-02) | historical, issue text |

**No retired id ever reappears.** 154 retired ids were checked across the covered windows up to cursor 959223268. None has any later row of any op, and none was retired twice. The schema backs this up: `tenders.id INTEGER PRIMARY KEY AUTOINCREMENT` (crates/store/src/canonical.rs:110) never re-issues a deleted id.

## Coverage

Contiguous all-kind windows hold one row for every cursor (rows = hi − lo, so there are no gaps): **(958787061, 959223268], 436,207 rows**, 2026-10-19 03:34Z to 2026-10-24 02:34Z. Before that window there are two tender-only reads: 2627's window, and a counted (unarchived) summary of 2617's window. The span (958764334, 958764958] between them is not covered; it held jobs 2618–2626.

| archive | path (under `.scratch/tender-db/`) | cursor range | kinds | rows |
|---|---|---|---|---|
| 568-u8 fresh568 (summary only) | `568-r208-lots/readback-u8/fresh568/changes_summary.json`; `readback-u8/readback_result.json` items (e), (g) | (957854566, 958764334] | tender + lot feeds, counted; raw pages not kept | 46,879 tender + 862,889 lot |
| b5-corpus | `551-lot-sum/b5/investigate/corpus_changes.json` | (958764958, 958787061] | tender only (/v1/changes) | 3,429 |
| u6-lchanged | `551-lot-sum/b6/u6/lchanged/changes/chunk_*.json` (44) | (958787061, 959004090] | all | 217,029 |
| u9-days12 | `551-lot-sum/b6/u9/20261023/build_changes_2026-10-23T184801Z.tar.xz` (36 chunks) | (959004090, 959180852] | all | 176,762 |
| u9 seg p0 | `551-lot-sum/b6/u9/segments/p0/read/changes/all/` | (959180852, 959192253] | all | 11,401 |
| u9 seg z539-1 | `551-lot-sum/b6/u9/segments/z539-1/result.json` (K == CA, empty) | (959192253, 959192253] | — | 0 |
| u9 seg w539-a | `551-lot-sum/b6/u9/segments/w539-a/read/changes/all/` | (959192253, 959211183] | all | 18,930 |
| u9 seg w539-b | `551-lot-sum/b6/u9/segments/w539-b/read/changes/all/` | (959211183, 959215522] | all | 4,339 |
| u9 seg w539-c | `551-lot-sum/b6/u9/segments/w539-c/read/changes/all/` | (959215522, 959223268] | all | 7,746 |

Cross-checks are independent copies, and every one agrees:
- **u2 tender feed** (`551-lot-sum/b6/u2/api/changes/000-008.json`, (958787061, 958848784]): 11 removed, the same ids as 2645.
- **U9 replay e1 tender pages** (`b6/u9/replay/e1/`): 24 removed, 5,182 added and 13,010 changed, the same ids as days12.
- **U7 d5 chunks** (`b6/u7/d5/*/changes/`): 0 removed over 2692–2704.
- **`b6/u9/20261023/switchoff.json` `retired`**: the same 24 ids.
- **`b6/u9/registry/R.json` `retired`**: 117 ids, equal to days12 (24) plus w539-c (93).

`b6/u6/box/raw/` and `b6/u6/evidence-u6-pre.tar.xz` hold the raw responses behind the same chunks, so they were not counted again.

## Method

- **Retirement.** A `tender removed` row (always version-null). `retire_chunk_tx` (canonical.rs ~43320) writes it, followed straight away by version-null `removed` rows for that Tender's lots, lot results, bids and contracts at consecutive cursors. Those rows are counted as "removed alongside". Any other `removed` row is listed separately as `other_removed_rows`: versioned round corrections, and a few version-null lots.
- **Minted.** A `tender added` row whose id is above every Tender id seen at an earlier cursor. Ids are AUTOINCREMENT. `tender added` is not the same as minted. Every chain also writes 1–306 `added` rows at `version_seq` 1 on **existing** ids (5 chains: 2, 1, 236, 277, 306; also 54 in 2738 and 95 in 2741). All but one carry a `changed` row from the same job (a rewrite from version 1). Those are not new Tenders.
- **Job attribution.** Each row's `changed_at` is matched to the job ledger: jobs 2629–2741 from `551-lot-sum/b6/u9/jobs/*_ca.json` / `*_cb.json` (`recent`), copied to `scripts/jobs_2629_2741.json`. A fold's `project` stamps every row with the `now` it took at start. A rederive-eur writes per-second stamps holding only version-null `changed` rows. So a stamp group holding only version-null changes goes to the rederive, and any other group goes to the project.

## Per archive and job

The "jobs that wrote it" column gives the writer and its trigger. Every daily chain is probe/process (ted, doe, fts), then fetch-rates, then rederive-eur-recent, then project, then reveal-recheck.

| archive | writer job (trigger) | class | Tenders re-folded/written | tender removed | removed alongside (lot / lot_result / bid / contract) | minted new ids | `added` on existing ids |
|---|---|---|---|---|---|---|---|
| fresh568 | 2617 (refold-fields 2616: 568 u8 + 570) | campaign | 1,250,447 | **0** (0 lot removed of 862,889) | — | 0 | 0 |
| b5-corpus | 2627 (refold-lot-sum 2626: 551 B5 corpus drain) | campaign | 251,526 | **0** | n/a (tender-only) | 0 | 0 |
| u6-lchanged | 2630 (refold-lot-sum 2629) | campaign | 3,502 | 0 | — | 0 | 0 |
| | 2632 (refold-fields 2631, 541) | campaign | 753 | 0 | — | 0 | 0 |
| | 2636 (refold-fields 2635, 571) | campaign | 67 | 0 | — | 0 | 0 |
| | **2645 chain 2026-10-19** (2637–2646) | daily | 4,070 | **11** | 27 / 0 / 0 / 0 | 2,021 (8,850,005–8,852,025) | 2 |
| | 2649, 2658, 2679 (refold-lot-sum 2648/2657/2678) | campaign | 3,502 each | 0 (no rows at all) | — | 0 | 0 |
| | 2651 apply-rehoming, 2652 project (545) | campaign | 48 | 0 | — | 0 | 0 |
| | 2654 refresh-org-names (548) | other | — | 0 (127 organization rows) | — | — | — |
| | **2669 chain 2026-10-20** (2661–2670; doe probe 504) | daily | 4,974 | **14** | 25 / 0 / 0 / 0 | 1,882 (8,852,026–8,853,907) | 1 |
| | 2672/2673 dissolve-guid-twins (530) | other | — | 0 (10,995 **organization** removed) | — | — | — |
| | **2688 chain 2026-10-21** (2680–2689) | daily | 5,887 | **12** | 43 / 26 / **9,864** / 23 | 2,442 (8,853,908–8,856,349) | 236 |
| u9-days12 | 2692/2695/2698/2704 (refold-lot-sum 2691/2694/2697/2703, 551 U7); 2701, 2707 wrote no rows | campaign | 426 + 3,100 + 489 + 33 + 206 + 1 | 0 | — | 0 | 0 |
| | **2716 chain 2026-10-22** (2708–2717) | daily | 5,785 | **13** | 70 / 12 / 12 / 11 | 2,314 (8,856,350–8,858,663) | 277 |
| | **2726 chain 2026-10-23** (2718–2727) | daily | 5,739 | **11** | 11 / 6 / 5 / 4 | 2,285 (8,858,664–8,860,948) | 306 |
| p0 | 2730 (refold-lot-sum 2729) | campaign | 3,537 | 0 | — | 0 | 0 |
| z539-1 | 2731/2732 repair-notice-instants | other | 0 | 0 (empty window) | — | — | — |
| w539-a | 2734 (refold 2733 eforms-sdk-0.1, 539 run a) | campaign | 676,936 | **0** | — | 0 | 0 |
| w539-b | 2738 (refold 2737 eforms-de-1.x, 539 run b) | campaign | 126,812 | **0** | — | 0 | 54 |
| w539-c | 2741 (refold 2740 eforms-de-2.x, 539 run c) | campaign | 147,825 | **93** | 99 / 232 / 222 / 222 | 93 (8,860,949–8,861,041) | 95 |

Totals per all-kind archive (tender removed / minted):
- u6-lchanged: 37 / 6,345
- u9-days12: 24 / 4,599
- p0, w539-a, w539-b: 0 / 0
- w539-c: 93 / 93

The days 1–2 archive holds the 10-22 and 10-23 chains plus the 551 U7 drain projects at its start (10-21 10:27–10:52Z), which retired nothing.

## Observations that bear on unit 1

1. **What retires is a regroup, not a re-fold as such.** Of 2.48M Tenders re-folded by campaigns, only 2741 retired anything. That refold carried 539's new DÖE `published_at`, which re-dates notices so chains re-link. Value, field, lot-sum and party drains retire nothing, and so did the eForms sdk-0.1 and DE-1.x refolds under the same 539 change. A campaign's retirement count is therefore set by whether it moves a grouping input (key scheme, links, dates), not by its size.
2. **2741 retired 93 and minted exactly 93.** Each retired Tender re-formed under a new id, so a one-to-one successor would have been well defined for that run. Daily chains retire about 12 while minting about 2,200, so the change rows alone cannot pair a retired id with its absorber.
3. **Retired ids are not only young ones.** Daily-chain retirements fall in three bands:
   - a recurring 1,166,545–1,219,473 band (3–7 in every chain);
   - 1.74M, and 7.94M–7.97M;
   - recent 8.5M–8.85M.

   One id was retired a day after it was minted: 8852015, minted by 2645 and retired by 2669. 539 run c's 93 are almost all 8.81M–8.85M.
4. **Removed-alongside volume is heavy-tailed.** One retired Tender, 8596072 (10-21 chain 2688), took 9,807 `bid removed` rows with it. 8682567 took 54 more. Every other retired Tender carries at most a few dozen.
5. **No job summary reports retirements.** None of the project `counts` lines for 2645–2741 mentions them. The only records are the `tender removed` rows and the project's stderr (`[project] retire …: N/M Tenders retired`). Any monitor or doc number has to come from `changes`.
6. **Organizations are a different contract.** 530's 10,995 `organization removed` rows (2672/2673) are merges covered by the 308 `merged_into` contract, not Tender retirements.

## Historical points (issue text, not archives)

- **Issue 93** (`issues/93-retire-regrouped-tenders-unbounded-transaction.md`): the eForms-DE 1.x refold of 2026-08-02 retired **216,450** Tenders in one run, "the cohort's island→keyed upgrade". That is about 64 % of its 339,914 touched Tenders, and it is the extreme. Both earlier production runs had reported 0.
- **Issue 278** (`issues/278-...ghost-tenders.md:374-379`):
  - 13,768 `tender removed` at one stamp, the 2026-08-26 backlog fold (job 387/1297) that cleaned up regrouped ghosts;
  - about 10,989 over 2026-08-27 to 09-01 (around cursor 497M);
  - 24,760 from cursor 434M to head at the time.
- **Issue 404** (`issues/404-...duplicate-when-publication-id-derivation-moves.md:869`): repair-member-twins 1625/1626 (2026-09-28) retired 3.
- **Issue 386:599**: a daily fold (2026-09-19) retired one weld (7954583) and minted 3.
- **Issue 244:1084**: closing fold 334, 352,196 Tenders, 0 retired.
- **Issues 58 and 91**: early incremental folds, 0 retired.

## Estimates

- **Per daily chain: about 12** (11–14 over 5 chains). That is 231 per 100k Tenders written, or 0.56 % of the day's minted ids. Evidence: u6-lchanged and u9-days12, jobs 2645, 2669, 2688, 2716 and 2726.
- **Per campaign refold, per 100k Tenders re-folded:**
  - 0 when the refold moves no grouping input (0 of 2.33M; upper bound ≤ 0.13);
  - **about 63** for a date or link-moving refold like 539 run c (w539-c);
  - up to about 64,000 for a key-scheme change (issue 93: 216,450 of 339,914 touched).
- **Monthly order of magnitude:**
  - **10^2–10^3** in a normal month: 12.2 × 30 ≈ 370 from the daily chains, plus tens to hundreds from any grouping-moving refold.
  - **10^4–10^5** in a month that carries a regrouping campaign. August 2026 had two: 216,450 on 08-02, then 13,768 on 08-26 plus about 11k in the following week.
- **Limits:**
  - Only one week of daily chains, at 4.6k–12.5k notices a day, so the rate may move with volume and with link rules (issue 481/482 changes).
  - The window holds only one grouping-moving campaign (run c).
  - 2617's zero comes from a counted summary, since its raw pages were not kept.

## Reproduce

1. Extract the days 1–2 tar into a scratch directory (`tar -xJf .../build_changes_2026-10-23T184801Z.tar.xz`).
2. Point `scripts/spec.json`'s `u9-days12` glob at it.
3. Run the three scripts:

```
python3 -I scripts/tally.py scripts/spec.json > tally.json
python3 -I scripts/retire.py scripts/spec.json scripts/jobs_2629_2741.json > retire.json
python3 -I scripts/build_json.py <dir holding tally.json, retire.json, jobs.json> per-archive-counts.json
```
