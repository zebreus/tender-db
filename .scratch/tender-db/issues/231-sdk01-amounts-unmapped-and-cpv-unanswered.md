# 231 — sdk-0.1 amounts are never mapped, and whether the era carries CPV at all is unanswered

Status: needs-triage — measured 2026-08-18 against prod (job 731, rev `a79540e`)
Kind: projection mapping gap (one era, two fields) + one research question
Blocked by: —
Relates to: 29 (the parent gap, now verified closed for title/buyer/deadline), 177 (the same
shape one era over: r208 values never projected), 172 (codelist drift across eras), 230 (the
measurement that found this)

## What

Split out of issue 29, whose broad claim ("sdk-0.1 measures 0 % on every field") is no longer
true. Measured over the whole era — **666,671 tender-versions**, not a fixture:

    era                     versions   title  buyer  value    cpv deadline winner
    DÖE sdk-0.1 island       666,671  100.0% 100.0%   0.0%   0.0%    77.9%   1.1%

Two fields remain on the floor and they are different kinds of problem.

## 1. value 0.0 % — a mapping gap, and the cause is already visible in the code

Issue 29's fix comment lists "value fields" among what it repaired, but the changes it then
enumerates touch `TEXTS`, `CLASSIFICATIONS` and `DATES` only. `AMOUNTS` never gained an
`SDK01-*` entry, so sdk-0.1 amounts have never had a canonical destination. 0.0 % is therefore
expected from the code as written — this is not a regression and not a mystery, it is an
unfinished mapping.

Do the archive inventory first (which `SDK01-*` field ids actually carry monetary values, and
at which scope — Tender, Lot, or result), then add the `AMOUNTS` entries, then re-project the
era and re-measure. Issue 177 is the template: it was exactly this, for r208.

## 2. cpv 0.0 % — do NOT map anything until the presence question is answered

Issue 29 mapped `RealizedLocation` NUTS → `place`. That is the *place* classification; CPV was
never in scope. Before any mapping work, answer from the archive: **do sdk-0.1 payloads carry a
CPV code at all?** An era that does not publish CPV is a 0 % that is simply TRUE, and mapping
effort spent against it is spent against nothing.

What the same report says about the neighbours, so the obvious wrong inference is closed off:
`eforms-de-1.1` through `eforms-de-2.1` all measure **100.0 % CPV** over 443k versions
combined. "German notices don't carry CPV" is not the explanation.

## What NOT to conclude from the winner column

`winner 1.1 %` is measured over ALL versions, and most sdk-0.1 versions are contract notices
with no winner to carry. That number cannot separate "winners are lost" from "few of these are
award notices", so it must not be used as evidence either way. The denominator that can is
award notices only — section 3 of the data-quality report (results materialisation), which
job 731 could not measure and which rev `c731a05` onward does. Judge sdk-0.1 winners there.

## Acceptance

- An archive inventory recorded here: which `SDK01-*` ids carry amounts, and whether any carry
  a CPV code.
- `AMOUNTS` mapped for whichever amount ids exist; era re-projected.
- The data-quality report's `DÖE sdk-0.1 island` row shows non-trivial `value`.
- For CPV: either mapped and non-trivial, OR a recorded finding that the era does not publish
  CPV — in which case the 0 % is documented as correct rather than left looking like a bug.
