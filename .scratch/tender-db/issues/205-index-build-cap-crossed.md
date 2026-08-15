# 205 — the 41M index-build cap traps two read-path indexes: a permanent one (slow tender detail) and a boot one (~22-min outage after a rebuild)

Status: needs-triage — filed 2026-08-15 (owner), REWRITTEN same day after tracing the mechanism to
ground truth. HIGH. Two distinct problems share one root (the cap has no build path for an over-cap
index): **P1 is live now — `/v1/tenders/{id}` measures ~4 s**; P2 is a ~22-min unserved boot that
recurs on every restart after a full rebuild.
Kind: performance / availability
Blocked by: —
Relates to: 111 (the auto-build row cap), 62/60 (deferred-index strip/rebuild for fold speed),
82/83 (the multi-hour-boot regression this re-introduces), 61 (serve-before-heavy-work), 57 (the ~4 GB
bounded-memory ceiling the cap protects), 117/120 (the read paths + isolation).

## Correction to the first draft

The first draft claimed all six refused indexes were absent and that country/cpv/buyer/winner reads
were degraded. **That was wrong.** Traced to the code + verified on prod: five of the six are in
`canonical::SCHEMA` as `CREATE INDEX IF NOT EXISTS`, so the first `Db::open` after the rebuild
**rebuilds them** (that is what the ~22 min was) and they end up **present**. Only one is genuinely,
permanently missing. The country=DE read measured 0.023 s (its index is present); the damage is
narrower but sharper than first written.

## The mechanism (verified)

A full `rebuild=true` calls `strip_tender_indexes()` (DROP INDEX for `DEFERRED_TENDER_INDEXES`) before
the fold — dropping the indexes makes the random-key fold fast (issue 60/62) — then
`build_tender_indexes()` after. That builder calls `too_large_to_build` (which is O(1):
`SELECT MAX(rowid)`) and **refuses** any table over `MAX_AUTO_INDEX_ROWS` (41M). Six indexes are over
the cap, so all six are left unbuilt at the rebuild's end. Then two things diverge:

- **Five are ALSO in `canonical::SCHEMA`** (`tender_version_classifications_code`,
  `tender_version_parties_org`, `tender_version_result_winners_org`, `tender_version_bid_parties_org`,
  `organization_mentions_org`). The next `Db::open` runs the schema batch, whose
  `CREATE INDEX IF NOT EXISTS` finds them dropped and **rebuilds all five, blocking, before the HTTP
  listener serves** — ~450M rows of sorted index build = the **~22-minute unserved boot** (16:08→16:30
  on 2026-08-15; `deploy.sh`'s ~22-min health-check patience was exhausted and it reported a false
  `returned 000`). This is exactly the multi-hour-boot trap issues 82/83 removed, re-introduced because
  these five sit in BOTH the schema batch and the deferred-strip set.
- **One is deferred-ONLY** — `tender_version_bid_parties_version` on `tender_version_bid_parties(tender_id, seq)`.
  Its own code comment (canonical.rs:1672) deliberately keeps it out of the schema batch precisely to
  avoid the blocking-boot rebuild. Consequence: nothing ever rebuilds it — the capped builder refuses
  it every time (the supervisor's boot check queued reindex job 698 for it, which refused it and
  reported "ok" having built nothing). So it is **permanently missing**, and
  `tender_detail` reads `tender_version_bid_parties` by `(tender_id, seq)` (read.rs:794) with no index
  and no PK → a full scan of 66.8M rows per detail. **Measured: `/v1/tenders/1` = 4.00 s** (healthy is
  <0.1 s).

So the cap's intent (never block on a giant CREATE INDEX) is defeated two ways: the schema batch
bypasses the cap and blocks at open (P2), while the one index that respects the cap can never be built
at all (P1).

## Current prod state (2026-08-15, rev d93fadf, post-16:08 boot)

- Present (rebuilt at the 16:08 open): the five schema-batch indexes. Reads using them are fast.
- Missing: `tender_version_bid_parties_version`. `/v1/tenders/{id}` ≈ 4 s.
- A **plain deploy now boots fast**: the five are present, so the schema `IF NOT EXISTS` is a no-op,
  and the deferred-only missing one is not in the schema batch. The ~22-min boot only recurs after the
  NEXT full rebuild (which strips the five again).

## Fix

**P1 — build `tender_version_bid_parties_version`.** There is no path to build an over-cap index today;
every builder refuses unconditionally. Needs a deliberate maintenance/force build. Memory is the reason
for the cap (issue 57: CREATE INDEX peak RSS ≈ 45 B/row against a ~4 GB ceiling): this index is
66.8M × 45 B ≈ **3.0 GB peak**, under the ceiling but tight — feasible **alone, in isolation**, in a
low-traffic window, NOT concurrently with another build. Options: (a) a `Reindex{force}` /
admin-named-index build that bypasses the cap for one explicitly-chosen index; (b) build it index-first
at the rebuild (it is append-mostly `(tender_id, seq)`, so index-first is cheap and the fold barely
slows) — this is the clean durable answer for this one.

**P2 — stop the schema batch rebuilding the five blocking at open.** Give the five the same treatment
the author gave `bid_parties_version`: **remove them from `canonical::SCHEMA`** so `Db::open` never
rebuilds them. But then they must be built somewhere — so pair it with either index-first-at-rebuild
(create empty before the fold; the random-key ones cost fold time, measure the trade) or the robust
guarantee below.

**The robust guarantee (covers P2 regardless): serve `/health` before any heavy startup index work.**
Open the listener and answer `/health` (process up + trivial DB ping) FIRST, then run schema-batch
index creation / deferred builds in a BACKGROUND task. Then a restart is never an outage whatever the
index state; reads needing a still-building index are briefly slow (isolation-contained) instead of the
whole service being down. This is issue 61's principle; the post-rebuild path violates it.

## Acceptance

`/v1/tenders/{id}` seeks its bids index (<0.1 s); a full rebuild followed by a restart serves `/health`
in seconds, not ~22 minutes; no read-path index is left permanently unbuildable by the cap.
