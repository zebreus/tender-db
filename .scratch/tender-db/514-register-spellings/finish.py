#!/usr/bin/env python3
"""Issue 514 unit 1: join groups (census.py) with the mention reads and write splits.json."""
import json, os, re, collections, sys

D = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, D)
import census  # noqa: E402

g = json.load(open(os.path.join(D, "groups.json")))
rows = g["rows"]
byid = {r["id"]: r for r in rows}

# ---- mention reads ---------------------------------------------------------------------
raw = collections.defaultdict(list)          # org -> [(literal, scheme, n)]
for f in sorted(os.listdir(os.path.join(D, "raw"))):
    if "mraw" in f and f.endswith(".out"):
        for oid, lit, sch, n in json.load(open(os.path.join(D, "raw", f)))["rows"]:
            raw[oid].append((lit, sch, n))
count = {}
for f in ("016-mcount1.out", "017-mcount2.out"):
    for oid, n in json.load(open(os.path.join(D, "raw", f)))["rows"]:
        count[oid] = n
for oid, ls in raw.items():
    count.setdefault(oid, sum(n for _, _, n in ls))
read_ids = set(raw) | set(count)


# A stored GBCHC value past a valid base is only a linked charity when the PUBLISHED literal shows
# the glued suffix (`1099485-2`); without one it is a company number missing its 0 or an 8-digit
# company number / UKPRN (by name, every one read). Re-label from the mention literals.
for r in rows:
    if r["flags"].get("linked"):
        hy = any(re.search(r"\d\s*[-/]\s*\d{1,2}\s*$", l or "") for l, _, _ in raw.get(r["id"], []))
        if hy:
            r["cls"] = "linked suffix (literal shows glued -N)"
        else:
            r["cls"] = "glued digits, no suffix in literal: CH number missing 0 / company no. / UKPRN (unmapped)"
            r["canonical"] = None
            r["flags"] = dict(r["flags"], linked=False)
for s, x in g["schemes"].items():
    x["classes"] = dict(collections.Counter(r["cls"] for r in rows if r["prefix_scheme"] == s).most_common())
    x["unmapped"] = [r for r in rows if r["prefix_scheme"] == s and not r["canonical"]]


def mentions(oid):
    return count.get(oid, 0) if oid in read_ids else None


TOKENS = ["GBUKPRN", "GBMPR", "GBNIC", "GBCHC", "GBSC", "GBCOH"]


def classify_literal(lit):
    v = "".join(c for c in (lit or "") if c.isascii() and c.isalnum()).upper()
    pre = next((t for t in TOKENS if v.startswith(t)), "")
    return v, census.classify(v, pre, "national")


# ---- manual name verdicts for the in-scheme splits (read by hand, 2026-10-11) ---------------
MANUAL = {
    ("MPR", "30938R"): ("same", "One Housing is a Riverside brand"),
    ("MPR", "17437R"): ("same", "B2G is a framework tag"),
    ("UKPRN", "10007157"): ("differ", "UKPRN 10007157 is The University of Sheffield; the bare org's head name is Sheffield Hallam University (10005790) -- a mis-named holder, the name gate must refuse"),
    ("UKPRN", "10007788"): ("same", "the University of Cambridge's statutory name"),
    ("CHC", "1088835"): ("same", "Safe Haven is a No Limits service"),
    ("CHC", "1179472"): ("partial", "Living Well Consortium vs Living Well UK: unconfirmed rename"),
    ("CHC", "208078"): ("same", "Cruse Bereavement Care renamed Cruse Bereavement Support (2021)"),
    ("CHC", "271028"): ("same", "Redwoods is a Rethink service"),
    ("CHC", "216250"): ("same", "Barnardo's / Barnardos"),
}
MANUAL_B = {
    ("MPR", "10457R"): "same (Cambridge Housing Society renamed CHS Group)",
    ("MPR", "14044R"): "differ (IDS Trustees Limited, CHC-labelled IP14044R, beside the Industrial Dwellings Society)",
    ("MPR", "28038R"): "same (Affinity Sutton Group renamed Clarion Housing Group, 2016)",
    ("MPR", "30224R"): "differ (Homes Plus Limited is a Housing Plus Group subsidiary on the parent's number)",
    ("MPR", "31693R"): "differ (Guinness Developments Limited on The Guinness Partnership's IP031693)",
    ("MPR", "27566R"): "same (NDTi is the acronym)",
    ("MPR", "7448"): "same (Sovereign Housing Association -> Sovereign Network Group)",
    ("UKPRN", "10007455"): "same (The WKCIC Group renamed Capital City College Group)",
    ("CHC", "207076"): "same registered charity (RSPB; 'RSPB Northern Ireland' is a branch name)",
}


def org_view(r):
    oid = r["id"]
    lits = sorted(raw.get(oid, []), key=lambda x: -x[2])
    return dict(id=oid, identifier=r["identifier"], name=r["name"], provisional=r["provisional"],
                spelling_class=r["cls"], prefix=r["prefix_scheme"], flags=r["flags"],
                mentions=mentions(oid), raw_literals=[{"literal": l, "scheme": s, "n": n} for l, s, n in lits])


