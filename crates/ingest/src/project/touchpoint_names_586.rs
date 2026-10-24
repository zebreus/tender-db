//! Issue 586 unit 2: the touchpoint alias fold (D586-1) and the touchpoint gate (D586-2), as pure functions of the
//! names, ported from the decision's oracle (`.scratch/tender-db/586-u1/final.py`, which layers decision.md's
//! amendments F1-F4, C4, G1, G1b, G2 and G3 over `fold.py` and `gate.py`; their lists are the source of truth).
//!
//! - [`alias_tier`]: whether a touchpoint name names its own parent Organization (D576-1's `parent`, the same
//!   notice) under another spelling — tier A, the two names' fold keys are equal — or as that Organization's body
//!   under its own governing organ — tier C (C0-C4). `touchpoint_decision` reads it after `store::org_name_norm`'s
//!   equality (which stays at spelling: the cross-notice reuse arm has no wall) and before the gate.
//! - [`gated`]: D586-2's six touchpoint-only tests (a whole contact label, a person addressed by title, filing
//!   prose, a legal reference, a platform's name, a bare organ, town-hall or registry word), beside
//!   `touchpoint_gate`'s D576-1 (iv) tests. Nothing else reads them: they are not `role_census::NAME_PATTERNS`,
//!   whose kinds every buyer census reads.
//!
//! No new dependency (ingest has no `regex` or `unicode-normalization`): the characters fold through
//! [`super::title_key_nfkd::NFKD_STRIPPED`] (generated from the reference's own `unicodedata`), and the reference's
//! regular expressions are written out as scans. The port check (`the_port_matches_the_reference_on_every_saved_pair`)
//! holds both functions to the oracle's answers on the saved pairs.
//!
//! Where the port departs from `final.py` on purpose (each moves none of the 3,751 saved pairs; each mints where the
//! oracle aliased or dropped another body):
//! 1. C4 needs a touchpoint that is the bare word as printed: the town key ([`Steps::Town`]) must equal the key
//!    (`Gemeente Epe`, EPE read as a legal form, is another municipality, not a bare `gemeente`).
//! 2. The town key, G2's (BARE) and C4's guard, keeps a last token that is an article (`Gemeinde Au`, `Commune de
//!    Y`, `Město Aš`, `Ajuntament de Les`: towns named like an article are no bare municipal word either).
//! 3. A touchpoint bracket naming a Spanish province is cut only when the parent names that place or none of the
//!    list ([`ParentMarks::cuts_place`]): `Tribunal Superior de Justicia (Madrid)` is not the Andalusian one.
//! 4. HONORIFIC: `M.` before a Spanish body's style (`M. Ilustre Colegio de Abogados`) is that style's abbreviation,
//!    as `M. I.` is, not a title.
//!
//! Accepted differences of the characters: the case-insensitive scans read `İ` and `ı` as `i` and `ſ` as `s`, as
//! the reference's `re` does ([`re_lower`]); [`crate::partyname::fold`] (the gate's fold) keeps an Other_Alphabetic
//! symbol such as `Ⓐ` where the reference's `pop.fold` drops it (`ⒶVer apartado …` mints here); and letters assigned
//! after Unicode 14 (U+A7CB-U+A7DC) fold by Rust's newer tables.

use std::cmp::Reverse;
use std::sync::LazyLock;

use super::role_census;
use super::title_key_nfkd::NFKD_STRIPPED;

/// The alias tier of a touchpoint name under its parent Organization's name ([`alias_tier`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tier {
    /// The fold keys are equal: spelling, accents, punctuation, the allowed brackets, legal forms, articles.
    A,
    /// One organ under two spellings (`junta gobierno local` / `junta gobierno`).
    C0,
    /// The entity under its organ: the parent minus one leading organ phrase.
    C1,
    /// The town hall under another municipal word (`Ajuntament de Biar` under `… Ayuntamiento de Biar`).
    C2,
    /// The bare town under its town hall (`Marugán` under `Pleno del Ayuntamiento de Marugán`).
    C3,
    /// The town hall of a parent printed as a bare place (`COMUNE DI LAURINO` under `LAURINO`).
    C3r,
    /// The parent's own leading organ (`Pleno` under `Pleno de la Diputación Provincial de Huelva`), or the bare
    /// town-hall word of its entity (`Ayuntamiento` under `Alcaldía del Ayuntamiento de …`).
    C4,
}

impl Tier {
    /// How `TouchpointRoles` counts it: `aliased_a` or `aliased_c`.
    pub(super) fn counted(self) -> super::AliasTier {
        match self {
            Tier::A => super::AliasTier::A,
            _ => super::AliasTier::C,
        }
    }

    /// The reference's name for it (`final.tier`'s answer).
    #[cfg(test)]
    fn label(self) -> &'static str {
        match self {
            Tier::A => "A",
            Tier::C0 => "C0",
            Tier::C1 => "C1",
            Tier::C2 => "C2",
            Tier::C3 => "C3",
            Tier::C3r => "C3r",
            Tier::C4 => "C4",
        }
    }
}

// ------------------------------------------------------------------------------------------------ the lists
//
// Every entry is written as folded tokens, one space between them (`every_list_entry_is_its_own_fold`). Phrases
// match token by token, the longest first.

/// Spelled-out legal forms, removed anywhere in a name (multi-word and unambiguous).
const LEGAL_ANYWHERE: &[&str] = &[
    "sociedad anonima unipersonal", "sociedad limitada unipersonal", "sociedad anonima municipal",
    "sociedad anonima", "sociedad limitada", "sociedad de responsabilidad limitada", "sociedad cooperativa",
    "societat anonima", "societat limitada", "sociedade anonima", "sociedade unipessoal",
    "spolka z ograniczona odpowiedzialnoscia", "spolka akcyjna", "spolka komandytowa", "spolka jawna",
    "akciova spolecnost", "spolecnost s rucenim omezenym", "spolocnost s rucenim obmedzenym", "societa per azioni",
    "societa a responsabilita limitata", "societa cooperativa", "societa consortile", "aktiengesellschaft",
    "gesellschaft mit beschrankter haftung", "anstalt des offentlichen rechts", "anstalt offentlichen rechts",
    "korperschaft des offentlichen rechts", "naamloze vennootschap", "besloten vennootschap", "societe anonyme",
    "societe par actions simplifiee", "societe a responsabilite limitee", "public limited company",
    "medio propio personificado", "medio propio", "mitja propi",
];

/// Abbreviated legal forms, removed only at the end of a name, repeatedly, never down to nothing. Left out on
/// purpose: two-letter tails that also end a Spanish department or region (`sc`, `uc`, `se`, `cv`, …).
const LEGAL_TRAILING: &[&str] = &[
    "s a u", "sau", "s l u", "slu", "s a", "sa", "s l", "sl", "s l l", "sll", "s l p", "slp", "s coop", "soc coop",
    "s a l", "sal", "s a m", "sam", "s m e", "sme", "m p", "mp", "e p e", "epe", "e p e l", "epel", "o a", "oa",
    "o a l", "oal", "e m", "em", "a i e", "aie", "e i r l", "eirl", "lda", "unipessoal lda", "f s p", "fsp", "i p",
    "ip", "s p a", "spa", "s r l", "srl", "s c a r l", "scarl", "s c r l", "scrl", "onlus", "s n c", "snc", "s a s",
    "sas", "s a r l", "sarl", "eurl", "sasu", "spl", "sem", "scop", "gie", "gmbh", "ggmbh", "mbh", "gmbh co kg",
    "co kg", "kg", "ag", "e v", "ev", "a o r", "aor", "k d o r", "kdor", "b v", "bv", "n v", "nv", "u a", "ua",
    "v o f", "vof", "bvba", "sprl", "cvba", "vzw", "asbl", "ltd", "limited", "plc", "llp", "llc", "inc", "cic",
    "dac", "clg", "sp z o o", "sp z oo", "spzoo", "sp k", "spk", "sp j", "s k a", "s r o", "sro", "spol s r o",
    "a s", "as", "p o", "o p s", "kft", "zrt", "nyrt", "d o o", "doo", "dooel", "eood", "ood", "ead", "еоод", "оод",
    "еад", "asa", "ab", "publ", "oy", "oyj", "aps", "uab", "sia", "α ε", "αε", "ε π ε", "επε", "ι κ ε", "ικε",
    "ο ε", "οε",
];

/// Articles, prepositions and conjunctions (es ca gl pt fr it de nl en pl), removed anywhere, after the legal
/// forms (`sociedad de responsabilidad limitada` holds `de`). None tells two bodies of one notice apart.
const ARTICLES: &[&str] = &[
    "el", "la", "los", "las", "lo", "de", "del", "al", "y", "e", "els", "les", "l", "d", "dels", "i", "o", "a",
    "os", "as", "do", "da", "dos", "das", "le", "du", "des", "et", "au", "aux", "il", "gli", "di", "dello", "della",
    "dei", "degli", "delle", "dell", "ed", "der", "die", "dem", "den", "und", "het", "van", "the", "of", "and", "w",
    "we", "en", "con",
];

