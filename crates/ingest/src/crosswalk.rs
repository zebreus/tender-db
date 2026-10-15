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
    /// Issue 470: the key is NOT the literal's own characters. A GB company
    /// number typed with a letter O for a zero (`SCO55775`, `O6611251`), or a
    /// zero for the O of `OC` (`0C301540`), keys to the number the register
    /// format says was meant — which one entity in sixteen measured pairs was
    /// not (the University of Aberdeen's charity number beside Net Zero
    /// Technology Centre's company number). So a folded key never binds or
    /// merges on the key alone: R2 merges a folded member only when its names
    /// agree with its group's (`altid_keys_agree`), the resolver binds a folded
    /// mention only to an owner its names match (452's guarded path), and
    /// every other consumer of [`canonical_key_flat`] reads the flag. `false`
    /// on every other arm.
    pub folded: bool,
}

impl CanonKey {
    fn e1(scheme: &'static str, key: impl Into<String>) -> Option<Self> {
        Some(CanonKey { scheme, key: key.into(), tier: Tier::E1, folded: false })
    }
    fn e2(scheme: &'static str, key: impl Into<String>) -> Option<Self> {
        Some(CanonKey { scheme, key: key.into(), tier: Tier::E2, folded: false })
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
                // Issue 358: a regional code keys in its register's series,
                // so the standing `RE` row and the `FR` row carrying one
                // SIREN share a canonical key (R2 folds them; the resolver
                // binds a new mention to whichever stands).
                Some(c) => store::register_jurisdiction(c),
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
        // GB — the FIRST alphanumeric register arm, which is the event the
        // `uuid_v4` guard above was written in anticipation of: "no platform
        // key ever merges" stops being emergent from all-digit bodies here,
        // and that guard is now the only thing holding it. Two registers, and
        // they NEVER cross-walk to each other — a company can hold both, and
        // `additionalIdentifiers` pairing them is E2 evidence for R3, not a
        // key (issue 342).
        //
        // Companies House numbers are 8 characters: eight digits, or two
        // letters and six digits (`SC123456` Scotland, `NI…`, `OC…`, `SO…`,
        // `NF…`, `FC…`, `GE…`, `IP…`, `RC…`), or `R0` and six digits (the
        // `R0` series: `R0000568` is NORTHERN BANK LIMITED, `R0000273` H.& J.
        // MARTIN LIMITED — issue 470; two-letters-then-six refused it, so
        // `GBCOHR0000273` and `R0000273` stood as two orgs). All three full
        // forms are E1. Only `R0`: an `R` before seven other digits is no
        // series the register was measured to issue, and `GBR1234567` (the
        // ISO-3 code before a number that lost its zero) would key as one. A 6-7 digit body is a lost leading zero and pads ⇒
        // E2, the CZ/BE precedent.
        //
        // An 8-character body with a letter O where the format has a digit,
        // or a digit 0 for the O of `OC`, is FOLDED to the number meant
        // ([`gb_coh_fold`], issue 470) and the key says so
        // ([`CanonKey::folded`]): 15 of 16 standing lookalike pairs named one
        // entity, the 16th two bodies sharing a core word, so the fold is a
        // key only a name gate may act on. A fold onto a number the v2 gate
        // condemns keys nothing: the gate reads the literal, and `OOOOOO12`
        // passes it while its fold `00000012` is a zero-padded stub.
        //
        // PPON is the Find a Tender supplier-registration series
        // (`GB-PPON-PBZB-4962-TVLR` → `GBPPONPBZB4962TVLR`), 12 alphanumerics
        // after the prefix and its own scheme forever: it is a PLATFORM
        // registration, so unifying it with a company number would be exactly
        // the merge `platform_guids_are_never_a_merge_key` refuses.
        //
        // `body` here is the WHOLE normalised string including `GB` — nothing
        // upstream strips it for a `national` kind (`LABEL_PREFIXES` has no
        // `G` entry and `REGISTER_PREFIXES` no GB one), so the strip is this
        // arm's job. A real GB VAT arrives in this same arm with `is_vat` and
        // a 9-digit body; every branch below is guarded against it, because
        // the downstream `scheme.starts_with(c)` check cannot catch it —
        // `"GB:coh".starts_with("GB")` is true.
        "GB" if !is_vat => {
            let ppon = body.strip_prefix("GBPPON").or_else(|| body.strip_prefix("PPON"));
            if let Some(serial) = ppon {
                return if serial.len() == 12 && serial.bytes().all(|b| b.is_ascii_alphanumeric()) {
                    CanonKey::e1("GB:ppon", serial)
                } else {
                    None
                };
            }
            let coh = body
                .strip_prefix("GBCOH")
                .or_else(|| body.strip_prefix("COH"))
                .or_else(|| body.strip_prefix("GB"))
                .unwrap_or(body);
            if coh.is_empty() {
                return None;
            }
            let fold = gb_coh_fold(coh);
            let (coh, folded) = match &fold {
                Some(f) => (f.as_str(), true),
                None => (coh, false),
            };
            if folded && crate::idgate::condemns(Some("GB"), "national", coh) {
                return None;
            }
            let digits = coh.bytes().all(|b| b.is_ascii_digit());
            let letters_then_six = coh.len() == 8
                && coh[..2].bytes().all(|b| b.is_ascii_alphabetic())
                && coh[2..].bytes().all(|b| b.is_ascii_digit());
            let r_series =
                coh.len() == 8 && coh.starts_with("R0") && coh[2..].bytes().all(|b| b.is_ascii_digit());
            match coh.len() {
                8 if digits || letters_then_six || r_series => {
                    CanonKey::e1("GB:coh", coh).map(|k| CanonKey { folded, ..k })
                }
                // `gb_coh_fold` reads 8-character bodies only, so a pad is
                // never folded.
                6 | 7 if digits => CanonKey::e2("GB:coh", format!("{:0>8}", coh)),
                // Every other `GB<SCHEME>…` — UKPRN, CHC, NHS, MPR — is a
                // register of its own that this arm deliberately does not
                // know: E0 exact equality only, per the FTS census where they
                // total 34 rows against GB-PPON's 5,606.
                _ => None,
            }
        }
        // DE, AT, IE, LU, CY, MT, EE, LT, and everything unlisted: no
        // cross-walk. E0 exact equality is the only merge path.
        _ => None,
    }
}

/// Issue 470: the O/0 lookalike fold of an 8-character Companies House body
/// (prefix already stripped), or `None` when nothing folds. The register format
/// itself says which character was meant:
/// - positions 3–8 are digits in every shape, so a letter O there is a zero
///   (`SCO55775` → `SC055775`, `9694399O` → `96943990`);
/// - at positions 1–2 a letter O is a zero only when the other character there
///   is a digit or another O (`O6611251` → `06611251`, `OO688424` →
///   `00688424`). `OC` (LLP), `SO` (Scottish LLP) and `OE` (overseas entity)
///   have a letter beside their O and are never folded;
/// - a digit 0 becomes an O only in a `0C` head, giving `OC`, the LLP prefix
///   (all five measured were LLPs by name). `R0` is a real series
///   (`R0000568`), so no other zero is ever read as a letter.
///
/// A fold whose result is not register-shaped keys nothing, as before. The
/// key it yields is marked [`CanonKey::folded`], and only a name gate acts on
/// it: the fold proposes the number, it never proves the entity.
fn gb_coh_fold(coh: &str) -> Option<String> {
    let orig = coh.as_bytes();
    if orig.len() != 8 {
        return None;
    }
    let mut out = orig.to_vec();
    for c in &mut out[2..] {
        if *c == b'O' {
            *c = b'0';
        }
    }
    let zero_like = |c: u8| c.is_ascii_digit() || c == b'O';
    if orig[0] == b'O' && zero_like(orig[1]) {
        out[0] = b'0';
    }
    if orig[1] == b'O' && zero_like(orig[0]) {
        out[1] = b'0';
    }
    if orig[0] == b'0' && orig[1] == b'C' {
        out[0] = b'O';
    }
    (out != orig).then(|| String::from_utf8(out).expect("an ASCII body folds to ASCII"))
}

#[cfg(test)]
mod tests {
    /// Issue 358: a national id under a regional code keys exactly as it
    /// would under the register's own code; a code with a register of its
    /// own has no arm and keys nothing.
    #[test]
    fn a_regional_code_keys_in_its_registers_series() {
        use super::canonical_key;
        for code in ["GP", "MQ", "GF", "RE", "YT", "PM", "BL", "MF", "WF"] {
            assert_eq!(
                canonical_key(Some(code), "national", "552081317"),
                canonical_key(Some("FR"), "national", "552081317"),
                "{code} keys as FR"
            );
        }
        assert!(canonical_key(Some("RE"), "national", "552081317").is_some());
        assert_eq!(
            canonical_key(Some("AX"), "national", "0100315-8"),
            canonical_key(Some("FI"), "national", "0100315-8")
        );
        assert_eq!(
            canonical_key(Some("SJ"), "national", "923609016"),
            canonical_key(Some("NO"), "national", "923609016")
        );
        assert!(canonical_key(Some("NC"), "national", "552081317").is_none(), "NC has no arm");
    }

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

    /// Companies House is 8 characters in two shapes, both E1; a 6-7 digit
    /// body is a lost leading zero and pads, so it is E2 like CZ and BE.
    #[test]
    fn gb_coh_pads_to_eight_at_e2_and_keys_full_forms_at_e1() {
        // The two full forms. `SC123456` is the shape that makes this the
        // first alphanumeric arm in the walk.
        assert_eq!(
            key(Some("GB"), "national", "GB-COH-SC305103"),
            Some(("GB:coh".into(), "SC305103".into(), Tier::E1))
        );
        assert_eq!(
            key(Some("GB"), "national", "GB-COH-07495895"),
            Some(("GB:coh".into(), "07495895".into(), Tier::E1))
        );
        // Prefixed and bare reach the same key — the strip deletes
        // information rather than manufacturing it, so it stays E1.
        assert_eq!(
            key(Some("GB"), "national", "07495895"),
            key(Some("GB"), "national", "GB-COH-07495895")
        );
        // The pad DOES collide, which is exactly why it may not auto-merge.
        let padded = key(Some("GB"), "national", "GB-COH-495895").unwrap();
        let real = key(Some("GB"), "national", "GB-COH-00495895").unwrap();
        assert_eq!(padded.1, real.1, "the pad collides — that is the danger");
        assert_eq!((padded.2, real.2), (Tier::E2, Tier::E1));
        // A letter-prefixed number is 8 characters or it is nothing: no pad
        // can invent which of the nine prefixes was lost.
        assert_eq!(key(Some("GB"), "national", "GB-COH-SC3051"), None);
    }

    /// PPON is Find a Tender's own supplier-registration series. Unifying it
    /// with a company number would be the merge
    /// `platform_guids_are_never_a_merge_key` exists to refuse.
    #[test]
    fn gb_ppon_is_its_own_series() {
        let ppon = key(Some("GB"), "national", "GB-PPON-PBZB-4962-TVLR").unwrap();
        assert_eq!(ppon, ("GB:ppon".to_owned(), "PBZB4962TVLR".to_owned(), Tier::E1));
        // Every PPON in the FTS census matches `^[A-Z]{4}-[0-9]{4}-[A-Z]{4}$`,
        // so 12 alphanumerics after the prefix; anything else keys nothing.
        assert_eq!(key(Some("GB"), "national", "GB-PPON-PBZB-4962"), None);
        // A company number and a PPON never share a scheme, so no pair of
        // them can ever unify however their characters line up.
        let coh = key(Some("GB"), "national", "GB-COH-SC305103").unwrap();
        assert_ne!(ppon.0, coh.0, "PPON and COH are separate registers");
    }

