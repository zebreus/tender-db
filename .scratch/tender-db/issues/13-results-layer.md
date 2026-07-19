# 13 — Results layer: Bids, Awards, Contracts, full BT coverage

Status: ready-for-agent
Blocked by: 04

Goal: the tendering-results half of the canonical model (eForms extension
entities) and the remaining eForms BT satellites — the all-BT checklist goes
green for the eForms profile.

Scope: LotResult/LotTender→Bid, TenderingParty, SettledContract→Contract,
contract modifications as version events; bid statistics; award criteria;
framework/DPS handling (repeated CANs under one Tender, tranche CAN
additivity, defensive lot refs — ted-empirical-checks.md); every remaining
fields.json field mapped or excluded (completeness green with zero
outstanding entries); extend API + SQL views accordingly.

Acceptance: eForms completeness test passes with no unaccounted fields; the
HU framework fixture (769333-2023 chain) projects without misgrouping;
competitor-style queries (org → won lots → values) return correct fixtures.
