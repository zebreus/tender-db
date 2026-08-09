# 167 — capacity/abuse model for the public surface

Status: open — research gap #1 (docs/research/research-gaps-2026-08.md), HARD PRE-LAUNCH
Role: run-driver (measurement campaign needs team-lead's word per prod-box-reads)

Eleven issues (17, 25, 55, 61, 70, 89, 115, 117, 120, 121, 122, 163) fixed
expensive-read problems one query at a time; nobody has measured what one
4-vCPU box actually sustains. Needed: per-endpoint worst-case cost, SSE
fan-out cost curve (predicate evaluation x subscriptions x change rate),
hostile-SQL saturation behavior, importer-vs-API contention — on a
prod-shaped copy, yielding a written capacity budget the rate-limit numbers
derive from. Context: queries cannot be interrupted once started (issue 120
measured it); 8 readers are the whole budget; the monolith cannot grow a
second reader process (ADR-0005 + turso single-process).

2026-08-09 (orchestrator): the research half is done —
docs/research/capacity-model-v0.md assembles every existing measurement
(issue 120's cancellation reality, tonight's per-endpoint numbers, the SSE
cost shape, the structural single-process ceiling) and specifies the
measurement campaign (rig, 4 experiments, deliverable, ~2 days). What
remains is EXECUTION, gated on Lennart: authorization for a prod-shaped
copy per docs/agents/prod-box-reads.md, plus the scratch-hardware cost
call. Queued as a bulk question.