/// The honorific styles of Spanish public bodies (`Excmo. Ayuntamiento de …`), removed anywhere.
const HONORIFICS: &[&str] = &[
    "excmo", "excma", "exmo", "exma", "excelentisimo", "excelentisima", "ilmo", "ilma", "iltmo", "iltma", "iltre",
    "ilustrisimo", "ilustrisima", "ilustre",
];

/// Touchpoint-side bracket contents that are cut (folded tokens, articles kept): contact and registry labels.
const TP_BRACKET_LABELS: &[&str] = &[
    "punto de contacto", "otro punto de contacto", "punto contacto", "pc", "tpo", "contact point",
    "point de contact", "punt de contacte", "sede electronica", "registro", "registro electronico",
    "registro general", "preguntas y respuestas", "a traves de placsp", "na", "n a",
];

/// … and a touchpoint bracket naming exactly a Spanish province or autonomous community (`Ayuntamiento de Lucena
/// (Córdoba)`).
const TP_BRACKET_PLACES: &[&str] = &[
    "alava", "araba", "albacete", "alicante", "alacant", "almeria", "asturias", "avila", "badajoz", "barcelona",
    "burgos", "caceres", "cadiz", "cantabria", "castellon", "castello", "ciudad real", "cordoba", "a coruna",
    "la coruna", "coruna", "cuenca", "girona", "gerona", "granada", "guadalajara", "gipuzkoa", "guipuzcoa",
    "huelva", "huesca", "illes balears", "islas baleares", "baleares", "jaen", "leon", "lleida", "lerida",
    "la rioja", "rioja", "lugo", "madrid", "malaga", "murcia", "navarra", "ourense", "orense", "palencia",
    "las palmas", "pontevedra", "salamanca", "santa cruz de tenerife", "tenerife", "segovia", "sevilla", "soria",
    "tarragona", "teruel", "toledo", "valencia", "valladolid", "bizkaia", "vizcaya", "zamora", "zaragoza", "ceuta",
    "melilla", "andalucia", "aragon", "canarias", "castilla la mancha", "castilla y leon", "cataluna", "catalunya",
    "comunidad de madrid", "comunitat valenciana", "extremadura", "galicia", "pais vasco", "euskadi",
    "region de murcia",
];

/// The [`TP_BRACKET_PLACES`] entries that name one place in two languages or spellings
/// ([`ParentMarks::cuts_place`]); an entry inside another (`murcia` in `region de murcia`) needs no row.
const PLACE_SPELLINGS: &[&[&str]] = &[
    &["alava", "araba"],
    &["alicante", "alacant"],
    &["castellon", "castello"],
    &["a coruna", "la coruna", "coruna"],
    &["girona", "gerona"],
    &["gipuzkoa", "guipuzcoa"],
    &["illes balears", "islas baleares", "baleares"],
    &["lleida", "lerida"],
    &["ourense", "orense"],
    &["bizkaia", "vizcaya"],
    &["cataluna", "catalunya"],
    &["pais vasco", "euskadi"],
];

/// A contact label leading the touchpoint's key (`Punto de contacto - Tribunal …`), cut from its head, never to
/// nothing.
const TP_HEAD_LABELS: &[&str] = &["otro punto contacto", "punto contacto"];

/// F3: a parent bracket holding one of these words (whole tokens; `departament` and `recurso` are prefixes) stays
/// in the parent key — it names a department, a registry or a review organ (`Ayuntamiento de Sevilla (TARCAS)`), so
/// the bare entity printed by the touchpoint is not its parent. Every other parent bracket (an acronym, a place, a
/// descriptor: `(SARGA)`) is cut.
const DEPT_OR_REVIEW: &[&str] = &[
    "contratacion", "compras", "servicio", "servei", "servizio", "area", "concejalia", "regidoria", "seccion",
    "unidad", "unitat", "negociado", "oficina", "distrito", "secretaria", "delegacion", "direccion", "gerencia",
    "tribunal", "tarcas", "tacrc", "tarc", "registro", "juzgado", "mesa", "abteilung", "amt", "afdeling", "dienst",
    "wydzial", "biuro", "ufficio", "settore",
];
/// F3's prefix entries (`departament\w*`, `recurso\w*`).
const DEPT_OR_REVIEW_PREFIXES: &[&str] = &["departament", "recurso"];
/// F3's one phrase.
const DEPT_OR_REVIEW_PHRASE: &[&str] = &["junta", "de", "contratacion"];

/// Tier C: the governing and collegiate organs and their officers, cut from the head of the PARENT key (one, the
/// longest that leaves something). `direccion` is one, so `Dirección Provincial de AVRA en Sevilla` loses it
/// (harmless: no touchpoint prints the remainder). Departments are not organs (Secretaría General, Concejalía,
/// Área, Servicio, Distrito, Delegación, Mesa / Junta de Contratación): a parent printed as a department keeps its
/// department words.
const ORGAN_PHRASES: &[&str] = &[
    "presidencia consejo administracion", "gerencia consejo administracion", "alcaldia presidencia",
    "junta gobierno local", "junta govern local", "xunta goberno local", "junta gobierno", "junta govern",
    "xunta goberno", "pleno", "ple", "pleno municipal", "alcaldia", "presidencia", "vicepresidencia", "gerencia",
    "direccion gerencia", "direccion gerente", "consejo administracion", "consell administracio",
    "consello administracion", "consejo rector", "consell rector", "consello reitor", "rectorado",
    "consejo gobierno", "consell govern", "consello goberno", "consejo gobierno insular", "comision ejecutiva",
    "comision gestora", "consejero delegado", "consejera delegada", "consejeria delegada", "direccion general",
    "direccion ejecutiva", "direccion", "asamblea", "asamblea concejales", "asamblea representantes",
    "junta general", "junta vecinal", "consejo general", "consejo comarcal", "junta directiva", "consejo directivo",
    "junta plenaria", "junta rectora", "comision permanente", "asamblea vecinal", "asamblea general",
    "comite directivo", "administrador unico", "organo administracion", "rector", "rectora", "presidente",
    "presidenta", "vicepresidente", "vicepresidenta", "alcalde", "alcaldesa", "alcalde presidente", "gerente",
    "director", "directora", "director gerente", "directora gerente", "director general", "directora general",
];

/// C0 and C4: one organ under two spellings, canonicalised at the head of both keys.
const ORGAN_SYNONYMS: &[(&str, &str)] = &[
    ("junta gobierno local", "junta gobierno"),
    ("junta govern local", "junta govern"),
    ("xunta goberno local", "xunta goberno"),
    ("pleno municipal", "pleno"),
    ("alcaldia presidencia", "alcaldia"),
];

/// F4: organs that over a whole government are a department or another institution, not its organ …
const UMBRELLA_ORGANS: &[&str] = &[
    "presidencia", "vicepresidencia", "direccion", "direccion general", "direccion ejecutiva", "junta general",
    "asamblea", "asamblea general", "asamblea representantes", "consejo general", "presidente", "presidenta",
    "vicepresidente", "vicepresidenta", "director", "directora", "director general", "directora general",
];
/// … when the entity left starts with one of these (`Gobierno de Canarias` under `Vicepresidencia del Gobierno de
/// Canarias` mints).
const UMBRELLA_HEADS: &[&str] = &[
    "gobierno", "govern", "generalitat", "xunta", "principado", "comunidad", "comunitat", "region", "estado",
    "administracion", "junta",
];

/// Municipal words, read as `ayuntamiento` at the head of a tier-C key. F1 leaves out [`MUNI_POLISH`]: in Poland a
/// rural gmina and the town of the same name are two units, each with its own REGON and NIP.
const MUNI: &[&str] = &[
    "ayuntamiento", "ajuntament", "concello", "ayto", "aj", "comune", "commune", "mairie", "ville", "gemeente", "stadt",
    "gemeinde", "municipio", "concelho", "obec", "mesto", "kommune",
];
/// The municipal words F1 keeps out of the fold; a bare one is still a label ([`gated`]'s BARE test).
const MUNI_POLISH: &[&str] = &["gmina", "miasto"];

/// G2 (BARE): registry and department labels, beside every organ phrase, organ synonym and municipal word.
const BARE_LABELS: &[&str] =
    &["registro", "registro general", "registro electronico", "sede electronica", "secretaria general"];

