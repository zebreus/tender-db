# 481 — cross-source Tender dedup is exact-key only; how many duplicates it misses is unmeasured, and there is no source-agnostic edge for future portals

Status: ready-for-agent — UNIT 2c (the buyer guard made precise) LANDED 2026-10-02, not yet deployed: two notices also share a buyer through one resolved organization, one raw identifier whatever its scheme, the contract signatory, an agency's principal, a whole-word prefix or a head of three content words, and an acronym; no extra refusal for disjoint jurisdictions. Job 1893's 56 samples re-verdicted: 14 of 19 legitimate pairs now overlap (18 clear + 1 probable; the 5 left are framework call-offs filed by another body or in Galician, a mid-name abbreviation, and one CN naming another body), every false merge and group-PIN/qualification-system hub stays refused. NEXT: deploy → `backfill-tender-links` dry → read `would_split` / `buyer_disjoint` and re-sample 30 of each → wet (if the false-refusal share is low) → the next daily splits the would-split welds; read its `issue-481` line (`buyer-disjoint`, largest component).
Was status: ready-for-agent — UNIT 2b (the buyer guard on previous-notice references) LANDED 2026-10-02 with its review fixes (tokens take one buyer's measured spellings: E1 identifier keys, every language variant, accents folded, register country; the census counts `not_earlier` apart from the joins), not yet deployed; unit 2 deployed 2026-10-02 09:15 UTC (`7b14469`), its wet backfill HELD for 2b. NEXT: deploy → `backfill-tender-links` dry → read `buyer_disjoint` / `would_split` and their samples (two placeholder hubs are already on prod: Tender 1012301, 184 versions from 60 buyers, and SCB's 526284 with six copiers) → wet → the next daily splits the would-split welds; read its `issue-481` line (`buyer-disjoint`, largest component).
Was status: ready-for-agent — UNIT 1 (CALIBRATION) DONE 2026-10-02 (workflow `wf_09fa7411-6db`; report `.scratch/tender-db/481-dedup/calibration-2026-10-02.md`). The TED↔DÖE misses are DECLARED links the fold does not follow, not fuzzy ones. TED `BT-701-notice` equals the DÖE notice UUID on 136 of 136 above-threshold DÖE islands in April 2025 (1.08 % of the month's merged count; ~2,650 extrapolated). Cross-source `OPP-090` links are dropped by a same-source condition. The best matched rule R1 measured 0 FP / 1,289 negatives at 98.0 % recall, but adds 0 joins beyond the declared link. NEXT: unit 2, the edge ledger with the `notice_uuid` and cross-source `OPP-090` producers and the fold reading it (incremental path included), plus the weld guards. The UUID-collision false merges it surfaced are issue 482.
Was status: ready-for-agent — DECIDED 2026-10-02 (Lennart: "fuzzy matches are probably fine if we are really really sure it's the same one. Nothing is deliberately forbidden if it is correct"; recorded as ADR-0003's 2026-10-02 amendment). A matched link is a merge warrant when its precision is measured near-certain. The first unit is calibration: measure candidate signals against the 243,588 UUID-merged TED↔DÖE pairs (labelled positives) and same-buyer different-procedure pairs (labelled negatives), then count the unmerged DÖE Tenders that a near-certain matcher would join.
Was status: ready-for-agent — filed 2026-10-02 from Lennart's question ("do we have proper general deduplication/merging,
between TED and DÖE, and between any current and future portals?"). The first unit is the measurement: count TED↔DÖE
duplicates the key rule misses.
Kind: data model (cross-source identity, ADR-0003)
Relates to: ADR-0003 (merge only on a strong explicit cross-reference, never heuristic), ADR-0011 (OPP-090
previous-notice edge, intra-TED), 12 and 34 (the BT-04 / sdk-0.1 ContractFolderID key), 369 (procedure-key shapes), 480
(Contracts Finder ↔ FTS, no machine link), 300 (organization matching)

## What exists (2026-10-02)

- **Tenders merge across Sources on an identical procedure key.** This means an eForms BT-04 UUID, or DÖE sdk-0.1's
  ContractFolderID when it is a UUID (issues 12 and 34). The `tenders.procedure_key` UNIQUE constraint does the merge,
  and precedence is ADR-0003's fold. The weekly data-quality report (job of 2026-09-29) reads **DÖE procedure Tenders
  921,031, merged with TED 243,588 (26.4 %)**.
- **Inside TED:** legacy OJ-number chains, plus ADR-0011's `OPP-090` previous-notice edge.
- **TED ↔ FTS:** they never overlap (ADR-0003, verified 2026-09-07).
- **Organizations** dedup across every Source, on identifiers plus the merge rules: r2, e0, p0, r3, e2-altid, rekey.

## What is missing

1. **The misses are unmeasured.** A procedure published on both portals without the shared key stays two Tenders.
   - DÖE's sdk-0.1 numeric channel produces islands; 98 % of its awards are unchained (dashboard `award_linkage`).
   - The 26.4 % rate is expected to sit far below 100 %, because DÖE carries under-threshold procedures TED never
     sees. But nothing counts the above-threshold DÖE procedures that have a TED twin and failed to merge.
2. **No source-agnostic declared edge.** Today the merge key means "same procedure key", which works for any
   eForms-family portal that reuses BT-04. It does not cover a portal that only *cites* another publication: an
   OJ/TED number in a national notice, an FTS notice id in a Contracts Finder release, or a SIMAP/Doffin
   cross-reference. ADR-0011's edge is the right shape, but it is wired for `OPP-090` → TED numbers. A future Source
   would need its own code path rather than emitting "this notice declares that notice of Source X".
3. **No matched merges.** Until 2026-10-02, ADR-0003 forbade heuristic merging, so a duplicate with no published link
   stayed two Tenders forever. The amendment replaces that with "merge when measured near-certain". Below the
   near-certain band, a "possible duplicate" signal is still worth serving.

## Units

1. **Measure (first).** For one recent month: take the DÖE Tenders without a TED version that are above threshold
   (eForms-DE with an EU legal basis, or sdk-0.1 with a TED-reference field). For each, look for a TED Tender with the
   same buyer identifier, an equal or near title, and a publication date within ±7 days. Count the candidates and
   read 30 by hand. This is a dry job and writes nothing. Also count DÖE notices that carry a TED publication number
   (`OPP-090`, BT-?? "previous notice", the sdk-0.1 TED-reference field if one exists) and are not on the TED twin's
   Tender. Each such row is a declared link that is not followed today.
2. **Generalise the edge.** A `notice_crossrefs(notice_id, target_source, target_publication_id, kind)` table that any
   parser fills from a published reference. The fold resolves it to a Tender and joins it under ADR-0003/0011's
   guards. TED's `OPP-090` becomes one producer of it, and DÖE, Contracts Finder (480) and future portals are others.
3. **The matched-link rule.** Write the matcher's admitted band as edge rows (`kind = matched`, rule name, evidence,
   job id) into the same ledger unit 2 builds, through dry → review → wet with parity, as the org arms do. The fold
   unions on edges exactly as it does on declared ones. Below the band, serve `possible_duplicate_of` (never merged)
   and count it on the dashboard.

## Verify

    ssh -o BatchMode=yes root@zebreus.click "/root/aj.sh /admin/reports/data-quality" | python3 -c 'import sys,json; b=json.load(sys.stdin)["body"]; i=b.find("merged with TED"); print(b[b.rfind("\n",0,i-60):i+40].strip())'

- **open** (2026-10-02): `DÖE procedure Tenders: 921,031; merged with TED: 243,588 (26.4%)`, with no count of
  missed twins anywhere.
- **done:** unit 1's measured miss count recorded here and, once units 2 and 3 land, on the weekly report beside the
  merged count.

## 2026-10-02 03:2x UTC — unit 1: calibration (April 2025)

Full report, code map, read shapes, per-lens analyses, scripts and the gzipped samples:
`.scratch/tender-db/481-dedup/`. Every prod read was a bounded single-table `/v1/sql` SELECT: 294 requests,
0 error bodies, 0 HTTP 408.

**Sample.** In April 2025, 26,169 Tenders carry a DÖE version: 12,513 merged with TED (98.9 % of eForms-DE) and
13,656 with no TED version. The 13,656 split as 12,763 sdk-0.1 numeric, 702 sdk-0.1 UUID, 132 eForms-DE and 59 EU
SDK. Labelled data:
- **458 positives:** UUID-merged pairs, stratified by eForms-DE version.
- **1,289 negatives:** different Tenders with the same buyer, published within ±30 days.
- **817 unmerged records,** with 10,012 TED candidates.

**Findings.**
1. **The miss is a declared link.** 136 above-threshold DÖE-only Tenders (eForms-DE 1.1, 1.2, 2.0 and T01;
   prior-information and T01 islands) have exactly one TED twin. In every case TED `BT-701-notice` equals the DÖE
   `publication_id` minus its `-NN` suffix, and all 136 agree on title, CPV, subtype, buyer and dispatch second.
   Extrapolated ~2,650 across the population (interval 2,220–3,130), which would move "merged with TED" from 26.4 %
   to ~26.7 %.
2. **Cross-source `OPP-090` is dropped.** `PREV_EDGE_JOIN_SQL` (canonical.rs ~1252) requires
   `n.source = e.a_source`, so a DÖE notice citing its TED predecessor is never joined.
3. **Matched rule R1:** org & exact normalised title & same subtype & Δ ≤ 7 d & no contradicting field &
   (deadline equal | dispatch equal to the second, off the hour | dispatch on the hour & internal ref equal).
   - Recall 449/458 = 98.0 % [96.3; 99.0].
   - 0 FP in 1,289 negatives (per-pair upper bound 0.30 %); precision Wilson lower bound 99.15 %.
   - **Marginal yield over finding 1: 0** in the sample (95 % upper bound ~72 in the population).
   - Fuzzy titles added 0 recall and only false matches. Similarity ≥ 0.95 took the negatives from 12 to 28.
4. **sdk-0.1** (13,465 DÖE-only Tenders a month): 0 joins under every rule. No numeric record shares an org id with
   TED, and there are 0 equal titles and 0 shared documents URLs across 2,637 pairs. Whether its ~330 EU-basis
   records a month have TED twins at all is unknown without hand labels.
5. **False-merge shapes** (report §4, 16 rows): lots published as separate procedures under one title (Bezirkskliniken,
   42 of 49 pairs wrong under title rules), framework lots changed the same day, repeated modifications under new
   BT-04s, re-published PINs, project-id internal refs, sibling trades sharing a deadline, the same dispatch second
   on different notices, and **UUID key collisions in the existing declared rule: issue 482**.

**Decision (owner, 2026-10-02).**
- Build the declared links first. They are exact identifiers, which ADR-0003 always admitted.
- R1 is **not** admitted as a merge rule now. Its measured yield beyond the declared link is 0, and a rule that joins
  nothing new adds risk without benefit. It stays the candidate generator for `possible_duplicate_of` (L5 precision
  0.9933, LB 0.9805), and is re-measured after unit 2 lands, on another month and on PINs with full dispatch data.
- sdk-0.1 needs a labelled sample before any rule. That is a later unit.

## Unit 2: the edge ledger and its declared producers — LANDED 2026-10-02 (not yet deployed)

**What landed** (two commits: A `ca95200` ledger, producers and full-path fold; B the incremental path, the
re-queue rule and the backfill job):
- **Ledger** `tender_links(id, a_notice_id, b_notice_id NULL, b_source, b_ref, kind declared|matched, rule,
  evidence, job_id, at)` in the canonical SCHEMA (canonical.rs, beside `legacy_ojs_keys`). Off `/v1/sql`.
- **Producers** at `insert_plan_tx`, diffed per planned non-legacy notice: `opp-090` (target Source always TED,
  so the cross-source drop is gone) and `logical-notice` (TED BT-701, a single non-placeholder uuid, resolved to
  every DÖE `<id>-<digits>` version). An unresolved reference keeps one row with `b_notice_id` NULL.
- **Fold**: every rule in the one `MinUnionFind`. Previous-notice keeps ADR-0011's direction check; same-notice
  and matched links are weld-guarded (one logical id per citing component, no two keyed components,
  `LINK_COMPONENT_CAP = 64`). Representative: keyed, then TED island, then earliest — no issue-278 ghost.
  `LinkTally` on the Report and the job row (`; issue-481 tender links joined … refused … deferred …`).
- **Incremental path** (`project_incremental_chunked_observed`): pass 1 collects the changed notices' declared
  links; after the touched expansion and the legacy closure, `link_closure` walks the ledger from EVERY planned
  notice (rows they state, rows naming them by id, unresolved rows naming them by publication id or its version
  stem) plus the changed notices' links resolved now, adding each reached Tender whole, to a fixpoint; past
  `LINK_CLOSURE_CAP` (= the legacy cap, 500k) it falls back to the full path. Proven equal to a full fold in
  both arrival orders (`the_ledger_joins_incrementally_exactly_as_a_full_fold_in_either_order`; it fails with
  the closure disabled) and for a later notice under an absorbed key (review fix below; keys absorbed before
  this binary are a follow-up).
- **Re-queue rule** ("a write or delete un-projects both endpoints"; the change-set key is `notices.projected =
  0`, read by `unprojected_parsed_notice_ids`): `Db::write_matched_links` / `Db::delete_matched_links` (unit 3's
  write path) and the backfill re-queue both notices in the same transaction as the row — a written row only when
  its notices do not already share a Tender, a deleted row always. Inside a fold, the grouping re-queues the far end
  of any link the plan holds only one end of and holds back the guarded joins beside its near end
  (`LinkTally::deferred`; zero when the closure is complete and the ledger attested).
- **Backfill job** `backfill-tender-links` (dry default; report `tender-link-backfill`): walks parsed non-legacy
  notices by id, 5,000 notices / 500k ids per window, loading only the link fields (BT-701, OPP-090 and their
  DE-1.x spelling) through one `notice_ids` range read per window whose `field_id` filter is read off the PK index
  entry (turso `DeferredSeek`, checked in bytecode). Derives links with the plan's own `declared_links`, diffs
  with the producer's own diff, writes each window in one short transaction and re-queues only would-merge pairs
  (pairs already one Tender by BT-04 re-queue nothing). Counts per rule (declared, present,
  resolved, unresolved, would_merge, stale), notices walked, re-queued, and 30 sampled would-merge pairs by both
  publication ids. Zero drift: a full re-plan after the backfill rewrites no ledger row (tested). Runtime estimate
  15–45 min wet on prod, about half dry (measured 0.73 / 1.55 ms per notice in the O0 test build at prod's id-row
  density; reasoning in `docs/operations.md`); the dry run's job row is the real measurement.
- **Deviations from the design below:** the producer is named `logical-notice`, not `notice_uuid`; no
  dispatch-second check (DÖE versions of one id carry different dispatch instants; the guards carry the weld
  protection); matched writes go through a store API rather than a trigger (no trigger exists in this codebase,
  and a trigger would fire per producer row on a full re-projection).

**Deploy order:** deploy → `backfill-tender-links` dry → read `would_merge`, `opp-090`'s `cross_source`,
`ledger_bytes` (about 0.56 GB expected; check free disk) and the samples → wet (attests the ledger complete) → the
next `project` (daily) joins the would-merge pairs → read the job row's `issue-481` line (largest component,
refusals, `deferred 0`). No full re-projection needed. Dailies between the deploy and the wet run hold every
fan-in- and weld-guarded join back (`deferred`; the wet run re-queues them); ADR-0011's same-Source joins go ahead.
Expect one-time renames: a pre-ledger OPP-090 Tender named after an island member is retired (`removed`) and
re-minted under its keyed member the first time a fold plans it.
*Owner check 2026-10-02 08:3x UTC, before deploy:* the rename is negligible. Seven 100k-id ranges of `tenders`
(1.0M, 2.0M, 3.0M, 5.0M, 6.0M, 7.0M, 8.7M; one bounded PK-range COUNT each) hold **0** island-named Tenders with
`current_seq > 1`, the only shape the keyed-first rank can rename. Islands are 0–536 per range, all single-version.
The weld guard reads "root is keyed" off that same order, so the order stays.

