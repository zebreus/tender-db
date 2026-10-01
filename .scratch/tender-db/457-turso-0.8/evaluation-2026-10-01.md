# turso 0.7.2 (+ vendored SDK patch) to 0.8.1: upgrade evaluation for tender-db

S = a directory holding the published crate sources (`turso`, `turso_core`, `turso_parser`, `turso_sdk_kit` at 0.7.2 and 0.8.1, from static.crates.io). In the citations, "core-X" means S/turso_core-X, "sdk-X" means S/turso-X, "kit-X" means S/turso_sdk_kit-X and "parser-X" means S/turso_parser-X. Repo paths are relative to /home/user/tender-db. I ran nothing; every plan or cost prediction below comes from reading the source.

## 1. Verdict

**Upgrade later, not now.** Do not bump until three things are done:
- issue 429's step-0 capture diff has run on 0.7.2 (it needs the 2026-10-04 snapshot);
- the two predicted plan regressions (IS NULL seeks on the org-merge path, and correlated MAX/COUNT unnesting on the read.rs page queries) are fixed or pinned;
- the widened /v1/sql surface has a ban-or-document decision.

The 684 GB file is not the obstacle. The format is the same in both directions, and nothing is migrated at open. Rollback is safe as a procedure (clean stop, reflink snapshot, no new ALTER), and the strongest reason to upgrade is 0.8.1's fix for a torn database during checkpoints under synchronous=NORMAL.

## 2. What 0.8.1 buys tender-db

- **Checkpoint durability, the main gain.**
  - 0.7.2 copies WAL frames into the main file without first fsyncing the WAL. The checkpoint goes from Start straight to Processing (core-0.7.2/storage/wal.rs:4785), and pager.rs:4620-4626 has no WAL sync. Commits under NORMAL skip the WAL fsync too (pager.rs:4310-4316).
  - 0.8.1 adds CheckpointState::SyncWal (core-0.8.1/storage/wal.rs:2531-2538, entered at :5015 and handled at :5041-5045), for both explicit and automatic PASSIVE checkpoints (pager.rs:3403-3410).
  - tender-db runs `PRAGMA synchronous = NORMAL` (crates/store/src/lib.rs:79-84).
  - Under 0.7.2, a power loss or kernel crash can leave the file torn. The window covers every TRUNCATE checkpoint and every 1000-frame auto-PASSIVE checkpoint, and it stays open after a PASSIVE checkpoint returns, until the WAL tail is written back. An OOM kill is safe.
  - Cost: one extra WAL fsync per checkpoint that has frames to backfill. Rolling back to 0.7.2 reopens the window.
- **Issue 438, engine side.** The interrupt, deadline and progress check now runs every 256 VDBE instructions instead of before every one:
  - MAX_CHECK_INTERVAL = 256 (core-0.8.1/vdbe/mod.rs:112);
  - the countdown is at :2326-2330;
  - the quiet early return is at :2546-2555;
  - the clock is read at :1964-1966 only when a deadline is set.

  I verified this by hand. One research finding said "the clock is still read before every instruction"; that was refuted by two challenges and by the source. A set_query_timeout now costs about one clock read per 256 instructions. That figure is from the source, not measured. The SDK still hides interrupt(), so issue 425's patch stays (see §4).
- **Builder::read_only(true)** (sdk-0.8.1/src/lib.rs:284-287, :341-345). An open with it makes no change of any kind: the DB and the -wal are opened O_RDONLY, a missing WAL gets no file, there is no fcntl lock, no checkpoint at close, and write transactions are refused.
  - Use it only on copies. With no lock it would read the serving file alongside the writer without coordination (core io/unix.rs:55-71).
  - The guarantee holds only for the first open of a file in the process. The registry returns an existing instance whatever flags were asked for (core-0.8.1/database.rs:1286-1304).
  - It suits plan-probe's read subcommands: stats, plan, run and mem.
