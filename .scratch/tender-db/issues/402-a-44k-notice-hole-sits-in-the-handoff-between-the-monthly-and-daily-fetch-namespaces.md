# 402 — ~44,600 TED notices are missing from 2026-06-30 to 2026-07-15, in the seam between the monthly and the daily fetch namespaces, and every guard on the board reads green over it

Status: ready-for-agent — found 2026-09-16 by the hourly audit (step 3) on prod rev `19010b8`. The hole is MEASURED, not inferred: zero notices in the window, on both sides of which the corpus publishes ~3,720/day. The fix has two halves — fetch the missing issues, and make the seam checkable — and the second is the one that matters, because nothing on the board can currently see a hole in this position.
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
