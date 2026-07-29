//! The XPath subset the eForms SDK actually uses in `fields.json`.
//!
//! Field and node locations come as absolute xpaths, e.g.
//! `/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cbc:ID/@schemeName`.
//! Across SDK 1.12–1.15 the predicates only ever take five shapes (existence,
//! `@attr='v'`, `text()='v'`, `text()=('v1','v2')`, and `not(...)` of those,
//! each optionally reached through a relative path with `..` steps). Rather
//! than pull in a full XPath engine we parse exactly that grammar — anything
//! outside it fails loudly at index-build time, which is a test failure, not a
//! silent mis-parse.

use std::fmt;

/// Prefix → namespace URI. eForms fixes six, SDK-DE adds `defext`; documents
/// rename the prefixes freely (`ns8:ContractNotice`, the DÖE JAXB serializer's
/// `ns2`…`ns9`), so matching is always by URI.
pub fn namespace(prefix: &str) -> Option<&'static str> {
    Some(match prefix {
        "cbc" => "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2",
        "cac" => "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2",
        "ext" => "urn:oasis:names:specification:ubl:schema:xsd:CommonExtensionComponents-2",
        "efext" => "http://data.europa.eu/p27/eforms-ubl-extensions/1",
        "efac" => "http://data.europa.eu/p27/eforms-ubl-extension-aggregate-components/1",
        "efbc" => "http://data.europa.eu/p27/eforms-ubl-extension-basic-components/1",
        // SDK-DE's national extension XSD declares this non-URL namespace
        // string verbatim (`german-eforms-extension.xsd`, eForms-DE ≥ 2.1 on
        // EU base 1.14) — the DEX statistics fields live under it.
        "defext" => "german-eforms-extension",
        _ => return None,
    })
}

/// One element step: a namespace-qualified name plus its predicates.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Step {
    pub ns: &'static str,
    pub local: String,
    pub preds: Vec<Pred>,
}

/// Where a path inside a predicate starts from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Origin {
    /// Relative to the context element, after `up` parent-element hops.
    Context { up: usize },
    /// The document-absolute descendant axis (`//`): the first step matches any
    /// descendant of the document root, the rest are child steps (SDK 1.0–1.7's
    /// `//efac:TenderingParty/…` join target, issue 74).
    Descendant,
}

/// A path used inside a predicate.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    pub origin: Origin,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pred {
    Not(Box<Pred>),
    /// A top-level `[(A) or (B)]` disjunction (issue 74). Left-associative for
    /// longer chains.
    Or(Box<Pred>, Box<Pred>),
    /// The path selects at least one element.
    Exists(Path),
    /// `path/@attr` equals one of the values.
    AttrEq { path: Path, attr: String, values: Vec<String> },
    /// `path/text()` equals one of the values.
    TextEq { path: Path, values: Vec<String> },
    /// `lhs/text() = rhs/text()` — a node-set join: some LHS text equals some RHS
    /// text (the SDK 1.0–1.7 Organization↔TenderingParty ID match, issue 74).
    TextJoin { lhs: Path, rhs: Path },
}

/// A parsed absolute location: element steps plus an optional attribute leaf.
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub steps: Vec<Step>,
    pub attribute: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported xpath: {}", self.0)
    }
}
impl std::error::Error for Error {}

/// Parse an SDK absolute xpath (`/*/…`). The leading `/*` root step is dropped:
/// the four eForms root elements differ per document type, so the root is
/// matched by position, not by name.
pub fn parse_absolute(xpath: &str) -> Result<Location, Error> {
    let rest = xpath.strip_prefix("/*/").or_else(|| xpath.strip_prefix("/*")).ok_or_else(|| Error(xpath.into()))?;
    if rest.is_empty() {
        return Ok(Location { steps: Vec::new(), attribute: None });
    }
    parse_relative(rest).map(|(path, attribute)| {
        debug_assert_eq!(path.origin, Origin::Context { up: 0 });
        Location { steps: path.steps, attribute }
    })
}

