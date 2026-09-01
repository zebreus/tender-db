# 333 — `anchor_wall_census` truncates a key run that fills a whole page

Status: DIAGNOSED 2026-09-01 — latent, not live. Root cause understood, fix
modelled on code already in the tree.
Kind: correctness (read-path measurement)
Relates to: 318 (which added the census), 300 Stage 4 (the E3 scan, which gets
this right and is the model), 332 (whose own test found the pattern)
Blocked by: nothing

## The defect

`anchor_wall_census` walks `org_match_keys` in pages ordered by `(key, org_id)`,
groups each page into key runs, and drops the last run because it may be split
across the page boundary:

```rust
if n == WINDOW && groups.len() > 1 {
    groups.pop();
}
after = groups.last().expect("a non-empty page has a last group").0.clone();
```

The guard is `groups.len() > 1`. **If a single key run fills the entire page,
`groups.len() == 1`, nothing is popped — and the truncated run is then treated as
complete, with `after` set past it.** The remaining carriers of that key are
never read.

So a key with more than `WINDOW` (20,000) carriers is counted as having exactly
20,000, and every carrier beyond that is invisible. The error is in the direction
that hides **the widest keys**, which for a genericness census is the entire
population of interest.

## How it was found, and why it is worth fixing while latent

Issue 332's census copied this idiom, and a test driving a window of 3 across a
run of 8 counted **3 carriers** and dropped the key below the cap entirely. The
same test shape would fail here.

**It cannot bite today.** The widest n2 key measured on prod is `siemens §ag` at
1,238 carriers (issue 331, job 570) — far under 20,000. So this is a latent
defect, not a live wrong number, and no past anchor-wall reading needs to be
retracted. It is worth fixing anyway: the margin is one growth spurt wide, and
the failure is silent.

## The fix is already in the tree

`scan_org_match_keys` handles exactly this case, and its handling is the model:

```rust
if groups.len() == 1 {
    let (key, _) = groups.pop().expect("one group");
    // ...COUNT(*) FROM org_match_keys WHERE key_kind = ? AND key = ?
```

— when one run fills the page, it takes the count from a dedicated query instead
of from the truncated page. Issue 332's census took a different route (`GROUP BY
key` with `LIMIT`, so the limit applies to groups rather than rows, which is
correct for a run of any length and has no stitching to get wrong). Either shape
works here; the `GROUP BY` one needs no special case at all.

## Acceptance

A test driving a small window across a run longer than it, asserting the key's
full carrier count — the test that already exists in
`crates/store/tests/generic_statistic_census.rs` as
`a_key_run_split_across_a_window_boundary_is_still_counted_whole`, transplanted.
