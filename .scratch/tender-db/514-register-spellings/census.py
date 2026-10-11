#!/usr/bin/env python3
"""Issue 514 unit 1: classify every standing GB org identity that spells one of 469's five
register schemes, derive a CANDIDATE canonical number per scheme, and group.

Input: the bounded range reads in raw/*.out (see unit1-census.md for every call's shape).
Output: groups.json (intermediate; splits.json is written by finish.py after mention reads).

Candidate rules (census only -- nothing here is a shipped key):
  MPR  bare 4 digits -> itself; RS + 6 digits whose significant part is <= 4 digits -> that
       bare number (RS007639 -> 7639); NNNNNR / NNNNRS -> itself; IP + NNNNNR (CH mirror) ->
       NNNNNR; IP0 + 5 digits [R] (CH padded mirror) -> NNNNNR, flagged r_inferred. NEVER bare
       <-> R. RSA..., IPI..., IP..O.. lookalikes and R-dropped forms are near-misses, unmapped.
  NIC  NIC/NI prefix or bare 1NNNNN -> 1NNNNN (label words stripped first).
  UKPRN label words (UKPRN, GBUKPRN) stripped, then 1 + 7 digits.
  CHC  label words stripped, then [2-9]NNNNN | 1NNNNNN; a trailing 1-2 digit run past a
       valid base is a LINKED candidate keyed base-N (its own group, flagged linked).
  SC   SC0 + 5 digits.
"""
import json, re, glob, os, itertools, collections

D = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(D, "raw")

SOURCES = {
    "001-mpr.out": ("GBMPR", "national"),
    "002-nic.out": ("GBNIC", "national"),
    "003-ukprn.out": ("GBUKPRN", "national"),
    "004-chc.out": ("GBCHC", "national"),
    "005-sc-national.out": ("GBSC", "national"),
    "006-sc-vat.out": ("GBSC", "vat"),
    "007-cohip.out": ("GBCOH", "national"),
    "008-cohrs.out": ("GBCOH", "national"),
    "009-bare-ip.out": ("", "national"),
    "010-bare-rs.out": ("", "national"),
    "011-vat-gbip.out": ("GB", "vat"),
    "012-vat-gbrs.out": ("GB", "vat"),
    "013-bare-ukprn.out": ("", "national"),
    "014-bare-charity.out": ("", "national"),
    "015-bare-nic.out": ("", "national"),
}
TOKEN_SCHEME = {"GBMPR": "MPR", "GBNIC": "NIC", "GBUKPRN": "UKPRN", "GBCHC": "CHC", "GBSC": "SC"}

CHC_LABELS = sorted([
    "REGISTEREDCHARITYNUMBER", "REGISTEREDCHARITYNO", "REGISTEREDCHARITY",
    "CHARITYREGISTRATIONNUMBER", "CHARITYREGISTRATIONNO", "CHARITYCOMMISSIONNO",
    "CHARITYNUMBERIS", "CHARITYNUMBER", "CHARITYREGNO", "CHARITYREG", "CHARITYNO",
    "REGISTRATIONNUMBER", "REGISTRATIONNO", "NO",
], key=len, reverse=True)
MPR_LABELS = sorted([
    "REGISTEREDSOCIETYNUMBER", "REGISTEREDSOCIETYNO", "REGISTRATIONNUMBER", "REGISTRATIONNO",
    "SOCIETYNO", "REGNO", "IPSNO", "NO",
], key=len, reverse=True)
NIC_LABELS = sorted(["CHARITYCOMMISSIONNO", "CHARITYNUMBER", "CHARITYNO", "NO"], key=len, reverse=True)
PPON_RE = re.compile(r"^(GBPPON)?P[A-Z]{3}\d{4}[A-Z]{4}$")


def strip_label(n, labels):
    for l in labels:
        if not n.startswith(l) or len(n) <= len(l):
            continue
        rest = n[len(l):]
        if rest[0].isdigit() or re.match(r"(NIC|NI|SC)\d", rest):
            return rest, l
    return n, None


