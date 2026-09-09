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
    /// A telephone number in the identifier slot — `t:04131153308`,
    /// `T03455141536`: a `t`/`T` (optional colon) then a 0-led 9–12-digit
    /// run. The German review chambers publish theirs consistently, so the
    /// class is a stable per-body key most of the time — and a fusion when
    /// two bodies share a switchboard (org 660: Vergabekammer Niedersachsen
    /// + the Bund's chambers, 2026-09-03 top-100 read). Census-only.
    pub phone: bool,
    /// A bare 5-digit number on a DE national row (`13754`, `13124`): no
    /// German register issues such ids, and each of the live specimens fused
    /// unrelated municipalities and associations (a platform's own record
    /// number under a raw `EU` scheme). Census-only until the weekly report
    /// sizes the class.
    ///
    /// The 4-digit half moved to [`GateCensus::bare_four_digit`] when that
    /// became a condemning rule (issue 365 unit 2); the two classes stay
    /// disjoint so the weekly report's counts remain readable.
    pub short_numeric: bool,
    /// An identifier scoped to a ROUTING DESTINATION or a REPORTING UNIT
    /// rather than to a legal person. CONDEMNING (issue 365 unit 3).
    ///
    /// A German Leitweg-ID addresses where an electronic invoice is delivered;
    /// a Berichtseinheit-ID names a statistical reporting bucket. Neither is a
    /// party, and shared-service arrangements put many bodies behind one of
    /// each — which is why `300-org-fuzzy-matching-design.md:827` already ruled
    /// that "GLN/IPA/DIR3/OIN/Leitweg are location/office/routing scoped: never
    /// merge keys".
    ///
    /// Measured corpus-wide on prod 2026-09-09. The aggregate elevation is real
    /// but unremarkable (28.9 % and 31.6 % of rows carry ≥2 distinct mention
    /// names against a 14.7 % baseline); the number that settles it is the
    /// WORST ROW: **43** distinct names on one invoice-routing address and
    /// **47** on one reporting unit. A routing address cannot be 43
    /// organizations. 766 + 611 rows, 72,502 mentions.
    ///
    /// Prefix families, not exact strings — publishers spell these many ways.
    /// `LEITWEG` covers LEITWEGID/LEITWEGEID/LEITWEGSID/LEITWEGLD/LEITWEG and
    /// `BERICHT` covers BERICHTSEINHEITID/BERICHTEINHEITID/BERICHTSID. The
    /// stragglers `LEITID`/`LEITWERTID` (3 rows) are deliberately NOT matched:
    /// stretching the prefix to reach them would start guessing.
    pub routing_scope: bool,
    /// A bare four-digit number — `2022`, `1000`, `8477`. CONDEMNING (issue
    /// 365 unit 2).
    ///
    /// `2022` and `1000` each key six unrelated bodies across six countries,
    /// and over `id <= 3000000` on prod 2026-09-09 the class ran 93 orgs with
    /// 37 (40 %) carrying ≥2 distinct mention names against a 14.7 % corpus
    /// baseline, the worst holding 71. The argument does not rest on whether
    /// some registry issues such numbers: 10,000 possible values cannot
    /// discriminate between 5.7M organizations — the same reasoning
    /// [`GateCensus::short_vat`] already applies to a short VAT tail. Five
    /// digits and up stay in; they are real short registry numbers.
    pub bare_four_digit: bool,
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
    out.phone = phone_shaped(value);
    // Prefix FAMILIES, because publishers spell these several ways (issue 365
    // unit 3): LEITWEGID/LEITWEGEID/LEITWEGSID/LEITWEGLD/LEITWEG, and
    // BERICHTSEINHEITID/BERICHTEINHEITID/BERICHTSID.
    out.routing_scope = ROUTING_SCOPE_PREFIXES.iter().any(|p| value.starts_with(p));
    out.bare_four_digit = !is_vat
        && value.len() == 4
        && value.bytes().all(|b| b.is_ascii_digit())
        // A sequence ("1234") or a zero-padded stub ("0012") is already its own
        // class, and double-counting would make the weekly report unreadable.
        && !out.sequence
        && !out.lexicon;
    out.short_numeric = !is_vat
        && country == Some("DE")
        && value.len() == 5
        && value.bytes().all(|b| b.is_ascii_digit())
        && !out.sequence
        && !out.lexicon;
    // Scheme resolution is census-only: vat keys resolve by their own
    // prefix, national keys by (country, shape). Never used for merging.
    // Issue 358: a national key under a regional code resolves in the
    // register's series — a SIREN under `RE` scores as `FR:siren`, a
    // Y-tunnus under `AX` as `FI:ytunnus` — so the census and the gate read
    // the row the way the resolver now mints it.
    let cc = if is_vat { value.get(..2) } else { country.map(store::register_jurisdiction) };
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

/// `t:04131153308` / `T03455141536`: a `t`/`T`, an optional colon, then a
/// 0-led run of 9–12 digits — a German telephone number written where an
/// identifier belongs (see [`GateCensus::phone`]).
pub fn phone_shaped(value: &str) -> bool {
    let Some(rest) = value.strip_prefix(['t', 'T']) else { return false };
    let rest = rest.strip_prefix(':').unwrap_or(rest);
    (9..=12).contains(&rest.len()) && rest.starts_with('0') && rest.bytes().all(|b| b.is_ascii_digit())
}

