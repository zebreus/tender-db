# 574 unit 1: read of rows 54..62 of rows.json (plan 2653, rule v13)

These nine rows are `held` entries on plan 2653, and all nine are now held `head_current`. v13 released their v12
bind-guard hold:
- 7 rows were held `consortium_removed`: 54, 55, 57 and 59 to 62.
- 2 rows were held `family`: 56 (spa -> srl) and 58 (srl -> spa). Both have H = 1.

I judged each row as if `old_name -> new_name` shipped today (2026-10-19). The evidence is the entry's dates
(`head_newest`, `top_newest`, `rival_newest`), its spellings and its V/T/H (voting/top/head).

The labels follow `548-triage/u1/judgements.json`:
- **R**: right.
- **N**: neutral (the same entity, and neither name is better).
- **W**: wrong (another entity, a member for the group, or a clearly worse name).
- **REVERT**: an older name that is no longer used.

**Tally: 8 R, 1 N, 0 W, 0 REVERT.**
- Six FR rows (54, 55, 57, 60, 61, 62) drop a "Groupement …" wrapper and its partners. Each serves the holder's own
  name, which is the name the same SIRET or SIREN carries in later bare prints.
- Row 56 (IT) replaces an RTI sentence that names another firm, the capogruppo Ares Line, with the firm that holds
  the P.IVA, Decima srl. It is R with medium confidence.
- Row 58 (IT) fixes a truncated spelling (`Biosensor Italia srl` -> `Biosensors International Italia S.p.A.`). It is R
  with low-medium confidence because the served legal form is doubtful. At worst it is a form slip on the same entity
  (N).
- Row 59 (ES) is **N**. The identifier is the UTE's own `U` NIF, so the UTE is the entity. The change keeps both
  members but drops the "UTE" marker and title-cases the forms (`Sl`, `Sau`). It also gains the accents. Neither name
  is clearly better.
- The skeptic has no wrongs to read here. If one row gets a second look, it should be row 56, then row 58.
- **None of the nine ships before 2027-09-30.** Every row has 2 clusters, so `top_stale` cannot fire. Each one ships on
  the first plan after its head ages out of `head_current`.

## Method and limits

