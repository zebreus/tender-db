# 548 U3: v13 re-judge, rows 9-17 of `rejudge_rows.json` (judge 2, 2026-10-19)

All nine rows are `consortium_removed` releases (rule v13's token-subset test). Each is R1, its head printed in 2016-2017
(`head_newest`) and its new name printed through 2019-2026 (`top_newest`).

**Tally: 9 R / 0 N / 0 W, 0 former-name reverts.** No row reverts to an older name: in every row the head is the OLDEST
printed spelling and the new name's newest print is 2-9 years later.

**Evidence:** 40 public GETs, all `rc=200`, no 408s, at least 2.2 s apart. They are logged in `u3/api_requests.log`, and
the bodies are in `u3/bodies/` (`o_<org>`, `t_<org>`, `td_<tender>`, `n_<notice>`, `np_<prefix>`).

- Per row: the org GET and the newest 10 tenders by winner.
- 15 tender details, read for each newest mention's `parties[].mention_name` and for the head's 2016-2017 mention.
- 1 notice content.
- 6 `name_prefix` lookups.

In every row the org GET still serves the held head, with the plan's identifier and the plan's mention count.

**9 — 9134957 BE 0825811478 — R.** The name changes from `Everis Spain, S.L.U., succursale en Belgique (as consortium
leader)` to `Everis Spain SLU, Succursale en Belgique` (V6 T5 H1). Only the role annotation is dropped, so the entity
is the same.
- The newest dated mention on this identifier is tender 6233425's award, published 2019-04-05 (notice 20350556). It
  prints exactly `Everis Spain SLU, Succursale en Belgique`.
- The head's one print is from 2016-12-02.
- Caveat: Everis has since become NTT DATA. The 2022-07-07 CORDIS modification (notice 22494706) prints `NTT Data Spain
  S.L.`, and `np_nttdata_be` lists `NTT DATA Spain S.L.U., Succursale en Belgique (co-contractor)` (20470677, no
  identifier). No mention under 0825811478 prints NTT, though (2 clusters, no rival). Both names are therefore Everis
  names, and the new one is the later of the two.
- This is not a revert. It matches the U1c precedent `Unisys Belgium S.A. (as consortium member)` → `Unisys Belgium SA`
  (R).

**10 — 9467375 FR 43407420900017 — R.** `OPSIA Méditerranée (mandataire)` → `OPSIA Méditerranée` (V26 T15 H1). The
role annotation is dropped.
- 2026-01-26: tender 1098461, lots 1 and 3, print exactly `OPSIA Méditerranée`.
- The rival's newest print (`rival_newest` 2026-06-18, tender 344465) is `OPSIA MEDITERRANEE SELACA`: the same firm with
  its professional-company form.
- `np_opsia` shows the SIREN 434074209 on `Opsia Méditerranée` (10025259) and on `opsia mediterrannee selafa`. So the
  identifier belongs to this firm, not to its co-member `OPSIA (cotraitant)` (SIRET 38377082300011).
- The head's one print is from 2017-03-11.

**11 — 9553896 FR 41317960700226 — R.** `Groupement TP Spada NGE génie civil` → `TP Spada` (V18 T13 H1). The firm is
taken out of its consortium wrapper.
- The newest read mention, tender 1100120 on 2026-02-09, prints `TP SPADA`. `top_newest` is 2026-09-10, and the
  spellings include `Entreprise TP SPADA SAS`.
- The identifier is TP Spada's, not NGE's: `np_tpspada` shows `tp spada sas` on 4131796070022 (SIREN 413179607
  truncated), and all 13 top votes print TP Spada.
- The head's one print is from 2017-03-31. NGE Génie Civil is the other member of that 2017 groupement.

**12 — 9699686 ES B80595747 — R.** `UTE Gesein, S.L.- Altran Innovación S.L.- Tecnocom España Solutions, S.L.U.` →
`Gesein, S.L.` (V31 T24 H2). The firm is taken out of its UTE.
- 2026-09-23: tender 8580506, notice 46695175, prints exactly `Gesein, S.L.`.
- The NIF has a `B` prefix, which is a sociedad limitada's NIF and not a UTE's (a UTE's starts with `U`). The 24 top
  votes print Gesein.
- The rival `UTE Altia — Gesein — Getronics` (newest print 2020-08-24) is another UTE wrapper of the same firm.
- The head's last print is from 2017-05-03.

**13 — 9741944 FR 40134697800018 — R.** `SAS MAG PRIM — mandataire du groupement solidaire MAG PRIM — GROUP MAG` →
`SAS MAG PRIM` (V15 T13 H1). The firm is taken out of the groupement sentence. The head's legal form is kept, so the
family is sas → sas.
- 2026-03-31: tender 641175, notice 25671907, lots 3 and 4, prints `MAG PRIM`.
- The new name is the most-printed spelling (6 votes). All the spellings are ALL-CAPS, so the casing loses nothing.
- The head's one print is from 2017-05-19. The rival's newest print is 2022-12-23.

**14 — 9795935 FR 44086377700023 — R.** `Groupement Agri Compost 70` → `AGRI Compost 70` (V7 T5 H1).
- "Groupement" is a consortium wrapper here, not the company's name. Unlike U1's CPR wrong, the head's only mention is
  the 2017-05-27 award of tender 5857778, lot 2, to a three-firm groupement: `Groupement Agri Compost 70` with `SARl Agri
  Compost Conflans` and `SARL Agri Trans 70`.
- Since then every print on this SIRET drops "Groupement". The newest is 2026-03-17: tender 845588, lots 1 and 5, print
  `AGRI COMPOST 70`.
- `np_agricompost` shows a separate `Agri Compost 70` (13431855) as well.
- The served casing `AGRI Compost 70` (1 vote) is odd. `Agri Compost 70` also has 1 vote and loses the byte-order
  tiebreak to it. The words are the entity's, though, so this is a casing quirk and not a worse name.

**15 — 9930423 FR 86320018400092 — R.** `Groupement Dumez Auvergne` → `DUMEZ AUVERGNE` (V8 T7 H1).
- The head's only mention is the 2017-06-24 award of a design-and-build contract (tender 5417144) to a team:
  `Groupement Dumez Auvergne` with Atelier 127, Eodd, NOX ingénierie, Génie Acoustique and Stéphane Pichon. It is a
  consortium wrapper.
- Every later print drops "Groupement". The newest is 2026-05-18: tender 1108906 prints `DUMEZ AUVERGNE`.
- `np_dumezauv` shows `DUMEZ AUVERGNE` on 863200184, this SIRET's SIREN.
- The served name is ALL-CAPS because no mixed-case spelling of the firm exists besides the head; the spellings are
  `DUMEZ AUVERGNE` ×5 and `dumez auvergne` ×2. Only casing changes and no word of the firm's name is lost, so the U2
  Mithieux precedent (it lost `TP`) does not apply.
- There is no rival.

**16 — 9945607 FR 54202182900107 — R.** `Agence Safège Bureau d'etudes VRD (Mandataire)` → `Safège` (V44 T26 H1,
second 3). The agency line and the role annotation are dropped, leaving the company.
- 2026-05-26: tender 1026993, notice 25795216, prints `SAFEGE`.
- `rival_newest` 2026-07-07: tender 892131, notice 45865869, prints `SAFEGE - SUEZ CONSULTING`. That is the same
  company (SAFEGE SAS, which trades as SUEZ Consulting), so no other entity is printed after the top.
- The spellings are `SAFEGE` ×9, `Safege` ×5, `safege` ×5, `Safege SAS` ×3 and `SAFEGE SAS` ×2.
- The accented `Safège` comes from the twin rule. Its accent is the head's own spelling, so the name is no downgrade.
- The head's one print is from 2017-06-28.

**17 — 10387870 FR 48953305900122 — R.** `Groupement alliance environnement exploitation / SARL Patrick Tubert` →
`Alliance environnement exploitation` (V22 T18 H1). The firm is taken out of its two-firm groupement, and the dropped
`SARL` is the other member's form.
- 2026-10-12: tender 1035771, notice 47386177, lot 2, prints `ALLIANCE ENVIRONNEMENT EXPLOITATION`. Its spellings
  include `SAS ALLIANCE ENVIRONNEMENT EXPLOITATION` ×3.
- `np_allianceenv` shows `ALLIANCE ENVIRONNEMENT EXPLOITATION` on 4895330590122 (this SIRET truncated).
- The rival's newest print (2026-03-13, tender 639545) is `GROUPEMENT ALLIANCE ENVIRONNEMENT / HYDRIAD EAU &
  ENVIRONNEMENT`: another groupement wrapper, possibly of the sister firm `Alliance environnement` (1964213). It is not
  a newer name for this firm.
- The head's one print is from 2017-09-27.
