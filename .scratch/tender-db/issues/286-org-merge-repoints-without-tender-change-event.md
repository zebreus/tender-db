# 286 — provisional-org merge repoints party/winner rows in place but emits no tender change event, so org-scoped tender subscribers miss/keep-stale events

Status: CONFIRMED (2026-08-26, owner) — the UNCERTAIN half is resolved: the
`merge-provisional-orgs` handler (supervisor.rs ~2140) runs the batched in-place
repoint loop and returns; it enqueues NO follow-on `project`, so nothing re-emits the
tender events. The gap is real. Fix DEFERRED to a dedicated firing (batched write path
over ~30M orgs — the turso-cost-sensitive category that caused the 2026-08-26 sweep
incident; not to be rushed at the tail of another unit). Plan below.
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

## Confirmation (2026-08-26, owner)

Read the `Spec::MergeProvisionalOrgs` handler (supervisor.rs 2140–2210): a `loop` over
`merge_provisional_organizations_batch`, checkpoint per batch, then a summary string.
No `project`/`project_incremental` call, no enqueue of one, no ride on a fold's change
emission. `merge-provisional-orgs` is its own STOPPABLE job kind. So the in-place
party/bid/winner repoints reach `current_seq` rows with zero `tender`/`lot` change rows —
confirmed, not merely plausible. (Sibling 285 — the org-op `changed` vs `updated` label —
is a different bug on the same path and is already fixed.)

## Turso-safe fix plan (deferred — its own firing)

Do it the 191 way (in-place change emission), NOT a re-fold — a re-fold of every touched
tender after a corpus-wide merge is exactly the heavy path to avoid.

Per batch, BOUNDED by the batch's loser set (ORG_MERGE_BATCH ids, small):
1. Before the repoint UPDATEs, `SELECT DISTINCT tender_id` from `tender_version_parties`,
   `tender_version_bid_parties`, `tender_version_result_winners` WHERE `organization_id`
   IN (this batch's losers). Union → the touched tender set for the batch. This is an
   indexed IN over a batch-sized id list — never a corpus GROUP BY (the incident lesson:
   any query that would scan the whole table stays off the turso job; here it does not).
2. After the repoints, `append_change(conn, "tender", tid, current_seq_of(tid), "changed",
   now)` for each touched tid — a version_seq-carrying in-place change like issue 191's
   reclaim remedy (confirm 191's exact seq/nullability convention when building). Emit
   once per tender per batch; a tender touched in several batches gets several rows, which
   is fine (the feed is a log, consumers coalesce by id).
3. Extend `OrgMergeBatch` with a `tender_changes` counter and surface it in the summary.

Guards: dry_run must emit nothing (mirror the existing repoint gating). Idempotency — a
re-run after a resumed cursor must not double-touch a merged-away group (the scan already
drops merged groups from scope, so the touched set is naturally per-run).

Pin with a fixture: a tender matching `winner=keep` ONLY after a merge produces a `tender`
`changed` row; a `winner=loser` subscriber sees the same tender's `changed` (its cue to
re-evaluate and drop it). Assert dry_run emits no change rows.
