# 121 — a standing structural gate, the empty-layer hole it closed, and two corrections it forced

Status: phase 0 landed; **phase 1 MEASURED AND PASSED 2026-08-04** — confinement proven, Tier B/C
released against it. Companion tooling: `snapwatch.sh` (live on the box), `thread_cpu.sh`,
`activity.sh` (shared sustained-activity primitive), `negative_amount_triage.sql`, `tierA_with_triage.sh`

## Phase 1 result (2026-08-04, pinned snapshot `tender-db-1785830601.db`)

The confinement claim is **measured, not asserted**. 1,145 samples over 2,367 s, t=0 through
completion, so the fill climb — the interval a spot reading misses and Tier B/C stress hardest — is
covered.

| window | health p95 | hot read p95 | cache_file_delta | refault_delta | majflt_delta |
|---|---|---|---|---|---|
| baseline | 0.6 ms | 22.9 ms | 0 | 0 | 0 |
| during (39 min) | 0.7 ms | 22.7 ms | 0 | 0 | 2 |
| after | 0.7 ms | 20.8 ms | 0 | 0 | 0 |

`workingset_refault_file` — pages evicted and read back — **did not move once** while 455 GB went
through the cache. Gate cgroup peaked at exactly the 512 MiB cap; the live service's `MemoryCurrent`
*rose*. Through the fill climb (`gate_bytes` 97.6M → 161.3M over six samples) `live_file` and
`refault` were unchanged at every sample.

**Latency was the correlate, not the claim.** At ~7% CPU duty the interference was never contention;
it was page-cache eviction, which is what `MemoryMax` exists to prevent. Measuring only p95 would
have answered an easier question — the same easier question the discarded salvage window answered.

**And the `0` is interpretable because the instrument was proven first**: the delta arithmetic was
shown to report non-zero on synthetic data before the run, so a flat reading means "nothing was
evicted", not "the delta never worked".

Tier B/C released on this, with the condition that each re-measures rather than inherits the result —
they stress the fill harder, so `refault = 0` at their weight has to be shown, not assumed.

### All three tiers measured (2026-08-04/05, same pinned snapshot, same warm 1.72 GB working set)

| tier | checks | gate-only wall | refault | majflt | cache_file | gate verdict |
|---|---|---|---|---|---|---|
| 0 | 11 | **0.2 s** | 0 | 0 | 0 | 11/11 ok |
| A | 24 | 1405 s (23 m) | 0 | 2 | 0 | 23/24 (the 51 negative ceilings) |
| B | 12 | **2013 s (34 m)** | **239** | 173 | +1,036,288 | 12/12 ok |
| C | 1 | **361 s (6 m)** | 0 | 2 | 0 | 1/1 ok |

