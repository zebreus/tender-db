# Daily-cycle integration for the standing structural gate (task #28, proj-fix half)

Status: DESIGN — not wired. Wiring gates on sdk-vendor's confinement measurement.
Owner: proj-fix (the cycle integration). The gate script itself is sdk-vendor's (`standing_gate.sh`).
Relates to: 28 (the gate), 107 (snapshot-freshness witness), 119, 23 (snapshots), 111 (reindex op)

## The cycle as it actually is

Read from `Supervisor::enqueue_daily` (`supervisor.rs`), not from memory:

```
09:35 Europe/Berlin  →  probe(ted) → process(ted)      [weekdays only]
                        probe(doe) → process(doe)
                        project {rebuild:false}
                        snapshot                        ← last, so it captures the fold
```

**There is no `reindex` step in the daily.** The cycle has been described in conversation as
"process → project → snapshot → reindex → VERIFY"; the reindex enqueued by issue 111 runs at **boot**,
not on the tick. Recorded because designing VERIFY to sit "after the reindex" would place it after a
step that never runs — and more importantly, because anyone believing reindex runs daily also believes
index completeness is re-checked daily, and it is not. Whether the daily *should* reindex is a separate
question, deliberately not answered here.

VERIFY therefore attaches after `snapshot`, which is already last.

## Why a systemd timer, not a supervisor job

The gate reads an **immutable snapshot**, so it cannot conflict with ingestion or projection for
*correctness* — the file's contents are already decided. The only conflict is I/O and page cache. That
makes "don't overlap ingestion" a **load** constraint, and systemd confines load (`MemoryMax`,
`IOWeight`, `CPUWeight`) better than the app can from inside its own process.

So the supervisor route's strongest claim — free serialisation — buys something obtainable another way,
while charging the failure mode that matters: a verifier that dies with the supervisor cannot report on
a wedged supervisor, which is exactly the condition its staleness check exists to catch. The asymmetry
decides it: `job_log` visibility is recoverable from the timer side (POST the verdict), surviving a
wedged supervisor is not recoverable from the other side.

## The pinned input, and the pointer that has to exist

sdk-vendor's gate now takes `SNAPSHOT=<path>` and carries `mode=pinned` vs `mode=newest` into its
verdict, because "newest" cannot establish *which run* wrote the file: a failed snapshot step leaves
yesterday's file as newest, still inside `MAX_AGE_H`, and the gate would verify it green and report
success for a cycle that produced nothing.

Pinning requires the pipeline to publish the path it just wrote. It does not today — `snapshot::run`
builds `dest`, takes the snapshot, prunes, and returns a summary **string**. Nothing durable names the
file.

**Proposed (my change, app side):** on success only, `snapshot::run` writes the absolute path to
`<dir>/latest` via write-temp-then-`rename` (atomic; a reader never sees a half-written pointer). The
timer reads that file and passes it as `SNAPSHOT=`.

> **Correction (sdk-vendor, 2026-08-04): the pointer does NOT close the hole it was proposed for, and I
> claimed it did.** If today's snapshot step fails, `latest` — written on success only — still names
> *yesterday's* file. So does "newest". Yesterday's file is ~24 h old, **inside `MAX_AGE_H`**, so the
> gate verifies it and reports green for a cycle that produced nothing. That is the exact failure I
> caught, surviving the fix I proposed for it.
>
> The pointer is still worth building, for a narrower reason than I gave: it is trustworthy about
> **what** it names — a verified-complete snapshot rather than whatever file matches a glob, ruling out
> partial writes and strays. It says nothing about **when**. Pointer answers *what*; the hole is a
> *when* question, and needs the mechanism below.

Three properties that make this the right shape:

- **Written only after the snapshot succeeds**, so a failed snapshot never advertises a path — meaning
  the pointer never names a partial file. It does *not* follow that a fresh pointer means a fresh
  snapshot; see freshness, below.
- **No fallback to "newest" if the pointer is missing or stale.** Falling back is precisely the hole
  sdk-vendor just closed; the verifier reports STALE and stops instead of guessing.
- **Prune-safe by construction.** The local ring keeps 2. If a prune deletes the pinned file mid-run,
  an already-open fd keeps it readable to completion on Linux — so the run finishes on the file it
  started, and the *pointer*, not the data, is what may have moved.

## Freshness: repeat detection, not an age threshold

