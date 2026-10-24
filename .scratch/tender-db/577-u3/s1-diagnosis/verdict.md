# 577 U3 slice 1: verdict

Concluded 2026-10-24 ~20:30Z from `code.md`, `data.md` and `skeptic.md` in this directory, with a spot re-read of
`code-r1.out`, `raw/s2_parties.sql` and `skeptic-r1.sql`. No box reads, GETs, archive extracts, cargo or git writes by
this step. The workflow's `/v1/sql` total stays at **7 of 8**.

## Root cause

The reparse job caused it, not 577 U2.

- `Db::reparse_notice` calls `clear_parsed` (crates/store/src/lib.rs:2667, then :2480-2488 at 6cb4408). For every
  re-parsed notice it runs `DELETE FROM tender_version_parties WHERE mention_notice_id = ?`, and the same for
  `tender_version_bid_parties`. These deletes ignore the issue-248 `keep` set.
- Job 2773 therefore removed every party and bid-party row citing one of the 218,876 de-1.x notices.
- Project 2774's issue-495 compare is positional: it checks the row count, then each value in rowid order
  (canonical.rs:24779, :1716-1719). It found those tables short, so it rewrote them and announced each one.
- The code has been there since 84feb9f, so it is not in the 512b297..6cb4408 diff.
- 539 run b was a refold, and a refold never calls `clear_parsed`. That is why it corrected only 2,211.

All three lines agree. The skeptic tried to refute it and could not:

- **Test A** (tenders 1–150): a table was rewritten exactly when it held a de-1.x-cited row. Parties: 29 of 29
  rewritten, 0 of 311 others. Bid parties: 26 of 26 rewritten, 0 of 107 others.
- **The other 13 tables were 100% identical.** `lot_results`, `bids` and `contracts` reference the notice without
  going through the FK, so the clear leaves them alone.

## Class

None of the three offered classes fits.

- It is not an order-only NOTE class, because the compare saw missing rows, not reordered ones.
- It is neither a content fix nor a content regression.
- Call it **class R: the reparse's own delete, followed by the fold restoring the rows.**

The expected change in served party content is **0**. This is inferred, not measured, because no copy of the rows from
before 2773 exists. The inference rests on four points:

1. **Row counts are conserved** wherever rowid gaps isolate a table: 25 of 25 party tables and 6 of 6 bid-party tables.
2. **577 U2 only relabels on de-1.x.** It moves 7 role-reference ids from the procedure-level DE1 id to the lot-level one.
   Each pair maps to the same `OPT-301-Lot-*` role, and the scope still comes from the section. No section moved, and
   no organization or bid-party field moved.
3. **The archive XML matches the restored rows** one to one: 26213590 has 8/8, 26198825 has 11/11.
4. **The mentions and organization ids are the old ones.** Their rowids are about 39.2M.

What remains unmeasured:

- An equal-count substitution (same number of rows, different role or organization).
- Party-path parser changes older than 512b297. A reparse applies every change since each notice was last parsed.

## Size

- **False corrections:** all 126,812 Tenders reported corrected (0 verified).
- **Tables and rows rewritten:** 406,659 version tables (220,649 party, 186,010 bid-party) and 4,905,308 rows.
- **Correction rows:** 326,839 (tender 126,812, lot 200,027), all feed noise.
- **Served gap:** the same 126,812 Tenders served without those party and bid-party rows for about 60 minutes, between
  2773 and the end of 2774. That gap is now closed.

## Slices 2 and 3

**Yes, both will do the same.**

- **Slice 2 (2776, then 2777):**
  - Project 2777 will correct about every stale Tender that has a party row citing a de-2.x notice.
  - Only the two party tables will differ.
  - The served window is open now. At 20:21Z, 12 of 32 de-2.x versions in tenders 1–150 served 0 party rows. Tender
    28's DÖE bid still serves `"parties": []`.
- **Slice 3 (2778, then 2779):**
  - It repeats both effects for the first 20 sdk packages.
  - 577 U2 moves no party field on sdk.
  - Its other 13 tables stay readable. That is where the probe's real signal is: the sdk moves BT-23, BT-531 and
    BT-772→BT-67(b), plus the 4 EXTRA overwrites. A correction there is a genuine 577 effect and must be judged
    against the 577 bar.
- **The 0-corrected bar cannot be met by any reparse slice until issue 591 ships.**

## Recommendation: B. Let the slices run, and land the fix

1. **Do not cancel 2777.** It does not apply a change. It restores the rows 2776 has already deleted. Cancelling it
   (option C) would turn a window of about an hour into a served gap with no end, for every slice-2 Tender, until some
   later refold. Option C is strictly worse.
2. **Slice 2 passes if both checks hold:**
   - 2777's identical/compared line shows 100% for every table except `tender_version_parties` and
     `tender_version_bid_parties`.
   - The last `/v1/sql` statement re-reads tender 28 in the `raw/s2_parties.sql` + `raw/s3_bid_parties.sql` shape and
     returns the 19:57Z before-image: the same values in the same order, with new rowids.
3. **Slice 3: let it run.** The velocity ruling says a known issue gets a banner and a fix, not a freeze.
   - Its party tables are unreadable, but they carry no 577 move. The probe's purpose is untouched.
   - If the last statement runs after 2779, extend it to tenders 2, 26 and 37 as well. `code-r1.out` and `raw/s2`/`s3`
     already hold their sdk-1.7/1.10/1.12 rows, so this gives a free before/after for any of those notices inside the
     20 packages.
   - The sdk rounds after the probe pick up 591 when it deploys. They do not wait for it.
4. **Banner, while any reparse slice runs.** It should say:
   - Tenders touched by the running reparse serve without party and bid-party rows citing its notices until the paired
     project ends, about an hour.
   - That project's party "corrections" are a restore, not a change (issue 591).
5. **The fix, issue 591 (already filed, `ready-for-agent`):**
   - In `clear_parsed`, delete party and bid-party rows only by `(mention_notice_id, mention_section_id)`, for the
     sections not in `keep`. That is exactly the set the FK forces, and the `_mention_key` index supports it.
   - Correct the stale comment at :2476.
   - Add a store test: re-parse a folded notice, and the compare stays neutral.
   - **Add the skeptic's sibling case to 591:** the issue-404 re-key path in daily `process` (lib.rs:1898-1969, which
     calls `clear_parsed` at :1946).
     - That path never stamps the Tender stale, so today its party rows are lost for good and its `publication_id` stays
       stale.
     - Narrow the delete there too, stamp the Tender stale, and test a re-keyed notice.

## Board inconsistency to reconcile

At 20:14Z, the files `issues/577-*.md` and `issues/591-*.md` were written with the text "DIAGNOSED ~20:45Z" and
"**slice 3 (2778/2779) CANCELLED 20:50Z**". Both times were in the future when the text was written.

- That cancel cannot have happened as written. Check `/admin/jobs` before relying on it.
- If 2778/2779 really were cancelled, re-enqueue the probe now under the velocity ruling, rather than holding it for 591.
  Holding it for 591 would also hold 519 M6, which is sequenced after job 2779.
- Either way, update both status lines to the decision actually taken.
