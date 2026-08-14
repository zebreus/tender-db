# 199 — text-era orphan lines: 49 unclaimed-content rows of stray record text

Status: RESOLVED (2026-08-15 — see bottom)
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

**2026-08-14 (orchestrator).** +1 row: the issue-200 reprocess re-held one merged 2010
record under its true reason — an in-record line the parser can't attach. Population now
~50.

**2026-08-14 ~20:4x CEST box time (orchestrator) — DIAGNOSED + FIX LANDED (pending
deploy).** Extracted representatives of every family from the archive (monthlies, exact
record splits). Finding: every one of the 60 rows is a line at COLUMN 0 (the parser's
unclaimed branch), and all but 4 are TED's own line-wrapper emitting a wrapped tail
flush-left — after a '!' in the text ('nicht öffnen ! ' → '" zu versehen…'), a closing
paren ('(EXCLUDING FOOTWEAR OF RUBBER OR OF WOOD' → ')'), a fixed-width mid-word sever
('TELECOMMUNICATI' → 'ON'), an indent defect on per-line fields (RC: ES511 → 'ES512',
RG: BARCELONA → 'GIRONA' — real values), plus the correction margin marker '!' and 2010
column-0 boilerplate/justifications. The other 4 are 1994 mainframe search-command echoes
('.S F=ALL;R=NNNN TO 1;SORT=PD;ND;HC') between fields — production residue. Fix bd6a5bf:
column-0 non-tag line continues the open field (margin marker stripped; per-line → own
value; scalar still rejects via the flush guard; no-field-open still holds); the echo
shape is consumed as layout. Tests pin all families. Next: deploy + reprocess
unclaimed-content (expect 58 text rows reclaimed; the 2 parked r208 rows re-hold), then
ledger entry naming the wrap-artifact class.

**2026-08-15 ~00:1x CEST box time (orchestrator) — RESOLVED.** Deploy a9c042f; reprocess
job 691: 41 packages, 60 reclaimed, 0 still held (the 58 text rows AND issue-194's 2 F19
rows in one pass), zero stamp anomalies. unclaimed-content outstanding is now 0 — the
reason is EMPTY for the first time. Ledger entries "Text-era column-0 wrap artifacts"
(profile text, line %) and "F19 concession award-criteria sentence" added (45 total,
render on next deploy). Trailing old-era fold running; panel verification next firing.
