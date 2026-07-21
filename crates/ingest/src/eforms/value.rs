//! Lexical forms → the notice-parsed layer's typed representations.
//!
//! Every conversion here is exact or it fails: a value tender-db cannot
//! represent without losing information quarantines the notice (ADR-0004)
//! rather than being rounded, truncated or coerced.

use store::NoticeValue as Value;

use super::index::FieldInfo;
use super::sdk::Decision;

#[derive(Debug, PartialEq)]
pub struct Error(pub String);

/// Convert one element's content to its stored value. `attr` looks up an XML
/// attribute of the same element — the SDK models `@currencyID`, `@languageID`,
/// `@listName`, `@schemeName` and `@unitCode` as fields in their own right, and
/// this is where they are consumed.
pub fn convert(
    field: &FieldInfo,
    text: &str,
    attr: impl Fn(&str) -> Option<String>,
) -> Result<Option<Value>, Error> {
    let text = text.trim();
    if text.is_empty() {
        // An empty element carries no value but is still claimed by its field.
        return Ok(None);
    }
    Ok(Some(match field.decision {
        Decision::Texts => Value::Text { lang: attr("languageID"), value: text.into() },
        // `@listName` is the SDK's spelling; `@listID` is genericode's own and
        // appears on real notices. Either names the code list.
        Decision::Codes => {
            Value::Code { list: attr("listName").or_else(|| attr("listID")), code: text.into() }
        }
        Decision::Classifications => {
            let scheme = if field.code_list.as_deref() == Some("cpv") { "cpv" } else { "nuts" };
            Value::Classification { scheme: scheme.into(), code: text.into() }
        }
        Decision::Amounts => {
            let currency = attr("currencyID")
                .ok_or_else(|| Error(format!("{}: amount without @currencyID", field.id)))?;
            Value::Amount { cents: cents(text).map_err(|e| Error(format!("{}: {e}", field.id)))?, currency }
        }
        Decision::Dates => match field.kind.as_str() {
            "date" => timestamp_for(field, text, None).map_err(|e| Error(format!("{}: {e}", field.id)))?,
            _ => timestamp_from_time(&offset_or_utc(field, text))
                .map_err(|e| Error(format!("{}: {e}", field.id)))?,
        },
        Decision::Integers => Value::Integer(match text {
            "true" => 1,
            "false" => 0,
            // Counts are published as decimals often enough (`0.0`); accept
            // them when the fraction is zero, which loses nothing.
            _ => text
                .split_once('.')
                .filter(|(_, fraction)| fraction.bytes().all(|b| b == b'0'))
                .map_or(text, |(whole, _)| whole)
                .parse()
                .map_err(|_| Error(format!("{}: not an integer: {text}", field.id)))?,
        }),
        Decision::Numbers => Value::Number {
            value: text.parse().map_err(|_| Error(format!("{}: not a number: {text}", field.id)))?,
            unit: attr("unitCode"),
        },
        Decision::Ids => Value::Id {
            // `@schemeID` is the ISO 6523 register code some buyers publish
            // instead of the SDK's `@schemeName`; either names the scheme.
            scheme: attr("schemeName").or_else(|| attr("schemeID")),
            value: text.into(),
            is_ref: field.kind == "id-ref",
        },
        // Never reached: attribute and virtual-view fields own no element.
        Decision::Attribute | Decision::VirtualView => return Ok(None),
    }))
}

/// Decimal string → integer cents, exactly. More than two fraction digits is a
/// value tender-db's money representation cannot hold, so it fails rather than
/// rounding silently (CONTEXT.md: money is INTEGER cents + currency).
pub fn cents(text: &str) -> Result<i64, String> {
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (-1i64, rest),
        None => (1, text.strip_prefix('+').unwrap_or(text)),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    // `561906.9100` is 561906.91 exactly; trailing zeros carry no value, so
    // they are not "more precision than cents can hold".
    let fraction = fraction.trim_end_matches('0');
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("not a decimal amount: {text}"));
    }
    if fraction.len() > 2 {
        return Err(format!("amount has more than two fraction digits: {text}"));
    }
    let whole: i64 = whole.parse().map_err(|_| format!("amount out of range: {text}"))?;
    let fraction: i64 = format!("{fraction:0<2}").parse().expect("two digits");
    whole
        .checked_mul(100)
        .and_then(|c| c.checked_add(fraction))
        .map(|c| sign * c)
        .ok_or_else(|| format!("amount out of range: {text}"))
}

/// [`timestamp`], with the one profile-scoped relaxation: the DÖE `sdk-0.1`
/// dialect (fields prefixed `SDK01-`) systematically publishes dates and
/// times *without* eForms' mandatory zone offset (`2022-11-29`,
/// `2000-01-01`). Quarantining would reject a large share of the profile's
/// whole history over a dialect trait, not a data error, so a missing offset
/// is read as UTC there — day precision is all such values carry. Every other
/// profile stays strict: an offsetless date is malformed and quarantines.
pub fn timestamp_for(field: &FieldInfo, date: &str, time: Option<&str>) -> Result<Value, String> {
    let date = offset_or_utc(field, date);
    let time = time.map(|t| offset_or_utc(field, t));
    timestamp(&date, time.as_deref())
}

