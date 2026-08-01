# 86 — org identity: German "HRB" register prefix mis-read as country code; NULL-identifier duplicate org rows

Status: open — DISCOVERED 2026-08-01 (sdk-vendor). Data-quality, low severity. Surfaced on DE data but likely general.
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