def split_view(s, sp, manual=None):
    orgs = [org_view(byid[r["id"]]) for r in sp["orgs"]]
    m = sum(o["mentions"] or 0 for o in orgs)
    mv = (manual or {}).get((s, sp["canonical"]))
    return dict(canonical=sp["canonical"], org_count=len(orgs), mentions=m,
                classes=sorted({o["spelling_class"] for o in orgs}),
                names_agree_auto=sp["agree"],
                names_agree=(mv[0] if isinstance(mv, tuple) else None) or (
                    "same" if sp["agree"] in ("same", "subset") else sp["agree"]),
                name_note=(mv[1] if isinstance(mv, tuple) else mv),
                linked=sp["linked"], orgs=orgs)


out = {
    "issue": 514, "unit": 1, "date": "2026-10-11",
    "source": "prod /v1/sql via /root/sq.sh, bounded index-served reads; see unit1-census.md",
    "note": "provisional is 0 on every org read; organization_merged_identifiers is NOT on /v1/sql's "
            "allow-list (call 20, HTTP 400), so merged spellings are proxied by each org's mention "
            "raw literals.",
    "schemes": {},
}

for s, x in g["schemes"].items():
    A = [split_view(s, sp, MANUAL) for sp in x["splits_A"]]
    A.sort(key=lambda v: -v["mentions"])
    keysA = {(v["canonical"], v["org_count"]) for v in A}
    B = [split_view(s, sp, MANUAL_B) for sp in x["splits_B"]]
    for v in B:
        mv = MANUAL_B.get((s, v["canonical"]))
        if mv:
            v["names_agree"] = mv.split(" ")[0]
        elif (s, v["canonical"]) in MANUAL:
            v["names_agree"], v["name_note"] = MANUAL[(s, v["canonical"])]
    B.sort(key=lambda v: -v["mentions"])
    Bonly = [v for v in B if (v["canonical"], v["org_count"]) not in keysA]
    inscheme = [r for r in rows if r["prefix_scheme"] == s]
    insc_ids = {r["id"] for r in inscheme}
    insc_mentions = sum(mentions(i) or 0 for i in insc_ids)
    # absorbed spellings / foreign numbers seen on each in-scheme org's mentions
    absorbed, foreign_same_scheme = [], []
    for r in [r for r in rows if r["scheme"] == s]:
        if not r["canonical"]:
            continue
        for lit, sch, n in raw.get(r["id"], []):
            v, c = classify_literal(lit)
            if c.get("scheme") != s or not c.get("canonical"):
                continue
            if c["canonical"] == r["canonical"] and v != r["identifier"]:
                absorbed.append(dict(org=r["id"], identifier=r["identifier"], literal=lit, n=n))
            elif c["canonical"] != r["canonical"]:
                foreign_same_scheme.append(dict(org=r["id"], identifier=r["identifier"], name=r["name"],
                                                literal=lit, n=n, literal_canonical=c["canonical"]))
    # linked: a GB-CHC literal with a glued -N past a valid base
    linked = []
    if s == "CHC":
        for r in inscheme:
            for lit, sch, n in raw.get(r["id"], []):
                m = re.search(r"(\d{6,7})\s*[-/]\s*(\d{1,2})\s*$", lit or "")
                if m and census.chc_base(m.group(1)):
                    base_orgs = [o for o in rows if o["prefix_scheme"] == "CHC" and o["canonical"] == m.group(1)]
                    linked.append(dict(org=r["id"], identifier=r["identifier"], name=r["name"], literal=lit,
                                       mentions=n, candidate=f"{m.group(1)}-{int(m.group(2))}",
                                       base_org_standing=[(o["id"], o["identifier"], o["name"]) for o in base_orgs]))
    # bare vs R guard: same digits as bare and R-suffixed -- never one identity
    guard = []
    if s == "MPR":
        mpr = [r for r in rows if r["scheme"] == "MPR" and r["canonical"]]
        bare = {r["canonical"]: r for r in mpr if r["canonical"].isdigit()}
        for r in mpr:
            if r["canonical"].endswith("R") and r["canonical"][:-1] in bare:
                b = bare[r["canonical"][:-1]]
                guard.append(dict(bare=(b["id"], b["identifier"], b["name"]), r=(r["id"], r["identifier"], r["name"])))
    out["schemes"][s] = dict(
        identities_scanned=len(inscheme),
        identities_with_candidate=sum(1 for r in inscheme if r["scheme"] == s and r["canonical"]),
        distinct_candidates=len({r["canonical"] for r in inscheme if r["scheme"] == s and r["canonical"]}),
        mentions_on_scanned=insc_mentions,
        spelling_classes=x["classes"],
        splits=dict(count=len(A), org_count=sum(v["org_count"] for v in A),
                    mention_count=sum(v["mentions"] for v in A),
                    names_not_confirmed_same=[v["canonical"] for v in A if v["names_agree"] in ("differ", "partial", "overlap")],
                    names_summary=dict(collections.Counter(v["names_agree"] for v in A)),
                    shapes=dict(collections.Counter(" + ".join(v["classes"]) for v in A).most_common()),
                    list=A),
        cross_prefix=dict(
            note="Level B: the same candidate number under another prefix -- GB-COH mirrors (IP/RS), legacy "
                 "bare literals (non-FTS publishers), or another scheme's token. Reported, not part of the "
                 "in-scheme split count.",
            rows_added=len(x["cross_prefix_rows"]),
            classes=dict(collections.Counter(r["cls"] for r in x["cross_prefix_rows"]).most_common()),
            splits_count=len(B), org_count=sum(v["org_count"] for v in B),
            mention_count=sum(v["mentions"] for v in B),
            new_or_larger_than_in_scheme=len(Bonly),
            names_summary=dict(collections.Counter(v["names_agree"] for v in B)),
            prefix_shapes=dict(collections.Counter(" + ".join(sorted({o["prefix"] for o in v["orgs"]})) for v in B).most_common()),
            list=B),
        absorbed_spellings=absorbed,
        foreign_same_scheme_literals=foreign_same_scheme,
        linked_suffix_literals=linked,
        bare_vs_R_guard=guard,
        unmapped=[dict(id=r["id"], identifier=r["identifier"], name=r["name"], cls=r["cls"],
                       mentions=mentions(r["id"]), flags=r["flags"]) for r in x["unmapped"]],
    )

