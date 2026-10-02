READ-ONLY. No repo edits and no cargo. Zero /v1/sql requests: the computed task gave no request count, so none were spent. One public GET of https://tenders.zebreus.click/v1/sql/schema was saved to .scratch/tender-db/481-dedup/ (data/, scripts/; working copy was the session scratchpad)schema.json. It lists columns only, no indexes, so every index below comes from the code.

# 1. How notices become Tenders

## Phase 1: one plan row per parsed notice
- Full run: `build_plan` (crates/ingest/src/project.rs:1562). A producer thread reads parsed chunks and calls `normalise_de1` (5720), then `Ident::read` (4114) and `into_plan_row` (4160). Mentions are resolved first, then `Db::insert_plan` writes the rows (canonical.rs:9365 → `insert_plan_tx` at 9558).
- The `Ident` struct (project.rs:3713) carries three identity regimes, documented at 3695-3712:
  - **procedure_key** = `procedure_key()` (project.rs:5865-5881).
    - BT-04-notice (const at 938) is taken unchecked if non-empty.
    - Otherwise the dialect folder id, but only when `is_uuid` passes (5884): `SDK01-ContractFolderID` (769) or `DE1-ContractFolderID` (936).
    - Non-uuid folder ids (the DÖE sdk-0.1 numeric channel and DE-1.x portal-local ids) produce no key, so the notice becomes an island.
  - **ojs_self / ojs_edges** (legacy TED only): `ojs_chain_edges` (4018) with the issue-364 citation-kind gate. Encoded `year*1e9+number` by `encode_ojs` (4066).
  - **prev_refs** = `previous_publications` (4074). These are OPP-090-Procedure ids (const at 966), normalised by `publication_ref` (4099) to the archive shape `NNNNNNNN-YYYY`. Anything else is dropped.
  - Also on the row:
    - `buyer_key` (5542): the parsed-side buyer set, as `country:kind:value` or the `n2:` name fallback.
    - `key_shaped` (`is_placeholder_key`, 5967).
    - `shared_kind`.
    - `source_rank` (4191): ted=1, everything else 0.
    - `published_at` = `notice_instants().0` (4134).
- Plan tables are transient. They are dropped and recreated per run by `clear_plan_on` (canonical.rs:9270-9354):
  - plan_notice (9282)
  - plan_refused_key (9309)
  - partial index plan_notice_fts_key (9319)
  - plan_ojs_node (9326; created but no longer written, see 9900-9906)
  - plan_ojs_edge (9330)
  - plan_prev_edge(a_notice_id, a_source, b_publication_id) (9337)
  - plan_group_merge(from_key PK, to_key) (9349)
- `insert_plan_tx` also writes two durable or edge things:
  - durable `legacy_ojs_keys` rows (9585-9596; table at canonical.rs:977, which survives `reset_tender_layer`);
  - one plan_prev_edge row per prev_ref (9599-9606), plus symmetric plan_ojs_edge rows (9612-9626).

## Grouping: `Db::build_plan_groups` (canonical.rs:9647), steps in order
1. **Placeholder gate (9676-9714).** A key is refused when `key_shaped=1` and the key has ≥3 distinct `buyer_key` sets (issue 369).
2. **FTS gate (9716-9755).** An FTS ocid with ≥2 buyer sets is refused (issue 386).
3. **Keyed / refused / island election (9768-9806).** A batched UPDATE CASE sets the group key:
   - `procedure_key` if the key is not refused;
   - `'refused:'||key||':'||buyer_key` (9793);
   - `'island:'||notice_id` (9797);
   - NULL for legacy notices that have an own OJS number.
4. **Legacy `ojs:` union-find (9809-9978).**
   - `MinUnionFind` (struct at 1340, `union` at 1370) runs over `SELECT a,b FROM plan_ojs_edge` (9865).
   - Edges touching a `shared_kind` endpoint are refused (9848-9876).
   - Naming: "a phantom may link but not name" (9921-9950). The component is labelled after the earliest existing notice, `ojs:{year}-{number:06}` (9968).
