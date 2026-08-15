# 205 — the 41M index-build cap has been crossed: two read-path indexes go unbuilt after a rebuild

Status: needs-triage — filed 2026-08-15 (owner), observed live at the end of the full `rebuild=true`
(job 1) that materialised 187/86/48. **UPGRADED same day to HIGH after the deploy that followed
exposed a ~22-minute unserved boot — see "SEVERE: the first post-rebuild boot blocks /health for ~22
minutes" below. That is the priority; the missing indexes are its cause.**
Kind: performance / availability (latent under-indexing → a startup outage on every restart)

## SEVERE: the first post-rebuild boot blocks /health for ~22 minutes (2026-08-15)

Deploying d93fadf right after the rebuild (a routine app-only change) restarted the service at
16:08:07 CEST. The new server bound `:8080` immediately but **did not answer `/health` until
16:30:17** — ~22 minutes — during which the public backend was down and `deploy.sh`'s own
health-check (120×~11s ≈ 22 min of patience) exhausted and reported a false `health check FAILED:
returned 000`, even though the switch had already happened and the server came up seconds later.

What it was doing: strace showed a pure `pread64` read scan (zero writes), ~72% CPU, RSS steady —
an active, bounded, CPU-bound scan, not a hang. It ended exactly when the six over-cap index
refusals logged (16:30:17), and the server served immediately after. So the ~22 min is the
end-of-boot index-handling path doing heavy READ work over the now-huge tables before `/health` is
allowed to answer. The refusal *decision* itself is cheap — `too_large_to_build` is
`SELECT MAX(rowid)` (O(1)) — so the scan is the surrounding boot index work (building the sub-cap
deferred indexes at startup, and/or a first-open WAL checkpoint of the freshly-rebuilt DB), not the
size check. Exact culprit still to pin (next step: instrument the boot ordering).

Why it is HIGH, not a one-off: this is the first re-open of the DB after a rebuild (the rebuilding
process never re-opened it), and it will recur on **every** restart that follows a rebuild — every
deploy, every crash-restart, every reboot — each a ~22-min outage, and each a false `deploy.sh`
failure. It is issue 61's "slow boot" regressed for the post-rebuild case.

**Fix (the robust one, independent of the index cap): serve `/health` BEFORE any heavy startup DB
work.** Open the listener, answer `/health` 200 (process up, DB answers a trivial ping), and run
index detection / builds / checkpoints in a BACKGROUND task. That is the issue-61 principle; the
post-rebuild path violates it. With that, a restart is never an outage regardless of index state.
Building the indexes index-first (below) removes the underlying work too, but serving-before-work is
the guarantee.

---


Blocked by: —
Relates to: 111 (the deferred-index builder + its row cap), 62 (deferring org indexes on the rebuild),
117 (the org read is already the slow one), 120 (isolation routing contains, but does not cure, the
org-list walk)

## What was observed

The index builder (issue 111) **refused SIX indexes** — at the rebuild's finalization AND again on
the next boot — because their tables exceed the 41,000,000-row cap. All six are therefore **absent**,
and several sit on hot read paths:

| index | table(cols) | ~rows | serves (read path) |
|---|---|---|---|
| `tender_version_classifications_code` | tender_version_classifications(scheme, code) | 138.9M | **country + cpv filters** (the `reachable()` prefix probe + `version_predicates` EXISTS) on /v1/tenders and /v1/lots |
| `tender_version_result_winners_org` | tender_version_result_winners(organization_id) | 127.8M | **winner filter** |
| `tender_version_parties_org` | tender_version_parties(organization_id) | 77.3M | **buyer filter** |
| `tender_version_bid_parties_org` | tender_version_bid_parties(organization_id) | 66.8M | org→bids reverse lookups |
| `tender_version_bid_parties_version` | tender_version_bid_parties(tender_id, seq) | 66.8M | tender-detail **bids** section (`/v1/tenders/{id}`) |
| `organization_mentions_org` | organization_mentions(organization_id) | 41.3M | /v1/organizations **mentions count** + buyer/winner org joins (JUST crossed the cap — new this rebuild) |

So the country/cpv/buyer/winner filters on the collection endpoints, the org list's mentions count,
and the tender-detail bids section all now run without their index. Isolation routing (120) keeps the
collection walks off the main pool, but tender detail runs on the MAIN pool — measure that one first
(112 rule 6). `tender_version_classifications_code` at 138.9M is the widest exposure: country and cpv
are the headline filters.

## Why it matters

- **`organization_mentions_org(organization_id)` — the sharper, NEW regression.** The table is
  41.27M rows, just 0.6% over the cap, so it crossed only recently: prior rebuilds (table < 41M) had
  the auto-builder BUILD it; this one refused. `/v1/organizations` computes a per-row
  `(SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id)` for its `mentions`
  column (read.rs `organizations_query`); without the index each org's count scans 41M rows, so a
  page of 100 orgs is 100 such scans. Issue 120's isolation routing keeps this off the main reader
  pool (it will shed, not wedge), so it is a degradation of a headline endpoint, not an outage — but
  it is a regression this rebuild introduced and it will not self-heal on the next rebuild either.

- **`tender_version_bid_parties_version(tender_id, seq)` — long-standing, verify the path.** At
  66.8M rows it has been over the cap for several rebuilds, so its absence is not new. Whether it
  actually sits on a hot read path needs confirming before prioritising: the tender-detail bids
  section (`/v1/tenders/{id}`) reads bid parties, and tender detail runs on the MAIN reader pool
  (not isolated), so if this index is the one that path needs, a 66.8M-row scan there is the worse
  exposure. **Measure which index the bids query actually plans against at prod scale first**
  (112 rule 6 — a plan names the path, only the clock names the cost).

## Root cause

Issue 111's cap deliberately blocks an end-of-run *bulk* `CREATE INDEX` (a ~1–3 GB sort would
re-introduce the multi-minute slow boot 61 removed). The cap was safe when these tables were
smaller; organic corpus growth has now pushed both past it. The auto-builder's own message names the
correct fix: **build the index INDEX-FIRST at a rebuild** — create it empty, before the fold
populates its table, so it is maintained incrementally and never needs the giant end sort, whatever
the final size. These two indexes are evidently NOT in the rebuild's index-first set (other large
indexes must be, or the rebuild could never finish under the cap), so they fall through to the
end-of-run auto-builder, which correctly refuses.

## Fix

- **Durable:** add `organization_mentions_org` and `tender_version_bid_parties_version` (after
  confirming the latter is load-bearing) to the rebuild's index-first creation set, alongside
  whatever large indexes already build that way. Then the cap never applies to them and they exist
  after every rebuild regardless of table size. Materialises on the next full rebuild.
- **Consider** a guard: if a *known-needed* index is missing AND its table is over the cap at
  end-of-run, that is a louder signal than a log line — it means a read path silently lost its index
  and only the next rebuild can restore it. A `/health/deep` check or a coverage cell would surface
  it instead of leaving it in journald.

## Immediate remediation (optional, this does not need the durable fix first)

The queue is idle post-rebuild — a maintenance window. A one-off `CREATE INDEX` for
`organization_mentions_org` (~1 GB sort, minutes) would restore the org-list read now rather than
waiting for the next rebuild. Do it deliberately, off-peak, and only after deciding it is worth the
one-time sort; the isolation routing means the degradation is contained in the meantime.

## Acceptance

Both indexes exist after a full rebuild (built index-first, not end-of-run); the org-list `mentions`
count seeks its index rather than scanning; a known-needed index that goes missing is surfaced louder
than a journald line.
