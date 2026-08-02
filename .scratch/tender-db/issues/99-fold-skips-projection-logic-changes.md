# 99 — the incremental fold SKIPS projection-logic changes: chain identity is not a content key

Status: DESIGN — recommendation ready, awaiting team-lead's go. **Blocks 98, the 2,185 issue-85 shells,
and every future projection-logic change (86/48/88).**
Kind: correctness (fold invalidation)
Blocked by: —
Relates to: 85 (the 2,185 factless shells this explains), 98 (which would be a total no-op without it),
58 (the incremental fold), 63, 91/94 (the fold speedups this makes deliverable)

## The defect

`apply_tender_tx` (canonical.rs:2380) early-returns when the Tender's chain of causing notices is
unchanged:

```rust
let stored = if rebuild { Vec::new() } else { self.stored_chain(conn, tender_id).await? };
let keep = stored.iter().zip(&p.versions)
    .take_while(|(a, b)| **a == b.caused_by_notice_id).count();
if keep == stored.len() && keep == p.versions.len() {
    return Ok(applied);          // ← no content is written
}
```

The comment above it states the assumption outright: *"The projection is deterministic, so the sequence
of causing notices is the state key."* That holds only while the projection **logic** is fixed. A
mapping change makes the same chain produce different content, so the chain stops being a state key and
the early-return silently discards the new content.

Two measured consequences:

- **Issue 85** left **2,185 factless shells** — the pure islands whose chains did not change. The
  216,450 that did get facts got them only because merging altered their chains.
- **Issue 98** changes no grouping at all, so no chain changes, so every DE-1.x Tender early-returns:
  **zero parties written, a total no-op.**

It will recur on 86/48/88 and on every future projection-logic change.

## Recommendation: the epoch (option 3). Skip option 2 entirely.

### Team-lead's sketch needs one correction

Changing only the early-return condition **does nothing**. With the chain unchanged,
`keep == stored.len() == p.versions.len()`, so falling through leaves both loops as no-ops:
`for seq in (keep+1..=stored.len())` is empty, and `p.versions.iter().skip(keep)` skips everything.

The fix must force **`keep = 0`** when the epoch is stale:

```rust
let stale = stored_epoch != PROJECTION_EPOCH;
let keep = if stale { 0 } else { computed_keep };
if !stale && keep == stored.len() && keep == p.versions.len() { return Ok(applied); }
```

`keep = 0` then reuses the existing repair machinery unchanged — the delete loop drops every stored
version (and its satellites, via `delete_version`), the write loop rewrites all of them.

### Which is also why option 2 is not the cheaper stopgap

Forcing `keep = 0` **is** the rewrite mechanism. Option 2 — force-deleting the cohort's
`tender_versions` out of band — needs a *new* bulk operation that deletes satellites while preserving
the `tenders` rows (surrogate ids are byte-identity surface) and suppresses `removed` change events.
That is strictly **more** new code than option 3, destructive, and out-of-band. There is no version of
option 2 that is a smaller change than the real fix.

### Cost: small, because three of the four touch points are free

| touch point | cost |
|---|---|
| `ALTER TABLE tenders ADD COLUMN projection_epoch INTEGER NOT NULL DEFAULT 0` | **O(1)** — metadata-only on a STRICT table, already proven for the issue-58 watermark by `alter_add_column_cost.rs`; same shape |
| read the epoch | **free** — `tender_identity`'s non-rebuild probe already runs `SELECT id, source FROM tenders WHERE …`; add a column to the same row |
| write the epoch | **free** — `head_update` already `UPDATE`s `tenders` on the rewrite path; add a column |
| `keep = 0` when stale, plus a `PROJECTION_EPOCH` const | the only genuinely new logic |

Estimate **~half a day including tests** — so no stopgap is warranted; the real fix lands faster than
the workaround.

### Scopeability: yes, with the mechanism stated precisely

The rewrite set is decided by the `projected = 0` marking (the `refold` op), **not** by the epoch. The
epoch only decides, per touched Tender, whether an unchanged chain still gets rewritten. So an epoch
bump + a profile-scoped refold rewrites exactly the cohort — the 8.1M corpus is never read.

After a bump every Tender in the corpus carries a stale epoch. That is harmless and in fact desirable:
untouched Tenders are rewritten only when something else touches them anyway, and their content
genuinely *is* stale with respect to current logic, so this self-heals over time at no extra scan cost.

## Consequences to accept

**Change-feed volume.** An epoch-forced rewrite re-runs `append_version_changes` for every rewritten
version, even where the resulting content is identical. `delete_version` emits nothing, so there are no
spurious `removed` events, but there are `added`/`updated` ones. For 98 that is correct — the content
really does change. It does mean **an epoch bump must always be paired with a scoped refold**: a global
bump plus a full refold would emit change events for all 8.1M Tenders. This is inherent to any
force-rewrite strategy, option 2 included.

**The bump is human discipline.** Nothing detects "the projection logic changed"; someone must bump the
const, and eventually someone will forget. Partial mitigation: `project_golden` already pins fold
output, so a logic change turns it red and forces a deliberate regeneration — put `PROJECTION_EPOCH`
into the golden fixture so regenerating it puts the bump on the same path. Not airtight, but it moves
the reminder onto the road the change already travels.

## Byte-identity gate

Two tests, both of which must be shown to fail without the mechanism:

1. **A forced rewrite is a content no-op.** Project a corpus (epoch N) → snapshot. Bump the epoch
   (test-injectable) and re-project incrementally with **unchanged logic** → snapshot. Assert `tenders`,
   `tender_versions` and every content satellite are **byte-identical**, while `versions_written > 0`
   proves it genuinely rewrote rather than early-returning again. `changes` is expected to grow and is
   excluded from the comparison. Falsification: without the bump, `versions_written` must be 0.
2. **A logic change rewrites only its own surface.** Simulate a mapping change (a test-only alias),
   bump, refold, and assert only the intended satellite moves — facts, lots, results and grouping
   identical. This is issue 98's acceptance invariant in miniature, and the same class as the alias
   allowlist gate.

## Sequencing

99 lands **before** the DE-1.x re-fold; Group 1 and 98 are green and ready but cannot deliver until the
fold applies them. It also carries the 2,185 issue-85 shells on the same mechanism, and it is a
precondition for the 86/48/88 semantics batch.