/// The generic-name cap: a corroboration name carried by MORE than this many
/// orgs is generic, and agreement on it is agreement nobody chose to make.
///
/// ONE constant, and it has to stay one. Three walls now measure against it —
/// the Stage-4 E3 scan's stoplist, the R3 batch merge arm (issue 316), and
/// the resolver's ingest-time anchor bind (issue 318) — and a name the scan
/// calls generic must not be a corroboration the resolver accepts. It lives
/// here, beside `hard_scheme`, because the two are read together everywhere
/// the wall appears.
pub const STOPLIST_CAP: usize = 20;

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
/// runs, short VAT stubs, PHONE NUMBERS (issue 365 unit 1), and a
/// HARD-scheme checksum failure. Letter-run and hex/compound classes
/// deliberately stay census-only — the letter-run composition is not fully
/// sampled (issue 365 unit 3 owes that read) and the compound class is
/// RECOVERABLE (Stage 2's canonical_key splits it at match time; rejecting
/// it here would discard real identifier evidence). Rejection can never
/// create a false SPLIT against the standing stock: the repair job
/// dissolves the stock twins with this same predicate.
///
/// THE STANDARD FOR ADDING A CLASS HERE — read this before wiring the next
/// one, because it was got wrong once and the wrong version is persuasive.
///
/// Measure **per VALUE**: group the published identifier values and count the
/// distinct organization names published against each. Do NOT measure per ORG
/// ("orgs that carry a mention of this class"), which is what the `OTROS`
/// scheme denial used on 2026-09-09 before being reverted the same day. An org
/// in such a class is usually reached by hundreds of other mentions, so that
/// statistic attributes a large buyer's whole name spread to whichever class
/// happens to appear among them — guilt by association. It read as 36.9 %
/// against a 14.7 % baseline for a class that is in fact BELOW baseline.
///
/// The per-value yardstick, prod 2026-09-09 (`notice_id > 30000000`, every
/// identifier value): **31,161 values, 5.8 % spanning >=2 distinct names,
/// worst 93.** That 5.8 % is what a candidate has to beat.
///
/// | class | values | >=2 names | worst |
/// | --- | --- | --- | --- |
/// | baseline | 31,161 | 5.8 % | 93 |
/// | `phone` (condemned) | 4,245 | **13.1 %** | **254** |
/// | `OTROS` (reverted) | 887 | 1.8 % | 11 |
///
/// `phone` was census-only on the reading in its own field doc — the review
/// chambers publish a switchboard consistently, so it keys a body more often
/// than it fuses two. It does not: 13.1 % of phone-shaped values carry two or
/// more distinct names, and the worst single value carries **254**. A telephone
/// number cannot be 254 organizations.
///
/// Note WHY the obvious metric misses this: after a fusion the bad key still
/// holds exactly one org row, so rows-per-distinct-value — the measure that
/// correctly spared the hex class in issue 312 — reads 1.0 and looks clean.
/// Names per VALUE is what exposes it.
pub fn condemns(country: Option<&str>, kind: &str, value: &str) -> bool {
    let c = census(country, Some(kind), value);
    c.lexicon
        || c.sequence
        || c.short_vat
        || c.phone
        || c.bare_four_digit
        || c.routing_scope
        || (c.checksum == Checksum::Fail && hard_scheme(c.scheme))
}

/// A version-4 UUID sitting in an identifier field: a submission
/// platform's own record key, leaked into the id slot (dashed, or the
/// undashed 32-hex form the corpus stores).
///
/// DELIBERATELY NOT part of [`condemns`] — issue 312 measured the class
/// before building the obvious gate and the conclusion reversed: 75,555
/// org rows carry one across 75,548 DISTINCT values, so the class merges
/// almost nothing falsely (which is all `condemns` prevents), while 93% of
/// a reviewed sample's GUID orgs span several notices at a mean of 15.3
/// mentions — the key is doing the LINKING. Condemning it would fragment
/// ~75k orgs to prevent ~0 bad merges. It lives here as the selector for
/// the 312 restore pass, and as the shape a future `platform-guid`
/// identifier kind will classify on.
pub fn uuid_v4(value: &str) -> bool {
    let hex: String = value.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return false;
    }
    let b = hex.as_bytes();
    b[12] == b'4' && matches!(b[16].to_ascii_lowercase(), b'8' | b'9' | b'a' | b'b')
}

