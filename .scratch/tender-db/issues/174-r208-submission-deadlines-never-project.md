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
