# 370 — the served contract is hand-written prose with no gate coupling it to behaviour: twelve published claims are now false

Status: REOPENED 2026-09-15 — units 1-2's "all thirteen claims corrected at their source" is incomplete: two of those claims (`provisional` = single-mention; a timed-out query's server-side work "was abandoned") are still served live on prod at rev `9e082fd` from surfaces the table never listed — the `/docs` const at `crates/app/src/v1/docs.rs:643` and the vendored spec at `crates/app/data/openapi.json:412`. See Comments, 2026-09-15.
Was: UNITS 1,2,3,5 DONE 2026-09-07 (owner) — all thirteen claims corrected at their source (`c185ed1`, 915 passed) and the provisional note coupled to the resolver by a test. Unit 4's second half (per-field provenance on `TenderRow`, so an inherited deadline is distinguishable rather than only documented) remains ready-for-agent. Was: ready-for-agent (filed 2026-09-07 from the external review's verified findings)
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

## Comments

### 2026-09-15 — API/data-quality review fan-out: INCOMPLETE FIX — `/docs` still calls provisional organizations "single-mention"

**This is rows 1–3 of the table above, still live on prod at rev `9e082fd1`**, on surfaces the
table never listed. Unit 2 named `crates/app/src/v1/sql.rs:705`, `crates/store/src/canonical.rs:346-348`,
`CONTEXT.md:57-61` "and the OpenAPI if it repeats it"; the `/docs` prose const at
`crates/app/src/v1/docs.rs:643` and the served-JSON builder comment at `crates/app/src/v1/json.rs:101-102`
are in neither the table nor the Done note, and `c185ed1` has no hunk touching the provisional line in
`docs.rs` and does not touch `json.rs` at all. Both lines blame to `b4a18a2` (2026-08-31) — the retired
wording predates and survived the sweep. So the "Done when" bullet *"the provisional description matches
the resolver on all four surfaces"* is not met: `/v1/sql/schema` and `/docs` now contradict each other
inside one binary.

Evidence (literal):

```
curl -sS https://tenders.zebreus.click/docs | grep -o 'Organizations are aggregated by identifier.\{0,220\}'
curl -sS https://tenders.zebreus.click/v1/organizations/77984
curl -sS 'https://tenders.zebreus.click/v1/organizations?name_prefix=stadt&limit=3'
curl -sS 'https://tenders.zebreus.click/v1/organizations?name_prefix=gemeinde&limit=5'
```

Served at rev `9e082fd1`:

> "Organizations are aggregated by identifier where the source publishes one, else by (name, country);
> mentions without either stay \<em\>provisional\</em\> single-mention organizations. The provisional flag
> on /v1/organizations tells you which kind you are looking at."

| org id | name | provisional | country | mentions |
| --- | --- | --- | --- | --- |
| 77984 | Katholische Kirchengemeinde St. Birgid (buyer of doe tender 7976396) | true | null | 6 |
| 46912 | Stadt | true | null | 6 |
| 11995507 | Stadt | true | DE | 3 |
| 22371859 | (`?name_prefix=stadt` row 3) | false | — | 2 |
| 4308436 | Gemeinde | true | — | 163 |
| 4 further `?name_prefix=gemeinde` rows | — | true | — | 8, 4, 2, 3 |

77984 and 46912 carry neither identifier nor country — exactly the "without either" case the sentence
says stays single-mention — and hold 6 mentions each, so the claim is false under the narrow reading as
well as the broad one. 5/5 rows of `?name_prefix=gemeinde&limit=5` are provisional and multi-mention.

