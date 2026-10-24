#!/usr/bin/env python3
"""W7 (issue 551) for issue 577 unit 2 (commit 406529a): which (old id -> new id) moves can the
patch cause? Read-only emulation of crates/ingest/src/eforms/index.rs::build at HEAD (post-577)
plus Index::bind (D577-1), done two ways:

  1. BIND moves: a port of the commit's own index-wide invariant test
     `a_declared_field_binds_over_a_patch_only_where_d577_1_says` (crates/ingest/tests/eforms.rs):
     every pair of fields one element can carry through two branches (`co_bound`, `co_match`,
     `select_order` copied), first in select order, whose `bind` result is not the first field.
     The port is validated against the test's own pins (SDK-shaped sets exact; eforms-de-1.x
     118 pairs / 113 declared ids and the three non-lot declared ids), which the gate ran green
     at 406529a. The test only pins de-1.x by count; this port names all 118 pairs.
  2. EXTRA moves: build the index twice, the top-level EXTRA insert gap_only=false (before 577)
     and gap_only=true (after), and list every branch whose field id differs. This catches the
     4 declared-xpath overwrites the commit names and any EXTRA-vs-EXTRA duplicate xpath.

Built on .scratch/tender-db/577-triage/shadow.py's parser (copied, not imported), extended with
the node descents and attribute-element descents the Rust build does (they decide the
`existing()` guards of the direct calls), the `declared` / `negated_only` flags and the bind rule.

Usage: python3 -I emulate_577u2.py <repo-root> <out-dir>
Writes <out-dir>/moves_bind.tsv, moves_extra.tsv, emulation_check.txt
"""
import json, re, sys
from collections import defaultdict

ROOT = "/home/user/tender-db"
SRC = open(f"{ROOT}/crates/ingest/src/eforms/index.rs").read()

ACCEPTED = [
    ("eforms-sdk-1.0", "fields-1.0.0.json"), ("eforms-sdk-1.2", "fields-1.2.0.json"),
    ("eforms-sdk-1.3", "fields-1.3.0.json"), ("eforms-sdk-1.5", "fields-1.5.0.json"),
    ("eforms-sdk-1.6", "fields-1.6.0.json"), ("eforms-sdk-1.7", "fields-1.7.0.json"),
    ("eforms-sdk-1.8", "fields-1.8.0.json"), ("eforms-sdk-1.9", "fields-1.9.0.json"),
    ("eforms-sdk-1.10", "fields-1.10.0.json"), ("eforms-sdk-1.11", "fields-1.11.0.json"),
    ("eforms-sdk-1.12", "fields-1.12.0.json"), ("eforms-sdk-1.13", "fields-1.13.0.json"),
    ("eforms-sdk-1.14", "fields-1.14.0.json"), ("eforms-sdk-1.15", "fields-1.15.0.json"),
    ("eforms-de-2.0", "fields-de-2.0.0.json"),
    ("eforms-de-2.1@eforms-sdk-1.13", "fields-de-2.1.0-eu-1.13.json"),
    ("eforms-de-2.1@eforms-sdk-1.14", "fields-de-2.1.0-eu-1.14.json"),
    ("eforms-sdk-0.1", "fields-sdk-0.1.json"),
    ("eforms-de-1.x", "fields-de-1.x.json"),
]


def rust_strings(block):
    out = []
    for m in re.finditer(r'"((?:[^"\\]|\\.)*)"', block, re.S):
        s = re.sub(r"\\\n\s*", "", m.group(1))
        out.append(s)
    return out


def const_block(name):
    i = SRC.index(f"pub const {name}")
    j = SRC.index("\n];", i)
    return SRC[i:j]


def tuples(block, n):
    res = []
    for m in re.finditer(r"\(\s*((?:\"(?:[^\"\\]|\\.)*\"\s*,?\s*){%d})\)" % n, block, re.S):
        res.append(tuple(rust_strings(m.group(1))))
    return res


ALIASES = tuples(const_block("ALIASES"), 2)
EXTRA = tuples(const_block("EXTRA"), 3)
assert len(ALIASES) == 36 and len(EXTRA) == 75, (len(ALIASES), len(EXTRA))


def split_top(s, sep="/"):
    parts, depth, cur, q = [], 0, "", None
    for ch in s:
        if q:
            cur += ch
            if ch == q:
                q = None
            continue
        if ch in "'\"":
            q = ch
        elif ch in "[(":
            depth += 1
        elif ch in "])":
            depth -= 1
        if ch == sep and depth == 0:
            parts.append(cur)
            cur = ""
        else:
            cur += ch
    parts.append(cur)
    return parts


def parse_step(seg):
    k = seg.find("[")
    name = seg if k < 0 else seg[:k]
    preds, rest = [], ("" if k < 0 else seg[k:])
    while rest:
        assert rest[0] == "[", seg
        depth, q = 0, None
        for i, ch in enumerate(rest):
            if q:
                if ch == q:
                    q = None
                continue
            if ch in "'\"":
                q = ch
            elif ch == "[":
                depth += 1
            elif ch == "]":
                depth -= 1
                if depth == 0:
                    break
        preds.append(re.sub(r"\s+", "", rest[1:i]))
        rest = rest[i + 1:]
    # Rust's Step equality compares the parsed predicate Vec in order; a frozenset is the
    # generous reading (two orderings of one set are one branch here), kept from shadow.py.
    return (name, frozenset(preds), len(preds))


