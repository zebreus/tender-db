## Unit 1: decision (2026-10-12)

Suggested status line: `Status: ready-for-agent — unit 1 DECIDED 2026-10-12 (../520-award-summaries/unit1-decision.md). NEXT: U2 build (D: seven narrowings of names_an_award, partyname only), U3 drain (refold-void-names, no job change); then U4 (A1 winner extraction: partyname + fold + walk) and U5 drain. (C) filed as <N> (text Lot-n parser); the eForms "(niet gegunde partij)" winner role filed as <N+1>. B kept per 511 §5.`

**In one paragraph.** The 376 orgs that the void walk exempts are not mostly award summaries. Almost half are void sentences that `names_an_award` misreads. Final classes:

| class | means | orgs |
|---|---|---:|
| D | pure void prose | 185 |
| B | lists or co-winners in one award block | 99 |
| A | prose naming exactly one winner | 57 |
| C | text-era `Lot n)` lists | 29 |
| F | other | 6 |
| E | real name | 0 |

What happens to each class:
- **D becomes `VoidLot`.** Seven narrow changes, all inside `names_an_award`'s own arms, move 173 D orgs (187 mentions) to `VoidLot`. A simulation over all 376 names moves no A, B, C or F org. 12 D orgs stay kept as an accepted residual.
- **A1 gets a winner extractor.** 47 A orgs follow the award-clause shapes (`attribué à X`, `Attributaire: X`, …). An extractor in `partyname` turns each into its winner's own name. The fold keeps the party and rebinds its mention, and no figures are read.
- **A2 is deferred.** The other 10 A orgs (a name followed by a void tail) wait for U6, which runs only if U5's read is clean.
- **B is kept as `AwardSummary`**, per 511 §5.
- **C goes to a sibling issue:** a fix in `text/parse.rs` plus a reparse of the pre-2004 text.
- **F is resolved row by row.** 4 orgs assert an award but name nobody, and stay kept. 2 are eForms tenderers the publisher marked "not awarded", and go to their own issue.

The issue's option (a), building `RES-n` results per lot from the summary string, is rejected for every class.

**Basis.**
- **The list.** `../525-void-templates/void-names-refold-after-dry.json` `exempted` holds 376 orgs and 396 mentions, and is identical to dry 2355's list. It was taken after 525's drain. Since 525, the walk exempts exactly the parties the fold keeps, so these 376 are every org in the void net the fold keeps today. The whole effect of the rules below on today's corpus is measured on this list.
- **Reasons in the list.**
  - 344 read "an award summary among them" (342, plus 2 with a mixed reason);
  - 31 read "0/1 names void", meaning the classifier returns a name after a void prefix;
  - 1 is kept by the majority tally (2/24).
  - The 300-character cut: names in the report stop at 300 characters (63 of 376). Four rows were classed on the full value a challenger read: 2378132, 31450417, 31491056 and 31375951.
- **Classification.** Four batches of 94, each labelled and then fully re-read by an adversarial challenger. About 86 public notice reads in all, with 0 failures and no retries. Downloads are in the session scratchpads (`520b0-notices`, `520b1-dl`, `520b1-rv-dl`, `520b2`, `v520b2r`, `v520b3`).
- **Simulation.** I wrote a Python port of `names_an_award` and its helpers (scratchpad only, not committed). It reproduces all 108 specimens of partyname's two award tests with 0 mismatches. I ran the seven narrowings (§4.1) and the A1 extractor (§4.2) over the 376 names and over the 226 void-net literals in partyname's tests.
- **Out of scope:** the 242 `AwardSummary` exemptions of the 511 placeholder dry (`../511-placeholders/u6/plan-dry.json`), which are pointer-plus-body names.
- **Line numbers** refer to HEAD `18172cd`. `partyname.rs`, `project.rs`, `supervisor.rs` and `text/parse.rs` have no uncommitted edits.

### 1. Classes and counts

| final class | sub-shape | orgs | mentions | decision |
|---|---|---:|---:|---|
| **D** pure void, no winner | D1–D7 (§4.1) | 173 | 187 | `VoidLot` (U2, U3) |
| | residual | 12 | 12 | stays kept (accepted cost) |
| **A** one real winner | A1 award clause or awardee label | 47 | 49 | extract the winner (U4, U5) |
| | A2 a name followed by a void tail | 10 | 11 | kept; U6, conditional |
| **B** several winners or lots in one block | B3 multi-winner lists | 76 | 76 | keep (511 §5) |
| | B1 list grammar with one distinct winner | 13 | 13 | keep; recorded candidate |
| | B2 co-winners of one result | 10 | 11 | keep; recorded candidate |
| **C** text `Lot n)` lists | | 29 | 29 | sibling issue (App. C) |
| **F** other | F-P: award asserted, no name | 4 | 4 | stays kept (residual) |
| | F-role: eForms tenderer marked not awarded | 2 | 4 | own issue (App. D) |
| **E** real name with a void word | | 0 | 0 | — |
| **total** | | **376** | **396** | |

- **Era.** A has 54 XML-era orgs (r208 2011–2016) and 3 from pre-2004 text (31689490, 31738609, 31739133). All 99 B are r208 (2008–2017). All 29 C are 1995–2003 text. D has about 167 XML-era orgs and 18 text-era ones: 1997–2009 `SECTION VI` spills, plus 31674188/9 and 30680275.
- **Reason today.**
  - A: 50 summaries, plus 7 plain names (the `Pas d'offre … Attribué à X` template, 18476377/8).
  - D: 160 summaries, 24 plain names, and 1 kept by the majority tally (7969717).
  - B, C and F: all summaries.

### 2. Disagreements resolved

