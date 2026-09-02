# 338 — a no-op projection reports the issue-318 genericness wall as DISABLED, and names a cause that is not true

Status: FIXED 2026-09-02 — found and fixed the same firing, red-first.
Kind: instrument honesty / false alarm
Relates to: 318 (the wall this misreports), 252 (a job that must not claim more
than it did), 278 (the run that surfaced it)
Blocked by: nothing

## What it says

Every `project` job's durable counts line carries a suffix about the issue-318
genericness wall. On 2026-09-02 the paired project behind the first `ghost-census`
run said:

```
0 notices → 0 tenders (0 islands), 0 versions; 0 tenders written, 0 verified
unchanged; issue-318 wall DISABLED this run (key build in flight or interrupted)
— anchor binds took the pre-318 bar
```

Two of those three claims are false. No key build was in flight or interrupted,
and no anchor bind took any bar, because **no bind happened at all**.

## Why

`project_incremental` returns early when nothing is unprojected:

```rust
let changed = db.unprojected_parsed_notice_ids().await?;
if changed.is_empty() {
    return Ok(Report::default());
}
```

so the mention resolver is never opened and `report.wall` stays
`WallCounts::default()` — and `enabled` is a `bool`, whose default is `false`.
The supervisor's suffix then reads that default as "the wall was switched off"
and prints the sentence written for that case, cause and all.

The proof it is the default and not a real disable: `wall_enabled` is resolved
inside `MentionResolver::open`, which logs **in both directions** to
`tender-db.db.diag.log`. That file's last issue-318 line is
`ARMED … 1788334608` — the 09:36 daily fold — and there is no line at all for the
11:46 run that printed DISABLED. The resolver was never opened.

## Why it matters more than a cosmetic wrong word

This suffix exists *specifically* to stop one failure: a prevention that is
switched off reading exactly like a prevention with nothing to do. Its own
comment says so —

> A DISABLED wall on a quiet day would otherwise be invisible: nothing reached
> the gate, so nothing is reported, so a prevention that is switched off reads
> exactly like one with nothing to do. That is the failure this whole instrument
> exists to avoid.

The bug is that same failure inverted: a run where nothing happened reads exactly
like a prevention that is switched off. An operator scanning job rows for the
DISABLED string — which is the intended use — finds alarms with a stated cause
("a key build is in flight or was interrupted") that would send them looking at
`org_match_keys` for a build that never ran.

It is low-severity because it only fires on a run that did nothing: paired
projects behind a job that turned out to have no work, mostly. It is worth fixing
because an instrument that cries wolf on empty runs is one an operator learns to
skip, and this one has exactly one job.

## The fix

`WallCounts` gains `resolved` — true only when the resolver was actually opened
and the wall's availability was decided. `Db::wall_counts` sets it; `Default`
leaves it false. The supervisor's DISABLED branch requires it, so:

| run | before | after |
| --- | --- | --- |
| resolver opened, wall off | DISABLED (correct) | DISABLED (correct) |
| resolver opened, wall on, nothing reached it | silent (correct) | silent |
| resolver never opened (0 notices) | **DISABLED (false)** | silent |

Red-first: the new test fails on the old code with the false DISABLED string, and
the existing "a real disable is loud" case is pinned beside it so the fix cannot
be the other error — silencing the alarm that matters.

## Verified live (2026-09-02, rev `77f853f`)

A no-op project enqueued by hand against the deployed fix, job 606:

```
0 notices → 0 tenders (0 islands), 0 versions; 0 tenders written, 0 verified unchanged
```

No suffix — where the identical run twenty minutes earlier (job 604, rev
`06cb836`) had appended the DISABLED sentence with its untrue cause.
