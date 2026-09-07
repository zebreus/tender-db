# 366 — the head columns elect MAX over facts that carry no quality flag: a 2005 tender is served as open, and €49 quadrillion tops the value ordering

Status: ready-for-agent (filed 2026-09-07 from the external review's verified findings;
unit 1 is the decision 132 has been holding)
Kind: defect (canonical head derivation + read filters) — data-profile rules 8/9/12 need
somewhere to live before they can be enforced
Relates to: 132 (rule 8 named as "the one remaining decision … a canonical-semantics
change, to be decided deliberately"), 171 §3 (rules 8/9/10/12/14 — seeded, never wired),
267 (the plausibility MEASURE, explicitly not a gate), 216 (the head columns), 343 (the
fold-vs-read tie-break; here both sides agree on a rule nobody justified), ADR-0014 D5

## Observed

**Magnitudes.** `SELECT id, current_value_eur_cents, current_title FROM tenders ORDER BY current_value_eur_cents DESC LIMIT 6` → 4490098 = 4,970,000,000,000,000,000 EUR cents ("Study on anti-corruption measures in EU border control") = **€4.97 × 10¹⁶**; 4576720 = 1,159,393,740,000,000,000; 4489350 = 598,403,000,000,000,000; 4490942 = 400,000,000,000,000,000. Counts: **174** tenders over €100bn, **387** over €10bn, 89 over 10¹² EUR cents. `/v1/tenders/43065` serves `value {cents: 25756286172000000, currency: PLN}` = 257.56 trillion PLN while the same row's `lot_results` award 181,515,452.29 PLN; `?min_value=1000000000000` returns it on page 1.

The parse layer shows the shape: notice 12376354 carries a PROCEDURE-scope `TED-VALUE_COST` of 4,970,000,000,000,000,000 cents beside a LotResult `TED-VALUE_COST` of 497,000,000 cents (€4.97M) and two `TED-VALUE` of 4,970,000 — exactly 10¹⁰ apart. 4576720's 1.15939374e18 is likewise exactly 10¹⁰ × €1,159,393.74.

**Sentinels pass the bounds.** `?max_value=100&limit=5` → id 26 `{cents:1}`, id 34 `{cents:-100}` ("Beschaffung von 3 Nutzfahrzeugen" — the -1.00 publisher sentinel, served as the row's value with no marker). `?max_value=0&limit=3` → 34, 49, 98.

**A closed 2005 tender is served as open.** `SELECT tender_id, seq, lot_id, field, utc_seconds FROM tender_version_dates WHERE tender_id=3323836` → exactly two rows, **both at seq 1**, both `lot_id` NULL, both `submission_deadline`: 1,118,793,600 (2005-06-15) and 32,677,554,600 (3005-07-06). One version, notice 2556682, published 2005-05-19 — one notice publishing two deadlines, not a cross-version conflict. `/v1/tenders/3323836` lists both in `dates` and serves `submission_deadline = "3005-07-06T10:30:00+00:00"`. And **`/v1/tenders?status=open&sort=deadline&order=desc` returns 3323836 FIRST**.

**The deadline extremes own both ends of the ordering.** `tenders_current_deadline` ASC: 5671586 = 0016-06-09, 1542367 = 0025-02-01, 5653434 = 0206-06-03, 1466977 = 0 (1970-01-01). DESC: 3323836 = 3005-07-06, 1154724/838350 = 2999-12-31, 279672/267598 = 2924, 3380840 = 2205-11-18. 35 in 2099.

## Why, exactly

**One sentence:** the head columns are an unconditional extremum over the head version's facts, and the canonical fact types have nowhere to record that a fact is implausible — so the worst value in a version is elected as the tender's headline and then owns the ordering, the bounds and the status.

- `head_value_eur_cents` is `.max()` over every tender- and lot-level `Fact::Amount` of the head version — `crates/store/src/canonical.rs:1239-1249`, the `.max()` at **:1248** — written to `tenders.current_value_eur_cents` (canonical.rs:8616). `head_deadline` is `.max()` over every `submission_deadline` `Fact::Date` — `crates/store/src/canonical.rs:1176-1187`, the `.max()` at **:1186** — written to `tenders.current_deadline` (canonical.rs:8614).
- **There is nowhere to put a flag.** `Fact::Amount { field, cents, currency, tax_basis }` and `Fact::Date { field, utc_seconds, offset_minutes, has_time }` — `crates/store/src/canonical.rs:1147-1148` — carry no quality field, and the satellites mirror them. That is exactly why data-profile rule 8 ("store as published, flag `sentinel`, exclude from canonical value columns") is still an undecided seed on 132 and 171, and rule 12's placeholder null-out likewise.
- **The only magnitude bound in the pipeline is i64 overflow.** `cents()` fails only when `checked_mul(100)` overflows (`crates/ingest/src/eforms/value.rs:114-119`), and that failure is what raises `unrepresentable-value` (`crates/ingest/src/eforms/parse.rs:258`) — so the "10⁵⁰ class" is quarantined by a *representation* limit, not a plausibility rule, and everything below ~9.2e16 EUR is accepted verbatim. The legacy era is weaker still: an unconvertible r209 amount is not quarantined at all, the raw text is kept (`crates/ingest/src/r209/parse.rs:437-443`).
- **The plausibility work built a measure, not a gate.** `amount_plausibility_template` COUNTs `a.cents > 100000000000000` per profile in the weekly report — `crates/ingest/src/data_quality.rs:689-698`, the constant at **:694** — and its own doc says it "is not a correctness claim about any single amount". Nothing consumes the count, and that line is the only magnitude constant in the non-test codebase.
- **The read side repeats the election instead of correcting it.** The list/detail deadline pick is the same rule in SQL — `ORDER BY s.utc_seconds DESC LIMIT 1`, `crates/store/src/read.rs:1435-1443` — and `status` is a range predicate on `current_deadline` justified by "MAX(d) > now ⟺ EXISTS(d > now)" (`crates/store/src/read.rs:768-778`), which is precisely why one implausible sibling makes a closed tender open. The value bounds compare the head column with no floor and no sentinel exclusion (`crates/store/src/read.rs:855-871`).
- **Two co-published deadlines are structurally normal.** Several source ids map to the one canonical `submission_deadline` — `TED-DATE_RECEIPT_TENDERS`, `TED-RECEIPT_LIMIT_DATE`, `TED-NEW_VALUE.DATE`, `TXT-DT`, `TXT-DD` at `crates/ingest/src/project.rs:215-241` — and `Fact::key` groups them all under `("date","submission_deadline")` (`crates/store/src/canonical.rs:1157-1165`), so `supersede` cannot drop either: they coexist in one version's BTreeSet and MAX arbitrates. This is issue 343's shape one field over, except that there the two sides disagreed; here they agree on "latest wins", a rule chosen for the status filter and never justified as a display rule.
- **One upstream contributor, uninspected**: `Rule::Amount` prefers `@FMTVAL` over the element text unconditionally and never compares the two — `crates/ingest/src/r209/parse.rs:426-428` — so a wrong eSender machine attribute is adopted and the human-readable figure discarded. The exact 10¹⁰ factor on two independent 2011 EU-institution notices fits that shape.

## Units

1. **Decide rules 8/9/12** (owner's call, first unit — this is 132's open decision): a quality flag on `Fact::Amount` / `Fact::Date` and the amount/date satellites, plus a per-currency ceiling and the sentinel/placeholder lists. Record the reasoning here.
2. **Head election skips flagged facts**: `head_value_eur_cents` (canonical.rs:1248) and `head_deadline` (canonical.rs:1186) fall back to the best unflagged fact, or NULL. Fixtures: 43065, 4490098, 3323836.
3. **State the deadline tie-break once** — a plausibility leg, then a stated rule rather than "latest" — and make canonical.rs:1186 and read.rs:1440 read the same ladder, the 343 way. 3323836 must serve 2005-06-15 and must not appear in `status=open`.
4. **Value bounds** exclude flagged amounts (`crates/store/src/read.rs:855-871`), so `max_value=100` stops returning the -1.00 sentinel and the zeros.
5. **Compare `@FMTVAL` against the element text** in `Rule::Amount` (`crates/ingest/src/r209/parse.rs:426`) and flag or prefer on disagreement. Needs one archive-member read first — gated per `docs/agents/prod-box-reads.md`.
6. **Make 267's measure a signal**: the weekly report lists the top N by magnitude per currency so the class is visible rather than counted.

## Done when

- no tender above the per-currency ceiling is served as a head value, and `sort=value`'s first page is re-read and recorded here;
- 3323836 is closed and absent from `status=open`;
- `?max_value=0` returns no negative-sentinel rows;
- `/docs#caveats`' quarantine sentence is corrected (see `served-claims-nothing-re-derives`).

*One issue because:* the €4.97e16 head value, the 257-trillion-PLN row, the -1.00 sentinel in `value`, the year-3005 deadline and the 2005-tender-served-as-open are one derivation — `.max()` over facts that have no field in which to be marked implausible — plus its mirror in the read layer's `ORDER BY … DESC LIMIT 1`.