def mpr_parse(n):
    """-> (canonical|None, class, flags dict)"""
    if re.fullmatch(r"[1-9]\d{3}", n):
        return n, "bare-4digit", {}
    m = re.fullmatch(r"RS(\d{6})", n)
    if m:
        sig = m.group(1).lstrip("0")
        if 1 <= len(sig) <= 4:
            return sig, "RS0-padded (CH mirror of 2014-Act)", {}
        return None, "RS0-5digit (unmapped)", {}
    if re.fullmatch(r"[1-9]\d{0,4}R", n):
        return n, "R-suffix (old Acts)", {}
    if re.fullmatch(r"[1-9]\d{2,3}RS", n):
        return n, "RS-suffix", {}
    m = re.fullmatch(r"IP([1-9]\d{0,4})R", n)
    if m:
        return m.group(1) + "R", "IP+R (CH mirror)", {}
    m = re.fullmatch(r"IP0([1-9]\d{4})R?", n)
    if m:
        return m.group(1) + "R", "IP0-padded (CH mirror, R inferred)", {"r_inferred": True}
    m = re.fullmatch(r"RSA(\d{6})", n)
    if m:
        return None, "RSA typo (unmapped)", {"near": m.group(1).lstrip("0")}
    if re.fullmatch(r"IP[A-Z0-9]*[IO][A-Z0-9]*R", n) and re.search(r"\d", n):
        fixed = n[2:].replace("I", "1").replace("O", "0")
        return None, "lookalike letter (unmapped)", {"near": fixed.lstrip("0") if fixed.endswith("R") else None}
    m = re.fullmatch(r"IPR([1-9]\d{0,4}R)", n)
    if m:
        return None, "IPR typo (unmapped)", {"near": m.group(1)}
    if re.fullmatch(r"[1-9]\d{4}", n):
        return None, "5-digit, R dropped? (unmapped)", {"near": n + "R"}
    return None, "nonconforming", {}


def nic_parse(n):
    n2, lab = strip_label(n, NIC_LABELS)
    flags = {"label": lab} if lab else {}
    for pre, cls in (("NIC", "NIC-prefix"), ("NI", "NI-prefix")):
        m = re.fullmatch(pre + r"(1\d{5})", n2)
        if m:
            return m.group(1), cls + (" +label" if lab else ""), flags
    if re.fullmatch(r"1\d{5}", n2):
        return n2, "bare" + (" +label" if lab else ""), flags
    return None, None, flags


def ukprn_parse(n):
    labs = []
    while True:
        for l in ("GBUKPRN", "UKPRN"):
            if n.startswith(l) and len(n) > len(l):
                n = n[len(l):]
                labs.append(l)
                break
        else:
            break
    flags = {"label": "+".join(labs)} if labs else {}
    if re.fullmatch(r"1\d{7}", n):
        return n, "label-polluted" if labs else "bare-8digit", flags
    if PPON_RE.fullmatch(n):
        return None, "PPON in UKPRN field", flags
    if re.fullmatch(r"10+\d{5,6}", n) and len(n) in (7, 9):
        return None, "digit-count typo (a 0 dropped/doubled; unmapped)", dict(flags, near=None)
    if re.fullmatch(r"1\d{5}", n) or re.fullmatch(r"[1-9]\d{5}", n):
        return None, "6-digit (DfE URN-like; unmapped)", flags
    if re.fullmatch(r"0\d{7}", n) or re.fullmatch(r"[2-9]\d{6}", n):
        return None, "company number (unmapped)", flags
    return None, "nonconforming", flags


def chc_base(n):
    return bool(re.fullmatch(r"[2-9]\d{5}|1\d{6}", n))


def chc_parse(n):
    """-> (canonical|None, class, flags). canonical for linked is 'base-N'."""
    flags = {}
    # compound: '...CHARITYNUMBER1165512' anywhere after a company label
    m = re.search(r"CHARITY(?:REGISTRATION)?(?:NUMBER|NO)(?:IS)?(\d{6,7})", n)
    if m and not n.startswith(("CHARITY", "REGISTERED")):
        if chc_base(m.group(1)):
            return m.group(1), "compound (company label + charity number)", {"compound": True}
    n2, lab = strip_label(n, CHC_LABELS)
    if lab:
        flags["label"] = lab
        mm = re.match(r"(\d+)(.*)", n2)
        if mm and mm.group(2):
            digits = mm.group(1)
            if chc_base(digits):
                return digits, "label + compound (charity then company)", dict(flags, compound=True)
    if chc_base(n2):
        return n2, ("label-polluted" if lab else "bare"), flags
    m = re.fullmatch(r"([2-9]\d{5})(\d{1,2})", n2) or re.fullmatch(r"(1\d{6})(\d{1,2})", n2)
    if m:
        base, suf = m.group(1), m.group(2)
        amb = "CH number missing its 0" if len(n2) == 7 and n2[0] != "1" else (
            "8-digit company number / UKPRN shape" if len(n2) == 8 else "long")
        return base + "-" + str(int(suf)), "linked-suffix candidate (glued -N)", dict(
            flags, linked=True, base=base, suffix=suf, ambiguous_with=amb)
    return None, None, flags


def sc_parse(n):
    if re.fullmatch(r"SC0\d{5}", n):
        return n, "SC-prefix", {}
    return None, None, {}


