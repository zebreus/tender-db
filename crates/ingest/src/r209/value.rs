//! Legacy lexical forms → the notice-parsed layer's typed values.
//!
//! One deliberate era convention: R2.0.9 publishes **no zone offsets** on any
//! date or time, so every stored instant carries `offset_minutes = 0` and
//! `utc_seconds` holds the *published wall-clock* seconds. That is lossless —
//! it round-trips exactly what TED printed — and the interpretation is
//! era-scoped: for the `ted-export-*` profiles an offset of 0 means "no
//! offset published", never "UTC asserted". Everything else is exact or the
//! notice quarantines (ADR-0004), same as the eForms profile.

use store::NoticeValue as Value;

use crate::eforms::value as eforms;

/// Money text → integer cents. Legacy amounts come in two lexical families:
/// machine decimals (`13260.00`, R2.0.9 forms) and display-formatted values
/// with NBSP/space thousands separators and an optional decimal comma
/// (`1 681 100`, `2 162 630,19` — coded VALUES and defence forms). Separators
/// carry no value, so they are stripped; a single trailing `,dd` fraction is a
/// decimal comma. Anything else fails exactly as the eForms parser would.
pub fn cents(text: &str) -> Result<i64, String> {
    eforms::cents(&normalize(text))
}

/// The separator stripping and decimal-comma rewrite [`cents`] applies before
/// the eForms decimal parser.
fn normalize(text: &str) -> String {
    let stripped: String = text.chars().filter(|c| !matches!(c, ' ' | '\u{a0}' | '\u{202f}')).collect();
    match (stripped.matches(',').count(), stripped.contains('.')) {
        // One comma and no dot: a decimal comma iff the fraction is 1–2
        // digits (`630,19`); three digits would be an ambiguous thousands
        // group and must fail loudly rather than guess.
        (1, false) => {
            let (whole, fraction) = stripped.split_once(',').expect("counted one comma");
            if (1..=2).contains(&fraction.len()) {
                format!("{whole}.{fraction}")
            } else {
                stripped
            }
        }
        _ => stripped,
    }
}

/// The suffix of the parse-layer row that keeps a RESCALED amount's raw
/// `@FMTVAL` (issue 471 unit 3): the attribute was an exact even power of ten
/// (`10^2`, `10^4`, … — the measured TED defect) ABOVE a correct element text,
/// so the text was adopted. The row sits in the amount's section, at the
/// amount's field id plus this suffix and at the amount's own ordinal, so it
/// pairs with exactly one amount row. It is the record of the correction: the
/// corrected amount itself is an ordinary canonical figure (owner decision
/// 2026-10-06), and this row reaches no canonical fact. Both representations
/// therefore stay in the parsed layer (ADR-0004: nothing published is dropped).
pub const FMTVAL_MISMATCH_SUFFIX: &str = ".FMTVAL_MISMATCH";

/// The suffix of the parse-layer row that keeps an amount's element TEXT when
/// it disagrees with its `@FMTVAL` in any shape OTHER than the measured defect
/// (issue 471 unit 3 review). The attribute is read exactly as before and the
/// text is NOT adopted: this class was never measured outside the sampled
/// 2011–2014 r208 months. The row only
/// keeps the published text (ADR-0004) and makes the class countable in the
/// parsed layer before anyone decides what it means. Same pairing as
/// [`FMTVAL_MISMATCH_SUFFIX`]: amount's section, field id + suffix, its ordinal.
pub const FMTVAL_TEXT_SUFFIX: &str = ".FMTVAL_TEXT";

/// What an amount element's two representations resolve to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AmountReading {
    /// One lexical form to parse the ordinary way ([`cents`], prose fallback):
    /// `@FMTVAL` when present, else the text. This is every amount before issue
    /// 471 and every amount whose representations agree, or whose text is not
    /// an unambiguous number (nothing to check against, so nothing is claimed).
    Lexical(String),
    /// The measured defect: `@FMTVAL` is the text times an exact even `10^k`,
    /// `k >= 2`. `cents` is the TEXT's figure; `attribute` the raw `@FMTVAL`.
    Rescaled { cents: i64, attribute: String },
    /// Both are numbers and they disagree in any other way. The attribute is
    /// read as before (`attribute`); the raw `text` is kept beside it, unmarked.
    Disagrees { attribute: String, text: String },
}

