# 46 — A full rebuild orphans the change log (no `removed`, no epoch signal)

Status: RESOLVED-VERIFIED on prod (2026-08-11 00:3x CEST, deploy e66967c) — /v1 root serves generation:1, /v1/changes envelope carries it
settled once across all three transports exactly as the 2026-08-09 note asked:
- `feed_generation` (single-row table, seeded 1) bumped by BOTH wipes:
  `clear_canonical` (before re-derivation starts, so mid-rebuild polls already
  see it) and `clear_changes` (whose cursor re-issue would otherwise let an old
  cursor "resume" inside the new feed).
- Poll: `/v1/changes` envelope and `/v1` root carry `generation`; contract
  documented in /docs (store it beside the cursor; on a move: drop state,
  re-fetch collections, continue from the new last_cursor).
- SSE: diff/live event ids are now generation-qualified resume tokens
  (`<gen>:<cursor>`, opaque per the docs); a cross-generation resume gets
  `reset {"reason":"feed_rebuilt"}`, a beyond-head bare cursor gets
  `reset {"reason":"cursor_ahead"}`, pruning keeps `cursor_expired`. `live` and
  `reset` carry the generation.
- Webhooks: every delivery body carries `generation` (detection); the SLOT
  semantics (stranded-slot reset) split to issue 178 with a design sketch —
  the sweeper's stored cursor needs its own generation-aware reset.
- Conformance: `a_rebuild_moves_the_generation_and_resets_stale_resumes`
  (app/tests/api.rs) — generation 1 → wipe → 3 visible at root/poll, stale SSE
  token → feed_rebuilt reset, bare ahead-of-head cursor → cursor_ahead reset.
The "one clean rebuild before launch" recommendation stands and is now safe to
execute at any time: the rebuild IS the signal.
Severity: MEDIUM (live-subscription coherence across a rebuild; not
imminent — see scoping)

Found by change-feed verification (2026-07-21), owner-confirmed in code.
`Db::clear_canonical` (crates/store/src/canonical.rs:684), run by
`project --rebuild=true` (ingest/src/project.rs:253), deletes all
canonical rows but appends NO `removed` change events; re-derivation then
appends fresh `added` rows with new autoincrement ids on the same
never-reset cursor. Result: the change log holds `added` events for ids
that no longer exist, with no `removed` — a client replaying `since=0`
across a rebuild reconstructs the pre-rebuild entities and never
converges to the current snapshot.

Evidence: of 21,489 ever-added tender ids only 7,163 still exist (ids
1..14326 gone, no `removed`); same for orgs/lots. From the day's
dress-rehearsal `rebuild=true` runs.

SCOPING (why not urgent):
- The `removed` op IS emitted on the normal paths (round replacement,
  `retire_absorbed_legacy_tenders` merges) — so incremental operation is
  coherent. Only full `rebuild=true` orphans the log.
- The QUEUED job 5 is `rebuild=false` → does NOT call clear_canonical →
  the imminent re-projection is coherent. No live break pending.
- No real subscribers exist yet (pre-launch).
- CONTEXT.md already anticipates this: "cursors may expire (documented
  reset path)". A rebuild is exactly that event — it's just not signaled.

Fix (design decision needed — don't rush):
- Preferred: a generation/epoch counter bumped by clear_canonical and
  surfaced at /v1 (and in the SSE snapshot marker), so a client detects
  "the world was rebuilt, re-snapshot from scratch" — the documented
  reset path made real. Cheaper and honest vs. emitting a removed-storm
  for millions of rows then re-adding them.
- Document the reset semantics for clients.
- Consider: after the issue-15 backfill fully settles, do ONE final
  clean rebuild so the shipped change log is coherent from since=0 for
  launch (or start the epoch at that point).

Acceptance: a rebuild is detectable by a live/poll client (epoch or
documented reset), so snapshot+diff coherence is restorable; documented
in the API docs.

2026-08-09 (completeness critique, research gap #5): the premise that "the
initial backfill runs before subscribers exist" died with ADR-0009 routine
rebuilds; SUMMARY §4 still calls B1 resolved. The genuine residue beyond this
issue's fix: settle the epoch/reset semantics ONCE as a documented protocol
(SSE + poll + webhooks), write it into the API docs, add a replaying-client
conformance test. This issue's own recommendation (one clean rebuild/epoch
start before launch) has a use-by date. See
docs/research/research-gaps-2026-08.md gap 5; siblings: 163, 164.
