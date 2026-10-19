# 574 unit 1: read of rows 18..26 of rows.json (plan 2653, rule v13)

These nine rows are `held` entries on plan 2653. v13 released their v12 bind-guard hold:
- 6 rows were held `consortium_removed`: 18, 19, 20, 23, 24 and 25.
- 3 rows were held `family`: 21, 22 and 26.

Rows 18 to 22 and row 24 are now held `top_stale`. Rows 23, 25 and 26 are now held `head_current`.

I judged each row as if `old_name -> new_name` shipped today (2026-10-19). The evidence is the entry's dates
(`head_newest`, `top_newest`, `rival_newest`), its spellings and its V/T/H (voting/top/head).

The labels follow `548-triage/u1/judgements.json`:
- **R**: right.
- **N**: neutral.
- **W**: wrong (another entity, a member for the group, or a clearly worse name).
- **REVERT**: an older name that is no longer used.

**Tally: 8 R, 0 N, 1 W, 0 REVERT.**
- The W is row 22, 17915448: `ECO CENTRO SARDEGNA SRL` -> `RTI COSTITUENDO MASNATA CHIMICI S.P.A. + CHIMICA SARDA
  S.R.L. (DATI MANDATARIA)`. The new name is a consortium sentence with an annotation. Confidence is high, and the
  skeptic should read it first.
- It came in through the **family** H ≤ 1 release. 548 U1 read that class 22/22 R, so this is its first recorded
  wrong.
- Row 22 is held `top_stale`: its top was last printed 2022-02-28. It ships only if that RTI spelling is printed
  again, so the risk is small, but it is not zero. No serve hold or bind guard catches a name that *adds* a
  consortium marker. 548's optional "role-marker-added serve hold" (U1d) was never built.
- Row 26 (SAREP, srl -> S.p.A.) is R with medium confidence.

## Method and limits

- I made 9 public GETs, `GET /v1/organizations/{id}`, one for each org. All returned `rc=200`, at least 2.5 s apart.
  They are logged in `574-triage/api_requests.log` (12:56:37Z to 12:57:01Z), and the bodies are saved as
  `574-triage/bodies/org_<id>.json`.
- All nine orgs still serve `old_name`. For each one, the `identifier` and the `mentions` count equal the plan entry's
  `identifier` and `n`, so the listing is current. Every identifier is `identifier_kind: national`, none is
  provisional, and none has merged identifiers.
- I made no /v1/sql request, no POST and no ssh, used no git, and edited nothing outside this directory.
- **Limit (same as read_1):** none of the three allowed endpoints lists an org's tenders, and I had no tender or notice
  id for these nine orgs. So I could not open the procedures behind `head_newest`, `top_newest` or `rival_newest`.
  Each verdict rests on:
  - the plan's spellings and dates;
  - the live org record;
  - the identifier's shape and check digit, all verified locally: the 4 SIRETs and SIRENs and SIREN 433900834 pass
    Luhn, P.IVA 01301570915 (office code 091, Nuoro) and 04585551007 (office code 100, Roma) pass, and CIF
    B18967729 passes (B = S.L., 18 = Granada);
  - what is known about the entities;
  - for row 21, the issue 357 slice-5 country verdict on FN223804Z.
- How I read the rival: for each row I checked whether `rival_newest > top_newest`, which is true only in row 18. That
  rival is `FORMA6`, the top's own name without the space. No rival is a better current name for its identifier.

## When each would ship

These dates assume the as_of rules from 548's filter. Boundary days may be off by one.
- **Row 25** (22048766): `head_current` lapses after 2026-10-23. `top_stale` fires from 2026-11-24 (3 clusters,
  top_newest 2025-11-24). So it ships on a plan between those dates unless new prints arrive. **This is the
  earliest row of the nine.**
- **Row 26** (22140242): `head_current` lapses after 2026-11-06. It has 2 clusters, so `top_stale` can never fire. It
  ships on the first plan after that date.
- **Row 23** (19426241): `head_current` lapses after 2026-11-12. `top_stale` would not fire until 2027-09-15. It ships
  on the first plan after 2026-11-12.
- **Rows 18 to 22 and 24** (`top_stale`): their heads are already more than 1,095 days old. Each ships only if its top
  cluster is printed again, and only within 365 days of that plan's as_of. Each top has been stale since:
  - row 18: 2026-10-03;
  - row 19: 2026-07-10;
  - row 20: 2026-09-29;
  - row 21: 2025-12-16;
  - row 22: 2023-02-28;
  - row 24: 2026-09-23.

## Verdicts

