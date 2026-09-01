# 68 — targeted per-pool cache_size split + fresh-rebuild Phase-1 measurement + union-find O(legacy) RAM

Status: CLOSED — SUPERSEDED 2026-09-01. Both follow-ups rested on premises that
have since changed, measured today. Part 1 is moot (the 128 MiB reduction it
exists to revisit is no longer in effect). Part 2's stated trigger HAS fired and
its conclusion inverted. Neither needs doing; the design note in part 2 is worth
keeping. See "Re-derived 2026-09-01" at the end.
Kind: performance / bounded-memory
Blocked by: —
Relates to: 61 (the incident this fell out of), 57/bounded-memory-principle, 63/66 (rebuild pipeline)
Owner: proj-fix (diagnosis)

## Context

The 2026-07-28 incident (deploy restart → salvage re-nuke + swap-thrash) had TWO
fixes shipped together (commit on issue62-defer-org-indexes): (A) the durable
`rebuild_in_progress` salvage flag, and (B) a global `cache_size` reduction
512 MiB → 128 MiB to bound the aggregate page-cache budget on the 8 GB box. (B)
was deliberately the MINIMAL form. This issue tracks the two deferred, non-blocking
follow-ups so they are not lost.

## 1. Targeted per-pool cache_size split (only if measured to be needed)

`cache_size` is charged PER CONNECTION and the process opens ~23 (writer + store
read pool 8 + API/SQL/webhook pools ~14 + Phase-2 pre-pass K shard readers). The
global 128 MiB bounds the ceiling to ~2.9 GiB and the active set (writer + K≈3
sweepers) to ~0.5 GB — safe. But 128 MiB is a compromise: a from-scratch rebuild's
**Phase-1** (`build_plan`) does ~11 indexed range scans per chunk (notices +
notice_sections + 9 value tables + the random org-mention resolution) and the
512 MiB cache was originally raised (issue 60) specifically to cut its ~1.8× read
amplification. The resume + the daily incremental both SKIP Phase-1, so 128 MiB is
free for our hot paths — but a genuine fresh rebuild might regress.

**Measurement first:** on the next from-scratch `rebuild:true`, compare Phase-1
wall-time / read_bytes at 128 MiB vs the old 512 MiB (or instrument read
amplification). Only if it regresses materially, promote to the split:
- projection writer + `db.readers(k)` pre-pass readers → **256 MiB** (the heavy
  sequential sweepers + the fold writer),
- API / SQL / webhook / store read pools → **64 MiB** (point queries need little).
Aggregate active during a rebuild ≈ writer(256) + K≈3×256 ≈ ~1 GB; idle API pools
20×64 = 1.25 GiB ceiling, ~0 active. Mechanically this means a `cache_size`
parameter on `Readers::open` + the writer open instead of the single `PRAGMAS`
constant.

Do NOT build speculatively — the global 128 MiB is correct until the fresh-rebuild
measurement says otherwise.

## 2. Union-find O(legacy-corpus) RAM in build_plan_groups

Separate, pre-existing bounded-memory tension (flagged during the incident dive,
NOT triggered by it): `build_plan_groups`' legacy-OJS transitive-closure union-find
is ~10.4M nodes / ~1.8 GB RSS — an in-RAM O(legacy-corpus) structure, in tension
with [[bounded-memory-principle]]. It is transient (only during grouping, freed
before Phase-2) and SKIPPED on resume when `plan_notice_fold` exists, so it was
NOT resident during the incident's pre-pass thrash — but on a FRESH full rebuild it
is a ~1.8 GB spike on top of the caches. Under the current 16 GB swap band-aid it
fits; once the band-aid is removed (its tracking memory) this becomes load-bearing.

Fix direction (future): spill the union-find to disk (the grouping already runs in
SQL over on-disk plan tables per issue 59; the union-find is the one part still in
RAM) OR bound it with a disk-backed disjoint-set. Not urgent; capture so the
band-aid removal accounts for it.

## Non-goals

- Neither item blocks the incident recovery or the current build. Both are
  measure-then-maybe-optimize follow-ups.

## Re-derived 2026-09-01 — measured, not argued

This issue is written around bounding memory "on the 8 GB box". Both of its parts
turn out to be settled by changes made elsewhere.

### The box

```
Mem:  62 GiB total, 57 GiB available     (was 8 GB)
Swap: 0                                  (the 16 GB band-aid is GONE)
Cores: 32
```

### Part 1 — moot, because the reduction it revisits is not in effect

Part 1 asks: measure a fresh rebuild's Phase-1 at 128 MiB versus the old 512 MiB,
and only split per-pool if 128 MiB regresses. **There is no 128 MiB to regress
from.** Issue 175 turned the cache into a per-box env valve, and prod is set to the
original value:

```
systemctl show tender-db -p Environment  ->  TENDER_CACHE_KIB=524288   (512 MiB)
```

`cache_pragma`'s own comment already states the reasoning and the number:

> the right cache is a property of the BOX, not the build — 128 MiB per connection
> protects an 8 GB machine, and starves a 64 GB one […] On the 64 GB / 32-core
> prod, 524288 (512 MiB) gives the fold ~16 GiB of active cache with ample
> headroom.

So the compromise this part exists to revisit was undone by a better mechanism
than the one proposed here: one valve instead of a per-pool split. And the split's
purpose — give the heavy sweepers 256 MiB while keeping API pools at 64 MiB — is
already exceeded, since the heavy paths now get 512 MiB. The ceiling concern is
comfortable too: 23 connections × 512 MiB ≈ 11.5 GiB worst case against 62 GiB.

**Nothing to build. Nothing to measure.**

### Part 2 — the trigger fired, and the conclusion inverted

Part 2 said of the ~1.8 GB union-find spike:

> Under the current 16 GB swap band-aid it fits; once the band-aid is removed (its
> tracking memory) this becomes load-bearing.

**The band-aid is removed — swap is 0.** So the stated trigger has fired. But it
does not mean what the sentence expected, because the other variable moved at the
same time: 1.8 GB against 62 GiB of RAM is **2.9%**, transient, and freed before
Phase-2. It is not load-bearing; it is noise.

Recording this explicitly because the sentence as written is a tripwire that would
now read as an alarm: someone checking "is the swap band-aid gone?" gets *yes* and
would conclude part 2 is urgent. It is the opposite.

**What survives is a design note, not a task.** An in-RAM O(legacy-corpus)
structure is still in tension with the bounded-memory principle regardless of how
much headroom the box happens to have, and if the corpus grows an order of
magnitude the spill-to-disk direction sketched above is the right one. Kept as a
note against that day; not open work.

### Method note

Neither conclusion needed a rebuild or a benchmark — both parts were decided by
reading what the code and the unit already say against what the box actually is.
The issue had been open on stale numbers for five weeks.
