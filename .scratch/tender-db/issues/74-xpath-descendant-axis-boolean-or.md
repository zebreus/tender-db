# 74 — xpath grammar: descendant axis `//` + boolean `or` (unblocks SDK 1.0/1.3/1.5/1.6/1.7, ~390K)

Status: grammar implemented + green (team-lead reviews xpath.rs eval); vendoring 1.0/1.3/1.5/1.6/1.7 is the remaining step
Kind: completeness / data-quality
Blocked by: —
Relates to: 71 (parent), ADR-0002 (fields.json is the checklist), ADR-0004 (exhaustive-or-quarantine), CONTEXT.md (all eForms BTs representable, "no omissions")
Owner: (assign)

## Finding (issue 71 spike, 2026-07-29)

Issue 71 vendored the clean cohort (SDK 1.8/1.9/1.10/1.11, ~591K notices) with JSON only.
The remaining EU-SDK versions — **1.0, 1.3, 1.5, 1.6, 1.7** (~390K notices, incl. the single
biggest slice **1.7 = 344K**) — deserialize into `struct Sdk` fine and all field types map, but
`eforms::index::build` panics because **one field's** `xpathAbsolute` uses two constructs the
`eforms::xpath` grammar does not model: the **descendant axis `//`** and a top-level boolean
**`or`**. Verbatim panic (SDK 1.7):

```
eforms-sdk-1.7: unsupported xpath: //efac:TenderingParty/efac:Tenderer/cbc:ID/text()) or (cac:PartyIdentification/cbc:ID/text() = //efac:TenderingParty/efac:Subcontractor/cbc:ID/text())
```