def locate(xpath):
    assert xpath.startswith("/*"), xpath
    rest = xpath[3:] if xpath.startswith("/*/") else xpath[2:]
    attr, steps = None, []
    for seg in split_top(rest):
        seg = seg.strip()
        if not seg or seg == ".":
            continue
        if seg.startswith("@"):
            attr = seg[1:]
        else:
            steps.append(parse_step(seg))
    return tuple(steps), attr


def is_not(p):
    if not p.startswith("not(") or not p.endswith(")"):
        return False
    depth, q = 0, None
    for i, ch in enumerate(p):
        if q:
            if ch == q:
                q = None
            continue
        if ch in "'\"":
            q = ch
        elif ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
            if depth == 0 and i != len(p) - 1:
                return False
    return True


def negated_only(steps):
    return all(is_not(p) for s in steps for p in s[1])


class Branch:
    __slots__ = ("children", "field")

    def __init__(self):
        self.children = {}
        self.field = None  # dict(id, prov, declared, negated_only)


def descend(root, steps):
    b = root
    for s in steps:
        if s not in b.children:
            b.children[s] = Branch()
        b = b.children[s]
    return b


def existing(root, steps):
    b = root
    for s in steps:
        b = b.children.get(s)
        if b is None:
            return None
    return b


def place(root, xpath, fid, gap_only, prov, declared):
    steps, attr = locate(xpath)
    b = descend(root, steps)  # Rust descends the element for attribute fields too
    if attr is not None:
        return
    if gap_only and b.field is not None:
        return
    b.field = {"id": fid, "prov": prov, "declared": declared, "negated_only": (not declared) and negated_only(steps)}


DIRECT = [
    ("/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:Organizations/efac:Organization/efac:Company/efbc:CompanySizeCode", "BT-165-Organization-Company"),
    ("/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efbc:TransmissionDate", "BT-803(d)-notice"),
    ("/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efbc:TransmissionTime", "BT-803(t)-notice"),
    ("/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efbc:FrameworkMaximumAmount", "UBL-FrameworkMaximumAmount"),
    ("/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:ContractExecutionRequirement[cbc:ExecutionRequirementCode/@listName='conditions']/cbc:ExecutionRequirementCode", "OPT-060-Lot"),
    ("/*/cac:TenderingProcess/cac:ProcessJustification/cbc:Description", "UBL-ProcessJustificationDescription"),
    ("/*/cac:ProcurementProject/cac:ProcurementAdditionalType/cbc:ProcurementTypeCode", "UBL-ProcurementAdditionalTypeCode"),
    ("/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:ProcurementProject/cac:ProcurementAdditionalType/cbc:ProcurementTypeCode", "UBL-ProcurementAdditionalTypeCode"),
    ("/*/cac:BusinessParty/cac:PartyLegalEntity[cbc:CompanyID/@schemeName='EU']/cbc:RegistrationName", "BT-500-Business-European"),
    ("/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:CallForTendersDocumentReference/cbc:DocumentTypeCode", "BT-707-Lot"),
    ("/*/cac:AdditionalDocumentReference/cbc:ID", "OPP-124-Business"),
    ("/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingProcess/cac:ProcessJustification/cbc:Description", "BT-745-Lot"),
    ("/*/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:NoticeResult/efac:LotTender/efac:SubcontractingTerm/efbc:TermCode", "BT-773-Tender"),
]


