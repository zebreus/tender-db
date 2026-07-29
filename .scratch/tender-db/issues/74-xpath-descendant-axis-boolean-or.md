# 74 — xpath grammar: descendant axis `//` + boolean `or` (unblocks SDK 1.0/1.3/1.5/1.6/1.7, ~390K)

Status: open (design decided below; ready to implement)
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
