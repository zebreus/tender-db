# 574 unit 1: read of rows 72..80 of rows.json (plan 2653, rule v13)

These nine rows are `held` entries on plan 2653, and **all nine are held `head_current`**. v13 released their v12
bind-guard hold:
- 8 rows were held `consortium_removed`: 72 to 79.
- 1 row was held `gb_family`: 80 (plc -> ltd, H = 1).

I judged each row as if `old_name -> new_name` shipped today (2026-10-19). The evidence is the entry's dates
(`head_newest`, `top_newest`, `rival_newest`), its spellings and its V/T/H (voting/top/head).

The labels follow `548-triage/u1/judgements.json`:
- **R**: right.
- **N**: neutral (the same entity, and neither name is better).
- **W**: wrong (another entity, a member for the group, or a clearly worse name).
- **REVERT**: an older name that is no longer used.

**Tally: 9 R, 0 N, 0 W, 0 REVERT.**
- Five supplier rows (72, 73, 74, 75, 78) drop a consortium wrapper: a groupement d'entreprises, an ARGE or a
  Bietergemeinschaft. Each serves the lead firm alone. That firm is the one the identifier belongs to, and later bare
  prints carry its name on the same number.
- Three buyer rows (76, 77, 79) drop a buyers' purchasing-group sentence: a "groupement de commandes" or a purchasing
  consortium. Each serves the coordinating body that holds the identifier: La Poste, Grand Paris Aménagement and
  Coastline Housing. A groupement de commandes has no legal personality, so the coordinator is the entity.
- Row 80 drops a company-number tail and a parent-company annotation ("- a Personal Group Holdings Plc Company") from
  the PPON holder's own name.
- In every row the top was printed after the head (`top_newest > head_newest`). Two rows have a rival, and in both the
  rival names the same entity:
  - Row 73's rival is `Entreprise LECONTE ELAGAGE`.
  - Row 76's rivals are other La Poste groupement-de-commandes sentences.
- Dated reads back three rows:
  - Row 75: I opened the head notice. Its tenderer's contact details are Terhalle's, and so is the VAT number.
  - Row 78: the same SIRET is a merged identifier on the big Egis Rail org (9255307).
  - Row 80: I opened the head notice, a 2026-06-11 ESPO framework award.
- Confidence is lowest on row 74, which is still R. The served spelling keeps an odd `S2 -architekten` spacing (4
  votes, against 2 for `S2 - architekten`). Even so, it is the partnership's own name, and the name it replaces is a
  two-firm ARGE sentence.
- **None of the nine ships before 2028-06-05.** Row 76 lapses first. Row 73 has three clusters and a top last printed
  2026-03-20, so it goes into `top_stale` long before its head lapses. See "When each would ship".

## Method and limits

- I made 12 public GETs, all `rc=200`. They are lines 73 to 81 and 91 to 93 of `574-triage/api_requests.log`. The
  bodies are in `574-triage/bodies/`:
  - `GET /v1/organizations/{id}` for each of the nine orgs (13:12:46Z to 13:13:10Z, 3 s apart; saved as
    `org_<id>.json`). All nine still serve `old_name`. For each one, the `identifier` and the `mentions` count equal
    the plan entry's `identifier` and `n`, so the listing is current. None is provisional, and none has merged
    identifiers. Kinds: 7 `national`, 2 `vat`.
  - `GET /v1/organizations/9255307` (13:16:42Z) for row 78. This is the Egis Rail org that 548 U3's saved tender
    421896 binds `EGIS RAIL` to.
  - `GET /v1/notices/26893135/content` (13:16:47Z) for row 75. It is one of the two head notices that issue 311's
    batch verdict on 23600961 names (`issues/311-batch-verdicts.json`).
  - `GET /v1/notices/46828995/content` (13:16:52Z) for row 80. It is the head spelling's only notice, taken from
    545 U2a's saved query `545-triage/u2a/s1.out.json`.
