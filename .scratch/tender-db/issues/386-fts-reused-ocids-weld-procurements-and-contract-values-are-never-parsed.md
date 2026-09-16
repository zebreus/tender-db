# 386 — FTS: a publisher-reused ocid welds different buyers' procurements into one Tender, and no FTS contract value is ever parsed

Status: ready-for-agent — unit 2a FIXED and gated 2026-09-16 (the contract's own published value, and the contract-less award's decision date; see the section at the foot). Unit 1 (the ocid weld) and unit 2b (periods, `BT-3202`/`OPT-315`, the ADR-0004 checklist) are open. Filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: defect (sources / fts profile) — unit 1 welds records that were never one procurement, unit 2 serves money and dates the source publishes as `null`
Relates to: 342 (the FTS source; unit 2 complete, OPEN on the 2021-01 backfill and the docs — the parent of both units), 369 (the placeholder procedure-key gate and its unit-5 buyer grouping, which unit 1 extends), 377 (the same constant-key-publisher shape, decided NO GATE on TED for a class of 4 — and it says a platform-level cause reverses that), 34 (the original "every notice sharing the key collapses into one Tender"), 364 (the weld gauge `c0c2581` the FTS arm should feed), 255 (the award decision date's canonical homes, which unit 2's award-only releases never reach), ADR-0003 (merge only on a strong explicit cross-reference), ADR-0004 (the per-profile mapped-or-ignored checklist the `fts` module does not declare), ADR-0014 (contracts as one of the four money loci), CONTEXT.md:113-114, `docs/research/uk-fts.md` §4, `.scratch/tender-db/342-fts-plan.md` §3
Blocked by: nothing

Two gaps in the same profile — the FTS parser and key election shipped with 342 unit 2 (`d7264c8`,
HEAD `5c47984`) — both found in the June-2025 fold (7,243 notices, 6,239 tenders, tender ids
7954583–7975000, the only FTS data on the box). They travel together for three reasons: each makes a
served FTS tender diverge in SHAPE from a TED tender carrying the same facts, so a consumer cannot
compare sources; both land on the same served record (`/v1/tenders/7954584` has another buyer's
contracts AND every one of its 18 contract values is `null`); and both get much more expensive after
plan step 11's 69-month backfill — unit 1 folds 100+ further releases per reused ocid into the
already-wrong tender, unit 2 costs a full FTS reparse to fix afterwards.

## Unit 1 — a reused ocid welds different buyers' procurements

### Observed (verified 2026-09-14 on prod)

At the source, `ocds-h6vhtk-02874c` is not one contracting process. Page 1 alone:

    curl -s 'https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages/ocds-h6vhtk-02874c' \
      | python3 -c "import json,sys,collections;d=json.load(sys.stdin);r=d['releases'];print(len(r),bool(d['links'].get('next')),collections.Counter((x.get('buyer') or {}).get('name') for x in r).most_common(12))"

| page 1 of ocid `ocds-h6vhtk-02874c` | |
| --- | --- |
| releases | 100 (98 `award+contract`, 1 `awardUpdate`, 1 `tenderUpdate`), `links.next` present |
| distinct buyer names | 29 |
| SSE plc | 26 |
| Scottish Hydro Electric Transmission PLC | 13 |
| UK POWER NETWORKS (OPERATIONS) LIMITED | 8 |
| Northern Powergrid | 7 |
| Northern Ireland Electricity Networks Limited | 6 |
| ANGLIAN WATER SERVICES LIMITED | 1 |
| tail (Yorkshire Water, NATIONAL GRID ELECTRICITY DISTRIBUTION, United Utilities Water, Wessex Water, Dŵr Cymru Cyfyngedig, NORTHERN GAS NETWORKS, ACHILLES INFORMATION LIMITED …) | the rest |
| release dates | 2024-04-10 → 2026-08-24 |
| `legalBasis` CELEX 32014L0025 + "Negotiated procedure with prior call for competition" | 99 of 100 |

Unrelated titles — Super Grid Transformers, Pipe & Fittings, fall arrest equipment, External Audit
Services, Driver Training — and the Achilles party carries a UVDB activity classification: this is the
utilities qualification-system shape, independent utilities filing their awards under the register's
ocid.