5. **ADR-0011 OPP-090 edge (9980-10182).**
   - ANALYZE (10018); edge count (10035).
   - Read the edges with `PREV_EDGE_JOIN_SQL` (1249-1258). Its guards:
     - target resolved in `notices` with `n.source = e.a_source` (same source only);
     - `b.published_at < a.published_at`;
     - `a.group_key <> b.group_key`.
   - Each group key is ranked by its earliest (published_at, publication_id) (10064-10076). Keys are sorted by rank (10095), and MinUnionFind runs over their positions (10097-10109), so the component root is the earliest-published key.
   - `from→to` pairs go into plan_group_merge (10115-10128), then a batched EXISTS relabel of plan_notice.group_key (10132-10172).
6. **Fold index (10190).** `plan_notice_fold(group_key, published_at, source_rank, publication_id, notice_id)`, then ANALYZE.

Counts: `plan_counts` (10213) and `plan_summary` (10237; it also collects the `ojs:` keys for retirement).

## Phase 2: one group → one Tender
- `next_plan_batch` (10263) streams whole groups in fold order as `PlanGroup` (6816), holding `notice_ids` and `sources`.
- `apply_plan_batch` (project.rs:2487) and the bucketed `fold_rows` (3011) both derive, identically (2518-2530 and 3023-3031):
  - `island_notice_id = group_key.strip_prefix("island:")`;
  - `procedure_key` = group_key otherwise (so `ojs:`, `refused:` and uuid keys are all stored as `tenders.procedure_key`);
  - `source = primary_source(sources)` (4202-4208): `'ted'` if any member is TED, else the first member's source;
  - `kind = kind_of(first_subtype)`;
  - `versions = fold(chain)` (4212).
- `fold` builds each version from the previous version's facts plus `supersede` (4284). Every version therefore holds the cumulative state.
- `Db::apply_tenders` (canonical.rs:11813) calls `tender_identity` (12100):
  - Non-rebuild: a keyed Tender is looked up by `procedure_key` (12124); an island by `(source, island_notice_id)` (12130-12134). `tenders.source` is updated when it flips to TED (12141-12146).
  - Rebuild: always a fresh insert.
- Retirement:
  - full path: `retire_absorbed_legacy_tenders` (project.rs:1486, canonical.rs:25845) and `retire_regrouped_nonlegacy_tenders` (project.rs:1493, canonical.rs:26118);
  - incremental: `retire_regrouped_tenders` (project.rs:2386, canonical.rs:26064). A touched Tender whose key `procedure_key` / `island:<id>` is absent from plan_notice gets `removed` events.

## Incremental path: `project_incremental_chunked_observed` (project.rs:2147)
- Pass 1 (2205-2232) collects only `procedure_key`s and legacy OJS seeds. It does not collect prev_refs.
- `legacy_closure` (2043-2117) walks the durable legacy_ojs_keys to a fixpoint, with a cap and a fallback to the full path.
- Touched expansion (2287-2305):
  - `touched_existing_tender_ids` (canonical.rs:25996): Tenders that hold a changed notice, plus Tenders whose `procedure_key` equals a new key;
  - `notice_ids_for_tenders` (26032).
- The same `build_plan_groups` then runs (2377) over the touched set only. canonical.rs:9996-10000 states the consequence: an OPP-090 target outside the touched set "unions nothing". ADR-0011 edges are effectively applied on full re-projections only.

**Stale doc:** ADR-0011's header still says "Implementation pending", but the code above implements it.

# 2. How a merged TED+DÖE Tender is represented
- **`tenders` row (canonical.rs:108):**
  - `procedure_key` = the shared BT-04 uuid (or the sdk-0.1/DE1 uuid folder id); `island_notice_id` is NULL;
  - `source = 'ted'` (`primary_source`), flipped in place when a TED twin arrives (12141).
  - Identity indexes are deferred: `tenders_procedure_key(procedure_key)` (8866) and `tenders_island(source, island_notice_id)` (8867).
