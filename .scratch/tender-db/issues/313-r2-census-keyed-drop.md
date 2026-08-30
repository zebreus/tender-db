# 313 — 1,830 org rows left the E1-keyed set between two r2-censuses, unexplained

Status: EXPLAINED 2026-08-30 (normal campaign merges; the real defect is the
observability gap that hid them) — the two gaps below are the live work
Kind: data quality / identity layer
Relates to: 300 (Stage 2/3), 311, 312

## The observation

Two r2-census runs, 2026-08-29 09:56 UTC (job <470, report body) and
2026-08-30 09:50 UTC (job 489):

| field           | 08-29   | 08-30   | delta  |
|-----------------|---------|---------|--------|
| keyed_e1        | 368,596 | 366,766 | -1,830 |
| orgs_in_groups  |   3,211 |   1,381 | -1,830 |
| groups_ge2      |     634 |     633 |     -1 |
| over_cap_groups |      71 |       3 |    -68 |
| mixed_kind      |      91 |      64 |    -27 |
| keyed_e2        |  44,916 |  44,916 |      0 |

The two -1,830s being identical says 1,830 rows stopped being E1-keyed and
every one of them had been a group member. 68 over-cap groups collapsed.

## RESOLUTION (2026-08-30, same day)

**The drop is ordinary campaign activity, seen through a blind window.** The
merge-plan reports carry their own `computed_at`, and both sit INSIDE the
unobservable window:

- `r2-merge-plan` computed **08-29 11:21 UTC**, `merged_this_run: 243`,
  `residual_of_wet_run: true` (so at least one wet R2 continuation ran after
  the 09:56 baseline census);
- `r3-merge-plan` computed **08-29 17:27 UTC**, `merged_this_run: 296`,
  pool 3,825 (en route to today's 3,529).

That also explains the signature that looked so odd. An R2 merge deletes
loser rows which are, BY CONSTRUCTION, E1-keyed members of a same-country
group of >=2 — so every one decrements `keyed_e1` and `orgs_in_groups` by
exactly 1, which is why the two deltas are identical (-1,830 each) rather
than merely similar. The 311 strips then account for the rest and for the
over-cap collapse (71 -> 3): the fusion class the campaign targeted is
precisely many consortium vehicles sharing one lead member's identifier,
which is what a >8-member group IS.

**My earlier "no merge job ran" was wrong**, and the way it was wrong is the
point: `journalctl` does not log job kinds, and `GET /admin/jobs` silently
caps at 20 rows, so a whole day of job history was invisible from every
surface I reached for. The stored report `computed_at` stamps were the only
witness, and they are incidental.

## What was ruled out along the way (all still true, and now merely context)

- **My own strips.** The 311 campaign stripped 456 (14 pilot + 442 batch)
  and issue 312 restored 280 → 176 net. Worse for the hypothesis: 280 of
  the strips were v4-GUID values on DE/AT rows, and `canonical_key` returns
  None for DE/AT, so those rows were never E1-keyed at all. Ceiling of ~162.
- **A merge or repair job.** None ran in the visible window (see gap below).
- **The Bietergemeinschaft lexicon floor** (3029fc2, landed in-window): the
  census keys on IDENTIFIERS via `canonical_key`; `consortium_name` is a
  NAME predicate the census never calls.
- **The 8-digit checksum tranche** (32c9cd6, 1a34949, in-window): the diffs
  touch `checksum_anchors`/`si_davcna` only. SI is not a census scheme arm,
  and neither commit changed the CZ/FI/DE scoring `census` uses. (CZ:ico,
  CZ:dic-ico, FI:ytunnus, FI:vat ARE hard schemes, so a scoring change there
  WOULD have done this — it just did not happen.)

## The snapshot experiment is NOT needed

It was specified to separate a code change from a data change. Both halves
were then settled locally and for free: `canonical_key`/`canonical_key_flat`
and the whole `Spec::R2Census` handler are BYTE-IDENTICAL between the rev
deployed at the baseline census and today, and the only two statements in
the entire tree that write `organizations.identifier` are the issue-311
strip and the issue-312 restore. No snapshot read, and no team-lead ask,
is required.

## Observability gaps this exposed (fix regardless of the outcome)

1. **`GET /admin/jobs` caps at 20 rows** whatever `limit` says, so the
   08-29 half of the window could not be inspected at all. Either honour
   `limit` (bounded, e.g. ≤200) or say the cap in the response.
2. **`reports` holds one row per kind**, so a census is only ever comparable
   against whatever happened to be read before it was overwritten. The job
   log's `counts` line is the accidental time series that saved this
   comparison — worth making deliberate (a `r2-census-history` append, or
   at minimum a doc note that the counts line IS the history).

## What remains (the actual work)

The data is fine — fewer E1-keyed rows means the merge campaign did its job.
What is broken is that answering "what touched the org layer yesterday?"
took an hour and was answerable only by accident. Fix the two gaps above:
honour `limit` on `GET /admin/jobs` (bounded, e.g. <=200) or state the cap
in the response, and give the census a real history instead of one
overwritable row per kind.
