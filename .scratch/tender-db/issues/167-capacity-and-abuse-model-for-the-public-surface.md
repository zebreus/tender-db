# 167 — capacity/abuse model for the public surface

Status: research half DONE 2026-08-24 (on-box campaign, capacity-measured-2026-08.md); the FIX work is issue 273, then a rate-limit change
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

## Rig decision (2026-08-23, owner) — on-box, aggressive, no money spent

Lennart delegated the rig call with a blanket risk acceptance ("be aggressive; it's fine if
prod crashes") and asked not to be asked again. Owner decision: the campaign runs ON the
production box — no scratch hardware. Concretely:

- The four experiments from capacity-model-v0.md run against the LIVE surface in quiet windows
  (pre-dawn, clear of the 09:35 daily): hostile-SQL saturation via /v1/sql, SSE fan-out cost
  curve with synthetic subscriptions, per-endpoint worst-case timing, importer-vs-API
  contention measured during a real fold.
- Reads that would gate on prod-box-reads.md are covered for THIS campaign by Lennart's
  2026-08-23 blanket word (recorded here); the standing gate stays for future cases.
- Degraded service during measurement windows is accepted; the deliverable is the written
  capacity budget the rate limits derive from.

Execution starts once the slice-9 close-out queue (jobs 363–365) drains.

## Campaign run 2026-08-24 (on-box) — research half complete

Ran E1 (per-endpoint worst case), E1-redo (filter vocabularies), E2 (SSE snapshot +
the walk pool), E3 (hostile-SQL swarm). Full log + the derived capacity budget in
docs/research/capacity-measured-2026-08.md. Headlines:
- Three cost classes spanning 5 orders of magnitude; rate limiting must be per-SHAPE.
- SQL surface is safe (10s budget fires, freed reader, 2-gate sheds, REST-isolated).
- **The scarce resource is the 4-slot walk pool × hold time** — not any per-IP number.
- **Real bug found: issue 273** — `status=open` + a low-volume country walks to the 30s
  bound → 503, uncancellable, so 4 trivial requests brown out all walk traffic.
- Hardware corrected: 32 vCPU / 62 GB, so the 4-slot pool is policy not cores.

Remaining: 168's identifier false-merge/split RATES + fake-country inventory (bounded
reads, same on-box authorization); E4 mixed-soak (confirmatory); and the rate-limit
code change, which is gated on issue 273's fix landing first.
