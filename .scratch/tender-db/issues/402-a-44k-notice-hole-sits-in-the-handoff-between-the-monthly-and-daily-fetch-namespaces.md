# 402 — ~44,600 TED notices are missing from 2026-06-30 to 2026-07-15, in the seam between the monthly and the daily fetch namespaces, and every guard on the board reads green over it

Status: ready-for-agent — **HALF (a) IS DONE AND VERIFIED ON PROD 2026-09-17**: the twelve OJ S
dailies 2026-00124…00135 are fetched, processed and folded (job 2352: **44,854 notices → 40,848
tenders, 85,747 versions**, then job 2353 rebuilt the deferred indexes), and the window that served
ZERO now serves on every day sampled. **Unit B's DETECTOR half is fixed and gated 2026-09-17** — the continuity window was the newest
2M NOTICE IDS, i.e. ingest order, and this hole ended exactly on that window's edge where
`publication_gaps` (interior-only, by design) can never see it; it is now 550 days of PUBLICATION
time, red-checked. **What remains is `fetched_to`'s lexical `MAX(period)` across two namespaces**
(`store/src/lib.rs:3702`), located and written up but not fixed — see the 2026-09-17 comment. Was: found 2026-09-16 by the hourly audit (step 3) on prod rev `19010b8`. The hole is MEASURED, not inferred: zero notices in the window, on both sides of which the corpus publishes ~3,720/day. The fix has two halves — fetch the missing issues, and make the seam checkable — and the second is the one that matters, because nothing on the board can currently see a hole in this position.
Kind: defect (coverage — the fetch plan's monthly→daily handoff, and the completeness verdict in `crates/app/src/coverage.rs` / `store::monthly_period_gaps`)
Relates to: 395 (RESOLVED 2026-09-15 — it built `monthly_period_gaps` exactly to catch a fetch hole the ✓ was hiding, and it CANNOT see this one: its sequence test is monthly-only and interior-only by design, and this hole is at the boundary between two period namespaces, which is neither), 396 (RESOLVED-VERIFIED 2026-09-16 — its 2026 SURPLUS and this DEFICIT are in the same coverage cell and cancel: 2026 reads 118.74 % because the denominator is a 2026-07-17 snapshot while the held count runs to 2026-09-11, so ~44,600 missing notices are invisible under an over-100 % ratio), 15 (RESOLVED 2026-08-16 — the backfill that set the fetch plan, and where OJ S issue numbering is pinned), 33 (the funnel panel), 401 (filed the same hour — the same panel, a different way its cells mislead), 06 (the ground truth the coverage ratio divides by)
Blocked by: nothing

## The measurement

    -- the window, on prod
    SELECT count(*) FROM notices
     WHERE source='ted' AND published_at >= 1782777600 AND published_at < 1784160000
    -> 0          -- 2026-06-30 00:00Z .. 2026-07-16 00:00Z

Zero. Both sides are ordinary:

| window | notices | publication days | first | last |
| --- | --- | --- | --- | --- |
| June 2026 | **74,415** | 20 | 2026-06-01 | **2026-06-29** |
| **2026-06-30 … 2026-07-15** | **0** | **0** | — | — |
| from 2026-07-16 | resumes at 3,722 that day | — | **2026-07-16** | … |

TED publishes Sunday–Thursday (calibrated on a known-good fortnight: 07-27…07-30 present, 07-31
and 08-01 absent, 08-02…08-06 present, 08-07/08-08 absent, 08-09 present — the absent pairs are
Fri/Sat, every week). The gap spans **12 publication days**: 06-30, 07-01, 07-02, 07-05…07-09,
07-12…07-15. At June's measured rate of 74,415 / 20 = **3,720 notices per publication day**, that is

> **≈ 44,600 missing notices.**

Comparable in size to issue 395's 2025-06 hole (~72,000), and found the same way — by not trusting a ✓.

## The mechanism: a seam between two period namespaces

`fetches.period` carries two different namespaces for `ted`:

| kind | packages | range |
| --- | --- | --- |
| monthly | 403 | `1993-01` … **`2026-06`** |
| daily | 43 | **`2026-00136`** … `2026-00178` |

Both were established in one sitting: every 2026 monthly (`2026-01`…`2026-06`, 344–414 MB each) was
fetched on **2026-07-19**, and the daily series starts at issue **136** and was fetched from
2026-07-19 onward. Issue 136 is 2026-07-16 — exactly where the corpus resumes.

So the monthly coverage ends at 2026-06-29 and the daily coverage begins at 2026-07-16, and **nothing
holds the OJ S issues in between** (roughly 2026-00125 … 2026-00135). The daily series itself is
contiguous — 136…178 with no interior gap — so the defect is entirely in where it was STARTED.

Which of two readings is true decides how it is fixed, and this issue does not yet settle it:

- **(a) a lag.** TED's monthly package for a month is published later, and the issues from 06-30 to
  07-15 will arrive inside a `2026-07` monthly package that does not exist yet. Then the hole closes
  itself on the next monthly fetch and the standing defect is only that `fetch complete ✓` claimed
  otherwise.
- **(b) a permanent hole.** The daily fetcher's start issue was chosen without checking where the
  monthly coverage actually ended, and only a daily backfill of issues ~125–135 closes it.

Evidence leans (b): the June package ends at **06-29**, one publication day short of the month it is
named for, which is what a build-time cut-off looks like rather than a clean calendar boundary — and
under (a) the missing 06-30 would have to live in a July package, meaning the monthly packages are
not month-aligned at all. Settle it by asking TED for a `2026-07` monthly and by probing issues
125–135 directly; both are cheap.

## Why every guard reads green over it

This is the part that matters more than the 44,600 notices, because the notices can be fetched and
the blindness will otherwise hide the next one:

| guard | verdict | why it cannot see this |
| --- | --- | --- |
| Pipeline `fetch complete ✓` | **✓** | `fetched_to` is `MAX(period)` — a LEXICAL max over both namespaces. `'2026-06' > '2026-00136'`, so the newest period reads `2026-06`, which `starts_with("2026")`, so the year test passes |
| `monthly_period_gaps` (issue 395) | no missing periods | it walks the MONTHLY sequence and reports INTERIOR holes only, both deliberate. A hole after the last monthly period is not interior, and a hole in the daily namespace is not monthly |
| the Coverage grid, 2026 | **118.74 % ✱‡** | issue 396's dated snapshot: the denominator is 497,791 counted through 2026-07-17, the held count runs through 2026-09-11, and the surplus from two extra months **exceeds** the 44,600 deficit. The cell reads over-complete while being under-complete |
| `/metrics`, the DQ report | silent | neither measures publication-day continuity |

Three independent completeness signals, all green, over a 12-day blackout. The Pipeline panel even
displays the evidence — `445 pkgs (1993-01 … 2026-06) · fetch complete ✓` on 2026-09-16, a range
ending 2½ months ago beside a tick claiming caught-up — and nothing reconciles the two halves of its
own cell.

## Why this is ours, not the publisher's

TED published those issues; the archive simply was never asked for them. The fetch plan was set in
one sitting on 2026-07-19 and the two halves were chosen independently: monthlies up to the newest
available, dailies from a start issue, with no check that the second begins where the first ends.

## Repro

1. `SELECT count(*) FROM notices WHERE source='ted' AND published_at >= 1782777600 AND published_at < 1784160000` → **0**.
2. The same query per day over 2026-06-27…2026-07-25 → 06-28, 06-29, then nothing until 07-16.
3. `SELECT kind, count(*), min(period), max(period) FROM v_fetches WHERE source='ted' GROUP BY kind`
   → `daily 43 2026-00136 2026-00178`, `monthly 403 1993-01 2026-06`.
4. `SELECT period, bytes, fetched FROM v_fetches WHERE source='ted' AND kind='monthly' AND period LIKE '2026-%'`
   → six rows, `2026-01`…`2026-06`, all fetched 2026-07-19.
5. `curl https://tenders.zebreus.click/` → Pipeline: `ted … 445 pkgs (1993-01 … 2026-06) · fetch complete ✓`.

## Done when

- **The gap is closed.** The missing OJ S issues are fetched and processed, and the per-day count over
  2026-06-30 … 2026-07-15 is non-zero on every publication day (Sun–Thu), at a rate consistent with
  the ~3,720/day either side. Whether that came from a `2026-07` monthly or from a daily backfill of
  125–135 is recorded here, because it settles (a) vs (b) for every future handoff.
- **The seam is checkable.** The completeness verdict stops resting on a lexical `MAX(period)` over
  mixed namespaces. The shape that would have caught this: derive coverage from what is HELD rather
  than from what was fetched — a publication-day continuity test over `notices.published_at` for the
  current era, which is namespace-agnostic and would have flagged 12 consecutive silent days
  immediately. Issue 395 built the monthly half of this; this is the other half, and the two should
  end up as one check rather than two.
- **`fetched_to` stops lying about the range.** `445 pkgs (1993-01 … 2026-06)` while dailies run to
  2026-09-11 understates by 2½ months. Either the range spans both namespaces, or the cell names them
  separately.
- A test pins the class: a fetch registry whose newest MONTHLY period is followed by a daily series
  that does not start at the next issue is not `fetch_complete`. Fixture-level, not corpus-level.
- Re-read after the fix: the 2026 coverage cell no longer nets a surplus against a deficit — with the
  gap filled, 2026 held rises by ~44,600 and the ratio against the dated 497,791 snapshot rises with
  it, which is CORRECT and is exactly why issue 396's `‡` had to land first for this number to be
  readable at all.
- 395's `duplicate_periods` entry for `2025-09` is still live and still unreconciled; not this issue's,
  but re-check it in the same pass since both are fetch-registry hygiene.

## The DURABLE half BUILT 2026-09-16 — DQ report section 14, publication-day continuity

Status: the detector is built and gated (`GATE-EXIT=0`), not yet deployed. The gap itself is NOT yet
closed — that is the other half and it needs the queue idle.

### Where it lives, and why not the dashboard

The `## Done when` asked for "a publication-day continuity test over `notices.published_at` … which is
namespace-agnostic and would have flagged 12 consecutive silent days immediately". It is **weekly DQ
report section 14**, not a dashboard panel, and the reason is cost measured rather than assumed:
`notices.published_at` carries **no index** (the table has `notices_profile`, `notices_parse_state`,
`notices_fetch_id` and nothing else), so any predicate on it is a full scan of 14.4M rows. The
dashboard's coverage section already pays for one such scan per refresh; adding a second would double
that section's cost forever, to detect something that does not need hourly resolution — a 12-day hole
is not a thing that opens and closes between two weekly runs.

The report bounds it the same way section 13 does: a NOTICE-ID window
(`id > MAX(id) - PUBLICATION_GAP_WINDOW_IDS`, 2,000,000 ≈ the last five months at the head), which is
a primary-key range rather than a scan.

### The threshold is calibrated, and the calibration is in the code

`PUBLICATION_GAP_MIN_DAYS = 4`. TED publishes **Sunday–Thursday** — measured on the known-good
fortnight 2026-07-27…2026-08-09, where the absent days come in Fri/Sat pairs every week and every
other day carries 3,247–3,752 notices. So a normal weekend is 2 silent days, a weekend plus a public
holiday is 3, and 4 is the first length that cannot be the calendar. This issue's hole was **12
publication days**, so the threshold has an order of magnitude of headroom over the noise and is
nowhere near the signal. The test pins exactly that: the real fortnight produces no gaps, a 3-day
silence produces none, a 4-day one produces one.

### `before` / `after` are the diagnosis, not decoration

Each row carries the notice counts on the days BRACKETING the stretch. Both being ordinary is what
makes a stretch a hole rather than the beginning or end of a source's life — this issue in two
numbers, 3,470 the day before and 3,722 the day after, nothing in between. The renderer says so.

Interior-only by construction: a stretch exists only BETWEEN two days that both carry notices, so the
window's own edges can never be reported. That is the same discipline issue 395 applied to monthly
periods — and the reason 395 could not catch this is that this hole is interior to the NOTICES and
exterior to the monthly sequence. Measuring what is HELD is what makes it namespace-agnostic.

### Two things the codebase caught, worth recording

- **The "deployed but inert" guard fired on me.** `every_report_field_is_read_by_the_renderer` failed
  with `pub fn render_json( never reads these Report fields, so their queries cost their scan every
  week and show that consumer nothing: ["publication_gaps"]`. That guard exists because section 13
  shipped in exactly that state and ran inert for weeks. It works. `render_json` now emits
  `publication_gaps`, present even when empty — an absent key and a healthy corpus are the same thing
  to a JSON consumer, and this section's whole point is that silence was being read as health.
- The civil-day round trip the fold rests on has its own test across leap years, century boundaries
  and this issue's own span, because a silent off-by-one would mis-name every stretch by a day.

### Still owed

- **The gap itself.** Nothing is fetched yet. Settle (a) lag vs (b) permanent hole by asking TED for a
  `2026-07` monthly and by probing daily issues ~125–135; then fetch and process.
- Once deployed, run the DQ report and confirm section 14 names this stretch — the detector has never
  been run against the corpus, only against fixtures.
- `fetched_to` still understates TED's range as `2026-06` (a lexical MAX across namespaces). Not
  touched by this unit.

## 2026-09-16, first run against the corpus: section 14 is NOISE, and the window is why

Status: REGRESSION in my own unit, found by running the acceptance instead of assuming it. The section
was deployed at 09:58Z in `1bd7dc1`; job 2324 (11:35Z, 5,277 s) is the first data-quality run that
computed it against the real corpus rather than fixtures. **Every one of the 37 stretches it printed is
an artifact of the window, the two TED entries are false alarms, and the hole this section exists to
find is not among them.**

### What it printed

    ted   2025-06-30   2025-09-24    87 days   before 3,765   after 1
    ted   2025-09-26   2026-09-14   354 days   before 1       after 3,534
    doe   … 35 further stretches, 2025-05-19 through 2026-07-19 …

The 2026-06-30…2026-07-15 stretch this issue was filed for is not named; it is swallowed inside the
354-day entry, indistinguishable from 11 months of ordinary publishing.

### Why — the window does not sample publication time at all

`publication_days_sql` takes `id > (SELECT MAX(id) FROM notices) - 2000000`, and the doc comment on
`PUBLICATION_GAP_WINDOW_IDS` claims that is "roughly the last five months at the corpus head". Measured
on the box today, it is not:

| in the id window (43,650,996 … 45,650,995) | |
| --- | --- |
| rows, all sources | **76,946** — roughly the last day or two of INGEST, not five months of publishing |
| `doe` | 1,138 rows carrying **88 distinct publication days**, 2025-05-18 … 2026-09-15 |
| `fts` | 443 rows |
| `ted` | 75,365 rows carrying **21 distinct publication days**, essentially all of June 2025 |

The id space is SPARSE — 14.4 M notices under a MAX(id) of 45.65 M — so two million ids is about 77 k
rows, not two million. That alone breaks the five-month claim. But the deeper error survives any window
size: **ids are assigned at INGEST time and the measure reads PUBLICATION time.** A re-ingest or a
backfill puts old publication days at the head of the id space — which is exactly what the June-2025
TED block in the window is — and one day's DÖE ingest spreads 1,138 notices over 88 publication days
across sixteen months. A set of days sampled that way is not an interval, so "the gaps between
consecutive days" is not a question it can answer.

### The false alarms were diagnosable from the report's own text

The note under the table already says it: *"`before` and `after` are the notice counts on the days
bracketing the stretch. BOTH being ordinary is what makes a stretch a hole rather than the edge of a
source's life."* Both TED entries have a bracket of **1**. The principle was written down and then not
enforced in code, so the renderer printed as findings two stretches its own caption disqualifies.

### Two units, and they are separable

- **Unit A — enforce the bracket rule the note already states.** A stretch is reported only when BOTH
  bracketing days are ordinary for that source inside the window. Cheap, no schema, and it suppresses
  both TED entries (bracket 1) and most of the DÖE ones (brackets of 1–6 against a DÖE in-window day
  average of ~13), while the real 2026-06-30…07-15 hole (3,470 before, 3,722 after, ~3,500 typical)
  passes untouched. This makes the section trustworthy; it does not make it able to SEE that hole.
- **Unit B — a window that samples publication time.** The options, none free:
  (a) predicate on `published_at` directly — correct, but the column has no index, so it is a full scan
      of `notices`; (b) index `notices(published_at)` — one build over 14.4 M rows plus standing disk,
      against issue 169's concern; (c) measure on the canonical layer instead, where
      `tenders.current_published_at` is indexed (issue 82) — answers "which publication days the corpus
      reached" for projected tenders, which is close to but not identical with "what is HELD"; (d) keep
      the id window and simply print its coverage, which is honest and useless. **Measure before
      choosing** — and a corpus-scale characterisation has no compliant on-box path
      (`docs/agents/prod-box-reads.md`), so this is not a read to improvise.

### What this does not change

The gap itself is still real and still unclosed — 2026-06-30…2026-07-15, ~12 publication days, ~44,600
notices, with 3,470 notices held the day before and 3,722 the day after. This entry is about the
detector, not the defect. That work (probe a `2026-07` monthly and daily issues ~125–135, then fetch
and process) is untouched.

### Unit A landed 2026-09-16 — the caption's rule is now the predicate

`publication_gaps` computes an `ordinary_bracket_floor` — `ORDINARY_BRACKET_SHARE_PCT` (50 %) of that
source's MEDIAN day count within the window — and reports a stretch only when BOTH bracketing days
clear it. Median rather than mean on purpose: the population being excluded is a long tail of
near-empty days, and a mean is dragged down by exactly the rows it must not admit.

50 % is bounded by the calibration rather than picked: across 2026-07-27…2026-08-09 every published
TED day carries 3,247–3,752, a spread of ±7 % around the median, so a floor five times wider than the
observed variation cannot suppress a real bracket. Issue 402's own hole brackets at 3,470 and 3,722
against a ~3,500 median — 99 % and 106 % — and still reports.

Test `a_stretch_bracketed_by_a_barely_sampled_day_is_the_window_not_a_hole` replays the window as it
actually was (the June-2025 block, the 1-notice stray, the head a year later): every candidate in it
brackets on the stray, so nothing is reported — and the same window with a genuine ordinary-to-ordinary
silence appended still reports that one, so the guard is not "refuse everything". Red first with the
share at 0.

**Unit B — a window that samples publication time — is untouched and is the one that matters.** With
unit A the section is quiet and honest; it is still blind to the 2026-06-30…07-15 hole, because that
hole is not inside the id window at all.


## Comment — 2026-09-17: half (a) closed, measured on prod

The backfill ran to completion on the drained queue. Per-daily `process` outcomes, all `ok`, from
`/admin/jobs` (its recent window holds 20 entries, so 124/125/135 had already aged out of it by the
time this was read — they are accounted for in the fold total below rather than by their own rows):

| daily | members | daily | members |
| --- | --- | --- | --- |
| 2026-00126 | 3,908 | 2026-00131 | 3,737 |
| 2026-00127 | 4,151 | 2026-00132 | 4,133 |
| 2026-00128 | 3,429 | 2026-00133 | 3,404 |
| 2026-00129 | 3,899 | 2026-00134 | 3,334 |
| 2026-00130 | 3,771 | **visible subtotal** | **33,766** |

Seven quarantined across the nine, zero unrecognised. The fold that followed (job **2352**,
`project rebuild=false`) reports **44,854 notices → 40,848 tenders (843 islands), 85,747 versions**,
which covers all twelve dailies and lands within 0.6 % of this issue's headline estimate of ~44,600 —
the estimate was built from the ~3,720/day rate on either side of the window, so the agreement is a
genuine check of the arithmetic rather than a restatement of it.

**Served acceptance** — the real test, since a fold that ran is not the same as a window that answers.
Three days spread across the hole, each of which returned nothing before:

    /v1/tenders?published_after=2026-06-30T00:00:00Z&published_before=2026-07-01T00:00:00Z&limit=3
      → 200, 3 rows, 0.48 s — "Hangtauglicher Geräteträger, spezial-Doppelach…"
    …2026-07-07 → 200, 3 rows, 0.63 s — "Fenster-, Türelemente und Sonnenschutz"
    …2026-07-15 → 200, 3 rows, 0.45 s — "Liepājas ostas piestātnes Nr. 46 pārbūves būvp…"

(First attempt used bare `2026-07-01`, which is a 400: these bounds are unix seconds or RFC 3339, and
the error message says so plainly. Noted because the issue's own measurement is in unix seconds and
the two spellings are easy to mix up when re-checking this window later.)

**What remains is unit B**, and this comment does not touch it. Half (a) removes the symptom; nothing
yet detects the NEXT hole in this position, because 395's sequence test is monthly-only and
interior-only and this seam is neither. The hole existed for two months under a green ✓ and an
over-100 % coverage ratio (396), and that is the part of this issue worth finishing.

## Comment — 2026-09-17: unit B's detector half is fixed, and the reason it was blind

The continuity section could not have caught this hole, and the reason is sharper than
"it had a window". Section 14 windowed by **notice id** — the newest 2,000,000 — because
`notices.published_at` carries no index, so a time predicate scans where an id predicate is a
primary-key range. Right about the cost, wrong about the measurement: **notice ids are ingest
order**, and ingest order has nothing to do with publication order.

Two consequences, and the second is the one that mattered:

1. **A backfill re-shuffles the window.** The dailies backfilled for half (a) were ingested on
   2026-09-16/17, so they now hold some of the corpus's NEWEST ids while holding its OLDEST 2026
   publication dates. Nothing about an id window is stable under the one operation this report
   exists to check.
2. **A hole at the window's oldest publication day has no `before` bracket.** `publication_gaps`
   reports interior silences only — a stretch lies between two days that BOTH carry notices — and
   the caption has always said so. This hole sat exactly there: the monthlies were ingested before
   the dailies, so the newest 2M ids began at publication day **2026-07-16**, which is the day the
   hole ENDS. The stretch was off the edge of the window rather than inside it.

So it was never a threshold problem. Neither `PUBLICATION_GAP_MIN_DAYS` nor the ordinary-bracket
floor could have helped, because the rows were not in the result set to be judged. Interior-only
is a sound design; interior-only **plus an ingest-ordered window** is what made a 12-day blackout
render as `none`.

**Fixed:** the window is now `PUBLICATION_GAP_WINDOW_DAYS = 550` days of publication time, and the
query pays a full pass over `notices` — the same shape as the whole-corpus sweeps it is registered
beside (`fresh_holds`, `sentinel_dates`, `weld_candidates`), in a weekly background report rather
than on a served path. The bounds are computed in Rust (`publication_days_sql_at(now)`) so the
window is a pure function of an argument the test controls and the query does not depend on turso's
`'now'` modifier. The upper bound is load-bearing as well: `published_at` is publisher-supplied and
the corpus holds far-future values (section 10 sweeps for exactly that), and a single 2099 notice
would otherwise become the newest publication day and drag the median that the bracket floor is
computed from.

**Gated** by `the_continuity_window_is_publication_time_not_ingest_order`
(`crates/ingest/tests/data_quality.rs`), which builds this issue's shape in miniature and asserts
both halves — that the time window SEES the 28-day stretch and that the id window it replaced does
NOT. The counterfactual is executed rather than described. A third block published 600 days back
and ingested LAST is what makes it discriminate at fixture scale: `MAX(id) - 2,000,000` is negative
on a 24-row fixture, so without that block the forward assertions would pass under either
implementation — the false-green this repo keeps re-learning. Red-checked by reverting the
predicate; the test fails naming the `2025-01-24` row the id window wrongly admits.

### What remains on unit B

**`fetched_to` still lies about the range, and the mechanism is now located.**
`Db::fetch_registry_summary` (`crates/store/src/lib.rs:3702`) is

    SELECT source, COUNT(DISTINCT period), MIN(period), MAX(period), MIN(kind LIKE 'rates%')
      FROM fetches GROUP BY source

— a **lexical** extremum over a column whose meaning depends on `kind`. `'2026-06' > '2026-00136'`
(common prefix `2026-0`, then `6` > `0`), so `to` reads `2026-06` while the daily series runs to
`2026-00178` = 2026-09-11, and the funnel prints `445 pkgs (1993-01 … 2026-06) · fetch complete ✓`
— a range ending 2½ months ago beside a tick claiming caught-up.

`fetch_complete`'s year test (`to.starts_with(current_year)`) is passing on that stale value. Note
it would ALSO have passed had the dailies stopped in January, so the test is weak independently of
this hole.

Making the range span both namespaces honestly needs a per-kind `period → instant` mapping, and
that is a design decision rather than a one-liner: `ted daily` periods are **OJ S issue numbers**
(`2026-00136`), not dates, so the mapping needs the issue→date relation this issue established by
measurement (`2026-00135` = 2026-07-15, `2026-00136` = 2026-07-16). The alternative the "Done when"
already offers — the cell NAMES the two namespaces separately — needs no mapping at all and is
probably the right first move. Left for its own unit rather than rushed in beside the detector.

The detector, though, is the half that generalises: it is namespace-agnostic by construction, so it
would flag the next seam hole whatever the two period vocabularies turn out to be.
