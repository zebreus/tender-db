# 442 — issue 441's trap is schema-wide: 11 parent tables' DELETEs walk child tables in their FK proof, because turso uses only an exact-shape child index

Status: ready-for-agent — **step 2 VERIFIED on prod 2026-09-29** (wet R2 1650, E0 1651, p0 1652, all foreign keys ON: ms per loser, 0 `FOREIGN KEY constraint failed`). What remains is step 4, the upstream request: the text is ready in `.scratch/tender-db/upstream-turso-requests.md`, and this session has no GitHub access to tursodatabase/turso to post it.
Was status (before 2026-09-29): ready-for-agent — **step 2 DEPLOYED 2026-09-29 06:10 UTC** (rev `92ebde0`). Its Verify waits for the next wet R2/E0/p0 run: per-loser time in ms, and no `FOREIGN KEY constraint failed` in the journal. Step 4 remains (post from the owner's account). Was: unit 1 DEPLOYED and VERIFIED 2026-09-28 (see the foot); steps 2–4 remain. Was: unit 1 BUILT 2026-09-27 (the `organizations` delete, see below): gated, committed and
not deployed. It deploys with the next bundle, and the auto-Reindex builds it. Filed 2026-09-27 from the hourly
AUDIT step, generalising issue 441. The remaining units are measure-first (see "What to decide").
mine to take.
Kind: performance (write paths), plus design debt: five foreign-key-off brackets exist to route around this cost
Relates to: 441 (the mention FK: the same mechanism, found and fixed), 352 (the R2/E0/R3 merge loops "pay ~0.4 s a
row on the write path" and were bracketed with foreign keys off, the cost never explained), 351 (the provisional fold's
bracket), 432 (`repair_provisional_name_norm`'s bracket), 404 (`apply_member_twin_repair`'s bracket), 19 (the
projection's bracket), 440 (`dissolve_condemned` deletes orgs with foreign keys ON)

## The rule (turso_core 0.7.2, `translate/fkeys.rs` `emit_fk_parent_key_probe`)

A parent DELETE (foreign keys ON) proves each referencing child has no row pointing at it. That proof uses a child
index only when its columns EQUAL the FK's child columns: same count, same order. A composite primary key or index
that merely LEADS with the FK column does not count. SQLite would use it; turso does not. Without an exact match, the
proof is a `Rewind` over the whole child table, per deleted parent row.

## Audit (2026-09-27, local schema with every deferred index built, `EXPLAIN DELETE FROM <parent> WHERE rowid = 1`)

| parent | child tables walked by the FK proof |
| --- | --- |
| `organizations` | `organization_names` (PK `(org_id, lang)`, no `(org_id)` index) — **unit 1, built** |
| `organization_mentions` | none since 441 |
| `notice_sections` | none |
| `notices` | `notice_dates`, `notice_amounts`, `notice_ids`, `lot_results`, `notice_numbers`, `notice_sections`, `notice_classifications`, `notice_codes`, `notice_texts`, `contracts`, `notice_integers`, `bids`, `tenders` |
| `tenders` | `lots`, `lot_results`, `contracts`, `bids`, `tender_versions` |
| `tender_versions` | `tender_version_lot_results`, `_lots`, `_result_winners`, `_bids`, `_contracts`, `_lot_group_members` |
| `lots` | `tender_version_parties`, `_classifications`, `_lot_results`, `_texts`, `_lots`, `_amounts`, `_dates`, `_bids`, `_lot_group_members` |
| `lot_results` | `tender_version_lot_results`, `_result_winners`, `_result_stats` |
| `bids` | `tender_version_bid_parties`, `tender_version_bids` |
| `contracts` | `tender_version_contracts` |
| `fetches` | `quarantine` |
| `webhook_endpoints` | `webhook_delivery_log` |
| `users` | none |

## Who pays it today (paths that delete parents with foreign keys ON)

- **`organizations`** (~78M `organization_names` rowids on prod, read 2026-09-27): `dissolve_condemned` (440's
  `repair-placeholder-orgs` wet, 425 orgs queued behind the re-parse chain), `merge_provisional_organizations_batch`
  and `repair_nested_losers`. R2/R3/E0/351/432 bracketed foreign keys off (issue 352's ~0.4 s per loser is this walk).
  **Unit 1** gives the FK its exact index.
- **`notices`**: `record_notice_tx`'s minted-row rollback (one row, rare) and `apply_member_twin_repair` (404,
  bracketed off). With 13 walked children, including `notice_texts`, a notice delete with foreign keys ON would cost
  minutes.
- **Tender-layer parents** (`tenders`, `tender_versions`, `lots`, `lot_results`, `bids`, `contracts`): the fold
  rewrites these. Not yet checked is which fold/repair paths delete them with foreign keys on (the projection's bulk
  path is bracketed, issue 19). That is the first thing to measure.

## What to decide (the next step)

1. **Unit 1 (built):** `organization_names_org` on `(org_id)` in `DEFERRED_ORG_INDEXES`, pinned by
   `an_organization_delete_proves_its_foreign_keys_by_index` (`crates/store/tests/mention_fk_probe.rs`).
2. **Then delete the brackets it makes pointless.** With unit 1 live, measure one R2 dry→wet with the
   `foreign_keys=OFF` bracket removed. If the per-loser time drops to the ms scale, delete the brackets in R2/R3/E0
   (352) and the 351 fold. That deletes code, and enforcement stays on.
3. **Tender layer:** list the fold/repair statements that delete tender-layer parents with foreign keys ON (grep
   `DELETE FROM tenders|tender_versions|lots|lot_results|bids|contracts` against the enforcement state at each call).
   Give an exact-shape index only where a live path pays the walk. Each index costs disk and write amplification on
   tables of 10⁷–10⁸ rows, so this is not a blanket fix.
4. **Upstream:** turso could accept a child index whose LEADING columns equal the FK's, as SQLite does (the
   `index_scan_match_any` prefix walk already exists beside the exact-match probe). That would fix every row above at
   once, with no new indexes. Draft the request with the 441 and 442 evidence; it should not be vendored here (turso_core
   is not ours to maintain, see `crates/vendor/turso/VENDORED.md`).

## Verify

    cargo test -p store --test mention_fk_probe

- **done** (unit 1): the organization-delete test is green AND on prod the auto-Reindex line names
  `organization_names_org`, and 440's `repair-placeholder-orgs` wet reports its per-org time in ms, not seconds.
- **was open**: unit 1 built, not deployed (2026-09-27).
- **read 2026-09-28**: the auto-Reindex after the 06:05 UTC deploy built `organization_names_org` (job 1598, 22 s; the
  table is smaller than its 78M rowid bound). 440's wet dissolve (job 1615) deleted all 425 condemned organizations
  in under a minute (queued 07:56, done 07:57 UTC): ms per org, not seconds. **Unit 1 done.**

## Unit 3 answered by reading the code (2026-09-27 15:5x UTC): no live path pays the tender-layer rows

Every statement that deletes a tender-layer parent runs inside a foreign-key-off bracket:

- `retire_tenders_chunked` (`DELETE FROM tenders` plus the satellites, `canonical.rs` `retire_chunk_tx`) is reached
  only from `retire_absorbed_legacy_tenders` / `retire_regrouped_nonlegacy_tenders`
  (`project_with_progress_phase2_stoppable`, under `project`'s issue-19 bracket or the incremental fallback into it)
  and from `retire_regrouped_tenders_chunked` (the incremental apply, under `project_incremental_observed_stoppable`'s
  bracket).
- `apply_member_twin_repair` (404) brackets itself.
- Nothing else deletes `tenders`, `tender_versions`, `lots`, `lot_results`, `bids` or `contracts` outside tests.

So the tender-layer rows of the audit cost nothing today, and adding exact-shape indexes there would be pure write
amplification. They matter only if a bracket is removed, and that is the condition for the upstream request (step 4)
rather than for indexes. What remains live is the `organizations` row (unit 1, built) and the merge-loop brackets it
lets us delete (step 2, measure after unit 1 deploys).

## Step 4 draft — the upstream request (drafted 2026-09-27, NOT posted; post from the owner's account)

> **FK parent-key probe ignores a child index that leads with the FK columns**
>
> `translate/fkeys.rs` `emit_fk_parent_key_probe` (turso_core 0.7.2) looks for a child index with
> `ix.columns.len() == child_cols.len()` and every column equal in order. An index that merely LEADS with the child
> columns fails that test, including a composite PRIMARY KEY, and the probe falls back to `table_scan_match_any`: a
> full scan of the child table per deleted (or re-keyed) parent row. SQLite uses any index whose leftmost columns
> are the child key.
>
>     CREATE TABLE p (id INTEGER PRIMARY KEY);
>     CREATE TABLE c (pid INTEGER NOT NULL REFERENCES p(id), lang TEXT NOT NULL, PRIMARY KEY (pid, lang));
>     PRAGMA foreign_keys = ON;
>     EXPLAIN DELETE FROM p WHERE id = 1;   -- OpenRead c + Rewind: the whole table, per row
>
> With `c` at 78M rows a single-row delete takes ~2 s. The function already has the pieces for a prefix match:
> `index_scan_match_any` iterates "the index entries whose leading columns equal `probe_start`". Selecting an index
> with `ix.columns.len() >= child_cols.len()` and a matching prefix, and taking the `index_scan_match_any` path
> whenever the index is longer than the key, would serve these without a scan. Workaround today: a redundant index
> of exactly the FK's columns.

Evidence to attach: 441's and 442's `EXPLAIN` excerpts (the `Rewind` lines) and the prod timings (2.2 s per mention
delete → 16 µs after the exact-shape index). The draft for issue 425's `interrupt()`/clock request sits in 425
step 4. Post the two together.

## Step 2 — BUILT 2026-09-28: the merge loops' foreign-keys-off brackets are deleted

- **Measurement.** Step 2 asked for one R2 dry→wet with the bracket removed. The prod number already exists from
  another path: 443's wet sweep (job 1628) deleted 1,768,353 organizations with foreign keys ON through the same five
  exact-shape child indexes, in 348 s for both walks (~0.2 ms a delete, walk included). 440's FK-on dissolve (job
  1615) repointed and deleted 425 organizations in under a minute. That is the ms scale step 2 set as the bar.
- **Deleted:** the `PRAGMA foreign_keys=OFF/ON` brackets and their `looped`/`folded`/`applied` wrappers in
  `match_org_identifiers_r2` (R2 and E0), `match_org_null_country_r3`, `fold_provisional_echoes` (p0) and
  `repair_provisional_name_norm` (p1). No merge path turns enforcement off any more. The projection's issue-19
  bracket and 404's twin repair are not org deletes and keep theirs (unit 3).
- **Found and fixed on the way: the p0 fold orphaned `organization_names` rows.** For a loser with no tender rows it
  took a mention-only shortcut (`full_repoint: false`), so the loser's name variants were never moved and the
  `DELETE FROM organizations` left them pointing at a deleted id. The bracket hid it; with foreign keys ON the delete
  would have been refused. The shortcut and the `full_repoint` flag are deleted, so every loser takes
  `repoint_org_references` (all five child tables). The statements it skipped are seeks on the party tables'
  `(organization_id)` indexes. Those indexes predate the shortcut (a review read `b4a18a2`), so the ~0.5 s its
  first slice measured was most likely the unindexed `organization_names` FK walk that unit 1 fixed. Test: `a_loser_without_tender_rows_hands_its_name_variants_to_the_keep`
  (`provisional_echo_fold.rs`), checked failing on the old code (2 orphaned variants). The rows past p0 runs left on
  prod are issue 444.
- **Guard:** `Supervisor::refuse_without_org_fk_indexes` (shared with the sweep; store side
  `org_fk_missing_indexes` over `ORG_FK_INDEXES`). Each wet merge arm calls it after its own refusals, boxed for the
  run_spec stack, and refuses while any of the five indexes is missing. All five are deferred, so a rebuild strips
  them. Test: `a_wet_merge_refuses_without_the_org_fk_indexes`.
- **Verify once deployed:** the next wet R2/E0/p0 run reports its per-loser time in ms, and journalctl shows no
  `FOREIGN KEY constraint failed` from a merge job.
- **Verified on prod 2026-09-29 (rev `92ebde0`), all three merge paths with foreign keys ON:**

  | job | path | plan | removed | wall time | per loser |
  | --- | --- | --- | --- | --- | --- |
  | 1650 | R2 wet | 551 groups | 741 org rows (10,135 mentions, 29,516 parties, 1,642 bid-parties, 2,311 winners repointed; 6,486 tenders touched) | 8 s | ~11 ms |
  | 1651 | E0 wet | 12 groups | 21 org rows | 2 s | — |
  | 1652 | p0 wet (`fold-provisional-echoes`) | 9,728 groups | 17,139 org rows (90,011 mentions, 93,273 parties, 90,221 winners; 52,355 tenders) | 243 s, of which ~200 s is the planning walk (dry 1649 took 203 s) | ~2.5 ms |

  Each wet plan matched its dry run exactly (1647 → 551, 1648 → 12, 1649 → 9,728 / 17,139).
  `journalctl -u tender-db --since -90min | grep -c 'FOREIGN KEY constraint failed'` = **0**, and no
  `reclaim stamped NO ledger rows`. Step 2 is done.
- **Change-feed audit of the same runs (2026-09-29 11:5x UTC).** I paged `/v1/changes` from cursor 673088407 to
  673167860: 79,453 events in 80 pages, 10:50:03 → 10:54:13 UTC. That window holds E0 1651 and p0 1652; R2 1650
  had finished before the start cursor. Every count matches a job summary exactly:
  - `organization removed` 17,160 = 21 + 17,139 rows removed;
  - `organization changed` 9,740 = 12 + 9,728 keeps, one per group;
  - `tender changed` 52,553 = 198 + 52,355 tenders touched.

  The only slack: those 52,553 tender events name 37,480 distinct tenders. A tender touched by several groups (or
  by both jobs) is published once per touch. Consumers key on id, so nothing is wrong, but about 15k of the events
  are redundant. Not worth a unit of its own unless feed volume becomes a problem.
- **Adversarial review (one read-only agent, 2026-09-28): no defect.** It confirmed from turso_core 0.7.2:
  - A child UPDATE re-checks only the foreign key whose columns change (`fkeys.rs` `child_key_changed`), so latent
    violations elsewhere in a repointed row are not re-checked.
  - Repointing `organization_mentions.organization_id` triggers no parent-key probe, because `(notice_id,
    section_id)` is untouched.
  - No table references `tender_version_result_winners` or `organization_names`.
  - No key is DEFERRABLE, and 0.7.2 has no `defer_foreign_keys` pragma.
  - `PRAGMA foreign_keys` just sets a connection flag, so the deleted comments' claim that it is "a no-op inside a
    transaction" was wrong too.
  - Its doc-comment findings are fixed.
- **Also from that review, not fixed here:** lib.rs's re-parse comments (~1689, ~2429) say foreign keys are
  "checked once at COMMIT". turso checks them immediately. Filed for the next firing (445).
