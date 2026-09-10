# 370 — the served contract is hand-written prose with no gate coupling it to behaviour: twelve published claims are now false

Status: UNITS 1,2,3,5 DONE 2026-09-07 (owner) — all thirteen claims corrected at their source (`c185ed1`, 915 passed) and the provisional note coupled to the resolver by a test. Unit 4's second half (per-field provenance on `TenderRow`, so an inherited deadline is distinguishable rather than only documented) remains ready-for-agent. Was: ready-for-agent (filed 2026-09-07 from the external review's verified findings)
several reviewer "defects" are really this issue: the behaviour was decided deliberately
and the published description was not updated)
Kind: defect (documentation / API contract) — a drift with no detector
Relates to: 234 / 351 (the invariant that was deliberately retired), 219 / 238 / 267 /
171 / 329 / 48 (the issues whose decisions these pages still contradict), 115 (the SQL
oracle — the one place a doc claim IS pinned by a test), and the four code issues whose
fixes make three of these rows true again

## Observed — the claim, where it is served, and what falsifies it

| claim | served at | measured 2026-09-07 |
| --- | --- | --- |
| "1 = a single-mention profile with no official identifier, **never merged** (CONTEXT.md)" | `crates/app/src/v1/sql.rs:702-706`, live on `/v1/sql/schema` | org 1197927 "Deutsche Bahn AG" is `provisional=1` with **7,668** mentions (`SELECT COUNT(*) FROM organization_mentions WHERE organization_id=1197927`). `/v1/organizations?name_prefix=Landeshauptstadt%20M%C3%BCnchen` returns provisional rows with 11 and 142 mentions **and** org 4310313 with `provisional:false` and exactly 1 — inverted in both directions |
| a provisional profile "represents exactly one mention and never absorbs another" | `crates/store/src/canonical.rs:345-348` | same |
| "name-only mentions stay separate provisional profiles" | `CONTEXT.md:57-61` | same |
| "The org-identity uniqueness is a NAMED index" | `crates/store/src/canonical.rs:345-347` | `organizations_identity` is built **non-UNIQUE** (`crates/store/src/canonical.rs:6091-6094`), for the reason written at :6063-6075; VAT `DE811569869` stands on two canonical rows (22318959 "DB Engineering & Consulting GmbH", 22697494 "Deutsche Bahn AG - Tender Office …") |
| "Astronomical garbage magnitudes (10⁵⁰-class) are quarantined at ingestion and never enter the corpus" | `crates/app/src/v1/docs.rs:571-572` | true only of i64-overflowing eForms amounts; 174 tenders over €100bn are served, the top at €4.97×10¹⁶ |
| "any query past the time limit … is 408, and **its server-side work is abandoned** so it never holds a slot past the cap" | `crates/app/src/v1/docs.rs:382` | false for a non-yielding aggregate — `crates/app/src/v1/sql.rs:239-245` and issue 238's own prod measurement say the thread stays pinned; the per-token *slot* is released, the work is not |
| "Filter, absent value ?country=ZZ <1 ms*" + "* an absent filter value short-circuits to an empty page" | `crates/app/src/v1/docs.rs:511`, :517 | `?currency=XXX&limit=2` → 200 empty after **29.78 s** |
| "Timestamps are ISO 8601; a source that published a date only yields a date only, **never an invented time**" | `crates/app/src/v1/docs.rs:113` | doe date-only publications serve `2026-09-04T22:00:00Z`; tender 7954578 is published a day before it is dispatched |
| "treat deadline < published_at as published noise", at 0.2–0.3% | `crates/app/src/v1/docs.rs:579-581`, restating `docs/research/data-profile-2026-08.md:145-154` | the ROW-level rate is **2,981,402 / 7,929,584 = 37.60%** (49.15% of rows that have a deadline). The study measured a different quantity — a deadline against its CAUSING notice, per version, where carry-forward cannot occur by construction |
| "This is the front door to participation history: resolve the VAT to **a** canonical org id" | `crates/app/src/v1/docs.rs:238` | `/v1/organizations?identifier=DE811569869&kind=vat` returns two canonical orgs (22 and 2 mentions). Issue 329 establishes the duplicate can be legitimate (a German Organschaft), so the defect is the promised functional dependency, not the rows |
| `organizations?country=ESP`, `tenders?country=DEU` | `README.md:74`, `README.md:91` | both 200 with **0 rows**; issue 48's 2026-08-15 note claims "every curl example" was corrected in /docs and the OpenAPI — README was missed |
| "legacy R2.0.7–R2.0.9 (title 100% fill, research §5.1)" | `crates/ingest/src/project.rs:95` | 29,455 titleless r208 tenders |
| "Zero deadlines beyond publication+10 y in any window" (rule 14) | `docs/research/data-profile-2026-08.md:154` | 3005-07-06, 2999-12-31 ×2, 2924-04-15, 2205-11-18 stand in the corpus — the rule was measured vacuously green on a window |

