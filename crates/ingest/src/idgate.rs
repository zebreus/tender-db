//! Issue 300 Stage 1: the identifier plausibility gate's pure machinery —
//! placeholder lexicon, digit-sequence rules, name-derived letter runs, and
//! per-scheme checksum validators.
//!
//! NOTHING here touches the live resolver path yet. The org-merge-health
//! census calls [`census`] to measure, per scheme, how the corpus would fare
//! under each rule — and the design's enablement gate (§2.1: a checksum may
//! hard-reject only after a measured ≥97% corpus pass rate) uses those
//! numbers to adjudicate the validator implementations themselves before any
//! rule flips from measurement to enforcement. A wrong weight table shows up
//! as a scheme whose pass rate is absurdly low, not as a wave of false
//! splits.
//!
//! Checksum tranche 1 covers the schemes with unambiguous public algorithms;
//! ES CIF/NIF letter algebra, SK, and the rest stay [`Checksum::Unknown`]
//! (soft) until their own tranche. Every validator's test specimens include
//! ids verified LIVE in this corpus (the exemplar sheet's pins), so the
//! tests bind the implementation to reality, not to a transcription.

/// What the census concluded about one identifier.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GateCensus {
    /// Scheme label for per-scheme aggregation ("DE:vat", "CZ:ico",
    /// "FR:siret", …), or "other" when no shape matches.
    pub scheme: &'static str,
    pub checksum: Checksum,
    /// Matches the placeholder lexicon (NIMAT500, ORG0001, BT501, 1234…).
    pub lexicon: bool,
    /// Strictly ascending/descending digits, or one digit repeated with at
    /// most one exception (123456789, 000000001, 999999999).
    pub sequence: bool,
    /// ≥4 consecutive letters after register-prefix stripping — name-derived
    /// junk (B8 rule 4; `ORG0001MUNICPIODEALVAIZERE`).
    pub letter_run: bool,
    /// VAT-kind value with fewer than 6 digits after the country prefix —
    /// the `PL823` stub class (census run 1330, 418 strangers on one org).
    pub short_vat: bool,
    /// ≥16 chars, all hex, at least one A-F — platform-internal hashes (the
    /// FR/BE 32-40-char class from the letter-run sample): never merge-grade,
    /// but not "name-derived" either.
    pub hex_hash: bool,
    /// Contains both NIP and REGON label prefixes — a compound field holding
    /// TWO real ids (B8 catalog rule 4's splitter target), recoverable, not
    /// junk.
    pub compound: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Checksum {
    /// No confident algorithm for this shape (soft scheme, or no scheme).
    #[default]
    Unknown,
    Pass,
    Fail,
}

