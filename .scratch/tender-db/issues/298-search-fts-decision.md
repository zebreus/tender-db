# 298 — the search/FTS decision (C14) — re-register the vanished decision

Status: CLOSED 2026-08-27 — decided as ADR-0016: no in-file FTS (FTS5 absent in
turso; native FTS is experimental AND forfeits the sqlite3-on-snapshot escape
hatch we operationally depend on); v1 search = the structured surface named
honestly; revisit triggers recorded (real consumer / FTS graduates with a
readable path / 304 grows the text corpus); interim stopgap named (normalized
title-prefix column, never a smuggled engine). SUMMARY.md register updated —
the C10/C14/C15 trio is fully decided (ADR-0014/0016/0015).
Kind: product decision + capability (text search)
Relates to: 291 (cross-language search names this), 217 (name-prefix seek).

Today the only text search is the organization name-prefix seek over `name_norm`;
tender titles/descriptions have no search at all. C14 (which search technology /
contract: turso FTS? external index? LIKE-bounded?) was never decided — it fell
off the register. Decide WITH the multilanguage design (291): the language model
determines whether search is per-language or cross-language, and the index choice
determines storage cost. Deliverable: an ADR + a first bounded implementation
(title search) or an explicit "not in v1" with a revisit trigger.
