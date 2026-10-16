#![allow(dead_code)]
mod project { pub use super::match_norm; }
pub fn consortium_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    let mut tokens = lower.split(|c: char| !c.is_alphanumeric());
    // "consórcio"/"consorcio" (PT/ES/IT): the precision review's live catch —
    // "Consórcio E.I.P. Serviços _ CME…" shares the lead member's NIF, the
    // Portuguese groupement. Deliberately NOT "konsorcjum": the same review
    // measured it appearing in MEMBER labels ("OPEGIEKA — członek
    // konsorcjum"), where the row IS the member and the merge is right.
    // "biege"/"bietergemeinschaft" (DE/AT Bietergemeinschaft — Lennart's
    // catch, 2026-08-29): measured 620 BIEGE-prefixed + 8,658 spelled-out
    // rows, 822 of the class identifier-bearing (the Colas shape in German).
    // Token matching keeps "Thoman Biegemaschinen GbR" (a bending-machine
    // builder — the live counter-case) clean: "biegemaschinen" is one token.
    // "arbeitsgemeinschaft" is "arge" spelled out (67 identifier-bearing).
    tokens.any(|t| {
        matches!(
            t,
            "groupement" | "gpt" | "consortium" | "mandataire" | "ute" | "arge" | "consórcio"
                | "consorcio" | "biege" | "bietergemeinschaft" | "arbeitsgemeinschaft"
        )
    })
}
fn family_token(t: &str) -> Option<&'static str> {
    match t {
        "gmbh" | "mbh" => Some("gmbh"),
        "ag" => Some("ag"),
        "sarl" | "eurl" => Some("sarl"),
        "sas" | "sasu" => Some("sas"),
        // Oy/Ab/Oyj/Abp are language and listing variants of ONE
        // Nordic legal form (aktiebolag/osakeyhtiö) — "X Oy" and
        // "X Ab" name the same company class, so they share one family
        // and never veto each other.
        "oy" | "oyj" | "ab" | "abp" | "uab" => Some("aktiebolag"),
        "aps" => Some("aps"),
        "bv" => Some("bv"),
        "nv" => Some("nv"),
        "kft" => Some("kft"),
        "zrt" | "nyrt" => Some("zrt"),
        "spa" => Some("spa"),
        "srl" => Some("srl"),
        "sia" => Some("sia"),
        // The undotted single-token spellings of the dotted forms ("Alfa
        // sro", "Beta spzoo") — the joined-stream probe caught these for
        // the veto; the shared table catches them for both consumers.
        "sro" => Some("sro"),
        "spzoo" => Some("spzoo"),
        _ => None,
    }
}
const FAMILY_SEQUENCES: [(&[&str], &str); 4] = [
    (&["spol", "s", "r", "o"], "sro"),
    (&["ges", "m", "b", "h"], "gmbh"),
    (&["sp", "z", "o", "o"], "spzoo"),
    (&["s", "r", "o"], "sro"),
];
pub fn n3_key(name: &str) -> String {
    let n2 = match_norm(name);
    let tokens: Vec<&str> = n2.split(' ').filter(|t| !t.is_empty()).collect();
    let mut out: Vec<String> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    'scan: while i < tokens.len() {
        for (seq, fam) in FAMILY_SEQUENCES {
            if tokens[i..].starts_with(seq) {
                out.push(format!("§{fam}"));
                i += seq.len();
                continue 'scan;
            }
        }
        match family_token(tokens[i]) {
            Some(fam) => out.push(format!("§{fam}")),
            None => out.push(tokens[i].to_owned()),
        }
        i += 1;
    }
    out.join(" ")
}
pub fn altid_name_key(name: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut initials = String::new();
    // `match_norm` deletes `&`; spelled out, it still ends an initialism below.
    let spelled = altid_trim(name).replace('&', " and ");
    for t in n3_key(&spelled).split(' ').filter(|t| !t.is_empty() && *t != "the") {
        if t == "and" {
            if !initials.is_empty() {
                out.push(std::mem::take(&mut initials));
            }
            continue;
        }
        let mut chars = t.chars();
        if let (Some(c), None) = (chars.next(), chars.next())
            && c.is_alphabetic()
        {
            initials.push(c);
            continue;
        }
        if !initials.is_empty() {
            out.push(std::mem::take(&mut initials));
        }
        out.push(
            match t {
                "ltd" | "limited" => "§ltd",
                "plc" => "§plc",
                "llp" => "§llp",
                "lp" => "§lp",
                "cic" => "§cic",
                "co" => "company",
                other => other,
            }
            .to_owned(),
        );
    }
    if !initials.is_empty() {
        out.push(initials);
    }
    out.join(" ")
}
pub fn altid_trim(name: &str) -> String {
    let find = |hay: &str, needle: &str| {
        hay.as_bytes().windows(needle.len()).position(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
    };
    let mut cut = name.len();
    for marker in [" t/a ", "(t/a ", " trading as ", "(trading as "] {
        if let Some(at) = find(name, marker) {
            cut = cut.min(at);
        }
    }
    let mut kept = name[..cut].trim_end();
    if kept.ends_with(')')
        && let Some(open) = kept.rfind('(')
    {
        let before = kept[..open].trim_end().trim_end_matches(['.', ',']);
        let lower = before.to_ascii_lowercase();
        if [" ltd", " limited", " plc", " llp"].iter().any(|form| lower.ends_with(form)) {
            kept = before;
        }
    }
    kept.to_owned()
}
pub fn head_vote_key(name: &str) -> String {
    let collapsed = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let alt = altid_name_key(&fold_diacritics(&collapsed));
    let kept: Vec<&str> = alt
        .split(' ')
        .filter(|t| !t.is_empty() && !t.starts_with('§') && family_token(t).is_none())
        .collect();
    if kept.is_empty() { alt } else { kept.join(" ") }
}
pub fn fold_diacritics(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars().flat_map(char::to_lowercase) {
        let cp = u32::from(c);
        if matches!(cp, 0x300..=0x36F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F) {
            continue;
        }
        match fold_latin_letter(c) {
            Some(base) => out.push_str(base),
            None => out.push(c),
        }
    }
    out
}
const LATIN_ADDITIONAL_LOW: &[u8; 75] = b"abbbcdddddeeeeefghhhhhiikkkllllmmmnnnnoooopprrrrsssssttttuuuuuvvwwwwwxxyzzz";
const LATIN_ADDITIONAL_HIGH: &[u8; 45] = b"aaaaaaaaaaaaeeeeeeeeiioooooooooooouuuuuuuyyyy";
fn fold_latin_letter(c: char) -> Option<&'static str> {
    let base = match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' | 'ǎ' | 'ǟ' | 'ǡ' | 'ǻ' | 'ȁ' | 'ȃ' | 'ȧ' => "a",
        'æ' | 'ǣ' | 'ǽ' => "ae",
        'ƀ' => "b",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' | 'ƈ' => "c",
        'ď' | 'đ' | 'ð' | 'ƌ' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' | 'ȅ' | 'ȇ' | 'ȩ' => "e",
        'ƒ' => "f",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' | 'ǥ' | 'ǧ' | 'ǵ' => "g",
        'ĥ' | 'ħ' | 'ȟ' => "h",
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' | 'ǐ' | 'ȉ' | 'ȋ' => "i",
        'ĳ' => "ij",
        'ĵ' | 'ǰ' => "j",
        'ķ' | 'ĸ' | 'ǩ' => "k",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        'ñ' | 'ń' | 'ņ' | 'ň' | 'ŉ' | 'ŋ' | 'ǹ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' | 'ơ' | 'ǒ' | 'ǫ' | 'ǭ' | 'ǿ' | 'ȍ' | 'ȏ' | 'ȫ' | 'ȭ'
        | 'ȯ' | 'ȱ' => "o",
        'œ' => "oe",
        'ŕ' | 'ŗ' | 'ř' | 'ȑ' | 'ȓ' => "r",
        'ś' | 'ŝ' | 'ş' | 'š' | 'ſ' | 'ș' => "s",
        'ß' => "ss",
        'ţ' | 'ť' | 'ŧ' | 'ț' => "t",
        'þ' => "th",
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' | 'ư' | 'ǔ' | 'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' | 'ȕ' | 'ȗ' => "u",
        'ŵ' => "w",
        'ý' | 'ÿ' | 'ŷ' | 'ƴ' | 'ȳ' => "y",
        'ź' | 'ż' | 'ž' | 'ƶ' => "z",
        _ => {
            let cp = u32::from(c);
            let byte = match cp {
                0x1E00..=0x1E95 => LATIN_ADDITIONAL_LOW[((cp - 0x1E00) / 2) as usize],
                0x1E96 => b'h',
                0x1E97 => b't',
                0x1E98 => b'w',
                0x1E99 => b'y',
                0x1E9A => b'a',
                0x1E9B => b's',
                0x1EA0..=0x1EF9 => LATIN_ADDITIONAL_HIGH[((cp - 0x1EA0) / 2) as usize],
                _ => return None,
            };
            return Some(ASCII_LETTERS[(byte - b'a') as usize]);
        }
    };
    Some(base)
}
const ASCII_LETTERS: [&str; 26] = [
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v",
    "w", "x", "y", "z",
];
fn mojibake(name: &str) -> bool {
    let continuation = |c: char| {
        matches!(
            u32::from(c),
            0x80..=0xBF
                | 0x152
                | 0x153
                | 0x160
                | 0x161
                | 0x178
                | 0x17D
                | 0x17E
                | 0x192
                | 0x2C6
                | 0x2DC
                | 0x2013..=0x203A
                | 0x20AC
                | 0x2122
        )
    };
    let chars: Vec<char> = name.chars().collect();
    let pair = chars.windows(2).any(|p| {
        (matches!(p[0], 'Ã' | 'Â' | 'Ä' | 'Å') && continuation(p[1])) || (p[0] == 'È' && matches!(p[1], '™' | '›'))
    });
    let word_final = chars.iter().enumerate().any(|(at, &c)| {
        matches!(c, 'Ã' | 'Â')
            && at > 0
            && chars[at - 1].is_lowercase()
            && chars.get(at + 1).is_none_or(|next| *next == ' ')
    });
    pair || word_final
}
pub fn head_twin_key(name: &str) -> String {
    fold_diacritics(name).chars().filter(|c| c.is_alphanumeric()).collect()
}
pub fn match_norm(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut gap = false;
    for c in name.chars() {
        if c.is_alphanumeric() {
            if gap && !out.is_empty() {
                out.push(' ');
            }
            gap = false;
            out.extend(c.to_lowercase().map(fold_greek_tonos));
        } else {
            gap = true;
        }
    }
    out
}
fn fold_greek_tonos(c: char) -> char {
    match c {
        'ά' => 'α',
        'έ' => 'ε',
        'ή' => 'η',
        'ί' | 'ϊ' | 'ΐ' => 'ι',
        'ό' => 'ο',
        'ύ' | 'ϋ' | 'ΰ' => 'υ',
        'ώ' => 'ω',
        'ς' => 'σ',
        other => other,
    }
}
fn unhex(s: &str) -> String { let b: Vec<u8> = (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i+2],16).unwrap()).collect(); String::from_utf8(b).unwrap() }
fn hex(s: &str) -> String { s.bytes().map(|b| format!("{:02x}", b)).collect() }
fn main() {
    use std::io::BufRead;
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let s = unhex(line.trim());
        let ctrl = s.chars().any(|c| c.is_control() || c == '\u{FFFD}');
        let collapsed = s.split_whitespace().collect::<Vec<_>>().join(" ");
        let cctrl = collapsed.chars().any(|c| c.is_control() || c == '\u{FFFD}');
        let cased = collapsed.chars().any(|c| c.is_lowercase() || c.is_uppercase());
        let lower = cased && !collapsed.chars().any(char::is_uppercase);
        println!("{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", hex(&head_vote_key(&s)), ctrl, cctrl, mojibake(&collapsed), collapsed.chars().count(), lower, consortium_name(&collapsed), hex(&collapsed));
    }
}
