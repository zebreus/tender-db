# 299 — the /v1/sql dialect promise (C15) — re-register the vanished decision

Status: BACKLOG (filed 2026-08-26; research-gaps-2026-08.md: C15 "appears in no
decision list, no ADR, no design doc")
Kind: API contract decision
Relates to: 210 (sandbox), 239 (v_* views), the OpenAPI/docs surface.

The public SQL endpoint serves turso's SQLite dialect with per-view caveats, but
we never DECIDED what we promise: which dialect surface is contractual, what may
change under a turso upgrade, which views are stable API vs internal. One turso
bump could silently break every saved analyst query. Deliverable: a short ADR
naming the promised surface (tables/views + dialect edition) and a CHANGELOG
discipline for it — cheap now, expensive to retrofit after real analyst adoption.
