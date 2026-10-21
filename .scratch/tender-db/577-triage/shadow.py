#!/usr/bin/env python3
"""Issue 577 triage: emulate crates/ingest/src/eforms/index.rs::build per vendored SDK
and list every alias-grafted field that SHADOWS a declared field at the declared
field's own element (graft branch is a predicate-superset sibling, so
Branch::matching sorts it first and parse.rs find_map binds its id).

Read-only: reads index.rs + crates/ingest/sdk/fields-*.json, prints a report.
Usage: python3 -I shadow.py <repo-root>
"""
import json, re, sys
from collections import defaultdict

ROOT = sys.argv[1]
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
    ("eforms-de-1.x", "fields-de-1.x.json"),
]


def rust_strings(block):
    # concatenated "..." literals with `\` line continuations (leading ws dropped)
    out = []
    for m in re.finditer(r'"((?:[^"\\]|\\.)*)"', block, re.S):
        s = m.group(1)
        s = re.sub(r"\\\n\s*", "", s)
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
assert len(ALIASES) > 30 and len(EXTRA) > 60, (len(ALIASES), len(EXTRA))


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
    return (name, frozenset(preds), len(preds))


def locate(xpath):
    rest = xpath[3:] if xpath.startswith("/*/") else xpath[2:]
    assert xpath.startswith("/*"), xpath
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


class Branch:
    __slots__ = ("children", "field", "order")

    def __init__(self):
        self.children = {}  # step -> Branch (dict keeps insertion order)
        self.field = None  # (id, provenance)


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


def insert(root, xpath, fid, gap_only, prov):
    steps, attr = locate(xpath)
    if attr is not None:
        return  # attribute fields are claimed, never emitted as values
    b = descend(root, steps)
    if gap_only and b.field is not None:
        return
    b.field = (fid, prov)


