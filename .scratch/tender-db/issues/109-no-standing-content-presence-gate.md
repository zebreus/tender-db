# 109 — nothing detects content staleness: every production gate counts rows, and shells have rows

Status: open — the gap the eForms-DE 1.x recovery exposed. Not blocking tonight (H1/H2/H3 are this
check, run once by hand); load-bearing before the next mapping change.
Kind: observability / production invariant
Blocked by: —
Relates to: 108 (the watermark whose over-claim this would catch), 99 (the cause the epoch closes),
85 (the incident: 218,635 shells, green on every gate), 105, 86/48/88 (the next changes that can do it)

## The gap

Every production invariant we have counts **rows**. A factless version **has** a row. So a cohort can go
completely content-stale and every gate stays green:

| gate | what it counts | sees a shell? |
|---|---|---|
| `G2` (`tender_versions` vs parsed-and-projected notices) | rows vs notices | **no** — the row exists |
| `B1` / per-cohort version counts | rows | **no** |
| `EXPECT_NO_REGROUPING` (tenders/islands/keyed/retire) | grouping | **no** |
| `projected` watermark | notices considered | **no** (issue 108) |

That is not hypothetical. **G2 was equal — measured 14,150,061 on both sides — for the entire period the
eForms-DE 1.x cohort was 218,635 factless shells.** Nothing flagged. The defect was found by a human
sampling facts, and only because someone went looking.

The issue-99 epoch closes the *cause* for logic changes. It does not create a detector, and the next
content-staleness — a narrowed mapping in 86/48/88, a dialect whose vendored metadata drifts — will sit
green on every counting gate exactly as this one did.

## What to measure: the symptom, not the cause

**Per-profile factless-version rate.** A version with no rows in any of `tender_version_texts`,
`_classifications`, `_dates`, `_amounts`, `_parties`, `_lots` is a shell regardless of how it got that
way. Cause-agnostic, which is the point: it fires on 99's skip, on 105's drop-out, on a bad alias table,
on a vendored-inventory regression, and on causes nobody has thought of.

Per **profile**, not globally: the DE-1.x cohort was 1.5% of all versions, so a global average barely
moves even at 100% staleness. Per profile it is unmissable — DE-1.x would have read 100% factless against
sdk-1.13's baseline.

## Making it cheap enough to run continuously

A full anti-join over 14.1M versions × 6 satellites is exactly the kind of full-corpus scan this project
keeps having to remove, so it must not be that.

- **Sample, don't scan.** ~1,000 versions per profile, random, gives a tight enough interval to catch a
  cohort going wholesale stale, at negligible cost. Detection floor scales with sample size: 0 → 100% is
  caught instantly; a 1%-of-cohort staleness needs a bigger sample and is out of scope for a cheap
  continuous check.
- **A fold-time counter does NOT work, and the reason is the point of this issue.** `Applied` could
  cheaply carry "versions written with zero facts" — but stale content is precisely what is *not*
  rewritten, so the counter is blind to exactly the population it needs to see. It has to be a read-side
  check.
- Natural home is the periodic coverage refresher rather than the request path.

## Alerting on the delta, not an absolute floor

Per-dialect fact density legitimately varies (a text-era notice carries less than an eForms one), so
hand-tuned per-profile floors would be wrong on day one and rot after. Track the rate per profile over
time and alert on a **step change** — profile X went 2% → 100% factless between runs. Cause-agnostic,
no constants to maintain, and it is the shape the incident actually had.

## Acceptance

- Per-profile factless-version rate recorded on every refresher run, visible on the dashboard.
- A step change alerts without anyone running an exhaustive suite.
- Backtest: run it against a snapshot from the issue-85 window; it must flag eForms-DE 1.x at ~100%.
  A gate that cannot retro-detect the incident that motivated it is not yet a gate.

## Note

The general form, which the recovery demonstrated five times over: **a check is only as good as its
knowledge of what it measures.** Counting gates answer "is a row there", and were read as answering "is
the content right". Those differ exactly when it matters most.
