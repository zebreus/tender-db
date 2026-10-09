**Issue 492 unit 3 review: adjudication of the stored lot values >= EUR 1 bn with an exact x100 partner (2026-10-09)**

Of the 11 Tenders, the lot rule is right on 9 and wrong on 2 (8748271, 8811221). But the fold only refuses lots at 6 of them, because of how it picks partners. Both wrong refusals have the same shape, which separates them cleanly from every right one. My recommendation is to add a narrow exemption for sibling-lot partners before the drain.

## Per-Tender verdicts

The two readers agreed on every Tender, so I didn't need to settle any disagreement. I checked the deciding points with bounded SELECTs and against `crates/store/src/canonical.rs` at `10fea11`. The fold picks version N's lot value using partners from versions 1 to N only (lines ~16211-16225). As a result, a lot figure whose ×100 partner first appears in a later version is not refused. The list you gave assumed partners from any version, so it over-counts.

| Tender | Verdict | Partner | Refused as built? | Lot falls to | Head |
|---|---|---|---|---|---|
| 292242 | SLIP_LOT | Procedure estimate and result, both €36 m, v1, single lot | Yes, v1 | NULL | €3.6 bn → €36 m |
| 627800 | SLIP_LOT | Procedure estimate €39.94 m, v1, single lot | Yes, v1 | NULL | €3.994 bn → €39.94 m |
| 1003919 | SLIP_LOT | Procedure estimate €27,838,963.70, v1, single lot (decimal dropped) | Yes, v1 | NULL | €2.78 bn → €27.84 m |
| 8715174 | SLIP_LOT | Procedure estimate £25 m, v1 and v2, single lot (checked: 2500000000 next to 250000000000 cents) | Yes, v1 and v2 | NULL | £2.5 bn → £25 m |
| 524394 | SLIP_LOT | Same lot, €15.359 m, first in v2 | No, the v1 row stays €1.5359 bn | (NULL if refused) | €1.5359 bn → €15.359 m, through the refusal of the v2 procedure figure, not this row |
| 952611 | SLIP_LOT | Same lot and procedure, €17 m, first in v2 | No, the v1 row stays €1.7 bn | (NULL) | Unchanged, €17 m |
| 8591463 | SLIP_LOT | Same lot and procedure, €25,276,449.32, first in v2 (the buyer's correction) | No, the v1 row stays €2.53 bn | (NULL) | Unchanged |
| 8730855 | SLIP_LOT | Same lot 1, £600 m, first in v3 (checked: v1/v2 hold only 6000000000000, 15000000000, 25000000000 cents) | No, v1/v2 stay £60 bn | (NULL) | Unchanged, £600 m |
| 8821990 | SLIP_LOT | Same lot and procedure, €14,653,165.41, first in seq 3 | No, seq 1/2 stay €1.465 bn | (NULL) | Unchanged |
| 8748271 | GENUINE_DISTINCT | Sibling lot 4 (laundry consultancy), £10 m, seq 1 and 2 | Yes, seq 1 and 2 | NULL; the head lot row loses lot 1's £1 bn | Unchanged, £2.71 bn |
| 8811221 | GENUINE_DISTINCT | Sibling lots 1 and 2, £10 m each, seq 1 | Yes, seq 1 | NULL; the head lot row loses Lot 3b | Unchanged, £47 bn |

Notes on the table:
- **8821990:** both readers missed the versions-1-to-N point here. I checked: seq 1/2 hold only 146531654100 cents (at both procedure and lot scope), and there are no lot awards before seq 4.
- **8748271:** the 8 lots sum to £2,710 m, exactly the procedure total, so lot 1 at £1 bn is real.
- **8811221:** Lot 3b is titled "SPV's/LLP's £100m+", so it cannot be worth £10 m.
- **Head changes:** all four Tenders whose head changes were already counted in the head adjudication.

## Counts

- **On the list's own terms (partners from any version):** 9 of 11 Tenders, 12 of 15 lot rows, would be refused rightly. 2 Tenders (8748271, 8811221), 3 rows, wrongly. None is SLIP_PARTNER, PLACEHOLDER or UNCLEAR.
- **As built:** 8 lot rows at 6 Tenders are refused.
  - 5 rows at 4 Tenders are right: 292242, 627800, 1003919, 8715174.
  - 3 rows at 2 Tenders are wrong: 8748271 seq 1 and 2, and 8811221 seq 1.
  - That is a 2-in-6 wrong rate by Tender, against 1 in 56 (8819939) for the head rule.
  - Of the 6 head lot rows that change, 4 are right and 2 blank the largest lot of a genuine framework.
- **Slips the rule cannot reach:** 7 stale earlier-version rows at 5 Tenders keep their errors: 524394, 952611, 8591463, 8730855 (about €69 bn each in `value_eur_cents`) and 8821990. This follows from picking partners from versions 1 to N only, and it is the price of keeping an older version's stored value stable. None of these rows is a head lot row. They are reachable through `/v1/sql` on `tender_version_lots`, so a lot ranking has to filter to the head version.

## What separates right from wrong

On this set, the separation is perfect:

- **Wrong (2 of 2):** the partner is a sibling lot (a different lot of the same version), the Tender has 8 or 7 lots, and the refused lot is no larger than its procedure total (£1 bn vs £2.71 bn; £1 bn vs £47 bn).
- **Right and refused (4 of 4):** single-lot Tenders whose partner is a procedure figure in the same version, with the lot 100× its own procedure total.
- **Right but not refused (5):** the partner is the same lot, or the procedure, in a later version.

This is the same framework shape the head exemption already protects: a big framework lot next to a small consultancy or legal lot.

## Recommendation for 492

Add a narrow lot exemption rather than recording the cost. A lot figure L with a ×100 partner P would be kept when all three hold:

1. L's version names 2 or more lots.
2. P only ever appears as a figure of other lots (a sibling's amount or award), never at procedure scope and never as this lot's own figure.
3. When the version has a procedure figure in the same currency, L is not above it.

On this set it keeps 8748271 and 8811221, still refuses all 4 slips the rule reaches today, and exempts no slip. A simpler version would also give the same result here: the existing yes/no "partner is only ever a lot figure" flag plus "2 or more lots". But that version would exempt a same-lot partner from an earlier version of a multi-lot Tender (none in this set). So I recommend tracking which lot each figure belongs to and passing the lot's key into the lot election.

What changes in the code:
- The test that asserts a ×100 lot figure is never exempt (`canonical.rs` ~33746) needs updating.
- New pin tests:
  - 8748271 kept.
  - 8811221 kept.
  - 292242, 627800, 1003919 and 8715174 refused.

If the exemption isn't built, record 8748271 and 8811221 as known wrong refusals, and note the 7 stale rows the rule cannot reach (524394, 952611, 8591463, 8730855, 8821990) in the issue. The sample is small (6 Tenders refused as built), but both wrong refusals blank a real framework's largest lot on the served head lot row.
