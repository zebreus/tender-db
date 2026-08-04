# 160 — 51 negative `estimated_value` / `framework_maximum` rows: the residue the re-spec still flags

Status: open — the follow-up the #33 re-spec does NOT close (team-lead, explicit)
Kind: data quality / invariant residue
Owner: unassigned (re-spec itself is proj-fix's; this is the leftover)
Relates to: 121 (the gate that found it), 131 (proj-fix's parse-vs-fold determination),
task #33 (the triage), `run_light.sh` 3.7 (the invariant being re-specified)

## Why this exists as its own issue

Task #33 triaged 17,738 `tender_version_amounts.cents < 0` rows found by the standing gate.
The verdict: the source publishes them, the fold copies them faithfully, and `run_light` 3.7
("no negative money", a hard-fail gate since 2026-07-25) is over-strict. The fix is to
**re-specify** 3.7 by field rather than relax it.

**But the re-spec is not a resolution for all 17,738.** Team-lead's framing, recorded here
because it is the thing most likely to be lost: *"those 51 are NOT closed by the re-spec —
they're the smaller genuinely-suspect set the narrowed invariant still flags. 17,738
unexplained → 51 worth explaining is progress, not resolution."*

A narrowed invariant that still fires on 51 rows has done its job. Marking those green
because the *other* 17,687 turned out legitimate would be the amplify-a-firing reflex run in
reverse — using a mostly-benign explanation to wave through the part it does not cover.

## The 51

Measured 2026-08-04 against pinned snapshot `tender-db-1785830601.db` (Aug 4 08:18 UTC):

| field | rows | currency | most negative |
|---|---|---|---|
| `estimated_value` | 27 | EUR | -42,000,000 (€420k) |
| `estimated_value` | 2 | DKK | -600,000,000 (DKK 6M) |
| `framework_maximum` | 22 | EUR | -1,180,000,000 (€11.8M) |

Against 17,687 `result_value` negatives, which are 99.6% between €1 and €100 and sit on
award/result notice subtypes (29/30/31 dominate) — i.e. adjustment-shaped.

## Why these are the suspect ones

A negative **result value** is semantically available: awards get corrected, contracts get
withdrawn, credit adjustments are published. A negative **estimated value** or **framework
maximum** is not — both are forward-looking ceilings, and a ceiling below zero does not
describe anything a buyer could mean. Their magnitudes also differ sharply from the
`result_value` mass: these are large (€420k, €11.8M), not the €1–€100 cluster.

## What is already known, so nobody re-derives it

* **Not the fold.** P4 = 0 folded negatives whose version chain carries no negative, and
  17,738/17,738 have an exact magnitude match in the chain's parse layer. The projection is
  a faithful copy (proj-fix's code-path analysis, issue 131, confirmed on data).
* **Not a parser-version defect.** The full set scatters across 13 SDK versions (1.6→1.14
  plus DE variants) and 4 years; a version-specific mapping fault would cluster.
* **So the source published them** — the same conclusion as for `result_value`. Which is
  precisely what makes these interesting: the source published a negative ceiling.

## Open questions

1. Do the 51 cluster on a profile/version/publisher once isolated from the 17,687? The
   whole-set scatter does not rule out a cluster *within* this subset — nobody has looked.
2. Is a negative `estimated_value` a published sentinel (e.g. "not disclosed" encoded as a
   negative) rather than a value? The `-100` = exactly −1.00 pattern in several currencies
   in the larger set hints that sentinels exist in this data.
3. Is the *mapping* right — is some field whose published semantic is a delta being mapped
   as an absolute ceiling? That would be a real defect and would not be visible from the
   sign alone.

## Not urgent

51 rows, no downstream breakage known, and the narrowed invariant will keep flagging them,
which is the correct behaviour. This exists so that "the negative-money finding was
explained" does not quietly become "all of it was fine."
