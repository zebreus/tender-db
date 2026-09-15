# 399 — `/v1/changes` accepts every collection filter, applies only `entity`, and has no `ignored_filters` to say so

Status: needs-triage — filed 2026-09-15 by the owner, found while fixing issue 390 unit 1
Kind: defect (app — the `changes` handler in `crates/app/src/v1/mod.rs`; a contract/observability gap, no wrong stored data)
Relates to: 118 (RESOLVED — introduced `ignored_filters` for exactly this class: a collection that does not honour a parameter must NAME it rather than accept it silently; the changes response never grew the array), 390 (unit 1, the sibling defect one endpoint over — shape-checking `country`/`cpv`; fixing it makes the inconsistency below observable), 211 (the public-feed `entity` enum, the ONE filter this endpoint does honour), 46 / 392 (the "same events, same cursor and same filtering as SSE" contract this contradicts), 336 (CLOSED NOT-WORTH-IT — an unmatchable filter returning an empty page is conventional; a filter that is not applied at all is a different claim)

## Observed (verified 2026-09-15 on prod, live rev `9f6980c`)

`/v1/changes` shares the `Params` struct with every collection endpoint, so it parses `country`,
`cpv`, `source`, `status`, `min_value`, `buyer`, `winner`, … and then uses exactly three of them:
`since`, `limit` and `entity`. The rest are accepted and dropped.

```
curl -s 'https://tenders.zebreus.click/v1/changes?since=0&entity=tender&limit=2&<filter>'
```

| filter added | events | reading |
| --- | --- | --- |
| *(none)* | 2 | baseline |
| `source=nonesuch` | 2 | a source that does not exist changes nothing |
| `status=open` | 2 | ignored |
| `min_value=999999999999` | 2 | ignored |
| `buyer=1` | 2 | ignored |
| `country=ZZ` | 3 of 3 | identical to `country=DE`, identical to no filter |

And the response carries no way to find out: its keys are exactly
`["events", "generation", "last_cursor", "more"]` — **no `ignored_filters`**, the array issue 118
built so a collection that does not honour a parameter says so.

The control, same firing: `entity=tender` DOES change the answer (`last_cursor` 3 → 25284142, all
events `entity: tender`), so the endpoint filters — just by one parameter out of ~20.

Source: `changes()` (`crates/app/src/v1/mod.rs`) reads `params.since()`, `params.limit()` and
`params.entity`, and never calls `params.filter()`. `/v1/changes` is registered as `get(changes)`
with no SSE branch (`mod.rs:160`), so unlike the collection endpoints there is no second arm that
applies the `Filter` — the parameters have no consumer at all on this route.

## Why it matters

The handler's own doc comment is the contract it breaks: *"Same events, same cursor and same
filtering as SSE — a client that cannot hold a connection open loses nothing but latency."* A client
that subscribes to `/v1/tenders` with `Accept: text/event-stream&country=DE` gets German tenders; the
same client falling back to `/v1/changes?country=DE` gets **every** change in the corpus, presented
as its filtered feed, and loses far more than latency. This is the silent-wrong-answer shape, and
`ignored_filters` exists precisely to make it loud.

It is about to become visibly inconsistent, which is what surfaced it: issue 390 unit 1 shape-checks
`country`/`cpv` in `Params::filter()`, so after that deploy `/v1/tenders?country=_E` is a 400 while
`/v1/changes?country=_E` is still a 200 — the same value, the same struct, two answers, because one
route never builds the `Filter`. That asymmetry is a symptom of this issue, not of 390.

## Why this is ours, not the publisher's

No publisher fact is involved. `Params` is one struct shared across routes for parsing convenience,
and nothing makes a route declare which of its fields it consumes; `ignored_filters` is the existing
convention for saying so and this route simply never adopted it.

## Repro

Under a minute, no token:

1. `curl -s 'https://tenders.zebreus.click/v1/changes?since=0&entity=tender&limit=2&source=nonesuch'`
   → 2 events, same as without the filter.
2. Same with `&status=open`, `&min_value=999999999999`, `&buyer=1`, `&country=ZZ` → unchanged.
3. `curl -s 'https://tenders.zebreus.click/v1/changes?since=0&limit=3' | jq keys` → no
   `ignored_filters`.
4. Control: `&entity=tender` vs no `entity` → different `last_cursor` and different event kinds, so
   the endpoint does filter by that one.

## Done when

Triage picks one of two, and they are not equally good:

- **Name them (recommended).** `changes()` reports every filter it received and did not apply in an
  `ignored_filters` array, the way the collection endpoints and the tender-notices sub-resource
  already do (`mod.rs`'s "any other filter the caller sent is named as ignored"). Cheap, additive,
  no behaviour change for a client that sends nothing extra, and it turns a silent wrong answer into
  a visible one. The doc comment's "same filtering as SSE" is then corrected to say what is actually
  shared — the entity kinds and the cursor — or the promise is kept by the second option.
- **Honour them.** `/v1/changes` applies the `Filter` the way the SSE arm does through
  `read_matches`. Strictly better for the client and strictly more expensive: the SSE path probes each
  changed entity's head against the filter, so a poll page would pay that per row. Not obviously
  wrong, but it is a performance decision, not a bug fix, and it should be measured before it is
  chosen.

Either way:

- A test pins the chosen answer for at least `country`, `source` and `status`, alongside the `entity`
  control — the existing behaviour is untested in both directions.
- `openapi.json` marks which parameters `/v1/changes` accepts. Today it inherits the shared parameter
  list, which is the document's own statement that they apply.
- The 390 asymmetry above resolves: `/v1/changes?country=_E` and `/v1/tenders?country=_E` agree, both
  400 (if the filter is honoured and therefore validated) or both documented as ignored here.

## What is NOT filed here

`since`/`limit` validation on this route (issue 216's lenience, and 392's reset guards which landed
2026-09-15). Whether `/v1/changes` should grow an SSE arm at all — it is poll-only by design, and
the collection endpoints carry the streams.
