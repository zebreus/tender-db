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
//! **Two classes, decided separately.**
//! - [`NotAName::VoidLot`] — a statement that the lot was not awarded. The fold drops
//!   such a party in every era and role (no organization, no role, no result winner, no
//!   bid party), and a legacy result left with no real winner reads `clos-nw`.
//! - [`NotAName::Placeholder`] — a pointer, a withheld name, a summary of the award, or a
//!   void phrase inside a name that also names an award ([`names_an_award`]: the lot was
//!   then awarded to a named company, or the "name" is a per-lot summary listing the
//!   winners of the other lots beside the void one). The text parser refuses it; the fold
//!   leaves it for issue 511, because these sit mostly in review, mediation and buyer
//!   slots, and a winner WAS chosen. Dropping such a summary would turn its real awards
//!   into `clos-nw` — the first drain's dry run (2026-10-10, job 2164) found them among
//!   the matches: `Lot 1) Sarl Bremond. Lot 2) S.A. Les Rapides Varois. Lot 3) Déclaré
//!   Infructueux …`.
//!
//! **Matching.** On the role census's fold (case, Latin accents and every non-alphanumeric
//! run folded to one space; [`fold`]). A stem matches at a left word boundary and may end
//! mid-word (`infructu` catches `infructueux`, `infructueuse`, `infructuosité`); a whole
//! value matches only the entire folded name, after one trailing lot qualifier
//! (`(lote 3)`, `(lots 2 et 4)`) is stripped. Every entry is a 2026-10-10 prod
//! measurement (`.scratch/tender-db/510-void-names/unit1-decision.md` §2): each whole
//! value says the lot was not awarded, has 20 or more winner-role mentions, and no real
//! company with that exact fold. The real names beside the phrases stay names —
//! `Desierto OÜ`, `DESERTOT`, `VÁRIOS MUNDOS`, `NIL d.o.o.`, `N/A s.r.o.`,
//! `Void arhitektura`, `Gestiver Información SL` — pinned by the tests below.

/// What a non-name is ([`not_a_name`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotAName {
    /// The lot was not awarded.
    VoidLot,
    /// A pointer, a withheld name or a summary; a winner may well exist.
    Placeholder,
}

/// Stems that say the lot was not awarded, matched at a left word boundary on the
/// fold. The 508 text list, plus the `infructeux` misspelling and the feminine
/// `declarada desierta`. Each carries its leading space, the boundary, so the match is
/// a `contains` on the space-padded fold with nothing built per stem (the projection's
/// hot loop: the issue-483 `PADDED_PATTERNS` lesson).
const VOID_STEMS: [&str; 8] = [
    " infructu",
    " infructeu",
    " sans suite",
    " non attribu",
    " declarado desiert",
    " declarada desiert",
    " queda desiert",
    " nessuna aggiudicazione",
];

/// Whole values that say the lot was not awarded, compared entire after the lot strip.
const VOID_WHOLE: [&str; 30] = [
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
    "aucune offre",
    "aucune offre recue",
    "pas d attributaire",
    "pas d offre",
    "sans offre",
    "abandon",
    "aufgehoben",
    "nicht vergeben",
];

/// Pointer and withheld-value stems (the rest of the 508 text list), padded like
/// [`VOID_STEMS`].
const PLACEHOLDER_STEMS: [&str; 8] = [
    " would prejudice",
    " not applicable",
    " see section",
    " perfil del contratante",
    " perfil de contratante",
    " voir autres informations",
    " voir renseignements",
    " ver informaci",
];

/// Placeholder whole values.
const PLACEHOLDER_WHOLE: [&str; 1] = ["various"];

/// The words that open a trailing lot qualifier ([`strip_lot_qualifier`]).
const LOT_WORDS: [&str; 6] = ["lot", "lots", "lote", "lotes", "lotto", "lotti"];

/// Lot designators on the fold, for [`several_lots`]: [`LOT_WORDS`], the Spanish sub-lot
/// (`sous-lot` folds to `sous lot`, so `lot` covers it), and the other words a per-lot
/// summary numbers its parts with (`Tranche 1 : X. Tranche 2 : infructueux`, `Partie`,
/// `Partida`). Not `marché`: it is numbered by its contract reference and its duration
/// (`marché 2012 02 1 0014`, `durée du marché: 5 ans`) inside one lot's void sentence.
const LOT_DESIGNATORS: [&str; 11] =
    ["lot", "lots", "lote", "lotes", "lotto", "lotti", "sublote", "sublotes", "tranche", "partie", "partida"];

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
const NEGATIONS: [&str; 18] = [
    "non", "pas", "sans", "aucun", "aucune", "no", "not", "niet", "sin", "ningun", "ninguno", "ninguna", "nessun",
    "nessuno", "nessuna", "alcun", "faute", "absence",
];

