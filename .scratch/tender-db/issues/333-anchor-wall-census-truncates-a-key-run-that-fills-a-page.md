# 333 — `anchor_wall_census` truncates a key run that fills a whole page

Status: FIXED 2026-09-01 — grouped walk landed, regression test in place.
Pre-change prod baseline recorded below and compared after, so the fix is shown
not to have moved a live number.
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

## Fixed

`anchor_wall_census` now pages with `GROUP BY key ... LIMIT`, so the limit applies
to groups rather than rows and a run of any length is counted whole — the same
route issue 332's census took, and simpler than the E3 scan's special case
because there is no special case to write. Carriers for over-cap keys are then
fetched with pure equality on both indexed columns, bounded by `per_group` (the
probe budget), which is all the walk ever looked at anyway.

The page size is a parameter rather than a private const, so the failure is
testable: `a_key_run_longer_than_a_page_is_counted_whole` drives a page of 3
across a run of 8. Under the old code that read **3** carriers and reported
`generic_orgs = 3`; it now reports 8.

### Baseline, and why it is recorded

The reading was latent-wrong, not live-wrong, so replacing the walk must not
change any number. Captured from the stored report before the change:

```
keys_walked   = 3,481,572     probed        = 3,553,602
generic_keys  = 55,312        anchored      = 15,809
generic_orgs  = 5,204,927     anchored_hard =  6,227
                              anchored_soft =  9,582
                              soft_slots    = 10,109
```

`anchored_soft = 9,582` is the figure issue 318 acted on, so it is the one that
matters most. Compared against a fresh run after the fix.
