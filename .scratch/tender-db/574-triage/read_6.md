# 574 unit 1: read of rows 45..53 of rows.json (plan 2653, rule v13)

These nine rows are `held` entries on plan 2653. All nine are now held `head_current`, and all nine had the v12
bind-guard hold `consortium_removed`, which v13 released.

I judged each row as if `old_name -> new_name` shipped today (2026-10-19). The evidence is the entry's dates
(`head_newest`, `top_newest`, `rival_newest`), its spellings and its V/T/H (voting/top/head).

The labels follow `548-triage/u1/judgements.json`:
- **R**: right.
- **N**: neutral.
- **W**: wrong (another entity, a member for the group, or a clearly worse name).
- **REVERT**: an older name that is no longer used.

**Tally: 9 R, 0 N, 0 W, 0 REVERT.**
- Every row drops a consortium wrapper or a role marker and serves the holder's own name. The wrappers are "Groupement
  …", "ARGE … c/o", "(Mandataire)", "(Architecte mandataire)" and "bevollmächtigter Vertreter der
  Bietergemeinschaft". In every row the new name is the bare print that the same identifier carries in later notices.
- Row 45 (Topserv, DE) is confirmed three ways: the issue 311 biege-batch review (2026-08-30), a live eForms notice
  and the VAT check digit.
- Row 46 (RS Suter AG, CH) has the least support of the nine: 4 mentions in all. I rate it R with medium-high
  confidence. The residual risk is described under the row.
- In rows 49, 51 and 52 the served spelling is a minority casing or accent form within the top cluster. That is a
  display choice for the same name, and these rows are R on the dropped marker alone. Precedents: read_3 row 24
  (Inddigo) and read_5 row 38 (Mobilink).
- No row is REVERT. In every row the top was printed after the head.
- The skeptic has no wrongs to read here.
- **None of the nine ages out soon.** The earliest, row 46, leaves `head_current` after 2027-05-17. Rows 48, 51, 52
  and 53 ship only if their top is printed again.

## Method and limits

- I made 10 public GETs, all `rc=200`, about 3 s apart. They are lines 62 to 71 of `574-triage/api_requests.log`:
  - `GET /v1/organizations/{id}` for each of the nine orgs (13:06:46Z to 13:07:10Z). The bodies are saved as
    `574-triage/bodies/org_<id>.json`.
  - `GET /v1/notices/26398148/content` (Topserv, row 45, 13:07:21Z). The body is saved as
    `574-triage/bodies/n_26398148.json`.
- **Note on the rate:** a parallel reader's last GET (line 61) was at 13:06:39Z, 7 s before my first. None of its
  GETs landed between mine.
- All nine orgs still serve `old_name`. For each one, the `identifier` and the `mentions` count equal the plan entry's
  `identifier` and `n`, so the listing is current. None is provisional, and none has merged identifiers.
  - The seven FR identifiers are `identifier_kind: national`.
  - The DE and CH identifiers are `vat`.
- I checked every identifier's check digit locally:
  - all seven SIRETs, and their SIRENs, pass Luhn;
  - DE356317230 passes the German USt-IdNr check (ISO 7064 MOD 11,10);
  - CHE105587665 passes the Swiss UID mod-11 check (weights 5 4 3 2 7 6 5 4).
- I made no /v1/sql request, no POST and no ssh, used no git, and edited nothing outside this directory.
- **Limit (same as read_1 and read_5):** none of the three allowed endpoints lists an org's tenders.
  - For the eight FR and CH rows I had no tender or notice id. Their verdicts rest on the plan's spellings and dates,
    the live org record and the identifier check.
  - For row 45, issue 311's review record (`issues/311-batch-verdicts.json`, and
    `519-nhs-digit-codes/measure/m3-raw/admin_case.json`, cohort `biege-batch-2026-08-30`) listed notice ids, and I
    opened one of them.
- **Limit: spellings are capped at 5.** Row 48's entry shows 14 of its 17 votes. The other 3 votes are in single-vote
  clusters I could not see. The other eight rows show every vote.
- How I read the rival: for each row I checked whether `rival_newest > top_newest`. That is true only in row 51, where
  the rival is the same company printed with a "(FIT)" annotation. No rival is a better current name for its
  identifier.

