# 574 unit 1: read of rows 36..44 of rows.json (plan 2653, rule v13)

These nine rows are `held` entries on plan 2653, and all nine are now held `head_current`. v13 released their v12
bind-guard hold:
- 8 rows were held `consortium_removed`: 36, 37, 38 and 40 to 44.
- 1 row was held `family`: 39 (srl -> S.p.A., H = 1).

I judged each row as if `old_name -> new_name` shipped today (2026-10-19). The evidence is the entry's dates
(`head_newest`, `top_newest`, `rival_newest`), its spellings and its V/T/H (voting/top/head).

The labels follow `548-triage/u1/judgements.json`:
- **R**: right.
- **N**: neutral.
- **W**: wrong (another entity, a member for the group, or a clearly worse name).
- **REVERT**: an older name that is no longer used.

**Tally: 9 R, 0 N, 0 W, 0 REVERT.**
- Eight rows drop a consortium wrapper or a role marker ("Groupement …", "(mandataire)", "als 2. Mitglied der
  Bietergemeinschaft", "bevollmächtigter Vertreter der Bietergemeinschaft") and serve the holder's own name. That name
  is the bare print the same identifier carries in later notices.
- Row 39 (EMMECCI, srl -> S.p.A. Benefit on one P.IVA) is a legal-form conversion. It is R with medium-high confidence.
- Rows 43 and 44 (DE) are confirmed three ways: the issue 311 biege-batch review (2026-08-30), a live eForms notice
  each, and the VAT check digit.
- The skeptic has no wrongs to read here.
- **None of the nine ages out soon.** The earliest, row 37, leaves `head_current` after 2027-03-04. Rows 36, 38 and 44
  ship only if their top is printed again.

## Method and limits

- I made 11 public GETs, all `rc=200`, at least 2.2 s apart. They are lines 39 to 44, 46, 48 and 50 to 52 of
  `574-triage/api_requests.log`:
  - `GET /v1/organizations/{id}` for each of the nine orgs (13:01:39Z to 13:02:00Z). The bodies are saved as
    `574-triage/bodies/org_<id>.json`.
  - `GET /v1/notices/26874419/content` (Bursch, row 43) and `GET /v1/notices/26868822/content` (GVS, row 44). The
    bodies are saved as `574-triage/bodies/n_<id>.json`.
- **Note on the rate:** a parallel reader's GETs (lines 45, 47 and 49) landed between my org GETs. My own spacing was
  2.2 s or more, but for about 6 s the combined rate against the API was close to 1 per second.
- All nine orgs still serve `old_name`. For each one, the `identifier` and the `mentions` count equal the plan entry's
  `identifier` and `n`, so the listing is current. None is provisional, and none has merged identifiers. The seven FR
  and IT identifiers are `identifier_kind: national`, and the two DE identifiers are `vat`.
- I checked every identifier's check digit locally:
  - all six SIRETs, and their SIRENs, pass Luhn;
  - P.IVA 05648090826 passes (office code 082, Palermo);
  - DE247989569 and DE300491024 pass the German USt-IdNr check (ISO 7064 MOD 11,10).
- I made no /v1/sql request, no POST and no ssh, used no git, and edited nothing outside this directory.
- **Limit (same as read_1):** none of the three allowed endpoints lists an org's tenders.
  - For the seven FR and IT rows I had no tender or notice id. Their verdicts rest on the plan's spellings and dates,
    the live org record, the identifier check and what is known about the entities.
  - For rows 43 and 44, issue 311's review record (`519-nhs-digit-codes/measure/m3-raw/admin_case.json`, cohort
    `biege-batch-2026-08-30`) listed notice ids, and I opened one notice for each row.
- How I read the rival: for each row I checked whether `rival_newest > top_newest`. That is true only in row 36,
  where the rival is another consortium sentence led by ACOPAD. No rival is a better current name for its identifier.
  In every row the top was printed after the head.

