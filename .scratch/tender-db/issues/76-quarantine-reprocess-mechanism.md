# 76 — the quarantine reprocess mechanism (the reclaim linchpin)

Status: DONE — deployed + prod-validated 2026-07-29 (reprocess mechanism live; reclaimed SDK 1.0=1,634, 1.7=299,829, 1.10=254,523; resume + isolation + in-place write all validated). See ADR-0009 for the reclaim-all-then-rebuild strategy.
Kind: mechanism / completeness
Blocked by: —
Unblocks: 71 (SDK cohort, ~1.2M), 72 (OC, 577K parse-level), and the bookkeeping for 73
Surfaced by: the 2026-07-29 reclaim-program audit (issues 71/72/73)

## Why

The whole quarantine-reclaim program (issues 35/36/41/71/72/73) ended in "reprocess
after deploy", but **no reprocess driver was ever built**. Traced this session:
no `reprocess` Spec/kind/bin, `reprocessed_at` was never set by production code, and
`/v1/sql` is read-only. Two quarantine classes, different reclaim behaviour:

- **Parse-level** (a `notices` row exists in state `quarantined`, empty of parsed
  values): OC (issue 72), the SDK cohort (issue 71). A plain `process` re-run is a
  **no-op** — `insert_notice_row`'s `INSERT OR IGNORE` short-circuits, so the
  re-parse never runs. These were **unreclaimable** without a real mechanism.
- **Profile-level** (failed before an identity existed, NO `notices` row): the
  2008-05 DTD bucket (issue 73). A `process` re-run reclaims the notices, but the
  stale quarantine rows are never flagged, so the ledger stays wrong.

## What was built

A first-class, isolated, bounded, resumable reprocess job.

- **Admin trigger:** `POST /admin/jobs {"kind":"reprocess","reason":"…","detail_like":"…","profile":"…"}`
  (`reason` required; `detail_like` is a SQL LIKE pattern; `profile` exact). Enqueues
  `Spec::Reprocess` + a trailing `Spec::Project {rebuild:false}` to fold what it reclaims.
- **`store::Db::reclaim_notice`** (the review-critical write): for a member whose
  earlier ingest quarantined it, writes the parsed layer IN PLACE when it now parses.
  Parse-level → `insert_parsed` + flip `parse_state` to `parsed` (clearing `projected`
  so the projection re-folds it) + fill resolved instants + set `reprocessed_at`;
  profile-level → the ordinary ingest write, then flag the held row by
  `(fetch_id, member_path)`. All in ONE transaction, so a crash leaves the member
  held and a re-run redoes it. Never re-touches an already-`parsed` notice (no
  double-write). The original `ingested_at` is preserved (the tender fold orders by
  `published_at`, never `ingested_at`, so a reclaimed fold is byte-identical to a
  fresh ingest).
- **`ingest::process::reclaim_package`** walks each affected archived package via the
  SAME producer as `process_package` (`spawn_record_producer`) — so a reclaimed
  member's parse is byte-identical to a first ingest — and streams one record at a
  time (peak RAM flat).
- **`store::Db::quarantine_reclaim_packages`** is the resumable work list: distinct
  held packages of the bucket (`reprocessed_at IS NULL`) past a `fetch_id` cursor, so
  the result is bounded by package count and reclaimed packages fall out of a re-query.
- **Isolation / resume / WAL:** runs on the supervisor's dedicated worker runtime
  (issue 61), counts as a `heavy_write` (coverage change-gate), records a per-package
  `fetch_id` resume cursor, and TRUNCATE-checkpoints between packages — exactly the
  `run_process` disciplines.

## Tests (green)

- store: parse-level in-place reclaim (state/values/`projected`/instants/`reprocessed_at`,
  original `ingested_at` preserved, in the projection change-set) + idempotent no-op
  re-run; profile-level record-and-flag (matched by member, content-hash-agnostic) +
  still-held; the resumable held-package work list.
- ingest: end-to-end `reclaim_package` over the era-ladder fixture — a no-op over a
  fully-parsed package (0 reclaimed, nothing corrupted), then a rewound member is
  re-parsed and reclaimed in place while the others are untouched.
- Full store/ingest/app suites pass, incl. the byte-identity projection gates
  (project_golden/equivalence/resume/fold_source).

## Ops (team-lead runs after review + deploy)

- OC (issue 72): `POST /admin/jobs {"kind":"reprocess","reason":"unknown-field-code","detail_like":"%: OC"}`
- SDK (issue 71, after the SDK vendoring ships): `reason":"unknown-customization","detail_like":"%eforms-sdk-1.7%"` per version (or the whole reason bucket once all vendored).
- 2008-05 DTD (issue 73): now reclaimable cleanly via the same endpoint
  (`reason":"unparsable-xml","detail_like":"XML with DTD detected"`) instead of a bare
  `process` re-run — this one also flags the stale rows.
