# 481 — cross-source Tender dedup is exact-key only; how many duplicates it misses is unmeasured, and there is no source-agnostic edge for future portals

Status: ready-for-agent — UNIT 1 (CALIBRATION) DONE 2026-10-02 (workflow `wf_09fa7411-6db`; report `.scratch/tender-db/481-dedup/calibration-2026-10-02.md`). The TED↔DÖE misses are DECLARED links the fold does not follow, not fuzzy ones. TED `BT-701-notice` equals the DÖE notice UUID on 136 of 136 above-threshold DÖE islands in April 2025 (1.08 % of the month's merged count; ~2,650 extrapolated). Cross-source `OPP-090` links are dropped by a same-source condition. The best matched rule R1 measured 0 FP / 1,289 negatives at 98.0 % recall, but adds 0 joins beyond the declared link. NEXT: unit 2, the edge ledger with the `notice_uuid` and cross-source `OPP-090` producers and the fold reading it (incremental path included), plus the weld guards. The UUID-collision false merges it surfaced are issue 482.
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
