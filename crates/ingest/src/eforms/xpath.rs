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

/// Prefix → namespace URI. eForms fixes these six; documents rename the
/// prefixes freely (`ns8:ContractNotice`), so matching is always by URI.
pub fn namespace(prefix: &str) -> Option<&'static str> {
    Some(match prefix {
        "cbc" => "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2",
        "cac" => "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2",
        "ext" => "urn:oasis:names:specification:ubl:schema:xsd:CommonExtensionComponents-2",
        "efext" => "http://data.europa.eu/p27/eforms-ubl-extensions/1",
        "efac" => "http://data.europa.eu/p27/eforms-ubl-extension-aggregate-components/1",
        "efbc" => "http://data.europa.eu/p27/eforms-ubl-extension-basic-components/1",
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

/// A relative path used inside a predicate.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    /// Leading `..` steps.
    pub up: usize,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pred {
    Not(Box<Pred>),
    /// The path selects at least one element.
    Exists(Path),
    /// `path/@attr` equals one of the values.
    AttrEq { path: Path, attr: String, values: Vec<String> },
    /// `path/text()` equals one of the values.
    TextEq { path: Path, values: Vec<String> },
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
        debug_assert_eq!(path.up, 0);
        Location { steps: path.steps, attribute }
    })
}

/// Parse a relative path, which may end in an `@attr` or `text()` leaf.
fn parse_relative(input: &str) -> Result<(Path, Option<String>), Error> {
    let mut path = Path { up: 0, steps: Vec::new() };
    let mut attribute = None;
    for segment in split_top_level(input, '/') {
        let segment = segment.trim();
        if segment == ".." {
            if !path.steps.is_empty() {
                return Err(Error(input.into()));
            }
            path.up += 1;
        } else if let Some(attr) = segment.strip_prefix('@') {
            attribute = Some(attr.to_owned());
        } else if segment == "text()" {
            attribute = Some("text()".into());
        } else {
            path.steps.push(parse_step(segment)?);
        }
    }
    Ok((path, attribute))
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
    let values = parse_values(rhs.trim())?;
    let (path, attribute) = parse_relative(lhs.trim())?;
    match attribute.as_deref() {
        Some("text()") => Ok(Pred::TextEq { path, values }),
        Some(attr) => Ok(Pred::AttrEq { path, attr: attr.into(), values }),
        None => Err(Error(body.into())),
    }
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

/// Index of the `]` closing the `[` at position 0.
fn matching_bracket(input: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut quoted = false;
    for (i, c) in input.char_indices() {
        match c {
            '\'' => quoted = !quoted,
            '[' if !quoted => depth += 1,
            ']' if !quoted => {
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

// ---------------------------------------------------------------- evaluation

type Node<'a, 'i> = roxmltree::Node<'a, 'i>;

impl Pred {
    /// Evaluate against the element the predicate is attached to.
    pub fn eval(&self, node: Node<'_, '_>) -> bool {
        match self {
            Pred::Not(inner) => !inner.eval(node),
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
        }
    }
}

impl Path {
    /// Elements selected by this relative path, starting at `node`.
    pub fn select<'a, 'i>(&self, node: Node<'a, 'i>) -> Vec<Node<'a, 'i>> {
        let mut start = node;
        for _ in 0..self.up {
            match start.parent_element() {
                Some(p) => start = p,
                None => return Vec::new(),
            }
        }
        let mut current = vec![start];
        for step in &self.steps {
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
        assert_eq!(path.up, 1);

        // Spaces around the operator.
        assert!(parse_absolute("/*/cac:PartyLegalEntity[cbc:CompanyID/@schemeName = 'EU']/cbc:CompanyID").is_ok());
    }

    #[test]
    fn rejects_paths_it_cannot_represent() {
        assert!(parse_absolute("/*/unknownprefix:Thing").is_err());
        assert!(parse_absolute("cbc:ID").is_err());
    }
}
