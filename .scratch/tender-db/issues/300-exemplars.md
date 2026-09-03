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
- **FI Y-tunnus/VAT pairs (pinned 2026-08-28, 8/8 of a sample had twins):**
  Telinekataja Oy — 21795788 (vat FI01003158) + 14218031 (national
  01003158), clean same-name R2. Ramboll — 2008748 (FI vat FI01011975) +
  2859694 (FI national 01011975) + 2261535 (NULL national 01011975): R2
  then R3 in one cluster. **Rename pairs — the case name evidence can never
  merge and identifier evidence must:** Linde/AGA — 16949417 (vat
  FI01003465, "Oy Linde Gas Ab") + 1741827 (national 01003465, "Oy Aga
  Ab"); ASSA ABLOY/Cardo — 16173732 (vat FI01010649) + 1691168 (national
  01010649). R2 requires no name corroboration precisely for these.
- **The CNFPT family (pinned 2026-08-28) — one entity, five rule paths:**
  SIREN 180014045 on 5599259 (FR, "CNFPT") + 5599260 (NULL, expansion);
  SIRET 18001404501577 on 3134661 (FR) + 3153628 (GP, name "CNFPF…" — a
  TYPO) + 3153629 (NULL); SIRET 18001404502245 on 3498493 (FR) + 3507716
  (NULL); SIRET 18001404501825 on 4404773 (FR). SIRET→SIREN truncation
  makes them one canonical key: FR-side rows merge via R2; NULL rows with
  N2-equal names corroborate via R3; 5599260 (expansion, no shared key)
  stays FLAGGED; the GP typo row shares no N2 key either — it tests that a
  typo blocks corroboration and lands as an edge, not a merge. The
  establishment ids survive only in raw_identifier.
- **RO bare/prefixed CUI pairs (pinned 2026-08-28, 5/6 sampled had twins):**
  Societatea de Transport București — 1932 (national 1589886) + 19250363
  (vat RO1589886); Registrul Auto Român — 4177/4176 (adjacent ids!); RAJA
  Constanța — 1666/13031830; Apavital Iași — 2360/15237; and Regia
  Națională a Pădurilor — 3224 (national 1590120, mention-named after its
  legal office "Oficiul Juridic al Direcției Silvice…") + 3223 (vat
  RO1590120): same CUI = same legal person, department-name variance —
  R2's no-corroboration design merging what names never could.
- **CZ pad family (pinned 2026-08-28):** must-MERGE — Ministerstvo financí
  3-way: 1864578 (7-digit `0006947`) + 4229 (`00006947`) + 1924720
  (NULL-country `00006947`), identical names, the corroborated-pad case.
  **must-NOT-merge — the pad collision that demoted padding to E2:**
  2364406 (CZ `0002542`, Ministerstvo spravedlnosti — corrupted id) vs
  2905864 (CZ `00002542`, Puncovní úřad — its REAL, checksum-valid IČO).
  Zero-padding manufactures information; see the design §3.1 amendment.
  Also must-merge: Ministerstvo zemědělství 1442788 (`0020478`) + 1414739
  (`00020478`) — corroborated (matching names).
- **FR zero-padded id note (2026-08-28):** FR national ids at length 14
  include left-zero-padded forms ("00000219740248") — canonical_key's FR
  arm must strip leading-zero padding before the SIRET/SIREN split; and
  "00000000000001" (Tribunal Judiciaire de Paris!) is live proof the
  repeated-digit-with-≤1-exception placeholder rule is needed.

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
  **Checked 2026-09-03 (bounded reads):** the satellites hold ONE row each —
  6642 → `ENG "Kærunefnd útboðsmála"` (Icelandic text under an ENG tag) and
  22671317 → `ENG "The Icelandic Public Procurement Complaint committee"`; no
  shared language, no shared string ⇒ **no corroboration → must-FLAG (edge)**,
  the identifier match alone must not merge it. (22671317's country is stored
  as `AF`, the garbage value the fold sends to NULL.)
- Any DE pair (no cross-walk; court-scoped registers), any SK DIČ↔IČO pair,
  any CZ699 group VAT, any ES UTE (letter U) across procedures.

