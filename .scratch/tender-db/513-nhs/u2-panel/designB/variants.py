import json, sys, collections, re
sys.path.insert(0, sys.argv[2])
import gate_b as G
reg = G.reg
base = G.CFGS['G3 +allowlist +O/0 fold (DESIGN B)']
AGENT = re.compile(r'\bc/o\b|\bcare of\b|\bon behalf of\b|\bacting (?:for|on behalf)\b', re.I)
def run(label, cfg, tie_refuses=False, agent=False, generic=None):
    saved = G.GENERIC
    if generic is not None: G.GENERIC = generic
    res = collections.defaultdict(collections.Counter); det = []
    for r in reg:
        code = r['code']
        if code not in G.table: continue
        tc, folded = G.resolve_tail(code, True)
        if tc is None: ok = False; why = 'not-allowlisted'
        else:
            ok, own, other, oc, why = G.gate(r['name'], tc, cfg)
            if ok and tie_refuses and other == own and own > 0: ok, why = False, 'tie'
            if ok and agent and AGENT.search(r['name']): ok, why = False, 'agent'
        k = res[r['final']]; k['holders'] += 1; k['coded'] += r['coded']
        if not ok:
            k['rh'] += 1; k['rc'] += r['coded']
            if r['final'] == 'SAME': det.append(('REFUSED-SAME', code, r['name'][:55], r['coded'], why))
        elif r['final'] in ('DIFFERENT', 'NOT_ORG'): det.append(('ADMITTED-' + r['final'], code, r['name'][:55], r['coded']))
    G.GENERIC = saved
    print('\n==', label)
    for cls in ('SAME', 'HOSTED_OR_SITE', 'DIFFERENT', 'NOT_ORG'):
        k = res[cls]; print(f"  {cls:15s} holders {k['holders']:3d} refused {k['rh']:3d} | coded {k['coded']:5d} refused {k['rc']:4d}")
    for d in sorted(det, key=str): print('   ', d)
run('B base', base)
run('B + ties refuse', base, tie_refuses=True)
run('B + agent markers refuse', base, agent=True)
small = set('nhs the and of trust foundation integrated care board ft icb limited ltd commissioning support unit csu ccg for in t as trading c o'.split())
run('B with a SMALL generic list (keeps hospital/university/teaching/health/healthcare/community/mental/partnership/services words)', base, generic=small)
# names with agent markers among SAME holders
print('\nagent-marker names:', [(r['code'], r['final'], r['name'][:70]) for r in reg if AGENT.search(r['name'])])
# fragile decisions: admitted SAME holders whose margin own-other < 0.15
fr = []
for r in reg:
    if r['code'] not in G.table: continue
    tc, _ = G.resolve_tail(r['code'], True)
    if tc is None: continue
    ok, own, other, oc, why = G.gate(r['name'], tc, base)
    if ok and own - other < 0.15: fr.append((r['final'], r['code'], r['name'][:50], round(own, 2), round(other, 2), oc, r['coded']))
print('\nadmitted with margin < 0.15:', len(fr), 'holders,', sum(x[-1] for x in fr), 'coded mentions; by class', collections.Counter(x[0] for x in fr))
for x in sorted(fr): print('   ', x)
