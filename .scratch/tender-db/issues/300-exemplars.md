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

### Top-100 multi-name orgs, ranks 6–20 — classified 2026-09-03 (bounded mention samples, top-6 names per org)

| rank | org | country / identifier | verdict | why |
| --- | --- | --- | --- | --- |
| 6 | 919 | FR SIREN 552081317 | **legitimate** | EDF SA + purchasing departments; one SIREN in four spellings |
| 7 | 9885866 | FR SIREN 329338883 (+SIRET suffixes) | **legitimate** | Colas France establishments under one SIREN |
| 8 | 3553 | FR 17750000600024 | **legitimate** (review body) | CCIRA — one interregional amicable-settlement committee, city variants |
| 9 | 46 | AT 210220y | **legitimate** (central purchasing) | Bundesbeschaffung GmbH; the prose "Auftraggeber sind die Republik Österreich…" names the represented buyers |
| 10 | 1774124 | ES NIF S4611001A | **legitimate** | Generalitat Valenciana — consellerias as names, one NIF |
| 11 | 8365812 | IT 95054920632 | **legitimate** | MIT Provveditorato interregionale, spelling variants |
| 12 | 1870 | ES NIF S1511001H | **legitimate**, head name wrong | the NIF is the Xunta de Galicia's; consellerias and the TACGAL tribunal share it — one legal entity, but the head name should be the Xunta, not the tribunal (a head-pick note, not a merge question) |
| 13 | 4882 | CZ IČO 01312774 | **legitimate** | Státní pozemkový úřad with regional branches |
| 14 | 176 | IT 80195990587 | **legitimate** (review body) | TAR Lazio, six spellings |
| 15 | 1127 | ES NIF S4833001C | **legitimate** | Gobierno Vasco departments |
| 16 | **660** | DE **`t:04131153308`** | **must-CONDEMN** | the "identifier" is a TELEPHONE number (a `t:` value); it fuses Vergabekammer Niedersachsen (≈17k mentions) with "Die Vergabekammern des Bundes" (708) — two different review bodies under one phone number. The `t:`/phone class must fail the §2.1 plausibility gate; dissolve in Stage 1 |
| 17 | 2687 | NL KvK 50555596 | **legitimate** (central purchasing) | RIS / UBR\|HIS; the "Ministerie … dtv UBR\|HIS" names are represented buyers |
| 18 | 8124383 | FR SIREN 552044992 | **legitimate** | Pomona Passion Froid establishments |
| 19 | 3094606 | FR 26060070500040 | **legitimate** | CHU de Nice, spellings |
| 20 | 3223 | RO RO1590120 | **legitimate** | Romsilva with regional directorates |

### Ranks 21–35 — classified 2026-09-03, same method (top-5 mention names per org)

| rank | org | country / identifier | verdict | why |
| --- | --- | --- | --- | --- |
| 21 | 1955 | ES NIF S0811001G | **legitimate**, head name wrong | the Generalitat de Catalunya's NIF: the Tribunal Català de Contractes AND departments (Treball, Interior) — one entity, head should be the Generalitat (same shape as 1870) |
| 22 | 5367052 | FI 0194099-3 | **legitimate** | Fysios Oy with branch offices ("/Tornion toimipiste") |
| 23 | 2909 | FI 0201256-6 | **legitimate** | Helsingin kaupunki divisions |
| 24 | 503 | PL NIP 8942556799 | **legitimate** | Urtica, spellings and NIP formatting |
| 25 | 9442 | FR SIREN 267500452 (+SIRET) | **legitimate** | AP-HP establishments under one SIREN |
| 26 | 13253714 | NL KvK 71710949 | **legitimate** (agent) | DASmakkelijk B.V. "namens" school foundations — the names append the REPRESENTED client; one KvK, one provider. Matching must not attribute the clients to the agent (a representation relation, not a name variant) |
| 27 | 11671698 | IT 80054330586 | **legitimate** | CNR and its institutes |
| 28 | 391 | ES NIF S7800001E | **legitimate** | Comunidad de Madrid consejerías |
| 29 | 12768 | FI 2296962-1 | **legitimate** | the ELY centres share the KEHA business id; regional names |
| 30 | 976 | PL NIP 6481997718 | **legitimate** | Zarys, spellings |
| 31 | 1242402 | ES P2807900B | **legitimate** | Ayuntamiento de Madrid áreas and distritos |
| 32 | 3444 | FR 13002928300012 | **legitimate** (review body) | CCIRA de Nantes and its DREETS host |
| 33 | 311 | GR 1000.E00961.0001 | **legitimate** (review body) | ΕΑΔΗΣΥ, Greek/Latin spellings |
| 34 | 116 | IT 97024970150 | **legitimate** (review body) | TAR Lombardia / TAR Milano |
| 35 | 4063 | FR 12000009600020 | **legitimate** (review body) | CCNRA and the ministry's Direction des affaires juridiques that hosts it |

Fifteen of fifteen legitimate. Two recurring shapes worth naming for the
allowlist rule: a regional government's NIF shared by its review tribunal and
its departments (1870, 1955 — the head name lands on the tribunal because it
is the most-mentioned name), and a procurement AGENT whose mention names carry
the represented client after "namens"/"dtv" (13253714, 2687). Ranks 36–100
remain.

