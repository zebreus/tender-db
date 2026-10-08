The SQL surface needs no allow-list or filterability change. The real work is the migration, tests that won't see the new columns, docs that describe the tender headline value as "the tender's highest amount", and a backfill of about 10.5 h. I edited no files and ran nothing on the box. The ALTER-survives-rebuild point below was read in the turso 0.7.2 source, not run.

## 1. How views are created, the allow-list and NOT FILTERABLE

**How views are (re)created at open**
- `canonical::SCHEMA` (crates/store/src/canonical.rs:100–1215) runs on every `Db::open`, at crates/store/src/lib.rs:1122. `migrate()` runs after it, at lib.rs:1127.
- Every view is a `DROP VIEW IF EXISTS` plus `CREATE VIEW` pair. `v_lots` is at canonical.rs:1086–1096. Its columns today are `l.id, l.tender_id, l.lot_key, vl.kind, vl.seq` and a correlated title subquery, joined through `v_tender_current`.
- So changing the view text is the whole deploy story for the view. It costs O(1) at each boot.
- On an existing database the view is created before the ALTER adds the column. Turso accepts that: `an_existing_database_gains_the_winner_is_buyer_column` (crates/store/tests/satellite_column_migration.rs:96–151) pins it for `v_lot_results.winner_is_buyer`. Copy that test for `tender_version_lots` and `v_lots`.
- If the value comes from a stored column, `v_lots` only projects it (`vl.value_*` or `l.<head col>`). Do not compute it in a subquery inside the view: that is the issue-239 `v_tenders` title mistake.

**The allow-list is by table name only**
- `ALLOWED` (crates/app/src/v1/sql.rs:139–206) lists names: `v_lots` at :142, `tender_version_lots` at :170, `lots` at :158, `tenders` at :156.
- `classify` (sql.rs:1254–1298) walks table references, never columns. A new column on any allow-listed table or view is readable with no change.

**NOT FILTERABLE is by view name**
- `unfilterable_view` (sql.rs:1376–1378) covers every `v_*` view except `v_fetches`. `filtered_view` (sql.rs:1335) refuses any SELECT that has a WHERE on, or a join to, a view (issue 239). Nothing there changes.
- The consequence: `v_lots` cannot serve a value filter. `SELECT … FROM v_lots WHERE value_eur_cents > X` is refused with a 400.
- `ORDER BY value_eur_cents DESC LIMIT n` with no WHERE is accepted. But it builds the whole view, with the per-row title subquery, so expect a 408. This is the tripwire in crates/store/tests/view_pushdown_probe.rs:52–72.
- So "index speed in SQL" means the base tables. The `v_lots` columns are only for unfiltered peeks and whole-corpus aggregates.

**Text that would change, and the tests that pin it**
- The `v_lots` table note (sql.rs:820–822) says `…NOT FILTERABLE, same as v_tenders (measured: \`WHERE tender_id = ?\` exceeds the time limit); join \`lots\` to \`tender_version_lots\` instead.`
  - `filtered_view_message` (sql.rs:1356–1368) serves the text after "NOT FILTERABLE" as the 400's guidance.
  - The test at sql.rs:2646 asserts `e.contains("join \`lots\` to \`tender_version_lots\`")`. Keep that phrase or update the test.
  - The test at sql.rs:2444–2477 requires every unfilterable view's guidance to name a backticked base table.
  - Better guidance would use the issue-421 CROSS JOIN order: `tenders t CROSS JOIN tender_version_lots vl ON vl.tender_id = t.id AND vl.seq = t.current_seq`.
- Column notes, `COLUMN_NOTES` (sql.rs:917–1024):
  - The `"*"` entry for `eur_cents` (:942–948) matches by exact column name, so a column named `value_eur_cents` gets no note unless one is added. One `("*", "value_eur_cents", …)` entry would cover both `tender_version_lots` and `v_lots`.
  - The existing `tenders.current_value_eur_cents` note (:949–955) says "The head version's highest amount". That is already stale: it is an elected figure after the sentinel, ceiling and 10ᵏ refusals.
  - The `quality` note (:962–968) says "Filter `quality IS NULL` before aggregating". That is exactly the naive `MAX` the issue describes as wrong for a lot value.
  - `TABLE_NOTES` has no entry for `tender_version_lots`.
