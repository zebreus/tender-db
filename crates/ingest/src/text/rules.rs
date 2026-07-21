//! The era checklist made executable: one recorded decision per text-era
//! header field code (the `sdk/text-inventory.json` universe).
//!
//! The completeness tests walk the vendored inventory and fail on any code
//! without a rule here, and on any rule naming a code the inventory does not
//! declare (ADR-0002, era-scoped). At parse time a tag outside this registry
//! quarantines the record (ADR-0004) — the coded vocabularies drifted within
//! the era (1993 spells `TD: 3` "Invitation to tender", 2008 "Contract
//! notice"; `PC`/`OL`/`TW` only exist from ~2000; `IA`/`MA` from ~2007), so
//! an unknown tag is exactly how the next vintage surprise must surface.

/// How one header field's line(s) are consumed. Continuation lines are
/// indented; whether they extend the value or repeat it is a per-field fact
/// (measured: `AU` wraps its single name, `PC` lists one CPV code per line).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rule {
    /// Exactly one line; an indented continuation under it is unclaimed
    /// content (no scalar field was ever observed wrapping).
    Scalar(Type),
    /// The head line and each continuation line is one value of its own.
    PerLine(Type),
    /// Head + continuation lines are one prose value, newline-joined; the
    /// language tag is `Some("EN")` for English renderings, `None` for names
    /// and original-language bodies (see the module doc on `OT`).
    Prose(Option<&'static str>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Type {
    /// Compact `YYYYMMDD`.
    Date,
    /// `YYYYMMDD`, optionally followed by a `HH MM` wall clock
    /// (`19930125  16 00`) — the only two shapes measured across the era.
    Deadline,
    /// A code token, optionally followed by ` - <label>`; the label is the
    /// redundant English display text and is consumed (the XML-era
    /// CODIF_DATA convention, ted-legacy-mapping.md §4).
    Code,
    /// CPV classification code.
    Cpv,
    /// NUTS region code.
    Nuts,
    /// Pre-CPV product classification code (1993 vintage), scheme `cc`.
    Product,
    /// An identifier of this notice's own publication (`ND: 154-2005`,
    /// `OJ: 1/2005`).
    Id,
    /// A back-reference to a previous OJ publication (`RN: 108785-2003`) —
    /// the text-era Tender chain edge.
    Ref,
    Integer,
    /// One short text value per line (classification labels, region names).
    Line(Option<&'static str>),
}

use Rule::{PerLine, Prose, Scalar};

/// Every decided field code. Kept sorted; the completeness tests hold this
/// bijective with the vendored inventory.
const FIELDS: &[(&str, Rule)] = &[
    ("AA", Scalar(Type::Code)),
    ("AB", Prose(Some("EN"))),
    ("AC", Scalar(Type::Code)),
    ("AU", Prose(None)),
    ("CC", PerLine(Type::Product)),
    ("CO", Prose(None)),
    ("CT", PerLine(Type::Line(Some("EN")))),
    ("CY", Scalar(Type::Code)),
    ("DD", Scalar(Type::Deadline)),
    ("DR", Scalar(Type::Date)),
    ("DS", Scalar(Type::Date)),
    ("DT", Scalar(Type::Deadline)),
    ("HD", Scalar(Type::Code)),
    ("IA", Prose(None)),
    ("MA", PerLine(Type::Code)),
    ("NC", Scalar(Type::Code)),
    ("ND", Scalar(Type::Id)),
    // The main object classification and its English description (issue 35): the
    // 1995-98 vintages publish the primary CPV as `OC` — one 8-digit code per
    // line, like `PC` — paired with `ON`, the English label per code, exactly as
    // `CT` labels the pre-CPV `CC`. `OC` (present 1995-98) and `ON` (1995, dropped
    // by 1998) were in no rule, so the whole record quarantined: ~577k real EN
    // notices, ~all the single code `OC`.
    ("OC", PerLine(Type::Cpv)),
    ("OJ", Scalar(Type::Id)),
    ("OL", Scalar(Type::Code)),
    ("ON", PerLine(Type::Line(Some("EN")))),
    ("OT", Prose(None)),
    ("PC", PerLine(Type::Cpv)),
    ("PD", Scalar(Type::Date)),
    ("PG", Scalar(Type::Integer)),
    ("PN", PerLine(Type::Line(Some("EN")))),
    ("PR", Scalar(Type::Code)),
    ("RC", PerLine(Type::Nuts)),
    ("RG", PerLine(Type::Line(None))),
    ("RN", PerLine(Type::Ref)),
    // The authority/regulation code, one per line: code `2` (international
    // financing) is published as the lead institution plus a continuation line
    // per co-financier (`European Bank for Reconstruction and Development`, …),
    // measured in 1993 ISO_ORG bundles. Single-code records (`4 - EEC`) are the
    // one-line case of the same rule (issue 31).
    ("RP", PerLine(Type::Code)),
    ("TD", Scalar(Type::Code)),
    ("TI", Prose(Some("EN"))),
    ("TW", Prose(None)),
    ("TX", Prose(Some("EN"))),
    ("TY", Scalar(Type::Code)),
];

/// The rule for one field code, if it is a decided one.
pub fn rule(code: &str) -> Option<Rule> {
    FIELDS.binary_search_by_key(&code, |(c, _)| c).ok().map(|i| FIELDS[i].1)
}

/// Every code with a rule — the reverse leg of the completeness test.
pub fn decided_codes() -> impl Iterator<Item = &'static str> {
    FIELDS.iter().map(|(code, _)| *code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_is_sorted_for_binary_search() {
        assert!(FIELDS.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(rule("TX"), Some(Prose(Some("EN"))));
        assert_eq!(rule("ZZ"), None);
    }
}
