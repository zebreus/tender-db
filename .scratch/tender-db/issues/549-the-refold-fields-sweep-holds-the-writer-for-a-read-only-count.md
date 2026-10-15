# 549 — `refold-fields`' carrier sweep holds the single writer for ~9 minutes of read-only work, even on a sizing run that writes nothing

Status: done — VERIFIED 2026-10-15 17:10 UTC on deploy `d23ce5a`. The sizing run `refold-fields` 2524 (BT-5141-Procedure/Lot over notice_codes, `expect: 1`) swept 16:58–17:10 UTC (11.8 min) and aborted as designed. `/metrics` reads `tender_db_writer_wait_seconds_total 0` and `tender_db_writer_longest_wait_seconds 0`, and the journal has 0 `writer acquired after` lines. Before the fix, the same sweep held the writer for 503–531 s. The Verify grep returns `0`. Units 2a/2b: `79fccdb` + `d23ce5a`. Follow-ups (projection internals, `plan_counts`, `unprojected_*`) stay as listed in Unit 1, unbuilt.
`refold-fields` (`expect: 1`) that journals no `writer acquired after a … s wait` line.
Kind: operations (writer contention)
Relates to:
- 20 (read-only accessors use a pooled reader, "never the writer");
- 241 (writer wait is measured, and a slow wait is journalled with its caller);
- 237 (`notice_ids_with_section_kind`, the indexed sibling);
- 532 and 534 (the two sizings that showed it).

## Verify

    grep -n "pub async fn notice_ids_carrying_fields" -A14 crates/store/src/canonical.rs | grep -c "self.conn().await"

- **done**: `0`. The sweep borrows a reader, and a sizing `refold-fields` run (`expect` far off) journals no
  `writer acquired after a … s wait` line.
- **open (2026-10-15)**: `1`.

## What was found

`/metrics` showed `tender_db_writer_longest_wait_seconds 507.13`. The journal timestamps are CEST:

    15:32:34 [store] writer acquired after a 503.0 s wait by crates/store/src/canonical.rs:40687 (1 caller(s) still queued)
    15:46:01 [store] writer acquired after a 507.1 s wait by crates/store/src/canonical.rs:40687 (1 caller(s) still queued)

The waiter is coverage's `layer_presence` heartbeat (`UPDATE layer_presence SET observed_at`). The holder is
`refold-fields`' carrier sweep, `Db::notice_ids_carrying_fields` (canonical.rs ~13592). It opens `self.conn()`,
the writer, and walks `notice_codes` in 500,000-id windows (`SELECT DISTINCT notice_id … WHERE field_id IN (…)`,
since `field_id` is unindexed) for 8–9 minutes:
- once for job 2519, issue 532's sizing (`expect: 1`, aborted, "nothing was written");
- once for job 2521, the wet run.

The supervisor's own phase text calls the sweep "read-only". It needs no write lock: the re-queue and the stamp that
follow it take the writer separately.

Consequences:
- Any other write queues behind a pure count for up to ~9 minutes: the coverage heartbeat, admin writes, the daily
  `probe`/`process`/`project` chain if the two overlap, and webhook/delivery bookkeeping.
- A sizing run, which runbooks recommend precisely because it is harmless, is the case that blocks writers longest
  for no write at all. 534's sizing (job 2494, 531 s) did the same.

## Units

1. **Audit (S).** List every `Db` method whose doc or behaviour is read-only but which opens `self.conn()`, ranked
   by how long it can run. Candidates:
   - `notice_ids_carrying_fields`;
   - `notice_ids_with_section_kind`, which also opens `self.conn()` (canonical.rs ~13656), though it is an index
     read and short;
   - the census-style walks.

   The writer-wait journal lines (241) give the prod evidence.
2. **Move them (S–M).** Use `self.reader()`. First check:
   - the read pool's per-statement deadline, if any: a 500k-id window took ~36 s on prod, so the window may need to
     shrink, or the job path may need a reader without the API deadline;
   - the WAL-gate interplay of a long reader during a heavy write.

   Add a test, or extend `the_refresh_plans_candidate_reads_seek`'s pattern, asserting that the sweep does not
   take the writer. For example, hold the writer in the test and expect the sweep to finish.

