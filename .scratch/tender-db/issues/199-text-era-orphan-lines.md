# 199 — text-era orphan lines: 49 unclaimed-content rows of stray record text

Status: needs-triage
Kind: text-era parser decision (keep vs fix)
Relates to: 195 (spun off at its close), 144 (text-era waves), 183 (attribution)

## What (measured via /v1/sql, 2026-08-14)

49 outstanding `unclaimed-content` rows, profile `text`, detail `line NN: <content>` —
free-text lines the text-era parser could not attach to a field. Two visible families:
real content (deadline lines "Schlußtermin für Angebotseingang: 2.11.1999", "Non sono
pervenute offerte" no-tenders notes, address continuations) and terminal garbage
(".S F=ALL;R=3746 TO 1;SORT=PD;ND;HC" control strings, bare ")" fragments). Per-row
member extraction needed to decide keep-vs-fix per family — the garbage family is a
documented-keep candidate; the deadline/no-tenders lines may deserve a continuation rule.

## Next

One query grouping by line-content shape, then extract 2-3 members per family.
