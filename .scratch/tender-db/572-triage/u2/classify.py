# usage: python3 -I classify.py — reads q8_rows.out.json, keeps the class rows with analyze_q6.py's exclusions
# (OWN_CLASS V/U/T names; a P@A name matching the agent's own pattern), tags direction, the 5 transferred
# mentions and q4 membership (the 191 class mentions of 2026-10-19), and writes q8_class.json.
import json, os, re, importlib.util
from collections import Counter
D = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
spec = importlib.util.spec_from_file_location('m', D + '/measure_a.py'); m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
ASSIGN = dict(m.ASSIGN); ASSIGN[30926924] = ('NWSSP', 'A@P')
j = json.load(open(D + '/u2/q8_rows.out.json')); assert 'rows' in j and not j.get('truncated')
rows = [dict(zip(j['columns'], r)) for r in j['rows']]
q4 = json.load(open(D + '/q4_class_refs.summary.json'))
q4keys = {(r['notice'], r['section']) for r in q4}
TRANSFERRED_NOTICES = {46844727, 46844778, 46798381, 46776118, 46768416}
transferred = {(r['notice'], r['section']) for r in q4 if r['notice'] in TRANSFERRED_NOTICES}
cls = []; excl = Counter()
for r in rows:
    agent, d = ASSIGN[r['org']]
    nm = (r['name'] or '').strip()
    oc = m.OWN_CLASS.get((r['org'], nm))
    if oc in ('V', 'U', 'T'):
        excl['own_class_' + oc] += 1; continue
    if d == 'P@A' and re.search(m.AGENTS[agent]['pat'], nm, re.I):
        excl['agent_own_pattern'] += 1; continue
    k = (r['notice_id'], r['section_id'])
    cls.append(dict(r, agent=agent, direction=d, in_q4=k in q4keys, transferred=k in transferred))
print('rows', len(rows), 'class', len(cls), 'excluded', dict(excl))
print('by direction/cur', Counter((c['direction'], c['cur']) for c in cls))
print('not in q4:', [c for c in cls if not c['in_q4']])
q8keys = {(c['notice_id'], c['section_id']) for c in cls}
print('q4 keys missing from q8 class:', sorted(q4keys - q8keys))
print('transferred rows:', [(c['tender_id'], c['cur'], c['role'], c['notice_id'], c['section_id'], c['name']) for c in cls if c['transferred']])
print('roles', Counter(c['role'] for c in cls))
json.dump(cls, open(D + '/u2/q8_class.json', 'w'), indent=0, ensure_ascii=False)
