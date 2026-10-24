# 580 — a Tender id retired on regroup answers 404, and no served doc says a Tender id can go away

Status: ready-for-agent — TRIAGED and DECIDED 2026-10-24 (wf_fcc4cb46-314: a sizer, a designer and an adversarial skeptic, CORRECTED; `.scratch/tender-db/580-triage/`). **D580-1: option (a), served docs only**, using the skeptic-corrected sentences; the successor pointer (b′) is split off as issue 581 (store write path, after 551 U9). Size: about 12 Tender ids retired per daily chain (11/14/12/13/11 over 10-19 → 10-23), 0 on refolds that move no grouping input (2.33M re-folded), and 93 on 539 run c; hundreds in a normal month, 10^4–10^5 in a regrouping campaign (issue 93: 216,450). Ids are never reissued within a generation (AUTOINCREMENT). NEXT: unit 2 (a) build: docs.rs Tender detail and ids/generation paragraphs, the version-null list, openapi generation / getTender 404 / ChangeEvent.version, CHANGELOG, and an assert_says test. These are 551 flip files, so the change gates with a twin re-gate and deploys with the 10-28 batch. Was: needs-triage — filed 2026-10-24 from the hourly AUDIT (issue 539's chain-order audit, wf_41e00ab3-eb4: its skeptic
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

## Triage (2026-10-24, wf_fcc4cb46-314)

**Sizing** (`580-triage/sizing.md`, `per-archive-counts.json`). The local change archives cover
(958787061, 959223268] without a gap: 436,207 rows, 10-19 → 10-24. The skeptic recounted them exactly.
- Daily chains (projects 2645, 2669, 2688, 2716, 2726) retired 11, 14, 12, 13 and 11 Tenders and minted 2,021–2,442
  each: about 231 per 100k written, 0.56 % of the ids minted that day.
- Every refold that changed no grouping input retired none (2.33M re-folded). That zero is structural, not a rate.
- 539 run c retired 93 and minted 93 (8,860,949–8,861,041); the change rows cannot show the pairing.
- None of the 154 retired ids ever reappeared, and none was retired twice.
- 8596072 (10-21 chain) took 9,807 bid rows with it.
- No project summary line counts retirements.

**Design** (`580-triage/design.md`; skeptic corrections in `skeptic.md`).
- `retire_chunk_tx` is given ids only, and no caller knows the successor. The full path retires after Phase 2; the
  daily path retires before it, when a new successor has no id yet.
- Splits, merges, merge-then-undo and chains all occur.
- The follow path that works today: keep `versions[].publication_id` and ask `/v1/tenders?publication_id=…&source=…`.
  On /v1/sql: `SELECT tender_id FROM tender_versions WHERE caused_by_notice_id = <notice id>` (no bind parameters;
  `v_tender_notices` refuses filters).
- `/v1/notices?tender=<retired>` answers 404, not an empty page.
- On the daily path the feed sends `removed` before the absorbing Tender's `added`/`changed`, and the full path sends
  it after, so the docs must say "in either order".

**D580-1 (owner, 2026-10-24): (a), docs only.** It closes Verify `done` at size S with no store change and no W input.
- (b′), a notice-anchored ledger with 308/410, is size M and touches crates/store, so it needs B4's probe and waits
  until after 11-04. It cannot backfill: 8811451 would answer 410 with `successors: null`, never 308.
- (c), `tender_id` on notices, leaves the bare 404.
- Filed (b′) as **581** with the skeptic's corrections:
  - D1: twin repair deletes the notices, so anchor on the kept twin or skip it.
  - D2: guard on `sqlite_sequence`, not `MAX(id)`.
  - D3: in (c), `MAX(tender_id)` picks the older Tender when notices merge into one.
  - F3: `Cache-Control: no-store` on 308/410.

**Unit 2 text rules (skeptic F1–F6).**
- Say "in either order".
- Say "never issued again within a generation".
- Say "retired by a fold or a repair".
- Allow for a retired Tender whose notices went nowhere (held).
- Never write "retired twice".
- Write the /v1/sql example with a literal id.
- Put "Tender id" and "retired" on one /docs source line (Verify greps by line).

