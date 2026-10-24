# 551 W7 for 577 unit 2 (406529a): skeptic re-check

Run 2026-10-24. Read-only: 0 box statements, 0 GETs, no cargo. HEAD 76509b7. Since 406529a nothing has changed in
`crates/ingest/src/eforms/`, `project.rs`, `project/`, `tests/eforms.rs` or `sdk/` (checked with `git log 406529a..HEAD`).

**Verdict: the first analyst's conclusion stands.** The intersection is NOT empty (`intersection_empty: false`).

- I could not refute it. The hits are field ids the fold reads as W inputs, by literal and through `DE1_FIELD_ALIASES`.
  Each half of the rule's input produces hits on its own:
  - 577's own lists: `shadow.out` names `DE1-ProcurementProjectLot-ProcurementProject-RequestedTenderTotal-EstimatedOverallContractAmount`
    as a shadow loser, and `BT-27-Lot` / `BT-137-Lot` in its overlap section (lines 6869, 1121).
  - The code's moves: the three DE-1.x pairs below.
- I found **no W-tier id or stem they missed.** The details are in sections 1–4.

## 1. The literal scan, redone independently

`skeptic/indep.py`:
- Builds A from scratch: every id-shaped token in `shadow.out`, `shadow_extra.out`, `moves_bind.tsv` and `moves_extra.tsv`.
  That gives 631 ids, identical to `set_A.tsv` in both directions.
- Matches A against **every** string literal in the fold's production code at HEAD. That is `project.rs` before
  `mod tests`, plus the production parts of guid_twin, key_census, role_census, escapes and title_key_nfkd.
- Checks four forms of each id, the way `canonical_name` matches them: the id, `stem(id)`, the id after `DE1_FIELD_ALIASES`
  normalisation, and that id's stem. Literals ending in `-` or `(` are matched as prefixes.

Result:
- **141 ids hit. These are exactly the analyst's `intersection.tsv` ids.**
- The scan also listed 22 extra `UBL-*` ids. Every one of them is a key of `UBL_PARSE_ONLY` (project.rs:800). That table is
  `#[cfg(test)]` and records ids the fold deliberately does NOT read, so none of them is a read.

Non-literal reads, checked by hand:
- `withheld_source` (`BT-195(` prefix): the only BT-195 declaration in A whose source is a W stem is
  BT-195(BT-720)-Tender, which the analyst already lists. BT-553/160/162 are not in `AMOUNTS`, and the withheld set is
  consulted only for amount stems (5696), BT-720 (7334) and BT-759/760 (7395).
- `role_name` / `de1_mark_reference` (`OPT-300-` / `OPT-301-` prefixes): already in the analyst's W-ind tier.
- The amount-marker `format!` at 10255 builds TED ids only.
- The link backfill (2764) reads only the DE1 sources of OPP-090 and BT-701, and neither is in A.

The other DE-1.x moved ids nothing reads:
- `DE1-ProcurementProject-ID` → `DE1-ProcurementProjectLot-ProcurementProject-ID` (this is not `DE1-ProcurementProjectLot-ID`,
  the published lot number, which no move touches);
- `DE1-TenderingProcess-NoticeDocumentReference-ID`;
- `UBL-TerminatedIndicator`.

## 2. Does 406529a move only the listed ids?

**The de-1.x pairs, cross-checked.** 577's own `shadow.py` is earlier code, independent of the analyst's port. Its de-1.x
rows, `shadow.out` and `shadow_extra.out` together, give exactly the same **118 pairs** as `moves_bind.tsv`'s de-1.x rows,
with 0 differences either way. Under D577-1 (a), every patch-first, declared-later element on de-1.x binds the declared id,
so the two sets must coincide.

**SDK-shaped inventories.** The gated test pins the moved sets exactly ({BT-23, BT-531, BT-772} pairs), and 577 records the
gate green at 406529a.

**EXTRA.** The 4 pinned overwrites.

**Other code paths in the diff:**
- `counterpart()` (date/time pairing) now runs `bind` over the sibling's time-kind fields. Every time-kind pair is already
  a co-bound pair in the test, and no SDK pin includes one. The de-1.x date and time ids (deadline, opening) are nonW.
- The `project.rs` part of 406529a adds three TEXTS keys (two full DE1 lot ids, plus `BT-749`) and the BT-749
  SelectionCriteria filter. All of them are nonW texts.
- Facts carry canonical names, not source ids (`store::canonical::Fact`).

### A blind spot in both the test and the port (no new W hit)

