# 29 — DÖE sdk-0.1 notices project to empty canonical Tenders

Status: needs-verification

Surfaced by the issue-27 data-quality report over the real fixture
corpus: the `eforms:eforms-sdk-0.1` era measures **0 %** on every field —
title, buyer, value, CPV, deadline, winner — while parsing cleanly at the
notice layer (`parse_state = 'parsed'`, no quarantine). Root cause is a
notice→canonical mapping gap, not a parse gap: the sdk-0.1 payloads carry
their content under `SDK01-*` field stems (`SDK01-ProcurementProject-Name`
= title, `SDK01-ProcurementProjectLot-ProcurementProject-Description`,
`SDK01-ContractingParty-Party-PartyName-Name` = buyer,
`SDK01-TenderResult-WinningParty-Party-PartyName-Name` = winner, CPV/NUTS
codes, dates), but `crates/ingest/src/project.rs`'s canonical field tables
(`TEXTS`/`AMOUNTS`/`CLASSIFICATIONS`/`DATES`) and `role_name` only know
`BT-*`, `TED-*` and `TXT-*` stems. The only `SDK01-*` stems mapped are the
two publication/dispatch dates (`SDK01-RequestedPublicationDate`,
`SDK01-IssueDate`). So every sdk-0.1 notice projects to a contentless
island Tender.

Impact is large: sdk-0.1 is a **permanent ~40 %-of-volume DÖE dialect**
(CONTEXT.md, not a transition artifact), so ~40 % of German holdings are
currently queryable only at the raw notice layer, invisible to the
Tender/Lot/Organization product surface (title search, buyer/winner
rollups, CPV filters all miss them).

Task: extend the projection's stem→canonical mappings (and `role_name` /
result-graph binding) to the `SDK01-*` vocabulary so sdk-0.1 notices yield
title, description, buyer, winner, CPV/NUTS, value and deadlines like the
other eForms profiles. The sdk-0.1 result graph
(`SDK01-TenderResult-WinningParty-…`) also needs binding into
`lot_results`/winners. Re-project and confirm with the issue-27
`data-quality` tool that the `DÖE sdk-0.1 island` era's completeness rises
off the floor.

Acceptance: data-quality report shows non-trivial title/buyer/winner
completeness for `eforms:eforms-sdk-0.1`; a spot-checked sdk-0.1 notice
resolves to a Tender carrying its title, buyer and (for the CAN) winner
through the API.

## Comments

### 2026-07-21 — fixed in the projection (needs-verification)

Root cause confirmed and fixed in `crates/ingest/src/project.rs`:
- **Value fields.** `canonical_name` now matches the full field id first, then
  the coarse `stem` — the sdk-0.1 path-shaped ids (`SDK01-ProcurementProject-Name`
  vs `-Description`) collide under the stem. Added `SDK01-*` entries to TEXTS
  (title/description, Tender + Lot scope), CLASSIFICATIONS (RealizedLocation NUTS
  → place), DATES (TenderSubmissionDeadlinePeriod EndDate → submission_deadline).
- **Buyer.** sdk-0.1 names its buyer by an inline `ContractingParty` section (no
  eForms `Organization`, no OPT-300 ref). `take_mentions` now seeds mentions from
  the sdk-0.1 party sections (reading the *direct* Party name/country, never the
  nested `ServiceProviderParty` eSender), and `read` synthesises a `buyer` role at
  the ContractingParty section.
- **Winner + results.** New `read_sdk01_results`: each `TenderResult` section
  becomes a LotResult whose `WinningParty` children are direct winners (resolved
  through the same org map as the legacy inline-award path) and whose
  `TenderResultCode` is the decision.

Verified on the two committed sdk-0.1 fixtures (regression tests in
`tests/project.rs::sdk01_projects_title_buyer_and_winner` and
`tests/data_quality.rs::sdk01_era_completeness_is_no_longer_zero`): the era went
from 0 % on every field to title="Lose Möblierung", buyer="VGem Volkach…",
winner="1. Firma: IABG mbH", plus description/place/deadline and a materialised
`lot_result`. Full `cargo test -p ingest` green, clippy clean (`-D warnings`), no
regressions across the other eras.

**To verify on prod:** after a re-projection over the sdk-0.1 backlog, run the
issue-27 `data-quality` tool and confirm the `DÖE sdk-0.1 island` era's
completeness is off the floor. Not deploying — the boundary is the team lead's.
Out of scope here and left as a possible follow-up: sdk-0.1 `SDK01-ContractFolderID`
is not read as a procedure key, so uuid-bearing sdk-0.1 CANs stay islands rather
than merging with a TED/eForms-DE twin (identity change, not a completeness gap).
