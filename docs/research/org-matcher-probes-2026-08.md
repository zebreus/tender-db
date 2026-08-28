# Org-matcher corpus probes — 2026-08-28 (issue 300 design input)

Provenance: produced by an unattended probe agent over bounded /v1/sql reads
during the 300 design run. The session was security-flagged mid-run (a
classifier block on one command shape; the agent split and retried per the
standing single-purpose-command discipline). Consequence, recorded in the
design: every number below that becomes a matcher gate constant is
RE-DERIVED once in Stage 0 under docs/agents/prod-box-reads.md discipline
before any constant freezes. The specimen org/identifier ids below feed
300-exemplars.md.

All measurements complete. Final data follows.

MEASUREMENT REPORT — org-matcher corpus stats (tender-db prod, via /v1/sql sandbox, 2026-08-28)

Schema note: actual columns differ from the task sketch — organizations(id, country, identifier_kind, identifier, name, provisional, created_at, name_norm); organization_names(org_id, lang, name, name_norm). identifier_kind ∈ {national, vat, NULL}. organizations: 12,622,339 rows, id range 1–30,898,296.

== 1. organization_names language coverage ==
SQL: `SELECT COUNT(*), MIN(org_id), MAX(org_id) FROM organization_names`; then per-window `SELECT COUNT(DISTINCT org_id) FROM organization_names WHERE org_id BETWEEN a AND b` and `SELECT COUNT(*) FROM (SELECT org_id FROM organization_names WHERE org_id BETWEEN a AND b GROUP BY org_id HAVING COUNT(DISTINCT lang) >= 2)`. The unwindowed COUNT(DISTINCT org_id) timed out at 10s; 4M-id windows each ran fine. Windows are disjoint and cover the full id range 1–30,898,296, so the sums are EXACT totals, not extrapolations.

- Total rows: 4,403,039
- Distinct orgs with >=1 satellite name: 4,306,297 (sum of windows: 766,488 / 1,254,322 / 568,307 / 412,015 / 354,562 / 911,344 / 31,550 / 7,709 for windows 1–4M, 4–8M, 8–12M, 12–16M, 16–20M, 20–24M, 24–28M, 28M–end)
- Orgs with >=2 distinct langs: 70,598 = 1.64% of orgs that have names (per-window: 15,465 / 11,986 / 11,281 / 8,187 / 6,678 / 16,498 / 503 for 1–4M, 4–8M, 8–12M, 12–16M, 16–20M, 20–24M, 24–end)
- So ~34% of all 12.6M orgs have a satellite name at all; of those, ~98.4% are single-language.
- Lang distribution (top): FRA 1,709,591; DEU 582,925; POL 467,688; ENG 434,382; ITA 232,645; SPA 229,790; NLD 119,277; SWE 86,601; ELL 73,244; RON 67,550; FIN 66,214; CES 56,936.

== 2. Top identifier values by frequency ==
SQL: `SELECT identifier, identifier_kind, COUNT(*) AS c FROM organizations WHERE identifier IS NOT NULL GROUP BY 1,2 ORDER BY 3 DESC LIMIT 30` (ran in budget, no windowing needed). Population: 1,072,004 national + 92,426 vat identifiers; 11,457,909 orgs have NULL identifier.

Top 30 (all kind=national): NIMAT3 32; NIMAT4 29; NIMAT5 19; 0001 19; 123456789 18; 123456 18; 12345678 16; 12345 16; ORG0002 15; ORG0001 15; NIMAT6 15; 43271911 15; 408712 15; 2021003831 14; 1234567 14; 1234 14; ORG0003 13; ORG001 12; NIMAT7 12; 0002 12; ORG003 11; NIMAT9 11; NIMAT8 11; NIMAT10 11; BT501 11; 0004 11; 0003 11; 00001 11; 000000001 11; ORG002 10.
Reading: the head IS the placeholder family — NIMATn, ORGnnn/ORG00nn, BT501, 1234…, 0001…, repeated zeros. The only plausibly real values in the top 30 are 43271911, 408712, 2021003831 (each ≤15 rows). Max multiplicity of any identifier is 32 — there is no giant collision cluster; a matcher should hard-deny the placeholder lexicon (NIMAT\d+, ORG0*\d+, BT501, ^1234…, ^0+\d?$-style) rather than frequency-cap.