sdk-vendor's mechanism (`d22bb6e`), and the right instrument. The gate records its input's identity
(`name|mtime|size` — no hashing a 455 GB file) and reports `repeat=yes|no|unknown`. On a daily cadence,
**`repeat=yes` means the pipeline produced nothing this cycle** — issue 119's cadence half, detected at
the consumer, with no threshold to tune. Tightening `MAX_AGE_H` toward 24 h was considered and rejected:
it makes correctness depend on tuning against job duration and clock drift, and buys a false negative the
first time a daily runs long.

**The timer sets `FAIL_ON_REPEAT=1`.** In the daily slot, re-reporting a green for an
already-verified input is the lie worth refusing.

Two things this integration must get right, both of which fail *silently* if missed:

- **`StateDirectory=` (or equivalent) is mandatory.** State lives at
  `/var/lib/tender-db/standing_gate.last`. Unwritable state degrades to `repeat=unknown` — correctly, and
  with a printed note, rather than to a clean-looking `no` — but a unit that cannot write state reports
  `unknown` on **every** run, and the whole mechanism quietly does nothing while continuing to print
  verdicts. sdk-vendor names this as the failure they'd most expect us to ship; I agree, so:
- **`repeat=unknown` is acceptable exactly once.** The first run legitimately has no prior state. From
  the second run onward, `unknown` means the state path is broken, and the timer must treat it as an
  alerting condition rather than a benign third value. A mechanism that reports `unknown` forever looks
  exactly like a mechanism that is running.

**What `repeat=no` does and does not prove.** It proves the *input is new*. It does not prove the fold
produced anything: if the projection did nothing but the snapshot step succeeded, the snapshot is still a
new file and `repeat=no` is still correct. The gate verifies the artifact it is given; "the cycle did
useful work" is a different claim and this does not make it.

## Cost: ~29 minutes, and I/O-bound — which decides more than the duration does

Tier A ran **28m50s** wall on the 455 GB snapshot, of which **2m04s was CPU — a ~7% duty cycle**. The run
is **I/O-bound**, not CPU-bound.

> First relayed to me as ~55 min; sdk-vendor caught that they had read elapsed time from when they started
> watching rather than the unit's own `Consumed … wall clock time` accounting, and corrected it before it
> hardened into this design. Halved — and still not "a few minutes", so the split question below stands,
> just less sharply than 55 made it look. **No cadence is hardcoded here** until their clean re-run lands.

**The I/O-bound finding is worth more than the duration**, because it decides three things duration alone
cannot:

- **`IOWeight` is the knob; `CPUWeight` is nearly decorative here.** At 7% duty, throttling CPU constrains
  almost nothing. The interference is disk bandwidth and **page-cache eviction** — pulling a 455 GB file
  through the cache is what evicts the live service's working set, which is exactly why `MemoryMax`
  matters: under it the scan's cache is charged to its own cgroup and reclaimed from there, not from the
  service.
- **It decides what may share a window.** A CPU-heavy job can overlap this cheaply. Another **I/O-bound**
  job cannot — and the daily projection is precisely that (read-bound, issues 62/91). So the rule is not
  "don't overlap heavy jobs", it is **don't overlap I/O-heavy jobs**: narrower, and actually actionable.
- **It narrows the interlock's question** to "is a heavy **I/O** job running?" — projection, reclaim,
  snapshot. Deferring behind a CPU-bound job would be a cost with no benefit.

Four further consequences of the run simply being long (~29 min or more), from the shape rather than the
figure:

1. **"Defer until quiet" stops being a plan and becomes a hope.** Dashing through a quiet moment works
   for a two-minute job. A ~29-minute job needs a half-hour quiet window, and this box wobbles rather than
   drains — there may be no such window on a given day. So the weight shifts off the interlock and onto
   **confinement**: the run must be survivable *during* contention (`IOWeight`, `CPUWeight`, `MemoryMax`),
   not merely scheduled around it. The interlock stays as a courtesy; confinement is what makes it safe.
2. **A cost split implies separate state, or the repeat signal is garbage.** If the presence checks (O(1)
   `EXISTS`) run daily and the full scans run weekly, they are two units — and sdk-vendor's repeat
   detection keys on the *input identity of the last run*. Sharing one state file means the weekly reads
   the daily's input and reports a meaningless `repeat`. **One state file per cadence**, or the mechanism
   silently reports nonsense while looking healthy. Same family as the unwritable-state failure above.
