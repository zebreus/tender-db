# 574 unit 1: read of rows 0..8 of rows.json (plan 2653, rule v13)

The nine rows are plan-2653 `held` entries with reason `top_stale`. v13 released their v12 bind-guard hold:
6 `consortium_removed`, 2 `family` and 1 `gb_family`. Each row is judged as if `old_name -> new_name` shipped today
(2026-10-19), using the entry's dates (`head_newest`, `top_newest`, `rival_newest`), its spellings and its
V/T/H (voting/top/head).

Labels follow `548-triage/u1/judgements.json`:
- **R**: right.
- **N**: neutral (the same entity, and neither name is better).
- **W**: wrong (another entity, a member for the group, or a clearly worse name).
- **REVERT**: an older name that is no longer used.

**Tally: 8 R, 0 N, 1 W, 0 REVERT.** The W is row 0, 6769473 (Capita). Confidence on it is medium-low, so the skeptic
should read it first. Row 2 (Vattenfall) is R with low confidence.

## Method and limits

- I made 10 public GETs, all `rc=200`, 2 to 3 s apart. Each is logged in `574-triage/api_requests.log`, and the
  bodies are in `574-triage/bodies/`:
  - `GET /v1/organizations/{id}` for each of the nine orgs. All nine still serve `old_name`, and each one's
    `identifier` and `mentions` equal the plan entry's `identifier` and `n`, so the listing is current.
  - `GET /v1/tenders/6207300` for row 8.
- I made no /v1/sql request, no POST and no ssh, and I edited nothing outside this directory.
- **Limit:** this read could only use `/v1/organizations/{id}`, `/v1/tenders/{id}` and `/v1/notices/{id}/content`.
  None of these lists an org's tenders. So, unlike 548 U3, I could not open the procedures dated at
  `head_newest`, `top_newest` or `rival_newest`.
  - The only tender id I had was 6207300 for row 8. 548 U3 had saved it.
  - Every other verdict rests on:
    - the plan's spellings and dates;
    - the live org record;
    - the shape of the identifier (SIRET/SIREN, KBO, a 9-digit GB number);
    - what is known about the entities.
  - Where that leaves real doubt, the row says so.
- How to read the rival column: `top_stale` fires because the top has not been printed in the last 365 days and at
  least 3 clusters exist. The 548 v11 gate-(d) harm shape is a fresher rival that is the real current name. So for each
  row I checked:
  - whether `rival_newest > top_newest`, which is true in rows 4, 5 and 7;
  - what that rival is.

  In none of the nine rows is the rival a better current name for the identifier.

## Verdicts

| # | org_id | old -> new | V/T/H | head_newest | top_newest | rival_newest | v12 hold | verdict |
|---|---|---|---|---|---|---|---|---|
| 0 | 6769473 | `Capita Group Plc` -> `Care and Custody (Health) Ltd` | 11/7/1 | 2015-06-30 | 2018-04-07 | 2015-12-15 | gb_family | **W** (medium-low) |
| 1 | 8121787 | `Groupement CSM Bessac (mandataire)/SADE — Service Travaux Spéciaux/Urbaine de Travaux` -> `Bessac` | 8/5/1 | 2016-04-28 | 2024-05-10 | 2022-09-16 | consortium_removed | **R** |
| 2 | 8500491 | `Vattenfall Europe Windkraft GmbH` -> `Vattenfall AB` | 6/4/1 | 2016-08-19 | 2020-05-18 | 2019-04-09 | family | **R** (low) |
| 3 | 8729140 | `DB ProjektBau GmbH` -> `DB Netz AG` | 30/27/1 | 2016-09-20 | 2021-02-02 | 2018-03-17 | family | **R** |
| 4 | 8752693 | `Canon France (mandataire solidaire du groupement)` -> `Canon France` | 9/7/1 | 2016-10-28 | 2018-10-05 | 2020-09-07 | consortium_removed | **R** |
| 5 | 8821225 | `La blanchisserie toulousaine de sante Groupement de coopération sanitaire` -> `Blanchisserie toulousaine de santé` | 9/5/1 | 2016-10-25 | 2020-05-29 | 2022-04-15 | consortium_removed | **R** (near N) |
| 6 | 9134952 | `Ordina Belgium S.A. (as consortium member)` -> `NV Ordina Belgium` | 8/6/1 | 2016-12-02 | 2024-02-29 | 2016-12-02 | consortium_removed | **R** |
| 7 | 9930896 | `Ingemetrie SAS — bureau Ile-de-France/Nord (Mandataire)` -> `Ingemetrie SAS` | 9/7/1 | 2017-06-24 | 2025-06-26 | 2026-02-19 | consortium_removed | **R** |
| 8 | 9943888 | `sas cereg Ingénierie (mandataire)` -> `CEREG Ingénierie` | 13/9/2 | 2017-06-27 | 2024-06-13 | 2021-02-22 | consortium_removed | **R** |

