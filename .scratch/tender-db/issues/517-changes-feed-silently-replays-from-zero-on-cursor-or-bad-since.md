# 517 — `/v1/changes` silently replays the whole feed from 0 on `?cursor=` or an unparseable `since`

Status: done — 2026-10-11, deployed 4910f82 (gate green at 4910f82). Verified on prod: `/v1/changes?cursor=1` → 400
naming `since`; `since=abc` → 200 from the start (the deliberate lenience, unchanged); `since=931236498` resumes there.
Was: built 2026-10-11, NARROWED — only the `cursor` half ships: `/v1/changes?cursor=` is a 400 naming `since`
(test `a_cursor_on_the_change_feed_is_refused_naming_since`). The unparseable-`since` half was REVERTED after the
first gate (GATE-EXIT=101): `the_poll_feed_resets_a_cursor_past_its_head_but_not_at_it` pins that lenience as a
decision on the record (issues 216 and 392: "an unparseable cursor restarts visibly rather than strands", SSE alike;
"if triage does decide to tighten unparseable `since`, it should move together with `/v1/tenders?cursor=` and issue
216's doc comment, not alone"). This issue had not read those. The two halves differ in kind: `cursor` is a
misspelled parameter NAME (the class `deny_unknown_fields` refuses everywhere; it slipped through only because
`Params` is shared), while `since=abc` is a VALUE under a deliberate policy. Tightening values stays a cross-route
decision, not taken here.
Was: ready-for-agent — filed 2026-10-11 from an hourly audit (reading the change feed after issue 516's merges).
Kind: API correctness (public /v1)
Relates to: 392 (resume verdicts: a cursor the feed never issued resets instead of stalling), 399 (a route names what
it ignores), 118 (`ignored_filters`)

## What is wrong

`/v1/changes` resumes from `since`. Every list route pages with `cursor`, and the feed's own response field is
`last_cursor`, so `?cursor=<last_cursor>` is the natural guess. The request is accepted:
- `cursor` is in the shared `Params`;
- `provided_filters` deliberately never names pagination controls;
- so `ignored_filters` comes back `[]`.

`since` is then unset and the page starts at cursor 0. On 2026-10-11, `GET /v1/changes?cursor=931236498&limit=3`
answered events 1–3 (`organization added`, 2026-07-30) with `ignored_filters: []` and `more: true`. A poller that
makes this mistake re-reads the whole log, ~931 M events, and never learns why.

The same happens with a `since` that does not parse. `Params::since` (v1/mod.rs) is
`parse().ok().unwrap_or(0)`, so `since=abc`, `since=12.5` and `since=` all read from the start. Issue 392 closed this
hole for numeric cursors the feed never issued (`cursor_expired` / `cursor_ahead`), but not for non-numeric ones.

## Proposed fix

On `/v1/changes` (and the SSE route, which shares the resume logic, if it reads `since` the same way):
- **`cursor` set → 400:** "`/v1/changes` resumes from `since`, not `cursor`; send the `last_cursor` you stored as
  `since`".
- **`since` set but not an integer → 400** naming the value. `since` absent or empty stays 0, "from the start"
  (a first poll built as `?since=${stored||''}` must keep working); a negative integer keeps 392's `cursor_expired`.

A 400 rather than a reset body: both are client mistakes, not cursors the feed could have issued, and a reset would
send the client back to 0 as well.

Tests (app v1 tests, `--features server`):
- `a_cursor_or_an_unparseable_since_on_the_change_feed_is_refused` (one test, both refusals)
- the existing `since=0` / absent `since` tests stay green.

CHANGELOG entry: an observable change for a client that sends either.

## Verify

After deploy, both `GET /v1/changes?cursor=1` and `GET /v1/changes?since=abc` answer 400.
`GET /v1/changes?since=931236498&limit=3` answers the events after that cursor.
