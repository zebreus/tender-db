# 174 — r208-era submission deadlines are parsed but may never project

Status: RESOLVED-VERIFIED 2026-08-09. Fix on main (red->green fixture test, PROJECTION_EPOCH 2), full-corpus era refold completed on the new box (14.2M notices -> 7.9M tenders, 6.5h), /health/deep green, and an r208-era tender (id 5000000, 2012) confirmed serving its submission_deadline via the public API. Detail at the bottom of this file.
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

2026-08-09 14:25 Berlin (orchestrator): scheduled verification check-in fired,
but the box was deliberately stopped ~13:37 Berlin for a machine migration
(Lennart is moving prod to bigger hardware). State at stop:

- Refold mapping pass (job 592, "refold ted-export-r208"): FINISHED OK at
  ~13:12 Berlin — re-queued 2,694,814 notices for the incremental projection.
- Follow-up projection (job 2, project rebuild=false): INTERRUPTED mid-run by
  the planned stop, ~85 min in. Not a failure: chunk commits are durable, the
  durable job row survives, and supervisor recovery resumes it automatically
  on first startup on the new machine.
- WAL was checkpoint-TRUNCATEd to zero and folded into the main file post-stop
  (460,357,677,056 bytes, self-contained) for the transfer; service+timers
  stopped and disabled on the old box.

Verification checklist (deadline density per era, r208 spot-check via
/v1/tenders/{id}, /health/deep) is DEFERRED until the new machine is up and
the resumed projection completes. Issue stays open at resolved-pending-
verification: the mapping fix and its fixture test are merged; only the prod
refold verification remains.

2026-08-09 22:25 Berlin (orchestrator): the era refold COMPLETED on the new
box — `[project] done: 14242418 notices → 7924166 tenders (700353 islands),
14242418 versions, 63220835 change rows in 23510.3s` (~6.5 h; phase 2
accelerated from ~13k to ~41k tenders/min as buckets warmed). WAL
checkpointed to 0 MB after the end-of-run index builds. `/health/deep` fully
green afterwards: canonical layer measured and non-empty, last_job
`project rebuild=false` outcome ok, ingest fresh.

Verification state — ALL DONE, issue RESOLVED-VERIFIED 2026-08-09 ~23:05
Berlin:
- /health/deep: green post-refold AND green again post-deploy of b56ce13,
  reporting the new rev.
- r208 spot-check: DONE. Bisect via /v1/tenders/{id} point lookups landed
  on id 5000000 — published 2012-11-21 (squarely in the r208 era) with
  submission_deadline 2012-09-10T23:59:00+00:00 served through the public
  API. (Deadline predating published_at is expected on aggregated tender
  docs: published_at reflects the latest notice version, e.g. an award
  published after the deadline closed.) Neighbouring probes: 4000000
  (2008, deadline set), 4500000 (2010, deadline set), 5500000 (2014,
  deadline None — plausible genuine absence, e.g. award-only tender).
- Deadline density per era: remains ABANDONED on-box (408 history, no
  compliant path); the follow-up per-era "headline fields project" matrix
  test is the durable guard instead.