3. **A long run holds an unlinked snapshot open.** The ring keeps 2 and the daily prunes. An open fd keeps
   a pruned file readable to completion (good — the run finishes on the file it started), but the space is
   not reclaimed until the fd closes. Reflink sharing makes the marginal cost small; on a volume with
   ~126 GB free it is still worth naming rather than discovering.
   **Ops constraint (team-lead):** run-driver watches `/data` free space *during* gate runs once this is
   operational, and **maximum hold-time is bounded by reclaim headroom** — the deep health probe trips at
   ~100 GB free, so the margin is real but not large. A longer tier is not automatically safe just because
   a shorter one was.
4. **`FAIL_ON_REPEAT` needs its own state per cadence too**, for the same reason as (2): "no new input
   since the last *weekly*" is a different question from "since the last *daily*", and both are
   answerable — but only against their own history.

## Cadence: the measured costs INVERT the proposed split

All three tiers, one pinned snapshot, warm 1.72 GB working set, directly comparable
(sdk-vendor, `bd95142`):

| tier | checks | wall | refault | verdict |
|---|---|---|---|---|
| A | 24 | 2369 s (39 min) | **0** | 23/24 (the 51 negative ceilings, expected) |
| B | 12 | 3193 s (53 min) | **239** | 12/12 |
| C | 1 | 361 s (6 min) | **0** | 1/1 |

> **Two numbers now exist for tier A and they are not reconciled here.** The earlier clean re-run gave
> **28m50s**; this comparable set gives **2369 s (39 min)** — a ~37 % difference for the same tier on the
> same snapshot. The comparison set is internally consistent, so the *relative* ordering (C ≪ A < B) is
> sound whatever the absolute drift; but the absolute number matters for the window B needs, and I have
> not established which run's conditions to plan against. Raised with sdk-vendor. Recording it rather than
> quietly carrying the more convenient figure — the whole point of the tier costs is to size a window, and
> a 10-minute uncertainty on A is a 10-minute uncertainty on the schedule.

The proposal was *A daily, C weekly*, on the assumption that C — the ~30 M
`organization_mentions` anti-join — was the heavyweight. **It is the cheapest tier by
an order of magnitude**: one index-assisted anti-join beats twelve joins or
twenty-four table scans. The tiers had been ordered by *row count of the largest
table touched* rather than by work done, and nothing checked it. (sdk-vendor's own
diagnosis, and it is issue 117's defect again: the buckets were fine, what they were
*ordered by* was never measured, and every tier still landed in a bucket so nothing
looked wrong.)

**Re-derived, cost-first:** C is nearly free and can run daily or more often. A is
39 min with zero eviction — daily is fine. **B is the hard case**: 53 min, and the
only tier that moved `refault` at all.

**But cost is not the only axis, and the tension should be stated rather than
resolved by arithmetic.** B is the referential-integrity tier — the joins and
grouped anti-joins over the version-keyed tables — which is plausibly the *most*
valuable thing to run often, since it is what catches fold damage. Putting the most
valuable tier on the rarest slot because it is the most expensive is a real
trade-off, not an obvious win.

**On the confined numbers, B is daily-affordable and the tension dissolves.** 34 min,
eviction contained to ~0.05 % of the working set, latency flat-or-better — that is
`MemoryMax` doing exactly the job it was added for. So the most valuable tier does
*not* have to go on the rarest slot, which is the outcome to prefer when the numbers
allow it. **All four tiers daily**, with B still carrying the tripwire since it is
the only one that moves the needle at all.

**One consequence of daily-everything worth designing for: run the units in COST
ORDER — 0, then C, then A, then B.** The daily total is now ~63 min of confined I/O.
If the window closes early, or the box is too busy and the interlock defers, a
cost-ordered sequence has already delivered the cheap high-value checks; a
cost-*descending* one would have spent the window on B and delivered nothing. Partial
completion should degrade to *less coverage*, never to *no coverage*.

**Gate on eviction, not latency.** Tier B moved `refault` while latency was *better*
during the run than at baseline. Any health check the timer grows must watch
refault/major-fault behaviour; latency alone would report "fine" about the one effect
a heavy tier is known to produce. sdk-vendor's tripwire (>50 pages/s over 5
consecutive samples) is armed but — their words — proven only synthetically and by a
wiring test, never on a real run. Treat a quiet tripwire accordingly.

