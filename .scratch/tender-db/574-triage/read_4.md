# 574 unit 1: read of rows 27..35 of rows.json (plan 2653, rule v13)

These nine rows are `held` entries on plan 2653, and **all nine are held `head_current`**. v13 released their v12
bind-guard hold:
- 8 rows were held `consortium_removed`: 28 to 35.
- 1 row was held `family`: 27 (Acea, srl -> spa, H = 1).

I judged each row as if `old_name -> new_name` shipped today (2026-10-19). The evidence is the entry's dates
(`head_newest`, `top_newest`, `rival_newest`), its spellings and its V/T/H (voting/top/head).

The labels follow `548-triage/u1/judgements.json`:
- **R**: right.
- **N**: neutral.
- **W**: wrong (another entity, a member for the group, or a clearly worse name).
- **REVERT**: an older name that is no longer used.

**Tally: 9 R, 0 N, 0 W, 0 REVERT.**
- In every row the old name is a consortium sentence (ARGE, RTI, Groupement, Bietergemeinschaft, "Consortium de
  ..."). The new name is the identifier's holder alone, and it is also the lead firm the sentence names first. In
  every row the top was printed after the head (`top_newest > head_newest`).
- No row has a fresher rival that is a better current name. Rows 28 and 32 have a fresher rival, and in both it is
  another consortium print led by the same firm.
- Row 35 (EDEKA) is the only row with a dated read: both sides were opened, and the result is R with high confidence.
- Confidence is lowest on two rows, both R:
  - Row 30 (LSS): `LSS GmbH` is a short form. The 2-vote rival `LSS Licht-, Steuer- und Schaltanlagenbau GmbH` looks
    like the full legal name.
  - Row 31 (Teridéal): the new name keeps a leading `SAS` and a spaced hyphen.
  Neither is a clearly worse name than the consortium sentence it replaces.
- **Rows 28 (2026-11-09) and 27 (2026-11-28) are the first of these nine to leave `head_current`.** Row 27 ships at
  that point. Row 28 goes into `top_stale` instead. See "When each would ship".

## Method and limits

- I made 12 public GETs, all `rc=200` and at least 2 s apart. Each is logged in `574-triage/api_requests.log`
  (13:00:38Z to 13:01:59Z), and the bodies are in `574-triage/bodies/`:
  - `GET /v1/organizations/{id}`, one for each of the nine orgs. All nine still serve `old_name`. For each one, the
    `identifier` and the `mentions` count equal the plan entry's `identifier` and `n`, so the listing is current. None
    is provisional, and none has merged identifiers. Kinds: 5 `vat`, 4 `national`.
  - `GET /v1/tenders/901151` for row 35. This is the head procedure (00152919-2024, published 2024-03-13). 548 U3
    found it for the sibling row 22534691 (Troiber; `548-triage/u3/judge_6.md`).
  - `GET /v1/notices/26865565/content` and `GET /v1/notices/24514828/content` for row 35. Both are bare-name EDEKA
    notices. Their ids come from issue 311's batch verdict on 22534692 (`issues/311-batch-verdicts.json`).
- I made no /v1/sql request, no POST and no ssh, used no git, and edited nothing outside this directory.
- **Limit (same as read_1 and read_3):** none of the three allowed endpoints lists an org's tenders. Apart from row 35,
  no saved file holds a tender or notice id for these orgs: I searched all of `.scratch/` for the org ids, the
  identifiers and the names. So the verdicts for rows 27 to 34 rest on:
  - the plan's spellings and dates, including the cluster arithmetic below;
  - the live org record;
  - the identifier's shape and check digit, all verified locally:
    - P.IVA 05394801004 passes, and its office code 100 is Roma;
    - SIRETs 82329934200076 and 38345880900027 and their SIRENs pass Luhn;
    - DE VATs 114170628, 150516887, 217539751 and 815728669 pass ISO 7064 MOD 11,10;
    - UID CHE-105.901.715 passes its mod-11 check;
    - HRA4169 is a Handelsregister A number. It is unique only within one register court, so it cannot be checked;
  - what is known about the entities. These facts were not checked against a register.

## When each would ship

These dates come from `crates/store/src/head_election.rs`:
- `head_current` holds while `head_newest >= as_of - 1,095 days`.
- `top_stale` is asked after `head_current`. It fires when two things are both true:
  - a rival cluster votes, which with H ≥ 1 means `clusters >= 3`;
  - `top_newest < as_of - 365 days`.

The dates below assume no new prints arrive, and boundary days may be off by one.

| # | org_id | leaves `head_current` after | clusters | top stale from | outcome at lapse |
|---|---|---|---|---|---|
| 28 | 22243890 | **2026-11-09** | 3 | 2025-12-06 (already) | goes to `top_stale`; ships only if `JPF Construction SA` is printed again |
| 27 | 22239725 | **2026-11-28** | 2 | never (no rival) | **ships** on the first plan after |
| 30 | 22378401 | 2026-11-30 | 3 | 2027-07-14 | ships |
| 29 | 22298985 | 2026-12-06 | 3 | 2027-01-19 | ships on a plan between 2026-12-06 and 2027-01-19, otherwise goes to `top_stale` |
| 34 | 22524929 | 2027-02-11 | 2 | never | ships |
| 32 | 22516776 | 2027-02-18 | 4 | 2026-07-18 (already) | goes to `top_stale`; ships only if the top is printed again |
| 33 | 22522314 | 2027-02-21 | 2 | never | ships |
| 31 | 22496469 | 2027-02-28 | 3 | 2027-08-11 | ships |
| 35 | 22534692 | 2027-03-13 | 2 | never | ships |

## Verdicts

| # | org_id | old -> new | V/T/H | head_newest | top_newest | rival_newest | v12 hold | verdict |
|---|---|---|---|---|---|---|---|---|
| 27 | 22239725 | `RTI ACEA AMBIENTE SRL E REFECTA` -> `Acea S.p.A.` | 49/48/1 | 2023-11-29 | 2026-08-07 | — | family | **R** (high) |
| 28 | 22243890 | `Consortium de la "STEP de Lucens" des entreprises: JPF Construction SA et Grisoni-Zaugg SA` -> `JPF Construction SA` | 8/6/1 | 2023-11-10 | 2024-12-06 | 2024-12-23 | consortium_removed | **R** |
| 29 | 22298985 | `ARGE INTEGRATION International Management Consultants GmbH (Lead) - CESI Algérie - INT-Frankfurt HR Consulting OÜ` -> `INTEGRATION International Management Consultants GmbH` | 9/5/2 | 2023-12-07 | 2026-01-19 | 2023-12-07 | consortium_removed | **R** |
| 30 | 22378401 | `ARGE NTM-Bühnenbeleuchtung, bestehend aus LSS Licht-, Steuer- und Schaltanlagen GmbH und Zeiler-Technik GmbH & Co. KG` -> `LSS GmbH` | 21/17/2 | 2023-12-01 | 2026-07-14 | 2024-01-05 | consortium_removed | **R** (medium) |
| 31 | 22496469 | `Groupement TERIDEAL ATLANTIQUE/TERIDEAL SIREV` -> `SAS TERIDEAL - ATLANTIQUE` | 12/8/1 | 2024-02-29 | 2026-08-11 | 2025-12-03 | consortium_removed | **R** (medium) |
| 32 | 22516776 | `ARGE Regierungsbaumeister Schlegel GmbH & Co.KG & BPR Dr. Schäpertöns Consult GmbH & Co.KG & Vogt Bauüberwachungsleistungen UG` -> `Regierungsbaumeister Schlegel GmbH & Co. KG` | 16/10/2 | 2024-02-19 | 2025-07-18 | 2026-08-14 | consortium_removed | **R** |
| 33 | 22522314 | `ARGE ECO Consult GmbH & Co. KG - SustaiNet Group Ltd.` -> `ECO Consult GmbH & Co KG` | 20/18/2 | 2024-02-22 | 2026-10-14 | — | consortium_removed | **R** (high) |
| 34 | 22524929 | `Groupement VILLA BATTESTI ARCHITECTES ET ASSOCIES / BATTESTI ASSOCIES / DIANA DE MARI / BG INGENIEURS CONSEILS / INVEO / PROFILS CONSULTANTS` -> `Villa Battesti Architectes et associés` | 4/3/1 | 2024-02-12 | 2025-03-05 | — | consortium_removed | **R** |
| 35 | 22534692 | `EDEKA Foodservice Stiftung & Co. KG (Bietergemeinschaft mit Union SB-Großmarkt Südbayern GmbH)` -> `EDEKA Foodservice Stiftung & Co. KG` | 6/4/2 | 2024-03-13 | 2026-02-03 | — | consortium_removed | **R** (high; dated read) |

## Per row

**Row 27: 22239725, P.IVA 05394801004 (IT, office code 100, Roma). R, high confidence.**

05394801004 is the codice fiscale and P.IVA of Acea S.p.A., the Rome multi-utility. The top holds 48 of 49 votes
under five casings of one name (`Acea S.p.A.` 21, `ACEA SPA` 8, `Acea Spa` 8, `ACEA S.p.A.` 6, `ACEA S.P.A.` 1, plus
4 votes in spellings not listed). It was last printed 2026-08-07.

The head is a single print from 2023-11-29: `RTI ACEA AMBIENTE SRL E REFECTA`. It is a temporary grouping (RTI) of
the group's waste subsidiary Acea Ambiente S.r.l. with the firm Refecta, printed with the parent's number. The `srl`
that set off v12's `family` guard belongs to the subsidiary in that sentence, not to the holder.

The change replaces a one-off group sentence with the identifier's holder. That holder is also the name nearly every
print uses. This is the reverse of the member-for-group shape, and the H = 1 release is working as 548 U2 intended.
Only 2 clusters exist, so this row ships unread on the first plan after **2026-11-28**. This verdict is its read.

**Row 28: 22243890, UID CHE-105.901.715 (CH). R.**

Three spellings print this UID:
- `JPF Construction SA`: the top, 6 of 8 votes, newest 2024-12-06.
- `Consortium de la "STEP de Lucens" des entreprises: JPF Construction SA et Grisoni-Zaugg SA`: the head, 1 vote,
  2023-11-10.
- `c/o JPF Construction SA - Grisoni Zaugg SA - Marti Construction SA - De Luca SA`: the rival, 1 vote, 2024-12-23.

Both group prints name JPF Construction SA first, and six prints carry the UID under JPF's name alone. So the UID is
JPF Construction SA's, not Grisoni-Zaugg's.

The rival is 17 days fresher than the top, but it is another consortium address print led by JPF. It is not a better
current name, so this is not the gate-(d) shape.

The top has been stale since 2025-12-06 and 3 clusters vote. So when `head_current` lapses after 2026-11-09, this row
goes to `top_stale` instead of shipping. It ships only if JPF Construction SA is printed on this UID again. A
verdict of R holds in that case too.

**Row 29: 22298985, VAT DE114170628. R.**

The holder is INTEGRATION International Management Consultants GmbH (Frankfurt), an EU-funded development
consultancy. The spellings sum exactly to V = 9:
- the top cluster (5): `INTEGRATION INTERNATIONAL MANAGEMENT CONSULTANTS GMBH` 3 and `INTEGRATION International
  Management Consultants GmbH` 2, newest 2026-01-19;
- the head (2): an `ARGE ... (Lead) - CESI Algérie - INT-Frankfurt HR Consulting OÜ` sentence, 2023-12-07;
- the rival (2): a `Consortium INTEGRATION ... – Encompass Alfa Line - INT-Frankfurt HR Consulting OÜ – Ekvita
  S.r.I.s` sentence, also 2023-12-07.

Both consortium prints mark INTEGRATION as the lead. The new name is the mixed-case spelling, which keeps the firm's
own capitalised `INTEGRATION`. It is the better of the two top spellings, not the all-caps one.

**Row 30: 22378401, VAT DE150516887. R, medium confidence.**

The spellings sum exactly to V = 21:
- `LSS GmbH`: the top, 17 votes, newest 2026-07-14.
- `ARGE NTM-Bühnenbeleuchtung, bestehend aus LSS Licht-, Steuer- und Schaltanlagen GmbH und Zeiler-Technik GmbH & Co.
  KG`: the head, 2 votes, 2023-12-01. This is a stage-lighting ARGE; NTM is most likely the Nationaltheater
  Mannheim.
- `LSS Licht-, Steuer- und Schaltanlagenbau GmbH`: the rival, 2 votes, 2024-01-05.

All three name the same stage-lighting firm. LSS is the head's lead member, and the rival spells out the same
initials. Seventeen prints under one VAT, last printed in 2026, make `LSS GmbH` the name buyers use for this number.

Doubt: the rival is probably the fuller legal name, so a register-exact display would be the rival, not the top.
That makes the new name a short form, not a different entity or a member-for-group. Against the ARGE sentence it
replaces, which names two companies, the new name is clearly better. Read_2 row 13 (Antea, brand over legal name) is
the precedent. If a reviewer counts the short form as worse than the legal name, the label falls to N, not W.

**Row 31: 22496469, SIRET 82329934200076 (SIREN 823299342). R, medium confidence.**

The listed spellings sum to 10 of V = 12. With top = 8, second = 3 and head = 1, the split is:
- top (8): `SAS TERIDEAL - ATLANTIQUE` 4, `TERIDEAL-ATLANTIQUE` 2, `SAS TERIDEAL ATLANTIQUE` 1, plus 1 unlisted.
  Newest 2026-08-11.
- rival (3): the bare brand `TERIDEAL` 2, plus 1 unlisted. Newest 2025-12-03.
- head (1): `Groupement TERIDEAL ATLANTIQUE/TERIDEAL SIREV`, 2024-02-29.

No other split reaches 8 with the two unlisted votes. The groupement names Teridéal Atlantique first (the
mandataire), and the top cluster is that firm's name in every listed spelling. So the SIRET is Teridéal Atlantique's,
not Teridéal Sirev's. The rival is the group brand, older than the top, so it is not a better current name.

The displayed spelling keeps a leading `SAS` and a spaced hyphen. `Teridéal Atlantique` would be cleaner, but this is
the top's most-printed spelling of the right entity, and it replaces a two-company groupement sentence. R.

**Row 32: 22516776, VAT DE217539751. R.**

The holder is Regierungsbaumeister Schlegel GmbH & Co. KG, a Munich engineering firm. The spellings sum exactly to
V = 16, in 4 clusters:
- the top (10): the firm's name alone, newest 2025-07-18;
- the head (2): an ARGE with BPR Dr. Schäpertöns Consult and Vogt Bauüberwachung, 2024-02-19;
- `BG Regierungsbaumeister Schlegel / p2m berlin` (2);
- `BG Regierungsbaumeister Schlegel GmbH & Co. KG / p2m berlin GmbH` (2). This and the cluster above are two
  spellings of one Bietergemeinschaft, and one of them carries the fresher `rival_newest` 2026-08-14.

The fresher rival is a bidding consortium led by the same firm. It is not a renamed holder, and a BG has no VAT of its
own, so it prints Schlegel's. The new name is the holder alone, with 10 of 16 votes, and passes the U1c token-subset
test.

The top has been stale since 2026-07-18 and 4 clusters vote. So this row goes to `top_stale` when `head_current`
lapses after 2027-02-18, unless the firm's bare name is printed again first.

**Row 33: 22522314, HRA4169 (DE, Handelsregister A). R, high confidence.**

The holder is ECO Consult GmbH & Co. KG, the Oberaula development consultancy. The spellings sum exactly to V = 20:
- `ECO Consult GmbH & Co KG`: the top, 18 votes, last printed 2026-10-14, five days before this read;
- `ARGE ECO Consult GmbH & Co. KG - SustaiNet Group Ltd.`: the head, 2 votes, 2024-02-22.

An HRA number registers partnerships such as a GmbH & Co. KG, which matches the holder's legal form. The change drops
the ARGE partner SustaiNet. `Co KG` rather than `Co. KG` is the top's own spelling, and it is cosmetic.

**Row 34: 22524929, SIRET 38345880900027 (SIREN 383458809). R.**

This row is small (V = 4) but clean:
- the top is `Villa Battesti Architectes et associés`, 3 votes, newest 2025-03-05;
- the head is a 2024-02-12 groupement of six firms: the architect Villa Battesti first (as mandataire), then
  co-contractors and engineering consultants.

The SIRET is printed with the practice's name alone three times, and only 2 clusters exist. The change names the
mandataire. It is not a member-for-group change, because the identifier is the practice's own.

**Row 35: 22534692, VAT DE815728669. R, high confidence; dated read.**

The head is one procedure:
- tender 901151, 00152919-2024, published 2024-03-13: food supply for the Bavarian prisons (Justizvollzugsanstalten);
- LOT-0010 was awarded (`selec-w`) to `EDEKA Foodservice Stiftung & Co. KG (Bietergemeinschaft mit Union SB-Großmarkt
  Südbayern GmbH)`;
- it was printed on two notices, 23796943 and 26295817. That gives H = 2 with `head_procs` = 1.

Its sibling in the same procedure, 22534691 (Troiber, LOT-0001 to LOT-0009), was read R by 548 U3, on the same shape.

Two later notices print the holder alone, on this VAT, with EDEKA's own contact data:
- **Notice 24514828** (00016283-2025, dispatched 2025-01-08, a framework for catering supplies). ORG-0004 reads `EDEKA
  Foodservice Stiftung & Co. KG`, BT-501 `DE815728669`, Viktoriastr. 26, Köln, `efs.ausschreibung@edeka-foodservice.de`.
  The tender text reads `ANGEBOT EDEKA FOODSERVICE`.
- **Notice 26865565** (dispatched 2026-02-02, `DR Lebensmittel Kantinen LAFP NRW`). ORG-0006 reads `EDEKA Foodservice
  Stiftung & Co. KG`, BT-501 `DE815728669`, Edekastr. 1, 77656 Offenburg, `edeka-foodservice.de`. Contract CON-0004
  goes to `EDEKA Foodservice Stiftung & Co. KG, 77656 Offenburg`. This matches `top_newest` 2026-02-03.

Issue 311's batch verdict reached the same answer independently: "Leave DE815728669 attached to EDEKA; treat as a
single-company row". In that verdict, 4 of 6 mentions are the bare name, and Union SB-Großmarkt never appears alone
on this VAT. The change drops the Bietergemeinschaft tail and serves the VAT holder.

## For unit 2

- These nine add no wrongs. The `consortium_removed` release (8 rows here) and the `family` H ≤ 1 release (row 27)
  both name the identifier's holder in every row.
- Two rows leave `head_current` within six weeks of this read:
  - row 28 after 2026-11-09, which moves to `top_stale`;
  - row 27 after 2026-11-28, which ships.
  Both are recorded R here, before they age out.
