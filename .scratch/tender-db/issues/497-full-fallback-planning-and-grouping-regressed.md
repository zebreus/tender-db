# 497 — full-fallback planning grew 90 % since August and grouping 5×, against 4.6 % more notices

Status: ready-for-agent — GROUPING HALF ATTRIBUTED AND BUILT 2026-10-08 (`wf_21089c8e-de5`, adversarially
verified; see "Grouping: attributed" below). Fix built with tests, rides with issue 490's deploy B; acceptance is
the next full fallback's journal showing `keyed/island` ≤ ~90 s and a new `refused-labels` line. PLANNING HALF
attributed and checked (see "Planning: the check"); unit P0 (per-half timers) BUILT 2026-10-08, rides with deploy B; NEXT read its `plan halves` line on the next full fallback, then P1 (shard the producer, chunk-interleaved).
Kind: performance / projection planning (`crates/ingest/src/project.rs`)
Relates to: 58 / 179 (the planning half of the full fallback, deliberately left open), 192, 305 (the
closure cap), 495, 496

## What was measured

Read from the `[project]` journal lines saved in `../495-refold/p2_*.txt`:

- **Plan** (phase 1 of the whole-corpus fallback):
  - about 5,213–5,357 s in August (the first run was 5,927 s);
  - 10,085.0 s in both job 2044 (2026-10-07) and job 2067 (today).
  - That is about +80 min (+90 %), while the notice count grew only about 4.6 %.
- **Grouping:**
  - 312.9 s in job 1616 (2026-09-28);
  - 1,613.7 s in job 1974 (2026-10-04);
  - 1,482 s in job 2044 and 1,468 s in job 2067.
  - That is about 5× in one week. No issue covers it.

Small fixes pay this too. Job 2031 (refold-fields, 191k notices) took 5 h 05 m, because it fell back to
the whole corpus ("legacy closure exceeds cap (622004 > 500000)").

Two oddities:

- The 1,489 per-chunk TRUNCATE checkpoints during job 2067's planning all returned busy and reclaimed
  nothing. The WAL stayed about 2.2 GB, the same as in job 2044. They do nothing as placed.
- The two identical 10,085.0 s plan times are real: the journal shows 2 h 48 m 06 s for each.

## Units

1. **Attribute.**
   - Bisect the grouping jump between job 1616 (09-28) and job 1974 (10-04) against the commits deployed
     in that week. Candidates are the identity and weld work of issues 479, 481 and 486, which add keys.
   - Do the same for planning since August: mention resolution (issue 434's refresh and re-bind
     accounting) and the per-chunk checkpoints.
2. **Fix** what unit 1 names. Expected gain: about 20–90 min per full fallback.
3. **Drop the planning checkpoints**, or move them to a point where they can succeed, if unit 1 confirms
   they never reclaim anything.

## Grouping: attributed (2026-10-08)

The whole jump is one step: the batched keyed/island `group_key` UPDATE in `Db::build_plan_groups`. It went
from 44 s (job 1616) to 1,350 s (job 1974), and has been 1,233–1,267 s since. Every other grouping step stayed
flat (241 s against 239 s).

**No code change caused it.** The CASE text is the same at 84feb9f, 14df8b4, 9e2e80d and HEAD. Two things
combined:

- turso 0.7.2 (and 0.8.1) answers every MISS of `procedure_key [NOT] IN (SELECT procedure_key FROM
  plan_refused_key)` by walking the whole materialised set. So each keyed notice that is not refused,
  about 3.1M on a full plan, pays O(K).
- K = |plan_refused_key| grew from 44 (3 placeholder keys + 41 FTS ocids) to 7,048–7,064 (3 + 6,897–6,912
  FTS + 148–149 UUID hubs). That happened as the FTS population went from about 15k to 314k notices (the 342
  backfill, the 465 re-parse with folds 1813/1814, the 477 top-up) through issue 386's existing gate of two
  or more buyer sets.

**Fit.** Ten small incremental plans from 09-30 and 10-01, all under the same CASE text, give 54–74 ns per
(row × refused key) at K from 570 to 915, and fold 1814 gives 349 µs per row at K = 6,440. Other
contributions:

- 482's hub scans: about 8 s.
- 481's link step: 5 s faster.
- 470/483: at most a few hundred keys.