def classify(ident, prefix, kind):
    """-> dict(prefix_scheme, scheme (target), canonical, cls, flags)"""
    out = {"prefix_scheme": TOKEN_SCHEME.get(prefix, prefix or "bare"), "kind": kind}
    if kind == "vat" and prefix == "GBSC":
        n = ident[4:]
        if re.fullmatch(r"0\d{5}", n):
            return dict(out, scheme="SC", canonical="SC" + n, cls="SC dropped -> VAT-minted", flags={})
        return dict(out, scheme="SC", canonical=None, cls="VAT-minted, not OSCR-shaped (placeholder?)", flags={})
    if prefix == "GBMPR":
        n, lab = strip_label(ident[5:], MPR_LABELS)
        c, cls, fl = mpr_parse(n)
        if lab:
            fl = dict(fl, label=lab)
            cls = cls + " +label"
        return dict(out, scheme="MPR", canonical=c, cls=cls, flags=fl)
    if prefix == "GBNIC":
        n = ident[5:]
        c, cls, fl = nic_parse(n)
        if c:
            return dict(out, scheme="NIC", canonical=c, cls=cls, flags=fl)
        return cross(out, n, "NIC")
    if prefix == "GBUKPRN":
        c, cls, fl = ukprn_parse(ident[7:])
        return dict(out, scheme="UKPRN", canonical=c, cls=cls, flags=fl)
    if prefix == "GBCHC":
        n = ident[5:]
        c, cls, fl = chc_parse(n)
        if c:
            return dict(out, scheme="CHC", canonical=c, cls=cls, flags=fl)
        return cross(out, n, "CHC")
    if prefix == "GBSC":
        n = ident[4:]
        c, cls, fl = sc_parse(n)
        if c:
            return dict(out, scheme="SC", canonical=c, cls=cls, flags=fl)
        if re.fullmatch(r"SC\d{6}", n):
            return dict(out, scheme="SC", canonical=None, cls="CH Scottish company number under GB-SC (unmapped)", flags={})
        return dict(out, scheme="SC", canonical=None, cls="nonconforming", flags={})
    if prefix == "GBCOH":
        c, cls, fl = mpr_parse(ident[5:])
        return dict(out, scheme="MPR", canonical=c, cls="GB-COH " + cls, flags=fl)
    # bare national ranges
    if ident.startswith(("IP", "RS")):
        n, lab = strip_label(ident, MPR_LABELS)
        c, cls, fl = mpr_parse(n)
        if lab:
            fl = dict(fl, label=lab)
        if re.fullmatch(r"IP\d{1,3}", ident):
            cls, c = "NI IPS number (not FCA; unmapped)", None
        return dict(out, scheme="MPR", canonical=c, cls="bare " + cls, flags=fl)
    if ident.startswith("UKPRN"):
        c, cls, fl = ukprn_parse(ident)
        return dict(out, scheme="UKPRN", canonical=c, cls="bare " + cls, flags=fl)
    if ident.startswith("CHARITY"):
        n2, lab = strip_label(ident, CHC_LABELS)
        if re.fullmatch(r"SC0\d{5}", n2):
            return dict(out, scheme="SC", canonical=n2, cls="bare label + SC", flags={"label": lab})
        c, cls, fl = chc_parse(ident)
        return dict(out, scheme="CHC", canonical=c, cls="bare " + (cls or "nonconforming"), flags=fl)
    if ident.startswith("NIC"):
        c, cls, fl = nic_parse(ident)
        return dict(out, scheme="NIC", canonical=c, cls="bare " + (cls or "nonconforming"), flags=fl)
    return dict(out, scheme=None, canonical=None, cls="out of scope", flags={})


def cross(out, n, home):
    """A literal under `home`'s token that does not parse as `home`: is it another scheme's?"""
    n2, lab = strip_label(n, CHC_LABELS if home == "CHC" else NIC_LABELS)
    if home != "CHC" and chc_base(n2):
        return dict(out, scheme="CHC", canonical=n2, cls=f"CHC number under GB-{home} (cross-scheme)", flags={"cross": True})
    if home == "CHC" and re.fullmatch(r"1\d{5}", n2) and False:
        pass
    c, cls, fl = mpr_parse(n2)
    if c:
        return dict(out, scheme="MPR", canonical=c, cls=f"MPR {cls} under GB-{home} (cross-scheme)", flags=dict(fl, cross=True))
    if re.fullmatch(r"SC0\d{5}", n2):
        return dict(out, scheme="SC", canonical=n2, cls=f"OSCR number under GB-{home} (cross-scheme)", flags={"cross": True})
    if re.fullmatch(r"1\d{7}", n2):
        return dict(out, scheme=home, canonical=None, cls="8-digit 1xxxxxxx (company number / UKPRN; unmapped)", flags={})
    if re.fullmatch(r"0\d{7}", n2):
        return dict(out, scheme=home, canonical=None, cls="company number, 8 digits (unmapped)", flags={})
    if re.fullmatch(r"RC\d{6,7}", n2):
        return dict(out, scheme=home, canonical=None, cls="CH royal-charter RC number (unmapped)", flags={})
    if PPON_RE.fullmatch(n2):
        return dict(out, scheme=home, canonical=None, cls="PPON pasted (unmapped)", flags={})
    if lab and re.match(r"COMPANY", n2) is None and n2.startswith("COMPANY"):
        pass
    if n.startswith("COMPANY"):
        return dict(out, scheme=home, canonical=None, cls="company-number label (unmapped)", flags={})
    if re.fullmatch(r"\d{5}", n2) and fl.get("near"):
        return dict(out, scheme="MPR", canonical=None, cls=f"5-digit under GB-{home}: R dropped or company number (unmapped)", flags={"near": fl["near"], "cross": True})
    if re.fullmatch(r"\d+", n2):
        return dict(out, scheme=home, canonical=None, cls="nonconforming digits", flags={})
    return dict(out, scheme=home, canonical=None, cls="nonconforming", flags={})