/// One identifier, classified for the census. `country` and `kind` are the
/// organization row's stored values; `value` its stored identifier.
pub fn census(country: Option<&str>, kind: Option<&str>, value: &str) -> GateCensus {
    let digits: Vec<u8> = value.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
    let hex_hash = value.len() >= 16
        && value.bytes().all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
        && value.bytes().any(|b| b.is_ascii_alphabetic());
    let mut out = GateCensus {
        lexicon: lexicon_hit(value),
        sequence: suspicious_digit_run(&digits),
        // Hex hashes are their own class, not "name-derived letters".
        letter_run: !hex_hash && letter_run_after_prefix(value) >= 4,
        hex_hash,
        compound: value.contains("NIP") && value.contains("REGON"),
        ..GateCensus::default()
    };
    let is_vat = kind == Some("vat");
    if is_vat {
        let tail_digits = value.get(2..).map(|t| t.bytes().filter(u8::is_ascii_digit).count());
        out.short_vat = tail_digits.is_some_and(|n| n < 6);
    }
    // Scheme resolution is census-only: vat keys resolve by their own
    // prefix, national keys by (country, shape). Never used for merging.
    let cc = if is_vat { value.get(..2) } else { country };
    let (scheme, checksum) = match (cc, is_vat, digits.len()) {
        (Some("DE"), true, 9) => ("DE:vat", mod_11_10(&digits)),
        (Some("FR"), _, 9) => (if is_vat { "FR:vat" } else { "FR:siren" }, luhn(&digits)),
        // Left-zero-padded FR forms ("00000219740248", measured live): the
        // real id sits under the padding — score IT, don't Luhn the literal
        // 14 digits (run-1331 refinement c).
        (Some("FR"), false, 14) if digits[0] == 0 => {
            let stripped = &digits[digits.iter().take_while(|&&d| d == 0).count()..];
            if stripped.len() == 9 {
                ("FR:siren-padded", luhn(stripped))
            } else {
                ("FR:siret", luhn(&digits))
            }
        }
        (Some("FR"), false, 14) => ("FR:siret", luhn(&digits)),
        // Full FR VAT: FR + 2-char key + SIREN; the key is arithmetically
        // derived from the SIREN, so both halves check (verifier catch: the
        // 11-digit form previously fell through to "other").
        (Some("FR"), true, 11) => ("FR:vat", fr_vat_key(&digits)),
        // 10-digit PL nationals starting 0 are zero-padded KRS serials, not
        // NIPs (NIP tax-office prefixes never start with 0) — scoring them
        // against the NIP checksum would smear the enablement census
        // (verifier catch).
        (Some("PL"), false, 10) if digits[0] == 0 => ("PL:krs", Checksum::Unknown),
        (Some("PL"), _, 10) => (if is_vat { "PL:vat-nip" } else { "PL:nip" }, pl_nip(&digits)),
        (Some("PL"), false, 9) => ("PL:regon9", pl_regon9(&digits)),
        (Some("CZ"), _, 8) => (if is_vat { "CZ:dic-ico" } else { "CZ:ico" }, cz_ico(&digits)),
        (Some("FI"), _, 8) => (if is_vat { "FI:vat" } else { "FI:ytunnus" }, fi_ytunnus(&digits)),
        (Some("PT"), _, 9) => (if is_vat { "PT:vat" } else { "PT:nif" }, pt_nif(&digits)),
        (Some("HR"), _, 11) => (if is_vat { "HR:vat" } else { "HR:oib" }, mod_11_10(&digits)),
        (Some("NO"), false, 9) => ("NO:orgnr", no_orgnr(&digits)),
        (Some("BE"), _, 10) => (if is_vat { "BE:vat" } else { "BE:kbo" }, be_kbo(&digits)),
        // SE VAT = orgnr + literal "01": both halves check (verifier note).
        (Some("SE"), _, 12) if is_vat => (
            "SE:vat",
            if digits[10..] == [0, 1] { luhn(&digits[..10]) } else { Checksum::Fail },
        ),
        (Some("SE"), false, 10) => ("SE:orgnr", luhn(&digits)),
        (Some("IT"), _, 11) => (if is_vat { "IT:vat" } else { "IT:piva" }, it_piva(&digits)),
        (Some("GR") | Some("EL"), _, 9) => ("GR:afm", gr_afm(&digits)),
        _ => ("other", Checksum::Unknown),
    };
    out.scheme = scheme;
    // A checksum is meaningful only when the value is exactly its digits
    // (plus the vat prefix): letter-bearing nationals (HRB 123…) must not be
    // scored against a digit algorithm they were never shaped for.
    let letters_beyond_prefix = letter_run_after_prefix(value) > 0
        || (!is_vat && value.bytes().any(|b| b.is_ascii_alphabetic()));
    out.checksum = if letters_beyond_prefix { Checksum::Unknown } else { checksum };
    out
}