**Corrected 2026-08-05 — the first version of this table was inconsistently measured.** It gave A
as 2369 s and B as 3193 s, which were gate **+ triage**; C was gate-only. The standing timer never
runs the triage (a one-off for #33), so the gate-only figures above are the ones to schedule
against. Split from journal timestamps: A gate 1405 s + triage 977 s; B gate 2013 s + triage 1195 s.

Found by proj-fix, who noticed A reported 28m50s in an earlier gate-only run and 39 min here and
asked which conditions to plan against. **Presenting a comparison where two of four members carried
a workload the others did not is worse than a wrong number** — it is an unstated difference between
things being compared, which is the defect this file objects to elsewhere. The ordering survives
(B > A > C > 0) so the cadence inversion below still holds, but B is a 34-minute job, not 53.

Residual, unexplained: the same Tier A gate measured 1730 s earlier and 1405 s here, ~23 % apart,
different times of day and host cache states. **Plan against the higher figure** — undersizing a
window is the expensive direction.

**The condition earned itself: Tier A's zero did NOT transfer.** Tier B moved `refault` — 239 pages
across 4 of 1,544 samples, 222 of them in a ~10-second burst, ≈1 MB against a 1.72 GB working set
(0.05%), while the service's cache *grew* and latency was flat-or-better. Had B inherited A's result
the report would have said "confinement holds" on the one signal that tests the claim.

**And the tier ORDERING was wrong.** These were split by assumed cost — C, the ~30M
`organization_mentions` anti-join, was assumed heaviest, which is why the original proposal was "A
daily, C weekly". Measured, **C is the cheapest tier at 6 minutes and B is the most expensive at 53**:
one index-assisted anti-join is far cheaper than twelve joins or twenty-four table scans. The tiers
were classified by *row count of the largest table touched* rather than by work done, and nothing
checked that.

*This is issue 117's taxonomy defect in a second place, on the same day.* The buckets were fine; what
they were **ordered by** was never measured, and every tier still landed in a bucket, so nothing looked
wrong. **A cadence derived from that ordering would have run the cheap tier weekly and the expensive
one daily.**

Cadence is therefore an open decision to be made from these numbers, not from the original split.

**Mechanism, honestly unsettled.** C's zero shows `refault` does not scale with tier weight — C spans
the largest table and evicted nothing. But C ran 361 s against B's 3193 s, **nine times less
exposure**, so it refutes *proportional-to-weight* and does **not** refute *occasional-burst-over-long-
exposure*. Settling it needs a long run at C's weight or a second long run at B's; neither exists.

**Guard status:** the refault tripwire (>50 pages/s × 5 consecutive samples, calibrated above B's
observed burst) was armed for C and stayed silent — correctly, since refault never moved. Its firing
is proven synthetically and by a wiring test, **not** by a real run, and today's silence is not
evidence it works.
Kind: verification (standing gate) + two corrections to load-bearing beliefs
Owner: sdk-vendor (gate) + proj-fix (daily-cycle integration)
Relates to: 107 (freshness witness — partly consumed here), 109 (presence gate — partly consumed here),
119 (corrected below), 111, task #20 (the prenuke), task #27 (its invariant is a check here)

## Why it exists

The prenuke (88 GB pre-incident canonical-layer backup) has residual value ONLY because we do not
continuously verify the tender layer: a latent salvage re-nuke could corrupt the live layer, and the
2-deep snapshot ring would faithfully copy the corruption before anyone noticed. The sound fix is to
verify in the **present**, continuously, rather than resurrect a week-old unreproducible verdict.

`canonical-verify/standing_gate.sh` — 37 count-free structural checks, stock sqlite3, snapshot-side,
read-only, in three cost tiers (A single-table, B joins/anti-joins, C the ~30M `organization_mentions`
anti-join) so the daily prod-box cost is bounded by construction.

Count-free is the point: corpus-independent expectations cannot go stale as the corpus grows, which is
exactly how `run_light.sh`'s four pinned totals rotted (1.1/1.2/1.3/1.8 — 6961311 / 640745 / 6320566,
taken seven days before the reclaims). Those are deliberately not carried over.

## The hole the self-test found — the part worth remembering

**"Zero rows violate X" is trivially true of an EMPTY table.** A pure violation-count suite therefore
reports a **nuked layer as green** — the precise catastrophe the prenuke exists to survive.

Verified rather than argued: on an empty `tenders`, both `identity_overlap` and `no_head` return `0`.

Closed with `present_*` checks that stay count-free — they assert existence, never a pinned total, so
they cannot go stale either.

**And closed only PARTLY, at first.** The initial set covered the spine plus `lot_results` and mentions
— not the satellites the orphan checks read. run-driver's sweep bed is exactly that shape (`tenders`
populated, every satellite empty), and against it `orphan_texts` / `orphan_amounts` / `orphan_parties` /
`orphan_winner_orgs` all return 0 and the gate goes green on absence: the same hole, one level down,
after I had announced it fixed. The rule now enforced is **no check may depend on a table whose
non-emptiness is unasserted** — one `present_*` per table any check reads (eleven), via `EXISTS` (O(1)),
so there is no cost argument for a partial set. This is the same family as **109** (every gate counts rows,
and shells have rows): 109 is "a row exists but is empty of content"; this is "no row exists at all".
Both are silent-green failure modes of count-based gates. Any future "EXPECT 0 violations" check inherits
this hole by default and needs a presence counterpart.

