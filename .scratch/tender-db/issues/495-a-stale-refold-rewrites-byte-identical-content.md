# 495 — a stale refold deletes and re-inserts the whole corpus, though more than 95 % of it is byte-identical

Status: ready-for-agent — UNIT 1 DESIGNED 2026-10-08 (`wf_21089c8e-de5`: three designs, a judge that verified
their claims against the code and refuted the ones that did not hold). Decision: option (b), compare with the
stored rows at version × table grain, plus "changed versus stored" feed rows. Recorded as
`docs/adr/0017-a-re-derivation-compares-before-it-writes.md`, PROPOSED: its consumer-visible part (D1–D5) flips
with unit 4, unless Lennart objects first. UNIT 1 DONE 2026-10-08: the ADR, the null-version wording
(docs.rs and openapi.json, pinned by `the_docs_say_what_a_null_version_means`), docs/architecture.md,
and docs/operations.md "Routing a re-derivation" (D6/D7). NEXT: unit 2 (prepared statements and the
leaf-table descriptor, byte-identical).
Kind: performance / projection (`apply_tender_tx`, `crates/store/src/canonical.rs`)
Relates to: 179 (scoped staleness; it rejected rebuild=true), 488 (the defrag, which was not the cause),
496 (writer per-row cost), 497 (planning and grouping regressions), 340 (the in-place backfill precedent),
490 (the epoch-4 refold that prompted this)

## What was measured

The last identical all-profile refold, job 2044 (2026-10-07, epoch 3), took 11 h 06 m end to end:

| Stage | Time |
|---|---|
| Requeue and stamp | 48 m |
| Plan | 2 h 48 m |
| Grouping | 25 m |
| Pre-pass | 29 m |
| **Fold** | **6 h 35 m** |

The fold is the largest share. An epoch bump makes every Tender stale (keep=0), so all 8.78M are deleted
and re-inserted in full:

- 1,117,908,878 leaf rows deleted and inserted again;
- about 6 B B-tree changes, about 1.7 B of them at random positions;
- 70,643,598 change rows, because `append_version_changes` re-emits the whole history.

For a fix like 484 or 490, more than 95 % of those rows come back byte-identical.

The single writer thread is on the CPU 84.5 % of the time (88 % of that in user space) and blocked
15.4 %. Cost scales with leaf rows: from 17.5 µs per row on text-heavy eForms buckets up to 30–46 µs on
legacy buckets heavy in parties and classifications. Job 2067 (epoch 4, today) tracks job 2044 within
seconds, bucket by bucket.

The filesystem is not the cause:

- The extent count is 1,947,070, against 1,946,893 right after 488's defrag.
- `xfs_iext_lookup_extent` is 0.12 % of samples.
- The defrag made the same fold at most 4.6 % faster. That 4.6 % also includes removing the reflink
  snapshot and the 484 deploy.

## Direction

Skip the rows that have not changed, at **version × table** grain. A per-version digest does not pay
off: a 490-type change touches `tender_version_lots` on most versions.

Two shapes:

- **(a) Stored digest.** Store a digest per (version, table) and compare before deleting. It needs a
  schema column, and the first refold after it ships still pays full price.
- **(b) Read and compare.** Read the stored rows and compare them with the new ones before deleting. No
  schema change. It still reads about 1.1 B rows through the same writer, so its gain is unmeasured.

Either digest must cover the derived columns (EUR, the stored lot value from 490) and the reused ids.
A wrong "unchanged" leaves stale content in place and raises no error. That is the main risk.

**The change feed is the decision inside this issue.** Today the only way a feed consumer learns what a
refold corrected is that the whole history is re-emitted. `append_version_changes` compares new versions
with each other, never with what was stored. So:

- Skipping a Tender whose rows did not change is safe.
- A Tender whose rows did change needs a new "changed versus stored" emission. Without it, a 484-type
  fix (`is_buyer`) never reaches `/v1/changes` or SSE subscribers, and nothing reports an error.
