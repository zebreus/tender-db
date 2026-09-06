//! Issue 330: the trailing postal block of a multi-line organization name.
//!
//! 356,749 organization names carry a line break, and the census settled that
//! every one was PUBLISHED that way. Most are wrapped names, department
//! suffixes, `c/o` lines or consortium lists, where the break is just
//! whitespace to `match_norm`. A minority append the postal address:
//!
//! ```text
//! Landkreis Saalekreis
//! Domplatz 9
//! 06217 Merseburg
//! ```
//!
//! whose name key is then a strict token SUPERSET of the clean spelling's, so
//! the row fails to find its twin — the E0 census filed exactly that specimen
//! under `contained` instead of `agree`. The failure is uniformly under-merge
//! (a superset key matches fewer things, never the wrong ones), which is why
//! this is a normalisation question and not an incident.
//!
//! This module recognises that trailing block so a key builder can drop it. It
//! is a MEASUREMENT input first: the name-pollution census counts what a strip
//! would change (twins gained, keys colliding) before any key is built from
//! it. The stored `organizations.name` is never touched — issue 328's precedent
//! is that the published value stays and only derived values move.
//!
//! Conservative by construction: the LAST line must be a postcode-and-locality
//! line; the line(s) before it go too only when street-shaped; at least one
//! line always survives as the name; and a bare four-digit code (the
//! AT/CH/BE/DK/HU/… shape, which is also a YEAR) needs corroboration — a
//! street line before it or a country prefix (`A-1010 Wien`) — so
//! `Expo\n2000 Hannover` keeps its year.
//!
//! Known asymmetry: the other shapes need no corroboration, so a trailing
//! `<five digits> <word>` line that is not an address (a case or budget
//! number with a label) is stripped too. Accepted while this feeds only the
//! census, whose listing shows every stripped value for a reader; a key
//! builder consuming it would want the street-line corroboration for every
//! shape, at the cost of the bare `Stadt Mainz\n55116 Mainz` form.

/// `Some(name without its trailing postal block)`, the surviving lines joined
/// with single spaces, when the name ends in one; `None` when nothing
/// recognisable trails it — including every single-line name, which is not in
/// the class this measures.
pub fn strip_trailing_address(name: &str) -> Option<String> {
    let lines: Vec<&str> = name
        .split(['\n', '\r'])
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() < 2 {
        return None;
    }
    let last = lines.len() - 1;
    let shape = postcode_locality(lines[last])?;
    // Walk back over up to two street-shaped lines, always keeping one line
    // as the name.
    let mut cut = last;
    while cut > 1 && last - cut < 2 && street_line(lines[cut - 1]) {
        cut -= 1;
    }
    if shape == Shape::FourDigit && cut == last {
        return None;
    }
    Some(lines[..cut].join(" "))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    /// `55116 Mainz`, `75008 Paris`, `28013 Madrid`, `00184 Roma`.
    FiveDigit,
    /// `1010 Wien`, `8000 Zürich`, `1000 Bruxelles` — also what a year looks like.
    FourDigit,
    /// `02-222 Warszawa`, `1000-001 Lisboa`.
    Dashed,
    /// `110 00 Praha`, `1234 AB Amsterdam`.
    Spaced,
    /// `D-14974 Ludwigsfelde`, `A-1010 Wien`, `PL-02-222 Warszawa`.
    Prefixed,
}

