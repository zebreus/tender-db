# 109 — nothing detects content staleness: every production gate counts rows, and shells have rows

Status: BUILT 2026-08-21 (owner) — the gate is in the weekly data-quality run. Remaining: deploy
(queue busy with the issue-234 merge), then the backtest and the first live report (see "Built"
at the bottom). Was: open — the gap the eForms-DE 1.x recovery exposed.
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

---

## Built (2026-08-21, owner) — the shell detector, in the weekly data-quality run

**Where it lives.** Not the sampled probe sketched above — a cheaper-than-expected EXACT count,
because the DQ run's windowed machinery (issue 230) already pays the traversal: a new `factless`
windowed query counts, per era, versions with NO rows in ANY version-keyed satellite (texts,
classifications, dates, parties, amounts, lots). The `NOT EXISTS` chain is ordered by fill rate
(texts first), so a normal version short-circuits on its FIRST probe — per-version cost is one
indexed seek, the same as a single field-completeness probe. Windowed and summed like every other
label; the windowed-equivalence gate covers it automatically.

**Where it shows.** Report section `== 7. Content presence ==` (per-era versions/factless/rate),
`content_presence` in the JSON, and — the alerting half — a STEP-CHANGE alarm: the run stores its
per-era rates as their own tiny report kind (`data-quality-presence`) and compares the next run
against them. A cohort that went wholesale stale between runs (≥ 20-point rise AND at least
doubled, on ≥ 1,000 versions — a step, not a floor, because per-era fact density legitimately
varies) leads the report body under `!! CONTENT-STALENESS STEP CHANGE !!` and rides the job
summary, which is what the operator sees first. First run: no baseline, no alarms, by construction.

**Gates.** `a_stripped_cohort_reads_factless_while_every_row_count_stays_green` (ingest,
live-DB): strips one era's satellites with the version rows SURVIVING — the issue-85 shape — and
asserts that era reads 100 % factless, untouched eras 0 %, and the version COUNT does not move
(the blindness restated as an assertion). `the_step_change_alarm_fires_on_a_jump_and_stays_quiet_on_noise`
(unit): 2 %→100 % fires; 2 %→8 % drift, sub-1,000 cohorts, no-baseline eras, and steady state
stay quiet.

**Acceptance mapping.** "Recorded on every refresher run" — every weekly DQ run now measures it.
"A step change alerts without an exhaustive suite" — the alarm above. "Backtest against the
issue-85 window" — STILL OWED: needs an on-box run against a pre-recovery snapshot once the box
is quiet; the live-DB gate reproduces the incident's shape in miniature meanwhile.