/// Parse a path used in a predicate, which may end in an `@attr` or `text()`
/// leaf. A leading `//` makes it document-descendant-rooted; otherwise it is
/// relative to the context element, with any leading `..` steps.
fn parse_relative(input: &str) -> Result<(Path, Option<String>), Error> {
    let input = input.trim();
    let (descendant, body) = match input.strip_prefix("//") {
        Some(rest) => (true, rest),
        None => (false, input),
    };
    let mut steps = Vec::new();
    let mut up = 0usize;
    let mut attribute = None;
    for segment in split_top_level(body, '/') {
        let segment = segment.trim();
        if segment == "." {
            // The self step (`./x` ≡ `x`, SDK 1.0–1.7 predicates) — a no-op.
            continue;
        } else if segment == ".." {
            // `..` is only valid leading the path, and never on a `//` root.
            if !steps.is_empty() || descendant {
                return Err(Error(input.into()));
            }
            up += 1;
        } else if let Some(attr) = segment.strip_prefix('@') {
            attribute = Some(attr.to_owned());
        } else if segment == "text()" {
            attribute = Some("text()".into());
        } else {
            steps.push(parse_step(segment)?);
        }
    }
    let origin = if descendant { Origin::Descendant } else { Origin::Context { up } };
    Ok((Path { origin, steps }, attribute))
}

fn parse_step(segment: &str) -> Result<Step, Error> {
    let name_end = segment.find('[').unwrap_or(segment.len());
    let (name, preds) = segment.split_at(name_end);
    let (prefix, local) = name.split_once(':').ok_or_else(|| Error(segment.into()))?;
    let ns = namespace(prefix).ok_or_else(|| Error(segment.into()))?;
    Ok(Step { ns, local: local.into(), preds: parse_predicates(preds)? })
}

fn parse_predicates(mut input: &str) -> Result<Vec<Pred>, Error> {
    let mut preds = Vec::new();
    while !input.is_empty() {
        if !input.starts_with('[') {
            return Err(Error(input.into()));
        }
        let end = matching_bracket(input).ok_or_else(|| Error(input.into()))?;
        preds.push(parse_predicate(&input[1..end])?);
        input = &input[end + 1..];
    }
    Ok(preds)
}

fn parse_predicate(body: &str) -> Result<Pred, Error> {
    let body = body.trim();

    // Top-level `(A) or (B)` disjunction (issue 74): split on the first depth-0
    // ` or `, strip each side's wrapping parens, recurse (left-associative).
    if let Some((lhs, rhs)) = split_top_level_word(body, "or") {
        return Ok(Pred::Or(
            Box::new(parse_predicate(unwrap_parens(lhs))?),
            Box::new(parse_predicate(unwrap_parens(rhs))?),
        ));
    }

    if let Some(inner) = body.strip_prefix("not(").and_then(|b| b.strip_suffix(')')) {
        return Ok(Pred::Not(Box::new(parse_predicate(inner)?)));
    }
    let Some((lhs, rhs)) = split_top_level_once(body, '=') else {
        let (path, attribute) = parse_relative(body)?;
        return match attribute {
            None => Ok(Pred::Exists(path)),
            Some(attr) => Ok(Pred::AttrEq { path, attr, values: Vec::new() }),
        };
    };
    let (lhs, rhs) = (lhs.trim(), rhs.trim());

    // A right side that is not a quoted literal / value list is another node-set
    // (a path ending in `text()`) — the join `lhs/text() = rhs/text()`. Only the
    // text()=text() shape occurs; anything else stays an error (strict subset).
    if !rhs.starts_with('\'') && !rhs.starts_with('(') {
        let (lhs_path, lhs_attr) = parse_relative(lhs)?;
        let (rhs_path, rhs_attr) = parse_relative(rhs)?;
        return match (lhs_attr.as_deref(), rhs_attr.as_deref()) {
            (Some("text()"), Some("text()")) => Ok(Pred::TextJoin { lhs: lhs_path, rhs: rhs_path }),
            _ => Err(Error(body.into())),
        };
    }

    let values = parse_values(rhs)?;
    let (path, attribute) = parse_relative(lhs)?;
    match attribute.as_deref() {
        Some("text()") => Ok(Pred::TextEq { path, values }),
        Some(attr) => Ok(Pred::AttrEq { path, attr: attr.into(), values }),
        None => Err(Error(body.into())),
    }
}

