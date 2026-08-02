# 96 — incremental apply: per-bucket time is highly variable, and the page cache collapses as the layer fills

Status: proposed — **the original "~13%/bucket monotonic degradation" claim is RETRACTED, see Correction below**
Kind: performance / scaling risk
Design owner: proj-fix
Relates to: 62 + 82 (deferred tender indexes — the existing mitigation, rebuild-only), 64 (reset index thrash), 67 (fold apply statement reduction), 94 (bucketed pre-pass stripe imbalance), 76 (quarantine reprocess mechanism)

## Context

Measured live on prod during the eForms-DE 1.x re-fold (2026-08-02, job 2,
`project rebuild=false`, rev `a55e599`). The bucketed Phase-2 apply folds
**128,004 Tenders** over **473,094 planned notices** in ~10 buckets of ~13,100
Tenders each (`APPLY_NOTICE_BATCH = 50_000`).

**Per-bucket apply time degrades monotonically, ~+50s per bucket (~13%), while
bucket size stays flat.** This is not larger buckets — it is slower work per Tender.

| bucket | window (UTC) | interval | Tenders | rate |
|---|---|---|---|---|
| 2 | 08:04:06 → 08:09:59 | 353s | 13,509 | 2,296/min |
| 3 | 08:09:59 → 08:16:39 | 400s | 13,237 | 1,986/min |
| 4 | 08:16:39 → 08:24:12 | **453s** | 13,090 | **1,734/min** |
| 5 | 08:24:12 → 08:30:11 | **359s** | 13,243 | **2,214/min** |

## CORRECTION — the monotonic-degradation reading was wrong

This issue was originally filed after buckets 2-4 (353s → 400s → 453s) as a clean
+~50s/bucket (~13%) degradation. **Bucket 5 came in at 359s and broke it** — back to
near bucket-2 speed, on the same flat ~13,200-Tender bucket size.

Four intervals: **353, 400, 453, 359** — mean ~391s, range 353-453s (±13%). That is
**variance, not a trend.** Three ascending points were not enough to establish
monotonicity, and the conclusion drawn from them ("self-reinforcing, expect it to
continue") was premature and is retracted.

What survives, and what does not:

- **RETRACTED**: per-bucket apply time degrades monotonically as the layer fills.
  Not supported by four intervals.
- **STANDS (observation)**: the page cache really does collapse during the apply —
  `buff/cache` 5,791 → 2,349 → 1,846 MB while process `used` climbs 1,707 → 3,395 MB.
  This is measured and reproducible.
- **UNPROVEN**: that the cache collapse actually costs throughput. Bucket 5 was fast
  *while the cache continued to shrink*, so the mechanism is not demonstrated by this
  data. It remains plausible but is now a hypothesis, not a finding.
- **OPEN**: what actually drives the ±13% spread between buckets. Most likely candidate
  is per-bucket content (Tenders differ in version-chain length and satellite-row count,
  so equal Tender counts are not equal work) rather than layer size.

The scaling question below is therefore **still worth answering** — a 50+ bucket
reprocess is a real unknown — but it must be answered by measuring a long run, not by
extrapolating this one.

### Mechanism

Two effects that are really one:

1. **Hot-index b-tree maintenance.** This is the INCREMENTAL path, which keeps the
   tender-layer satellite indexes (organization_id / CPV / published_at / notice_id /
   tenders-identity) **live** throughout. Every per-bucket upsert pays full random-position
   b-tree maintenance into indexes that grow with each bucket applied.

2. **Page-cache collapse.** The apply's own RSS grows (bucket rows held for the in-RAM
   sort, index buffers) and squeezes the page cache on a 7.7 GB box against a 449 GB DB:

   - start of apply: `buff/cache` **5,791 MB**, process `used` 1,707 MB
   - four buckets in: `buff/cache` **2,349 MB**, process `used` **3,189 MB**

   Fewer index pages stay resident → progressively more index lookups hit disk → each
   bucket slower. Self-reinforcing as the layer fills.

Available memory stayed at ~4.5 GB throughout (never near pressure) and there were no
errors — **the trade is speed, not stability.**

### Ruled out: external I/O contention

A 188 GB reflink copy ran concurrently during buckets 2-3 and completed ~08:16.
Bucket 4 ran **08:16:39 → 08:24:12, entirely after the copy finished, with the box
otherwise idle (nginx down) — and was the SLOWEST bucket.** If contention were the
driver, bucket 4 should have recovered; it degraded by the same step. The degradation
is intrinsic.

## Why this matters — the reprocess is the real exposure

At 10 buckets the total cost is acceptable and the run completes. **The risk is that it
does not extrapolate.** The quarantine reprocess (issue 76) targets **2.42M notices** —
roughly **50+ buckets**, not 10. If per-bucket time keeps climbing ~13%, the tail buckets
dominate and the run becomes infeasible rather than merely slow. This must be sized
before that work is scheduled, not discovered during it.

Note this compounds with 94 (stripe imbalance): the pre-pass already costs a full-layer
sweep, and the apply then degrades superlinearly on top of it.

## Design lead (proj-fix)

**The full-REBUILD path already solves exactly this and the incremental path does not
inherit it.** `project(_, rebuild=true)` calls `strip_tender_indexes()` before the fold and
rebuilds them once, sorted, at the end — the `DEFERRED_TENDER_INDEXES` work from 62/82.
That is precisely the "don't pay random-position b-tree maintenance during the fold"
mitigation, and it is currently **rebuild-only**.

Two candidate routes, to be chosen on measurement:

- **(a) Route large reprocesses as a full rebuild** so they get deferred indexes for free.
  Simple, uses only existing machinery; cost is that a rebuild resets the whole layer
  (`reset_tender_layer`) rather than touching only the delta — acceptable when the delta
  approaches the corpus, wasteful when it does not.

- **(b) Extend deferred indexing to large incremental applies above a threshold.** Strip
  the satellite indexes when the plan exceeds some size, apply unindexed, rebuild once at
  the end via the existing `Reindex` op (which is already an idempotent
  `CREATE INDEX IF NOT EXISTS` loop and already runs standalone). Keeps the incremental
  path's "touch only the delta" property while removing the degradation.

  The natural threshold to reuse is the one that already routes Phase-2 to `Buckets`
  (`INCREMENTAL_BUCKET_THRESHOLD = 100_000` planned notices) — an apply big enough to
  warrant bucketing is arguably big enough to warrant deferred indexes.

Byte-identity is the hard constraint on either route: gate on `project_fold_source`,
`project_golden`, and `project_resume` as with 62/67.

## Acceptance

- A measured per-bucket curve for a large apply showing time-per-bucket **flat** rather
  than climbing ~13%.
- The 2.42M-notice reprocess sized against the corrected curve before it is scheduled.

## Comments

Filed from live prod measurement during the 2026-08-02 DE-1.x re-fold, at team-lead's
request. Raw data above is from journald `[project] incremental fold: N/M Tenders applied`
heartbeats plus `free -m` sampling; the apply itself completed correctly — this issue is
about scaling, not correctness.
