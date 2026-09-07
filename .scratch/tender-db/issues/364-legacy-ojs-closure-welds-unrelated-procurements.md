# 364 — the legacy OJS closure is an unbounded transitive closure over unguarded edges: 2,983 versions and 127 buyers in one Tender

Status: ready-for-agent (filed 2026-09-07 from the external review's verified findings; five verifiers reproduced every number below on prod)
Kind: defect (identity / grouping) — correctness, the CONTEXT.md:112-113 invariant
Relates to: 92 (records chain 3,282 only as a fold-performance cost, not as a correctness
signal), ADR-0011 (the eForms edge's three guards, which this edge has none of), ADR-0003
(no heuristic merging), 34 (the same "every notice sharing it collapses into one Tender"
failure for a folder id), and the sibling key defect (`procedure-key-accepted-unchecked`)

## Observed

**Tender 2816628** — `SELECT id, source, procedure_key, current_title, current_seq FROM tenders WHERE id=2816628` → `ted`, `ojs:2001-185105`, "Heizung- und Wasserinstallation", current_seq **2983**. `SELECT tender_id, MIN(published_at), MAX(published_at), COUNT(*), COUNT(DISTINCT publication_id) FROM tender_versions WHERE tender_id=2816628` → 2,983 versions / 2,983 distinct publication ids, 1297814400 (2011-02-16) → 1541721600 (2018-11-09). `SELECT role, COUNT(DISTINCT organization_id) FROM tender_version_parties WHERE tender_id=2816628 GROUP BY role` → buyer = **127** distinct organizations, winner = 107; buyer countries DE, PL, FR mixed. The notice payloads confirm the three named specimens: notice 4405514 (`052696-2011`, seq 1) `TED-ORGANISATION` = "Vermögen und Bau Baden-Württemberg, Universitätsbauamt Heidelberg"; notice 12339370 (`185105-2011`, seq 4) is PL with `TED-ACTIVITY_OF_CONTRACTING_ENTITY = EXPLORATION_EXTRACTION_COAL_OTHER_SOLID_FUEL` and a 112-lot title "1)Dostawa drewna kopalnianego …"; notice 20099441 (`493180-2018`, seq 2983) `TED-OFFICIALNAME` = "Krankenhaus Reinbek St.-Adolf Stift GmbH"; notice 12420678 (`266302-2011`) is French (SERS / Tribunal de grande instance de Strasbourg). `curl /v1/tenders/2816628` serves one German-titled record carrying a EUR `estimated_value` plus ~100 PLN coal-mine lot values.

**Tender 4812955** — procedure_key `ojs:2012-239732`, current_seq **2119**, 1343347200 (2012-07-27) → 1706659200 (2024-01-31); buyer = **182** distinct organizations, winner = 257; `SELECT COUNT(DISTINCT value) FROM tender_version_texts WHERE tender_id=4812955 AND field='title'` → **1,711** (1,445 at procedure level). Buyers: DB Netz AG, DB ProjektBau, DB Station & Service, DB Regio, DB Energie and the TEI-N/TEI-M/TEI-W/TEI-SO/TEI3 units — plus "Alliance élevage Loir-et-Loire" (FR), riding in on one stray edge.

**Blast radius** — one full-table aggregate: total **7,929,584** tenders; **43,088** with ≥10 versions; **2,574** ≥50; **212** ≥200; **15** ≥1000; max 3,282.

**The hub, measured**: notices 17283790 / 17284506 / 17284981 / 17285150 / 17285196 / 17285468 each carry their OWN distinct predecessor in `TED-REF_NOTICE.NO_DOC_OJS` but ALSO cite `TED-NOTICE_NUMBER_OJ = "2012/S 123-203577"`, so all six union onto node (2012, 203577) and become one component.

**The name is a typo's, and the node does not exist**: notice 12426474 (publication `273139-2011`) carries `TED-NOTICE_NUMBER_OJ = "2001/S 112-185105"` while its eleven sibling notices in the same component write `"2011/S 112-185105"`. `SELECT id FROM notices WHERE publication_id IN ('185105-2001','00185105-2001')` returns **0 rows** — the corpus holds no such publication — yet node (2001, 185105) is created, is the component's MIN, and therefore names all 2,983 versions.

## Why, exactly

1. **Every OJS ref is an edge, regardless of what field it came from.** The plan builder collects every parsed id with `scheme == "ojs" && is_ref` as a chain edge, with no regard for its origin — `crates/ingest/src/project.rs:3313-3325` ("The chain edges: every `is_ref` OJS-scheme id the notice carries").
2. **`NOTICE_NUMBER_OJ` is mapped as a ref globally.** `crates/ingest/src/r209/rules.rs:223` maps it to `Rule::Id(IdKind::Ref)`, and the parent-context table in the same file — `crates/ingest/src/r209/rules.rs:145-148`, which exists precisely to disambiguate `NO_DOC_OJS` ("the notice's own OJS number everywhere except inside REF_NOTICE") — has no entry for it. So a corrected notice's number (F14), a modified award's (F20), and a prior-information / qualification-system number that dozens of separate procurements were called under all become the same undirected same-procedure edge.
3. **The closure has no guard and no cap.** `insert_plan_tx` writes the edges symmetrically (`crates/store/src/canonical.rs:7051-7065`) and `build_plan_groups` unions both endpoints in a `MinUnionFind` with no existence check, no direction check, no buyer check and no size cap — `crates/store/src/canonical.rs:7151-7160` — then labels the component from the MIN endpoint as `ojs:{year}-{number:06}` (`crates/store/src/canonical.rs:7169-7171`), which is how a phantom endpoint gets to name and weld a component.
4. **The contrast is in the same file.** The eForms ADR-0011 edge joins through `PREV_EDGE_JOIN_SQL` (`crates/store/src/canonical.rs:1060-1070`), which requires the target to EXIST (`JOIN notices n ON n.source = e.a_source AND n.publication_id = e.b_publication_id`), the same source, and `b.published_at < a.published_at`. The legacy closure has none of the three. ADR-0011 records the omission as intentional ("the legacy OJS closure … deliberately admits not-yet-ingested edge targets so identity is stable as backfill deepens") — but with no counterweight, one mistyped digit permanently renames and can permanently weld.
5. **Nobody could see it.** CONTEXT.md:112-113 promises "transitive edges — a missed link splits a Tender, never wrongly merges", and the only standing component-size signal is `longest_chain` = `SELECT MAX(current_seq) FROM tenders` (`crates/ingest/src/data_quality.rs:715`), surfaced as `tender_db_dq_longest_chain` (`crates/app/src/v1/metrics.rs:248-254`) and framed by issue 92 as a fold-performance tripwire firing at ≥4,000. At 3,282 the gauge reads green while 212 tenders each fuse hundreds of unrelated procurements. No check anywhere counts distinct buyers, distinct titles or year span per tender.
6. **A weld is irreversible in served data**: the representative is `MIN(encoded ojs key)`, so growth only ever absorbs more.

## Units

1. **Decide the edge-admission rule** (owner's call, first unit): give the legacy edge the three guards its eForms sibling has — target-exists, same-source, strictly-earlier. Target-exists conflicts with ADR-0011's deliberate allowance, so either retire that allowance, or keep it and require a phantom endpoint to be well-shaped AND refuse it as component representative. Record the decision and its reasoning here.
2. **Context-gate `NOTICE_NUMBER_OJ`** through the mechanism `crates/ingest/src/r209/rules.rs:145-148` already provides for `NO_DOC_OJS`: an edge only where it means a predecessor; the notice's own number elsewhere. Pin with a fixture of the six-notice hub (17283790 …) — it must yield six components, not one.
3. **Measure the invariant.** A component-plausibility gauge in the weekly DQ report: per tender, distinct buyer organizations, distinct procedure titles, year span, with the top N listed. This is the detector for this issue *and* for `procedure-key-accepted-unchecked`; build it once. Issue 92's `longest_chain` stays what it is.
4. **Repair**: with the guards in place a re-projection re-derives components; re-measure the 212 ≥200-version tenders before/after and record. Expect a large change-feed burst (the 351 fold's shape).

## Done when

- the six-notice hub fixture splits into six components and a phantom-target edge cannot name a component;
- the plausibility gauge is in the weekly report, and 2816628 / 4812955 are either named in it or gone;
- the ≥200-version count is re-measured after re-projection and written here.

*One issue because:* the 2,983-version Heidelberg/coal-mine/Reinbek tender, the 2,119-version DB Netz tender, the phantom `ojs:2001-185105` key and the whole ≥10/≥50/≥200/≥1000 distribution are one edge rule feeding one unguarded union-find — the distribution is that mechanism's blast radius, and the missing gauge is why it stood.
