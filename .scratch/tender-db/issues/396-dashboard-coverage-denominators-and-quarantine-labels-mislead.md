# 396 — the dashboard's own explanatory copy contradicts the numbers beside it: 2026 reads 117.38 % under a footnote about shortfalls, and the only benign quarantine reason is glossed as unmapped content

Status: needs-triage — filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: docs (dashboard presentation, `crates/app/src/ui.rs` + the vendored denominator `crates/app/data/ted-notice-counts.csv`) — no data or API surface is wrong in either unit; both are the page saying one thing while its own adjacent column, footnote or ledger says another
Relates to: 229 (RESOLVED-VERIFIED 2026-08-18, rev `cc0ef20` — it introduced `year_held`/`year_ratio` and the † that unit 1's cell carries; it fixed the NUMERATOR side of a shared year and never touched the denominator's age), 189 (RESOLVED-VERIFIED 2026-08-12 — the other issue on this board where a coverage ratio reads above 1.0; there the cause is TED reusing document numbers in 1993–1999 and the CSV header calls "ratios marginally above 1.0 expected and healthy" for those years only, a mechanism that cannot produce 2026's 17 %), 06 (resolved — the panel itself, and the decision that ground truth is vendored at `crates/app/data/ted-notice-counts.csv` via `include_str!`, transcribed from `docs/research/ted-access-channels.md` §6), 33 (RESOLVED-VERIFIED 2026-08-17 — the sibling panel, and the precedent that a number which stops moving must SAY why), 15 (RESOLVED 2026-08-16 — the backfill that made the coverage grid the standing read, and where the `unreadable zip bundle …` reason first appeared in job 1's final counts), 30 (RESOLVED-VERIFIED 2026-08-17 — the quarantine class split that put unit 2's Class column on the page), 201 (RESOLVED 2026-08-14/15 — named the 8-row "unreadable zip bundle: … Could not find EOCD" bucket and ledgered it), 202 (RESOLVED 2026-08-14 — wrote the ledger entry unit 2's gloss contradicts), 137 (measured 2026-08-05 — "unreadable zip (EOCD) | 8 | 0 | 8" in the reclaim census), 303 (CLOSED 2026-08-27 — `quarantine_terminal_policy`, EOCD-corrupt zips `Fixed(8)`; it pins the count, not the copy), ADR-0004 (quarantine is content-no-profile-maps held whole — the sentence unit 2's fallback is quoting at a row it does not describe)
Blocked by: nothing

Two low-severity dashboard defects, found by the same lens, filed together because they are one
mechanism: **explanatory copy written for one case is served against a row that is the other case**,
with nothing on the page marking the mismatch. Unit 1 stars a 17 % SURPLUS with a footnote that
explains only a shortfall, because the denominator is a frozen mid-year snapshot the page never dates.
Unit 2 glosses the only BENIGN quarantine reason with the unmapped-content fallback, because the gloss
function is an exact-string match where its neighbour in the same row is a prefix match. In both units
the data are right, the arithmetic is right, and a reader of the page draws the wrong conclusion — a
duplicate scare in unit 1, a phantom mapping gap in unit 2. Both fixes are small and local to
`crates/app`; neither needs a refold, a reparse or a box read.

## Unit 1 — 2026 coverage reads 117.38 % against a 2026-07-17 snapshot denominator

### Observed (verified 2026-09-14 on prod)

    curl https://tenders.zebreus.click/

Coverage grid, rev `9e082fd` (confirmed via `/health`): all six 2026 `ted` rows read

    2026 | … | 497 791 | 117.38 % *†

with the cell title "2026 is served by more than one profile; this is the whole year's coverage
(584 293 held)".

| 2026 `ted` profile | held |
| --- | --- |
| eforms-sdk-1.7 | 11 |
| eforms-sdk-1.10 | 1 |
| eforms-sdk-1.11 | 10 |
| eforms-sdk-1.12 | 102 366 |
| eforms-sdk-1.13 | 379 429 |
| eforms-sdk-1.14 | 102 476 |
| **year_held** | **584 293** |
| published (the denominator served) | 497 791 |
| year_ratio | 584 293 / 497 791 = 1.17377 → **117.38 %** |

`GET /api/dashboard` carries the same numbers as JSON: every ted/2026 row is `published 497791`,
`year_held 584293`, `year_ratio 1.17377`, `partial true`, `ratio null`.

The only starred footnote on the page (`crates/app/src/ui.rs:589`) is

    * the year is not over; a shortfall there is the calendar, not a gap.

and the section header says "100 % means we hold the whole year". A grep of the served page for
`07-17`, `through 2026` and `snapshot` finds nothing: the denominator is never dated on the page.

Where the 497 791 comes from — the system's own mid-year snapshot, not a TED publication:

| source | content |
| --- | --- |
| `crates/app/data/ted-notice-counts.csv:63` | `2026,497791,1` |
| same file, header | "2026 is a partial year (through 2026-07-17) — `partial` marks it so the dashboard does not report a shortfall that is really just the calendar" |
| same file, header | "an API-vs-filename comparison for 2026 differed by 0.13 %" — so ~100 % is the expected reading |
| `docs/research/ted-access-channels.md:375` | `| 2026→07-17 | 497 791 |` |
| that CSV's last commit | `b4a18a2` (2026-08-31), a checksum-evidence commit for issues 311+314 — not a denominator refresh |

The corpus has moved ~8 weeks past the snapshot:

    SELECT kind, count(*), min(period), max(period),
           strftime('%Y-%m-%d',min(fetched_at),'unixepoch'),
           strftime('%Y-%m-%d',max(fetched_at),'unixepoch')
      FROM v_fetches WHERE source='ted' AND substr(period,1,4)='2026' GROUP BY kind

→ daily: 41 packages, `2026-00136`..`2026-00176`, fetched 2026-07-19..**2026-09-11**; monthly:
`2026-01`..`2026-06`, fetched 2026-07-19.

The surplus is the stale denominator, not duplicates. 497 791 notices through OJ S issue 136
(issue 15 pins 136 = publication 07-17) ≈ 3 660/issue → ≈ **644 200** expected through issue 176
(2026-09-11); the 584 293 held is ≈ **90.7 %** of that. The adjacent year 2025 reads **91.75 %†** on
every ted row, so the >100 % reading is confined to the partial year.

Mechanism, in code: `crates/app/src/coverage.rs:424` computes
`year_ratio: published.map(|p| year_held as f64 / p.notices as f64)` with no partial-year cap, and
`coverage_pct` (`ui.rs:660`) prints any ratio, appending `" *"` whenever `partial`. Meanwhile the
system's own verifier models a partial year's snapshot as a FLOOR: `classify()`
(`crates/ingest/src/bin/verify.rs:147`) returns `Over` only `if !partial`, and its test is named
`classify_partial_year_allows_surplus_but_flags_shortfall`. So held > snapshot is BY DESIGN on the
verify side; what is not designed is the dashboard printing that surplus as a coverage percentage
under a one-directional caveat.

Adjacent, recorded but NOT part of this unit: the era summary line reads
`ted · eforms:eforms-sdk-1.14 | 102 476 / 497 791 · 20.59 % *` — one profile's era total over the
whole-year denominator, which is issue 229's shape recurring at the era-summary level.

### Why it matters

The coverage grid is the page that answers "do we hold the whole year". For 2026 it can no longer
answer either half of that question: a reader cannot tell a stale denominator from 86 502 duplicate
notices, and a real gap in the current year is invisible because the ratio is already over 100 %.
The direction is benign (it reads over-complete, so nobody re-fetches data already held), but the
drift grows every day the pipeline runs, and every year-end repeats it.

### Why this is ours, not the publisher's

TED never published "497 791 for 2026". This system froze a mid-year count on 2026-07-17, kept
ingesting dailies through 2026-09-11, served the frozen number undated as "published", and wrote a
caveat that covers only the shortfall direction. Its own CSV header says an API-vs-filename comparison
differed by 0.13 %, so ~100 % is the expected reading; issue 189's document-number-reuse explanation
for ratios "marginally above 1.0" applies to 1993–1999 and cannot produce 17 %.

### Repro

1. `curl https://tenders.zebreus.click/` → Coverage grid: six 2026 `ted` rows, each
   `2026 | … | 497 791 | 117.38 % *†`; the six held counts sum to 584 293; 584 293 / 497 791 = 1.17377.
2. Same page: the only `*` footnote is "the year is not over; a shortfall there is the calendar, not a
   gap"; grep the HTML for `07-17`, `through 2026`, `snapshot` → no hit.
3. `grep -n '^2026' crates/app/data/ted-notice-counts.csv` → `2026,497791,1`; `head -30` of the same
   file → "2026 is a partial year (through 2026-07-17)".
4. `SELECT max(period), strftime('%Y-%m-%d', max(fetched_at), 'unixepoch'), count(*) FROM v_fetches
   WHERE source='ted' AND kind='daily' AND period LIKE '2026-%'` → `2026-00176`, `2026-09-11`, `41`.

### Done when

- The 2026 cell no longer reads a bare percentage over an undated snapshot. Whichever of these is
  chosen is written down here with the reason: (a) the denominator is refreshed (or fetched live from
  the Search API `totalNoticeCount`), (b) the page prints "published through 2026-07-17" beside a
  partial year, or (c) the ratio is suppressed once `year_held > published`.
- The `*` footnote covers the surplus case, so a starred row above 100 % explains itself — today the
  note is written for shortfalls only and `coverage_pct` stars both directions.
- The dashboard and `verify.rs` agree in writing: `classify()` treats a partial year's count as a
  floor, so the page must not present crossing that floor as coverage. A test pins the rendered text
  for `year_held > published, partial = true`.
- The fix is durable across year-ends, not a one-off re-vendor: either the denominator carries its
  as-of date through to the page, or it is fetched. Recorded either way, because every January
  repeats this.
- Re-read after the fix: the 2026 ted rows read ≤ ~100 % or carry a dated denominator; 2025 still
  reads 91.75 %† and 2008's `year_ratio` 1.0014 (issue 229's verification) is unchanged.