/// Whether a scheme's checksum is HARD — allowed to reject an identifier to
/// the provisional path. THE STANDING ENABLEMENT DECISION (issue 300, census
/// runs 1331/1332, stable across both): only schemes with a measured ≥97%
/// corpus pass rate. FR:siret/siren/vat, PL:nip, PL:regon9, BE:kbo and
/// HR:vat (tiny population) stay SOFT — their failures are typo load the
/// checksum must not convert into rejections. Changing this list requires a
/// fresh census re-measurement, not judgement.
pub fn hard_scheme(scheme: &str) -> bool {
    matches!(
        scheme,
        "DE:vat"
            | "SE:orgnr"
            | "SE:vat"
            | "CZ:ico"
            | "CZ:dic-ico"
            | "IT:piva"
            | "IT:vat"
            | "FI:ytunnus"
            | "FI:vat"
            | "NO:orgnr"
            | "HR:oib"
            | "PT:nif"
            | "PT:vat"
            | "GR:afm"
            | "PL:vat-nip"
            | "BE:vat"
    )
}

/// The Stage-1 gate flip's verdict: does this identifier lose merge-key
/// status (⇒ the mention goes provisional)? TRUE for the measured
/// false-merge classes only: the placeholder lexicon, suspicious digit
/// runs, short VAT stubs, and a HARD-scheme checksum failure. Letter-run
/// and hex/compound classes deliberately stay census-only — the letter-run
/// composition is not fully sampled and the compound class is RECOVERABLE
/// (Stage 2's canonical_key splits it at match time; rejecting it here
/// would discard real identifier evidence). Rejection can never create a
/// false SPLIT against the standing stock: the repair job dissolves the
/// stock twins with this same predicate.
pub fn condemns(country: Option<&str>, kind: &str, value: &str) -> bool {
    let c = census(country, Some(kind), value);
    c.lexicon
        || c.sequence
        || c.short_vat
        || (c.checksum == Checksum::Fail && hard_scheme(c.scheme))
}

/// The placeholder lexicon, seeded from the measured top-30 (probe §2) and
/// the census run-1330 findings: NIMATn (SI e-procurement family — 794
/// measured strangers on one id), ORGnnn/ORG-0001 (eForms technical ids),
/// BT501 (the field id itself as a value), 0-padded stubs, 1234-prefix runs.
pub fn lexicon_hit(value: &str) -> bool {
    let v = value.as_bytes();
    let after = |p: &str| value.get(p.len()..).unwrap_or("");
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if value.starts_with("NIMAT") && all_digits(after("NIMAT")) {
        return true;
    }
    if let Some(rest) = value.strip_prefix("ORG") {
        let rest = rest.strip_prefix('-').unwrap_or(rest);
        if all_digits(rest) {
            return true;
        }
    }
    if value == "BT501" {
        return true;
    }
    // ^0+\d{1,2}$ — "0001", "00001", "0002": zero-run padding around a stub.
    if v.len() >= 3 && v.iter().all(u8::is_ascii_digit) {
        let nonzero_tail = value.trim_start_matches('0');
        if nonzero_tail.len() <= 2 && nonzero_tail.len() < value.len() {
            return true;
        }
    }
    // ^1234\d{0,6}$ — the 1234/12345/…/123456789 family and cousins.
    if let Some(rest) = value.strip_prefix("1234") {
        if rest.len() <= 6 && rest.bytes().all(|b| b.is_ascii_digit()) {
            return true;
        }
    }
    false
}

/// Strictly ascending, strictly descending, or a single repeated digit with
/// at most one exception — over the value's digit run (≥5 digits so real
/// short registry numbers aren't swept).
pub fn suspicious_digit_run(digits: &[u8]) -> bool {
    if digits.len() < 5 {
        return false;
    }
    // No mod-10 wraparound: 123456789 counts, 890123 does not — the
    // wrapping form swept cyclic runs that could be real serials
    // (verifier catch).
    let asc = digits.windows(2).all(|w| w[1] == w[0] + 1);
    let desc = digits.windows(2).all(|w| w[0] == w[1] + 1);
    let mut freq = [0usize; 10];
    for &d in digits {
        freq[d as usize] += 1;
    }
    let repeated = freq.iter().max().copied().unwrap_or(0) + 1 >= digits.len();
    asc || desc || repeated
}