/// The placeholder lexicon, seeded from the measured top-30 (probe §2) and
/// the census run-1330 findings: NIMATn (SI e-procurement family — 794
/// measured strangers on one id), ORGnnn/ORG-0001 (eForms technical ids),
/// Value prefixes that mark an identifier as addressing a ROUTE or a REPORTING
/// UNIT rather than a party (issue 365 unit 3, and the routing denial recorded
/// at `300-org-fuzzy-matching-design.md:827`).
///
/// Kept as a short explicit list rather than a general rule: the letter-run
/// class these live in ALSO contains real registry numbers wearing a label
/// (`CVRNR…`, `SIRET…`, `HANDELSREGISTERHRB…`, `REGISTRIERUNGSNUMMER…`), which
/// the 359/363 strip vocabulary should recover rather than have refused here,
/// and at least one genuine high-volume key (org 28's `0204994DOEVD83`,
/// 370,791 mentions). A blanket letter-run rule would take all of them.
const ROUTING_SCOPE_PREFIXES: &[&str] = &["LEITWEG", "BERICHT"];

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
    // The eForms field vocabulary pasted into the identifier slot. This was an
    // equality on "BT501" alone, which refused the bare field id while letting
    // `BT-501-Organization-Company` — normalised to BT501ORGANIZATIONCOMPANY,
    // and the form publishers actually paste — become a live merge key for six
    // unrelated bodies in six countries (issue 365).
    //
    // Kept deliberately narrow so real registrants are not swept: the prefix
    // must be BT/OPT/OPP, the field number at most four digits, and what follows
    // must be either nothing or a run of ≥4 letters (the "ORGANIZATIONCOMPANY"
    // tail). So BT12345678 and OPTIMA2020 are still identifiers.
    for prefix in ["BT", "OPT", "OPP"] {
        if let Some(rest) = value.strip_prefix(prefix) {
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            let tail = &rest[digits..];
            if (1..=4).contains(&digits)
                && (tail.is_empty()
                    || (tail.len() >= 4 && tail.bytes().all(|b| b.is_ascii_uppercase())))
            {
                return true;
            }
        }
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
/// `pub(crate)` for the crosswalk: the FR VAT→SIREN E1 key exists only when
/// this arithmetic PROVES the mapping (issue 300 Stage 2).
pub(crate) fn fr_vat_key(digits: &[u8]) -> Checksum {
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

/// DK CVR (8): weights 2,7,6,5,4,3,2,1 over ALL eight digits, weighted sum
/// divisible by 11. Verified live against 400 DK-country rows: 380 pass, and
/// the 20 failures are visibly mis-filed foreign numbers (a UK company
/// number, a P-nummer-shaped row). Anchor-path only — NOT wired into the
/// census gate (Stage-1 condemnation policy is a separate decision).
fn dk_cvr(digits: &[u8]) -> Checksum {
    const W: [u32; 8] = [2, 7, 6, 5, 4, 3, 2, 1];
    let sum: u32 = W.iter().zip(digits).map(|(w, &d)| w * u32::from(d)).sum();
    to_checksum(sum % 11 == 0)
}

/// SI davčna številka (8): weights 8,7,6,5,4,3,2 over the first 7; rem 0 ⇒
/// NEVER issued (the stdnum/jsvat rule — an adversarial panel caught the
/// first cut folding rem 0 to check 0, and since SI's and CZ's check rules
/// agree at every rem ≥ 1, that made every UNIQUE SI anchor a phantom of
/// the never-issued class); rem 1 ⇒ check 0 (the 10→0 half, live-mandated:
/// rem-1 specimens like 11022680 are real, verified 60/60 on SI-prefixed
/// VAT bodies where the "10 = not issued" variant fails exactly those ten);
/// else check = 11 − rem. SI-valid values therefore ALWAYS co-anchor CZ and
/// stay ambiguous — the honest reading. Anchor-path only, like dk_cvr.
fn si_davcna(digits: &[u8]) -> Checksum {
    const W: [u32; 7] = [8, 7, 6, 5, 4, 3, 2];
    let sum: u32 = W.iter().zip(digits).map(|(w, &d)| w * u32::from(d)).sum();
    let check = u32::from(digits[7]);
    match sum % 11 {
        0 => Checksum::Fail,
        1 => to_checksum(check == 0),
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
/// BG EIK / BULSTAT (9): weights 1..8 over the first eight, mod 11. A remainder
/// of 10 is not a valid check digit, so the scheme re-weights with 3..10 and
/// takes that mod 11; a second 10 becomes 0.
///
/// Issue 326: this is the LARGEST single arm in the undecidable class — 147 of
/// the 430 clusters have `BG` as their heaviest code, and a Bulgarian EIK
/// currently cannot be confirmed Bulgarian because no arm asks.
fn bg_eik(digits: &[u8]) -> Checksum {
    let weighted = |w: [u32; 8]| -> u32 {
        let sum: u32 = w.iter().zip(digits).map(|(w, &d)| w * u32::from(d)).sum();
        sum % 11
    };
    let first = weighted([1, 2, 3, 4, 5, 6, 7, 8]);
    let check = if first == 10 {
        let second = weighted([3, 4, 5, 6, 7, 8, 9, 10]);
        if second == 10 { 0 } else { second }
    } else {
        first
    };
    to_checksum(check == u32::from(digits[8]))
}

/// LT juridinio asmens kodas (9): weights 1..9 over the first eight in the
/// first pass — 1,2,3,4,5,6,7,8 — mod 11; a remainder of 10 re-weights with
/// 3,4,5,6,7,8,9,1 and takes that mod 11, a second 10 becoming 0.
///
/// Issue 326: 110 of the 430 undecidable clusters have `LT` as their heaviest
/// code, second only to Bulgaria. Note that `LT` and `LV` are one letter apart
/// and Latvia is overwhelmingly the TARGET rather than the source — 91
/// appearances but only 15 as the heavy side — so arming Lithuania is what
/// decides most of that pair.
///
/// **THIS ARM AND [`bg_eik`] ARE VERY NEARLY THE SAME FUNCTION**, and a reader
/// comparing their outputs has to know it. Both take weights 1..8 mod 11 as
/// their first pass and differ only in the second, which runs when that
/// remainder is 10 — about one value in eleven. Measured against prod:
/// `bg_eik` passes **93.8%** of Lithuanian rows and `lt_kodas` **84.2%** of
/// Bulgarian ones. So a joint pass is NOT evidence for either country, exactly
/// as `CZ:ico` and `SK:ico` sharing arithmetic is not evidence between those
/// two.
///
/// This is harmless for the census that motivated the arms, because `named`
/// intersects the anchors with the CLUSTER's own codes: a `BG`/`BI` cluster
/// never contains `LT`, so the Lithuanian pass cannot muddy it. It would only
/// bite on a cluster holding both, and `BG`/`LT` are not one letter apart, so
/// such a cluster is `no-one-letter-pair` before any of this is consulted.
fn lt_kodas(digits: &[u8]) -> Checksum {
    let weighted = |w: [u32; 8]| -> u32 {
        let sum: u32 = w.iter().zip(digits).map(|(w, &d)| w * u32::from(d)).sum();
        sum % 11
    };
    let first = weighted([1, 2, 3, 4, 5, 6, 7, 8]);
    let check = if first == 10 {
        let second = weighted([3, 4, 5, 6, 7, 8, 9, 1]);
        if second == 10 { 0 } else { second }
    } else {
        first
    };
    to_checksum(check == u32::from(digits[8]))
}

fn gr_afm(digits: &[u8]) -> Checksum {
    let sum: u32 =
        digits[..8].iter().enumerate().map(|(i, &d)| u32::from(d) << (8 - i)).sum();
    to_checksum((sum % 11) % 10 == u32::from(digits[8]))
}

/// Issue 300 Stage 3: which national schemes' checksums a COUNTRY-LESS digit
/// string satisfies — the anchoring probe for the NULL-country rescue pool.
/// Returns the `(scheme, canonical key)` pairs whose length fits AND whose
/// checksum PASSES. Exactly one anchor makes the value R3-attributable to a
/// country (the design's "hard-checksum pass" condition); several make it the
/// EBSCO class (a bare digit string is not single-country evidence — a
/// 10-digit Luhn pass could be a SE orgnr or match PL:nip's shape); zero
/// leaves it unanchored. Anchor schemes are checksum probes, not the census
/// hard/soft roster: the 8-digit arm's dk_cvr/si_davcna are anchor-path-only
/// arithmetic (validated live, DK own-bucket 95%), and the FR 14-digit SIRET
/// is deliberately included via its SOFT Luhn because its key is the
/// TRUNCATED SIREN and the design treats a Luhn-passing 14-digit as
/// SIRET-shaped-with-country (the CNFPT NULL class — Stage 3's cleanest
/// rescue; the R3 stack still demands name corroboration on top).
pub fn checksum_anchors(value: &str) -> Vec<(&'static str, String)> {
    let digits: Vec<u8> = value.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
    if value.bytes().any(|b| b.is_ascii_alphabetic()) || digits.is_empty() {
        // Letter-bearing values are register-prefixed forms — the design's
        // OTHER R3 alternative, out of this probe's scope.
        return Vec::new();
    }
    let key: String = digits.iter().map(|d| (d + b'0') as char).collect();
    let mut out: Vec<(&'static str, String)> = Vec::new();
    for (scheme, check) in uniform_arm(digits.len()) {
        if check(&digits) == Checksum::Pass {
            out.push((scheme, key.clone()));
        }
    }
    if digits.len() == 14 && !ambiguous_pad(&key) && luhn(&digits) == Checksum::Pass {
        out.push(("FR:siren", key[..9].to_owned()));
    }
    out
}

/// The uniform arms of the anchor probe: for each value SHAPE (digit count),
/// the schemes whose arithmetic is even ATTEMPTED.
///
/// ONE table, because two readers need the same answer and any drift between
/// them is a lie in a reviewer's evidence. `checksum_anchors` runs the
/// arithmetic and reports what PASSED; `anchor_vocabulary` reports what was
/// ASKED. A reviewer who sees "the row says SK and the value anchors CZ" has
/// to be able to tell "tested as SK, failed" from "no SK scheme has this
/// shape, so the silence means nothing" — and only the asked set answers that
/// (issue 314). The 14-digit FR:siren arm is not in the table: it carries a
/// leading-zero judgment and a TRUNCATED key, so both readers special-case
/// it — through the shared `ambiguous_pad` predicate, for the same reason.
fn uniform_arm(len: usize) -> &'static [(&'static str, fn(&[u8]) -> Checksum)] {
    match len {
        8 => &[
            ("CZ:ico", cz_ico),
            ("FI:ytunnus", fi_ytunnus),
            ("DK:cvr", dk_cvr),
            ("SI:davcna", si_davcna),
        ],
        9 => &[("FR:siren", luhn), ("NO:orgnr", no_orgnr), ("PT:nif", pt_nif), ("GR:afm", gr_afm)],
        10 => &[("SE:orgnr", luhn), ("PL:nip", pl_nip), ("BE:kbo", be_kbo)],
        11 => &[("IT:piva", it_piva), ("HR:oib", mod_11_10)],
        _ => &[],
    }
}

/// The arms that exist for EVIDENCE rather than for MERGE DECISIONS, and why
/// that distinction had to be drawn (issue 326).
///
/// **These are NOT in `uniform_arm`, and the reason is measured.** The
/// resolver's anchor path and the R3 merge both require EXACTLY ONE anchor —
/// `real.len() == 1` — so every scheme added to the shared table makes some
/// previously-decidable value ambiguous and silently narrows a merge path.
/// Adding these three to `uniform_arm` and sampling 1,500 random corpus values
/// per shape:
///
/// ```text
///   8-digit: single-anchor 800 -> 385   — 51.9% of anchored values LOSE the path
///   9-digit: single-anchor 662 -> 521   — 21.3% LOSE it
/// ```
///
/// Slovakia is the extreme case: `SK:ico` is the SAME mod-11 arithmetic as
/// `CZ:ico`, so adding it under its own name turns every single Czech anchor
/// into a double one. That is a halving of the 8-digit merge path's reach in
/// exchange for 18 census clusters, and it is not a trade worth making
/// silently.
///
/// The census asks a different question. It intersects anchors with a
/// CLUSTER's own country codes, so ambiguity is not fatal there — it surfaces
/// as the honest `anchor-names-several` verdict — and a scheme that names a
/// country the cluster does not hold is simply ignored. So the evidence probe
/// gets these arms and the decision probe does not.
///
/// One table each, and `census_anchors` is built ON TOP of `checksum_anchors`
/// rather than beside it, so the shared arms cannot drift apart.
fn census_only_arm(len: usize) -> &'static [(&'static str, fn(&[u8]) -> Checksum)] {
    match len {
        // Slovakia rides Czechia's arithmetic; this file already said so, in
        // `anchor_vocabulary`'s doc. Issue 326 makes the observation usable
        // without making it costly.
        8 => &[("SK:ico", cz_ico)],
        // The two biggest arms in the undecidable class: BG is the heaviest
        // code in 147 of the 430 clusters, LT in 110.
        9 => &[("BG:eik", bg_eik), ("LT:kodas", lt_kodas)],
        _ => &[],
    }
}

/// The anchor probe for EVIDENCE: every scheme `checksum_anchors` reports plus
/// the census-only arms above. Issue 326's census uses this; nothing that
/// merges does.
pub fn census_anchors(value: &str) -> Vec<(&'static str, String)> {
    let mut out = checksum_anchors(value);
    let digits: Vec<u8> = value.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
    if value.bytes().any(|b| b.is_ascii_alphabetic()) || digits.is_empty() {
        return out;
    }
    let key: String = digits.iter().map(|d| (d + b'0') as char).collect();
    for (scheme, check) in census_only_arm(digits.len()) {
        if check(&digits) == Checksum::Pass {
            out.push((scheme, key.clone()));
        }
    }
    out
}

/// The vocabulary counterpart of [`census_anchors`] — what the evidence probe
/// ASKED about, so a negative stays readable (issue 314's `country_probed`).
pub fn census_vocabulary(value: &str) -> Vec<&'static str> {
    let mut out = anchor_vocabulary(value);
    let digits: Vec<u8> = value.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
    if value.bytes().any(|b| b.is_ascii_alphabetic()) || digits.is_empty() {
        return out;
    }
    out.extend(census_only_arm(digits.len()).iter().map(|(s, _)| *s));
    out
}

