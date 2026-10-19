# usage: python3 -I analyze_q6.py — reads q6_current.out.json: for each class-A mention's party row, whether it
# sits on the tender's CURRENT version (served by /v1/tenders/{id} parties[] and v_tender_buyers) or an older one.
# Same exclusions as analyze_q4.py (agent's own variants, host Velindre, framework title). Writes q6_current.summary.json.
import json, os, re, sys
from collections import Counter, defaultdict
D = os.path.dirname(os.path.abspath(__file__))
import importlib.util
spec = importlib.util.spec_from_file_location('m', D + '/measure_a.py'); m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
ASSIGN = dict(m.ASSIGN); ASSIGN[30926924] = ('NWSSP', 'A@P')
j = json.load(open(D + '/q6_current.out.json')); assert 'rows' in j and not j.get('truncated')
rows = [dict(zip(j['columns'], r)) for r in j['rows']]
c = Counter(); ca = defaultdict(Counter); tend = defaultdict(int)
for r in rows:
    agent, d = ASSIGN[r['org']]
    nm = (r['name'] or '').strip()
    if m.OWN_CLASS.get((r['org'], nm)) in ('V', 'U', 'T'):
        continue
    if d == 'P@A' and re.search(m.AGENTS[agent]['pat'], nm, re.I):
        continue
    k = 'current' if r['cur'] else 'older-version'
    c[(d, k)] += r['n']; ca[agent + ' ' + d][k] += r['n']
out = {'by_direction': {f'{a} {b}': v for (a, b), v in c.items()}, 'by_agent': {a: dict(v) for a, v in ca.items()},
       'total_party_rows': sum(c.values())}
json.dump(out, open(D + '/q6_current.summary.json', 'w'), indent=1)
print(json.dumps(out, indent=1))
