# Everything runs in one server process on one server

The API server, the dashboard, the scraper/importer, SSE fan-out, and webhook
delivery are all one process (tokio tasks in the Dioxus/Axum server), deployed
as one systemd unit on one machine (the Hetzner VPS), owning the single Turso
SQLite file exclusively.

Splitting the scraper into its own process/host was rejected deliberately:
SQLite has no network protocol, Turso's multi-process WAL story is younger
than SQLite's, and a one-box product gains nothing from process isolation
except IPC, deployment surface, and failure-mode complexity. The scraper
remains a separate module with its own tests — the monolith is a runtime
shape, not a code shape.

Do not "fix" this by extracting services. If scale ever genuinely demands it,
the seams are the module boundaries and the change cursor.

Amendment (2026-07-21, from the architecture review): the "one writer, N
readers" shape is made concrete. Within the process the single writer connection
is reserved for writes and schema; reads run over WAL through bounded reader
pools that parallelise with the writer and with each other, so a dashboard,
admin, API, SQL, or webhook read never queues behind an ingestion job that is
holding the writer for the length of its transaction (the issue-20 guarantee — a
held `BEGIN IMMEDIATE` must not stall `/`). There are four independently-sized
pools over the one file: the store-internal pool for `Db`'s own accessors
(`READ_POOL` = 8), the public API pool (`READERS` = 8), the SQL-endpoint pool
(`SQL_READERS`), and the webhook-delivery pool (2). Keeping them separate stops
one subsystem's fan-out from starving another's, at the cost that no single
owner tracks the process's total open-connection budget — accepted for a one-box
product at current sizes, revisit if connection pressure ever appears. The pool
sizes are the seam to tune; a second writer is not.