## When each would ship

These dates assume the as_of rules from 548's filter. Boundary days may be off by one.
- `head_current` holds while `head_newest` is within 1,095 days.
- `top_stale` holds when `clusters > 1 + (H > 0)` and `top_newest` is more than 365 days old.

For all nine rows H = 1 or 2, so `top_stale` can fire only with 3 or more clusters.

**Ship on the first plan after the head ages out.** These rows have 2 clusters, so `top_stale` can never fire.

| # | org_id | leaves `head_current` after | note |
|---|---|---|---|
| 46 | 22618129 | 2027-05-17 | earliest of the nine |
| 47 | 22619950 | 2027-05-21 | |
| 45 | 22609164 | 2027-05-30 | |
| 50 | 22658782 | 2027-07-09 | |

**Ships in a window.** This row has 3 clusters, and its top is still fresh when the head ages out.

| # | org_id | leaves `head_current` after | top stale after | note |
|---|---|---|---|---|
| 49 | 22638904 | 2027-06-26 | 2027-09-08 | ships between these dates, or later if `GTM SUD-OUEST TP GC` is printed again |

**Ship only if the top is printed again.** These rows have 3 or more clusters, and their top is already stale, or will
be, by the time the head ages out.

| # | org_id | leaves `head_current` after | top stale since |
|---|---|---|---|
| 48 | 22628426 | 2027-06-11 | 2026-10-02 (already stale) |
| 51 | 22667236 | 2027-07-29 | 2027-02-02 |
| 52 | 22696968 | 2027-08-23 | 2027-03-25 |
| 53 | 22729641 | 2027-09-25 | 2026-12-01 |

## Verdicts

| # | org_id | old -> new | V/T/H | head_newest | top_newest | rival_newest | v12 hold | verdict |
|---|---|---|---|---|---|---|---|---|
| 45 | 22609164 | `Topserv Vertriebs GmbH - bevollmächtigter Vertreter der Bietergemeinschaft` -> `Topserv Vertriebs GmbH` | 8/6/2 | 2024-05-30 | 2024-07-11 | — | consortium_removed | **R** (high) |
| 46 | 22618129 | `ARGE RSW c/o RS Suter AG` -> `RS Suter AG` | 4/3/1 | 2024-05-17 | 2024-10-15 | — | consortium_removed | **R** (medium-high) |
| 47 | 22619950 | `Groupement CHAUVIER SAS associé à ACTION HABITAT` -> `CHAUVIER` | 8/7/1 | 2024-05-21 | 2026-06-22 | — | consortium_removed | **R** |
| 48 | 22628426 | `Groupement LEASYS FRANCE et AUTOMOBILE DE BOURGOGNE` -> `LEASYS France` | 17/12/1 | 2024-06-11 | 2025-10-02 | 2025-08-07 | consortium_removed | **R** |
| 49 | 22638904 | `GTM SUD-OUEST TP GC (Mandataire)` -> `GTM Sud-Ouest TP GC` | 6/4/1 | 2024-06-26 | 2026-09-08 | 2026-07-22 | consortium_removed | **R** |
| 50 | 22658782 | `Groupement ADG ENERGY (mandataire) / BALTHAZARD / CARRION TP / SERP / CITEOS` -> `ADG ENERGY` | 5/4/1 | 2024-07-09 | 2025-07-21 | — | consortium_removed | **R** |
| 51 | 22667236 | `GROUPEMENT GEOFIT (Mandataire) - SINTEGRA (cotraitant)` -> `Géofit` | 11/7/1 | 2024-07-29 | 2026-02-02 | 2026-08-14 | consortium_removed | **R** |
| 52 | 22696968 | `Atelier MIMA (Architecte mandataire)` -> `Atelier MIMA` | 6/4/1 | 2024-08-23 | 2026-03-25 | 2024-12-10 | consortium_removed | **R** |
| 53 | 22729641 | `GROUPEMENT SETEC INTERNATIONAL SAS / SETEC ALS` -> `SETEC INTERNATIONAL` | 5/3/1 | 2024-09-25 | 2025-12-01 | 2024-12-18 | consortium_removed | **R** |

## Per row

