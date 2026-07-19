//! Organization-identifier normalization and plausibility gating.
//!
//! CONTEXT.md merges organization mentions only on exact official identifiers
//! — but legacy TED NATIONALIDs carry no scheme attribute and 16% of filled
//! values are junk ("Romania", "n/a"; docs/research/ted-legacy-mapping.md §6).
//! The parsers store the raw published value; the canonical projection calls
//! [`normalize`] and merges only on `(country, normalized)` pairs that pass
//! the gate. eForms BT-501 values can go through the same gate.

/// A normalized, merge-grade organization identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrgId {
    pub scheme: Scheme,
    /// Uppercased, separator-free form — the merge key (scoped by country).
    pub normalized: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    /// Prefixed with a plausible ISO country code — VAT-number shaped
    /// (`NL804595859B01`, `GB287461957`).
    Vat,
    /// A bare national registry number (`65993390`); which register is
    /// country-specific and unknowable from the notice (no scheme attribute
    /// exists in the legacy era).
    National,
}

/// Normalize a published identifier, or reject it as merge-unusable.
///
/// The gate: strip separators (spaces, NBSP, dots, dashes, slashes),
/// uppercase, and require at least one digit and at least three significant
/// characters — "Romania", "n/a", "Ukjent" and empty strings all fail, and a
/// rejected value simply stays a name-only provisional profile (never an
/// error: quarantine is for unconsumed structure, not low-quality values).
pub fn normalize(raw: &str) -> Option<OrgId> {
    let normalized: String = raw
        .chars()
        .filter(|c| !matches!(c, ' ' | '\u{a0}' | '.' | '-' | '/' | '\\'))
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if normalized.len() < 3 || !normalized.bytes().any(|b| b.is_ascii_digit()) {
        return None;
    }
    let vat_shaped = normalized.len() >= 4
        && normalized.as_bytes()[..2].iter().all(u8::is_ascii_uppercase)
        && normalized.bytes().skip(2).any(|b| b.is_ascii_digit());
    Some(OrgId { scheme: if vat_shaped { Scheme::Vat } else { Scheme::National }, normalized })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measured_real_world_ids_normalize_or_gate_out() {
        // Real values from the 2019 package dissection (research §6).
        assert_eq!(
            normalize("65993390"),
            Some(OrgId { scheme: Scheme::National, normalized: "65993390".into() })
        );
        assert_eq!(
            normalize("NL804595859B01"),
            Some(OrgId { scheme: Scheme::Vat, normalized: "NL804595859B01".into() })
        );
        assert_eq!(
            normalize("GB287461957"),
            Some(OrgId { scheme: Scheme::Vat, normalized: "GB287461957".into() })
        );
        // Free-form with a stray space still yields one exact key.
        assert_eq!(normalize("FR463307 15368").unwrap().normalized, "FR46330715368");
        // Belgian CBE style with dots and an underscore-free suffix.
        assert_eq!(normalize("0242.069.537_22553").unwrap().normalized, "0242069537_22553");

        // The measured junk: no digits, no merge.
        assert_eq!(normalize("Romania"), None);
        assert_eq!(normalize("n/a"), None);
        assert_eq!(normalize("Ukjent"), None);
        assert_eq!(normalize(""), None);
    }
}
