# 423 — `/v1/lots?status=open&cpv=` runs past the 30 s bound, and the 503 tells the caller to retry it

Status: ready-for-agent — filed 2026-09-26 14:0x UTC by the hourly audit (step 3), measured on prod, the
routing traced in code (read-only). The fix direction is named below; it is mine to take.
Kind: public API latency (a documented filter pair that cannot complete) + a misleading error message
Relates to: 275 (the open-head seed that fixed status+country — the code's pointer to "275's residuals"
for cpv+status is broken: 275 never mentions cpv), 408 (the 500k-id band this shape does get), 273
(tenders' head-range), 120 (no cancellation — a timed-out query keeps its slot), 115 (the per-lot
`(tender_id, seq)` slice cost), 241 (origin of the deadline message), 422 (adds one more seek to the
same status EXISTS)

## What (measured 2026-09-26 13:5x–14:0x UTC, rev `16933e7`)

| `/v1/lots?…` | answer |
|---|---|
| `status=open&cpv=45&limit=100` | **503 after 30.0 s** (twice), `cpv=45000000` the same |
| `status=open&cpv=45&limit=30` | 200, 5.7 s |
| `status=open&cpv=45&limit=10` | 200, 2.6–3.8 s |
| `status=open&cpv=45&limit=1` | 200, 0.9 s |
| `status=open&cpv=72&limit=10` | 200, **14.4 s** |
| `status=open&country=DE&cpv=45&limit=10` | 200, 1.2 s (the 275 seed) |
| `cpv=45&limit=100` / `status=closed&cpv=45&limit=100` | 200, 0.8 s / 0.7 s |
| `/v1/tenders?status=open&cpv=45&limit=100` | 200, 0.8 s |

Cost grows with `limit` and with the rarity of the cpv prefix among OPEN lots: the lot stream is an
id-ordered walk from the bottom of `lots`, and open lots sit at the top of the id range.

## Why (code trace)

- The shape routes `collection()` → isolated pool → `read_page` → `lots_page` (read.rs ~2780–2804);
  `bounded_walk` is true, so it gets issue 408's band — **500,000 lot ids per page**. The band bounds
  IDS EXAMINED, not time; 408 sized it from tenders (~144–280k ids/s), and this shape manages under
  ~17k lots/s because every walked lot pays the head EXISTS and the cpv EXISTS, each with its own
  `LOT_SEQ` MAX seek, over a `(tender_id, seq)` slice that includes lot-level rows.
- `lot_seed_predicates` (read.rs ~3019–3056) seeds status=open from the open HEADS only when
  `country` is set (issue 275). Its own comment lists "cpv+status (no cpv seed anywhere yet)" as
  unseeded. Tenders avoid all of this through `t.current_deadline > now` on
  `tenders_current_deadline(current_deadline, id)` — ~38k open tenders, then the cpv EXISTS on those.

## The message is wrong AND harmful here

The 503 comes from `deadline_with` (mod.rs ~528–551): "no response within the 30s service bound — a
stalled internal wait, not your request; safe to retry". Nothing stalled — it is this request's own
query. And "safe to retry" is the opposite of true: turso cannot be interrupted, so the query keeps
computing and holds one of the four isolation `SLOTS`; each retry adds another, and once they are full
every caller of an expensive filtered read gets "too many expensive filtered reads".

## Fix direction

1. **Seed the lots status=open walk from the open heads regardless of `country`** — the 275 seed at
   read.rs ~3044 already exists for the country case; extending it to cpv (and bare `status=open`) makes
   the walk start from ~38k open tenders' lots instead of 500k ids of closed ones. Measure with the
   table above before/after; the plan gate (`status_head_range.rs`) pins the tenders side — pin this one.
2. **Fix the 30 s message** so it does not claim a stall or invite a retry when the handler was simply
   still computing (it cannot tell the two apart today — say so rather than guess).
3. Correct the code comment's pointer to 275's residuals to point here.

## Verify

    curl -s -o /dev/null -w '%{http_code} %{time_total}\n' --max-time 40 'https://tenders.zebreus.click/v1/lots?status=open&cpv=45&limit=100'

- **done**: `200` in well under 10 s
- **open**: `503 30.0…` (read 2026-09-26 twice: `503 30.44`, `503 30.57`)

A public read, free — but it holds an isolation slot for its full duration while open, so run it once,
not in a loop.
