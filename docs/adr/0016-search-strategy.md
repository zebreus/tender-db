# ADR-0016 — Search strategy (C14): no in-file FTS; the v1 search story named

Status: ACCEPTED 2026-08-27 (owner, closing issue 298 — the last of the three
register decisions research-gaps-2026-08.md found had "vanished from every
list"; C10 became ADR-0014, C15 became ADR-0015, this is C14. Decided under
Lennart's standing direction: maintainability, scalability, flexibility).

## Context

Tender titles and descriptions have no text search; the only text seek is the
organization `name_prefix` index. The engine facts are researched and verified
(turso-capabilities.md): **FTS5 does not exist in turso** ("no such module"),
and turso's native tantivy-backed FTS works but is (a) experimental, and (b)
stored inside the single DB file in a way that **makes the file unreadable by
the sqlite3 CLI**. That readability is not a nicety here: the
sqlite3-on-snapshot workflow is how every corpus-wide analysis runs (the 278
offline identification, the 172 validation pass, the 168 false-merge study)
and is the documented disaster escape hatch.

## Decision

**D1 — No in-file FTS.** Not FTS5 (absent), not turso-native FTS (an
experimental feature bought by giving up the snapshot/escape-hatch property).
This is a decision, not a deferral by neglect: the price is known and today's
buyer does not exist.

**D2 — The v1 search story is the structured surface, named honestly**:
exact identifier lookups (`publication_id`, org `identifier`), the org
`name_prefix` seek, the filter vocabulary (`country`/`cpv`/`status`/
`min_value`/`max_value`/`currency`/`kind`/date bounds, `lang` for the served
text), and bounded `LIKE` over `/v1/sql` for ad-hoc needs (isolation-class,
documented). Title search as a product feature is explicitly **not in v1**,
and /docs does not pretend otherwise.

**D3 — Revisit triggers, so this cannot vanish again**: (a) a real consumer
asks for text search; (b) turso FTS graduates from experimental AND the
snapshot workflow keeps a readable path (either the file stays sqlite3-
readable or a scheduled export job provides one); (c) portal expansion or the
304 language-editions campaign materially grows the text corpus. When
triggered, the first implementation is TITLE-ONLY, per-language on ADR-0013's
vocabulary, and sized against the deferred-index budget discipline.

**D4 — The interim stopgap, if demand arrives before the triggers**: a
materialized normalized-title prefix column on `tenders` (the `name_prefix`
pattern, additive, sqlite3-safe, droppable) — never an FTS engine smuggled in
as a stopgap.

## Consequences

- SUMMARY.md's open-decision register: C14 now points here; with ADR-0014 and
  ADR-0015 the "vanished" trio is fully re-registered and decided.
- Issue 298 closes. Issue 291's "cross-language search" note inherits D3 —
  search waits for a trigger; the language model (ADR-0013) already fixes the
  vocabulary any future index would use.