    /// The registers this arm deliberately does not know, and the GB VAT that
    /// arrives in the same arm. `scheme.starts_with(country)` cannot catch the
    /// VAT — `"GB:coh".starts_with("GB")` is true — so the arm must.
    #[test]
    fn gb_other_registers_and_vats_key_nothing() {
        for other in ["GB-UKPRN-10007789", "GB-CHC-1089464", "GB-NHS-RJ100", "GB-MPR-12345"] {
            assert_eq!(key(Some("GB"), "national", other), None, "{other} is its own register");
        }
        // Issue 514 unit 2: the register-spelling rewrite's outputs keep their token, so
        // none becomes a company key (a bare 6-7 digit charity number would pad to E2).
        for canonical in ["GBUKPRN10007798", "GBCHC216250", "GBCHC1088835", "GBNICNIC102575", "GBMPR16337R", "GBMPR30938R"] {
            assert_eq!(key(Some("GB"), "national", canonical), None, "{canonical} is its own register");
        }
        // A real GB VAT: nine digits after the prefix, `kind = "vat"`.
        assert_eq!(key(Some("GB"), "vat", "GB553298332"), None);
        // Northern Ireland keeps its own code and never folds to GB, so it
        // does not reach this arm at all.
        assert_eq!(key(Some("XI"), "national", "GB-COH-SC305103"), None);
    }