/// Strip one layer of parentheses when they wrap the whole string.
fn unwrap_parens(input: &str) -> &str {
    let s = input.trim();
    if s.starts_with('(') && matching_delim(s, '(', ')') == Some(s.len() - 1) {
        return s[1..s.len() - 1].trim();
    }
    s
}

/// `'a'` or `('a','b')`.
fn parse_values(input: &str) -> Result<Vec<String>, Error> {
    let inner = match input.strip_prefix('(').and_then(|i| i.strip_suffix(')')) {
        Some(list) => list,
        None => input,
    };
    inner
        .split(',')
        .map(|v| {
            let v = v.trim();
            v.strip_prefix('\'')
                .and_then(|v| v.strip_suffix('\''))
                .map(str::to_owned)
                .ok_or_else(|| Error(input.into()))
        })
        .collect()
}

/// Split on `sep`, ignoring separators inside `[]` or `''`.
fn split_top_level(input: &str, sep: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut start = 0;
    for (i, c) in input.char_indices() {
        match c {
            '\'' => quoted = !quoted,
            '[' if !quoted => depth += 1,
            ']' if !quoted => depth = depth.saturating_sub(1),
            c if c == sep && depth == 0 && !quoted => {
                parts.push(&input[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&input[start..]);
    parts
}

fn split_top_level_once(input: &str, sep: char) -> Option<(&str, &str)> {
    let parts = split_top_level(input, sep);
    (parts.len() > 1).then(|| (parts[0], &input[parts[0].len() + sep.len_utf8()..]))
}

/// Split on the first top-level ` <word> ` (a space-delimited keyword outside any
/// `()`, `[]` or `''`), dropping the word — the boolean `or` between predicates.
fn split_top_level_word<'a>(input: &'a str, word: &str) -> Option<(&'a str, &'a str)> {
    let needle = format!(" {word} ");
    let (mut paren, mut bracket, mut quoted) = (0usize, 0usize, false);
    for (i, c) in input.char_indices() {
        match c {
            '\'' => quoted = !quoted,
            '(' if !quoted => paren += 1,
            ')' if !quoted => paren = paren.saturating_sub(1),
            '[' if !quoted => bracket += 1,
            ']' if !quoted => bracket = bracket.saturating_sub(1),
            ' ' if !quoted && paren == 0 && bracket == 0 && input[i..].starts_with(&needle) => {
                return Some((&input[..i], &input[i + needle.len()..]));
            }
            _ => {}
        }
    }
    None
}

/// Index of the delimiter closing the `open` at position 0 (respecting quotes).
fn matching_delim(input: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut quoted = false;
    for (i, c) in input.char_indices() {
        match c {
            '\'' => quoted = !quoted,
            c if c == open && !quoted => depth += 1,
            c if c == close && !quoted => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Index of the `]` closing the `[` at position 0.
fn matching_bracket(input: &str) -> Option<usize> {
    matching_delim(input, '[', ']')
}

// ---------------------------------------------------------------- evaluation

type Node<'a, 'i> = roxmltree::Node<'a, 'i>;

impl Pred {
    /// Evaluate against the element the predicate is attached to.
    pub fn eval(&self, node: Node<'_, '_>) -> bool {
        match self {
            Pred::Not(inner) => !inner.eval(node),
            Pred::Or(a, b) => a.eval(node) || b.eval(node),
            Pred::Exists(path) => !path.select(node).is_empty(),
            Pred::AttrEq { path, attr, values } => path.select(node).into_iter().any(|n| {
                match n.attribute_node(attr.as_str()).map(|a| a.value()) {
                    Some(v) => values.is_empty() || values.iter().any(|w| w == v),
                    None => false,
                }
            }),
            Pred::TextEq { path, values } => path
                .select(node)
                .into_iter()
                .any(|n| n.text().map(str::trim).is_some_and(|t| values.iter().any(|w| w == t))),
            // Node-set join: some LHS text equals some RHS text (XPath general
            // comparison). Both sides trimmed and empties dropped, like `TextEq`.
            Pred::TextJoin { lhs, rhs } => {
                let texts = |p: &Path| -> Vec<String> {
                    p.select(node)
                        .into_iter()
                        .filter_map(|n| n.text().map(|t| t.trim().to_owned()))
                        .filter(|t| !t.is_empty())
                        .collect()
                };
                let (left, right) = (texts(lhs), texts(rhs));
                left.iter().any(|l| right.contains(l))
            }
        }
    }
}

impl Path {
    /// Elements this path selects. `Context` paths start at `node` (after `up`
    /// parent hops); a `Descendant` (`//`) path starts at the document root and
    /// matches its first step against any descendant, then child-steps the rest.
    pub fn select<'a, 'i>(&self, node: Node<'a, 'i>) -> Vec<Node<'a, 'i>> {
        let (mut current, rest) = match &self.origin {
            Origin::Context { up } => {
                let mut start = node;
                for _ in 0..*up {
                    match start.parent_element() {
                        Some(p) => start = p,
                        None => return Vec::new(),
                    }
                }
                (vec![start], &self.steps[..])
            }
            Origin::Descendant => {
                let Some((first, rest)) = self.steps.split_first() else { return Vec::new() };
                let hits =
                    node.document().root().descendants().filter(|n| first.matches(*n)).collect();
                (hits, rest)
            }
        };
        for step in rest {
            current = current.iter().flat_map(|n| n.children()).filter(|c| step.matches(*c)).collect();
        }
        current
    }
}

impl Step {
    /// Does this element match the step's name and all its predicates?
    pub fn matches(&self, node: Node<'_, '_>) -> bool {
        self.matches_name(node) && self.preds.iter().all(|p| p.eval(node))
    }

    /// Name and namespace only — matching by URI, never by prefix.
    pub fn matches_name(&self, node: Node<'_, '_>) -> bool {
        node.is_element()
            && node.tag_name().name() == self.local
            && node.tag_name().namespace() == Some(self.ns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps(xpath: &str) -> Location {
        parse_absolute(xpath).unwrap()
    }

    #[test]
    fn parses_the_predicate_shapes_the_sdk_uses() {
        // Plain path.
        let l = steps("/*/cac:TenderingTerms/cbc:Note");
        assert_eq!(l.steps.len(), 2);
        assert!(l.attribute.is_none());

        // Attribute leaf.
        let l = steps("/*/cbc:ID/@schemeName");
        assert_eq!(l.attribute.as_deref(), Some("schemeName"));

        // Attribute equality on a child.
        let l = steps("/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cbc:ID");
        assert_eq!(l.steps[0].preds.len(), 1);

        // Value-set equality inside not().
        let l = steps(
            "/*/cac:ProcurementLegislationDocumentReference[not(cbc:ID/text()=('CrossBorderLaw','LocalLegalBasis'))]/cbc:ID",
        );
        let Pred::Not(inner) = &l.steps[0].preds[0] else { panic!("expected not()") };
        let Pred::TextEq { values, .. } = &**inner else { panic!("expected text()=") };
        assert_eq!(values, &["CrossBorderLaw", "LocalLegalBasis"]);

        // Nested predicate inside a predicate path, and bare existence.
        let l = steps(
            "/*/cac:TenderingTerms[not(cac:SpecificTendererRequirement/cbc:TendererRequirementTypeCode[@listName='exclusion-ground'])]/cbc:Note",
        );
        assert!(matches!(&l.steps[0].preds[0], Pred::Not(_)));

        // Parent axis.
        let l = steps("/*/cac:CallForTendersDocumentReference[../cbc:DocumentType/text()='restricted-document']/cbc:ID");
        let Pred::TextEq { path, .. } = &l.steps[0].preds[0] else { panic!() };
        assert_eq!(path.origin, Origin::Context { up: 1 });

        // Spaces around the operator.
        assert!(parse_absolute("/*/cac:PartyLegalEntity[cbc:CompanyID/@schemeName = 'EU']/cbc:CompanyID").is_ok());
    }

    /// Issue 74: the SDK 1.0–1.7 `efbc:CompanySizeCode` (BT-165) predicate uses
    /// three constructs the earlier grammar lacked — a top-level `or`, the `//`
    /// descendant axis, and a node-set join `text() = path/text()`.
    #[test]
    fn parses_the_company_size_join_predicate() {
        let l = steps(
            "/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company[(cac:PartyIdentification/cbc:ID/text() = //efac:TenderingParty/efac:Tenderer/cbc:ID/text()) or (cac:PartyIdentification/cbc:ID/text() = //efac:TenderingParty/efac:Subcontractor/cbc:ID/text())]/efbc:CompanySizeCode",
        );
        // The predicate hangs on the efac:Company step (second from the leaf).
        let company = l.steps.iter().find(|s| s.local == "Company").expect("Company step");
        let [Pred::Or(a, b)] = &company.preds[..] else { panic!("expected one Or predicate") };

        // Each side is a join of a relative LHS against a `//`-rooted RHS.
        let Pred::TextJoin { lhs, rhs } = &**a else { panic!("side A is a TextJoin") };
        assert_eq!(lhs.origin, Origin::Context { up: 0 });
        assert_eq!(lhs.steps.iter().map(|s| s.local.as_str()).collect::<Vec<_>>(), ["PartyIdentification", "ID"]);
        assert_eq!(rhs.origin, Origin::Descendant);
        assert_eq!(rhs.steps.iter().map(|s| s.local.as_str()).collect::<Vec<_>>(), ["TenderingParty", "Tenderer", "ID"]);

        let Pred::TextJoin { rhs, .. } = &**b else { panic!("side B is a TextJoin") };
        assert_eq!(rhs.steps.last().map(|s| s.local.as_str()), Some("ID"));
        assert_eq!(rhs.steps[1].local, "Subcontractor");
    }

    #[test]
    fn rejects_paths_it_cannot_represent() {
        assert!(parse_absolute("/*/unknownprefix:Thing").is_err());
        assert!(parse_absolute("cbc:ID").is_err());
        // A join against something other than text()=path/text() stays unsupported.
        assert!(parse_absolute("/*/efac:Company[cbc:ID/@schemeName = //efac:X/cbc:ID/@schemeName]/efbc:X").is_err());
    }

    /// The join evaluates as an XPath general comparison: the Company is selected
    /// iff its PartyIdentification/ID matches a Tenderer's (side A) OR a
    /// Subcontractor's (side B) ID, anywhere in the document.
    #[test]
    fn the_join_predicate_matches_the_right_company() {
        const DOC: &str = r#"<root xmlns:efac="http://data.europa.eu/p27/eforms-ubl-extension-aggregate-components/1"
    xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
    xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <efac:Organizations>
    <efac:Organization><efac:Company id="winner"><cac:PartyIdentification><cbc:ID>ORG-0001</cbc:ID></cac:PartyIdentification></efac:Company></efac:Organization>
    <efac:Organization><efac:Company id="buyer"><cac:PartyIdentification><cbc:ID>ORG-0009</cbc:ID></cac:PartyIdentification></efac:Company></efac:Organization>
    <efac:Organization><efac:Company id="sub"><cac:PartyIdentification><cbc:ID>ORG-0002</cbc:ID></cac:PartyIdentification></efac:Company></efac:Organization>
  </efac:Organizations>
  <efac:TenderingParty>
    <efac:Tenderer><cbc:ID>ORG-0001</cbc:ID></efac:Tenderer>
    <efac:Subcontractor><cbc:ID>ORG-0002</cbc:ID></efac:Subcontractor>
  </efac:TenderingParty>
</root>"#;
        let doc = roxmltree::Document::parse(DOC).unwrap();
        let pred = &parse_predicate(
            "(cac:PartyIdentification/cbc:ID/text() = //efac:TenderingParty/efac:Tenderer/cbc:ID/text()) or (cac:PartyIdentification/cbc:ID/text() = //efac:TenderingParty/efac:Subcontractor/cbc:ID/text())",
        )
        .unwrap();
        let company = |id: &str| {
            doc.descendants()
                .find(|n| n.tag_name().name() == "Company" && n.attribute("id") == Some(id))
                .unwrap()
        };
        assert!(pred.eval(company("winner")), "the tenderer org matches side A");
        assert!(pred.eval(company("sub")), "the subcontractor org matches side B");
        assert!(!pred.eval(company("buyer")), "a non-party org matches neither side");
    }
}
