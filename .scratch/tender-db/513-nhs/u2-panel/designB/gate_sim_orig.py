import json, re, sys, collections
U1 = sys.argv[1]; ODS = sys.argv[2]
rows = json.load(open(f'{U1}/adjudication-rows.json'))
names = {}
for f in ['list_RO197.json','list_RO261.json','list_RO213.json','list_RO98.json']:
    for o in json.load(open(f'{ODS}/{f}'))['Organisations']:
        if len(o['OrgId']) == 3:
            names[o['OrgId']] = o['Name']
# the listed codes' own cached names (covers DQF, TAD, TAJ etc.)
import os
for fn in os.listdir(f'{U1}/ods'):
    if not fn.endswith('.json') or fn.startswith('_'): continue
    d = json.load(open(f'{U1}/ods/{fn}'))
    b = d.get('body') or {}
    if 'Organisation' in b and len(fn) == 8:
        names[fn[:3]] = b['Organisation']['Name']
GENERIC = set('nhs the and of trust foundation integrated care board ft icb limited ltd university universities hospital hospitals teaching healthcare health services service partnership community mental group commissioning support unit csu ccg for in t as trading c o sub location strategic local'.split())
SYN = [(r'\bicb\b', 'integrated care board'), (r'\bft\b', 'foundation trust'), (r'\buhb\b','university hospitals birmingham'), (r"'s\b", 's'), (r'\bst\b','saint')]
def toks(n):
    s = (n or '').lower().replace('&', ' and ').replace("’", "'")
    for a, b in SYN: s = re.sub(a, b, s)
    s = s.replace("'", '')
    return {t for t in re.findall(r'[a-z0-9]+', s) if t not in GENERIC}
def sim(a, b):
    A, B = toks(a), toks(b)
    if not A or not B: return 0.0
    return len(A & B) / len(A | B)
reg = [r for r in rows if r['side'] == 'registry']
missing = sorted({r['code'] for r in reg if r['code'] not in names})
print('codes without an ODS name:', missing)
res = collections.defaultdict(lambda: collections.Counter())
detail = []
for r in reg:
    code = r['code']
    if code not in names: continue
    own = names[code]
    A = toks(r['name']); B = toks(own)
    core_share = bool(A & B)
    s_own = sim(r['name'], own)
    best_other = max(((sim(r['name'], n), c) for c, n in names.items() if c != code), default=(0, None))
    gateA_refuse = bool(A) and not core_share
    gateB_refuse = best_other[0] > s_own
    cls = r['final']
    for g, refused in (('A_core_token', gateA_refuse), ('B_best_match', gateB_refuse)):
        res[(g, cls)]['holders'] += 1
        res[(g, cls)]['coded'] += r['coded']
        if refused:
            res[(g, cls)]['refused_holders'] += 1
            res[(g, cls)]['refused_coded'] += r['coded']
    if cls != 'SAME' or gateB_refuse:
        detail.append((cls, code, r['name'], own, round(s_own,2), best_other, gateA_refuse, gateB_refuse, r['coded']))
for k in sorted(res): print(k, dict(res[k]))
print()
for d in sorted(detail): print(d)
