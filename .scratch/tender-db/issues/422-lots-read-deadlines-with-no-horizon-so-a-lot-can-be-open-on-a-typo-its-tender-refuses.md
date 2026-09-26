# 422 — `/v1/lots` reads deadlines with no horizon, so a lot can be open on a typo its tender refuses

Status: ready-for-agent — **BUILT 2026-09-26 14:0x UTC** (see the foot): decision taken — the horizon
applies to BOTH the lot row and the lots `status` EXISTS; tests red-then-green by mutation; gate, deploy
and the before/after latency read follow. Was: filed 2026-09-26 from issue 171's rule-12 unit.
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

## BUILT 2026-09-26 — one window on both halves of the pair

**Decision**: apply the horizon to both, as the default leaning said. A lot's display and its `status`
answer are the pair issue 389 pins; changing one without the other re-creates the split 366 closed on
the tender side. Keeping both horizon-free and documenting it would leave a lot `open` on a date its
own tender row calls a typo — a disagreement inside one response of `/v1/tenders/{id}`.

- **The lot row** (`read.rs` `summarise`): the `tender_versions` PK seek that already fetched
  `original_lang` now also fetches `published_at`; a deadline row beyond `published_at +
  DEADLINE_HORIZON_SECS` is skipped like a pre-floor one, before the lot/procedure split — so a lot's
  own typo leaves it inheriting the procedure's real date, and a procedure's typo is not inherited.
- **The lots `status` EXISTS** (`version_predicates`, the `deadline_col = None` arm — lots only; every
  tenders caller reads the head column): `AND d.utc_seconds - (SELECT pv.published_at FROM
  tender_versions pv WHERE pv.tender_id = {tid} AND pv.seq = {seq}) <= DEADLINE_HORIZON_SECS`, after
  the `> now` term, so the seek is paid only by a date already in the future. `NOT EXISTS` (closed) is
  the same predicate negated, so a lot on a typo alone reads closed.
- **Test**: `lot_deadline_scope.rs::a_lot_is_never_open_on_a_deadline_beyond_the_horizon` — the typo
  alone (row null, not open, closed) and a lot's own typo beside a real procedure date (inherits it,
  open by it). **Mutation-checked**: neutralising the EXISTS term (`<= H OR 1`) fails the test on
  exactly "the filter must refuse the date the row refuses".
- `/docs#caveats` Dates: a Lot's deadline and `/v1/lots?status=` apply the tender's window.

**Cost to watch**: in the lot STREAM shape `{seq}` is `LOT_SEQ` (a `MAX(seq)` subquery), so the new term
adds a MAX seek + PK seek per future-dated deadline row. Baseline before deploy (2026-09-26 13:5x,
two reads each): `status=open&limit=100` 2.3 / 2.0 s, `status=open&country=DE&limit=100` 5.8 (cold) /
0.9 s, `status=closed&limit=100` 0.57 / 0.58 s, `status=open&tender=8436333` 0.64 / 0.50 s. The
`status=open&cpv=45&limit=100` 503 is pre-existing and is issue 423's, not this change's.