| org | given → challenger | final | why |
|---|---|---|---|
| 1341883 | D → F | **F-P** | 12497785 RES-1 is an award: contract number 112121, award date 2011-07-01, and the text itself says "257 lots attribués". The name names no winner. `VoidLot` would mark a mostly awarded 333-lot result as void, which is the costly direction. No new rule touches it, because it carries an award word with no negation. |
| 1506401 | A → F | **D** (future counterparty) | 12543416 RES-3 has 0 offers, no award date, no contract number, and future-tense prose ("sera passé avec la STÉ France Télécom"). The lot is void in this notice. This is the same shape as the SDIS 33 relaunch-intent rows that both reviewers put in D (3526206–10). The challenger is right that it must never be extracted. Here it is dropped as void (D6b). |
| 7151562 | A → F | **A1** ("san see") | The publisher's past-tense "Attribué à" is the same as in its six siblings, which match real awardees. No rule can single out an unverifiable spelling, and the extractor serves names as published (`CSL Berhing`, `Sarltacavl`). Today the same claim is already served, inside a sentence. Recorded as unverified. The U5 read lists it. |
| 30533823, 30533824 | E → F | **F-role** | Real tenderers (KvK 69341172, 61820210). In 24400562, CON-0001 BT-3202 lists their tenders, so the fold serves them as winners, but the publisher annotates them "(niet gegunde partij)". This is a role defect, not a name class: App. D. |
| 31375951 | D → F | **F-P** | 4357356 RES-1 has award date 2010-12-27 and value 312,675.04 EUR, and the text says "61 lots attribués à 29 fournisseurs". It stays kept: the un-negated `attribués` keeps it. |
| 31377625 | D → F | **F-P** | 4359993 RES-1 award date 2010-07-22, "220 lots ont été attribués", and "La liste des attributaires … ci-dessous" points to RES-2..30. Stays kept. |
| 31486027 | D → F | **F-P** (lower confidence) | "attribué par marché négocié … suite à appel d'offres infructueux". RES-4 has no date or value, but the text asserts an award. Stays kept: an award word with no negation is never read as void. |

**Reclassed without a dispute.**
- **31527955/56/60/61/64/68: F → D.** The labeller and the challenger agree that each names a failed bidder (deadline after publication, and no value fields), and that the party should be dropped as void. That is D by definition.
- **Reconciling A and B (A means prose; a list grammar is B):**
  - 5351956, 5351964, 31518419, 31919182 and 31919183 move A → B1. Batches 1 and 3 had put one-winner lists in A; batches 0 and 2 had put them in B.
  - 2381362 moves B → A1: one award clause ("Lot 32 attribué à Pierre Fabre Dermatologie"), and the RES's other lots relaunched or void. This is the same shape as 31393407 (Nectarys), which both reviewers confirmed as A. The winner of any part of a result's lots is that result's winner, and void parts do not contradict `selec-w`. The same rule makes every partial-void A correct.
  - 3993625 and 4836476 stay A, as A2.

### 3. Representative examples

- **A** (A1 unless marked):
  1. 1435451 `Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Arrow Génériques suite à procédure négociée …` → `Arrow Génériques`
  2. 7791767 `Infructueux (1 offre irrégulière). Le lot a été relancé en procédure négociée (article 35.II.1 CMP) et a été attribué à l'entreprise Promocash (02100 Saint-Quentin)` → `Promocash`
  3. 7030205 `Lot infructueux relancé suivant procédure négociée … Attributaire: ARDP` → `ARDP`
  4. 31482010 `lot déclaré infructueux le 21.2.2011, procédure négociée relancée … marché attribué à: Berthelet` → `Berthelet`
  5. 4782526 `… a décidé d'attribuer ce lot à la société Eiffage Construction SISE rue François Durieux – 59174 La Sentinelle …` → `Eiffage Construction`
  - A2: 1844226 `BGE Guyane 16 rue Lt Becker 97300 Cayenne / La consultation a été déclarée sans suite pour les parts non couvertes des besoins`
- **B:**
  1. 3371523 `lot 43 INFRUCTUEUX - 44 reckitt benckiser 28692.07 - 45 reckitt b 165.24 - 46 ipsen p 2497.138 …` (B3; abbreviations and unit prices)
  2. 1642155 `BRI: lot 1: marché n° 12.001 / Arthrex: lots 7 et 10: marché n° 12.006/ Stryker: …` (B3)
  3. 2957542 `lot 1: Philips Lot 2: Philips Lot 3: Infructueux Lot 4: PHILIPS Lot 5: SIGIL Lot 6: PHILIPS Lot 7: Sans suite` (B3)
  4. 1813067 `1/ Gidef, … Bondy - 2/ Auxet, … La consultation a été déclarée sans suite …` (B2; 16859068 RES-16 reads "Nombre de titulaires: 3")
  5. 7685392 `Gallego secteurs 3A à 3D / infructueux secteurs 3E à 3H /` (B1)
- **C:**
  1. 31735007 `Lot 1) Blin. Lot 2) Prodirest. Lot 3) Ouest Distribution. … Lot 6) Déclaré infructueux. …`
  2. 30687978 `Lote 1) Declarado desierto. Lote 2) Limpiezas Luz`
  3. 30695615 `A) non attribué. B1) et B2) Climelec`
  4. 30698459 `Lot 1) déclaré infructueux. Lots 2)` (a cut list; TXT-CO goes on to `SA EVM - Les Pépinières des Vallées`)
  5. 26966868 `Lote I: declarado desierto. Lote II: Winterthur Seguros Generales`
- **D:**
  1. 3666893 `Le lot 102 : lot B10: Menuiseries bois PVC serrurerie vitrerie (Genevois) a été déclaré infructueux` (D1)
  2. 1171947 `Procédure incomplète. Le marché n'a pas été attribué. Appel d'offres sans suite` (D2a)
  3. 3526206 `Déclaré infructueux - à relancer en marché négocié … avec les sociétés ayant soumissionné à l'appel d'offre : lot 1 : société TIB` (D2c)
  4. 4774593 `décision de déclarer infructueux le lot nº 10 … ne permettent pas d'en établir l'attribution …` (D3)
  5. 25755356 `Desierto. SECTION VI: OTHER INFORMATION VI.3) Date of contract award: 2.1.2004. …` (D7)
  - Also 31527955 `Société Sindarro Lot déclaré infructueux relancé en marché négocié …` (D6c).
- **F:**
  1. 1341883 `Demander le détail à la cellule marchés … 257 lots attribués Les 74 lots infructueux sont relancés 2 lots sans suite …` (F-P)
  2. 31377625 `… 220 lots ont été attribués le 22.7.2010. … La liste des attributaires des 220 marchés (circuits) est présentée ci-dessous.` (F-P)
  3. 31486027 `attribué par marché négocié passé selon art. 35-I.1 du CMP suite à appel d'offres infructueux (publicité …)` (F-P)
  4. 31375951 `Au terme de cette analyse … 11 lots ont été déclarés "Sans suite", 5 lots … "Infructueux" …` (F-P)
  5. 30533823 `Humble Inpect B.V. (niet gegunde partij)` (F-role)

