# 211 — /v1/changes emits three undocumented entity kinds (lot_result, bid, contract), contradicting the schema and the "same events as SSE" promise

Status: REOPENED 2026-09-15 — incomplete fix, not a regression: the Option-1 filter is live and correct
(no undocumented kind reaches the wire at serving rev `9e082fd`), but filtering post-fetch silently redefined
`limit` on the unfiltered poll path — it bounds change-log ROWS scanned, not events, so pages come back short
(158 of 1000) or empty with `more:true`, while the published contract still calls `limit` "Page size"; the
closing claim below that "the code now matches the docs" is false for `limit`. See the 2026-09-15 comment.

History — previous status:
Status: RESOLVED — DEPLOYED & VERIFIED 2026-08-16 (serving rev `005c617`). Prod re-probe:
`/v1/changes?entity=lot_result` and `?entity=bid` → **400**; `/v1/changes?since=0` returns only public
kinds (a 50-row page was all `organization`, no lot_result/bid/contract). Fix in `5023ceb` ("changes: filter the poll feed and
webhook delivery to the public entity kinds"), pushed to `origin/main` + the handover branch. Took
**Option 1** (filter the feed) as recommended: the poll feed AND webhook delivery — the reviewer's issue
was `/v1/changes`, but the webhook sweeper had the identical leak (both call `changes_since(None)` and
serialize via `change_event`) — now serialize only `tender`/`lot`/`organization` (the kinds SSE emits via
`Collection::entity_kind`), and `/v1/changes?entity=<non-public>` returns 400 instead of undocumented 200s.
The cursor still advances over the full fetch (poll `last`/`more`; webhook `to`/`changes.len()`), so a
window of only result-graph rows carries the client past them without redelivery — poll ≡ webhook ≡ SSE.
Result-graph data stays reachable via the parent tender's detail (no data lost). No schema change: the
published `ChangeEvent.entity` enum already listed only the three kinds, so the code now matches the docs
and the "same events as SSE" parity claim is true.

Shared predicate `sse::is_public_change_kind` / `PUBLIC_CHANGE_KINDS`. Tests: new
`the_change_feed_carries_only_the_public_entity_kinds` (asserts the raw log DOES contain result-graph kinds
via a store reader, then that the feed excludes them and `entity=lot_result`/`bid` → 400); existing
changes/collections/filters + all five webhook-delivery tests stay green (api 6/6, webhooks 5/5).

**NOT yet deployed** — same harness deploy gate (all ssh + `./deploy.sh` refused this session). Prod still
serves `8938e02`. The dead-reference-id note in this issue is fully resolved by Option 1 (no unresolvable
id is ever emitted now).

Was: needs-triage — CONFIRMED (code + live) 2026-08-15. Filed from the API review (subagent).
Kind: correctness / API contract (the poll feed vs its published schema)
Blocked by: —
Relates to: 51 (error-envelope), 164/163 (the SSE side of the change feed), 46 (feed_generation)

## Symptom

`GET /v1/changes` returns change events whose `entity` is `lot_result`, `bid`, or `contract` — values
absent from every published schema and never produced by the SSE stream the docs say it mirrors.

Confirmed live:

```
GET /v1/changes?since=0&entity=lot_result&limit=3
→ 200 {"events":[{"entity":"lot_result","op":"added","id":1,"version":2, …}, …]}
```

The `entity=lot_result` filter is accepted and returns rows, although the documented `entity` param enum
lists only `tender|lot|organization`.

## Root cause — the poll feed serializes the raw change kind; SSE queries a narrower set

- Producing side writes six kinds: `crates/store/src/read.rs:26` (`ENTITY_KINDS` = tender, lot,
  organization, **lot_result, bid, contract**); `crates/store/src/canonical.rs:3313-3327`
  (`append_round_changes` emits `lot_result`/`bid`/`contract` change rows).
- Poll path passes them through unfiltered: `crates/app/src/v1/mod.rs:791` calls `changes_since(…,
  params.entity.as_deref())` with no kind restriction; `:794` maps each row through
  `sse::change_event`; `crates/app/src/v1/sse.rs:462-471` emits `"entity": change.entity_kind` verbatim.
- SSE never emits these: `sse.rs:365` drives off `collection.entity_kind()` (`mod.rs:595-603`), which
  only covers tender/lot/organization. So poll ≠ SSE, despite the "Same events as SSE" wording.
- Documented surfaces that say only the three: `crates/app/data/openapi.json:844` (`ChangeEvent.entity`
  enum), `openapi.json:282` (the `/v1/changes` `entity` param enum), `crates/app/src/v1/docs.rs:252`
  (event schema), and the parity claim at `docs.rs:194` / `openapi.json:269`.

## Failure scenario

A client generated from `openapi.json` (strict `ChangeEvent.entity` enum) polls `/v1/changes?since=0` and
crashes on the first `bid`/`contract`/`lot_result` row (enum deserialization failure). A lenient client
silently drops events it was told could only be one of three kinds.

## Fix (pick one; filtering keeps poll ≡ SSE and is the smaller behavioral change)

1. **Filter the poll feed** to the three canonical collection kinds before serialization, matching what
   SSE actually delivers — restores the documented parity, no schema change, and stops the invalid
   `entity=` filter values returning rows. Downside: the result-graph changes are simply not exposed on
   the change feed (they still land in the canonical layer).
2. **Document them**: add `lot_result`/`bid`/`contract` to the `ChangeEvent.entity` enum, the `entity`
   param enum, the `/docs` schema, and drop/refine the "same events as SSE" wording. Downside: SSE would
   still not carry them, so the parity claim needs to go either way.

Decide which is the intended contract (are result-graph changes a public feed concern?). Team-lead call.

## Verification

- After the fix, `GET /v1/changes?since=0` over a window known to contain award activity returns only
  documented kinds (option 1), OR the OpenAPI enum validates every emitted kind (option 2).
- `GET /v1/changes?entity=lot_result` returns 400 (option 1) or is documented (option 2), never an
  undocumented 200.

## Note — the emitted ids are also unresolvable (completeness review, 2026-08-15)

Beyond the undocumented *kind*, the emitted `{entity:"lot_result"|"bid"|"contract", id:N}` events carry an
id that resolves to **no** REST endpoint: there is no `/v1/lot_results/{id}`, `/v1/bids/{id}`, or
`/v1/contracts/{id}` (router mod.rs:143-161), and `Collection::entity_kind` maps only
tender/lot/organization/notice (mod.rs:595-603). So even a client that tolerates the extra kinds cannot
fetch the referenced entity — the change event is a dead reference. This sharpens the fork above:

- **Option 1 (filter the feed to the three snapshot-backed kinds)** also fixes this — no unresolvable id
  is ever emitted. Recommended.
- **Option 2 (document the kinds)** is incomplete on its own: it would additionally require adding
  `/v1/lot_results/{id}` (+ bids/contracts) resolvable endpoints, or the ids stay dead.

The underlying data is reachable today via the parent tender's detail (`lot_results`/`bids`/`contracts`
are embedded there), so no data is lost — only the change feed's own ids are unfetchable.

