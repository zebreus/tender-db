import sys, json
sys.path.insert(0, '.')
from lib577 import *
from collections import defaultdict

def get(branch, steps):
    b = branch
    for s in steps:
        b = b.children.get(s)
        if b is None: return None
    return b

def can_relax(parent, name):
    kids = [s for s in parent.children if s[0] == name]
    if any(s[2] == 0 for s in kids): return False
    # p and not(p) among siblings: one always matches
    preds = set(p for s in kids if len(s[1]) == 1 for p in s[1])
    for p in preds:
        if is_not(p) and p[4:-1] in preds: return False
    return True

results = defaultdict(set)
detail = {}
def walk(ver, root):
    de1 = ver == "eforms-de-1.x"
    # enumerate every branch node as a potential relaxed level parent
    stack = [((), root)]
    while stack:
        path, parent = stack.pop()
        for s, ch in parent.children.items():
            stack.append((path + (s,), ch))
        names = set(s[0] for s in parent.children)
        for name in names:
            if not can_relax(parent, name): continue
            order = [kv for kv in select_order(parent) if kv[0][0] == name]
            if len(order) < 2: continue
            # collect every relative path below each relaxed sibling, exact steps
            rel = defaultdict(list)  # relative steps -> [(idx, field)]
            def collect(idx, b, steps):
                if b.field: rel[steps].append((idx, b.field))
                for s2, c2 in b.children.items():
                    collect(idx, c2, steps + (s2,))
            for idx, (s, b) in enumerate(order):
                collect(idx, b, ())
            # identical relative paths (exact step equality) — the simplest consistent case
            for steps, lst in rel.items():
                if len(lst) < 2: continue
                lst.sort(key=lambda t: t[0])
                fields = [f for _, f in lst]
                b = bind(de1, fields)
                if b["id"] != fields[0]["id"]:
                    k = (fields[0]["id"], b["id"])
                    results[k].add(ver)
                    detail.setdefault(k, (fmt(path) + "/" + name + " (relaxed) " + fmt(steps), [f["id"] + ("*" if f["declared"] else "") for f in fields]))

for ver, fn in ACCEPTED:
    sdk = json.load(open(f"{ROOT}/crates/ingest/sdk/{fn}"))
    walk(ver, build(sdk, ver, True))
strict = set()
for line in open('/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/w7/moves_bind.tsv').read().splitlines()[1:]:
    o, n = line.split('\t')[:2]; strict.add((o, n))
print("relaxed-sibling moves (identical relative steps, full ordered candidate list):", len(results))
for k, v in sorted(results.items()):
    tag = "IN-STRICT" if k in strict else "NEW"
    print(f"{tag}\t{k[0]} -> {k[1]}\t{len(v)} inv\n\t{detail[k][0]}\n\tcandidates {detail[k][1]}")
