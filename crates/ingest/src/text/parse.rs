//! The exhaustive-consumption parser for one text-era record (ADR-0004).
//!
//! Same contract as the XML walkers: every line of the record is claimed by a
//! field rule or the record quarantines whole, naming the line. The record
//! marker (`1.0/003065`) is the splitter's delimiter and is validated then
//! ignored; blank lines are layout. A record has no internal structure beyond
//! its header fields, so everything lands in one `PROCEDURE` section of kind
//! `Notice` — the same root-section convention as the other profiles.
//!
//! Value policy mirrors ted-legacy-mapping.md §8.2: quarantine is for
//! unconsumed structure (an unknown tag, a continuation under a scalar
//! field), never for low-quality values — a malformed date or code degrades
//! to a raw text row, typed value absent.

use store::{NoticeValue, Parsed, Section, ValueRow};

use super::rules::{self, Rule, Type};
use crate::r209::value;

/// The section every text-era value lands in, except the awarding authority.
const SECTION: &str = "PROCEDURE";

/// The synthesized Organization section holding the awarding authority's name
/// (issue 232).
///
/// The era publishes no section structure, so this is manufactured exactly as the
/// r209 walker manufactures its `ORG-n` sections for inline address blocks: open a
/// section of kind `Organization`, put the party's own values inside it, and record
/// the ROLE as an id-ref on the enclosing section. Reusing the legacy `TED-` role
/// vocabulary is deliberate — `project.rs`'s `legacy_role` already folds
/// `ADDRESS_CONTRACTING_BODY` onto `buyer`, so the projection needs no new mapping
/// and no new code path to reach this.
///
/// Why it has to be a section at all: `organization_mentions` carries FOREIGN KEY
/// (notice_id, section_id) REFERENCES notice_sections, and the projection only
/// seeds a mention for sections whose KIND says party. A name sitting on the
/// notice root — which is where `TXT-AU` sat for 3.79M notices — can never become
/// an Organization, whatever the field tables say.
const AUTHORITY_SECTION: &str = "ORG-1";

/// The labels under which a text-era body announces its winner (issue 244),
/// upper-cased for a case-insensitive match. Each is the tail of a longer heading and
/// each ends at the colon the value follows.
///
/// The 2004-onward *sectioned* form:
///
/// - `V.1.1)  Name and address of successful supplier, contractor or service provider:`
///   — the 2004/2005 vintage, the same wording the era's own `CO:` line carries;
/// - `V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR TO WHOM THE CONTRACT HAS BEEN
///   AWARDED:` — 2006 onward;
/// - `V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT AWARD
///   DECISION HAS BEEN TAKEN:` — the standard forms published from 2009-12-02 on (issue
///   508; first body notice 3,931,555). Measured: of 35,758 award notices from then to
///   the 2010-03 package, 142 carried a winner. From 2010-03 the same heading ends its
///   line with no colon at all, which no label here can match; [`line_awarded_names`]
///   reads that.
///
/// Measured coverage of award notices carrying an English body, per June window on
/// prod: 2004 202/203, 2005 311/314, 2006 347/354, 2008 361/371. Matching the tail
/// rather than the whole heading is deliberate: the heading itself wraps at ~72
/// columns, and the words before the colon are the part that stayed stable.
///
/// The pre-2004 *numbered* form, which neither of those two labels reaches — measured
/// on the 2001-06 monthly (fetch 300), where 619 of a 2,000-notice band are TD:7 award
/// records and the sectioned labels matched exactly ONE of them. Two shapes:
///
/// - `6.  Successful contractor(s): Mill Group, 3 Burlington Mews, UK-London W1R 8QA.`
///   — the works/services award form, items 1-14 with the winner at 6;
/// - `8.  Name and address of successful tenderer: Symonds Travers Morgan Ltd (UK) …`
///   — the EC external-aid (SCR/EuropeAid) form, winner at 8.
///
/// The singular variants are carried too: the era writes both `contractor(s)` and
/// `contractor`, and both `tenderer` and `tenderer(s)`.
///
/// The supplies and utilities forms name the winner under a *different* item and a
/// different word, which is why the first pre-2004 pass still missed four fifths of
/// the era's awards. Counted over `fetch 300`'s 13,734 bodies (2001-06), one scan:
///
/// ```text
///     …successful contractor…                                785
///     …successful tenderer…                                  122
///     supplier(s):                                         1,435
///     supplier(s), contractor(s) or service provider(s):      410
///     contractor(s):                                          735   (mostly the above)
/// ```
///
/// So `SUPPLIER(S):` alone is the single biggest remaining label. `SERVICE PROVIDER(S):`
/// is the tail of the long combined heading — matching the tail rather than the whole
/// thing covers the standalone spelling too, and the value starts at the same colon
/// either way. Note the existing `SERVICE PROVIDER:` does NOT reach that heading: the
/// era writes `service provider(s):` there and the `(s)` breaks the match.
const AWARD_LABELS: [&str; 11] = [
    "SERVICE PROVIDER:",
    "SERVICE PROVIDER(S):",
    "HAS BEEN AWARDED:",
    "HAS BEEN TAKEN:",
    "SUCCESSFUL CONTRACTOR(S):",
    "SUCCESSFUL CONTRACTOR:",
    "SUCCESSFUL TENDERER(S):",
    "SUCCESSFUL TENDERER:",
    "SUPPLIER(S):",
    "SUPPLIER:",
    "CONTRACTOR(S):",
];

/// How far past the label a name's end is looked for. Generous next to the measured
/// shapes — the longest name seen on prod is 89 characters and its comma follows
/// immediately — and the bound is what keeps the scan linear in the body rather than
/// quadratic in (awards × length).
const NAME_WINDOW: usize = 256;

/// Where the winner *value* ends: the next form item or heading. The value is not the
/// name — one value can carry several winners — so this bounds the value and the comma
/// rule inside [`awarded_names`] then bounds each name within it.
///
/// ` 7.`, ` 9.` and ` 10.` are the numbered form's item after the winner: 6 in the
/// works/services and supplies forms, 8 in the external-aid one, 9 in the utilities one. Without them
/// a winner whose address carries no comma runs on into the following item —
/// `Successful contractor(s): ACME Ltd. 7. Works provided: CPV: 45210000, 74222000`
/// would name the organization `ACME Ltd. 7. Works provided: CPV: 45210000`. The body
/// is flattened to single spaces before this runs, so the leading space makes the match
/// reliable.
const ITEM_STOPS: [&str; 8] =
    ["V.1.2)", "V.2)", "V.3)", "V.4)", "CONTRACT NO", " 7.", " 9.", " 10."];

/// Phrases that mean the value is not a name, so no organization is minted from it.
///
/// The pre-2004 numbered form fills a withheld item with boilerplate rather than
/// leaving it blank — `6.  Successful contractor(s): Publication of this information
/// would prejudice the legitimate commercial interests of a particular undertaking.`
/// (measured on prod: 2001-06 uses it for items 8, 9 and 10 of the same notice). That
/// sentence reaches a boundary well inside `NAME_WINDOW`, so the fall-through that
/// protects against runaway values does NOT catch it — it would be minted as an
/// organization, once per notice that withholds, which is exactly the identity-less
/// provisional-org problem of issue 234 manufactured on purpose.
///
/// Matched case-insensitively against the trimmed candidate. Deliberately literal and
/// short: a general "does this read like prose" test would also reject real names, and
/// any other withholding wording the era uses will surface as a junk organization and
/// can be added with its own evidence.
///
/// The 2010 print (issue 508) put a further family of non-names in the winner slot,
/// measured over the colonless sample (232 of 22,622 names) and the first drained round
/// (about 3,000 of its names): an unsuccessful or discontinued lot in French
/// (`Lot déclaré infructueux`, `Sans suite`, `Non attribué`), Spanish (`Desierto`) and
/// Italian (`Nessuna aggiudicazione`), and a pointer elsewhere — `Véase perfil del
/// contratante` alone is 243 names in one 20k-notice window, `See Section VI.2)
/// Additional information`, `Voir autres informations`. Each would be ONE nameless-
/// identity organization collecting every award that printed it.
const NAME_REJECTS: [&str; 13] = [
    "WOULD PREJUDICE",
    "NOT APPLICABLE",
    "INFRUCTU",
    "SANS SUITE",
    "NON ATTRIBU",
    "PERFIL DEL CONTRATANTE",
    "SEE SECTION",
    "VOIR AUTRES INFORMATIONS",
    "VOIR RENSEIGNEMENTS",
    "VER INFORMACI",
    "DECLARADO DESIERT",
    "QUEDA DESIERT",
    "NESSUNA AGGIUDICAZIONE",
];

/// Whole values that are no name (compared entire, so a company whose name merely
/// contains the word is untouched): the era's `Various`, and the Spanish and Italian
/// for a lot left void (issue 508: `Desierto` ×11 in the colonless sample).
const NAME_WHOLE_REJECTS: [&str; 5] = ["VARIOUS", "DESIERTO", "DESIERTA", "DESERTO", "DESERTA"];

/// Whether a candidate can be a company at all, before it is allowed to mint an
/// organization. Every rule here comes from a payload that would otherwise have minted
/// nonsense, and each is cheap enough to run per candidate:
///
/// - **it must contain a letter.** `6.  Supplier(s): 99.` is a real 1993 body (notice
///   21,123): under the supplies form that item sometimes holds the *number* of
///   suppliers rather than a name. `99` as an organization is worse than no winner.
/// - **it must not be the era's word for "no single answer".** `6.  Supplier(s): Various.`
///   appears four times in the committed 1993 daily alone. Compared whole, not as a
///   substring, so a company whose name contains the word is untouched.
/// - **it must not be one of [`NAME_REJECTS`]** — the withheld-value boilerplate.
fn plausible_name(name: &str) -> bool {
    if name.is_empty() || !name.chars().any(char::is_alphabetic) {
        return false;
    }
    if NAME_WHOLE_REJECTS.iter().any(|r| name.eq_ignore_ascii_case(r)) {
        return false;
    }
    if opens_with_contact_line(name) {
        return false;
    }
    !NAME_REJECTS.iter().any(|r| find_ascii_ci(name, r).is_some())
}

/// Whether a candidate is the tail of a contact block — `Fax 0044 2920 644615`,
/// `URL: www.puertomalaga.com. Fax 952 12 50 02` — rather than a name (issue 485).
///
/// Both strings were minted as organizations and served as winners: 19 of them in
/// notice 2808875 alone. The root cause is [`lot_prefix_len`] reading a phone number as
/// a lot reference, and that is fixed there; this is the second fence, so a contact
/// line that reaches a name slot some other way is still refused.
///
/// Narrow on purpose. `Fax`/`Tel`/`Telefax` count only when what follows (past any
/// `.`, `:` or space) is a number — `Tel Aviv Holdings` is a name. `URL`/`E-mail` count
/// only with their colon.
fn opens_with_contact_line(name: &str) -> bool {
    let bytes = name.as_bytes();
    for word in ["TELEFAX", "FAX", "TEL"] {
        if bytes.len() > word.len() && bytes[..word.len()].eq_ignore_ascii_case(word.as_bytes()) {
            let tail = name[word.len()..].trim_start_matches(['.', ':', ' ']);
            if tail.len() < name.len() - word.len()
                && tail.starts_with(|c: char| c.is_ascii_digit() || c == '+' || c == '(')
            {
                return true;
            }
        }
    }
    ["URL:", "E-MAIL:", "EMAIL:"]
        .iter()
        .any(|w| bytes.len() >= w.len() && bytes[..w.len()].eq_ignore_ascii_case(w.as_bytes()))
}

/// Labels under which the numbered form states what the contract cost (issue 244).
/// Counted over `fetch 300`'s 13,734 bodies: `Price:` 3,081, `Value of winning award…` 483,
/// `Contract value:` 25 — against 4,153 TD:7 award records, so the money is stated about
/// as often as the winner is.
const VALUE_LABELS: [&str; 5] =
    ["PRICE:", "PRICE(S):", "CONTRACT VALUE:", "VALUE OF WINNING AWARD", "TOTAL FINAL VALUE"];

/// Where a value item ends in the SECTIONED form, which does not end its items with a
/// number (issue 244 slice 7).
///
/// The 2005-2010 vintage writes the award value as prose that continues past the figure:
///
/// ```text
///     Total final value of the contract: Value: 791 805 EUR. Excluding VAT.
///     Total final value of contract(s): Value: 39 279 748,48 PLN. Including VAT.
///                                      VAT rate (%): 22,00 %.
/// ```
///
/// `next_item_marker` cannot bound that — there is no ` <n>. ` — and the sub-label retry
/// would strip to after the LAST colon, which in the second body is `VAT rate (%):` and
/// loses the figure entirely. So the value ends at whichever of these comes first, and
/// when it ends at a VAT phrase, that phrase IS the basis: measured on `fetch 200`
/// (2009-10), `Total final value` appears in 9,549 of the package's 11,943 award notices
/// against 53 for `Price:`, so this is where the era's money actually is.
///
/// `SECTION ` is deliberately the bare word rather than `SECTION V` (slice 9). The
/// aggregate at `II.2.1` sits BEFORE section IV, so the heading that follows it is often
/// `SECTION IV: PROCEDURE` — 52 of `fetch 200`'s 1,904 remaining refusals were bodies
/// whose figure ran on into `IV.1.1) Type of procedure: Open.` and lost to the sub-label
/// retry. Any section heading ends a value; none of them is ever part of one.
const VALUE_STOPS: [(&str, Option<&str>); 5] = [
    ("EXCLUDING VAT", Some("excl")),
    ("INCLUDING VAT", Some("incl")),
    ("SECTION ", None),
    ("CONTRACT NO", None),
    ("AWARD OF CONTRACT", None),
];

/// The words that mark a total as the NOTICE's rather than one contract's (issue 244
/// slice 8).
///
/// The sectioned form states the same kind of fact at two scopes:
///
/// ```text
///     II.2.1)  Total final value of contract(s): Value: 81 605 403,00 SEK.
///     ...
///     CONTRACT NO: 1  V.4)  ... Total final value of the contract: Value: 40 087 596,00 SEK.
///     CONTRACT NO: 2  V.4)  ... Total final value of the contract: Value: 15 605 000,00 SEK.
///     CONTRACT NO: 3  V.4)  ... Total final value of the contract: Value: 14 700 772,00 SEK.
///     CONTRACT NO: 4  V.4)  ... Total final value of the contract: Value: 11 212 035,00 SEK.
/// ```
///
/// That is notice 3871014, and it settles what the relationship is: the four contract
/// figures sum to 81 605 403 — EXACTLY the `II.2.1` figure. So a body stating both is not
/// stating one fact twice and contradicting itself; it is stating a total and its parts.
/// `TED-VAL_TOTAL` is a notice-scope field, so the aggregate is the one to claim, and the
/// per-contract figures are a `lot_results`-scope fact this does not yet have a home for.
///
/// Measured on `fetch 200` after slice 7: of 3,656 bodies that state `Total final value`
/// and still yield no amount, **1,959 state the aggregate with a figure** — the single
/// largest remaining class, and every one of them was refused as a self-contradiction.
///
/// The plural is the whole signal: `of contract(s)` is II.2.1, `of the contract` is V.4.
const AGGREGATE_SCOPE: &str = "OF CONTRACT(S)";

/// How the sectioned form heads each contract it awards, and therefore how many contracts
/// a body awards (issue 244 slice 8). Counting these is what separates a notice whose
/// single `V.4` figure IS its total from one where that figure is a part.
///
/// The leading newline is load-bearing. `CONTRACT NO` is a heading in the sectioned form
/// but a REFERENCE in the pre-2004 numbered one, which writes it inside item 6 —
/// `6. Successful contractor(s): Contract No 710-7009: AS Anlegg, Arvid` (notice 1710588)
/// — and some bodies mention it twice. Counted as a bare substring, that reads as two
/// contracts and drops the notice's only price: measured over `fetch 300`, 7 of its 996
/// amounts. Counted at line starts only, the numbered form scores 0 in all 996 bodies,
/// while the sectioned band still flags 2,154 of `fetch 200`'s 9,549 — against 2,317 for
/// the bare count, and 159 of that difference are bodies with ONE heading plus a mid-line
/// mention, i.e. single-contract notices the bare count would have refused.
const CONTRACT_MARKER: &str = "\nCONTRACT NO";

/// How a sectioned body heads an award that it numbers by lot instead of by contract
/// (issue 508). Line starts only, for the reason [`CONTRACT_MARKER`] gives.
const LOT_MARKER: &str = "\nLOT NO";

/// The sectioned form's winner heading, printed once per award (V.3 from 2006 on), for a
/// body that heads its awards with neither [`CONTRACT_MARKER`] nor [`LOT_MARKER`] (issue
/// 508). Counted on the flattened body, so a wrap inside it does not hide it.
const AWARD_BLOCK_HEADING: &str = "NAME AND ADDRESS OF ECONOMIC OPERATOR";

/// How many times a line-start heading marker occurs, not counting an occurrence that a
/// letter continues: the 2010 print puts IV.3.2's previous-publication type on a line of
/// its own (`Contract notice`), which a bare count of `\nCONTRACT NO` reads as a second
/// contract heading and so drops a single contract's only figure (notice 4,041,078), and
/// `Lot not awarded` is no `LOT NO` heading either. `CONTRACT NO:`, `CONTRACT No 001` and
/// `LOT NO:  - TITLE` still count.
fn count_heading(body: &str, marker: &str) -> usize {
    let mut n = 0;
    let mut at = 0;
    while let Some(i) = find_ascii_ci(&body[at..], marker) {
        let end = at + i + marker.len();
        if !body.as_bytes().get(end).is_some_and(u8::is_ascii_alphabetic) {
            n += 1;
        }
        at = end;
    }
    n
}

/// How a monetary value must be written to be claimed at all.
///
/// This is deliberately the strictest reading of the shapes on prod, because a wrong
/// amount is worse than a missing one — it lands in `tender_version_amounts` as a
/// `result_value` and nothing downstream can tell it from a published figure. The
/// measured shapes, and what happens to each:
///
/// ```text
///     2 143 000 EUR.                                    claimed
///     5 301 802,22 FRF TTC.                             refused — `TTC` is a tax basis
///     562 680 GBP p.a.                                  refused — annual, not a total
///     15 564 000 ATS / 1 131 079,99 EUR.                refused — two currencies
///     Minimum/maximum: Lit 2 610/Lit 3 289.             refused — a range
///     Lit 1 000 000 000.                                refused — `Lit` is not a code
///     Publication of this information would prejudice…  refused — withheld
/// ```
///
/// So: exactly one number and exactly one three-letter upper-case currency code, in
/// either order, and NOTHING else in the value but a closing period. A qualifier that
/// changes what the number means (`p.a.`) and one that changes its basis (`TTC`, `HT`)
/// are both refused rather than silently mixed into one column with figures that carry
/// neither. Whatever this refuses stays where it already was — inside the `TXT-TX` prose
/// that is claimed as a whole — so refusing costs a fact and never exhaustiveness
/// (ADR-0004).
fn parse_money(value: &str) -> Option<(i64, String, Option<&'static str>)> {
    let value = value.trim().trim_end_matches('.').trim();
    if value.is_empty() || value.len() > 64 {
        return None;
    }
    let mut whole: Option<i64> = None; // units, accumulated across thousands groups
    let mut fraction: Option<i64> = None; // the cents, once a decimal group is seen
    let mut currency: Option<String> = None;
    let mut basis: Option<&'static str> = None;
    for token in value.split(' ').filter(|t| !t.is_empty()) {
        // Tax markers are tested BEFORE currency codes, because `TTC` is three
        // upper-case letters and would otherwise read as a second currency and refuse
        // the whole value — which is exactly what it did until this was measured.
        if let Some(marked) = tax_marker(token) {
            if basis.replace(marked).is_some_and(|seen| seen != marked) {
                return None; // both bases claimed at once
            }
            continue;
        }
        if is_currency_code(token) {
            if currency.replace(token.to_owned()).is_some() {
                return None; // two codes — a dual-currency restatement
            }
            continue;
        }
        // A decimal group ends the number: anything after it is a second figure.
        if fraction.is_some() {
            return None;
        }
        let (digits, group_fraction) = digit_group(token)?;
        // Every group after the first is a thousands group and must be exactly three
        // digits, so `2 143 000` groups and `2 14 3000` refuses.
        if whole.is_some() && digits.len() != 3 {
            return None;
        }
        let group: i64 = digits.parse().ok()?;
        whole = Some(match whole {
            Some(n) => n.checked_mul(1000)?.checked_add(group)?,
            None => group,
        });
        fraction = group_fraction;
    }
    if whole.is_some_and(|units| run_together_range(units, fraction)) {
        return None; // a minimum and a maximum, not one figure (issue 491)
    }
    match (whole, currency) {
        (Some(units), Some(code)) => {
            let cents = units.checked_mul(100)?.checked_add(fraction.unwrap_or(0))?;
            (cents > 0).then_some((cents, code, basis))
        }
        _ => None,
    }
}

/// Whether a figure is really a purchase-order framework's minimum and maximum printed
/// side by side (issue 491).
///
/// French award notices of 2005–2008 write a bons-de-commande range as two numbers in one
/// value. The grouping rule above cannot tell them from one figure, and the same body's
/// V.4 restates the pair with the spaces gone:
///
/// ```text
///     II.2.1) … Value: 40 000 120 000 EUR.   min 40 000, max 120 000   was claimed as €40 bn
///     V.4)    … Value: 40000120000 EUR.      the same pair, unspaced  (CAN 26854-2007)
///     Value: 87 250 349 000 EUR.             min 87 250, max 349 000   was claimed as €87 bn
///     Value: 5 500 22 000. EUR               `22` is not a thousands group  already refused
/// ```
///
/// So the test reads the NUMBER, not its spacing: Y = the last six digits (the maximum,
/// at least 100 000), X = the digits before them (the minimum). French law (CMP 2001/2004)
/// caps the maximum at four times the minimum, so the figure is a range when X < Y ≤ 4X
/// and the print is the range's own: Y exactly 4X, or X and Y both whole thousands. Y ≥
/// 100 000 and Y ≤ 4X force X ≥ 25 000, so every figure this matches is at least
/// 25 000 100 000 units, and an ordinary-sized figure can never match. Measured on the
/// number over all 3,786,955 text notices (spaced or not): 25 EUR notices, all from
/// 2005–2008. The 12 non-round lira and forint figures with the same raw arithmetic
/// (e.g. LIT 57 157 228 327) are ordinary published figures and stay claimed.
fn run_together_range(units: i64, fraction: Option<i64>) -> bool {
    if fraction.is_some_and(|cents| cents != 0) {
        return false;
    }
    let (x, y) = (units / 1_000_000, units % 1_000_000);
    y >= 100_000 && x < y && y <= 4 * x && (y == 4 * x || (x % 1000 == 0 && y % 1000 == 0))
}

/// A standalone token that states whether the figure beside it includes tax.
///
/// Four, all measured on prod: the French `TTC` / `HT` pair and the Belgian `TVAC` /
/// `HTVA`. `TTC` is the reason this runs before the currency test — three upper-case
/// letters, indistinguishable from a code by shape alone.
///
/// The basis has no canonical destination yet (issue 251: `tender_version_amounts`
/// records no tax basis, and the corpus already mixes the two unlabelled). It is
/// captured in the parse layer anyway, so that when a destination exists the era does
/// not have to be re-parsed to find out what it already said.
fn tax_marker(token: &str) -> Option<&'static str> {
    let token = token.trim_end_matches(['.', ',']);
    match token {
        "TTC" | "TVAC" => return Some("incl"),
        "HT" | "HTVA" => return Some("excl"),
        _ => {}
    }
    // The German pair also appears as a bare word AFTER the figure, with no sub-label and
    // so no colon to retry past: `8.  Price: 8 600 000 DEM netto.` (prod, 2001-06). Before
    // this it was an unknown token and refused the whole value.
    if token.eq_ignore_ascii_case("netto") {
        return Some("excl");
    }
    if token.eq_ignore_ascii_case("brutto") {
        return Some("incl");
    }
    None
}

/// Phrases a local-language sub-label uses to state the basis, searched in the label
/// that stands between the price heading and the figure. Measured shapes:
/// `Auftragssumme (ohne Umsatzsteuer): 689 655,17 DEM.` and its `mit` twin, which
/// between them are most of the era's refused values.
const BASIS_PHRASES: [(&str, &str); 8] = [
    ("OHNE UMSATZSTEUER", "excl"),
    ("OHNE UST", "excl"),
    ("EXCLUDING VAT", "excl"),
    ("NETTO", "excl"),
    ("MIT UMSATZSTEUER", "incl"),
    ("MIT UST", "incl"),
    ("INCLUDING VAT", "incl"),
    ("BRUTTO", "incl"),
];

/// The basis a sub-label states, or `None` when it states neither or both.
fn phrase_basis(label: &str) -> Option<&'static str> {
    let mut found: Option<&'static str> = None;
    for (phrase, basis) in BASIS_PHRASES {
        if find_ascii_ci(label, phrase).is_some() {
            match found {
                Some(seen) if seen != basis => return None,
                _ => found = Some(basis),
            }
        }
    }
    found
}

/// A three-letter upper-case ASCII currency code (`EUR`, `FRF`, `ATS`, `GBP`). Not a
/// closed list: the era spans a dozen pre-euro currencies and this only has to
/// distinguish a code from a word — `Lit`, the Italian lira's own spelling, is
/// deliberately NOT one, and its shapes are ranges this refuses anyway.
fn is_currency_code(token: &str) -> bool {
    token.len() == 3 && token.chars().all(|c| c.is_ascii_uppercase())
}

