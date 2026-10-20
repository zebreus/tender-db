# 562 — a whole-Tender rewrite of `source` or `kind` (a DÖE Tender turning `ted`) is invisible to an SSE subscription on it

Status: DONE 2026-10-20 — unit 5 PASSED both halves (10-19: project 2645, 102 flips = 102 announced = relabelled 102; 10-20: project 2669, 57 flips = 57 announced = relabelled 57; TED-first 0, co-minted 0, kind moves 0 on both days; wf_c31642a7-759 and wf_d44820c4-5dc, each adversarially verified). Was: ready-for-agent — unit 5 FIRST HALF PASSED 2026-10-19 (the 10-19 daily fold, project 2645: 102 flips = 102 announced = relabelled 102; TED-first 0; co-minted 0; kind 0; wf_c31642a7-759, verified). NEXT: the second half on the 10-20 daily fold (same read), then close. Was: ready-for-agent — units 2–4 BUILT, reviewed (wf_250ada72-32e), GATED (900efeb) and DEPLOYED 2026-10-17 10:50 UTC as `900efeb` with 560 and 551 unit 4a. NEXT: unit 5 — the paired read over the next Monday/Tuesday DÖE→TED flips (2026-10-19/20 daily folds) against the fold line's `relabelled` count; then close.
read, or a seq-less tender `changed` when the label flips), then build.
Kind: correctness (SSE `added`/`removed` on `source` and `kind`)
Relates to:
- 547: listed `kind` and `source` as "probably harmless"; the build found they are rewritten for the whole Tender and
  left them on `t.*` (comment in `tenders_query`).
- 462: `kind` is re-derived from the fold-first notice.
- 495 / ADR-0017 D3: a stale refold (compare On) already emits a seq-less correction row, which the in-place path
  classifies `(false, false) => removed` correctly; an ordinary current-epoch fold does not.

## Verify

    grep -c 'fn a_ted_twin_joining_a_doe_tender_reaches_a_source_subscription' crates/app/tests/api.rs

- **done**: 1 (the test passes: a `source=doe` tape hears `removed` and a `source=ted` tape `added` when a TED twin
  joins), OR a `## Decision` below keeps the gap, documented in CHANGELOG.
- **open (2026-10-16)**: 0; the CHANGELOG 547 entry names the gap.

## What was found

`canonical.rs` (~22446-22457) rewrites `tenders.source` (`primary_source`: `ted` once any notice is TED) and
`tenders.kind` in place. `tenders_query` emits `AND t.source = ?` / `AND t.kind = ?` on both `Scope::At` probes, so
seq and seq − 1 read one label. Reproduced in the 547 review: CHAIN[0] as a DÖE notice, then CHAIN[1] as TED for the
same procedure → the Tender goes `doe` → `ted`, pages agree, `/v1/changes` holds only `tender 1 changed version 2` and
`lot 1 changed version 2`; the `source=doe` tape got nothing (10 s), the `source=ted` tape got `changed` (not `added`).
`kind` was not run; same mechanism.

## Units

1. **Decide (S).** A: source at a version = `ted` if any version up to that seq was caused by a TED notice (an
   EXISTS over `tender_versions` ⋈ `notices` keyed `(tender_id, seq <= ?)`; read its plan); kind at a version from the
   fold-first notice up to it. B: when the fold flips the label, emit a seq-less tender (and lot) `changed`, which the
   in-place path classifies by the head alone.
2. **Build (S–M)**, with the api test the Verify names.

## Decision (2026-10-17, workflow wf_3176baf8-d14)

Two independent designs (minimal / complete), a judge, then an adversarial check (sound: True); the check's amendments are ADOPTED and override the judge's text where they differ. Full record: `.scratch/tender-db/56x-decisions/wf_3176baf8-d14_result.json`.

### The decision

## Decision (2026-10-17): B. A fold that moves a Tender's label announces it with ADR-0017 D3's seq-less rows, and the read path stays as it is

**What to build.** In `Db::apply_tender_tx`, `tender_identity` already sets `StoredIdentity.moved` when it rewrites `source` or `kind` in place (canonical.rs ~23200-23216). Today only the acting compare-On stale path reads that flag (~23027). With this change, every fold that is not `acting` and has `moved` set writes D3's rows through `inplace::correction_rows`, last for the Tender:
- rule T, a seq-less tender `changed`;
- one seq-less lot `changed` for each lot of the head version.