**Fix (built 2026-10-08).**

- The CASE keeps only `procedure_key` / `island:` / NULL.
- A new `Db::refused_key_labels` loads `plan_refused_key` into a `HashSet` and computes the old arms in
  Rust:
  - `refused:<key>:<buyer>`;
  - `island:<id>` for a buyerless notice;
  - NULL for a legacy chain.
- The labels are point-written BEFORE the uuid-hub labels, so a hub's label still wins.
- New journal line: `group step refused-labels: N notice(s) under K refused key(s), Xs`.

**Tests.**

- `a_refused_key_labels_every_arm_the_batched_case_did` covers the per-buyer, buyerless, refused-legacy-
  chain, kept-legacy-chain, kept-key and keyless arms.
  - It passes on the OLD code too, so the output is equivalent.
  - It fails with the new write disabled (mutation check).
- The hub write order is already pinned at `crates/ingest/tests/project_incremental.rs:3843` (a buyerless hub
  notice keeps `refused:<hub>:#n…`).

**Expected gain.** `group:` drops from about 1,470 s to about 250–330 s, saving about 20 min per full
fallback, and the cost stops growing with every newly refused FTS ocid. The engine behaviour is now recorded
in `docs/research/turso-scale.md`.

**Open, a correctness question for 386 rather than timing.** The refused FTS ocids grew about 7× faster than
the FTS population (41 → 6,440 against about 15k → 314k notices). Some may be over-splits from buyer-id drift
between releases of one ocid rather than real welds. Worth a sample: file it if a read confirms it.

## Planning: attributed (2026-10-08; checked, corrections in "Planning: the check" below)

The regression is a staircase confined to the legacy bands. Comparing 2067 with Aug 14 by notice-id band:

| Band | s per 1M notices, Aug 14 → 2067 | Added |
|---|---|---|
| Text era | 55–76 → 153–227 | +551 s |
| r208/r209 | 308 → 817 | +3,563 s |
| eForms | — | +6 % |
| Tail (mostly corpus growth) | — | +573 s |

Each step lines up with a mass RE-PARSE of the band that slowed:

- **Aug 21:** job 305, the r209 re-parse, in the same binary.
- **Sep 3:** jobs 609/611, under d416104's 24-language text policy. That policy put 2.3–4.4× the text rows on
  each legacy notice.
- **Sep 28:** re-parses 1595/1599/1596.

Job 1616 also carried a one-off +1,000 s from 1.1M issue-434 mention refreshes (its WAL peaked at 23 GB).
Since Sep 28 no band has slowed.

Phase 1 has been producer-bound since issue 175. Every per-chunk TRUNCATE has returned busy on every
pipelined run since Aug 10, because the producer's reader holds a WAL mark. That costs about 0 wall time and
reclaims nothing. The writer-side work (434, 448–470, 458, 481) is hidden behind the producer.

Units:

- **P0. Measure.**
  - Add per-half timers to `build_plan`:
    - producer: `parsed_chunk_on`, decode, and time blocked in `send`;
    - writer: `resolve_mentions`, `insert_plan`, checkpoint, and time blocked in `recv`.
  - Report them on the existing every-16-chunks diag line.
  - This settles read I/O against decode, and today's writer time W.
- **P1. Shard the producer.**
  - K reader threads over count-balanced id stripes, as the 31-way pre-pass already does.
  - A reorder buffer so the writer still sees chunks in id order: byte-identical output.
  - Expected: phase 1 drops from about 10.1k s to roughly max(W, read+decode/K), perhaps 1.5–3k s, which is
    about 2 h per full fallback.
  - Gate with `project_equivalence`, plan-resume and fold-source invariance.
- **P2. Drop the per-chunk TRUNCATE** (it never reclaims). Keep the WAL-size diag. If a WAL ceiling is still
  wanted (issue 63), pause the producers at a size barrier and run one TRUNCATE.
- **P3 (only if P0 shows decode dominates).** A filtered parsed read for phase 1 that skips the 24-language
  description and title rows that `Ident::read` and the mentions never use. Needs an equivalence test.

## Planning: the check (2026-10-08, adversarial pass)

The direction holds. These corrections take precedence over the section above:

