# 52 — Server RSS grows monotonically across the backfill (possible leak)

Status: ready-for-agent
Severity: LOW (no OOM risk for job 1; investigate)

Observed during the 2026-07-21 backfill (rev 62f7255): the server RSS
climbs roughly linearly with packages processed — ~300 MB early
(2013-05) → 797 MB by pkg 38/158 (2016-06), ~20 MB/package. The
supervisor processes one package at a time and each package's working
state should be released after it commits, so monotonic growth suggests
something is retained: candidates — a per-package accumulator not
cleared, the reader-pool / prepared-statement caches, turso page cache
(may be benign/bounded), or the mention/org maps in the walker.

Not urgent: extrapolated to the ~120 remaining packages that's ~+2.4 GB
→ ~3.2 GB on an 8 GB box — no OOM for job 1. The projection (job 5) is
the memory-sensitive phase (issue 19 chunks it); confirm RSS there too.

Task: profile where the retained memory goes (a heap snapshot across a
few package boundaries, or bisect by disabling caches), confirm whether
it plateaus or is unbounded, and fix if it's a real retention bug. If it
turns out to be bounded turso page cache, document that and close.

Acceptance: RSS growth explained — either bounded/benign (documented) or
a retention bug fixed so RSS plateaus across the backfill.
