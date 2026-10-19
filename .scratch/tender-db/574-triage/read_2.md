# 574 unit 1: read of rows 9..17 of rows.json (plan 2653, rule v13)

These are nine plan-2653 `held` entries. v13 released the v12 bind-guard hold `consortium_removed` on all nine.
- Eight are held `top_stale`.
- One, row 11 (TSO), is held `head_current`.

Each row is judged as if `old_name -> new_name` shipped today (2026-10-19). The evidence is the entry's dates
(`head_newest`, `top_newest`, `rival_newest`), its spellings and its V/T/H (voting/top/head).

Labels follow `548-triage/u1/judgements.json`:
- **R**: right.
- **N**: neutral.
- **W**: wrong (another entity, a member for the group, or a clearly worse name).
- **REVERT**: an older name that is no longer used.

**Tally: 9 R, 0 N, 0 W, 0 REVERT.**
- Every new name is the identifier's holder, with the consortium sentence or role marker removed. Each also passes
  v13's token-subset test.
- No row has a fresher rival that is a better current name.
- Confidence is lowest on two rows:
  - Row 15 (European Service Network): the new name's casing is worse, though the row is still R.
  - Row 13 (Antea): the new name is the brand. The legal name, `Antea France`, is the rival.

**Lapse dates.**
- Row 11 is the only clock hold. Its head was printed 2025-07-25, so it leaves `head_current` on **2028-07-24**
  (`head_newest` + 1,095 days).
- `top_stale` (365 days, with a rival cluster voting) only gets staler with time. Those eight rows ship only if one of
  two things happens on a later plan:
  - a spelling in the top cluster is printed again within 365 days of that plan's `as_of`;
  - the rival cluster stops voting.
- A verdict here may therefore be used long after today. Rows 13, 14, 15 and 16 have tops printed in 2025, so they
  are the likeliest to be re-printed and ship.

## Method and limits

- I made 10 public GETs, all `rc=200`, 2 s or more apart. Each is logged in `574-triage/api_requests.log`, and
  the bodies are in `574-triage/bodies/`:
  - `GET /v1/organizations/{id}` for each of the nine orgs. All nine still serve `old_name`. Each one's
    `identifier` and `mentions` equal the plan entry's `identifier` and `n`, so the listing is current.
  - `GET /v1/notices/20680957/content` for row 9. This is the 2019 SITCO award (tender 6435162). Issue 551's saved
    reads list org 10142430 as a LOT-5 winner of tender 6435162 and of the DIGIT-TM tender 8407303.
- I made no /v1/sql request, no POST and no ssh, and I edited nothing outside this directory.
- **Limit:** the three allowed endpoints cannot list an org's tenders, and no saved file holds a tender id for rows
  10..17. Their verdicts rest on:
  - the plan's spellings and dates, including the cluster arithmetic below;
  - the live org record;
  - the shape of the identifier (SIRET/SIREN, KBO);
  - what is known about the entities. These facts were not checked against a register.
- Checking the rivals:
  - I worked out which spelling is the rival from the spelling counts. When the five listed spellings sum to V, the
    split is exact.
  - `rival_newest > top_newest` holds in rows 9 and 12. In both, the rival is the same firm: in row 9 with a
    "(leader)" marker, in row 12 with its legal form. Neither rival is the 548 v11 gate-(d) harm shape.

## Verdicts