- The adjacent era-summary line (`102 476 / 497 791 · 20.59 % *`) is either fixed with this or filed
  as its own issue against 229's shape; it is not left undecided.

## Unit 2 — the only benign quarantine reason is glossed with the unmapped-content fallback

### Observed (verified 2026-09-14 on prod)

    curl https://tenders.zebreus.click/      # GET /, rev 9e082fd1, confirmed via /health

Quarantine reason table, row 1 of 2, verbatim:

| Reason | Class | What it means | Count |
| --- | --- | --- | --- |
| `unreadable zip bundle: invalid Zip archive: Could not find EOCD` | **benign** | **Content this notice's profile has no mapping for — held whole (ADR-0004).** | 8 |

The count and the exact reason string are independently confirmed on `/metrics`:

    tender_db_quarantine_reason_members{reason="unreadable zip bundle: invalid Zip archive: Could not find EOCD"} 8

(one label, one fixed string — so the scope is exactly this row and its 8 members; `/metrics` and the
JSON surfaces carry no gloss and are unaffected.)

The same page contradicts itself two panels down, in the Resolved-categories ledger:

    Corrupt zip bundles in the TED archive (EOCD missing) | … the bundle cannot be opened at all and is
    held whole … | issue 202 | 0 · 8 still held | 2026-08-14

