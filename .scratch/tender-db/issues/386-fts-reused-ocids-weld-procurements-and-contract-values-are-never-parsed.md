# 386 — FTS: a publisher-reused ocid welds different buyers' procurements into one Tender, and no FTS contract value is ever parsed

Status: ready-for-agent — **unit 1's key election is BUILT and gated 2026-09-18** (`2c2d0b0`, see the foot): an FTS ocid whose releases carry two or more distinct buyer sets splits per buyer at the plan's refused-key gate, pinned at the store and end to end; the standing FTS rows keep the welded shape until the fts profile is refolded — a production write the operating session's classifier refuses, so it waits for Lennart's go-ahead with the command at the foot. Unit 2a FIXED and gated 2026-09-16 (the contract's own published value, and the contract-less award's decision date). Unit 2b's ADR-0004 checklist is BUILT, gated (126/126) and DEPLOYED 2026-09-18 11:11 UTC at `8b895e1` (see the foot: `fts::checklist`, pinned by a census over every fixture release — 224 paths, all disposed, 35 owed); **the `BT-3202`/`OPT-315` linkage is BUILT, gated (127/127) and DEPLOYED 2026-09-18 15:03 UTC at `6a840ae` (see the foot; new FTS ingests carry it from the next daily tick, the standing rows wait for the gated re-parse)**; **and the periods landed, gated (127/127) and DEPLOYED 2026-09-18 18:10 UTC at `846f856` (see the foot: no schema change — a lot publishing no period inherits its single-lot award's `contractPeriod`, else that award's contract's `period`, as the `BT-536/537-Lot` pair; only the `maxExtentDate` leaves stay `owed:`). Unit 2b is COMPLETE at the parse layer; every unit's standing rows wait for the gated FTS refold + re-parse.** Filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: defect (sources / fts profile) — unit 1 welds records that were never one procurement, unit 2 serves money and dates the source publishes as `null`
Relates to: 342 (the FTS source; unit 2 complete, OPEN on the 2021-01 backfill and the docs — the parent of both units), 369 (the placeholder procedure-key gate and its unit-5 buyer grouping, which unit 1 extends), 377 (the same constant-key-publisher shape, decided NO GATE on TED for a class of 4 — and it says a platform-level cause reverses that), 34 (the original "every notice sharing the key collapses into one Tender"), 364 (the weld gauge `c0c2581` the FTS arm should feed), 255 (the award decision date's canonical homes, which unit 2's award-only releases never reach), ADR-0003 (merge only on a strong explicit cross-reference), ADR-0004 (the per-profile mapped-or-ignored checklist the `fts` module does not declare), ADR-0014 (contracts as one of the four money loci), CONTEXT.md:113-114, `docs/research/uk-fts.md` §4, `.scratch/tender-db/342-fts-plan.md` §3
Blocked by: nothing

## Verify

    for p in 033117-2025 031078-2025; do curl -s --max-time 20 "https://tenders.zebreus.click/v1/tenders?publication_id=$p&limit=1" | python3 -c "import json,sys; print('$p', [t['id'] for t in json.load(sys.stdin)['items']])"; done