== 3. False-split candidate pool (same identifier, >1 org) ==
SQL: `SELECT COUNT(*), SUM(c) FROM (SELECT identifier_kind, identifier, COUNT(*) c FROM organizations WHERE identifier IS NOT NULL GROUP BY 1,2 HAVING COUNT(*) > 1)` → 8,420 (kind, identifier) values shared by >1 org, covering 18,027 org rows (avg 2.14 rows/value).
Country split (`...SUM(CASE WHEN nnull>0...)/nc=1/nc>1` over the same grouping):
- 0 groups where all members share one non-NULL country — same-country identifier duplicates simply do not exist (resolver already merges on (country, kind, identifier)).
- 3,769 groups involve at least one NULL-country row.
- 4,651 groups span >=2 different non-NULL countries.
So the entire false-split pool is a country-mismatch problem: NULL country vs known, or two different country codes for the same entity.

Specimens (SQL: random sample of shared identifiers len>=8 excluding 0000%/1234%/1111%/9999%, then `SELECT id,country,identifier_kind,identifier,provisional,name FROM organizations WHERE identifier IN (...)`), 21 rows:
- 180014045: (5599259, FR, "CNFPT") + (5599260, NULL, "Centre national de la fonction publique territoriale") — same entity, acronym vs full name, NULL country.
- 20445111: (2476219, FI, "Maintpartner Oy") + (5276790, NULL, same name).
- 26710682100015: (4046514, FR, "Centre hospitalier de Montceau-les-Mines") + (4737224, NULL, "Centre hospitalier").
- 49371185: (3377602, CZ) + (6062809, NULL), identical name "Gymnázium".
- 5020490073: (11531400, GB, "EBSCO International Inc") + (23311674, SE, same name) — cross-country split of one supplier.
- 5402696459: (6642, IS, "Kærunefnd útboðsmála") + (22671317, AFG, "The Icelandic Public Procurement Complaint committee") — same entity, one row has garbage country AFG.
- 5562490192: (9743730, SE, "Softronic Aktiebolag") + (23044815, GB, "Softronic AB").
- 61100277: (8323897, CZ) + (8362150, NULL), identical school name.
- 79453852000014: (9626564, FR) + (19527928, GP) — "Grand Port Maritime de la Guadeloupe", FR vs GP country coding.
- A28017143: (4727, ES, "PHILIPS IBÉRICA S.A.,") + (9017282, FI, "Philips Ibérica, S.A. (Sociedad Unipersonal)") + (11663831, NULL, "Philips Ibérica, S. A.") — one Spanish supplier split three ways by buyer-country contamination.
All sampled specimens are false splits of one real entity (NULL-country, buyer-country contamination, or FR/GP-style coding variance), not distinct entities colliding on an identifier.

