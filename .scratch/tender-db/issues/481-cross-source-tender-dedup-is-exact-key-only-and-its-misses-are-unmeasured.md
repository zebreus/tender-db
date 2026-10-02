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

## Unit 2 (next): the edge ledger and its declared producers

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
