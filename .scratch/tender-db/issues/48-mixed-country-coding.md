# 48 — Country codes stored mixed alpha-2 / alpha-3; documented filter hangs

Status: RESOLVED 2026-09-01 — re-measured, and the residual this line used to
claim is gone. Both halves of the original complaint verified fixed; a 162-row
deliberate residual is documented below with a per-value reason. The previous
Status ("RESIDUAL: 2,168 length-3 + 151 length-10") was a 2026-08-15 reading that
the issue-319 fold has since superseded — it sat stale on the board for two weeks.
Was: FIXED-IN-CODE, PENDING-REFOLD.
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

**2026-08-15 ~08:xx UTC (orchestrator) — BATCHED REBUILD RUNNING.** Triggered the full
`project rebuild=true` (job 1) on the Saturday low-traffic window to materialise the
three deployed org/chaining fixes together (187 internal-ojs chaining, 86 register
false-country, 48 country canonicalisation). Reissues all tender/org ids + bumps feed
generation (webhooks handle it via issue 178's reset; poll/SSE re-snapshot). ~6-10h wall.
Health stays green through clear_canonical (issue-133 presence detector skips heavy
writes). Verify on completion: internal-ojs award-unchained ratio drops from 1.000;
German HRB orgs read country DE not HR; DEU/DE/UK country codes converged to alpha-2.

## Re-measured 2026-09-01 (job 578, dry) — the storage half is complete

```
fold-org-countries DRY: 247 distinct country values, 0 to fold over 0 rows,
                        0 collisions
```

**Nothing left to fold.** The issue-319 wet fold (1,345 rows over 396 values)
finished the job, and this issue's Status had simply not been updated since the
pre-fold reading.

### The residual is 162 rows, and every one is deliberate

| value | rows | why it stays |
| --- | --- | --- |
| `1A` | 142 | Not ISO 3166 at all. A placeholder in the source. Mapping it needs evidence of what the publisher meant, which we do not have. |
| `AN` | 15 | **Retired** ISO alpha-2 — Netherlands Antilles, dissolved 2010 into BQ / CW / SX. Picking a successor would invent information the notice never carried. |
| `1A0` | 3 | Not ISO. Same as `1A`. |
| `XI` | 2 | The EU **VAT** country code for Northern Ireland. The ISO country is `GB`, but folding `XI` → `GB` erases a distinction the publisher deliberately encoded. |

Plus two values the fold **skips on purpose** and always will:
`EL` (2 rows) and `UK` (1 row) — VAT-scope prefixes the resolver binds on, so
folding them would break binds. That was issue 319's decision, not a gap.

162 rows out of 12,588,066 is 0.0013%. More to the point, **there is no correct
mapping to apply** — each is non-ISO, retired, or a different kind of code
entirely. A repair here would be a guess wearing a normalisation's clothes.

## The user-visible half, verified against the live API

The original complaint had two parts, and the storage fold only addresses one.
Checked directly rather than assumed:

```
GET /v1/tenders?country=DEU&limit=1  ->  200 in 1.76s, 0 items
GET /v1/tenders?country=DE&limit=1   ->  200 in 1.05s, 1 item
/docs                                ->  "country level NUTS is ISO-3166"
```

* **The 30s+ hang is gone.** `?country=DEU` answers in under two seconds.
* **The docs no longer document alpha-3.** They say ISO-3166.

Both halves of the filed complaint are fixed. Closing.

## One thing noticed while verifying, filed separately as 336

`?country=DEU` and `?country=ZZ` both return `200`, zero items, and an **empty
`ignored_filters`** — so a caller cannot tell "no tenders in that country" from
"that code can never match anything we store". Anyone still following the old
alpha-3 habit this issue was about gets a plausible-looking empty answer.

**`ignored_filters` is NOT the fix**, and that is worth saying here so nobody
reaches for it: its contract (`Collection::honoured_params`) is that it names
request *parameters the collection does not honour at all* — and `country` is
honoured by Tenders, so the filter genuinely applied. `honoured_params_match_the_emitted_sql`
enforces that invariant by byte-comparing emitted SQL. Putting an unmatchable
*value* in that list would make it lie about what applied.

Filed as **336** as a distinct UX question, not folded in here.
