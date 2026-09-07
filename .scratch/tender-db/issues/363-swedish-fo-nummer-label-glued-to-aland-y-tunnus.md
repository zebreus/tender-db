# 363 — `FONR01446821`: the Swedish "FO-nummer" label glued to Åland Y-tunnus rows

Status: BUILT 2026-09-07 (owner) — vocabulary + shape rule + tests; gate/deploy/repair recorded below. The class is ~4× the AX finding: 432 labelled FI rows, 355 with a bare twin standing. Was: ready-for-agent (filed 2026-09-07 from the issue-358 unit-2 listing)
Kind: defect (organization layer — identifier normalisation, the issue-328/359 label class)
Relates to: 328 (label prefixes in front of the identifier), 359 (the PL/IT/ES vocabulary
extension and the CIF-shape guard), 358 (whose unit 2 moves these rows to `FI`)

## What the listing showed

Of the 81 identifier rows under `AX` that issue 358's unit 2 moves to `FI`, 16 carry the
Swedish-language label of the Finnish business id glued to the number: 12 × `FONR…`, 2 ×
`FONUMMER…`, 2 × `FO…` (FO-nummer = företags- och organisationsnummer = Y-tunnus). Åland
publishes in Swedish — and `countries::LABEL_PREFIXES` carries no Finnish label in ANY
language (no `YTUNNUS`, no `FONUMMER`; the list is DE/IT/PL/ES), so nothing fires. After the move these rows stand as
`(FI, national, FONR01446821)` beside the bare `(FI, national, 01446821)` twin — the
label splits an organization from its own correctly-formed row, which is exactly issue
328's class, and neither the resolver's exact triple nor the crosswalk's `FI:ytunnus` arm
(digits only) reunites them.

Mainland-Finland rows may carry the same label (Swedish-speaking municipalities publish in
Swedish too); the listing only looked at `AX`. Measure with a bounded read on the `FI`
rows: `identifier LIKE 'FONR%' OR identifier LIKE 'FONUMMER%'`.

## The unit

1. Add `FONUMMER`, `FONR` — and `YTUNNUS` for the Finnish spelling, measured first on the
   `FI` rows — to `countries::LABEL_PREFIXES` (3+ letters, prefix alone suffices under the
   existing rule; longest first, the table's ordering invariant). `FO` is a two-letter tag: only with the
   existing two-letter discipline (followed by a digit) — and note `FO` is also the Faroe
   Islands' code, which is why it must not reach the VAT sniffer; the register-prefix arm
   runs first, so check the order in `normalise_identifier_with`.
2. The strip-then-revalidate guard (issue 328) accepts the remainder when it classifies:
   an 8-digit remainder under `FI` is `FI:ytunnus` — HARD checksum, so a mistyped one is
   refused rather than mangled. Test both.
3. Run `repair-label-prefixes` dry → wet (issue 359's path) so the standing rows catch up;
   then R2 folds the reunited pairs.

## Measured (2026-09-07, bounded reads on the `FI` national rows, after 358 folded AX in)

| label | rows | with a bare twin standing |
|---|---|---|
| `Y` + digits (`Y 0123456-7` → `Y01234567`) | 266 | 213 |
| `YTUNNUS…` | 149 | 129 |
| `FONR…` | 12 | 9 |
| `FONUMMER…` | 3 | 3 |
| `BUSINESSID…` | 2 | 1 |
| **total** | **432** | **355** |

Not taken: `YT` + digits (1 row), `YTUNNUJ…` (2, a typo), `UUDELY1…` (12 — an ELY-centre
label, not an id), `RYHM600…` (4). The twin rate (82 %) is the 328 shape exactly: the label
splits an organization from its own correctly-formed row.

## Built (2026-09-07)

- `countries::LABEL_PREFIXES` gains `YTUNNUS`, `FONUMMER`, `FONR`, `BUSINESSID` (the table is
  re-sorted longest-first; the ordering test asserts it).
- The bare `Y` is a SHAPE RULE in `label_prefix_stripped`, not a table entry: `Y` followed by
  seven or eight digits and nothing else. A one-letter table entry would match every Y-led
  word; a Spanish NIE (`Y7395817K`) ends in its check letter and never matches; `YT22493`
  and `YMPARISTO…` keep their Y. The caller's re-validation still decides: under `FI` the
  remainder meets the HARD Y-tunnus checksum, so `Y01274856` (bad check digit) stays as
  published rather than being mangled — pinned in
  `finnish_labels_come_off_and_the_y_tunnus_is_checked` (project.rs) and
  `the_bare_y_strips_only_by_shape` (countries.rs).
- The repair path is unchanged: `repair-label-prefixes` walks every identifier row through
  the injected `label_prefix_stripped`, so the shape rule is picked up by the same job.