/// One space-separated group of a written number: its digits, and the two decimal
/// digits if this group carried them. `802,22` is `("802", Some(22))`; `301` is
/// `("301", None)`. Three decimals would be sub-cent (ADR-0010) and one a typo, so
/// neither is a number this reads.
fn digit_group(token: &str) -> Option<(&str, Option<i64>)> {
    let (digits, fraction) = match token.split_once([',', '.']) {
        Some((whole, fraction)) => {
            // One or two decimal digits, and one digit means TENTHS (slice 9). The
            // earlier rule refused a single digit beside the sub-cent refusal, as though
            // `176 713,2` were as unreadable as `1 000,255` — but tenths are exactly
            // representable in cents and sub-cent amounts are not, which is the whole
            // distinction ADR-0010 draws. Comma-as-thousands stays refused, because a
            // thousands group is three digits and a three-digit fraction is not cents.
            // Measured: 114 of `fetch 200`'s 1,904 remaining refusals write one digit.
            if !matches!(fraction.len(), 1 | 2) || !fraction.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let scale = if fraction.len() == 1 { 10 } else { 1 };
            (whole, Some(fraction.parse::<i64>().ok()? * scale))
        }
        None => (token, None),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((digits, fraction))
}

/// How long a lot reference in front of a winner's name is allowed to be.
/// `1, 2, 3 and 4:` is the longest measured, at 14 characters.
const LOT_PREFIX_MAX: usize = 24;

/// Strip a leading lot reference from a winner value, so the *name* starts where the
/// name starts.
///
/// The 1993 supplies form keys each winner to the lots it won, and it is not a rare
/// shape — the committed `1993-daily-en-19930102` fixture uses it for roughly a third
/// of its winners, in every one of these spellings:
///
/// ```text
///     6.  Supplier(s): A: Apotecnia, Climo
///     6.  Supplier(s): 1: Ailsa Truck and Bus Limited, 101 Kelburn Street, …
///     6.  Supplier(s): 1/2: Evans MacShaw Leyland DAF Limited, Shefford Road, …
///     6.  Supplier(s): 1, 2: Carlier Chaines, 37/41, rue Roger Salengro, …
///     6.  Supplier(s): 1, 2, 3 and 4: Dolmen Computer Applications NV, …
///     6.  Supplier(s): 1: Baxter Healthcare; 2: B. Braun Medical; 3: Fresenius …
/// ```
///
/// Taken verbatim these mint `1: Ailsa Truck and Bus Limited` — a second spelling of a
/// company that also appears unprefixed, and since these winners carry no identifier the
/// name IS the identity (issue 234). Fail-closed was the earlier stance and it is worse:
/// it drops a third of the era's oldest winners rather than reading them.
///
/// A prefix qualifies only if everything before the terminator is lot-reference material
/// — digits, single letters, separators, and the word `and` — and short. Anything else is
/// left alone, so `ARGE: Walter-Bau-AG` and `Groupement solidaire: Entreprise Quille`,
/// both real consortium designations in the same fixture, keep their colons.
///
/// The terminator may be `.` instead of `:` (`6.  Supplier(s): 1. Poul Pedersen A/S`) —
/// but then the reference must be DIGITS. A single letter followed by a period is an
/// initial, not a lot: `H. Meyer GmbH` and `B. Braun Medical` are companies, and
/// stripping there would rename them.
///
/// The terminator may also be `)` — `6.  Supplier(s): 1) CGC, BP 129, … 2) Furic …`
/// (900123), `1) Biotronik France, … 2) Ela Médical, …` (2002409) — and then the
/// reference must be ONE run of digits: `(a)`/`a)` enumerate clauses, and a `)` after
/// anything else closes a parenthesis inside a name (issue 484 review).
fn lot_prefix_len(value: &str) -> Option<usize> {
    let (at, terminator) = value
        .char_indices()
        .take_while(|(i, _)| *i <= LOT_PREFIX_MAX)
        .find(|(_, c)| matches!(c, ':' | '.' | ')'))?;
    let head = &value[..at];
    if head.is_empty() {
        return None;
    }
    if terminator == ')' {
        return head.bytes().all(|b| b.is_ascii_digit()).then_some(at + 1);
    }
    // Two digit groups with nothing but whitespace between them are a phone number,
    // not a lot list (issue 485): `Tel. 952 12 50 00. URL: …` split at the `.` after
    // `Tel`, and the segment behind it minted `URL: www.puertomalaga.com. Fax 952 12 50
    // 02` as a winner. Every measured lot list separates its references with `,` `/`
    // `-` or `and` (`1, 2, 3 and 4:`, `1/2:`), never with a bare space.
    if digit_groups_space_separated(head) {
        return None;
    }
    let parts = head.split([',', '/', '-', ' ']).filter(|p| !p.is_empty());
    let mut any_letter = false;
    for part in parts {
        if part.eq_ignore_ascii_case("and") {
            continue;
        }
        if part.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if part.chars().count() == 1 && part.chars().all(|c| c.is_ascii_alphabetic()) {
            any_letter = true;
            continue;
        }
        return None;
    }
    // A lone letter before a period is an initial, not a lot reference.
    if terminator == '.' && any_letter {
        return None;
    }
    Some(at + terminator.len_utf8())
}

/// Whether `head` holds two runs of digits separated only by spaces — phone notation.
fn digit_groups_space_separated(head: &str) -> bool {
    let bytes = head.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let gap = i;
            while i < bytes.len() && bytes[i] == b' ' {
                i += 1;
            }
            if i > gap && i < bytes.len() && bytes[i].is_ascii_digit() {
                return true;
            }
        } else {
            i += 1;
        }
    }
    false
}

fn strip_lot_prefix(value: &str) -> &str {
    match lot_prefix_len(value) {
        Some(n) => value[n..].trim_start(),
        None => value,
    }
}

/// How far into a value a `CONTRACT NO <ref>:` prefix may run before the winner's
/// name. The measured shape is `Contract No 04/2004/OIL:` — a file reference, well
/// under this — and the bound is what keeps a stray `CONTRACT NO` mid-sentence from
/// swallowing half the window hunting for a colon.
const CONTRACT_NO_PREFIX_MAX: usize = 48;

/// The length of a leading `CONTRACT NO <ref>:` prefix on a winner value, or 0.
///
/// The 2004/2005 sectioned form numbers each award INSIDE the value — the residue
/// read's specimen (notice 2,368,067) prints
/// `V.1.1) Name and address of the successful … provider: Contract No 04/2004/OIL:
/// Martin Reinert Sàrl, …` — and `CONTRACT NO` is also an [`ITEM_STOPS`] entry (it is
/// the boundary BETWEEN awards in the multi-contract shape), so without this hop the
/// value ends before it begins and a published, extractable winner reads as no winner
/// at all (issue 244 slice 9). The hop is taken only when the marker OPENS the value:
/// mid-window occurrences keep their boundary meaning.
fn contract_no_prefix_len(rest: &str) -> usize {
    const MARKER: &str = "CONTRACT NO";
    let spaces = rest.len() - rest.trim_start_matches(' ').len();
    let after = &rest[spaces..];
    if after.len() < MARKER.len() || !after.as_bytes()[..MARKER.len()].eq_ignore_ascii_case(MARKER.as_bytes())
    {
        return 0;
    }
    let tail = &after[MARKER.len()..];
    let bound = char_bound(tail, CONTRACT_NO_PREFIX_MAX);
    // The reference runs to a colon and never contains a comma — a comma first means
    // this is not a reference but prose, and the hop does not apply.
    match tail[..bound].find(':') {
        Some(colon) if !tail[..colon].contains(',') => spaces + MARKER.len() + colon + 1,
        _ => 0,
    }
}

/// Split one winner value into one segment per winner.
///
/// Two separators, both measured in the committed 1993 daily. `;` is the utilities
/// form's (`BP, Hamburg; Thelen, Mainz.`). The supplies form instead ends each entry
/// with a period and opens the next with its lot reference:
///
/// ```text
///     6.  Supplier(s): 1: Discol. 2: Rault. 3: Discol. … 14: Sarl Fuseau
/// ```
///
/// Fourteen winners in one item. Read as one value that is a 150-character "company"
/// name; read as fourteen it is fourteen organizations, which is what the payload says.
/// A period only separates when a lot reference follows it — otherwise it is the end of
/// a sentence or of an abbreviation, and the value stays whole.
fn winner_segments(value: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let bytes = value.as_bytes();
    let (mut start, mut i) = (0usize, 0usize);
    while i < bytes.len() {
        if bytes[i] == b';' || bytes[i] == b'.' {
            let mut next = i + 1;
            while next < bytes.len() && bytes[next] == b' ' {
                next += 1;
            }
            // `;` separates on its own; `.` only in front of a lot reference, or where
            // a contact block ends and the next entry's own `Name, address … Tel.` begins.
            let separates = if bytes[i] == b';' {
                true
            } else {
                next > i + 1 && (lot_prefix_len(&value[next..]).is_some() || entry_after_contact(value, i, next))
            };
            if separates && next > start {
                segments.push(&value[start..i]);
                start = next;
                i = next;
                continue;
            }
        }
        i += 1;
    }
    segments.push(&value[start..]);
    segments
}

/// Whether the `.` at `dot` ends a contact block (the token before it is an e-mail
/// address or a `www.`/`http` host) and what starts at `next` is another entry: an
/// upper-case `Name,` with no `:` before its comma within [`ITEM_LOOKBACK`] bytes, whose
/// own text carries a phone, fax or e-mail. 2002408 prints two unkeyed entries —
/// `Gobierno Vasco, … URL: www.ej-gv.net. Profinsa, Productos de Oficina e Informática,
/// … Tel.: 945-22 22 69. Fax: …` — and without this split Profinsa, the real supplier,
/// is the tail of the authority's segment and never read (issue 484 review). Each
/// condition keeps a contact block's own continuation (`… x@y.es. Fax 952 …`,
/// `… www.x.es. Internet address: …`) from passing for an entry.
fn entry_after_contact(value: &str, dot: usize, next: usize) -> bool {
    let token = value[..dot].rsplit(' ').next().unwrap_or("").trim_start_matches('(').to_ascii_lowercase();
    if !(token.contains('@') || token.starts_with("www.") || token.starts_with("http")) {
        return false;
    }
    let tail = &value[next..];
    if !tail.starts_with(char::is_uppercase) {
        return false;
    }
    let head = &tail[..char_bound(tail, ITEM_LOOKBACK)];
    match head.find(',') {
        Some(comma) if !head[..comma].contains(':') => {}
        _ => return false,
    }
    ["TEL", "FAX", "@"].iter().any(|m| find_ascii_ci(tail, m).is_some())
}

/// Drop the period that ends the sentence, and keep the one that ends an
/// abbreviation (issue 244).
///
/// The value is a sentence fragment, so `Grahams Engineering Ltd.` wants its trailing
/// dot gone. But `European Dynamics … Telematics S.A.` does not: trimming there gives
/// `… S.A`, a second spelling of a company that also appears written out, and since
/// these winners carry no identifier their canonical identity IS the name — two
/// spellings are two organizations (issue 234).
///
/// The discriminator is what precedes the dot: a single letter means an abbreviation
/// mid-run (`S.A.`, `A.G.`, `S.p.A.`), anything longer means a word that happened to
/// end the fragment (`Ltd.`, `GmbH.`).
fn trim_sentence_period(name: &str) -> &str {
    let Some(head) = name.strip_suffix('.') else { return name };
    // TRIMMED, because the letter is often preceded by a space: the campaign's own output
    // showed `Hurtownia Farmaceutyczna Ismed Sp. J.` stored as `… Sp. J` — the Polish legal
    // form `sp. j.` written with a space — while `Balton Spółka z o.o.` came through intact.
    // Two spellings of one company are two organizations in a layer with no identifier to
    // merge on (issue 234), so the discriminator has to see `J`, not ` J`.
    let last = head.rsplit('.').next().unwrap_or(head).trim();
    if last.chars().count() == 1 { name } else { head.trim_end() }
}

/// The winner names a 2004-or-later award body publishes, in document order
/// (issue 244).
///
/// The body is the era's own prose — `TXT-TX`, claimed whole and left authoritative;
/// these names are a DERIVED claim on top of it, which is what lets the projection
/// see a text-era award at all (the era publishes no result section, so before this
/// all 1,306,514 of its award notices materialised nothing).
///
/// Newlines collapse to spaces first, because TED's wrapper breaks both the heading
/// and the value at ~72 columns — `HAS BEEN \nAWARDED:` is the common case, and a
/// line-oriented match would miss most of the corpus.
///
/// One name per label occurrence, and the occurrences repeat: a notice awarding four
/// contracts prints the heading four times, each under its own `CONTRACT NO:`
/// (measured: 25% of a 2008 window carries more than one, up to 18). Only the name
/// is taken, not the address that follows it — an address-bearing string mints a new
/// organization for every spelling variation of the same company ("Zac Satolas Green"
/// vs "Zac de Satolas Green", both Stryker France, in one notice), which is issue
/// 234's problem manufactured on purpose.
fn awarded_names(body: &str) -> Vec<String> {
    // Gate before allocating anything. Every award body says `AWARD` at least twice —
    // in `SECTION V: AWARD OF CONTRACT` and again in the label — and a body that says
    // it nowhere cannot carry either label. This matters because the era is 3.8M
    // notices: measured on prod, doing the flatten-and-scan unconditionally made a
    // package's re-parse SEVEN TIMES slower (1,012 members in 148 s became 1 member a
    // second, CPU-pegged), which is most of an era's re-parse spent on notices that
    // are not awards at all.
    if find_ascii_ci(body, "AWARD").is_none() && find_ascii_ci(body, "PROVIDER").is_none() {
        return Vec::new();
    }
    let flat = flatten(body);
    let authority = awarding_authority(&flat);

    let mut names = Vec::new();
    let mut at = 0usize;
    while at < flat.len() {
        // The earliest label from here, so the two vintages can both appear (they do
        // not in any measured notice, but a mixed body would otherwise skip awards).
        let Some((start, label)) = AWARD_LABELS
            .iter()
            .filter_map(|l| find_ascii_ci(&flat[at..], l).map(|i| (at + i, *l)))
            .min_by_key(|(i, _)| *i)
        else {
            break;
        };
        let value_at = start + label.len();
        let rest = &flat[value_at..];
        // Hop a leading `Contract No <ref>:` before bounding the value — see
        // `contract_no_prefix_len` for why this must run before the ITEM_STOPS scan.
        let rest = &rest[contract_no_prefix_len(rest)..];
        // Where the value ends. Two shapes (issue 484):
        //
        // - the pre-2004 NUMBERED form (`6.  Supplier(s): …`) ends the value at its own
        //   successor item ` 7. `, looked for within `NUMBERED_VALUE_MAX`. One numbered
        //   value can carry several lot-keyed entries with full contact blocks — notice
        //   2002406 prints the authority's ~330-byte contact entry as `1:` and the real
        //   supplier, Montte, as `2:` — so the 256-byte window cut Montte off and served
        //   the authority as its own contractor. The successor, not a blanket larger
        //   window, is the bound: a bare 2 KiB would run on into ` 8.  Price(s): 1:
        //   2 750 000 SEK` (1200610) and read the price list as winners.
        // - everything else, and a numbered value whose successor is not found, keeps
        //   the WINDOW: scanning the whole remainder made the function quadratic in
        //   (awards × body length), and a notice awarding hundreds of contracts then
        //   took minutes (`fetch 186`: 148 s to under two members a minute). A winner's
        //   name is never 8 kB from its own label.
        //
        // Inside the value, `;` separates winners and `,` ends each name:
        //
        //     9.  Supplier(s), …: BP, Hamburg; Thelen, Mainz.   10.  …
        //                         ^^          ^^^^^^            ^^^^ item stop
        //
        // Reading that as one name would mint `BP, Hamburg; Thelen` as an organization,
        // which is the withheld-boilerplate failure in a new costume (issue 234).
        //
        // The successor wins over the numeric ITEM_STOPS (` 7.` ` 9.` ` 10.`): those are
        // guesses at the successor and also match an address (`UK-Sheffield 9.
        // Edmundson`), and with the successor missing (numbered forms skip items:
        // 2002406 has no 4) a ` 9.` a kilobyte out would run the value over the items
        // between — `8.  Price(s): 1: STERLING 400 000` read as a winner. So: the next
        // item marker of ANY higher number (n+1 first, up to n+4), cut earlier only by
        // a sectioned stop; no marker found means the old window, unchanged.
        let numbered = numbered_item_before(&flat, start).and_then(|n| {
            let wide = &rest[..char_bound(rest, NUMBERED_VALUE_MAX)];
            let succ = successor_marker(wide, n)?;
            let stop = SECTIONED_STOPS.iter().filter_map(|stop| find_ascii_ci(wide, stop)).min();
            Some(&wide[..stop.map_or(succ, |stop| stop.min(succ))])
        });
        let (value, item_end) = match numbered {
            Some(value) => (value, Some(value.len())),
            None => {
                let window = &rest[..char_bound(rest, NAME_WINDOW)];
                let item_end = ITEM_STOPS.iter().filter_map(|stop| find_ascii_ci(window, stop)).min();
                (&window[..item_end.unwrap_or(window.len())], item_end)
            }
        };
        names.extend(names_in_value(value, item_end.is_some(), authority.as_ref()));
        at = value_at;
    }
    names.extend(line_awarded_names(body, authority.as_ref()));
    names
}

/// The winner names one value states, in order: each [`winner_segments`] entry read up
/// to its first comma. `bounded` says whether something other than the window cut the
/// value — an item stop, a successor marker, a line end — which is what makes a
/// comma-less entry safe to take whole.
fn names_in_value(value: &str, bounded: bool, authority: Option<&Authority>) -> Vec<String> {
    // (name, whether the segment is the authority's own contact entry)
    let mut found: Vec<(String, bool)> = Vec::new();
    for segment in winner_segments(value) {
        let segment = segment.trim_start();
        // A contract reference is not a name (issue 484), but what follows it may be:
        // 2002406 opens its value with `Contrato n° S-036/02-DJ.` as a segment of its
        // own, 2002408 with `Contrato n° S-037/02-DJ. Gobierno Vasco, …` and 2002409
        // with `Marché n° 03/010002: 1) Biotronik France, …`. Hop the reference; a
        // segment that is nothing else is skipped.
        let segment = match contract_reference_len(segment) {
            Some(len) => segment[len..].trim_start(),
            None => segment,
        };
        if segment.is_empty() {
            continue;
        }
        // The name ends at the first comma — the address follows it in every
        // measured shape. With no comma, the name is the whole segment, which is
        // safe only because the item stop already bounded it: with NEITHER boundary
        // the segment is the raw 256-byte window, and a "name" that long mints one
        // organization per notice and poisons an identity that has no identifier to
        // fall back on. Refuse it instead.
        let segment = strip_lot_prefix(segment);
        let (name, bounded) = match segment.find(',') {
            Some(comma) => (&segment[..comma], true),
            None => (segment, bounded),
        };
        if !bounded {
            continue;
        }
        let name = trim_sentence_period(name.trim());
        if plausible_name(name) {
            let contact = authority.is_some_and(|a| a.contact_entry(name, segment));
            found.push((name.to_owned(), contact));
        }
    }
    // Drop the authority's own contact entry — and only when another named entry
    // survives in the same value (issue 484). A single entry naming the buyer is what
    // the notice says (3002722, 3009398: the publisher repeated its own block in
    // V.3), and serving it as published is the projection's call, not this one's.
    let others = found.iter().filter(|(_, contact)| !contact).count();
    found.into_iter().filter(|(_, contact)| others == 0 || !contact).map(|(n, _)| n).collect()
}

/// The heading the 2010 print ends its line with (issue 508). It is
/// [`AWARD_LABELS`]' `HAS BEEN TAKEN:` with the colon gone.
const LINE_AWARD_HEADING: &str = "HAS BEEN TAKEN";

/// The winner names of the 2010 print, which drops the colons (issue 508):
///
/// ```text
///     V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT
///     AWARD DECISION HAS BEEN TAKEN
///     Clarke Machinery Ltd.
///     New Inn, Ballyjamesduff, Co. Cavan
///     IRELAND
///     V.4)  INFORMATION ON VALUE OF CONTRACT
/// ```
///
/// That is notice 4,200,000, and the print runs from the 2010-03 package (first body notice
/// 4,023,765) to the era's end on 2010-12-31: about 121,000 award notices, under 1 % of
/// which carried a winner. The address follows
/// the name on LINES of its own, not after a comma, so the flattened body that
/// [`awarded_names`] scans cannot tell where the name ends: `Clarke Machinery Ltd. New
/// Inn` would be its name. So this reads the unflattened body: a line that ends in
/// [`LINE_AWARD_HEADING`] names its winner on the next non-empty line, which then goes
/// through [`names_in_value`] like any other value, line end as its bound.
///
/// Measured over 10,339 such headings (windows of 2010-03, -05 and -11): every one ends
/// its line and 2 have no line after them. Two shapes of that next line need care:
///
/// - **A bare country.** A publisher who leaves the name and address blank still gets
///   the address block's country printed: `…HAS BEEN TAKEN` / `GERMANY` / `V.4)` (notice
///   4,201,806). Taken as a name, every such slot in the era folds into ONE nameless-
///   identity organization per country — 198 awards across the colonless windows, 80 of
///   them `GERMANY` — so a line that is only a country names nobody.
/// - **A wrapped name.** About 1 % of names fill the ~72-column wrap and continue on the
///   next line (`…Centrum Badawczo-Konstrukcyjne Obrabiarek Sp.` / `z o.o.`, notice
///   4,101,615). Cut at the wrap, each mints a second spelling of an identifier-less
///   company (issue 234). [`wrapped_continuation`] joins the next line only where the
///   wrapper must have broken the name.
fn line_awarded_names(body: &str, authority: Option<&Authority>) -> Vec<String> {
    // Gate before collecting lines: the line pass runs on every award body of a
    // 3.8M-notice era, and only the 2010 print carries the heading.
    if find_ascii_ci(body, LINE_AWARD_HEADING).is_none() {
        return Vec::new();
    }
    let lines: Vec<&str> = body.lines().map(str::trim).collect();
    let width = wrap_width(&lines);
    let mut names = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i].as_bytes();
        i += 1;
        let n = LINE_AWARD_HEADING.len();
        if line.len() < n || !line[line.len() - n..].eq_ignore_ascii_case(LINE_AWARD_HEADING.as_bytes()) {
            continue;
        }
        while i < lines.len() && lines[i].is_empty() {
            i += 1;
        }
        let Some(&next) = lines.get(i) else { break };
        // The publisher left the slot empty: the next item follows at once, or only the
        // address block's country was printed.
        if opens_an_item(next) || is_country(next) {
            continue;
        }
        i += 1;
        match wrapped_continuation(next, lines.get(i).copied(), width) {
            Some(rest) => names.extend(names_in_value(&format!("{next} {rest}"), true, authority)),
            None => names.extend(names_in_value(next, true, authority)),
        }
    }
    names
}

/// The width the body is wrapped at, or `None` when it is not wrapped. TED's wrapper
/// breaks the 2010 print at about 72 columns in most packages (its longest lines run
/// 72–80 characters) and not at all in others, whose headings run past 100 on one line.
fn wrap_width(lines: &[&str]) -> Option<usize> {
    let widest = lines.iter().map(|l| l.chars().count()).max()?;
    (widest <= 80).then_some(widest)
}

/// The line that continues a wrapped name, when the wrapper must have broken it there:
/// the body is wrapped, the first word of `following` could NOT have fitted on the name's
/// line (so the wrapper, not the publisher, ended it), and `following` reads as more name
/// rather than an address — no item, no country, no digit, no street opener.
///
/// Measured on the colonless sample (issue 508 review): the join repairs about 100 of 139
/// comma-less names that fill the line, and takes 2 street lines without a digit or an
/// opener (`routes des Gatines`, a Greek locality) against the ~31 an unconditional join
/// would take.
fn wrapped_continuation<'a>(name: &str, following: Option<&'a str>, width: Option<usize>) -> Option<&'a str> {
    let width = width?;
    let following = following?;
    let first = following.split(' ').next().unwrap_or("");
    if first.is_empty() || name.chars().count() + 1 + first.chars().count() <= width {
        return None;
    }
    if opens_an_item(following) || is_country(following) || following.bytes().any(|b| b.is_ascii_digit()) {
        return None;
    }
    let opens_street = STREET_OPENERS.iter().any(|w| {
        following.len() >= w.len() && following.as_bytes()[..w.len()].eq_ignore_ascii_case(w.as_bytes())
    });
    (!opens_street).then_some(following)
}

/// Words an address line of the 2010 print opens with, measured on the colonless sample's
/// lines after a name: Polish `ul.`/`al.`/`pl.`, Italian `via`/`viale`, French `rue`/
/// `avenue`/`route`, Spanish `calle`/`c/`, German `Str.`, Dutch `Postbus`, French `BP`.
const STREET_OPENERS: [&str; 13] =
    ["UL.", "AL.", "PL.", "VIA ", "VIALE ", "RUE ", "AVENUE ", "ROUTE ", "CALLE ", "C/", "STR.", "POSTBUS", "BP "];

/// Whether a line is nothing but a country's name (`GERMANY`, `United Kingdom.`).
fn is_country(line: &str) -> bool {
    let line = line.trim_end_matches('.').trim();
    crate::countries::NAME_TO_ALPHA2.iter().any(|(name, _)| name.eq_ignore_ascii_case(line))
}

/// Whether a line of the sectioned form opens an item or a heading rather than holding
/// a value: `V.4)  INFORMATION ON VALUE OF CONTRACT`, `SECTION VI: …`, `CONTRACT NO: 2`,
/// `LOT NO: 2`.
fn opens_an_item(line: &str) -> bool {
    let b = line.as_bytes();
    if b.len() >= 3 && b[0] == b'V' && b[1] == b'.' && b[2].is_ascii_digit() {
        return true;
    }
    ["SECTION ", "CONTRACT NO", "LOT NO"]
        .iter()
        .any(|m| b.len() >= m.len() && b[..m.len()].eq_ignore_ascii_case(m.as_bytes()))
}

/// How far a NUMBERED-form value is followed looking for its successor item (issue
/// 484). The longest measured value is 2002406's item 6, two lot entries with full
/// contact blocks at ~560 bytes; the bound is what keeps the scan linear.
const NUMBERED_VALUE_MAX: usize = 2048;

/// How far before a label its item number is looked for. `9.  Supplier(s),
/// contractor(s) or service provider(s):` puts 46 bytes of heading between the number
/// and the matched tail.
const ITEM_LOOKBACK: usize = 96;

/// The number `N` of the numbered form item whose heading contains the label at
/// `label_at` — `6` for `… 6.  Supplier(s): …` — or `None` when the label is not the
/// heading of a numbered item (the sectioned forms: `V.3)`, `V.1.1)`).
///
/// The marker is ` N. ` with one or two digits, preceded by a space (or the start of the
/// body), and nothing between it and the label may contain `:` or `.` — so a date
/// (`30. 8. 1999.`) or a sentence before the heading never passes for its number.
fn numbered_item_before(flat: &str, label_at: usize) -> Option<u32> {
    let from = char_bound(flat, label_at.saturating_sub(ITEM_LOOKBACK));
    let look = &flat[from..label_at];
    let bytes = look.as_bytes();
    let mut last: Option<(u32, usize)> = None;
    for i in 0..bytes.len() {
        let preceded = if i == 0 { from == 0 } else { bytes[i - 1] == b' ' };
        if !preceded || !bytes[i].is_ascii_digit() {
            continue;
        }
        let mut j = i;
        while j < bytes.len() && j - i < 2 && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if bytes.get(j) == Some(&b'.') && matches!(bytes.get(j + 1), Some(b' ') | None) {
            if let Ok(n) = look[i..j].parse::<u32>() {
                last = Some((n, (j + 2).min(bytes.len())));
            }
        }
    }
    let (n, end) = last?;
    let between = &look[end..];
    (!between.contains([':', '.'])).then_some(n)
}

/// Where the numbered item ` n. ` begins in `window`, or `None`. Also matches ` n.` at
/// the very end, which is how an empty last item flattens.
fn find_item_marker(window: &str, n: u32) -> Option<usize> {
    let marker = format!(" {n}.");
    let mut from = 0;
    while let Some(i) = window[from..].find(&marker) {
        let at = from + i;
        let after = at + marker.len();
        if after == window.len() || window.as_bytes()[after] == b' ' {
            return Some(at);
        }
        from = after;
    }
    None
}

/// Where the item after `n` begins in `window`: ` {n+1}. ` if present, else the next
/// higher number up to `n+4` (the numbered forms skip empty items — 2002406 has no
/// item 4, 1200610 jumps from 1 to 6). Lower numbers are tried first, so an address
/// number further out never wins over the real successor.
fn successor_marker(window: &str, n: u32) -> Option<usize> {
    (n + 1..=n + 4).find_map(|k| find_item_marker(window, k))
}

/// The [`ITEM_STOPS`] that are headings of the sectioned forms, not guesses at a
/// numbered successor. Inside a numbered value bounded by its successor only these
/// cut it shorter (issue 484).
const SECTIONED_STOPS: [&str; 5] = ["V.1.2)", "V.2)", "V.3)", "V.4)", "CONTRACT NO"];

/// The openings of a contract reference in a winner value (issue 484), lower-cased.
/// `CONTRACT NO <ref>:` at the very start of the value is already hopped by
/// [`contract_no_prefix_len`]; these are the same reference in the era's other
/// languages, and the English one without its colon, at the start of a SEGMENT
/// (`Contrato n° S-036/02-DJ.` in 2002406, `Marché n° 03/010002:` in 2002409).
const CONTRACT_REFERENCES: [&str; 18] = [
    "contrato n°",
    "contrato nº",
    "contrato no",
    "contrato n.",
    "contract no",
    "contrat n°",
    "contrat nº",
    "contrat no",
    "marché n°",
    "marché nº",
    "marché no",
    "marche n°",
    "marche no",
    "vertrag nr",
    "contratto n.",
    "contratto n°",
    "contratto nº",
    "contratto no",
];