Mechanism, in code — the two functions rendered side by side in that row (`ui.rs:750`) disagree about
how to match the reason:

| function | file | match style | verdict for this reason |
| --- | --- | --- | --- |
| `quarantine_class` | `crates/model/src/dashboard.rs:250` | `if reason.starts_with("unreadable zip")` — comment: "A corrupt archive entry is never a notice, whatever its trailing detail" | `Benign` |
| `quarantine_terminal_policy` | `crates/model/src/dashboard.rs` | `starts_with("unreadable zip")` | `Fixed(8)` (issue 303) |
| `quarantine_reason_explained` | `crates/app/src/ui.rs:1257-1277` | exact match over 17 reason codes, no prefix arm | falls to `_ => "Content this notice's profile has no mapping for — held whole (ADR-0004)."` |

Ingest emits the reason with a variable trailing detail — `format!("unreadable zip bundle: {e}")`
(`crates/ingest/src/package.rs:278`) — so **no exact-match arm can ever hit it**; only a prefix arm
can. Two sibling reasons have the same shape and would fall through identically while being classed
`Benign`: `unreadable zip entry #{i}: {e}` (`package.rs:291`) and `unreadable zip entry: {e}`
(`package.rs:306`). Neither has live rows today. The same function also feeds the Most-recent hover
title (`ui.rs:772`) when `detail` is NULL, so the wrong gloss can appear there too.