## Unit 1 — scoped (2026-10-15; workflow wf_f0071216-50b, analyst plus adversarial check)

### Reader pool facts (checked)
Most of the first analyst's pool analysis holds. The corrections and additions below were checked against the code.

1. Db's own pool: Db::reader() (lib.rs:1357) calls read_pool.get(). READ_POOL = 8 (lib.rs:168).
   - Readers (read.rs:38-89) is a tokio Semaphore of 8 permits plus a LIFO stack of idle connections, opened lazily with PRAGMAS and cache_pragma. Readers get busy_timeout = 5000 and the TENDER_CACHE_KIB cache, the same as the writer.
   - Drop (read.rs ~128-150) pools a connection only if it is in autocommit and was not discarded, so an open read transaction is never re-lent (issue 53).
   - CORRECTION: this pool is NOT isolated from request traffic. It serves token authentication (accounts.rs:423, authenticate_token on self.reader()), admin GETs, the dashboard and /health/deep (plain /health reads only the in-memory cursor; corrected 2026-10-15 in review). The API's REST, /v1/sql, isolate and webhook pools are separate (v1/mod.rs:184, isolate.rs:152, webhooks.rs:315/331, main.rs:52), but every authenticated request borrows from Db's 8.
   - Per job, the sweep holds 1 permit at a time and an R2/R3 dry plan holds 2 (its own plus a helper's). That leaves at least 6 for auth.
   - The rule this implies: never fan the windows or tables out concurrently (join_all, spawn) over Db's pool.

2. Deadlines: there is no per-statement deadline and no interrupt on any store::Readers (read.rs:33-39). Issue 438 removed issue 120's set_query_timeout, which made every read 1.7 to 2.9 times slower.
   - Only API borrows are bounded, through app/v1/stop.rs: STATEMENT_DEADLINE = 25 s on the REST pools (v1/mod.rs:644, answered 503 "stopped"), and DEFAULT_TIMEOUT = 15 s plus 1 s grace on /v1/sql with SQL_READERS = 6 (v1/sql.rs:101-112). The /v1/sql timeout is what produces a 408. Both are on separate pools.
   - So a job-side Db::reader() is never stopped. No job-only reader is needed and the window need not shrink.
   - Long pool reads are an established pattern: the weekly data-quality windows run for minutes by design (read.rs doc), and the dry planners of altid, rekey, scan_org_match_keys, fold_org_countries and the repair_* walks already scan on Db's pool.

3. WAL snapshot (issues 42 and 53). CORRECTION: the snapshot is per statement in autocommit, not per borrow.
   - A drained autocommit SELECT holds no snapshot afterwards; checkpoint.rs's idle-pooled-reader test verifies this. A borrow per window therefore does not change WAL pinning.
   - What a per-window borrow does bound is the semaphore permit and the WAL-gate lease, both held for the borrow's whole lifetime.
   - Back-to-back 36 s windows keep some snapshot live almost continuously. Because jobs are serialized (supervisor.rs:6 and spawn_worker :5723, which awaits one JoinHandle at a time), the only concurrent writers are small: the touch_layer_presence heartbeat every 300 s, admin verdict POSTs, webhook bookkeeping, job progress and the scheduler's enqueue. WAL growth over a 9-minute to 3-hour sweep stays in the kilobyte-to-megabyte range.
   - A concurrent TRUNCATE does not stall on a live reader in this turso: a_live_reader_snapshot_blocks_reclaim_until_it_ends (checkpoint.rs ~295) asserts that a busy TRUNCATE returns in under 1 s with busy = 1. And no non-job code path checkpoints at all; every checkpoint call site in supervisor.rs and ingest is inside a job.
   - The coverage refresher stands down for refold-fields, refold, refold-sections and match-org-identifiers (heavy_write_kind, supervisor.rs:16835).
   - Rule: never wrap the windows in BEGIN…COMMIT. That would freeze one snapshot for the whole sweep, and a cancelled future would hit the issue-53 discard path.

4. WAL gate (issue 63, TENDER_WAL_READ_GATE):
   - It is opt-in, and None means zero overhead. No in-repo unit or nix file sets it (only issue notes mention it); I could not check the box's environment.
   - When it is on, every borrow from every pool holds the shared side of one fair tokio RwLock for the borrow's lifetime. checkpoint_gated (checkpoint.rs:113) waits at most 5 s for the exclusive side and then checkpoints ungated. Its only caller is project's TRUNCATE (ingest/project.rs:2358).
   - Additional hazard: a dry R2 or R3 on a reader holds a gate lease while a helper takes a second lease. If a gated checkpoint queued its exclusive request between them, the nested shared acquire would wait behind it while the checkpoint waits on the outer lease. That deadlock is bounded by the 5 s timeout, and it cannot happen while project is serialized with match-org-identifiers. match_org_altid_pairs' dry path already has this shape.

5. Planner statistics. CORRECTION: the first analyst's bullet does not apply to the sweep.
   - The notice_* value tables carry no sqlite_stat1 rows: issue 428 kept the raw notice layer out of ANALYZE_TABLES (analyze.rs:12-23).
   - For the one table that does have statistics (`notices`, used by the count and the section-kind join), finish_analyze's CREATE plus DROP schema bump (analyze.rs:84-104) makes the writer and every pooled reader reload the same statistics.
   - A fresh test DB has no statistics either, so an EXPLAIN test cannot detect statistics drift. It is a shape guard only.
   - In turso the value tables' composite PRIMARY KEY on a rowid table is an autoindex, shown as `SEARCH <t> USING INDEX … (notice_id>? …)`. The existing assertion at lib.rs:5206 relies on exactly that text.

6. Runtime and cache:
   - The sweep still runs inside the job's future on the isolated worker_runtime (spawn_worker), so its blocking preads stay off the API runtime, as before.
   - With per-window borrows, the LIFO idle stack usually hands the same connection back to the sweep. Under concurrent load a window can land on another connection, which then fills its page cache (up to TENDER_CACHE_KIB) with cold value-table pages. That stays within the per-connection ceiling cache_pragma's doc already budgets.

Verdict: a job can hold a pooled Db reader for about 10 minutes, or for hours, safely. There is no deadline, no 408, no stalled checkpoint, and no gate interplay while jobs are serialized. Use a borrow per window anyway: no permit or gate lease outlives a window, the loop is the natural stop point for issue 543 unit 3, and keeping the work sequential protects the auth path's share of the pool.

### Read-only work on the writer, ranked
| accessor | where | cost | move? |
| --- | --- | --- | --- |
| `notice_ids_carrying_fields` | crates/store/src/canonical.rs:13592 (writer taken at :13604; one guard is held across MAX(id) and every window of every table) | Measured on prod, with one correction to the first analyst's wording: 503.0 s (job 2519, a sizing run that wrote nothing), 507.1 s (job 2521) and 531 s (job 2494, the 534 sizing run) are how long the heartbeat WAITER (`t | yes. It is pure SELECT and calls no helper with &conn: no TEMP table, no checkpoint, no PRAGMA. The job writes nothing before the sweep, and its following write |
| `match_org_identifiers_r2 (dry path; also serves rule e0)` | crates/store/src/canonical.rs:22292 (writer :22299, taken before the dry_run branch; `if args.dry_run` at :23013 returns at :23033; the first write is BEGIN IMMEDIATE at :23065) | The dry run writes nothing but holds the writer through the whole B-ID preload. The preload is keyset pages of 20,000 ids, each its own statement, over every identifier-bearing org (1.16M+). The writer also stays held th | needs-care: only the dry path moves, using the reader/writer switch from match_org_altid_pairs (:25969-25977). I verified the dry region: its only &conn helper  |
| `match_org_null_country_r3 (dry path)` | crates/store/src/canonical.rs:25396 (writer :25401; `if args.dry_run` at :25773 returns at :25774; the first write is BEGIN IMMEDIATE at :25793) | The dry run holds the writer through the canonical target-map preload, null_country_ident_orgs (a reader), per-candidate name probes (readers) and evidence_keys(&conn) twice per candidate pair (read-only queries). It wri | needs-care: dry path only, using the same switch as R2. The switch keeps a `Reader` (about 70 to 100 bytes: Arc pool, Option<turso::Connection>, a semaphore per |
| `projected_notice_count_for_profiles` | crates/store/src/canonical.rs:13408 (writer :13412) | COUNT over notices with profile IN (...), probably via notices_profile, with a row lookup per hit for parse_state and projected. It scales with the profile cohort: tens of seconds to minutes for a whole eForms SDK profil | yes. No writes, no helpers. Its count is already non-atomic with the UPDATE that follows (the guard dropped between them before), so a reader changes nothing. ` |
| `notice_ids_with_section_kind` | crates/store/src/canonical.rs:13652 (writer :13656) | An index range on notice_sections_kind joined to notices by rowid. Measured at 15 ms for 11,416 GroupComposition sections on prod; seconds for a very common kind. | yes. Trivial and safe, and named in the issue. |
| `requeue_tender_notices (dry path)` | crates/store/src/canonical.rs:13694 (writer :13695) | tender_versions seeks plus one COUNT per REQUEUE_CHUNK (500), bounded by the cohort. Seconds. | no, as written. CORRECTION to the first analyst: the dry branch is NOT pure read. requeue_notice_ids (canonical.rs ~12175) runs `checkpoint_on(conn, Truncate)`  |
| `plan_summary` | crates/store/src/canonical.rs:18023 (writer :18024) | A streamed ORDER BY group_key over all of plan_notice: about 47M rows on a full rebuild, minutes. | no for 549. It is on the rebuild path, which monopolises the writer anyway. If a follow-up ever moves any plan_* reader: plan_notice is a regular table (not TEM |
| `plan_counts` | crates/store/src/canonical.rs:17999 (writer :18000) | COUNT(DISTINCT) over plan_notice: the in-RAM distinct form that plan_summary's doc (:18015-18019) says 'spun for many minutes at 12.4M'. A minute or more on a big refold delta (3.29M); milliseconds on a daily delta. | needs-care: follow-up issue. Fix the query shape (stream it like plan_summary) before moving it, and add the DROP/CREATE schema-refresh test noted under plan_su |
| `plan_is_complete` | crates/store/src/canonical.rs:15399 (writer :15400) | Four sqlite_master probes, then COUNT(*) of plan_notice and a COUNT of parsed notices with id <= MAX(planned): tens of seconds to minutes when a plan is on disk. | no: rebuild path. |
| `unprojected_legacy_notice_count` | crates/store/src/canonical.rs:13319 (writer :13320) | Likely a notices_unprojected partial-index walk with a row lookup per hit for profile. Milliseconds on a daily delta; tens of seconds at 3.29M; minutes after a whole-corpus re-queue. | yes, in a follow-up unit. The requeue that fed it committed in an earlier job, and it runs before the fold takes the writer. notices_unprojected is built by ens |
| `unprojected_parsed_notice_ids` | crates/store/src/canonical.rs:13338 (writer :13339) | An id-only walk of the notices_unprojected partial index. Seconds at 3.29M; longer after a whole-corpus re-queue. | yes, in the same follow-up unit. |
| `touched_existing_tender_ids / notice_ids_for_tenders / tenders_for_notice_ids / legacy_notices_for_keys / legacy_keys_for_notices / refused_family_tender_ids / refused_keys_of_tenders / tender_link_neighbours / resolve_declared_links / next_plan_batch` | canonical.rs:39750 / 39901 / 15760 / 15720 / 15739 / 39810 / 39866 / 16482 / 16405 / 18049 | IN_CHUNK seeks bounded by the delta or closure: milliseconds to seconds on a daily delta, estimated at tens of seconds each for touched_existing_tender_ids and notice_ids_for_tenders over a 3.29M refold. Each releases th | needs-care: not in 549. These are projection internals. next_plan_batch reads a DROP/CREATEd plan table (see plan_summary). The writer-view doctrine is stated a |
| `mention_resolver (plus arm_altid_alias / arm_nhs_body_alias)` | canonical.rs:18142 (writer :18153); arm_altid_alias :18453 (:18459); arm_nhs_body_alias :18597 (:18598) | An ORDER BY id walk of every identifier-bearing org plus the withheld set, on every fold: seconds to tens of seconds. The arms are short ledger seeks. | needs-care: separate issue. The guard drops on return, so the writer adds no isolation, but this is the fold's correctness surface. |
| `retire_absorbed_legacy_tenders (conditionally read-only)` | crates/store/src/canonical.rs:39622 (writer :39627) | Scans tenders with procedure_key LIKE 'ojs:%' and writes only when something was absorbed. | no: rebuild path. |
| `per-window dry runs: merge_provisional_organizations_batch, backfill_merged_window, build_org_match_keys_batch, repair_nested_org_mentions_batch, repair_placeholder_orgs_batch, sweep_orphan_orgs_batch, restore_dropped_satellites, unwind_nhs_body, apply_rehoming, apply_country_verdicts, apply_case_reviews, unapply_case_reviews` | canonical.rs:21846 / 22074 / 13863 / 37440 / 38033 / 38898 / 24437 / 29071 / 30513 / 30890 / 32095 / 32264 | Each takes the writer before its dry_run branch. The batch functions hold it for one window (the supervisor re-acquires per batch); the verdict applies are bounded by a reviewed cohort. Seconds per hold. | no for 549. Several are wet-in-one-transaction by design. For completeness, the whole-corpus planners I checked ALREADY scan on readers and take the writer only |
| `stamp_stale_for_notices / unmark_projected_by_ids (the refold-fields wet phase, for context)` | canonical.rs:13800 / 13680 | For a 3.29M-carrier cohort, the tender_versions reads plus the batched UPDATEs and TRUNCATEs are minutes of writer work. | no: real writes. Listed so the prod verify does not expect a wet run to journal zero writer waits. Only the sweep phase and the sizing run become writer-free. |
| `missing_deferred_indexes` | crates/store/src/canonical.rs:14938 (writer :14939) | A sqlite_master scan, milliseconds; it only waits behind a held writer at boot. | yes, but optional and low priority. A reader sees committed DDL. |
| `tender_layer_state` | crates/store/src/canonical.rs:40704 (writer :40705) | EXISTS plus one point read: O(1). | no need. It is O(1), and /health/deep already reads the stored verdicts on the pool (doc ~:40716-40723). |
| `rebuild_in_progress / legacy_adjacency_watermark / tender_links_complete / currency_presence_complete` | canonical.rs:15372 / 15611 / 15640; lib.rs:3945 | Single-row reads: O(1). | no. Documented writer-view gates (canonical.rs:15620-15626 keeps a separate _observed reader twin), and O(1). |
| `foreign_keys_enabled` | crates/store/src/lib.rs:1370 (writer :1371) | A PRAGMA read, O(1). | no. It reports the writer connection's own PRAGMA state, which is the classic writer-side state. |

### Build spec (units 2a and 2b)
UNIT 2a, the 549 fix (S). This is a store-only source change, but NOT free of supervisor stack risk. The three store futures are awaited inline in run_maintenance_spec, whose future-size budget is 2,176 bytes (gauge in run_spec_futures_stay_inside_their_size_budgets, supervisor.rs ~:24038). The R3 future is inline in run_match_identifiers_spec (3,840 bytes, about 40 bytes of headroom per supervisor.rs:7487). A `Reader` held across awaits is about 70 to 100 bytes, against the 8 bytes of a MutexGuard.

1. canonical.rs:13592 `notice_ids_carrying_fields`:
   - Remove `self.conn()`. Take `max_id` from the existing `self.max_notice_id().await?` (canonical.rs:37751: reader, NULL mapped to 0).
   - In the window loop, borrow per window: `let conn = self.reader().await?;`, then query and drain, with `rows` dropped before `conn`.
   - Keep WINDOW = 500_000, sequential windows and tables, autocommit statements, no BEGIN, and the `[store] field sweep` lines.
   - Extract `pub(crate) fn carrier_window_sql(table: &str, n_fields: usize) -> String`.
   - Doc comment: reads go through Db's pool, never the writer (issue 549). A borrow per window bounds the pool permit and any WAL-gate lease to one window. Snapshots are per statement either way. Never wrap the windows in a transaction, and never parallelize them: the pool also serves token auth.
   - The issue's Verify grep must then return 0.
2. `notice_ids_with_section_kind` (:13656): change `self.conn().await` to `self.reader().await?`.
3. `projected_notice_count_for_profiles` (:13412): the same change. Its caller `unmark_projected_for_profiles` (:13770) counts and only then takes the writer, so the calls are sequential and cannot deadlock.
4. Do NOT move `requeue_tender_notices`' dry branch in this unit. requeue_notice_ids runs checkpoint_on(Truncate) every 100 chunks even when dry, so it must not run on a reader connection.

UNIT 2b (S, same PR or the next):
- Replace the unconditional `self.conn()` in `match_org_identifiers_r2` (:22299) and `match_org_null_country_r3` (:25401) with altid's switch (:25969-25977): `let reader; let writer; let conn: &Connection = if args.dry_run { reader = self.reader().await?; &reader } else { writer = self.conn().await; &writer };`
- I verified that every write is after the dry return: R2 returns at :23033 and first writes at :23065; R3 returns at :25774 and first writes at :25793. The only &conn helpers on the dry path are withheld_identifier_orgs and R3's evidence_keys, both read-only, and the other helpers already use readers.
- In the same change, wrap supervisor.rs:11121 as `Box::pin(self.db.match_org_null_country_r3(...)).await`, the issue-467 rule that :10506 already follows. The e0 arm (:10710) and r2 (:10506) are already boxed.
- Wet runs keep the writer from plan to last merge. Add a sentence to both doc comments: a dry plan now runs beside admin verdict POSTs, so post verdicts before the dry run.

TESTS:
(a) `the_carrier_sweep_runs_while_the_writer_is_held`, in the lib.rs #[cfg(test)] module, which can call the private `db.conn()`:
   - Seed like lib.rs:5262, plus `UPDATE notices SET projected = 1 WHERE id IN (7,8)` and one `notice_sections(notice_id, section_id, kind)` row, e.g. (7,'S1','GroupComposition').
   - Then `let held = db.conn().await;`, and read `let before = db.writer_stats().acquisitions;` AFTER taking `held`.
   - Under `tokio::time::timeout(Duration::from_secs(5), …)`, each with the message "must not wait on the writer (issue 549)": `notice_ids_carrying_fields(&["TED-DATE_OF_CONTRACT_AWARD"], None)` expects vec![7]; `notice_ids_with_section_kind(&["GroupComposition"])` expects vec![7]; `projected_notice_count_for_profiles(&["ted-export-r208"])` expects 2.
   - Assert `writer_stats().depth == 0` and acquisitions == before. Then drop(held).
   - Pattern: accounts.rs:546.
(b) `the_carrier_window_seeks_each_value_table` on `db.reader()`:
   - For each t in NOTICE_VALUE_TABLES, run `EXPLAIN QUERY PLAN carrier_window_sql(t, 2)` with bound params (lo, hi, two text ids). Read column 3, as head_election.rs:2790 does.
   - Assert a line containing "SEARCH", t and "notice_id" (turso shows the composite-PK autoindex as `USING INDEX …`; do not assert "PRIMARY KEY"), and no line starting with "SCAN ".
   - Justify it as a shape guard for the extracted SQL, not a statistics guard: the value tables have no sqlite_stat1 rows, and a test DB has none at all.
(c) For 2b: the R2 and R3 fixtures are integration tests (store/tests/r2_merge.rs `seed`/`args(dry_run, expect)` at :106/:228, r3_merge.rs :67/:245). Their fn-pointer rules and seeds are about 150 lines each, so do not copy them into a unit module.
   - Add `#[doc(hidden)] pub async fn hold_writer_for_test(&self) -> tokio::sync::MutexGuard<'_, turso::Connection>`, following the `execute_for_test` and `set_auto_index_row_cap_for_test` precedent.
   - In each file, add a test that holds it and runs `args(true, None)` under a 5 s timeout, expecting the same report the existing dry test expects.
   - Test (a) may live in store/tests/refold_fields.rs through the same hook instead.
(d) Existing tests stay unchanged and must pass: lib.rs:5262 and :10180-10215, store/tests/refold_fields.rs, ingest/tests/project.rs:1143, r2_merge/r3_merge/e0_merge.
(e) The future-size gauge `run_spec_futures_stay_inside_their_size_budgets` and the stack messenger `an_execute_without_an_expected_count_is_refused` must stay green. If run_maintenance_spec goes over budget, box the inline store awaits at supervisor.rs:7905/7943/8079. Never raise the number.

Focused run, with the gate's flags and package set, multiple filters after `--`:
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model -p store -p ingest -p tender-db --features tender-db/server -- carrier_ field_sweep writer_is_held dry_plan r3_ run_spec_futures_stay_inside_their_size_budgets an_execute_without_an_expected_count_is_refused`.
Then run `ops/check.sh` redirected to a file, and read the `GATE-EXIT=` value, never a piped tail. Before staging each file, run `git diff` on it. Do not run any of this while another gate is running.

DOCS:
- The doc comments of the sweep, R2 and R3.
- docs/operations.md:490-503 (refold-fields block): the sweep reads through the reader pool. Heartbeat, admin, webhook and enqueue writes proceed during it; queued jobs still wait their turn. A sizing run (`expect:1`) holds no writer. A wet run still holds the writer for its re-queue and stamp phase.
- supervisor.rs:5178 (UNSTOPPABLE_KINDS reason for refold-fields): the wording stays true. Add that the per-window loop is unit 3's stop point (issue 543), and do not widen 549 with a stop closure.
- Optionally refresh the stale arm comment at supervisor.rs:8058-8064 ("a full walk of notice_texts and notice_amounts").
- The issue 549 file: put this audit table, Status and Next.

PROD VERIFY after deploy (the restart resets the /metrics high-water):
- The Verify grep returns 0.
- Enqueue a sizing run: `{"kind":"refold-fields","profiles":[…],"tables":["notice_codes"],"expect":1}`.
- The journal shows `[store] field sweep` lines and NO `writer acquired after a … s wait` line for the sweep's duration.
- `tender_db_writer_longest_wait_seconds` stays under SLOW_WRITER_WAIT (10 s, lib.rs:597).
- The job aborts with its count and "nothing was written".
- Do not judge a WET run by zero waits: unmark_projected_by_ids and stamp_stale_for_notices are real writer work.
- After 2b, an R2 dry run (`match-org-identifiers` r2, default dry) likewise journals no wait.

FOLLOW-UP ISSUE (do not widen 549):
- Projection-start reads: unprojected_parsed_notice_ids (:13338) and unprojected_legacy_notice_count (:13319).
- plan_counts (:17999): first fix the COUNT(DISTINCT) form to plan_summary's streamed shape; then any plan_* reader move needs a test that a pooled reader sees plan tables that clear_plan_on DROPped and re-CREATEd.
- requeue_tender_notices dry: first skip requeue_notice_ids' TRUNCATE when dry, then switch to a reader.
- missing_deferred_indexes at boot.
- Measure first, from a big refold's [project] stage timings.
- Leave on the writer on purpose: the writer-view gates (legacy_adjacency_watermark, tender_links_complete, rebuild_in_progress), the fold internals and mention_resolver, foreign_keys_enabled, the rebuild-only plan_summary, plan_is_complete and retire_absorbed_legacy_tenders, and the per-window dry batches.

