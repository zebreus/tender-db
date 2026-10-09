**Issue 492 unit 2: adjudication of the nine ×100 heads at or above €10 bn, done before the drain**

The task text says these heads are €1–10 bn, but all nine convert to €10 bn or more. They run from 751664 at €10.1 bn to 8595426 at £38.3 bn. So this is the set that unit 2 item 6 asked to adjudicate before the drain, not more of the 56-Tender sample. 8784848 is not among the nine and still needs its drain re-read.

**Recommendation:** leave the decided rule as it is (k = 2 with the exemption (b)+(c)+(d), already built in 3aa9272). On this set it refuses all 6 slips and keeps both firm genuine frameworks. One Tender is contested, 4871119; record it rather than add an exemption for it. The drain can go ahead.

I checked each head version with bounded reads of `tender_version_amounts` and `tender_version_lot_results` by `tender_id`. Those reads correct some of the "falls to" figures the adjudicators gave (section 4).

**1. Counts**

| Verdict | Count | Tenders |
|---|---|---|
| SLIP_HEAD | 6 | 751664, 5545591, 6409799, 6640498, 7490161, 8810872 |
| GENUINE_DISTINCT | 2 firm + 1 contested | 8595426, 8618327; 4871119 (low confidence) |
| SLIP_PARTNER | 0 | |
| PLACEHOLDER | 0 | |
| UNCLEAR | 0 | 4871119 is close to this |

| Partner relation | Count | Verdicts | Pooled with the 56 |
|---|---|---|---|
| same field, earlier version | 4 | 3 SLIP_HEAD (5545591, 6409799, 7490161), 1 contested (4871119) | 21: 19 slip heads, 1 slip partner, 1 contested |
| same version, other field | 2 | 2 SLIP_HEAD (751664, 6640498) | 14, all slip heads |
| lot vs procedure | 3 | 2 GENUINE (8595426, 8618327), 1 SLIP_HEAD (8810872) | 23: 12 slip heads, 11 genuine |
| sibling lot | 0 | | 1 placeholder |
| other | 0 | | 6, all slip heads |

Two labels need a footnote:
- 6640498's partner is v1's €140 m estimate carried into v2, so it is really cross-version and cross-field.
- 8618327's partner is a lot of an earlier version.

**2. False positives**

A plain k = 2 rule would refuse all 9 heads:
- Wrong refusals on this set: 2 of 9 (22.2%, 8595426 and 8618327), or 3 of 9 (33.3%) if 4871119 is genuine. No SLIP_PARTNER cases.
- Pooled with the 56: 12 of 65 (18.5%), or 13 of 65 (20.0%).

The decided rule:
- Wrong refusals on this set: none firm, 1 contested (4871119).
- Pooled: 51 of 51 slips refused. Wrong refusals are 1 of 65 (1.5%, 8819939), or 2 of 65 (3.1%) with 4871119. Precision is 51/52 (98.1%), or 51/53 (96.2%) with 4871119.

**3. Whether a narrower condition separates the slips**

Each condition tried as the trigger for refusal, on this set:
- **Partner is the same field in an earlier version:** catches 3 of 6 slips. It misses 751664, 6640498 and 8810872, and it would refuse the contested 4871119. Pooled: 19 of 51 slips.
- **Partner is a sibling lot:** catches 0 of 6.
- **A lot estimate above its own procedure total:** catches 0 of 6.
- **Lots sum to the procedure total:** this is what the genuine heads look like. The sum/F ratio is exactly 1.000 for both 8595426 and 8618327. It is also true of the slip 6640498. There, lot 1 carries the same €14 bn slip, and the four €0.01 lots fall below the 10.00 partner floor, so the sum equals F exactly. Used as a trigger it would refuse both genuine heads. Used as the only exemption it would keep a slip.

The exemption, Tender by Tender:
- **(b) fails (fewer than two lots):**
  - 751664, 5545591, 6409799, 7490161 and 8810872 have 1 lot.
  - 4871119's head version has no lots.
