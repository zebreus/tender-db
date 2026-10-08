# 471 — r209 amounts are never checked against their `@FMTVAL`, and nothing adjudicates the €10–100 bn head-value band (366 units 5 and 6, dropped when 366 closed)

Status: ready-for-agent — 2026-10-08: unit 5 DECIDED (no outside signal: buyer history catches 6 of 20 sample errors at zero false flags and fails structurally; single figures with no in-Tender partner are served as published under a /docs caveat; see "Unit 5 — decision (2026-10-08)"). 4(c) closed caveat-only (dropped decimals leave no in-notice signal). Unit 6 done (band 332 → 305). The mechanisms the sample exposed moved: the text-era min/max run-together, the unread V.4 partner and the free-text corrigendum to issue 491; the ×100 (k = 2) slip to issue 492. NEXT: gate and deploy the docs caveat, then done.
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

## Unit 1 deployed and measured (2026-10-06)

- **Deploy.** `b1fcb29`, gated green on that exact revision.
- **Run.** Data-quality job 2019 (`dry_run: false`, 4,833 s). The default is a dry run: job 2018 stored nothing. The
  report text is saved as `.scratch/tender-db/471-values/section16-dq2019-2026-10-06.txt`.
- **Done check.** `grep -c 'issue 471'` → 1. The summary line `Tender(s) in N currenc…` → 1. `UNMEASURED — the
  .band_listing` → 0.
- **Section 16.**
  - 332 Tenders in 9 currencies: GBP 171, EUR 131, DKK 7, RON 7, SEK 5, CZK 4, PLN 4, HUF 2, BGN 1.
  - 22 rows carry an exact 10^k partner, for example 6941544 at £80 bn against £80 m (10³), and 4972513 at
    £43.6 bn against £43.6 m (10³).
  - 0 elected rows were NOT FOUND, and there was no LISTING FULL.
  - Most GBP rows are FTS OCDS framework ceilings of £30–80 bn with no partner, which is plausible for UK national
    frameworks and nothing is adjudicated.
- **Next.** Unit 2, the gated archive read (`@FMTVAL` against the stored amount) for the r208/r209 rows. Then a
  verdict pass on the 22 rows with a 10^k partner.

## Unit 2 — the archive read (2026-10-06): answered, and the answer splits by era

These are bounded member reads, one stream per monthly tar. The members are saved under
`.scratch/tender-db/471-values/archive/`.

| tender | member | what the XML says | verdict |
|---|---|---|---|
| 4490098 (r208 profile; the member is an `R2.0.7.S03.E01` form) | `2011-07.tar` → `2011-07-15.tar.gz` → `20110715_134/222043_2011.xml` | `<VALUE_COST FMTVAL="49700000000000000">49 700` (twice), beside `FMTVAL="4970000">49 700` and `FMTVAL="5000000">50 000` — these two are ALSO wrong, 10² above their texts (corrected by the unit-3 review; the first reading took them as agreeing) | **`@FMTVAL` disagrees with its text.** The text is 49,700 EUR; the attribute is 4.97×10¹⁶. The stored figure (4.97×10¹⁸ cents) is the attribute, so the error is TED's attribute, not the publisher's. |
| 6581010 (r209) | `2020-01.tar` → `01/20200120_2020013.tar.gz` → `20200120_13/00026682_2020.xml` | `<VAL_TOTAL CURRENCY="EUR">59733280000.00` (no FMTVAL); lot `46549800.00` | The publisher's own text. The first notice published 597,332,800.00 (10²). |
| 6941544 (r209) | `2021-10.tar` → `20211027_209/548977_2021.xml` | `<VAL_TOTAL CURRENCY="GBP">80000000000.00` beside `<VAL_ESTIMATED_TOTAL CURRENCY="GBP">80000000.00` (no FMTVAL) | The publisher's own text, with a 10³ partner in the same notice. |
| 6843260 (r209) | `2021-01.tar` → `20210111_006/010347_2021.xml` | `<VAL_TOTAL CURRENCY="GBP">3097158480` and a total of `9318469680`, both without a decimal point, while sibling lots read `34647558.40`, `26438016.80`, `1127536.80` (no FMTVAL) | The publisher's own text: a dropped decimal point. The buyer's F14 prints 3 097 158.48. |

- **Answer.**
  - **The r208 profile can carry a wrong `@FMTVAL` behind a correct text** (the exhibit is an R2.0.7 form; "r2.0.8"
    below means the `ted-export-r208` profile, which covers R2.0.7 and R2.0.8). Unit 3 (parse both, mark the fact when they
    disagree) is needed for r2.0.8.
  - **The three r2.0.9 members read publish no `@FMTVAL` on these values,** so their band errors are the publishers'.
    (Unit-3 review: this holds for those 3 members, NOT for the profile — the committed r209 F06 and F18 fixtures do
    carry `@FMTVAL` on `VALUE_COST`, agreeing with their texts.) Units 4–5 (the
    in-tender signals) carry them.
- **A third signal for unit 4.** 010347 is a dropped decimal point: an integer text of ≥ 9 digits whose siblings in
  the same notice and currency carry 2 decimals, and whose value /100 matches nothing. It is weaker than the exact
  10^k partner. Record it and decide it on unit 1's listing.
- **Next.** Unit 3 for r2.0.8. Measure how many r208 amount elements carry an `@FMTVAL` that disagrees with their
  text, as a bounded member sample over the r208 band rows (34 tenders), before building. Then unit 4.

## Unit 3 measurement (2026-10-06): the r2.0.8 `@FMTVAL` defect is a three-week window in July 2011

**Method.** `.scratch/tender-db/471-values/fmtval.py` and `fmtval2.py` stream a monthly tar (nice/ionice, 137–196 MB
each). They compare every element's `FMTVAL` with its text parsed as a number. The parser takes the attribute when
present (`r209/rules.rs:63`, "Money: `@FMTVAL` if present (defence), else the element text"), so a disagreement is a
stored wrong amount.

| month | value elements | disagree | exact 10^k, k ≥ 2 | where |
|---|---|---|---|---|
| 2008-01 … 2010-07 (quarterly) | 0 | — | — | no `FMTVAL` in these months' formats |
| 2011-01 | 102,880 | 76 | 0 | 10⁻³ only: the script's thousands-separator ambiguity, not a defect |
| 2011-04 | 95,791 | 138 | 46 (10¹²) | one batch |
| 2011-05 | 90,954 | 70 | 46 (10¹⁰) | 2011-05-26 |
| 2011-06 | 91,517 | 106 | 69 (10⁴, 10¹⁰) | 2011-06-03 |
| **2011-07** | **106,288** | **6,253 (5.9 %)** | **≈5,900 (10² … 10¹⁴, even powers)** | **17 daily packages, 2011-07-08 … 2011-07-30** (07-15: 943, 07-12: 805, 07-28: 782, 07-20: 759 …) |
| 2011-08 | 98,937 | 74 | 46 (10⁴) | 2011-08-02 |
| 2011-09, 2011-10, 2012-01/04/07/10 | ~100k each | 3–66 | 0 | parse-ambiguity residue only |
| 2013-03, 2014-06 | 108k, 111k | 24, 0 | 0 | clean |

**Conclusions.**
- **The defect is TED's July-2011 generator.** It affected about 5.9 % of value elements in 17 daily packages, plus
  four isolated 46/69-element batches in April–August 2011. The attribute differs from the text by an exact even
  power of ten, and the text is right (4490098: text `49 700`, attribute 4.97×10¹⁶).
- **Fix for unit 3, in `r209/parse.rs`:**
  - keep both representations (ADR-0004);
  - when the text parses unambiguously and the attribute/text ratio is an exact 10^k with |k| ≥ 2, adopt the TEXT and
    mark the fact `fmtval_mismatch`;
  - in any other disagreement keep today's attribute but mark it, so it is visible and not silently trusted.
- **Re-parse scope.** Only the r208 packages 2011-04 … 2011-08, at most ~25 daily packages, or 2011-07-08 … 07-30 plus
  four days. Not the era's 15 M amounts, so the notice-list `reparse` form in step 3 is not needed. Run `reparse` over
  `ted-export-r208` from that window's fetch floor with a package count.
- **Wider than the band.** Most of the ~6,000 wrong amounts are below €10 bn, so they never showed in section 16, but
  they distort every 2011 value statistic.

## Unit 3 — landed (2026-10-06, in the tree: not committed, not deployed, not re-parsed)

