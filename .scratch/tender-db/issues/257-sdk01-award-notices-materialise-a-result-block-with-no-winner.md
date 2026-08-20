# 257 — sdk-0.1 award notices materialise a result block and ~95 % of them name nobody

Status: MEASURED 2026-08-20 from the stored data-quality report, not guessed. Not yet diagnosed — the
next step is to read one sdk-0.1 CAN's result graph end to end and find where the winner is lost
Kind: extraction/resolution gap, one dialect, ~139k award notices
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
