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
