//! DÖE bulk-export addressing (docs/research/german-portals.md).
//!
//! One account-free OpenData endpoint serves whole day or month exports as
//! ZIPs of eForms-DE XML. `pubDay` and `pubMonth` are mutually exclusive;
//! times are Europe/Berlin.
//!
//! The API is strictly **T+1**: a day becomes fetchable the morning after it
//! closes, and today/future days are rejected 400. `pubMonth` does serve the
//! in-progress month and accumulates as days close, so the current month must
//! be re-fetched rather than skipped as known ([`FIRST_MONTH`] bounds the
//! other end — earlier months are rejected 400 too).

use crate::fetch::Target;

pub const BASE: &str = "https://oeffentlichevergabe.de";

/// Earliest month the API serves; anything before is rejected 400.
pub const FIRST_MONTH: (u16, u8) = (2022, 12);

/// Monthly export, e.g. (2026, 6). Serves the current month too, still growing.
pub fn monthly(base: &str, year: u16, month: u8) -> Target {
    Target {
        source: "doe",
        kind: "monthly",
        period: format!("{year}-{month:02}"),
        url: format!("{base}/api/notice-exports?pubMonth={year}-{month:02}&format=eforms.zip"),
        rel_path: format!("doe/monthly/{year}-{month:02}.zip"),
    }
}

/// Export for one completed day. Today and future days are rejected 400.
pub fn day(base: &str, date: (u16, u8, u8)) -> Target {
    let (year, month, day) = date;
    Target {
        source: "doe",
        kind: "daily",
        period: format!("{year}-{month:02}-{day:02}"),
        url: format!(
            "{base}/api/notice-exports?pubDay={year}-{month:02}-{day:02}&format=eforms.zip"
        ),
        rel_path: format!("doe/daily/{year}-{month:02}-{day:02}.zip"),
    }
}

/// Months from [`FIRST_MONTH`] through `end` inclusive — the backfill walk.
pub fn months_through(end: (u16, u8)) -> Vec<(u16, u8)> {
    let mut out = Vec::new();
    let (mut year, mut month) = FIRST_MONTH;
    while (year, month) <= end {
        out.push((year, month));
        (year, month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_match_documented_patterns() {
        let m = monthly(BASE, 2026, 6);
        assert_eq!(
            m.url,
            "https://oeffentlichevergabe.de/api/notice-exports?pubMonth=2026-06&format=eforms.zip"
        );
        assert_eq!(m.source, "doe");
        assert_eq!(m.kind, "monthly");
        assert_eq!(m.period, "2026-06");
        assert_eq!(m.rel_path, "doe/monthly/2026-06.zip");

        let d = day(BASE, (2026, 7, 18));
        assert_eq!(
            d.url,
            "https://oeffentlichevergabe.de/api/notice-exports?pubDay=2026-07-18&format=eforms.zip"
        );
        assert_eq!(d.source, "doe");
        assert_eq!(d.kind, "daily");
        assert_eq!(d.period, "2026-07-18");
        assert_eq!(d.rel_path, "doe/daily/2026-07-18.zip");
    }

    #[test]
    fn backfill_walk_spans_first_month_through_end() {
        let months = months_through((2023, 2));
        assert_eq!(months, vec![(2022, 12), (2023, 1), (2023, 2)]);

        // Ends inclusive, and covers whole years without gaps.
        let long = months_through((2026, 7));
        assert_eq!(long.first(), Some(&FIRST_MONTH));
        assert_eq!(long.last(), Some(&(2026, 7)));
        assert_eq!(long.len(), 1 + 12 + 12 + 12 + 7);

        // Nothing to walk before the service existed.
        assert!(months_through((2022, 11)).is_empty());
    }
}
