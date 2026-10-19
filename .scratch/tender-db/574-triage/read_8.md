# 574 unit 1: read of rows 63..71 of rows.json (plan 2653, rule v13)

These nine rows are `held` entries on plan 2653, and all nine are now held `head_current`. v13 released their v12
bind-guard hold:
- 7 rows were held `consortium_removed`: 64 to 69 and 71.
- 2 rows were held `family`: 63 (srl -> GmbH, H = 1) and 70 (S.P.A. -> S.r.l., H = 1).

I judged each row as if `old_name -> new_name` shipped today (2026-10-19). The evidence is the entry's dates
(`head_newest`, `top_newest`, `rival_newest`), its spellings and its V/T/H (voting/top/head).

The labels follow `548-triage/u1/judgements.json`:
- **R**: right.
- **N**: neutral.
- **W**: wrong (another entity, a member for the group, or a clearly worse name).
- **REVERT**: an older name that is no longer used.

**Tally: 9 R, 0 N, 0 W, 0 REVERT.**
- The seven consortium rows drop a wrapper or a role marker and serve the holder's own name. The markers are
  "-Mandataire groupement", "coordonnateur pour le compte du groupement de commandes …", "mandataire au nom et pour le
  compte de …", "Groupement … / …", "GPT CONJOINT … (MANDATAIRE)/STOA", "(mandataire du groupement)" and
  "(mandataire)". In every row, the new name is the bare print that the same identifier carries in later notices.
  Every new name is the top cluster's majority spelling.
- Row 63 (Brunner & Leiter, IT) serves the German half of a South Tyrolean bilingual name in place of an
  HTML-escaped print (`&amp;`). The P.IVA's office code is 021 (Bolzano). R, medium-high confidence.
- Row 70 (SEDA S.P.A. -> Mediberg S.r.l., IT) is the only row where the old and new names share no token. 7 of 8
  prints name Mediberg, in five spellings. One of them is the Registro Imprese-style spelled-out form. The P.IVA's
  office code is 016 (Bergamo), and Mediberg is a Bergamo-province company. The lone SEDA print is a stray. R,
  medium-high confidence.
- No row is REVERT. In every row the top was printed after the head.
- No rival is newer than its top.
- The skeptic has no wrongs to read here. If a later reader wants to spend a check, rows 70 and then 63 are the ones
  to open.
- **None of the nine ages out before 2028.** The earliest, row 63, leaves `head_current` after 2028-01-21. Rows 64,
  66 and 70 ship only if their top is printed again.

## Method and limits

- I made 9 public GETs, all `rc=200`, about 5 s apart. They are lines 82 to 90 of `574-triage/api_requests.log`.
  - Each one is `GET /v1/organizations/{id}` for one of the nine orgs (13:13:54Z to 13:14:36Z).
  - The bodies are saved as `574-triage/bodies/org_<id>.json`.
  - My first GET came 44 s after a parallel reader's last one (line 81). No other reader's GET landed between mine.
- All nine orgs still serve `old_name`. For each one, the `identifier` and the `mentions` count equal the plan entry's
  `identifier` and `n`, so the listing is current.
  - None is provisional, and none has merged identifiers.
  - All nine identifiers are `identifier_kind: national`, and `identifier_status` is null.
- I checked every identifier's check digit locally:
  - the six SIRETs and their SIRENs pass Luhn;
  - the SIREN 646050476 (row 67) passes Luhn;
  - both P.IVAs pass the Italian check digit (Luhn form). Their office codes (digits 8 to 10) are 021 (Bolzano) for
    02347430213 and 016 (Bergamo) for 01471280162.
- I made no /v1/sql request, no POST and no ssh, used no git, and edited nothing outside this directory.
- **Limit (same as read_1, read_5 and read_6):** none of the three allowed endpoints lists an org's tenders.
  - No local record holds a tender or notice id for any of these nine orgs. I searched the repo for the org ids,
    the identifiers and the names "Mediberg" and "Brunner & Leiter".
  - So every verdict rests on the plan's spellings and dates, the live org record, the identifier check and what is
    known about the entities.
- Every row's spellings list shows all of its votes. Each list sums to `n`.
- How I read the rival: for each row I checked whether `rival_newest > top_newest`. This is false in all three rows
  that have a rival (64, 66 and 70). Each of those rivals is another spelling of the holder.

## When each would ship

