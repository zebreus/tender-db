# 474 — the submission-deadline window is written by hand at five sites in two languages, and only the writer count is pinned

Status: **DONE 2026-10-02** — deployed at `08c3dd9`. The submission-deadline window has one home in `canonical` (`3626de2`), pinned by a source-scan test with an exact-line allowlist (`08c3dd9`). The Verify reads `0`.
Was status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an
adversarial pass). The first unit is one commit: two helpers in `canonical` (a Rust predicate and its SQL fragment),
all five sites moved onto them, and a source-scan test that fails when a sixth copy appears.
Kind: risk (one rule, five hand copies; the fix is meant to change no answer)
Relates to: 366 (the horizon, `aa732c5`, and the read pick's drift, unit 3), 375 (the backfill's drift, and the
writer-count test), 171 (the floor, rule 12, `59651d9`), 422 (the lots row and status drift, `756d867`), 424 (the
lots EXISTS's own-lot row, `83187a5`)

## What is wrong

The head election admits a submission deadline only inside a window: not before `DEADLINE_FLOOR_SECS` (1990-01-01,
`crates/store/src/canonical.rs:1534`) and not more than `DEADLINE_HORIZON_SECS` (ten years) past the version's
publication (`canonical.rs:1514`). The two constants have one home. The rule that applies them does not. It is
written out by hand at five sites, three in SQL and two in Rust. Read at HEAD `9fca0d8`; the three source files are
identical at the deployed `9b44528`.

| # | site | language | floor | horizon | publication measured from |
|---|---|---|---|---|---|
| 1 | `canonical.rs:1391`, `head_deadline` (the fold) | Rust | `>=` | `<=` | `head.published_at` |
| 2 | `read.rs:1897-1901`, `tender_select_head`'s deadline pick | SQL | `>=` | `<=` | `v.published_at` |
| 3 | `read.rs:1075-1089`, `version_predicates`, the `/v1/lots` `status` EXISTS | SQL | own-lot row only (:1088) | both rows (:1081, :1089) | a correlated `tender_versions` lookup (:1071-1074) |
| 4 | `read.rs:3683-3687`, `summarise`, the lot row | Rust, negated (skip if outside) | `<` skips | `>` skips, only when `published_at` is `Some` | `tender_versions.published_at`, read as `Option<i64>` (:3545-3548) |
| 5 | `lib.rs:3657-3658`, `backfill_current_deadline` | SQL | `>=` | `<=` | `tenders.current_published_at`, a nullable column (`lib.rs:784`) |

The copies share the constants, not the shape. Site 3 leaves the floor off its outer row on purpose, because
`d.utc_seconds > now` already implies it (`read.rs:1065-1067`). Site 4 applies the horizon only when the version's
publication was found. The SQL sites drop a row whose publication is NULL. Site 5 measures from the denormalised
head column, and the others measure from the version row. Each of these differences is defensible, but each lives
only at its own site.

**History.** Read from the deployed checkout on 2026-10-01 (`git log -G'DEADLINE_(FLOOR|HORIZON)_SECS'` over
`crates/store/src` and `crates/ingest/src`):

| commit | date | what reached which copy |
|---|---|---|
| `aa732c5` | 2026-09-08 | 366: the horizon added to `head_deadline` only |
| `40410a0` | 2026-09-10 | 366 unit 3: the read pick catches up. Until then prod served tender 3323836's `submission_deadline` as 3005-07-06 while `status` and `sort=deadline` used 2005-06-15 (`crates/store/tests/head_election_agreement.rs:1-13`) |
| `b07e03c` | 2026-09-10 | 375: the backfill catches up. Running it would have re-stamped 3323836 back to 3005-07-06 (`lib.rs:3612-3616`) |
| `59651d9` | 2026-09-26 | 171 rule 12: the floor reaches the fold, the read pick, the backfill and the lot row in one commit |
| `756d867` | 2026-09-26 | 422: the horizon reaches the lot row and the lots `status` EXISTS, 18 days after `aa732c5` |
| `83187a5` | 2026-09-26 | 424: the lots EXISTS gains its own-lot row, with both edges written out again |

So the horizon change reached one of its copies, and the others caught up in three separate fixes over 18 days. One
of the gaps was served on prod. The floor reached every site that needed it in one commit, but only because its
author found and listed the sites by hand. Today `/v1/tenders/3323836` serves `"submission_deadline":"2005-06-15"` with both dates in `dates[]`
(public read, 2026-10-01).

**What is pinned.** Only the number of writers is checked structurally.
`the_head_columns_have_exactly_one_writer_that_decides_them` (`head_election_agreement.rs:271`) counts code lines
assigning `current_deadline =` or `current_value_eur_cents =` in four store files and expects 2. It does not check
which window a writer applies, and sites 2-4 are readers, not writers. Each reader is checked against today's two
edges only:
- `the_read_layer_reuses_the_election_rather_than_repeating_it` (:229) checks the read pick's SQL text for
  `s.utc_seconds >= {FLOOR}` and `<= {HORIZON}`;
- behaviour tests pin one edge each, on fixture dates picked for that edge: `head_election_agreement.rs:104`
  (horizon) and `:183` (floor) hold the read pick to the fold, `deadline_backfill.rs:136` and `:189` check the
  backfill against fixed values, and `lot_deadline_scope.rs:249` and `:280` check the lot row and the lots `status`
  filter.

No test compares the five windows with each other; the fold and the read pick are compared only on those fixture
dates. If a third edge goes into `head_deadline` with its own unit test, every
existing test still passes, unless some fixture happens to hold a date the new edge refuses. The other four sites
keep two edges. That is the `aa732c5` shape exactly.

The comments that chose to transcribe the rule rest on a premise that has expired. `lib.rs:3620-3622` says: "transcribed rather than looked up because it
CAN be: one comparison against one constant, interpolated from `canonical` so the number cannot drift". `read.rs:1878-1881`
says the same about two comparisons. Interpolation keeps the numbers from drifting, not the shape. "One comparison"
has been false since `59651d9`.

Out of scope: the weekly sentinel detector (`crates/ingest/src/data_quality.rs:777`, `:783`). It reads the floor
constant, retypes the horizon's value as `10 * 365 * 86_400`, and measures from the run rather than from the
publication (its doc, :772-776). It ranks candidates and elects nothing.

## Proposed fix

Give the window's shape one home, in `canonical` beside the constants:

- `pub fn deadline_admitted(utc: i64, published_at: i64) -> bool`, which is
  `utc >= DEADLINE_FLOOR_SECS && utc - published_at <= DEADLINE_HORIZON_SECS`. `head_deadline` filters with it, and
  `summarise` skips on `!deadline_admitted(..)`.
- `pub fn deadline_admitted_sql(utc: &str, published_at: &str) -> String` builds the same predicate as a
  parenthesised SQL fragment over two column expressions, with the constants interpolated. The read pick calls it
  with `("s.utc_seconds", "v.published_at")`. Both rows of the lots EXISTS call it with the correlated lookup. The
  backfill calls it with `("d.utc_seconds", "tenders.current_published_at")`.

Three shape decisions. None of them is meant to change an answer:
1. The lots EXISTS's outer row takes the full window. Beside `d.utc_seconds > now` the floor is a no-op, so the
   site-specific shape goes away and the result does not change. This EXISTS is the per-row filter that 423/424
   tuned, so re-read the `/v1/lots?status=open` timings that 422 recorded after the deploy.
2. In `summarise`, a missing publication admits nothing, as SQL's NULL already does. `tender_versions.published_at`
   is `NOT NULL` and dates are foreign-keyed to their version (`canonical.rs:189`, `:341`), so the `None` arm is a
   version row that should not be missing.
3. The backfill keeps measuring from `tenders.current_published_at`. It is now a visible argument at the call site,
   not a sixth hand-written predicate.

Rewrite the comments at `lib.rs:3620-3624` and `read.rs:1878-1881`, plus the writer-count message at
`head_election_agreement.rs:286-289` (it says the backfill "transcribes the horizon faithfully"). They should say
that the window is called from `canonical`. `data_quality.rs:777` should read `store::canonical::DEADLINE_HORIZON_SECS`
rather than retype the value (it already reads the floor that way at :783). Its own run-relative window stays.

Tests, in `crates/store/tests/head_election_agreement.rs`:
- **`the_deadline_window_is_written_once`** is the source scan. It is built like the writer count, with comments
  stripped at `//` (:277), and runs over `crates/*/src/**/*.rs`. It checks two things:
  (a) Outside `canonical.rs`, no code line names `DEADLINE_FLOOR_SECS` or `DEADLINE_HORIZON_SECS` or retypes
  `631_152_000` or `10 * 365 * 86_400`. The one exception is `data_quality.rs`'s detector, named in the test. Inside
  `canonical.rs`, only the two definitions and the two helpers name them.
  (b) The code lines in `crates/store/src` and `crates/app/src` that select `submission_deadline` facts
  (`= 'submission_deadline'`, `Some("submission_deadline")`, `== "submission_deadline"`) are counted. Today there
  are 6: `canonical.rs:1372`, `read.rs:1080`, `:1086`, `:1895`, `:3673`, `lib.rs:3656`. A new reader then fails the
  test until it is moved onto a helper and the count is raised in the same commit. (b) is what catches the 366
  shape, because the read pick that drifted there was a raw `MAX` that named no constant.
  On failure, the test names each offending file:line.
- **`the_sql_window_and_the_rust_window_agree_at_every_edge`** evaluates `deadline_admitted_sql` with a `SELECT` on
  a store connection. It covers the floor −1/0/+1 and publication + horizon −1/0/+1, and asserts each result
  against `deadline_admitted`.
- `the_read_layer_reuses_the_election_rather_than_repeating_it` asserts that the statement contains
  `deadline_admitted_sql("s.utc_seconds", "v.published_at")`, instead of checking the two edges by text.
- These tests stay unchanged and green: the fixtures in `head_election_agreement.rs`, `deadline_backfill.rs`,
  `lot_deadline_scope.rs`, `lots_open_head_seed.rs`, and canonical's
  `the_head_deadline_ignores_a_deadline_a_thousand_years_out`.

No schema change, no refold, no job. Gate with `ops/check.sh` and deploy as usual.

## Verify

    grep -hE 'DEADLINE_(FLOOR|HORIZON)_SECS' crates/store/src/read.rs crates/store/src/lib.rs | grep -vc '^\s*//'

- **open** (2026-10-01, HEAD `9fca0d8`): `8`. These are the hand copies at `read.rs:1075`, `1076`, `1899`, `1900`,
  `3684`, `3686` and `lib.rs:3660`, `3661`.
- **done**: `0`. The read sites and the backfill reach the window only through `canonical`'s helpers.
  `the_deadline_window_is_written_once` is what keeps it at 0.

## RESOLVED-VERIFIED 2026-10-02 00:5x UTC

Gated in batch 3 and deployed at `08c3dd9`. The Verify reads `0` hand copies.
