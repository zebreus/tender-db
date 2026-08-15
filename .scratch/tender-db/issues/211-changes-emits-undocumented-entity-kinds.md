# 211 — /v1/changes emits three undocumented entity kinds (lot_result, bid, contract), contradicting the schema and the "same events as SSE" promise

Status: needs-triage — CONFIRMED (code + live) 2026-08-15. Filed from the API review (subagent).
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
