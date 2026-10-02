## Files
All files are in `.scratch/tender-db/481-dedup/ (data/, scripts/; working copy was the session scratchpad)`:
- `positives.json`
- `negatives.json`
- `unmerged.json`

Each file has a `meta` block with:
- the period and the census definition;
- `requests`: the total, and for each read shape the request count, row count, max wall time and one example SQL;
- `gaps` (13 entries);
- `signal_notes`, which says where each signal comes from;
- `quick_stats`.

Supporting files in the same folder:
- `reqlog2.jsonl`: the exact SQL of every request, with wall time, row count and timestamp.
- `r2/NNN-<label>.json`: the raw response bodies.
- `cache/`: chunks keyed by an SQL hash, so a rerun sends no requests.
- Scripts: `sq2.py` (the logged helper; it stops on any body without `rows` and never retries), `reads.py`, `sat.py`, `satseq.py`, `nl.py`, `census_lib.py`, `stage1`–`stage11*.py`, and `build.py`, which assembles the files offline.
- Intermediates: `census_rows.jsonl`, `notices_doe.json`, `legal_doe.json`, `orgs*.json`, `rev_lists.json`, `cands_sel.json`, `sats.json`, `nl.json`, `de1_sections.json`.

The earlier run's 25 requests stay separately in `reqlog.jsonl`.

## Requests: 251
They ran between 01:40 and 02:38 UTC, one at a time, at least 12 s apart (about 260 per hour). There were 0 error bodies, 0 truncations and no 408.

| Stage | Requests | Shape |
|---|---|---|
| Census | 50 | `SELECT tender_id,seq,caused_by_notice_id,publication_id,published_at,notice_subtype FROM tender_versions WHERE published_at >= A AND published_at < B AND <publication_id filter>`. April: one request per day, filter `+publication_id NOT GLOB` 6-digit-year (drops FTS). Outer windows: TED-shaped 8-digit-year only, 2–3 days per request. |
| DÖE notices | 5 | `SELECT id,source,profile,publication_id,published_at,dispatched_at FROM notices WHERE id BETWEEN a AND b` over 26,614,793–26,646,999, plus one request of PK arms for the outliers. |
| Field discovery | 1 | Per notice, distinct field ids per table, with `GROUP BY +field_id` / `+kind` so the GROUP BY cannot pick an index. |
| Legal basis | 5 | `notice_codes WHERE notice_id BETWEEN … AND (+field_id='BT-01-notice' OR +field_id='SDK01-RegulatoryDomain')`. |
| Tenders | 3 sample + 8 candidates | PK arms `WHERE id=X`, UNION ALL. |
| Buyers | 4 sample + 13 candidates | `tender_version_parties WHERE tender_id=X [AND seq=Y] AND role LIKE '%uyer%'`. |
| Organizations | 5 | PK arms, and arms of `name_norm='…'`. |
| Reverse buyer lookups | 28 | Count: `SELECT X AS o,count(DISTINCT tender_id) … WHERE organization_id=X AND role='Procedure-Buyer' AND (tender_id<=1200000 OR tender_id>=7930000)`. List: `SELECT DISTINCT X AS o,tender_id …`. The tender-id range keeps only the blocks that hold TED Tenders in the window. Served from the `org_role` index alone. |
| Satellites per (tender, seq) | 86 | Title `lot_id IS NULL`; CPV `+scheme='cpv' AND field='main' AND lot_id IS NULL`; NUTS `+scheme='nuts' AND field='place'`; deadline as an aggregate; `estimated_value` rows; lot count. |
| Notice layer | 38 | Arms by `notice_id=X` with `+field_id` filters on texts, classifications, dates, amounts, sections, ids and codes. |
| TED notice check | 2 | `notices.source` and profile for the 458 positive TED notices plus 300 random candidate notices. |
| DE1 re-read | 3 | eForms-DE 1.x leaves read again with `section_id`. |

## Census
- 182,530 Tenders in the window.
- 27,660 DÖE versions in April 2025, all with `notices.source = doe`. Profiles: eForms-DE 1.1 2,460, 1.2 4,981, 2.0 9,608, 2.1 173; sdk-0.1 numeric 13,762, sdk-0.1 uuid 1,268; EU SDK 94.
- 26,169 Tenders have an April DÖE version:
  - **Merged: 12,513.** All eForms-DE: 1.1 1,934, 1.2 3,773, 2.0 6,758, 2.1 48. That is 98.9% of eForms-DE.
  - **No TED version in the window: 13,656.** sdk-0.1 numeric 12,763, sdk-0.1 uuid 702, eForms-DE 132, EU SDK 59.
- DÖE-only sdk-0.1 numeric Tenders with an EU legal basis (32014L0024): 330. The uuid channel has none.

## positives.json (458)
**Sample.** Stratified random, seed 4810: eForms-DE 1.1 120, 1.2 120, 2.0 170, 2.1 48.

**Pairing.** The DÖE side is the first April DÖE version. The TED side is the TED version of the same subtype nearest in time; 454 of the pairs are within 7 days.

