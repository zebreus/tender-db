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
const PLACEHOLDER_WHOLE: [&str; 70] = [
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
/// 3. a placeholder stem — an anchored pointer stem may follow one polite first word
///    ([`POINTER_LEAD_INS`]: `Please see section VI.4.2`): [`NotAName::AwardSummary`] when a
///    pointer is followed by a name ([`pointer_names`], a form section's own title not counted:
///    [`FORM_SECTION_TITLES`]; a perfil stem's genitive owner included: [`perfil_owner`]), a
///    boundary stem follows one ([`named_before_stem`]) or the name has a summary's shape
///    ([`summary_shape`]), else [`NotAName::Placeholder`];
/// 4. a blank stem: [`NotAName::AwardSummary`] for a summary's shape or a `but` clause that
///    names a body ([`but_names`]), else [`NotAName::Blank`];
/// 5. the whole values: 510's void ones first (with its reason and prefix rules), then
///    [`PLACEHOLDER_WHOLE`] (and a legal reference closing on `erfolgt hier keine Angabe`:
///    [`withheld_by_reference`]), [`BLANK_WHOLE`], and [`SCOPED_WHOLE`] in its languages and a
///    plain case — after a bare lot qualifier only; one that names a company or a list is
///    an [`NotAName::AwardSummary`] ([`bare_lot_qualifier`]).
pub fn not_a_name_in(name: &str, lang: Option<&str>) -> Option<NotAName> {
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
/// Kier Construction` and the residual's `Please see section VI.4.3 Translink` are
/// `Placeholder` (no separator, so the keep rule never reads the tail), but the tail names a
/// body, and its untranslated copy `Siehe Abschnitt VI.3 Kier Construction` keeps the party.
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
/// at ANY left word boundary, any whole value, scoped words included, and a translated
/// pointer's shape with a form section number (unit 4's echo, [`echo_shaped`]) — before the
/// anchor, the pointer keep rule, [`summary_shape`] and the language scope. The job
/// re-queues every hit [`not_a_name`] does not read as an [`NotAName::AwardSummary`], so
/// the fold decides the anchor, the scope and the identifier guard, and its dry plan lists
/// the hits the anchor refuses.
pub fn mentions_placeholder(name: &str) -> bool {
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
        // Issue 511 unit 4 (the review of item 1): every name the fold's translation echo can
        // sweep. A party's head name is its first-seen copy, the ORIGINAL language's (`r209::parse`
        // walks the primary original first), so a FR-original pointer winner heads as `Voir point
        // VI.2) Autres informations.` while the English placeholder that drops it is a later name
        // no walk reads; the echo is the first drop rule whose party drops with its head name
        // unclassified.
        || echo_shaped(name)
}

/// Issue 511 unit 4: whether `raw` is what the fold's translation echo reads as a translated
/// pointer when its party's placeholder carries the same number — a form section number
/// ([`form_section_tokens`]) in a translated pointer's shape ([`translation_shaped`]). The walk's
/// net arm for the echo ([`mentions_placeholder`]): it costs a refold for a short name that cites
/// a section (`Referat II.3`), and the fold decides.
fn echo_shaped(raw: &str) -> bool {
    !form_section_tokens(raw).is_empty() && translation_shaped(raw)
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
pub fn translation_shaped(raw: &str) -> bool {
    raw.split_whitespace().count() <= TRANSLATION_MAX_WORDS && !names_a_legal_form(raw)
}

/// Whether `raw` carries a company legal form by any of the crate's readings: [`company_token`]
/// (`SA`, `GmbH`, `Ltd`, `s.r.l.`), [`polish_company_form`], the crosswalk's family tokens
/// ([`crate::crosswalk::legal_form_family`]: `AG`, `Oy`, `s.r.o.`, `Kft`) and
/// [`LEGAL_FORM_TOKENS`] (`mbH`, `plc`, `A/S`, `d.o.o.`). Errs toward a form: a hit keeps a party.
fn names_a_legal_form(raw: &str) -> bool {
    company_token(raw)
        || polish_company_form(&format!(" {}", fold(raw)))
        || crate::crosswalk::legal_form_family(raw).is_some()
        || raw
            .split_whitespace()
            .any(|t| LEGAL_FORM_TOKENS.contains(&t.trim_matches(|c: char| !c.is_alphanumeric() && c != '.')))
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
/// Royal Courts of Justice`), and a perfil stem that opens the name, whose genitive names
/// the profile's owner ([`perfil_owner`]: `Perfil de contratante del Gobierno de Aragón`).
fn pointer_names(raw: &str, padded: &str, stem: &str, pos: usize) -> bool {
    let Some(after) = after_stem(raw, padded, stem, pos) else {
        return false;
    };
    let (rest, referenced) = strip_references(after);
    let rest = match open_separator(rest) {
        Some(after) => strip_references(after).0,
        None if stem == " see section" && !referenced => rest,
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

/// Issue 511 unit 4 item 2 (residual dry 2273): the titles TED's standard forms give their
/// sections, on the fold — a closed list, not a rule about capitals. After a pointer one names
/// the section pointed at, not a body, so [`pointer_names`] reads past it: `See section VI.3
/// (Additional Information)` (144 winner mentions read as summaries with its spellings), `See
/// Section VI: Complementary Information`, `See Section II.2.3) Place of performance`, `Please see
/// section II.1.5 Short Description`. Taken from the plan's `non_exact` list and the 2008 and
/// 2014 form labels.
const FORM_SECTION_TITLES: [&str; 28] = [
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
];

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
        PLACEHOLDER_WHOLE, POINTER_LEAD_INS, POINTER_STEMS, REFERENCE_HEAD_WORDS, SCOPED_WHOLE, VOID_STEMS, VOID_WHOLE, fold,
        is_exact_entry, mentions_placeholder, not_a_name, not_a_name_in,
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
        }
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
        // A body after the number with no separator: `Placeholder` (the keep rule wants one), but
        // not bare, so it seeds nothing — the residual's `Translink` (1 mention, review body).
        for named in [
            "See Section VI.3 Kier Construction",
            "Please see section VI.4.3 Translink",
            "See section VI.4.3 Translink",
            "See Section VI.3 Acme plc",
            // A capitalised noun withholds the seed too, erring toward a name.
            "Please see section VI.3 for list of awarded Suppliers",
        ] {
            assert_eq!(not_a_name(named), Some(NotAName::Placeholder), "premise: {named}");
            assert!(names_after_stem(named), "{named} names something after its stem");
        }
        assert!(names_after_stem("See section VI.4.1: High Court of Justice"), "the keep rule's own case");
        // The net: a FR- or DE-original award heads its pointer party with the translation.
        for copy in [
            "Voir point VI.2) Autres informations.",
            "siehe Abschnitt VI.2 „Sonstige Informationen“.",
            "Βλέπε Τμήμα VI.2 «Πρόσθετες πληροφορίες».",
            "Lásd a VI.2., További információk című pontot.",
            "Siehe Abschnitt VI.4.3 Translink",
        ] {
            assert_eq!(not_a_name(copy), None, "premise: {copy} reads as a name alone");
            assert!(mentions_placeholder(copy), "the walk's net catches {copy}");
            assert!(!is_exact_entry(copy), "and the dry plan lists {copy} for reading");
        }
        for name in [
            "Müller Bau GmbH (siehe Abschnitt VI.3)",
            "Consortium of the participating institutions of the Netherlands Aerospace Laboratory listed in VI.3 below",
            "Se punkt VI Yderligere oplysninger",
            "Region Hovedstaden",
            "Acme Ltd",
        ] {
            assert!(!mentions_placeholder(name), "{name}");
        }
    }
}
