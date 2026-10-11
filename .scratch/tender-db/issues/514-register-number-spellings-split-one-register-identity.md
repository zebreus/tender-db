# 514 — spellings of one register number (label text, linked-charity suffixes) split one registry identity

Status: needs-triage — filed 2026-10-11 from issue 469's unit-2 decision (decision 4: spelling repair stays out of
the 469 arm).
Kind: data quality (organization identity)
Relates to: 469, 470 (the GB O/0 company-number lookalike repair, the precedent)

## What is wrong

FTS register literals carry label pollution (`GBUKPRNUKPRN10007798`, `GBCHCCHARITYNUMBER216250`) and glued
linked-charity suffixes (`216250-1`), so one register number can mint several identities. 469's arm refuses them
(its shape gate) rather than repairing them.

## Proposed unit

Measure the spellings per scheme on the 469 dry plan's literal shapes (U0), then decide a per-scheme canonical form
behind the name gate, as 470 did for GB company numbers.