- I checked every identifier locally:
  - SIRETs 65203895100024, 91215307900039, 64203694100036 and 96850255900105, their SIRENs, and SIREN 356000000 all
    pass Luhn;
  - DE VATs 293551738 and 123772598 pass ISO 7064 MOD 11,10;
  - `GBCOH03284666` is Companies House number 03284666, and `GBPPONPBPJ2845LVWT` is the PPON PBPJ-2845-LVWT. Neither
    can be checked locally.
- I made no /v1/sql request, no POST and no ssh, used no git, and edited nothing outside this directory.
- **Limit (same as earlier reads):** none of the three allowed endpoints lists an org's tenders. I searched
  `.scratch/` for the nine org ids, identifiers and names, and found tender or notice ids only for rows 75, 78 and 80.
  The other verdicts rest on:
  - the plan's spellings and dates;
  - the live org record;
  - the identifier check;
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
| 76 | 30934047 | **2028-06-05** | 7 | 2027-10-15 | ships if `La Poste` is printed on this SIREN again by then (21 prints so far, newest 2026-10-15); otherwise goes to `top_stale` |
| 77 | 30937468 | 2028-06-12 | 2 | never (no rival) | ships |
| 78 | 30939728 | 2028-06-15 | 2 | never | ships |
| 72 | 23458031 | 2028-12-29 | 2 | never | ships |
| 73 | 23525636 | 2029-02-17 | 3 | 2027-03-20 | goes to `top_stale` unless `LECONTE ELAGAGE` is printed on this SIRET again in the year before the lapse |
| 75 | 23600961 | 2029-03-08 | 2 | never | ships |
| 74 | 23587877 | 2029-03-23 | 2 | never | ships |
| 80 | 31576946 | 2029-06-10 | 2 | never | ships |
| 79 | 31534713 | 2029-09-27 | 2 | never | ships. The head was last printed 2026-09-28 and is still in use, so each new Cirrus print pushes the lapse later |

## Verdicts

| # | org_id | old -> new | V/T/H | head_newest | top_newest | rival_newest | v12 hold | verdict |
|---|---|---|---|---|---|---|---|---|
| 72 | 23458031 | `Groupement d'entreprises SEKATOL/MENUISERIE STANOISE` -> `SEKATOL` | 4/3/1 | 2025-12-30 | 2026-09-17 | — | consortium_removed | **R** |
| 73 | 23525636 | `Groupement dont LECONTE ELAGAGE est mandataire` -> `LECONTE ELAGAGE` | 5/3/1 | 2026-02-18 | 2026-03-20 | 2026-02-18 | consortium_removed | **R** (high) |
| 74 | 23587877 | `ARGE - S2 -architekten und ingenieure PartGmbB / KLÄHNE BUNG Beratende Ingenieure im Bauwesen GmbH` -> `S2 -architekten und ingenieure PartGmbB` | 8/6/2 | 2026-03-24 | 2026-04-01 | — | consortium_removed | **R** (medium-high) |
| 75 | 23600961 | `Bietergemeinschaft Terhalle Holzbau GmbH, Ahaus, und Beton- und Monierbau GmbH, Nordhorn, vertreten durch die Terhalle Holzbau GmbH` -> `Terhalle Holzbau GmbH` | 6/4/2 | 2026-03-09 | 2026-07-10 | — | consortium_removed | **R** (high; dated read) |
| 76 | 30934047 | `Groupement de commandes composé de La Poste et sa filiale, La Poste Immobilier , représenté par La Poste en qualité de coordonnateur.` -> `La Poste` | 27/21/1 | 2025-06-06 | 2026-10-15 | 2026-09-30 | consortium_removed | **R** (high) |
| 77 | 30937468 | `Grand Paris Aménagement au nom et pour le compte du groupement de commandes GPA/ORSA/SPLAIN` -> `Grand Paris Aménagement` | 7/6/1 | 2025-06-13 | 2026-10-01 | — | consortium_removed | **R** (high) |
| 78 | 30939728 | `GROUPEMENT EGIS RAIL (mandataire) - IN SITU (cotraitant) - STOA (co-traitant) - INGEROP CONSEIL ET INGENIERIE (co-traitant)` -> `EGIS RAIL` | 4/3/1 | 2025-06-16 | 2025-06-27 | — | consortium_removed | **R** (high) |
| 79 | 31534713 | `Cirrus Consortium c/o Coastline Housing Limited` -> `COASTLINE HOUSING LIMITED` | 26/24/2 | 2026-09-28 | 2026-10-13 | — | consortium_removed | **R** (high) |
| 80 | 31576946 | `INNECTO PEOPLE CONSULTING LIMITED (04741738) - a Personal Group Holdings Plc Company` -> `INNECTO PEOPLE CONSULTING LIMITED` | 4/3/1 | 2026-06-11 | 2026-09-04 | — | gb_family | **R** (high; dated read) |

