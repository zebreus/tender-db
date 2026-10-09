# 492 — a ×100 scale slip (k = 2) is below the exact-10ᵏ rule

Status: ready-for-agent — UNIT 3 BUILT 2026-10-09 (`3aa9272`, review fixes `10fea11`, sibling-lot exemption on top): the
k = 2 rule with the framework exemption, plus `refold-value-band`. NEXT: gate, deploy, then the drain
(`refold-value-band` dry, then wet, then `project`), then re-read the adjudicated ids and the lot moves (operations.md,
"The ×100 extension").
- **Review** `wf_45a636e0-ec3`: 12 findings, 10 confirmed, 2 refuted (the k ≥ 3 corroboration shielding a ×100 figure:
  no measured head has both partners; the drain precondition: the ≥ €10 bn adjudication was already done). Fixed in
  `10fea11`:
  - `set_head` counted LotsGroups and carried PIN parts as lots, so a group ceiling doubled the lot sum. Only Lots
    count now; parts only in a head with no Lot.
  - Sentinels and withheld figures are out of the lot sum.
  - A tender-scope copy of a head lot award is a lot figure (decision (c)).
  - Docs: /docs, the data-quality partner column, operations.md, the misplaced doc comment, the 6988280 sentences;
    `refold-value-band` joins `heavy_write_kind`.