/// Mirror the crosswalk's leading-zero judgment (verification round): a
/// 14-digit value whose zero-strip is EXACTLY 9 digits is ambiguous — a
/// zero-padded 9-digit national as plausibly as a low-SIREN SIRET — and the
/// crosswalk demotes it to E2. An anchor must not be more confident than the
/// key it anchors to. The probe therefore DECLINES to decide such a value,
/// which is why the vocabulary must not claim FR was asked about it.
fn ambiguous_pad(key: &str) -> bool {
    key.starts_with('0') && key.trim_start_matches('0').len() == 9
}

/// The schemes the anchor probe ASKS about this value — pass or fail.
///
/// `checksum_anchors` reports what passed; this reports what was asked, and
/// the difference is what makes a NEGATIVE readable. A value that anchors
/// CZ:ico on a row claiming SK was never tested as SK — no SK scheme has an
/// 8-digit arm — so that row's country is UNPROBED, not contradicted (and CZ
/// and SK IČO share the same mod-11 arithmetic, so the CZ pass is not even
/// weak evidence against SK). The same value on a row claiming FI *was*
/// tested, FI:ytunnus being in the 8-digit arm, and its silence is a real
/// contradiction. Empty for letter-bearing values, which the probe declines
/// wholesale — there the silence is about the VALUE's shape, not the row's
/// country.
pub fn anchor_vocabulary(value: &str) -> Vec<&'static str> {
    let digits: Vec<u8> = value.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
    if value.bytes().any(|b| b.is_ascii_alphabetic()) || digits.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<&'static str> = uniform_arm(digits.len()).iter().map(|(sc, _)| *sc).collect();
    if digits.len() == 14 {
        let key: String = digits.iter().map(|d| (d + b'0') as char).collect();
        if !ambiguous_pad(&key) {
            out.push("FR:siren");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue 358: a national key under a regional code is scored in its
    /// register's series; a code with a register of its own is not.
    #[test]
    fn a_regional_code_scores_in_its_registers_series() {
        assert_eq!(census(Some("RE"), Some("national"), "552081317").scheme, "FR:siren");
        assert_eq!(census(Some("MQ"), Some("national"), "55208131700013").scheme, "FR:siret");
        let ax = census(Some("AX"), Some("national"), "01003158");
        assert_eq!((ax.scheme, ax.checksum), ("FI:ytunnus", Checksum::Pass));
        assert!(
            condemns(Some("AX"), "national", "01003159"),
            "the HARD FI checksum now gates a Y-tunnus under AX"
        );
        assert_eq!(census(Some("SJ"), Some("national"), "923609016").scheme, "NO:orgnr");
        assert_eq!(census(Some("NC"), Some("national"), "552081317").scheme, "other");
        assert_eq!(census(Some("FO"), Some("national"), "01003158").scheme, "other");
    }

    /// The drift guard for the two readers of `uniform_arm` (issue 314).
    ///
    /// The packet tells a reviewer "the probe asked about your row's country
    /// and it refused" only when the country is in `anchor_vocabulary`. If a
    /// scheme could PASS without being in the vocabulary, that sentence would
    /// be built from a set that does not contain the scheme that produced the
    /// evidence — the packet would report a contradiction as an unasked
    /// question, or worse, the reverse. So: everything that can pass must be
    /// something the vocabulary admits was asked.
    #[test]
    fn every_anchor_that_passes_was_one_the_vocabulary_admits_asking() {
        // Shapes across every arm, including the 14-digit special case in both
        // of its states (ambiguous zero-pad, and a clean SIRET).
        for v in [
            "12345670",
            "27074358",
            "980921565",
            "123456789",
            "5560269986",
            "1234567890",
            "12345678903",
            "00012345678901",
            "73282932000074",
            "",
            "HRB 12345",
            "DE123456789",
        ] {
            let vocab = anchor_vocabulary(v);
            for (scheme, _) in checksum_anchors(v) {
                assert!(
                    vocab.contains(&scheme),
                    "{v}: {scheme} passed but the vocabulary does not admit asking it"
                );
            }
        }
    }

    /// The other half of parity: a value the probe DECLINES to decide must not
    /// appear in the vocabulary either. The 14-digit zero-pad is the only such
    /// case — the probe refuses it for ambiguity, and a reviewer told "FR was
    /// asked and refused" would read a refusal-to-decide as a verdict.
    #[test]
    fn a_declined_shape_is_not_reported_as_a_question_that_was_asked() {
        // 14 digits whose zero-strip is exactly 9: the crosswalk's E2 demotion.
        let padded = "00000123456789";
        assert_eq!(padded.len(), 14);
        assert!(checksum_anchors(padded).is_empty(), "the probe declines to decide it");
        assert!(
            !anchor_vocabulary(padded).contains(&"FR:siren"),
            "so it must not be reported as an FR question that was asked"
        );
        // …while a clean 14-digit IS asked, whatever the arithmetic says.
        assert!(anchor_vocabulary("73282932000074").contains(&"FR:siren"));
    }

    /// Issue 312: the platform-GUID shape. Specimens are REAL values the
    /// issue-311 campaign reviewed case by case (orgs 22310065, 22149631,
    /// 22495412), stored undashed as the corpus holds them. The predicate
    /// must stay OUT of the gate: the measurement says this class links
    /// rather than false-merges.
    #[test]
    fn v4_uuids_are_recognised_but_never_condemned() {
        for real in [
            "DA23095600854B59BC39FE71D8AF0A7C",
            "2B0E62BAFDC94209A833C559DC25A351",
            "1271A766403E4FB8AE5E0974483504A6",
        ] {
            assert!(uuid_v4(real), "{real} is a v4 platform key");
            assert!(
                !condemns(Some("DE"), "national", real),
                "{real} must KEEP merge-key status (issue 312: it links, it does not merge)"
            );
        }
        assert!(uuid_v4("da230956-0085-4b59-bc39-fe71d8af0a7c"), "the canonical dashed form too");
        // Narrow by construction: version nibble 1, then a non-RFC variant.
        assert!(!uuid_v4("DA23095600851B59BC39FE71D8AF0A7C"), "v1 is not the leaked class");
        assert!(!uuid_v4("DA23095600854B597C39FE71D8AF0A7C"), "variant 7 is not RFC-4122");
        assert!(!uuid_v4("DA23095600854B59BC39FE71D8AF0A7"), "31 chars");
        assert!(!uuid_v4("ZA23095600854B59BC39FE71D8AF0A7C"), "non-hex lead");
        assert!(!uuid_v4("DE144202483"), "a real VAT is not a GUID");
    }

    /// Stage 3's anchoring probe: a Luhn-valid 14-digit anchors uniquely to
    /// its truncated SIREN (the CNFPT NULL class); an 8-digit value anchors
    /// uniquely when exactly one of the four register checksums accepts
    /// (the 8-digit slice — SI/CZ co-anchor by construction); bare digit strings
    /// with several passing schemes stay EBSCO-class ambiguous.
    #[test]
    fn checksum_anchors_classify_the_null_country_shapes() {
        let anchors = checksum_anchors("18001404501577");
        assert_eq!(anchors, vec![("FR:siren", "180014045".to_owned())]);
        // 8-digit anchors are real probes since the Stage-3 8-digit slice:
        // Telinekataja's and Maintpartner's Y-tunnus values pass ONLY the FI
        // arithmetic (CZ/DK/SI all reject) — the unique-anchor rescue shape.
        assert_eq!(checksum_anchors("01003158"), vec![("FI:ytunnus", "01003158".to_owned())]);
        assert_eq!(checksum_anchors("20445111"), vec![("FI:ytunnus", "20445111".to_owned())]);
        // A live CVR (Tømrer, Murer & Kloakmester John A. Laursen A/S)
        // anchors uniquely DK; the SI 10→0 rem-1 specimen anchors SI but
        // co-anchors CZ (near-identical mod-11) — ambiguous, correctly.
        assert_eq!(checksum_anchors("10006511"), vec![("DK:cvr", "10006511".to_owned())]);
        let si = checksum_anchors("11022680");
        assert!(si.iter().any(|(s, _)| *s == "SI:davcna"), "the 10->0 rule must accept rem-1");
        assert!(si.len() >= 2, "SI/CZ co-anchor by construction: {si:?}");
        // A mis-filed UK company number under a DK row: every scheme rejects.
        assert!(checksum_anchors("00971289").is_empty(), "foreign noise anchors nowhere");
        // The panel's phantom class: prefix-rem-0 values are NEVER-ISSUED
        // davčna numbers, and since SI ≡ CZ at every rem ≥ 1, a rem-0
        // acceptor would make every unique SI anchor a phantom. 10000070 is
        // rem-0-check-0 (uniquely-SI under the defective first cut): it must
        // anchor NOWHERE.
        assert!(
            checksum_anchors("10000070").is_empty(),
            "rem-0 davcna values are never issued"
        );
        assert!(checksum_anchors("180014045").iter().any(|(s, _)| *s == "FR:siren"));
        assert!(checksum_anchors("HRB 12345").is_empty(), "letters are the register path");
        // The crosswalk's leading-zero judgment, mirrored (verification
        // round): zero-strip == 9 is a plausible zero-padded 9-digit national
        // — no anchor, matching the crosswalk's E2 demotion. Leading zeros
        // are Luhn-invariant, so 00000 + a valid SIREN is Luhn-valid 14.
        assert!(
            checksum_anchors("00000732829320").is_empty(),
            "zero-strip==9 must not anchor"
        );
        // A genuine 0-leading SIRET (live: RDT 13) whose strip is NOT 9
        // still anchors to its truncated SIREN.
        assert_eq!(
            checksum_anchors("06880164600040"),
            vec![("FR:siren", "068801646".to_owned())]
        );
    }

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

    /// The two classes the 2026-09-03 top-100 multi-name read added, pinned
    /// on the live specimens (issue 300 exemplar sheet). Both are census-only:
    /// nothing here changes `condemns`.
    #[test]
    fn phone_ids_and_short_de_numerics_are_censused_like_the_top_100_read() {
        // Orgs 660 (stored with its colon), 447, 633 and 122 (stored bare).
        for v in ["t:04131153308", "T03455141536", "T03318661719", "T022894990"] {
            assert!(census(Some("DE"), Some("national"), v).phone, "{v} is a phone number");
        }
        // Not phones: a real AT register id, a DE court id, a Thüringen chamber id.
        for v in ["210220Y", "HRB12345", "16900334000129", "T1234"] {
            assert!(!census(Some("DE"), Some("national"), v).phone, "{v} is not a phone number");
        }
        // Orgs 22318692, 21985079: bare 5-digit DE "identifiers". The 4-digit
        // half of this class (org 22165664's `8477`) moved to
        // `bare_four_digit` when issue 365 unit 2 made it condemning; the two
        // stay disjoint so the weekly report's counts stay readable.
        for v in ["13754", "13124"] {
            let c = census(Some("DE"), Some("national"), v);
            assert!(c.short_numeric && !c.sequence, "{v} is the short-numeric class");
        }
        let c = census(Some("DE"), Some("national"), "8477");
        assert!(c.bare_four_digit && !c.short_numeric, "8477 is the four-digit class now");
        // Not the class: a sequence (already caught), a zero-padded 8-digit
        // Hessian id, the same digits on a non-DE row, a VAT stub.
        assert!(!census(Some("DE"), Some("national"), "1234").short_numeric);
        assert!(!census(Some("DE"), Some("national"), "00002636").short_numeric);
        assert!(!census(Some("AT"), Some("national"), "8477").short_numeric);
        assert!(!census(Some("DE"), Some("vat"), "8477").short_numeric);
        // A phone id is never a checksum candidate.
        assert_eq!(census(Some("DE"), Some("national"), "T03455141536").checksum, Checksum::Unknown);
    }

    /// Issue 365 unit 3: routing and reporting ids lose merge-key status, while
    /// the letter-run class they sit inside does NOT.
    ///
    /// The composition read this unit owed found the class is a mixture, so
    /// condemning `letter_run` wholesale would have been wrong in both
    /// directions. It holds routing references that fuse (a Leitweg-ID with 43
    /// distinct mention names), real registry numbers merely wearing a label
    /// (`CVRNR…`, `SIRET…`, `HANDELSREGISTERHRB…` — those want STRIPPING, not
    /// refusing), and at least one genuine high-volume key: org 28's
    /// `0204994DOEVD83` carries **370,791** mentions across only 15 names,
    /// which is a key doing its job.
    ///
    /// So the negative cases below are the point of the test, not decoration.
    #[test]
    fn routing_ids_are_refused_but_the_letter_run_class_around_them_is_not() {
        for v in [
            "LEITWEGID08A986640",
            "LEITWEGID09162000ZRE100000009",
            "LEITWEGEID140201004SK0113",
            "LEITWEGSID0516200080083100142",
            "LEITWEGLD08A986640",
            "BERICHTSEINHEITID00002636",
            "BERICHTEINHEITID00002636",
            "BERICHTSID00007427",
        ] {
            assert!(condemns(Some("DE"), "national", v), "{v} addresses a route, not a party");
            assert!(census(Some("DE"), Some("national"), v).routing_scope, "{v} classified");
        }

        // Real registry numbers wearing a label prefix. These are the 359/363
        // strip vocabulary's business — recovering the id underneath — and must
        // NOT be swept away here, even though every one of them has a ≥4 letter
        // run and would fall to a blanket letter-run rule.
        // NB: realistic digit runs on purpose. An ascending run like
        // `CVRNR12345678` is condemned by `sequence` whatever this rule does, so
        // it would assert nothing — a trap that has now caught two of these
        // tests during authoring.
        for v in ["CVRNR29189498", "SIRET78467169500087", "HANDELSREGISTERHRB93017"] {
            assert!(!condemns(Some("DK"), "national", v), "{v} is a real id under a label");
        }
        // And the high-volume genuine key, which a blanket rule would have cost
        // 370,791 mentions.
        assert!(!condemns(Some("DE"), "national", "0204994DOEVD83"));
        // The class itself stays census-only: computed, reported, not condemning.
        assert!(census(Some("DE"), Some("national"), "0204994DOEVD83").letter_run);
    }

    /// Issue 365 unit 1: the phone class now LOSES merge-key status, not just a
    /// census tick.
    ///
    /// The class was left census-only on the reading that the review chambers
    /// publish a switchboard consistently, so it keys a body more often than it
    /// fuses two. Re-measured on prod 2026-09-09 over `id <= 3000000`, that is
    /// not what the stock looks like: of 68 phone-keyed canonical orgs, 47 (69 %)
    /// carry two or more distinct mention names and 24 (35 %) carry six or more,
    /// against a corpus baseline of 14.7 % and 1.1 % — and the worst single row
    /// holds **264** distinct names over 154,671 mentions on those 68 rows. A row
    /// with 264 names is a switchboard, not an organization.
    ///
    /// This is the mirror image of the hex-hash class (issue 312), which was
    /// measured and deliberately spared because its values were 1:1 with rows and
    /// were doing the linking. Same method, opposite answer.
    #[test]
    fn phone_numbers_lose_merge_key_status() {
        for v in ["t:04131153308", "T03455141536", "T03318661719", "T022894990"] {
            assert!(condemns(Some("DE"), "national", v), "{v} must not be a merge key");
        }
        // The shape stays narrow: a real register id that merely starts with T,
        // and a too-short run, are untouched.
        // Non-sequential on purpose: an ascending digit run like HRB12345 is
        // condemned by `sequence` regardless, so it would prove nothing here.
        for v in ["T1234", "HRB93017", "210220Y"] {
            assert!(!condemns(Some("DE"), "national", v), "{v} is a real identifier");
        }
    }

    /// Issue 365 unit 2: the eForms field NAME, not just its bare id.
    ///
    /// `lexicon_hit` already knew `BT501` — but as an equality, so the bare field
    /// id was refused while `BT-501-Organization-Company`, which is what
    /// publishers actually paste into the identifier slot, walked straight past
    /// and became a live merge key for six unrelated bodies in six countries
    /// (ES/FR/GR/IE/IT/SE, 32 mentions on one key).
    #[test]
    fn an_eforms_field_name_is_never_an_identifier() {
        for v in [
            "BT501ORGANIZATIONCOMPANY",   // the six-country row, as stored
            "BT500ORGANIZATIONCOMPANY",   // its sibling, seen as a mention NAME
            "BT501",                      // the bare id, already covered
            "OPT200ORGANIZATIONTECHNICAL",
            "OPP105BUSINESS",
        ] {
            assert!(condemns(None, "national", v), "{v} is a field name, not an id");
        }
        // Not swept: real identifiers that merely begin with those letters, and
        // a long digit run that is a plausible registration number.
        for v in ["BT93017425", "OPTIMA2020", "BTG1234", "OPPENHEIM99"] {
            assert!(!condemns(None, "national", v), "{v} is a real identifier");
        }
    }

    /// Issue 365 unit 2: a bare four-digit number is not a register number.
    ///
    /// `2022` and `1000` each key six unrelated bodies across six countries.
    /// Measured on prod 2026-09-09 over `id <= 3000000`: 93 orgs keyed by a bare
    /// four-digit value, 37 (40 %) carrying two or more distinct mention names
    /// against the 14.7 % baseline, the worst holding 71. Independently of
    /// whether some registry issues such numbers, a 10,000-value space cannot
    /// discriminate between 5.7M organizations — the same reasoning `short_vat`
    /// already applies to a short VAT tail.
    #[test]
    fn a_bare_four_digit_number_is_not_a_merge_key() {
        for v in ["2022", "1000", "2019", "8477"] {
            assert!(condemns(None, "national", v), "{v} cannot discriminate");
        }
        // Five digits and up stay in: those are real short registry numbers,
        // and the zero-padded and sequence families already have their own rules.
        for v in ["84771", "52830", "HRB1234"] {
            assert!(!condemns(None, "national", v), "{v} keeps its merge-key status");
        }
    }
}

#[cfg(test)]
mod issue_326_arms {
    use super::*;

    /// Real registrants, pulled from prod's own `BG`/`LT`/`SK` rows. The
    /// algorithms were written from specification and then MEASURED against the
    /// corpus before being deployed: 94.0%, 94.0% and 98.5% pass against a
    /// chance rate of about 9.1% for a mod-11 scheme. A wrong arm reads near
    /// chance, so the measurement is the proof and these fixtures are the
    /// regression guard on it.
    #[test]
    fn the_new_arms_validate_real_registrants() {
        for v in ["000003338", "000003361", "000003577", "000010756", "000010838"] {
            let d: Vec<u8> = v.bytes().map(|b| b - b'0').collect();
            assert_eq!(bg_eik(&d), Checksum::Pass, "BG {v}");
        }
        for v in ["105149515", "105708716", "108784411", "110005648", "110011925"] {
            let d: Vec<u8> = v.bytes().map(|b| b - b'0').collect();
            assert_eq!(lt_kodas(&d), Checksum::Pass, "LT {v}");
        }
        // Slovakia rides Czechia's arithmetic — the same function, now also
        // ASKED under its own name.
        for v in ["00002313", "00002801", "00002895", "00003328", "00003964"] {
            let d: Vec<u8> = v.bytes().map(|b| b - b'0').collect();
            assert_eq!(cz_ico(&d), Checksum::Pass, "SK {v}");
        }
    }

    /// How well the arm catches a single mistyped digit — which is exactly the
    /// error issue 326 is about, so the number matters and it is NOT 100%.
    ///
    /// **98.35% caught, 1.65% missed**, measured over every position and every
    /// substitution on 200 passing prod values. A first draft of this test
    /// asserted that every slip fails; it does not, and the reason is the
    /// scheme's own shape. The two-pass fallback means a changed digit can move
    /// a value INTO the second weighting, where a different coefficient set
    /// applies and the result can coincidentally validate. A plain single-pass
    /// mod-11 would catch all of them; this one cannot, by construction.
    ///
    /// The misses are spread evenly across positions (41/35/26/31/36/29/38/31),
    /// so there is no position a typo can hide in — it is a uniform 1-in-60,
    /// not a blind spot.
    #[test]
    fn a_one_digit_slip_is_caught_but_not_always() {
        let (mut caught, mut missed) = (0u32, 0u32);
        for v in ["000003338", "000003361", "000003577", "000010756", "000010838"] {
            let orig: Vec<u8> = v.bytes().map(|b| b - b'0').collect();
            assert_eq!(bg_eik(&orig), Checksum::Pass, "fixture {v} must pass to begin with");
            for pos in 0..9 {
                for delta in 1..10u8 {
                    let mut d = orig.clone();
                    d[pos] = (d[pos] + delta) % 10;
                    match bg_eik(&d) {
                        Checksum::Fail => caught += 1,
                        _ => missed += 1,
                    }
                }
            }
        }
        let total = caught + missed;
        assert!(
            caught * 100 / total >= 95,
            "the arm must catch the great majority of single-digit slips: \
             {caught}/{total}"
        );
        assert!(
            missed > 0,
            "and it does NOT catch all of them — if this ever passes, the \
             two-pass fallback has been dropped and the doc above is stale"
        );
    }

    /// THE MERGE PATH MUST NOT MOVE. This is the guard on the split between the
    /// evidence probe and the decision probe, and it exists because adding
    /// these arms to the shared table was measured to halve the 8-digit merge
    /// path: single-anchor values fell 800 -> 385 in a 1,500-value sample,
    /// because `SK:ico` is the same arithmetic as `CZ:ico` and doubles every
    /// Czech anchor.
    ///
    /// The resolver and R3 both require `real.len() == 1`, so a second scheme
    /// name on the same arithmetic is not a richer answer — it is a silently
    /// disabled path.
    #[test]
    fn the_census_arms_never_reach_the_decision_probe() {
        for v in ["00002313", "00002801", "00003328", "000003338", "105149515", "110005648"] {
            let decision = checksum_anchors(v);
            let evidence = census_anchors(v);
            assert!(
                evidence.len() >= decision.len(),
                "{v}: the evidence probe is a superset"
            );
            for scheme in ["SK:ico", "BG:eik", "LT:kodas"] {
                assert!(
                    !decision.iter().any(|(s, _)| *s == scheme),
                    "{v}: {scheme} must NOT be in the decision probe"
                );
            }
        }
        // The Slovak case in full, stated as a DELTA rather than an absolute:
        // the census probe adds exactly the SK arm on top of whatever the
        // decision probe already found, and the decision probe's own answer is
        // byte-for-byte unchanged. (An absolute count was the first draft's
        // mistake — `00002313` anchors four 8-digit schemes, not one, so it was
        // never the single-anchor example I assumed.)
        let ico = "00002313";
        let decision = checksum_anchors(ico);
        let evidence = census_anchors(ico);
        assert!(decision.iter().any(|(s, _)| *s == "CZ:ico"));
        assert_eq!(
            evidence.len(),
            decision.len() + 1,
            "exactly one arm added: {evidence:?}"
        );
        assert_eq!(
            evidence[..decision.len()],
            decision[..],
            "and the decision probe's own answer is untouched, in order"
        );
        assert!(evidence.iter().any(|(s, _)| *s == "SK:ico"));
        // …and the vocabulary tracks it, so a negative stays readable.
        assert!(census_vocabulary(ico).contains(&"SK:ico"));
        assert!(!anchor_vocabulary(ico).contains(&"SK:ico"));
    }

    /// THE ARMS ARE NEARLY THE SAME FUNCTION, and the test says so out loud
    /// rather than leaving it to be discovered. `bg_eik` and `lt_kodas` share
    /// their first pass and diverge only when the first remainder is 10.
    ///
    /// It does not harm the census that motivated them, because `named`
    /// intersects anchors with the CLUSTER's own codes and a `BG`/`BI` cluster
    /// holds no `LT`. It WOULD matter to anything treating a lone anchor as
    /// country evidence, so it is pinned here as a property of the pair.
    #[test]
    fn bg_and_lt_are_not_evidence_against_each_other() {
        let mut agree = 0;
        let mut total = 0;
        for n in 100_000_000u32..100_002_000 {
            let d: Vec<u8> = n.to_string().bytes().map(|b| b - b'0').collect();
            total += 1;
            if bg_eik(&d) == lt_kodas(&d) {
                agree += 1;
            }
        }
        assert!(
            agree * 100 / total >= 95,
            "the two arms agree on {agree}/{total} — they are near-identical by \
             construction, and a joint pass is evidence for NEITHER country"
        );
    }

    /// And the arms still discriminate against the other schemes sharing their
    /// digit length, or adding them would only manufacture ambiguity.
    #[test]
    fn the_nine_digit_arm_still_separates_its_older_members() {
        // A Greek AFM that is not a Bulgarian EIK.
        let mut hits = 0;
        for n in 100_000_000u32..100_001_000 {
            let d: Vec<u8> = n.to_string().bytes().map(|b| b - b'0').collect();
            let g = gr_afm(&d) == Checksum::Pass;
            let b = bg_eik(&d) == Checksum::Pass;
            if g != b {
                hits += 1;
            }
        }
        assert!(hits > 100, "GR and BG must disagree often; they agreed on all but {hits}");
    }
}
