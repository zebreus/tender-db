# 533 — `status=open` serves procedures their newest notice closed or awarded: a no-winner result or UK12 termination inherits the contract notice's deadline, and DÖE sdk-0.1 award notices publish one of their own

Status: ready-for-agent — filed 2026-10-14 from an external data review (verified: verifier + adversarial refute). Next: unit 1, the decision (S–M).
Kind: API semantics / data quality (read layer: `status` on `/v1/tenders` and `/v1/lots`, SSE and webhook filters; the fold's head deadline)
Relates to:
- 273 (RESOLVED): `status` is a range on the indexed head column `t.current_deadline`; a fix must stay one.
- 216 (RESOLVED): `current_deadline` and `sort=deadline` ("closes soon", paired with `status=open`); job 706 is the
  backfill precedent.
- 370 (DONE): carry-forward documented; its "deadline < published_at" marker cannot see a result published BEFORE
  the carried deadline.
- 389 / 422 / 424 (DONE): per-lot `status` reads the same carried dates, so these tenders' lots are open too.
- 366 / 474 (DONE): the deadline window, one home in `canonical::deadline_admitted`. It bounds typos, not results.
- 375 (DONE): a backfill must not carry a second head election.
- 257 (CLOSED): sdk-0.1 award notices keep an unstated decision NULL by design, so a `clos-nw` rule misses DÖE.
- 29 (VERIFIED): `project.rs:424` maps SDK01 `…TenderSubmissionDeadlinePeriod-EndDate` to `submission_deadline`,
  award notices included.
- 510 / 512 / 525 (done), 521 (ready-for-agent, low): the no-award markers that make a result read `clos-nw`;
  inputs to a fix, not this defect. 386 (done): FTS OCID welds, relevant only to unit 3's multi-round shapes.
- None covers a result or termination that closes a procedure before its deadline, and no board issue or earlier
  review (2026-09-07, 09-15, 09-27) records a decision against it.

## Verify

    curl -s "https://tenders.zebreus.click/v1/tenders?publication_id=00043118-2025&status=open&limit=1" "https://tenders.zebreus.click/v1/tenders?publication_id=25892804-1&status=open&limit=1" "https://tenders.zebreus.click/v1/openapi.json" | jq -c 'if .components then .components.parameters.status.description else [.items[].id] end'

It prints three lines: the carry-forward exemplar (841082), the DÖE exemplar (8831547), and the served `status`
description. Lines 1–2 read unit 2's result and line 3 reads unit 4's. Unit 3 is a measurement.
- **done**: `[]`, `[]`, and a description that names what closes a Tender besides its deadline. 841082 is a
  restricted procedure, not an open-ended scheme, so no outcome of unit 1(a) keeps it open. If a later plausibility
  rule drops its 2029 deadline for another reason, take a new exemplar from the cohort read in Repro.
- **open (2026-10-14 16:04 UTC)**: `[841082]`, `[8831547]`, `"By submission deadline."`

## Repro

Public GETs, 2026-10-14 16:04 UTC:
- `/v1/lots?tender=403644&status=open&limit=5` gives `[1108460]`, so the lots surface is affected.
- `/v1/tenders?publication_id=00043118-2025&status=closed&limit=1` gives `[]`.

The mechanism-1 cohort read is a data-page read (gated per `prod-box-reads.md`; never the Verify line). Substitute
the current epoch for `<now>`. Plan read locally 2026-10-14: `SEARCH t USING INDEX tenders_current_deadline`, every
subquery a `(tender_id, seq)` or PK seek. It returned 178 rows at now=1791990321.

    echo "SELECT t.id, t.source, v.notice_subtype, v.publication_id, t.current_deadline FROM tenders t CROSS JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq WHERE t.current_deadline > <now> AND EXISTS (SELECT 1 FROM tender_version_lot_results r CROSS JOIN lot_results lr ON lr.id = r.lot_result_id WHERE r.tender_id = t.id AND r.seq = t.current_seq AND lr.notice_id = v.caused_by_notice_id) AND NOT EXISTS (SELECT 1 FROM tender_version_lot_results r WHERE r.tender_id = t.id AND r.seq = t.current_seq AND (r.decision IS NULL OR r.decision <> 'clos-nw')) AND NOT EXISTS (SELECT 1 FROM tender_version_lots l WHERE l.tender_id = t.id AND l.seq = t.current_seq AND l.kind = 'Lot' AND NOT EXISTS (SELECT 1 FROM tender_version_lot_results r WHERE r.tender_id = t.id AND r.seq = t.current_seq AND r.lot_id = l.lot_id)) ORDER BY t.current_deadline DESC LIMIT 500" | ssh root@zebreus.click /root/sq.sh

It does not see mechanism 2 (DÖE, decision NULL); those 141 need their own read.

## What was found

`status=open` is `t.current_deadline > now` (`crates/store/src/read.rs:1123-1133`, `StatusBy::Head`). The fold writes
`current_deadline` from `canonical::head_deadline` (`canonical.rs:3109`), which is the MAX admitted
`submission_deadline` over the head version's tender and lot facts. Two mechanisms put a live date on a finished
procedure:
1. **Carry-forward.** These result notices publish no deadline: TED eForms 29/30/32/33/E4, legacy TED results, FTS
   UK12 (procurement termination) and FTS award+contract. The head version carries forward every fact the newest
   notice is silent about (the /docs Dates caveat, 370). So it inherits the contract notice's deadline and stays
   open until that date passes.
2. **Own deadline (DÖE sdk-0.1).** A `can-standard` award notice publishes its own
   `TenderSubmissionDeadlinePeriod-EndDate`, and `project.rs:424` maps that to `submission_deadline`. The tender has a
   single version, headed by the award, with decision NULL (257).

The code assumes neither exists: `read.rs:1124` ("award notices … Closed … it cannot be bid on"), `Status::Closed`
(`read.rs:162-166`, "award notices do not" carry one) and `head_deadline`'s doc ("`None` … (award notices)").
`fts/checklist.rs:116` ignores OCDS `tender.status` because status "is derived from its notices and deadlines"; the
notices half is not implemented. The public contract, "By submission deadline." (`openapi.json:755`, `docs.rs:172`),
is met to the letter.

**Counts** (the verifier's bounded reads, 2026-10-14 15:05 UTC, now=1791990321; the refute re-ran the 178 by bucket
at 15:40 and read the 141; nobody re-measured 42,062, 441/353/88 or the 175):

| cohort | tenders |
| --- | --- |
| open (`current_deadline > now`, indexed count) | 42,062 |
| head version carries any lot_results row | 684 |
| … every lot decided (`selec-w`/`clos-nw`, no `open-nw`) | 441 |
| … … newest notice is a CN or tenderUpdate after an earlier result (a re-launch; inferred from head subtype only) | 88 |
| … … newest notice is the result itself | 353 |
| … … … all `clos-nw`, no winner (mechanism 1) | **178** |
| … … … at least one `selec-w` (unit 3) | 175 |
| DÖE single-version sdk-0.1 award head, decision NULL, its own deadline (mechanism 2) | **141** |
| unclassified: partial, `open-nw` or null-decision head results outside DÖE (684 − 441 − 141) | ~102 |

- **The 178 by source:** TED 29: 126, legacy TED: 12, 33: 7, 30: 4, 32: 2, E4: 1; FTS UK12: 18, FTS award+contract: 8.
- **The 178 by deadline, measured from now:**
  - 128 are ≤30 days out: they leave `open` within a month on their own, but they top "closes soon". 38279's
    result was published 2026-06-30, four months before its 2026-10-29 deadline, and it was served open throughout.
  - 16 are 30 days to a year out; 5 are open-ended schemes (445631 ДСП, 336277 DAS, 634469 and 913640 OH-Rabatt,
    179459 a non-exclusive Rahmenvertrag).
  - 34 are more than a year out: about 24 clearly open-ended and about 5 probably, by title (DPS/DAS, SGB V §130a(8)
    open-house, LOV/auktorisationssystem, qualification systems, an e-marketplace); 2 explicit cancellations; 2
    implausible deadlines (all four in the table below).
- **The 141 DÖE:** all seq 1, decision NULL, deadline 23 to 239 days after publication (median 184). They were
  published 2026-04 to 2026-10, so new ones keep arriving. The raw content was read for 2 of 2 (below). No open
  DÖE tender has a `clos-nw` or `selec-w` head result, so mechanism 1 is TED and FTS only.
- **The 175 with a winner:** 171 have the deadline more than 60 days after the award, and 112 more than a year out.
  That fits DPS, framework, dynamic-market (UK14/UK15), qualification-system or multi-round shapes, where `open` can
  be right. 458370 has 34 versions: LOT-0001 was re-tendered 2026-09-25/28 and awarded 2026-10-01.

**Exemplars** (public reads, 2026-10-14 16:04 UTC; all served by `status=open`):

| tender | head notice | result | deadline | role |
| --- | --- | --- | --- | --- |
| 841082 | TED 29 00043118-2025, after CNs of 2024-12-09 and 2024-12-31; restricted | 6 lots `clos-nw`, reason `other` | 2029-12-11 | plain cancellation, Verify line 1; deadline 5 years after a restricted CN |
| 8831547 | DÖE 25892804-1, 2026-10-05, single version ("Trockenbauarbeiten") | decision NULL | 2027-04-30, its own | mechanism 2, Verify line 2 |
| 1706547 | DÖE 25232038-1, 2026-04-15 (notice 26924865: `can-standard`, AwardDate 2026-04-15) | NULL | 2026-10-15, its own | leaves `open` tonight |
| 38279 | TED 29 00447924-2026, 2026-06-30 | `clos-nw`, `other` | 2026-10-29 | the review's example |
| 66939 | TED 29 00704410-2026, 6 versions | `clos-nw`, `chan-need` | 2026-10-30 | the review's example |
| 8576007 | FTS UK12 095413-2026, after UK4 088907-2026 | `clos-nw` | 2026-10-20 | termination |
| 6739006 | legacy TED 438170-2022, "Aufhebung des Open-House-Verfahrens …" | `clos-nw` | 2030-02-28 | explicit cancellation of a scheme |
| 7422682 | legacy TED 366281-2022, "CANCELLED Dynamic purchasing system …" | `clos-nw` | 2028-07-17 | explicit cancellation of a scheme |
| 7207004 | legacy TED 641439-2022, restricted | 2 results, `clos-nw` | 2029-07-31 (= duration_end) | implausible deadline |
| 403644 | TED 29 00059850-2025, Kalmar auktorisationssystem, `oth-single` | `clos-nw`, `other` | 2034-12-30 | **counter-example:** the date is the scheme's admission window, and `open` may be right |

**Surfaces affected:** `/v1/tenders?status=`, `/v1/lots?status=` (403644 gives lot 1108460), `sort=deadline` with
`status=open`, and SSE and webhook subscriptions, which share `Filter`.

### What the external review got wrong

- "3,660 open tenders": it was 42,062 on 2026-10-14. The 3,660 does not reproduce and was probably a subset.
- "98 have a result deciding every lot": it is 441, and that mixes two groups. 88 are re-launches, correctly open
  (inferred from the head subtype). For 353, the result itself is the newest notice.
- "33 closed with no winner": it is 178, or 206 counting re-launches. The 178 include 18 FTS UK12 terminations and
  8 FTS award+contract notices, so this is not TED only.
- "Finished procedures": this holds only in part. About 29 to 34 of the 178 are open-ended schemes, and most of the
  175 with a winner are DPS/framework shapes.
- It missed mechanism 2 entirely: the 141 DÖE award notices with their own deadlines.
- "Status only looks at the deadline" is accurate, and documented. The review missed why the deadline is there.

Nor re-file the first verification draft's errors: "a result notice publishes no deadline" (false for DÖE
sdk-0.1); "128 ≤30 days out = cancellations shortly before the deadline" (counted from now, not from the result);
403644 as a cancellation or a Verify exemplar.