### 4. Decision per class

#### 4.1 D → `VoidLot`

**What it changes.** A party whose every name is `VoidLot` gets no mention. Its legacy result reads `clos-nw`, which outranks value and date (510; project.rs:6652, :6718). So about 187 results move from `selec-w`, whose winner today is a void sentence, to `clos-nw`. That is the correction, and it is also the risk: a D rule applied to an awarded result would close it. Hence two rules:
- **Award words with no negation are never read as void.** So the F-P rows (`257 lots attribués`, `attribué par marché négocié`) and 4409817/20/21 (`attribué le 28.10.2013`) stay kept.
- **The drain reads each result's award evidence before the wet run** (§6, U3).

**The seven narrowings.** Each misfiring signal is fixed in its narrowest place, inside `names_an_award` (partyname.rs:2822):

| rule | misfiring signal | narrowest change | orgs / mentions | specimens | near misses that stay kept (tests) |
|---|---|---|---:|---|---|
| **D1** one lot's BOAMP void sentence | `several_lots` (a lot's two designators: `Le lot 102 : lot B10`, `le lot 1 : Lot no1`, `Lot 2011-231 01`, sub-lot codes `nob02`); `mixed_segments` (a `;` in the lot title, `O&Amp;K`); `awardee_noun` (`titulaire` in the description); `company_token` (`SAU`, a typo for *sauf*); `numbered_entries` (bullets) | new pre-check `lot_title_void(raw, fold)`. The raw text opens `Le lot <n°?> <digits> :`, with exactly one such opener. The fold ends `a ete declare(e)(s) infructueux / infructueuse / sans suite`. The fixed `award_clause` is quiet. Then `VoidLot`, before `names_an_award` | 68 / 70 | 3666893, 31481092 (`Communauté de Communes du Genevois` is the lot title), 4133918 (`JCB` is a brand), 1513898, 5772779 | `lot 25) la société Storz a été retenue. Le lot 32 a été déclaré infructueux` (no `Le lot n :` opener); `Le lot 3: X. Le lot 4: Dupont SARL a été déclaré infructueux` (two openers); `Le lot 2 : attribué à Dupont SARL ; le lot 2b a été déclaré infructueux` (award clause) |
| **D2** `award_clause` (:3116) | (a) `n'a pas été attribué`: the check looks only one word back; (b) `déjà attribué (au lot A 12)`; (c) the `avec la/les société(s)` cue in a relaunch intent | (a) negation is the nearest word that is not in `NEGATION_FILLERS`, as `awardee_noun` already does; (b) `deja` directly before → no award; (c) the cue is ignored after the infinitive `relancer`/`negocier` or the future `sera` in the same sentence | 17 / 17 (8 + 3 + 6) | 1171947, 7093933, 31674188, 3526991, 4032000 | `puis attribué à la Société Sandoz`; `relancé en procédure négocié … avec la société B BRAUN` (past participle); `Sans suite, marché passé avec la société Dupont`; `marché attribué le 9/12/13 à Assurances securité`; `attribué le 28.10.2013` (stays kept) |
| **D3** `awardee_noun` (:3020) | descriptive or hypothetical uses: `en cours d'attribution`, `avis d'attribution`, `ne permettent pas d'en établir l'attribution`, `l'attributaire pressenti`, `société attributaire` in liquidation, `multi-attributaires`, `désistement du titulaire`, `l'offre retenue`, `propuesto adjudicatario`, `awarded as part of a new project`, `wordt gegund op` | an awardee noun counts only as a **label** (`Attributaire:`, `Nom du titulaire:`, `titulaire 1:`, `fournisseur retenu =`), never `attribution` as a label (`Date d'attribution :`); or as `(mandataire)`, `attribution à`, or a participle followed by a party preposition (`adjudicado a`, `aggiudicato alla`, `awarded to`, `gegund aan`). 510's negation test stays | 39 / 47 | 4774593, 6286857, 31489554, 7596880, 10528138 | `Nom du titulaire: Keolis`; `Attributaire: ARDP`; `fournisseur retenu = zoll medical France`; `GRPT GCC (mandataire) et Andre BTP, suite à …`; `Lot 3) attribution à: SA Maîtres Laitiers Distribution`; `lote 1 adjudicado a Construcciones Pérez` |
| **D4** lot lists | `several_lots` / `numbered_entries` / `lettered_entries` firing on lists of void lots and on a relaunch's new lots | (a) a glued number mark (`no3`) reads as its number; (b) designators after a relaunch marker (`relanc…`, `recomposition`, `décompos…`, `nouvelle procédure`, `nouveaux lots`, `se reporter`) belong to the relaunch; (c) a list whose every entry carries a void verdict is void. Entries holding only numbers and conjunctions join the next one. An entry with nothing after its label keeps the summary | 11 / 12 | 2948392, 4726611, 7815395, 6333652, 12169942 | `Lot 1) déclaré infructueux. Lots 2)` (a cut list); `G) non attribué. Marché conserves: A)`; `Lot 1) Dupont. Lot 2) infructueux`; every C specimen |
| **D5** `mixed_segments` (:3099) | a reason or publication reference after `;`; a `;` inside parentheses or an HTML entity | split at `;` outside parentheses, after decoding `&amp;`. A segment is void by `mentions_void` or a `BLANK_WHOLE` value (`néant`). A segment with an offer or bidder word (`offre`, `oferta`, `offerta`, `oferty`, `ofert…`, `licitador`, `soumissionn…`, `wykonawc…`, `inschrijving…`) is a reason. `annonce`, `JOUE` and `BOAMP` join the relaunch-note words | 12 / 15 | 1513105, 3534542, 1814340, 11663720, 2678824 | `Bp innovis ; infructueux`; `2013023201/zone Belley-Bas Bugey : Sarl Mcb - 01300 Chazey Bons ; …: infructueux`; `2012021901 = JBE Electrcité; …` |
| **D6** `company_token` (:3002) | the form names a bidder, not a winner | a company form does not count when: (a) it comes after an offer word in a sentence that rejects the offer (`inacceptable`, `irrégulière`, `escluso`, `excluida`, `écartée`); (b) it comes after `relancer`/`négocier`/`sera` in the same sentence; (c) the name is `Société`/`Sté <X>[.] Lot déclaré(e) infructueux …` with no lot number and no award cue | 9 / 9 | 31527955 (invert partyname.rs:3298), 1974244, 11251840, 1506401, 31527961 | `Yvelin SA. Lot 2 : infructueux`; `Stryker France SAS: lot nº 36 sans suite: lot nº 37`; `BGE Guyane 16 rue … / … sans suite pour les parts non couvertes …`; `Lot infructueux, sera relancé. Lot 2: Dupont SARL` (new sentence); `DESIERTO, S.L.` |
| **D7** `named_before_spill` (:2991) | the void head of a text-era `SECTION VI` spill read as a name; `see section VI.2` pointers | the head names a party only when `mentions_void` is false for it (whole value, reason head, prefix), after a trailing `see` is dropped | 14 / 14 | 25755356, 29760975, 30537999, 14580326, 31854611 | `Zundel Et Kohler. SECTION VI: …`; `Getinge France. SECTION VI: …` |

