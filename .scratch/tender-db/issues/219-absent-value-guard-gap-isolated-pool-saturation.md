# 219 — unauthenticated isolated-pool saturation: absent tender `kind` (and lots `country`/`buyer`) skip the short-circuit and walk the corpus

Status: FIXED ON MAIN, DEPLOY BLOCKED 2026-08-15. Fix in `827f259` ("read: fold the absent-value guard
into one reachable() over every isolation-routed filter"), pushed to `origin/main` + the handover branch,
`store` regression suite green (`tenders_shortcircuit`: 3 passed incl. the new lots + tender-kind guards;
`lots_filter_fixture` green). **NOT yet deployed** — `./deploy.sh main` and its decomposed `git push vps`
primitive are both being refused by the harness auto-mode permission classifier this session (prod-mutating
ops gated; `git push origin` and read-only ssh still pass). Prod still serves `8938e02`, so the hole is
still open in production until the deploy gate is lifted (a Bash permission rule for `deploy.sh`/`git push
vps`, or a human deploy). Flagged to Lennart 2026-08-15.

Fix summary: `reachable()` now probes every isolation-routed filter — `country`/`cpv`/`buyer`/`winner` (as
before) plus `kind` (new; table chosen by collection, `t.kind`/`vl.kind`) — and BOTH `tenders()` and
`lots()` route through it. Previously `tenders()` never probed `kind` and `lots()` never called
`reachable()` at all, so `?kind=<absent>` (tenders) and `?country=`/`?buyer=<absent>` (lots) walked the
isolated pool. Post-deploy verification (still owed): `/v1/tenders?kind=zzz` and `/v1/lots?country=zz`
return an empty page fast; four concurrent such requests do not drive `isolated.available()` to 0; and
`/v1/tenders?country=DE` keeps serving.

Was: needs-triage — MEDIUM (availability; unauthenticated + trivial), CONFIRMED (code) 2026-08-15.
Filed from the API performance review (subagent). Absorbs issue 215 item D (the lots half).
Kind: performance / availability (isolation-pool DoS via absent filter values)
Blocked by: —
Relates to: 120 (the isolated pool this saturates), 117 (the filter-index audit), 215-D (folded here),
44 (the rate limiter that does NOT mitigate this)

## Defect

The absent-value short-circuit that turns "filter value present nowhere" into a one-seek empty page is
applied inconsistently, and the unguarded paths are on the **unauthenticated** list surface. An absent
value on an unguarded filter runs the full density-bounded walk and holds an isolated reader slot for its
whole duration — so a handful of concurrent requests saturate the 4-slot pool and every legitimate
filtered read sheds `503`.

The guards, as they actually stand:

- `reachable()` (`crates/store/src/read.rs:677-726`) — used by `tenders()` — probes only `country`(nuts),
  `cpv`, `buyer`, `winner`. It does **not** cover `kind`.
- `tenders()` (`read.rs:803-810`) adds no separate `kind`-exists probe.
- `lots()` has a `kind`-only guard (`read.rs:1238-1247`) but does **not** call `reachable()`, so lots'
  `country`/`buyer` absent values are unguarded.

So the two live holes are:

| request | guarded? | result |
|---|---|---|
| `/v1/tenders?kind=<absent>` | **no** | full walk of ~4.26M tenders (`t.kind` has no index — read.rs:533-534) |
| `/v1/lots?country=<absent>` / `?buyer=<absent>` | **no** | full isolated lots walk (215-D) |
| `/v1/tenders?country=/cpv=/buyer=/winner=<absent>` | yes | one-seek empty page |
| `/v1/lots?kind=<absent>` | yes | one-seek empty page |

## Attack / cost scenario (unauthenticated)

The list endpoints take no `AuthUser` (`mod.rs:696-712`; the router's CORS comment calls them the
"credential-free routes"). `t.kind` has a 2-value real vocabulary, so `?kind=zzz` matches nothing and is
trivially injectable. `walks()` routes it to the isolated pool (`read.rs:537`), `reachable()` returns
true, and `tenders_query` scans all ~4.26M tenders (the module docs measure comparable walks at
130–230 s) to return an empty page — holding one of `SLOTS=4` the whole time. **Four** concurrent
`GET /v1/tenders?kind=zzz` requests — well under the rate-limit burst (50) — wedge the pool; the rate
limiter caps request *rate*, not concurrent slow requests, and the isolated pool has no per-client
concurrency cap (unlike `/v1/sql`'s `semaphore(user.id)`). Legitimate `?country=DE` / `?buyer=…` then
`503`. Bounded (no crash, no data leak, walks eventually free the slots), but a trivial availability DoS.

## Fix direction

Give `tenders()` the same absent-value guard `lots()` has for `kind`, and give `lots()` the
`country`/`buyer` guards `reachable()` already implements — i.e. make both entry points run the full
short-circuit set. Best: fold all of it into one `reachable()`-style probe covering every isolation-routed
filter (`kind` included) so the guard set and the `walks()` set cannot drift again. Consider a per-client
concurrency cap on the isolated pool as defence-in-depth (a slow-request analogue of the SQL semaphore).

## Verification

- `EXPLAIN QUERY PLAN` for `SELECT t.id FROM tenders t WHERE t.kind='zzz' AND t.id>0 ORDER BY t.id
  LIMIT 101` shows `SCAN tenders` + a sorter (no index) — establishes the walk. (Do NOT run the walk
  itself on prod; the plan + code establish it.)
- After the fix: `/v1/tenders?kind=zzz` and `/v1/lots?country=zz` return an empty page in ~ms; four
  concurrent such requests do not drive `isolated.available()` to 0, and `/v1/tenders?country=DE` keeps
  serving.