## Proposed units

1. **Decide (S–M; the decision is the step).** Record the decision here with its reasoning. It has three parts.
   - (a) **Does an all-`clos-nw` head result, or an FTS UK12 head, close a Tender?**
     - The candidate answer is yes for a one-shot procedure (841082, 38279, 8576007).
     - For an open-ended scheme (DPS/DAS, open-house, LOV/authorisation, qualification system, e-marketplace), a
       `clos-nw` award may report one round with no award. Explicit cancellations (6739006, 7422682) sit in the same
       class as 403644.
     - BT-765 and BT-766 (framework/DPS) are not projected; grep finds them only in comments. Either project them
       first, as a sub-unit, or decide on what is projected (procedure type, subtype, the deadline's distance from
       publication). Never decide on the title.
   - (b) **What about award-type heads with a NULL decision (DÖE sdk-0.1 `can-*`)?** Two options:
     - Stop admitting a `submission_deadline` that an award-type notice publishes, in `deadline_admitted` or
       `head_deadline` (one home, 474). This also changes the served `submission_deadline`, `sort=deadline` and
       `deadline_after/_before`.
     - Or keep the date served and close an award-type head through the status column.
     Read 3 to 5 more raw notices first, to settle what that date means on an award.
   - (c) **The read shape.** `status` must stay an indexed range on a head column (273). Two options:
     - Option A: a fold-maintained `current_open_until` (equal to `head_deadline`, NULL when the rule fires), with a
       `(col, id)` index read by `status`. `current_deadline` keeps sort and the deadline bounds.
     - Option B: a flag folded into the same range.
     Today `status=open&sort=deadline` and the `deadline_after` "closes soon" shape ride `tenders_current_deadline`
     (`read.rs:1903`). Read both plans locally and re-run 273's closing matrix; nothing may regress.
     - Lots: a lot whose head-notice result is `clos-nw` is closed. That is a NOT EXISTS term in 424's per-lot EXISTS,
       through the `canonical` helper. 423's cpv-prefix control must not regress.
     - Re-launch: a head CN after an earlier `clos-nw` stays open. The 88 were inferred from head subtype only, so
       confirm this on a sample.
