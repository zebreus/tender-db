# 50 — SQL analyst surface: time format, schema noise, missing views

Status: needs-verification (app-side); analyst VIEWS deferred (store)
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

DEFERRED — Point 3 (analyst VIEWS: buyers-per-tender, awards-with-buyer,
classifications, dates, amounts, tender's-notices), plus a path-free `v_fetches`
provenance view (from issue 45's fetches exclusion — expose source/package/
period/sha256, NOT the filesystem `path`). All store-side
(crates/store/canonical.rs); build them cheap (indexed / off the current-version
pointer), not another full scan (issue-20/25 pathology).