**Judge:** Verified. (1) System-introduced, not source-published: the sentence is a hand-written HTML
const in `crates/app/src/v1/docs.rs:643` ("mentions without either stay \<em\>provisional\</em\>
single-mention organizations"), served live at `/docs` on rev 9e082fd1 (re-run:
`curl -sS https://tenders.zebreus.click/docs | grep -o 'Organizations are aggregated by identifier.{0,260}'`
still prints it). The behaviour it describes was deliberately retired by issue 234 (CLOSED 2026-08-21:
identifier-less mentions reuse the standing (name_norm, country) row, rows stay provisional) and issue 351
(DONE 2026-09-05: country-less mentions reuse the (name_norm, NULL) row under the wall, plus a 5.76M-row
fold). Live confirms: `/v1/organizations/77984` returns country:null, identifier:null, provisional:true,
mentions:6 — exactly a 351-shape row that the /docs sentence says cannot exist. (2) Not already resolved:
issue 370 corrected this claim on three surfaces (sql.rs:705 column note, canonical.rs comment,
CONTEXT.md:57-62) in c185ed1, and its unit-3 coupling test (sql.rs:1536, asserts the COLUMN_NOTES text
contains neither "single-mention" nor "never merged") pins only the SQL column note.
`git show c185ed1 -- crates/app/src/v1/docs.rs` has no hunk touching the provisional line and the commit
does not touch json.rs at all; `git log -S'single-mention' -- docs.rs` and `-S'deliberately never merged'
-- json.rs` both last land at b4a18a2 (2026-08-31), i.e. the retired wording predates and survived 370's
sweep. 370's own unit-2 list named "sql.rs:705, canonical.rs:346-348, CONTEXT.md:57-61, and the OpenAPI" —
/docs was never in the list. The OpenAPI does not repeat the claim (it never uses the word "provisional").
(3) Actionable: one sentence to rewrite in docs.rs:643 (and the dead comment at json.rs:101-102), ideally
with the 370 unit-3 pattern extended to the /docs const so the same drift cannot recur there. Result
today: two surfaces of the same binary contradict each other — /v1/sql/schema says a provisional row "can
hold many mentions … not a promise of one mention" while /docs says provisional rows are single-mention —
which is the "serves inconsistently" case. Severity low: documentation only, one sentence, no data or
API-shape defect; but it sits on the primary human docs page and actively misleads consumers about what
`provisional:true` means.

**To close:** rewrite `crates/app/src/v1/docs.rs:643` and the dead comment at `crates/app/src/v1/json.rs:101-102`
in the wording unit 2 already landed on the other three surfaces, and widen unit 3's coupling test
(`crates/app/src/v1/sql.rs:1527-1537`, which greps `COLUMN_NOTES` only) to the `/docs` HTML const.

### 2026-09-15 — API/data-quality review fan-out: INCOMPLETE FIX — the OpenAPI's 408 text still says a timed-out query's "server-side work was abandoned"

**This is row 6 of the table above, still live on prod at rev `9e082fd`**, on the second of its two
surfaces. `crates/app/src/v1/docs.rs:382` was corrected by `c185ed1` and the corrected sentence is live;
`git show --stat c185ed1` touches `docs.rs` but **not** `crates/app/data/openapi.json`, even though unit 2's
surface list names "the OpenAPI". The vendored spec is served verbatim via `include_str!`
(`crates/app/src/v1/openapi.rs:18`), so the retired claim is still published — this is a missed residue of
the sweep, not deploy lag and not a separately decided wording.

Evidence (literal):

```
curl -s https://tenders.zebreus.click/v1/openapi.json | grep -o 'its server-side work was abandoned[^"]*'
  -> its server-side work was abandoned. Make the query cheaper (narrow the range, add a LIMIT) before retrying.

curl -s https://tenders.zebreus.click/v1/openapi.json | python3 -c "import json,sys; print(json.load(sys.stdin)['paths']['/v1/sql']['post']['responses']['503']['description'])"
  -> '… every SQL worker thread is pinned by an earlier query that cannot be interrupted …'
```

| surface | source line | what it says | true? |
| --- | --- | --- | --- |
| OpenAPI `/v1/sql` 408 description | `crates/app/data/openapi.json:412` | "its server-side work was abandoned" | **no** |
| OpenAPI `/v1/sql` 503 description | `crates/app/data/openapi.json:414` | "pinned by an earlier query that cannot be interrupted" | yes |
| `/docs` | `crates/app/src/v1/docs.rs:382` (corrected in `c185ed1`) | "the ANSWER is abandoned, but the work is not always … further queries can meet a 503 (issue 238)" | yes |
| `/v1/sql/schema` note | `crates/app/src/v1/sql.rs:812-815` | "a query past the cap — including a slow aggregate — is 408" | silent, not wrong |
| code | `sql.rs:55-59`, :191, :239-245, `SATURATED` :492-493, :887-890 | "the work itself is never stopped — turso exposes no `interrupt()`" | — |