- **Only (c) fails: 6640498.**
  - (b) holds: 5 lots, read from the lot results.
  - (d) holds: the lots sum to F.
  - (c) fails because the partner is the procedure estimate.
  - This is the third case, after 5864294 and 7240718, where (c) alone carries the refusal.
- **Kept: 8595426.** 8 lots, the partner is lot 6 (the 1% share) in the same version, sum/F = 1.000.
- **Kept: 8618327.**
  - 3 lots, and the head version's lots sum to F (7 + 2.5 + 0.5 = £10 bn).
  - The partner is lot 3's £100 m in v1 and v2; in v4, lot 3 is £500 m.
  - It is kept only because `lot_only` reads the partner's scope across the whole chain. If (c) read only the head version, 8618327 would be wrongly refused.

**4. What the drain will elect (the head is the largest admitted amount of the head version)**

| Tender | Falls to | Note |
|---|---|---|
| 751664 | €101.2 m | Correct. |
| 8810872 | £100 m (the lot estimate) | Correct. |
| 6640498 | €140 m | Correct. |
| 5545591 | HUF 30.913 bn (the estimate) | Not the HUF 51.36 bn the adjudicator expected. The head version, v10, holds only the estimate and the slipped award, so the head comes out below the real award of about HUF 51–53 bn. It is still a genuine figure. |
| 7490161 | €156.4 m (the v2 award) | v2 replaces v1's €204 m estimate. It is probably still ×100 too high: about €1.56 m would fit a village of 2,500. It is below €1 bn, so no rule reaches it. Re-read it. |
| 6409799 | RON 7,175,520,821.14, about €1.4–1.5 bn | This is the next largest of the head version's 101 amounts. It is not an exact 10^k of any figure (its cents end in 14), so it stays in the €1–10 bn band as a garbled figure about 10 times too large. The true value is about RON 750 m. Re-read it. |
| 4871119 | NULL, which takes it out of the value ordering | Not £250 m. v4 stores only £25 bn, v1 stores no amounts, and £250 m exists only in v3. |
| 8595426, 8618327 | Unchanged | |

**5. 4871119**

- **For genuine:** v4 deliberately restates £25 bn in an OLD/NEW block. The same F14 widens the scope by adding "construction" and "in the United Kingdom".
- **Against:** v3 called £25 bn "incorrect". £25 bn is about 36 times the predecessor framework's £700 m ceiling and far above the buyer group's turnover.
- **Interaction with 489:** issue 489 recorded, as open by design, that `ScalePartners` refuses an upward ×10^k correction. At k = 2 that case is now real.
- **Recommendation:** do not build a corrigendum exemption for one low-confidence Tender. Any exemption for upward corrections would need its own measurement across the corpus. Record 4871119 next to 8819939 as the known possible cost. A NULL head for a publisher that contradicted itself twice is defensible.

**6. Pin tests to add**

None of the nine is pinned today. The only existing reference is 4871119's 489 flip-flop test at `crates/ingest/tests/project.rs:1867`.
- 8618327 kept: the partner is an earlier version's lot, and the lots sum exactly.
- 6640498 refused by (c) alone: (b) and (d) hold, and the €0.01 lots sit below the floor.
- 8595426 kept: 8 lots summing exactly, with the 1% lot as the partner.
- Optionally, 4871119 with a NULL head.

**Re-read list for the drain:**
- From this set: 6409799, 7490161 and 8784848.
- Still owed from the 56: 5864294, 7240718, 6449525 and 6988280.

Files:
- `/home/user/tender-db/.scratch/tender-db/issues/492-a-x100-scale-slip-is-below-the-exact-10k-rule.md`
- `/home/user/tender-db/crates/store/src/canonical.rs` (`ScalePartners::framework_total`, `partner_x100`, `set_head`, around lines 3440–3620)
- `/home/user/tender-db/.scratch/tender-db/492-x100/adjudication-synthesis-wf_b250dc41-ad9.md`