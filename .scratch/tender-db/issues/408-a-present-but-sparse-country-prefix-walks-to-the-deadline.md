# 408 — `?country=GR` walks to the 30 s deadline: the country seed's density cap measures history, and the walk it hands off to is ordered by id

Status: ready-for-agent — **the 503 IS FIXED on prod 2026-09-17** (`301ee34`): `?country=GR` went from
503-after-30.68 s to **200 in 5.29 s** with correct rows, by raising `COUNTRY_SEED_CAP` past GR's
measured 87,026 entries. It is still ~7x slower than a dense country, and a retired spelling above the
new cap would fail the same way — so **unit 1's decision, option (b), a bounded fallback walk, STANDS**
as the structural fix and the 5.29 s is now evidence for it. See the two comments at the foot.
Was: REPRODUCED FIRSTHAND on an idle box 2026-09-17 (`GR` 503 at **30.68 s**,
`EL` 200 at **0.69 s**, queue empty, so it is the shape and not contention) and the mechanism is
MEASURED by three bounded probes below. It is not the mechanism the report proposed, nor the one
this issue's first draft assumed. Unit 1 is a DECISION, not a measurement: `COUNTRY_SEED_CAP`'s
premise is false for a retired codelist vintage, and the fix has to choose what replaces it.
Kind: performance / availability (issue-61 class; unauthenticated, trivially reachable) — the
`tenders` endpoint with a bare `country` and no `status`, the one combination 273 and 275 both miss
Relates to: 275 (RESOLVED — the same over-cap country failure, on LOTS; its cause #1 is this cause,
and its fix is the shape this one wants), 273 (the `current_deadline` head-range that rescues an
over-cap country on tenders — but only when `status` is in the filter), 219 (the isolated-pool
saturation this keeps reachable), 371 / 223 (the other two live members of "a guard passes and the
read walks"), 117 (no selectivity statistics — why the cap is a proxy at all), 172 (codelist VINTAGE
drift, 2003-vs-2008 — the open classification half is exactly this question one codelist over),
48 (country coding), 171 (`/docs` #caveats, where the GR/EL and GB/UK spellings belong)
Blocked by: nothing

## Verify

    curl -s -o /dev/null -w '%{http_code} %{time_total}\n' --max-time 40 'https://tenders.zebreus.click/v1/tenders?country=GR&limit=2'

- **done** (the 503, unit 1's live defect): `200 <n>` with n well under 30 — the retired spelling answers
- **open**: `503 30.xx` — the walk runs to the deadline again (a cap regression, or a spelling above the new cap)
- the structural half (option (b), the bounded fallback walk) is not this line's to verify: it reads done when a dense country and `GR` answer within the same order of magnitude
- read 2026-09-18 at `71a15b1`: see the comment stream — 200 in 5.29 s on 2026-09-17, unchanged shape

## Observed

Reported by an outside reader against rev `5841c9b`: `/v1/tenders?country=GR` stalls ~30 s and
returns 503, while `?country=EL` answers normally. Separately `?country=GB` returns nothing while
`?country=UK` works.

**The GB/UK half is correct behaviour, not a defect.** NUTS has no `GB` — the United Kingdom is `UK`
— so `GB` is genuinely absent, `prefix_reachable` short-circuits, and the empty page is both right
and fast. `/docs` already says the filter is NUTS and that at country level NUTS is ISO-3166 alpha-2
(`crates/app/src/v1/docs.rs:150`), so the report's "the docs describe this as ISO alpha-2" is a
misreading. What IS missing is smaller and real, and belongs in 171's caveats: nothing tells a reader
that `GR` and `GB` are the spellings that will disappoint them, or why.

**The GR half is a real defect.** Greece was `GR` in NUTS until the 2013 revision and `EL` after it,
so a corpus reaching back to the text era carries both.

Timed live 2026-09-17, `?country=<c>&limit=2`, with a `project` job folding (warm-but-contended):

| value | HTTP | wall | box state |
| --- | --- | --- | --- |
| `GB` | 200 | 2.13 s | fold running — absent from NUTS ⇒ short-circuit to an empty page |
| `UK` | 200 | 1.16 s | fold running |
| `EL` | 200 | 0.44 s | fold running |
| **`GR`** | **503** | **30.68 s** | **queue idle** — `no response within the 30s service bound` |
| `EL` | 200 | 0.69 s | queue idle (control, same minute as the `GR` run) |

The `GR` request was deliberately deferred until the fold drained and then run **once**: a 408/503
here is an uninterruptible statement and prod-box-reads.md forbids stacking retries on it. Measuring
it against an idle box with `EL` as a same-minute control is what rules out contention — the two
differ by 44x on a box doing nothing else.

## The mechanism, measured

Three bounded probes, each one a statement the app itself already runs per request.

**1. `GR` is present**, so `prefix_reachable` admits it — correctly; that guard is one-sided by
construction and answers *matches-nothing* only:

    SELECT code FROM tender_version_classifications
     WHERE scheme = 'nuts' AND code >= 'GR' AND code < 'GS' LIMIT 1
    → 'GR'        (a bare country-level code)

**2. `GR` is OVER the country-seed cap**, so the seed declines — this is the step that decides it.
`country_seed_viable` (`crates/store/src/read.rs:1859`) is a capped count at
`COUNTRY_SEED_CAP = 60_000`; at the cap it returns `false`:

    SELECT COUNT(*) FROM (SELECT 1 FROM tender_version_classifications
      WHERE scheme = 'nuts' AND code >= 'GR' AND code < 'GS' LIMIT 60000)
    → 60000       (at the cap ⇒ country_seed_viable = false, no seed)

With no `status` in the filter there is also no `current_deadline` head-range (273 step 1) to bound
the candidates. So the read falls back to the plain `ORDER BY t.id LIMIT ?` walk
(`crates/store/src/read.rs:1954`) with nothing driving it.

**3. And `EL` is over the cap too** — which is what makes this a real finding rather than a tuning
complaint:

    … code >= 'EL' AND code < 'EM' LIMIT 60000
    → 60000       (also at the cap ⇒ also no seed)

**Both decline the seed. One answers in 0.44 s and the other runs to the deadline.** So the cap is
not what separates them, and no adjustment of the cap fixes this.

## Why — and what is actually wrong

`COUNTRY_SEED_CAP`'s premise is written at `crates/store/src/read.rs:1854-1857`:

> Under the cap ⇒ the seed enumerates quickly and the read stops paying an EXISTS per deadline-range
> candidate; **at the cap ⇒ dense country, the range shape is already the right drive side.**

That second clause is the bug. The cap counts entries in `tender_version_classifications` — density
**over all history**. What the fallback plan needs is density **in the id order it walks**. For a
current codelist spelling those are the same thing, and `EL` is fast because it is dense in the head.
For a **retired** spelling they are opposites: `GR` is dense historically (60k+ entries, comfortably
over a cap meant to mean "plenty of rows about") and effectively absent from the recent id range, so
the walk crosses the whole modern corpus without filling a 50-row page.

This is issue **275's cause #1** — "LU is over the seed cap … the flag never armed" — on the
`tenders` endpoint instead of `lots`. 275 noted that on tenders the over-cap countries "are saved by
the `current_deadline` head-range (273 step 1)", and they are: **when `status` is in the filter.** A
bare `?country=<over-cap>` is the combination neither issue covers, and a retired codelist vintage is
the value class that makes it bite.

Severity is occupancy, not an outage — issue 120's isolation confines it to the isolated pool, which
is doing its job. It is still an unauthenticated request that pins a reader for 30 s, which is 219's
concern with the guard passing rather than missing.

## Units

**Unit 1 — decide what replaces the density proxy.** The measurement above already settles the
diagnosis, so this unit is a choice, not a probe. Recorded so it is not re-litigated:

- **(a) Seed by head presence rather than by history.** The question the plan needs answered is "does
  this country appear near the head of the id order", which `country_seed_viable` cannot answer and
  117 says we have no statistics for. Cheapest honest form: probe the classifications index for the
  prefix restricted to a recent id band; absent there ⇒ seed (enumerate) regardless of total count.
  This fixes the class, including every other retired spelling.
- **(b) Bound the fallback walk.** Stop after N driven rows and return a short page with a cursor.
  Fixes every sparse-value walk on every leg, not just country — the widest fix and the largest
  change to the served contract, since a short page becomes possible where it was not.
- **(c) Alias `GR → EL` at the fold** (394 unit 2's and 292's normalise-at-the-boundary pattern).
  **Tempting and probably wrong here**: it CHANGES SERVED DATA, retires a spelling the publisher
  actually used (against ADR-0003's treatment of published facts), leaves the walk unbounded for the
  next sparse value, and 172's open classification half is exactly the "are two codelist vintages the
  same value" question — which should be decided there, on its own merits, not as a performance fix.

**(a) is the one that matches the diagnosis**; (b) is the one that matches the class. They are not
exclusive.

**Unit 2 — find the other members before fixing.** `GR` was found by a stranger typing the obvious
wrong spelling. Enumerate the retired NUTS spellings the corpus carries and check each for the same
over-cap-but-headless shape. One bounded capped count per candidate, in an idle window, printing each
result (prod-box-reads.md: a loop that accumulates instead of printing is how a shed request becomes a
measurement). Check the sample against a known instance first: `GR` must come back over-cap and `EL`
over-cap-but-fast, or the probe is measuring something else.

**Unit 3 — `/docs` caveat** (issue 171's #caveats): NUTS codelist vintages coexist, so `GR` (pre-2013
Greece) and `EL` both appear, and `GB` is not a NUTS code at all — `UK` is. One sentence beside the
existing CPV-2003/2008 line, which is the same phenomenon on the other codelist.

## Provenance note

The report this came from re-derived seven findings already on the board — 377 (the `5e001394…`
weld, DONE with a deliberate NO GATE), 364 (the OJS-closure weld it mistook for the same thing),
368 unit 2 (the ~16k titleless residue, whose own write-up records that the publisher published no
title), 370 (the `provisional` wording, fixed on its last two surfaces the same day), 394 (DÖE CPV
shapes — the board counts four, the report three), 231 (the 0.6 % sdk-0.1 value share, which is the
recorded POST-fix outcome, from 0.0 %), and 84 (the ~600k "skipped", settled as a counting artifact).

That is not a criticism of the report; it is the useful part of the signal. Those findings are all
**re-findable from outside in an afternoon**, which says the served surface does not carry the
explanations the board holds. `GR` is the one thing it found that the board did not have — and the
mechanism underneath it turned out to be neither what the report proposed nor what this issue first
assumed.

## Comment — 2026-09-17: unit 1 is DECIDED — (b), and (a) turns out not to be cheaply buildable

Read the schema and the read path rather than reasoning from the options as written. Two facts
change the choice.

### (a) "seed by head presence" is not cheaply implementable

The probe it needs is *"does this country prefix appear among recent tender ids"*. The index is

    CREATE INDEX tender_version_classifications_code ON tender_version_classifications(scheme, code);

— **no `tender_id`**. So `MAX(tender_id) … WHERE scheme='nuts' AND code >= ? AND code < ?` has to
visit every matching ROW, which for a dense prefix like `DE` is millions. Widening the index to
`(scheme, code, tender_id)` does not rescue it either: the index would be ordered by code THEN
tender_id, and a country prefix spans many distinct codes (`DE1`, `DE11`, `DE300`…), so a max across
the range still touches every code in it unless the engine skip-scans, which turso is not assumed to
do. The cheap version of (a) does not exist with a realistic index, and the expensive version is the
thing being avoided.

Choosing (a) anyway would need timing evidence for the dense-country seed, and that is a
**characterisation run**, which `docs/agents/prod-box-reads.md` says has *no compliant on-box path*.
So (a) is not merely harder — it is blocked behind an owner conversation, for a fix that is narrower
than (b).

### (b) "bound the fallback walk" needs no new measurement, and no contract change

This was the open worry: a bounded walk returns a SHORT page, and a short page sounds like a breaking
change. It is not, and the reason is already in the served contract:

> Envelope: `{"items": [ … ], "next_cursor": "1234"|null, "more": true|false, "ignored_filters": []}`
> … Paginate by following `next_cursor` until `more` is false.

Pagination is **`more`-driven, not length-driven**. A compliant client follows the cursor until
`more` is false; it is never told that a short page means the end. And `more` is not derived from
page length — every list handler fetches `limit + 1` and truncates:

    crates/app/src/v1/mod.rs:1204,1215  read_ordered(… limit + 1)   → rows.truncate(limit)
    crates/app/src/v1/mod.rs:1290,1301  read_org_named(… limit + 1) → rows.truncate(limit)
    crates/app/src/v1/mod.rs:1535       changes_since(… limit + 1)  → rows.truncate(limit)

So "we stopped early, there is more" is expressible in the envelope exactly as it stands.

### The one detail that must be right

**The cursor must be the last row EXAMINED, not the last row RETURNED.** A bounded walk that gives
up after N driven rows has examined far past whatever it managed to return; emitting a cursor at the
last returned item makes the next page re-walk the same ground and the client loops forever without
advancing. That is the whole risk of (b) and it is a single, testable property:

> given a bound that trips, two successive pages must not examine overlapping ranges, and following
> the cursor must terminate.

### Unit 1: decided

**(b), bound the fallback walk.** It fixes the whole class rather than the country leg — the same
bound serves `kind`, `source`, a sparse `currency`, and every future filter that routes to the
isolated pool — it needs no new index, no new statistics, and no measurement that has no compliant
path. Unit 2 (enumerate other retired spellings) stays useful but drops from blocking to
informational, since the fix no longer depends on knowing which values are sparse.

## Comment — 2026-09-17 (later): I overstated the cap's irrelevance, and the correction is shippable

This issue says, twice, that *"the cap is not what separates them, and no adjustment of the cap fixes
this"*. **The second clause is wrong**, and the reason I wrote it is that I never measured the two
cohorts — both probes were the app's own CAPPED count, which returns `60000` for anything at or above
the cap and therefore cannot distinguish 60,001 from six million.

Uncapped, on prod:

| prefix | classification entries | vs the old 60,000 cap |
| --- | --- | --- |
| `GR` (pre-2013 Greece) | **87,026** | 1.45× over — *barely* |
| `EL` (current Greece) | **656,330** | 10.9× over |

GR was sitting just above the line. Raising the cap past it moves GR into the SEEDED regime, where it
is fast for a different reason than EL is — so a cap change does fix this value, even though the cap
remains the wrong KIND of instrument. Those are two different claims and I collapsed them.

**Shipped**: `COUNTRY_SEED_CAP` 60,000 → **200,000**, chosen to clear GR with room and stay far below
EL, with both measured cohorts pinned by
`the_cap_admits_a_retired_spelling_and_still_declines_a_dense_one` so the constant is a decision
rather than a guess — it fails if anyone restores 60,000.

### What this does and does not settle

- **Does**: `?country=GR` should now seed instead of walking. That is a falsifiable prediction with a
  one-request test, and the result is recorded below either way.
- **Does not**: make the cap measure head density. It still counts history off an index with no
  `tender_id`, and **a retired spelling with more than 200,000 entries fails exactly as GR did.**
  Unit 1's decision — option (b), a bounded fallback walk — stands, and the code comment and the test
  both say so in as many words. This is a cheap fix for a live 503, not a replacement for the
  structural one.
- The crossover is still unmeasured. 200,000 is a judgement; so was 60,000, and nobody wrote down
  where it came from either. Recorded so the next person does not mistake it for a derived number.


## Comment — 2026-09-17: the prediction held — 503 → 200 in 5.29 s

The cap change deployed at `301ee34`, then `?country=GR` run **once** on an idle box with two
same-minute controls:

| value | before (`92f12ec`) | after (`301ee34`) |
| --- | --- | --- |
| **`GR`** | **503 after 30.68 s** | **200 in 5.29 s**, 2 rows, `more: true` |
| `EL` | 200 in 0.69 s | 200 in 0.77 s |
| `UK` | 200 in 1.16 s | 200 in 0.95 s |

The ROWS are right, which matters as much as the latency — the first is
`GR-Veroia: arts, cultural or leisure fac…`, a genuine Greek tender carried under the retired
spelling. The controls did not move, so nothing was traded away for it.

### Read this honestly: fixed, not fast

**5.29 s is ~7x EL's 0.77 s.** The seed does pay at 87,026 entries — it is the difference between a
503 and an answer — but it is nowhere near the sub-second the dense path gives. The result splits:

- **The live defect is gone.** An unauthenticated request no longer pins a reader for 30 s, which was
  issue 219's concern and why this was worth fixing today rather than after the refactor.
- **The structural problem is not.** The cap still measures history rather than head density, 5.29 s
  is still an outlier among country queries, and a retired codelist spelling with more than 200,000
  entries falls off the same cliff GR did. **Option (b) stands**, and the 5.29 s is now evidence FOR
  it rather than a prediction, because it shows the seeded path is not itself cheap at this size.

Recorded as the crossover data nobody had: the seed is viable but expensive somewhere around 87k
entries. That is one point, not the curve — but it is the first real evidence about where this
threshold belongs, and 60,000 never had any.
