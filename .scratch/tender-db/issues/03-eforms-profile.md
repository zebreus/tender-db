# 03 — eForms mapping profile + completeness harness

Status: resolved
Blocked by: 02

Goal: eForms notices parse into the notice-parsed layer with the
mapped-or-ignored guarantee enforced by tests — the ADR-0002/0004 machinery
exists and is real.

Scope:
- roxmltree-based parser core: namespace-URI matching, exhaustive
  consumption bookkeeping (every element/attribute must be claimed by a
  mapping or an ignore rule; leftovers ⇒ quarantine the notice).
- Notice-parsed tables for the eForms core: procedure fields, lots (+
  LotsGroups, Parts-as-lots-with-kind), organization mentions + roles,
  amounts (cents+currency), texts (EN + original), classifications (CPV,
  NUTS), dates (UTC + offset), withheld-field satellite, id-refs.
- Mapping registry as data: field id → target, or documented exclusion
  (pointless BTs, OPP plumbing). Completeness test walks the pinned SDK
  fields.json (vendor the fields.json of the pinned SDK version into the
  repo) and fails on unaccounted fields. Multi-version: registry keyed by
  (sdk-version range); wild CustomizationIDs outside known range ⇒
  quarantine.
- Fixture notices (real, committed): one per major notice type (CN, CAN,
  corrigendum/change, PIN, veat, BRIN) — parse tests assert extracted
  values.

Acceptance: the fetched day's eForms notices parse with zero unexplained
quarantines; completeness test green against the vendored fields.json;
`cargo test -p ingest` covers fixtures.

## Answer

Delivered: the eForms mapping profile (`crates/ingest/src/eforms/`), the
notice-parsed layer (`crates/store`), and the ADR-0002 completeness harness.
Verified on the real TED daily **2026-136**: **3722 notices, 3715 parsed, 0
unclaimed-content quarantines**, re-run wrote 0 rows.

### The claim system

Four pieces, deliberately reusable by the r209/r208/text profiles:

- `sdk` — the pinned SDK metadata, vendored verbatim per accepted minor
  (**1.12.0, 1.13.0, 1.14.0, 1.15.0** — the archive's whole eForms range; the
  daily is 1.12/1.13/1.14), plus the mapping registry. The registry is *data*
  driven by field **type + xpath context**, not 1256 hand-written arms: eight
  value targets (texts, codes, classifications, amounts, dates, integers,
  numbers, ids) and two documented exclusions (`attributeOf` plumbing — 480 of
  1256 fields — and the three `OPA-*` virtual views). Every field id resolves to
  exactly one decision, which is what the completeness test asserts.
- `xpath` — the XPath subset fields.json actually writes. Predicates take five
  shapes; anything outside the grammar fails at index-build time, so an SDK bump
  that introduces a new shape is a red test, not a silent gap.
- `index` — node and field xpaths folded into a match tree keyed by
  namespace-URI + local name. The tree *is* the ignore rule: an element with no
  branch is unclaimed content, and there is no second list to drift.
- `parse` — a simultaneous descent of document and tree. Every element,
  attribute and text node must be claimed; the first that is not aborts the
  notice naming the exact path. No partial results: identity row, satellite
  rows and quarantine row are written in one transaction.

### Schema

eForms' *node tree*, not its field list, defines the relational shape
(eforms-data-model.md §2.1): every repeatable-node instance becomes a
`notice_sections` row (`PROCEDURE`, `LOT-0001`, `ORG-0002`, `RES-0001`, … — the
ids change notices reference in BT-13716), and values hang off the nearest
enclosing section in nine typed satellites. Consequences: Lots, LotsGroups and
Parts are sections distinguished by `kind`; organization mentions are
`Organization` sections and their *roles* are the `OPT-300-*`/`OPT-301-*` id-ref
rows; the withheld-field satellite is a **view** over `FieldsPrivacy` sections
rather than a fourth copy of BT-195/196/197/198. Deep results-layer nodes
(LotResult, LotTender, SettledContract, UBO) need no separate extension table —
they are already stored losslessly as sections, so issue 13 promotes them
without a migration of meaning. Money is INTEGER cents, timestamps are UTC
seconds + the published offset, `kind` comes from the SDK **node id** (stable
across minors) rather than `businessEntityId` (recased 1.13 "lot" → 1.15 "Lot").

### Real-daily verification (2026-136, scratch db, production db untouched)

| | |
|---|---|
| notices | 3722 (1.12: 452, 1.13: 2225, 1.14: 1045) |
| parsed | **3715** |
| quarantined by the parser | **7** — and all 7 are ADR-0004 working as designed |
| unclaimed-content | **0** |
| re-run | 0 new rows, 3722 duplicates |

Satellite rows: sections 253 295 (Lot 12 609, LotsGroup 5, Part 27,
Organization 14 388), ids 250 215 (46 926 of them organization role references),
codes 317 396, texts 262 512 in 24 languages, integers 78 350, classifications
62 307, numbers 55 550, dates 36 538, amounts 33 215; the withheld view yields
454 rows.

The 7 quarantines are values the schema cannot hold without loss: 4 amounts with
sub-cent precision (`35016290.22161`) and 3 dates missing eForms' mandatory zone
offset. Rounding or assuming UTC would be the silent data loss ADR-0004 exists
to prevent, so they stay quarantined and reprocessable.

### Five findings from the real data

1. **The SDK's field inventory is not a superset of what TED publishes.** Real
   notices carry UBL elements eForms defines no business term for — a contact's
   `cbc:JobTitle`, a `cbc:Postbox`, a document's `cbc:FileName`/`cbc:LanguageID`,
   `cac:PostAwardProcess` indicators. Dropping them would break "nothing
   silently dropped", so they are mapped through a reviewed `EXTRA` table with
   synthetic `UBL-` field ids (4441 values on the daily), distinguishable from
   business terms by prefix.
2. **Publishers mount SDK subtrees at contexts the SDK does not enumerate** —
   procedure-level `cac:ProcurementProject`/`TenderingTerms`/`TenderingProcess`,
   Lot-level `cac:LotDistribution`, `efac:LotTender` nested inside
   `efac:LotResult`, Lot-only fields on a Part. `ALIASES` grafts the subtree onto
   the target, gap-filling only, so a field the SDK declares at the target always
   wins. This alone accounted for several hundred notices.
3. **Sibling node definitions overlap; they do not partition.** One
   `cac:TendererQualificationRequest` element is described by five node
   definitions whose predicates are mutually non-exclusive. Matching had to
   become the *union* of matching branches. Where a notice omits the
   discriminator a predicate keys on, the element is claimed by name alone and
   the walker requires the candidates to agree on a field before binding a value
   — so relaxing can never mis-attribute (`ambiguous-field` is its own reason).
4. **`dateFieldId`/`timeFieldId` only exist from SDK 1.15**, so date/time pairs
   are recognised structurally by UBL's own naming (`EndDate`/`EndTime`) and a
   deadline is stored as one instant with its offset, not two rows.
5. **Lexical forms are looser than the spec.** eSenders publish sub-second times
   (`18:00:00.0000000`), amounts with trailing zeros (`561906.9100`), counts as
   `0.0`, `@listID` for `@listName` and `@schemeID` for `@schemeName`. Each is
   accepted exactly where no information is lost, and only there.

`ND-ContractingParty`'s `identifierFieldId` is an `id-ref` to the buyer
organization, not its own id — naming the buyer-role section after it collided
with the organization's own section, so only a real `id` names a section.