Fourteen of fifteen are one legal entity with departments, establishments or
spellings — the shape the allowlist exists for. The one condemn is a new gate
class: a phone number in the identifier slot (`t:` prefix), which no register
scheme produces and which fused two bodies. Ranks 21–100 remain (the report's
`top` list, saved for the pass).

### Top-100 ranks 36–50, classified 2026-09-03 (bounded mention samples)

| rank | org | class | note |
| --- | --- | --- | --- |
| 36 | 1381 Vergabekammer Baden-Württemberg (DE) | legitimate | chamber + host RP Karlsruhe |
| 37 | 1816 CCIRA (FR) | legitimate | review body |
| 38 | 2424 CCIRA / Préfecture PACA (FR) | legitimate | review body on its host's SIRET |
| 39 | 244 Univerzita Karlova (CZ) | legitimate | faculties |
| 40 | 1448 Regierung von Oberbayern, Vergabekammer Südbayern (DE VAT `DE811335517`) | legitimate legal person, **caution** | "Vergabekammer Nordbayern" (853 mentions) rides the same Bavarian state VAT — one Land, two chambers; the head name is one of them. The design's "any DE pair" caution, seen from inside one org |
| 41 | 8107 Región de Murcia consejerías (ES) | legitimate | departments |
| 42 | 3363 Telefónica Soluciones (ES) | legitimate | spellings |
| 43 | 2950 Vergabekammer Thüringen (DE) | legitimate | chamber + host |
| 44 | 557 Salus International (PL) | legitimate | spellings |
| 45 | 1647242 Gobierno de Canarias consejerías (ES) | legitimate | departments |
| 46 | 1414739 Ministerstvo zemědělství (CZ) | legitimate | head name is a branch (Pozemkový úřad Jihlava) |
| 47 | 1916229 SID (FR) | legitimate | defence infrastructure service, regional SIRETs |
| 48 | 11215 Stockholms stad (SE) | legitimate | departments |
| 49 | 11778404 Office of Government Procurement (IE) | legitimate | central purchasing body; a few client names (Irish Prison Services, University of Galway) ride its VAT — the "namens" shape |
| 50 | **22165664 "Gemeinde Obersulm" (DE, `8477`, scheme EU)** | **must-CONDEMN** | OEW Breitband GmbH, Zweckverband Breitband Ravensburg / Schwäbisch Hall / Breisgau-Hochschwarzwald, BLS Sigmaringen — unrelated entities on one 4-digit placeholder; the head name is not even among the top mentions |

### Top-100 ranks 51–65, classified 2026-09-03 (bounded mention samples)

| rank | org | class | note |
| --- | --- | --- | --- |
| 51 | 3930 CNRS (FR) | legitimate | délégations = SIRET establishments of one SIREN |
| 52 | 4816201 Stadt Halle (DE) | legitimate | departments under one tax number |
| 53 | 9157 ESO EAD (BG) | legitimate | Cyrillic/Latin transliterations |
| 54 | 1093 Ministerstvo obrany (CZ) | legitimate | |
| 55 | 10711 Ville de Paris (FR) | legitimate | |
| 56 | 8386925 Gras Savoye / Willis Towers Watson France (FR) | legitimate | a rename; one SIREN |
| 57 | **22318692 "Verwaltungsgemeinschaft Nordendorf" (DE, `13754`, scheme EU)** | **must-CONDEMN** | Landratsamt Straubing-Bogen, Gemeinde Wehrheim, Stadt Schriesheim, Kreisstadt St. Wendel, VG Bad Grönenbach — unrelated municipalities on one 5-digit placeholder; the `8477` class again |
| 58 | 8604932 id verde (FR) | legitimate | establishments |
| 59 | 447 Vergabekammer Sachsen-Anhalt (DE, `t:03455141536`) | legitimate cluster, **phone-number id** | every name is the one chamber/host — unlike 660, no fusion; the gate should NULL the id and let the name carry the entity |
| 60 | 2824 Landkreis Darmstadt-Dieburg (DE, `00002636`) | legitimate cluster, short zero-padded id | the Kreis's organs |
| 61 | 2439 CNAIR / DRDP (RO, `16054368_3`) | legitimate | regional road directorates of one CUI; head name is a branch |
| 62 | **1079 ΕΝΙΑΙΑ ΑΡΧΗ ΔΗΜΟΣΙΩΝ ΣΥΜΒΑΣΕΩΝ (GR)** | **must-MERGE with 311** | same authority split by a HOMOGLYPH: raw ids `1000.E00961.0001` (Latin E) vs `1000.Ε00961.0001` (Greek Ε, U+0395); the normaliser keeps the Latin letter (`1000E009610001`) and drops the Greek one as non-ASCII (`1000009610001`) — a confusable fold before the ASCII strip unifies them (§2.1/§2.2 exemplar) |
| 63 | 225 Raad van State / Conseil d'État (BE) | legitimate | bilingual |
| 64 | 1513 Vergabekammer Rheinland (DE, `05315-03002-81`) | legitimate legal person, **caution** | "Vergabekammer Westfalen" (1,202 mentions) rides the same NRW id — the 1448 shape |
| 65 | 633 Vergabekammer Brandenburg (DE, `t:03318661719`) | legitimate cluster, phone-number id | coherent, like 447 |

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
