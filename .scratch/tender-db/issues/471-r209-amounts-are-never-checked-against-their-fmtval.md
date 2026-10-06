# 471 — r209 amounts are never checked against their `@FMTVAL`, and nothing adjudicates the €10–100 bn head-value band (366 units 5 and 6, dropped when 366 closed)

Status: ready-for-agent — UNIT 1 LANDED IN THE TREE 2026-10-06, review fixes applied the same day (uncommitted, not deployed): section 16 of the data-quality report lists the band; see "Unit 1 — landed (2026-10-06)". NEXT: `ops/check.sh`, commit, deploy; then BEFORE issue 429's weekly `analyze` schedule goes live, a plan-probe of `band_listing_sql()` on an analyzed prod snapshot (the fixture-ANALYZE pin is not prod's stats, and `measure_rows` has no deadline); then the stored report's Done check (the section's summary line present, no `UNMEASURED — the \`band_listing\``), then unit 2's gated archive read. Was: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is 366's unit 6: a weekly-report section that lists every elected head value at or above €10 bn, grouped by published currency, with each row's signals beside it, read off the `tenders_current_value_eur` index.
Kind: data quality (amount plausibility: the legacy parse layer and the head election)
Relates to: 366 (promised units 5 and 6, closed 2026-09-12 without them), 380 (its sweep still points at "the open half of
issue 366"), 267 (the plausibility measure), 372 (the `quality` marker on `Fact::Amount`), 385 (F14 corrigendum dates:
the date half of the value corrigenda below), ADR-0004

## What is wrong

### 366 promised two units and closed without them

- 366 `## Units`, item 5 (line 65): compare `@FMTVAL` against the element text in `Rule::Amount`, "Needs one
  archive-member read first — gated per `docs/agents/prod-box-reads.md`". Item 6 (line 66): "the weekly report lists
  the top N by magnitude per currency".
- Until the close, 366's status line ended "Next: unit 5's gated archive read for the €10–100bn band that now tops the
  ordering, and unit 6's magnitude listing" (lines 18–19). "Still open in this issue" (line 608) and "the issue is NOT
  closeable" (line 842) name both again.
- The close (line 1165) ticked off the four bullets of the 2026-09-10 "Still open after this firing" list. Neither unit
  was on that list, and the follow-ons the close names (378–381) cover neither.
