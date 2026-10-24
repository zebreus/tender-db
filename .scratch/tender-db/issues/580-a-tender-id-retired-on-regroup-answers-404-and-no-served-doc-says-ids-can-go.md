# 580 — a Tender id retired on regroup answers 404, and no served doc says a Tender id can go away

Status: needs-triage — filed 2026-10-24 from the hourly AUDIT (issue 539's chain-order audit, wf_41e00ab3-eb4: its skeptic
found run c had minted new Tenders). The served-docs gap was verified by a finder and an independent refuter (wf_bc2614fa-460):
both answered gap_real = true, and the refuter could not refute it. Next: unit 1, decide the remedy (docs only, or docs plus a
successor pointer) and size how often a regroup retires an id.
Kind: API contract / docs (an omission that misleads; no wrong data is served)
Relates to:
- 93 (retire_regrouped_tenders, the mechanism),
- 386 (its line 612: "Tender ids are not stable across a regroup ... a consumer holding 7954583 must follow the feed"),
- ADR-0011:78-79 ("Tender ids are retired on merge", unserved),
- 539 (run c retired 93 and minted 93),
- the organization 308 / `merged_into` contract (docs.rs:140-150), which Tenders lack.

## Verify

    curl -s -o /dev/null -w '%{http_code}\n' https://tenders.zebreus.click/v1/tenders/8811451; curl -s https://tenders.zebreus.click/docs | grep -c -i 'tender id.*retired\|retired.*tender id'

- **open (2026-10-24)**: `404`, then `0`. The id answers a bare 404, and /docs never says a Tender id can be retired.
- **done**: the second number is ≥ 1: /docs and OpenAPI say a Tender id can be retired within a generation, and how a
  client follows its notices. If unit 1 also adds a successor pointer, the first line answers `308` with the successor in
  `Location`.

## What was found

- **The behaviour.** When a fold regroups notices (a link added or refused, a re-dated chain re-linking, a weld split), it
  can retire a Tender and mint a new one. `retire_chunk_tx` (crates/store/src/canonical.rs ~43320-43354) writes `tender
  removed` (version null) plus `removed` rows for its lots, lot results, bids and contracts, then deletes the rows. The
  absorbing Tender emits `tender added` (new) or `tender changed` (existing). Nothing links the old id to the new one.
- **Example.** Issue 539's refold run c (2026-10-24, project 2741) retired 93 Tenders, e.g. 8811451, 8680277 and
  8818210, and created 93 new ones (8,860,949–8,861,041). `GET /v1/tenders/8811451` answers `404 {"error":{"message":"no
  such tender","status":404}}`. Issue 93 recorded 216,450 retirements in one 2026-08-02 refold.
- **The served docs suggest the opposite.**
  - docs.rs:458-460: "entity ids are scoped to the feed's `generation` and are reissued by a rebuild".
  - docs.rs:501-504: generation "moves only when the dataset is rebuilt from scratch ... entity ids are reissued".
  - openapi.json:65: "Feed generation — moves only on an operator-initiated rebuild".

  A careful client concludes that Tender ids are stable within a generation. A regroup breaks that without moving the
  generation.
- **No served surface says otherwise.** /docs, OpenAPI, the /v1/sql schema notes and CHANGELOG were all searched. The
  only statements of the real behaviour are unserved: ADR-0011:78-79 and issue 386:612-613.
- **The version-null list is incomplete.** docs.rs:569-571 and openapi.json:1155 list organization events, a lot that
  left its Tender, and in-place `changed` rows, but not a retired Tender's own `tender removed`.
- **How a client can follow today: only partly, and only with data saved before the id vanished.**
  - `/v1/notices/{id}` has no `tender_id` (json.rs:186-200; `NoticeRow`, read.rs:554-572).
  - `/v1/notices?tender=<old>` is empty after the retirement.
  - The working path: keep the old Tender's `versions[].caused_by_notice_id` or publication numbers, then call
    `/v1/tenders?publication_id=…&source=…` (read.rs ~2050-2097), or run `SELECT tender_id FROM tender_versions WHERE
    caused_by_notice_id = ?` on /v1/sql. No doc describes this path.
- **Contrast.** A merged-away organization id keeps working: 308 with `Location` and `"merged_into"` (docs.rs:140-150,
  openapi.json:239-250).

## Proposed units

1. **Decide and size (S).** Options:
   - (a) Docs only: a Tenders paragraph beside the organization one, saying a Tender id can be retired within a
     generation when its notices regroup; the feed's `tender removed` (version null) then the absorbing Tender's
     `added` or `changed`; the publication-number follow path. Also fix the version-null list.
   - (b) (a) plus a successor pointer: a `retired_tenders(old_id → new_id(s), retired_at)` ledger written in
     `retire_chunk_tx`, and `GET /v1/tenders/{old}` answering 308 (one successor) or 410 with `successors` (a split).
   - (c) (a) plus `tender_id` on `/v1/notices/{id}`.

   Sizing: read the retirement rate from the `tender removed` change rows over a recent window (a bounded keyset read
   over `changes`, plan-probed), split into daily folds and campaign refolds.
2. **Build (S for (a), M for (b) or (c))**, with a docs test and, for (b), the ledger written in the same transaction
   as the retirement.
3. **Verify.**

**551 note.** Docs (a) touch docs.rs and openapi.json, both flip files, so they deploy with a twin re-gate (C7). (b)
adds a store write path inside retirement and needs B4's W-on probe. Neither moves a W input.
