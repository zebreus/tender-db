//! Issue 538 unit 3: the publisher's extra escapes, decoded in served titles and descriptions.
//!
//! Some publishers escape a text twice, or three times, before it reaches the XML. TED eForms
//! carries the class (0.9–2.1 % of Tenders per measured window; legacy r2.0.8/r2.0.9 about
//! 0.2–0.35 %, mostly the text era's title-cased `&Amp;`). The parse layer decodes the XML once and
//! keeps what that leaves (ADR-0001), so its text still holds `&#224;`, `&amp;quot;` or
//! `&amp;lt;br/&amp;gt;`, and the projection copied that into served titles and descriptions. The
//! projection now decodes those two canonical fields, lot ones included, and nothing else: party
//! names are deferred (the issue's decision 2, since a decoded name changes `name_norm`, the bind
//! key of identifier-less organizations), and the other text fields stay as parsed.
//!
//! The rule, [`decode_publisher_escapes`] (owner decision of 2026-10-16):
//!
//! 1. One pass decodes every character reference of the text, left to right, without re-reading
//!    what it produced. A reference is `&#` decimal digits `;`, `&#x` (or `&#X`) hex digits `;`, or
//!    one of the five XML names `&amp;` `&lt;` `&gt;` `&quot;` `&apos;`, matched ASCII
//!    case-insensitively so the text era's `&Amp;` decodes. Nothing else is one: no other HTML name
//!    (`&nbsp;` stays), nothing without its `;`, and a bare `&` or `AT&T` stays. A numeric reference
//!    decodes only to a character XML 1.0 allows that is not a control other than tab, line feed and
//!    carriage return, so `&#0;`, `&#27;`, the C1 range `&#128;`–`&#159;` (Windows-1252 code units,
//!    not code points), a surrogate (`&#xD800;`), `&#xFFFE;`, `&#xFFFF;` and anything above U+10FFFF
//!    stay as written. So do `&#xZZ;` and a number too long for 32 bits.
//! 2. Passes repeat while the last one decoded something, at most [`MAX_PASSES`]. 8561112's
//!    description held `&amp;amp;` after the parse layer's own decode, so it needs two; a fourth
//!    level stays escaped once.
//! 3. A `<br>`, `<br/>` or `<br />` (any case) whose `<` a decode produced becomes a line feed, and a
//!    private-use code point U+E000–U+F8FF a numeric reference produced becomes `•` (Symbol-font
//!    bullets: 8561312's `&#61655;` is U+F0D7). Both reach only what a decode produced. Markup or a
//!    private-use character the parsed text carries literally is not touched.
//!
//! So the output differs from the input only where a reference was decoded. A text with no
//! decodable reference comes back borrowed and unchanged, so a deploy of this rule can move only a
//! Tender whose title or description holds an `&`. Mangled forms no decode can repair
//! (`modalità;`, 8561858's BT-24-Procedure) stay as they are.
//!
//! No `PROJECTION_EPOCH` bump (ADR-0017 route R3): a field-id finder enumerates the carriers
//! (`refold-fields` over the title and description field ids, docs/operations.md), and the 495
//! compare engine rewrites and announces only the Tenders whose text moved.

use std::borrow::Cow;

/// The most decode passes one text gets. Three clears every nesting unit 1 measured (`&amp;amp;`
/// after the parse layer's decode is the deepest seen) with one level to spare.
pub const MAX_PASSES: usize = 3;

/// The longest reference body read after a `&`, up to its `;`. `#x10FFFF` is 8; the slack admits
/// leading zeros. Bounding it keeps a text full of bare `&`s linear.
const MAX_BODY: usize = 16;

/// `text` with the publisher's extra escapes decoded, by the rule in the module doc. Borrowed and
/// unchanged when it holds no decodable reference.
pub fn decode_publisher_escapes(text: &str) -> Cow<'_, str> {
    if !text.contains('&') {
        return Cow::Borrowed(text);
    }
    // Each character, and whether a decode produced it: rule 3 reaches only those.
    let mut chars: Vec<(char, bool)> = text.chars().map(|c| (c, false)).collect();
    let mut decoded = false;
    for _ in 0..MAX_PASSES {
        match pass(&chars) {
            Some(next) => {
                chars = next;
                decoded = true;
            }
            None => break,
        }
    }
    if !decoded {
        return Cow::Borrowed(text);
    }
    Cow::Owned(finish(&chars))
}