- The `v_tender_amounts` note (sql.rs:867–879) has no lot-value guidance today. It should point at the new column.
- `/v1/sql/schema` resolves a view column's type by parsing the view's SQL (`view_column_types` and `resolve_projection`, sql.rs:2009–2079). A projected `vl.value_eur_cents` resolves to INTEGER through `PRAGMA table_info`; a CASE or COALESCE expression would resolve to `null`. A natural addition to the test at crates/app/tests/sql.rs:604–632 is `assert_eq!(col_type("v_lots","value_eur_cents"),"INTEGER")`.
- Side note: `v_tenders` does not expose `current_value_eur_cents` either.

**Two facts that bear on choosing the storage shape**
- A value-range index on `tender_version_lots` covers every version's rows. A current-lot range query (`vl … JOIN tenders t ON t.id=vl.tender_id AND t.current_seq=vl.seq WHERE vl.value_eur_cents >= ? ORDER BY … LIMIT`) therefore discards non-current rows one probe at a time. A `COUNT(*)` over a range cannot be index-only, unlike the tenders' 56–66 ms. The only recorded ratio is the bed in `.scratch/tender-db/canonical-verify/fixture_selectivity_gate.sh:17` (40.6M version rows to 13.2M lots, not prod).
- A head column on `lots` with an index `(col, id)` would mirror `tenders_current_value_eur` exactly. It would also need writing whenever the head version changes.

## 2. Adding a column and its index on prod

**The column**
- Add it to the `CREATE TABLE` in SCHEMA (canonical.rs:217–224 for `tender_version_lots`).
- Add the ALTER to `MIGRATIONS` (lib.rs:685–772). It is `const MIGRATIONS: [&str; 30]`, so the length must change. `migrate()` (lib.rs:774–781) swallows "duplicate column", and `add_column` (lib.rs:957–963) does the same for the ALTERs inside `migrate()`. Both must ship in the same commit; the issue-372 lesson is at lib.rs:733–744.
- Nullable with no default is metadata-only, O(1) at boot: crates/store/tests/alter_add_column_cost.rs and the is_buyer note at lib.rs:766–771.

**Will it survive a rebuild?**
- `tender_version_lots` and `lots` are emptied by `drop_and_recreate` (canonical.rs:10464–10483), which re-runs the stored `sqlite_master.sql`. Turso 0.7.2 rewrites that text on ADD COLUMN (`turso_core-0.7.2/translate/alter.rs:1414–1422`, `UPDATE sqlite_schema SET sql = btree.to_sql()`), so the column survives. No test pins this today.
- Only `tenders` uses a hardcoded CREATE (canonical.rs:10504–10523). A new `tenders` or head column must also go there (the lesson at lib.rs:927–931).

**The index**
- Do not put it in the schema batch: it would build at every open (issues 82/83).
- Add it to `DEFERRED_TENDER_INDEXES` (canonical.rs:10327; the `[…; 19]` length changes). The precedent is `tenders_current_value_eur` at :10412–10417.
- Built by: `build_tender_indexes` (:10643–10668, cap-checked by `too_large_to_build`), `MAX_AUTO_INDEX_ROWS` = 240M (:10230); dropped by `strip_tender_indexes` (:10620–10627) before a rebuild.
- At boot, `missing_deferred_indexes` (:10282–10312) feeds `Supervisor::ensure_deferred_indexes` (crates/app/src/supervisor.rs:3535–3580), which queues a Reindex at the front of the queue after the deploy. Per issue 111, the index exists only after that Reindex runs.
- `tender_version_lots` is already in `ANALYZE_TABLES` (crates/store/src/analyze.rs:35). The new index has no stats until the next weekly `analyze`.

**Tests that list columns or snapshot the layer**
- These are named digests with hand-written column lists. None breaks, but none covers the new column until extended:
  - crates/ingest/tests/project_golden.rs:95 (`version_lots` digest), checked against `fixtures/golden/project_apply.snapshot`. The snapshot header includes `PROJECTION_EPOCH` (:114–117). Extending the digest means deliberately regenerating the golden.
  - crates/ingest/tests/project.rs:5224–5232 `LAYER_DIGESTS`.
  - project_incremental.rs:196–229 has no `tender_version_lots` digest. `snapshot_content` cuts at `"--- digest 8 ---"`, the changes digest, so inserting a new digest before it shifts that index.
  - project_equivalence.rs:221, project_resume.rs:167 and project_fold_source.rs:217 have no version_lots digest at all. Add one so full, incremental and resumed folds are compared on the new column.
