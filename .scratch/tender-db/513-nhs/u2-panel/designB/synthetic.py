# Synthetic strays: every SAME registry holder's name evaluated under every OTHER
# allowlisted code that (a) shares a distinctive token with it ("name neighbour") or
# (b) sits next to its true code in sort order ("dropdown one row off"). The gate
# should refuse all of them.
import sys, os, collections, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gate_b as G, gate_b2 as G2, gate_b3 as G3
reg = [r for r in G.reg if r['final'] == 'SAME' and r['code'] in G.table]
allow_sorted = sorted(G.allow)
base = G.CFGS['G3 +allowlist +O/0 fold (DESIGN B)']
def related(c, d): return d in G.preds.get(c, []) or c in G.preds.get(d, [])
out = {k: collections.Counter() for k in ('B', 'B2')}
admitted_examples = collections.defaultdict(list)
for r in reg:
    c = r['code']
    D, _, _ = G2.split(r['name'])
    i = allow_sorted.index(c) if c in allow_sorted else None
    adj = set()
    if i is not None:
        adj = {allow_sorted[k] for k in (i - 1, i + 1) if 0 <= k < len(allow_sorted)}
    nb = {d for d in G.allow if d != c and (D & G2.split(G.table[d]['name'])[0])}
    for d in (nb | adj):
        if d == c or related(c, d): continue
        kind = 'name-neighbour' if d in nb else 'sort-neighbour'
        okB = G.gate(r['name'], d, base)[0]
        okB2 = G3.gate3(r["name"], d)[0]
        for lab, ok in (('B', okB), ('B2', okB2)):
            o = out[lab]
            o[kind + ' pairs'] += 1; o[kind + ' mentions'] += r['coded']
            if ok:
                o[kind + ' ADMITTED pairs'] += 1; o[kind + ' ADMITTED mentions'] += r['coded']
                admitted_examples[lab].append((kind, c, r['name'][:48], '->', d, G.table[d]['name'][:48], r['coded']))
for lab in ('B', 'B2'):
    print('==', lab, dict(out[lab]))
for lab in ('B2', 'B'):
    ex = admitted_examples[lab]
    print(f'\n{lab}: admitted synthetic strays ({len(ex)})')
    for e in sorted(ex, key=lambda e: -e[-1])[:40]: print('   ', e)
