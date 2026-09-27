# 442 — issue 441's trap is schema-wide: 11 parent tables' DELETEs walk child tables in their FK proof, because turso uses only an exact-shape child index

Status: ready-for-agent — unit 1 BUILT 2026-09-27 (the `organizations` delete, see below): gated, committed and
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
- **open**: unit 1 built, not deployed (2026-09-27).