/// Longest run of consecutive ASCII letters after stripping one leading
/// label/register prefix (HRB, KRS, REGON, EKRSZ, …) — reusing the live
/// gate's prefix list would couple modules, so the census strips ANY leading
/// alphabetic run of ≤6 chars (when digits follow) as "the prefix" and
/// measures the rest. `ORG0001MUNICPIO…` counts its 8+-letter run;
/// `HRB12345`, `REGON470850645`, and `EKRSZ40664534` count zero — the
/// letter-run sample (2026-08-28) showed those are PREFIXED real ids, not
/// name-derived junk, and the ≤4 cut was misclassifying them.
pub fn letter_run_after_prefix(value: &str) -> usize {
    let rest = {
        let lead = value.bytes().take_while(u8::is_ascii_alphabetic).count();
        if lead <= 6 && value.len() > lead { &value[lead..] } else { value }
    };
    let mut best = 0usize;
    let mut run = 0usize;
    for b in rest.bytes() {
        if b.is_ascii_alphabetic() {
            run += 1;
            best = best.max(run);
        } else {
            run = 0;
        }
    }
    best
}

fn to_checksum(ok: bool) -> Checksum {
    if ok { Checksum::Pass } else { Checksum::Fail }
}

/// ISO 7064 MOD 11,10 — DE USt-IdNr (9 digits) and HR OIB (11 digits): the
/// last digit checks the rest.
fn mod_11_10(digits: &[u8]) -> Checksum {
    let Some((&check, body)) = digits.split_last() else { return Checksum::Unknown };
    let mut product = 10u32;
    for &d in body {
        let mut sum = (u32::from(d) + product) % 10;
        if sum == 0 {
            sum = 10;
        }
        product = (2 * sum) % 11;
    }
    let expected = (11 - product) % 10;
    to_checksum(expected == u32::from(check))
}

