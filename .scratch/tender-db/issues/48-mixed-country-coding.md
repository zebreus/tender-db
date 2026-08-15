# 48 — Country codes stored mixed alpha-2 / alpha-3; documented filter hangs

Status: FIXED-IN-CODE, PENDING-REFOLD (2026-08-15 — hang gone, docs corrected, org country now canonicalised; materialises on the batched refold)
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

## 2026-08-15 (orchestrator) — hang fixed upstream, docs corrected

Two of the three sub-problems are closed:

- **The 30s HANG is gone.** Verified on prod: `/v1/tenders?country=DE` returns in
  ~0.8s and a no-match `?country=DEU` returns empty in ~0.55s. The issue-115/117/120
  read-path work (isolated + bounded walk, deployed 2026-08-08) removed the O(scan)
  pathology this issue reported on 2026-07-21. The "no-match returns empty promptly,
  never hangs" acceptance criterion is met.
- **The documented-filter defect is fixed (commit 4cc6d3e).** The collection `country`
  filter matches a NUTS place-code PREFIX against the tender's places — at country
  level ISO-3166 alpha-2 (`DE`), not alpha-3 (`DEU`). /docs and the OpenAPI spec said
  `DEU`, so the documented example silently returned 0 German tenders. Corrected the
  param description + every curl example in both; documented the finer NUTS prefixes
  (`DE1`, `DEB35`). NOT translated alpha-3→alpha-2 in the filter: NUTS is natively
  alpha-2 and the finer-prefix semantics make a translation shim awkward; documenting
  the reality is the honest fix.

**Residual (still open): `organizations.country` mixed alpha-2/alpha-3.** A SQL-analyst
data-quality issue (the `v_organizations.country` column holds both DEU and DE for the
same country), separate from the REST country filter (which is NUTS, now correctly
documented). Normalizing it to one canonical form is a parse/projection-boundary change
plus a reprojection — batch it with the other pending-refold work (issues 187, 179).
Scoped down to this one item.

## 2026-08-15 (orchestrator) — org.country normalization (defect 1) fixed in code

`canonical_country()` (crates/ingest/src/project.rs) folds the org country to one
alpha-2 vocabulary at the mention-finalize point: alpha-3 → alpha-2 (DEU→DE, FRA→FR,
… + EEA + common third countries), TED's non-ISO UK→GB, eurostat EL→GR; an unrecognised
code passes through unchanged. Applied before the country is stored AND before it scopes
a national id, so both agree. Test `country_codes_canonicalise_to_alpha2`. Deployed the
code; existing rows converge on the next reprojection.

**All three of issue 48's sub-problems are now closed in code:** the 30s hang (fixed
upstream by issue-115/117/120), the documented-filter defect (docs corrected, commit
4cc6d3e), and the org.country mixed coding (this commit). The refold to materialise the
org rewrite batches with issues 187 (internal-ojs chaining) and 86 (register false
country) — one rebuild clears all three org/chaining defects.
