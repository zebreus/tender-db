# 247 — the same text-era package re-parses 40× slower on a later run, pread-bound inside the DB

Status: RESOLVED 2026-08-19 — the 40× was real and is explained; three fixes landed (153ms → 2.2s a
notice, and ~1ms for a notice with no mentions). The remaining constraint outlives this issue and is now
issue 248.
Kind: performance regression, re-parse path
Blocked by: —
Relates to: 244 (the campaign that hit it), 100 (the re-parse mechanism), 92 (fold quadratic in chain
length — a different quadratic), 179 (legacy refold pays full corpus), 80 (WAL bound in the reclaim walk)

## The observation

`fetch 186` (the 2010-12 TED monthly: a 2.1 GB tar, 1,012 members, 35,830 text-era notices) re-parsed
**twice today with the same code path** and the same selection:

| run | wall clock | rate |
|-----|-----------|------|
| job 760, ~11:50 | **148 s** | 410 members/min |
| the current run, from 13:08 | 27 min for 256 of 1,012 members | **9.5 members/min** |

That is a factor of ~40. Nothing about the package changed, and the second run is the same binary plus
three parser commits that only touch award-name extraction (the notices in this package produce **no**
award sections at all — verified: 0 `LotResult` across 2,000 sampled notices of the package, so the
extractor returns from its gate).

## What it is not

Ruled out by measurement rather than by reasoning:

- **Not runaway extraction.** 0 `LotResult` sections on this package's notices, so `Emit::award` never
  runs and the parse layer written per notice is the same as before.
- **Not the tar.** `reparse_package` streams through `spawn_record_producer` — one pass, one walker
  thread.
- **Not checkpoint thrash.** `CHECKPOINT_EVERY` is 5,000 notices and the WAL is 39 MB.
- **Not disk pressure.** `/data` is 40 % used, 1 TB free.
- **Not my parser's allocations** (two commits fixed real problems there — a 7× flatten/uppercase cost
  and a quadratic name search — and neither changed this package's rate, which is the clue that led
  here).

## What the box says it is

`perf record` on the hot thread, and `/proc/<pid>/io` sampled over 30 s:

    rchar        +8.1 GB / 30 s   (270 MB/s of read() traffic)
    read_bytes   +2.8 GB / 30 s   (94 MB/s actually from disk)
    write_bytes  +188 KB / 30 s   (almost nothing is being written)
    rchar total   182 GB in ~6 minutes of runtime

The stack under the samples is `pread64 → xfs_file_read_iter → filemap_read → _copy_to_iter` with
`xas_load` alongside — page-cache lookups for a file being pread in small pieces. User-space symbols are
stripped in the release build, but the hot addresses cluster in one region, consistent with the DB
engine's page reader. Memory peaked at 19.3 GB in an 11-minute lifetime.

So: per notice, the run is reading hundreds of megabytes out of the 491 GB database and writing almost
nothing. Something in the re-parse's per-notice work is scanning where it used to seek.

## The suspects, in order

1. **`notice_state`'s lookup** — `WHERE source = ? AND publication_id = ? AND content_hash = ?`. There
   IS a `UNIQUE(source, publication_id, content_hash)` for it to seek, but a plan that stopped using it
   (stale statistics after the morning's 12 package re-parses and their folds, which stamped 2.6 M
   tenders epoch-stale and rewrote ~250 k notices) would look exactly like this.
2. **`clear_parsed`'s deletes** across the six parse-layer tables — cheap only if each has its
   `notice_id` prefix available. Issues 111/112 exist because deferred indexes have no guaranteed
   builder; an index that is missing or unusable here turns every delete into a scan.
3. **Page-cache eviction**: the first run may have had its working set warm from the preceding
   `process`/`project` jobs, and a 491 GB database on a box with far less RAM will re-read what it needs.
   This would make the first run the anomaly rather than the second — worth knowing, because it changes
   the campaign's arithmetic from 11 hours to days.

## The decisive experiment (queued)

Re-parse `fetch 240` — the 2006-06 package that took **72 s** for 20,755 notices earlier today, right
after `fetch 186`'s fast run. If it is now slow too, the cause is global (plan/statistics/index or cache),
not the package. If it is still fast, `fetch 186` has a property the others do not, and the 2.1 GB tar
and its 1,012 members are the place to look.

Deliberately NOT done: killing the running job. `cancel` covers queued jobs only, and the
`TENDER_DROP_JOBS` hatch needs a service-environment change this session could not make. The run is
idempotent and will finish; the queue behind it was cancelled so nothing else is blocked.