== 4. Same name_norm, different orgs, one country (name+country residual) ==
Measured on organizations.name_norm + country (organization_names has no country column). SQL per country: `SELECT COUNT(*), SUM(c) FROM (SELECT name_norm, COUNT(*) c FROM organizations WHERE country='XX' AND name_norm IS NOT NULL GROUP BY 1 HAVING COUNT(*) > 1)`.
- AT (60,638 orgs): 5,085 duplicated name_norms covering 28,456 org rows (46.9% of the country's rows sit in a duplicate-name group)
- CZ (71,067 orgs): 5,992 duplicated name_norms covering 34,942 rows (49.2%)
- PT (40,987 orgs): 2,455 duplicated name_norms covering 10,339 rows (25.2%)

Specimens (random dup name_norms, then member rows; id, identifier_kind:identifier, provisional):
- AT "bdo consulting gmbh": 15861816 (none, prov=1); 16195269 (national:9110016379333); 17931328 (national:217731V) — ERsB-style vs Firmenbuch-style identifier for one company, plus a provisional row.
- AT "call consult fertschnig gmbh": 16361545 (national:9110018019596); 20476216 (none, prov=1); 22759757 (national:391876P) — same pattern.
- AT "stadt bruck an der mur": 15049646 (national:9110003673925); 21650538 (none, prov=1).
- CZ "liberecký kraj" (4 orgs): 1438215 (national:70891508 — the real IČO); 2019561 (none, prov=1); 15036307 (national:7089150008); 15075537 (national:708991508) — the last two are TYPO variants of the real IČO. Typo'd identifiers actively create splits that name+country would catch.
- CZ "raven cz a.s.": 5054230 (none, prov=1); 10021416 (national:25884581).
- CZ "technocrane s.r.o.": 2393002 (none, prov=1); 23129284 (vat:CZ26353458) — kind mismatch (national vs vat) also splits.
- PT "município de alvaiázere" (4 orgs): 21521463 (national:506605949 — real NIF); 21605110 (none, prov=1); 22552609 (national:ORG0001MUNICPIODEALVAIZERE — placeholder built from the name); 22752504 (national:50605949 — dropped-digit typo).
- PT "oliveiras, sa": 1721773 (none, prov=1); 4914278 (vat:PT501157344).
- PT "refer telecom serviços de telecomunicações sa": 1302911 (national:505065630); 2784771 (none, prov=1).
- PT "comando da logística da força aérea — dmsa": 11553494 (none, prov=1); 12370990 (national:AVDAFORAAREAPORTUGUESA1 — name-derived junk identifier).
Pattern: in every sampled group the duplicates are one real entity split by (a) a provisional identifier-less row, (b) two different national-register schemes under one identifier_kind, (c) typo'd digits, or (d) name-derived placeholder identifiers. Same-name-same-country genuine distinct entities did not appear in the sample.

== 5. Legal-form suffix frequency (sample) ==
Sample: 20,000 organization_names.name rows, 5,000 each from org_id ranges 1–2M, 6–8M, 14–16M, 21–23M (`SELECT name FROM organization_names WHERE org_id BETWEEN a AND b LIMIT 5000`), tokenized locally (trailing-phrase match for multi-word forms, else last token).
27.3% of sampled names (5,451/20,000) end in a recognizable legal form. Ranked families (count in 20k):
GmbH/mbH 967; sp. z o.o. (incl. spelled-out "…ograniczoną odpowiedzialnością") 518; SRL/S.r.l. 517; S.A./SA 442; Ltd/Limited 377; AS / A/S / a.s. 309; s.r.o./spol. s r.o. 263; S.L./S.L.U./S.A.U. 254; AB 239; BV/B.V. 183; S.p.A./SpA 159; Oy/Oyj 154; SAS 149; ЕООД/ООД (Cyrillic) 130; Lda 113; d.o.o. 113; AG 111; GmbH & Co. KG 108; Kft/Zrt/Bt 84; NV/N.V. 72; SARL 64; sp. j./sp. k. 62.
Punctuation/spacing variance within one family is heavy (s.r.o. 216 / s. r. o. 16 / s.r.o 4 / spol. s r.o. 25 in raw counts; sp. z o.o. 359 vs sp. z o. o. 38; S.A. 265 vs S.A 23) — the stripper must normalize dots/spaces, handle spelled-out Polish/Hungarian ("…odpowiedzialnością" 88, "…társaság" 51 as raw trailing tokens) and Cyrillic ЕООД (60). Caveat: many trailing tokens are city names (paris 130, katowice 124, warszawa 123, lyon 84…) from court/tribunal-style official names — legal-form stripping alone won't normalize public-body names.

== 6. Satellite cross-language evidence ==
SQL: random 10 org_ids with >=2 langs from window 20–24M, then `SELECT org_id, lang, name ... IN (...)`. 21 rows:
- 22431710: ENG "Veolia Environmental Services BE" / FRA "VEOLIA ENVIRONMENTAL SERVICES BE SA" / NLD "Veolia N.V." — same entity, suffix+case variance.
- 22503157: FRA "VERBRAEKEN INFRA n.v." / NLD "Verbraeken Infra nv" — case/punct only.
- 22563993: DEU "ROCHE DIABETES CARE ITALY SPA" / ITA "...S.P.A." — punct only.
- 22695260: DEU=FRA "PRODYNA (Schweiz) AG" — identical string filed under two langs.
- 22726928: DEU "DE_PZM Luzern AG" / FRA "FR_PZM Luzern AG" — lang-prefix junk baked into the name.
- 22836715: FRA=NLD "PLUXEE BELGIUM" — identical.
- 23037293: DEU "PostAuto AG" / FRA "CarPostal SA" — genuine translated brand names, zero string overlap; only the shared org row links them.
- 23344120: DEU "Bundesamt für Statistik" / FRA "Office fédéral de la statistique" — genuine translation, zero token overlap.
- 23544319: FRA=NLD "Ernst  & Young Advisory Services" (double space preserved).
- 23294544: ENG "Bertin Exensor AB" / NLD "Het Ministerie van Defensie" — NOT translations: a Swedish supplier and the Dutch MoD under one org_id. Looks like a false merge or a mention mis-capture; the satellite table can carry contradictory evidence and one such case surfaced in a sample of just 10.
Verdict: 8/10 are clearly the same entity (most differ only in case/punct/suffix; 2 are true zero-overlap translations — valuable alias pairs a string matcher can never derive), 1 identical-string duplicate, 1 contaminated.

== Caveats ==
- The only query that exceeded the 10s budget was the unwindowed COUNT(DISTINCT org_id); its windowed replacement covers the full range, so all reported totals are exact counts, not extrapolations.
- #4 was measured on organizations.name_norm (organization_names lacks country); satellite-side residual would need a join I did not attempt within budget.
- #5's 27.3% legal-form rate is over satellite name rows (supplier-heavy, FRA/POL-heavy per the lang distribution), not over all 12.6M organizations.
- Specimen sampling in #3 deliberately excluded placeholder-prefixed identifiers, so it characterizes the realistic-identifier share of the pool; the placeholder family itself is quantified in #2.