# 548 U3: v13 re-judge, rows 45-50 of `rejudge_rows.json` (judge 6, 2026-10-19)

All six rows are `consortium_removed` releases (rule v13's token-subset test), all R1, all single-head-procedure
(`head_procs` 1, except row 50 with 2). In each one the head is the oldest printed spelling (`head_newest` 2024-03-13 to
2025-03-26), and the new name's newest print is 2025-12-23 to 2026-10-07. I read every served name live on 2026-10-19
(`/v1/organizations/{id}`), and all six still serve the old head. The printed `mention_name` values come from
`/v1/tenders/{id}`, with the tenders found through `/v1/tenders?winner=<id>&sort=published_at` (`buyer=` for row 50,
plus `bidder=` for row 47).

**Tally: 6 R / 0 N / 0 W, 0 former-name reverts.** Every new name is the firm or body itself with its role or group
wrapper removed: "(Bietergemeinschaft mit ...)", "(mandataire) cotraitant ...", "Titulaire 1: ... / Titulaire 2 ...",
"ARGE ARCUS + DEGAT", "Coordonnateur du groupement de commande ...". Its identifier carries that firm's standalone
prints through 2025-2026. No new name loses a name token, a legal form or a diacritic from the firm name inside the old
head. The only other differences are casing (rows 49, 50). Those are vote-key-equal and are not a Brlic/Mithieux
downgrade, because the head they replace is not a correct name.

**Evidence:** 36 public GETs, all `rc=200`, no 408s, at least 2.2 s apart (11:40:50Z-11:43:38Z). They are logged in
`u3/api_requests.log`, and the bodies are in `u3/bodies/j6_*` (`j6_org_<org>`, `j6_tw_<org>` winner lists,
`j6_tb_<org>` buyer/bidder lists, `j6_td_<tender>`). I ran them from a private helper in my own session scratchpad
(`j6_get.sh`), not a shared path, so the judge-4 script-swap incident cannot apply here.

**Row 45 - org 22534691, VAT DE361508840: R.** "Troiber GmbH & Co. KG (Bietergemeinschaft mit Bauer GmbH & Co.KG,
Rödental; Nußbaumer GmbH & Co.KG, Kürnach; Omega Sorg GmbH, Rednitzhembach; Früchte Jork GmbH, Isny; Rittner Food
Service GmbH & Co.KG, Unterschleißheim)" becomes "Troiber GmbH & Co. KG".
- The head is one procedure: tender 901151 (00152919-2024, 2024-03-13, food for the Bavarian prisons), with that
  bidding-consortium string on all 18 lots.
- Six days later, tender 660090 (00163858-2024, 2024-03-19) already prints "Troiber GmbH & Co. KG" by itself.
- The winner list shows nine more tenders through 2026 (2024-09-16 to 2026-08-07). I read the two newest, 386855
  (00549509-2026, 2026-08-07, poultry) and 1017771 (00545369-2026, 2026-08-06, pork). Both print exactly "Troiber
  GmbH & Co. KG" as tenderer and winner on every lot. The top cluster holds 20 of 22 votes.

The new name is the lead firm, with its legal form intact, cut out of its Bietergemeinschaft. The five partners are
separately named legal entities, and none of them is promoted. This is the same legal entity under its current print.
Skeptic: could the VAT belong to the consortium rather than Troiber? It does not, because every standalone print under
this identifier from 2024-03-19 to 2026-08 is Troiber's alone.

