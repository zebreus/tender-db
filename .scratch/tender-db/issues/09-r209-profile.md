# 09 — TED_EXPORT R2.0.9 mapping profile

Status: claimed
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

## Comments

parser done in worktree, projection integration pending merge

- `crates/ingest/src/r209/` parses TED_EXPORT R2.0.9 (all forms F01-F25 +
  MOVE T01/T02) plus the R2.0.8-grammar defence forms F16-F19 into the
  notice-parsed layer; era checklist vendored as
  `crates/ingest/sdk/r209-inventory.json` (739 elements from the mirrored
  XSDs), completeness tests enforce rules<->inventory both ways.
- Verified on the real 2019-01-02 daily: 1529/1529 members accounted,
  1382 parsed (1372 r209 + 10 defence), zero quarantines of any kind;
  147 non-defence R2.0.8 standard-form files stay pending for issue 10
  (surprise: they are ~10% of a 2019 daily). 2017/2018 dailies also parse
  clean after an S01/S02 `<VALUE>`-wrapper fix.
- Chain edges are stored as is_ref id rows (`TED-REF_NOTICE.NO_DOC_OJS`,
  `TED-NOTICE_NUMBER_OJ`); union-find grouping + F14 version events are the
  projection half, blocked on issue 04's merge.
- NATIONALID normalization/plausibility gate lives in `ingest::orgid`.