| # | org_id | old -> new | V/T/H | head_newest | top_newest | rival_newest | hold | verdict |
|---|---|---|---|---|---|---|---|---|
| 9 | 10142430 | `Tratos+ Consortium — Trasys International EEIG (group leader)` -> `TRASYS International EEIG` | 10/6/2 | 2020-04-14 | 2019-10-07 | 2020-04-14 | top_stale | **R** (high; dated read) |
| 10 | 10653142 | `Eiffage génie civil réseaux (mandataire)/SAT` -> `EIFFAGE GENIE CIVIL RESEAUX` | 7/4/1 | 2017-11-18 | 2024-09-09 | 2019-08-13 | top_stale | **R** |
| 11 | 10658772 | `TSO (Mandataire)` -> `TSO` | 17/14/2 | 2025-07-25 | 2026-06-17 | 2024-02-26 | head_current (lapses 2028-07-24) | **R** (high) |
| 12 | 10808017 | `Groupement Bonglet Breuil Expert Étanche DBTS` -> `Bonglet` | 9/6/1 | 2017-12-20 | 2019-11-22 | 2020-10-13 | top_stale | **R** |
| 13 | 11099325 | `Antea group (Mandataire du groupement conjoint)` -> `Antea Group` | 7/4/1 | 2018-02-14 | 2025-07-24 | 2024-03-27 | top_stale | **R** |
| 14 | 13183959 | `SAS Cogite (mandataire du groupement)` -> `Cogite SAS` | 9/7/1 | 2019-03-07 | 2025-04-29 | 2021-12-01 | top_stale | **R** (high) |
| 15 | 13230085 | `European Service Network SA (leader Consortium Lumiere)` -> `EUROPEAN SERVICE NETWORK sa` | 5/3/1 | 2019-04-08 | 2025-03-17 | 2023-07-14 | top_stale | **R** (medium; casing) |
| 16 | 13340328 | `Titulaire nº 1 sur 3: groupement conjoint composé de groupe Sirius, département IAMTE (mandataire) et KPMG SA` -> `Groupe Sirius` | 7/5/1 | 2019-04-30 | 2025-03-20 | 2021-08-20 | top_stale | **R** |
| 17 | 13396243 | `STUDIO 02 ARCHITECTES (mandataire)` -> `studio 02 ARCHITECTES` | 10/6/2 | 2019-05-07 | 2024-08-13 | 2024-08-08 | top_stale | **R** |

## Per row

**Row 9: 10142430, BE VAT BE0667692570 (KBO 0667.692.570). R, high confidence.**

The dated read settles which entity the number belongs to. The 2019 SITCO award, notice 20680957, has one
organization section (ORG-42, result RES-11) with:
- `TED-NATIONALID` `BE0667692570`;
- `TED-OFFICIALNAME` `Trasys International EEIG` in all 24 languages;
- the address Rue d'Arlon 53, 1040 Brussels.

The notice never prints "Tratos". The served tenders 6435162 (2019) and 8407303 (2018) carry mention names
`Trasys International EEIG` and `TRASYS International EEIG` on this org. So the number is the EEIG's own
registration. "Tratos+" is a bidding consortium that the EEIG leads, and it has no KBO number of its own.

The clusters:
- Top: `TRASYS International EEIG` 4 and `Trasys International EEIG` 2, 6 of 10, newest 2019-10-07. That date
  matches tender 6435162's award version.
