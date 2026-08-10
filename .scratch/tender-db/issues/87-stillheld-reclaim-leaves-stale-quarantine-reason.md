# 87 — a failed reclaim leaves a stale quarantine reason, so the real failure cause is never recorded

Status: RESOLVED-VERIFIED on prod (2026-08-10, orchestrator). Deployed in 6ed1b0f; the
residual re-examination ran as job 599: `reprocess unknown-customization LIKE %eforms-de-1%`
→ "18 package(s): 0 reclaimed, 238 still held, 0 already parsed, 0 skipped by dispatch
policy; still held by current reason: unrepresentable-value 238". The last acceptance item
is met exactly as designed: the residuals no longer claim 'no vendored SDK metadata' — they
are truthfully labeled `unrepresentable-value` (the predicted malformed-money cohort; the
class issue 144 already analyzes, ADR-0004 holds by design). Reason freshness is now a
falsifiable check, and attempts/last_attempt_at stamp on every pass. (238 vs the earlier
"241": the count was approximate across sources; the LIKE-scoped bucket held 238.)

What landed, per the design notes below:
- Both `StillHeld` exits of `reclaim_notice_tx` now write the row inside the existing tx: a
  `Parse::Quarantined` re-parse rewrites `reason`/`detail` to the CURRENT failure and preserves
  the first-ingest pair ONCE in new nullable `first_reason`/`first_detail`; a `Parse::Pending`
  re-parse stamps only. `last_attempt_at` + `attempts` (nullable, additive migration) make
  attempted-and-failing distinguishable from never-reached. `reprocessed_at` untouched, exactly
  as the design note demands — the member stays in the backlog, now under its TRUE reason (a
  re-run of the ORIGINAL bucket no longer finds it, by design: it belongs to the new reason's
  bucket, which is the honest work list).
- The profile-level population the issue's mechanism section could not see: a held member that
  STILL yields no identity arrives as `Record::Quarantine` and was walked past silently —
  uncounted and unwritten. It now counts as `still_held` and records its attempt via
  `Db::record_reclaim_attempt` keyed (fetch_id, member_path).
- The job result now carries `still held by current reason: …` — a bounded (top-8 + other)
  sample, so a residual that failed for a NEW cause is visible in the job log without querying.
- Test: a_failed_reclaim_records_the_current_failure_on_the_row (ingest/tests/process.rs) —
  both branches, relabeled-stale-reason staging, attempt counting across two runs, backlog
  membership under the new reason.
Kind: observability / data quality
Blocked by: —
Relates to: 76 (the reprocess mechanism), 85 (found here), ADR-0009 (bulk reclaim), 40 (the resolution ledger)
Owner: unassigned
Found by: sdk-vendor, during the issue-85 DE-1.x recovery verification

When a bulk reclaim re-parses a held member and the re-parse **fails again**, the
quarantine row is left completely untouched — it keeps the reason and detail that
were written at first ingest, and `reprocessed_at` stays NULL. The row is
indistinguishable from one the reclaim never attempted, and the actual reason the
re-parse failed is written nowhere.

`Db::reclaim_notice_tx` (store/src/lib.rs) has two `Reclaim::StillHeld` exits:

```rust
Some((id, _)) => match parse {
    Parse::Parsed(parsed) => { /* …insert, flip state, flag reprocessed_at… */ }
    _ => Ok(Reclaim::StillHeld),      // <-- parse failed again: row untouched
},
None => {
    if !self.record_notice_tx(conn, n, parse).await? { … }
    if matches!(parse, Parse::Parsed(_)) { /* …flag… */ }
    else { Ok(Reclaim::StillHeld) }   // <-- same
}
```

The job's counters do report an aggregate (`… reclaimed, N still held, …` in
`job_log.counts_json`), so the *number* survives. What is lost is per-row: which
members failed, and why.

## Why it matters — the observed case

The DE-1.x reclaim (ADR-0009) left **241 of 218,876** notices held. Every one of
those rows still reads:

```
reason = unknown-customization
detail = "no vendored SDK metadata for eforms-de-1.1"   (or -1.2)
```

which is now false — the SDK metadata *was* vendored (issue 75), which is exactly
why the other 218,635 reclaimed. The 241 failed for some other, unrecorded reason.
They are spread over 18 fetches (max 55 in one), so this is per-member, not one bad
package.

The pre-reclaim expectation was ~293 residual `malformed-money` (>2 fraction digits).
The actual residual is 241 rows that claim to be something else entirely. Whether
those are the same members under a stale label, or a different failure, **cannot be
determined from the database** — which is the bug.

## The verification hazard this creates

A check of the form *"did any new quarantine reason bucket appear for this cohort?"*
is **structurally unable to fail**. `StillHeld` never writes a new reason, so the
answer is always "no new bucket", whether the cohort reclaimed perfectly or every
single member failed for a novel reason. During the issue-85 verification this
looked like a clean signal and was not one.

Any future bulk-reclaim sign-off that relies on reason-bucket deltas inherits the
same false confidence. This is the observability half of the same lesson as issue 85:
a check that cannot fail is not a check.

## Fix

On a `StillHeld` outcome, record the *current* failure on the quarantine row rather
than leaving the first-ingest one in place. Minimally: update `reason`/`detail` to
what the re-parse actually returned, and stamp an attempt marker so a re-parse
failure is distinguishable from "never attempted".

Design notes for whoever picks this up:

- **Don't reuse `reprocessed_at` for this.** It means "reclaimed back into the parse
  layer" and is what `quarantine_reason`-driven metrics, the issue-40 resolution
  split, and the reprocess job's own `WHERE reprocessed_at IS NULL` resume predicate
  all key on. Overloading it to mean "attempted" would make a failed member invisible
  to the next reclaim run — silently dropping it from the backlog. A separate
  `last_attempt_at` (and ideally `attempts`) keeps the resume predicate intact.
- **Preserve the original reason somewhere** if the ledger needs first-ingest
  provenance, or make it explicit that `reason` means *current* status. Issue 40's
  ledger and the dashboard's Resolved section both read this column; check both
  before changing its meaning.
- The write must stay inside the existing transaction so a crash can't leave a
  half-updated row (the current `BEGIN IMMEDIATE` / `COMMIT` in `reclaim_notice`).
- Cheap and worth it: have the reprocess job log a bounded sample of distinct
  still-held reasons in its result string, so an operator sees the shape of the
  residual without querying.

## Acceptance

- After a reclaim in which some members fail to re-parse, those rows' `reason` /
  `detail` describe the **re-parse** failure, not the first-ingest one.
- A row the reclaim attempted and failed is distinguishable from one it never
  reached, without consulting job logs.
- The reprocess resume predicate still finds failed members on a re-run (they are
  not accidentally excluded from the backlog).
- A regression test covering the `StillHeld` path on both branches (notice row
  present, and profile-level with no notice row).
- Re-examine the 241 DE-1.x residuals once landed: confirm whether they are the
  expected `malformed-money` cohort or something new.
