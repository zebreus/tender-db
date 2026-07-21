# The dashboard is measured off the request path, never on it

The dashboard root `/` shows coverage, quarantine, and pipeline numbers that
require aggregating the whole notice layer — `COUNT`/`GROUP BY` over the
multi-million-row `notices` table (3.5M in production). Under continuous backfill
writes those scans are cold: the page cache thrashes, so the scan hits disk
every time.

A single background tokio task therefore re-measures the entire dashboard on an
interval (60s) and stores the result in an in-process memo
(`OnceLock<RwLock<Option<Dashboard>>>`). Requests call `latest()` — a
synchronous, store-free read of the memoized snapshot — and can never trigger a
scan, no matter how slow a measurement would be. That `latest()` takes no `Db`
handle and is not `async` makes "a request recomputes" a compile error rather
than a runtime hope. It is stale-while-revalidate: a measurement error keeps the
last good snapshot, and before the first fill (a fresh boot) the empty default
renders as "no data yet", its `measured_at` telling the client how fresh the
numbers are.

Rejected alternative (superseded today for cause — issue 20 part 3): a
request-path TTL cache. Under sustained write churn its scan was always cold, so
the TTL never actually protected anything, and concurrent cold misses each
scanned and pinned a core because there was no single-flight — the acute form of
the `/` timeout seen in production. Moving the scan off the request path
entirely is what removes the failure: no public traffic can cause a scan at all,
so no amount of load on `/` can stall it.

Consequences: the dashboard numbers can lag reality by up to one refresh
interval. That is acceptable — live job progress comes from the supervisor,
which never scans, and coverage figures moving on a minute boundary are
invisible to a human reader. The refresher is a process singleton with an
idempotent start (a dev hot-reload re-running the server initializer must not
spawn a second loop), and it reads through the store's reader pool (ADR-0005),
so even its own scan runs over WAL and never blocks the writer.