**Row 46 - org 22564460, SIRET 45066958500025: R.** "CUADRA (mandataire) / cotraitant ACDe / cotraitant B3E /
cotraitant CDB ACOUSTIQUE / cotraitant PLANTAGO" becomes "CUADRA".
- The head is one print, tender 468510 (00203924-2024, 2024-04-08, a maîtrise d'œuvre framework).
- The winner list shows nine more tenders from 2024-06-10 to 2026-06-01, all MOE, design-build or feasibility work. I
  read three: 252041 (00340188-2024, 2024-06-10), 1130536 (00262313-2026, 2026-04-17) and the newest, 101480
  (00372757-2026, 2026-06-01). Each prints "CUADRA" alone.
- The top cluster holds 9 of 10 votes.

The identifier is the mandataire's SIRET: the head names CUADRA as mandataire, and the four cotraitants are other
firms. The new name is the mandataire with the cotraitant list cut off. Same legal entity, its current print.

**Row 47 - org 22590748, SIRET 91236214200024: R.** "Titulaire 1: SOLVHA (mandataire) / MEOTEC (co-traitant) /
Titulaire 2: KLB GROUP / Titulaire 3: VISIATIV" becomes "SOLVHA".
- The head is one print, tender 666256 (00256927-2024, 2024-04-30, procurement support for the Ville de Marseille).
  It is a multi-holder framework in which SOLVHA leads holder 1.
- Tender 488585 (00437936-2024, 2024-07-22) prints "SOLVHA" alone.
- Tender 119567 (00264641-2025, 2025-04-24, AMO marchés publics) prints "SOLVHA" alone on all 3 lots.
- The plan's `top_newest` is 2026-02-12. Neither the winner list nor the bidder list reaches a mention of that date
  (both return the same 3 tenders, `more: false`), so it sits in a non-bid party role that the public filters do not
  index. I infer it is a procurement service-provider party, which fits SOLVHA's line of work, but I did not read it.
  Either way, the plan's spellings put it in the 4-print cluster that reads exactly "SOLVHA".

KLB GROUP, VISIATIV and MEOTEC are other firms, and none of them is promoted. The new name is the holder-1 mandataire,
whose SIRET this is. Same legal entity, its current print.

**Row 48 - org 22648040, VAT DE138773422: R.** "ARGE ARCUS + DEGAT / ARCUS Planung + Beratung
Bauplanungsgesellschaft mbH" becomes "ARCUS Planung + Beratung Bauplanungsgesellschaft mbH".
- The head is one procedure, tender 581970 (00340509-2024, 2024-06-10, DLR site Cottbus infrastructure), 2 lots.
- The new name is printed verbatim on 309024 (00413845-2024, 2024-07-11), on 490635 (00582680-2024, 2024-09-27) and on
  the newest, 1031795 (00857901-2025, 2025-12-23), where it appears on 8 bids / 4 party rows.
- The plan's rival, "ARCUS Planung + Beratungs Bauplanungsgesellschaft" (2 votes, newest 2025-10-20), is tender
  965058 (00690081-2025). It is a buyer's misspelling that adds an "s" and drops "mbH". The new name is newer
  (2025-12-23), more frequent (6 of 10) and carries the correct GmbH-family form.

The head's own second line is the new name, so the ARGE label is the only thing removed. DEGAT is the partner and is
not promoted. Skeptic: is the VAT DEGAT's or the ARGE's? It is ARCUS's, because the identifier's four later
standalone prints all name ARCUS. Same legal entity.

**Row 49 - org 22715315, SIRET 48046024500024: R.** "COLAS DURAND ARCHITECTES (MANDATAIRE) / COTRAITANTS /
BSO/OPRYME/EICE/ACCOUSTIBEL" becomes "COLAS DURAND Architectes".
- The head is one print, tender 238068 (00558554-2024, 2024-09-17, Bâtiment ISPAIA Institut Boussingault).
- Tender 363333 (00217935-2026, 2026-03-30, competition) prints both "COLAS DURAND ARCHITECTES" and "COLAS DURAND
  Architectes" (two bids).
- The newest, 749903 (00540505-2026, 2026-08-05), prints exactly "COLAS DURAND Architectes".

The new name keeps every token of the mandataire firm ("COLAS DURAND ARCHITECTES") and drops the role word and the
four cotraitants (BSO, OPRYME, EICE, ACCOUSTIBEL), none of which is promoted. The casing is the newest print's own.
"COLAS" here is part of the architects' firm name. It is not the Colas construction group, and the identifier's prints
are all architecture MOE. Same legal entity.

**Row 50 - org 22838635, SIRET 21710230000019: R.** "Ville de Gueugnon / Coordonnateur du groupement de commande
Ville de Gueugnon / Communauté de Communes Entre Arroux, Loire et Somme" becomes "Ville de Gueugnon".
- This is a buyer. SIREN 217102300 is the commune of Gueugnon (INSEE 71230; the 21 prefix is a commune).
- The head (2 votes, a newline spelling and a space spelling) comes from the two joint fuel-card procurements the
  town coordinated for itself and the Communauté de Communes: tender 388523 (00667089-2024, 2024-11-04) and tender
  294733 (00194543-2025, 2025-03-26, the head's newest).
- Since then, the buyer list shows the town's own procedures: 289597 (2025-07-28), 477147 (2026-02-20), 181866
  (2026-06-03), 1014010 and 8830475. I read the two newest. Tender 1014010 (00517704-2026, 2026-07-27, médiathèque MOE)
  and tender 8830475 (00690403-2026, 2026-10-07, the town's insurance) both print "VILLE DE GUEUGNON" as
  Procedure-Buyer. The upper-case and mixed-case spellings hold 3 votes each, so the top cluster holds 6 of 8.

The new name is the commune itself, without its role as group coordinator. The Communauté de Communes is a separate
body and is not promoted. The only change from the newest print is casing. Same legal entity.
