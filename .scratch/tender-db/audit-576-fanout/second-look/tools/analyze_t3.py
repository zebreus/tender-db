"""Second look, T3 (notice window): every touchpoint mention, its bound org and its parent ORG's org.
usage: python3 -I analyze_t3.py <raw json>"""
import json, sys, re, unicodedata, collections
src = open(sys.argv[0].replace('analyze_t3.py', 'analyze_t2.py')).read()
# reuse fold/loose/toks/REVIEW/DEPT from analyze_t2 (definitions only)
defs = src.split('rows = []')[0].split('n4 = json.load')[0]
ns = {}
exec(compile('import json, sys, re, unicodedata, collections, os\n' + defs.split('S, NEWROWS')[0] +
             src[src.index('LEGAL ='):src.index('rows = []')], 'defs', 'exec'), ns)
fold, loose, toks, REVIEW, DEPT = ns['fold'], ns['loose'], ns['toks'], ns['REVIEW'], ns['DEPT']
d = json.load(open(sys.argv[1])); cols = d['columns']
G = [dict(zip(cols, r)) for r in d['rows']]
NEW = 31974972
def cls(g):
    name, pp, pn = g['oname'] or '', g['p_printed'] or '', g['porg_name'] or ''
    if g['porg'] is None: return 'no-parent'
    if g['org'] == g['porg']: return 'SAME-ORG'
    ln, lpp, lpn = loose(name), loose(pp), loose(pn)
    tn, tp = toks(name), toks(pp) | toks(pn)
    cover = len(tn & tp) / len(tn) if tn else 0
    pcover = len(tn & toks(pp)) / len(toks(pp)) if toks(pp) else 0
    if ln and (ln == lpp or ln == lpn): return 'SELF-spelling'
    if ln and lpp and (lpp in ln or (lpn and lpn in ln)) and len(lpp) >= 6: return 'SELF-plus'
    if ln and (ln in lpp or (lpn and ln in lpn)) and len(ln) >= 6: return 'SELF-part'
    if REVIEW.search(fold(name)): return 'review-body'
    if pcover >= 0.6 and cover >= 0.5: return 'SELF-overlap'
    if DEPT.search(fold(name)) and cover >= 0.34: return 'dept-of-buyer?'
    return 'other'
tab = collections.defaultdict(lambda: [0, 0, set()])
tot = collections.Counter(); fb = collections.Counter()
for g in G:
    k = cls(g); side = 'new' if g['org'] >= NEW else 'pre'
    g['cls'] = k
    t = tab[(side, k)]; t[0] += g['n']; t[1] += 1; t[2].add(g['org'])
    tot[side] += g['n']; fb[side] += g['no_own_cc']
allm = sum(tot.values())
print(f"window mentions {allm}: new-org {tot['new']} ({100*tot['new']/allm:.1f}%), pre-existing {tot['pre']}")
print(f"no own BT-514 (country fallback): new {fb['new']}, pre {fb['pre']} ({100*(fb['new']+fb['pre'])/allm:.1f}% of all)")
for side in ('pre', 'new'):
    print(f'-- {side}')
    for (s, k), (n, groups, orgs) in sorted(tab.items(), key=lambda x: -x[1][0]):
        if s == side:
            print(f'   {k:15} mentions {n:5} ({100*n/tot[side]:4.1f}% of {side})  orgs {len(orgs):4}')
json.dump(G, open(sys.argv[1].replace('.json', '-classified.json'), 'w'), ensure_ascii=False, indent=0)
