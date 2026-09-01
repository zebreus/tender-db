# 335 — No measurement can be diffed against its own past

Status: DONE 2026-09-01 — `report_history` landed, ADDITIVELY, with no migration
at all. Both plans this issue proposed were superseded: see "What was actually
built".
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

## Checked, and the cursor risk is smaller than this issue first claimed

I wrote that the cursors were "the real risk in the change". **They are not.**
Enumerated every reader: there are 40 call sites and **all of them go through
`Db::latest_report(kind)`**, including `rehash-cursor` and `reveal-cursor`, plus
one dynamic reader (`admin.rs:320`, the report endpoint) and `report_stamps` for
`/metrics`. Nothing reads the table directly.

So changing the key and rewriting `latest_report` as
`ORDER BY computed_at DESC LIMIT 1` preserves latest-wins **for every caller by
construction** — cursors included. There is no per-caller audit to do.

## What the work actually is

A **schema migration on a live table**, which is the part that needs care:

* `reports` is `kind TEXT PRIMARY KEY`, created via `CREATE TABLE IF NOT EXISTS`,
  so the prod table will not change shape on its own.
* The additive-migration list in `crates/store/src/lib.rs` is `ALTER TABLE ... ADD
  COLUMN` only — it cannot change a primary key, so this needs either a
  `drop_and_recreate`-style rebuild (the helper exists, used by the reset paths)
  or a new-table-and-copy step.
* `report_stamps` does `SELECT kind, computed_at FROM reports` with no grouping.
  With history it would return one row per version, so it needs a
  `GROUP BY kind` / `MAX(computed_at)`. **This is the one caller that genuinely
  changes**, and it feeds `/metrics` report-freshness.
* Retention still needs sizing: the issue-332 census body carries a 200-row
  listing, so ten versions of the widest kinds is the number to check against the
  box before picking a bound.

None of that is large, but a primary-key migration on a table the dashboard and
`/metrics` both read is not a step-3 drive-by. Left ready rather than rushed.

## What was actually built — and neither plan above survived

The cheap plan was to key `reports` on `(kind, computed_at)`. **The codebase said
no, and it was right.** From `MIGRATIONS` in `crates/store/src/lib.rs`:

> Anything beyond ADD COLUMN stays out of scope by policy — the canonical layer is
> rebuildable, and destructive changes recreate from the archive instead.

A primary-key change is exactly what that excludes. Rather than argue the policy
did not apply (`reports` is not the canonical layer, so the stated *rationale*
does not fit — but the *rule* is a rule), the design changed to be fully additive:

```sql
CREATE TABLE IF NOT EXISTS report_history (
    kind TEXT NOT NULL, computed_at INTEGER NOT NULL, body TEXT NOT NULL,
    PRIMARY KEY (kind, computed_at)
) STRICT;
```

`put_report` writes the existing upsert into `reports` **unchanged**, then records
the version and prunes to `REPORT_HISTORY_DEPTH` (10) in the same write. So:

* **no migration**, because the table is new;
* `reports`, `latest_report` and `report_stamps` are untouched, so the forty
  readers — cursors included — cannot change behaviour, which the earlier plan
  could only promise by argument and this one gets by construction;
* `report_stamps` needs no `GROUP BY` after all. That was the one caller the
  earlier plan genuinely broke, and the additive design deletes the problem
  instead of solving it.

The cost is the current body stored twice. At a few dozen kinds that is not worth
a schema change to avoid, and saying so is cheaper than the migration would have
been.

### Reachable, not just stored

`GET /admin/reports/{kind}/previous` serves the prior version with
`versions_held` and `depth`, so "what changed since the last run?" is a request.
That was the actual gap: the data was never the problem, the *absence of a way to
ask* was.

### Pinned

`crates/store/tests/report_history.rs` — nine cases, including two that exist to
protect other people's assumptions rather than this feature: that latest-wins is
unchanged for cursor kinds, and that `report_stamps` still returns one row per
kind rather than one per version.

### What this does not recover

The 102 cases that left issue 311's cohort are still unenumerable. History starts
now. That is the ordinary cost of noticing a gap by falling into it.