`co_match` assumes every ancestor step matched exactly. `parse.rs` falls back to `select_by_name` (a "relaxed claim") when
no branch matches. For example, a `cac:ProcurementProjectLot` whose `cbc:ID` has a missing or unlisted `@schemeName` is
claimed as a container under all of [Lot], [LotsGroup] and [Part].
- Its descendants are then selected EXACTLY under all three subtrees.
- The ambiguity check runs only at the relaxed element itself.
- So a child can get a candidate list that spans subtrees `co_match` rules out (`[schemeName='Lot']` vs `'Part'`).

`can_relax` is true for the root lot on all 17 SDK-shaped inventories. It is false on de-1.x and sdk-0.1, because those
have a predicate-free lot branch.

I emulated this on the analyst's `build` port (`skeptic/lib577.py`):
- **`relaxed.py` (superset):** ancestors are allowed to match by name only, and the element itself must still co_match.
  - It finds 53 (old, new) pairs beyond the strict 125 (105 ids, `relaxed.out`).
  - None of these ids is W-tier: BT-13(d), BT-23, BT-262/263, BT-5071, BT-5141, BT-536/537, BT-766, plus 20 OPT-301-Lot/Part
    role refs, which are W-ind and already in the analyst's overlap-list tier. Ids in the pairs that no fold table reads
    (BT-115, BT-124, BT-125(i), BT-14, BT-15, BT-36, BT-707, BT-708, BT-71, BT-736, BT-737, …) are left out.
  - No BT-21, BT-27, BT-271, BT-137, BT-720, BT-142, BT-13713, OPT-300 or BT-195(BT-720) appears.
- **`relaxed4.py` (full ordered list):** it uses the full ordered candidate list. A pair is dropped when an earlier subtree
  holds a declared field that necessarily matches wherever the patch does.
  - 4 pairs survive (`r4.out`): BT-531-Lot→BT-531-Part (already in the strict set), and 3 new ones, all nonW:
    - BT-23-Procedure→BT-23-Part
    - BT-531-Procedure→BT-531-Part
    - BT-707-Lot→BT-707-Part
  - **The OPT-301 role pairs are all guarded,** because [Lot]'s declared OPT-301-Lot-* comes first.

This partly corrects the analyst's caveat 3. Its stated reason, "co_match excludes those", does not cover relaxed claims.
Its conclusion still holds, because no W-tier overlap id is a move the code can make.

## 3. Stem collisions

**BT-27-Lot vs BT-27-Procedure (and BT-271, BT-21):**
- Both share the AMOUNTS / TEXTS stem key, so the canonical name is the same.
- The only exact W read of an alias target is `stated_total_kind`'s `"BT-27-Procedure"` (8925).
- No SDK move, strict or relaxed, touches BT-27-*.
- The DE-1.x move turns a lot section's normalised BT-27-Procedure into BT-27-Lot. `notice_stated_totals` returns early for
  every `eforms:eforms-de-1.*` profile (8950), and those are exactly the profiles that map to the eforms-de-1.x index
  (sdk.rs:96).
- No other crate reads those ids exactly in production. canonical.rs:6216 is a label only.

**DE1 lot vs project ids.** `stem()` cuts them to `DE1-ProcurementProjectLot`, `DE1-ProcurementProject` and
`DE1-TenderingTerms`. No fold table holds any of these as a key, so `canonical_name`'s stem branch never fires for a DE1 id.
DE1 ids are read by full id only:
- `normalise_de1`;
- `PUBLISHED_LOT_NO_FIELDS` (`DE1-ProcurementProjectLot-ID`, exact);
- 577's two new full-id TEXTS keys;
- `CONTRACTING_SYSTEM_FIELDS`.

The analyst's 146 generous-stem matches are therefore noise, as they said.

## 4. The worktree (576 U2, uncommitted)

The `project.rs` diff is now about 800 lines. Before `mod tests`, the only new id literals are still
`BT-500/BT-514-Organization-TouchPoint`; the other new literals are all in tests.

It also adds `touchpoint_role` (`"Lot-"` / `"Part-"` role prefixes). That is a further non-literal read of the OPT-301
role refs, which are already W-ind. For the DE-1.x role pairs, both sides alias to the same OPT-301-Lot-* role.
It hits nothing new in A.

## 5. Caveats

- **No cargo.** Another agent's uncommitted 576 U2 edits sit in the fold, so the relaxed-claim emulation is Python only.
  It inherits the analyst's port, which reproduces every gated pin.
- **The relaxed model is approximate.** It allows one relaxed level, plus a name-only superset for any number of levels.
  The superset already shows no W-tier id, so a finer model can only remove pairs.
- **Re-running the scripts.** `skeptic/indep.py` reads HEAD copies taken in the session scratchpad:
  `git show HEAD:crates/ingest/src/project.rs` and the project/ modules. Point its two path constants at fresh copies to
  re-run it. `relaxed*.py` run from `skeptic/` with `python3 -I`.
- **Files written.** Only this file and `skeptic/`. No git operations. The issue-539 edit is untouched.