def build(sdk, version, extra_gap_only):
    root = Branch()
    for node in sdk.get("xmlStructure", []):
        if not node.get("repeatable") and "efac:FieldsPrivacy" not in node["xpathAbsolute"]:
            continue
        descend(root, locate(node["xpathAbsolute"])[0])
    fields = sdk["fields"]
    for f in fields:
        if f["id"].startswith("OPA-"):
            continue  # VirtualView: Rust returns before descending
        place(root, f["xpathAbsolute"], f["id"], False, "declared", True)
    if version == "eforms-sdk-0.1":
        return root
    for x, fid, kind in EXTRA:
        place(root, x, fid, extra_gap_only, "extra", False)
    for x, fid in DIRECT:
        place(root, x, fid, True, "direct", False)
    bare_cer = "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:ContractExecutionRequirement"
    if existing(root, locate(bare_cer)[0]):
        place(root, bare_cer + "/cbc:Description", "UBL-ContractExecutionDescription", True, "direct", False)
    for scheme in ["Lot", "LotsGroup"]:
        p = f"/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='{scheme}']/cac:TenderingTerms/cac:AwardingTerms/cac:AwardingCriterion/cac:SubordinateAwardingCriterion/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:AwardCriterionParameter"
        if existing(root, locate(p)[0]):
            place(root, p + "/efbc:ParameterCode", "UBL-AwardCriterionParameterCode", True, "direct", False)
    for scheme in ["Lot", "LotsGroup"]:
        p = f"/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='{scheme}']/cac:TenderingTerms/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:SelectionCriteria/efac:CriterionParameter"
        if existing(root, locate(p)[0]):
            place(root, p + "/efbc:ParameterCode", "UBL-SelectionCriterionParameterCode", True, "direct", False)
    if not existing(root, locate("/*/cac:ProcurementProjectLot/cac:TenderingTerms/cac:ProcurementLegislationDocumentReference")[0]):
        for scheme in ["Lot", "Part"]:
            p = f"/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='{scheme}']/cac:TenderingTerms/cac:ProcurementLegislationDocumentReference"
            place(root, p + "/cbc:ID", "UBL-ProcurementLegislationID", True, "direct", False)
            place(root, p + "/cbc:DocumentDescription", "UBL-ProcurementLegislationDescription", True, "direct", False)
    if not existing(root, locate("/*/cac:ProcurementProjectLot/cac:ProcurementProject/cac:ContractExtension/cbc:RenewalsIndicator")[0]):
        place(root, "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:ProcurementProject/cac:ContractExtension/cbc:RenewalsIndicator", "UBL-RenewalsIndicator", True, "direct", False)
    for src, tgt in ALIASES:
        for f in fields:
            if f["id"].startswith("OPA-"):
                continue
            x = f["xpathAbsolute"]
            if x.startswith(src):
                place(root, tgt + x[len(src):], f["id"], True, f"alias:{src}=>{tgt}", False)
        for x, fid, kind in EXTRA:
            if x.startswith(src):
                place(root, tgt + x[len(src):], fid, True, f"alias:{src}=>{tgt}", False)
    return root


# --- the test's co_match / co_bound / select_order, ported --------------------------------
ATTR_EQ = re.compile(r"^(?:([A-Za-z0-9_-]+:)?([A-Za-z0-9_-]+)/)?@([A-Za-z0-9_:-]+)=(.+)$")


def attr_eq(p):
    m = ATTR_EQ.match(p)
    if not m:
        return None
    path_local = m.group(2)
    vals = m.group(4)
    if vals.startswith("(") and vals.endswith(")"):
        vals = [v.strip("'\"") for v in vals[1:-1].split(",")]
    elif (vals.startswith("'") and vals.endswith("'")) or (vals.startswith('"') and vals.endswith('"')):
        vals = [vals[1:-1]]
    else:
        return None
    return (m.group(1) or "") + (path_local or ""), path_local, m.group(3), frozenset(vals)


def excludes(p, q):
    if is_not(p) and p[4:-1] == q:
        return True
    a, b = attr_eq(p), attr_eq(q)
    if a and b:
        single = a[1] is None or a[1] == "ID"
        return single and a[0] == b[0] and a[2] == b[2] and a[3] and b[3] and not (a[3] & b[3])
    return False


def co_match(sa, sb):
    if sa[0] != sb[0]:
        return False
    return not any(excludes(p, q) or excludes(q, p) for p in sa[1] for q in sb[1])


def select_order(branch):
    items = list(branch.children.items())
    return sorted(items, key=lambda kv: -kv[0][2])  # stable: ties keep insertion order


def co_bound(branch, out):
    def down(first, other):
        (sa, a), (sb, b) = first, other
        if a.field and b.field:
            out.append((a.field, b.field))
        for ca in a.children.items():
            for cb in b.children.items():
                if co_match(ca[0], cb[0]):
                    down(ca, cb)

    order = select_order(branch)
    for i, first in enumerate(order):
        for other in order[i + 1:]:
            if co_match(first[0], other[0]):
                down(first, other)
        co_bound(first[1], out)


def same_term(a, b):
    def unscoped(fid):
        depth, dashes = 0, []
        for i, c in enumerate(fid):
            if c == "(":
                depth += 1
            elif c == ")":
                depth = max(0, depth - 1)
            elif c == "-" and depth == 0:
                dashes.append(i)
        if len(dashes) < 2:
            return None
        first, scope = dashes[0], dashes[1]
        if not fid[first + 1:first + 2].isdigit():
            return None
        return (fid[:scope], fid[dashes[2]:] if len(dashes) > 2 else "")
    ua, ub = unscoped(a), unscoped(b)
    return ua is not None and ub is not None and ua == ub


def bind(de1, fields):
    first = fields[0]
    if first["declared"]:
        return first
    for d in fields:
        if d["declared"] and (de1 or first["negated_only"] or same_term(d["id"], first["id"])):
            return d
    return first


def diff_trees(old, new, path, out):
    of, nf = old.field, new.field
    if (of and of["id"]) != (nf and nf["id"]):
        out.append((path, of and of["id"], nf and nf["id"], of and of["prov"], nf and nf["prov"]))
    assert list(old.children) == list(new.children), path
    for s in old.children:
        diff_trees(old.children[s], new.children[s], path + (s,), out)


def fmt(path):
    return "/" + "/".join(n + "".join(f"[{p}]" for p in sorted(ps)) for n, ps, _ in path)