## Why it matters beyond this campaign

The text era is 215 packages. At the first run's rate the campaign is ~11 hours of queue time; at this
one's it is over a week, and it holds the queue against the daily ingest tick the whole time. Either
number is worth knowing before committing to the remaining 200 packages — and if suspect 3 is the
answer, then no re-parse campaign of this corpus can be planned from a single warm measurement.


## Eliminated on prod, 2026-08-19 (so the next session need not redo these)

The rate got worse on longer observation — 256 → 448 members in 55 minutes is **3.5 members/min**,
against 410/min on the first pass. Each of the three suspects was then tested directly:

**1. `notice_state`'s identity lookup — NOT it.** Through the reader pool, the exact query the re-parse
uses answers in **1–3 ms** on a fetch-186 notice:

    SELECT id, parse_state FROM notices
     WHERE source='ted' AND publication_id='355567-2010' AND content_hash='…'   → 0.001 s

**2. `clear_parsed`'s per-notice work — NOT it.** Every parse-layer table answers a `notice_id` lookup in
about a millisecond, and the row counts are tiny (sections 2, texts 5, codes 10, classifications 1,
amounts 0, dates 2):

    notice_sections 0.0010s · notice_texts 0.0010s · notice_codes 0.0012s
    notice_classifications 0.0010s · notice_amounts 0.0008s · notice_dates 0.0011s

**3. WAL / checkpoint pressure — NOT it.** Sampled every 20 s during the run, the WAL grows ~24 KB/s and
stays small (1.5 → 3.4 MB), so checkpoints are succeeding. That 24 KB/s is also the honest write rate:
about 2 notices a second.

**And the box is not degraded.** Load 1.01 (our process alone), `md3` clean `[UU]` on NVMe, 62 GB RAM
with 57 GB in page cache and 59 GB available. The 270 MB/s of `rchar` against 94 MB/s of `read_bytes` is
therefore mostly cache hits, not a disk problem.

### What that leaves

**~135 MB of page reads per notice** (270 MB/s ÷ 2 notices/s), burning one core, while the same
statements answer in a millisecond each through the reader pool. The difference between the two is the
CONNECTION: `reparse_notice` runs everything on the long-lived WRITER connection inside
`BEGIN IMMEDIATE`, and the millisecond timings above came from the reader pool. A per-connection plan or
prepared-statement difference on the writer would explain a fast reader and a scanning writer, and it
would explain why restarts do not help (each new process re-establishes the same writer).

Next steps, in order:
1. The queued `fetch 240` comparison still decides package-specific vs global.
2. If global: instrument the writer path — time `notice_state` / `clear_parsed` / `insert_parsed`
   individually inside `reparse_notice` behind a flag, and log the per-notice split. Guessing has cost
   two hours; the numbers are cheap to collect.
3. A symbol-preserving build would let `perf` name the hot function — the release binary is stripped and
   the samples only resolve to addresses.


---

## Cause found, by instrumenting instead of guessing (2026-08-19)