tender-db keys FTS tenders on the ocid (`BT-04-notice` = `procedure_key`; 342-fts-plan.md:15 "`BT-04-notice`
= ocid groups releases into one Tender", :236 "`procedure_key` = ocid", :353 "one Tender per ocid"), so
the 8 June-2025 releases fold into ONE served procedure:

    curl -s https://tenders.zebreus.click/v1/tenders/7954584 \
      | python3 -c "import json,sys;d=json.load(sys.stdin);print(d['procedure_key'],d['title'],d['parties'],[(v['seq'],v['publication_id'],v['notice_subtype']) for v in d['versions']],len(d['lot_results']),len(d['contracts']))"

→ `procedure_key` `ocds-h6vhtk-02874c`, title **"Supply of Pipe & Fittings"**, parties = Anglian Water
only (`Procedure-Buyer` org 12958110, `Lot-ReviewOrg` 1167362), 8 versions, 18 lot_results, 18 contracts.
Attributing each served contract by its notice:

| served contracts / lot_results | source notice (publication id) | whose, at the source |
| --- | --- | --- |
| 11 | 30810748 (034114-2025) | ANGLIAN WATER — the elected head buyer, BT-21 "Supply of Pipe & Fittings" |
| 5 | 30808452 / 30808563 / 30808581 / 30808612 / 30808647 (031810/031921/031940/031971/032006-2025) | Scottish Hydro Electric Transmission PLC (`ORG-GB-FTS-100545`), BT-21 "CO Supply & Install - Super Grid Transformers 1200MVA - LT407 Banniskirk…" |
| 2 | 30808873 (032234-2025) | SSE plc (`ORG-GB-FTS-68899`), BT-21 "FW Supply & Install … 33kv Outdoor Circuit Breaker" |
| 0 | 30807546 (030892-2025, `tenderUpdate`) | SSE plc + SSEN (`ORG-GB-FTS-150812`), BT-21 "www.ssen.co.uk" |
| **18 total** | | **7 of them belong to a buyer other than the head buyer** |

    curl -s https://tenders.zebreus.click/v1/notices/30808452/content   # BT-04-notice = ocds-h6vhtk-02874c
    curl -s https://tenders.zebreus.click/v1/notices/30807546/content   # same BT-04, different BT-21 and buyer section

A second, independent weld under the sibling ocid `ocds-h6vhtk-02874b` (30 distinct buyers on its page 1):
its 4 June-2025 releases are Dŵr Cymru (031078-2025), Northern Gas Networks (031746-2025), Cadent Gas
(032061-2025) and Affinity Water (033117-2025), and

    curl -s 'https://tenders.zebreus.click/v1/tenders?source=fts&publication_id=033117-2025'
    curl -s 'https://tenders.zebreus.click/v1/tenders?source=fts&publication_id=031078-2025'

both resolve to **tender 7954583**, `procedure_key` `ocds-h6vhtk-02874b`, version 4, head title
"C-04193 Electrical Tools & Components" — Affinity Water's.

Two SQL reads were written for this and refused by the permission classifier as Production Reads; they
were not worked around, and their numbers stand as the reporter's measurement, not as verified facts:

    SELECT p.tender_id, count(DISTINCT p.organization_id) AS buyer_orgs, min(o.name) AS a_name, max(o.name) AS z_name
    FROM tender_version_parties p JOIN organizations o ON o.id=p.organization_id
    WHERE p.tender_id IN (7954590,7954584,7962341,7958424) AND p.role='Procedure-Buyer' GROUP BY p.tender_id

| tender | buyer orgs (reported) | |
| --- | --- | --- |
| 7954584 | 4 | 'Anglian Water Services Ltd' … 'Scottish Hydro Electric Transmission plc' |
| 7954590 ("Taxi Vehicles", 123 versions) | 1 | a genuine DPS/framework chain, NOT this class |
| 7962341, 7958424 | 1 each | likewise |

    SELECT count(*) AS tenders, sum(nv) AS versions_total FROM (SELECT p.tender_id AS tid, max(t.current_seq) AS nv
      FROM tender_version_parties p JOIN tenders t ON t.id=p.tender_id AND t.source='fts' AND t.current_seq>1
      JOIN (SELECT tender_id, seq, count(DISTINCT mention_section_id) AS n FROM tender_version_parties
            WHERE tender_id BETWEEN 7954583 AND 7975000 AND role='Procedure-Buyer' GROUP BY tender_id, seq) pv
        ON pv.tender_id=p.tender_id AND pv.seq=p.seq
      WHERE p.tender_id BETWEEN 7954583 AND 7975000 AND p.role='Procedure-Buyer'
      GROUP BY p.tender_id HAVING count(DISTINCT p.mention_section_id) > max(pv.n))

→ reported 19 tenders / 55 versions of the June-2025 fold (of 7,616 FTS tenders, 856 multi-version)
whose buyer party ids differ across versions. That is a CANDIDATE count, not 19 convictions — 7954590
shows single-buyer chains are a separate, legitimate class — and it needs re-running.

### Why it matters

A consumer of `/v1/tenders/7954584` reads that Anglian Water Services Ltd awarded five Super Grid
Transformer contracts and a 33kV circuit-breaker contract. It did not. The served record is internally
coherent and entirely constructed here: head title from seq 8, head buyer from seq 8, every buyer's
contracts underneath. Any market-share, competitor or buyer-spend read over FTS — the product's stated
use — attributes seven of eighteen contracts to the wrong utility, and there is nothing in the served
payload that shows the fold happened. `winners` are not affected: the source awards carry
`suppliers: []` and every lot_result serves `winners: []`, so awards and contracts are the misattributed
objects.

It gets structurally worse on schedule. Each of these two ocids already has 100+ further source releases
spanning 2024-04 → 2026-08 from ~30 utilities, and plan step 11's 2021-01 backfill will fold every one
of them into these same two tenders.

### Why this is ours, not the publisher's

The publisher published one ocid, but it never published "Anglian Water awarded Super Grid Transformers"
— it published distinct buyers, distinct BT-21 titles and distinct award objects per release, all
visible in the source JSON. tender-db's ocid-only key plus head election constructs the claim. That is
the "internally coherent, entirely fabricated record" the board convicted on in 369, and CONTEXT.md:113-114
promises the opposite direction of failure: "a missed link splits a Tender, never wrongly merges";
ADR-0003 merges only on a strong explicit cross-reference. The existing gates cannot see this: 369's
`is_placeholder_key` (`crates/ingest/src/project.rs:5327-5331`) returns `false` for any non-UUID string,
so an FTS ocid never reaches its buyer test; 377 decided "no gate" for TED on a class of four and said
in terms that an identifiable platform-level cause justifies a parser-level rule — here the platform
and the shape ARE identifiable (FTS, CELEX 32014L0025 utilities qualification systems), and 377 does
not name FTS.

### Repro

1. `curl -s 'https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages/ocds-h6vhtk-02874c'`
   and count distinct `buyer.name` over `releases[]` — 29 names, one ocid, `links.next` present.
2. `curl -s https://tenders.zebreus.click/v1/tenders/7954584` — one tender, one buyer (Anglian Water),
   8 versions, 18 contracts.
3. `curl -s https://tenders.zebreus.click/v1/notices/30808452/content` — `BT-04-notice` =
   `ocds-h6vhtk-02874c`, BT-21 "CO Supply & Install - Super Grid Transformers…", buyer
   `ORG-GB-FTS-100545`. Same BT-04, different buyer, different procurement.
4. Independent second case: `curl -s 'https://tenders.zebreus.click/v1/tenders?source=fts&publication_id=033117-2025'`
   and `…publication_id=031078-2025` — both land on 7954583.

### Done when

- An FTS key-election rule exists and is recorded on 342's "departures from §3" list: either 369 unit 5's
  refusal pre-filter (which already stores `buyer_key` on `plan_notice` and groups a refused key as
  `refused:{procedure_key}:{buyer_key}`) is extended to `source='fts'` keys whose notices carry disjoint
  buyer keys, or the parser keys the utilities qualification-system class on ocid+buyer.
- `GET /v1/tenders/7954584` no longer serves Scottish Hydro's 5 and SSE plc's 2 contracts under Anglian
  Water: every contract on each served FTS tender traces to that tender's own buyer.
- `?publication_id=033117-2025` and `?publication_id=031078-2025` resolve to two different tenders.
- A fixture pins both directions: two releases under one ocid from two buyers fold to two tenders, and a
  genuine single-buyer chain (the 7954590 shape, 123 versions, 1 buyer) still folds to one.
- The buyers-differ-across-versions SELECT is re-run over 7954583–7975000 and its before/after numbers are
  recorded here (the reported 19 tenders / 55 versions is unverified).
- 364's weld gauge counts the FTS arm, so a recurrence is visible on the weekly report before the
  2021→ backfill lands.

## Unit 2 — contract values are never parsed, and award-only releases lose their date and period

### Observed (verified 2026-09-14 on prod)

Every FTS contract is served with `value: null` and `amounts: []`, although the release publishes an
amount. Source:

    curl -s 'https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages/028961-2025' \
      | python3 -c "import json,sys;r=json.load(sys.stdin)['releases'][0];print(r['contracts'])"

→ `contracts[0].value = {amount 54393.6, amountGross 67992, currency GBP}`, period 2025-05-19 → 2028-05-18,
`dateSigned` 2025-04-04. Served:

    curl -s https://tenders.zebreus.click/v1/notices/30805632/content

→ section `CON-1` holds `BT-150-Contract` and `BT-145-Contract` (2025-04-04T00:00:00+00:00) and nothing
else. `/v1/tenders/7956308` (that release's tender) serves `CON-1` with `value: null`, `amounts: []`,
`dates: []` against a published 54,393.60 GBP.

The award-only shapes lose the award decision date and the contract period outright:

    curl -s 'https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages/029014-2025' \
      | python3 -c "import json,sys;r=json.load(sys.stdin)['releases'][0];print(r['awards'],r.get('contracts'))"
    curl -s https://tenders.zebreus.click/v1/notices/30805683/content \
      | python3 -c "import json,sys;d=json.load(sys.stdin);print([(s['section_id'],v['field_id'],v.get('value')) for s in d['sections'] for v in s['values'] if v['type'] in ('date','amount')])"

| 029014-2025 (UK6) | source | served notice 30805683 |
| --- | --- | --- |
| `awards[0].date` | 2025-05-08T00:00:00Z | absent |
| `awards[0].contractPeriod` | 2025-06-06 → 2025-06-06T23:59:59Z | absent |
| `awards[0].value` | 100000 GBP | `TEN-1-0` `BT-720-Tender` 10000000 (cents) GBP |
| `contracts[]` | none | — |
| the only date on the notice | | `OPP-012-notice` |

Canonical layer, June-2025 fold, tender ids 7954583–7975000 (both SELECTs in the finding's evidence;
the box read was denied at review time, so these stand as the reporter's measurement):

| | fts | ted, same id range |
| --- | --- | --- |
| contracts | 5,158 | 2,263 |
| … with `cents` | **0 (0%)** | 2,042 |
| … with `decided_utc` | **0** | 856 |
| … with `concluded_utc` | 5,066 | 2,054 |
| lot_results | 16,948 | 10,749 |
| … with `decided_utc` | **0** | 5,828 |
| lot_results with `awarded_cents` | 1,247 | — (award value does flow) |

Parse-layer census over the whole fold (notice_id 30805000–30813000, source `fts`, 7,243 notices = all):

| field id | rows / notices |
| --- | --- |
| `BT-145-Contract` | 4,190 / 2,848 |
| `BT-1451-Contract` | **0** |
| `BT-720-Tender` | 824 / 741 |
| any contract-value field id | **0 — none exists** |

Shape census by OPP-070, which says where each loss lands:

| subtype | notices | with a RES section | with a CON section |
| --- | --- | --- | --- |
| UK7 | 1,399 | 1,399 | 1,399 |
| award+contract | 1,407 | 1,407 | 1,368 |
| UK6 | 483 | 483 | **0** |
| UK5 | 285 | 285 | **0** |
| awardUpdate+contractUpdate | 86 | 86 | 86 |

Code, `crates/ingest/src/fts/parse.rs` at HEAD `5c47984`: `struct Contract` (:671-678) deserialises only
`{id, awardID, dateSigned, documents}` — no `value`, no `period`; `struct Award` (:642-653) has no
`contractPeriod`; and the award `date` is pushed only as `BT-1451-Contract` *inside*
`for contract in &release.contracts {` (:289-301), so a release with no `contracts[]` loses it entirely.
`grep -c 'BT-3202\|OPT-315' crates/ingest/src/fts/parse.rs` → **0**, although 342-fts-plan.md:154 promises
both for `contracts[]`; and `crates/ingest/src/project.rs:4370-4377` derives a contract's cents ONLY from
its `BT-3202` bid refs ("A contract's value is the value of the Bid(s) it settled — eForms contracts carry
no value of their own"). So even the award+contract shape whose `award.value` reaches `BT-720-Tender` has
no route to a contract value.

Two corrections to the finding's own framing, kept because they bound the fix: "1,399 UK7-less award
releases" is wrong — UK7 (1,399) is the contract-BEARING class; the award-date-bearing no-contract classes
are UK6 483 + UK5 285 (+43 UK12, +39 award+contract without a CON section). And on the contract-bearing
shapes the source itself carries no `awards[].date` (probed on 032363-2025, which publishes 45,070.80 GBP
only at `contracts[0].value` with `awards[0].date`/`value` null), so `BT-1451` = 0 is partly a source fact
there; the DATE loss is confined to the no-contract classes, while the contract-VALUE loss is ours everywhere.

### Why it matters

A consumer asking "what did this contract cost" gets `null` for every FTS contract in the corpus, and
`decided` for every FTS lot_result, while the same questions answered against TED in the same id range
return 2,042 of 2,263 and 5,828 of 10,749. The API gives no way to tell "the publisher did not say" from
"we did not read it" — 028961-2025 says 54,393.60 GBP in plain sight. A cross-source comparison silently
reads FTS as a corpus of value-less contracts, which is the shape ADR-0014 exists to prevent, and
`tender.value` / award value flowing while contract value does not makes the gap look like a publisher
quirk rather than a parser hole.

### Why this is ours, not the publisher's

The source publishes all of it and `docs/research/uk-fts.md` §4 (l.127-128) recorded it as published:
award values, `dateSigned`, award `contractPeriod` and contract `period`. The parser simply does not
deserialise those fields, and the plan's §3b note (l.177-179) observed that `awards[].contractPeriod`
and `contracts[].period` are where the periods live and then mapped neither — a scoping note, not a
recorded wontfix; the `fts` module declares no mapped-or-ignored checklist although ADR-0004's amendment
promises one per profile. Issue 255 is the destination, not the fix: it gave eForms `BT-1451` and legacy
award dates their canonical homes before FTS existed, and the FTS profile should feed them. 342 is open
but tracks only the 2021-01 backfill and the docs; neither of parse.rs's two commits (`d7264c8`, `18ce0c1`)
adds value or period; no issue on the board mentions FTS contract values, `BT-3202` for FTS, or award
`contractPeriod`.

### Repro

1. `curl -s 'https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages/028961-2025'` — read
   `releases[0].contracts[0].value` → `{amount 54393.6, amountGross 67992, GBP}`.
2. `curl -s https://tenders.zebreus.click/v1/notices/30805632/content` — section `CON-1` has only
   `BT-150-Contract` and `BT-145-Contract`. Then `/v1/tenders/7956308` — that contract serves
   `value: null`, `amounts: []`.
3. `curl -s 'https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages/029014-2025'` — award
   date 2025-05-08, `contractPeriod`, no `contracts[]`. Then `/v1/notices/30805683/content` — the only
   date is `OPP-012-notice`.
4. `grep -n 'struct Contract' -A 8 crates/ingest/src/fts/parse.rs` (no `value`, no `period`) and
   `grep -c 'BT-3202\|OPT-315' crates/ingest/src/fts/parse.rs` → 0.

### Done when

- `Contract` deserialises `value` and `period`, `Award` deserialises `contractPeriod`, and each has a
  mapped destination (contract value → the contract's amount; periods → the `BT-536/537` pair the plan
  already keeps at Lot scope).
- The parser emits `BT-3202-Contract` and `OPT-315-LotResult` as 342-fts-plan.md:154 promises, or the plan
  records in writing why it does not — today the grep count is 0 and `project.rs:4370-4377` has no other
  route to a contract's cents.
- An award-only release's `awards[].date` reaches issue 255's legacy destination
  (`tender_version_lot_results.decided_*`) instead of being emitted only inside the `contracts[]` loop —
  covering the 483 UK6 + 285 UK5 (+43 UK12, +39) notices that have a RES section and no CON section.
- `/v1/notices/30805632/content` `CON-1` carries 54,393.60 GBP and `/v1/tenders/7956308` serves that
  contract with a value and a non-empty `amounts`; `/v1/notices/30805683/content` carries 2025-05-08 and
  the 2025-06-06 → 2025-06-06 period.
- The two canonical SELECTs over 7954583–7975000 are re-run and recorded: FTS contracts `with_value` and
  lot_results `with_decided` are no longer 0 of 5,158 and 0 of 16,948, and the residue is explained by
  source absence (the contract-bearing shapes genuinely carry no `awards[].date`).
- The `fts` module declares the per-profile mapped-or-ignored checklist ADR-0004's amendment promises, so
  a published field cannot go unmapped silently again.
- Shipped BEFORE plan step 11's 69-month backfill, or the backfill pays a full FTS reparse afterwards.

## Unit 2a landed 2026-09-16 — the contract's own value, and the contract-less award's date

Status: fixed and gated, not yet deployed. Unit 1 (the ocid weld) and unit 2b (periods,
`BT-3202`/`OPT-315`, the ADR-0004 checklist) are untouched.

### What changed

**`Contract` deserialises `value`, and it has a destination.** The awkward part of this unit was
that there is nowhere obvious to put it: eForms contracts carry no value of their own — their money
is the value of the Bid they settled, reached through `BT-3202` — so there is no BT to borrow, and
`crates/ingest/src/fts/parse.rs` opens by saying its field ids are "the eForms ones the projection
already reads … not cosmetic". Weighed three ways:

- **Route it through `BT-3202` to the award's LotTender.** Rejected: on the shape that actually
  carries the money the award publishes NO value (028961-2025 has `awards[0].value: null` beside
  `contracts[0].value: 54393.6`), so this would mean writing the contract's amount onto a *bid* the
  publisher never priced, and two contracts settling one award would each report the other's money.
- **Mint a LotTender per contract.** Rejected outright — it invents a bid object, with no supplier
  and no tender behind it, to hold a number.
- **One new field id, `OCDS-ContractValue`, read as the contract's value.** Taken. It is the
  source's own vocabulary for a fact eForms does not model, `pub(crate)` in the FTS parser and
  IMPORTED by `project.rs` rather than re-spelled, so the emitter and the reader cannot drift.

The projection prefers the bid-derived total and falls back to the published amount, so **no eForms
or TED contract changes shape** — they emit no `OCDS-ContractValue` and take the same branch they
always did.

**The award's decision date now lands on the RESULT too.** It was emitted only inside
`for contract in &release.contracts`, so the UK6/UK5 classes — an award with no `contracts[]` at all,
483 + 285 notices — dropped it whole. It is emitted as `BT-1451-LotResult` on the result section and
read there into `lot_results.decided`: the same destination and the same reasoning issue 255 recorded
for the legacy award blocks, which also have no contract graph to hang it on. Inert for every other
profile, which emits no such spelling.

### Test

`a_contracts_own_value_and_a_contractless_awards_date_are_both_emitted` (parse layer) and
`an_fts_contract_keeps_its_published_value_and_a_contractless_award_its_date` (projection), both on
committed fixtures of the two releases the finding named. Run red first, one destination at a time:
`left: (None, None)` against the published 54,393.60 GBP, and `left: None` against the award date.

`028961-2025.json` is a new fixture, fetched from the live API today; it independently reproduces the
finding's central claim (`awards[0].value` null, `contracts[0].value` 54,393.60 GBP).

### Deliberately NOT done, and why — unit 2b

- **The periods.** `Contract.period` and `Award.contractPeriod` are read by nothing because
  `tender_version_contracts` has no duration columns at all (`crates/store/src/canonical.rs:783`),
  and the `BT-536/537` pair this issue points at is a LOT-scoped destination that
  `tender.lots[].contractPeriod` already fills. Pushing a contract's duration there would collide
  with a different fact under the same key. That is a schema decision, not a parser one, and
  inventing the collision is worse than the current absence. Unit 2b, stated as a choice rather than
  an omission.
- **`BT-3202-Contract` / `OPT-315-LotResult`**, which 342-fts-plan.md:154 promises: they link the
  results graph rather than carry money, and with the value question settled independently they are
  now a linkage unit, not a blocker. Still owed, still unit 2b.
- **The ADR-0004 per-profile mapped-or-ignored checklist** for the `fts` module — the thing that
  would have caught this class before a consumer did.

### Live acceptance, owed after deploy (needs an FTS re-parse — the parse layer changed)

- `/v1/notices/30805632/content` `CON-1` carries 54,393.60 GBP, and `/v1/tenders/7956308` serves that
  contract with a value and a non-empty `amounts`.
- `/v1/notices/30805683/content` carries the 2025-05-08 award date.
- The two canonical SELECTs over 7954583–7975000 re-run and recorded: FTS contracts `with_value` and
  lot_results `with_decided` are no longer 0 of 5,158 and 0 of 16,948, with the residue explained by
  source absence.