Both OpenAPI strings were authored together in `b4a18a2` (2026-08-31) and are untouched since. Measured
during this review on the live box: a `notice_withheld_fields … LIMIT 1` query returned 408 at **11.0 s**
while its GROUP BY kept running. Scope correction from the review: only the 408 string is wrong — the 503
string is the accurate side of the pair, and the runtime 408 body (`sql.rs:432`) makes no "abandoned" claim.

**Judge:** System-introduced and still live. The 408 description is a hand-written string in the vendored
static file `crates/app/data/openapi.json:412` (served verbatim via `include_str!` in
`crates/app/src/v1/openapi.rs:18`), written in b4a18a2 on 2026-08-31 and untouched since. It states "its
server-side work was abandoned", which the system's own code contradicts: the sql.rs module doc (lines
55-58, "The work itself is never stopped — turso exposes no interrupt()"), the `in_flight` counter comment
(sql.rs:239-245, "keeps computing after the client has gone and after AbortOnDrop has fired"), and the
`SATURATED` const (sql.rs:493) plus the 503 description in the same OpenAPI document ("pinned by an earlier
query that cannot be interrupted"). Issue 238's prod measurement established the mechanism. Not already
resolved: issue 370 (c185ed1, 2026-09-07) corrected this exact claim on the /docs surface (docs.rs, now
reading "The ANSWER is abandoned, but the work is not always …") but c185ed1's file list does not include
crates/app/data/openapi.json, so the OpenAPI twin was missed even though 370's scope line names "the
OpenAPI" as one of the surfaces. c185ed1 is deployed (live /docs shows the corrected sentence; live
/v1/openapi.json still shows the old 408 text), so this is a live residual, not deploy lag. Actionable: one
string edit mirroring the docs.rs wording, optionally with a test in openapi.rs (which already parses SPEC
in two tests) asserting the 408 description does not claim the work is abandoned — the 370 unit-3 coupling
pattern. Correction to the finding: only the 408 string is wrong; the 503 string is accurate, so the scope
is one description, not two. Severity low: a contract-text inconsistency whose practical effect (a client
re-sending a still-heavy rewrite and meeting 503) is already explained by the 503 text and /docs.

**To close:** one string edit at `crates/app/data/openapi.json:412` mirroring the landed `docs.rs:382`
wording (the request is abandoned, the query keeps its worker until it finishes, expect 503 if re-sent too
soon), plus an assertion in `crates/app/src/v1/openapi.rs`'s existing SPEC-parsing tests that the 408
description makes no "abandoned" claim.

### 2026-09-15 — API/data-quality review fan-out: unlisted row — `/v1/sql/schema` says `tender_version_classifications.scheme` is "One of: cpv, nuts", but 9.7% of sampled rows carry scheme `cc`

**Not a regression and not one of the thirteen** — no row of the table names the scheme note and no other
board issue mentions it. It is a fourteenth instance of exactly this issue's class: a `COLUMN_NOTES` string
literal that nothing re-derives, true when issue 50 (RESOLVED-VERIFIED) wrote it on 2026-07-23 and made
false by the text-era projection landing afterwards. Recorded here rather than filed anew. Status untouched
by this comment; the two entries above are what reopened it.

Evidence (literal):

```
Doc text (crates/app/src/v1/sql.rs:674; served at /v1/sql/schema
  tables[name=tender_version_classifications].columns[name=scheme].note): "One of: cpv, nuts."

ssh root@zebreus.click 'echo "SELECT scheme, COUNT(*) FROM tender_version_classifications WHERE tender_id BETWEEN 8000000 AND 8050000 GROUP BY scheme" | /root/sq.sh'
  -> ["cc",35396],["cpv",276812],["nuts",53870]

ssh root@zebreus.click 'echo "SELECT field, code, COUNT(*) FROM tender_version_classifications WHERE tender_id BETWEEN 8000000 AND 8050000 AND scheme='cc' GROUP BY field, code ORDER BY 3 DESC LIMIT 8" | /root/sq.sh'
  -> main/5011 4048, main/5041 2624, main/5031 2576, main/5027 1996, main/5022 1389, main/5043 1241, main/5025 1121, main/5017 1043
```

| scheme | rows, `tender_id` 8000000–8050000 | share | documented? |
| --- | --- | --- | --- |
| cpv | 276,812 | 75.6% | yes |
| nuts | 53,870 | 14.7% | yes |
| **cc** | **35,396** | **9.7%** | **no** |
| total | 366,078 | 100% | |