/// WHOLE: the folded name IS one of these contact or role labels (never a phrase inside a body's name).
const WHOLE: &[&str] = &[
    "principal", "sede tibidabo", "direccion principal", "info adicional", "recursos", "direccion postal",
    "direccion recepcion de recursos", "palacio provincial", "direccion recurso", "ayuntamiento",
    "procurement department", "direcci n principal", "direccion de recursos", "presentacion de recursos",
    "direccion informacion adicional", "informacion adicional", "contact", "beschaffende stelle",
    "body responsible for mediation procedures",
    "service from which information about the review procedure may be obtained",
    "organizacion que proporciona informacion adicional sobre el procedimiento de contratacion",
    "direccion para presentacion de recursos",
];

/// HONORIFIC: a leading title, matched on the raw name's first word, case-insensitively. The dot carries the
/// meaning: folded `m`, `ing`, `sig` and `ms` lead real names (`M-30`, `ING Bank`, `SIG Combibloc`). Not listed:
/// `Dr.`, `D.`/`Don`/`Doña`, `Excmo.`/`Ilmo.` (they lead bodies' names).
const HONORIFIC_TITLES: &[&str] = &[
    "monsieur", "madame", "mademoiselle", "mme", "mme.", "mlle", "mlle.", "herr", "frau", "dott", "dott.", "dott.ssa",
    "dottssa", "sig.", "sig.ra", "ing.", "m.", "mr.", "mrs.", "ms.",
];
/// … and these two only as written (`MRS Logística` is a company).
const HONORIFIC_TITLES_EXACT: &[&str] = &["Mr", "Mrs"];
/// G3: the word after the title that makes it an office's name (`Ing. Büro Schneider`), not a person's.
const OFFICE_NEXT: &[&str] = &["buro", "buero", "bureau", "studio"];

/// PROSE: filing or pointer sentences, whole words anywhere in the folded name.
const PROSE: &[&str] = &[
    "procede recurso", "ver apartado", "en los lugares", "lugares establecidos", "lugares indicados",
    "lugares previstos", "lugares habilitados",
];

/// PLATFORM: an e-procurement platform's name, whole words anywhere in the folded name.
const PLATFORM: &[&str] = &[
    "tuttogare", "plataforma electronica de contratacion", "platforma zakupowa", "platforma przetargowa",
    "e zamowienia", "ezamowienia", "logintrade", "achizitii md",
];

/// LEGALREF: the folded words that lead `art <number>` (`el art 44`, `y art 16`).
const LEGALREF_LEADS: &[&str] = &["el", "del", "y", "en", "segun", "al"];
/// G1: a name citing an EU act is no reference sentence (the Stadt Dortmund mandate name).
const EU_ACT: &[&str] = &[
    "verordnung", "richtlinie", "regulation", "directive", "reglamento", "regolamento", "directiva", "direttiva", "eg",
    "ewg", "eu", "ce", "ue", "cee",
];
/// G1b: a reference needs a statute cue — a law word or a review-body word (or a numbered act, [`numbered`]).
const LAW_CUE: &[&str] = &[
    "ley", "lcsp", "trlcsp", "lrjsp", "lpacap", "decreto", "rd", "lgs", "gesetz", "gwb", "vgv", "ustawa", "pzp", "loi",
    "codice", "tribunal", "tar", "juzgado", "vergabekammer",
];

/// Phrases as token lists, the longest (by tokens) first.
fn phrases(list: &'static [&'static str]) -> Vec<Vec<&'static str>> {
    let mut out: Vec<Vec<&str>> = list.iter().map(|p| p.split(' ').collect()).collect();
    out.sort_by_key(|p| Reverse(p.len()));
    out
}

static LEGAL_ANY: LazyLock<Vec<Vec<&'static str>>> = LazyLock::new(|| phrases(LEGAL_ANYWHERE));
static LEGAL_END: LazyLock<Vec<Vec<&'static str>>> = LazyLock::new(|| phrases(LEGAL_TRAILING));
static ORGANS: LazyLock<Vec<Vec<&'static str>>> = LazyLock::new(|| phrases(ORGAN_PHRASES));
static TP_HEAD: LazyLock<Vec<Vec<&'static str>>> = LazyLock::new(|| phrases(TP_HEAD_LABELS));
static PLACES: LazyLock<Vec<(&'static str, Vec<&'static str>)>> =
    LazyLock::new(|| TP_BRACKET_PLACES.iter().map(|p| (*p, p.split(' ').collect())).collect());
static SYNONYMS: LazyLock<Vec<(Vec<&'static str>, Vec<&'static str>)>> = LazyLock::new(|| {
    let mut out: Vec<(Vec<&str>, Vec<&str>)> =
        ORGAN_SYNONYMS.iter().map(|(k, v)| (k.split(' ').collect(), v.split(' ').collect())).collect();
    out.sort_by_key(|(k, _)| Reverse(k.len()));
    out
});

// ------------------------------------------------------------------------------------------------ characters

/// Python's `str.isspace` (the reference's `\s` and `str.split()`): Rust's White_Space plus U+001C..U+001F.
fn py_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// Python's `\w`: an alphanumeric character or `_`.
fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Python's `\d`: a decimal digit (general category Nd, Unicode 14.0, the reference's `unicodedata`).
fn decimal(c: char) -> bool {
    const ND: &[(u32, u32)] = &[
        (0x30, 0x39), (0x660, 0x669), (0x6f0, 0x6f9), (0x7c0, 0x7c9), (0x966, 0x96f), (0x9e6, 0x9ef), (0xa66, 0xa6f),
        (0xae6, 0xaef), (0xb66, 0xb6f), (0xbe6, 0xbef), (0xc66, 0xc6f), (0xce6, 0xcef), (0xd66, 0xd6f), (0xde6, 0xdef),
        (0xe50, 0xe59), (0xed0, 0xed9), (0xf20, 0xf29), (0x1040, 0x1049), (0x1090, 0x1099), (0x17e0, 0x17e9),
        (0x1810, 0x1819), (0x1946, 0x194f), (0x19d0, 0x19d9), (0x1a80, 0x1a89), (0x1a90, 0x1a99), (0x1b50, 0x1b59),
        (0x1bb0, 0x1bb9), (0x1c40, 0x1c49), (0x1c50, 0x1c59), (0xa620, 0xa629), (0xa8d0, 0xa8d9), (0xa900, 0xa909),
        (0xa9d0, 0xa9d9), (0xa9f0, 0xa9f9), (0xaa50, 0xaa59), (0xabf0, 0xabf9), (0xff10, 0xff19), (0x104a0, 0x104a9),
        (0x10d30, 0x10d39), (0x11066, 0x1106f), (0x110f0, 0x110f9), (0x11136, 0x1113f), (0x111d0, 0x111d9),
        (0x112f0, 0x112f9), (0x11450, 0x11459), (0x114d0, 0x114d9), (0x11650, 0x11659), (0x116c0, 0x116c9),
        (0x11730, 0x11739), (0x118e0, 0x118e9), (0x11950, 0x11959), (0x11c50, 0x11c59), (0x11d50, 0x11d59),
        (0x11da0, 0x11da9), (0x16a60, 0x16a69), (0x16ac0, 0x16ac9), (0x16b50, 0x16b59), (0x1d7ce, 0x1d7ff),
        (0x1e140, 0x1e149), (0x1e2f0, 0x1e2f9), (0x1e950, 0x1e959), (0x1fbf0, 0x1fbf9),
    ];
    let c = c as u32;
    ND.iter().any(|&(lo, hi)| (lo..=hi).contains(&c))
}