/// The length of a contract reference opening `segment`, through its terminator — the
/// first `:`, or a `.` that ends the sentence — or the whole segment when no
/// terminator follows within [`CONTRACT_NO_PREFIX_MAX`] (the segment is only a
/// reference). `None` when the segment does not open with one; the opening must not run
/// on into a word (`Contract Northern Ltd` is a name), and a comma before the
/// terminator means prose, not a reference, so the whole segment goes.
fn contract_reference_len(segment: &str) -> Option<usize> {
    let r = CONTRACT_REFERENCES.iter().find(|r| {
        segment.get(..r.len()).is_some_and(|head| head.to_lowercase() == **r)
            && segment[r.len()..].chars().next().is_none_or(|c| !c.is_alphabetic())
    })?;
    let at = r.len() + (segment[r.len()..].len() - segment[r.len()..].trim_start_matches('.').len());
    let tail = &segment[at..];
    let bytes = tail.as_bytes();
    let bound = char_bound(tail, CONTRACT_NO_PREFIX_MAX);
    let end = (0..bound).find(|&i| {
        bytes[i] == b':' || bytes[i] == b',' || (bytes[i] == b'.' && matches!(bytes.get(i + 1), Some(b' ') | None))
    });
    match end {
        Some(i) if bytes[i] != b',' => Some(at + i + 1),
        _ => Some(segment.len()),
    }
}

/// The awarding authority as the body itself states it: its folded name, and the
/// e-mail addresses and URL hosts of its contact block (issue 484).
struct Authority {
    name: String,
    contacts: Vec<String>,
}

impl Authority {
    /// Whether a winner entry is the authority's own contact entry: the same name AND
    /// one of the same e-mail addresses or URL hosts. The name alone is not enough —
    /// 1200610's `1: Staffanstorps kommun, Städservice` is the municipality's own
    /// cleaning service, a genuine in-house lot with its own phone and no shared
    /// address. An attention marker (`A la atención de`, `Att:`) is not a signal
    /// either: 2808875 prints `Att: <person>` on all 19 genuine Cardiff providers.
    fn contact_entry(&self, name: &str, segment: &str) -> bool {
        if self.contacts.is_empty() || fold_name(name) != self.name {
            return false;
        }
        let segment = segment.to_lowercase();
        self.contacts.iter().any(|c| segment.contains(c.as_str()))
    }
}

/// The labels that head the authority's item: the numbered form's item 1, and the
/// sectioned form's `I.1)`.
const AUTHORITY_LABELS: [&str; 3] = ["AWARDING AUTHORITY:", "AWARDING ENTITY:", "CONTRACTING ENTITY:"];

/// The body's awarding authority, or `None` when the body does not state one in a
/// shape read here. Pure over the flattened body, like everything [`awarded_names`]
/// derives.
fn awarding_authority(flat: &str) -> Option<Authority> {
    let value = if let Some((start, label)) = AUTHORITY_LABELS
        .iter()
        .filter_map(|l| find_ascii_ci(flat, l).map(|i| (i, *l)))
        .min_by_key(|(i, _)| *i)
    {
        let rest = &flat[start + label.len()..];
        let wide = &rest[..char_bound(rest, NUMBERED_VALUE_MAX)];
        let end = numbered_item_before(flat, start)
            .and_then(|n| successor_marker(wide, n))
            .unwrap_or(char_bound(wide, NAME_WINDOW));
        &wide[..end]
    } else {
        // `I.1)` at a word start, so `II.1)` does not match.
        let bytes = flat.as_bytes();
        let start = (0..flat.len()).find(|&i| {
            (i == 0 || bytes[i - 1] == b' ') && bytes[i..].len() >= 4 && &bytes[i..i + 4] == b"I.1)"
        })?;
        let rest = &flat[start + 4..];
        let wide = &rest[..char_bound(rest, NUMBERED_VALUE_MAX)];
        let end = ["I.2)", "SECTION II"].iter().filter_map(|s| find_ascii_ci(wide, s)).min();
        let item = &wide[..end.unwrap_or(wide.len())];
        // The heading (`NAME, ADDRESSES AND CONTACT POINT(S):`) ends at its colon.
        &item[item.find(':')? + 1..]
    };
    let value = value.trim();
    let name = trim_sentence_period(value[..value.find(',').unwrap_or(value.len())].trim());
    let name = fold_name(name);
    if name.is_empty() {
        return None;
    }
    let mut contacts = Vec::new();
    for token in value.split_whitespace() {
        let token = token.trim_matches(|c: char| matches!(c, '.' | ',' | ';' | ':' | '(' | ')'));
        let lower = token.to_lowercase();
        let contact = if lower.contains('@') {
            lower
        } else {
            let host = lower.trim_start_matches("https://").trim_start_matches("http://");
            if !host.starts_with("www.") {
                continue;
            }
            let host = &host["www.".len()..];
            host[..host.find('/').unwrap_or(host.len())].to_owned()
        };
        if contact.contains('.') && contact.len() > 4 && !contacts.contains(&contact) {
            contacts.push(contact);
        }
    }
    Some(Authority { name, contacts })
}

/// A name folded for comparison: letters and digits only, lower-cased.
fn fold_name(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// The one monetary value a numbered-form award body states, or `None`.
///
/// `None` covers three different situations and deliberately does not distinguish them,
/// because the outcome is the same: no value is claimed. The body may state no price;
/// it may state one in a shape [`parse_money`] refuses; or it may state SEVERAL that
/// disagree — `8. Price:` and `9. Value of winning award(s):` both present with
/// different figures is a notice this cannot resolve, and picking one would be a guess
/// recorded as a fact.
fn awarded_value(body: &str) -> Option<(i64, String, Option<&'static str>)> {
    if find_ascii_ci(body, "PRICE").is_none() && find_ascii_ci(body, "VALUE").is_none() {
        return None;
    }
    let flat = flatten(body);
    // Per scope (0 = one contract, 1 = the whole notice): the claim held there, and
    // whether two claims at that scope disagreed (issue 244 slice 8).
    let mut claim: [Option<(i64, String, Option<&'static str>)>; 2] = [None, None];
    let mut conflict = [false, false];
    let mut at = 0usize;
    while at < flat.len() {
        let Some((start, label)) = VALUE_LABELS
            .iter()
            .filter_map(|l| find_ascii_ci(&flat[at..], l).map(|i| (at + i, *l)))
            .min_by_key(|(i, _)| *i)
        else {
            break;
        };
        let value_at = start + label.len();
        let rest = &flat[value_at..];
        let window = &rest[..char_bound(rest, NAME_WINDOW)];
        // Which scope this occurrence states, read from the words right after the label
        // and before the colon skip below eats them.
        let scope = usize::from(find_ascii_ci(window.trim_start(), AGGREGATE_SCOPE) == Some(0));
        // `VALUE OF WINNING AWARD` is matched without its `(s):` tail, since the era writes
        // both the plural and the singular. Skip to just past the colon that ends the
        // heading — by position, so `(s):` and `(S):` behave the same. The bound keeps this
        // from running to some later item's colon when the label already ended in one.
        let window = match window.find(':') {
            Some(colon) if colon <= 4 => &window[colon + 1..],
            _ => window,
        };
        // The value item is 4, 8 or 9 depending on the form, so the item that FOLLOWS it
        // is 5, 9 or 10 — [`ITEM_STOPS`], which is aimed at the winner item, does not
        // bound this. Any numbered item does.
        let end = next_item_marker(window);
        let item = &window[..end.unwrap_or(window.len())];
        let money = read_value_item(item)
            .or_else(|| if label == COLONLESS_TOTAL_LABEL { colonless_total(item) } else { None });
        if let Some(money) = money {
            match &claim[scope] {
                // Two labels agreeing is one fact stated twice; two disagreeing AT THE
                // SAME SCOPE is a notice this cannot read.
                Some(seen) if *seen != money => conflict[scope] = true,
                Some(_) => {}
                None => claim[scope] = Some(money),
            }
        }
        at = value_at;
    }
    // A per-contract figure is the NOTICE's total only when the notice awards ONE
    // contract. With several, one contract's value is a PART: notice 3871014 awards four
    // and its first V.4 figure is 40 087 596 SEK against a real total of 81 605 403, so
    // claiming it would understate by half. This drops such a claim rather than record it,
    // and it cannot touch the pre-2004 numbered form, which never writes `CONTRACT NO`.
    //
    // A sectioned body may head its awards `LOT NO: n` with no `CONTRACT NO` at all (issue
    // 508): notice 4040506 awards sixteen lots that way. Each lot's V.4 total but the
    // last runs into the next `LOT NO:` line, which no stop ends, so only the last one
    // parses, and without this it was claimed as the notice's total. Some bodies print no
    // heading at all, only one V.3 winner block per award (4300188: three, of 185 650,
    // 7 140 and 103 835 EUR, the last claimed), so the V.3 heading counts too. All three
    // are counted only where the body is sectioned (`SECTION V`), so the numbered form's
    // own `Lot No` lines stay out of it.
    let awards = count_heading(body, CONTRACT_MARKER).max(if find_ascii_ci(body, "SECTION V").is_some() {
        count_heading(body, LOT_MARKER).max(count_ascii_ci(&flat, AWARD_BLOCK_HEADING))
    } else {
        0
    });
    if awards > 1 {
        claim[0] = None;
    }
    // The widest scope the notice states wins, and a conflict THERE is still a refusal —
    // a narrower figure is not a fallback for an unreadable total.
    claim
        .iter()
        .enumerate()
        .rev()
        .find_map(|(scope, held)| held.as_ref().map(|money| (scope, money)))
        .and_then(|(scope, money)| (!conflict[scope]).then(|| money.clone()))
}

/// One value item: the figure it states, whether or not a sub-label stands in front of
/// it (issue 244 slice 5).
///
/// Measured on `fetch 300`: the strict shape alone claimed 390 of the 3,227 bodies that
/// state a price label, and SIX of eight sampled refusals were one shape — a
/// local-language sub-label ending in a colon before an otherwise perfect figure:
///
/// ```text
///     Price: Auftragssumme (ohne Umsatzsteuer): 689 655,17 DEM.
/// ```
///
/// So a value that does not parse whole is retried after its LAST colon — but only when
/// the part being skipped **contains no digit**. That guard is the whole safety of this
/// rule: without it, `1 000 000 EUR, of which subcontracted: 200 000 EUR` would claim
/// the subcontracted figure as the contract price. A pure label has no digits; a second
/// figure does.
fn read_value_item(item: &str) -> Option<(i64, String, Option<&'static str>)> {
    let (item, stop_basis) = cut_at_value_stop(item);
    if let Some((cents, currency, basis)) = parse_money(item) {
        return Some((cents, currency, basis.or(stop_basis)));
    }
    let (label, figure) = item.rsplit_once(':')?;
    if label.bytes().any(|b| b.is_ascii_digit()) {
        return None;
    }
    let (cents, currency, basis) = parse_money(figure)?;
    // A marker beside the figure wins over the sub-label's wording, and both win over the
    // stop phrase; they agree in every measured body, and the nearer statement is the more
    // specific one.
    Some((cents, currency, basis.or_else(|| phrase_basis(label)).or(stop_basis)))
}

/// The value item up to its first [`VALUE_STOPS`] phrase, and the basis that phrase
/// states. The sectioned form's prose continues past the figure, so it is cut at the
/// first stop (issue 244 slice 7).
fn cut_at_value_stop(item: &str) -> (&str, Option<&'static str>) {
    match VALUE_STOPS
        .iter()
        .filter_map(|(stop, basis)| find_ascii_ci(item, stop).map(|at| (at, *basis)))
        .min_by_key(|(at, _)| *at)
    {
        Some((at, basis)) => (&item[..at], basis),
        None => (item, None),
    }
}

/// The only label [`colonless_total`] reads after.
const COLONLESS_TOTAL_LABEL: &str = "TOTAL FINAL VALUE";

/// The 2010 print's total, which carries no colon to retry after (issue 508):
///
/// ```text
///     II.2.1)  Total final value of contract(s)          V.4)  … Total final value of the contract
///     Value 80 515 EUR                                    Value 80 515 EUR
///     Including VAT. VAT rate (%) 21                      Including VAT. VAT rate (%) 21
/// ```
///
/// From the 2010-03 package to the era's end, about 98,000 award bodies state a total this
/// way and 1,698 carried a figure: [`read_value_item`]'s retry splits on the LAST colon,
/// and here there is none. This reads exactly that shape and nothing looser — the scope words, the word
/// `Value`, then a figure [`parse_money`] takes whole — because a loose "after the word
/// Value" would also read `Price: Estimated value 2 000 000 ECU` as a price. Only the
/// `TOTAL FINAL VALUE` label calls it, so no other vintage's outcome moves.
fn colonless_total(item: &str) -> Option<(i64, String, Option<&'static str>)> {
    let (item, stop_basis) = cut_at_value_stop(item);
    let rest = item.trim_start();
    let rest = [AGGREGATE_SCOPE, "OF THE CONTRACT"].iter().find_map(|scope| strip_prefix_ci(rest, scope))?;
    let figure = strip_prefix_ci(rest.trim_start(), "VALUE ")?;
    let (cents, currency, basis) = parse_money(figure)?;
    Some((cents, currency, basis.or(stop_basis)))
}

/// `s` without its leading `prefix`, compared ASCII-case-insensitively.
fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let n = prefix.len();
    (s.len() >= n && s.as_bytes()[..n].eq_ignore_ascii_case(prefix.as_bytes())).then(|| &s[n..])
}

/// Where the next numbered form item begins: ` <n>. ` with one or two digits, in the
/// flattened body. Used to bound a value item whose successor is not one of the three
/// [`ITEM_STOPS`] the winner item is followed by.
///
/// The digits must be followed by a period and then a space or the end, which is what
/// separates an item marker from a date (`11.5.2001` has no space after `11.`) or a
/// house number (`Emilienstrasse 8,` has no period).
fn next_item_marker(window: &str) -> Option<usize> {
    let bytes = window.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] != b' ' {
            continue;
        }
        let mut j = i + 1;
        while j < bytes.len() && j - i <= 2 && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j == i + 1 || j >= bytes.len() || bytes[j] != b'.' {
            continue;
        }
        if j + 1 == bytes.len() || bytes[j + 1] == b' ' {
            return Some(i);
        }
    }
    None
}

/// Flatten TED's ~72-column wrap into one line.
///
/// Built directly rather than through `split_whitespace().collect::<Vec<_>>().join(" ")`,
/// which allocated a vector of slices as well as the string.
fn flatten(body: &str) -> String {
    let mut flat = String::with_capacity(body.len());
    for word in body.split_whitespace() {
        if !flat.is_empty() {
            flat.push(' ');
        }
        flat.push_str(word);
    }
    flat
}

/// The largest char boundary at or below `at`, so a window can be cut by byte length
/// without splitting a character.
fn char_bound(s: &str, at: usize) -> usize {
    let mut at = at.min(s.len());
    while at > 0 && !s.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// ASCII-case-insensitive substring search, returning a byte index into `haystack`.
///
/// Replaces `haystack.to_uppercase().find(needle)`, which was wrong as well as slow.
/// Uppercasing can CHANGE a string's byte length — `ı` (Turkish dotless i, 2 bytes)
/// uppercases to `I` (1 byte), and TED's text era carries Turkish and German names —
/// so an index found in the uppercased copy did not necessarily address the same place
/// in the original, and slicing there was either wrong or a panic on a non-boundary.
///
/// The labels are pure ASCII, and an ASCII byte can never occur inside a multi-byte
/// UTF-8 sequence, so a match found byte-wise is always at a char boundary and always
/// means what it appears to mean. No allocation, which is the other half of the fix.
fn find_ascii_ci(haystack: &str, needle: &str) -> Option<usize> {
    let (h, n) = (haystack.as_bytes(), needle.as_bytes());
    if n.is_empty() || h.len() < n.len() {
        return None;
    }
    h.windows(n.len()).position(|w| w.eq_ignore_ascii_case(n))
}

/// How many times `needle` occurs in `haystack`, ASCII-case-insensitively and without
/// overlap. Slicing past each match is safe because an ASCII needle can only match ASCII
/// bytes, which are never part of a multi-byte UTF-8 sequence.
fn count_ascii_ci(haystack: &str, needle: &str) -> usize {
    let mut n = 0;
    let mut at = 0;
    while let Some(i) = find_ascii_ci(&haystack[at..], needle) {
        n += 1;
        at += i + needle.len();
    }
    n
}

#[derive(Debug, PartialEq)]
pub struct Rejected {
    pub reason: &'static str,
    pub detail: String,
}

/// Parse one decoded record into the notice-parsed layer.
pub fn parse(text: &str) -> Result<Parsed, Rejected> {
    let mut emit = Emit::default();
    let mut open: Option<Field> = None;

    let mut lines = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).enumerate();

    // The first non-blank line must be the record marker the splitter keyed on.
    for (n, line) in lines.by_ref() {
        if line.trim().is_empty() {
            continue;
        }
        if !is_marker(line) {
            return Err(unclaimed(n, line));
        }
        break;
    }

    for (n, line) in lines {
        if let Some((code, rest)) = tag_line(line) {
            let Some(rule) = rules::rule(code) else {
                return Err(Rejected {
                    reason: "unknown-field-code",
                    detail: format!("line {}: {code}", n + 1),
                });
            };
            flush(&mut emit, open.take())?;
            open = Some(Field {
                code: code.to_owned(),
                rule,
                lines: vec![rest.strip_prefix(' ').unwrap_or(rest).trim_end().to_owned()],
                line_no: n,
            });
        } else if line.trim().is_empty() {
            // Layout inside a prose blob is a paragraph break; elsewhere noise.
            if let Some(field) = &mut open
                && matches!(field.rule, Rule::Prose(_))
            {
                field.lines.push(String::new());
            }
        } else if line.starts_with(' ') || line.starts_with('\t') {
            match &mut open {
                Some(field) => field.lines.push(dedent(line).to_owned()),
                None => return Err(unclaimed(n, line)),
            }
        } else if is_terminal_echo(line) {
            // The era's production system echoed its own search command into
            // the file (`.S F=ALL;R=…;SORT=PD;ND;HC`) — measured 4 members,
            // all 1994, each after the record's last header field. Production
            // residue, never notice content: consumed like a blank line.
        } else if let Some(field) = &mut open {
            // A column-0 continuation (issue 199): TED's own line-wrapper
            // emits a wrapped tail flush-left — after a `!` in the text, a
            // closing quote/paren, or a fixed-width mid-word sever — and the
            // correction margin marker `!` occupies column 0 the same way.
            // All 54 measured members were wrapped content inside an open
            // field, never structure; under a prose field the line joins the
            // body, under a per-line field it is its own value (the measured
            // RC/RG rows are real codes), and under a scalar the flush guard
            // still rejects it as unconsumed structure.
            field.lines.push(strip_margin(line).to_owned());
        } else {
            return Err(unclaimed(n, line));
        }
    }
    flush(&mut emit, open)?;

    if emit.parsed.values.is_empty() {
        return Err(Rejected { reason: "empty-record", detail: "no header fields".into() });
    }
    claim_award_skeleton(&mut emit);
    claim_awarded_value(&mut emit);
    claim_award_date(&mut emit);
    claim_tenders_received(&mut emit);
    home_authority_descriptors(&mut emit);
    drop_nature_atoms(&mut emit);
    Ok(emit.parsed)
}

/// `CY` and `TW` describe the awarding authority — the inventory reads "Country
/// (code)" and "Town of the awarding authority" — so when the record opened an
/// authority section they belong inside it, beside the `TXT-AU` name (issue 232's
/// recorded follow-on). `TXT-CY` is already in the projection's
/// `ORG_COUNTRY_FIELDS`, so homing it gives the buyer mention a country, and a
/// mention with a country joins the issue-234 reuse scope: the era's authorities
/// aggregate instead of minting one provisional Organization per notice.
///
/// A post-pass rather than a routing decision in `flush`, because the era
/// publishes the fields in header order — `CY:` arrives BEFORE `AU:` (every
/// fixture vintage), when the authority section does not exist yet. A record with
/// no `AU:` keeps both on the root, where they have always sat: a country row
/// inside an org section that names nobody would make the projection mint a
/// nameless buyer.
fn home_authority_descriptors(emit: &mut Emit) {
    if !emit.parsed.sections.iter().any(|s| s.id == AUTHORITY_SECTION) {
        return;
    }
    for row in &mut emit.parsed.values {
        if row.section_id == SECTION && (row.field_id == "TXT-CY" || row.field_id == "TXT-TW") {
            row.section_id = AUTHORITY_SECTION.to_owned();
        }
    }
}

/// The contract's price, claimed after the whole record is read (issue 244).
///
/// A post-pass rather than a hook in the prose flush, for one reason that matters: the
/// claim is only legitimate on an **award** notice, and the document-type code `TD` may
/// be consumed before or after the `TX` body depending on the record's field order. A
/// post-pass sees both regardless.
///
/// Why the gate is needed: measured over `fetch 300`, of the 3,250 bodies stating a price
/// label, 3,227 are `TD:7` awards — but 18 are `TD:3` invitations to tender, 4 are `TD:0`
/// and 1 is a `TD:2` corrigendum. A notice with no result must not carry a
/// `result_value`, so those 23 are exactly the wrong facts this refuses to write. (The
/// winner labels needed no such gate: all 3,857 bodies carrying one are `TD:7`.)
///
/// Notice scope, not the LotResult: the price is stated once per notice while a notice
/// can name several winners, so attaching it to the first result would hand one of them a
/// whole contract. `TED-VAL_TOTAL` at root is already mapped to `result_value` in the
/// projection's `AMOUNTS`, so this needs no mapping change.
/// How the era labels the date the contract was awarded (issue 255 slice 3, issue 244).
///
/// Two spellings cover both forms, and the shorter one is a prefix of the numbered form's
/// longer variant, so it matches that too:
///
/// ```text
///     3.  Date of award: 30.3.2001.                    numbered form (prod 1,710,441+)
///     5.  Date of award of the contract: 11.5.2001.    numbered form, external aid
///     VI.3)  Date of contract award: 25.11.2004.       sectioned form (2005 CAN fixture)
/// ```
///
/// Both spellings are in the committed fixtures: the 1993 daily uses `Date of award:` in
/// dozens of its 199 records, and `2005-can-154-2005` uses `Date of contract award:`.
const AWARD_DATE_LABELS: [&str; 2] = ["DATE OF AWARD", "DATE OF CONTRACT AWARD"];

/// How far past the label the colon and the figure may sit. `Date of award of the
/// contract:` is 30 characters from the label's end to its colon.
const AWARD_DATE_WINDOW: usize = 64;

/// The date one text-era award body states, or `None`.
///
/// `None` means the same three things it means for [`awarded_value`] and for the same
/// reason: no date stated, a shape this refuses, or SEVERAL that disagree — a notice
/// awarding two contracts on two days states no single award date, and picking one would
/// be a guess recorded as a fact.
fn award_date(body: &str) -> Option<(i64, i64, bool)> {
    let flat = flatten(body);
    let mut found: Option<(i64, i64, bool)> = None;
    let mut at = 0usize;
    while at < flat.len() {
        let Some((start, label)) = AWARD_DATE_LABELS
            .iter()
            .filter_map(|l| find_ascii_ci(&flat[at..], l).map(|i| (at + i, *l)))
            .min_by_key(|(i, _)| *i)
        else {
            break;
        };
        at = start + label.len();
        let rest = &flat[at..];
        let window = &rest[..char_bound(rest, AWARD_DATE_WINDOW)];
        // The label ends before its colon in every measured shape, so the figure starts
        // after the colon; without one there is no value to read.
        let Some(colon) = window.find(':') else { continue };
        if let Some(stamp) = read_dmy(&window[colon + 1..]) {
            match found {
                // Two labels agreeing is one fact stated twice; two disagreeing is a
                // notice this cannot read.
                Some(seen) if seen != stamp => return None,
                Some(_) => {}
                None => found = Some(stamp),
            }
        }
    }
    found
}

/// `30.3.2001` at the head of `text`, as (utc seconds, offset minutes, has_time).
///
/// Scanned rather than split, because the era writes the separators with optional spaces
/// (`2. 11. 1999`) and because whatever follows the year is the next item, not part of the
/// date. Two-digit day and month, four-digit year, and nothing clever: a two-digit year
/// would be ambiguous across an era spanning 1993-2010 and is refused.
fn read_dmy(text: &str) -> Option<(i64, i64, bool)> {
    let b = text.as_bytes();
    let mut i = 0usize;
    let number = |i: &mut usize, max: usize| -> Option<String> {
        while *i < b.len() && b[*i] == b' ' {
            *i += 1;
        }
        let from = *i;
        while *i < b.len() && b[*i].is_ascii_digit() && *i - from < max {
            *i += 1;
        }
        (*i > from).then(|| String::from_utf8_lossy(&b[from..*i]).into_owned())
    };
    let day = number(&mut i, 2)?;
    let sep = |i: &mut usize| -> Option<()> {
        while *i < b.len() && b[*i] == b' ' {
            *i += 1;
        }
        (*i < b.len() && b[*i] == b'.').then(|| *i += 1)
    };
    sep(&mut i)?;
    let month = number(&mut i, 2)?;
    sep(&mut i)?;
    let year = number(&mut i, 4)?;
    if year.len() != 4 {
        return None;
    }
    // The parts must be a real calendar date BEFORE they reach the shared parser, because
    // that parser NORMALISES rather than refuses: `30.13.2001` comes back as 2002-01-30
    // and `31.2.2001` as 2001-03-03. Rolling a typo into a neighbouring month is exactly
    // the kind of quiet wrong fact this era's prose can produce at scale.
    let (d, m, y): (u32, u32, i32) =
        (day.parse().ok()?, month.parse().ok()?, year.parse().ok()?);
    if !(1..=12).contains(&m) || d < 1 || d > days_in_month(m, y) {
        return None;
    }
    match value::date_from_parts(&day, &month, &year, None) {
        Ok(NoticeValue::Date { utc_seconds, offset_minutes, has_time }) => {
            Some((utc_seconds, offset_minutes, has_time))
        }
        _ => None,
    }
}

