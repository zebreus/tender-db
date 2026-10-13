//! Issue 510: whether a party's published name is not a name at all.
//!
//! A party slot sometimes says the lot was not awarded (`Infructueux`, `Sans suite`,
//! `Desierto`, `Lotto deserto`, `Niet gegund`, `Unieważniony`, `Not awarded`) or points
//! elsewhere (`See Section VI.2`, `Véase perfil del contratante`, `Various`). Read as a
//! name, each mints one identifier-less organization that collects every award that
//! printed it: the largest, `infructueux`, held 3,219 r208 winner mentions, all served
//! as a winner of a result the void phrase itself marked `selec-w`.
//!
//! Same contract as [`crate::orgid`]: parsers store what was published, the projection
//! decides. One definition serves every era — the text parser's early drop and the fold.
//!
//! **Four classes** (issue 510, widened by issue 511's unit-1 decision,
//! `.scratch/tender-db/511-placeholders/unit1-decision.md`). The fold drops a party whose
//! every published name is one of the first three ([`crate::project`]'s
//! `dropped_party_sections`), in every era and role; the class decides only what a legacy
//! or sdk-0.1 result left with no real winner reads:
//! - [`NotAName::VoidLot`] — a statement that the lot was not awarded: `clos-nw`.
//! - [`NotAName::Placeholder`] — a winner exists; its name is elsewhere (a pointer: `See
//!   Section VI.3`, `siehe oben`), withheld (`Withheld Section94 Supplier`, `keine Angabe`),
//!   missing (`TBC`, `Unknown`) or plural (`Various suppliers`): `selec-w`, no winner bound.
//! - [`NotAName::Blank`] — the slot holds nothing (`N/A`, `nie dotyczy`, `néant`, `.`): no
//!   marker, so the result's other evidence decides, as if the slot were empty.
//! - [`NotAName::AwardSummary`] — a non-name phrase beside a name: the void phrase in a
//!   summary that names an award ([`names_an_award`]: `Lot 1) Sarl Bremond. Lot 2)
//!   Infructueux`, `Infructueux … attribué à la Société X`), a pointer followed by the body
//!   it points at (`siehe oben: NEB Betriebsgesellschaft mbH`), or a placeholder inside a
//!   list. The text parser refuses it, but the fold KEEPS it: it is the only place the real
//!   winners' names reach the served Tender until per-lot parsing exists (510's drain found
//!   them among the matches, job 2164; 511 §5).
//!
//! **Matching.** On the role census's fold (case, Latin accents and every non-alphanumeric
//! run folded to one space; [`fold`]). A stem matches at a left word boundary and may end
//! mid-word (`infructu` catches `infructueux`, `infructueuse`, `infructuosité`); a whole
//! value matches only the entire folded name, after one trailing lot qualifier
//! (`(lote 3)`, `(lots 2 et 4)`) is stripped — for issue 511's values, a bare one only
//! ([`bare_lot_qualifier`]). Every void entry is a 2026-10-10 prod
//! measurement (`.scratch/tender-db/510-void-names/unit1-decision.md` §2): each whole
//! value says the lot was not awarded, has 20 or more winner-role mentions, and no real
//! company with that exact fold — save TED's own multilingual templates, whose every copy
//! is a void value because the fold reads a party by all its copies: 17674598's (510's drain
//! Verify), and since issue 525 the other templates the kept parties printed (`Kontrakten
//! ikke tildelt.`, `Los wurde nicht vergeben`, `This lot was unsuccessful.`, `entfällt — Los
//! nicht vergeben`), each a form wording, never a company's name. Those are EXACT values
//! ([`VOID_TEMPLATES`]: never measured as a reason head or a prefix, so a name that opens
//! with one and goes on is a name), save TED's own bracketed pointer after one
//! ([`template_pointed`]). Issue 511's entries are its 2026-10-11 census (§4), with
//! four guards for the real names beside them:
//! - every new single word or abbreviation is a whole value (`N/A s. r. o.`, `NIL d.o.o.`,
//!   `Diverse Care Services Ltd`, `Brake` stay names);
//! - issue 511's stems open the name ([`StemAt::Start`], after one optional lot designator),
//!   so `Siemens – see section VI.3` is a name; and a pointer followed by a name is an
//!   [`NotAName::AwardSummary`] ([`pointer_names`]);
//! - the three short words with measured real-name collisions (`ingen`, `inget`, `nd`) need
//!   the notice's language and a plain case ([`not_a_name_in`], [`SCOPED_WHOLE`]: `Ingen` FR,
//!   `InGen`);
//! - the fold keeps a [`NotAName::Blank`] party that carries an identifier unless the
//!   identifier echoes a non-name (the projection's guard, not this module's).
//!
//! Issue 511's unit 4 (the residual after the unit-3 wet run, dry 2273) closes what the
//! anchor and the all-names rule left: a polite first word before a pointer
//! ([`POINTER_LEAD_INS`]: `Please see section VI.4.2`), a form section's own title after one
//! ([`FORM_SECTION_TITLES`]: `See section VI.3 (Additional Information)`), the UK and German
//! withheld and not-applicable sentences (whole values, [`withheld_by_reference`]), and the
//! shape the fold's TED translation echo reads ([`form_section_tokens`], [`translation_shaped`]).
//!
//! Issue 511's unit 5 (dry 2283, `.scratch/tender-db/511-placeholders/u4/plan-dry.json`: about
//! 20k orgs / 53k mentions the lexicon left as names) adds the SECTION-REFERENCE POINTER, a name
//! made of nothing but closed-list words and form section references, in every language the plan
//! holds and TED's 23-language template: `siehe VI.4.1` (2,550 orgs), `V.1) Award and contract
//! value` (2,629 winner orgs), `jak w pkt VI.4.1)`, `cf rubrique VI.4.1`, `Όπως I.1`, `As in I.1
//! above` ([`section_reference_pointer`]). A body's name beside the pointer keeps it a name
//! (`Krajowa Izba Odwoławcza – jak wyżej w pkt VI.4.1)`, `Referat II.3`).
//!
//! Issue 511's unit 6 (the units 4+5 reads and residual dry 2310,
//! `.scratch/tender-db/511-placeholders/u45/plan-residual.json`) closes what the combined wet left:
//! a body named right after a pointer's reference with no separator keeps its party
//! ([`opens_with_a_name`]: `Please see section VI.4.3 Translink`); the `As` family's verbs and the
//! generic nouns, notice names and typos the closed lists lacked (`As shown in section I.1 above`,
//! `See links detailed in II.1.5`, `Siehe die unter Ziffer VI.4.1) genannte Stelle.`), and the UK's
//! long withheld sentences, whole; a spaced reference (`Wie VI.4 1)`, `siehe VI 4.1`:
//! [`raw_pieces`]); the dry plan's legal-form list without the pointers ([`plan_legal_form`]); and
//! TED's earlier template wording (notice 19530163's ORG-10: `Consultar o ponto VI.3`, `Glejte
//! oddelek VI.3)`), each copy a pointer by itself, with the fold's TED template signal for the
//! wordings no list holds ([`template_shaped`], the projection's `template_echoes`).
//!
//! Issue 520's unit 2 (`.scratch/tender-db/520-award-summaries/unit1-decision.md` §4.1) narrows
//! [`names_an_award`] to the award a void phrase states. Of the 376 names the void walk kept after
//! issue 525, 185 were void prose its signals misread — one lot's BOAMP void sentence, `n'a pas été
//! attribué`, `en cours d'attribution`, a list of void lots only, a reason after a `;`, a failed
//! bidder, the text era's `Desierto. SECTION VI` spill — and seven narrowings inside its own arms
//! (D1–D7) make 173 of them void lots; no name with a winner moves.
//!
//! Issue 520's unit 4 (§4.2) reads the one awardee such a kept phrase names ([`award_winner`]: `Lot
//! déclaré infructueux … puis attribué à la Société Arrow Génériques suite à …` → `Arrow Génériques`,
//! `Attributaire: ARDP` → `ARDP`), so the fold binds the party under the winner's own name instead of
//! the summary; two or more names, a list, a bidder, a future or a pointer name nobody, and since the
//! adversarial review neither does an award its clause refuses, plans, negates or undoes, nor a role, a
//! count or a status where the name would stand. The second review refuses more, never extracts more:
//! another contract's, the incumbent's, a scored bidder's or a joint award in the cue's sentence or the
//! next, an undo or a void verdict in a later sentence, a selection-stage `retenue`, a co-winner list,
//! a name that ran on into a procedure, a figure or an address — and keeps the name a dotted legal form
//! opens (`la S.A.R.L. Dupont` → `S.A.R.L. Dupont`, which the build cut to `S.A.R.L.`).
//!
//! The real names beside the phrases stay names — `Desierto OÜ`, `DESERTOT`, `VÁRIOS
//! MUNDOS`, `NIL d.o.o.`, `N/A s.r.o.`, `Void arhitektura`, `Gestiver Información SL`,
//! `Bank of America, N.A.`, `X Corp` — pinned by the tests below.

/// What a non-name is ([`not_a_name`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotAName {
    /// The lot was not awarded.
    VoidLot,
    /// Issue 511: a winner exists; its name is elsewhere, withheld, missing or plural.
    Placeholder,
    /// Issue 511: the slot holds nothing (`n/a`, `none`, `néant`, punctuation only).
    Blank,
    /// Issue 511: a non-name phrase beside a name (the award, the company or the body it
    /// names). Never dropped by the fold; refused by the text parser.
    AwardSummary,
}

/// Where an issue-511 stem must sit on the fold. 510's own stems match at any left word
/// boundary; issue 511 anchors its stems at the start (after one optional lot designator,
/// [`after_lot_designator`]), because a stem matched anywhere drops a real name followed by
/// the phrase (`Firma Handlowa Jan Kowalski – nie dotyczy`, `Siemens – see section VI.3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StemAt {
    /// The stem opens the name.
    Start,
    /// At any left word boundary, as in 510: the phrases that never open a name (`Véase /
    /// Ver perfil del contratante`, `Disclosure would prejudice …`). Anchoring `would
    /// prejudice` would retire it from the fold and the text parser alike.
    Boundary,
}

/// Stems that say the lot was not awarded, matched at a left word boundary on the
/// fold. The 508 text list, plus the `infructeux` misspelling and the feminine
/// `declarada desierta`. Each carries its leading space, the boundary, so the match is
/// a `contains` on the space-padded fold with nothing built per stem (the projection's
/// hot loop: the issue-483 `PADDED_PATTERNS` lesson).
const VOID_STEMS: [&str; 15] = [
    " infructu",
    " infructeu",
    " sans suite",
    " non attribu",
    " declarado desiert",
    " declarada desiert",
    " queda desiert",
    " nessuna aggiudicazione",
    // Belgian bilingual notices pair `Non attribué` with the Dutch, typos included (`Niet
    // gegung`, `Niet gegunde percelen`): a stem, so the pair is void together (the drain's
    // Verify, 2026-10-11: `Non attribué` survived beside `Niet gegung`).
    " niet gegun",
    " niet toegewez",
    " niet toegeken",
    " niet gunning",
    " geen gunning",
    // Polish "annulled" in every form and its common typo (`Unieważnia postępowanie`,
    // `UNIEWAZNIONO POSTĘPOWANIE …`, `Uniewaniono postepowanie`).
    " uniewazni",
    " uniewanion",
];

/// Whole values that say the lot was not awarded, compared entire after the lot strip.
const VOID_WHOLE: [&str; 50] = [
    "desierto",
    "desierta",
    "deserto",
    "deserta",
    "desiertos",
    "lote desierto",
    "lotto deserto",
    "gara deserta",
    "non aggiudicato",
    "non aggiudicata",
    "lotto non aggiudicato",
    "not awarded",
    "lot not awarded",
    "no award",
    "no award made",
    "contract not awarded",
    "no contract awarded",
    "niet gegund",
    "uniewazniony",
    "uniewazniono",
    "postepowanie uniewaznione",
    "brak ofert",
    "brak oferty",
    "aucune offre",
    "aucune offre recue",
    "pas d attributaire",
    "pas d offre",
    "sans offre",
    "abandon",
    "aufgehoben",
    "nicht vergeben",
    // TED's own "not awarded", which r208 award notices print in the winner slot in all
    // two dozen languages at once (`Not awarded` / `Ikke tildelt` / … / `Ugovor nije
    // sklopljen`, notice 17674598): every one must read void for the party to be void.
    "ikke tildelt",
    "no ha sido adjudicado",
    "sopimusta ei tehty",
    "δεν ανατεθηκε",
    "nao adjudicado",
    "ej tilldelat",
    "nebyla zadana",
    "lepingut ei solmitud",
    "nem iteltek oda",
    "nesudaryta",
    "nav pieskirts",
    "ma ngħatax",
    "zamowienia nie udzielono",
    "nebola pridelena",
    "narocilo ni bilo oddano",
    "gan damhachtain",
    "не е възложена",
    "nu a fost atribuit",
    "ugovor nije sklopljen",
];

/// Issue 525: TED's OTHER winner-slot templates for a contract or lot not awarded, from the
/// copies of the parties the fold kept (dry 2351's 37 orgs, re-listed after every drain; public
/// `/content`, 2026-10-12). A 2-copy party drops only when both copies read void and a 24-copy one
/// on half, so each template is carried in every language its notices print.
///
/// EXACT values: the entire fold, or the fold before a bare lot qualifier
/// ([`bare_lot_qualifier`]: `(lot 3)`) — never a reason head nor a prefix, as [`VOID_WHOLE`]'s
/// are ([`reason_head`], [`void_prefix`]). TED prints its templates exactly; a name that opens
/// with one and goes on is national free text, which these were never measured against (510's
/// admission rule: a prefix census), and the award vocabulary that would keep it a name
/// ([`names_an_award`]) is French, Spanish, Italian, English and Dutch only — `Kontrakten ikke
/// tildelt, men givet til Hansen Byg` and `Los wurde nicht vergeben e.K.` are names (the review).
const VOID_TEMPLATES: [&str; 191] = [
    // `Contract not awarded.` / `Marché non attribué.` (16893864 ORG-2..5: DA, DE, ES, EL, IT, PT,
    // SV, CS, HU, LT, LV, MT, SK, SL, GA, BG, RO):
    "kontrakten ikke tildelt",
    "der auftrag wurde nicht vergeben",
    "el contrato no ha sido adjudicado",
    "η συμβαση δεν κατακυρωθηκε",
    "appalto non aggiudicato",
    "contrato nao adjudicado",
    "kontraktet har inte tilldelats",
    "zakazka nebyla zadana",
    "a szerzodest nem iteltek oda",
    "sutartis nepaskirta",
    "ligums nav pieskirts",
    "il kuntratt ma ngħatax",
    "obstaravanie nebolo pridelene",
    "pogodba ni dodeljena",
    "nior damhadh an conradh",
    "поръчката не беше възложена",
    "contract neatribuit",
    // …its later wording (16891918 ORG-2: DA, DE, ES, FR, EL, SV, LV, PL, SK, BG):
    "kontrakten blev ikke tildelt",
    "es wurde kein auftrag vergeben",
    "no se ha adjudicado ningun contrato",
    "le marche n a pas ete attribue",
    "η συμβαση δεν ανατεθηκε",
    "inget kontrakt har tilldelats",
    "liguma slegsanas tiesibas nav pieskirtas",
    "zamowienie nie zostalo udzielone",
    "zmluva nebola udelena",
    "поръчката не бе възложена",
    // `No contract awarded.` (16967678 ORG-4: DA, FR, IT, NL, PT, HU, GA, BG), and 17674598's
    // own LV copy as 16967678's ORG-2 spells it:
    "ingen kontrakt tildelt",
    "aucun marche n a ete attribue",
    "non e stato aggiudicato alcun contratto",
    "er is geen opdracht gegund",
    "nao foi adjudicado nenhum contrato",
    "szerzodes odaitelesere nem kerult sor",
    "nior damhadh aon chonradh",
    "поръчката не е възложена",
    "nav pieskirta",
    // `Lot not awarded` (12563312 ORG-23: DA, DE, ES, FI, EL, PT, SV, CS, ET, HU, LT, LV, MT, PL,
    // SK, SL, GA, BG, RO):
    "partiet blev ikke tildelt",
    "los wurde nicht vergeben",
    "lote no adjudicado",
    "erasta ei tehty sopimusta",
    "η παρτιδα δεν ανατεθηκε",
    "lote nao adjudicado",
    "delen har inte tilldelats",
    "polozka nebyla zadana",
    "osa kohta lepingut ei solmitud",
    "a tetel nem kerult odaitelesre",
    "dalis nepaskirta",
    "dala netika pieskirta",
    "il lott ma ngħatax",
    "na te czesc nie udzielono zamowienia",
    "cast nebola pridelena",
    "sklop ni bil oddan",
    "nior damhadh an beart",
    "партидата не е възложена",
    "lot neatribuit",
    // `Ce lot est infructueux.` / `This lot was unsuccessful.` (12466332 ORG-2: every language
    // but FR and ES, which the `infructu` stem reads):
    "dette parti er ikke blevet tildelt",
    "dieses los blieb erfolglos",
    "this lot was unsuccessful",
    "tama era oli tulokseton",
    "η διαδικασια αναθεσησ τησ παρτιδασ ειναι ανεπιτυχησ",
    "questo lotto e risultato infruttuoso",
    "het perceel bleef zonder resultaat",
    "este lote foi considerado infrutifero",
    "denna del har inte givit nagot resultat",
    "tato polozka byla neuspesna",
    "selle osa menetlus luhtus",
    "a tetel sikertelen volt",
    "si dalis buvo nesekminga",
    "si dala bija neveiksmiga",
    "dan il lott kien mingħajr suċċess",
    "zamowienie dla tej czesci uznano za nieskuteczne",
    "tato cast bola neuspesna",
    "ta sklop je bil neuspesen",
    "bhi an beart seo neamhrathuil",
    "тази партида беше неуспешна",
    "acest lot a fost nereusit",
    // `Non applicable — Lot non attribué` (18344075 ORG-4..7, 24 languages with HR): the slot's
    // "not applicable" and the lot's "not awarded" whole together, since the opener alone is no
    // void phrase (the FR and NL copies read void by their stems, the EN and PL ones `Blank`):
    "finder ikke anvendelse partiet blev ikke tildelt",
    "entfallt los nicht vergeben",
    "no procede lote no adjudicado",
    "ei sovelleta erasta ei tehty sopimusta",
    "ανευ αντικειμενου το τμημα δεν ανατεθηκε",
    "non applicabile lotto non aggiudicato",
    "nao aplicavel lote nao adjudicado",
    "ej tillampligt delen har inte tilldelats",
    "nepouzije se polozka nebyla zadana",
    "ei kohaldata osa lepingut ei solmitud",
    "nem alkalmazhato a tetel nem kerult odaitelesre",
    "netaikoma nes dalis nebuvo paskirta",
    "nav piemerojams liguma slegsanas tiesibas par so dalu nav pieskirtas",
    "mhux applikabbli il lott ma ngħatax",
    "neuplatnitelne cast nebola pridelena",
    "ni navedeno sklop ni bil oddan",
    "neamhbhainteach nior damhadh beart",
    "не е приложимо партидата не е възложена",
    "neaplicabil lot neatribuit",
    "nije primjenjivo grupa nije dodijeljena",
    // The review: the other kept parties of dry 2351, from diagnosis 2's copies (public `/content`,
    // 2026-10-12). `Non-attribution du marché` (17359294 ORG-2, org 3530158: DA, DE, EN, FI, EL, IT,
    // PT, SV, CS, LT, LV, PL, SK, SL, GA, BG, RO):
    "ikke tildeling af kontrakten",
    "nichtvergabe des auftrags",
    "non award of the contract",
    "sopimuksen tekematta jattaminen",
    "μη αναθεση τησ συμβασησ",
    "non aggiudicazione dell appalto",
    "nao adjudicacao do contrato",
    "utebliven kontraktstilldelning",
    "nezadani zakazky",
    "sutarties nepaskyrimas",
    "liguma slegsanas tiesibu nepieskirsana",
    "nieudzielenie zamowienia",
    "nepridelenie obstaravania",
    "neoddano narocilo",
    "neamhdhamhachtain an chonartha",
    "невъзлагане на поръчката",
    "neatribuirea contractului",
    // `non applicable: marché non attribué.` (18137696 ORG-2, org 6023876), the colon spelling of
    // the "not applicable — not awarded" compound with the contract's wording (DA … HR; the FR and NL
    // copies read void by their stems, the EN and PL ones `Blank`):
    "ikke relevant kontrakten blev ikke tildelt",
    "entfallt der auftrag wurde nicht vergeben",
    "no procede el contrato no se ha adjudicado",
    "ei sovelleta sopimusta ei tehty",
    "ανευ αντικειμενου η συμβαση δεν ανατεθηκε",
    "non applicabile appalto non aggiudicato",
    "nao aplicavel contrato nao adjudicado",
    "ej tillampligt kontraktet har inte tilldelats",
    "nepouzije se zakazka nebyla zadana",
    "ei kohaldata lepingut ei solmitud",
    "nem alkalmazhato a szerzodest nem iteltek oda",
    "netaikoma sutartis nepaskirta",
    "nav piemerojams liguma slegsanas tiesibas nav pieskirtas",
    "ma japplikax il kuntratt ma ngħatax",
    "neuplatnitelne zmluva nebola pridelena",
    "ni ustrezno narocilo ni bilo oddano",
    "neamhbhainteach nior damhadh an conradh",
    "не е приложимо поръчката не е възложена",
    "nu se aplica contract neatribuit",
    "nije primjenjivo ugovor nije sklopljen",
    // `Non applicable — Marché non attribué` (18549508 ORG-2, org 7400280), its dash spelling (the
    // copies it shares with the colon one listed there):
    "finder ikke anvendelse kontrakten blev ikke tildelt",
    "entfallt auftrag nicht vergeben",
    "no procede contrato no adjudicado",
    "ανευ αντικειμενου μη ανατεθεισα συμβαση",
    "netaikoma nes sutartis nebuvo paskirta",
    "nav liguma slegsanas tiesibas nav pieskirtas",
    "mhux applikabbli il kuntratt ma ngħatax",
    "neuplatnitelne zmluva nebola uzatvorena",
    "ni navedeno narocilo ni bilo oddano",
    "neamhbhainteach nior damhadh conradh",
    "neaplicabil contract neatribuit",
    // `Aucune offre reçue` (18236286 ORG-3, org 5286300; the FR copy is a `VOID_WHOLE` value):
    "der blev ikke modtaget nogen bud",
    "es ging kein angebot ein",
    "no tenders were received",
    "no se ha recibido ninguna oferta",
    "yhtaan tarjousta ei saatu",
    "δεν παρεληφθη καμια προσφορα",
    "non sono pervenute offerte",
    "er zijn geen inschrijvingen ontvangen",
    "nao foi recebida nenhuma proposta",
    "inga anbud mottogs",
    "nebyly obdrzeny zadne nabidky",
    "uhtegi pakkumist ei laekunud",
    "nem erkezett be ajanlat",
    "negauta jokiu pasiulymu",
    "netika sanemts neviens piedavajums",
    "ma waslu ebda offerti",
    "nie otrzymano ofert",
    "nebola dorucena ziadna ponuka",
    "prejeta ni bila nobena ponudba",
    "ni bhfuarthas aon tairiscinti",
    "не бяха получени оферти",
    "nu s a primit nicio oferta",
    "nije zaprimljena nijedna ponuda",
    // `Procédure infructueuse` (18720683 ORG-2, org 7985877; the FR and ES copies read by the
    // `infructu` stem):
    "resultatlos procedure",
    "verfahren erfolglos",
    "unsuccessful procedure",
    "tulokseton menettely",
    "ατελεσφορη διαδικασια",
    "procedura infruttuosa",
    "de procedure is zonder resultaat gebleven",
    "concurso declarado infrutifero",
    "resultatlost forfarande",
    "neuspesne rizeni",
    "menetlus luhtus",
    "sikertelen eljaras",
    "nesekminga procedura",
    "neveiksmiga procedura",
    "proċedura mingħajr suċċess",
    "postepowanie nieskuteczne",
    "neuspesne konanie",
    "neuspesen postopek",
    "nos imeachta neamhrathuil",
    "неуспешна процедура",
    "procedura nereusita",
    "neuspjesan postupak",
    // South Tyrol's bilingual `non aggiudicato` / `Zuschlag nicht vergeben` (12501289, 4416766):
    // two copies, so the DE one must read void by itself.
    "zuschlag nicht vergeben",
    // The text era's `NO AWARD WAS MADE` (3322691 ORG-2): `no award` and a lowercase `was` read
    // void, an uppercase `WAS` a company's next word, so case decided it. Whole, it does not.
    "no award was made",
];

/// Pointer and withheld-value stems (the rest of the 508 text list, and issue 511's
/// census, §4), padded like [`VOID_STEMS`]. ` not applicable` moved to [`BLANK_STEMS`]
/// (issue 511: it says the slot holds nothing).
const PLACEHOLDER_STEMS: [(&str, StemAt); 11] = [
    (" see section", StemAt::Start),
    (" voir autres informations", StemAt::Start),
    (" voir renseignements", StemAt::Start),
    (" ver informaci", StemAt::Start),
    (" perfil del contratante", StemAt::Boundary),
    (" perfil de contratante", StemAt::Boundary),
    (" would prejudice", StemAt::Boundary),
    // `Withheld Section94 Supplier` (one PPON on all 123 mentions), `Withheld for
    // Security Reasons`.
    (" withheld", StemAt::Start),
    (" see contracts finder", StemAt::Start),
    (" siehe oben", StemAt::Start),
    // The usual German and Austrian wording for a withheld winner (216 lot results read,
    // all `selec-w`, none with a co-winner): a stem, so `keine Angaben` and `keine Angabe
    // aus Gründen des Wettbewerbs` come with it.
    (" keine angabe", StemAt::Start),
];

/// Issue 511 unit 4 (residual dry 2273): the polite words a pointer may open with, on the fold —
/// `Please see section VI.4.2` (378 orgs / 411 mentions, review and appeal slots), `please see
/// Section VI.3.2`, `Please see Section VI.4.3)`. Admitted only as the name's first word (after
/// one optional lot designator) and only directly before an anchored [`POINTER_STEMS`] stem, so
/// `Acme Ltd – please see section VI.3` stays a name and `Please Hold Ltd` never reaches a stem.
const POINTER_LEAD_INS: [&str; 2] = [" please", " kindly"];

/// The pointer stems among [`PLACEHOLDER_STEMS`]: the ones a name may follow
/// ([`pointer_names`]: `siehe oben: NEB Betriebsgesellschaft mbH`).
const POINTER_STEMS: [&str; 8] = [
    " see section",
    " siehe oben",
    " see contracts finder",
    " voir autres informations",
    " voir renseignements",
    " ver informaci",
    " perfil del contratante",
    " perfil de contratante",
];

/// Issue 511: stems that say the slot holds nothing, every one anchored ([`StemAt::Start`]):
/// `Nie dotyczy - brak`, `Lot 3: not applicable`, `Niet van toepassing (perceel 2)`.
const BLANK_STEMS: [&str; 3] = [" nie dotyczy", " not applicable", " niet van toepassing"];

/// Placeholder whole values: a winner exists and its name is missing, pending, withheld,
/// elsewhere or plural. Classed by what each says in a winner slot (issue 511 §1): `tbc`,
/// `unknown` and the other "name missing" words moved here from [`BLANK_WHOLE`] (66
/// winner/bidder rows between them, none with an identifier).
const PLACEHOLDER_WHOLE: [&str; 74] = [
    // The name is missing or pending.
    "k a",
    "ka",
    "tbc",
    "tba",
    "to be confirmed",
    "not provided",
    "not specified",
    "not available",
    "unknown",
    "not known",
    "nicht bekannt",
    "niet bekend",
    "non connu",
    "non noto",
    "desconocido",
    "non renseigne",
    "brak danych",
    // A pointer; the DE hub `s.o.` is *siehe oben*.
    "s o",
    "see below",
    "please see below",
    "see attached",
    "see attached list",
    "see list below",
    "see additional information",
    "see vi 3",
    "siehe unten",
    "siehe vergabeunterlagen",
    // Withheld.
    "confidential",
    "redacted",
    "commercially sensitive",
    "not published",
    "not disclosed",
    "non communique",
    // Issue 511 unit 4 (residual dry 2273): the UK's withheld-supplier sentences, whole — 88
    // `Tenderer`/winner mentions on the first, 59 + 20 on the sensitive-contract pair.
    "information withheld for security reasons",
    "due to the sensitive nature of this contract the supplier name will be withheld",
    "due to the sensitive nature of this contract the supplier name will be withheld from publication",
    "due to the sensitive nature of this contract the supplier name is withheld from publication",
    // The s.94 sentences on the withheld-supplier PPON `PBXY-3232-WNHZ` (org 30914328, read
    // 2026-10-12): the one PPON stands for unrelated suppliers, so it names none of them.
    "information withheld due to security",
    "this information has been omitted from this notice in accordance with s 94 procurement act 2023",
    // Issue 511 unit 6 item 2 (the units 4+5 reads, the plan's `long` list): the UK sentences
    // that say the award or its winners are elsewhere or withheld, whole — each name exactly as
    // published, folded. Hertfordshire's award notice sentence WITH the council's name (`… for
    // further details. Hertfordshire County Council`, 17 mentions) is a different fold and stays a
    // name; these are its bare spellings (10804641, 7610087).
    "this is an award notice and this opportunity is now closed please see section v award of contract for further details",
    "the name s of the winning tenderer s are being withheld for security reasons however these will be made available to the commission on request",
    "the name s of the winning tenderers are being withheld for security reasons however these will be made available to the commission on request",
    "a full list of successful suppliers will be made available on contracts finder please see section vi 3 additional information",
    // Plural: the award went to several, named nowhere.
    "various",
    "various suppliers",
    "various contractors",
    "various providers",
    "various companies",
    "various organisations",
    "various see below",
    "multiple",
    "multiple suppliers",
    "multiple providers",
    "multiple contractors",
    "varios",
    "varias",
    "varios adjudicatarios",
    "varios licitadores",
    "varios proveedores",
    "varios contratistas",
    "varias empresas",
    "diversos",
    "diverse",
    "diverse firmen",
    "diverse bieter",
    "diverse ditte",
    "diversi",
    "divers",
    "verschiedene",
    "verschiedene firmen",
    "mehrere",
    "mehrere bieter",
    "ditte varie",
    "varie",
];

/// Issue 511: whole values that say the slot holds nothing — "not applicable", "none", a
/// filler. The "nobody" words (`ingen`, `neant`, `geen`, `aucun`) stay here, not in
/// [`VOID_WHOLE`]: they fill review and mediation slots as often as winner slots, and
/// promoting one needs 510's per-word value/date table (511 §8.7). `n a`, `na`, `nc`, `n d`
/// and `none specified` are ambiguous in a winner slot and stay here, asserting nothing.
const BLANK_WHOLE: [&str; 59] = [
    "n a",
    "no procede",
    "x",
    "xx",
    "xxx",
    "entfallt",
    "entfaellt",
    "neant",
    "keine",
    "nicht vorhanden",
    "nicht zutreffend",
    "nicht anwendbar",
    "nicht relevant",
    "ohne",
    "nein",
    "na",
    "none",
    "none specified",
    "not required",
    "not relevant",
    "n v t",
    "nvt",
    "geen",
    "brak",
    "n d",
    "sans objet",
    "so",
    "sans",
    "non",
    "nc",
    "n c",
    "aucun",
    "aucune",
    "non concerne",
    "non applicable",
    "nessuno",
    "nessuna",
    "non previsto",
    "non applicabile",
    "ninguno",
    "ninguna",
    "no aplica",
    "no hay",
    "no",
    "null",
    "ikke relevant",
    "ikke aktuelt",
    "ej aktuellt",
    "nao aplicavel",
    "nao se aplica",
    "δεν εφαρμοζεται",
    "δεν απαιτειται",
    "nu este cazul",
    "nem relevans",
    "neni",
    "ei ole",
    // Issue 511 unit 4 (residual dry 2273): UK review and `Lot-ReviewOrg` slots, 15 + 14 + 1
    // mentions — "not applicable", so the slot holds nothing.
    "this is not applicable in the uk",
    "this is not applicable to the uk",
    "this is not applicable in uk",
];

/// Issue 511 unit 4 (residual dry 2273): the German "no information is given here" a winner
/// slot closes on, padded like a stem and matched at the END of the whole value — `unter
/// Bezugnahme auf § 39 Abs 6 Nr. 4 VgV erfolgt hier keine Angabe` (19 mentions), `Unter Berufung
/// auf Art. 24 Abs. 2 erfolgt keine Angabe` (13), `1 VOL/A Abschnitt 4 SKR erfolgt hier keine
/// Angabe`. A withheld winner, so `Placeholder`; read only after a legal reference
/// ([`withheld_by_reference`]).
const NO_INFORMATION_TAILS: [&str; 4] =
    [" erfolgt hier keine angabe", " erfolgt keine angabe", " erfolgt hier keine angaben", " erfolgt keine angaben"];

/// The folded words a legal reference before a [`NO_INFORMATION_TAILS`] phrase is written with
/// (`unter Bezugnahme auf`, `Unter Berufung auf Artikel 24 Absatz 2 der Richtlinie 93/38 EWG`, `§ 11
/// SKR Abs. 2 (VOL/A-SKR)`, `gem. § 39 VgV i. V. m.`); any word with a digit counts too.
const REFERENCE_HEAD_WORDS: [&str; 36] = [
    "unter", "bezugnahme", "berufung", "auf", "gemass", "gem", "nach", "i", "v", "m", "art", "artikel", "abs",
    "absatz", "satz", "nr", "ziff", "ziffer", "buchst", "lit", "der", "des", "die", "richtlinie", "ewg", "eg",
    "vgv", "vol", "vob", "a", "skr", "sektvo", "konzvgv", "gwb", "abschnitt", "und",
];

/// Issue 511 guard 2: the short whole values with measured real-name collisions, `Blank`
/// only in the listed notice languages (the fold's 639-2/T tags,
/// [`crate::project::original_lang`]) and only in a plain case ([`plain_case`]). An unknown
/// language never matches, so the party is kept: 1188404 `Ingen` FR (134 mentions, likely a
/// French firm), the two `InGen` orgs (by case), `ND` HR (by language).
const SCOPED_WHOLE: [(&str, &[&str]); 3] = [
    ("ingen", &["DAN", "SWE", "NOR", "NOB", "NNO"]),
    ("inget", &["DAN", "SWE", "NOR", "NOB", "NNO"]),
    ("nd", &["POL", "ITA"]),
];

/// The words that open a trailing lot qualifier ([`strip_lot_qualifier`]).
const LOT_WORDS: [&str; 6] = ["lot", "lots", "lote", "lotes", "lotto", "lotti"];

/// Lot designators on the fold, for [`several_lots`]: [`LOT_WORDS`], the Spanish sub-lot
/// (`sous-lot` folds to `sous lot`, so `lot` covers it), and the other words a per-lot
/// summary numbers its parts with (`Tranche 1 : X. Tranche 2 : infructueux`, `Partie`,
/// `Partida`). Not `marché`: it is numbered by its contract reference and its duration
/// (`marché 2012 02 1 0014`, `durée du marché: 5 ans`) inside one lot's void sentence.
const LOT_DESIGNATORS: [&str; 20] = [
    "lot", "lots", "lote", "lotes", "lotto", "lotti", "sublote", "sublotes", "tranche", "partie", "partida", "secteur",
    "secteurs", "zone", "zones", "rang", "perceel", "percelen", "partij", "partijen",
];

/// Street words that, beside a five-digit postcode, make an address: a void statement
/// carries none, so the name holds a party (`BGE Guyane 16 rue Lt Becker 97300 Cayenne /
/// La consultation a été déclarée sans suite pour la part non couverte des besoins`).
const STREET_WORDS: [&str; 18] = [
    "rue", "avenue", "av", "bd", "boulevard", "chemin", "quartier", "zi", "za", "zac", "route", "allee", "impasse",
    "place", "bat", "rocade", "cedex", "bp",
];

/// Folded phrases that name an award beside a void lot: a partial void (`sans suite pour la
/// part non couverte des besoins`, `tous les autres lots … infructueux`), a negotiated award
/// after a void call (`marché négocié après appel d'offres infructueux : 5 Mepy Système`),
/// and the award section's own heading spilled into the slot.
const AWARD_PHRASES: [&str; 2] = [" non couvert", " autres lots"];

/// The award section's heading spilled into the slot: what follows it is the supplier's name
/// when anything follows (`Lot 3 sans suite. V.1) Award and contract value V.1.1) Name and
/// address of successful supplier` alone is a void lot).
const SUPPLIER_HEADINGS: [&str; 2] = [" successful supplier", " nom et adresse du titulaire"];

/// A void call that the award followed (`marché négocié après appel d'offres infructueux : 5
/// Mepy Système`): when every void stem sits inside one of these, the void is the earlier
/// procedure's and the slot names the negotiated award ([`only_an_earlier_void`]).
const EARLIER_VOID: [&str; 7] = [
    " apres appel d offres infructu",
    " apres un appel d offres infructu",
    " apres l appel d offres infructu",
    " apres ao infructu",
    " suite a appel d offres infructu",
    " suite a un appel d offres infructu",
    " suite a l appel d offres infructu",
];

/// Number marks between a lot designator and its number (`lot nº 3`, `lot n° 3`, `lote n.º 3`
/// — which folds to `n º 3` — and `lote núm. 3`). Up to two are skipped.
const NUMBER_MARKS: [&str; 9] = ["n", "nº", "no", "nr", "num", "numero", "nos", "º", "o"];

/// Roman lot numbers (`Lote I: X. Lote II: declarado desierto`), on the fold.
const ROMAN: [&str; 20] = [
    "i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x", "xi", "xii", "xiii", "xiv", "xv", "xvi", "xvii",
    "xviii", "xix", "xx",
];

/// Folded words that name an awardee: a void phrase beside one of them, not negated, is a
/// summary that names an award (`GRPT vallée sas (mandataire), … suite à procédure
/// négociée après AO infructueux`).
const AWARD_NOUNS: [&str; 26] = [
    "mandataire",
    "titulaire",
    "titulaires",
    "attributaire",
    "attributaires",
    "attribution",
    "retenu",
    "retenue",
    "adjudicatario",
    "adjudicataria",
    "adjudicatarios",
    "adjudicado",
    "adjudicada",
    "adjudicados",
    "adjudicadas",
    "aggiudicatario",
    "aggiudicataria",
    "aggiudicatari",
    "aggiudicato",
    "aggiudicata",
    "aggiudicati",
    "attribuito",
    "attribuita",
    "attribuiti",
    "awarded",
    "gegund",
];

/// Issue 520 (D3): the [`AWARD_NOUNS`] that are participles. One names the party only before the
/// party's preposition ([`PARTY_PREPOSITIONS`]: `lote 1 adjudicado a Construcciones Pérez`, `lotto 1
/// aggiudicato alla ditta Rossi`) or as a label (`fournisseur retenu = zoll medical France`); `awarded
/// as part of a new project`, `wordt gegund op totaal der percelen` and `l'offre retenue` name nobody.
const AWARD_PARTICIPLES: [&str; 14] = [
    "retenu",
    "retenue",
    "adjudicado",
    "adjudicada",
    "adjudicados",
    "adjudicadas",
    "aggiudicato",
    "aggiudicata",
    "aggiudicati",
    "attribuito",
    "attribuita",
    "attribuiti",
    "awarded",
    "gegund",
];

/// Issue 520 (D3): the folded prepositions that introduce the party after an award participle, by
/// the participle's start (`adjudicado a`, `aggiudicato alla`, `all'impresa`, `awarded to`, `gegund
/// aan`). By language, since the review: `awarded as part of a new project` holds the Spanish `a`
/// four words on. `retenu` takes none; it names the party as a label or after a copula.
const PARTY_PREPOSITIONS: [(&str, &[&str]); 5] = [
    ("adjudicad", &["a", "al"]),
    ("aggiudicat", &["a", "al", "alla", "alle", "allo", "ai", "agli", "all"]),
    ("attribuit", &["a", "al", "alla", "alle", "allo", "ai", "agli", "all"]),
    ("awarded", &["to"]),
    ("gegund", &["aan"]),
];

/// Issue 520 (D3, the review): how many words may stand between an award participle and its
/// party's preposition, in one clause — a date or an adverb (`gegund op 12-03-2014 aan`, `aggiudicato
/// in data 12/03/2014 alla ditta`, `adjudicado posteriormente por procedimiento negociado a`).
const PARTICIPLE_REACH: usize = 6;

/// Issue 520 (D3, the review): the folded words a label may hold between its awardee noun and its
/// `:` — an entry number and its mark, and a short complement with no verb (`Titulaire du marché :`,
/// `titulaire n°1 :`, `Nom du titulaire / organisme :`, `Attributaire après négociation :`). Since the
/// second review this closed list binds a participle's label only; a noun's takes any word that is
/// no negation, copula or awardee word ([`labels`]).
const LABEL_COMPLEMENTS: [&str; 18] = [
    "du", "de", "des", "d", "la", "le", "l", "les", "marche", "lot", "lots", "contrat", "accord", "cadre", "organisme",
    "apres", "negociation", "n",
];

/// Issue 520 (D3, the review): the folded words for a company that an awardee noun names the party
/// with (`Resultó adjudicataria la empresa Construcciones Pérez`, `aggiudicataria la ditta Rossi`).
const COMPANY_NOUNS: [&str; 9] =
    ["empresa", "ditta", "impresa", "societa", "societe", "entreprise", "sociedad", "firma", "compagnia"];

/// Issue 520 (D3, the review): the folded copulas after which `retenu(e)` names its subject (`la
/// société Storz a été retenue`) and before which it names what follows (`l'entreprise retenue est
/// Dupont Frères`).
const COPULAS: [&str; 7] = ["ete", "est", "sont", "etait", "etaient", "fut", "furent"];

/// Issue 520 (D2c, D6b, the review): the folded words of an award already made. A relaunch still to
/// come ([`RELAUNCH_INTENT`]) governs nothing past one: `Après avoir décidé de relancer en procédure
/// négociée, le marché a été conclu avec la société Dupont`, `Décision de négocier : marché conclu
/// avec Dupont SARL`. `a été` / `ont été` count too ([`intent_spans`]).
const PAST_AWARDS: [&str; 10] = [
    "conclu", "conclue", "conclus", "notifie", "notifiee", "notifies", "attribue", "attribuee", "attribues", "attribuees",
];

/// Issue 520 (D2c, D6b, the second review): the folded participles that state an award made when no
/// future stands before them ([`intent_spans`]): `Décision de négocier : marché passé avec Dupont
/// SARL`, `marché signé avec`, `marché confié à`. Not [`PAST_AWARDS`]: `un marché négocié … sera passé
/// avec la STÉ France Télécom` is still to come.
const PAST_PARTICIPLES: [&str; 12] = [
    "passe", "passee", "passes", "passees", "signe", "signee", "signes", "signees", "confie", "confiee", "confies",
    "confiees",
];

/// Issue 520 (D2c, D4b, D6b): the folded words of a relaunch still to come, the infinitive and the
/// future (`Déclaré infructueux - à relancer en marché négocié … avec les sociétés ayant soumissionné`,
/// `Il a donc été proposé de négocier avec les sociétés candidates`, `un marché négocié … sera passé
/// avec la STÉ France Télécom`). A company named after one in its sentence is one the buyer may
/// negotiate with, not the one the lot went to; after the past participle it is (`relancé en
/// procédure négocié … avec la société B BRAUN`).
const RELAUNCH_INTENT: [&str; 3] = ["relancer", "negocier", "sera"];

/// Issue 520 (D4b): what opens a relaunch on the space-padded fold (`relancé`, `Recomposition d'un
/// lot`, `Ce lot a été décomposé en 2 nouveaux lots`, `lancer une nouvelle procédure`, `se reporter
/// au lot no9`). The lot designators after one, in its sentence, number the new procedure's lots,
/// not the parts of the void lot's summary.
const RELAUNCH_MARKERS: [&str; 6] =
    [" relanc", " recomposition", " decompos", " nouvelle procedure", " nouveaux lots", " se reporter"];

/// Issue 520 (D5): the relaunch notes a void lot's `;`-separated reason goes on to (the 510 list), and
/// the publication references that cite the relaunch (`annonce(s) n° 116`, `JOUE: n° 2014/S
/// 250-444751`, `BOAMP: B n° 250`), on the space-padded fold: a segment holding one names nobody.
const RELAUNCH_NOTES: [&str; 8] = [
    " relanc",
    " nouvelle consultation",
    " nueva licitaci",
    " nuova gara",
    " nuevo procedimiento",
    " annonce",
    " joue",
    " boamp",
];

/// Issue 520 (D5, D6a): the folded word stems of an offer or a bidder (`Aucune offre n'a été déposée`,
/// `oferta excluida`, `oferty odrzucone`, `l'unica offerta ricevuta`, `el licitador ha renunciado`,
/// `soumissionnaire`, `Wykonawca wykluczony`, `geen geldige inschrijvingen`), matched at a word's
/// start. A segment that speaks of one gives the reason the lot is void; a company named after one,
/// in a sentence that rejects it ([`OFFER_REJECTIONS`]), is the bidder.
const OFFER_WORDS: [&str; 7] = ["offre", "offert", "ofert", "licitador", "soumissionn", "wykonawc", "inschrijving"];

/// Issue 520 (D6a): the folded word stems that reject an offer (`l'offre du groupement auxi+/ sas
/// Viviani est inacceptable`, `Offres déclarées irrégulières`, `ha escluso dalla gara l'unica offerta …
/// UnipolSai S.p.A.`, `oferta excluida`, `écartée`), matched at a word's start.
const OFFER_REJECTIONS: [&str; 5] = ["inacceptab", "irreguli", "esclus", "excluid", "ecarte"];

/// Issue 520 (D5, the review): the folded word stems that say an offer failed or was missing, beside
/// [`OFFER_REJECTIONS`], matched at a word's start: a reason segment speaks of an offer AND of its
/// failure (`Aucune offre n'a été déposée`, `oferty odrzucone`, `nie wpłynęła żadna oferta`, `el
/// licitador ha renunciado`, `le nombre d'offres conformes n'étant pas suffisant`). An offer alone
/// names the winner in Polish (`wybrano ofertę firmy …`, `Wykonawca: Jan Kowalski`) and in French
/// (`offre de la société Dupont retenue`, `le seul soumissionnaire, Dupont Frères`).
const OFFER_FAILURES: [&str; 9] =
    ["odrzuc", "wyklucz", "brak", "zadn", "renunci", "retir", "insuffis", "irrecevab", "ongeldig"];

/// Issue 520 (D5, the review): the folded words that negate an offer in a reason segment — the
/// [`NEGATIONS`] that cannot be a number mark (`n°` folds to `n`, `nº` to `no`), and the Polish and
/// Dutch `nie`, `geen`.
const OFFER_NEGATIONS: [&str; 19] = [
    "non", "pas", "sans", "aucun", "aucune", "not", "niet", "sin", "ningun", "ninguno", "ninguna", "nessun", "nessuno",
    "nessuna", "alcun", "faute", "absence", "nie", "geen",
];

/// The words that negate an award word before it (`pas d'attributaire`, `no adjudicado`,
/// `ningún adjudicatario`, `nessun aggiudicatario`, `faute d'attributaire`).
const NEGATIONS: [&str; 20] = [
    "non", "pas", "sans", "aucun", "aucune", "no", "not", "niet", "sin", "ningun", "ninguno", "ninguna", "nessun",
    "nessuno", "nessuna", "alcun", "faute", "absence", "n", "ne",
];

/// Words skipped between an award word and its negation (`pas d'attributaire`, `no hay
/// adjudicatario`, `non è stato individuato alcun aggiudicatario` reads its `alcun`).
const NEGATION_FILLERS: [&str; 30] = [
    "d", "de", "du", "des", "l", "la", "le", "un", "une", "el", "del", "di", "hay", "ha", "sido", "a", "ete",
    // The noun between a negation and its participle (`No contract awarded`, `aucune offre
    // retenue`, `ningún contrato adjudicado`).
    "contract", "contrat", "contratto", "contrato", "offre", "offres", "oferta", "ofertas", "offerta", "offerte",
    "candidature", "candidat", "tender",
];

/// Issue 520 (D2a, the review): [`NEGATION_FILLERS`] without its nouns, the words skipped between
/// `attribué` and its negation (`Le marché n'a pas été attribué`). A noun there is what the
/// negation denies (`Pas d'offre … attribué à X`, `faute d'offre, attribué à`), not the award.
const CLAUSE_FILLERS: [&str; 17] =
    ["d", "de", "du", "des", "l", "la", "le", "un", "une", "el", "del", "di", "hay", "ha", "sido", "a", "ete"];

/// Company forms as published, compared on the raw token with its surrounding punctuation
/// trimmed and its dots removed ([`company_token`]): a void phrase beside one names the
/// company another lot went to. Case-sensitive, so the French possessive `sa` (`lors de sa
/// séance`, `et Sa périphérie`) is no `SA`; [`COMPANY_FORMS_DOTTED`] takes any case when
/// the token was written with dots (`s.r.l.`, `S.A.R.L`). Not the census's folded
/// `COMMERCIAL_FORMS`, which fold the possessive onto `SA`.
const COMPANY_TOKENS: [&str; 39] = [
    "SA", "SAS", "Sas", "sas", "SASU", "SAU", "SARL", "Sarl", "sarl", "EURL", "Eurl", "eurl", "SNC", "Sté", "STÉ",
    "Ets", "ETS", "GmbH", "GMBH", "SL", "SLU", "SpA", "SPA", "Srl", "SRL", "srl", "Ltd", "LTD", "Limited",
    "LIMITED", "Lda", "LDA", "SPRL", "Société", "SOCIÉTÉ", "Societe", "SOCIETE", "BV", "NV",
];

/// Company forms written with dots, any case once the dots are gone (`s.r.l.` → `srl`). Not
/// `Ste`/`STE` in either list: that is Sainte in a place name (`Ste Marie de Figaniella`).
const COMPANY_FORMS_DOTTED: [&str; 13] =
    ["sa", "sas", "sasu", "sau", "sarl", "srl", "spa", "sl", "slu", "ltd", "ets", "bv", "nv"];

/// The folded words after `SA` that make it the possessive (`LORS DE SA SÉANCE`).
const POSSESSIVE_NEXT: [&str; 8] =
    ["seance", "peripherie", "forme", "duree", "reunion", "decision", "deliberation", "session"];

/// A name as the lists read it: the role census's fold — [`crate::project::match_norm`]
/// (lowercase, every non-alphanumeric run one space), then Latin diacritics folded.
pub fn fold(name: &str) -> String {
    store::buyer_name_fold(&crate::project::match_norm(name))
}

/// What `name` is when it is not a name; `None` for a name. No notice language, so the
/// scoped words ([`SCOPED_WHOLE`]) never match: the text parser, the role census's
/// `buyer_equal` and the drain's org walk read it this way ([`not_a_name_in`]).
pub fn not_a_name(name: &str) -> Option<NotAName> {
    not_a_name_in(name, None)
}

/// [`not_a_name`] as the fold reads it, with the notice's original language (`lang`,
/// [`crate::project::original_lang`]'s 639-2/T tag) for the scoped words. Issue 511 §4:
/// 1. a non-empty name with no alphanumeric character, or only zeros (`.`, `-/-`, `0`,
///    `000`), is [`NotAName::Blank`] — before the fold, which is empty for punctuation;
///    other digit-only names (`1`, `360`, `24/7`) stay names;
/// 2. a void stem: [`NotAName::AwardSummary`] when the name also names an award, else
///    [`NotAName::VoidLot`] (510);
/// 3. issue 511 unit 5: a section-reference pointer or a reference-led form label, nothing but
///    closed-list words around form section references ([`section_reference_pointer`]: `siehe
///    VI.4.1`, `V.1) Award and contract value`), is [`NotAName::Placeholder`] — before the stems,
///    whose keep rule could only misread a word of it as a name; a list word where a body's name
///    stands (`See section VI.2: Avis`, `I.4) Avis`) is no pointer, and the stems read it;
/// 4. a placeholder stem — an anchored pointer stem may follow one polite first word
///    ([`POINTER_LEAD_INS`]: `Please see section VI.4.2`): [`NotAName::AwardSummary`] when a
///    pointer is followed by a name ([`pointer_names`], a form section's own title not counted:
///    [`FORM_SECTION_TITLES`]; a perfil stem's genitive owner included: [`perfil_owner`]; since
///    issue 511 unit 6 a name right after the references with no separator: `See section VI.4.3
///    Translink`), a
///    boundary stem follows one ([`named_before_stem`]) or the name has a summary's shape
///    ([`summary_shape`]), else [`NotAName::Placeholder`];
/// 5. a blank stem: [`NotAName::AwardSummary`] for a summary's shape or a `but` clause that
///    names a body ([`but_names`]), else [`NotAName::Blank`];
/// 6. the whole values: 510's void ones first (with its reason and prefix rules) and issue 525's
///    TED templates (exact, or with TED's own bracketed pointer: [`VOID_TEMPLATES`]), then
///    [`PLACEHOLDER_WHOLE`] (and a legal reference closing on `erfolgt hier keine Angabe`:
///    [`withheld_by_reference`]), [`BLANK_WHOLE`], and [`SCOPED_WHOLE`] in its languages and a
///    plain case — after a bare lot qualifier only; one that names a company or a list is
///    an [`NotAName::AwardSummary`] ([`bare_lot_qualifier`]).
pub fn not_a_name_in(name: &str, lang: Option<&str>) -> Option<NotAName> {
    classify(name, lang, true)
}

/// Issue 511 unit 5, the review: whether `name` may seed the fold's TED translation echo
/// ([`crate::project`]'s `translation_echoes`) — unit 4's seeds only, a bare placeholder
/// ([`names_after_stem`]) that the lexicon classes WITHOUT unit 5's section-reference pointers
/// ([`section_reference_pointer`]). TED's template always carries the English stem seed (`See
/// Section VI.2 'Additional information'.`), and since unit 5 its translations are classed by
/// themselves. A pointer only unit 5 classes would seed every language: a legacy bilingual award's
/// FI `Katso kohta VI.4.1` swept its SV `Se punkt VI.4.1 Marknadsdomstolen` (pointer-led, short, no
/// legal form, the same number) and dropped the review body, which no copy classed before unit 5.
pub fn echo_seed(name: &str) -> bool {
    classify(name, None, false) == Some(NotAName::Placeholder) && !names_after_stem(name)
}

/// [`not_a_name_in`], reading unit 5's section-reference pointers only when `references`
/// ([`echo_seed`] reads without them).
fn classify(name: &str, lang: Option<&str>, references: bool) -> Option<NotAName> {
    // Issue 527: TenderNed's award mark is no part of the name, so it is never read as the void verdict
    // `niet gegund` (`Facilicom Solutions (niet gegunde partij)` is a losing tenderer, not a void lot).
    if let Some((base, _)) = award_annotation(name) {
        return classify(base, lang, references);
    }
    if blank_raw(name) {
        return Some(NotAName::Blank);
    }
    let full = fold(name);
    if full.is_empty() {
        return None;
    }
    let padded = format!(" {full}");
    if VOID_STEMS.iter().any(|stem| padded.contains(stem)) {
        // "Lot déclaré infructueux … puis attribué à la Société X", "Lot 1) Sarl Bremond.
        // Lot 2) Infructueux": an award IS named beside the void lot. Reading it as no
        // award would contradict the publisher, so it is a summary (the fold keeps it).
        return Some(if names_an_award(name, &full) { NotAName::AwardSummary } else { NotAName::VoidLot });
    }
    // Issue 511 unit 5: every word closed-list and where a pointer's words stand, so nothing in it
    // names a body; read before the stems, whose keep rule would take a capitalised list word for
    // a name (`see section VI.3.2.below` read as an award summary in dry 2283). Where a body's name
    // would stand — after a separator past a reference, a lot-numbered winner slot — a list word
    // may be a company's (`See section VI.2: Avis`, `I.3) Point P`), so the grammar refuses it and
    // the stems read it as before unit 5 (the review, [`reference_shape`]).
    if references && section_reference_pointer(name) {
        return Some(NotAName::Placeholder);
    }
    let (head, hit) = placeholder_hit(&full, &padded);
    if let Some((stem, at, pos)) = hit {
        let pointer = POINTER_STEMS.contains(&stem) && pointer_names(name, &padded, stem, pos);
        // A boundary stem may close a name, so what precedes it is read too (the unit-2
        // review: `Ayuntamiento de Madrid (véase perfil del contratante)`).
        let named_before = at == StemAt::Boundary && named_before_stem(name, &padded, pos);
        return Some(if pointer || named_before || summary_shape(name, &full) {
            NotAName::AwardSummary
        } else {
            NotAName::Placeholder
        });
    }
    if let Some(stem) = BLANK_STEMS.iter().find(|stem| head.starts_with(*stem)) {
        let but = after_stem(name, &padded, stem, padded.len() - head.len()).is_some_and(but_names);
        return Some(if but || summary_shape(name, &full) { NotAName::AwardSummary } else { NotAName::Blank });
    }
    // Folded again only when a lot qualifier came off: the fold already drops leading and
    // trailing punctuation and space, so an unstripped name folds to `full`.
    let qualifier = lot_qualifier(name);
    let stripped = qualifier.map(|(head, _)| fold(head));
    let whole = stripped.as_deref().unwrap_or(&full);
    if VOID_WHOLE.contains(&whole) {
        return Some(NotAName::VoidLot);
    }
    // Issue 525: TED's other templates, exact — the whole fold, or before a bare lot qualifier
    // only; neither the reason nor the prefix rule below reads them ([`VOID_TEMPLATES`]).
    if VOID_TEMPLATES.contains(&whole) && qualifier.is_none_or(|(_, group)| bare_lot_qualifier(group)) {
        return Some(NotAName::VoidLot);
    }
    if template_pointed(name) {
        return (!names_an_award(name, &full)).then_some(NotAName::VoidLot);
    }
    // A void whole value followed by its reason (`Desierto (no ofertas)`, `Lotto deserto -
    // non aggiudicato per importo offerto superiore alla base d'asta`, `Niet gegund - perceel
    // teruggetrokken`) is void — unless the name also names an award, and then it is a name
    // (`DESIERTO, S.L.` is a company; the text parser must keep it).
    if reason_head(name).is_some_and(|head| VOID_WHOLE.contains(&fold(head).as_str())) || void_prefix(name) {
        return (!names_an_award(name, &full)).then_some(NotAName::VoidLot);
    }
    let scoped = SCOPED_WHOLE.iter().find(|(word, _)| *word == whole);
    let class = if PLACEHOLDER_WHOLE.contains(&whole) || withheld_by_reference(whole) {
        Some(NotAName::Placeholder)
    } else if BLANK_WHOLE.contains(&whole)
        || scoped.is_some_and(|(_, langs)| lang.is_some_and(|l| langs.contains(&l)) && plain_case(name))
    {
        Some(NotAName::Blank)
    } else {
        None
    };
    match qualifier {
        // Issue 511 (the unit-2 review): its whole values come off a BARE lot qualifier only
        // (`N/A (lot 2)`, `Néant (lots 2 et 4)`). A qualifier that says more is more text,
        // not a qualifier: a summary when it names a company or a list (`N/A (lot 2 attribué
        // à Dupont SARL)`, `Various (Lot 1: ACME Ltd, Lot 2: Beta Ltd)`), else the name read
        // whole, which no list holds. 510's void values above keep any lot qualifier.
        Some((_, group)) if class.is_some() && !bare_lot_qualifier(group) => {
            summary_shape(group, &fold(group)).then_some(NotAName::AwardSummary)
        }
        _ => class,
    }
}

/// The anchored head of a fold and the placeholder stem it holds, as [`not_a_name_in`] reads
/// them: `full` the fold, `padded` it with its leading space. The head is a suffix of
/// `padded` that keeps its boundary space (so nothing is built per name for it): the stems
/// open the name after one optional lot designator (issue 511's anchor: `Lot 3: not
/// applicable`) and, before an anchored pointer stem only, one polite first word (issue 511
/// unit 4: [`POINTER_LEAD_INS`], `Please see section VI.4.2`) — the stem's position, and the
/// keep rule after it, read as without it. The hit is (stem, anchor, byte position in
/// `padded`): a [`StemAt::Start`] stem opening the head, or a [`StemAt::Boundary`] stem
/// anywhere.
fn placeholder_hit<'p>(full: &str, padded: &'p str) -> (&'p str, Option<(&'static str, StemAt, usize)>) {
    let rest = after_lot_designator(full);
    let head = match rest.len() {
        0 => "",
        n if n == full.len() => padded,
        n => &padded[full.len() - n..],
    };
    let head = POINTER_LEAD_INS
        .iter()
        .find_map(|lead| head.strip_prefix(lead))
        .filter(|rest| {
            PLACEHOLDER_STEMS.iter().any(|(stem, at)| *at == StemAt::Start && POINTER_STEMS.contains(stem) && rest.starts_with(stem))
        })
        .unwrap_or(head);
    let hit = PLACEHOLDER_STEMS.iter().find_map(|(stem, at)| match at {
        StemAt::Start => head.starts_with(stem).then(|| (*stem, *at, padded.len() - head.len())),
        StemAt::Boundary => padded.find(stem).map(|pos| (*stem, *at, pos)),
    });
    (head, hit)
}

/// Issue 511 unit 4 (the review of item 1): whether the text after `name`'s placeholder stem
/// names something — the pointer keep rule's reading ([`pointer_names`]: the section references
/// stripped, an opening separator and the references after it stripped, the form section titles
/// blanked) WITHOUT its separator requirement. The fold's translation echo seeds only from a
/// placeholder this is false for ([`crate::project`]'s `translation_echoes`): `See Section VI.3
/// Kier Construction` and the residual's `Please see section VI.4.3 Translink` were
/// `Placeholder` (no separator, so the keep rule never read the tail) until issue 511 unit 6 item
/// 1 read a name right after the references as an award summary ([`opens_with_a_name`]); a tail
/// that names something later (`Please see section VI.3 for list of awarded Suppliers`) still
/// leaves a `Placeholder` that seeds nothing.
/// `See Section VI.2 'Additional information'.` (the 460) and `See section VI.3 (Additional
/// Information)` (the 144) leave only a blanked title, and seed. False for a name with no
/// placeholder stem (a whole value names nothing after it); true when the fold's words and the
/// raw name's runs do not line up, erring toward a name.
pub fn names_after_stem(name: &str) -> bool {
    let full = fold(name);
    if full.is_empty() {
        return false;
    }
    let padded = format!(" {full}");
    let Some((stem, _, pos)) = placeholder_hit(&full, &padded).1 else { return false };
    let Some(after) = after_stem(name, &padded, stem, pos) else { return true };
    let (rest, _) = strip_references(after);
    let rest = open_separator(rest).map_or(rest, |after| strip_references(after).0);
    names_something(&without_section_titles(rest))
}

/// Issue 511's raw-name arm: a non-empty name with no alphanumeric character (`.`, `-`,
/// `-/-`, `_`) or only zeros (`0` to `0000`). Any other digit is a name: `1` holds 176
/// internal-ojs winner rows on one org, `360` FR is six orgs with distinct national ids.
fn blank_raw(name: &str) -> bool {
    let t = name.trim();
    !t.is_empty() && (!t.chars().any(char::is_alphanumeric) || matches!(t, "0" | "00" | "000" | "0000"))
}

/// Whether `raw` has no lowercase letter directly followed by an uppercase one (issue 511
/// guard 2): `INGEN` and `Ingen` are plain, `InGen` is a brand.
pub fn plain_case(raw: &str) -> bool {
    !raw.chars().zip(raw.chars().skip(1)).any(|(a, b)| a.is_lowercase() && b.is_uppercase())
}

/// The scoped word ([`SCOPED_WHOLE`]) `name` folds to, whatever the language or case — the
/// drain's dry plan splits these hits by the org's country, a proxy for the notice language.
/// After a bare lot qualifier only ([`bare_lot_qualifier`]), as [`not_a_name_in`] reads it.
pub fn scoped_word(name: &str) -> Option<&'static str> {
    let full = fold(name);
    let stripped = lot_qualifier(name).filter(|(_, group)| bare_lot_qualifier(group)).map(|(head, _)| fold(head));
    let whole = stripped.as_deref().unwrap_or(&full);
    SCOPED_WHOLE.iter().find(|(word, _)| *word == whole).map(|(word, _)| *word)
}

/// Issue 511: whether `name`'s first word is a scoped word ([`SCOPED_WHOLE`]) — the
/// decider's cheap test before it reads the notice language for [`not_a_name_in`]. Every
/// name the scope can class opens with its word, whatever lot qualifier follows (`INGEN
/// (lots 1, 2, 3, 4, 5 og 6)`): the first word of the lot-stripped fold is the first
/// alphanumeric run, folded.
pub fn opens_with_scoped_word(name: &str) -> bool {
    let first = name.split(|c: char| !c.is_alphanumeric()).find(|w| !w.is_empty()).unwrap_or("");
    // The longest scoped word is five letters; a run past a few bytes cannot fold to one.
    if first.len() > 12 {
        return false;
    }
    let first = fold(first);
    SCOPED_WHOLE.iter().any(|(word, _)| *word == first)
}

/// The languages a scoped word ([`scoped_word`]) is `Blank` in.
pub fn scoped_languages(word: &str) -> &'static [&'static str] {
    SCOPED_WHOLE.iter().find(|(w, _)| *w == word).map(|(_, langs)| *langs).unwrap_or(&[])
}

/// The walk's net for issue 511's `refold-placeholder-names` (an injectable `fn(&str) ->
/// bool`, as [`mentions_void`] is 510's): the raw-name arm, any placeholder or blank stem
/// at ANY left word boundary, any whole value, scoped words included, unit 5's
/// section-reference pointers ([`section_reference_pointer`]), and every name the fold's
/// translation echo can sweep ([`echo_shaped`]) — before the anchor, the pointer keep rule,
/// [`summary_shape`] and the language scope. The job re-queues every hit [`not_a_name`] does
/// not read as an [`NotAName::AwardSummary`], so the fold decides the anchor, the scope and
/// the identifier guard, and its dry plan lists the hits the anchor refuses.
///
/// **Contract: a superset of the fold's drops.** Every name the fold drops a party by — one
/// [`not_a_name_in`] classes as `Placeholder` or `Blank` in any language, or one the echo sweeps —
/// is a hit, because a party's head name (`organizations.name`, the one the walk reads) can be
/// any of its copies. Pinned per name by `the_walks_net_holds_every_name_the_fold_drops` and per
/// dropped party by the projection's 511 harness (`placeholder_511::decided`). Issue 511 unit 5
/// narrowed unit 4's echo arm, which took any short name with a section number (`Referat
/// II.3`, `Stadtverwaltung Kaiserslautern - Stabstelle IV.1 Zentrale Vergabestelle`: dry
/// 2283's 25,333 orgs and 1,277 buyer rows), to what the echo can sweep since unit 5 — a
/// pointer-led copy ([`translation_shaped`]). Issue 511 unit 6 item 6 widens it to what the TED
/// template signal sweeps ([`template_shaped`]: a copy whose reference a locator or a title frames,
/// whatever its opener): 126 orgs and 1 buyer row more on dry 2283 (`u6-build/simulation.md`).
pub fn mentions_placeholder(name: &str) -> bool {
    // Issue 511 unit 4 (the review of item 1), narrowed by unit 5: every name the fold's
    // translation echo can sweep. A party's head name is its first-seen copy, the ORIGINAL
    // language's (`r209::parse` walks the primary original first), so a copy the echo sweeps
    // unclassified can head the party while the English placeholder that drops it is a later
    // name no walk reads.
    mentions_by_class(name) || echo_shaped(name)
}

/// [`mentions_placeholder`] without the echo's arm: every name the fold classes as a dropping
/// non-name by itself, in any language, and the hits the anchor, the keep rule, the summary
/// shape and the scope refuse.
fn mentions_by_class(name: &str) -> bool {
    if blank_raw(name) {
        return true;
    }
    let full = fold(name);
    if full.is_empty() {
        return false;
    }
    let padded = format!(" {full}");
    if PLACEHOLDER_STEMS.iter().any(|(stem, _)| padded.contains(stem)) || BLANK_STEMS.iter().any(|stem| padded.contains(stem)) {
        return true;
    }
    let stripped = strip_lot_qualifier(name).map(fold);
    let whole = stripped.as_deref().unwrap_or(&full);
    PLACEHOLDER_WHOLE.contains(&whole)
        || BLANK_WHOLE.contains(&whole)
        || SCOPED_WHOLE.iter().any(|(w, _)| *w == whole)
        // Issue 511 unit 4: the ` keine angabe` stem above already catches every such hit; named
        // here so the net lists every rule the fold drops by.
        || withheld_by_reference(whole)
        // Issue 511 unit 5: the section-reference pointers and reference-led form labels, which
        // the fold classes `Placeholder` before any stem (`siehe VI.4.1`, `V.1) Award and contract
        // value`, `Voir point VI.2) Autres informations.`).
        || section_reference_pointer(name)
}

/// Issue 511 unit 4, narrowed by unit 5: whether `raw` is what the fold's translation echo can
/// sweep as a translated pointer when its party's placeholder carries the same number — a form
/// section number ([`form_section_tokens`]) in a translated pointer's shape
/// ([`translation_shaped`]: pointer-led, short, no legal form). The same two tests the echo
/// applies (`translation_echoes`), so the walk's net arm holds every echo sweep by construction
/// ([`mentions_placeholder`]); since unit 5 it costs a refold only for a pointer-led name that
/// cites a section and still reads as a name (`Siehe Punkt VI.4.1) - VKS Wien`), and the fold
/// decides.
///
/// Issue 511 unit 6 item 6: widened to what the TED template signal sweeps ([`template_shaped`]:
/// short, no legal form, and pointer-led OR its reference framed by a locator before it or a title
/// after it), which holds every translated-pointer shape, so the arm holds both sweeps by
/// construction. A body's name beside a bare number (`Referat II.3`, `Region Hovedstaden VI.3`)
/// stays out, as since unit 5.
fn echo_shaped(raw: &str) -> bool {
    !form_section_tokens(raw).is_empty() && template_shaped(raw)
}

/// Issue 511 unit 6 item 6: the fewest language-labelled copies, each carrying the English seed's
/// section number, short and with no legal form ([`template_copy`]), that make a party's names
/// TED's multilingual pointer template (the fold's `template_echoes`): TED prints the pointer in
/// 23 or 24 languages, a bilingual award (FI/SV, FR/NL) in two.
pub const TEMPLATE_MIN_LANGUAGES: usize = 10;

/// Issue 511 unit 6 item 6: whether `name` may seed the TED template signal — a bare placeholder
/// with a form section number: the lexicon's `Placeholder`, unit 5's section-reference pointers
/// included (`Same as in point VI.4.1`, `Please refer to VI.4.1.`, `See Section VI.3 Additional
/// information`), naming nothing after a stem ([`names_after_stem`]). Wider than [`echo_seed`],
/// whose narrowing guards a bilingual award; the template signal asks for
/// [`TEMPLATE_MIN_LANGUAGES`] copies and an English seed instead.
pub fn template_seed(name: &str) -> bool {
    !form_section_tokens(name).is_empty() && classify(name, None, true) == Some(NotAName::Placeholder) && !names_after_stem(name)
}

/// Issue 511 unit 6 item 6: whether `raw` counts toward the TED template signal — at most
/// [`TRANSLATION_MAX_WORDS`] words and no legal form ([`names_a_legal_form`]). A real party's copy
/// that names its form (`Consultar o ponto VI.3 Acme Lda`) never counts and is never swept.
pub fn template_copy(raw: &str) -> bool {
    raw.split_whitespace().count() <= TRANSLATION_MAX_WORDS && !names_a_legal_form(raw)
}

/// Issue 511 unit 6 item 6: what the TED template signal sweeps once a party holds it — a
/// [`template_copy`] that opens with a pointer word ([`pointer_led`]) OR whatever its opener has a
/// section reference framed as the template frames it ([`reference_framed`]: a locator before it,
/// `Consultar o ponto VI.3`, `Glejte oddelek VI.3)`; a form title or a postposed locator after
/// it, `Lásd a VI.3. szakaszt`, `… VI.3 «Informação adicional»`). A body named beside a bare
/// number is no copy (`Referat II.3`, `Stadtverwaltung Kaiserslautern - Stabstelle IV.1 Zentrale
/// Vergabestelle`), so the walk's net arm ([`echo_shaped`]) stays narrow.
pub fn template_shaped(raw: &str) -> bool {
    template_copy(raw) && (pointer_led(raw) || reference_framed(raw))
}

/// Issue 511 unit 6 item 6: whether `raw`'s first form section reference ([`reference_pieces`]) sits
/// where a template copy's does, after an opener of a few words, framed as the template frames it:
/// - at most [`TEMPLATE_OPENER_WORDS`] words before it — the first any word, each later one
///   lowercase or a closed-list word but never a [`SECTION_POINTERS`] phrase — with no separator
///   ([`Pieces::separated`]) and no full stop between them, except after a one-word opener that
///   more words follow (`Zob. sekcja`). A body named before a pointer (`KIO – jak w pkt VI.3.1)`,
///   `Vergabekammer des Landes Berlin; siehe Punkt VI.4.1)`), a capitalised name (`LMU München,
///   Referat IV.1`) or a supplier before the award heading it spilled (`Transports Chalavan. V.1)
///   AWARD AND CONTRACT VALUE:`, `Bouygues. V.1) …`) is no copy;
/// - and a [`SECTION_LOCATORS`] phrase of two or more letters right before it (`ponto VI.3`,
///   `t-Taqsima VI.3`, not `P VI.3`), or a [`FORM_SECTION_TITLES`] phrase or a
///   [`POSTPOSED_LOCATORS`] word right after it (`VI.3) Nadere inlichtingen`, `VI.3. szakaszt`).
fn reference_framed(raw: &str) -> bool {
    let Some(pieces) = reference_pieces(raw) else { return false };
    let words = &pieces.words;
    let Some(k) = words.iter().position(Option::is_none) else { return false };
    if k > TEMPLATE_OPENER_WORDS {
        return false;
    }
    // Before the first reference the pieces are the raw name's alphanumeric runs, one for one.
    let runs = alphanumeric_runs(raw);
    let gap = |i: usize| &raw[runs[i - 1].1..runs[i].0];
    let opener = (1..=k).all(|i| !pieces.separated[i] && (!gap(i).contains('.') || (i == 1 && k >= 2)))
        && (1..k).all(|i| match reference_phrase(words, i) {
            Some((_, ReferencePart::Pointer)) => false,
            Some(_) => true,
            None => !raw[runs[i].0..runs[i].1].starts_with(char::is_uppercase),
        });
    let before = (1..=k.min(3)).any(|n| {
        reference_phrase(words, k - n) == Some((n, ReferencePart::Locator))
            && words[k - n..k].iter().flatten().map(|w| w.chars().count()).sum::<usize>() >= 2
    });
    let after = match reference_phrase(words, k + 1) {
        Some((_, ReferencePart::Title)) => true,
        Some((1, ReferencePart::Locator)) => words[k + 1].as_deref().is_some_and(|w| POSTPOSED_LOCATORS.contains(&w)),
        _ => false,
    };
    opener && (before || after)
}

/// Issue 511 unit 6 item 6: the most words before a template copy's reference ([`reference_framed`]):
/// TED's openers with their locator run to four (`A se vedea secțiunea VI.3`, `Véase el apartado
/// VI.3`, `Consultar o ponto VI.3`).
const TEMPLATE_OPENER_WORDS: usize = 4;

/// Issue 511 unit 6 item 4: whether the drain's dry plan lists `name` among the hits it reads for a
/// legal form (`legal_form`, the stand-in for a legal-form guard, 510 unit1-decision §5.3) — the
/// role census's case-blind reading ([`crate::project::role_census::has_commercial_form`]), less a
/// section-reference pointer ([`section_reference_pointer`]), whose `As`/`Same as` the census
/// reads as the Norwegian/Estonian `AS`: residual dry 2310's list held 268 orgs, its readers found
/// every placeholder among them such a pointer (`Same as VI.4.1`, `As in I.1 above`, `Address as
/// per I.1) above.`). A real company keeps its place (`Norconsult AS`, `Acme AS, see VI.3`).
pub fn plan_legal_form(name: &str) -> bool {
    crate::project::role_census::has_commercial_form(name) && !section_reference_pointer(name)
}

/// Issue 511 unit 4: whether a whole value (`whole`, the fold after a bare lot qualifier) is a
/// legal reference followed by a [`NO_INFORMATION_TAILS`] phrase — every word before the phrase a
/// [`REFERENCE_HEAD_WORDS`] word or one with a digit (`unter Bezugnahme auf § 39 Abs 6 Nr. 4 VgV
/// erfolgt hier keine Angabe`, `1 erfolgt hier keine Angabe`, the bare phrase). A name before the
/// phrase leaves the whole a name (`Stadtwerke Kiel – erfolgt keine Angabe`, `Müller GmbH
/// (erfolgt hier keine Angabe)`).
fn withheld_by_reference(whole: &str) -> bool {
    let padded = format!(" {whole}");
    NO_INFORMATION_TAILS.iter().find_map(|tail| padded.strip_suffix(tail)).is_some_and(|head| {
        head.split(' ')
            .filter(|w| !w.is_empty())
            .all(|w| REFERENCE_HEAD_WORDS.contains(&w) || w.bytes().any(|b| b.is_ascii_digit()))
    })
}

/// Issue 511 unit 4 item 1: the form section numbers `raw` carries, lowercased — a whitespace
/// token that, its surrounding punctuation off, is a [`ROMAN`] numeral and one or more dotted
/// digit groups (`VI.2`, `VI.4.2`, `II.2.3`; `VI.2)`, `VI.2.,`, `(IV.1.1)`). Not a bare `VI`,
/// not an arabic `6.3.2`, not a malformed `V1.3` or `VI3.2`. The fold's translation echo
/// compares them between one party's names ([`crate::project`]'s `translation_echoes`).
pub fn form_section_tokens(raw: &str) -> Vec<String> {
    raw.split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|t| {
            let mut parts = t.split('.');
            t.contains('.')
                && parts.next().is_some_and(|numeral| ROMAN.contains(&numeral.to_lowercase().as_str()))
                && parts.all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        })
        .map(str::to_lowercase)
        .collect()
}

/// A translated pointer's length cap, in whitespace words ([`translation_shaped`]): TED's
/// translations of `See Section VI.2 'Additional information'` run to seven
/// (`Lásd a VI.2., További információk című pontot.`).
const TRANSLATION_MAX_WORDS: usize = 12;

/// Issue 511 unit 4 item 1: whether `raw` has the shape of a translated pointer rather than a
/// party's name — at most [`TRANSLATION_MAX_WORDS`] words and no company legal form
/// ([`names_a_legal_form`]). `Müller Bau GmbH (siehe Abschnitt VI.3)` is a party that cites the
/// section, and keeps its party.
///
/// Issue 511 unit 5: and pointer-led — it opens, after an optional polite lead-in, with a
/// [`SECTION_POINTERS`] word in one of the languages the plan and TED's template hold (`Se`,
/// `siehe`, `Véase`, `Βλέπε`, `Lásd`, `Вж.`, `A se vedea`). Every copy of TED's 23-language pointer
/// opens with one (and, since the review, the reconstructed Croatian `Vidjeti`/`Vidi`), so the echo
/// still sweeps a template wording the lexicon misses (`Se punkt VI.2) Nye oplysninger`), while a
/// body's name with a section number beside a bare pointer no longer drops with it (`Region
/// Hovedstaden VI.3`, `Referat II.3`), and the walk's net arm for the echo ([`echo_shaped`]) stays
/// narrow. The echo is only as wide as [`SECTION_POINTERS`]: a copy whose opener the list misses
/// stays a name and keeps its party, which unit 4's echo swept.
pub fn translation_shaped(raw: &str) -> bool {
    raw.split_whitespace().count() <= TRANSLATION_MAX_WORDS && !names_a_legal_form(raw) && pointer_led(raw)
}

/// Whether `raw` carries a company legal form by any of the crate's readings: [`company_token`]
/// (`SA`, `GmbH`, `Ltd`, `s.r.l.`), [`polish_company_form`], the crosswalk's family tokens
/// ([`crate::crosswalk::legal_form_family`]: `AG`, `Oy`, `s.r.o.`, `Kft`) and
/// [`LEGAL_FORM_TOKENS`] (`mbH`, `plc`, `A/S`, `d.o.o.`). Errs toward a form: a hit keeps a party.
///
/// Issue 511 unit 5: the Norwegian/Estonian `AS` is no form when it is the English pointer word
/// of a section-reference pointer, every other word closed-list ([`reference_shape`]: `AS IN I.1
/// ABOVE`, `AS VI.4.1`); dry 2283's `legal_form` read list, a case-blind reading, held 518 such
/// pointers among its 665 orgs (`As in I.1 above`, `Same as VI.4.1`). A name beside it keeps the
/// form: `Firma AS`, `AS Tallinna Vesi`, `Acme AS VI.3`; `A/S` is never the pointer (`Hansen
/// A/S`).
fn names_a_legal_form(raw: &str) -> bool {
    legal_form_reading(raw, reference_pieces(raw).and_then(|pieces| reference_shape(&pieces)).is_some())
}

/// [`names_a_legal_form`], with the `AS` token read as the pointer word `as` when `pointer_as`.
fn legal_form_reading(raw: &str, pointer_as: bool) -> bool {
    company_token(raw)
        || polish_company_form(&format!(" {}", fold(raw)))
        || crate::crosswalk::legal_form_family(raw).is_some()
        || raw.split_whitespace().any(|t| {
            let t = t.trim_matches(|c: char| !c.is_alphanumeric() && c != '.');
            LEGAL_FORM_TOKENS.contains(&t) && !(pointer_as && t == "AS")
        })
}

/// The legal forms the crate's other readings leave out, as whole whitespace tokens with their
/// dots ([`names_a_legal_form`]): the German ones [`names_something`] reads, and the UK, US,
/// Nordic, Baltic, Slavic and German partnership forms the review of issue 511 unit 4 found
/// [`translation_shaped`] passing (`Acme plc VI.3`, `Acme A/S VI.3`, `Acme d.o.o. VI.3`).
const LEGAL_FORM_TOKENS: [&str; 20] = [
    "mbH", "e.V.", "e.V", "gGmbH", "plc", "Plc", "PLC", "LLP", "Inc", "Inc.", "INC", "KG", "AS", "A/S", "OÜ", "d.o.o.",
    "D.O.O.", "a.s.", "A.S.", "LLC",
];

/// Whether `name` is exactly an issue-511 lexicon entry: the raw-name arm, a whole value,
/// or a stem's own words and nothing else (`Siehe oben`). The drain's dry plan must read
/// every hit that is NOT (511 §8.2: stem hits with more text, mid-string hits the anchor
/// refuses, keep-rule and summary names). Compared on the FULL fold: a lot qualifier is
/// more text, so `N/A (lot 2)` (`Blank`) is read too — the unit-2 review found
/// placeholders whose qualifier named the suppliers.
pub fn is_exact_entry(name: &str) -> bool {
    if blank_raw(name) {
        return true;
    }
    let full = fold(name);
    let whole = full.as_str();
    PLACEHOLDER_WHOLE.contains(&whole)
        || BLANK_WHOLE.contains(&whole)
        || SCOPED_WHOLE.iter().any(|(w, _)| *w == whole)
        || PLACEHOLDER_STEMS.iter().map(|(stem, _)| *stem).chain(BLANK_STEMS).any(|stem| stem[1..] == full)
}

/// The fold after one optional leading lot designator — a [`LOT_DESIGNATORS`] word, up to
/// two [`NUMBER_MARKS`], and a number with its dotted parts (`lot 3 not applicable`, `lote n
/// 2 …`, `lot 6 01 01 …`) — or the whole fold when it opens with none. A suffix of `full`.
fn after_lot_designator(full: &str) -> &str {
    if !full.split(' ').next().is_some_and(|w| LOT_DESIGNATORS.contains(&w)) {
        return full;
    }
    let words: Vec<&str> = full.split(' ').collect();
    let mut j = 1;
    while j < words.len() && j <= 2 && NUMBER_MARKS.contains(&words[j]) {
        j += 1;
    }
    let numbered = words.get(j).is_some_and(|w| w.bytes().any(|b| b.is_ascii_digit()) || ROMAN.contains(w));
    if !numbered {
        return full;
    }
    j += 1;
    while j < words.len() && !words[j].is_empty() && words[j].bytes().all(|b| b.is_ascii_digit()) {
        j += 1;
    }
    // Every word is one alphanumeric run and one space apart, so the suffix starts after
    // the first `j` words and their spaces.
    let skipped: usize = words[..j].iter().map(|w| w.len() + 1).sum();
    full.get(skipped..).unwrap_or("")
}

/// Issue 511's pointer keep rule: whether the text after a pointer stem (`stem`, found at
/// byte `pos` of `padded`, the space-padded fold of `raw`) names something, so the name is
/// a pointer followed by the body it points at (`siehe oben: NEB Betriebsgesellschaft mbH`,
/// `siehe oben (Vergabekammer Südbayern)`, `Siehe oben – VKS`):
/// 1. the rest of `raw` after the stem and the word it ends in;
/// 2. its section references stripped ([`strip_references`]: `VI.4.1`, `VI.3)`, `Ziffer`,
///    `unter`, `below`, one `Lot 2`);
/// 3. when it then opens with a separator (`:`, ` - `, `–`, `—`, `(`), stripped again — a
///    reference's own trailing `:` counts (`See section VI.4.1: High Court of Justice`);
/// 4. what remains names something ([`names_something`]).
///
/// Without a separator nothing follows the pointer but its own words (`See Section VI.3)
/// Additional information`, `See Contracts Finder Notice for full supplier list`) — except
/// a `see section` with no reference at all, which names its body directly (`see section
/// Royal Courts of Justice`), a perfil stem that opens the name, whose genitive names
/// the profile's owner ([`perfil_owner`]: `Perfil de contratante del Gobierno de Aragón`), and
/// since issue 511 unit 6 item 1 a name right after the stripped references ([`opens_with_a_name`]:
/// `Please see section VI.4.3 Translink`, `See Section VI.3 Kier Construction`).
fn pointer_names(raw: &str, padded: &str, stem: &str, pos: usize) -> bool {
    let Some(after) = after_stem(raw, padded, stem, pos) else {
        return false;
    };
    let (rest, referenced) = strip_references(after);
    let rest = match open_separator(rest) {
        Some(after) => strip_references(after).0,
        None if stem == " see section" && !referenced => rest,
        // Issue 511 unit 6 item 1 (the units 4+5 reads: `Please see section VI.4.3 Translink`, a
        // review body, the one real party the combined wet dropped): no separator, but the text
        // after the references opens with a name.
        None if referenced => return opens_with_a_name(&without_section_titles(rest)),
        // Issue 511 unit 3 read: only when nothing precedes the stem — `Ver perfil del
        // contratante de la Junta de Andalucía` points a winner slot at the buyer's
        // profile (the 2010 print's junk, `text::parse`), and a name before the stem is
        // `named_before_stem`'s to read.
        None if PERFIL_STEMS.contains(&stem) && pos == 0 => return perfil_owner(after),
        None => return false,
    };
    // Issue 511 unit 4: a form section's own title names the section, not a body.
    names_something(&without_section_titles(rest))
}

/// Issue 511 unit 6 item 1: whether `s`, the text after a pointer's section references with the form
/// section titles blanked ([`pointer_names`]), opens with a name — its first word, past the words
/// of a position ([`REFERENCE_WORDS`], [`SECTION_POSITIONS`]: `below`), starts with a capital, has
/// two or more letters, is no malformed reference ([`roman_with_digits`]: `V1`) and no closed-list
/// word of a section-reference pointer ([`REFERENCE_LISTS`]: `For`, `Section`, `Notice`, `Avis`,
/// `Point`). `Translink`, `Kier Construction` and `Acme plc` open with one; `for list of awarded
/// Suppliers`, `of the Project Information Memorandum` and a blanked `Additional information` do
/// not. Without a separator, a body named after a joiner (`The High Court`) or with a list word
/// (`Point P`) stays unread, as before unit 6: erring toward the pointer, which the separator rule
/// reads ([`names_something`]) when the publisher writes one.
///
/// Issue 511 unit 6, the build review: a capitalised word is no name by itself. The first word also
/// opens no closed-list phrase of any length ([`reference_phrase`]: `Contract Award Notice`), no
/// [`TAIL_NON_NAMES`] phrase (`Suppliers`, `Lots 1-5`, `Annex`, `Name`, `Contracts Finder`, `Es
/// handelt sich …`) unless the tail names a legal form (`NIL d.o.o.`), and is no bare Roman
/// numeral (`siehe oben Ziff. VI.4.1Ort`, whose glued `VI.4.1Ort` no reference strip splits); and
/// the tail from it is no non-name the lexicon classes by itself (`See section VI.3 Not
/// applicable`, `… TBC`, `… Withheld`). The form labels a pointer names its target with are
/// [`FORM_SECTION_TITLES`] (`Name and addresses`, `Mediation body`, `Zuständige Stelle`), blanked
/// before this reads. Before the review all of these read as a body, an award summary the fold
/// never drops — three names unit 3's wet dropped among them (`u3/plan-2265.json`).
fn opens_with_a_name(s: &str) -> bool {
    let runs = alphanumeric_runs(s);
    let words: Vec<Option<String>> = runs.iter().map(|&(start, end)| Some(fold(&s[start..end]))).collect();
    let mut i = 0;
    while i < words.len() {
        let folded = words[i].as_deref().unwrap_or_default();
        if REFERENCE_WORDS.contains(&folded) {
            i += 1;
            continue;
        }
        if let Some((n, ReferencePart::Position)) = reference_phrase(&words, i) {
            i += n;
            continue;
        }
        let (start, end) = runs[i];
        let (word, tail) = (&s[start..end], &s[start..]);
        return word.chars().next().is_some_and(char::is_uppercase)
            && word.chars().filter(|c| c.is_alphabetic()).count() >= 2
            && !roman_with_digits(word)
            && !(ROMAN.contains(&folded) && word.bytes().all(|b| matches!(b, b'I' | b'V' | b'X')))
            && reference_phrase(&words, i).is_none()
            && (!tail_non_name(&words, i) || names_a_legal_form(tail))
            && !matches!(not_a_name(tail), Some(NotAName::Placeholder | NotAName::Blank | NotAName::VoidLot));
    }
    false
}

/// Issue 511 unit 6, the build review: the words that may follow a pointer's reference with no
/// separator and name no body ([`opens_with_a_name`]), on the fold — the generic nouns a pointer
/// names its target with that are no form's title (`See section VI.3 Suppliers`, `See Section VI.3
/// Lots 1-5`, `… Annex`, `siehe oben I.1 Name`, `See section VI.3 Contracts Finder`), the deferred
/// `nil` (`… Nil`; `NIL d.o.o.` names its form), and a sentence's opening pronoun (`Siehe oben
/// Abschnitt II.1.4). Es handelt sich um eine freiwillige EU-Bekanntmachung …`, 6 mentions dropped
/// by unit 3's wet). Read for the tail only: none is a closed-list word of the section-reference
/// grammar ([`REFERENCE_LISTS`]), so a name opening with one stays out of the pointer class.
const TAIL_NON_NAMES: [&str; 13] =
    ["supplier", "suppliers", "annex", "annexe", "anhang", "name", "names", "contracts finder", "nil", "es", "this", "these", "there"];

/// Whether a [`TAIL_NON_NAMES`] phrase or a [`LOT_WORDS`] word opens `words` (folded) at `i`.
fn tail_non_name(words: &[Option<String>], i: usize) -> bool {
    let at = |t: usize| words.get(i + t).and_then(Option::as_deref);
    LOT_WORDS.iter().any(|w| at(0) == Some(*w))
        || TAIL_NON_NAMES.iter().any(|phrase| phrase.split(' ').enumerate().all(|(t, w)| at(t) == Some(w)))
}

/// Issue 511 unit 6, the build review: `raw` after its LAST form section reference
/// ([`reference_pieces`]), `None` without one.
fn after_last_reference(raw: &str) -> Option<&str> {
    let pieces = reference_pieces(raw)?;
    let last = pieces.words.iter().rposition(Option::is_none)?;
    Some(&raw[pieces.ends[last]..])
}

/// Issue 511 unit 6, the build review: whether the text after `raw`'s last form section reference,
/// its form section titles blanked, opens with a name ([`opens_with_a_name`]) — a body named right
/// after a pointer's reference, in a copy no class reads (`Glejte oddelek VI.4.1 Državna revizijska
/// komisija`, `Se référer au point VI.4.1 Conseil d'État`, `As set out at VI.4.1 Office of
/// Government Procurement`). The fold's two sweeps (`translation_echoes`, `template_echoes`) leave
/// such a copy a name beside a bare seed ([`titled_after_reference`]). Every copy of 12516947's and
/// 19530163's pointer is false: a section-reference pointer by itself, its tail closed-list words.
pub fn names_after_reference(raw: &str) -> bool {
    after_last_reference(raw).is_some_and(|tail| opens_with_a_name(&without_section_titles(tail)))
}

/// Issue 511 unit 6, the build review: whether a [`FORM_SECTION_TITLES`] phrase follows `raw`'s
/// last form section reference — a seed that names the section pointed at (`See Section VI.2
/// 'Additional information'.`, `See Section VI.3 Additional information`), whose copies carry that
/// title translated, in a wording the lists may miss (`Se punkt VI.2) Nye oplysninger`, unit 4's
/// echo case). Beside such a seed a capitalised tail is read as the title and swept; beside a bare
/// one (`See section VI.4.1`, `Siehe oben VI.4.1`, `Same as in point VI.4.1`) a copy's tail can only
/// be a body ([`names_after_reference`]), and the copy is left a name.
pub fn titled_after_reference(raw: &str) -> bool {
    after_last_reference(raw).is_some_and(|tail| matches!(without_section_titles(tail), std::borrow::Cow::Owned(_)))
}

/// Issue 511 unit 4 item 2 (residual dry 2273): the titles TED's standard forms give their
/// sections, on the fold — a closed list, not a rule about capitals. After a pointer one names
/// the section pointed at, not a body, so [`pointer_names`] reads past it: `See section VI.3
/// (Additional Information)` (144 winner mentions read as summaries with its spellings), `See
/// Section VI: Complementary Information`, `See Section II.2.3) Place of performance`, `Please see
/// section II.1.5 Short Description`. Taken from the plan's `non_exact` list and the 2008 and
/// 2014 form labels.
///
/// Issue 511 unit 5 (dry 2283): the titles and section I's labels that follow a reference in the
/// plan's `name` folds, in every language there (`V.1) Award and contract value` 2,629 winner orgs,
/// `siehe Kontaktstelle unter Punkt I.1` 249, `Vergabestelle (siehe I.1)` 67, `Zie VI.2 nadere
/// inlichtingen` 66, `Vgl. VI.3.1) Zuständige Stelle für Rechtsbehelfs-/Nachprüfungsverfahren`
/// 29), and the "Additional information" title of TED's 23-language pointer (notice 12516947's
/// ORG-2, `TED_VI2_POINTER` in the projection's tests). A section-reference pointer may close on
/// them ([`reference_shape`]), and the keep rule reads past them as before.
const FORM_SECTION_TITLES: [&str; 107] = [
    // Section VI: complementary information; VI.2 / VI.3 additional information; VI.3 / VI.4 the
    // review procedures (2008 and 2014 numbering).
    "additional information",
    "complementary information",
    "procedures for review",
    "review procedures",
    "review procedure",
    "review body",
    "procedures for appeal",
    "body responsible for appeal procedures",
    "body responsible for mediation procedures",
    "lodging of appeals",
    // Section II: the object.
    "object of the contract",
    "scope of the procurement",
    "short description",
    "description of the procurement",
    "place of performance",
    "main place of delivery",
    "main place of performance",
    "award criteria",
    "information about lots",
    "information about variants",
    "information about options",
    "information about european union funds",
    // Sections I, III, IV and V.
    "contracting authority",
    "contracting entity",
    "conditions for participation",
    "administrative information",
    "award of contract",
    "information about tenders",
    // Issue 511 unit 5, English: the award section's heading spilled into the winner slot alone
    // (`V.1) AWARD AND CONTRACT VALUE:` 2,629 orgs, `V.1.4) Information on value of contract: V.1)
    // AWARD AND CONTRACT VALUE:` 286), and the other titles after a reference.
    "award and contract value",
    "information on value of contract",
    "name and address of successful supplier",
    "other information",
    "total quantity or scope",
    "short description of the contract or purchase",
    "short description of the contract or purchase s",
    "address",
    // German: the titles, section I's labels (`Öffentlicher Auftraggeber`, `Offizielle
    // Bezeichnung`, `Kontaktstelle(n)`, `Anschrift`) and the awarding office named generically
    // (`Vergabestelle (siehe I.1)`).
    "sonstige informationen",
    "zusatzliche angaben",
    "zusatzliche informationen",
    "zustandige stelle fur rechtsbehelfs nachprufungsverfahren",
    "zustandige stelle fur nachprufungsverfahren",
    "zustandige stelle fur nachprufverfahren",
    "offentlicher auftraggeber",
    "offentliche auftraggeber",
    "auftraggeber",
    "offizielle bezeichnung",
    "kontaktstelle",
    "kontaktstellen",
    "vergabestelle",
    "anschrift",
    // French, Spanish, Portuguese, Italian.
    "autres informations",
    "informations complementaires",
    "renseignements complementaires",
    "instance chargee des procedures de recours",
    "point de contact",
    "points de contacts",
    "informacion adicional",
    "informacion complementaria",
    "informacao complementar",
    "ulteriori informazioni",
    "informazioni complementari",
    "punti di contatto",
    "punto di contatto",
    "indirizzo",
    // Dutch, Polish, Greek, the Nordic, Baltic and Slavic languages, Hungarian, Maltese, Irish,
    // Bulgarian, Romanian: TED's "Additional information" in each (and its Danish synonym), the
    // Dutch section I title and the Polish review body's.
    "nadere inlichtingen",
    "aanbestedende dienst",
    "informacje dodatkowe",
    "organ odpowiedzialny za procedury odwolawcze",
    "προσθετεσ πληροφοριεσ",
    "yderligere oplysninger",
    "supplerende oplysninger",
    "kompletterande information",
    "lisatietoja",
    "lisatiedot",
    "dodatecne informace",
    "doplnujuce informacie",
    "dodatne informacije",
    "tovabbi informaciok",
    "papildoma informacija",
    "papildu informacija",
    "lisateave",
    "tagħrif addizzjonali",
    "faisneis bhreise",
    "допълнителна информация",
    "informatii suplimentare",
    // Issue 511 unit 5, the review: Croatian, the 24th language of TED's pointer (2013 on), which
    // notice 12516947 predates — `Dodatni podaci` (the 2014 form's VI.3 label) and `Dopunski
    // podaci` (section VI's), reconstructed from the form labels, not read off a notice.
    "dodatni podaci",
    "dopunski podaci",
    // Issue 511 unit 6 item 6: the wordings of TED's earlier template (notice 19530163's ORG-10,
    // `See Section VI.3 Additional information` in 24 languages, read 2026-10-12) that 12516947's
    // read did not show — EL `Συμπληρωματικές πληροφορίες`, PT `Informação adicional`, SV
    // `Kompletterande upplysningar`, CS `Další informace` — and item 2's typo of `Kontaktstelle`
    // (`siehe Kontakstelle unter Punkt I.1)`, 9 mentions in residual dry 2310).
    "συμπληρωματικεσ πληροφοριεσ",
    "informacao adicional",
    "kompletterande upplysningar",
    "dalsi informace",
    "kontakstelle",
    // Issue 511 unit 6, the build review: the form labels a pointer names its target with, which
    // the no-separator keep rule ([`opens_with_a_name`]) read as a body — section I's `Name and
    // addresses` and contact point, VI.4's mediation body and the VI.4.3/VI.4.4 service title, the
    // plan's typo `See Section VI.3 Additonal Information, sub-section 2.` (dropped by unit 3's
    // wet), and the German labels (`siehe oben VI.4.1 Nachprüfungsstelle`, `siehe oben Ziffer
    // VI.4.1 Zuständige Stelle`, `siehe oben II.1.4 Kurze Beschreibung`, `siehe oben VI.3 Weitere
    // Angaben`, `siehe oben I.1 Adresse`, `… Ansprechpartner`).
    "name and address",
    "name and addresses",
    "names and addresses",
    "contact point",
    "contact points",
    "contact details",
    "mediation body",
    "service from which information about the lodging of appeals may be obtained",
    "additonal information",
    "nachprufungsstelle",
    "zustandige stelle",
    "kurze beschreibung",
    "weitere angaben",
    "adresse",
    "ansprechpartner",
];

/// Issue 511 unit 5: the polite lead-ins a section-reference pointer may open with, on the fold —
/// unit 4's [`POINTER_LEAD_INS`] and the winner slot's `For details of the awarded supplier(s),
/// please see section VI.2` (94 orgs in dry 2283).
const REFERENCE_LEAD_INS: [&str; 4] =
    ["please", "kindly", "for details of the awarded suppliers", "for details of the awarded supplier"];

/// Issue 511 unit 5 (dry 2283): the pointer words, on the fold, every one from the plan's `name`
/// folds or from TED's 23-language pointer template (notice 12516947), and since the review
/// Croatian and the abbreviations below. By orgs in the plan:
/// `siehe` 7,424, `see` 2,869, `wie` 993 (`wie oben` 421), `refer to` 919, `jak w` 767, `s` 671,
/// `cf` 668, `voir` 661, `vedasi` 197, `as in` 206, `zie` 182, `οπωσ` 182, `vease` 161, `vd` 133,
/// `come al` 123, `same as in` 111, `s o` 97, `confer` 95, `vedi` 89. A typo of `siehe` is kept
/// where the plan holds it (`sieh`, `siehs`, `sihe`, `seihe`, `sehe`).
const SECTION_POINTERS: [&str; 126] = [
    // English.
    "see",
    "see also",
    "see details at",
    "see details in",
    "refer to",
    "refer",
    "as",
    "as in",
    "as per",
    "same as",
    "same as in",
    "as stated in",
    "as described in",
    "as detailed in",
    "as set out in",
    "as listed in",
    "as specified in",
    // Issue 511 unit 6 item 2 (the units 4+5 reads, `legal_form` list): the `As` family's verbs
    // the list missed — `As shown in section I.1 above`, `As defined in (section) VI.3 of this
    // notice`, `As set out at I.1`, `As noted in VI.4.1`.
    "as shown in",
    "as defined in",
    "as set out",
    "as noted in",
    "cf",
    "confer",
    "idem",
    "vide",
    // German.
    "siehe",
    "siehe auch",
    "siehe hierzu",
    "siehe oben",
    "sieh",
    "siehs",
    "sihe",
    "seihe",
    "sehe",
    "s",
    "sh",
    "s h",
    "s o",
    "vgl",
    "vergleiche",
    "vergleich",
    "wie",
    "wie oben",
    "wie unter",
    "gemas",
    "gem",
    "analog",
    "analog zu",
    "verweis auf",
    // French.
    "voir",
    "voit",
    "voir modalites",
    "voir modalites exposees",
    "cfr",
    "cfer",
    "se reporter a",
    "se referer a",
    // Issue 511 unit 6 item 2: `Se référer au point VI.2 autres informations` (`au`, not `à`).
    "se reporter",
    "se referer",
    "meme entite que celle mentionnee",
    "lire",
    "supra",
    // Spanish and Portuguese.
    "vease",
    "ver",
    "se indica en",
    "indicado en",
    "el indicado en",
    // Italian.
    "vedasi",
    "vedi",
    "vd",
    "vds",
    "v",
    "si veda",
    "come",
    "come al",
    "come da",
    "come riportato",
    "di cui al",
    "rif",
    // Dutch.
    "zie",
    "zie onder",
    "zie bij",
    // Polish.
    "jak",
    "jak w",
    "jak wyzej",
    "tak jak w",
    "jw",
    "j w",
    "j w w",
    "zob",
    "patrz",
    "zgodnie z",
    "wg",
    // Greek.
    "οπωσ",
    "οπωσ αναφερεται",
    "ωσ",
    "ομοιωσ",
    "βλεπε",
    "η αναφερομενη",
    // Danish, Swedish, Finnish, Czech, Slovak, Slovenian, Hungarian, Lithuanian, Latvian,
    // Estonian, Maltese, Irish, Bulgarian, Romanian.
    "se",
    "katso",
    "kts",
    "sama kuin",
    "viz",
    "pozri",
    "ako v",
    "uvedene v",
    "glej",
    "lasd",
    "zr",
    "skatit",
    "vt",
    "ara",
    "feach",
    "вж",
    "a se vedea",
    // Issue 511 unit 5, the review: Croatian (TED's 24th language, 2013 on; reconstructed, not
    // read off a notice), the common abbreviations of listed languages the template read did not
    // show (FI `ks.` 4 orgs in dry 2283, LV `sk.`, EL `βλ.`, RO `vezi`, BG `виж`), and IT `vgs.`
    // (2 orgs). The echo sweeps only a copy one of these opens ([`pointer_led`]), so a missing
    // opener keeps a pointer party.
    "vidi",
    "vidjeti",
    "ks",
    "sk",
    "βλ",
    "vezi",
    "виж",
    "vgs",
    // Issue 511 unit 6 item 6: the openers of 19530163's PT and SL copies, the two no list held
    // (`Consultar o ponto VI.3 «Informação adicional»`, `Glejte oddelek VI.3) Dodatne
    // informacije`), which kept ORG-10 (140 mentions) through the units 4+5 wet.
    "consultar",
    "glejte",
];

/// Issue 511 unit 5 (dry 2283): the words that name the kind of place pointed at, on the fold —
/// `section` 1,898 orgs, `pkt` 1,200, `ziffer` 1,006, `point` 747, `punkt` 703, `punto` 638,
/// `unter` 593, `rubrique` 557, `abschnitt` 289 — and TED's template's (`oddíl`, `pontot`, `dalį`,
/// `iedaļu`, `jaotist`, `Taqsima`, `Alt`, `раздел`, `secţiunea`). A bare number or Roman numeral
/// may follow one (`Nr. 1`, `Section VI`, `bloc 7`).
const SECTION_LOCATORS: [&str; 82] = [
    "section",
    "point",
    "points",
    "paragraph",
    "para",
    "article",
    "art",
    "item",
    "heading",
    "part",
    "field",
    "punkt",
    "punkte",
    "pkt",
    "p",
    "ziffer",
    "ziff",
    "nr",
    "nummer",
    "abschnitt",
    "kapitel",
    "unter",
    "tz",
    "artikel",
    "rubrique",
    "paragraphe",
    "bloc",
    "apartado",
    "punto",
    "punti",
    "p to",
    "pt",
    "sezione",
    "sez",
    "paragrafo",
    "par",
    "ponto",
    "afdeling",
    "afd",
    "punt",
    "paragraaf",
    "hoofdstuk",
    "rubriek",
    "onderdeel",
    "punkcie",
    "punktu",
    "sekcja",
    "sekcji",
    "τμημα",
    "σημειο",
    "παραγραφο",
    "παρ",
    "afsnit",
    "avsnitt",
    "kohta",
    "kohdassa",
    "oddil",
    "oddiel",
    "bod",
    "bode",
    "oddelek",
    "pontot",
    "dali",
    "iedalu",
    "jaotist",
    "taqsima",
    "alt",
    "раздел",
    "sectiunea",
    // Issue 511 unit 5, the review: Croatian `odjeljak` / `točku` (reconstructed), SL `točko`
    // (`Glej točko VI.2.`, a winner in dry 2283), and the abbreviations `Abschn.` (3 orgs) and
    // `sec.` (2).
    "odjeljak",
    "odjeljku",
    "tocku",
    "tocki",
    "tocko",
    "abschn",
    "sec",
    // Issue 511 unit 6 item 6: 19530163's ET `Vt punkti VI.3 (lisateave)`, HU `Lásd a VI.3.
    // szakaszt` (postposed, [`POSTPOSED_LOCATORS`]) and MT `Ara t-Taqsima VI.3` (the article
    // hyphened onto the noun); item 2's typos with their mentions in residual dry 2310 (`Siehe
    // Abschnittt VI.4.1)` 6, `siehe Abschnit I.1` 6, `cf rubirque Vi.4.1` 6).
    "punkti",
    "szakaszt",
    "t taqsima",
    "abschnittt",
    "abschnit",
    "rubirque",
];

/// Issue 511 unit 5 (dry 2283): where the place pointed at sits — `below` 545 orgs, `above` 286,
/// `ci dessus` 34, `powyzej` 12.
const SECTION_POSITIONS: [&str; 17] = [
    "above",
    "below",
    "oben",
    "unten",
    "ci dessus",
    "ci dessous",
    "susmentionne",
    "susmentionnes",
    "sopra",
    "precedente",
    "hierboven",
    "powyzej",
    "ponizej",
    "ανωτερω",
    "ovan",
    // Issue 511 unit 6 item 2: `As specified in section I.1 and the address/URL details remain the
    // same.` — the place pointed at holds them unchanged.
    "remain the same",
    "remains the same",
];

/// Issue 511 unit 5 (dry 2283): the notice itself, which a reference may name (`der
/// Bekanntmachung` 186 orgs, `ogłoszenia` 96, `of this notice` 59, `del presente avviso` 47, `der
/// vorliegenden Bekanntmachung` 36, `dieser Vergabebekanntmachung` 33).
const SECTION_NOTICE_WORDS: [&str; 33] = [
    "notice",
    "contract notice",
    "bekanntmachung",
    "vergabebekanntmachung",
    "auftragsbekanntmachung",
    "veroffentlichung",
    "avis",
    "avviso",
    "bando",
    "bando di gara",
    "anuncio",
    "licitacion",
    "ogloszenia",
    "aankondiging",
    "publicatie",
    "oznamenia",
    "this",
    "dieser",
    "vorliegenden",
    "present",
    "presente",
    "este",
    "niniejszego",
    "deze",
    "tohto",
    // Issue 511 unit 6 item 2 (the units 4+5 reads): `See VI.4.1) Review body field of this
    // Contract Award Notice.` (33 mentions), `Se référer à l'article VI.3 du présent appel à
    // candidatures` (34), `Come indicato al punto I.1) del presente avviso di gara`, `As detailed
    // in section II.2.1) of the corresponding Contract Notice`, `Please see section II.24 of
    // original notice`, and the plan's typo `Ver el apartado VI.3.2 del presenta anuncio` (16).
    "contract award notice",
    "award notice",
    "appel a candidatures",
    "appel d offres",
    "avviso di gara",
    "corresponding",
    "original",
    "presenta",
];

/// Issue 511 unit 5 (dry 2283): the articles, prepositions and conjunctions between a pointer's
/// words (`jak w pkt`, `Ver el apartado … del presente anuncio`, `jw. dla pkt`, `Voir au point`,
/// `VI.3.1) und VI.3.2)`, HU `című`) — never the name's first word.
const SECTION_JOINERS: [&str; 45] = [
    "in", "at", "of", "the", "under", "and", "for", "within", "der", "die", "bei", "und", "oder", "sowie", "bzw", "zu", "nach",
    "le", "la", "les", "l", "du", "de", "au", "aux", "a", "en", "el", "del", "al", "o", "u", "il", "i", "alla", "van", "onder",
    "w", "dla", "στο", "στην", "με", "cimu", "relative",
    // Issue 511 unit 6 item 2: the Dutch article (`Zie het gestelde in VI.3`).
    "het",
];

/// Issue 511 unit 5 (dry 2283): the generic words a pointer names its target with (`See
/// information in VI.4.3`, `Siehe Angaben unter Ziff. VI.4.1` 17 orgs, `refer to information
/// contained in Section VI.4.2`, `punti di contatto indicati al punto I.1`) — never the name's
/// first word.
const SECTION_POINTER_NOUNS: [&str; 30] = [
    "details",
    "information",
    "informations",
    "angaben",
    "guidance",
    "contained",
    "given",
    "provided",
    "stated",
    "listed",
    "indicated",
    "genannte",
    "genannten",
    "indicati",
    "indicato",
    "citee",
    // Issue 511 unit 6 item 2 (the units 4+5 reads and residual dry 2310): `As per the body in
    // I.1`, `As per those listed in the additional information field VI.3)`, `Refer to named
    // Contracting Authority in section I.1`, `See links detailed in II.1.5` (58 mentions), `See link
    // in II.2.4` (17), `Refer to instructions contained in Section VI.4.2` (13), `Please refer to
    // the answer in section VI.4.2`, `Siehe die unter Ziffer VI.4.1) genannte Stelle.` (31), `Siehe
    // die unter VI.4.1) bezeichnete Stelle`, `Siehe Kontaktdaten unter Nr. I.1).`, `Zie het
    // gestelde in VI.3`, `Voir informations citées à l'article VI.3)`, and the `address/URL
    // details` of an `As specified in` pointer.
    "body",
    "those",
    "named",
    "link",
    "links",
    "detailed",
    "instructions",
    "answer",
    "stelle",
    "bezeichnete",
    "kontaktdaten",
    "gestelde",
    "citees",
    "url",
];

/// Issue 511 unit 5: what a piece of a section-reference name is ([`reference_shape`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReferencePart {
    /// [`REFERENCE_LEAD_INS`]: only before every other piece.
    Lead,
    /// [`SECTION_POINTERS`]: never after a reference.
    Pointer,
    /// [`SECTION_LOCATORS`].
    Locator,
    /// [`SECTION_POSITIONS`].
    Position,
    /// [`SECTION_NOTICE_WORDS`].
    Notice,
    /// [`FORM_SECTION_TITLES`].
    Title,
    /// [`SECTION_JOINERS`].
    Joiner,
    /// [`SECTION_POINTER_NOUNS`].
    Noun,
    /// A form section reference ([`reference_pieces`]: `VI.4.1`, `I.1)`, `V.1.4`).
    Reference,
    /// A bare number or Roman numeral, only right after a locator (`Nr. 1`, `Section VI`).
    Number,
}

/// The closed lists of [`ReferencePart`], in the order a phrase that two of them hold is read:
/// a longer phrase wins, then the earlier list (`v` is the Italian pointer `v.`, not the Czech
/// `v`; `a se vedea` is the Romanian pointer, not the joiner `a`).
const REFERENCE_LISTS: [(ReferencePart, &[&str]); 8] = [
    (ReferencePart::Lead, &REFERENCE_LEAD_INS),
    (ReferencePart::Pointer, &SECTION_POINTERS),
    (ReferencePart::Locator, &SECTION_LOCATORS),
    (ReferencePart::Position, &SECTION_POSITIONS),
    (ReferencePart::Notice, &SECTION_NOTICE_WORDS),
    (ReferencePart::Title, &FORM_SECTION_TITLES),
    (ReferencePart::Joiner, &SECTION_JOINERS),
    (ReferencePart::Noun, &SECTION_POINTER_NOUNS),
];

/// Issue 511 unit 5: the two shapes of a section-reference name ([`section_reference`]). The
/// fold classes both `Placeholder`; the shape is for the tests and the dry plan's reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReferenceShape {
    /// A pointer word before the references: `siehe VI.4.1`, `As in I.1 above`, `jak w pkt
    /// VI.4.1) ogłoszenia`, `Voir point VI.2) Autres informations.`.
    Pointer,
    /// References and form labels only, the form's own heading spilled into the slot: `V.1)
    /// Award and contract value`, `VI.4.1`, `Section VI.4.2) below`.
    Label,
}

/// Issue 511 unit 5 (dry 2283, the `name` class): whether `raw` is a SECTION-REFERENCE POINTER —
/// nothing but closed-list words around one or more form section references, so it names no body:
/// - an optional polite lead-in ([`REFERENCE_LEAD_INS`]), then a pointer ([`SECTION_POINTERS`],
///   every language in the plan and in TED's template), a locator ([`SECTION_LOCATORS`]), a form
///   label ([`FORM_SECTION_TITLES`]) or a reference opens it;
/// - one or more form section references (a [`ROMAN`] numeral and dotted digits: `I.1`, `II.1.5`,
///   `VI.4.1)`), joined by `and`, `und`, `i`, `,`;
/// - and between and after them only locators, positions ([`SECTION_POSITIONS`]: `above`, `ci-
///   dessus`), the notice ([`SECTION_NOTICE_WORDS`]: `der Bekanntmachung`), form labels, joiners
///   ([`SECTION_JOINERS`]) and a pointer's generic nouns ([`SECTION_POINTER_NOUNS`]) — no pointer
///   after a reference;
/// - and, since the review of unit 5, after a separator past a reference only titles, positions
///   and joiners, every locator pointing at a reference, and a label closing on labels
///   ([`reference_shape`]: `See section VI.2: Avis`, `I.4) Avis`, `Point P (voir I.1)` name a
///   body);
/// - and no legal form ([`legal_form_reading`]; the pointer word `as` is no `AS`).
///
/// Anything left over is a name, or whatever the rest of [`not_a_name_in`] reads: a named body
/// beside the pointer (`Krajowa Izba Odwoławcza – jak wyżej w pkt VI.4.1)`, `Siemens – see section
/// VI.3`, `Siehe Punkt VI.4.1) - VKS Wien`), a role (`responsabile del procedimento di cui al punto
/// VI.3 lettera q`), a unit (`Referat II.3`, `Abteilung VI.4`), a legal form (`Section 1.2 Ltd`), a
/// date (`V.1) DATE OF CONTRACT AWARD: 15.5.2006`), an answer (`V.5) … SUB-CONTRACTED: No`).
pub fn section_reference_pointer(raw: &str) -> bool {
    section_reference(raw).is_some()
}

/// [`section_reference_pointer`], with its shape.
fn section_reference(raw: &str) -> Option<ReferenceShape> {
    let shape = reference_shape(&reference_pieces(raw)?)?;
    (!legal_form_reading(raw, true)).then_some(shape)
}

/// Whether `raw` opens, after its polite lead-ins, with a [`SECTION_POINTERS`] word — the pointer
/// a translated pointer opens with in every language of TED's template ([`translation_shaped`]).
pub fn pointer_led(raw: &str) -> bool {
    let pieces = raw_pieces(raw);
    let mut i = 0;
    loop {
        match reference_phrase(&pieces.words, i) {
            Some((n, ReferencePart::Lead)) => i += n,
            Some((_, ReferencePart::Pointer)) => return true,
            _ => return false,
        }
    }
}

/// Whether `raw` holds a form section reference at all (a [`ROMAN`] letter, a dot, a digit): the
/// cheap test before [`reference_pieces`] builds anything, since the fold reads every party name.
/// Issue 511 unit 6 item 3: or a [`ROMAN`] letter, spaces and a dotted digit pair, the numeral
/// spaced off its number (`siehe VI 4.1`), which [`raw_pieces`] reads as one reference.
fn has_form_reference(raw: &str) -> bool {
    let roman = |b: u8| matches!(b, b'I' | b'V' | b'X' | b'i' | b'v' | b'x');
    let bytes = raw.as_bytes();
    bytes.windows(3).any(|w| w[1] == b'.' && roman(w[0]) && w[2].is_ascii_digit())
        || bytes.windows(2).enumerate().any(|(i, w)| {
            roman(w[0]) && w[1] == b' ' && {
                let rest = &bytes[i + 1..];
                let rest = &rest[rest.iter().take_while(|b| **b == b' ').count()..];
                let n = rest.iter().take_while(|b| b.is_ascii_digit()).count();
                n > 0 && rest.get(n) == Some(&b'.') && rest.get(n + 1).is_some_and(u8::is_ascii_digit)
            }
        })
}

/// Issue 511 unit 5: a name as [`reference_shape`] reads it ([`raw_pieces`]).
struct Pieces {
    /// One per alphanumeric run, folded ([`fold`]), except that a [`ROMAN`] run followed by `.`
    /// and a run of ASCII digits, and any further `.` and digits, is one reference (`None`).
    words: Vec<Option<String>>,
    /// Index for index: whether the raw text before the piece holds a separator, as unit 4's
    /// keep rule reads one ([`open_separator`]: `:`, `(`, an en or em dash, a hyphen followed by
    /// a space) — `See section VI.2: Avis`, `Voir VI.2 - Point P`.
    separated: Vec<bool>,
    /// Index for index: the byte offset in the raw name where the piece ends (issue 511 unit 6, the
    /// build review: [`after_last_reference`]).
    ends: Vec<usize>,
}

/// `raw` as [`reference_shape`] reads it, `None` without a form section reference
/// ([`has_form_reference`]): [`raw_pieces`]' words — `VI.4.1)`, `I.1°`, `Nr .I.1`, `Vi.4.2`,
/// `pkt.VI.4.1` each one reference, and since issue 511 unit 6 item 3 the spaced spellings of one
/// (`VI.4 1`, `VI.4. 1`, `VI 4.1`). A digit run glued to a letter ends the reference (`VI.4.1a`
/// leaves the word `1a`), and a bare `VI` is a word.
fn reference_pieces(raw: &str) -> Option<Pieces> {
    if !has_form_reference(raw) {
        return None;
    }
    let pieces = raw_pieces(raw);
    pieces.words.iter().any(Option::is_none).then_some(pieces)
}

/// [`reference_pieces`] without the test that a reference is there.
fn raw_pieces(raw: &str) -> Pieces {
    let runs = alphanumeric_runs(raw);
    // The raw text between run `j` and the next, and whether the next is an ASCII digit run (of at
    // most `max` digits).
    let gap = |j: usize| runs.get(j + 1).map(|&(start, _)| &raw[runs[j].1..start]);
    let digits = |j: usize, max: usize| {
        runs.get(j).is_some_and(|&(start, end)| end - start <= max && raw[start..end].bytes().all(|b| b.is_ascii_digit()))
    };
    let spaces = |g: &str| !g.is_empty() && g.bytes().all(|b| b == b' ');
    // Whether run `j` is followed by a `.` and a digit run: the next group of a reference.
    let dotted = |j: usize| gap(j) == Some(".") && digits(j + 1, usize::MAX);
    // Issue 511 unit 6 item 3 (residual dry 2310's mixed folds): a reference's LAST group may follow
    // a space after the dot (`Siehe VI.4. 1)`) or stand off with a space for the second dot when
    // `first` (`Wie VI.4 1)`, `Voir section VI.4 2`) — one or two digits that no dotted group
    // follows, so a year, a count or a date (`V.1 12.05.2010`) is no part of it.
    let spaced_last = |j: usize, first: bool| {
        gap(j).is_some_and(|g| g.strip_prefix('.').is_some_and(spaces) || (first && spaces(g)))
            && digits(j + 1, 2)
            && !dotted(j + 1)
    };
    let mut pieces = Pieces {
        words: Vec::with_capacity(runs.len()),
        separated: Vec::with_capacity(runs.len()),
        ends: Vec::with_capacity(runs.len()),
    };
    let mut k = 0;
    let mut previous_end = 0;
    while k < runs.len() {
        let (start, end) = runs[k];
        pieces.separated.push(separator_gap(&raw[previous_end..start]));
        let roman = ROMAN.contains(&raw[start..end].to_ascii_lowercase().as_str());
        // Issue 511 unit 6 item 3: the numeral spaced off a dotted number (`siehe I.1 oder VI 4.1`).
        let spaced = roman && gap(k).is_some_and(spaces) && digits(k + 1, usize::MAX) && dotted(k + 1);
        if roman && (dotted(k) || spaced) {
            let mut j = k + 1;
            while dotted(j) {
                j += 1;
            }
            // A space stands for the second dot only right after the first group of an unspaced
            // reference (`VI.4 1`, not `VI 4.1 2` or `VI.4.1 2`).
            if spaced_last(j, j == k + 1 && !spaced) {
                j += 1;
            }
            pieces.words.push(None);
            previous_end = runs[j].1;
            pieces.ends.push(previous_end);
            k = j + 1;
        } else {
            pieces.words.push(Some(fold(&raw[start..end])));
            previous_end = end;
            pieces.ends.push(end);
            k += 1;
        }
    }
    pieces
}

/// Whether `gap`, the raw text between two pieces, holds a separator as [`open_separator`] reads
/// one: `:`, `(`, an en or em dash, or a hyphen followed by a space (`VI.2 - Point P`, not
/// `Rechtsbehelfs-/Nachprüfungsverfahren` or `ci-dessus`).
fn separator_gap(gap: &str) -> bool {
    gap.contains([':', '(', '–', '—']) || gap.split('-').skip(1).any(|after| after.starts_with(char::is_whitespace))
}

/// The longest [`REFERENCE_LISTS`] phrase that the words of `pieces` open with at `i` — its
/// length in pieces and its part, the earlier list on a tie; `None` at a reference, past the end,
/// or where no phrase matches.
fn reference_phrase(pieces: &[Option<String>], i: usize) -> Option<(usize, ReferencePart)> {
    let first = pieces.get(i)?.as_deref()?;
    let mut best: Option<(usize, ReferencePart)> = None;
    for (part, list) in REFERENCE_LISTS {
        for phrase in list {
            // The first word decides most phrases, before any count.
            if phrase.split(' ').next() != Some(first) {
                continue;
            }
            let n = phrase.split(' ').count();
            if best.is_some_and(|(m, _)| m >= n) {
                continue;
            }
            if phrase.split(' ').enumerate().all(|(t, word)| pieces.get(i + t).and_then(Option::as_deref) == Some(word)) {
                best = Some((n, part));
            }
        }
    }
    best
}

/// Issue 511 unit 5, the review: the [`SECTION_LOCATORS`] a language writes AFTER its reference —
/// TED's HU `Lásd a VI.2., További információk című pontot.`, LT `Žr. VI.2 dalį …`, LV `Skatīt
/// VI.2) iedaļu …` — anchored by the reference before them ([`reference_shape`]).
const POSTPOSED_LOCATORS: [&str; 4] = ["pontot", "dali", "iedalu", "szakaszt"];

/// Issue 511 unit 5: the grammar of a section-reference name ([`section_reference_pointer`]) over
/// its pieces ([`reference_pieces`]): every word in a [`REFERENCE_LISTS`] phrase, or a bare number
/// right after a locator; lead-ins first; then a pointer, locator, label or reference opens; at
/// least one reference; no pointer after one. A pointer before the references makes the
/// [`ReferenceShape::Pointer`], none the [`ReferenceShape::Label`].
///
/// The review of unit 5 (a list word can be a company's name: Avis, Point P) adds three rules,
/// each refusing the name to the rest of [`not_a_name_in`], which reads it as before unit 5:
/// - **a separator after a reference** (`:`, `(`, a dash: [`Pieces::separated`]) opens text unit
///   4's keep rule reads as a name, so up to the next reference it may hold only titles, positions
///   and joiners (`See section VI.3 (Additional Information)`, `Patrz sekcja VI.2) - informacje
///   dodatkowe`, `see VI.4.2 (below)`) — `See section VI.2: Avis` stays the
///   [`NotAName::AwardSummary`] it was, `Voir VI.2 - Point P` a name;
/// - **a locator points at a reference**: forward through locators, joiners and titles to a
///   reference or a number (`Please see the item under section VI.4.2`, `Se reporter à la rubrique
///   IV.3 point 16`), or back to its reference for a [`POSTPOSED_LOCATORS`] word; one right after
///   a title is part of it (`Additional Information Section`, `Review Body Field`). `Point P
///   (voir I.1)` and `siehe I.1) Point P` name a body;
/// - **a label closes on labels**: after the last reference of a [`ReferenceShape::Label`] only
///   titles, positions and joiners (`V.1) AWARD AND CONTRACT VALUE:`, `Section VI.4.2) below`),
///   so a lot-numbered winner is a name (`I.4) Avis`, `I.3) Point P`, `II.1) Le Point`) as dry
///   2283's `I.6) Colas` and `I.4) Brake France` are.
fn reference_shape(pieces: &Pieces) -> Option<ReferenceShape> {
    use ReferencePart::{Joiner, Lead, Locator, Number, Pointer, Position, Reference, Title};
    let words = &pieces.words;
    // The phrases read: (first piece, part, read as a title — a title, or a locator right after
    // one).
    let mut units: Vec<(usize, ReferencePart, bool)> = Vec::new();
    let (mut opened, mut referenced, mut pointer, mut after_separator) = (false, false, false, false);
    let mut i = 0;
    while i < words.len() {
        let previous = units.last().copied();
        let (n, part) = match &words[i] {
            None => (1, Reference),
            Some(word) => match reference_phrase(words, i) {
                Some(found) => found,
                None if previous.is_some_and(|(_, part, _)| part == Locator)
                    && (word.bytes().all(|b| b.is_ascii_digit()) || ROMAN.contains(&word.as_str())) =>
                {
                    (1, Number)
                }
                None => return None,
            },
        };
        let titled = part == Title || (part == Locator && previous.is_some_and(|(_, _, titled)| titled));
        if part == Lead {
            if opened {
                return None;
            }
        } else {
            if !opened && !matches!(part, Pointer | Locator | Reference | Title) {
                return None;
            }
            opened = true;
            if part == Reference {
                after_separator = false;
            } else if referenced && pieces.separated[i] {
                after_separator = true;
            }
            if after_separator && !(titled || matches!(part, Position | Joiner)) {
                return None;
            }
            match part {
                Pointer if referenced => return None,
                Pointer => pointer = true,
                Reference => referenced = true,
                _ => {}
            }
        }
        units.push((i, part, titled));
        i += n;
    }
    if !referenced {
        return None;
    }
    // Every locator points at a reference.
    let through = |&(_, part, titled): &(usize, ReferencePart, bool)| titled || matches!(part, Locator | Joiner);
    let target = |&(_, part, _): &(usize, ReferencePart, bool)| matches!(part, Reference | Number);
    for (at, &(first, part, titled)) in units.iter().enumerate() {
        if part != Locator || titled {
            continue;
        }
        let forward = units[at + 1..].iter().find(|unit| !through(unit)).is_some_and(target);
        let postposed = words[first].as_deref().is_some_and(|word| POSTPOSED_LOCATORS.contains(&word));
        let backward = postposed && units[..at].iter().rev().find(|unit| !through(unit)).is_some_and(target);
        if !forward && !backward {
            return None;
        }
    }
    if pointer {
        return Some(ReferenceShape::Pointer);
    }
    // A label closes on labels.
    let last = units.iter().rposition(|&(_, part, _)| part == Reference).unwrap_or(0);
    units[last + 1..]
        .iter()
        .all(|&(_, part, titled)| titled || matches!(part, Position | Joiner))
        .then_some(ReferenceShape::Label)
}

/// `s` with every [`FORM_SECTION_TITLES`] phrase blanked out — whole words, compared on the fold,
/// each replaced by as many spaces, so what is left (a body named beside a title, `High Court of
/// Justice (additional information)`, and its punctuation) reads as before. `s` itself when it
/// holds no title.
fn without_section_titles(s: &str) -> std::borrow::Cow<'_, str> {
    let runs = alphanumeric_runs(s);
    let words: Vec<String> = runs.iter().map(|&(start, end)| fold(&s[start..end])).collect();
    let mut title = vec![false; words.len()];
    for phrase in FORM_SECTION_TITLES {
        let phrase: Vec<&str> = phrase.split(' ').collect();
        for (at, window) in words.windows(phrase.len()).enumerate() {
            if window.iter().zip(&phrase).all(|(word, p)| word == p) {
                title[at..at + phrase.len()].fill(true);
            }
        }
    }
    if !title.contains(&true) {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = s.to_owned();
    for (&(start, end), _) in runs.iter().zip(&title).filter(|(_, title)| **title) {
        // Same byte length, so the later runs' ranges stay valid.
        out.replace_range(start..end, &" ".repeat(end - start));
    }
    std::borrow::Cow::Owned(out)
}

/// The raw text after a stem (`stem`, found at byte `pos` of `padded`, the space-padded
/// fold of `raw`) and the word it ends in; `None` when the fold's words and `raw`'s
/// alphanumeric runs do not line up.
fn after_stem<'a>(raw: &'a str, padded: &str, stem: &str, pos: usize) -> Option<&'a str> {
    // The fold's words are the raw name's alphanumeric runs, one for one (`match_norm`
    // spaces every non-alphanumeric run; the Latin fold maps char to char).
    let runs = alphanumeric_runs(raw);
    let first = padded[..pos].matches(' ').count();
    let last = first + stem.trim_start().split(' ').count() - 1;
    let &(_, end) = runs.get(last).filter(|_| runs.len() == padded.trim_start().split(' ').count())?;
    Some(&raw[end..])
}

/// The two perfil stems among [`POINTER_STEMS`]: a genitive after one names the profile's
/// owner ([`perfil_owner`]).
const PERFIL_STEMS: [&str; 2] = [" perfil del contratante", " perfil de contratante"];

/// Issue 511 unit 3 read: the folded words that never name a perfil's owner — the Spanish
/// function words title case capitalises (`Perfil Del Contratante De La Página Web`), and
/// the web lead-in's and a URL's words (`de la página WEB de la Fábrica Nacional …`,
/// `sitio internet`). Excluded wherever they sit, the lead-in needs no strip of its own.
const PERFIL_NON_NAMES: [&str; 15] =
    ["de", "del", "la", "las", "los", "el", "y", "en", "pagina", "sitio", "web", "internet", "www", "http", "https"];

/// Issue 511 unit 3 read: the generic phrases a perfil's genitive may open on, on the fold —
/// the buyer's office or the national platform, never a named owner (`Perfil del Contratante
/// del Órgano de Contratación`, `perfil del contratante de la Plataforma de Contratación del
/// Sector Público`). Their words count for nothing, the rest still reads (`Perfil de
/// Contratante del Ayto. de Soria y plataforma de Contratación del Estado` names Soria).
const PERFIL_GENERIC: [&str; 3] = [
    "organo de contratacion",
    "plataforma de contratacion del sector publico",
    "plataforma de contratacion del estado",
];

/// Issue 511 unit 3 read (the dry plan's real-party hits, 26 orgs / 91 mentions in plan
/// 2265, nearly all specifications-provider): whether the text after a perfil stem that
/// opens the name (`after`, [`pointer_names`]) names the profile's owner — `Perfil de
/// contratante del Gobierno de Aragón`, `PERFIL DEL CONTRATANTE DE LA CARM`, `Perfil de
/// contratante y Registro Auxiliar de la Universidad de Valladolid`. The same body written
/// before the stem was already kept ([`named_before_stem`]: `Junta de Andalucía - ver perfil
/// de contratante`), so its fate hung on word order alone.
/// 1. `after` opens, with nothing but space before it, on a genitive ([`genitive_at`]) — or
///    on `y`, some words and a genitive (the profile and another office of the owner);
/// 2. a word after that genitive is capitalised with two or more letters (an acronym too:
///    `SESCAM`, `FNMT-RCM`), and is no [`PERFIL_NON_NAMES`] word, no part of a URL
///    ([`url_token`]: `(www.x.es)`) and no part of a [`PERFIL_GENERIC`] phrase.
///
/// `Perfil del contratante de la página web` and `… del órgano de contratación` name nothing.
fn perfil_owner(after: &str) -> bool {
    let runs = alphanumeric_runs(after);
    if runs.first().is_none_or(|&(start, _)| !after[..start].trim().is_empty()) {
        return false;
    }
    let words: Vec<String> = runs.iter().map(|&(start, end)| fold(&after[start..end])).collect();
    let owner = genitive_at(&words, 0)
        .or_else(|| (words[0] == "y").then(|| (1..words.len()).find_map(|i| genitive_at(&words, i))).flatten());
    let Some(owner) = owner else {
        return false;
    };
    let mut generic = vec![false; words.len()];
    for phrase in PERFIL_GENERIC {
        let phrase: Vec<&str> = phrase.split(' ').collect();
        for (at, window) in words.windows(phrase.len()).enumerate() {
            if window.iter().zip(&phrase).all(|(word, p)| word == p) {
                generic[at..at + phrase.len()].fill(true);
            }
        }
    }
    // The whitespace tokens that are URLs, as byte ranges of `after`.
    let mut urls: Vec<std::ops::Range<usize>> = Vec::new();
    let mut start: Option<usize> = None;
    for (at, c) in after.char_indices().chain(std::iter::once((after.len(), ' '))) {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(at),
            (true, Some(from)) => {
                if url_token(&after[from..at]) {
                    urls.push(from..at);
                }
                start = None;
            }
            _ => {}
        }
    }
    (owner..words.len()).any(|i| {
        let word = &after[runs[i].0..runs[i].1];
        word.chars().next().is_some_and(char::is_uppercase)
            && word.chars().filter(|c| c.is_alphabetic()).count() >= 2
            && !PERFIL_NON_NAMES.contains(&words[i].as_str())
            && !generic[i]
            && !urls.iter().any(|url| url.contains(&runs[i].0))
    })
}

/// The index of the first word after a Spanish or Catalan genitive at word `i` of the folded
/// `words`: `del`, `de la`, `de las`, `de los` or `de`; `None` for any other word. The elided
/// `de l'Ajuntament` reads as `de`: its one-letter `l` is no name ([`perfil_owner`]).
fn genitive_at(words: &[String], i: usize) -> Option<usize> {
    match words.get(i)?.as_str() {
        "del" => Some(i + 1),
        "de" if words.get(i + 1).is_some_and(|w| matches!(w.as_str(), "la" | "las" | "los")) => Some(i + 2),
        "de" => Some(i + 1),
        _ => None,
    }
}

/// A whitespace token that is a URL (`www.carm.es`, `(www.fnmt.es)`, `http://contratacion…`,
/// any case): none of its words names a perfil's owner ([`perfil_owner`]).
fn url_token(token: &str) -> bool {
    let t = token.trim_start_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
    t.starts_with("www.") || t.starts_with("http") || t.contains("://")
}

/// Issue 511 unit 3 read: whether the text after a blank stem (`rest`, [`after_stem`]) is a
/// `but` clause that names a body ([`names_something`]) — the review-body slot's `Not
/// applicable, but proceedings may be started in the High Court of England and Wales …`
/// (plan 2265: 6 orgs / 8 mentions, the decision's `see section Royal Courts of Justice` in
/// other words). The `but` is required, an optional `,` before it: a comma alone continues
/// the blank (`Not applicable, see section VI.4`, `Not Applicable, since Minesto is a private
/// company`, `nie dotyczy, zamówienie … ustawie PZP`), whose capitals name nobody.
fn but_names(rest: &str) -> bool {
    let t = rest.trim_start();
    let t = t.strip_prefix(',').unwrap_or(t).trim_start();
    t.get(..3).is_some_and(|word| word.eq_ignore_ascii_case("but"))
        && t[3..].starts_with(|c: char| !c.is_alphanumeric())
        && names_something(&t[3..])
}

/// The words that lead into a [`StemAt::Boundary`] stem as part of its phrase (`Véase /
/// Ver perfil del contratante`, `Disclosure would prejudice …`), on the fold: read off the
/// end of the text before the stem, so they never count as a name there.
const BOUNDARY_LEAD_INS: [&str; 6] = ["ver", "vease", "veanse", "consultar", "consulte", "disclosure"];

/// Issue 511's keep rule for a [`StemAt::Boundary`] stem (found at byte `pos` of `padded`,
/// the space-padded fold of `raw`), which may close a name as well as open one: whether the
/// text BEFORE it, once the phrase's own lead-in words are off ([`BOUNDARY_LEAD_INS`]),
/// names something ([`names_something`]) or has a summary's shape ([`summary_shape`]) —
/// `Ayuntamiento de Madrid (véase perfil del contratante)`, `Junta de Andalucía - ver perfil
/// de contratante`, `Serco — disclosure would prejudice commercial interests`. The bare
/// phrases (`Ver perfil del contratante`, `Disclosure would prejudice …`) have nothing left.
fn named_before_stem(raw: &str, padded: &str, pos: usize) -> bool {
    let runs = alphanumeric_runs(raw);
    if runs.len() != padded.trim_start().split(' ').count() {
        return false;
    }
    // The stem's first word, by the fold's words (one per alphanumeric run of `raw`).
    let mut words = padded[..pos].matches(' ').count();
    while words > 0 && BOUNDARY_LEAD_INS.contains(&fold(&raw[runs[words - 1].0..runs[words - 1].1]).as_str()) {
        words -= 1;
    }
    if words == 0 {
        return false;
    }
    let before = &raw[..runs[words - 1].1];
    names_something(before) || summary_shape(before, &fold(before))
}

/// The byte ranges of `raw`'s alphanumeric runs: the fold's words, in order.
fn alphanumeric_runs(raw: &str) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut start: Option<usize> = None;
    for (at, c) in raw.char_indices() {
        match (c.is_alphanumeric(), start) {
            (true, None) => start = Some(at),
            (false, Some(from)) => {
                runs.push((from, at));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(from) = start {
        runs.push((from, raw.len()));
    }
    runs
}

/// The words a pointer's section reference is written with (`siehe oben unter Ziffer
/// VI.4.1`, `Nr. 3`, `see section below`), compared lowercased without trailing punctuation.
const REFERENCE_WORDS: [&str; 10] = ["ziff", "ziffer", "punkt", "abschnitt", "unter", "nr", "below", "above", "unten", "oben"];

/// `s` without its leading section references — section numbers (`VI.4.1`, `VI.3)`, `1)`,
/// `II`), [`REFERENCE_WORDS`], and one lot designator with its number (`Lot 2`, `lot n° 2`,
/// `Lot 1A`) — and whether any came off. A reference that ends in `:` hands the colon on
/// (the rest starts at it), so the keep rule's step 3 sees its separator: `See section
/// VI.4.1: High Court of Justice` names the court, as `See section VI.4.1 - High Court` does.
fn strip_references(s: &str) -> (&str, bool) {
    let mut s = s;
    let (mut stripped, mut lot) = (false, false);
    loop {
        let t = s.trim_start();
        let end = t.find(char::is_whitespace).unwrap_or(t.len());
        let token = &t[..end];
        if token.is_empty() {
            return (t, stripped);
        }
        let word = token.trim_end_matches(['.', ',', ';', ':', ')']).to_lowercase();
        if section_number(token) || REFERENCE_WORDS.contains(&word.as_str()) {
            if token.ends_with(':') {
                return (&t[end - 1..], true);
            }
            s = &t[end..];
            stripped = true;
            continue;
        }
        if !lot && LOT_DESIGNATORS.contains(&word.as_str()) {
            // The designator, up to two number marks, and the number.
            let mut after = &t[end..];
            for _ in 0..3 {
                let u = after.trim_start();
                let end = u.find(char::is_whitespace).unwrap_or(u.len());
                let next = &u[..end];
                if section_number(next) || lettered_lot_number(next) {
                    if next.ends_with(':') {
                        return (&u[end - 1..], true);
                    }
                    s = &u[end..];
                    stripped = true;
                    lot = true;
                    break;
                }
                if !NUMBER_MARKS.contains(&next.trim_end_matches(['.', ':']).to_lowercase().as_str()) {
                    break;
                }
                after = &u[end..];
            }
            if lot {
                continue;
            }
        }
        return (t, stripped);
    }
}

/// A lot number with a sub-lot letter (`1A`, `12b)`, `3a:`): digits, then one letter, with
/// trailing punctuation — after a lot designator only ([`strip_references`]).
fn lettered_lot_number(token: &str) -> bool {
    let core = token.trim_end_matches(['.', ',', ';', ':', ')']);
    let digits = core.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0 && core.len() == digits + 1 && core.as_bytes()[digits].is_ascii_alphabetic()
}

/// A section number token (`VI.4.1`, `VI.3)`, `1)`, `II.2.4.`, `6`): dot-separated parts,
/// each digits or a [`ROMAN`] numeral, with trailing punctuation.
fn section_number(token: &str) -> bool {
    let core = token.trim_end_matches(['.', ',', ';', ':', ')']);
    !core.is_empty()
        && core.split('.').all(|part| {
            !part.is_empty()
                && (part.bytes().all(|b| b.is_ascii_digit()) || ROMAN.contains(&part.to_lowercase().as_str()))
        })
}

/// `s` after the separator it opens with (`:`, `(`, an en or em dash, or a hyphen followed
/// by a space), or `None`.
fn open_separator(s: &str) -> Option<&str> {
    let t = s.trim_start();
    let c = t.chars().next()?;
    match c {
        ':' | '(' | '–' | '—' => Some(&t[c.len_utf8()..]),
        '-' if t[1..].chars().next().is_none_or(char::is_whitespace) => Some(&t[1..]),
        _ => None,
    }
}

/// Whether the text after a pointer names something: a word of two or more letters that
/// starts with a capital (`NEB`, `Vergabekammer`, `VKS`), or a legal form (`mbH`, `e.V.`,
/// [`company_token`]'s forms). A Roman numeral with digits glued on is a malformed section
/// reference, not a name (`See Section VI3.2`, `IV2`: [`roman_with_digits`]).
fn names_something(s: &str) -> bool {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !roman_with_digits(w))
        .any(|w| w.chars().next().is_some_and(char::is_uppercase) && w.chars().filter(|c| c.is_alphabetic()).count() >= 2)
        || company_token(s)
        || s.split_whitespace()
            .any(|t| matches!(t.trim_matches(|c: char| !c.is_alphanumeric() && c != '.'), "mbH" | "e.V." | "e.V" | "gGmbH"))
}

/// A [`ROMAN`] numeral with digits attached (`VI3`, `V1`, `IV2`): the letters, then nothing
/// but ASCII digits.
fn roman_with_digits(word: &str) -> bool {
    let letters = word.bytes().take_while(u8::is_ascii_alphabetic).count();
    letters > 0
        && letters < word.len()
        && word[letters..].bytes().all(|b| b.is_ascii_digit())
        && ROMAN.contains(&word[..letters].to_ascii_lowercase().as_str())
}

/// Issue 511: a name shaped like a summary of several parties — a company form
/// ([`company_token`], [`polish_company_form`]), several numbered lots ([`several_lots`]),
/// numbered or lettered entries ([`numbered_entries`], [`lettered_entries`]). It guards
/// every non-void stem: a stem inside such a name is an [`NotAName::AwardSummary`], kept by
/// the fold (`Lot 2: not applicable. Lot 1: ACME Ltd`, `Nie dotyczy (część 2); część 1: X
/// sp. z o.o.`). [`names_an_award`] without its award-word tests ([`award_clause`],
/// [`awardee_noun`]): those would read "the list of **awarded** suppliers" inside a
/// pointer as an award. A list of placeholders only stays a summary too, erring toward
/// keeping the party as 510's exemption does.
fn summary_shape(raw: &str, folded: &str) -> bool {
    let words: Vec<&str> = folded.split(' ').collect();
    company_token(raw)
        || polish_company_form(&format!(" {folded}"))
        || several_lots(&words)
        || numbered_entries(raw)
        || lettered_entries(raw)
}

/// A Polish company form on the space-padded fold (`Olprint Sp. z o.o.`, `spółka`).
fn polish_company_form(padded: &str) -> bool {
    [" sp z o o", " sp z oo", " spolka ", " spolki "].iter().any(|form| format!("{padded} ").contains(form))
}

/// Whether `name` says the lot was not awarded ([`NotAName::VoidLot`]) — the fold's rule for
/// ONE name, as a plain predicate for injection. The fold drops a PARTY by all its copies
/// ([`crate::project::fold_parties`]), which issue 525's `refold-void-names` walk asks about
/// every mention; this decides only the walk's orgs mentioned nowhere.
pub fn is_void_lot(name: &str) -> bool {
    not_a_name(name) == Some(NotAName::VoidLot)
}

/// Whether `name` carries a void-lot stem or whole value at all, before the award
/// exemption ([`names_an_award`]) — the `refold-void-names` walk's net, so its dry plan
/// can list what the exemption keeps beside what the fold drops.
pub fn mentions_void(name: &str) -> bool {
    // Issue 527: the award mark is no void phrase ([`award_annotation`]).
    if let Some((base, _)) = award_annotation(name) {
        return mentions_void(base);
    }
    let full = fold(name);
    if full.is_empty() {
        return false;
    }
    let padded = format!(" {full}");
    if VOID_STEMS.iter().any(|stem| padded.contains(stem)) {
        return true;
    }
    let qualifier = lot_qualifier(name);
    let stripped = qualifier.map(|(head, _)| fold(head));
    let whole = stripped.as_deref().unwrap_or(&full);
    // Issue 525: a template only as [`classify`] reads one (whole, or before a BARE lot
    // qualifier), since `names_an_award` asks this whether an entry is void: `procedura
    // infruttuosa (lotto 3 Rossi Costruzioni)` names a winner and is no void entry.
    let template = VOID_TEMPLATES.contains(&whole) && qualifier.is_none_or(|(_, group)| bare_lot_qualifier(group));
    VOID_WHOLE.contains(&whole)
        || template
        || template_pointed(name)
        || reason_head(name).is_some_and(|head| VOID_WHOLE.contains(&fold(head).as_str()))
        || void_prefix(name)
}

/// A void whole value opening the name and running straight on into its reason: the next
/// character is punctuation (`BRAK OFERT-unieważniona …`, `Desierto / Ninguna oferta`), the
/// next word is lowercase (`Desierto al no haberse presentado ninguna oferta`, `Lotto deserto
/// in quanto …`, `Brak ofert na zadanie nr 1`), a [`CONTINUATIONS`] word in any case
/// (`DESIERTO POR NO PRESENTARSE LICITADORES`), or a TED section number spilled into the slot
/// (`Brak ofert V.1) Award and contract value …`). `Desierto OÜ` — an uppercase word after it —
/// is a company's name.
fn void_prefix(name: &str) -> bool {
    let name = name.trim();
    // Every place a word ends (an alphanumeric run before a non-alphanumeric char), the first
    // dozen: the head can end at a space or at glued punctuation.
    let ends: Vec<usize> = name
        .char_indices()
        .zip(name.char_indices().skip(1))
        .filter(|((_, a), (_, b))| a.is_alphanumeric() && !b.is_alphanumeric())
        .map(|(_, (at, _))| at)
        .take(12)
        .collect();
    for end in ends {
        if !VOID_WHOLE.contains(&fold(&name[..end]).as_str()) {
            continue;
        }
        let rest = &name[end..];
        let next = rest.trim_start();
        let Some(first) = next.chars().next() else { return false };
        let glued = !rest.starts_with(char::is_whitespace);
        let word: String = next.chars().take_while(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
        let section = word == "section" || ["v.", "vi.", "iv.", "ii."].iter().any(|m| next.to_lowercase().starts_with(m));
        if glued || !first.is_alphanumeric() || first.is_lowercase() || CONTINUATIONS.contains(&word.as_str()) || section {
            return true;
        }
    }
    false
}

/// Issue 525: one of TED's templates ([`VOID_TEMPLATES`]) followed by TED's own pointer to the
/// form's additional information, in one bracketed group that closes the name — 12381194's ORG-4
/// prints `Lot not awarded (see section VI, point VI.2 'Additional information').` in 23 languages
/// (`Los wurde nicht vergeben (siehe Abschnitt VI Ziffer VI.2 „Sonstige Informationen“).`). The
/// group is the pointer's translation shape ([`translation_shaped`]: a pointer word first, a dozen
/// words at most, no legal form), holds a form section reference ([`has_form_reference`]) and names
/// nothing after it ([`names_after_reference`]); a template followed by anything else is no
/// template.
fn template_pointed(name: &str) -> bool {
    let trimmed = name.trim().trim_end_matches(|c: char| c.is_whitespace() || ".,;:".contains(c));
    let Some(open) = trimmed.find(['(', '[']) else { return false };
    let Some(inner) = trimmed[open + 1..].strip_suffix([')', ']']) else { return false };
    // ONE group closing the name: a second opener means text sits between two groups (`… (siehe
    // VI.2). Los 2: Müller Bau (siehe VI.3)`), which this rule must never read past.
    if inner.contains(['(', '[']) {
        return false;
    }
    VOID_TEMPLATES.contains(&fold(&trimmed[..open]).as_str())
        && has_form_reference(inner)
        && translation_shaped(inner)
        && !names_after_reference(inner)
}

/// Words that continue a void phrase into its reason, in any case (all-caps names hide the
/// lowercase cue): `DESIERTO POR NO PRESENTARSE LICITADORES`, `NOT AWARDED DUE TO …`.
const CONTINUATIONS: [&str; 24] = [
    "por", "al", "en", "de", "del", "per", "in", "na", "for", "due", "to", "as", "pour", "faute", "par", "w", "z",
    "ze", "sin", "ya", "on", "nr", "da", "zgodnie",
];

/// The head of a name that goes on to give a reason: the text before the first `(`, `[`,
/// `:`, `;`, `,`, ` - `, ` – `, ` — ` or `. `, when something follows it. `None` for a name
/// with no such break.
fn reason_head(name: &str) -> Option<&str> {
    let name = name.trim();
    let mut cut = name.len();
    for sep in ["(", "[", ":", ";", ",", " - ", " – ", " — ", ". "] {
        if let Some(at) = name.find(sep) {
            cut = cut.min(at);
        }
    }
    let head = name[..cut].trim();
    (cut < name.len() && !head.is_empty() && name[cut..].chars().any(char::is_alphanumeric)).then_some(head)
}

/// Whether a name that carries a void stem also names an award (issue 510's drain review):
/// - [`award_clause`]: the lot was then awarded (`puis attribué à`, `avec la société`);
/// - an awardee noun not negated ([`AWARD_NOUNS`]: `mandataire`, `titulaire`, …) that labels
///   a party ([`award_awardee_noun`]);
/// - [`several_lots`]: a per-lot summary (`Lot 1) X. Lot 2) Infructueux`), one of whose lots
///   is not void ([`award_several_lots`]);
/// - [`mixed_segments`]: `;`-separated entries, one with no void phrase (`zone A : Firm ;
///   zone B : infructueux`) and no reason ([`award_mixed_segments`]);
/// - a company form among its raw tokens ([`COMPANY_TOKENS`]) that is no bidder's
///   ([`award_company_token`]).
///
/// Each errs toward keeping the party, which costs a junk name and never a real award. Issue
/// 520's unit 1 (`.scratch/tender-db/520-award-summaries/unit1-decision.md` §4.1) read the 376
/// names the void walk kept: 185 were void prose these signals misread, so seven narrowings
/// (D1–D7) quiet them inside this function's own arms — one lot's BOAMP void sentence
/// ([`lot_title_void`]), a negated, earlier or intended award ([`award_clause`]), an awardee
/// word that labels nobody, a list of void lots (`Les lots 2 et 11 sont déclarés sans suite.
/// Le lot 10 est déclaré infructueux`, kept until then), a reason segment, a bidder, and a
/// void head before a spilled section ([`named_before_spill`]). The shared shape helpers 511
/// reads stay as they were ([`summary_shape`], [`named_before_stem`], [`names_a_legal_form`],
/// [`names_something`]).
fn names_an_award(raw: &str, folded: &str) -> bool {
    if lot_title_void(raw, folded) {
        return false;
    }
    let words: Vec<&str> = folded.split(' ').collect();
    let padded = format!(" {folded}");
    award_clause(raw, folded)
        || award_awardee_noun(raw, &words)
        || award_several_lots(raw, &words)
        || award_mixed_segments(raw)
        || award_company_token(raw)
        || polish_company_form(&padded)
        || AWARD_PHRASES.iter().any(|phrase| padded.contains(phrase))
        || only_an_earlier_void(&padded)
        || SUPPLIER_HEADINGS.iter().any(|heading| {
            padded.find(heading).is_some_and(|at| padded[at + heading.len()..].chars().any(char::is_alphabetic))
        })
        || address(raw, &words)
        || rebate(raw)
        || named_before_spill(raw, folded)
        || labelled_entries(raw)
}

/// Parallel `label : value` entries split at `;`, ` - ` or ` / ` — labels with one first word
/// — one value void and another naming something (`Agence d'Amboise : appel d'offres déclaré
/// sans suite - Agence de Chinon : savelys GDF suez`). Parallel, because a lot's own sentence
/// carries `label : value` details too (`Le lot 2: béton. … seuil minimum Ht par an: 2 508,36
/// EUR / seuil maximum … a été déclaré infructueux`).
fn labelled_entries(raw: &str) -> bool {
    let mut void: Vec<String> = Vec::new();
    let mut named: Vec<String> = Vec::new();
    for entry in raw.split(';').flat_map(|e| e.split(" - ")).flat_map(|e| e.split(" / ")) {
        let Some((label, value)) = entry.split_once(':') else { continue };
        let Some(head) = fold(label).split(' ').next().filter(|h| !h.is_empty()).map(str::to_owned) else { continue };
        if !value.chars().any(char::is_alphabetic) {
            continue;
        }
        let folded = fold(value);
        let padded = format!(" {folded}");
        if mentions_void(value) {
            void.push(head);
        } else if !VOID_WHOLE.contains(&folded.as_str())
            && ![" relanc", " nouvelle consultation", " nueva licitaci", " nuova gara"].iter().any(|n| padded.contains(n))
        {
            named.push(head);
        }
    }
    void.iter().any(|head| named.contains(head))
}

/// Every void stem of the padded fold sits inside an [`EARLIER_VOID`] phrase.
fn only_an_earlier_void(padded: &str) -> bool {
    let mut rest = padded.to_owned();
    let mut found = false;
    for phrase in EARLIER_VOID {
        while let Some(at) = rest.find(phrase) {
            found = true;
            rest.replace_range(at..at + phrase.len(), " ");
        }
    }
    found && !VOID_STEMS.iter().any(|stem| rest.contains(stem))
}

/// A five-digit postcode beside a [`STREET_WORDS`] word.
fn address(raw: &str, words: &[&str]) -> bool {
    let postcode = raw.split(|c: char| !c.is_alphanumeric()).any(|t| t.len() == 5 && t.bytes().all(|b| b.is_ascii_digit()));
    postcode && words.iter().any(|w| STREET_WORDS.contains(w))
}

/// A list of numbered entries (`lot 43 INFRUCTUEUX - 44 reckitt benckiser 28692.07 - 45 …`,
/// `lot268 sanofi av - 269sanofi AV - …`, `1/ Gidef, … Bondy - 2/ Auxet, …`): entries split at
/// `;` and at a `-` or `/` with space on one side (not the `/` of `1/`), two or more of
/// them led by distinct numbers, and one of those naming something after its number.
fn numbered_entries(raw: &str) -> bool {
    let chars: Vec<char> = raw.chars().collect();
    let mut entries: Vec<String> = vec![String::new()];
    for (i, &c) in chars.iter().enumerate() {
        let before = i.checked_sub(1).map(|j| chars[j]);
        let after = chars.get(i + 1).copied();
        let spaced = before.is_some_and(char::is_whitespace) || after.is_some_and(char::is_whitespace);
        let split = c == ';'
            || (c == '-' && spaced)
            || (c == '/' && spaced && !before.is_some_and(|b| b.is_ascii_digit()));
        if split {
            entries.push(String::new());
        } else {
            entries.last_mut().expect("one entry").push(c);
        }
    }
    let mut numbers: Vec<&str> = Vec::new();
    let mut names_something = false;
    for entry in &entries {
        let mut tokens = entry.split_whitespace().peekable();
        if tokens.peek().is_some_and(|t| LOT_WORDS.contains(&t.to_lowercase().as_str())) {
            tokens.next();
        }
        let Some(first) = tokens.next() else { continue };
        // Glued: `lot268`, `Lots12` (the designator that leaves digits).
        let lower = first.to_lowercase();
        let first = LOT_WORDS
            .iter()
            .filter(|w| lower.starts_with(**w) && first.is_char_boundary(w.len()))
            .map(|w| &first[w.len()..])
            .find(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
            .unwrap_or(first);
        let digits = first.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            continue;
        }
        // A sub-lot letter right after the number is part of it (`3a:` and `3b:` are two lots;
        // `269sanofi` is lot 269 of Sanofi).
        let lettered = first[digits..].chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && !first[digits..].chars().nth(1).is_some_and(char::is_alphabetic);
        let end = if lettered { digits + 1 } else { digits };
        numbers.push(&first[..end]);
        let rest: String = std::iter::once(&first[end..]).chain(tokens).collect::<Vec<_>>().join(" ");
        if rest.chars().any(char::is_alphabetic) && !mentions_void(&rest) {
            names_something = true;
        }
    }
    numbers.sort_unstable();
    numbers.dedup();
    numbers.len() >= 2 && names_something
}

/// Two or more distinct entry markers `A)`, `b)`, `B1)` at an entry start (`Lot A) Rivadis.
/// Lot B) Ontex. … Lot E) infructueux`, `A) Coca à Lille. B) Placé: sans suite. C) SRTT …`).
fn lettered_entries(raw: &str) -> bool {
    let chars: Vec<char> = raw.chars().collect();
    let mut markers: Vec<String> = Vec::new();
    for (i, &c) in chars.iter().enumerate() {
        if c != ')' {
            continue;
        }
        let start = (0..i).rev().take_while(|&j| chars[j].is_ascii_alphanumeric()).last().unwrap_or(i);
        let marker: String = chars[start..i].iter().collect();
        let boundary = start == 0 || matches!(chars[start - 1], ' ' | '.' | ';' | ':' | '\n');
        // `A)`, `B1)`, and the sub-lot `3a)` / `3b)`; never a bare number (`Lot 2)`).
        let letter_first = matches!(marker.len(), 1 | 2)
            && marker.chars().next().is_some_and(|m| m.is_ascii_alphabetic())
            && marker.chars().skip(1).all(|m| m.is_ascii_digit());
        let digits = marker.chars().take_while(char::is_ascii_digit).count();
        let sub_lot = (1..=3).contains(&digits)
            && marker.len() == digits + 1
            && marker.chars().last().is_some_and(|m| m.is_ascii_alphabetic());
        let shaped = letter_first || sub_lot;
        if boundary && shaped {
            markers.push(marker.to_lowercase());
        }
    }
    markers.sort_unstable();
    markers.dedup();
    markers.len() >= 2
}

/// A rebate or price variation in parentheses (`ACS DECO (+5 %) et non attribué
/// (infructueux)`): an awarded bid's terms.
fn rebate(raw: &str) -> bool {
    raw.split('(').skip(1).any(|group| {
        let inner = group.split(')').next().unwrap_or("").trim();
        let inner = inner.trim_start_matches(['+', '-', '−']).trim_start();
        inner.ends_with('%')
            && inner.trim_end_matches('%').trim().chars().all(|c| c.is_ascii_digit() || c == ',' || c == '.')
            && inner.chars().any(|c| c.is_ascii_digit())
    })
}

/// The award notice's next section spilled into the slot after the winner's name (`Zundel Et
/// Kohler. SECTION VI: OTHER INFORMATION … VI.7) Other information: Lots Infructueux`): the
/// head before `section vi` names a party when it says nothing void. Issue 520 (D7): nothing
/// void as [`mentions_void`] reads it — a void whole value, reason head or prefix too, not only
/// a stem — once a trailing `see` is off, since the text era's spills open with the void phrase
/// itself (`Desierto. SECTION VI: OTHER INFORMATION VI.3) Date of contract award: 2.1.2004`, `Gara
/// deserta per mancanza di offerte valide. SECTION VI`, `No contract awarded. SECTION VI: …
/// Limerick City Council`) and TED's pointer names the section (`No Contract awarded, see section
/// VI.2`). What follows the head's last `:` or `;` is read alone too: a name there ([`names_something`])
/// that says nothing void and gives no reason (a negation, an offer: `Desierto: Ninguna oferta`)
/// keeps the party — where the head holds no void stem, so 510 kept it too (the second review: the
/// arm only ever quiets, and `Infructueux : Voir section VI.3` stays void).
fn named_before_spill(raw: &str, folded: &str) -> bool {
    let padded = format!(" {folded} ");
    let Some(at) = padded.find(" section vi ") else { return false };
    let runs = alphanumeric_runs(raw);
    if runs.len() != folded.split(' ').count() {
        // The fold's words and the raw runs do not line up: the 510 reading, erring toward a name.
        let head = &padded[..at];
        return head.chars().any(char::is_alphabetic) && !VOID_STEMS.iter().any(|stem| head.contains(stem));
    }
    // The head's words, by the fold's words (one per alphanumeric run of `raw`), a trailing `see` off.
    let mut words = padded[..at].split_whitespace().count();
    if words > 0 && fold(&raw[runs[words - 1].0..runs[words - 1].1]) == "see" {
        words -= 1;
    }
    let head = if words == 0 { "" } else { &raw[..runs[words - 1].1] };
    // The review: a name after the head's last `:` or `;` is the winner the void phrase led to (`Not
    // awarded in first round, re-tendered: Acme Construction. SECTION VI`) — only where the 510
    // reading already keeps the name, its head holding no void stem (the second review: D7 only ever
    // quiets; `Infructueux : Voir section VI.3`, `Infructueux : LOT 3. SECTION VI` and `Lot déclaré
    // infructueux : Relance en procédure négociée. SECTION VI` stay void, as 510 read them).
    let base_keeps = !VOID_STEMS.iter().any(|stem| padded[..at].contains(stem));
    let last = head.rsplit([':', ';']).next().unwrap_or(head);
    let reason =
        fold(last).split(' ').any(|w| NEGATIONS.contains(&w) || OFFER_WORDS.iter().any(|stem| w.starts_with(stem)));
    let named_last = base_keeps && last.len() < head.len() && names_something(last) && !mentions_void(last) && !reason;
    head.chars().any(char::is_alphabetic) && (!mentions_void(head) || named_last)
}

/// A company form among the raw tokens ([`COMPANY_TOKENS`], [`COMPANY_FORMS_DOTTED`]), each
/// trimmed of surrounding punctuation (`Yvelin SA.`, `«SARL»`) and compared without its dots.
/// `SA` before a [`POSSESSIVE_NEXT`] word is the possessive in capitals, not a company.
fn company_token(raw: &str) -> bool {
    let tokens: Vec<&str> = raw
        .split(|c: char| c.is_whitespace() || ",;:()/[]".contains(c))
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|t| !t.is_empty())
        .collect();
    tokens.iter().enumerate().any(|(i, token)| {
        let bare: String = token.chars().filter(|c| *c != '.').collect();
        let dotted = token.contains('.');
        let form = COMPANY_TOKENS.contains(&bare.as_str())
            || (dotted && COMPANY_FORMS_DOTTED.contains(&bare.to_lowercase().as_str()));
        form && !(bare == "SA" && tokens.get(i + 1).is_some_and(|next| POSSESSIVE_NEXT.contains(&fold(next).as_str())))
    })
}

/// An [`AWARD_NOUNS`] word not negated: the nearest word before it that is not a
/// [`NEGATION_FILLERS`] word is not a [`NEGATIONS`] word (`pas d'attributaire`, `no hay
/// adjudicatario`).
fn awardee_noun(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(i, w)| AWARD_NOUNS.contains(w) && !negated(&words[..i]))
}

/// Whether the word after `before` is negated: the nearest word of `before` that is not a
/// [`NEGATION_FILLERS`] word is a [`NEGATIONS`] word (`pas d'attributaire`, `no hay
/// adjudicatario`, `Le marché n'a pas été attribué`).
fn negated(before: &[&str]) -> bool {
    before.iter().rev().find(|b| !NEGATION_FILLERS.contains(*b)).is_some_and(|b| NEGATIONS.contains(b))
}

/// Issue 520 (D3): [`awardee_noun`] where the word names the party — as a label (`Attributaire:
/// ARDP`, `Nom du titulaire: Keolis`, `titulaire 1: SAGA titulaire 2: Chapelec`, `fournisseur
/// retenu = zoll medical France`: [`labels`]), never `attribution`, whose label heads a date (`Date
/// d'attribution : CAO du 17.4.2013`); as `(mandataire)`; as `attribution à` (`Lot 3) attribution
/// à: SA Maîtres Laitiers Distribution`); as a participle that names its party
/// ([`participle_names`]: `lote 1 adjudicado a Construcciones Pérez`, `la société Storz a été
/// retenue`); or as a noun right before the party (`titulaire Dupont Frères`, `Resultó
/// adjudicataria la empresa Construcciones Pérez`: [`noun_names`]). The descriptive and
/// hypothetical uses name nobody: `en cours d'attribution`, `Voir l'avis d'attribution … pour
/// l'entreprise attributaire du lot`, `ne permettent pas d'en établir l'attribution`,
/// `l'attributaire pressenti`, `la liquidation judiciaire de la société attributaire`,
/// `accord-cadre multi-attributaire`, `désistement du titulaire`, `l'offre retenue`, `propuesto
/// adjudicatario`, `awarded as part of a new project`. 510's negation test stays ([`negated`]).
fn award_awardee_noun(raw: &str, words: &[&str]) -> bool {
    let runs = alphanumeric_runs(raw);
    if runs.len() != words.len() {
        // The fold's words and the raw runs do not line up: the 510 reading, erring toward a name.
        return awardee_noun(words);
    }
    awardee_noun(words)
        && words.iter().enumerate().any(|(i, w)| {
            let (from, to) = runs[i];
            AWARD_NOUNS.contains(w)
                && !negated(&words[..i])
                && if *w == "attribution" {
                    words.get(i + 1) == Some(&"a")
                } else {
                    labels(&raw[to..], !AWARD_PARTICIPLES.contains(w))
                        || (*w == "mandataire" && raw[..from].trim_end().ends_with('(') && closes_group(&raw[to..]))
                        || if AWARD_PARTICIPLES.contains(w) {
                            participle_names(raw, &runs, words, i)
                        } else {
                            noun_names(raw, &runs, words, i)
                        }
                }
        })
}

/// Whether the text right after a word makes the word a label: a `:`, `=` or full-width `：` — or a
/// spaced dash before a capitalised word (`Attributaire - Keolis`) — after an optional plural mark,
/// entry number and short complement of up to six words ([`LABEL_COMPLEMENTS`]: `titulaire 1: SAGA`,
/// `Titulaire(s) :`, `titulaire n°1 :`, `Titulaire du marché :`, `Nom du titulaire / organisme :`).
/// For a noun (`open`), since the second review, the complement may hold any word that is no
/// negation, copula or awardee word (`Titulaire du marché négocié :`, `Attributaire désigné :`,
/// `attributaire unique :`; not `Attributaire non designe - …`); a participle's keeps the closed
/// list (`wordt gegund op totaal der percelen = perceel 3` names nobody).
fn labels(after: &str, open: bool) -> bool {
    let mut rest = after;
    let mut words = 0;
    loop {
        rest = rest.trim_start();
        if let Some(r) = ["(s)", "(S)", "(es)"].iter().find_map(|mark| rest.strip_prefix(mark)) {
            rest = r;
            continue;
        }
        if let Some(r) = rest.strip_prefix(['/', '\'', '’', '°', 'º']) {
            rest = r;
            continue;
        }
        let run = rest.find(|c: char| !c.is_alphanumeric()).unwrap_or(rest.len());
        if run == 0 || words == 6 {
            break;
        }
        let word = fold(&rest[..run]);
        let word = word.as_str();
        let listed = digits_only(word) || NUMBER_MARKS.contains(&word) || LABEL_COMPLEMENTS.contains(&word);
        let free = open && !NEGATIONS.contains(&word) && !COPULAS.contains(&word) && !AWARD_NOUNS.contains(&word);
        if !(listed || free) {
            break;
        }
        rest = &rest[run..];
        words += 1;
    }
    rest.starts_with([':', '=', '：'])
        || ["- ", "– ", "— "].iter().any(|dash| {
            rest.strip_prefix(dash).is_some_and(|r| r.trim_start().chars().next().is_some_and(char::is_uppercase))
        })
}

/// Whether `(mandataire` closes its group: `)` next, or one word and then `)` (`(mandataire
/// solidaire)`).
fn closes_group(after: &str) -> bool {
    let rest = after.trim_start();
    let rest = rest.trim_start_matches(char::is_alphabetic).trim_start();
    rest.starts_with(')')
}

/// Issue 520 (D3): whether the award participle `words[i]` ([`AWARD_PARTICIPLES`]) names its party:
/// its language's preposition within [`PARTICIPLE_REACH`] words in its clause ([`PARTY_PREPOSITIONS`]:
/// `lote 1 adjudicado a`, `gegund op 12-03-2014 aan Bouwbedrijf Jansen`, `awarded on 12.3.2014 to`);
/// for `retenu(e)`, a copula before it with a subject that is no offer (`la société Storz a été
/// retenue`) or after it before the party ([`party_after_copula`]: `l'entreprise retenue est Dupont
/// Frères`, `l'entreprise retenue est la société Dupont`), or a capitalised name right before it (`:
/// Keolis retenu`). `wordt gegund op totaal der percelen`,
/// `awarded as part of a new project`, `l'offre retenue` and `Pas d'attributaire retenu` name nobody.
fn participle_names(raw: &str, runs: &[(usize, usize)], words: &[&str], i: usize) -> bool {
    let w = words[i];
    let gap = |k: usize| &raw[runs[k - 1].1..runs[k].0];
    if let Some((_, prepositions)) = PARTY_PREPOSITIONS.iter().find(|(start, _)| w.starts_with(start)) {
        return (i + 1..words.len().min(i + PARTICIPLE_REACH + 2))
            .take_while(|&k| !clause_break(gap(k)))
            .any(|k| prepositions.contains(&words[k]));
    }
    let offer = |word: &str| {
        OFFER_WORDS.iter().any(|stem| word.starts_with(stem))
            || word.starts_with("candidat")
            || AWARD_NOUNS.contains(&word)
            || LOT_DESIGNATORS.contains(&word)
    };
    // A copula before it: the subject is the nearest word before the copula and its auxiliary.
    let copula = i > 0 && COPULAS.contains(&words[i - 1]);
    let subject = words[..i.saturating_sub(1)].iter().rev().find(|b| !["a", "ont", "n", "ne"].contains(*b));
    let capitalised = |k: usize| raw[runs[k].0..].chars().next().is_some_and(char::is_uppercase);
    (copula && subject.is_some_and(|s| !offer(s)))
        || (words.get(i + 1).is_some_and(|n| COPULAS.contains(n)) && party_after_copula(raw, runs, words, i + 2))
        || (i > 0 && capitalised(i - 1) && gap(i).trim().is_empty() && !offer(words[i - 1]))
}

/// Issue 520 (D3, the second review): whether the words from `words[k]`, right after a copula, name
/// the party — after an optional article, a company word ([`COMPANY_NOUNS`]: `est la société Dupont`,
/// `est l'entreprise Dupont Frères`) or a capitalised word that is no lot, negation or void word (`est
/// Dupont Frères`). `le titulaire est défaillant` names nobody.
fn party_after_copula(raw: &str, runs: &[(usize, usize)], words: &[&str], k: usize) -> bool {
    let capitalised = |k: usize| raw[runs[k].0..].chars().next().is_some_and(char::is_uppercase);
    let article = words.get(k).is_some_and(|w| matches!(*w, "la" | "le" | "l" | "les"));
    if article && k + 1 >= words.len() {
        // A name cut right after its article (`… le candidat retenu est La`): the capitalised article
        // is all that is left of the party, as the copula's first reading took it.
        return capitalised(k);
    }
    let k = if article { k + 1 } else { k };
    // A lot designator is no party only as a lot, with its number (`est lot 2`); a name that opens on
    // the word (`Zone Ouest Transports`, `Secteur Nord BTP`) is one (the third review).
    let a_lot = LOT_DESIGNATORS.contains(&words[k.min(words.len().saturating_sub(1))])
        && words.get(k + 1).is_some_and(|n| n.bytes().any(|b| b.is_ascii_digit()) || ROMAN.contains(n));
    k < words.len()
        && (COMPANY_NOUNS.contains(&words[k])
            || (capitalised(k)
                && !a_lot
                && !NEGATIONS.contains(&words[k])
                && !mentions_void(&raw[runs[k].0..runs[k].1])))
}

/// Issue 520 (D3): whether the awardee noun `words[i]` is followed by its party — a capitalised word
/// right after it that is no lot, void or negation word (`titulaire Dupont Frères`), a company
/// word after an optional article ([`COMPANY_NOUNS`]: `adjudicataria la empresa Construcciones
/// Pérez`, `aggiudicataria la ditta Rossi`), or — since the second review — a copula and the party
/// ([`party_after_copula`]: `le titulaire est Dupont Frères`, `l'attributaire est l'entreprise Dupont
/// Frères`). `l'entreprise attributaire du lot`, `1 attributaire ne convient pas`, `Attributaire non
/// designe` and `le titulaire est défaillant` name nobody.
fn noun_names(raw: &str, runs: &[(usize, usize)], words: &[&str], i: usize) -> bool {
    let Some(&next) = words.get(i + 1) else { return false };
    let spaced = raw[runs[i].1..runs[i + 1].0].trim().is_empty();
    let capitalised = raw[runs[i + 1].0..].chars().next().is_some_and(char::is_uppercase);
    let company = |k: usize| words.get(k).is_some_and(|w| COMPANY_NOUNS.contains(w));
    (spaced
        && capitalised
        && !LOT_DESIGNATORS.contains(&next)
        && !NEGATIONS.contains(&next)
        && !mentions_void(&raw[runs[i + 1].0..runs[i + 1].1]))
        || company(i + 1)
        || (["la", "el", "il", "lo", "le", "l"].contains(&next) && company(i + 2))
        || (COPULAS.contains(&next) && party_after_copula(raw, runs, words, i + 2))
}

/// Issue 520 (the review): whether the text between two words ends a clause — a `,`, `;`, `:`,
/// bracket, ellipsis, line break, dash, spaced hyphen or a `.` that a space follows. A cut too many
/// only shortens what one word reads (a negation, a participle's reach, an offer's rejection).
fn clause_break(gap: &str) -> bool {
    gap.contains([',', ';', ':', '(', ')', '[', ']', '…', '–', '—', '\n'])
        || gap.contains(". ")
        || gap.contains(" - ")
}

/// Issue 520 (the second review): whether the text between two words ends a sentence, as
/// [`sentences`] cuts one — a `;`, a line break, an ellipsis or a `.` that a space follows. Only the
/// plain ends: a cut too few only lets `ne … que` restrict across it (an award read, as 510 read it).
fn sentence_end(gap: &str) -> bool {
    gap.contains([';', '\n', '…']) || gap.contains(". ")
}

/// Two or more DISTINCT numbered lot designators (`lot 1 … lot 3`, `sous lot 5b … lot 6`,
/// `lote nº 2 … lote 3`, `Lote I … Lote II`, `lot1 … lot2`): the name is a per-lot summary,
/// not one lot's outcome. One lot named twice (`Lot 3) Ce lot 3 est déclaré infructueux`) is
/// one lot. A number is the whole run of digit words after the designator, because the fold
/// splits `6.01.01` and `2002-03` into words (`Lot 6.01.01 : PARAMAT … Lot 6.01.06 : sans
/// suite` is six lots, not lot 6 six times).
fn several_lots(words: &[&str]) -> bool {
    let number = |w: &str| w.bytes().any(|b| b.is_ascii_digit()) || ROMAN.contains(&w);
    // Numbers count within one designator family: `le lot 66 … secteur 6 a été déclaré
    // infructueux` is one lot of one sector, not a summary.
    fn family(w: &str) -> &str {
        match w {
            "secteur" | "secteurs" => "secteur",
            "zone" | "zones" => "zone",
            "tranche" | "partie" | "partida" | "rang" => w,
            "perceel" | "percelen" | "partij" | "partijen" => "perceel",
            _ => "lot",
        }
    }
    let mut numbers: Vec<String> = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if LOT_DESIGNATORS.contains(w) {
            // Skip up to two number marks or void words (`lot nº 3`, `lots infructueux : 1501`).
            let mut j = i + 1;
            while j < words.len()
                && j <= i + 2
                && (NUMBER_MARKS.contains(&words[j])
                    || VOID_STEMS.iter().any(|stem| stem.trim_start().split(' ').next().is_some_and(|head| words[j].starts_with(head))))
            {
                j += 1;
            }
            if let Some(n) = words.get(j).filter(|n| number(n)) {
                let mut whole = format!("{}:{n}", family(w));
                for more in words[j + 1..].iter().take_while(|w| !w.is_empty() && w.bytes().all(|b| b.is_ascii_digit())) {
                    whole.push('.');
                    whole.push_str(more);
                }
                numbers.push(whole);
            }
        } else if let Some(rest) = LOT_DESIGNATORS
            .iter()
            .find_map(|d| w.strip_prefix(d).filter(|r| !r.is_empty() && r.bytes().all(|b| b.is_ascii_digit())))
        {
            // Glued: `lot1`, `lote12`.
            numbers.push(format!("lot:{rest}"));
        }
    }
    numbers.sort_unstable();
    numbers.dedup();
    let several = numbers.windows(2).any(|pair| pair[0].split(':').next() == pair[1].split(':').next());
    // Sector or sub-lot codes without a designator each (`Spie secteurs 5a à 5d infructueux 5e
    // 5f Marc Elec 5g Chenelec 5h`): three or more distinct `<digits><letter>` codes.
    let mut codes: Vec<&str> = words
        .iter()
        .copied()
        .filter(|w| {
            let digits = w.bytes().take_while(u8::is_ascii_digit).count();
            digits > 0 && w.len() == digits + 1 && w.as_bytes()[digits].is_ascii_lowercase()
        })
        .collect();
    codes.sort_unstable();
    codes.dedup();
    several || codes.len() >= 3
}

/// Issue 520 (D4): [`several_lots`] as an award reads it — a list names an award only when one of
/// its lots is not void:
/// - a number mark glued to its number is the number (`Le lot no 3 a été déclarée infructueux. Le
///   présent avis ne concerne que le lot no3`: one lot);
/// - the lot designators after a relaunch marker number the new procedure's lots
///   ([`words_before_relaunch`]: `Ce lot a été décomposé en 2 nouveaux lots: lot nº 06 — …, lot nº 07
///   — …`, `lancer une nouvelle procédure … lot no 3 - agence Paul Signac lot no 4 - agence
///   Beaumonts`);
/// - a list whose every lot entry is void is void ([`every_lot_void`]: `lot 9: Sans suite Lot 10:
///   Infructueux`, `Les lots n° 1 et 3 ont été déclarés infructueux. Le lot n° 2 a été déclaré sans
///   suite`, `Perceel 2 en Perceel 4 worden niet gegund`).
///
/// `several_lots` must hold on the whole fold too (`words`), so the arm only ever quiets: a relaunch
/// cut joins what is left of its sentence to the next one, and `Lot relancé. 4 offres` must not
/// read as lot 4 (the review). [`numbered_entries`] and [`lettered_entries`] count as before: each
/// already needs an entry that names something, and an entry the designators miss (`lot 43
/// INFRUCTUEUX - 44 reckitt benckiser 28692.07`, `Lot 1 : Keolis / 2 : sans suite`) is one of theirs.
fn award_several_lots(raw: &str, words: &[&str]) -> bool {
    let own = words_before_relaunch(raw);
    let own: Vec<&str> = own.iter().map(String::as_str).collect();
    (several_lots(words) && several_lots(&own) && !every_lot_void(raw))
        || numbered_entries(raw)
        || lettered_entries(raw)
}

/// `raw`'s folded words without each sentence's relaunch (a [`RELAUNCH_MARKERS`] phrase and the
/// rest of its sentence: [`sentences`]), each glued number mark read as its number
/// ([`glued_number`]). A new entry of the summary ends the relaunch early — a lot label followed by
/// its `:` or `)` or by a capitalised winner ([`entry_label`]: `Lot 1 infructueux et relancé, lot 2 :
/// Dupont`, `…, lot 2 Dupont Frères, lot 3 Martin`), which the new procedure's lots are not written
/// with (`lot nº 06 — bâtiments démontables`).
fn words_before_relaunch(raw: &str) -> Vec<String> {
    let mut words = Vec::new();
    for sentence in sentences(raw) {
        let runs = alphanumeric_runs(sentence);
        let folded: Vec<String> = runs.iter().map(|&(from, to)| fold(&sentence[from..to])).collect();
        let folded: Vec<&str> = folded.iter().map(String::as_str).collect();
        let mut k = 0;
        while k < folded.len() {
            let padded = format!(" {}", folded[k..].join(" "));
            if RELAUNCH_MARKERS.iter().any(|marker| padded.starts_with(marker)) {
                match (k + 1..folded.len()).find(|&j| entry_label(sentence, &runs, &folded, j)) {
                    Some(j) => k = j,
                    None => break,
                }
                continue;
            }
            words.push(glued_number(folded[k]).to_owned());
            k += 1;
        }
    }
    words
}

/// A folded word that is a number mark glued to its number (`no3`, `nº12`) as the number;
/// any other word as it is.
fn glued_number(word: &str) -> &str {
    NUMBER_MARKS.iter().find_map(|mark| word.strip_prefix(mark).filter(|rest| digits_only(rest))).unwrap_or(word)
}

/// A non-empty run of ASCII digits.
fn digits_only(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// Issue 520 (D4c): whether `raw` lists lot entries and every one is void. An entry runs from a lot
/// designator with its number (as [`several_lots`] reads one, [`lot_label`]) to the next; the text
/// before the first is no entry. An entry holding only a conjunction after its label joins the next
/// one (`Perceel 2 en Perceel 4 worden niet gegund`, `Lot 1) et lot 2) Marché classé sans suite`);
/// as the last entry, it keeps the summary. An entry with nothing after its label keeps it, wherever
/// it stands: the winner was named before the label (`Paredes lot 506 - lots 501 à 505
/// infructueux`, `Orthomed : lots nº 25 et 26 / lot nº 24 : infructueux`), or the slot cut the list
/// short (`Lot 1) déclaré infructueux. Lots 2)`). Any entry that is no void verdict naming nobody
/// keeps it too ([`void_entry`]: `Lot 1) Dupont. Lot 2) infructueux`, `Lot 2 : Dupont (marché
/// négocié, AO infructueux)`).
fn every_lot_void(raw: &str) -> bool {
    let runs = alphanumeric_runs(raw);
    let words: Vec<String> = runs.iter().map(|&(from, to)| fold(&raw[from..to])).collect();
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    // Each entry: (the run its designator opens, the run after its label).
    let mut entries: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        match lot_label(raw, &runs, &words, i) {
            Some(after) => {
                entries.push((i, after));
                i = after;
            }
            None => i += 1,
        }
    }
    !entries.is_empty()
        && entries.iter().enumerate().all(|(k, &(_, after))| {
            let end = entries.get(k + 1).map_or(raw.len(), |&(next, _)| runs[next].0);
            let start = runs.get(after).map_or(raw.len(), |run| run.0).min(end);
            let text = &raw[start..end];
            let folded = fold(text);
            if folded.is_empty() {
                return false;
            }
            if folded.split(' ').all(|w| LOT_CONJUNCTIONS.contains(&w)) {
                // A conjunction only: joins the next entry, or keeps the summary as the last.
                return k + 1 < entries.len();
            }
            void_entry(text)
        })
}

/// Issue 520 (D4c, the review): whether a lot entry's text is a void verdict that names nobody — its
/// head (before the first break, [`reason_head`]) says the lot is void ([`mentions_void`]), so no
/// name stands before the verdict, and the whole text reads void ([`is_void_lot`]), so no award
/// follows it. `: Dupont (marché négocié, AO infructueux)` names Dupont before its earlier void call,
/// which read alone would be a void lot.
fn void_entry(text: &str) -> bool {
    let text = text
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .trim_end_matches(|c: char| c.is_whitespace() || "-–—/.,;:".contains(c));
    let head = reason_head(text).unwrap_or(text);
    mentions_void(head) && is_void_lot(text)
}

/// The label of a lot entry opening at `words[i]` (one folded word per alphanumeric run of `raw`,
/// `runs`), as [`several_lots`] reads a designator: a [`LOT_DESIGNATORS`] word, up to two number
/// marks or void words, and a number (digits, a glued number mark: [`glued_number`], or a [`ROMAN`]
/// numeral) — or a designator glued to its digits (`lot1096`). The label runs on over more of the
/// number written with a number's punctuation ([`number_gap`], [`label_number`]: `Sublote 9.1`, `Lot
/// 2011-231 01`, `lots 4b/4c`, `Lot 9 A`, `Lots 1, 2`), and over a conjunction only before another
/// number (`Lots 2a et 2c`, `lots 501 à 505`) — never across a `:` or `)`, where the entry's text
/// opens (`Lot 1 : 3M`, `Lot 1 : A.C.H.`), and never over a conjunction that the next designator
/// follows (`Perceel 2 en Perceel 4`). The index after the label, or `None`.
fn lot_label(raw: &str, runs: &[(usize, usize)], words: &[&str], i: usize) -> Option<usize> {
    let w = words[i];
    let mut j = i + 1;
    if LOT_DESIGNATORS.contains(&w) {
        // The marks and void words several_lots skips (`lot nº 3`, `lots infructueux : 1501`).
        let void_word = |word: &str| {
            VOID_STEMS.iter().any(|stem| stem.trim_start().split(' ').next().is_some_and(|head| word.starts_with(head)))
        };
        while j < words.len() && j <= i + 2 && (NUMBER_MARKS.contains(&words[j]) || void_word(words[j])) {
            j += 1;
        }
        let n = glued_number(words.get(j)?);
        if !(n.bytes().any(|b| b.is_ascii_digit()) || ROMAN.contains(&n)) {
            return None;
        }
        j += 1;
    } else if !LOT_DESIGNATORS.iter().any(|d| w.strip_prefix(d).is_some_and(digits_only)) {
        return None;
    }
    let joined = |k: usize| k < words.len() && number_gap(&raw[runs[k - 1].1..runs[k].0]);
    // A number at `k`, or a number mark that a number follows (`Lots n° 1 et n° 2`).
    let mark = |k: usize| NUMBER_MARKS.contains(&words[k]) && joined(k + 1) && label_number(words[k + 1]);
    let number_at = |k: usize| joined(k) && (label_number(glued_number(words[k])) || mark(k));
    loop {
        if number_at(j) {
            j += if label_number(glued_number(words[j])) { 1 } else { 2 };
        } else if joined(j) && LOT_CONJUNCTIONS.contains(&words[j]) && number_at(j + 1) {
            j += 2;
        } else {
            return Some(j);
        }
    }
}

/// Issue 520 (D4c): a folded word that goes on a lot's number — digits, digits and a sub-lot letter
/// (`2a`, `4c`), a lone letter (`Lot 9 A`) or a [`ROMAN`] numeral; never a word of several letters
/// (`Lote 2 no adjudicado`), and a sub-lot code only where a number's punctuation joins it
/// ([`number_gap`]: not `Lot 1 : 3M`).
fn label_number(w: &str) -> bool {
    let digits = w.bytes().take_while(u8::is_ascii_digit).count();
    digits_only(w)
        || (digits > 0 && w.len() == digits + 1 && w.as_bytes()[digits].is_ascii_alphabetic())
        || w.chars().count() == 1
        || ROMAN.contains(&w)
}

/// Issue 520 (D4c): whether the text between two runs joins parts of one lot number: spaces and a
/// number's punctuation only (`.`, `-`, `/`, `,`, `°`, `º`), and no sentence end (`. `).
fn number_gap(gap: &str) -> bool {
    gap.chars().all(|c| c.is_whitespace() || ".-/,°º".contains(c)) && !gap.contains(". ")
}

/// Issue 520 (D4b): a lot label at `words[j]` ([`lot_label`]) followed by its `:` or `)`, which opens
/// a summary's entry (`lot 2 : Dupont`, `Lot 3) Martin`) — or, since the second review, by a
/// capitalised word that is no void word, the winner written straight after its label (`, lot 2
/// Dupont Frères, lot 3 Martin`). The new procedure's lots take a dash and a lowercase title (`lot nº
/// 06 — bâtiments démontables`, `lot no 3 - agence Paul Signac`); a lowercase winner (`, lot 2
/// dupont`) stays cut with them.
fn entry_label(raw: &str, runs: &[(usize, usize)], words: &[&str], j: usize) -> bool {
    lot_label(raw, runs, words, j).is_some_and(|after| {
        let end = runs[after - 1].1;
        let rest = raw[end..].trim_start();
        rest.starts_with([':', ')'])
            || (rest.starts_with(char::is_uppercase)
                && runs.get(after).is_some_and(|&(from, to)| !mentions_void(&raw[from..to])))
    })
}

/// Two or more `;`-separated entries with letters, one of which names something: no void
/// stem, not a void whole value, and not a relaunch note (`infructueux ; relance en procédure
/// adaptée` is one void lot).
fn mixed_segments(raw: &str) -> bool {
    let segments: Vec<&str> = raw.split(';').filter(|s| s.chars().any(char::is_alphabetic)).collect();
    segments.len() >= 2
        && segments.iter().any(|segment| {
            let folded = fold(segment);
            let padded = format!(" {folded}");
            !VOID_STEMS.iter().any(|stem| padded.contains(stem))
                && !VOID_WHOLE.contains(&folded.as_str())
                && !VOID_TEMPLATES.contains(&folded.as_str())
                && ![" relanc", " nouvelle consultation", " nueva licitaci", " nuova gara", " nuevo procedimiento"]
                    .iter()
                    .any(|note| padded.contains(note))
        })
}

/// Issue 520 (D5): [`mixed_segments`] where a segment names a party. The name is split at a `;`
/// outside parentheses once `&amp;` is decoded (`(cf avis JOUE no 2012/s 133-221467 du 13.7.2012;
/// et Annonce no 484, BOAMP no 134b, du 13.7.2012)`, `de marque O&Amp;K`), and a segment names
/// nobody when it is void ([`mentions_void`]), a blank value (`néant`: [`BLANK_WHOLE`]), a reason
/// that speaks of an offer or a bidder and of its failure ([`offer_failure`]: `Aucune offre n'a été
/// déposée; le lot n° 2 a été déclaré infructueux`, `Desierto; el licitador ha renunciado a la
/// adjudicación del contrato`) or a relaunch note ([`RELAUNCH_NOTES`]: `; annonce(s) n° 116`). `Bp
/// innovis ; infructueux` names one, and so does `część 2 – wybrano ofertę firmy Usługi
/// Transportowe Jan Kowalski`.
fn award_mixed_segments(raw: &str) -> bool {
    if !mixed_segments(raw) {
        return false;
    }
    // `&amp;` decoded in any case, byte for byte on the ASCII lowercase.
    let lower = raw.to_ascii_lowercase();
    let mut decoded = String::with_capacity(raw.len());
    let mut at = 0;
    while let Some(found) = lower[at..].find("&amp;") {
        decoded.push_str(&raw[at..at + found + 1]);
        at += found + "&amp;".len();
    }
    decoded.push_str(&raw[at..]);
    let mut segments: Vec<String> = vec![String::new()];
    let mut depth = 0usize;
    for c in decoded.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ';' if depth == 0 => {
                segments.push(String::new());
                continue;
            }
            _ => {}
        }
        segments.last_mut().expect("one segment").push(c);
    }
    segments.retain(|segment| segment.chars().any(char::is_alphabetic));
    segments.len() >= 2
        && segments.iter().any(|segment| {
            let folded = fold(segment);
            let padded = format!(" {folded}");
            !mentions_void(segment)
                && !BLANK_WHOLE.contains(&folded.as_str())
                && !offer_failure(&folded)
                && !RELAUNCH_NOTES.iter().any(|note| padded.contains(note))
        })
}

/// Issue 520 (D5, the review): whether a folded segment gives the reason the lot is void — an offer
/// or bidder word ([`OFFER_WORDS`]) beside a word for its failure ([`OFFER_REJECTIONS`],
/// [`OFFER_FAILURES`], [`OFFER_NEGATIONS`] near the offer: `oferty odrzucone`, `Offres inacceptables`,
/// `nie wpłynęła żadna oferta`). An offer word alone names the winner (`najkorzystniejsza oferta: PHU
/// Kowalski`, `Wykonawca: Jan Kowalski`, `offre de la société Dupont retenue après négociation`).
fn offer_failure(folded: &str) -> bool {
    let starts = |w: &str, stems: &[&str]| stems.iter().any(|stem| w.starts_with(stem));
    let words: Vec<&str> = folded.split(' ').collect();
    let offers: Vec<usize> = (0..words.len()).filter(|&k| starts(words[k], &OFFER_WORDS)).collect();
    // A negation counts within [`OFFER_REACH`] words of an offer word only (the second review): a
    // winning offer's segment negates other things (`seule offre reçue : Dupont Frères, retenue sans
    // négociation`).
    let near_offer = |k: usize| offers.iter().any(|&o| o.abs_diff(k) <= OFFER_REACH);
    !offers.is_empty()
        && words.iter().enumerate().any(|(k, w)| {
            starts(w, &OFFER_REJECTIONS) || starts(w, &OFFER_FAILURES) || (OFFER_NEGATIONS.contains(w) && near_offer(k))
        })
}

/// Issue 520 (D5, the second review): how many words from an offer word an [`OFFER_NEGATIONS`] word
/// may stand and still negate it (`Aucune offre`, `nie wpłynęła żadna oferta`, `le nombre d'offres
/// conformes n'étant pas suffisant`).
const OFFER_REACH: usize = 4;

/// Whether a void phrase (`raw`, folded `folded`) goes on to say the lot was awarded after all:
/// an `attribué` form not negated, or `avec la société`. Issue 520 (D2) reads the award it
/// states, no other:
/// - the negation is the nearest word before it in its clause that is no [`CLAUSE_FILLERS`] word
///   ([`clause_break`]: `Le marché n'a pas été attribué`, whose word right before is `été`). The
///   clause, since the review: `Pas d'offre. Attribué à Léo Lagrange` and `Infructueux (pas
///   d'offre), attribué à Dupont` negate the offer, not the award; and the restrictive `ne … que`
///   is no negation (`il n'a été attribué qu'après négociation à la société Martin`, `n'a été
///   attribué, après négociation, qu'à …`: an `n'` and a `qu'` later in its sentence). 510's
///   reading stays the floor, since the second review: `non`, `pas` or `sans` right before the verb
///   negates it across any break (`Lot 2 : Non - Attribué`, `non attribué, que la commission
///   relancera`);
/// - `déjà attribué` is another procedure's award (`A 97.23.19) sans suite (déjà attribué au lot
///   A 12)`, `Lot déjà attribué a été déclaré sans suite`, `déjà été attribué au lot`) — unless the
///   verb goes on to `à` its party (`déjà été attribué à l'entreprise Dupont Frères`);
/// - `avec la société` that a relaunch still to come governs ([`intent_spans`]: `Déclaré
///   infructueux - à relancer en marché négocié … avec les sociétés ayant soumissionné`) names whom
///   the buyer will negotiate with.
///
/// An award word with no negation is never read as void: `257 lots attribués`, `attribué par
/// marché négocié … suite à appel d'offres infructueux` and `attribué le 28.10.2013` keep their
/// party, and so does `relancé en procédure négocié … avec la société B BRAUN` (a past relaunch).
fn award_clause(raw: &str, folded: &str) -> bool {
    let words: Vec<&str> = folded.split(' ').collect();
    let runs = alphanumeric_runs(raw);
    let aligned = runs.len() == words.len();
    let awarded = (0..words.len()).any(|i| award_verb(raw, &runs, &words, aligned, i));
    awarded
        || sentences(raw).iter().any(|sentence| {
            let runs = alphanumeric_runs(sentence);
            let folded: Vec<String> = runs.iter().map(|&(from, to)| fold(&sentence[from..to])).collect();
            let words: Vec<&str> = folded.iter().map(String::as_str).collect();
            let spans = intent_spans(sentence, &runs, &words);
            (0..words.len()).any(|k| avec_cue(&words, k) && !spans.iter().any(|&(from, to)| from < k && k < to))
        })
}

/// Whether the fold's word `words[i]` is an `attribué` form that states an award ([`award_clause`]'s
/// verb branch, one word at a time; `runs` are `raw`'s alphanumeric runs, one per word when `aligned`):
/// not negated in its clause, not `déjà` another procedure's.
fn award_verb(raw: &str, runs: &[(usize, usize)], words: &[&str], aligned: bool, i: usize) -> bool {
    award_verb_in(raw, runs, words, aligned, i, None)
}

/// [`award_verb`], given the first word of the verb's clause when the caller has read it once for every
/// word (`clause`: [`winner_cues`], the review — a scan back per verb made a long clause quadratic).
fn award_verb_in(
    raw: &str,
    runs: &[(usize, usize)],
    words: &[&str],
    aligned: bool,
    i: usize,
    clause: Option<usize>,
) -> bool {
    if !matches!(words[i], "attribue" | "attribuee" | "attribues" | "attribuees" | "attribuer") {
        return false;
    }
    // The 510 reading is the floor (the second review): `non`, `pas` or `sans` right before the
    // verb negates it, whatever stands between (`Lot 2 : Non - Attribué`, `non\nattribué`, `NON –
    // ATTRIBUÉ`). It is also the whole reading when the fold's words and the raw runs do not line up.
    if i > 0 && matches!(words[i - 1], "non" | "pas" | "sans") {
        return false;
    }
    if !aligned {
        return true;
    }
    let gap = |k: usize| &raw[runs[k - 1].1..runs[k].0];
    // The verb's clause: from the word after the last clause break before it.
    let from = clause.unwrap_or_else(|| (1..=i).rev().find(|&k| clause_break(gap(k))).unwrap_or(0));
    let before = words[from..i].iter().rev().find(|b| !CLAUSE_FILLERS.contains(*b));
    // `ne … que` restricts, it does not negate: an `n'` whose `qu'` follows in the sentence (`n'a été
    // attribué qu'après négociation`, `n'a été attribué, après négociation, qu'à la société Martin`).
    // Only `n'` / `ne`: `non attribué, que la commission relancera` stays negated.
    let restrictive = before.is_some_and(|b| matches!(*b, "n" | "ne"))
        && (i + 1..words.len()).take_while(|&k| !sentence_end(gap(k))).any(|k| matches!(words[k], "qu" | "que"));
    // `déjà` before the verb is another procedure's award, unless the verb goes on to its party (the
    // second review: `il a depuis déjà été attribué à l'entreprise Dupont Frères`), not to a lot
    // (`déjà attribué au lot A 12`) or an auxiliary (`Lot déjà attribué a été déclaré sans suite`).
    let to_party = words.get(i + 1) == Some(&"a") && {
        let k = i + 2 + usize::from(words.get(i + 2).is_some_and(|w| matches!(*w, "l" | "la" | "le" | "les")));
        words.get(k).is_some_and(|w| *w != "ete" && !LOT_DESIGNATORS.contains(w))
    };
    !before.is_some_and(|b| (*b == "deja" && !to_party) || (NEGATIONS.contains(b) && !restrictive))
}

/// Whether `words[k]` opens the `avec la société` cue (`avec l'entreprise`, `avec les sociétés`).
fn avec_cue(words: &[&str], k: usize) -> bool {
    words[k] == "avec"
        && matches!(words.get(k + 1..k + 3), Some(["la", "societe"] | ["l", "entreprise"] | ["les", "societes"]))
}

/// Issue 520 (D2c, D6b): the word ranges of one sentence (`words`, one folded word per run of
/// `raw`, `runs`) that a relaunch still to come governs ([`RELAUNCH_INTENT`]): from the intent word
/// to the first award already made after it ([`PAST_AWARDS`], `a été`, `ont été`: `Après avoir décidé
/// de relancer …, le marché a été conclu avec la société Dupont`; a [`PAST_PARTICIPLES`] word with no
/// future between, since the second review: `Décision de négocier : marché passé avec Dupont SARL`,
/// not `sera passé avec la STÉ France Télécom`) or the first new lot ([`lot_label`]:
/// `Lot infructueux, sera relancé - Lot 2: Dupont SARL`), or to the sentence's end. A lot after the
/// intent's own `avec les sociétés` stays governed: the cue's list names the bidders the buyer will
/// negotiate with (`à relancer … avec les sociétés ayant soumissionné à l'appel d'offre : lot 5 :
/// Société Massias`).
fn intent_spans(raw: &str, runs: &[(usize, usize)], words: &[&str]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if !RELAUNCH_INTENT.contains(w) {
            continue;
        }
        let mut cue = false;
        let mut end = words.len();
        for j in i + 1..words.len() {
            cue |= avec_cue(words, j);
            // The second review: `passé`, `signé`, `confié` state an award made too, unless a future
            // stands between (`sera passé avec la STÉ France Télécom`, `sera ensuite passé`).
            let future = words[i..j].iter().any(|w| matches!(*w, "sera" | "seront" | "etre"));
            let past = PAST_AWARDS.contains(&words[j])
                || (words[j] == "ete" && matches!(words[j - 1], "a" | "ont"))
                || (!future && PAST_PARTICIPLES.contains(&words[j]));
            if past || (!cue && lot_label(raw, runs, words, j).is_some()) {
                end = j;
                break;
            }
        }
        spans.push((i, end));
    }
    spans
}

/// Issue 520 (D1): BOAMP's void sentence for ONE lot, `Le lot <n> : <the lot's title> a été
/// déclaré infructueux` (or `sans suite`). The title is free text, and every keep signal fires
/// inside one: a second designator (`Le lot 102 : lot B10: Menuiseries bois PVC serrurerie vitrerie
/// (Genevois)`, `le lot 1 : Lot no1 : …`, `Le lot 6: Lot 2011-231 01 Communauté de Communes du
/// Genevois`, the sub-lot codes `nob02`), a `;` (`Rd1005; Rd24 et Rd32`, `O&Amp;K`), an awardee
/// word (`la réalisation par le titulaire de diagnostics`), a brand (`de marque JCB`), a typo (`SAU`
/// for *sauf*), bullets. So the name is void when the raw text opens with `Le lot <n°?> <digits>
/// :`, holds `le lot` nowhere else, the fold closes on the verdict whose subject is no other lot
/// (`; lot 2 a été déclaré infructueux`), no later sentence or dashed piece opens on another lot
/// (`… Dupont SARL. Lot 2 : Charpente a été déclaré infructueux`), and no award clause speaks
/// ([`award_clause`]). `Le lot 3: X. Le lot 4: Dupont SARL a été déclaré infructueux`, `Le lot 1 :
/// Keolis. Le lot 2 a été déclaré infructueux` (a second lot, its colon or not), `Le lot 1 : Dupont
/// SARL - lot 2 (Sols) a été déclaré infructueux`, `lot 25) la société Storz a été retenue. Le lot 32
/// a été déclaré infructueux` (no opener) and `Le lot 2 : attribué à Dupont SARL ; le lot 2b a été
/// déclaré infructueux` stay summaries.
fn lot_title_void(raw: &str, folded: &str) -> bool {
    let lower = raw.trim_start().to_lowercase();
    // Every `le lot` at a word boundary, its colon or not: the opener must be the only one.
    let lots = lower
        .match_indices("le lot")
        .filter(|(at, _)| {
            !lower[..*at].chars().next_back().is_some_and(char::is_alphanumeric)
                && !lower[at + "le lot".len()..].chars().next().is_some_and(char::is_alphanumeric)
        })
        .count();
    let words: Vec<&str> = folded.split(' ').collect();
    let declared = |w: &str| matches!(w, "declare" | "declaree" | "declares" | "declarees");
    let (verdict, subject) = match words.as_slice() {
        [subject @ .., "a", "ete", declare, "infructueux" | "infructueuse" | "infructueuses"]
        | [subject @ .., "a", "ete", declare, "sans", "suite"] => (declared(declare), subject),
        _ => (false, &[][..]),
    };
    // The verdict's subject is another lot: `lot 2`, `lot n 2`, `lot no2`, `lot2` — not a sub-lot
    // code that closes the title (`(sous lot nob06) a été déclaré sans suite`).
    let numbered = |w: &&str| {
        let n = glued_number(w);
        let digits = n.bytes().take_while(u8::is_ascii_digit).count();
        digits > 0 && (digits == n.len() || (digits + 1 == n.len() && n.as_bytes()[digits].is_ascii_alphabetic()))
    };
    let other_lot = match subject {
        [.., "lot" | "lots", n] | [.., "lot" | "lots", "n" | "no" | "nº" | "o", n] => numbered(n),
        [.., glued] => glued.strip_prefix("lot").is_some_and(digits_only),
        [] => false,
    };
    // The second review: a later sentence ([`sentences`]) or a later piece after a spaced dash that
    // opens on another lot, `le` or not, is a second lot (`… Dupont SARL. Lot 2 : Charpente a été
    // déclaré infructueux`, `Le lot 2 : Keolis SA ; lot 3 - Transport scolaire a été déclaré
    // infructueux`, `Le lot 1 : Dupont SARL - lot 2 (Sols) a été déclaré infructueux`). The title's own
    // designators stand in the opener's piece (`Le lot 102 : lot B10:`, `le lot 2 : Lot no2 - sorties
    // scolaires`), and its dashes go on to a title (`- gros oeuvre (sous-lot nob03)`).
    let opens_lot = |piece: &str| {
        let piece = fold(piece);
        match piece.split(' ').collect::<Vec<_>>().as_slice() {
            ["lot" | "lots", mark, n, ..] if NUMBER_MARKS.contains(mark) && numbered(n) => true,
            ["lot" | "lots", n, ..] => numbered(n),
            [glued, ..] => glued.strip_prefix("lot").is_some_and(|n| numbered(&n)),
            [] => false,
        }
    };
    let later_lot = sentences(raw)
        .into_iter()
        .flat_map(|sentence| sentence.split(" - "))
        .flat_map(|piece| piece.split(" – "))
        .flat_map(|piece| piece.split(" — "))
        .skip(1)
        .any(opens_lot);
    lots == 1 && lot_opener(&lower, 0) && verdict && !other_lot && !later_lot && !award_clause(raw, folded)
}

/// Whether the lowercased `lower` holds a lot opener at byte `at`: `le lot`, at a word boundary,
/// then an optional number mark (`n°`, `nº`, `no`) and the lot's digits, then a `:` (`Le lot 102 :
/// lot B10`, `le lot 01: Terrassement`, `Le lot n° 2 :`).
fn lot_opener(lower: &str, at: usize) -> bool {
    if !lower[at..].starts_with("le lot") || lower[..at].chars().next_back().is_some_and(char::is_alphanumeric) {
        return false;
    }
    let rest = &lower[at + "le lot".len()..];
    if !rest.starts_with(char::is_whitespace) {
        return false;
    }
    let mut rest = rest.trim_start();
    let mark = rest.split_whitespace().next().unwrap_or("");
    if NUMBER_MARKS.contains(&mark.trim_end_matches(['.', '°'])) || mark == "n°" {
        rest = rest[mark.len()..].trim_start();
    }
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0 && rest[digits..].trim_start().starts_with(':')
}

/// Issue 520: `raw` cut into its sentences, for the rules that read one ([`award_clause`]'s
/// relaunch intent, [`award_several_lots`]' relaunch markers, [`award_company_token`]'s bidders):
/// at a `;`, a line break or an ellipsis, after a `.` that a space or the end follows, and — since
/// the review — after a `.` glued between a word of two letters or more and a capital (`sera
/// relancé.Lot 2: Dupont SARL`; not the dots of `S.A.`, `S.p.A.`). A cut too many (`art. 35`) only
/// lets a keep signal count again, so each rule errs toward keeping the party.
fn sentences(raw: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut from = 0;
    let chars: Vec<(usize, char)> = raw.char_indices().collect();
    for (k, &(at, c)) in chars.iter().enumerate() {
        let next = chars.get(k + 1).map(|&(_, n)| n);
        let end = match c {
            ';' | '\n' | '…' => true,
            '.' => {
                let word = chars[..k].iter().rev().take_while(|(_, b)| b.is_alphabetic()).count();
                next.is_none_or(char::is_whitespace) || (word >= 2 && next.is_some_and(char::is_uppercase))
            }
            _ => false,
        };
        if end {
            let to = at + c.len_utf8();
            out.push(&raw[from..to]);
            from = to;
        }
    }
    if from < raw.len() {
        out.push(&raw[from..]);
    }
    out
}

/// Issue 520 (D6): [`company_token`] where the form is no bidder's. It does not count when it
/// stands in a rejected offer's span ([`rejected_offer_spans`]: `l'offre du groupement auxi+/ sas
/// Viviani est inacceptable`, `ha escluso dalla gara l'unica offerta ricevuta dalla compagnia
/// UnipolSai S.p.A.`), where a relaunch still to come governs it ([`intent_spans`]: `sera passé avec
/// la STÉ France Télécom`, `à relancer … avec les sociétés ayant soumissionné … : lot 5 : Société
/// Massias`), or in a failed bidder's line ([`bidder_before_void_lot`]: `Société Sindarro Lot déclaré
/// infructueux relancé en marché négocié`). `Yvelin SA. Lot 2 : infructueux`, `DESIERTO, S.L.`,
/// `Lot infructueux, sera relancé. Lot 2: Dupont SARL` (a new sentence), `Infructueux (1 offre
/// irrégulière), relancé … et conclu avec Promocash SAS` (another clause) and `Décision de négocier :
/// marché conclu avec Dupont SARL` (an award already made) keep theirs.
fn award_company_token(raw: &str) -> bool {
    company_token(raw)
        && !bidder_before_void_lot(raw)
        && sentences(raw).iter().any(|sentence| company_token(&winner_text(sentence)))
}

/// `sentence` without the spans a bidder is named in ([`rejected_offer_spans`], [`intent_spans`]),
/// the pieces left joined by a space.
fn winner_text(sentence: &str) -> String {
    let runs = alphanumeric_runs(sentence);
    let folded: Vec<String> = runs.iter().map(|&(from, to)| fold(&sentence[from..to])).collect();
    let words: Vec<&str> = folded.iter().map(String::as_str).collect();
    let mut spans = rejected_offer_spans(sentence, &runs, &words);
    spans.extend(intent_spans(sentence, &runs, &words));
    spans.sort_unstable();
    let byte = |k: usize| runs.get(k).map_or(sentence.len(), |run| run.0);
    let mut text = String::new();
    let mut at = 0;
    for (from, to) in spans {
        if byte(from) > at {
            text.push_str(&sentence[at..byte(from)]);
            text.push(' ');
        }
        at = at.max(byte(to));
    }
    text.push_str(&sentence[at..]);
    text
}

/// Issue 520 (D6a): the word ranges of one sentence that name a rejected offer's bidder. Each clause
/// ([`clause_break`], but a comma: `l'offre de la société X SA, jugée irrégulière, …`) is cut at its
/// offer words ([`OFFER_WORDS`]); an offer's span runs from the word after it to the next offer word,
/// an award made or kept ([`PAST_AWARDS`], `retenu(e)`: `1 offre irrégulière, relancé … et conclu
/// avec Promocash SAS`), the first comma after its rejection (the second review: `offres
/// irrégulières, marché négocié avec Dupont SARL`, `l'offre de Martin SA étant irrégulière, marché
/// confié à Dupont SARL`) or the clause's end, and names the bidder when a rejection
/// ([`OFFER_REJECTIONS`]) stands in it — or, for the clause's first offer, before it (`ha escluso
/// dalla gara l'unica offerta ricevuta dalla compagnia UnipolSai S.p.A.`). `l'offre de Dupont SARL a
/// été retenue, les autres offres étant irrégulières` rejects the others, not Dupont's.
fn rejected_offer_spans(raw: &str, runs: &[(usize, usize)], words: &[&str]) -> Vec<(usize, usize)> {
    let starts = |w: &str, stems: &[&str]| stems.iter().any(|stem| w.starts_with(stem));
    let ends_span = |w: &str| starts(w, &OFFER_WORDS) || PAST_AWARDS.contains(&w) || w.starts_with("retenu");
    let mut spans = Vec::new();
    let mut clause = 0;
    for end in 1..=words.len() {
        if end < words.len() {
            let gap = &raw[runs[end - 1].1..runs[end].0];
            if !clause_break(gap) || gap.trim() == "," {
                continue;
            }
        }
        let offers: Vec<usize> = (clause..end).filter(|&k| starts(words[k], &OFFER_WORDS)).collect();
        for (n, &offer) in offers.iter().enumerate() {
            let from = if n == 0 { clause } else { offer };
            let rejected = |k: usize| words[from..k].iter().any(|w| starts(w, &OFFER_REJECTIONS));
            // The second review: a comma after the rejection closes the rejected offer's span
            // (`offres irrégulières, marché négocié avec Dupont SARL`); one before it does not
            // (`l'offre de la société X SA, jugée irrégulière, …`).
            let to = (offer + 1..end)
                .find(|&k| ends_span(words[k]) || (rejected(k) && raw[runs[k - 1].1..runs[k].0].contains(',')))
                .unwrap_or(end);
            if rejected(to) {
                spans.push((offer + 1, to));
            }
        }
        clause = end;
    }
    spans
}

/// Issue 520 (D6c): a failed bidder named before the void verdict on its whole lot — `Société`
/// or `Sté`, the bidder's name (one to four words, no number), then `Lot déclaré(e) infructueux`
/// with no lot number anywhere, no award clause ([`award_clause`]), and nothing after the verdict
/// that names an award, a company form or an award word (`marché notifié à Société Dupont`, since
/// the review): `Société Sindarro Lot déclaré infructueux relancé en marché négocié avec publicité
/// et mise enconcurence (date limite le 10 juin 2008)`, `Société Energys. Lot déclaré infructueux …`,
/// `Société SOTEP Lot Déclaré infructueux et sera relancé …`. The relaunch's deadline fell after the
/// notice, and the awarded lots of the same notice name their winners with an address and a value.
fn bidder_before_void_lot(raw: &str) -> bool {
    let folded = fold(raw);
    let words: Vec<&str> = folded.split(' ').collect();
    let Some(lot) = words.iter().position(|w| *w == "lot") else { return false };
    let digit = |w: &&str| w.bytes().any(|b| b.is_ascii_digit());
    // A lot number anywhere: a designator whose next word past the number marks holds a digit.
    let numbered = words.iter().enumerate().any(|(i, w)| {
        LOT_DESIGNATORS.contains(w) && words[i + 1..].iter().find(|next| !NUMBER_MARKS.contains(*next)).is_some_and(digit)
    });
    // What follows the verdict: no company form, and no award word.
    let runs = alphanumeric_runs(raw);
    let award_word = |w: &&str| {
        PAST_AWARDS.contains(w) || AWARD_NOUNS.contains(w) || matches!(*w, "attribuer" | "passe" | "signe")
    };
    let quiet_after = runs.len() == words.len()
        && runs.get(lot + 2).is_some_and(|run| !company_token(&raw[run.1..]))
        && !words.iter().skip(lot + 3).any(award_word);
    matches!(words[0], "societe" | "ste")
        && (2..=5).contains(&lot)
        && !words[1..lot].iter().any(digit)
        && words.get(lot + 1).is_some_and(|w| w.starts_with("declare"))
        && words.get(lot + 2).is_some_and(|w| w.starts_with("infructu"))
        && !numbered
        && quiet_after
        && !award_clause(raw, &folded)
}

/// `name` without ONE trailing parenthetical lot qualifier — `(lote 3)`,
/// `(lotes VIII y IX)`, `(lot 9/10)` — and the punctuation after it; `None` when there is
/// none. Only for the whole-value compare: `Desierto (lotes VIII y IX)` is the whole value
/// `desierto`, while `ACME (lot 3)` stays the name `ACME`.
fn strip_lot_qualifier(name: &str) -> Option<&str> {
    lot_qualifier(name).map(|(head, _)| head)
}

/// [`strip_lot_qualifier`] with the group it took off: `(the name before it, the text
/// inside the parentheses)`.
fn lot_qualifier(name: &str) -> Option<(&str, &str)> {
    let trimmed = name.trim().trim_end_matches(|c: char| c.is_whitespace() || ".,;:".contains(c));
    let inner = trimmed.strip_suffix(')')?;
    let open = inner.rfind('(')?;
    let group = &inner[open + 1..];
    if group.contains(['(', ')']) {
        return None;
    }
    let first = group.trim_start().split(|c: char| !c.is_alphanumeric()).next().unwrap_or("");
    LOT_WORDS.iter().any(|w| first.eq_ignore_ascii_case(w)).then(|| (inner[..open].trim_end(), group))
}

/// The words a bare lot qualifier joins its numbers with, on the fold (`(lots 2 et 4)`,
/// `(lotes VIII y IX)`, `(lots 1 à 3)`, `(Lose 1 und 2)`).
const LOT_CONJUNCTIONS: [&str; 15] =
    ["et", "and", "y", "e", "und", "en", "i", "o", "ou", "or", "og", "och", "oraz", "a", "bis"];

/// Issue 511: whether a lot qualifier's `group` says nothing but which lots — designators,
/// number marks, numbers (digits, a sub-lot letter, a Roman numeral, a lone letter) and
/// conjunctions: `lot 2`, `lots 2 et 4`, `lot n° 3`, `lotes VIII y IX`, `lot 9/10`. Not `lot
/// 2 attribué à Dupont SARL`, which names who won another lot.
fn bare_lot_qualifier(group: &str) -> bool {
    fold(group).split(' ').filter(|w| !w.is_empty()).all(|w| {
        LOT_DESIGNATORS.contains(&w)
            || NUMBER_MARKS.contains(&w)
            || LOT_CONJUNCTIONS.contains(&w)
            || ROMAN.contains(&w)
            || w.bytes().any(|b| b.is_ascii_digit())
            || w.chars().count() == 1
    })
}

/// Issue 520 U4 (A1): the folded `attribué` forms whose award clause names its party ([`award_winner`]
/// cue a); the infinitive names it only as `d'attribuer ce lot à` (cue b).
const AWARDED_FORMS: [&str; 4] = ["attribue", "attribuee", "attribues", "attribuees"];

/// Issue 520 U4 (A1): how many words after an `attribué` form its party's `à` may stand, in one sentence
/// — `le <date>` (`attribué le 9/12/13 à Assurances securité`) and `dans le cadre de cette nouvelle
/// procédure` (`puis attribué dans le cadre de cette nouvelle procédure à la Sarltacavl`).
const AWARD_REACH: usize = 8;

/// Issue 520 U4 (A1): the folded futures beside [`RELAUNCH_INTENT`] — an award still to come names no
/// winner (`un marché négocié … sera passé avec la STÉ France Télécom`, `pour être attribué à`).
const FUTURE_AWARDS: [&str; 4] = ["seront", "serait", "seraient", "etre"];

/// Issue 520 U4 (A1, the review): the folded words that, in an `attribué`'s clause before it, make the
/// award one that is not, or no longer, made — a subjunctive or a conditional (`Il est envisagé qu'il
/// soit attribué à`, `aurait été attribué à`), a plan or a refusal (`prévu`, `proposé`, `pressenti`,
/// `refusé`, `impossible`), `jamais`, and an earlier award (`Le lot 3, initialement attribué à la société
/// Dupont, a été déclaré sans suite`, `Marché précédemment attribué à`; the second review: the pluperfect,
/// `Le marché avait été attribué à la société Dupont ; celle-ci s'étant désistée`, `La CAO avait décidé
/// d'attribuer ce lot à`). Such a cue names nobody.
const AWARD_DOUBTS: [&str; 33] = [
    "soit", "soient", "aurait", "auraient", "pourrait", "pourraient", "pourra", "pourront", "devrait", "devraient",
    "devra", "devront", "si", "envisage", "envisagee", "envisages", "envisagees", "prevu", "prevue", "propose",
    "proposee", "pressenti", "pressentie", "refuse", "refusee", "impossible", "jamais", "initialement",
    "precedemment", "anciennement", "auparavant", "avait", "avaient",
];

/// Issue 520 U4 (A1, the second review): the folded words that, anywhere in a cue's sentence or the next
/// one, make the award another contract's, the incumbent's, a scored bidder's or a co-winner's — `déjà`
/// (`prestations déjà attribuées à la société Dupont (marché n° 2012-15)`), the current or outgoing holder
/// and its extension (`Le marché actuel, attribué à la société Dupont, est prolongé par avenant`), a
/// framework (`le besoin étant couvert par l'accord-cadre attribué à l'UGAP`), a score (`la note
/// attribuée à la société Dupont, inférieure au seuil`), and a joint award (`attribué conjointement à`,
/// `en groupement avec`, `(accord-cadre multi-attributaires)`, a co- or sub-contractor). Such a text
/// names no one awardee: the whole answer is `None`.
const AWARD_CONTEXT: [&str; 40] = [
    "deja", "actuel", "actuelle", "actuels", "actuelles", "actuellement", "sortant", "sortante", "sortants",
    "sortantes", "prolonge", "prolongee", "prolonges", "prolongees", "prolongation", "avenant", "avenants",
    "reconduit", "reconduite", "reconduction", "accord", "note", "notes", "notee", "notation", "seuil",
    "conjointement", "solidairement", "groupement", "groupements", "grpt", "cotraitant", "cotraitants",
    "cotraitance", "traitant", "traitants", "multi", "mandataire", "mandataires", "titulaires",
];

/// Issue 520 U4 (A1, cue e, the second review): the folded words that, after `a été retenue` in its
/// sentence, make it a selection-stage verdict rather than the award (`retenue pour la négociation`,
/// `… pour la phase de négociation`, `admise à présenter une offre`, `retenue au stade de la
/// candidature`). The candidature before the company word is read beside them (`La candidature de la
/// société Dupont a été retenue`).
const SELECTION_STAGE: [&str; 12] = [
    "admis", "admise", "admises", "negociation", "negociations", "candidature", "candidatures", "phase", "dialogue",
    "audition", "presenter", "participer",
];

/// Issue 520 U4 (A1, cue e, the second review): the folded prepositions before `la société` that make it
/// a complement, not the subject of `a été retenue` (`Suite au désistement de la société Dupont, Martin
/// SA a été retenue`, `L'offre de la société Dupont ayant été écartée`, `Le recours de la société Dupont
/// a été retenu`).
const COMPLEMENT_PREPOSITIONS: [&str; 15] =
    ["de", "d", "du", "des", "au", "aux", "a", "par", "pour", "avec", "sur", "contre", "chez", "entre", "envers"];

/// Issue 520 U4 (A1, the second review): the folded stems that, after the name in its sentence or a
/// later one, reject or exclude the bidder it names ([`OFFER_REJECTIONS`] and `rejet`, `exclu`:
/// `L'offre de la société Dupont a été retenue mais jugée inacceptable`, `dont l'offre a été rejetée`),
/// matched at a word's start beside [`AWARD_UNDONE`].
const BIDDER_REJECTIONS: [&str; 7] = ["inacceptab", "irreguli", "esclus", "excluid", "ecarte", "rejet", "exclu"];

/// Issue 520 U4 (A1, the second review): the folded words that, inside a name after its first word,
/// make it a sentence that ran on rather than a name — a procedure, a date or a figure glued to the
/// winner (`Dupont dans le cadre d'une procédure négociée`, `Dupont en procédure négociée`, `Dupont
/// selon la procédure`, `Dupont au terme d'une procédure`, `Dupont lors de la CAO`, `Dupont sous réserve
/// de`, `DUPONT SAS montant HT`, `Dupont et notifié`, `Dupont sans succès`, `Dupont mais sans résultat`).
/// Read as published, lower-case or in capitals (`Restaurant Au Bon Coin` keeps its name).
const NAME_TAILS: [&str; 20] = [
    "dans", "selon", "lors", "sous", "au", "aux", "en", "suivant", "moyennant", "montant", "notifie", "notifiee",
    "signe", "signee", "conclu", "conclue", "sans", "mais", "puis", "vain",
];

/// Issue 520 U4 (A1, the second review): the figure tokens, as published, that make a name a sentence
/// with an amount in it (`Dupont au prix de 48 225 € HT`, `d'un montant de 12 000 EUR TTC`).
const FIGURE_TOKENS: [&str; 8] = ["EUR", "eur", "euros", "Euros", "EUROS", "HT", "TTC", "H.T."];

/// Issue 520 U4 (A1, the second review): the folded heads that make a label's value a status, not a
/// name (`Attributaire : négociation en cours`, `consultation en cours`, `Relancé en procédure
/// négociée`, `Sera désigné ultérieurement`, `Indéterminé`), beside [`RELAUNCH_STEMS`].
const STATUS_HEADS: [&str; 14] = [
    "consultation", "procedure", "sera", "seront", "indetermine", "indeterminee", "indefini", "indefinie",
    "attente", "litige", "ulterieurement", "inconnus", "designe", "designee",
];

/// Issue 520 U4 (A1, the second review): the possessives, as published, that make a name a pointer to
/// someone already named (`attribuées à son titulaire actuel`, `à sa régie`).
const POSSESSIVES: [&str; 9] = ["son", "sa", "ses", "leur", "leurs", "notre", "nos", "votre", "vos"];

/// Issue 520 U4 (A1, the review): the folded adverbs a negation reads past before `attribué` (`Lot pas
/// encore attribué à`, `non encore attribué à`) — [`award_verb`] reads the nearest word only.
const NEGATED_ADVERBS: [&str; 2] = ["encore", "toujours"];

/// Issue 520 U4 (A1, cue b, the review): the folded decisions after which `d'attribuer ce lot à` states
/// the award (`La commission d'appel d'offres réunie le 23.12.2013 a décidé d'attribuer ce lot à la
/// société Eiffage Construction`). Any other governor names nobody: a refusal (`La CAO a refusé
/// d'attribuer`), an impossibility (`Il n'a pas été possible d'attribuer`), a plan (`envisage d'attribuer`).
const AWARD_DECISIONS: [&str; 5] = ["decide", "decidee", "decides", "decidees", "decident"];

/// Issue 520 U4 (A1, cue c, the review): the folded words right before an awardee label that make it an
/// earlier or a later awardee's (`Ancien titulaire: Dupont`, `Précédent attributaire : Dupont SARL`).
const LABEL_QUALIFIERS: [&str; 10] =
    ["ancien", "ancienne", "anciens", "precedent", "precedente", "ex", "futur", "future", "initial", "initiale"];

/// Issue 520 U4 (A1, cue d): the folded past participles of a relaunch, after which `avec la société X`
/// closing the name is the relaunch's award (`relancé en procédure négocié … avec la société B BRAUN`).
const PAST_RELAUNCH: [&str; 8] =
    ["relance", "relancee", "relances", "relancees", "negocie", "negociee", "negocies", "negociees"];

/// Issue 520 U4 (A1, cue d, the review): the folded words a past relaunch's procedure is described with
/// between its participle and `avec la société X` (`relancé en procédure négocié sans mise en concurrence
/// et dispensé de publicité (art.35-I.1°) avec la Société Vascutek`), beside numbers, number marks and
/// roman numerals. Any other word there governs the `avec` instead — a failure, a negation, another verb
/// (`mais déclaré sans suite après échec des négociations avec`, `négocié sans succès avec`,
/// `négociations rompues avec`, `aucun accord trouvé avec`) — and names no relaunch's awardee.
const RELAUNCH_PROCEDURE: [&str; 43] = [
    "en", "de", "d", "du", "des", "la", "le", "l", "les", "et", "ni", "a", "au", "aux", "selon", "sous", "par", "sans",
    "avec", "procedure", "marche", "negocie", "negociee", "mise", "concurrence", "publicite", "publication",
    "prealable", "nouvelle", "forme", "dispense", "dispensee", "article", "art", "alinea", "code", "marches",
    "publics", "cmp", "decret", "application", "er", "eme",
];

/// Issue 520 U4 (A1): the folded words after `à` (and its article) that make it no party's preposition
/// (`attribué à l'issue de la négociation à la société X`, `à compter du`, `à partir du`, `à la suite
/// de`, `à hauteur de`; the review: `à titre provisoire`, `à nouveau`, `à l'unanimité`, `à la majorité`,
/// `à l'occasion de`, `à défaut de`, `à l'article 35`, `à la fin de`): the cue reads on to the next `à`.
const NO_PARTY_AFTER: [&str; 23] = [
    "issue", "compter", "partir", "suite", "hauteur", "titre", "nouveau", "unanimite", "majorite", "occasion", "defaut",
    "article", "art", "raison", "cause", "condition", "travers", "exception", "terme", "date", "fin", "effet", "moins",
];

/// Issue 520 U4 (A1, the review): the folded words right before an `à` that make it no party's
/// preposition (`attribué, conformément à l'article 35, à la société Dupont`, `suite à une procédure
/// négociée à`, `grâce à`, `jusqu'à`, `par recours à`): the cue reads on to the next `à`.
const NO_PARTY_BEFORE: [&str; 7] = ["conformement", "suite", "grace", "jusqu", "recours", "quant", "relativement"];

/// Issue 520 U4 (A1, the review): the folded word stems that, after the name in its sentence, undo the
/// award it states or make it an earlier one (`Lot attribué à la société Martin, qui s'est désistée`,
/// `(annulé)`, `résilié pour faute`, `qui a été mise en liquidation`, `Attributaire : Dupont SARL
/// (désisté)`, `la société Dupont a été retenue mais s'est désistée`), matched at a word's start; and
/// `sans suite` (`Le marché attribué à la société Dupont a été déclaré sans suite`). Such a cue names nobody.
const AWARD_UNDONE: [&str; 16] = [
    "desist", "annul", "resili", "liquidation", "defaill", "faillite", "renonc", "retir", "retrait", "abandon",
    "forfait", "caduc", "rompu", "rupture", "initialement", "precedemment",
];

/// Issue 520 U4 (A1): a count or a set where the winner would stand (`61 lots attribués à 29
/// fournisseurs`, `attribué à plusieurs entreprises`; the review: `deux entreprises`, `des entreprises
/// différentes`, `chacun des candidats`, `un prestataire local`, `un montant de 12 000 EUR`): no one
/// awardee. Folded, the extracted name's first word.
const COUNT_NOUNS: [&str; 33] = [
    "fournisseurs", "fournisseur", "lots", "lot", "marches", "marche", "societes", "entreprises", "titulaires",
    "attributaires", "plusieurs", "un", "une", "deux", "trois", "quatre", "cinq", "des", "ces", "chacun", "chacune",
    "chaque", "tous", "toutes", "divers", "diverses", "differents", "differentes", "certains", "certaines", "aucun",
    "aucune", "autres",
];

/// Issue 520 U4 (A1): a pointer where the winner would stand (`Attributaire : voir l'annexe`, `cf avis
/// d'attribution`; the review: `idem lot 1`, `ibid`, `ci-dessous`, `identique au lot n° 3`, `se reporter
/// à l'avis`, `consulter l'avis`, `la société suivante`, `la même entreprise`). Folded, the extracted
/// name's first word.
const WINNER_POINTERS: [&str; 26] = [
    "voir", "cf", "see", "siehe", "ver", "vease", "idem", "ibid", "ci", "identique", "meme", "memes", "suivant",
    "suivante", "suivants", "suivantes", "se", "consulter", "reporter", "precite", "precitee", "ledit", "ladite",
    "celui", "celle", "lui",
];

/// Issue 520 U4 (A1, the review): a role, a body, a quality or a status where the winner's name would
/// stand (`titulaire du lot 1`, `l'unique soumissionnaire`, `l'entreprise ayant remis la meilleure
/// offre`, `l'entreprise mieux-disante`, `l'offre économiquement la plus avantageuse`, `la CAO du 12 mai`,
/// `la séance du 12 mai`, `pouvoir adjudicateur`, `Monsieur le Maire`, `ce lot`, `en cours`, `non
/// désigné`, `pas de titulaire`, `inconnu`): every notice that writes it would share one organization.
/// Folded, the extracted name's first word.
const WINNER_ROLES: [&str; 41] = [
    "titulaire", "attributaire", "soumissionnaire", "soumissionnaires", "candidat", "candidats", "candidate",
    "prestataire", "prestataires", "offre", "offres", "unique", "seul", "seule", "mieux", "moins", "meilleur",
    "meilleure", "ayant", "montant", "cao", "commission", "seance", "pouvoir", "acheteur", "monsieur", "madame", "ce",
    "cet", "cette", "en", "non", "pas", "personne", "rien", "inconnu", "inconnue", "confidentiel", "confidentielle",
    "ancien", "precedent",
];

/// Issue 520 U4 (A1): the folded descriptors an awardee is introduced with and that the name drops (`la
/// Société Arrow Génériques`, `l'entreprise Promocash`, `l'esat "Les Ateliers de Polangis"`, `la STÉ`).
const WINNER_DESCRIPTORS: [&str; 4] = ["societe", "ste", "entreprise", "esat"];

/// Issue 520 U4 (A1, the review): the folded words that, right after a descriptor, make it part of the
/// legal name (`la Société Générale`, `la Société Française de Restauration`, `l'Entreprise Générale de
/// Bâtiment`, `la STE Nouvelle Dupont`): the descriptor stays, as after a lower-case connector (`la
/// Société des Eaux de Marseille`, `la Société d'Exploitation des Transports`: [`NAME_CONNECTORS`]).
const NAME_ADJECTIVES: [&str; 24] = [
    "generale", "general", "francaise", "nouvelle", "europeenne", "nationale", "internationale", "industrielle",
    "commerciale", "civile", "cooperative", "anonyme", "mutuelle", "centrale", "regionale", "immobiliere",
    "financiere", "fonciere", "technique", "auxiliaire", "parisienne", "lyonnaise", "marseillaise", "publique",
];

/// Issue 520 U4 (A1, the review): the lower-case connectors after which a descriptor is part of the legal
/// name ([`NAME_ADJECTIVES`]), as published — not `De Dietrich`, a name of its own.
const NAME_CONNECTORS: [&str; 4] = ["de", "des", "du", "d"];

/// Issue 520 U4 (A1, the review): the folded company abbreviations written with a final dot before the
/// name (`Ets. Dupont`, `la Sté. Dupont`, `Cie. Générale des Eaux`, `Assoc. Léo Lagrange`, `St. Gobain`):
/// their dot ends no sentence, and alone they are no name.
const NAME_ABBREVIATIONS: [&str; 8] = ["ets", "ste", "cie", "st", "assoc", "etab", "etabl", "soc"];

/// Issue 520 U4 (A1, the review): the folded words that open the next sentence or field after a dotted
/// legal form or an initial (`Dupont S.A. Montant HT : …`, `Dupont S.A.S. Date d'attribution : …`, `Martin
/// J. Le marché a été notifié`): the name ends with that dot. `A.E.C. Castellane` stays whole.
const FIELD_OPENERS: [&str; 26] = [
    "le", "la", "les", "l", "un", "une", "il", "elle", "ce", "cette", "montant", "date", "adresse", "lot", "lots",
    "marche", "prix", "valeur", "offre", "nombre", "attributaire", "titulaire", "duree", "objet", "decision", "suite",
];

/// Issue 520 U4 (A1, the review): the folded word stems a lot entry opens on when it relaunches its lot
/// rather than naming a winner (`lot 30 relancé en marché négocié 35 II 3`).
const RELAUNCH_STEMS: [&str; 5] = ["relanc", "negoci", "reporter", "recompos", "decompos"];

/// Issue 520 U4 (A1, the review): how many bytes past a name's opening [`winner_end`] reads — one cue's
/// scan of the whole rest made the extractor quadratic on a long text. A name that has not ended by
/// then ran on, and [`plausible_winner`] refuses it ([`WINNER_MAX_CHARS`]).
const WINNER_SCAN: usize = 240;

/// Issue 520 U4 (A1, the review): the longest name, in characters, that [`plausible_winner`] takes.
const WINNER_MAX_CHARS: usize = 120;

/// Issue 520 U4 (A1, unit-1 decision §4.2): whether `name` holds a word any [`award_winner`] cue needs
/// (`attribu…`, `titulaire`, `retenu`, `avec l…`), compared on the raw bytes, ASCII case-blind, with
/// nothing allocated — the fold's gate before it reads a kept party's copies.
pub fn may_name_a_winner(name: &str) -> bool {
    ["attribu", "titulaire", "retenu", "avec l"].iter().any(|cue| find_ascii_ci(name, cue).is_some())
}

/// The first byte at which `needle` (ASCII) stands in `hay`, ASCII case-blind. A match opens on a
/// char boundary: UTF-8 never puts an ASCII byte inside a multi-byte char.
fn find_ascii_ci(hay: &str, needle: &str) -> Option<usize> {
    let (hay, needle) = (hay.as_bytes(), needle.as_bytes());
    (needle.len() <= hay.len())
        .then(|| hay.windows(needle.len()).position(|w| w.eq_ignore_ascii_case(needle)))
        .flatten()
}

/// Issue 520 U4 (A1, unit-1 decision §4.2): the one awardee a void phrase names, as published (`CSL
/// Berhing`, `Sarltacavl`, `san see`), or `None`. The fold reads it only for a party it keeps whose
/// copies are in the void net ([`mentions_void`]) — an award summary (`Lot déclaré infructueux … puis
/// attribué à la Société Arrow Génériques suite à procédure négociée`) or a name after a void prefix
/// (`Pas d'offre. Relancé en procédure 35.ii.3 … Attribué à Léo Lagrange`); ordinary names never.
///
/// The cues, the measured French forms:
/// - (a) an `attribué(e)(s)` that states an award ([`award_verb`]: not negated in its clause, not `déjà`
///   another procedure's; nor, since the review, negated past `encore` or doubted: [`AWARD_DOUBTS`]),
///   with no relaunch or award still to come before it in its clause ([`RELAUNCH_INTENT`],
///   [`FUTURE_AWARDS`]), followed in its sentence within [`AWARD_REACH`] words by `à` and an optional
///   `:` (`attribué le 9/12/13 à`, `marché attribué à: Berthelet`) — not an adverbial `à`
///   ([`NO_PARTY_BEFORE`], [`NO_PARTY_AFTER`]: `conformément à l'article 35`, `à titre provisoire`);
/// - (b) `d'attribuer ce lot à` after a decision ([`AWARD_DECISIONS`]: `a décidé d'attribuer ce lot à`);
/// - (c) a label: `Nom du titulaire:`, `Nom de l'attributaire:`, `Attributaire:`, `titulaire:` (an awardee
///   word straight before its `:`, not negated, not an earlier one's: [`LABEL_QUALIFIERS`]) and
///   `fournisseur retenu =`;
/// - (d) `avec la société X` / `avec l'entreprise X` closing the name, after a past relaunch
///   ([`PAST_RELAUNCH`]) in its sentence with only its procedure between ([`RELAUNCH_PROCEDURE`]), and with
///   no [`RELAUNCH_INTENT`] or [`FUTURE_AWARDS`] word before it;
/// - (e) `la société X a été retenue`, not negated (`aucune société n'a été retenue`), with no relaunch to
///   come after it in its sentence.
///
/// A cue whose own sentence undoes the award after the name ([`AWARD_UNDONE`]: `qui s'est désistée`,
/// `(annulé)`, `résilié`) names nobody. The name drops its descriptor (`la Société`, `société`,
/// `l'entreprise`, `L'entreprise`, `LA SOCIÉTÉ`, `l'esat`, `la STÉ` — unless the legal name goes on from
/// it: `la Société Générale`, `la Société des Eaux de Marseille`), a lower-case article (`la Sarltacavl`;
/// `La Poste` keeps its capital `La`) and its quotes, and ends where [`winner_end`] says: ` suite`, `,`,
/// `:`, ` (`, ` pour un montant`, ` après négociation`, ` SISE`, ` le <digit>`, a spaced dash, a five-digit
/// postcode, a line break, a sentence end (a `.` that does not follow a single letter or a company
/// abbreviation: `A.E.C. Castellane`, `C.A.J.L`, `Ets. Dupont` stay whole; `Dupont S.A. Montant …` ends at
/// `S.A.`), a list separator (`;`, ` / `), the next cue, or the end of the text. Since the second review it
/// also ends at ` pour <digits>` (`pour 48 225,00 € HT`) and ` par <article> <lower-case word>` (`par le
/// pouvoir adjudicateur`), and a dotted legal form that opens the name is the name's own (`la S.A.R.L.
/// Dupont` → `S.A.R.L. Dupont`, never `S.A.R.L.`).
///
/// `None` with no cue; for a name that opens with no letter (`à 29 fournisseurs`), on a count, a pointer
/// or a role ([`COUNT_NOUNS`], [`WINNER_POINTERS`], [`WINNER_ROLES`]), that is a descriptor alone, cut
/// mid-phrase, holds another party's `à` or another lot, or is itself a non-name ([`plausible_winner`]);
/// for two or more distinct names (`lot 10 attribué à Bayer Santé Familiale … Lot 14 attribué à Pfizer`,
/// `titulaire 1: SAGA titulaire 2: Chapelec`, two `attributaire:` labels); when a list separator after
/// the name opens a segment that is not void (`Lots suivants attribués à la société RMI, … / 4.2
/// (portables): …`); and when a lot entry no cue reads names a winner (`Lot 1) attribué à Dupont. Lot 2)
/// Martin`: [`lot_entry_names_another`]). A future counterparty (`sera passé avec la STÉ France
/// Télécom`), a relaunch's bidders (`à relancer … avec les sociétés ayant soumissionné`), a failed,
/// rejected or excluded bidder, a pointer (`Voir l'avis d'attribution … pour l'entreprise attributaire du
/// lot`) and an award with no party (`257 lots attribués`, `attribué le 28.10.2013`, `attribué par marché
/// négocié`) name nobody.
///
/// The second review, each a refusal of the whole answer (the party keeps its summary, the state before
/// U4):
/// - an award undone or a bidder rejected after the name, in its sentence or a later one in the scanned
///   window, or a void verdict after it that is no other numbered lot's ([`undone_after`]: `… attribué à
///   la société Dupont. Celle-ci n'ayant pas produit les certificats, le lot est déclaré sans suite`);
/// - an [`AWARD_CONTEXT`] word, or an undo or a rejection stem, anywhere in the cue's sentence or the next
///   (`prestations déjà attribuées`, `Le marché actuel, attribué à …, est prolongé par avenant`, `la note
///   attribuée à`, `attribué conjointement à`, `Suite au désistement de la société Martin, le lot a été
///   attribué à …`), and a pluperfect before cues a and b ([`AWARD_DOUBTS`]: `avait été attribué`);
/// - a cue e whose company another sentence or a complement holds (`Suite au désistement de la société
///   Dupont, Martin SA a été retenue`, `La candidature de la société Dupont`), that a selection stage
///   follows ([`SELECTION_STAGE`], `pour la négociation`), whose name a new subject follows rather than an
///   apposition ([`apposition`]), or whose name opens on a lower-case qualifier (`la société classée
///   première`);
/// - a list of names after the name ([`listed_names_after`]: `Dupont SARL, Martin SA et Durand SAS`), and a
///   cue d name that ran on into prose ([`runs_on`]: `Dupont sans succès`);
/// - the `à été` typo for the verb ends the cue's reach (`le marché à été notifié`).
pub fn award_winner(name: &str) -> Option<String> {
    if !may_name_a_winner(name) {
        return None;
    }
    let runs = alphanumeric_runs(name);
    let folded: Vec<String> = runs.iter().map(|&(from, to)| fold(&name[from..to])).collect();
    let words: Vec<&str> = folded.iter().map(String::as_str).collect();
    let cues = winner_cues(name, &runs, &words);
    let starts: Vec<usize> = cues.iter().map(|cue| cue.at).collect();
    // The second review: every numbered lot (`lot 25`, `Le lot 32`, `lot n° 3`), where it stands and its
    // number, for [`undone_after`]'s reading of a void verdict after the name.
    let lots: Vec<(usize, &str)> = (0..words.len())
        .filter(|&k| LOT_DESIGNATORS.contains(&words[k]))
        .filter_map(|k| {
            let at = k + 1 + usize::from(words.get(k + 1).is_some_and(|mark| NUMBER_MARKS.contains(mark)));
            words.get(at).filter(|n| n.bytes().any(|b| b.is_ascii_digit())).map(|n| (runs[k].0, *n))
        })
        .collect();
    // The winner as published, and its fold (distinct names compare on it).
    let mut winner: Option<(String, String)> = None;
    for cue in &cues {
        // The second review: a cue its sentence (or the next) makes another contract's, the incumbent's,
        // a scored bidder's or a joint award ([`AWARD_CONTEXT`]), or a cue e that is a selection stage or
        // whose company a complement or another sentence holds, names nobody, whatever else is said.
        if cue.refused {
            return None;
        }
        let end = winner_end(name, cue.name, &starts);
        // Cue e's name ends before `a été retenue` at the latest, and at its own terminators before that.
        let end = cue.until.map_or(end, |until| until.min(end));
        // Cue d names the winner only where it closes the name (`… avec la société B BRAUN`).
        if cue.closing && !name[end..].trim_matches(|c: char| c.is_whitespace() || c == '.').is_empty() {
            continue;
        }
        // The second review: cue e's name ended early, and only an apposition may stand between it and
        // `a été retenue` (`Dupont SARL, Lyon, a été retenue`, `Dupont (69003 Lyon)`, `Dupont - 69003
        // Lyon`) — not a new subject (`… de la société Dupont, Martin SA a été retenue`, `Dupont s'étant
        // désistée, Martin SA a été retenue`).
        if let Some(until) = cue.until
            && end < until
            && !apposition(&name[end..until])
        {
            return None;
        }
        // The review, widened by the second: an award undone or a bidder rejected after the name, in its
        // sentence or a later one, or a void verdict after it that is no other lot's (`… attribué à la
        // société Dupont. Celle-ci n'ayant pas produit les certificats, le lot est déclaré sans suite`).
        if undone_after(name, end, &lots) {
            return None;
        }
        // The third check: a second awardee after the name in its sentence (`attribué à Dupont pour 12 000 €
        // HT et à Martin pour 8 000 € HT`, `… ainsi qu'à Martin`, `… puis à Martin`) is a joint award, which
        // names no one winner.
        if second_awardee_after(name, end) {
            return None;
        }
        let candidate = strip_winner(&name[cue.name..end]);
        // A later cue's text is a tail of the first one's, so the first reads every separator after it.
        // The second review: cue e's name opened after its descriptor on a lower-case word (`la société
        // classée première a été retenue`); cue d's name, which runs to the end of the text, swallowed a
        // tail (`avec la société Dupont sans succès`); a list of names after this one (`Dupont SARL,
        // Martin SA et Durand SAS`, `Dupont SARL - Martin SA`).
        if !plausible_winner(&candidate)
            || (cue.introduced && candidate.starts_with(char::is_lowercase))
            || (cue.closing && runs_on(&candidate))
            || listed_names_after(&name[end..])
            || (winner.is_none() && listed_after(&name[cue.name..]))
        {
            return None;
        }
        let key = fold(&candidate);
        match &winner {
            Some((_, first)) if *first != key => return None,
            Some(_) => {}
            None => winner = Some((candidate, key)),
        }
    }
    let (winner, _) = winner?;
    (!lot_entry_names_another(name, &runs, &words, &starts)).then_some(winner)
}

/// Issue 527: TenderNed's correction marks each tenderer of a re-published award after its name,
/// `(gegunde partij)` for the one awarded and `(niet gegunde partij)` for the others (00798764-2024:
/// `Planon Building Management B.V. (gegunde partij)`, `Humble Inpect B.V. (niet gegunde partij)`).
/// The name without its mark, and whether the mark says the party was NOT awarded; `None` for a name
/// without one. Only a closing bracket group whose fold is exactly one of the two marks counts (`niet-
/// gegunde partij` folds the same), so `Gegunde Partijen Holding` and `X (gegund)` are no marks.
pub fn award_annotation(name: &str) -> Option<(&str, bool)> {
    let trimmed = name.trim_end();
    let inside = trimmed.strip_suffix(')')?;
    let open = inside.rfind('(')?;
    let not_awarded = match fold(&inside[open + 1..]).as_str() {
        "niet gegunde partij" => true,
        "gegunde partij" => false,
        _ => return None,
    };
    let base = inside[..open].trim_end();
    // The review: a base with an unclosed bracket (`Foo (Bar (niet gegunde partij))`) or no letter or
    // digit (`- (gegunde partij)`) is no name the mark sits after.
    let balanced = base.matches('(').count() == base.matches(')').count();
    (balanced && base.chars().any(char::is_alphanumeric)).then_some((base, not_awarded))
}

/// Issue 520 U6 (A2, unit-1 decision §4.2): the one awardee a reviewed summary names before a void tail,
/// as published, or `None`. A2's reliable subset is these five rows, read whole — the name's fold equals a
/// row's (`/` or ` - ` between the same words reads the same) — and nothing else:
/// - BGE Guyane before its address and BOAMP's partial void (1844226/29/32): `BGE Guyane 16 rue Lt Becker
///   97300 Cayenne / La consultation a été déclarée sans suite pour les parts non couvertes des besoins`;
/// - a GRPT consortium before the negotiated award after a void call (2734624/5): `GRPT GCC (mandataire)
///   et Andre BTP, suite à procédure négociée après AO infructueux`.
///
/// No general shape is reliable. Two panels (wf_8883fc55-316: 97 probes; wf_421af829-0da: 37) broke each
/// one in the costly direction — a winner named that is not the one contractor. A `;` void-only segment
/// cannot tell a company from a lot title, a procedure, a status or a failed bidder (`Plomberie sanitaire ;
/// infructueux`, `Dupont SARL ; offre infructueuse`), and is B1's list grammar, which 511 §5 keeps
/// (`Bp innovis ; infructueux`, 4836476, stays). An address proves a place, not one contractor: a lot
/// title with its site, a buyer, a generic role word or a second party beside the first all carry one
/// (`Restauration scolaire 16 rue …`, `La Mairie de Cayenne 1 place …`, `BGE Guyane 16 rue … 97300
/// Cayenne Point Com / …`). A consortium head admits members that are buyers, failed bidders or another
/// lot's (`Groupement de commandes Ville de Cayenne (mandataire) et CACL`, `GRPT Dupont (mandataire) et
/// Martin, Durand SA non conforme`). The fold reads this, like [`award_winner`], only for a kept party
/// whose copies are in the void net; a name any `award_winner` cue reads is never one of these rows.
pub fn void_tail_winner(name: &str) -> Option<String> {
    let key = fold(name);
    A2_REVIEWED.iter().find(|(published, _)| fold(published) == key).map(|(_, winner)| (*winner).to_owned())
}

/// Issue 520 U6: A2's reviewed rows (unit-1 decision §4.2, the walk's dry 2375), each as published and
/// the winner [`void_tail_winner`] reads from it.
const A2_REVIEWED: [(&str, &str); 5] = [
    (
        "BGE Guyane 16 rue Lt Becker 97300 Cayenne / La consultation a été déclarée sans suite pour les parts non couvertes des besoins",
        "BGE Guyane",
    ),
    (
        "BGE Guyane 16 rue Lieutenant Becker 97300 Cayenne - la consultation a été déclarée sans suite pour les parts non couvertes des besoins",
        "BGE Guyane",
    ),
    (
        "BGE Guyane 16 rue Lieutenant Becker 97300 Cayenne/ la consultation a été déclarée sans suite pour les parts non couvertes des besoins",
        "BGE Guyane",
    ),
    (
        "GRPT GCC (mandataire) et Andre BTP, suite à procédure négociée après AO infructueux",
        "GRPT GCC (mandataire) et Andre BTP",
    ),
    (
        "GRPT vallée sas (mandataire), La Comec et Arbat System, suite à procédure négociée après AO infructueux",
        "GRPT vallée sas (mandataire), La Comec et Arbat System",
    ),
];

/// Issue 520 U4: one [`award_winner`] cue — where it opens (`at`, the next cue's name ends there), where
/// its name opens, where that name ends at the latest when the cue says so (cue e: before `a été
/// retenue`), and whether the name must close the text (cue d). Since the second review: whether the
/// name opened after a descriptor it dropped (cue e: `la société X a été retenue`), and whether the cue
/// makes the whole answer `None` ([`AWARD_CONTEXT`] in its sentence or the next; a cue e that is a
/// selection stage, or whose company a complement or another sentence holds).
struct WinnerCue {
    at: usize,
    name: usize,
    until: Option<usize>,
    closing: bool,
    introduced: bool,
    refused: bool,
}

/// Issue 520 U4: every [`award_winner`] cue in `raw` (`runs` its alphanumeric runs, `words` their folds,
/// one per run), in text order. Each word's clause and sentence, and where the next future or doubt word
/// stands, are read once (the review: a re-scan per cue made a long text quadratic).
fn winner_cues(raw: &str, runs: &[(usize, usize)], words: &[&str]) -> Vec<WinnerCue> {
    let gap = |k: usize| &raw[runs[k - 1].1..runs[k].0];
    let future = |w: &str| RELAUNCH_INTENT.contains(&w) || FUTURE_AWARDS.contains(&w);
    // Per word: the first word of its clause, as [`award_verb`] reads one, and its sentence's number.
    let (mut clause, mut sentence) = (vec![0; words.len()], vec![0; words.len()]);
    for k in 1..words.len() {
        clause[k] = if clause_break(gap(k)) { k } else { clause[k - 1] };
        sentence[k] = sentence[k - 1] + usize::from(sentence_end(gap(k)));
    }
    // Per word: the next future word, and the next doubt, at it or after it.
    let (mut next_future, mut next_doubt) = (vec![None; words.len() + 1], vec![None; words.len() + 1]);
    for k in (0..words.len()).rev() {
        next_future[k] = if future(words[k]) { Some(k) } else { next_future[k + 1] };
        next_doubt[k] = if AWARD_DOUBTS.contains(&words[k]) { Some(k) } else { next_doubt[k + 1] };
    }
    let between = |next: &[Option<usize>], from: usize, to: usize| next[from].is_some_and(|k| k < to);
    // The second review: per sentence, whether it holds an [`AWARD_CONTEXT`] word, or an undo or a
    // rejection ([`AWARD_UNDONE`], [`BIDDER_REJECTIONS`]) before the cue as much as after it (`Suite au
    // désistement de la société Martin, le lot a été attribué à la société Dupont`: refused, not read
    // for whom the undo governs); a cue reads its own sentence and the next.
    let mut context_in = vec![false; sentence.last().map_or(0, |last| last + 2)];
    for k in 0..words.len() {
        if AWARD_CONTEXT.contains(&words[k])
            || AWARD_UNDONE.iter().chain(&BIDDER_REJECTIONS).any(|stem| words[k].starts_with(stem))
        {
            context_in[sentence[k]] = true;
        }
    }
    let context = |i: usize| context_in[sentence[i]] || context_in[sentence[i] + 1];
    // The review: a negation the nearest word hides (`Lot pas encore attribué`, `non encore attribué`).
    let negated_past_adverb = |i: usize| {
        let mut before = words[clause[i]..i].iter().rev().filter(|b| !CLAUSE_FILLERS.contains(*b));
        before.next().is_some_and(|b| NEGATED_ADVERBS.contains(b))
            && before.next().is_some_and(|b| NEGATIONS.contains(b))
    };
    // `à` as published: the fold reads the verb `a` the same way (`Lot déjà attribué a été déclaré`).
    let to = |k: usize| matches!(&raw[runs[k].0..runs[k].1], "à" | "À");
    // The byte after run `k`, past spaces and one `:` (`attribué à: Faure/Annequin`).
    let past_colon = |k: usize| {
        let rest = &raw[runs[k].1..];
        let trimmed = rest.trim_start();
        runs[k].1 + rest.len() - trimmed.len() + trimmed.strip_prefix(':').map_or(0, |_| 1)
    };
    // The byte after a label's mark, when one of `marks` follows run `k` past spaces.
    let label = |k: usize, marks: &[char]| {
        let rest = &raw[runs[k].1..];
        let trimmed = rest.trim_start();
        let mark = trimmed.chars().next().filter(|c| marks.contains(c))?;
        Some(runs[k].1 + rest.len() - trimmed.len() + mark.len_utf8())
    };
    let procedural = |w: &str| {
        RELAUNCH_PROCEDURE.contains(&w)
            || NUMBER_MARKS.contains(&w)
            || ROMAN.contains(&w)
            || w.bytes().any(|b| b.is_ascii_digit())
    };
    let first_future = next_future[0];
    let mut cues = Vec::new();
    // Cue d: the last past relaunch participle, and whether only its procedure stands after it.
    let (mut relaunched, mut procedure_only): (Option<usize>, bool) = (None, false);
    for i in 0..words.len() {
        let w = words[i];
        let clause_future = between(&next_future, clause[i], i);
        // (a) `attribué … à`.
        if AWARDED_FORMS.contains(&w)
            && award_verb_in(raw, runs, words, true, i, Some(clause[i]))
            && !clause_future
            && !between(&next_doubt, clause[i], i)
            && !negated_past_adverb(i)
        {
            // The second review: the `à été` typo for the verb (`le marché à été notifié`) is no party's
            // `à`, and the cue reads no further — never `été notifié` as the winner.
            let party = (i + 1..words.len().min(i + AWARD_REACH + 1))
                .take_while(|&k| !sentence_end(gap(k)) && !(to(k) && words.get(k + 1) == Some(&"ete")))
                .find(|&k| {
                    let next = k + 1 + usize::from(words.get(k + 1).is_some_and(|n| matches!(*n, "l" | "la" | "le" | "les")));
                    to(k)
                        && !NO_PARTY_BEFORE.contains(&words[k - 1])
                        && !words.get(next).is_some_and(|n| NO_PARTY_AFTER.contains(n))
                });
            if let Some(k) = party {
                let (at, name) = (runs[i].0, past_colon(k));
                cues.push(WinnerCue { at, name, until: None, closing: false, introduced: false, refused: context(i) });
            }
        }
        // (b) `a décidé d'attribuer ce lot à`.
        if w == "attribuer"
            && i >= 2
            && words[i - 1] == "d"
            && AWARD_DECISIONS.contains(&words[i - 2])
            && clause[i] <= i - 2
            && !negated(&words[clause[i]..i - 2])
            && words.get(i + 1..i + 3) == Some(&["ce", "lot"][..])
            && i + 3 < words.len()
            && to(i + 3)
            && award_verb_in(raw, runs, words, true, i, Some(clause[i]))
            && !clause_future
            && !between(&next_doubt, clause[i], i)
        {
            let (at, name) = (runs[i - 1].0, past_colon(i + 3));
            cues.push(WinnerCue { at, name, until: None, closing: false, introduced: false, refused: context(i) });
        }
        // (c) The awardee labels.
        if matches!(w, "attributaire" | "titulaire")
            && !negated(&words[..i])
            && !(i > 0 && LABEL_QUALIFIERS.contains(&words[i - 1]))
            && let Some(name) = label(i, &[':', '：'])
        {
            let at = runs[i].0;
            cues.push(WinnerCue { at, name, until: None, closing: false, introduced: false, refused: context(i) });
        }
        if w == "retenu"
            && i > 0
            && words[i - 1] == "fournisseur"
            && let Some(name) = label(i, &['='])
        {
            let at = runs[i - 1].0;
            cues.push(WinnerCue { at, name, until: None, closing: false, introduced: false, refused: context(i) });
        }
        // (d) `avec la société X` closing the name, after a past relaunch in its sentence with only its
        // procedure between, and before no future one.
        if w == "avec"
            && matches!(words.get(i + 1..i + 3), Some(["la", "societe"] | ["l", "entreprise"]))
            && relaunched.is_some_and(|j| sentence[j] == sentence[i])
            && procedure_only
            && first_future.is_none_or(|f| f > i)
        {
            let (at, name) = (runs[i].0, runs[i + 1].0);
            cues.push(WinnerCue { at, name, until: None, closing: true, introduced: false, refused: context(i) });
        }
        // (e) `la société X a été retenue`, the name from the company word to the verdict.
        if w == "a"
            && words.get(i + 1) == Some(&"ete")
            && words.get(i + 2).is_some_and(|r| matches!(*r, "retenue" | "retenu"))
            && !(i > 0 && matches!(words[i - 1], "n" | "ne"))
            && !next_future[i + 3].is_some_and(|f| sentence[f] == sentence[i])
            && let Some(k) = (i.saturating_sub(7)..i)
                .rev()
                .find(|&k| words[k] == "societe" || (words[k] == "entreprise" && k > 0 && words[k - 1] == "l"))
            && !words[k.saturating_sub(2)..k].iter().any(|q| NEGATIONS.contains(q) || *q == "nulle")
        {
            let head = if k > 0 && matches!(words[k - 1], "la" | "l") { k - 1 } else { k };
            let at = runs[head].0;
            // The name opens after the company word, unless the legal name goes on from it (`la Société
            // Générale a été retenue`): [`strip_winner`] reads the descriptor then.
            let introduced = !descriptor_names(&raw[runs[k].1..]);
            let name = if introduced { runs[k].1 } else { at };
            // The second review: no award when the company stands in another sentence (`Offre de la
            // société Dupont irrégulière. Martin a été retenu`) or is a complement (`Suite au désistement
            // de la société Dupont, Martin SA a été retenue`, `La candidature de la société Dupont a été
            // retenue`), or when the verdict is a selection stage (`retenue pour la négociation`, `pour
            // présenter une offre`, `admise à négocier`; not `pour le lot 2`, `pour un montant`).
            let purpose = words.get(i + 3) == Some(&"pour")
                && !award_circumstance(&raw[runs[i + 3].0..runs[i + 3].1], &raw[runs[i + 3].1..]);
            // A `.` after a single letter or a company abbreviation inside the name ends no sentence
            // (`La société S.A.R.L. Dupont a été retenue`, `la société Ets. Dupont`).
            let split = (k + 1..=i).any(|j| {
                let before = &raw[runs[j - 1].0..runs[j - 1].1];
                sentence_end(gap(j))
                    && (gap(j).contains([';', '\n', '…'])
                        || !(before.chars().count() == 1 || NAME_ABBREVIATIONS.contains(&words[j - 1])))
            });
            let refused = context(i)
                || split
                || (head > 0 && COMPLEMENT_PREPOSITIONS.contains(&words[head - 1]))
                || words[k.saturating_sub(3)..k].iter().any(|q| q.starts_with("candidat"))
                || purpose
                || (i + 3..words.len())
                    .take(AWARD_REACH * 4)
                    .take_while(|&j| sentence[j] == sentence[i])
                    .any(|j| SELECTION_STAGE.contains(&words[j]));
            cues.push(WinnerCue { at, name, until: Some(runs[i].0), closing: false, introduced, refused });
        }
        if PAST_RELAUNCH.contains(&w) {
            (relaunched, procedure_only) = (Some(i), true);
        } else if !procedural(w) {
            procedure_only = false;
        }
    }
    cues.sort_by_key(|cue| cue.at);
    cues
}

/// Issue 520 U4: whether what follows a name in `raw` (from byte `end`, at most [`WINNER_SCAN`] bytes)
/// undoes the award: in the name's own sentence, an [`AWARD_UNDONE`] or [`BIDDER_REJECTIONS`] stem or
/// `sans suite`. Since the second review, the later sentences in that window too (`… attribué à la
/// société Dupont. Celle-ci n'ayant pas produit les certificats, le lot est déclaré sans suite`), and a
/// void verdict anywhere after the name ([`mentions_void`]: `la société Dupont a été retenue mais n'a pas
/// produit les attestations ; le lot est déclaré infructueux`) — unless that sentence is another
/// numbered lot's (`lots`: where each numbered lot stands in `raw`, and its number): `lot 25) la société
/// Storz a été retenue. Le lot 32 a été déclaré infructueux` names lot 32, which no lot before it is.
fn undone_after(raw: &str, end: usize, lots: &[(usize, &str)]) -> bool {
    let rest = &raw[end..floor_boundary(raw, end + WINNER_SCAN)];
    let mut from = end;
    for (n, piece) in sentences(rest).into_iter().enumerate() {
        let to = from + piece.len();
        let folded = fold(piece);
        let words: Vec<&str> = folded.split(' ').filter(|w| !w.is_empty()).collect();
        let undone =
            words.iter().any(|w| AWARD_UNDONE.iter().chain(&BIDDER_REJECTIONS).any(|stem| w.starts_with(stem)));
        // Another lot's sentence: it names a numbered lot, and none of its numbers stands before it.
        let named: Vec<&str> = lots.iter().filter(|(at, _)| (from..to).contains(at)).map(|(_, lot)| *lot).collect();
        let other_lot = !named.is_empty()
            && named.iter().all(|lot| !lots.iter().any(|(at, before)| *at < from && before == lot));
        if n == 0 && (undone || words.windows(2).any(|pair| pair == ["sans", "suite"])) {
            return true;
        }
        if (undone || mentions_void(piece)) && !other_lot {
            return true;
        }
        from = to;
    }
    false
}

/// Issue 520 U4 (the third check): whether the rest of the name's sentence, after `end`, gives the award to
/// a second party — `et à` / `et au(x)` (not `et a été`), `ainsi qu'à`, `puis à`.
fn second_awardee_after(raw: &str, end: usize) -> bool {
    let rest = &raw[end..floor_boundary(raw, end + WINNER_SCAN)];
    let Some(first) = sentences(rest).into_iter().next() else { return false };
    let folded = fold(first);
    let words: Vec<&str> = folded.split(' ').filter(|w| !w.is_empty()).collect();
    words.windows(3).any(|w| match w {
        ["et", "a", next] => *next != "ete",
        ["et", "au" | "aux", _] | ["ainsi", "qu", "a"] | ["puis", "a", _] => true,
        _ => false,
    }) || words.ends_with(&["et", "a"])
}

/// Issue 520 U4 (the second review): whether the text between cue e's name and its `a été retenue` is an
/// apposition — closed by a `,` or a `)` (`Dupont SARL, Lyon, a été retenue`, `Dupont (69003 Lyon) a été
/// retenue`) or an address opening on a postcode (`Dupont - 69003 Lyon a été retenue`).
fn apposition(gap: &str) -> bool {
    let gap = gap.trim();
    gap.is_empty()
        || gap.ends_with([',', ')'])
        || gap.trim_start_matches(['-', '–', '—']).trim_start().starts_with(|c: char| c.is_ascii_digit())
}

/// Issue 520 U4 (cue d, the second review): whether a name that runs to the end of the text holds a
/// lower-case word no legal name joins with — prose that ran on (`Dupont sans succès`, `Dupont mais sans
/// résultat`, `Dupont en vain`), not `Société des Eaux de Marseille`, `Ets Dupont et Fils`,
/// `Société d'Exploitation des Transports`.
fn runs_on(name: &str) -> bool {
    name.split_whitespace().any(|w| {
        w.starts_with(char::is_lowercase)
            && !NAME_CONNECTORS.contains(&w)
            && !matches!(w, "et" | "la" | "le" | "les")
            && !w.contains(['\'', '’'])
    })
}

/// Issue 520 U4 (the second review): whether the name that ended where `rest` opens, at a `,` or a dash,
/// is the first of a list of names (B2's co-winners, unit-1 decision §4.3: kept as a summary) — in its
/// sentence, two items or more that open on a capital (`Dupont SARL, Martin SA et Durand SAS`, `Dupont
/// SARL - Martin SA - Durand SAS`), or one that carries a company form or that ` et ` joins (`Dupont
/// SARL, Martin SA`). Not an apposition or an address: `Dupont SARL, Lyon, a été retenue`, `Martin
/// Frères - 69003 Lyon`, `CSL Berhing, suite marché négocié`.
fn listed_names_after(rest: &str) -> bool {
    let rest = &rest[..floor_boundary(rest, WINNER_SCAN)];
    let end = [rest.find([';', '\n', '…']), rest.find(". ")].into_iter().flatten().min().unwrap_or(rest.len());
    let opens = rest[..end].trim_start();
    if !opens.starts_with([',', '-', '–', '—']) {
        return false;
    }
    let text = [" - ", " – ", " — "].iter().fold(opens.to_owned(), |text, dash| text.replace(dash, ","));
    let text = text.trim_start_matches(|c: char| c.is_whitespace() || ",-–—".contains(c));
    let (mut capitalised, mut joined, mut formed) = (0, false, false);
    for item in text.split(',') {
        for (n, part) in item.split(" et ").flat_map(|part| part.split(" ET ")).enumerate() {
            let part = part.trim();
            if part.starts_with(char::is_uppercase) {
                capitalised += 1;
                joined |= n > 0;
                formed |= company_token(part);
            }
        }
    }
    capitalised >= 2 || (capitalised == 1 && (joined || formed))
}

/// Issue 520 U4: where the name that opens at byte `from` of `raw` ends ([`award_winner`]'s terminators),
/// within [`WINNER_SCAN`] bytes:
/// - `,`, `;`, `:`, a line break (the review: `Dupont SARL\nAdresse : …`, `Dupont : 12 000 EUR HT`);
/// - after whitespace of any kind (the review: a tab or a no-break space as much as a space): `(`, a
///   dash or a `/` that whitespace follows, whole-word ` suite` and ` SISE` (ASCII case-blind), ` le
///   <digit>`, ` pour un montant` / ` pour le lot` / ` pour 48 225` / ` après négociation` / ` par
///   marché négocié` / ` par le pouvoir adjudicateur` ([`award_circumstance`]), ` en date`, ` qui`,
///   ` dont`, ` a été` / ` ont été` / ` à été` (`Dupont a été résilié`), a five-digit postcode
///   (`Assurances securité 59000 Lille`);
/// - a sentence end: a `.` after digits or after two letters or more that are no company abbreviation
///   ([`NAME_ABBREVIATIONS`]: `Ets. Dupont` goes on); after a single letter, only where whitespace and a
///   capitalised field or sentence opener follow ([`FIELD_OPENERS`]: `Martin J. Le marché`) or the
///   dotted letters spell a legal form ([`COMPANY_FORMS_DOTTED`]: `Dupont S.A. Montant`) — the dot kept;
///   since the second review, not after a legal form that opens the name (`la S.A.R.L. Dupont`, `S.A.R.L.
///   Les Jardins`) unless a field opens there (`S.A.R.L. Montant`, `S.A.R.L. Le marché`);
/// - the next cue (`cues`, where each opens, in order), or the end of the text.
fn winner_end(raw: &str, from: usize, cues: &[usize]) -> usize {
    let text = &raw[from..floor_boundary(raw, from + WINNER_SCAN)];
    let mut end = text.find([',', ';', ':', '：', '\n', '\r']).unwrap_or(text.len());
    // The alphanumeric run that opens `s`.
    fn run(s: &str) -> &str {
        &s[..s.find(|c: char| !c.is_alphanumeric()).unwrap_or(s.len())]
    }
    // A sentence end first: it bounds the scan below.
    for (at, _) in text[..end].match_indices('.') {
        let letters = text[..at].chars().rev().take_while(|c| c.is_alphabetic()).count();
        if letters != 1 {
            let opens = text[..at].char_indices().rev().nth(letters.saturating_sub(1)).map_or(at, |(k, _)| k);
            let token = &text[opens..at];
            if letters >= 2 && NAME_ABBREVIATIONS.contains(&fold(token).as_str()) {
                continue;
            }
            end = at;
            break;
        }
        // A dotted letter: the name goes on (`A.E.C. Castellane`), unless the next sentence or field opens.
        let after = &text[at + 1..];
        let opener = after.trim_start();
        if opener.len() == after.len() || !opener.starts_with(char::is_uppercase) {
            continue;
        }
        let token = text[..at].rsplit(char::is_whitespace).next().unwrap_or("");
        let dotted: String = token.chars().filter(|c| c.is_alphabetic()).collect::<String>().to_lowercase();
        let form = COMPANY_FORMS_DOTTED.contains(&dotted.as_str());
        // The second review: a dotted legal form that opens the name — only an article or a descriptor
        // before it (`la S.A.R.L. Dupont`, `Titulaire : S.A. Martin et Fils`, `la société S.A.R.L. Les
        // Jardins`) — is the name's own: it goes on, or the form alone would be the winner.
        let opens_name = fold(&text[..at - token.len()])
            .split(' ')
            .filter(|w| !w.is_empty())
            .all(|w| matches!(w, "la" | "le" | "les" | "l") || WINNER_DESCRIPTORS.contains(&w));
        let field = fold(run(opener));
        let field = FIELD_OPENERS.contains(&field.as_str()).then_some(field);
        // …unless the next field opens there (`la S.A.R.L. Montant HT`, `S.A.R.L. Le marché a été notifié`:
        // the form alone, which [`plausible_winner`] refuses); an article before a capitalised word is the
        // name's own (`la S.A.R.L. Les Jardins de Cocagne`).
        let named = field.as_deref().is_none_or(|field| {
            matches!(field, "le" | "la" | "les" | "l")
                && opener[run(opener).len()..]
                    .trim_start_matches(|c: char| c.is_whitespace() || "'’´ʼ".contains(c))
                    .starts_with(char::is_uppercase)
        });
        if form && opens_name && named {
            continue;
        }
        if field.is_some() || form {
            end = at + 1;
            break;
        }
    }
    // The folded word after the run that opens `s`, past whitespace.
    let next_word = |s: &str| {
        let rest = &s[run(s).len()..];
        let trimmed = rest.trim_start();
        (trimmed.len() < rest.len()).then(|| fold(run(trimmed)))
    };
    for (at, c) in text[..end].char_indices() {
        if !c.is_whitespace() {
            continue;
        }
        let rest = &text[at + c.len_utf8()..];
        let word = run(rest);
        let after_word = &rest[word.len()..];
        let stops = rest.starts_with('(')
            || (rest.starts_with(['-', '–', '—', '/']) && rest.chars().nth(1).is_some_and(char::is_whitespace))
            || word.eq_ignore_ascii_case("suite")
            || word.eq_ignore_ascii_case("sise")
            || (word.eq_ignore_ascii_case("le")
                && after_word.starts_with(char::is_whitespace)
                && after_word.trim_start().starts_with(|c: char| c.is_ascii_digit()))
            || award_circumstance(word, after_word)
            || matches!(word, "qui" | "dont")
            || (word == "en" && next_word(rest).as_deref() == Some("date"))
            || (matches!(word, "a" | "à" | "ont") && next_word(rest).as_deref() == Some("ete"))
            || (word.len() == 5 && word.bytes().all(|b| b.is_ascii_digit()));
        if stops {
            end = at;
            break;
        }
    }
    let next = cues.partition_point(|&cue| cue <= from);
    if let Some(&cue) = cues.get(next) {
        end = end.min(cue - from);
    }
    from + end
}

/// The largest char boundary of `s` at or below `at` (`s.len()` past its end).
fn floor_boundary(s: &str, at: usize) -> usize {
    let mut at = at.min(s.len());
    while !s.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// Issue 520 U4: whether `word` (`pour`, `après`, `par`, any case) and what follows it (`rest`) state the
/// award's circumstance, not the name's own words: a figure or a lot after `pour` (`pour un montant`,
/// `pour le lot 2`, since the second review `pour 48 225,00 € HT`, `pour 3 ans`; not `Association pour
/// la Formation`), a procedure after `après` or `par` (`après négociation`, `par marché négocié`, `par la
/// commission`; the build's deviation 4, closed by the review: `Dupont Frères après négociation` read as
/// one name), and since the second review an agent after `par` (`par le pouvoir adjudicateur`: an article
/// and a lower-case word; `Peinture par Excellence`, `par la Couleur` go on).
fn award_circumstance(word: &str, rest: &str) -> bool {
    const FIGURES: [&str; 7] = ["lot", "lots", "montant", "marche", "prix", "total", "somme"];
    const PROCEDURES: [&str; 16] = [
        "negociation", "negociations", "marche", "procedure", "appel", "mise", "consultation", "avis", "decision",
        "deliberation", "lettre", "voie", "analyse", "relance", "commission", "cao",
    ];
    let preposition = match word.len() {
        3..=6 => fold(word),
        _ => return false,
    };
    if !matches!(preposition.as_str(), "pour" | "apres" | "par") {
        return false;
    }
    // The two words after it: a short prefix of `rest` holds them.
    let prefix = &rest[..floor_boundary(rest, 48)];
    let folded = fold(prefix);
    let mut words = folded.split(' ').filter(|w| !w.is_empty());
    let (first, second) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
    let article = matches!(first, "le" | "la" | "les" | "l" | "un" | "une");
    // The third check: an amount or a duration in digits only with its unit (`pour 48 225,00 € HT`, `pour
    // 3 ans`), so `Association pour 1 avenir` keeps its words; and an agent after `par` only from a closed
    // list (`par le pouvoir adjudicateur`), so `Insertion par l'emploi`, `Réussir par le sport` keep theirs.
    const UNITS: [&str; 11] = ["ht", "ttc", "eur", "euro", "euros", "an", "ans", "mois", "jours", "semaines", "k"];
    const AGENTS: [&str; 16] = [
        "pouvoir", "acheteur", "adjudicateur", "maitre", "personne", "representant", "president", "jury",
        "collectivite", "ville", "commune", "conseil", "bureau", "comite", "direction", "assemblee",
    ];
    let figure_with_unit = || {
        let mut rest_words = folded.split(' ').filter(|w| !w.is_empty()).skip_while(|w| w.bytes().all(|b| b.is_ascii_digit()));
        first.bytes().all(|b| b.is_ascii_digit()) && rest_words.next().is_some_and(|w| UNITS.contains(&w))
    };
    match preposition.as_str() {
        "pour" => {
            matches!(first, "un" | "une" | "total" | "somme")
                || FIGURES.contains(&first)
                || (article && FIGURES.contains(&second))
                || (!first.is_empty() && figure_with_unit())
        }
        "apres" => PROCEDURES.contains(&first) || (article && PROCEDURES.contains(&second)),
        "par" => PROCEDURES.contains(&first) || (article && (PROCEDURES.contains(&second) || AGENTS.contains(&second))),
        _ => false,
    }
}

/// Issue 520 U4: `raw` without the punctuation and spaces at its edges — keeping the final dot of a
/// dotted legal form or an initial (`Dupont S.A.`, `Martin J.`; the review: the trim took it).
fn trim_winner(raw: &str) -> &str {
    let edge = |c: char| c.is_whitespace() || ".,:;-–—".contains(c);
    let name = raw.trim_matches(edge);
    let start = raw.len() - raw.trim_start_matches(edge).len();
    let end = start + name.len();
    let mut tail = name.chars().rev();
    let dotted =
        tail.next().is_some_and(char::is_alphabetic) && tail.next().is_some_and(|c| c == '.' || c.is_whitespace());
    if dotted && raw[end..].starts_with('.') { &raw[start..=end] } else { name }
}

/// Issue 520 U4: `raw` without its descriptor (`la Société`, `l'entreprise`, `L'entreprise`, `LA SOCIÉTÉ`,
/// `l'esat`, `la STÉ` — after an article any case; bare, only `société`, so `Société Générale` keeps its
/// name), unless the legal name goes on from it ([`descriptor_names`]: `la Société Générale` →
/// `Société Générale`); a lower-case article (`la Sarltacavl`; `La Poste` keeps its capital `La`); its
/// quotes (`l'esat "Les Ateliers de Polangis"`; inside the name, the review: `la société "Les Jardins"
/// SARL` → `Les Jardins SARL`, none left unbalanced); and the punctuation around it ([`trim_winner`]).
fn strip_winner(raw: &str) -> String {
    const ARTICLES: [&str; 11] = ["la ", "La ", "LA ", "l'", "L'", "l’", "L’", "l´", "L´", "lʼ", "Lʼ"];
    const QUOTES: &str = "\"«»“”„";
    let name = trim_winner(raw);
    let (article, rest) =
        ARTICLES.iter().find_map(|a| name.strip_prefix(a).map(|r| (true, r.trim_start()))).unwrap_or((false, name));
    let (noun, after) = rest.split_at(rest.find(|c: char| !c.is_alphanumeric()).unwrap_or(rest.len()));
    let descriptor = if article { WINNER_DESCRIPTORS.contains(&fold(noun).as_str()) } else { noun == "société" };
    // The second review: whether a descriptor or a lower-case article was dropped before what is left.
    let mut introduced = false;
    let name = match descriptor {
        true if descriptor_names(after) => rest,
        true if !after.is_empty() => {
            introduced = true;
            after.trim_start()
        }
        _ => name,
    };
    let name = match ["la ", "le ", "les ", "l'", "l’", "l´", "lʼ"].iter().find_map(|a| name.strip_prefix(a)) {
        Some(stripped) => {
            introduced = true;
            stripped
        }
        None => name,
    };
    let unquoted = if name.contains(|c: char| QUOTES.contains(c)) {
        name.split(|c: char| QUOTES.contains(c)).flat_map(str::split_whitespace).collect::<Vec<_>>().join(" ")
    } else {
        name.to_owned()
    };
    let out = trim_winner(unquoted.trim_matches(|c: char| c.is_whitespace() || c == '\''));
    // The second review: what a descriptor or an article introduced opens on a capital as a name does
    // (`la Société Arrow Génériques`, `la Sarltacavl`, `l'esat "Les Ateliers de Polangis"`); a lower-case
    // word there is a qualifier or a role (`l'entreprise retenue après négociation`, `la société classée
    // première`, `la société sortante`, `l'actuel prestataire`, `l'opérateur économique classé premier`),
    // which every notice writing it would share as one organization: no name.
    if introduced && out.starts_with(char::is_lowercase) { String::new() } else { out.to_owned() }
}

/// Issue 520 U4 (the review): whether the words after a descriptor (`after`, from the byte after it)
/// make it part of the legal name — whitespace, then an adjective of one ([`NAME_ADJECTIVES`]: `Société
/// Générale`) or a lower-case connector ([`NAME_CONNECTORS`]: `Société des Eaux de Marseille`).
fn descriptor_names(after: &str) -> bool {
    let next = after.trim_start();
    let word = &next[..next.find(|c: char| !c.is_alphanumeric()).unwrap_or(next.len())];
    next.len() < after.len() && (NAME_CONNECTORS.contains(&word) || NAME_ADJECTIVES.contains(&fold(word).as_str()))
}

/// Issue 520 U4: whether an extracted name can be the awardee — it opens with a letter, holds two letters
/// or digits at least and at most [`WINNER_MAX_CHARS`] characters; its first word is no count, pointer or
/// role ([`COUNT_NOUNS`], [`WINNER_POINTERS`], [`WINNER_ROLES`]); it is no descriptor alone (`la
/// société`, `l'entreprise, pour un montant`) and no company abbreviation alone (`Ets`); it does not end on a
/// connector (`Dupont et`) nor hold another party's `à` or another lot (`Dupont SARL et lot 4 à Martin
/// SA`, `Dupont et à la société Martin`, `titre provisoire à Dupont`); and it is no non-name itself
/// ([`not_a_name`], [`mentions_void`]).
///
/// Since the second review, also none of these:
/// - after a capital article, a head on those lists or a lone descriptor (`Le même`, `La même
///   entreprise`, `LA SOCIÉTÉ`; `La Poste` passes);
/// - a legal form alone (`S.A.R.L.`, `S.A.S.`, `E.T.S.`, `SARL`): every notice writing it would share one
///   organization;
/// - a status (`négociation en cours`, `Relancé en procédure négociée`, `Sera désigné ultérieurement`,
///   `Indéterminé`: [`STATUS_HEADS`], [`RELAUNCH_STEMS`], `en cours`, `en attente`), or an unaccented `A`
///   before an infinitive (`A définir`, `A DESIGNER`; `A.E.C. Castellane` passes);
/// - a possessive head (`son titulaire actuel`, `sa régie`: [`POSSESSIVES`]);
/// - a sentence that ran on: a [`NAME_TAILS`] word after the first, lower-case or in capitals (`Dupont
///   dans le cadre de`, `Dupont en procédure négociée`, `DUPONT SAS montant HT`), a figure (`€`,
///   [`FIGURE_TOKENS`]), or a number before a street word (`SARL Dupont 12 rue des Lilas`);
/// - an elided `à` (`Dupont ainsi qu'à la société Martin`), or a second party joined on (`Dupont et la
///   société Martin`, `Dupont avec la société Martin`, `Dupont SARL et Martin SA` — a company form on both
///   sides of ` et `; `Ets. Dupont et Fils` passes).
fn plausible_winner(name: &str) -> bool {
    let folded = fold(name);
    let words: Vec<&str> = folded.split(' ').filter(|w| !w.is_empty()).collect();
    let Some(&first) = words.first() else { return false };
    let core = if matches!(first, "le" | "la" | "les" | "l") && words.len() > 1 { &words[1..] } else { &words[..] };
    let raw: Vec<&str> = name.split_whitespace().collect();
    let head_refused = |head: &str| {
        COUNT_NOUNS.contains(&head)
            || WINNER_POINTERS.contains(&head)
            || WINNER_ROLES.contains(&head)
            || STATUS_HEADS.contains(&head)
            || RELAUNCH_STEMS.iter().any(|stem| head.starts_with(stem))
    };
    let joined = core.concat();
    // Every form is eight letters at most (`limited`): a longer name is no form, and the fold is spared.
    let legal_form_only = joined.len() <= 8
        && (COMPANY_FORMS_DOTTED.contains(&joined.as_str()) || COMPANY_TOKENS.iter().any(|form| fold(form) == joined));
    let tail = raw.iter().skip(1).any(|w| {
        let w = w.trim_matches(|c: char| !c.is_alphanumeric());
        NAME_TAILS.contains(&fold(w).as_str()) && (w == w.to_lowercase() || w == w.to_uppercase())
    });
    let figure = raw.iter().skip(1).any(|w| {
        let token = w.trim_matches(|c: char| ",;:()[]".contains(c));
        token.contains('€') || FIGURE_TOKENS.contains(&token) || FIGURE_TOKENS.contains(&token.trim_end_matches('.'))
    });
    let street = words.windows(2).any(|pair| {
        pair[0].starts_with(|c: char| c.is_ascii_digit()) && (STREET_WORDS.contains(&pair[1]) || pair[1] == "quai")
    });
    let status_pair = words.windows(2).any(|pair| matches!(pair, ["en", "cours" | "attente"]));
    let unaccented_a = matches!(raw.first(), Some(&("A" | "a")))
        && words.get(1).is_some_and(|w| w.len() > 3 && (w.ends_with("er") || w.ends_with("ir") || w.ends_with("re")));
    let possessive = raw.first().is_some_and(|w| POSSESSIVES.contains(w));
    let elided_to = raw.iter().any(|w| ["'à", "’à", "'À", "’À"].iter().any(|elided| w.ends_with(elided)));
    let second_party = (0..words.len()).any(|k| {
        matches!(words[k], "et" | "ou" | "ainsi" | "avec") && {
            let at = k + 1 + usize::from(words.get(k + 1).is_some_and(|w| matches!(*w, "la" | "l" | "le" | "les")));
            words.get(at).is_some_and(|w| WINNER_DESCRIPTORS.contains(w) || matches!(*w, "societes" | "entreprises"))
        }
    }) || (1..raw.len()).any(|k| {
        matches!(raw[k], "et" | "ET" | "&")
            && company_token(&raw[..k].join(" "))
            && company_token(&raw[k + 1..].join(" "))
    });
    name.starts_with(char::is_alphabetic)
        && name.chars().filter(|c| c.is_alphanumeric()).count() >= 2
        && name.chars().count() <= WINNER_MAX_CHARS
        && !head_refused(first)
        && !head_refused(core[0])
        && !(core.len() == 1 && (WINNER_DESCRIPTORS.contains(&core[0]) || NAME_ABBREVIATIONS.contains(&core[0])))
        && !legal_form_only
        && !matches!(words.last(), Some(&("et" | "ou" | "de" | "du" | "des" | "la" | "le" | "les")))
        && !name.split_whitespace().any(|w| matches!(w, "à" | "À"))
        && !elided_to
        && !words
            .windows(2)
            .any(|pair| LOT_DESIGNATORS.contains(&pair[0]) && pair[1].bytes().any(|b| b.is_ascii_digit()))
        && !status_pair
        && !unaccented_a
        && !possessive
        && !tail
        && !figure
        && !street
        && !second_party
        && not_a_name(name).is_none()
        && !mentions_void(name)
}

/// Issue 520 U4: whether a list separator (`;`, ` / `) in `text` — what follows a name's opening — is
/// followed by a segment that names something: not void, with letters (`RMI, … / 4.2 (portables): 36
/// 319,94 EUR HT`; not `Dupont SARL ; lot 2 infructueux`).
fn listed_after(text: &str) -> bool {
    text.split(';').flat_map(|segment| segment.split(" / ")).skip(1).any(|segment| {
        segment.chars().any(char::is_alphabetic) && !mentions_void(segment)
    })
}

/// Issue 520 U4 (the review): whether a lot entry that no cue reads names a winner of its own — the text
/// is then a per-lot list (`Lot 1) attribué à Dupont. Lot 2) Martin. Lot 3) infructueux`, `Lot 2 attribué
/// à Dupont, lot 3 : Martin SARL, lot 4 infructueux`), whose other awardees one name would drop: a B or C
/// list never extracts (unit-1 decision §4.3, §4.4). Entries as [`every_lot_void`] reads them
/// ([`lot_label`], `raw`'s `runs` and their folded `words`); one that holds a cue's start (`cues`, in
/// order), or opens in a cue's clause (`attribué à la société Dupont pour le lot 2`), is that cue's. An
/// entry is no winner when its head is void or opens on a relaunch ([`RELAUNCH_STEMS`]: `Lot 31 déclaré
/// sans suite`, `lot 30 relancé en marché négocié 35 II 3`, `Le lot 32 a été déclaré infructueux`), or
/// when a conjunction alone joins it to the next. An entry with nothing after its label (a list the slot
/// cut short: `Lots 2)`) may name anyone, so it counts.
fn lot_entry_names_another(raw: &str, runs: &[(usize, usize)], words: &[&str], cues: &[usize]) -> bool {
    let mut entries: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        match lot_label(raw, runs, words, i) {
            Some(after) => {
                entries.push((i, after));
                i = after;
            }
            None => i += 1,
        }
    }
    entries.iter().enumerate().any(|(k, &(open, after))| {
        let (start, end) = (runs[open].0, entries.get(k + 1).map_or(raw.len(), |&(next, _)| runs[next].0));
        // A cue's own entry: the cue opens in it, or the entry opens in the cue's clause (`attribué à la
        // société Dupont pour le lot 2`; not `attribué à Dupont, lot 3 : Martin SARL`).
        let before = cues.partition_point(|&cue| cue < start);
        let in_clause = before > 0 && {
            let cue = cues[before - 1];
            start - cue <= WINNER_SCAN && !clause_break(&raw[cue..start])
        };
        if in_clause || cues.get(before).is_some_and(|&cue| cue < end) {
            return false;
        }
        let text = raw[runs.get(after).map_or(raw.len(), |run| run.0).min(end)..end]
            .trim_end_matches(|c: char| c.is_whitespace() || "-–—/.,;:".contains(c));
        let folded = fold(text);
        if folded.is_empty() {
            return true;
        }
        if folded.split(' ').all(|w| LOT_CONJUNCTIONS.contains(&w)) {
            return k + 1 == entries.len();
        }
        let head = reason_head(text).unwrap_or(text);
        let opens = fold(head);
        let opens = opens.split(' ').next().unwrap_or("");
        !(mentions_void(head) || RELAUNCH_STEMS.iter().any(|stem| opens.starts_with(stem)))
    })
}

#[cfg(test)]
mod tests {
    use super::{
        BLANK_STEMS, BLANK_WHOLE, FORM_SECTION_TITLES, NO_INFORMATION_TAILS, NotAName, PERFIL_STEMS, PLACEHOLDER_STEMS,
        PLACEHOLDER_WHOLE, POINTER_LEAD_INS, POINTER_STEMS, REFERENCE_HEAD_WORDS, REFERENCE_LISTS, REFERENCE_LEAD_INS,
        ReferenceShape, SCOPED_WHOLE, VOID_STEMS, VOID_TEMPLATES, VOID_WHOLE, fold, is_exact_entry, mentions_placeholder,
        mentions_void,
        not_a_name, not_a_name_in, section_reference, section_reference_pointer,
    };

    /// One prod specimen per entry, and the spellings the 508 lists missed.
    #[test]
    fn every_measured_void_lot_spelling_is_refused() {
        for void in [
            "Infructueux",
            "INFRUCTUEUX",
            "infructueux — relance en marché négocié",
            "Lot déclaré infructueux",
            "Lot déclaré infructeux",
            "Sans suite",
            "Procédure déclarée sans suite",
            "Sans objet (lot déclaré sans suite)",
            "Non attribué",
            "Non-attribué",
            "non attribué à ce jour",
            "Declarado desierto",
            "Declarada desierta",
            "Queda desierto",
            "Nessuna aggiudicazione",
            "Desierto",
            "Desierto.",
            "desierto (lote 3)",
            "Desierto (lotes VIII y IX)",
            "Desiertos (lotes 3 y 5)",
            "DESERTO",
            "Deserta",
            "Lote desierto",
            "Lotto deserto",
            "Gara deserta",
            "Non aggiudicato",
            "Lotto non aggiudicato",
            "Not awarded",
            "Lot not awarded",
            "No award",
            "No award made",
            "Contract not awarded",
            "No contract awarded",
            "Niet gegund",
            "Unieważniony",
            "Unieważniono",
            "Postępowanie unieważnione",
            "brak ofert",
            "Aucune offre",
            "Aucune offre reçue",
            "Pas d'attributaire",
            "Pas d'offre",
            "Sans offre",
            "abandon",
            "Aufgehoben",
            "Nicht vergeben",
        ] {
            assert_eq!(not_a_name(void), Some(NotAName::VoidLot), "{void}");
        }
    }

    /// A void phrase that goes on to name the company the lot was then awarded to: an award
    /// summary (issue 511 split it out of `Placeholder`; the fold keeps it).
    #[test]
    fn a_void_phrase_that_goes_on_to_name_an_award_is_an_award_summary() {
        for awarded in [
            "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Sandoz",
            "Infructueux, marché attribué à: Berthelet",
            "lot infructueux, relancé et attribuée à LD Bio Diagnostics",
            "Sans suite, marché passé avec la société Dupont",
        ] {
            assert_eq!(not_a_name(awarded), Some(NotAName::AwardSummary), "{awarded}");
        }
        assert_eq!(not_a_name("Non attribué à ce jour"), Some(NotAName::VoidLot), "a negated award is no award");
    }

    /// The first drain's dry run (job 2164): a "name" that summarises several lots, or
    /// names a company beside the void phrase, is not a void lot — dropping it would turn
    /// the real awards it lists into `clos-nw`. Each is a prod specimen, and an award
    /// summary since issue 511 (kept by the fold until per-lot parsing exists). The pure
    /// void sentences beside them (French possessive `sa` included) stay void.
    #[test]
    fn a_summary_that_names_an_award_beside_a_void_lot_is_an_award_summary() {
        for summary in [
            "Lot 1) Sarl Bremond. Lot 2) S.A. Les Rapides Varois. Lot 3) Déclaré Infructueux. Lot 4) Déclaré Infructueux",
            "Lot 1: Yvelin SA. Lot 2: infructueux",
            "Lot 1) Actiforest. Lot 2) Infructueux. Lot 3) ETS Guintoli",
            "Lot 3) Ce lot a été déclaré sans suite. Marché n° 411226 lot 4) Paget SA",
            "sous-lot 5B) infructueux. sous-lot 5C) Jacquinot. Lot 6: sous-lot 6A) SIA Revêtements",
            "Sublote 12.1: Bernadí, S.A.; Expert Line, S.L.; S&T 96, S.L. Sublote 12.2: Declarado desierto.",
            "Importaciones Canarias de Automóviles SA (Lote A2.5). Se ha declarado desierto el Lote A3.2.",
            "Stryker France SAS: lot nº 36 sans suite: lot nº 37",
            "lot 4) Marché infructueux. Marché 4: SA Cofida d'Hauwers",
            "Lots 1) et 2) déclarés infructueux. Lot 3) attribution à: SA Maîtres Laitiers Distribution",
            "2013023201/zone Belley-Bas Bugey : Sarl Mcb - 01300 Chazey Bons ; 2013073202/zone Bresse : infructueux",
            "2013061903/zone Mâcon : infructueux ; 2013061904/zone Paray le Monial : Cd'Elec - 71600 Paray-le-Monial",
            "GRPT vallée sas (mandataire), La Comec et Arbat System, suite à procédure négociée après AO infructueux",
            "Lot 1) Dupont. Lot 2) infructueux",
            // The second review's shapes (wf_d9c376dd-312).
            "Yvelin SA. Lot 2 : infructueux",
            "Rossi s.r.l. (lotto 2 nessuna aggiudicazione)",
            "ACME S.A.R.L (lot 2 infructueux)",
            "Construcciones Pérez, S.A.U. (lote 2 declarado desierto)",
            "Lote I: Construcciones Pérez. Lote II: declarado desierto",
            "Lot1 : Dupont. Lot2 : infructueux",
            "Lote12 : Dupont. Lote13 : declarado desierto",
            "Lote n.º 1: Construcciones Pérez. Lote n.º 2: declarado desierto",
            "Tranche 1 : Dupont. Tranche 2 : infructueux",
            "lot 9.02.01 : sans suite Lot 9.02.02 : Vygon",
            "Lot 2002-01: infructeux. Lots 2002-03 et 2002-06: Faurie Midi-Pyrénées",
            "Gaudais Distribution (lots 1-1, 1-3, 1-5) Transgourmet (lot 1-6) lot 1-7 est déclaré sans suite",
            "lot1096 INFRUCTUEUX- lot1097 fresenius k 22500- lot1098 fresenius m 396",
            "Lote 2 declarado desierto; lote 1 adjudicado a Construcciones Pérez",
            "Lotto 2 nessuna aggiudicazione, lotto 1 aggiudicato alla ditta Rossi",
            // The cohort read of the second dry run (wf_bc5a1594-f9c), prod specimens.
            "BGE Guyane 16 rue Lt Becker 97300 Cayenne / La consultation a été déclarée sans suite pour les parts non couvertes des besoins",
            "1/ Gidef, 37 rue du chemin latérale, 93140 Bondy - 2/ Auxet, Bât 1, 97139 Les Abymes. La consultation a été déclarée sans suite",
            "lot 43 INFRUCTUEUX - 44 reckitt benckiser 28692.07 - 45 reckitt b 165.24 - 46 ipsen p 2497.138- 47 ASTELLAS 590.15",
            "lot109 BAYER 5530.56 - 110 INFRUCTUEUX - 111cooper 6243.12 - 112 leo p 977",
            "lot268 sanofi av - 269sanofi AV - 270renaudin - 282 sans suite",
            "74 secteur Sud: ACS DECO (+5 %) et non attribué (infructueux) 74 secteur Nord: ACS DECO (+3 %) et non attribué (infructueux)",
            "Zundel Et Kohler. SECTION VI: OTHER INFORMATION VI.7) Other information: Lots Infructueux (sans relance en marché négocié): 001",
            "Getinge France. SECTION VI: OTHER INFORMATION VI.7) Other information: Tous les autres lots (sur 123 lots au total) sont infructueux.",
            "Lot A) Rivadis. Lot B) Ontex. Lot C) Maury. Lot E) infructueux.",
            "A) Coca à Lille. B) Placé: sans suite. C) SRTT à Grande-Synthe.",
            "Procédure de marché négocié après appel d'offres infructueux: 5 Mepy Système",
            "Rang 1 Helmlinger. Rang 2 Jonnette. Rang 3 infructueux",
            "SPIE secteurs 5A à 5D/ infructueux 5E 5F/ Marc Elec 5G/ Chenelec 5H",
            "Secteur Allier: grange (0 %) et Fayolle (-4 %) Secteur Puy-de-Dôme: lot non attribué (infructueux)",
            "A) non attribué. B1) et B2) Climelec",
            "Agence d'Amboise : appel d'offres déclaré sans suite - Agence de Chinon : savelys GDF suez",
            "Mongin Jauffret : lots 1504 et 1505 Zi Delta industrie la Valentine lots infructueux : 1501, 1502 et 1503",
            "3a) SIAL 54520 3b) lot infructueux",
            "3a: SIAL 54520 - 3b: lot infructueux",
            // The fourth dry run (job 2173): Dutch forms and lots. (`Humble Inpect B.V. (niet gegunde
            // partij)` reads as a plain name since issue 527: TenderNed's mark is no void verdict.)
            "Perceel 1: Agens de werkende kracht B.V. en SDW Arbeidsintegratie BV. Voor perceel 2 hebben wij geen geldige aanbieding ontvangen. Perceel 2 is derhalve niet gegund",
            // The seventh dry run (job 2182): the Polish company form.
            "Olprint Sp. z o.o. unieważnienie art. 94 ust. 1 pkt 4",
            "Lot 2 infructueux. V.1) Award and contract value V.1.1) Name and address of successful supplier: Dupont",
        ] {
            assert_eq!(not_a_name(summary), Some(NotAName::AwardSummary), "{summary}");
        }
        for void in [
            "Lot déclaré \"sans suite\" par l'Assemblée Départementale, lors de sa séance publique du 18.7.2011",
            "le lot 42 : Prestations Sur Vl Et Vu- fresnay Sur Sarthe Et Sa Peripherie a été déclaré Infructueux",
            "Aucune réponse reçue - lot déclaré sans suite sous sa forme réservée et à relancer prochainement",
            "Marché déclaré infructueux par la commission d'appel d'offres en sa séance du 07.10.2002",
            "Lot 2 infructueux",
            "Lots nº 2, 3 et 5 déclarés infructueux",
            "Declarado desierto (lotes 3 y 4)",
            "pas d'attributaire, lot infructueux",
            "Lote no adjudicado, declarado desierto",
            "Lot infructueux (not awarded)",
            "Lotto deserto (non aggiudicato)",
            "Lotto deserto - non aggiudicato per importo offerto superiore alla base d'asta",
            "Niet gegund - perceel teruggetrokken door aanbestedende dienst",
            "Niet gegung",
            "Niet gegunde percelen",
            "Niet toegewezen",
            "Not awarded - no compliant bids",
            "Aufgehoben (siehe VI.3)",
            // The fifth dry run's survivors (job 2174 → Verify).
            "Desierto al no haberse presentado ninguna oferta",
            "Lotto deserto in quanto non è pervenuta alcuna offerta valida",
            "Brak ofert na zadanie nr 1.",
            "BRAK OFERT -unieważniona na podstawie art. 93 ust. 1 pkt 1 uPzp",
            "Desierto / Ninguna oferta adecuada",
            // The sixth dry run's survivors.
            "DESIERTO POR NO PRESENTARSE LICITADORES",
            "BRAK OFERT-unieważniona na podstawie art. 93 ust. 1 pkt. 1 uPzp",
            "Brak ofert V.1) Award and contract value V.1.1) Name and address of successful supplier",
            "Brak oferty",
            "Uniewaniono postepowanie",
            "Unieważnia postępowanie przetargowe na podstawie art. 93 ust. 1",
            "UNIEWAZNIONO POSTĘPOWANIE NA PODSTAWIE ART.93 UST.1 PKT 1",
            "Niet gunning",
            "No contract awarded due to budgetary constraints",
            "aucune offre retenue",
            "Perceel 2 en 3 zijn niet gegund",
            "De percelen 2, 3, 4, 5 en 6 zijn overeenkomstig het bepaalde in het bestek niet gegund",
            "De opdracht wordt niet gegund aan Entropia op basis van deze vooraankondiging",
            "Lot 3) Ce lot 3 est déclaré infructueux",
            "Le lot 10: lot n° 10: CQP Vienne a été déclaré infructueux",
            "Le lot 48: Transport scolaire circuit: Ste Marie de Figaniella / Propriano a été déclaré Infructueux",
            "Le lot 3: assurance des véhicules. Durée du marché: 5 ans à compter du 1.1.2011 a été déclaré infructueux",
            "Lots 2 - 3 - 5 : infructueux",
            "Lot 2 - Fourniture de mobilier - déclaré infructueux",
            "Le lot 3 : zone nord a été déclaré infructueux",
            "Lot 4 : déclaré sans suite (motif : a) absence d'offre)",
            "Le lot 66 menuiserie bois et PVC secteur 6 a été déclaré infructueux",
            "Lot 3 sans suite (chemises). V.1) Award and contract value V.1.1) Name and address of successful supplier",
            "Lots infructueux : 1501, 1502 et 1503",
            "Lot non attribué relancé en marché négocié suite à appel d'offres infructueux",
            "Aucune candidature n'a été retenue, le lot a été déclaré infructueux",
            "LOT DÉCLARÉ SANS SUITE LORS DE SA SÉANCE DU 3 MAI",
            "Infructueux ; relance en procédure adaptée",
            "Lot infructueux faute d'attributaire",
            "Declarado desierto, no hay adjudicatario",
            "Nessuna aggiudicazione, non è stato individuato alcun aggiudicatario",
            // Issue 520 D6c: the cohort read's failed bidder, named before its lot's void verdict (the
            // relaunch's deadline fell after the notice), read as a summary until then.
            "Société Sindarro Lot déclaré infructueux relancé en marché négocié avec publicité",
        ] {
            assert_eq!(not_a_name(void), Some(NotAName::VoidLot), "{void}");
            assert!(super::mentions_void(void), "{void}");
        }
        assert_eq!(
            not_a_name("Sublote 9.1: Declarado desierto. Sublote 9.2: Declarado desierto."),
            Some(NotAName::VoidLot),
            "issue 520 D4c: a summary of void lots only is void (kept until then, the accepted cost)"
        );
        assert!(super::mentions_void("Lot 1) Dupont. Lot 2) infructueux"), "the walk's net still catches a summary");
        assert!(!super::mentions_void("ACME SA"));
    }

    /// The two-sided check of issue 520's narrowings: each name `names` lists reads `class`. An id
    /// is the walk's org (520 unit 1, `unit1-classified.json`; the name as the dry report
    /// `void-names-refold-after-dry.json` prints it); id 0 marks a near miss written for the test.
    fn assert_class(names: &[(u64, &str)], class: Option<NotAName>) {
        for (org, name) in names {
            assert_eq!(not_a_name(name), class, "{org}: {name}");
        }
    }

    /// Issue 520 U4: [`assert_class`], and the A1 extractor names no winner in any of `names`
    /// ([`super::award_winner`]) — every class of unit 1 but A1: the D void prose, the B lists and
    /// co-winners, the C `Lot n)` lists, the F rows, the A2 names before a void tail and the D residual.
    /// Since U6 the A2 extractor names none either ([`super::void_tail_winner`]), but on A2's reliable
    /// subset ([`void_tail_winner_cuts_the_reliable_a2_tails`]).
    fn assert_class_and_no_winner(names: &[(u64, &str)], class: Option<NotAName>) {
        assert_class(names, class);
        for (org, name) in names {
            assert_eq!(super::award_winner(name), None, "{org}: {name}");
            assert_eq!(super::void_tail_winner(name), None, "{org}: {name}");
        }
    }

    /// Issue 520 D1 (unit-1 decision §4.1, 68 orgs / 70 mentions): BOAMP's void sentence for one
    /// lot, `Le lot <n> : <title> a été déclaré infructueux`, is void whatever its title holds — a
    /// second designator, a `;`, `titulaire`, a brand, the typo `SAU`, bullets. Two openers, no
    /// opener, or an award clause keep the summary.
    #[test]
    fn one_lots_void_sentence_is_void() {
        assert_class(
            &[
                (3666893, "Le lot 102 : lot B10: Menuiseries bois PVC serrurerie vitrerie (Genevois) a été déclaré infructueux"),
                (31481092, "Le lot 6: Lot 2011-231 01 Communauté de Communes du Genevois a été déclaré Infructueux"),
                (4133918, "Le lot 91 : lot 9 A: Fourniture de pièces détachées pour l'entretien d'engins et matériel de marque JCB (type chargeur ou centrale hydraulique) a été déclaré infructueux"),
                (1513898, "Le lot 3: Diagnostics sur bâtiments occupés à usage principal d'habitation Le présent lot a pour objet la réalisation par le titulaire de diagnostics sur bâtiments occupés à usage principal d'habitation à réaliser au fur et à mesure des besoins de Paris habitat-OPH. a été déclaré sans suite"),
                (5772779, "le lot 002 : B Gros oeuvre étendu : Fondations spéciales (sous-lot nob02) - gros oeuvre (sous-lot nob03) - charpente métallique (sous-lot nob04) - etanchéité (sous-lot nob05) - façade brique (sous lot nob06) a été déclaré sans suite"),
                (5101244, "Le lot 6: livres scolaires et documents pédagogiques d'accompagnement (SAU ouvrages distribués exclusivement par les éditeurs eux-mêmes et livres de fonction et documentaires pour les bibliothèques des écoles) a été déclaré sans suite"),
                (1392146, "Le lot 1: secteur du canton d'Evian - Rd1005; Rd24 et Rd32 a été déclaré Infructueux"),
                (3107850, "le lot 8 : Electricite, Courant Fort - courant Faible ; a été déclaré Infructueux"),
                (1537987, "le lot 1: Rédaction d'un feuillet du magazine qui sera constitué au maximum de: - 2 doubles pages de portraits d'entreprises, associations, clubs de sports .... - 1 double page patrimoine, faune ou flore du département a été déclaré Infructueux"),
                (2021395, "le lot 12: sans objet se reporter au lot no9 plomberie sanitaire a été déclaré infructueux"),
                (4133921, "Le lot 11 : Fourniture de pièces détachées et prestations de maintenance pour l'entretien d'engins de TP de marque O&Amp;K (type Niveleuse) a été déclaré infructueux"),
            ],
            Some(NotAName::VoidLot),
        );
        assert_class(
            &[
                (31738609, "lot 25) la société Storz a été retenue. Le lot 32 a été déclaré infructueux"),
                (0, "Le lot 3: Fourniture de mobilier. Le lot 4: Dupont SARL a été déclaré infructueux"),
                (0, "Le lot 2 : attribué à Dupont SARL ; le lot 2b a été déclaré infructueux"),
                // The U2 review: a second lot written without its colon is a second lot, and so is
                // a verdict whose subject is another lot (`; lot 2 a été déclaré`).
                (0, "Le lot 1 : Keolis. Le lot 2 a été déclaré infructueux"),
                (0, "Le lot 1 : Keolis, le lot 2 a été déclaré infructueux"),
                (0, "Le lot 1 : Société Dupont ; le lot 2 a été déclaré infructueux"),
                (0, "Le lot 1 : Dupont SARL. Les lots 2 à 4 : sans suite. Le lot 5 a été déclaré infructueux"),
                (0, "Le lot 1 : Menuiserie Dupont SARL - 69000 Lyon. Le lot 2 a été déclaré infructueux"),
                (0, "Le lot 1 : Attributaire : Martin SARL. Le lot 2 a été déclaré infructueux"),
                (0, "Le lot 1 : attribution à Martin SARL. Le lot 2 a été déclaré sans suite"),
                (0, "Le lot 1 : Keolis ; lot 2 a été déclaré infructueux"),
                // The second review: a later sentence or dashed piece that opens on another lot, `le`
                // or not, a title between its number and the verdict.
                (0, "Le lot 1 : Gros oeuvre - Titulaire : Dupont SARL. Lot 2 : Charpente a été déclaré infructueux"),
                (0, "Le lot 1 : Eiffage Construction SAS. Lot 2 : Electricité a été déclaré infructueux."),
                (0, "Le lot 1 : Peinture : Dupont SARL ; lot 2 (Sols) a été déclaré infructueux"),
                (0, "Le lot 2 : Keolis SA ; lot 3 - Transport scolaire a été déclaré infructueux"),
                (0, "Le lot 1 : Voirie, entreprise retenue : Colas. Lot 2 : Réseaux a été déclaré infructueux"),
                (0, "Le lot 1 : Dupont SARL - lot 2 (Sols) a été déclaré infructueux"),
                (0, "Le lot 1 : Dupont SARL – lot n° 2 : Sols a été déclaré sans suite"),
                (0, "Le lot 1 : Dupont SARL. Lot2 : Sols a été déclaré infructueux"),
            ],
            Some(NotAName::AwardSummary),
        );
        // A later piece that opens on a lot word but no lot number is the title's.
        assert_class(
            &[
                (0, "Le lot 4 : Gros oeuvre - lots techniques a été déclaré infructueux"),
                (0, "Le lot 4 : Voirie ; lotissement des Prés a été déclaré infructueux"),
            ],
            Some(NotAName::VoidLot),
        );
        // A sub-lot code closing the title is no other lot (5772779 above: `(sous lot nob06) a été
        // déclaré sans suite`); a sub-lot number is.
        assert_class(&[(0, "Le lot 3 : façade brique (sous lot nob06) a été déclaré sans suite")], Some(NotAName::VoidLot));
        assert_class(&[(0, "Le lot 3 : Dupont SARL ; sous lot 3b a été déclaré sans suite")], Some(NotAName::AwardSummary));
    }

    /// Issue 520 D2 (17 orgs): `award_clause` reads the award it states — `n'a pas été attribué` is
    /// negated by the nearest word no filler is (`pas`), `déjà attribué` is another procedure's
    /// award, and `avec la société` after `relancer` / `négocier` / `sera` in its sentence names whom
    /// the buyer will negotiate with. A past relaunch's `avec la société`, an award after the void
    /// call, and an award word with no negation keep the party.
    #[test]
    fn a_negated_earlier_or_intended_award_is_no_award() {
        assert_class(
            &[
                (1171947, "Procédure incomplète. Le marché n'a pas été attribué. Appel d'offres sans suite"),
                (7093933, "Le lot n° 2 n'a pas été attribué et fut déclaré sans suite"),
                (31674188, "A 97.23.18) sans suite (déjà attribué au lots A 06 et A 12)"),
                (3526991, "décalré infructueux= le lot 2 infructueux et de le relancer en marché négocié en application de l'article 35 - iI - 3ème alinéa du code des marchés publics, avec la société LHENRY"),
                (4032000, "Offres déclarées irrégulières. Lot déclaré donc infructueux. Il a donc été proposé de négocier avec les sociétés candidates dans le cadre de la procédure prévue à l'article 35-I-1 3ème alinéa du code des marchés publics par une lettre de consultation dont le formalisme est prévu à l'article 66 du co"),
                (2131011, "Le marché n'a pas été attribué: la procédure est déclarée infructueuse le 17.4.2012"),
                (31400142, "Le Lot n'a pas été attribué (lot infructueux)"),
                (6069583, "Le lot n° 2 du marché n'a pas été attribué. Le lot n° 2 est infructueux"),
                (31674189, "A 97.23.19) sans suite (déjà attribué au lot A 12)"),
                (6011832, "le lot 01: Terrassement substitution de sol: Lot déjà attribué a été déclaré sans suite"),
                (3526206, "Déclaré infructueux - à relancer en marché négocié, en application de l'article 35 - i - 1er alinéa du code des marchés publics, avec les sociétés ayant soumissionné à l'appel d'offre : lot 1 : société TIB"),
            ],
            Some(NotAName::VoidLot),
        );
        assert_class(
            &[
                (1435457, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Sandoz suite à procédure négociée avec publicité avec mise en concurrence (IAP 11-32 C)"),
                (1397079, "Lot déclaré infructueux lors de l'appel d'offre puis relancé en procédure négocié sans mise en concurrence et sans publicité (art.35-2.3) avec la société B BRAUN"),
                (0, "Sans suite, marché passé avec la société Dupont"),
                (4552687, "marché déclaré infructueux et relancé sous la forme d'un marché négocié article 33-i1°2ème du décret 2005-1742 du 32/12/2005.marché attribué le 9/12/13 à Assurances securité 59000 Lille"),
                // 520's residual and F-P rows: the award word is not negated, so they stay kept.
                (4409817, "Projet déclaré sans suite : relancé sous le n° de projet ESID 12 418, attribué le 28.10.2013"),
                (1341883, "Demander le détail à la cellule marchés sont établis 66 marchés pour l'ensemble des 257 lots attribués Les 74 lots infructueux sont relancés 2 lots sans suite (n° 12 et 300)"),
                (31486027, "attribué par marché négocié passé selon art. 35-I.1 du CMP suite à appel d'offres infructueux (publicité AAPC parue dans BOAMP no 83 du 28.4.2011 - annonce no 221 et JOUE no 2011/S 81-133894 du 27.4.2011)"),
                // The U2 review: the negation is read in the verb's clause and not through a noun —
                // these negate the offer, not the award — and `ne … que` restricts, it does not negate.
                (0, "Lot infructueux faute d'offre, attribué à Dupont Frères après négociation"),
                (0, "Infructueux (pas d'offre), attribué à Dupont Frères après négociation"),
                (0, "Infructueux : aucune offre, attribué à Promocash par marché négocié"),
                (0, "Infructueux faute d'offres. Attribué ensuite à Dupont Frères"),
                (0, "Lot déclaré infructueux (aucune offre). Attribué après négociation à la société Martin."),
                (0, "Infructueux faute de candidature. Le contrat a été attribué à Dupont."),
                (0, "Lot déclaré infructueux en première phase, il n'a été attribué qu'après négociation à la société Martin"),
                // A relaunch intent governs nothing past an award made, or past a new lot.
                (0, "Lot infructueux. Après avoir décidé de relancer en procédure négociée, le marché a été conclu avec la société Dupont"),
                (0, "Lot 1 infructueux : à relancer.Lot 2 : marché passé avec la société Martin"),
                // The second review: `passé` with no future before it is an award made; `ne … que`
                // restricts across a comma; `déjà` before an award to a party is no other procedure's.
                (0, "Infructueux. Décision de relancer en procédure négociée : marché passé avec la société Dupont"),
                (0, "Infructueux. Le lot n'a été attribué, après négociation, qu'à la société Martin"),
                (0, "Lot initialement déclaré infructueux, il a depuis déjà été attribué à l'entreprise Dupont Frères après négociation"),
            ],
            Some(NotAName::AwardSummary),
        );
        // The second review: 510's reading is the floor — `non` right before the verb negates it
        // across a spaced hyphen, a line break and a dash; a `que` restricts only an `n'` (`ne … pas`
        // and `non` stay negations); a future still governs `passé`; and `déjà attribué` to lots
        // stays another procedure's.
        assert_class(
            &[
                (0, "Lot 2 : Non - Attribué"),
                (0, "Lot 2 : non\nattribué"),
                (0, "Lot 4 – NON – ATTRIBUÉ"),
                (0, "Lot 2 non attribué, que la commission relancera"),
                (0, "Lot infructueux : le lot n'a pas été attribué, que la commission relancera"),
                (0, "Lot infructueux, à relancer : le marché sera ensuite passé avec la société Dupont"),
                (0, "Lot sans suite, déjà attribué aux lots 3 et 4"),
            ],
            Some(NotAName::VoidLot),
        );
        // The 7151483 template without its middle sentence names its winner (a plain name after a
        // void prefix), as 510 read it.
        assert_class(
            &[
                (0, "Pas d'offre. Attribué à Léo Lagrange"),
                (0, "Pas d'offre - attribué à l'entreprise Promocash après négociation"),
            ],
            None,
        );
        // `déjà` read past the fillers, as the negation is.
        assert_class(&[(0, "Lot sans suite (déjà été attribué au lot A 12)")], Some(NotAName::VoidLot));
    }

    /// Whether `name`'s awardee word names a party (issue 520 D3, `award_awardee_noun`).
    fn awardee_labels(name: &str) -> bool {
        let folded = fold(name);
        super::award_awardee_noun(name, &folded.split(' ').collect::<Vec<_>>())
    }

    /// Issue 520 D3 (39 orgs / 47 mentions): an awardee word names a party only as a label, as
    /// `(mandataire)`, as `attribution à`, or as a participle before the party's preposition — not
    /// in `en cours d'attribution`, `établir l'attribution`, `l'attributaire pressenti`, `la société
    /// attributaire` in liquidation, `désistement du titulaire`, `propuesto adjudicatario` or
    /// `awarded as part of a new project`.
    #[test]
    fn an_awardee_word_must_label_a_party() {
        let void = [
            (4774593, "décision de déclarer infructueux le lot nº 10 - plomberie sanitaire pour le motif suivant : toutes les offres réceptionnées sur ce lot ont été déclarées soit irrégulières soit inacceptables et ne permettent pas d'en établir l'attribution, de plus les crédits budgétaires alloués au marché après évalu"),
            (6286857, "Suite à la cession de l'entreprise pressentie attributaire et conformément à l'article 59.IV du code des marchés publics, le marché est déclaré sans suite pour motif d'intérêt général. Avis nº 14-167620 publié au BOAMP et au JOUE"),
            (31489554, "Lot infructueux. En cours d'attribution, suite marché négocié sans publicité."),
            (7596880, "Not awarded (was retended and awarded as part of a new project)"),
            (10528138, "Desierto, al retirar su oferta el único licitador propuesto adjudicatario"),
            (5611738, "Sans suite, en cours d'attribution"),
            (6499376, "infructueux suite à désistement du titulaire"),
            (31383538, "Lot infructueux et relancer selon une procédure négocié après publicité préalable et mise en concurence selon l'article 35-1 du code des marchés publics. Voir l'avis d'attribution no 10-279996 pour l'entreprise attributaire du lot."),
            (7395857, "Declaré sans suite en raison de la liquidation judiciaire de la société attributaire"),
            (6654938, "L'attributaire pressenti pour ce lot ayant modifié son offre, elle ne correspond plus au besoin du pouvoir adjudicateur. Lot declare sans suite"),
            (1633917, "résultat du concours: déclaration sans suite pour motif de défaillance du mandataire de l'équipe, suite à la liquidation judiciaire de l'agence Toth-Fasquelle préalablement à la signature du marché de maîtrise d'oeuvre"),
            (28821451, "Not Awarded - Other FE College awarded area. V.5) THE CONTRACT IS LIKELY TO BE SUB-CONTRACTED: No"),
        ];
        assert_class(&void, Some(NotAName::VoidLot));
        for (org, name) in void {
            assert!(!awardee_labels(name), "{org}: {name}");
        }
        // Each names its awardee by a label (`Nom du titulaire:`, `Attributaire:`, `fournisseur retenu =`).
        let labelled = [(6191476, "Déclaré infructueux. Consultation relancée en procédure adaptée. Nom du titulaire: Keolis"), (7030205, "Lot infructueux relancé suivant procédure négociée sans publicité et sans concurrence en application de l'article 35 Ii-3 du code des marchés publics par lettre de consultation du 9.1.2015 et lettre de négociation du 07/04/15. Attributaire: ARDP"), (3982542, "infructueux. Relancé sous forme de marché négocié : 013086 - fournisseur retenu = zoll medical France"), (7805728, "Marché infructueux, recours à procédure négociée: titulaire: SMACL"), (1621278, "ce lot a été déclaré infructueux par la CAO et une procédure négociée à été mise en place conformément à l'art 35 du CMP et sans nouvelle mesure de publicité. Nom de l'attributaire: Brake France Service"), (5425803, "Lot 10a: déclaration sans suite lot 10b: procédure infructueuse lot 10c titulaire 1: SAGA titulaire 2: Chapelec")];
        assert_class(&labelled, Some(NotAName::AwardSummary));
        for (org, name) in labelled {
            assert!(awardee_labels(name), "{org}: {name}");
        }
        for name in [
            "GRPT GCC (mandataire) et Andre BTP, suite à procédure négociée après AO infructueux",
            "Lots 1) et 2) déclarés infructueux. Lot 3) attribution à: SA Maîtres Laitiers Distribution",
            "Lote 2 declarado desierto; lote 1 adjudicado a Construcciones Pérez",
            "Lotto 2 nessuna aggiudicazione, lotto 1 aggiudicato alla ditta Rossi",
            "Lot 2 not awarded; lot 1 awarded to Acme Ltd",
            "Perceel 2 niet gegund; perceel 1 gegund aan Bouwbedrijf Jansen",
        ] {
            assert!(awardee_labels(name), "{name}");
        }
        assert!(!awardee_labels("Infructueux. Date d'attribution : CAO du 17.4.2013."), "an attribution label heads a date");
        assert!(!awardee_labels("Pas d'attributaire, lot infructueux"), "510's negation stays");
        // The U2 review: a label with a plural mark, an entry number, a short complement, a dash or
        // a full-width colon; a participle whose preposition follows a date or an adverb; `retenu(e)`
        // after a copula or a name; a noun right before its party or a company word.
        let review = [
            "Lot infructueux relancé en procédure négociée. Attributaire(s) : Keolis",
            "Lot infructueux relancé en procédure négociée. Titulaire(s) : Keolis",
            "Lot infructueux relancé en procédure négociée. Titulaire du marché : Keolis",
            "Lot infructueux relancé en procédure négociée. Nom du titulaire / organisme : Keolis",
            "Lot infructueux relancé en procédure négociée. Attributaire n° 1 : Keolis",
            "Lot infructueux relancé en procédure négociée. Attributaire - Keolis",
            "Lot infructueux relancé en procédure négociée. Attributaire： Keolis",
            "Marché infructueux, recours à procédure négociée: titulaire n°1 : SMACL",
            "Lot infructueux relancé. Attributaire après négociation : Dupont Frères",
            "Infructueux. Relancé en procédure négociée, titulaire Dupont Frères",
            "Infructueux. Après négociation, l'entreprise retenue est Dupont Frères",
            "Niet gegund in eerste ronde, na heraanbesteding gegund op 12-03-2014 aan Bouwbedrijf Jansen",
            "Declarado desierto en primera licitación y adjudicado el 12/03/2014 a Construcciones Pérez",
            "Declarado desierto en primera licitación y adjudicado posteriormente por procedimiento negociado a Construcciones Pérez",
            "Nessuna aggiudicazione nella prima seduta, aggiudicato in data 12/03/2014 alla ditta Rossi Costruzioni",
            "Lot déclaré infructueux puis relancé en procédure négociée : la société Storz a été retenue",
            "La société Storz a été retenue. Le lot 32 a été déclaré infructueux",
            "Infructueux en première phase. Relancé en procédure négociée : Keolis retenu",
            "Declarado desierto el lote 2. Resultó adjudicataria la empresa Construcciones Pérez",
            "Nessuna aggiudicazione per il lotto 2, aggiudicataria la ditta Rossi Costruzioni",
            "Marché infructueux, relancé en négocié: groupement Gidef (mandataire solidaire) / Auxet",
            // The second review: a noun's label takes any complement with no negation, copula or
            // awardee word; a copula then the party, after an article or a company word, names it.
            "Lot infructueux, relancé en procédure négociée. Titulaire du marché négocié : Dupont Frères",
            "Lot infructueux. Titulaire du marché après relance : Dupont Frères",
            "Lot infructueux. Titulaire après mise en concurrence : Dupont Frères",
            "Lot infructueux. Attributaire du marché passé en procédure négociée : Dupont Frères",
            "Lot infructueux, attributaire unique : Dupont Frères",
            "Première procédure infructueuse. Attributaire désigné : Dupont Frères",
            "Lot infructueux en premier tour. Après négociation, l'entreprise retenue est la société Dupont",
            "Lot infructueux en première consultation. Après négociation, le candidat retenu est l'entreprise Dupont Frères",
            // The third review: a party that opens on a lot designator word, and one cut after its article.
            "Lot infructueux en premier tour. Après négociation, le candidat retenu est Zone Ouest Transports",
            "Lot infructueux en premier tour. Après négociation, le candidat retenu est Secteur Nord BTP",
            "Lot infructueux en première consultation. Après négociation, le titulaire est Dupont Frères",
            "Lot infructueux en première consultation. Après négociation, l'attributaire est l'entreprise Dupont Frères",
        ];
        assert_class(&review.map(|name| (0, name)), Some(NotAName::AwardSummary));
        for name in review {
            assert!(awardee_labels(name), "{name}");
        }
        // A void prefix then the named party: a name, as 510 read it.
        assert_class(
            &[
                (0, "Desierto en primera licitación, adjudicado definitivamente a Construcciones Pérez"),
                (0, "Lotto deserto in prima gara, aggiudicato definitivamente alla ditta Rossi Costruzioni"),
                (0, "Not awarded initially, subsequently awarded on 12.3.2014 to Acme Services"),
            ],
            None,
        );
        // Still nobody: the reach stops at the clause, `awarded` takes `to` only, `retenu` after an
        // offer or a candidate, and a noun before a lowercase word.
        for name in [
            "Not Awarded - Other FE College awarded area. V.5) THE CONTRACT IS LIKELY TO BE SUB-CONTRACTED: No",
            "Not awarded (was retended and awarded as part of a new project)",
            "Lot infructueux : l'offre a été retenue puis retirée",
            "Lot 2 : sans suite. Le candidat retenu s'est désisté",
            "Pas d'attributaire retenu pour l'instant. Décision de la commission",
            "Attributaire non designe - lot infructueux",
        ] {
            assert!(!awardee_labels(name), "{name}");
        }
        // The second review: a negation still ends a noun's label, and a copula before a lowercase
        // word or a negation names nobody.
        let nobody = [
            "Lot infructueux : attributaire non désigné : voir avis ultérieur",
            "Lot infructueux, le titulaire est défaillant",
            "Lot infructueux : l'attributaire n'est pas désigné",
        ];
        assert_class(&nobody.map(|name| (0, name)), Some(NotAName::VoidLot));
        for name in nobody {
            assert!(!awardee_labels(name), "{name}");
        }
    }

    /// Issue 520 D4 (11 orgs / 12 mentions): a list of lots names an award only when one of its
    /// lots is not void — a glued number mark is its number, a relaunch's designators belong to the
    /// relaunch, and a list whose every entry is void is void. A cut list (an empty last entry), a
    /// lettered heading, and every text-era `Lot n)` list of class C keep their summary.
    #[test]
    fn a_list_of_void_lots_is_void() {
        assert_class(
            &[
                (2948392, "lot 9: Sans suite Lot 10: Infructueux"),
                (4726611, "Sublote 9.1: Declarado desierto. Sublote 9.2: Declarado desierto. Sublote 9.3: Declarado desierto."),
                (7815395, "Les lots n° 1 et 3 ont été déclarés infructueux. Le lot n° 2 a été déclaré sans suite"),
                (6333652, "Le présent lot n'a reçu aucune offre. Il est déclaré sans suite à l'article 59-III du code des marchés publics. Ce lot a été décomposé en 2 nouveaux lots: lot nº 06 — bâtiments démontables, lot nº 07 — travaux d'installation des bâtiments démontables, et relancé en appel d'offres ouvert le 17.7.2014"),
                (12169942, "Perceel 2 en Perceel 4 worden niet gegund. Er zijn geen geldige inschrijvingen ontvangen."),
                (3717094, "lot 1 sans suite Lot 3 infructueux"),
                (5425806, "Lots 2a et 2c: procédure infructueuse lot 2b: déclaration sans suite"),
                (5425808, "Lot 4a: procédure infructueuse lots 4b/4c: déclaration sans suite"),
                (30680275, "Marché numéro: 01072 - Lot 1) et lot 2) Marché classé sans suite"),
                (31428159, "Lot 1 sans suite / lot 2 infructueux"),
                // With D3: the relaunch's `en vue de l'attribution de 2 marchés`, the label `Date
                // d'attribution :`.
                (1537305, "Marché classé sans suite. Il a été décidé de lancer une nouvelle procédure en appel d'offres ouvert européen en vue de l'attribution de 2 marchés se décomposant de la manière suvante: lot no 3 - agence Paul Signac lot no 4 - agence Beaumonts."),
                (3556403, "Infructuosité. Le lot no 3 a été déclarée infructueux. Le présent avis ne concerne que le lot no3 -Sud-Ouest). Date d'attribution : CAO du 17.4.2013."),
                (0, "Les lots 2 et 11 sont déclarés sans suite. Le lot 10 est déclaré infructueux"),
            ],
            Some(NotAName::VoidLot),
        );
        assert_class(
            &[
                (30698459, "Lot 1) déclaré infructueux. Lots 2)"),
                (31731990, "G) non attribué. Marché conserves: A)"),
                (0, "Lot 1) Dupont. Lot 2) infructueux"),
                (2957546, "lot 26: Carestream Lot 27: Sans suite lot 28: CARESTREAM"),
                (26966868, "Lote I: declarado desierto. Lote II: Winterthur Seguros Generales"),
                (30687978, "Lote 1) Declarado desierto. Lote 2) Limpiezas Luz"),
                (30695615, "A) non attribué. B1) et B2) Climelec"),
                (31735007, "Lot 1) Blin. Lot 2) Prodirest. Lot 3) Ouest Distribution. Lot 4) Ouest Distribution. Lot 5) Corol Jeager. Lot 6) Déclaré infructueux. Lot 7) Prodirest. Lot 8) Déclaré infructueux. Lot 9) Déclaré infructueux. Lot 10) Blin. Lot 11) Prodirest. Lot 12) Déclare infructueux. Lot 13) Ame Haslé. Lot 14) Gef"),
                // 520's residual: one verdict governs a list whose entries carry none of their own.
                (31391828, "non retenus: lots 6-44-56-68-74-91-94-95 sans suite: lots 9-65-92-96"),
                (23532535, "Déclaration sans suite du lot n°15 et du lot n°17- Recomposition d’un lot -Relance de consultation du lot n°15 « Chape-Carrelage-Faïence »."),
            ],
            Some(NotAName::AwardSummary),
        );
        assert_class(&[(28310858, "Desierto lote 4 y 5 por falta de licitadores. Desierto lotes 1")], None);
        // The U2 review's near misses, each a summary at HEAD: an entry the designators miss is a
        // numbered entry (`- 44 reckitt …`, 3371523's own template cut to two lots); an entry with
        // nothing after its label keeps the summary wherever it stands (the winner named first, the
        // B1 lists); a winner made of digit words or initials is no part of a label; an entry that
        // names its winner before its own earlier void call is no void verdict; and a relaunch ends
        // at the summary's next entry.
        assert_class(
            &[
                (0, "lot 43 INFRUCTUEUX - 44 reckitt benckiser 28692.07"),
                (0, "lot268 infructueux - 269sanofi AV"),
                (0, "lot 1045 sans suite - 1046 serb 83268ht"),
                (0, "Lot 1 Keolis - 2 infructueux"),
                (0, "Lot 1 : Keolis / 2 : sans suite"),
                (0, "lot 43 Reckitt Benckiser 28692.07 - 44 INFRUCTUEUX"),
                (0, "Lots 1 et 2 infructueux - 3 Keolis"),
                (0, "Paredes lot 506 - lots 501 à 505 infructueux"),
                (0, "Gallego secteurs 3A à 3D / secteurs 3E à 3H infructueux"),
                (0, "Mongin Jauffret : lots 1504 et 1505. Lots 1501, 1502 et 1503 : infructueux"),
                (0, "Orthomed : lots nº 25 et 26 / lot nº 24 : infructueux"),
                (0, "Carestream : lots 26 et 28 - lot 27 : sans suite"),
                (0, "Lot 1 : 3M. Lot 2 : infructueux"),
                (0, "Lot 1 : A.C.H. Lot 2 : sans suite"),
                (0, "Lot 1 : infructueux. Lot 2 : Dupont (marché négocié, AO infructueux)"),
                (0, "Lot 1 : infructueux. Lot 2 : sans suite puis relancé, titulaire du marché : Dupont"),
                (0, "Lot 1 infructueux et relancé, lot 2 : Dupont"),
                // The second review: a winner written straight after its label, capitalised, also
                // ends the relaunch.
                (0, "Lot 1 infructueux et relancé, lot 2 Dupont Frères, lot 3 Martin"),
                (0, "Lots 1 et 2 infructueux et relancés, lot 3 Dupont Frères"),
            ],
            Some(NotAName::AwardSummary),
        );
        // Every entry void, its conjunction joined, its numbers in the label: void. And the arm only
        // quiets: a relaunch cut must not pair `Lot relancé.` with the next sentence's number, so a
        // name void at HEAD stays void.
        assert_class(
            &[
                (0, "Lot 1 : infructueux. Lot 2 : sans suite"),
                (0, "Lot 1 et Lot 2 : infructueux"),
                (0, "Lots 1 et 2 : infructueux. Lot 3 : sans suite"),
                (0, "Lote 1: declarado desierto. Lote 2: declarado desierto"),
                (0, "Lot 1 : déclaré infructueux (offre de Dupont SARL irrégulière). Lot 2 : sans suite"),
                (0, "Infructueux. Lot 3 : 2 offres reçues. Lot relancé. 4 offres"),
                (0, "Lot 1 infructueux et relancé, Lot 2 Sans suite"),
            ],
            Some(NotAName::VoidLot),
        );
    }

    /// Issue 520 D5 (12 orgs / 15 mentions): a `;` segment that gives the reason (an offer or a
    /// bidder), a blank (`néant`) or the relaunch's publication (`annonce`, `JOUE`, `BOAMP`) names
    /// nobody, and a `;` inside parentheses or an `&amp;` splits nothing.
    #[test]
    fn a_reason_segment_names_nobody() {
        let void = [
            (1513105, "Aucune offre n'a été déposée; le lot n° 2 a été déclaré infructueux"),
            (3534542, "La SEMAPA a déclaré sans suite l'appel d'offres relatif au marché de prestations de maquettistes ; le nombre d'offres conformes n'étant pas suffisant pour répondre aux besoins de la consultation"),
            (1814340, "Zadanie unieważnione - Wykonawcy wykluczeni; oferty odrzucone"),
            (11663720, "Desierto; el licitador ha renunciado a la adjudicación del contrato"),
            (2678824, "Déclaré sans suite pour motif d'intérêt général et relancé (cf avis JOUE no 2012/s 133-221467 du 13.7.2012; et Annonce no 484, BOAMP no 134b, du 13.7.2012)"),
            (1710867, "néant; la consultation a été déclarée infructueuse en raison de l'absence d'offre recevable"),
            (5148433, "Offres inacceptables ; lot déclaré infructueux"),
            (6357683, "Lot déclaré sans-suite et relancé en décembre 2014, sous les références suivantes: JOUE: n° 2014/S 250-444751 du 30.12.2014 — BOAMP: B n° 250 du 30.12.2014; annonce(s) n° 116"),
            (7075593, "postępowanie unieważniono – ofertę odrzucono; Wykonawca wykluczony – nie spełnia warunków udziału w postępowaniu"),
            (11663718, "Desierto; oferta excluida del proceso de valoración"),
            (2369100, "nie wpłynęła żadna oferta; postepowanie unieważniono art. 93 ust. 1 pkt 1 Pzp"),
            (5961568, "déclaré sans suite pour un motif d'intérêt général (redéfinition du besoin) - relance en 2 lots distincts (publicité de la relance: JOUE 2014/S 161-289268 du 23.8.2014 / BOAMP parution nº 20140161b; annonce nº 63 du 23.8.2014 / profil acheteur achatpublic.com et site de la ville: 21.8.2014)"),
        ];
        assert_class(&void, Some(NotAName::VoidLot));
        for (org, name) in void {
            assert!(!super::award_mixed_segments(name), "{org}: {name}");
        }
        let named = [
            (4836476, "Bp innovis ; infructueux"),
            (1722465, "2012023401 = Cle de Sol; 2012023402 = Infructueux; 2012043401 = Logis Services; 2012053401 = ATH; 2012053402 = MCB; 2012053403 = Vittet; 2012093401 = Laffin"),
            (0, "2013023201/zone Belley-Bas Bugey : Sarl Mcb - 01300 Chazey Bons ; 2013073202/zone Bresse : infructueux"),
        ];
        assert_class(&named, Some(NotAName::AwardSummary));
        for (org, name) in named {
            assert!(super::award_mixed_segments(name), "{org}: {name}");
        }
        // The U2 review: an offer or a bidder word alone is no reason — Polish names the winner as
        // the chosen offer or under `Wykonawca`, French as the offer kept or the sole bidder.
        let review = [
            "Część 1 unieważniona; część 2 – wybrano ofertę firmy Usługi Transportowe Jan Kowalski",
            "Zadanie nr 1 – postępowanie unieważnione; zadanie nr 2 – najkorzystniejsza oferta: PHU Kowalski",
            "Część nr 1 – unieważniono; część nr 2 – Wykonawca: Jan Kowalski Usługi Remontowe",
            "Zadanie 1 – postępowanie unieważnione; Zadanie 2 – Wykonawca Jan Kowalski, Kraków",
            "Część 1 unieważniona; część 2: wybrany wykonawca Jan Kowalski",
            "Lot infructueux en première consultation ; offre de la société Dupont retenue après négociation",
            "Infructueux ; marché négocié conclu avec le seul soumissionnaire, Dupont Frères",
            "Lot 2 infructueux ; l'offre de la société Keolis a été retenue",
            "Lot 2 infructueux ; offre de Keolis retenue",
            // The second review: a negation far from the offer word is no offer's failure.
            "Infructueux au premier tour ; au second tour, seule offre reçue : Dupont Frères, retenue sans négociation",
        ];
        assert_class(&review.map(|name| (0, name)), Some(NotAName::AwardSummary));
        for name in review {
            assert!(super::award_mixed_segments(name), "{name}");
        }
        assert_eq!(not_a_name("Desierto el lote 2; lote único: oferta de Construcciones Pérez"), None);
    }

    /// Issue 520 D6 (9 orgs): a company form names a bidder, not a winner, after an offer word in a
    /// sentence that rejects the offer, after `relancer` / `négocier` / `sera` in its sentence, or in
    /// `Société X Lot déclaré infructueux …` with no lot number and no award cue. A form before a
    /// lot's verdict, a form in a new sentence, and `DESIERTO, S.L.` keep their party.
    #[test]
    fn a_bidder_is_no_winner() {
        let void = [
            (31527955, "Société Sindarro Lot déclaré infructueux relancé en marché négocié avec publicité et mise enconcurence (date limite le 10 juin 2008)"),
            (1974244, "la commission d'appel d'offres décide d'interrompre les discussions et déclare la consultation infructueuse car l'offre du groupement auxi+/ sas Viviani est inacceptable"),
            (11251840, "Lotto non aggiudicato in quanto la commissione ha escluso dalla gara l’unica offerta ricevuta dalla compagnia UnipolSai S.p.A. — Ag. Marcantoni — in quanto le varianti proposte sono state ritenute in grado di comportare una sostanziale modifica della copertura assicurativa richiesta tali da evidenzi"),
            (1506401, "Aucune offre n'ayant été déposée, un marché négocié sans publicité et sans mise en concurrence, sera passé avec la STÉ France Télécom"),
            (31527961, "Société SOTEP Lot Déclaré infructueux et sera relancé en marché negocié avec publicité et mise en concurence (date limite le 10 juin 2008)"),
            (31527956, "Société Sanchez lot déclaré infructueux relancé en marché négocié avec publicité et mise en concurence (date limite le 10 juin 2008)"),
            (31527960, "Société Energys. Lot déclaré infructueux relancé en marché négocié avec publicité et mise en concurence (date limite le 10 juin 2008)"),
            (31527964, "Société Spada. Lot déclarée infructueux relancé en procédure négocié avec publicité et mise en concurence (date limite le 10 juin 2008)"),
            (31527968, "Société Herman Aluminium Lot déclaré infructueux, relancé en marché négocié avec publicité avec mise en concurence (date limite le 10 juin 2008)"),
            // With D2c: `à relancer … avec les sociétés ayant soumissionné … : lot 5 : Société Massias`.
            (3526210, "Déclaré infructueux - à relancer en marché négocié, en application de l'article 35 - i 1er alinéa du code des marchés publics, avec les sociétés ayant soumissionné à l'appel d'offre : lot 5 : Société Massias"),
        ];
        assert_class(&void, Some(NotAName::VoidLot));
        for (org, name) in void {
            assert!(!super::award_company_token(name), "{org}: {name}");
        }
        let kept = [
            (0, "Yvelin SA. Lot 2 : infructueux"),
            (6603163, "Stryker France SAS: lot nº 36 sans suite: lot nº 37"),
            (1844226, "BGE Guyane 16 rue Lt Becker 97300 Cayenne / La consultation a été déclarée sans suite pour les parts non couvertes des besoins"),
            (0, "Lot infructueux, sera relancé. Lot 2: Dupont SARL"),
        ];
        assert_class(&kept, Some(NotAName::AwardSummary));
        assert!(super::award_company_token("Lot infructueux, sera relancé. Lot 2: Dupont SARL"), "a new sentence");
        assert!(super::award_company_token("Société Dupont Lot déclaré infructueux, lot 2 : Martin"), "a lot number: a list");
        assert_eq!(not_a_name("DESIERTO, S.L."), None);
        // The U2 review: the rejection must be the form's own offer's (not the other offers', not
        // in another clause); an intent governs nothing past an award made or a new lot, nor across
        // a glued sentence end, a line break or an ellipsis; and a failed bidder's line names no
        // award after its verdict.
        let review = [
            "Lot infructueux en première consultation. Relancé : l'offre de Dupont SARL a été retenue, les autres offres étant irrégulières.",
            "Infructueux (1 offre irrégulière), relancé en procédure négociée et conclu avec Promocash SAS",
            "Infructueux, 1 offre irrégulière, relancé en procédure négociée et conclu avec Promocash SAS",
            "Infructueux. Décision de négocier : marché conclu avec Dupont SARL",
            "Société Dupont Lot déclaré infructueux puis négocié : marché notifié à Société Dupont le 3.6.2008",
            "Lot infructueux, sera relancé.Lot 2: Dupont SARL",
            "Lot infructueux, sera relancé\nLot 2: Dupont SARL",
            "Lot infructueux, sera relancé - Lot 2: Dupont SARL",
            "Lot infructueux, sera relancé… Lot 2: Dupont SARL",
            "Lot 1 infructueux et sera relancé, lot 2 : Dupont SARL",
            // The second review: a comma after the rejection closes the rejected offer, and `passé`,
            // `signé`, `confié` with no future before them are awards made.
            "Infructueux : offres irrégulières, marché négocié avec Dupont SARL",
            "Appel d'offres infructueux, offres irrégulières écartées, marché négocié avec Dupont SARL",
            "Appel d'offres infructueux, l'offre de Martin SA étant irrégulière, marché négocié avec Dupont SARL",
            "Lot infructueux, l'offre de Martin SA ayant été jugée irrégulière, marché confié à Dupont SARL après négociation",
            "Infructueux. Décision de négocier : marché passé avec Dupont SARL",
            "Lot infructueux, décision de négocier, marché signé avec Dupont SARL",
            "Infructueux. Décision de négocier : marché confié à Dupont SARL",
        ];
        assert_class(&review.map(|name| (0, name)), Some(NotAName::AwardSummary));
        for name in review {
            assert!(super::award_company_token(name), "{name}");
        }
        assert_eq!(not_a_name("Lotto deserto: offerta della ditta Bianchi esclusa, aggiudicato poi alla Rossi S.r.l."), None);
        // The rejected offer's owner between commas stays a bidder: the comma before the rejection
        // does not close its span (the second review keeps that while the one after it does).
        for bidder in [
            "L'offre de la société Dupont SA, jugée irrégulière, a été écartée. Lot infructueux",
            "Lot infructueux : l'offre de Martin SA, irrégulière, a été écartée",
        ] {
            assert_eq!(not_a_name(bidder), Some(NotAName::VoidLot), "{bidder}");
            assert!(!super::award_company_token(bidder), "{bidder}");
        }
    }

    /// Issue 520 D7 (14 orgs): the head before a spilled `SECTION VI` names a party only when it
    /// says nothing void — a whole value, a reason head or a prefix, once a trailing `see` is off.
    /// A company's name there keeps it, a company named like the phrase included.
    #[test]
    fn a_void_head_before_a_spilled_section_is_void() {
        let spilled = |name: &str| super::named_before_spill(name, &fold(name));
        let void = [
            (25755356, "Desierto. SECTION VI: OTHER INFORMATION VI.3) Date of contract award: 2.1.2004. VI.4) Number of tenders received: 12. VI.5) This contract was subject to a notice in the OJ: Yes. Of: 21.10.2003. VI.7) Other information: I"),
            (29760975, "No contract awarded. SECTION VI: COMPLEMENTARY INFORMATION VI.2) ADDITIONAL INFORMATION: (ET Ref:42985). VI.3) PROCEDURES FOR APPEAL: VI.3.1) Body responsible for appeal procedures: Limerick City Council"),
            (30537999, "No award made. SECTION VI: OTHER INFORMATION VI.4) Number of tenders received:"),
            (14580326, "No Contract awarded, see section VI.2"),
            (31854611, "Not awarded. Procedure cancelled according to FOA § 22-1.. SECTION VI: COMPLEMENTARY INFORMATION VI.2) ADDITIONAL INFORMATION: Criterias for area"),
            (25043000, "Gara deserta per mancanza di offerte valide. SECTION VI: OTHER INFORMATION VI.3) Date of contract award: 8.4.2005. VI.4) Number of tenders received: 3. VI.7) Other information:"),
            (30899597, "No contract awarded — see section VI.2)"),
            (30336777, "Not awarded. Section VI: Complementary information VI.1) Contract related to a project and/or programme financed by EU funds: No. VI.2) Additional information:"),
            (30786740, "Desiertos.. SECTION VI: OTHER INFORMATION VI.1) Non-mandatory notice: No. VI.2) Reference number attributed to the file by the contracting authority: 15-3-2.1-0027/2003. VI.3) Date of contract award:"),
        ];
        assert_class(&void, Some(NotAName::VoidLot));
        for (org, name) in void {
            assert!(!spilled(name), "{org}: {name}");
        }
        let named = [
            (0, "Zundel Et Kohler. SECTION VI: OTHER INFORMATION VI.7) Other information: Lots Infructueux (sans relance en marché négocié): 001"),
            (0, "Getinge France. SECTION VI: OTHER INFORMATION VI.7) Other information: Tous les autres lots (sur 123 lots au total) sont infructueux."),
        ];
        assert_class(&named, Some(NotAName::AwardSummary));
        for (org, name) in named {
            assert!(spilled(name), "{org}: {name}");
        }
        assert!(spilled("Desierto Florido SL. SECTION VI: OTHER INFORMATION VI.7) Other information: Lote 2 declarado desierto"), "a company named like the phrase");
        // The U2 review: a winner after the head's last `:` names the party; a reason there does not.
        let winner = "Not awarded in first round, re-tendered: Acme Construction. SECTION VI: OTHER INFORMATION";
        assert!(spilled(winner));
        assert_eq!(not_a_name(winner), None);
        for name in [
            "Not awarded: no bids received. SECTION VI: OTHER INFORMATION",
            "Desierto: Ninguna oferta. SECTION VI: OTHER INFORMATION",
            // The second review: a capitalised word after the head's `:` is no winner where the head
            // holds a void stem — 510 read each of these void, and D7 only quiets.
            "Infructueux : Voir section VI.3 Informations complémentaires",
            "Infructueux : LOT 3. SECTION VI : RENSEIGNEMENTS COMPLÉMENTAIRES",
            "Lot 3 déclaré sans suite : Se reporter à la section VI.3",
            "Lot déclaré infructueux : Relance en procédure négociée. SECTION VI: RENSEIGNEMENTS COMPLÉMENTAIRES",
            "Déclaré sans suite : Motif d'intérêt général. SECTION VI: RENSEIGNEMENTS COMPLÉMENTAIRES",
        ] {
            assert!(!spilled(name), "{name}");
            assert_eq!(not_a_name(name), Some(NotAName::VoidLot), "{name}");
        }
    }

    /// Issue 520 (§7): the narrowings live in `names_an_award`'s own arms only, so the shape helpers
    /// issue 511 reads give the answers they gave — on 511's specimens, and on 520's void names, which
    /// keep their summary shape for a placeholder or blank stem's keep rule though they no longer name
    /// an award.
    #[test]
    fn the_shared_shape_helpers_are_unchanged() {
        use super::{names_a_legal_form, names_something, summary_shape};
        let shaped = |name: &str| summary_shape(name, &fold(name));
        for name in [
            "Lot 2: not applicable. Lot 1: ACME Ltd",
            "Nie dotyczy (część 2); część 1: X sp. z o.o.",
            "Lot 1: ACME Ltd, Lot 2: Beta Ltd",
            "lot 3 : Société Martin SAS",
            // 520's D names: the shape is there; only the award reading changed.
            "Le lot 102 : lot B10: Menuiseries bois PVC serrurerie vitrerie (Genevois) a été déclaré infructueux",
            "Société Sindarro Lot déclaré infructueux relancé en marché négocié avec publicité",
            "lot 9: Sans suite Lot 10: Infructueux",
            "Sublote 9.1: Declarado desierto. Sublote 9.2: Declarado desierto.",
            "la commission d'appel d'offres décide d'interrompre les discussions et déclare la consultation infructueuse car l'offre du groupement auxi+/ sas Viviani est inacceptable",
        ] {
            assert!(shaped(name), "{name}");
        }
        for name in [
            "See section VI.3 for the list of awarded suppliers",
            "Not applicable, see section VI.4",
            "nie dotyczy",
            "Procédure incomplète. Le marché n'a pas été attribué. Appel d'offres sans suite",
            "Sans suite, en cours d'attribution",
        ] {
            assert!(!shaped(name), "{name}");
        }
        for name in ["Firma AS", "Hansen A/S", "Norconsult AS", "Société Sindarro Lot déclaré infructueux", "UnipolSai S.p.A."] {
            assert!(names_a_legal_form(name), "{name}");
        }
        for name in ["As in I.1 above", "AS VI.4.1", "Sans suite, en cours d'attribution"] {
            assert!(!names_a_legal_form(name), "{name}");
        }
        for name in [": High Court of Justice", " NEB Betriebsgesellschaft mbH", "Société Sindarro", " Stryker France SAS"] {
            assert!(names_something(name), "{name}");
        }
        for name in ["", " VI3.2", " siehe oben", " 4.1)"] {
            assert!(!names_something(name), "{name}");
        }
        // `named_before_stem`, through the boundary stems it reads.
        assert_eq!(not_a_name("Ayuntamiento de Madrid (véase perfil del contratante)"), Some(NotAName::AwardSummary));
        assert_eq!(not_a_name("Serco — disclosure would prejudice commercial interests"), Some(NotAName::AwardSummary));
        assert_eq!(not_a_name("Véase perfil del contratante"), Some(NotAName::Placeholder));
    }

    /// Issue 520 unit 1's D class (§4.1, Appendix A): every org the decision moves to `VoidLot` — 173
    /// orgs, D1 to D7 and the three that need two rules — now reads void. The names are the dry
    /// report's (`void-names-refold-after-dry.json`, cut at 300 characters); the three cut D1 rows a
    /// challenger read in full carry the tail they quote (`unit1-classified.json`), joined with `…`.
    /// Since U4 none of them names a winner either ([`super::award_winner`]).
    #[test]
    fn every_void_520_exemption_is_a_void_lot() {
        assert_class_and_no_winner(
            &[
                // D1: one lot's BOAMP void sentence.
                (1392146, "Le lot 1: secteur du canton d'Evian - Rd1005; Rd24 et Rd32 a été déclaré Infructueux"),
                (1513898, "Le lot 3: Diagnostics sur bâtiments occupés à usage principal d'habitation Le présent lot a pour objet la réalisation par le titulaire de diagnostics sur bâtiments occupés à usage principal d'habitation à réaliser au fur et à mesure des besoins de Paris habitat-OPH. a été déclaré sans suite"),
                (1537987, "le lot 1: Rédaction d'un feuillet du magazine qui sera constitué au maximum de: - 2 doubles pages de portraits d'entreprises, associations, clubs de sports .... - 1 double page patrimoine, faune ou flore du département a été déclaré Infructueux"),
                (1943868, "Le lot 1: génie civil marché de travaux (lot pricipal) décomposé en une tranche ferme et 3 tranches conditionnelles: TF: parc Paul Vernet; TC1: parc Clos de la Tour; TC2: parc Porte d'Orée; TC3: parc Agricola. A été déclaré sans suite"),
                (1943869, "Le lot 2: fourniture et pose de matériel d'exploitation de stationnement. Marché de fournitures décomposé en une tranche ferme et 4 tranches conditionnelles: TF: parc Paul Vernet; TC1: parc Clos de la Tour; TC2: parc Porte d'Orée; TC3: parc Agricola; TC4: parc Aubenas. A été déclaré sans suite"),
                (2021395, "le lot 12: sans objet se reporter au lot no9 plomberie sanitaire a été déclaré infructueux"),
                (2378132, "Le lot 3: portage de courses et de personnes sur le Val de Marne Le présent lot porte sur la réalisation de missions d'accompagnement et d'aide à la personne relatives au portage de courses et de personnes sur Paris et petite couronne à réaliser au fur et à mesure des besoins. Les prestations donner … trois (3) titulaires seront désignés … a été déclaré infructueux"),
                (2528483, "Le lot 1: Lot 1-2012-225-01 Syndicat intercommunal de la Vallée d'abondance a été déclaré Sans suite"),
                (2847767, "le lot 3: Lot 03 A: Ite parement pierre et lot et Lot 03 B: Ite enduits a été déclaré infructueux"),
                (2900386, "Le Lot 2: acheminement du trafic sortant sur les accès du lot 1 et accessible en présélection du transporteur. A été déclaré infructueux"),
                (2900516, "Le lot 2: lot no2 a été déclaré infructueux"),
                (3066482, "le lot 3 : Lot no3 : Films Radiographiques Specifiques Aux Besoins De La Radiotherapie a été déclaré Sans Suite"),
                (3107850, "le lot 8 : Electricite, Courant Fort - courant Faible ; a été déclaré Infructueux"),
                (3238976, "le lot 2 : Lot no2 - sorties scolaires, périscolaires, extrascolaires et autres a été déclaré Infructueux"),
                (3425880, "le lot 47 : Prothèse de tête radiale pivotante ; mise à disposition à la demande a été déclaré Infructueux"),
                (3666893, "Le lot 102 : lot B10: Menuiseries bois PVC serrurerie vitrerie (Genevois) a été déclaré infructueux"),
                (3666894, "Le lot 103 : lot C10: Menuiseries bois PVC serrurerie vitrerie (Chablais) a été déclaré infructueux"),
                (3666895, "Le lot 104 : lot D10: Menuiseries bois PVS serrurerie vitrerie (Vallée de l'arve) a été déclaré infructueux"),
                (3666903, "Le lot 92 : lot B9: Stores volets occultation (Genevois) a été déclaré infructueux"),
                (3666904, "Le lot 93 : lot C9: Stores volets occultation (Chablais) a été déclaré infructueux"),
                (3666905, "Le lot 94 : lot D9: Stores volets occultation (Vallée de l'arve) a été déclaré infructueux"),
                (3666906, "Le lot 101 : lot A10: Menuiseries bois PVC serrurerie vitrerie (Annecy) a été déclaré infructueux"),
                (3915725, "le lot 9 : Lot 9b : Ossature bois, Charpente - capitainerie a été déclaré sans suite"),
                (3915726, "le lot 10 : Lot 9c : Couverture, Bardage, Etanchéité - capitainerie a été déclaré sans suite"),
                (3915727, "le lot 11 : Lot 9d : Châssis, Murs Rideaux - capitainerie a été déclaré sans suite"),
                (3915728, "le lot 12 : Lot 9e : Metallerie, Portes Métalliques - capitainerie a été déclaré sans suite"),
                (3915729, "le lot 13 : Lot 9f : Menuiseries intérieures, Plâtrerie - capitainerie a été déclaré sans suite"),
                (3915730, "le lot 14 : Lot 9g : Electricité Chauffage - capitainerie a été déclaré sans suite"),
                (3915731, "le lot 15 : Lot 9h : Ventilation - capitainerie a été déclaré sans suite"),
                (3915732, "le lot 16 : Lot 9i : Plomberie, Sanitaire - capitainerie a été déclaré sans suite"),
                (3915733, "le lot 17 : Lot 9j : Peinture, revêtement de sols - capitainerie a été déclaré sans suite"),
                (3915742, "le lot 8 : Lot 9a : Gros-Oeuvre - capitainerie a été déclaré sans suite"),
                (3984070, "Le lot 4 : lot 7 : cloisons doublage et faux plafonds a été déclaré sans suite"),
                (3984072, "Le lot 6 : lot 10 : équipements sportifs a été déclaré sans suite"),
                (3984073, "Le lot 7 : lot 11 : peintures interieures a été déclaré sans suite"),
                (4133918, "Le lot 91 : lot 9 A: Fourniture de pièces détachées pour l'entretien d'engins et matériel de marque JCB (type chargeur ou centrale hydraulique) a été déclaré infructueux"),
                (4133919, "Le lot 92 : Lot 9 B : Prestations de maintenance pour l'entretien d'engins et matériel de marque JCB (type chargeur ou centrale hydraulique) a été déclaré infructueux"),
                (4133921, "Le lot 11 : Fourniture de pièces détachées et prestations de maintenance pour l'entretien d'engins de TP de marque O&Amp;K (type Niveleuse) a été déclaré infructueux"),
                (4133935, "Le lot 21 : lot 2 A : Fourniture de pièces détachées pour l'entretien d'engins de TP de marque Dynapac (type Compacteur) a été déclaré infructueux"),
                (4133936, "Le lot 22 : lot 2 B : Prestations de maintenance pour l'entretien d'engins de TP de marque Dynapac (type Compacteur) a été déclaré infructueux"),
                (4272807, "le lot 2 : 1 carrosserie de Bibliobus prévue pour aller sur un porteur nu caractéristiques du lot no1 (Aoo) a été déclaré sans suite"),
                (4425125, "Le lot 115 : lot B11 C11 Sols souples Genevois et Chablais a été déclaré infructueux"),
                (4425126, "Le lot 114 : lot D11 Sols souples Arve a été déclaré infructueux"),
                (4425128, "Le lot 55 : lot B5 C5 Plomberie chauffage climatisation vmc genevois chablais a été déclaré infructueux"),
                (4425129, "Le lot 54 : lot D5 Plomberie chauffage climatisation vmc arve a été déclaré infructueux"),
                (4425130, "Le lot 61 : lot A6 Couverture charpente zinguerie Annecy a été déclaré infructueux"),
                (4425131, "Le lot 65 : lot B6 C6 Couverture chapente zinguerie genevois chablais a été déclaré infructueux"),
                (4425132, "Le lot 64 : lot D6 Couverture charpente zinguerie Arve a été déclaré infructueux"),
                (4764054, "le lot 1 : Lot no1 : réparation des volets roulants pour les RPA Balaÿ et Bel Horizon a été déclaré infructueux"),
                (4846679, "le lot 115 : lot B11 C11 Sols souples genevois chablais a été déclaré INFRUCTUEUX"),
                (5101244, "Le lot 6: livres scolaires et documents pédagogiques d'accompagnement (SAU ouvrages distribués exclusivement par les éditeurs eux-mêmes et livres de fonction et documentaires pour les bibliothèques des écoles) a été déclaré sans suite"),
                (5605092, "Le lot 6 : lot M6 Palneca - ajaccio a été déclaré infructueux"),
                (5772779, "le lot 002 : B Gros oeuvre étendu : Fondations spéciales (sous-lot nob02) - gros oeuvre (sous-lot nob03) - charpente métallique (sous-lot nob04) - etanchéité (sous-lot nob05) - façade brique (sous lot nob06) a été déclaré sans suite"),
                (5772782, "le lot 005 : E Finitions : Menuiseries bois courantes (portes bois) (sous-lot noe09) - cloisons, Plafonds, Peinture (sous-lot noe10) - carrelage, Faïence, Sols souples (sous-lot noe11), a été déclaré sans suite"),
                (5772784, "le lot 007 : G lots techniques : appareils élévateurs (sous-lot nog12) - cVC, Désenfumage, Plomberie Sanitaire (sous-lot nog13) - electricité courants forts et courants faibles,vdi,ssi (sous-lot nog14) a été déclaré sans suite"),
                (31427032, "Le lot 3: entretien des espaces verts aux aobrds de bâtiments et équipements entretien des ensemencements (tonte); entretien des espaces arborés, massifs et plantations et maintenance des barrières et clôtures. a été déclaré sans suite"),
                (31450417, "Le Lot 2: Gros Oeuvre - Clos et Couvert - Amenagements Interieurs. Le présent lot concerne tous les corps d'État techniques de bâtiment et intègre les spécialités suivantes: protection; gros oeuvre; charpente bois; charpente métallique; couverture ardoises et zinc; menuiseries extérieures bois; serr … ascenseurs a été déclaré infructueux"),
                (31471384, "Le lot 6: service d'assurance pour les travaux de construction neuve de logements locatifs sociaux \"Zac Ballastière Sud lots 3-2, 3-3 - 94450 Limeil-Brévannes\" a été déclaré infructueux"),
                (31473527, "Le lot 1: Ressources humaines et sciences sociales. Emploi et droits sociaux, inclusion; Politique régionale et développement local,cohésion du territoire. Agriculture, pêche et qualité alimentaire; politique environnementale, innovation, entreprise. Culture, et a été déclaré Sans Suite"),
                (31481076, "Le lot 5: Lot 2011-210 02 Communauté de Communes de la Rive-Gauche-du-Lac a été déclaré infructueux"),
                (31481089, "Le lot 9: Lot 2011-216 01 Communauté de Communes du Pays de Cruseilles a été déclaré Infructueux"),
                (31481092, "Le lot 6: Lot 2011-231 01 Communauté de Communes du Genevois a été déclaré Infructueux"),
                (31481093, "Le lot 7: Lot 2011-231 02 Communauté de Communes du Genevois a été déclaré Infructueux"),
                (31481094, "Le lot 8: Lot 2011-231 03 Communauté de Communes du Genevois a été déclaré Infructueux"),
                (31481098, "Le lot 3: lot 2011-230 01 Communauté de communes de la Vallée Verte a été déclaré infructueux"),
                (31481123, "Le lot 7: Lot 2011-231 02 Communauté de communes du Genevois a été déclaré sans suite"),
                (31491050, "Le lot 13: Agencement - TF: Etudes d'exécution; - TC1: Fauteuils pliants avec tablettes (300 unités env) - TC2: Fauteuils pliants (300 unités env. a été déclaré Sans Suite"),
                (31491056, "Le lot 6: Electricité courants forts et courants faibles 6.01 Courants forts - Poste de transformation (1000 kva); - Onduleur (220 kva); - Alimentation ASI (100 kva); - Tgbt et distribution principale (sur 11000 mètres carrés Shon); - Distribution secondaire (30 tableaux divisionnaires env.); - Ecla … a été déclaré Sans Suite"),
                // D2a: `n'a pas été attribué`.
                (1171947, "Procédure incomplète. Le marché n'a pas été attribué. Appel d'offres sans suite"),
                (2131011, "Le marché n'a pas été attribué: la procédure est déclarée infructueuse le 17.4.2012"),
                (3314070, "le lot n'a pas été attribué, la procédure a été déclarée infructueuse"),
                (4229970, "ce lot no 2 de la procédure n'a pas été attribué et a été déclaré infructueux"),
                (6069583, "Le lot n° 2 du marché n'a pas été attribué. Le lot n° 2 est infructueux"),
                (7093933, "Le lot n° 2 n'a pas été attribué et fut déclaré sans suite"),
                (7524029, "Le lot n° 1 n'a pas été attribué. Au regard de l'article 59-IV, ce lot est déclaré sans suite pour le motif d'intérêt général suivant: compte-tenu du contexte de fusion entre BBA et les communautés de communes avoisinantes, les besoins devront être redéfinis par rapport à ce qui avait été initialeme"),
                (31400142, "Le Lot n'a pas été attribué (lot infructueux)"),
                // D2b: `déjà attribué`.
                (6011832, "le lot 01: Terrassement substitution de sol: Lot déjà attribué a été déclaré sans suite"),
                (31674188, "A 97.23.18) sans suite (déjà attribué au lots A 06 et A 12)"),
                (31674189, "A 97.23.19) sans suite (déjà attribué au lot A 12)"),
                // D2c: `avec la société` after a relaunch to come.
                (3526206, "Déclaré infructueux - à relancer en marché négocié, en application de l'article 35 - i - 1er alinéa du code des marchés publics, avec les sociétés ayant soumissionné à l'appel d'offre : lot 1 : société TIB"),
                (3526207, "Déclaré infructueux - à relancer en marché négocié, en application de l'article 35 - iI - 3ème alinéa du code des marchés publics, avec les sociétés ayant soumissionné à l'appel d'offre : lot 2 : société picot petit"),
                (3526208, "Déclaré infructueux - à relancer en marché négocié, en application de l'article 35 I 1er du code des marchés publics, avec les sociétés ayant soumissionné à l'appel d'offre : lot 3 : société Sides"),
                (3526209, "Déclaré infructueux - à relancer en marché négocié, en application de l'article 35 - iI 3 ème alinéa du code des marchés publics, avec les sociétés ayant soumissionné à l'appel d'offre : lot 4 : société WAS"),
                (3526991, "décalré infructueux= le lot 2 infructueux et de le relancer en marché négocié en application de l'article 35 - iI - 3ème alinéa du code des marchés publics, avec la société LHENRY"),
                (4032000, "Offres déclarées irrégulières. Lot déclaré donc infructueux. Il a donc été proposé de négocier avec les sociétés candidates dans le cadre de la procédure prévue à l'article 35-I-1 3ème alinéa du code des marchés publics par une lettre de consultation dont le formalisme est prévu à l'article 66 du co"),
                // D3: an awardee word that labels nobody.
                (1300853, "Marché déclaré sans suite à relancer en MAPA compte tenu de son montant (l'accord cadre était multi attributaires hors une seule offres était conforme)"),
                (1610364, "Lot 4 est déclaré sans suite pour motif d'intérêt général. En effet, après évaluation des besoins à satisfaire, l'Université de Nice Sophia Antipolis est dans l'incapacité de prendre en charge budgétaire l'offre retenue"),
                (1633917, "résultat du concours: déclaration sans suite pour motif de défaillance du mandataire de l'équipe, suite à la liquidation judiciaire de l'agence Toth-Fasquelle préalablement à la signature du marché de maîtrise d'oeuvre"),
                (1753646, "lot declare sans suite pour motif D'Interet general une seule offre ayant ete Reçue alors que la rmn-gp souhaitait conclure un accord-cadre multi-attributaire"),
                (1861834, "Lot classé \"sans suite\" et ce pour motif d'intérêt général (art. 59 du CMP). En effet, seul 1 candidat a déposé une offre conforme mais 1 attributaire ne convient pas pour le bon fonctionnement de l'établissement"),
                (1861839, "Lot classé \"sans suite\" et ce pour motif d'intérêt général et ce conformément à l'article 59 du code des marchés publics. En effet, seul 1 candidat a déposé une offre conforme mais 1 attributaire ne convient pas pour le bon fonctionnement de l'établissement"),
                (3720532, "le lot no2 a été déclaré infructueux, les offres reçues ont été déclarées irrégulières car ne respectant pas la rubrique Iv.2.1.2 de l'avis d'appel public à la concurrence (annexes financière dûment complétées). En vertu de l'article 35.i.1 du code des marchés publics, un marché négocié sera relancé"),
                (4774593, "décision de déclarer infructueux le lot nº 10 - plomberie sanitaire pour le motif suivant : toutes les offres réceptionnées sur ce lot ont été déclarées soit irrégulières soit inacceptables et ne permettent pas d'en établir l'attribution, de plus les crédits budgétaires alloués au marché après évalu"),
                (4774594, "décision de déclarer infructueux le Lot nº 11 - chauffage - ventilation - climatisation pour le motif suivant : toutes les offres réceptionnées sur ce lot ont été déclarées inacceptables et ne permettent pas d'en établir l'attribution car les crédits budgétaires alloués au marché après évaluation du"),
                (4774596, "décision de déclarer infructueux le lot nº 13 - toiles tendues pour le motif suivant : la seule offre réceptionnée sur ce lot a été déclarée inacceptable et ne permet pas d'en établir l'attribution car les crédits budgétaires alloués au marché après évaluation du besoin à satisfaire ne permettent pa"),
                (4774599, "décision de déclarer infructueux le lot nº 16 - plomberie process pour le motif suivant : la seule offre réceptionnée pour ce lot a été déclarée inacceptables et ne permet pas d'en établir l'attribution car les crédits budgétaires alloués au marché après évaluation du besoin à satisfaire ne permette"),
                (4774607, "décision de déclarer infructueux le lot nº 05 - menuiseries extérieures alu et pvc - serrurerie pour le motif suivant : toutes les offres réceptionnées sur ce lot ont été déclarées soit irrégulières soit inacceptables et ne permettent pas d'en établir l'attribution, de plus les crédits budgétaires a"),
                (4774610, "décision de déclarer sans suite le lot nº 08 - revêtements de sols durs et souples pour le motif suivant : toutes les offres réceptionnées sur ce lot ont été déclarées soit irrégulières soit inacceptables et ne permettent pas d'en établir l'attribution, de plus les crédits budgétaires alloués au mar"),
                (4877271, "décision de déclarer sans suite le Lot no15 - captage pompage et rejet pour le motif suivant : La seule offre réceptionnée pour ce lot a été déclarée inacceptable et ne permet pas d'en établir l'attribution car les crédits budgétaires alloués au marché après évaluation du besoin à satisfaire ne perm"),
                (4885217, "ce lot a été déclaré sans suite par décision du pouvoir adjudicateur en date du 11 juillet 2013. En effet, les prestations objet du lot 13 sont couvertes par un accord-cadre multi-attributaire en cours de validité, conclu par l'université Joseph Fourier. Les titulaires de cet accord-cadre bénéficien"),
                (5543579, "Aucun nom d'attributaire n'est mentionné compte tenu du fait que le lot 4 a été déclaré sans suite. un complément d'information est apporté à la Rubrique V.2 Autres informations point 2"),
                (5611738, "Sans suite, en cours d'attribution"),
                (6286857, "Suite à la cession de l'entreprise pressentie attributaire et conformément à l'article 59.IV du code des marchés publics, le marché est déclaré sans suite pour motif d'intérêt général. Avis nº 14-167620 publié au BOAMP et au JOUE"),
                (6499376, "infructueux suite à désistement du titulaire"),
                (6654938, "L'attributaire pressenti pour ce lot ayant modifié son offre, elle ne correspond plus au besoin du pouvoir adjudicateur. Lot declare sans suite"),
                (6688295, "Ce lot déclaré sans suite a fait l'objet d'une relance. Un avis d'attribution est en cours de publication"),
                (6825282, "Pour information, ce lot a été déclaré infructueux en commission d'appel d'offres. Par conséquent, ce lot n'a pas fait l'objet d'une attribution"),
                (7024169, "Pas d'attributaire retenu pour l'instant. Décision de la commission d'appels d'offres repoussée"),
                (7395857, "Declaré sans suite en raison de la liquidation judiciaire de la société attributaire"),
                (7517636, "Lot Déclaré sans suite pour motif d'intérêt général: les motifs sont fondés sur le besoin du pouvoir adjudicateur modifié entre le recensement des besoins et la date de la commission interne d'attribution"),
                (7596880, "Not awarded (was retended and awarded as part of a new project)"),
                (7596896, "Not awarded (was retendered and awarded as part of a new project)"),
                (7732764, "Marché déclaré sans suite en effet les réactifs de ce lot sont utilisés uniquement en cas de validation d'une nouvelle méthode d'analyse, ce qui n'a pas lieu d'être puisque le titulaire pressenti pour l'attribution du lot 1 est identique à celui du précédent marché"),
                (7732765, "Marché déclaré sans suite en effet les réactifs de ces lots sont utilisés uniquement en cas de validation d'une nouvelle méthode d'analyse, ce qui n'a pas lieu d'être puisque le titulaire pressenti pour l'attribution du lot 1 est identique à celui du précédent marché"),
                (9907415, "Marché déclaré sans suite le 15.6.2017, en raison d'une modification opérée par l'attributaire de son offre initiale lors d'une demande de prolongation de validité de celle-ci post attribution."),
                (10528138, "Desierto, al retirar su oferta el único licitador propuesto adjudicatario"),
                (28821451, "Not Awarded - Other FE College awarded area. V.5) THE CONTRACT IS LIKELY TO BE SUB-CONTRACTED: No"),
                (31383538, "Lot infructueux et relancer selon une procédure négocié après publicité préalable et mise en concurence selon l'article 35-1 du code des marchés publics. Voir l'avis d'attribution no 10-279996 pour l'entreprise attributaire du lot."),
                (31383542, "Lot infructueux et relancer selon une procédure négocié après publicité préalable et mise en concurence selon l'article 35-1 du code des marchés publics. Voir l'avis d'attribution no10-279996 pour l'entreprise attributaire du lot."),
                (31415171, "Lot déclaré infructueux lors de l'attribution du 13.12.2010 relancé en procédure négociée selon l'art. 35-I-1 dernier alinéa du CMP) - en cours d'attribution"),
                (31424633, "Attributaire non designe - lot infructueux"),
                (31425155, "Non attribuée - en cours d'analyse - attribution reportée"),
                (31489554, "Lot infructueux. En cours d'attribution, suite marché négocié sans publicité."),
                (31489559, "Lot infructueux. En cours d'attribution, suite marché négocié avec publicité."),
                // D4: a list of void lots.
                (2948392, "lot 9: Sans suite Lot 10: Infructueux"),
                (3717094, "lot 1 sans suite Lot 3 infructueux"),
                (4726611, "Sublote 9.1: Declarado desierto. Sublote 9.2: Declarado desierto. Sublote 9.3: Declarado desierto."),
                (4885215, "décision du pouvoir adjudicateur du 14 octobre 2013 - lot déclaré sans suite pour le motif suivant : \" l'organisation du lot par son contenu de prestations relatives à des métiers différents a rendu la réponse difficile. Donc pour un motif d'intérêt général, d'ordre économique et technique, ce lot e"),
                (5425806, "Lots 2a et 2c: procédure infructueuse lot 2b: déclaration sans suite"),
                (5425808, "Lot 4a: procédure infructueuse lots 4b/4c: déclaration sans suite"),
                (6333652, "Le présent lot n'a reçu aucune offre. Il est déclaré sans suite à l'article 59-III du code des marchés publics. Ce lot a été décomposé en 2 nouveaux lots: lot nº 06 — bâtiments démontables, lot nº 07 — travaux d'installation des bâtiments démontables, et relancé en appel d'offres ouvert le 17.7.2014"),
                (7815395, "Les lots n° 1 et 3 ont été déclarés infructueux. Le lot n° 2 a été déclaré sans suite"),
                (12169942, "Perceel 2 en Perceel 4 worden niet gegund. Er zijn geen geldige inschrijvingen ontvangen."),
                (30680275, "Marché numéro: 01072 - Lot 1) et lot 2) Marché classé sans suite"),
                (31428159, "Lot 1 sans suite / lot 2 infructueux"),
                // D5: a reason segment.
                (1513105, "Aucune offre n'a été déposée; le lot n° 2 a été déclaré infructueux"),
                (1710867, "néant; la consultation a été déclarée infructueuse en raison de l'absence d'offre recevable"),
                (1814340, "Zadanie unieważnione - Wykonawcy wykluczeni; oferty odrzucone"),
                (2369100, "nie wpłynęła żadna oferta; postepowanie unieważniono art. 93 ust. 1 pkt 1 Pzp"),
                (2678824, "Déclaré sans suite pour motif d'intérêt général et relancé (cf avis JOUE no 2012/s 133-221467 du 13.7.2012; et Annonce no 484, BOAMP no 134b, du 13.7.2012)"),
                (3534542, "La SEMAPA a déclaré sans suite l'appel d'offres relatif au marché de prestations de maquettistes ; le nombre d'offres conformes n'étant pas suffisant pour répondre aux besoins de la consultation"),
                (5148433, "Offres inacceptables ; lot déclaré infructueux"),
                (5961568, "déclaré sans suite pour un motif d'intérêt général (redéfinition du besoin) - relance en 2 lots distincts (publicité de la relance: JOUE 2014/S 161-289268 du 23.8.2014 / BOAMP parution nº 20140161b; annonce nº 63 du 23.8.2014 / profil acheteur achatpublic.com et site de la ville: 21.8.2014)"),
                (6357683, "Lot déclaré sans-suite et relancé en décembre 2014, sous les références suivantes: JOUE: n° 2014/S 250-444751 du 30.12.2014 — BOAMP: B n° 250 du 30.12.2014; annonce(s) n° 116"),
                (7075593, "postępowanie unieważniono – ofertę odrzucono; Wykonawca wykluczony – nie spełnia warunków udziału w postępowaniu"),
                (11663718, "Desierto; oferta excluida del proceso de valoración"),
                (11663720, "Desierto; el licitador ha renunciado a la adjudicación del contrato"),
                // D6: a bidder.
                (1506401, "Aucune offre n'ayant été déposée, un marché négocié sans publicité et sans mise en concurrence, sera passé avec la STÉ France Télécom"),
                (1974244, "la commission d'appel d'offres décide d'interrompre les discussions et déclare la consultation infructueuse car l'offre du groupement auxi+/ sas Viviani est inacceptable"),
                (11251840, "Lotto non aggiudicato in quanto la commissione ha escluso dalla gara l’unica offerta ricevuta dalla compagnia UnipolSai S.p.A. — Ag. Marcantoni — in quanto le varianti proposte sono state ritenute in grado di comportare una sostanziale modifica della copertura assicurativa richiesta tali da evidenzi"),
                (31527955, "Société Sindarro Lot déclaré infructueux relancé en marché négocié avec publicité et mise enconcurence (date limite le 10 juin 2008)"),
                (31527956, "Société Sanchez lot déclaré infructueux relancé en marché négocié avec publicité et mise en concurence (date limite le 10 juin 2008)"),
                (31527960, "Société Energys. Lot déclaré infructueux relancé en marché négocié avec publicité et mise en concurence (date limite le 10 juin 2008)"),
                (31527961, "Société SOTEP Lot Déclaré infructueux et sera relancé en marché negocié avec publicité et mise en concurence (date limite le 10 juin 2008)"),
                (31527964, "Société Spada. Lot déclarée infructueux relancé en procédure négocié avec publicité et mise en concurence (date limite le 10 juin 2008)"),
                (31527968, "Société Herman Aluminium Lot déclaré infructueux, relancé en marché négocié avec publicité avec mise en concurence (date limite le 10 juin 2008)"),
                // D7: a void head before a spilled section.
                (14580326, "No Contract awarded, see section VI.2"),
                (25043000, "Gara deserta per mancanza di offerte valide. SECTION VI: OTHER INFORMATION VI.3) Date of contract award: 8.4.2005. VI.4) Number of tenders received: 3. VI.7) Other information:"),
                (25755356, "Desierto. SECTION VI: OTHER INFORMATION VI.3) Date of contract award: 2.1.2004. VI.4) Number of tenders received: 12. VI.5) This contract was subject to a notice in the OJ: Yes. Of: 21.10.2003. VI.7) Other information: I"),
                (25893172, "Nicht vergeben. SECTION VI: OTHER INFORMATION VI.3) Date of contract award: 25.7.2003. VI.4) Number of tenders received: Bis zu 4. VI.5) This contract was subject to a notice in the OJ: Yes. Of: 27.5.2003. VI.7) Other information: I"),
                (29760975, "No contract awarded. SECTION VI: COMPLEMENTARY INFORMATION VI.2) ADDITIONAL INFORMATION: (ET Ref:42985). VI.3) PROCEDURES FOR APPEAL: VI.3.1) Body responsible for appeal procedures: Limerick City Council"),
                (30336777, "Not awarded. Section VI: Complementary information VI.1) Contract related to a project and/or programme financed by EU funds: No. VI.2) Additional information:"),
                (30535445, "Desiertos. SECTION VI: OTHER INFORMATION VI.1) Non-mandatory notice: No. VI.2) Reference number attributed to the file by the contracting authority: 15-3-2.1-0043/2003. VI.3) Date of contract award:"),
                (30535976, "Desierto. SECTION VI: OTHER INFORMATION VI.1) Non-mandatory notice: No. VI.2) Reference number attributed to the file by the contracting authority: 06-5-2.1-0075/2003. VI.3) Date of contract award: 22.7.2003. VI.4) Number of tenders received:"),
                (30537999, "No award made. SECTION VI: OTHER INFORMATION VI.4) Number of tenders received:"),
                (30538906, "Desierto. SECTION VI: OTHER INFORMATION VI.2) Reference number attributed to the file by the contracting authority: RQ/01/001/S. VI.3) Date of contract award:"),
                (30539882, "Desierto. SECTION VI: OTHER INFORMATION VI.1) Non-mandatory notice: No. VI.2) Reference number attributed to the file by the contracting authority: CC04/140. VI.3) Date of contract award:"),
                (30786740, "Desiertos.. SECTION VI: OTHER INFORMATION VI.1) Non-mandatory notice: No. VI.2) Reference number attributed to the file by the contracting authority: 15-3-2.1-0027/2003. VI.3) Date of contract award:"),
                (30899597, "No contract awarded — see section VI.2)"),
                (31854611, "Not awarded. Procedure cancelled according to FOA § 22-1.. SECTION VI: COMPLEMENTARY INFORMATION VI.2) ADDITIONAL INFORMATION: Criterias for area"),
                // Two rules together: D3 and D4 (1537305, 3556403), D2c and D6b (3526210).
                (1537305, "Marché classé sans suite. Il a été décidé de lancer une nouvelle procédure en appel d'offres ouvert européen en vue de l'attribution de 2 marchés se décomposant de la manière suvante: lot no 3 - agence Paul Signac lot no 4 - agence Beaumonts."),
                (3556403, "Infructuosité. Le lot no 3 a été déclarée infructueux. Le présent avis ne concerne que le lot no3 -Sud-Ouest). Date d'attribution : CAO du 17.4.2013."),
                (3526210, "Déclaré infructueux - à relancer en marché négocié, en application de l'article 35 - i 1er alinéa du code des marchés publics, avec les sociétés ayant soumissionné à l'appel d'offre : lot 5 : Société Massias"),
            ],
            Some(NotAName::VoidLot),
        );
    }

    /// Issue 520 unit 1's kept classes (§4.2–§4.5, Appendix A): every A, B, C and F org, and the
    /// 12-org D residual, keeps the class it read before the narrowings — an award summary the fold
    /// keeps, or a plain name. 31375951 (F-P) carries the tail a challenger read past the 300-character
    /// cut (`61 lots attribués à 29 fournisseurs`): on its cut name alone it reads void, as before.
    /// 7969717 (residual) reads void by itself, as before: EP 18715886's 24-language composite keeps its
    /// party by the fold's majority (2 of 24 copies void), a gap of issue 525's templates. Since U4
    /// (§4.2) only the A1 rows name a winner ([`award_winner_names_the_one_awardee`]), and since U6 the
    /// reliable A2 subset ([`void_tail_winner_cuts_the_reliable_a2_tails`]): no other A2, nor any B, C, F
    /// or residual name does — a B list must never extract.
    #[test]
    fn every_kept_520_exemption_keeps_its_class() {
        // A1 (47): an award clause or an awardee label.
        assert_class(
            &[
                (1397079, "Lot déclaré infructueux lors de l'appel d'offre puis relancé en procédure négocié sans mise en concurrence et sans publicité (art.35-2.3) avec la société B BRAUN"),
                (1397080, "Lot déclaré infructueux lors de l'appel d'offre puis relancé en procédure négocié sans mise en concurrence et dispensé de publicité (art.35-I.1°) avec la Société Vascutek"),
                (1435451, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Arrow Génériques suite à procédure négociée sans publicité avec mise en concurrence"),
                (1435452, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Abbott Products suite à procédure négociée sans publicité sans mise en concurrence"),
                (1435453, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Fresenius Kabi suite à procédure négociée sans publicité avec mise en concurrence"),
                (1435455, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Sanofi Aventis suite à procédure négociée sans publicité avec mise en concurrence"),
                (1435456, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Grunenthal/CSP suite à procédure négociée sans publicité avec mise en concurrence"),
                (1435457, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Sandoz suite à procédure négociée avec publicité avec mise en concurrence (IAP 11-32 C)"),
                (1484019, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Bionobis suite à un marché négocié sans concurrence"),
                (1484119, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société CSL Berhing, suite marché négocié sans concurrence"),
                (1484121, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société LFB BIOMEDICAMENTS, suite marché négocié sans concurrence"),
                (1503868, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la société Henry Schein suite à la passation d'un marché négocié sans publicité avec mise en concurrence."),
                (1621278, "ce lot a été déclaré infructueux par la CAO et une procédure négociée à été mise en place conformément à l'art 35 du CMP et sans nouvelle mesure de publicité. Nom de l'attributaire: Brake France Service"),
                (1668604, "procédure infructueuse, le marché a été relancé en marché négocié sans publication, ni mise en concurrence, en application de l'article 35-I-1 du code des marchés publics, puis attribué dans le cadre de cette nouvelle procédure à la Sarltacavl, pour un montant (TTC) du DDED de 408 994,55 EUR"),
                (1986366, "lot déclaré infructueux lors de l'appel d'offre puis attribué à la société Peters suite à un marché négocié sans mise en concurrence"),
                (2381362, "lot 30 relancé en marché négocié 35 II 3. Lot 31 déclaré sans suite. Lot 32 attribué à Pierre Fabre Dermatologie le 5.6.2012 (no marché 2012-2177)"),
                (2973447, "Lot infructueux relancé en marché négocié. Le lot est attribué à l'esat \"Les Ateliers de Polangis\""),
                (3863573, "Le marché a été infructueux. Après négociation, le marché a été attribué à Thermo Onix LTD"),
                (3982542, "infructueux. Relancé sous forme de marché négocié : 013086 - fournisseur retenu = zoll medical France"),
                (4552687, "marché déclaré infructueux et relancé sous la forme d'un marché négocié article 33-i1°2ème du décret 2005-1742 du 32/12/2005.marché attribué le 9/12/13 à Assurances securité 59000 Lille"),
                (4782526, "Déclaré infructueux et relancé en procédure négociée de l'article 35-I-1° du code des marchés publics. La commission d'appel d'offres réunie le 23.12.2013 a décidé d'attribuer ce lot à la société Eiffage Construction SISE rue François Durieux – 59174 La Sentinelle CE pour un montant fina lde 5 141 1"),
                (6191476, "Déclaré infructueux. Consultation relancée en procédure adaptée. Nom du titulaire: Keolis"),
                (7030205, "Lot infructueux relancé suivant procédure négociée sans publicité et sans concurrence en application de l'article 35 Ii-3 du code des marchés publics par lettre de consultation du 9.1.2015 et lettre de négociation du 07/04/15. Attributaire: ARDP"),
                (7030206, "Lot infructueux relancé suivant procédure négociée sans publicité et sans concurrence en application de l'article 35 Ii-3 du code des marchés publics, par lettre de consultation du 9.1.2015 et lettre de négociation du 07/04/15. Attributaire: JV Distribution"),
                (7279113, "infructueux — suite à négociation attribué à société Somatico"),
                (7791659, "Infructueux. Le lot a été relancé en procédure négociée (art 35 II 3°) et a été attribué à l'entreprise Transgourmet (44470 Carquefou)"),
                (7791721, "Infructueux. Le lot a été relancé en procédure négociée et a été attribué à L'entreprise Lodfirais (24100 Bergerac)"),
                (7791767, "Infructueux (1 offre irrégulière). Le lot a été relancé en procédure négociée (article 35.II.1 CMP) et a été attribué à l'entreprise Promocash (02100 Saint-Quentin)"),
                (7805728, "Marché infructueux, recours à procédure négociée: titulaire: SMACL"),
                (9104280, "À la suite de l'infructuosité de ce marché, celui-ci a été relancé selon la procédure prévue à l'article 30 du décret n° 2016-360. Il a été attribué à l'entreprise OTIS"),
                (9104287, "À la suite de l'infructuosité de ce marché, celui-ci a été relancé selon la procédure prévue à l'article 30 du décret n° 2016-360. Il a été attribué à l'entreprise Menuiserie du Chalonnais"),
                (31381712, "Déclaré infructueux lors de l'appel d'offres. Relancé en procédure négociée sans publicité sans mise en concurrence et attribuée à LD Bio Diagnostics."),
                (31393407, "lot 16-1 lait frais est déclaré sans suite le lot 16-2 est attribué à Nectarys"),
                (31482009, "lot déclaré infructueux le 21.2.2011, procédure négociée relancée avec nouvelle publicité selon les termes de l'article 35.I al.1 du code des marchés publics, marché attribué à: Faure/Annequin"),
                (31482010, "lot déclaré infructueux le 21.2.2011, procédure négociée relancée sans nouvelle publicité selon les termes de l'article 35.I al.1 du code des marchés publics, marché attribué à: Berthelet"),
                (31482011, "lot déclaré infructueux le 21.2.2011, procédure négociée relancée sans nouvelle publicité selon les termes de l'article 35.I al.1 du code des marchés publics, marché attribué à: Segui"),
                (31482013, "Lot déclaré infructueux le 21.2.2011, procédure négociée relancée sans nouvelle publicité selon les termes de l'article 35.I al.1 du code des marchés publics, marché attribué à: Cars du Trièves"),
                (31482014, "lot déclaré infructueux le 21.2.2011, procédure négociée relancée avec nouvelle publicité selon les termes de l'article 35.I al.1 du code des marchés publics, marché attribué à: Faure/Eyraud"),
                (31482016, "Lot déclaré infructueux le 21.2.2011, procédure négociée relancée sans nouvelle publicité selon les termes de l'article 35.I al.1 du code des marchés publics, marché attribué à: Eyraud/Perraud"),
                (31738609, "lot 25) la société Storz a été retenue. Le lot 32 a été déclaré infructueux"),
            ],
            Some(NotAName::AwardSummary),
        );
        assert_class(
            &[
                (7151483, "Pas d'offre. Relancé en procédure 35.ii.3 sous le numéro 2015_20403_0016 Attribué à Léo Lagrange"),
                (7151494, "Pas d'offre. Relancé en procédure 35.ii.3 sous le numéro 2015_20403_0016 Attribué à FAIL 13"),
                (7151502, "Pas d'offre. Relancé en procédure 35.ii.3 sous le numéro 2015_20403_0016 Attribué à C.A.J.L"),
                (7151511, "Pas d'offre. Relancé en procédure 35.ii.3 sous le numéro 2015_20403_0016 Attribué à Apis"),
                (7151529, "Pas d'offre. Relancé en procédure 35.ii.3 Attribué à Léo Lagrange (AAPC n° 2015_20403_0016)"),
                (7151543, "Pas d'offre. Relancé en procédure 35.ii.3 sous le numéro 2015_20403_0016 Attribué à A.E.C. Castellane"),
                (7151562, "pas d'offre. Relancé en procédure 35.ii.3 sous le numéro 2015_20403_0016 Attribué à san see"),
            ],
            None,
        );
        // A2 (10): a name before a void tail. A1's extractor reads none of them; since U6 the reliable
        // subset names its winner before the tail ([`void_tail_winner_cuts_the_reliable_a2_tails`]).
        assert_class(
            &[
                (1844226, "BGE Guyane 16 rue Lt Becker 97300 Cayenne / La consultation a été déclarée sans suite pour les parts non couvertes des besoins"),
                (1844229, "BGE Guyane 16 rue Lieutenant Becker 97300 Cayenne - la consultation a été déclarée sans suite pour les parts non couvertes des besoins"),
                (1844232, "BGE Guyane 16 rue Lieutenant Becker 97300 Cayenne/ la consultation a été déclarée sans suite pour les parts non couvertes des besoins"),
                (2035260, "LV Consultants Cv1 Simarouba - 97310 Kourou, FRANCE/ La consultation a été déclarée sans suite pour la part du marché non couverte des besoins"),
                (2734624, "GRPT GCC (mandataire) et Andre BTP, suite à procédure négociée après AO infructueux"),
                (2734625, "GRPT vallée sas (mandataire), La Comec et Arbat System, suite à procédure négociée après AO infructueux"),
                (3993625, "Pentax France life care montants mini : lot 10 : 10 054 EUR HT et lot 11 : 4 399 EUR HT Les lots 1 à 9 sont déclarés sans suite"),
                (4836476, "Bp innovis ; infructueux"),
                (31689490, "Procédure de marché négocié après appel d'offres infructueux: 5) Mepy Système"),
                (31739133, "Importaciones Canarias de Automóviles SA (Lote A2.5). Se ha declarado desierto el Lote A3.2. Nacionalidad: española"),
            ],
            Some(NotAName::AwardSummary),
        );
        for (org, name) in A2_WINNERS.iter().map(|&(org, name, _)| (org, name)).chain(A2_UNRELIABLE) {
            assert_eq!(super::award_winner(name), None, "{org}: {name}");
        }
        // B1 (13): one distinct winner in a list grammar.
        assert_class_and_no_winner(
            &[
                (2957546, "lot 26: Carestream Lot 27: Sans suite lot 28: CARESTREAM"),
                (5351956, "lot 7.0401 : pansements raffin Lot 7.04.02 : sans suite"),
                (5351964, "lot 9.02.01 : sans suite Lot 9.02.02 : Vygon"),
                (6603163, "Stryker France SAS: lot nº 36 sans suite: lot nº 37"),
                (7685392, "Gallego secteurs 3A à 3D / infructueux secteurs 3E à 3H /"),
                (7685396, "Dubreuilh secteurs 7A 7B / Infructueux secteurs 7C 7D"),
                (8747281, "Secteur 74 Sud: AMS (+30 %) et non attribué (infructueux) secteur 74 Nord: AMS (+30 %) et non attribué (infructueux)"),
                (8747284, "74 secteur Sud: ACS DECO (+5 %) et non attribué (infructueux) 74 secteur Nord: ACS DECO (+3 %) et non attribué (infructueux)"),
                (8788741, "Secteur Allier: lot non attribué (infructueux) Secteur Puy-de-Dôme: Dômes Etanch (0 %)"),
                (8788743, "Secteur Allier: lot non attribué (infructueux) Secteur Puy-de-Dôme: SARF (0 %)"),
                (31518419, "Paredes lot 506 i Les Paluds - 140 avenue du Millet Bp 1340 lots 501 à 505 infructueux"),
                (31919182, "Agence d'Amboise : appel d'offres déclaré sans suite - Agence de Chinon : savelys GDF suez"),
                (31919183, "Mongin Jauffret : lots 1504 et 1505 Zi Delta industrie la Valentine lots infructueux : 1501, 1502 et 1503"),
            ],
            Some(NotAName::AwardSummary),
        );
        // B2 (10): co-winners of one result.
        assert_class_and_no_winner(
            &[
                (1813067, "1/ Gidef, 37 rue du chemin latérale, 93140 Bondy - 2/ Auxet, Bât 1, apt du gazon, Res. Les champs, 97139 Les Abymes. La consultation a été déclarée sans suite pour la part non couverte des besoins"),
                (1813070, "1/ CE Cedille, quartier AKR, 97218 Basse Pointe - 2/ Gidef, 37 rue du chemin latérale, 93140 Bondy. La consultation a été déclarée sans suite pour la part non couverte des besoins"),
                (1813087, "1/ Gidef, 37 rue du chemin latérale, 93140 Bondy - 2/ AECD, 56 avenue des Chutes Lavie, 13004 Marseille. La consultation a été déclarée sans suite pour la part non couverte des besoins"),
                (1844213, "Equinoxe Formation Rocade Zéphir Centre Commercial Katoury 97300 Cayenne / BGE Guyane 16 rue Lt Becker 97300 Cayenne/ La consultation a été déclarée sans suite pour la part non couverte des besoins"),
                (1844215, "BGE Guyane 16 rue Lt Becker 97300 Cayenne / Equinoxe Formation Rocade Zéphir Centre Commercial Katoury 97300 Cayenne / La consultation a été déclarée sans suite pour la part non couverte des besoins"),
                (1844218, "BGE Guyane 16 rue Lt Becker 97300 Cayenne/ Equinoxe Formation Rocade Zéphir Centre Commercial Katoury 97300 Cayenne/ La consultation a été déclarée sans suite pour la part non couverte des besoins"),
                (1844224, "BG Guyane 16 rue Lt Becker 97300 Cayenne / Point Com 77 rue René Jadfard 97300 Cayenne / La consultation a été déclarée sans suite sur la part non couverte des besoins"),
                (4726614, "Sublote 12.1: Bernadí, S.A.; Expert Line, S.L.; S&T 96, S.L. Sublote 12.2: Declarado desierto. Sublote 12.3: Declarado desierto."),
                (5425803, "Lot 10a: déclaration sans suite lot 10b: procédure infructueuse lot 10c titulaire 1: SAGA titulaire 2: Chapelec"),
                (15591728, "Rang 1: Helmlinger, Rang 2: Jonnette, Rang 3: Infructueux"),
            ],
            Some(NotAName::AwardSummary),
        );
        // B3 (76): several winners in one award block.
        assert_class_and_no_winner(
            &[
                (1722464, "2012021901 = JBE Electrcité; 2012021902 = Someci; 2012021903 = Burdiwatt; 2012021904 = Infructueux; 2012031901 = Mont Blanc Electrcité; 2012051901 = ATH; 2012051902 = Krice Elec; 2012061901 & 2012061902 = Infructueux; 2012061903 = Sanuelec; 2012071901 = Coteb; 2012071902 = Gandit; 2012081901 = SPIE "),
                (1722465, "2012023401 = Cle de Sol; 2012023402 = Infructueux; 2012043401 = Logis Services; 2012053401 = ATH; 2012053402 = MCB; 2012053403 = Vittet; 2012093401 = Laffin"),
                (1722466, "2012030801 = Infructueux; 2012040801 = Decobois; 2012050801 = ATH; 2012050802 = Logis Services; 2012050803 = Infructueux; 2012070801 = Logis Services; 2012080801 = Menui 2 B; 2012080802 = Allo Lyon Depannage; 2012080803 = Villette Renovations"),
                (1722468, "2012041701 = DFG Travaux; 2012041702 = SETIM; 2012041703 = DFG Travaux; 2012051701 = ATH; 2012051702 = Proxiserve; 2012051703 = SETIM; 2012051704 = Thermy Confort; 2012051705 = Infructueux; 2012061701 = Proxiserve; 2012061702 = Proxiserve; 2012061703 = Magnien; 2012061704 = Magnien; 2012061705 = Pro"),
                (1722469, "2012043201 = Logis Services; 2012063201 = ART SMA; 2012073201 = Infructueux; 2012083201 = Logis Services"),
                (2381358, "lot 10 attribué à Bayer Santé Familiale le 5.6.2012 (no marché 2012-2179). Lot 11 relancé en marché négocié 35 II 3. Lot 12 relancé en marché négocié 35i1. Lot 13 déclaré sans suite (arrêt de commercialisation). Lot 14 attribué à Pfizer le 5.6.2012 (no marché 2012-2180)"),
                (2381360, "lot 19 attribué à Mylan le 5.6.2012 (no marché 2012-2183). Lot 20 attribué à Pierre Fabre Dermatologie le 5.6.2012 (nomarché 2012-2177). Lot 21 déclaré sans suite. Lot 22 attribué à EG Labo le 5.6.2012 (no marché 2012-2184)"),
                (2381361, "lot 23 attribué à Pierre Fabre Dermatologie le 5.6.2012 (no marché 2012-2177). Lot 24 déclaré sans suite. Lot 25 attribué à Sinclair Pharma le 5.6.2012 (no marché 2012-2185). Lots 26 et 28 attribués à Glaxosmithline le 5.6.2012 (no marché 2012-2186). Lot 29 attribué à MSD France le 5.6.2012 (no marc"),
                (2450314, "procédure déclaré sans suite relancée en 2 lots distincts: 5a menuiseries intérieures bois: attributaire: L'Art du bois, 2 rue Georges Politzer 38 Echirolles. Montant HT: 48 225,34 EUR. Date d'attribution: 24.4.2012. 5b mobilier bois: attributaire: Ellipse Bois, ZA en Chuel, 69 Quincieux. Montant HT"),
                (2957539, "lot 35: ETIAM Lot 36: KEOSYS Lot 37: Infructueux"),
                (2957542, "lot 1: Philips Lot 2: Philips Lot 3: Infructueux Lot 4: PHILIPS Lot 5: SIGIL Lot 6: PHILIPS Lot 7: Sans suite"),
                (2957543, "lot 8: ge medical systems Lot 9: ge medical systems Lot 10: Sans suite Lot 11: SIGIL Lot 12: Infructueux Lot 13: Sans suite"),
                (2957545, "lots 19 à 22: EUROMEDICA Lot 23: SIGIL Lots 24 et 25: Infructueux"),
                (2957549, "lot 32: Medrad Lot 33: Covidien Lot 34: Infructueux"),
                (3219667, "2013023401/zone Bourgoin Jallieu-Nord Isère : Jonyk Franck - 69720 St Laurent de Mure ; 2013023402/zone Belley-Bas Bugey : Infructueux ; 2013023403/zone Bourgoin Jallieu-Nord Isère : Sarl Da Cunha - 69124 Colombier Saugnieu ; 2013033401/zone Pays de Gex : SOS 01 Dépannage - 01360 St Genis Pouilly ; "),
                (3219671, "2013023201/zone Belley-Bas Bugey : Sarl Mcb - 01300 Chazey Bons ; 2013033201/zone Pays de Gex : AMS - 74000 Annecy ; 2013033202/zone Pays de Gex : SOS 01 Dépannage - 01630 St Genis Pouilly ; 2013073201 Zone Bresse : Logis Services - 69120 Vaulx en Velin ; 2013073202/zone Bresse : infructueux ; 20130"),
                (3219672, "2013024201/zone Bourgoin Jallieu-Nord Isère : Abs décors - 38090 Villefontaine ; 2013024202/zone Bourgoin Jallieu-Nord Isère : Parducci - 69140 Rillieux la Pape ; 2013024203/zone Bourgoin Jallieu-Nord Isère : Netsol expansion - 69150 Décines ; 2013024204/zone Belley-Bas Bugey : Infructueux ; 2013034"),
                (3371493, "lot268 sanofi av - 269sanofi AV - 270renaudin - 271navartis P - 272 PFIZER - 273msd - 274 sanofi av - 275aguettant - 276arrow G - 277healthcare - 278astra Z - 279 PFIZER - 280mylan - 281arrow G - 282 sans suite - 283 ACTELION - 284leo P - 285leo P - 286infructueux - 287bms - 288infructueux - 289sans"),
                (3371495, "lot 418 - gLAXOSMITHKLINE - 1691.1ht Lot419 GLAXOSMITHKLINE - 94488.6 ht lot 420 - msd f 32544.72 ht lot 421 sanofi av - 69365.01 ht lot 422 sans suite lot 423 sanofi av 1352.46 LOT 424 meda p 1447.2"),
                (3371500, "lot 643 - sans suite - lOT 644 MYLAN - 2459.7ht LOT 645 infructueux lot 646 BMS - 26581.5 ht lot 647 msd france 63072 ht lot 648 janssen c 30016.8 ht lot 649 BMS 2208.91ht LOT 650 jansse c 353446.8 ht lot 651 viiv healthcare 891 ht lot 652 boehringer ingelheim 7608.9 LOT 653 TOCHE 1575 ht lot 654 ms"),
                (3371513, "lot1045sans SUITE- lot1046infructueux- lot1047serb 83268ht- lOT 1048-sans suite- lot 1049serb 4032ht- lot1050infructueux- lot1051infructueux- lot1052cooper 3817.965ht- lot1053ferring 6780ht- lot1054ipsen P 60000- 1055csp 7650- 1056ferring 2520- 1057-genzyme256300.5ht- lot1058infructueux- lot1059stal"),
                (3371518, "lot1096 INFRUCTUEUX- lot1097 fresenius k 22500- lot1098 fresenius m 396- lot1099 INFRUCTUEUX- lot1100 INFRUCTUEUX- lot1101 INFRUCTUEUX- lot1102 INFRUCTUEUX- lot1103 INFRUCTUEUX- lot1104 BAXTER 42750- lot1105 maco p 27090- lot1106 HOSPAL 256 690ht-lot 1107 HEMOTECH 84000ht- lot1108 HOSPAL 31500ht- lo"),
                (3371519, "lot1125 INFRUCTUEUX- lot1126 GILBERT 11400 Ht-Lot1127 BAXTER 13241.8ht- lot1128 BAXTER 169215ht- lot1129 AGUETTANT 12884.7ht- lot1130 fresenius k 7650ht- lot1131 Gilbert48000ht- lot1132 fresenius k 368269.7ht- lot1133 AGUETTANT 199566.2ht-"),
                (3371520, "lot1112 AGUETTANT 42600.36- lot1113 INTERSURGICAL 9765- lot1114 STALLERGENES 862.56- lot1115 DIAGAST 136797.8- lot1116 INFRUCTUEUX- lot1117 INFRUCTUEUX- lot1118 chaix et 6190.954- lot1119 sans suite- lot1120 FENWALL 3060 HT- lot1121 GENZYME 156000 HT- lot1122 GILBERT 5353.92ht-lot1123 sans suite- lo"),
                (3371521, "lot470 INFRUCTUEUX - lOT 471infructueux- lOT 472infrucueux- lOT 473carl ZEISS 6300ht- lOT 474alcon 88500ht - lOT 475 PHYSIOL 32850 (H.T.)."),
                (3371523, "lot 43 INFRUCTUEUX - 44 reckitt benckiser 28692.07 - 45 reckitt b 165.24 - 46 ipsen p 2497.138- 47 ASTELLAS 590.15- 48 BMS 35091.84 - 49 AGUETTANT 17954.86 - 50 PFM 63767.71 - 51 CSP 16062.45 - 52 INFRUCTUIEUX - 53 sans suite - 54 arrow g 9005.64 - 55 INFRUCTUEUX - 56 sans suite - 57ipsen P 4395.461"),
                (3371525, "lot109 BAYER 5530.56 - 110 INFRUCTUEUX - 111cooper 6243.12 - 112 leo p 977 - 113 leo p 643.37 - 113 leo p 453.6 - 114 leo p 1092.6 - 115 eg labo csp 12315.58 - 116 COOPER 2787.89 - 117 CSP 9538.36 - 118 galenique v 10108.17 - 119 INFRUCTUEUX - 120 galeniques v 36010.62 - 121 merck s 19041.03 - 122 n"),
                (4048470, "pro a pro (lots no 47, 48, 49, 50, 51, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 68, 69) SODIAL (lot no 52), lot no 67 (sans suite)"),
                (4804536, "2014024201/zone Belley-Bas Bugey : PARDUCCI -69140 rillieux la pape ; 2014024202/zone Belley bas Bugey : ezm peinture - 01300 BELLEY ; 2014024203/zone Belley Bas Bugey : INFRUCTUEUX ; 2014034201/zone Pays de Gex : cazajous decor - 01200 bellegarde sur valserine ; 2014034202/zone Pays de Gex : NUANCE"),
                (5351950, "lot 6.01.01 : PARAMAT Lot 6.01.02 : PARAMAT Lot 6.01.03 : PARAMAT Lot 6.01.04 : PARAMAT Lot 6.01.05 : PARAMAT Lot 6.01.06 : sans suite Lot 6.01.07 : novo nordisk Lot 6.01.08 : APOTECNIA Lot 6.01.09 : APOTECNIA Lot 6.01.10 APOTECNIA Lot 6.01.11 : APOTECNIA Lot 6.01.12 : APOTECNIA Lot 6.01.13 : APOTEC"),
                (5351959, "lot 8.03.01 : pansements raffin Lot 8.03.02 : pansements raffin Lot 8.03.03 : pansements raffin Lot 8.03.04 : pansements raffin Lot 8.03.05 : Hartmann Lot 8.03.06 : Hartmann Lot 8.03.07 : Hartmann Lot 8.03.08 : sans suite Lot 8.03.09 : molnlycke healthcare Lot 8.03.10 : molnlycke healthcare Lot 8.03"),
                (5351961, "lot 8.05.01 : pansements raffin Lot 8.05.02 : Lohmann et Rausscher Lot 8.05.03 : Hartmann Lot 8.05.04 : sans suite"),
                (5351965, "lot 10.01.01 : Paramat Lot 10.01.02 : pansements raffins Lot 10.01.03 : sans suite"),
                (5351966, "lot 10.02.01 : Teleflex Lot 10.02.02 : Novo Nordisk Lot 10.02.03 : ABS Lot 10.02.04 : BSN médical Lot 10.02.05 : sans suite Lot 10.02.06 : Cooper"),
                (6603158, "Infructueux: lot nº 24 Orthomed: lots nº 25 et 26 Biotech Ortho: lot nº 27"),
                (6603162, "Biotech Ortho: lot nº 32 Zimmer France: lot nº 33 Depuy France SA: lot nº 34 lot nº 35: infructueux"),
                (6603175, "Depuy France sa: lots 2, 3, 4 Stryker France SAS: lot 5 Arthrex: lot 6 sans suite: lot 7"),
                (7512199, "Lots 2, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20 attribués à TOPOS SARL —  4 rue des Artisans — 67270 Bernardswiller — lots 21 et 22 attribués à G2C Territoire — 26 chemin de Fondeyre —  31200 Toulouse — lots 1, 3, 9, 23 déclarés sans suite"),
                (7685393, "Acorus secteurs 4A et 4D/ Maisonning 4B et 4C/ Infructueux secteurs 4E et 4F / Chenelec secteurs 4G et 4H"),
                (7685394, "SPIE secteurs 5A à 5D/ infructueux 5E 5F/ Marc Elec 5G/ Chenelec 5H"),
                (7685395, "Acorus secteurs 6A à 6D/ Infructueux secteurs 6E 6F / Chenelec secteurs 6G 6H"),
                (8788747, "Secteur Allier: grange (0 %) et Fayolle (-4 %) Secteur Puy-de-Dôme: lot non attribué (infructueux)"),
                (8788751, "Secteur Allier: AFP (0 %); DIAZ (-4 %) et infructueux Secteur Puy-de-Dôme: AFP (-23 %); SARF (-10 %); Volcapaint (-10 %) et Vallée Dome Finitions (0 %)"),
                (9811971, "Lot 1: Meal-o-Matic; Lots 2,4 et 5: Carolo Drink Service; Lot 3: non attribué; Lot 6: Topsec Équipement"),
                (10497189, "Secteur 1-2-3 MENUISERIE NORD GIRONDE; Secteur 4 HAROCARENE; Secteur 5-6 Procédure infructueuse."),
                (23645704, "CAT8-MS1 - Attribué à Ganix, CAT8-MS2 - Attribué à RacinesUrbaines, CAT8-MS3 - Attribué à Rebiffé, CAT8-MS4 - Non attribué, CAT8-MS5 - Attribué à Ets Lefeuvre, CAT8-MS6 - Attribué à LePain2Mains"),
                (31393408, "Gaudais Distribution (lots 1-1, 1-3, 1-5) Pomona Passion Frois (lots 1-2,1-4) Transgourmet (lot 1-6) lot 1-7 est déclaré sans suite Earl Lefleche (lot 1-6)"),
                (31428158, "Lot 3 Brake / lot 9 Davigel / lot 5 le Roy / lots 7,2,4,10 le Saint / lot 11 Pomona PF / lot 6, 1 Pomona TA / lot 8 infructueux"),
                (31428160, "Lots 9, 10, 4, 5, 8, 6, 7 Bernard / lots 2,13 Brake / lots 3,1 Pomona PF / lot 14 SDA / lot 15 terre et plume / lots 11,12 sans suite"),
                (31497710, "GE Healthcare Clinical Systems - 69760 Limonest: lot 1: 157 555,30 EUR HT; Philips France - 92156 Suresnes: lot 2: 150 845,70 EUR HT; lot 3: 77702,90 EUR HT; lot 4: 51217,50 EUR HT; lot 5: 28 199 EUR HT; le lot 6 a été déclaré sans suite"),
                (31518405, "Adhesia lot 1001 et 1006 Coldis : lots 103, 1004, 1005 et 1007 lots infructueux : 1002"),
                (31518406, "Coldis : lots 1101, 1102, 1103, 1104, 1105 et 1107 Abena frantex : lot 1111 lots infructueux : 1107, 1108, 1109 et 1110"),
                (31518407, "Mongin jauffret lot 1220 à lot 1237 Chomette favor : lot 1238 sauf lot 1226 infructueux"),
                (31518408, "Paredes : lot 1301, 1309, 1310, 1311, 1317, 1319 1320, 1324, 1325, 1327, 1328 et 1329 Coldis : lot 1302, 1308, 1312, 1318, 1321 et 1323 Argos hygiène : lots 1313, 1314 et 1322 Cedisud : lot 1315 Chomette favor : lot 1316 lots infructueux : 1303, 1304, 1305, 1306, 1307, 1326 et 1329"),
                (31518409, "Paredes : lot 1330 Coldis : lot 1331 Abena frantex : lot 1332 Groupe 5 s : lot 1333 Coldis : lot 1335 et 1337 Eif : lot 1336 lot infructueux : 1334"),
                (31518416, "Usagunic lots 2001 et 2002 Apmp lot 2003 lots infructueux : 2004, 2005 et 2006"),
                (31518420, "Argos hygiène lots 601 et 612 Eumarcom lots 602 et 616 Coldis lots 603, 615, 618 et 620 Paredes lots 604, 605, 607, 610 et 619 Thermofina lot 608 lots infructueux : 609, 611, 613 et 614"),
                (31518422, "Coldis lots 901, 902, 903 et 909 Argos lots 904, 906, 908 Phs lot 907 Adhesia lot 910 lots infructueux : 905 et 911"),
            ],
            Some(NotAName::AwardSummary),
        );
        assert_class_and_no_winner(
            &[
                (1642155, "BRI: lot 1: marché n° 12.001 / Arthrex: lots 7 et 10: marché n° 12.006/ Stryker: lots 8-9-24-25-27: marché n° 12.009 / Heraeus: lots 4 et 5: marché n° 12.004 / Science et médecine: lots 6-19-20: marché n° 12.005 Zimmer France: lot 3: marché n° 12.003 Orthofix: lot 26: marché n° 12.008/ Compagnie fin"),
                (3219666, "2013021901/zone Belley Bas Bugey : Gandit - 69120 Vaulx-en-Velin ; 2013031901/zone Pays de Gex : JS Electricité - 01200 Confort ; 2013051901/zone Oyonnax : ABT Electricité - 01000 Bourg-en-Bresse ; 2013051902/zone Oyonnax - Pascal Gritti - 01110 Hauteville-Lompnes ; 2013061901/zone Chalon-sur-Saône "),
                (3219668, "2013020801/zone Bourgoin-Nord Isère : Logis services - 69120 Vaulx en Velin ; 2013020802/zone Belley-Bas Bugey : Sarl MCB - 01300 Chazey-Bons ; 2013030801/ Zone Pays de Gex : SOS 01 Dépannage - 01630 St Genis Pouilly ; 2013060801/zone Agence de Chalon : RB dépannage - 71000 Maçon ; 2013060802/zone A"),
                (3290486, "096-13 LOT 1 A.C.H. - Depann'Express 40100 DAX 097-13 Lot3 TSMP 40180 NARROSSE 098-13 Lot4 CG.POSES 40390 st laurent de gosse 099-13 LOT 5 andre morlaes 40400 TARTAS 100-13 LOT 6 eurl vieira 40990 st paul les dax 101-13 LOT 7 eurl denis crepin 40400 carcen ponson 102-13 LOT 8 massy et fils 40180 HEU"),
                (3371526, "lot 197 LFB Bi969134 - 198 lfb bio 142692 - 199 lfb bio 98455 - 200 lfb bio 98455 - 200 lfb bio 4995220 - 201 BAXTER 1011344 - 202 BAXTER 81225 - 203 lfb bio 946.74 - 204 TAKEDA 249000 - 205 novo n 5413632 - 206 lfb bio 2702700 - 207 lfb bio 151200 -208 lfb bio 1438080 - 209 lfb bio 126480 - 211 INF"),
                (3586203, "Sous-lot nº 1 – barquettes, assiettes à usage unique et papier film : Firplast SAS, 4/6 rue de Provence, 69800 Saint-Priest. Montant : 149 755 EUR HT. Sous-lot nº 2 – vaisselle à usage unique : Paredes, 1 rue Georges Besse, BP 302, ZI de Revoisson, 69745 Genas Cedex. Montant : 5 206 EUR HT. Sous-lot"),
                (4359873, "-321-13 lots 1-2-3-4-14-15 france inox 40180 SAUBUSSE -322-13 lot6 compagnie hobart 77183 croissu beaubourg -323-13 lot7 aft equipement 34210 ARBONNE -324-13 lots 8-9 ESSOR 64600 ANGLET -325-13 lot10 le froid pyreneen 64140 BILLERE -326-13 Lot 11 SEFTI 40350 POUILLON Les lots 12, 13, sont déclarés i"),
                (4804532, "2014033401/zone Pays de Gex : ninet gavin - 01200 LANCRANS ; 2014033402/zone Bellegarde sur Valserine : CUTELLE - 01200 bellegarde sur valserine ; 2014053401/zone Oyonnax : logis services - 69120 vaulx en velin ; 2014063401/zone Autun : art sma - 71230 st vallier ; 2014063402/zone Autun : bois serru"),
                (4804533, "2014030801/zone Pays de Gex : ninet gavin - 01200 LANCRANS ; 2014030802/zone Pays de Gex : DUCIMETIERE - 01200 GENISSIAT ; 2014030803/zone Bellegarde sur Valserine : ninet gavin - 01200 LANCRANS ; 2014050801/ zone Oyonnax : VITTET - 01100 OYONNAX ; 2014060801/zone Autun : art sma - 71230 st vallier "),
                (4804535, "2014020801/zone Bourgoin Jallieu-Nord Isère : logis services - 69120 vaulx en velin ; 2014020802/zone Bourgoin Jallieu-Nord Isère : EGV - 69100 VILLEURBANNE ; 2014020803/zone Belley-Bas Bugey : logis services - 69120 vaulx en velin ; 2014020804/zone Belley-Bas Bugey : MCB - 01300 chazey bons ; 20140"),
                (4804537, "2014022801/zone Belley Bas Bugey : BATILYON - 69140 rillieux la pape ; 2014022802/zone Belley Bas Bugey : PARDUCCI - 69140 rillieux la pape ; 2014022803/zone Belley-Bas Bugey : BERNARD - 38690 TORCHEFELON ; 2014022804/zone Bourgoin Jallieu-Nord Isère : abs decors - 38090 VILLEFONTAINE ; 2014022805/z"),
                (6209983, "2015020801/zone Bourgoin-Jallieu-Nord Isère: Ecco bat service — 38090 Villefontaine; 2015020802/zone Belley-Bas Bugey: ID services — 42500 Saint-Romain-en-Jarez; 2015040801/zone agence de Rillieux: Serfal — 69009 Lyon; 2015040802/zone agence de Rillieux: SEMI — 69800 Saint-Priest; 2015040803/zone ag"),
                (6209984, "2015024201/zone Bourgoin-Jallieu-Nord Isère: Parducci — 69140 Rillieux-la-Pape; 2015024202/zopne Bourgoin-Jallieu-Nord Isère: ABS deocrs — 38090 Villefontaine; 2015024203/zone Bourgoin-Jallieu-Nord Isère: SCOP NEA — 73492 La Ravoire; 2015024204/zone Belley-Bas Bugey: Parducci — 69140 Rillieux-la-Pap"),
                (6672986, "2 lots attribués à la société Alere (78350)le 27/04/2015; 11 lots attribués à la société Becton Dickinson (38801) le 16/04/2015; 22 lots attribués à la société Biomérieux (69280) le 15/04/2015; 6 lots attribués à la société Bio-Rad (92430) le 17/04/2015; 1 lot attribué à la société Elitech (92800) l"),
                (6672989, "2 lots attribués à la société Alldiag (67038) le 24/04/2015; 1 lot attribué à la société Bio-Rad (92430) le 07/05/2015; 26 lots attribués à la société CML (77792) le 17/04/2015; 4 lots attribués à la société Greiner Bio One (91941) le 16/04/2015; 3 lots attribués à la société Less France (77500) le "),
                (7674818, "orthomed(2015-2652) pour les lots 1,2,et4; GROUPE Lepine(2015-2658) pour les lots 5,6 et9; BIOTECH Ortho(2015-2657) pour les lots8,34,35,36,37 et 38; Depuy(2015-2660) pour les lots 7,12,14,19,20,21,22,23,24,25,26,29,31 et 33; ETHICON Sas(2015-2659) pour le lot 30; FH Orthopedics(2015-2651) pour le l"),
                (31414415, "Lots suivants attribués à la société RMI, 1 rue Blaise Pascal, 54320 Maxeville, FRANCE: 4.1 (écrans TFT): 5 439,15 EUR HT / 4.2 (portables): 36 319,94 EUR HT / 4.3 (unités centrales): 22 445,10 EUR HT / 4.7 (routeurs): 1 413,65 EUR HT / 4.8 (téléphones IP): 3 140,00 EUR HT / 4.9 (complément HP Blade"),
                (31528120, "MARCHE 4008821.083-08 ALDIS-BORGEL Lots : 11-12-14-18 MARCHE 4008821.084-08 BRAKE Lots : 21 MARCHE 4008821.085-08 CAFES RICHARD Lots : 19 MARCHE 4008821.086-08 FRIGERAL Lots : 20 MARCHE 4008821.087-08 POMONA Lots : 4-24 MARCHE 4008821.088-08 PRO A PRO DISTRIBUTION SUD Lots : 10-15 MARCHE 4008821.089"),
            ],
            None,
        );
        // C (29): text-era `Lot n)` lists.
        assert_class_and_no_winner(
            &[
                (26966868, "Lote I: declarado desierto. Lote II: Winterthur Seguros Generales"),
                (27028633, "Lot 1) non attribution pour cause administratif. Lot 2) Hospithera"),
                (27567379, "Lot 3) Figuères Services. Lots 1 et 2 déclarés infructueux"),
                (30593829, "lot 2) sans suite. lot 3) Aloka Sàrl"),
                (30665788, "Lot 3) sans suite. Lot 4) Ets. S.A.D. (73)"),
                (30666364, "Lot 1) véhicules légers: Foucque. Lot 2) engin équipé d'un robot tailleur: infructueux. Lot 3) engin de terrassement: Nassibou. Lot 4) engin de nivelage-compactage: Nicisa. Lot 5) camion de dépannage"),
                (30680323, "Lot 1) Vallée de l'Oise et lot 2 (vallée de la Serre et du Vilpion): déclarés infructueux. Lot 2) Vallée de la Marne: IWACO France"),
                (30686841, "Lot 1) SPLO. Lot 2) SPLO. Lot 3) Distralls. Lot 4) SPLO. Lot 5) Prodirest. Lot 6) SPLO. Lot 7) Prodirest. Lot 8) Prodirest. Lot 9) Prodirest. Lot 10) Déclaré infructueux. Lot 11) Déclaré infructueux. Lot 12) Distralis. Lot 13) Rault"),
                (30687978, "Lote 1) Declarado desierto. Lote 2) Limpiezas Luz"),
                (30688273, "Lot 1) sans suite. Lot 2) Astem Sécurité"),
                (30689270, "Lot 1) Dactyl Buro. Lot 2) Buro +. Lot 3) infructueux. Lot 4) Buro +. Lot 5) infructueux. Lot 6) Willecocq. Lot 7) Rover. Lot 8) Navarre. Lot 8bis) Willecocq. Lot 9) infructueux. Lot 10) Buro +"),
                (30690240, "Lots 1) et 2) Entreprise Mazza. Lot 3) GCTélécom Lor 4) déclaré sans suite"),
                (30695615, "A) non attribué. B1) et B2) Climelec"),
                (30698459, "Lot 1) déclaré infructueux. Lots 2)"),
                (30820452, "Lot 1) USP Normandie SA. Lot 2) déclaré infructueux. Lot 3) déclaré infructueux"),
                (31689502, "Bernas Médical. Procédure de marché négocié après appel d'offres infructueux: 28) Kodack"),
                (31731990, "G) non attribué. Marché conserves: A)"),
                (31731993, "E) non attribué. Marché produits d'entretien: A)"),
                (31733948, "Lot 1) Sté Doumenge. Lot 2) SA Rault. Lot 3) Sté Discol-Prodirest. Lot 4) SA Rault. Lot 5) Lot infructueux. Lot 6) Sté Discol-Prodirest. Lot 7) Sté Doumenge. Lot 8) Sté Doumenge. Lot 9) Sté Elidis. Lot 10) Sté Andrieux. Lot 11) Sté GDA. Lot 12) Sté Krill. Lot 13) Sté Krill. Lot 14) Sté GDA. Lot 15) "),
                (31734131, "Lot 1) Sans suite. Lot 2) Infructueux. Lot 3) Progetech. Lot 4) Vidalauto. Lot 5) Vidalauto. Lot 6) Vidalauto Lot 7) L'approvisionnement électrique. Lot 8) Vidalauto. Lot 9) Descours et Cabaud. Lot 10) Vidalauto. Lot 11) Vidalauto. Lot 12) Vidalauto. Lot 13) Vidalauto. Lot 14) Descours et Cabaud. Lo"),
                (31734422, "lot 4) Marché infructueux. Marché 4: SA Cofida d'Hauwers"),
                (31735007, "Lot 1) Blin. Lot 2) Prodirest. Lot 3) Ouest Distribution. Lot 4) Ouest Distribution. Lot 5) Corol Jeager. Lot 6) Déclaré infructueux. Lot 7) Prodirest. Lot 8) Déclaré infructueux. Lot 9) Déclaré infructueux. Lot 10) Blin. Lot 11) Prodirest. Lot 12) Déclare infructueux. Lot 13) Ame Haslé. Lot 14) Gef"),
                (31738162, "Lot 1) Société SEF Environnement. Lot 2) Société France Elagage. Lot 3) Société SEF Environnement. Lot 4) et 5) lot déclaré infructueux - il sera relancé ultérieurement sous forme de marché négocié"),
                (31738464, "Lot 1) Norisolec. Lot 2) Non attribué. Lot 3) Millery. Lot 4) Thommeret. Lot 5) Sauval. Lot 6) Isaac. Lot 7) Cuiller. Lot 8) Roussel. Lot 9) SGM. Lot 10) Menuiserie Devilloise. Lot 11) Delaloche. Lot 12) Abrisol. Lot 13) CTMC. Lot 14) Gamm. Lot 15) Sogeris. Lot 16) Savec. Lot 17) Follin. Lot 18) ATC"),
                (31740148, "Lot 1) Roche Diagnostics. Lot 2) Dade Behring. Lot 3) Biomerieux. Lot 4) Infructueux. Lot 5) Biomerieux. Lot 6) Biomerieux. Lot 7) Biomedical Diagnostics. Lot 8) Abbott. Lot 9) Biomerieux. Lot 10) Abbott. Lot 11) Instrumentation Laboratory. Lot 12) Sebia. Lot 13) Abbott. Lot 14) Servibio. Lot 15) Se"),
                (31740189, "Lot n° 1) Renault France VI S.A. Lot n° 2) SES. Lot n° 3) W41 TP. Lots n° 4) et n° 6) Coullare. Lot n° 5) Mauguin CMTP. Lot n° 7) CTD Pulvérisation. Lot n° 8) Europe Service. Lot n° 9) appel d'offres déclaré sans suite. Une nouvelle procédure est engagée"),
            ],
            Some(NotAName::AwardSummary),
        );
        assert_class_and_no_winner(
            &[
                (31734397, "Lot 1) Tissus Gisele. Lot 2) Tissus Gisele. Lot 3) Mulliez. Lot 4) Tissus Gisele. Lot 5) Tissus Gisele. Lot 6) Tissus Gisele. Lot 7) Tissus Gisele. Lot 8) Centraco. Lot 9) Aucune réponse. Lot 10) CTD. Lot 11) CTD. Lot 12) CTD. Lot 13) TTE. Lot 14) Mulliez. Lot 15) Mulliez. Lot 16) Mulliez. Lot 17) H"),
                (31736174, "Lot 1) Didactic. Lot 2) Vygon. Lot 3) LPI. Lot 4) LPI. Lot 5) Allegiance. Lot 6) Terumo. Lot 7) Vygon. Lot 8) NM Medical. Lot 9) Porges. Lot 10) Owen Mumford. Lot 11) CIM. Lot 12) Swann Morton. Lot 13) Bbraun. Lot 14) Tyco. Lot 15) Tyco. Lot 16) Peters. Lot 17) Tyco. Lot 18) Dorvit. Lot 19) Non attr"),
                (31737127, "Lot 1) \"papier offset\": société Antalis. Lot 2) \"papier bristol et dossiers\": société Buhrmann. Lot 3) \"papier autocopiant\": société Top Papers. Lot 4) \"papier photocopieurs et imprimantes\": société Agenpa. Lot 5) \"papier quadrichromie\": société Papeteries du Dauphiné. Lot 6) \"papier autocollant\": s"),
            ],
            None,
        );
        // F (6): an award with no name (F-P), and eForms tenderers marked not awarded (F-role).
        assert_class_and_no_winner(
            &[
                (1341883, "Demander le détail à la cellule marchés sont établis 66 marchés pour l'ensemble des 257 lots attribués Les 74 lots infructueux sont relancés 2 lots sans suite (n° 12 et 300)"),
                (31375951, "Au terme de cette analyse et conformément aux avis et suggestions émis par commission des dispositifs médicaux, réunie le 6 décembre dernier, les choix opérés sont les suivants: - 11 lots ont été déclarés \"Sans suite\", - 5 lots sont déclarés \"Infructueux\", en raison de l'absence de soumissionnaire,  … 61 lots attribués à 29 fournisseurs"),
                (31377625, "La consultation comportait 310 lots (circuits). 220 lots ont été attribués le 22.7.2010. 90 lots ont été déclarés infructueux - aucune consultation n'a été relancé pour ces lots. La liste des attributaires des 220 marchés (circuits) est présentée ci-dessous. Le circuit se décompose de la façon suiva"),
                (31486027, "attribué par marché négocié passé selon art. 35-I.1 du CMP suite à appel d'offres infructueux (publicité AAPC parue dans BOAMP no 83 du 28.4.2011 - annonce no 221 et JOUE no 2011/S 81-133894 du 27.4.2011)"),
            ],
            Some(NotAName::AwardSummary),
        );
        // F-role: since issue 527 TenderNed's mark is no part of the name, so the two read as plain names
        // (kept either way); the fold drops their winner role on the mark itself.
        assert_class_and_no_winner(
            &[(30533823, "Humble Inpect B.V. (niet gegunde partij)"), (30533824, "Provadi B.V. (niet gegunde partij)")],
            None,
        );
        // The D residual (11 of 12): each costs a junk leaf name, never an award.
        assert_class_and_no_winner(
            &[
                (4409817, "Projet déclaré sans suite : relancé sous le n° de projet ESID 12 418, attribué le 28.10.2013"),
                (4409820, "Projet déclaré sans suite : relancé sous le n° de projet ESID 12 419, attribué le 28.10.2013"),
                (4409821, "Projet déclaré sans suite : relancé sous le n° de projet ESID 12 420, attribué le 28.10.2013"),
                (4802635, "Perceel 1 wordt niet gegund – opdracht wordt gegund op totaal der percelen = perceel 3"),
                (4802636, "Perceel 2 wordt niet gegund – opdracht wordt gegund op totaal der percelen = perceel 3"),
                (10893714, "Perceel 1 NIET gegungd B.V."),
                (23532535, "Déclaration sans suite du lot n°15 et du lot n°17- Recomposition d’un lot -Relance de consultation du lot n°15 « Chape-Carrelage-Faïence »."),
                (31391828, "non retenus: lots 6-44-56-68-74-91-94-95 sans suite: lots 9-65-92-96"),
                (31497872, "Le directeur de l'atelier industriel de l'aéronautique de Clermont-Ferrand à déclaré sans suite les lots 10, 15, 17, et 19 indiqués ci-dessus en date du 1.7.2011 et les lots 14 et 18 indiqués ci-dessus déclarés sans suite par le directeur de l'AIA de Clermont-Ferrand en date du 5.9.2011"),
                (31733023, "aucune entreprise ne s'est portée candidate pour le marché négocié passé suite à l'appel d'offres infructueux"),
            ],
            Some(NotAName::AwardSummary),
        );
        assert_class_and_no_winner(
            &[
                (28310858, "Desierto lote 4 y 5 por falta de licitadores. Desierto lotes 1"),
            ],
            None,
        );
        assert_class_and_no_winner(&[(7969717, "Non applicable — Aucune offre n'a été reçue — Lot 5 non attribué")], Some(NotAName::VoidLot));
    }

    /// Issue 520 U4 (unit-1 decision §4.2, Appendix B): the A1 extractor names the one awardee of each
    /// of the 47 A1 orgs (49 mentions), exactly as App. B reads it and as published — `CSL Berhing`,
    /// `Sarltacavl`, `Léo Lagrange`, `san see` — from its award clause (`attribué à`, `d'attribuer ce lot
    /// à`, `avec la société` closing a past relaunch, `la société X a été retenue`) or its awardee label
    /// (`Attributaire:`, `Nom du titulaire:`, `titulaire:`, `fournisseur retenu =`). Each is in the walk's
    /// net and a name the fold keeps (an award summary, or a plain name after `Pas d'offre`), so the fold
    /// consults it. The strip and terminator cases keep a capital article (`La Poste`), single-letter dots
    /// (`A.E.C. Castellane`, `C.A.J.L`) and a slash pair (`Grunenthal/CSP`), and end at a comma (`CSL
    /// Berhing,`), a postcode, a date, `SISE`, a void segment. The near misses (§4.2) name a future
    /// counterparty, a relaunch's bidders, a failed, rejected, excluded or liquidated bidder, a pointer, an
    /// award with no party, a count, or several awardees: nothing.
    #[test]
    fn award_winner_names_the_one_awardee() {
        const A1: [(u64, &str, &str); 47] = [
            (1397079, "Lot déclaré infructueux lors de l'appel d'offre puis relancé en procédure négocié sans mise en concurrence et sans publicité (art.35-2.3) avec la société B BRAUN", "B BRAUN"),
            (1397080, "Lot déclaré infructueux lors de l'appel d'offre puis relancé en procédure négocié sans mise en concurrence et dispensé de publicité (art.35-I.1°) avec la Société Vascutek", "Vascutek"),
            (1435451, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Arrow Génériques suite à procédure négociée sans publicité avec mise en concurrence", "Arrow Génériques"),
            (1435452, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Abbott Products suite à procédure négociée sans publicité sans mise en concurrence", "Abbott Products"),
            (1435453, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Fresenius Kabi suite à procédure négociée sans publicité avec mise en concurrence", "Fresenius Kabi"),
            (1435455, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Sanofi Aventis suite à procédure négociée sans publicité avec mise en concurrence", "Sanofi Aventis"),
            (1435456, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Grunenthal/CSP suite à procédure négociée sans publicité avec mise en concurrence", "Grunenthal/CSP"),
            (1435457, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Sandoz suite à procédure négociée avec publicité avec mise en concurrence (IAP 11-32 C)", "Sandoz"),
            (1484019, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Bionobis suite à un marché négocié sans concurrence", "Bionobis"),
            (1484119, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société CSL Berhing, suite marché négocié sans concurrence", "CSL Berhing"),
            (1484121, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société LFB BIOMEDICAMENTS, suite marché négocié sans concurrence", "LFB BIOMEDICAMENTS"),
            (1503868, "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la société Henry Schein suite à la passation d'un marché négocié sans publicité avec mise en concurrence.", "Henry Schein"),
            (1621278, "ce lot a été déclaré infructueux par la CAO et une procédure négociée à été mise en place conformément à l'art 35 du CMP et sans nouvelle mesure de publicité. Nom de l'attributaire: Brake France Service", "Brake France Service"),
            (1668604, "procédure infructueuse, le marché a été relancé en marché négocié sans publication, ni mise en concurrence, en application de l'article 35-I-1 du code des marchés publics, puis attribué dans le cadre de cette nouvelle procédure à la Sarltacavl, pour un montant (TTC) du DDED de 408 994,55 EUR", "Sarltacavl"),
            (1986366, "lot déclaré infructueux lors de l'appel d'offre puis attribué à la société Peters suite à un marché négocié sans mise en concurrence", "Peters"),
            (2381362, "lot 30 relancé en marché négocié 35 II 3. Lot 31 déclaré sans suite. Lot 32 attribué à Pierre Fabre Dermatologie le 5.6.2012 (no marché 2012-2177)", "Pierre Fabre Dermatologie"),
            (2973447, "Lot infructueux relancé en marché négocié. Le lot est attribué à l'esat \"Les Ateliers de Polangis\"", "Les Ateliers de Polangis"),
            (3863573, "Le marché a été infructueux. Après négociation, le marché a été attribué à Thermo Onix LTD", "Thermo Onix LTD"),
            (3982542, "infructueux. Relancé sous forme de marché négocié : 013086 - fournisseur retenu = zoll medical France", "zoll medical France"),
            (4552687, "marché déclaré infructueux et relancé sous la forme d'un marché négocié article 33-i1°2ème du décret 2005-1742 du 32/12/2005.marché attribué le 9/12/13 à Assurances securité 59000 Lille", "Assurances securité"),
            (4782526, "Déclaré infructueux et relancé en procédure négociée de l'article 35-I-1° du code des marchés publics. La commission d'appel d'offres réunie le 23.12.2013 a décidé d'attribuer ce lot à la société Eiffage Construction SISE rue François Durieux – 59174 La Sentinelle CE pour un montant fina lde 5 141 1", "Eiffage Construction"),
            (6191476, "Déclaré infructueux. Consultation relancée en procédure adaptée. Nom du titulaire: Keolis", "Keolis"),
            (7030205, "Lot infructueux relancé suivant procédure négociée sans publicité et sans concurrence en application de l'article 35 Ii-3 du code des marchés publics par lettre de consultation du 9.1.2015 et lettre de négociation du 07/04/15. Attributaire: ARDP", "ARDP"),
            (7030206, "Lot infructueux relancé suivant procédure négociée sans publicité et sans concurrence en application de l'article 35 Ii-3 du code des marchés publics, par lettre de consultation du 9.1.2015 et lettre de négociation du 07/04/15. Attributaire: JV Distribution", "JV Distribution"),
            (7151483, "Pas d'offre. Relancé en procédure 35.ii.3 sous le numéro 2015_20403_0016 Attribué à Léo Lagrange", "Léo Lagrange"),
            (7151494, "Pas d'offre. Relancé en procédure 35.ii.3 sous le numéro 2015_20403_0016 Attribué à FAIL 13", "FAIL 13"),
            (7151502, "Pas d'offre. Relancé en procédure 35.ii.3 sous le numéro 2015_20403_0016 Attribué à C.A.J.L", "C.A.J.L"),
            (7151511, "Pas d'offre. Relancé en procédure 35.ii.3 sous le numéro 2015_20403_0016 Attribué à Apis", "Apis"),
            (7151529, "Pas d'offre. Relancé en procédure 35.ii.3 Attribué à Léo Lagrange (AAPC n° 2015_20403_0016)", "Léo Lagrange"),
            (7151543, "Pas d'offre. Relancé en procédure 35.ii.3 sous le numéro 2015_20403_0016 Attribué à A.E.C. Castellane", "A.E.C. Castellane"),
            (7151562, "pas d'offre. Relancé en procédure 35.ii.3 sous le numéro 2015_20403_0016 Attribué à san see", "san see"),
            (7279113, "infructueux — suite à négociation attribué à société Somatico", "Somatico"),
            (7791659, "Infructueux. Le lot a été relancé en procédure négociée (art 35 II 3°) et a été attribué à l'entreprise Transgourmet (44470 Carquefou)", "Transgourmet"),
            (7791721, "Infructueux. Le lot a été relancé en procédure négociée et a été attribué à L'entreprise Lodfirais (24100 Bergerac)", "Lodfirais"),
            (7791767, "Infructueux (1 offre irrégulière). Le lot a été relancé en procédure négociée (article 35.II.1 CMP) et a été attribué à l'entreprise Promocash (02100 Saint-Quentin)", "Promocash"),
            (7805728, "Marché infructueux, recours à procédure négociée: titulaire: SMACL", "SMACL"),
            (9104280, "À la suite de l'infructuosité de ce marché, celui-ci a été relancé selon la procédure prévue à l'article 30 du décret n° 2016-360. Il a été attribué à l'entreprise OTIS", "OTIS"),
            (9104287, "À la suite de l'infructuosité de ce marché, celui-ci a été relancé selon la procédure prévue à l'article 30 du décret n° 2016-360. Il a été attribué à l'entreprise Menuiserie du Chalonnais", "Menuiserie du Chalonnais"),
            (31381712, "Déclaré infructueux lors de l'appel d'offres. Relancé en procédure négociée sans publicité sans mise en concurrence et attribuée à LD Bio Diagnostics.", "LD Bio Diagnostics"),
            (31393407, "lot 16-1 lait frais est déclaré sans suite le lot 16-2 est attribué à Nectarys", "Nectarys"),
            (31482009, "lot déclaré infructueux le 21.2.2011, procédure négociée relancée avec nouvelle publicité selon les termes de l'article 35.I al.1 du code des marchés publics, marché attribué à: Faure/Annequin", "Faure/Annequin"),
            (31482010, "lot déclaré infructueux le 21.2.2011, procédure négociée relancée sans nouvelle publicité selon les termes de l'article 35.I al.1 du code des marchés publics, marché attribué à: Berthelet", "Berthelet"),
            (31482011, "lot déclaré infructueux le 21.2.2011, procédure négociée relancée sans nouvelle publicité selon les termes de l'article 35.I al.1 du code des marchés publics, marché attribué à: Segui", "Segui"),
            (31482013, "Lot déclaré infructueux le 21.2.2011, procédure négociée relancée sans nouvelle publicité selon les termes de l'article 35.I al.1 du code des marchés publics, marché attribué à: Cars du Trièves", "Cars du Trièves"),
            (31482014, "lot déclaré infructueux le 21.2.2011, procédure négociée relancée avec nouvelle publicité selon les termes de l'article 35.I al.1 du code des marchés publics, marché attribué à: Faure/Eyraud", "Faure/Eyraud"),
            (31482016, "Lot déclaré infructueux le 21.2.2011, procédure négociée relancée sans nouvelle publicité selon les termes de l'article 35.I al.1 du code des marchés publics, marché attribué à: Eyraud/Perraud", "Eyraud/Perraud"),
            (31738609, "lot 25) la société Storz a été retenue. Le lot 32 a été déclaré infructueux", "Storz"),
        ];
        for (org, name, winner) in A1 {
            assert_eq!(super::award_winner(name).as_deref(), Some(winner), "{org}: {name}");
            assert!(mentions_void(name), "{org}: the walk's net holds it");
            assert!(matches!(not_a_name(name), None | Some(NotAName::AwardSummary)), "{org}: the fold keeps it");
        }
        // Strip and terminator cases (id 0: written for the test).
        for (name, winner) in [
            ("Lot déclaré infructueux puis attribué à La Poste", "La Poste"),
            ("Lot infructueux, relancé et attribué à la Société Dupont SARL ; lot 2 infructueux", "Dupont SARL"),
            ("Infructueux, puis attribué à l'entreprise Martin Frères - 69003 Lyon", "Martin Frères"),
            ("Infructueux. Lot relancé, attribué à Durand BTP 69003 Lyon", "Durand BTP"),
            ("Lot infructueux. Relancé et attribué à « Les Jardins de Cocagne »", "Les Jardins de Cocagne"),
            ("Lot infructueux puis attribué à l'issue de la négociation à la société Dupont SARL", "Dupont SARL"),
            ("Infructueux. Le lot a été attribué à la société Dupont. Le lot 2 a été attribué à la Société DUPONT", "Dupont"),
            ("Infructueux, relancé en procédure négociée avec la société Martin Frères.", "Martin Frères"),
            ("Infructueux. Relancé : Attributaire : Keolis Lyon", "Keolis Lyon"),
        ] {
            assert_eq!(super::award_winner(name).as_deref(), Some(winner), "0: {name}");
        }
        // The near misses: each names no one awardee.
        for (org, name) in [
            // A future counterparty, a relaunch's bidders (D2c, D6b).
            (1506401, "Aucune offre n'ayant été déposée, un marché négocié sans publicité et sans mise en concurrence, sera passé avec la STÉ France Télécom"),
            (3526206, "Déclaré infructueux - à relancer en marché négocié, en application de l'article 35 - i - 1er alinéa du code des marchés publics, avec les sociétés ayant soumissionné à l'appel d'offre : lot 1 : société TIB"),
            (3526210, "Déclaré infructueux - à relancer en marché négocié, en application de l'article 35 - i 1er alinéa du code des marchés publics, avec les sociétés ayant soumissionné à l'appel d'offre : lot 5 : Société Massias"),
            (3526991, "décalré infructueux= le lot 2 infructueux et de le relancer en marché négocié en application de l'article 35 - iI - 3ème alinéa du code des marchés publics, avec la société LHENRY"),
            (4032000, "Offres déclarées irrégulières. Lot déclaré donc infructueux. Il a donc été proposé de négocier avec les sociétés candidates dans le cadre de la procédure prévue à l'article 35-I-1 3ème alinéa du code des marchés publics par une lettre de consultation dont le formalisme est prévu à l'article 66 du co"),
            (0, "Lot infructueux, il sera attribué à la société Dupont"),
            (0, "Lot infructueux à relancer : la société Dupont a été retenue pour négocier"),
            // A failed, rejected, excluded or liquidated bidder (D6, D3).
            (31527955, "Société Sindarro Lot déclaré infructueux relancé en marché négocié avec publicité et mise enconcurence (date limite le 10 juin 2008)"),
            (31527961, "Société SOTEP Lot Déclaré infructueux et sera relancé en marché negocié avec publicité et mise en concurence (date limite le 10 juin 2008)"),
            (1974244, "la commission d'appel d'offres décide d'interrompre les discussions et déclare la consultation infructueuse car l'offre du groupement auxi+/ sas Viviani est inacceptable"),
            (11251840, "Lotto non aggiudicato in quanto la commissione ha escluso dalla gara l’unica offerta ricevuta dalla compagnia UnipolSai S.p.A. — Ag. Marcantoni — in quanto le varianti proposte sono state ritenute in grado di comportare una sostanziale modifica della copertura assicurativa richiesta tali da evidenzi"),
            (1633917, "résultat du concours: déclaration sans suite pour motif de défaillance du mandataire de l'équipe, suite à la liquidation judiciaire de l'agence Toth-Fasquelle préalablement à la signature du marché de maîtrise d'oeuvre"),
            (7395857, "Declaré sans suite en raison de la liquidation judiciaire de la société attributaire"),
            // A pointer, a negated or another procedure's award, an award with no party, a count.
            (31383538, "Lot infructueux et relancer selon une procédure négocié après publicité préalable et mise en concurence selon l'article 35-1 du code des marchés publics. Voir l'avis d'attribution no 10-279996 pour l'entreprise attributaire du lot."),
            (0, "Lot infructueux - Attributaire : voir annexe"),
            (0, "Lot infructueux. Le marché n'a pas été attribué à la société Dupont"),
            (4409817, "Projet déclaré sans suite : relancé sous le n° de projet ESID 12 418, attribué le 28.10.2013"),
            (31674188, "A 97.23.18) sans suite (déjà attribué au lots A 06 et A 12)"),
            (1341883, "Demander le détail à la cellule marchés sont établis 66 marchés pour l'ensemble des 257 lots attribués Les 74 lots infructueux sont relancés 2 lots sans suite (n° 12 et 300)"),
            (31375951, "Au terme de cette analyse et conformément aux avis et suggestions émis par commission des dispositifs médicaux, réunie le 6 décembre dernier, les choix opérés sont les suivants: - 11 lots ont été déclarés \"Sans suite\", - 5 lots sont déclarés \"Infructueux\", en raison de l'absence de soumissionnaire,  … 61 lots attribués à 29 fournisseurs"),
            (31486027, "attribué par marché négocié passé selon art. 35-I.1 du CMP suite à appel d'offres infructueux (publicité AAPC parue dans BOAMP no 83 du 28.4.2011 - annonce no 221 et JOUE no 2011/S 81-133894 du 27.4.2011)"),
            (0, "Infructueux, relancé en procédure négociée avec la société Dupont, pour un montant de 3 000 EUR"),
            // Two or more awardees, or a list after the name (B).
            (2381358, "lot 10 attribué à Bayer Santé Familiale le 5.6.2012 (no marché 2012-2179). Lot 11 relancé en marché négocié 35 II 3. Lot 12 relancé en marché négocié 35i1. Lot 13 déclaré sans suite (arrêt de commercialisation). Lot 14 attribué à Pfizer le 5.6.2012 (no marché 2012-2180)"),
            (6672986, "2 lots attribués à la société Alere (78350)le 27/04/2015; 11 lots attribués à la société Becton Dickinson (38801) le 16/04/2015; 22 lots attribués à la société Biomérieux (69280) le 15/04/2015; 6 lots attribués à la société Bio-Rad (92430) le 17/04/2015; 1 lot attribué à la société Elitech (92800) l"),
            (7512199, "Lots 2, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20 attribués à TOPOS SARL —  4 rue des Artisans — 67270 Bernardswiller — lots 21 et 22 attribués à G2C Territoire — 26 chemin de Fondeyre —  31200 Toulouse — lots 1, 3, 9, 23 déclarés sans suite"),
            (23645704, "CAT8-MS1 - Attribué à Ganix, CAT8-MS2 - Attribué à RacinesUrbaines, CAT8-MS3 - Attribué à Rebiffé, CAT8-MS4 - Non attribué, CAT8-MS5 - Attribué à Ets Lefeuvre, CAT8-MS6 - Attribué à LePain2Mains"),
            (5425803, "Lot 10a: déclaration sans suite lot 10b: procédure infructueuse lot 10c titulaire 1: SAGA titulaire 2: Chapelec"),
            (0, "Lot infructueux. titulaire: SAGA titulaire: Chapelec"),
            (2450314, "procédure déclaré sans suite relancée en 2 lots distincts: 5a menuiseries intérieures bois: attributaire: L'Art du bois, 2 rue Georges Politzer 38 Echirolles. Montant HT: 48 225,34 EUR. Date d'attribution: 24.4.2012. 5b mobilier bois: attributaire: Ellipse Bois, ZA en Chuel, 69 Quincieux. Montant HT"),
            (31414415, "Lots suivants attribués à la société RMI, 1 rue Blaise Pascal, 54320 Maxeville, FRANCE: 4.1 (écrans TFT): 5 439,15 EUR HT / 4.2 (portables): 36 319,94 EUR HT / 4.3 (unités centrales): 22 445,10 EUR HT / 4.7 (routeurs): 1 413,65 EUR HT / 4.8 (téléphones IP): 3 140,00 EUR HT / 4.9 (complément HP Blade"),
            (0, "Lot infructueux puis attribué à la société Dupont SARL ; lot 2 : Martin SA"),
        ] {
            assert_eq!(super::award_winner(name), None, "{org}: {name}");
        }
        // A name with none of the cue words reads nothing, and ordinary names never reach the cues.
        assert!(!super::may_name_a_winner("Lot 1) Sarl Bremond. Lot 2) Infructueux"));
        assert!(super::may_name_a_winner("Pas d'offre. ATTRIBUÉ À Apis"));
    }

    /// Issue 520 U4, the adversarial review: a cue names the awardee only when the clause around the name
    /// states an award made and kept. Each of these names nobody — a negated or quantified cue e
    /// (`aucune société n'a été retenue`, which read the elided `n'` as an org named `n`); a cue d whose
    /// `avec` another clause governs (failed or broken-off negotiations, a contract not concluded or
    /// rescinded, in its own sentence or not); a cue a or b under a refusal, an impossibility, a plan, a
    /// subjunctive, a `jamais`, a negation past `encore`, or an earlier award; an award its sentence undoes
    /// after the name (withdrawn, annulled, rescinded, liquidated) or a former awardee's label; a per-lot
    /// list whose other entry names another winner (§4.3/§4.4: B and C never extract) or two awardees glued
    /// into one name; and a role, a pointer, a count, a quality or a status where the name would stand,
    /// which every notice writing it would share as one organization. The counterparts that state the
    /// award still name it.
    #[test]
    fn award_winner_reads_the_clause_around_the_name() {
        for name in [
            // Cue e negated or quantified, or undone after the name.
            "Lot infructueux, aucune société n'a été retenue (Dupont SARL écartée)",
            "Lot infructueux : aucune société n'a été retenue. Lot 2 : Dupont SARL",
            "Lot infructueux. la société Dupont, attributaire pressentie, a été retenue puis s'est désistée",
            "Lot infructueux. la société Dupont a été retenue mais s'est désistée",
            // Cue d governed by another clause.
            "Lot infructueux relancé en procédure négociée mais déclaré sans suite après échec des négociations avec la société Dupont",
            "Lot infructueux relancé en procédure négociée, négociations rompues avec la société Dupont",
            "Lot infructueux relancé en procédure négociée. Négociations en cours avec la société Dupont",
            "Lot infructueux relancé en procédure négociée, aucun accord trouvé avec la société Dupont",
            "Lot infructueux relancé en procédure négociée ; marché non conclu avec la société Dupont",
            "Lot infructueux relancé en procédure négociée ; contrat résilié avec la société Dupont",
            "Lot infructueux relancé en procédure négociée ; litige en cours avec la société Dupont",
            "Lot infructueux relancé en procédure négociée ; l'offre a été rejetée après négociation avec la société Dupont",
            "Lot infructueux, négocié sans succès avec la société Dupont",
            // Cues a and b refused, impossible, planned, subjunctive, never, not yet.
            "Lot infructueux. La CAO a refusé d'attribuer ce lot à la société Dupont",
            "Lot infructueux. Il n'a pas été possible d'attribuer ce lot à la société Dupont",
            "Lot infructueux. Il est impossible d'attribuer ce lot à la société Dupont",
            "Lot infructueux. La CAO envisage d'attribuer ce lot à la société Dupont",
            "Lot infructueux. La CAO a renoncé d'attribuer ce lot à la société Dupont",
            "Lot infructueux. Lot pas encore attribué à la société Dupont",
            "Lot infructueux. Lot non encore attribué à la société Dupont",
            "Lot infructueux. Jamais attribué à la société Dupont",
            "Lot infructueux. Il est envisagé qu'il soit attribué à la société Dupont",
            // An earlier award, or one undone after the name.
            "Le lot 3, initialement attribué à la société Dupont, a été déclaré sans suite suite à la liquidation judiciaire de celle-ci",
            "Lot infructueux. Marché précédemment attribué à la société Dupont",
            "Lot infructueux. Lot attribué à la société Martin, qui s'est désistée",
            "Lot infructueux. Lot attribué à la société Martin (annulé)",
            "Lot infructueux. Lot attribué à la société Martin - décision annulée par le tribunal administratif",
            "Lot sans suite : marché attribué à la société Dupont, résilié pour faute",
            "Lot infructueux puis attribué à la société Dupont qui a été mise en liquidation",
            "Lot infructueux. Le marché attribué à la société Dupont a été résilié.",
            "Lot déclaré sans suite. Attributaire : Dupont SARL (désisté)",
            "Lot déclaré sans suite. Titulaire : Dupont SARL - marché résilié",
            "Lot déclaré infructueux. Ancien titulaire: Dupont",
            "Lot déclaré infructueux. Précédent attributaire : Dupont SARL",
            // A per-lot list with another winner, or two awardees in one name.
            "Lot 1) attribué à Dupont. Lot 2) Martin. Lot 3) infructueux",
            "Lot 1 : infructueux - Lot 2 attribué à Dupont - Lot 3 : Martin SARL",
            "Lot 2 attribué à Dupont, lot 3 : Martin SARL, lot 4 infructueux",
            "Lot 1) infructueux. Lot 2) attribué à la société Dupont. Lot 3) Martin SA",
            "Lot 1 attributaire : Dupont. Lot 2 : Martin. Lot 3 : sans suite",
            "lot 1 infructueux ; lot 2 attribué à Dupont SARL, lot 3 Martin SA",
            "Lot 1) Martin. Lot 2) attribué à Dupont. Lot 3) infructueux",
            "Lots 1 et 2 infructueux, lot 3 attribué à Dupont SARL et lot 4 à Martin SA",
            "Lot infructueux puis attribué à la société Dupont et à la société Martin",
            "Lot infructueux puis attribué à la société Dupont à Lyon",
            // A role, a pointer, a count, a quality or a status where the name would stand.
            "Lot infructueux. Attributaire : idem lot 1",
            "Lot infructueux puis attribué à l'unique soumissionnaire, la société Dupont",
            "Lot infructueux. Lot attribué à l'entreprise",
            "Lot infructueux puis attribué à la société (voir annexe)",
            "Lot infructueux puis attribué à la société.",
            "Lot infructueux puis attribué à l'entreprise, pour un montant de 5000 EUR",
            "Lot infructueux puis attribué à la société suivante : Dupont",
            "Lot infructueux. Attributaire : titulaire du lot 1",
            "Lot infructueux. Attributaire : attributaire du lot 1",
            "Lot infructueux puis attribué à l'attributaire du lot 2",
            "Lot infructueux puis attribué à l'entreprise ayant remis la meilleure offre",
            "Lot infructueux puis attribué à l'entreprise mieux-disante",
            "Lot infructueux puis attribué à deux entreprises",
            "Lot infructueux puis attribué à trois sociétés",
            "Lot infructueux puis attribué à des entreprises différentes",
            "Lot infructueux puis attribué à chacun des candidats",
            "Lot infructueux puis attribué à la même entreprise que le lot 2",
            "Lot infructueux puis attribué à la même entreprise",
            "Lot infructueux puis attribué à un prestataire local",
            "Lot infructueux puis attribué à l'offre économiquement la plus avantageuse",
            "Lot infructueux puis attribué à un montant de 12 000 EUR",
            "Lot infructueux. Montant attribué à ce lot : 0 EUR",
            "Lot infructueux puis attribué à la CAO du 12 mai",
            "Lot infructueux puis attribué à la séance du 12 mai",
            "Lot infructueux puis attribué au pouvoir adjudicateur à Monsieur le Maire",
            "Lot infructueux puis attribué à Monsieur le Maire",
            "Lot infructueux puis attribué à la candidat classé premier",
            "Lot infructueux. Attributaire : idem",
            "Lot infructueux. Attributaire : ibid",
            "Lot infructueux. Attributaire : même",
            "Lot infructueux. Attributaire : ci-dessous",
            "Lot infructueux. Attributaire : ci-après",
            "Lot infructueux. Attributaire : identique au lot n° 3",
            "Lot infructueux. Attributaire : se reporter à l'avis d'attribution",
            "Lot infructueux. Attributaire : consulter l'avis d'attribution",
            "Lot infructueux. Attributaire : en cours",
            "Lot infructueux. Attributaire : en attente",
            "Lot infructueux. Attributaire : à venir",
            "Lot infructueux. Attributaire : à désigner",
            "Lot infructueux. Attributaire : non désigné",
            "Lot infructueux. Attributaire : non publié",
            "Lot infructueux. Attributaire : confidentiel",
            "Lot infructueux. Attributaire : inconnu",
            "Lot infructueux. Attributaire : personne",
            "Lot infructueux. Attributaire : rien",
            "Lot infructueux. Attributaire : pas de titulaire",
            "Lot infructueux puis attribué à Ets.",
        ] {
            assert_eq!(super::award_winner(name), None, "{name}");
        }
        // The counterparts: an award stated and kept names its awardee.
        for (name, winner) in [
            ("Lot infructueux. la société Dupont SARL, Lyon, a été retenue", "Dupont SARL"),
            ("Lot infructueux, il n'a été attribué qu'après négociation à la société Martin", "Martin"),
            ("Lot infructueux. La CAO réunie le 3.2.2014 a décidé d'attribuer ce lot à la société Dupont", "Dupont"),
            ("Lot infructueux. Nouvel attributaire : Dupont SARL", "Dupont SARL"),
            ("Lot 1) infructueux. Lot 2) attribué à la société Dupont. Lot 3) sans suite", "Dupont"),
            ("Lot infructueux puis attribué à la société Dupont pour le lot 2", "Dupont"),
        ] {
            assert_eq!(super::award_winner(name).as_deref(), Some(winner), "{name}");
        }
    }

    /// Issue 520 U6 (A2, unit-1 decision §4.2): A2's reliable subset, each row as the walk's dry 2375 prints
    /// it, and the winner [`super::void_tail_winner`] cuts from it.
    const A2_WINNERS: [(u64, &str, &str); 5] = [
        (1844226, "BGE Guyane 16 rue Lt Becker 97300 Cayenne / La consultation a été déclarée sans suite pour les parts non couvertes des besoins", "BGE Guyane"),
        (1844229, "BGE Guyane 16 rue Lieutenant Becker 97300 Cayenne - la consultation a été déclarée sans suite pour les parts non couvertes des besoins", "BGE Guyane"),
        (1844232, "BGE Guyane 16 rue Lieutenant Becker 97300 Cayenne/ la consultation a été déclarée sans suite pour les parts non couvertes des besoins", "BGE Guyane"),
        (2734624, "GRPT GCC (mandataire) et Andre BTP, suite à procédure négociée après AO infructueux", "GRPT GCC (mandataire) et Andre BTP"),
        (2734625, "GRPT vallée sas (mandataire), La Comec et Arbat System, suite à procédure négociée après AO infructueux", "GRPT vallée sas (mandataire), La Comec et Arbat System"),
    ];

    /// Issue 520 U6: A2's other rows — the four the decision reads as unreliable, and 4836476, whose `;`
    /// shape the review withdrew.
    const A2_UNRELIABLE: [(u64, &str); 5] = [
        (2035260, "LV Consultants Cv1 Simarouba - 97310 Kourou, FRANCE/ La consultation a été déclarée sans suite pour la part du marché non couverte des besoins"),
        (3993625, "Pentax France life care montants mini : lot 10 : 10 054 EUR HT et lot 11 : 4 399 EUR HT Les lots 1 à 9 sont déclarés sans suite"),
        (4836476, "Bp innovis ; infructueux"),
        (31689490, "Procédure de marché négocié après appel d'offres infructueux: 5) Mepy Système"),
        (31739133, "Importaciones Canarias de Automóviles SA (Lote A2.5). Se ha declarado desierto el Lote A3.2. Nacionalidad: española"),
    ];

    /// Issue 527: TenderNed's award marks, read only as a closing bracket group whose fold is exactly a
    /// mark.
    #[test]
    fn award_annotation_reads_tenderneds_marks_only() {
        for (name, base, not_awarded) in [
            ("Humble Inpect B.V. (niet gegunde partij)", "Humble Inpect B.V.", true),
            ("Provadi B.V. (niet-gegunde partij) ", "Provadi B.V.", true),
            ("PVM Eindhoven B.V. (NIET GEGUNDE PARTIJ)", "PVM Eindhoven B.V.", true),
            ("Planon Building Management B.V. (gegunde partij)", "Planon Building Management B.V.", false),
        ] {
            assert_eq!(super::award_annotation(name), Some((base, not_awarded)), "{name}");
        }
        // The classifiers read the name without its mark: a losing tenderer is no void lot (24400562's
        // `Facilicom Solutions`, which 510's Dutch stem `niet gegun` had dropped), and no void net catches it.
        for name in ["Facilicom Solutions (niet gegunde partij)", "Humble Inpect B.V. (niet gegunde partij)", "Planon (gegunde partij)"] {
            assert_eq!(not_a_name(name), None, "{name}");
            assert!(!super::mentions_void(name), "{name}");
        }
        assert_eq!(not_a_name("Niet gegund"), Some(NotAName::VoidLot), "the bare verdict still reads void");
        for name in [
            "Gegunde Partijen Holding",
            "Dupont B.V. (gegund)",
            "Dupont B.V. (niet gegund)",
            "Dupont B.V. (niet gegunde partij) Utrecht",
            "(niet gegunde partij)",
            "Dupont B.V.",
            "Dupont B.V. (Utrecht)",
            "Foo (Bar (niet gegunde partij))",
            "- (gegunde partij)",
        ] {
            assert_eq!(super::award_annotation(name), None, "{name}");
        }
    }

    /// Issue 520 U6 (A2, unit-1 decision §4.2; panels wf_8883fc55-316 and wf_421af829-0da): the five
    /// reviewed rows name their winner, read whole on the fold (a `/` for a ` - ` reads the same), and
    /// nothing else does — not A2's other rows (4836476's `;` shape is withdrawn), not the same words with
    /// one changed (`la part` for `les parts`), not a B2 list around BGE, and none of the shapes the panels
    /// broke (a `;` tail, an address before a lot title, a buyer or two parties, a consortium with a buyer
    /// or a failed bidder in it). Over the 402 names of the void walk's dry reports 2351–2375 it answers
    /// exactly the five.
    #[test]
    fn void_tail_winner_cuts_the_reliable_a2_tails() {
        for (org, name, winner) in A2_WINNERS {
            assert_eq!(super::void_tail_winner(name).as_deref(), Some(winner), "{org}: {name}");
            assert_eq!(not_a_name(name), Some(NotAName::AwardSummary), "{org}: the fold keeps it as a summary");
            assert!(!super::may_name_a_winner(name), "{org}: no award_winner cue reads it");
        }
        for (org, name) in A2_UNRELIABLE {
            assert_eq!(super::void_tail_winner(name), None, "{org}: {name}");
        }
        for (name, winner) in [
            ("BGE Guyane 16 rue Lt Becker 97300 Cayenne - La consultation a été déclarée sans suite pour les parts non couvertes des besoins", "BGE Guyane"),
            ("GRPT GCC (mandataire) et Andre BTP - suite à procédure négociée après AO infructueux.", "GRPT GCC (mandataire) et Andre BTP"),
        ] {
            assert_eq!(super::void_tail_winner(name).as_deref(), Some(winner), "{name}");
        }
        for name in [
            "BGE Guyane 16 rue Lt Becker 97300 Cayenne / La consultation a été déclarée sans suite pour la part non couverte des besoins",
            "BGE Guyane 16 rue Lt Becker 97300 Cayenne / Equinoxe Formation Rocade Zéphir Centre Commercial Katoury 97300 Cayenne / La consultation a été déclarée sans suite pour la part non couverte des besoins",
            "BGE Guyane 16 rue Lt Becker 97300 Cayenne Point Com / La consultation a été déclarée sans suite pour les parts non couvertes des besoins",
            "Equinoxe Formation BGE Guyane 16 rue Lt Becker 97300 Cayenne / La consultation a été déclarée sans suite pour les parts non couvertes des besoins",
            "Restauration scolaire 16 rue Lt Becker 97300 Cayenne / La consultation a été déclarée sans suite pour les parts non couvertes des besoins",
            "La Mairie de Cayenne 1 place Léopold Héder 97300 Cayenne / la consultation a été déclarée sans suite pour la part non couverte des besoins",
            "Dupont SARL 3 avenue Foch 69006 Lyon / la consultation a été déclarée sans suite pour la part non couverte des besoins",
            "GRPT GCC (mandataire) et Andre BTP, Martin SA, suite à procédure négociée après AO infructueux",
            "GRPT GCC (mandataire) et Andre BTP, suite à procédure négociée sans offre après AO infructueux",
            "Groupement de commandes Ville de Cayenne (mandataire) et CACL, suite à procédure négociée après AO infructueux",
            "GRPT Dupont (mandataire) et Martin, Durand SA non conforme, suite à procédure négociée après AO infructueux",
            "Dupont SARL ; infructueux",
            "Plomberie sanitaire ; infructueux",
            "GRPT GCC (mandataire) et Andre BTP",
            "BGE Guyane",
        ] {
            assert_eq!(super::void_tail_winner(name), None, "{name}");
        }
    }

    /// Issue 520 U4, the adversarial review: where the name ends and what it drops. An adverbial `à`
    /// (`conformément à l'article 35`, `suite à`, `à titre provisoire`, `à l'unanimité`) is no party's,
    /// so the cue reads on to the next one; the name ends at a line break, a field's colon, `pour le lot`,
    /// `en date du`, `après négociation`, `par marché négocié`, a cue e's own terminators, any whitespace
    /// before a terminator (a tab, a no-break space), and after a dotted legal form or an initial when the
    /// next field opens (`Dupont S.A. Montant`, the dot kept) — not after a company abbreviation's dot
    /// (`Ets. Dupont`) nor inside a name (`Association pour la Formation`, `Peinture par Excellence`). A
    /// descriptor the legal name goes on from stays (`la Société Générale`, `la Société des Eaux de
    /// Marseille`), an upper-case or acute-accent article goes, and quotes inside the name go in pairs.
    #[test]
    fn award_winner_ends_and_strips_the_name_as_published() {
        for (name, winner) in [
            // An adverbial `à`.
            ("Lot infructueux, attribué, conformément à l'article 35, à la société Dupont", "Dupont"),
            ("Lot infructueux, attribué suite à une procédure négociée à la société Dupont", "Dupont"),
            ("Lot infructueux, attribué grâce à une négociation à la société Dupont", "Dupont"),
            ("Lot infructueux, attribué par recours à une procédure négociée à la société Dupont", "Dupont"),
            ("Lot infructueux, attribué jusqu'à la fin de l'année à la société Dupont", "Dupont"),
            ("Lot infructueux, attribué à titre provisoire à la société Dupont", "Dupont"),
            ("Lot infructueux, attribué à titre définitif à la société Dupont", "Dupont"),
            ("Lot infructueux, attribué à nouveau à la société Dupont", "Dupont"),
            ("Lot infructueux, attribué à l'unanimité à la société Dupont", "Dupont"),
            ("Lot infructueux, attribué à la majorité à la société Dupont", "Dupont"),
            ("Lot infructueux, attribué à l'occasion de la relance à la société Dupont", "Dupont"),
            ("Lot infructueux, attribué à défaut d'autre offre à la société Dupont", "Dupont"),
            ("Lot infructueux puis attribué à titre provisoire à Dupont", "Dupont"),
            // Line breaks, colons, figures, dates, procedures.
            ("Lot infructueux\nAttributaire : Dupont SARL\nAdresse : rue de la Paix Lyon", "Dupont SARL"),
            ("Lot infructueux puis attribué à Dupont SARL\nMontant : 12 000 EUR HT", "Dupont SARL"),
            ("Lot infructueux.\nAttributaire : Dupont\r\nAdresse : 1 rue de la Paix", "Dupont"),
            ("Lot infructueux puis attribué à la société Dupont : 12 000 EUR HT", "Dupont"),
            ("Lot infructueux puis attribué à la société Dupont en date du 12/05/2014", "Dupont"),
            ("Lot infructueux faute d'offre, attribué à Dupont Frères après négociation", "Dupont Frères"),
            ("Infructueux : aucune offre, attribué à Promocash par marché négocié", "Promocash"),
            ("Lot infructueux puis attribué à l'Association pour la Formation des Adultes", "Association pour la Formation des Adultes"),
            ("Lot infructueux puis attribué à Peinture par Excellence", "Peinture par Excellence"),
            // Any whitespace before a terminator.
            ("Lot déclaré infructueux puis attribué à la Société Dupont\nsuite à procédure négociée", "Dupont"),
            ("Lot infructueux puis attribué à Dupont\tsuite à procédure négociée", "Dupont"),
            ("Lot infructueux puis attribué à la société Dupont\u{a0}suite à procédure négociée", "Dupont"),
            ("Lot infructueux puis attribué à Dupont\u{a0}pour un montant de 5000 EUR", "Dupont"),
            ("Lot infructueux puis attribué à Dupont\u{a0}(69003 Lyon)", "Dupont"),
            // Cue e's terminators.
            ("Lot 3) infructueux. La société Dupont (69003 Lyon) a été retenue", "Dupont"),
            ("Lot infructueux. La société Dupont - 69003 Lyon a été retenue", "Dupont"),
            ("Lot infructueux. Après négociation, la société Dupont, mieux-disante, a été retenue", "Dupont"),
            ("Lot infructueux. Société Dupont a été retenue", "Dupont"),
            // Dotted legal forms, initials and company abbreviations.
            ("Lot infructueux puis attribué à Dupont S.A. Montant HT : 48 225,34 EUR", "Dupont S.A."),
            ("Lot infructueux puis attribué à Dupont S.A.R.L. Montant du marché : 12000 EUR HT", "Dupont S.A.R.L."),
            ("Lot infructueux. Attributaire : Dupont S.A.S. Date d'attribution : 24.4.2012", "Dupont S.A.S."),
            ("Lot infructueux puis attribué à Martin J. Le marché a été notifié le 6 janvier", "Martin J."),
            ("Lot infructueux puis attribué à Dupont S.A.", "Dupont S.A."),
            ("Lot infructueux puis attribué à la société Dupont S.A. pour un montant de 5000 EUR", "Dupont S.A."),
            ("Lot infructueux puis attribué à Ets. Dupont et Fils", "Ets. Dupont et Fils"),
            ("Lot infructueux puis attribué à la Sté. Dupont", "Dupont"),
            ("Lot infructueux puis attribué à Cie. Générale des Eaux", "Cie. Générale des Eaux"),
            ("Lot infructueux puis attribué à Assoc. Léo Lagrange", "Assoc. Léo Lagrange"),
            ("Lot infructueux puis attribué à St. Gobain", "St. Gobain"),
            // A descriptor of the legal name stays; one before the name goes, in any case and spelling.
            ("Lot infructueux, relancé et attribué à la Société Générale", "Société Générale"),
            ("Lot infructueux, relancé et attribué à la Société des Eaux de Marseille", "Société des Eaux de Marseille"),
            ("Lot infructueux, relancé et attribué à la Société Française de Restauration", "Société Française de Restauration"),
            ("Lot infructueux, relancé et attribué à la Société d'Exploitation des Transports", "Société d'Exploitation des Transports"),
            ("Lot infructueux, relancé et attribué à l'Entreprise Générale de Bâtiment", "Entreprise Générale de Bâtiment"),
            ("Lot infructueux, relancé et attribué à la STE Nouvelle Dupont", "STE Nouvelle Dupont"),
            ("Lot infructueux, relancé en procédure négociée avec la Société Générale", "Société Générale"),
            ("Lot infructueux. Attributaire : la Société Générale", "Société Générale"),
            ("Lot infructueux. La Société Générale a été retenue", "Société Générale"),
            ("Lot infructueux puis attribué à la société De Dietrich", "De Dietrich"),
            ("LOT INFRUCTUEUX PUIS ATTRIBUÉ À LA SOCIÉTÉ DUPONT", "DUPONT"),
            ("LOT INFRUCTUEUX, RELANCÉ EN PROCÉDURE NÉGOCIÉE AVEC LA SOCIÉTÉ DUPONT", "DUPONT"),
            ("Lot infructueux puis attribué à l´entreprise Dupont", "Dupont"),
            // Quotes inside the name go in pairs.
            ("Lot infructueux puis attribué à la société \"Les Jardins\" SARL", "Les Jardins SARL"),
            ("Lot infructueux puis attribué à « Les Jardins » de Cocagne", "Les Jardins de Cocagne"),
        ] {
            assert_eq!(super::award_winner(name).as_deref(), Some(winner), "{name}");
        }
        // Past the cue's reach once the adverbial `à` is skipped: nobody, never `article 35-II-3° du CMP`.
        assert_eq!(super::award_winner("Lot infructueux, attribué conformément à l'article 35-II-3° du CMP à la société Dupont"), None);
    }

    /// Issue 520 U4, the adversarial review: the extractor reads a long text once — each cue's scan is
    /// bounded and each word's clause, sentence and doubts are read up front. Before, 1,000 award clauses
    /// (32 KB) took 4.2 s and 128 KB 66 s in the test profile; 128 KB now takes well under a second.
    #[test]
    fn award_winner_reads_a_long_text_once() {
        for (unit, winner) in [
            ("Le lot est attribué à Dupont. ", Some("Dupont")),
            ("relancé en procédure négociée avec la société Dupont, pour un montant de 5 EUR ", None),
            ("attribué à Dupont et ", None),
        ] {
            let text = format!("Lot infructueux. {}", unit.repeat(4_000));
            let started = std::time::Instant::now();
            assert_eq!(super::award_winner(&text).as_deref(), winner, "{unit}");
            assert!(started.elapsed() < std::time::Duration::from_secs(5), "{unit}: {:?}", started.elapsed());
        }
    }

    /// Issue 520 U4, the second adversarial review: each reviewer's specimen, with the verdict the fix
    /// gives it — a wrong extraction now names nobody or the true awardee, and the counterparts that state
    /// one awardee still name it. Every fix only refuses or ends a name earlier, but for the leading dotted
    /// legal form, which the build cut down to the form alone (`la S.A.R.L. Dupont` → `S.A.R.L.`, one
    /// organization every such notice would share) and which now keeps the name it opens.
    #[test]
    fn award_winner_reads_the_second_review() {
        for (name, winner) in [
            // A dotted legal form before the name is the name's own (`la S.A.R.L. Dupont`), never the winner
            // alone; a form alone, or one the next field follows, names nobody.
            ("Lot infructueux puis attribué à la S.A.R.L. Dupont", Some("S.A.R.L. Dupont")),
            ("Lot déclaré infructueux puis attribué à la S.A.R.L. Les Jardins de Cocagne", Some("S.A.R.L. Les Jardins de Cocagne")),
            ("Lot infructueux. Attributaire : S.A.S. Dupont Frères", Some("S.A.S. Dupont Frères")),
            ("Titulaire : S.A. Martin et Fils", Some("S.A. Martin et Fils")),
            ("La société S.A.R.L. Dupont a été retenue", Some("S.A.R.L. Dupont")),
            ("Lot infructueux puis attribué à E.T.S. Dupont", Some("E.T.S. Dupont")),
            ("Lot infructueux puis attribué à la société S.A. Les Rapides Varois", Some("S.A. Les Rapides Varois")),
            // The third check: a joint award after a figure names no one winner, and the stops after `pour`
            // and `par` take only a figure with its unit and an agent from the closed list.
            ("Lot infructueux puis attribué à la société Dupont pour 12 000 € HT et à la société Martin pour 8 000 € HT", None),
            ("Infructueux. Lot relancé, attribué à Durand BTP pour 12 000 € HT et à Martin SA pour 8 000 € HT", None),
            ("Lot infructueux puis attribué à la société Dupont pour 12 000 € HT ainsi qu'à la société Martin", None),
            ("Lot infructueux puis attribué à la société Dupont pour 3 ans et a été notifié le 2 mai", Some("Dupont")),
            ("Lot infructueux. Attributaire : Insertion par l'emploi", Some("Insertion par l'emploi")),
            ("Lot infructueux. Attributaire : Réussir par le sport", Some("Réussir par le sport")),
            ("Lot infructueux puis attribué à Solidarité par le travail", Some("Solidarité par le travail")),
            ("Lot infructueux. Attributaire : Association pour 1 avenir", Some("Association pour 1 avenir")),
            ("Lot infructueux puis attribué à la S.A.R.L.", None),
            ("Lot infructueux. Attributaire : S.A.S.", None),
            ("Lot infructueux. Attributaire : SARL", None),
            ("Lot infructueux puis attribué à Dupont S.A. Montant HT : 48 225,34 EUR", Some("Dupont S.A.")),
            ("Lot infructueux puis attribué à la S.A.R.L. Le marché a été notifié le 6 janvier", None),
            ("Lot infructueux. Attributaire : S.A. Montant HT : 48 225,34 EUR", None),
            // Cue e names no company another clause or sentence holds, nor a complement's; an apposition may
            // still stand between the name and `a été retenue`.
            ("Lot infructueux. Suite au désistement de la société Dupont, Martin SA a été retenue.", None),
            ("Lot infructueux. Après l'exclusion de la société Dupont, l'offre de Martin SA a été retenue.", None),
            ("Lot infructueux. L'offre de la société Dupont ayant été écartée, Martin SA a été retenue.", None),
            ("Lot infructueux. Offre de la société Dupont irrégulière. Martin a été retenu.", None),
            ("Lot infructueux. La société Dupont s'étant désistée, Martin SA a été retenue.", None),
            ("Lot infructueux. Le recours de la société Dupont a été retenu", None),
            ("Lot infructueux. la société Dupont SARL, Lyon, a été retenue", Some("Dupont SARL")),
            ("Lot infructueux. La société Dupont - 69003 Lyon a été retenue", Some("Dupont")),
            // A procedure, a date, a figure, an agent or an address after the name: refused, or the name ends
            // before it.
            ("Lot déclaré infructueux puis attribué à la société Dupont dans le cadre d'une procédure négociée", None),
            ("Lot déclaré infructueux puis attribué à la société Dupont pour 48 225,00 € HT", Some("Dupont")),
            ("Lot déclaré infructueux puis attribué à la société Dupont en procédure négociée sans publicité", None),
            ("Lot déclaré infructueux puis attribué à la société Dupont selon la procédure négociée", None),
            ("Lot déclaré infructueux puis attribué à la société Dupont au terme d'une procédure négociée", None),
            ("Lot déclaré infructueux puis attribué à la société Dupont lors de la CAO du 12 mars 2013", None),
            ("Lot déclaré infructueux puis attribué à la société Dupont sous réserve de la production des pièces", None),
            ("Lot déclaré infructueux puis attribué à la société Dupont au prix de 48 225 € HT", None),
            ("Lot déclaré infructueux puis attribué à la société Dupont d'un montant de 48 225 € HT", None),
            ("Lot infructueux. Attribué à DUPONT SAS montant HT 12 000 €", None),
            ("Lot déclaré infructueux puis attribué à la société Dupont pour 3 ans", Some("Dupont")),
            ("Lot déclaré infructueux puis attribué à la société Dupont et notifié le 12 mars 2013", None),
            ("Lot déclaré infructueux puis attribué à la société Dupont par le pouvoir adjudicateur", Some("Dupont")),
            ("Lot infructueux puis attribué à SARL Dupont 12 rue des Lilas 69003 Lyon", None),
            ("LOT DÉCLARÉ INFRUCTUEUX PUIS ATTRIBUÉ À LA SOCIÉTÉ DUPONT DANS LE CADRE D'UNE PROCÉDURE NÉGOCIÉE", None),
            ("Lot infructueux puis attribué à Peinture par la Couleur", Some("Peinture par la Couleur")),
            // An award undone after the name, in its sentence or a later one, or a pluperfect: nobody. Another
            // numbered lot's void verdict is not this award's.
            ("Le lot a été attribué à la société Dupont. Celle-ci n'ayant pas produit les certificats, le lot est déclaré sans suite.", None),
            ("Lot infructueux. La société Dupont a été retenue mais n'a pas produit les attestations fiscales et sociales ; le lot est déclaré infructueux", None),
            ("Lot attribué à la société Dupont, qui n'a pas produit les pièces demandées. Le lot est déclaré infructueux.", None),
            ("Procédure déclarée sans suite pour motif d'intérêt général. Le marché avait été attribué à la société Dupont.", None),
            ("Le marché avait été attribué à la société Dupont ; celle-ci s'étant désistée, le lot est déclaré infructueux", None),
            ("La CAO avait décidé d'attribuer ce lot à la société Dupont ; celle-ci n'ayant pas produit ses attestations, le lot est déclaré infructueux", None),
            ("Lot infructueux. Lot 3 attribué à la société Dupont. Le lot 3 a été déclaré sans suite.", None),
            ("Lot infructueux. Lot attribué à la société Dupont, puis déclaré infructueux", None),
            ("Lot infructueux. Suite au désistement de la société Martin, le lot a été attribué à la société Dupont", None),
            ("Infructueux (1 offre irrégulière), relancé et attribué à l'entreprise Promocash", None),
            ("lot 25) la société Storz a été retenue. Le lot 32 a été déclaré infructueux", Some("Storz")),
            ("Lot infructueux, relancé et attribué à la Société Dupont SARL ; lot 2 infructueux", Some("Dupont SARL")),
            // Another contract's, the incumbent's, a framework's or a scored bidder's award: nobody.
            // `intégrées au lot 2` names the result's winner (the 2381362 rule, App. B).
            ("Lot déclaré sans suite : prestations déjà attribuées à la société Dupont (marché n° 2012-15)", None),
            ("Lot déclaré sans suite, ces prestations ayant déjà été attribuées à la société Dupont, titulaire du marché de maintenance", None),
            ("Lot infructueux. Le marché actuel, attribué à la société Dupont, est prolongé par avenant", None),
            ("Lot déclaré sans suite, le besoin étant couvert par l'accord-cadre attribué à l'UGAP", None),
            ("Lot déclaré infructueux, la note attribuée à la société Dupont, inférieure au seuil, ne permettant pas de retenir son offre", None),
            ("Lot initialement déclaré infructueux, il a depuis déjà été attribué à l'entreprise Dupont Frères après négociation", None),
            ("Lot 3 déclaré sans suite : prestations intégrées au lot 2 attribué à la société Dupont", Some("Dupont")),
            // A selection-stage `retenue`: nobody; `pour le lot 2`, `pour un montant` still name the awardee.
            ("Lot infructueux. Relancé en procédure négociée, la société Dupont a été retenue pour la négociation", None),
            ("Lot infructueux. Relancé en procédure négociée, la société Dupont a été retenue pour participer à la négociation", None),
            ("Lot infructueux. Relancé en procédure négociée, la société Dupont a été retenue pour la phase de négociation", None),
            ("Lot infructueux. La candidature de la société Dupont a été retenue", None),
            ("Lot infructueux. Seule la candidature de la société Dupont a été retenue, son offre étant inacceptable", None),
            ("Lot infructueux. Seule la société Dupont a été retenue pour présenter une offre ; aucune offre n'a été remise", None),
            ("Lot infructueux. L'offre de la société Dupont a été retenue mais jugée inacceptable", None),
            ("Lot infructueux. La société Dupont a été retenue et admise à négocier", None),
            ("Lot infructueux. La société Dupont a été retenue pour le lot 2", Some("Dupont")),
            ("Lot infructueux. La société Dupont a été retenue pour un montant de 5 000 EUR HT", Some("Dupont")),
            // A qualifier, a role or a possessive where the name would stand: nobody.
            ("Lot infructueux puis attribué à l'entreprise retenue après négociation", None),
            ("Lot infructueux puis attribué à l'entreprise classée première", None),
            ("Lot infructueux puis attribué à la société classée première", None),
            ("Lot infructueux puis attribué à la société sortante", None),
            ("Lot infructueux puis attribué à l'entreprise pressentie", None),
            ("Lot infructueux puis attribué à la société adjudicataire", None),
            ("Lot infructueux puis attribué à l'entreprise lauréate", None),
            ("Lot infructueux puis attribué à l'entreprise consultée", None),
            ("Lot infructueux puis attribué à l'entreprise locale", None),
            ("Lot infructueux puis attribué à l'actuel prestataire", None),
            ("Lot infructueux puis attribué à l'opérateur économique classé premier", None),
            ("Lot infructueux, prestations attribuées à son titulaire actuel", None),
            ("Lot infructueux puis attribué à sa régie", None),
            ("Lot infructueux. La société classée première a été retenue", None),
            ("Lot infructueux puis attribué à l'entreprise économiquement la plus avantageuse", None),
            ("Lot infructueux puis attribué à la Société Dupont", Some("Dupont")),
            ("Lot infructueux puis attribué à la société des Eaux de Marseille", Some("société des Eaux de Marseille")),
            // A status in an awardee label: nobody.
            ("Lot infructueux. Attributaire : A définir", None),
            ("Lot infructueux. Attributaire : A désigner", None),
            ("Lot infructueux. Attributaire : A venir", None),
            ("LOT INFRUCTUEUX. ATTRIBUTAIRE : A DEFINIR", None),
            ("Lot infructueux. Attributaire : négociation en cours", None),
            ("Lot infructueux. Attributaire : consultation en cours", None),
            ("Lot infructueux. Titulaire : relance en cours", None),
            ("Lot infructueux. Attributaire : Relancé en procédure négociée", None),
            ("Lot infructueux. Attributaire : Sera désigné ultérieurement", None),
            ("Lot infructueux. Attributaire : Indéterminé", None),
            ("Lot infructueux. Attributaire : Le même", None),
            ("Lot infructueux. Attributaire : La même entreprise", None),
            ("Lot infructueux. Attributaire : Le même que pour le lot 2", None),
            ("Lot infructueux. Attributaire : LA SOCIÉTÉ", None),
            ("Lot infructueux. Attributaire : La Poste", Some("La Poste")),
            // Co-winners (B2, decided keep): nobody; an apposition or a name's own `et` still names the awardee.
            ("Lot déclaré infructueux puis attribué à Dupont SARL, Martin SA et Durand SAS (accord-cadre multi-attributaires)", None),
            ("Lot infructueux. Attributaire : Dupont SARL - Martin SA - Durand SAS", None),
            ("Lot infructueux. Attributaire : groupement Dupont SARL, Martin SA", None),
            ("Lot infructueux puis attribué conjointement à Dupont SARL et Martin SA", None),
            ("Lot infructueux puis attribué à la société Dupont et la société Martin", None),
            ("Lot infructueux puis attribué à la société Dupont ainsi qu'à la société Martin", None),
            ("Lot infructueux puis attribué à la société Dupont en groupement avec la société Martin", None),
            ("Lot infructueux, relancé en procédure négociée avec la société Dupont et la société Martin", None),
            ("Lot infructueux puis attribué à Dupont SARL et Martin SA", None),
            ("Lot infructueux. Attributaire : Dupont SARL, Martin SA", None),
            ("Lot infructueux puis attribué à Dupont SARL, Lyon", Some("Dupont SARL")),
            ("Lot infructueux puis attribué à Ets. Dupont et Fils", Some("Ets. Dupont et Fils")),
            // Cue d's name that swallowed a failure tail: nobody.
            ("Lot infructueux, relancé en procédure négociée avec la société Dupont sans succès", None),
            ("Lot infructueux, relancé en procédure négociée avec la société Dupont mais sans résultat", None),
            ("Lot infructueux, relancé en procédure négociée avec la société Dupont en vain", None),
            ("Infructueux, relancé en procédure négociée avec la société Martin Frères.", Some("Martin Frères")),
            // The `à été` typo for the verb is no party's `à`, and the cue reads no further.
            ("Lot déclaré infructueux puis attribué après négociation, le marché à été notifié le 12 mai 2014", None),
            ("Lot déclaré infructueux. Lot attribué après négociation, le contrat à été signé", None),
            ("Lot déclaré infructueux puis attribué, le marché à été signé avec la société Dupont", None),
            ("Lot déclaré infructueux puis attribué, le marché à été notifié à la société Dupont", None),
        ] {
            assert_eq!(super::award_winner(name).as_deref(), winner, "{name}");
        }
    }

    /// TED's "not awarded" in every language one r208 winner slot carried (notice 17674598):
    /// each must be void, or the party keeps its organization.
    #[test]
    fn teds_not_awarded_is_void_in_every_language() {
        for void in [
            "Not awarded", "Ikke tildelt", "nicht vergeben", "No ha sido adjudicado", "sopimusta ei tehty",
            "Non attribué", "Δεν ανατέθηκε", "non aggiudicato", "Niet gegund", "Não adjudicado", "Ej tilldelat",
            "Nebyla zadána", "lepingut ei sõlmitud", "Nem ítélték oda", "Nesudaryta", "Nav piešķirts", "Ma ngħatax",
            "Zamówienia nie udzielono", "Nebola pridelená.", "Naročilo ni bilo oddano", "Gan dámhachtain",
            "Не е възложена", "Nu a fost atribuit", "Ugovor nije sklopljen",
        ] {
            assert_eq!(not_a_name(void), Some(NotAName::VoidLot), "{void}");
        }
    }

    /// Issue 525 (classes A and C): TED's OTHER winner-slot templates for a contract or lot not
    /// awarded — one specimen per template and language family here; the projection's drop tests
    /// pin every copy of each template with its notice (`TED_*` beside
    /// `teds_other_not_awarded_templates_drop_their_party`). And the text era's all-caps `NO AWARD
    /// WAS MADE` (3322691 ORG-2), whose uppercase `WAS` the prefix rule reads as a company's next
    /// word: a whole value, so case never decides it.
    #[test]
    fn teds_other_not_awarded_templates_are_void() {
        for void in [
            "NO AWARD WAS MADE",
            "No award was made",
            "No award was made.",
            "Zuschlag nicht vergeben",
            "Der Auftrag wurde nicht vergeben.",
            "Kontrakten ikke tildelt.",
            "El contrato no ha sido adjudicado.",
            "Appalto non aggiudicato.",
            "Contract neatribuit.",
            "Es wurde kein Auftrag vergeben",
            "Le marché n'a pas été attribué",
            "Aucun marché n'a été attribué.",
            "Ingen kontrakt tildelt.",
            "Partiet blev ikke tildelt",
            "Los wurde nicht vergeben",
            "Lote no adjudicado",
            "LOTE NO ADJUDICADO",
            "Lot neatribuit",
            "This lot was unsuccessful.",
            "Dieses Los blieb erfolglos.",
            "entfällt — Los nicht vergeben",
            "No procede: lote no adjudicado",
            "Поръчката не беше възложена.",
            "Η παρτίδα δεν ανατέθηκε",
            "Il-lott ma ngħatax",
            "Nav piešķirta.",
            // The review: the other kept parties of dry 2351 (diagnosis 2's copies).
            "Nichtvergabe des Auftrags",
            "non-award of the contract",
            "entfällt: Der Auftrag wurde nicht vergeben.",
            "ikke relevant: Kontrakten blev ikke tildelt.",
            "Non applicabile: appalto non aggiudicato.",
            "entfällt — Auftrag nicht vergeben",
            "finder ikke anvendelse — kontrakten blev ikke tildelt",
            "neaplicabil – contract neatribuit",
            "Es ging kein Angebot ein",
            "no tenders were received",
            "Verfahren erfolglos",
            "unsuccessful procedure",
            "Procedura infruttuosa",
            // A bare lot qualifier comes off, as from every whole value.
            "Lote no adjudicado (lote 3)",
            // TED's own pointer after a template (12381194's ORG-4, every language).
            "Los wurde nicht vergeben (siehe Abschnitt VI Ziffer VI.2 „Sonstige Informationen“).",
            "Delen har inte tilldelats (se avsnitt VI, punkt VI.2 Kompletterande information).",
            "Lot neatribuit (a se vedea secţiunea VI, punctul VI.2 „Informaţii suplimentare”).",
        ] {
            assert_eq!(not_a_name(void), Some(NotAName::VoidLot), "{void}");
            assert!(mentions_void(void), "510's net holds {void}");
        }
    }

    /// Issue 525: the real names beside the new templates stay names. A template is the entire
    /// name ([`VOID_TEMPLATES`]), so a company that opens with one and goes on — a capitalised
    /// word, a legal form, an award in a language the award words do not hold — is no void lot,
    /// and a name that merely shares a word with a template (`Award`, `LOT`, `Partiet`, `Sklop`)
    /// is nowhere near one.
    #[test]
    fn real_names_beside_teds_templates_are_names() {
        for name in [
            "Nicht Vergeben GmbH",
            "Zuschlag Nicht Vergeben GmbH",
            "NO AWARD WAS MADE LTD",
            "Award Ltd",
            "Awarded Contracts Ltd",
            "LOT Polish Airlines",
            "Polskie Linie Lotnicze LOT S.A.",
            "Lote Uno SL",
            "Lote No Adjudicado SL",
            "Contract Neatribuit SRL",
            "Sklop d.o.o.",
            "Partiet Ingeniører ApS",
            "Dalis UAB",
            "Unsuccessful Ventures Ltd",
            // The review: a template is EXACT ([`VOID_TEMPLATES`]) — neither a reason head nor a
            // prefix, since the award vocabulary that keeps a summary a name is French, Spanish,
            // Italian, English and Dutch only, and a legal form outside `COMPANY_TOKENS` is none.
            "Kontrakten ikke tildelt, men givet til Hansen Byg",
            "Zakázka nebyla zadána, vybrán dodavatel Novák stavby",
            "Zamówienie nie zostało udzielone w części 2, część 1 udzielono Firmie Handlowej Jan Kowalski",
            "Los wurde nicht vergeben, Zuschlag an Müller Bau AG",
            "Appalto non aggiudicato in via definitiva, aggiudicazione provvisoria a Rossi Costruzioni",
            "Lot neatribuit, lotul 2 atribuit catre Alfa Construct",
            "Verfahren erfolglos, neu vergeben an Schmidt Bau",
            "Sklop ni bil oddan d.o.o.",
            "Los wurde nicht vergeben e.K.",
            "Lote no adjudicado por falta de ofertas",
            // A qualifier that says more than which lot, and a pointer group that names a company.
            "Lote no adjudicado (lote 3 adjudicado a Pérez SL)",
            // Kept by its legal form (`translation_shaped`), not by the name after the reference.
            "Los wurde nicht vergeben (siehe Müller Bau AG, VI.2)",
            // The second review: a pointer is ONE group closing the name, so a winner named between
            // two groups is never read past.
            "Los wurde nicht vergeben (siehe VI.2). Los 2: Müller Bau (siehe VI.3)",
            "Delen har inte tilldelats (se avsnitt VI.2). Del 2 tilldelad Svensson Bygg (se VI.3)",
        ] {
            assert_eq!(not_a_name(name), None, "{name}");
        }
        // The second review: a numbered list whose second entry is a template with a qualifier that
        // names a winner keeps its award (the entry is not void), as it did before issue 525.
        for summary in [
            "1) nessuna aggiudicazione - 2) procedura infruttuosa (lotto 3 Rossi Costruzioni)",
            "1) infructueux - 2) no tenders were received (lot 3 Dupont et Fils)",
        ] {
            assert_ne!(not_a_name(summary), Some(NotAName::VoidLot), "{summary}");
        }
    }

    #[test]
    fn placeholders_are_their_own_class() {
        for placeholder in [
            "See section VI.3 for the list of awarded suppliers",
            "Véase perfil del contratante",
            "Ver información adicional",
            "Voir autres informations",
            "Disclosure would prejudice commercial interests",
            "Various",
            "VARIOUS",
        ] {
            assert_eq!(not_a_name(placeholder), Some(NotAName::Placeholder), "{placeholder}");
        }
    }

    /// The real companies beside the phrases, read off prod on 2026-10-10.
    #[test]
    fn real_names_beside_the_phrases_are_names() {
        for name in [
            "Desierto OÜ",
            "DESIERTO, S.L.",
            "Desierto Florido SL",
            "DESERTOT",
            "Desertot SARL",
            "VÁRIOS MUNDOS UNIP. LDA.",
            "Variosun GmbH",
            "Diverse Care Services Ltd",
            "Diversey",
            "NIL d.o.o.",
            "Nilfisk",
            "N/A, s.r.o.",
            "SEE LAUER",
            "See & Go s.r.o.",
            "CF Cefarm SA",
            "Brake",
            "Vedise Hospital SpA",
            "Void arhitektura d.o.o.",
            "VOID SISTEMAS S.L.",
            "Unknown Architects",
            "Confidential Waste Services",
            "Multiplex Techniques Ltd",
            "Vacant Vårdbemanning AB",
            "InGen",
            "Gestiver Información SL",
            "Server Información Tecnológica",
            "Les Artisans Suite",
            "Tennessee Section",
            "Cannon Attributes",
            "Abandon Records Ltd",
            "Aufgehoben Bau GmbH",
            "ACME (lot 3)",
            "Acme Ltd",
            // Issue 511's traps: whole values only, the deferred words, digit-only names.
            "NIL",
            "N/A s. r. o.",
            "Bank of America, N.A.",
            "X Corp",
            "VARI, a.s.",
            "EI",
            "SIN",
            "365",
            "24/7",
        ] {
            assert_eq!(not_a_name(name), None, "{name}");
        }
    }

    #[test]
    fn the_lot_qualifier_is_stripped_only_for_the_whole_compare() {
        assert_eq!(not_a_name("desierto (lote 3)"), Some(NotAName::VoidLot));
        assert_eq!(not_a_name("Desierto (lot 9/10)."), Some(NotAName::VoidLot));
        assert_eq!(not_a_name("ACME (lot 3)"), None);
        assert_eq!(not_a_name("Desierto (no ofertas)"), Some(NotAName::VoidLot), "a void head and its reason");
        assert_eq!(not_a_name("Desierto, S.L."), None, "a company form: a company named like the phrase");
        assert_eq!(not_a_name("Desierto OÜ"), None, "no break: a company's name");
        assert_eq!(not_a_name(""), None);
        assert_eq!(not_a_name("  ...  "), Some(NotAName::Blank), "issue 511: punctuation only is a blank slot");
    }

    /// Issue 511 (the unit-2 review): its whole values come off a BARE lot qualifier only;
    /// one that names who won another lot is a summary the fold keeps. Any qualifier is more
    /// text, so the dry plan reads every qualified hit (`is_exact_entry` is the full fold).
    #[test]
    fn a_lot_qualifier_that_names_a_winner_is_an_award_summary() {
        use NotAName::{AwardSummary, Blank, Placeholder};
        for summary in
            ["N/A (lot 2 attribué à Dupont SARL)", "Néant (lot 3 : Société Martin SAS)", "Various (Lot 1: ACME Ltd, Lot 2: Beta Ltd)"]
        {
            assert_eq!(not_a_name(summary), Some(AwardSummary), "{summary}");
            assert!(mentions_placeholder(summary), "the net still lists {summary}");
            assert!(!is_exact_entry(summary), "{summary}");
        }
        for (bare, class) in [
            ("N/A (lot 2)", Blank),
            ("Néant (lots 2 et 4)", Blank),
            ("Various (lotes VIII y IX)", Placeholder),
            ("TBC (lot n° 3)", Placeholder),
        ] {
            assert_eq!(not_a_name(bare), Some(class), "{bare}");
            assert!(!is_exact_entry(bare), "{bare}: a qualifier is more text, read before the wet run (511 §8.2)");
        }
        assert_eq!(not_a_name("Néant (lot 3 sans attributaire)"), None, "more text naming nothing: read whole, no list holds it");
        assert_eq!(not_a_name_in("INGEN (lots 1, 2, 3, 4, 5 og 6)", Some("DAN")), Some(Blank), "a scoped word too");
        assert_eq!(super::scoped_word("INGEN (lots 1, 2, 3, 4, 5 og 6)"), Some("ingen"));
        assert_eq!(super::scoped_word("Ingen (lot 2 til Dupont SARL)"), None, "the plan's split reads as the fold does");
        assert!(super::opens_with_scoped_word("INGEN (lots 1, 2, 3, 4, 5 og 6)") && super::opens_with_scoped_word("nd."));
        assert!(!super::opens_with_scoped_word("Ingenieurbüro Müller") && !super::opens_with_scoped_word("N.D."));
    }

    /// The fold is the role census's (`role_census::fold`), so the two lists can never
    /// disagree about one spelling.
    #[test]
    fn the_fold_is_the_role_census_fold() {
        assert_eq!(fold("Lot  déclaré—INFRUCTUEUX."), "lot declare infructueux");
        assert_eq!(fold("Unieważniony"), "uniewazniony");
        assert_eq!(fold("Pas d'offre"), "pas d offre");
    }

    /// Every entry is already in the fold's alphabet, so it can match at all, and every
    /// stem carries its boundary space (the match is a bare `contains`).
    #[test]
    fn every_entry_is_folded_and_every_stem_padded() {
        let placeholder_stems: Vec<&str> = PLACEHOLDER_STEMS.iter().map(|(stem, _)| *stem).collect();
        for stem in VOID_STEMS.iter().chain(&placeholder_stems).chain(&BLANK_STEMS).chain(&POINTER_STEMS) {
            let bare = stem.strip_prefix(' ').unwrap_or_else(|| panic!("{stem:?} lacks its boundary space"));
            assert_eq!(fold(bare), bare, "{stem:?}");
        }
        for pointer in POINTER_STEMS {
            assert!(placeholder_stems.contains(&pointer), "{pointer:?} is a placeholder stem");
        }
        for perfil in PERFIL_STEMS {
            assert!(POINTER_STEMS.contains(&perfil), "{perfil:?} is a pointer stem");
        }
        let scoped: Vec<&str> = SCOPED_WHOLE.iter().map(|(word, _)| *word).collect();
        let wholes: Vec<&str> = VOID_WHOLE
            .iter()
            .chain(&VOID_TEMPLATES)
            .chain(&PLACEHOLDER_WHOLE)
            .chain(&BLANK_WHOLE)
            .chain(&scoped)
            .copied()
            .collect();
        for whole in &wholes {
            assert_eq!(fold(whole), *whole, "{whole:?}");
            assert_eq!(wholes.iter().filter(|w| *w == whole).count(), 1, "{whole:?} is in one list only");
        }
        // Issue 511 unit 4's lists: padded where they are matched as a `contains` or a suffix,
        // folded wherever they are compared word for word.
        for padded in POINTER_LEAD_INS.iter().chain(&NO_INFORMATION_TAILS) {
            let bare = padded.strip_prefix(' ').unwrap_or_else(|| panic!("{padded:?} lacks its boundary space"));
            assert_eq!(fold(bare), bare, "{padded:?}");
        }
        for word in FORM_SECTION_TITLES.iter().chain(&REFERENCE_HEAD_WORDS) {
            assert_eq!(fold(word), *word, "{word:?}");
        }
        // Issue 511 unit 5's lists: folded, each phrase in one list only (a phrase two lists held
        // would be read by list order alone), and unit 4's lead-ins among unit 5's.
        let mut phrases: Vec<&str> = Vec::new();
        for (part, list) in REFERENCE_LISTS {
            for phrase in list {
                assert_eq!(fold(phrase), *phrase, "{part:?} {phrase:?}");
                assert!(!phrases.contains(phrase), "{phrase:?} is in two of unit 5's lists");
                phrases.push(phrase);
            }
        }
        for lead in POINTER_LEAD_INS {
            assert!(REFERENCE_LEAD_INS.contains(&lead.trim_start()), "{lead:?}");
        }
        for word in super::POSTPOSED_LOCATORS {
            assert!(super::SECTION_LOCATORS.contains(&word), "{word:?} is a locator");
        }
    }

    /// Issue 511: one prod specimen per lexicon entry (the census of 2026-10-11 and the
    /// review's reads), each with its class, each caught by the drain's net and, the
    /// qualified ones aside, an exact entry.
    #[test]
    fn every_measured_placeholder_spelling_has_its_class() {
        use NotAName::{Blank, Placeholder};
        let placeholders = [
            // Stems.
            "See section VI.4.1",
            "See Section VI.3 Additional information",
            "See Section VI.3) Additional information",
            "See section VI.3 below",
            "voir autres informations",
            "Voir renseignements complémentaires",
            "Ver información adicional",
            "Ver perfil del contratante",
            "Véase perfil de contratante",
            "Disclosure would prejudice commercial interests",
            "Withheld",
            "Withheld Section94 Supplier",
            "Withheld Section 94 Supplier",
            "See Contracts Finder Notice for full supplier list",
            "Siehe oben",
            "Siehe oben.",
            "keine Angabe.",
            "Keine Angaben",
            // The review's `keine Angabe` winners: a withheld winner, never a void lot.
            "Keine Angabe - der Zuschlag ging an mehrere Bieter, die weiterhin im Wettbewerb stehen",
            "keine Angabe nach § 39 (6) 2 VgV, Bieter Nummer 1/2/3",
            "Keine Angabe gemäß § 61 Abs 4 / § 62 Abs. 3 BVergG 2018",
            "keine Angabe aus Gründen des Wettbewerbs",
            // Whole values: the name is missing or pending.
            "k.A.",
            "K. A.",
            "kA",
            "TBC",
            "TBA",
            "To be confirmed",
            "Not Provided",
            "Not Specified",
            "Not available",
            "Unknown",
            "Not Known",
            "nicht bekannt",
            "Niet bekend",
            "non connu",
            "NON NOTO",
            "desconocido",
            "non renseigné",
            "brak danych",
            // A pointer.
            "s.o.",
            "S/O",
            "See below",
            "Please see below",
            "See attached",
            "See attached list",
            "See list below",
            "See Additional Information",
            "See VI.3",
            "Siehe unten",
            "Siehe Vergabeunterlagen",
            // Withheld.
            "Confidential.",
            "Redacted",
            "Commercially sensitive",
            "Not published",
            "Not disclosed",
            "Non communiqué.",
            // Plural.
            "Various",
            "Various Suppliers",
            "Various contractors",
            "Various Providers",
            "Various companies",
            "Various organisations",
            "Various - see below",
            "Multiple",
            "Multiple Suppliers",
            "Multiple providers",
            "Multiple Contractors",
            "Varios",
            "Vários",
            "Varias",
            "Varios adjudicatarios",
            "VARIOS LICITADORES",
            "Varios proveedores",
            "Varios contratistas",
            "Varias Empresas",
            "Diversos",
            "Diverse",
            "Diverse Firmen",
            "diverse Bieter",
            "Diverse ditte",
            "Diversi",
            "Divers",
            "Verschiedene",
            "Verschiedene Firmen",
            "mehrere",
            "mehrere Bieter",
            "Ditte varie",
            "varie",
        ];
        let blanks = [
            // The raw-name arm.
            ".",
            "-",
            "--",
            "---",
            "/",
            "/.",
            "*",
            "?",
            "./.",
            "-/-",
            "0",
            // Stems.
            "Nie dotyczy.",
            "Not Applicable",
            "Niet van toepassing.",
            // Whole values.
            "N/A",
            "N.A.",
            "N/ A",
            "N /A",
            "(N/a)",
            "No procede",
            "x.",
            "XX",
            "Xxx",
            "Entfällt.",
            "-entfällt-",
            "- entfällt -",
            "Entfaellt",
            "Néant.",
            "NEANT",
            "Keine",
            "Nicht vorhanden",
            "nicht zutreffend",
            "Nicht anwendbar",
            "Nicht relevant",
            "Ohne",
            "Nein",
            "NA",
            "NONE",
            "None specified",
            "Not Required",
            "Not relevant",
            "n.v.t.",
            "NVT",
            "Geen",
            "Brak",
            "n/d",
            "N.D.",
            "Sans objet",
            "SO",
            "Sans",
            "Non",
            "NC",
            "N/C",
            "Aucun",
            "Aucune",
            "Non concerné",
            "Non applicable",
            "Nessuno",
            "Nessuna",
            "Non previsto",
            "non applicabile",
            "Ninguno",
            "NINGUNA",
            "No aplica",
            "No hay",
            "NO",
            "null",
            "Ikke relevant",
            "Ikke aktuelt",
            "Ej aktuellt",
            "Não aplicável",
            "Não se aplica",
            "Δεν εφαρμόζεται.",
            "Δεν απαιτείται",
            "nu este cazul",
            "Nem releváns.",
            "není",
            "ei ole",
        ];
        for (names, class) in [(&placeholders[..], Placeholder), (&blanks[..], Blank)] {
            for name in names {
                assert_eq!(not_a_name(name), Some(class), "{name}");
                assert!(mentions_placeholder(name), "the walk's net catches {name}");
            }
        }
        for exact in ["Siehe oben", "N/A", "Néant.", "Various Suppliers", "-/-", "0", "Withheld", "Nie dotyczy."] {
            assert!(is_exact_entry(exact), "{exact}");
        }
        for qualified in ["Withheld Section94 Supplier", "See Section VI.3 Additional information", "Siehe oben (VI.4.1)"] {
            assert!(!is_exact_entry(qualified), "{qualified} is read before the wet run (511 §8.2)");
        }
        // `lettered_entries` stays quiet on a section number's `VI.3)`: no summary shape.
        assert!(!super::lettered_entries("See Section VI.3) Additional information"));
        assert!(!mentions_placeholder("Acme Ltd") && !mentions_placeholder("1") && !mentions_placeholder("Tennessee Section"));
    }

    /// Issue 511: a slot of punctuation or zeros holds nothing; any other digit is a name,
    /// and an empty or blank string is no name at all.
    #[test]
    fn punctuation_only_names_are_blank() {
        for blank in [".", "-/-", "..", "_", "0", "000", "0000", " - "] {
            assert_eq!(not_a_name(blank), Some(NotAName::Blank), "{blank:?}");
        }
        for nothing in ["", "  "] {
            assert_eq!(not_a_name(nothing), None, "{nothing:?}");
        }
        for name in ["1", "360", "24/7", "2000", "00001", "0.0"] {
            assert_eq!(not_a_name(name), None, "{name:?} stays a name");
        }
    }

    /// Issue 511 guard 2: `ingen`, `inget` and `nd` are blank only in their languages and in
    /// a plain case; with no language they are names (the text parser, the walk's class).
    #[test]
    fn scoped_words_need_their_language_and_a_plain_case() {
        assert_eq!(not_a_name_in("Ingen", Some("DAN")), Some(NotAName::Blank));
        assert_eq!(not_a_name_in("INGEN", Some("SWE")), Some(NotAName::Blank));
        assert_eq!(not_a_name_in("Inget", Some("NOR")), Some(NotAName::Blank));
        assert_eq!(not_a_name_in("Ingen", Some("FRA")), None, "1188404 `Ingen` FR is likely a French firm");
        assert_eq!(not_a_name_in("Ingen", None), None, "an unknown language keeps the party");
        assert_eq!(not_a_name("Ingen"), None);
        assert_eq!(not_a_name_in("InGen", Some("DAN")), None, "a brand's case");
        assert_eq!(not_a_name_in("nd.", Some("POL")), Some(NotAName::Blank));
        assert_eq!(not_a_name_in("ND", Some("ITA")), Some(NotAName::Blank));
        assert_eq!(not_a_name_in("ND", Some("HRV")), None, "`ND` HR carries an OIB");
        assert_eq!(super::scoped_word("INGEN"), Some("ingen"));
        assert_eq!(super::scoped_word("Acme"), None);
        assert!(mentions_placeholder("InGen"), "the net takes every scoped spelling; the fold decides");
    }

    /// Issue 511: the stems open the name (after one lot designator), so a real name
    /// followed by the phrase stays a name. The two phrases that never open a name keep
    /// 510's match anywhere.
    #[test]
    fn stems_open_the_name() {
        for name in [
            "Firma Handlowa Jan Kowalski – nie dotyczy",
            "Siemens – see section VI.3",
            "Acme GmbH, siehe oben",
            // One per anchored stem, with no company form for `summary_shape` to rescue (the
            // unit-2 review: unanchoring any one of these passed every test).
            "Stadtwerke Kiel – keine Angabe",
            "Jan Kowalski – withheld",
            "Mairie de Lyon – voir autres informations",
            "Mairie de Lyon – voir renseignements",
            "Ayuntamiento de Vigo – ver información",
            "Kent County Council – see Contracts Finder",
        ] {
            assert_eq!(not_a_name(name), None, "{name}");
            assert!(mentions_placeholder(name), "the net still lists {name}");
        }
        assert_eq!(not_a_name("Nie dotyczy - brak"), Some(NotAName::Blank));
        assert_eq!(not_a_name("Lot 3: not applicable"), Some(NotAName::Blank));
        assert_eq!(not_a_name("Lote nº 2: no procede"), None, "a whole value is compared whole");
        assert_eq!(not_a_name("Lot 3º"), None, "a designator with nothing after it");
        assert_eq!(not_a_name("Withheld for Security Reasons"), Some(NotAName::Placeholder));
        assert_eq!(not_a_name("Disclosure would prejudice commercial interests"), Some(NotAName::Placeholder));
        assert_eq!(not_a_name("Véase perfil del contratante"), Some(NotAName::Placeholder));
    }

    /// Issue 511's keep rule: a pointer followed by the body it points at names that body
    /// (the review's prefix reads: NEB, Vergabekammer Südbayern, VKS, Royal Courts); a
    /// pointer followed only by its own section reference or description does not.
    #[test]
    fn a_pointer_followed_by_a_name_is_kept() {
        for kept in [
            "siehe oben: NEB Betriebsgesellschaft mbH (die hiesige Angabe ist Pflicht)",
            "siehe oben (Vergabekammer Südbayern)",
            "see section Royal Courts of Justice",
            "Siehe oben – VKS",
            "Siehe oben — WSA Toenning",
            // The unit-2 review: a `:` right after the section reference is the separator.
            "See section VI.4.1: High Court of Justice",
            "Siehe oben Ziffer VI.4.1: Vergabekammer Südbayern",
            "siehe oben VI.4.1: Vergabekammer des Bundes",
        ] {
            assert_eq!(not_a_name(kept), Some(NotAName::AwardSummary), "{kept}");
        }
        for pointer in [
            "siehe oben (VI.4.1)",
            "siehe oben unter Ziffer VI.4.1",
            "See section VI.3 - Lot 2",
            "See Section VI.3) Additional information",
            "See Contracts Finder Notice for full supplier list",
            "See section VI.3 below",
            "siehe oben Nr. 3",
            // A colon with nothing that names after it.
            "siehe oben: VI.4.1",
            "See Section VI: complementary information",
            // A malformed section reference and a lettered lot are no names (the unit-2 review).
            "See Section VI3.2",
            "See section VI.3 - Lot 1A",
        ] {
            assert_eq!(not_a_name(pointer), Some(NotAName::Placeholder), "{pointer}");
        }
        assert!(!super::names_something("VI3.2 V1 IV2") && super::names_something("Vergabekammer"));
        assert!(super::names_something("MI5") && !super::roman_with_digits("VI") && !super::roman_with_digits("VIa3"));
    }

    /// Issue 511 (the unit-2 review): the three boundary stems may close a name as well as
    /// open one, so the text before one keeps the party when it names something; the bare
    /// phrases, their lead-in words included, stay placeholders.
    #[test]
    fn a_name_before_a_boundary_stem_is_kept() {
        for kept in [
            "Ayuntamiento de Madrid (véase perfil del contratante)",
            "Junta de Andalucía - ver perfil de contratante",
            "Serco — disclosure would prejudice commercial interests",
        ] {
            assert_eq!(not_a_name(kept), Some(NotAName::AwardSummary), "{kept}");
            assert!(!is_exact_entry(kept), "{kept}: read before the wet run");
        }
        for bare in [
            "Véase perfil del contratante",
            "Ver perfil de contratante",
            "Ver Perfil del Contratante.",
            "Disclosure would prejudice commercial interests",
        ] {
            assert_eq!(not_a_name(bare), Some(NotAName::Placeholder), "{bare}");
        }
    }

    /// Issue 511 unit 3 read: a perfil stem that opens the name, then a genitive (or `y`, an
    /// office and a genitive), names the profile's owner — the dry plan's real-party hits, 26
    /// orgs / 91 mentions in plan 2265, kept when written before the stem but dropped after
    /// it. A genitive onto the web page, the buyer's office or the national platform names
    /// nobody, and a lead-in leaves the pointer a pointer.
    #[test]
    fn a_perfil_followed_by_its_owner_is_kept() {
        for kept in [
            "Perfil de contratante del Gobierno de Aragón",
            "Perfil de contratante de la página WEB de la Fábrica Nacional de Moneda y Timbre - Real Casa de la Moneda",
            "Perfil del Contratante de la Comunidad Autónoma de la Región de Murcia",
            "Perfil de Contratante de la Universidad Complutense de Madrid",
            "Perfil del contratante del Ayuntamiento de San Fernando",
            "Perfil de contratante del SESCAM",
            "PERFIL DEL CONTRATANTE DE LA CARM",
            "Perfil del contratante y Servicio Central de Contratación de la Diputación de Alicante",
            "Perfil de contratante y Registro Auxiliar de la Universidad de Valladolid",
            "Perfil del contratante de la Empresa Malagueña de Transportes S.A.M.",
            // More of the plan's spellings: an acronym after a bare `de`, a URL after the
            // owner, the platform beside a named owner; and the Catalan elision.
            "Perfil del contratante de Anfaco-Cecopesca",
            "Perfil del contratante de la página web de la FNMT-RCM (www.fnmt.es)",
            "PERFIL DEL CONTRATANTE DE LA COMUNIDAD AUTÓNOMA DE LA REGIÓN DE MURCIA WWW.CARM.ES",
            "Perfil de Contratante del Ayto. de Soria y plataforma de Contratación del Estado",
            "Perfil del contratante de l'Ajuntament de Girona",
        ] {
            assert_eq!(not_a_name(kept), Some(NotAName::AwardSummary), "{kept}");
            assert!(mentions_placeholder(kept) && !is_exact_entry(kept), "{kept}: listed, and read before the wet run");
        }
        for pointer in [
            "Ver perfil del contratante",
            "Véase perfil del contratante",
            "Perfil del contratante de la página web",
            "Perfil Del Contratante De La Página Web",
            "Perfil del contratante del órgano de contratación",
            "Perfil del Contratante del Órgano de Contratación",
            "Perfil del contratante (www.x.es)",
            "perfil del contratante de la Plataforma de Contratación del Sector Público",
            "Perfil del contratante y Plataforma de Contratación del Estado",
            "Perfil del contratante de la web municipal (puntos de contacto)",
            "Perfil del contratante de WWW.MADRID.ES",
            "perfil de contratante www.gelves.es",
            // A lead-in: a winner slot pointed at the buyer's profile (`text::parse`'s junk).
            "Ver perfil del contratante de la Junta de Andalucía",
            "Ver Perfil de Contratante del Hospital Clínic de Barcelona",
            // `o`, not `y`: another place to look, not the owner's other office.
            "Perfil del Contratante o Jta. de Contratación",
        ] {
            assert_eq!(not_a_name(pointer), Some(NotAName::Placeholder), "{pointer}");
        }
    }

    /// Issue 511 unit 3 read: a blank stem followed by a `but` clause that names a body names
    /// that body — the review-body slot's High Court (plan 2265: 6 orgs / 8 mentions). A comma
    /// alone continues the blank, and a `but` that names nothing leaves it blank.
    #[test]
    fn a_blank_slot_but_a_named_body_is_kept() {
        for kept in [
            "Not applicable, but proceedings may be started in the High Court of England and Wales",
            "Not applicable but proceedings may be started in the High Court of England and Wales",
            "Not Applicable, but proceedings may be started in the High Court of England and Wales in accordance with Part 3 (Remedies) of the Public Contracts Regulations 2015",
        ] {
            assert_eq!(not_a_name(kept), Some(NotAName::AwardSummary), "{kept}");
        }
        for blank in [
            "Not applicable",
            "N/A",
            "Not applicable.",
            // A comma alone: the capitals after it name nobody.
            "Not applicable, see section VI.4",
            "Not Applicable, since Minesto is a private company.",
            "nie dotyczy, zamówienie udzielane w oparciu o zasadę konkurencyjności niepodlegające ustawie PZP",
            "Not applicable, but see below",
            "Not applicable in Northern Ireland",
        ] {
            assert_eq!(not_a_name(blank), Some(NotAName::Blank), "{blank}");
        }
    }

    /// Issue 511: a stem inside a summary of several parties is an award summary, which the
    /// fold keeps; a stem the anchor misses leaves the name a name, as today.
    #[test]
    fn a_stem_inside_a_summary_is_an_award_summary() {
        assert_eq!(not_a_name("Lot 2: not applicable. Lot 1: ACME Ltd"), Some(NotAName::AwardSummary));
        assert_eq!(not_a_name("Nie dotyczy (część 2); część 1: X sp. z o.o."), Some(NotAName::AwardSummary));
        assert_eq!(not_a_name("Lot 1: ACME Ltd; Lot 2: not applicable"), None, "the anchor misses it");
        assert!(mentions_placeholder("Lot 1: ACME Ltd; Lot 2: not applicable"), "and the dry plan lists it");
        assert_eq!(not_a_name("Keine Angabe (Bietergemeinschaft Muster GmbH)"), Some(NotAName::AwardSummary));
    }

    /// Issue 511 §6: every fold of the retired `role_census::NON_NAME_FOLDS` is a non-name
    /// now — except `nil`, which is NIL SAS (FR SIRET) and found only in award slots.
    #[test]
    fn the_former_non_name_folds_are_classified() {
        for fold in [
            "n a",
            "na",
            "none",
            "unknown",
            "not known",
            "confidential",
            "withheld",
            "not published",
            "not disclosed",
            "not applicable",
            "not specified",
            "not provided",
            "various",
            "x",
        ] {
            assert!(not_a_name(fold).is_some(), "{fold}");
        }
        assert_eq!(not_a_name("nil"), None, "deferred: a real company");
    }

    /// Issue 511: `Not applicable` moved from 510's placeholder stems to the blank ones.
    #[test]
    fn not_applicable_is_a_blank_slot() {
        assert_eq!(not_a_name("Not applicable"), Some(NotAName::Blank));
        assert_eq!(not_a_name("Not applicable (lot 2)"), Some(NotAName::Blank));
    }

    /// Issue 511 unit 4 item 2 (residual dry 2273: 144 winner mentions read as summaries): a TED
    /// form section's own title after a pointer is the section's name, not a body's — `See
    /// section VI.3 (Additional Information)` points, as `See Section VI.3 Additional
    /// information` always did. A real body beside a title still names itself.
    #[test]
    fn a_form_section_title_after_a_pointer_is_no_name() {
        for pointer in [
            "See Section VI.3 Additional information",
            "See Section VI.3) Additional information",
            "See section VI.3 (Additional Information)",
            "See Section VI.2 'Additional information'.",
            "See Section VI.2 'Additional information'",
            "See Section VI.2 Additional information",
            "see section VI complementary information",
            "See Section VI: complementary information",
            "See Section VI: Complementary Information",
            "See section II.2.3 Place of performance",
            "See Section II.2.3) Place of performance",
            "See section VI.3 - Additional Information",
            "See Section II.1.2: Main place of delivery",
            "See section V (Award of Contract)",
            "See section VI.4: Review procedures",
            // A malformed reference (`V1.3` for `VI.3`) names nothing either (2 mentions in 2273).
            "See Section V1.3 Additional information",
        ] {
            assert_eq!(not_a_name(pointer), Some(NotAName::Placeholder), "{pointer}");
            assert!(mentions_placeholder(pointer), "the walk's net catches {pointer}");
        }
        for kept in [
            "siehe oben: NEB Betriebsgesellschaft mbH (die hiesige Angabe ist Pflicht)",
            "See section VI.4.1: High Court of Justice",
            "See section VI.4.1: High Court of Justice (additional information)",
            "See Section VI.3 (Additional Information: Royal Courts of Justice)",
            "see section Royal Courts of Justice",
            "See section 1 Contracting authority for details of Viridian Group Ltd contacts",
        ] {
            assert_eq!(not_a_name(kept), Some(NotAName::AwardSummary), "{kept}");
        }
        assert_eq!(super::without_section_titles("High Court (Additional Information)"), format!("High Court ({})", " ".repeat(22)));
        assert_eq!(super::without_section_titles("Additional informationQual"), "Additional informationQual", "whole words only");
    }

    /// Issue 511 unit 4 item 3 (residual dry 2273: `Please see section VI.4.2` 378 orgs / 411
    /// mentions, `please see Section VI.3.2` 267 / 315, `Please see Section VI.4.3)` 5 / 106): a
    /// polite first word before a pointer stem leaves the pointer a pointer. Only `please` and
    /// `kindly`, only first, only directly before the stem.
    #[test]
    fn politeness_before_a_pointer_stem_is_still_the_pointer() {
        for pointer in [
            "Please see section VI.4.2",
            "Please see section VI.4.2.",
            "please see section Vi.4.2",
            "please see Section VI.3.2",
            "Please see Section VI.4.3)",
            "Please see section 6.3.2 below",
            "Please see section VI.3 below",
            "Kindly see section VI.3",
            "Lot 2: please see section VI.3",
            // A pointer to the suppliers names none of them (19 + 2 winner mentions).
            "Please see section VI.3 for list of awarded Suppliers",
            "Please see Section VI.3) for list of awarded suppliers",
            // With item 2's titles.
            "Please see section V1.2 Additional Information",
            "Please see Section VI.2 (Additional information)",
            "Please see section VI) — VI.3) Additional Information",
        ] {
            assert_eq!(not_a_name(pointer), Some(NotAName::Placeholder), "{pointer}");
            assert!(mentions_placeholder(pointer), "the walk's net catches {pointer}");
        }
        assert_eq!(not_a_name("Please see section VI.4.1: High Court of Justice"), Some(NotAName::AwardSummary), "the keep rule");
        for name in [
            "Siemens – see section VI.3",
            "Acme Ltd – please see section VI.3",
            "Kindly Care Services – see section VI.3",
            "Please Hold Ltd",
            "Pleasance Theatre Trust",
            "Pleased to see section heads",
        ] {
            assert_eq!(not_a_name(name), None, "{name}");
        }
    }

    /// Issue 511 unit 4 item 4 (residual dry 2273): the withheld and not-applicable sentences the
    /// lexicon missed, each an exact shape — whole values, or the German `… erfolgt (hier) keine
    /// Angabe` after a legal reference only. Withheld is `Placeholder`, not applicable `Blank`.
    #[test]
    fn withheld_and_not_applicable_sentences_have_their_class() {
        use NotAName::{Blank, Placeholder};
        for (name, class, exact) in [
            ("Information withheld for security reasons", Placeholder, true),
            ("Due to the sensitive nature of this contract the supplier name will be withheld", Placeholder, true),
            ("Due to the sensitive nature of this contract the supplier name will be withheld from publication", Placeholder, true),
            ("Due to the sensitive nature of this contract the supplier name is withheld from publication", Placeholder, true),
            ("Information withheld due to security.", Placeholder, true),
            ("This information has been omitted from this notice in accordance with s.94 Procurement Act 2023", Placeholder, true),
            ("unter Bezugnahme auf § 39 Abs 6 Nr. 4 VgV erfolgt hier keine Angabe", Placeholder, false),
            ("Unter Berufung auf Art. 24 Abs. 2 erfolgt keine Angabe", Placeholder, false),
            ("Unter Berufung auf Artikel 24 Abs. 2 der Richtlinie 93/38 EWG erfolgt hier keine Angabe", Placeholder, false),
            ("Unter Berufung auf § 11 SKR Abs. 2 (VOL/A-SKR) erfolgt hier keine Angabe", Placeholder, false),
            ("1 VOL/A Abschnitt 4 SKR erfolgt hier keine Angabe", Placeholder, false),
            ("1 erfolgt hier keine Angabe", Placeholder, false),
            ("This is not applicable in the UK", Blank, true),
            ("This is not applicable to the UK", Blank, true),
            ("This is not applicable in UK", Blank, true),
        ] {
            assert_eq!(not_a_name(name), Some(class), "{name}");
            assert!(mentions_placeholder(name), "the walk's net catches {name}");
            assert_eq!(is_exact_entry(name), exact, "{name}");
        }
        for name in [
            "Stadtwerke Kiel – erfolgt keine Angabe",
            "Müller GmbH (erfolgt hier keine Angabe)",
            "Information Withheld Ltd",
            "Due Diligence Partners Ltd",
        ] {
            assert_eq!(not_a_name(name), None, "{name}");
        }
    }

    /// Issue 511 unit 4 item 1: the form section numbers a name carries, for the fold's
    /// translation echo — a Roman numeral and dotted digits, its surrounding punctuation off, any
    /// case; and the shape a translation of a pointer has (short, no legal form).
    #[test]
    fn form_section_tokens_and_the_translation_shape() {
        use super::{form_section_tokens, translation_shaped};
        for (raw, tokens) in [
            ("See Section VI.2 'Additional information'.", vec!["vi.2"]),
            ("Lásd a VI.2., További információk című pontot.", vec!["vi.2"]),
            ("Skatīt VI.2) iedaļu \"Papildu informācija\".", vec!["vi.2"]),
            ("please see section Vi.4.2", vec!["vi.4.2"]),
            ("See Section II.2.3) Place of performance", vec!["ii.2.3"]),
            ("(IV.1.1)", vec!["iv.1.1"]),
            ("See Section VI: complementary information", vec![]),
            ("Please see section 6.3.2 below", vec![]),
            ("See Section V1.3", vec![]),
            ("See Section VI3.2", vec![]),
            ("Acme Ltd", vec![]),
        ] {
            assert_eq!(form_section_tokens(raw), tokens, "{raw}");
        }
        assert!(translation_shaped("Se punkt VI.2) Yderligere oplysninger"));
        assert!(translation_shaped("Βλέπε Τμήμα VI.2 «Πρόσθετες πληροφορίες»."));
        for party in [
            "Müller Bau GmbH (siehe Abschnitt VI.3)",
            "Acme Ltd, see VI.3",
            "Olprint Sp. z o.o. (VI.3)",
            "Nordic Oy VI.3",
            "Verein für Bildung e.V. VI.3",
            "Consortium of the participating institutions of the Netherlands Aerospace Laboratory listed in VI.3 below",
            // The forms the crate's other readings leave out (the review).
            "Acme plc VI.3",
            "Acme LLP (VI.3)",
            "Acme Inc. VI.3",
            "Acme A/S VI.3",
            "Acme AS VI.3",
            "Acme OÜ VI.3",
            "Acme d.o.o. VI.3",
            "Acme a.s. VI.3",
            "Acme GmbH & Co. KG VI.3",
            "Acme Bau KG VI.3",
        ] {
            assert!(!translation_shaped(party), "{party}");
            // Issue 511 unit 5, the review: no pointer word opens these, so `pointer_led` alone
            // refuses them; led by one, only the legal form or the word cap keeps each.
            let led = format!("Se punkt VI.3 {party}");
            assert!(super::pointer_led(&led) && !translation_shaped(&led), "{led}");
        }
        for party in [
            "Se punkt VI.3 Hansen A/S",
            "Voir point VI.3 Dupont SARL",
            "Wie VI.3 Acme plc",
            "Siehe Abschnitt VI.3 Müller Bau GmbH",
            "Zie VI.3 voor de deelnemende instellingen van het consortium rond het Nederlands Lucht- en Ruimtevaartlaboratorium",
        ] {
            assert!(super::pointer_led(party), "{party}");
            assert!(!translation_shaped(party), "{party}");
        }
        // The guards' controls: the same copies without the form, or cut to twelve words.
        for copy in ["Se punkt VI.3 Hansen", "Siehe Abschnitt VI.3 Müller Bau", "Zie VI.3 voor de deelnemende instellingen van het consortium rond het Nederlands"] {
            assert!(translation_shaped(copy), "{copy}");
        }
        // Croatian, TED's 24th language (reconstructed): pointer-led, so the echo can sweep it.
        assert!(translation_shaped("Vidjeti odjeljak VI.2 „Dodatni podaci”."));
        assert!(translation_shaped("Vidi odjeljak VI.2 „Dodatni podaci”."));
    }

    /// Issue 511 unit 4, the review of item 1: the fold's translation echo seeds only from a bare
    /// pointer — nothing named after its stem once the references are stripped and the section
    /// titles blanked, the keep rule's separator requirement aside — and the walk's net reads
    /// every name the echo can sweep, because a non-English original heads its party with one.
    #[test]
    fn a_bare_pointer_seeds_the_echo_and_the_net_reads_every_copy_it_sweeps() {
        use super::names_after_stem;
        for bare in [
            "See Section VI.2 'Additional information'.",
            "See Section VI.3 Additional information",
            "See section VI.3 (Additional Information)",
            "See Section VI.3) Additional information",
            "Please see section VI.4.2",
            "Please see Section VI.4.3)",
            "please see Section VI.3.2",
            "siehe oben unter Ziffer VI.4.1",
            // No stem: a whole value names nothing after one.
            "Various suppliers",
            "unter Bezugnahme auf § 39 Abs 6 Nr. 4 VgV erfolgt hier keine Angabe",
        ] {
            assert!(!names_after_stem(bare), "{bare} is a bare pointer");
        }
        // A body after the number with no separator: not bare, so it seeds nothing — the
        // residual's `Translink` (1 mention, review body). Since issue 511 unit 6 item 1 the name
        // right after the reference makes it an award summary by itself; a capitalised noun later in
        // the tail leaves the pointer `Placeholder`, and withholds the seed, erring toward a name.
        for (named, class) in [
            ("See Section VI.3 Kier Construction", NotAName::AwardSummary),
            ("Please see section VI.4.3 Translink", NotAName::AwardSummary),
            ("See section VI.4.3 Translink", NotAName::AwardSummary),
            ("See Section VI.3 Acme plc", NotAName::AwardSummary),
            ("Please see section VI.3 for list of awarded Suppliers", NotAName::Placeholder),
        ] {
            assert_eq!(not_a_name(named), Some(class), "premise: {named}");
            assert!(names_after_stem(named), "{named} names something after its stem");
            assert!(!super::echo_seed(named) && !super::template_seed(named), "{named} seeds nothing");
        }
        assert!(names_after_stem("See section VI.4.1: High Court of Justice"), "the keep rule's own case");
        // Issue 511 unit 5, the review: the echo's seeds stay unit 4's — a bare placeholder the
        // lexicon classes without the section-reference pointers. TED's English template seeds; a
        // pointer only unit 5 classes, in any language, is `Placeholder` and seeds nothing (a
        // bilingual award's FI `Katso kohta VI.4.1` swept its SV copy that names the body).
        for seed in [
            "See Section VI.2 'Additional information'.",
            "See section VI.3 (Additional Information)",
            "Please see section VI.4.2",
            "siehe oben unter Ziffer VI.4.1",
        ] {
            assert!(super::echo_seed(seed), "{seed}");
        }
        for pointer in ["Katso kohta VI.4.1", "Voir VI.4.1", "See VI.4.2", "siehe VI.4.1", "V.1) AWARD AND CONTRACT VALUE:"] {
            assert_eq!(not_a_name(pointer), Some(NotAName::Placeholder), "{pointer}");
            assert!(!super::echo_seed(pointer), "{pointer} seeds nothing");
        }
        for named in ["See Section VI.3 Kier Construction", "Acme Ltd"] {
            assert!(!super::echo_seed(named), "{named}");
        }
        // The net: a FR- or DE-original award heads its pointer party with the translation. Since
        // issue 511 unit 5 TED's template copies are section-reference pointers by themselves.
        for copy in [
            "Voir point VI.2) Autres informations.",
            "siehe Abschnitt VI.2 „Sonstige Informationen“.",
            "Βλέπε Τμήμα VI.2 «Πρόσθετες πληροφορίες».",
            "Lásd a VI.2., További információk című pontot.",
        ] {
            assert_eq!(not_a_name(copy), Some(NotAName::Placeholder), "{copy}");
            assert!(mentions_placeholder(copy), "the walk's net catches {copy}");
            assert!(!is_exact_entry(copy), "and the dry plan lists {copy} for reading");
        }
        // A pointer-led copy that still reads as a name — a template wording the lexicon misses,
        // or a body copied untranslated — is what the echo can still sweep, so the net reads it.
        for copy in ["Se punkt VI.2) Nye oplysninger", "Siehe Abschnitt VI.4.3 Translink"] {
            assert_eq!(not_a_name(copy), None, "premise: {copy} reads as a name alone");
            assert!(super::translation_shaped(copy) && mentions_placeholder(copy), "the walk's net catches {copy}");
        }
        for name in [
            "Müller Bau GmbH (siehe Abschnitt VI.3)",
            "Consortium of the participating institutions of the Netherlands Aerospace Laboratory listed in VI.3 below",
            "Se punkt VI Yderligere oplysninger",
            "Region Hovedstaden",
            "Acme Ltd",
            // Issue 511 unit 5: a short name with a section number is no translated pointer unless
            // a pointer word opens it, so the narrowed net leaves it (unit 4's net took it).
            "Region Hovedstaden VI.3",
            "Referat II.3",
        ] {
            assert!(!mentions_placeholder(name), "{name}");
        }
    }

    /// Issue 511 unit 5 (dry 2283, the `name` class: about 20k orgs / 53k mentions): a
    /// section-reference pointer — a pointer, locators, form section references, positions, the
    /// notice and form labels, nothing else — is a placeholder in every language the plan's top
    /// folds hold, as each spelling was published. Each is caught by the walk's net and read
    /// before the wet run (not an exact entry).
    #[test]
    fn a_section_reference_pointer_is_a_placeholder_in_every_language() {
        use ReferenceShape::{Label, Pointer};
        for (name, shape) in [
            // German: `siehe VI.4.1` (2,550 orgs / 10,629 mentions), `siehe VI.3.1` (1,495), `siehe
            // I.1` (980), and the forms with a locator, a position, the notice and a title.
            ("Siehe VI.4.1)", Pointer),
            ("siehe VI.3.1", Pointer),
            ("siehe I.1)", Pointer),
            ("Siehe unter VI.4.1)", Pointer),
            ("S. VI.4.1.", Pointer),
            ("s. Ziffer I.1)", Pointer),
            ("Siehe Punkt VI.4.1)", Pointer),
            ("siehe Ziffer VI.4.1)", Pointer),
            ("Siehe Ziff. VI.4.1)", Pointer),
            ("Siehe Abschnitt I.1)", Pointer),
            ("Siehe Nr. VI.4.1", Pointer),
            ("Wie oben Ziffer VI.4.1)", Pointer),
            ("wie (oben Ziffer) VI.3.1", Pointer),
            ("Wie Pkt. VI.4.1)", Pointer),
            ("wie unter VI.4.1", Pointer),
            ("s.o. VI.4.1)", Pointer),
            ("sh. VI.4.1)", Pointer),
            ("S. h. VI.4.1", Pointer),
            ("Vgl. VI.3.1) Zuständige Stelle für Rechtsbehelfs-/Nachprüfungsverfahren", Pointer),
            ("Siehe Kontaktstelle unter Punkt I.1) der Bekanntmachung", Pointer),
            ("Wie oben Ziffer VI.4.1 der vorliegenden Bekanntmachung", Pointer),
            ("siehe Tz. VI.3 dieser Vergabebekanntmachung", Pointer),
            ("Siehe I.1.) Öffentlicher Auftraggeber", Pointer),
            ("Vergabestelle (siehe I.1)", Pointer),
            ("Anschrift siehe Nr. I.1)", Pointer),
            ("s.o. Ziff. VI.3) Nr. 1 der Bekanntmachung", Pointer),
            ("Siehe Angaben unter Ziff. VI.4.1)", Pointer),
            ("sh. Punkte VI.2) und VI.3.2)", Pointer),
            // English: `see VI.4.2 (below/above)`, `refer to point VI.3.2`, `as in I.1 above`, the
            // award section's heading alone in a winner slot (2,629 + 286 orgs).
            ("See VI.4.2", Pointer),
            ("see VI.4.2 below", Pointer),
            ("See VI.4.2 Above", Pointer),
            ("Refer to point VI.3.2", Pointer),
            ("Please refer to VI.4.1.", Pointer),
            ("Please refer to section VI.2 of this Notice", Pointer),
            ("As in I.1 above", Pointer),
            ("AS IN I.1 ABOVE", Pointer),
            ("As VI.4.1", Pointer),
            ("As per I.1", Pointer),
            ("Same as VI.4.1", Pointer),
            ("Same as in point VI.4.1", Pointer),
            ("See Details at VI.4.3)", Pointer),
            ("See VI.3) Additional information", Pointer),
            ("For details of the awarded suppliers, please see section VI.2", Pointer),
            ("For details of the awarded supplier please see section VI.2", Pointer),
            ("see section VI.3.2.below", Pointer),
            ("V.1) AWARD AND CONTRACT VALUE:", Label),
            ("V.1.4) Information on value of contract: V.1) AWARD AND CONTRACT VALUE:", Label),
            ("V.1) Award and contract value V.1.1) Name and address of successful supplier", Label),
            ("Section VI.4.2) below", Label),
            ("VI.4.1", Label),
            // French: `voir section VI.4.2`, `cf rubrique VI.4.1`.
            ("Voir section VI.4.2", Pointer),
            ("cf rubrique VI.4.1", Pointer),
            ("cf.rubrique VI.4.1", Pointer),
            ("Cf art. VI.4.1)", Pointer),
            ("Voir rubrique VI.3.1 ci-dessus.", Pointer),
            ("voir article VI.3.2", Pointer),
            ("(Voir modalités exposées au VI.3)", Pointer),
            ("se reporter à la rubrique Vi.3) relative aux informations complémentaires.", Pointer),
            ("Même entité que celle mentionnée en VI.3.1.", Pointer),
            // Italian: `vedasi punto I.1`, `vd punto I.1`.
            ("Vedasi punto I.1", Pointer),
            ("vd. punto I.1)", Pointer),
            ("vedi punto I.1)", Pointer),
            ("V. punto I.1) del presente avviso", Pointer),
            ("come al punto VI.3.1)", Pointer),
            ("Vedasi p.to VI.3.1)", Pointer),
            ("Come riportato al punto I.1)", Pointer),
            ("Punti di contatto di cui al punto I.1.", Pointer),
            // Spanish: `véase I.1`.
            ("Véase I.1", Pointer),
            ("Ver el apartado VI.4.3 del presente anuncio", Pointer),
            ("Se indica en el apartado I.1)", Pointer),
            ("Ver punto I.1) de este anuncio de licitación", Pointer),
            // Polish: `jak w pkt VI.3.1`.
            ("jak w pkt VI.3.1)", Pointer),
            ("Jak w pkt. VI.4.1)", Pointer),
            ("jak w pkt VI.4.1) ogłoszenia", Pointer),
            ("zgodnie z pkt. VI.4.1 ogłoszenia", Pointer),
            ("j.w. w pkt. VI.3.1)", Pointer),
            ("jw. dla pkt. VI.4.1.", Pointer),
            ("jak w sekcji VI.4.1) niniejszego ogłoszenia", Pointer),
            ("jak w punkcie VI.4.1)", Pointer),
            ("tak jak w pkt VI.3.1)", Pointer),
            ("Patrz sekcja VI.2) - informacje dodatkowe", Pointer),
            // Dutch, Greek, Danish, Finnish, Slovak, Czech, Hungarian.
            ("Zie VI.2", Pointer),
            ("Zie VI.2) Nadere inlichtingen", Pointer),
            ("Zie afdeling I.1", Pointer),
            ("Όπως I.1", Pointer),
            ("όπως I.1.", Pointer),
            ("Ως τμήμα I.1.", Pointer),
            ("Βλέπε σημείο VI.4.1.", Pointer),
            ("Se afsnit VI.3.", Pointer),
            ("Se supplerende oplysninger i del VI.2)", Pointer),
            ("Sama kuin kohdassa I.1", Pointer),
            ("Uvedené v bode VI.3) Doplňujúce informácie", Pointer),
            ("viz bod VI.3)", Pointer),
            ("lásd VI.2)", Pointer),
            // The review of unit 5: a locator points at a reference — forward through locators,
            // joiners and titles, back for a postposed one (TED's HU, LT, LV), or as part of the
            // title before it; a separator past a reference opens titles, positions and joiners
            // only; a label closes on labels.
            ("Please see the item under section VI.4.2", Pointer),
            ("Siehe VI.3) Nr. 1", Pointer),
            ("Se reporter à la rubrique IV.3 point 16", Pointer),
            ("Lásd a VI.2., További információk című pontot.", Pointer),
            ("Žr. VI.2 dalį „Papildoma informacija“.", Pointer),
            ("Please see VI.3 'Additional Information Section'", Pointer),
            ("See VI.4.1) Review Body Field of this Contract Notice", Pointer),
            ("See section VI.4.1: Review Body", Pointer),
            ("see VI.4.2 (below)", Pointer),
            ("SECTION VI: OTHER INFORMATION VI.7) Other information: I", Label),
            ("VI.4.1) i VI.4.2)", Label),
            // The review: Croatian, TED's 24th language (reconstructed from the form labels, not
            // read off a notice: the unit-4 canary reads one), SL `točko` (a winner in dry 2283),
            // and the abbreviations of listed languages.
            ("Vidjeti odjeljak VI.2 „Dodatni podaci”.", Pointer),
            ("Vidi odjeljak VI.2 „Dodatni podaci”.", Pointer),
            ("Glej točko VI.2.", Pointer),
            ("Ks. kohta I.1", Pointer),
            ("Sk. VI.2) iedaļu \"Papildu informācija\".", Pointer),
            ("Βλ. τμήμα VI.2", Pointer),
            ("Vezi secţiunea VI.2", Pointer),
            ("Виж раздел VI.2", Pointer),
            ("Vgs. punto I.1)", Pointer),
            ("siehe Abschn. VI.4.1", Pointer),
            ("see sec. VI.4.1 above", Pointer),
        ] {
            assert_eq!(section_reference(name), Some(shape), "{name}");
            assert_eq!(not_a_name(name), Some(NotAName::Placeholder), "{name}");
            for lang in [Some("DEU"), Some("POL"), Some("DAN")] {
                assert_eq!(not_a_name_in(name, lang), Some(NotAName::Placeholder), "{name} in {lang:?}");
            }
            assert!(mentions_placeholder(name), "the walk's net catches {name}");
            assert!(!is_exact_entry(name), "{name} is read before the wet run (511 §8.2)");
        }
    }

    /// Issue 511 unit 5's near misses: anything left over beside the pointer — a named body, a
    /// person or role, a legal form, an organisational unit numbered like a section, a date, an
    /// answer — and the name is not a section-reference pointer; each reads as it did before unit
    /// 5. The ones no pointer word opens stay out of the walk's net, which unit 4's echo arm took
    /// (`Referat II.3`, the Kaiserslautern office's 311 buyer rows).
    #[test]
    fn a_name_beside_a_section_reference_stays_a_name() {
        for (name, in_net) in [
            // A named body and the pointer.
            ("Krajowa Izba Odwoławcza – jak wyżej w pkt VI.4.1)", false),
            ("KIO – jak w pkt VI.3.1)", false),
            ("Vergabekammer des Landes Berlin; siehe Punkt VI.4.1)", false),
            ("Tribunal Catalán de Contratos del Sector Público (ver apartado VI.4.3)", false),
            ("Court of Justice of the European Union (see point VI.4.1. above)", false),
            ("Siehe Punkt VI.4.1) - VKS Wien", true),
            ("siehe VI.3.1) Vergabekammer Nordbayern bei der Regierung von Mittelfranken", true),
            // A person or a role.
            ("Responsabile del procedimento di cui al punto VI.3, lettera q)", false),
            ("responsabile del procedimento di cui al punto VI.3 lettera q", false),
            ("arch. Paola Zancanato (Vedi punto I.1)", false),
            ("R.U.P. di cui al punto VI.3) lettera o)", false),
            // A legal form.
            ("Leinemann Partner Rechtsanwälte mbB, siehe I.1)", false),
            ("Acme AS VI.3", false),
            ("Hansen A/S, se pkt. VI.3", false),
            ("Section 1.2 Ltd", false),
            ("Section II.3 Ltd", false),
            // An organisational unit numbered like a form section.
            ("Referat II.3", false),
            ("Abteilung VI.4", false),
            ("LMU München, Referat IV.1", false),
            ("Deutsche Rentenversicherung Knappschaft-Bahn-See, Dezernat IV.2.6", false),
            ("Stadtverwaltung Kaiserslautern - Stabstelle IV.1 Zentrale Vergabestelle", false),
            ("Rheingau-Taunus-Kreis FD I.7", false),
            // The award heading after a supplier's name, a date or an answer after a label.
            ("Transports Chalavan. V.1) AWARD AND CONTRACT VALUE:", false),
            ("V.1) DATE OF CONTRACT AWARD: 15.5.2006", false),
            ("V.5) THE CONTRACT IS LIKELY TO BE SUB-CONTRACTED: No", false),
            // No form section reference: an arabic number, a bare numeral, a glued sub-item.
            ("Siehe Abschnitt 6.3.2", false),
            ("Se punkt VI Yderligere oplysninger", false),
            ("Siehe VI.4.1a", false),
            // The review of unit 5: a company whose name is list words. A lot-numbered winner
            // (dry 2283 holds ~50 `I.n) <winner>` spellings, `I.6) Colas`, `I.4) Brake France`):
            // a label closes on labels, and a locator points at a reference.
            ("I.4) Avis", false),
            ("I.4) AVIS", false),
            ("I.3) Point P", false),
            ("I.3) POINT P", false),
            ("I.3) Point.P", false),
            ("II.1) Le Point", false),
            ("I.2) Il Punto", false),
            ("I.2) The Point", false),
            ("I.2) De Punt", false),
            ("I.5) Art", false),
            ("I.6) Item", false),
            ("I.7) Bloc", false),
            ("I.4) Ricoh", false),
            ("I.2) Avis Location", false),
            // …a locator that runs into a pointer, or into the end of the name.
            ("Point P (voir I.1)", false),
            ("Point P, voir VI.2", false),
            ("siehe I.1) Point P", true),
            // …and after a separator, where the reference does not reach (unit 4's keep rule
            // reads it, no stem here).
            ("Voir VI.2 - Point P", true),
            ("Voir rubrique VI.2 : Point P", true),
        ] {
            assert!(!section_reference_pointer(name), "{name}");
            assert_eq!(not_a_name(name), None, "{name}");
            assert_eq!(mentions_placeholder(name), in_net, "{name}");
        }
        // The review of unit 5: a body named after a separator past the reference, in list words,
        // goes through unit 4's keep rule as before unit 5 — an award summary after a pointer stem
        // (`Avis`, `Point P`: frequent French winners), so one such name keeps its party.
        for name in [
            "See section VI.2: Avis",
            "See section VI.2: Point P",
            "Siehe oben (I.1): Point P",
            "See section: Avis",
            "Siehe oben: Point P",
            "See section VI.2: Colas",
            "See section VI.2: Avis Budget",
        ] {
            assert!(!section_reference_pointer(name), "{name}");
            assert_eq!(not_a_name(name), Some(NotAName::AwardSummary), "{name}");
        }
        // The unit-4 stem rules read as before: a body after a stem's pointer is kept.
        assert_eq!(not_a_name("Siemens – see section VI.3"), None);
        assert_eq!(not_a_name("See section VI.4.1: High Court of Justice"), Some(NotAName::AwardSummary));
        assert_eq!(not_a_name("siehe oben: Vergabekammer Südbayern (VI.3)"), Some(NotAName::AwardSummary));
    }

    /// Issue 511 unit 5 (dry 2283's `legal_form` read list held 518 pointers such as `As in I.1
    /// above`): the English pointer word `as` is no Norwegian or Estonian `AS` when every other
    /// word of the name is a section-reference pointer's; beside a name it is the form again.
    #[test]
    fn the_pointer_word_as_is_no_legal_form() {
        use super::{names_a_legal_form, translation_shaped};
        for pointer in ["As in I.1 above", "AS IN I.1 ABOVE", "AS VI.4.1", "As VI.4.1", "AS PER I.1"] {
            assert!(!names_a_legal_form(pointer), "{pointer}");
            assert!(translation_shaped(pointer), "{pointer}");
            assert_eq!(not_a_name(pointer), Some(NotAName::Placeholder), "{pointer}");
        }
        for company in ["Firma AS", "Hansen A/S", "AS Tallinna Vesi", "Acme AS VI.3", "Norconsult AS"] {
            assert!(names_a_legal_form(company), "{company}");
        }
        assert!(!translation_shaped("Acme AS VI.3"));
        // The review: led by a pointer word, so only the form keeps it from the echo.
        for led in ["Se punkt VI.3 Acme AS", "Wie VI.3 Norconsult AS"] {
            assert!(super::pointer_led(led) && names_a_legal_form(led) && !translation_shaped(led), "{led}");
        }
    }

    /// Issue 511 unit 6 item 1 (the units 4+5 reads: `Please see section VI.4.3 Translink`, a review
    /// body and the one real party the combined wet dropped; `See section VI.4.3 Translink` since unit
    /// 2): a pointer stem whose references are followed, with no separator, by a name keeps the party
    /// as an award summary — the text after the references and the form section titles, past a
    /// position, opens with a capitalised word no closed list holds. A title, a position, a function
    /// word or a list word after the reference leaves the pointer a pointer.
    #[test]
    fn a_name_right_after_a_pointers_reference_is_kept() {
        for kept in [
            "Please see section VI.4.3 Translink",
            "See section VI.4.3 Translink",
            "Please see section VI.4.3) Translink",
            "See section VI.4.3 below Translink",
            "See Section VI.3 Kier Construction",
            "See Section VI.3 Additional information Kier Construction",
            "See Section VI.3 Acme plc",
            "Siehe oben VI.4.1 Vergabekammer Südbayern",
            "siehe oben unter Ziffer VI.4.1 Vergabekammer des Bundes",
        ] {
            assert_eq!(not_a_name(kept), Some(NotAName::AwardSummary), "{kept}");
            assert!(super::names_after_stem(kept), "{kept}");
            assert!(mentions_placeholder(kept) && !is_exact_entry(kept), "{kept}: listed, and read before the wet run");
        }
        // No stem: the section-reference grammar refuses the city, so it stays a name, as before.
        for name in ["S. VI.4.1) Weimar Deutschland", "Siehe VI.4.1 Weimar Deutschland"] {
            assert_eq!(not_a_name(name), None, "{name}");
        }
        for pointer in [
            "See Section VI.3 Additional information",
            "please see Section VI.3.2",
            "Please see section VI.3 for list of awarded Suppliers",
            "Please see Section VI.3) for list of awarded suppliers",
            "Please see Section VI.3 Additional Information below",
            "Please see section VI.3) Additional Information for further details",
            "Please see Section 5.2 of the Project Information Memorandum",
            "Please see section VI.2 for full list of 27 Economic Operators",
            "Please see Section VI.3 For details",
            "See section VI.2 Avis",
            "See Section VI.3 Below",
            "Please see section V1.4.2",
            "See section VI.3 - Lot 2",
            "See Contracts Finder Notice for full supplier list",
            "siehe oben unter Ziffer VI.4.1",
            "Siehe oben Nr. 3",
        ] {
            assert_eq!(not_a_name(pointer), Some(NotAName::Placeholder), "{pointer}");
        }
        assert!(super::opens_with_a_name("Translink") && super::opens_with_a_name(" below  Translink"));
        for tail in ["for list of awarded Suppliers", "Section 4", "Notice", "V1 Acme", "X", "  ", "below"] {
            assert!(!super::opens_with_a_name(tail), "{tail:?}");
        }
    }

    /// Issue 511 unit 6, the build review of item 1: a capitalised word right after a pointer's
    /// reference is no body when the lexicon classes the tail by itself, when it is a form label, a
    /// generic noun, a sentence's pronoun or a bare Roman numeral — each read as an award summary by
    /// the first build of item 1, which the fold never drops and the walk never revisits. Three of
    /// them unit 3's wet dropped (`u3/plan-2265.json`: the `Es handelt sich` sentence, 6 mentions;
    /// `Additonal Information, sub-section 2.`; `Ziff. VI.4.1Ort`). Each stays `Placeholder`, as
    /// before unit 6; a body beside each near miss stays an award summary.
    #[test]
    fn a_label_or_a_non_name_right_after_a_pointers_reference_is_no_body() {
        for pointer in [
            // The lexicon's own non-names.
            "See section VI.3 Not applicable",
            "See section VI.3 TBC",
            "See section VI.3 Various",
            "See section VI.3 Withheld",
            "See section VI.3 Confidential",
            "See section VI.3 Unknown",
            "See section VI.3 Nil",
            // Form labels and generic nouns.
            "See section I.1 Name and addresses",
            "See Section V.2.3 Name and address of the contractor",
            "Please see section VI.4.1 Mediation body",
            "See section VI.4.3 Contact details",
            "See section VI.3 Contracts Finder",
            "See section VI.3 Suppliers",
            "See Section VI.3 Lots 1-5",
            "See section VI.3 Annex",
            "See section VI.4.3 Service from which information about the lodging of appeals may be obtained",
            "See Section VI.3 Contract Award Notice",
            // German nouns, always capitalised.
            "siehe oben VI.4.1 Nachprüfungsstelle",
            "siehe oben Ziffer VI.4.1 Zuständige Stelle",
            "siehe oben I.1 Adresse",
            "siehe oben I.1 Ansprechpartner",
            "siehe oben I.1 Name",
            "siehe oben II.1.4 Kurze Beschreibung",
            "siehe oben VI.3 Weitere Angaben",
            // A Roman numeral a glued reference leaves, and unit 3's dropped names.
            "siehe oben Ziff. VI.4.1Ort",
            "Siehe oben Abschnitt II.1.4). Es handelt sich um eine freiwillige EU-Bekanntmachung einer unterschwelligen Dienstleistungskonzession.",
            "See Section VI.3 Additonal Information, sub-section 2.",
        ] {
            assert_eq!(not_a_name(pointer), Some(NotAName::Placeholder), "{pointer}");
            assert!(mentions_placeholder(pointer), "{pointer}");
        }
        for kept in [
            "See section VI.3 NIL d.o.o.",
            "See section VI.4.3 Service Public de Wallonie",
            "Please see section VI.4.1 Mediation Service Northern Ireland",
            "See section VI.4.3 Contact Energy Ltd",
            "siehe oben VI.4.1 Vergabekammer Nordbayern",
            "See section VI.4.3 Vi Bank",
        ] {
            assert_eq!(not_a_name(kept), Some(NotAName::AwardSummary), "{kept}");
        }
        for tail in ["Not applicable", "TBC", "Contract Award Notice", "Lots 1-5", "Suppliers", "VI 4 1Ort", "Es handelt sich um", "Nil"] {
            assert!(!super::opens_with_a_name(tail), "{tail:?}");
        }
        for tail in ["NIL d.o.o.", "Vi Bank", "Translink"] {
            assert!(super::opens_with_a_name(tail), "{tail:?}");
        }
    }

    /// Issue 511 unit 6, the build review of items 2 and 6: the fold's two sweeps read what follows a
    /// copy's last reference ([`super::names_after_reference`]) and whether the seed names its
    /// section's title there ([`super::titled_after_reference`]). Unit 6's openers made these
    /// copies pointer-led, so beside a bare seed of their number the echo swept them; each names a
    /// body after its reference and stays a name by itself.
    #[test]
    fn a_body_named_after_a_copys_reference_is_read_and_a_seeds_title_is_told_apart() {
        use super::{form_section_tokens, names_after_reference, titled_after_reference, translation_shaped};
        for copy in [
            "Glejte oddelek VI.4.1 Državna revizijska komisija",
            "Se référer au point VI.4.1 Conseil d'État",
            "Consultar o ponto VI.4.1 Tribunal Administrativo de Círculo de Lisboa",
            "As set out at VI.4.1 Office of Government Procurement",
            "Glejte oddelek VI.4.1: Državna revizijska komisija",
            // A title the lists miss reads the same: only the seed tells them apart.
            "Se punkt VI.2) Nye oplysninger",
        ] {
            assert_eq!(not_a_name(copy), None, "premise: {copy}");
            assert!(translation_shaped(copy) && !form_section_tokens(copy).is_empty(), "premise: the echo's shape, {copy}");
            assert!(names_after_reference(copy), "{copy}");
        }
        for bare in [
            "Glejte oddelek VI.4.1",
            "Consultar o ponto VI.3 «Informação adicional»",
            "Lásd a VI.3. szakaszt (További információk)",
            "Qqq punkt VI.4.1",
            "siehe oben: Vergabekammer Südbayern (VI.3)",
            "Translink, section VI.4.3",
            "Translink",
        ] {
            assert!(!names_after_reference(bare), "{bare}");
        }
        for titled in ["See Section VI.3 Additional information", "See Section VI.2 'Additional information'.", "See section VI.3 (Additional Information)"] {
            assert!(titled_after_reference(titled), "{titled}");
        }
        for bare in ["See section VI.4.1", "Siehe oben VI.4.1", "Same as in point VI.4.1", "See Additional information VI.3", "Additional information"] {
            assert!(!titled_after_reference(bare), "{bare}");
        }
    }

    /// Issue 511 unit 6 item 2 (the units 4+5 reads' junk kept as a name, and residual dry 2310's
    /// `name` folds): the `As` family's missing verbs, the generic nouns, the notice's names and the
    /// typos the readers listed are closed-list words now, so each spelling is a section-reference
    /// pointer; the residual's forms a sibling copy keeps were pointers already. A body named with
    /// one of the new words stays a name: a noun, a notice word or a position never opens one.
    #[test]
    fn the_as_family_and_the_readers_junk_are_section_reference_pointers() {
        for pointer in [
            "As shown in section I.1 above",
            "As per the body in I.1",
            "As defined in (section) VI.3 of this notice",
            "As defined in VI.3 of this notice",
            "As defined in section VI.3 of this Contract Notice",
            "As set out at I.1",
            "As set out at I.1)",
            "As noted in VI.4.1",
            "As specified in section I.1 and the address/URL details remain the same.",
            "As per those listed in the additional information field VI.3)",
            "As set out above in VI.3) Additional information",
            "As detailed in section II.2.1) of the corresponding Contract Notice",
            "See links detailed in II.1.5",
            "See link in II.2.4",
            "Se référer à l'article VI.3 du présent appel à candidatures",
            "See VI.4.1) Review body field of this Contract Award Notice.",
            "Siehe die unter Ziffer VI.4.1) genannte Stelle.",
            "Wie unter Ziffer I.1) genannte Stelle.",
            "Siehe die unter VI.4.1) bezeichnete Stelle",
            "Refer to instructions contained in Section VI.4.2",
            "Refer to named Contracting Authority in section I.1",
            "Please refer to the answer in section VI.4.2",
            "Zie het gestelde in VI.3",
            "Zie het gestelde in afdeling IV.3.3 van deze aankondiging",
            "Voir informations citées à l'article VI.3) Informations complémentaires",
            "Se référer au point VI.2 autres informations",
            "Come indicato al punto I.1) del presente avviso di gara",
            "Ver el apartado VI.3.2 del presenta anuncio",
            "siehe Kontakstelle unter Punkt I.1)",
            "Siehe Kontaktdaten unter Nr. I.1).",
            "Siehe Abschnittt VI.4.1)",
            "siehe Abschnit I.1",
            "cf rubirque Vi.4.1",
            // Residual dry 2310's `placeholder` folds, kept by a sibling copy: pointers already.
            "Same as in point VI.4.1",
            "Please refer to VI.4.1.",
            "Please refer to VI.3.1",
            "As in point VI.3.1",
            "Same as VI.3.1.",
            "As in VI.4.1)",
            "Voir point VI.4.1)",
        ] {
            assert_eq!(section_reference(pointer), Some(ReferenceShape::Pointer), "{pointer}");
            assert_eq!(not_a_name(pointer), Some(NotAName::Placeholder), "{pointer}");
            assert!(mentions_placeholder(pointer) && !is_exact_entry(pointer), "{pointer}: listed, and read before the wet run");
        }
        for name in [
            "Body Shop International (see VI.3)",
            "Links Communications, see I.1",
            "Stelle für Bauwesen, siehe I.1",
            "Original Factory Shop (see I.1)",
            "Award Notice Services, see VI.3",
            "Het Gestelde Bouwbedrijf, zie VI.3",
            // Not in this item's lists, and no pointer by the grammar: read before the next wet run.
            "As above (Please see IV.4.1)",
            "As per Regulation 50(6)(a) - information withheld for security reasons",
        ] {
            assert!(!section_reference_pointer(name), "{name}");
            assert_eq!(not_a_name(name), None, "{name}");
        }
        // The long UK sentences, whole: exactly as published (the plan's `long` list), each its own
        // exact entry. With the council's name, a different fold, and a name.
        for sentence in [
            "This is an award notice and this opportunity is now closed. Please see section V (Award of Contract) for further details.",
            "This is an award notice and this opportunity is now closed. Please see section V) (Award of Contract) for further details",
            "The name(s) of the winning tenderer(s) are being withheld for security reasons, however these will be made available to the commission on request",
            "The name (s) of the winning tenderers are being withheld for security reasons; however, these will be made available to the commission on request",
            "A full list of successful suppliers will be made available on Contracts Finder. Please see Section VI.3) Additional information.",
        ] {
            assert_eq!(not_a_name(sentence), Some(NotAName::Placeholder), "{sentence}");
            assert!(mentions_placeholder(sentence) && is_exact_entry(sentence), "{sentence}");
        }
        for name in [
            "This is an award notice and this opportunity is now closed. Please see section V (Award of Contract) for further details. Hertfordshire County Council",
            "This is an award notice and this opportunity is now closed. Please see section V (Award of Contract) for further details - Hertfordshire County Council",
            "Hertfordshire County Council — This is an award notice and this opportunity is now closed. Please see section V (Award of Contract) for further details.",
            "This is an award notice and this opportunity is closed. Please see section V (award of contract) for further details. Hertfordshire County Council ",
        ] {
            assert_eq!(not_a_name(name), None, "{name}");
        }
    }

    /// Issue 511 unit 6 item 3 (residual dry 2310's mixed folds `siehe vi 4 1`, `wie vi 4 1`, `voir
    /// section vi 4 2` and `siehe i 1 oder vi 4 1`, whose kept spelling the dry plan's three samples
    /// hid): a space for the second dot (`Wie VI.4 1)`), a space after a dot (`Siehe VI.4. 1)`) or
    /// the numeral spaced off its number (`VI 4.1`) is one reference. Only the last group, of one or
    /// two digits with no dotted group after it, may stand off.
    #[test]
    fn a_spaced_section_reference_is_one_reference() {
        for pointer in [
            "Wie VI.4 1)",
            "Voir section VI.4 2",
            "siehe I.1 oder VI 4.1",
            "Siehe VI.4. 1)",
            "siehe VI 4.1",
            "Siehe VI.4.1. 2)",
        ] {
            assert_eq!(section_reference(pointer), Some(ReferenceShape::Pointer), "{pointer}");
            assert_eq!(not_a_name(pointer), Some(NotAName::Placeholder), "{pointer}");
            assert!(mentions_placeholder(pointer), "the walk's net catches {pointer}");
        }
        for name in [
            // Another group after a spaced one: a date, not a reference.
            "V.1 12.05.2010",
            "Siehe VI.4 2010",
            // A space for a dot only once, right after the first group.
            "Siehe VI.4.1 2",
            "Siehe VI 4.1 2",
            // No dot at all, and a body.
            "Siehe VI 4",
            "Abteilung VI 4.1",
        ] {
            assert!(!section_reference_pointer(name), "{name}");
            assert_eq!(not_a_name(name), None, "{name}");
        }
    }

    /// Issue 511 unit 6 item 4: the dry plan's legal-form read list (the census's case-blind forms,
    /// whose `as` holds the pointer word) leaves out a section-reference pointer — residual dry
    /// 2310's 268 entries held the `As …`/`Same as …` pointers among the real companies — and keeps
    /// every name the census reads a form in otherwise.
    #[test]
    fn the_plans_legal_form_list_leaves_out_section_reference_pointers() {
        use super::plan_legal_form;
        for pointer in [
            "As in I.1 above",
            "AS IN I.1 ABOVE",
            "As VI.4.1",
            "Same as VI.4.1",
            "Same as in point VI.4.1",
            "As shown in section I.1 above",
            "Address as per I.1) above.",
        ] {
            assert!(crate::project::role_census::has_commercial_form(pointer), "premise: the census reads `as` in {pointer}");
            assert!(!plan_legal_form(pointer), "{pointer}");
        }
        for company in [
            "Norconsult AS",
            "Hansen A/S",
            "Acme AS, see VI.3",
            "Gatwick Airport Limited",
            "ENDESA ENERGIA S.A.",
            // Not a pointer by the grammar, so read for its `as`.
            "As per Regulation 50(6)(a) - information withheld for security reasons",
        ] {
            assert!(plan_legal_form(company), "{company}");
        }
    }

    /// Issue 511 unit 6 item 6: notice 19530163's ORG-10 (org 10601576, 140 mentions) publishes `See
    /// Section VI.3 Additional information` in TED's earlier template wording, 24 copies verbatim
    /// (bounded read, 2026-10-12). Each is a section-reference pointer by itself since unit 6 (the
    /// PT `Consultar` and SL `Glejte` openers, the EL, PT, SV and CS titles, the ET, HU and MT
    /// locators), and each has the shape the TED template signal sweeps. The signal's shape takes a
    /// copy whatever its opener when a locator or a title frames its reference; a body named beside a
    /// bare number, a legal form or a long name is no copy.
    #[test]
    fn teds_earlier_template_wordings_are_pointers_and_template_copies() {
        use super::{echo_seed, form_section_tokens, template_copy, template_seed, template_shaped, translation_shaped};
        for copy in [
            "See Section VI.3 Additional information",
            "Se del VI.3 Yderligere oplysninger",
            "Siehe Abschnitt VI.3 Zusätzliche Angaben",
            "Véase el apartado VI.3, «Información adicional»",
            "Katso kohta VI.3 (lisätiedot)",
            "voir point VI.3) Informations complémentaires",
            "Βλέπε τμήμα VI.3 Συμπληρωματικές πληροφορίες",
            "cfr. sezione VI.3) Informazioni complementari",
            "zie VI.3) Nadere inlichtingen",
            "Consultar o ponto VI.3 «Informação adicional»",
            "Se punkt VI.3 Kompletterande upplysningar",
            "Viz oddíl VI.3 Další informace",
            "Vt punkti VI.3 (lisateave)",
            "Lásd a VI.3. szakaszt (További információk)",
            "Žr. VI.3 dalį „Papildoma informacija“",
            "Skatīt VI.3. iedaļu \"Papildu informācija\"",
            "Ara t-Taqsima VI.3 Tagħrif addizzjonali",
            "Zob. sekcja VI.3 Informacje dodatkowe",
            "Pozri oddiel VI.3 Doplňujúce informácie",
            "Glejte oddelek VI.3) Dodatne informacije",
            "Féach Alt VI.3 Faisnéis Bhreise",
            "Вж. раздел VI.3 Допълнителна информация",
            "A se vedea secțiunea VI.3 Informații suplimentare",
            "Vidjeti odjeljak VI.3. Dodatni podaci",
        ] {
            assert_eq!(section_reference(copy), Some(ReferenceShape::Pointer), "{copy}");
            assert_eq!(not_a_name(copy), Some(NotAName::Placeholder), "{copy}");
            assert!(translation_shaped(copy) && template_shaped(copy), "{copy}");
            assert_eq!(form_section_tokens(copy), vec!["vi.3"], "{copy}");
            assert!(mentions_placeholder(copy) && !is_exact_entry(copy), "{copy}");
            // Issue 511 unit 6, the build review: closed-list words after the reference, so no copy
            // names a body there, and the sweeps' guard never spares one.
            assert!(!super::names_after_reference(copy), "{copy}");
        }
        // Seeds: the English stem pointer seeds both; a section-reference pointer seeds only the
        // template signal (ten languages and an English label stand in for the echo's narrowing).
        assert!(echo_seed("See Section VI.3 Additional information") && template_seed("See Section VI.3 Additional information"));
        for seed in ["Same as in point VI.4.1", "Please refer to VI.4.1.", "See VI.4.1"] {
            assert!(!echo_seed(seed) && template_seed(seed), "{seed}");
        }
        for name in ["Various suppliers", "See Section VI: complementary information", "See Section VI.3 Kier Construction"] {
            assert!(!template_seed(name), "{name}: no number, or not a bare placeholder");
        }
        // Whatever the opener: a locator before the reference, or a title or a postposed locator
        // after it. None of these is a pointer alone or a translation to the echo.
        for copy in [
            "Qqq o ponto VI.3 «Informação adicional»",
            "Qqq a VI.3. szakaszt (További információk)",
            "Qqq del VI.3 Yderligere oplysninger",
            "Qqq t-Taqsima VI.3 Tagħrif addizzjonali",
            "Qqq VI.3) Nadere inlichtingen",
        ] {
            assert_eq!(not_a_name(copy), None, "premise: {copy}");
            assert!(!translation_shaped(copy), "premise: {copy} is no translation to the echo");
            assert!(template_shaped(copy), "{copy}");
            assert!(mentions_placeholder(copy), "the walk's net holds {copy}");
        }
        for name in [
            "Referat II.3",
            "Region Hovedstaden VI.3",
            "Abteilung VI.4",
            "LMU München, Referat IV.1",
            "Stadtverwaltung Kaiserslautern - Stabstelle IV.1 Zentrale Vergabestelle",
            "Rheingau-Taunus-Kreis FD I.7",
            "I.4) Avis",
            // A body named before a pointer, or a reference past the opener's words.
            "KIO – jak w pkt VI.3.1)",
            "Vergabekammer des Landes Berlin; siehe Punkt VI.4.1)",
            "Krajowa Izba Odwoławcza – jak wyżej w pkt VI.4.1)",
            "Regierung von Oberbayern Vergabekammer Südbayern Punkt VI.4.1",
            "Vergabekammer Südbayern, Punkt VI.4.1",
            // A supplier before the award heading it spilled.
            "Transports Chalavan. V.1) AWARD AND CONTRACT VALUE:",
            "Bouygues. V.1) Award and contract value",
            // Issue 511 unit 6, the build review: one string per guard of `reference_framed`, which
            // that guard alone refuses — a pointer phrase in the opener (a body named before it), a
            // separator in the opener, a one-letter locator, and a reference past four opener words.
            "Vergabekammer siehe Punkt VI.4.1",
            "KIO jak w pkt VI.3.1)",
            "VKS – Punkt VI.4.1",
            "Marknadsdomstolen – punkt VI.4.1",
            "Acme P VI.3",
            "the awarded suppliers are listed in section VI.2",
        ] {
            assert!(!template_shaped(name), "{name}");
            assert!(!mentions_placeholder(name), "{name}: the net stays narrow");
        }
        // A legal form, or more than twelve words: no copy, so never counted and never swept.
        for name in [
            "Consultar o ponto VI.3 Acme Lda",
            "Qqq o ponto VI.3 Acme GmbH",
            "Qqq o ponto VI.3 «Informação adicional» para todos os lotes deste concurso público internacional",
        ] {
            assert!(!template_copy(name) && !template_shaped(name), "{name}");
        }
    }

    /// Issue 511 unit 5 item C: the walk's net is a superset of the fold's drops, name by name.
    /// Every string literal in this module's tests and in the projection's — the drop tests' every
    /// specimen, prod spellings and TED translations included — that the fold classes
    /// `Placeholder` or `Blank` in any language the scope reads is a hit of
    /// [`mentions_placeholder`]; every `VoidLot` one is a hit of [`mentions_void`] (510's net). The
    /// echo's sweeps are pinned per party by the projection's harness (`placeholder_511::decided`).
    #[test]
    fn the_walks_net_holds_every_name_the_fold_drops() {
        let (mut placeholder, mut void, mut swept) = (0, 0, 0);
        for (file, src) in [("partyname.rs", include_str!("partyname.rs")), ("project.rs", include_str!("project.rs"))] {
            let tests = &src[src.find("#[cfg(test)]\nmod tests").unwrap_or_else(|| panic!("{file} has a tests module"))..];
            for literal in string_literals(tests) {
                // Issue 511 unit 6 item 6: a name the fold's echo or its TED template signal can sweep
                // (a form section number in a translation's or a template copy's shape) is a hit too,
                // whatever its own class: the party's other copies decide the sweep, and the walk reads
                // the head name alone.
                if !super::form_section_tokens(&literal).is_empty()
                    && (super::translation_shaped(&literal) || super::template_shaped(&literal))
                {
                    assert!(mentions_placeholder(&literal), "{file}: the fold can sweep {literal:?}, the net misses it");
                    swept += 1;
                }
                for lang in [None, Some("DAN"), Some("SWE"), Some("NOR"), Some("POL"), Some("ITA")] {
                    match not_a_name_in(&literal, lang) {
                        Some(NotAName::Placeholder | NotAName::Blank) => {
                            assert!(mentions_placeholder(&literal), "{file}: the fold drops {literal:?} ({lang:?}), the net misses it");
                            placeholder += 1;
                        }
                        Some(NotAName::VoidLot) => {
                            assert!(mentions_void(&literal), "{file}: the fold drops {literal:?} as void, 510's net misses it");
                            void += 1;
                        }
                        _ => {}
                    }
                }
            }
        }
        // The scan read the drop tests at all (every literal is read in six languages).
        assert!(placeholder > 6 * 400 && void > 6 * 100, "{placeholder} placeholder, {void} void readings");
        assert!(swept > 300, "{swept} sweepable readings");
    }

    /// The string literals of Rust source `src`, unescaped: `"…"` with `\"`, `\\`, `\n`, `\t`, `\'`,
    /// `\0`, `\u{…}` and line continuations, and raw `r"…"` / `r#"…"#`. A char literal `'"'` opens
    /// no string. For [`the_walks_net_holds_every_name_the_fold_drops`].
    fn string_literals(src: &str) -> Vec<String> {
        let chars: Vec<char> = src.chars().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            match chars[i] {
                '\'' if chars.get(i + 1) == Some(&'"') && chars.get(i + 2) == Some(&'\'') => i += 3,
                '\'' if chars.get(i + 1) == Some(&'\\') && chars.get(i + 3) == Some(&'\'') => i += 4,
                '/' if chars.get(i + 1) == Some(&'/') => {
                    while i < chars.len() && chars[i] != '\n' {
                        i += 1;
                    }
                }
                'r' if (chars.get(i + 1) == Some(&'"') || chars.get(i + 1) == Some(&'#'))
                    && !chars.get(i.wrapping_sub(1)).is_some_and(|c| c.is_alphanumeric() || *c == '_') =>
                {
                    let hashes = chars[i + 1..].iter().take_while(|c| **c == '#').count();
                    if chars.get(i + 1 + hashes) != Some(&'"') {
                        i += 1;
                        continue;
                    }
                    let start = i + 2 + hashes;
                    let mut j = start;
                    while j < chars.len() && !(chars[j] == '"' && chars[j + 1..].iter().take(hashes).filter(|c| **c == '#').count() == hashes) {
                        j += 1;
                    }
                    out.push(chars[start..j.min(chars.len())].iter().collect());
                    i = j + 1 + hashes;
                }
                '"' => {
                    let mut s = String::new();
                    i += 1;
                    while i < chars.len() && chars[i] != '"' {
                        if chars[i] == '\\' {
                            i += 1;
                            match chars.get(i) {
                                Some('n') => s.push('\n'),
                                Some('t') => s.push('\t'),
                                Some('0') => s.push('\0'),
                                Some('u') => {
                                    let close = (i..chars.len()).find(|&k| chars[k] == '}').expect("a closed \\u{…}");
                                    let hex: String = chars[i + 2..close].iter().collect();
                                    s.push(char::from_u32(u32::from_str_radix(&hex, 16).expect("hex")).expect("a char"));
                                    i = close;
                                }
                                Some('\n') => {
                                    while chars.get(i + 1).is_some_and(|c| c.is_whitespace()) {
                                        i += 1;
                                    }
                                }
                                Some(c) => s.push(*c),
                                None => {}
                            }
                        } else {
                            s.push(chars[i]);
                        }
                        i += 1;
                    }
                    out.push(s);
                    i += 1;
                }
                _ => i += 1,
            }
        }
        out
    }

    /// Issue 511 unit 5 item E, the SIMULATION: every `non_exact` fold of a
    /// `refold-placeholder-names` dry plan (dry 2283's,
    /// `.scratch/tender-db/511-placeholders/u4/plan-dry.json`, a job row whose `body` is the plan)
    /// re-classified spelling by spelling with this build's [`not_a_name`], and the narrowed net
    /// ([`mentions_placeholder`]) re-run on every spelling ([`simulate_plan`]). Writes
    /// `simulation.json` and `simulation.md` to `ISSUE_511_SIM_OUT`. Run with ABSOLUTE paths (cargo
    /// runs the test from `crates/ingest`) and the gate's flags and package set (CLAUDE.md):
    /// `ISSUE_511_PLAN=/home/user/tender-db/.scratch/tender-db/511-placeholders/u4/plan-dry.json
    /// ISSUE_511_SIM_OUT=/home/user/tender-db/.scratch/tender-db/511-placeholders/u5-build
    /// CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model -p
    /// store -p ingest -p tender-db --features tender-db/server -- issue_511_unit_5_simulation
    /// --ignored`. Since issue 511 unit 6 it reads with the current build, unit 6 included
    /// ([`issue_511_unit_6_simulation`] runs several plans).
    #[test]
    #[ignore = "reads the dry plan named by ISSUE_511_PLAN and writes ISSUE_511_SIM_OUT"]
    fn issue_511_unit_5_simulation() {
        let path = std::env::var("ISSUE_511_PLAN").expect("ISSUE_511_PLAN");
        let out_dir = std::env::var("ISSUE_511_SIM_OUT").expect("ISSUE_511_SIM_OUT");
        let (report, md) = simulate_plan(&path, None);
        std::fs::create_dir_all(&out_dir).expect("the output directory");
        std::fs::write(format!("{out_dir}/simulation.json"), serde_json::to_string_pretty(&report).expect("JSON") + "\n").expect("write simulation.json");
        std::fs::write(format!("{out_dir}/simulation.md"), format!("# Issue 511 — simulation over one dry plan\n\n{md}")).expect("write simulation.md");
    }

    /// Issue 511 unit 6, the SIMULATION over several dry plans: `ISSUE_511_PLANS` holds `;`-separated
    /// entries `label|plan[|prior]` — a `refold-placeholder-names` job row, and optionally a prior
    /// simulation's `simulation.json`, whose `changed` folds give the classes the prior build read
    /// where the plan's own are older (dry 2283 was planned by unit 4; unit 5's simulation of it holds
    /// unit 5's). Each plan is read by [`simulate_plan`]; `simulation.json` and `simulation.md` go to
    /// `ISSUE_511_SIM_OUT`. Unit 6's run:
    /// `ISSUE_511_PLANS='residual dry 2310|/home/user/tender-db/.scratch/tender-db/511-placeholders/
    /// u45/plan-residual.json;dry 2283|/home/user/tender-db/.scratch/tender-db/511-placeholders/u4/
    /// plan-dry.json|/home/user/tender-db/.scratch/tender-db/511-placeholders/u5-build/
    /// simulation.json' ISSUE_511_SIM_OUT=/home/user/tender-db/.scratch/tender-db/511-placeholders/
    /// u6-build`, with the gate's flags and package set (CLAUDE.md) and `-- issue_511_unit_6_simulation
    /// --ignored`.
    #[test]
    #[ignore = "reads the dry plans named by ISSUE_511_PLANS and writes ISSUE_511_SIM_OUT"]
    fn issue_511_unit_6_simulation() {
        use serde_json::{Value, json};
        let plans = std::env::var("ISSUE_511_PLANS").expect("ISSUE_511_PLANS");
        let out_dir = std::env::var("ISSUE_511_SIM_OUT").expect("ISSUE_511_SIM_OUT");
        let mut reports: Vec<Value> = Vec::new();
        let mut md = String::from(
            "# Issue 511 unit 6 — simulation over residual dry 2310 and dry 2283\n\n\
             Generated by `partyname::tests::issue_511_unit_6_simulation`. Every `non_exact` fold of each plan is re-classified \
             spelling by spelling with unit 6's `not_a_name` (items 1–3: the no-separator keep, the closed-list words, the spaced \
             references; item 6's openers and titles) and re-run through unit 6's walk net (`mentions_placeholder`, whose echo arm \
             now holds the TED template signal's shape). The TED template signal itself decides per party, across a notice's \
             copies, which a plan (head names only) does not hold: its effect is not simulated here, only its net arm.\n\n",
        );
        for entry in plans.split(';').filter(|e| !e.trim().is_empty()) {
            let mut parts = entry.split('|');
            let label = parts.next().expect("a label").trim();
            let path = parts.next().expect("a plan path").trim();
            let prior = parts.next().map(str::trim).filter(|p| !p.is_empty());
            let (report, section) = simulate_plan(path, prior);
            md.push_str(&format!("## {label}\n\n{section}\n"));
            reports.push(json!({ "label": label, "report": report }));
        }
        std::fs::create_dir_all(&out_dir).expect("the output directory");
        std::fs::write(format!("{out_dir}/simulation.json"), serde_json::to_string_pretty(&json!({ "plans": reports })).expect("JSON") + "\n")
            .expect("write simulation.json");
        std::fs::write(format!("{out_dir}/simulation.md"), md).expect("write simulation.md");
        for r in &reports {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "label": r["label"], "transitions": r["report"]["transitions"], "net": r["report"]["net"],
                    "superset_misses": r["report"]["superset_misses"].as_array().map(Vec::len),
                    "changed": r["report"]["changed"].as_array().map(Vec::len),
                    "legal_form_dropped": r["report"]["legal_form_dropped"].as_array().map(Vec::len),
                }))
                .expect("JSON")
            );
        }
    }

    /// The simulation of one `refold-placeholder-names` dry plan (`path`, a job row whose `body` is the
    /// plan) for [`issue_511_unit_5_simulation`] and [`issue_511_unit_6_simulation`]: every `non_exact`
    /// fold re-classified spelling by spelling with this build's [`not_a_name`], against the prior
    /// class — the plan's own, or a prior simulation's (`prior`, its `changed` folds' `to`) — and the
    /// walk's net ([`mentions_placeholder`]) re-run on every spelling. Returns the JSON report and its
    /// markdown: per class transition the folds, orgs, mentions, buyer rows and role mix; every fold
    /// that changes class; what the net would re-queue and its buyer rows, beside the net without the
    /// TED template signal's framed shape (issue 511 unit 6 item 6: [`super::template_shaped`] over
    /// [`super::translation_shaped`]) and the class arms alone; the net's superset check on the plan's
    /// own spellings; and the plan's `legal_form` entries this build's list leaves out
    /// ([`super::plan_legal_form`]).
    ///
    /// The plan counts orgs per fold, not per spelling, so a fold whose spellings split is counted
    /// apart (`split`); the net's figures add the plan's exact entries (no `non_exact` fold: whole
    /// values and bare stems, all still hits, their class unchanged) to the re-run folds. A name no
    /// earlier net caught is in no plan, so a hit beyond the plan's net (a reference glued to its
    /// locator, `pkt.VI.4.1`) is not counted here.
    fn simulate_plan(path: &str, prior: Option<&str>) -> (serde_json::Value, String) {
        use serde_json::{Value, json};
        use std::collections::{BTreeMap, BTreeSet};
        use std::fmt::Write as _;
        let job: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("read the plan")).expect("a JSON job row");
        let plan: Value = match &job["body"] {
            Value::String(body) => serde_json::from_str(body).expect("the plan body"),
            other => other.clone(),
        };
        let prior_sim: Option<Value> =
            prior.map(|p| serde_json::from_str(&std::fs::read_to_string(p).expect("read the prior simulation")).expect("a JSON simulation"));
        let prior_classes: BTreeMap<String, String> = prior_sim
            .iter()
            .flat_map(|sim| sim["changed"].as_array().into_iter().flatten())
            .filter_map(|c| Some((c["fold"].as_str()?.to_owned(), c["to"].as_str()?.to_owned())))
            .collect();
        let class_name = |class: Option<NotAName>| match class {
            None => "name",
            Some(NotAName::VoidLot) => "void",
            Some(NotAName::Placeholder) => "placeholder",
            Some(NotAName::Blank) => "blank",
            Some(NotAName::AwardSummary) => "award_summary",
        };
        let count = |v: &Value| v.as_u64().unwrap_or(0);
        let buyer_rows = |roles: &Value| count(&roles["buyer"]) + count(&roles["Procedure-Buyer"]);
        #[derive(Default)]
        struct Tally {
            folds: u64,
            orgs: u64,
            mentions: u64,
            buyer_rows: u64,
            roles: BTreeMap<String, u64>,
        }
        impl Tally {
            fn add(&mut self, orgs: u64, mentions: u64, roles: &Value) {
                self.folds += 1;
                self.orgs += orgs;
                self.mentions += mentions;
                for (role, n) in roles.as_object().into_iter().flatten() {
                    *self.roles.entry(role.clone()).or_default() += n.as_u64().unwrap_or(0);
                }
                self.buyer_rows = self.roles.get("buyer").copied().unwrap_or(0) + self.roles.get("Procedure-Buyer").copied().unwrap_or(0);
            }
            fn json(&self) -> Value {
                let mut roles: Vec<(&String, &u64)> = self.roles.iter().collect();
                roles.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
                json!({
                    "folds": self.folds, "orgs": self.orgs, "mentions": self.mentions, "buyer_rows": self.buyer_rows,
                    "roles": roles.into_iter().map(|(r, n)| json!([r, n])).collect::<Vec<_>>(),
                })
            }
            fn roles_line(&self, top: usize) -> String {
                let mut roles: Vec<(&String, &u64)> = self.roles.iter().collect();
                roles.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
                roles.iter().take(top).map(|(r, n)| format!("{r} {n}")).collect::<Vec<_>>().join(", ")
            }
        }
        let folds = plan["non_exact"].as_array().expect("non_exact");
        let mut transitions: BTreeMap<(String, String), Tally> = BTreeMap::new();
        let mut changed: Vec<Value> = Vec::new();
        let mut shapes: BTreeMap<&str, Tally> = BTreeMap::new();
        // The net over the re-run folds: every spelling a hit and re-queued (not an award summary),
        // none, or split; this build's net, the net without the template signal's framed shape (the
        // class arms and the echo's pointer-led shape), and the class arms alone.
        let (mut plan_requeued, mut net_requeued, mut unframed_requeued, mut class_requeued, mut split_net) =
            (Tally::default(), Tally::default(), Tally::default(), Tally::default(), Tally::default());
        let (mut echo_only, mut framed_only) = (Tally::default(), Tally::default());
        let mut framed_folds: Vec<Value> = Vec::new();
        // The re-queued folds that hold buyer rows, for the buyer canary's reading.
        let mut buyer_folds: Vec<Value> = Vec::new();
        let mut new_by_class: BTreeMap<String, Tally> = BTreeMap::new();
        let (mut spellings, mut superset_misses): (u64, Vec<String>) = (0, Vec::new());
        for fold in folds {
            let names: Vec<&str> = fold["names"].as_array().expect("names").iter().filter_map(Value::as_str).collect();
            let (orgs, mentions, roles) = (count(&fold["orgs"]), count(&fold["mentions"]), &fold["roles"]);
            let plan_key = fold["classes"].as_array().expect("classes").iter().filter_map(Value::as_str).collect::<Vec<_>>().join("+");
            let fold_key = fold["fold"].as_str().unwrap_or("");
            let old_key = prior_classes.get(fold_key).cloned().unwrap_or_else(|| plan_key.clone());
            let classes: Vec<Option<NotAName>> = names.iter().map(|n| not_a_name(n)).collect();
            let new: BTreeSet<&str> = classes.iter().map(|c| class_name(*c)).collect();
            let new_key = new.iter().copied().collect::<Vec<_>>().join("+");
            transitions.entry((old_key.clone(), new_key.clone())).or_default().add(orgs, mentions, roles);
            new_by_class.entry(new_key.clone()).or_default().add(orgs, mentions, roles);
            let shape: BTreeSet<&str> = names
                .iter()
                .map(|n| match section_reference(n) {
                    Some(ReferenceShape::Pointer) => "pointer",
                    Some(ReferenceShape::Label) => "label",
                    None => "none",
                })
                .collect();
            if shape.len() == 1 && !shape.contains("none") {
                shapes.entry(shape.iter().next().copied().unwrap_or("none")).or_default().add(orgs, mentions, roles);
            }
            if old_key != new_key {
                changed.push(json!({
                    "fold": fold["fold"], "from": old_key, "to": new_key, "shape": shape.iter().collect::<Vec<_>>(),
                    "names": names, "orgs": orgs, "mentions": mentions, "roles": roles,
                }));
            }
            if plan_key != "award_summary" {
                plan_requeued.add(orgs, mentions, roles);
            }
            let tokens = |n: &str| !super::form_section_tokens(n).is_empty();
            let hits: Vec<bool> = names.iter().map(|n| mentions_placeholder(n)).collect();
            let class_hits: Vec<bool> = names.iter().map(|n| super::mentions_by_class(n)).collect();
            let unframed_hits: Vec<bool> =
                names.iter().zip(&class_hits).map(|(n, c)| *c || (tokens(n) && super::translation_shaped(n))).collect();
            let requeued = |hit: &[bool]| -> Option<bool> {
                let kept = hit.iter().zip(&classes).map(|(h, c)| *h && *c != Some(NotAName::AwardSummary)).collect::<Vec<_>>();
                if kept.iter().all(|k| *k) {
                    Some(true)
                } else if kept.iter().all(|k| !*k) {
                    Some(false)
                } else {
                    None
                }
            };
            if requeued(&hits) != Some(false) && buyer_rows(roles) > 0 {
                buyer_folds.push(json!({
                    "fold": fold["fold"], "class": new_key, "names": names, "orgs": orgs, "buyer_rows": buyer_rows(roles),
                    "echo_arm_only": hits.iter().zip(&class_hits).all(|(h, c)| *h && !*c),
                }));
            }
            match requeued(&hits) {
                Some(true) => net_requeued.add(orgs, mentions, roles),
                Some(false) => {}
                None => split_net.add(orgs, mentions, roles),
            }
            if requeued(&unframed_hits) == Some(true) {
                unframed_requeued.add(orgs, mentions, roles);
            }
            if requeued(&class_hits) == Some(true) {
                class_requeued.add(orgs, mentions, roles);
            }
            if hits.iter().zip(&class_hits).all(|(h, c)| *h && !*c) {
                echo_only.add(orgs, mentions, roles);
            }
            if requeued(&hits) == Some(true) && requeued(&unframed_hits) != Some(true) {
                framed_only.add(orgs, mentions, roles);
                framed_folds.push(json!({ "fold": fold["fold"], "class": new_key, "names": names, "orgs": orgs, "mentions": mentions, "buyer_rows": buyer_rows(roles) }));
            }
            for ((name, class), hit) in names.iter().zip(&classes).zip(&hits) {
                spellings += 1;
                if matches!(class, Some(NotAName::Placeholder | NotAName::Blank)) && !hit {
                    superset_misses.push((*name).to_owned());
                }
            }
        }
        // The plan's exact entries: its re-queued orgs and buyer rows less the re-run folds' (by the
        // plan's own classes, which its net split by).
        let (plan_orgs, plan_buyer_rows) = (count(&plan["orgs"]), count(&plan["buyer_rows"]));
        let exact_orgs = plan_orgs.saturating_sub(plan_requeued.orgs);
        let exact_buyer_rows = plan_buyer_rows.saturating_sub(plan_requeued.buyer_rows);
        let legal_form: Vec<&str> = plan["legal_form"].as_array().into_iter().flatten().filter_map(|o| o["name"].as_str()).collect();
        let legal_form_dropped: Vec<&str> = legal_form.iter().copied().filter(|n| !super::plan_legal_form(n)).collect();
        changed.sort_by(|a, b| count(&b["orgs"]).cmp(&count(&a["orgs"])).then(count(&b["mentions"]).cmp(&count(&a["mentions"]))));
        framed_folds.sort_by(|a, b| count(&b["orgs"]).cmp(&count(&a["orgs"])));
        let net = json!({
            "plan_requeued_orgs": plan_orgs,
            "plan_buyer_rows": plan_buyer_rows,
            "prior_simulation_net": prior_sim.as_ref().map(|sim| sim["net"].clone()),
            "exact_entry_orgs": exact_orgs,
            "exact_entry_buyer_rows_estimate": exact_buyer_rows,
            "non_exact_requeued_by_plan_classes": plan_requeued.json(),
            "non_exact_requeued_by_this_net": net_requeued.json(),
            "non_exact_requeued_without_the_framed_shape": unframed_requeued.json(),
            "non_exact_split_folds": split_net.json(),
            "non_exact_requeued_by_class_arms_only": class_requeued.json(),
            "non_exact_echo_arm_only": echo_only.json(),
            "non_exact_framed_shape_only": framed_only.json(),
            "net_orgs": exact_orgs + net_requeued.orgs,
            "net_orgs_with_split_folds": exact_orgs + net_requeued.orgs + split_net.orgs,
            "net_buyer_rows_estimate": exact_buyer_rows + net_requeued.buyer_rows,
            "net_buyer_rows_with_split_folds": exact_buyer_rows + net_requeued.buyer_rows + split_net.buyer_rows,
            "net_without_framed_orgs": exact_orgs + unframed_requeued.orgs,
            "net_without_framed_buyer_rows": exact_buyer_rows + unframed_requeued.buyer_rows,
            "class_arms_only_orgs": exact_orgs + class_requeued.orgs,
            "class_arms_only_buyer_rows": exact_buyer_rows + class_requeued.buyer_rows,
        });
        let report = json!({
            "plan": path,
            "prior": prior,
            "non_exact_folds": folds.len(),
            "spellings": spellings,
            "transitions": transitions.iter().map(|((from, to), t)| json!({"from": from, "to": to, "tally": t.json()})).collect::<Vec<_>>(),
            "non_exact_by_new_class": new_by_class.iter().map(|(c, t)| json!({"class": c, "tally": t.json()})).collect::<Vec<_>>(),
            "section_reference_shapes": shapes.iter().map(|(s, t)| json!({"shape": s, "tally": t.json()})).collect::<Vec<_>>(),
            "net": net,
            "superset_misses": superset_misses,
            "net_buyer_folds": buyer_folds,
            "framed_shape_folds": framed_folds,
            "legal_form_orgs": legal_form.len(),
            "legal_form_dropped": legal_form_dropped,
            "changed": changed,
        });
        let mut md = String::new();
        let _ = writeln!(
            md,
            "Plan `{path}`{}: {} `non_exact` folds, {spellings} spellings. The `from` class is {}.\n",
            prior.map(|p| format!(", prior classes from `{p}`")).unwrap_or_default(),
            folds.len(),
            if prior.is_some() { "the prior simulation's (its `changed` folds), else the plan's" } else { "the plan's own" },
        );
        let _ = writeln!(md, "### Class transitions (fold classes, prior → this build)\n");
        let _ = writeln!(md, "| from | to | folds | orgs | mentions | buyer rows | role mix (top 6) |\n|---|---|---:|---:|---:|---:|---|");
        for ((from, to), t) in &transitions {
            let _ = writeln!(md, "| {from} | {to} | {} | {} | {} | {} | {} |", t.folds, t.orgs, t.mentions, t.buyer_rows, t.roles_line(6));
        }
        let _ = writeln!(md, "\n### Non-exact folds by this build's class\n");
        let _ = writeln!(md, "| class | folds | orgs | mentions | buyer rows |\n|---|---:|---:|---:|---:|");
        for (class, t) in &new_by_class {
            let _ = writeln!(md, "| {class} | {} | {} | {} | {} |", t.folds, t.orgs, t.mentions, t.buyer_rows);
        }
        let _ = writeln!(md, "\n### Section-reference shapes (folds whose every spelling has the shape)\n");
        let _ = writeln!(md, "| shape | folds | orgs | mentions | buyer rows | role mix (top 6) |\n|---|---:|---:|---:|---:|---|");
        for (shape, t) in &shapes {
            let _ = writeln!(md, "| {shape} | {} | {} | {} | {} | {} |", t.folds, t.orgs, t.mentions, t.buyer_rows, t.roles_line(6));
        }
        let _ = writeln!(md, "\n### The walk's net\n");
        let _ = writeln!(md, "| | orgs | buyer rows |\n|---|---:|---:|");
        let _ = writeln!(md, "| the plan's net (its re-queued orgs, `buyer_rows`) | {plan_orgs} | {plan_buyer_rows} |");
        if let Some(prior_net) = prior_sim.as_ref().map(|sim| &sim["net"]) {
            let _ = writeln!(
                md,
                "| the prior simulation's net (`unit_5_net_orgs`, buyer rows estimate) | {} | {} |",
                count(&prior_net["unit_5_net_orgs"]),
                count(&prior_net["unit_5_net_buyer_rows_estimate"])
            );
        }
        let _ = writeln!(md, "| exact entries (outside `non_exact`, unchanged) | {exact_orgs} | {exact_buyer_rows} (estimate) |");
        let _ = writeln!(md, "| `non_exact` re-queued by the plan's classes | {} | {} |", plan_requeued.orgs, plan_requeued.buyer_rows);
        let _ = writeln!(md, "| `non_exact` re-queued by this build's net | {} | {} |", net_requeued.orgs, net_requeued.buyer_rows);
        let _ = writeln!(md, "| `non_exact` folds whose spellings split | {} | {} |", split_net.orgs, split_net.buyer_rows);
        let _ = writeln!(
            md,
            "| **this build's net** (exact + re-queued; + split) | **{}** (+{}) | **{}** (+{}) |",
            exact_orgs + net_requeued.orgs,
            split_net.orgs,
            exact_buyer_rows + net_requeued.buyer_rows,
            split_net.buyer_rows
        );
        let _ = writeln!(
            md,
            "| the net without the template signal's framed shape (class arms + pointer-led echo arm) | {} | {} |",
            exact_orgs + unframed_requeued.orgs,
            exact_buyer_rows + unframed_requeued.buyer_rows
        );
        let _ = writeln!(md, "| the class arms alone (no echo arm) | {} | {} |", exact_orgs + class_requeued.orgs, exact_buyer_rows + class_requeued.buyer_rows);
        let _ = writeln!(md, "| of the net: echo arm only (a sweepable shape, still a name) | {} | {} |", echo_only.orgs, echo_only.buyer_rows);
        let _ = writeln!(md, "| of the net: the framed shape only (unit 6 item 6's cost) | {} | {} |", framed_only.orgs, framed_only.buyer_rows);
        let _ = writeln!(
            md,
            "\nSuperset check on the plan's own spellings: {} of {spellings} spellings are `Placeholder`/`Blank` and missed by the net.\n",
            superset_misses.len()
        );
        for miss in &superset_misses {
            let _ = writeln!(md, "- MISS `{miss}`");
        }
        let names_cell = |f: &Value| -> String {
            f["names"].as_array().into_iter().flatten().filter_map(Value::as_str).map(|n| format!("`{}`", n.replace('|', "\\|"))).collect::<Vec<_>>().join(" · ")
        };
        let _ = writeln!(md, "Folds the framed shape alone re-queues ({}):\n", framed_folds.len());
        for f in &framed_folds {
            let _ = writeln!(md, "- {} org(s), {} mention(s), {} buyer row(s), {}: {}", count(&f["orgs"]), count(&f["mentions"]), count(&f["buyer_rows"]), f["class"].as_str().unwrap_or(""), names_cell(f));
        }
        let _ = writeln!(md, "\nRe-queued folds with buyer rows ({}):\n", buyer_folds.len());
        for f in &buyer_folds {
            let _ = writeln!(
                md,
                "- {} buyer row(s), {} org(s), {}{}: {}",
                count(&f["buyer_rows"]),
                count(&f["orgs"]),
                f["class"].as_str().unwrap_or(""),
                if f["echo_arm_only"].as_bool() == Some(true) { " (echo arm only)" } else { "" },
                names_cell(f)
            );
        }
        let _ = writeln!(
            md,
            "\nLegal-form list (issue 511 unit 6 item 4): {} of the plan's {} `legal_form` orgs leave this build's list (`plan_legal_form`: section-reference pointers): {}\n",
            legal_form_dropped.len(),
            legal_form.len(),
            legal_form_dropped.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", ")
        );
        let _ = writeln!(md, "### Every fold that changes class ({})\n", changed.len());
        let _ = writeln!(md, "| orgs | mentions | from → to | shape | fold | spellings | roles |\n|---:|---:|---|---|---|---|---|");
        let cell = |s: &str| s.replace('|', "\\|").replace('\n', " ");
        for c in &changed {
            let roles: Vec<String> = c["roles"].as_object().into_iter().flatten().map(|(r, n)| format!("{r} {n}")).collect();
            let shape: Vec<&str> = c["shape"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
            let _ = writeln!(
                md,
                "| {} | {} | {} → {} | {} | `{}` | {} | {} |",
                count(&c["orgs"]),
                count(&c["mentions"]),
                c["from"].as_str().unwrap_or(""),
                c["to"].as_str().unwrap_or(""),
                shape.join("+"),
                cell(c["fold"].as_str().unwrap_or("")),
                names_cell(c),
                roles.join(", ")
            );
        }
        (report, md)
    }
}
