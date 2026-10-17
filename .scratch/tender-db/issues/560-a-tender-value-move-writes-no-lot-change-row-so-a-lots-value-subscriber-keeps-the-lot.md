# 560 — a version that moves only the Tender's value writes no lot change row, so a `/v1/lots` value-bound subscriber keeps a lot its bound now excludes

Status: ready-for-agent — DECIDED 2026-10-17 (wf_3176baf8-d14, judge + adversarial check, amendments adopted): build A-EUR — a version whose elected Tender EUR value moved writes lot `changed` (and tender `changed`) rows beside 533's closure flip, with a net-move rule at the last written version and the stored-head seed for a pure append; no epoch bump, no refold. Units below (U-a filed as issue 565). NEXT: U-b (store), then U-c (api test + served text), U-d (deploy + coverage read crediting D5's seq-less rows).
over a real SSE socket). Next: decide between emitting lot `changed` rows on a Tender head-value move (533's closure-flip
shape) and documenting the gap as permanent; measure the extra rows first.
Kind: correctness (SSE `added`/`removed` on `/v1/lots`)
Relates to:
- 547: the SSE `Scope::At` probe now reads the Tender's value at the probed version (`ValueBy::Version`), on
  `/v1/tenders` and `/v1/lots` alike. The lots half only runs when a lot change row exists at that version.
- 533: the same gap for `status`, closed by adding `closed_before != closed_now` to the lot `changed` condition.
- 493: a lot's OWN served value can move without a lot row too (measured 0 of 14,480 on prod, so left documented).
- 490 decision 6: a lot's `min_value`/`max_value` bound reads its Tender's value.
- 561: the earlier side of the value probe is a superset; independent of this one.

## Verify

    grep -c 'fn a_tender_value_move_alone_reaches_a_lots_value_subscriber' crates/app/tests/api.rs

- **done**: 1 (the test exists and passes: a version that changes only the procedure total moves the lot in or out of
  a `/v1/lots?min_value=`/`max_value=` subscription), OR a `## Decision` below keeps the gap, with the CHANGELOG
  547 bullet still naming it.
- **open (2026-10-16)**: 0. CHANGELOG 547's `/v1/lots` bullet and the `ValueBy::Version` doc name the gap.

## What was found

A lots stream reads only `lot` change rows (`sse.rs` `diff`, `changes_since(.., Some(kind))`). The fold writes a lot
`changed` row only when `before.facts != lot.facts || before.kind != lot.kind || closed_before != closed_now`
(`canonical.rs` `append_version_changes`). The Tender's value is elected over procedure and lot figures
(`head_value_eur_cents_with`), so it can move while no lot fact does:
- a corrigendum of the procedure total only (TED R2.0.9 F14 correcting II.1.5);
- a sibling lot's figure moving the election in a multi-lot Tender (every other lot keeps its facts);
- a CAN's result value beside lot facts carried over unchanged ("a lot's facts seldom move on a CAN", 533).

Reproduced twice in the 547 review (scratch logs, 2026-10-16):
- synthetic R2.09 CN (procedure EUR 1,000,000, LOT-1 EUR 300,000) then an F14 correcting II.1.5 to EUR 400,000:
  Tender value 100000000 → 40000000 cents, change rows `tender:1@1:added lot:1@1:added tender:1@2:changed` (no lot row
  at seq 2). `/v1/tenders?min_value=50000000` got `removed`; `/v1/lots?min_value=`, `?max_value=` and the unfiltered
  lots stream stayed silent, while the pages flipped.
- the eForms chain with lot-scope parties stripped and only `RequestedTenderTotal` changed: tenders min/max got
  `added`/`removed`, every lots tape `None`.

eForms lots that name a lot-scope party carry a per-notice `Fact::Party`, so they get a lot row on every version and
hide the gap (the api.rs chain fixtures do).

## Units

1. **Measure (S).** On prod, a bounded windowed read: consecutive versions whose `current_value` would differ (or a
   proxy: tender `changed` rows with no lot row at the same seq, for Tenders with lots). Size the extra lot rows that
   option A would write per daily append.
