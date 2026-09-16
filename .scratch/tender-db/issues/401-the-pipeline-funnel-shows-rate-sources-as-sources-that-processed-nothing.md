# 401 — the Pipeline funnel counts two CURRENCY-RATE sources as import sources that fetched packages and processed nothing, so `ecb` reads as 20 stalled packages and `eurostat` as 1

Status: ready-for-agent — found 2026-09-16 by the hourly audit (step 3), on the live dashboard at rev `19010b8`. Fully specified; the only open question is which of the three renderings below to take, and that decision is the first unit.
Kind: docs/presentation (dashboard, `crates/app/src/ui.rs`'s Pipeline panel and the `PipelineStage` rows that feed it). No data is wrong, no job is stalled, nothing is missing: both sources are doing exactly their job and the funnel has no column that can say so.
Relates to: 396 (RESOLVED-VERIFIED 2026-09-16 — the same family, a panel whose own explanatory copy does not describe a row beside it; its precedent is that this is worth fixing rather than knowing), 395 (RESOLVED — it put `fetch complete ✓`, `missing_periods` and `duplicate_periods` on these rows, which is how a rates source comes to display a fetch-completeness verdict at all), 33 (RESOLVED-VERIFIED 2026-08-17 — "a number that stops moving must SAY why"; a number that was never going to move is the same rule), ADR-0014 (the EUR-at-publication-date conversion these rates serve), 06 (the panel itself)
Blocked by: nothing

## Observed (verified 2026-09-16 on prod, rev `19010b8`)

    curl https://tenders.zebreus.click/

Pipeline panel, whose intro sentence reads

> Where each source is in the import — fetched packages, then notices processed out of them, then
> Tenders projected.

| Source | Published | Fetched | Processed | Projected |
| --- | --- | --- | --- | --- |
| `doe` | — | 101 pkgs (2022-12 … 2026-09-14) · fetch complete ✓ | 1 135 578 | 675 904 |
| **`ecb`** | **—** | **20 pkgs (2026-08-27 … 2026-09-15) · fetch complete ✓** | **0** | **0** |
| **`eurostat`** | **—** | **1 pkgs (1993-1998 … 1993-1998)** | **0** | **0** |
| `fts` | — | 9 pkgs (2025-06 … 2026-09-14) · fetch complete ✓ | 10 157 | 8 848 |
| `ted` | 13 201 520 | 445 pkgs (1993-01 … 2026-06) · fetch complete ✓ · registered twice: 2025-09 | 13 307 653 | 7 836 705 |

Two of the five rows are not import sources at all. `ecb` and `eurostat` fetch **currency rates**:
`supervisor.rs:8472` fetches `source: "ecb"` / `kind: "rates"` and reconciles into `currency_rates`;
`supervisor.rs:8550-8559` loads the eurostat SDMX ECU series for 1993–1998 as the historical half
(`fetch-rates-ecu`, "eurostat ecu 1993-1998"). Neither will ever produce a notice or a Tender, so
their `0 | 0` is not a stall — it is the only value those cells can hold, forever.

## They are working, and the funnel cannot say so

Measured on the box the same minute the panel was read:

    SELECT count(*) AS rows, count(DISTINCT currency) AS currencies,
           min(rate_date) AS first_date, max(rate_date) AS last_date FROM currency_rates
    -> [277445, 54, "1993-01-04", "2026-09-14"]

    SELECT substr(rate_date,1,4) AS yr, count(*) FROM currency_rates
     WHERE rate_date < '1999-01-01' GROUP BY yr
    -> 1993:8581  1994:9123  1995:9177  1996:9842  1997:9835  1998:10148

**277,445 rate rows across 54 currencies, continuous from 1993-01-04 to 2026-09-14.** The eurostat
one-package row is carrying the entire pre-euro era, and the ECB row is carrying everything since,
current to the day before the read. Both are among the healthiest things on the box, and the panel
that is supposed to say "where each source is in the import" gives them the exact display signature
of a source whose processing has stalled: packages in, nothing out.

## Why it matters

This panel is the operational read — it is where "is anything stuck" gets answered. Two of its five
rows permanently show the stuck shape, which trains a reader to ignore zeros in that column. The next
time a real source fetches and fails to process, it will look like `ecb` and `eurostat` have looked
all along.

It also mis-states completeness in the other direction: `ecb` carries **`fetch complete ✓`** (issue
395's verdict — latest period in the current year, no monthly hole), which for a rates feed answers a
question nobody asked, while `eurostat` correctly lacks it only by accident — its one package's
period is the literal string `1993-1998`, which is not in the current year. Neither verdict means
anything for a source whose "completeness" is whether today's rates arrived.

## Why this is ours, not the publisher's

The ECB and Eurostat both published exactly what was asked of them, and this system stored all of it.
What this system then did was list two reference-data feeds in a table whose columns are `Published`
/ `Processed` / `Projected` — quantities that do not exist for them — and let three cells read `—`,
`0`, `0` with nothing marking the difference between "none yet" and "never any".

## Repro

1. `curl https://tenders.zebreus.click/` → Pipeline panel: `ecb` row `20 pkgs … fetch complete ✓ | 0 | 0`,
   `eurostat` row `1 pkgs (1993-1998 … 1993-1998) | 0 | 0`.
2. `grep -n '"ecb"' crates/app/src/supervisor.rs` → `:8472`, `kind: "rates"`, reconciling into
   `currency_rates`; `grep -n 'eurostat' crates/app/src/supervisor.rs` → `:1810` `fetch-rates-ecu`,
   `:8550` the SDMX endpoint.
3. The two bounded SELECTs above → 277,445 rows / 54 currencies / 1993-01-04 … 2026-09-14.
4. `grep -rn 'processed_notices' crates/app/src/coverage.rs` → the funnel counts notices per
   `fetches.source`, and a rates fetch is a row in the same registry, which is why they appear here
   at all.

## Done when

- A decision is written here with its reason, among at least:
  (a) the funnel splits into two tables — import sources and reference feeds — each with the columns
      that mean something for it (for rates: rows, currencies, date span, freshness);
  (b) the rate rows stay in place but their `Processed`/`Projected` cells render `—` with a title
      saying a rates feed produces no notices, and their `fetch complete` verdict is replaced by a
      freshness one (newest `rate_date` vs today);
  (c) the rate sources are excluded from the funnel entirely and surfaced wherever ADR-0014's
      conversion health belongs instead.
  Whichever is taken, no cell on this panel may show `0` for a quantity that is structurally always
  zero.
- The panel's intro sentence is true of every row it renders, or the rows it is not true of are
  somewhere else.
- A rates feed's actual health IS visible somewhere after the change — the newest `rate_date` against
  today is the number that matters, and nothing on the dashboard shows it now. An unconvertible head
  value is ADR-0014's failure mode and a stale rate table is its cause; today the only way to see it
  is a box read.
- A test pins the invariant rather than the source names: a `PipelineStage` that cannot produce
  notices is never rendered with a `0` in a notice column. Hard-coding `ecb`/`eurostat` in the UI is
  the version of this that breaks the next time a reference feed is added.
- Re-read after the fix: `ted`, `doe` and `fts` are unchanged (445/101/9 packages, the same processed
  and projected counts, ted still carrying `registered twice: 2025-09` until issue 395's duplicate is
  reconciled); `currency_rates` still reads 277,445 rows over 54 currencies — this is presentation
  only and nothing is refetched.

## Adjacent, recorded not fixed

`ted`'s Fetched cell still reads `registered twice: 2025-09`, which is issue 395's known-open
duplicate reconciliation, confirmed still live here. Not this issue's.

## BUILT 2026-09-16 — decision: (b), and the discriminator is the fetch KIND

Status: built, gate green (`GATE-EXIT=0`), not yet deployed.

### The decision

Three renderings were offered. **Taken: (b)** — the rate rows stay in the funnel, their
`Processed`/`Projected` cells render `—` with a title, and their `fetch complete` verdict is replaced
by a freshness one.

- Not **(a), two tables**: splitting a five-row panel in two for two rows is a bigger change to how
  the page reads than the defect justifies, and it buries the fact that these feeds ARE part of the
  import — ADR-0014's EUR conversion depends on them.
- Not **(c), exclude them**: the `## Done when` requires a rates feed's health to be visible after the
  change, and removing the only place they appear makes that harder, not easier. A working feed that
  is nowhere on the dashboard is one nobody checks.

### The discriminator is `fetches.kind`, never the source name

`ecb` fetches kind `rates`; `eurostat` fetches `rates-ecu-h` and `rates-ecu-bil`. So
`fetch_registry_summary` gains a fourth aggregate, `MIN(kind LIKE 'rates%')` — **every** package of
the source must be a rates download — surfaced as `FetchRegistryRow::reference_only` and carried to
`PipelineStage::reference_feed`.

A name list (`source == "ecb" || source == "eurostat"`) would have been three lines shorter and is
the version of this that goes stale: it silently mis-classifies the next reference feed, which is
exactly how these two came to be mis-classified in the first place. The test says so by using source
names (`ratesonly`, `mixed`, `notices`) that no name-matching implementation could pass.

`MIN(...)` rather than `MAX(...)` is load-bearing and has its own fixture arm: a source that fetches
one rates package AND notice packages (`mixed`) is an import source, and a `MAX` would have blanked
its real counts on the strength of a single row.

### Rate health is now on the page

`Db::newest_rate_date()` — `SELECT MAX(rate_date) FROM currency_rates`, an index extremum on that
table's primary key, read once for the whole funnel rather than per row. It rides as
`PipelineStage::rates_through` and renders as `· rates through 2026-09-14`.

That is deliberately the DATA's newest date and not the fetch date: a fetch that lands a stale file
looks healthy in the registry, and this is the number that would show it. Both feeds load the same
table so both rows show the same date; the cell's title says so, and per-feed attribution would be a
bigger measurement than this defect warrants.

`fetch_complete` is now `false` for a reference feed by construction. Both of its old verdicts were
accidents: `ecb` passed the current-year test because its period IS a civil date (`2026-09-15`), and
`eurostat` failed it because its period is the span label `1993-1998`. Neither said anything.

### The panel's own sentence

Extended to cover the rows it was not true of: "A source whose packages are exchange rates rather
than notices has no notices and no Tenders to count (issue 401): its last two cells read — and its
Fetched cell says how current the rate table is instead."

### Live acceptance still owed (after deploy)

- `ecb` reads `20 pkgs (…) · rates through 2026-09-14 | — | —`, with no `fetch complete ✓`.
- `eurostat` reads `1 pkgs (1993-1998 … 1993-1998) · rates through 2026-09-14 | — | —`.
- `ted`, `doe` and `fts` are byte-identical: 445/101/9 packages, the same processed and projected
  counts, `fetch complete ✓` still on all three, and `ted` still carrying
  `registered twice: 2025-09` (issue 395's open duplicate).
- `currency_rates` still reads 277,445 rows over 54 currencies — presentation only, nothing refetched.
