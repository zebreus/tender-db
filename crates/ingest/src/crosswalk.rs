//! Issue 300 Stage 2: deterministic scheme cross-walks (E1) — the canonical
//! identifier key that unifies representations of ONE registration under ONE
//! country's rules (design §3.1). Two org rows whose canonical keys agree are
//! the same legal entity as far as the scheme's arithmetic can prove it:
//! `FI0100315​8` (VAT) and `01003158` (Y-tunnus) are one Finnish company,
//! `18001404501577` (SIRET) is an establishment of SIREN `180014045`.
//!
//! The E1/E2 split (the 2026-08-28 pad amendment, exemplar-driven):
//! transformations that DELETE redundant information — prefix strips,
//! establishment→legal-unit truncations — stay E1 (auto-merge material under
//! R2's remaining conditions). Transformations that MANUFACTURE information —
//! zero-padding a short digit string — are E2: candidate-edge material only,
//! never auto-merged, because the live CZ collision proved padding unsafe
//! (`0002542`, a corrupted id on Ministerstvo spravedlnosti, pads onto
//! `00002542`, the REAL checksum-valid IČO of Puncovní úřad).
//!
//! This module computes KEYS ONLY. The R2 merge rule's other conditions —
//! same normalized row country, both rows pass the v2 gate, the denial list
//! (VAT-group wall, UTE exclusion, group cap, gate-poison, legal-form veto),
//! group ≤ cap — live with the matcher, not here. The one exception is
//! denials that make the KEY ITSELF meaningless: CZ699 group VATs and ES UTE
//! NIFs identify ephemeral/group constructs, so they get no key at all.
//!
//! Countries with NO cross-walk (design §3.1): DE (court-scoped registers,
//! zero arithmetic yield), AT, IE, LU, CY, MT, EE (VAT and registrikood are
//! separate series), LT — `canonical_key` returns `None`; E0 exact-key
//! equality remains their only merge path.

use crate::idgate::{fr_vat_key, uuid_v4, Checksum};

/// How much the key's derivation is allowed to prove (design §3.1 amendment).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tier {
    /// Information-deleting derivation: eligible for R2 auto-merge.
    E1,
    /// Pad-derived (information-manufacturing): candidate edges only; merges
    /// require R3's full corroboration stack.
    E2,
}

/// A canonical identifier key: `scheme` names the national series the key
/// lives in (two keys unify ONLY within one scheme), `key` is the canonical
/// digit/character string, `tier` how the derivation may be used.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CanonKey {
    pub scheme: &'static str,
    pub key: String,
    pub tier: Tier,
}

impl CanonKey {
    fn e1(scheme: &'static str, key: impl Into<String>) -> Option<Self> {
        Some(CanonKey { scheme, key: key.into(), tier: Tier::E1 })
    }
    fn e2(scheme: &'static str, key: impl Into<String>) -> Option<Self> {
        Some(CanonKey { scheme, key: key.into(), tier: Tier::E2 })
    }
}