- **`tender_versions` (canonical.rs:185):** one row per notice of either source.
  - PK (tender_id, seq); UNIQUE (tender_id, caused_by_notice_id).
  - `seq` follows plan_notice_fold order: published_at, then source_rank (TED folds last on a tie), then publication_id, then notice_id.
  - There is no source column. The source of a version is caused_by_notice_id → notices.id → notices.source. The `v_tender_notices` view does that join (1127-1132), but /v1/sql refuses filtered views, so do the join client-side: `tender_versions WHERE tender_id=?`, then `notices WHERE id=?` by PK.
  - `publication_id` shapes differ: TED is `NNNNNNNN-YYYY`; DÖE is `<notice-uuid|numeric>-<VersionID>` (profile.rs:337-340, `notice_id_and_version` at 628). Exception: a DÖE notice that carries a real efac `NoticePublicationID` takes the TED number as its publication_id (placeholder `00000000-1900` is filtered at profile.rs:383).
  - Indexes: `tender_versions_notice(caused_by_notice_id)`, `tender_versions_published(published_at)`, `tender_versions_publication(publication_id, tender_id)` (deferred list at 8794-8800).
- **Existing metric:** `MERGE_SQL` (data_quality.rs:606-612) counts DÖE-touching Tenders that have any version whose notice is TED.

# 3. Where a source-agnostic edge ledger plugs in

**A. Durable table.** Add it to the canonical SCHEMA next to legacy_ojs_keys (canonical.rs:977-987), not among the plan tables in `clear_plan_on`.
- Key it by notice, never by tender id, since Tender ids are retired on merge.
- Suggested shape: `(a_notice_id, b_notice_id NULL | b_source + b_publication_id, kind declared|matched, rule, evidence JSON, job_id, at)`.
- Indexes needed: on `a_notice_id`, on `b_notice_id`, and on `(b_source, b_publication_id)`, so the incremental closure can walk both directions.
- Declared producers fill it at the same choke point that writes legacy_ojs_keys (`insert_plan_tx`, 9585-9606): today's prev_refs, plus new parser cross-references. Matched rows are written by a dry → review → wet job (the org_merge_log precedent, 466).

**B. Grouping.** Plug in at the ADR-0011 step (canonical.rs:9980-10182), after the keyed/island election (9768) and the legacy pass (9978) so every notice already has a group_key, and before the fold index (10190).
1. Generalise plan_prev_edge (DDL 9337; insert 9599) to carry `b_source` and/or `b_notice_id` plus `kind`.
2. Load the ledger rows whose `a` endpoint is in plan_notice.
3. Replace `n.source = e.a_source` in PREV_EDGE_JOIN_SQL (1252) with the edge's own target source. **This is today's cross-source drop:** a DÖE OPP-090 citing a TED number is looked up among DÖE notices and never found.
4. Make `b.published_at < a.published_at` kind-specific. Matched and cross-source edges are undirected; the union-find needs no direction.
5. Push every edge kind into the one `edges` vector before the union (10097-10109). That alone makes the fold union transitively across declared, matched, OJS and BT-04 links. The plan_group_merge relabel needs no change.

**C. Representative rule (needed before any cross-source merge).** The rank at 10064-10096 is the earliest (published_at, publication_id).
- A DÖE island usually publishes before its TED twin, so the merged Tender would be named `island:<doe notice>`.
- The TED uuid Tender would then be retired and re-minted as an island.
- On non-rebuild runs, `tender_identity`'s island lookup `(source='ted', island_notice_id)` (12130-12134) misses the stored `source='doe'` island row and inserts a new one. `retire_regrouped_tenders` keeps the old row because `island:X` is still in the plan (26080-26091). The result is a duplicate Tender with the issue-278 ghost signature.
- Fix: rank by `(is_island, published_at, publication_id)` so a keyed member names the component.
- Weld guards belong at the same point, tallied into the Report like `target_refusals`:
  - refuse matched edges that would join two distinct TED keys;
  - cap component size;
  - add a buyer-set disagreement check in the plan_refused_key style.

