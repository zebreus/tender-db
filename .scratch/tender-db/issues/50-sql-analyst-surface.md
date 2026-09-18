# 50 — SQL analyst surface: time format, schema noise, missing views

Status: REOPENED 2026-09-15 — point 2's column-type clause was never fixed: all 105 columns of
all 13 views still publish `"type": "TEXT"` on prod rev `9e082fd` while the served values are
integers. Incomplete fix, not a regression — the 2026-08-17 closure verified the other clauses.

Prior status (history, superseded 2026-09-15):

Status: RESOLVED-VERIFIED (2026-08-17, owner — probed `/v1/sql/schema` on prod rev `62f0e19`). All
three acceptance clauses hold: **time columns unambiguous** (an explicit note — "Unix epoch seconds,
NOT ISO … filter with strftime(col,'unixepoch'); WHERE published_at LIKE '2012%' silently matches
nothing" — plus a per-column flag); **schema clean and documented** (45 allow-listed tables/views, 7
notes covering the allow-list, dialect gaps, row/byte caps and rate limits, 3 worked examples); **the
common questions one view away** (12 analyst views live: v_tenders, v_tender_current, v_awards,
v_lot_results, v_lots, v_organizations, v_tender_buyers, v_tender_amounts, v_tender_dates,
v_tender_classifications, v_tender_notices, v_fetches).

One thing found and fixed while verifying: the "mid-backfill scope" note was STALE, still telling
analysts the v_* layer held only 2026-forward data long after the backfill completed — which would
make a real gap read as expected emptiness. Corrected in this firing to state the full 1993-onward
coverage and to say that an empty v_* year IS a finding.
Severity: MEDIUM (the "easy data inspection" goal clause)

Found by usability audit (2026-07-21). The /v1/sql experience has
avoidable friction for a competent analyst:

1. **Time format trap.** `v_tenders.published_at` in SQL is epoch INTEGER
   (1784239200) while `/docs` promises "Timestamps are ISO 8601" and REST
   returns ISO. `WHERE published_at LIKE '2012%'` silently returns
   nothing; `substr(published_at,1,4)` = "1784". Fix: either expose ISO
   in the views too, or document loudly that SQL time columns are
   epoch-seconds with a `strftime(...,'unixepoch')` example.
2. **/v1/sql/schema noise + gaps.** It leaks ~15 `__turso_internal_seq_*`
   internal tables (hide them), reports every column type as "TEXT" even
   when numeric (misleading), and gives no relationships / column notes /
   enum vocabularies (role strings like 'Procedure-Buyer', notice_subtype,
   decision). Add per-table/column docs, FK hints, enum value lists, and
   a couple of worked example queries.
3. **Missing analyst views.** No `v_` view for buyers-per-tender,
   awards-with-buyer, classifications, dates, amounts, or a tender's
   notices — you must reverse-engineer undocumented base tables
   (tender_version_parties, v_tender_current) and magic role strings. Add
   analyst-facing views for the common questions.
4. **Doc the mid-backfill state.** The canonical `v_*` layer currently
   reflects only PROJECTED tenders (2026-only until job 5 projects the
   backfilled history) — a `since`/scope caveat in /docs and
   /v1/sql/schema notes so "buyers in 2012" doesn't silently return
   nothing pre-projection.

Note: the flagship-view aggregation cost (v_tenders 24s / notices count
>40s) is the same scan pathology tracked in the issue-20 family + the
issue-25 pointer — the analyst-view additions here should be built cheap
(indexed / off the current-version pointer), not another full scan.

Acceptance: SQL time columns are unambiguous; schema is clean +
documented with enums; the common analyst questions are one view away;
mid-backfill scope is documented.

## Progress (api-polish, 2026-07-23)
App-side parts done (sql.rs + docs.rs):
- Point 1 (time trap): every timestamp column carries an epoch-seconds note with
  a strftime example, plus a top-level schema note and a /docs bullet.
- Point 2 (schema noise + gaps): __turso_internal_seq_* are already hidden by
  the issue-45 allow-list (only allow-listed objects are listed); added per-table
  descriptions, per-column notes, enum vocabularies (parse_state, scheme, role,
  decision, notice_subtype, provisional), and an `examples` array of worked
  queries to /v1/sql/schema.
- Point 4 (mid-backfill): scope caveat in the schema notes and /docs (v_* is
  2026-forward until the historical backfill projects; notice_*/quarantine hold
  the full history).
Test: `the_schema_documents_time_format_and_enums`.

