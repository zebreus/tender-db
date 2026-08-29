# 310 — Stage-3 prevention: country-less identifier mentions keep minting NULL-country twins

Status: ready-for-agent
Kind: capability (organization layer quality)
Relates to: 300 (Stage 3), 234 (provisional line), the Stage-2 prevention
precedent (resolver pre-probe canonicalization)

## The gap, measured

The R3 merge arm removed 1,287 NULL-country identifier orgs (2026-08-29,
pool 4,816 → 3,529) and the refold canary PASSED — a refold of notice
17910103 kept its mention bound to the Maintpartner keep (2476219) and
re-minted nothing, because `resolve_mentions`' idempotency map keeps a
recorded (notice, section) on its Organization by design.

But NEW notices are unprotected: a mention with an identifier and NO
country (the DÖE/eForms shapes that built the original 4,816) still walks
the E0-miss path and mints a fresh NULL-country provisional. The pool
regrows at ingest rate until a merge sweeps it again. Stage 2 closed the
same loop for same-country twins with the resolver canon-probe; Stage 3
has repair without prevention.

## Design sketch (needs its own panel round before build)

In `resolve_one_mention`'s miss path, when the mention has NO country and
the raw identifier's digits have a UNIQUE `idgate::checksum_anchors` probe
(exactly one real anchor — same fn the census/merge inject):

1. look up the anchored `(scheme, key)` in the resolver's canon map
   (already preloaded for Stage-2 prevention; keys are country-ful E1);
2. require sole-owner (not poisoned) — the merge arm's single-target rule;
3. require exact cross-language N2 corroboration between the mention name
   and the target org's names (`match_norm` equality — the R3 merge's own
   bar; prevention must NOT be more aggressive than the verified merge);
4. bind on success; otherwise mint the provisional exactly as today.

Open questions for the panel: where the corroboration name read comes from
(org head name in the canon map vs a point read of organization_names);
whether the consortium veto belongs in the prevention path too (Stage-2
prevention poisons consortium-named mints — mirror it); cost of the anchor
probe per country-less miss (cheap: digits-only arithmetic).

## Interim mitigation

Fold rule=r3 into the periodic match-org-identifiers cadence (the Sunday
cadence decision, pending): a weekly dry+capped-wet sweep bounds regrowth
at a week even before prevention lands. The wet's parity guard requires
the fresh dry each time by construction.

## Acceptance

- A fixture notice with a country-less uniquely-anchored corroborated
  identifier resolves to the standing org, minting nothing.
- Uncorroborated / multi-anchor / poisoned cases still mint (deny
  direction preserved).
- r3-census pool stops growing week-over-week in steady state (the
  measurable outcome).
