# 287 — the SSE diff drops seq-less in-place `changed` events (the org-merge membership move never reaches a stream)

Status: RESOLVED-IN-CODE 2026-08-26 (owner — found by the SSE adversarial review minutes
after 286 deployed; verified by the owner against sse.rs; all three defects fixed, api
test green end-to-end, full `ops/check.sh` green 65 suites). Deploy pending idle queue.
Kind: correctness (change-feed delivery — SSE only; poll and webhooks were correct)
Severity: HIGH (defeats exactly the event issue 286 was built to carry)
Relates to: 286 (emits the row this drops), 285 (three-transport op agreement), 164 (over-delivery contract), 191 (in-place-write class)
Found by: the 2026-08-26 SSE adversarial review (finding 1 HIGH, finding 2 LOW — both fixed by this).

## The gap

`diff` (app/src/v1/sse.rs ~378) probed every change row at
`seq = version_seq.unwrap_or(0)`. A seq-LESS `changed` row — the in-place
rewrite issue 286 emits when the org-merge repoints party/winner rows — probed
at seq 0, which matches no version, so both presence probes missed and the
`(false, false)` arm `continue`d (only `op == "removed"` had a fallback, for
retirement). The event reached `/v1/changes` poll and webhooks (raw op passed
through) but was silently dropped on SSE:

* a `winner=<survivor>` subscriber never received the `added` for a Tender that
  now matches;
* a `winner=<loser>` subscriber kept a ghost Tender forever.

Finding 2 (LOW, same arm): the survivor ORGANIZATION's seq-less `changed`
row was re-emitted on SSE as `added` (orgs ignore seq, so only the new side
probed true) — the three-transport op disagreement issue 285 closed, reopened
on this one path.

## The fix

In `diff`:
1. A seq-less `changed` probes the CURRENT head instead of seq 0 — new read
   helpers `tender_head_seq` / `lot_head_seq` (read.rs; `ORDER BY seq DESC
   LIMIT 1`, never MAX() — turso does not short-circuit aggregates); orgs keep
   seq 0 (unversioned, ignored).
2. For an unversioned entity (organization), the "old" probe of an in-place
   `changed` is the same current-row query — so `old_matches = new_matches`,
   classifying as `changed`, not `added` (fixes finding 2).
3. The `(false, false)` fallback over-delivers `removed` for an in-place
   `changed` too (not just retirement's `removed`): the pre-rewrite state is
   unevaluable and the subscriber may hold the entity under it — the issue-164
   over-delivery contract, extended to this arm.

**Third defect, found by the test itself:** the merge never rang the change
doorbell — `merge_provisional_organizations_batch` committed its change rows
without `publish_cursor`, so even with the classifier fixed the SSE stream only
learned of the merge's events when some LATER write published. (The first test
run failed exactly here: the row existed, `tender_changes` counted it, the
stream stayed silent.) Fixed with the retirement path's after-COMMIT
`publish_cursor` pattern.

Pinned by `an_org_merge_membership_move_reaches_the_stream` (api.rs): seeds a
provisional-org pair + a winner row on the ingested Tender's head via a raw
connection, runs the REAL `merge_provisional_organizations_batch`, and asserts
the survivor-side subscriber receives `added` and the loser-side one `removed`
(both were silent before the fix — and the test fails if EITHER the classifier
arm or the doorbell publish regresses).