All `cc` rows sit on field `main`; top codes 5011 (4,048), 5041 (2,624), 5031 (2,576), 5027 (1,996),
5022 (1,389), 5043 (1,241), 5025 (1,121), 5017 (1,043).

REST cross-check at rev `9e082fd` (the SQL above could not be re-run by the verifier — the box helper was
denied by the permission classifier and public `POST /v1/sql` is 401 without a bearer token — so the fact
was re-derived through the served REST surface):

| request | result |
| --- | --- |
| `GET /v1/tenders/8039943` (published 1993-12-31) | `classifications = [{code 5027, field main, scheme "cc"}]`; list-level `cpv = []` |
| `GET /v1/tenders/8039938` | 4 rows, all scheme `cc` (3410/3420/3430/3446), field main |
| `GET /v1/tenders/1900306` (1993-12-31) | scheme `cc`, code 3710 |
| `GET /v1/tenders/8042224` (1994-06-02) | 13 rows, all `cc` (5002, 5011, 5012, …) |
| `GET /v1/tenders/8087528` (text era, 2000-03-02) | cpv 7 + nuts 1, **no** `cc` |
| `GET /v1/tenders?published_before=1994-01-01T00:00:00Z&limit=50` | 50/50 items have `cpv = []` |
| `GET /v1/tenders?cpv=50&published_before=1994-01-01T00:00:00Z&limit=3` | 0 items, though 8039943 carries `cc` 5027 and 8042224 carries 5002/5011 |

Origin: `crates/ingest/src/text/rules.rs:65` maps the text-era header `CC` to `Type::Product` and
`crates/ingest/src/text/parse.rs:1387` emits `classification("cc", raw)`; fixtures
`crates/ingest/tests/text.rs:205` and `:270` assert `scheme == "cc"`; `docs/research/ted-legacy-mapping.md:493`
and issue 11 record CC/CT as the pre-CPV product codes of the 1993-era files. Scope correction from the
review: `cc` is the **early** text-era vintage (1993 through at least June 1994), not the whole 1993–2010
text era — PC (CPV) supersedes CC/CT within the text files, and issue 11's "1993-only" is itself slightly
narrow. 1997 has no tenders in the corpus at all.

**Judge:** Verified live: `SELECT scheme, COUNT(*) FROM tender_version_classifications WHERE tender_id
BETWEEN 8000000 AND 8050000 GROUP BY scheme` returns cc 35,396 / cpv 276,812 / nuts 53,870, while
/v1/sql/schema serves the hand-written note "One of: cpv, nuts." (crates/app/src/v1/sql.rs:674) and the
table notes at :603 and :613 say "CPV and NUTS codes". The third scheme is this system's own label:
crates/ingest/src/text/rules.rs:65 maps the text-era header CC to Type::Product and
crates/ingest/src/text/parse.rs:1387 emits classification("cc", raw); docs/research/ted-legacy-mapping.md:493
records CC/CT as the pre-CPV product codes of the 1993-era files. The rows themselves are source-published,
but the scheme vocabulary and the enum note are this system's, and the note is false today. Not resolved on
the board: no issue mentions the cc scheme or the scheme note. Issue 50 (RESOLVED-VERIFIED) wrote the enum
note on 2026-07-23 before the text era was projected, so it was true when written; issue 370 (open, unit 4
second half ready-for-agent) is the umbrella for exactly this drift class (COLUMN_NOTES literals nothing
re-derives, "correct every row at its source line") but its thirteen-row table does not list the scheme
note, so this is an unlisted row of 370, not a regression of a fixed one. Actionable: a one-line note change
plus the two table notes; the enum test at crates/app/tests/sql.rs:499 does not pin the scheme string.
Surface is SQL-schema only: the REST detail serves all schemes unfiltered (crates/store/src/read.rs:2077)
but the OpenAPI describes classification as {scheme, code} with no enum, so REST docs are not wrong.
Severity low: a docs-only mismatch that misleads someone enumerating schemes or assuming cpv filters reach
the whole corpus; no data is lost or mislinked.

**To close:** extend the note at `crates/app/src/v1/sql.rs:674` to "One of: cpv, nuts, cc" naming `cc` as the
1993–94 pre-CPV TED product code (4-digit, not comparable to CPV, unreachable by `?cpv=` prefix filters) and
fix the two table notes at `:603` and `:613`; nothing fails today because `crates/app/tests/sql.rs:499` does
not pin the scheme string.