- **Lot values, measured and adjudicated** (`wf_5ec959ae-7cc`; `.scratch/tender-db/492-x100/lot-adjudication-*`). The
  review's open point: decision (b) was measured on heads, and a ×100 LOT figure had no exemption.
  - 4,974 stored lot rows are at ≥ €1 bn (862 Tenders, any version).
  - 13 (Tender, value) pairs of those have an exact ×100 partner. 474292 and 8819939 were already adjudicated as heads;
    the other 11 were read by two independent readers each, who agreed on all 11.
  - Result: 9 slips, 2 genuine (8748271's £1 bn laundry lot beside a £10 m consultancy lot; 8811221's "SPV's/LLP's
    £100m+" lot beside two £10 m lots).
  - As built (no lot exemption) the fold refuses 6 of the 11: 4 right (292242, 627800, 1003919, 8715174, each a single
    lot 100× its own procedure total) and 2 wrong. That is 2 wrong in 6, against 1 in 56 for the head rule.
  - The other 5 slips (524394, 952611, 8591463, 8730855, 8821990) are unreachable. The fold elects version N's lot
    value over versions 1..=N, and their partner arrives in a later version, so they stay on earlier-version rows. No
    head lot row carries one.
  - **Built: the sibling-lot exemption.** A ×100 lot figure is kept when all three hold: the version has ≥ 2 lots; its
    partner is only ever a figure of OTHER lots; the version has a same-currency procedure figure at least as large.
    It keeps both genuine lots and refuses all 4 reached slips. Pinned by
    `a_x100_lot_figure_is_kept_only_beside_a_sibling_lot_partner`.
  - **Review** `wf_cd4a83f3-6b2`: 8 confirmed, 1 refuted (legacy lot keys being per-notice positions). Fixed:
    - The bound took any procedure figure, including one the election refuses or one over the ceiling, so a procedure
      copy of the slip vouched for it and the head rose to it. Now it must be an admitted procedure figure.
    - The head election no longer applies the exemption to lot candidates: an admitted bound outranks them anyway.
    - A Part's or a LotsGroup's figure is no sibling partner. A PIN's PAR-0001 may be the CN's LOT-0001.
    - Only a `result_value` award copy is excluded from the bound; an `estimated_value` total stays.
    - Docs: 474292 and 8819939 added to the expected lot moves; /docs and the CHANGELOG state the full condition.
  - **Review** `wf_963404e4-a92` of the fix (`36101c6`): 7 confirmed, all minor, fixed:
    - The bound and the head election disagreed on what an award copy is, so a total the head refuses could still
      vouch for a lot. Both now use the head's test (any field equal to a head lot award). An estimated total that
      happens to equal an award is lost as a bound, which only refuses.
    - A carried PIN Part's own figure got the exemption against the CN lot it may be. A Part is now elected with no
      key and gets no exemption. A LotsGroup keeps it.
    - The operations.md and CHANGELOG wording now matches the code.
  - Left, conservative direction (a genuine lot refused, never a slip kept): a legacy F03's V.2.4 lot estimate is
    folded at tender scope as `estimated_value`. When it differs from the lot's award it registers as a procedure
    figure and poisons that figure as a sibling partner. Attributing RES-section amounts to the award's lot needs a
    section-origin marker on the fact. Reopen if a drain re-read shows a genuine lot lost this way.
- **≥ €10 bn adjudication** (`wf_fd3e7a72-62a`; files `.scratch/tender-db/492-x100/adjudication-over-10bn-*`): 9 heads
  have a ×100 partner: 6 slips, 2 firm genuine and 1 contested.
  - Slips: 751664, 5545591, 6409799, 6640498, 7490161, 8810872.
  - Firm genuine: 8595426 and 8618327. Each has 8 or so lots summing exactly to F.
  - Contested: 4871119. The publisher's second F14 deliberately restates £25 bn over the corrected £250 m. Low
    confidence it is genuine.
  - The built rule refuses all 6 slips, keeps both firm frameworks and refuses 4871119.
  - Pooled with the 56: 51 of 51 slips refused, with 1 wrong refusal (8819939) or 2 if 4871119 is genuine. A
    plain k = 2 rule makes 12 or 13 wrong refusals.
  - Re-read at the drain: 7490161 falls to €204 m, itself probably cents-as-units; 6409799 may fall to another
    wrong figure (RON 7.18 bn); 8784848 still needs a re-read.
- **Adjudication** `wf_b250dc41-ad9`, 56 of 56 Tenders: 45 SLIP_HEAD, 9 GENUINE_DISTINCT, 1 SLIP_PARTNER (8819939), 1
  PLACEHOLDER (6988280). The verdicts and the synthesis are in `.scratch/tender-db/492-x100/adjudication-*`.
  - All nine genuine heads are multi-lot frameworks or DPSs, with 4 to 22 lots. The head is the procedure total and
    the ×100 partner is one small lot: 8713680, 8423121, 8804016, 6572976, 8730895, 6904931, 7956096, 8800131,
    8638612.
  - A **plain k = 2 rule** would wrongly refuse 10 of 56 (17.9%), against 471's k ≥ 3 rate of 1 in 65.
- Unit 2 decided 2026-10-09: extend the rule to k = 2 with a framework exemption.
- **Decision.** Refuse ×100 heads, but keep a head F when all three hold:
  - (b) the head version has two or more lots, and F is a procedure figure;
  - (c) every ×100 partner is a lot figure: a lot-scope amount, a lot award, or a lot-null `result_value` equal to
    a lot award of the same version;
  - (d) the head version's lot figures sum to between F/10 and F.
  - On the sample this refuses 45 of 45 slips with 1 wrong refusal (8819939, 1.8%), which matches k ≥ 3's rate.
    8819939 is recorded as the known cost, as 8287294 is for 471.
- **Unit 3 hazards** from the synthesis:
  - `ScalePartners.figures` carries only (currency, cents). It needs per-figure scope, the head version's lot count
    and the lot sums.
  - Pre-eForms award notices store lot awards in `tender_version_amounts` with `lot_id` NULL. Reading scope from
    `amounts.lot_id` alone would wrongly refuse 8423121, 6572976 and 6904931. Use `tender_version_lot_results`.
  - 471's head-version corroboration exemption protects no genuine head at k = 2, and keeps 5–8 slips (one figure
    copied into two slots of one notice). Do not apply it at k = 2.
  - Pin tests: 8800131 kept (lots sum exactly); 8423121 kept (r2.0.9, lot partners at lot NULL); 8413585 refused
    (27.7×); 5864294 refused (sum ≈ F, but the partner is a procedure figure); 474292 refused (one lot, the lot at
    100× the procedure); 8819939 the recorded wrong refusal.
  - Refused is not corrected. 5864294, 7240718, 6449525 and 6988280 fall to other wrong figures at the drain; re-read
    them.
  - Before the drain, adjudicate the ≥ €10 bn ×100 rows (8595426, 8618327, 8810872, plus the known slips 6640498 and
    8784848).
