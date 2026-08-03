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

Preference is (1), with (2) as a useful complement — (1) fixes the input, (2) removes the
dependency for the specific case where the system has just told us the file is current.

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