> Superseded on the canonical marker by "Unit 3 — decision (2026-10-06)" below: a corrected figure is no longer
> marked `fmtval_mismatch` and IS elected. Superseded in part by "Unit 3 — review fixes" below: only attribute = text × an exact EVEN 10^k (k ≥ 2) adopts and
> marks; every other disagreement is now UNMARKED (`.FMTVAL_TEXT` beside it), and the attribute compares in i128.

- **Parse** (`crates/ingest/src/r209/value.rs`, `parse.rs` `Rule::Amount`). `value::read_amount(@FMTVAL, text)`:
  no attribute → the text, as before; attribute not a number, text not an UNAMBIGUOUS number
  (`value::display_cents`), or the two equal → the attribute, unmarked, as before. Otherwise a mismatch:
  attribute/text an exact `10^k`, `|k| >= 2` → the TEXT's cents are stored; any other disagreement → the
  attribute's. Either way `Walk::emit_mismatched_amount` files the UNADOPTED representation, raw, as a
  text row beside the amount — same section, same ordinal, field id + `.FMTVAL_MISMATCH`
  (`value::FMTVAL_MISMATCH_SUFFIX`). Both representations stay in the parse layer (ADR-0004: nothing
  published is dropped). Suppressed together with the amount in a translation copy.
  - `display_cents` reads space/NBSP/narrow-NBSP thousands, `20 550,54`, `13260.00`, `1.234.567`,
    `1.234,56`, `1,234.56`; it refuses a lone `.`/`,` before exactly three digits (`1.234`), any
    non-3-digit group, letters/currency signs. A refused text means "nothing to check against" — the
    attribute is read and nothing is marked (the measurement's 2011-01 "10⁻³" residue is this class).
  - The early-R2.0.8 `Rule::Section` `@FMTVAL` (on `AWARD_AND_CONTRACT_VALUE`) has no text to check;
    `Rule::Number`'s `@FMTVAL` is not checked (no measured defect; out of this unit).
- **Fold** (`crates/ingest/src/project.rs`). `NoticeState::read` collects `(section, field, ordinal)` of
  every `.FMTVAL_MISMATCH` row and marks the paired `Fact::Amount` `quality = 'fmtval_mismatch'`
  (`store::QUALITY_FMTVAL_MISMATCH`, beside 372's `QUALITY_WITHHELD` in `canonical.rs`; `withheld` wins
  if both apply). The mark rows themselves fold to nothing, and `has_destination(Text)` reads them, so
  the unmapped-fields diagnostics do not list them as dropped.
- **Consequence, decided by reuse rather than new code:** every reader of `quality` treats any value as
  "not a figure" — `head_value_eur_cents` skips it (`quality.is_none()`), and `/v1` serves
  `value: null` + `quality`. So even an ADOPTED text (the right figure, by the measurement) is not
  elected and not served as a value. That is the conservative direction (the source contradicts
  itself); if unit 6's re-read shows heads lost that only the adopted text could carry, splitting the
  marker (`fmtval_rescaled` electable vs `fmtval_mismatch` not) is the follow-up. The legacy lot-result
  `awarded_cents` (`read_legacy_results`, `direct_cents`) takes the adopted figure and has no quality
  column, so it is corrected but unmarked.
- **Tests** (focused, gate flags + package set):
  - `crates/ingest/tests/project.rs::an_fmtval_that_disagrees_with_its_element_text_does_not_reach_the_head`
    — the real member `222043_2011.xml`, committed as
    `crates/ingest/tests/fixtures/r208/f03-fmtval-mismatch-222043-2011.xml`, ingested + projected: no
    amount row above 1,000,000.00 EUR, every `fmtval_mismatch` row holds an adopted text (4,970,000 /
    5,000,000 cents), and the head is NULL. Measured on the fixture: an award notice files ONE canonical
    amount here (`result_value`, 4,970,000 cents, marked — the coded `VALUES` block and the
    `INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT` figures are not canonical amounts of an award form), so
    marking the adopted text leaves 4490098 with no elected head rather than a corrected one. That is
    the "Consequence" above made concrete; it is the first thing to rule on before the re-parse.
  - `crates/ingest/tests/r208.rs`: `a_scaled_fmtval_yields_to_its_element_text_and_is_kept_beside_it`
    (every `VALUE_COST` of the member is 10¹² or 10² off and pairs with exactly one mark holding the raw
    attribute), `an_agreeing_fmtval_is_unchanged_and_unmarked` (the member with agreeing attributes:
    same cents, no mark row), `an_ambiguous_element_text_leaves_the_fmtval_as_it_was` (`49.700` beside
    the 10¹² attribute: the attribute is read, unmarked). `every_r208_fixture_is_consumed_exhaustively`
    now counts 12 fixtures.
  - `r209::value` lib tests: `a_display_amount_is_read_only_when_its_decimal_point_is_unambiguous`,
    `an_fmtval_is_overruled_only_by_an_exact_power_of_ten`.
- **Re-parse runbook**: `docs/operations.md`, "Re-parsing the July-2011 `@FMTVAL` cohort (issue 471
  unit 3)" — r208 packages 2011-04 … 2011-08 only, a one-package probe on `2011-07` re-reading
  `/v1/notices/12376354/content`, then the rest, ONE `project`, and the exhibit
  `/v1/tenders/4490098` (expected: no attribute figure; no head unless an earlier notice in the chain
  carries an unmarked one).
- **Next.** `ops/check.sh`, commit, deploy; the re-parse per the runbook; then unit 4.

## Unit 3 — review fixes (2026-10-06, in the tree: not committed, not deployed, not re-parsed)

An 11-finding review of the unit-3 change; each verified against the code before fixing.

- **F1 (fixed) — the 10¹²/10¹⁴ scales overflowed before the check.** `read_amount` compared only when
  `cents(@FMTVAL)` parsed in `i64`; `FMTVAL="100000000000000000">100 000` (10¹⁹ cents) fell through as raw text,
  unmarked, its correct text dropped. Now the attribute is read by `value::wide_cents` (same normalisation and
  rounding as `cents`, in `i128`; shared `value::normalize`), so an overflowing exact scale adopts its text. Tests:
  lib `an_fmtval_is_overruled_only_by_the_measured_scale_error` (10¹² × 100 000, 10¹⁴ × 922,34, a non-scale
  overflow), r208 `a_scaled_fmtval_too_large_for_the_stored_integer_still_yields_to_its_text`.