- **Only one step is proven.** d416104's 24-language policy, applied by re-parses 609/611, added about
  +208 s per 1M × 7M ≈ +1,450 s. That is about 30 % of the +4,858 s, not most of it.
  - The band slowed +47 %. That tracks issue 304's text BYTES (+52.5 %) more than its row count (2.26×), which
    is weak evidence that the read is I/O-bound rather than decode-bound.
  - Job 1616's +1,000 s is the 434 refresh WAL. An Aug 15 run with a 12.7 GB WAL was +8 % on its own, so a large
    WAL slows the producer by itself.
- **"A re-parse rewrites the band's layout and slows it" is refuted.**
  - Job 302 re-parsed all of r208 and caused no slowdown.
  - Job 305 re-parsed r209 only, and the r208 positions slowed too.
  - Steps 1 and 3, plus about +1.1k s of text-era and tail drift, are UNATTRIBUTED. A global I/O or page-cache
    effect fits as well as anything: the DB grew from 526 to 709 GB, and MemoryHigh=54G has been in place
    since Oct 1.
- **"Busy every chunk ⇒ producer-bound" is not proof.** Job 2044's final checkpoint, taken after the producer
  had finished, was busy too, so another reader sometimes holds the WAL. P0's per-half timers have to settle it.
  Add the producer thread's `/proc/<pid>/task/<tid>/{io,schedstat}`.
- **Phase 1's WAL is reclaimed INSIDE `build_plan_groups`** ("WAL after grouping … 0 MB" is logged before the
  post-grouping TRUNCATE), not by the TRUNCATE at project.rs:1772.
- **P1 as written would stall.** With count-balanced contiguous stripes feeding an in-order writer, the writer
  waits for stripe 0, and the workers fill the reorder window. Use chunk-granular interleaving instead:
  - precompute today's exact 10,000-notice chunk boundaries with one id-only scan;
  - workers take the next chunk index from an atomic counter;
  - the writer consumes in sequence from a window of about 2K chunks.
  - RAM: add K × 512 MiB of turso page cache.
  - Budget 2.5–4k s at K = 8 (the pre-pass shards show shared contention), not 1.5k. Expose K through an env
    valve and measure K = 4, 8 and 16.
  - Tests: make READ_CHUNK and K parameters (today every fixture is one chunk, so K = 1 against K = N proves
    nothing). Add a phase-1 analogue of `sharded_prepass_matches_the_serial_prepass` over the organizations,
    mentions, names, plan_notice (with buyer_guard), legacy keys, OJS and link edges, the ledger, the Report
    tallies and the post-phase-2 layer. Also cover stopping mid-plan and an error or panic in one worker.
- **P2 condition.** Keep one TRUNCATE after the producers are joined. Any WAL-ceiling barrier must park all K
  readers and go through `checkpoint_gated`.
- **P3.** If turso tests `field_id` off the index before the table lookup, the filtered read also saves
  table-page I/O. Check with EXPLAIN. Prefer a deny-list of the heavy translated title and description fields,
  so a newly read field is included by default.
- **Side finding.** `plancapture.conf` (issue 429 step 0) is still enabled on the box, though its own comment
  says "Remove after the 2026-10-04 snapshot diff". Its cost lands on the phase-2 WRITER, the fold's
  bottleneck thread: a mutex and a hash per `Preparing:` event, with the writer re-preparing per row. → issue 496.

## P0 built (2026-10-08)

`build_plan` clocks each half. The producer uses atomics: `read` (`parsed_chunk_on`), `decode`
(`normalise_de1`, `Ident::read`, mentions, plan rows) and `send_wait`. The writer uses durations: `recv_wait`,
`resolve`, `insert` (buyer tokens plus `insert_plan`) and `checkpoint`.

Where the numbers appear:

- The journal prints `[project] plan halves (issue 497): producer read=… decode=… send_wait=… | writer
  recv_wait=… resolve=… insert=… checkpoint=…` once per plan build, before the `plan:` line.
- The every-16-chunks `.diag.log` line carries the running totals.

How to read it:

- A large `send_wait` means phase 1 is writer-bound.
- A large `recv_wait` means it is producer-bound.
- `read` against `decode` decides between P1 and P3.

Test: `the_plan_halves_line_reports_each_halfs_clock`.