Three more rows need two rules together:
- 1537305 and 3556403 need D3 and D4;
- 3526210 needs D2c and D6b.

Total moved: **173 orgs / 187 mentions**, behind 116 or more notices.

**Where the rules live.** Every change goes into `names_an_award`'s own arms, through wrappers: `lot_title_void`, `award_awardee_noun`, `award_several_lots`, `award_mixed_segments` and `award_company_token`. The shared helpers stay as they are, because 511 reads them:
- `summary_shape` (:2683);
- `named_before_stem` (:2540);
- `names_a_legal_form` (:1285);
- `names_something` (:2659).

Each change either quiets `names_an_award` or adds a `VoidLot` path, so no name that is `VoidLot` today stops being one.

**Simulation.**
- **The 376 names.** The rules move the 173 D orgs and no A, B or C org. One F org, 31375951, reads void on its cut name only. Its full value carries "61 lots attribués à 29 fournisseurs", which keeps it.
- **The test literals.** Of the 226 void-net literals in partyname's tests, two flip, both intended:
  - `Société Sindarro Lot déclaré infructueux relancé …` (:3298, summary → void);
  - the "accepted cost" assertion `Sublote 9.1: Declarado desierto. Sublote 9.2: Declarado desierto.` (:3379, → void).

**Residual (12 orgs / 12 mentions, kept).** Each costs a junk leaf name and never an award:

| org(s) | why it stays kept |
|---|---|
| 4409817, 4409820, 4409821 | `attribué le 28.10.2013`: an award word with no negation; the award belongs to the relaunched ESID project |
| 4802635, 4802636 | `opdracht wordt gegund op totaal der percelen = perceel 3`: two `perceel` numbers |
| 10893714 | `Perceel 1 NIET gegungd B.V.`, a dummy contractor |
| 23532535, 28310858, 31391828, 31497872 | one void verdict governing a lot list, so some entries carry no verdict of their own |
| 31733023 | `aucune entreprise ne s'est portée candidate … suite à l'appel d'offres infructueux`: `only_an_earlier_void` fires |
| 7969717 | EP 18715886's 24-language composite (`Non applicable — Aucune offre n'a été reçue — Lot 5 non attribué`). Only 2 of 24 copies read void. This is a 525 template gap, not `names_an_award`; it goes to 525's template tail |

#### 4.2 A → extract the winner (A1 now, A2 later)

**The A1 rule.** A new pure function, `partyname::award_winner(name) -> Option<String>`. The fold consults it only for a name in the void net that it keeps: an `AwardSummary` name, or a plain name after a void prefix (`Pas d'offre … Attribué à X`). Ordinary names are never read.

