# A publisher-declared previous-notice reference joins two notices into one Tender

**Status:** accepted 2026-08-19 (project owner, under the standing mandate). Implementation pending —
this ADR decides the warrant and the guards, not the schedule.

EU eForms does not keep BT-04 stable across the notices of one procedure. Measured on prod
(issue 236): award-bearing Tenders whose first version is an `eforms:eforms-sdk-1.%` notice are
single-notice islands 27–39 % of the time, flat across 2024–2025, while eForms-DE — where DÖE keeps
BT-04 stable — chains at 98–100 %. The award and its contract notice are both in the corpus, both
eForms, and separate only because their BT-04 UUIDs differ.

`OPP-090-Procedure` (`ND-PreviousNoticeReference`) carries the prior publication's TED number. It is
parsed today and unused. **Decision: it is an identity edge — two notices joined by it belong to one
Tender.**

## Why this is not heuristic merging

ADR-0003 permits a merge only on "a strong explicit cross-reference" and forbids heuristics. It framed
that for the cross-*source* case (DÖE citing TED). This is the intra-source case: one TED notice
declaring which earlier TED publication continues its procedure. The warrant is the same in kind — the
publisher stated the link — so the same permission applies, and no inference is added: nothing here
matches on buyer, CPV, value or dates.

That line also fixes the scope. Of the unchained subtype-29 awards in a measured month (4,641), only
1,347 carry the reference and 563 resolve to a notice we hold. **The remaining ~71 % stay unlinked**,
and that is the decision, not a gap in it: without a published link, joining them would be inference of
exactly the kind ADR-0003, issue 234 (identifier-less organizations) and issue 237 (synthetic lots
groups) each refused.

## Guards, measured before being written down

All four hold on the 563 resolvable edges of the measured month:

1. **Same source only.** Resolve within `source = 'ted'`; the reference is a TED publication number and
   `notices` is unique on `(source, publication_id, content_hash)`.
2. **The target must exist.** A reference to a notice we do not hold creates nothing — no placeholder
   Tender, no pending edge. (Unlike the legacy OJS closure, which deliberately admits not-yet-ingested
   edge targets so identity is stable as backfill deepens: there the target's OJS number is itself the
   component key, so an absent target still names the component. Here it names nothing.)
3. **The target must be EARLIER.** 563 of 563 references point at an earlier publication; none pointed
   at the same instant or later. A forward or self reference is therefore not a shape the corpus has, and
   refusing it costs nothing while ruling out a cycle.
4. **A notice may carry several.** 1,340 notices carried one reference, 2 carried two, 1 carried three.
   All of a notice's references join the same component — a procedure republished in parts is still one
   procedure — so the edge set is a set, never a single value.

Normalisation is part of the guard: eForms writes `615938-2024` where `notices.publication_id` holds
`00615938-2024`. Zero-pad the number to 8 digits before lookup. A reference that does not parse into
`number-year` is ignored rather than guessed at.

## Mechanism: a merge edge between keyed components, not a change of key

Two ways to implement it, and the choice matters:

- **Rejected: fold eForms notices into the OJS node/edge space.** Their publication ids do parse as
  `(year, number)`, so `Ident::ojs_self` / `ojs_edges` would carry them with no new machinery. But the
  legacy component is identified by the MIN OJS key, so eForms Tender identity would silently stop being
  BT-04 — reissuing ids across the whole eForms corpus to fix a 12 % linkage gap.
- **Chosen: an edge between procedure keys.** The referring notice keeps its BT-04. The reference
  contributes an edge to the union-find over *components*, and when it joins two keyed components they
  merge exactly as a late legacy edge already does — the absorbed key's rows retired with `removed`
  events (ADR-0003-style merge, `project.rs`). BT-04 stays the key of everything it already keys.

## Consequences

- **A corpus-wide re-projection is required** for existing data; the fold only revisits what it touches.
  Bundle it with the other pending re-projections (issues 100, 232, 235) rather than paying the window
  twice.
- **Tender ids are retired on merge**, so the change log emits removals and any consumer holding an
  absorbed id must follow them (issue 46's feed-generation protocol already covers this path).
- **Two Tenders becoming one changes published counts.** The dashboard's Tender total drops by the number
  of merges — expected, and worth a ledger row when it lands so the drop is not read as data loss.
- **An incorrect merge is undoable** by re-projecting, since notices remain append-only per source
  (ADR-0003's own escape hatch).
- The mirror repair comes free: 373 of the 527 referenced Tenders in the measured month were themselves
  single-notice CN-only Tenders, so the same edge de-orphans both halves.

## What this ADR does not decide

- Whether the other ~71 % (no published reference) should ever be linked. Currently: no.
- Precedence when A references B and B carries a BT-04 that also matches C. The union-find makes all
  three one component; whether that is desirable needs the case to actually occur, and it has not been
  measured yet.
- Whether the same treatment applies to `BT-125` (previous planning notice) and the other
  previous-publication references. Same warrant on its face, unmeasured, and therefore out of scope
  until someone counts it.
