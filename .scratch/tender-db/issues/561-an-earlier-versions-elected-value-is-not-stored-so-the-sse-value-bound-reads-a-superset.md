# 561 — an earlier version's elected value is not stored, so the SSE value bound reads a superset and `max_value` (and at times `min_value`) hears `changed` where `added` was due

Status: ready-for-agent — DECIDED 2026-10-17 (wf_3176baf8-d14, judge + adversarial check, amendments adopted): A — store each version's elected value (`tender_versions.value_eur_cents`, PROJECTION_EPOCH 4→5, lazy fill by later folds, quiet single-column compare rule) and read it in the At probe behind an epoch gate with the superset as fallback; `v` pinned to its PK seek; a `reset` for value-bounded resumes from before the migration. Unit 7 filed as issue 566. NEXT: unit 3 (CHANGELOG/Verify wording, markdown only), then unit 4 (build) after 551 closes.
Tender value per version (as 490 stores the lots'), which needs a refold, or keep the documented one-sided superset.
Kind: correctness (SSE `added`/`changed` under `min_value`/`max_value`)
Relates to:
- 547: `ValueBy::Version` reads the head column at the head and, at an earlier version, `EXISTS` an amount row of that
  version inside the bound — a superset, because the election (sentinels, scale partners over the chain; issues 366,
  375, 471, 492, 505, 537) is not transcribed into SQL.
- 490: lot values are already stored per version for exactly this reason (`Scope::At` serves non-head versions).
- 164 / 287: removal over-delivery, the accepted direction of the error.
- 560: lots hear a Tender value move only with a lot row; independent of this one.

## Verify

    grep -c 'ELSE EXISTS (SELECT 1 FROM tender_version_amounts va' crates/store/src/read.rs

- **done**: 0 (the At probe reads a stored per-version elected value), OR a `## Decision` below keeps the superset
  with its reason.
- **open (2026-10-16)**: 1 (the superset leg in `version_predicates`' `ValueBy::Version` arm).

## What was found

The earlier side matches when ANY amount row of that version lies within the bound. Under `max_value` that includes
every lot figure below the procedure total, so on a Tender with several lots an earlier version nearly always reads as
inside a `max_value` bound: a Tender whose value drops into the bound arrives as `changed`, never `added`. Reproduced
in the 547 review: CHAIN[0] with procedure EUR 900,000 / lot EUR 300,000, then procedure EUR 400,000 / lot EUR 300,000
→ `/v1/tenders?max_value=50000000` and `/v1/lots?max_value=50000000` both got `changed` (snapshot 0, page 1); with the
v1 lot at EUR 600,000 instead both got `added`. Under `min_value` the residue is any stored figure the earlier
version did not elect lying inside the bound — a refused figure (e.g. implausible or scale-refused), a
partner-only BT-709/BT-660 framework figure (stored and served at its lot, issue 506, never elected), a `Part` figure
(`elect(false)` on a version with Lots, issue 507). It gives an over-delivered idempotent `removed` when the new head
leaves the bound, or `changed` where `added` was due when the new head is inside it. Reproduced in the 547
re-review (2026-10-16): CHAIN[0] with the procedure at EUR 400,000 plus an injected BT-709 maximum of EUR 1,000,000
for LOT-0001 (stored as `result_framework_maximum`, not elected; both `min_value=50000000` snapshots empty), then
CHAIN[1] at EUR 600,000 → `/v1/tenders?min_value=50000000` and `/v1/lots?min_value=50000000` both got `changed`;
a refused figure (the lot at EUR 200 bn) in place of the BT-709 one did the same. The served docs now say to apply
`changed` as an upsert by id, which makes the client state right either way; the event is still the wrong op.

## Units

1. **Decide (S).** A: a `tender_versions.value_eur_cents` (or a sibling table) written by the fold for every version
   with `head_value_eur_cents_with` over the running `ScalePartners`, read by the At probe in place of the EXISTS;
   costs a column fill over every version (job-706 shape, 375's rule: the fold's own function, not SQL). B: keep the
   superset.
2. **Build + fill (M–L)** if A, then retire the CHANGELOG 547 caveat and the `/docs` superset clause (keep the
   upsert advice: `version: null` rows still need it).

## Decision (2026-10-17, workflow wf_3176baf8-d14)

Two independent designs (minimal / complete), a judge, then an adversarial check (sound: True); the check's amendments are ADOPTED and override the judge's text where they differ. Full record: `.scratch/tender-db/56x-decisions/wf_3176baf8-d14_result.json`.

### The decision

Decision: A. Store each version's elected value and read it in the At probe, with the superset kept as a guarded fallback. Build it the way COMPLETE's A1 describes, but fill it lazily as in A2: no dedicated all-profile refold.

What gets built:
- A column on the version row, `tender_versions.value_eur_cents`. `write_version` (canonical.rs:40803) writes it using `head_value_eur_cents_with(v, partners, rates).eur_cents`, over the running `ScalePartners` that is already set for that version (canonical.rs:22849-22853). So the stored value is exactly what the head column held when that version was the head (issue 375's rule: the fold's own function, never SQL).
- A quiet compare rule. When only this column differs, the compare runs one single-column UPDATE. It does not rewrite the whole version (D1.4) and writes no rule T/L correction row (D3). It is counted as `values_refreshed`, so `verified + corrected == stamped` still holds.
- `PROJECTION_EPOCH` goes from 4 to 5.
- An epoch-gated reader in `ValueBy::Version`, in this order:
  - at the head: the head column;
  - when `t.projection_epoch >= 5`: `v.value_eur_cents`;
  - otherwise: today's superset, with `t.projection_epoch < 5` added to each of its subqueries.
  The superset code stays for good. It is the fallback for a rolled-back binary, which stamps 4, for Tenders stamped 0, and for Tenders no fold has touched yet.

How it fills: every fold that touches an epoch-4 Tender after the deploy fills all of that Tender's versions. It does so in the same transaction that writes that Tender's new change rows. So every transition row written from the deploy on is probed exactly, and that is the live traffic. The remaining Tenders fill with the next all-profile refold that runs for any other reason (ADR-0017 D6). No R4 is run for 561 alone, because the only thing left on the superset is replay of change rows written before the deploy, on Tenders not folded since.

Rejected:
- B, keeping the superset for good. It leaves about 16-19 % of max_value probes with the wrong op on live traffic, permanently.
- N and B+. They are partial fixes, and B+ risks the forbidden under-delivery direction.
- A sibling table. It needs a second 14.9 M-row B-tree and an extra seek per probe, and an older binary would leave orphan rows that break the FK.
- A dedicated R4 now. Its reach beyond the lazy fill is close to nil.

561 closes when the deploy is live and the canary read has passed (unit 5). Two things happen now, before the build: the Verify is re-pinned (the old grep prints 0 while the superset is live, so it falsely reads done), and the CHANGELOG 547 bullet gains 551's lot-sum leg, which no served text names today.

### Units

- Unit 3: docs now (S). Status: ready-for-agent. Blocked by: nothing. Markdown only (CHANGELOG.md plus the issue file), so no gate and no deploy. (a) Replace the issue's Verify with the one below. The old grep `ELSE EXISTS (SELECT 1 FROM tender_version_amounts va` prints 0 on 7e61f2b because 551 unit 3 re-spelled the leg `ELSE (EXISTS (...` (read.rs:1463, count 1), so the issue falsely reads done. (b) In CHANGELOG.md's issue-547 bullet (lines 79-88), add the leg 551 introduced to the list of things that make an earlier version match: 'or, under `min_value`, its lot figures summed (estimates, awards and framework maxima, EUR 0 to the cap, plus one cent a row) reach the bound (issue 551)'. MINIMAL measured this leg as the cause of 79-97 % of min_value over-reads. Leave /docs line 498 (docs.rs) alone: it is still true, and unit 4 rewrites it. Check: `git show --stat` lists CHANGELOG.md and the issue file only.
- Unit 4: build (M). Status: ready-for-agent. Blocked by: 551 closing (its post-drain bounded read, wf_dc85fa7e-c00), so that the epoch bump does not mix into 551's evidence. If 560 is decided as A by then, batch it into the same gate: its `append_version_changes` needs exactly this per-version HeadValue for N-1 and N. One gated commit set.
(a) Migration. Add `ALTER TABLE tender_versions ADD COLUMN value_eur_cents INTEGER` to lib.rs's add-column list, beside 490's line 830.
(b) Fold. `write_version` pushes `head_value_eur_cents_with(v, partners, rates).eur_cents` into the Versions row. The head UPDATE reuses the last version's HeadValue instead of electing a second time.
(c) Compare (canonical.rs ~1640-1700, `write_what_differs` at 23060). `LeafTable` gains a quiet column, which is INSERTed but left out of the `equal[Leaf::Versions]` test. When only that column differs, one `UPDATE tender_versions SET value_eur_cents=? WHERE tender_id=? AND seq=?` runs. That is neither `differs` nor `head_differs`, so no rule T/L row is written. Count it as `Applied.values_refreshed` and print it on the project counts line.
(d) `PROJECTION_EPOCH` 4 -> 5 (canonical.rs:1957), plus the contract text. Gate the reader on a named const `VERSION_VALUE_EPOCH = 5`, not on `PROJECTION_EPOCH`, which will move again.
(e) Reader, the `ValueBy::Version` arm (read.rs:1424-1470). The variant gains `version_value: "v.value_eur_cents"` and `epoch: "t.projection_epoch"`. The SQL becomes `CASE WHEN seq = head_seq THEN <head> WHEN epoch >= 5 THEN v.value_eur_cents <op> ? ELSE <today's superset> END`, and every fallback subquery also carries `epoch < 5`, as it carries `seq IS NOT head_seq` today. Rewrite the variant's doc comment (read.rs:1205-1226) to match.
(f) ADR-0017 amendment:
- D1.4: a quiet column is left out of version-row equality.
- D3: a quiet refresh is not a difference.
- D5: a stored per-version copy of an election that only the SSE transition probe reads is filled and refreshed quietly. A live subscriber's held state is set by the head probe, which was already exact.
- Accepted residual: a lagging subscriber whose catch-up straddles the commit that fills a Tender, between that Tender's rows N and N+1, can keep that Tender. This happens only in the over-read class and in a window of milliseconds.
(g) Docs:
- operations.md:1428-1429: `<> 4` becomes `<> 5`, with a note that until an all-profile refold runs, an epoch-4 Tender is simply one no fold has touched since the deploy and reads the superset. R3's `= 0` residue checks are unaffected.
- operations.md rollback paragraph: an older binary stamps 4, so the reader falls back to the superset. That is over-delivery only, with no NULL hazard.
- operations.md all-profile-refold checklist gains unit 6's step.
- CHANGELOG: the new column is quiet. The 547 bullet narrows to 'a change written before <deploy date>, on a Tender not projected since, reads its earlier version as a superset'.
- /docs line 498: narrowed the same way. Keep the upsert advice (version:null rows, 560, 562).
Tests:
- canonical.rs `the_stored_version_value_is_the_head_value_of_each_prefix`. The value stored at seq N equals `head_value_eur_cents(&chain[..N])` for every fixture chain, including a 551 path-V/W derived value at a non-head version, scale-refused, partner-only, Part, EUR-0, sentinel and no-rate figures. This follows the pattern of `the_running_scale_rule_is_of_chain_of_each_prefix`.
- `assert_heads_match` (canonical.rs:23120) also asserts that `v.value_eur_cents` at `current_seq` equals `t.current_value_eur_cents`.
- canonical.rs `a_value_only_difference_is_refreshed_quietly`. A stale refold with only the column NULLed gives one UPDATE per version, 0 whole rewrites, 0 change rows, `values_refreshed == versions`, and `verified + corrected == stamped`.
- Update project_golden.rs `off_and_shadow_write_the_same_for_every_stale_chain_shape` and project_incremental.rs `an_epoch_forced_rewrite_reproduces_identical_content`, and keep satellite_column_migration.rs `leaf_tables_match_the_schema_fresh_and_migrated` green.
- project_incremental.rs `an_append_fills_an_epoch_4_tenders_kept_versions_and_leaves_an_epoch_5_tenders_alone`.
- api.rs `an_earlier_versions_unelected_figure_reads_outside_a_value_bound`, with the issue's reproductions on /v1/tenders and /v1/lots:
  - max_value=50000000, procedure 900k / lot 300k, then 400k / 300k: `added`;
  - min_value=50000000, 400k plus a BT-709 1,000,000, then 600k: `added`;
  - the EUR 200 bn refused-figure variant: `added`;
  - an EUR-0 earlier version under max_value: `added`;
  - the same chain with `projection_epoch` set to 4 after the fold still gives `changed` (the fallback).
  Also update the 561 sentence in `a_version_that_moves_the_value_across_a_bound_is_removed_and_added`.
- read.rs `at_probe_plan_tests`:
  - new `a_gated_version_probe_walks_no_amount_or_lot_rows`: EXPLAIN shows no `va` or `vr` loop when the epoch is >= 5;
  - 547's 2,400-row timing fixture, run on a gated probe;
  - the existing `vl` full-key fault still holds.
- head_election_agreement.rs `the_version_value_has_one_writer`, beside `the_derived_head_marker_has_one_writer` at :1120.
- project_golden.rs `a_rederive_eur_walk_then_its_fold_equals_a_fold_under_the_new_rates` covers the column: the walk stamps 0 and the requeue fold refreshes the value quietly.
- Golden re-bless. Record the fold wall time on the golden run before and after the change (one extra election per version).
Then run `ops/check.sh` (the epoch bump re-hashes nothing, so expect no auto-clean) and read GATE-EXIT.
- Unit 5: deploy, canary, close (S). Status: blocked. Blocked by: unit 4.
(1) Deploy, and push by explicit ref.
(2) Before the next daily project runs, run a canary R3: `refold` by about 1,000 tender ids, spread over multi-version and multi-lot Tenders, a 551 derived head, a 537-refused head and a rederive-eur-touched Tender. Then `project`. Pass: `corrected` is 0 (any non-zero must be explained by a non-value table in the compare tally), `values_refreshed` is > 0, and `verified + corrected == stamped`. If it fails, roll back. The old binary stamps 4, so readers fall back to the superset, and at most about 1,000 Tenders were over-announced.
(3) Read the first daily project's counts line: `corrected` within its usual range, `values_refreshed` about equal to the versions of the epoch-4 Tenders it touched.
(4) Bounded reads, at most 3, each plan-checked on a copy of plan.db: in one 100k-id window, the epoch-5 Tenders where `v.value_eur_cents` at `current_seq` IS NOT `current_value_eur_cents` must be 0.
(5) Set Status to done.
- Unit 6: fill the residue on the next all-profile refold (S). This is a step in operations.md's all-profile-refold checklist, written by unit 4. It is not a board item, and 561 does not wait on it. When the next corpus-wide refold runs for any reason (ADR-0017 D6), it fills every Tender still at epoch 4. Afterwards: the R4 completeness read (`projection_epoch <> 5`, or the current epoch, over 100k windows) reads 0; then remove the 'written before <deploy date>' residue clause from CHANGELOG's 547 bullet and from /docs line 498. Keep the upsert advice, because version:null rows, 560 and 562 still need it. The superset fallback code stays.
- Unit 7: file a follow-up (S). Status: ready-for-agent. Blocked by: nothing. File issue 565 (564 is taken): a superseded version's served `value`, for example in an SSE catch-up payload with `include_data`, is picked as the amount row whose `eur_cents` equals the HEAD's value (read.rs:2584-2592, `s.eur_cents = t.current_value_eur_cents`), not as that version's own elected figure. It is null or wrong whenever the version elected something else. Once unit 4 lands, the pick can match `v.value_eur_cents`, with an api test of a two-version chain whose v1 elects a different figure.

### Verify

grep -c 'tender_versions ADD COLUMN value_eur_cents' crates/store/src/lib.rs; grep -c 'fn an_earlier_versions_unelected_figure_reads_outside_a_value_bound' crates/app/tests/api.rs

- **done**: prints `1` and `1`, the build is deployed, and unit 5's canary has read `corrected` 0, `values_refreshed` > 0, and 0 epoch-5 heads where `v.value_eur_cents` at `current_seq` IS NOT `current_value_eur_cents` in a 100k window. From then on the At probe reads each version's stored elected value. The superset is left only as the epoch-gated fallback, plus a documented residue: replay of changes written before the deploy, on Tenders not projected since.
- **open (2026-10-17, main 7e61f2b)**: prints `0` and `0`. The superset leg is live as `ELSE (EXISTS (SELECT 1 FROM tender_version_amounts va` (read.rs:1463, count 1). 551 re-spelled it, which is why the previous Verify grep printed 0 while the issue was still open, a false done.

### Adversarial check — amendments (adopted)

- BLOCKING for unit 4(e): pin `v` to its PK seek in both At shapes, for example `JOIN tender_versions v INDEXED BY sqlite_autoindex_tender_versions_1 ON ...` (INDEXED BY is already used in read.rs:2451 and lib.rs:7059). On plan.db this restores `(tender_id=? AND seq=?)` in all 12 gated shapes, including source/kind, and keeps vl on its full key (at561_ib.sql). Extend `faults()` to require `SEARCH v ... (tender_id=? AND seq=?)`, with a control that fails without the pin. Do not use a `(SELECT x.value_eur_cents FROM tender_versions x WHERE x.rowid = v.rowid)` nudge: turso 0.7.2 panics on it ('Cursor not found', vdbe/builder.rs:1556). A scalar or EXISTS subquery on tender_versions just moves the walk into the subquery.
- BLOCKING for the tests: replace the EXPLAIN-based `a_gated_version_probe_walks_no_amount_or_lot_rows` with a runtime pin. Use the 2,400-row fixture: the gated probe at epoch 5 must be within a small factor of a head probe, with epoch 4 as the control (measured 0.004 s against 0.10 s). Keep faults() requiring the va/vr legs, which the fallback still carries.
- BLOCKING for the residual (preferred, S): record the change cursor when the column migration first runs, for example in projection_state. In sse.rs `start()`, answer a resume of any value-bounded stream (/v1/tenders or /v1/lots with min_value or max_value) whose cursor is below it with `reset` (reason e.g. `value_probe_changed`). The deploy drops every connection, so every subscriber resumes. After that, value-bounded subscribers read only post-deploy rows, and those rows are written by folds that fill the Tender in the same transaction. That closes the 4->5 straddle for the lazy fill and for unit 6, and the 'written before <deploy date>' replay residue becomes unreachable for these streams. Add an api test where a pre-cursor value-bounded resume gets reset and an unbounded resume does not. If this is rejected, the D5 amendment, /docs line 496 and CHANGELOG must instead state the residual accurately: it under-delivers (a kept ghost or a lost removed), its window is the lagging subscriber's catch-up between that Tender's rows N and N+1, it stays open for every unfilled Tender until unit 6, and it recurs on unannounced 0->5 re-stamps. Add it as a third exception to 'final state exact' and qualify 'always arrives as removed'.
- Unit 4(c)/(tests): when a Tender has values_refreshed > 0, return its id (or check every compared Tender) so that the `v.value_eur_cents` at current_seq IS `t.current_value_eur_cents` check runs on quiet-only Tenders. Run that check after the batch flush.
- Fix the expected counts. In the test and in unit 5(3), `values_refreshed` should equal the versions whose fresh elected value is non-NULL, about half of all versions. Unit 5(4): run the bounded head check on the newest 100k-id window, where filled Tenders cluster, and plan-check it, since the v join will itself prefer autoindex_2 unless pinned.
- Unit 4(g): add an exact ("tender_versions", "value_eur_cents") COLUMN_NOTE: the Tender's elected value at that version, the same as current_value_eur_cents when that version was the head, including 551 lot sums but with no per-version marker. Not indexed. NULL means nothing elected, or not filled while t.projection_epoch < 5. Alternatively, rename the column (e.g. `elected_value_eur_cents`) so the wildcard note cannot apply.
- Unit 5(1): before deploying, check that the boot log has no `TENDER_REFOLD_COMPARE=` warning and no 'compare refused (issue 495)' line. Ops note: while any Tender is at epoch 4, Off or Shadow means a history replay for each one touched. Roll back the binary instead of flipping the switch.
- Unit 4 scope decision for 565: either add the 551 marker as quiet per-version columns in the same epoch bump (value_derivation, value_cents, value_currency; refreshed by the same quiet rule, excluded from equality), which avoids a second epoch bump and lazy fill and enables a bounded prod check that a non-NULL non-head value matches an amount row of its version or carries a derivation; or have unit 7's issue text say that a derived superseded version serves `value: null`.
- ADR-0017: besides D1.4/D3/D5, amend D6/D7 to name the route actually taken: the epoch bumped without an R4, completeness not required because the reader is epoch-gated, and the fill riding the next R4. In the ops R2 template, E4 should name tender_versions.value_eur_cents as a derived stored value that an in-place walk must re-derive or stamp 0 for. Unit 4(a): add the column to canonical.rs's CREATE TABLE tender_versions in the same commit as the ALTER (the issue-372 lesson). Unit 4(b): the shrink path (keep == p.versions.len()) runs no write loop, so the head UPDATE must still elect there and not reuse a last-version value.
- CHANGELOG/docs narrowing (units 4 and 6): the residue clause must also cover Tenders awaiting a refold after an epoch-0 stamp (rederive-eur, merges, scoped refolds) and a rollback window, all over-delivery only. Unit 3 wording: 'or, under min_value, the sum of its lot amounts, or separately the sum of its lot awards, each counted from EUR 0 to the cap plus one cent a row, reaches the bound (issue 551)'.
- Record the evidence: prod reads used 2 of 5 (q0: top tender id 8,848,695; q1: newest-window version tail and 0 Tenders at epoch 0). Plan and timing files are under /tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/d56x/adv561/ (at561.sql, at561.plan, var.sql, fix.sql, fix2.sql, fix3.sql, at561_ib.sql, time.sql, fx.db, q0.out, q1.out). Nothing in the repo was edited.

### Adversarial check — objections (for the record)

- The accepted residual is the forbidden failure, and its window is described wrongly. Here is how it happens. A lagging subscriber reads row N of a Tender while N is no longer the head and the Tender is at epoch 4 or 0. The new side then reads the superset, which over-reads, so the subscriber gets a false added/changed. The fill then commits. When the subscriber reads row N+1, the old side at N is exact (outside), and if N+1 is outside too the result is (false,false) -> `continue` (sse.rs). The subscriber keeps a ghost Tender until that Tender enters the bound and leaves it again. Before 561 the superset was consistent across both reads, so row N+1 delivered `removed`. The window is not milliseconds. It is the time the subscriber takes to catch up from row N to row N+1, which is unbounded for a resumed cursor. Under the lazy fill it stays open for every unfilled Tender until unit 6, and unit 6's R4 fills millions of Tenders while subscribers may be catching up. It recurs on any 0->5 re-stamp that is not announced (scoped refolds). rederive-eur and resolver re-stamps are safe, because the seq-less correction rows they write later deliver `removed` through the in-place (false,false) arm. The change also makes /docs line 496 false ('the diff stream then makes your final state exact, with two known exceptions') and CHANGELOG's 'a Tender whose current version leaves a bound always arrives as removed'.
- The claims of 'no extra seek' and 'O(1)' are wrong for `v`. I measured on a copy of plan.db with the column added: in all 12 gated shapes (tenders/lots/lots+kind/tenders+source × max/min/band), turso 0.7.2 moves `v` from `sqlite_autoindex_tender_versions_1 (tender_id=? AND seq=?)` to `sqlite_autoindex_tender_versions_2 (tender_id=?)`, which walks every version of the Tender on each probe. Today's lots At shapes seek the PK. Any non-key `v` column triggers it (v.published_at does too), so the real tenders At probe, whose SELECT list names v.published_at/publication_id/notice_subtype, very likely walks already today. The tail is real: in the newest 100k-id window on prod (8,748,696-8,848,695) there are 98,221 Tenders, 23 with 30 or more versions, and max current_seq is 622. `at_probe_plan_tests::faults()` checks va/vr/vd/vl but never `v`, so the planned gate would pass this unseen. Evidence: /tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/d56x/adv561/at561.plan, var.sql, /tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/d56x/adv561/q1.out.
- The test `a_gated_version_probe_walks_no_amount_or_lot_rows` (EXPLAIN shows no va/vr loop when epoch >= 5) cannot be written. EXPLAIN QUERY PLAN is static, the fallback legs stay in the SQL, and the existing faults() requires the va and vr legs to be present. The guard does work at runtime. On a 2,400-amount-row fixture, a non-head probe at epoch 5 takes 0.004-0.005 s, against 0.10 s (max_value) and 0.06-0.09 s (min_value) at epoch 4 and with today's SQL, and the results are correct. That is a timing pin, not a plan pin.
- Unit 4's `assert_heads_match` extension misses the main fill path. Under compare On, a Tender with only a quiet refresh (`acting && head_equal`, no whole rewrite, nothing appended) returns `chain_moved.then_some(id)` = None, so the batch check never sees it. Most lazy-fill Tenders and most of unit 6 take this path.
- Expected counts are wrong. Stored NULL equals fresh NULL, so a version with no electable value is not refreshed. `values_refreshed` is therefore about the versions with a non-NULL elected value (54% in COMPLETE's 3-window sample), not 'about equal to the versions' (unit 5(3)), and not `== versions` in `a_value_only_difference_is_refreshed_quietly` unless every fixture version carries a value.
- /v1/sql would describe the new column falsely. tender_versions is queryable, and the wildcard note ("*", "value_eur_cents") in sql.rs:1254 would apply. That note describes a lot value with a partial index and says 'award results are never elected', which has been false for the Tender election since 551's path W. It also does not say that NULL can mean 'not filled' (t.projection_epoch < 5).
- The kill switch turns into a history replay for as long as the 4/5 split lasts. Under TENDER_REFOLD_COMPARE=off or shadow, or with `tender_version_bid_parties_version` missing (refold_compare_for_fold returns Off), a stale Tender takes keep=0 and runs `append_version_changes` for every version. That is the pre-495 replay. A2 keeps epoch-4 Tenders around for months, so flipping the switch, or losing that index, would replay the full history of every epoch-4 Tender the daily project touches. The decision says nothing about this.
- Unit 7 (565) as specified only partly works for 551. A derived superseded version has no amount row carrying S, so picking by `v.value_eur_cents` still serves null with no derivation, while the head serves current_value_cents/currency. Without a per-version derivation marker, prod also cannot check stored non-head values (a derived value cannot be told apart from a bad one).
- The narrowed CHANGELOG/docs residue ('written before <deploy date>, on a Tender not projected since') leaves out Tenders that are temporarily back on the superset after an epoch-0 stamp: the daily rederive-eur-recent cohort, resolver merges, scoped refolds, and a rollback window. These are over-delivery only, but the text has to say they exist.
- Minor inaccuracies in the rationale. Correction rows' labels do depend on the old side: (false,true) gives added and (true,true) gives changed. That does not matter, because head-1 is not the state before the correction anyway. '300-600 wrong-op events a day per subscriber' applies to an unfiltered value-only stream, and '×10 on lots streams' was not measured. Neither changes the choice of A over B. Reach was measured adequately: the two methods agree, there were 0 false negatives, and the per-day volume of about 3k transitions is consistent.
- Unit 3's wording merges two separate legs. The >= leg compares the sum of the version's lot-level amount rows (any field, EUR 0 to cap, plus one cent a row) and, separately, the sum of its lot award results. The proposed parenthetical reads as one combined sum.