**Row 45: 22609164, USt-IdNr DE356317230 (check digit valid). R, high confidence.**

There are two clusters:
- the top, `Topserv Vertriebs GmbH`: 6 of 8 votes, newest 2024-07-11;
- the head, the representative-role print `… - bevollmächtigter Vertreter der Bietergemeinschaft`: 2 votes, one
  procedure, newest 2024-05-30.

Two other sources agree:
- **Issue 311's biege-batch review (2026-08-30)** read this org as `member-row-mislabelled`
  (`single-company-consortium-mention`), with high confidence: "Leave DE356317230 attached to Topserv; classify as a
  single-company representative row, not a vehicle". It found the bare name with the identical VAT in notices
  24120934 and 26398148, three times each. It also tied this org to the same joint bid as GVS-Service, Dr. Butze and
  Kurt Müller. read_5 row 44 (GVS-Service, R) is the sibling row.
- **Live notice 26398148** (DE1 result notice, issued 2024-07-10, buyer EK-UNICO GmbH, lots "Hygienepapier") prints
  `Topserv Vertriebs GmbH` with CompanyID `DE356317230` three times: ORG-0026, ORG-0037 and ORG-0047. Each is a
  Tenderer in a different tendering party (TPA-0001, TPA-0003 and TPA-0005). The notice carries no
  "Bietergemeinschaft" wording.

The change drops the role sentence and serves the company's own name. The top is over two years old, but it is still
newer than the head, so this is not a REVERT. The row has 2 clusters, so it ships after 2027-05-30.

**Row 46: 22618129, UID CHE105587665 (check digit valid). R, medium-high confidence.**

There are two clusters:
- the top, `RS Suter AG`: 3 of 4 votes, newest 2024-10-15;
- the head, `ARGE RSW c/o RS Suter AG`: 1 vote, 2024-05-17.

"ARGE" is a Swiss/German joint venture (Arbeitsgemeinschaft), and "c/o RS Suter AG" gives the ARGE's address at the
lead member. Three of four prints bind this UID to the bare company name, and the top was printed five months after
the head. So the UID is RS Suter AG's, and the change drops the joint-venture wrapper.

Residual risk: a Swiss ARGE can hold its own UID (for example, when it is registered for VAT). If CHE-105.587.665 were
the ARGE's own number, the new name would be a member for the group (W). Three separate prints of the bare AG name
against an ARGE's number are much less likely than one ARGE label printed with the lead's number. I could not open a
notice for this row (no id is reachable through the allowed endpoints). If a later reader wants certainty, this is
the row to check against the UID register before 2027-05-17.

**Row 47: 22619950, SIRET 48820308400030. R.**

There are two clusters:
- the top, which joins `CHAUVIER` (5 votes) and `CHAUVIER SAS` (2 votes) under the vote key: 7 of 8 votes, newest
  2026-06-22;
- the head, `Groupement CHAUVIER SAS associé à ACTION HABITAT`: 1 vote, 2024-05-21.

The head names CHAUVIER SAS as the lead and ACTION HABITAT as its associate. No print names ACTION HABITAT alone on
this SIRET. The served name is the majority spelling, without the form. The form is lost (`family_old: sas`,
`family_new: null`), but this is the spelling most buyers use for the same company, so it is not a worse name.

**Row 48: 22628426, SIRET 41336018100049. R.**

The org has six clusters:
- the top, which joins `LEASYS France` 5, `LEASYS FRANCE` 4 and `LEASYS France SAS` 3: 12 of 17 votes, newest
  2025-10-02;
- the head, `Groupement LEASYS FRANCE et AUTOMOBILE DE BOURGOGNE`: 1 vote, 2024-06-11;
- `Groupement conjoint LEASYS France (mandataire) / SCP CHOPARD Cannes`: 1 vote;
- three more single-vote clusters, not shown (the spellings list is capped at 5). `second` = 1, so none holds more
  than one vote.

Both visible consortium prints name LEASYS France first, and the second marks it as mandataire. With 12 bare prints
in three spellings, the SIRET is Leasys France's. Automobile de Bourgogne and SCP Chopard are co-contractors. The
served spelling is the most frequent one. The three unseen votes cannot outweigh 12.

