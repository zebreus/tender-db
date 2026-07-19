//! TED bulk-package addressing (docs/research/ted-access-channels.md).
//!
//! Daily packages are keyed by OJ S issue number (sequential per year,
//! Mon–Fri); monthly packages bundle a month's dailies. URLs carry no
//! checksums or cache headers — idempotency is ours (fetch.rs).

use crate::fetch::Target;

pub const BASE: &str = "https://ted.europa.eu";

/// Daily package for one OJ S issue, e.g. (2026, 137).
pub fn daily(base: &str, year: u16, issue: u32) -> Target {
    Target {
        source: "ted",
        kind: "daily",
        period: format!("{year}-{issue:05}"),
        url: format!("{base}/packages/daily/{year}{issue:05}"),
        rel_path: format!("ted/daily/{year}-{issue:05}.tar.gz"),
    }
}

/// Monthly package, e.g. (2026, 6). Note: the URL uses an unpadded month.
pub fn monthly(base: &str, year: u16, month: u8) -> Target {
    Target {
        source: "ted",
        kind: "monthly",
        period: format!("{year}-{month:02}"),
        url: format!("{base}/packages/monthly/{year}-{month}"),
        rel_path: format!("ted/monthly/{year}-{month:02}.tar"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_match_documented_patterns() {
        let d = daily(BASE, 2026, 137);
        assert_eq!(d.url, "https://ted.europa.eu/packages/daily/202600137");
        assert_eq!(d.period, "2026-00137");
        assert_eq!(d.rel_path, "ted/daily/2026-00137.tar.gz");

        let m = monthly(BASE, 2026, 6);
        assert_eq!(m.url, "https://ted.europa.eu/packages/monthly/2026-6");
        assert_eq!(m.rel_path, "ted/monthly/2026-06.tar");
    }
}
