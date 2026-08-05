# 137 — the quarantine is 72% already reclaimed; the 1.2M target is really 71,707

Status: **measured 2026-08-05**, all three checks run under team-lead's word. Answers Lennart's
"is `unknown-customization` the next major push?" question.
Owner: sdk-vendor
Relates to: 84/#29 (the DTD population, which this puts in context), 74 (which already added the SDKs)

## Provenance

Snapshot `/data/db/snapshots/tender-db-1785916688.db`, pinned by name, header-complete, **`-wal` is
0 bytes** — checked first, because stock sqlite3 reading a turso-written WAL silently returns a
*wrong* answer (`quarantine_2008_dtd_population.sh`). The live DB's WAL was 392 MB at the time, which
is exactly why this ran against the snapshot and not the serving database.

## The headline: the dashboard's 2.4M counts rows that were already recovered

The schema carries **three** outcomes and says why, at `crates/store/src/lib.rs:158`:

> *Distinct from `reprocessed_at`, which means RECLAIMED: using that column here would claim ~593k
> notices entered the corpus that never did. Three outcomes exist (outstanding / reclaimed /
> skipped) so the schema carries three, rather than hiding one inside another.*

| | rows |
|---|---|
| quarantine rows total | **2,419,410** |
| **reclaimed** (entered the corpus) | **1,734,594 — 71.7%** |
| skipped | 0 |
| **truly outstanding** | **684,816** |

**A quarantine row is retained as a historical record after reclaim.** Counting rows therefore
counts work that is already done.

## Per reason

| reason | rows | reclaimed | truly outstanding |
|---|---|---|---|
| `unknown-customization` | 1,201,074 | 1,129,367 | **71,707** |
| `unparsable-xml` | 628,204 | 26,948 | **601,256** |
| `unknown-field-code` | 576,753 | 576,743 | **10** |
| `unclaimed-content` | 7,503 | 1,536 | 5,967 |
| `unrepresentable-value` | 3,213 | 0 | 3,213 |
| `not-utf8` | 1,700 | 0 | 1,700 |
| `unknown-root` | 753 | 0 | 753 |
| `missing-publication-id` | 108 | 0 | 108 |
| `translation-structure-mismatch` | 94 | 0 | 94 |
| unreadable zip (EOCD) | 8 | 0 | 8 |

**`unknown-field-code` is 576,753 rows and 10 outstanding** — 99.998% done, and on the dashboard it
reads as a 577K gap.

## And the 601,256 `unparsable-xml` is mostly the DTD population

| detail | outstanding |
|---|---|
| **XML with DTD detected** | **594,915** |
| unknown token at 1:1 | 4,441 |
| unknown token at 1:3 | 1,900 |

That is issue 84 / #29's population — the 592,856 team-lead's dry-run found markable as
skipped-by-policy (duplicates), plus the 154 the sibling guard rejected. `skipped_at` is 0
everywhere because that execute was correctly a NO-GO.

**So once #29 lands, the genuine remaining gap is ≈ 89,901**, not 2.4M:

    684,816 truly outstanding − 594,915 DTD-as-duplicates ≈ 89,901

## The answer to the question asked

**Is `unknown-customization` straightforwardly recoverable? Yes — and it needs no parser work at all.**

Grouping by the actual CustomizationID (stored in `quarantine.detail`) gives **13 distinct IDs, all
recognised eForms identifiers, no long national/bespoke tail**:

    eforms-sdk-1.7  344,517     eforms-de-1.1  145,859     eforms-sdk-1.3  4,834
    eforms-sdk-1.10 255,426     eforms-de-1.2   72,986     eforms-sdk-1.0  3,473
    eforms-sdk-1.8  140,834     eforms-sdk-1.6  37,669     eforms-sdk-1.5     35
    eforms-sdk-1.11 107,969                                eforms-de-1.0      31
    eforms-sdk-1.9   87,438                                eforms-sdk-1.2      3

**Every one of these except `eforms-sdk-1.2` (3 rows) is already in `ACCEPTED` with its metadata
vendored** (`crates/ingest/src/eforms/sdk.rs:40`; `resolve()` maps `eforms-de-1.0/1.1/1.2` →
`eforms-de-1.x`, also vendored). Issue 74 did this work. The code comment even names the figure:
*"The 1.0–1.7 minors (issue 74, ~390K notices incl. 1.7 = 344K)"* — matching 344,517 exactly.

**Falsifier run, and it holds:** every row's `first_seen` is 2026-07-22/23, and **zero
`unknown-customization` rows have been created in the last 7 days**. The parser is not still failing.
These are historical rows from before issue 74, of which 94% have already been reclaimed.

**So: not "add SDK support + reprocess" (a development project), but "finish the reprocess" — 71,707
members whose parser support already ships.**

## Two counting corrections, and one of them was nearly mine

* **The remembered/dashboard framing was wrong by 17×** for the target class: 1,201,074 advertised,
  **71,707** real. `unknown-customization` *is* still the largest genuinely-recoverable class once the
  DTD population is correctly classed as duplicates — the target was right, the **size** was not.
* **I nearly made the same error one layer down.** My first pass used `reprocessed_at IS NULL` as
  "outstanding", which silently folds **skipped** in with **outstanding** — precisely the conflation
  the schema comment above exists to prevent. It happens to make no difference today because
  `skipped_at` is 0 everywhere, and it would have started lying the moment #29 executed. The schema
  author anticipated the exact mistake and wrote the warning next to the column.

## Recommendation

1. **Don't size a push at 1.2M.** The recoverable figure is **71,707**, needing a reprocess run, not
   parser development. Cheap — and the reclaim mechanism already ran at this scale (issues 76/77/80).
2. **Land #29 first.** It reclassifies 592,856 rows from "outstanding" to "skipped (duplicate)", which
   is 87% of everything still outstanding. Nothing else moves the number remotely as much, and it
   costs no reprocessing at all — it is a *counting* correction.
3. **Fix the dashboard to report the three-way split.** Presenting reclaimed rows inside a
   "quarantined" total is an honesty defect of the same family #29 exists to fix, and it is what made
   a 71,707 target look like 1,201,074.
4. `eforms-sdk-1.2` (3 rows) is the only genuinely unsupported customization. Not worth a push.
