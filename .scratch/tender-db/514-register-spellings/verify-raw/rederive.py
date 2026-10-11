import json, re, collections, os, sys
D = "/home/user/tender-db/.scratch/tender-db/514-register-spellings"
def rows(f): return json.load(open(os.path.join(D, "raw", f)))["rows"]
src = {"MPR": ("001-mpr.out","GBMPR"), "NIC": ("002-nic.out","GBNIC"), "UKPRN": ("003-ukprn.out","GBUKPRN"),
       "CHC": ("004-chc.out","GBCHC"), "SC": ("005-sc-national.out","GBSC")}
# mention literals
lits = collections.defaultdict(list); cnt = {}
for f in sorted(os.listdir(os.path.join(D,"raw"))):
    if f.endswith(".out") and "mraw" in f:
        for oid, lit, sch, n in rows(f): lits[oid].append((lit, sch, n))
for f in ("016-mcount1.out","017-mcount2.out"):
    for oid, n in rows(f): cnt[oid] = n
def ment(oid): return cnt.get(oid, sum(n for _,_,n in lits.get(oid,[])))
# independent keys: digit runs after the token, scheme-shaped
def keys(scheme, v):
    out = set()
    if scheme == "MPR":
        # a number ending the value, with optional R/RS suffix; RS+6 digit CH mirror -> bare
        m = re.search(r"(?:^|[A-Z])RS0*(\d{1,4})$", v)
        if m and re.search(r"RS\d{6}$", v): out.add(m.group(1).lstrip("0"))
        m = re.search(r"(\d+)(R|RS)?$", v)
        if m:
            d, suf = m.group(1).lstrip("0"), m.group(2) or ""
            if not re.search(r"RS\d{6}$", v): out.add(d + suf)
    elif scheme == "UKPRN":
        for d in re.findall(r"\d+", v):
            if re.fullmatch(r"1\d{7}", d): out.add(d)
    elif scheme == "CHC":
        for d in re.findall(r"\d+", v):
            if re.fullmatch(r"[2-9]\d{5}|1\d{6}", d.lstrip("0")) : out.add(d.lstrip("0"))
    elif scheme == "NIC":
        for d in re.findall(r"\d+", v):
            if re.fullmatch(r"1\d{5}", d): out.add(d)
    elif scheme == "SC":
        for d in re.findall(r"SC(\d+)", v):
            if re.fullmatch(r"0\d{5}", d): out.add("SC"+d)
    return out
census = json.load(open(os.path.join(D,"splits.json")))
res = {}
for s,(f,tok) in src.items():
    g = collections.defaultdict(list)
    for oid, ident, name, prov in rows(f):
        for k in keys(s, ident[len(tok):]): g[k].append((oid, ident, name, ment(oid)))
    mine = {k: v for k, v in g.items() if len({x[0] for x in v}) >= 2}
    cs = {sp["canonical"]: {o["id"] for o in sp["orgs"]} for sp in census["schemes"][s]["splits"]["list"]}
    print("==", s, "mine", len(mine), "census", len(cs))
    for k, v in sorted(mine.items(), key=lambda kv: -sum(x[3] for x in kv[1])):
        ids = {x[0] for x in v}
        tag = "MATCH" if cs.get(k) == ids else ("CENSUS-DIFF" if k in cs else "NOT-IN-CENSUS")
        print(" ", tag, k, [(x[0], x[1], x[2][:40], x[3]) for x in v])
    for k, ids in cs.items():
        if k not in mine: print("  CENSUS-ONLY", k, ids)