## Comments

### 2026-09-15 — API/data-quality review fan-out: `limit` on the unfiltered `/v1/changes` is a raw change-log row window, not a page size (INCOMPLETE FIX — not a regression of the original symptom)

**Plainly, what is and is not live at serving rev `9e082fd`.** The defect this issue was filed for is
**fixed and stays fixed**: no `lot_result`/`bid`/`contract` event reaches the wire, and `entity=` restricted
to the non-public kinds is rejected. What is still live is the *contract half* of the Option-1 fix. Filtering
was implemented **post-fetch** (`changes_since(limit + 1)` → `truncate(limit)` → `filter(is_public_change_kind)`),
so on the unfiltered poll path `limit` bounds **change-log rows scanned**, not **events delivered** — while the
published OpenAPI and `/docs` still call it "Page size". This issue's closing line, "the code now matches the
docs", is therefore false for `limit` on the very endpoint it fixed. That is why this is recorded here as an
incomplete fix rather than as a new regression.

**Evidence (literal commands and values).**

```
curl -s 'https://tenders.zebreus.click/v1/changes?since=605108400&limit=1000' | jq '(.events|length), .last_cursor, .more'
→ 158, "605108675", false
  (last_cursor − since = 275 change-log rows consumed; 158 events delivered; last event cursor 605108672)

curl -s 'https://tenders.zebreus.click/v1/changes?since=605099999&limit=11' | jq '(.events|length), .last_cursor, .more'
→ 0, "605100010", true          (rows 605100000..605100010 are eleven consecutive lot_result rows)

curl -s 'https://tenders.zebreus.click/v1/changes?since=605099999&entity=lot&limit=11' | jq '.events|length'
→ 11                            (with entity= set, limit IS an event count — the kind predicate is pushed into SQL)

# judge re-probe, sharper minimal case:
curl -s 'https://tenders.zebreus.click/v1/changes?since=605100027&limit=1' | jq '(.events|length), .last_cursor, .more'
→ 0, "605100028", true          (an empty page while more:true, live)

ssh -o BatchMode=yes -o StrictHostKeyChecking=no root@zebreus.click 'echo "SELECT entity_kind, op, count(*) AS n FROM changes WHERE cursor BETWEEN 605058675 AND 605108675 GROUP BY entity_kind, op" | /root/sq.sh'
→ lot/added 18994, lot/changed 2559, lot/removed 584,
  lot_result/added 17131, lot_result/removed 1826,
  tender/added 7568, tender/changed 1339
```