STOP = {"limited", "ltd", "the", "plc", "cic", "cio", "llp", "lp", "co", "company", "and", "of", "t", "a",
        "ta", "trading", "as", "c", "o", "for", "itself", "on", "behalf", "group", "uk", "cyf", "cyfyngedig"}


def name_tokens(s):
    s = s.lower().replace("&", " and ").replace("'", "").replace("\u2019", "")
    s = re.sub(r"\([^)]*\)", " ", s) if len(re.sub(r"\([^)]*\)", " ", s).strip()) > 3 else s
    toks = re.findall(r"[a-z0-9]+", s)
    return {t for t in toks if t not in STOP}


def name_agree(a, b):
    A, B = name_tokens(a), name_tokens(b)
    if not A or not B:
        return "unknown"
    if A == B:
        return "same"
    if A <= B or B <= A:
        return "subset"
    j = len(A & B) / len(A | B)
    if j >= 0.34 or any(len(t) >= 5 for t in A & B):
        return "overlap"
    return "differ"


def group_agree(names):
    vals = [name_agree(a, b) for a, b in itertools.combinations(names, 2)]
    order = ["same", "subset", "overlap", "differ", "unknown"]
    return max(vals, key=order.index)


def load():
    rows = []
    for f, (prefix, kind) in SOURCES.items():
        p = os.path.join(RAW, f)
        if not os.path.exists(p):
            continue
        d = json.load(open(p))
        for oid, ident, name, prov in d["rows"]:
            c = classify(ident, prefix, kind)
            rows.append(dict(id=oid, identifier=ident, name=name, provisional=prov, source=f, **c))
    return rows


def main():
    rows = load()
    schemes = ["MPR", "NIC", "UKPRN", "CHC", "SC"]
    res = {"schemes": {}}
    for s in schemes:
        inscheme = [r for r in rows if r["prefix_scheme"] == s]
        target = [r for r in rows if r["scheme"] == s]
        classes = collections.Counter(r["cls"] for r in inscheme)
        # Level A: in-scheme literal, parses as this scheme
        def groups(rs):
            g = collections.defaultdict(list)
            for r in rs:
                if r["canonical"]:
                    g[r["canonical"]].append(r)
            return g
        ga = groups([r for r in inscheme if r["scheme"] == s])
        gb = groups(target)
        def splits(g):
            out = []
            for k, rs in g.items():
                ids = {r["id"] for r in rs}
                if len(ids) >= 2:
                    out.append(dict(canonical=k, orgs=rs, agree=group_agree([r["name"] for r in rs]),
                                    linked=any(r["flags"].get("linked") for r in rs)))
            return out
        res["schemes"][s] = dict(
            inscheme_scanned=len(inscheme),
            inscheme_parsed=sum(1 for r in inscheme if r["scheme"] == s and r["canonical"]),
            inscheme_distinct_canonical=len(ga),
            classes=dict(classes.most_common()),
            cross_prefix_rows=[r for r in target if r["prefix_scheme"] != s],
            splits_A=splits(ga),
            splits_B=splits(gb),
            unmapped=[r for r in inscheme if not r["canonical"]],
        )
    res["rows"] = rows
    json.dump(res, open(os.path.join(D, "groups.json"), "w"), indent=1)
    for s in schemes:
        x = res["schemes"][s]
        print(s, "scanned", x["inscheme_scanned"], "parsed", x["inscheme_parsed"], "splitsA", len(x["splits_A"]),
              "orgsA", sum(len({r['id'] for r in g['orgs']}) for g in x["splits_A"]),
              "splitsB", len(x["splits_B"]), "orgsB", sum(len({r['id'] for r in g['orgs']}) for g in x["splits_B"]))
        for c, n in x["classes"].items():
            print("   ", n, c)


if __name__ == "__main__":
    main()