    /// Issue 470: a letter O where the register format has a digit (or a zero
    /// for the O of `OC`) keys to the number meant, MARKED folded so only a
    /// name gate acts on it. Real prefixes with an O beside a letter never
    /// fold, the `R0` series keys as itself, and a trailing-O rotation with no
    /// twin gets a folded key that binds nothing on its own.
    #[test]
    fn gb_coh_folds_an_o_where_the_register_format_has_a_digit() {
        let gb = |v: &str| {
            canonical_key(Some("GB"), "national", v).map(|k| (k.scheme, k.key, k.tier, k.folded))
        };
        let folded = |n: &str| Some(("GB:coh", n.to_owned(), Tier::E1, true));
        let unfolded = |n: &str| Some(("GB:coh", n.to_owned(), Tier::E1, false));
        // The measured lookalikes, bare and prefixed.
        assert_eq!(gb("SCO55775"), folded("SC055775"), "an O at positions 3–8 is a zero");
        assert_eq!(gb("GB-COH-SCO55775"), folded("SC055775"));
        assert_eq!(gb("O6611251"), folded("06611251"), "an O beside a digit at position 1");
        assert_eq!(gb("OO688424"), folded("00688424"), "an O beside another O");
        assert_eq!(gb("GBCOHO2O84294"), folded("02084294"), "both rules in one body");
        assert_eq!(gb("0C301540"), folded("OC301540"), "a zero for the O of `OC`");
        assert_eq!(gb("GB-COH-0C429964"), folded("OC429964"));
        // The register's own spellings key as themselves, unflagged —
        // including the real prefixes that carry an O beside a letter.
        for real in ["SC055775", "OC301540", "SO300123", "OE012345", "06611251"] {
            assert_eq!(gb(real), unfolded(real), "{real} is the register's own spelling");
        }
        // The `R0` series is a register number: it keys E1 as itself, and its
        // zero is never read as an O (`RO…` would be another number).
        assert_eq!(gb("R0000568"), unfolded("R0000568"));
        assert_eq!(gb("GB-COH-R0000273"), gb("R0000273"), "prefixed and bare share the key");
        // A zero beside any other letter is not the `OC` lookalike, and a
        // letter-then-seven-digits body other than `R0` stays unkeyed — `R`
        // before seven other digits included, bare or behind the ISO-3 `GBR`
        // (issue 470 review: the decision keyed the `R0` series only).
        assert_eq!(gb("I0097973"), None);
        assert_eq!(gb("GBCOHN0790518"), None);
        assert_eq!(gb("R1234567"), None);
        assert_eq!(gb("GBR1234567"), None);
        assert_eq!(gb("GBCOHR9876543"), None);
        // A fold onto a number the v2 gate condemns keys nothing: the literal
        // passes the gate (too few digits for a run, letters for the stub
        // rule), its fold would not (issue 470 review).
        assert_eq!(gb("OOOOOO12"), None, "folds to the zero-padded stub 00000012");
        assert_eq!(gb("SCOOOOO1"), None, "folds to SC000001, a repeated-digit run");
        assert!(crate::idgate::condemns(Some("GB"), "national", "00000012"));
        assert!(crate::idgate::condemns(Some("GB"), "national", "SC000001"));
        assert!(!crate::idgate::condemns(Some("GB"), "national", "OOOOOO12"));
        assert!(!crate::idgate::condemns(Some("GB"), "national", "SCOOOOO1"));
        // A trailing O folds like any other position, but its key is FLAGGED:
        // with no standing twin it binds nothing new, because every bind and
        // merge of a folded key goes through a name gate (the decision's
        // refinement 2), so the rotation stays inert rather than becoming a
        // wrong number.
        assert_eq!(gb("9694399O"), folded("96943990"));
        // A fold that does not reach a register shape keys nothing, and a pad
        // is never folded.
        assert_eq!(gb("SCO5577X"), None);
        assert_eq!(gb("O661125").map(|k| k.3), None);
        // The flat form the store's slots take carries the flag.
        assert_eq!(
            canonical_key_flat(Some("GB"), "national", "SCO55775"),
            Some(("GB:coh", "SC055775".to_owned(), true, true))
        );
        assert_eq!(
            canonical_key_flat(Some("GB"), "national", "SC055775"),
            Some(("GB:coh", "SC055775".to_owned(), true, false))
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
/// rule slots take: `(scheme, key, is_e1, folded)`. One definition, used by the
/// resolver's Stage-2 prevention hook and the R2 merge job alike — the two
/// MUST share one crosswalk, or prevention and repair drift apart. `folded`
/// is [`CanonKey::folded`] (issue 470): every slot that binds or merges on the
/// key reads it, so a lookalike's key reaches nobody name-blind.
pub fn canonical_key_flat(
    country: Option<&str>,
    kind: &str,
    value: &str,
) -> Option<(&'static str, String, bool, bool)> {
    canonical_key(country, kind, value).map(|ck| (ck.scheme, ck.key, ck.tier == Tier::E1, ck.folded))
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
) -> Option<(&'static str, String, bool, bool)> {
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
    Some(("E0", format!("{kind}:{norm}"), true, false))
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
/// `n2v2`: issue 346 — Greek tonos/dialytika and final-sigma fold in N2
/// (2026-09-04); every key built under `n2v1` restarts from zero.
pub const NAME_KEY_EPOCH: &str = "n2v2+n3v1";

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
            Some(("E0", "national:1000E009610001".to_owned(), true, false))
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

/// Issue 448: a mention's raw identifier keyed the way the resolver keys it —
/// [`crate::project::normalise_identifier`] under the mention's country (the v2
/// gate included: a condemned value keys nothing), then [`canonical_key_flat`]
/// on the kind the normaliser decided. The altid arm's evidence wall reads every
/// mention of both orgs through this, and [`altid_pair_key`] is built on it.
///
/// The R2/R3 walls infer the kind from the raw string instead — two leading
/// letters are VAT-shaped — which sends every FTS raw (`GB-COH-…`, `GB-PPON-…`)
/// to kind `vat`, and the GB arm keys no VAT: through that rule a wall is blind
/// to exactly the values the altid arm is about. The normaliser tells a GB VAT
/// (nine digits after the prefix) from `GBCOH…` by the letter run after the
/// country code.
pub fn mention_key(country: Option<&str>, raw: &str) -> Option<(&'static str, String, bool, bool)> {
    let id = crate::project::normalise_identifier(raw, country)?;
    canonical_key_flat(id.country.as_deref(), &id.kind, &id.value)
}

/// Issue 448: one side of the Companies House ↔ PPON pairing an FTS party
/// publishes — ONE `BT-501-Organization-Company` row `(notice_ids.scheme,
/// notice_ids.value, the party's country)` in the flat `(scheme, key, is_e1,
/// folded)` shape, or `None`.
///
/// The pairing itself is E2 evidence, never a key (the GB arm's contract in
/// [`canonical_key`]); this names its two ends so the altid arm can build the
/// graph. It keys ONLY a row whose published scheme is exactly `GB-COH` or
/// `GB-PPON`, and only into that scheme's own series. The value alone cannot
/// say which register it is: the GB arm strips a bare `GB` and keys any
/// 8-digit remainder as a company number, so a charity, UKPRN or NHS code
/// published under its own scheme would otherwise arrive as somebody's
/// Companies House number.
///
/// A 6-7 digit company number comes back padded at `is_e1 = false` (the CZ/BE
/// pad precedent); the arm counts it and never pairs it.
pub fn altid_pair_key(
    scheme: &str,
    value: &str,
    country: Option<&str>,
) -> Option<(&'static str, String, bool, bool)> {
    let series = match scheme {
        "GB-COH" => "GB:coh",
        "GB-PPON" => "GB:ppon",
        _ => return None,
    };
    mention_key(country, value).filter(|(s, _, _, _)| *s == series)
}

/// Issue 469 U3: the GB registers other than Companies House that the altid arm pairs
/// with a PPON — publisher scheme, series, the identity's scheme token, and the shape
/// the number after the token must have.
///
/// The shapes are frozen from the registers themselves (the bulk extracts read on
/// 2026-10-11, `.scratch/tender-db/469-registry-pairs/u2/u2-results.md`), never
/// stripped or repaired, because a wrong merge has no undo:
/// - UKPRN: 8 digits starting `1` (every UKPRN in GIAS); `10000018991` (11) fails.
/// - CHC: 6 digits starting `2`–`9` (200000–900642) or 7 starting `1` (1000000–
///   1220015, the extract's ranges). A linked charity's glued `-1` fails both, and so
///   does a company number missing its zero (`2204085`).
/// - MPR (the FCA Mutuals register): a 2014-Act society's own number is BARE, four
///   digits (`7522`, Thirteen Housing Group); the old Acts' carry `R` (`30938R`); a few
///   carry `RS` (`2571RS`); Companies House mirrors the 2014 Act as `RS` + six digits
///   (`RS007639`). The suffix is identity — `7522R` is a different society — so none
///   is folded. Every other FCA suffix (friendly societies, credit unions, `IP`) fails.
/// - SC (OSCR): `SC0` + five digits. Without its `SC` the value mints a VAT
///   (`GBSC012345`: a two-letter run) and the national-kind test refuses it.
/// - NIC (CCNI): `1` + five digits, bare or under the `NIC` / `NI` spellings
///   publishers use (`NIC 100095`, `NI 100770`); three spellings are three identities,
///   which is issue 514's.
///
/// Issue 514 unit 2: the shapes are still never stripped HERE. The live normaliser
/// (`project::register_spelling`) rewrites a closed set of label spellings
/// (`GB-UKPRN-UKPRN 10007798`, `GB-CHC-Registered Charity No. 1088835`, `GB-MPR-IP16337R`)
/// into the scheme token plus the number, and only when the number after the token
/// fails its shape on input and passes it on output (invariant I1, through
/// [`registry_number_shaped`], so the two read one table). The keys below therefore see
/// the canonical identity of a label spelling, and every value 469's arm refused for a
/// broken number (`ip6243r`, a glued `-1`, a dropped zero) stays refused. The three NIC
/// spellings stay three identities (owner ruling 2).
const REGISTRIES: [(&str, &str, &str, fn(&str) -> bool); 5] = [
    ("GB-UKPRN", "GB:ukprn", "GBUKPRN", |n| n.len() == 8 && n.starts_with('1') && all_digits(n)),
    ("GB-CHC", "GB:chc", "GBCHC", |n| {
        all_digits(n) && ((n.len() == 6 && n.as_bytes()[0] >= b'2') || (n.len() == 7 && n.starts_with('1')))
    }),
    ("GB-MPR", "GB:mpr", "GBMPR", |n| {
        let lead = |d: &str, lens: std::ops::RangeInclusive<usize>| {
            lens.contains(&d.len()) && !d.starts_with('0') && all_digits(d)
        };
        lead(n, 4..=4)
            || n.strip_suffix("RS").is_some_and(|d| lead(d, 3..=4))
            || n.strip_suffix('R').is_some_and(|d| lead(d, 3..=5))
            || n.strip_prefix("RS").is_some_and(|d| d.len() == 6 && all_digits(d))
    }),
    ("GB-SC", "GB:sc", "GBSC", |n| n.len() == 8 && n.starts_with("SC0") && all_digits(&n[2..])),
    ("GB-NIC", "GB:nic", "GBNIC", |n| {
        let d = n.strip_prefix("NIC").or_else(|| n.strip_prefix("NI")).unwrap_or(n);
        d.len() == 6 && d.starts_with('1') && all_digits(d)
    }),
];

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// Issue 514 unit 2: whether `number` — the characters after a register's scheme token
/// (`GBUKPRN`, `GBCHC`, `GBMPR`, `GBSC`, `GBNIC`) — has that register's [`REGISTRIES`]
/// shape. `false` for an unknown token. The normaliser's register-spelling rewrite reads
/// its invariant I1 through this, so the rewrite and the register keys share one table.
pub(crate) fn registry_number_shaped(token: &str, number: &str) -> bool {
    REGISTRIES.iter().any(|(_, _, t, shape)| *t == token && shape(number))
}

/// Issue 469 U3: a mention's raw identifier as a GB register identity the altid arm's
/// register channel reads — `(series, identity)`, the identity exactly as the resolver
/// mints it (`GBCHC1044521`), or `None`.
///
/// [`crate::project::normalise_identifier`] decides (the v2 gate included), and the
/// identity must be a GB `national` one whose scheme token leads it and whose number
/// has the register's shape ([`REGISTRIES`]). The evidence wall reads both orgs'
/// mentions through this, so two different UKPRNs are two keys — [`mention_key`]'s GB
/// arm keys no register at all (`gb_other_registers_and_vats_key_nothing`).
pub fn registry_mention_key(country: Option<&str>, raw: &str) -> Option<(&'static str, String)> {
    let id = crate::project::normalise_identifier(raw, country)?;
    if id.kind != "national" || id.country.as_deref().map(store::register_jurisdiction) != Some("GB") {
        return None;
    }
    // `GBSC` is a prefix of no other token, and no token is a prefix of another, so the
    // first match is the only one.
    REGISTRIES.iter().find_map(|(_, series, token, shape)| {
        id.value.strip_prefix(token).filter(|n| shape(n)).map(|_| (*series, id.value.clone()))
    })
}

/// Issue 469 U3: one register side of a register ↔ PPON pairing an FTS party publishes
/// — the publisher's scheme and literal (`GB-CHC`, `GB-CHC-1044521`) and the party's
/// country — as `(series, identity)`, or `None`.
///
/// Like [`altid_pair_key`], the publisher's scheme decides: it keys only a row whose
/// scheme is one of the five registers, and only into that register's own series, so a
/// society number published as `GB-CHC` (`GB-CHC-17434R`) is refused, not re-routed.
/// The pairing is E2 evidence, never a key.
pub fn registry_pair_key(scheme: &str, literal: &str, country: Option<&str>) -> Option<(&'static str, String)> {
    let (_, series, _, _) = REGISTRIES.iter().find(|(s, _, _, _)| *s == scheme)?;
    registry_mention_key(country, literal).filter(|(s, _)| s == series)
}

/// Issue 448: the altid arm's corroboration key — [`n3_key`], then the GB legal
/// forms folded to `§` markers (`ltd`/`limited` → `§ltd`, `plc` → `§plc`,
/// `llp` → `§llp`, `lp` → `§lp`, `cic` → `§cic`), then `the` and `and`
/// dropped. Two orgs corroborate a company-number/PPON pair only when some name
/// of each yields keys that [`altid_keys_agree`].
///
/// Deliberately strict, and deliberately here rather than in the shared tables.
/// `uk`, `group` and `holdings` stay: `Acme UK Ltd` and `Acme Holdings plc`
/// are the sister and parent companies a publisher writes by mistake (issue
/// 447), and a core-token rule that drops two-letter tokens reads `Acme UK Ltd`
/// as `Acme Ltd`. The forms stay as markers, so `Acme plc` never equals `Acme
/// Ltd`. And none of it touches `family_token`, the store's `NAME_STOP_TOKENS` or
/// [`NAME_KEY_EPOCH`]: teaching those GB forms would change R2's name gate and
/// every N3 key in every country, where this fold is for this arm alone.
///
/// Three folds the first dry plan on prod asked for (unit 1b, dry job 1681's
/// denied listing), none of which loosens the words:
/// - a run of single letters is ONE initialism, so `U.K.`, `U K` and `UK` key
///   alike (`JEOL (U.K.) Limited` / `JEOL (UK) Ltd`, `E.P.BARRUS` / `E P Barrus`).
///   An `&` or `and` ends the run: `J & M Smith Ltd` is not `JM Smith Ltd`, and
///   Companies House registers them apart;
/// - `co` is `company` (`Mulalley & Co. Limited` / `Mulalley and Company Ltd`);
/// - a trading-as clause names the brand, not the registered entity, and a
///   parenthetical after the legal form is an annotation (`… Ltd (BDP)`,
///   `… LIMITED (01659837)`), so both are cut before keying ([`altid_trim`]).
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

/// Issue 448 unit 1b: what [`altid_name_key`] cuts before keying. A trading-as
/// clause (` t/a `, `(t/a `, ` trading as `, `(trading as `) and everything after
/// it; then a trailing parenthetical that follows a GB legal form. A
/// parenthetical INSIDE the name (`JEOL (UK) Ltd`, `PBS Construction (NE) Ltd`)
/// is part of it and stays.
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

/// Issue 448 unit 1b: whether two [`altid_name_key`] outputs name one entity —
/// equal, or equal but for a GB legal form only ONE of them carries (`Carnall
/// Farrar Ltd` / `Carnall Farrar`, 105 of dry job 1681's 341 overlap denials).
/// Two DIFFERENT forms stay apart (`Acme plc` / `Acme Ltd`), and the words must
/// match exactly, so `Acme` never agrees with `Acme UK Ltd`. Only the five GB
/// markers this key adds are a form here: [`n3_key`]'s own `§` families (`§ag`,
/// `§gmbh`, `§spa`…) are words, so `Siemens Healthineers AG` never agrees with a
/// formless `Siemens Healthineers`, and `Acme Spa` is not `Acme`.
pub fn altid_keys_agree(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let strip = |key: &str| {
        let mut tokens: Vec<&str> = key.split(' ').collect();
        let mut formed = false;
        while tokens.last().is_some_and(|t| matches!(*t, "§ltd" | "§plc" | "§llp" | "§lp" | "§cic")) {
            tokens.pop();
            formed = true;
        }
        (tokens.join(" "), formed)
    };
    let ((a, a_formed), (b, b_formed)) = (strip(a), strip(b));
    a_formed != b_formed && !a.is_empty() && a == b
}

/// Issue 448: the GB legal-form family a name carries — `ltd` (Ltd or
/// Limited), `plc` or `llp` — for the altid arm's head-against-head veto.
/// [`legal_form_family`] knows no GB form, so it never vetoes `Acme plc`
/// against `Acme Ltd`; this does. The LAST family token wins, because an
/// English name puts its form at the end and may start with a form word
/// (`Limited Editions plc`). The spelled-out forms (`Public Limited Company`,
/// `Public Limited`, `Limited Liability Partnership`) read as the abbreviation
/// they spell, never as their `Limited` (issue 516).
pub fn gb_legal_family(name: &str) -> Option<&'static str> {
    let norm = crate::project::match_norm(name);
    let tokens: Vec<&str> = norm.split(' ').collect();
    let mut last = None;
    let mut at = 0;
    while at < tokens.len() {
        // The spelled-out forms first: their `limited` is not a Ltd (the 469 U1 read,
        // 2026-10-11: `NATIONAL WESTMINSTER BANK PUBLIC LIMITED COMPANY` against
        // `Natwest Bank plc` and `RAPLEYS LIMITED LIABILITY PARTNERSHIP` against `Rapleys LLP`
        // read as a form conflict, and nine register-confirmed pairs could never merge).
        let (form, used) = match &tokens[at..] {
            ["public", "limited", "company", ..] => (Some("plc"), 3),
            ["public", "limited", ..] => (Some("plc"), 2),
            ["limited", "liability", "partnership", ..] => (Some("llp"), 3),
            ["ltd" | "limited", ..] => (Some("ltd"), 1),
            ["plc", ..] => (Some("plc"), 1),
            ["llp", ..] => (Some("llp"), 1),
            _ => (None, 1),
        };
        last = form.or(last);
        at += used;
    }
    last
}

// ---- Issue 524 unit 2: the head election's ingest side, injected into `store::elect_head`.

/// Issue 524 unit 2: the key an organization's mentions VOTE under when its served head is
/// re-elected — [`altid_name_key`] with every legal-form token dropped: the `§` markers (the GB
/// forms this key adds, and [`n3_key`]'s `§gmbh`/`§srl`/`§sarl`… families) and every BARE token
/// [`family_token`] knows. The bare ones come from the initialism collapse, which reads `S.R.L.`
/// as the letters `s r l` and joins them into `srl`, while `SRL` is one token [`n3_key`] marks
/// `§srl`; dropping both makes `WECO TMC`, `WECO TMC S.R.L.` and `WECO TMC SRL` one name. When
/// nothing is left (`S.R.L.` alone), the alt key itself, so no name keys empty.
///
/// Issue 524 unit 2a.1: the name is lower-cased and its Latin diacritics folded first
/// ([`fold_diacritics`]), because publishers drop accents in capitals: dry plan 2501 split
/// `Comuna Dumbrăveni` from `COMUNA DUMBRAVENI` into two clusters and served the ALL-CAPS ASCII
/// form (13731240), and made accent-only flips of `Szervíz`, `Dotación` and `Géosat`. The fold is the
/// head election's alone — this key, [`head_twin_key`], the family hold's reads and [`head_r2_hold`];
/// the merge planners' keys (`match_norm` (N2), [`n3_key`], [`altid_name_key`]) and their vetoes keep
/// their diacritics.
///
/// It groups the spellings of ONE organization — casing, punctuation, diacritics, a leading `the`,
/// Ltd/Limited/none, S.R.L./SRL/none — and so subsumes [`altid_keys_agree`] (a GB form one side
/// carries) as a plain key: no union-find, no transitivity question. It merges no organizations
/// and feeds no bind: [`altid_name_key`], [`altid_keys_agree`], [`family_token`] and
/// [`NAME_KEY_EPOCH`] stay as they are, so the altid arm, the R2/R3 vetoes and every N3 key do
/// not move.
///
/// Whitespace-invariant (issue 524 rule v6's review): the name is trimmed and every run of Unicode
/// whitespace made one ASCII space FIRST. The election serves a display spelling with its
/// whitespace collapsed, and [`altid_name_key`]'s trim matches its markers (` t/a `, ` trading as `)
/// and its trailing ` ltd`/` limited`/` plc`/` llp` byte for byte, so `Acme Ltd trading  as Foo` and
/// its served `Acme Ltd trading as Foo` keyed apart: the served head left its own cluster, S1 never
/// kept it, and every later plan proposed it again. Spellings that differ only in whitespace are now
/// one cluster too.
pub fn head_vote_key(name: &str) -> String {
    let collapsed = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let alt = altid_name_key(&fold_diacritics(&collapsed));
    let kept: Vec<&str> = alt
        .split(' ')
        .filter(|t| !t.is_empty() && !t.starts_with('§') && family_token(t).is_none())
        .collect();
    if kept.is_empty() { alt } else { kept.join(" ") }
}

/// Issue 524 unit 2a.1: a name LOWER-CASED (char by char, so a final `Σ` reads `σ`, as `match_norm`
/// reads it anyway) with its Latin diacritics folded, for [`head_vote_key`], [`head_twin_key`], the
/// head's legal-form family and [`head_r2_hold`] — what NFD followed by dropping the combining marks
/// gives, done by table because the crate carries no normalisation data:
/// - the generic combining marks (U+0300–U+036F, U+1AB0–U+1AFF, U+1DC0–U+1DFF, U+20D0–U+20FF,
///   U+FE20–U+FE2F) are dropped after a letter of ANY script, so decomposed input folds like
///   precomposed input (and Turkish `İ`, which lower-cases to `i` and a combining dot, folds to `i`).
///   Without that, `match_norm` would read each mark as a word break;
/// - the precomposed Latin letters fold to their base letter: Latin-1, Extended-A, the Extended-B
///   letters with a diacritic, and Latin Extended Additional (Vietnamese, `ṣ`, `ẓ`, …);
/// - of the letters NFD leaves whole, `ø` → `o`, `đ` → `d`, `ł` → `l` and `ı` → `i` fold as store's
///   issue-359 table (`fold_latin`, the R2 name gate and the buyer guard) folds them, and this table
///   adds `ð` → `d`, `ħ` → `h`, `ŧ` → `t` and a few more that `fold_latin` does not know. Three expand
///   to the two letters an ASCII writer uses where `fold_latin` keeps one, so the ALL-CAPS twin joins
///   the cluster: `ß` → `ss` (`STRASSENBAU` beside `Straßenbau`; its capital is `SS`), `æ` → `ae`,
///   `œ` → `oe` (and `þ` → `th`, `ĳ` → `ij`). `fold_latin` is not changed.
///
/// A precomposed letter of another script (Cyrillic `й`, Greek `ά`) is lower-cased and otherwise
/// passes through: the Greek tonos fold stays `match_norm`'s.
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

/// Latin Extended Additional's base letters, one per upper/lower pair: U+1E00–U+1E95, then
/// U+1EA0–U+1EF9 (the Vietnamese letters).
const LATIN_ADDITIONAL_LOW: &[u8; 75] = b"abbbcdddddeeeeefghhhhhiikkkllllmmmnnnnoooopprrrrsssssttttuuuuuvvwwwwwxxyzzz";
const LATIN_ADDITIONAL_HIGH: &[u8; 45] = b"aaaaaaaaaaaaeeeeeeeeiioooooooooooouuuuuuuyyyy";

/// [`fold_diacritics`]' table: the base letter(s) of a lower-case Latin letter with a diacritic.
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

/// One `&'static str` per ASCII lower-case letter, for [`fold_latin_letter`]'s byte tables.
const ASCII_LETTERS: [&str; 26] = [
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v",
    "w", "x", "y", "z",
];

/// Issue 524 unit 2: why `candidate` may not be SERVED as an organization's head in place of
/// `head` — the display-spelling filter `store::elect_head` applies inside the winning cluster —
/// or `None` for a fit spelling. The first that holds answers:
/// - `control`: a C0/C1 control character or U+FFFD (sample 99's `C\x8cUR`, 98's line break);
/// - `mojibake` (unit 2a.2): UTF-8 read as cp1252 or Latin-1 (`MÃ¼nchen`, `UniversitÃ\u{a0}degli`);
/// - `long`: more than 200 characters;
/// - `lowercase`: cased letters and no upper-case one (sample 334's `les alchimistes`);
/// - `non_name`: [`crate::partyname::not_a_name`] classes it — any class, `award_summary` too,
///   which still VOTES but is never served;
/// - `net`: issue 511's [`crate::partyname::mentions_placeholder`] or issue 510's
///   [`crate::partyname::mentions_void`] hits it and not `head`. Those walks read HEADS, so a
///   refresh must not hand them a hit the old head did not give;
/// - `consortium`: [`consortium_name`] and not `head`'s — a grouping's name would turn the
///   member's head into the vehicle's, which the altid bar and the anchor read as a veto.
pub fn head_spelling_unfit(candidate: &str, head: &str) -> Option<&'static str> {
    if candidate.chars().any(|c| c.is_control() || c == '\u{FFFD}') {
        return Some("control");
    }
    if mojibake(candidate) {
        return Some("mojibake");
    }
    if candidate.chars().count() > 200 {
        return Some("long");
    }
    let cased = candidate.chars().any(|c| c.is_lowercase() || c.is_uppercase());
    if cased && !candidate.chars().any(char::is_uppercase) {
        return Some("lowercase");
    }
    if crate::partyname::not_a_name(candidate).is_some() {
        return Some("non_name");
    }
    use crate::partyname::{mentions_placeholder, mentions_void};
    if (mentions_placeholder(candidate) && !mentions_placeholder(head))
        || (mentions_void(candidate) && !mentions_void(head))
    {
        return Some("net");
    }
    if consortium_name(candidate) && !consortium_name(head) {
        return Some("consortium");
    }
    None
}

/// Issue 524 unit 2a.2: whether a spelling shows UTF-8 read as cp1252 or Latin-1:
/// - a lead byte's letter (`Ã`, `Â`, `Ä`, `Å`) directly followed by a continuation byte's character:
///   U+0080–U+00BF (`¼`, `©`, the no-break space of `UniversitÃ\u{a0}`), or a cp1252 special that
///   byte maps to (`Œ`, `œ`, `Š`, `š`, `Ÿ`, `Ž`, `ž`, `ƒ`, `ˆ`, `˜`, U+2013–U+203A, and since unit 2a.3
///   `€` and `™`: `SOCIETÃ€`, `Ä™` for `ę`);
/// - unit 2a.3: `È` followed by `™` or `›` (`È™`, `È›` for Romanian `ș`, `ț`);
/// - unit 2a.3: `Ã` or `Â` right after a LOWER-case letter and before a space or the end
///   (`SocietÃ  Cooperativa`, where the no-break space became a space). A Portuguese word-final `ã`
///   is lower-case itself, and an ALL-CAPS `IRMÃ DULCE` has a capital before its `Ã`, so both stay fit.
///
/// Conservative otherwise: `São Paulo`, `Ålesund` and `ÄRZTE` have a letter after the lead. The 2a.1
/// review's case: an Italian word-final `à` read as `Ã` and a no-break space folded into the right
/// cluster and won the display order on its marked letters.
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

/// Issue 524 unit 2: why a re-elected head `new` is HELD for review instead of replacing `old`, or
/// `None` to let it through. An identifier-bearing organization's head is an input to the
/// resolver's and merge planners' name gates — the altid bar's consortium veto and GB head-vs-head
/// form check, the anchor's owner consortium veto, `legal_form_family` and N2 corroboration — so a
/// change that would flip one of them is listed, not applied. The first that holds answers:
/// - `family`: [`legal_form_family`] names a family on both sides, and they differ;
/// - `gb_family`: [`gb_legal_family`] names a family on both sides, and they differ (`… plc` →
///   `… Ltd`);
/// - `consortium_removed`: `old` is a [`consortium_name`] and `new` is not;
/// - `nhs_check`: the org's identifier is a GB-NHS code (`national` `GBNHS<code>`,
///   [`crate::nhs_ods::nhs_tail`]) and `new` fails [`crate::nhs_ods::check`] where `old` was
///   admitted (a `Tie` is no pass, as in `child_check`).
///
/// A family gained or lost (`None` → `spzoo`, sample 760's address tail cut to `Getinge Polska Sp.
/// z o.o.`) is allowed; the dry plan counts those moves.
pub fn head_change_hold(
    old: &str,
    new: &str,
    identifier_kind: Option<&str>,
    identifier: Option<&str>,
) -> Option<&'static str> {
    // Issue 524 unit 2a.2/2a.3: the families are read as `folded_family` reads them — on the folded
    // name, then with its single-letter runs joined — because the vote key folds and joins: `Sàrl`,
    // `S.à r.l.`, `S.à.r.l.` and `S.p.A.` are one name with `SARL` / `SPA` there, and
    // `legal_form_family` alone sees only `SARL`, so an AG -> Sàrl change passed unheld.
    // `legal_form_family` itself, and the merge vetoes, are unchanged.
    if let (Some(a), Some(b)) = (folded_family(old, legal_form_family), folded_family(new, legal_form_family))
        && a != b
    {
        return Some("family");
    }
    if let (Some(a), Some(b)) = (folded_family(old, gb_legal_family), folded_family(new, gb_legal_family))
        && a != b
    {
        return Some("gb_family");
    }
    if consortium_name(old) && !consortium_name(new) {
        return Some("consortium_removed");
    }
    if identifier_kind == Some("national")
        && let Some(code) = identifier.and_then(crate::nhs_ods::nhs_tail)
    {
        let admitted = |name: &str| crate::nhs_ods::check(code, name) == crate::nhs_ods::NameCheck::Admitted;
        if admitted(old) && !admitted(new) {
            return Some("nhs_check");
        }
    }
    None
}

/// Issue 524 unit 2a.2: why an R2 change is HELD as an INITIALISM of the head it would replace, or
/// `None`. R2 replaces a stray head (one vote) by a top cluster of only 25%, and dry plan 2501's
/// judged samples showed that minority can be a publisher's shorthand of a correct full head:
/// `Communauté de communes Terroir de Caux` → `CDC Terroir de Caux` (15075247). Store applies this to
/// R2 alone; R1's 55% majority with twice the head's votes is not second-guessed.
///
/// `initialism`: `new` has a token of 3 to 5 letters, all upper case as published and no legal form
/// or designation ([`R2_LEGAL_FORM_TOKENS`] or a [`family_token`], diacritics folded: `SRL`, `GMBH`,
/// `SCOP`, `PPH`), that is the initials of a run of the head's words — the words split on whitespace
/// and punctuation, lower-cased and folded, one initial per word, except that a word of one to three
/// letters whose initial is not the next letter may be skipped inside the run (unit 2a.3: `CDCPC` ↔
/// `Communauté de communes du Pays de Caux`); and either
/// - `new` is NOTHING BUT that token, legal forms aside (unit 2a.3: `CDC`, `CDCPC SAS`), whatever its
///   case, or
/// - `new` is not ALL-CAPS as a whole, so the token stands out of its other words.
///
/// A run does not count when `new` still spells every word whose initial it took — the token then
/// annotates the full name (`Restauration Pour Collectivités (RPC)`, `HKI - Hans Knudsen
/// Instituttet`) — nor when every such word is a single letter — the head then spells the same
/// initialism with dots (`S.A.V.E.T. SRL` → `SAVET s.r.l.`, `C.P.I. Α.Ε.` → `CPI …`).
///
/// Unit 2a.1's looser arm held 447 of 2501's 1,948 R2 changes, 4 of its 13 right samples among them,
/// so the 2a.2 review narrowed it: a 2-letter token (`EY` ↔ `Ernst & Young`) is no initialism, a legal
/// form is none even where the head's words spell it (`SRL` ↔ `Società Responsabilità Limitata`, or
/// `societate cu răspundere limitată` once `cu` is skipped), and a short word of an ALL-CAPS name with
/// other words is none (`DRAEGER ROMANIA …`). 2a.1's head-cut-to-some-of-its-words arm is gone: the
/// vote key's diacritic fold keeps its motivating case (13839658, `C 3 I - CONCEPT …` → `SAS C3i`).
pub fn head_r2_hold(old: &str, new: &str) -> Option<&'static str> {
    let old_words: Vec<String> = fold_diacritics(old)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect();
    let new_folded = fold_diacritics(new);
    let new_words: std::collections::HashSet<&str> =
        new_folded.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    let legal = |folded: &str| R2_LEGAL_FORM_TOKENS.contains(&folded) || family_token(folded).is_some();
    let candidate =
        |token: &str| (3..=5).contains(&token.chars().count()) && token.chars().all(|c| c.is_alphabetic() && c.is_uppercase());
    let abbreviates = |token: &str| {
        let folded = fold_diacritics(token);
        !legal(&folded) && initials_of_a_run(&folded.chars().collect::<Vec<_>>(), &old_words, &new_words)
    };
    // Nothing but the initialism, legal forms aside: held whatever its case.
    let named: Vec<&str> = new
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty() && !legal(&fold_diacritics(t)))
        .collect();
    if let [only] = named.as_slice()
        && candidate(only)
        && abbreviates(only)
    {
        return Some("initialism");
    }
    if all_caps(new) {
        return None;
    }
    new.split(|c: char| !c.is_alphanumeric()).any(|t| candidate(t) && abbreviates(t)).then_some("initialism")
}