**Cues** (the measured French forms):
- (a) `attribué(e)(s)`: not negated (D2a's test), not after `déjà`, and followed in the same sentence, within 8 words, by `à` and an optional `:`. The 8 words cover `le <date>` and `dans le cadre de cette nouvelle procédure`.
- (b) `d'attribuer ce lot à`.
- (c) A label: `Nom du titulaire:`, `Nom de l'attributaire:`, `Attributaire:`, `titulaire:`, `fournisseur retenu =`.
- (d) `avec la société X` closing the name, after a past `relancé`/`négocié`, with no `relancer`/`négocier`/`sera` before it.
- (e) `la société X a été retenue`.

**Stripped:**
- a descriptor: `la Société`, `société`, `l'entreprise` / `L'entreprise`, `l'esat`, `la STÉ`;
- a lower-case article (`la Sarltacavl` → `Sarltacavl`; `La Poste` keeps its capital `La`);
- quotes.

**The name ends at:**
- ` suite`, `,`, ` (`, ` pour un montant`, ` SISE`, ` le <digit>`, or a spaced dash;
- a five-digit postcode;
- a sentence end, meaning a `.` that does not follow a single letter (`A.E.C. Castellane` and `C.A.J.L` stay whole);
- the next cue, or the end of the text.

**It returns nothing when:**
- there is no cue;
- the name does not open with a letter (`à 29 fournisseurs`), or opens with a count noun (`fournisseurs`, `lots`);
- there are two or more distinct names (2381358, 6672986, 7512199, 23645704, `titulaire 1: SAGA titulaire 2: Chapelec`, 2450314's two `attributaire:` labels);
- a list separator (` / ` or `;`) is followed by a non-void segment after the name (31414415: `RMI … / SCC … / Est Multicopie`);
- the party's copies disagree.

**Near misses (each a test, expected nothing):**
- 1506401 `sera passé avec la STÉ France Télécom`;
- 3526206–10, 3526991, 4032000 (relaunch intent);
- 31527955 family (failed bidder before a void verdict on the whole lot);
- 1974244 (rejected offer); 11251840 (excluded bidder); 1633917 and 7395857 (liquidation);
- 31383538 `Voir l'avis d'attribution … pour l'entreprise attributaire du lot`;
- 4409817 `attribué le 28.10.2013`; 31674188 `déjà attribué au lot A 12`;
- 1341883 `257 lots attribués`; 31375951 `61 lots attribués à 29 fournisseurs`; 31486027 `attribué par marché négocié`.

**Simulated on the 376.**
- The draft extracts the expected winner for 46 of 47 A1 orgs. The 47th, 2381362, fails only because the draft looked for negation across a sentence boundary (`sans suite. Lot 32 attribué`). The spec scopes it to the clause, as D2a does.
- No extraction on any D, C or F org.
- On B, it refuses 7 multi-name orgs. The 3 single "extractions" it makes come from the 300-character cut or a label overrun, which the spec closes: 31414415 (separator rule), 5425803 (next-cue terminator), and 2450314 (second label past the cut).

**In the fold.**
- **The mention.** The party keeps its section. Its mention's name, and every variant with the same text, becomes the winner.
- **Identity.** A winner's name is its identity (issue 234). So the mention binds the existing org of that name and country where one exists (Promocash, Berthelet, Faure/Annequin, Eyraud/Perraud, `Assurances Securite` 2992764), or mints a new one.
- **The result** stays `selec-w`.
- **No figure is read** (491's rule): Eiffage's 5,141,184.66 and Sarltacavl's 408,994.55 stay unclaimed.

Spellings are served as published:
- `CSL Berhing` (a typo);
- `Sarltacavl` (fused; it will not merge with SARL TACAVL 9757611, which is 514/524's work);
- `Léo Lagrange` (a short form of `Léo Lagrange Méditerranée`);
- `san see` (unverified).

**A2 (10 orgs).** A name followed by a void tail: BGE Guyane ×3, LV Consultants, GRPT ×2, Pentax, Bp innovis, Mepy Système, Importaciones Canarias. It is not in U4. Cutting the tail is easy for these:
- the `EARLIER_VOID` tail (2734624/5);
- a trailing void-only `;` segment (4836476);
- an address opening `<digits> <street word>` (BGE).

It is not reliable for the others:
- `LV Consultants Cv1 Simarouba - 97310 Kourou` (no street word);
- Pentax's amounts;
- the two text-era rows.

U6 builds the reliable subset only if U5's read is clean.

#### 4.3 B → keep (511 §5)

All 99 stay `AwardSummary`. The leaf-org reasoning of 511 §5 holds, and the measured shapes add to it:
- **B3 (76): several winners in one award block.** Splitting would invent result sections from a string. It would mint abbreviations (`sanofi av`, `reckitt b`, `fresenius k`, `leo p`) and claim unit prices (`28692.07`, `1691.1ht`) as awarded amounts. The summary is the only place these names reach the Tender.
- **B1 (13): one distinct winner in a list grammar** (`Name: lot nº 36 sans suite: lot nº 37`, `Name secteurs 3A à 3D / infructueux …`, `Secteur S: Name (n %)`, `lot a : X Lot b : sans suite`, `Name : lots n … lots infructueux : …`). Extraction would be correct for the result, but each grammar needs its own parser, and two of them (8747281, 8747284) carry rebate figures. Not scheduled.
- **B2 (10): co-winners of one result** (`1/ Gidef … - 2/ Auxet …`, `titulaire 1: SAGA titulaire 2: Chapelec`, `Rang 1: Helmlinger, Rang 2: Jonnette`, three S.A./S.L. names in 4726614). Splitting them into N winners would be correct, but it needs N mentions from one party section, which means a new mention key. The same shape also appears without any void word (1844214, 1844219, 1844230, 1844256: 16859068's three-awardee rows), so a B2 build is its own census and issue. Not scheduled.
- **No follow-up filed.** The class is closed: it comes from r208 2008–2017, and eForms publishes one party per winner. It cannot grow.

#### 4.4 C → sibling issue

The 29 orgs are pre-2004 text. The fix belongs to the text parser: `lot_prefix_len` (text/parse.rs:540) and `winner_segments` (:660), plus a reparse. The draft is in App. C.

Under D1–D7, all 29 stay kept, including the cut and lettered lists (30698459, 31731990, 31731993), because an empty entry keeps the summary. They must never become `VoidLot`, because their results carry awarded lots:
- SA EVM for lots 2–4 (30698459);
- Eurocash for A) and D) (1801614).

The A1 extractor's text hook belongs to that issue. Today `plausible_name` (:148) refuses every non-name, so `lot 25) la société Storz a été retenue` would be lost on a reparse.

#### 4.5 F → row by row

- **F-P (1341883, 31375951, 31377625, 31486027): kept as `AwardSummary`.** The right class would be `Placeholder` (drop the org, `selec-w`, no winner). A rule for "award with no named party" would also catch 4409817/20/21, whose award belongs to another project. Not built. 4 leaf orgs.
- **F-role (30533823, 30533824): its own issue (App. D).** The pinned specimen `Humble Inpect B.V. (niet gegunde partij)` must stay a kept name.

#### 4.6 E → none

Batch 2's two E rows resolved to F-role.

### 5. Build units

1. **U2 (S–M): D predicate.** Only `crates/ingest/src/partyname.rs` changes: D1–D7 as in §4.1.
   - **Rewrite the assertions that flip:**
     - Sindarro (:3298) moves from the summary list to the void list;
     - Sublote 9.x (:3379) now expects `VoidLot` (the accepted cost becomes the fix).
   - **New tests** (prod specimens; at least 5 per rule where measured):
     - `one_lots_void_sentence_is_void` (D1)
     - `a_negated_earlier_or_intended_award_is_no_award` (D2)
     - `an_awardee_word_must_label_a_party` (D3)
     - `a_list_of_void_lots_is_void` (D4)
     - `a_reason_segment_names_nobody` (D5)
     - `a_bidder_is_no_winner` (D6)
     - `a_void_head_before_a_spilled_section_is_void` (D7)
   - **Pins over this record:**
     - `every_kept_520_exemption_keeps_its_class`: the A, B, C and F specimens and the 12 residual D;
     - `the_shared_shape_helpers_are_unchanged`: `summary_shape` and `names_a_legal_form` give the same answers on 511's specimens.
   - **Gate** through `ops/check.sh`, then one deploy.
   - **No supervisor change.** Since 525, the walk re-queues mentions by the fold's own decision (`split_void_hits`, supervisor.rs:3849).
2. **U3 (S): drain D** (§6).
3. **U4 (M): A1 extraction.**
   - **partyname:** `award_winner` and its tests: `award_winner_names_the_one_awardee` (47 specimens against App. B), the near misses in §4.2, the strip and terminator cases (`La Poste`, `A.E.C. Castellane`, `Grunenthal/CSP`), and a guard that `award_winner` returns nothing for every D and C specimen.
   - **project.rs:**
     - `drop_decision` (:4610) records a `renamed` winner for a kept party whose void-net copies all yield the same winner;
     - `NoticeState::mentions_and_dropped` (:5394) uses it as the name;
     - add `PartyVerdict::Renamed` (:4542);
     - add `Report.extracted`, with a count and up to 20 samples per fold.
   - **supervisor.rs:**
     - `split_void_hits` re-queues mentions whose verdict is `Renamed`;
     - the `void-names-refold` report adds `extracted: [{org, name, winner, notice}]`;
     - `docs/operations.md` gets one runbook line.
   - **Fold tests** (r208 and 434 harnesses): `a_summary_party_binds_its_extracted_winner`, `copies_that_disagree_keep_the_summary`, `a_refold_rebinds_the_mention_to_the_winner` (rebound = 1, the summary org orphaned), and `the_void_walk_requeues_an_extracted_summary`.
   - **No new `Spec` arm,** so no `run_spec` frame growth. If one is ever added, wrap it in `Box::pin`.
   - **Gate:** `ops/check.sh` (supervisor.rs needs `--features server`; a bare `cargo check` compiles nothing there).
4. **U5 (S): drain A1** (§6).
5. **U6 (S, conditional): A2**, the reliable subset of §4.2. Same harness, same drain shape.
6. **File now:** the (C) issue (App. C) and the F-role issue (App. D).

U2 and U4 ship in separate deploys, and U2 does not wait for U4.

### 6. Drains

**U3, right after U2's deploy.** Run `refold-void-names` dry straight after the deploy. Otherwise add back the `mentions_retired` of any folds that ran in between.
- **Expected:**
  - 173 re-queued orgs, 187 mentions, 116 or more notices;
  - about 203 exempted: 57 A, 99 B, 29 C, 6 F and the 12 residual D;
  - `identified` 0, `legal_form` 0.
- **Stop if:**
  - re-queued orgs fall below 165 or rise above 181;
  - any re-queued org is on App. A's A, B, C or F lists;
  - any of those lists is missing from `exempted`.
- **Read before the wet run:**
  - every re-queued name not on App. A's D list;
  - the award evidence of each re-queued mention's result block (public `/content` paced at 0.5 s, or a bounded `/v1/sql`): award date, `VAL_TOTAL` / `VALUE_COST`, contract number, offers received. Read every result that carries an award date or a contract number. Stop if any reads as an award (the 1341883 test).
- **Wet:** `refold-void-names expect=<dry notices>`, then its paired fold.
  - `mentions_retired` ≥ 187, after the add-back;
  - `retire_corrections` > 0 (the `selec-w → clos-nw` corrections);
  - `mentions_rebound` about 0.
- **Sweep:** auto, under the cap; `counted` about 173.
- **Afterwards:** the `Infructueux` prefix read returns 6 (6499376 is gone). 3666893, 1171947 and 25755356 read `clos-nw` on their Tenders.

**U5, after U4's deploy.** The dry run lists `extracted`: 47 orgs, 49 mentions, 28 notices.
- **Read all 47** against App. B. Stop on any extraction that is not in App. B, names a different winner, or comes from an org outside the A1 list.
- **Wet + fold:** `mentions_rebound` about 49; no retirement from this set.
- **Sweep:** about 47 summary orgs.
- **Afterwards:**
  - the `Infructueux` prefix read returns 1;
  - the org `Promocash` gains 7791767's Tender;
  - `name_prefix=Lot%20d%C3%A9clar%C3%A9%20infructueux%20lors` returns 0, down from the 13 A1 rows today.

### 7. Risks and pins

| risk | pin |
|---|---|
| A real award becomes `clos-nw` | Award words with no negation are never read as void (F-P and 4409817 stay kept). D1 needs exactly one `Le lot n :` opener and the verdict closing the name. D4c needs every entry void, and an empty entry keeps the summary. D6 needs a rejection, a relaunch intent, or a void verdict on the whole lot. The simulation moved 0 non-D orgs. U3 reads each result's award evidence, with a stop rule. |
| A real winner list is lost | C and B stay kept (29/29 and 99/99 in the simulation). The cut-list and lettered near misses are tests. App. C forbids a `VoidLot` fallback. |
| The extractor names a bidder, a future counterparty, a pointer or a count | The near-miss tests in §4.2. Two or more names, or a list separator, returns nothing. Every extraction is read against App. B before the wet run. |
| An extracted name pollutes a real org | Only the 47 expected strings bind (U5's read). Merges into existing same-name orgs are intended. Spelling splits (`Sarltacavl`, `Léo Lagrange`) are accepted and left to 514/524. |
| 511's placeholder keep rule shifts | The narrowings live only in `names_an_award`'s wrappers, pinned by `the_shared_shape_helpers_are_unchanged`. |
| The 300-character cut hides a keep signal | Four rows were classed on full reads. The dry walk reads full values, and U3's stop rule requires every A, B, C and F org to be in `exempted`. |
| The text parser loses winners | D does not change `plausible_name` (it refuses every class today). The A1 hook and the `Lot n)` fix land together in App. C, before any pre-2004 reparse. |
| A new arm overflows `run_spec` | No new arm. `ops/check.sh` only, never piped through a filter, and read `GATE-EXIT=`. |

### 8. Verify

```
curl -s "https://tenders.zebreus.click/v1/organizations?name_prefix=Infructueux&limit=50" | jq '.items|length'
```

- **Open:** 7 (2026-10-12): 3982542, 6499376, 6603158, 7279113, 7791659, 7791721, 7791767.
- **After U3:** 6.
- **Done (U5):** 1. The one left is 6603158 `Infructueux: lot nº 24 Orthomed: lots nº 25 et 26 Biotech Ortho: lot nº 27`, a B list kept by 511 §5.

This corrects the triage's "Done: 0", which assumed B would be resolved.

### Appendix A: ids by class

- **D1** (68): 1392146, 1513898, 1537987, 1943868, 1943869, 2021395, 2378132, 2528483, 2847767, 2900386, 2900516, 3066482, 3107850, 3238976, 3425880, 3666893, 3666894, 3666895, 3666903, 3666904, 3666905, 3666906, 3915725, 3915726, 3915727, 3915728, 3915729, 3915730, 3915731, 3915732, 3915733, 3915742, 3984070, 3984072, 3984073, 4133918, 4133919, 4133921, 4133935, 4133936, 4272807, 4425125, 4425126, 4425128, 4425129, 4425130, 4425131, 4425132, 4764054, 4846679, 5101244, 5605092, 5772779, 5772782, 5772784, 31427032, 31450417, 31471384, 31473527, 31481076, 31481089, 31481092, 31481093, 31481094, 31481098, 31481123, 31491050, 31491056
- **D2** (17):
  - a: 1171947, 2131011, 3314070, 4229970, 6069583, 7093933, 7524029, 31400142
  - b: 6011832, 31674188, 31674189
  - c: 3526206, 3526207, 3526208, 3526209, 3526991, 4032000
- **D3** (39): 1300853, 1610364, 1633917, 1753646, 1861834, 1861839, 3720532, 4774593, 4774594, 4774596, 4774599, 4774607, 4774610, 4877271, 4885217, 5543579, 5611738, 6286857, 6499376, 6654938, 6688295, 6825282, 7024169, 7395857, 7517636, 7596880, 7596896, 7732764, 7732765, 9907415, 10528138, 28821451, 31383538, 31383542, 31415171, 31424633, 31425155, 31489554, 31489559
- **D4** (11): 2948392, 3717094, 4726611, 4885215, 5425806, 5425808, 6333652, 7815395, 12169942, 30680275, 31428159
- **D5** (12): 1513105, 1710867, 1814340, 2369100, 2678824, 3534542, 5148433, 5961568, 6357683, 7075593, 11663718, 11663720
- **D6** (9): 1506401, 1974244, 11251840, 31527955, 31527956, 31527960, 31527961, 31527964, 31527968
- **D7** (14): 14580326, 25043000, 25755356, 25893172, 29760975, 30336777, 30535445, 30535976, 30537999, 30538906, 30539882, 30786740, 30899597, 31854611
- **Two rules together** (3): 1537305, 3556403 (D3 + D4); 3526210 (D2c + D6b)
- **D residual** (12): 4409817, 4409820, 4409821, 4802635, 4802636, 7969717, 10893714, 23532535, 28310858, 31391828, 31497872, 31733023
- **A2** (10): 1844226, 1844229, 1844232, 2035260, 2734624, 2734625, 3993625, 4836476, 31689490, 31739133
- **B1** (13): 2957546, 5351956, 5351964, 6603163, 7685392, 7685396, 8747281, 8747284, 8788741, 8788743, 31518419, 31919182, 31919183
- **B2** (10): 1813067, 1813070, 1813087, 1844213, 1844215, 1844218, 1844224, 4726614, 5425803, 15591728
- **B3** (76): 1642155, 1722464, 1722465, 1722466, 1722468, 1722469, 2381358, 2381360, 2381361, 2450314, 2957539, 2957542, 2957543, 2957545, 2957549, 3219666, 3219667, 3219668, 3219671, 3219672, 3290486, 3371493, 3371495, 3371500, 3371513, 3371518, 3371519, 3371520, 3371521, 3371523, 3371525, 3371526, 3586203, 4048470, 4359873, 4804532, 4804533, 4804535, 4804536, 4804537, 5351950, 5351959, 5351961, 5351965, 5351966, 6209983, 6209984, 6603158, 6603162, 6603175, 6672986, 6672989, 7512199, 7674818, 7685393, 7685394, 7685395, 8788747, 8788751, 9811971, 10497189, 23645704, 31393408, 31414415, 31428158, 31428160, 31497710, 31518405, 31518406, 31518407, 31518408, 31518409, 31518416, 31518420, 31518422, 31528120
- **C** (29): 26966868, 27028633, 27567379, 30593829, 30665788, 30666364, 30680323, 30686841, 30687978, 30688273, 30689270, 30690240, 30695615, 30698459, 30820452, 31689502, 31731990, 31731993, 31733948, 31734131, 31734397, 31734422, 31735007, 31736174, 31737127, 31738162, 31738464, 31740148, 31740189
- **F** (6): 1341883, 31375951, 31377625, 31486027 (F-P); 30533823, 30533824 (F-role)

### Appendix B: the A1 expected winners (U4 tests, U5 read)

| org | winner | org | winner | org | winner |
|---|---|---|---|---|---|
| 1397079 | B BRAUN | 1397080 | Vascutek | 1435451 | Arrow Génériques |
| 1435452 | Abbott Products | 1435453 | Fresenius Kabi | 1435455 | Sanofi Aventis |
| 1435456 | Grunenthal/CSP | 1435457 | Sandoz | 1484019 | Bionobis |
| 1484119 | CSL Berhing | 1484121 | LFB BIOMEDICAMENTS | 1503868 | Henry Schein |
| 1621278 | Brake France Service | 1668604 | Sarltacavl | 1986366 | Peters |
| 2381362 | Pierre Fabre Dermatologie | 2973447 | Les Ateliers de Polangis | 3863573 | Thermo Onix LTD |
| 3982542 | zoll medical France | 4552687 | Assurances securité | 4782526 | Eiffage Construction |
| 6191476 | Keolis | 7030205 | ARDP | 7030206 | JV Distribution |
| 7151483 | Léo Lagrange | 7151494 | FAIL 13 | 7151502 | C.A.J.L |
| 7151511 | Apis | 7151529 | Léo Lagrange | 7151543 | A.E.C. Castellane |
| 7151562 | san see | 7279113 | Somatico | 7791659 | Transgourmet |
| 7791721 | Lodfirais | 7791767 | Promocash | 7805728 | SMACL |
| 9104280 | OTIS | 9104287 | Menuiserie du Chalonnais | 31381712 | LD Bio Diagnostics |
| 31393407 | Nectarys | 31482009 | Faure/Annequin | 31482010 | Berthelet |
| 31482011 | Segui | 31482013 | Cars du Trièves | 31482014 | Faure/Eyraud |
| 31482016 | Eyraud/Perraud | 31738609 | Storz | | |

### Appendix C: draft of the (C) issue

```
# <N> — text-era `Lot n)` winner lists are never split, so one list mints one org

Status: ready-for-agent — filed 2026-10-12 from 520 unit 1 (§4.4). NEXT: unit 1 build (S–M), then the 1997–2003 reparse.
Kind: data quality (text parser; organizations)
Relates to: 520 (award summaries; its A1 extractor `partyname::award_winner`), 484/485/244 (`lot_prefix_len`, `winner_segments`), 491 (the reparse path), 510/511 (`plausible_name` refuses every non-name).

## What is wrong
`lot_prefix_len` (text/parse.rs:540) accepts a `)` reference only when the head is all digits. So `Lot 1)` is never
stripped, and `winner_segments` (:660) never splits at `. Lot 2)`. One list becomes one org (31735007
`Lot 1) Blin. Lot 2) Prodirest. …`, 20187-2002), and every prefixed name splits an identifier-less company
(`Lot 1) SARL Bonsignore & Olivetta` 30587843 beside `Sarl Bonsignore & Olivetta` 25920173). Since a3153bc, a reparse
refuses a void-bearing list whole (`plausible_name`, :148), so its real winners are lost.
Size (520 triage, 2026-10-12, name_norm ranges): 5,239 orgs start with `lot 1)`, 994 of them lists and 17 with a void word;
141 bare `Lot 1)` and 19 bare `lot 2)`; about 92% text era, mostly 1997–2003. 520 unit 1 classed 29 void-bearing lists (its App. A, C).

## Measured variants (520's 29)
`Lot n)`; `Lote n)` (30687978); `Lot n° n)` (31740189); `Lot 8bis)` (30689270); `Lots 1) et 2)` and the typo `Lor 4)`
(30690240); Roman `Lote I:` / `Lote II:` (26966868); lettered `A)`, `B1) et B2)` (30695615); headings
`Marché <category>: A)` (31731990, 31731993; 1801614 lost Eurocash), `Marché n:` (31734422), `Procédure de marché négocié
après appel d'offres infructueux:` (31689502, 31689490); `lot n) la société X a été retenue` (31738609 and siblings);
a list cut at `Lots 2)` (30698459; TXT-CO goes on `Lots 2), 3) et 4) SA EVM - Les Pépinières des Vallées`).