## Per row

**Row 0: 6769473, GB, identifier 618184140. W (a member for the group), medium-low confidence.**

The identifier is a 9-digit number. It is not a Companies House number, which has 8 characters; its shape fits a GB
VAT or DUNS number. Three different legal names print it:
- `Capita Group Plc`: 1 vote, 2015-06-30. This is the head. It is the parent's former name: The Capita Group Plc is
  now Capita plc.
- `Capita Business Services Ltd (Capita Transformation)`: 3 votes, newest 2015-12-15. This is the rival.
- `Care and Custody (Health) Ltd/Limited`: 7 votes, newest 2018-04-07. This is the top.

When one number covers a parent, its main operating subsidiary and a third company, it is most likely a group-level
number (a VAT group or a parent DUNS). Individual group members' prints carry it. 548 found the same pattern with
subsidiary prints on Capita plc's CH 02081330 (11628199).

Serving `Care and Custody (Health) Ltd` would name one member for the whole number. The 4 Capita Business
Services and Capita Group mentions would then sit under the custody-health company's name. The old head is the stale
former name of the parent, but it names the right umbrella.

Doubt: I could not open the 2018 Care and Custody procedures to see whom the buyer meant by the number, or whether
Care and Custody was still a Capita company then. If 618184140 is Care and Custody's own number and the 2015 Capita
prints are strays, the verdict becomes R. The skeptic should settle that first.

**Row 1: 8121787, SIRET 30158934700022. R.**

The head is a 2016 consortium sentence. It names the mandataire under its old trading name, `CSM Bessac`. The top
cluster holds 5 of 8 votes (`Bessac` 2, `BESSAC` 1, `SASU Bessac` 1, `Bessac (mandataire)` 1) and was last printed
2024-05-10. The SIREN 301589347 is the Bessac SASU (tunnelling).

The top is fresher than the 2-vote rival (2022-09-16), so this is not the fresher-rival shape. The new name is the
holder alone, under its current name, and passes the U1c token-subset test.

**Row 2: 8500491, DE, identifier 2016002480. R, low confidence.**

Three Vattenfall-group names print one DE identifier:
- `Vattenfall Europe Windkraft GmbH`: the head, 1 vote, 2016-08-19.
- `DanTysk Sandbank Offshore Wind GmbH & Co KG`: the rival, 1 vote, 2019-04-09. This is the Vattenfall/SWM offshore
  JV.
- `Vattenfall AB`: the top, 4 votes, newest 2020-05-18.

The number is not a Swedish organisationsnummer (its third digit would have to be 2 or more), nor any register format
I can place. Because the subsidiary, the JV and the parent all print it, it acts as a buyer-side number for
Vattenfall's German wind procurement, not as one company's register entry.

The change moves the name from one stray 2016 subsidiary print to the parent, which:
- covers every mention;
- is 4 of the 6 votes;
- is the freshest print.

This is the opposite direction from row 0's member-for-group shape.

Doubt: if 2016002480 is really the GmbH's own number (for example a Hamburg tax number), then `Vattenfall AB` would
be the parent served for a subsidiary, and the verdict W. The dated procedures were not reachable.

**Row 3: 8729140, identifier TEISOA41512328891. R.**

- The top `DB Netz AG` has 27 of 30 votes and was last printed 2021-02-02.
- The head `DB ProjektBau GmbH` has 1 vote, 2016-09-20. DB ProjektBau ran procurements for DB Netz until it was
  dissolved in 2016 (merged into DB Engineering & Consulting / DB Netz). So the head is a stray agent print on a DB
  Netz buyer org.
