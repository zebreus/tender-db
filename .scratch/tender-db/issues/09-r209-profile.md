# 09 — TED_EXPORT R2.0.9 mapping profile

Status: ready-for-agent
Blocked by: 04

Goal: the 2016–2024 era parses and projects — legacy chains become Tenders.

Scope:
- Parser for TED_EXPORT R2.0.9 per docs/research/ted-legacy-mapping.md:
  CODED_DATA_SECTION fully mapped, form sections for F02/F03/F14/F20 first,
  then remaining forms; OJS-number chain edges (REF_NOTICE) into the
  projection's union-find grouping; F14 typed diffs as version events;
  legacy-only elements (~23) into a legacy satellite; era-scoped
  completeness checklist from the R2.0.9 XSD element inventory (vendored).
- NATIONALID normalization + plausibility gate for org mentions.
- Fixtures: real notices per form type from the archived samples.

Acceptance: a 2019 daily package ingests end-to-end with chains forming
multi-notice Tenders at the measured linkage rates; checklist green;
unchained-award metric populated.
