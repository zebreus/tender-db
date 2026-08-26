# 298 — the search/FTS decision (C14) — re-register the vanished decision

Status: BACKLOG (filed 2026-08-26; research-gaps-2026-08.md: C14 "vanished from
every list"; docs/research/SUMMARY.md:310-312 still lists it open)
Kind: product decision + capability (text search)
Relates to: 291 (cross-language search names this), 217 (name-prefix seek).

Today the only text search is the organization name-prefix seek over `name_norm`;
tender titles/descriptions have no search at all. C14 (which search technology /
contract: turso FTS? external index? LIKE-bounded?) was never decided — it fell
off the register. Decide WITH the multilanguage design (291): the language model
determines whether search is per-language or cross-language, and the index choice
determines storage cost. Deliverable: an ADR + a first bounded implementation
(title search) or an explicit "not in v1" with a revisit trigger.
