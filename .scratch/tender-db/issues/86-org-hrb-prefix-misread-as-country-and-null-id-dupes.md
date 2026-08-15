# 86 — org identity: German "HRB" register prefix mis-read as country code; NULL-identifier duplicate org rows

Status: RESOLVED-VERIFIED (2026-08-15 — materialised by the full rebuild and confirmed on prod: HRB
(Handelsregister-B) orgs now carry country DE (11,263), with AT/SE/NL/GE/CH… as the small legitimate
remainder and NO false "HR" (Croatia) bucket anywhere in the distribution — the prefix→country
minting is gone). Finding #2 (NULL-identifier org dedup) remains deferred — see below. Was:
FIXED-IN-CODE, PENDING-REFOLD.
Kind: data-quality / correctness (organization canonicalization)
Blocked by: —
Relates to: 85 (DE-1.x, where it was noticed), organization identity/merge

## Findings (snapshot 531)

1. **Register prefix mis-parsed as a country code.** Org 10793 "Städtisches Klinikum Görlitz gGmbH" has
   `country = HR` with `identifier = HRBDRESDEN4115AGDRESDEN`. The German Handelsregister number
   "**HRB** Dresden 4115 …" has its `HRB` prefix being read as ISO country `HR` (Croatia) — a German org
   flagged Croatian. The country/identifier split for German register ids is wrong.

2. **Duplicate org rows with no merge key.** The same "Städtisches Klinikum Görlitz gGmbH" also has 5+ rows
   with NULL identifier — nothing to merge on, so the identity resolver can't dedup them; the org fragments
   across rows.

## Fix (investigate)

- Correct the country/identifier derivation so a German register scheme (HRB/HRA/…) is recognised as a
  register-id scheme, not parsed as a leading country code. Check the identity-normalisation path that
  splits `country` from `identifier`.
- For NULL-identifier orgs, decide the dedup key (normalised name + address/NUTS?) so same-entity rows merge
  instead of fragmenting — or confirm the current behaviour is intended (keep distinct when unidentifiable).

## Validation

Org 10793 (and its dupes) resolve to a single DE org with `country = DE` and a correctly-parsed HRB register
identifier; a spot-check of other German orgs with register ids shows no `HR`/register-prefix confusion.

## Note

Low severity relative to issue 85, but it's a correctness bug in the org register (buyer/supplier identity)
and the mis-country would corrupt any country-based filter/aggregation over German orgs.

2026-08-09 (data-profile study): this is a CLASS, measured at >=50,700 org
rows across >=15 register-prefix families (HRB/HRA/NIP/REGON/UST/FN/KRS/
SIRET/CVR/CIF/NIF/OIB/CUI/ICO/DIC...) — see
docs/research/data-profile-2026-08.md §1.5(a) and the register-prefix
normalization rule (§3 rule 1). 96.6% of stored (HR, vat) orgs are German
HRB/HRA companies.

## Fix (2026-08-15, orchestrator) — finding #1 (false country)

`normalise_identifier` (crates/ingest/src/project.rs) minted a country from any
2-letter alphabetic id prefix; `HRB 22388` → country `HR`. Per data-profile §3 rule 1:
- A **register-prefix table** (HRB, HRA, VR, GNR, PR, FN, KRS, NIP, REGON, CUI, CIF,
  NIF, OIB, CVR, ICO, DIC, SIREN, SIRET, UST) is classified national BEFORE VAT
  sniffing, so the scheme tag never mints a country. 3+-letter tags match on the
  prefix alone (handles `HRBDRESDEN4115`, court name inline); 2-letter tags (FN/VR/PR)
  also require a following digit.
- A VAT prefix must now be an actual European VAT country (`VAT_COUNTRIES`: EU-27 with
  EL, EEA, GB/UK/XI, CH). A two-letter prefix outside the set is national. Austrian
  `ATU…` (3rd char a letter) and Greek `EL…` still parse as VAT.
Tests: `register_prefixes_are_national_not_a_minted_country`,
`real_vat_ids_keep_their_country_prefix`, plus the existing merge-plausibility test.
Deployed the code; **materialises on the next reprojection** — batches with issue 187
(internal-ojs chaining) and issue 48-residual (org country normalization). One rebuild
now clears three org/chaining defects at once.

## Residual (finding #2, still open)
NULL-identifier duplicate org rows fragmenting the same entity — a dedup-key decision
(normalised name + NUTS?) per data-profile §3 rules 4–5. Separate from the false-country
fix; not addressed here.

**2026-08-15 ~08:xx UTC (orchestrator) — BATCHED REBUILD RUNNING.** Triggered the full
`project rebuild=true` (job 1) on the Saturday low-traffic window to materialise the
three deployed org/chaining fixes together (187 internal-ojs chaining, 86 register
false-country, 48 country canonicalisation). Reissues all tender/org ids + bumps feed
generation (webhooks handle it via issue 178's reset; poll/SSE re-snapshot). ~6-10h wall.
Health stays green through clear_canonical (issue-133 presence detector skips heavy
writes). Verify on completion: internal-ojs award-unchained ratio drops from 1.000;
German HRB orgs read country DE not HR; DEU/DE/UK country codes converged to alpha-2.