- The documented feed behaviour (issue 179; `docs/operations.md` around lines 929–931) changes either
  way.

**Cheaper route for fixes that only add or derive a column:** update rows in place with a backfill, as
`backfill-original-lang` did (issue 340: 7.9M Tenders in 70 min, at one commit per row).

- 490's lot value could have been derived from stored rows alone, because REST already computed it at
  read time.
- That would have avoided an 11 h refold and 70M re-emitted change rows.
- The epoch-4 refold was chosen on purpose, because the epoch doubles as the completeness marker.
- Even so, the runbook should offer this route first, before reaching for an epoch bump.

## Units (superseded 2026-10-08 by "Design decision" below)

1. Design. 2. Build behind a fold test. 3. Measure.

## Design decision (2026-10-08)

Build option (b), read-and-compare on the writer at version x table grain, as the mechanism. Use design (c)'s feed semantics and routing rule, corrected as listed below. Do not build option (a).

WHY (b)
- Its 'unchanged' verdict is a value-exact comparison, in row order, of what the same write_version call would insert against what is actually stored.
- It is therefore correct whoever wrote the stored rows: org merges and their 6 callers, dissolve_condemned, repair_version_instants, the re-parse's party deletes (lib.rs:2369), backfill-original-lang, rederive, rekey, or a rolled-back binary.
- It needs no invariant, no keeper code at each writer, no seeding job, no audit job, no schema change and no new dependency. Rolling back below the flip is free.
- The gain applies to every stale path from the first refold: profile refolds, the epoch-0 scoped cohorts and the daily fold.
- The per-row read cost is no longer a guess. The rederive walk (job 1001) read 267,401,093 rows in 646 s on one connection, about 2.4 us per row.

WHY NOT (a)
- It would save about 1-1.5 h more per all-profile refold, because it does not read the stored rows.
- The price is an invariant that more than 10 outside-the-fold writers must keep, and every future writer too. A writer that forgets is exactly the silent staleness issue 495 names.
- It also needs a 1-1.2 GB side table, a seeding job, an audit job, a rollback guard and 2-3 more agent-days.
- It gives nothing to epoch-0 cohorts, which it may not trust.
- As proposed it has two verified defects. Its sum digest ignores order. Its tender/lot slot split would serve lot parties before tender parties, because PARTIES_SQL, BID_PARTIES_SQL and the stats read have no ORDER BY (read.rs:2688, 2700, 3023).

CORRECTIONS FOLDED IN
1. Compare in rowid order: count first, then every value by turso::Value equality, with reals compared by bit pattern.
2. A differing tender_versions row rewrites the whole version. A differing table is always rewritten whole for that version, so relative row order always matches a full rewrite.
3. Read the head columns from the identity row. When they are equal, write only the epoch, so the three head indexes are not churned. Stamp the epoch on every stale Tender, so it stays the completeness marker.
4. The feed uses (c)'s rule T and a corrected rule L:
   - one seq-less tender changed;
   - one seq-less lot changed for every lot of the head version whenever anything at the head differs;
   - plus every lot whose own rows differ, plus every minted lot, minus swept lots.
   No correction rows for lot_result, bid or contract, because they are not public kinds. (b)'s attribution to lot_result would have hidden a 484-type fix from /v1/lots?winner= subscribers.
5. Taken from (a): one LEAF_TABLES registry that drives the INSERTs, compare SELECTs, DELETEs and retire/reset lists, checked against PRAGMA table_info; a per-Tender identity cache; the tenders_verified and tenders_corrected counters.
6. TENDER_REFOLD_COMPARE=off|shadow|on. Run a shadow measurement before the flip, and keep off as a kill switch for at least one release.
7. D5: in-place walks that move a value REST already serves announce the same correction rows. rederive-eur stops being quiet.
8. The backfill-first routing rule (R0-R4 with E1-E6) and completeness without an epoch bump go into docs/operations.md now.

