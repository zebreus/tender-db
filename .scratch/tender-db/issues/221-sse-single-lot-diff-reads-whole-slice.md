# 221 — SSE single-lot diff (Scope::At) re-reads the whole tender-version lot slice via summarise (quadratic per version bump)

Status: RESOLVED — committed `cb80e60`, awaiting deploy. The diff no longer decorates to classify. Split
`read::lots` into `lots_identity` (the match set, no `summarise`) + decoration; added `read_matches` over
it; the diff loop now classifies added/changed/removed by PRESENCE on each side and decorates only the
NEW side, only under `?include_data=true`. A lot change costs at most one `summarise` (down from up to
four: new+old × decorate, of which the old side's fields and — by default — the new side's were never
emitted). Behaviour-preserving (`entity_event` embeds `data` only under `include_data`); the include_data
diff test now asserts the payload, and `lot_summary_equivalence` pins `lots_identity` ≡ `lots` on the
identity columns across every fixture shape. A true O(1) `lot_id` seek would still need a satellite index
(heavier than this path warranted, and now moot for the common case).

Was: needs-triage — LOW (rare reachability, but per-trigger amplification is quadratic), CONFIRMED (code)
2026-08-15. Filed from the API performance review (subagent).
Kind: performance (SSE diff decoration amplification)
Blocked by: —
Relates to: 115 (the summarise rewrite that fixed the page/containment path — this is its Scope::At residue),
163 (SSE isolation), 55 (SSE snapshot)

## Defect

`read_items(Lots, Scope::At{id})` still routes through `summarise`, whose three queries read the
tender-version's **entire** lot-satellite slices (all lots' titles, amounts, dates) and then pick the one
lot in memory — so decorating a single lot costs O(lots-in-tender), not O(1).

- `crates/app/src/v1/sse.rs:377-389` — the lot `diff` calls `read_items` for the new and the old seq, per
  change.
- `crates/store/src/read.rs:1442-1443` — `lots_query` `Scope::At` path.
- `crates/store/src/read.rs:1514-1600` — `summarise` reads `WHERE tender_id=? AND seq=?` with **no
  `lot_id` predicate**, regardless of how many rows it will discard (no satellite is indexed by `lot_id`).

## Cost scenario

A lot-feed subscriber's diff loop (on the fast pool, `readers.get()` sse.rs:364) processes each `lot`
change with two `read_items` calls (new + old seq). A version bump on a 2,604-lot tender can emit up to one
`lot` change per lot; each re-reads all three full satellite slices twice → on the order of
`lots × 3 × 2 × lots` satellite rows read to emit one version bump's worth of lot events, per subscriber.
Rare (only ~16 tenders exceed 1,000 lots, and it needs an active lot subscription over one of them), but
the per-trigger amplification is quadratic and lands on the main pool.

## Fix direction

For the single-row (`Scope::At`) case, skip `summarise` and fetch that lot's satellite rows directly by
`(tender_id, seq)` with a `lot_id = ?` filter (still a slice scan, but it stops at the one lot's rows), or
bypass summary decoration entirely for the diff probe — which only needs match/no-match against the
subscription filter, not the display fields. A `lot_id` index on the satellites would make it a true O(1)
seek, but is heavier than this narrow path needs.

## Verification

- For a fat tender at its current seq: `SELECT COUNT(*) FROM tender_version_texts WHERE tender_id=<id>
  AND seq=<cur> AND field='title' AND lot_id IS NOT NULL` (repeat for `_amounts`, `_dates`) — rows scanned
  to decorate one lot, vs. the 1 kept. Multiply by the tender's lot-change count for a version bump.
