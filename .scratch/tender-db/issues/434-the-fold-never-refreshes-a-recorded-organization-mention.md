# 434 — the fold never refreshes a recorded organization mention, so no parse fix reaches the org layer

Status: ready-for-agent — **BUILT 2026-09-27** (see the foot): a recorded mention whose published facts changed is
re-resolved and rewritten in place. Uncommitted, not deployed. Filed 2026-09-27.
Kind: defect (org layer — the mention resolver's idempotency preload in `crates/store/src/canonical.rs`
`resolve_mentions` / `resolve_one_mention`)
Relates to: 248 (the re-parse keep-set that makes this bite: a re-created section keeps its mention), 247 (why a
mention is never deleted to refresh it), 259 (the same surprise, repaired then by a direct walk), 365 unit 6
(`refold-denied-schemes`, documented as unable to re-bind for the same reason), 435 (R2.0.7 names — the largest
population this blocks), 393 unit 3 (Greek ISO-8859-7 re-decode), 436 (text-era `AU` line wrap), 439 (which row a
refreshed identifier binds to), 432 (the provisional name key a refreshed mention probes with)

## Observed (2026-09-27, prod)

Three parse-layer fixes that cannot reach the org layer, all measured today:

1. **R2.0.7 names (issue 435).** ~1.06M mentions (profile `ted-export-r208`, declared `R2.0.7.*`, 2010-03 → 2011-09)
   are nameless, on ~1.05M nameless provisional organizations. Mapping `TED-ORGANISATION` fixes the parse → mention
   derivation, and the queued r208 re-parse (job 1596) re-creates every section id — so every one of those mentions
   survives the re-parse and the fold keeps it nameless.
2. **Text-era countries.** `TXT-CY` is re-homed onto `ORG-1` by `home_authority_descriptors`
   (`crates/ingest/src/text/parse.rs`), but standing `ORG-1` mentions keep `country` NULL. Deutsche Bahn's row
   24630207 holds 2,332 country-less text-era mentions.
3. **Greek mojibake.** Issue 393 unit 3's ISO-8859-7 decode re-parse (job 1595) cannot reach the ~1,150 mojibake
   mentions.

## Mechanism

Two decisions, each right on its own, compose into a wall:

