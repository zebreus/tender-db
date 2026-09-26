# 423 — `/v1/lots?status=open&cpv=` runs past the 30 s bound, and the 503 tells the caller to retry it

Status: **DONE 2026-09-26** — deployed at `69f2a0e` 15:08 UTC (gate 130/130, health 200, 0 error lines);
the `## Verify` block reads done (`200 1.21` / `200 1.15`), paging verified (foot). Was: BUILT 15:0x UTC —
the open-head seed fires for a cpv prefix with `status=open`, the 30 s message no longer claims a stall or
invites a retry, the 275 pointer is corrected. Was: filed 2026-09-26 14:0x UTC by the hourly audit.
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

## BUILT 2026-09-26 — measured first, then the seed

**Measured on prod through `/v1/sql` (bounded, one at a time, ~1.5 s of each figure is ssh):**

| statement (lots page, 101 rows, `now` = run time) | result |
|---|---|
| seed alone: open tenders with head cpv `45%` | **8,647** (of 39,871 open), ~1 s |
| unseeded, one 500k-id band, cpv `45` | **>10 s (408)** |
| seeded (open-head IN before the head EXISTS), cpv `45` | full page 101 rows, ids 322..191,361, 2.3 s |
| seeded (seed after the head EXISTS — the shipped order), cpv `45` | same 101 rows, 3.5 s (first, colder) |
| unseeded, one band, cpv `72` | **>10 s (408)** |
| seeded, one band, cpv `72` | 70 rows, 2.1 s |

**Decisions.**
- **Seed cpv, not bare `status=open`.** The seed is the existing 275 arm generalised: `status=open` AND
  (an over-cap country OR a cpv prefix) → `l.tender_id IN (SELECT t.id FROM tenders t WHERE
  t.current_deadline > ? [AND nuts EXISTS] [AND cpv EXISTS])`, each prefix tested per tender at
  `t.current_seq`. Bare `status=open` stays unseeded: it fills from the dense walk (~2 s for 100) and a
  seed would enumerate every open tender's lots to sort them — no better for a value that dense.
- **Exactness.** `t.current_deadline > now` is 273's proven status-open equivalence; since issue 422
  the lots `status` EXISTS carries the same horizon as the election, so seed and predicate agree on a
  typo too. The per-lot predicates still decide membership (pinned: seeded and seed-stripped statements
  return the same lots on a fixture with a head that moved off cpv 45, a closed tender, and a
  horizon-typo tender).
- **Seed placement left as is** (after the head EXISTS, like the org and country seeds). The one-shot
  measurement favoured seed-first by ~1 s but the pair was not cache-controlled, and moving it changes
  every seeded lots shape — a separate measurement if it is ever worth it.
- **The message** (`deadline_with`): now says the request was still being served, that the service
  cannot tell a slow filter from a stall, that the work may continue after the answer, and to narrow the
  filters or retry later — never "not your request; safe to retry". The unit test pins both phrases OUT.
  `REQUEST_DEADLINE`'s doc no longer claims the bound only fires on an outage.

**Tests**: `crates/store/tests/lots_open_head_seed.rs` — the statement (cpv seed present, per-tender at
the head, one seed carrying both prefixes, bare status and cpv-without-status unseeded) and the superset
trap (seeded = seed-stripped answer). **Mutation-checked**: narrowing the arm back to country-only fails
both tests.

**After deploy (15:08 UTC, `69f2a0e`)** — `/v1/lots?…`:

| shape | before | after |
|---|---|---|
| `status=open&cpv=45&limit=100` | 503, 30.4 / 30.6 s | **200, 1.21 / 1.15 s** |
| `status=open&cpv=72&limit=10` | 200, 14.4 s | **200, 0.79 s** |
| `status=open&cpv=45&limit=10` | 200, 2.6–3.8 s | 200, 0.82 s |
| `status=open&country=DE&cpv=45&limit=10` | 200, 1.19 s | 200, 0.89 s |
| `status=open&limit=100` (unseeded, unchanged) | 200, 2.0 s | 200, 1.73 s |
| `status=open&country=DE&limit=100` | 200, 1.0 s | 200, 0.92 s |
| `status=closed&cpv=45&limit=100` | 200, 0.70 s | 200, 0.64 s |

Paging through the seeded stream, three pages of `status=open&cpv=45&limit=100`: ids strictly ascending,
no overlap across pages, 0.9–1.2 s each. The walk surfaced a pre-existing contract gap, filed as **424**:
the lots `status` EXISTS decides per TENDER (no `lot_id` term), so a lot is returned as open on a SIBLING
lot's future deadline while its own row serves `null` (lot 219239) or its own past date (187173/187174).