Point 3 — DONE (store lane cleared after wal-fix landed). Added 7 views to
canonical.rs, all off the maintained `current_seq` pointer (never MAX(seq)):
v_tender_buyers, v_awards (winner + representative buyer, no fan-out),
v_tender_classifications, v_tender_amounts, v_tender_dates, v_tender_notices,
and the path-free v_fetches (source/kind/period/url/sha256/bytes/fetched_at,
NOT `path`). All allow-listed in /v1/sql + noted in the schema; v_fetches
re-added to ALLOWED (issue 45). Added tender_version_parties_version index
(parties was the lone satellite lacking a (tender_id, seq) index).

turso-planner finding (verified via EXPLAIN QUERY PLAN): turso does NOT push a
WHERE predicate through a view — a filtered `SELECT * FROM v_tender_buyers
WHERE tender_id=?` materialises the view and scans the satellite, EXACTLY like
the pre-existing v_tenders/v_lots. So the "cheap" guarantee these deliver is the
one that matters: no MAX(seq) aggregation (the issue-25 pathology) — they read
current_seq, same as v_tenders. A filtered scan is bounded by the 10s SQL cap on
the isolated runtime (issue 17), unlike the dashboard path issue-25 fixed. The
_version indexes make the seek reachable once turso has row stats (ANALYZE).
Tests: the_analyst_views_answer (e2e), analyst_views_read_the_current_pointer_not_a_max_aggregation.

## Comments

### 2026-09-15 — API/data-quality review fan-out: INCOMPLETE FIX — point 2's "every column type is TEXT" is still live for all 13 views on prod rev `9e082fd`

This issue is closed RESOLVED-VERIFIED (2026-08-17), but the column-type clause of point 2
("reports every column type as 'TEXT' even when numeric (misleading)") was never addressed. The
2026-07-23 progress note for point 2 lists hidden internals, per-table descriptions, per-column
notes, enum vocabularies and worked examples — not the `type` field. The base tables have real
types today; the views do not. Reproduced literally against the live surface on 2026-09-14/15.

**Evidence (literal commands).**

```
curl -s https://tenders.zebreus.click/v1/sql/schema | python3 -c "import json,sys; d=json.load(sys.stdin); print(sorted({c['type'] for t in d['tables'] if t['type']=='view' for c in t['columns']}), sum(len(t['columns']) for t in d['tables'] if t['type']=='view'))"
  -> ['TEXT'] 105

ssh -o BatchMode=yes -o StrictHostKeyChecking=no root@zebreus.click 'echo "SELECT * FROM v_fetches LIMIT 1" | /root/sq.sh'
  -> [1,"ted","daily","2026-00136","https://ted.europa.eu/packages/daily/202600136","0b79…",19809161,1784489994]

ssh ... 'echo "SELECT typeof(id), typeof(source), typeof(bytes), typeof(fetched_at) FROM v_fetches LIMIT 1" | /root/sq.sh'
  -> ["integer","text","integer","integer"]

ssh ... 'echo "SELECT id, tender_id, notice_id, typeof(id), typeof(tender_id), typeof(notice_id) FROM v_lot_results LIMIT 1" | /root/sq.sh'
  -> 4, 3, 24135424, "integer","integer","integer"

ssh ... 'echo "SELECT typeof(id), typeof(published_at) FROM v_tenders LIMIT 1" | /root/sq.sh'
  -> integer, integer
```

**What the schema publishes vs. what the engine serves.**

| object class | objects | columns | `type` published by /v1/sql/schema |
|---|---:|---:|---|
| views (12 `v_*` + `notice_withheld_fields`) | 13 | 105 | `TEXT` for 105 of 105, `notnull` false, `pk` false — no exceptions |
| base tables | 35 | 239 | real `INTEGER`/`REAL`/`TEXT` with real pk flags (`tenders.id` INTEGER, pk true) |

| column | schema says | engine `typeof()` | sample value |
|---|---|---|---:|
| `v_fetches.id` | TEXT | integer | 1 |
| `v_fetches.bytes` | TEXT | integer | 19809161 |
| `v_fetches.fetched_at` | TEXT | integer | 1784489994 |
| `v_lot_results.id` | TEXT | integer | 4 |
| `v_lot_results.tender_id` | TEXT | integer | 3 |
| `v_lot_results.notice_id` | TEXT | integer | 24135424 |
| `v_tenders.published_at` | TEXT | integer | (epoch seconds) |

