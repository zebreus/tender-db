# 383 — `TED-DATE_OF_CONTRACT_AWARD`: a second legacy spelling of the award date that the results reader does not match

Status: ready-for-agent (filed 2026-09-12 by the owner, from issue 368's per-profile probe)
Kind: defect (data) — a MODELLED concept (the award decision date, issue 255) missing for one
spelling, which is exactly issue 368's shape
Blocked by: nothing

## What the probe showed

`GET /admin/unmapped-fields?profile=ted-export-r208` over the era's own head window (ids
27,061,440–27,161,439), with the per-channel sieve (`aa9c080`):

| field id | table | rows in window |
| --- | --- | --- |
| `TED-CONTRACT_AWARD_DATE` | notice_dates | 1,688 — read by `read_legacy_results`; was a predicate gap, fixed in 368 unit 2 |
| `TED-DATE_OF_CONTRACT_AWARD` | notice_dates | **79 — matched by nothing** |

`read_legacy_results` (`crates/ingest/src/project.rs`) matches `LEGACY_AWARD_DATE_FIELD` =
`TED-CONTRACT_AWARD_DATE` only. `grep -c DATE_OF_CONTRACT_AWARD crates/ingest/src/project.rs` → 0.
The r209 parser's element list carries the spelling (it is a real published element), so the parse
layer stores it and the projection drops it, and those awards carry no `decided` instant.

## What is not known yet

- Which form/era publishes this spelling (79 rows in a 100k-id window of r208's head; corpus size
  unknown — `refold-fields` with `expect: 1` sizes it in one sweep, ~46 min).
- Whether it sits on the award block the way `CONTRACT_AWARD_DATE` does (the reader keys on the
  section), or somewhere the LotResult synthesis does not look.

## Units

1. Read one carrier (bounded band + `notice_dates`, then the archive member) — which form, which
   section, same instant semantics as `CONTRACT_AWARD_DATE`?
2. If it is the same fact in another spelling: match it beside `LEGACY_AWARD_DATE_FIELD` in the
   reader AND the date-channel predicate (the same constant, so they cannot drift — issue 384 is
   the guard), fixture, `refold-fields` over it.
3. Re-run the probe: the row disappears from the r208 listing.