## Why, exactly

Every one of these is a hand-written const or a prose line with nothing that re-derives it: `COLUMN_NOTES` is a table of string literals (`crates/app/src/v1/sql.rs:~690-707`), `/docs` is a formatted HTML const, CONTEXT.md and the schema comments are prose, and the research rules are numbers taken once, in a window.

The `provisional` row shows the shape exactly, and it is the most damaging because three surfaces repeat it. The invariant was **retired deliberately**: `provisional` is set purely by which INSERT arm the resolver takes — `provisional = 0` in the identifier arm (`crates/store/src/canonical.rs:8137-8142`), `= 1` in the identifier-less arms (:8225, :8337) — so it has meant `identifier IS NULL` and nothing else since issue 234, and prod agrees to the row: of 5,703,677 provisional rows, **0** carry an identifier. Issue 234 then made the identifier-less arm REUSE a standing row, and its own comment states the consequence: "The row STAYS `provisional = 1`, so a later identifier can still split or canonicalise it — the merge is a reuse policy, not a promotion" (`crates/store/src/canonical.rs:8186-8188`). Issue 351 widened the same reuse to country-less mentions (`crates/store/src/canonical.rs:8240`). So "never merged" became false by design on 2026-08-21 and more so on 2026-09-04 — and nothing couples the description to the behaviour: no test asserts a provisional row has ≤1 mention, and the change that landed 234 had no reason to touch the app crate.

The deadline row is the same failure in the other direction: a per-version measurement restated in ROW terms. The row is a UNION of versions by design — `supersede` leaves a field the notice is silent about alone (`crates/ingest/src/project.rs:3465-3471`) while `published_at` advances to the new notice's own (`crates/ingest/src/project.rs:3448-3452`) — so an award notice that publishes no submission_deadline inherits it and `deadline < published_at` is the *expected* shape for every tender that reached award. Verified end to end on tender 1: head seq 7 = notice 26990894, subtype 29 (award), publishing four date rows and no submission_deadline; the deadline comes from seq 6, notice 25826280 (subtype 16), `BT-131(d)-Lot = 2026-06-30T10:00:00+02:00`. Nothing is wrong with the data; a consumer applying a 0.2–0.3% expectation to a 37.6% comparison concludes the corpus is broken. `TenderRow` (`crates/store/src/read.rs:264-284`) carries the head version's seq and published_at but no per-field "which version published this" marker, so a client cannot tell an inherited deadline from a republished one.

## Units

1. Correct every row of the table at its source line. The three rows whose code half is being fixed elsewhere (the 10⁵⁰ quarantine, the absent-value short-circuit, the invented time) get corrected **when that issue lands** — do not describe the future as present.
2. Restate `provisional` for what it is on all four surfaces (`crates/app/src/v1/sql.rs:705`, `crates/store/src/canonical.rs:346-348`, `CONTEXT.md:57-61`, and the OpenAPI if it repeats it): "no official identifier — identity is name-scoped and may be reused across mentions".
3. Couple what can be coupled: one test that reads `COLUMN_NOTES`' provisional note and the resolver's behaviour together (the 115 SQL-oracle pattern is the precedent). For the numeric claims, put the query beside the claim — the tracker's own "record how to re-take it" rule (`docs/agents/issue-tracker.md`) — so the next reader re-derives instead of trusting a photograph.
4. The deadline claim: correct `crates/app/src/v1/docs.rs:579-581` to say the row is a union of versions and that `deadline < published_at` is expected after award, with the row-level rate measured; and decide whether to add per-field provenance to `TenderRow` (`crates/store/src/read.rs:264-284`) so the distinction is answerable rather than only documented.
5. `README.md:74` and `README.md:91` — the two alpha-3 examples issue 48's sweep missed.

## Done when

- every row of the table is corrected or carries a dated re-take instruction;
- the provisional description matches the resolver on all four surfaces;
- a test fails if the resolver's reuse policy changes without the note changing.