Three rounds of reasoning from the code got it wrong three times, so the phases went on
`/metrics` (`tender_db_reparse_phase_seconds_total`, issue 241's counter pattern). One scrape,
555 notices:

    lookup   0.043 s
    clear   85.072 s      <- 99.6%
    insert   0.267 s
    commit   0.080 s

The clear runs nine statements, so it got a per-statement series too. Same scrape:

    organization_mentions      84.71 s     <- 153 ms per notice
    notice_classifications      0.16 s
    notice_ids                  0.05 s
    notice_texts                0.05 s
    tender_version_parties      0.014 s
    tender_version_bid_parties  0.008 s
    …everything else            microseconds

**`DELETE FROM organization_mentions WHERE notice_id = ?` is the entire cost of a re-parse.**
The table has 41.78M rows and the statement scans them.

### Why it scans, and why that is not obvious

`organization_mentions` declares `PRIMARY KEY (notice_id, section_id)`, so the predicate is a
PK prefix and should seek. Two measurements say the write does not:

- a `SELECT COUNT(*)` with the identical predicate answers in **0.8–1.1 ms** — reads seek fine;
- the two party deletes in the same clear, which have EXPLICIT single-column indexes on their
  predicate (`tender_version_parties_mention`, added by issue 100 for exactly this DELETE),
  cost 14 ms and 8 ms **in total over 555 notices**.

So: turso's DELETE does not use the implicit composite-PK index, and an explicit index on the
predicate is what the fast statements have. `organization_mentions(notice_id)` is now declared
in `DEFERRED_ORG_INDEXES` (41.78M rows, well under the 240M auto-build cap), and startup
reported it missing and queued the Reindex, as designed.

### The queue deadlock, and the one thing that did not work

`recover()` restores durable job rows in id order, so the crawling re-parse holds position 1 and
the Reindex that would fix it waits behind. A seek-before-delete was added to break that without
touching the queue — and **it did not help this workload**, which is worth recording: the
2010-era notices DO have a mention each (their `TXT-AU` buyer, folded by this morning's
campaign), so `has_mentions` is true, the DELETE runs, and the scan is paid anyway. The seek
still earns its place for notices with no mentions, but it is not the fix here.

What actually clears this: the queued `reindex` (job 24). The fetch-240 comparison was cancelled
— it was only ever a way to find out whether the cause was global, and the cause is now known.

### Also worth knowing

Why the FIRST pass of this package was fast (148 s) is still unexplained. The most likely story
is that it ran before this morning's folds gave the text era its first mentions at all: with no
mention rows for those notices, `has_mentions` would have been false for every one of them —
which is precisely the case the new seek skips. If that is right, the fast run was fast for the
reason the seek now makes permanent, and the campaign's arithmetic should be based on the SLOW
number until the index is built and measured.


## The queue was the other half of the bug (2026-08-19)

The index fix was committed and the box promptly demonstrated why that was not enough. Sequence, from
the journal:

1. `ensure_deferred_indexes` noticed `organization_mentions_notice` missing and queued a Reindex — at
   the **back**, behind the crawling re-parse it would have made a hundred times faster.
2. Every later boot found that row and logged *"a reindex is already queued"*, then left it exactly
   where it was.

So `push_front` was added for this one caller, and then the early-return had to go too: **already queued
is not queued FIRST**. The bootstrap now moves a queued Reindex to the head, reports whether the move
was needed, and wakes the worker. Verified on prod, in the log line the previous deploy had been
producing:

    supervisor: 1 deferred index(es) missing (organization_mentions_notice);
                a reindex was already queued — moved to the front

    CURRENT 24 reindex auto: organization_mentions_notice
    QUEUED  1:reparse

Ordinary work keeps its ordering — an operator's sequence is a sequence — and `push_front` has exactly
one caller for that reason. The priority is in-memory only: a restart rebuilds the queue from durable
rows in id order and the bootstrap re-applies it on every boot, which is simpler than persisting a
second ordering that could disagree with the first.

### Acceptance, still to confirm

The index is building now. The number that closes this issue is the re-parse's clear time after it
lands: `tender_db_reparse_clear_statement_seconds_total{stmt="organization_mentions"}` should fall from
153 ms a notice to the microseconds every other statement in that clear costs, and the package rate
should return to the ~400 members/min of the first pass.


## After the index: 4× better, and a tail that is not explained (2026-08-19)

`reindex` (job 787) built `organization_mentions_notice`, and the re-parse ran on with it. Two scrapes a
minute apart, both after the build:

    798 notices, clear 31.5 s      → 39 ms a notice on average
    804 notices, clear 93.3 s      → the LAST SIX notices took 61.8 s, about 10 s each

So the index moved the base cost from 153 ms to about 39 ms, and there is a residual tail of notices
costing ~10 s each. That is a different shape from the flat 153 ms this issue opened with: it looks like
per-notice work proportional to something that varies — most likely the number of mention rows the notice
has, each delete of which is checked against `tender_version_parties` / `tender_version_bid_parties` by
their `(mention_notice_id, mention_section_id)` foreign keys.

**Next measurement** (cheap, and it decides the fix): count mentions per notice for this package and see
whether the expensive notices are the many-mention ones. If they are, the FK check is the remaining cost
and the fix is an index matching the FK pair rather than its first column — the same lesson one level
down.

## The operational half is closed

`DELETE /admin/jobs/{id}` now stops a RUNNING job, cooperatively, and it was used to stop this one:

    788 reparse ok | re-parsed 864 notices across 1 packages (1012 members walked, 0 unmatched,
        0 now failing and left untouched); stamped 2601443 tender(s) epoch-stale;
        STOPPED at an operator's request after 0 of 1 package(s)

The queue is empty, the box is idle, and tomorrow's 09:35 tick is not queued behind anything. Before
this, the only ways to stop a job were a service-environment escape hatch and a restart that re-ran it
from the top — which is why this issue cost two firings rather than one.

## Campaign status

Paused deliberately. 12 of 215 packages are re-parsed (and 11 of those under the old naming rule, so they
want redoing once the rate is understood). The arithmetic that matters: at 39 ms a notice the era is about
40 hours of queue time, and at the first pass's rate it was 11. Neither is worth starting until the tail
above is explained — and now that a running job can be stopped, starting it is no longer irreversible.


## Every notice has mentions, so this was never a tail (2026-08-19)

The "residual tail" reading was wrong, and the correction matters. Mentions per notice, sampled:

    fetch 250 (never re-parsed):  1 → 288 notices, 2 → 222, 3 → 27, 4 → 14, 8 → 8
    fetch 186 (re-parsed twice):  0 → 864 notices, 1 → 636

So text-era notices carry one to eight mentions each — the 864 zeros in fetch 186 are exactly the notices
the cancelled run had already cleared, waiting for a fold to re-record them. Every notice pays the mention
delete, and the earlier "39 ms average" came from a package most of whose notices had nothing left to
delete. **This is the whole era, not a tail.**

## Three fixes, each measured on prod

| form | per notice |
|------|-----------|
| prefix DELETE, no index | 153 ms (scanning, but deleting nothing — the notices were already cleared) |
| prefix DELETE, index built, a row actually deleted | **10.2 s** (fetch 240 and fetch 250 alike) |
| + `PRAGMA defer_foreign_keys = ON` (accepted, no error logged) | ~4 s |
| + delete by FULL primary key, one statement per mention row | **2.2 s** |

4.6× from the last change alone, and still 100× short of what the era needs: 3.79M notices at 2.2 s is
2,300 hours. The cost tracks the FK verification against `tender_version_parties` (78,033,566 rows) and
`tender_version_bid_parties`, which reads of exactly that shape — including the full FK pair — serve in
1 ms.

## The next move is operational, not another micro-fix

The code comments already name it: *"the bulk path (`reclaim_only` then one rebuild) spends that window
with the canonical layer under reconstruction anyway"*. If the canonical layer is emptied FIRST, the
notices have no mentions, the seek added earlier skips the DELETE entirely, and the era re-parses at the
~250 notices/s the mechanism is actually capable of. Then one rebuild re-derives everything, including the
new award winners — an operation this project has run before (issue 179 sizes it).

So the campaign plan becomes: empty the canonical layer for the era (or accept a full rebuild), re-parse
all 215 packages with `reclaim_only`, then one rebuild. That is a deliberate, sized operation rather than
215 incremental steps each paying 20,000 seconds of foreign-key proving.

Before committing to it, one cheap check remains: confirm that a notice with ZERO mentions really does
re-parse at the fast rate now (the seek path). fetch 186's 864 cleared notices are exactly that
population, so re-running that package measures it directly.


## Closed, with the confirmation the plan needed (2026-08-19)

Re-running `fetch 186` — the package whose 864 notices had already had their mentions cleared — settles it:

    870 notices in 80 seconds, of which the mention DELETE accounted for 61.3 s across
    the ~28 notices that still had one. The other ~842 cost about a millisecond each.

So a notice with no mentions re-parses at full speed and a notice with mentions costs 2.2 s, which is the
whole story of this issue: the "40× slower on a second pass" was the population changing under the
measurement, not the mechanism degrading.

What landed here, all deployed and measured on prod:

- `organization_mentions_notice`, and the deferred-index bootstrap that now moves its reindex to the FRONT
  of the queue instead of filing it behind the job that needs it;
- per-phase and per-statement re-parse timings on `/metrics`, which is what ended three rounds of wrong
  guesses;
- a seek before the mention delete, so a notice with nothing to delete pays a millisecond;
- deletion by full primary key rather than PK prefix (10.2 s → 2.2 s);
- `PRAGMA defer_foreign_keys` inside the transaction (accepted; worth little on its own, kept because it
  is correct and costs nothing);
- and `DELETE /admin/jobs/{id}` can now stop a RUNNING job, which is what made five experiments in an
  afternoon affordable at all.

The part that does not belong to this issue is the constraint underneath: an in-place re-parse cannot scale
while `tender_version_parties` is full, because the FK proof is not index-served. That is **issue 248**,
with the three options sized — and it, not this issue, is where the campaign's fate is decided.
