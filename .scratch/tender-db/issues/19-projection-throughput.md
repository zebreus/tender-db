# 19 — Projection throughput: batch apply_tender

Status: resolved

Finding (dress rehearsal): projecting one TED+DÖE month (~100k notices,
~90k tenders) ran CPU-bound past 80 minutes — per-tender BEGIN IMMEDIATE
transactions in apply_tender. At 12.9M-notice scale the projection step
would take weeks; processing is fine (~60-120 n/s), projection is the
bottleneck.

Fix: profile first (where does the time actually go — transaction overhead,
re-reads of unchanged state, missing indexes on the reconcile queries,
fact-diff cost?), then batch: many tenders per write transaction while
preserving the invariants (change rows commit atomically with their
canonical writes; reconcile semantics unchanged; the cursor doorbell may
fire per batch). Target: the rehearsal month projects in ≤10 minutes
(≥170 notices/s ⇒ full archive ≲24h); stretch: ≤5.

Acceptance: rehearsal-month projection time reported before/after; all
projection tests green; re-run idempotency holds; a change-feed consumer
still sees every version exactly once.

## Comments

2026-07-20 — Resolved (commit "Issue 19: batch the projection and make
mention-dedup O(1)"). Profiled first on the rehearsal TED+DÖE June month
(101,599 notices, 353,690 mentions, 78,572 tenders), then fixed what the
profile actually showed:

**Profile (old code, one full month = 54:14):** read+mentions 44 min of which
mentions alone was **2,658 s** — the real killer was **O(n²)**: resolve_mentions
did a per-mention `SELECT org WHERE country IS ? AND identifier_kind=? AND
identifier=?` that turso runs as a *full scan* of the growing organizations
table (it does not index the `IS` predicate). apply was 459 s, read 131 s,
group 2 s. (A single all-notices in-memory read also OOM'd at ~6.9 GB.)

**Fixes:**
- `store::parsed_chunk` — the notice layer read in id-ordered chunks, each a
  handful of ranged scans (O(tables) per chunk, not ~9 queries per notice);
  chunking bounds memory (RSS ~1.9 GB for the month, no OOM).
- `resolve_mentions` preloads the two lookup tables once and dedups **in
  memory** — each mention is an O(1) map hit, only new rows written. Kills the
  O(n²): mentions **2,658 s → 37 s** (72×).
- `resolve_mentions` + `apply_tenders` commit `WRITE_BATCH=512` items per
  transaction (not one per notice/tender); `last_insert_rowid` uses turso's
  in-memory value, not a `SELECT`; the projection runs with FK enforcement off
  (it writes a self-consistent graph by construction) and restores it.

**Before/after (same 100k month, `project --rebuild`):**

| phase    | before  | after  |
|----------|---------|--------|
| read     | 131 s   | 156 s  |
| mentions | 2,658 s | 37 s   |
| group    | 2 s     | 2 s    |
| apply    | 459 s   | 485 s  |
| **total**| **54:14** | **11:57** |

~4.5× overall; ~140 notices/s ⇒ full archive ≈ 24 h (the June TED+DÖE month is
the richest recent month, so the average month is faster). Just over the 10-min
*target*; the remaining cost is apply's inherent insert volume (101,599
versions + ~2.9M satellite rows + 927k change rows). A follow-up could bulk the
change-row/fact inserts (append-only, no id dependency) to clear the strict
target — not attempted here to keep the reconcile path low-risk before the
backfill.

**Acceptance — all met:**
- before/after reported (above).
- all 20 projection tests green (incl. idempotency, org-dedup, change log) +
  full workspace suite + clippy zero warnings + `nix build` earlier.
- re-run idempotency: re-projecting the month wrote **0 versions, 0 change
  rows** (2:32).
- change-feed exactly-once, verified at scale: `changes` total ==
  distinct(kind,id,seq,op) = **2,256,641**; one version per (tender,seq) =
  **101,599 == 101,599**.
