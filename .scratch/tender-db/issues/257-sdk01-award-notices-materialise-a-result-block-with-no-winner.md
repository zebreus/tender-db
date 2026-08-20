# 257 — sdk-0.1 award notices materialise a result block and ~95 % of them name nobody

Status: DIAGNOSED + FIXED 2026-08-20 — and the diagnosis inverts the issue. The winner gap is the
PUBLISHER's (~87 % of this dialect's award notices name nobody; where a winner IS published we resolve
100 % of them). What was ours is the opposite defect the investigation turned up: we read that silence
as `clos-nw`, "closed, no award", on ~125k notices whose result block states the award DATE. Fixed:
the date now projects, and an unstated decision stays NULL. Not yet deployed — the fold is running
Kind: was "extraction gap"; actually publication reality + a fabricated decision of ours
Blocked by: —
Relates to: 100 (the same SHAPE one dialect over: eForms-DE 1.x's winner chain resolves to nothing),
231 (where I first noticed the 1.1 % and correctly refused to file without a denominator), 101 (the
report column that will now measure this directly), 13 (results layer)

## The measurement

From the 2026-08-20 report (`/admin/reports/data-quality`), three sections read together:

    section 1   DÖE sdk-0.1 island   667,084 versions   winner  1.1%
    section 3   DÖE sdk-0.1 island   138,950 award notices → 138,950 with lot_results (100.0%),
                                     0 with no block parsed
    section 2   DÖE sdk-0.1 island   140,638 awards, 137,889 unchained (2.0% linked)

So the era's result blocks materialise **perfectly** — 138,950 of 138,950 — and the winner column
says only ~1.1 % of ALL versions carry a winner. 1.1 % of 667,084 is ≈ 7,300 versions, against 138,950
award notices: **roughly 5 % of this dialect's award notices name who won**, and ~95 % carry a result
block with nobody in it.

This is exactly the question issue 231 left open and told me to measure before filing ("if sdk-0.1 is
overwhelmingly contract notices, 1.1 % may be at or near its ceiling"). It is not: award notices are
21 % of the era's versions, and the result blocks are all there.

The committed fixture proves the path CAN work —
`sdk01_projects_title_buyer_and_winner` resolves `1. Firma: IABG mbH` from an inline `WinningParty` —
so this is not "the reader does not exist", it is "the reader misses most real payloads".

## Why it is filed separately from 100

Issue 100 is `eforms-de-1.x`: synthetic result-section ids versus published-id references, with a
design already decided. sdk-0.1 is a different dialect with a different result graph (inline
`ContractingParty`/`WinningParty`/`TenderResult` sections rather than eForms' RES→CON→TEN→TPA chain),
and its own 139k-notice cohort. Same symptom, different mechanism — merging them would blur the fix.

## Next step

One sdk-0.1 CAN from prod, read end to end: does the payload carry a `WinningParty` at all, or does
it name the winner somewhere the reader does not look? The parse layer's own `SDK01-*` inventory is
the place to start, and issue 231's probe technique (parse every committed fixture and print what it
claims) is the cheapest version of it.

The number itself stops needing arithmetic on the next report run: issue 101's new section-3 column
prints `with winner` and its rate against the materialised results directly.
## Diagnosed 2026-08-20 — it is the publisher's gap, and OUR defect was the opposite one

Measured against the raw DÖE archive on the box (`/data/archive/doe/monthly/*.zip`), three months,
namespace-agnostically:

| month | sdk-0.1 notices | award-type | carry a `TenderResult` | carry a `WinningParty` | of those, carry a `PartyName` |
|---|---|---|---|---|---|
| 2023-01 | 14,162 | 2,895 | **2,895 (100 %)** | 392 (13.5 %) | 392 (100 %) |
| 2023-06 | 19,261 | 4,002 | **4,002 (100 %)** | 603 (15.1 %) | 603 (100 %) |
| 2024-06 | 13,581 | 2,845 | **2,845 (100 %)** | 53 (1.9 %) | 53 (100 %) |

Three things fall out, and the third is the finding.

**1. The 100.0 % density is the serializer, not richness.** Every award-type notice carries a
`TenderResult` element. That is why section 3 of the data-quality report reads exactly 100.0 % with
`no block parsed: 0` for this era — a number I had flagged as suspiciously exact when I filed this
issue, and it is: the container is unconditional.

**2. Our extraction of the winner is not lossy — it is exhaustive.** Where a `WinningParty` is
published we resolve it, and every one of the 1,048 sampled carried a `PartyName` for us to resolve.
The corroboration from the other side: section 1 reports **buyer 100.0 %** for this era, and the buyer
travels the identical `mentions()` → `resolve_one_mention` path as the winner. A broken resolver
cannot be 100 % on one role and 5 % on the other. So the ~95 % is what the publisher withholds.

**3. What the block DOES carry is an award date — and we were overwriting that with a false claim.**
All 2,487 of 2023-01's winnerless award notices carry exactly this, and nothing else:

    <ns5:TenderResult><ns3:AwardDate>2023-01-04+01:00</ns3:AwardDate><ns3:AwardTime>10:00:00+01:00</ns3:AwardTime></ns5:TenderResult>

`read_sdk01_results` synthesised a decision whenever the publisher stated none:

    if r.decision.is_none() {
        r.decision = Some(if r.direct_winners.is_empty() { "clos-nw" } else { "selec-w" }.to_owned());
    }

`clos-nw` is documented to the SQL sandbox as **"closed, no award"** — a positive assertion. So on
roughly 125,000 sdk-0.1 notices we asserted that no contract was awarded, on notices whose only
published content is *the day the contract was awarded*. The winner gap is disclosure; this was
fabrication, and it is the more serious of the two.

The cross-tab that settles it (2023-01, one row per award notice):

    2,487  no TenderResultCode, no WinningParty   → was clos-nw   ← the fabricated 86 %
      387  selec-w + winner named                 → correct
       16  no-rece (no tenders received), none     → correctly no winner, publisher SAID so
        5  no-rece + winner named                  → publisher's own inconsistency

Note the 16: when this publisher does mean "nothing was awarded", it says so with a code. The silence
is not a quiet `clos-nw`; it is silence.

## Fixed

1. **`SDK01-TenderResult-AwardDate` now projects** into the result's decision date
   (`tender_version_lot_results.decided_*`), beside issue 255's `TED-CONTRACT_AWARD_DATE` (legacy) and
   `BT-1451` (eForms) arms. The field was claimed by the parse layer and dropped by the projection.
   `AwardTime` is deliberately not merged — the day is the fact anyone reads.
2. **The `clos-nw` half of the fallback is gone.** A named winner still infers `selec-w` (that half was
   always justified); an unstated status stays NULL.

Gates: `an_sdk01_result_that_states_only_a_date_claims_no_award_decision` (falsified — restoring the
old fallback fails it with `left: 0, right: 1`) and `a_named_sdk01_winner_is_still_read_as_a_selection`,
which pins the half that must survive. New fixture `doe/sdk-0.1-can-awarddate-only-19191760-1.xml`,
the real 2023-01 payload.

## One methodological note worth keeping

My first pass at this measurement grepped for `cac:TenderResult` and concluded 86 % of award notices
had no result block at all — a "finding" that flatly contradicted the report. The report was right and
I was wrong: this dialect serialises with **generated prefixes** (`ns5:TenderResult`), which the
fixtures README has documented since the numeric-channel fixture landed ("fully prefix-mangled
namespaces… the hardest namespace case in the corpus"). The parser resolves namespace URIs and was
never fooled; my grep was. Recorded because the near-miss is instructive: a probe that contradicts a
measured number is more likely to be a broken probe than a broken number, and the check that caught it
was reading one payload end to end instead of trusting the aggregate.

## Still open

The winner gap itself stays a **disclosure**, not a defect — nothing to extract. Section 3's `named`
column (issue 101) is now the place it is visible. Two follow-ups worth considering, neither filed as
work yet:

- The `named` rate divides by every materialised result, including the ones that correctly have no
  winner (`no-rece`, and now the NULL-decision ones). That is the same double-counting the column was
  careful to avoid one place to the left. A `decision`-aware denominator would read truer.
- ~125k results now carry a decision date and a NULL decision. Whether an AwardDate with no code should
  infer `selec-w` is a real question — it is evidence an award happened, but not evidence of who. Left
  as NULL deliberately: the date is the fact, and the inference is not ours to make.

## What it takes to land on prod

The code change alone moves nothing: `decision` and `decided_*` are written by the fold, so the
~125,000 rows keep their fabricated `clos-nw` and their missing date until the sdk-0.1 era is
re-projected. The parse layer already holds `SDK01-TenderResult-AwardDate` (it is an SDK field and has
been claimed all along — only the projection dropped it), so this needs a **refold, not a re-parse**,
which is the cheaper of the two.

That refold is already owed to issue 231's acceptance for the same era, so the two should ride
together rather than paying the cost twice. Order: deploy after job 288 drains → sdk-0.1 refold →
re-read section 3, where `named` should be unchanged (the winner counts do not move) while the
`clos-nw` population collapses to NULL. If `named` moves, something else changed and I should find out
what before believing either number.

Verification after the refold, as a bounded SELECT rather than a re-derivation:

    -- was ~125k, must go to ~0 for this era
    SELECT COUNT(*) FROM tender_version_lot_results r
      JOIN tender_versions v ON v.tender_id = r.tender_id AND v.seq = r.seq
      JOIN notices n ON n.id = v.caused_by_notice_id
     WHERE n.profile = 'eforms:eforms-sdk-0.1' AND r.decision = 'clos-nw';

    -- and the same population must now carry a date
    SELECT COUNT(*) FROM … AND r.decision IS NULL AND r.decided_utc IS NOT NULL;
