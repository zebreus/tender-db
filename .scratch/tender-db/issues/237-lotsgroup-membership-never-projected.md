# 237 — a multi-lot bid names a LotsGroup, and we never record which lots that group contains

Status: needs-triage — found 2026-08-18 answering "how do we represent bids covering multiple lots?"
Kind: projection mapping gap (parse layer HAS the data) + a coverage-gate blind spot
Blocked by: —
Relates to: 13 (results layer), 116 (tender detail reported more lots than it shipped), 88/85 (the
"parser captures it, fold drops it" class), 235 (same class, one metric over)

## The model is right; one edge is missing

A Bid is never many-to-many with Lots, and tender-db is faithful to eForms here. `efac:LotTender`
(TEN-nnnn) carries exactly ONE lot reference via `BT-13714-Tender`, pointing at either a Lot or a
**LotsGroup** (`docs/research/eforms-data-model.md:173` — "**Tender (tender-db: Bid)** N—1
Lot/LotsGroup"). A bid covering five lots is expressed by the buyer declaring a LotsGroup (GLO-nnnn)
for combined award, and the bid referencing the group.

tender-db models that correctly: `LOT_KINDS = ["Lot", "LotsGroup", "Part"]` (project.rs:297), a
LotsGroup is a row in `lots` distinguished by `tender_version_lots.kind`, and
`tender_version_bids.lot_id` can therefore name a group. Its `PRIMARY KEY (tender_id, seq, bid_id)`
with a single `lot_id` is the CORRECT cardinality, not a limitation — worth stating plainly, because
the table comment "one offer on one Lot" reads like a simplification and is not one.

## What is missing: the group's composition

**`BT-1375-Procedure` ("Group Lot Identifier") — the LotsGroup's member-lot references — is captured
by the parser and dropped by the fold.**

- The SDK declares it (`sdk/fields-1.13.0.json`:
  `…LotsGroup/cac:ProcurementProjectLotReference/cbc:ID[@schemeName='Lot']`), and the eForms parser is
  inventory-driven, so it is claimed into the parse layer. ADR-0004's exhaustive-consumption rule makes
  this near-certain rather than hopeful: a field the parser did NOT claim would quarantine every notice
  carrying a lots-group composition, and those notices are in the corpus, parsed.
- It appears **nowhere** in `project.rs` — not in a fact table, not in the explicit-ignore list. There is
  no `lot_group_members` table and no lot→lot linkage of any kind.

So we can say "this bid covers GLO-0002" and cannot say which lots GLO-0002 is.

## Why it matters

- Any per-lot rollup of bids or awards **silently omits combined-award bids**: they attach to the group
  row, not to its member lots. A bidder's per-lot win rate, a lot-level competition count, "who bid on
  lot 3" — all quietly wrong wherever a group was used, and right everywhere else, which is the hardest
  kind of wrong to notice.
- It is **recoverable without a re-parse**: the facts are already in the parse layer, so this is a
  projection mapping plus a re-fold — far cheaper than the text-era (issue 232) and DE-1.x (issue 100)
  cohorts, which need the parse layer rewritten.

## Second finding: the disposition gate does not cover SDK-native ids

`ubl_grafts_are_all_mapped_or_ignored` (project.rs:3856) enforces "every field is MAPPED or EXPLICITLY
IGNORED with a reason" — but only for tender-db's own `UBL-*` grafts, by scanning `index.rs` for the
`UBL-` prefix. SDK-native `BT-*` ids have no equivalent gate, which is exactly how BT-1375 slipped
through silently. That is the same class the gate was built to stop (issue 85's 218K factless tenders).

Extending it to `BT-*` means dispositioning every SDK field, which is a much larger set — so this
probably wants to start as a report (how many SDK fields the fold reads vs. how many the parser claims)
rather than a hard gate that fails on day one. Sized honestly, it is its own piece of work.

## Acceptance

- A `lot_group_members` (or equivalent) canonical link, populated from `BT-1375-Procedure`, with the
  group and member both resolving to `lots` rows of the right `kind`.
- A test built on the committed fixture `eforms/can-maximal-sdk17.xml`, which carries BT-1375 twice, so
  no new fixture is needed.
- A re-fold, then a check that bids on groups can be attributed to member lots.
- Separately: a recorded decision on whether the SDK-native disposition gap becomes a gate or a report.