# ---- near misses (typos the rules refuse), with the counterpart they resemble -----------------
near = []
def holders(scheme, canon):
    return [(o["id"], o["identifier"], o["name"], mentions(o["id"])) for o in rows
            if o["scheme"] == scheme and o["canonical"] == canon]
for r in rows:
    f = r["flags"]
    if f.get("near"):
        near.append(dict(id=r["id"], identifier=r["identifier"], name=r["name"], cls=r["cls"],
                         mentions=mentions(r["id"]), resembles=f["near"],
                         resembled_holders=holders("MPR", f["near"])))
    if r["cls"].startswith("digit-count typo"):
        n = r["identifier"][7:]
        cands = set()
        for i in range(1, len(n)):
            if len(n) == 9 and n[i] == "0":
                cands.add(n[:i] + n[i + 1:])
            if len(n) == 7:
                cands.add(n[:i] + "0" + n[i:])
        cands = {c for c in cands if re.fullmatch(r"1\d{7}", c)}
        hs = [h for c in sorted(cands) for h in holders("UKPRN", c)]
        near.append(dict(id=r["id"], identifier=r["identifier"], name=r["name"], cls=r["cls"],
                         mentions=mentions(r["id"]), resembles=sorted(cands), resembled_holders=hs))
hastoe = byid.get(31563025)
if hastoe:
    near.append(dict(id=31563025, identifier=hastoe["identifier"], name=hastoe["name"],
                     cls="IP+R typo (issue 514's ip6243r; maps to 6243R under the rule, which is NOT Hastoe)",
                     mentions=mentions(31563025), resembles="16243R", resembled_holders=holders("MPR", "16243R")))
out["near_misses"] = near

# ---- cross-scheme checks: CHC-labelled 8-digit 1xxxxxxx that are UKPRNs ----------------------
ukprns = {o["canonical"]: o for o in rows if o["scheme"] == "UKPRN" and o["canonical"]}
xs = []
for r in rows:
    if r["prefix_scheme"] == "CHC" and re.fullmatch(r"GBCHC1\d{7}", r["identifier"]):
        u = ukprns.get(r["identifier"][5:])
        xs.append(dict(id=r["id"], identifier=r["identifier"], name=r["name"],
                       ukprn_holder=(u["id"], u["identifier"], u["name"]) if u else None))
out["chc_8digit_check"] = xs

json.dump(out, open(os.path.join(D, "splits.json"), "w"), indent=1, ensure_ascii=False)

# ---- console summary -----------------------------------------------------------------------
print(f"{'scheme':6} {'scanned':>7} {'cand':>5} {'ment':>6} {'splits':>6} {'orgs':>5} {'ment':>5} | {'B-spl':>5} {'B-org':>5} {'B-men':>5}")
for s, v in out["schemes"].items():
    sp, cp = v["splits"], v["cross_prefix"]
    print(f"{s:6} {v['identities_scanned']:>7} {v['identities_with_candidate']:>5} {v['mentions_on_scanned']:>6} "
          f"{sp['count']:>6} {sp['org_count']:>5} {sp['mention_count']:>5} | {cp['splits_count']:>5} {cp['org_count']:>5} {cp['mention_count']:>5}")
    print("   not-same:", sp["names_not_confirmed_same"], sp["names_summary"], sp["shapes"], cp["prefix_shapes"], cp["names_summary"], "absorbed:", len(v["absorbed_spellings"]),
          "foreign:", len(v["foreign_same_scheme_literals"]), "linked:", len(v["linked_suffix_literals"]),
          "guard:", len(v["bare_vs_R_guard"]))
