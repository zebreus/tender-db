# 178 — a webhook slot strands silently across a feed rebuild

Status: ready-for-agent
Severity: MEDIUM now, HIGH once external webhook consumers exist
Found: 2026-08-10 (orchestrator), while closing issue 46 — webhooks are the
third transport of the same protocol and got only the detection half.
Relates to: 46 (the protocol), 08 (delivery engine)

## The bug

Each endpoint's consumer slot is `last_delivered_cursor` over the change log
(webhooks.rs `deliver`). After a rebuild with `clear_changes`, the feed is
re-issued from cursor 1 — an endpoint whose slot is beyond the new head reads
an empty batch, `deliver` returns "caught up", and the endpoint NEVER receives
another event. No error, no backoff, no delivery row: indistinguishable from a
quiet feed. (SSE and poll clients get a `reset`/`generation` signal for this
exact situation since issue 46; the sweeper itself checks nothing.)

Without `clear_changes` (rebuild appends onto the old log), the endpoint keeps
receiving — but the batches compose pre- and post-rebuild worlds (dead entity
ids with no `removed`), which is the issue-46 incoherence delivered over HTTPS.

## What issue 46 already landed

Every delivery body now carries `"generation"` (read once per sweep), so a
consumer CAN detect the rebuild the moment a batch arrives. That fixes
detection for the flowing case, but not the stranded-slot case (no batch ever
arrives to carry the signal) and it does not decide what the SLOT should do.

## Design sketch (decide, then implement)

Add `last_generation` to webhook endpoints (nullable, additive migration).
In `deliver`:
- slot generation == current → normal path;
- mismatch (or NULL on first contact after upgrade) → the endpoint's mirrored
  state does not compose: reset the slot to the CURRENT HEAD (not 0 — a
  consumer re-snapshots via REST like every other client; redelivering 80M
  historical rows is not a reset), stamp the new generation, and POST a
  batch-shaped notice `{"reset": "feed_rebuilt", "generation": N, "events": []}`
  so the consumer hears the reset even when no events are flowing;
- independently, slot cursor > head with SAME generation cannot happen
  (append-only log) — treat as corruption, log loudly.

Acceptance: red test first — endpoint delivered under generation 1, rebuild
(clear_canonical + clear_changes), new change lands, endpoint receives the
reset notice and then the new change; a stranded endpoint (no new changes at
all) still receives the reset notice on the next sweep. Document the reset
body in /docs webhooks section.