/// The postcode-and-locality line: an optional one/two-letter country prefix
/// with a dash, a postcode in one of the European shapes, then a locality of
/// letters (with `/`, `()`, `-`, `.`, `,`, `'` allowed, and an optional
/// trailing district number such as `Praha 1`).
fn postcode_locality(line: &str) -> Option<Shape> {
    let mut rest = line;
    let mut prefixed = false;
    let b = rest.as_bytes();
    if let Some(dash) = b.iter().position(|&c| c == b'-') {
        if (1..=2).contains(&dash)
            && b[..dash].iter().all(u8::is_ascii_uppercase)
            && b.get(dash + 1).is_some_and(u8::is_ascii_digit)
        {
            rest = &rest[dash + 1..];
            prefixed = true;
        }
    }
    let digits = |s: &str, n: usize| -> bool {
        s.len() >= n
            && s.as_bytes()[..n].iter().all(u8::is_ascii_digit)
            && !s.as_bytes().get(n).is_some_and(u8::is_ascii_digit)
    };
    let leading = rest.bytes().take_while(u8::is_ascii_digit).count();
    let (code_len, shape) = match leading {
        5 => (5, Shape::FiveDigit),
        4 => {
            let tail = &rest[4..];
            if tail.starts_with('-') && digits(&tail[1..], 3) {
                (8, Shape::Dashed)
            } else if tail.starts_with(' ')
                && tail.len() >= 3
                && tail.as_bytes()[1..3].iter().all(u8::is_ascii_uppercase)
                && !tail.as_bytes().get(3).is_some_and(u8::is_ascii_alphanumeric)
            {
                (7, Shape::Spaced)
            } else {
                (4, Shape::FourDigit)
            }
        }
        3 => {
            let tail = &rest[3..];
            if tail.starts_with(' ') && digits(&tail[1..], 2) {
                (6, Shape::Spaced)
            } else {
                return None;
            }
        }
        2 => {
            let tail = &rest[2..];
            if tail.starts_with('-') && digits(&tail[1..], 3) {
                (6, Shape::Dashed)
            } else {
                return None;
            }
        }
        _ => return None,
    };
    let after = &rest[code_len..];
    // The locality is separated from the code by whitespace (a comma before it
    // is tolerated: `55116, Mainz` occurs).
    let locality = after.trim_start_matches(',').trim_start();
    if locality.len() == after.len() || locality.is_empty() || locality.chars().count() > 48 {
        return None;
    }
    let tokens: Vec<&str> = locality.split_whitespace().collect();
    let word = |t: &str| {
        t.chars().next().is_some_and(char::is_alphabetic)
            && t.chars().all(|c| c.is_alphabetic() || "/().-,'’".contains(c))
    };
    let district = |t: &str| (1..=2).contains(&t.len()) && t.bytes().all(|c| c.is_ascii_digit());
    let (head, tail) = tokens.split_at(tokens.len() - 1);
    let ok = head.iter().all(|t| word(t)) && (word(tail[0]) || (!head.is_empty() && district(tail[0])));
    if !ok {
        return None;
    }
    Some(if prefixed { Shape::Prefixed } else { shape })
}

/// Street tokens as whole words (lower-cased): the prefix forms that carry the
/// number after them.
const STREET_WORDS: &[&str] = &[
    "ul.", "ul", "ulica", "al.", "al", "aleja", "os.", "pl.", "plac", "ulice", "nám.", "náměstí",
    "via", "viale", "piazza", "corso", "strada", "str.", "bulevardul", "bd-ul", "rue", "avenue",
    "av.", "ave", "bd", "bd.", "boulevard", "place", "route", "chemin", "allée", "impasse",
    "calle", "c/", "cl.", "plaza", "avda", "avda.", "paseo", "carrer", "road", "rd", "street",
    "st.", "lane", "square", "postfach", "pf", "p.o.", "b.p.", "box", "λεωφ.", "λεωφόρος", "οδός",
    "utca", "út", "tér", "vej", "gade", "gatan", "vägen", "katu", "tie", "straat", "laan", "weg",
    "plein", "gracht",
];

/// Street suffixes on a compound token: `Domplatz`, `Stiftsstraße`,
/// `Ernst-Kamieth-Straße`, `Dr.-Ernst-Zimmermann-Str.`.
const STREET_SUFFIXES: &[&str] = &[
    "straße", "strasse", "str.", "-str", "weg", "gasse", "platz", "allee", "ring", "damm", "ufer",
    "chaussee", "markt", "steig", "pfad", "promenade", "vej", "gade", "gatan", "vägen", "veien",
    "katu", "straat", "laan", "plein", "kade", "dreef", "singel",
];

