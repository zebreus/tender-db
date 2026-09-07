# 363 — `FONR01446821`: the Swedish "FO-nummer" label glued to Åland Y-tunnus rows

Status: ready-for-agent (filed 2026-09-07 by the owner from the issue-358 unit-2 listing)
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
