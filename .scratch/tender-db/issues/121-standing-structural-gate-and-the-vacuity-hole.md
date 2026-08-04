# 121 — a standing structural gate, the empty-layer hole it closed, and two corrections it forced

Status: phase 0 landed (`8e96b8f`, hardened `ca1a9b8`, `d22bb6e`, `728d7cf`); phase 1 PRE-AUTHORIZED
(Tier A, confined arm only) and STAGED — held by the box, not by permission; see Not yet done.
Companion tooling: `snapwatch.sh` (`b05befc`, live on the box), `thread_cpu.sh` (`6150399`, `18b0e90`)
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

## Not yet done

* **Phase 1** — PRE-AUTHORIZED and STAGED (`phase1_confined_probe.sh`), not yet run. Blocked on the box,
  not on permission: an abandoned Class B read has held a slot since 09:33:44 with no client attached
  (~66 min at 10:39, non-monotonic). The wait is bounded to ~11:10Z, after which run-driver proposes a
  controlled restart. Running contended was considered and REJECTED — the residual is non-steady, so it
  could finish mid-run, latency would improve, and the confinement would be credited for it: a false
  green on the precise question. Two defects were found in the probe before it was fit to spend the
  authorization, both of the same shape (`7afac43`, `02b443d`): the input was resolved TWICE (once in
  preconditions, again in the confined unit at launch) so the verified file and the measured file could
  differ silently; and the script enforced only ONE of the two gates the authorization named, passing
  while a scan ran. The rule both produced: **encode the standard, do not hold it in your head** — a
  precondition that codes fewer gates than the authorization names is a false green waiting for the day
  nobody checks the other signal by hand. Original text below stands as the design:
  one confined, instrumented
  run against a real snapshot. Proposed confinement is a systemd unit with `MemoryMax=` (page cache is
  charged to the faulting cgroup, so it reclaims its own rather than evicting the live service's ~2 GB),
  plus low `IOWeight`/`CPUWeight` and `Nice=19`. That mechanism must be **measured, not asserted** — cf.
  issue 17, "resolved" on construction and unverified under load for a week. Run only the confined arm;
  the unconfined control is deliberately the harmful case.
* **Phase 2** (only if phase 1 holds): timer in the low-traffic window, sequenced after the daily
  pipeline's snapshot step so it never overlaps ingestion/projection. Coordinate with proj-fix.
* **Phase 3 / the prenuke:** N consecutive green days makes the gate a sound replacement for the
  *unreproducible 07-28 verdict*, NOT for the *backup*. Deletion stays parked at #20 on its own evidence.
  The gate's greenness must not quietly become a deletion argument.

## A green is not end-to-end verification where a preventer runs upstream

Recorded 2026-08-04 (proj-fix's correction, sharper than the limit I first stated).

I offered `head_not_max` as a detector proj-fix could check their #27 projection-time
assertion against — "assertion fires ⟺ gate finds violations". I hedged it only with a
window-overlap caveat. The real limit is stronger and partly dissolves the pairing:

**#27's assertion refuses and rolls back.** A violation therefore never lands, the
snapshot stays clean, and this gate finds nothing. So once #27 ships, gate-green over
that invariant is consistent with *both* "no damage" and "damage attempted and
prevented" — **indistinguishable from here**. The signal that separates them has moved
somewhere this gate cannot see: a failed projection job carrying the assertion's error
text.

The honest statement of the relationship is therefore **not** mutual confirmation:

* the **assertion prevents** new violations;
* the **gate covers what the assertion cannot see** — damage predating it, and Tenders
  no projection has touched since.

Generalised, because it is not specific to #27: **a check downstream of a guard cannot
validate that guard.** The guard working and the guard being absent-but-unneeded produce
the same clean state. Reading gate-green as end-to-end verification of an invariant that
something upstream enforces is the decorative reading — the check is real, but it is not
answering the question it appears to answer. Written into `standing_gate.sh`'s header as
"WHAT A GREEN DOES NOT MEAN" so it cannot be misremembered as confirmation.

This is the same family as the vacuity hole above: in both cases a check returns 0 and
the 0 means something other than what a reader would assume. Vacuity is "0 because
nothing is there"; this is "0 because something upstream stopped it". Neither is
detectable from the number alone.
