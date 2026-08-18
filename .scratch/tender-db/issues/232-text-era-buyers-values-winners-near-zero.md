# 232 — the text era projects titles but almost no buyers, values or winners (3.79M versions)

Status: needs-triage — measured 2026-08-18 against prod (job 731, rev `a79540e`)
Kind: projection mapping gap, largest single era by volume
Blocked by: —
Relates to: 11 (the text-era profile), 176 (the per-era headline-fields matrix — fixture-level,
which is why this never showed there), 172 (codelist/currency drift, same era), 230 (the
measurement that found this), 187 (INTERNAL_OJS award linkage — the neighbouring island)

## What

The first full-corpus data-quality measurement (job 731, 32 id windows, 1258 s) puts the text
era at **3,786,955 tender-versions — the largest era in the corpus, ~27 % of all versions** —
and reports:

    era                    versions   title  buyer  value    cpv deadline winner
    text 1993–2010        3,786,955  100.0%   0.5%   0.1%  95.5%    84.4%   0.1%

Title 100 %, CPV 95.5 %, deadline 84.4 % — the era projects well on three of six fields. Buyer
**0.5 %**, value **0.1 %**, winner **0.1 %**.

## Why this is a finding and not just "old data is thin"

A 1993–2010 OJ tagged-text notice names its contracting authority — that is the point of the
publication. So "the buyer isn't in the source" is not a plausible explanation for 99.5 % of
3.79M notices, and it should not be assumed to be one. Compare the neighbouring islands in the
same report: `INTERNAL_OJS 2008` measures **91.5 % buyer** on the same kind of legacy content,
and `TED_EXPORT r2.0.8` measures 98.4 %. Whatever the text-era parser or projection does with
authority names, the eras on either side of it do something different and better.

The two candidate causes, and they are distinguishable:
1. **Parse side** — the text-era parser does not emit an authority/party section (or emits it
   under a stem the projection's `role_name` does not recognise), so no `organization_mention`
   is ever seeded. Check `notice_sections` for a text-era notice directly.
2. **Projection side** — the sections exist but `TEXTS`/`AMOUNTS`/`role_name` lack the text-era
   (`TXT-*`) stems for party, amount and winner, exactly as issue 177 found for r208 values and
   issue 29 found for sdk-0.1. Grep the mapping tables for `TXT-` coverage per field.

Start by reading ONE text-era notice end to end — raw archive bytes → `notice_sections` →
`organization_mentions` → `tender_version_parties`. Whichever layer the buyer disappears at
names the cause, and one notice is enough to name it.

## Why it hid

Issue 176's per-era matrix test asserts the headline fields for one fixture per era, and it
passes: a text-era fixture that DOES carry its fields projects them. That test proves the path
works for the fixture chosen; it says nothing about the share of 3.79M real notices that take
that path. Corpus-scale completeness is a different question and, until issue 230, nothing
could ask it — every query timed out. This is the first answer.

## Impact

Buyer rollups, buyer search and any authority-level analytics silently exclude the largest era
in the corpus — a user filtering by contracting authority sees 1993–2010 as nearly empty rather
than as unmapped. That is the same class of harm as issue 29's island Tenders, at five times the
volume.

## Acceptance

- The layer where the buyer is lost is identified from one traced notice and recorded here.
- The mapping (parse or projection, whichever it is) is fixed and unit-tested per the era.
- The era is re-projected and the data-quality report shows non-trivial `buyer`.
- `value` and `winner` are judged separately and honestly: pre-1999 amounts are national
  currencies (issue 172) and most text-era notices are not award notices, so those two columns
  need the right denominators before anyone calls them bugs. Section 3 of the report (results
  materialisation, measurable from rev `c731a05`) is where the winner question belongs.