| # | org_id | old -> new | V/T/H | head_newest | top_newest | rival_newest | v12 hold | now | verdict |
|---|---|---|---|---|---|---|---|---|---|
| 18 | 13824614 | `Forma 6 (mandataire du groupement) Racine BE/SAS Techniques et Chantiers/…/BECG` -> `FORMA 6` | 5/3/1 | 2019-08-09 | 2025-10-03 | 2026-04-27 | consortium_removed | top_stale | **R** |
| 19 | 14983077 | `UTE Greening Ingeniería Civil y Ambiental, S. L. y  pk. 28` -> `Greening Ingeniería Civil y Ambiental, S. L.` | 16/12/2 | 2020-03-17 | 2025-07-10 | 2020-08-05 | consortium_removed | top_stale | **R** |
| 20 | 15774943 | `Mandataire: Sogea Martinique SAS` -> `SOGEA MARTINIQUE` | 7/4/1 | 2020-09-11 | 2025-09-29 | 2025-07-15 | consortium_removed | top_stale | **R** |
| 21 | 15783942 | `Consortium consisting of Kontron Transportation Austria AG and Kontron Transportation Sp. z o. o.` -> `Kontron Transportation GmbH` | 5/3/1 | 2020-09-15 | 2024-12-16 | 2021-02-01 | family | top_stale | **R** |
| 22 | 17915448 | `ECO CENTRO SARDEGNA SRL` -> `RTI COSTITUENDO MASNATA CHIMICI S.P.A. + CHIMICA SARDA S.R.L. (DATI MANDATARIA)` | 11/8/1 | 2021-10-12 | 2022-02-28 | 2022-02-28 | family | top_stale | **W** (high) |
| 23 | 19426241 | `ingerop conseil et ingenierie (Mandataire du Groupement)` -> `INGEROP Conseil et Ingénierie` | 26/19/4 | 2023-11-13 | 2026-09-15 | 2026-06-16 | consortium_removed | head_current | **R** |
| 24 | 21555937 | `inddigo sas (Mandataire)` -> `INDDIGO SAS` | 5/3/1 | 2023-07-07 | 2025-09-23 | 2024-12-11 | consortium_removed | top_stale | **R** |
| 25 | 22048766 | `BOUYGUES BATIMENT ILE DE FRANCE (mandataire)` -> `BOUYGUES BATIMENT ILE DE FRANCE` | 6/4/1 | 2023-10-24 | 2025-11-24 | 2025-02-10 | consortium_removed | head_current | **R** |
| 26 | 22140242 | `S.A.R.E.P. SRL (Mandante)` -> `SAREP S.p.A.` | 4/3/1 | 2023-11-07 | 2026-07-03 | — | family | head_current | **R** (medium) |

None of the nine is a revert. In every row the top was printed after the head (`top_newest > head_newest`).

## Per row

**Row 18: 13824614, SIRET 38238763700053. R.**

SIREN 382387637 is the Forma 6 architecture practice. The head is a 2019 consortium sentence: Forma 6 as mandataire,
followed by seven co-contractors. The `SAS` that set off the v12 `family` reading belongs to the member "SAS
Techniques et Chantiers", not to Forma 6.

The top is `FORMA 6`: 3 of 5 votes, newest 2025-10-03. The only fresher print, `FORMA6` (1 vote, 2026-04-27), is the
same name without the space. So the firm still bids under this name in 2026, and the fresher rival does not compete
with it. The new name is the holder alone and passes the U1c token-subset test.

**Row 19: 14983077, CIF B18967729 (ES). R.**

`B` marks a sociedad limitada and `18` is Granada. A UTE has its own `U` NIF, so this B number belongs to the S.L.
member, not to the joint venture.

The top cluster holds 12 of 16 votes across three spellings of "Greening Ingeniería Civil y Ambiental S.L." (5 + 4 +
3) and was last printed 2025-07-10. That is well after:
- the head, `UTE … y pk. 28` (2 votes, newest 2020-03-17);
- the rival `Greening, S. L.` (2 votes, newest 2020-08-05).

So the short form is older, not a later rename. The change drops the UTE wrapper and the partner "pk. 28". It is the
holder's full name, printed through 2025.

**Row 20: 15774943, SIRET 34222703000018. R.**

SIREN 342227030 is Sogea Martinique, the Vinci Construction subsidiary. Issue 357 slice 4 confirms that this org
stands in FR with a matching name on the same SIREN.

The top holds 4 of 7 votes, `SOGEA MARTINIQUE` 3 and `SOGEA MARTINIQUE SAS` 1, the same name under the vote key's
legal-form convention. It was last printed 2025-09-29. The rivals are older (newest 2025-07-15) and worse: a
`Groupement SOGEA / AGENOV / ACB` sentence and the bare brand `SOGEA`.

The change drops the "Mandataire:" prefix and the form. Read_1 row 4 (Canon France) is the precedent.

**Row 21: 15783942, Firmenbuch FN 223804z (AT). R.**

FN plus digits plus a check letter is an Austrian company-register number. It names one Austrian company, and that is
the Austrian member of the head's consortium:
- the head is a 2020 consortium sentence: Kontron Transportation Austria AG and the Polish Kontron Transportation Sp.
  z o.o.;
- the AG itself is the 1-vote rival, newest 2021-02-01;
- the top, `Kontron Transportation GmbH`, holds 3 of 5 votes, newest 2024-12-16.

