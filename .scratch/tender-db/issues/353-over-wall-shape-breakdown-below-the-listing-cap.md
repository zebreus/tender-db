# 353 — the over-wall backlog below the listing cap: ~55,500 names / ~2.4M rows the verdict cohorts cannot reach

Status: BUILT 2026-09-04 15:5x (gate running) — the fold's dry run now tallies every raw-wall group by shape (`id0/c0` pure echo … `id>cap/c2+` shared) with rows, and carries a 100-name uniform sample of them; deploys with `b71e809` at the next idle window; the cohorts 4+5 dry run is the measurement. Was: ready-for-agent (filed 2026-09-04 from issue 351 unit 5's cohort 5)
Kind: measurement first (organization layer, the provisional-echo fold) — small
Relates to: 351 (the fold, its tiers and verdict cohorts), 350 (the wall conflates fragmentation with genericness), 349 (echo vs shared carriers), 234 (the exclusion guards the wall stands in for)

## Observed

Issue 351 unit 5 recorded a verdict on every over-wall name the census could
list: 2,158 names in five hand-read cohorts (top 200 by rows, then 400, 400,
400 and the last 758; cap 2,000, 2,176 over-wall entries). Across the cohorts
the split was steady — ~84% `single`, ~4% `generic`, ~3% `non-name`, ~8%
`unclear` — and the singles admitted ~730k rows.

What the listing cannot reach: after the stock fold the class still held
57,692 multi-row names / 2,968,199 rows; the cohorts settled ~2,200 of them.
The remaining ~55,500 names hold ≤229 rows each, ~2.4M rows in all. At 400
names a hand-read cohort that is ~140 cohorts — not a path.

Why they are over the wall at all: `echo_tier_by_wall` reads a key over the
cap as `EchoOfOne` only when 1..=cap carriers hold an identifier and those
sit in one country. A name whose carriers are NOTHING BUT its own
country-less, identifier-less rows (`with_identifier = 0`) cannot qualify —
it is `OverWall` by construction, though the wall measured its
fragmentation, not any sharing (issue 350's finding). The cohorts show the
tail is mostly such names: city councils, hospitals, départements, ministries
with a country in the string.

## Built (this unit — measure before choosing)

- `GENERIC_KEY_BREAKDOWN_SQL` gains a sixth column, `COUNT(DISTINCT
  o.country)` over all carriers (the fifth counts the identified carriers'
  countries only). Existing readers use columns 0–4 by index; unchanged.
- `WallBreakdown` keeps the six counts; `echo_tier_detail` returns the tier
  with the breakdown whenever the wall was asked and found the key over the
  cap; `echo_tier_on` wraps it (the resolver, the census and the probe are
  unchanged).
- `ProvisionalFoldReport.over_wall_shapes` / `over_wall_shape_rows`: the
  raw-wall `over-wall` groups (verdict-refused ones excluded — a verdict
  settled those) by `WallBreakdown::shape`: `id0 | id1-cap | id>cap` ×
  `c0 | c1 | c2+`, groups and rows. `over_wall_sample`: a reservoir of 100
  of those groups (fixed xorshift seed; the walk is ordered, so the same
  stock yields the same sample), (name_norm, name, rows, shape).
- The supervisor's `provisional-echo-plan` report and the job message carry
  all three.
- `provisional_echo_fold.rs`: `Gemeinde Generic` (12 carriers, none
  identified) reads `id0/c0`; a new `Bank Many` (three country-less rows,
  ten identified rows in DE and AT) reads `id>cap/c2+` and stands; the
  verdict-refused `Kreis Zwei` has no shape.

## Next

Deploy; the cohorts 4+5 dry run (`fold-provisional-echoes {"dry_run":true}`)
is the measurement. Then, with the shape split and the sample read:

- If `id0/c0` (pure echo) holds most of the rows and its sample reads like
  the cohorts' singles: a rule — a pure-echo name over the wall folds like an
  under-wall one — is the same policy unit 2 already applies under the wall,
  minus a wall that says nothing about it. The residual risk is the
  cohorts' ~15% non-single names (generics, form strings, two-country names
  like `Stadt Burgdorf`): a fold merges their echoes into one junk or
  two-entity row instead of leaving N junk rows. Weigh it with the sample;
  a row cap (fold pure echoes of ≤ N rows) or a refusing-verdict pass over
  the sample's generics first are the knobs.
- If `id>cap` / `c2+` shapes carry the rows: those ARE shared names, and
  stay behind the wall; the backlog is then smaller than it looks.
- Either way, the census listing cap (5,000 max) can rise cheaply — the
  heap is bounded by the cap — for a sixth hand cohort of the largest
  remaining names if the rule is refused.
