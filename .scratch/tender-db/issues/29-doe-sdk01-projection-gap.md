# 29 — DÖE sdk-0.1 notices project to empty canonical Tenders

Status: ready-for-agent

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
