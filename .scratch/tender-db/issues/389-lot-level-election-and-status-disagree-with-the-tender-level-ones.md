# 389 — the lot row re-decides two facts the tender row has already decided: it serves an exact-zero `value` the tender headline refuses, and it serves `submission_deadline: null` on the lots its own `status=open` returned

Status: ready-for-agent — unit 1 BUILT 2026-09-16 (gate green, not yet deployed; see the build section at the foot for the technique, the decisions recorded with it, and the live acceptance still owed). Unit 2 — the lot `submission_deadline`'s scope against the `status` filter's — is undecided and is this issue's open half. Filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: defect (read layer — `summarise()` in `crates/store/src/read.rs`, the lot row served by `/v1/lots` and by `lot_details` on the tender detail; unit 2 is also a docs defect, in `/docs` and the OpenAPI `Lot` schema)
Relates to: 366 (DONE — its unit 3 retired the display-side "second implementation" of the value
election for "every list shape AND the detail payload" by reading the fold's election
`s.eur_cents = t.current_value_eur_cents`, and its "The exact zeros, DECIDED" section plus `55d239d`
is the decision unit 1 shows still unapplied on lots; 366 mentions lots only for the value BOUNDS),
372 (DONE — `436f73e`, "the lot headline value stops picking one", the only leg ever added to the lot
amount pick in `summarise`; its withheld marker is why there are no negatives left at lot level),
375 (DONE — the same "a second head election exists somewhere else" class, found in the backfill
jobs; the lots pick is the third instance), 378 (DONE — its closing verification is
`?max_value=0` returns nothing, which is exactly the filter unit 1's payload contradicts), 118
(RESOLVED — `ignored_filters` is the array that is supposed to name a filter the collection did not
apply; it is `[]` in both units, so neither failure has an honest signal), 115 (the set-based lot
summary that `summarise` became; the `lot_id IS NOT NULL` election is its implementation detail),
275 (RESOLVED — pins the lots `status` EXISTS as tender-scoped BY DESIGN, on 273's
"status ≡ head-range equivalence"; unit 2 does not dispute that, it says the row does not show the
deadline that decided it), 273 (the head-range equivalence the lots status filter rests on),
370 (unit 4's open half — per-field provenance on `TenderRow` "so an inherited deadline is
distinguishable rather than only documented"; unit 2 is the same axis one level down, lot vs tender
rather than version vs version), 174 (RESOLVED-VERIFIED — r208/r209 deadlines DO project now, which
is why the legacy-era tenders in unit 2 carry a tender-level deadline and no lot-level one)
Blocked by: nothing

## What ties these together

Both units are `summarise()` (`crates/store/src/read.rs` ~2772–2810), the one function that builds
the lot row for `/v1/lots` and for `lot_details` on the tender detail. In both, the lot row runs its
own election over `tender_version_*` facts instead of reading the election the tender level already
made, and in both the disagreement is visible inside a single HTTP response.

| unit | lot field | what the tender row says | what the lot row says | why they differ |
| --- | --- | --- | --- | --- |
| 1 | `value` | `null` — the head column refuses an exact zero (issue 366, `55d239d`) | `{"cents":0,"currency":"GBP"}` | `summarise` elects `MAX(cents)` over `quality IS NULL` lot rows, never calling `sentinel_amount` |
| 2 | `submission_deadline` | `2029-04-29T10:00:00+00:00` — the tender inherits lot-level dates | `null`, on a lot that `status=open` just returned | `summarise` elects `s.lot_id IS NOT NULL` rows only, while the status filter reads the union |

They are mirror images of the same asymmetry: facts inherit UPWARD (a lot's amount and a lot's
deadline both count toward the tender's headline) and never downward, and the lot row re-derives
rather than reads. Each also makes a filter and a payload on the same endpoint contradict each other
— `?max_value=0` does not return the lot whose payload says `0`; `?status=open` does return the lot
whose deadline is `null`. Filed as one issue because a maintainer fixing either is inside the same
function with the same tests open.

## Why it matters

There is no lot detail endpoint (`/v1/lots/13714324` → 404 `"no such endpoint"`), so the list row and
`lot_details` are the lot's only two surfaces, and both are built by `summarise`. A consumer has
nowhere else to look.

Unit 1 gives a consumer a number where the system has decided there is none. `/docs` tells every
reader that the derived value column "treats a 0 as an absence and elects nothing from it … so a zero
never appears in it for any reason", and the tender payload honours that by serving `null`. The same
response then prices the tender's only lot at `0 GBP` from the same published figure. A client
summing lot values, ranking by lot value, or deciding "this contract is worth nothing" reads a
decision the board explicitly made the other way. It also cannot round-trip: `?tender=25773&max_value=0`
returns an empty page while `?tender=25773` serves that lot at `{cents: 0}` — the
payload-versus-filter incoherence issue 366 unit 3 removed one layer up.

Unit 2 hands a consumer of the open-lots feed a row asserted to be open whose only visible deadline
is `null`. `status` is documented for every collection as "By submission deadline", and the row
carries a `submission_deadline` field, so the natural reading — "the deadline that decided `open` is
on the row" — is wrong for 100% of the open-with-lots tenders in the newest 100k ids. A bidder-facing
client filtering `status=open` then sorting or displaying by deadline gets a page it cannot order and
cannot show a date for.

## Why this is ours, not the publisher's

Nothing here comes from a source. In unit 1 the publisher wrote `0 GBP` at both scopes; tender-db
chose (issue 366, "The exact zeros, DECIDED", `55d239d`) that a derived headline must not assert a 0,
and only the tender-level election got the rule — `summarise` still elects `MAX(cents)` over
`quality IS NULL` lot rows with no `sentinel_amount` call and no ceiling. In unit 2 the publisher
published one procedure-scoped deadline (the causing notices for 8436333 are both `ted-export-r209`,
227352-2019 and 070235-2022, an era whose form-section `DATE_RECEIPT_TENDERS` is procedure-level by
design) and the projection stored it faithfully with `lot_id NULL`; what the system then added is two
different readings of that one stored fact on one row — the filter takes the union of tender- and
lot-level dates, the row field takes lot-level only, and the tender row does the opposite of the lot
row (`head_deadline`, `crates/store/src/canonical.rs:1286-1295`, takes MAX over the version with lot
rows included, and `crates/store/tests/deadline_backfill.rs` asserts "a lot-level deadline counts").
Every input needed to behave consistently is already in the process, and `summarise` is Rust, so it
can call the fold's own `sentinel_amount` and read the fold's own deadline directly — there is no
digit-walk transcription problem and no source read required.

Board check: 366, 371, 372, 375, 378, 379, 380 are all DONE and none of them touches `summarise`'s
amount pick; 275 records the lots `status` EXISTS as intended but says nothing about the row field;
174/177 are about parse→projection, not lot-row presentation; 370 unit 4 is cross-version, not
cross-scope. No open issue names the lot-level election or the example ids.

## Unit 1 — the lot headline `value` elects an exact zero the tender headline refuses

Severity: MEDIUM as both lenses rated it (`api-lots`, `dq-lots`).

### Observed (verified 2026-09-14 on prod, rev 9e082fd)

    curl -sS 'https://tenders.zebreus.click/v1/tenders/25773'

serves, in one response:

    "value": null,
    "amounts": [{"field":"estimated_value","lot":null,"value":{"cents":0,"currency":"GBP"}},
                {"field":"estimated_value","lot":"LOT-0000","value":{"cents":0,"currency":"GBP"}}],
    "lot_details": [{"id":62039,"lot_key":"LOT-0000","value":{"cents":0,"currency":"GBP"}, ...}]

Both amounts are quality-unflagged. The tender has no value; its only lot is worth 0.

    curl -sS 'https://tenders.zebreus.click/v1/lots?currency=GBP&limit=10'

→ items include `{"id":62039,"tender_id":25773,"value":{"cents":0,"currency":"GBP"}}` and
`{"id":80385,"tender_id":31033,"value":{"cents":0,"currency":"GBP"}}`.

The sharper form, on one endpoint:

| request | result |
| --- | --- |
| `curl -sS 'https://tenders.zebreus.click/v1/lots?tender=25773&max_value=0'` | `items: []`, `ignored_filters: []` |
| `curl -sS 'https://tenders.zebreus.click/v1/lots?tender=25773'` | serves lot 62039 with `value {"cents":0,"currency":"GBP"}` |

**Scope**, bounded counts on the box:

    ssh root@zebreus.click 'echo "SELECT count(*) AS tenders_null_value_with_zero_lot_amount FROM tenders t WHERE t.id BETWEEN 1 AND 100000 AND t.current_value_eur_cents IS NULL AND EXISTS (SELECT 1 FROM tender_version_amounts a WHERE a.tender_id = t.id AND a.seq = t.current_seq AND a.lot_id IS NOT NULL AND a.cents = 0 AND a.quality IS NULL)" | /root/sq.sh' -> [706]

| id range | tenders: NULL head value + unflagged zero lot amount at the head version | share |
| --- | --- | --- |
| 1–100,000 | 706 | 0.7% |
| 5,000,000–5,100,000 | 23 | 0.023% |

Cohort-dependent, dense at the low-id end. Lot side, ids 1–100,000:

| measure | count |
| --- | --- |
| distinct lots served `value {cents:0}` under a null tender value | 1,280 |
| unflagged zero lot amounts at the head version | 2,513 |
| lots with any unflagged amount | 103,918 |
| unflagged NEGATIVE lot amounts | 0 |
| unflagged nines-run lot amounts | 0 |

A NULL head means those 1,280 lots have no positive unflagged figure either (the head elects over lot
facts too), so all 1,280 are served at `{cents: 0}`. The zero is the only sentinel class still
diverging at lot level — issue 372's withheld marker already took the negatives.

**Mechanism**, verified in the tree:

| site | what it does |
| --- | --- |
| `crates/store/src/read.rs` `summarise()` ~2772–2793 | the lot pick: `MAX(cents)` over `quality IS NULL` lot rows, no zero refusal, no ceiling |
| `crates/store/src/read.rs` (no `sentinel_amount` call at all; only a comment at :1675) | the fold's predicate is never reached from the read layer |
| `crates/store/src/canonical.rs:1361-1375` | the fold filters every tender- AND lot-level `Fact::Amount` through `quality.is_none() && !sentinel_amount(*cents)` |
| `crates/store/src/canonical.rs` (test at :21650) | `sentinel_amount(0)` is true since `55d239d` |
| `crates/store/src/read.rs:1705-1706` | the TENDER pick reads the head column: `t.current_value_eur_cents IS NOT NULL AND s.eur_cents = t.current_value_eur_cents` |
| `crates/store/src/read.rs:2651-2652` | the LOTS value filter compares the TENDER's head column: `(SELECT tt.current_value_eur_cents FROM tenders tt WHERE tt.id = l.tender_id)` |

So the filter refused the zero (it reads the head column) and the payload elected it (it re-derives),
which is why `max_value=0` and the served row disagree.

On "expected": `/docs` states the rule for the derived column, and the OpenAPI `Lot` schema does not
document `value` at all, so the expectation rests on internal consistency rather than an explicit
lot-level doc sentence. The consistency case is concrete without one — one response says the tender
has no value and its only lot is worth 0, from the same published figure.

### Repro

Under two minutes, public API only; the SELECT needs a snapshot read.

1. `curl -sS 'https://tenders.zebreus.click/v1/tenders/25773'` → `value: null`, two unflagged
   `estimated_value` amounts of `0 GBP` (one `lot: null`, one `lot: "LOT-0000"`), `lot_details[0]` =
   `{id: 62039, value: {cents: 0, currency: "GBP"}}`.
2. `curl -sS 'https://tenders.zebreus.click/v1/lots?tender=25773'` → lot 62039 with
   `value {cents: 0, currency: "GBP"}`.
3. `curl -sS 'https://tenders.zebreus.click/v1/lots?tender=25773&max_value=0'` → `items: []`,
   `ignored_filters: []`. Steps 2 and 3 are the same endpoint on the same lot.
4. Second example, same shape: `curl -sS 'https://tenders.zebreus.click/v1/tenders/31033'` →
   `value: null`, both amounts `0 GBP` unflagged, lot 80385 at `{cents: 0, currency: "GBP"}`.
5. Scope: the bounded SELECT above over ids 1–100,000 → `706`; the same SELECT over
   5,000,000–5,100,000 → `23`.

### Done when

- `summarise()` (`crates/store/src/read.rs` ~2772–2793) stops re-implementing the value election: it
  either reads the fold's election the way issue 366 unit 3 made the tender payload read it, or calls
  `sentinel_amount` (and the ceiling) on each candidate row, so there is ONE rule and it cannot drift.
- A lot whose only unflagged amount is an exact zero serves `value: null`, on `/v1/lots` AND in
  `lot_details` — the two surfaces come from the same function, so both move together by construction.
- `/v1/lots?tender=25773` and `/v1/lots?tender=25773&max_value=0` agree: either the lot is served with
  a value and the filter returns it, or it is served `null` and the filter does not.
- A store test seeds a lot whose only unflagged amount is `0` under a NULL tender head and asserts the
  served lot `value` is `null`; a second seeds a lot with an unflagged `0` and an unflagged positive
  amount and asserts the positive one is still elected.
- An app test pins the payload-vs-filter agreement for `max_value=0` on `/v1/lots` the way issue 378's
  verification pins it for `/v1/tenders`.
- Live after deploy: `/v1/tenders/25773` → `lot_details[0].value` is `null`;
  `/v1/lots?currency=GBP&limit=10` no longer serves lots 62039 or 80385 at `{cents: 0}`;
  `/v1/tenders/31033` → lot 80385 `null`.
- The bounded SELECT's 706 (ids 1–100,000) and 23 (ids 5,000,000–5,100,000) are unchanged — this is a
  read-layer election, so nothing is drained and no stored row moves. If a drain is wanted instead,
  say so explicitly on this issue; the default is that the fold already refuses the figure and only
  the read repeats it.

## Unit 2 — `status=open` on `/v1/lots` is decided by the tender's deadline, but the row's `submission_deadline` is lot-scoped only

Severity: MEDIUM as the lens rated it; the adversarial judge argued LOW (the row field is
undocumented so no stated contract is broken, `status=closed` is consistent, the population is the
legacy-era tail, and the fix is a one-line fallback or a doc sentence). Triage's call.

### Observed (verified 2026-09-14 on prod, rev 9e082fd)

    ssh root@zebreus.click 'echo "SELECT t.id, t.source, t.kind, t.current_seq, strftime(\"%Y-%m-%dT%H:%M:%SZ\", t.current_deadline, \"unixepoch\") AS deadline_utc, (SELECT count(*) FROM tender_version_lots vl WHERE vl.tender_id = t.id AND vl.seq = t.current_seq) AS lots FROM tenders t WHERE t.id = 8436333" | /root/sq.sh'
    -> [8436333, "ted", "procedure", 2, "2029-04-29T10:00:00Z", 6]

    curl -sS 'https://tenders.zebreus.click/v1/lots?tender=8436333&status=open&limit=5'
    -> 200, items [{"id":13900615,"lot_key":"LOT-1","submission_deadline":null,"title":"Core Services","version":2},
                   {"id":13900616,"lot_key":"LOT-2","submission_deadline":null, ...}, ...]

All 5 items `submission_deadline: null`, `ignored_filters: []`, `more: true`. `tender_version_dates`
for 8436333 holds exactly 4 rows (seq 1 and 2 × `opening_date` + `submission_deadline`), ALL with
`lot_id NULL` — no lot-level date exists to serve.

The same fact on the three surfaces it has:

| surface | `submission_deadline` |
| --- | --- |
| `/v1/tenders/8436333` (tender row) | `2029-04-29T10:00:00+00:00`, `dates` = 2 tender-level rows |
| `/v1/tenders/8436333` → `lot_details` (6 lots) | `null` × 6 |
| `/v1/lots?tender=8436333&status=open` | `null` × 6, and the rows are returned as OPEN |

    curl -sS 'https://tenders.zebreus.click/v1/tenders/8353548'
    -> "submission_deadline":"2015-03-24T11:00:00+00:00", dates: [1 tender-level row],
       lot_details: 9 lots all "submission_deadline":null

**Scope**, with today's epoch `1789344000` (2026-09-14T00:00Z):

    ssh root@zebreus.click 'echo "SELECT count(*) AS open_tenders_lots_no_lot_deadline, min(t.id) AS example_id, count(DISTINCT t.source) AS sources FROM tenders t WHERE t.id BETWEEN 8436069 AND 8536069 AND t.current_deadline > 1757721600 AND EXISTS (SELECT 1 FROM tender_version_lots vl WHERE vl.tender_id = t.id AND vl.seq = t.current_seq) AND NOT EXISTS (SELECT 1 FROM tender_version_dates d WHERE d.tender_id = t.id AND d.seq = t.current_seq AND d.field = \"submission_deadline\" AND d.lot_id IS NOT NULL)" | /root/sq.sh'
    -> [109, 8436333, 1]        (same query over ids 1-100000 -> [0, null])

| epoch in the query | meaning | count over ids 8,436,069–8,536,069 |
| --- | --- | --- |
| `1757721600` (as the lens ran it) | 2025-09-13 — a year stale | 109 |
| `1789344000` | 2026-09-14T00:00Z, today | **73** |

73 is also the TOTAL number of open-now tenders with lots in that range, so the shape covers
**73/73 = 100%** of them; all source `ted`, all carrying a tender-level deadline. Over ids
1–100,000 (eForms, which dates its lots): 0.

| control | result |
| --- | --- |
| `/v1/lots?tender=132&status=open` (eForms, lot-level date) | lot 322 serves `2026-09-29T10:30:00+02:00` — the field works when the date is lot-scoped |
| `/v1/lots?tender=8436333&status=closed` | 0 items — consistent with the filter's own reading |
| `/v1/lots/13714324` | 404 `"no such endpoint"` — no lot detail surface to fall back to |

No open multi-lot tender with only SOME lots dated exists in ids 1–30,000, so there is no evidence
the `EXISTS` leaks across lots within eForms.

**Mechanism**, verified in the tree:

| site | what it does |
| --- | --- |
| `crates/store/src/read.rs` `lots_query` ~2651 | calls `version_predicates(&mut q, filter, "l.tender_id", SEQ, None, ...)` — `deadline_col: None` |
| `crates/store/src/read.rs` `version_predicates` 934–955 (status leg 946–953) | `EXISTS (SELECT 1 FROM tender_version_dates d WHERE d.tender_id = l.tender_id AND d.seq = <head seq> AND d.field = 'submission_deadline' AND d.utc_seconds > now)` — **no `lot_id` term**, so a tender-level date opens every lot |
| `crates/store/src/read.rs` `summarise` 2795–2810 | the row's deadline: `... AND s.field = 'submission_deadline' AND s.lot_id IS NOT NULL` — lot-scoped only |
| `crates/store/src/read.rs` `pick` 1055, deadline closure 1681–1691 | the TENDER row has no `lot_id` restriction — MAX over all rows, so the tender inherits lot-level dates |
| `crates/store/src/canonical.rs:1286-1295` `head_deadline` | MAX over the version, lot rows included |
| `crates/store/tests/deadline_backfill.rs` | asserts "a lot-level deadline counts" |

Upward inheritance is tested and intended; downward inheritance does not exist; the filter reads the
union. Issue 275 pins the `EXISTS` form deliberately (`lots_status_keeps_the_exists_form`, on 273's
status ≡ head-range equivalence), so the filter is not the thing to change.

On "expected": `status` is documented as "open or closed (by submission deadline)" for every
collection including `/v1/lots`; the lot row's `submission_deadline` is undocumented (the OpenAPI
`Lot` schema types only `id` and `tender_id` with `additionalProperties`, and `/docs` contains zero
mentions of `lot_key` or `submission_deadline`), and nothing in the code records the lot-only scoping
as a deliberate contrast to the filter — `LotRow`/`summarise` doc comments just say "the submission
deadline".

### Repro

Under two minutes; steps 1 and 3–5 are public API only.

1. `curl -sS 'https://tenders.zebreus.click/v1/lots?tender=8436333&status=open&limit=5'` → 200, 5
   items (13900615..13900619), every `submission_deadline: null`, `ignored_filters: []`, `more: true`.
2. The box SELECT on 8436333 above → `[8436333, "ted", "procedure", 2, "2029-04-29T10:00:00Z", 6]`;
   its `tender_version_dates` rows are all `lot_id NULL`.
3. `curl -sS 'https://tenders.zebreus.click/v1/tenders/8436333'` → tender
   `submission_deadline 2029-04-29T10:00:00+00:00`, `dates` = 2 tender-level rows, `lot_details` = 6
   lots all `null`. One response, both readings.
4. Positive control: `curl -sS 'https://tenders.zebreus.click/v1/lots?tender=132&status=open'` → lot
   322 with `2026-09-29T10:30:00+02:00`.
5. Legacy detail case: `curl -sS 'https://tenders.zebreus.click/v1/tenders/8353548'` → tender
   `2015-03-24T11:00:00+00:00`, 9/9 `lot_details` null.
6. Scope: the count SELECT above with `1789344000` in place of `1757721600` → 73 over ids
   8,436,069–8,536,069; the same query over ids 1–100,000 → 0.

### Done when

One of the two is chosen and the OTHER is closed off, not left ambiguous:

- **Either** `summarise` (`crates/store/src/read.rs` ~2795–2810) falls back to the version's
  `lot_id IS NULL` submission deadline when the lot publishes none, ranking a lot-scoped row above a
  tender-scoped one, so every lot returned by `status=open` carries the deadline that decided it —
  and the row says which scope it came from, in the shape issue 370 unit 4 is choosing for
  `TenderRow`, so an inherited deadline stays distinguishable from a published one.
- **Or** the lot field is documented as lot-scoped and explicitly distinct from the filter's basis, in
  `/docs`, in the OpenAPI `Lot` schema (which today types only `id` and `tender_id`), and in the
  `LotRow`/`summarise` doc comments — with the reason the `status` filter reads the union recorded
  beside it, pointing at 275.
- Whichever is chosen, a store test pins the pair: a tender with a tender-level deadline and undated
  lots, asserting both what `status=open` returns AND what the row's `submission_deadline` is, so the
  two can no longer drift apart silently.
- An app test asserts the same on `/v1/lots` and on `lot_details`, since `summarise` feeds both and a
  fix that moved only one would be the next instance of this issue.
- Live after deploy: `/v1/lots?tender=8436333&status=open&limit=5` → the 5 rows carry
  `2029-04-29T10:00:00+00:00` (fallback path) or the field is documented as lot-scoped and the rows
  still read `null` (docs path); `/v1/lots?tender=132&status=open` still serves lot 322 with
  `2026-09-29T10:30:00+02:00`; `/v1/lots?tender=8436333&status=closed` still returns 0 items.


## Unit 1 BUILT 2026-09-16 — the lot pick CALLS the fold's predicate; unit 2 still open

Status: unit 1 ready to deploy, gate green (`GATE-EXIT=0`). Unit 2 (the `submission_deadline` scope)
is untouched and remains this issue's open half.

### What was done

`summarise()` (`crates/store/src/read.rs`) skips a candidate row when
`crate::canonical::sentinel_amount(cents)` — the fold's own function, called, not transcribed — and
when the row's `eur_cents` exceeds `crate::canonical::IMPLAUSIBLE_EUR_CENTS`. The `SELECT` gains
`s.eur_cents` for the second test. Nothing else about the pick changes: it is still `MAX(cents)` over
the survivors, still `quality IS NULL` (issue 372), still first-of-ties for the currency.

The Done-when offered two techniques and this is the second one, for a reason worth recording.
`tender_select_head`'s own comment rules out the first: it warns that walking the digits in SQL would
be "exactly the second implementation" this class of bug is made of, and solves the tender pick by
LOOKING UP the row the fold chose (`s.eur_cents = t.current_value_eur_cents`). That lookup is not
available a level down — `current_value_eur_cents` is tender-scoped and no per-LOT twin exists — but
`summarise` is Rust, so the predicate itself is in reach and there is still exactly one rule.

**The ceiling is applied only where a EUR conversion exists**, and that is deliberate. The tender head
column IS a EUR figure, so an unconvertible amount has nothing to say there and drops out. The lot row
serves the PUBLISHED figure, so blanking it for want of a rate would be a new defect rather than this
one's fix. A 900 000 XXX lot still serves 900 000 XXX.

**Issue 378's zero-conversion rule is NOT carried down**, for the same reason: it governs the derived
EUR column ("a conversion that lands on ZERO is declined"), and the lot row is not that column. Said
here so the omission is a decision rather than an oversight.

### Tests

`crates/store/tests/lot_value_election.rs`, new — NOT an oracle test, because
`lot_summary_equivalence.rs` pins `summarise` against the pre-115 SQL and this is exactly where the
two are meant to diverge. One fixture, seven lots, one page:

| lot | amounts (all `quality IS NULL` unless noted) | serves |
| --- | --- | --- |
| LOT-0000 | `0 GBP` | `null` — the issue's case |
| LOT-0001 | `0 GBP`, `50 000 GBP` | `50 000 GBP` — the zero is skipped, not the lot |
| LOT-0002 | `1 234 EUR` | `1 234 EUR` — control |
| LOT-0003 | `-1 EUR` | `null` — `sentinel_amount`'s oldest leg |
| LOT-0004 | `9.99…e16 EUR`, converted | `null` — over the ceiling |
| LOT-0005 | `900 000 XXX`, no `eur_cents` | `900 000 XXX` — no conversion is not a reason to blank |
| LOT-0006 | `-1 EUR`, `quality = withheld` | `null` — issue 372, pinned so the new arms cannot drop the old one |

Plus `the_lot_payload_and_the_value_filter_agree_about_a_zero`, and in `crates/app/tests/api.rs`
`a_lot_priced_at_zero_is_served_as_no_value_and_agrees_with_the_filter`, which drives the real router:
it ingests the chain, asserts a positive lot value as a precondition, rewrites every head-version lot
amount to an exact zero with `current_value_eur_cents` NULL (the measured shape), then asserts
`/v1/lots`, `lot_details` AND `?max_value=0` all agree — `lot_details.len() == lots.len()` guards
against a vacuous pass.

**All three were run red first**, by short-circuiting the guard. The store test failed with
`left: ("LOT-0000", Some(0), Some("GBP"))` — the live payload from tender 25773, reproduced exactly.

### Live acceptance still owed (after the next deploy)

- `/v1/tenders/25773` → `lot_details[0].value` is `null`; `/v1/tenders/31033` → lot 80385 `null`.
- `/v1/lots?currency=GBP&limit=10` no longer serves 62039 or 80385 at `{cents: 0}`.
- `/v1/lots?tender=25773` and `…&max_value=0` agree.
- The bounded SELECTs are UNCHANGED — 706 over ids 1–100,000 and 23 over 5,000,000–5,100,000. This is
  a read-layer election; nothing is drained and no stored row moves, exactly as the issue specifies.