These dates assume the as_of rules from 548's filter. Boundary days may be off by one.
- `head_current` holds while `head_newest` is within 1,095 days.
- `top_stale` holds when `clusters > 1 + (H > 0)` and `top_newest` is more than 365 days old.

For all nine rows H = 1 or 2, so `top_stale` can fire only with 3 or more clusters.

1,095 days is three years minus a day whenever 2028-02-29 falls in the span. That is why rows 65 and later land one
day before the calendar anniversary.

**Ship on the first plan after the head ages out.** These rows have 2 clusters, so `top_stale` can never fire.

| # | org_id | leaves `head_current` after | note |
|---|---|---|---|
| 63 | 22968488 | 2028-01-21 | earliest of the nine |
| 65 | 23025749 | 2028-03-16 | |
| 67 | 23139907 | 2028-05-20 | |
| 68 | 23224838 | 2028-08-25 | |
| 69 | 23312263 | 2028-08-31 | |
| 71 | 23410461 | 2028-11-10 | |

**Ship only if the top is printed again.** These rows have 3 or more clusters (more than 1 + 1). Their top turns
stale long before the head ages out. Each row ships on the first plan after the head date only if the top cluster
was printed within the 365 days before that plan.

| # | org_id | leaves `head_current` after | top stale after |
|---|---|---|---|
| 64 | 22970973 | 2028-02-19 | 2027-06-18 |
| 70 | 23330965 | 2028-10-27 | 2027-08-13 |
| 66 | 23131602 | 2029-04-09 | 2027-09-07 |

## Verdicts

| # | org_id | old -> new | V/T/H | head_newest | top_newest | rival_newest | v12 hold | verdict |
|---|---|---|---|---|---|---|---|---|
| 63 | 22968488 | `Brunner &amp; Leiter srl` -> `Brunner & Leiter GmbH` | 6/5/1 | 2025-01-21 | 2026-08-31 | — | family | **R** (medium-high) |
| 64 | 22970973 | `Fondation pour la recherche stratégique (FRS)-Mandataire groupement` -> `Fondation pour la recherche stratégique (FRS)` | 8/5/1 | 2025-02-19 | 2026-06-18 | 2026-03-12 | consortium_removed | **R** (high) |
| 65 | 23025749 | `Communauté d'Agglomération de Haguenau coordonnateur pour le compte du groupement de commandes constitué avec la Communauté de Communes de la Basse Zorn` -> `Communauté d'Agglomération de Haguenau` | 4/3/1 | 2025-03-17 | 2026-07-10 | — | consortium_removed | **R** (high) |
| 66 | 23131602 | `SPL ALES CEVENNES mandataire au nom et pour le compte de la Communauté de communes ALES AGGLOMERATION` -> `SPL ALES CEVENNES` | 9/6/2 | 2026-04-10 | 2026-09-07 | 2025-09-22 | consortium_removed | **R** (high) |
| 67 | 23139907 | `Groupement SIGNAUX GIROD/SIGNAUX GIROD NORD` -> `SIGNAUX GIROD` | 5/4/1 | 2025-05-21 | 2026-03-25 | — | consortium_removed | **R** |
| 68 | 23224838 | `GPT CONJOINT AXES INGENIERIE (MANDATAIRE)/STOA` -> `AXES INGENIERIE` | 6/5/1 | 2025-08-26 | 2026-09-17 | — | consortium_removed | **R** |
| 69 | 23312263 | `COCO ARCHITECTURE (mandataire du groupement)` -> `COCO ARCHITECTURE` | 5/4/1 | 2025-09-01 | 2026-10-15 | — | consortium_removed | **R** (high) |
| 70 | 23330965 | `SEDA S.P.A.` -> `Mediberg S.r.l.` | 8/5/1 | 2025-10-28 | 2026-08-13 | 2026-04-10 | family | **R** (medium-high) |
| 71 | 23410461 | `ARTELIA (mandataire)` -> `ARTELIA` | 4/3/1 | 2025-11-11 | 2026-08-18 | — | consortium_removed | **R** (high) |

## Per row

**Row 63: 22968488, P.IVA 02347430213 (IT, office code 021, Bolzano). R, medium-high confidence.**

There are two clusters:
- the top, which joins `Brunner & Leiter GmbH` 3 and `Brunner & Leiter Srl` 2 under the vote key's legal-form
  convention: 5 of 6 votes, newest 2026-08-31;