/// Issue 471 unit 3: check an amount's `@FMTVAL` against its element text.
///
/// TED's 2011 generator (the July-2011 packages, R2.0.7-schema notices of the
/// `ted-export-r208` profile) wrote attributes an exact even power of ten
/// ABOVE a correct text (`<VALUE_COST FMTVAL="49700000000000000">49 700`,
/// 222043-2011; ~5.9 % of that month's value elements, 10^2 … 10^14). So:
///
/// - text not an unambiguous number ([`display_cents`]), or attribute not a
///   number, or the two agree: today's reading, the attribute wins;
/// - attribute = text × `10^k`, `k >= 2` and EVEN: the TEXT is adopted
///   ([`AmountReading::Rescaled`]) — the measured shape and direction only;
/// - any other disagreement (text larger, odd `k`, not a power of ten): the
///   attribute is kept and NOT marked ([`AmountReading::Disagrees`]); the text
///   is kept beside it so the class can be counted before it is judged.
///
/// The attribute is compared in `i128` cents, so a scaled attribute too large
/// for the stored `i64` (10^12 / 10^14 scales: `FMTVAL="100000000000000000"`)
/// is still recognised and its text adopted.
pub fn read_amount(fmtval: Option<&str>, text: &str) -> Option<AmountReading> {
    let Some(attr) = fmtval else {
        return (!text.is_empty()).then(|| AmountReading::Lexical(text.to_owned()));
    };
    let lexical = || Some(AmountReading::Lexical(attr.to_owned()));
    let (Some(by_attr), Some(by_text)) = (wide_cents(attr), display_cents(text)) else {
        return lexical();
    };
    if by_attr == i128::from(by_text) {
        return lexical();
    }
    if scaled_by_even_power_of_ten(by_attr, i128::from(by_text)) {
        Some(AmountReading::Rescaled { cents: by_text, attribute: attr.to_owned() })
    } else {
        Some(AmountReading::Disagrees { attribute: attr.to_owned(), text: text.to_owned() })
    }
}

