# 197 — reprocess skip tally counts records, not members: "599262 skipped" for a 4,441-row bucket

Status: needs-triage
Kind: observability wart (job counts / ledger provenance)
Relates to: 180 (the run that surfaced it), 84 (skip-by-policy provenance), 87 (outcomes must sum to the held set)

## What (job 653, 2026-08-14)

The CS drain's job history line reads:

> 79 package(s): 0 reclaimed, 0 still held, 0 already parsed, **599262 skipped by dispatch policy**

The unparsable-xml bucket held **4,441 rows** (one per CS member FILE). 599,262/4,441 ≈ 135:
`tally.skipped` counts text-era **records** the dispatch policy declined, while
`flag_skipped_members` stamps (correctly — verified on the box) the 4,441 member-FILE rows.
The job summary's "outcomes sum to the bucket" contract (issue 87) silently breaks for
text-era packages: the line mixes units, and a reader comparing the summary to the bucket
size concludes the run walked 135× more held work than existed.

Job 654 has the milder twin: "4 reclaimed" counts records; the ledger stamped 2 member rows
(+20 skipped siblings). Both are record counts over a member-row ledger.

## Fix directions (pick one)

- Count declines at the member-FILE level in the walker's tally (dedupe by `member_file()`
  before counting, or count only paths that matched a quarantine row when flagging), or
- Keep record counts but label them: "599262 record(s) of 4441 held member(s) skipped".

Bounded: reporting only — the row stamps themselves are correct (issue-180 verification).
Check `reclaim_package`'s report fields and the `run_reprocess` summary formatting; a unit
test on a two-record declined file should pin members=1 (or the labeled form).
