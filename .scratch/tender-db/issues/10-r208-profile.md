# 10 — TED_EXPORT R2.0.8 profile (incl. R2.0.7, defence)

Status: claimed
Blocked by: 09

Goal: 2011–2016 (and defence forms wherever they appear) parse and project.

Scope: R2.0.8 parser variant (thinner fields, OTH_NOT prose corrigenda as
version events without typed diffs, defence forms); absorb R2.0.7 delta
empirically (XSD hunt was inconclusive — derive from real files, quarantine
surprises); own completeness inventory; fixtures from 2011/2014 packages.

Acceptance: a 2014 daily ingests cleanly; era checklist green.

## Comments

parser done in worktree (parse half; projection is issue 04's machinery)

- Design: extension of the r209 module, not a sibling — `crates/ingest/src/r209/`
  now parses the whole TED_EXPORT family (`ted-export-r209` and
  `ted-export-r208` incl. R2.0.7). One walker, one rule registry: element
  names are globally unique across the grammars, and every R2.0.8 idiom
  (D/M/Y dates, FMTVAL amounts, IDEM markers) was already in place for the
  defence forms. New machinery: `Rule::TextGroup` (R2.0.7 inline
  `ORGANISATION` text), block-aware prose rows (OTH_NOT btx vocabulary),
  Section-level FMTVAL amounts, and a degrade-to-raw-text path for junk
  values in money/number/count slots (research §8.2: quarantine is for
  unconsumed structure, not low-quality values — "10 000 per laureaat" is
  kept as text, measured on real dailies).
- Era checklist: `crates/ingest/sdk/ted-export-inventory.json` (replaces
  r209-inventory.json; 1314 elements = R2.0.9 S01+S05 + R2.0.8 S03+S05
  unions + the empirically-mined `r208-observed` names). The R2.0.7 delta
  turned out tiny: only 5 element names (misspelled `CONTACTING_*` variants,
  `CHOICES_NEGOTIATED_PROCEDURE_WITHOUT_COMPETITION`, `NO_RESERVED_CONTRACTS`)
  plus 7 `@VALUE`-attribute variants appear in real 2011–2019 dailies beyond
  the XSDs. OTH_NOT/EEIG bodies map as declared text (prose version events).
- Verified on the VPS (scratch db, `/opt/tender-db/src-era10/`), five real
  dailies, all with zero quarantines / zero pending / zero skipped and
  idempotent re-runs (0 new rows):
  - 2011-01-04: 1817/1817 parsed, all `ted-export-r208` (R2.0.7.S03 —
    F03 853, F02 533, OTH_NOT 177, utilities + concession tail).
  - 2014-01-02: 1156/1156 parsed (F03 536, F02 326, OTH_NOT 162, F15 37…).
  - 2015-01-02: 1294/1294; 2016-01-04: 1102/1102 (1090 r208 + 12 early r209).
  - 2019-01-02 re-run: 1529/1529 accounted — 1372 r209 + 157 r208; the 147
    previously-pending standard-form files (F02 63, F03 41, OTH_NOT 29,
    F05/F06 11, F01 2, F13 1) now parse next to the 10 defence forms.
  - Value-junk degrade path fired exactly 11 times across 6898 notices
    (prose in money/count/duration slots), each kept as a raw text row.
- Root-cause fix that surfaced on real 2014 data: `eforms::value::timestamp`
  panicked (slice out of range) on one-component clock strings ("12.00" era
  times); now an Err like every other malformed lexical.

