"""Issue 546 unit 1: classify each legacy notice's mapped `place` rows (project.rs CLASSIFICATIONS) as
performance vs buyer/winner/other-address, and what the address rows add to `?country=` matching.
Inputs: a_<w>.out.json (notice_classifications nuts rows), c_<w>.out.json (ORG pointer rows, w19/w17 only),
b_<w>.out.json (denominators). Capped windows: the last notice id is dropped (may be partial)."""
import json, collections, sys
MAPPED = {"TED-NUTS","TED-PERFORMANCE_NUTS","TED-ORIGINAL_NUTS","TED-CA_CE_NUTS","TED-CURRENT_NUTS","TED-TENDERER_NUTS"}
PERF_FIELDS = {"TED-PERFORMANCE_NUTS","TED-ORIGINAL_NUTS","TED-CURRENT_NUTS"}
BUYER_PTR = {"TED-ADDRESS_CONTRACTING_BODY","TED-ADDRESS_CONTRACTING_BODY_ADDITIONAL","TED-CA_CE_CONCESSIONAIRE_PROFILE"}
WINNER_PTR = {"TED-ADDRESS_CONTRACTOR","TED-ADDRESS_WINNER","TED-WINNER","TED-NAME_ADDRESS_WINNER","TED-ECONOMIC_OPERATOR_NAME_ADDRESS"}
def kind(sec, fid, roles):
    if fid in PERF_FIELDS: return "perf"
    if fid == "TED-CA_CE_NUTS": return "buyer"
    if fid == "TED-TENDERER_NUTS": return "winner"
    if sec.startswith("ORG-"):
        r = roles.get(sec)
        if r is None: return "org?"
        return r
    return "perf"   # TED-NUTS in LOT-n / PROCEDURE: OBJECT_DESCR / LOCATION_NUTS place of performance
def new_prefix_len(c, perf):
    """Shortest prefix of c that no perf code starts with (None: c adds no ?country= match)."""
    for L in range(2, len(c)+1):
        if not any(d.startswith(c[:L]) for d in perf): return L
    return None
out = {}
for w in sys.argv[1:]:
    a = json.load(open(f"a_{w}.out.json")); rows = a["rows"]
    capped = a["row_count"] >= 10000
    last = max(r[0] for r in rows)
    if capped: rows = [r for r in rows if r[0] != last]
    roles = collections.defaultdict(dict)
    try:
        for nid, sec, fid, val, isref in json.load(open(f"c_{w}.out.json"))["rows"]:
            if not isref: continue
            role = "buyer" if fid in BUYER_PTR else "winner" if fid in WINNER_PTR else "other-addr"
            if fid.startswith("TED-DESCRIPTION_PROCUREMENT."): role = "winner-mod"
            roles[nid][val] = role
        have_roles = True
    except FileNotFoundError:
        have_roles = False
    den = sum(r[1] for r in json.load(open(f"b_{w}.out.json"))["rows"])
    per = collections.defaultdict(list)
    unmapped = collections.Counter()
    for nid, sec, fid, code in rows:
        if fid not in MAPPED: unmapped[(sec.split("-")[0], fid)] += 1; continue
        per[nid].append((sec, fid, code))
    cls = collections.Counter(); newlen = collections.Counter(); kinds_rows = collections.Counter()
    added_by = collections.Counter(); examples = collections.defaultdict(list); served_growth = []
    for nid, rs in per.items():
        if have_roles:
            ks = [(kind(s, f, roles[nid]), c) for s, f, c in rs]
        else:
            ks = [(kind(s, f, {}) if not s.startswith("ORG-") else "org(no-role-read)", c) for s, f, c in rs]
        for k, _ in ks: kinds_rows[k] += 1
        perf = {c for k, c in ks if k == "perf"}
        oth = {(k, c) for k, c in ks if k != "perf"}
        if not oth: cls["perf-only"] += 1; continue
        if not perf:
            cls["address-only"] += 1; examples["address-only"].append(nid); continue
        lens = {}
        for k, c in oth:
            L = new_prefix_len(c, perf)
            if L is not None:
                lens[(k, c)] = L
        if not lens: cls["both-harmless"] += 1; continue
        m = min(lens.values())
        cls["both-adds"] += 1
        newlen["country(2)" if m == 2 else f"len{m}"] += 1
        for k in {k for k, _ in lens}: added_by[k] += 1
        served = perf | {c for _, c in oth}
        served_growth.append(len(served) - len(perf))
        if len(examples[f"adds-L{m}"]) < 6: examples[f"adds-L{m}"].append(nid)
    n_with = len(per)
    out[w] = dict(capped=capped, id_lo=min(r[0] for r in rows), id_hi=max(r[0] for r in rows), notices_in_range=den,
                  notices_with_mapped_place=n_with, no_mapped_place=den - n_with, classes=dict(cls),
                  adds_by_shortest_new_prefix=dict(newlen), adds_contributed_by=dict(added_by),
                  row_kinds=dict(kinds_rows), unmapped_nuts_rows={f"{k[0]} {k[1]}": v for k, v in unmapped.items()},
                  mean_codes_added=(sum(served_growth)/len(served_growth) if served_growth else 0),
                  examples={k: v[:6] for k, v in examples.items()})
print(json.dumps(out, indent=1))