- **F2 + F8 (fixed) — the rule went past its evidence.** Adoption is now ONLY attribute > text by an exact EVEN
  `10^k`, `k ≥ 2` (`value::scaled_by_even_power_of_ten`) — the measured direction (fmtval.py's ratio is attr/text,
  every exact k positive and even). A text 10² ABOVE its attribute (010347's dropped-decimal signature) and odd
  powers (10³, 10⁵) keep the attribute. With the adopted class restricted to the measured shape, the unmarked
  lot-result channel (`read_legacy_results` → `awarded_cents`, no quality column) now only ever takes a measured
  correction; carrying the mark onto lot results would need a schema column — not done, recorded here.
- **F5 + F7 (fixed by narrowing) — the "other disagreement" mark was unmeasured.** It nulled served values for every
  FMTVAL/text disagreement in r209 and r208 2014-07…2018 and for the ~400 non-10^k disagreements inside the window
  (fmtval2.py's `num()` is not `display_cents`). Now that class is `AmountReading::Disagrees`: the attribute is
  read exactly as before and NOT marked; the text is filed beside the amount as `.FMTVAL_TEXT`
  (`value::FMTVAL_TEXT_SUFFIX`, same section/ordinal pairing) — parse-layer evidence only (ADR-0004), countable
  after a re-parse before anyone decides to mark it. The fold skips both suffixes (`project::is_fmtval_beside_row`)
  and `has_destination(Text)` claims both. A scaled attribute with no currency in scope keeps today's raw-text row
  plus a `.FMTVAL_TEXT` row, unmarked. This REVISES the measurement section's "any other disagreement … mark it".
- **F3 (accepted as intended; runbook corrected).** Verified: `supersede` keys on `("amount", field)` and ignores
  `quality`, so a later marked `result_value` replaces an earlier unmarked one and the head can go NULL. Kept (the
  latest notice contradicts itself; same as `withheld`). `docs/operations.md` step 6 now says so instead of "unless
  an earlier notice of the chain carries an unmarked figure"; unit 6's re-read should count those Tenders.
- **F4 (fixed).** `v_tender_amounts` now selects `a.quality`; the `/v1/sql` table note and a new `*.quality`
  column note describe both values. Asserted in `an_fmtval_that_disagrees_with_its_element_text_does_not_reach_the_head`.
- **F6 (fixed).** Runbook step 4: ONE job over all five months (`after` = lowest id − 1, `packages` = 5; 07 sits in
  the middle, so `packages` = 4 would never reach 08 — verified against `Db::reparse_packages`: id > after, fetch-id
  order, capped), or one job per package when ids are not contiguous.
- **F9 (fixed).** Fold test of the unmarked branch and of a no-currency mismatch
  (`project.rs::an_fmtval_disagreement_outside_the_measured_shape_folds_as_before`: attribute folded, unmarked,
  elected; no currency → no amount, no mark); parse tests `a_disagreement_outside_the_measured_shape_keeps_the_attribute_and_files_the_text`,
  `a_scaled_fmtval_with_no_currency_in_scope_stays_raw_text`; zero beside rows asserted on every committed
  FMTVAL-bearing fixture (`r208.rs::no_other_committed_r208_fmtval_fixture_files_anything_beside_its_amounts`,
  `r209.rs::the_committed_fmtval_fixtures_agree_with_their_texts` — the r209 profile path, incl. f18's
  `2162630.19` / `2 162 630,19`), and the internal-ojs path (`internal_ojs.rs::internal_ojs_amounts_read_their_text_and_file_nothing_beside`:
  R2.0.5 publishes no `@FMTVAL`, text read as before).
- **F10 (fixed).** `tests/fixtures/README.md`: totals (116 files, 3.0 MB; r208 12 files, 524 KB — both were already
  stale before this unit) and a provenance row for `f03-fmtval-mismatch-222043-2011.xml` (monthly archive member,
  byte-identical to the unit-2 save). `QUALITY_WITHHELD`'s doc, the `tender_version_amounts.quality` schema comment
  and the public `/docs` Amounts list now name `fmtval_mismatch`. (Three older r208 fixtures —
  `f03-099900-2018`, `f03-annexd-neg-022211-2011`, `veat-294050-2011` — still lack README rows; not this unit's.)
- **F11 (fixed in text).** The exhibit is `R2.0.7.S03.E01` under the `ted-export-r208` profile; the code docs, the
  runbook and the Unit 2 table now say so, and the table notes `FMTVAL="4970000"` / `"5000000"` are 10² off.
- **Tests** (gate flags + package set, output to file, GATE-EXIT read): `--test r208 --test r209 --test internal_ojs
  --test sql --test project` GATE-EXIT=0 (15 / 19 / 8 / 17 / 93 passed); `--lib` GATE-EXIT=0 (ingest 356, model 3,
  store 152, app 182); `--test api --test withheld_fields_view --test stage4_schema --test view_pushdown_probe`
  GATE-EXIT=0 (78 / 1 / 1 / 2). `ops/check.sh` NOT run.
- **Next.** `ops/check.sh`, commit, deploy; the re-parse per the runbook; after it, a bounded count of
  `.FMTVAL_TEXT` rows in the five months (the parse layer of those notices) to size the unmarked class; then unit 4.

## Unit 3 — decision (2026-10-06): a corrected figure is an ordinary amount

> This revises "Unit 3 — landed" (Consequence) and the review's F3/F4 wherever they assume a `fmtval_mismatch` mark.

**Owner decision.** An amount whose `@FMTVAL` was its element text scaled by an exact even `10^k` (`k ≥ 2`, the
attribute the larger) and whose text was adopted is CORRECT, and is elected and served like any other amount. The
projection no longer marks it.

**Why.**
- The correction is applied only to the exact measured shape (unit 3 measurement: TED's July-2011 generator, every
  exact ratio even and positive); every other disagreement keeps the attribute, as before. There is no residual
  doubt for a marker to carry.
- The element text is the published, human-read figure — what the notice actually says to a reader; the attribute
  is the generator's machine copy, and in this shape it is the wrong one.
- Marking it nulled heads: on the real member 4490098 lost its only canonical figure (the head went NULL), and by
  supersession a cohort notice would also have replaced an earlier good `result_value` with a null one. That
  discards correct information to express a doubt the measurement already resolved.
- The record of the correction is kept where it belongs: the parse-layer `<field>.FMTVAL_MISMATCH` text row beside
  the amount (raw attribute, same section and ordinal), unchanged — countable later, and ADR-0004 holds.

**What changed in the tree.**
- `crates/ingest/src/project.rs`: the `fmtval_mismatched` set and the `.or_else(… QUALITY_FMTVAL_MISMATCH …)` are gone;
  `quality` is set only by a withholding declaration. The `.FMTVAL_MISMATCH` / `.FMTVAL_TEXT` rows still fold to
  nothing (`is_fmtval_beside_row`) and stay claimed by `has_destination(Text)`.
- `store::QUALITY_FMTVAL_MISMATCH` removed (constant, `lib.rs` re-export, `canonical.rs` docs/schema comments): the
  `quality` vocabulary is only `'withheld'` again. `v_tender_amounts` keeps `a.quality` (withheld is useful there);
  its view comment, the `/v1/sql` table note and the `*.quality` column note now name only `'withheld'`. `/docs`
  Amounts: the `quality` bullet names only `withheld`, and a new bullet says a legacy amount is read from its
  printed text when the attribute is that text × an exact even power of ten, served like any other.
- Parse-layer docs (`r209/value.rs`, `parse.rs`, `rules.rs`) say the `.FMTVAL_MISMATCH` row is the record of the
  correction, not a mark. `docs/operations.md` runbook: the rescaled bullet, the scope note and step 6 rewritten — no
  head is nulled by this class, and the old supersession caveat about marked figures no longer applies.
- Tests: `project.rs::an_fmtval_that_disagrees_with_its_element_text_does_not_reach_the_head` is now
  `an_fmtval_scaled_by_ten_to_the_k_yields_to_its_text_and_is_elected`: on the real 222043 member, no amount row above
  €1M, no `quality` on any amount, `result_value` = 4,970,000 cents, ONE `notice_texts` row
  `….FMTVAL_MISMATCH` = `49700000000000000`, and tender 4490098's head (`current_value_eur_cents`) = 4,970,000 —
  €49,700, verified by the run. The r208 parse tests are unchanged (their "marks" are the parse-layer rows).
- Focused runs (gate flags + package set, output to file): `--test r208 --test r209 --test project --test sql --test
  internal_ojs` GATE-EXIT=0 (15 / 19 / 93 / 17 / 8 passed); `--lib` GATE-EXIT=0 (ingest 356, model 3, store 152,
  app 182); `--test api --test withheld_fields_view --test stage4_schema --test view_pushdown_probe` GATE-EXIT=0
  (78 / 1 / 1 / 2). `ops/check.sh` NOT run.
- **Next.** `ops/check.sh` → commit → deploy → reparse r208 2011-04…08 (one job, `packages` = 5, per the runbook) →
  one `project` → re-read `/v1/tenders/4490098` (expect €49,700) and section 16 of the data-quality report; then the
  `.FMTVAL_TEXT` count; then unit 4.

## Unit 3 deployed and re-parsed (2026-10-06)

- **Deploy.** `25b0d10`, gated green.
- **Fetch ids.** `/v1/sql` on `notices.fetch_id` gives 2011-05 → 181, 2011-07 → 179, 2011-09 → 177. Fetch ids run
  backwards in time, so 2011-04 … 08 are 182 … 178, contiguous. `fetches` itself is not queryable, so the runbook's
  step 2 reads `notices.fetch_id` for a handful of sampled ids instead.
- **Probe.** Reparse 2020 (`after` 178, `packages` 1, `reclaim_only`): 36,485 notices, 0 unmatched, 0 re-keyed.
  `/v1/notices/12376354/content` now serves `TED-VALUE_COST` 4,970,000 with `TED-VALUE_COST.FMTVAL_MISMATCH`
  `49700000000000000` beside it.
- **Wet.** Reparse 2021 (`after` 177, `packages` 5): 172,281 notices across 5 packages, 0 unmatched, 0 re-keyed,
  0 failing. It stamped 1,635,859 r208 Tenders stale, by profile as documented.
- **Fold.** Project 2022: `172281 notices → 176926 tenders … 167133 written, 9793 unchanged`, 2,349 s. It did not
  take the full fallback, so it is not a corpus-wide timing for issue 488.
- **Exhibit 4490098.**
  - `result_value` is now 4,970,000 cents (€49,700); before, it was the 4.97×10¹⁸ attribute.
  - `estimated_value` is 5,000,000 (€50,000), and the lot award is 4,970,000.
  - The served head stays the €50,000 estimate, as before. The old figure was above `IMPLAUSIBLE_EUR_CENTS` and never
    elected; the corrected one is plausible, and the election still prefers the estimate here.
- **Next.**
  - Re-read section 16 after the next data-quality run. The July-2011 figures were mostly below the band, so a small
    change is expected.
  - Unit 4: the in-tender signals (exact 10^k sibling, the dropped decimal point, F14 value supersession).

## Unit 4 — adjudication (2026-10-06)

**Input.** These are the 22 section-16 rows (data-quality job 2019) that have an exact 10^k partner. Each row got two
independent verdicts (SCALE_ERROR / GENUINE / UNSURE), and each judge read `/v1/tenders/{id}` and the notices'
`/content`. On the same day, each row's served `amounts` and `lot_results` were re-read from `/v1/tenders/{id}`.
"×1" in the last column means the big figure sits in one canonical amount field; "×2" means it sits in two different
fields.

| tender | k | head field (the big figure) | partner field | verdict 1 | verdict 2 | agreed? | big figure in amounts |
|---|---|---|---|---|---|---|---|
| 6941544 | 3 | `result_value` (RES-1 VAL_TOTAL, CAN 548977-2021) | `estimated_value` (VAL_ESTIMATED_TOTAL, CN + CAN), same notice | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| 4972513 | 3 | `result_value` (CAN 425454-2014 VALUE/VALUE_COST) | `estimated_value` | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| 8452561 | 3 | `result_value`, procedure VAL_TOTAL (VEAT 129760-2020) | `result_value` RES-1 + lot award, same notice | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| **8400892** | 3 | `estimated_value` (procedure, CAN 231141-2017; also PIN, CN, every RES VAL_ESTIMATED_TOTAL) | `result_value` RES-1..3 VAL_TOTAL, same notice | **GENUINE** | **GENUINE** | yes | **×2** (`estimated_value` + `result_value`, procedure VAL_TOTAL) |
| 6803400 | 3 | `result_value`, procedure TED-VALUE (CAN 634169-2020) | lot estimates/awards, LOT-4..7 £10 M | UNSURE | SCALE_ERROR | **no** (both say the head is wrong; the real total is the CN's ~£500 M, not the partner) | ×1 |
| 5592948 | 3 | `result_value` (CAN 295406-2016; corrected by F14 071343-2017) | `estimated_value` LOT-1..6 | SCALE_ERROR | SCALE_ERROR | yes | ×1 (also every lot award; lot results are not counted) |
| 6988280 | 4 | `result_value`, procedure VAL_TOTAL (CAN 291034-2021) | RES-3 lot award €10 M | SCALE_ERROR | SCALE_ERROR | yes (real value UNKNOWN: every lot value is a power-of-ten placeholder) | ×1 |
| 577127 | 3 | `framework_maximum` BT-271-Lot LOT-0001 | `estimated_value` BT-27-Lot LOT-0001, same notice | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| 6581010 | 4 | `result_value`, procedure (CAN 026682-2020) | `estimated_value` (CN 382778-2019) | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| 4685893 | 3 | `result_value`, procedure GLOBAL (CAN 011602-2012) | lot award RES-1, same notice | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| 8822396 | 3 | `result_value` BT-161 / BT-720 LOT-0001 | `estimated_value` BT-27-Lot LOT-0001, same notice | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| 5094790 | 3 | `estimated_value` (CN 374171-2013) | `result_value` (CAN 171786-2014) | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| 224156 | 3 | `framework_maximum` BT-271-Lot LOT-0002 | `framework_maximum` BT-271-Lot LOT-0001 (a sibling lot) | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| 568960 | 3 | `framework_maximum` BT-271-Lot LOT-0000 (the only lot) | `estimated_value` + `framework_maximum`, procedure | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| 404296 | 3 | `estimated_value` BT-27-Procedure | `result_value` BT-161 (+ 3× BT-720, 3 contracts), same notice | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| 4578779 | 6 | r208 `@FMTVAL` (CN 222050-2011) | VALUE/VALUE_COST text, CN + CAN | SCALE_ERROR | SCALE_ERROR | yes, **already drained by unit 3** (serves €20,000) | — |
| 4785037 | 3 | `result_value`, GLOBAL (CAN 284708-2013) | `estimated_value` (CN 189251-2012) + 2 lot estimates | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| 6577862 | 3 | `estimated_value` (CN 375960-2019) | `result_value` (CAN 374021-2020) + RES-1 | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| 4581663 | 4 | r208 `@FMTVAL` (CN 226981-2011) | VALUE/VALUE_COST text, CN + CAN | SCALE_ERROR | SCALE_ERROR | yes, **already drained by unit 3** (serves €1,500,000) | — |
| 6721266 | 3 | `estimated_value` (head: modification 554082-2023) | CN 062874-2020 VAL_ESTIMATED_TOTAL, **an earlier version only** | SCALE_ERROR | SCALE_ERROR | yes (verdict 2: neither notice it read carries the ×1000 figure; origin not read) | ×1 |
| 6852637 | 3 | `estimated_value` (RES-6 VAL_ESTIMATED_TOTAL, CAN 613978-2020) | the same field in RES-1..5 + procedure total | SCALE_ERROR | SCALE_ERROR | yes | ×1 |
| 1163733 | 3 | `estimated_value` LOT-0001 (the only lot) | `estimated_value`, procedure | SCALE_ERROR | SCALE_ERROR | yes | ×1 |

**Counts.**
- 22 rows: 21 have both verdicts the same, and 1 is split.
- Agreed SCALE_ERROR: 20. Two of them (4578779, 4581663) were already corrected by the unit-3 re-parse, so 18 are
  still in the band.
- Agreed GENUINE: 1 (8400892, a £10.8 bn London-wide housing framework ceiling).
- Split: 1 (6803400, UNSURE against SCALE_ERROR). Both judges say the £10 bn head is wrong. They disagree only because
  the 10³ partner is one lot's figure and not the real total.
- 18 of the 22 have k = 3, 3 have k = 4 and 1 has k = 6.

**Separators that do NOT work (checked against the table).**
- **k.** The GENUINE row has k = 3, like 17 of the scale errors.
- **Field kind ("estimated total over a result").** 6577862, 5094790 and 404296 are the same shape as 8400892: a big
  procedure estimate over a small award total. All three are agreed scale errors.
- **Same notice against a different notice.** 8400892's pair is in one notice, and so are 6941544, 8452561, 577127,
  224156, 404296, 6852637 and others.
- **Scope ("a procedure figure over one of several lots or results").** This would exempt 8400892, but it also exempts
  5592948, whose partners are only the six lot estimates. It also exempts 6988280 and 6803400.
- **"The big figure is the only one at its scope across notices."** 6577862's 18 bn also repeats from the CN into the
  CAN's RES-1 estimate, so it would be exempt too.

**What does separate them: corroboration.**
- In 8400892, the big figure is the only one published in two different canonical amount fields of the tender. They are
  `estimated_value` (procedure VAL_ESTIMATED_TOTAL) and `result_value` (procedure VAL_TOTAL = 10.8 bn). The 10.8 m is
  the odd one out, as both GENUINE verdicts argue.
- In every agreed scale error, the big figure sits in exactly ONE canonical amount field. The publisher made one slip in
  one field.
- Lot results do not count as corroboration. `lot_results.awarded` is derived from the same RES VAL_TOTAL / BT-720
  element as `result_value`, so it is not an independent declaration. 5592948's five £9 bn lot awards and 8822396's
  LOT-0001 award would otherwise exempt them.

**Decision: the rule for 4(a).**

A positive `Fact::Amount` F in currency C is **refused from the head election** when both of these hold:

1. There is a positive figure P of the same Tender in currency C with F = P × 10^k exactly, where k ≥ 3, compared in
   i128 cents.
   - P can come from any version's `tender_version_amounts` or `tender_version_lot_results.awarded_cents`. This is the
     partner search section 16 measured and that these 22 rows were judged on.
   - P must be above `SENTINEL_AMOUNT_CEILING` (10.00 as published).
2. F's exact cents value in C is NOT also carried by a canonical amount with a different `field`
   (`estimated_value` / `result_value` / `framework_maximum` / …) in the head version. Lot-result awards do not count.

The rule applies to every amount, not only the current head, so a second scaled figure falls too. Example: 6988280's
€10 bn `result_value` also has the €10 M partner, so its head falls to €1 bn, which is still a placeholder. Refusal goes
through the same `head_value_eur_cents` skip as `withheld`. Whether it is a stored `quality` value or computed at
election time is the implementer's call. The unit-3 decision kept the `quality` vocabulary at `'withheld'` only, so
adding to it needs that decision revisited.

- **Why the any-version partner.** Only 6721266 needs it, because its 458,962,965.47 partner is superseded in the head
  version. A same-version rule leaves it in the band. Verdict 2 suspects our side made the ×1000. The member read of
  554082-2023 is still owed, but the rule should not wait on it.
- **The round-mantissa case the pin was asked to rule on** ("a €2 bn framework over a €2 M lot") is **refused unless
  corroborated**.
  - In this sample every uncorroborated round ceiling was an error: 224156, 577127 and 568960 (`framework_maximum`),
    and 6803400 and 6988280.
  - The only genuine ceiling, 8400892, was corroborated.
- **Pin.** `a_figure_exactly_ten_to_the_k_above_a_sibling_is_not_elected` (canonical tests) covers these cases:
  - 224156's sibling-lot `framework_maximum` is refused.
  - 577127's same-lot `framework_maximum` over `estimated_value` (a round mantissa) is refused.
  - 8400892's shape (`estimated_value` = `result_value` = 10.8 bn over a 10.8 m `result_value`) stays elected.
  - 5592948's shape (a big `result_value` and big lot awards over lot estimates) is refused, and the head falls to
    £9 M. That is the Verify's "done".
  - The superseded-partner case (6721266's shape) is refused.
  - k = 2 (10²) is NOT refused.
  - A 1.00 placeholder is no partner.

**Precision on this sample.**
- **As written in 4(a)** (no corroboration exemption): it refuses all 20 rows still in the band. There is 1 false
  positive (8400892), so precision is 18/19 on the agreed rows, or 19/20 counting 6803400, whose head both judges call
  wrong.
- **The decided rule:** it refuses 19 and keeps 8400892. False positives: **0**. Every refused head is wrong per both
  judges, so precision is 19/19, or 18/18 on agreed rows. Recall on the agreed scale errors still in the band is 18/18.
- **Caveat on n.** One genuine row decides the exemption. A genuine framework ceiling that is published in one field
  only would be a false positive, and this sample cannot show how common that is. Unit 6's re-read lists every row the
  rule removes, so a reviewer can catch one.
- **Refused is not the same as corrected.**
  - 6988280's head falls to a €1 bn placeholder. Both judges say the real value is unknown.
  - 6803400 falls to £50 M, not the CN's ~£500 M, because that figure was superseded.
  - 8822396's lot awards for lots 2–4 (about 10³ over their estimates, but not exactly) stay as they are.

**The dropped-decimal signal (unit 2, 010347).** It does **not** appear among these rows.
- None of the 44 verdicts reports an integer element text beside sibling figures that carry decimals.
- The members read in unit 2 for 6941544 and 6581010 both print `.00`.
- The exhibit 6843260 is below the band (£3.1 bn), so it is not in section 16.
- The only row whose source text has not been read is 6721266 (the ×1000 of a `,47` figure). Its member read decides
  whether it belongs to this class or is a parse defect of ours.
- So the signal stays recorded, with no band row to decide it on. It should not be built under 4(a).

**4(b) is unchanged by this.** 4(a) already refuses 5592948, so the Verify no longer depends on (b). Whether (b) stays
here is still open.

**Next.** Implement 4(a) as decided above, with the pin. Then read the 554082-2023 member for 6721266. Then 4(b),
5 and 6.

## Unit 4(a) — landed (2026-10-06, in the tree: not committed, not deployed, not drained)

**Where the rule lives: in the election, not in a stored `quality` marker.**
- `head_value_eur_cents` now takes the Tender's whole chain (`&p.versions` at the one fold call
  site, `write_tender`'s head update) and elects from its last version as before. It skips every
  amount `ScalePartners::refuses`, beside the existing `withheld` / `sentinel_amount` /
  ceiling / zero-conversion skips.
- Why not a marker set in the fold's projection: the rule's input is the WHOLE chain (6721266's
  partner is superseded at the head), while a `Fact` belongs to one version. A marker on version n
  that flips when version n+1 brings a partner would change a kept version's content without the
  stored chain (the fold's state key) changing, so either it goes stale or every arrival rewrites
  the prefix and emits change rows for nothing the publisher changed. It would also reopen the
  unit-3 decision that `quality` holds only what the SOURCE declared (`'withheld'`); this is our
  inference.
- One place still decides (366 unit 4): the read layer's `elected` pick finds the served row by
  `eur_cents = t.current_value_eur_cents`, so it follows with no edit; a refused and an elected row
  never share `(cents, currency)` (equal figures in a different field are corroboration, and equal
  figures in the same field share the partner and fall together).
- Section 16 shares the constants: `band_listing_sql` reads `SCALE_PARTNER_FLOOR_CENTS` (10.00)
  and `SCALE_ERROR_MIN_EXPONENT` (3) from canonical; `the_band_partner_floor_is_the_elections`
  pins them equal to `SENTINEL_AMOUNT_CEILING`.
- Poll budgets (issue 467): the rule is a synchronous call evaluated before the head update's
  `.await`, so its `HashSet` lives on the ordinary stack, not in `run_project`'s future; no local
  was added to `run_project` / `project_incremental_chunked_observed`.

**The rule as implemented** (exactly the adjudication's): F > 0 is refused when some P > 1000
cents of the same Tender and currency (any version's amounts at tender or lot scope, or any
round's `awarded_cents`) has F = P × 10ᵏ, k ∈ 3..18, by exact integer division; UNLESS an amount
of the head version (tender or lot scope) with a different `field` carries the same cents and
currency. Lot awards never corroborate. Every figure is tested on its own, so a refused figure is
still a partner of a bigger one.

**Pin.** `a_figure_exactly_ten_to_the_k_above_a_sibling_is_not_elected` (canonical tests), cases:
6941544 (×1000 result over its estimate → estimate elected); 8400892 (10.8 bn in
`estimated_value` AND `result_value` over a 10.8 m lot result + lot award → 10.8 bn stays);
5592948 (9 bn result + 9 bn lot awards over six 9 m lot estimates → 9 m: a lot award does not
corroborate); k = 2 kept; the round mantissa (€2 bn `framework_maximum` over a €2 m lot → refused;
the same ceiling also in `estimated_value` → kept); 577127 (same-lot ceiling over estimate);
224156 (sibling-lot `framework_maximum`); 6721266 (partner only in an earlier version: elected
alone, refused with the chain); 6988280 (two scaled figures both fall, the 10² one stays); 1.00
and 10.00 are no partner, 10.01 is; another currency and a near-miss are no partner.

**Not covered, recorded.** The per-lot value `summarise` serves (read.rs, the lot pick that calls
`sentinel_amount`) does not apply the rule: 224156's LOT-0002 keeps its refused ceiling as that
lot's value. Applying it there needs the Tender's partner set at read time; left for unit 6's
re-read to size. Below the band the rule lands lazily (no epoch bump), on an unmeasured
population.

**Drain** (docs/operations.md, "The exact-10ᵏ election rule"): `refold-notices` over every band
Tender's head notice (~330, under the cap; bounded `/v1/sql` range seek on
`tenders_current_value_eur`), then `project`. Expected: the 18 agreed scale errors still in the
band plus 6803400 leave it (19); 8400892 stays; `/v1/tenders/5592948` serves £9,000,000.


**Tests (focused, gate flags and package set, 2026-10-06).** `a_figure_exactly_ten_to_the_k_above_a_sibling_is_not_elected`
alone: GATE-EXIT=0, 1 passed. Then the filters `head value band sentinel run_spec_futures_stay_inside elect amount`:
GATE-EXIT=0, 112 passed, 0 failed, including `the_band_partner_floor_is_the_elections`,
`the_band_listing_shows_a_head_that_repeats_nowhere`, the `head_election_agreement` read/fold pins,
`an_fmtval_scaled_by_ten_to_the_k_yields_to_its_text_and_is_elected` and
`run_spec_futures_stay_inside_their_size_budgets`. The full `ops/check.sh` has NOT been run: owed before the commit.

## Unit 4(a) — review fixes (2026-10-06, in the tree: not committed, not deployed, not drained)

A nine-finding review of the uncommitted 4(a) change. Outcomes:

1. **The rule reached every Tender, on evidence drawn from the band only (high + medium,
   two findings, one fix).** The 22 adjudicated rows came from heads ≥ €10 bn, where the big
   figure is suspect by selection. Below the band the slip often runs the other way (the SMALL
   figure typed in thousands, or a 1,000.00 / 10,000.00 placeholder beside a genuine ceiling), so
   the ungated rule would drop a genuine €5 M estimate beside a €5,000 lot result, or a €1 M
   `framework_maximum` beside a 1,000.00 award, and it would roll out lazily and unattributed
   through any later refold, rederive or epoch bump. **Fixed by a third condition**: F's EUR
   conversion must be ≥ `SCALE_ERROR_MIN_EUR_CENTS` (€10 bn), and the data-quality report's
   `BAND_FLOOR_EUR_CENTS` is now that constant (pinned in `the_band_partner_floor_is_the_elections`).
   Since the election only removes candidates and takes the max, the rule can now move ONLY a
   Tender whose elected value is in the €10–100 bn band, and only down — exactly the ~330 rows the
   documented drain re-elects. This narrows the adjudication's rule to the population it was
   measured on; the adjudication's own ruling on the round mantissa ("a €2 bn framework over a
   €2 M lot is refused") now holds at band scale (€20 bn over €20 M) and is deferred below it.
   It was decided here rather than waiting, per the triage convention, because the measurement
   needs the box and this session must not touch it. **Widening it is a decision owed to the
   below-band measurement** (NEXT in the status line): run section 16's `pow10_amount` /
   `pow10_award` partner subqueries (same floor and powers) without the band predicate, add
   `NOT EXISTS` (head-seq amount, same cents and currency, different field), window by `t.id`
   (bounded `/v1/sql` chunks or a windowed data-quality section), count the Tenders whose elected
   row would be refused by value decade and k (and how many of those rows are lot-scoped), and
   adjudicate a sample. Then either lower the gate with an explicit drain of the whole cohort
   (stamping tender ids, ≤ 1,000 per chunk) or keep it at the band.
2. **Cross-currency twin escapes (low): rejected as a code change, recorded.** The decided rule
   fixes the partner and the corroboration as same-currency. A twin in another currency keeps
   the head in the band (no false positive, a miss). Unit 6's re-read checks for a refused
   figure whose other-currency twin is still elected.
3. **Per-lot value in `summarise` (low + medium, two findings, one fix).** Fixed: the lot pick
   collects its candidates, and when any is in the band it loads the chain up to the version
   (`tender_version_amounts` + `tender_version_lot_results`, prefix seeks on `(tender_id, seq)`)
   into `ScalePartners` and skips what `refuses_amount` refuses — the fold's predicate called,
   not transcribed (389 unit 1's pattern). 224156's LOT-0002 now serves no value instead of its
   refused ceiling, so `/v1/lots?min_value=` and the served lot value agree again. Outside the
   band no query is added.
4. **No fold-path test (low): fixed.** `head_election_agreement.rs` gains
   `a_scale_slip_partnered_only_by_an_earlier_version_is_refused_by_the_fold_and_the_row`
   (6721266's two-version shape through `apply_tenders`: stored column = the smaller figure, the
   list row and `tender_detail` serve it, `?min_value` at/above it agrees; it fails if the call
   site passes only the head) and `a_refused_lot_figure_is_not_served_as_the_lots_value`
   (224156's shape through `read::lots`, plus the same shape below the band kept).
5. **Quadratic corroboration scan (low): fixed.** `ScalePartners` builds a
   `(currency, cents) → Some(field) | None (≥ 2 fields)` map of the head once; each check is
   O(1). The API is now `new` / `add_partner` / `add_head_amount` / `of_chain` /
   `refuses_amount(field, currency, cents, eur_cents)`, so the read layer feeds it from rows.
6. **Ops note understated exposure (low): fixed.** docs/operations.md now states the band gate,
   that the rule can move only band Tenders, and warns that `rederive-eur`, `reparse`,
   `refold-*` jobs and any epoch bump re-elect band Tenders they touch under the rule until the
   drain has run, with how to attribute such drops.
7. **Stack budgets / single decider (info): no change.** `ScalePartners` is still built inside a
   synchronous call evaluated before the head update's `.await`; nothing was added to
   `run_project` / `project_incremental_chunked_observed`. The read-side additions are in
   `summarise` (read path, not the fold's futures). `amounts` still lists a refused figure
   unmarked, by the decision not to grow the `quality` vocabulary.

**Tests (focused, gate flags and package set, 2026-10-06).** The four new/changed pins:
GATE-EXIT=0, 5 passed. Filters `head value band sentinel run_spec_futures_stay_inside elect amount
lot summar`: GATE-EXIT=0, 163 passed, 0 failed (incl. `run_spec_futures_stay_inside_their_size_budgets`).
Filters `window written_once reuses_the_election data_quality`: GATE-EXIT=0, 84 passed, 0 failed.
The full `ops/check.sh` has NOT been run: owed before the commit.

## Unit 4(a) deployed and drained (2026-10-06)

- **Deploy.** `900b13b`, gated green.
- **Drain.** A bounded `/v1/sql` read of the band (`current_value_eur_cents >= 1e12`) found 324 Tenders with 324 head
  notices. `refold-notices` 2023 re-queued all 324, then project 2024 ran:
  `324 notices → 504 tenders … 359 written, 145 unchanged`, 22 s.
- **Band.** 324 → **305**: exactly the 19 predicted (the 18 agreed scale errors still in the band, plus the split row
  6803400).

| tender | before | after |
|---|---|---|
| 8400892 (GENUINE, corroborated) | £10.8 bn | **£10.8 bn (kept)** |
| 5592948 | £9 bn | £9,000,000 |
| 6941544 | £80 bn | £80,000,000 |
| 4972513 | £43.6 bn | £43,600,000 |
| 8452561 | £27.6 bn | £27,600,000 |
| 6803400 (split) | £10 bn | £50,000,000 |
| 224156 | €40 bn | €40,000,000 |

- **Next.**
  - Measure the rule below the band before ever lowering `SCALE_ERROR_MIN_EUR_CENTS`.
  - 4(b): F14 value-correction supersession.
  - 4(c): the dropped-decimal-point signal.
  - Unit 5: 20905's class.
  - Unit 6: re-read section 16 after the next data-quality run.


## Below-band measurement (2026-10-06)

The €1–10 bn decade, read whole with `471-values/below-band-query.sh` (section 16's partner
subqueries without the band predicate, plus the count of distinct head-version fields carrying the
elected figure), 7 bounded `/v1/sql` reads by value range: **3,579 Tenders, 71 with an exact 10^k
(k ≥ 3) partner of the elected head, 6 of them corroborated, 65 that the rule would refuse** (64 at
k = 3, 1 at k = 4; raw rows `below-band-1to10bn-2026-10-06.json`, the 65 in
`below-band-refuse65.tsv`).

All 65 adjudicated against their notices (workflow wf_01656f0d-7c4, 8 agents, bounded reads;
`below-band-verdicts-2026-10-06.json`): **64 big-is-error, 1 genuine, 0 unclear.** The errors are
the expected shape — municipal catering, a ministry's SAP licences, a health-centre build, design
services for a gas interconnector — at €1–10 bn beside the same digits ×10⁻³; several state the
real figure in words (100246: "προϋπολογισμού 7.849.600,00€"). The genuine one is **8287294**, the
UK Government Procurement Service's 12-lot national IT hardware framework at GBP 4 bn (in all three
versions), whose GBP 4 m `result_value` is the slip: the rule now drops it to GBP 4 m — the
recorded false positive (1/65). Neither corroboration nor cross-version repetition separates it:
100246's €7.85 bn slip is also repeated in all 4 versions.

**Decision (owner, 2026-10-06):** lower `SCALE_ERROR_MIN_EUR_CENTS` to €1 bn — 64 fixes against
1 regression, on a fully adjudicated population. Stop there: below €1 bn is unmeasured, and the
decade under it is where the "small figure in thousands" reading becomes ordinary.
`BAND_FLOOR_EUR_CENTS` (section 16's listing) stays €10 bn, decoupled: the €1–10 bn decade is past
its 1,000-row cap. The drain is the 65, not the decade (docs/operations.md).

## €1 bn drain — result (2026-10-06)

Deployed `5401035`; `refold-notices` over the 65 head notices (job 2934: re-queued 65, stamped
65), `project` 2935 (71 Tenders written, 94 verified unchanged). Re-read of the 65:

- **56 now below €1 bn**, among them 8287294 at €5.1 m (GBP 4 m — the recorded false positive,
  as predicted).
- **6 with NO head value** (748911, 873826, 4393056, 5070016, 5763279, 8021864): the refused
  figure was the head version's only electable amount and its partner sits where the election does
  not look for a head — a lot award (748911, 4393056, 5070016, 8021864) or a superseded version
  (873826: €2,860,863.10 in versions 1–2, ×1000 in the 3rd; 5763279: CZK 71.74 m in version 2).
  Refused is not corrected (unit 4(a) decision), so "no value" beats a ×1000 one; whether the
  head should FALL BACK to the partner is an open question for 4(b)'s supersession work, which
  deals in exactly "an earlier/other declaration of the same figure".
- **3 still ≥ €1 bn:** 514188 (94-lot vehicle framework, every lot estimate ×1000 — the expected
  unit-5 case); 355258 and 591121, whose WHOLE award notice is scaled ×1000 (estimate AND result:
  355258 est €1.45 bn / result €1.159 bn against version 1's €1.45 m; 591121 est €1.454 bn /
  result €1.009 bn against €1.454 m). The estimate falls to its partner, but the result has no
  partner of its own (the real €1.159 m / €1.009 m was never published), so it is elected.
  Unit 5 material: a notice whose every figure is ×10^k of the previous version's.

## 4(b) — split out (2026-10-06)

Decided: 4(b) becomes issue 489 (`489-an-f14-value-correction-never-supersedes-the-figure-it-corrects.md`).
It is a different mechanism (free-text NEW_VALUE parsing, same-field supersession in the version state),
and inside this issue's band it moves ONE row: of the five section-16 rows with a value corrigendum,
5592948 and 6852637 are already refused by 4(a), 7257797 is a genuine €12.4 bn whose corrections are
small restatements, 4871119's second F14 corrects £250 m BACK to £25 bn, and only 6891632 (€25.3 bn →
€85.5 m) is fixed by it. The no-head fallback question (above) goes with it.


## Status history

Earlier status lines, newest first (moved out of the Status line 2026-10-07):

- ready-for-agent — €1 bn GATE DEPLOYED + DRAINED 2026-10-06 (`5401035`; jobs 2934/2935: 65 re-queued, 65 stamped; 56 now < €1 bn, 6 with no head value, 3 still ≥ €1 bn: 514188, 355258, 591121 — see "€1 bn drain — result"). NEXT: 4(c) (dropped-decimal signal), 5 (now incl. the whole-notice ×1000 CANs 355258/591121), 6 DONE 2026-10-08 (data-quality 2954: band 332 → 305, every departure attributed, only 8400892 still carries a partner — see "Unit 6 — the re-read"). 4(b) and the no-head fallback question MOVED to issue 489 (see "4(b) — split out"); 489 units 2+3 deployed and draining 2026-10-07.
- BELOW-BAND MEASURED + GATE LOWERED TO €1 bn 2026-10-06 (see "Below-band measurement (2026-10-06)"; €1–10 bn: 3,579 Tenders, 65 refused heads, adjudicated 64 error / 1 genuine (8287294); `SCALE_ERROR_MIN_EUR_CENTS` = €1 bn, `BAND_FLOOR_EUR_CENTS` decoupled at €10 bn). NEXT: gate → commit → deploy → drain the 65 per docs/operations.md ("The €1 bn extension's drain") → re-run below-band-query.sh; then 4(b), 4(c), 5, 6.
- UNIT 4(a) DEPLOYED + DRAINED 2026-10-06 (`900b13b`; band 324 → 305, the predicted 19 out, 8400892 kept). NEXT: below-band measurement, 4(b), 4(c), 5, 6.
- UNIT 4(a) LANDED + REVIEW FIXES APPLIED 2026-10-06 (uncommitted, not deployed, not drained; see "Unit 4(a) — review fixes (2026-10-06)": the rule is now GATED to the band (a refused figure must convert to ≥ €10 bn, `SCALE_ERROR_MIN_EUR_CENTS` = the data-quality `BAND_FLOOR_EUR_CENTS`), so it can move only band Tenders and only down; the per-lot pick in `summarise` calls the same `ScalePartners::refuses_amount`; corroboration is an O(1) map; two fold-path pins in head_election_agreement.rs). NEXT: `ops/check.sh` → commit → deploy → drain the band with `refold-notices` + `project` per docs/operations.md ("The exact-10ᵏ election rule") → expect 19 rows out, 8400892 in; then the BELOW-BAND MEASUREMENT (before ever lowering the gate: section 16's partner subqueries without the band predicate, plus "no other head field carries it", windowed by `t.id`, bucketed by value decade and k, a sample adjudicated); then the 554082-2023 member read for 6721266; then 4(b), 5, 6.
- ready-for-agent — UNIT 4(a) LANDED IN THE TREE 2026-10-06 (uncommitted, not deployed, not drained; see "Unit 4(a) — landed (2026-10-06)": the decided rule is computed in the head election (`ScalePartners`, canonical.rs), pinned by `a_figure_exactly_ten_to_the_k_above_a_sibling_is_not_elected`). NEXT: `ops/check.sh` → commit → deploy → drain the band with `refold-notices` + `project` per docs/operations.md ("The exact-10ᵏ election rule") → expect 19 rows out, 8400892 in; then the 554082-2023 member read for 6721266; then 4(b), 5, 6.
- ready-for-agent — UNIT 4 ADJUDICATED 2026-10-06 (see "Unit 4 — adjudication (2026-10-06)": 22 rows with an exact 10^k partner, 20 agreed SCALE_ERROR (2 already drained by unit 3), 1 agreed GENUINE (8400892), 1 split (6803400); rule decided: refuse a figure exactly 10^k (k ≥ 3) above a same-currency partner of the Tender (any version, amounts or lot awards, > 10.00) UNLESS the same figure is also carried by a different canonical amount field of the head version — 0 false positives on the sample, 19/19). NEXT: implement the decided 4(a) rule with `a_figure_exactly_ten_to_the_k_above_a_sibling_is_not_elected`; then the 554082-2023 member read for 6721266; then 4(b), 5, 6.
- ready-for-agent — UNIT 3 DEPLOYED + RE-PARSED 2026-10-06 (`25b0d10`; reparse 2021 over fetches 178–182, 172,281 notices; fold 2022; 4490098 result_value €49,700). NEXT: unit 4.
- UNIT 3 LANDED + OWNER DECISION APPLIED 2026-10-06 (uncommitted, not deployed, not re-parsed; see "Unit 3 — decision (2026-10-06)"): an amount whose `@FMTVAL` was its text × an exact even 10^k (k ≥ 2) stores the TEXT as an ORDINARY amount — no `quality` marker, electable and served (4490098's fixture head = 4,970,000 cents, €49,700); the raw attribute stays in the parse layer as `.FMTVAL_MISMATCH`; `QUALITY_FMTVAL_MISMATCH` removed, `quality` is only 'withheld' again. NEXT: `ops/check.sh` → commit → deploy → reparse r208 2011-04…08 per docs/operations.md (one job, packages = 5) → `project` → re-read `/v1/tenders/4490098` and section 16 of the data-quality report; then count `.FMTVAL_TEXT` rows in those months; then unit 4.
- ready-for-agent — UNIT 3 CODE LANDED IN THE TREE 2026-10-06, REVIEW FIXES APPLIED THE SAME DAY (uncommitted, not deployed, not re-parsed; see "Unit 3 — landed" and "Unit 3 — review fixes"): `@FMTVAL` checked against its text in the shared TED_EXPORT walk; ONLY the measured shape (attribute = text × an exact EVEN 10^k, k ≥ 2, compared in i128) adopts the text and marks `fmtval_mismatch`; every other disagreement keeps the attribute UNMARKED with the text filed beside it as `.FMTVAL_TEXT`. NEXT: decide whether an adopted text stays unelectable (today it does: 4490098's head goes NULL on the fixture), `ops/check.sh`, commit, deploy, re-parse r208 2011-04…08 per docs/operations.md (one job, packages = 5), then count `.FMTVAL_TEXT` rows in the re-parsed months before ever marking that class, then unit 4.
- ready-for-agent — UNIT 3 CODE LANDED IN THE TREE 2026-10-06 (uncommitted, not deployed): exact-10^k mismatch adopts the text, every mismatch marked.
- ready-for-agent — UNIT 1 DEPLOYED + MEASURED 2026-10-06 (`b1fcb29`, dq 2019: 332 Tenders ≥ €10 bn in 9 currencies, 22 with an exact 10^k partner); UNIT 2 READ DONE (r208: @FMTVAL disagrees with text — 4490098; r209: no FMTVAL, publisher text errors incl. a dropped decimal point); UNIT 3 MEASURED (the r208 @FMTVAL defect is TED July 2011: ~5.9 % of value elements in 17 daily packages, exact even 10^k; text is right); NEXT: unit 3 code (adopt text on an exact-10^k mismatch, mark fmtval_mismatch) + reparse 2011-04…08, then unit 4.
- ready-for-agent — UNIT 1 LANDED IN THE TREE 2026-10-06, review fixes applied the same day (uncommitted, not deployed): section 16 of the data-quality report lists the band; see "Unit 1 — landed (2026-10-06)". NEXT: `ops/check.sh`, commit, deploy; then BEFORE issue 429's weekly `analyze` schedule goes live, a plan-probe of `band_listing_sql()` on an analyzed prod snapshot (the fixture-ANALYZE pin is not prod's stats, and `measure_rows` has no deadline); then the stored report's Done check (the section's summary line present, no `UNMEASURED — the \`band_listing\``), then unit 2's gated archive read.
- ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is 366's unit 6: a weekly-report section that lists every elected head value at or above €10 bn, grouped by published currency, with each row's signals beside it, read off the `tenders_current_value_eur` index.

## Unit 6 — the re-read (2026-10-08)

Data-quality job 2954 (enqueued as 2045, after the all-profile refold 2953 rewrote every Tender; 4,882 s,
0 labels unmeasured). Section 16 saved as `471-values/section16-dq2954-2026-10-08.txt`, compared row by
row with job 2019's (`section16-dq2019-2026-10-06.txt`): **332 → 305 Tenders; 29 left, 2 entered.**

Every departure is attributed:

| cause | Tenders |
|---|---|
| unit 4(a), the exact-10ᵏ rule (the predicted 19) | 6941544, 4972513, 8452561, 5592948, 6988280, 577127, 6581010, 4685893, 8822396, 5094790, 224156, 568960, 404296, 4785037, 6577862, 6721266, 6852637, 1163733, 6803400 |
| unit 3, the July 2011 `@FMTVAL` fix (r2.0.8, drops by an even 10ᵏ) | 4578779, 4581663 (named at unit 3), and 4449756 (€12 bn → €1.2 m), 4580833 (€30 bn → €30 k), 4583254 (€60 bn → €60 k), 4587738 (€40 bn → €4 m), 4589686 (€25.5 bn → €2.55 m), 4589688 (€16 bn → €1.6 m) — 2011-chain Tenders whose 2012 head notice inherited the corrected figure and that the full rewrite of 2953 re-elected |
| issue 489, the F14 value correction | 6891632 (€25.28 bn → €85.5 m) |
| re-keyed, not a value change | 8654927 → 8831875 (same notice 043150-2026, same £26 bn; the Tender id moved in the full refold) |

Entered: 8831875 (the re-key above) and 8831662 (FTS 094087-2026, £20 bn, no partner, no corrigendum —
unit 5's class, one figure with nothing inside the Tender to test it against).

**Partner column:** of the 305, exactly ONE row still shows an exact 10ᵏ partner — 8400892, the
£10.8 bn housing framework kept on purpose (corroborated by two fields). So within the band the rule
has removed everything it can see, and its one survivor is the adjudicated genuine row. What remains in
the band is unit 5's class (single figures with no in-tender signal) and 4(c) (dropped decimal).

## Unit 5 — decision (2026-10-08): no outside signal; serve as published with a caveat

**Sample.** 48 of the 305 section-16 rows (job 2954), stratified by source: FTS 16 of 104, TED 16 of
124, text 8 of 27, eForms 8 of 50. 8400892 is excluded. Each row was adjudicated from three sources:
`/v1/tenders/{id}`, the buyer's other Tenders (`/v1/tenders?buyer=<org>&limit=100`) and the notice text.
Workflow `wf_f00023ee-530`; every verdict and its evidence is in
`471-values/unit5-sample-verdicts-2026-10-08.json`.

| stratum | genuine | error | unclear | weighted errors in the band |
|---|---|---|---|---|
| FTS | 13 | 1 | 2 | ≈ 7 of 104 |
| TED | 10 | 6 | 0 | ≈ 47 of 124 |
| text (1993–2010) | 0 | 8 | 0 | ≈ 27 of 27 |
| eForms | 3 | 5 | 0 | ≈ 31 of 50 |
| **all** | 26 | 20 | 2 | **≈ 111 of 305 (≈ 36 %, wide: n = 48)** |

The genuine rows are national framework ceilings and programmes, internally consistent and usually
repeated across notices. Examples: CCS CWAS 3 at £80 bn (7958277), PSSV at £26.5 bn with nine lots that
sum exactly (7955574), National Grid HVDC at £24.6 bn beside sibling frameworks (8681166), and the GB
Nuclear SMR partner at £20 bn (8749111).

**The outside signal is rejected.** The tested rule compares the head with the buyer's largest OTHER
value. Scored on the 46 decided rows:

| rule | errors flagged (of 20) | genuine rows flagged (of 26) |
|---|---|---|
| ratio ≥ 10, any history | 12 | 6 |
| ratio ≥ 20, ≥ 3 other values | 11 | 3 |
| ratio ≥ 100, ≥ 3 other values | 7 | 2 |
| ratio ≥ 1000, ≥ 3 other values | 6 | 0 |

At zero false flags it catches under a third of the errors. Each way it fails is structural, so more
data would not fix it:

- **A young buyer's first programme reads as a slip.** 8749111 (GB Nuclear's SMR partner, £20 bn) has
  one other value, a €1.5 m pension scheme, so the ratio is 15,800.
- **Same-class sibling errors poison the buyer's max.** OPAM (3420171) has six more min/max run-together
  awards up to €80 bn, so its ratio is 0.6. Corse-du-Sud (3450659) is the same case.
- **One procurement split across Tenders compares a row with itself.** 8811221 and 8811222 are both the
  Pagabo framework; 8681166 and 514891 are both National Grid HVDC.
- **A median rule flags national frameworks.** CCS (7958277, ratio 255 to the median) and 8649015
  (ratio 29,509) look like slips because their buyers mostly publish small call-offs.
- **The history itself is not served.** `/v1/tenders?buyer=` pages by id, so an old buyer's first page
  is its 1990s notices: 8784848's default page held 1 value in 100. A usable signal needs a per-buyer
  value aggregate, which nothing keeps today.

Refusing a genuine national figure is worse than serving a publisher's typo under a caveat. Election
stays as is: **a single figure with no in-Tender partner is served as published.**

**The caveat** is now on `/docs` (`crates/app/src/v1/docs.rs`, the Amounts list). The stale bullet
claimed `value` is the raw published figure ("257 trillion PLN"). It is replaced by the election, the
exact-10ᵏ rule and the "roughly a third of the €10 bn-and-up residue were typos" warning. The value-filter
bullet loses its "the payload `value` can be a figure the filters ignore" sentence, which issue 366
unit 3 made untrue: `value` is the elected row.

**What the sample found instead.** These are in-notice mechanisms, outside unit 5's class:

1. **Text era, a min/max range read as one number** (5 of the 8 text rows). Published V.4 values such
   as `Value: 60 000 220 000 EUR` are a French bons-de-commande minimum and maximum (CMP 2001/2004:
   maximum ≤ 4 × minimum). `parse_money` accepts them as one correctly grouped figure. On all 27 text
   band rows, 19 split at a group boundary into X < Y ≤ 4X. → **issue 491**.
2. **Text era, a partner and a corrigendum that are never read.** 3427333: II.2.1 prints €35,076,200,000,
   and the same notice's V.4 prints €35,076,200, but V.4 is not stored when II.2.1 is present. 4179951:
   the head notice's free-text "Instead of: … 15 000 000 000 GBP. Read: … 150 000 000 GBP" is not
   applied. → **issue 491** (b) and (c).
3. **A ×100 slip (k = 2) below the k ≥ 3 rule.** 6640498 (section 16 shows ×100) and 8784848 (lot 1 at
   £60 bn; at £600 m the three lots sum exactly to the procedure's £1 bn). → **issue 492**, which must
   measure k = 2 precision first, because round hundreds are common.
4. **Dropped decimals (4(c))** do occur: 5265429 (×10⁴), 355006 (€35,000,094,166 ≈ €350,000,941.66),
   525252 (every figure in the notice ×100) and 6640498. None leaves a signal inside the notice. A lone
   figure has nothing to compare with, and a notice scaled as a whole agrees with itself. **4(c) is
   closed as caveat-only: there is no signal to build.**
   The same goes for the whole-notice ×1000 CANs that the €1 bn drain left at ≥ €1 bn (355258, 591121;
   status line of 2026-10-06): every figure in the notice is scaled alike, so they stay as published
   under the caveat.
5. **The rest are publisher-side.** A total-of-all-buyers ceiling filed under one buyer (8434466). Per-lot
   quantities summed over 37 identical lots (909191). An exact €10¹⁰ result over a €2 m estimate
   (535069); round powers are deliberately not sentinels. Typos whose real figure exists only in prose
   (7226190, 4785789, 578884). One garbled total (4722400).
