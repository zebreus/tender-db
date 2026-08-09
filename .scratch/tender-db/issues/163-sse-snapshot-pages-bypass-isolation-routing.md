# 163 — SSE snapshot pages bypass the issue-120 isolation routing

Status: resolved (2026-08-09) — snapshot page reads route through store::read::walks to the isolated pool, exactly as the list endpoint (issue-120 routing); Shed ends the stream with a named error and the client re-subscribes under the pool's own admission. Diff probes stay on main readers deliberately: Scope::At is id-anchored and cheap under any filter shape. Test: a_walk_shaped_snapshot_pages_on_the_isolated_pool (red-checked).
Severity: MEDIUM (availability hardening on the public API)
Role: run-driver

Found by the issue-55 fix's adversarial review (2026-08-09).

The list endpoint routes filter shapes that CAN walk to the isolated
runtime/pool (`state.isolated`, issue 120, mod.rs walkable-filter routing), so
an uncancellable walk never holds a main-pool reader. The SSE snapshot reads
its keyset pages straight from `state.readers` (sse.rs snapshot loop) with the
same unauthenticated filters. A walking filter therefore pins a main-pool
reader per page, back-to-back, for as long as the client stays connected — up
to 5 streams per IP against 8 prod readers.

The issue-55 paging bounds what a VANISHED client costs (cancellation at the
next yield); a CONNECTED slow-reading client with a walk-shaped filter is
still closer to the 2026-08-09 incident shape than the module doc suggests.

Fix direction: route snapshot page reads through the same walkable-filter
decision the list endpoint uses (isolated runtime for walkers), or gate
walk-shaped filters out of SSE subscriptions with a 400 naming the reason.
