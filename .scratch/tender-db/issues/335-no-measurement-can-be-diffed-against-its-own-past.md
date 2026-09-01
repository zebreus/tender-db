# 335 — No measurement can be diffed against its own past

Status: NEEDS-TRIAGE 2026-09-01 — gap identified from a concrete loss (see
"How it bit"), fix shape sketched, not sized.
Kind: capability (observability / measurement discipline)
Relates to: 311 (where it bit), 230 (data-quality windows), 326/332/333 (censuses
whose before/after comparisons are the whole point)
Blocked by: nothing

## The gap

`Db::put_report` stores one row per report kind:

```sql
INSERT INTO reports(kind, computed_at, body) VALUES(?, ?, ?)
  ON CONFLICT(kind) DO UPDATE SET computed_at = ..., body = ...
```

So **every re-run destroys the previous measurement.** There is no history, and
therefore no way to answer "what changed since the last run?" for any census,
packet, plan or probe in the system — the exact question most of them exist to
support.

## How it bit

2026-09-01, issue 311. The cross-border review cohort was re-cut after this
session's repairs: **589 → 487 components, 102 gone.** The obvious and necessary
follow-up is *which* 102 and why — for a review queue that is the audit trail. It
is unrecoverable: only the scalar count had been read before the re-run, and the
prior membership was overwritten.

The same shape nearly bit twice more the same day. Issue 333's fix was only
verifiable because a baseline was copied out by hand first (`generic_orgs`
5,204,927 → 5,265,991, which is how the "latent" claim was caught as wrong). That
comparison worked by luck and diligence, not by design.

## Why it matters beyond convenience

Several standing decisions in this project rest on before/after comparisons:

* tripwires assert a counter has not moved (issue 325 step 5, issue 327);
* the org-merge-health baseline exists specifically to be compared (issue 300
  Stage 0);
* every repair's `expect_rows` parity gate is a comparison.

All of them currently work by an agent copying numbers into a commit message or an
issue file. That is a durable record only as long as someone remembers to write
it, and this issue exists because that failed once already.

## Fix shape — cheap version first

**Do not** build a general time-series store. The minimum that closes the audit
gap:

1. Drop the `ON CONFLICT(kind)` uniqueness and key reports on
   `(kind, computed_at)`, keeping the existing "latest of kind" read as
   `ORDER BY computed_at DESC LIMIT 1` so nothing downstream changes.
2. Retain a bounded number per kind (say 10) and prune older ones in the same
   write, so the table cannot grow without limit — the same discipline
   `ops/check.sh` applies to test binaries.
3. Add nothing else. No diffing UI, no API surface. Being able to fetch the
   previous body is the whole requirement; a reader or a later job can diff.

## What must be checked before doing it

* **Who reads `reports`?** The dashboard, the admin report endpoint, and several
  jobs that read their own last output (`rehash-cursor`, `reveal-cursor` are
  stored as reports and are *cursors*, not measurements — those must keep
  latest-wins semantics or a resumed job could read a stale cursor). This is the
  real risk in the change and needs enumerating first.
* Report bodies can be large (the issue-332 census listing is 200 rows). Ten
  versions of the biggest kinds needs sizing against the storage the box has.