## Per row

**Row 72: 23458031, SIRET 65203895100024 (SIREN 652038951). R.**

There are two clusters:
- the top, `SEKATOL`: 3 of 4 votes, newest 2026-09-17;
- the head, `Groupement d'entreprises SEKATOL/MENUISERIE STANOISE`: 1 vote, 2025-12-30.

A groupement momentané d'entreprises has no SIREN of its own. So this SIRET belongs to one of the two members, and all
3 bare prints on it, made after the head, name Sekatol. The SIREN's `65…` prefix also points to a long-registered
company, not to a vehicle set up for one bid. Menuiserie Stanoise is the partner in that one groupement, and no print
names it alone. The change drops the wrapper and the partner, and the new name passes the token-subset test. I could
open no notice for this org.

**Row 73: 23525636, SIRET 91215307900039 (SIREN 912153079). R, high confidence.**

There are three clusters:
- the top, `LECONTE ELAGAGE`: 3 of 5 votes, newest 2026-03-20;
- the rival, `Entreprise LECONTE ELAGAGE`: 1 vote, 2026-02-18;
- the head, `Groupement dont LECONTE ELAGAGE est mandataire`: 1 vote, 2026-02-18.

The head names no other member. It only says that Leconte Élagage leads a groupement. The rival is the same firm
with an "Entreprise" prefix, and it was printed on the same day as the head. Every spelling on this SIRET names
Leconte Élagage, so the change serves that firm's own bare name.

This is the one row of the nine that will probably never ship as it stands. Its top goes stale on 2027-03-20, almost
two years before the head lapses. It ships only if the bare name is printed again on this SIRET.

**Row 74: 23587877, USt-IdNr DE293551738. R, medium-high confidence.**

There are two clusters:
- the top, which joins `S2 -architekten und ingenieure PartGmbB` (4 votes) and `S2 - architekten und ingenieure
  PartGmbB` (2 votes) under the vote key: 6 of 8 votes, newest 2026-04-01;
- the head, the ARGE sentence: 2 votes from 1 procedure (`head_procs` 1), newest 2026-03-24.

Klähne Bung Beratende Ingenieure im Bauwesen GmbH is the structural-engineering partner in that one ARGE, and no print
names it alone on this VAT. A German ARGE can hold its own USt-IdNr, but two things point to the number being S2's:
- six bare S2 prints carry it;
- to my knowledge the DE29… range was issued around the mid-2010s, years before an ARGE that bid in 2026.

The family guard did not fire here (`family_new` is null, because PartGmbB is not in the family list). The
consortium guard did, and dropping the ARGE wrapper and the partner is the fix.

Doubt: the spelling. The served spelling has the unusual `S2 -architekten` spacing. It is the majority spelling (4
against 2), so this is a matter of punctuation, not of which entity is named. I could not tell how many notices the 6
top votes come from.

**Row 75: 23600961, USt-IdNr DE123772598. R, high confidence (dated read).**