## When each would ship

These dates assume the as_of rules from 548's filter: `head_current` holds while `head_newest` is within 1,095 days,
and `top_stale` holds when `clusters > 1 + (H > 0)` and `top_newest` is more than 365 days old. Boundary days may be
off by one.

Row 38's entry suggests that `head_current` is tested before `top_stale`: its top has been stale since 2025-12-09, yet
it reads `head_current`.

**Ship on the first plan after the head ages out.** These rows have 2 clusters, so `top_stale` can never fire, or
their top is still fresh at that date.

| # | org_id | leaves `head_current` after | note |
|---|---|---|---|
| 37 | 22553939 | 2027-03-04 | earliest of the nine |
| 40 | 22572466 | 2027-04-03 | |
| 39 | 22570322 | 2027-04-18 | 3 clusters: ships before 2027-09-15 (`top_stale`) unless the S.p.A. spelling is printed again |
| 41 | 22585173 | 2027-04-23 | |
| 43 | 22597727 | 2027-05-03 | the plan entry also carries `release_withheld: junk_only` (see the row) |
| 42 | 22596745 | 2027-05-08 | |

**Ship only if the top is printed again.** These rows have 3 or more clusters, and their top is already stale, or
will be, by the time the head ages out.

| # | org_id | leaves `head_current` after | top stale since |
|---|---|---|---|
| 36 | 22542189 | 2027-03-15 | 2026-10-22 |
| 38 | 22564157 | 2027-04-08 | 2025-12-09 |
| 44 | 22609154 | 2028-03-24 | 2027-02-06 |

## Verdicts

| # | org_id | old -> new | V/T/H | head_newest | top_newest | rival_newest | v12 hold | verdict |
|---|---|---|---|---|---|---|---|---|
| 36 | 22542189 | `Groupement ACOPAD/LE CANA/AECD/ACPM` -> `ACOPAD` | 6/4/1 | 2024-03-15 | 2025-10-22 | 2026-07-10 | consortium_removed | **R** |
| 37 | 22553939 | `NOUVETRA (mandataire du groupement NOUVETRA-SERPOLLET)` -> `NOUVETRA` | 7/6/1 | 2024-03-04 | 2025-09-15 | — | consortium_removed | **R** |
| 38 | 22564157 | `MOBILINK(mandataire)` -> `Mobilink` | 5/3/1 | 2024-04-08 | 2024-12-09 | 2024-06-17 | consortium_removed | **R** |
| 39 | 22570322 | `EMMECCI SRL` -> `EMMECCI S.P.A. Benefit` | 7/5/1 | 2024-04-18 | 2026-09-15 | 2025-10-06 | family | **R** (medium-high) |
| 40 | 22572466 | `Groupement solidaire conduit par PARTNER FINE ART` -> `PARTNER FINE ART` | 4/3/1 | 2024-04-03 | 2026-09-28 | — | consortium_removed | **R** |
| 41 | 22585173 | `GROUPEMENT ARBONIS CCL` -> `ARBONIS` | 5/4/1 | 2024-04-23 | 2026-02-02 | — | consortium_removed | **R** |
| 42 | 22596745 | `GROUPEMENT FAURIE (MANDATAIRE) / EHTP (co-traitant)` -> `FAURIE SAS` | 7/6/1 | 2024-05-08 | 2026-05-21 | — | consortium_removed | **R** |
| 43 | 22597727 | `Bursch Medizintechnik GmbH als 2. Mitglied der Bietergemeinschaft` -> `Bursch Medizintechnik GmbH` | 12/8/4 | 2024-05-03 | 2026-10-08 | — | consortium_removed | **R** (high) |
| 44 | 22609154 | `GVS Service GmbH - bevollmächtigter Vertreter der Bietergemeinschaft` -> `GVS-Service GmbH` | 22/14/4 | 2025-03-25 | 2026-02-06 | 2024-07-11 | consortium_removed | **R** (high) |