**D. Incremental path.**
- Add a ledger closure beside `legacy_closure` (project.rs:2043). Seeds are the changed notices plus the endpoints of ledger rows added or removed since the last fold. Each hop goes ledger → notices → `tenders_for_notice_ids` (canonical.rs:9508) → `notice_ids_for_tenders`, under a cap with a full fallback.
- Merge the result into touched_tenders at project.rs:2287-2305.
- A ledger write or delete must set `notices.projected=0` on both endpoints, so a wet run or an undo reaches the next daily fold.
- Retirement (2386, 1486/1493) already handles absorbed keys.
- The tender_identity key lookup (12124), the fold order and `primary_source` need no change.

# 4. Matchable per-version signals
**Carry-forward caveat:** the `tender_version_*` satellites store CUMULATIVE state per seq (fold, project.rs:4212-4216). For merged positives, a later seq mixes in the other source's facts. Read each notice's own values from the notice layer, or use seq 1 of an unmerged Tender. Only parties keep provenance per row (`mention_notice_id`). All notice-layer tables have PK `(notice_id, section_id, field_id, ordinal)`, which is a seek by notice_id.

| Signal | Canonical table / columns | Canonical indexes | Notice layer and field ids |
|---|---|---|---|
| Buyer org id | `tender_version_parties` (canonical.rs:367): tender_id, seq, lot_id, role, organization_id, mention_notice_id, mention_section_id. Buyer roles: `Procedure-Buyer` (eForms, DE2, DE1 alias) and `buyer` (legacy, sdk-0.1); BUYER_ROLES at project.rs:5513; `v_tender_buyers` uses `LIKE '%uyer%'` | `_version(tender_id, seq)` (386); `_org(organization_id)`; deferred `_org_role(organization_id, role, tender_id)` (8833) and `_mention_key(mention_notice_id, mention_section_id)` | `organization_mentions` (433): PK (notice_id, section_id), organization_id, name, country, raw_identifier, scheme; indexes `_org`, deferred `_notice(notice_id)`. `organizations` (413): country, identifier_kind, identifier, name, name_norm, provisional; deferred `organizations_identifier_id(identifier, id)` (8593), `_name_norm_id`, `_name_country(name_norm, country)`, `_country_id`, `_kind_id`. `organization_merged_identifiers` (PK identifier, loser). `notice_ids` is_ref=1 rows under `OPT-300-Procedure-Buyer`, or sdk-0.1 `ContractingParty` sections (`notice_sections_kind_notice(kind, notice_id)`). `buyer_key()` (project.rs:5542) is a ready normaliser. The org layer has a measured duplicate floor of 2 (issue 369), so compare normalised identifiers, not only org ids |
| Buyer name | `organizations.name` / `name_norm` | as above | `organization_mentions.name` (as published); `organization_names` (PK org_id, lang); `match_norm` (project.rs:5398) |
| Title | `tender_version_texts` (254): field='title' / 'description', lang, value, lot_id NULL = tender scope. Head copy `tenders.current_title` (not indexed) | `_version(tender_id, seq)` (263). No text index (ADR-0016) | `notice_texts` (lib.rs:321): BT-21-Procedure / BT-21-Lot (eForms, DE2); DE1-ProcurementProject-Name (stored under the DE1 id, aliased only at fold, project.rs:801); SDK01-ProcurementProject-Name; TED-TITLE_CONTRACT / TED-TITLE |
| CPV main | `tender_version_classifications` (345): field='main', scheme='cpv', code (split by `normalize_cpv`, project.rs:5291) | `_code(scheme, code)` (354); `_version(tender_id, seq)` (362) | `notice_classifications` (lib.rs:346), index `(scheme, code)` (355): BT-262-*, SDK01-…MainCommodityClassification-ItemClassificationCode, DE1-…MainCommodity…, TED-CPV_CODE |
| NUTS | same table, field='place', scheme='nuts' | same | BT-5071-*, SDK01-/DE1-…RealizedLocation-Address-CountrySubentityCode, TED-NUTS / TED-PERFORMANCE_NUTS |
| Submission deadline | `tender_version_dates` (333): field='submission_deadline', utc_seconds, offset_minutes, has_time. Head `tenders.current_deadline` | `_version` (343); deferred `tenders_current_deadline(current_deadline, id)`, which allows a range seek for candidate generation | `notice_dates` (lib.rs:373): BT-131(d)-Lot / -Procedure, DE1-…TenderSubmissionDeadlinePeriod-EndDate, SDK01-ProcurementProjectLot-TenderingProcess-TenderSubmissionDeadlinePeriod-EndDate |
| Estimated value | `tender_version_amounts` (265): field='estimated_value', cents, currency, tax_basis, eur_cents, quality ('withheld'). Head `tenders.current_value_eur_cents` is the MAX over all amounts, not just the estimate | `_version` (296); deferred `tenders_current_value_eur(current_value_eur_cents, id)` | `notice_amounts` (lib.rs:359): BT-27-*, DE1 alias, SDK01-…RequestedTenderTotal-EstimatedOverallContractAmount / TotalAmount |
| Lot count | `tender_version_lots` (217): PK (tender_id, seq, lot_id), kind Lot / LotsGroup / Part. `lots` (208): UNIQUE (tender_id, lot_key); lot_key is e.g. LOT-0001 | PK | `notice_sections` (lib.rs:301): kind 'Lot', or DE1 'ProcurementProjectLot'; index `notice_sections_kind_notice(kind, notice_id)` |
| Publication / dispatch date | `tender_versions.published_at`, `dispatched_at`; head `tenders.current_published_at` | `tender_versions_published`; deferred `tenders_current_published(current_published_at, id)`. dispatched_at is not indexed | `notices.published_at` / `dispatched_at` are not indexed; walk by id window via deferred `notices_source_id(source, id)` |
| BT-22 internal reference / Vergabenummer | **not projected** | none | Only `notice_ids` (lib.rs:409): BT-22-Procedure / BT-22-Lot (eForms incl. DE 2.x), DE1-ProcurementProject-ID, SDK01-ProcurementProject-ID, legacy TED-REFERENCE_NUMBER. The partial index `notice_ids_target(value) WHERE is_ref=1` (419) does not cover them (is_ref=0), so no value seek exists |