There are two clusters:
- the top, `Terhalle Holzbau GmbH`: 4 of 6 votes, newest 2026-07-10;
- the head, the Bietergemeinschaft sentence: 2 votes from 1 procedure, newest 2026-03-09. Issue 311 names the two
  head notices, 25641642 and 26893135.

**Live notice 26893135** (eForms `can-standard`, dispatched 2026-03-05) is from Klingenstadt Solingen, SD 23
Gebäudemanagement. The procedure is "Vergabeverfahren Generalunternehmer Neubau Grundschule Augustastraße, Solingen",
and the contract was concluded 2026-03-05. Its tenderer ORG-0003 is the Bietergemeinschaft, and every contact detail
on it is Terhalle's:
- BT-500: the Bietergemeinschaft sentence;
- BT-501: `Ust.-Id Nr.: DE 123 772 598`;
- website `https://www.terhalle.de`;
- contact `ppa. Marcus Erbacher`, `erbacher@terhalle.de`;
- address Solmsstraße 46, 48683 Ahaus.

The tender's own label is `Angebot Terhalle`. The number is from the first DE12… range, issued from 1993; the buyer
city's own VAT on the same notice is DE120871761. A GbR formed to bid in 2025 would not hold a 1990s number. So the
VAT is Terhalle Holzbau's own, and the 4 later bare prints of `Terhalle Holzbau GmbH` on it agree. Beton- und
Monierbau GmbH (Nordhorn) is the other member, and no print names it alone.

Side note, outside the verdict: issue 311's batch verdict on this org (cohort `biege-batch-2026-08-30`) read it as
`consortium-vehicle-sound` / `plausible-but-unverifiable`: "keep the identifier as the vehicle's plausible own VAT …
worth checking if further evidence surfaces". At that time it had 2 mentions. The 4 bare Terhalle prints since then
and the contact block above are that further evidence: the VAT is the representing partner's own number. That is for
whoever owns org identity. It does not affect the R.

**Row 76: 30934047, SIREN 356000000 (La Poste SA). R, high confidence.**

The org has seven clusters:
- the top, `La Poste`: 21 of 27 votes, newest 2026-10-15;
- the head, `Groupement de commandes composé de La Poste et sa filiale, La Poste Immobilier , représenté par La Poste
  en qualité de coordonnateur.`: 1 vote, 2025-06-06;
- five 1-vote rival clusters, newest 2026-09-30. The plan's spelling list stops at five entries, so it shows three of
  them. All three are groupement-de-commandes sentences coordinated by La Poste: two other La Poste Immobilier
  wordings, one of them with mojibake (`compos?`), and one with La Banque Postale, Chronopost and Alturing.

SIREN 356000000 is La Poste's own. A groupement de commandes is a buyers' purchasing agreement with no legal
personality, and the coordinator signs for it. So the entity behind this SIREN is La Poste, and 21 bare prints say so.
The change serves the holder's name in place of one group-purchasing sentence. The fresher rivals are no better
current name, because each is another such sentence led by La Poste.

**Row 77: 30937468, SIRET 64203694100036 (SIREN 642036941). R, high confidence.**

There are two clusters:
- the top, which joins `Grand Paris Aménagement` (4 votes) and `GRAND PARIS AMENAGEMENT` (2 votes): 6 of 7 votes,
  newest 2026-10-01;
- the head, `Grand Paris Aménagement au nom et pour le compte du groupement de commandes GPA/ORSA/SPLAIN`: 1 vote,
  2025-06-13.

Grand Paris Aménagement is the State planning EPIC, formerly AFTRP. In the head it acts "au nom et pour le compte" of
a groupement de commandes with EPA ORSA and SPLA-IN. The served name is the coordinator's own name, in the accented
mixed-case spelling, and the head is already that name with a mandate clause appended.

**Row 78: 30939728, SIRET 96850255900105 (SIREN 968502559). R, high confidence.**