The legal form moving from AG to GmbH, under the same FN and with only later prints, is a conversion or rename of the
same register entity. A German or other foreign GmbH would not carry an Austrian FN. Issue 357 slice 5 adds an
independent print: a Bulgarian buyer's row on FN223804Z carries a Cyrillic transliteration of "Kontron Transportation
GmbH". That row was moved to AT with high confidence.

This is the 548 cross-guard row. It leaves both `family` and `consortium_removed`, lands in `top_stale`, and ships
only if the GmbH spelling is printed again.

**Row 22: 17915448, P.IVA 01301570915 (IT, office code 091, Nuoro). W: a group sentence served as one company's name.
High confidence.**

The old name is a clean company name, `ECO CENTRO SARDEGNA SRL`: 1 vote, 2021-10-12.

The new name is the 8-vote top `RTI COSTITUENDO MASNATA CHIMICI S.P.A. + CHIMICA SARDA S.R.L. (DATI MANDATARIA)`.
It is:
- a temporary grouping of firms ("raggruppamento temporaneo di imprese costituendo");
- naming two companies;
- followed by the annotation "(dati mandataria)", meaning the record carries the lead firm's details.

The other cluster is the same sentence ending "(mandataria)": 2 votes, also dated 2022-02-28. So all 10 RTI votes
come from one date, which looks like one multi-lot procedure. That is the votes-per-notice shape behind 548's
Barameda and CPR wrongs.

Whoever holds this P.IVA, the change is wrong:
- If it is Eco Centro Sardegna S.r.l., the change moves the name to another entity's grouping.
- If it is the lead firm's, the right name would be `MASNATA CHIMICI S.P.A.`. Even then, the RTI sentence with its
  annotation is a clearly worse name. 548 U1b labelled 9168556, which adds a "Partner konsorcjum" prefix, W. 548 U1
  family labelled 3728305 and 10037554 W because their new names carry annotations.
- The best case for the new name is N, never R.
- The office code 091 (Nuoro) makes the Cagliari-area lead firm a less likely holder. I could not confirm that
  locally.

Why it was released: the v12 `family` guard fired on srl -> S.P.A. with H = 1, and v13's family H ≤ 1 release let it
go. No later bind guard looks for a consortium marker *added* in the new name. The `consortium_removed` guard covers
only the opposite direction.

**Row 23: 19426241, SIRET 48962613500391. R.**

SIREN 489626135 is INGEROP Conseil et Ingénierie. The top holds 19 of 26 votes (`INGEROP CONSEIL ET INGENIERIE` 7,
`INGEROP Conseil et Ingénierie` 6, `ingerop conseil et ingenierie` 2, and more) and was last printed 2026-09-15. The
head is the "(Mandataire du Groupement)" role print: 4 votes, 2 procedures, newest 2023-11-13. The rival (newest
2026-06-16) is older than the top.

The new name is the mixed-case, accented form of the same denomination, with the role marker dropped. It passes the
token-subset test.

**Row 24: 21555937, SIRET 40225042700125. R.**

SIREN 402250427 is Inddigo SAS, the consultancy. The top holds 3 of 5 votes (`INDDIGO SAS` 2, `INDDIGO` 1) and was
last printed 2025-09-23. The rival is the 1-vote typo `INDIGGO SAS` (2024-12-11). The change only drops "(Mandataire)"
and fixes the casing.

**Row 25: 22048766, SIREN 433900834. R.**

All six prints name Bouygues Bâtiment Île-de-France: `… ILE DE FRANCE` 2, `… ILE DE FRANCE,` 1, `… ILE-DE-FRANCE` 1,
`(mandataire)` 1 and `bouygues bâtiment IDF` 1. The SIREN passes Luhn. The top holds 4 of 6 votes and was last printed
2025-11-24, after the head (2023-10-24). The change only drops "(mandataire)", and it is the same entity either way.

This is the first row of the nine to age out. It ships on a plan between about 2026-10-23 and 2026-11-24.

**Row 26: 22140242, P.IVA 04585551007 (IT, office code 100, Roma). R, medium confidence.**

The org has two clusters:
- the top, which joins `SAREP S.p.A.` (2 votes) and `S.A.R.E.P. SRL` (1 vote) under the vote key, newest
  2026-07-03;
- the head, `S.A.R.E.P. SRL (Mandante)` (1 vote, 2023-11-07).

The change drops the role marker "(Mandante)" and takes the newest-printed form, S.p.A. An srl turning into an S.p.A.
on the same P.IVA is a routine conversion. 548 U1 family recorded 16674633 ("srl -> S.p.A. conversion") as R.

Doubt: both S.p.A. votes may come from a single 2026 notice, and I could not open it. If that buyer printed the
legal form wrongly and the company is still an S.r.l., the change is a same-entity form error, which would be N, not
W. It is never another entity, because the abbreviation and the P.IVA are the same.

## Verdicts (machine-readable)

```json
{"13824614": "R", "14983077": "R", "15774943": "R", "15783942": "R", "17915448": "W",
 "19426241": "R", "21555937": "R", "22048766": "R", "22140242": "R"}
```
