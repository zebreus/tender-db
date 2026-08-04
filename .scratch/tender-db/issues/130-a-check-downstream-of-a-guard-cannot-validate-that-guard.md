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

## Sibling hazard: a check exercised both ways can still be blind (2026-08-04)

Same family, different mechanism, recorded here because it was found the same day and would otherwise
live only inside a SQL file.

Proving a check can answer **both ways** — planting a violation, seeing it fire, removing it, seeing it
pass — is the discipline this team has applied all day, and it is necessary. It is **not** sufficient.
There are two independent axes:

1. **Construction** — can the check distinguish, or is it structurally unable to fail? (The alias-blind
   plan assertion; a `repeat=` grep matching its own fallback note.)
2. **Coverage** — does the fixture span the states the *real data* actually occupies?

Axis 2 is the one that bites when **the fixture is authored by the check's author**, because the fixture
then contains exactly the cases its author already had in mind. The instance: sdk-vendor's P4 query was
exercised both ways against a **single-version** tender, and was correct for that. But the fold carries
facts forward across versions (`project.rs:1887`/`:1931`), so a carried negative appears at every later
seq whose causing notice has none — and the query flagged those as fold-introduced. Carry-forward cannot
appear below two versions, so no amount of both-ways rigour on that fixture could have surfaced it.

sdk-vendor's own note on why they missed it is the useful part: they *had* read the fold code, to answer
"does it do arithmetic on cents" — which it does not. **The semantics that mattered were invisible from
the question they were asking.**

Practice: a fixture should be reviewed by whoever owns the semantics under test, not only by whoever owns
the check. And where a check spans two subsystems, keeping each half with its owner beats one person
doing both — not for capacity, but because a single author cannot see the state they did not model.

## The practice this suggests

For each standing check, state which of **prevented** / **detected** it covers, and where the *other*
signal lives. Two lines per check, and it forecloses the reading in advance. Worth doing when the gate's
check inventory is next revised — not urgent, and deliberately not filed as work on anyone.

The stronger version, if it ever seems worth the cost: prove the guard by **disabling it against a
poison** rather than by observing a clean downstream. That is what #27's test does
(`a_head_that_is_not_the_last_version_is_refused_before_commit`, verified to fail with the gate forced
off) — the guard is proven by its own test, not by the gate's silence, and that is the only place the
proof can live.
