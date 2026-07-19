# 13 — Results layer: Bids, Awards, Contracts, full BT coverage

Status: resolved
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

## Answer

Delivered on branch `worktree-agent-a9f5ff7446052fedc` (commit d8ba678 +
this note): the canonical results layer — Bids, LotResults, Contracts —
projected from the section/satellite parsed layer issue 03 already stores
losslessly. (The issue's original "full BT coverage" half was moot by issue
03's design: completeness is total at the parsed layer; this issue was pure
canonical modeling.) Verified on the real TED daily **2026-136** (scratch db,
production untouched): **6248 lot_results, 11 006 bids, 6660 contracts,
13 270 bid-party links**; a re-run wrote nothing.

### Schema (crates/store/src/canonical.rs)

A results entity is identified by **(tender, origin notice, section key)** —
identity tables `lot_results` / `bids` / `contracts` (RES-/TEN-/CON- keys are
notice-local, so rounds can never collide) — with version-keyed state
satellites exactly like the lot pattern: `tender_version_lot_results`
(resolved lot FK, BT-142 decision, BT-144 reason, awarded value),
`tender_version_result_winners`, `tender_version_result_stats` (BT-759/760
received-submission counts as published: tenders, t-sme, t-eea, …),
`tender_version_bids` (lot FK, BT-720 value), `tender_version_bid_parties`
(the TenderingParty flattened onto the Bid: role tenderer|subcontractor, org
FK, mention evidence), `tender_version_contracts` (BT-150 buyer id, BT-145
conclusion date, value derived from the settled Bids — eForms contracts
carry no value of their own). `v_lot_results` is the competitor view: one
row per (result, winning org). Change kinds `lot_result`/`bid`/`contract`
join the cursor spine same-transaction, diff-scoped like lots.

### Round semantics (the FA/DPS rules, crates/ingest/src/project.rs)

Results are **additive**, never superseded: each result notice contributes a
`Round`, and version N's results are the union of all rounds so far — the
verified tranche pattern (24/24, 37/37 union) and repeated-CAN pattern both
fall out of one rule. The one exception implements the empirical verdict
"same BT-701 + efac:Changes ⇒ correction": a correction *replaces* the round
of its logical notice instead of duplicating it (diff reads removed+added).
Round-local lot relabeling cannot misgroup: results keep their origin
notice, the reused lot id maps to one Lot identity whose content stays
per-version (asserted in the FA test). Winners and awarded values resolve
through the notice's own graph — LotResult → SettledContract → LotTender →
TenderingParty → Organization, falling back to the result's direct tender
refs when no contract is linked (real eSenders list *all* received tenders
under OPT-320, so the settled contract is the stronger signal). Award-side
party roles (Tenderer, Contract-Signatory, …) are now scoped to their Lot
via that graph — issue 04's noted limitation is closed.

### API (kept minimal)

`/v1/tenders/{id}` gains `lot_results` (winners + statistics embedded),
`bids` (consortium embedded), `contracts`; `/v1/tenders` gains
`winner=<org id>` (a version predicate, so SSE inherits it).

### Verification (2026-136, scratch db)

| | |
|---|---|
| lot_results / bids / contracts | **6248 / 11 006 / 6660** |
| decisions (distinct results) | selec-w 4805, clos-nw 1044, none 368, open-nw 29, "unpublished" 2 |
| winner-link resolution | **4805/4805 awarded results link a winner (100%)** — 4781 canonical orgs, 25 provisional |
| lot reference resolution | 6248/6248 |
| contracts with derived value | 6334/6660 |
| rebuild / re-run | 87.6 s / no-op (0 writes, 0 change rows) |

Competitor query over `v_lot_results` (single-winner results, EUR): Intesa
Sanpaolo 2 lots €123.75M; Max Bögl Nederland €103.46M; E4 €80.00M; John Sisk
& Son 2 lots €71.76M. The unfiltered top-5 is dominated by one Italian
multi-winner framework (00495141-2026, neurosurgery materials, 35 results ×
~30 winners) — for multi-winner awards `awarded_cents` is the total across
winning bids attributed to each winner, documented on the view; per-org own-
bid attribution would need the result→bid edge, deliberately not stored.

### Surprises

1. The **stale-binary trap**: the first VPS run silently used issue 04's
   hardlinked release bins (build session killed by a same-name tmux
   session) and wrote zero results. `strings <bin> | grep lot_results` is
   the cheap freshness check.
2. **Multi-winner FAs are common enough to dominate naive rankings** (one
   notice with ~30 winners per lot) — competitor queries must decide whether
   a shared award counts fully per winner.
3. BT-142 carries the literal code `unpublished` in the wild (2 results) —
   withheld-by-code, not by FieldsPrivacy.