/// [`head_r2_hold`]'s match: whether `token` (folded letters) is the initials of a run of `words` (the
/// head's, folded), one per word, a word of one to three letters whose initial is not the next letter
/// skipped inside the run, and the run not spelled out in `kept` (the new name's words) nor made of
/// single letters only.
fn initials_of_a_run(token: &[char], words: &[String], kept: &std::collections::HashSet<&str>) -> bool {
    (0..words.len()).any(|start| {
        let mut taken: Vec<&str> = Vec::with_capacity(token.len());
        let mut at = start;
        while taken.len() < token.len() && at < words.len() {
            let word = words[at].as_str();
            if word.starts_with(token[taken.len()]) {
                taken.push(word);
            } else if taken.is_empty() || word.chars().count() > 3 {
                break;
            }
            at += 1;
        }
        taken.len() == token.len()
            && !taken.iter().all(|w| kept.contains(w))
            && !taken.iter().all(|w| w.chars().count() == 1)
    })
}

/// Issue 524 unit 2a.2: the legal-form abbreviations and designations [`head_r2_hold`] never reads as an
/// initialism of the head's words (lower-case, diacritics folded), beside every [`family_token`]: the
/// company forms; since unit 2a.3 the 2a.2 review's list — the French cooperative, public and
/// professional forms (`sca`, `scs`, `scg`, `spl`, `snc`, `scop`, `sci`, `selarl`, `selas`, `eurl`,
/// `sasu`), the Polish trade designations a firm's own name follows (`pph`, `fuh`, `phu`; `zoz`,
/// `spzoz`, `spz` for public health care units), the Spanish temp agency `ett`, and `grsk` (an
/// intermunicipal body's own acronym the review read on 19213225).
pub const R2_LEGAL_FORM_TOKENS: &[&str] = &[
    "sa", "srl", "spa", "sas", "sarl", "ag", "bv", "nv", "ab", "as", "oy", "kg", "gmbh", "ltd", "plc", "llp", "inc",
    "sl", "slu", "sca", "scs", "scg", "spl", "snc", "scop", "sci", "selarl", "selas", "eurl", "sasu", "pph", "fuh",
    "phu", "zoz", "spzoz", "spz", "ett", "grsk",
];