- I made 10 public GETs, all `rc=200`. They are lines 53 to 61 and 72 of `574-triage/api_requests.log`:
  - `GET /v1/organizations/{id}` for each of the nine orgs (13:06:15Z to 13:06:39Z, about 3 s apart, with no other
    reader's line in between). The bodies are saved as `574-triage/bodies/org_<id>.json`.
  - `GET /v1/organizations/8909987` (13:09:51Z), the `Sofaxis` org, for row 61. The body is saved as
    `574-triage/bodies/org_8909987.json`.
- All nine orgs still serve `old_name`. For each one, the `identifier` and the `mentions` count equal the plan entry's
  `identifier` and `n`, so the listing is current. None is provisional, none has merged identifiers, and all nine
  identifiers are `identifier_kind: national`.
- I checked every identifier's check digit locally:
  - the six SIRETs and their SIRENs, and the SIREN 301291738, pass Luhn;
  - P.IVA 04761760281 (office code 028, Padova) and P.IVA 10034370964 (office code 096, Milano) pass the Italian
    check;
  - CIF U09983099 passes the Spanish CIF check (control digit 9). Prefix `U` means a UTE, and province code 09 is
    Burgos.
- I made no /v1/sql request, no POST and no ssh, used no git, and edited nothing outside this directory.
- **Limit (same as read_1):** none of the three allowed endpoints lists an org's tenders.
  - For row 55 I reused two tender bodies that 548 U3 had saved. They are
    `548-triage/u3/bodies/j4_td_832931.json` and `548-triage/u3/bodies/j4_td_430805.json`.
  - For row 61 I reused `548-triage/u3/bodies/j5_t_393107.json`, together with the live Sofaxis org record.
  - For the other seven rows I had no tender or notice id. Their verdicts rest on:
    - the plan's spellings and dates;
    - the live org record;
    - the identifier check;
    - what is known about the entities.
- **Rival:** no row has a rival (`rival_newest` is null in all nine). In every row the top was printed after the head.

## When each would ship

These dates assume the as_of rules from 548's filter:
- `head_current` holds while `head_newest` is within 1,095 days.
- `top_stale` holds when `clusters > 1 + (H > 0)` and `top_newest` is more than 365 days old.

All nine rows have `clusters = 2` and `H >= 1`. So `top_stale` cannot fire, and each row ships as `change:R1` on the
first plan after the date below. Boundary days may be off by one.

| # | org_id | leaves `head_current` after | note |
|---|---|---|---|
| 54 | 22741977 | 2027-09-30 | earliest of the nine |
| 56 | 22778540 | 2027-10-07 | |
| 57 | 22781837 | 2027-10-08 | |
| 55 | 22755534 | 2027-10-10 | |
| 58 | 22824583 | 2027-11-15 | |
| 60 | 22876648 | 2027-12-20 | |
| 61 | 22915652 | 2028-01-03 | |
| 62 | 22958521 | 2028-01-08 | |
| 59 | 22857451 | 2028-05-06 | H = 2; the only N |

## Verdicts

| # | org_id | old -> new | V/T/H | head_newest | top_newest | rival_newest | v12 hold | verdict |
|---|---|---|---|---|---|---|---|---|
| 54 | 22741977 | `GROUPEMENT ROUGNON ET DEGRE CELSIUS SERVICES` -> `Rougnon` | 7/6/1 | 2024-09-30 | 2026-10-05 | — | consortium_removed | **R** |
| 55 | 22755534 | `Groupement COLOMBUS CONSULTING / BALTHAZAR / GREENFLEX` -> `Colombus Consulting` | 10/9/1 | 2024-10-10 | 2026-02-18 | — | consortium_removed | **R** (high) |
| 56 | 22778540 | `RTI ARES LINE SPA CAPOGRUPPO` -> `Decima srl` | 4/3/1 | 2024-10-07 | 2026-04-22 | — | family | **R** (medium) |
| 57 | 22781837 | `GROUPEMENT SAS KUBA / LA BANQUE POSTALE` -> `KUBA` | 4/3/1 | 2024-10-08 | 2026-09-11 | — | consortium_removed | **R** |
| 58 | 22824583 | `Biosensor Italia srl` -> `Biosensors International Italia S.p.A.` | 7/6/1 | 2024-11-15 | 2026-07-07 | — | family | **R** (low-medium; worst case N) |
| 59 | 22857451 | `UTE ZENER SOFT CONSULTING SL, TECNICOS ASOCIADOS INFORMATICA SAU` -> `Zener Soft Consulting Sl, Técnicos Asociados Informática Sau` | 8/6/2 | 2025-05-07 | 2026-06-04 | — | consortium_removed | **N** |
| 60 | 22876648 | `Groupement SOCIETE D'ETUDES DE TRAVAUX HYDRAULIQUES ET D'ADDUCTION D'EAU (SETHA - mandataire) / GTIE INFI / SADE TRAVAUX SPECIAUX` -> `SOCIETE D ETUDES, DE TRAVAUX HYDRAULIQUES ET D'ADDUCTION D'EAU` | 4/3/1 | 2024-12-20 | 2026-09-11 | — | consortium_removed | **R** |
| 61 | 22915652 | `GROUPEMENT RELYENS SPS / RELYENS MUTUAL INSURANCE / RELYENS LIFE INSURANCE` -> `RELYENS SPS` | 5/4/1 | 2025-01-03 | 2026-09-10 | — | consortium_removed | **R** (high) |
| 62 | 22958521 | `Groupement PARIN CLAIDIERE - PETER MARCEL - MARISOL - COTE PEINT` -> `PARIN CLAIDIERE` | 5/4/1 | 2025-01-08 | 2026-01-05 | — | consortium_removed | **R** |

## Per row

**Row 54: 22741977, SIRET 42878870700035. R.**

There are two clusters:
- the top, which joins `ROUGNON` (5 votes) and `Rougnon` (1 vote) under the vote key: 6 of 7 votes, newest
  2026-10-05;
- the head, `GROUPEMENT ROUGNON ET DEGRE CELSIUS SERVICES`: 1 vote, 2024-09-30.

Degré Celsius Services is the partner in that one groupement. No print names it alone on this SIRET, so the SIRET is
Rougnon's. The change drops the wrapper and the partner, and the new name passes the token-subset test. The served
casing, `Rougnon`, comes from the minority spelling. Against `ROUGNON` it is a casing choice only, as for read_5 row 38
(Mobilink).

**Row 55: 22755534, SIRET 42299315400067. R, high confidence.**

There are two clusters:
- the top, which joins `COLOMBUS CONSULTING` (6 votes) and `Colombus Consulting` (3 votes): 9 of 10 votes, newest
  2026-02-18;
- the head, `Groupement COLOMBUS CONSULTING / BALTHAZAR / GREENFLEX`: 1 vote, 2024-10-10.

Two saved tender bodies print the bare name on this org id:
- **Tender 832931** (TED, last version 2024-12-16): notice 24378019 prints `Colombus Consulting` and
  `COLOMBUS CONSULTING` as tenderer. Its contract `Colombus lot 1` was concluded 2024-11-15.
- **Tender 430805** (EDF consulting framework, last version 2026-02-18): notice 25529760 prints
  `COLOMBUS CONSULTING` as tenderer and winner. This is the cluster's `top_newest`.

Balthazar and Greenflex are other consultancies in that one groupement. The change serves the mandataire's own name,
in the mixed-case spelling.

**Row 56: 22778540, P.IVA 04761760281 (IT, office code 028, Padova). R, medium confidence.**

There are two clusters:
- the top: `DECIMA S.R.L.` 1, `DECIMA SRL` 1 and `Decima srl` 1, so 3 of 4 votes from three different spellings,
  newest 2026-04-22;
- the head, `RTI ARES LINE SPA CAPOGRUPPO`: 1 vote, 2024-10-07.

The family guard fired because the head's form is spa and the top's is srl. This is not a conversion. The head is an
RTI (temporary grouping) sentence that names its capogruppo, Ares Line S.p.A. To my knowledge Ares Line is the
seating maker in Carrè, Vicenza province. A Vicenza P.IVA would carry office code 024, not 028, although a P.IVA keeps
the office where the firm first registered, so this is weak evidence. Three independent spellings put Decima srl on
this P.IVA after the head was printed. So the likely reading is this: Decima was a mandante in an Ares Line-led RTI, and
one buyer printed the RTI's name with Decima's codice fiscale. Serving `Decima srl` replaces another firm's name with
the holder's own name, like the 548 U1 family "stray" verdicts (JEOL, Progress Rail, Toscana Bus).

Doubt: I could open no notice for this org. If 04761760281 were Ares Line's own P.IVA, the three Decima prints would be
strays and the change would be W. Two facts argue against that:
- the office code 028;
- the org's small mention count (n = 4). A busy supplier like Ares Line would carry bare prints under its own name.

The skeptic should open the 2024-10-07 RTI notice first if a tender id turns up.

**Row 57: 22781837, SIRET 38369397500070. R.**

There are two clusters:
- the top, which joins `KUBA` (2 votes) and `KUBA SASU` (1 vote): 3 of 4 votes, newest 2026-09-11;
- the head, `GROUPEMENT SAS KUBA / LA BANQUE POSTALE`: 1 vote, 2024-10-08.

SIREN 383693975 is not La Banque Postale's (421100645), and no print names La Banque Postale alone. So the SIRET is
Kuba's, and the change drops the wrapper and the bank partner. The served name `KUBA` loses the SAS/SASU form that
the head and one top print carry. It is still the majority bare spelling (2 of the 3 top votes), and the form drop
does not change which entity is named.

**Row 58: 22824583, P.IVA 10034370964 (IT, office code 096, Milano). R, low-medium confidence; worst case N.**

There are two clusters:
- the top, which joins `BIOSENSORS INTERNATIONAL ITALIA S.R.L.` (4 votes) and `Biosensors International Italia
  S.p.A.` (2 votes) under the vote key: 6 of 7 votes, newest 2026-07-07;
- the head, `Biosensor Italia srl`: 1 vote, 2024-11-15.

The head is a truncated spelling: it drops the plural "s" and the word "International". The new name is the Italian
subsidiary's full name, Biosensors International Italia (Biosensors International Group, the stent maker), on the same
P.IVA. So the entity is the same and the new name is better.

Doubt: the form.
- The served spelling carries `S.p.A.`, but 4 of the 6 top votes say `S.R.L.`.
- The pick of the S.p.A. spelling is likely the mixed-case preference (the only mixed-case spelling in the cluster),
  not recency.
- I could not tell whether the S.p.A. prints are newer than the S.R.L. prints.
- If the company converted, the row is R, like 548 U1 family 16674633 (srl -> S.p.A. on one P.IVA). If two buyers
  slipped, the change trades a truncated name with the right form for the full name with a wrong form. That is N,
  not W, because it is never another entity.

**Row 59: 22857451, CIF U09983099 (ES; `U` = Unión Temporal de Empresas, province 09 Burgos; check digit valid). N.**

There are two clusters:
- the top, `Zener Soft Consulting Sl, Técnicos Asociados Informática Sau`: 6 of 8 votes, newest 2026-06-04;
- the head, `UTE ZENER SOFT CONSULTING SL, TECNICOS ASOCIADOS INFORMATICA SAU`: 2 votes, newest 2025-05-07.

This differs from read_3 row 19 (Greening), where the identifier was the S.L.'s own `B` NIF. Here the identifier is
the UTE's own `U` NIF, so the joint venture itself is the entity, and the old name names it exactly. The new name keeps
both members, so it is not a member for the group. The changes are these:
- **Lost:** it drops "UTE", the one token that says this is a joint venture. Without it, the name reads like two
  companies listed in one org.
- **Garbled:** it title-cases the forms to `Sl` and `Sau`.
- **Gained:** it adds the accents (`Técnicos`, `Informática`).

The result is the same entity, with gains and losses on each side and neither name clearly better, so N. The v12
`consortium_removed` guard was arguably right to hold this one: "UTE" here is not a wrapper around a holder, because
the holder is the UTE.

**Row 60: 22876648, SIREN 301291738. R.**

There are two clusters:
- the top, `SOCIETE D ETUDES, DE TRAVAUX HYDRAULIQUES ET D'ADDUCTION D'EAU`: 3 of 4 votes, newest 2026-09-11;
- the head: 1 vote, 2024-12-20. It is a three-member groupement sentence that itself marks SETHA as mandataire, with
  GTIE INFI and SADE Travaux Spéciaux as partners.

The new name is SETHA's full legal name in the registry style (`D ETUDES` without the apostrophe, with a comma). It
loses the acronym, but it serves the holder's own legal name and drops the partners.

**Row 61: 22915652, SIRET 33517109600035. R, high confidence.**

There are two clusters:
- the top, `RELYENS SPS`: 4 of 5 votes, newest 2026-09-10;
- the head, `GROUPEMENT RELYENS SPS / RELYENS MUTUAL INSURANCE / RELYENS LIFE INSURANCE`: 1 vote, 2025-01-03. This is
  the broker with two group insurers.

Two sources agree:
- **Live org 8909987** (`Sofaxis`, 308 mentions) carries the **same** identifier 33517109600035, with SIREN 335171096
  merged in.
- **Saved tender 393107** (Communauté de Communes du Pays Fouesnantais insurance, last version 2026-02-03): notice
  25556499 prints `RELYENS SPS` as tenderer and winner on lot 6, and binds it to org 8909987. Its contract was
  concluded 2025-12-08.

So RELYENS SPS is the current name of the company formerly called Sofaxis, and this SIRET is its own. Relyens Mutual
Insurance is a different legal entity (the former SHAM, org 8969574). The change drops the two insurers and serves
the broker's current name.

Side note, outside the verdict: two orgs carry this one SIRET. They are 22915652 (this row) and 8909987, which still
serves the older name `Sofaxis`. After this row ships, the two will serve different names for one company. That is a
duplicate-identifier question for whoever owns org identity. It does not affect the R.

**Row 62: 22958521, SIRET 49813957500027. R.**

There are two clusters:
- the top, `PARIN CLAIDIERE`: 4 of 5 votes, newest 2026-01-05;
- the head, `Groupement PARIN CLAIDIERE - PETER MARCEL - MARISOL - COTE PEINT`: 1 vote, 2025-01-08.

Parin Claidière is named first. Peter Marcel, Marisol and Côte Peint are the co-contractors, and none of them appears
alone on this SIRET. The change drops the wrapper and the partners, and the new name passes the token-subset test.
