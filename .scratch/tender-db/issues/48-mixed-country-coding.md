# 48 — Country codes stored mixed alpha-2 / alpha-3; documented filter hangs

Status: ready-for-agent
Severity: MEDIUM (data quality + a hanging documented filter)

Found by usability audit, owner-confirmed via SQL (2026-07-21):
`organizations.country` holds BOTH ISO alpha-3 and alpha-2 for the same
countries — e.g. DEU 1173 **and** DE 584, ROU 606 **and** RO 530, FRA
2375 (alpha-3) alongside many alpha-2. So no single value filters a
country reliably, and `/docs` documents alpha-3 ("DEU") while
`?country=DEU` **hangs 30s+ with no response**; the undocumented `DE`
returns in ~1s.

Two defects:
1. Ingestion writes country codes inconsistently across eras/profiles —
   normalize to ONE canonical form (alpha-2 or alpha-3, pick and
   document) at the projection/parse boundary; the mapping tables likely
   emit the source's raw code. Decide the canonical form, normalize on
   write, and plan a reprojection so existing rows converge.
2. The `country=DEU` filter HANG is the O(scan) pathology on a no-match
   (or type-mismatched) filter — it should short-circuit to empty fast,
   never hang. Check the country filter's query plan / index.

Acceptance: one country encoding across all orgs; the documented filter
value returns matching rows quickly; a no-match filter returns empty
promptly, never hangs.
