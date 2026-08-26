# 286 — provisional-org merge repoints party/winner rows in place but emits no tender change event, so org-scoped tender subscribers miss/keep-stale events

Status: DIAGNOSED (2026-08-26, owner — exploratory review; verifier verdict UNCERTAIN, mechanism concrete)
Kind: correctness (change-feed completeness) — the 191 in-place-write-invisible class, on the merge path
Severity: MEDIUM
Relates to: 234 (the merge), 191 (in-place reclaim writes invisible to the change gate), 164 (missing removal events), 285 (the same merge's org-op bug)
Found by: the 2026-08-26 change-log review.

## The gap

`merge_provisional_organizations_batch` (canonical.rs ~4210) repoints every
referencing row IN PLACE: `UPDATE tender_version_parties SET organization_id =
keep WHERE organization_id = loser` (~4288), same for `tender_version_bid_parties`
(~4295) and `tender_version_result_winners` (~4314), plus a winners dedup DELETE
(~4300). These hit every version of every affected tender, `current_seq` included,
so they change tender membership for the org-role filters `Filter.buyer` /
`winner` / `bidder` (all canonical org ids). But the only change rows the merge
emits are for `organization` (loser `removed`, survivor `changed` — issue 285). No
`append_change(conn, "tender"|"lot", …)` is called anywhere in the merge path.

## Failure scenario

A subscriber on `/v1/tenders?winner=<keep_org>` (or buyer/bidder): a merge batch
repoints tender T's winner row from loser_org to keep_org; T now matches the filter
but no tender change row exists, so the subscriber never gets an `added` for T
(missing event). Mirror: a subscriber holding T under `winner=<loser_org>` keeps a
stale T forever — after the merge T no longer matches (row repointed) and loser_org
is deleted, yet no `removed`/`changed` tells the subscriber to drop it.

## Why UNCERTAIN / what to confirm

Confirm the org-role filters are canonical-id based on the current-version rows the
merge touches (they are, per read.rs), and that the merge runs outside a projection
that would otherwise re-emit tender events. The merge is its own job
(`merge-provisional-orgs`), so it does not ride a fold's change emission — the gap
is real unless a follow-on projection covers it.

## Fix direction

After each merge batch, collect the distinct tender_ids whose party/bid/winner rows
were repointed and emit a `tender` `changed` row for each (the 191 remedy applied
to the merge), or enqueue a scoped incremental re-fold of the touched tenders so the
normal change emission covers them. Pin with a fixture: a tender matching
`winner=keep` only after a merge must produce a tender change row.