**Adversarial review fixes (2026-10-02, third commit):**
- *A notice under a key a link merge absorbed split off on the daily* (two reviewers, major): the merged Tender is
  named after the other key, and the touched expansion looked new keys up by `tenders.procedure_key` only. Fixed:
  the grouping records every absorbed procedure key in `tender_key_merges(from_key, to_key)` (a full plan replaces
  it, an incremental plan the rows of the keys it holds), and `touched_existing_tender_ids` resolves through it.
  Test `a_notice_under_an_absorbed_key_finds_the_merged_tender_on_the_daily` (TED→TED and DÖE→TED; failed before).
- *Matched rows had no notice-level one-to-one guard* (major, calibration §4 shape 16): fixed in the fold — a notice
  whose matched partners sit in two components has every match refused (`not-one-to-one`). Test
  `a_notice_matched_to_two_notices_joins_neither`. Unit 3 writes one row per logical pair.
- *Cross-source OPP-090 was unguarded and uncounted* (major): fan-in guard — cross-Source references into one target
  from two or more keyed components are all refused (`fan-in`); admitted ones counted `cross-source` on the job row;
  the backfill counts `cross_source` per rule. ADR-0011's same-Source mechanism unchanged. Test
  `two_doe_procedures_citing_one_ted_notice_stay_apart`.
