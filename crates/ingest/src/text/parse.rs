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

/// The two labels under which a 2004-or-later body announces its winner (issue 244),
/// upper-cased for a case-insensitive match. Both are the tail of a longer heading and
/// both end at the colon the value follows:
///
/// - `V.1.1)  Name and address of successful supplier, contractor or service provider:`
///   — the 2004/2005 vintage, the same wording the era's own `CO:` line carries;
/// - `V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR TO WHOM THE CONTRACT HAS BEEN
///   AWARDED:` — 2006 onward.
///
/// Measured coverage of award notices carrying an English body, per June window on
/// prod: 2004 202/203, 2005 311/314, 2006 347/354, 2008 361/371. Matching the tail
/// rather than the whole heading is deliberate: the heading itself wraps at ~72
/// columns, and the words before the colon are the part that stayed stable.
const AWARD_LABELS: [&str; 2] = ["SERVICE PROVIDER:", "HAS BEEN AWARDED:"];

/// How far past the label a name's end is looked for. Generous next to the measured
/// shapes — the longest name seen on prod is 89 characters and its comma follows
/// immediately — and the bound is what keeps the scan linear in the body rather than
/// quadratic in (awards × length).
const NAME_WINDOW: usize = 256;

/// Where a winner's name ends. The value runs `<name>, <address…>. Tel. …`, so the
/// comma is the boundary in every measured shape; the rest are stops that catch a
/// value with no comma at all before the next heading, so a missing comma truncates
/// to something plausible instead of swallowing the remaining form.
const NAME_STOPS: [&str; 6] = [",", "V.1.2)", "V.2)", "V.3)", "V.4)", "CONTRACT NO"];

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
    let last = head.rsplit('.').next().unwrap_or(head);
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
    // Flatten TED's ~72-column wrap into one line. Built directly rather than through
    // `split_whitespace().collect::<Vec<_>>().join(" ")`, which allocated a vector of
    // slices as well as the string.
    let mut flat = String::with_capacity(body.len());
    for word in body.split_whitespace() {
        if !flat.is_empty() {
            flat.push(' ');
        }
        flat.push_str(word);
    }

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
        // Look for the name's end in a WINDOW, not in the rest of the notice. Scanning
        // the whole remainder made the function quadratic in (awards × body length), and
        // a notice awarding hundreds of contracts then took minutes: the campaign's
        // `fetch 186` went from 148 s to under two members a minute at 133% CPU with
        // three writer acquisitions in 45 seconds. A winner's name is never 8 kB from
        // its own label.
        let window = &rest[..char_bound(rest, NAME_WINDOW)];
        let Some(end) = NAME_STOPS.iter().filter_map(|stop| find_ascii_ci(window, stop)).min()
        else {
            // No boundary inside the window: the value is not a shape this recognises.
            // Skipping beats taking the window verbatim — a 256-byte "name" would mint
            // an organization per notice and poison the identity that has no identifier
            // to fall back on (issue 234).
            at = value_at;
            continue;
        };
        let name = trim_sentence_period(window[..end].trim());
        if !name.is_empty() {
            names.push(name.to_owned());
        }
        at = value_at;
    }
    names
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
    Ok(emit.parsed)
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
                // `AU` is the awarding authority's NAME — one clean line in every
                // vintage the fixtures cover (1993, 1995, 2000, 2005, 2008) — so it
                // becomes an Organization rather than a text row on the root.
                if field.code == "AU" {
                    emit.authority(text);
                } else {
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
        }
    }
    Ok(())
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
        // matter: `ORG_NAME_FIELDS` in the projection reads only `TED-OFFICIALNAME`
        // and `TXT-AU`, so a name filed anywhere else leaves the organization
        // NAMELESS; and `TXT-CO` already exists on the root when the era publishes its
        // `CO:` line, so reusing it would make one field id mean two different things
        // in one notice. The buyer's `TXT-AU` has neither problem — it is in that list
        // and it is not also a root value.
        self.push_into(&org, "TED-OFFICIALNAME", NoticeValue::Text { lang: None, value: name });
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

}