/// [`cents`] without the `i64` ceiling: the same normalisation and rounding,
/// so where `cents` succeeds the two agree, and where it overflows this still
/// answers (up to `i128`). `None` for anything `cents` would refuse as a form.
fn wide_cents(text: &str) -> Option<i128> {
    if let Ok(cents) = cents(text) {
        return Some(i128::from(cents));
    }
    let normalized = normalize(text);
    let (sign, digits) = match normalized.strip_prefix('-') {
        Some(rest) => (-1i128, rest),
        None => (1, normalized.strip_prefix('+').unwrap_or(&normalized)),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    let fraction = fraction.trim_end_matches('0');
    if !whole.bytes().all(|b| b.is_ascii_digit()) || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let round_up = fraction.len() > 2 && fraction.as_bytes()[2] >= b'5';
    let fraction = &fraction[..fraction.len().min(2)];
    let whole: i128 = if whole.is_empty() { 0 } else { whole.parse().ok()? };
    let fraction: i128 = format!("{fraction:0<2}").parse().ok()?;
    whole.checked_mul(100)?.checked_add(fraction)?.checked_add(i128::from(round_up)).map(|c| sign * c)
}

/// `attr == text × 10^k` exactly, with `k >= 2` and `k` EVEN — the measured
/// July-2011 shape (issue 471 unit 3 measurement: 10^2 … 10^14, every one
/// even, every one with the attribute the larger). Both non-zero, one sign.
fn scaled_by_even_power_of_ten(attr: i128, text: i128) -> bool {
    if attr == 0 || text == 0 || (attr < 0) != (text < 0) {
        return false;
    }
    let (attr, text) = (attr.unsigned_abs(), text.unsigned_abs());
    if attr <= text || attr % text != 0 {
        return false;
    }
    let mut ratio = attr / text;
    let mut k = 0;
    while ratio % 10 == 0 {
        ratio /= 10;
        k += 1;
    }
    ratio == 1 && k >= 2 && k % 2 == 0
}

/// A display-formatted amount text → cents, CONSERVATIVELY: `None` whenever the
/// spelling does not say unambiguously where the decimal point is. This is the
/// check side of [`read_amount`], so a wrong guess here would rewrite a correct
/// attribute; refusing costs only the check.
///
/// Accepted: digits with space / NBSP / narrow-NBSP thousands separators
/// (`49 700`), a decimal comma or dot with 1–2 fraction digits (`20 550,54`,
/// `13260.00`), dot or comma thousands groups when the separator repeats
/// (`1.234.567`) or the other mark is the decimal (`1.234,56`, `1,234.56`).
/// Refused: a single `.` or `,` followed by exactly three digits (`1.234` —
/// thousands or mills?), any group that is not three digits, any letter or
/// currency sign, an empty string.
pub fn display_cents(text: &str) -> Option<i64> {
    let text = text.trim();
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest.trim_start()),
        None => (false, text),
    };
    if !body.chars().all(|c| c.is_ascii_digit() || matches!(c, ' ' | '\u{a0}' | '\u{202f}' | ',' | '.')) {
        return None;
    }
    if !body.starts_with(|c: char| c.is_ascii_digit()) || !body.ends_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let (commas, dots) = (body.matches(',').count(), body.matches('.').count());
    // Which mark (if any) is the decimal point.
    let decimal = match (commas, dots) {
        (0, 0) => None,
        // Both marks: the LAST one is the decimal point, and it must be unique.
        (_, _) if commas > 0 && dots > 0 => {
            let last = if body.rfind(',') > body.rfind('.') { ',' } else { '.' };
            if body.matches(last).count() != 1 {
                return None;
            }
            Some(last)
        }
        // One mark, once: a decimal iff 1–2 digits follow; 3 is ambiguous.
        (1, 0) | (0, 1) => {
            let mark = if commas == 1 { ',' } else { '.' };
            let fraction = &body[body.find(mark)? + 1..];
            if (1..=2).contains(&fraction.len()) && fraction.bytes().all(|b| b.is_ascii_digit()) {
                Some(mark)
            } else {
                return None;
            }
        }
        // One mark, repeated: thousands groups only.
        _ => None,
    };
    let (whole, fraction) = match decimal {
        Some(mark) => {
            let at = body.rfind(mark)?;
            (&body[..at], &body[at + 1..])
        }
        None => (body, ""),
    };
    if !(fraction.is_empty() || ((1..=2).contains(&fraction.len()) && fraction.bytes().all(|b| b.is_ascii_digit())))
    {
        return None;
    }
    // The whole part: groups split on every remaining separator. A grouped
    // number is 1–3 digits then 3-digit groups; an ungrouped one any length.
    let groups: Vec<&str> = whole.split([' ', '\u{a0}', '\u{202f}', ',', '.']).collect();
    if groups.len() > 1
        && !(groups[0].len() <= 3
            && !groups[0].is_empty()
            && groups[1..].iter().all(|g| g.len() == 3))
    {
        return None;
    }
    if groups.iter().any(|g| g.is_empty() || !g.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let whole: i64 = groups.concat().parse().ok()?;
    let fraction: i64 = format!("{fraction:0<2}").parse().ok()?;
    let cents = whole.checked_mul(100)?.checked_add(fraction)?;
    Some(if negative { -cents } else { cents })
}

/// A date without a published time: ISO (`2019-02-01`) or the coded section's
/// compact form (`20190102`).
pub fn date(text: &str) -> Result<Value, String> {
    eforms::timestamp(&format!("{}Z", iso(text)?), None)
}

/// A date plus its paired wall-clock time (`2019-02-01` + `12:00`).
pub fn date_with_time(date: &str, time: &str) -> Result<Value, String> {
    eforms::timestamp(&format!("{}Z", iso(date)?), Some(&format!("{time}Z")))
}

