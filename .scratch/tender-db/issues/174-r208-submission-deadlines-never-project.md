# 174 — r208-era submission deadlines are parsed but may never project

Status: open — needs one bounded canonical-side check, then a mapper fix + refold
Role: run-driver
Severity: MEDIUM (silent per-era data gap on a headline canonical field)

Found by the 2026-08-09 data-profile study (docs/research/data-profile-2026-08.md
§2.4). The r208 2014 notice window carries submission deadlines as
`TED-DT_DATE_FOR_SUBMISSION` (262 rows) and `TED-RECEIPT_LIMIT_DATE` (203 rows)
and has ZERO `TED-DATE_RECEIPT_TENDERS` rows — but `project.rs::DATES` maps
only the latter to `submission_deadline` (exact/stem match on canonical_name).
If nothing else re-routes those two field ids, every r208-era Tender
(2011–2016, ~2M notices) has a parsed-but-unprojected deadline, and any
"deadline fill per era" completeness metric under-reports the era.

Check first (bounded): count tender_versions with NULL submission_deadline
whose causing notices are r208-era vs r209-era. If confirmed: add the two
field ids to the DATES mapping and refold the affected era (issue-91 scoped
refold machinery exists). The era-profile checklists (ted-legacy-mapping
§8.3) should have caught this — worth asking why the r208 checklist marked
deadline as mapped.

2026-08-09 ~10:15 Berlin (orchestrator): verification path changed. The
bounded canonical-side prod check was ABANDONED after it stacked five 408s
— even index-driven shapes (tender_versions.published_at BETWEEN one-day
window + per-row EXISTS) blow the 10s cap; the planner evidently doesn't
serve them the way the schema suggests, and learning planner behavior
against the serving box is issue-167's rig work, not this issue's. (One of
the five was my own carelessness: a quoting probe with the window dropped
— full scan. Budget lesson from the data-profile study re-confirmed the
hard way.) The proof is instead the house-standard failing fixture test:
r208/f02-000333-2014.xml carries RECEIPT_LIMIT_DATE 07/02/2014 17:00; new
test an_r208_contract_notice_projects_its_submission_deadline asserts it
projects. Decision on scope: map TED-RECEIPT_LIMIT_DATE (form section)
only — the coded TED-DT_DATE_FOR_SUBMISSION stays unprojected in BOTH
eras, mirroring r209's existing form-wins convention and avoiding
divergent duplicate facts (facts dedupe by identical value only; no
uniqueness on (tender,seq,field)).

2026-08-09 ~10:30 Berlin (orchestrator): the "why didn't the checklist catch
it" question is answered — every_r208_fixture_is_consumed_exhaustively and
the era checklists gate PARSE coverage (nothing dropped reading the XML);
nothing gates PROJECTION coverage (parsed field id -> canonical field). The
parse->projection seam is structurally unguarded, so any era whose element
NAMES differ from the era the DATES/TEXTS/AMOUNTS tables were written
against can silently lose canonical fields. Follow-up worth its own issue
after this fix: a per-era "headline fields project" matrix test (each era
fixture asserts title/deadline/value/cpv reach the canonical layer when the
fixture demonstrably carries them) — the new r208 deadline test is the
first instance of the class.