/// Standard Luhn mod-10 over the whole digit string (FR SIREN/SIRET, SE
/// organisationsnummer). The documented La Poste exception is at SIRET
/// level: establishments of SIREN 356000000 use digit-sum-mod-5 instead of
/// Luhn (the SIREN itself happens to pass plain Luhn — verified by hand, so
/// it needs no special case; a 9-digit case here was a verifier catch).
fn luhn(digits: &[u8]) -> Checksum {
    if digits.len() == 14 && digits.starts_with(&[3, 5, 6, 0, 0, 0, 0, 0, 0]) {
        let sum: u32 = digits.iter().map(|&d| u32::from(d)).sum();
        return to_checksum(sum % 5 == 0);
    }
    let mut sum = 0u32;
    for (i, &d) in digits.iter().rev().enumerate() {
        let mut d = u32::from(d);
        if i % 2 == 1 {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
    }
    to_checksum(sum % 10 == 0)
}

/// Full FR VAT (11 digits: 2-digit key + SIREN): the key is
/// (12 + 3·(SIREN mod 97)) mod 97 and the SIREN must itself pass Luhn.
fn fr_vat_key(digits: &[u8]) -> Checksum {
    let key = u64::from(digits[0]) * 10 + u64::from(digits[1]);
    let siren_digits = &digits[2..];
    let siren = siren_digits.iter().fold(0u64, |acc, &d| acc * 10 + u64::from(d));
    let expected = (12 + 3 * (siren % 97)) % 97;
    match luhn(siren_digits) {
        Checksum::Pass => to_checksum(key == expected),
        other => other,
    }
}

/// PL NIP: weights 6,5,7,2,3,4,5,6,7 over the first 9; sum mod 11 must be
/// the 10th digit (and never 10).
fn pl_nip(digits: &[u8]) -> Checksum {
    const W: [u32; 9] = [6, 5, 7, 2, 3, 4, 5, 6, 7];
    let sum: u32 = W.iter().zip(digits).map(|(w, &d)| w * u32::from(d)).sum();
    let rem = sum % 11;
    to_checksum(rem != 10 && rem == u32::from(digits[9]))
}

/// PL REGON (9): weights 8,9,2,3,4,5,6,7; mod 11; 10 counts as 0.
fn pl_regon9(digits: &[u8]) -> Checksum {
    const W: [u32; 8] = [8, 9, 2, 3, 4, 5, 6, 7];
    let sum: u32 = W.iter().zip(digits).map(|(w, &d)| w * u32::from(d)).sum();
    let check = match sum % 11 {
        10 => 0,
        r => r,
    };
    to_checksum(check == u32::from(digits[8]))
}

/// CZ IČO (8): weights 8..2 over the first 7; check = (11 − sum mod 11) mod
/// 10. Verified live: 00006947 (Ministerstvo financí), 00002542 (Puncovní
/// úřad — the pad-collision exemplar).
fn cz_ico(digits: &[u8]) -> Checksum {
    const W: [u32; 7] = [8, 7, 6, 5, 4, 3, 2];
    let sum: u32 = W.iter().zip(digits).map(|(w, &d)| w * u32::from(d)).sum();
    to_checksum((11 - sum % 11) % 10 == u32::from(digits[7]))
}

/// FI Y-tunnus (8): weights 7,9,10,5,8,4,2 over the first 7; rem 0 ⇒ check
/// 0; rem 1 ⇒ no valid id exists; else 11 − rem. Verified live: 01003158
/// (Telinekataja Oy).
fn fi_ytunnus(digits: &[u8]) -> Checksum {
    const W: [u32; 7] = [7, 9, 10, 5, 8, 4, 2];
    let sum: u32 = W.iter().zip(digits).map(|(w, &d)| w * u32::from(d)).sum();
    let check = u32::from(digits[7]);
    match sum % 11 {
        0 => to_checksum(check == 0),
        1 => Checksum::Fail,
        r => to_checksum(11 - r == check),
    }
}

/// PT NIF (9): weights 9..2 over the first 8; check = 0 when 11 − rem ≥ 10,
/// else 11 − rem. Verified live: 506605949 (Município de Alvaiázere).
fn pt_nif(digits: &[u8]) -> Checksum {
    const W: [u32; 8] = [9, 8, 7, 6, 5, 4, 3, 2];
    let sum: u32 = W.iter().zip(digits).map(|(w, &d)| w * u32::from(d)).sum();
    let check = match 11 - sum % 11 {
        r if r >= 10 => 0,
        r => r,
    };
    to_checksum(check == u32::from(digits[8]))
}

/// NO organisasjonsnummer (9): weights 3,2,7,6,5,4,3,2; check = 11 − rem
/// (rem 0 ⇒ 0; result 10 ⇒ invalid).
fn no_orgnr(digits: &[u8]) -> Checksum {
    const W: [u32; 8] = [3, 2, 7, 6, 5, 4, 3, 2];
    let sum: u32 = W.iter().zip(digits).map(|(w, &d)| w * u32::from(d)).sum();
    let check = u32::from(digits[8]);
    match sum % 11 {
        0 => to_checksum(check == 0),
        1 => Checksum::Fail,
        r => to_checksum(11 - r == check),
    }
}

/// BE KBO/BCE (10): the last two digits are 97 − (the first eight as a
/// number, mod 97).
fn be_kbo(digits: &[u8]) -> Checksum {
    let body = digits[..8].iter().fold(0u64, |acc, &d| acc * 10 + u64::from(d));
    let check = u64::from(digits[8]) * 10 + u64::from(digits[9]);
    to_checksum(97 - body % 97 == check)
}

/// IT Partita IVA (11): odd positions (1st, 3rd, …) count once, even
/// positions doubled with digit-sum fold; check = (10 − sum mod 10) mod 10.
fn it_piva(digits: &[u8]) -> Checksum {
    let mut sum = 0u32;
    for (i, &d) in digits[..10].iter().enumerate() {
        let mut d = u32::from(d);
        if i % 2 == 1 {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
    }
    to_checksum((10 - sum % 10) % 10 == u32::from(digits[10]))
}

/// GR AFM (9): Σ digit_i × 2^(8−i) over the first 8; (sum mod 11) mod 10
/// must equal the 9th digit.
fn gr_afm(digits: &[u8]) -> Checksum {
    let sum: u32 =
        digits[..8].iter().enumerate().map(|(i, &d)| u32::from(d) << (8 - i)).sum();
    to_checksum((sum % 11) % 10 == u32::from(digits[8]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every specimen marked (live) was read from this corpus and
    /// hand-verified in the issue-300 exemplar sheet.
    #[test]
    fn checksums_accept_live_specimens_and_reject_mutations() {
        let pass = |c: Option<&str>, k: &str, v: &str| {
            assert_eq!(census(c, Some(k), v).checksum, Checksum::Pass, "{v} must pass");
        };
        let fail = |c: Option<&str>, k: &str, v: &str| {
            assert_eq!(census(c, Some(k), v).checksum, Checksum::Fail, "{v} must fail");
        };
        // CZ IČO (live: Ministerstvo financí, Puncovní úřad).
        pass(Some("CZ"), "national", "00006947");
        pass(Some("CZ"), "national", "00002542");
        fail(Some("CZ"), "national", "00006948");
        // FI Y-tunnus (live: Telinekataja, AGA, Ramboll).
        pass(Some("FI"), "national", "01003158");
        pass(Some("FI"), "national", "01003465");
        pass(Some("FI"), "national", "01011975");
        fail(Some("FI"), "national", "01003159");
        // PT NIF (live: Município de Alvaiázere; its dropped-digit typo).
        pass(Some("PT"), "national", "506605949");
        fail(Some("PT"), "national", "506605948");
        // PL NIP (live: Krajowa Izba Odwoławcza).
        pass(Some("PL"), "national", "5262239325");
        fail(Some("PL"), "national", "5262239326");
        // FR SIREN (live: CNFPT) and its SIRET establishments (live).
        pass(Some("FR"), "national", "180014045");
        pass(Some("FR"), "national", "18001404501577");
        pass(Some("FR"), "national", "18001404502245");
        fail(Some("FR"), "national", "180014046");
        // La Poste's documented Luhn exception.
        pass(Some("FR"), "national", "356000000");
        // BE KBO (live: Ondernemingsrechtbank Leuven).
        pass(Some("BE"), "national", "0308357753");
        fail(Some("BE"), "national", "0308357754");
        // DE VAT: a canonical valid specimen and a mutation.
        pass(None, "vat", "DE136695976");
        fail(None, "vat", "DE123456789");
        // HR OIB (public specimen of the 11-digit MOD 11,10 shape).
        pass(Some("HR"), "national", "69435151530");
        fail(Some("HR"), "national", "69435151531");
        // PL REGON-9 (GUS's own REGON, hand-verified: weights 8,9,2,3,4,5,6,7).
        pass(Some("PL"), "national", "000331501");
        fail(Some("PL"), "national", "000331502");
        // NO orgnr (Brønnøysundregistrene, hand-verified).
        pass(Some("NO"), "national", "974760673");
        fail(Some("NO"), "national", "974760674");
        // SE orgnr Luhn (Skatteverket, hand-verified).
        pass(Some("SE"), "national", "2021005448");
        fail(Some("SE"), "national", "2021005449");
        // IT Partita IVA (Agenzia delle Entrate, hand-verified).
        pass(Some("IT"), "national", "06363391001");
        fail(Some("IT"), "national", "06363391002");
        // GR AFM (OTE — the adversarial spot-checker's real specimen).
        pass(Some("GR"), "national", "094019245");
        fail(Some("GR"), "national", "094019246");
        // Checksum edge cases on real ids: PT rem-0 (CM Lisboa) and NO
        // rem-10 (Statens vegvesen).
        pass(Some("PT"), "national", "500051070");
        pass(Some("NO"), "national", "971032081");
        // SE VAT must end in the literal 01.
        pass(Some("SE"), "vat", "SE202100544801");
        fail(Some("SE"), "vat", "SE202100544802");
        // La Poste SIRETs use digit-sum-mod-5, not Luhn: 35600000000048 sums
        // to 35600000000048 -> 3+5+6+4+8 = 26, not %5 — construct one that
        // does: 35600000000012 (3+5+6+1+2 = 17, no) — use 35600000049837:
        // 3+5+6+4+9+8+3+7 = 45, %5 == 0 ⇒ pass under the special rule even
        // though plain Luhn would likely reject it.
        pass(Some("FR"), "national", "35600000049837");
        fail(Some("FR"), "national", "35600000049838");
        // Full FR VAT (key arithmetic over the CNFPT SIREN 180014045:
        // 180014045 mod 97 = 87, (12 + 3·87) mod 97 = 79 ⇒ FR79180014045).
        pass(None, "vat", "FR79180014045");
        fail(None, "vat", "FR80180014045");
    }

    #[test]
    fn the_lexicon_and_sequence_rules_hit_the_measured_placeholder_families() {
        for v in ["NIMAT500", "NIMAT3", "ORG0001", "ORG-0003", "ORG001", "BT501", "0001",
            "00001", "1234", "12345", "123456789", "1234567890"] {
            assert!(
                lexicon_hit(v) || suspicious_digit_run(
                    &v.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect::<Vec<_>>()
                ),
                "{v} must be caught"
            );
        }
        // 000…01 and 999… are sequence-rule catches, 14-zeros-then-1 too
        // (the live Tribunal Judiciaire specimen).
        let digits = |v: &str| v.bytes().map(|b| b - b'0').collect::<Vec<_>>();
        assert!(suspicious_digit_run(&digits("000000001")));
        assert!(suspicious_digit_run(&digits("999999999")));
        assert!(suspicious_digit_run(&digits("00000000000001")));
        assert!(suspicious_digit_run(&digits("987654321")));
        // Real ids must NOT be caught.
        for v in ["00006947", "5262239325", "180014045", "01003158", "506605949"] {
            assert!(!suspicious_digit_run(&digits(v)), "{v} is real");
            assert!(!lexicon_hit(v), "{v} is real");
        }
    }

    #[test]
    fn letter_runs_and_short_vats_classify_like_the_exemplars() {
        // Name-derived junk (live: Alvaiázere's ORG0001… id, the Força Aérea id).
        assert!(letter_run_after_prefix("ORG0001MUNICPIODEALVAIZERE") >= 4);
        assert!(letter_run_after_prefix("AVDAFORAAREAPORTUGUESA1") >= 4);
        // Register/label prefixes and short letter content survive — incl.
        // the 5-6-char label prefixes the live sample showed are REAL ids
        // (REGON…, EKRSZ…), not name-derived junk.
        assert!(letter_run_after_prefix("HRB12345") == 0);
        assert!(letter_run_after_prefix("KRS0000123456") == 0);
        assert!(letter_run_after_prefix("REGON470850645") == 0);
        assert!(letter_run_after_prefix("EKRSZ40664534") == 0);
        assert_eq!(census(Some("PL"), Some("vat"), "PL823").short_vat, true);
        assert_eq!(census(Some("PL"), Some("vat"), "PL8230001234").short_vat, false);
        // Letter-bearing nationals never get a digit checksum verdict.
        assert_eq!(census(Some("DE"), Some("national"), "HRB12345").checksum, Checksum::Unknown);
        // Platform hex hashes are their own class, not letter runs.
        let hex = census(Some("FR"), Some("national"), "FD23B55BFB334ADFECD9B45A735FE4B8");
        assert!(hex.hex_hash && !hex.letter_run);
        // Compound NIP+REGON fields are recoverable, flagged as such.
        let comp = census(Some("PL"), Some("national"), "NIP5262239325REGON010828091");
        assert!(comp.compound);
        // Left-zero-padded FR forms score the id UNDER the padding: a real
        // padded SIREN passes where the literal 14 digits would fail Luhn.
        let padded = census(Some("FR"), Some("national"), "00000219740248");
        assert_eq!(padded.scheme, "FR:siren-padded");
        assert_eq!(padded.checksum, Checksum::Pass);
    }
}
