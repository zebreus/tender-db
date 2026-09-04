# 353 — the over-wall backlog below the listing cap: ~55,500 names / ~2.4M rows the verdict cohorts cannot reach

Status: COHORT 6 FOLDED 2026-09-04 22:0x UTC — **job 679 folded 21,235 verdict-single names / 1,329,491 rows** (1,329,408 mentions, 1,383,354 parties, 454,816 winners repointed, 949,163 tenders touched; 1,692 s, parity held), after a two-pass agent read of 24,675 names with an echo alignment guard (pass 1 drifted in one batch; the guard caught it, the cautious merge settled 1,021 disagreements). Under rule `p0` the class has now shed **5,477,733 rows** (stock 3,407,962 + cohorts 2–5 740,280 + cohort 6 1,329,491). Standing: 33,243 raw-wall names / 607,472 rows (`id0/c0` 293k, `id0/c1` 193k; ≤26 rows each) and 1,534 verdict-refused names. Next: wait for Sunday's `org_match_keys` rebuild (stale carriers drop; some names fall under the wall on their own), re-census with the cap at 40,000, and run one more two-pass read over what stands. Was: MEASURED 2026-09-04 17:1x — deployed `7e51dcc`, measured by job 673: **90% of the standing rows (1.85M of 2.05M) sit on names with NO identified carrier at all** (`id0/c0` 32,337 names / 1,174,327 rows; `id0/c1` 19,478 / 672,599); the shared shapes (`id>cap/*`, `id1-cap/c2+`) hold 6.7%. The wall is over these names by spelling fragmentation: one N2 key unites the punctuation/case variants of one name (`COMMUNE DE SAINT-BON COURCHEVEL` + `…SAINT-BON-COURCHEVEL` + … = 44 live carriers, one commune). The 100-sample reads ~93% single, ~7% generic/non-name/multinational (`Price: Various`, `Shell`, `Centre hospitalier, service pharmacie`) — the same ratio as the hand cohorts, so a blind pure-echo rule would merge ~9% of the rows wrongly (the tendsign shape at small scale). Decision: verdicts, at scale — the census listing cap rises to 60,000 with a `shape` per listed group (this unit's second half, gate running), then the ~20k names of 21–229 rows go to agent-read batches. Re-measure after Sunday's `org_match_keys` rebuild. Was: BUILT 2026-09-04 15:5x — the fold's dry run tallies every raw-wall group by shape with rows and carries a 100-name uniform sample; deployed with `b71e809`. Was: ready-for-agent (filed 2026-09-04 from issue 351 unit 5's cohort 5)
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

## Measured (job 673, 2026-09-04, 955 s; the cohorts 2–5 dry run)

Raw-wall `over-wall` groups 55,855 / 2,046,351 rows (verdict-refused 157
and verdict-single 1,680 excluded):

| shape | groups | rows | rows % |
|---|---|---|---|
| `id0/c0` | 32,337 | 1,174,327 | 57.4 |
| `id0/c1` | 19,478 | 672,599 | 32.9 |
| `id1-cap/c2+` | 1,103 | 63,159 | 3.1 |
| `id0/c2+` | 1,268 | 53,545 | 2.6 |
| `id>cap/c1` | 838 | 43,338 | 2.1 |
| `id>cap/c2+` | 482 | 29,692 | 1.5 |
| `id1-cap/c1` | 338 | 9,356 | 0.5 |
| `id1-cap/c0` | 11 | 335 | 0.0 |

`id0/c1` — no identified carrier, one country among the carriers — is a
name that also stands as a `(name_norm, country)` provisional row in one
country: the strongest one-entity signal the data holds short of an
identifier. `id0/c2+` (2.6%) is where two-country names like `Stadt
Burgdorf` live.

**Why 2-row groups are over the wall.** `/admin/name-key` on `COMMUNE DE
SAINT-BON - COURCHEVEL` (2 rows): 44 live carriers, all country-less and
identifier-less, spread over the punctuation and case variants of the same
commune — the N2 key folds what `name_norm` keeps apart. The walk groups by
exact `name_norm`; the wall counts by N2; a name with a dozen spellings of
a few rows each is over the wall in every spelling, so none folds and every
new country-less mention mints. Fragmentation across spellings, not sharing.

**The 100-sample (uniform over raw-wall groups):** 71 `id0/c0`, 24 `id0/c1`;
39 groups of ≤20 rows hold 272 of the sample's 3,225 rows, the 61 groups of
21–229 rows hold 2,953 — the rows are in the mid-size groups. Reading it:
~93 single entities (city halls, hospitals, ministries' directorates,
universities, named companies with a legal form, two people), and `Price:
Various` (95 rows), `Shell` (99, `id0/c2+`), `Centre hospitalier, service
pharmacie` (70), `AZIENDA OSPEDALIERA OSPEDALE CIVILE`, `Texaco Ltd`,
`Yamaha Motor Co`, `Ernst&Young`, `- Philibert -` — ~7% of names, ~9% of
rows, the same ratio the five hand cohorts gave.

**Decision.** A blind pure-echo rule is refused: it would fold `Price:
Various` into one 95-mention junk org and `Shell` into one row for several
national companies — the 234 exclusion's tendsign shape at small scale, and
nothing in the carriers tells those apart from `PAPWORTH HOSPITAL NHS
TRUST`. A row cap does not help: the rows are in the 21–229 groups, where
the generics also are. What does tell them apart is a read of the string,
which the cohorts did at 400 names an hour by hand. So: verdicts at scale.
The census listing cap rises from 5,000 to 60,000 (the heap is bounded by
the cap; ~200 bytes a listed group, so a 25,000 listing is ~5 MB) and every
listed group carries its `shape`; the ~20k names of 21–229 rows then go to
agent-read batches of ~1,000 with the cohorts' rubric, POSTed as cohorts,
then dry, then wet (issue 351's rule: nothing posted between the two).

**Caveat.** `org_match_keys` still carries the rows the folds deleted
(foreign keys were off, so nothing cascaded, and no rebuild has run since);
the wall's carrier counts and this breakdown's `carriers` include them. The
weekly tick's rebuild on Sunday 2026-09-06 drops them; re-run the dry run
after it — some `id0/*` names may fall under the wall on their own.

## The wide census (job 675, 2026-09-04, 714 s, cap 25,000; `2cd8e3e`)

After job 674: 3,884,820 rows under 1,714,593 names; 56,012 names hold
more than one row (2,226,239 rows); sizes 2: 5,313 · 3–5: 6,461 · 6–20:
10,981 · 21–100: 28,991 · 101+: 4,266. The 25,000 largest (26 → 229 rows)
are all raw-wall `over-wall` bar 157 verdict-refused; 168 of them carry an
earlier `unclear` verdict (the census cannot show it — `unclear` falls to
the wall) and are excluded by the cutter. **24,675 names / 1,573,950 rows in
50 batches of 500**, by shape: `id0/c0` 14,945 · `id0/c1` 8,206 · `id0/c2+`
523 · `id1-cap/c2+` 389 · `id>cap/c1` 363 · `id>cap/c2+` 130 · `id1-cap/c1`
113 · `id1-cap/c0` 6. Ten readers, five batches each, answer by index only
(`pec-rubric.md`; `pec-batches.py` cuts, `pec-assemble.py` builds the POST
body from the census's stored names, so no name is ever retyped).

## Cohort 6 at scale (2026-09-04, evening)

Census job 675 (cap 25,000; 714 s) listed 24,843 raw-wall names; 168 of them
carry an earlier `unclear` verdict (a verdict that falls back to the wall
shows as `over-wall`, so the cutter excludes the cohort files by name).
24,675 candidates / 1,573,950 rows (26–229 rows each) went to ten reader
subagents in fifty index-keyed batches of 500 with the rubric
(`scratchpad/pec-rubric.md`). Pass 1: 21,507 `single` (1,364,328 rows) /
2,002 `unclear` / 624 `generic` / 511 `non-name` / 31 `platform` — the hand
cohorts' ratio. Assembled from the census's stored names by index
(`pec-assemble.py`), posted as `over-wall-6-2026-09-04` in five parts.

**Drift.** A spot check of the 31 `platform` verdicts found a Polish
ambulance station carrying "UK regional procurement portal URL": pass 1's
batch 13 had slipped one row somewhere before index 6887, so from there each
answer described the NEXT name — `www.supplyingthesouthwest.org.uk` was
posted `single` with a Greek ministry's rationale. A reader that writes 500
answers in one go can lose a row and nothing in the index-only protocol
catches it. The dry run planning on those verdicts (job 676) was cancelled
(the b71e809 guard kept the recorded plan untouched) and batch 13 was
re-posted as `unclear` (`353-batch13-hold.json`) until a verified read
replaces it.

**The fix in protocol, not in trust:** a second independent pass over every
batch, in chunks of 100, where each answer carries `echo` = the first twelve
characters of its name; `pec-verify.py` drops any answer whose echo does not
match the stored name, and merges the two passes — agreement keeps the
verdict, disagreement takes the more cautious one (`single` never wins over
a refusal or `unclear`), an index with only pass 1 becomes `unclear`. The
first pass-2 fleet (Fable) was cut off by a per-model rate limit after 17
batches; the remaining 33 run on Opus. What changes against the posted
cohort is re-posted (`changed.json`), then dry, then wet.

**Wet run (job 679, 1,692 s):** dry job 678 planned 21,235 groups /
1,329,491 rows (tiers: over-wall 33,243 · verdict-refused 1,534 ·
verdict-single 21,235); the wet run held parity and removed 1,329,491 rows
— 1,329,408 mentions, 1,383,354 parties, 454,816 winners repointed, 949,163
tenders touched, ~1,500 rows/s. Bounded reads after: `renfrewshire council
(abc)` 223 → 1 with its 223 mentions on the keep; `hessen mobil gelnhausen`
→ 1; `zaklad gospodarki mieszkaniowej` (generic) and
`www.supplyingthesouthwest.org.uk` (platform) untouched at 72 each. The
standing raw-wall class after the fold: 33,243 names / 607,472 rows
(`id0/c0` 18,134 / 293,112; `id0/c1` 12,065 / 193,117; `id1-cap/c2+` 910 /
49,027; `id0/c2+` 990 / 34,560; `id>cap/c2+` 378 / 20,718; `id>cap/c1` 526 /
14,172; `id1-cap/c1` 235 / 2,691; `id1-cap/c0` 5 / 75), all ≤26 rows.

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