/// Append `Z` for `SDK01-` fields whose lexical value carries no offset.
fn offset_or_utc<'a>(field: &FieldInfo, text: &'a str) -> std::borrow::Cow<'a, str> {
    if field.id.starts_with("SDK01-") && split_offset(text).is_err() {
        return format!("{text}Z").into();
    }
    text.into()
}

/// eForms dates always carry a zone offset (`2019-11-26+01:00`, or `Z`).
/// A time from the paired `time` field may be merged in.
pub fn timestamp(date: &str, time: Option<&str>) -> Result<Value, String> {
    let (date, mut offset) = split_offset(date)?;
    let [y, m, d] = split_ints(date, '-', "date")?[..] else {
        return Err(format!("not a date: {date}"));
    };
    let mut seconds = 0;
    let mut has_time = false;
    if let Some(time) = time {
        let (clock, time_offset) = split_offset(time)?;
        // eSenders publish sub-second precision (`18:00:00.0000000`,
        // `06:37:50.331`). The stored instant is whole seconds — CONTEXT.md's
        // representation decision — so the fraction is truncated, deliberately
        // and only here: a publication clock's sub-second digits carry no
        // procurement meaning, unlike an amount's minor units.
        let clock = clock.split_once('.').map_or(clock, |(whole, _)| whole);
        let parts = split_ints(clock, ':', "time")?;
        let [h, min, ..] = parts[..] else { return Err(format!("not a time: {clock}")) };
        seconds = h * 3600 + min * 60 + parts.get(2).copied().unwrap_or(0);
        // The pair is one instant; the time's offset is the authoritative one
        // (it is the one a submission deadline is expressed in).
        offset = time_offset;
        has_time = true;
    }
    let utc = crate::fetch::days_from_civil(y as u16, m as u8, d as u8) * 86_400 + seconds - offset * 60;
    Ok(Value::Date { utc_seconds: utc, offset_minutes: offset, has_time })
}

/// A `time` field with no published date of its own: stored as that clock time
/// on the epoch day, so the offset and wall-clock survive round-tripping.
fn timestamp_from_time(time: &str) -> Result<Value, String> {
    let Value::Date { utc_seconds, offset_minutes, .. } = timestamp("1970-01-01Z", Some(time))? else {
        unreachable!("timestamp returns a date")
    };
    Ok(Value::Date { utc_seconds, offset_minutes, has_time: true })
}

/// Split a trailing `Z` or `±HH:MM` offset; returns offset in minutes.
fn split_offset(text: &str) -> Result<(&str, i64), String> {
    if let Some(rest) = text.strip_suffix('Z').or_else(|| text.strip_suffix('z')) {
        return Ok((rest, 0));
    }
    // The offset sign is the last +/- that is not the date's leading sign; a
    // date is `YYYY-MM-DD`, so search from position 1 onwards for `+`, and for
    // `-` only in the last six characters.
    let sign_at = text.rfind('+').or_else(|| {
        let from = text.len().saturating_sub(6);
        text[from..].rfind('-').map(|i| i + from)
    });
    let Some(i) = sign_at.filter(|&i| i > 0 && text.len() - i == 6) else {
        return Err(format!("no zone offset: {text}"));
    };
    let (value, offset) = text.split_at(i);
    let sign = if offset.starts_with('-') { -1 } else { 1 };
    let [h, m] = split_ints(&offset[1..], ':', "offset")?[..] else {
        return Err(format!("not an offset: {offset}"));
    };
    Ok((value, sign * (h * 60 + m)))
}

fn split_ints(text: &str, sep: char, what: &str) -> Result<Vec<i64>, String> {
    text.split(sep)
        .map(|p| p.parse::<i64>().map_err(|_| format!("not a {what}: {text}")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_are_exact_cents_or_nothing() {
        assert_eq!(cents("1234"), Ok(123_400));
        assert_eq!(cents("1234.5"), Ok(123_450));
        assert_eq!(cents("1234.56"), Ok(123_456));
        assert_eq!(cents("0.07"), Ok(7));
        assert_eq!(cents("-12.50"), Ok(-1250));
        // Rounding would silently lose data — quarantine instead (ADR-0004).
        assert!(cents("1234.567").is_err());
        assert!(cents("1e6").is_err());
        assert!(cents("").is_err());
    }

    #[test]
    fn dates_keep_the_published_offset() {
        // Date only: UTC midnight of the local day.
        let Value::Date { utc_seconds, offset_minutes, has_time } = timestamp("2019-11-26+01:00", None).unwrap()
        else {
            panic!()
        };
        assert_eq!((offset_minutes, has_time), (60, false));
        assert_eq!(utc_seconds, 1_574_722_800); // 2019-11-25T23:00:00Z

        // Date + paired time: the deadline instant.
        let Value::Date { utc_seconds, offset_minutes, has_time } =
            timestamp("2026-03-02+01:00", Some("10:00:00+01:00")).unwrap()
        else {
            panic!()
        };
        assert_eq!((offset_minutes, has_time), (60, true));
        assert_eq!(utc_seconds, 1_772_442_000); // 2026-03-02T09:00:00Z

        assert!(matches!(timestamp("2024-02-29Z", None), Ok(Value::Date { offset_minutes: 0, .. })));
        assert!(matches!(timestamp("2019-11-26-03:00", None), Ok(Value::Date { offset_minutes: -180, .. })));
        // eForms requires the offset; a bare date is malformed.
        assert!(timestamp("2019-11-26", None).is_err());
    }
}
