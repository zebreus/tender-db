import json, sys
sys.path.insert(0, '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/r524judge')
from keys import alt, vote_key
from collections import defaultdict
j = json.load(open(sys.argv[1]))
assert 'rows' in j and 'columns' in j and not j.get('truncated')
orgs = defaultdict(lambda: {'spell': defaultdict(lambda: [0, None, None])})
for oid, ident, head, country, mname, scheme, n, first in j['rows']:
    o = orgs[oid]; o['ident'] = ident; o['head'] = head; o['country'] = country
    if mname is None or not mname.strip():
        continue
    s = o['spell'][(mname, scheme)]
    s[0] += n; s[1] = scheme; s[2] = first if s[2] is None else min(s[2], first)
def agree(a, b):
    # approximation of altid_keys_agree: equal once legal-form (§) tokens are dropped
    va = ' '.join(t for t in a.split() if not t.startswith('§'))
    vb = ' '.join(t for t in b.split() if not t.startswith('§'))
    return va == vb and va != ''
tot_orgs = len(orgs); ge2 = 0; stray_rows = []
scheme_tot = defaultdict(int)
for oid, o in orgs.items():
    n = sum(v[0] for v in o['spell'].values())
    if n < 2: continue
    ge2 += 1
    altc = defaultdict(int); keyed = []
    for (sp, sch), (c, scheme, first) in o['spell'].items():
        k = alt(sp); altc[k] += c; keyed.append((sp, k, c, scheme, first))
    top = max(altc.values())
    if top < 2: continue
    for sp, k, c, scheme, first in keyed:
        if altc[k] == 1 and not any(other != k and agree(k, other) for other in altc):
            scheme_tot[scheme] += 1
            stray_rows.append({'org': oid, 'ident': o['ident'], 'head': o['head'], 'n': n, 'stray': sp,
                               'scheme': scheme, 'notice': first,
                               'others': sorted(((c2, s2) for (s2, _), (c2, _, _) in o['spell'].items() if s2 != sp), reverse=True)[:4]})
print('orgs', tot_orgs, 'with >=2 mentions', ge2, 'stray mentions', len(stray_rows), 'on orgs', len({r['org'] for r in stray_rows}))
print('by scheme', dict(scheme_tot))
json.dump(stray_rows, open(sys.argv[2], 'w'), indent=1, ensure_ascii=False)
for r in stray_rows:
    print(f"{r['org']} [{r['ident']}] head={r['head']!r} n={r['n']} | STRAY {r['stray']!r} ({r['scheme']}, notice {r['notice']}) | others {r['others']}")