/// Issue 524 unit 2a.2: whether a spelling is ALL-CAPS — an upper-case letter and no lower-case one
/// that has a one-letter capital (`ß`, `ª`, `º` have none), store's `all_caps` read the same way.
fn all_caps(name: &str) -> bool {
    let has_capital = |c: char| {
        let mut upper = c.to_uppercase();
        matches!((upper.next(), upper.next()), (Some(u), None) if u != c)
    };
    name.chars().any(char::is_uppercase) && !name.chars().any(|c| c.is_lowercase() && has_capital(c))
}

/// Issue 524 unit 2a.2: the key two spellings of one cluster are TWINS under — lower-cased,
/// diacritics folded ([`fold_diacritics`]) and every non-alphanumeric character dropped. Store's
/// display order prefers the twin with more marked letters (`Dotación y Equipamiento, S. L.` over
/// `Dotacion y Equipamiento S.L.`) and nothing else: a cluster also joins spellings that differ by a
/// trading-as clause, a parenthetical, a legal form, `the` or `&`, and those are no twins (`Acme Ltd
/// t/a Café Rouge` never beats `Acme Ltd` on its accent).
pub fn head_twin_key(name: &str) -> String {
    fold_diacritics(name).chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Issue 524 unit 2: the legal-form family a head names, for the dry plan's `family_old` /
/// `family_new` and its family-move counts — [`legal_form_family`], else [`gb_legal_family`], each read
/// as [`head_change_hold`] reads them ([`folded_family`]: `Acme Sàrl` and `Acme S.à r.l.` are `sarl`).
pub fn head_family(name: &str) -> Option<&'static str> {
    folded_family(name, legal_form_family).or_else(|| folded_family(name, gb_legal_family))
}

/// Issue 524 unit 2a.3: a family table's answer for a head as the head election reads it — on the
/// diacritic-folded name, and when that names none, on the folded name with each run of
/// single-letter words joined into one (`s a r l` → `sarl`, `s p a` → `spa`), the join
/// [`altid_name_key`] makes for the vote key. Without the second read a dotted form folds into
/// letters no family table knows (`Acme S.à r.l.` → `acme s a r l`).
fn folded_family(name: &str, read: fn(&str) -> Option<&'static str>) -> Option<&'static str> {
    let folded = fold_diacritics(name);
    read(&folded).or_else(|| {
        let mut words: Vec<String> = Vec::new();
        let mut run = String::new();
        for word in folded.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
            let mut chars = word.chars();
            if let (Some(c), None) = (chars.next(), chars.next())
                && c.is_alphabetic()
            {
                run.push(c);
                continue;
            }
            if !run.is_empty() {
                words.push(std::mem::take(&mut run));
            }
            words.push(word.to_owned());
        }
        if !run.is_empty() {
            words.push(run);
        }
        read(&words.join(" "))
    })
}

#[cfg(test)]
mod head_vote {
    use super::{
        NAME_KEY_EPOCH, altid_keys_agree, altid_name_key, family_token, fold_diacritics, head_change_hold, head_family,
        head_r2_hold, head_spelling_unfit, head_twin_key, head_vote_key,
    };

    /// Issue 524 rule v6's review: the vote key is whitespace-invariant. The election serves a display
    /// spelling with its whitespace collapsed, and `altid_name_key`'s trim matches its markers and its
    /// trailing GB legal form byte for byte, so these keyed apart from their collapsed forms: the served
    /// head left its cluster.
    #[test]
    fn the_vote_key_reads_a_spelling_and_its_collapsed_form_alike() {
        let collapse = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        for raw in [
            "Acme Ltd trading  as Foo",
            "Acme Ltd t/a\u{a0}Foo",
            "Acme\u{a0}Ltd (BDP)",
            "  Acme   GmbH ",
            "Julius Brune GmbH  Co. KG",
        ] {
            assert_ne!(collapse(raw), raw);
            assert_eq!(head_vote_key(&collapse(raw)), head_vote_key(raw), "{raw:?}");
        }
        // Not vacuous: the key it wraps reads the two forms apart.
        assert_ne!(
            altid_name_key(&fold_diacritics("Acme Ltd trading  as Foo")),
            altid_name_key(&fold_diacritics("Acme Ltd trading as Foo"))
        );
        // And the collapse cuts the trading-as clause, as the published single space does.
        assert_eq!(head_vote_key("Acme Ltd trading  as Foo"), head_vote_key("Acme Ltd trading as Foo"));
        assert_eq!(head_vote_key("Acme Ltd trading  as Foo"), head_vote_key("Acme Ltd"));
    }

    /// Issue 524 unit 2a.1: dry plan 2501's accent-only splits are one vote key each.
    #[test]
    fn a_spelling_with_its_diacritics_dropped_is_the_same_vote_key() {
        for (accented, plain) in [
            ("Comuna Dumbrăveni", "COMUNA DUMBRAVENI"),
            ("Szervíz", "Szerviz"),
            ("Dotación y Equipamiento, S. L.", "Dotacion y Equipamiento S.L."),
            ("Géosat", "Geosat"),
            ("Łódź", "Lodz"),
            ("Straßenbau NRW", "STRASSENBAU NRW"),
            ("Ærø Kommune", "AERO KOMMUNE"),
            ("Œuvre Sociale", "OEUVRE SOCIALE"),
            ("Ørsted", "Orsted"),
            ("Đakovo", "Dakovo"),
            ("Diyarbakır", "DIYARBAKIR"),
            ("İstanbul", "Istanbul"),
            ("Hà Nội", "Ha Noi"),
        ] {
            assert_eq!(head_vote_key(accented), head_vote_key(plain), "{accented} / {plain}");
        }
        assert_eq!(head_vote_key("Comuna Dumbrăveni"), "comuna dumbraveni");
        // Decomposed (NFD) input folds like precomposed input, instead of breaking the word.
        assert_eq!(head_vote_key("Ge\u{301}osat"), "geosat");
        // `ø` folds to `o`, as `fold_latin` folds it; the Danish `oe` is a spelling the key does not guess.
        assert_ne!(head_vote_key("Ærø"), head_vote_key("AEROE"));
        // Other scripts pass through; the Greek tonos fold is N2's, as before.
        assert_eq!(head_vote_key("ОБЩИНА БЕЛОВО"), "община белово");
        assert_eq!(head_vote_key("Δήμος Αθηναίων"), head_vote_key("ΔΗΜΟΣ ΑΘΗΝΑΙΩΝ"));
        // A legal form written with a diacritic is still a form once folded (`S.à r.l.`).
        assert_eq!(head_vote_key("Acme S.à r.l."), "acme");
        // The fold is the head election's alone: the altid key keeps its diacritics.
        assert_eq!(altid_name_key("Géosat"), "géosat");
        assert_ne!(altid_name_key("Géosat"), altid_name_key("Geosat"));
    }

    #[test]
    fn the_fold_follows_nfd_through_the_latin_blocks() {
        assert_eq!(fold_diacritics("àéîõüçñ ăčęğıłńőřșțůž"), "aeioucn acegilnorstuz");
        assert_eq!(fold_diacritics("ḍ ṃ ṕ ṭ ẅ ẍ ẏ ẓ"), "d m p t w x y z");
        assert_eq!(fold_diacritics("ạ ặ ẹ ể ỉ ọ ộ ợ ụ ự ỳ ỹ"), "a a e e i o o o u u y y");
        assert_eq!(fold_diacritics("ß æ œ þ ø đ ð"), "ss ae oe th o d d");
        assert_eq!(fold_diacritics("a\u{300}\u{1dc0}"), "a", "combining marks are dropped");
        assert_eq!(fold_diacritics("москва αθήνα 東京"), "москва αθήνα 東京");
        // It lower-cases as it folds, char by char.
        assert_eq!(fold_diacritics("ÀÉ STRAẞE Ærø ΣΑΣ İzmir"), "ae strasse aero σασ izmir");
    }

    /// Issue 524 unit 2a.2: R2's initialism hold, narrowed by the 2a.1 review — and what it leaves alone,
    /// now that 2a.1's truncation arm is gone.
    #[test]
    fn an_r2_initialism_of_the_head_is_held_and_nothing_else_is() {
        // 15075247: `CDC` is the initials of `Communauté de communes`.
        assert_eq!(head_r2_hold("Communauté de communes Terroir de Caux", "CDC Terroir de Caux"), Some("initialism"));
        // Diacritics are folded on both sides: `ÉDF` ↔ `Électricité de France`.
        assert_eq!(head_r2_hold("Électricité de France Renouvelables", "ÉDF Renouvelables"), Some("initialism"));
        // Two letters are too few: `EY` ↔ `Ernst & Young` is no hold.
        assert_eq!(head_r2_hold("Ernst & Young Advisory", "EY Advisory"), None);
        // A legal form is never an initialism, even where the head's words spell it.
        assert_eq!(head_r2_hold("Ernst Young Società Responsabilità Limitata", "Ernst & Young SRL"), None);
        assert_eq!(head_r2_hold("Acme Gesellschaft Mit Beschränkter Haftung", "Acme GMBH"), None);
        // An ALL-CAPS new name has no token that stands out; the same token in a mixed-case name does.
        assert_eq!(head_r2_hold("Dräger Romania Safety SRL", "DRS ROMANIA SRL"), None);
        assert_eq!(head_r2_hold("Dräger Romania Safety SRL", "DRS Romania"), Some("initialism"));
        // An initialism BESIDE the words it abbreviates replaces nothing: no hold.
        assert_eq!(head_r2_hold("Restauration pour collectivités", "Restauration Pour Collectivités (RPC)"), None);
        assert_eq!(head_r2_hold("Den Erhvervsdrivende Fond Hans Knudsen Instituttet", "HKI - Hans Knudsen Instituttet"), None);
        // The head's own dotted initialism, written without dots, abbreviates nothing either.
        assert_eq!(head_r2_hold("S.A.V.E.T. SRL CON UNICO SOCIO", "SAVET s.r.l."), None);
        // …but a run of real words beside it still counts: `SEA` is `Società Elettromeccanica Arzignanense`.
        assert_eq!(head_r2_hold("S.E.A. Società Elettromeccanica Arzignanense S.p.A.", "SEA S.p.A"), Some("initialism"));
        // The truncations 2a.1 held now change: 13839658 (whose fold keeps it an S1 in the data), a
        // head cut to its first words, samples 764/765's address and NIP strip-down, a name that grows.
        assert_eq!(head_r2_hold("C 3 I - CONCEPT EN INGENIERIE D'INFRASTRUCTURES INNOVANTES", "SAS C3i"), None);
        assert_eq!(head_r2_hold("Tribunal administratif de Nice Greffe", "Tribunal administratif de Nice"), None);
        assert_eq!(head_r2_hold("Firma Kowalski ul. Polna 1 00-001 Warszawa NIP 1234567890", "Firma Kowalski"), None);
        assert_eq!(head_r2_hold("FREGONESE", "FREGONESE ET FILS"), None);
        // No upper-case token in the new spelling at all: nothing to read as an initialism.
        assert_eq!(head_r2_hold("TRIBUNAL ADMINISTRATIF DE NICE (TA)", "Tribunal administratif de Nice"), None);
        assert_eq!(head_r2_hold("Communauté de communes Terroir de Caux", "Cdc Terroir de Caux"), None, "not upper case");
        // An upper-case token that abbreviates no run of the head's words.
        assert_eq!(head_r2_hold("Stadtwerke Bonn", "SWB Energie"), None, "s-b, not s-w-b");
        assert_eq!(head_r2_hold("Sheffield Hallam University", "University of Sheffield"), None);
    }

