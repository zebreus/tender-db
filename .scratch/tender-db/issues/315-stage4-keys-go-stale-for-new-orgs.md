# 315 — New orgs are invisible to the candidate scan until a keys rebuild

Status: ready-for-agent (accepted-for-v1 deferral, now filed properly)
Kind: capability (organization layer)
Relates to: 300 (Stage 4 Units 3-5)

## The gap

`org_match_keys` is built wholesale by `build-org-match-keys` and never
maintained incrementally. Every org minted after a build is invisible to
`scan-org-match-keys` — no keys, so no groups, so no candidate edges —
until someone rebuilds. The weekly wet scan therefore re-walks a keyspace
that ages away from the corpus all week.

This was an explicit v1 acceptance (edges are advisory and a rebuild is
cheap), recorded in the Stage-4 plan's Unit-5 notes. It was supposed to be
filed as an issue and never was — it lived only as prose inside the plan
file, which is where deferrals go to be forgotten. Filed now.

## Scale of the staleness

Measured 2026-08-30: the corpus grew ~373 org rows net in one day, and the
satellite holds 13,034,812 key rows over 12,586,144 orgs. A week of drift
is order 10^3-10^4 orgs — small against the whole, but concentrated in
exactly the rows a matcher cares about most (freshly minted provisionals
from this week's notices).

## The two options

1. **Rebuild on the cadence.** The wet build measured **85 seconds** on
   prod for the whole 13M-row satellite. Scheduling it before the weekly
   scan is trivially affordable and needs no new machinery beyond an
   enqueue — but it rewrites 13M rows a week to capture ~10^3 new ones, and
   it breaks the scan's epoch/watermark preconditions if it ever fails
   mid-way (the scan refuses on a non-zero watermark, by design).
2. **Incremental maintenance.** The resolver already writes name variants
   on mention capture (the issue-307 satellite); the same hook could emit
   key rows for a newly minted org. Cheaper per week, but it puts the key
   semantics on the ingest path, where an epoch change (NAME_KEY_EPOCH)
   now has to be handled online rather than by a wholesale rebuild.

Recommendation: option 1 first (it is an enqueue and a doc line), with
option 2 only if the rebuild's 85s ever stops being noise. Decide before
Stage 4's cadence has run long enough for anyone to trust edge coverage.