- *A same-notice link could rename a merged Tender* (minor): a keyed key's rank now comes from its previous-notice
  links only (ADR-0011's input). Test `a_same_notice_link_does_not_rename_a_merged_tender`. The one-time rename of
  island-named pre-ledger components is documented (ADR-0011 amendment, operations.md, deploy order above).
- *Unparsed notices entered incremental plans* (minor): link targets resolve among parsed notices only, and the
  closure keeps parsed notices only. Test `a_link_to_an_unparsed_notice_joins_only_once_it_parses`.
- *The daily admitted guarded joins a full fold refuses* (minor): a guarded join beside a one-ended link waits
  (`deferred`), and every fan-in- or weld-guarded join waits until `projection_state.tender_links_complete` (set by
  a full plan build or a finished wet backfill; at open on a file with no notice). The backfill's `would_merge` now
  counts held rows too, so a join a pre-attestation daily held back is re-queued. Refusals never wait. Tests
  `a_guarded_join_beside_a_one_ended_link_waits_for_both_ends`, `the_daily_holds_guarded_joins_until_the_ledger_is_attested`.
- *Prod-scale* (minor): matched writes/undos commit per 5,000 links with prepared statements; the closure's by-name
  read is prepared once (the superset of names kept on purpose); the one-ended pass runs on incremental plans only
  (`PlanScope`) and ignores a link whose far end is unparsed (no plan holds it; a full fold drops it too); the
  ledger's measured 229 B/row is on the backfill report (`ledger_bytes`) and in operations.md.
- *Largest component only on stderr* (minor): `LinkTally::largest_component` (max across chunks) on the job row.
- All six new behaviour tests were checked to fail with their fix reverted.

**Follow-ups (deferred by the review fixer):**
- **Dispatch-second check on `logical-notice`** (review minor; the design's consistency check). Not cheap: it needs
  `dispatched_at` on the plan row, a measurement of how often DÖE eForms versions carry no or hour-grain dispatch
  (calibration §6 lists this as unmeasured), and its own refusal counter. The suggested shape — resolve the link only
  when at least one DÖE version's dispatch equals the TED notice's, then join every version of the stem — keeps
  "all versions join". No BT-701 collision is measured; the one-to-one, keyed-weld, cap and placeholder guards stand.
- **`run_project`'s issue-467 poll budget is nearly spent**: it needs between 462 and 472 KiB against its 472 KiB
  budget (re-measured 2026-10-02; 449 before unit 2, so A/B's link step and closure grew it, and the review fixes
  pushed it over until the link step was boxed as `Db::link_step`). The next growth on the incremental path trips
  the test: box the link closure (`link_closure_capped` → `tender_link_neighbours`) or the three
  `incremental_full_fallback` sites before raising the number.
- **Keys absorbed before this binary** (the 2026-08-20 re-projection's ADR-0011 merges, and island-named
  components): no `tender_key_merges` row until a fold plans their Tender or the next full projection writes them
  all. Until then a later notice under such a key folds apart exactly as before the deploy (pre-existing, not a
  regression). Bundle the full non-rebuild projection with the next pending one rather than paying a window for it.

Design from the code map (`481-dedup/code-map.md`):
- **Table:** `tender_links(a_notice_id, b_notice_id NULL, b_source, b_publication_id, kind declared|matched, rule,
  evidence JSON, job_id, at)`, in the canonical SCHEMA beside `legacy_ojs_keys`, so it survives `reset_tender_layer`.
  Keyed by notice, never by Tender id. A write or delete un-projects both endpoints.
- **Producers:**
  - the existing `OPP-090` edge (`prev_refs`), now allowed to cross Sources;
  - a new `notice_uuid` producer: TED `BT-701-notice` = DÖE notice UUID, required one-to-one, with the dispatch
    second as its consistency check.
- **The fold:** generalise `plan_prev_edge` / `PREV_EDGE_JOIN_SQL` to the edge's own target Source. Feed every edge
  kind into the one `MinUnionFind`. Rank the representative by `(is_island, published_at, publication_id)`, so a DÖE
  island published first does not name the component (no issue-278 ghost). Add the incremental path: a ledger
  closure beside `legacy_closure` (project.rs ~2043), with a cap. Today ADR-0011 edges apply only on full
  re-projections.
- **Weld guards:** refuse an edge that joins two keyed Tenders or pairs one notice with two, cap the component size,
  and tally refusals in the Report.
- **Tests:** a BT-701 pair merges; a cross-source `OPP-090` resolves; an island published first does not ghost; a
  ledger row merges on the next incremental fold and un-merges when deleted; the weld refusals; one fixture per
  false-merge shape that must stay apart.

## 2026-10-02 09:1x–09:5x UTC — unit 2 deployed; backfill dry read; wet HELD for a buyer guard (unit 2b)

- Gate on `cdbad21`: GATE-EXIT=0, 145 suites, 843 s, marker written. Deployed `7b14469` at 09:15 UTC on an idle
  queue (health green, journal `-p err` empty).
- `backfill-tender-links` dry, job 1882: **585 s**. 3,869,187 notices walked, 2,403,802 declaring, ~610 MB of ledger
  rows.
  - `logical-notice`: 2,394,418 declared, 463,759 resolved, 1,930,659 unresolved (TED notices with no DÖE twin), and
    **5,050 would merge**, all cross-source. That is about 2× the calibration's ~2,650 Tenders: a pair is a link,
    and DÖE versions -01/-02 of one id are two links.
  - `opp-090`: 267,415 declared, 181,222 resolved, **403 would merge** (67 cross-source).
  - The progress text says "re-queued" on a dry run. It means "would re-queue": the dry run wrote nothing.
- **Samples read by hand.**
  - 4 of 4 logical-notice pairs are correct: same buyer, same or near title, DÖE 1–3 days before TED. The pairs were
    DRK Biberach "Innentüren"; Don Bosco-Schule Stappenbach; Wismut GmbH; Klinikum Ludwigshafen PET/CT.
  - 2 of 3 opp-090 pairs are correct Polish CAN→CN pairs (Województwo Mazowieckie; RCKiK Gdańsk).
  - **The third is a copied placeholder.** TED 00045334-2026 (Älvkarleby kommun) cites `00123456-2026`. That is a
    real, unrelated notice: Statistiska centralbyrån, tender 526284. It is refused only because the target is newer.
    Any later citer of `00123456-2026` passes ADR-0011's direction check, and same-Source previous-notice edges are
    unguarded. Since this deploy the daily's link closure pulls such targets in, so the weld can happen on a daily,
    not only on a full re-projection.
- **Decision (owner).** Hold the wet backfill until a buyer-overlap guard on previous-notice edges lands (unit 2b,
  workflow `wf_d8091709-da9`). The guard refuses an edge whose two notices name disjoint buyer token sets, comparing
  each buyer's identifier key and N2 name key, so the same buyer spelled with or without an identifier still overlaps.
  The backfill dry report gains `buyer_disjoint` and `would_split` with samples. The 5,050 logical-notice joins wait a
  day; they are guarded already, but the wet run also re-queues the opp-090 would-merges. Until 2b deploys, a daily may
  weld a new placeholder citer. Any such weld splits on the first fold after the guard.

## Unit 2b: the buyer guard on previous-notice references — LANDED 2026-10-02 (not yet deployed)

**Why.** Job 1882's sample: TED `00045334-2026` (Älvkarleby kommun) cites OPP-090 `00123456-2026`, a real,
unrelated notice (Statistiska centralbyrån, Tender 526284). That one was refused only because its target is newer.
Any later citer passed ADR-0011's direction check, and same-Source previous-notice edges were unguarded. Since
`7b14469` the daily's link closure pulls such targets in, so the weld can happen on a daily.

**What landed.**
- **The token set.** Each planned notice carries a tolerant buyer token set, `PlanRow::buyer_tokens` /
  `plan_notice.buyer_tokens`. For every buyer mention it holds BOTH the identifier key (`country:kind:value`, when
  the identifier passes the gate) AND the N2 name key (`n2:country:match_norm(name)`). It is built by `buyer_key`'s
  own machinery: `buyer_mentions` is shared, and `Ident::read` reads the mentions once for both. Each token is
  stored as a 4-byte FNV-1a digest, sorted, as a BLOB (NULL when no buyer was parsed).
  - Measured on a 200k-row synthetic `plan_notice` (≈1.9 tokens a notice: 5 % two buyers, 80 % with an
    identifier): **+9.6 B/row, about +137 MB on a full re-projection's 14.3M rows** (+7 % of the table). The same
    tokens as joined text cost +55 B/row (+789 MB).
  - A digest collision can only make two sets overlap, so the guard fails open, at about 1e-9 per compared pair.
- **The guard** (`Db::link_step`). A previous-notice edge is refused when both endpoints' sets are non-empty and
  disjoint, counted as `LinkTally::buyer_disjoint` (job row: `refused: … buyer-disjoint N …`). It applies to
  same-Source and cross-Source edges alike. It runs after the direction check (`not-earlier` keeps its meaning) and
  before the fan-in count, so a refused copier is not a second procedure there. Unknown is not disjoint.
- **Notice against notice, not against the cited component.** The reference names one notice, so the citer's
  buyers are checked against that notice's buyers. A component's buyer set only grows as it welds: Tender 1012301
  below holds 60 buyers, and a component check would let it vouch for nearly any citer, which makes the guard
  weakest exactly on a hub. The two-notice verdict depends on nothing else in the plan, so it never waits
  (`deferred` is untouched) and is equal on the incremental and full paths. A joint procurement's CAN naming one
  of its CN's buyers overlaps that CN itself, and stays joined.
- **The census** (`backfill-tender-links`). Each window is now resolved first (`Db::resolve_declared_window`).
  Both ends of every resolved previous-notice row are then read through the plan row's own derivation
  (`link_endpoints`: full parse → `Ident::read`), and the window is classified and written
  (`Db::backfill_declared_window`). The census counts, per rule (non-zero for `opp-090` only):
  - `buyer_disjoint`: would-merge rows the guard refuses (target strictly earlier). They are not re-queued, and
    they are sampled in `disjoint_samples` instead of the joins' `samples`.
  - `would_split`: rows whose notices share a Tender today but are buyer-disjoint, with the target strictly
    earlier and different procedure keys. Both notices are re-queued on the wet run, so the next daily splits them.
    They are sampled in `split_samples`.
  
  There are up to 30 samples of each, by both publication ids, on the report (`buyer_disjoint_samples`,
  `would_split_samples`), the summary and the progress line. The progress line now says "would be re-queued" on a
  dry run.
- **Stack budget** (issue 467). The guard pushed `run_project`'s poll frame past its 472 KiB budget again (the
  gate aborted on the gauge). Unit 2's follow-up is done: the link closure and the three `incremental_full_fallback`
  sites in `project_incremental_chunked_observed` are boxed, and the frame now needs between 420 and 452 KiB
  (bisected 2026-10-02), so there is at least 20 KiB of headroom.
- **Tests** (each new one checked to fail with the guard disabled; the overlap ones also fail with
  identifier-OR-name tokens):
  - `a_placeholder_previous_notice_reference_to_another_buyers_notice_is_refused`: the sample's shape. A later
    same-Source citer and a DÖE stranger are refused; the earlier citer is `not-earlier`; SCB's own DÖE procedure,
    naming SCB by name only, joins with `fan-in 0`.
  - `overlapping_or_unknown_buyers_keep_a_previous_notice_reference_joined`: identifier-vs-name, a joint
    procurement, and no buyers on either side.
  - The parity test, `the_ledger_joins_incrementally_exactly_as_a_full_fold_in_either_order`, now carries a copier
    and an SCB citer in both arrival orders.
  - `a_pre_guard_buyer_disjoint_weld_is_counted_and_split_by_the_next_daily`: dry `would_split` 1 with its
    sample, wet re-queues both, then the daily split. Equal to a full fold, no `removed`, the absorbed key's
    `tender_key_merges` row gone, a shared-BT-04 pair neither counted nor split.
  - The backfill test now covers `buyer_disjoint` with its sample, not re-queued.
  - Unit test `the_buyer_tokens_carry_both_keys_so_one_buyer_spelled_two_ways_overlaps`.

**What the 2026-08-20 full re-projection already welded** (bounded `/v1/sql` reads, 2026-10-02 10:0x UTC, 34
requests, 0 errors). Placeholder numbers looked up by publication id, their Tender by `tender_versions_notice`,
versions and buyers by the Tender's PK:
- **Tender 1012301: 184 versions (2024-02-28 … 2026-09-28) from 60 distinct buyer organizations.** It holds
  `00123456-2024` (seq 1), `00012345-2025` (seq 8) and `00123456-2025` (seq 12).
  - Member `00484510-2024` (a CAN) cites OPP-090 `00123456-2024`.
  - Member `00012345-2025` itself cites a real `729455-2023`, so whole procedures hang off each placeholder.
- **Tender 526284 (SCB's): 8 versions.** Seqs 1–2 are SCB (organization 4335: CN `00640006-2025`, CAN
  `00123456-2026`). Seqs 3–8 (2026-03 … 2026-08) come from four other buyers.
  - Organizations 3354129 and 3350848 sit in both hubs: repeat copiers.
- **Clean:** `00012345-2026` (1 buyer), `00000001-2025` (1 version), `00111111-2025` (17 versions, 1 buyer),
  `00654321-2025` (1 version).
- **Unread:** `00000001-2026` sits in Tender 455235 with 10 versions and 3 buyers.
- **Not held:** `00123456-2023`, `00012345-2024`, `00000001-2024`.
- **So `would_split` is at least ~63 from these two hubs alone, and probably 100–200.** Joining 60 and 5
  buyer-disjoint groups takes at least 59 + 4 cross-buyer edges, and the 184 + 6 welded versions bound the count
  from above unless a shared BT-04 holds a member. Other placeholder numbers were not sampled. The next daily after
  the wet run decomposes Tender 1012301 into its procedures: one Tender keeps the id, the rest are minted, and
  nothing is retired.

**Open.**
- **False refusals.** A buyer respelled between notices with no identifier in common, or a CAN filed by a central
  purchasing body for a CN filed by the authority (or the reverse), reads as disjoint. Count these in the dry run's
  samples before the wet run. The guard runs on every daily once deployed whatever the wet run does.
- **Census cost.** One full parse per endpoint of a resolved `opp-090` row: ≤ ~360k notices against job 1882's
  181,222 resolved rows, so expect the dry run well above 585 s. A raw-BT-04 pre-filter (pairs under one key never
  split) would halve it if it matters.

**Review fixes (2026-10-02, adversarial panel on `7d4cc3b`; each new test checked to fail with its fix reverted).**
- *False refusals of one buyer published two ways* (major + three minors, all fixed). The first token set forgave
  casing and punctuation only. Prod (org 9442, AP-HP) publishes five SIRETs of SIREN 267500452 under 40 name
  spellings, so two of its notices could share no raw identifier and no raw N2 name. That refused the reference, and
  `would_split` would have split the 2026-08-20 projection's correct Tenders on the next daily. The guard's tokens
  (`buyer_guard_tokens`; the org layer's `buyer_key` is unchanged) now take every spelling one buyer is measured to
  publish. A wider token only fails open, and only for one buyer written two ways:
  - the identifier as its E1 cross-walk key (`x:FR:siren:…` for every SIRET of one SIREN, `x:PL:nip:…` for a bare
    or `PL…` NIP, `x:SE:orgnr:…` for an organisationsnummer and its `SE…01` VAT), else as published. Equal raw
    identifiers give equal E1 keys, so nothing is lost. E2 (pad) keys are excluded: the CZ Justice/Assay pad
    collision stays disjoint, and so do SCB and Älvkarleby;
  - every name the mention published, i.e. each labelled BT-500 language variant, not only the first-seen head.
    This also makes the set independent of language order, the implementer's open parity risk;
  - Latin diacritics folded (`store::buyer_name_fold`, the R2 name gate's `fold_latin` table), so `HOPITAUX` meets
    `hôpitaux`;
  - the name scoped by `register_jurisdiction(country)`, so `RE` meets `FR` as the identifier already did.
  - Token count per buyer is unchanged except for variants (4 bytes per extra language).
  - Tests: unit `one_buyer_spelled_as_it_is_published_overlaps_and_two_buyers_do_not` (seven must-overlap shapes,
    two must-stay-apart shapes, the org layer's N2 key still split on accents); four more pairs in
    `overlapping_or_unknown_buyers_keep_a_previous_notice_reference_joined` (SIRETs, NIP, accents, RE/FR); an AP-HP
    pair in the incremental-equals-full parity test, both arrival orders.
- *Not-earlier rows read as joins in the census* (minor, fixed). The census judged direction only inside its
  buyer verdict, so a resolved `opp-090` row whose target is not strictly earlier counted in `would_merge`, was
  re-queued and was offered to `samples` ("the joins"). The fold refuses those first. Job 1882's Älvkarleby →
  SCB pair is one, and it showed among the joins. Now it is judged in the fold's order: such a row is
  `not_earlier` (per rule and in total, a subset of `would_merge` like `buyer_disjoint`). It is neither re-queued
  nor sampled. The backfill test gained that early copier.
- *A pre-guard plan counted as resumable* (minor, fixed). A complete plan from `7b14469` has `plan_link_edge` but
  no `plan_notice.buyer_tokens`. A rebuild salvaged on this binary would skip Phase-1 and fail in the link step
  over an already-reset tender layer, on every retry. `plan_is_complete` now refuses a plan whose `plan_notice`
  lacks the column, so the rebuild plans again. Test `a_plan_from_before_the_buyer_guard_is_rebuilt_not_resumed`.
- *The census read DE-1.x endpoints unfolded* (minor, fixed; latent, since no DE-1.x notice is an `opp-090`
  endpoint today). `link_endpoints` now runs `normalise_de1` on each batch before `Ident::read`, as all three plan
  paths and the walk do. No test: no DE-1.x notice can reach it yet.
- *Prod read* (1 bounded `/v1/sql` seek on `organization_mentions_org`): AP-HP's mentions carry SIRETs …00623,
  …01928, …00672, …01746, …00011, …00201, …00565 under names from "Hôpital Bicêtre, service achat" to "ACHAT".
- *Still refused, by design*: a buyer that renamed itself, or respelled beyond case, punctuation and accents, with
  no identifier in common; a CAN filed by another body than the CN's buyer. The reviewer found the CPB-as-service-
  provider shape outside both sets (eForms `OPT-300-Procedure-SProvider`), and 0 of 13 sampled daily citers
  refused.


## 2026-10-02 12:5x–13:4x UTC — 2b deployed (a5c7666); the census shows the guard is too strict; wet still held (unit 2c)

- Gate on `a5c7666`: GATE-EXIT=0, 145 suites, 890 s, marker written. Deployed with 477 unit 1b at 12:5x UTC.
- Census dry, job 1893, 1,468 s: opp-090 **would_split 5,359** and **buyer_disjoint 26**. Also 164 not-earlier, now
  counted apart from the joins. logical-notice: 0 refused, 0 split.
- **The 30 would-split samples, read by hand** (organization ids and first buyer names per notice; table in
  `.scratch/tender-db/481-dedup/census-2026-10-02/ws-class.txt`):
  - About 16 are real false merges, mostly across countries. BG school → LV hospital; BG power plant → FR parking
    operator; BG district → SE (00123456-2024); CZ ČD Cargo → ES municipality; and more. Bulgarian national register
    numbers collide with TED numbers, and the 2026-08-20 full projection joined them.
  - About 12 are **legitimate pairs the guard refuses**:
    - 4 share the resolved organization id. Santaros klinikos: org 3988 on both, the same raw identifier
      124364561, but the scheme is NULL on one and `002` on the other, and one name ends in "(PV)".
    - Agencies buying on a buyer's behalf: "Onderwijs Inkoop Groep B.V. namens <school>" and "DASmakkelijk B.V.
      namens AT Scholen".
    - Sister bodies and name variants: KIS Potsdam / KIS, ARPAS, Instytut Biologii Doświadczalnej, and DB InfraGO
      against DB AG Konzernleitung.
  - That is a ~40 % false-refusal rate on existing merges, so the guard as deployed would split correct merges.
- **Decision (owner).**
  - The wet run stays held.
  - Unit 2c (workflow `wf_47a7e711-29f`) adds the missing overlap evidence, re-verdicts all 56 samples, and pins
    them as fixtures. The evidence: a shared resolved organization id, the same raw identifier whatever its scheme,
    agency-on-behalf names, and whole-word name prefixes. It also decides whether disjoint jurisdictions are an extra
    refusal reason.
  - Until 2c deploys, the live guard applies only to Tenders a daily happens to plan. Any wrong split is re-joined by
    the first fold after 2c. 2c must land before the 2026-10-03 07:35 UTC tick.

## Unit 2c: the buyer guard made precise — LANDED 2026-10-02 (not yet deployed)

**Why.** Job 1893's census: `would_split` 5,359 and `buyer_disjoint` 26, and about 12 of the 30 split samples were
one buyer named two ways, so the 2b guard would split correct merges on the next fold that plans them.

**Re-verdict of all 56 samples.** Buyer-side mentions by `OPT-300` role, resolved organization, raw identifier and
scheme, and titles came from 10 bounded `/v1/sql` reads (notices by `(source, publication_id)`, mentions by
`notice_id`, `notice_texts` by `notice_id`; 0 errors). Each notice was rebuilt as a fixture and run through the 2b and
2c token derivations (`buyer_tokens_of` + `add_buyer_org_tokens`). The harness was a temporary `#[ignore]` test,
removed before commit.

| | legit (one procedure) | false merge / hub | 2b overlaps | 2c overlaps |
|---|---|---|---|---|
| `would_split` (30) | 9 (+ ws4 probable) | 20 | 0 | 8: ws3, 10, 14, 18, 20, 21, 25, 30 |
| `buyer_disjoint` (26) | 9 | 17 | 0 | 6: bd4, 14, 16, 19, 24, 25 |

- **Legit pairs that now overlap (14)**, and the rule that joins each:
  - ws3 (Santaros klinikos): the signatory, raw identifier and resolved organization. The award names the hospital as
    buyer and the ministry as signatory, and the CN names the ministry as buyer.
  - ws14, ws18, ws30: an agency's principal (`Onderwijs Inkoop Groep B.V. namens …`, `DASmakkelijk B.V. namens …`).
  - ws10 (KIS Potsdam), ws21 (Nencki), ws25 (ARPAS), bd16 (MINARM/TERRE/SIMMT), bd19 (Salerno CUC), bd24 (ZDW
    Lublin): a whole-word prefix.
  - ws20, bd25 (two units of the Andalusian health service), bd4 (a SERGAS area): a head.
  - bd14 (`ICS - Gerència de compres` / `Institut Català de la Salut`): an acronym.
- **Legit pairs still refused (4, + ws4)**:
  - ws6, bd6: a SERGAS area's call-off citing the SERGAS framework, which is written in Galician (`Servizo Galego
    de Saúde`).
  - bd1: a SAS hospital's call-off under the Junta's postal framework (the framework's buyer is the Junta's DG
    Contratación).
  - bd13: `Instytut Biologii Doświadczalnej im. M.Nenckiego PAN`, abbreviated mid-name.
  - ws4: probable. Bremerhaven's e-car framework award cites a CN that names `Umweltbetrieb Bremen` under the review
    chamber's identifier.
  
  None of rules 1–4 reaches these. The call-offs need another kind of evidence, for example a shared winning
  tenderer (Janssen in ws6, Correos in bd1). That is not decided here.
- **False merges, all still refused (20 + 17)**:
  - The cross-country placeholder collisions: ws1, 2, 5, 8, 9, 11, 12, 13, 15, 16, 19, 24, 26, 28, 29, bd3, 5, 7, 12,
    and bd22, which is `00123456-2024` again.
  - Same-country different procedures: ws7, cited `00000129-2025`. The owner's "same organization" was the
    Publications Office eSender (org 12) and not a buyer. Also ws23, a municipality against DB Netz.
  - **Hubs**, a PIN or qualification system cited by many procedures:
    - The DB group's deadline-shortening PINs `00558776-2025` and `00558909-2025`: ws17, 22, 27, bd8–11, 17, 21, 23,
      26. Read as "sister bodies" in the owner's list, but the PIN's Tender holds 147 versions.
    - The ÖBB-Holding qualification system: bd2, bd15.
    - Achilles' Repro supplier classification: bd18, bd20.

**What landed.**
- **Rule 1, one resolved organization.** Each buyer-side mention's Organization is a FULL token `o:<id>`. The plan
  paths resolve the chunk's mentions BEFORE writing its plan rows. The full fold and the incremental delta use the
  resolver's answer; the incremental closure and the census's `link_endpoints` read `organization_mentions`, which a
  merge repoints. So all three see the merged id.
- **Rule 2, one raw identifier, whatever its scheme.** The token is `r:<jurisdiction>:<ALNUM upper>`, emitted whether
  the gate kept the identifier or not. The `idgate` placeholder classes emit nothing: lexicon, digit runs, phones,
  routing scopes, TED notice numbers, bare short numbers, all-zero and one-character values.
- **The contract signatory** (`OPT-300-Contract-Signatory`) is on the guard's buyer side, but only on a notice that
  names a buyer. Issue 369's buyer key is unchanged.
- **Rule 3, agency on behalf.** `namens | im Auftrag von/der/des | on behalf of | pour le compte de/du/des/d' | en
  nombre de/del | per conto di/del/della/dei | w imieniu`. The principal's folded name is a FULL token. The agent is
  only a PREFIX of the whole name: it meets the agency's own notices, never the agency's notice for another
  principal. Templates are reused, which is where a copied OPP-090 is likeliest.
- **Rule 4, prefix.** A name's whole-word prefixes are PREFIX tokens, which meet only a FULL token (another notice's
  whole name or head), never another prefix.
  - `Gemeente Utrecht` / `Gemeente Amersfoort`, `Commune de Lyon` / `Commune de Nice`, `Uniwersytecki Szpital
    Kliniczny w …` and `Stadt Köln` / `Stadt Bonn - …` stay apart.
  - A one-word prefix needs ≥ 5 letters (`ARPAS` yes, `DB` no), and a prefix ending in a function word is skipped.
  - The **head** (the text before the first separator) is FULL when it has ≥ 3 content words.
- **Acronym.** A one-word name or head of 3–8 letters is an ACRONYM token. A name of ≥ 3 content words gives its
  INITIALS. These two meet each other only.
- **Token kinds.** The two low bits of the 4-byte digest are the kind: FULL, PREFIX, ACRONYM, INITIALS
  (`store::buyer_tokens_disjoint`). The digest is 30 bits; a collision still only fails open.
- **Plan column.** The column is renamed `plan_notice.buyer_guard`, so neither a 2b plan (kindless tokens) nor a
  pre-guard plan is resumable.
- **Jurisdictions: no extra refusal.** Every sampled cross-country false merge is already refused, because the tokens
  are register-scoped. A jurisdiction rule could only refuse where one resolved Organization spans two countries,
  which rule 1 calls overlap.
- **Generic-prefix measurement.** Every Organization mention of the 56 samples' notices, in any role, was taken as a
  one-buyer notice: 297 parties, 43,927 cross-organization pairs.
  - 67 overlap. 66 are one body under two org ids (AP-HP-like respellings, `S.A.`/`SA`, KIO and `Krajowa Izba
    Odwoławcza`, Brandenburg's renamed review chamber, the units of DB InfraGO, SAS and SERGAS).
  - 1 is two bodies: `TRIBUNALE AMMINISTRATIVO REGIONALE - TAR SARDEGNA` meets `Tribunale Amministrativo Regionale
    Campania - Salerno`, because the generic 3-word head meets the other's prefix. Both are review bodies, never
    buyers.
  - Before the head rule counted content words, a second false overlap showed: two Bulgarian water utilities
    (`Водоснабдяване и канализация - Варна`). `и` and Italian `dell'` are now function words.
- **Tests:**
  - The unit test `the_census_samples_one_buyer_overlaps_and_the_false_merges_stay_apart` holds 15 legit fixtures
    named after their samples, each asserted 2b-refused and 2c-overlapping, and 16 apart fixtures.
  - The other unit tests cover rule 2's scheme pair, the signatory only with a buyer, rule 1, and the generic shapes.
  - `one_resolved_organization_keeps_a_previous_notice_reference_joined`: refused, then after a merge-style mention
    repoint it is joined by the census, the daily and a full fold.
  - The placeholder test adds the DB PIN and one agency for two schools, both refused.
  - The overlap test adds ws14, ws10, ws25, ws21 and ws20.
  - The parity test adds ws18 in both arrival orders.
  - The split test adds KIS (ws10): joined, not counted, not split.
  - The resume test adds a 2b plan.

**Open.**
- The 5 remaining legit refusals above.
- The TAR-style generic 3-word head.
- Hubs (group PINs, qualification systems) are refused here only because their buyer differs. A DB InfraGO PIN
  cited by DB InfraGO procedures would still weld them; same-Source previous-notice references have no fan-in guard.

## Unit 2c review: generic kinds of body, agency names, country-less buyers — LANDED 2026-10-02 (not yet deployed)

An adversarial review of 4e0166e read bounded `organizations.name_norm` range seeks on prod (scratchpad
`rev2c/*.json`) and found the 2c widening joining whole classes of separate bodies. Verified against the code and the
saved reads; dispositions:

- **Fixed (major): a head shared by a kind of body.** `CENTRE HOSPITALIER UNIVERSITAIRE (CHU) DE BORDEAUX / … DE
  CAEN`, `AZIENDA SANITARIA LOCALE (ASL) NAPOLI 1 / - VITERBO`, `(ARPA)`, `(CCAS)`, `(OPH)`, the CUCs and SUAs all cut
  to a 3-word type name, which was a FULL head shared by every body of the type. Now (a) a parenthesis the name
  goes on after is a gloss, dropped, and the head read on (`centre hospitalier universitaire de bordeaux`); (b) a
  `/` after a one-letter word (`c/o`) is no separator; (c) a measured stoplist of kinds of body
  (`GENERIC_BODY_NAMES`, also matched with the kind's initials appended, `… chu`) gives no head, principal or
  prefix token. The Camaiore-for-Altopascio / for-Camaiore CUC pair is now apart.
- **Fixed (major): a bare generic name met by every sibling's prefix.** Prefixes equal to a kind of body are
  skipped (`Zarząd Dróg Wojewódzkich` of `… w Krakowie`), and a one-word prefix now stands only as the name's
  one-word head (`ARPAS` of `ARPAS - …`, `MINARM` of `MINARM/…`; not `Stadt` of `Stadt Köln`, `Mairie` of `Mairie
  de Pau`). The bare whole name keeps its 2b FULL token, which only another bare name meets.
- **Fixed (major, partly): agency phrases.** Added `namen` (typo), `tbv` / `t.b.v.` / `ten behoeve van`, `in opdracht
  van`, `iov` / `I.O.V.`, `en representación de/del`, `por cuenta de/del`, `per conto dell/delle/degli` and bare
  `per conto`, `ente delegato dal/dalla`, `reprezentująca`, `på vegne af/av`, `på uppdrag av`. **Deferred:** the
  principal-before forms (`vertreten durch`, `mandataire`, `vertegenwoordigd door`): the one sampled is the
  ÖBB-Holding qualification system (bd15), a hub the guard refuses only because the buyer differs, so reading its
  principal would weaken a hub refusal before a fan-in guard exists. **Refuted:** `na rzecz` (PL) — mostly the name
  of a foundation or association (`Fundacja na rzecz …`), not an agency; `c/o` — the host municipality, not the
  principal.
- **Fixed (minor): agent prefixes.** An agency name (`<agent> namens <principal>`) emits no prefixes of its own
  name: `Onderwijs Inkoop Groep` and `DASmakkelijk` are published bare beside 80+ / 40+ `… namens <school>` orgs.
  The 2c fixture "the agency's own notice" (asserted overlapping) is now an apart fixture.
- **Fixed (minor, partly): VAT prefix in rule 2.** A leading VAT prefix of the buyer's own register (`ES`, `EL` for
  `GR`) is dropped (`ESQ2769003A` meets `Q2769003A`). **Deferred:** register labels (`FN`) and the hub fan-in limit:
  normalising `FN71396w` would weld the ÖBB-Holding hub (bd15), whose refusal stays accidental until same-Source
  previous-notice references get a fan-in guard.
- **Deferred (minor): the CPB-files-the-CAN shape** (legacy `purchasing-body`, eForms serv-prov). Measure first
  whether legacy `PURCHASING_ON_BEHALF_YES` sections name the principals.
- **Fixed (minor): country-less mentions.** A mention with no country gets the 2b tokens only (whole names,
  identifier key): no raw-identifier, head, prefix, principal or acronym tokens. Legacy rows carry `mairie`, CHU,
  SPZOZ country-less by the hundred. Falling back to the notice's own country is deferred (needs plumbing).

**Tests.** `the_census_samples_one_buyer_overlaps_and_the_false_merges_stay_apart`: all 14 sample-legit fixtures
still overlap; 5 more one-buyer spellings (agency `namen` and `I.O.V.`, `per conto dell'`, a glossed CHU against
its plain name, an `ES`-prefixed NIF); 14 more apart fixtures (CHUs glossed and dashed, a CHU against the bare kind,
ASLs, ARPAs, CCAS, the CUC for two principals, two CUCs `c/o`, two SUAs, a ZDW against the bare kind, `Stadt Köln`
against `Stadt`, the agency against its own and its bare name). Mutation: the stoplist switched off fails the CHU
fixture. `a_mention_without_a_country_gets_no_widened_tokens`. The 2c sample verdicts are unchanged (no sample
legit pair relied on a removed token; the synthetic agency-own-notice fixture was the only one).

**Still open.** A kind of body not on the stoplist and written `<type> - <place>` still shares its head; the list
is measured from the review's reads, not exhaustive. The plan column is not renamed again: 4e0166e was never
deployed, so no 2c plan exists on prod.


## 2026-10-02 17:xx UTC — 2c deployed (`696dd9d`, with 470 unit 2); census re-read; WET run

- Gate on `696dd9d`: GATE-EXIT=0, 145 suites, 884 s, marker written. Deployed on an idle queue.
- Census dry, job 1903, 1,618 s: opp-090 **would_split 4,564** (2b's census: 5,359), buyer_disjoint 20, not-earlier
  164. logical-notice is unchanged: 5,050 would merge.
- **30 fresh would-split samples, read by hand** (`481-dedup/census-2026-10-02/ws-class-2c.txt`):
  - About 25 are false merges. They are mostly Bulgarian national numbers colliding with TED numbers across
    countries. DB InfraGO procedures cite the DB group's deadline-shortening notices, and splitting those is right.
  - Stadt Erlangen is the same buyer but two different procedures (Trafostation vs a school build), so splitting it
    is right.
  - **One false refusal remains.** Stadt Bochum's façade renovation was published on DÖE by "Stadt Bochum - Zentrale
    Dienste" and on TED by "Stadt Bochum, Referat Zentraler Einkauf". The two have no shared identifier or
    organization, and 2c's review removed kind-of-body prefixes ("Stadt X"). Kaunas (city vs the architects' union)
    is unclear.
  - The false-refusal rate is ~3–7 % (was ~40 %), so roughly 150–300 correct merges split against ~4,300 wrong
    ones fixed.
- **Decision (owner): wet now.** Job 1905. It writes the ledger and re-queues ~17.5k notices, so the 2026-10-03
  07:35 UTC daily splits the placeholder welds (hub 1012301, 526284, …) and joins the 5,050 BT-701 twins.
  - Follow-up: one buyer published by two departments of one city with no shared identifier. A whole-word place token
    after a kind-of-body head, e.g. "stadt bochum", needs measuring against the generic wall first.
  - NEXT: read the daily's `issue-481` line (largest component, refusals, deferred 0) and the Verify of 1110706
    (issue 482), then the data-quality "merged with TED" count.
- 17:5x UTC: wet 1905 finished; it attests `tender_links_complete` and re-queued 17,537 notices for the 2026-10-03 daily.

## 2026-10-03 04:0x UTC — the wet backfill's re-queue was folded by project 1915 (not the daily)

- Project 1915 (the FTS chunk's own fold) picked up the 17.5k re-queued notices: 18,136 notices → 11,568 Tenders in
  423 s. Its `issue-481` line:
  - **joined 10,410**: previous-notice 5,366, of which cross-source 1,531; logical-notice 5,044;
  - **refused 4,686**: buyer-disjoint 4,580, not-earlier 106; deferred 0;
  - largest component 226 keys.
- **Placeholder hub 1012301: 184 versions → 3** (Столична община, район „Триадица“).
- **Tender 1110706: 2 versions, buyer Stadt Wesel**, so issue 482's Verify reads done for its exhibit.
- 526284 now holds two notices sharing BT-04 `73206638-…`: a Tenerife EV-charging award (00640006-2025) and SCB's
  00123456-2026. That is a BT-04 collision, issue 482's shape, which the census will list.
- NEXT: inspect the 226-key component; the data-quality report's "merged with TED" after the Sunday report; then
  close unit 2 (unit 3 is the matched-link rule).
