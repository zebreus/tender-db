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
//! company with that exact fold. Issue 511's entries are its 2026-10-11 census (§4), with
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
/// 6. the whole values: 510's void ones first (with its reason and prefix rules), then
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

/// Whether `name` says the lot was not awarded ([`NotAName::VoidLot`]) — the fold's rule,
/// as a plain predicate for injection (issue 510's `refold-void-names` org walk).
pub fn is_void_lot(name: &str) -> bool {
    not_a_name(name) == Some(NotAName::VoidLot)
}

/// Whether `name` carries a void-lot stem or whole value at all, before the award
/// exemption ([`names_an_award`]) — the `refold-void-names` walk's net, so its dry plan
/// can list what the exemption keeps beside what the fold drops.
pub fn mentions_void(name: &str) -> bool {
    let full = fold(name);
    if full.is_empty() {
        return false;
    }
    let padded = format!(" {full}");
    if VOID_STEMS.iter().any(|stem| padded.contains(stem)) {
        return true;
    }
    let stripped = strip_lot_qualifier(name).map(fold);
    VOID_WHOLE.contains(&stripped.as_deref().unwrap_or(&full))
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
/// - an awardee noun not negated ([`AWARD_NOUNS`]: `mandataire`, `titulaire`, …);
/// - [`several_lots`]: a per-lot summary (`Lot 1) X. Lot 2) Infructueux`);
/// - [`mixed_segments`]: `;`-separated entries, one with no void phrase (`zone A : Firm ;
///   zone B : infructueux`);
/// - a company form among its raw tokens ([`COMPANY_TOKENS`]).
///
/// Each errs toward keeping the party: a summary of void lots only (`Les lots 2 et 11 sont
/// déclarés sans suite. Le lot 10 est déclaré infructueux`) stays an organization for issue
/// 511, which costs a junk name and never a real award.
fn names_an_award(raw: &str, folded: &str) -> bool {
    let words: Vec<&str> = folded.split(' ').collect();
    let padded = format!(" {folded}");
    award_clause(folded)
        || awardee_noun(&words)
        || several_lots(&words)
        || mixed_segments(raw)
        || company_token(raw)
        || polish_company_form(&padded)
        || AWARD_PHRASES.iter().any(|phrase| padded.contains(phrase))
        || only_an_earlier_void(&padded)
        || SUPPLIER_HEADINGS.iter().any(|heading| {
            padded.find(heading).is_some_and(|at| padded[at + heading.len()..].chars().any(char::is_alphabetic))
        })
        || address(raw, &words)
        || numbered_entries(raw)
        || lettered_entries(raw)
        || rebate(raw)
        || named_before_spill(folded)
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
/// head before `section vi` names a party when it carries no void stem.
fn named_before_spill(folded: &str) -> bool {
    let padded = format!(" {folded} ");
    padded.find(" section vi ").is_some_and(|at| {
        let head = &padded[..at];
        head.chars().any(char::is_alphabetic) && !VOID_STEMS.iter().any(|stem| head.contains(stem))
    })
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
    words.iter().enumerate().any(|(i, w)| {
        AWARD_NOUNS.contains(w)
            && !words[..i]
                .iter()
                .rev()
                .find(|b| !NEGATION_FILLERS.contains(*b))
                .is_some_and(|b| NEGATIONS.contains(b))
    })
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
                && ![" relanc", " nouvelle consultation", " nueva licitaci", " nuova gara", " nuevo procedimiento"]
                    .iter()
                    .any(|note| padded.contains(note))
        })
}

/// Whether a folded void phrase goes on to say the lot was awarded after all: an
/// `attribué` form not right after `non` / `pas` / `sans`, or `avec la société`.
fn award_clause(folded: &str) -> bool {
    let words: Vec<&str> = folded.split(' ').collect();
    let awarded = words.iter().enumerate().any(|(i, w)| {
        matches!(*w, "attribue" | "attribuee" | "attribues" | "attribuees" | "attribuer")
            && !(i > 0 && matches!(words[i - 1], "non" | "pas" | "sans"))
    });
    let padded = format!(" {folded} ");
    awarded
        || [" avec la societe ", " avec l entreprise ", " avec les societes "].iter().any(|p| padded.contains(p))
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

#[cfg(test)]
mod tests {
    use super::{
        BLANK_STEMS, BLANK_WHOLE, FORM_SECTION_TITLES, NO_INFORMATION_TAILS, NotAName, PERFIL_STEMS, PLACEHOLDER_STEMS,
        PLACEHOLDER_WHOLE, POINTER_LEAD_INS, POINTER_STEMS, REFERENCE_HEAD_WORDS, REFERENCE_LISTS, REFERENCE_LEAD_INS,
        ReferenceShape, SCOPED_WHOLE, VOID_STEMS, VOID_WHOLE, fold, is_exact_entry, mentions_placeholder, mentions_void,
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
            "Société Sindarro Lot déclaré infructueux relancé en marché négocié avec publicité",
            "Procédure de marché négocié après appel d'offres infructueux: 5 Mepy Système",
            "Rang 1 Helmlinger. Rang 2 Jonnette. Rang 3 infructueux",
            "SPIE secteurs 5A à 5D/ infructueux 5E 5F/ Marc Elec 5G/ Chenelec 5H",
            "Secteur Allier: grange (0 %) et Fayolle (-4 %) Secteur Puy-de-Dôme: lot non attribué (infructueux)",
            "A) non attribué. B1) et B2) Climelec",
            "Agence d'Amboise : appel d'offres déclaré sans suite - Agence de Chinon : savelys GDF suez",
            "Mongin Jauffret : lots 1504 et 1505 Zi Delta industrie la Valentine lots infructueux : 1501, 1502 et 1503",
            "3a) SIAL 54520 3b) lot infructueux",
            "3a: SIAL 54520 - 3b: lot infructueux",
            // The fourth dry run (job 2173): Dutch forms and lots.
            "Humble Inpect B.V. (niet gegunde partij)",
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
        ] {
            assert_eq!(not_a_name(void), Some(NotAName::VoidLot), "{void}");
            assert!(super::mentions_void(void), "{void}");
        }
        assert_eq!(
            not_a_name("Sublote 9.1: Declarado desierto. Sublote 9.2: Declarado desierto."),
            Some(NotAName::AwardSummary),
            "a summary of void lots only is kept — the accepted cost (a junk name, never a lost award)"
        );
        assert!(super::mentions_void("Lot 1) Dupont. Lot 2) infructueux"), "the walk's net still catches a summary");
        assert!(!super::mentions_void("ACME SA"));
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
        let wholes: Vec<&str> =
            VOID_WHOLE.iter().chain(&PLACEHOLDER_WHOLE).chain(&BLANK_WHOLE).chain(&scoped).copied().collect();
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
