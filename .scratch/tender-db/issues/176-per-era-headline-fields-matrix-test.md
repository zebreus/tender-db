# 176 — a per-era "headline fields project" matrix test

Status: implemented same evening (2026-08-09, orchestrator) — test
`every_era_projects_its_headline_fields` in ingest/tests/project.rs. Value
column deliberately EXCLUDED, see issue 177 (found while building this).
Kind: verification hardening (the issue-174 follow-up)
Blocked by: —
Relates to: 174 (the motivating loss), 177 (found by this), 104 (golden fixture)

## Why

Issue 174's post-mortem: parse coverage is exhaustively gated per era
(ADR-0002/0004 harnesses), but NOTHING gates projection coverage — a parsed
field becomes canonical only if the projection's mapping tables know its era
name, and the r208 era lost `submission_deadline` for 2.7M notices silently
because `RECEIPT_LIMIT_DATE` wasn't in DATES. Any era whose element names
differ from the era the tables were written against can silently lose
canonical fields. The r208 deadline test was the first instance of the class;
this matrix is the class.

## What it asserts

One real CN-family fixture per era; for each headline field the fixture
DEMONSTRABLY carries (verified against the raw fixture bytes, listed in the
test), the canonical layer must carry the fact after projection:

| era | fixture | title | deadline | cpv |
|---|---|---|---|---|
| eForms EU | eforms/cn-16-00494343-2026.xml | x | x | x |
| eForms-DE 1.x | doe/eforms-de-1.1-cn-7d69b0f7.xml | x | x | x |
| DÖE sdk-0.1 | doe/sdk-0.1-numeric-cn-25599482-1.xml | x | x | (no cpv in fixture) |
| r209 | r209/f02-000245-2019.xml | x | x | x |
| r208 | r208/f02-000333-2014.xml | x | x | x |
| internal-OJS 2008 | internal_ojs/115908_2008.en | x | x | x |
| text era | text/2008-cn-723-2008.txt | x | x | x |

eForms-DE 2.x is not a row: its projection mapping is the same BT-level table
the EU row exercises, and the committed DE-2.1 fixture is a CAN (no deadline).

## The value column is issue 177

Estimated/total values were meant to be the fourth column and turned out to be
a live 174-class loss with a genuinely ambiguous mapping (r208's `VALUE_COST`
means estimate, award value or final total depending on container and form) —
too consequential to patch inline. The matrix ships without the column; 177
carries the full analysis and adds the column when it lands.