The offending field is the same in every affected version — `efbc:CompanySizeCode` (**BT-165**,
the tenderer's company size), whose predicate joins an `efac:Organization`/`efac:Company` to a
`TenderingParty`'s `Tenderer` **or** `Subcontractor` by matching `cac:PartyIdentification/cbc:ID`
against a document-absolute `//efac:TenderingParty/.../cbc:ID`. Full xpath:

```
/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company[(cac:PartyIdentification/cbc:ID/text() = //efac:TenderingParty/efac:Tenderer/cbc:ID/text()) or (cac:PartyIdentification/cbc:ID/text() = //efac:TenderingParty/efac:Subcontractor/cbc:ID/text())]/efbc:CompanySizeCode
```

Confirmed empirically (curl + jq over `xpathAbsolute` for `//` and ` or `): exactly **one**
field in each of 1.0/1.3/1.5/1.6/1.7; **zero** in 1.8/1.9/1.10/1.11/1.12–1.15 (the construct was
dropped from the SDK at 1.8). So this single grammar gap is the whole blocker for ~390K notices.

The `eforms::xpath` grammar today (see the module doc + `Pred`): predicates are existence,
`@attr='v'`, `text()='v'`, value-set equality `text()=('a','b')`, and `not(...)` of those, over
relative paths with `..` steps. It models neither `//` (descendant-or-self anywhere in the doc)
nor a top-level `or` between two predicates.

## Options

**(a) Extend the grammar (RECOMMENDED).** Teach `eforms::xpath` two things:
- a top-level `Pred::Or(Box<Pred>, Box<Pred>)` (parse the `[(A) or (B)]` shape; `eval` = `||`),
- a leading-`//` path form (descendant search from the document root), i.e. a `Path` variant that
  starts at the root and selects by descendant axis rather than child steps from the context node.

  `CompanySizeCode` is real BT-165 data; CONTEXT.md requires all BTs representable with no
  omissions, so the correct end state captures the field, not skips it. This is the principled fix
  and unlocks all five versions at once. Scope is bounded — the two constructs appear in exactly
  one predicate shape, so the parser/eval additions are small and testable against that one xpath.

  Caveat to design for: the join predicate correlates *two subtrees* of the document (Organization
  ↔ TenderingParty). Evaluating `//` from the document root inside `Pred::eval` needs the node's
  document handle (roxmltree `Node::document`/root), which `eval` already has via the node. Verify
  the walker's per-element evaluation cost stays acceptable (a `//` scan per Company element) — the
  bounded-memory / flat-per-item principle still applies at 12.4M-notice scale.

**(b) Patch-table the one field (FALLBACK).** Route `efbc:CompanySizeCode` for these versions
through the existing `IGNORED`/`EXTRA` mechanism in `index.rs` — either an `EXTRA` entry that binds
the value at the predicate-free `efac:Company/efbc:CompanySizeCode` leaf, or an `IGNORED` claim if
capturing it is deemed disproportionate. Cheaper, no grammar change, but it either drops a real BT
(violates "no omissions") or hard-codes a per-version quirk the grammar should own. Use only if (a)
proves disproportionate for a single field.

**Recommendation: (a).** It's the ADR-0002/CONTEXT.md-aligned fix and the construct is contained.

## Design (proj-fix, 2026-07-29 — review before implementing)

### It is THREE constructs, not two

The offending predicate on the `efac:Company` step is:

```
[(cac:PartyIdentification/cbc:ID/text() = //efac:TenderingParty/efac:Tenderer/cbc:ID/text())
  or (cac:PartyIdentification/cbc:ID/text() = //efac:TenderingParty/efac:Subcontractor/cbc:ID/text())]
```

Beyond `//` and `or`, each side is a **node-set join**: `LHS/text() = RHS/text()`
where the RHS is *another path's* text, not a quoted literal. Today `Pred::TextEq`
only compares against literals (`text()=('a','b')`). So the grammar needs three
additions, all confined to this one predicate shape; everything else stays a strict
subset that still errors loudly at build time (not a silent mis-parse).

### (1) Grammar + evaluator

**Parser (`xpath.rs`):**

- **`Pred::Or(Box<Pred>, Box<Pred>)`.** In `parse_predicate`, before the `not(`/`=`
  handling, split the body on a top-level ` or ` (depth 0 w.r.t. `()`, `[]`, `''`),
  strip one layer of surrounding `()` from each side, recurse, fold left-assoc for
  >2 disjuncts. Requires teaching the splitter to also track `()` depth (today it
  tracks `[]` + quotes only) — add a `split_top_level_word(body, "or")`.
- **Descendant origin for `//`.** Replace `Path.up: usize` with an origin enum:
  ```
  enum Origin { Context { up: usize }, Descendant }   // Descendant = leading // from the document root
  struct Path { origin: Origin, steps: Vec<Step> }
  ```
  `parse_relative`: a path with a leading `//` → `Origin::Descendant`, strip `//`,
  parse the rest as steps (+ optional `text()`/`@attr` leaf); `..` still yields
  `Origin::Context { up }`. Migrate the two `Path { up: 0, .. }` sites in `index.rs`
  (the node `identifier` path, line ~609) and `parse_absolute`'s `up==0` assert.
- **`Pred::TextJoin { lhs: Path, rhs: Path }`.** In `parse_predicate`, after the
  top-level `=` split: if the RHS starts with `'` or `(` it is literal → existing
  `TextEq`/`AttrEq`; otherwise parse the RHS as a path ending in `text()` → `TextJoin`
  (parse the LHS the same way). Scope strictly to `text() = <path>/text()`; an
  `@attr` join is not in any SDK, so leave it an error.

**Evaluator (`xpath.rs`):**

- `Or(a, b)` → `a.eval(node) || b.eval(node)`.
- `Path::select` for `Origin::Descendant` → `node.document().root().descendants()
  .filter(|n| steps[0].matches(n))` then child-step the remainder. (roxmltree
  `Node::document()`, `Document::root()`, `Node::descendants()` all exist — verified.)
- `TextJoin { lhs, rhs }` → the trimmed non-empty text set of `lhs.select(node)`
  intersects that of `rhs.select(node)` (any-equal), matching `TextEq`'s trim.

### (2) Bounded memory / cost (the flat-per-item requirement)

Each notice is parsed as its **own** `roxmltree::Document` (per member in
`process.rs`), so `//` scans ONE notice's tree — never the corpus, never
cross-notice. The trie only consults this predicate when the walker reaches an
`efac:Company` element, so evals per notice = #`efac:Company` (orgs, ~1–50); each
eval does two `//efac:TenderingParty` descendant scans over the notice tree
(KB–low-MB). ⇒ per-notice cost `O(#Company × notice_nodes)`, **bounded and flat vs
corpus size**, no in-RAM O(corpus) structure — peak RAM unchanged at 12.4M scale.
Optional micro-opt (NOT for v1): both or-sides re-scan `//efac:TenderingParty`; a
per-notice memo could halve it, but the scan is cheap and adding state buys nothing
measured — flag only if profiling asks.

### (3) Join correctness + the ADR-0004 safety net (defuses the risk)

The general comparison `A/text() = B/text()` is true iff some LHS value equals some
RHS value: the org is selected iff its `PartyIdentification/cbc:ID` equals any
tenderer ID (side A) or any subcontractor ID (side B) — i.e. this Organization is a
tenderer/subcontractor — and its `efbc:CompanySizeCode` is BT-165. `//efac:TenderingParty`
(document-absolute) correctly spans every lot-tender's party; all prefixes
(efac/cbc/cac) are already in `namespace()`.

**Crucially, exhaustive consumption is NOT gated on this predicate.** The walker's
RELAXED CLAIM (`parse.rs:147–153`): when no branch matches an `efac:Company` by
predicate, it re-matches by NAME, still claims the element, and still binds the leaf
field — quarantining only on genuine field-id AMBIGUITY. `efbc:CompanySizeCode` maps
to exactly one field id (BT-165) at that leaf, with no competing id, so a Company
that fails the join still has its CompanySizeCode consumed via the relaxed path and
lands in the correct org section (the enclosing `efac:Organization`). Therefore:
build MUST parse the construct (the current panic is the whole bug); eval SHOULD be
correct for precise attribution, but a mis-eval degrades to "still consumed, correct
section", never a quarantine. This is what removes the ADR-0004 worry the Finding
raised. (The no-ambiguity assumption is validated by fixture + inspecting the built
index — see tests.)

### (5) Risks

- **R1 parser over-reach** — the wider grammar could mask a truly-unsupported xpath.
  Mitigation: each addition stays strict (only `text()=path/text()`, only leading
  `//`, `or` only between predicates); `rejects_paths_it_cannot_represent` guards.
- **R2 join mis-eval → wrong attribution.** Mitigation: relaxed-claim net + a fixture
  asserting exact attribution.
- **R3 ambiguous-field quarantine** if CompanySizeCode resolved to >1 field id under
  relaxed claim. Mitigation: fixture + assert a single field id at that leaf in the
  built index; if it ever bites, fall back to option (b) EXTRA predicate-free binding.
- **R4 `//` cost** — bounded per-notice (above); an optional timing assertion.

Scope: contained to `xpath.rs` (+ two tiny `index.rs` migration sites). No projection
changes. Recommendation stands: **option (a)**.

## Validation

- `every_sdk_xpath_folds_into_the_match_index` folds 1.6/1.7 (and 1.0/1.3/1.5) once vendored.
- A fixture notice of a `CompanySizeCode`-bearing 1.7 CAN parses the field to the right
  Organization section (the join actually resolves to the tenderer/subcontractor org).
- Existing xpath unit tests (`crates/ingest/src/eforms/xpath.rs`) stay green; add cases for the
  `[(A) or (B)]` and `//` shapes.
- No regression on the already-vendored 1.8–1.15 path.

## Rollout (after the grammar lands)

Vendor 1.0/1.3/1.5/1.6/1.7 the mechanical way (issue 71 clean-cohort pattern: fetch from
OP-TED/eForms-SDK tags, add to `ACCEPTED` + the pinned-versions test). 1.7 (344K) is the prize;
1.6=37K, 1.3=4.8K, 1.0=3.5K, 1.5=35. Then ops deploys + reprocesses the quarantined archive.
