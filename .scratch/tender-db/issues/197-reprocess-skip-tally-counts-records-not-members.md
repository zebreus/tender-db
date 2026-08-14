# 197 — reprocess skip tally counts records, not members: "599262 skipped" for a 4,441-row bucket

Status: needs-triage
Kind: observability wart (job counts / ledger provenance)
Relates to: 180 (the run that surfaced it), 84 (skip-by-policy provenance), 87 (outcomes must sum to the held set)

## What (job 653, 2026-08-14)

The CS drain's job history line reads:

> 79 package(s): 0 reclaimed, 0 still held, 0 already parsed, **599262 skipped by dispatch policy**

Issue 180 recorded the bucket as **4,441 CS rows**; 599,262/4,441 ≈ 135 ≈ records-per-CS-file.
But the obvious "tally counts records" explanation does NOT survive the code: the walker's
`skipped` increments once per DECLINED MEMBER (`Disposition::Skipped`, process.rs:246-251),
and only members present in the held set reach dispatch at all (the issue-77 early-return),
while `quarantine_held_member_files` DISTINCTs on `member_file()` — so a 599,262 tally
implies ~599k distinct held member paths across those 79 packages, i.e. the bucket may have
really held ~600k per-record rows ("unknown token" is a parse-level, per-record failure)
and issue 180's 4,441 was the FILE count, not the row count. The two readouts cannot both
be right — and whichever is wrong, the job summary and the issue ledger disagree with each
other today.

Job 654's milder twin: "4 reclaimed" counts records; the quarantine ledger stamped 2 member
rows (+20 skipped siblings).

## Next (one bounded query each)

- `SELECT count(*), count(DISTINCT member_path) FROM quarantine WHERE reason='unparsable-xml'
  AND skipped_at BETWEEN 1786656055 AND 1786657368` — pins rows-vs-files for job 653.
- Same split for the historical bucket definition used in issue 180's 4,441 figure.
- Then decide: fix the tally's unit, fix the summary label, or fix issue 180's record —
  and make the summary line state its unit either way.

Row stamps themselves verified correct on the box (issue-180 pass: bucket empty, 0 still
held). Reporting/records only.