/// HTML entities decoded once, left to right (the reference's `decode_entities`): `&#NNN;` (1-7 digits) and
/// `&#xHHH;` (1-6 hex digits) to their character, `&amp; &quot; &apos; &lt; &gt;`, and `&nbsp;` to a space. An
/// invalid code point (0, a surrogate, past U+10FFFF) stays literal.
fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let tail = &rest[at + 1..];
        match entity(tail) {
            Some((c, len)) => {
                out.push(c);
                rest = &tail[len..];
            }
            None => {
                out.push('&');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The entity at the head of `tail` (the text after a `&`) and its length up to and including its `;`.
fn entity(tail: &str) -> Option<(char, usize)> {
    let semi = tail.bytes().take(9).position(|b| b == b';')?;
    let body = &tail[..semi];
    let c = match body {
        "amp" => '&',
        "quot" => '"',
        "apos" => '\'',
        "lt" => '<',
        "gt" => '>',
        "nbsp" => ' ',
        _ => {
            let number = body.strip_prefix('#')?;
            let n = match number.strip_prefix(['x', 'X']) {
                Some(hex) if (1..=6).contains(&hex.len()) && hex.bytes().all(|b| b.is_ascii_hexdigit()) => {
                    u32::from_str_radix(hex, 16).ok()?
                }
                Some(_) => return None,
                None if (1..=7).contains(&number.len()) && number.bytes().all(|b| b.is_ascii_digit()) => {
                    number.parse().ok()?
                }
                None => return None,
            };
            if n == 0 {
                return None;
            }
            char::from_u32(n)?
        }
    };
    Some((c, semi + 1))
}

/// Step 2's letters NFKD leaves whole, ASCII-ised; the Greek final sigma meets the medial one.
fn push_extra_fold(out: &mut String, c: char) {
    match c {
        'ł' => out.push('l'),
        'ø' => out.push('o'),
        'đ' | 'ð' => out.push('d'),
        'ħ' => out.push('h'),
        'ı' => out.push('i'),
        'ŧ' => out.push('t'),
        'ĸ' => out.push('k'),
        'ŋ' => out.push('n'),
        'ß' => out.push_str("ss"),
        'æ' => out.push_str("ae"),
        'œ' => out.push_str("oe"),
        'þ' => out.push_str("th"),
        'ς' => out.push('σ'),
        _ => out.push(c),
    }
}

/// Steps 1-2: entities decoded, the whole string lower-cased (`str::to_lowercase`, so a word-final `Σ` is `ς`),
/// each character through NFKD with its combining characters dropped, then the extra letters, then lower-cased
/// again.
fn char_fold(name: &str) -> String {
    let lowered = decode_entities(name).to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    for c in lowered.chars() {
        match NFKD_STRIPPED.binary_search_by_key(&c, |(k, _)| *k) {
            Ok(i) => NFKD_STRIPPED[i].1.chars().for_each(|x| push_extra_fold(&mut out, x)),
            Err(_) => push_extra_fold(&mut out, c),
        }
    }
    out.to_lowercase()
}

/// Step 3's split: the text outside the brackets (`parts`, one more than `contents`) and each bracket's content.
/// An opener's first closer of the same kind ends it, an unclosed opener runs to the end, a stray closer is a space.
fn split_brackets(s: &str) -> (Vec<String>, Vec<String>) {
    let chars: Vec<char> = s.chars().collect();
    let (mut parts, mut contents) = (vec![String::new()], Vec::new());
    let mut i = 0;
    while i < chars.len() {
        let close = match chars[i] {
            '(' => Some(')'),
            '[' => Some(']'),
            '{' => Some('}'),
            _ => None,
        };
        if let Some(close) = close {
            let end = chars[i + 1..].iter().position(|&c| c == close).map_or(chars.len(), |k| i + 1 + k);
            contents.push(chars[i + 1..end].iter().collect());
            parts.push(String::new());
            i = end + 1;
        } else {
            let c = if matches!(chars[i], ')' | ']' | '}') { ' ' } else { chars[i] };
            parts.last_mut().expect("parts is never empty").push(c);
            i += 1;
        }
    }
    (parts, contents)
}

/// Step 4: the runs of alphanumeric characters.
fn tokens_of(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).map(str::to_owned).collect()
}

// ------------------------------------------------------------------------------------------------ token lists

fn starts_with(toks: &[String], phrase: &[&str]) -> bool {
    toks.len() >= phrase.len() && toks.iter().zip(phrase).all(|(t, p)| t == p)
}

fn ends_with(toks: &[String], phrase: &[&str]) -> bool {
    toks.len() >= phrase.len() && starts_with(&toks[toks.len() - phrase.len()..], phrase)
}

fn same(toks: &[String], phrase: &[&str]) -> bool {
    toks.len() == phrase.len() && starts_with(toks, phrase)
}

/// Step 5: every phrase dropped wherever it stands, leftmost and longest first.
fn drop_anywhere(toks: Vec<String>, phrases: &[Vec<&str>]) -> Vec<String> {
    let mut out = Vec::with_capacity(toks.len());
    let mut i = 0;
    while i < toks.len() {
        match phrases.iter().find(|p| starts_with(&toks[i..], p)) {
            Some(p) => i += p.len(),
            None => {
                out.push(toks[i].clone());
                i += 1;
            }
        }
    }
    out
}

/// Step 6: phrases dropped from the end, repeatedly and longest first, never down to nothing.
fn drop_trailing(mut toks: Vec<String>, phrases: &[Vec<&str>]) -> Vec<String> {
    'strip: while !toks.is_empty() {
        for p in phrases {
            if toks.len() > p.len() && ends_with(&toks, p) {
                toks.truncate(toks.len() - p.len());
                continue 'strip;
            }
        }
        break;
    }
    toks
}

