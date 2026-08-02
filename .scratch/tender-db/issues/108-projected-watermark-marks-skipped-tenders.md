# 108 — the `projected` watermark marks a notice folded even when its Tender was skipped

Status: open — surfaced while answering the issue-99 re-fold's G2 question. Not urgent; latent until a
skip is not a true no-op, which is exactly what 99 exists for.
Kind: correctness (record accuracy) / observability
Blocked by: —
Relates to: 99 (the case where a skip is NOT a no-op), 58 (the watermark), 105 (the other way a planned
notice can end up marked-but-unfolded), 85

## The defect

Both fold paths mark **every notice in the batch** projected, not the notices whose Tender was actually
written. The comment says so plainly:

```rust
let applied = db.apply_tenders(&projections, now, rebuild).await?;
// "Mark every applied notice as folded into the canonical layer (issue 58)
//  — WHETHER OR NOT ITS TENDER CHANGED — so the next incremental run skips it."
db.mark_projected(&ids).await?;          // apply_plan_batch — every notice in the batch
db.mark_projected(&applied_ids).await?;  // fold_bucket      — every spilled row
```

`projected = 1` therefore means "this notice was *considered* by a fold", not "this notice's content is
in the canonical layer". Those coincide only while `apply_tender_tx`'s early return is a genuine no-op.

## Why it is benign today, and exactly when it stops being

While the projection logic is fixed, a skipped Tender's stored content already equals what the fold
would have written, so "considered" and "folded" are the same claim.

Issue 99 is the case where they diverge. Before the epoch, a projection-logic change made the same chain
yield different content, the chain-unchanged early return skipped it, and the notice was still marked
`projected = 1` — the watermark asserting the new logic had been applied when it had not. That is how
issue 85 could report the eForms-DE 1.x cohort fully projected while 2,185 Tenders were factless shells:
the watermark was telling the truth about what it measured and a falsehood about what everyone read it
as.

The epoch removes the *cause* for logic changes. It does not fix the watermark's semantics, and issue
105 supplies a second route to the same lie: a planned notice whose `parse_state` is no longer
`'parsed'` is never spilled by the pre-pass, never reaches a fold chain, produces no version row — and
is still marked projected.

## Why it matters beyond tidiness

`G2` (`COUNT(tender_versions)` vs `COUNT(notices WHERE parse_state='parsed' AND projected=1)`) is a
production invariant precisely because the two should agree. Every route by which a notice is marked
projected without producing a version row breaks G2 — so the gate cannot distinguish "the fold is
broken" from "the watermark over-claims". Today that ambiguity has to be resolved by hand, by someone
who knows both mechanisms.

## Fix sketch

Mark only what was written: have `apply_tenders` return the notice ids whose Tenders it actually wrote
(it already returns `Applied`; extend it), and mark those. A Tender that early-returns genuinely needs
no re-fold, so its notices can still be marked — but by an explicit "unchanged, verified current" path
rather than by lumping them in with the written ones, so the two cases stay distinguishable.

Cheaper interim: leave the marking as-is and make G2's failure message name both candidates, so whoever
reads a G2 breach is not left deducing the difference under time pressure.

## Note

This is the day's recurring shape rather than a one-off: a record that is accurate about what it
measured and misleading about what it is read as. Same family as `versions_written` being read as a row
count, `tenders folded` being read as tenders written, and `project_golden` green being read as DE-1.x
coverage.
