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
    let stripped: String = text.chars().filter(|c| !matches!(c, ' ' | '\u{a0}' | '\u{202f}')).collect();
    let normalized = match (stripped.matches(',').count(), stripped.contains('.')) {
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
    };
    eforms::cents(&normalized)
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
