# 315 — New orgs are invisible to the candidate scan until a keys rebuild

Status: DONE 2026-08-30 (option 1 landed, 340d907) — awaiting its first
Sunday firing (2026-09-06)
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


## RESOLVED 2026-08-30: option 1, on the Sunday tick (340d907)

The wet build now rides the weekly chain AHEAD of the scan. The queue is
FIFO, so pushing it first is the whole dependency; the test pins the
sequence `data-quality, rehash-probe, build-org-match-keys,
scan-org-match-keys` rather than just their presence, because a membership
assert would pass with the build running after the scan it feeds.

Option 2 (incremental maintenance on the resolver's mention-capture hook)
stays unbuilt on its stated cost: it moves key semantics onto the ingest
path, where a `NAME_KEY_EPOCH` change has to be handled online instead of by
a wholesale rebuild. 85 s a week does not buy that.

What makes the weekly rewrite affordable is that its failure mode is
contained and LOUD. A build that dies mid-walk leaves a non-zero watermark
and no covering index; the scan then refuses into `org-edge-scan-alarm`, and
a wet r3 refuses through issue 316's new guard. Neither runs against a
half-built keyspace. Checked before shipping: `reset_org_match_keys` does
NOT touch `org_edge_total`, so tripwire 6's baseline survives the rebuild —
the weekly monotone check still means what it meant.

**Audit finding, same session:** this morning's tick (01:10 UTC) enqueued
only data-quality and the rehash probe. Not a defect — the Unit-5 cadence
code reached prod at 02:31 UTC, an hour and a half AFTER the tick — but it
does mean the weekly scan has never yet fired on its own clock. Next Sunday
is the first firing for both it and the build, and it is the thing to check
on 2026-09-06.