- `the_head_columns_have_exactly_one_writer_that_decides_them` (crates/store/tests/head_election_agreement.rs:354–372) greps for the text `current_value_eur_cents =`. A `lots` head column with that same name would make it count three writers and fail. A similar one-writer pin for the lot value is advisable.
- All test inserts into `tender_version_lots` name their columns, so nothing breaks at compile or insert time. The fold's writer, `flush_rows(… "INSERT INTO tender_version_lots(tender_id, seq, lot_id, kind) VALUES ", 4, …)` at canonical.rs:31055, must widen.
- Tests that hand-insert amounts and assert the served lot value will read NULL once `summarise` reads the stored column. They need to write the column or go through `apply_tenders`: lot_value_election.rs, lot_summary_equivalence.rs, lots_filter_fixture.rs, lot_summary_cost.rs, and crates/app/tests/api.rs.
- Lists by table name only, no change: `delete_version` (canonical.rs:15446–15468), `RETIRE_TABLES` (:29440–29460), `reset_tender_layer` (:10524–10543), data_quality.rs:710.
- There is no schema-version constant and no snapshot library. In openapi.json the `Lot` schema is the REST shape, not SQL (see section 3).

## 3. Docs passages that would change

**/docs (crates/app/src/v1/docs.rs)**
- :172, the `min_value`/`max_value` row: "compared against the **tender's** highest amount converted to EUR… A tender with no convertible amount never matches". If `/v1/lots?min_value=` moves to the lot value, this needs a per-endpoint split. "Highest amount" is already loose.
- :201, the `/v1/lots` row that lists `min_value, max_value`.
- :469–476, the view list ("`v_tenders`, `v_lots`, … show the current version of each row"). Optionally name the lot value columns here.
- :477–482, "do not filter a view… query the base tables… through `tenders.current_seq`". Add a lot-value base-table example.
- :683–688: "filter `quality IS NULL` before aggregating", which is the naive lot `MAX`.
- :736–742: "A derived EUR-at-publication-date column lives beside… and is what `min_value`/`max_value` compare against".
- :743–754: "A Tender's `value` is the ONE figure elected from them… One scale slip is refused…". Nothing on this page says a Lot's value follows the same rule, or that SQL now carries it.
- :755–766: "`min_value`/`max_value` compare that elected figure…".
- The tests at docs.rs:960–1080 are prose guards on specific phrases and are unaffected unless a retired phrase is reused.

**openapi.json (crates/app/data/openapi.json)**
- :453, the `/v1/sql/schema` description, which names `v_lots`.
- :758–768, the `min_value`/`max_value` parameters. They are shared by `/v1/tenders` and `/v1/lots` (:166) and say "compared against the tender's highest amount…".
- :1013, `Lot.value`: "…The same rule the tender headline uses, so the two cannot disagree". It omits the 10ᵏ rule and would gain a pointer to the SQL column.

**Markdown docs**
- README.md:102–108: "See with the views (`v_tenders`, `v_lots`, …) … query with the tables".
- CHANGELOG.md:3–5 policy: additive changes land without an entry unless an existing request answers differently.
  - SQL columns alone are additive; ADR-0015 D1, docs/adr/0015-sql-surface-promise.md:19–27, says new columns may appear without notice.
  - Switching `/v1/lots?min_value=` off the tender head does change existing answers, so it needs an entry. The entry at :93–108 states the current contract: "The value bounds on `/v1/tenders` and `/v1/lots` now compare against the tender's highest amount…".
  - The issue-484 entry (:7–28) is the precedent for "On `/v1/sql`, `v_lot_results` and `v_awards` gain …".
