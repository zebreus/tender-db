# 562 — a whole-Tender rewrite of `source` or `kind` (a DÖE Tender turning `ted`) is invisible to an SSE subscription on it

Status: ready-for-agent — DECIDED 2026-10-17 (wf_3176baf8-d14, judge + adversarial check, amendments adopted): B — a non-acting fold that rewrites a Tender's `source` or `kind` writes ADR-0017 D3's seq-less rows (rule T + every head lot), head lots read from disk when the write loop did not run; no read-path, schema or refold change. NEXT: unit 2 (store; name the `a_refold_corrects_a_stored_tender_kind` change), unit 3 (api test pinning event sequences), unit 4 (served text), unit 5 (deploy + paired read).
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
- 1,128 of the 1,275 DÖE+TED merges in that window were co-minted or minted TED-first, so `moved` stays false and B never touches them.
- B's cost is about 370 extra seq-less rows a week.

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
- Take each seq-carrying `tender` row whose version is caused by a TED notice, where v1 is caused by a DÖE notice, there is no earlier TED version, and tenders.created_at < that notice's ingested_at. These are the flips.
- Count the flips that have a seq-less `tender changed` row for the same entity_id with the SAME changed_at (the fold's `now`) and a higher cursor. Expect all of them.
- Count same-changed_at seq-less tender rows on TED joins that are not flips: co-minted, TED-first or prepend. Expect 0.
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