- Unit 1 (2026-10-09): **56 of the 3,510 heads in €1–10 bn** (49 TED, 7 FTS) have an exact ×100 same-currency
  partner. The list is `.scratch/tender-db/492-x100/x100-partner-hits-2026-10-09.json`.
- Unit 1's read was bounded `/v1/sql`: the band in keyset pages off `tenders_current_value_eur`, then a partner test in
  batches of 200 ids. A partner is any version and any field of the same Tender.
- Many partners are the same field in an earlier version, so a corrigendum is restating a value ×100 one way or
  the other. Some hits are placeholder ladders: 6988280's result values are €100k, €10m, €100m and €1bn.
Was: ready-for-agent — filed 2026-10-08 from issue 471 unit 5. Band count done (5 of 306 rows at ×100; see
"Unit 1 — first count"). The €1–10 bn range was to be a windowed read after issue 490's backfill project (a
direct partner probe 408s while the fold runs), then adjudicated.
Kind: data quality / head election (`ScalePartners`, `crates/store/src/canonical.rs`)
Relates to: 471 (unit 4(a), which set `SCALE_ERROR_MIN_EXPONENT` = 3; unit 5's sample)

## What is wrong

Issue 471 unit 4(a) refuses a figure worth ≥ €1 bn when it is exactly 10ᵏ times a same-currency
partner of the same Tender, but only for k ≥ 3. Two of the 20 errors in unit 5's sample are ×100
slips, and the rule therefore keeps them:

- **6640498** (TED 337801-2020, €14 bn). Section 16 shows a ×100 partner. The adjudication found ×100,
  either a typo or a dropped decimal.
- **8784848** (Essex County Council care-homes framework, FTS 004793-2022, lot 1 at £60 bn). The
  procedure estimate is £1 bn and the other lots are £150 m and £250 m. With lot 1 at £600 m the three
  lots sum exactly to the procedure total. The buyer's later IRN notices are £600 m and £1.57 bn.

## Why k = 3 was the floor

Round hundreds are common between genuine figures of one Tender: a €1 m lot inside a €100 m framework,
or a €10 m estimate beside a €1 bn ceiling. Unit 4's adjudication measured k ≥ 3 only. The k = 2
false-positive rate was never measured, and the €1 bn gate alone does not bound it.

## Units

1. **Measure.** List the Tenders whose head (≥ €1 bn) has an exact ×100 same-currency partner, using
   the data-quality band machinery or a bounded read off `tenders_current_value_eur`. Adjudicate a
   sample.
2. **Decide.** Choose one: extend the rule to k = 2 (perhaps only when the partner is a sibling LOT or
   the sum check of 8784848 holds), add a narrower structural signal ("a lot estimate above its own
   procedure total"), or caveat-only.
3. Build, gate, drain and re-read section 16, if step 2 says so.

## Unit 1 — first count (2026-10-08)

Read off section 16 of dq 2954 (`471-values/section16-dq2954-2026-10-08.txt`). Its "x smallest" column
gives the head figure's ratio to the Tender's smallest figure. **5 of the 306 band rows sit at exactly
×100:**

- 8595426 (FTS, £38.3 bn)
- 4871119 (TED, £25 bn)
- 8618327 (FTS, £10 bn)
- 8810872 (FTS, £10 bn)
- 6640498 (TED, €14 bn)

The other band ratios: ×10 on 6 rows and ×1,000 on 2. "×smallest" is not the same as an exact partner:
the smallest figure can sit in a different field or version, and the rule also wants a single-field head.

4871119 is issue 489's flip-flop case: an F14 corrects £25 bn to £250 m, then a second F14 corrects it back
to £25 bn. A ×100 rule would refuse a figure the publisher restated twice. That is one reason k = 2 needs
its own adjudication before it is extended.

Two attempts at a direct partner probe over heads of €1 bn or more, and then €10 bn or more, both answered
408 during project 2067 and were not retried. The €1–10 bn range waits for a quiet box: a windowed
`tenders_current_value_eur` read with the partner test per window, sized like section 16's.

