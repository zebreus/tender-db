# 183 — `unclaimed-content`: the issue-31 fixes landed but the reclaim never finished, and 2,617 rows are unattributed

Status: ready-for-agent
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
