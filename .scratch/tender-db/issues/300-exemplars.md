# 300 — pre-registered exemplar sheet (checked in BEFORE implementation)

Per the design's Stage-0 gate: these org ids and identifiers are the
must-hit / must-not-hit panel every stage's dry-run is judged against.
Sources: [M28] probe specimens re-verified by hand on 2026-08-28 via bounded
/v1/sql (the CNFPT, contamination, and 15176 rows below were each re-read
live), docs/research/org-matcher-probes-2026-08.md, identifier-rates doc.
Ids not yet pinned are Stage-0 census items, marked ⚙.

## Must-MERGE (identifier evidence carries it)

- **Maintpartner Oy** — orgs 2476219 (FI) + 5276790 (NULL), national
  20445111, identical name. R3 NULL-country rescue, Stage 3.
- **Philips Ibérica** — orgs 4727 (ES) + 9017282 (FI) + 11663831 (NULL),
  NIF A28017143. 3-way; ES NIF letter-checksum anchors the country ⇒ keep =
  4727; N2/N3 corroboration across suffix/punctuation variance. Stage 3.
- **Grand Port Maritime de la Guadeloupe** — orgs 9626564 (FR) + 19527928
  (GP), SIRET 26710682100015-style 14-digit 79453852000014. Scheme-family
  compatibility (FR family includes GP); SIRET→SIREN truncation. Stage 3.
- **Centre hospitalier de Montceau-les-Mines** — orgs 4046514 (FR, full
  name) + 4737224 (NULL, name "Centre hospitalier" — GENERIC). The stoplist
  disqualifies the generic shared key, so this merges ONLY because SIRET
  hard-checksum passes — the designed stoplist+hard-checksum path's positive
  test. Stage 3.
- **Gymnázium** — orgs 3377602 (CZ) + 6062809 (NULL), national 49371185,
  identical generic name. Decides BY CHECKSUM: CZ IČO mod-11 pass ⇒ merge;
  fail ⇒ edge. Whichever way the checksum lands, the decision path is the
  exemplar. Stage 3.
- ⚙ Stage-2 same-country pairs (pin concrete ids in the Stage-0 census):
  an FI Y-tunnus/VAT pair, an RO bare/prefixed CUI pair, a CZ zero-pad IČO
  pair, an FR SIRET/SIREN pair.

## Must-FLAG (edges only; auto-merge forbidden)

- **CNFPT** — orgs 5599259 (FR, name "CNFPT") + 5599260 (NULL, full name),
  SIREN 180014045. **Stage-0 determination MADE 2026-08-28 by live probe:**
  the satellite carries FRA "CNFPT" on one and FRA "Centre national de la
  fonction publique territoriale" on the other — an acronym↔expansion (E4)
  relation, NOT exact N2/N3 equality, so R3's corroboration condition fails
  ⇒ **must-FLAG** (`r3-uncorroborated` edge). No pressure to weaken exact
  corroboration; the E4 acronym generator is the future lever.
- **EBSCO** — orgs 11531400 (GB) + 23311674 (SE), national 5020490073.
  Bare 10-digit (PL NIP is also 10 digits): not scheme-anchored ⇒
  `r3-unanchored`, flag-first per the design's demotion.
- **Softronic** — orgs 9743730 (SE, "Softronic Aktiebolag") + 23044815 (GB,
  "Softronic AB"), national 5562490192. Same unanchored class — flag-first —
  AND the designated first promotion-review cohort member: its edge must
  carry N3-equal corroboration evidence ("softronic" + canonical ⟨AB⟩), the
  strongest possible candidate for a Stage-6 `r3-unanchored` promotion.
- **Kærunefnd útboðsmála** — orgs 6642 (IS) + 22671317 (AFG — garbage
  country, English-translated name), national 5402696459. Country fold sends
  AFG→NULL; names share no N2 key (Icelandic vs English, zero overlap) ⇒
  edge unless a satellite cross-language pair corroborates (⚙ Stage-0 check).
- Any DE pair (no cross-walk; court-scoped registers), any SK DIČ↔IČO pair,
  any CZ699 group VAT, any ES UTE (letter U) across procedures.

## Must-CONDEMN (Stage-1 placeholder dissolve)

- **org 15176** — (DE, vat, DE123456789), canonical, name "Land
  Baden-Württemberg, vertreten durch das Ministerium für Kultus, Jugend und
  Sport", 144+ distinct mention names. Re-verified live 2026-08-28.
- ⚙ the bare-`123456789` org carrying 450 distinct names (id pin: Stage-0
  census).
- Placeholder lexicon seeds (all in the measured top-30 [M28] §2, every one
  kind=national): NIMAT3-10, ORG0001-0003/ORG001-003, BT501, 123456789,
  12345678, 1234567, 123456, 12345, 1234, 0001-0004, 00001, 000000001.

## Must-NOT-TOUCH (protected allowlist, freeze at Stage 0)

- Tribunal Administrativo de Recursos Contractuales (ES, 947 distinct
  names), ELEKTRO PRIMORSKA (SI, 789), Ministères sociaux (FR, 753), Gmina
  Rzeszów (PL, 588), ⚙ + the rest of the Stage-0 top-100 census. No stage's
  dry-run may condemn, split, or merge-away any allowlist member.

## Satellite contamination (must surface as edge ONLY)

- **org 23294544** — (SE, national, 5562964618, "Bertin Exensor AB"), ONE
  mention (notice 25038532, ORG-0003), satellite rows ENG "Bertin Exensor
  AB" + NLD "Het Ministerie van Defensie". **Mechanism found 2026-08-28: the
  SOURCE notice is defective** — 25038532's XML fills ORG-0003's NLD variant
  slot with the buyer's name, and ORG-0002's NLD slot with an English
  string; the publisher's authoring tool crossed the multilingual slots.
  Our capture is faithful; the source is dirty. Consequence: satellite
  variants are source-published claims, so a variant may name a DIFFERENT
  real org — exactly why cross-language equality corroborates but never
  merges, why corroboration provenance is logged, and why the wrong-name
  variant here must never corroborate anything.
- ⚙ Stage-0 census item (sharper than the first cut): count same-notice
  same-lang identical BT-500 values on >1 ORG section WHERE the duplicated
  value differs from the section's own primary name. First bounded read
  (notices 25,000,000-25,020,000): 1,833 duplicate-value groups over 17,274
  BT-500-bearing notices ≈ 10.6% incidence UPPER bound — the wrong-name
  subset (the Bertin class) is inside it and unmeasured; the census sizes
  the satellite's contamination prior for corroboration weighting.
