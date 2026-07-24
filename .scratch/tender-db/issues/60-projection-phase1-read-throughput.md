# 60 — Projection Phase-1 read throughput is ~30 MB/s (slow)

Status: ready-for-agent
Severity: MEDIUM (makes every full projection multi-hour; complements 58)

Observed 2026-07-24 during the first-ever full projection rebuild on prod
(rev 20767e8): Phase-1 (stream parsed layer + resolve mentions + build plan)
read ~183 GB over ~2h at a steady ~25-31 MB/s. Memory bounded (~1.9GB, no
swap), CPU ~50% — it is I/O-bound, not CPU/memory-bound. 30 MB/s is far below
what the VPS disk should sustain, so the bottleneck is likely the ACCESS
PATTERN, not raw disk bandwidth.

Suspected cause: parsed_chunk / the per-notice parsed re-read fans out into
many small scattered reads across notice_sections + each value table by
id-range, rather than large sequential scans — so it's seek/È-bound and turso
per-statement overhead dominates. (The new code also reads the parsed layer
~twice: Phase-1 + Phase-2 scattered re-reads.)

This is DISTINCT from and COMPLEMENTARY to:
- issue 58 (incremental: don't re-read the WHOLE corpus each daily run) — cuts
  the AMOUNT of work.
- issue 59 (bounded/flat memory) — already done.
Issue 60 is about making the bytes it DOES read stream faster.

Directions to investigate (reproduce-first, measure MB/s):
- Larger READ_CHUNK / batch the per-notice value reads into fewer, larger
  ranged scans; ensure the value-table queries hit an index that yields
  sequential id order.
- Avoid the double read of the parsed layer if Phase-2 can reuse Phase-1's
  stream (memory permitting under 59's bounded design).
- Check turso statement/prepare overhead per chunk; reuse prepared statements.
- Confirm it's not fsync/WAL-checkpoint interleaving stalling reads.

Acceptance: full-corpus Phase-1 read throughput materially higher (target a
few×), measured; projection output unchanged (equivalence tests still pass).

## RESOLUTION 2 (2026-07-24): superlinear plan-build inserts (12.4M scale)

Surfaced on the live 12.4M-notice run (rev 8f0dc74): Phase-1's per-500k interval
was SUPERLINEAR (152→…→551s), disk unsaturated → random-I/O, not throughput. Not
the fold index (that was already deferred to build_plan_groups). The culprit:
`plan_ojs_node` has PRIMARY KEY(key) where key = the encoded OJS number (random
w.r.t. insert order), and Phase-1 did `INSERT OR IGNORE` per legacy notice + per
edge — random-position probes that thrashed once the node table outgrew the
cache. Classic bulk-load-into-randomly-indexed-table trap.

FIX: Phase-1 now appends ONLY sequential rows (plan_notice by its notice_id PK,
plan_ojs_edge by rowid); `plan_ojs_node` is built ONCE, sorted, in
build_plan_groups from `SELECT ojs_self FROM plan_notice WHERE legacy UNION SELECT
b FROM plan_ojs_edge`. Reproduced (`crates/store/tests/plan_bulk_load.rs`, bounded
cache): OLD random-key inserts degrade 3.1× tail/head; NEW append-only stays flat
(1.03×). Grouping output identical (equivalence + 22 tests pass). Net win: the
sorted build does strictly less work than the random inserts it replaces, and
Phase-1 is now flat.

OPEN (measure at prod): the one-time deferred node build + label-propagation is a
bounded grouping-phase cost, but in the DEBUG test it was ~linear-with-high-
constant (71s/200k nodes). Release should be far faster; the group-phase heartbeat
logs it. If prod grouping is too slow at 12.4M, the fallback is an in-memory Rust
union-find over the OJS nodes only (~hundreds of MB, bounded by distinct OJS
numbers, not total notices — the small structure, unlike the 1.9 GB per-notice
plan which stays on disk) — fast, and a modest bounded-RAM compromise on the
flat-memory goal. Flagged for team-lead's call.

## RESOLUTION (2026-07-24, proj-fix)

Reproduce-first confirmed the premise: **turso honors `PRAGMA cache_size`.** A
mechanism repro (`crates/store/tests/cache_size.rs`) scans a table larger than a
tiny cache but smaller than a large one and counts read-syscall bytes
(`/proc/self/io` rchar): tiny (64 KiB) re-reads the table every scan; large
(256 MiB) keeps it resident and does ~zero reads — a >5000× difference. So the
default ~2 MB cache re-reading interior pages it can't hold IS the amplification,
and a large cache fixes it.

FIX SHIPPED: `PRAGMA cache_size = -524288` (512 MiB) added to the store's
connection PRAGMAS (crates/store/src/lib.rs), applied to the writer and every
pooled reader (read.rs) — so it reaches the projection's read path.

MEMORY (per-connection cap, mind the 8 GB box): the cache is a per-connection
*cap* that fills lazily, not a preallocation. Connections: writer(1) +
read_pool(8) + API(8) + SQL(4) + webhooks(~4) ≈ 25. Light/capped queries (API,
webhooks, paginated reads) touch few pages → small caches. Heavy scanners fill
toward the cap. During a **dedicated backfill** the pool reuses ONE reader (LIFO),
so only the writer + one reader go hot ≈ ~1 GB — safe, and 59 keeps the plan
off-heap. The theoretical stacking risk is many concurrent *large* SQL reads
(bounded by SQL_READERS=4 and the SQL result-size/timeout caps). If prod RSS ever
presses, dial to 256 MiB (still 128× the old default).

Local repro proves the mechanism and that the fix is real; the prod-scale
before/after (rchar/DB ≈ 1.8× → ~1×, MB/s multi-×) can only be confirmed at
254 GB — watch run-driver's rchar and throughput after the next projection deploy.
Batches with the 59 deploy (no separate restart).

## ROOT-CAUSE LEAD (2026-07-24, high-value, one-line fix)

The store sets NO `PRAGMA cache_size` (see crates/store/src/lib.rs PRAGMAS =
foreign_keys/busy_timeout/journal_mode/synchronous only). So turso runs on its
tiny DEFAULT page cache (~MBs). On the 254GB prod DB, Phase-1 does ~11 indexed
range scans PER CHUNK (notices + notice_sections + 9 value tables); with a
multi-MB cache the B-tree INTERIOR pages of those 11 tables can't stay resident,
so they evict and re-read from disk every chunk → the observed ~1.8× read
amplification (rchar 449GB＞254GB DB) and the ~4h Phase-1 wall-clock.

FIX (cheap, broad win): add `PRAGMA cache_size = -524288` (512 MiB; negative =
KiB) — or tune 256MiB–2GiB — to the connection PRAGMAS so hot interior/leaf
pages stay cached. Benefits ALL reads, not just the projection. Consider a
LARGER cache just for the projection's connection during a full rebuild.
Also worth testing: `PRAGMA mmap_size` if turso honors it.

Validate: measure Phase-1 read amplification (rchar / DB-bytes-scanned) and
wall-clock before/after on a large scratch DB; expect amplification →~1x and a
multi-× speedup. Ship this with the next projection deploy (batch with issue 59)
so it lands without a separate restart. Watch memory: a 512MiB–2GiB cache adds
to RSS — fits the 8GB box, and issue 59 freed the plan from RAM so there's room.
