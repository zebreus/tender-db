# 366 — the head columns elect MAX over facts that carry no quality flag: a 2005 tender is served as open, and €49 quadrillion tops the value ordering

Status: ready-for-agent — **UNIT 1 DECIDED 2026-09-08 (owner), see "Unit 1 DECIDED": two flag legs (negative + all-9s sentinels, 15,899 rows; >€100bn implausible, 175 rows), with the €10–100bn band explicitly left to the lot-sum/FMTVAL signals because no threshold separates the NHS England contract from a €10bn vending-machine notice.** Was: ready-for-agent (filed 2026-09-07 from the external review's verified findings;
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

## Unit 1 DECIDED (owner, 2026-09-08) — and the measurement moved the rule

Unit 1 asked for "a per-currency ceiling and the sentinel/placeholder lists". Measuring the tail first
changed what the rule should be: **the dominant defect is not magnitude, it is a handful of exact
published values**, and a magnitude threshold alone cannot separate the rest.

### What the tail actually looks like

Counts of `tenders.current_value_eur_cents` above each threshold:

| above | tenders |
| --- | --- |
| €1e15 | 8 |
| €1e14 | 12 |
| €1e13 | 25 |
| €1 trillion | 89 |
| €100 bn | 175 |
| €10 bn | 394 |
| €1 bn | 2,902 |

Every one of the top 25 is absurd on its face — €4.97e16 for "Study on anti-corruption measures in EU
border control", €5.98e15 for "Renovation of 4 Linac flight path cabins", €65 trillion for cleaning
services. The 25th row is still €11.7 trillion.

### Exact values, and they are the bulk of it

| value | tenders |
| --- | --- |
| **−1.00** | **15,529** |
| any negative | 15,650 |
| 999,999,999 | 199 |
| 9,999,999,999 | 36 |
| 99,999,999,999 | 14 |
| 1,000,000,000 | 56 |
| 10,000,000,000 | 12 |
| 100,000,000,000 | 2 |
| 0 | 24,512 |

The all-9s family is a **form-width maximum**, and the counts falling with width (199 → 36 → 14) are its
signature. Reading the €10–100 bn band top-down confirms it from the other side: it is dominated by
`99,999,999,999` and `100,000,000,000` sitting on tiny municipal contracts — "Étanchéité terrasse"
(roof waterproofing), "Gazole non routier" (off-road diesel), "Épinal", "Assurance des risques
statutaires". €100 bn for a roof is not a magnitude error, it is the widest number the form accepted.

### The decision

**Leg A — `sentinel`, by exact value.** Flag, exclude from the head columns and from the value bounds,
keep the published row:
1. **any negative amount.** No procurement has a negative value; −1.00 alone is 15,529 rows and is a
   documented publisher convention for "not stated".
2. **an all-9s run of ≥9 digits in the major unit** (999999999, 9999999999, 99999999999, …) — 249 rows,
   evidenced above as a field-width maximum rather than a figure.

**Leg B — `implausible`, by magnitude.** Flag amounts **above €100 bn EUR-equivalent** (1e13 cents).
175 rows, and inspection of the top 25 plus the 99999999999 cluster shows the class is junk throughout.
This is deliberately far above any real single procurement: the whole EU procures on the order of
€2 trillion a year across every member state, so one notice at €100 bn is not a close call.

**Explicitly NOT flagged, and why:**
- **Round powers of ten** (1e9 → 56, 1e10 → 12, 1e11 → 2 rows). A €1 bn framework is a real thing; the
  all-9s neighbours are not. Flagging round numbers would delete genuine values to catch nothing the
  all-9s rule misses.
- **Zero** (24,512 rows). Ambiguous by construction — a planning or market-engagement notice
  legitimately publishes 0 (UK FTS release 083645 does exactly this, issue 342). It stays, and
  `?max_value=0` returning zeros is correct; what that filter must stop returning is the negatives.
- **The €10 bn–€100 bn band** (219 rows between the two thresholds). This is the honest limit of a
  magnitude rule: the band genuinely mixes real mega-frameworks — "NHS England 2019/21 Specialised
  Commissioning Contracts" €10.6 bn, the Île-de-France transport authority contract €10.8 bn — with
  obvious junk like "Homecare and Support on the Isle of Wight" €10.5 bn (population ~140,000) and
  "Vending Machine Services" €10.65 bn. **No threshold separates those**, and picking one would either
  delete the NHS contract or keep the vending machines. Separating them needs the tender-amount-versus-
  lot-sum ratio and unit 5's `@FMTVAL`-versus-element-text comparison — the two signals that can tell a
  10¹⁰ scale error from a big contract. That is the next unit, not a number guessed here.

**Rule 9/12 (placeholder null-out):** subsumed. A flagged fact is not nulled — it is stored as
published and skipped by the election, which is rule 8's disposition and keeps the parse layer
faithful (ADR-0004). Nothing needs a separate null-out path.

**Shape of the flag:** one nullable `quality TEXT` on `Fact::Amount`/`Fact::Date` and their satellites,
carrying `sentinel` or `implausible` — not a boolean, because the two have different causes and the
report should be able to say which. Deadlines get the same column for unit 3's ladder; the year-3005
value is `implausible` by a date ceiling (a deadline more than ~10 years out), decided with unit 3.

## Done when

- no tender above the per-currency ceiling is served as a head value, and `sort=value`'s first page is re-read and recorded here;
- 3323836 is closed and absent from `status=open`;
- `?max_value=0` returns no negative-sentinel rows;
- `/docs#caveats`' quarantine sentence is corrected (see `served-claims-nothing-re-derives`).

*One issue because:* the €4.97e16 head value, the 257-trillion-PLN row, the -1.00 sentinel in `value`, the year-3005 deadline and the 2005-tender-served-as-open are one derivation — `.max()` over facts that have no field in which to be marked implausible — plus its mirror in the read layer's `ORDER BY … DESC LIMIT 1`.
