# 225 — `buyer=` still walks for ubiquitous non-buyer party orgs; needs a role/org covering index

Status: needs-triage — MEDIUM (usability; the last slow case of the org reverse-lookups, but a small
set of orgs and no regression), CONFIRMED (prod-measured) 2026-08-16. Split from issue 223, which fixed
`winner`/`bidder` fully and `buyer` for all but this class.

Kind: performance (index infra — a covering index + a seed predicate)
Blocked by: 111 (the deferred-index builder that would create the index without a full rebuild)
Relates to: 223 (the reverse-lookup driving fix this completes), 117 (the walk class), 120 (the isolated
pool that keeps this from starving the main pool)

## Defect

Issue 223 seeds an org reverse-lookup's driven set from the participation table's `organization_id`
index. For `winner`/`bidder` the seed is small (an org wins/bids on a bounded set of tenders) and every
tested org is sub-25 ms. For `buyer` the seed is
`SELECT DISTINCT tender_id FROM tender_version_parties WHERE organization_id = ?`, and
`tender_version_parties` holds **every** tender-level party role, not only buyers. A few orgs are parties
on a large fraction of the corpus in a NON-buyer role, so:

- the seed returns a huge candidate set, which the `role LIKE '%Buyer%'` EXISTS then filters to ~0 — the
  cost is paid before the filter narrows anything; and
- `tender_version_parties_org` covers only `organization_id`, so the seed does one **table lookup per
  row** just to read `tender_id` — it cannot be answered index-only.

Measured on prod (serving rev `f3a1628`, 2026-08-16):

| org | `winner=` | `bidder=` | `buyer=` |
|---|---|---|---|
| 2 | 0.023 s | 0.001 s | **19.5 s** |
| 3 | 0.001 s | 0.002 s | **16.9 s** |
| 1, 4, 5, 6 | fast | fast | 0.003–0.29 s (fast) |

No regression — these orgs walked before 223 too, and the isolated pool (issue 120) kept the main REST
pool at 0.03 s throughout — but `buyer=<ubiquitous-party org>` is still a poor experience.

## Fix

1. Add a covering index on `tender_version_parties`. `(organization_id, tender_id)` makes the seed
   index-only (no per-row table lookup); `(organization_id, role, tender_id)` additionally lets the seed
   filter to buyer rows without reading the table, collapsing the candidate set to the org's actual
   buyer tenders. Build it through the deferred-index builder (issue 111) so no full rebuild is needed.
2. Add the role predicate to the buyer seed in `participation_seed`/`tenders_query`/`lots_query` (today
   the seed is role-agnostic and the EXISTS carries the whole `%Buyer%` narrowing). With the covering
   index this becomes an index-only range and the walk disappears.

`role LIKE '%Buyer%'` is a prefix-unfriendly pattern; confirm the real stored buyer role values so the
seed can use an equality/`IN` set the index can seek rather than a `LIKE`.

## Verification

- `EXPLAIN QUERY PLAN` for the buyer seed shows an index-only search on the new covering index (no
  `tender_version_parties` table access in the seed).
- `/v1/tenders?buyer=2` and `?buyer=3` return in well under a second; the previously-fast orgs stay fast;
  and the result set is unchanged from the current (correct, if slow) behaviour.