- **EXPLAIN QUERY PLAN FORMAT=JSON with per-table estimates** (core-0.8.1/translate/eqp.rs:712-835). It is useful for tender-db's plan-gate tests, which could assert the access path they expect instead of matching strings. It is not useful for a /v1/sql admission check: issue 238's admission work is DONE/superseded, and classify() is an allow-list built on the parser. The statement metrics are not new (core-0.7.2/vdbe/metrics.rs:69-95), and neither SDK version exposes them.
- **Small planner wins:**
  - partial-index matching can prove one branch of an OR (core-0.8.1/translate/optimizer/constraints.rs:1639-1649);
  - a partial fix for a wrong-results bug in LEFT JOINs against the first FROM table (:1679-1681; inner-joined tables are still exposed);
  - implied IN filters (lift_common_subexpressions.rs:15-62), which fire on literals only, so in practice only on /v1/sql SQL;
  - transitive join equalities (constraints.rs:519-600), which might fix issue 428 §3's remaining lots → lot_results → result_winners trap. Unverified.
- **What it does NOT buy.** Every documented trap and workaround stays:
  - 239: still no predicate pushdown into views (core-0.8.1/translate/planner.rs:1877-1903). Keep the view refusal and view_pushdown_probe.
  - 329: IN on a non-leading index column still cannot seek (access_method.rs:500-504, :582).
  - The GROUP BY index-stealing trap is still there (access_method.rs:304). Keep `GROUP BY +profile`.
  - 323: without stats, the `parse_state` versus id-range trap is the same (cost_params.rs:103-110). All four unary-`+` workarounds still work (constraints.rs:794, :894).
  - 421: CROSS JOIN is still honoured as the written join order (planner.rs:2522, :2730; plan.rs:1128-1161). The guidance stays valid.
  - 429/428: statistics are still sqlite_stat1 only. Excluding organizations is still required. Pooled readers still need the schema bump to pick up new stats (connection.rs:2028-2061). The 428 measurements must be redone under 0.8.1.
  - 442: the parent-key probe still needs a child index of exactly the FK's shape (core-0.8.1/translate/fkeys.rs:949, :971). The upstream request and the five FK-off brackets stay.
  - 445: FKs are still checked per statement, and `defer_foreign_keys` is still silently ignored (translate/pragma.rs:242). The test stays valid.

## 3. Risk register (ranked)

**R1. On-disk compatibility and rollback. Rating: medium, if the procedure below is followed. The research first rated this a blocker; a challenge corrected that.**
- **Format is the same in both directions:**
  - The DB header code is byte-identical (core-*/storage/sqlite3_ondisk.rs:190-402). Header validation differs only in how errors are wrapped (0.7.2 lib.rs:1713-1960, 0.8.1 database.rs:1896-2160).
  - The WAL format, magic and checksum are the same (sqlite3_ondisk.rs:404-409, :455). With no page codec, the WAL frame bytes are identical: 0.7.2 :2064-2091 against 0.8.1 :2154-2212.
  - Recovery, the 1000-frame threshold and the TRUNCATE shutdown checkpoint are unchanged (0.8.1 wal.rs:4099-4102; pager.rs:5441-5478).
  - Overflow thresholds and float encoding are unchanged.
- **Nothing is written at open for a 2/2 WAL file:**
  - Only a Legacy header is rewritten (0.8.1 database.rs:2069-2086).
  - MVCC opens only from header byte 255 (:2047).
  - Every DatabaseOpts flag defaults to false (database.rs:70-84).
  - `PRAGMA journal_mode=WAL` does nothing when the mode is already WAL (vdbe/execute.rs:19006).
- **The schema is safe in both directions:**
  - token.rs is byte-identical.
  - All of tender-db's DDL parses: STRICT, AUTOINCREMENT, CHECK, composite FKs, partial indexes, DESC, views.
  - The 0.8.1 renderer's output still parses under 0.7.2: CHECK keeps its source text, X'' is printed, and so on (parser-0.8.1/src/ast/fmt.rs:1812-1828, 1933-1949).
  - The first 0.8.1 boot re-creates every view, because the DROP/CREATE runs on every open (canonical.rs ~985-1110, lib.rs:419-420). 0.7.2 re-creates them again at its own next boot.