    /// Issue 524 unit 2a.3: a new name that is nothing but the initialism is held whatever its case,
    /// particles inside the run may be skipped, an ALL-CAPS name with other words stays exempt, and
    /// the review's designations are no initialism.
    #[test]
    fn a_bare_initialism_is_held_and_designations_are_not_initialisms() {
        let caux = "Communauté de communes du Pays de Caux";
        assert_eq!(head_r2_hold(caux, "CDC"), Some("initialism"));
        assert_eq!(head_r2_hold(caux, "CDCPC"), Some("initialism"), "`du` and `de` skipped inside the run");
        assert_eq!(head_r2_hold(caux, "CDCPC SAS"), Some("initialism"), "a legal form beside it is no other word");
        assert_eq!(head_r2_hold(caux, "CDC PAYS DE CAUX"), None, "ALL-CAPS with other words: exempt");
        assert_eq!(head_r2_hold(caux, "CDC Pays de Caux"), Some("initialism"));
        assert_eq!(head_r2_hold("European Public Law Organisation", "EPLO"), Some("initialism"));
        assert_eq!(head_r2_hold("Ernst & Young Advisory", "EY"), None, "two letters, bare or not");
        // The skip takes only short words whose initial is not the next letter: `Pays` is no particle.
        assert_eq!(head_r2_hold(caux, "CDCC"), None);
        // Designations the review listed are no initialism of the words they abbreviate.
        assert_eq!(head_r2_hold("Société publique locale aéroportuaire régionale", "SPL Aéroportuaire Régionale"), None);
        assert_eq!(head_r2_hold("Société coopérative agricole le Pré Vert", "SCA le Pré Vert"), None);
        assert_eq!(head_r2_hold("Przedsiebiorstwo Produkcyjno-Handlowe POLMAG Sp. z o.o.", "PPH POLMAG Sp. z o.o."), None);
        assert_eq!(head_r2_hold("Interim Aire Empresa Trabajo Temporal, S. L. U.", "Interim Aire ETT SLU"), None);
        // A legal form the head's words spell once a particle is skipped (`cu`): still no initialism.
        assert_eq!(head_r2_hold("Firma Societate cu Răspundere Limitată", "Firma SRL"), None);
    }

    /// Issue 524 unit 2a.2: mojibake — UTF-8 read as cp1252 or Latin-1 — is unfit; a real accented
    /// capital followed by a letter is not.
    #[test]
    fn a_mojibake_spelling_is_unfit_and_an_accented_capital_is_not() {
        let head = "Universita degli Studi";
        assert_eq!(head_spelling_unfit("UniversitÃ\u{a0}degli Studi di Napoli", head), Some("mojibake"));
        assert_eq!(head_spelling_unfit("MÃ¼nchen Stadtwerke", head), Some("mojibake"));
        assert_eq!(head_spelling_unfit("CafÃ© Rouge", head), Some("mojibake"));
        assert_eq!(head_spelling_unfit("NÂ° 1 Logistique", head), Some("mojibake"));
        assert_eq!(head_spelling_unfit("Ã‰cole Normale", head), Some("mojibake"), "a cp1252 special: ‰");
        // Unit 2a.3: the euro and trade-mark signs, Romanian `ș`/`ț` read as `È™`/`È›`, and a lead letter
        // after a lower-case letter at a word's end.
        assert_eq!(head_spelling_unfit("ATI SOCIETÃ€ COOPERATIVA", head), Some("mojibake"));
        assert_eq!(head_spelling_unfit("Leonardo SocietÃ  Cooperativa Sociale", head), Some("mojibake"));
        assert_eq!(head_spelling_unfit("Primaria Municipiului PiteÈ™ti", head), Some("mojibake"));
        assert_eq!(head_spelling_unfit("UniversitÃ", head), Some("mojibake"), "at the end");
        for fit in [
            "Università degli Studi",
            "São Paulo",
            "Ålesund",
            "Ärzte GmbH",
            "ÄRZTE",
            "SÃO PAULO",
            "ÅRHUS KOMMUNE",
            "IRMÃ DULCE",
            "Irmã Dulce",
        ] {
            assert_eq!(head_spelling_unfit(fit, head), None, "{fit}");
        }
    }

    /// Issue 524 unit 2a.2: twins are equal once lower-cased, folded and stripped of punctuation —
    /// accents and punctuation only, never a clause, a parenthetical or a legal form.
    #[test]
    fn twins_differ_by_accents_and_punctuation_only() {
        assert_eq!(head_twin_key("Dotación y Equipamiento, S. L."), head_twin_key("Dotacion y Equipamiento S.L."));
        assert_eq!(head_twin_key("Comuna Dumbrăveni"), head_twin_key("COMUNA DUMBRAVENI"));
        assert_eq!(head_twin_key("Comuna Dumbrăveni"), "comunadumbraveni");
        assert_ne!(head_twin_key("Acme Ltd"), head_twin_key("Acme Ltd t/a Café Rouge"));
        assert_ne!(head_twin_key("Acme GmbH"), head_twin_key("Acme Sàrl"));
        assert_ne!(head_twin_key("The Acme Trust"), head_twin_key("Acme Trust"));
    }

    /// Issue 524 unit 2a.2: the head's families are read on the folded names, so the `Sàrl` the vote key
    /// folds into `SARL` is a family the hold sees.
    #[test]
    fn a_family_swap_spelled_with_a_diacritic_is_held() {
        assert_eq!(head_change_hold("Acme Schweiz AG", "Acme Sàrl", None, Some("CHE123")), Some("family"));
        assert_eq!(head_family("Acme Sàrl"), Some("sarl"));
        assert_eq!(head_family("Acme Schweiz AG"), Some("ag"));
        assert_eq!(head_change_hold("Acme SARL", "Acme Sàrl", None, Some("FR1")), None, "the same family");
        // Unit 2a.3: the dotted forms fold into single letters; the second read joins them.
        assert_eq!(head_change_hold("Acme Schweiz AG", "Acme S.à r.l.", None, Some("CHE123")), Some("family"));
        assert_eq!(head_change_hold("Acme SRL", "Società Acme S.p.A.", None, Some("IT1")), Some("family"), "srl vs spa");
        assert_eq!(head_family("Acme S.à r.l."), Some("sarl"));
        assert_eq!(head_family("Acme S.à.r.l."), Some("sarl"));
        assert_eq!(head_family("Società Acme S.p.A."), Some("spa"));
        assert_eq!(head_change_hold("Acme SARL", "Acme S.à r.l.", None, Some("FR1")), None, "still the same family");
    }

    #[test]
    fn a_legal_form_dotted_bare_marked_or_absent_is_one_vote_key() {
        let weco = head_vote_key("WECO TMC");
        assert_eq!(weco, "weco tmc");
        assert_eq!(head_vote_key("WECO TMC S.R.L."), weco, "the initialism collapse's bare `srl`");
        assert_eq!(head_vote_key("WECO TMC SRL"), weco, "n3's `§srl`");
        assert_eq!(head_vote_key("The University of Sheffield"), head_vote_key("UNIVERSITY OF SHEFFIELD"));
        assert_eq!(head_vote_key("The University of Sheffield"), "university of sheffield");
        assert_eq!(head_vote_key("Arkil"), head_vote_key("Arkil Ltd"));
        assert_eq!(head_vote_key("Arkil Limited"), "arkil");
        assert_eq!(head_vote_key("SARL Fradin"), head_vote_key("FRADIN SARL"));
        assert_eq!(head_vote_key("SARL Fradin"), "fradin");
        // A different name stays apart, and the words still have to match.
        assert_ne!(head_vote_key("Sheffield Hallam University"), head_vote_key("University of Sheffield"));
        assert_ne!(head_vote_key("Acme UK Ltd"), head_vote_key("Acme Ltd"));
    }

    #[test]
    fn a_name_that_is_only_a_legal_form_keeps_its_alt_key() {
        assert_eq!(head_vote_key("S.R.L."), "srl");
        assert_eq!(head_vote_key("SRL"), "§srl");
        assert_eq!(head_vote_key("Ltd"), "§ltd");
        assert!(!head_vote_key("S.R.L.").is_empty());
    }

    /// The vote key wraps the altid key and the family table; it changes neither.
    #[test]
    fn the_keys_it_wraps_are_unchanged() {
        assert_eq!(altid_name_key("WECO TMC S.R.L."), "weco tmc srl");
        assert_eq!(altid_name_key("WECO TMC SRL"), "weco tmc §srl");
        assert_eq!(altid_name_key("The University of Sheffield"), "university of sheffield");
        assert_eq!(altid_name_key("Arkil Ltd"), "arkil §ltd");
        assert_eq!(altid_name_key("SARL Fradin"), "§sarl fradin");
        assert!(altid_keys_agree("arkil §ltd", "arkil"));
        assert!(!altid_keys_agree("weco tmc §srl", "weco tmc"), "n3's families are words to the altid arm");
        assert_eq!(family_token("srl"), Some("srl"));
        assert_eq!(family_token("sarl"), Some("sarl"));
        assert_eq!(family_token("ltd"), None, "GB forms are the altid key's, not the family table's");
        assert_eq!(NAME_KEY_EPOCH, "n2v2+n3v1");
    }

    #[test]
    fn an_unfit_spelling_is_named_for_its_first_defect() {
        let head = "Sheffield Hallam University";
        assert_eq!(head_spelling_unfit("The University of Sheffield", head), None);
        assert_eq!(head_spelling_unfit("UNIVERSITY OF SHEFFIELD", head), None, "ALL-CAPS is fit, only ranked last");
        // Sample 99's mojibake and 98's multi-line list.
        assert_eq!(head_spelling_unfit("C\u{8c}UR", head), Some("control"));
        assert_eq!(head_spelling_unfit("Alpha GmbH\nBeta GmbH", head), Some("control"));
        assert_eq!(head_spelling_unfit("Caf\u{FFFD} Nord", head), Some("control"));
        assert_eq!(head_spelling_unfit(&"Sehr Lange Firma ".repeat(13), head), Some("long"));
        assert_eq!(head_spelling_unfit(&"x".repeat(200).to_uppercase(), head), None, "200 characters is fit");
        // Sample 334.
        assert_eq!(head_spelling_unfit("les alchimistes", head), Some("lowercase"));
        assert_eq!(head_spelling_unfit("eBay", head), None);
        assert_eq!(head_spelling_unfit("ΔΗΜΟΣ ΑΘΗΝΑΙΩΝ", head), None);
        assert_eq!(head_spelling_unfit("123", head), None, "no cased letter, nothing lower-case");
        // The first defect answers: the pointer `s.o.` is all lower case before it is a non-name.
        assert_eq!(head_spelling_unfit("s.o.", head), Some("lowercase"));
        assert_eq!(head_spelling_unfit("S.O.", head), Some("non_name"));
        assert_eq!(head_spelling_unfit("-", head), Some("non_name"), "blank: no cased letter, a non-name");
        assert_eq!(head_spelling_unfit("Infructueux, marché attribué à: Berthelet", head), Some("non_name"));
        // A grouping's name never replaces a member's head; it may replace a grouping's.
        let groupement = "Groupement Colas / Barthelemy";
        assert_eq!(head_spelling_unfit(groupement, "Colas Centre Ouest"), Some("consortium"));
        assert_eq!(head_spelling_unfit(groupement, "groupement colas"), None);
    }

