# Pending decisions — the Lennart queue

The decisions only the team lead can make, assembled from the issues that name them
(167, 168, 170, 173). Each has its study DONE and a recommendation attached: a
one-word answer unblocks each line. Maintained by the owner; delete an entry when
its decision lands in the issue it came from.

## 1. Measurement rig for the capacity + identifier studies (issues 167, 168)

Both pre-launch studies are written (capacity-model-v0.md; data-profile-2026-08.md §1)
and both stop at the same gate: the remaining experiments need a **prod-shaped copy**
per docs/agents/prod-box-reads.md — hostile-SQL saturation, SSE fan-out curves, and
the org false-merge/false-split rates are exactly the reads that must never run
against the serving DB. A DB snapshot already exists on-box (issue 269's timer).

**Ask:** (a) your word for the campaign against a copy, and (b) the hardware call —
rent a scratch box for ~2 days (est. €10–40 one-off) or run on the prod box only in
quiet windows (slower, contends with the pipeline).
**Recommendation:** the scratch box; the campaign then runs unattended and the
serving box never feels it.

## 2. DR re-decision (issue 170)

The 2026-07-19 "no off-box backups, rebuildable in a day" premise is corrected by
the study (dr-premise-2026-08.md): honest RTO is ~1–1.5 d for a layer rebuild and
~4–6 d for DB/volume loss, and there is now **<1 MB of genuinely unrebuildable state
(users, API tokens, webhook registrations, cursor/epoch continuity) with zero copies
anywhere**. A lost DB also currently forces a full ~180 GB re-download because the
fetches registry is DB-resident.

**Ask:** re-decide with the real numbers.
**Recommendation:** the study's §7(b) minimum — a tiny off-box copy of just the
user-state tables (<100 KB today, ~€0–4/mo) plus the two no-regret items (the
archive re-register-from-disk path; scheduling D4). The 500 GB corpus stays
re-derivable, exactly as originally intended.

## 3. The GDPR assessment record (issue 173a)

The lawyer's assessment is the factual basis of a standing design rule, but the
document was deleted — the repo retains no record of what it covered while the
system demonstrably stores natural-person data (UBOs, contact persons).

**Ask:** ~1 hour with you (or your lawyer's notes) to restore a one-page dated
scope record. Nothing else in 173 needs you — the tombstone design note and the
D4/D5 probes are owner work.

## Not queued (deliberately)

- 198 (per-BT coverage panel) stays backlog per your 2026-08-14 word.
- Everything else on the board is owner-actionable or measurement-parked.
