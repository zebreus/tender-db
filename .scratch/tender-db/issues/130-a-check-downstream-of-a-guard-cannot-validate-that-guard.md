# 130 — a check downstream of a guard cannot validate that guard

Status: RECORDED — a verification hazard, not a defect. No code is wrong; the risk is in how a green is
read. One small practice proposed at the end.
Kind: verification discipline
Owner: proj-fix (first instance is mine)
Relates to: 28 (the standing gate), 27 (the projection-time head assertion), 109, 112, ADR-0004
First stated by: sdk-vendor (as a window-overlap caveat), sharpened here into its general form

## The general form

When a **guard** prevents a bad state upstream, and a **check** looks for that bad state downstream, the
check cannot tell you the guard works. Both of these produce an identical clean result:

- the guard is working, and stopped the damage;
- the guard is absent, and there was no damage to stop.

The number is `0` either way. Nothing in the output distinguishes them, and a reader who knows the check
exists will tend to read `0` as "the guard is verified end to end", which is the one thing it cannot mean.

This is a sibling of the vacuity hole in sdk-vendor's gate (`standing_gate.sh`, "WHAT A GREEN DOES NOT
MEAN"). Both are *"a check returns 0 and the 0 means something other than what a reader assumes"*:

| | why it is 0 |
|---|---|
| **vacuity** | nothing is there to violate — an empty table satisfies every violation count |
| **guard shadow** | something upstream stopped it before it could land |

Neither is visible in the number.

## The instance that produced it

Task #27 added a projection-time assertion: a Tender whose head pointer is not its last version is
**refused inside the write transaction and rolled back** (`canonical.rs`, `assert_heads_match`). Task
#28's standing gate has `head_not_max`, which looks for exactly that violation in the daily snapshot.

I described this as the same invariant guarded from both sides, and was pleased with the pairing. It is
weaker than that. Because the assertion *prevents*, a violation it catches never commits; because it
never commits, the snapshot is clean; because the snapshot is clean, `head_not_max` finds nothing. After
#27 ships, `head_not_max` green means **"no damage or damage prevented"** — indistinguishable from the
snapshot side.

The two are not redundant, but they are not confirmations of each other either. The honest division:

- **the assertion prevents**, over the Tenders a projection run rewrote;
- **the gate detects**, over everything the assertion cannot see — pre-existing damage, and Tenders no
  run has touched;
- **the signal that a violation actually occurred** is a **failed projection job** carrying the
  assertion's error text. It exists nowhere else. If nobody watches job failures, that evidence is lost
  and the system looks exactly as it does when nothing ever went wrong.

## Why it is worth a tracker entry rather than a comment

It is already written at both ends (the doc comment on `assert_heads_match`, and sdk-vendor's gate
header). Those help whoever is reading *that* code. The misreading happens somewhere else: to whoever
reads a green dashboard, or a verification summary, and concludes an invariant is verified. That reader
is not looking at either file.

## The practice this suggests

For each standing check, state which of **prevented** / **detected** it covers, and where the *other*
signal lives. Two lines per check, and it forecloses the reading in advance. Worth doing when the gate's
check inventory is next revised — not urgent, and deliberately not filed as work on anyone.

The stronger version, if it ever seems worth the cost: prove the guard by **disabling it against a
poison** rather than by observing a clean downstream. That is what #27's test does
(`a_head_that_is_not_the_last_version_is_refused_before_commit`, verified to fail with the gate forced
off) — the guard is proven by its own test, not by the gate's silence, and that is the only place the
proof can live.