- the head, `Brunner &amp; Leiter srl`: 1 vote, one procedure, 2025-01-21.

The head is the same name with the ampersand left HTML-escaped. That is why the vote key keeps it apart from the top.
The P.IVA was issued in Bolzano. In South Tyrol a company commonly registers its name in both languages, as
"… GmbH" and "… Srl", and buyers print either half. Both halves sit on this one P.IVA with identical name tokens, so
they are the same legal entity.

The change does two things:
- it repairs the `&amp;` escape;
- it serves the German half, which is the majority spelling (3 to 2).

GmbH against Srl is a display choice for the same company. 548 U1 family recorded 18179679 ("bilingual S.R.L./G.mbH
-> S.r.l.") as R, and this row is the same class, decided by votes the other way. Dropping the escape makes the new
name better on its own, so the row is R rather than N.

Residual risk: the `family` guard exists because srl -> GmbH can mean an Austrian or German GmbH printed on an
Italian number. Against that: 2 of the 5 top prints carry the Italian form with the same tokens, and the office code
is South Tyrolean. A foreign GmbH would have to share the exact name and be printed three times on another company's
P.IVA. I could not open a notice to confirm the GmbH prints' addresses.

The row has 2 clusters, so it ships after 2028-01-21.

**Row 64: 22970973, SIRET 39409553300060. R, high confidence.**

The org has four clusters:
- the top, `Fondation pour la recherche stratégique (FRS)`: 5 of 8 votes, newest 2026-06-18;
- `Fondation pour la recherche stratégique`: 1 vote. The vote key does not fold the parenthesised acronym;
- `GROUPEMENT FONDATION POUR LA RECHERCHE STRATEGIQUE (FRS)/INSTITUT FRANCAIS DES RELATIONS INTERNATIONALES`: 1 vote;
- the head, `… (FRS)-Mandataire groupement`: 1 vote, 2025-02-19.

The rival (2026-03-12) is one of the two single-vote clusters. Either way it names FRS, and it is older than the top.

Both consortium prints name FRS first, and the head marks it as mandataire. No print names IFRI alone on this SIRET.
So the SIRET is FRS's, and IFRI is the co-contractor. The change drops the role suffix and serves the exact top
spelling.

Its top turns stale after 2027-06-18, before the head ages out (2028-02-19). The row ships only if the top is printed
again.

**Row 65: 23025749, SIRET 20006787400061. R, high confidence.**

There are two clusters:
- the top, `Communauté d'Agglomération de Haguenau`: 3 of 4 votes, newest 2026-07-10;
- the head, the coordinator sentence: 1 vote, 2025-03-17.

The SIREN 200067874 has the 200 prefix of a public body (an EPCI). The head names the Communauté d'Agglomération as
coordinator of a joint purchasing group (groupement de commandes) with the Communauté de Communes de la Basse Zorn.
The coordinator signs for the group under its own SIRET. The new name is the coordinator alone, which is what the
three bare prints say. The row has 2 clusters, so it ships after 2028-03-16.

**Row 66: 23131602, SIRET 52945836600026. R, high confidence.**

The org has three clusters:
- the top, `SPL ALES CEVENNES`: 6 of 9 votes, newest 2026-09-07;
- the head, `SPL ALES CEVENNES mandataire au nom et pour le compte de la Communauté de communes ALES AGGLOMERATION`:
  2 votes from one procedure, 2026-04-10;
- the rival, `SPL ALES CEVENNES Mandataire de la Ville D'ALES`: 1 vote, 2025-09-22.

A société publique locale acts as delegated project owner (mandataire) for its shareholder authorities, and here
those are Alès Agglomération and the Ville d'Alès. The SIREN 529458366 is a company-range number, not a 200-prefix
public-body SIREN, so it belongs to the SPL, not to either authority. The change drops the mandate sentence and serves
the SPL's own name. That name is the majority print. All three clusters name the SPL first.

This head is the freshest of the nine (2026-04-10). It ages out after 2029-04-09. The top turns stale after
2027-09-07, so the row ships only if the top is printed again.

**Row 67: 23139907, SIREN 646050476. R.**

There are two clusters:
- the top, `SIGNAUX GIROD`: 4 of 5 votes, newest 2026-03-25;
- the head, `Groupement SIGNAUX GIROD/SIGNAUX GIROD NORD`: 1 vote, 2025-05-21.

The identifier is a bare 9-digit SIREN. The head is a groupement of SIGNAUX GIROD and its subsidiary SIGNAUX GIROD
NORD. A French groupement has no legal personality and no SIREN of its own, so the number is a member's. The four
bare prints all name SIGNAUX GIROD, and no print names SIGNAUX GIROD NORD alone. So the SIREN is the lead's.

This is not a member-for-group change: the group never had the number. The row has 2 clusters, so it ships after
2028-05-20.

**Row 68: 23224838, SIRET 89800553300010. R.**

There are two clusters:
- the top, `AXES INGENIERIE`: 5 of 6 votes, newest 2026-09-17;
- the head, `GPT CONJOINT AXES INGENIERIE (MANDATAIRE)/STOA`: 1 vote, 2025-08-26.

"GPT CONJOINT" is a groupement conjoint. The head itself marks AXES INGENIERIE as mandataire, and STOA is the
co-contractor. STOA never appears alone on this SIRET. The new name is the mandataire's own name, and the row ships
after 2028-08-25.

**Row 69: 23312263, SIRET 48159528800095. R, high confidence.**

There are two clusters:
- the top, `COCO ARCHITECTURE`: 4 of 5 votes, newest 2026-10-15;
- the head, `COCO ARCHITECTURE (mandataire du groupement)`: 1 vote, 2025-09-01.

The change drops the trailing role marker and nothing else. The top was printed four days before this read. The row
ships after 2028-08-31.

**Row 70: 23330965, P.IVA 01471280162 (IT, office code 016, Bergamo). R, medium-high confidence.**

The org has four clusters, and 7 of its 8 votes name Mediberg:
- the top, 5 votes, newest 2026-08-13. It holds `Mediberg S.r.l.` 3 and two more of the single-vote Mediberg
  spellings. By the vote key's form convention those are most likely `MEDIBERG SRL` and `MEDIBERG`;
- two single-vote clusters. By elimination these are `Mediberg Srl Unipersonale` and `MEDIBERG SOCIETA' A
  RESPONSABILITA' LIMITATA`. The spelled-out form and the extra token keep them apart. The rival (2026-04-10) is one
  of them;
- the head, `SEDA S.P.A.`: 1 vote, one procedure, 2025-10-28.

Why the P.IVA is Mediberg's:
- **Votes and spellings.** Seven prints in five spellings, from different buyers, bind the same company to this
  number. One is `MEDIBERG SOCIETA' A RESPONSABILITA' LIMITATA`, the spelled-out style of the Registro Imprese
  company name. That style usually comes from a registry lookup on the P.IVA itself.
- **Office code.** The office code 016 is Bergamo, and Mediberg S.r.l. is a Bergamo-province company (Calcinate). An
  office code shows only where the number was first issued, so this is supporting evidence, not proof.
- **Dates.** The single SEDA print (2025-10-28) has Mediberg prints on both sides of it: the rival 2026-04-10 and the
  top 2026-08-13. It is not a later rename.

So SEDA S.P.A. is a stray, most likely another bidder or a distributor whose row a buyer filled with Mediberg's P.IVA.
548 U1 family recorded strays of this shape as R: 20957059 (JEOL Nordic AB on a JEOL GmbH VAT), 20861533 and
19852879. The change serves the registry holder under its majority spelling. The S.P.A. -> S.r.l. form change that
set off the v12 `family` guard belongs to two different companies, not to a downgrade of one company.

Residual risk: if SEDA S.p.A. had absorbed Mediberg or held this number, 7 of 8 prints would have to bind the wrong
name. That is unlikely, and the newest prints are Mediberg's. A check would open the 2025-10-28 SEDA procedure, but
no id is reachable through the allowed endpoints.

Its top turns stale after 2027-08-13, before the head ages out (2028-10-27). The row ships only if Mediberg is printed
again in the top spelling family.

**Row 71: 23410461, SIRET 44452352601034. R, high confidence.**

There are two clusters:
- the top, `ARTELIA`: 3 of 4 votes, newest 2026-08-18;
- the head, `ARTELIA (mandataire)`: 1 vote, 2025-11-11.

The SIREN 444523526 is an Artelia company. The NIC 01034 marks one of its establishments. The change drops the
role marker. The row ships after 2028-11-10, the latest of the nine.