/// Words skipped between an award word and its negation (`pas d'attributaire`, `no hay
/// adjudicatario`, `non è stato individuato alcun aggiudicatario` reads its `alcun`).
const NEGATION_FILLERS: [&str; 15] =
    ["d", "de", "du", "des", "l", "la", "le", "un", "une", "el", "del", "di", "hay", "ha", "sido"];

/// Company forms as published, compared on the raw token with its surrounding punctuation
/// trimmed and its dots removed ([`company_token`]): a void phrase beside one names the
/// company another lot went to. Case-sensitive, so the French possessive `sa` (`lors de sa
/// séance`, `et Sa périphérie`) is no `SA`; [`COMPANY_FORMS_DOTTED`] takes any case when
/// the token was written with dots (`s.r.l.`, `S.A.R.L`). Not the census's folded
/// `COMMERCIAL_FORMS`, which fold the possessive onto `SA`.
const COMPANY_TOKENS: [&str; 33] = [
    "SA", "SAS", "Sas", "sas", "SASU", "SAU", "SARL", "Sarl", "sarl", "EURL", "Eurl", "eurl", "SNC", "Sté", "STÉ",
    "Ets", "ETS", "GmbH", "GMBH", "SL", "SLU", "SpA", "SPA", "Srl", "SRL", "srl", "Ltd", "LTD", "Limited",
    "LIMITED", "Lda", "LDA", "SPRL",
];

/// Company forms written with dots, any case once the dots are gone (`s.r.l.` → `srl`). Not
/// `Ste`/`STE` in either list: that is Sainte in a place name (`Ste Marie de Figaniella`).
const COMPANY_FORMS_DOTTED: [&str; 11] = ["sa", "sas", "sasu", "sau", "sarl", "srl", "spa", "sl", "slu", "ltd", "ets"];

/// The folded words after `SA` that make it the possessive (`LORS DE SA SÉANCE`).
const POSSESSIVE_NEXT: [&str; 8] =
    ["seance", "peripherie", "forme", "duree", "reunion", "decision", "deliberation", "session"];

/// A name as the lists read it: the role census's fold — [`crate::project::match_norm`]
/// (lowercase, every non-alphanumeric run one space), then Latin diacritics folded.
pub fn fold(name: &str) -> String {
    store::buyer_name_fold(&crate::project::match_norm(name))
}

/// What `name` is when it is not a name; `None` for a name.
pub fn not_a_name(name: &str) -> Option<NotAName> {
    let full = fold(name);
    if full.is_empty() {
        return None;
    }
    let padded = format!(" {full}");
    if VOID_STEMS.iter().any(|stem| padded.contains(stem)) {
        // "Lot déclaré infructueux … puis attribué à la Société X", "Lot 1) Sarl Bremond.
        // Lot 2) Infructueux": an award IS named beside the void lot. Reading it as no
        // award would contradict the publisher, so it is a placeholder (the fold leaves it).
        return Some(if names_an_award(name, &full) { NotAName::Placeholder } else { NotAName::VoidLot });
    }
    if PLACEHOLDER_STEMS.iter().any(|stem| padded.contains(stem)) {
        return Some(NotAName::Placeholder);
    }
    // Folded again only when a lot qualifier came off: the fold already drops leading and
    // trailing punctuation and space, so an unstripped name folds to `full`.
    let stripped = strip_lot_qualifier(name).map(fold);
    let whole = stripped.as_deref().unwrap_or(&full);
    if VOID_WHOLE.contains(&whole) {
        return Some(NotAName::VoidLot);
    }
    if PLACEHOLDER_WHOLE.contains(&whole) {
        return Some(NotAName::Placeholder);
    }
    None
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
    award_clause(folded) || awardee_noun(&words) || several_lots(&words) || mixed_segments(raw) || company_token(raw)
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
    let mut numbers: Vec<String> = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if LOT_DESIGNATORS.contains(w) {
            let mut j = i + 1;
            while j < words.len() && j <= i + 2 && NUMBER_MARKS.contains(&words[j]) {
                j += 1;
            }
            if let Some(n) = words.get(j).filter(|n| number(n)) {
                let mut whole = (*n).to_owned();
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
            numbers.push(rest.to_owned());
        }
    }
    numbers.sort_unstable();
    numbers.dedup();
    numbers.len() >= 2
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
    let trimmed = name.trim().trim_end_matches(|c: char| c.is_whitespace() || ".,;:".contains(c));
    let inner = trimmed.strip_suffix(')')?;
    let open = inner.rfind('(')?;
    let group = &inner[open + 1..];
    if group.contains(['(', ')']) {
        return None;
    }
    let first = group.trim_start().split(|c: char| !c.is_alphanumeric()).next().unwrap_or("");
    LOT_WORDS.iter().any(|w| first.eq_ignore_ascii_case(w)).then(|| inner[..open].trim_end())
}