- **Conditions for a rollback to stay valid:**
  1. Ship no new ALTER migration with the bump. ALTER rewrites sqlite_schema through BTreeTable::to_sql, and 0.8.1 adds quoted identifiers, ON CONFLICT, COLLATE, DESC and GENERATED ALWAYS (core-0.8.1/schema.rs:3568ff, translate/alter.rs:1155, :1441). The 29 existing migrations stop on "duplicate column" before writing anything; the error text is the same in both versions (0.7.2 alter.rs:1257, 0.8.1 :1282).
  2. Never switch on MVCC, which writes byte 255 and creates a .db-log in the redesigned log format (storage/journal_mode.rs:20-47).
  3. Create no objects that only 0.8.1 understands: custom types, array columns, index methods or turso FTS, generated columns, encryption or page codec.
  4. Swap binaries only on a cleanly stopped service.
- **Remaining real risk: integrity, not format.** A logic bug in 0.8.1's B-tree writer (a 7.3k-line diff) could write malformed pages. Check with SQLite integrity_check on the tables 0.8.1 touched, run on a reflink copy.
- **Stricter reads in 0.8.1:**
  - Invalid UTF-8 TEXT raises Corrupt (0.8.1 sqlite3_ondisk.rs:1277-1282) on any query that touches the row, not at open. This is unlikely to hit, because all of tender-db's text comes from Rust Strings.
  - There are new b-tree guards against pages that are already corrupt.
- **New destructive path:** a read-write open deletes a non-empty WAL when the .db is 0 bytes (database.rs:2207-2245). Check the DB path and size before the first start.
- **0.8.1 silently opens read-only** when the header shows autovacuum and the experimental flag is not set (database.rs:1932-1940).
- **Unverified:** the live prod header. Read it before anything else; this is metadata and bounded, so it needs no approval. Expected values:
  - bytes 16-17 = 0x1000;
  - bytes 18-19 = 02 02;
  - byte 20 = 0;
  - bytes 52-55 and 64-67 = 0 (no autovacuum; these are the standard SQLite header offsets, my mapping);
  - bytes 56-59 = 1.

**R2. Correlated scalar-aggregate subqueries get rewritten (high).**
- In 0.8.1, a correlated MAX, MIN, COUNT, AVG or TOTAL subquery can be rewritten "group-first": a LEFT JOIN to a GROUP BY over the whole inner table, chosen by cost per SELECT as all or nothing (core-0.8.1/translate/optimizer/unnest.rs:1-36, :500-702, :934-1020; optimizer/mod.rs:984-1055).
- A literal LIMIT scales down the cost of the correlated form, but `LIMIT ?` does not (mod.rs:1158-1186, util.rs:1477-1496). tender-db's page queries use `LIMIT ?` (read.rs:2297-2302, :2367), so the correlated form is costed at the full outer estimate.
- Shapes that qualify:
  - read.rs:1777, :2285, :2360, :2978 and :3149, each `(SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)`;
  - read.rs:3795 and :3856, each a COUNT over organization_mentions, a table excluded from ANALYZE for good.
- Shapes that do not: read.rs:1927 correlates on two outer tables, and read.rs:577 is dead code.
- Predicted failure: a GROUP BY over about 14.5M tender_versions rows on every page request. It would surface as 408s (/v1/sql, 15 s) and 503s (REST, 25 s), not as pinned workers, provided the interrupt patch is ported.
- Separately: 0.7.2 unnested EXISTS unconditionally (core-0.7.2/translate/optimizer/unnest.rs:50-73). 0.8.1 can turn it back into a correlated subquery.
- Fix that removes the dependency on the planner: read `t.current_seq` (maintained at lib.rs:842) instead of the MAX subquery. Check first that it is equivalent. This is worth doing on 0.7.2.