## Declared-link carriers that are parsed today but not followed
- **OPP-090-Procedure** (`notice_ids`, is_ref=0) on DÖE eForms-DE 2.x notices. Dropped by the same-source join.
- **DE1-TenderingProcess-NoticeDocumentReference-ID.** Not aliased (absent from DE1_FIELD_ALIASES), so unused.
- **DE1-Publication-NoticePublicationID / GazetteID**, and DÖE `notices.publication_id` when it is OJS-shaped. Seekable through `notices_publication_id_id(publication_id, id)` and `UNIQUE(source, publication_id, content_hash)`. Counting these at corpus scale has no bounded single-index seek, because a digit-prefix range also walks every TED row. That needs a bounded design.
- **SDK01 / DE1 lot-level `…NoticeDocumentReference-ID`** (the BT-125 analogue).

## Above-threshold filter fields (notice layer only)
- BT-01-notice → `notice_codes`.
- DE1-RegulatoryDomain → `notice_texts`.
- SDK01-RegulatoryDomain → `notice_codes`.
- Subtype: `tender_versions.notice_subtype` (OPP-070-notice; DE1 alias at project.rs:796).

## Calibration notes for unit 1
- The labelled positives all share a uuid key. The targets are the sdk-0.1 numeric and DE1 non-uuid islands, whose field coverage may differ.
- DÖE months before TED's switch to eForms (TED r209 era) have TED twins keyed `ojs:`, not uuid. That changes which pairs can exist in an early-DÖE month.