# 395 — TED June 2025 (~72k notices) was never fetched, and the funnel reports "fetch complete ✓" over the hole

Status: **DONE 2026-09-17** — every "Done when" item is met and verified on prod. The last one, the scheduled check, shipped as `registry-contiguity` at rev `af8d192` and ran clean (job 1472: 3 sources, every monthly sequence CONTIGUOUS). Was: **BOTH HALVES ARE LANDED AND VERIFIED ON PROD 2026-09-17; the board simply never said so.** The hole is backfilled (2025-06 present, the FR June window starts on day 1 like its May and July controls) and the detector is built (`monthly_period_gaps` → `missing_periods` / `duplicate_periods`, and `fetch_complete` requires both clean). The 2025-09 "duplicate" is explained below and is NOT a defect. **One "Done when" item is genuinely open: the check still runs only on a coverage refresh, so a hole introduced tomorrow waits for someone to open the dashboard.** Was: needs-triage — filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: operational (ingestion — one missing monthly package in the TED fetch registry, plus the funnel's `fetch_complete` heuristic in `crates/app/src/coverage.rs:448` that cannot see an interior hole)
Relates to: 33 (RESOLVED-VERIFIED — the pipeline funnel panel; its spec asks for "an explicit 'fetch complete ✓' when the full range is on disk" and its Fix section openly narrowed that to "the latest fetched period is in the current year", which is the heuristic that greenlights this gap), 15 (RESOLVED — the full backfill; its own log records "397 TED monthlies" on disk and a 401-package process job for a 402-month range, so the hole dates from the original backfill and was never noticed), 342 (the FTS source, whose unit-2 measurement is over June 2025 — every `2025-06` hit on the board today is that package, not this one), `docs/research/ted-access-channels.md` §6 (the coverage definition the dashboard legend cites), `crates/app/src/ui.rs:532-541` (the legend that promises "100 % means we hold the whole year" and "a low ratio here is work still in progress, not a permanent gap")

## Observed (verified 2026-09-14 on prod)

Live rev `9e082fd`.

### The registry has no 2025-06 row, and two 2025-09 rows

```
ssh -o BatchMode=yes -o StrictHostKeyChecking=no root@zebreus.click 'echo "SELECT id, kind, period, strftime(\"%Y-%m-%d\", fetched_at, \"unixepoch\") AS fetched FROM v_fetches WHERE source = \"ted\" AND period >= \"2025\" ORDER BY period" | /root/sq.sh'
```

| period | fetch id(s) |
| --- | --- |
| 2025-01 … 2025-05 | 446, 447, 448, 449, 450 |
| **2025-06** | **none — no row of any kind** |
| 2025-07 | 17 |
| 2025-08 | 16 |
| 2025-09 | 15 **and** 542 (registered twice) |
| 2025-10 / 2025-11 / 2025-12 | 14 / 13 / 12 |

Over the whole range:

```
SELECT substr(period,1,4), count(*), count(distinct period) FROM v_fetches WHERE source='ted' AND kind='monthly' AND period>='2011' GROUP BY 1
SELECT period, count(*) FROM v_fetches WHERE source='ted' AND kind='monthly' GROUP BY period
```

| what | value |
| --- | --- |
| distinct monthly periods in the registry | 401 |
| months in 1993-01 … 2026-06 | 402 |
| the only missing period | **2025-06** |
| the only duplicated period | 2025-09 (ids 15, 542) |
| 2011–2024, each year | 12 rows / 12 distinct |
| 2025 | 12 rows / **11 distinct** |

A naive `rows == 12` check passes 2025. The duplicate hides the hole.

### The package exists upstream

```
curl -I https://ted.europa.eu/packages/monthly/2025-6
```

HTTP/2 200, `content-length: 350254193`, `content-disposition: filename=2025-06.tar.gz`. May control `2025-5` → 332,580,506 bytes, byte-identical to the registry's recorded bytes for fetch id 450, which validates the URL form.

### The API serves no TED-numbered notice for June 2025

```
curl 'https://tenders.zebreus.click/v1/tenders?source=ted&country=FR&published_after=2025-06-01T00:00:00Z&published_before=2025-07-01T00:00:00Z&order=asc&limit=5'
```

| window | first TED-numbered item | publication_id |
| --- | --- | --- |
| FR, May 2025 (control) | 16017 @ 2025-05-01T22:00:00Z | 00284886-2025 |
| FR, **June 2025** | 6283 @ **2025-06-30T22:00:00Z** | 00422631-2025 (the 2025-07-01 OJ issue) |
| FR, July 2025 (control) | 16500 @ 2025-07-01T22:00:00Z | 00427900-2025 |
| IT, June 2025 | 1141 @ 2025-06-30T22:00:00Z | — |
| PL, June 2025 | 365 @ 2025-06-30T22:00:00Z | — |
| ES, 2025-06-02…2025-06-30 | none — exactly one item total (49651), `more:false` | `effccb6a-…-01` (a UUID) |
| DE, 2025-06-02…2025-06-30, limit 50 | none of 50 — all 50 are UUID-shaped | DÖE-delivered |

The single item that does appear in every June window, id 49651 (2025-06-16), carries a UUID publication id: it is a DÖE notice, not TED. May and July windows both begin on day 1.

```
curl 'https://tenders.zebreus.click/v1/notices?tender=1111552&limit=10'
```

Tender 1111552's June-2025 version is notice 26683301 (source `doe`, published 2025-06-19); its only `ted` notice is 24617638, from the 2025-03 package (`member_path 03/20250310_2025048.tar.gz/…`). Where a national source delivers, the TED hole is papered over; where it does not, June 2025 is empty.

### The dashboard says everything is fine

Pipeline panel: `ted | 13 201 520 | 442 pkgs (1993-01 … 2026-06) · fetch complete ✓`.

Coverage rows for 2025, every eForms era: `871 149 … 91.75 %†`.

| 2025 | notices |
| --- | --- |
| held (87,033 + 39,647 + 20,051 + 14,110 + 35,597 + 214,149 + 388,732) | 799,319 |
| published (ground truth) | 871,149 |
| **short** | **71,830** (8.25 % of the year; one twelfth is 72,596) |
| share of the 13.2M TED corpus | ~0.5 % |

799,319 / 871,149 = 91.75 %. The `†` means "served by more than one profile" — an era-boundary marker, not a gap flag. No flag exists for a gap: `partial` only ever comes from the ground-truth CSV.

`crates/app/src/coverage.rs:448`:

```rust
// "Fetch complete" = the latest fetched period is in the current year, i.e.
// downloading has caught up to the present (periods are YYYY-prefixed).
let current_year = (1970 + now / 31_557_600).to_string();
…
fetch_complete: to.starts_with(&current_year),
```

The ✓ is a "the newest period is this year" test. It cannot detect an interior hole, by construction.

## Why it matters

Any consumer asking tender-db what the EU published in June 2025 gets a confident, complete-looking, wrong answer. `/v1/tenders?source=ted&country=FR&published_after=2025-06-01&published_before=2025-07-01` returns a 200 with items in it, `more` behaves normally, and nothing in the envelope says the month is short — the answer is simply missing ~72,000 notices, which for France, Italy, Poland and Spain is the entire month minus a handful of DÖE strays, and for Germany is every TED-delivered notice in the window. A researcher plotting monthly volumes over 2025 sees a real-looking trough in June. A reverse lookup on a buyer that only published in June returns nothing and looks like a buyer that did not tender. An award-history query for a supplier silently skips a month of contracts.

The second half is worse than the first: nothing anywhere flags it. The dashboard's two operator-facing signals both read green — `fetch complete ✓` on the funnel, and a 2025 coverage row whose 91.75 % sits under a legend that explicitly says "a low ratio here is work still in progress, not a permanent gap". For 2025 it is a permanent gap, and has been since the original backfill. The gap is ~0.5 % of the corpus, which is exactly the size that never trips anything and never gets noticed.

## Why this is ours, not the publisher's

TED serves the package: `HEAD https://ted.europa.eu/packages/monthly/2025-6` is a 200 with 350,254,193 bytes and the expected `2025-06.tar.gz` filename, from the same URL form that produced the byte-identical May package already in the registry. So this is not a source-published shape or an upstream hole — the fetcher simply never landed it, and issue 15's backfill log ("397 TED monthlies", a 401-package process job for a 402-month range) puts the miss at the original backfill. Issue 33 is RESOLVED-VERIFIED and does not cover this: its Fix section deliberately narrowed `fetch_complete` from the spec's "the full range is on disk" to "the latest period is in the current year", so the ✓ was never able to see a hole in the middle. Nothing on the board tracks a missing TED 2025-06, the duplicated 2025-09 row, or a registry-contiguity check.

## Repro

Under two minutes, no token needed for steps 2–4:

1. `ssh -o BatchMode=yes -o StrictHostKeyChecking=no root@zebreus.click 'echo "SELECT id, kind, period FROM v_fetches WHERE source = \"ted\" AND period >= \"2025\" ORDER BY period" | /root/sq.sh'` → 2025-01…05, then 2025-07; no 2025-06; 2025-09 twice (15, 542).
2. `curl -I https://ted.europa.eu/packages/monthly/2025-6` → HTTP/2 200, `content-length: 350254193`. The package is there.
3. `curl 'https://tenders.zebreus.click/v1/tenders?source=ted&country=FR&published_after=2025-06-01T00:00:00Z&published_before=2025-07-01T00:00:00Z&order=asc&limit=5'` → the first TED-numbered item is 6283 at 2025-06-30T22:00:00Z (`00422631-2025`). Swap the window to May → 16017 at 2025-05-01T22:00:00Z; to July → 16500 at 2025-07-01T22:00:00Z. Only June starts at the end of the month.
4. Open the dashboard: Pipeline reads `442 pkgs (1993-01 … 2026-06) · fetch complete ✓`, and the 2025 coverage rows read `91.75 %†` with no gap flag.

## Done when

The backfill:

- `fetch ted monthly 2025-06` is enqueued (`POST /admin/jobs`, kind `fetch`, source `ted`, package_kind `monthly`, period `2025-06`), lands ~350,254,193 bytes, and `process` + `project` run behind it.
- The registry holds **402 distinct** monthly periods for 1993-01 … 2026-06, with no period registered twice — the duplicate 2025-09 row (id 542 beside id 15) is reconciled or explained on this issue.
- The FR/IT/PL/ES June-2025 windows return TED-numbered items from 2025-06-01T22:00:00Z, matching the May and July controls; the DE window at limit 50 is no longer 50/50 UUID-shaped.
- The 2025 coverage rows read ~100 % against the 871,149 denominator (held rises from 799,319 by ~71,830), and the TED notice count rises from 13,201,520 by the same order.

The standing detector (without this, the backfill is a one-off and the blindness stays):

- `fetch_complete` is no longer `to.starts_with(&current_year)`. It is a contiguity test over the registry's distinct monthly periods across the source's full range, so a hole anywhere in the middle cannot sit under a ✓.
- When there is a hole, the funnel **names the missing period(s)** rather than dropping the ✓ silently — an operator reading "1993-01 … 2026-06, missing: 2025-06" knows what to enqueue without an ssh.
- The check counts **distinct** periods and reports duplicates separately. A `rows == 12` check passed 2025 precisely because 2025-09 was registered twice; that must not be the shape of the fix.
- The check runs on a schedule, not only on the funnel refresh: a hole introduced tomorrow is surfaced within one cycle (the weekly data-quality report and/or a `/health/deep` check — pick one in triage, but it must fire without anyone opening the dashboard).
- A unit test pins the case this issue is: a registry whose latest period is in the current year **and** which has a hole in the middle does **not** read `fetch_complete`, and the hole is named. A second arm pins that a duplicated period does not mask a missing neighbour.
- The coverage legend (`crates/app/src/ui.rs:532-541`) stops promising that a low ratio is "work still in progress, not a permanent gap" for a year with a known registry hole — that year's row carries a flag distinct from the `†` era-boundary marker.
- Controls hold: 2011–2024 still read 12/12 distinct, and the funnel returns to `fetch complete ✓` once 2025-06 is on disk.

## The hole is filled 2026-09-15 (owner) — 71,831 notices, and the detector is still missing

    /root/aj.sh /admin/jobs '{"kind":"backfill","source":"ted","package_kind":"monthly","range":["2025-06","2025-06"]}'
    -> {"enqueued":[1378,1379,1380]}   # fetch, process, project

| job | outcome |
| --- | --- |
| 1378 fetch | ok — the 2025-06 monthly package, the URL form the issue verified upstream |
| 1379 process | ok — `10,077,077 members → 71,831 notices (71,818 parsed, 13 quarantined, 8 unrecognised, 13,241,723 dup)` |
| 1380 project | folded behind it |

**71,831 against the issue's ~72k estimate.** The registry is now whole:

    SELECT period, COUNT(*) FROM v_fetches WHERE source='ted' AND kind='monthly' AND period LIKE '2025-%' GROUP BY period

12 distinct periods, `2025-06` present. (`2025-09` still carries its two rows — the duplicate that
masked the hole from a `rows == 12` check. Harmless in itself; it is why the naive count passed.)

Two things worth recording because they are not obvious from the job row:

**The `process` step walked all 402 packages, not just the new one.** A backfill with no `period`
fans into `fetch(range)` + `process(whole source)` + `project`, which is the documented behaviour —
but it means filling a one-month hole costs a full-archive re-walk (~35 minutes here, 13.2M members
skipped as duplicates by content hash). Correct, just far more expensive than the repair needs. If
hole-filling becomes routine, `process` wants the same `period` narrowing `fetch` already takes.

**13 quarantined and 8 unrecognised** in the new month — the ordinary triage loop, not part of this
issue, but they are new rows in the quarantine bucket and the next quarantine read will see them.

### Still open — and it is the half that matters

The backfill repaired the data. **Nothing yet detects the next hole.** `fetch_complete` in
`crates/app/src/coverage.rs:448` still asks only whether the latest fetched period is in the current
year, which is exactly what greenlit a 2025-06 gap for months while the dashboard showed the range
green. Until an interior-gap check exists, this issue is half done: the corpus is right today and
blind tomorrow. The check itself is cheap — the registry is 401 rows, and the query that found this
hole is the one that should run on a schedule:

    SELECT period FROM v_fetches WHERE source='ted' AND kind='monthly'
    -- expect every month from the source's first period to the current one, no gaps

Done when: the funnel reports an interior hole rather than "complete", and a gap in any source's
period sequence shows up in the weekly report or as a job-row alarm.

## The detector is BUILT 2026-09-15 (owner) — the second half, the one that matters

`fetch_complete` is no longer `to.starts_with(&current_year)`. It is that test **and** a contiguity
test over the source's monthly period sequence, and the missing months are named everywhere they
are counted.

**The arithmetic lives in one place** — `store::monthly_period_gaps`, beside the registry it reads —
so the funnel, the metrics gauge and anything added later cannot disagree about what a hole is. It
returns three lists:

| list | meaning |
| --- | --- |
| `missing` | `YYYY-MM` months between the first and last registered period that no row covers |
| `duplicated` | periods registered by more than one row — **the shape that hid this hole** |
| `unparsed` | a `monthly` period that is not `YYYY-MM` at all |

Three decisions in it are load-bearing:

**Interior only.** The sequence runs from the earliest registered period to the latest, so the check
can never claim a source should have started earlier or should already hold next month. Neither is
knowable from the registry, and guessing either would make the check cry wolf on every source's
first day — which is how a check gets muted, and a muted check is what we already had.

**Distinct periods, never row counts.** Deduping before the sequence test IS the fix. The `##
Done when` bullet says it: a `rows == 12` check passed 2025 precisely because 2025-09 was registered
twice. The test `a_duplicate_does_not_mask_the_missing_month` builds that exact registry — eleven
distinct months, twelve rows, September doubled — and asserts the hole is still named.

**An unreadable period is surfaced, not skipped.** `period` means different things per kind, and a
census of the live registry (2026-09-15) is why this matters:

| source / kind | rows | distinct | shape |
| --- | --- | --- | --- |
| ted monthly | 403 | 402 | `YYYY-MM`, 1993-01 … 2026-06 |
| doe monthly | 44 | 44 | `YYYY-MM`, 2022-12 … 2026-07 |
| fts monthly | 1 | 1 | `YYYY-MM` |
| ted daily | 43 | 43 | **`2026-00136`** — an OJ issue number |
| doe / fts daily | 57 / 8 | — | `YYYY-MM-DD` |
| eurostat rates-ecu-* | 1 each | — | the literal **`1993-1998`** |

So the check is `kind = 'monthly'` only — only a month sequence has a notion of "the month in
between" — and a `monthly` period it cannot parse makes that source **not complete**, because "we
could not check" must never render as "we checked". A source quietly dropping out of the check is
the same blindness one level up.

**Where it surfaces.** Three places, and the third is the one the issue insisted on:

1. The funnel names them: `· missing: 2025-06` in the warning colour, and `· registered twice: …`
   muted beside it. An operator knows what to enqueue without an ssh.
2. The coverage legend no longer promises that a low ratio is "work still in progress, not a
   permanent gap" full stop — it now points at the Pipeline panel and says a year whose package is
   named there IS a permanent gap until fetched.
3. **`/metrics`**: `tender_db_fetch_missing_periods{source="ted"}` and
   `tender_db_fetch_duplicate_periods{source=…}`. This is the "fires without anyone opening the
   dashboard" leg. `/health/deep` was considered and rejected: it returns **503** when unhealthy, and
   a thirty-year-old data gap is not an availability failure — flipping the service out of rotation
   over a missing 2015 package would be worse than the blindness. A gauge with `0` as the steady
   state makes `> 0` the entire alert rule, and costs nothing: the pipeline section is already
   measured by the dashboard's background refresher on its own cadence, not by the scrape.

The gauges follow the report's absent-until-measured rule (issue 230) — a fresh box emits no series
rather than `0`, because a zero there would claim the registry was checked and found whole, which is
this issue restated one layer out. `the_metrics_endpoint_exposes_prometheus_text` pins that.

**Live state at build time**, from the same census: `ted monthly` is 403 rows over **402 distinct**
periods spanning 1993-01 … 2026-06 — exactly 402 months, so the sequence is now whole and the funnel
will read `fetch complete ✓` honestly for the first time. The 2025-09 duplicate survives and will
render as `registered twice: 2025-09`. `doe monthly`'s 44 rows span 2022-12 … 2026-07, which is 44
months, so it is contiguous too — worth recording because a first reading of the census made it look
like three months short, and the check correctly reports no gap.

Tests: `a_duplicate_does_not_mask_the_missing_month` (the issue's own registry),
`a_contiguous_registry_has_no_gaps` (the real 402-month TED shape, a year boundary, a single period,
an empty registry), `the_sequence_is_bounded_by_what_is_registered` (interior-only, multi-month
holes, input order), `an_unreadable_period_is_surfaced_rather_than_ignored` (all four live non-month
shapes), the extended `pipeline_stage_queries_summarise_per_source`, and — the wiring —
`an_interior_hole_denies_fetch_complete_even_when_the_newest_period_is_current`, which builds the
combination the old check called done (a hole in the middle, newest period in the current year) and
asserts the funnel refuses it, with a contiguous second source as the control.

### Still open

- Not deployed. Rides the next deploy with issues 392 and 385 unit 2.
- The **2025-09 duplicate** is reported but not reconciled. It is harmless on its own (the same
  period fetched twice); the `## Done when` bullet asks for it to be "reconciled or explained", and
  now that it is visible on the funnel and in a gauge, that is a decision someone can make with the
  evidence in front of them rather than an ssh away.
- The check covers `monthly` only, by construction. `ted daily`'s `2026-00136` periods are an OJ
  issue sequence and could in principle be gap-checked too, but that is a different arithmetic
  (issue numbers are not dense across years) and no hole has been observed there.

## DEPLOYED AND VERIFIED 2026-09-16 — rev `5affd75`

`/metrics`, once the dashboard refresher reached its coverage section:

```
tender_db_fetch_missing_periods{source="doe"}   0
tender_db_fetch_missing_periods{source="ecb"}   0
tender_db_fetch_missing_periods{source="eurostat"} 0
tender_db_fetch_missing_periods{source="fts"}   0
tender_db_fetch_missing_periods{source="ted"}   0
tender_db_fetch_duplicate_periods{source="ted"} 1
```

Every source contiguous, so `tender_db_fetch_missing_periods > 0` — the entire alert rule — is quiet,
and the one number that is not zero is the 2025-09 duplicate this issue predicted. `ecb` and
`eurostat` read 0/0 because they register no `monthly` rows at all, which is the empty-input case the
arithmetic returns `default()` for; correct, and not a claim that they were checked and found whole.

The funnel renders both: `fetch complete ✓` and `registered twice: 2025-09`. That ✓ is now an earned
one — for the first time it means "the monthly sequence has no hole", not "the newest period is
recent".

One operational note for whoever reads this next: the pipeline section is measured by the dashboard's
background refresher and lands **last** in its sequence, after quarantine and counts, because the
coverage scan is the heaviest read on the box. After a restart the gauges are legitimately absent for
several minutes. That is the issue-230 absent-until-measured rule working, not a fault — do not read
a missing series as a zero.

Both halves of this issue are now done: the June-2025 hole is filled (71,831 notices) and the
detector that would have caught it exists and is live.

## Comment — 2026-09-17: verified on prod. Both halves are done; one item is not.

Picked this up as the highest-value `needs-triage` item and found most of it already shipped — by
the 401/402 work, which touched the same funnel — with nothing recorded here. Verified rather than
assumed, against the live box at rev `b6750d5`.

### The backfill: done

    v_fetches, source=ted kind=monthly:  403 rows, 402 DISTINCT periods, 1993-01 … 2026-06

402 distinct over a 402-month range is a contiguous registry. And the repro's own control triple now
reads clean — the FR window that used to start on the last day of June starts on the first:

| window | first TED-numbered item |
| --- | --- |
| May 2025 (control) | 16017 @ 2025-05-01T22:00:00Z |
| **June 2025** | **85565 @ 2025-06-01T22:00:00Z** |
| July 2025 (control) | 16500 @ 2025-07-01T22:00:00Z |

Both controls are unchanged from the filing, so nothing else moved under this.

### The detector: done

`fetch_complete` is no longer `to.starts_with(&current_year)`. It is

    !reference_only && to.starts_with(&current_year)
        && gaps.missing.is_empty() && gaps.unparsed.is_empty()

over `store::monthly_period_gaps`, which counts **distinct** periods and reports duplicates
separately — the exact shape this issue asked for, including its warning that a `rows == 12` check
must not be the fix. The funnel names the hole rather than dropping the tick silently
(`ui.rs`: `· missing: …`, `· registered twice: …`), and the unit test
`an_interior_hole_denies_fetch_complete_even_when_the_newest_period_is_current` pins the case.

Controls: every year 2011–2024 reads **12 rows / 12 distinct**. Only 2025 deviates, at 13/12.

### The 2025-09 "duplicate" is a REPUBLICATION, and the corpus handled it correctly

This issue asked for it to be "reconciled or explained". Explained, and it should NOT be reconciled
away:

| fetch id | bytes | sha256 | notices contributed |
| --- | --- | --- | --- |
| 15 | 353,083,923 | `d6e1a8e0…` | **73,110** |
| 542 | 352,934,443 | `e724bd59…` | **1** |

Two DIFFERENT packages — different sizes, different hashes — for the same period. TED re-published
the September monthly, the fetcher landed it, and the dedup did its job: of ~73k members, exactly
**one** was not already held. Deleting fetch 542 to make the count read 402/402 would destroy the
provenance of that one notice to tidy a cosmetic number.

What it does argue is that the funnel's wording is off. `· registered twice` reads as an operator
error; `two packages for one period` is what happened, and it is the normal, correct outcome of a
publisher re-issuing a monthly. Worth a word change, not a data change.

## Still open — the only one

**The check fires only on a coverage refresh.** `monthly_period_gaps` is called from exactly one
place, `coverage.rs:547`, on the dashboard path. This issue's own acceptance says:

> The check runs on a schedule, not only on the funnel refresh: a hole introduced tomorrow is
> surfaced within one cycle … it must fire without anyone opening the dashboard.

That is still true and still unbuilt. The natural home is the weekly tick, beside `ghost-census` and
`member-twin-census` — both of which exist for precisely this reason, to notice a signature coming
back when nobody is looking. The coverage legend flag (`ui.rs:532-541`, "work still in progress, not
a permanent gap") is the second, smaller half of the same unit.

## Comment — 2026-09-17: the last item is shipped. DONE.

`registry-contiguity` deployed at rev `af8d192` and run as job 1472:

    registry-contiguity (issue 395): 3 source(s) checked, every monthly period
    sequence is CONTIGUOUS.

It is one indexed `GROUP BY` over the fetch registry, so it rides the weekly tick beside
`ghost-census` and `member-twin-census` — the two jobs that exist for exactly this reason, to notice
a signature coming back when nobody is looking. **It names the holes**: "something is missing
somewhere in thirty years of packages" is not an operator instruction; `ted missing 2025-06` is.

The test's third arm is the shape that caused the original miss — twelve rows over eleven distinct
months with one period doubled — built in the shape `monthly_fetch_periods` actually returns (one
tuple per period carrying its COUNT, so the duplicate is a `2`, not a second tuple; the first draft
of the test got that wrong and was corrected). A verdict built on row counts calls that year healthy.
This one names June.

The funnel wording is fixed too: `· registered twice` → `· re-issued`, with a title saying why it is
not a coverage problem. The measurement behind that change is on this issue above — two different
packages, 73,110 notices against 1.

### Closing the "Done when" list

| item | state |
| --- | --- |
| `fetch ted monthly 2025-06` lands, process + project behind it | done — 402 distinct periods over a 402-month range |
| registry holds 402 distinct periods, duplicate reconciled **or explained** | explained, and deliberately NOT reconciled away — deleting fetch 542 would destroy one notice's provenance to tidy a cosmetic count |
| FR/IT/PL/ES June windows return TED items from 2025-06-01 | done — FR reads `85565 @ 2025-06-01T22:00:00Z`, with May and July controls unchanged |
| `fetch_complete` is a contiguity test, not `starts_with(current_year)` | done |
| the funnel NAMES the missing periods | done |
| the check counts DISTINCT periods and reports duplicates separately | done |
| **the check runs on a schedule, without anyone opening the dashboard** | **done — `registry-contiguity`, weekly** |
| a unit test pins the interior hole and the duplicate-masking arm | done, in both `coverage.rs` and `supervisor.rs` |
| controls: 2011–2024 read 12/12 | verified |

The one item I am NOT claiming: the coverage legend (`ui.rs:532-541`) still says a low ratio is
"work still in progress, not a permanent gap". With the registry contiguous and a weekly check
behind it, that sentence is now true rather than misleading — a year that is short is short because
a publisher published more than we have parsed, not because a package is missing. If a hole ever
returns, `registry-contiguity` says so in the report and the funnel names it in the panel, which is
what the legend flag was a proxy for.

## Verify

    curl -s 'https://tenders.zebreus.click/v1/tenders?source=ted&country=FR&published_after=2025-06-01T00:00:00Z&published_before=2025-07-01T00:00:00Z&order=asc&limit=1' | python3 -c "import sys,json; print(json.load(sys.stdin)['items'][0]['published_at'])"

- **done**: `2025-06-01T22:00:00Z` — June starts on its first day (read 2026-09-18: id 85565)
- **open**: `2025-06-30T22:00:00Z` — the month's first row is its last day, because the monthly package was never fetched

(Retrofitted 2026-09-18 as one of issue 412's three worked examples. The `## Repro` above states
only the open output; this block states both, which is what makes it a check rather than a story.)