/// Days in a Gregorian month, for the range check [`read_dmy`] does before handing its
/// parts to a parser that would normalise them instead.
fn days_in_month(month: u32, year: i32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

/// The award date, onto every result block the body yielded (issue 255 slice 3).
///
/// `TED-CONTRACT_AWARD_DATE` is the legacy form eras' own field id and the projection
/// already routes it to `tender_version_lot_results.decided_*` (issue 255 slice 2), so
/// this reaches the canonical layer with no mapping change — the same trick
/// [`claim_awarded_value`] plays with `TED-VAL_TOTAL`.
///
/// A body with no winner has no result block, and then the date has nowhere to land: the
/// canonical model hangs an award date on an award, not on a Tender. Those notices keep
/// the date in their `TXT-TX` prose, retrievable, exactly as before.
/// Phrases with which an award body says the procedure ended WITHOUT an award, in
/// the winner slot or beside it. Literal and short on purpose, like [`NAME_REJECTS`]:
/// each entry is added with its own evidence, because a broad "sounds cancelled" test
/// would also match bodies that merely mention a cancelled predecessor.
const REJECTION_PHRASES: [&str; 3] =
    ["ALL TENDERS WERE REJECTED", "ALL OFFERS WERE REJECTED", "ALL TENDERS HAVE BEEN REJECTED"];

/// Mint the result a winner-silent award body earns (issue 244 slice 9).
///
/// The residue read behind this: of the 219,907 award notices left with no result
/// block after slices 1-8, three of four sampled bands were REAL awards whose winner
/// slot the publisher left empty, filled with `Various`, or filled with rejection
/// prose — the same publisher-silence the sdk-0.1 study (issue 257) taught us never
/// to read as "no result". The extractor minted results only via a winner name, so
/// those awards materialised nothing.
///
/// So: an award-typed body (`TD: 7`) that minted NO result via its names, but which
/// states an award date or says outright that every tender was rejected, yields ONE
/// bare `LotResult` — no Organization is ever invented (the 257 rule). What the
/// projection then does with the evidence (`read_legacy_results`):
///
/// - rejection phrase → `TED-NO_AWARDED_CONTRACT` here → decision `clos-nw`;
/// - award date, no winner, no result-scoped value → decision stays NULL — an award
///   the publisher announced and did not detail is silence, not closure;
///
/// and `claim_award_date`/`claim_tenders_received` run AFTER this, so the date and
/// the count land on the section minted here.
fn claim_award_skeleton(emit: &mut Emit) {
    let is_award = emit.parsed.values.iter().any(|v| {
        v.field_id == "TXT-TD" && matches!(&v.value, NoticeValue::Code { code, .. } if code == "7")
    });
    if !is_award || emit.parsed.sections.iter().any(|s| s.kind == "LotResult") {
        return;
    }
    let Some(body) = emit.parsed.values.iter().find_map(|v| match (&v.field_id, &v.value) {
        (f, NoticeValue::Text { value, .. }) if f == "TXT-TX" => Some(value.as_str()),
        _ => None,
    }) else {
        return;
    };
    // Flattened first: the era wraps at ~72 columns, so a phrase can straddle a
    // line break — the same reason `awarded_names` flattens before matching.
    let flat = flatten(body);
    let rejected = REJECTION_PHRASES.iter().any(|p| find_ascii_ci(&flat, p).is_some());
    if !rejected && award_date(body).is_none() {
        return;
    }
    emit.root();
    emit.parsed.sections.push(Section {
        id: "RES-1".into(),
        kind: "LotResult".into(),
        parent: Some(SECTION.into()),
    });
    if rejected {
        emit.push_into("RES-1", "TED-NO_AWARDED_CONTRACT", NoticeValue::Code {
            list: None,
            code: "1".into(),
        });
    }
}

/// The labels under which the era states how many tenders the buyer received.
/// `TENDERS RECEIVED:` is the tail of both the numbered form's `5. Tenders
/// received:` and the sectioned form's `VI.4) Number of tenders received:`.
const TENDER_COUNT_LABELS: [&str; 2] = ["TENDERS RECEIVED:", "OFFERS RECEIVED:"];

/// The one tenders-received count an award body states, or `None` — including when
/// two statements disagree, the same refusal `awarded_value` makes: picking one
/// would be a guess recorded as a fact.
fn tenders_received(body: &str) -> Option<i64> {
    if find_ascii_ci(body, "RECEIVED").is_none() {
        return None;
    }
    let flat = flatten(body);
    let mut claim: Option<i64> = None;
    for label in TENDER_COUNT_LABELS {
        let mut at = 0usize;
        while let Some(i) = find_ascii_ci(&flat[at..], label) {
            let rest = flat[at + i + label.len()..].trim_start();
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            at += i + label.len();
            // At most 6 digits: the biggest real count measured is two digits and a
            // longer run is a reference number the label does not own.
            if digits.is_empty() || digits.len() > 6 {
                continue;
            }
            let n: i64 = digits.parse().ok()?;
            match claim {
                Some(seen) if seen != n => return None,
                _ => claim = Some(n),
            }
        }
    }
    claim
}

/// Attach the notice's tenders-received count to its result (issue 244 slice 9) —
/// `TED-NB_TENDERS_RECEIVED` is already in the projection's
/// `LEGACY_BID_COUNT_FIELDS`, so it lands in the result's statistics with no new
/// projection code. Only when the notice holds exactly ONE result: the count is
/// notice-scoped, and copying it onto each of several contracts would state it
/// several times over.
fn claim_tenders_received(emit: &mut Emit) {
    let is_award = emit.parsed.values.iter().any(|v| {
        v.field_id == "TXT-TD" && matches!(&v.value, NoticeValue::Code { code, .. } if code == "7")
    });
    if !is_award {
        return;
    }
    let results: Vec<String> = emit
        .parsed
        .sections
        .iter()
        .filter(|s| s.kind == "LotResult")
        .map(|s| s.id.clone())
        .collect();
    let [result] = results.as_slice() else { return };
    let count = emit
        .parsed
        .values
        .iter()
        .find_map(|v| match (&v.field_id, &v.value) {
            (f, NoticeValue::Text { value, .. }) if f == "TXT-TX" => Some(value.as_str()),
            _ => None,
        })
        .and_then(tenders_received);
    if let Some(n) = count {
        let result = result.clone();
        emit.push_into(&result, "TED-NB_TENDERS_RECEIVED", NoticeValue::Integer(n));
    }
}

fn claim_award_date(emit: &mut Emit) {
    let is_award = emit.parsed.values.iter().any(|v| {
        v.field_id == "TXT-TD" && matches!(&v.value, NoticeValue::Code { code, .. } if code == "7")
    });
    if !is_award {
        return;
    }
    let stamp = emit
        .parsed
        .values
        .iter()
        .find_map(|v| match (&v.field_id, &v.value) {
            (f, NoticeValue::Text { value, .. }) if f == "TXT-TX" => Some(value.as_str()),
            _ => None,
        })
        .and_then(award_date);
    let Some((utc_seconds, offset_minutes, has_time)) = stamp else { return };
    let results: Vec<String> = emit
        .parsed
        .sections
        .iter()
        .filter(|s| s.kind == "LotResult")
        .map(|s| s.id.clone())
        .collect();
    for result in results {
        emit.push_into(&result, "TED-CONTRACT_AWARD_DATE", NoticeValue::Date {
            utc_seconds,
            offset_minutes,
            has_time,
        });
    }
}

fn claim_awarded_value(emit: &mut Emit) {
    let is_award = emit.parsed.values.iter().any(|v| {
        v.field_id == "TXT-TD" && matches!(&v.value, NoticeValue::Code { code, .. } if code == "7")
    });
    if !is_award {
        return;
    }
    // The borrow of `values` ends with the expression, so the body is read in place
    // rather than cloned — these are multi-kilobyte prose blobs and the era has 3.8M of
    // them.
    let money = emit
        .parsed
        .values
        .iter()
        .find_map(|v| match (&v.field_id, &v.value) {
            (f, NoticeValue::Text { value, .. }) if f == "TXT-TX" => Some(value.as_str()),
            _ => None,
        })
        .and_then(awarded_value);
    if let Some((cents, currency, basis)) = money {
        emit.value(cents, currency, basis);
    }
}

struct Field {
    code: String,
    rule: Rule,
    lines: Vec<String>,
    line_no: usize,
}

fn flush(emit: &mut Emit, field: Option<Field>) -> Result<(), Rejected> {
    let Some(field) = field else { return Ok(()) };
    let id = format!("TXT-{}", field.code);
    match field.rule {
        Rule::Scalar(kind) => {
            if field.lines.len() > 1 {
                return Err(Rejected {
                    reason: "unclaimed-content",
                    detail: format!(
                        "line {}: continuation under scalar field {}",
                        field.line_no + 2,
                        field.code
                    ),
                });
            }
            emit.typed(&id, kind, &field.lines[0]);
        }
        Rule::PerLine(kind) => {
            for line in &field.lines {
                emit.typed(&id, kind, line);
            }
        }
        Rule::Prose(lang) => {
            let mut lines = field.lines;
            while lines.last().is_some_and(|l| l.is_empty()) {
                lines.pop();
            }
            let text = lines.join("\n");
            if !text.is_empty() {
                // `TX` is the English body, and from 2004 it carries the award
                // block the era publishes no section for (issue 244). The prose
                // is emitted whole either way — the winners are derived from it,
                // not moved out of it.
                if field.code == "TX" {
                    for name in awarded_names(&text) {
                        emit.award(name);
                    }
                }
                emit.text(&id, lang, text);
            }
        }
        Rule::Name => {
            // Issue 436: one name, space-joined — the wrapper's break is never
            // content in a name (the heading unwrap, without a title's annotation
            // vocabulary).
            let text = flatten(&field.lines.join(" "));
            if !text.is_empty() {
                // `AU` is the awarding authority's name, and it becomes an
                // Organization rather than a text row on the root. It is NOT one
                // clean line in every vintage: 108345-1997 wraps it, and so did
                // ~1,000 of every 6–7k text-era organizations on prod.
                if field.code == "AU" {
                    emit.authority(text);
                } else {
                    emit.text(&id, None, text);
                }
            }
        }
        Rule::Heading(lang) => {
            // Issue 397: one line, space-joined — the wrapper's break is never
            // content in a heading — then the OJ's authenticity boilerplate is
            // dropped from the trailing annotation block.
            let text = strip_authenticity_note(&flatten(&field.lines.join(" ")));
            if !text.is_empty() {
                emit.text(&id, lang, text);
            }
        }
    }
    Ok(())
}

/// The annotation atoms the OJ prints in a parenthesis under a text-era title.
///
/// A CLOSED vocabulary, measured rather than guessed: over tender ids
/// 7,960,000–8,059,999 there are 26 distinct trailing `(…)` blocks on
/// newline-carrying titles, covering 10,993 rows, and splitting them on ` - `
/// yields exactly these atoms plus two one-off strings that are not annotations
/// at all (`(PCs)` and `(GeophysB 2026)`, one row each — genuine title text that
/// the wrapper happened to isolate).
///
/// Those two are why the block is matched by VOCABULARY and not by shape. A
/// "trailing parenthetical is boilerplate" rule would have eaten them, which is
/// the same silent-loss mistake in the other direction: a title is content, and
/// content nobody has classified must be kept, not tidied away.
///
/// Case varies in the source (`With participation…` 686 rows, `with…` 269;
/// `Supply contract` 2,918, `supply contract` 11), so matching is
/// case-insensitive.
const TITLE_ANNOTATIONS: &[&str] = &[
    "only the original text is authentic",
    "supply contract",
    "works contract",
    "service contract",
    "combined contract",
    "open to us bidders",
    "with participation by gatt countries",
];

/// The OJ's authenticity notice — pure boilerplate, printed under 8,576 of the
/// 11,769 wrapped titles in the measured range and under none of the XML or
/// eForms eras, so it is the single largest reason a text-era title cannot be
/// compared with a modern one.
const AUTHENTICITY_NOTE: &str = "only the original text is authentic";

/// Drop the authenticity boilerplate from a title's trailing annotation block,
/// leaving the block's substantive atoms in place (issue 397).
///
/// Conservative by construction: the trailing `(…)` is rewritten only when EVERY
/// one of its ` - `-separated atoms is in [`TITLE_ANNOTATIONS`]. An unrecognised
/// atom means this is not an annotation block — it is title text that happens to
/// be parenthesised — and the title is returned untouched.
///
/// The other atoms are handled by who owns the fact. The four NATURE atoms
/// (`Supply contract`, …) are the label of the record's own `NC` code, which
/// the fold serves as a `nature` classification (issue 397 unit 2, step 1), so
/// [`drop_nature_atoms`] removes them once the whole record is in — and only
/// when the record carries that code. `Open to US bidders` and `With
/// participation by GATT countries` are regime flags no field carries yet, and
/// they stay in the title until one does: dropping them would destroy a fact,
/// which is worse than the defect being fixed.
fn strip_authenticity_note(title: &str) -> String {
    strip_annotations(title, &[AUTHENTICITY_NOTE])
}

/// The four nature atoms of the annotation vocabulary — each the label the text
/// era prints beside its `NC` code (`NC: 2 - Supply contract`), re-published by
/// the ~72-column wrapper into the title's trailing block.
const NATURE_ATOMS: &[&str] = &["supply contract", "works contract", "service contract", "combined contract"];

/// Rewrite a title's trailing annotation block without the atoms in `drop`,
/// leaving every other atom in place (issue 397).
///
/// Conservative by construction: the trailing `(…)` is rewritten only when EVERY
/// one of its ` - `-separated atoms is in [`TITLE_ANNOTATIONS`]. An unrecognised
/// atom means this is not an annotation block — it is title text that happens to
/// be parenthesised — and the title is returned untouched.
fn strip_annotations(title: &str, drop: &[&str]) -> String {
    let trimmed = title.trim_end();
    let Some(open) = trimmed.rfind('(') else { return title.trim().to_owned() };
    if !trimmed.ends_with(')') {
        return title.trim().to_owned();
    }
    let block = &trimmed[open + 1..trimmed.len() - 1];
    let atoms: Vec<&str> = block.split(" - ").map(str::trim).collect();
    if atoms.is_empty()
        || !atoms.iter().all(|a| {
            let a = a.to_lowercase();
            TITLE_ANNOTATIONS.contains(&a.as_str())
        })
    {
        return title.trim().to_owned();
    }
    let kept: Vec<&str> = atoms
        .into_iter()
        .filter(|a| {
            let a = a.to_lowercase();
            !drop.contains(&a.as_str())
        })
        .collect();
    let head = trimmed[..open].trim_end();
    if kept.is_empty() {
        return head.to_owned();
    }
    format!("{head} ({})", kept.join(" - "))
}

/// Issue 397 unit 2, step 2: the nature atoms leave the title once the record is
/// whole. They duplicate the record's own `NC` code — the era prints code and
/// label together, the parser keeps the code as `TXT-NC`, the fold serves it as
/// a `nature` classification — so the title loses nothing the notice does not
/// still carry. Gated on the code being PRESENT on this record: a record without
/// an `NC` line keeps its atom, because then the atom is the only copy. Runs
/// after every header is flushed because `TI` precedes `NC` in the era's order.
fn drop_nature_atoms(emit: &mut Emit) {
    let coded = emit
        .parsed
        .values
        .iter()
        .any(|v| v.field_id == "TXT-NC" && matches!(&v.value, NoticeValue::Code { .. }));
    if !coded {
        return;
    }
    for v in emit.parsed.values.iter_mut() {
        if v.field_id == "TXT-TI"
            && let NoticeValue::Text { value, .. } = &mut v.value
        {
            let stripped = strip_annotations(value, NATURE_ATOMS);
            if stripped != *value {
                *value = stripped;
            }
        }
    }
}

#[derive(Default)]
struct Emit {
    parsed: Parsed,
    ordinals: std::collections::HashMap<String, i64>,
}

impl Emit {
    fn typed(&mut self, field: &str, kind: Type, raw: &str) {
        let raw = raw.trim();
        if raw.is_empty() {
            return;
        }
        let value = match kind {
            Type::Date => value::date(raw).ok(),
            Type::Deadline => deadline(raw),
            Type::Code => code(raw),
            Type::Cpv => Some(classification("cpv", raw)),
            Type::Nuts => Some(classification("nuts", raw)),
            Type::Product => Some(classification("cc", raw)),
            Type::Id => Some(NoticeValue::Id { scheme: None, value: raw.to_owned(), is_ref: false }),
            Type::Ref => Some(NoticeValue::Id {
                scheme: Some("ojs".to_owned()),
                value: raw.to_owned(),
                is_ref: true,
            }),
            Type::Integer => raw.parse::<i64>().ok().map(NoticeValue::Integer),
            Type::Line(lang) => {
                self.text(field, lang, raw.to_owned());
                return;
            }
        };
        match value {
            Some(value) => self.push(field, value),
            // A shape the era should not publish: raw kept, typed absent.
            None => self.text(field, None, raw.to_owned()),
        }
    }

    fn text(&mut self, field: &str, lang: Option<&'static str>, value: String) {
        self.push(field, NoticeValue::Text { lang: lang.map(str::to_owned), value });
    }

    /// The awarding authority as an Organization (issue 232): its name inside a
    /// synthesized `ORG-1` section, and a `buyer` role reference to it on the root.
    ///
    /// Opened at most once per record. A second `AU` — not observed, but the parser
    /// does not forbid it — adds another name row to the same section, and the
    /// projection's mention keeps the first, so the role never doubles.
    fn authority(&mut self, name: String) {
        self.root();
        if !self.parsed.sections.iter().any(|s| s.id == AUTHORITY_SECTION) {
            self.parsed.sections.push(Section {
                id: AUTHORITY_SECTION.into(),
                kind: "Organization".into(),
                parent: Some(SECTION.into()),
            });
            self.push_into(SECTION, "TED-ADDRESS_CONTRACTING_BODY", NoticeValue::Id {
                scheme: None,
                value: AUTHORITY_SECTION.into(),
                is_ref: true,
            });
        }
        self.push_into(AUTHORITY_SECTION, "TXT-AU", NoticeValue::Text { lang: None, value: name });
    }

    /// One award as a result section with its winner (issue 244), manufactured the
    /// same way [`Emit::authority`] manufactures the buyer — because the projection
    /// already reads this shape for every other legacy profile and needs no new code
    /// path to reach it:
    ///
    /// - a `LotResult` section, which `read_legacy_results` turns into a
    ///   `lot_results` row (`is_legacy_profile` has covered `text` all along; the era
    ///   simply never produced a section for it to find);
    /// - an `Organization` section holding the name, since
    ///   `organization_mentions` has a FOREIGN KEY on `(notice_id, section_id)` and
    ///   only a party-kinded section can carry a mention;
    /// - a `TED-ADDRESS_CONTRACTOR` id-ref from the result to that organization,
    ///   which `legacy_role` already folds onto `winner`.
    ///
    /// Ids are numbered per award so a notice awarding several contracts gets several
    /// results, and the organization ids continue past `ORG-1` — the buyer's — so the
    /// two never collide.
    fn award(&mut self, name: String) {
        self.root();
        let n = self.parsed.sections.iter().filter(|s| s.kind == "LotResult").count() + 1;
        let result = format!("RES-{n}");
        let org = format!("ORG-{}", n + 1);
        self.parsed.sections.push(Section {
            id: result.clone(),
            kind: "LotResult".into(),
            parent: Some(SECTION.into()),
        });
        self.parsed.sections.push(Section {
            id: org.clone(),
            kind: "Organization".into(),
            parent: Some(result.clone()),
        });
        self.push_into(&result, "TED-ADDRESS_CONTRACTOR", NoticeValue::Id {
            scheme: None,
            value: org.clone(),
            is_ref: true,
        });
        // `TED-OFFICIALNAME`, not the era's own `TXT-CO`, for two reasons that both
        // matter: `ORG_NAME_FIELDS` in the projection reads only `TED-OFFICIALNAME`,
        // `TED-ORGANISATION` and `TXT-AU`, so a name filed anywhere else leaves the organization
        // NAMELESS; and `TXT-CO` already exists on the root when the era publishes its
        // `CO:` line, so reusing it would make one field id mean two different things
        // in one notice. The buyer's `TXT-AU` has neither problem — it is in that list
        // and it is not also a root value.
        self.push_into(&org, "TED-OFFICIALNAME", NoticeValue::Text { lang: None, value: name });
    }

    /// The contract's price, at notice scope (issue 244). `TED-VAL_TOTAL` is already in
    /// the projection's `AMOUNTS` map as `result_value`, so this reaches
    /// `tender_version_amounts` with no mapping change — it is the same fact the r209
    /// era publishes under the same field id, arrived at from prose instead of a tag.
    fn value(&mut self, cents: i64, currency: String, basis: Option<&'static str>) {
        self.push("TED-VAL_TOTAL", NoticeValue::Amount { cents, currency });
        // The tax basis the source states, captured in the parse layer even though the
        // canonical layer has nowhere to put it yet (issue 251). Recording it now means
        // that when a destination exists, the era does not have to be re-parsed to learn
        // what it already said — and a reader of the parse layer can already tell an
        // inclusive figure from an exclusive one.
        if let Some(basis) = basis {
            self.push("TED-VAL_TOTAL_TAX_BASIS", NoticeValue::Code {
                list: None,
                code: basis.to_owned(),
            });
        }
    }

    fn push(&mut self, field: &str, value: NoticeValue) {
        self.root();
        self.push_into(SECTION, field, value);
    }

    /// The root section, created on first use — a record with no consumed field
    /// emits no sections at all, which is what makes the `empty-record` rejection
    /// detectable.
    fn root(&mut self) {
        if self.parsed.sections.is_empty() {
            self.parsed.sections.push(Section {
                id: SECTION.into(),
                kind: "Notice".into(),
                parent: None,
            });
        }
    }

    fn push_into(&mut self, section: &str, field: &str, value: NoticeValue) {
        let ordinal = self.ordinals.entry(field.to_owned()).or_insert(-1);
        *ordinal += 1;
        self.parsed.values.push(ValueRow {
            section_id: section.to_owned(),
            field_id: field.to_owned(),
            ordinal: *ordinal,
            value,
        });
    }
}

/// `YYYYMMDD` or `YYYYMMDD  HH MM` — the deadline fields' two measured shapes.
fn deadline(raw: &str) -> Option<NoticeValue> {
    let (date, clock) = raw.split_at(raw.len().min(8));
    let clock = clock.trim();
    if clock.is_empty() {
        return value::date(date).ok();
    }
    let (h, m) = clock.split_once(' ')?;
    value::date_with_time(date, &format!("{h}:{}", m.trim())).ok()
}

/// `3 - Invitation to tender` → `3`; the label is the redundant display text.
fn code(raw: &str) -> Option<NoticeValue> {
    let mut parts = raw.splitn(2, char::is_whitespace);
    let token = parts.next()?;
    let rest = parts.next().unwrap_or("").trim_start();
    if !rest.is_empty() && !rest.starts_with('-') {
        return None;
    }
    Some(NoticeValue::Code { list: None, code: token.to_owned() })
}

fn classification(scheme: &str, code: &str) -> NoticeValue {
    NoticeValue::Classification { scheme: scheme.to_owned(), code: code.to_owned() }
}

/// `TAG:` at column 0 — two ASCII uppercase alphanumerics, first alphabetic.
fn tag_line(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    if bytes.len() < 3
        || !bytes[0].is_ascii_uppercase()
        || !(bytes[1].is_ascii_uppercase() || bytes[1].is_ascii_digit())
        || bytes[2] != b':'
    {
        return None;
    }
    Some((&line[..2], &line[3..]))
}

/// Continuation lines carry a 4-space layout indent; deeper indentation (the
/// numbered-list alignment of `TX` bodies) is content and stays.
fn dedent(line: &str) -> &str {
    line.strip_prefix("    ").unwrap_or_else(|| line.trim_start_matches([' ', '\t']))
}

/// The correction margin marker: `!` at column 0 followed by the layout
/// indent. Presentation (TED flags corrected lines in the margin), not
/// content — stripped so the line aligns with its dedented siblings.
fn strip_margin(line: &str) -> &str {
    match line.strip_prefix('!') {
        Some(rest) if rest.starts_with(' ') || rest.starts_with('\t') => dedent(rest),
        _ => line,
    }
}

/// The mainframe search-command echo (`.S F=ALL;R=19212 TO 1;SORT=PD;ND;HC`)
/// the era's production left between records.
fn is_terminal_echo(line: &str) -> bool {
    line.starts_with(".S ") && line.contains(";SORT=")
}

/// `<digits>.<digits>/<digits>` alone on its line — the splitter's marker.
fn is_marker(line: &str) -> bool {
    let line = line.trim();
    let Some((version, number)) = line.split_once('/') else { return false };
    let Some((major, minor)) = version.split_once('.') else { return false };
    [major, minor, number].iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

fn unclaimed(line_no: usize, line: &str) -> Rejected {
    Rejected {
        reason: "unclaimed-content",
        detail: format!("line {}: {:.60}", line_no + 1, line),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_tags_are_how_vintage_surprises_surface() {
        let r = parse("1.0/000001\nND: 1-1993\nZZ: boom\n").unwrap_err();
        assert_eq!(r.reason, "unknown-field-code");

        // A column-0 line under a SCALAR field is still unconsumed structure
        // (the flush guard), and one before any field opens has nothing to
        // continue — the issue-199 loosening reaches neither.
        let r = parse("1.0/000001\nND: 1-1993\nstray column-0 prose\n").unwrap_err();
        assert_eq!(r.reason, "unclaimed-content");
        assert!(r.detail.contains("scalar field ND"), "{}", r.detail);
        let r = parse("1.0/000001\nno field open yet\nND: 1-1993\n").unwrap_err();
        assert_eq!(r.reason, "unclaimed-content");

        // A wrapped value under a scalar field is unconsumed structure.
        let r = parse("1.0/000001\nCY: FR\n    extra\n").unwrap_err();
        assert_eq!(r.reason, "unclaimed-content");
    }

    /// Issue 199: TED's own line-wrapper emits a wrapped tail flush-left (after
    /// a `!` in the text, a closing quote/paren, a fixed-width sever), and the
    /// correction margin marker `!` occupies column 0 the same way. All 54
    /// measured members were wrapped CONTENT inside an open field — so a
    /// column-0 non-tag line continues that field instead of holding the
    /// record.
    #[test]
    fn column0_continuations_join_their_open_field() {
        // Prose: the wrapped tail joins the body as its own line (the 1997 US
        // notice's sentence-final '.', the Belgian ') voegen…' family).
        let p = parse("1.0/000001\nND: 1-1993\nTX: body (with a phone\n) and more.\n").unwrap();
        let tx = p.values.iter().find(|v| v.field_id == "TXT-TX").expect("TX emitted");
        assert_eq!(
            *value_text(&tx.value),
            "body (with a phone\n) and more.",
            "the flush-left tail stays in the body"
        );

        // The correction margin marker is presentation, not content: stripped,
        // and the line aligns with its dedented siblings (the 1999 German
        // 'Schlußtermin' family).
        let p = parse("1.0/000001\nND: 1-1993\nTX: 5. b)  Zahlung: 40 DEM.\n!    6. a)  Schlußtermin: 2. 11. 1999.\n").unwrap();
        let tx = p.values.iter().find(|v| v.field_id == "TXT-TX").expect("TX emitted");
        assert_eq!(*value_text(&tx.value), "5. b)  Zahlung: 40 DEM.\n6. a)  Schlußtermin: 2. 11. 1999.");

        // Per-line: a flush-left line is its own value — the 1999 C01 record's
        // second NUTS code and second region name are real values.
        let p = parse("1.0/000001\nND: 1-1993\nRC: ES511\nES512\n").unwrap();
        let codes: Vec<_> = p.values.iter().filter(|v| v.field_id == "TXT-RC").collect();
        assert_eq!(codes.len(), 2, "both NUTS codes are claimed");
        assert_eq!(
            codes[1].value,
            NoticeValue::Classification { scheme: "nuts".into(), code: "ES512".into() }
        );

        // The 1994 mainframe search-command echo is production residue between
        // fields — consumed like layout, even where a continuation could not
        // attach (an open scalar field).
        let p = parse(
            "1.0/000001\nND: 1-1993\nTD: 7 - Something\n.S F=ALL;R=19212 TO 1;SORT=PD;ND;HC   \n",
        )
        .unwrap();
        assert!(p.values.iter().any(|v| v.field_id == "TXT-TD"), "the record parses whole");
    }

    fn value_text(value: &NoticeValue) -> &String {
        match value {
            NoticeValue::Text { value, .. } => value,
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn deadline_shapes_and_code_labels_are_consumed() {
        assert_eq!(
            deadline("19930125  16 00"),
            Some(NoticeValue::Date { utc_seconds: 727_977_600, offset_minutes: 0, has_time: true })
        );
        assert_eq!(
            deadline("19930127"),
            Some(NoticeValue::Date { utc_seconds: 728_092_800, offset_minutes: 0, has_time: false })
        );
        assert_eq!(deadline("whenever"), None);
        assert_eq!(
            code("3 - Invitation to tender"),
            Some(NoticeValue::Code { list: None, code: "3".into() })
        );
        assert_eq!(code("FR"), Some(NoticeValue::Code { list: None, code: "FR".into() }));
        assert_eq!(code("not a code"), None);
    }
    /// Issue 244: the 2006-onward body, verbatim from prod notice 2,821,477 including
    /// the wrap that splits `HAS BEEN` from `AWARDED:`.
    #[test]
    fn a_2006_body_yields_its_winner_as_a_result_with_an_organization() {
        let body = "CONTRACT AWARD NOTICE\n\
                    SECTION V: AWARD OF CONTRACT\n\
                    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR TO WHOM THE CONTRACT HAS BEEN \n\
                    AWARDED: Gagneraud Construction, 198 chemin des Eucalyptus, F-06160 \n\
                    Antibes-Juan-les-Pins.\n\
                    V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the contract:";
        assert_eq!(awarded_names(body), vec!["Gagneraud Construction".to_owned()]);

        // …and the whole record, so the manufactured shape is what the projection reads.
        let record =
            format!("1.0/000001\nND: 1-2006\nTX: {}\n", body.replace('\n', "\n    "));
        let p = parse(&record).expect("parses");
        let results: Vec<&str> =
            p.sections.iter().filter(|s| s.kind == "LotResult").map(|s| s.id.as_str()).collect();
        assert_eq!(results, vec!["RES-1"], "one award, one result section");
        let org = p.sections.iter().find(|s| s.kind == "Organization").expect("an organization");
        assert_eq!(org.parent.as_deref(), Some("RES-1"), "the winner sits inside its award");
        assert!(
            p.values.iter().any(|v| v.field_id == "TED-ADDRESS_CONTRACTOR"
                && v.section_id == "RES-1"
                && matches!(&v.value, NoticeValue::Id { value, is_ref: true, .. } if value == &org.id)),
            "the result references the winner, which is what legacy_role folds onto `winner`"
        );
        assert_eq!(
            p.values
                .iter()
                .find(|v| v.field_id == "TED-OFFICIALNAME")
                .map(|v| value_text(&v.value).as_str()),
            Some("Gagneraud Construction"),
            "the name must sit in a field the projection reads as a name"
        );
        // The prose stays whole: derived facts are ADDED, never moved out (ADR-0004).
        assert!(
            p.values.iter().any(|v| v.field_id == "TXT-TX"
                && value_text(&v.value).contains("AWARDED: Gagneraud Construction")),
            "the body is still claimed verbatim"
        );
    }

    /// The 2004/2005 vintage labels the same value differently, and one notice can
    /// award many contracts — prod notice 3,401,496 awards four under one heading each.
    #[test]
    fn both_label_vintages_and_repeated_awards_are_read() {
        // 2004: `V.1.1)  Name and address of successful supplier, contractor or service
        // provider:` — mixed case, so the match has to be case-insensitive.
        let y2004 = "V.1)  Award and contract value\n\
                     V.1.1)  Name and address of successful supplier, contractor or service \n\
                     provider: Eurovia Méditerranée, Att: Christophe Verweirde, Route de Gréoux\n\
                     V.1.2)  Information on value of contract";
        assert_eq!(awarded_names(y2004), vec!["Eurovia Méditerranée".to_owned()]);

        // An abbreviation keeps its period; a word does not. Measured on prod: this
        // notice's winner is written `… Telematics S.A., 209 Kifissias Avenue …`, and
        // trimming to `S.A` would make it a second organization for the same company
        // (these winners carry no identifier, so the name IS the identity).
        let abbrev = "V.3)  … HAS BEEN AWARDED: European Dynamics Advanced Systems of \n\
                      Telecommunications Informatics and Telematics S.A., 209 Kifissias Avenue.";
        assert_eq!(
            awarded_names(abbrev),
            vec![
                "European Dynamics Advanced Systems of Telecommunications Informatics and \
                 Telematics S.A."
                    .to_owned()
            ]
        );
        assert_eq!(trim_sentence_period("Grahams Engineering Ltd."), "Grahams Engineering Ltd");
        assert_eq!(trim_sentence_period("Foo S.A."), "Foo S.A.");
        assert_eq!(trim_sentence_period("Foo Ltd"), "Foo Ltd");
        // The abbreviation's letter can be preceded by a space — the Polish `sp. j.`, seen
        // in the campaign's own output as `Hurtownia Farmaceutyczna Ismed Sp. J.`
        assert_eq!(trim_sentence_period("Ismed Sp. J."), "Ismed Sp. J.");
        assert_eq!(trim_sentence_period("Balton Spółka z o.o."), "Balton Spółka z o.o.");

        let multi = "SECTION V: AWARD OF CONTRACT\n\
                     CONTRACT NO: 088273\n\
                     V.3)  NAME AND ADDRESS ... HAS BEEN AWARDED: Stryker France, Zac Satolas \n\
                     Green, F-69881 Meyzieu Cedex. Tel. 04 72 45 36 00.\n\
                     V.4)  INFORMATION ON VALUE OF CONTRACT Value: 303 504,79 EUR.\n\
                     CONTRACT NO: 080050\n\
                     V.3)  NAME AND ADDRESS ... HAS BEEN AWARDED: Stryker France, Zac de \n\
                     Satolas Green, F-69881 Meyzieu Cedex.";
        // Both spellings of the address, one name — which is the point of taking only
        // the name: an address-bearing string would mint two organizations here.
        assert_eq!(
            awarded_names(multi),
            vec!["Stryker France".to_owned(), "Stryker France".to_owned()]
        );

        let record =
            format!("1.0/000001\nND: 1-2008\nTX: {}\n", multi.replace('\n', "\n    "));
        let p = parse(&record).expect("parses");
        let results: Vec<&str> =
            p.sections.iter().filter(|s| s.kind == "LotResult").map(|s| s.id.as_str()).collect();
        assert_eq!(results, vec!["RES-1", "RES-2"], "two contracts, two results");
        // Distinct organization sections, each parented to its own award, and neither
        // colliding with the buyer's ORG-1.
        let orgs: Vec<(&str, Option<&str>)> = p
            .sections
            .iter()
            .filter(|s| s.kind == "Organization")
            .map(|s| (s.id.as_str(), s.parent.as_deref()))
            .collect();
        assert_eq!(orgs, vec![("ORG-2", Some("RES-1")), ("ORG-3", Some("RES-2"))]);
    }

    /// Issue 232's recorded follow-on: `CY:` and `TW:` describe the awarding
    /// authority, so a record that opened `ORG-1` carries them there — beside the
    /// name, where the projection's `ORG_COUNTRY_FIELDS` can reach the country.
    /// The order matters and is the fixtures' real header order: `CY:` is
    /// published BEFORE `AU:`, when no authority section exists yet, which is
    /// what makes this a post-pass rather than routing in `flush`.
    #[test]
    fn the_authoritys_country_and_town_land_beside_its_name() {
        fn homes<'p>(p: &'p Parsed, field: &str) -> Vec<&'p str> {
            p.values
                .iter()
                .filter(|v| v.field_id == field)
                .map(|v| v.section_id.as_str())
                .collect()
        }
        let record =
            "1.0/000001\nND: 1-2008\nCY: FR\nAU: ECOLE NATIONALE DES PONTS\nTW: MARNE-LA-VALLEE\n";
        let p = parse(record).expect("parses");
        assert_eq!(homes(&p, "TXT-AU"), vec!["ORG-1"]);
        assert_eq!(homes(&p, "TXT-CY"), vec!["ORG-1"], "the country joins the authority");
        assert_eq!(homes(&p, "TXT-TW"), vec!["ORG-1"], "the town joins the authority");
        // Still a Code: the mention reader matches `NoticeValue::Code` only, so a
        // re-homed country that degraded to text would silently give the org nothing.
        assert!(p.values.iter().any(|v| v.field_id == "TXT-CY"
            && matches!(&v.value, NoticeValue::Code { code, .. } if code == "FR")));

        // No `AU:`, no authority section — the descriptors keep their root residence
        // rather than manufacturing an org section that names nobody.
        let p = parse("1.0/000001\nND: 2-2008\nCY: DE\nTW: BONN\n").expect("parses");
        assert_eq!(homes(&p, "TXT-CY"), vec!["PROCEDURE"]);
        assert_eq!(homes(&p, "TXT-TW"), vec!["PROCEDURE"]);
    }

    /// Issue 436: a NAME the wrapper broke is one line again — the authority (`AU`)
    /// and its town (`TW`) — joined with one space, runs collapsed: the unwrap issue
    /// 397 gave titles. The two `AU` breaks are the shapes measured on prod. A break after
    /// a hyphen keeps the text as published, `RECHNER- UND`, space included: that is
    /// a German suspended compound ("Rechner- und Netzwerktechnologie"), and gluing
    /// it would invent a word. The contractor list `CO` is a LIST, one supplier per
    /// line, so it stays prose and keeps its lines.
    #[test]
    fn a_wrapped_name_rejoins_on_one_line() {
        let only = |p: &Parsed, field: &str| -> String {
            let found: Vec<&NoticeValue> =
                p.values.iter().filter(|v| v.field_id == field).map(|v| &v.value).collect();
            assert_eq!(found.len(), 1, "one {field}: {found:?}");
            value_text(found[0]).clone()
        };
        let record = "1.0/000001\nND: 1-2005\n\
            AU: UNIVERSITAET BEISPIELSTADT, ZENTRUM FUER\n    RECHNER- UND \n    NETZWERKTECHNOLOGIE\n\
            TW: GARCHING BEI\n    MUENCHEN\n\
            CO: Name and address of successful supplier: \n    Acme Ltd.\n    Beta GmbH.\n";
        let p = parse(record).expect("parses");
        assert_eq!(only(&p, "TXT-AU"), "UNIVERSITAET BEISPIELSTADT, ZENTRUM FUER RECHNER- UND NETZWERKTECHNOLOGIE");
        assert_eq!(only(&p, "TXT-TW"), "GARCHING BEI MUENCHEN");
        assert_eq!(only(&p, "TXT-CO"), "Name and address of successful supplier:\nAcme Ltd.\nBeta GmbH.");

        let p = parse("1.0/000001\nND: 2-2005\nAU: MINISTERE DE LA DEFENSE,  SOUS-DIRECTION DE LA\n    COMMUNICATION\n")
            .expect("parses");
        assert_eq!(only(&p, "TXT-AU"), "MINISTERE DE LA DEFENSE, SOUS-DIRECTION DE LA COMMUNICATION");
    }

    /// The scan must stay linear in the body, and must refuse a value with no boundary
    /// in sight rather than inventing a name out of the window (issue 244).
    #[test]
    fn a_long_body_with_many_awards_stays_cheap_and_never_invents_a_name() {
        // 400 awards in one body — the shape that took minutes per notice when the
        // name's end was looked for in the whole remainder.
        let one = "CONTRACT NO: 1 V.3)  … HAS BEEN AWARDED: Acme Ltd, 1 Road, Town. ";
        let body: String = std::iter::repeat_n(one, 400).collect();
        let names = awarded_names(&body);
        assert_eq!(names.len(), 400, "every award is read");
        assert!(names.iter().all(|n| n == "Acme Ltd"), "{:?}", &names[..3]);

        // A value with no comma and no following heading inside the window yields
        // NOTHING: a 256-byte name would be one organization per notice.
        let runaway = format!("V.3)  HAS BEEN AWARDED: {}", "x".repeat(400));
        assert!(awarded_names(&runaway).is_empty(), "no boundary, no name");

        // And the window is cut on a char boundary, not mid-character: a body whose
        // 256th byte lands inside a multi-byte character must not panic.
        let multibyte = format!("V.3)  HAS BEEN AWARDED: {}, rest", "é".repeat(200));
        assert_eq!(awarded_names(&multibyte).len(), 0, "no boundary within the window");
        let near = format!("V.3)  HAS BEEN AWARDED: {}é, rest", "é".repeat(120));
        let _ = awarded_names(&near); // must not panic
    }

    /// The index arithmetic must survive a character whose uppercase is a different
    /// LENGTH, and the gate must not swallow a real award (issue 244).
    #[test]
    fn a_non_ascii_body_is_indexed_by_bytes_not_by_its_uppercase_copy() {
        // `ı` (Turkish dotless i, 2 bytes) uppercases to `I` (1 byte). Under the old
        // `to_uppercase().find()` the label's index came from a string one byte shorter
        // than the one being sliced, so the name came out shifted — or the slice landed
        // mid-character and panicked.
        let body = "V.3)  Bakırköy Belediyesi tender … HAS BEEN AWARDED: Çınar İnşaat A.Ş., \n\
                    Bakırköy, TR-34140 İstanbul.";
        assert_eq!(awarded_names(body), vec!["Çınar İnşaat A.Ş.".to_owned()]);

        // The gate is a cheap pre-check, not a filter: a body that says AWARD anywhere
        // still goes through the scan.
        assert!(find_ascii_ci("section v: award of contract", "AWARD").is_some());
        assert!(find_ascii_ci("SECTION II: OBJECT", "AWARD").is_none());
        // And it is ASCII-case-insensitive both ways round.
        assert_eq!(find_ascii_ci("xxAwArDedxx", "awarded"), Some(2));
        assert_eq!(find_ascii_ci("short", "longer needle"), None);
    }

    /// A body with no award block must manufacture nothing — most of the era is not an
    /// award notice at all, and inventing empty results would put 2.5M phantom rows in
    /// the results layer.
    #[test]
    fn a_body_without_an_award_label_manufactures_no_result() {
        assert!(awarded_names("SECTION II: OBJECT OF THE CONTRACT\nII.1) Description").is_empty());
        let p = parse("1.0/000001\nND: 1-2006\nTX: SECTION II: OBJECT\n").expect("parses");
        assert!(p.sections.iter().all(|s| s.kind != "LotResult"));
        // And the 1993 flat grammar is NOT yet read (issue 244's second slice), so it
        // must fail closed rather than half-read: no result, no phantom winner.
        assert!(awarded_names(" 6.  Supplier(s): A: Apotecnia, Climo").is_empty());
    }

    /// Issue 244, pre-2004 slice: the numbered form's two winner labels, both bodies
    /// verbatim from prod notices in the 2001-06 monthly (fetch 300, the package the
    /// campaign was re-parsing when the coverage gap was measured — 619 TD:7 award
    /// records in a 2,000-notice band, of which the sectioned labels matched one).
    #[test]
    fn the_pre_2004_numbered_form_yields_its_winner() {
        // notice 1,710,454 — the works/services form, winner at item 6.
        let redcar = "1.  Awarding authority: Redcar and Cleveland Borough Council, Economic
                      Development Department, Cargo Fleet Offices, Middlesbrough Road, PO Box 
                      South Bank 20, UK-Middlesbrough TS6 6EL. 
                      2.  Award procedure, justification (Article 7(4)): Negotiated.
                      3.  Date of award: 30.3.2001.
                      4.  Award criteria: Most economically advantageous.
                      5.  Tenders received: 2.
                      6.  Successful contractor(s): Mill Group, 3 Burlington Mews, UK-London 
                      W1R 8QA.
                      7.  Works provided: CPV: 45210000, 74222000, 74873100.";
        assert_eq!(awarded_names(redcar), vec!["Mill Group".to_owned()]);

        // notice 1,710,456 — same form, a longer name, and the wrap inside the address.
        let southwark = "2.  Award procedure, justification (Article 7(4)): Restricted procedure.
                         5.  Tenders received: 6.
                         6.  Successful contractor(s): Independent Lift Services Ltd, Unit 3J, 
                         Barlow Way, Fairview Industrial Park, Manor Way, UK-Rainham RM13 8BT, 
                         Essex.
                         7.  Works provided: CPV: 45510000.";
        assert_eq!(awarded_names(southwark), vec!["Independent Lift Services Ltd".to_owned()]);

        // notice 1,710,387 — the EC external-aid form, winner at item 8, and a consortium
        // name that the comma boundary correctly keeps whole (the comma is the one before
        // the street, not one inside the name).
        let starcm = "Service contract award notice
                      5.  Date of award of the contract: 11.5.2001.
                      6.  Number of tenders received: 6.
                      7.  Overall score of chosen tender: 100%.
                      8.  Name and address of successful tenderer: Symonds Travers Morgan Ltd 
                      (UK) in association with Tecnica y Proyectos SA (ES), Symonds House, Wood 
                      Street, UK-East Grinstead RH19 1 UU, West Sussex.";
        assert_eq!(
            awarded_names(starcm),
            vec!["Symonds Travers Morgan Ltd (UK) in association with Tecnica y Proyectos SA (ES)"
                .to_owned()]
        );

        // A winner with no comma after the name: the next numbered item is the boundary,
        // without which the name runs on into item 7 (see NAME_STOPS).
        assert_eq!(
            awarded_names(
                "Award notice 6.  Successful contractor(s): ACME Ltd. \n\
                 7.  Works provided: CPV: 45210000, 74222000."
            ),
            vec!["ACME Ltd".to_owned()]
        );

        // The labels are ENGLISH even when the notice is not: prod notice 1,710,458 has
        // `OL: FR` and a French buyer and winner, under `1. Awarding authority:` and
        // `6. Successful contractor(s):`. So this grammar is not an English-only slice of
        // the era — it reaches every language's bodies. The name here also has no comma
        // before its period, so the ` 7.` stop is what keeps the next item out of it, and
        // `Rhône` makes the window a byte window over a multi-byte character.
        let lyon = "1.  Awarding authority: Communauté urbaine de Lyon, délégation générale\n\
                    aux services urbains et à la proximité, F-69399 Lyon Cedex 03. \n\
                    6.  Successful contractor(s): Groupement d'entreprises CGEV \n\
                    Rhône-Alpes/Parcs et Sports.\n\
                    7.  Works provided: CPV: 45112430, 77321000.\n\
                    8.  Price: 5 301 802,22 FRF TTC.";
        assert_eq!(
            awarded_names(lyon),
            vec!["Groupement d'entreprises CGEV Rhône-Alpes/Parcs et Sports".to_owned()]
        );

        // The singular spellings the era also uses.
        assert_eq!(
            awarded_names("3. Date of award: 1.1.2001. 6. Successful contractor: Mill Group, 3 Burlington Mews."),
            vec!["Mill Group".to_owned()]
        );
        assert_eq!(
            awarded_names("Award notice 8. Name and address of successful tenderer(s): Acme Ltd, Wood Street."),
            vec!["Acme Ltd".to_owned()]
        );
    }

    /// Issue 244 slice 3: the supplies and utilities forms, which name the winner under a
    /// different item and a different word — measured as four fifths of the era's awards.
    /// Every body here is verbatim from prod.
    #[test]
    fn the_supplies_and_utilities_forms_yield_their_winners() {
        // notice 1,456,070 (fetch 319, 1999-11) — utilities, winner at item 9, and TWO of
        // them in one value, `;`-separated, each `Name, City`.
        let wiesbaden = "1.  Contracting entity: Stadtwerke Wiesbaden AG, Postfach 55 40, D-65045\n\
                         Wiesbaden.\n\
                         5.  Award procedure: Verhandlungsverfahren.\n\
                         6.  Tenders received: 11.\n\
                         7.  Date of award: 30. 8. 1999.\n\
                         9.  Supplier(s), contractor(s) or service provider(s): BP, Hamburg; \n\
                         Thelen, Mainz.\n\
                         10.  \n\
                         11.  Other information: Auftragsart: Lieferauftrag.";
        assert_eq!(awarded_names(wiesbaden), vec!["BP".to_owned(), "Thelen".to_owned()]);

        // notice 1,710,469 (fetch 300, 2001-06) — supplies, `Supplier(s):` at item 6.
        let wien = "1.  Awarding authority: Bundesministerium für Landesverteidigung, A-1090 Wien.\n\
                    3.  Date of award: 14.5.2001.\n\
                    5.  Tenders received: 4.\n\
                    6.  Supplier(s): Kovosluzba, Priemyselna 4, 04234 Kosice, Slowakei.\n\
                    7.  Goods, CPA reference number: CPV: 28632200, 36121120.\n\
                    8.  Price: Gezahlter Preis ohne USt.: 15 564 000 ATS / 1 131 079,99 EUR.";
        assert_eq!(awarded_names(wien), vec!["Kovosluzba".to_owned()]);

        // The 1993 supplies form keys winners to their lots, in every spelling the
        // committed `1993-daily-en-19930102` fixture uses. Taken verbatim these would mint
        // `1: Ailsa Truck and Bus Limited` — a second spelling of a company that also
        // appears unprefixed, and the name IS the identity here (issue 234).
        let lots = |v: &str| awarded_names(&format!("Award notice 6.  Supplier(s): {v}\n 7.  Goods."));
        assert_eq!(lots("A: Apotecnia, Climo"), vec!["Apotecnia".to_owned()]);
        assert_eq!(
            lots("1: Ailsa Truck and Bus Limited, 101 Kelburn Street,"),
            vec!["Ailsa Truck and Bus Limited".to_owned()]
        );
        assert_eq!(
            lots("1/2: Evans MacShaw Leyland DAF Limited, Shefford Road,"),
            vec!["Evans MacShaw Leyland DAF Limited".to_owned()]
        );
        assert_eq!(
            lots("1, 2, 3 and 4: Dolmen Computer Applications NV,"),
            vec!["Dolmen Computer Applications NV".to_owned()]
        );
        assert_eq!(lots("1: Discol."), vec!["Discol".to_owned()]);
        // …and lot-keyed AND multi-winner at once, which is where the two rules meet.
        assert_eq!(
            lots("1: Baxter Healthcare; 2: B. Braun Medical; 3: Fresenius Ltd."),
            vec!["Baxter Healthcare".to_owned(), "B. Braun Medical".to_owned(), "Fresenius Ltd".to_owned()]
        );
        // A name that merely contains a colon further in is NOT truncated to nothing.
        assert_eq!(lots("Compagnie IBM France, F-92400 Courbevoie."), vec!["Compagnie IBM France".to_owned()]);

        // notice 21,133 (fetch 400, 1993-02) — the same label eight years earlier, so this
        // slice reaches the era's oldest packages too.
        let persiceto = "1.  Awarding authority: Amministrazione comunale, I-40017 San Giovanni.\n\
                          3.  Date of award: 9. 12. 1992.\n\
                          5.  Tenders received: 1.\n\
                          6.  Supplier(s): CAMST Scrl, via Tosarelli 318, Villanova di Castenaso (BO)\n\
                          .\n\
                          7.  Goods supplied: Foodstuffs to make school-canteen meals.";
        assert_eq!(awarded_names(persiceto), vec!["CAMST Scrl".to_owned()]);
    }

    /// Two ways the era fills the winner item with something that is NOT a winner. Both
    /// must mint nothing — an organization named `99` or `A: Apotecnia` is worse than a
    /// missing winner, because these names carry no identifier and so ARE the identity.
    #[test]
    fn a_winner_item_that_holds_no_name_mints_nothing() {
        // notice 21,123 (1993-02): under the supplies form this item sometimes holds the
        // NUMBER of suppliers.
        let count = "1.  Awarding authority: Unita sanitaria locale BA/8, I-70032 Bitonto.\n\
                     5.  Tenders received: 110.\n\
                     6.  Supplier(s): 99.\n\
                     7.  Goods supplied: Therapeutic substances.";
        assert!(awarded_names(count).is_empty(), "a count became a name: {:?}", awarded_names(count));

        // `Various.` is the era's word for "no single answer" — four times in the committed
        // 1993 daily alone. An organization called `Various` is worse than no winner.
        let various = "Award notice 6.  Supplier(s): Various.\n 7.  Goods supplied: Fuel.";
        assert!(awarded_names(various).is_empty(), "{:?}", awarded_names(various));

        // notice 1,710,467 (2001-06): a CANCELLED procedure — item 6 is empty and item 11
        // says so. There is no winner to find, and none is invented.
        let cancelled = "1.  Awarding authority: Direction départementale de l'équipement.\n\
                         2.  Award procedure, justification (Article 7(4)): Appel d'offres ouvert.\n\
                         6.\n\
                         7.  Works provided: CPV: 45112210, 45233220.\n\
                         11.  Other information: Procédure annulée: décision de la PRM du 15.5.2001.";
        assert!(awarded_names(cancelled).is_empty());
    }

    /// Issue 244 slice 9: a `Contract No <ref>:` prefix between the label and the name
    /// is hopped, so the published winner behind it is read. Verbatim from prod notice
    /// 2,368,067 (2003), one of the residue read's four specimens — `CONTRACT NO` is
    /// also an ITEM_STOP, so before the hop this value ended before it began.
    #[test]
    fn a_contract_no_prefix_between_label_and_name_is_hopped() {
        let body = "Section V: Award of contract\n\
                    V.1.1)  Name and address of the successful supplier, contractor or service \n\
                    provider: Contract No 04/2004/OIL:\n\
                    Martin Reinert Sàrl, Mr Martin Reinert, 2, Op Tomm (Zone industrielle), \n\
                    L-5485 Wormeldange-Haut. Tel.: (352) 76 92 98.\n\
                    V.1.2)  Information on value of contract: Lowest tender: 268 352,20 EUR.";
        assert_eq!(awarded_names(body), vec!["Martin Reinert Sàrl".to_owned()]);

        // Mid-window the marker keeps its boundary meaning: the multi-contract shape's
        // second award is NOT swallowed into the first value.
        let two = "Award notice V.3) TO WHOM THE CONTRACT HAS BEEN AWARDED: Acme Ltd, Wood Street. \
                   CONTRACT NO 2: V.3) TO WHOM THE CONTRACT HAS BEEN AWARDED: Bolt GmbH, Ringstr. 1.";
        assert_eq!(awarded_names(two), vec!["Acme Ltd".to_owned(), "Bolt GmbH".to_owned()]);

        // A comma before the colon means prose, not a reference — no hop, and the
        // ITEM_STOP still bounds the value to nothing rather than minting the prose.
        let prose = "Award notice 6.  Supplier(s): Contract no pending, see notes: none. 7.  Goods: X.";
        assert_eq!(awarded_names(prose), Vec::<String>::new());
    }

    /// Issue 484 defect 1: the numbered value reads to its own successor item, a
    /// contract reference is not a name, and the authority's own contact entry is
    /// dropped when another entry survives. Bodies are the stored `TXT-TX` of the
    /// exhibits, verbatim (wrap included), from `/v1/notices/{id}/content`.
    #[test]
    fn the_authoritys_contact_entry_is_not_its_own_contractor() {
        // 2002406 (TED 190812-2002): entry `1:` repeats the item-1 authority with its
        // e-mail and URL; the real supplier is entry `2:`, past the old 256-byte window.
        let vasco = "1.  Awarding authority: Gobierno Vasco, Departamento de Justicia, Empleo\n\
                     y Seguridad Social, c/ Donostia-San Sebastián, 1, entreplanta, zona F, \n\
                     E-01010 Vitoria-Gasteiz. Tel.: 945 01 90 98. Fax: 945 01 94 50. E-mail: \n\
                     mv-ruiz@ej-gv.es. URL: www.ej-gv.net\n\
                     2.  Award procedure, justification (Article 6(3)): Open procedure.\n\
                     3.  Date of award: 25.10.2002.\n\
                     5.  Tenders received: 6.\n\
                     6.  Supplier(s): Contrato n° S-036/02-DJ.\n\
                     1: Gobierno Vasco, A la atención de Mesa de Contratación del Departamento \n\
                     de Justicia, Empleo Seguridad Social, C/ Donostia-San Sebastián, 1, \n\
                     Edifico Lakua I, entreplanta zona F, E-01010 Vitoria-Gasteiz. Teléfono: \n\
                     945-01.9098, Fax: 945-01.94.50, Correo electrónico: mv-ruiz@ej-gv.es, \n\
                     Dirección Internet (URL): www.ej-gv.net.\n\
                     2: Montte, Polígono Industrial 10, E-20200 Beasain (Gipuzkoa). Teléfono: \n\
                     902-108888, Fax: 902-208888.\n\
                     7.  Goods, CPA reference number: CPV: 21230000.\n\
                     8.  Price: Precios unitarios.";
        assert_eq!(awarded_names(vasco), vec!["Montte".to_owned()]);
        // …and through the whole record: one result, and its winner is Montte.
        let record = format!("1.0/000001\nND: 190812-2002\nTX: {}\n", vasco.replace('\n', "\n    "));
        let p = parse(&record).expect("parses");
        let winners: Vec<&str> = p
            .values
            .iter()
            .filter(|v| v.field_id == "TED-OFFICIALNAME")
            .map(|v| value_text(&v.value).as_str())
            .collect();
        assert_eq!(winners, vec!["Montte"]);

        // 1200610 (Staffanstorp): `1:` is the municipality's own cleaning service — the
        // same name, but no shared e-mail or URL — a genuine in-house lot, kept. The
        // successor bound stops at ` 8.  Price(s): 1: 2 750 000 SEK`, which a blanket
        // wide window would read as winners.
        let staffanstorp = "1.  Awarding authority: Staffanstorps kommun,  S-245 80 Staffanstorp.\n\
                            Tel. (046) 25 11 00. Facsimile (046) 25 55 70.\n\
                            6.  Tenders received: 9.\n\
                            7.  Service provider(s): 1: Staffanstorps kommun, Städservice, S-245 80 \n\
                            Staffanstorp, tel. (046) 25 14 27, facsimile (046) 25 11 66.\n\
                            2: Clean Service System AB, Box 41, S-291 21 Kristianstad, tel. (044) 10 \n\
                            60 38, facsimile (044) 10 60 39.\n\
                            8.  Price(s): Prices (for the duration of the contract):\n\
                            1: 2 750 000 SEK; 2: 245 000 SEK.\n\
                            9.\n\
                            10.\n\
                            11.  Notice published on: 21. 10. 1997.";
        assert_eq!(
            awarded_names(staffanstorp),
            vec!["Staffanstorps kommun".to_owned(), "Clean Service System AB".to_owned()]
        );

        // 3002722: the single V.3 entry repeats the authority's block (same e-mail, same
        // URL). With no other entry it is what the notice says, and stays as published.
        let toledo = "SECTION I: CONTRACTING AUTHORITY\n\
                      I.1)  NAME, ADDRESSES AND CONTACT POINT(S): Consejería de Educación y \n\
                      Ciencia, Bulevar del Río Alberche, s/nº, Attn: Servicio de Planificación y \n\
                      Centros (Secretaría General), E-45007 Toledo. Tel. 925 247439/27/17. \n\
                      E-mail: jaramon@jccm.es. Fax 925 247442.\n\
                      Internet address(es):\n\
                      General address of the contracting authority: www.jccm.es/contratacion.\n\
                      SECTION II: OBJECT OF THE CONTRACT\n\
                      SECTION V: AWARD OF CONTRACT\n\
                      CONTRACT NO: 33/06\n\
                      V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR TO WHOM THE CONTRACT HAS BEEN \n\
                      AWARDED: Consejería de Educación y Ciencia, A la atención de Servicio de \n\
                      Planificación y Centros (Secretaría General), Bulevar del Río Alberche \n\
                      s/nº, E-45007 Toledo. E-mail: jaramon@jccm.es. Tel. 925247439/17/27. URL: \n\
                      www.jccm.es/contratacion. Fax 925 247442.\n\
                      V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the contract:";
        assert_eq!(awarded_names(toledo), vec!["Consejería de Educación y Ciencia".to_owned()]);
        let auth = awarding_authority(&flatten(toledo)).expect("the I.1) authority is read");
        assert_eq!(auth.contacts, vec!["jaramon@jccm.es".to_owned(), "jccm.es".to_owned()]);

        // The same Vasco value with the authority entry ALONE: nothing else survives, so
        // it is served as published (the projection's `is_buyer` flag is unit 3).
        let alone = vasco.replace(
            "2: Montte, Polígono Industrial 10, E-20200 Beasain (Gipuzkoa). Teléfono: \n\
                     902-108888, Fax: 902-208888.\n",
            "",
        );
        assert_eq!(awarded_names(&alone), vec!["Gobierno Vasco".to_owned()]);

        // The contact match is the safeguard, not the name: the same Vasco value with
        // entry 1's e-mail and URL replaced by ones the authority does not publish is a
        // separate office of the same name, and is kept beside Montte.
        let other_office = vasco
            .replace("Correo electrónico: mv-ruiz@ej-gv.es", "Correo electrónico: compras@lakua.es")
            .replace("Dirección Internet (URL): www.ej-gv.net.", "Dirección Internet (URL): www.lakua.es.");
        assert_eq!(awarded_names(&other_office), vec!["Gobierno Vasco".to_owned(), "Montte".to_owned()]);

        // Contract references in the era's languages are hopped through their
        // terminator; a segment that is only a reference is nothing; names that merely
        // start the same way are untouched.
        assert_eq!(contract_reference_len("Contrato n° S-036/02-DJ."), Some(25));
        assert_eq!(contract_reference_len("Contrato n° S-036/02-DJ"), Some(24), "only a reference");
        assert_eq!(contract_reference_len("Marché n° 03/010002: 1) Biotronik"), Some(22));
        assert_eq!(contract_reference_len("Vertrag Nr. 12: Acme"), Some(15));
        assert_eq!(contract_reference_len("Contract No. 12.3: Acme"), Some(18));
        assert_eq!(contract_reference_len("Contract Northern Ltd"), None);
        assert_eq!(contract_reference_len("Contratos Navales SA"), None);
    }

    /// Issue 484 review: the shapes next to 2002406 in the same OJ and era. Bodies are
    /// the stored `TXT-TX` from `/v1/notices/{id}/content`, verbatim.
    #[test]
    fn the_sibling_shapes_of_the_contact_entry() {
        // 2002408: an UNKEYED value — reference, the authority's contact entry, then the
        // real supplier Profinsa after the authority's URL. Before: `Contrato n°
        // S-037/02-DJ. Gobierno Vasco` (and Profinsa unread).
        let profinsa = "1.  Awarding authority: Gobierno Vasco, Departamento de Justicia, Empleo\n\
                        y Seguridad Social, c/ Donostia-San Sebastián, 1, entreplanta, zona F, \n\
                        E-01010 Vitoria-Gasteiz. Tel.: 945-019098. 945-019100. Fax: 945-019450. \n\
                        E-mail: mv-ruiz@ej-gv.es URL: www.ej-gv.net\n\
                        2.  Award procedure, justification (Article 6(3)): Open procedure.\n\
                        3.  Date of award: 1.10.2002.\n\
                        4.  Award criteria: Economically most advantageous offer assessed on the \n\
                        basis of price, quality, time limit for completion and other criteria.\n\
                        5.  Tenders received: 10.\n\
                        6.  Supplier(s): Contrato n° S-037/02-DJ.\n\
                        Gobierno Vasco, a la atención de Mesa de Contratación del Departamento de \n\
                        Justicia, Empleo Seguridad Social, c/ Donostia-San Sebastián, 1, Edifico \n\
                        Lakua I, entreplanta zona F., E-01010 Vitoria-Gasteiz. Tel.: 945-01 90 \n\
                        98. Fax: 945-01 94 50. E-mail: mv-ruiz@ej-gv.es. URL: www.ej-gv.net.\n\
                        Profinsa, Productos de Oficina e Informática, Plaza San Martín, 4, \n\
                        E-01009 Vitoria-Gasteiz. Tel.: 945-22 22 69. Fax: 945-24 02 65.\n\
                        7.  Goods, CPA reference number: CPV: 30000000.\n\
                        Office and computing machinery, equipment and supplies. Total estimated \n\
                        value, excluding VAT: 403 681,03 EUR.\n\
                        8.  Price: Precios unitarios.\n\
                        9.\n\
                        10.  Subcontract: Sí.\n\
                        Importe (sin IVA): porcentaje 50 %.\n\
                        11.  Other information: Tipo de poder adjudicador: administración \n\
                        regional / local.\n\
                        Tipo de contrato: suministros.\n\
                        ¿Se trata de un contrato marco? No.\n\
                        ¿Se trata de un anuncio no obligatorio? No.\n\
                        ¿Se relaciona el contrato con un proyecto o programa financiado por \n\
                        fondos de la UE? No.\n\
                        12.  Contract notice published on: 10.7.2002.\n\
                        2002/S 132-103321.\n\
                        13.  Notice postmarked: 5.12.2002.\n\
                        14.  Notice received on: 5.12.2002.";
        assert_eq!(awarded_names(profinsa), vec!["Profinsa".to_owned()]);

        // 2002409: `Marché n° …:` then `N)`-keyed entries. Before: `Marché n°
        // 03/010002: 1) Biotronik France`, and six suppliers unread.
        let pacemakers = "1.  Awarding authority: SIHCUS-CMCO, 19, rue Louis Pasteur, BP 120,\n\
                        F-67303 Schiltigheim. Tel.: 3 88 62 83 31. Fax: 3 88 62 84 21. E-mail: \n\
                        sihcus-cmco.administration@wanadoo.fr. Att: M. Daniel Prange. \n\
                        2.  Award procedure, justification (Article 6(3)): Open procedure.\n\
                        3.  Date of award: 6.11.2002.\n\
                        4.  Award criteria: Economically most advantageous offer assessed on the \n\
                        basis of price, time limit for completion and other criteria.\n\
                        5.  Tenders received: 8.\n\
                        6.  Supplier(s): Marché n° 03/010002:\n\
                        1) Biotronik France, à l'attention de M. Alain Van Michel, 2, rue Nicolas \n\
                        Ledoux, Silic, F-94528 Rungis, tel.: 1 46 75 96 60, télécopieur: 1 49 76 \n\
                        08 81.\n\
                        2) Ela Médical, à l'attention de M. Christian Bak, centre d'affaires La \n\
                        Boursidière, F-92357 Le-Plessis-Robinson Cedex, tel.: 1 46 01 33 33, \n\
                        télécopieur: 1 46 01 34 58.\n\
                        3) Guidant France, à l'attention de Mme Catherine Jérosme, 9, rue \n\
                        d'Estienne d'Orves, F-92504 Rueil-Malmaison Cedex, tel.; 1 47 14 49 14, \n\
                        télécopieur: 1 47 49 09 57.\n\
                        4) Medtronic France, à l'attention de M. Yves Drapp, 122, avenue du \n\
                        Général Leclerc, F-92514 Boulogne-Billancourt Cedex, tel.: 1 55 38 17 00, \n\
                        télécopieur: 1 55 38 18 00.\n\
                        5) Sorin Biomédica France, à l'attention de M. Yannick Gasnier, 9, rue \n\
                        Georges Besse, F-92160 Antony, tel.: 1 46 11 52 71, télécopieur: 1 46 66 \n\
                        20 11.\n\
                        6) St-Jude Médical France SAS, à l'attention de M. Claude Van \n\
                        Droogenbroeck, 1, rond-point Victor Hugo, F-92137 Issy-les-Moulineaux \n\
                        Cedex, tel.: 1 41 46 45 00, télécopieur: 1 41 46 45 45.\n\
                        7) Vitatron, à l'attention de M. Olivier Clapeau, 16, rue Jean-Jacques \n\
                        Rousseau, BP 110, F-92184 Issy-les-Moulineaux Cedex, tel.: 1 46 48 01 01, \n\
                        télécopieur: 1 46 49 77 55.\n\
                        7.  Goods, CPA reference number: CPV: 33100000, 33182210.\n\
                        Medical devices. Pacemaker.\n\
                        8.  Price: Montants HT:\n\
                        1) 9 449,02 EUR;\n\
                        2) 57 285,17 EUR;\n\
                        3) 71 610 EUR;\n\
                        4) 179 461,88 EUR;\n\
                        5) 671,92 EUR;\n\
                        6) 11 890,04 EUR;\n\
                        7) 74 258,33 EUR.\n\
                        9.\n\
                        10.  Subcontract: Non.\n\
                        11.  Other information: Le marché est-il couvert par l'Accord sur les \n\
                        Marchés Publics (AMP): oui.\n\
                        Type de marché: fournitures.\n\
                        S'agit-il d'un avis non obligatoire: non.\n\
                        Numéro de référence attribué au dossier par le pouvoir adjudicateur: \n\
                        03/010002.\n\
                        12.  Contract notice published on: 27.8.2002.\n\
                        2002/S 165-132895.\n\
                        13.  Notice postmarked: 3.12.2002.\n\
                        14.  Notice received on: 3.12.2002.";
        assert_eq!(
            awarded_names(pacemakers),
            [
                "Biotronik France",
                "Ela Médical",
                "Guidant France",
                "Medtronic France",
                "Sorin Biomédica France",
                "St-Jude Médical France SAS",
                "Vitatron",
            ]
            .map(str::to_owned)
        );

        // 900123: `N)` keys without a reference. Before: `1) CGC`, Furic unread.
        let mackerel = "1.  Awarding authority: Ministère de la défense, service central d'études\n\
                        et de réalisations du commissariat de l'armée de terre, 1, boulevard \n\
                        Louis-Loucheur, F-92211 Saint-Cloud Cedex.\n\
                        Tel. (1) 49 11 64 99.\n\
                        2.  Award procedure, justification (Article 6 (3)): Restricted procedure.\n\
                        3.  Date of award: 8. 3. 1996.\n\
                        4.  Award criteria: Economically most advantageous offer assessed on the \n\
                        basis of: price, quality.\n\
                        5.  Tenders received: 3.\n\
                        6.  Supplier(s): 1) CGC, BP 129, F-56004 Vannes Cedex.\n\
                        2) Furic (Jules et Alain) et Fils.\n\
                        7.  Goods, CPA reference number: CPV: 15201415.\n\
                        Canned and other prepared or preserved mackerel.\n\
                        8.\n\
                        9.\n\
                        10.  Other information: CCP n° 6066.\n\
                        11.  Notice published on: 15. 9. 1995.\n\
                        12.  This notice postmarked: 4. 4. 1996.\n\
                        13.  This notice received on: 9. 4. 1996.";
        assert_eq!(awarded_names(mackerel), vec!["CGC".to_owned(), "Furic (Jules et Alain) et Fils".to_owned()]);

        // 1200611: item 7 ends at its successor ` 8.`, so the price list is not read.
        // Before: + `Price(s): 1: 208 332 GBP p.a.`, `164 811 GBP p.a.`.
        let eastbourne = "1.  Awarding authority: Eastbourne Borough Council, Town Hall, Grove\n\
                        Road, UK-Eastbourne BN21 4UG, East Sussex.\n\
                        2.  Award procedure chosen, justification (Article 11 (3)): Restricted \n\
                        procedure for both contracts.\n\
                        3.  Category of service and description, CPC reference number: CPV: \n\
                        01411200.\n\
                        Part B, category 27 for both contracts.\n\
                        1: contract A: grounds maintenance - cemeteries and crematorium.\n\
                        2: contract B: grounds maintenance - highway verges and amenity areas.\n\
                        4.  Date of award of the contract: 1: 23. 3. 1998; 2: 18. 2. 1998.\n\
                        5.  Criteria: Most economically advantageous tender.\n\
                        6.  Tenders received: 1: 6; 2: 4.\n\
                        7.  Service provider(s): 1: Serco Limited, Serco House, Hayes Road, \n\
                        UK-Southall, Middlesex.\n\
                        2: Serviceteam Limited, Nuthampstead, UK-Royston SG8 8LZ, Hertfordshire.\n\
                        8.  Price(s): 1: 208 332 GBP p.a.; 2: 164 811 GBP p.a.\n\
                        9.\n\
                        10.\n\
                        11.\n\
                        12.  Notice postmarked: 14. 4. 1998.\n\
                        13.  Notice received on: 14. 4. 1998.\n\
                        14.";
        assert_eq!(awarded_names(eastbourne), vec!["Serco Limited".to_owned(), "Serviceteam Limited".to_owned()]);
    }

    /// Issue 484 review: a numbered value whose successor is MISSING ends at the next
    /// higher item, not at a numeric stop further out — here item 7 is skipped and
    /// ` 9.` lies past the price list of item 8.
    #[test]
    fn a_skipped_successor_does_not_run_the_value_into_the_prices() {
        let body = "1.  Awarding authority: Borough Council, Town Hall, UK-Anytown AN1 1AA.\n\
                    3.  Date of award: 1. 2. 1998.\n\
                    6.  Supplier(s): 1: Acme Cleaning Services Limited, Unit 4, Riverside Industrial \n\
                    Estate, Long Lane, UK-Anytown AN2 2BB, tel. (01234) 56 78 90, facsimile (01234) \n\
                    56 78 91.\n\
                    2: Beta Grounds Maintenance Limited, The Old Mill, Station Road, UK-Othertown \n\
                    OT3 3CC, tel. (01234) 11 22 33, facsimile (01234) 11 22 34.\n\
                    8.  Price(s): 1: STERLING 400 000; 2: STERLING 156 000.\n\
                    9.  Notice published on: 3. 4. 1998.";
        assert_eq!(
            awarded_names(body),
            vec!["Acme Cleaning Services Limited".to_owned(), "Beta Grounds Maintenance Limited".to_owned()]
        );
        assert_eq!(successor_marker(" a 8.  b", 6), Some(2), "7 absent: 8 is next");
        assert_eq!(successor_marker(" 9. x 7. y", 6), Some(5), "the real successor beats a later-number match");
        assert_eq!(lot_prefix_len("1) CGC"), Some(2));
        assert_eq!(lot_prefix_len("a) clause"), None, "a letter list is not a lot key");
        assert_eq!(lot_prefix_len("Furic (Jules et Alain) et Fils"), None);
    }

    /// Issue 485: a phone number is not a lot reference, and a contact line is not a
    /// name. Bodies are the stored V.3 values of 3009398 and 2808875, verbatim.
    #[test]
    fn a_phone_number_is_not_a_lot_reference() {
        let malaga = "SECTION I: CONTRACTING AUTHORITY\n\
                      I.1)  NAME, ADDRESSES AND CONTACT POINT(S): Autoridad Portuaria de Málaga, \n\
                      Muelle de Cánovas, s/n, Attn: Presidente, E-29001 Málaga. Tel. 952 12 50 \n\
                      00. E-mail: bgalvez@puertomalaga.com. Fax 952 12 50 02.\n\
                      SECTION V: AWARD OF CONTRACT\n\
                      CONTRACT NO: 50-30-26-E\n\
                      V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR TO WHOM THE CONTRACT HAS BEEN \n\
                      AWARDED: Autoridad Portuaria de Málaga, a la atención de Presidente, \n\
                      Muelle de Cánovas, s/n, E-29001 Málaga. E-mail: bgalvez@puertomalaga.com. \n\
                      Tel. 952 12 50 00. URL: www.puertomalaga.com. Fax 952 12 50 02.\n\
                      V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the contract:";
        // Before: [`Autoridad Portuaria de Málaga`, `URL: www.puertomalaga.com. Fax 952 12 50 02`].
        assert_eq!(awarded_names(malaga), vec!["Autoridad Portuaria de Málaga".to_owned()]);

        let cardiff = "SECTION V: AWARD OF CONTRACT\n\
                       CONTRACT NO: LOC957/AJ/TSS/06\n\
                       V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR TO WHOM THE CONTRACT HAS BEEN \n\
                       AWARDED: AWETU, Att: Suzanne Smith. 41a, Lower Cathedral Road, UK-Cardiff \n\
                       CF11 6LW. Tel. 0044 2920 394141. Fax 0044 2920 644615.\n\
                       V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the contract:\n\
                       CONTRACT NO: LOC957/AJ/TSS/06\n\
                       V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR TO WHOM THE CONTRACT HAS BEEN \n\
                       AWARDED: BAWSO, Att: Angelina Jones. 9,Cathedral Road, UK-Cardiff CF11 \n\
                       9HA. Tel. 0044 2920 644633. Fax 0044 2920 644588.\n\
                       V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the contract:";
        // Before: [`AWETU`, `Fax 0044 2920 644615`, `BAWSO`, `Fax 0044 2920 644588`].
        assert_eq!(awarded_names(cardiff), vec!["AWETU".to_owned(), "BAWSO".to_owned()]);

        assert_eq!(lot_prefix_len("952 12 50 00. URL: x"), None, "phone notation");
        assert_eq!(lot_prefix_len("0044 2920 394141. Fax"), None, "phone notation");
        assert!(lot_prefix_len("1, 2, 3 and 4: Dolmen").is_some(), "a real lot list");
        assert!(lot_prefix_len("1/2: Evans").is_some());
        assert!(!plausible_name("Fax 0044 2920 644615"));
        assert!(!plausible_name("URL: www.puertomalaga.com. Fax 952 12 50 02"));
        assert!(!plausible_name("Tel. (046) 25 14 27"));
        assert!(plausible_name("Tel Aviv Holdings"));
        assert!(plausible_name("Faxon Ltd"));
    }

    /// Issue 244 slice 9: a winner-silent award body still yields its RESULT — a bare
    /// `LotResult` with the date and the count, and NO organization (the issue-257
    /// rule: publisher silence is never read as "no result", and a non-name is never
    /// minted as a company). Bodies are the residue read's own specimens.
    #[test]
    fn a_winner_silent_award_body_yields_a_bare_result() {
        // Notice 17,438 (1993): date + count + `Supplier(s): Various.` — a real award
        // whose winner the era's word for "no single answer" withholds.
        let record = "1.0/000001\nND: 54814-1992\nTD: 7 - Contract awards\n\
                      TX: 2. (a)  Award procedure: Restricted.\n    \
                      3.  Date of award: 1. 12. 1992.\n    \
                      5.  Tenders received: 8.\n    \
                      6.  Supplier(s): Various.\n    \
                      7.  Goods supplied: IV and irrigation fluids.";
        let p = parse(record).expect("parses");
        let results: Vec<&str> =
            p.sections.iter().filter(|s| s.kind == "LotResult").map(|s| s.id.as_str()).collect();
        assert_eq!(results, vec!["RES-1"], "the award materialises even winner-less");
        assert!(
            !p.sections.iter().any(|s| s.kind == "Organization" && s.parent.as_deref() == Some("RES-1")),
            "no organization is invented for `Various`"
        );
        assert!(
            p.values.iter().any(|v| v.section_id == "RES-1"
                && v.field_id == "TED-CONTRACT_AWARD_DATE"
                && matches!(&v.value, NoticeValue::Date { .. })),
            "the award date lands on the minted result"
        );
        assert!(
            p.values.iter().any(|v| v.section_id == "RES-1"
                && v.field_id == "TED-NB_TENDERS_RECEIVED"
                && matches!(&v.value, NoticeValue::Integer(8))),
            "the tenders-received count lands in the statistics channel"
        );
        assert!(
            !p.values.iter().any(|v| v.field_id == "TED-NO_AWARDED_CONTRACT"),
            "a dated award with a silent winner is NOT closed-without-award"
        );

        // An explicit rejection is: the phrase wraps across the era's ~72-column lines
        // and still reads, and the result carries the closure marker.
        let rejected = "1.0/000001\nND: 1-2005\nTD: 7 - Contract awards\n\
                        TX: SECTION V: AWARD OF CONTRACT\n    \
                        V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR: All tenders \n    \
                        were rejected.";
        let p = parse(rejected).expect("parses");
        assert!(
            p.sections.iter().any(|s| s.kind == "LotResult"),
            "a rejected procedure is a result, not an absence"
        );
        assert!(
            p.values.iter().any(|v| v.field_id == "TED-NO_AWARDED_CONTRACT"),
            "the rejection reaches the projection's clos-nw mapping"
        );

        // The guards: a non-award body mints nothing however date-like its text, and a
        // dateless, phraseless award body still mints nothing (evidence, not type,
        // earns the skeleton).
        let non_award = "1.0/000001\nND: 2-1999\nTD: 3 - Invitation to tender\n\
                         TX: 3.  Date of award: 1. 12. 1992.";
        assert!(!parse(non_award).expect("parses").sections.iter().any(|s| s.kind == "LotResult"));
        let bare = "1.0/000001\nND: 3-1999\nTD: 7 - Contract awards\n\
                    TX: 7.  Goods supplied: Fuel.";
        assert!(!parse(bare).expect("parses").sections.iter().any(|s| s.kind == "LotResult"));
    }

    /// Issue 244 slice 9: the count is claimed once, refused on disagreement, and
    /// never copied onto a multi-result notice.
    #[test]
    fn tenders_received_is_one_agreed_fact_on_one_result() {
        assert_eq!(tenders_received("5.  Tenders received: 8."), Some(8));
        assert_eq!(tenders_received("VI.4)  Number of tenders received: 4."), Some(4));
        assert_eq!(tenders_received("Offers received: 12. Tenders received: 12."), Some(12));
        // Disagreement is refused, not resolved.
        assert_eq!(tenders_received("Tenders received: 8. Tenders received: 9."), None);
        // A digit run too long to be a count is a reference the label does not own.
        assert_eq!(tenders_received("Tenders received: 20040101."), None);
        // Two results, one notice-scoped count: attaching it to either would state
        // it twice, so it is attached to neither.
        let two = "1.0/000001\nND: 4-2005\nTD: 7 - Contract awards\n\
                   TX: V.3) TO WHOM THE CONTRACT HAS BEEN AWARDED: Acme Ltd, Wood Street. \n    \
                   CONTRACT NO 2: V.3) TO WHOM THE CONTRACT HAS BEEN AWARDED: Bolt GmbH, Ring 1. \n    \
                   VI.4) Number of tenders received: 4.";
        let p = parse(two).expect("parses");
        assert_eq!(p.sections.iter().filter(|s| s.kind == "LotResult").count(), 2);
        assert!(!p.values.iter().any(|v| v.field_id == "TED-NB_TENDERS_RECEIVED"));
    }

    /// Issue 244: the money the numbered form states, and the far longer list of shapes
    /// it states money in that this refuses. Every string is from a prod body.
    #[test]
    fn a_price_is_claimed_only_in_one_unambiguous_shape() {
        // Claimed: one number, one code, nothing else. Either order.
        assert_eq!(parse_money("2 143 000 EUR."), Some((214_300_000, "EUR".to_owned(), None)));
        assert_eq!(parse_money("5 301 802,22 FRF"), Some((530_180_222, "FRF".to_owned(), None)));
        assert_eq!(parse_money("EUR 1 131 079,99"), Some((113_107_999, "EUR".to_owned(), None)));
        assert_eq!(parse_money("562680 GBP"), Some((56_268_000, "GBP".to_owned(), None)));

        // Refused, and each for its own reason.
        assert_eq!(parse_money("562 680 GBP p.a."), None, "annual, not a total");
        assert_eq!(
            parse_money("5 301 802,22 FRF TTC"),
            Some((530_180_222, "FRF".to_owned(), Some("incl"))),
            "the marker is read, not refused (slice 5)"
        );
        assert_eq!(parse_money("15 564 000 ATS / 1 131 079,99 EUR"), None, "two currencies");
        assert_eq!(parse_money("Minimum/maximum: Lit 2 610/Lit 3 289"), None, "a range");
        assert_eq!(parse_money("Lit 1 000 000 000"), None, "`Lit` is not a currency code");
        assert_eq!(parse_money("2 143 000"), None, "no currency at all");
        assert_eq!(parse_money("EUR"), None, "no number at all");
        assert_eq!(parse_money("0 EUR"), None, "zero is not a price");
        assert_eq!(
            parse_money("Publication of this information would prejudice the interests"),
            None,
            "withheld"
        );
        // Sub-cent is ADR-0010's quarantine trigger, so it must never become an Amount.
        assert_eq!(parse_money("1 000,255 EUR"), None, "three decimals is sub-cent");
        // Slice 9 overturns this one: it used to be refused beside the sub-cent case, but
        // tenths ARE cents — 20 of them — and `fetch 200` writes them 114 times a package.
        assert_eq!(
            parse_money("1 000,2 EUR"),
            Some((100_020, "EUR".to_owned(), None)),
            "one decimal is TENTHS, which is exactly representable"
        );
        assert_eq!(
            parse_money("33 030 818,1 LTL"),
            Some((3_303_081_810, "LTL".to_owned(), None)),
            "notice 3871371"
        );
        assert_eq!(
            parse_money("176 713,2 RON"),
            Some((17_671_320, "RON".to_owned(), None)),
            "notice 3872503"
        );
        // And comma-as-thousands is still not a number this reads, which is what keeps
        // the tenths reading unambiguous.
        assert_eq!(parse_money("1,000 EUR"), None, "a three-digit fraction is not cents");
        assert_eq!(parse_money("1,000,000 EUR"), None, "comma thousands throughout");
        // Mis-grouped digits are not a number this reads.
        assert_eq!(parse_money("2 14 3000 EUR"), None, "groups are not thousands");
        assert_eq!(parse_money("1 000,00 2 000,00 EUR"), None, "two figures");
    }

    /// Issue 491: a bons-de-commande minimum and maximum printed side by side is a range,
    /// not one figure, even though its digits group correctly. Every string is from a prod body.
    #[test]
    fn a_minimum_and_maximum_run_together_are_refused_as_a_range() {
        // CAN 199890-2008 (Ville du Robert) and 128539-2006 (Corse-du-Sud): round, and Y = 4X.
        assert_eq!(parse_money("60 000 220 000 EUR."), None, "min 60 000, max 220 000");
        assert_eq!(parse_money("87 250 349 000 EUR"), None, "min 87 250, max 349 000 = 4x");
        assert_eq!(parse_money("62 709 250 836 EUR"), None, "non-round, but exactly 4x");
        assert_eq!(parse_money("25 000 100 000,00 EUR"), None, "a zero fraction is still the print");
        // CAN 26854-2007's V.4 restates its II.2.1 pair with the spaces gone.
        assert_eq!(parse_money("40000120000 EUR."), None, "the same pair, unspaced");
        // The same procedure's other award: already refused by the grouping rule.
        assert_eq!(parse_money("5 500 22 000. EUR"), None);
        // Ordinary figures of the same raw shape stay claimed: non-round lira and forint
        // (notices 925750 and 3442687), and round figures whose last six digits are zero.
        assert_eq!(
            parse_money("LIT 57 157 228 327").map(|m| m.0),
            Some(5_715_722_832_700),
            "non-round: Y/X = 3.99 but not the range print"
        );
        assert_eq!(parse_money("119 564 161 135 HUF").map(|m| m.0), Some(11_956_416_113_500));
        assert_eq!(parse_money("30 000 000 000 EUR").map(|m| m.0), Some(3_000_000_000_000), "Y = 0");
        assert_eq!(parse_money("2 500 400 000 EUR").map(|m| m.0), Some(250_040_000_000), "Y > 4X");
        assert_eq!(parse_money("3 000 012 000 EUR").map(|m| m.0), Some(300_001_200_000), "0-led Y");
        assert_eq!(parse_money("250 800 EUR").map(|m| m.0), Some(25_080_000), "two groups is one figure");
        assert_eq!(parse_money("60 000 220 000,50 EUR").map(|m| m.0), Some(6_000_022_000_050), "cents: one figure");
    }

    /// The value in place, read off the whole body — including the two ways a body
    /// states money that this must NOT resolve.
    #[test]
    fn the_notices_price_reaches_the_amount_at_notice_scope() {
        // notice 1,710,387 (2001-06), the external-aid form.
        let starcm = "Service contract award notice\n\
                      4.  Contract value: 2 143 000 EUR.\n\
                      5.  Date of award of the contract: 11.5.2001.\n\
                      6.  Number of tenders received: 6.";
        assert_eq!(awarded_value(starcm), Some((214_300_000, "EUR".to_owned(), None)));

        // …and the whole record, so the fact lands where the projection reads it.
        let record = |td: &str| {
            format!("1.0/000001\nND: 1-2001\nTD: {td}\nTX: {}\n", starcm.replace('\n', "\n    "))
        };
        let p = parse(&record("7 - Contract award")).expect("parses");
        let amount = p
            .values
            .iter()
            .find(|v| v.field_id == "TED-VAL_TOTAL")
            .expect("the price is claimed as TED-VAL_TOTAL");
        assert_eq!(amount.section_id, SECTION, "notice scope, not a result");
        assert_eq!(
            amount.value,
            NoticeValue::Amount { cents: 214_300_000, currency: "EUR".to_owned() }
        );

        // The SAME body on a notice that is not an award claims nothing: a notice with no
        // result must not carry a result_value. Measured on fetch 300, 23 of the 3,250
        // bodies stating a price label are TD:3/0/2 rather than TD:7.
        for td in ["3 - Invitation to tender", "2 - Corrigendum"] {
            let p = parse(&record(td)).expect("parses");
            assert!(
                !p.values.iter().any(|v| v.field_id == "TED-VAL_TOTAL"),
                "a non-award notice claimed a result value ({td})"
            );
        }
        // …and a notice with no TD at all is not assumed to be an award.
        let p = parse(&format!("1.0/000001\nND: 1-2001\nTX: {}\n", starcm.replace('\n', "\n    ")))
            .expect("parses");
        assert!(!p.values.iter().any(|v| v.field_id == "TED-VAL_TOTAL"));

        // notice 1,710,454: the price is withheld and the value item is a sentence.
        let withheld = "3.  Date of award: 30.3.2001.\n\
                        8.  Price: Publication of this information would prejudice the \n\
                        legitimate commercial interests of a particular undertaking.\n\
                        9.  Value of winning award(s): Publication of this information would \n\
                        prejudice the legitimate commercial interests of a particular undertaking.";
        assert_eq!(awarded_value(withheld), None);

        // notice 1,710,458: `8.  Price: 5 301 802,22 FRF TTC.` — slice 4 refused this on the
        // reasoning that a stated tax basis must not be mixed silently into a column that
        // records none. Slice 5 reverses it, because the measurement said so: refusing
        // every value that states its basis discards most of the era's money, and the
        // column already mixes bases corpus-wide (the form eras' VAT indicator is not
        // mapped either — issue 251). So the figure is claimed AND the basis captured.
        let ttc = "6.  Successful contractor(s): Groupement d'entreprises CGEV.\n\
                   8.  Price: 5 301 802,22 FRF TTC.";
        assert_eq!(awarded_value(ttc), Some((530_180_222, "FRF".to_owned(), Some("incl"))));

        // Two labels stating the SAME figure is one fact twice, and is read.
        let agreeing = "8.  Price: 1 000 000 EUR.\n 9.  Value of winning award(s): 1 000 000 EUR.";
        assert_eq!(awarded_value(agreeing), Some((100_000_000, "EUR".to_owned(), None)));

        // Two labels DISAGREEING is a notice this cannot read, so it claims nothing
        // rather than guessing which one the analyst wanted.
        let disagreeing = "8.  Price: 1 000 000 EUR.\n 9.  Value of winning award(s): 900 000 EUR.";
        assert_eq!(awarded_value(disagreeing), None);

        // A body with no money says so cheaply — the gate returns before flattening.
        assert_eq!(awarded_value("6.  Successful contractor(s): Mill Group, 3 Road."), None);

        // The item boundary must not mistake a date or a house number for an item.
        assert_eq!(next_item_marker(" 2 143 000 EUR. 5. Date"), Some(15));
        assert_eq!(next_item_marker(" 11.5.2001 is a date"), None);
        assert_eq!(next_item_marker(" Emilienstrasse 8, O-5900"), None);
        assert_eq!(next_item_marker(" 1 000 000 EUR. 10. Subcontract"), Some(15));
        // Both spellings of the plural, and the singular.
        for heading in ["Value of winning award(s):", "VALUE OF WINNING AWARD(S):", "Value of winning award:"] {
            assert_eq!(
                awarded_value(&format!("8.  {heading} 1 000 000 EUR.\n 10.  Subcontract: No.")),
                Some((100_000_000, "EUR".to_owned(), None)),
                "{heading}"
            );
        }

        // A price stated right before a two-digit item still parses.
        assert_eq!(
awarded_value("9.  Value of winning award(s): 1 000 000 EUR. 10.  Subcontract: No."),
            Some((100_000_000, "EUR".to_owned(), None))
        );
    }

    /// Issue 244 slice 5: the sub-label the era puts between the price heading and the
    /// figure, which was six of eight sampled refusals — and the one shape where skipping
    /// to the figure would be WRONG.
    #[test]
    fn a_sub_label_between_the_heading_and_the_figure_is_skipped_but_a_second_figure_is_not() {
        // The measured shape, both bases, verbatim from prod (notices 1,710,441-1,710,446).
        assert_eq!(
            read_value_item(" Auftragssumme (ohne Umsatzsteuer): 689 655,17 DEM."),
            Some((68_965_517, "DEM".to_owned(), Some("excl")))
        );
        assert_eq!(
            read_value_item(" Auftragssumme (mit Umsatzsteuer): 110 761,16 DEM."),
            Some((11_076_116, "DEM".to_owned(), Some("incl")))
        );
        // No separators, and a sub-label that states no basis at all.
        assert_eq!(
            read_value_item(" Auftragssumme: 1 944 255 DEM."),
            Some((194_425_500, "DEM".to_owned(), None))
        );

        // THE GUARD. Skipping to after the last colon here would claim the SUBCONTRACTED
        // figure as the contract price. A pure label carries no digits; a second figure
        // does, and that is the whole test of whether the skip is safe.
        assert_eq!(
            read_value_item(" 1 000 000 EUR, of which subcontracted: 200 000 EUR"),
            None,
            "a second figure must never be read as the price"
        );
        assert_eq!(
            read_value_item(" Total for lot 2: 200 000 EUR"),
            None,
            "a label carrying a digit is not a label this trusts"
        );

        // Still refused after the retry, because the figure itself does not qualify.
        assert_eq!(read_value_item(" Preis: 15 564 000 ATS / 1 131 079,99 EUR"), None);
        assert_eq!(read_value_item(" Price of product plus price of transport."), None);
        assert_eq!(read_value_item(" Minimum/maximum: Lit 2 610/Lit 3 289"), None);

        // The French/Belgian markers, which slice 4 read as a second currency code.
        assert_eq!(tax_marker("TTC"), Some("incl"));
        assert_eq!(tax_marker("TVAC"), Some("incl"));
        assert_eq!(tax_marker("HT"), Some("excl"));
        assert_eq!(tax_marker("HTVA"), Some("excl"));
        assert_eq!(tax_marker("EUR"), None);
        // The German pair as a bare trailing word, with no sub-label to skip past
        // (prod: `8.  Price: 8 600 000 DEM netto.`).
        assert_eq!(tax_marker("netto"), Some("excl"));
        assert_eq!(tax_marker("Brutto"), Some("incl"));
        assert_eq!(
            read_value_item(" 8 600 000 DEM netto."),
            Some((860_000_000, "DEM".to_owned(), Some("excl")))
        );
        assert_eq!(read_value_item(" 1 000 000 FRF HT"), Some((100_000_000, "FRF".to_owned(), Some("excl"))));
        // A body claiming both bases at once states neither.
        assert_eq!(read_value_item(" 1 000 000 FRF HT TTC"), None);
        // A label that literally states both bases states neither.
        assert_eq!(phrase_basis("netto (ohne Umsatzsteuer) und brutto (mit Umsatzsteuer)"), None);
        assert_eq!(phrase_basis("Auftragssumme (ohne Umsatzsteuer)"), Some("excl"));
        assert_eq!(phrase_basis("Auftragssumme"), None);

        // And the basis reaches the parse layer as its own code beside the amount.
        let body = "3.  Date of award: 30.3.2001.\n\
                    8.  Price: Auftragssumme (ohne Umsatzsteuer): 689 655,17 DEM.\n\
                    9.";
        let record = format!(
            "1.0/000001\nND: 1-2001\nTD: 7 - Contract award\nTX: {}\n",
            body.replace('\n', "\n    ")
        );
        let p = parse(&record).expect("parses");
        assert_eq!(
            p.values.iter().find(|v| v.field_id == "TED-VAL_TOTAL").map(|v| &v.value),
            Some(&NoticeValue::Amount { cents: 68_965_517, currency: "DEM".to_owned() })
        );
        assert_eq!(
            p.values.iter().find(|v| v.field_id == "TED-VAL_TOTAL_TAX_BASIS").map(|v| &v.value),
            Some(&NoticeValue::Code { list: None, code: "excl".to_owned() }),
            "the basis has no canonical home yet (issue 251), but it is captured"
        );
    }

    /// Issue 244 slice 7: the SECTIONED form, which is where the era's money actually is.
    ///
    /// Measured over `fetch 200` (2009-10, notices 3,870,856-3,903,775): of the package's
    /// 11,943 TD:7 award notices, 9,549 bodies state `Total final value` and only 53 state
    /// `Price:` — the label slices 4-6 read is the rare one in this vintage. The shape is
    /// also different in kind: the item does NOT end with its figure. It continues into
    /// prose that states the VAT basis, or runs straight into the next section heading or
    /// the next lot's contract number. Every body below is verbatim from prod, with the
    /// notice id in the comment above it.
    #[test]
    fn the_sectioned_forms_total_final_value_carries_its_vat_basis() {
        // 3870957 — the `V.4)` flavour: the label, the figure a line below it, the basis
        // a line below that.
        assert_eq!(
            awarded_value(
                "V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                 Total final value of the contract:\n\
                 Value: 791 805 EUR.\n\
                 Excluding VAT."
            ),
            Some((79_180_500, "EUR".to_owned(), Some("excl")))
        );

        // 3870958 — the `II.2.1)` flavour, where the label appears TWICE: once as the
        // section heading and once as the item. The heading occurrence is refused on its
        // own (see below) and the item occurrence a few words later is what lands, so the
        // duplication costs nothing. `VAT rate (%): 22,00 %.` is past the stop and never
        // reached — without the stop the sub-label retry would strip to after ITS colon
        // and lose the figure entirely.
        assert_eq!(
            awarded_value(
                "II.2)  TOTAL FINAL VALUE OF CONTRACT(S)\n\
                 II.2.1)  Total final value of contract(s): Value: 39 279 748,48 PLN.\n\
                 Including VAT. VAT rate (%): 22,00 %.\n\
                 SECTION V: AWARD OF CONTRACT\n\
                 CONTRACT NO: 1\n\
                 V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR"
            ),
            Some((3_927_974_848, "PLN".to_owned(), Some("incl")))
        );
        // Why the heading occurrence is harmless: its label carries the `II.2.1)` marker's
        // digits, so the retry refuses it. A refusal does not poison the scan — only a
        // second CLAIM that disagrees does.
        assert_eq!(
            read_value_item(
                " OF CONTRACT(S) II.2.1) Total final value of contract(s): Value: 39 279 748,48 PLN."
            ),
            None,
            "a label carrying the section marker's digits is not a label this trusts"
        );

        // 3870959 — THE GUARD, and it is a real body rather than a contrived one: the era
        // writes a range here, and this one has LOST its second figure at source
        // (`highest offer: PLN.`). Reading `16 184 142,63` as the contract value would
        // record the lowest offer as the price.
        assert_eq!(
            awarded_value(
                "II.2)  TOTAL FINAL VALUE OF CONTRACT(S)\n\
                 II.2.1)  Total final value of contract(s): Lowest offer: 16 184 142,63 /\n\
                 highest offer: PLN.\n\
                 Excluding VAT.\n\
                 SECTION V: AWARD OF CONTRACT"
            ),
            None,
            "a range must claim nothing, whether or not both ends survived"
        );

        // 3870962 — no VAT phrase at all: the value ends at the next SECTION heading and
        // the basis stays unstated. An unstated basis is NULL, not a guess.
        assert_eq!(
            awarded_value(
                "II.2)  TOTAL FINAL VALUE OF CONTRACT(S)\n\
                 II.2.1)  Total final value of contract(s): Value: 54 639 833,00 SEK.\n\
                 SECTION V: AWARD OF CONTRACT\n\
                 CONTRACT NO: 1"
            ),
            Some((5_463_983_300, "SEK".to_owned(), None))
        );

        // 3870965 — a long contract number after the basis, which the sub-label retry
        // would otherwise read as the figure's own label.
        assert_eq!(
            awarded_value(
                "II.2.1)  Total final value of contract(s): Value: 104 131,80 EUR.\n\
                 Excluding VAT.\n\
                 SECTION V: AWARD OF CONTRACT\n\
                 CONTRACT NO: 4300023446"
            ),
            Some((10_413_180, "EUR".to_owned(), Some("excl")))
        );

        // 3871306 and 3873797 — the aggregate at `II.2.1` sits BEFORE section IV, so the
        // heading that bounds its figure is `SECTION IV: PROCEDURE` (slice 9). Without a
        // stop there the figure runs on into `IV.1.1) Type of procedure: Open.` and the
        // sub-label retry strips to after THAT colon.
        assert_eq!(
            awarded_value(
                "II.2)  TOTAL FINAL VALUE OF CONTRACT(S)\n\
                 II.2.1)  Total final value of contract(s): Value: 7 015 000 GBP.\n\
                 SECTION IV: PROCEDURE\n\
                 IV.1)  TYPE OF PROCEDURE\n\
                 IV.1.1)  Type of procedure: Open."
            ),
            Some((701_500_000, "GBP".to_owned(), None)),
            "any section heading ends a value, not just section V"
        );

        // 3870961 — the same, without a VAT phrase between the figure and the contract
        // number: `CONTRACT NO` is what bounds the value, and without that stop this body
        // reads as `... CONTRACT NO: 2` and is refused for the digit in its label.
        assert_eq!(
            read_value_item(" of the contract: Value: 24 950 EUR. CONTRACT NO: 2"),
            Some((2_495_000, "EUR".to_owned(), None))
        );

        // The basis must come from THIS item's stop, never from a later lot's. The first
        // stop wins, so the `Excluding VAT` belonging to the second lot below does not
        // reach the first lot's figure.
        assert_eq!(
            read_value_item(
                " of the contract: Value: 100 000 EUR. CONTRACT NO: 2 V.4) Total final \
                 value: 200 000 EUR. Excluding VAT."
            ),
            Some((10_000_000, "EUR".to_owned(), None))
        );
        // And a marker beside the figure still outranks the stop phrase, because it is
        // the nearer statement.
        assert_eq!(
            read_value_item(" 1 000 000 FRF HT. Including VAT."),
            Some((100_000_000, "FRF".to_owned(), Some("excl")))
        );

        // Two lots stating DIFFERENT totals is a notice this cannot resolve, and the
        // sectioned form is where that actually happens.
        assert_eq!(
            awarded_value(
                "V.4)  Total final value of the contract:\n\
                 Value: 24 950 EUR.\n\
                 Excluding VAT.\n\
                 CONTRACT NO: 2\n\
                 V.4)  Total final value of the contract:\n\
                 Value: 30 000 EUR.\n\
                 Excluding VAT."
            ),
            None,
            "two labels disagreeing is a notice this cannot read"
        );

        // End to end, through a TD:7 record: the amount and its basis both reach the
        // parse layer.
        let body = "V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                    Total final value of the contract:\n\
                    Value: 791 805 EUR.\n\
                    Excluding VAT.";
        let record = format!(
            "1.0/000001\nND: 1-2009\nTD: 7 - Contract award\nTX: {}\n",
            body.replace('\n', "\n    ")
        );
        let p = parse(&record).expect("parses");
        assert_eq!(
            p.values.iter().find(|v| v.field_id == "TED-VAL_TOTAL").map(|v| &v.value),
            Some(&NoticeValue::Amount { cents: 79_180_500, currency: "EUR".to_owned() })
        );
        assert_eq!(
            p.values.iter().find(|v| v.field_id == "TED-VAL_TOTAL_TAX_BASIS").map(|v| &v.value),
            Some(&NoticeValue::Code { list: None, code: "excl".to_owned() })
        );
    }


    /// Issue 244 slice 8: the sectioned form states its total at TWO scopes, and treating
    /// the pair as a self-contradiction was discarding the one `TED-VAL_TOTAL` asks for.
    ///
    /// After slice 7, `fetch 200` still yielded no amount for 3,656 bodies that state
    /// `Total final value` — and 1,959 of them state the `II.2.1` aggregate WITH a figure.
    /// They were refused as disagreements. They are not disagreements.
    #[test]
    fn the_notice_scope_total_outranks_one_contracts_share() {
        // Verbatim from notice 3871014: the aggregate, then its four parts.
        let four_contracts = "II.2)  TOTAL FINAL VALUE OF CONTRACT(S)\n\
             II.2.1)  Total final value of contract(s): Value: 81 605 403,00 SEK.\n\
             Excluding VAT.\n\
             SECTION V: AWARD OF CONTRACT\n\
             CONTRACT NO: 1\n\
             V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the contract:\n\
             Value: 40 087 596,00 SEK.\n\
             Excluding VAT.\n\
             CONTRACT NO: 2\n\
             V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the contract:\n\
             Value: 15 605 000,00 SEK.\n\
             Excluding VAT.\n\
             CONTRACT NO: 3\n\
             V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the contract:\n\
             Value: 14 700 772,00 SEK.\n\
             Excluding VAT.\n\
             CONTRACT NO: 4\n\
             V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the contract:\n\
             Value: 11 212 035,00 SEK.\n\
             Excluding VAT.";
        // THE ARGUMENT, in cents: the four parts sum to the aggregate exactly. That is
        // what makes `II.2.1` a total and the `V.4` figures its parts, rather than five
        // readings of one fact that happen to disagree.
        assert_eq!(4_008_759_600i64 + 1_560_500_000 + 1_470_077_200 + 1_121_203_500, 8_160_540_300);
        assert_eq!(
            awarded_value(four_contracts),
            Some((8_160_540_300, "SEK".to_owned(), Some("excl"))),
            "the notice's own total, not the first contract's share"
        );

        // Verbatim from notice 3871013, where the two scopes genuinely differ by more than
        // grouping — 116.25M against 116.0M — and the notice-scope figure still wins.
        assert_eq!(
            awarded_value(
                "II.2)  TOTAL FINAL VALUE OF CONTRACT(S)\n\
                 II.2.1)  Total final value of contract(s): Value: 116 250 000,00 SEK.\n\
                 Excluding VAT.\n\
                 SECTION V: AWARD OF CONTRACT\n\
                 CONTRACT NO: 1\n\
                 V.4)  INFORMATION ON VALUE OF CONTRACT Initial estimated total value of \
                 the contract:\n\
                 Value: 110 000 000,00 SEK.\n\
                 Excluding VAT.\n\
                 Total final value of the contract:\n\
                 Value: 116 000 000,00 SEK.\n\
                 Excluding VAT."
            ),
            Some((11_625_000_000, "SEK".to_owned(), Some("excl")))
        );

        // THE GUARD THIS SLICE OWES. A body awarding several contracts and stating no
        // aggregate must claim NOTHING, even when its per-contract figures agree — two
        // contracts of 40 087 596 make a notice total of twice that, and recording one of
        // them as `TED-VAL_TOTAL` would understate it by half. Before this slice the
        // agreeing pair read as one fact stated twice, and was claimed.
        assert_eq!(
            awarded_value(
                "SECTION V: AWARD OF CONTRACT\n\
                 CONTRACT NO: 1\n\
                 V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the contract:\n\
                 Value: 40 087 596,00 SEK.\n\
                 Excluding VAT.\n\
                 CONTRACT NO: 2\n\
                 V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the contract:\n\
                 Value: 40 087 596,00 SEK.\n\
                 Excluding VAT."
            ),
            None,
            "one contract's share is not the notice's total when there are two of them"
        );

        // And a single-contract body still claims its own figure: the guard counts
        // contracts, it does not distrust the `V.4` scope.
        assert_eq!(
            awarded_value(
                "SECTION V: AWARD OF CONTRACT\n\
                 CONTRACT NO: 1\n\
                 V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the contract:\n\
                 Value: 40 087 596,00 SEK.\n\
                 Excluding VAT."
            ),
            Some((4_008_759_600, "SEK".to_owned(), Some("excl")))
        );

        // Two AGGREGATES disagreeing is still unreadable — the scope rule ranks scopes,
        // it does not stop a notice from contradicting itself within one.
        assert_eq!(
            awarded_value(
                "II.2.1)  Total final value of contract(s): Value: 81 605 403,00 SEK.\n\
                 Excluding VAT.\n\
                 II.2.1)  Total final value of contract(s): Value: 70 000 000,00 SEK.\n\
                 Excluding VAT."
            ),
            None,
            "a conflict at the widest scope is a refusal, not a reason to look narrower"
        );

        // The marker is a HEADING, counted at line starts only. The pre-2004 numbered form
        // writes the winner's contract REFERENCE inside item 6, and a bare substring count
        // reads that as a second contract — 7 of `fetch 300`'s 996 amounts would have been
        // dropped, all of them correct.
        assert_eq!(count_ascii_ci(four_contracts, CONTRACT_MARKER), 4);
        assert_eq!(
            count_ascii_ci(
                "6.  Successful contractor(s): Contract No 710-7009: AS Anlegg, Arvid\n\
                 8.  Price: 15 887 897 NOK.",
                CONTRACT_MARKER
            ),
            0,
            "a contract reference inside an item is not a contract heading"
        );
        // Modelled on notice 1710588, one of those 7: two mentions, one contract, and a
        // price that must still be claimed. Item 9's `Approximately 16 000 000 NOK` is
        // refused by [`parse_money`] as it always was, so the price stands alone.
        assert_eq!(
            awarded_value(
                "3.  Date of award: 24.4.2001.\n\
                 6.  Successful contractor(s): Contract No 710-7009: AS Anlegg, Arvid\n\
                 8.  Price: 15 887 897 NOK.\n\
                 9.  Value of winning award(s): Approximately 16 000 000 NOK. Contract No \
                 710-7009."
            ),
            Some((1_588_789_700, "NOK".to_owned(), None))
        );
    }

    /// Issue 255 slice 3 / issue 244: the era's award DATE, read off the same prose the
    /// price and the winners come from — and landing on `TED-CONTRACT_AWARD_DATE`, the
    /// legacy form eras' own field id, so slice 2's projection carries it to
    /// `tender_version_lot_results.decided_*` with no mapping change.
    #[test]
    fn the_award_date_is_read_from_the_body_and_lands_on_the_result() {
        // The numbered form, verbatim from prod (notices 1,710,441-1,710,588): item 3 in
        // the works/supplies forms, item 5 in the external-aid one.
        assert_eq!(award_date("3.  Date of award: 30.3.2001."), read_dmy(" 30.3.2001"));
        assert_eq!(
            award_date("5.  Date of award of the contract: 11.5.2001."),
            read_dmy("11.5.2001"),
            "the longer label is matched by the shorter one it starts with"
        );
        // The sectioned form's heading, upper-case and with its own item marker.
        assert!(award_date("V.1)  DATE OF CONTRACT AWARD DECISION: 14.12.2018").is_some());

        // The separators carry optional spaces in this era (`2. 11. 1999` appears in the
        // 1999 deadline lines), and whatever follows the year is the next item.
        assert_eq!(read_dmy(" 2. 11. 1999"), read_dmy("2.11.1999"));
        assert_eq!(read_dmy(" 30.3.2001. 4.  Award criteria: price."), read_dmy("30.3.2001"));

        // Refusals. A two-digit year is ambiguous across an era spanning 1993-2010; a
        // missing separator is not a date; an impossible date is not a date.
        assert_eq!(read_dmy(" 30.3.01"), None, "a two-digit year is not read");
        assert_eq!(read_dmy(" 3032001"), None);
        // The shared parser NORMALISES out-of-range parts — `30.13.2001` comes back as
        // 2002-01-30 and `31.2.2001` as 2001-03-03 — so the range check happens here,
        // before it. A rolled-over typo would be a wrong fact, not a missing one.
        assert_eq!(read_dmy(" 30.13.2001"), None, "month 13 is not a month");
        assert_eq!(read_dmy(" 31.2.2001"), None, "February has no 31st");
        assert_eq!(read_dmy(" 0.3.2001"), None, "there is no zeroth day");
        assert!(read_dmy(" 29.2.2000").is_some(), "2000 was a leap year");
        assert_eq!(read_dmy(" 29.2.1999"), None, "1999 was not");
        assert_eq!(award_date("3.  Date of award: to be announced."), None);
        assert_eq!(award_date("6.  Successful contractor(s): ACME Ltd."), None, "no label");

        // Two award dates that disagree is a notice this cannot read — the same rule the
        // price follows, for the same reason.
        assert_eq!(
            award_date("V.1)  Date of award: 30.3.2001. V.1)  Date of award: 2.4.2001."),
            None
        );
        // …and the same date stated twice is one fact stated twice.
        assert!(
            award_date("V.1)  Date of award: 30.3.2001. V.1)  Date of award: 30.3.2001.").is_some()
        );

        // End to end: a TD:7 record with a winner puts the date on the result block, under
        // the field id the legacy eras use.
        let body = "3.  Date of award: 30.3.2001.\n\
                    6.  Successful contractor(s): Gagneraud Construction, F-33000 Bordeaux.\n\
                    8.  Price: 1 000 000 EUR.";
        let record = format!(
            "1.0/000001\nND: 1-2001\nTD: 7 - Contract award\nTX: {}\n",
            body.replace('\n', "\n    ")
        );
        let p = parse(&record).expect("parses");
        let result = p.sections.iter().find(|s| s.kind == "LotResult").expect("a result block");
        let date = p
            .values
            .iter()
            .find(|v| v.field_id == "TED-CONTRACT_AWARD_DATE")
            .expect("the award date is claimed");
        assert_eq!(date.section_id, result.id, "on the result, not on the root");
        assert!(matches!(date.value, NoticeValue::Date { has_time: false, .. }));

        // A dated body with no winner used to mint NOTHING — the date had nowhere to
        // land. Slice 9 inverted that deliberately: the date is itself the award
        // evidence, so a bare result is minted and the date lands on it (the
        // winner-silence rule; see `a_winner_silent_award_body_yields_a_bare_result`
        // for the full shape).
        let no_winner = format!(
            "1.0/000001\nND: 2-2001\nTD: 7 - Contract award\nTX: {}\n",
            "3.  Date of award: 30.3.2001.".replace('\n', "\n    ")
        );
        let p = parse(&no_winner).expect("parses");
        let bare = p.sections.iter().find(|s| s.kind == "LotResult").expect("slice 9 mints the result");
        assert!(
            p.values.iter().any(|v| v.field_id == "TED-CONTRACT_AWARD_DATE" && v.section_id == bare.id),
            "the date lands on the minted result"
        );
    }

    /// A withheld winner must mint NOTHING. The era fills the item with boilerplate
    /// rather than leaving it blank, and that sentence reaches a comma well inside the
    /// window — so the runaway-value fall-through does not catch it and the reject list
    /// is what stands between it and one provisional organization per withholding
    /// notice (issue 234).
    #[test]
    fn a_withheld_winner_mints_no_organization() {
        let body = "3.  Date of award: 30.3.2001.
                    6.  Successful contractor(s): Publication of this information would 
                    prejudice the legitimate commercial interests of a particular undertaking.
                    7.  Works provided: CPV: 45210000, 74222000, 74873100.";
        assert!(
            awarded_names(body).is_empty(),
            "withheld boilerplate became a name: {:?}",
            awarded_names(body)
        );
        // …and the whole record, so nothing downstream sees a phantom result either.
        let record = format!("1.0/000001\nND: 1-2001\nTX: {}\n", body.replace('\n', "\n    "));
        let p = parse(&record).expect("parses");
        assert!(p.sections.iter().all(|s| s.kind != "LotResult"));

        // `Not applicable.` is the era's other non-answer.
        assert!(awarded_names("Award notice 6. Successful contractor(s): Not applicable, none.").is_empty());
    }

    /// Issue 397: the authenticity boilerplate leaves the title, the substantive
    /// atoms stay, and anything the vocabulary does not know is left alone.
    ///
    /// Every input here is a real trailing block from the corpus (tender ids
    /// 7,960,000–8,059,999), with its measured row count, so this is a test
    /// against what TED published rather than against what a rule would like.
    #[test]
    fn the_authenticity_note_leaves_the_title_and_the_rest_stays() {
        let case = |t: &str| strip_authenticity_note(t);

        // The boilerplate alone: the whole block goes, parentheses included (5,742).
        assert_eq!(
            case("D-Herzogenrath: sewage-treatment plant (Only the original text is authentic)"),
            "D-Herzogenrath: sewage-treatment plant"
        );
        // Mixed with a substantive atom: the block is rebuilt without the note (1,312).
        assert_eq!(
            case("F-Lyons: batteries (Supply contract - Only the original text is authentic)"),
            "F-Lyons: batteries (Supply contract)"
        );
        // Three atoms, one dropped, order preserved (397).
        assert_eq!(
            case("X: y (Supply contract - Only the original text is authentic - Open to US bidders)"),
            "X: y (Supply contract - Open to US bidders)"
        );
        // Case varies in the source, both spellings measured (686 vs 269).
        assert_eq!(
            case("X: y (With participation by GATT countries - Only the original text is authentic)"),
            "X: y (With participation by GATT countries)"
        );
        assert_eq!(
            case("X: y (with participation by GATT countries)"),
            "X: y (with participation by GATT countries)"
        );
        // A block with nothing to drop is returned unchanged (883).
        assert_eq!(case("X: y (Supply contract)"), "X: y (Supply contract)");

        // THE POINT: an unrecognised atom means this is not an annotation block,
        // it is title text that happens to be parenthesised. Both of these are
        // real titles — one row each in the measured range — and a shape-based
        // rule would have eaten them.
        assert_eq!(case("X: personal computers (PCs)"), "X: personal computers (PCs)");
        assert_eq!(case("X: survey (GeophysB 2026)"), "X: survey (GeophysB 2026)");
        // Including when the note is mixed WITH an unknown atom: refuse the whole
        // block rather than guess which half is content.
        assert_eq!(
            case("X: y (PCs - Only the original text is authentic)"),
            "X: y (PCs - Only the original text is authentic)"
        );

        // Shapes that are not a trailing block at all.
        assert_eq!(case("D-Naumburg: general construction work (new work and renovation)"),
                   "D-Naumburg: general construction work (new work and renovation)");
        assert_eq!(case("X: y"), "X: y");
        assert_eq!(case("X: y (unclosed"), "X: y (unclosed");
        assert_eq!(case(""), "");
        // A title that is ONLY the note collapses to empty rather than to "()".
        assert_eq!(case("(Only the original text is authentic)"), "");
    }

    /// Issue 397 unit 2, step 2: the nature atoms leave the title when — and only
    /// when — the record carries the `NC` code they duplicate. The regime flags
    /// (`Open to US bidders`, GATT) stay either way, and a block the vocabulary
    /// does not know is untouched as before.
    #[test]
    fn the_nature_atoms_leave_the_title_only_when_the_record_carries_the_code() {
        // The rule alone.
        assert_eq!(strip_annotations("F-Lyons: batteries (Supply contract)", NATURE_ATOMS), "F-Lyons: batteries");
        assert_eq!(
            strip_annotations("X: y (Supply contract - Open to US bidders)", NATURE_ATOMS),
            "X: y (Open to US bidders)"
        );
        assert_eq!(
            strip_annotations("X: y (works contract - With participation by GATT countries)", NATURE_ATOMS),
            "X: y (With participation by GATT countries)"
        );
        assert_eq!(strip_annotations("X: personal computers (PCs)", NATURE_ATOMS), "X: personal computers (PCs)");

        // The record: TI precedes NC in the era's order, so the drop is a post-pass.
        let title_of = |p: &Parsed| {
            p.values
                .iter()
                .find_map(|v| match (&v.field_id, &v.value) {
                    (f, NoticeValue::Text { value, .. }) if f == "TXT-TI" => Some(value.clone()),
                    _ => None,
                })
                .expect("a title")
        };
        let with_code = "1.0/000001\nND: 1-2001\nTI: F-Lyons: batteries (Supply contract - Only the original text is authentic)\nNC: 2 - Supply contract\nTX: Some prose.\n";
        let p = parse(with_code).expect("parses");
        assert_eq!(title_of(&p), "F-Lyons: batteries", "note and nature atom both gone; the code carries the nature");
        assert!(
            p.values.iter().any(|v| v.field_id == "TXT-NC" && matches!(&v.value, NoticeValue::Code { code, .. } if code == "2")),
            "the NC code is on the record"
        );
        let without_code = "1.0/000001\nND: 1-2001\nTI: F-Lyons: batteries (Supply contract - Only the original text is authentic)\nTX: Some prose.\n";
        let q = parse(without_code).expect("parses");
        assert_eq!(title_of(&q), "F-Lyons: batteries (Supply contract)", "no code on the record: the atom is the only copy and stays");
        let flags = "1.0/000001\nND: 1-2001\nTI: X: y (Supply contract - Open to US bidders)\nNC: 2 - Supply contract\nTX: Some prose.\n";
        let r = parse(flags).expect("parses");
        assert_eq!(title_of(&r), "X: y (Open to US bidders)", "the regime flag stays");
    }

    /// Issue 397: a wrapped heading is space-joined, and the `flatten` it goes
    /// through is the same one the parser already used for `TX`-derived facts —
    /// so a continuation that itself wrapped (`Open to US\nbidders`, 15 rows)
    /// rejoins before the vocabulary is consulted.
    #[test]
    fn a_wrapped_heading_rejoins_before_its_annotation_is_read() {
        let joined = flatten(&["X: y (Supply contract - Open to US", "bidders)"].join(" "));
        assert_eq!(joined, "X: y (Supply contract - Open to US bidders)");
        assert_eq!(strip_authenticity_note(&joined), "X: y (Supply contract - Open to US bidders)");

        let wrapped = flatten(
            &["NO-Tromso: architectural, engineering, construction and related technical", "consultancy services"]
                .join(" "),
        );
        assert_eq!(
            wrapped,
            "NO-Tromso: architectural, engineering, construction and related technical consultancy services"
        );
        assert!(!wrapped.contains('\n'));
    }

    /// Issue 508: the standard forms published from 2009-12-02 on name the winner under `IN FAVOUR
    /// OF WHOM A CONTRACT AWARD DECISION HAS BEEN TAKEN:`. Notice 3,990,107, verbatim
    /// (section V), served with its value and without its winner until this.
    #[test]
    fn the_2009_heading_names_its_winner_after_its_colon() {
        let body = "II.2)  TOTAL FINAL VALUE OF CONTRACT(S)\n\
                    II.2.1)  Total final value of contract(s): Value: 59 752 EUR.\n\
                    Excluding VAT.\n\
                    SECTION V: AWARD OF CONTRACT\n\
                    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT \n\
                    AWARD DECISION HAS BEEN TAKEN: Mike Priwitzer, Friedensstr. 39, 17179 \n\
                    Gnoien, DEUTSCHLAND.\n\
                    V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the contract:\n\
                    Value: 59 752 EUR.\n\
                    Excluding VAT.";
        assert_eq!(awarded_names(body), vec!["Mike Priwitzer".to_owned()]);
        assert_eq!(awarded_value(body), Some((5_975_200, "EUR".to_owned(), Some("excl"))));
    }

    /// Issue 508: from the 2010-03 package the same heading ends its line with no colon, the name is
    /// the NEXT line and the address the lines after it, and the totals read `Value 80 515
    /// EUR` with no colon either. Notice 4,200,000 (Tender 8255697), verbatim: before this
    /// it served neither the winner nor the value.
    #[test]
    fn the_2010_print_names_its_winner_on_the_next_line_and_states_its_total_without_colons() {
        let body = "II.2)  TOTAL FINAL VALUE OF CONTRACT(S)\n\
                    II.2.1)  Total final value of contract(s)\n\
                    Value 80 515 EUR\n\
                    Including VAT. VAT rate (%) 21\n\
                    SECTION IV: PROCEDURE\n\
                    IV.1)  TYPE OF PROCEDURE\n\
                    IV.1.1)  Type of procedure\n\
                    Open\n\
                    SECTION V: AWARD OF CONTRACT\n\
                    CONTRACT NO: AS/0030/10\n\
                    LOT NO:  - TITLE Tractors for OPW.\n\
                    V.1)  Date of contract award decision:\n\
                    3.8.2010\n\
                    V.2)  NUMBER OF OFFERS RECEIVED:\n\
                    4\n\
                    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT\n\
                    AWARD DECISION HAS BEEN TAKEN\n\
                    Clarke Machinery Ltd.\n\
                    New Inn, Ballyjamesduff, Co. Cavan\n\
                    IRELAND\n\
                    V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                    Total final value of the contract\n\
                    Value 80 515 EUR\n\
                    Including VAT. VAT rate (%) 21\n\
                    SECTION VI: COMPLEMENTARY INFORMATION";
        // The name only: the flattened body would have read `Clarke Machinery Ltd. New Inn`.
        assert_eq!(awarded_names(body), vec!["Clarke Machinery Ltd".to_owned()]);
        assert_eq!(awarded_value(body), Some((8_051_500, "EUR".to_owned(), Some("incl"))));

        // …and the whole record: one result, its winner, the value at notice scope.
        let record = format!(
            "1.0/000001\nND: 239051-2010\nTD: 7 - Contract award\nTX: {}\n",
            body.replace('\n', "\n    ")
        );
        let p = parse(&record).expect("parses");
        let org = p.sections.iter().find(|s| s.kind == "Organization").expect("a winner");
        assert_eq!(org.parent.as_deref(), Some("RES-1"));
        assert!(p.values.iter().any(|v| v.section_id == org.id
            && v.field_id == "TED-OFFICIALNAME"
            && matches!(&v.value, NoticeValue::Text { value, .. } if value == "Clarke Machinery Ltd")));
        let amount = p.values.iter().find(|v| v.field_id == "TED-VAL_TOTAL").expect("the total");
        assert_eq!(amount.section_id, SECTION);
        assert_eq!(amount.value, NoticeValue::Amount { cents: 8_051_500, currency: "EUR".to_owned() });
    }

    /// Issue 508: a 2010 body awarding five contracts (notice 4,101,747, the first two and
    /// the aggregate verbatim) names every winner and claims the aggregate, never one
    /// contract's figure.
    #[test]
    fn a_2010_multi_contract_body_names_every_winner_and_claims_its_aggregate() {
        let body = "II.2)  TOTAL FINAL VALUE OF CONTRACT(S)\n\
                    II.2.1)  Total final value of contract(s)\n\
                    Value 1 139 185,00 PLN\n\
                    Excluding VAT\n\
                    SECTION V: AWARD OF CONTRACT\n\
                    CONTRACT NO: 1\n\
                    LOT NO: 1\n\
                    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT\n\
                    AWARD DECISION HAS BEEN TAKEN\n\
                    Stryker Polska Sp. z o.o.\n\
                    ul. Łopuszańska 38B\n\
                    02-232 Warszawa\n\
                    POLAND\n\
                    V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                    Total final value of the contract\n\
                    Value 359 050,00 PLN\n\
                    Excluding VAT\n\
                    CONTRACT NO: 2\n\
                    LOT NO: 2\n\
                    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT\n\
                    AWARD DECISION HAS BEEN TAKEN\n\
                    Biomet Polska Sp. z o.o.\n\
                    ul. Płowiecka 75\n\
                    04-501 Warszawa\n\
                    POLAND\n\
                    V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                    Total final value of the contract\n\
                    Value 28 200,00 PLN\n\
                    Excluding VAT";
        assert_eq!(
            awarded_names(body),
            vec!["Stryker Polska Sp. z o.o.".to_owned(), "Biomet Polska Sp. z o.o.".to_owned()]
        );
        assert_eq!(awarded_value(body), Some((113_918_500, "PLN".to_owned(), Some("excl"))));
    }

    /// Issue 508: a 2010 body heading its awards `LOT NO: n` with no `CONTRACT NO` (notice
    /// 4,040,506, sixteen lots; the first and last two verbatim) and no aggregate. Each lot
    /// total but the last runs into the next `LOT NO:` line and refuses, so the last one
    /// alone parsed, and it was claimed as the notice's total: EUR 147 795,73 for sixteen
    /// lots. One lot's figure is a part, so nothing is claimed.
    #[test]
    fn a_2010_body_awarding_lots_without_contract_numbers_claims_no_part_as_its_total() {
        let body = "SECTION V: AWARD OF CONTRACT\n\
                    LOT NO: 01\n\
                    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT AWARD DECISION HAS BEEN TAKEN\n\
                    Entreprise ITE\n\
                    ZAC des Cettons, rue Panhard et Levassor\n\
                    78570 Chanteloup Les Vignes\n\
                    FRANCE\n\
                    V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                    Total final value of the contract\n\
                    Value 1 912 138 EUR\n\
                    LOT NO: 15\n\
                    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT AWARD DECISION HAS BEEN TAKEN\n\
                    Entreprise SRBG\n\
                    Cité du Grand Cormier, BP 20878\n\
                    78108 Saint Germain en Laye Cedex\n\
                    FRANCE\n\
                    V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                    Total final value of the contract\n\
                    Value 607 822,70 EUR\n\
                    LOT NO: 16\n\
                    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT AWARD DECISION HAS BEEN TAKEN\n\
                    GC Cuisines (Groupe LANEF PRO)\n\
                    9 village d'Entreprises, avenue Mauldre\n\
                    78680 Epone\n\
                    FRANCE\n\
                    V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                    Total final value of the contract\n\
                    Value 147 795,73 EUR";
        assert_eq!(
            awarded_names(body),
            vec![
                "Entreprise ITE".to_owned(),
                "Entreprise SRBG".to_owned(),
                "GC Cuisines (Groupe LANEF PRO)".to_owned()
            ]
        );
        assert_eq!(awarded_value(body), None);
        // The numbered form's own `Lot No` lines are not award headings: without a
        // `SECTION V`, a body's one price stays claimed whatever lot lines it prints.
        let numbered = "6.  Supplier(s):\n\
                        Lot No 1: Acme Ltd.\n\
                        Lot No 2: Acme Ltd.\n\
                        8.  Price: 1 000 000 EUR.\n\
                        9.  Other information: None.";
        assert_eq!(awarded_value(numbered), Some((100_000_000, "EUR".to_owned(), None)));
    }

    /// Issue 508: what the 2010 name line and colonless total refuse.
    #[test]
    fn the_2010_print_refuses_what_its_colon_twin_refuses() {
        // A comma in the name line ends the name, as it does after a colon (notice 4,040,925),
        // and an initial estimate is not a final value.
        let comma = "V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT AWARD DECISION HAS BEEN TAKEN\n\
                     Ghenova Civil, S.L.\n\
                     V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                     Initial estimated total value of the contract \n\
                     Value 324 078,71 EUR\n\
                     Excluding VAT";
        assert_eq!(awarded_names(comma), vec!["Ghenova Civil".to_owned()]);
        assert_eq!(awarded_value(comma), None);
        // An empty slot names nobody: the next line opens an item.
        let empty = "V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT\n\
                     AWARD DECISION HAS BEEN TAKEN\n\
                     V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                     Total final value of the contract\n\
                     Value 0 EUR";
        assert!(awarded_names(empty).is_empty());
        assert_eq!(awarded_value(empty), None, "a zero is no figure (notice 4,101,508)");
        // A range is not a total, with or without colons.
        let range = "V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                     Total final value of the contract\n\
                     Lowest offer 1 200 000 and highest offer 1 900 000 EUR\n\
                     Excluding VAT";
        assert_eq!(awarded_value(range), None);
        // Only the exact shape: scope words, `Value`, a figure. Other words before the
        // figure are not read past, and no other label retries without a colon.
        let loose = "Total final value of the contract estimated at Value 1 000 EUR";
        assert_eq!(awarded_value(loose), None);
        let price = "8.  Price: Estimated value 2 000 000 ECU.\n 9.  Other information: None.";
        assert_eq!(awarded_value(price), None);
    }

    /// Issue 508 review: a publisher who leaves the slot blank still gets the address
    /// block's country printed (notice 4,201,806, verbatim). As a name it would fold every
    /// such award of the era into one `GERMANY` organization.
    #[test]
    fn a_2010_slot_that_prints_only_a_country_names_nobody() {
        let body = "SECTION V: AWARD OF CONTRACT\n\
                    CONTRACT NO: 1\n\
                    LOT NO: 1\n\
                    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT\n\
                    AWARD DECISION HAS BEEN TAKEN\n\
                    GERMANY\n\
                    V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                    Total final value of the contract\n\
                    Value 12 060,50 EUR\n\
                    Including VAT. VAT rate (%) 19,00";
        assert!(awarded_names(body).is_empty());
        assert!(is_country("United Kingdom.") && !is_country("Clarke Machinery Ltd."));
    }

    /// Issue 508 review: a name the ~72-column wrapper broke is read whole (notice
    /// 4,101,615, verbatim: cut at the wrap it was `…Obrabiarek Sp`), and a name that fills
    /// the line before its street is not joined to the street (notice 4,101,904).
    #[test]
    fn a_2010_name_the_wrapper_broke_is_joined_and_its_street_is_not() {
        let wrapped = "V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT\n\
                       AWARD DECISION HAS BEEN TAKEN\n\
                       Konsorcjum: MDT Sp. z o.o. i Centrum Badawczo-Konstrukcyjne Obrabiarek Sp.\n\
                       z o.o.\n\
                       ul. Barcelońska 3/95\n\
                       02-762 Warszawa\n\
                       POLAND\n\
                       V.4)  INFORMATION ON VALUE OF CONTRACT";
        assert_eq!(
            awarded_names(wrapped),
            vec!["Konsorcjum: MDT Sp. z o.o. i Centrum Badawczo-Konstrukcyjne Obrabiarek Sp. z o.o.".to_owned()]
        );
        let street = "V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT\n\
                      AWARD DECISION HAS BEEN TAKEN\n\
                      Κοινωφελής Δημοτική Επιχείρηση Αλληλεγγύης Δήμου Ελευθερίου Κορδελιού\n\
                      Εθνικής Αντιστάσεως 58\n\
                      GREECE\n\
                      V.4)  INFORMATION ON VALUE OF CONTRACT";
        assert_eq!(
            awarded_names(street),
            vec!["Κοινωφελής Δημοτική Επιχείρηση Αλληλεγγύης Δήμου Ελευθερίου Κορδελιού".to_owned()]
        );
        // A word that would have fitted on the name's line was put on the next one by the
        // publisher, not the wrapper: no join.
        let fits = "V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT\n\
                    AWARD DECISION HAS BEEN TAKEN\n\
                    Sadec\n\
                    Rue du Bas Perreux\n\
                    V.4)  INFORMATION ON VALUE OF CONTRACT";
        assert_eq!(awarded_names(fits), vec!["Sadec".to_owned()]);
    }

    /// Issue 508 review: the 2010 print puts IV.3.2's `Contract notice` on a line of its
    /// own, which is no contract heading (notice 4,041,078, verbatim): counted as one, it
    /// dropped the single contract's only figure.
    #[test]
    fn a_contract_notice_line_is_no_contract_heading() {
        let body = "IV.3.2)  Previous publication(s) concerning the same contract\n\
                    Contract notice\n\
                    Notice number in the OJEU: 2009/s186268028 of 21.9.2009\n\
                    SECTION V: AWARD OF CONTRACT\n\
                    CONTRACT NO: AFRS 2009-03\n\
                    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT AWARD DECISION HAS BEEN TAKEN\n\
                    Bailey Maintenance Ltd\n\
                    Thames Valley House 14-16 Theale Lakes Business Park Moulden Way Sulhampstead\n\
                    RG7 4GB Reading\n\
                    UNITED KINGDOM\n\
                    V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                    Total final value of the contract\n\
                    Value 149 677,14 GBP\n\
                    Excluding VAT\n\
                    If annual or monthly value number of months 36";
        assert_eq!(awarded_names(body), vec!["Bailey Maintenance Ltd".to_owned()]);
        assert_eq!(awarded_value(body), Some((14_967_714, "GBP".to_owned(), Some("excl"))));
        assert_eq!(count_heading(body, CONTRACT_MARKER), 1);
        assert_eq!(count_heading("\nLot not awarded\nLOT NO: 2\nLOT NO:  - TITLE x", LOT_MARKER), 2);
    }

    /// Issue 508 review: a body of several award blocks with no `CONTRACT NO` or `LOT NO`
    /// heading at all (notice 4,300,188, verbatim): only the last figure parses, and one
    /// award of three is no notice total.
    #[test]
    fn several_award_blocks_without_headings_claim_no_part_as_the_total() {
        let block = |value: &str| {
            format!(
                "V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT\n\
                 AWARD DECISION HAS BEEN TAKEN\n\
                 SAS Nouvelle Technique du Bâtiment\n\
                 446 voie Georges Pompidou\n\
                 83300 Draguignan\n\
                 FRANCE\n\
                 V.4)  INFORMATION ON VALUE OF CONTRACT\n\
                 Total final value of the contract\n\
                 Value {value} EUR"
            )
        };
        let body = format!(
            "SECTION V: AWARD OF CONTRACT\n{}\n{}\n{}",
            block("185 650"),
            block("7 140"),
            block("103 835")
        );
        assert_eq!(awarded_names(&body).len(), 3);
        assert_eq!(awarded_value(&body), None);
        // One block is one award, and its figure is the notice's.
        let one = format!("SECTION V: AWARD OF CONTRACT\n{}", block("103 835"));
        assert_eq!(awarded_value(&one), Some((10_383_500, "EUR".to_owned(), None)));
    }

    /// Issue 508: the non-names the 2010 print put in the winner slot are refused,
    /// and real names that merely contain such a word are not.
    #[test]
    fn a_void_lot_or_a_pointer_in_the_winner_slot_names_nobody() {
        for junk in [
            "Véase perfil del contratante",
            "Ver perfil del contratante de la Junta de Andalucía",
            "Lot déclaré infructueux",
            "Infructueux le 25.1.2010",
            "Marché déclaré sans suite",
            "Non attribué",
            "See Section VI.2) Additional information",
            "Voir autres informations",
            "Ver información adicional",
            "Desierto",
            "Declarado Desierto",
            "Nessuna aggiudicazione",
            "Various",
        ] {
            assert!(!plausible_name(junk), "{junk}");
        }
        for name in ["Suite Hotels Ltd", "Desierto Florido SL", "Desertec GmbH", "Voirol SA", "Perfiles Andaluces SL"] {
            assert!(plausible_name(name), "{name}");
        }
        let body = "V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR IN FAVOUR OF WHOM A CONTRACT\n\
                    AWARD DECISION HAS BEEN TAKEN\n\
                    Véase perfil del contratante\n\
                    V.4)  INFORMATION ON VALUE OF CONTRACT";
        assert!(awarded_names(body).is_empty());
    }
}