/// The canonical key for an org identifier, or `None` when the scheme offers
/// no cross-walk (the identifier then matches by E0 exact equality only).
///
/// `country` is the org row's country (alpha-2, as stored post-canonicalise);
/// for `kind = "vat"` the value's own prefix is the scheme authority (the
/// idgate census precedent) — the matcher checks row-country agreement
/// separately, so a contradictory row surfaces as an anomaly rather than
/// silently keying into the wrong country's series.
pub fn canonical_key(country: Option<&str>, kind: &str, value: &str) -> Option<CanonKey> {
    // Only the two kinds the resolver mints today. Anything else — a future
    // GLN/DIR3/platform-id kind, say — must NOT ride the national arms into
    // an E1 key (verifier catch: a 13-digit GLN under BG would have keyed as
    // an EIK, which denial rule 2 forbids).
    if kind != "vat" && kind != "national" {
        return None;
    }
    // A submission platform's own v4-UUID record key is never a register
    // identity, whatever kind the row happens to carry (issue 312). It stays
    // a perfectly good LINK — the resolver binds mentions on the raw
    // (country, kind, value) triple, which this does not touch — but it must
    // never become a MERGE key.
    //
    // MEASURED EFFECT TODAY: ZERO ROWS. The estimate that motivated this
    // guard was wrong and the prod census caught it — keyed_e1 was 366,766
    // before and after the deploy. A GUID cannot reach the national arms
    // anyway: they gate on `digits_only` below, and hex letters are the one
    // thing a 32-char UUID always has (1 FR specimen in 400 does carry
    // exactly 14 DIGITS, the SIRET arm's count, which is what fooled the
    // estimate — necessary, but nowhere near sufficient).
    //
    // Kept because it states the invariant instead of leaving it emergent
    // from an unrelated implementation detail: today "no platform key ever
    // merges" is TRUE ONLY BECAUSE every keying arm happens to require
    // all-digit bodies. Add one alphanumeric register scheme — and they
    // exist, HRB/FN shapes among them — and the property silently dies.
    // This line is what would survive that.
    if uuid_v4(value) {
        return None;
    }
    // Normalise: uppercase, strip every separator (spaces, dots, hyphens,
    // slashes) — `1234567-8`, `556 649-0192` and `CZ 699 000 797` all carry
    // their identity in the alphanumerics alone.
    let norm: String =
        value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    if norm.is_empty() {
        return None;
    }
    let is_vat = kind == "vat";
    // Scheme country: the VAT prefix. A vat value WITHOUT its prefix gets no
    // key at all (verifier catch: the idgate census scores prefix-less vats
    // as scheme "other" — Checksum::Unknown, so the hard-checksum gate never
    // sees them — and letting them key E1 through a row-country fallback
    // would build merges from an entirely ungated value class; the two
    // modules must resolve schemes identically).
    let (cc, body): (&str, &str) = if is_vat {
        match norm.get(..2) {
            Some(p) if p.bytes().all(|b| b.is_ascii_alphabetic()) => (
                match p {
                    // EL ≡ GR (design §3.1).
                    "EL" => "GR",
                    other => other,
                },
                norm.get(2..).unwrap_or(""),
            ),
            _ => return None,
        }
    } else {
        (
            match country {
                Some("EL") => "GR",
                Some(c) => c,
                None => return None,
            },
            norm.as_str(),
        )
    };
    let digits_only = body.bytes().all(|b| b.is_ascii_digit());
    let d = body.len();

    match cc {
        // FR — SIREN is the legal-unit key. SIRET (14 = SIREN + NIC
        // establishment counter) truncates to it (denial-list rule 2:
        // establishment ≠ entity; the fine id survives in raw_identifier).
        // A full FR VAT's 2-digit key is arithmetic over the SIREN — the E1
        // key exists only when that arithmetic PROVES the mapping.
        // Left-zero-padded 14-char forms are live ("00000219740248"): the
        // stripped 9-digit SIREN is a PAD-derived key ⇒ E2. Letter-bearing
        // FR VAT keys (real but rare) are deliberately refused by the
        // digits_only guard — a narrowing in the safe direction.
        "FR" if digits_only => {
            if is_vat {
                if d == 11 {
                    let digits: Vec<u8> = body.bytes().map(|b| b - b'0').collect();
                    return match fr_vat_key(&digits) {
                        Checksum::Pass => CanonKey::e1("FR:siren", &body[2..]),
                        _ => None,
                    };
                }
                return None;
            }
            match d {
                9 => CanonKey::e1("FR:siren", body),
                // A 14-digit leading-zero value whose zero-strip is EXACTLY 9
                // digits is genuinely ambiguous: a bare SIREN left-padded to
                // 14 (live: "00000219740248") — or a real SIRET of a very
                // low SIREN, whose true key would be the 0-leading first 9.
                // The strip reading gets the key, at E2: ambiguity is an
                // information gap only R3's corroboration may close.
                // Any OTHER leading-zero 14 is just a SIRET whose SIREN
                // happens to start with 0 (0-leading SIRENs exist —
                // verifier catch): plain truncation, E1.
                14 if body.starts_with('0') => {
                    let stripped = body.trim_start_matches('0');
                    if stripped.len() == 9 {
                        CanonKey::e2("FR:siren", stripped)
                    } else {
                        CanonKey::e1("FR:siren", &body[..9])
                    }
                }
                14 => CanonKey::e1("FR:siren", &body[..9]),
                _ => None,
            }
        }
        // PL — VAT ↔ NIP (10 digits); leading-zero 10-digit nationals are
        // zero-padded KRS serials, not NIPs (idgate precedent). REGON-14
        // truncates to the REGON-9 legal unit — its OWN series, never
        // cross-walked to NIP.
        "PL" if digits_only => match (is_vat, d) {
            (_, 10) if !body.starts_with('0') => CanonKey::e1("PL:nip", body),
            (false, 14) => CanonKey::e1("PL:regon", &body[..9]),
            (false, 9) => CanonKey::e1("PL:regon", body),
            _ => None,
        },
        // IT — VAT ↔ Partita IVA (11 digits). The 16-char Codice Fiscale is
        // NO signal either way (design §3.1): no key.
        "IT" if digits_only && d == 11 => CanonKey::e1("IT:piva", body),
        // ES — VAT ↔ NIF, literal key, demoted whole to E2 (verifier catch,
        // the DIR3 collision): Spanish public bodies are pervasively
        // identified by DIR3 codes (letter + 8 digits, e.g. "L01280796"),
        // which share their exact shape with letter-check CIFs ("A01002820"
        // is a valid CIF shape AND a valid DIR3 shape), and both arrive as
        // kind="national". An E1 key here could auto-merge a company with a
        // public administration; no syntactic test separates the series, so
        // the ambiguity is an information gap — E2, R3's corroboration
        // stack only. A pure-digit 9-char body is NEITHER series (a NIF
        // always carries a letter — and bare "123456789" is the pinned
        // placeholder): no key. UTE NIFs (letter U) are ephemeral
        // per-procedure constructs: never a key (denial rule 3).
        "ES" if d == 9 => {
            if body.starts_with('U') || !body.bytes().any(|b| b.is_ascii_alphabetic()) {
                return None;
            }
            CanonKey::e2("ES:nif", body)
        }
        // RO — CUI is prefix-insensitive digits (2-10 of them). No zero
        // games: the digits ARE the key, as stored.
        "RO" if digits_only && (2..=10).contains(&d) => CanonKey::e1("RO:cui", body),
        // CZ — DIČ = CZ + the 8-digit IČO: prefix strip at FULL length is
        // E1. CZ699… group DIČs identify VAT GROUPS, not entities (denial
        // rule 1): no key. A 9/10-digit DIČ body is a birth-number
        // (individual): no cross-walk. A 7-digit national is a lost leading
        // zero — pad-derived ⇒ E2 (the Justice/Assay collision exemplar);
        // a 7-digit VAT body deliberately gets NO pad (a mis-stored DIČ is
        // less trustworthy than a mis-stored national — narrower than the
        // amendment, in the safe direction).
        "CZ" if digits_only => {
            if body.starts_with("699") && is_vat {
                return None;
            }
            match d {
                8 => CanonKey::e1("CZ:ico", body),
                7 if !is_vat => CanonKey::e2("CZ:ico", format!("0{body}")),
                _ => None,
            }
        }
        // BE — KBO/BCE ↔ VAT at 10 digits, ENTERPRISE numbers only (leading
        // 0/1): establishment-unit numbers lead 2-8 and are location-scoped
        // (denial rule 2 — establishment ≠ entity; no arithmetic reaches the
        // parent enterprise, so no key at all). The pre-2008 9-digit legacy
        // form gains a leading zero: pad-derived ⇒ E2.
        "BE" if digits_only => match d {
            10 if body.starts_with('0') || body.starts_with('1') => CanonKey::e1("BE:kbo", body),
            9 => CanonKey::e2("BE:kbo", format!("0{body}")),
            _ => None,
        },
        // SE — VAT = organisationsnummer + literal "01" suffix; the strip is
        // E1 only when the suffix really is "01" (else the value is not the
        // documented VAT shape and proves nothing).
        "SE" if digits_only => match (is_vat, d) {
            (true, 12) if body.ends_with("01") => CanonKey::e1("SE:orgnr", &body[..10]),
            (false, 10) => CanonKey::e1("SE:orgnr", body),
            _ => None,
        },
        // DK — CVR ↔ VAT (8 digits). The 10-digit P-nummer (establishment)
        // has no arithmetic relation to its CVR: no key (denial rule 2).
        "DK" if digits_only && d == 8 => CanonKey::e1("DK:cvr", body),
        // FI — Y-tunnus ↔ VAT (8 digits; the hyphen died in normalisation).
        // The legacy 6-digit series needs a pad whose position is not
        // recoverable from the value — no key at all (narrower than the
        // design's E2 note, deliberately: an unprovable pad is not a key).
        "FI" if digits_only && d == 8 => CanonKey::e1("FI:ytunnus", body),
        // PT — NIF ↔ VAT (9 digits).
        "PT" if digits_only && d == 9 => CanonKey::e1("PT:nif", body),
        // HR — OIB ↔ VAT (11 digits).
        "HR" if digits_only && d == 11 => CanonKey::e1("HR:oib", body),
        // NL — VAT → RSIN head, demoted whole to E2 (verifier catches): the
        // design's "legal entities only" qualifier has no enforceable
        // syntactic test here — post-2020 sole-trader btw-ids carry a
        // generated 9-digit head that is NOT an RSIN, and a fiscale-eenheid
        // (VAT group) btw-id looks exactly like a member's. So a VAT-derived
        // head is candidate-edge material only; R3's corroboration stack
        // decides. A bare 9-digit national RSIN keeps E1 — that key is the
        // identity itself, no derivation. The 8-digit KvK number is a
        // DIFFERENT register: no cross-walk.
        "NL" => {
            if is_vat && d == 12 && body.as_bytes()[9] == b'B' {
                let (head, tail) = (&body[..9], &body[10..]);
                if head.bytes().all(|b| b.is_ascii_digit())
                    && tail.bytes().all(|b| b.is_ascii_digit())
                {
                    return CanonKey::e2("NL:rsin", head);
                }
                return None;
            }
            if !is_vat && digits_only && d == 9 {
                return CanonKey::e1("NL:rsin", body);
            }
            None
        }
        // HU — the 8-digit törzsszám heads every form: adószám (8-1-2) and
        // VAT (HU + 8) both truncate to it (design §3.1 "first-8") — EXCEPT
        // a group id: áfakód 5 (the 9th digit) marks a VAT GROUP's
        // csoportazonosító szám, whose törzsszám names the group, not any
        // member (verifier catch — two members publishing it must not key
        // together): no key.
        "HU" if digits_only && (d == 8 || d == 11) => {
            if d == 11 && body.as_bytes()[8] == b'5' {
                return None;
            }
            CanonKey::e1("HU:torzsszam", &body[..8])
        }
        // BG — EIK ↔ VAT at 9 digits, the design's listed pair. (A 13-digit
        // branch UIC is EIK + 4 and truncation would be rule-2-shaped, but
        // §3.1 does not enumerate it — an unratified auto-merge path stays
        // out until the board ratifies it.)
        "BG" if digits_only && d == 9 => CanonKey::e1("BG:eik", body),
        // LV — the 11-digit register number is the VAT body verbatim.
        "LV" if digits_only && d == 11 => CanonKey::e1("LV:regnr", body),
        // SK — IČ-DPH (SK + 10) ↔ DIČ (10 digits) ONLY. The 8-digit IČO is
        // a different register and NEVER cross-walks (design §3.1; the
        // must-FLAG panel). KNOWN HAZARD with no syntactic test: Slovak
        // group VAT registration issues ONE shared IČ DPH to every member —
        // the mention-evidence VAT-group wall (denial rule 1) is the ONLY
        // defense, so the Stage-2 merge job MUST implement it before any SK
        // wet run.
        "SK" if digits_only => match d {
            10 => CanonKey::e1("SK:dic", body),
            8 if !is_vat => CanonKey::e1("SK:ico", body),
            _ => None,
        },
        // NO — organisasjonsnummer ↔ VAT (9 digits; a trailing "MVA" died in
        // normalisation? No — MVA is alphabetic and survives; strip it here).
        "NO" => {
            let stripped = body.strip_suffix("MVA").unwrap_or(body);
            if stripped.len() == 9 && stripped.bytes().all(|b| b.is_ascii_digit()) {
                CanonKey::e1("NO:orgnr", stripped)
            } else {
                None
            }
        }
        // SI — davčna številka ↔ VAT (8 digits).
        "SI" if digits_only && d == 8 => CanonKey::e1("SI:davcna", body),
        // GR — AFM ↔ VAT (9 digits), EL folded to GR above.
        "GR" if digits_only && d == 9 => CanonKey::e1("GR:afm", body),
        // DE, AT, IE, LU, CY, MT, EE, LT, and everything unlisted: no
        // cross-walk. E0 exact equality is the only merge path.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    /// Issue 312: a platform's v4-UUID record key never becomes a merge
    /// key, in any country, under either kind — while remaining untouched
    /// as a LINK (the resolver binds on the raw triple, not on this).
    ///
    /// The FR specimen is the shape the measurement found: exactly 14 digit
    /// characters, which is what the SIRET arm keys on, so before this
    /// guard it produced an E1 key.
    #[test]
    fn platform_guids_are_never_a_merge_key() {
        // A REAL specimen from the corpus (an FR row, measured 2026-08-30):
        // exactly 14 digit characters, the SIRET arm's count. It did NOT
        // key before the guard — `digits_only` already excluded it — so
        // this pins the invariant, not a repaired defect.
        let siret_shaped = "0EC1CA3FA1F94A4FAF1F8EB4DC20CC01";
        assert_eq!(
            siret_shaped.chars().filter(char::is_ascii_digit).count(),
            14,
            "the specimen is the siret-shaped case this guard exists for"
        );
        assert_eq!(canonical_key(Some("FR"), "national", siret_shaped), None);
        for (country, kind) in
            [(Some("FR"), "national"), (Some("DE"), "vat"), (Some("BE"), "national"), (None, "national")]
        {
            assert_eq!(
                canonical_key(country, kind, "DA23095600854B59BC39FE71D8AF0A7C"),
                None,
                "{country:?}/{kind} must not key a platform GUID"
            );
        }
        // The dashed form too, and a non-v4 32-hex value is NOT swept in.
        assert_eq!(canonical_key(Some("FR"), "national", "da230956-0085-4b59-bc39-fe71d8af0a7c"), None);
        // Real identifiers are untouched — the guard is narrow.
        assert!(canonical_key(Some("FR"), "national", "180014045").is_some(), "a real SIREN still keys");
        assert!(
            canonical_key(Some("FI"), "national", "01003158").is_some(),
            "a real FI Y-tunnus still keys"
        );
    }

    use super::*;

    fn key(country: Option<&str>, kind: &str, value: &str) -> Option<(String, String, Tier)> {
        canonical_key(country, kind, value)
            .map(|k| (k.scheme.to_owned(), k.key.clone(), k.tier))
    }

    /// The pinned Stage-2 positive panel (300-exemplars.md): representations
    /// of one registration MUST share one E1 key.
    #[test]
    fn pinned_e1_pairs_unify() {
        // Telinekataja Oy — FI vat FI01003158 ↔ FI national 01003158.
        assert_eq!(key(Some("FI"), "vat", "FI01003158"), key(Some("FI"), "national", "01003158"));
        // The hyphenated storage form (Telinekataja's real Y-tunnus shape)
        // is the same key — the separator dies in normalisation.
        assert_eq!(key(Some("FI"), "national", "0100315-8"), key(Some("FI"), "vat", "FI01003158"));
        // Ramboll — the rename pairs merge on identifier evidence alone.
        assert_eq!(key(Some("FI"), "vat", "FI01011975"), key(Some("FI"), "national", "01011975"));
        // RO CUI — bare ↔ prefixed (Societatea de Transport București).
        assert_eq!(key(Some("RO"), "national", "1589886"), key(Some("RO"), "vat", "RO1589886"));
        // CNFPT — SIRET establishments truncate onto the SIREN, and the full
        // FR VAT's key arithmetic proves the same SIREN (79 is 180014045's
        // real key).
        let siren = key(Some("FR"), "national", "180014045");
        assert_eq!(key(Some("FR"), "national", "18001404501577"), siren);
        assert_eq!(key(Some("FR"), "national", "18001404502245"), siren);
        assert_eq!(key(Some("FR"), "vat", "FR79180014045"), siren);
        assert_eq!(siren, Some(("FR:siren".into(), "180014045".into(), Tier::E1)));
        // A WRONG FR VAT key proves nothing: no key at all.
        assert_eq!(key(Some("FR"), "vat", "FR12180014045"), None);
        // SE — Softronic's orgnr ↔ its VAT form (orgnr + "01").
        assert_eq!(
            key(Some("SE"), "vat", "SE556249019201"),
            key(Some("SE"), "national", "5562490192")
        );
        // A VAT tail that is not "01" is not the documented shape.
        assert_eq!(key(Some("SE"), "vat", "SE556249019299"), None);
        // CZ — DIČ ↔ IČO at full 8-digit length (Ministerstvo financí).
        assert_eq!(key(Some("CZ"), "vat", "CZ00006947"), key(Some("CZ"), "national", "00006947"));
        // GR/EL fold.
        assert_eq!(key(Some("GR"), "vat", "EL094019245"), key(Some("GR"), "national", "094019245"));
        // PL — VAT ↔ NIP; REGON-14 → REGON-9 stays its own series.
        assert_eq!(key(Some("PL"), "vat", "PL5262239325"), key(Some("PL"), "national", "5262239325"));
        assert_eq!(
            key(Some("PL"), "national", "47085064500000"),
            key(Some("PL"), "national", "470850645")
        );
        assert_ne!(
            key(Some("PL"), "national", "470850645"),
            key(Some("PL"), "national", "4708506450"),
            "REGON-9 and a 10-digit NIP-shaped value never share a series"
        );
        // HU — first-8 heads the 11-digit adószám and the row form.
        assert_eq!(
            key(Some("HU"), "national", "10773381-2-07"),
            key(Some("HU"), "vat", "HU10773381")
        );
        // NO — MVA suffix strips.
        assert_eq!(key(Some("NO"), "vat", "NO974760673MVA"), key(Some("NO"), "national", "974760673"));
        // FR — a genuine SIRET of a 0-leading SIREN truncates at E1: only
        // the strip-to-exactly-9 shape is pad-ambiguous (verifier catch).
        assert_eq!(
            key(Some("FR"), "national", "05548012300012"),
            Some(("FR:siren".into(), "055480123".into(), Tier::E1))
        );
        // BE — enterprise numbers (leading 0/1) key E1.
        assert_eq!(key(Some("BE"), "vat", "BE0123456749"), key(Some("BE"), "national", "0123456749"));
    }

    /// The pinned must-NOT panel: pairs the walk must REFUSE to unify at E1.
    #[test]
    fn pinned_negatives_refuse() {
        // SK — DIČ never cross-walks to IČO: different schemes.
        let dic = key(Some("SK"), "national", "2021853504").unwrap();
        let ico = key(Some("SK"), "national", "31364501").unwrap();
        assert_ne!(dic.0, ico.0, "SK DIČ and IČO are separate registers");
        // CZ699 group VATs identify VAT groups, not entities.
        assert_eq!(key(Some("CZ"), "vat", "CZ699000797"), None);
        // ES UTE NIFs are ephemeral per-procedure constructs.
        assert_eq!(key(Some("ES"), "national", "U12345678"), None);
        // DE has no cross-walk at all, and the two halves are refused for
        // DIFFERENT reasons — stated separately because "court-scoped
        // registers" is an argument about HRB that says nothing about VAT, and
        // reading it as covering both is what re-opened this once (issue 329).
        //
        // VAT: German public bodies share a Land-level VAT registration.
        // MEASURED, issue 329 job 568: of the 3,215 standing duplicate
        // (DE, vat, DEnnnnnnnnn) triples, 559 (17.4%) hold names that disagree
        // outright, and reading them shows the class is governmental —
        // DE811335517 is held by the Regierung von Oberbayern, the Regierung
        // von Mittelfranken and two Vergabekammern, over 25,018 mentions. An
        // identifier-only arm would merge distinct public authorities, and
        // would do it hardest where it moved the most corpus.
        assert_eq!(key(Some("DE"), "vat", "DE136695976"), None);
        // NATIONAL: HRB numbers are scoped to the issuing court, so the same
        // string names different companies in different registers.
        assert_eq!(key(Some("DE"), "national", "HRB 12345"), None);
        // EE VAT and registrikood are separate series.
        assert_eq!(key(Some("EE"), "vat", "EE100931558"), None);
        assert_eq!(key(Some("EE"), "national", "10913146"), None);
        // NULL-country nationals cannot resolve a scheme.
        assert_eq!(key(None, "national", "12345678"), None);
        // IT Codice Fiscale is no signal either way.
        assert_eq!(key(Some("IT"), "national", "RSSMRA85T10A562S"), None);
        // NL KvK (8 digits) is not the RSIN register.
        assert_eq!(key(Some("NL"), "national", "12345678"), None);
        // A kind the resolver does not mint gets no key — a future GLN/DIR3
        // kind must not ride the national arms (verifier catch).
        assert_eq!(key(Some("BE"), "gln", "0123456789"), None);
        // A prefix-less VAT is an UNGATED value class (the idgate census
        // scores it "other"): no key, matching the gate's own blindness.
        assert_eq!(key(Some("FI"), "vat", "01003158"), None);
        // HU áfakód 5 marks a VAT GROUP's id: the group's törzsszám names
        // no member.
        assert_eq!(key(Some("HU"), "national", "12345678-5-02"), None);
        // BE establishment-unit numbers (leading 2-8) are location-scoped.
        assert_eq!(key(Some("BE"), "national", "2123456789"), None);
        // BG 13-digit branch UICs stay unkeyed until the board ratifies the
        // truncation (§3.1 lists only VAT↔EIK).
        assert_eq!(key(Some("BG"), "national", "1234567890123"), None);
        // A bare-digit 9-char ES body is neither a NIF nor a DIR3 shape —
        // and "123456789" is the pinned placeholder.
        assert_eq!(key(Some("ES"), "national", "123456789"), None);
    }

    /// The DIR3 demotion (verifier catch): Spanish DIR3 authority codes
    /// share their exact shape with letter-check CIFs, so ES keys are E2 —
    /// present as candidate edges, NEVER auto-merge material — and the NL
    /// VAT head is E2 for its own reasons (sole-trader heads and
    /// fiscale-eenheid group ids are not RSINs).
    #[test]
    fn ambiguous_series_demote_to_e2() {
        // A company CIF (vat) and a public body's DIR3 (national) with the
        // same characters: both key, both E2 — the collision becomes an
        // edge for R3, never an auto-merge.
        let cif = key(Some("ES"), "vat", "ESA01002820").unwrap();
        let dir3 = key(Some("ES"), "national", "A01002820").unwrap();
        assert_eq!(cif.1, dir3.1, "the shapes really do collide — that is the danger");
        assert_eq!((cif.2, dir3.2), (Tier::E2, Tier::E2));
        // NL: the VAT-derived head is E2; the bare national RSIN keeps E1.
        assert_eq!(key(Some("NL"), "vat", "NL003660564B01").unwrap().2, Tier::E2);
        assert_eq!(key(Some("NL"), "national", "003660564").unwrap().2, Tier::E1);
        assert_eq!(
            key(Some("NL"), "vat", "NL003660564B01").unwrap().1,
            key(Some("NL"), "national", "003660564").unwrap().1,
            "the head still names the same key — as an edge"
        );
    }

    /// The pad amendment (design §3.1): pad-derived keys are E2 — candidate
    /// edges only. The live CZ collision is the reason.
    #[test]
    fn pad_derived_keys_are_e2_only() {
        // The Justice/Assay collision: 0002542 (corrupted, 7 digits) pads
        // onto 00002542 (Puncovní úřad's REAL IČO). The keys collide — but
        // at E2, so R2/E1 never merges them; R3's corroboration stack (which
        // the mismatched names fail) is the only path.
        let padded = key(Some("CZ"), "national", "0002542").unwrap();
        let real = key(Some("CZ"), "national", "00002542").unwrap();
        assert_eq!(padded.1, real.1, "the pad does collide — that is the danger");
        assert_eq!(padded.2, Tier::E2, "…so the pad-derived key must be E2");
        assert_eq!(real.2, Tier::E1, "the full-length form is the real key");
        // The corroborated Ministerstvo financí family rides the same rails:
        // 0006947 (7-digit) keys E2 onto 00006947's E1 key.
        assert_eq!(key(Some("CZ"), "national", "0006947").unwrap().2, Tier::E2);
        assert_eq!(
            key(Some("CZ"), "national", "0006947").unwrap().1,
            key(Some("CZ"), "national", "00006947").unwrap().1
        );
        // FR left-zero-padded 14-char forms (live: "00000219740248").
        let fr = key(Some("FR"), "national", "00000219740248").unwrap();
        assert_eq!((fr.1.as_str(), fr.2), ("219740248", Tier::E2));
        // BE 9-digit legacy pads onto the 10-digit KBO.
        let be = key(Some("BE"), "national", "123456789").unwrap();
        assert_eq!((be.1.as_str(), be.2), ("0123456789", Tier::E2));
        assert_eq!(key(Some("BE"), "national", "0123456789").unwrap().2, Tier::E1);
    }
}

/// [`canonical_key`] flattened to the fn-pointer shape the store's injected
/// rule slots take: `(scheme, key, is_e1)`. One definition, used by the
/// resolver's Stage-2 prevention hook and the R2 merge job alike — the two
/// MUST share one crosswalk, or prevention and repair drift apart.
pub fn canonical_key_flat(
    country: Option<&str>,
    kind: &str,
    value: &str,
) -> Option<(&'static str, String, bool)> {
    canonical_key(country, kind, value).map(|ck| (ck.scheme, ck.key, ck.tier == Tier::E1))
}

/// Issue 329's E0 rule in the flat key shape: an org's exact `(country, kind,
/// identifier)` triple is its own group WHEN no cross-walk arm keys the value
/// — an arm-keyed value is R2's business, never both. The scheme is the
/// literal `"E0"`; the kind rides in the key so a VAT row and a national row
/// with the same literal never group. The store's E0 name rule (agree on one
/// non-generic N3 key) does the rest.
pub fn e0_key_flat(
    country: Option<&str>,
    kind: &str,
    value: &str,
) -> Option<(&'static str, String, bool)> {
    if kind != "vat" && kind != "national" {
        return None;
    }
    if canonical_key(country, kind, value).is_some() {
        return None;
    }
    let norm: String =
        value.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
    if norm.is_empty() {
        return None;
    }
    Some(("E0", format!("{kind}:{norm}"), true))
}

/// Consortium / temporary-grouping detection over an org NAME (the census
/// finding, 2026-08-29): FR groupements publish the LEAD MEMBER's SIRET
/// ("groupement colas / Barthelemy" carries Colas's establishment id — org
/// 10207212, the live specimen), the FR analog of the ES UTE class. Merging
/// the grouping INTO its lead member is wrong the way UTE merges are wrong,
/// so a name hit routes the group to the edge path, never auto-merge.
/// Token-boundary matching on the lowercased name: "gpt" the token, not
/// "egypt"; the list stays deliberately short and measured — the Stage-2
/// precision review grows it, guesses do not.
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

/// The legal-form FAMILY named in an org name, when one is unambiguous — the
/// input to denial rule 7 (the legal-form-contradiction veto): two orgs whose
/// names carry DIFFERENT families ("X GmbH" vs "X AG" sharing a VAT — the
/// Organschaft signature) must not auto-merge; the miss is recoverable, the
/// merge is not. Same-country matching only (R2), so cross-country token
/// collisions ("a.s." CZ vs "AS" NO) never meet. Returns the FIRST family
/// token found; names with none return None and never veto.
/// One legal-form TOKEN's family, shared by the veto (`legal_form_family`)
/// and the Stage-4 N3 key builder (`n3_key`) — one table, so the veto and
/// the edge keys can never disagree about what a form token means.
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

/// The dotted multi-token forms as they appear in a `match_norm` token
/// stream ("s.r.o." → `s r o`, "spol. s r.o." → `spol s r o`,
/// "Ges.m.b.H." → `ges m b h`, "Sp. z o.o." → `sp z o o`). Longest first —
/// the scanner takes the first sequence that matches at a position.
const FAMILY_SEQUENCES: [(&[&str], &str); 4] = [
    (&["spol", "s", "r", "o"], "sro"),
    (&["ges", "m", "b", "h"], "gmbh"),
    (&["sp", "z", "o", "o"], "spzoo"),
    (&["s", "r", "o"], "sro"),
];

pub fn legal_form_family(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    // Dotted forms ("s.r.o.", "a/s", "sp. z o.o.") collapse once separators
    // die; tokenising the raw lowercase on non-alphanumerics yields their
    // letters as consecutive tokens, so match on the JOINED stream too.
    let joined: String = lower.chars().filter(char::is_ascii_alphanumeric).collect();
    for t in lower.split(|c: char| !c.is_alphanumeric()) {
        let fam = family_token(t);
        if fam.is_some() {
            return fam;
        }
    }
    // The dotted multi-token forms, whole-name scoped: rare enough that a
    // substring probe on the alphanumeric stream is honest (an embedded
    // "sro" inside a WORD cannot happen — the stream only collapses across
    // separators the name actually printed).
    for (needle, fam) in [("spzoo", "spzoo"), ("sro", "sro")] {
        if joined.ends_with(needle) {
            return Some(fam);
        }
    }
    None
}

/// Issue 300 Stage 4 (§2.3), the N3 name key: the N2 key
/// ([`crate::project::match_norm`] — the plan's fidelity round pinned that
/// match_norm IS N2; N3 never forks it) with legal-form tokens
/// CANONICALIZED to a `§family` marker in place — never stripped (stripping
/// is E4 material, a later unit). `s.r.o.` / `s. r. o.` / `spol. s r.o.`
/// all become one `§sro` token; `GmbH` / `Ges.m.b.H.` / `mbH` one `§gmbh`.
/// The `§` marker cannot collide with real content: match_norm folds every
/// non-alphanumeric away, so no N2 token ever contains it. Names with no
/// recognized form come out EXACTLY equal to their N2 key (the documented
/// invariant, pinned in test). Over-recognition costs only an edge key —
/// N3 feeds candidate EDGES, never merges — while the family table itself
/// is shared with the R2/R3 legal-form veto via [`family_token`], so keys
/// and vetoes cannot drift apart.
/// The name-key SEMANTICS epoch, stamped beside the org_match_keys build
/// watermark: bump it whenever `match_norm` (N2) or [`n3_key`] (N3)
/// changes meaning, so a build resumed across the deploy restarts from
/// zero instead of mixing semantics in one table.
pub const NAME_KEY_EPOCH: &str = "n2v1+n3v1";

pub fn n3_key(name: &str) -> String {
    let n2 = crate::project::match_norm(name);
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

#[cfg(test)]
mod veto_tests {
    use super::*;

    #[test]
    fn consortium_names_flag_and_plain_names_do_not() {
        assert!(consortium_name("groupement colas / Barthelemy"));
        assert!(consortium_name("GPT SNBR / EHTP"));
        assert!(consortium_name("Consortium Stabile Arcale"));
        assert!(consortium_name("Bouygues Énergies & Services (mandataire)"));
        assert!(consortium_name("UTE Acciona-Sacyr"));
        assert!(
            consortium_name("Consórcio E.I.P. Serviços, S.A. _ CME"),
            "the precision review's PT catch"
        );
        assert!(
            !consortium_name("OPEGIEKA Sp. z o.o. - członek konsorcjum"),
            "a MEMBER labelled as such is not the vehicle"
        );
        assert!(!consortium_name("Colas Centre Ouest"));
        assert!(!consortium_name("Egyptian Trading Co"), "gpt must match as a token only");
        // The German Bietergemeinschaft class (Lennart's catch): both the
        // BIEGE abbreviation and the spelled-out forms, plus arge's
        // spelled-out sibling.
        assert!(consortium_name("BIEGE: Bernard Ingenieure ZT GmbH"));
        assert!(consortium_name("BIEGE VE Wärme AG:Zechbau GmbH, Umweltschutz Ost GmbH"));
        assert!(consortium_name("Bietergemeinschaft Müller Bau / Schulz Tiefbau"));
        assert!(consortium_name("Arbeitsgemeinschaft Tunnelbau Nord"));
        assert!(
            !consortium_name("Thoman Biegemaschinen GbR"),
            "a bending-machine builder is not a Bietergemeinschaft — token match only"
        );
    }

    #[test]
    fn n3_keys_canonicalize_forms_and_leave_formless_names_as_n2() {
        // Punctuation-variant forms of ONE company collapse to one key.
        assert_eq!(n3_key("Alfa s.r.o."), "alfa §sro");
        assert_eq!(n3_key("Alfa s. r. o."), n3_key("Alfa s.r.o."));
        assert_eq!(n3_key("Alfa spol. s r.o."), n3_key("Alfa sro"));
        assert_eq!(n3_key("Siemens GmbH"), "siemens §gmbh");
        assert_eq!(n3_key("Siemens Ges.m.b.H."), n3_key("Siemens GmbH"));
        assert_eq!(n3_key("Siemens Gesellschaft mbH"), "siemens gesellschaft §gmbh");
        assert_eq!(n3_key("Beta Sp. z o.o."), "beta §spzoo");
        // The Nordic family folds to one token, so the Linde/AGA rename
        // shape keys equal across the Oy/Ab spellings.
        assert_eq!(n3_key("Telinekataja Oy"), "telinekataja §aktiebolag");
        assert_eq!(n3_key("Telinekataja Ab"), n3_key("Telinekataja Oy"));
        // The documented invariant: no recognized form ⇒ n3 == n2 exactly.
        for name in ["Ministerstvo financí", "Ville de Calais", "OPAC du Rhône"] {
            assert_eq!(n3_key(name), crate::project::match_norm(name), "{name}");
        }
        // The marker cannot collide: a literal § in the INPUT is folded
        // away by match_norm before the scan ever runs (the surviving
        // "sro" token then canonicalizes like any undotted form).
        assert_eq!(n3_key("Weird §sro Name"), "weird §sro name");
        assert_eq!(n3_key("Weird§Name"), "weird name");
    }

    #[test]
    fn legal_form_families_split_and_agree() {
        assert_eq!(legal_form_family("Siemens GmbH"), Some("gmbh"));
        assert_eq!(legal_form_family("Siemens AG"), Some("ag"));
        assert_eq!(legal_form_family("Alfa s.r.o."), Some("sro"));
        assert_eq!(legal_form_family("Beta Sp. z o.o."), Some("spzoo"));
        assert_eq!(legal_form_family("Ministerstvo financí"), None);
        // Oy/Ab/Oyj are ONE Nordic family: naming variants of the same
        // company ("X Oy" vs "X Ab") and the rename exemplars (Linde/AGA)
        // must never veto each other.
        assert_eq!(legal_form_family("Oy Linde Gas Ab"), legal_form_family("Telinekataja Oy"));
        assert_eq!(legal_form_family("Ramboll Ab"), legal_form_family("Ramboll Oyj"));
    }
}

#[cfg(test)]
mod e0 {
    use super::{canonical_key_flat, e0_key_flat};

    /// The Greek authority code (the 311/1079 specimen) has no arm, so it is
    /// an E0 group; a Finnish Y-tunnus is the FI arm's and never E0's; the
    /// kind is part of the key; a non-identifier kind is nobody's.
    #[test]
    fn e0_groups_exactly_the_triples_no_arm_keys() {
        assert_eq!(
            e0_key_flat(Some("GR"), "national", "1000E009610001"),
            Some(("E0", "national:1000E009610001".to_owned(), true))
        );
        assert!(canonical_key_flat(Some("FI"), "national", "01003158").is_some());
        assert_eq!(e0_key_flat(Some("FI"), "national", "01003158"), None);
        assert_ne!(
            e0_key_flat(Some("DE"), "vat", "DEX1"),
            e0_key_flat(Some("DE"), "national", "DEX1")
        );
        assert_eq!(e0_key_flat(Some("DE"), "gln", "9110000000001"), None);
        assert_eq!(e0_key_flat(Some("DE"), "national", "--"), None);
    }
}