## Per row

**Row 36: 22542189, SIRET 78288385400161. R.**

The org has three clusters:
- the top, `ACOPAD`: 4 of 6 votes, newest 2025-10-22;
- the head, a four-member groupement led by ACOPAD (`Groupement ACOPAD/LE CANA/AECD/ACPM`): 1 vote, 2024-03-15;
- the rival, another groupement led by ACOPAD (`Groupement ACOPAD / Nes&Cité`): 1 vote, 2026-07-10.

Every print names ACOPAD first, and the only bare print is ACOPAD. So the SIRET is ACOPAD's, and LE CANA, AECD, ACPM
and Nes&Cité are co-contractors. The rival is the only print newer than the top. It shows that ACOPAD still bids
under that name in 2026, inside a different consortium, so it does not compete as a name. The change keeps only the
holder, and the new name passes the U1c token-subset test.

The row ships only if `ACOPAD` is printed again: its top turns stale on 2026-10-22, before the head ages out.

**Row 37: 22553939, SIRET 31591977900027. R.**

The org has two clusters:
- the top, which joins `NOUVETRA` (4 votes) and `NOUVETRA SAS` (2 votes) under the vote key: 6 of 7 votes, newest
  2025-09-15;
- the head, a role print (`NOUVETRA (mandataire du groupement NOUVETRA-SERPOLLET)`): 1 vote, 2024-03-04. SERPOLLET is
  the other member of the groupement.

The change drops the role sentence and serves the holder's own name. This is the read_1 row 4 (Canon France)
precedent. It is the first row of the nine to age out, after 2027-03-04.

**Row 38: 22564157, SIRET 37990656300034. R.**

All five prints name the same company:
- the top, `MOBILINK` 2 + `Mobilink` 1, newest 2024-12-09;
- the rival, `MOBILINK (sous-traitant)`, 2024-06-17;
- the head, `MOBILINK(mandataire)`, 2024-04-08.

The change drops the role marker. The served casing, `Mobilink`, comes from a minority spelling within the top
cluster. Against `MOBILINK` it is a casing choice only, and the change is right on the dropped marker alone, as for
read_3 row 24 (Inddigo).

Its top has been stale since 2025-12-09. After 2027-04-08 it moves to `top_stale` and ships only on a fresh print.

**Row 39: 22570322, P.IVA 05648090826 (IT, office code 082, Palermo). R, medium-high confidence.**

The org has three clusters:
- the top: `EMMECCI SPA BENEFIT` 3, `EMMECCI S.P.A. Benefit` 1 and `Emmecci SpA Benefit` 1, so 5 of 7 votes from at
  least three different spellings, newest 2026-09-15;
- the head, `EMMECCI SRL`: 1 vote, 2024-04-18;
- the rival, `EMMECCI SRL (Mandante 2)`: 1 vote, 2025-10-06.

An Italian P.IVA survives a transformation, so an srl becoming an S.p.A. (here also a società benefit) on the same
P.IVA is the same company. 548 U1 family recorded 16674633 ("srl -> S.p.A. conversion") as R, and read_3 row 26
(SAREP) did the same.

Doubt: the srl rival (2025-10-06) is newer than the head. The conversion date may therefore fall in late 2025 or
2026, or that buyer may have copied an old name. Either way the S.p.A. Benefit form is printed after the srl form,
by several buyers' spellings. The worst case is a same-entity form slip (N), never another entity.

**Row 40: 22572466, SIRET 91383543500011. R.**

There are two clusters. The top, `PARTNER FINE ART`, holds 3 of 4 votes, newest 2026-09-28. The head, `Groupement
solidaire conduit par PARTNER FINE ART`, has 1 vote (2024-04-03). The change only drops the groupement wrapper. The
new name passes the token-subset test.

**Row 41: 22585173, SIRET 79582006700143. R.**

