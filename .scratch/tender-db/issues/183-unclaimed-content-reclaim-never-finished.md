# 183 — `unclaimed-content`: the issue-31 fixes landed but the reclaim never finished, and 2,617 rows are unattributed

Status: RESOLVED (2026-08-12) — reclaim complete and verified (RP and @REASON ledger rows at outstanding 0); remainder attributed on-row; detail readout pending /v1/sql access; 77-row stamping gap split off as issue 193
Kind: reclaim completion + attribution
Blocked by: —
Relates to: 31 (the parser fixes), 40 (ledger), 76 (reprocess mechanism), 137 (measured the bucket unchanged since 2026-08-05)

## Why

The dashboard's still-held `unclaimed-content` is 5,967 (public `/api/dashboard`, 2026-08-10) —
the single largest *actionable* (confirmed-loss) population. It splits:

| sub-bucket | outstanding | state |
|---|---|---|
| text RP co-financing (`%scalar field RP`) | 3,326 | parser fixed 2026-07-21 (issue 31 · 5858159), reclaimed=1,536, then stalled |
| r208 award `%@REASON` | 24 | parser fixed 2026-07-21 (issue 31 · 5858159), reclaimed=0 |
| unattributed remainder | ~2,617 | never sampled, never split |

The ADR-0009 reclaim-all sequence queued jobs for the two big suspected buckets (OC = job [4],
DTD = job [5]) but no job ever covered `unclaimed-content` — so two ledger rows sit at
"resolved 2026-07-21" with live outstanding counts that will never fall on their own. Issue 137's
table shows the bucket byte-identical on 2026-08-05: nothing is draining it.

## What

1. Queue the reprocess (issue 76 mechanism) for `reason='unclaimed-content'` — the fixed RP and
   @REASON rows reclaim; with the issue-87 fix deployed, anything that still fails re-records its
   CURRENT failure, so the pass simultaneously attributes the remainder.
2. Read the post-pass reason/detail split of whatever is left and file/extend issues per named
   cause (the 141–144 pattern).
3. Ledger: the RP and @REASON rows' outstanding should go to ~0; whatever new named populations
   appear get their rows.

Bounded: the bucket is 5,967 members — a small job by the standards of the 1.1M eForms pass.

## Comments

**2026-08-12 afternoon (orchestrator) — the pass ran (job 623, 157 packages, rev a953188) and
the reclaim half is DONE.** Results, verified on the live panel after the trailing fold
(job 624: 3,553 versions; tender layer at 7,932,822):

- **3,352 reclaimed** — the whole fixed backlog: text RP co-financing (ledger row now
  outstanding 0, reclaimed 4,861 lifetime) and r208 award @REASON (outstanding 0,
  reclaimed 24), plus their long tail.
- **unclaimed-content 6,180 → 2,829.** The remainder re-recorded its CURRENT failure on-row
  (issue 87's contract): still-genuinely-unclaimed content under today's parsers. Reading the
  detail-level split of the 2,829 (step 2's attribution table) needs row access — blocked on
  the /v1/sql token with issues 180–182; the reasons are on the rows, waiting.
- **77 records now fail as missing-publication-id but their rows did not relabel** —
  `record_reclaim_attempt`'s member-path addressing misses identity-less text records (same
  family as 139's second act, fail-safe direction). Split off as issue 193 (77 rows, bounded).
- 1,062,584 co-resident text records hit the already-parsed arm (one held record puts its whole
  member file on the walk list); harmless, but it exposed that the new zero-stamp journal line
  fired for every one of them — fixed same day (1761e2a): the warning now fires only on the
  reclaimed path, where a silent zero is issue 139's failure shape.
