# B3 = B2 + prefix-tolerant distinctive tokens (>=6 chars, cambridge~cambridgeshire) + agent-marker refusal.
import sys, os, re, collections
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gate_b as G, gate_b2 as G2
AGENT = re.compile(r'\bc/o\b|\bcare of\b|\bon behalf of\b|\bacting (?:for|on behalf)\b', re.I)
def tm(a, b): return a == b or (min(len(a), len(b)) >= 6 and (a.startswith(b) or b.startswith(a)))
def jf(A, B):
    if not A or not B: return 0.0
    m = sum(1 for a in A if any(tm(a, b) for b in B))
    return m / (len(A) + len(B) - m)
def key_for(m, names):
    D, E, r = m; best = (0, 0.0, 0.0)
    for nm, role in names:
        d, e, _ = G2._cache.setdefault(nm, G2.split(nm))
        crole = G2.CODE_ROLE.get(role)
        best = max(best, (1 if (r is None or crole == r) else 0, jf(D, d), G2.j(E, e)))
    return best
def gate3(name, code, use_preds=True):
    if AGENT.search(name or ''): return False, None, None, None, 'agent'
    m = G2.split(name)
    own_set = {code} | (set(G.preds.get(code, [])) if use_preds else set())
    own = key_for(m, G2.code_names(code, use_preds))
    if own[0] == 0: return False, own, None, None, 'role'
    if own[1] == 0: return False, own, None, None, 'zero'
    best, bc = max(((key_for(m, [(v['name'], v['role'])]), c) for c, v in G.table.items() if c not in own_set), default=((0, 0, 0), None))
    if best > own: return False, own, best, bc, 'better'
    return True, own, best, bc, ('tie' if best == own else 'ok')
def admit(name, raw_tail):
    tc, folded = G.resolve_tail(raw_tail, True)
    if tc is None: return None, 'not-allowlisted', folded
    ok, own, best, bc, why = gate3(name, tc)
    return (tc if ok else None), why, folded
if __name__ == '__main__':
    res = collections.defaultdict(collections.Counter); det = []; whys = collections.Counter()
    for r in G.reg:
        if r['code'] not in G.table: continue
        tc, why, folded = admit(r['name'], r['code'])
        ok = tc is not None
        k = res[r['final']]; k['holders'] += 1; k['coded'] += r['coded']
        if not ok:
            k['rh'] += 1; k['rc'] += r['coded']; whys[(r['final'], why)] += r['coded']
            if r['final'] == 'SAME': det.append(('REFUSED-SAME', r['code'], r['name'][:55], r['coded'], why))
        else:
            if why == 'tie': det.append(('TIE', r['final'], r['code'], r['name'][:50], r['coded']))
            if r['final'] != 'SAME': det.append(('ADMITTED-' + r['final'], r['code'], r['name'][:60], r['coded']))
    print('== B3 (role, prefix-tolerant distinctive, descriptor; agent markers; preds; allowlist+fold)')
    for cls in ('SAME', 'HOSTED_OR_SITE', 'DIFFERENT', 'NOT_ORG'):
        k = res[cls]; print(f"  {cls:15s} holders {k['holders']:3d} refused {k['rh']:3d} | coded {k['coded']:5d} refused {k['rc']:4d}")
    print('  refusal reasons (coded):', dict(whys))
    for d in sorted(det, key=str): print('   ', d)
