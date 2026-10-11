import sys, os, collections, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gate_b as G, gate_b2 as G2, gate_b3 as G3
orgs = {}
for r in G.reg: orgs.setdefault(r['org_id'], []).append(r)
def norm(raw): return ''.join(c for c in (raw or '') if c.isascii() and c.isalnum()).upper()
listed = {r['code'] for r in G.reg}
print('== the 22 extra three-character tails under B3')
tot = collections.Counter()
for org, rs in sorted(orgs.items()):
    for raw, n in G.mc[org].items():
        v = norm(raw)
        if not (v.startswith('GBNHS') and len(v) == 8): continue
        t = v[5:]
        if any(ch.isdigit() and ch != '0' for ch in t): continue   # mints today
        if t in listed and t in {x['code'] for x in rs}: continue
        tc, why, folded = G3.admit(rs[0]['name'], t)
        tot['admitted' if tc else 'refused'] += n
        print(f"  {t} x{n} on {org} '{rs[0]['name'][:45]}' (listed {[x['code'] for x in rs]}) -> {tc} {why}{' folded' if folded else ''}")
print(dict(tot))
# bare three-character raws (non-FTS sources) on the registry holders
bare = collections.Counter(); bare_allow = collections.Counter()
for org in orgs:
    for raw, n in G.mc[org].items():
        v = norm(raw)
        if raw is not None and len(v) == 3:
            bare[v] += n
            if v in G.allow: bare_allow[v] += n
print('bare 3-char raws', sum(bare.values()), 'of which an allowlisted code', sum(bare_allow.values()), bare_allow.most_common(10))
# RWW
for nm in ['Warrington and Halton Teaching Hospitals NHS Foundation Trust', 'Bridgewater Community Healthcare NHS Foundation Trust', 'North Cheshire and Mersey NHS Foundation Trust']:
    print('RWW', nm, 'with preds:', G3.gate3(nm, 'RWW')[0::4], G3.gate3(nm, 'RWW')[1:4], '| without preds:', G3.gate3(nm, 'RWW', use_preds=False)[0::4])
print('RWW preds', G.preds['RWW'], [G.table[p]['name'] for p in G.preds['RWW']], 'RJQ =', G.table.get('RJQ'))
