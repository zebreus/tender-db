# 299 — the /v1/sql dialect promise (C15) — re-register the vanished decision

Status: CLOSED 2026-08-27 — decided as ADR-0015: names/columns/envelope are the
contract (breaks only with a CHANGELOG.md entry), the dialect is described not
promised and pinned by a gate-time canary suite (12 shapes incl. the promised
row_number() OVER), v_* views are the stable analyst API, and currency_rates
joined the allow-list as reference data (the 291 exposure question).
Kind: API contract decision
Relates to: 210 (sandbox), 239 (v_* views), the OpenAPI/docs surface.

The public SQL endpoint serves turso's SQLite dialect with per-view caveats, but
we never DECIDED what we promise: which dialect surface is contractual, what may
change under a turso upgrade, which views are stable API vs internal. One turso
bump could silently break every saved analyst query. Deliverable: a short ADR
naming the promised surface (tables/views + dialect edition) and a CHANGELOG
discipline for it — cheap now, expensive to retrofit after real analyst adoption.