- `clear_parsed` (`crates/store/src/lib.rs`, issue 248's keep-set) deletes a mention only when its SECTION goes away.
  Deleting one costs ~2.2 s on prod (FK proving over `tender_version_parties`' 78M rows is not index-served on the
  write path), so an era re-parse that re-creates its section ids keeps every mention.
- `resolve_one_mention` returned early for ANY recorded `(notice_id, section_id)`:
  `if let Some(&org_id) = mention_of.get(&(m.notice_id, m.section_id.clone())) { return Ok((org_id, false)); }`.
  Only the new-mention path wrote `name`/`country`/`raw_identifier`/`scheme` and chose an organization.

So a re-parse keeps the row, the fold keeps its facts and its binding, and Phase 2 (`mentions_by_ids`) binds the
party rows to the organization minted from the OLD facts. No parse-layer or mapping fix reaches a standing mention.

## What to build

1. The preload carries what the table stores — `name`, `country`, `raw_identifier`, `scheme`, and the row's rowid —
   still scoped to the batch's notices (issue 57).
2. Recorded and **equal** on those four facts → keep, exactly as today (no write, no mint). Recorded and
   **different** → resolve through the SAME path a new mention takes, then `UPDATE` the existing row in place
   (`organization_id` and the four facts). Never DELETE + INSERT (247's ~2.2 s per row).
3. A `mentions_refreshed` counter (and `mentions_rebound`, the ones that moved to another organization) on the
   fold's report and its durable job line, so the next fold says how far a fix reached.
4. Parties must follow. Phase 2 binds sections through `organization_mentions` as it stands, but only for a Tender it
   REWRITES, and the fold early-returns on an unchanged chain of causing notices at the current `projection_epoch` —
   which a refresh changes neither of. So a re-bind stamps its notice's Tenders epoch-stale, in the same transaction
   as the mention rewrite, and Phase 2 of the same run rewrites them.

Deliberately NOT compared: the organization the resolver would choose today. Merges, rehomings, dissolves and case
verdicts move recorded mentions between organizations on purpose; re-deciding every unchanged mention would undo them
on the next fold. So `refold-denied-schemes` (365 unit 6) still cannot re-bind — a denied scheme leaves the published
facts unchanged — and its doc comment stays true.

**Follow-up, not built: the organizations a refresh leaves without a mention.** No job reaps a mention-less
organization. The deletes that exist are all merge-shaped (a loser repointed into a keep): `merge-provisional-orgs`,
`match-org-identifiers-r2`, `match-org-null-country-r3`, `fold-provisional-echoes` / `repair-provisional-name-norm`
(`fold_provisional_plan`), `repair-placeholder-orgs` (`dissolve_condemned`), `repair-nested-orgs` (a nameless loser
with EXACTLY ONE mention). None selects an organization with zero mentions. Issue 393's unit-1 note ("the
mention-less-provisional sweep the org layer already runs") assumes one that does not exist. After the re-parse chain's
fold with this deployed, expect ~1.05M nameless R2.0.7 provisionals left mention-less (each nameless mention minted its
own row, issue 234), plus the country-less rows the text-era mentions leave (Deutsche Bahn 24630207). They are
unreferenced by mentions and by the re-derived party rows, but they stand in `organizations` and in `/v1/organizations`
listings until something reaps them. The fix is a sweep: provisional, `NOT EXISTS` a mention, no referencing party /
bid-party / winner row, and a `removed` change event per row. It needs its own issue and a dry count first.

## Verify

After the first fold that runs with this deployed, the job line carries
`; N recorded mention(s) refreshed, M re-bound to another organization (issue 434)` (silent when N = 0), and the
`.diag.log` carries `[issue 434] … refreshed in place`. Then the Deutsche Bahn row:

    ssh -o BatchMode=yes root@zebreus.click 'curl -s --max-time 15 -H "Authorization: Bearer $(cat /root/tender-sql-token)" --data-binary "SELECT COUNT(*) FROM organization_mentions WHERE organization_id = 24630207 AND country IS NULL" https://tenders.zebreus.click/v1/sql'

- **done**: `0` once the text era's mentions are refolded (they re-bind to the DE-scoped row)
- **open**: `2332` (2026-09-27)

**Run order.** The re-parse chain queued 2026-09-27 (1595 text, 1596 r208/r209, 1597 one full fold) is where this
pays off: 1597 re-resolves every re-parsed notice's mentions, and with this deployed it refreshes them. If 1597 has
already run when this deploys, the affected cohorts are `projected = 1` and an incremental fold will not revisit them —
queue a `refold` of `ted-export-r208` (435's population) and of the text era, and read the counter. The size of a
refresh over the whole corpus is NOT known in advance: every mention whose derived facts drifted since it was
recorded refreshes (a newer `canonical_country` table, say), not only the three cohorts above. The counter is the
measurement; a dry census was not built.

## Built (2026-09-27)

Uncommitted, not deployed, not run on prod. Gate (`ops/check.sh`) not run; the focused suites below are green.

**The comparison rule** (`RecordedMention::publishes`, `crates/store/src/canonical.rs`): the stored row's `name`,
`country`, `raw_identifier` and `scheme` against the incoming `Mention`'s, byte-exact, with a stored NULL name equal to
`""` (both mean "no name published"; the resolver always writes the string). Equal → return the stored organization,
no write. Different → the new-mention path (identifier arms, Stage-2/3 prevention, the 318 wall, 234/351 name reuse,
mint), then `UPDATE organization_mentions SET organization_id, name, country, raw_identifier, scheme WHERE rowid = ?`
(rowid from the preload; the PK form is a fallback for a mention recorded earlier in the same call, which no fold
produces). Only non-key columns change, so turso's parent-side FK check (`parent_key_may_change`) does not scan the
party tables. A re-bind also stamps the notice's Tenders stale in the same transaction (below). The refreshed mention's labelled variants are written to its (possibly new) organization, as for a new
mention; a new organization gets its `added` change event as any mint does.

**How parties follow — corrected after the first cut failed the gate.** The first cut claimed Phase 2 would carry the
refreshed binding on its own (`apply_plan_batch` → `mentions_by_ids` → `bind_organizations`), and that is true only
for a Tender Phase 2 REWRITES. `apply_tender_tx` early-returns when the chain of causing notices is unchanged and
`projection_epoch` is current ("verified unchanged"), and a refresh changes neither, so the gate caught the mention
on the new organization while `tender_version_parties` stayed on the old one. The `reparse` job would have hidden it
for its own cohort (it stamps its profiles stale, supervisor.rs `Spec::Reparse`), but a fold that refreshes without
such a stamp (a mapping fix reaching a full fallback fold, a refold of another cohort) would leave the two layers
disagreeing for good, because the next fold finds the mention equal and keeps it. Now `resolve_mentions` collects
the notices whose mention was RE-BOUND in each write chunk and, before that chunk's COMMIT, stamps their Tenders
`projection_epoch = 0` (`stamp_tenders_of_notices_stale`: the `tender_versions_notice` probe, then a PK update, as
`stamp_stale_for_notices` does, without its checkpoints, which cannot run inside a transaction). Committing in the same
transaction means a crash cannot leave a refreshed mention whose Tender nothing will revisit. Phase 2 of the same run
then rewrites the Tender whole (`keep = 0`), binding roles, bids and winners to the refreshed row; the rewrite
restores the current epoch. A refresh that keeps the organization (a renamed registration) stamps nothing: no party
row moves. Both fold paths resolve a changed notice's mentions (full `build_plan`, incremental pass 2) before their
Phase 2 reaches its Tender.

**Counters.** `MentionResolver.mentions_refreshed` / `mentions_rebound` / `tenders_stamped`, read by
`Db::mention_refresh` → `store::MentionRefresh`; `project::Report.mentions_refreshed` / `mentions_rebound` (full and
incremental paths); the `[project] plan:` and `pass-2 plan build` stage lines; the durable `project` job line
(`supervisor.rs`); a `[issue 434]` `.diag.log` line from `finish_mention_resolver` that also names the Tenders stamped.

**Tests** — store `crates/store/tests/mention_refresh.rs`:
`a_stale_mention_is_rewritten_in_place_and_an_unchanged_one_is_kept` (same rowid, one row, refreshed 1 / rebound 1 /
1 Tender stamped and only that one; an identifier-bearing rename refreshes with rebound 0 and stamps nothing; NULL vs
`""` is not a change), `a_mention_gaining_a_country_rebinds_to_the_country_scoped_row` (the Deutsche Bahn shape).
Ingest `crates/ingest/tests/project.rs`, the shared body `a_refreshed_mention_moves_its_party` run over BOTH fold
paths and with NO `stamp_stale_*` call of its own: `a_refold_refreshes_a_recorded_mention_whose_published_name_changed`
(incremental) and `a_full_fold_refreshes_a_recorded_mention_and_moves_its_party` (full non-rebuild fold,
`project(db, false)` → `build_plan`) — the mention row's name and `organization_id` change, the buyer party row
follows, nothing still names the old org, `mentions_refreshed == 1`, and the rewritten Tender is back at the current
epoch. `an_unchanged_refold_refreshes_no_mention_and_mints_no_organization` (refreshed 0, same org, same org count,
`tenders_written == 0`: nothing over-stamped). `a_refold_names_the_standing_r207_mentions_and_moves_their_parties`
(435's stock reproduced by folding the fixture without its `TED-ORGANISATION` rows, then re-parsed: 18 refreshed, 18
re-bound, no nameless mention or party left, the 18 nameless provisionals left mention-less).

**Red first**, each variant patched in, run, and restored byte-identical:
- the ORIGINAL code (early return + `TED-ORGANISATION` unmapped): all three store tests fail (the renamed mention stays
  on org 1; the country-gaining mention stays on its country-less row; the preload binds 9000). Four ingest tests fail:
  both party-follow tests (`mentions_refreshed` 0), the R2.0.7 refold (0 of 18 refreshed), and the R2.0.7 fixture fold
  (18 nameless mentions). The unchanged-refold test and the fixture sweep pass on both, as they should.
- the FIRST CUT (refresh without the stamp): both party-follow tests fail at "the buyer party row re-bound with its
  mention", left 0 right 1, on the incremental AND the full fold. That is the gate's finding, on both paths. The
  R2.0.7 refold test passes even without the stamp (its legacy Tender is rewritten on that path for another reason),
  so the eForms pair is what pins the stamp.

**Green** (focused, gate env): store `mention_refresh` 3/3, `resolver_prevention` 10/10, `provisional_name_norm` 3/3,
`placeholder_dissolve`, `reparse_fk`, store `--lib` 135/135; ingest `project` 72/72 (incl. the tier-5
`a_stamped_refold_rederives_winners_from_rewritten_mentions`, whose repointed binding survives because its facts are
unchanged), `project_incremental` 25/25, `project_equivalence`, `project_golden`, `project_resume` 3/3,
`project_fold_source` 5/5, `data_quality` 9/9, `fts` 4/4; `cargo check -p tender-db --features server` clean.