There are two clusters:
- the top, which joins `EGIS RAIL` (2 votes) and `egis rail` (1 vote): 3 of 4 votes, newest 2025-06-27;
- the head, a four-member groupement sentence: 1 vote, 2025-06-16. The sentence itself marks EGIS RAIL as mandataire,
  with In Situ, Stoa and Ingérop Conseil et Ingénierie as co-traitants.

**Live org 9255307** (149 mentions) has identifier `968502559`, and it carries **this SIRET, 96850255900105**, as a
merged identifier, together with VAT `FR50968502559`. It serves `„Ежис Рай“ АД, рег. № Siret 968 502 559 00105`, a
Bulgarian transliteration. 548 U3's saved tender 421896 (notice 24882932) binds the mention `EGIS RAIL` to that org on
lots 3 and 6. So this SIRET is Egis Rail's own. The change drops the three co-contractors, and the new name passes the
token-subset test.

All four votes were printed within 11 days in June 2025, so they may come from one procedure. That is no risk here:
every spelling names Egis Rail on Egis Rail's own number.

Side note, outside the verdict: two orgs carry this one SIRET, 30939728 (this row) and 9255307. After this row ships,
the two will serve `EGIS RAIL` and the Bulgarian transliteration for one company. 9255307's served name is a separate
defect. Both are for whoever owns org identity.

**Row 79: 31534713, Companies House 03284666. R, high confidence.**

There are two clusters:
- the top, `COASTLINE HOUSING LIMITED`: 24 of 26 votes, newest 2026-10-13;
- the head, `Cirrus Consortium c/o Coastline Housing Limited`: 2 votes from 1 procedure, newest 2026-09-28.

The all-capitals `COASTLINE HOUSING LIMITED` printed 24 times on this number reads as the register name. To my
knowledge, Cirrus is a purchasing consortium of South West housing associations that Coastline Housing (Redruth,
Cornwall) hosts. "c/o Coastline Housing Limited" in the head says the same: Coastline is the legal body that contracts
for the consortium, and the company number is Coastline's. So the change serves the identifier holder's name in
place of the consortium's trading label. It is not a member named for the group: the group has no company number of
its own here, and the org's own prints are 24 to 2 for Coastline.

The head is still current (printed three weeks ago), so this row is the furthest of the nine from shipping.

Side note, outside the verdict: 545 U2a's saved query shows a second Coastline org, 31561711 `Coastline Housing Ltd.`,
under PPON GBPPONPHYP7343GBBP. That is a PPON/company-number pairing question for 469/545, not for this verdict.

**Row 80: 31576946, PPON PBPJ-2845-LVWT. R, high confidence (dated read).**

There are two clusters:
- the top, which joins `INNECTO PEOPLE CONSULTING LIMITED` (2 votes) and `INNECTO PEOPLE CONSULTING LIMITED
  (04741738)` (1 vote) under the vote key: 3 of 4 votes, newest 2026-09-04;
- the head: 1 vote, 2026-06-11.

**Live notice 46828995** (UK6 contract award, published 2026-06-11) is ESPO's framework "3S_26 Strategic HR and
Recruitment Services", the award for lots 5 to 8. Its ORG-GB-PPON-PBPJ-2845-LVWT has BT-500 equal to the head
spelling and BT-501 `GB-PPON-PBPJ-2845-LVWT`, and it tenders on lot 7. The head is the supplier's self-declared name.
It carries its company number in brackets and a parent-company tagline: to my knowledge Personal Group Holdings plc
bought Innecto in 2022. The bare prints come from two later notices, 46844428 and 47159048. The other framework
suppliers in 545's query (Instructure, Oxehealth, 4C Europe) share them.

The `gb_family` guard fired only because `Plc` appears in the tagline. The new name is the company's own name with the
number and the tagline dropped. `release_withheld: junk_only` is the rule v9 withholding of the `digits` release for
the head's company number: the elected name also drops more than one other word. It concerns the `recent` window,
not the name served.