2. **Decide (S).** A: add a Tender head-value move to the lot `changed` condition — needs the elected value of the
   previous and this version in `append_version_changes` (the fold carries a running `ScalePartners`; the previous
   version's value is the previous iteration's). B: keep the gap documented.
3. **Build (M)** if A: the condition, an api test (the Verify's name) that changes only the procedure figure, and
   retire the CHANGELOG 547 lots caveat.

## Decision (2026-10-17, workflow wf_3176baf8-d14)

Two independent designs (minimal / complete), a judge, then an adversarial check (sound: True); the check's amendments are ADOPTED and override the judge's text where they differ. Full record: `.scratch/tender-db/56x-decisions/wf_3176baf8-d14_result.json`.

### The decision

Build A-EUR in BOTH arms of append_version_changes, taking COMPLETE's shape with MINIMAL's cost frame. In the fold's existing per-Tender pass, carry the previous version's elected EUR value. Pass `value_moved` (NULL<->value counts as a move) into append_version_changes and add `|| value_moved` to the lot arm (lots present on both sides) and to the tender arm, beside 533's `closed_before != closed_now`. Compare EUR cents, not the published figure, because the bound compares EUR. Option 2 is rejected: it saves about a quarter of the rows, changes the return type of the election 551 deployed hours ago, and leaves crossings caused only by the exchange rate unprobed for good. B is rejected: on 2026-10-15 a value-bound lots subscriber held 119-145 lots wrongly at EUR 100k/1M, every day, and a resume never re-snapshots. That fails 493's 'negligible reach' test (493 measured 0 of 14,480). A-wide is deferred to its own issue rather than slipped in under 560.

The tender arm is in because it costs nothing and closes a real hole. All 5,512 version pairs on 10-14/10-15 already had a tender row, so it adds about 0 tender rows. It matches 533, which put closure in both arms. Without it, 547's promise that 'a Tender whose current version leaves a bound always arrives as removed' is false for a version whose facts, rounds and groups are identical while a non-EUR value moves with the rate.

No PROJECTION_EPOCH bump, no refold, no backfill. No stored column changes. The change log is append-only, and the refold and rederive paths already announce head differences (ADR-0017 D3 rule L announces every head lot when the head differs; D5 for rederive). So a refold of millions of Tenders would buy nothing here.

Cost: about 1.1k-2.8k extra lot `changed` rows per daily append. That is +12-18% on daily-shaped windows; the +59% in W2 was a fold batch, not a day. About 1M rows a year against a 955M-row log. Fold cost is one in-memory head election per written version.

Two things neither design covered, and the build must include both:
(1) /docs Live feed (crates/app/src/v1/docs.rs:496) and the OpenAPI EventStream description (crates/app/data/openapi.json:1153) list 560 as a known exception, and the docs.rs test at ~2038 asserts "(issue 560)" is there. All three change.
(2) Those same surfaces say 'the diff stream then makes your final state exact' apart from 560 and 562. But version_predicates reads country, cpv, buyer, winner, bidder, procedure_type and the deadline at the Tender version (tid, seq), not per lot. So on /v1/lots those filters miss a move whenever no lot row is written. Retiring 560 must not leave /docs claiming more than is true: the retired exception is replaced by an honest one that names a new follow-up issue.

### Units

- U-a (S, docs only; Blocked by: none). File the follow-up issue at the next free number: 'a Tender-scope filter on /v1/lots (country, cpv, buyer, winner, bidder, procedure_type, currency, status/deadline) re-evaluates a lot only when the lot's own row is written'. Evidence: version_predicates in crates/store/src/read.rs:1268-1330 reads (tid, seq). For example, a second CAN that names a winner on an already-closed Tender, with the lot facts unchanged, writes no lot row, so /v1/lots?winner= misses it. Starting measurements are A-wide's: W1 332 versions / 3,619 uncovered lots (56 versions with no lot row at all), W2 3,628 / 17,012 (2,794). Name A-wide (every surviving lot `changed` whenever the tender row is written) and the SSE fan-out as the options, with 493's cost argument against A-wide. Status ready-for-agent, Relates to 560/533/493/547. Verify: the file exists and `grep -c 'version_predicates'` on it is at least 1.
- U-b (M, crates/store only; Blocked by: none). Fold change in crates/store/src/canonical.rs.
(1) In the write loop (~22846-22897), hoist `let rates = self.rates_lookup();` and carry `prev_value: Option<Option<i64>>`, where the outer None means no previous version. When keep > 0, seed it after the prefix's add_version_figures with `partners.set_head(&p.versions[keep-1])` and `head_value_eur_cents_with(&p.versions[keep-1], &partners, &rates).eur_cents`. That equals of_chain(prefix), because set_head is a full overwrite (line 4429).
(2) After `partners.set_head(version)`, compute `value_now` the same way. Do NOT call it `now`: `now` is already the timestamp argument. Skip the computation only when `acting && i + 1 < compared` (no transition row reads it), so the first appended version after an On-mode compared prefix is compared against the prefix's last value. Set `value_moved = prev_value.is_some_and(|p| p != value_now)`, then `prev_value = Some(value_now)`.
(3) Give append_version_changes (41189) a `value_moved: bool`. Add `|| value_moved` to the tender `changed` guard (41221) and to the lot arm (41248).
(4) Leave the post-loop head UPDATE's election untouched: one head writer, no coupling.
(5) Rewrite the 493/533 comment at the lot arm: a value move is now the lot's own change, as closure is. Keep 493's note that a lot's OWN value can still move without a row.
Tests:
(a) Extend `the_running_scale_rule_is_of_chain_of_each_prefix` and add `the_running_head_value_is_head_value_of_each_prefix` beside it. The running value must equal `head_value(&chain[..=n], &rates).eur_cents` for every n, including a keep>0 seed: add_version_figures over the prefix, then set_head(prefix.last()), must equal of_chain(prefix).
(b) New crates/store/tests/value_move_change_rows.rs in the style of closed_by_notice.rs, with `a_tender_value_move_is_a_change_row_for_the_tender_and_its_lots`. Cases: procedure total only (R2.09 CN EUR 1,000,000 with LOT-1 at EUR 300,000, then an F14 setting II.1.5 to EUR 400,000) gives `lot:LOT-1@2:changed`; a sibling lot's figure moving the election gives a row for the unmoved lot; NULL->value and value->NULL; an identical figure restated with identical facts gives no lot or tender row; a GBP chain whose two versions differ only in published_at, rates seeded through `upsert_currency_rates` + `reload_rates_lookup` at two different rates, gives tender AND lot `changed` at seq 2; an append with keep=1 (fold v1, then v1+v2) exercises the seed.
(c) `an_on_mode_refold_compares_the_first_appended_version_against_the_prefix_value`: a stale Tender under RefoldCompare::On whose compared prefix keeps its value, plus one appended value-moving version, writes exactly one lot `changed` at the appended seq and no transition rows for the prefix.
(d) Run fold_writer_golden.rs and project_golden.rs. If crates/store/tests/fixtures/golden/fold_writer.snapshot or crates/ingest/tests/fixtures/golden/project_{apply,refold}.snapshot move, regenerate them on purpose and review the diff: it may contain only inserted `lot|changed|<seq>` / `tender|changed|<seq>` rows and the matching `changes=` counts. The commit message names the regeneration.
Gate with ops/check.sh, read GATE-EXIT=0 from the redirected file, and stage named files after `git diff <file>`.
- U-c (S-M, app + docs; Blocked by: U-a for the issue number, U-b for the behaviour).
(1) In crates/app/tests/api.rs, add `a_tender_value_move_alone_reaches_a_lots_value_subscriber`. Use an R2.09 CN (procedure EUR 1,000,000, LOT-1 EUR 300,000), then an F14 correcting II.1.5 to EUR 400,000, then an F14 back to EUR 1,000,000. No lot-scope party, so no lot fact moves. Expected tapes over a real SSE socket:
- `/v1/lots?min_value=50000000`: removed at v2, added at v3;
- `/v1/lots?max_value=50000000`: changed at v2 (the documented 561 superset, applied as an upsert), removed at v3;
- unfiltered `/v1/lots`: changed, changed;
- the `/v1/tenders` min/max tapes: unchanged from today's behaviour.
Also assert that `/v1/changes?kind=lot` carries `lot changed` at seqs 2 and 3.
(2) Retire the gap wherever it is named:
- the docs.rs:496 /docs Live feed and the openapi.json:1153 EventStream description: drop the 560 exception and replace it with the U-a issue's honest exception for Tender-scope filters on /v1/lots. Do not reuse the phrase 're-evaluates no lot'. Add that a lot `changed` can arrive when only its Tender's value or closure moved;
- the docs.rs test (~2038): drop "(issue 560)" from the must-say list, require the U-a issue's tag, and assert that neither surface contains 're-evaluates no lot';
- read.rs ValueBy::Version (~1208-1212): replace the 'Probed: ... (issue 560)' sentence;
- the doc comment on `a_version_that_moves_the_value_across_a_bound_is_removed_and_added` (api.rs:3935-3937).
(3) CHANGELOG.md: delete 547's /v1/lots bullet (~89-93) and add `## Unreleased (issue 560)`. It says:
- from the deploy on, a version that moves only the Tender's EUR value writes a `changed` row for the Tender and for each lot present on both sides, so lots value-bound streams hear `removed`, and `added` or `changed` (applied as an upsert);
- unfiltered lots streams, `/v1/changes` polls and lot webhooks get those extra `changed` rows (about 1-3k a day), including moves caused only by the rate on non-EUR Tenders;
- change rows written before the deploy keep the gap, so a lots value-bound client holding state from before the deploy should re-subscribe once without a cursor.
Gate with ops/check.sh, GATE-EXIT=0.
- U-d (S, ops; Blocked by: U-b, U-c). Do an ordinary deploy of the gated rev: no migration, no refold, no backfill, no epoch bump. Confirm `git ls-remote --heads origin main`. After deploy, run one bounded read: re-run /tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/d56x/q3_1014.txt (plan-checked first on a COPY of v551/plan.db) with its published_at window moved to the first publication day folded entirely after the deploy. Bar: `me_lots` (silent lots behind a moved EUR proxy) falls from the pre-deploy 1,069-1,268 to 25% of that or less. Read the 3 largest residue versions and show each keeps its elected value, i.e. the moved maximum is a refused, partner-only, Part or derived-sum figure, the proxy's documented blind spots. A residue version whose elected value DID move is a stop. Record the numbers in 560 and close it.

### Verify

Done means all of these hold:
- `grep -c 'fn a_tender_value_move_alone_reaches_a_lots_value_subscriber' crates/app/tests/api.rs` = 1, and the test passes inside an ops/check.sh run whose redirected output reads GATE-EXIT=0 at the deployed rev;
- `grep -c 're-evaluates no lot' crates/app/data/openapi.json CHANGELOG.md` = 0 for each file;
- `grep -c '^## Unreleased (issue 560)' CHANGELOG.md` = 1;
- the U-a follow-up issue file exists;
- after deploy, the q3 read for the first publication day folded entirely after the deploy shows me_lots at 25% or less of 1,069-1,268, with the 3 largest residue versions read as unchanged elected values.
Open (2026-10-17): the api test count is 0 and CHANGELOG 547's /v1/lots bullet names the gap.

### Adversarial check — amendments (adopted)

- U-b: ADD A NET-MOVE RULE AT THE LAST VERSION THE LOOP WRITES. Keep `run_seed` = the value before the first version written in this run: the keep>0 prefix value, or the compared prefix's last value in On mode. When i + 1 == p.versions.len() and a seed exists, set value_moved |= (value_now != run_seed), so the head itself is probed whenever the run's net value moved. Tests: (i) one fold run that appends v2 (EUR 1M to 2M) and v3 (2M) writes `lot:LOT-1@3:changed`; (ii) an api.rs variant where v2 and v3 fold in ONE batch before the subscriber reads. Starting from 400k, then 1M, then 1M under /v1/lots?max_value=50000000, it must end with `removed`. Without the rule it ends with `changed`.
- U-b test (c): use a compared prefix of two or more versions with different values, for example [1M, 400k]. Assert that appending 1M writes a lot row, and that appending 400k writes none. This catches a comparison against None or against a stale earlier prefix value in both directions.
- U-b: for a pure append (keep == stored.len(), !stale), also set value_moved on the first appended version when stored_identity.head.value_eur_cents != value_now. The value is already read and costs nothing. Do not hold the whole HeadValue across an await, only the Option<i64>: apply_tender_tx's future is already stack-sensitive (the goldens run on_a_big_stack). Reuse the hoisted `rates` Arc in the post-loop head UPDATE. The rates stay the same Arc, and the election logic stays separate.
- U-b (d): review fold_writer.snapshot with the rowid and cursor columns stripped. Compare the kind|entity_id|seq|op sequence of each phase, and require it to differ only by inserted lot/tender `changed` rows, with matching changes= and row-count headers. The commit message records that check.
- U-d: change the read. (a) Count a silent lot as covered when a seq-less lot `changed` row follows within the next morning's rederive window. Report seq-row coverage and D5 coverage separately (the q_adv.sql encoding does this in one 4 s pass). (b) Set the bar on lots that neither path covers, against the pre-deploy baseline measured the same way: about 257 lots on 2026-10-14 (TED EUR-only 122, TED non-EUR 32, FTS 103). The rough bar 'falls to <=25% of 1,069-1,268' is out of scale once D5 is credited. (c) Treat a residue version as a stop only when its value moved at FOLD-TIME rates. A move that only the next morning's rederive introduced is a D5 case, not a defect. (d) Choose a publication day whose D5 morning has no refold or drain running.
- Record of 560 and the CHANGELOG (U-c): restate the reach. Before the deploy, TED heads with a non-EUR amount were already re-announced to lot streams by the next morning's rate rederive (D5). The gap that persisted was EUR-only Tenders and FTS, about 250 lots a day in measured silent lots, not 1,069-1,268. Qualify 'including moves caused only by the rate': a TED version published today is compared at yesterday's fixing, and the next morning's rederive still re-announces its lots. Keep the advice to re-subscribe once, for EUR-only Tenders and FTS.
- U-c /docs and OpenAPI: do not claim that the diff stream makes the final state exact under min_value/max_value for a client that resumes or lags past a version since superseded. Both probes then read the superset, and a `changed` can name an entity outside the bound until a later change probes the current version. Name 561 for this. Record the in-run and cross-run superset artifact in 561 with the 151-of-2,751 same-day figure.
- U-a: say that its starting numbers (W1 3,619 / W2 17,012 uncovered lots) include lots A now covers, and do not credit D5 or rule-L seq-less corrections. Re-baseline after U-d before A-wide's volume is weighed.
- Cost note in 560: an Off-mode or Shadow epoch refold, and a rebuild, now writes value-moved lot rows for every historical version. That is on the order of W2's +59% lot rows, or O(versions x lots) on long non-EUR chains. On mode is unaffected beyond one election at the compared prefix's last version.

### Adversarial check — objections (for the record)

- REACH IS OVERSTATED ABOUT 5x FOR THE PERSISTENT ERROR, AND NEITHER DESIGN CREDITED SEQ-LESS ROWS. Both measurements count a lot as silent only when no lot row exists at version_seq = seq. But the daily fold converts a TED version published today at yesterday's fixing (docs/operations.md:307-311). The next morning, rederive-eur-recent re-derives it, and inplace.rs Moved::announce writes ADR-0017 D5 rule-L seq-less `lot changed` rows for every lot of a head that has a moved row. So almost every non-EUR TED head is re-announced, all of its lots, about a day later. Measured on prod (1 bounded read, 3.8 s; plan-checked; the query is in /tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/d56x/adv/q_adv.sql, the output in q_adv.out). For 2026-10-14 TED, 1,011 of the 1,165 me_lots got a seq-less lot `changed` on 10-15, and 1,009 of those 1,011 sit on versions with a non-EUR amount. Of the 124 EUR-only lots, 2 were covered. All 103 FTS lots are uncovered: FTS is folded the day after publication with the correct rate, so D5 never moves it, and these moves are rate-only (mp_lots is 1). The gap that persists forever is about 124 + 30 + 103 = roughly 257 of 1,268 lots a day (~20%), not 1,069-1,268. The '119-145 lots/day held wrongly and piling up' figure is mostly next-morning-corrected for TED. The decision still holds: about 250 lots a day still accumulate, far beyond 493's 0 of 14,480. But the rationale, the 560 record and the CHANGELOG rest on the wrong framing. For most TED non-EUR rows, A buys latency (next morning becomes immediate). For EUR-only and FTS Tenders it buys correctness.
- A ALONE DOES NOT CLOSE THE GAP WHEN SEVERAL VERSIONS OF A TENDER ARE FOLDED IN ONE RUN. ValueBy::Version is exact only at t.current_seq. At any other version, both At probes read the superset (read.rs ~1186-1230 and 1424-1470). apply_tender_tx writes all of a Tender's appended versions in one transaction, so SSE probes every non-last appended version superset-vs-superset, every time. Under max_value on a multi-lot Tender the superset is almost always true, so such a row classifies as (true,true) `changed`. An in-to-out move (400k to 1M, then 1M) never sends `removed`, and an out-to-out move (1M to 2M, then 2M) upserts a lot that is outside the bound. The run's last version writes no lot row because its value did not move. Measured (1 bounded read, 2.0 s; d56x/adv/q_run.sql and q_run.out): on 10-14, 151 of 2,751 later TED versions were followed by another version of the same Tender published the same day. 17 TED and 9 FTS value-moving versions were followed that day by a version that did not move the value: 83 + 21 lots, about 10% of A's rows and about 40% of the persistent reach. U-c's api test and U-b's store tests fold one version at a time, so they cannot see this. The cross-run version of the same artifact (a client that resumes after its Tender has moved on) is 561's. Today /docs says that applying `changed` as an upsert makes the state right, which is false in that case.
- THE U-d BAR AND ITS STOP RULE WILL MISFIRE. q3 counts silent lots by seq rows only and reads stored eur_cents after the rederive. After A, a TED non-EUR version whose previous version was published the day before is folded with that same D-1 fixing on both sides, so the fold sees no move. The next morning the rederive moves it, and D5 covers its lots with seq-less rows. q3 will report those lots as residue whose elected value DID move, which is U-d's stop condition, although nothing is wrong. Separately, the 10-16 drains (550 project 2531, 2563/2564, 551's 2573-2575) wrote millions of seq-less corrections. Any day whose D5 morning overlaps a drain makes a 'covered' count meaningless.
- THE GOLDEN REVIEW RULE CANNOT BE CHECKED AS WRITTEN. fold_writer.snapshot digests the change log with rowid and cursor as its first columns (table_digest(db, "changes", "cursor", ...); rows look like `5|5|'tender'|1|3|'changed'`). One inserted row renumbers every later row of that phase, so the line diff will not be 'only inserted lot|changed / tender|changed rows'. project_golden's change digest has no cursor (kind|op|seq|entity_id), so a line diff works there. Its Off-mode refold replays the whole history, so expect more inserted rows than the append phases suggest.
- OFF AND SHADOW REFOLDS AND REBUILDS GET MORE EXPENSIVE, AND THE DECISION DOES NOT SAY SO. With TENDER_REFOLD_COMPARE=off or shadow, and on a rebuild, append_version_changes runs for every version of every Tender. A then writes value-moved lot rows across all of history, on the order of W2's +59% lot rows, or O(versions x lots) on long non-EUR chains, plus one election per version everywhere. On is the default, so this does not block, but the cost and ops notes claim 'negligible' only for daily appends.
- THE SEED FOR A PURE APPEND IS RECOMPUTED, NOT READ FROM WHAT SUBSCRIBERS HOLD. When keep == stored.len() and the Tender is not stale, the stored head value (StoredIdentity.head.value_eur_cents) is already read, free, by tender_identity's SELECT. It is what every snapshot and head probe saw. A recomputed of_chain(prefix) value differs from it whenever the election or the rates changed without a refold of that Tender, for example a head that 551's walk did not stamp. In that case A can skip a row that a subscriber needs. This is minor and rare.
- THE RATE-ONLY SHARE IS INCONSISTENT ACROSS THE RECORD. The judge leans on COMPLETE's W1 figure of 5.5% (all native figures identical). MINIMAL's me-not-mp measure on 10-14 is about 27%: FTS 102 of 103 lots, TED 237 of 1,165. 'Exactness is cheap' holds either way, in absolute rows. But the record should not state 5.5% as the share.