/// One left-to-right pass, or `None` when it found no reference to decode.
fn pass(chars: &[(char, bool)]) -> Option<Vec<(char, bool)>> {
    let mut out = Vec::with_capacity(chars.len());
    let mut changed = false;
    let mut i = 0;
    while i < chars.len() {
        if chars[i].0 == '&'
            && let Some((c, len)) = reference(&chars[i + 1..])
        {
            out.push((c, true));
            i += 1 + len;
            changed = true;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    changed.then_some(out)
}

/// The character the reference after a `&` names, and how many characters it spans, its `;`
/// included.
fn reference(rest: &[(char, bool)]) -> Option<(char, usize)> {
    let semi = rest.iter().take(MAX_BODY + 1).position(|(c, _)| *c == ';')?;
    let body: String = rest[..semi].iter().map(|(c, _)| *c).collect();
    let c = match body.strip_prefix('#') {
        Some(number) => numeric(number)?,
        None => named(&body)?,
    };
    Some((c, semi + 1))
}

/// A numeric reference's character: decimal, or hex after `x`/`X`. Only a character XML 1.0
/// allows that is no control besides tab, line feed and carriage return.
fn numeric(body: &str) -> Option<char> {
    let (digits, radix) = match body.strip_prefix(['x', 'X']) {
        Some(hex) => (hex, 16),
        None => (body, 10),
    };
    // `from_str_radix` alone would take a leading `+`.
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    // Overflow, a surrogate and anything above U+10FFFF all end here.
    let c = char::from_u32(u32::from_str_radix(digits, radix).ok()?)?;
    let allowed = matches!(c, '\t' | '\n' | '\r') || !(c.is_control() || matches!(c, '\u{FFFE}' | '\u{FFFF}'));
    allowed.then_some(c)
}

/// The five XML names, ASCII case-insensitive.
fn named(body: &str) -> Option<char> {
    Some(match body.to_ascii_lowercase().as_str() {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        _ => return None,
    })
}

/// Rule 3 over the decoded characters, into the served string.
fn finish(chars: &[(char, bool)]) -> String {
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let (c, decoded) = chars[i];
        if decoded
            && c == '<'
            && let Some(len) = line_break(&chars[i + 1..])
        {
            out.push('\n');
            i += 1 + len;
            continue;
        }
        out.push(if decoded && ('\u{E000}'..='\u{F8FF}').contains(&c) { '•' } else { c });
        i += 1;
    }
    out
}

/// After a `<`: how many characters `br>`, `br/>` or `br />` spans there, any case.
fn line_break(rest: &[(char, bool)]) -> Option<usize> {
    let ahead: String = rest.iter().take("br />".len()).map(|(c, _)| c.to_ascii_lowercase()).collect();
    ["br>", "br/>", "br />"].into_iter().find(|tail| ahead.starts_with(tail)).map(str::len)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(text: &str) -> String {
        decode_publisher_escapes(text).into_owned()
    }

    fn unchanged(text: &str) {
        assert!(
            matches!(decode_publisher_escapes(text), Cow::Borrowed(t) if t == text),
            "{text:?} must come back borrowed and unchanged, got {:?}",
            decode(text)
        );
    }

    /// The issue's exhibits, as the parse layer holds them after its own decode.
    #[test]
    fn the_issues_exhibits_decode() {
        // 8561858 (00489010-2026), BT-21-Procedure: the XML carried `&amp;#224;`.
        assert_eq!(
            decode("Gara europea a procedura aperta telematica, in modalit&#224; multilotto, per l&#8217;affidamento"),
            "Gara europea a procedura aperta telematica, in modalità multilotto, per l’affidamento"
        );
        // 8561112 (00458719-2026): three times escaped in the XML, twice after the parse layer.
        assert_eq!(decode("souscription &amp;amp; la gestion"), "souscription & la gestion");
        // 8561120 (00484041-2026), lot descriptions: an escaped HTML line break.
        assert_eq!(decode("épicerie bio&amp;lt;br/&amp;gt;"), "épicerie bio\n");
        // 8561167 (00460412-2026).
        assert_eq!(decode("en phase &amp;quot;offre&amp;quot;"), "en phase \"offre\"");
        // 8561279 (00455590-2026).
        assert_eq!(decode("l&#8217;affidamento … attivit&#224;"), "l’affidamento … attività");
        // 8561312 (00485551-2026): a Symbol-font private-use bullet.
        assert_eq!(decode("&#61655; Lot 1"), "• Lot 1");
        assert_eq!(decode("&#xF0B7;&#xe000;&#xF8FF;&#xF900;"), "•••\u{F900}", "U+E000–U+F8FF only");
    }

    /// 8561858's BT-24-Procedure is mangled another way, and no decode can repair it.
    #[test]
    fn a_mangled_form_stays_as_it_is() {
        unchanged("in modalità; multilotto");
        // Beside a reference that does decode, it still stays.
        assert_eq!(decode("modalità; per l&#8217;affidamento"), "modalità; per l’affidamento");
    }

    /// The five names match ASCII case-insensitively: the text era's `&Amp;` (520's `O&Amp;K`).
    #[test]
    fn the_five_names_decode_in_any_case() {
        assert_eq!(decode("O&Amp;K"), "O&K");
        assert_eq!(decode("w.l. Gore &Amp; Associés"), "w.l. Gore & Associés");
        assert_eq!(decode("&AMP; &LT; &Gt; &QUOT; &aPoS;"), "& < > \" '");
        assert_eq!(decode("CENTRE COMMUNAL D&apos;ACTION SOCIALE"), "CENTRE COMMUNAL D'ACTION SOCIALE");
    }

    /// No other name, nothing without its `;`, and an `&` that starts no reference: unchanged.
    #[test]
    fn what_is_no_reference_stays() {
        unchanged("Fish & chips");
        unchanged("AT&T");
        unchanged("&");
        unchanged("R&D;");
        unchanged("&nbsp;&eacute;&hellip;&rsquo;");
        unchanged("&amp without its semicolon");
        unchanged("&#224 without its semicolon");
        unchanged("&#;&#x;&#-1;&#+65;&# 65;");
        unchanged("Ref &#xZZ; and &#12a;");
        unchanged("&ampamp;");
        // A literal `<br/>` and a literal private-use character are not a decode's output.
        unchanged("épicerie bio<br/>\u{F0D7}");
        // A body longer than any reference is not read as one.
        unchanged("&#0000000000000000224;");
    }

    /// A numeric reference to a control character, a surrogate, a noncharacter or past U+10FFFF
    /// stays as written; tab, line feed and carriage return decode.
    #[test]
    fn a_numeric_reference_to_no_character_stays() {
        unchanged("a&#0;b");
        unchanged("a&#1;b");
        unchanged("a&#27;b");
        unchanged("a&#x7F;b");
        unchanged("a&#150;b"); // C1: a Windows-1252 en dash, not a code point
        unchanged("a&#x9F;b");
        unchanged("a&#xD800;b");
        unchanged("a&#57343;b"); // U+DFFF
        unchanged("a&#xFFFE;b&#xFFFF;");
        unchanged("a&#x110000;b");
        unchanged("a&#4294967296;b"); // past u32
        unchanged("a&#xFFFFFFFFF;b");
        assert_eq!(decode("a&#9;b&#10;c&#13;&#x0A;d"), "a\tb\nc\r\nd");
        assert_eq!(decode("&#x1F600; &#X41;&#0065;"), "😀 AA");
        // The mixed line: what decodes, decodes; what does not, stays.
        assert_eq!(decode("&#0;&amp;amp;&#xD800;"), "&#0;&&#xD800;");
    }

    /// Three passes at most: a fourth level of escaping stays escaped once.
    #[test]
    fn nesting_stops_at_three_passes() {
        assert_eq!(decode("&amp;amp;amp;"), "&", "three levels: three passes");
        assert_eq!(decode("&amp;amp;amp;amp;"), "&amp;", "four levels: the fourth stays");
        assert_eq!(decode("&amp;amp;amp;lt;br/&amp;amp;amp;gt;"), "&lt;br/&gt;", "and is no line break");
        // A reference a pass produced is read by the next pass, not by the same one.
        assert_eq!(decode("&amp;#224;"), "à");
        assert_eq!(decode("&#38;amp;"), "&");
    }

    /// The three line-break spellings, any case, once their `<` was decoded; nothing else of markup.
    #[test]
    fn a_decoded_line_break_becomes_a_newline() {
        assert_eq!(decode("a&lt;br&gt;b&lt;BR/&gt;c&lt;Br /&gt;d"), "a\nb\nc\nd");
        assert_eq!(decode("a&amp;lt;br /&amp;gt;b"), "a\nb");
        // Other markup is decoded and left as markup.
        assert_eq!(decode("&lt;p&gt;x&lt;/p&gt;"), "<p>x</p>");
        assert_eq!(decode("&lt;br  /&gt;"), "<br  />", "not one of the three spellings");
        assert_eq!(decode("&lt;bra&gt;"), "<bra>");
        assert_eq!(decode("&lt;5 years"), "<5 years");
        // A literal line break beside a decoded one: only the decoded one goes.
        assert_eq!(decode("a<br/>b&lt;br/&gt;"), "a<br/>b\n");
    }
}
