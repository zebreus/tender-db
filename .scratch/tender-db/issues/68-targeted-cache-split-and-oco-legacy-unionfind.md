# 68 — targeted per-pool cache_size split + fresh-rebuild Phase-1 measurement + union-find O(legacy) RAM

Status: open — but its PREMISE IS STALE (2026-09-01): this issue is written
around bounding memory "on the 8 GB box". The box now has 64 GB (measured
2026-09-01: `free -m` reports 64,071 MB total, 57,556 MB available with the
service resident at ~1.0 GB). A cache_size reduction 512 MiB -> 128 MiB to
protect an 8 GB budget is a different trade at 64 GB, so the fix this issue
proposes should be re-derived rather than executed as written.
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
