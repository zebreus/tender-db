# 119 — section A's freshness depends on a snapshot nothing guarantees

Status: open — filed 2026-08-03 (sdk-vendor), from the 117 post-deploy gate run
Kind: verification (input has no reliable producer)
Relates to: 111 (the identical shape, for indexes), 112 (section A), 107

## The defect

`hot_read_plans.sh` section A answers "are the declared indexes actually present on the
live DB". It reads `sqlite_master` from `TDB_SNAPSHOT` with `immutable=1`, and — since
the WAL-merged fallback was removed (turso holds the `-wal` against stock sqlite3;
`database is locked (5)`) — it **requires a checkpointed snapshot**.

**Nothing guarantees one exists.** run-driver went looking during the 117 deploy:

* the only snapshots are `/data/db/snapshots/*.db` — two of them, Aug 2 20:55 and
  Aug 3 09:04;
* the service drop-in grants `ReadWritePaths=/data/snapshots`, a **different, empty**
  directory;
* nothing in the journal names a snapshot cycle, and no cron, timer or `/admin` route
  was found that produces them.

So the freshest available input was **hours older than the change it was being asked to
verify**, and section A would have reported all four new indexes `DECLARED at 5c197e7
but ABSENT from the DB` — a wall of red on a healthy database.

## Why this is 111's shape, not a nuisance

111 was "the deferred indexes have no guaranteed builder": a thing the system depends on,
with no component whose job is to produce it. This is the same sentence with *snapshot*
substituted for *index* — a **check whose input has no reliable producer**.

And it fails the same way: silently, and only when it matters. A snapshot that happens to
be recent gives a correct verdict; the moment the thing under test is newer than the last
snapshot, the gate reports the opposite of the truth. Nothing in the gate is wrong — the
input is — which is exactly why section A now prints its snapshot's path and age
(`8b6a2d6`). **But declaring staleness is not the same as preventing it**, and this issue
is the difference.

## Not deploy-blocking, and why

run-driver's watchdog read-timings are the primary live verification of the 117 fix
(`notices?source=doe` in ms, `organizations?country=DE` flat across densities). Section A
is a *corroborating* catalogue check — it distinguishes "index absent, reindex owed" from
"index present, planner ignores it". Valuable, not sole witness. It can wait for a fresh
snapshot without holding a deploy.

## Options

1. **Give the snapshot ring a guaranteed producer** — the same answer 111 reached for
   indexes: a component whose job is to produce the artifact, rather than an operator's
   memory. Matches the existing `/data/db/snapshots/` convention.
2. **Let section A read a freshly-checkpointed live file.** The reindex already ends with
   a TRUNCATE checkpoint, so immediately afterwards the main file *is* current and its
   `-wal` is 0 bytes. A gate that can detect that state could read the live file directly
   under the existing `immutable=1`, no-lock contract. Narrower than (1) and needs the
   post-checkpoint window to be recognisable rather than assumed.
3. **Accept it and require the operator to produce a snapshot before a verifying run.**
   Honest but it is 111's rejected answer: a durability obligation transferred to
   someone's memory.

~~Preference is (1), with (2) as a useful complement.~~ **CORRECTED — option (1) is not
implementable on this box, and I filed this issue without the numbers.**

## The measured constraint (run-driver, 2026-08-03) — a snapshot is IMPOSSIBLE, not awkward

* prod DB **453 GB**; `/data` has **~122 GB free**. A fresh snapshot cannot be cut in
  place at all — not "inconveniently large", *there is nowhere to put it*.
* the live `-wal` **never reaches 0 at idle**: checkpointing is volume-triggered, and it
  sat at **12,392 B across 18 polls over 4½ minutes**.

So "point `TDB_SNAPSHOT` at a checkpointed snapshot" has **no achievable input on this
box**. It was not advice; it was a refusal to answer wearing the shape of a prerequisite.
The snapshot ring's two files exist because they were cut when the DB was smaller.

### What this changes

**The main-file split (`b1d5669`) is not a stopgap pending this issue — given the disk it
is currently the ONLY way section A can ever answer**, and it does answer: run-driver's
re-run reported **25/25 present with columns matching, against the live serving file**,
each verdict carrying its own inline caveat.

That reorders the options:

1. ~~Give the snapshot ring a guaranteed producer~~ — **unimplementable at 453 GB with
   122 GB free.** Anyone planning one should read this section first; the disk cannot
   hold the artifact.
2. **Read the live main file, split by direction** — DONE. Present is sound (frames only
   add); absent is unknowable and reported no-input. This is now the primary mechanism.
3. **Close the remaining gap: the WAL's contents.** What is still unestablished is
   narrow — an index created since the last checkpoint, or dropped in unread frames.
   The honest asks are either a way to read the WAL-merged catalogue through the engine
   that owns it (the app, since stock sqlite3 cannot), or a trigger that checkpoints on
   demand so the main file can be made current without copying 453 GB.

**So this issue is no longer "section A has no fresh input".** It is "section A's input
is sound in one direction and silent in the other, and closing the silent half needs
either a WAL-aware reader or an on-demand checkpoint — not a snapshot." Filed originally
under the wrong premise, corrected by measurement rather than by argument.

## Acceptance

* A verifying run after a change can obtain a snapshot **newer than that change**, without
  an operator remembering to make one.
* Section A on that snapshot reports the post-change catalogue — verified by the case that
  motivated this: run it after a reindex and the four 117 indexes are PRESENT, not ABSENT.
* Point `TDB_SNAPSHOT` at a snapshot older than the change and the gate still says so
  loudly (the age line stays; this issue adds a producer, it does not remove the
  declaration).


## The fourth instance: the OBSERVER, not the instrument (run-driver, 2026-08-03)

Three inputs in this suite can be older than the change they verify — section B's plan
DB, section A's snapshot, section E's fixture. run-driver found a fourth during the 117
deploy, and it is the one none of us had looked at:

> *"I tracked index arrival with `immutable=1` catalogue reads, which cannot see an index
> until it checkpoints — so my progress log lagged reality by minutes and I nearly
> mistook 'not visible' for 'not built'."*

**A stale monitor is worse than a stale gate**, because the monitor is what you consult to
decide whether the gate can be run at all. A gate reading a stale input gives a wrong
answer that the age-line can expose; a monitor reading a stale input makes you *choose
wrongly about when to look*, and nothing downstream can recover it.

### And the same read is correct in one role and a defect in the other

The pre-flight readiness query — poll the main file at `immutable=1` for the four index
names — uses **exactly the limitation** that made the progress log wrong. It is immune
because it asks the stale question **deliberately**: main-file visibility is precisely
the property both consumers (`planschema.db`'s rebuild and section A's snapshot) depend
on, so "cannot see the WAL" is the specification rather than a flaw.

Which is the general point worth keeping: **an input is not stale or fresh in itself —
it is stale relative to the question.** `immutable=1` is a defect in a monitor asking
"has the index been built?" and correct in a readiness check asking "is the index visible
where my consumers will look?" The same read, the same limitation, opposite verdicts.

So the fix for this issue is not "always read the freshest thing available". It is to
state, at each consumer, **which question its input is being asked** — which is what
section A's age line does, and what the progress log did not.