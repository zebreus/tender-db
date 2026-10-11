# B4 = B3 + a coverage floor: at least half of the mention's distinctive tokens must be
# matched by the code's best-fitting own name.
import sys, os, collections
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gate_b as G
for c in list(G.preds):
    G.preds[c] = [p for p in G.preds[c] if p in G.table and G.table[p]['status'] == 'Inactive']
import gate_b2 as G2, gate_b3 as G3
def coverage(name, code):
    D, _, _ = G2.split(name)
    if not D: return 0.0
    best = 0.0
    for nm, _ in G2.code_names(code):
        d, _, _ = G2._cache.setdefault(nm, G2.split(nm))
        best = max(best, sum(1 for a in D if any(G3.tm(a, b) for b in d)) / len(D))
    return best
def admit4(name, tail, floor=0.5):
    tc, why, folded = G3.admit(name, tail)
    if tc is None: return None, why
    if coverage(name, tc) < floor: return None, 'coverage'
    return tc, why
if __name__ == '__main__':
    for side in ('registry', 'ppon'):
        res = collections.defaultdict(collections.Counter); det = []
        for r in G.rows:
            if r['side'] != side or r['code'] not in G.table: continue
            tc, why = admit4(r['name'], r['code'])
            k = res[r['final']]; k['h'] += 1; k['c'] += r['coded']
            if tc is None:
                k['rh'] += 1; k['rc'] += r['coded']
                if r['final'] == 'SAME': det.append(('REFUSED-SAME', r['code'], r['name'][:60], r['coded'], why))
            elif r['final'] in ('DIFFERENT', 'NOT_ORG'): det.append(('ADMITTED-' + r['final'], r['code'], r['name'][:60]))
        print('==', side, {k: dict(v) for k, v in res.items()})
        for d in sorted(det, key=str): print('   ', d)
    # synthetic
    allow_sorted = sorted(G.allow); adm = 0; pairs = 0
    for r in G.reg:
        if r['final'] != 'SAME' or r['code'] not in G.table: continue
        c = r['code']; D, _, _ = G2.split(r['name'])
        i = allow_sorted.index(c) if c in allow_sorted else None
        adj = {allow_sorted[k] for k in ((i - 1, i + 1) if i is not None else ()) if 0 <= k < len(allow_sorted)}
        nb = {d for d in G.allow if d != c and (D & G2.split(G.table[d]['name'])[0])}
        for d in nb | adj:
            if d in G.preds.get(c, []) or c in G.preds.get(d, []): continue
            pairs += 1
            if admit4(r['name'], d)[0]: adm += 1; print('   synthetic admitted', c, r['name'][:40], '->', d)
    print('synthetic pairs', pairs, 'admitted', adm)