## Fix
1. `lot_prefix_len`: in the `)` and `:` branches, accept an optional `Lot`/`Lots`/`Lote`/`Lotes` word (any case) and an
   optional number mark (`n°`, `nº`, `no`) before the digits, a `bis` or one-letter suffix, and the multi-reference
   `Lots 1) et 2)` / `Lots 2), 3) et 4)`. Stay within LOT_PREFIX_MAX (24).
2. `winner_segments` then splits at `. Lot m)`.
3. Where `plausible_name` refuses an award-summary segment, take `partyname::award_winner(segment)` when it names one
   (`la société Storz a été retenue` → `Storz`).
4. Lettered and heading shapes, and cut lists: refuse as today or keep the segment. NEVER a `VoidLot` drop: these results
   carry awarded lots.

## Tests
`Lot 1) X. Lot 2) Y`; `Lots 2) 8) 20) X`; `Lote 1) Declarado desierto. Lote 2) Limpiezas Luz`; `Lot n° 3) X`;
`Lot 8bis) Willecocq`; a bare `Lot 1)` mints nothing; `Lot A)`, `Lot unique: X`, `LOT Polish Airlines` and
`Lot-et-Garonne` keep their text; the existing `1) CGC` (:3383).

## Drain
Reparse the 1997–2003 text fetches (270+) through 491's path: a dry run first, then read a stride sample of the
re-minted winners, then the wet run and the 443 sweep. This must land before any other pre-2004 reparse.

