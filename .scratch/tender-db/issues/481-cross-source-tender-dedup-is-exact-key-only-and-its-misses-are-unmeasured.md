# 481 — cross-source Tender dedup is exact-key only; how many duplicates it misses is unmeasured, and there is no source-agnostic edge for future portals

Status: ready-for-agent — filed 2026-10-02 from Lennart's question ("do we have proper general deduplication/merging,
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
3. **No duplicate signal.** ADR-0003 forbids heuristic merging, and that stays. Readers still have no way to see
   "probable duplicate, not merged", so the statistics the ADR worries about inflate silently wherever no link
   exists.

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
3. **Decide the duplicate signal** from unit 1's precision. If the measured matches are near-certain but undeclared,
   serve them as `possible_duplicate_of` on the Tender (never merged) and count them on the dashboard. If precision is
   low, record the no and keep counting.

## Verify

    ssh -o BatchMode=yes root@zebreus.click "/root/aj.sh /admin/reports/data-quality" | python3 -c 'import sys,json; b=json.load(sys.stdin)["body"]; i=b.find("merged with TED"); print(b[b.rfind("\n",0,i-60):i+40].strip())'

- **open** (2026-10-02): `DÖE procedure Tenders: 921,031; merged with TED: 243,588 (26.4%)`, with no count of
  missed twins anywhere.
- **done:** unit 1's measured miss count recorded here and, once units 2 and 3 land, on the weekly report beside the
  merged count.
