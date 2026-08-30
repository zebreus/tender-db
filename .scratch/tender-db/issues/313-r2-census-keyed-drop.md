# 313 — 1,830 org rows left the E1-keyed set between two r2-censuses, unexplained

Status: needs-triage (measurement anomaly; no known harm, no known cause)
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

## What is ruled OUT (checked, 2026-08-30)

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

## The decisive experiment (cheap, not yet run)

Run today's binary's r2-census against the **08-28 reflink snapshot**
(`/data/db/snapshots/tender-db-1787903537.db`, held, per issue 169):

- reproduces ~366,766 on old data ⇒ a CODE change moved the keyspace, and
  the diff to hunt is in `crosswalk::canonical_key`/`canonical_key_flat`
  between the deployed revs of 08-29 09:56 and today;
- reproduces ~368,596 ⇒ a DATA change, and the hunt is which job or ingest
  path removed 1,830 keyed rows.

Snapshot reads are the sanctioned path for anything touching data pages
(prod-box-reads doc), so this costs the serving DB nothing.

## Observability gaps this exposed (fix regardless of the outcome)

1. **`GET /admin/jobs` caps at 20 rows** whatever `limit` says, so the
   08-29 half of the window could not be inspected at all. Either honour
   `limit` (bounded, e.g. ≤200) or say the cap in the response.
2. **`reports` holds one row per kind**, so a census is only ever comparable
   against whatever happened to be read before it was overwritten. The job
   log's `counts` line is the accidental time series that saved this
   comparison — worth making deliberate (a `r2-census-history` append, or
   at minimum a doc note that the counts line IS the history).

## Why this is filed rather than fixed now

No harm is visible: fewer E1-keyed rows means fewer auto-merge candidates,
not lost data; `groups_ge2` 634 → 633 satisfies the standing prevention
acceptance (twins not growing), the R3 pool is unchanged at 3,529, and
tripwire 6 reads clear. But a 1,830-row shift in the identity layer that no
change of mine accounts for is exactly the kind of thing that must not sit
unexplained on the board.