**`GATE_LABEL` per unit is now load-bearing rather than hypothetical**: A, B and C ran
with separate labels and separate state files. Sharing one would have had the three
answering each other's `repeat` question.

## The final split — and one correction to what tier 0 can promise

sdk-vendor measured the presence checks rather than assuming: **11 checks, 34 ms (195 ms with startup)**
against the real 455 GB snapshot, versus 93 s for a single full-scan check. ~2,700×. They are free.

| tier | what | measured | eviction | cadence |
|---|---|---|---|---|
| 0 | 11 presence checks | 0.2 s | 0 | see the correction below |
| C | 1 mention anti-join | 361 s (6 min) | 0 | daily |
| A | 24 checks (**including** the 11) | 1405 s (23 min; **plan ~29**) | 0 | daily |
| B | 12 joins / anti-joins | 2013 s (34 min) | 239 refault | **daily** — see below |

> **Corrected twice, and the second correction is to my reasoning, not their arithmetic.**
>
> *(a)* The first figures conflated gate and triage: A and B included a one-off #33 diagnostic the standing
> timer will never run, C did not. Gate-only: **B is 34 min, not 53; A is 23 min, not 39.** The ordering
> survives (B > A > C > 0), so the cadence inversion and everything resting on it holds — but a comparison
> where two of four members carried a workload the others didn't is a worse defect than a wrong number
> (sdk-vendor's own assessment). A also has an unreconciled 1730 s vs 1405 s spread across runs, cause
> unestablished; **plan against ~29 min**, since undersizing a window is the expensive direction.
>
> *(b)* **I wrote that B's cadence was "not decidable from these numbers, because they are unconfined".
> That was false, and I never checked it.** All three tiers ran inside the cgroup —
> `MemoryMax=512M`, `IOWeight=10`, `CPUWeight=10`, `Nice=19`, confinement proof firing on each. So B's 239
> refaults are the **confined** figure: the cap did not hold eviction to zero, it held it to ~0.05 % of the
> 1.72 GB working set, with the service's cache growing and latency flat-or-better throughout. The
> measurement I called missing had been taken; I built an argument on an assumption about someone else's
> conditions without reading them.

### The correction: a 5-minute cadence against a daily snapshot buys nothing

The proposed reframing is that tier 0 moves catastrophe detection — an emptied or nuked layer, the thing
the prenuke is insurance against — *from within-a-day to within-minutes*. **It does not, as designed.**

The gate reads the **snapshot**, produced once per day by the pipeline. Running its presence checks every
5 minutes therefore answers, 288 times a day, whether *a photograph taken this morning* still has rows in
it. A live layer emptied at noon stays invisible until tomorrow's snapshot. **Detection latency for the
catastrophic case is bounded by the snapshot cadence, not the check cadence** — so running the same
snapshot-side check more often cannot tighten it.

What a 5-minute snapshot-side tier 0 *would* catch quickly is a snapshot file that is itself truncated or
corrupted. Real, but not the prenuke case, and not what the promise says.

Worse, it works against this design's own state-not-event requirement: 288 identical PASSes a day is a
stream with no information in it, and the failure mode we already committed to avoiding is an operator
learning to ignore a recurring green.

**Two honest options, and the choice is not mine to make:**

1. **Tier 0 runs against the LIVE database.** Then the within-minutes promise is real. The cost is 11
   bounded `EXISTS(SELECT 1 FROM …)` probes — roughly one page each, ~34 ms, no scan. But under the
   adopted prod-box rule that is a *data-page* read and therefore snapshot-only, so this needs an explicit
   carve-out: **"bounded EXISTS probes, no scan"** as a third category. That is a rule change and belongs
   to team-lead, not to me — and it is exactly the sort of exception that should be argued once and
   written down, not taken quietly because the number is small.
2. **Tier 0 stays snapshot-side**, in which case it runs **once per snapshot** (fold it into the daily
   unit) because more often adds no information, and the issue's claim is restated as *within one snapshot
   cycle* — which is what it already was.

**Resolved: neither — take (2) now, and put the live detector where it belongs (issue 133).**
sdk-vendor's third option is better than both of mine: the live-catastrophe detector belongs **in the
app**, which already holds the layer open and reads these tables constantly. It detects immediately
rather than on a poll, raises no prod-box read question at all (the app reading its own database
in-process is what it does anyway), needs no carve-out — and is a **state change rather than a
heartbeat**, firing when the layer becomes empty instead of reporting 288 times a day that it has not.

That reasoning is issue 119's producer argument again: *the thing that already touches the artifact
continuously is the right place to notice it changing.* A snapshot-side gate is the right instrument for
structural verification of a point-in-time artifact and the wrong one for liveness of a running system.

So: **tier 0 stays snapshot-side and runs once per snapshot**, its honest claim being *within one snapshot
cycle* — which is what it was before the measurement — and it keeps earning its place as a subset
selector and as the cheap answer to "did the snapshot arrive intact". The live detector is **issue 133**,
filed separately rather than held as a blocker on this cadence.

### Two constraints not to optimise away

- **`TIER=0` is a subset selector, not a partition.** Tier A must keep running all 24 including the
  eleven. Splitting them out reopens the vacuity hole — a Tier-A-only run would once again pass over an
  empty table, since "zero rows violate X" is trivially true of nothing. At 34 ms there is no cost
  argument for weakening it, and the apparent redundancy in the unit config is what keeps each tier
  self-sufficient.
- **`FAIL_ON_REPEAT=1` on the daily and weekly units only; `0` on tier 0.** Repeat detection asks "did the
  pipeline produce a new input", which is meaningless at a cadence faster than the input changes — on a
  5-minute unit it would fire constantly and correctly. Tier 0's question is *"is the layer still there
  right now"*, not *"did a new cycle happen"*. Stated in the config, not discovered.

## A known-red baseline needs no suppression — that is what state-not-event is FOR

The two newly-gated negative-money surfaces fired on prod: **35,001 `awarded_cents` + 36,987
`bids.cents`** (issue 36). So **Tier A is red, and stays red until #36 resolves.** The question put to
me was: suppress it as a known-red baseline with a hard expiry, or block the timer on #36.

**Neither.** Both are workarounds for a problem this design already solved. The state-not-event
requirement says the verdict renders as *the standing condition of the layer* with **new violations
distinguishable from continuing ones**. A red Tier A with 72,000 known rows **is** the standing
condition — so it is **reported**, not **alerted**, every morning, and nothing needs suppressing.

The alerting unit is therefore a **transition**, not a run:

| event | alert? |
|---|---|
| a check goes red that was green | **yes** — a new finding |
| a check stays red, same condition | no — it is the standing state, visible on the surface |
| a check goes green that was red | **yes** — resolution is worth knowing |

**Why this is better than suppression, and not merely tidier:** a suppression on
`negative_amount(awarded_cents)` hides the case where 35,001 becomes 50,000. That is an *escalation on
a suppressed check* — the single most valuable thing the check could tell us after the initial finding,
and precisely what a blanket suppression discards. It also needs an expiry someone must remember, which
is a promise, and this design has already established what happens to those.

And blocking the timer on #36 would mean **the gate that just found 72,000 unchecked rows does not run
until someone finishes triaging what it found** — the detector held hostage by its own first success.

**Consequence to accept deliberately:** the first wired run alerts on every currently-red check at once.
That is correct — it is the first time this detector has spoken — and it should be read as an inventory,
not an incident.

**And that framing has to reach the reader BEFORE the alert does** (sdk-vendor's point, and the right
one). It goes in the unit's `Description=` and in the first journal line, not in an issue someone finds
afterwards: whoever opens a red verdict at 07:40 on day one is forming their model of what this thing is
*in that moment*, and "the detector just inventoried a layer nobody had checked" and "the pipeline broke
last night" produce very different reactions to the same output. A caveat that arrives after the reader
has already decided is not a caveat, it is an excuse.

**This is also the first real test of the state-not-event requirement**, which until now was an argument
about a hypothetical. It arrived within a day of being written and the requirement covered it without
amendment, which is the outcome that argues for having made it firm rather than optional.

## REQUIREMENT: the verdict is a state, not an event (issue 33)

**Firm requirement, team-lead 2026-08-04 — build it in from the start, not as polish.** The concrete
shape: the verdict renders as the **standing condition of the layer** — *"violates X, N rows, unchanged
since `<date>`"* — on the same user-facing surface as the Resolved-categories ledger, as a condition and
never as a ping. **New violations must be distinguishable from continuing ones**, so that a first
occurrence is visible without a persistent one having to re-announce itself daily to stay true.

The gate's first real run was **not** a dry green: it caught `cents < 0` on 17,738 rows in
`tender_version_amounts` (issue 33) — an invariant agreed as a hard fail ten days earlier that went
unenforced because its carrier, four pinned totals, had rotted. So build for FAIL as a **normal, expected
outcome**, not an exception path: no retry loop, no crash, no special-casing.

The consequence that matters for the integration: a real violation will persist across days until someone
fixes it, so a naive per-run alert fires every morning, gets muted within a week, and the gate is
decorative again — the same ending as the rotted totals, reached by a different road. So the surface must
show the **current verdict as state** (this is the standing condition of the layer), not rely on anyone
noticing a repeated event. New violations should be distinguishable from continuing ones; neither should
be silenceable by habituation.

## The interlock: bounded deferral, never a precondition

Load, not correctness — so the interlock must never be able to suppress the verifier indefinitely:

1. Ask the app whether a heavy job is in flight (short timeout).
2. If yes, defer and re-ask, up to `MAX_DEFER` (**number deliberately unset** — it depends on the measured
   run length and on how long a window this box actually offers; see cost, above).
3. Past the bound, **run anyway** and mark the verdict `contended`.
4. If the app is **unreachable**, do NOT defer — run immediately. An unreachable app is the wedge case
   the timer exists to survive, and there is no contention to avoid when nothing is running.

The failure this avoids: gating VERIFY behind "no heavy job running" *unconditionally* means a wedged
pipeline (a heavy job stuck forever) suppresses the verifier at exactly the moment its staleness check
is the thing that would tell us. A verifier that goes quiet when the system breaks is the
decorative-detector shape wearing a different hat. Under-confined and noisy beats silent.

## Verdict vocabulary — a degraded run must not read as a clean one

`PASS` · `FAIL` (violations) · `STALE` (pointer missing/stale, or input older than the bound) ·
`CONTENDED` (ran under load; timings untrustworthy, violations still valid) · `ERROR` (could not run).

`CONTENDED` exists because a run that had to fight the disk is not equivalent to a quiet one, and
silently collapsing the two is how a caveat gets lost. sdk-vendor is carrying the marker in the gate's
output.

## Publication

- **journald always** — the durable channel, and the only one that works when the app is wedged.
- **Best-effort POST to the app** so the verdict reaches `/admin` and the dashboard.
- A failed POST changes the *reporting*, never the verdict, and says so in the journald line.

## Confinement

`MemoryMax` (so the scan reclaims its own page cache instead of evicting the live service's ~2 GB),
plus `IOWeight`/`CPUWeight`/`Nice`. **Values deliberately absent**: they are sdk-vendor's measurement to
make, and inventing numbers here would be the same species of unmeasured assumption this whole task
exists to remove. Wiring waits on that measurement.

## Failure semantics

The verifier is **downstream and read-only**: a FAIL never blocks or reverts the pipeline. It alerts.
The pipeline has already committed by the time the gate reads the snapshot, so there is nothing for it
to stop — its product is knowledge, not control.

## What a green does not mean

Two limits, both already written where the green is produced (sdk-vendor's "WHAT A GREEN DOES NOT MEAN"
header):

- **Vacuity** — 0 violations is trivially true of an empty table; the `*_present` checks close it.
- **A check downstream of a guard cannot validate that guard.** For `head_not_max` specifically, my
  task-#27 assertion refuses and rolls back, so a violation never lands, so the snapshot is clean and the
  gate finds nothing. Gate-green there means "no damage **or** damage prevented" — indistinguishable.
  The signal that a violation occurred is a failed projection job, nowhere else.

## Before wiring

1. sdk-vendor's confinement measurement (blocks wiring, not design).
2. The `latest` pointer lands in `snapshot::run` (mine, small, testable: a failed snapshot must leave
   the pointer untouched — that is the test worth writing first). Demoted from "the fix" to "names the
   right file"; freshness is repeat detection.
3. The unit grants a writable state directory, **and the integration proves it** — a first run producing
   `repeat=unknown` followed by a second producing `repeat=no` is the check that the mechanism is
   actually armed. Without that, the most likely way this ships is silently inert.
4. Where the dashboard renders the verdict — a surface question for team-lead, not a mechanism one.