- The rival is junk: a contract title parsed as a name ("Planungsleistungen LPH 3/4 ... Zittau", 2 votes, 2018).

The new name is the principal and the plurality, and it is newer than the head, so this is not a revert.

Caveat, which does not change the label: DB Netz AG has traded as DB InfraGO AG since 2024-01-01. The org has no
print after 2021, so the data cannot show the newer name.

**Row 4: 8752693, SIRET 73820526901395. R.**

The SIREN 738205269 is Canon France SAS. The top holds 7 of 9 votes: `Canon France` 4 and `Canon France SAS` 3, the
same name under the vote key's legal-form convention. The only spelling fresher than the top (2020-09-07) is the
1-vote typo `Cacnon France`, so the fresher rival is junk.

The change drops the role annotation "(mandataire solidaire du groupement)".

**Row 5: 8821225, SIRET 13002162900014. R (close to N).**

The SIREN 130021629 is a public body in the 13xxxxxxx range, the GCS's own number. The CHU de Toulouse is SIREN
263100125.

The top holds 5 of 9 votes across three casings of "Blanchisserie toulousaine de santé" and was last printed
2020-05-29. The fresher rivals, `chu de toulouse` 2 and `CHU Toulouse` 1 (newest 2022-04-15), are a member
hospital, which is another entity. They are not a better name for this SIRET.

The old head is the body's full legal name, with the article "La" and the unaccented "sante". The new name is the
same denomination with the accent fixed, without "La" and without the spelled-out legal form. The U1c neutral
15497200 ("Groupement de coopération sanitaire (GCS) Achats du Centre -> GCS Achats du Centre") is the nearest
precedent. This row reads R rather than N only because the head has a diacritic defect and the new name has none.

**Row 6: 9134952, KBO 0428.364.866. R.**

The KBO number is Ordina Belgium NV. The head is a 2016 consortium-role print, "S.A. (as consortium member)". The
rival's newest print has the same date as the head, likely the same notice's "(as consortium leader)" print. The top
holds 6 of 8 votes (`NV Ordina Belgium` 2, `Ordina Belgium NV` 2, `Ordina Belgium` 1, ...) and was last printed
2024-02-29.

The new name is the holder alone, with its Dutch legal form. Ordina has been owned by Sopra Steria since 2023. If
the Belgian NV has since been renamed, no print of that name is on this org, and the change is still forward from a
2016 annotated head. So it is not a revert.

**Row 7: 9930896, SIRET 80076540600015. R.**

The top holds 7 of 9 votes (`ingemetrie` 4, `INGEMETRIE` 2, `Ingemetrie SAS` 1) and was last printed 2025-06-26.
The fresher rival (2026-02-19) is the 1-vote consortium sentence "INGEMETRIE (mandataire) / BETEM ILE-DE-France /
GRUE / EI Thierry HAROUTIOUNIAN". It is the same firm inside a wrapper, so the firm still bids as Ingemetrie in
2026.

The change drops "(Mandataire)" and the office qualifier "bureau Ile-de-France/Nord". It is the same legal entity.
The office label is not a separate company, and no other print carries it.

**Row 8: 9943888, SIRET 49270633800026. R.**

The head has 2 votes and was last printed 2017-06-27. The top holds 9 of 13 votes and was last printed 2024-06-13;
the 2-vote rival is older (2021-02-22). The one dated read shows the new name is the firm's own print on this org:
`GET /v1/tenders/6207300` (007287-2019, published 2019-01-08) prints `CEREG Ingénierie` as a LOT-1 co-winner on org
9943888, whose served name is still `sas cereg Ingénierie (mandataire)`.

The new name drops "sas" together with the role marker. That is the 548 U3 Accenture precedent (legal form dropped
under the vote key's convention), not a member-of-group change.

## Verdicts (machine-readable)

```json
{"6769473": "W", "8121787": "R", "8500491": "R", "8729140": "R", "8752693": "R",
 "8821225": "R", "9134952": "R", "9930896": "R", "9943888": "R"}
```