def build(fields, version):
    root = Branch()
    for f in fields:
        if f.get("attributeOf") or f["id"].startswith("OPA-"):
            continue
        insert(root, f["xpathAbsolute"], f["id"], False, "declared")
    if version == "eforms-sdk-0.1":
        return root
    for x, fid, kind in EXTRA:
        insert(root, x, fid, False, "extra")
    direct = [
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
    for x, fid in direct:
        insert(root, x, fid, True, "direct")
    bare_cer = "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/cac:ContractExecutionRequirement"
    if existing(root, locate(bare_cer)[0]):
        insert(root, bare_cer + "/cbc:Description", "UBL-ContractExecutionDescription", True, "direct")
    for scheme in ["Lot", "LotsGroup"]:
        p = f"/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='{scheme}']/cac:TenderingTerms/cac:AwardingTerms/cac:AwardingCriterion/cac:SubordinateAwardingCriterion/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:AwardCriterionParameter"
        if existing(root, locate(p)[0]):
            insert(root, p + "/efbc:ParameterCode", "UBL-AwardCriterionParameterCode", True, "direct")
    for scheme in ["Lot", "LotsGroup"]:
        p = f"/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='{scheme}']/cac:TenderingTerms/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:SelectionCriteria/efac:CriterionParameter"
        if existing(root, locate(p)[0]):
            insert(root, p + "/efbc:ParameterCode", "UBL-SelectionCriterionParameterCode", True, "direct")
    if not existing(root, locate("/*/cac:ProcurementProjectLot/cac:TenderingTerms/cac:ProcurementLegislationDocumentReference")[0]):
        for scheme in ["Lot", "Part"]:
            p = f"/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='{scheme}']/cac:TenderingTerms/cac:ProcurementLegislationDocumentReference"
            insert(root, p + "/cbc:ID", "UBL-ProcurementLegislationID", True, "direct")
            insert(root, p + "/cbc:DocumentDescription", "UBL-ProcurementLegislationDescription", True, "direct")
    if not existing(root, locate("/*/cac:ProcurementProjectLot/cac:ProcurementProject/cac:ContractExtension/cbc:RenewalsIndicator")[0]):
        insert(root, "/*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:ProcurementProject/cac:ContractExtension/cbc:RenewalsIndicator", "UBL-RenewalsIndicator", True, "direct")
    for src, tgt in ALIASES:
        for f in fields:
            if f.get("attributeOf") or f["id"].startswith("OPA-"):
                continue
            x = f["xpathAbsolute"]
            if x.startswith(src):
                insert(root, tgt + x[len(src):], f["id"], True, f"alias:{src}=>{tgt}")
        for x, fid, kind in EXTRA:
            if x.startswith(src):
                insert(root, tgt + x[len(src):], fid, True, f"alias:{src}=>{tgt}")
    return root


def leaves(root):
    out = []

    def rec(b, path):
        if b.field:
            out.append((path, b.field))
        for s, c in b.children.items():
            rec(c, path + (s,))

    rec(root, ())
    return out


def fmt(path):
    return "/" + "/".join(n + "".join(f"[{p}]" for p in sorted(ps)) for n, ps, _ in path)


def shadows(root):
    """Pairs (winner F, loser G) where every element at G's declared path that also
    satisfies F's extra predicates binds F's id instead of G's."""
    by_names = defaultdict(list)
    for path, fld in leaves(root):
        by_names[tuple(s[0] for s in path)].append((path, fld))
    res = []
    for names, group in by_names.items():
        for pf, (fid, fprov) in group:
            for pg, (gid, gprov) in group:
                if pf == pg or fid == gid:
                    continue
                # F's branch must sort before G's at the first divergent step and
                # an element matching F must also match G (G's preds ⊆ F's preds at every step)
                if not all(sg[1] <= sf[1] for sf, sg in zip(pf, pg)):
                    continue
                res.append((fid, fprov, fmt(pf), gid, gprov, fmt(pg)))
    return res


def overlaps(root):
    """Same element names, predicates not nested either way (both could match one
    element when both predicate sets hold) — listed for a manual look."""
    by_names = defaultdict(list)
    for path, fld in leaves(root):
        by_names[tuple(s[0] for s in path)].append((path, fld))
    res = []
    for names, group in by_names.items():
        for i, (pf, (fid, fprov)) in enumerate(group):
            for pg, (gid, gprov) in group[i + 1:]:
                if fid == gid:
                    continue
                sub = all(a[1] <= b[1] for a, b in zip(pg, pf)) or all(a[1] <= b[1] for a, b in zip(pf, pg))
                if sub:
                    continue
                if fprov.startswith("alias") or gprov.startswith("alias"):
                    res.append((fid, fprov, fmt(pf), gid, gprov, fmt(pg)))
    return res


report = defaultdict(set)
ov = defaultdict(set)
for ver, fn in ACCEPTED:
    fields = json.load(open(f"{ROOT}/crates/ingest/sdk/{fn}"))["fields"]
    root = build(fields, ver)
    for fid, fprov, pf, gid, gprov, pg in shadows(root):
        if fprov.startswith("alias") and not gprov.startswith("alias"):
            report[(fid, gid, fprov.split(":", 1)[1], gprov, pg)].add(ver)
    for row in overlaps(root):
        ov[row].add(ver)


def short(v):
    return v.replace("eforms-sdk-", "").replace("eforms-", "")


print(f"ALIASES={len(ALIASES)} EXTRA={len(EXTRA)}")
print("\n== alias graft SHADOWS a non-alias field at its own element ==")
rows = sorted(report.items(), key=lambda kv: (kv[0][3] != "declared", kv[0][1], kv[0][0]))
for (fid, gid, alias, gprov, pg), vers in rows:
    vs = ",".join(short(v) for v in sorted(vers, key=lambda v: [int(x) if x.isdigit() else x for x in re.split(r"[.@-]", v)]))
    src, tgt = alias.split("=>")
    print(f"- served {fid}  instead of  {gid} ({gprov})  at {pg}\n    alias {src}  =>  {tgt}\n    versions: {vs}")
print(f"\n{len(report)} shadow rows; distinct loser ids: {sorted({k[1] for k in report})}")
print("\n== non-nested predicate overlaps involving a graft (manual look) ==")
for row, vers in sorted(ov.items()):
    print(f"- {row[0]} ({row[1][:5]}) {row[2]}\n  vs {row[3]} ({row[4][:5]}) {row[5]}\n  versions: {','.join(short(v) for v in sorted(vers))}")