There are two clusters. The top, `ARBONIS`, holds 4 of 5 votes, newest 2026-02-02. The head, `GROUPEMENT ARBONIS CCL`,
has 1 vote (2024-04-23). Arbonis is the timber-construction firm, and CCL is the other member of that groupement. No
print names CCL alone on this SIRET, so the identifier is Arbonis's. The change drops the wrapper and the partner.

**Row 42: 22596745, SIRET 33878625400056. R.**

The org has two clusters:
- the top, which joins `FAURIE SAS` (4 votes) and `FAURIE` (2 votes) under the vote key: 6 of 7 votes, newest
  2026-05-21;
- the head, `GROUPEMENT FAURIE (MANDATAIRE) / EHTP (co-traitant)`: 1 vote, 2024-05-08.

The head itself marks Faurie as mandataire and EHTP as co-traitant. The change serves the mandataire's own name, with
its form.

**Row 43: 22597727, USt-IdNr DE247989569 (check digit valid). R, high confidence.**

There are two clusters. The top, `Bursch Medizintechnik GmbH`, holds 8 of 12 votes, newest 2026-10-08. The head is
the member-role print `… als 2. Mitglied der Bietergemeinschaft`: 4 votes, one procedure, newest 2024-05-03.

Two other sources agree:
- **Issue 311's biege-batch review (2026-08-30)** read this org as `member-row-mislabelled`, with high confidence:
  "Leave DE247989569 attached to Bursch; classify as a single-company member row". It found the bare name with the
  identical VAT on 6 notices. Its sibling rows (Böhm as the 3rd member, Fichtner & Traeder as the 4th) follow the same
  per-member convention.
- **Live notice 26874419** (eForms, dispatched 2026-02-11, buyer Knappschaft Kliniken Universitätsklinikum Bochum)
  has ORG-0003 with BT-500 `Bursch Medizintechnik GmbH` and BT-501 `DE247989569`.

The plan entry also carries `release_withheld: junk_only`. That is the rule v9 withholding of the `digits` release,
for the "2." in "2. Mitglied". It concerns the `recent` window's release, not the name served. The row is held
`head_current` either way, and the verdict on the change does not depend on it.

**Row 44: 22609154, USt-IdNr DE300491024 (check digit valid). R, high confidence.**

The org has four clusters:
- the top, `GVS-Service GmbH`: 14 of 22 votes, newest 2026-02-06;
- the head, the two "bevollmächtigter Vertreter der Bietergemeinschaft" spellings, with and without the hyphen: 2 + 2
  = 4 votes, 2 procedures, newest 2025-03-25;
- two 2-vote clusters for fellow consortium members, `Dr. Butze GmbH & Co. KG - Bietergemeinschaftsmitglied` and
  `Kurt Müller GmbH`, newest 2024-07-11.

Two other sources agree:
- **Issue 311's review** read this org as `member-row-mislabelled`, with high confidence: "Leave DE300491024 attached
  to GVS Service GmbH". It found the bare `GVS-Service GmbH` with the identical VAT on notices outside any
  Bietergemeinschaft framing. It flagged the Dr. Butze prints on this VAT as a mis-copied identifier.
- **Live notice 26868822** (eForms, dispatched 2026-02-04, a Prospitalia framework award) has ORG-0007 with BT-500
  `GVS-Service GmbH` and BT-501 `DE300491024`. That is the company's own spelling, with the hyphen.

So the new name is the holder's own current print. It is not one of the members' names, which stay minority rivals.
The head's spelling without the hyphen is the variant.

The row ships only on a fresh print: its top turns stale 2027-02-06, before the head ages out on 2028-03-24.

## Verdicts (machine-readable)

```json
{"22542189": "R", "22553939": "R", "22564157": "R", "22570322": "R", "22572466": "R",
 "22585173": "R", "22596745": "R", "22597727": "R", "22609154": "R"}
```
