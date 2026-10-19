# 548 U3: v13 re-judge, rows 27-35 of `rejudge_rows.json` (judge 4, 2026-10-19)

All nine rows are `consortium_removed` releases (rule v13's token-subset test), all R1. In each one the head is the
oldest printed spelling (`head_newest` 2018-03-09 to 2020-02-10), and the new name's newest print is 2025-11-25 to
2026-09-30. I read every served name live on 2026-10-19 (`/v1/organizations/{id}`), and all nine still serve the old
head. The printed `mention_name` values come from `/v1/tenders/{id}`, with the tenders found through
`/v1/tenders?winner=<id>&sort=published_at` (and `bidder=` for row 35).

**Tally: 9 R / 0 N / 0 W, 0 former-name reverts.** Every new name is the firm itself with its role or consortium
wrapper removed ("mandataire", "Rang 2 —", "ARGE ... — <partner>", "Agrupamento ... liderado por", "em consórcio").
Its identifier carries that firm's standalone prints through 2025-2026. No new name loses a name token or a
diacritic from the firm name inside the old head. The only changes are a dropped legal form (SCOP/SARL/SAS), a dropped
parenthetical (EVD, groupe NGE), casing, and one hyphen. Those are vote-key-equal and are not a Brlic/Mithieux
downgrade, because the head they replace is not a correct name. The judge-1 precedent for this is row 5,
Accenture sas -> Accenture = R.

**Evidence:** 63 public GETs, all `rc=200`, no 408s, at least 2.2 s apart. They are logged in
`u3/api_requests.log`, and the bodies are in `u3/bodies/j4_*` (`j4_org_<org>`, `j4_tw_<org>` winner lists,
`j4_tb_` bidder list, `j4_td_<tender>`, `j4_np_` name-prefix lookups).

**Log incident (11:36:44-11:36:56Z).** Another judge wrote its own `get.sh` to the same shared scratchpad path in
the middle of my loop, and bash ran the swapped script. As a result:
- The line `GET /v1/tenders/924169 rc=000` is a garbled-script artifact. The body `j4_td_924169.json` arrived intact
  (tender 924169, 00767907-2024).
- The three lines `GET j4_td_832931.json`, `GET j4_td_76308.json` and `GET j4_td_135771.json` (`rc=000`) were
  malformed URLs that the egress proxy refused. They never reached the API.

I re-ran those three as proper GETs (`rc=200`) from a private helper.

**Row 27 - org 11226363, SIRET 34094705000050: R.** "Scop Euro Flandres TP (mandataire du groupement)" becomes
"EURO FLANDRES TP".
- The head is one print, tender 6009728 (104770-2018, 2018-03-09).
- "SCOP Euro Flandres TP" appears once, on tender 6959783 (603576-2020, 2020-12-15). That is the rival.
- The newest tender, 7973586 (00626773-2026, 2026-09-11), prints exactly "EURO FLANDRES TP" as tenderer and winner.
  The top cluster holds 4 of 6 votes.

The new name drops the role phrase and the SCOP status word (a cooperative form, a form token like sas/SARL), and keeps
every name token. Same legal entity, its current print.

**Row 28 - org 12055155, NIF 505760215: R.** "Agrupamento “Consórcio Teixeira Couto” — Liderado por Iluminações —
Teixeira Couto, Lda." becomes "Iluminações Teixeira Couto, Lda.".
- The head (2 votes) is newest 2018-08-11. The top cluster holds 11 of 14 votes.
- Tender 7930105 (00779095-2025, 2025-11-24) prints the new name verbatim as tenderer and winner.
- The newest tender, 7943384 (00597927-2026, 2026-08-31), prints "ILUMINAÇÕES - TEIXEIRA COUTO, LDA": the same
  words with a hyphen.
- The plan's rival is dated 2024-10-25. Tender 437509 (00651545-2024) is the winner tender of that date and prints
  "Iluminações Teixeira do Couto Unipessoal Lda", a buyer's variant that is not elected.

The head names the consortium led by this Lda. A Portuguese NIF belongs to the company, not to the agrupamento, so
the new name is the leader firm out of its wrapper. The hyphenated form is printed 5 times against 3 for the elected
form, but the missing hyphen is cosmetic: no token or diacritic is lost, like the dot in judge 1's row 7. R.

**Row 29 - org 12622564, SIREN 413824319: R.** "Avenir Deconstruction (mandataire)" becomes
"AVENIR DECONSTRUCTION".
- The head is one print, tender 6253229 (529479-2018, 2018-12-01, demolition of the Albret/Alsace towers).
- The plan's other 3 mentions are "AVENIR DECONSTRUCTION" (winner tenders 2025-05-23 to 2026-09-23). The newest,
  52154 (00654350-2026, 2026-09-23), prints it as tenderer and winner. There is no rival.

The all-caps form is the firm's only standalone print. This is the U1c "ARENA SI (mandataire) -> Arena SI" pattern.
Same firm, current.

**Row 30 - org 14043573, SIRET 32161684900022: R.** "SARL Bouchet Francis et Fils (mandataire)" becomes
"Bouchet Francis et Fils".
- The head is one print, tender 6435272 (451684-2019, 2019-09-26).
- Tender 6686680 (232697-2020, 2020-05-20) prints "Bouchet Francis et Fils".
- The newest tender, 713416 (00805669-2025, 2025-12-04), prints "SARL BOUCHET FRANCIS ET FILS" as tenderer and
  winner. There is no rival.

The skeptic's point is that the newest print carries "SARL" and the new name does not. That is only a dropped form
token, which the vote key treats as equal (judge 1's Accenture row). Every name word survives, so this is no
Brlic-type loss. The firm is the same SARL under its own SIRET. R.

**Row 31 - org 14103011, AT 9110016732015: R.** "ARGE Feuerstein Josef GmbH & Co KG — Johann Huter & Söhne" becomes
"Josef Feuerstein GmbH & Co KG".
- The head is one print, tender 8427868 (512352-2019, 2019-10-30), from a joint venture (ARGE) with Johann Huter &
  Söhne.
- The plan's other 6 of 7 mentions are "Josef Feuerstein GmbH & Co KG" alone. The winner tenders run from 2019-10-29
  to 2025-11-25, and the newest, 263740 (00781898-2025, 2025-11-25), prints that name as tenderer and winner.

An ARGE is a project joint venture with no register identity, so this identifier is Feuerstein's. This is the U1c
precedent 18124034 (ARGE of 3 -> S Hoch 2 ZT GmbH) = R: the firm out of its consortium, not a member of a group.

**Row 32 - org 14394565, SIRET 44775408600075: R.** "SAS Guintoli (groupe NGE), mandataire" becomes
"Guintoli SAS".
- The head is one print from 2019-11-25.
- Tender 6959749 (291032-2021, 2021-06-11) prints exactly "Guintoli SAS".
- The newest tender, 326243 (00509562-2026, 2026-07-23), prints "GUINTOLI SAS" alongside "NGE FONDATION", the
  parent group's other companies listed as separate tenderers.
- Tenders 849994 (2026-01-19) and 337726 (2025-11-13) print "GUINTOLI".

"(groupe NGE)" names the parent group, so the new name is the subsidiary firm the SIRET belongs to, not the group.
This is the U1c precedent 11219959 (Guintoli SAS (mandataire du groupement) -> Guintoli) = R.

**Row 33 - org 14395645, SIRET 38468875000023: R.** "SAS Espaces verts du dauphine (EVD), mandataire" becomes
"ESPACES VERTS DU DAUPHINE".
- The head is one print from 2019-11-25.
- Tenders 802519 (00668542-2026, 2026-09-29) and 294885 (00486064-2026, 2026-07-14) print exactly the new name.
- The newest tender, 8845586 (00711674-2026, 2026-10-15), prints "SAS ESPACES VERTS DU DAUPHINE".
- The rival dated 2024-12-06 is tender 916071 (00747811-2024), which prints "ESPACES VERTS DU DAUPHINE
  (co-traitant n° 4)", a co-contractor annotation on the same firm.

The dropped "(EVD)" is a parenthetical acronym that no 2024-2026 print carries, and the dropped "SAS" is a form. The
head also lacks the accent on "Dauphiné", so no diacritic is lost. Same firm, current.

**Row 34 - org 14873976, NIF 513600728: R.** "WWS/Informantem em consórcio — WWS, Consultoria Informática, Lda."
becomes "WWS - Consultoria Informática, Lda.".
- The head is one print from 2020-02-04. The top cluster holds 20 of 25 votes.
- The newest tender, 7940004 (00594783-2026, 2026-08-28), prints the new name verbatim as tenderer and winner.
- Tender 910469 (2026-08-18) prints "WWS - CONSULTORIA INFORMÁTICA, LDA".
- The plan's rival is dated 2025-10-01. The two winner tenders of that date, 7930596 (00639675-2025) and 7930472
  (00641619-2025), print "WWS - Consultoria Informática, Ldª" and "...LDA,": the same firm with a
  form-abbreviation variant.

The head is the WWS/Informantem consortium. The NIF is WWS's, and the new name is WWS alone. R.

**Row 35 - org 14892786, SIRET 47976680000060: R (one caveat, not a revert).** "Rang 2 — Capgemini Consulting
(mandataire)" becomes "Capgemini Consulting".
- The head is one print dated 2020-02-10. Tender 6533931 (065241-2020) is the winner tender of that date; I did not
  fetch it. The top cluster holds 15 of 26 votes.
- The winner filter returns 16 tenders. I read the other 15, from 2020-06-09 through 2026-09-30, and every one prints
  this firm as Capgemini Consulting: "capgemini consulting", "SAS Capgemini Consulting", "Capgemini Consulting
  (Cotraitant)", "Capgemini Consulting SAS", "CAPGEMINI Consulting".
- Two 2024 tenders also print a bare "CAPGEMINI" next to it: 832931 (2024-12-16) and 135771 (2024-02-19).
- The newest tenders are:
  - 682148 (00673914-2026, 2026-09-30, interARS): "CAPGEMINI CONSULTING", tenderer and winner with KPMG.
  - 430805 (2026-02-18, EDF conseil): "CAPGEMINI CONSULTING".
  - 1022439 (2026-02-10): "Capgemini Consulting".
  - 852565 (2026-02-09): "CAPGEMINI CONSULTING".
- `name_prefix=Capgemini Consulting` shows the same-SIREN sibling 9068707 (SIRET 47976680000037), which is served
  "Capgemini Consulting". `name_prefix=Capgemini Invent` returns only an identifier-less 2-mention org.

The caveat is the 2018 Capgemini Invent rebrand. This org has 4 "CAP(GEMINI) INVENT" prints, and the plan's rival
cluster is dated 2026-10-16. None of these sits on a tender that `winner=` or `bidder=` reaches:
`winner=...&published_after=2026-10-01` is empty, and the bidder list's newest tender is 2026-09-30. So I could not
date them through the public API.

That does not change the verdict. The old head already serves the words "Capgemini Consulting", so this change only
removes "Rang 2 —" and "(mandataire)" and cannot move the served name to an older name. Buyers print "Capgemini
Consulting" through 2026-09-30, so it is a current print. Whether Invent should eventually win is a head-versus-rival
question for a later run, not a wrong in this change.
