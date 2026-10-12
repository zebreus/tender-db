#!/usr/bin/env python3
"""Issue 514 U2.1, step 1: materialise the decision's §1.3 rewrite over the unit-1 census rows.

Re-implements unit2-decision.md §1.3 (the five rules) and §1.2 I1 (fails the REGISTRIES shape
on input, passes on output; crosswalk.rs:1208-1227), applies them to every row of the four
in-scheme census ranges (raw/001-mpr.out, 002-nic.out, 003-ukprn.out, 004-chc.out: unit-1 calls
1-4), and groups the post-rewrite identifiers into exact triples (GB, national, value), which is
what E0's `e0_key_flat` groups on.

- E0 group: a post-rewrite value held by two or more orgs, at least one of which moved.
- Lone mover: a moved row whose post-rewrite value no other org holds.

Mentions come from the unit-1 mention reads: raw/016-017 (COUNT per org, split ids) and the
raw_identifier breakdowns raw/018-019, 021-029 (every in-scheme org). Census call numbers are
cited per row.

Usage: python3 -I materialise.py      (paths are relative to this script)
Writes groups.json next to this script.
"""
import collections
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
U1 = os.path.dirname(HERE)
RAW = os.path.join(U1, "raw")

RANGES = {  # census file -> (call, token)
    "001-mpr.out": (1, "GBMPR"),
    "002-nic.out": (2, "GBNIC"),
    "003-ukprn.out": (3, "GBUKPRN"),
    "004-chc.out": (4, "GBCHC"),
}
SCHEME = {"GBMPR": "MPR", "GBNIC": "NIC", "GBUKPRN": "UKPRN", "GBCHC": "CHC"}


# ---- REGISTRIES shapes (crosswalk.rs:1208-1227), the number after the token ---------------------
def _lead(d, lo, hi):
    return lo <= len(d) <= hi and not d.startswith("0") and d.isdigit()


def shaped(token, n):
    if token == "GBUKPRN":
        return len(n) == 8 and n.startswith("1") and n.isdigit()
    if token == "GBCHC":
        return n.isdigit() and ((len(n) == 6 and n[0] >= "2") or (len(n) == 7 and n.startswith("1")))
    if token == "GBMPR":
        return (_lead(n, 4, 4)
                or (n.endswith("RS") and _lead(n[:-2], 3, 4))
                or (n.endswith("R") and _lead(n[:-1], 3, 5))
                or (n.startswith("RS") and len(n) == 8 and n[2:].isdigit()))
    if token == "GBNIC":
        d = n[3:] if n.startswith("NIC") else n[2:] if n.startswith("NI") else n
        return len(d) == 6 and d.startswith("1") and d.isdigit()
    raise ValueError(token)


# ---- §1.3 rules -----------------------------------------------------------------------------------
CHC_LABELS = sorted([
    "REGISTEREDCHARITYNUMBER", "REGISTEREDCHARITYNO", "REGISTEREDCHARITY",
    "CHARITYREGISTRATIONNUMBER", "CHARITYREGISTRATIONNO", "CHARITYCOMMISSIONNO",
    "CHARITYREGNO", "CHARITYNUMBER", "CHARITYNO",
], key=len, reverse=True)
RULES = [
    ("UKPRN", re.compile(r"^GBUKPRN(?:(?:GB)?UKPRN)+(1\d{7})$"), lambda m: "GBUKPRN" + m.group(1)),
    ("NIC", re.compile(r"^GBNIC(CHARITYCOMMISSIONNO|CHARITYNUMBER|CHARITYNO)((?:NIC|NI)?1\d{5})$"),
     lambda m: "GBNIC" + m.group(2)),
    ("MPR-a", re.compile(r"^GBMPRIP([1-9]\d{4}R)$"), lambda m: "GBMPR" + m.group(1)),
    ("MPR-b", re.compile(r"^GBMPRREGNO([1-9]\d{2,4}R)$"), lambda m: "GBMPR" + m.group(1)),
]
CHC_RE = re.compile(r"^GBCHC(" + "|".join(CHC_LABELS) + r")(\d+)$")  # alternation is longest first
HOLDS = frozenset()  # REGISTER_SPELLING_HOLDS: empty until the review fixes it (§1.2 I4)


def token_of(v):
    return next(t for t in ("GBUKPRN", "GBMPR", "GBNIC", "GBCHC") if v.startswith(t))


def register_spelling(v):
    """-> (rule, output) or None. `v` is the stored (upper-cased alphanumeric) value."""
    hit = None
    for name, rx, out in RULES:
        m = rx.match(v)
        if m:
            hit = (name, out(m))
            break
    if hit is None:
        m = CHC_RE.match(v)
        if m and shaped("GBCHC", m.group(2)):
            hit = ("CHC", "GBCHC" + m.group(2))
    if hit is None:
        return None
    rule, o = hit
    tok = token_of(v)
    # I1: fails the shape on input, passes on output; I2: keeps the token
    assert not shaped(tok, v[len(tok):]), (v, "input already shaped")
    assert o.startswith(tok) and shaped(tok, o[len(tok):]), (v, o, "output not shaped")
    # I3: no digit added/removed/changed, no R / RS added or removed
    dig = lambda s: re.sub(r"\D", "", s)
    assert dig(o) == dig(v[len(tok):]) or dig(o) in dig(v), (v, o)
    assert o.endswith("R") == v.endswith("R") and o.endswith("RS") == v.endswith("RS"), (v, o)
    if o in HOLDS:  # I4
        return None
    return rule, o