- Head: the two "Tratos+ … (group leader)" casings, 2 votes.
- Rival: `TRASYS/Trasys International EEIG (leader)`, 2 votes. It is fresher than the top (2020-04-14, the
  head's date, probably the same notice), but it is the same firm with a role marker.

The new name serves the holder rather than the group, which is the opposite of a member-for-group change. An EEIG
is itself a grouping, but it is a registered legal person, and this number is its registration.

**Row 10: 10653142, SIRET 48773772800070. R.**

The old head (2017) is the mandataire `Eiffage génie civil réseaux` plus "/SAT", a co-contractor in the group.

The clusters (4 of them):
- Top: `EIFFAGE GENIE CIVIL RESEAUX` 2, a double-space variant 1 and `Eiffage génie civil réseaux — SNC` 1.
  That is 4 of 7, newest 2024-09-09.
- Two 1-vote clusters outside the top and head. One is `Eiffage travaux publics réseaux`; the other spelling is not
  listed. The newer of the two is 2019-08-13.

The top's 2024 print is the newest of all, so `Eiffage Génie Civil Réseaux` is the name in current use for this
SIRET. The change is not a revert to the older Eiffage TP Réseaux name, and SAT is not carried over. The all-caps
form is the register style that the 548 U1c consortium reads accepted.

**Row 11: 10658772, SIRET 74725212000015. R, high confidence.**

The top holds 14 of 17 votes (`TSO` 10, `TSO SAS` 2, `TSO — SAS` 1, and 1 unlisted) and is fresh: 2026-06-17. The
head `TSO (Mandataire)` has 2 votes and was last printed 2025-07-25. The rival is `TSO CHELLES (Mandataire)`, 1 vote,
2024-02-26, which is the same firm (TSO, the rail-works contractor, is based in Chelles).

The change only drops the role marker.

This is the only row on a clock: it ships on the first plan after 2028-07-24. By then the top will be stale unless
TSO is printed again after 2027-07. But the firm prints often (10 of the plain `TSO` votes), so a re-print is likely.

**Row 12: 10808017, SIRET 31543485200036. R.**

The head is a 2017 group sentence: "Groupement Bonglet Breuil Expert Étanche DBTS". A groupement has no SIRET, so the
identifier belongs to one member, and the group names Bonglet first.

The spellings sum to V exactly:
- top `Bonglet` 6;
- rival `Bonglet SA` 2, newest 2020-10-13, a separate cluster under the vote key;
- head 1.

The rival is fresher than the top (2019-11-22), but it is the same firm with its legal form, not a different
current name. Serving `Bonglet` rather than `Bonglet SA` is at worst neutral. The change replaces a group with the
holder, so R.

**Row 13: 11099325, SIRET 39320673500655. R.**

The head and the new name are the same brand, `Antea Group`. The change drops "(Mandataire du groupement conjoint)".

The top holds 4 of 7 votes (`Antea Group` 2, `ANTEA GROUP` 1, `Antea group` 1) and was last printed 2025-07-24. The
rival (2024-03-27) is older than the top. It is `Antea France (mandataire)` or an unlisted 1-vote spelling. Antea
France is the legal name of the French Antea Group company, and that print still has a role marker.

A legal-name serve (`Antea France`) would be marginally better, but it is not on offer. The brand is the same entity,
so this is R rather than W.

**Row 14: 13183959, SIRET 53039736300041. R, high confidence.**

The family is `sas` on both sides. The listed spellings `Cogite SAS` 2, `cogite sas` 2, `COGITE SAS` 1, `COGITE` 1
and `Cogite` 1 make up the top's 7 of 9, newest 2025-04-29. The 1-vote rival (2021-12-01) is older.

The change moves the legal form to the end and drops "(mandataire du groupement)".

**Row 15: 13230085, KBO 0428.304.785. R, medium confidence.**

All five spellings are 1-vote prints of European Service Network, the Brussels communications agency. The head
(2019-04-08) adds "(leader Consortium Lumiere)". The top is 3 of 5, newest 2025-03-17. The rival (2023-07-14) is
another spelling of the same firm.

The new name `EUROPEAN SERVICE NETWORK sa` has worse casing than the head: an all-caps name with a lower-case form.
But it removes a consortium-role annotation that presents the firm as a consortium leader on every mention.

Weighing the two: the 548 U1c skim filed `sas cabinet kerguen - mANDROIT`, a far worse casing, as neutral. Here
the dropped annotation is the larger gain, so R. N is the fallback if casing is weighted above annotation removal. It
is not W, because it is the same entity under a readable name.

**Row 16: 13340328, SIRET 43156861700028. R.**

The head is a 2019 award sentence: "Titulaire nº 1 sur 3: groupement conjoint composé de groupe Sirius, département
IAMTE (mandataire) et KPMG SA". The number cannot be KPMG's, because KPMG SA's SIREN is 775726417 and this SIREN is
431568617. So it is the mandataire's: Groupe Sirius.

The spellings sum to V exactly:
- top `GROUPE SIRIUS` 3, `Groupe Sirius` 1 and `groupe sirius` 1: 5 of 7, newest 2025-03-20;
- rival `sirius Département IAMTE` 1, 2021-08-20, the same firm's department;
- head 1.

The new name is the holder alone.

**Row 17: 13396243, SIRET 49446222900035. R.**

- Top: `studio 02 ARCHITECTES` 5 and `STUDIO 02 ARCHITECTES` 1, 6 of 10, newest 2024-08-13.
- Head: `STUDIO 02 ARCHITECTES (mandataire)`, 2 votes, 2019-05-07.
- Rival: the short form `STUDIO 02` / `studio 02`, 2 votes, 2024-08-08. It is older than the top and is the same
  firm.

The change drops the role marker. The mixed case `studio 02 ARCHITECTES` is the majority print, 5 votes, and reads
as the firm's own styling, so it is not a worse name.

## Verdicts (machine-readable)

```json
{"10142430": "R", "10653142": "R", "10658772": "R", "10808017": "R", "11099325": "R",
 "13183959": "R", "13230085": "R", "13340328": "R", "13396243": "R"}
```
