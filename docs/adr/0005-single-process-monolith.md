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
