# 422 — `/v1/lots` reads deadlines with no horizon, so a lot can be open on a typo its tender refuses

Status: ready-for-agent — filed 2026-09-26 from issue 171's rule-12 unit. Small, code-only; the
decision (below) is the first step and is mine to take when this is picked up.
Kind: API consistency (the lots status filter and row disagree with the tender election)
Relates to: 366 (the horizon), 375 (one election, no second transcription), 389 (the lots
status ≡ row pairing), 171 (the floor, which the lots row now applies)

## What

The tender head's submission deadline is elected inside a window: not before
`DEADLINE_FLOOR_SECS` (1990-01-01, issue 171) and not more than `DEADLINE_HORIZON_SECS` (ten
years) past the version's publication (issue 366). `head_deadline`, the list/detail read pick and
the backfill all apply both edges, pinned by `head_election_agreement.rs` and `deadline_backfill.rs`.

The lots path applies only the floor, as of issue 171's commit:

- the lot ROW deadline (`read.rs` `summarise`, the `tender_version_dates` loop) skips pre-1990
  rows but not beyond-horizon ones;
- the lots `status=open` filter (`version_predicates`' EXISTS, `d.utc_seconds > now`) has
  neither edge. The floor cannot change a `> now` answer, so that half is consistent; the
  horizon can.

So a lot whose deadline is tender 3323836's shape — `3005-07-06` beside or instead of a real
date — is returned by `/v1/lots?status=open` and serves the year-3005 date, while its tender
reads closed with the real date (or none). That is exactly the display/filter split issue 366
closed on the tender side.

## Size

Unmeasured. The weekly data-quality report's section 10 dates listing (2026-09-20) shows no
`submission_deadline` cluster in its top 40 — the far-future mass is `duration_end` and
`participation_deadline`, which feed no status — so the population is likely small. A count
needs a scan of `tender_version_dates` by value (no index on `utc_seconds`), i.e. a windowed
probe or the weekly run.

## The decision this needs

Whether the lots status EXISTS gains the horizon (it would need `tender_versions.published_at`
in the EXISTS — one PK seek per candidate, measure the `status=open` lots shapes before and
after) or whether the lot row and filter both stay horizon-free and `/docs#caveats` says so.
Default leaning: apply it to both, because the tender side already paid for "one rule", and
measure the EXISTS cost first.

## Verify

    grep -c 'DEADLINE_HORIZON_SECS' crates/store/src/read.rs

- **done**: 3 or more — the tender pick's one use plus the lot row's and the lots EXISTS'
- **open**: 1 (read 2026-09-26 after issue 171's commit: `1`)

A source read, free.