/// A street line carries a digit and either ends in a house number (`9`,
/// `181a`, `2/4`, `35-37`) or names a street / box word somewhere on it
/// (`ul. Jana Pawła II 35`, `Postfach 10 01 61`, `12 rue de la Paix`).
fn street_line(line: &str) -> bool {
    if !line.bytes().any(|b| b.is_ascii_digit()) {
        return false;
    }
    let lower = line.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
        .collect();
    let Some(&last) = tokens.last() else {
        return false;
    };
    let house_number = |t: &str| {
        let b = t.as_bytes();
        b.first().is_some_and(u8::is_ascii_digit)
            && b.len() <= 8
            && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'/' || *c == b'-')
            && b.iter().filter(|c| c.is_ascii_alphabetic()).count() <= 1
    };
    if house_number(last) {
        return true;
    }
    tokens.iter().any(|t| {
        STREET_WORDS.contains(t) || STREET_SUFFIXES.iter().any(|s| t.ends_with(s) && t.len() > s.len())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(s: &str) -> Option<String> {
        strip_trailing_address(s)
    }

    #[test]
    fn the_live_specimens_lose_their_postal_block_and_keep_their_name() {
        // Read off the name-pollution census listing (job 569), 2026-09-06.
        assert_eq!(
            strip("Landkreis Saalekreis\nDomplatz 9\n06217 Merseburg").as_deref(),
            Some("Landkreis Saalekreis")
        );
        assert_eq!(
            strip("Vergabekammer Rheinland-Pfalz\nStiftsstraße 9\n55116 Mainz").as_deref(),
            Some("Vergabekammer Rheinland-Pfalz"),
            "the issue's own specimen, 12,249 mentions"
        );
        assert_eq!(
            strip("1. und 2. Vergabekammer beim Landesverwaltungsamt Sachsen-Anhalt\nErnst-Kamieth-Straße 2\n06112 Halle/Saale")
                .as_deref(),
            Some("1. und 2. Vergabekammer beim Landesverwaltungsamt Sachsen-Anhalt")
        );
        assert_eq!(
            strip("Tomaszowskie Centrum Zdrowia Sp.zo.o.\nul. Jana Pawła II 35\n97-200 Tomaszów Mazowiecki")
                .as_deref(),
            Some("Tomaszowskie Centrum Zdrowia Sp.zo.o."),
            "the Polish dashed code and a street word with the number last"
        );
        // Two street-shaped lines walk back; a bare locality line before them
        // is NOT an address line and stays with the name — a superset key
        // still, and the census reports it as `still-differs`.
        assert_eq!(
            strip("MTU Maintenance\nBerlin-Brandenburg GmbH\nLudwigsfelde\nDr.-Ernst-Zimmermann-Str. 2\nD-14974 LUDWIGSFELDE")
                .as_deref(),
            Some("MTU Maintenance Berlin-Brandenburg GmbH Ludwigsfelde")
        );
        // A postcode line alone, five digits: corroboration enough.
        assert_eq!(strip("Stadt Mainz\n55116 Mainz").as_deref(), Some("Stadt Mainz"));
        // Windows line ends and a trailing district number.
        assert_eq!(
            strip("Úřad práce\r\nDobrovského 1278/25\r\n170 00 Praha 7").as_deref(),
            Some("Úřad práce")
        );
    }

    #[test]
    fn other_european_postcode_shapes_are_recognised() {
        assert_eq!(strip("Gemeente Amsterdam\nAmstel 1\n1011 PN Amsterdam").as_deref(), Some("Gemeente Amsterdam"));
        // A street line without a number is not recognised as one and stays
        // with the name — the same rule as `Rathaus` below; the census reports
        // such a row as `still-differs` and the listing shows why.
        assert_eq!(strip("Câmara Municipal\nPraça do Município\n1100-365 Lisboa").as_deref(), Some("Câmara Municipal Praça do Município"));
        assert_eq!(strip("Câmara Municipal\nPraça do Município, 1\n1100-365 Lisboa").as_deref(), Some("Câmara Municipal"));
        assert_eq!(strip("Stadt Wien\nRathaus\nA-1010 Wien").as_deref(), Some("Stadt Wien Rathaus"), "a prefix corroborates a four-digit code; `Rathaus` carries no digit so it stays");
        assert_eq!(strip("Stadt Wien\nRathausplatz 1\n1010 Wien").as_deref(), Some("Stadt Wien"), "a street line corroborates it too");
        assert_eq!(strip("Mairie de Paris\n4 place de l'Hôtel de Ville\n75004 Paris").as_deref(), Some("Mairie de Paris"));
        assert_eq!(strip("Comune di Roma\nVia del Campidoglio 1\n00186 Roma").as_deref(), Some("Comune di Roma"));
        assert_eq!(strip("Ayuntamiento\nPlaza Mayor, 27\n28012, Madrid").as_deref(), Some("Ayuntamiento"), "a comma after the code is tolerated");
    }

    #[test]
    fn a_bare_four_digit_code_is_a_year_until_corroborated() {
        assert_eq!(strip("Expo\n2000 Hannover"), None);
        assert_eq!(strip("Landesgartenschau\n2024 Wangen im Allgäu"), None);
        // With a street line before it, the same shape is an address.
        assert_eq!(strip("Expo\nMessegelände 1\n2000 Hannover").as_deref(), Some("Expo"));
        // The documented asymmetry: a bare FIVE-digit line needs no
        // corroboration, so a labelled number is stripped as if it were one.
        // Pinned so the trade-off is visible, not so it is desirable.
        assert_eq!(strip("Kommission\n54321 Sonderfall").as_deref(), Some("Kommission"));
    }

    #[test]
    fn a_two_letter_prefix_corroborates_and_the_walk_back_stops_at_two_lines() {
        assert_eq!(strip("Urząd Miasta\nPL-02-222 Warszawa").as_deref(), Some("Urząd Miasta"));
        assert_eq!(strip("Stadt Wien\nCH-8000 Zürich").as_deref(), Some("Stadt Wien"));
        // Three street-shaped lines: only the two nearest the postcode go.
        assert_eq!(
            strip("Firma\nGebäude 3\nEingang 2\nHauptstraße 1\n12345 Ort").as_deref(),
            Some("Firma Gebäude 3")
        );
    }

    #[test]
    fn wrapped_names_department_lines_and_consortium_lists_are_left_alone() {
        // The bulk of the class: a line break inside a name, whitespace to
        // `match_norm` already.
        for name in [
            "Landeshauptstadt\nDresden, GB Finanzen und Liegenschaften, Zentrales Vergabebüro",
            "Zentrum für Sonnenergie- und Wasserstoff-Forschung Baden-\nWürttemberg",
            "IURIDICO\nLegal & Financial Translations Sp. z o.o.",
            "Bundesverwaltungsamt (BVA)\nObere Bundesbehörde",
            "Evangelischer Kirchenkreisverband Berlin Mitte-Nord\nc/o STATTBAU Stadtentwicklungsgesellschaft mbH",
            "Amt der Tiroler Landesregierung\r\nInnsbruck",
            "Konsorcjum firm:\n1) Przedsiębiorstwo ALBA Sp. z o. o. – Lider konsorcjum\n2) MPGKiM Sp. z o.o. - Konsorcjant",
            "Konsorcjum\nLider: WIMED Sp z o.o., Sp. K., \nPartner: BRUK-MAR F.H.U. MARCIN GĄSIOR,",
            // Register lines after an inline address: the last line is words.
            "RADIOMETER SP. Z O.O.,  AL. JEROZOLIMSKIE 181, 02-222 WARSZAWA\nNIP: 526-272-36-18\nREGON: 015543565\nwojewództwo: MAZOWIECKIE",
            "Trenitalia S.p.A.\nDirezione Logistica Industriale\nAcquisti Tecnici",
            "Amt\nStraße",
        ] {
            assert_eq!(strip(name), None, "{name:?}");
        }
        // Single-line names are never in the class, address or not.
        assert_eq!(strip("Firma GmbH, Hauptstr. 1, 12345 Berlin"), None);
        // The name line always survives: a two-line name that is street plus
        // postcode has nothing to keep and is not stripped to nothing.
        assert_eq!(strip("Domplatz 9\n06217 Merseburg").as_deref(), Some("Domplatz 9"));
    }

    #[test]
    fn street_lines_are_recognised_by_number_or_by_word() {
        assert!(street_line("Domplatz 9"));
        assert!(street_line("Stiftsstraße 9"));
        assert!(street_line("Dr.-Ernst-Zimmermann-Str. 2"));
        assert!(street_line("ul. Jana Pawła II 35"));
        assert!(street_line("Postfach 10 01 61"));
        assert!(street_line("12 rue de la Paix"));
        assert!(street_line("Hauptstraße 35-37"));
        assert!(street_line("Dobrovského 1278/25"));
        assert!(!street_line("Ludwigsfelde"));
        assert!(!street_line("Berlin-Brandenburg GmbH"));
        assert!(!street_line("Direzione Logistica Industriale"));
        assert!(!street_line("1) Przedsiębiorstwo ALBA Sp. z o. o. – Lider konsorcjum"));
    }
}