2. **Build, gate, deploy, backfill (M).** The rule only changes answers for the open head (`current_deadline > now`,
   an indexed range of about 42k), because a past deadline is closed already.
   - Option B, with a flag that defaults to not-closed, needs a backfill over that range only. Option A needs the
     column on every row: job 706 filled 7.92M for `current_deadline`.
   - Either way, reuse the fold's own function (375), in 216's job-706 shape, not a corpus refold.
   - A result that arrives later must flip the tender at fold time.
   - Then read the Verify and the Repro cohort read. The cohort should drop to what 1(a) deliberately keeps open.
     Check that an SSE `status=open` subscription and a webhook filter see the change.
3. **Measure the rest (M).** File what remains as new issues.
   - The 175 with a winner: read BT-765/BT-766, UK13–16 and the deadline distance on a sample. Does "awarded before
     the deadline" ever close a non-DPS procedure?
   - The ~102 unclassified non-DÖE heads (partial, `open-nw`, null decision).
   - The channels nobody has measured: FTS OCDS `tender.status` (ignored at `checklist.rs:116`, so a cancellation
     carried only there is invisible) and eForms BT-140 change notices (not parsed).
4. **Docs (S).** `openapi.json:755` and `docs.rs:172` name what closes a Tender besides its deadline. Correct the
   `Status::Closed` doc, the `read.rs:1124` rationale and `head_deadline`'s doc (award notices DO carry a deadline,
   inherited or their own), and make `checklist.rs:116` true or correct it. Gate the served sentence (the 370 class).
