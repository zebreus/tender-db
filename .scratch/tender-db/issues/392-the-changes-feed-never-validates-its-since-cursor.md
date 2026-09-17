# 392 — `/v1/changes` never validates `since`: a cursor ahead of the head is echoed back as `last_cursor` with `more:false`, so a poller stalls forever where SSE resets

Status: **DONE 2026-09-17** — verified on the live box: a cursor ahead of the head is detected and SIGNALLED, not echoed back. Was: needs-triage — filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: defect (app — the poll half of the change feed, `crates/app/src/v1/mod.rs`; the guard it is missing already exists one file over in `crates/app/src/v1/sse.rs`)
Relates to: 46 (RESOLVED-VERIFIED — the generation/reset protocol; its status line says it was "settled once across all three transports", but only SSE got the ahead-of-head guard and its conformance test `a_rebuild_moves_the_generation_and_resets_stale_resumes` exercises SSE only), 178 (the same stale-cursor class one transport over — the webhook sweeper's stored cursor, split out of 46 with a design sketch; poll was never split out at all), 211 (RESOLVED — the sibling parameter on this exact endpoint: an out-of-enum `entity` is a 400, which is the value-validation precedent here), 215-C (RESOLVED — the `limit+1` look-ahead in this same handler, i.e. the last time `more` was made to mean what it says), 216 (RESOLVED — the deliberate lenience for the id-ordered LIST cursor; its own status line scopes that lenience to `/v1/tenders?cursor=`, where an unparseable cursor "restarts visibly rather than strands"), 70 (F1 — `oldest_cursor` on the changes feed; the read this fix needs is already there and already cheap), 390 (filed by this fan-out — the `/v1` input-validation cluster; `since` is the one parameter on this endpoint that is neither in that cluster nor validated)

## Observed (verified 2026-09-14 on prod)

Live rev `9e082fd1`, head cursor `605108675` (`/v1` root), `generation: 2`.

```
curl -s 'https://tenders.zebreus.click/v1/changes?since=999999999999&limit=5'
curl -s 'https://tenders.zebreus.click/v1/changes?since=605108676&limit=5'
curl -s 'https://tenders.zebreus.click/v1/changes?since=0&limit=5'
curl -s 'https://tenders.zebreus.click/v1/changes?entity=bogus&since=0&limit=5'
```

| request | status | body |
| --- | --- | --- |
| `?since=999999999999&limit=5` | 200 | `{"events":[],"generation":2,"last_cursor":"999999999999","more":false}` |
| `?since=605108676&limit=5` (head+1) | 200 | `{"events":[],"generation":2,"last_cursor":"605108676","more":false}` |
| `?since=0&limit=5` (control) | 200 | events `cursor` 1..5, `last_cursor "5"`, `more true` |
| `?entity=bogus&since=0&limit=5` (control) | 400 | `{"error":{"message":"unknown entity \"bogus\"; expected one of tender, lot, organization","status":400}}` |
| `?limit=garbage` (control) | 400 | `{"error":{"message":"Failed to deserialize query string: limit: invalid digit found in string","status":400}}` |

`999999999999` is 999,394,891,324 cursors past the head; `605108676` is one past it. Neither was ever issued by this server, and both come back stamped in `last_cursor` — the field the docs tell a client to store and send back verbatim.

The SSE half, same value, same firing:

```
curl -N -s -H 'Accept: text/event-stream' 'https://tenders.zebreus.click/v1/tenders?cursor=999999999999'
```

| transport, `cursor`/`since` = 999999999999 | answer |
| --- | --- |
| SSE (`/v1/tenders`, `Accept: text/event-stream`) | `event: reset` — `{"generation":2,"reason":"cursor_ahead"}` |
| poll (`/v1/changes`) | 200, empty page, `more:false`, the unissued value echoed as `last_cursor` |

Source. `crates/app/src/v1/mod.rs:679-681`:

```rust
fn since(&self) -> i64 {
    self.since.as_deref().and_then(|c| c.parse().ok()).unwrap_or(0)
}
```

The handler (`mod.rs:1301-1341`) never calls `read::latest_cursor` or `read::oldest_cursor`. `mod.rs:1323` is why an unissued value is certified back to the client:

```rust
let last = rows.last().map(|c| c.cursor).unwrap_or_else(|| params.since());
```

The store's `changes_since` selects on `cursor > ?`, so an empty page is indistinguishable from a caught-up one. One file over, `sse.rs:294-315` does the work: below `oldest_cursor` → `reset {"reason":"cursor_expired"}`, `from > read::latest_cursor(&reader)` → `reset {"reason":"cursor_ahead"}`. The poll handler's own doc comment at `mod.rs:1297-1299` promises "Same events, same cursor and same filtering as SSE — a client that cannot hold a connection open loses nothing but latency."

### What is not filed here

The same probe found `since=garbage`, `since=-5`, `since=1.5` and `since=` (empty) all returning 200 with the byte-identical first page of the log (cursors 1..5, `last_cursor "5"`, `more true`). **That half is by design and should not be fixed under this issue.** It matches issue 216's explicit lenience for the id-ordered list cursor, and SSE is lenient the same way (`cursor=garbage` → a silent fresh snapshot, no 400). What makes the ahead-of-head class different is that the two transports measurably disagree on it, and that the poll answer is indistinguishable from "you are up to date". If triage does decide to tighten unparseable `since`, it should move together with `/v1/tenders?cursor=` and issue 216's doc comment, not alone. For the same reason, do not read the OpenAPI `BadRequest` text ("an unknown or misspelled query parameter is rejected rather than silently ignored") as covering this: it is about parameter NAMES. The value-validation precedent on this endpoint is `entity` (issue 211) and `limit` (serde), both of which 400.

## Why it matters

A poller with a cursor ahead of the head receives a healthy-looking page — 200, no error, `more:false`, its cursor acknowledged — and **never receives another event, at any poll interval, forever.** There is no signal to detect and nothing to retry. The identical value on SSE gets `cursor_ahead` and the client re-snapshots.

This is not only reachable through client-side corruption. It is reachable through the documented recovery path. `mod.rs:1337-1340` and `/docs` tell a poll client to store `generation` beside the cursor and, when it moves, to "drop state, re-snapshot the collections, and continue from this response's last_cursor". A generation move is a rebuild, and a rebuild re-issues cursors from the bottom (issue 46: `clear_changes`), so the old cursor is far ahead of the new log's head. On the first poll after that rebuild, `since=<old 605M cursor>` returns an empty page whose `last_cursor` is that same stale value — so the documented recovery hands the client its own dead cursor back as the new one and the client stalls on the step that was supposed to unstall it. SSE clients were given `feed_rebuilt`/`cursor_ahead` for exactly this; poll clients were given the sentence and not the mechanism.

Second, smaller: `last_cursor` is documented as a value to "pass back verbatim", which makes it a value the server vouches for. Echoing an input the server never issued means the one field a client is told to trust round-trips garbage.

## Why this is ours, not the publisher's

No publisher fact is anywhere near this — it is a cursor protocol tender-db designed, serves, and documents. Issue 46 closed with the claim that the reset protocol was "settled once across all three transports", and it genuinely settled SSE (`feed_rebuilt`, `cursor_ahead`, `cursor_expired`) and webhooks' detection half; the poll half got `generation` in the envelope and a paragraph of client instructions, but none of the three guards and no conformance arm. So this is a gap in 46's closure rather than a regression, and the handler's own docstring already commits to the behaviour it is missing. The fix is small because the reads it needs (`read::latest_cursor`, `read::oldest_cursor`) are already used by `sse::start` and already cheap (issue 70 F1).

## Repro

Under two minutes, no token:

1. `curl -s 'https://tenders.zebreus.click/v1' | head -c 200` → note `"cursor":"605108675"` (the head) and `"generation":2`.
2. `curl -s 'https://tenders.zebreus.click/v1/changes?since=605108676&limit=5'` → 200 `{"events":[],"generation":2,"last_cursor":"605108676","more":false}`. Poll it again with that `last_cursor`: same answer, forever.
3. `curl -N -s -H 'Accept: text/event-stream' 'https://tenders.zebreus.click/v1/tenders?cursor=999999999999'` → `event: reset`, `{"generation":2,"reason":"cursor_ahead"}`. Same class of input, different transport, different answer.
4. `curl -s 'https://tenders.zebreus.click/v1/changes?entity=bogus&since=0&limit=5'` → 400. The sibling parameter on the same endpoint is validated.

## Done when

- `changes` reads `read::latest_cursor` and answers a `since` above the head with the SSE protocol rather than an empty page: either a 400 in the standard envelope, or a `{"reset":"cursor_ahead","generation":N}` body. Pick one in triage — what matters is that poll and SSE give the same verdict for the same cursor, which is what `mod.rs:1297-1299` already promises.
- `since` below `read::oldest_cursor` gets the `cursor_expired` verdict the same way (`sse.rs:294-301`), so the poll half has all three resets, not one.
- `last_cursor` is never a value the server did not issue: `mod.rs:1323`'s `unwrap_or_else(|| params.since())` no longer echoes an unvalidated input on an empty page.
- `a_rebuild_moves_the_generation_and_resets_stale_resumes` (`crates/app/tests/api.rs`) grows a poll arm: after a wipe, a pre-rebuild cursor on `/v1/changes` gets the reset verdict instead of an empty page — so issue 46's "settled across all three transports" is true of the test as well as the prose.
- A test pins head+1 specifically (`since = latest_cursor + 1`), since that is the boundary the documented post-rebuild recovery walks into.
- `/docs` and `openapi.json`'s `since` description say what an ahead-of-head cursor gets, and the generation-move recovery paragraph (`mod.rs:1337-1340`) stops being the instruction that creates the stall.
- Controls unchanged: `since=0` returns the first page, a real `last_cursor` round-trips, `entity=bogus` → 400, `limit=garbage` → 400.
- `since=garbage` / `since=-5` still return the first page, unless triage explicitly decides otherwise together with `/v1/tenders?cursor=` and issue 216's lenience comment.

## BUILT 2026-09-15 (owner) — the poll half gets SSE's two resume verdicts

**Triage's pick from the "Done when" fork: the reset body, not a 400.** The handler's own doc
comment is the argument — "Same events, same cursor and same filtering as SSE — a client that
cannot hold a connection open loses nothing but latency." SSE does not error on an incomposable
resume; it emits `reset` with a reason and a cursor of `0`, meaning "drop state, re-snapshot,
start over". A 400 would make the two transports disagree in a second way while fixing the first.

`crates/app/src/v1/mod.rs`, mirroring `sse.rs:294-315` predicate for predicate:

| `since` | answer |
| --- | --- |
| `< oldest_cursor - 1` (a pruned log's horizon) | `{"events":[],"last_cursor":"0","more":false,"generation":N,"reset":"cursor_expired"}` |
| `> latest_cursor` | the same body with `"reset":"cursor_ahead"` |
| otherwise | unchanged |

The `oldest > 0` guard keeps an empty log from rejecting `since=0`, and the `- 1` is SSE's: a
cursor of `oldest - 1` is a legitimate "everything from the start" position. `feed_generation` was
already read once per request and is now read once and reused, so the guarded path costs at most
two extra O(1) cursor reads.

`last_cursor`'s `unwrap_or_else(|| params.since())` is kept, and it is now *safe* rather than
merely harmless: the guards establish that `since` lies within `[oldest-1, latest]`, so an empty
page means "caught up" and echoing the cursor back is the position the client should resume from.
An unissued value can no longer reach that line.

**Deliberately NOT changed:** `since=garbage`, `since=-5`, `since=` still serve the first page, per
the issue's own "What is not filed here" and issue 216's lenience. A test now pins that so the
lenience is a decision on the record rather than an accident.

Tests: `the_poll_feed_resets_a_cursor_past_its_head_but_not_at_it` — head+1 gets `cursor_ahead` and
`last_cursor "0"` (asserted NOT to be the invented value), while the head itself is caught-up with
NO reset and the cursor round-tripping, `since=0` is the first page, `entity=bogus` is still 400,
and unparseable `since` is still lenient. The boundary pair is the point: a guard that fired at the
head would be worse than the defect, because every healthy poller sits there. And
`a_rebuild_moves_the_generation_and_resets_stale_resumes` grows the poll arm the issue asked for, so
issue 46's "settled once across all three transports" is now true of the test, not just the prose.

Docs: the `since` parameter in `openapi.json` and the `/docs` polling paragraph both state the reset
verdict and that it matches SSE.

### Still open

Not deployed — the box is folding issue 385's F14 refold. The acceptance reads (`since=999999999999`
and head+1 against the live feed) are for the next idle window.

## DEPLOYED AND VERIFIED 2026-09-16 — rev `5affd75`

Against the live feed (head cursor 612,967,375 at the time of reading):

| request | answer |
| --- | --- |
| `since=999999999999` | `reset: cursor_ahead`, `last_cursor: "0"`, `events: []`, `more: false` |
| `since=612967376` (head + 1) | `reset: cursor_ahead`, `last_cursor: "0"`, `events: []` |
| `since=612967375` (the head) | **no `reset`**, `last_cursor: "612967375"` — the cursor round-trips, `events: []` |
| `since=0&limit=1` | no `reset`, `last_cursor: "1"`, one event, `more: true` |

The head/head+1 pair is the whole point and it lands exactly on the boundary in production: every
healthy poller sits at the head, so a guard that fired one cursor early would have been worse than
the defect it fixes. Closed.


## Comment — 2026-09-17: verified fixed on prod

    curl '/v1/changes?since=999999999999&limit=3'
    {"events":[],"generation":2,"ignored_filters":[],"last_cursor":"0","more":false,"reset":"cursor_ahead"}

`reset: "cursor_ahead"` and `last_cursor: "0"` — the cursor is recognised as ahead of the head and the
client is told to reset, rather than having its own impossible cursor handed back as if it were the
new position. Checked as part of the 412 sweep, before doing any work on it.