    /// `net`: a spelling the 510/511 walks' nets hit (they read HEADS) while `not_a_name` classes it
    /// nothing — the Scandinavian `INGEN` ("none") is a scoped word, a non-name only in its notice
    /// languages, so the language-less fold leaves it a name while `mentions_placeholder` takes it.
    /// Unfit over a head the net misses; fit over a head it already hits.
    #[test]
    fn a_spelling_only_the_nets_hit_is_unfit_over_a_head_they_miss() {
        assert_eq!(crate::partyname::not_a_name("INGEN"), None, "no language, no scoped word");
        assert!(crate::partyname::mentions_placeholder("INGEN"));
        assert!(!crate::partyname::mentions_placeholder("Kalmar kommun"));
        assert_eq!(head_spelling_unfit("INGEN", "Kalmar kommun"), Some("net"));
        assert_eq!(head_spelling_unfit("INGEN", "Ingen"), None, "the head is a hit already");
    }

    #[test]
    fn a_change_that_moves_a_bind_input_is_held() {
        // plc -> Ltd: a GB form swap, the altid arm's head-vs-head veto.
        assert_eq!(head_change_hold("Acme Holdings plc", "Acme Services Ltd", None, Some("GB1")), Some("gb_family"));
        assert_eq!(head_change_hold("Siemens AG", "Siemens GmbH", None, Some("DE1")), Some("family"));
        // Sample 760: an address-tailed head gains `spzoo` — allowed, and counted by the plan.
        let old = "Getinge Polska ul. Ostrobramska 101 04-041 Warszawa NIP 5272468405";
        let new = "Getinge Polska Sp. z o.o.";
        assert_eq!(head_change_hold(old, new, Some("national"), Some("PL5272468405")), None);
        assert_eq!((head_family(old), head_family(new)), (None, Some("spzoo")));
        assert_eq!(head_change_hold("Arkil Ltd", "Arkil", None, Some("GB2")), None, "a form dropped is no swap");
        assert_eq!(
            head_change_hold("Groupement Colas / Barthelemy", "Colas Centre Ouest", None, Some("FR1")),
            Some("consortium_removed")
        );
        // GB-NHS: the old head passes the ODS name check, the new one fails it.
        let trust = "Oxford University Hospitals NHS Foundation Trust";
        assert_eq!(
            head_change_hold(trust, "University of Oxford", Some("national"), Some("GBNHSRTH")),
            Some("nhs_check")
        );
        assert_eq!(head_change_hold(trust, "Oxford University Hospitals", Some("national"), Some("GBNHSRTH")), None);
        assert_eq!(head_change_hold(trust, "University of Oxford", Some("vat"), Some("GBNHSRTH")), None, "not an ODS code");
        assert_eq!(head_change_hold("University of Oxford", trust, Some("national"), Some("GBNHSRTH")), None);
        assert_eq!(head_family("Acme Holdings plc"), Some("plc"));
        assert_eq!(head_family("Ministerstvo financí"), None);
    }
}

#[cfg(test)]
mod altid {
    use super::{
        altid_keys_agree, altid_name_key, altid_pair_key, altid_trim, canonical_key_flat, gb_legal_family,
        mention_key, n3_key, registry_mention_key, registry_pair_key,
    };

    /// The FTS literal forms, as `fts/parse.rs` stores them (`<scheme>-<id>`).
    #[test]
    fn altid_pair_key_reads_the_fts_literal_forms() {
        let gb = Some("GB");
        assert_eq!(
            altid_pair_key("GB-COH", "GB-COH-03914810", gb),
            Some(("GB:coh", "03914810".to_owned(), true, false))
        );
        assert_eq!(
            altid_pair_key("GB-COH", "GB-COH-SC305103", gb),
            Some(("GB:coh", "SC305103".to_owned(), true, false))
        );
        assert_eq!(
            altid_pair_key("GB-PPON", "GB-PPON-PHDQ-2359-NZMP", gb),
            Some(("GB:ppon", "PHDQ2359NZMP".to_owned(), true, false))
        );
        // A lost leading zero pads, at E2: counted by the arm, never paired.
        assert_eq!(
            altid_pair_key("GB-COH", "GB-COH-3914810", gb),
            Some(("GB:coh", "03914810".to_owned(), false, false))
        );
        // The hole the scheme check closes: the GB arm keys a bare 8-digit value
        // as a company number…
        assert!(canonical_key_flat(gb, "national", "12345678").is_some());
        // …so another register's number, digits only or prefixed, keys nothing.
        for scheme in ["GB-CHC", "GB-SC", "GB-UKPRN", "GB-NHS"] {
            assert_eq!(altid_pair_key(scheme, "12345678", gb), None, "{scheme}");
            assert_eq!(altid_pair_key(scheme, &format!("{scheme}-12345678"), gb), None, "{scheme}");
        }
        // A GB VAT under the company-number scheme is not a company number, and
        // a side keys only into its own series.
        assert_eq!(altid_pair_key("GB-COH", "GB553298332", gb), None);
        assert_eq!(altid_pair_key("GB-COH", "GB-PPON-PHDQ-2359-NZMP", gb), None);
        assert_eq!(altid_pair_key("GB-PPON", "GB-COH-03914810", gb), None);
    }

    /// The wall's key sees the FTS raws the two-letter-lead rule cannot.
    #[test]
    fn mention_key_reads_the_fts_raws_a_vat_lead_rule_cannot() {
        let gb = Some("GB");
        assert_eq!(mention_key(gb, "GB-COH-03914810"), Some(("GB:coh", "03914810".to_owned(), true, false)));
        assert_eq!(
            mention_key(gb, "GB-PPON-PHDQ-2359-NZMP"),
            Some(("GB:ppon", "PHDQ2359NZMP".to_owned(), true, false))
        );
        // The R2/R3 wall's inference: a two-letter lead is VAT, and GB keys no VAT.
        assert_eq!(canonical_key_flat(gb, "vat", "GB-COH-03914810"), None);
        assert_eq!(mention_key(gb, "GB553298332"), None, "a real GB VAT stays unkeyed");
    }

    #[test]
    fn altid_name_key_folds_ltd_and_limited_but_keeps_plc_llp_and_uk() {
        assert_eq!(altid_name_key("Acme Widgets Ltd"), "acme widgets §ltd");
        assert_eq!(altid_name_key("ACME WIDGETS LIMITED"), altid_name_key("Acme Widgets Ltd."));
        assert_eq!(altid_name_key("The Acme Widgets Limited"), altid_name_key("Acme Widgets Ltd"));
        assert_eq!(altid_name_key("Smith and Jones Ltd"), altid_name_key("Smith & Jones Limited"));
        assert_eq!(altid_name_key("Acme PLC"), "acme §plc");
        assert_eq!(altid_name_key("Acme LLP"), "acme §llp");
        assert_eq!(altid_name_key("Acme Housing CIC"), "acme housing §cic");
        assert_ne!(altid_name_key("Acme plc"), altid_name_key("Acme Ltd"), "a parent is not its subsidiary");
        assert_ne!(altid_name_key("Acme LLP"), altid_name_key("Acme Ltd"));
        assert_ne!(altid_name_key("Acme UK Ltd"), altid_name_key("Acme Ltd"), "uk names a sister company");
        assert_ne!(altid_name_key("Acme Group Ltd"), altid_name_key("Acme Ltd"));
        assert_ne!(altid_name_key("Acme Holdings Ltd"), altid_name_key("Acme Ltd"));
        // N3 still runs underneath: a formless or non-GB name keys as N3 does.
        assert_eq!(altid_name_key("Siemens GmbH"), n3_key("Siemens GmbH"));
    }

    /// Unit 1b's folds, each from a pair dry job 1681 denied on prod.
    #[test]
    fn altid_name_key_folds_initials_co_and_trading_as_clauses() {
        let same = |a: &str, b: &str| assert_eq!(altid_name_key(a), altid_name_key(b), "{a} / {b}");
        same("JEOL (U.K.) Limited", "JEOL (UK) Ltd");
        same("Cardinal Health U.K. 432 Limited", "Cardinal Health UK 432 Limited");
        same("E.P.BARRUS LIMITED", "E P Barrus Ltd");
        same("C.R. REYNOLDS LIMITED", "CR Reynolds Limited");
        same("MULALLEY & CO. LIMITED", "Mulalley and Company Ltd");
        same("Fenax Developments Ltd t/a FDL Contractors", "FENAX DEVELOPMENTS LIMITED");
        same("Quality Education Solutions Ltd T/A QES", "QUALITY EDUCATION SOLUTIONS LIMITED");
        same("Tremco CPG StructureCare Services Limited; (trading as StructureCare)", "TREMCO CPG STRUCTURECARE SERVICES LIMITED");
        same("Building Design Partnership Ltd (BDP)", "Building Design Partnership Limited");
        same("ISS MEDICLEAN LIMITED (01659837)", "ISS Mediclean Limited");
        assert_eq!(altid_name_key("Acme U.K. Ltd"), "acme uk §ltd");
        // A lone initial stays a word of its own, and the words still decide.
        assert_eq!(altid_name_key("A Purkiss Ltd"), "a purkiss §ltd");
        assert_ne!(altid_name_key("Acme U.K. Ltd"), altid_name_key("Acme Ltd"), "uk still names a sister");
        // A parenthetical INSIDE the name is part of it.
        assert_ne!(altid_name_key("PBS Construction (NE) Ltd"), altid_name_key("PBS Construction Ltd"));
        assert_eq!(altid_trim("PBS Construction (NE) Ltd"), "PBS Construction (NE) Ltd");
    }

    #[test]
    fn altid_keys_agree_across_a_legal_form_only_one_side_carries() {
        let agree = |a: &str, b: &str| altid_keys_agree(&altid_name_key(a), &altid_name_key(b));
        assert!(agree("Carnall Farrar Ltd", "Carnall Farrar"));
        assert!(agree("Clear Skies Software", "CLEAR SKIES SOFTWARE LIMITED"));
        assert!(agree("Acme Widgets Ltd", "ACME WIDGETS LIMITED"), "equal keys agree");
        assert!(!agree("Acme plc", "Acme Ltd"), "two different forms never agree");
        assert!(!agree("Acme", "Acme UK Ltd"), "the words must match exactly");
        assert!(!agree("Northumbrian Water Group Limited", "Northumbrian Water Ltd"));
        assert!(!agree("Ltd", "Limited Editions"), "a bare form is no name");
        assert!(!altid_keys_agree("§ltd", ""), "nothing left once the form is gone");
        // Only the GB markers are forms: n3's own families are words here.
        assert!(!agree("Siemens Healthineers AG", "Siemens Healthineers"), "a German AG is not a formless UK name");
        assert!(!agree("Acme Spa", "Acme"));
        assert!(agree("Acme Spa Ltd", "Acme Spa"), "the same name with and without Ltd");
    }

    /// The review's case: `&` and `and` end an initialism, so two registrable
    /// names stay apart, while dotted and spaced initials still join.
    #[test]
    fn an_ampersand_ends_an_initialism() {
        assert_eq!(altid_name_key("J & M Smith Ltd"), altid_name_key("J and M Smith Limited"));
        assert_ne!(altid_name_key("J & M Smith Ltd"), altid_name_key("JM Smith Ltd"));
        assert_eq!(altid_name_key("J.M. Smith Ltd"), altid_name_key("JM Smith Ltd"));
        assert_eq!(altid_name_key("Smith & Jones Ltd"), "smith jones §ltd");
    }