Nothing pins the current text as intended: no test references `quarantine_reason_explained`, and
neither `docs/` nor `CONTEXT.md` specifies the column. The expected value rests on the page's own
internal consistency — the Class column, the code comment, and the ledger entry written by issue 202.

### Why it matters

The quarantine count is the headline data-quality metric (CONTEXT.md), and "What it means" is the
column a reader uses to decide whether a bucket is a gap to chase. This row's real meaning is
"8 truncated zip bundles in the TED archive whose content is held from other language editions" —
terminal, decided, nothing to chase (issues 201/202/303). The text served instead says content exists
that no profile maps, which is the actionable shape: a reader takes a settled, benign, ledgered row as
an open mapping gap, while the Class column one cell to the left says `benign`.

### Why this is ours, not the publisher's

The publisher's part is 8 truncated zips, and that part is already correctly diagnosed and ledgered
on this board. What this system then wrote is a gloss that contradicts, in the same table row, its own
class column, its own code comment ("a corrupt archive entry is never a notice"), and its own resolved
ledger entry. The mismatch exists only because one function matches on a prefix and its neighbour
matches on an exact string that ingest can never emit.

### Repro

1. `curl https://tenders.zebreus.click/` → Quarantine reason table, row 1: reason
   `unreadable zip bundle: invalid Zip archive: Could not find EOCD`, class `benign`, What it means
   "Content this notice's profile has no mapping for — held whole (ADR-0004).", count 8.
2. `curl -s https://tenders.zebreus.click/metrics | grep quarantine_reason_members` → the same string,
   `8`.
3. Same page, Resolved categories: "Corrupt zip bundles in the TED archive (EOCD missing) … the bundle
   cannot be opened at all …  | 0 · 8 still held".
4. `sed -n '1257,1278p' crates/app/src/ui.rs` (exact match, `_` fallback) against
   `sed -n '250,260p' crates/model/src/dashboard.rs` (`starts_with("unreadable zip")`), and
   `grep -n 'unreadable zip' crates/ingest/src/package.rs` → `:278`, `:291`, `:306`, all `format!`-ed
   with a trailing detail.

### Done when

- `quarantine_reason_explained` gains a `starts_with("unreadable zip")` arm mirroring
  `quarantine_class`, so all three emitted spellings (`… bundle: {e}`, `… entry #{i}: {e}`,
  `… entry: {e}`) are covered by one rule rather than by 17 exact strings.
- The live row reads a gloss that matches its class and the ledger — "a corrupt archive entry that
  cannot be opened at all; never a distinct notice, held whole" or equivalent — and the
  Most-recent hover title for a `detail`-NULL row of that reason says the same.
- A test pins the invariant rather than the string: no reason that `quarantine_class` decides by
  PREFIX may fall to `quarantine_reason_explained`'s `_` arm. Today no test names that function at all.
- `/metrics` still reports `tender_db_quarantine_reason_members{reason="unreadable zip bundle:
  invalid Zip archive: Could not find EOCD"} 8` — this is copy only; the count and 303's `Fixed(8)`
  terminal policy must not move.