**R3. IS and IS NULL become index seek keys (high).**
- 0.8.1 treats `IS` as an equality seek key (core-0.8.1/translate/optimizer/constraints.rs:1547, :2001-2006). In 0.7.2 such a term cut off the seek prefix (constraints.rs:1213).
- ORG_MERGE_SCAN_SQL (canonical.rs:1248-1250) is predicted to switch to organizations_identifier_id plus a TEMP B-TREE sort over about 24.6M identifier-less rows per batch. Indexes are evaluated newest first (schema.rs 0.8.1:1415-1436), and the sort-avoidance bonus is asymmetric (access_method.rs:435-441). The gate at crates/store/src/lib.rs:6495-6519 should fail loudly, which is the right outcome.
- The window walk at canonical.rs:21263 is predicted to flip to identifier_id on a 0.1 selectivity guess, probably a mild regression.
- The resolver probe at canonical.rs:11059 stays on name_country, but now runs the new NULL-matching seek code (null_matching_mask, core-0.8.1 execute.rs:6512-6527) on the fold's hot path.
- Out of scope, so none of this applies: `IS NOT NULL`, `IS TRUE`/`IS FALSE`, the one-word postfix `ISNULL`/`NOTNULL`, and outer-join null-extended tables.

**R4. The /v1/sql surface widens silently (high; ties to issue 426's OOM class).**
- Newly accepted by 0.8.1:
  - recursive CTEs (core-0.8.1/translate/planner.rs:1245-1252; 0.7.2 refused them at :750-751);
  - 10 window functions (function.rs:530-545);
  - custom ROWS/RANGE/GROUPS frames and EXCLUDE (plan.rs:3408; 0.7.2 refused them at plan.rs:3034-3035);
  - get_byte, set_byte and subtype.
- BANNED_FUNCTIONS (sql.rs:1379-1401) is a deny-list, so classify() lets all of it through. No test pins the old refusals, so the gate stays green while the docs go wrong (sql.rs:1085-1087, docs.rs:454, tests/sql.rs:716-719).
- Trap: 0.8.1 never reads `with.recursive`. Any CTE that references its own name is executed recursively, so `WITH tenders AS (… UNION ALL … FROM tenders)` already passes the walker as a base-table read (sql.rs:1544). A ban keyed on the RECURSIVE keyword would be bypassed; the ban has to detect self-reference.
- Real memory and CPU risks:
  - the recursive queue, which grows with fan-out (recursive_cte.rs:101);
  - the UNION seen-rows index (:136);
  - EXCLUDE frames, which rescan the frame for every row (execute.rs:8105).
- The three scalars do not amplify output. The 10 window functions keep fixed-size state per row, so they add nothing over the OVER() buffering that 0.7.2 already does.

**R5. Broad planner drift and plan-text changes (medium).**
- Automatic indexes are now costed during join-order search (access_method.rs:903-985), which risks temp-disk growth on /data for /v1/sql joins on unindexed columns.
- The cost model changed: repeated seeks (cost.rs:197-208), cost per WHERE step, constant selectivity on join probes, and IN-subquery cardinality (constraints.rs:1086-1104). Issue 428's 461-plan diff no longer applies.
- EQP text changes: SEARCH lines can now print "USING COVERING INDEX" (eqp.rs:331-336). About 40 pin sites:
  - 23 in crates/store/src/lib.rs;
  - one helper each in 14 crates/store/tests files;
  - crates/ingest/tests/data_quality.rs:626/797/806;
  - crates/app/src/v1/sql.rs:2512/2517.

  These need re-baselining. lib.rs:7085 fails with no plan change, and its comment at :7089-7090 becomes false. Assertions of absence ("never a SCAN") can pass without testing anything.

**R6. SDK operation gate (medium).**
- New in 0.8.1: execute_batch, batch and transactional_batch fail at once with Misuse while any Rows is open on the same connection or a clone (sdk-0.8.1/src/connection.rs:68, :80, :147, :265, :521-549; lib.rs:447, :488; rows.rs:13, :71-74).
- tender-db's two production sites are safe: crates/store/src/lib.rs:1080-1084 and crates/store/src/analyze.rs:94.
- The re-vendored interrupt() must not take the gate.

**R7. Interrupts are seen within 256 instructions, not at the next step (low).**
- There is no check when a statement step begins (core-0.8.1/vdbe/mod.rs:2252-2290), and the countdown carries across steps and resets (:875-879, :1049).
- crates/store/tests/interrupt_probe.rs:119-155 most likely still passes, with little margin.
- The race in interrupt() is unchanged (core-0.8.1/connection.rs:4863-4878), so keep Reader::discard.
- Production is unaffected: stop.rs re-fires every 50 ms.

**R8. Build and gate (medium, operational).**
- Bumping the patch re-hashes turso, store, ingest and app, which filled the disk on 2026-09-27 (CLAUDE.md). Gate from a cargo clean, about 35 minutes.
- New dependencies: crc32fast, getrandom 0.2/0.3, simdutf8. Neither crate declares a rust-version, so the box's nix/fenix build is the real toolchain check.
- turso_core's autovacuum feature only flips the polarity of its cfg (pager.rs:72); behaviour is neutral.

## 4. Code changes needed

**crates/vendor/turso cannot be deleted.**
- The published 0.8.1 SDK still has no interrupt or set_query_timeout. Its only timeout-like public method is busy_timeout (sdk-0.8.1/src/connection.rs:501-505), and get_inner_connection is still private (:138).
- The methods to forward to still exist: kit-0.8.1/src/rsapi.rs:1181 and :1186.
- Only half of VENDORED.md's drop condition is now met: the engine half (cheap clock reads).

The changes:

- **Cargo.toml.** Change line 44 to `turso = "=0.8.1"` and line 80 to `turso_parser = "=0.8.1"`; they move together, as in issue 166. Update the comments. The [patch.crates-io] entry (lines 147-148) stays. Then run `cargo update -p turso -p turso_parser`. Expect only the turso family plus crc32fast, getrandom and simdutf8 to move, and no "patch was not used" warning.
- **crates/vendor/turso: re-vendor the whole sdk-0.8.1 tree.** It is mechanical:
  - 7 source files, including the new batch.rs (238 lines); connection.rs grows from 251 to 596 lines.
  - Re-append the 28-line pass-through block after busy_timeout and before `impl Debug` (connection.rs:508). interrupt() calls only `get_inner_connection()?.interrupt()`, with no gate.
  - Re-derive Cargo.toml: strip [[example]] (:55-70), [[test]] (:72-78) and the dev-dependencies (:133-155); keep the target-gated mimalloc (:155-160).
  - Rewrite VENDORED.md. The issue-438 warning becomes history. The new drop condition is "expose interrupt()", plus set_query_timeout for as long as plan-probe.rs:245 uses it.
- **crates/app/src/v1/sql.rs:1229.** Change to `Cmd::ExplainQueryPlan { .. }` (parser-0.8.1/src/ast.rs:35). This is the only compile break. Expr and every other walked type are byte-identical, so walk_expr at sql.rs:1748-1856 compiles as is.
- **/v1/sql surface.**
  - Refuse any CTE that references its own name.
  - Decide ban or document for the 10 window functions, custom frames (at least EXCLUDE) and the 3 scalars.
  - Pin each decision in memory_amplifying_functions_are_refused and in canary shapes.
  - Fix sql.rs:1085-1087, docs.rs:454, tests/sql.rs:716-719, and the CHANGELOG (ADR-0015 D2).
  - New tests: `EXPLAIN QUERY PLAN FORMAT=JSON SELECT 1` is refused; `1 IN ((SELECT id FROM api_tokens))` is denied; `[api_tokens]` is denied.
- **Plan hardening.**
  - read.rs page queries: switch to t.current_seq (5 or more sites).
  - ORG_MERGE_SCAN_SQL and the window walk: stop the planner from choosing identifier_id. One option is `+identifier IS NULL`; unary `+` still hides a column from the planner (constraints.rs 0.8.1:794, :894). Untested under 0.8.1.
  - Add a result-equivalence test for IS NULL seeks over (identifier, id) and (name_norm, country IS NULL).
  - Add a plan test per qualifying correlated shape.
- **Tests.**
  - Re-baseline the ~40 EQP pins to assert the access path they expect.
  - interrupt_probe: at least 10k rows; contract becomes "within the engine's check interval".
  - Add an operation-gate contract test.
- **Stale docs.** crates/app/src/v1/stop.rs:4-12, crates/store/src/read.rs:38-40, crates/app/src/v1/sql.rs:63-65 (add a version qualifier), the patch doc comment (connection.rs:248-258), upstream-turso-requests.md §1, and about 26 "turso 0.7.2" comments and probes (canonical.rs ×7, lib.rs ×5, view_pushdown_probe.rs, mention_fk_probe.rs, turso_temp_db_leak.rs). Some probes may flip.

**Size.** About 1 to 5 lines fix the compile break, plus the mechanical re-vendor. The real work is the ~40 EQP re-baselines, 2 or 3 plan-hardening changes and the /v1/sql surface decision. My estimate is a few hundred lines including tests; that number is unverified.

## 5. Step-by-step plan

**Step 0: preconditions, all on 0.7.2.**
- Issue 429's step-0 capture diff is done. Changing the engine first would confound both studies.
- Land the R2 and R3 fixes that do not depend on the planner (current_seq, org-merge hardening) on 0.7.2, gated and deployed. This shrinks what the bump can break.
- Read the prod header bytes (R1 list).
- Copy /data/tmp/plan-capture-429.sql off /data/tmp, then de-duplicate it across processes. It has 21,296 blocks, and the capture de-duplicates only within a process.
- Parameters for timed statements must be hand-picked; the capture records none.

**Step 1: local build and gate.**
1. Work in a separate worktree, because this is a contended repo. Keep a single target directory so the disk does not hold two artifact families.
2. If you want a cross-version crash loop, copy the 0.7.2 crash_probe and plan-probe binaries out of target/ first.
3. Make the §4 changes.
4. `cargo clean` (23.5 GiB last time).
5. Run `ops/check.sh > F 2>&1; echo GATE-EXIT=$?` and read the GATE-EXIT value. Never pipe it.
6. Triage every red test. Expect the EQP pins, lib.rs:6495-6519 and possibly view_pushdown_probe. For each: re-pin with a reason, or fix the statement.
7. Run `ops/turso-crash-loop.sh 32` on 0.8.1.
8. A cross-version kill -9 loop (0.8.1 writes, 0.7.2 verifies, and the reverse) is optional. It needs two builds, which is the disk-filling pattern.

**Step 2: on-box measurement, on reflink copies only, with the queue idle.**
- Method: issue 428's .scratch/tender-db/428/time-428.sh.
- Build plan-probe at both revisions into /root. The 0.8.1 build uses read_only(true) for plan and run. A symmetric read-only baseline on 0.7.2 needs a few lines in the vendored patch: core-0.7.2 already honours OpenFlags::ReadOnly (io/unix.rs:55, lib.rs:974).
- Make reflink copies of the newest snapshot: a for 0.7.2 and b for 0.8.1. Their sqlite_stat1 must match what prod will have at bump time.
- Run everything under `systemd-run --wait --pipe --collect -p MemoryMax=4G -p IOSchedulingClass=idle -p Nice=10 timeout 120`, watching /health.
- Never start the app server on a copy: the supervisor would resume durable job rows and schedule the 07:35 UTC tick.
- **Phase R (reads):**
  - After the 0.8.1 read_only opens, b.db, b.db-wal and the directory listing must be unchanged.
  - Run `plan` over the 429 corpus plus 428's captured, timed and traps sets, every IS NULL statement and every correlated shape.
  - Normalise "USING COVERING INDEX" out before diffing.
  - Time a fixed hot set on both engines, not only the statements whose plan changed, because the execution layer changed too: 428's timed.sql, the list endpoints and the weekly-job statements.
  - Also time the three /v1/sql examples (0.73 s / 0.62 s, 1.3 s, 17 ms) and the time-to-408 of a runaway query.
- **Phase M (memory):** run plan-probe mem on a recursive UNION ALL with fan-out, a recursive UNION, and an EXCLUDE frame over tenders. Record peak RSS and /data/tmp growth.
- **Phase W (b only, writes):**
  - Open read-write with 0.8.1 and diff the 100-byte header before close; explain every changed byte. A close-time checkpoint would confound the diff, so diff before close or confirm the WAL is empty.
  - `exec` a representative write, checkpoint, and kill -9 once mid-write.
  - Fold the WAL with python3 sqlite3 `PRAGMA wal_checkpoint(TRUNCATE)`.
  - Run `PRAGMA integrity_check(<table>)` on each touched table.
  - Reopen with 0.7.2 plan-probe (stats, plan, run). This is the rollback proof.
- Delete both copies afterwards.

**Step 3: deploy.** Every Step 2 result must be explained, and every R2-R4 item fixed or pinned.
1. Commit individual files after `git diff` on each.
2. `git push origin HEAD:main HEAD:claude/tender-db-handover-i691vo`, then confirm with `git ls-remote --heads origin main`.
3. Deploy with the queue idle, well before 07:35 UTC, ideally right after a fresh weekly snapshot.
4. Sequence:
   1. Stop the service cleanly, so the shutdown TRUNCATE empties the WAL.
   2. Check that -wal is 0 bytes.
   3. Take a reflink snapshot of the db (and -wal) as the rollback point. /data is XFS with reflink, and had about 446 GB free at handover.
   4. Check the DB path and size (the orphan-WAL guard).
   5. Start 0.8.1.
5. Verify:
   - the /health rev matches;
   - journalctl is clean, nothing in `-k` mentions oom, and there is no "[store] reclaim stamped NO ledger rows" line;
   - the three /v1/sql examples are timed;
   - a runaway query still gets its 408 at about the old time;
   - /metrics stops and abandons look normal;
   - plan capture still emits: "Preparing: {}" is still at core-0.8.1/connection.rs:1088, and crates/app/src/plan_capture.rs:63/124 filter on it;
   - the system runs cleanly through one daily tick and one Sunday cycle.

**Step 4: rollback.**
- **(a) Software rollback.** Stop 0.8.1 cleanly, or fold the WAL with python3 sqlite3, then start the 0.7.2 binary on the same file. The format analysis supports this as long as the R1 conditions held (no MVCC, no 0.8.1-only objects, no new ALTER). The views are rebuilt at 0.7.2's boot. This reopens the torn-checkpoint window.
- **(b) Data rollback.** Restore the reflink snapshot taken at deploy. This loses writes made since; the canonical layer can be rebuilt from /data/archive.
- Keep the snapshot until 0.8.1 has run cleanly through a weekly cycle. It is on the same volume, so it is not disaster recovery (issue 269).

**Unverified, by category:**
- The live prod header values.
- Every plan-flip prediction in R2, R3 and R5: worked out by hand from the cost model, with no EXPLAIN run.
- The "about 1/256" deadline cost.
- The memory and CPU cost of the new /v1/sql shapes.
- That t.current_seq is equivalent to the MAX subquery.
- That `+identifier IS NULL` would stop the IS NULL flip.
- The size estimates in §4.