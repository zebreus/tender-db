# 455 — a merged-away organization id answers a bare 404, and as a filter a silent empty page

Status: ready-for-agent — filed 2026-10-01 by the hourly audit, after 453 and 448 removed ~4,800 org rows in one morning.
Kind: API correctness (stable identifiers)
Relates to: 286 (merge emits change events), 448 (e2-altid), 453 (re-key), 49 (`/v1/organizations/{id}`)

## What happens

An organization id is the handle every reader keeps: `parties[].organization_id`, `?winner=`, `?bidder=`, saved
subscriptions. A merge deletes the loser's row and repoints its references to the survivor, so the loser's id
stops resolving. Read 2026-10-01 07:5x UTC on the box (loser 31556979 → survivor 31544276, merged by 448's job
1765):

| request | answer |
|---|---|
| `GET /v1/organizations/31556979` | `404 {"error":{"message":"no such organization"}}` |
| `GET /v1/tenders?winner=31556979` | **`200 {"items":[],"more":false}`** |
| `GET /v1/lots?bidder=31556979` | **`200 {"items":[],"more":false}`** |
| `GET /v1/tenders?winner=31544276` | 200 with that supplier's tenders |

The 404 at least says something changed, although not what. The filter answer is worse: it is a well-formed,
certified-complete empty page. A reader watching a supplier by id goes silently blank the day the supplier's
duplicate row is folded. This happens with every merge rule (r2, e0, p0, e2-altid, rekey), and the merge loops have
removed millions of rows over time.

## What already exists

`org_merge_log(keep, loser, rule, evidence, job_id, at)` with `PRIMARY KEY (loser, at)` records every merge. A
lookup by loser is one index seek; a chain (A→B, then B→C) is a few. The 453 resolver alias already follows
`applied_literal` chains up to 8 hops, so the same bound applies.

## Decision (owner)

1. **Detail.** On a miss, seek `org_merge_log` by loser and follow `keep` (at most 8 hops, cycle-guarded) to a
   live row. Answer **`308 Permanent Redirect`**, with `Location: /v1/organizations/<survivor>` and a JSON body
   `{"error":{"message":"organization merged","status":308},"merged_into":<survivor>}`. A client that follows
   redirects gets the survivor, which is the truth: that org IS the survivor now. A client that does not can read
   the id from the body. An id that was never minted stays a 404.
2. **Filters** (`winner`, `bidder`, `buyer` and any other org-id filter on `/v1/tenders`, `/v1/lots` and the
   change feed): resolve each id through the same chain before seeding, filter by the survivor, and say so in
   the page: `"resolved_filters":{"winner":{"asked":31556979,"merged_into":31544276}}`, beside the existing
   `ignored_filters`. A filter id that is neither live nor merged-away keeps today's empty answer. It is a real
   "no such org", and the lookup costs one seek.
3. **Docs + OpenAPI:** one paragraph on id stability, covering merges, the 308, `resolved_filters` and the 8-hop
   bound.

## Tests

- store: a chain A→B→C resolves A to C; a cycle stops at the bound; a live id resolves to itself; an unknown id is
  None.
- handler: detail 308 + `Location` + `merged_into`; `?winner=<loser>` returns the survivor's page with
  `resolved_filters`; `?winner=<never-minted>` stays an empty 200 with no `resolved_filters`.

## Verify

    curl -s -o /dev/null -w '%{http_code} %{redirect_url}\n' https://tenders.zebreus.click/v1/organizations/31556979
    curl -s 'https://tenders.zebreus.click/v1/tenders?winner=31556979&limit=1' | jq '.resolved_filters, (.items|length)'

- **open:** `404`; then `null` and `0`.
- **done:** `308 …/v1/organizations/31544276`; then `{"winner":{"asked":31556979,"merged_into":31544276}}` and `1`.