EXPECTED (estimates)
- Fold of a 484- or 490-type all-profile refold: 6.6 h becomes about 1.4-3.1 h, central about 2 h.
- Change rows: 70.6M become 0 with no logic change, thousands for a 484-type fix, and about 10-16M for a 490-type fix.
- The whole job: 11 h becomes about 6.5 h. Planning, grouping and requeue (about 4.5 h, issue 497) then dominate.
- Unit 3's shadow measurement decides the flip. If compare reads exceed about 6 us per row, try these in order:
  1. the WAL-gated in-fold TRUNCATE, which today wipes the writer's page cache every 32 batches;
  2. reading per Tender instead of per version;
  3. the fenced reader-thread pre-compare.
  Consider a stored digest only after all three.

SEQUENCING
- Nothing touches prod before job 2067 ends (about 16:45 UTC).
- canonical.rs currently has another agent's uncommitted issue-497 grouping edits. Unit 2 must start in its own worktree or after that commit.

### Units

1. ADR-0017 and the docs truth-up. One gate run, because docs.rs is Rust.
- Commit docs/adr/0017-a-re-derivation-compares-before-it-writes.md. Mark D1-D5 as taking effect with unit 4; D6 and D7 take effect now.
- Fix the 'version: null' wording in crates/app/src/v1/docs.rs:458 and crates/app/data/openapi.json:1125, and the SSE event section, to say 'existing versions rewritten in place; re-read and upsert'. This is already true for org merges since issue 286. Pin the wording with a docs test in the shape of the_docs_say_the_sse_snapshot_is_at_least_once.
- docs/architecture.md:86 and :95.
- docs/operations.md:
  - a new 'Routing a re-derivation' section (R0-R4, E1-E6, and D7's completeness checks) placed before the 479 and 490 runbooks;
  - a note on 929-931 and 840-842 that the replay ends with issue 495 unit 4.
- Done when: the ADR is committed and ops/check.sh is green.
2. Leaf-table descriptor and prepared statements (diagnosis lever 3). Byte-identical, no behaviour change.
- LEAF_TABLES (name, INSERT columns, column count, lot attribution) generates:
  - the Pending::flush prefixes;
  - delete_version, as 14 prepared DELETEs instead of format!, which removes about 209M re-parses;
  - the leaf part of RETIRE_TABLES and the reset list.
- Prepare stored_chain, returning the full tender_versions columns, and prepare append_change.
- Move values in flush_rows instead of cloning them with to_vec.
- Add a per-Tender identity cache for lot_identity and result_identity, so mega-chains stop re-probing the same entity once per version and again in append_version_changes.
- Add a PRAGMA table_info test: the schema columns equal the descriptor columns for all 14 tables, on a fresh DB and on a migrated one.
- canonical.rs currently carries another agent's uncommitted issue-497 grouping edits, so work in a separate worktree or wait for that commit.
- Done when: project_golden, project_incremental, project_equivalence and project_resume are green with no golden regeneration. Deploy it alone and record the next daily fold's timing.
3. Compare engine in shadow mode.
- Add a per-version scratch Pending. write_version writes into it unchanged.
- compare_version uses 13 prepared SELECTs of rowid plus the INSERT columns, WHERE tender_id = ? AND seq = ?. It drains the result before any write, sorts by rowid, and compares positionally with same(): count first, Real by bits.
- The head compare reads the head columns through a widened tender_identity SELECT.
- A Corrections accumulator computes D3's sets (rule T, and rule L as head-version lots plus lots whose own rows differ plus minted lots, minus swept) and only counts them.
- New Applied counters: tenders_verified, tenders_corrected, tables_skipped, tables_rewritten, rows_skipped, rows_rewritten, correction_rows_planned. Show them on the heartbeat and the counts line.
- TENDER_REFOLD_COMPARE=off|shadow|on, default off. Compare is refused when tender_version_bid_parties_version is missing.
- EXPLAIN pins for the 13 SELECTs. Classifications and parties must use the version index, not the code or org index.
- Tests:
  - off against shadow is byte-identical, by content digest and by a new per-table digest ordered by (tender_id, seq, rowid);
  - shadow counts the identical/different split that on would act on.
- On prod, after job 2067 ends: run refold-notices on a stratified cohort of about 50k notices (legacy and eForms, under both the 100k bucketed threshold and the 500k legacy-closure cap) with shadow set. Record the compare cost in us per row, the identical share per table, and the predicted correction rows.
- Done when the measurement is recorded in issue 495. It projects whether a no-change corpus fold fits in about 3 h or less. If it does not, take the adjuncts in unit 7 first.
4. The flip.
- Compare is on by default; off is the kill switch.
- Skip identical tables. Rewrite only the differing tables of a version: a prepared per-table DELETE now, and the scratch rows moved into pending. A differing tender_versions row rewrites the whole version.
- Update only the epoch when the head is equal, and stamp the epoch on every stale Tender.
- Run the sweep for every stale Tender.
- Emit D3's correction rows. Minted lots get a seq-less changed. lot_result, bid and contract get no correction rows.
- Include verified Tenders in assert_heads_match only when their head was updated.
- Tests:
  - the issue's fold pair, adapted to rule L: a refold with no logic change rewrites 0 rows, writes 0 change rows, leaves every leaf rowid unchanged and restamps the epoch. A change to one table on a non-head version rewrites only that table and writes exactly one ('tender', id, NULL, 'changed'). The same change at the head also writes one seq-less lot changed per head lot;
  - the 490 shape (value_* on tender_version_lots);
  - the 484 shape (is_buyer). An SSE /v1/lots?winner= subscriber receives it;
  - a head-only difference, and a source/kind difference;
  - a shrinking chain: sweep removed, no lot changed for the swept lot;
  - an appended notice: transition rows at N+1, plus a correction only if the prefix differed;
  - a differing version row with FK on;
  - extra, missing and reordered stored rows, with PARTIES_SQL serving a fresh fold's order;
  - a rates reload rewriting only the eur columns;
  - a re-parse that deleted parties gets them restored;
  - currency presence when amounts are skipped;
  - resume equals fresh, with compare on;
  - API tests for SSE, poll and webhooks with version null.
- Re-specify project_incremental.rs:1020 and :1097 to tenders_verified, each with an off twin that keeps the replay assertions.
- ADR-0017 takes effect: CHANGELOG.md entry, docs.rs event text, operations.md 840-842, 881, 929-931 and the counts-line wording, the PROJECTION_EPOCH doc (canonical.rs:1280-1317), and the runbook stop rule (actual correction rows outside 0.5x-2x of the shadow prediction).
- Deploy in a queue gap.
5. In-place backfills announce, and the R2 template (D5).
- Add a shared correction helper that writes rule T and rule L rows for a set of tender ids inside the caller's transaction.
- rederive-eur emits through it for the windows where a served eur_cents moved. Run 1001's shape (0 moved) emits nothing.
- Add an R2 job template: tender-PK windows, one BEGIN IMMEDIATE and one checkpoint per window, a projection_state watermark and a completion flag, and the mandatory E2 fixture (fold(new) is byte-identical to fold(old) + backfill on the golden corpus).
- Done when: rederive-eur's test shows corrections only for moved windows, and the template carries one worked example.
6. Measure (issue 495's original unit 3), on the next all-profile refold or the largest scoped one. Record in issue 495:
- fold time against job 2044's 23,674 s;
- skip ratio per table;
- us per compared row and per rewritten row;
- change rows against 70,643,598 and against the shadow prediction;
- writer hold per bucket and WAL size.
Close 495 if the fold is 3 h or less with no logic change; otherwise open unit 7's follow-ups.
7. Follow-ups that unit 6 decides, each as its own issue:
- gate the in-fold TRUNCATE (canonical.rs:15278-15282) on WAL size, so explicit checkpoints stop wiping the writer page cache the compare reads lean on;
- prepare the remaining per-Tender statements (identity, sweep, chain), since the per-Tender floor becomes the largest fold term;
- the fenced reader-thread pre-compare, only if compare reads dominate: act on 'different' freely, and accept 'identical' only if no non-fold writer acquired Db::conn since the reader's snapshot;
- a stored digest, only if that fails too.
Planning and grouping (about 4.5 h) remain issue 497's.

### Unresolved risks

- The compare cost is modelled, not measured. Job 1001 (267.4M rows in 646 s on one connection, about 2.4 us per row) anchors the read rate. Three things are still unmeasured: decoding full rows (long texts with overflow pages, classifications), the 194M per-version prepared-statement executions, and the per-Tender floor (identity, chain and sweep queries). Unit 3's shadow run settles them before the flip.
- Every 32 batches the in-fold TRUNCATE checkpoint (canonical.rs:15278-15282) wipes the writer's page cache (diagnosis correction 1). The compare reads depend on that cache, so the measured rate may be worse than the model until the TRUNCATE is gated on WAL size.
- The consumer contract changes. Rows with version null are wrongly documented today, as 'null for notices'. A consumer that ignores them misses every refold correction after the flip, and they also stop receiving the history replay. Unit 1's docs fix and the CHANGELOG entry must land before unit 4.
- Rule L over-delivers: up to about 13.2M lot rows on a change that touches the whole corpus, against 70.6M today. SSE's in-place classification against the post-correction head-1 means a subscriber may see an added or removed event for an entity whose filter membership did not move (issue 287). This is harmless under upsert semantics, but clients will notice it.
- Completeness without an epoch bump (R3 residue, R2 flag) is not proof against a restore or a rollback. A runbook rule covers it, and the per-Tender fold_rev marker is deferred.
- The compare is order-sensitive. A logic change that only reorders rows rewrites every table it reorders. That is correct, but costs up to today's time plus 5-20%. The optional automatic fallback (keep=0 for the rest of the run when a bucket's rewrite share exceeds 80%) is not designed yet.
- The changes table is on the /v1/sql allow-list. Analysts counting change rows, or relying on lot_result, bid and contract rows after a refold, see far fewer. ADR-0015 D1 makes no promise about row counts, but the change in meaning needs the CHANGELOG entry.
- Re-specifying the issue-99 replay tests (project_incremental.rs:1020 and :1097, snapshot_content) could weaken coverage of the full-rewrite path. The off twins and the equivalence test of off against on must stay in the gate as long as the kill switch exists.
- After the flip, planning, grouping and requeue (about 4.5 h of an 11 h all-profile job) dominate. 495 alone takes the job to about 6.5 h, and the rest is issue 497 and lever 2. Writer hold per bucket shrinks but is still per bucket, so other writers still wait during a refold.
- The identity-lookup share of the per-Tender floor is unmeasured, because the binary has no symbols (issue 496). If it dominates, the per-Tender identity cache in unit 2 is the lever, and its effect is also unmeasured.
- Coordination: canonical.rs has another agent's uncommitted issue-497 grouping edits in this worktree (205 lines changed). Unit 2 rewrites the same file, so it needs its own worktree or must wait for that commit. Line anchors in this review are to the committed 90a2d84.

### The judge's scores (correctness 1–10, gain in fold hours saved on a 490-type refold, cost in agent-days)

**Option (a): a stored fold digest per (version, slot), with corrections announced in place** — correctness 5; gain 5.6; cost 7

Scales used for all three rows. Correctness runs 1-10, where 10 means no plausible wrong 'unchanged'. Gain is the fold hours saved on a 490-type all-profile refold against today's 6.6 h, central estimate. Cost is agent-days to the point where the gain is live.

VERIFIED at 90a2d84 (I read the committed file; the working tree carries another agent's uncommitted issue-497 grouping edits that shift line numbers by about 60). These anchors are all correct: apply_tender_tx 15290; stale forces keep=0 at 15320-15329; early return 15330; delete loop 15361; write loop 15377; sweep 15409; head UPDATE 15427-15445; delete_version's 14 format! DELETEs 15603-15627; Pending::flush 31228-31262; flush_rows 31273; write_version, write_round and write_facts at 29131, 29213 and 29393; RETIRE_TABLES 29616; reset_tender_layer 10641; clear_canonical 8749; Applied 8009; the epoch-0 stamp sites at 8342, 9066 and 30340.
- /v1/sql uses a positive allow-list (sql.rs:135-143), so a new table is invisible there.
- twox-hash 2.1.3 is already in Cargo.lock, and sha2 is already a store dependency.
- The re-parse calls the same stamp_stale_for_profiles helper as Spec::Refold (supervisor.rs:12627 and 5409).
- clear_parsed deletes canonical party rows (lib.rs:2369-2376).

REFUTED:
(1) The design says untouched rows keep their old order and nothing observes it. That is false. PARTIES_SQL and BID_PARTIES_SQL (read.rs:2688 and 2700) and the result_stats read (read.rs:3023) have no ORDER BY, so they serve rows in (tender_id, seq, rowid) order, and the parties Vec is served exactly as read (read.rs:2847-2868). Two defects follow:
- The wrapping-sum digest ignores order, so a logic change that only reorders rows is skipped and the old order keeps being served.
- Splitting each fact table into a tender slot and a lot slot means a rewrite of only the tender slot gives those rows higher rowids, so lot parties would be served before tender parties. No full rewrite produces that order.
- T12 tests for ORDER BY rowid, which is the wrong question.
(2) The lot feed rule leaves out tender-scope changes at the head. Lots inherit every tender version predicate: read.rs:702-705, and the lots query at 3374 passes t.current_value_eur_cents. So a correction to a buyer party or a tender CPV changes which lots /v1/lots and its SSE subscriptions return, and no lot row is emitted.
(3) The backfill's '30k rows/s per reader' is the pre-collapse rowid-window figure quoted in a rates.rs comment. The rederive walk windowed by tender id (job 1001) read 267,401,093 rows in 646 s on one connection, about 2.4 us per row. That makes the backfill cheaper, but it also makes option (b)'s reads cheap, which removes most of (a)'s advantage.

STRUCTURAL: soundness depends on an invariant that every leaf writer outside the fold must maintain. At 90a2d84 those writers are:
- repoint_org_references: 5 statements, called from 6 merge paths;
- dissolve_condemned: 6 statements, including INSERT OR IGNORE into winners;
- repair_version_instants;
- clear_parsed: 2 tables;
- backfill-original-lang;
- rederive: 4 money tables;
- retire, reset, clear and rekey.
Every writer added later must maintain it too. Tenders stamped with epoch 0 are never trusted. That covers refold-notices, refold-sections, refold-fields, re-parse, rederive, the 434 rebind, rekey and the merge stamps at 21389 and 28230, so all of those keep today's full cost.

GAIN: a no-change refold folds in about 0.5-1.1 h, but only after a 25-45 min seeding job. A 490-type refold folds in about 0.6-1.4 h, saving about 5.6 h at the centre. A 484-type refold costs about the same as no change.

COST: about 7 agent-days, plus a 1-1.2 GB side table, one ~35 min gate run from a clean build (the new dependency), and seeding, audit and rollback-guard machinery.

**Option (b): in-writer read-and-compare at version x table grain (no schema change), with corrections announced as seq-less `changed` rows** — correctness 8; gain 4.5; cost 4.5

VERIFIED:
- Every anchor it cites is correct, as listed for (a).
- turso::Value derives PartialEq (vendor value.rs:5-12).
- turso 0.7.2's UPDATE deletes and reinserts the entry in every index over a SET column even when the value is unchanged (turso_core-0.7.2 translate/emitter/update.rs ~2172-2212). So updating only the epoch when the head is unchanged really saves work.
- Read order is visible to readers (read.rs:2688, 2700, 3023), and comparing rows in rowid order is the correct answer to that. That makes the compare stricter than (a)'s digest.
- Only tender_version_bid_parties_version is deferred (canonical.rs:10528). Every other leaf table has a primary key or schema index that starts with (tender_id, seq).
- Buckets are disjoint groups, and the writer is released between buckets (project.rs:3454-3457, 3534, 3546). So moving the compare to reader threads would need a fence, as the design says.
- Preparing the DELETEs removes about 209M re-parses (14.9M versions x 14 tables).

WHY IT IS SAFER: the compare is exact against whatever is actually stored. Org merges, the re-parse's party deletes, in-place backfills, rekey and a rolled-back binary all need no extra code to stay correct, and rolling back below the flip is free.

REFUTED or NEEDS FIXING:
- Feed attribution. Correction rows for lot_result, bid and contract never reach public consumers (PUBLIC_CHANGE_KINDS, sse.rs:526-533). So a 484 fix would reach them only as 'tender changed'.
- /v1/lots?winner= subscribers would get nothing, because winner corrections are attributed to lot_result rather than to the lot, while lots inherit the tender's predicates (read.rs:702-705). Replace this with (c)'s rule L.
- 'Minted entity gets added at its first seq' puts a seq-carrying row inside a correction. Use a seq-less changed instead.

GAIN: the per-row read cost now has a measured anchor. Job 1001 read 267.4M rows in 646 s on one connection, 2.4 us per row including the rate derivation. Decoding full text and classification rows will land in the upper half of the 2-6 us bracket. The 194M per-version statement executions are unmeasured.
- No-change refold: about 1.3-2.8 h.
- 490-type refold: about 1.4-3.1 h (central about 2.1 h), so about 4.5 h saved.
- 484-type refold: about 1.3-2.8 h.
The gain applies to every stale Tender from the first refold on, including scoped cohorts and the daily fold.

COST: about 4.5 agent-days. No schema change, no migration, no new admin job.

**Head-anchored in-place correction feed plus backfill-first routing (ADR-0017)** — correctness 8; gain 0.4; cost 2.5

This design covers only the feed and the routing, so its scores rate the emission rules and the routing rather than a fold mechanism. Gain counts only the fold time the feed itself costs; the routing gain is described below.

VERIFIED:
- docs.rs:458 and openapi.json:1125 say version is 'null for notices'. That has been false since issue 286.
- The SSE in-place arm and its over-delivery when neither side matches (sse.rs:406-465).
- changes.version_seq is nullable (canonical.rs:921-929).
- Lots inherit the tender's predicates (read.rs:702-705), and the lots query reads t.current_value_eur_cents (read.rs:3374). So rule L must cover every lot at the head.
- lot_result, bid and contract are not public kinds (sse.rs:526-533).
- 340's normalize function is injected across the crate seam (supervisor.rs:5685-5687).
- m490 found 6,297 of 6,297 values equal (issue 490, line 71).
- The currency_presence_complete flag is a precedent for a completion flag (lib.rs:862-877).
- changes is on the /v1/sql allow-list.

CORRECTIONS:
- Commit b02a222 does not resolve in this repo. The fix for one commit per row is documented in lib.rs:3962-3970 itself.
- Rule L as written (SELECT id FROM lots WHERE tender_id) also returns lots that are no longer in the head version. Over-delivering is allowed, but the lot ids of the re-derived head version are already in memory and give a tighter set.
- 'Unknown counts as differing' has no role under (b), because nothing is ever unknown there.
- Its interim unit 3 (cautious emission before the compare exists) would change the consumer contract twice inside one issue. I dropped it.
- D5 is right. rederive-eur moves eur_cents in place, and REST serves those amounts. The fold's compare would then find the rows already updated, so without D5 the change never reaches the feed.

GAIN: the feed costs about 15-40 min per all-profile fold (70.6M rows become 0, or about 10-16M for a 490-type fix). The routing gain is larger: R2 would have turned 490's 11 h job into about 1-2 h with zero change rows.

COST: about 2.5 agent-days for the docs, the emission and the D5 helper. Exact emission is built as part of the compare.