#[cfg(test)]
mod tests {
    use super::{NotAName, PLACEHOLDER_STEMS, PLACEHOLDER_WHOLE, VOID_STEMS, VOID_WHOLE, fold, not_a_name};

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

    /// A void phrase that goes on to name the company the lot was then awarded to.
    #[test]
    fn a_void_phrase_that_goes_on_to_name_an_award_is_a_placeholder() {
        for awarded in [
            "Lot déclaré infructueux lors de l'appel d'offre puis attribué à la Société Sandoz",
            "Infructueux, marché attribué à: Berthelet",
            "lot infructueux, relancé et attribuée à LD Bio Diagnostics",
            "Sans suite, marché passé avec la société Dupont",
        ] {
            assert_eq!(not_a_name(awarded), Some(NotAName::Placeholder), "{awarded}");
        }
        assert_eq!(not_a_name("Non attribué à ce jour"), Some(NotAName::VoidLot), "a negated award is no award");
    }

    /// The first drain's dry run (job 2164): a "name" that summarises several lots, or
    /// names a company beside the void phrase, is not a void lot — dropping it would turn
    /// the real awards it lists into `clos-nw`. Each is a prod specimen. The pure void
    /// sentences beside them (French possessive `sa` included) stay void.
    #[test]
    fn a_summary_that_names_an_award_beside_a_void_lot_is_a_placeholder() {
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
        ] {
            assert_eq!(not_a_name(summary), Some(NotAName::Placeholder), "{summary}");
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
            "Lot 3) Ce lot 3 est déclaré infructueux",
            "Le lot 10: lot n° 10: CQP Vienne a été déclaré infructueux",
            "Le lot 48: Transport scolaire circuit: Ste Marie de Figaniella / Propriano a été déclaré Infructueux",
            "Le lot 3: assurance des véhicules. Durée du marché: 5 ans à compter du 1.1.2011 a été déclaré infructueux",
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
            Some(NotAName::Placeholder),
            "a summary of void lots only is kept — the accepted cost (a junk name, never a lost award)"
        );
        assert!(super::mentions_void("Lot 1) Dupont. Lot 2) infructueux"), "the walk's net still catches a summary");
        assert!(!super::mentions_void("ACME SA"));
    }

    #[test]
    fn placeholders_are_their_own_class() {
        for placeholder in [
            "See section VI.3 for the list of awarded suppliers",
            "Véase perfil del contratante",
            "Ver información adicional",
            "Voir autres informations",
            "Not applicable",
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
        ] {
            assert_eq!(not_a_name(name), None, "{name}");
        }
    }

    #[test]
    fn the_lot_qualifier_is_stripped_only_for_the_whole_compare() {
        assert_eq!(not_a_name("desierto (lote 3)"), Some(NotAName::VoidLot));
        assert_eq!(not_a_name("Desierto (lot 9/10)."), Some(NotAName::VoidLot));
        assert_eq!(not_a_name("ACME (lot 3)"), None);
        assert_eq!(not_a_name("Desierto (no ofertas)"), None, "not a lot qualifier: prose stays out of the whole compare");
        assert_eq!(not_a_name(""), None);
        assert_eq!(not_a_name("  ...  "), None);
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
        for stem in VOID_STEMS.iter().chain(&PLACEHOLDER_STEMS) {
            let bare = stem.strip_prefix(' ').unwrap_or_else(|| panic!("{stem:?} lacks its boundary space"));
            assert_eq!(fold(bare), bare, "{stem:?}");
        }
        for whole in VOID_WHOLE.iter().chain(&PLACEHOLDER_WHOLE) {
            assert_eq!(fold(whole), *whole, "{whole:?}");
        }
    }
}
