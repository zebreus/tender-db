import sys, json
sys.path.insert(0, '.')
from lib577 import *
from collections import defaultdict

def co_bound_relaxed(branch, out, depth_relaxed=False):
    """Pairs of fields one element can carry when some ANCESTOR level was a relaxed claim
    (select_by_name: name equality only). The element's own level must still co_match."""
    def down(first, other, exact, via_relaxed):
        (sa, a), (sb, b) = first, other
        if exact and a.field and b.field:
            out.append((a.field, b.field, via_relaxed))
        for ca in a.children.items():
            for cb in b.children.items():
                if ca[0][0] == cb[0][0]:
                    cm = co_match(ca[0], cb[0])
                    down(ca, cb, cm, via_relaxed or not exact)
    order = select_order(branch)
    for i, first in enumerate(order):
        for other in order[i + 1:]:
            if first[0][0] == other[0][0]:
                down(first, other, co_match(first[0], other[0]), False)
        co_bound_relaxed(first[1], out)

res = defaultdict(set)
for ver, fn in ACCEPTED:
    sdk = json.load(open(f"{ROOT}/crates/ingest/sdk/{fn}"))
    new = build(sdk, ver, True)
    pairs = []
    co_bound_relaxed(new, pairs)
    for f, d, via in pairs:
        b = bind(ver == "eforms-de-1.x", [f, d])
        if b["id"] != f["id"]:
            res[(f["id"], d["id"], via)].add(ver)
strict = set()
for line in open('/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/w7/moves_bind.tsv').read().splitlines()[1:]:
    o, n = line.split('\t')[:2]; strict.add((o, n))
new_only = {(o,n): v for (o,n,via), v in res.items() if (o,n) not in strict}
print("relaxed-ancestor moves total distinct (old,new):", len({(o,n) for (o,n,_) in res}))
print("NOT in analyst's strict set:", len(new_only))
for (o,n), v in sorted(new_only.items()):
    print(f"{o}\t{n}\t{','.join(sorted(v))}")