This also covers the unchanged-chain early return. `acting` folds are left alone, because D3 already writes one set there. A rebuild never sets `moved`. No change to SQL, plans, schema or the read path. No refold, no drain.

**What subscribers see.** SSE's in-place arm (sse.rs:404-465) probes head and head−1 under the new label:
- A `source=doe` tape, or an old-kind tape, gets `removed` for the Tender and for each head lot.
- A `source=ted` tape gets `changed`.

**Verify changes.** The `source=ted` tape hears `changed`, not `added`. /docs already says to apply that as an upsert (ADR-0017 D4, and 547's decision for value bounds). The Verify's `added` is replaced by `changed`.

**Measured.** Both designs read prod, bounded, and agree:
- Visible DÖE→TED flips run at about 110-150 a week, all on Mondays and Tuesdays: 100 on 10-05, 44 on 10-06, about 58 on 10-12, 51 on 10-13. These are weekend DÖE publications whose TED twin arrives one daily cycle later.
- 147 flips with 225 head lots in the window of TED versions published 10-12 to 10-16.
- Every flip has a seq-carrying tender row at the TED version.
- Today each flip leaves a permanent ghost in a `source=doe` subscription.
- Kind moves: 0 in 12,794 later versions.
- At most 1,128 of the 1,275 DÖE+TED merges in that window were co-minted or minted TED-first, so `moved` stays false and B never touches them (a TED notice fetched before, but parsed after, a DÖE-only fold is a flip the created_at criterion files as co-minted; B announces it, correctly).
- B's cost is at least about 370 extra seq-less rows a week.

**Not built:**
- **Option 3 (announced-label columns `source_prior`/`source_since`).** It only buys exact operations: `added` instead of `changed` on the new-label tape, and no extra `removed` on unrelated filtered tapes. The house bar for the diff stream is an exact final state, with over-delivered `removed` (164/287) and `changed`-as-upsert (D4, 547/561) as the accepted errors. Clients have to keep upsert handling anyway for `max_value` (561), so op-exactness for `source` alone saves them nothing. The cost is an M-sized build: a schema change on a public, allow-listed table, and a new invariant ("the label as announced at a seq") that every replay, every stale refold under Off/Shadow/On, and every rebuild must keep. Its failure mode is the error the house does not accept. A stale `source_since` after a later prepend to a flipped Tender gives a doe tape a fake `added` at v1, unless a clamp rule holds on every path. B can never send an old-label tape anything but `removed`.
- **A (per-version source read from notices).** It reports the 1,128 co-minted and TED-first merges as DÖE at v1. That is about 226 fake added/removed pairs a day, to close a gap of about 29 a day. It also puts an EXISTS join on notices, which the CASE does not short-circuit, on every source-filtered At probe. And it cannot fix kind.
- **C (keep the gap).** About 110-150 permanent ghosts a week, roughly 2% of weekly DÖE intake. Not negligible.

**No correction walk for past flips.** A single metrics read showed 0 live SSE streams, and webhooks and polls are unfiltered. A stream that resumes across the deploy keeps its earlier ghosts until it re-snapshots, and the CHANGELOG says so.

**Reopen if** the docs ever drop "apply `changed` as an upsert". Option 3's columns are the shape to build then.

### Units

- 1. Decide (S). Status: done 2026-10-17. Append this `## Decision` to .scratch/tender-db/issues/562-a-whole-tender-rewrite-of-source-or-kind-is-invisible-to-sse.md, with the measurements, the rejected options (3, A, C) and why, and the changed Verify. Status: ready-for-agent. Next: unit 2.
- 2. Store: write D3's seq-less rows when a non-acting fold moves the label (S). Blocked by: none.

Where: crates/store/src/canonical.rs, `Db::apply_tender_tx`.

Add one helper that writes `crate::inplace::correction_rows(tender_id, &head_lots)` as seq-less `changed` rows via `stmts.append_change(kind, id, None, "changed", now)` and returns `1 + lots` for `applied.changes`, so the doorbell rings. Call it in two places:
- (a) When `stored_identity.moved && !acting`, after the transition rows and the orphan sweep's `removed` rows, just before the final `Ok((applied, Some(tender_id)))`. The head lots are the lot ids the write loop resolved for `p.versions.last()`. An empty chain gives rule T alone.
- (b) In the unchanged-chain early return (~22766) when `moved` is set. Read the head lots with one seek, `SELECT lot_id FROM tender_version_lots WHERE tender_id=? AND seq=<stored current_seq>`. Plan-read it first on a COPY of scratchpad/v551/plan.db; it must SEARCH sqlite_autoindex_tender_version_lots_1 (tender_id=? AND seq=?).

Never call it under `acting`: the existing D3 branch at ~23027-23042 already covers that case.

Tests:
- `a_label_moved_by_an_ordinary_fold_writes_rule_t_and_the_head_lots_seqless`: a DÖE fold, then the TED twin on the current epoch. Expect the v2 transitions, then exactly one seq-less tender `changed` and one seq-less lot `changed` per head lot, counted in Applied.changes.
- `a_fold_that_leaves_the_label_alone_writes_no_correction`: a second DÖE version on a DÖE Tender.
- `a_ted_first_tender_gaining_its_doe_notice_writes_no_correction`: TED minted, then DÖE prepended (keep=0 replay).
- `a_co_minted_doe_and_ted_pair_writes_no_correction`.
- `a_label_moved_on_an_unchanged_chain_writes_the_correction`: if no fixture can move group.sources without changing the fold chain, record why in the issue instead of faking it.
- `a_stale_refold_that_moves_the_label_writes_one_correction_set`: compare On writes exactly one set; Off and Shadow write identical rows.
- crates/store/tests/fold_writer_golden.rs stays byte-identical.

Focused runs use the gate's flags and package set; run ops/check.sh before committing.
- 3. App: the Verify api test (S). Blocked by: 2.

Add `a_ted_twin_joining_a_doe_tender_reaches_a_source_subscription` to crates/app/tests/api.rs.

Setup:
- Ingest CHAIN[0] as a DÖE notice.
- Open four streams: /v1/tenders?source=doe, /v1/tenders?source=ted, /v1/lots?source=doe and /v1/lots?source=ted.
- Read their snapshots: the doe tapes hold the Tender and lot 1 as `added`, the ted tapes are empty.
- Ingest CHAIN[1] as TED for the same procedure.

Assert:
- The doe tenders tape hears the Tender `removed`, and the doe lots tape hears lot 1 `removed`.
- The ted tenders tape hears the Tender as `changed`, and the ted lots tape hears lot 1 as `changed`. Neither ted tape hears a `removed`.
- /v1/changes holds the v2 transitions, then one `version: null` tender `changed` and one per head lot.
- Pages agree: source=ted lists the Tender and source=doe does not.
- 4. Served text (XS). Blocked by: 2; ships in the same deploy as 3.
- crates/app/src/v1/docs.rs:496 (Snapshot): drop the 562 exception, leaving one known exception (560).
- docs.rs:498 (Diff): keep 'under source or (on /v1/tenders) kind a changed can name an entity you never received where added was due'. Restate why: a label move arrives as a `version: null` correction, which a new-label subscriber hears as `changed` (issue 562).
- docs.rs:545, the version: null list: add 'or the fold re-labelling a Tender's source or kind (a DÖE Tender turning ted)'.
- crates/app/data/openapi.json: the same edits in EventStream (line 1153) and in any description that lists the causes of `version: null`.
- crates/store/src/read.rs:3031: the comment now says the fold's correction rows close the gap, not the probe.
- Docs pin in `the_docs_say_a_date_bound_implies_a_sort_on_pages_only` (docs.rs:2039): replace the bare '(issue 562)' with a phrase only the new text contains. Add an absence check that 'reaches a subscriber to the old label as nothing' is gone from both surfaces.
- CHANGELOG.md: a new issue-562 entry. Polls and webhooks get about 370 more seq-less rows a week, on Mondays and Tuesdays. SSE: the old label hears `removed`, the new label `changed` (upsert), and other filtered streams may get an extra `removed`. A stream resumed across the deploy keeps ghosts of earlier flips until it re-snapshots. Replace the 547 entry's source/kind bullet with a pointer.
- docs/adr/0017: add a dated note under D3 that its source/kind rule now also covers ordinary current-epoch folds (issue 562).
- 5. Deploy and read (S). Blocked by: 3 and 4.
- Gate with ops/check.sh: redirect output to a file and read the GATE-EXIT value.
- Push with `git push origin HEAD:main ...` and confirm with ls-remote.
- Deploy with the next code. No drain.
- After the next Monday or Tuesday fold, run one bounded /v1/sql read, plan-checked on a plan.db copy first. Count the seq-less `tender` `changed` rows in the cursor range since the deploy, joined to tenders where source='ted'. Expect about 50-100, matching that day's flips (TED notices joining an existing Tender with a DÖE v1), with their head-lot rows beside them.
- Record the numbers in the issue and close it.

### Verify

grep -c 'fn a_ted_twin_joining_a_doe_tender_reaches_a_source_subscription' crates/app/tests/api.rs

- **done**: the count is 1 and the test passes. The `source=doe` tenders and lots tapes hear `removed` when the TED twin joins. The `source=ted` tenders and lots tapes hear `changed`, which is applied as an upsert per ADR-0017 D4; `added` is not required (Decision 2026-10-17). /docs and openapi no longer list issue 562 as an exception to 'the diff stream then makes your final state exact'. The first post-deploy Monday or Tuesday read finds seq-less tender rows matching that day's flips.
- **open (2026-10-17)**: the count is 0. The decision is recorded and the build is not started.

### Adversarial check — amendments (adopted)

- Unit 2: name the test that will go red, so the gate failure is expected. Update `a_refold_corrects_a_stored_tender_kind` part 1 (crates/ingest/tests/project.rs ~1224) to expect `[(tender, Some(1), "added"), (tender, None, "changed")]`. Say explicitly that B must NOT be guarded by `!stale`: Off and Shadow stale moves are in scope.
- Unit 2(a): compute the head lots in a way that does not depend on what the write loop did. Either call `lot_identity` over `p.versions.last().lots` after the sweep and `stmts.ids.reset()`, or, when the head version was not rewritten in this call (keep == p.versions.len()), call `crate::inplace::head_lots(conn, tender_id, head_seq, &mut lots)`, because its rows are on disk. Add the test `a_label_moved_by_a_chain_shrinking_to_its_prefix_announces_the_head_lots`: a ted Tender whose TED last version is regrouped out, so it flips ted→doe; expect rule T plus every head lot.
- Unit 2(b): reuse `inplace::head_lots` (inplace.rs:144). Its SQL is identical to the planned statement, and I plan-read it on a copy of v551/plan.db: `SEARCH tender_version_lots USING INDEX sqlite_autoindex_tender_version_lots_1 (tender_id=? AND seq=?)`. Build `a_label_moved_on_an_unchanged_chain_writes_the_correction` from the 462 fixture: a hand-set stored label plus `unmark_projected` plus `project_incremental`. Do it once for kind (extend part 2 of project.rs:1235 to assert the trailing `(tender, None, "changed")` and changes > 0) and once for source on a keyed Tender with lots (`UPDATE tenders SET source='doe'`). Record in the issue that the fold itself cannot move source on an unchanged chain (one version per notice, and primary_source reads the same notices), so (b) is reached only from a stored label that disagrees with the derivation.
- Unit 2: add an `Applied` counter (e.g. `label_corrections`) and print it on the projection report line. Unit 5 can then check the job log against the feed, and the early return no longer reports a Tender that wrote rows as 'unchanged' with nothing else to show for it.
- Unit 3: pin the event sequences, not just which ops appear. After CHAIN[1]:
- The doe tenders tape's FIRST event is `removed` with `version: null`. This proves the v2 transition was dropped and the correction carried the event.
- The doe lots tape hears lot 1 as `removed` with `version: null`.
- The ted tenders tape hears exactly `[changed v2, changed null]`, and the ted lots tape hears lot 1 as `[changed v2, changed null]`.
- Each tape is then `quiet()`.
The assertion 'no ted tape hears a removed' holds only because CHAIN[1] keeps lot 1 and drops no lot. Write that down next to it, because a lot dropped at the TED version gives the ted lots tape a (true,false) `removed`.
- Unit 4: restate the Diff reason accurately. Suggested text: 'both the version's own change and the version: null correction that follows read the Tender's current label on both sides, so a new-label subscriber hears changed (issue 562)'. In crates/app/data/openapi.json, edit line 1146 (ChangeEvent.version, which lists the causes of version: null) together with line 1153. CHANGELOG: tell source- and kind-filtered SSE consumers to re-subscribe once without Last-Event-ID after the deploy. A resumed stream keeps ghosts from earlier flips.
- Unit 5 and the Verify 'done' line: replace the absolute '50-100 seq-less tender rows on ted Tenders' with a paired read. Over the cursor range of that day's daily fold:
- Take each seq-carrying `tender` row whose version is caused by a TED notice n, where tenders.created_at < n's ingested_at, the Tender's chain holds a DÖE notice of LOWER notice id than n (ingested earlier), and no TED notice of lower id. These are the flips: the decision's own q1 criterion (scratchpad/d56x/i562/q1.sql, `src_flips`). *Restated 2026-10-17 (build review): the wording here was 'v1 is caused by a DÖE notice, there is no earlier TED version', which leaves out a TED notice published before, but ingested after, the DÖE notice that minted the Tender. The fold replays that Tender from v1, so the TED version becomes v1; its label still moves doe → ted and B announces it, correctly (q1 counts it under `flips_prepended`: 0 of 59 on 10-12..10-16). Notice ids order ingestion, not publication, so the lower-id test keeps it a flip.*
- Count the flips that have a seq-less `tender changed` row for the same entity_id with the SAME changed_at (the fold's `now`) and a higher cursor. Expect all of them.
- Count same-changed_at seq-less tender rows on TED joins that are not flips. TED-first (a TED notice of lower id already in the chain): expect 0. Co-minted (tenders.created_at ≥ n's ingested_at): expect ≥ 0, each one explained as a TED notice fetched before, but parsed after, the DÖE-only fold that minted the Tender (the line on decision text below). *Restated 2026-10-17 (build review): this line read 'co-minted, TED-first or prepend. Expect 0'; a prepend is a flip (above), and the co-minted class holds real flips that B announces, so neither is an expect-0 bucket.*
- Drive the read from the rowid range (`+entity_kind` so the planner cannot take changes_entity, as my first plan showed it would) and plan-read it on a copy of v551/plan.db.
- Cross-check against the new Applied counter in that job's log.
- Decision text: change '1,128 ... B never touches them' to 'at most 1,128', and '~370 rows a week' to 'at least ~370 rows a week'. A TED notice fetched before, but parsed after, a DÖE-only fold is a flip that the created_at criterion files as co-minted, and B announces it, which is correct. Keep 'kind measured 0' as stated.

### Adversarial check — objections (for the record)

- The SSE trace holds, so B closes the gap. sse.rs:404-465 sends a seq-less `changed` to the arm that probes `t.current_seq` and head−1. Both At probes read the single rewritten `t.source`/`t.kind` (read.rs:3029-3035, lots read.rs:3989). The old-label tape gets (false,false) and `in_place_changed`, so `removed`. The new-label tape gets (true,true), so `changed`, or (false,true), so `added`, for a lot first written at the head. The v2 transition rows still go to the old-label tape as (false,false) on a non-removed op and are dropped (`continue`). So the correction is the only event that reaches it, and it is a re-assertion of the head state, which is safe under every other filter as well. `moved` is set only in tender_identity (canonical.rs:23202), the only writer of tenders.source/kind. Acting and the D3 branch do not overlap: acting = shadow && On, shadow requires stale && !stored.is_empty(), and the early return requires !stale. I found nothing that breaks 551's derived heads: B touches no head column, and ValueBy::Version (read.rs:1424) is re-read only as the head, exact, against head−1, a superset, which is the current in-place behaviour.
- An existing test will go red and the decision does not name it. In crates/ingest/tests/project.rs:1187, `a_refold_corrects_a_stored_tender_kind` part 1 sets `kind='procedure'` by hand, then stamps the Tender stale and refolds. On that test DB the compare is refused, because `tender_version_bid_parties_version` is a DEFERRED index that `project(&db,false)` never builds (canonical.rs:16662, 22617-22640), so the fold is stale but not acting. With B, moved && !acting appends `(tender, None, "changed")`. The test asserts `feed == vec![(tender, Some(1), "added")]` (line ~1224), so it fails. If this is not named, an implementer may 'fix' it by guarding B with `!stale`, which would leave Off and Shadow stale moves unannounced.
- Unit 5's post-deploy read cannot tell flips apart from other writers. I ran one bounded read on prod, plan-checked first on a copy of v551/plan.db: `SEARCH changes USING INTEGER PRIMARY KEY`, cursors 954,896,761-955,146,760, all on 2026-10-16. In those 249,926 rows there were 31,275 seq-less tender `changed` rows (30,913 of them on `source='ted'` Tenders) and 202,649 seq-less lot `changed` rows. They come from D3 drains, the daily rederive-eur-recent (D5) and org merges (canonical.rs:23613, 24843, 27553, 30696, 31361, 31927). 'Seq-less tender changed joined to tenders where source=ted, expect about 50-100' would therefore read thousands, and the run would be called failed or explained away. The Verify 'done' line inherits the same problem. Files: scratchpad/d56x/adv562/s3.sql, s3.out.
- The head-lot source in unit 2(a) has a hole. 'The lot ids the write loop resolved for p.versions.last()' is empty when the chain shrank back to its kept prefix (keep == p.versions.len() < stored.len()): the write loop never runs, and the head's VersionLots rows are on disk, not in `pending`. The realistic case is a TED notice regrouped out of a group whose last version it was, which flips ted→doe. B would then write rule T only, and a `source=ted` lots tape would keep ghosts of the head lots.
- The fixture question for unit 2(b) has an answer already, and the decision leaves it open. The fold cannot move `source` on an unchanged chain: `fold` emits one version per group notice, and `primary_source` reads those same notices' sources (project.rs:3796, 6619). A STORED label that disagrees with the derivation, followed by a bare requeue, does reach the early return with moved=true. Part 2 of `a_refold_corrects_a_stored_tender_kind` (project.rs:1235-1250) already drives exactly this, and asserts (tenders_unchanged, tenders_written) = (1, 0). The same trick works for source (`UPDATE tenders SET source='doe'` plus `unmark_projected`). Prod reaches this path through any hand-made or pre-fix label, as with the 462 cohort.
- The reach numbers are bounds, not exact counts. 147 (and 110-150 a week) is a floor. The 'co-minted' class is defined as created_at ≥ TED ingested_at, which includes TED notices that were fetched but not yet parsed when the DÖE-only fold minted the Tender. Those are real flips that B will also announce. So 'the 1,128 are never touched' should read 'at most 1,128', and about 370 rows a week is a floor. Neither changes the choice of B, but unit 5 should expect count(corrections) ≥ count(flips by that criterion), not an absolute band.
- The restated Diff reason in unit 4 is slightly wrong. The new-label tape first hears `changed` with `version: 2`: the transition row is classified (true,true), because both probes read the rewritten label. The `version: null` correction then repeats it. The text 'a label move arrives as a version: null correction, which a new-label subscriber hears as changed' implies the null event is the only one.
- The no-walk decision rests on one gauge sample (`tender_db_sse_streams 0` at 06:05; metrics.rs:52 exposes only a live gauge). It cannot show that no client resumes source- or kind-filtered streams through Last-Event-ID between connections. Accepting the leftover ghosts is defensible, but only if the CHANGELOG tells such clients what to do, and the planned wording ('keeps ghosts until it re-snapshots') does not say that.

### Build notes (units 2-4, 2026-10-17)

Built in the tree beside issue 560's uncommitted change (they share `apply_tender_tx`); not yet gated.

- **Store (unit 2).** `Db::apply_tender_tx` (crates/store/src/canonical.rs) calls one helper,
  `announce_label_move`, which writes `inplace::correction_rows(tender_id, &head_lots)` as seq-less
  `changed` rows through `stmts.append_change` and returns `1 + lots` for `Applied.changes`, so the doorbell
  rings. Two call sites:
  - (a) `!acting && stored_identity.moved`, after the transition rows, the sweep's `removed` rows and
    `stmts.ids.reset()`, just before the final `Ok`. Not guarded by `!stale`: stale Off and Shadow refolds are
    in scope; the acting compare returns earlier with D3's own set. The head lots never come from the write
    loop: no version gives rule T alone; a head the call did not rewrite (`keep == p.versions.len()`, a chain
    cut back to its prefix) is read from disk with `inplace::head_lots` (now `pub(crate)`); a written head
    (its rows still in `Pending`) is looked up by lot key with `lot_identity`, after the reset.
  - (b) The unchanged-chain early return, when `moved` is set: `inplace::head_lots` at the stored
    `current_seq`. Its SQL is the statement the adversarial check plan-read on a copy of v551/plan.db
    (`SEARCH tender_version_lots USING INDEX sqlite_autoindex_tender_version_lots_1 (tender_id=? AND seq=?)`);
    not re-read here (no prod or plan.db access in this build).
  - Both futures are `Box::pin`ned, so `apply_tender_tx`'s frame gains only a pointer.
  - New `Applied.label_corrections` (one per Tender announced by B; the acting set stays under
    `tenders_corrected`), printed on the supervisor's durable fold line (`…, N verified unchanged, M relabelled
    (issue 562)`) and on ingest's `[project] done:` line, for unit 5's job-log check.
- **The fold cannot move `source` on an unchanged chain** (adversarial check): `fold` emits one version per
  group notice and `primary_source` reads those same notices' sources, so (b) is reached only from a stored
  label that disagrees with the derivation (a hand-set or pre-fix one, as with issue 462's kinds) and a bare
  requeue. `a_label_moved_on_an_unchanged_chain_writes_the_correction` builds exactly that.
- **The named test that changed:** `a_refold_corrects_a_stored_tender_kind` (crates/ingest/tests/project.rs).
  Part 1 (stale, compare refused so not acting) now expects `[(tender, Some(1), "added"), (tender, None,
  "changed")]`; part 2 (the early return) asserts the trailing `(tender, None, "changed")`, `changes > 0` and
  `label_corrections == 1`.
- **Tests.** crates/store/tests/label_move_corrections.rs: `a_label_moved_by_an_ordinary_fold_writes_rule_t_and_the_head_lots_seqless`
  (includes a lot minted by the TED version, so the written-head lookup is exercised),
  `a_fold_that_leaves_the_label_alone_writes_no_correction`, `a_ted_first_tender_gaining_its_doe_notice_writes_no_correction`,
  `a_co_minted_doe_and_ted_pair_writes_no_correction`, `a_stale_refold_that_moves_the_label_writes_one_correction_set`
  (On: exactly one set, D3's, `label_corrections` 0; Off and Shadow: identical rows, the replay then B's set),
  `a_label_moved_by_a_chain_shrinking_to_its_prefix_announces_the_head_lots` (the sweep's `removed`, then rule T
  and every head lot read from disk). crates/ingest/tests/project.rs: `a_label_moved_on_an_unchanged_chain_writes_the_correction`
  (source, keyed Tender with a lot) beside the kind half in the 462 test.
- **App (unit 3).** `a_ted_twin_joining_a_doe_tender_reaches_a_source_subscription` (crates/app/tests/api.rs),
  with a new `Server::record_bytes_from(source, …)`: CHAIN[0] recorded as DÖE, four tapes, CHAIN[1] as TED. Pins
  the doe tenders tape's FIRST event (`removed`, `version: null`), the doe lots tape's lot `removed` null, each ted
  tape's exact `[changed v2, changed null]`, every tape quiet, the pages, and `/v1/changes`
  (`added v1 ×2, changed v2 ×2, changed null ×2`). The "no ted tape hears removed" note (CHAIN[1] keeps lot 1 and
  drops none) sits beside it.
- **Served text (unit 4).** /docs and openapi EventStream: the 562 exception is dropped from the Snapshot list
  (the remaining exceptions are 565 and 561, after 560's rewrite); the Diff says both the version's own change and
  the `version: null` correction read the current label on both sides, so the new label hears `changed` and the
  old `removed`. The `version: null` cause lists (docs event schema, openapi `ChangeEvent.version`) name the fold
  re-labelling a Tender's source or kind. Docs pins: the new phrases, absence of "reaches a subscriber to the old
  label as nothing" on both surfaces, and the null-version cause. read.rs comment, CHANGELOG entry (with the
  re-subscribe-once advice; the 547 bullet now points here), ADR-0017 dated note under D3.


### Build review fixes (2026-10-17, uncommitted)

- **CHANGELOG SSE bullet.** "`added` is not sent there" was false: SSE classifies by presence at head and head−1,
  so a new-label subscriber hears `added` for an entity the version before did not hold. Reproduced by the review:
  a lot first written by the re-labelling version (LOT-3 in
  `a_label_moved_by_an_ordinary_fold_writes_rule_t_and_the_head_lots_seqless`: `added` for both events), every
  entity when the fold leaves the Tender at version 1 (the shrink to a one-version DÖE prefix: `added` once, no
  version of its own), and every entity at v1 when the fold rewrites it from version 1 (a TED notice prepended to a
  DÖE-minted Tender). The bullet now says `changed` for an entity it held (twice when a version moved the label),
  `added` for one the version before did not hold, either applied as an upsert. /docs and openapi's Diff say "hears
  `changed` (or `added`, for an entity the version before did not hold)"; the docs pin follows.
- **Unit 5's paired read** (amendment above, restated in place): the flip criterion is q1's notice-id form, so a TED
  notice prepended to a DÖE-minted Tender counts as a flip; the expect-0 bucket is TED-first only, and co-minted is
  expect ≥ 0, each one explained.
- **docs/operations.md** "Reading a `project` job's `counts` line": the template gains `, <r> relabelled (issue
  562)`, with what it counts (B's announcements: ordinary folds, the unchanged-chain early return, Off/Shadow stale
  refolds) and what it does not (an `on` refold's label moves count under the compare's `corrected`).

## Unit 5 — first half: the 2026-10-19 daily fold (PASSED; workflow wf_c31642a7-759)

`.scratch/tender-db/562-u5/` (SQL from gen.py, plans, raw outputs, result.md; verify-1019.md). Project 2645 (07:35Z daily
fold: 4,644 notices → 4,427 Tenders, 4,070 written, 357 verified unchanged, **102 relabelled**; the counts line read from
`/admin/jobs` by the orchestrator). Change rows (958810037, 958848408]: 38,371 rows, all changed_at 07:35:55Z (the
rederive-eur-recent and reveal-recheck jobs of the chain wrote none; the 551 zero-drift probe after it wrote none). 26
bounded SELECTs (≤ 5,000 change rows each), every plan read first on a copy of v551/plan.db: 216 plan lines, 0 SCAN.
- **(1) Flips (q1 notice-id form): 102** on 102 Tenders (all seq 2 = current_seq; prepends 0; every Tender minted 10-18 by a
  DÖE notice, its TED twin ingested 10-19 — the weekend pattern).
- **(2) Announced: 102 of 102** — each has a seq-less tender `changed` row, same changed_at, 2–15 cursors later; their head
  lots total 156 = the 156 seq-less lot `changed` rows in the range.
- **(3) Not flips: 0** — TED-first 0 rows (305 TED-first Tenders in the fold, none got one); co-minted 0 rows (8851169 and
  8851990 correctly none).
- **(4) Log: 102 seq-less tender rows = relabelled 102**, all flips; the early-return, Off/Shadow and D3 paths wrote none;
  kind moves 0 (no prepends, so no fold-first change).
- Verifier (2 confirmation reads): criterion = q1 exactly; pairing and bounds right; recount from the notice side gives the
  same 102. **PASS.**

## Unit 5 — second half: the 2026-10-20 daily fold (PASSED; workflow wf_d44820c4-5dc) — issue DONE

`.scratch/tender-db/562-u5/second-half/` (gen.py, 41 SQL, plans 344 lines 0 SCAN, raw outputs, result.md, verify.md).
- Chain range (958848784, 958904203], 55,419 rows (dense): rederive-eur-recent 2668's D5 rows first (958848785..958853608: 913
  tender + 3,911 lot = 4,824 = its job line), then project 2669's rows (958853609..958904203, all changed_at 1792482087);
  reveal-recheck 2670 wrote 0. The paired read ran over the project sub-range only.
- **(1) Flips (q1 notice-id form): 57** on 57 Tenders, all seq 2 = current_seq, 0 prepends, none D5-stamped (minted by the 10-19
  fold from DÖE notices 47414726..47414807; TED twins 47415000..47418535).
- **(2) Announced: 57 of 57** (same changed_at, 2–15 cursors later).
- **(3) Not flips:** TED-first 0 (353 TED-first Tenders, none got a row); co-minted 0 rows (8853588, a re-minted island, correctly none).
  The other 897 seq-less tender rows are ADR-0017 D3's compare-On refold of the 913 D5-stamped Tenders (`corrected` 897).
- **(4) Log:** 57 seq-less tender rows on non-D5 Tenders = relabelled 57; lots 101 (flips) + 3,712 (D3) = 3,813 = every seq-less
  lot row in the range; kind moves 0.
- Verifier (1 confirmation read, notice side): same 57. **PASS.** Both halves passed, so the issue is done.