/// Step 8: every run of two or more one-character tokens made one token (`c u c` → `cuc`, C.U.C. = CUC).
fn join_initials(toks: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(toks.len());
    let mut run: Vec<String> = Vec::new();
    let flush = |run: &mut Vec<String>, out: &mut Vec<String>| {
        if run.len() >= 2 {
            out.push(run.concat());
        } else {
            out.append(run);
        }
        run.clear();
    };
    for t in toks {
        if t.chars().count() == 1 {
            run.push(t);
        } else {
            flush(&mut run, &mut out);
            out.push(t);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// Whether a bracket's tokens are only legal forms and articles (`(Medio Propio)`); one legal form alone counts.
fn legal_or_article_only(toks: &[String]) -> bool {
    let rest: Vec<String> = drop_trailing(drop_anywhere(toks.to_vec(), &LEGAL_ANY), &LEGAL_END)
        .into_iter()
        .filter(|t| !ARTICLES.contains(&t.as_str()))
        .collect();
    rest.is_empty() || LEGAL_END.iter().any(|p| same(&rest, p))
}

// ------------------------------------------------------------------------------------------------ the key

/// Which side of the pair a name is: the parent Organization cuts every bracket, the touchpoint only the allowed
/// ones.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Parent,
    Touchpoint,
}

/// What the touchpoint side reads from its parent Organization's name ([`ParentMarks::of`]); none for the parent
/// side and for G2's key, which has no parent.
#[derive(Default)]
struct ParentMarks {
    /// The parent's bracket contents, initials joined: a touchpoint bracket equal to one is cut.
    brackets: Vec<String>,
    /// The [`TP_BRACKET_PLACES`] entries the parent names anywhere, in its key or a bracket (articles kept).
    places: Vec<&'static str>,
}

impl ParentMarks {
    fn of(parent: &str) -> Self {
        let folded = char_fold(parent);
        let brackets = split_brackets(&folded).1.iter().map(|b| join_initials(tokens_of(b)).join(" ")).collect();
        let toks = tokens_of(&folded);
        let places = PLACES
            .iter()
            .filter(|(_, place)| (0..toks.len()).any(|i| starts_with(&toks[i..], place)))
            .map(|(entry, _)| *entry)
            .collect();
        ParentMarks { brackets, places }
    }

    /// Whether a touchpoint bracket naming the province or community `place` is cut: the parent names that place
    /// (one spelling of it, or an entry holding it or held by it: `murcia` / `region de murcia`), or names none of
    /// the list. The port's addition to the reference, which cut every such bracket: `Tribunal Superior de Justicia
    /// (Madrid)` under `Tribunal Superior de Justicia (Andalucía)`, or `Ayuntamiento (Toledo)` under `Alcaldía del
    /// Ayuntamiento de Madrid`, names another body and keeps its bracket.
    fn cuts_place(&self, place: &str) -> bool {
        let holds = |a: &str, b: &str| format!(" {a} ").contains(&format!(" {b} "));
        self.places.is_empty()
            || self.places.iter().any(|p| {
                holds(p, place)
                    || holds(place, p)
                    || PLACE_SPELLINGS.iter().any(|s| s.contains(p) && s.contains(&place))
            })
    }
}

/// Steps 1-4. On the touchpoint side a bracket is cut only when its content is a contact or registry label, a
/// Spanish province the parent does not contradict ([`ParentMarks::cuts_place`]), empty, only legal forms and
/// articles, or (initials joined) one of the parent's own brackets; anything else stays in the key (`(TARCAS)`,
/// `(Contratación y Compras)`).
fn fold_tokens(name: &str, side: Side, parent: &ParentMarks) -> Vec<String> {
    let (parts, contents) = split_brackets(&char_fold(name));
    let mut text = parts[0].clone();
    for (content, after) in contents.iter().zip(&parts[1..]) {
        let toks = tokens_of(content);
        let cut = side == Side::Parent || toks.is_empty() || {
            let joined = toks.join(" ");
            TP_BRACKET_LABELS.contains(&joined.as_str())
                || (TP_BRACKET_PLACES.contains(&joined.as_str()) && parent.cuts_place(&joined))
                || parent.brackets.contains(&join_initials(toks.clone()).join(" "))
                || legal_or_article_only(&toks)
        };
        text.push(' ');
        if !cut {
            text.push_str(content);
            text.push(' ');
        }
        text.push_str(after);
    }
    tokens_of(&text)
}

/// Which steps a key runs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Steps {
    /// D586-1's steps 1-9.
    All,
    /// The town key, G2's ([`bare_key`]) and the port's C4 guard: no legal-form steps 5-6, and step 7 keeps a last
    /// token that is an article. A town named like a legal form or an article (`Gemeente Epe`, `Gemeinde Au`,
    /// `Commune de Y`, `Město Aš`) is then no bare municipal word; `El Pleno` and `Junta de Gobierno Local` key as
    /// in D586-1.
    Town,
}

/// Step 7: articles and honorifics dropped; with `keep_last`, a last token that is an article stays.
fn drop_small_words(toks: Vec<String>, keep_last: bool) -> Vec<String> {
    let last = toks.len().saturating_sub(1);
    toks.into_iter()
        .enumerate()
        .filter(|(i, t)| {
            !HONORIFICS.contains(&t.as_str()) && (!ARTICLES.contains(&t.as_str()) || (keep_last && *i == last))
        })
        .map(|(_, t)| t)
        .collect()
}

/// The D586-1 key, steps 1-9: legal forms (spelled out anywhere, abbreviated at the end), then articles and
/// honorifics, then initials joined, then (touchpoint side) a leading contact label.
fn key_tokens(name: &str, side: Side, parent: &ParentMarks) -> Vec<String> {
    key_tokens_with(name, side, parent, Steps::All)
}

/// [`key_tokens`] with all its steps, or as the town key ([`Steps::Town`]).
fn key_tokens_with(name: &str, side: Side, parent: &ParentMarks, steps: Steps) -> Vec<String> {
    let mut toks = fold_tokens(name, side, parent);
    if steps == Steps::All {
        toks = drop_trailing(drop_anywhere(toks, &LEGAL_ANY), &LEGAL_END);
    }
    let mut toks = join_initials(drop_small_words(toks, steps == Steps::Town));
    if side == Side::Touchpoint
        && let Some(head) = TP_HEAD.iter().find(|h| toks.len() > h.len() && starts_with(&toks, h))
    {
        toks.drain(..head.len());
    }
    toks
}

/// F3: whether a parent bracket names a department, a registry or a review organ.
fn dept_or_review(toks: &[String]) -> bool {
    toks.iter().any(|t| {
        DEPT_OR_REVIEW.contains(&t.as_str()) || DEPT_OR_REVIEW_PREFIXES.iter().any(|p| t.starts_with(p))
    }) || (0..toks.len()).any(|i| starts_with(&toks[i..], DEPT_OR_REVIEW_PHRASE))
}

/// The parent's key, with F3's department, registry and review brackets left in (every other bracket cut).
fn parent_key(parent: &str) -> Vec<String> {
    let (parts, contents) = split_brackets(&char_fold(parent));
    let kept: Vec<bool> = contents.iter().map(|b| dept_or_review(&tokens_of(b))).collect();
    if !kept.contains(&true) {
        return key_tokens(parent, Side::Parent, &ParentMarks::default());
    }
    let mut text = parts[0].clone();
    for ((content, after), kept) in contents.iter().zip(&parts[1..]).zip(kept) {
        text.push(' ');
        if kept {
            text.push_str(content);
            text.push(' ');
        }
        text.push_str(after);
    }
    key_tokens(&text, Side::Parent, &ParentMarks::default())
}

/// F2: whether the parent prints a legal form (steps 1-6 shorten its tokens).
fn parent_has_form(parent: &str) -> bool {
    let toks = fold_tokens(parent, Side::Parent, &ParentMarks::default());
    let n = toks.len();
    drop_trailing(drop_anywhere(toks, &LEGAL_ANY), &LEGAL_END).len() < n
}

/// A tier-C key with its leading municipal word read as `ayuntamiento` (F1's [`MUNI`]).
fn muni(toks: &[String]) -> Vec<String> {
    let mut out = toks.to_vec();
    if let Some(head) = out.first_mut()
        && MUNI.contains(&head.as_str())
    {
        "ayuntamiento".clone_into(head);
    }
    out
}

/// One leading organ phrase cut from the parent key: the longest that leads it and leaves something.
fn organ_split(p: &[String]) -> (Option<&'static [&'static str]>, &[String]) {
    match ORGANS.iter().find(|o| p.len() > o.len() && starts_with(p, o)) {
        Some(o) => (Some(o.as_slice()), &p[o.len()..]),
        None => (None, p),
    }
}

/// C0's synonym canonicalisation at the head of a key that keeps something after the organ.
fn organ_syn(toks: &[String]) -> Vec<String> {
    synonym(toks, false)
}

/// C4's: the synonym also when it is the whole key (`Junta Gobierno Local` alone is `junta gobierno`).
fn syn_whole(toks: &[String]) -> Vec<String> {
    synonym(toks, true)
}

fn synonym(toks: &[String], whole: bool) -> Vec<String> {
    for (k, v) in SYNONYMS.iter() {
        if (toks.len() > k.len() || (whole && toks.len() == k.len())) && starts_with(toks, k) {
            return v.iter().map(|s| (*s).to_owned()).chain(toks[k.len()..].iter().cloned()).collect();
        }
    }
    toks.to_vec()
}

