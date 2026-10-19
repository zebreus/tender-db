# Lists the tie-the-top gap (the 2a check's Bupa point): an alt key whose count EQUALS the org's top count while
# another key also holds it. Neither STRAY nor HELD-k (count < top) can list it. Same prefilter as candidates.py
# (tokens disjoint from the head and the other top keys). NOT reviewed in this unit: written for the next one.
import json, sys
from collections import defaultdict
sys.path.insert(0, '.')
from keys import alt
import candidates as C
out = {}
for label, paths in (('2a', ['../u2a/s%d.out.json' % i for i in range(1, 7)]),
                     ('2b', ['b%d.out.json' % i for i in range(1, 8)] + ['../q8_cohslice.out.json'])):
    orgs = C.load(paths); allk = 0; rows = []
    for oid, o in orgs.items():
        altc = defaultdict(int); sp = defaultdict(list)
        for (s, sch), x in o['spell'].items(): altc[alt(s)] += x['n']; sp[alt(s)].append(s)
        if not altc: continue
        top = max(altc.values()); tops = [k for k, c in altc.items() if c == top]
        if top < 2 or len(tops) < 2: continue
        for k in tops:
            if any(o2 != k and C.agree(k, o2) for o2 in altc): continue
            allk += 1
            pool = C.toks(o['head'])
            for k2 in tops:
                if k2 != k:
                    for s2 in sp[k2]: pool |= C.toks(s2)
            if not (C.toks(sp[k][0]) & pool):
                rows.append({'org': oid, 'ident': o['ident'], 'head': o['head'], 'key_count': top, 'spellings': sp[k]})
    out[label] = {'tie_keys': allk, 'kept': rows}
    print(label, 'tie-the-top keys', allk, 'prefilter-kept', len(rows))
json.dump(out, open('ties.json', 'w'), indent=1, ensure_ascii=False)