**Per side, each record holds:**
- `canonical`: `tender_version_*` values at that version, flagged `carried_state_possible`.
- `notice`: that notice's own values from the notice layer.
- `ids`: internal reference (BT-22-Procedure, DE1-/SDK01-ProcurementProject-ID), BT-04 / ContractFolderID, and OPP-090.
- Buyers taken from party rows whose mention is that notice, with org name, `name_norm` and identifier.
- Legal basis, profile and dates.

**Agreement of the two notices' own values:**

| Signal | Agree | Differ | Missing on one or both |
|---|---|---|---|
| Title (exact, case-insensitive) | 454 | 4 | 0 |
| CPV main | 451 | 7 | 0 |
| CPV division | 457 | 1 | 0 |
| Buyer org id | 457 | 1 | 0 |
| BT-04 | 456 | 2 | 0 |
| Internal reference | 407 | 3 | 48 |
| NUTS | 446 | 1 | 11 |
| Deadline | 206 | 4 | 248 |
| Procedure-level value | 67 | 3 | 388 |
| Lot count | 458 | 0 | 0 |
| Legal basis | 458 | 0 | 0 |

The pair's true TED twin turns up among same-buyer TED Tenders within ±30 days in 457 of 458 cases. The other buyer side shows the same organisation under two org ids.

## negatives.json (1,289)
Every pair is two different Tenders that share a buyer org id and were published within ±30 days. The list is sorted with same-CPV-division pairs first; each pair carries `same_subtype`, `internal_ref_equal` and `title_equal_ci` flags.

- **`twin_known`: 1,028 pairs.** The DÖE notice belongs to a UUID-merged Tender whose TED twin is known; it is paired with up to 3 other TED Tenders of the same buyer org. The DÖE side is therefore not DÖE-only (see Gaps). 524 share a CPV division, 909 share a subtype, 16 have identical titles and 17 share an internal reference.
- **`doe_only_key_disjoint`: 261 pairs.** A UUID-keyed DÖE-only Tender against a TED Tender with a different UUID. 187 share a CPV division.

## unmerged.json (817)
**Sample.** Every DÖE-only Tender in April that has an EU legal basis or comes from the eForms family, plus seeded random national sdk-0.1: 200 numeric and 100 uuid. Four sampled Tenders turned out to be merged (TED version after 2025-05-30) and were dropped: 448921, 925462, 1067200, 1144042.

**Composition.**

| Profile | Legal basis | Count |
|---|---|---|
| eForms-DE 1.1 | EU | 29 |
| eForms-DE 1.2 | EU | 25 |
| eForms-DE 1.2 | national | 1 |
| eForms-DE 2.0 | EU | 73 |
| sdk-0.1 numeric | EU | 330 |
| sdk-0.1 numeric | national | 200 |
| sdk-0.1 uuid | national | 100 |
| EU SDK 1.0 (national E-forms) | national | 50 |
| EU SDK 1.12 / 1.13 (transport prior-information notices, 32007R1370) | EU | 8 / 1 |

**Candidates.** 10,012 TED candidates in total, 6,038 of them with signals. A candidate is a TED Tender within ±30 days that shares either the buyer org id (`via=org_id`) or the buyer's `name_norm` (`via=name_norm`). Each candidate records its time gap, subtype, `census_sources` (whether the TED Tender already has a DÖE twin) and `title_equal_ci`; candidates with signals also have their satellites, ids and buyers.

**Results.**
- **eForms family: 136 of 137** have a same-org, identical-title, same-subtype TED Tender, usually one day apart. Both sides are islands, mostly prior-information notices without BT-04.
- **sdk-0.1:**
  - Org-id candidates: 0 for numeric islands; 19 of 100 uuid records.
  - Name-based candidates: 342 of 630 records.
  - Identical titles: 0. Word-overlap ≥ 0.5: 8 records, of which 1 looks like a true twin and the rest are same-buyer sibling procedures.

## Gaps
These are recorded in each file's `meta.gaps`:
1. **No sdk-0.1 positives.** Calibrating that band needs hand labels.
2. **The negatives' DÖE side is not DÖE-only.** DÖE-only Tenders that share an org id with TED are the eForms-family ones, and those are mostly real twins. So the certain negatives use the DÖE notice of merged Tenders, plus the 261 DÖE-only keyed pairs.
3. **±30-day window,** the census's reach. This is inside the 60 days asked for negatives.
4. **Canonical values carry forward.** `tender_version_*` values are cumulative, so a later version can carry an earlier version's facts. Positives therefore also carry each notice's own values. Unmerged records, candidates and negatives carry canonical values plus notice-layer ids only.
5. **Candidate signal caps.** Positives keep their 3 nearest competitors with signals. Non-sdk-0.1 unmerged records keep their 15 nearest. Every other candidate is listed with ids and dates only (`has_signals=false`).
6. **eForms-DE 1.x lot fields share field ids with procedure-level fields.** These were re-read with `section_id` and scoped to the procedure. NUTS scope cannot be separated (`nuts.unscoped`).
7. **Deadlines and values are often missing,** mostly on award and prior-information notices.
8. **Organisation fragmentation.** Name families reach 1,806 orgs (Immobilien Bremen). All 5,199 sibling orgs were looked up.
9. **Index presence was not checked directly.** `/v1/sql` exposes neither `sqlite_master` nor EXPLAIN, so index use is inferred from timings. The maximum was 3.83 s wall, about 2.1 s on the server.