Hidden (fetched, counted against `limit`, then dropped before serialisation) share by window:

| cursor window | log rows | public-kind rows emitted | hidden `lot_result` rows | hidden share |
|---|---|---|---|---|
| 605108401–605108675 (the `limit=1000` page above) | 275 | 158 | 117 | 42.5 % |
| 605058675–605108675 (tail sample) | 50,001 | 31,044 | 18,957 | 37.9 % |
| 300000000–300020000 (older sample) | 20,001 | 12,095 | 7,906 | 39.5 % |

So the hidden share is structural, not a tail artefact — a poll request returns roughly 62 % of its nominal
page size, and a window dominated by result-graph rows (a retirement/sweep burst of `lot_result removed`)
returns `events: []` with `more: true`.

| request | `limit` | events returned | `more` |
|---|---|---|---|
| `since=605108400&limit=1000` | 1000 | 158 | false |
| `since=605099999&limit=11` | 11 | 0 | true |
| `since=605100027&limit=1` | 1 | 0 | true |
| `since=605099999&entity=lot&limit=11` | 11 | 11 | — |

Code and published text, as they stand today: `crates/app/src/v1/mod.rs:1319-1331` (fetch `limit + 1`,
`rows.truncate(limit)`, then `.filter(|c| sse::is_public_change_kind(&c.entity_kind))`);
`crates/store/src/read.rs:3113` and `:3137-3153` (`changes_since` applies a kind predicate in SQL **only**
when `entity=` is given — hence one parameter with two meanings);
`crates/app/data/openapi.json` `components.parameters.limit` → `"Page size."`;
`crates/app/src/v1/docs.rs:168` → `"Page size, default 100, max 1000"`. Neither the `/v1/changes` description
nor the `/docs` change-feed paragraph discloses that a page may be short or empty while `more` is true.

**Judge's reasoning for why this is ours.** Premise re-verified live: `GET /v1/changes?since=605108400&limit=1000`
→ 158 events (116 `lot`, 42 `tender`), `last_cursor` 605108675, `more:false` — 275 change-log rows consumed for
158 events; the sharper probe `since=605100027&limit=1` → 0 events, `last_cursor "605100028"`, `more:true`, exactly
the case the finding predicts. The cause is in the changes handler (`crates/app/src/v1/mod.rs`): `changes_since(…, limit + 1, entity)`
then `rows.truncate(limit)` then `.filter(is_public_change_kind)`, while the store's `changes_since` only applies a kind
filter when `entity=` is given — so without `entity=` the `lot_result`/`bid`/`contract` rows that issue 211 hides still
count against `limit`. This is behaviour tender-db introduced — its own change-log rows and its own fix in `5023ceb` —
not anything a publisher published. Board state: 211 is RESOLVED/DEPLOYED 2026-08-16 and deliberately chose "the cursor
advances over the full fetch", but it records that only in the issue text and a code comment; it did not touch the
published contract, which still reads "Page size" for every endpoint. Issue 215-C (also resolved) fixed `more` on an
exactly-full page and is unrelated; grepping the board for empty/short poll pages, "carries the client past" and
"until more" found no other issue, open or closed — so this residual is untracked and 211 is its origin. Two genuine
inconsistencies a maintainer can act on: (1) `limit` means "events" with `entity=` set and "raw rows including hidden
kinds" without it; (2) the published "Page size" wording is false for the unfiltered feed. Severity stays **low**:
the documented loop ("pass `last_cursor` as the next `since` until `more` is false") terminates correctly, so only a
client using the undocumented `events.length < limit ⇒ done` idiom stops early and silently misses everything after;
the otherwise-practical cost is throughput (~62 % of nominal per request in the sampled tail). The same shape exists on
the webhook transport (`webhooks.rs` `post()`: a BATCH-row window filtered post-fetch, so a non-reset batch can carry
`events: []`, distinguishable from the reset notice by the `reset` key).

**To close:** either add one sentence to the `/v1/changes` OpenAPI description, `/docs` #changes and the webhook docs —
`limit` bounds change-log rows scanned, pages may be short or empty while `more` is true, and only `more` signals the
end — or make the unfiltered poll collect `limit` *public* events (loop the fetch, or push
`entity_kind IN (tender,lot,organization)` into SQL, checking issue 70's turso planner caveat before relying on an `IN`
over the `changes_entity_cursor` index).
