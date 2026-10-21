# 579 — a refold of a Tender issue 530's wet drain touched rewrites head tables the drain left in place

Status: needs-triage — filed 2026-10-21 from issue 551 B6's L_changed run lc01 (project 2698). Next: unit 1, name the
leaf table(s) and columns that differ between 530's in-place rehome and a fresh projection, size it over 530's touched
set (2,742 Tenders, `551-lot-sum/b6/u6/l_changed.json` source S7), and decide between refolding that set and fixing the
wet path.
Kind: data quality (organization identity / rebuild parity): the stored state after a stock drain is not the fixed point
a refold reaches; medium severity until sized (no value moved on the three seen; what moved is head tables, likely party or
winner rows)
Relates to:
- 530 (DONE 2026-10-21): wet `dissolve-guid-twins` 2672 + 2673 (10,995 GUID orgs dissolved, 22,056 mentions moved, 52 minted;
  change cursor 958904203 → 958919213; **0 Tenders stamped stale**, project 2674 projected 0 notices). Its Build 2 review
  asked for rebuild parity ("a from-archive rebuild under the gate produces exactly the org layer the wet path leaves").
- 578: the same family — `repair-placeholder-orgs` (440) lands a mention where a rebuild would not; 530 Build 2 factored
  440's body into the shared `Db::dissolve_org_mentions`.
- 551 B6 (path W flip): this surfaced as 3 non-W corrections in its L_changed run lc01; it changes no 551 verdict (no value
  moved), and any refold over 530's touched set must respect 551's standing rule (port-diff re-run before a W-input drain
  while W is on: org merges feed R1/R6).

## What was seen (2026-10-21, issue 551 B6 lc01)

- `refold-lot-sum` lc01 (dry 2696 → wet 2697 → project 2698) refolded 489 L_changed Tenders under the flip rev e718684:
  481 verified, **8 corrected; tables 85,144 skipped / 8 rewritten; rows 72,768,313 skipped / 376 rewritten; 76
  correction rows** (lot 68, tender 8). Five corrections are path-W fires (head value columns only — those rewrite no
  table: stage 2's 3,100 corrections rewrote 0 tables).
- The other three — **497154, 675459, 780251** (TED; heads 2026-06-16 / 2025-07-22 / 2024-10-31; max-elected, no
  derivation) — moved **no** served tender column the PK snapshot holds (value, currency, EUR, derivation, seq,
  published_at, projection_epoch all identical before and after), yet the compare engine rewrote 8 head tables / 376 rows
  among them, so its rule L announced every head lot (34 / 6 / 8 lot `changed` rows) and one `tender changed` each.
- Their only change rows between cursor 958787061 and the run are 530's wet drain: `tender changed` at 01:29Z (497154),
  01:30Z (675459), 01:30Z ×2 and 01:32Z (780251). 530 rehomed their mentions in place and did not refold them.
- So the in-place rehome (parties / bid parties by `(mention_notice_id, mention_section_id)`, winners by the 309 origin
  tier, per 530's A2) leaves some head table different from what a fresh projection writes for the same notices.

## Open questions for unit 1

1. Which leaf tables and columns differ (the project's per-leaf tallies, `tables_rewritten_by_leaf`, are not in the job
   summary; a dry compare of one touched Tender or a fixture run of the wet path followed by a refold will show it).
2. Is the stored (pre-refold) state wrong as served — e.g. a party row on the GUID org id 530 deleted, a winner on the
   wrong target, a duplicate party row — or only a representation difference the compare engine counts (row order,
   rowids, a denormalised name)?
3. How many of 530's 2,742 touched Tenders carry it (lc01 refolded only those inside the lot-sum cohort: 3 of them)?
4. Fix: refold 530's touched set by `tender_ids` (stamps + project; after 551's port-diff re-run if W is on), and/or make
   the wet path write exactly what the projection writes (or stamp its touched Tenders stale so the project reconciles).