Its top has been stale since 2026-10-02. After 2027-06-11 it moves to `top_stale` and ships only on a fresh print.

**Row 49: 22638904, SIRET 50140147500022. R.**

All six prints name the same company:
- the top, 4 votes, newest 2026-09-08. It joins `GTM SUD-OUEST TP GC` 2, `GTM Sud-Ouest TP GC` 1 and, by the vote
  count, `GTM SUD-OUEST TP GC, SASU` 1;
- the rival, by elimination `GTM SUD OUEST` (the vote key keeps "TP GC" apart, and strips the SASU form): 1 vote,
  2026-07-22;
- the head, `GTM SUD-OUEST TP GC (Mandataire)`: 1 vote, 2024-06-26.

The change drops the role marker. The served casing, `GTM Sud-Ouest TP GC`, is a 1-vote spelling within the top
cluster. Against `GTM SUD-OUEST TP GC` it is a casing choice only, and the change is right on the dropped marker alone.
The rival is a shorter spelling of the same company and is older than the top.

The row has 3 clusters. It ships after 2027-06-26 and before 2027-09-08, unless the top is printed again before then.

**Row 50: 22658782, SIRET 84031454600028. R.**

There are two clusters. The top, `ADG ENERGY`, holds 4 of 5 votes, newest 2025-07-21. The head, `Groupement ADG
ENERGY (mandataire) / BALTHAZARD / CARRION TP / SERP / CITEOS`, has 1 vote (2024-07-09). The head itself marks ADG
ENERGY as mandataire. The other four members never appear alone on this SIRET. The change serves the mandataire's
own name, and the new name passes the token-subset test.

**Row 51: 22667236, SIRET 34217401800370. R.**

The org has three clusters:
- the top, which joins `GEOFIT` 6 and `Géofit` 1 (accents fold under the vote key): 7 of 11 votes, newest 2026-02-02;
- the rival, `GEOFIT (FIT)`: 3 votes, 2026-08-14;
- the head, `GROUPEMENT GEOFIT (Mandataire) - SINTEGRA (cotraitant)`: 1 vote, 2024-07-29.

The head marks GEOFIT as mandataire and SINTEGRA as cotraitant. No print names SINTEGRA alone. This is the one row
where the rival is newer than the top. But the rival is the same company with a parenthesised annotation, so it is
not a competing name, and it does not make the top a REVERT. The served form, `Géofit`, is the accented, mixed-case
spelling of the top. It is a 1-vote display choice against `GEOFIT`, the same name, so the row is R on the dropped
groupement alone.

Its top turns stale on 2027-02-02, before the head ages out (2027-07-29). The row ships only if `GEOFIT` is printed
again.

**Row 52: 22696968, SIRET 79810877500023. R.**

All six prints name the same practice:
- the top, `ATELIER MIMA` 2 + `Atelier MIMA` 2, newest 2026-03-25;
- the rival, `Atelier MIMA (Mandataire, Architecture)`, 2024-12-10;
- the head, `Atelier MIMA (Architecte mandataire)`, 2024-08-23.

The change drops the role marker. The new name is one of the two tied top spellings, and it passes the token-subset
test.

Its top turns stale on 2027-03-25, before the head ages out (2027-08-23). The row ships only on a fresh print.

**Row 53: 22729641, SIRET 72201317400114. R.**

The org has three clusters:
- the top, which joins `SETEC INTERNATIONAL` 2 and `SETEC INTERNATIONAL SAS` 1: 3 of 5 votes, newest 2025-12-01;
- the head, `GROUPEMENT SETEC INTERNATIONAL SAS / SETEC ALS`: 1 vote, 2024-09-25;
- the rival, `Groupement SETEC INTERNATIONAL / SETEC ALS / GETEC OCEAN INDIEN`: 1 vote, 2024-12-18.

Both consortium prints name SETEC INTERNATIONAL first, and the only bare prints name SETEC INTERNATIONAL. So the
SIRET is Setec International's, and SETEC ALS and GETEC OCEAN INDIEN are co-contractors. SETEC ALS is a sister
company in the same group, but no print names it alone on this SIRET. The served name is the majority spelling,
without the form.

Its top turns stale on 2026-12-01, long before the head ages out (2027-09-25). The row ships only if the top is
printed again.