- **done**: two DIFFERENT tender ids — unit 1's split reached the standing rows, i.e. the gated fts refold ran. Unit 2b's standing rows ride the gated FTS re-parse in the same go-ahead: `/v1/tenders/7956308` (028961-2025) then serves lot `1`'s `duration_start` 2025-05-19 in `dates`, contract 1's period.
- **open**: the same id twice (read 2026-09-18 18:10 UTC at `846f856`: both `7954583`; 7956308's `dates` is `[]`)

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
  now a linkage unit, not a blocker. **Landed 2026-09-18, see the foot.**
- **The ADR-0004 per-profile mapped-or-ignored checklist** for the `fts` module — the thing that
  would have caught this class before a consumer did. **Landed 2026-09-18, see the foot.**

### Live acceptance, owed after deploy (needs an FTS re-parse — the parse layer changed)

- `/v1/notices/30805632/content` `CON-1` carries 54,393.60 GBP, and `/v1/tenders/7956308` serves that
  contract with a value and a non-empty `amounts`.
- `/v1/notices/30805683/content` carries the 2025-05-08 award date.
- The two canonical SELECTs over 7954583–7975000 re-run and recorded: FTS contracts `with_value` and
  lot_results `with_decided` are no longer 0 of 5,158 and 0 of 16,948, with the residue explained by
  source absence.

### Unit 2a VERIFIED LIVE 2026-09-16, rev `8bf051b`

FTS re-parsed and re-projected to pick up the parse-layer change — job 2325: 10,600 notices across
10 packages, **0 unmatched, 0 re-keyed, 0 now failing**; job 2326: 9,233 tenders written.

**The contract's own value reaches the API.**

    GET /v1/tenders/7956308 → contracts[0].value = {cents: 5439360, currency: "GBP"}

54,393.60 GBP, the figure `contracts[0].value` publishes and the award it settles does not. It was
`null` for every FTS contract in the corpus.

**The contract-less award keeps its date.**

    GET /v1/notices/30805683/content → ('RES-1-1', 'BT-1451-LotResult', '2025-05-08T00:00:00+00:00')

Exactly the date this issue recorded as "absent".

**Canonical counts over tender ids 7954583–7975000:**

| | before (finding, 2026-09-14) | now |
| --- | ---: | ---: |
| fts contracts | 5,158 | 17,482 |
| … with `cents` | **0 (0 %)** | **16,553 (94.7 %)** |
| … with `decided_utc` | **0** | 1,009 |
| fts lot_results | 16,948 | 55,609 |
| … with `decided_utc` | **0** | **11,819 (21.3 %)** |

Read the DENOMINATORS with care: they roughly tripled, so this is not a like-for-like delta — the FTS
corpus grew between the finding and today (the source now holds 10 packages from 2025-06). The claim
that stands is the zero-to-nonzero transition in each "with" row, which no amount of corpus growth
produces on its own.

**One correction to this issue's own acceptance.** It asked for `/v1/tenders/7956308` to serve the
contract "with a value and a non-empty `amounts`". `amounts` is `null` on TED contracts too — checked
against `/v1/tenders/4`, whose contracts carry `value: {cents: 63895000, currency: "PLN"}` beside
`amounts: null`. So a non-empty `amounts` was never the shape of a contract in this API, for any
source, and it is the wrong thing to have asked for. The right test is the one this issue argues for
everywhere else — PARITY with a TED tender carrying the same facts — and FTS now matches it field for
field.

**The residue, named rather than rounded off:** 929 contracts still without a value and 43,790
lot_results still without a decision date. Sizing that against source absence (the contract-bearing
shapes genuinely publish no `awards[].date`, per the note in unit 2 above) is unit 2b's work, beside
the periods and the `BT-3202`/`OPT-315` linkage.

## Unit 1 BUILT 2026-09-18 — the register ocid splits per buyer; the refold waits for a go-ahead

**The rule, and where it lives (`2c2d0b0`; gate 124/124 green; DEPLOYED 2026-09-18 08:15 UTC at `a52390b` on an
idle queue — new FTS ingests group under the rule from the next daily tick, the standing rows wait for the refold below).** The `## Done when` offered two routes; the first is taken:
issue 369 unit 5's refused-key pre-filter is extended to `source = 'fts'`. In `build_plan_groups`
(`crates/store/src/canonical.rs`) a second arm fills `plan_refused_key` with every FTS `procedure_key`
whose notices carry **two or more distinct buyer SETS** (`COUNT(DISTINCT buyer_key) >= 2`), and the
existing `refused:` arm of the `group_key` election then splits the ocid per buyer — so the served
`procedure_key` of each part reads `refused:<ocid>:<buyer key>`, exactly as 369 unit 5 already serves
TED's placeholder keys. Nothing in the parser changed: `BT-04-notice` is still the ocid.

Why 2 and not the placeholder gate's 3: that `>= 3` absorbs the org-layer duplicate floor measured on
TED (two "buyers" that are one entity twice). `buyer_key` is parsed-side, and on FTS it is the
publisher's own stable party id (`GB-PPON-…`) through the resolver's normaliser, so two distinct sets
under one ocid are two buyers. A joint procurement lists its buyers in ONE release and repeats that
set, so it stays one Tender; a single-buyer chain of any length (7954590's 123 versions) is untouched.
The failure direction is CONTEXT.md's: a buyer that re-registers under a new id splits its own chain —
never a weld. The arm reads a partial index `plan_notice_fts_key … WHERE source = 'fts'`, so it is an
index read of the FTS rows rather than a walk of the 14M-row plan, and the index-free `group_key`
UPDATE never touches it. The projection logs `refused-keys (fts): N ocid(s)` every run, zero included.

**Tests, both directions.** Store: `an_fts_ocid_shared_by_two_buyers_splits_per_buyer_and_a_one_buyer_chain_stays_one`
— a register with Anglian, Scottish Hydro ×2 and SSE → three Tenders with Scottish Hydro's two releases
together; a buyer-less release of the split ocid stays an island; a five-release one-buyer chain keeps
its ocid; a joint procurement repeating one two-buyer set keeps its ocid; a TED key with two buyer sets
is untouched (the arm is FTS-only). End to end (`crates/ingest/tests/fts.rs`,
`releases_under_one_ocid_from_two_buyers_fold_to_two_tenders_and_a_one_buyer_chain_to_one`): six
synthetic UK6 releases through `process` and `project` — two buyers under one ocid become two Tenders,
each carrying exactly one buyer organization, and a one-buyer chain becomes one Tender with three
versions. Gate: full suite green.

**The weld gauge counts the FTS arm.** `weld_fts` in the weekly DQ report — FTS Tenders carrying ≥ 2
distinct buyer organizations, scoped through `tenders(source, id)` — with its own UNMEASURED line,
because the register weld (2–30 buyers) sits below the corpus-wide bands and a top-40 listing never
shows it. Expected reading after the refold: **0**; any non-zero is a recurrence ahead of the 2021→
backfill (or a joint procurement, which the listing's `per-ver` separates). JSON: `weld_candidates.fts_multi_buyer`.

**Departure recorded.** `342-fts-plan.md` §3b item 7: "one Tender per ocid" does not hold on the
utilities register, and what replaces it.

**Not done here, and why.** The buyers-differ SELECT over 7954583–7975000 (the reporter's 19 tenders /
55 versions) was not re-run: it is a data-page read over an unindexed `mention_section_id` grouping
and would 408; the gauge above is its standing form. The live acceptance (7954584 no longer serving
Scottish Hydro's and SSE's contracts; `?publication_id=033117-2025` and `031078-2025` resolving to two
tenders) needs the standing FTS rows re-grouped, which is a `refold` over profile `fts:ocds-1.1` —
**a production write the operating session's classifier refuses**, so it waits with the other gated
jobs. After the deploy, via `/root/aj.sh` on the box:

1. **size it**: `refold` with `profiles: ["fts:ocds-1.1"]` and `expect: 1` — it aborts with
   "N notices match, expected ~1 (nothing was written)"; N should be ~10,600 (job 2325's count);
2. **the wet run**: the same body with `expect: N`; then a `project` (or the daily tick's).

Then: `GET /v1/tenders?source=fts&publication_id=033117-2025` and `…=031078-2025` land on two different
tenders; `GET /v1/tenders/7954584` (or whichever id Anglian's part keeps) lists only Anglian's 11
contracts; the next weekly report's `weld_fts` reads 0. Record the before/after here.

## Unit 2b — the ADR-0004 checklist landed 2026-09-18 (`8b895e1`, gate 126/126, deployed 11:11 UTC on an idle queue); the periods and the linkage are now named debts

**What exists.** `crates/ingest/src/fts/checklist.rs` is the profile's disposition record for
`fts:ocds-1.1`: a `CHECKLIST` of every published OCDS path the parser has decided about, each
`Mapped(<the eForms field it becomes>)` or `Ignored(<why>)`, and `disposition(path)` answering by the
LONGEST entry that names the path or a container above it on a path boundary — `tender` covers
`tender.lotsGroup`, `tender.lots` does not, `id` does not cover `identifier`. `None` is the finding: a
path this profile has never decided about. The reasons are the parser's own (`fts::parse`'s emit map,
`342-fts-plan.md` §3b's deliberate refusals such as `documentType` not being keyed, plan D4's
amendment skeletons), so the checklist is a reading of the code, not a second opinion beside it.

**How it is pinned.** `every_published_fts_path_is_mapped_or_ignored_on_record`
(`crates/ingest/tests/fts.rs`) walks every release in `tests/fixtures/fts/{pages,members}` — the
recorded pages and the register's member releases — builds the census of key paths (arrays as `[]`),
and asserts every one has a disposition. On the day it landed: **224 distinct paths over 14
releases, all disposed, 35 of them `owed`**. It also asserts `contracts[].value` is `Mapped`, so the
exact hole this issue was filed on is the one path the test names by hand. A new key the register
starts publishing fails the suite the first time a fixture carries it. The unit test
`the_longest_entry_wins_and_containers_cover_their_subtrees` pins the boundary rule; its first
draft asserted `tender.lotsGroup` had NO disposition, which was the test's error, not the
function's — the `tender` container legitimately covers it — and it now asserts which container wins.

**What is `owed`, now written down instead of remembered.** Twenty-one entries carry a reason
starting `owed:` — content the register publishes, the profile drops, and the checklist says so:

- party address lines (`parties[].address.streetAddress/postalCode/locality`, BT-510/512/513) and
  `parties[].contactPoint` (BT-502/503/506) — the org resolver binds on identifier and name;
- `tender.lots[].hasOptions/options`, `awards[].hasOptions/options`, `tender.lots[].hasRenewal`,
  `contracts[].hasRenewal` (BT-58);
- `tender.procurementMethod`, `procurementMethodDetails` (BT-105), `mainProcurementCategory` on the
  tender and the award (BT-23), `procurementMethodRationale` and its classifications (BT-136);
- `tender.awardPeriod`; `tender.lots[].contractPeriod.maxExtentDate`;
- **`awards[].contractPeriod` and `contracts[].period`** — this unit's own schema decision, unchanged:
  `tender_version_contracts` has no duration columns and BT-536/537 are a LOT destination the lot's
  period already fills;
- `bids.statistics[].relatedLot` — STAT sections are Tender-scoped today.

Reclassifying one of these to `Mapped` is how the next unit records itself. `BT-3202-Contract` /
`OPT-315-LotResult` are not paths and so not entries: they are the linkage the profile does not emit,
still owed, still this unit.

**Not a served change.** Nothing in the parser or the fold moved; the profile's output is
byte-identical. The deploy (`/health` reads `8b895e1` at 11:11 UTC) carries it only so the box's tree
matches main. Doc pointer:
`docs/research/uk-fts.md` §4, last paragraph.

## Unit 2b — the results graph is linked both ways (2026-09-18)

`crates/ingest/src/fts/parse.rs` now emits the two references the plan's §3b table promised and
the checklist carried as debt:

- **`OPT-315-LotResult`** on every lot-result of an award: one `CON-<id>` ref per contract whose
  `awardID` is that award. The same refs on every lot-result of a multi-lot award, because OCDS links
  contracts to awards and never to lots.
- **`BT-3202-Contract`** on the contract: one `TEN-<award>-<n>` ref per supplier WITH an id, `n`
  being the awards loop's own enumerate index (gaps included) so the ids agree with the sections it
  minted. A delta award (`{id, amendments}`) minted nothing, so a contract pointing at it gets no
  reference — a dangling ref is worse than an absent one.

What the fold gains: `RawLotResult.contract_refs` and `RawContract.bid_refs` are the eForms paths it
already walks, so an FTS contract now reaches its bids (and its lot) from either end, and its value
is the bid-derived total wherever the award carries one — the contract's own published amount
(unit 2a's `CONTRACT_VALUE`) stays the fallback exactly as unit 2a designed it. Pinned by
`a_contract_and_its_award_reference_each_other_and_only_each_other`: 028961-2025 (award 1 ↔ contract
1, `RES-1-1` → `CON-1`, `CON-1` → `TEN-1-0`, and the tender exists), 083650-2026 (a UK6 award with no
contracts carries no OPT-315), the contract-amendment member (a contract whose award is not in the
release carries no BT-3202). The checklist's `contracts[].awardID` entry reads the linkage now.

**Standing rows.** A parse-layer change: the FTS notices on prod carry it only after the same
re-parse unit 2a already owes (the gated FTS reprocess at the foot of unit 2a's section, then a
refold); nothing on prod changes at the deploy. The periods landed the same evening — next section.

## Unit 2b — the periods landed 2026-09-18 (`846f856`, gate 127/127, deployed 18:10 UTC on an idle queue)

**The schema decision, decided: no schema change.** The "collision" unit 2a's note feared — a
contract's duration pushed under `BT-536/537-Lot` beside the lot's own — only exists when a lot
publishes a period AND its award publishes a different one. So the rule never overrides a lot's
own period, and the key still carries exactly one fact: the period that applies to that lot.
`tender_version_contracts` stays without duration columns; the lot's period lands where an eForms
`BT-536-Lot` lands, the fold's `DATES` table (`duration_start` / `duration_end` at the lot's scope).

What `crates/ingest/src/fts/parse.rs` does now:

- `Award.contract_period` and `Contract.period` are read (both were absent from the structs — unit
  2's repro step 4).
- `inherited_periods(awards, contracts)`: a lot that publishes no `contractPeriod` of its own takes
  the period of the one award that names EXACTLY that lot — the award's own `contractPeriod` first,
  else its contracts' `period` — and only when every candidate agrees. A multi-lot award's period is
  the award's, not any one lot's: nothing. Two single-lot awards on one lot that disagree, or two
  contracts of one award that disagree: nothing. A delta award (`{id, amendments}`) says nothing, as
  it does everywhere else in the walk.
- The lots loop consults that map only when the lot's own period is absent, and the quarantine
  detail names the source (`award contractPeriod.startDate`, `contract period.endDate`) when a date
  is unreadable.

Pinned by `a_lot_without_a_period_inherits_its_single_lot_awards_or_that_awards_contracts` —
083650-2026: lot `1` takes the award's 2026-09-18T00:00:00+01:00 → 2027-03-31T23:59:59+01:00 (a UK6
with no `contracts[]` at all); 028961-2025: lot `1` takes contract 1's 2025-05-19Z → 2028-05-18T23:59:59Z,
the award publishing none; 083563-2026: lot `1` keeps its own 2026-11-04 — and by
`the_inherited_period_is_narrow`, one synthetic release carrying every refusal plus the case where
an award and a contract agree. The checklist's `awards[].contractPeriod` and `contracts[].period`
entries read the mapping; their `maxExtentDate` leaves join `tender.lots[].contractPeriod.maxExtentDate`
as the one line still `owed:` (the maximum extension date has no destination — a schema question
this unit does not answer, and the census over every fixture release stays all-disposed).

**Not covered, on record.** An award naming two or more lots: its period reaches no lot. That is the
publisher's job, and the UK4 shape does it — 083563-2026's five lots each carry their own. How many
standing FTS lots are period-less today was not measured: they carry a period only after the gated
re-parse, so the number would describe the re-parse's backlog, not the rule.

**Standing rows.** A parse-layer change like the linkage: nothing on prod changes at the deploy
(7956308's `dates` read `[]` at 18:10 UTC, as before); new FTS ingests carry the periods from the
next daily tick; the standing FTS notices after the gated re-parse unit 2a's foot names, then the
refold. Owed read: an FTS tender ingested after the 2026-09-19 07:35 tick whose lot serves
`duration_start` in `dates`.
