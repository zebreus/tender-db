# 581 — a retired Tender id could point to where its notices went (308 / 410 with successors)

Status: ready-for-agent — filed 2026-10-24, split from 580's triage (wf_fcc4cb46-314; option (b′) in
`.scratch/tender-db/580-triage/design.md`, corrected by `skeptic.md`). Blocked by: 551 U9 ending 11-04, because this adds a
store write path inside retirement (B4 probe, C9'), and by 580 unit 2 (the docs) landing first. Next: build after 11-04.
Kind: API contract (follow-up to 580: docs only today, a pointer here)
Relates to: 580, 93, 386, 455 (the organization 308 / merged_into precedent, docs.rs:140-150).

## Verify

    curl -s -o /dev/null -w '%{http_code}\n' https://tenders.zebreus.click/v1/tenders/8811451

- **open (2026-10-24)**: `404`.
- **done**: `410`, with `"successors": null` (it was retired before the ledger existed, so it cannot be backfilled).
  In addition, an id retired after the deploy answers `308` (one successor) or `410` with its successor list, as read
  in unit 3.

## Proposal (from 580's triage; the skeptic's corrections applied)

- **Ledger of notices, not successor ids.** `retired_tenders(retired_id, notice_id, retired_at)` plus an index on
  `retired_id`.
  - No UNIQUE constraint. Not in CONTENT_TABLES. Not named `tender_version_*`, because the golden tests count those.
    Off the /v1/sql allow-list.
  - Written inside `retire_chunk_tx`, in the same transaction, from the Tender's own `tender_versions` (one statement
    per 512 ids).
  - Emptied by `reset_tender_layer` / `clear_canonical`.
  - **Twin repair (D1):** its versions' notices are deleted, so anchor on the kept twin's notice
    (`tender_of(repair.keep)`), or leave twin repair out.
- **Read (`resolve_tender`).** Successors are the distinct live Tenders that hold the ledger's notices (cap 101). No
  ledger rows but a `tender removed` change row, with the id ≤ `sqlite_sequence` (D2), means retired before the ledger.
- **API.**
  - One successor: 308 with `Location` (keeping `?lang=`) and `"successors": [s]`.
  - Several or none: 410 with `successors`; none means `[]`, which also covers held notices and twin repair.
  - Retired before the ledger: 410 with `null`.
  - Never issued: 404.
  - `Cache-Control: no-store` on 308/410 (F3: a merge-then-undo moves the successor).
- **Feed.** No change: the change log stays the byte-identity surface.
- **Tests.**
  - project_incremental.rs ~2403 (merge, then undo) and ~3771 (a split).
  - A ledger digest in the chunk-invariance test.
  - Store resets.
  - api.rs beside the organization 308 tests.
  - Docs.
- **551.** canonical.rs is a flip file (twin rebuild, C7), and the change touches crates/store (B4 W-on probe). Not
  a W input.
