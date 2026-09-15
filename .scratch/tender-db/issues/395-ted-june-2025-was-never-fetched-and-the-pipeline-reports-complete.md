# 395 — TED June 2025 (~72k notices) was never fetched, and the funnel reports "fetch complete ✓" over the hole

Status: needs-triage — filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
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
