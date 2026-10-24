# 583 — an organization merge leaves party rows in an order the next refold rewrites (579's merge-side twin)

Status: ready-for-agent — filed 2026-10-24 as issue 579's U6. Next: unit 1, measure whether merges leave Tenders whose next
refold rewrites a party table with no fact change, and how many.
Kind: data quality / rebuild parity (spurious corrections; no wrong value is served)
Relates to:
- 579 (the dissolve path; fixed by U4 1fb7541, which adds every touched Tender to `refold`);
- 495 (the compare engine compares rowid-ordered rows by position);
- 530 (dissolve), 469/514/513/519 (the merge jobs).

## Verify

    grep -c 'issue 583' crates/store/src/canonical.rs

- **done**: `1` or more. The merge paths add every touched Tender to the trailing refold (or write rows in fold order),
  under a test that a merge followed by a refold rewrites 0 tables.
- **open (2026-10-24)**: `0`.

## What was found

- `repoint_org_references` (crates/store/src/canonical.rs ~7466) moves a loser's references to the survivor in place:
  - `UPDATE organization_mentions`, `UPDATE tender_version_parties` and `UPDATE tender_version_bid_parties` SET
    organization_id;
  - for result winners, a DELETE of rows that would duplicate the survivor, then an UPDATE.
- A fresh projection writes each group sorted by org id, while the compare engine compares rowid-ordered rows by position
  (579 unit 1). An updated row keeps its rowid, which sits where the old org id sorted. So when the survivor's id sorts
  differently from the loser's, the next refold of that Tender sees the table "differ", rewrites it and announces a
  `version: null` correction with no fact change. This is the same mechanism 579 measured for `dissolve_org_mentions`
  (9 of 530's 2,742 touched Tenders).
- Its callers are all the merge paths:
  - `merge_provisional_organizations_batch` (~25058);
  - `match_org_identifiers_r2` (~26238);
  - `match_org_null_country_r3` (~28970);
  - `nhs_body_fold` (~32122);
  - `altid_merge_steps` (~33342);
  - `match_org_rekey` (~34851);
  - `fold_provisional_plan` (~38054);
  - `repair_nested_losers` (~40802).

  A grep near each call finds no Tender stale-stamp or refold. That is not yet confirmed per path: unit 1 checks each
  caller's tail.
- Served effect: the head `parties[]` order of an affected Tender (and `bids[].parties`), and possibly v_awards'
  representative buyer (579 U3: 179248 flipped at a refold). Values, row sets and org ids are already right.

## Proposed units

1. **Measure (S).**
   - Read each caller's tail for a stale-stamp or refold.
   - Take one recent merge job's touched Tenders (from its change rows or its log) and dry-compare them: how many would a
     refold rewrite with 0 fact change?
   - Decide between "stamp touched Tenders stale" (579 U4's remedy) and "write in fold order".
2. **Build (S–M)**, the remedy shared by every caller (inside `repoint_org_references`, or in a wrapper that records
   touched Tenders), plus a merge-then-refold-rewrites-0-tables test per caller family.
3. **Drain.** A refold of the measured cohort, if any, is an in-place W-input-adjacent writer under issue 551 (org merges
   feed R1/R6). Run it after 11-04 under C9', or as a reviewed segment.