    #[test]
    fn gb_legal_family_separates_ltd_plc_llp() {
        assert_eq!(gb_legal_family("Acme Ltd"), Some("ltd"));
        assert_eq!(gb_legal_family("ACME LIMITED"), Some("ltd"));
        assert_eq!(gb_legal_family("Acme Holdings PLC"), Some("plc"));
        assert_eq!(gb_legal_family("Acme LLP"), Some("llp"));
        assert_eq!(gb_legal_family("Acme Group"), None);
        assert_ne!(gb_legal_family("Acme plc"), gb_legal_family("Acme Ltd"));
        assert_ne!(gb_legal_family("Acme LLP"), gb_legal_family("Acme Limited"));
        assert_eq!(gb_legal_family("Limited Editions plc"), Some("plc"), "the last form wins");
    }

    /// Issue 469 U3: each register keys its own literals, into its own series, as the
    /// identity the resolver mints — the publishers' spellings from the U2 sample.
    #[test]
    fn registry_pair_key_requires_the_scheme_token_and_a_national_kind() {
        let gb = Some("GB");
        for (scheme, literal, series, identity) in [
            ("GB-UKPRN", "GB-UKPRN-10000840", "GB:ukprn", "GBUKPRN10000840"),
            ("GB-CHC", "GB-CHC-1044521", "GB:chc", "GBCHC1044521"),
            ("GB-CHC", "GB-CHC-207043", "GB:chc", "GBCHC207043"),
            ("GB-MPR", "GB-MPR-30938R", "GB:mpr", "GBMPR30938R"),
            ("GB-MPR", "GB-MPR-21186r", "GB:mpr", "GBMPR21186R"),
            ("GB-MPR", "GB-MPR-7522", "GB:mpr", "GBMPR7522"),
            ("GB-MPR", "GB-MPR-RS007639", "GB:mpr", "GBMPRRS007639"),
            ("GB-MPR", "GB-MPR-2571RS", "GB:mpr", "GBMPR2571RS"),
            ("GB-SC", "GB-SC-SC005336", "GB:sc", "GBSCSC005336"),
            ("GB-NIC", "GB-NIC-NIC 100095", "GB:nic", "GBNICNIC100095"),
            ("GB-NIC", "GB-NIC-NI 100770", "GB:nic", "GBNICNI100770"),
        ] {
            let want = Some((series, identity.to_owned()));
            assert_eq!(registry_pair_key(scheme, literal, gb), want, "{literal}");
            assert_eq!(registry_mention_key(gb, literal), want, "{literal}: the wall reads what the pair keys");
            // The GB arm still keys none of them (issue 448's contract, untouched).
            assert_eq!(mention_key(gb, literal), None, "{literal}");
        }
        // The publisher's scheme decides: a register's literal under another scheme is
        // refused, not re-routed — `GB-CHC-17434R` is a society number.
        assert_eq!(registry_pair_key("GB-CHC", "GB-MPR-30938R", gb), None);
        assert_eq!(registry_pair_key("GB-CHC", "GB-CHC-17434R", gb), None);
        assert_eq!(registry_pair_key("GB-COH", "GB-UKPRN-10000840", gb), None);
        assert_eq!(registry_pair_key("GB-NHS", "GB-NHS-RR8", gb), None, "NHS waits for U7");
        assert_eq!(registry_pair_key("GB-PPON", "GB-PPON-PHDQ-2359-NZMP", gb), None);
        assert_eq!(registry_mention_key(gb, "GB-COH-03914810"), None);
        assert_eq!(registry_mention_key(gb, "GB-PPON-PHDQ-2359-NZMP"), None);
        // The suffix is identity: `7522` (Thirteen Housing Group) and `7522R` (an
        // allotment society) are two societies, and `RS007639` is not folded to `7639`.
        assert_ne!(registry_pair_key("GB-MPR", "GB-MPR-7522", gb), registry_pair_key("GB-MPR", "GB-MPR-7522R", gb));
        assert!(registry_pair_key("GB-MPR", "GB-MPR-7522R", gb).is_some());
        assert_ne!(registry_pair_key("GB-MPR", "GB-MPR-RS007639", gb), registry_pair_key("GB-MPR", "GB-MPR-7639", gb));
    }

    /// Issue 469 U3: the values the U2 read found under the wrong label, or broken, all
    /// fail the shape — refused, never repaired.
    #[test]
    fn mislabelled_and_broken_register_values_are_refused() {
        let gb = Some("GB");
        for (scheme, literal) in [
            // FCA society numbers published as charity numbers.
            ("GB-CHC", "GB-CHC-10433R"),
            ("GB-CHC", "GB-CHC-28258R"),
            ("GB-CHC", "GB-CHC-28687R"),
            // A company number (SLM 02204085) missing its zero, published as a charity's.
            ("GB-CHC", "GB-CHC-2204085"),
            // Eleven digits is no UKPRN.
            ("GB-UKPRN", "GB-UKPRN-10000018991"),
            ("GB-UKPRN", "GB-UKPRN-2000840"),
            // Companies House's `IP` spelling with a lost digit (Hastoe is 16243R).
            ("GB-MPR", "GB-MPR-ip6243r"),
            // Other FCA suffixes and shapes are not housing or community societies.
            ("GB-MPR", "GB-MPR-1231W"),
            ("GB-MPR", "GB-MPR-244IP"),
            ("GB-MPR", "GB-MPR-07522"),
            ("GB-MPR", "GB-MPR-75222"),
            ("GB-MPR", "GB-MPR-RS7639"),
            // A Scottish company number is not an OSCR number.
            ("GB-SC", "GB-SC-SC305103"),
            ("GB-NIC", "GB-NIC-NIC 200095"),
        ] {
            assert_eq!(registry_pair_key(scheme, literal, gb), None, "{literal}");
            assert_eq!(registry_mention_key(gb, literal), None, "{literal}");
        }
        // Not a GB party: the identity is not GB's, so no register channel.
        assert_eq!(registry_pair_key("GB-CHC", "GB-CHC-1044521", Some("IE")), None);
    }

    /// Issue 469 U3: without its `SC`, an OSCR number reads VAT-shaped (a two-letter
    /// run after the country: the decision's `kind = vat` hazard; Strathclyde's
    /// SC015263, a number the v2 gate does not condemn as a placeholder run). Whatever the
    /// normaliser makes of it — on 2026-10-11 the VAT gate condemns it and it mints
    /// nothing — it is never a `national` identity, and the register channel refuses it.
    #[test]
    fn a_scottish_charity_without_its_sc_prefix_mints_a_vat_and_is_refused() {
        let gb = Some("GB");
        let id = crate::project::normalise_identifier("GB-SC-015263", gb);
        assert!(id.as_ref().is_none_or(|id| id.kind != "national"), "{id:?}");
        assert_eq!(registry_pair_key("GB-SC", "GB-SC-015263", gb), None);
        assert_eq!(registry_mention_key(gb, "GB-SC-015263"), None);
        assert!(registry_pair_key("GB-SC", "GB-SC-SC015263", gb).is_some(), "with it, it is OSCR's");
    }

    /// Issue 469 U3: a linked charity's `-1` glues onto the number once the dashes go,
    /// and the result fails both CHC lengths.
    #[test]
    fn a_glued_linked_charity_suffix_fails_the_chc_shape() {
        let gb = Some("GB");
        assert!(registry_pair_key("GB-CHC", "GB-CHC-1077722", gb).is_some());
        assert!(registry_pair_key("GB-CHC", "GB-CHC-212808", gb).is_some());
        for linked in ["GB-CHC-1077722-1", "GB-CHC-212808-1", "GB-CHC-212808-12", "GB-CHC-1077722-01"] {
            assert_eq!(registry_pair_key("GB-CHC", linked, gb), None, "{linked}");
        }
    }

    /// Issue 469 U3 refused a field label written into the value. Issue 514 unit 2: the
    /// live normaliser now rewrites the closed label set into the token plus the number
    /// (I1: unshaped in, shaped out), so a label spelling keys as its canonical identity —
    /// the identity its bare twin keys as — through the pair key and the wall alike. A
    /// label outside the set (`reg. charity`) is still refused, never stripped.
    #[test]
    fn label_spellings_key_as_their_canonical_identity() {
        let gb = Some("GB");
        for (scheme, literal, series, identity) in [
            ("GB-UKPRN", "GB-UKPRN-UKPRN10007798", "GB:ukprn", "GBUKPRN10007798"),
            ("GB-UKPRN", "GB-UKPRN-UKPRN: 10007798", "GB:ukprn", "GBUKPRN10007798"),
            ("GB-UKPRN", "GB-UKPRN-GB-UKPRN10004116", "GB:ukprn", "GBUKPRN10004116"),
            ("GB-CHC", "GB-CHC-Charity Number 216250", "GB:chc", "GBCHC216250"),
            ("GB-CHC", "GB-CHC-CHARITYNUMBER216250", "GB:chc", "GBCHC216250"),
            ("GB-CHC", "GB-CHC-Registered Charity No. 1088835", "GB:chc", "GBCHC1088835"),
            ("GB-NIC", "GB-NIC-Charity Commission No NIC102575", "GB:nic", "GBNICNIC102575"),
            ("GB-MPR", "GB-MPR-IP16337R", "GB:mpr", "GBMPR16337R"),
            ("GB-MPR", "GB-MPR-Reg No 30938R", "GB:mpr", "GBMPR30938R"),
        ] {
            let want = Some((series, identity.to_owned()));
            assert_eq!(registry_pair_key(scheme, literal, gb), want, "{literal}");
            assert_eq!(registry_mention_key(gb, literal), want, "{literal}: the wall reads what the pair keys");
            let twin = format!("{scheme}-{}", &identity[scheme.len() - 1..]);
            assert_eq!(registry_mention_key(gb, &twin), want, "{literal}: its bare twin {twin}");
        }
        assert_eq!(registry_pair_key("GB-CHC", "GB-CHC-reg. charity 1044521", gb), None);
        assert_eq!(registry_mention_key(gb, "GB-CHC-reg. charity 1044521"), None);
        // Outside GB nothing is rewritten, so the label spelling still keys nothing.
        assert_eq!(registry_mention_key(Some("IE"), "GB-UKPRN-UKPRN10007798"), None);
    }

    /// The spelled-out forms are the abbreviations they spell, not a Ltd: the nine
    /// register-confirmed pairs the 469 U1 read found denied as a form conflict.
    #[test]
    fn spelled_out_forms_are_their_abbreviations() {
        for (spelled, short) in [
            ("NATIONAL WESTMINSTER BANK PUBLIC LIMITED COMPANY", "Natwest Bank plc"),
            ("Bayer Public Limited Company", "Bayer PLC"),
            ("3M UNITED KINGDOM PUBLIC LIMITED COMPANY (01123045)", "3M United Kingdom PLC"),
            ("BRITISH TELECOMMUNICATIONS PUBLIC LIMITED", "British Telecommunications PLC"),
            ("RAPLEYS LIMITED LIABILITY PARTNERSHIP", "Rapleys LLP"),
            ("DANIEL CONNAL LIMITED LIABILITY PARTNERSHIP", "Daniel Connal Partnership LLP"),
        ] {
            assert_eq!(gb_legal_family(spelled), gb_legal_family(short), "{spelled}");
            assert!(gb_legal_family(spelled).is_some(), "{spelled}");
        }
        assert_eq!(gb_legal_family("Acme Public Limited Company"), Some("plc"));
        assert_eq!(gb_legal_family("Acme Limited Liability Partnership"), Some("llp"));
        // …and still apart from the form they are not.
        assert_ne!(gb_legal_family("Acme Public Limited Company"), gb_legal_family("Acme Limited"));
        assert_ne!(gb_legal_family("Acme Limited Liability Partnership"), gb_legal_family("Acme Ltd"));
        // A word between them is no spelled-out form, and the last form still wins.
        assert_eq!(gb_legal_family("Public Health Limited"), Some("ltd"));
        assert_eq!(gb_legal_family("Limited Liability Partnership Services Ltd"), Some("ltd"));
        assert_eq!(gb_legal_family("Public Limited Company Registrars LLP"), Some("llp"));
    }
}