# ---- inputs -----------------------------------------------------------------------------------------
def load_rows():
    rows = []
    for f, (call, tok) in RANGES.items():
        d = json.load(open(os.path.join(RAW, f)))
        assert d["columns"] == ["id", "identifier", "name", "provisional"], f
        for oid, ident, name, prov in d["rows"]:
            assert ident.startswith(tok)
            rows.append(dict(id=oid, identifier=ident, name=name, provisional=prov, census_call=call,
                             census_file="raw/" + f, token=tok, scheme=SCHEME[tok]))
    return rows


def load_mentions():
    raw, raw_src = collections.defaultdict(list), {}
    for f in sorted(os.listdir(RAW)):
        if "mraw" in f and f.endswith(".out"):
            for oid, lit, sch, n in json.load(open(os.path.join(RAW, f)))["rows"]:
                raw[oid].append(dict(raw_identifier=lit, scheme=sch, n=n))
                raw_src.setdefault(oid, "raw/" + f)
    count, count_src = {}, {}
    for f in ("016-mcount1.out", "017-mcount2.out"):
        for oid, n in json.load(open(os.path.join(RAW, f)))["rows"]:
            count[oid], count_src[oid] = n, "raw/" + f
    return raw, raw_src, count, count_src


def main():
    rows = load_rows()
    raw, raw_src, count, count_src = load_mentions()
    for r in rows:
        hit = register_spelling(r["identifier"])
        r["rule"], r["canonical"] = (hit if hit else (None, r["identifier"]))
        r["moves"] = hit is not None
        lits = raw.get(r["id"], [])
        r["mentions"] = count.get(r["id"], sum(x["n"] for x in lits))
        r["mentions_source"] = count_src.get(r["id"]) or raw_src.get(r["id"])
        r["mention_raws"] = sorted(lits, key=lambda x: -x["n"])
        r["mention_raws_source"] = raw_src.get(r["id"])
        if r["id"] in count and lits:
            assert count[r["id"]] == sum(x["n"] for x in lits), r["id"]

    by_canon = collections.defaultdict(list)
    for r in rows:
        by_canon[r["canonical"]].append(r)

    def member(r):
        return dict(org_id=r["id"], stored_identifier=r["identifier"], head_name=r["name"],
                    provisional=r["provisional"], moves=r["moves"], rule=r["rule"], mentions=r["mentions"],
                    mention_raws=r["mention_raws"],
                    census=dict(row=f'{r["census_file"]} (call {r["census_call"]})',
                                mentions=r["mentions_source"], raws=r["mention_raws_source"]))

    groups, movers = [], []
    for canon, rs in sorted(by_canon.items()):
        if not any(r["moves"] for r in rs):
            continue
        ms = sorted(rs, key=lambda r: r["id"])
        tok = token_of(canon)
        ent = dict(canonical=canon, scheme=SCHEME[tok], number=canon[len(tok):],
                   rules=sorted({r["rule"] for r in ms if r["rule"]}),
                   members=[member(r) for r in ms], mentions=sum(r["mentions"] for r in ms))
        if len(ms) >= 2:
            ent["kind"] = "reunion onto a standing twin" if any(not r["moves"] for r in ms) else "movers reunite"
            ent["survivor_if_merged"] = ms[0]["id"]  # E0 keeps the lower id (canonical.rs:21521-21525)
            groups.append(ent)
        else:
            ent["kind"] = "lone mover"
            movers.append(ent)

    moved = [r for r in rows if r["moves"]]
    by_rule = collections.Counter(r["rule"] for r in moved)
    summary = dict(
        rows_scanned={f: sum(1 for r in rows if r["census_file"] == "raw/" + f) for f in RANGES},
        rows_rewritten=len(moved),
        rows_rewritten_by_scheme=dict(collections.Counter(r["scheme"] for r in moved)),
        rows_rewritten_by_rule=dict(by_rule),
        movers_in_groups=sum(1 for g in groups for m in g["members"] if m["moves"]),
        e0_groups=len(groups),
        e0_groups_reunion_onto_twin=sum(1 for g in groups if g["kind"].startswith("reunion")),
        e0_groups_movers_reunite=sum(1 for g in groups if g["kind"] == "movers reunite"),
        group_members=sum(len(g["members"]) for g in groups),
        group_sizes=dict(collections.Counter(len(g["members"]) for g in groups)),
        group_mentions=sum(g["mentions"] for g in groups),
        lone_movers=len(movers),
        lone_movers_by_rule=dict(collections.Counter(m["rules"][0] for m in movers)),
        lone_mover_mentions=sum(m["mentions"] for m in movers),
        orgs_to_review=sum(len(g["members"]) for g in groups) + len(movers),
    )
    out = dict(
        issue=514, unit="U2.1 review packet, step 1", date="2026-10-12",
        source=("unit-1 census rows raw/001-004 (calls 1-4) for identifiers and head names; raw/016-017 "
                "(calls 16-17) and raw/018-019, 021-029 (calls 18-19, 21-29) for mention counts and raw "
                "identifiers; rules from unit2-decision.md §1.3, invariants §1.2"),
        summary=summary, groups=groups, lone_movers=movers)
    json.dump(out, open(os.path.join(HERE, "groups.json"), "w"), indent=1, ensure_ascii=False)
    print(json.dumps(summary, indent=1))
    for g in groups:
        print("G", g["canonical"], g["kind"], [(m["org_id"], m["stored_identifier"], m["mentions"]) for m in g["members"]])
    for g in movers:
        m = g["members"][0]
        print("L", g["canonical"], m["org_id"], m["stored_identifier"], m["mentions"], m["head_name"])


if __name__ == "__main__":
    main()