## Verify
`curl -s "https://tenders.zebreus.click/v1/organizations?name_prefix=Lot%201)%20Blin&limit=10" | jq '.items|length'`
Open: 1 (31735007, 2026-10-12). Done: 0.
```

### Appendix D: draft of the F-role issue

```
# <N+1> — an eForms tenderer marked "(niet gegunde partij)" is served as a winner

Status: needs-triage — filed 2026-10-12 from 520 unit 1 (§4.5).
Kind: data quality (results; roles)
Relates to: 520, 524 (served spelling).

## What is wrong
In Gemeente Maastricht 24400562 (and 24424017), RES-0001 (BT-142 selec-w) has OPT-320 listing TEN-0001..0006, and
CON-0001's BT-3202 lists TEN-0001..0005. The publisher names the real winner `Planon Building Management B.V.
(gegunde partij)` (ORG-0003), but it also puts these tenders in the contract:
- TEN-0002 → ORG-0004 `Humble Inpect B.V. (niet gegunde partij)` (KvK 69341172, org 30533823);
- TEN-0004 → ORG-0005 `Provadi B.V. (niet gegunde partij)` (KvK 61820210, org 30533824);
- ORG-0006 PVM Eindhoven B.V. and ORG-0007 Facilicom Solutions have the same shape.
The fold serves each of them as a winning bidder. The annotation is the only sign that they lost.

## First unit
Measure how far the annotation spreads (name_norm range for `gegunde partij`, with roles). Then decide whether to strip the
annotation from the served name and drop the winner role of a `niet gegunde` party, keeping its Bid. The partyname
specimen `Humble Inpect B.V. (niet gegunde partij)` must stay a kept name.

## Verify
`curl -s "https://tenders.zebreus.click/v1/organizations?name_prefix=Humble%20Inpect&limit=10" | jq '[.items[].name]'`
Open: includes "(niet gegunde partij)". Done: the annotation is gone and the org holds no winner role on 24400562.
```
### U3 award-evidence read (2026-10-13, wf_2f835d23-17e; `u3-award-evidence.json`)

- **Coverage:** all 187 re-queued mentions on 116 notices were read from public `/content`, with 0 unread. 186 read
  `void`: the result block carries no award date, contract number or value; or carries only a decision date or a
  procedure or file reference; or every sublot is void.
- **One reads `award` (owner ruling: proceed, recorded residual).**
  - The mention: 5101244 on 17854467 ORG-7 / RES-6 (lot 6). RES-6 carries no date, number or value, and its party
    says the lot `a été déclaré sans suite`.
  - The award: the notice's free-text ADDITIONAL_INFORMATION reports the lot's RELAUNCH and award (`Lot 6 relancé
    après avoir été déclaré sans suite … Date d'attribution: 8.4.2014 … Marché nº 13PRA01 - Société
    Birmann-Majuscule`).
  - Why proceed: the stop rule targets results that are themselves awarded (1341883). The relaunch award was never
    captured, before or after the drain. Today's served winner is the sentence `… a été déclaré sans suite`, which
    already says no award. After the drain, RES-6 reads `clos-nw`. Birmann-Majuscule is in neither state.
  - Residual: relaunch awards reported only in ADDITIONAL_INFORMATION are not captured; 1 known, 17854467 lot 6.