The same three columns are declared INTEGER on the underlying base tables in the *same* schema
response (`lot_results.id/tender_id/notice_id`, `tenders.id`, `tender_versions.seq/published_at`),
so the response contradicts itself. Mechanism is ours: `crates/app/src/v1/sql.rs:757-762` runs
`PRAGMA table_info("{name}")` and copies column 2 verbatim into `"type"` via `store_text(&row, 2)`;
turso 0.7.2 (Cargo.lock) returns `TEXT` for every view column. Stock SQLite 3.45.1 fed the identical
DDL (`crates/store/src/lib.rs:122-132`, `crates/store/src/canonical.rs:1034-1036`) reports
`INTEGER` for pass-through view columns and `''` for expression columns — so turso is inventing
`TEXT` where it should report unknown, and the endpoint republishes the invention. Partial
mitigation, which is why this is low and not medium: 12 of the 105 view columns carry a note and 4
of those say "Unix epoch seconds" (`v_fetches.fetched_at`, `v_tenders.published_at` among them), so
those 4 disclose integer-ness in prose; for the other ~93 the wrong `type` is the only signal.

**Judge's ruling (why it is ours, and why this issue):**

> Premise confirmed against the live rev 9e082fd: GET /v1/sql/schema reports type 'TEXT' for all 105
> columns of all 13 views (v_fetches: id/source/kind/period/url/sha256/bytes/fetched_at all TEXT),
> while base tables carry real declared types (tenders.id INTEGER pk). Two bounded reads through
> /root/sq.sh show the served storage classes are integers: SELECT
> typeof(id),typeof(bytes),typeof(fetched_at),typeof(source) FROM v_fetches LIMIT 1 ->
> integer,integer,integer,text; SELECT typeof(id),typeof(published_at) FROM v_tenders LIMIT 1 ->
> integer,integer. Mechanism is in this system's own code: crates/app/src/v1/sql.rs fn schema (line
> ~757-762) copies PRAGMA table_info column 2 verbatim into "type" for tables and views alike; turso
> 0.7.2 (Cargo.lock) returns TEXT for every view column. Nothing mitigates it: the schema's notes
> array (sql.rs ~780-820) has no caveat about view column types, no column note flags it, the
> OpenAPI response is untyped (additionalProperties: true) but its description sells the endpoint as
> discovery of the surface with the views as "the main entry points", and the only schema test
> (crates/app/tests/sql.rs:499 the_schema_documents_time_format_and_enums) asserts nothing about the
> type field. This is not a publisher-published fact; it is a value tender-db's own endpoint
> publishes wrongly. Board check: issue 50 item 2 recorded this exact symptom ("reports every column
> type as TEXT even when numeric (misleading)") but its progress note for point 2 lists only hidden
> internals, descriptions, enums and examples, and the RESOLVED-VERIFIED closure (2026-08-17) never
> claims the type field was fixed — the sub-item was dropped, not resolved, so the closed issue does
> not cover it and this is not a regression either. No other issue on the board (grep for
> table_info/declared type/column type/TEXT across 384 files: 07, 43, 45, 50, 204, 240, 370) tracks
> the view-type value. Actionable by a maintainer: derive the type from the underlying column of each
> view's definition, hand-curate the 13 views, emit null for views, or at minimum add a schema note
> saying view types are unreliable and to consult the base table. Severity low: JSON results carry
> their real types so a client that only reads rows is unaffected; only a client generating typed
> bindings or validation from the schema is misled, and only on the views.

Scope note on the judge's own honesty: the fan-out's engine probes covered 2 of 13 views directly
(`v_fetches`, `v_lot_results`) plus schema-side base-vs-view comparison for `v_tenders`; a third
probe (`v_organizations`) was blocked by a permission classifier, not by the box. The `TEXT`-for-all
count itself is a full census of all 105 columns from the schema response.

**To close:** in `crates/app/src/v1/sql.rs` fn `schema`, stop copying `PRAGMA table_info`'s type for
views — resolve each view column to its underlying base column's declared type (or emit `null`/omit
the field for views) — and extend `the_schema_documents_time_format_and_enums`
(`crates/app/tests/sql.rs:499`) to assert a view column's type, since today no test touches the
field; the cheap interim is a schema note saying view `type` is unreliable, consult the base table.

## Verify

    curl -s https://tenders.zebreus.click/v1/sql/schema | python3 -c "import sys,json; ts=json.load(sys.stdin)['tables']; v=[t for t in ts if t['name']=='v_tenders'][0]; print([(c['name'],c['type']) for c in v['columns'][:3]])"

- **done**: `[('id', 'INTEGER'), ('source', 'TEXT'), ('procedure_key', 'TEXT')]` — a view column carries its base column's declared type, or `null`, or a note says view types are unreliable
- **open**: `[('id', 'TEXT'), ('source', 'TEXT'), ('procedure_key', 'TEXT')]` — every view column is `TEXT` (read 2026-09-18; the base table reads `INTEGER` for `id`)
