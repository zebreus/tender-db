# usage: python3 -I analyze_q4.py — reads q4_class_refs.out.json (every role reference on the notices of the
# class-A mentions) and says, per A mention, which role its section plays and whether the OTHER party of the
# agent/principal pair is printed on the same notice in a section of its own. Writes q4_class_refs.summary.json.
import json, re, os, sys
from collections import Counter, defaultdict
D = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, D)
import importlib.util
spec = importlib.util.spec_from_file_location('m', D + '/measure_a.py'); m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
ASSIGN, AGENTS, OWN_CLASS = dict(m.ASSIGN), m.AGENTS, m.OWN_CLASS
# Part 2 adds one principal org 545's detectors missed (NWSSP x7, held 7 > K=6)
ASSIGN[30926924] = ('NWSSP', 'A@P')   # Cardiff and Vale UHB
j = json.load(open(D + '/q4_class_refs.out.json'))
assert 'rows' in j and not j.get('truncated')
cols = j['columns']
rows = [dict(zip(cols, r)) for r in j['rows']]
by = defaultdict(list)
for r in rows:
    by[(r['org'], r['notice'], r['sec'])].append(r)
out = []
stats = Counter(); stats_agent = defaultdict(Counter)
for (org, notice, sec), rs in sorted(by.items()):
    agent, direction = ASSIGN.get(org, ('?', '?'))
    mname = rs[0]['mname'] or ''
    if OWN_CLASS.get((org, mname.strip())) in ('V', 'U', 'T'):
        continue  # not a principal name (NWSSP's own variants / host Velindre / a framework title)
    pat = re.compile(AGENTS[agent]['pat'], re.I)
    if direction == 'P@A' and pat.search(mname):
        continue  # the agent's own name on its own org (e.g. 'NWSSP- Procurement Services'): not in the class
    own_roles = sorted({r['role'] for r in rs if r['ref'] == sec})
    buyers = {r['ref']: (r['ref_org'], r['ref_name']) for r in rs if r['role'] == 'OPT-300-Procedure-Buyer'}
    others = {k: v for k, v in buyers.items() if k != sec}
    if direction == 'P@A':
        # is the AGENT printed in a buyer section of its own on this notice?
        partner = {k: v for k, v in others.items() if pat.search(v[1] or '')}
    else:
        # is a PRINCIPAL (any other buyer section, not naming the agent) printed on this notice?
        partner = {k: v for k, v in others.items() if not pat.search(v[1] or '')}
    rec = dict(org=org, agent=agent, direction=direction, notice=notice, section=sec, name=mname,
               own_roles=own_roles, n_buyer_sections=len(buyers), partner_printed=bool(partner),
               partner=[(k, v[0], v[1]) for k, v in partner.items()],
               other_buyers=[(k, v[0], v[1]) for k, v in others.items()])
    out.append(rec)
    key = ('buyer' if own_roles == ['OPT-300-Procedure-Buyer'] else '+'.join(own_roles) or 'none',
           'partner-printed' if partner else 'alone')
    stats[(direction,) + key] += 1
    stats_agent[agent][key] += 1
json.dump(out, open(D + '/q4_class_refs.summary.json', 'w'), indent=1, ensure_ascii=False)
print('A mentions seen:', len(out))
for k, v in sorted(stats.items()): print('  ', k, v)
for a, c in stats_agent.items(): print('  ', a, dict(c))