## Anti-decorative discipline (README rule 2 / commit `0bae958`)

Every check carries its own `poison`: the minimal edit that must make it fire. `--self-test` asserts each
check reports 0 on a clean fixture and >0 on its own poison — 37 clean + 37 poisoned, all passing. A
check with no poison is reported **UNEXERCISED**, not silently trusted.

The runner's own failure modes are exercised in isolation, because that is where a gate rots quietest:

| mode | behaviour | why it matters |
|---|---|---|
| snapshot 40 h old (max 30 h) | refuses, exit 2, `VERDICT stale_input` | verifying a stale snapshot green is the failure (107) |
| unbound schema (dropped table) | `ERROR`, exit 1 — never a silent `0` | an errored query reading as "0 violations" is how a broken detector hides |
| snapshot resolved by newest, not pinned | runs, but records `mode=newest` | age is a BOUND, not an IDENTITY: a failed snapshot step leaves yesterday's file newest and inside the age bound (proj-fix) |
| clean fresh input | 37/37, exit 0, one `VERDICT` line | machine-readable for journal/dashboard |

**Stated limit:** the self-test proves detector *logic* against a minimal hand-built schema. It does NOT
prove the SQL binds to the real schema — only a real run does, and as of phase 0 that has not happened.

## Correction 1 — issue 119 is wrong on its central claim (there IS a producer)

119 says the snapshot section A depends on has no producer: *"no cron, timer or `/admin` route"*. That
search was in the right places and still missed it, because **the producer is a supervisor job, not a
timer**: `supervisor.rs:enqueue_daily()` ends with `Spec::Snapshot`, deliberately last in the daily
pipeline so it captures the freshly folded layer. The Aug 3 09:04 and Aug 4 08:18 snapshots in
`/data/db/snapshots/` are its output. Ring depth is `TENDER_SNAPSHOT_KEEP`, default 2.

119's *worry* survives in sharpened form, and the sharpening changes the fix. The producer is real but
**conditional on the daily cycle running**: if ingestion wedges or the daily does not fire, the file
simply stops refreshing and its mtime is the only tell. So the correct fix is not "build a producer" but
either (a) make every consumer detect the producer's silence — which the gate's age assertion now does —
or (b) decouple snapshotting from the ingest pipeline. Recommend (a) now, (b) only if it recurs.

## Correction 2 — snapshots are ~free, and `prod-disk-constraint` memory is wrong on a load-bearing line

The memory states: *"No snapshot can be made in place — a copy needs another ~453 GB."* **False on this
filesystem.**

`/data` is XFS with `reflink=1`, and `filefrag -v` on the Aug 4 snapshot reports **`shared` on every
extent** (199,997 of the first 200,000 lines). `std::fs::copy` → `copy_file_range` → XFS
`remap_file_range`, i.e. a reflink: near-zero space, near-zero I/O, no long writer freeze — which is also
why `frozen_secs` stays small under the writer guard.

Corroborated independently by arithmetic: apparent sizes on `/data` sum to ~2.1 TB (live 455 +
snapshots 847 + baseline-prefold 442 + prenuke 356) on a **1000 GB filesystem with 126 GB free**. Only
shared extents explain that.

Consequences: 119's capacity half dissolves; **reclaim math for the prenuke/baseline (task #20) is
extent-sharing math, not file-size math** — deleting a file whose extents are shared with the live DB
frees far less than its apparent size; and "point the check at a fresh snapshot" is cheap, not
impossible. The memory should be corrected. VACUUM (~2× file) remains genuinely impossible — that half
of the memory stands.

### 2026-08-05 — the prenuke's cost is quoted in two forms and measured in neither

