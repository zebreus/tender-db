import sys, json
sys.path.insert(0, '.')
from lib577 import *
from collections import defaultdict
exec(open('relaxed3.py').read().split("results = defaultdict")[0].split("from collections import defaultdict")[1])

def subsumes(c_steps, a_steps):
    """c necessarily matches wherever a matches: same length, same names, c's preds a subset of a's."""
    return len(c_steps) == len(a_steps) and all(cs[0] == as_[0] and cs[1] <= as_[1] for cs, as_ in zip(c_steps, a_steps))

def comatch_path(x, y):
    return len(x) == len(y) and all(co_match(p, q) for p, q in zip(x, y))

results = defaultdict(set); detail = {}
def walk(ver, root):
    de1 = ver == "eforms-de-1.x"
    stack = [((), root)]
    while stack:
        path, parent = stack.pop()
        for s, ch in parent.children.items():
            stack.append((path + (s,), ch))
        for name in set(s[0] for s in parent.children):
            if not can_relax(parent, name): continue
            order = [kv for kv in select_order(parent) if kv[0][0] == name]
            if len(order) < 2: continue
            per = []  # per subtree: list of (steps, field)
            for idx, (s, b) in enumerate(order):
                lst = []
                def collect(bb, steps):
                    if bb.field: lst.append((steps, bb.field))
                    for s2, c2 in bb.children.items():
                        collect(c2, steps + (s2,))
                collect(b, ())
                per.append(lst)
            for i in range(len(order)):
                for a_steps, a in per[i]:
                    if a["declared"]: continue
                    for j in range(i + 1, len(order)):
                        for b_steps, b in per[j]:
                            if not comatch_path(a_steps, b_steps): continue
                            if bind(de1, [a, b])["id"] == a["id"]: continue
                            guarded = any(c["declared"] and subsumes(c_steps, a_steps)
                                          for k in range(i) for c_steps, c in per[k])
                            # a declared field in a's own subtree at a path a co-matches that sorts AHEAD of a
                            k = (a["id"], b["id"], guarded)
                            results[k].add(ver)
                            detail.setdefault(k, f"{fmt(path)}/{name}(relaxed: {[sorted(o[0][1]) for o in order]}) a@{fmt(a_steps)} b@{fmt(b_steps)}")

for ver, fn in ACCEPTED:
    sdk = json.load(open(f"{ROOT}/crates/ingest/sdk/{fn}"))
    walk(ver, build(sdk, ver, True))
strict = set()
for line in open('/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/w7/moves_bind.tsv').read().splitlines()[1:]:
    o, n = line.split('\t')[:2]; strict.add((o, n))
unguarded = {(a,b):v for (a,b,g),v in results.items() if not g}
guarded = {(a,b):v for (a,b,g),v in results.items() if g}
print("pairs:", len(results), "unguarded:", len(unguarded), "guarded-only:", len(set(guarded)-set(unguarded)))
for (a,b),v in sorted(unguarded.items()):
    print(("IN-STRICT" if (a,b) in strict else "NEW"), a, "->", b, len(v), "inv:", ",".join(sorted(v))[:120])
    print("   ", detail[(a,b,False)][:600])