- Three pointers still send this work to the closed issue: `crates/ingest/src/data_quality.rs:801` ("left as the open
  half of issue 366"), `.scratch/tender-db/api-dq-review-2026-09-15.md:350–353` (4490098's gated archive read "under
  366 unit 5"), and `crates/store/src/canonical.rs:1501–1506` (the €10–100 bn band "needs the tender-versus-lot-sum
  ratio, not a bigger constant").

### 366 unit 5: the legacy parser adopts `@FMTVAL` and never reads the text beside it

`crates/ingest/src/r209/parse.rs:426–428`:

```rust
Rule::Amount => {
    let lexical = el.attribute("FMTVAL").map(str::to_owned)
        .or_else(|| (!text.is_empty()).then(|| text.clone()));
```

The element text is read only when `@FMTVAL` is absent. A wrong machine value is adopted, and the human-readable figure
is discarded without a trace (`rules.rs:63`: "`@FMTVAL` if present (defence)"). The walker serves both TED_EXPORT
profiles, `ted-export-r208` and `ted-export-r209` (`r209/mod.rs:1`). `Rule::Number` (parse.rs:448) has the same shape.

Four legacy award notices carry a figure that is an exact power of ten away from another figure of the same tender,
which is the shape a machine-value scale error leaves. Read 2026-10-01 from `/v1/notices/{id}` and `/content`:

| tender | notice (profile), archive member | the figure (cents) | its partner |
|---|---|---|---|
| 4490098 | 12376354 (r208), `2011-07-15.tar.gz/20110715_134/222043_2011.xml` | PROCEDURE `TED-VALUE_COST` 4,970,000,000,000,000,000 EUR | RES-1 `TED-VALUE_COST` 497,000,000, same notice: 10¹⁰ |
| 6581010 "Planungsleistungen für Schulerweiterungsbau …" | 20852242 (r209), `01/20200120_2020013.tar.gz/20200120_13/00026682_2020.xml` | PROCEDURE `TED-VAL_TOTAL` 5,973,328,000,000 EUR and RES-1 `TED-VAL_TOTAL` 4,654,980,000 EUR | the tender's first notice 382778-2019: total 597,332,800 (10⁴) and lot 46,549,800 (10²) |
| 6941544 "Strategic Partner for the Sunderland Smart City 5G Neutral Host" | 22005354 (r209), `20211027_209/548977_2021.xml` | RES-1 `TED-VAL_TOTAL` 8,000,000,000,000 GBP | PROCEDURE `TED-VAL_ESTIMATED_TOTAL` 8,000,000,000 GBP, same notice: 10³ |
| 6843260 "Homecare and Support on the Isle of Wight" | 21466794 (r209), `20210111_006/010347_2021.xml` | RES-1 `TED-VAL_TOTAL` 309,715,848,000 GBP | "3 097 158.48" printed in the buyer's own F14 21473937 (II.2.14): 10³ |

The API serves the parsed value only, so whether `@FMTVAL` produced these is exactly what the archive read answers.
Archive members are "a bounded I/O job, gated like any data read" (`docs/agents/prod-box-reads.md`).

### 366 unit 6: nothing lists the band row by row

The weekly report (stored 2026-09-29 22:28 UTC, 15 sections, read through `/root/aj.sh /admin/reports/data-quality`)
ranks the implausible tail by repetition only. Section 10 (`sentinel_amounts_sql`, data_quality.rs:827–840) keeps a
value only if it repeats at least 10 times (`SENTINEL_MIN_REPEATS`), orders by `hits DESC`, and caps at 40 rows. A
figure that occurs once never appears. Neither 20905's EUR 33,260,500,000.00 nor 5948128's GBP 12,345,678,910.00 ("Da
Vinci Si HD Surgical Robot", a digit run) is in the stored report.

### The band, re-read 2026-10-01

`GET /v1/tenders?min_value=1000000000000` (18 cursor pages) returns **324 tenders** whose elected head is at or above
€10 bn: 229 from TED and 95 from FTS. `IMPLAUSIBLE_EUR_CENTS` (€100 bn, canonical.rs:1507) caps the band from above.
Every row is served with no marker. The count moves as FTS rows land: the same walk at 13:41 UTC that day
returned 330 (229 TED, 101 FTS) on 18 pages. The tables below are the 324-row read.

**Tender 20905, "Neubau und Erweiterungsbau Lukas Schulen"**, serves `value {cents: 3326050000000, currency: EUR}`,
which is €33.26 bn. It is a single lot for the buyer Lukas-Schule gemeinnützige GmbH. The lot drew two electronic
tenders and went to Georg Reisch GmbH & Co. KG. The figure is the publisher's own. Both award notices (eForms-DE
`3efdb7c5-…-01` and TED `00447172-2025`, subtype 29, 2025-07-08/09) publish 33,260,500,000 EUR in
`BT-161-NoticeResult` and in `BT-720-Tender`. The two subtype-16 contract notices (2024-05-13/15) publish no amount at
all. So nothing inside the tender can separate it from a real €33 bn contract:
- the `@FMTVAL` check cannot reach it, because eForms has only one representation of the figure;
- the lot-sum ratio cannot reach it either. The one lot's award is the same figure, so the ratio is 1.0;
- there is no estimate to compare it with.

**What the signals inside a tender reach.** For each band tender, the head figure was compared with the other positive
figures in the same currency in its served `amounts` and `lot_results`. Era: FTS by source, TED rows with an eForms
subtype as eForms, and the remaining TED rows by their head notice's profile.

| head era | tenders | another figure ≥ 10× below the head | of which exactly 10ᵏ below (k ≥ 3) | other figures, all within 10× | one figure, no lot figure |
|---|---|---|---|---|---|
| TED eForms | 56 | 26 | 6 | 26 | 4 |
| FTS | 95 | 23 | 0 | 27 | 45 |
| TED r208 | 34 | 14 | 6 | 8 | 12 |
| TED r209 | 112 | 35 | 9 | 15 | 62 |
| TED text (1993–2010) | 27 | 0 | 0 | 0 | 27 |
| **all** | **324** | **98** | **21** | **76** | **150** |

- **An exact power of ten is a sharp signal.** Among the 21 are 5592948 "Vending Machine Services" (£9 bn against six
  £9 M lot estimates, 10³) and 4578779 "Analysis of sexual health in the European Union" (€20 bn against a €20,000 lot
  award, 10⁶). Six of the 21 are eForms, so this signal works in every era, unlike `@FMTVAL`.
- **A plain ratio is not a signal.** The other 77 rows at ≥ 10× mix real frameworks with junk, for example 8596298
  "National Framework for Developer Led" (FTS, £19 bn, ratio ~540) and 188920 "Smart incubators" (SEK 500 bn). A ratio
  threshold repeats the magnitude problem one level down.
- **The lot-sum ratio needs a lot figure.** 150 of the 324 carry only one distinct figure in the head's currency and
  no lot figure. The 74 of those that are r208/r209 are reachable only by the `@FMTVAL` check.

**The publisher's own correction is stored as prose.** An F14 corrigendum's `TED-NEW_VALUE.TEXT` is projected as a
`description` text (`crates/ingest/src/project.rs:159`), so a value the buyer corrected stays elected. 3 of the 146
r208/r209 head notices in the band are value corrigenda:

| tender | corrigendum | section | old → new (as published) | served |
|---|---|---|---|---|
| 5592948 Vending Machine Services | 071343-2017 (2017-02-24) | II.1.7, V.2.4 | 9 000 000 000.00 GBP → 9 000 000.00 GBP | £9 bn, the corrected-away figure |
| 6891632 ARIA_2020_270.9 | 458701-2020 (2020-09-30) | II.1.5, II.2.6 | 25 280 256 000,00 EUR → 85 536 000,00 EUR | €25.28 bn, the corrected-away figure |
| 4871119 Green Services Framework | 250165-2017 (2017-06-30) | II.1.5 | 250 000 000.00 GBP → 25 000 000 000.00 GBP | £25 bn, which agrees with the correction |

Only head notices were read, so an earlier corrigendum in a tender's chain is not counted here.

These tables are a photograph taken by a hand walk of the public API (18 list pages, 324 tender details, and the head
notice's `/v1/notices/{id}` and `/content`). Unit 1 makes the system take it.

## Proposed fix

The root cause is that the head election has no signal except magnitude, while the signals that separate a scale error
from a big contract sit inside the tender: a second representation of the figure (`@FMTVAL` against the text), a
sibling figure exactly 10ᵏ away, or the publisher's own correction. Each one becomes a `quality` marker on the
`Fact::Amount`, which `head_value_eur_cents` already skips (`quality.is_none()`, canonical.rs:1460–1461). No new
threshold is added (366 unit 1: no threshold separates the band), and no read-layer filter (366 unit 4: a second place
that decides which amounts count drifts from the fold).

1. **The band listing (366 unit 6).** Add a data-quality section that range-reads `tenders_current_value_eur`
   (`(current_value_eur_cents, id)`, canonical.rs:8311) from €10 bn upward. That is 324 rows today, read through an
   index, not a scan of the tail. It lists every row grouped by published currency, with the head's profile, the
   published figure, and its signals: the exact-10ᵏ partner if one exists, the ratio to the smallest sibling (for
   reading only), and whether a value corrigendum (F14 `NEW_VALUE.TEXT` under II.1.5, II.1.7, II.2.6 or V.2.4) is in
   the chain. Each signal is a per-tender seek, not a scan. Pin it with
   `the_band_listing_shows_a_head_that_repeats_nowhere`: a fixture with one unique band figure appears in the listing
   and not in section 10. Done when the stored report carries the section MEASURED — the header
   alone is not enough, because it prints before the UNMEASURED check (review, 2026-10-06):
   `ssh -o BatchMode=yes root@zebreus.click "/root/aj.sh /admin/reports/data-quality" | jq -r .body | grep -cE 'Tender\(s\) in [0-9]+ currenc'`
   prints `1` (it printed `0` on 2026-10-01, before the section existed) and the same body
   piped to `grep -c 'UNMEASURED — the .band_listing'` prints `0`.
2. **The gated archive read (366 unit 5's precondition).** Read the four members in the first table on the team
   lead's word, per read, and record each value element's `@FMTVAL` and text side by side. If they agree everywhere,
   the `@FMTVAL` question is answered "no": record that the scale errors are the publishers', and units 4 and 5 below
   carry the band.
3. **The `@FMTVAL` check (366 unit 5), if the read shows disagreement.** `Rule::Amount` parses both representations.
   The parse layer keeps both (ADR-0004), and the projection marks the adopted fact with a second `quality` value
   beside 372's `withheld`. Pin it with `an_fmtval_that_disagrees_with_its_element_text_does_not_reach_the_head`
   (fixture in `crates/ingest/tests/r209.rs`, election in the canonical tests). Standing rows need a re-parse. `reparse` takes a
   profile cohort, bounded only by a package count and a starting fetch (`crates/app/src/supervisor.rs:1275`). That is
   the era's ~15 M amounts (report section 8: 3,686,659 r2.0.8 + 11,275,425 r2.0.9) to reach about 146 band notices,
   so add a notice-list form the way `refold-notices` has one.
4. **The two in-tender signals, decided on unit 1's listing.**
   (a) A figure exactly 10ᵏ (k ≥ 3) above another positive figure of the same version in the same currency is refused.
   Pin it with `a_figure_exactly_ten_to_the_k_above_a_sibling_is_not_elected` (canonical.rs tests), including a case
   with a round mantissa, because a €2 bn framework over a €2 M lot is the false positive to rule on.
   (b) A TED F14 that restates a value section supersedes the figure it corrects, keyed by section the way 385 keyed
   `NEW_VALUE.DATE`. Pin it with `an_f14_value_correction_supersedes_the_figure_it_corrects`
   (`crates/ingest/tests/project.rs`). Whether (b) stays here or becomes its own issue is part of this unit's decision.
5. **20905's class: a single figure, repeated in every slot, with no sibling.** No signal inside the tender reaches it.
   Decide between a signal from outside the tender (the figure against its CPV's or its buyer's distribution) and
   serving the residue as published with a sentence in `/docs#caveats`, and record the reasoning here.
6. **Drain and re-read.** Refold the affected tenders with `refold-notices` (366's route), re-read the unit-1 section
   and the Verify, and record which band rows each signal removed and which remain.

## Unit 1 — landed (2026-10-06)

Section 16 of the weekly data-quality report, "The head-value band (every elected head at or above
EUR 10,000,000,000.00, by published currency — issue 471)". In the tree, not committed, not deployed;
the Done check above (the summary line, not the header) is still owed after deploy and the next
Sunday run (or an on-demand `data-quality` job).

- **Query**: `data_quality::band_listing_sql()`, registered in `whole_corpus_queries()` as
  `band_listing` (whole-corpus because it is driven by a value range the tender-id windows cannot bind
  to). The supervisor's data-quality job runs `whole_corpus_queries()` generically, so
  `supervisor.rs` is untouched. The section reaches `/admin/reports/data-quality` as TEXT only: the
  job stores `render_text`'s body, and the endpoint serves `{kind, computed_at, age_seconds, body}`.
  The JSON form (`.head_value_band.rows`, plus `full` and `measured`) comes only from
  `bin/data-quality --json` (every query over `/v1/sql`), so unit 4/6's before/after comparison reads
  the text rows or runs the CLI.
- **One SQL row per Tender**: the elected row `h` is a LEFT JOIN on ONE rowid, picked in a correlated
  subquery the read layer's way (`eur_cents = t.current_value_eur_cents` on the head version, tie
  broken `cents DESC, currency` as `read.rs`'s `elected` does), so the listed figure and currency
  group are what `/v1/tenders` serves, and the LIMIT counts Tenders. A Tender whose head column
  matches no amount row at its head version (a `rederive-eur` move awaiting its refold, issue 375)
  is listed under "elected row NOT FOUND" with no published figure, not dropped.
- **Order and cap**: walked top-down (`ORDER BY current_value_eur_cents DESC, id DESC`, still the
  index's order — no sorter over the listing), so the safety cap `BAND_LISTING_CAP` = 1,000 Tenders
  (3× the measured 324–330) cuts the LOWEST heads; LISTING FULL prints before the rows, and JSON has
  `full`.
- **Signals per row**: head notice + era; published figure and EUR; the exact-10ᵏ partner (k = 3…18:
  the largest positive figure of the same Tender and currency, ANY version, from
  `tender_version_amounts` and `tender_version_lot_results.awarded_cents`); `x smallest` (head over
  the smallest same-currency sibling, reading only); and the newest F14 value corrigendum in the
  chain (`TED-NEW_VALUE.TEXT` in a `CHG-n` whose `TED-SECTION`, normalised as
  `project::f14_coordinate` does — `trim(rtrim(trim(x), ')'))` — is II.1.5 / II.1.7 / II.2.6 /
  V.2.4). Siblings at or below 10.00 as published (`SENTINEL_AMOUNT_CEILING`, issue 380's
  placeholders) are neither partners nor siblings, so a round €10 bn ceiling over a 1.00 placeholder
  does not read as a 10¹² scale error. Rows are grouped by published currency, biggest group first,
  largest figure first inside.
- **Known blind spot, recorded rather than guessed at**: the corrigendum column knows the
  2014-directive (r2.0.9) numbering only. r2.0.8 F14s name value sections in the 2004 numbering
  (e.g. II.2.1 "total quantity or scope", which in 2014 is a lot's TITLE); none has been measured
  carrying a `NEW_VALUE.TEXT` on prod. The footer says `—` on an r2.0.8 chain means NOT CHECKED, and
  that a no-signal r2.0.8/r2.0.9 row is not 20905's class (the `@FMTVAL` check, units 2–3, can still
  reach it). Measuring the 2004 value coordinates belongs to unit 4b.
- **Bound, pinned by plan** in `the_band_listing_shows_a_head_that_repeats_nowhere`
  (`crates/ingest/tests/data_quality.rs`): first line `SEARCH t USING INDEX tenders_current_value_eur
  (current_value_eur_cents>=?)`, `h` by `INTEGER PRIMARY KEY (rowid=?)`, every satellite a SEARCH by
  its `tender_id` / `notice_id` prefix, no SCAN anywhere, no sorter at the top level (the only sorter
  is the per-Tender tiebreak over the head version's few rows) — asserted on an EMPTY `sqlite_stat1`
  AND again after `store::ANALYZE_TABLES` are analyzed over a 2,000-Tender filler. That second half
  is fixture statistics, not prod's: on the 8-Tender fixture alone the analyzed plan turns every read
  into a SCAN (correct for 8 rows, and proof the stats are read). `measure_rows` has no deadline and
  `band_listing` runs after every window, so a stats-driven scan on prod would hang the whole weekly
  job — hence the plan-probe on an analyzed prod snapshot owed before 429's schedule (NEXT above).
  The head-version profile / notice lookups are per-Tender chain seeks on `tender_versions` (turso
  picks the `(tender_id, caused_by_notice_id)` autoindex and filters `seq`), not PK seeks; the doc
  comment says so.
- **Tests** (focused, gate flags + package set, GATE-EXIT=0 after the review fixes):
  `the_band_listing_shows_a_head_that_repeats_nowhere` (scratch DB: a once-only EUR 33.26 bn head is
  in section 16 and not in section 10, which does list a 10× repeated PLN value; €9,999,999,999.99
  out, exactly €10 bn in, NULL head out; top-down order `[7, 2, 8, 9, 1, 4]`, one row per Tender;
  EUR/GBP/SEK groups; 10³ lot estimate, 10⁴ lot award, `II.1.7 )` corrigendum; a 1.00 placeholder is
  no sibling; the DKK/EUR tie resolves to the read layer's row; a stale head is listed figure-less;
  the plan, before and after ANALYZE), `the_band_listing_groups_by_currency_and_merges_its_signals`
  (render: grouping, dedupe guard, the two sibling sources merged, the footer's era caveats, the
  stale-head group and JSON nulls, LISTING FULL before the rows at 1,000 and JSON `full`, `none`,
  UNMEASURED). The other data-quality lib tests touched by the new label
  (`every_report_field_is_read_by_the_renderer`, `the_sections_render_in_numbered_order`,
  `queries_are_labelled_in_execution_order`, …) re-run after the fixes: all 61
  `data_quality::tests` pass (GATE-EXIT=0). `ops/check.sh` NOT yet run.
- Also: `data_quality.rs`'s `sentinel_amounts_sql` doc no longer sends the low/unrepeated tail to
  "the open half of issue 366" (it names 380 and this section). The other two stale pointers
  (`api-dq-review-2026-09-15.md:350–353`, `canonical.rs:1501–1506`) are untouched — the second is in
  `store`, and a comment edit there re-hashes every crate above it.
- Docs: `docs/operations.md`, "Section 16, the head-value band", beside the report-reading recipe.

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/5592948 | jq -c .value

This verifies the last unit (6): the band adjudicated, deployed and drained. Unit 1 has its own check above; units 2
and 3 are recorded here as they land.

- **open** (2026-10-01 12:41 UTC): `{"cents":900000000000,"currency":"GBP"}`. That is £9 bn for "Vending Machine
  Services": exactly 10³ × each lot's £9,000,000 estimate, and the figure the buyer's own corrigendum 071343-2017
  lowered to £9,000,000.00.
- **done**: `{"cents":900000000,"currency":"GBP"}`. Either 4a or 4b refuses or supersedes the £9 bn figure, so the
  election falls to the £9 M lot estimates.