The consequence above was recorded and then not absorbed. Task #20 still described the prenuke as
**88 GB**; the arithmetic four lines up puts it at **356 GB apparent**. Both figures were in active
use today, in an argument that turned on the size. They cannot both be right, and — the point of the
paragraph above — **neither is the reclaimable figure**, because on a reflink filesystem what a
deletion frees is the file's *exclusive* extents, not its apparent size.

So the standing cost of keeping the prenuke is **unmeasured**, while being quoted confidently in two
mutually inconsistent forms. It is also cheaply measurable: `filefrag`/fiemap reports a `SHARED` flag
per extent, which makes exclusive-vs-shared a read-only, metadata-only, bounded question — the free
category, not the gated one. Nobody should argue from the reclaim, in either direction, until that
runs. We may be keeping 356 GB apparent to avoid re-deriving a verdict, and freeing very little by
deleting it; **neither side of that trade has been measured.**

**And #20's blocker has changed shape**, which matters more than the number. "Deletion basis
unreproducible" was true of the ad-hoc 07-28 verdict — its gates were never committed, so only the
results survived. But the properties that verdict named are now re-encoded here, count-free and
committed, and this was checked in the file rather than recalled: `no_head`, `identity_overlap`,
`kind_bad` (tier A), `orphan_versions` and `junk_hub` (tier B). The blocker is therefore no longer
"the basis cannot be reproduced" but "the gate must run, standing, and be green" — an install, not a
research problem. #20 stays parked, deliberately; it should just be parked for the true reason.

Note the shape, because it recurred twice in one day: **a status that mis-describes the shape of the
remaining work**, while stating nothing false, so the work sits waiting for something it no longer
needs. #28's unit files were the same — recorded as awaiting an on-box window when they had never
been written, and authoring never needed a box at all.

## State at 2026-08-05 — what is done, what is not, and what is unproven

**Done and measured.** 28 checks across four tiers, all run confined against a pinned prod
snapshot with the eviction signal sampled throughout. Layer verdicts: tier 0 11/11, A 25/28
(three negative-money findings, all triaged), B 12/12, C 1/1. Every structural and referential
invariant in the suite is clean on the live layer.

**Guards, and what each is proven by** — listed separately because "proven" did too much work
during this build:

| guard | proven to fire | proven not to fire spuriously |
|---|---|---|
| latency abort | synthetic breach, real unit, killed at 4 s not 40 s | six real runs, never tripped |
| refault tripwire | rising values at the production threshold, killed at 10 s not 90 s | Tier B's real 222-refault burst series, correctly silent |
| confinement assertion | stray holder present → refuses, exit 2 | five real runs, all five checks ok |
| freshness / repeat | 40 h input refused; fresh label → `unknown`, repeat → `yes` | daily runs, correct each time |

**Not done, and not mine.** The systemd timer (proj-fix, cost-ordered 0 → C → A → B, all daily,
`FAIL_ON_REPEAT=1` on all four units, alerting on the *transition* rather than the state). Live
catastrophe detection moved to the app as issue 133 — this gate reads a snapshot, so its
detection latency is bounded by the snapshot cadence and no check frequency can tighten it.

**Unproven, stated rather than left implied.**

* *The bound kills a long gate at the bound.* `TimeoutStartSec` is proven applied (the probe's
  own assertion read it from a live unit), and `RuntimeMaxSec` is proven inert on `Type=oneshot`.
  But one test showed a 6 s exit against a 200 s gate with a 20 s bound, unexplained. Not
  load-bearing: the remote `timeout` has done this job throughout and is measured.
* *What caused Tier B run 1's 222-refault burst.* Not proportional to tier weight (C spans the
  largest table, evicted nothing) and not a function of duration (two B runs, same duration,
  222 and 0). So it is not deterministic in the workload — which is a description of a pattern,
  not an identified mechanism, and the distinction is deliberate.

**Phase 3 / the prenuke, unchanged.** N consecutive green days makes this gate a sound
replacement for the *unreproducible 07-28 verdict*, never for the *backup*. Deletion stays parked
at task #20 on its own evidence, and the gate's greenness must not quietly become a deletion
argument.