*One issue because:* twelve false statements across `/docs`, `/v1/sql/schema`, the OpenAPI, README, CONTEXT.md and two code comments have one cause — the published contract is prose that nothing re-derives, so a deliberate behaviour change (234/351, 219, 267, 328) leaves the description behind and the drift is only ever found by a reader.

## Done (2026-09-07, `c185ed1`)

All thirteen rows corrected at their source line, and — departing from unit 1 — the three whose
code half is queued elsewhere were corrected too, to TODAY's behaviour. A served claim that is
false is a defect whether or not a fix is queued; those lines get revised again when 366, 371 and
367 unit 3 land, which is cheap.

- **`provisional`, on all three surfaces** (`sql.rs` column note, the schema comment,
  CONTEXT.md): it means no official identifier, with a name-scoped identity that may hold many
  mentions. The retired promise was a year old.
- **Corrected to today's truth**: implausible magnitudes are served, not quarantined (only
  integer-overflowing amounts are refused; 174 tenders over €100bn, top €4.97×10¹⁶ — issue 366);
  a timed-out query's answer is abandoned but its work can keep its slot, there being no engine
  interrupt (issue 238); an absent filter value short-circuits only where the filter has a
  reachability test, and `currency` has none (issue 371); a date-only publication can still
  render with a time (issue 367 unit 3); an identifier can resolve to more than one canonical
  org, so callers must take every id (issue 329); the org-identity index is named and NOT unique.
- **Restated rather than corrected**, because the numbers were right and the sentence wrong: the
  0.2–0.3% deadline-before-publication rate is a WITHIN-NOTICE measurement, and at row level it
  is 37.6% and is the expected shape after an award notice, since a row unions its versions and
  carries a deadline the newest notice is silent about. The research file's "zero deadlines
  beyond publication+10y in any window" carries a dated re-take note: it was green because of
  the windows chosen, and five counter-examples stand in the corpus.
- **README**'s two alpha-3 examples (unit 5) now use alpha-2 and return rows.
- **The coupling** (unit 3): `the_provisional_note_describes_what_the_resolver_actually_does`
  reads the served note AND exercises the resolver in one test — revert the reuse and the
  behaviour half fails; restore the old wording and the note half fails.

Left: unit 4's second half — whether `TenderRow` should carry per-field provenance so a consumer
can tell an inherited deadline from a republished one, rather than only being told about it.

## The 10⁵⁰-quarantine row is revised again, as unit 2 said it would be (2026-09-10)

Unit 2's note said the three rows whose code half was queued elsewhere were corrected to *today's*
truth and would "get revised again when 366, 371 and 367 unit 3 land, which is cheap". **366's
standing rows landed**, so the quarantine row's replacement text is now itself out of date: it read
*"174 tenders exceed €100bn and the largest is €4.97×10¹⁶, so an ordering by value is topped by
publisher errors"*, and after 366's drain there are **zero** tenders above €100bn in the head column.

Revised, and the revision changed shape rather than just numbers, because the interesting fact is no
longer a magnitude:

- **The published figure is still served** — 257 trillion PLN on tender 43065, whose own lot results
  award 181.5 million. Ingestion still refuses only i64 overflow, and ADR-0004 keeps the parse layer
  faithful. That half of the old sentence was right and stays.
- **`min_value`/`max_value` no longer compare it.** They compare the derived EUR head column, which
  now skips negatives, repdigit field maxima and anything over €100bn. The caveat has to say so,
  because a caller filtering on value and a caller reading `value` are now looking at two different
  numbers — which is the *substance* of the change, and no magnitude figure conveys it.
- **Two consequences stated rather than left to be discovered:** a Tender whose only amount is refused
  has no known value and is returned by NEITHER bound (SQL three-valued logic on a NULL column), and
  its payload's `value` can therefore be a figure the value filters ignore.

**The general lesson for this issue, which is about served claims going stale:** a caveat written as a
*measurement* ("174 tenders exceed €100bn, the top is €4.97×10¹⁶") goes stale the moment the defect it
describes is fixed, and then reads as a live warning about a corpus that no longer exists. A caveat
written as a *rule* ("the bounds skip these three classes; the payload does not") stays true across
the fix. Unit 1's instruction was "do not describe the future as present"; this is its mirror — **do
not describe the present as a number when the durable claim is a rule.** Worth applying to the
remaining rows the next time one is touched.