/// The coded section's `DT_DATE_FOR_SUBMISSION`: `20190207 11:00`.
pub fn datetime(text: &str) -> Result<Value, String> {
    match text.split_once(' ') {
        Some((date, time)) => date_with_time(date, time.trim()),
        None => date(text),
    }
}

/// A wall-clock time with no date of its own: stored on the epoch day so the
/// clock survives round-tripping (the eForms profile's convention).
pub fn time_only(text: &str) -> Result<Value, String> {
    let Value::Date { utc_seconds, offset_minutes, .. } =
        eforms::timestamp("1970-01-01Z", Some(&format!("{text}Z")))?
    else {
        unreachable!("timestamp returns a date")
    };
    Ok(Value::Date { utc_seconds, offset_minutes, has_time: true })
}

/// Defence-style split date: DAY/MONTH/YEAR (+ optional TIME) child values.
pub fn date_from_parts(day: &str, month: &str, year: &str, time: Option<&str>) -> Result<Value, String> {
    let (d, m, y) = (int(day)?, int(month)?, int(year)?);
    let iso = format!("{y:04}-{m:02}-{d:02}Z");
    match time {
        Some(t) => eforms::timestamp(&iso, Some(&format!("{t}Z"))),
        None => eforms::timestamp(&iso, None),
    }
}

/// `2019-02-01` stays; `20190102` becomes `2019-01-02`; anything else fails.
fn iso(text: &str) -> Result<String, String> {
    let text = text.trim();
    if text.len() == 8 && text.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(format!("{}-{}-{}", &text[..4], &text[4..6], &text[6..8]));
    }
    if text.len() == 10 && text.as_bytes()[4] == b'-' && text.as_bytes()[7] == b'-' {
        return Ok(text.to_owned());
    }
    Err(format!("not a date: {text}"))
}