- docs/adr/0013-language-model.md:106–107: "The fold-time surfaces (`v_tenders`/`v_lots` on /v1/sql, `current_title`) deliberately keep the deterministic default." Still true if only the value is stored.
- docs/operations.md:545–562 (the 471 rule) says: "The per-LOT value `summarise` serves on a lot row calls the same predicate (`ScalePartners::refuses_amount`, fed the chain up to the version from `tender_version_amounts` + `tender_version_lot_results`, loaded only when a lot candidate is in the band)". This becomes "the fold stores it".
- In the code docs:
  - canonical.rs:138–146 ("MAX over the version's Amount facts") is stale today.
  - read.rs:757–759 (`FILTER_CLASSIFICATION` `min_value`) and read.rs:3374/3576 (`/v1/lots` value predicates on `t.current_value_eur_cents`) change if lots get their own bound.
- docs/architecture.md and CONTEXT.md have nothing about lot values in SQL. No passage today tells an analyst how to get a lot value in SQL. The implied route is `v_tender_amounts` / `tender_version_amounts` with `lot_id` and `quality IS NULL`, which is the incoherent figure.

## 4. Sizing the backfill (operations.md and the issue record)

**How long a refold takes**

| Run | Elapsed | Tenders written | Source |
|---|---|---|---|
| All-profile `refold` + project, jobs 2952/2953 (14,896,923 notices requeued, 8,780,780 Tenders stamped) | 10 h 26 m (07:50 → 18:16 UTC, 2026-10-07): planning about 4 h, folding about 6.5 h | 8,780,780 (0 unchanged) | 484 issue :644–647; 488 Status line (the post-defrag baseline) |
| Issue 489 drain: 156,263 Tenders stamped, routed to the bucketed path (jobs 2031/2940) | 4 h 25 m | 1.62M (the rest kept by the unchanged-chain early return) | ops.md:540–542; 488 Status line |

- Planning in the 2940 run read the parsed layer at about 1.55k notices/s.
- The 2026-10-07 all-profile refold covered 24 profiles, sized at 14,891,713 notices.

**Why any non-trivial cohort pays the whole-corpus floor**
- `INCREMENTAL_BUCKET_THRESHOLD = 100_000` (crates/ingest/src/project.rs:2617–2634), picked at :3279. At 100k or more planned notices the incremental fold switches to the bucketed sweep, which reads the whole 14.9M-notice parsed layer. So any cohort that size or larger costs about 4.5 h even if it writes little.
- `LEGACY_CLOSURE_CAP` = 500,000 forces the full fallback (ops.md:875–876, 940–941, 1598–1603).
- `refold-notices` is capped at 1,000 ids (ops.md:469–471).

**operations.md is stale on this cost**
- The all-profile runbook says "takes the full fallback (~7.5 h)" (ops.md:894) and "a SECOND full fold (~7.5 h)" (:839).
- Measured when every Tender is written, it is 10 h 26 m. Size on about 10.5 h.

**What the backfill has to be**
- No existing kept version gets the column without a forced rewrite. The fold's unchanged-chain early return is why 2940 backfilled `is_buyer` on only 1.62M Tenders.
- So the backfill is the all-profile `refold`, which stamps every Tender stale (runbook at ops.md:869–895). Size it first with `expect:1`, then run it with the real count.
- Do not bump `PROJECTION_EPOCH` (canonical.rs:1247–1302, currently 3). Issues 179 and 484 chose the refold over the bump.
- Batch it with any other pending corpus-wide fold change (ops.md:884–886).
- Deploy in a queue gap, never under a running project (ops.md:591, 1549).

**Side costs**
- One change event per rewritten Tender, about 8.78M, and the feed is append-only (ops.md:880).
- Disk: the new columns on every `tender_version_lots` row, plus the deferred index if one is added. Check `df -h /data` first (ops.md:883).
- If the index already exists while the refold runs, the fold maintains it as random-position writes, the same as `tenders_current_value_eur` during job 2953.
- Prod's `tender_version_lots` row count is not recorded in the repo. A `MAX(rowid)` read (what `too_large_to_build` uses) would size the index build cheaply.

**Rollout order**
- Before the backfill finishes, an unrefolded lot's NULL can mean either "no value" or "not computed yet".
- If `summarise` switches to reading the column in the same deploy, REST lot values go blank for up to about 10.5 h.
- Either ship the fold write and the view column first, then backfill, verify REST against SQL on a window, and only then switch `summarise`; or store a "computed" marker beside the value.
- `rederive-eur` already stamps the Tenders whose `eur_cents` moved so the fold re-elects them (crates/store/src/rates.rs:436–451). That covers the stored lot EUR value as long as the fold stays its only writer.