## Must-CONDEMN (Stage-1 placeholder dissolve)

- **org 15176** — (DE, vat, DE123456789), canonical, name "Land
  Baden-Württemberg, vertreten durch das Ministerium für Kultus, Jugend und
  Sport", 144+ distinct mention names. Re-verified live 2026-08-28.
- **the bare-`123456789` buckets (pinned 2026-08-28):** 18 org rows, one
  per country — the resolver key's country scoping splits the placeholder
  into per-country stranger-mergers. The DE bucket is org 15566 ("Immobilien
  Bremen…", the 450-name candidate — the census's top-100 will confirm);
  every one of the 18 is condemned (15566, 8901735, 13183829, 13867390,
  13867392 (VA!), 14014373, 15464496, 20072700, 20078858, 20833217,
  22155931, 22298600, 22523324, 22540472, 22843472, 23324113, 23444156,
  23584728 (ARE)). Two carry garbage countries — the dissolve must not
  stumble on those.
- Placeholder lexicon seeds (all in the measured top-30 [M28] §2, every one
  kind=national): NIMAT3-10, ORG0001-0003/ORG001-003, BT501, 123456789,
  12345678, 1234567, 123456, 12345, 1234, 0001-0004, 00001, 000000001.

## Must-NOT-TOUCH (protected allowlist — CLASSIFIED, not blanket-frozen)

The first census run (2026-08-28, run 1330) proved the top of the
distribution is a MIX, so the allowlist is per-org classified, never
"top-100 as legitimate":

- **Legitimate (allowlist):** Tribunal Administrativo de Recursos
  Contractuales — org 2660 (ES, NIF S4111001F, 700 N2-distinct names);
  Ministères sociaux — org 2861 (FR, SIRET 11000201100044, 660);
  Ondernemingsrechtbank Leuven — org 45 (BE, 0308357753, 347); Osakidetza —
  org 1031 (ES, S5100023J, 338); Krajowa Izba Odwoławcza — org 36 (PL, NIP
  5262239325, 303). ⚙ classify the rest of the top-100 in Stage 0 proper.
- **RECLASSIFIED must-CONDEMN — the NIMAT family (2026-08-28):** the 168
  study called "ELEKTRO PRIMORSKA (SI, 789)" legitimate name-variance; the
  census's first run showed its identifier is `NIMAT500` and a mention
  sample on org 211 contains STRANGERS (Elektro Primorska, Elektro
  Ljubljana, the SI Interior Ministry, Pošta Slovenije, Luka Koper,
  Generali…). Five NIMAT orgs sit in the top-12 alone: 211/NIMAT500 (794),
  378/NIMAT501 (532), 1229/NIMAT502 (365), 22683098/NIMAT100 (320),
  2791/NIMAT503 (295). `NIMAT\d+` is a placeholder id family (SI
  e-procurement), lexicon entry confirmed at top priority.
- **Also condemned by the census:** org 10583053 (PL, vat `PL823`, 418
  names) — a 5-char VAT stub; short-VAT stubs (<6 digits after the country
  prefix) join the lexicon.

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
  **Measured 2026-09-03, the sharper cut** (same window, 67,878 BT-500 rows
  over 17,274 notices, 24 languages): of the 1,833 duplicate-value groups,
  only **2** carry a duplicated value that differs from the section's own
  original-language BT-500 (joined through BT-702) — and both are Belgian
  bilingual buyers whose FR/EN translation of their OWN name sits on two of
  their own sections (notice 25006588 "SERVICE PUBLIC FÉDÉRAL STRATÉGIE ET
  APPUI" ↔ NL primary "FEDERALE OVERHEIDSDIENST BELEID EN ONDERSTEUNING";
  25019799 "Ministry of Defence" ↔ "Ministerie van Defensie"). The Bertin
  shape (a translation slot naming a DIFFERENT org) is **0 in the window**;
  the 1,831 others are the same org in two roles with identical names. So the
  contamination prior for cross-language satellite corroboration is ≤ 0.01%
  of notices — corroboration may weight satellite pairs as reliable, with the
  Bertin exemplar kept as the known rare shape rather than a prior.