fn int(text: &str) -> Result<i64, String> {
    text.trim().parse().map_err(|_| format!("not a number: {text}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_amount_lexical_forms_normalize_exactly() {
        assert_eq!(cents("13260.00"), Ok(1_326_000));
        assert_eq!(cents("1\u{a0}681\u{a0}100"), Ok(168_110_000));
        assert_eq!(cents("2 162 630,19"), Ok(216_263_019));
        assert_eq!(cents("1590482.50"), Ok(159_048_250));
        // An ambiguous 3-digit comma group is not guessed at.
        assert!(cents("1,681").is_err());
        assert!(cents("about 5").is_err());
    }

    /// Issue 471 unit 3: the check side reads TED's spellings and refuses
    /// to guess where a decimal point is.
    #[test]
    fn a_display_amount_is_read_only_when_its_decimal_point_is_unambiguous() {
        assert_eq!(display_cents("49 700"), Some(4_970_000));
        assert_eq!(display_cents("49\u{a0}700"), Some(4_970_000));
        assert_eq!(display_cents("20 550,54"), Some(2_055_054));
        assert_eq!(display_cents("13260.00"), Some(1_326_000));
        assert_eq!(display_cents("1.234.567"), Some(123_456_700));
        assert_eq!(display_cents("1.234.567,8"), Some(123_456_780));
        assert_eq!(display_cents("1,234,567.89"), Some(123_456_789));
        assert_eq!(display_cents("3097158480"), Some(309_715_848_000));
        assert_eq!(display_cents("0"), Some(0));
        // Ambiguous or not a number: refused, never guessed.
        for text in ["1.234", "1,234", "1.2345", "12 34", "1.234.56", "1,234,567.8.9", "EUR 500", "500 EUR",
            "ca. 5 000", "", "-", "1 000,", ",50", "1,234.567,8"]
        {
            assert_eq!(display_cents(text), None, "{text:?}");
        }
    }

    #[test]
    fn an_fmtval_is_overruled_only_by_the_measured_scale_error() {
        // Agreement and an unparseable text read exactly as before.
        assert_eq!(read_amount(Some("49700"), "49 700"), Some(AmountReading::Lexical("49700".into())));
        assert_eq!(read_amount(Some("1234"), "1.234"), Some(AmountReading::Lexical("1234".into())));
        assert_eq!(read_amount(None, "49 700"), Some(AmountReading::Lexical("49 700".into())));
        assert_eq!(read_amount(Some("49700"), ""), Some(AmountReading::Lexical("49700".into())));
        assert_eq!(read_amount(Some("about 5"), "49 700"), Some(AmountReading::Lexical("about 5".into())));
        assert_eq!(read_amount(None, ""), None);
        // r209 f18's agreeing defence pair stays an ordinary reading.
        assert_eq!(
            read_amount(Some("2162630.19"), "2 162 630,19"),
            Some(AmountReading::Lexical("2162630.19".into()))
        );
        // 222043-2011: 10^12 above, and 10^2 (`FMTVAL="5000000">50 000`): the text wins.
        assert_eq!(
            read_amount(Some("49700000000000000"), "49 700"),
            Some(AmountReading::Rescaled { cents: 4_970_000, attribute: "49700000000000000".into() })
        );
        assert_eq!(
            read_amount(Some("5000000"), "50 000"),
            Some(AmountReading::Rescaled { cents: 5_000_000, attribute: "5000000".into() })
        );
        // Review finding 1: a 10^12 / 10^14 attribute overflows i64 cents and is
        // still recognised — 100 000 × 10^12 and 922.34 × 10^14.
        assert_eq!(
            read_amount(Some("100000000000000000"), "100 000"),
            Some(AmountReading::Rescaled { cents: 10_000_000, attribute: "100000000000000000".into() })
        );
        assert!(cents("92234000000000000.00").is_err(), "the case must overflow i64 cents");
        assert_eq!(
            read_amount(Some("92234000000000000.00"), "922,34"),
            Some(AmountReading::Rescaled { cents: 92_234, attribute: "92234000000000000.00".into() })
        );
        // An overflowing attribute that is NOT a scale of the text: read as before.
        assert_eq!(
            read_amount(Some("100000000000000001"), "100 000"),
            Some(AmountReading::Disagrees { attribute: "100000000000000001".into(), text: "100 000".into() })
        );
        // Review finding 2: a text LARGER than the attribute is the dropped-decimal
        // signature (010347), not the measured defect — the attribute stays, unmarked.
        assert_eq!(
            read_amount(Some("30971584.80"), "3097158480"),
            Some(AmountReading::Disagrees { attribute: "30971584.80".into(), text: "3097158480".into() })
        );
        assert_eq!(
            read_amount(Some("497"), "49 700"),
            Some(AmountReading::Disagrees { attribute: "497".into(), text: "49 700".into() })
        );
        // Review finding 8: odd powers were never measured — 10^1, 10^3, 10^5 stay.
        for attr in ["4970", "49700000", "4970000000"] {
            assert_eq!(
                read_amount(Some(attr), "49 700"),
                Some(AmountReading::Disagrees { attribute: attr.into(), text: "49 700".into() }),
                "{attr}"
            );
        }
        // Any other disagreement keeps the attribute, unmarked, text beside it.
        assert_eq!(
            read_amount(Some("49800"), "49 700"),
            Some(AmountReading::Disagrees { attribute: "49800".into(), text: "49 700".into() })
        );
        assert_eq!(
            read_amount(Some("100"), "0"),
            Some(AmountReading::Disagrees { attribute: "100".into(), text: "0".into() })
        );
        assert_eq!(
            read_amount(Some("-4970000"), "49 700"),
            Some(AmountReading::Disagrees { attribute: "-4970000".into(), text: "49 700".into() })
        );
    }

    #[test]
    fn both_date_shapes_share_one_instant_encoding() {
        let compact = date("20190102").unwrap();
        let iso = date("2019-01-02").unwrap();
        assert_eq!(compact, iso);
        assert_eq!(
            compact,
            Value::Date { utc_seconds: 1_546_387_200, offset_minutes: 0, has_time: false }
        );
        assert_eq!(
            datetime("20190207 11:00").unwrap(),
            Value::Date { utc_seconds: 1_549_537_200, offset_minutes: 0, has_time: true }
        );
        assert_eq!(
            date_from_parts("14", "12", "2018", None).unwrap(),
            date("2018-12-14").unwrap()
        );
        assert!(date("7.2.2019").is_err());
    }
}
