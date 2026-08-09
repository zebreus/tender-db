# 164 — retirement `removed` change rows are invisible to SSE subscribers

Status: resolved (2026-08-09) — diff loop relays the log's own `removed` op when both probes miss; over-delivery to filtered subscribers is the documented contract. Test: a_retirement_reaches_the_stream_as_removed (red-checked). Webhooks unaffected (raw change_event already carries op).
Severity: MEDIUM (correctness of the live feed's "final state exact" promise)
Role: run-driver

Found by the issue-55 fix's adversarial review (2026-08-09). Pre-existing —
the diff loop is unchanged since before the paged snapshot.

`retire_chunk_tx` appends `removed` change rows with `version_seq = NULL`,
then hard-deletes the entity rows including versions
(crates/store/src/canonical.rs, retire path). The SSE diff loop maps a NULL
seq to 0, probes `Scope::At { seq: 0 }` → `new = None`, and `seq > 1` is false
→ `old = None`; `(false, false) → continue` drops the event silently
(sse.rs diff loop). A subscriber that snapshotted a Tender before its
retirement keeps a ghost forever — the log recorded the removal, the feed
never says so.

Under the issue-55 paged snapshot there is a second wrinkle: a retire landing
mid-snapshot produces pages mixing pre/post-retire state, and the diff phase
cannot repair it for the same reason.

Fix direction: the `(old, new) = (None, None)` arm must consult the change
row's own `op` — a log `removed` should emit `removed` to any subscriber whose
filter COULD have matched the entity (we no longer have the old row to
evaluate the filter against; over-delivering `removed` is safe, clients treat
it as an idempotent delete). Needs a test retiring a snapshotted Tender under
an open tape.
