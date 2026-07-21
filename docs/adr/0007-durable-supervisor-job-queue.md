# The supervisor's job queue is durable across restarts

Ingestion jobs (fetch, process, project, snapshot) come from the `/admin` API
and the daily scheduler, and run one at a time inside the server process
(ADR-0005 — the store has a single writer anyway). A backfill is not a quick
job: it fans a period range into per-package fetches plus a multi-hour walk over
years of archived packages. A deploy, crash, or restart mid-run must not
silently lose the outstanding queue, nor the job that happened to be running.

Each outstanding job is therefore persisted as a row in `job_queue`, keyed by
the supervisor's own monotonic id and written *before* it enters the in-memory
queue — the durable row is what a restart rebuilds from, so it must exist first.
The row lives from enqueue until the job concludes (ok or error); completion and
cancellation both delete it. Recovery runs at startup *before* the worker or
scheduler start, rebuilding the in-memory queue oldest-id-first. The job that
was running when the process died was popped from memory but never removed from
the durable table, so it is the lowest surviving id and lands back at the front
to be re-run from the top.

Re-running is safe because ingestion is idempotent: notice identity is
`(source, publication_id, content_hash)` and the projection is a pure function
of the parsed layer, so a repeated run inserts nothing new. That gives
at-least-once execution with an idempotent re-walk. To avoid paying for a full
re-walk of a multi-year backfill after a late restart, a process job also
carries a resume cursor — the last package it fully committed, advanced only
after every member of that package has committed — so recovery resumes at the
next package and the interrupted (partial) one re-runs and dedups.

The job's `spec` is an opaque, app-owned serialized payload; the store
round-trips it verbatim and never interprets it. It is serde's
externally-tagged enum (`{"Fetch":{…}}`, `"Snapshot"`), decoded in exactly one
place (recovery). An unknown tag is a clean unknown-variant miss that is dropped
and its row removed — never a silent misparse into the wrong variant. Adding a
`Spec` variant is therefore forward/backward safe: a newer rev may enqueue a
variant an older rev cannot read, and on a rollback the old rev simply sheds
what it does not understand (a dropped Snapshot is regenerable — it is not
taken, not corrupted).

Rejected alternatives: an in-memory-only queue (the previous shape) loses every
outstanding job — hours of queued backfill — on any deploy, and cannot bring
back the job that was mid-run. A general durable job framework (a real broker,
ret/ack, dead-letter queues) is over-built for one-writer sequential work on one
box; the whole point of ADR-0005 is to not carry that machinery.

Consequences: the queue and cancellations survive restarts, and the daily
pipeline (probe → process → project → snapshot) resumes mid-sequence. The
running job is identified purely as the lowest surviving id, so ordering by id
is load-bearing, and `next_id` must be advanced past every recovered id at
startup so a fresh enqueue cannot collide. Persistence is best-effort telemetry
in spirit: a failed enqueue or cursor write still runs this session, it just
costs a re-walk (never correctness) on the next restart. Live per-job progress
stays in memory — only the queue and the finished-run log land in the store.
