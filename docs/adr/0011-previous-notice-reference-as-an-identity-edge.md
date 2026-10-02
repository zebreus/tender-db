# A publisher-declared previous-notice reference joins two notices into one Tender

**Status:** accepted 2026-08-19 (project owner, under the standing mandate). Implemented in the
grouping (`Db::build_plan_groups`), running on full re-projections since 2026-08-20 (issue 256). Amended
2026-10-02 (issue 481, below): the edge is a row of the Tender-link ledger, resolves in its own Source,
and applies on the incremental fold too.

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
   edge targets so identity is stable as backfill deepens. Here an absent target names nothing.)

   **Amended 2026-09-10 (issue 364): over there, a phantom may LINK but may not NAME.** The allowance
   stands — an absent endpoint still joins the component, and identity still survives a deepening
   backfill. What changed is the representative: the closure is union-to-min, so the component's label
   used to be its minimum OJS number over *all* nodes, phantoms included, and one mistyped digit in one
   citation could permanently give a Tender an identity no notice in it ever published. The label is now
   the minimum over the nodes that EXIST. A phantom that is later ingested and turns out to be the
   earliest is an ADR-0003 absorption, which the pipeline already handles. Membership is untouched: this
   decides what a component is called, never who is in it.
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

## Amendment (2026-10-02, issue 481): a ledger row, resolved in its own Source

- **The edge is a row of `tender_links`**, the one link ledger ADR-0003's 2026-10-02 amendment calls for.
  A row is keyed by notice: the citing notice, the Source and identifier it names (`b_source`, `b_ref`),
  and the target notice once one is held (`b_notice_id`, NULL until then). Its kind is `declared` and its
  rule `opp-090`. The plan build writes it for the notice that publishes the reference, and replaces that
  notice's declared rows every time it is planned. The grouping unions every resolved row whose two
  notices are both in the plan. Two other kinds of row share the ledger and the one union-find: matched
  links (kind `matched`), and TED↔DÖE same-notice links (rule `logical-notice`: a TED eForms notice's
  `BT-701-notice` is the id DÖE publishes the same notice under, as `<id>-<version>`).
- **Guard 1 now reads "the reference's own Source", not "the same Source".** The reference is a TED
  publication number whoever cites it, since the normaliser admits only `NNNNNNNN-YYYY`. So it resolves
  among TED notices. Requiring the citing notice's Source looked every DÖE citation of a TED
  predecessor up among DÖE notices, and dropped it. Guards 2–4 stand; guard 2's target must be a
  PARSED notice, as a full plan holds them. A refused not-earlier reference is now counted on the job
  row (`not-earlier`), where before it was a silent filter in the join.
- **A reference from another Source is fan-in-guarded.** The 563 of 563 above were TED citing TED; DÖE
  citing TED was never measured. When the cross-Source references into one target come from two or
  more procedure-keyed components, they are a PIN several procedures cite or a colliding key (issue
  482), not one procedure, and all of them are refused (`fan-in`). One DÖE procedure citing its TED
  predecessor joins it as a TED notice would. Admitted ones are counted apart (`cross-source`).
- **A keyed member names the component**, then a TED island, and only then the earliest publication.
  Two keyed components are named exactly as before: a keyed key's publication is taken from the
  previous-notice links it carries only, this ADR's input, so a same-notice or matched link cannot
  rename a merged Tender. The change stops a DÖE island published before its TED twin from naming the
  merged Tender, which on a non-rebuild run left the issue-278 ghost. One consequence: a component this
  ADR's rank named after an earlier island member is renamed after its keyed member the first time a
  fold plans it (the island-named Tender retired with a `removed` event) — a rename, not a loss.
- **The new rules are weld-guarded; this edge is not.** A same-notice or matched link may not put two
  keyed components into one Tender. One logical id must name one notice on the citing side, and one
  notice may be matched to notices of one component only (unit 3 writes one row per logical pair).
  And such a component may not join more than 64 components. Refused edges are counted on the job
  row. The previous-notice edge keeps this ADR's mechanism: it exists to join keyed components, and it
  has no cap; only its cross-Source fan-in is guarded. The largest component is on the job row too.
- **The incremental fold applies it too.** A link unions only when both notices are in the plan, so the
  incremental fold walks the ledger from every planned notice (`link_closure`: rows the notices state,
  rows naming them by id, unresolved rows naming them by publication id, and the changed notices' own
  links resolved from their parse) and plans each Tender it reaches whole, to a fixpoint, falling back to
  the full path past the legacy closure's cap. A link the plan still holds only one end of is deferred:
  its far end is re-queued for the next fold and counted (`deferred` on the job row), and a guarded
  join beside its near end waits for that fold too — a guard counting part of a component could admit
  what a full fold refuses. Until the ledger is attested complete (`tender_links_complete`: a full plan
  or a finished wet `backfill-tender-links`) every guarded join waits, since a notice planned before
  the ledger has no row for the closure to find it by. A procedure key a merge absorbed is recorded
  (`tender_key_merges`), so a later notice under it finds the merged Tender. A ledger row written or
  deleted outside a fold (`backfill-tender-links`, a matched row and its undo) re-queues both notices,
  so it reaches the next daily fold. Until then these edges applied on full re-projections only.
