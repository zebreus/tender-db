# 212 — walks() ignores `tender`, so a bounded `/v1/lots?tender=X&kind=…` read is routed to the shed-only isolated pool and can 503

Status: FIXED ON MAIN, DEPLOY BLOCKED 2026-08-16. Fix in `0625baf` ("read: let a tender= containment bound
suppress isolation routing"), pushed to `origin/main` + the handover branch. Applied the recommended
one-liner: the Lots arm of `walks()` is now
`tender.is_none() && (version_predicate || source.is_some() || kind.is_some())`, and the `let _ = tender`
drop is gone — a `tender=X` bound now SUPPRESSES isolation (the read is a bounded containment lookup over
one Tender's lot slice, index-served, cannot walk), instead of a companion `kind`/`source` re-isolating the
cheap read and 503-ing it under pool saturation. The sparse/absent-density cases WITHOUT a tender bound
(the shapes issue 120 protects) still isolate.

Test `a_tender_containment_bound_suppresses_isolation_even_with_a_companion_filter` covers
tender+kind/source/country → main pool and the no-tender density cases → still isolated; the existing
`isolation_routing` suite stays green (7/7). `walks()` is a pure function so this is fully unit-verified
without a DB.

**NOT yet deployed** — same harness deploy gate (all ssh/curl/`./deploy.sh` refused this session; git
push works). Prod still serves `8938e02`.

Was: needs-triage — CONFIRMED (code) 2026-08-15. Filed from the API review (subagent).
Kind: correctness / routing (issue-120 isolation classifier)
Blocked by: —
Relates to: 120 (the isolated read pool + Shed this misroutes into), 115/116 (the `tender` containment
shape), 117 (the index audit the Lots arm descends from)

## Symptom

`GET /v1/lots?tender=123&kind=Lot` (or `&source=…`, and any other companion filter) is classified as
walk-capable and routed to the isolated reader pool (SLOTS=4, sheds with 503), even though the presence of
`tender` makes the whole read a bounded containment lookup over one Tender's lot slice (whole-corpus max
~2,604 lots) that cannot walk. Under isolated-pool saturation it is refused with
`503 "too many expensive filtered reads in flight"` — and it needlessly consumes an isolated slot a
genuine walk needs — when on the main pool it would return in ~ms.

## Root cause

`crates/store/src/read.rs`:

- `:527-529` — the comment states `tender` "is the containment shape … so it never routes to isolation,"
  then `let _ = tender;` drops it: `tender` is never consulted by `walks()`.
- `:554` — the Lots arm returns `version_predicate || source.is_some() || kind.is_some()`. With
  `tender` set *and* `kind`/`source` set, `kind.is_some()` (or `source.is_some()`) makes the whole
  predicate true → isolation, regardless of the `tender` bound.

So `tender` is documented as "never isolates" but only actually suppresses isolation when it is the
*sole* filter (the arm is false with no companion). Combined with any companion filter, the companion
wins and the bounded read is isolated. The `tender=X` containment query itself
(`read.rs:1435-1444` → the containment branch `read.rs:1344-1356`) reads only that Tender's slice and
applies the companion predicate within it — always cheap, always satisfiable.

## Failure scenario

Isolated pool saturated (4 genuine walks in flight). `GET /v1/lots?tender=123&kind=Lot` — an
always-cheap containment read — returns `503` and burns a slot, instead of the ~ms main-pool response.

## Fix

In the Lots arm of `walks()`, short-circuit to `false` when `filter.tender.is_some()` — the containment
shape bounds every companion predicate, so nothing under a `tender` filter can walk. The
`FILTER_CLASSIFICATION` note already says `tender` "never isolates"; the code needs to let it *suppress*
isolation (return false), not merely abstain from adding it.

```rust
Collection::Lots => tender.is_none() && (version_predicate || source.is_some() || kind.is_some()),
```

## Verification

- Unit: `walks(Lots, {tender: Some, kind: Some(sparse)})` == false; `walks(Lots, {kind: Some(sparse)})`
  (no tender) == true (the sparse-density case issue 120 protects stays isolated).
- Live: `GET /v1/lots?tender=<id>&kind=Lot` served on the main pool (fast, never 503) while the isolated
  pool is busy.