/// D586-1: the tier at which a touchpoint name names its parent Organization's name (both as published), or
/// `None` (it mints, or meets the gate). T is the touchpoint's key, P the parent's ([`key_tokens`], [`parent_key`]):
///
/// - A: T == P.
/// - C0: T and P are equal once a leading organ synonym is canonicalised on both.
/// - C1: an organ was cut from P (the entity E), and T == E — a one-token E only when the parent prints a legal form
///   (F2: `GESPESA SAU` under `Gerencia de GESPESA, S.A.U.` aliases, `Madrid` under `Asamblea de Madrid` does not),
///   and no cut of a presidency, direction, assembly, general council or officer over a whole government (F4).
/// - C2: E starts with a municipal word and a place, and T equals it, municipal words read alike.
/// - C3: T is E's place (the bare town under its town hall).
/// - C3r: T is a municipal word plus P, and P has no municipal head.
/// - C4: T is P's leading organ (synonyms canonicalised, also when the synonym is all of T), or exactly one
///   municipal word while E starts with a municipal word and a place — and T's town key equals T (the port's
///   guard: `Gemeente Epe` and `Gemeinde Au` are no bare `gemeente` / `gemeinde`).
///
/// Never: an organ cut from the touchpoint, a one-word place under anything but its town hall (F2; a place of two
/// or more words under an organ is read as the body, C1: `Valverde de Leganés` under `Alcaldía de Valverde de
/// Leganés`), a prefix, a suffix, an overlap, a typo or an acronym; the parent plus a department, an address or a
/// branch (tier B: `Stadt Fürth - Zentrale Vergabestelle` under `Stadt Fürth`) mints.
pub(super) fn alias_tier(touchpoint: &str, organization: &str) -> Option<Tier> {
    let marks = ParentMarks::of(organization);
    let t = key_tokens(touchpoint, Side::Touchpoint, &marks);
    let p = parent_key(organization);
    if t.is_empty() || p.is_empty() {
        return None;
    }
    if t == p {
        return Some(Tier::A);
    }
    if organ_syn(&t) == organ_syn(&p) {
        return Some(Tier::C0);
    }
    let (organ, e) = organ_split(&p);
    let entity_ok = e.len() >= 2 || (e.len() == 1 && e[0].chars().count() >= 4);
    let mut ok = entity_ok && !(e.len() == 1 && !parent_has_form(organization));
    if organ.is_some_and(|o| UMBRELLA_ORGANS.contains(&o.join(" ").as_str()))
        && e.first().is_some_and(|head| UMBRELLA_HEADS.contains(&head.as_str()))
    {
        ok = false;
    }
    if organ.is_some() && ok && t == e {
        return Some(Tier::C1);
    }
    let (em, tm) = (muni(e), muni(&t));
    if em[0] == "ayuntamiento" && em.len() >= 2 {
        if tm == em {
            return Some(Tier::C2);
        }
        if t[..] == em[1..] {
            return Some(Tier::C3);
        }
    }
    if tm[0] == "ayuntamiento" && tm.len() >= 2 && !MUNI.contains(&p[0].as_str()) && p[..] == tm[1..] {
        return Some(Tier::C3r);
    }
    let own_organ = organ.is_some_and(|organ| {
        syn_whole(&t) == syn_whole(&organ.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
    });
    let town_hall_word = same(&tm, &["ayuntamiento"]) && em[0] == "ayuntamiento" && em.len() >= 2;
    // C4 needs a touchpoint that IS the bare word as printed: its town key (no legal-form steps, a last article
    // kept; the module doc's deviations 1 and 2) must equal its key, as G2's key reads it for the same reason.
    // `Gemeente Epe` (EPE reads as a legal form) and `Gemeinde Au` (`au` reads as an article) under `Gemeente
    // Utrecht` are other municipalities and mint, as decision.md's gate test requires for Epe; `final.py`
    // aliased both at C4. Neither moves any of the 3,751 saved pairs (no saved C4 touchpoint, in the pairs or
    // either window, ends in a legal-form or article token).
    ((own_organ || town_hall_word) && key_tokens_with(touchpoint, Side::Touchpoint, &marks, Steps::Town) == t)
        .then_some(Tier::C4)
}

// ------------------------------------------------------------------------------------------------ the gate

/// Whether the folded name holds the phrase as whole words.
fn words_in(padded: &str, phrase: &str) -> bool {
    padded.len() > 2 && padded.contains(&format!(" {phrase} "))
}

/// HONORIFIC (B): the raw name's first word is a title (read case-insensitively as the reference's `re` reads it,
/// [`re_lower`]), and another word starting with a word character follows; `m.` not before a one-letter
/// abbreviation (`M. I. Ayuntamiento`) nor (the port's deviation 4) before a Spanish body's style (`M. Ilustre
/// Colegio de Abogados de Madrid`, `M. Iltre. Cabildo Insular de Tenerife`); G3: not before an office word; and the
/// name holds no commercial legal form and no public-law stem.
fn honorific(name: &str) -> bool {
    let mut words = name.split(py_space).filter(|w| !w.is_empty());
    let (Some(title), Some(next)) = (words.next(), words.next()) else { return false };
    let lower: String = title.chars().map(re_lower).collect();
    if !HONORIFIC_TITLES.contains(&lower.as_str()) && !HONORIFIC_TITLES_EXACT.contains(&title) {
        return false;
    }
    let mut after = next.chars();
    let (first, second) = (after.next(), after.next());
    if !first.is_some_and(word_char) || (lower == "m." && second == Some('.')) {
        return false;
    }
    if lower == "m." && tokens_of(&char_fold(next)).first().is_some_and(|w| HONORIFICS.contains(&w.as_str())) {
        return false;
    }
    let office = crate::partyname::fold(next);
    if office.split(' ').next().is_some_and(|w| OFFICE_NEXT.contains(&w)) {
        return false;
    }
    !role_census::has_commercial_form(name) && !role_census::has_public_stem(name)
}

/// The name with every `(…)` span replaced by a space, innermost first, until none is left.
fn unbracketed(name: &str) -> String {
    let mut current: Vec<char> = name.chars().collect();
    loop {
        let mut out = Vec::with_capacity(current.len());
        let mut i = 0;
        while i < current.len() {
            if current[i] == '('
                && let Some(k) = current[i + 1..].iter().position(|&c| c == '(' || c == ')')
                && current[i + 1 + k] == ')'
            {
                out.push(' ');
                i += k + 2;
            } else {
                out.push(current[i]);
                i += 1;
            }
        }
        if out == current {
            return out.into_iter().collect();
        }
        current = out;
    }
}

/// The index after the whitespace run at `i`.
fn skip_space(c: &[char], mut i: usize) -> usize {
    while i < c.len() && py_space(c[i]) {
        i += 1;
    }
    i
}

/// One character as the reference's case-insensitive patterns (Python `re`, `(?i)`) compare it: its simple
/// lower-case mapping, so `İ` is `i` (`char::to_lowercase` gives `i` plus a combining dot), and `re`'s case-folding
/// fixes for the two letters the patterns spell, `ı` as `i` and `ſ` as `s`.
fn re_lower(c: char) -> char {
    match c {
        'İ' | 'ı' => 'i',
        'ſ' => 's',
        _ => {
            let mut lower = c.to_lowercase();
            match (lower.next(), lower.next()) {
                (Some(l), None) => l,
                _ => c,
            }
        }
    }
}

/// Whether `c[i]` lower-cases to exactly `x` ([`re_lower`]).
fn lower_is(c: &[char], i: usize, x: char) -> bool {
    i < c.len() && re_lower(c[i]) == x
}

/// The index after the literal `word` at `i` (case-insensitive), if it stands there.
fn literal(c: &[char], i: usize, word: &str) -> Option<usize> {
    word.chars().enumerate().all(|(k, x)| lower_is(c, i + k, x)).then(|| i + word.chars().count())
}

/// One whitespace character or more at `i`, then a decimal digit.
fn spaces_then_digit(c: &[char], i: usize) -> bool {
    let k = skip_space(c, i);
    k > i && k < c.len() && decimal(c[k])
}

/// The reference's raw article pattern, `(?i)(?<!\w)(?:art\s*[.\-]\s*|art[ií]culos?\s+|artigo\s+|articolo\s+|
/// artikel\s+)\d`, on the unbracketed name.
fn raw_article(name: &str) -> bool {
    let c: Vec<char> = name.chars().collect();
    (0..c.len()).any(|i| {
        if i > 0 && word_char(c[i - 1]) {
            return false;
        }
        let Some(art) = literal(&c, i, "art") else { return false };
        let k = skip_space(&c, art);
        if k < c.len() && (c[k] == '.' || c[k] == '-') {
            let k = skip_space(&c, k + 1);
            if k < c.len() && decimal(c[k]) {
                return true;
            }
        }
        if (lower_is(&c, art, 'i') || lower_is(&c, art, 'í'))
            && let Some(k) = literal(&c, art + 1, "culo")
            && (spaces_then_digit(&c, k) || (lower_is(&c, k, 's') && spaces_then_digit(&c, k + 1)))
        {
            return true;
        }
        ["igo", "icolo", "ikel"].iter().any(|rest| literal(&c, art, rest).is_some_and(|k| spaces_then_digit(&c, k)))
    })
}

/// The reference's folded pattern, `(?:^| )(?:el|del|y|en|segun|al) art \d`: a lead word, `art`, then a word
/// starting with a digit.
fn folded_article(folded: &str) -> bool {
    let words: Vec<&str> = folded.split(' ').collect();
    words.windows(3).any(|w| {
        LEGALREF_LEADS.contains(&w[0]) && w[1] == "art" && w[2].chars().next().is_some_and(decimal)
    })
}

/// G1b's numbered act or sub-article in the raw unbracketed name (`\d\s*[/.]\s*\d`: `39/2015`, `16.4`).
fn numbered(bare: &str) -> bool {
    let c: Vec<char> = bare.chars().collect();
    (0..c.len()).any(|i| {
        if !decimal(c[i]) {
            return false;
        }
        let k = skip_space(&c, i + 1);
        if k >= c.len() || !matches!(c[k], '/' | '.') {
            return false;
        }
        let k = skip_space(&c, k + 1);
        k < c.len() && decimal(c[k])
    })
}

/// LEGALREF (D), read outside every `(…)`: an article number (`art. 16`, `art- 51.3`, `artículo 44`, `y art 16`).
/// G1: not on a name citing an EU act, nor one holding a public-law stem or a commercial form. G1b: only beside a
/// statute cue (a law or review-body word, or a numbered act or sub-article).
fn legalref(name: &str, folded: &str) -> bool {
    let bare = unbracketed(name);
    if !raw_article(&bare) && !folded_article(&crate::partyname::fold(&bare)) {
        return false;
    }
    let words = || folded.split(' ');
    let body = role_census::has_public_stem(name) || role_census::has_commercial_form(name);
    if body || words().any(|w| EU_ACT.contains(&w)) {
        return false;
    }
    words().any(|w| LAW_CUE.contains(&w)) || numbered(&bare)
}

/// G2's key: the D586-1 key's steps 1-4, 7 and 8 on the touchpoint side with no parent, as the town key
/// ([`Steps::Town`]): WITHOUT the legal-form steps (`Gemeente Epe` keeps `epe`, and is no bare `gemeente`), and
/// step 7 keeping a last article (`Gemeinde Au` keeps `au`: the port's deviation 2). No leading contact label is
/// cut.
fn bare_key(name: &str) -> String {
    let toks = fold_tokens(name, Side::Touchpoint, &ParentMarks::default());
    join_initials(drop_small_words(toks, true)).join(" ")
}

/// BARE (F, G2): the key is exactly one organ, officer, town-hall or registry word — another body's label, since
/// C4 has already aliased the parent's own (minting it would bind a cross-buyer row: 2130626 "Pleno").
fn bare(name: &str) -> bool {
    let key = bare_key(name);
    let key = key.as_str();
    ORGAN_PHRASES.contains(&key)
        || ORGAN_SYNONYMS.iter().any(|(k, _)| *k == key)
        || MUNI.contains(&key)
        || MUNI_POLISH.contains(&key)
        || BARE_LABELS.contains(&key)
}

/// D586-2: whether a touchpoint name, as published, is no party by one of the six touchpoint-only tests — WHOLE
/// (`Dirección Principal`), HONORIFIC (`Monsieur Anthony LINTILHAC`), PROSE (`Procede recurso especial…`),
/// LEGALREF (`TACRC y art. 16 Ley 39/2015`), PLATFORM (`TUTTOGARE RACCONIGI`) and BARE (`Registro General`). "Folded"
/// is [`crate::partyname::fold`], the census's fold.
pub(super) fn gated(name: &str) -> bool {
    let folded = crate::partyname::fold(name);
    let padded = format!(" {folded} ");
    WHOLE.contains(&folded.as_str())
        || honorific(name)
        || PROSE.iter().any(|p| words_in(&padded, p))
        || legalref(name, &folded)
        || PLATFORM.iter().any(|p| words_in(&padded, p))
        || bare(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One fixture row: `[touchpoint, parent, tier or null, gated and not aliased]` (vectors) or `["tier",
    /// touchpoint, parent, tier or null]` / `["gate", name, null, gated]` (selftest), as `final.py --jsonl` writes
    /// them.
    fn rows(jsonl: &str) -> Vec<Vec<serde_json::Value>> {
        jsonl.lines().filter(|l| !l.trim().is_empty()).map(|l| serde_json::from_str(l).expect("a JSON row")).collect()
    }

    fn label(tier: Option<Tier>) -> Option<&'static str> {
        tier.map(Tier::label)
    }

    /// Every vector row checked; the mismatches, the first 20 printed.
    fn check_vectors(jsonl: &str) -> (usize, Vec<String>) {
        let rows = rows(jsonl);
        let mut bad = Vec::new();
        for row in &rows {
            let (tp, parent) = (row[0].as_str().expect("touchpoint"), row[1].as_str().expect("parent"));
            let want = (row[2].as_str(), row[3].as_bool().expect("gated"));
            let tier = label(alias_tier(tp, parent));
            let got = (tier, tier.is_none() && gated(tp));
            if got != want {
                bad.push(format!("{tp:?} under {parent:?}: want {want:?}, got {got:?}"));
            }
        }
        (rows.len(), bad)
    }

    /// The port check (decision.md, unit 2): the tier and the gated flag of the saved pairs, as the oracle
    /// (`final.py`) answers them. The committed fixture is `port_subset.py`'s stratified subset of
    /// `fixtures/vectors.jsonl` (all 3,751 pairs, 452 KB, over what the build embeds): every pair the oracle
    /// aliases (867), every one it gates (92) and 200 salted minted ones (salt `586-u2-port`).
    #[test]
    fn the_port_matches_the_reference_on_every_saved_pair() {
        let (n, bad) = check_vectors(include_str!("../../tests/fixtures/touchpoint_586/vectors.jsonl"));
        assert_eq!(n, 1_159, "the subset's rows");
        assert!(bad.is_empty(), "{} of {n} pairs disagree:\n{}", bad.len(), bad[..bad.len().min(20)].join("\n"));
    }

    /// The same check over all 3,751 saved pairs, read from the decision's folder at run time (the file is over
    /// what the build embeds). `cargo test … -- --ignored the_port_matches_the_reference_on_all`.
    #[test]
    #[ignore = "reads .scratch/tender-db/586-u1/fixtures/vectors.jsonl at run time"]
    fn the_port_matches_the_reference_on_all_3751_saved_pairs() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../.scratch/tender-db/586-u1/fixtures/vectors.jsonl");
        let jsonl = std::fs::read_to_string(path).expect("the decision's vectors.jsonl");
        let (n, bad) = check_vectors(&jsonl);
        assert_eq!(n, 3_751);
        assert!(bad.is_empty(), "{} of {n} pairs disagree:\n{}", bad.len(), bad[..bad.len().min(20)].join("\n"));
    }

    /// The oracle's 80 named cases (`final.py`'s SELF and GATE_SELF): unit 2's tests, check.md's must-not-alias
    /// pairs (F1-F4), C4, and the gate's drops and keeps (G1-G3).
    #[test]
    fn the_named_cases_hold() {
        let rows = rows(include_str!("../../tests/fixtures/touchpoint_586/selftest.jsonl"));
        assert_eq!(rows.len(), 80);
        let mut bad = Vec::new();
        for row in &rows {
            let name = row[1].as_str().expect("name");
            match row[0].as_str() {
                Some("tier") => {
                    let got = label(alias_tier(name, row[2].as_str().expect("parent")));
                    if got != row[3].as_str() {
                        bad.push(format!("tier {name:?} under {}: want {}, got {got:?}", row[2], row[3]));
                    }
                }
                Some("gate") => {
                    let got = gated(name);
                    if got != row[3].as_bool().expect("want") {
                        bad.push(format!("gate {name:?}: want {}, got {got}", row[3]));
                    }
                }
                other => panic!("unknown row kind {other:?}"),
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// Every list entry is written as its own fold: the key's lists as `char_fold` and the tokens make them, the
    /// gate's phrase lists as `partyname::fold` makes them, the titles lower-case; no list repeats an entry.
    #[test]
    fn every_list_entry_is_its_own_fold() {
        let key_lists: &[(&str, &[&str])] = &[
            ("LEGAL_ANYWHERE", LEGAL_ANYWHERE),
            ("LEGAL_TRAILING", LEGAL_TRAILING),
            ("ARTICLES", ARTICLES),
            ("HONORIFICS", HONORIFICS),
            ("TP_BRACKET_LABELS", TP_BRACKET_LABELS),
            ("TP_BRACKET_PLACES", TP_BRACKET_PLACES),
            ("TP_HEAD_LABELS", TP_HEAD_LABELS),
            ("DEPT_OR_REVIEW", DEPT_OR_REVIEW),
            ("DEPT_OR_REVIEW_PREFIXES", DEPT_OR_REVIEW_PREFIXES),
            ("DEPT_OR_REVIEW_PHRASE", DEPT_OR_REVIEW_PHRASE),
            ("ORGAN_PHRASES", ORGAN_PHRASES),
            ("UMBRELLA_ORGANS", UMBRELLA_ORGANS),
            ("UMBRELLA_HEADS", UMBRELLA_HEADS),
            ("MUNI", MUNI),
            ("MUNI_POLISH", MUNI_POLISH),
            ("BARE_LABELS", BARE_LABELS),
        ];
        let synonyms: Vec<&str> = ORGAN_SYNONYMS.iter().flat_map(|(k, v)| [*k, *v]).collect();
        for (list, entries) in key_lists.iter().copied().chain([("ORGAN_SYNONYMS", synonyms.as_slice())]) {
            for e in entries {
                assert_eq!(tokens_of(&char_fold(e)).join(" "), *e, "{list}: {e:?} is not its own key fold");
            }
        }
        let gate_lists: &[(&str, &[&str])] = &[
            ("WHOLE", WHOLE),
            ("PROSE", PROSE),
            ("PLATFORM", PLATFORM),
            ("OFFICE_NEXT", OFFICE_NEXT),
            ("LEGALREF_LEADS", LEGALREF_LEADS),
            ("EU_ACT", EU_ACT),
            ("LAW_CUE", LAW_CUE),
        ];
        for (list, entries) in gate_lists {
            for e in *entries {
                assert_eq!(crate::partyname::fold(e), *e, "{list}: {e:?} is not its own fold");
            }
        }
        for t in HONORIFIC_TITLES {
            assert_eq!(t.to_lowercase(), *t, "HONORIFIC_TITLES: {t:?} is matched lower-cased");
        }
        let every = key_lists.iter().chain(gate_lists).copied().chain([("HONORIFIC_TITLES", HONORIFIC_TITLES)]);
        for (list, entries) in every {
            let mut seen = std::collections::BTreeSet::new();
            for e in entries {
                assert!(seen.insert(*e), "{list}: {e:?} twice");
            }
        }
        assert!(ORGAN_SYNONYMS.iter().all(|(k, _)| ORGAN_PHRASES.contains(k)), "every synonym is an organ");
        assert!(UMBRELLA_ORGANS.iter().all(|o| ORGAN_PHRASES.contains(o)), "every F4 organ is an organ");
    }

    /// The port's C4 guard and G2's key (deviations 1 and 2): a touchpoint that is a bare municipal word or organ
    /// only once the legal-form steps or the articles cut a town's name (`Epe` reads as E.P.E., `Au`, `Y`, `Aš`
    /// and `Les` as articles) is no bare word — another municipality under a town hall mints, and BARE does not
    /// drop it under any other parent — while the printed bare words still alias, or meet BARE.
    #[test]
    fn a_town_named_like_a_legal_form_is_no_bare_town_hall_word() {
        assert_eq!(alias_tier("Gemeente Epe", "Gemeente Utrecht"), None);
        assert_eq!(alias_tier("Gemeente Epe", "Alcaldía del Ayuntamiento de Valverde de Leganés"), None);
        assert_eq!(alias_tier("Gemeente Epe", "Gemeente Epe"), Some(Tier::A), "its own name still aliases");
        assert_eq!(alias_tier("Ayuntamiento", "Gemeente Utrecht"), Some(Tier::C4));
        assert_eq!(alias_tier("Excmo. Ayuntamiento", "Alcaldía del Ayuntamiento de Biar"), Some(Tier::C4));
        assert_eq!(alias_tier("Ayuntamiento", "Alcaldía del Ayuntamiento de Biar"), Some(Tier::C4));
        assert_eq!(alias_tier("El Pleno", "Pleno de la Diputación Provincial de Huelva"), Some(Tier::C4));
        assert!(!gated("Gemeente Epe"), "G2's key keeps `epe` too");
        for (town, other) in [
            ("Gemeinde Au", "Stadt Bregenz"),
            ("Commune de Y", "Commune de Ham"),
            ("Město Aš", "Město Cheb"),
            ("Mesto As", "Mesto Brno"),
            ("Ajuntament de Les", "Ajuntament de Vielha e Mijaran"),
        ] {
            assert_eq!(alias_tier(town, other), None, "{town:?} under {other:?} is another municipality");
            assert_eq!(alias_tier(town, "Landratsamt Breisgau-Hochschwarzwald"), None, "{town:?}");
            assert!(!gated(town), "{town:?}: G2's key keeps the last article ({:?})", bare_key(town));
        }
        assert_eq!(bare_key("Gemeinde Au"), "gemeinde au");
        assert_eq!(alias_tier("Gemeinde Au", "Gemeinde Au"), Some(Tier::A), "its own name still aliases");
        for label in ["El Pleno", "Junta de Gobierno Local", "Ayuntamiento", "Registro General"] {
            assert!(gated(label), "{label:?}: a bare word whose last token is no article still meets BARE");
        }
        assert_eq!(bare_key("El Pleno"), "pleno");
    }

    /// Deviation 3: a touchpoint's province bracket is cut beside a parent that names that place (in any listed
    /// spelling, or an entry holding it) or none of the list, and kept beside one naming another place — two
    /// courts, two provincial councils or two town halls of different places do not alias.
    #[test]
    fn a_province_bracket_is_cut_only_beside_the_same_place_or_none() {
        let mints = [
            ("Tribunal Superior de Justicia (Madrid)", "Tribunal Superior de Justicia (Andalucía)"),
            ("Diputación Provincial (Sevilla)", "Diputación Provincial (Huelva)"),
            ("Ayuntamiento (Toledo)", "Alcaldía del Ayuntamiento de Madrid"),
        ];
        for (tp, parent) in mints {
            assert_eq!(alias_tier(tp, parent), None, "{tp:?} under {parent:?}");
        }
        let aliases = [
            ("Ayuntamiento de Lucena (Córdoba)", "Ayuntamiento de Lucena", Tier::A),
            ("Excmo. Ayuntamiento de Lucena (Córdoba)", "Ayuntamiento de Lucena", Tier::A),
            ("Ayuntamiento de Sevilla (Sevilla)", "Ayuntamiento de Sevilla", Tier::A),
            ("Diputació de Lleida (Lérida)", "Diputació de Lleida", Tier::A),
            ("Universidad de Murcia (Región de Murcia)", "Universidad de Murcia", Tier::A),
            ("Diputación de A Coruña (La Coruña)", "Diputación de A Coruña", Tier::A),
            ("Ayuntamiento de Lucena (Córdoba)", "Alcaldía del Ayuntamiento de Lucena", Tier::C1),
        ];
        for (tp, parent, tier) in aliases {
            assert_eq!(alias_tier(tp, parent), Some(tier), "{tp:?} under {parent:?}");
        }
        for row in PLACE_SPELLINGS {
            for place in *row {
                assert!(TP_BRACKET_PLACES.contains(place), "PLACE_SPELLINGS: {place:?} is a TP_BRACKET_PLACES entry");
                assert_eq!(PLACE_SPELLINGS.iter().filter(|r| r.contains(place)).count(), 1, "{place:?} in one row");
            }
        }
    }

    /// check.md's pairs 31-42: the Unicode traps fold alike on both sides (Turkish İ, ß against SS, Greek final
    /// sigma and tonos, Catalan l·l and ŀ, decomposed accents, fullwidth letters, U+037E against `;`), and an entity
    /// decodes once.
    #[test]
    fn the_unicode_traps_fold_alike() {
        let key = |name: &str| key_tokens(name, Side::Parent, &ParentMarks::default()).join(" ");
        assert_eq!(key("İSTANBUL BÜYÜKŞEHİR BELEDİYESİ"), "istanbul buyuksehir belediyesi", "Turkish İ");
        assert_eq!(key("Straßenbauamt"), key("STRASSENBAUAMT"), "ß / SS");
        assert_eq!(key("ΔΗΜΟΣ ΑΘΗΝΑΙΩΝ"), key("Δήμος Αθηναίων"), "Greek final sigma and tonos");
        assert_eq!(key("Δήμος"), "δημοσ");
        assert_eq!(key("Col·legi d'Advocats"), key("Coŀlegi d'Advocats"), "Catalan l·l and ŀ");
        assert_eq!(key("Col·legi d'Advocats"), "col legi advocats");
        assert_eq!(key("Caf\u{65}\u{301} Municipal"), key("Café Municipal"), "decomposed accents");
        assert_eq!(key("ＭＡＤＲＩＤ Ａｒｅｎａ"), "madrid arena", "fullwidth letters");
        assert_eq!(
            key("Gmina Panki ul. Tysiąclecia 5\u{37e} 42-140 Panki"),
            key("Gmina Panki ul. Tysiąclecia 5; 42-140 Panki"),
            "U+037E against ;"
        );
        assert_eq!(
            char_fold("l&#8217;attuazione &amp;amp; &#0; &#xD800; &#12345678;"),
            "l\u{2019}attuazione &amp; &#0; &#xd800; &#12345678;",
            "decoded once; an invalid code point or an eighth digit stays literal"
        );
        assert_eq!(char_fold("A&nbsp;B"), "a b");
    }

    /// The reference's two regular expressions written out as scans: what the raw article pattern and HONORIFIC's
    /// lead accept and refuse.
    #[test]
    fn the_scans_read_as_the_reference_patterns() {
        let hits = ["art. 16", "Art-3", "art -  51.3", "ARTÍCULO 44", "articulos 5", "artigo 7", "articolo 120", "Artikel 2"];
        for hit in hits.into_iter().chain(["x (art.5"]) {
            assert!(raw_article(hit), "{hit:?}");
        }
        for miss in ["Art 21 Gallery", "smart. 3", "articulo s5", "articulos5", "artículo", "art.", "Artikel x"] {
            assert!(!raw_article(miss), "{miss:?}");
        }
        assert!(folded_article("tribunal de acuerdo con el art 44 de la lcsp"));
        assert!(!folded_article("hotel art 5"));
        assert_eq!(unbracketed("a (b (c) d) e (f"), "a   e (f");
        assert!(numbered("Ley 39/2015") && numbered("16 . 4") && !numbered("Ley 39 2015"));
        let titled = ["Mr John Smith", "Mrs. Jane Doe", "MME Julie ROLLIN", "dott marco lesto", "M. Damien SIMON"];
        // `re`'s (?i) reads İ and ı as i, and ſ as s.
        let turkish = ["İng. Mehmet Yılmaz", "ıng. Mehmet Yilmaz", "MONSİEUR Jean Dupont", "Mſ. Jane Doe"];
        for drop in titled.into_iter().chain(turkish).chain(["  Herr Thomas Müller"]) {
            assert!(honorific(drop), "{drop:?}");
        }
        assert!(raw_article("ARTİCULO 5") && raw_article("artıculo 5") && raw_article("ARTİKEL 2"));
        assert!(gated("ARTİCULO 5 de la Ley 9/2017"), "LEGALREF reads it as the reference does");
        let untitled = ["MR John", "MRS Logística", "M. I. Ayuntamiento de Elche", "M. I.Ayuntamiento", "Herrera de Pisuerga"];
        // Deviation 4: `M.` before a Spanish body's style abbreviates it (Muy Ilustre), as `M. I.` does.
        let styled = ["M. Ilustre Colegio de Abogados de Madrid", "M. Iltre. Cabildo Insular de Tenerife"];
        let others = ["Monsieur", "Mme.Julie", "Ing. Büro-Planung", "Mr. Bricolage SA"];
        for keep in untitled.into_iter().chain(styled).chain(others) {
            assert!(!honorific(keep), "{keep:?}");
        }
    }
}
