import json, os, sys, collections, re
U1 = sys.argv[1]
L = f'{U1}/ods-lists'
lists = {}
for f in ['list_RO197.json','list_RO261.json','list_RO213.json','list_RO98.json','list_RO108.json']:
    lists[f[5:-5]] = json.load(open(f'{L}/{f}'))['Organisations']
# per-code ODS records
ods = {}
roles = {r['id']: r['displayName'] for r in json.load(open(f'{U1}/ods/_roles.json'))['Roles']}
for fn in os.listdir(f'{U1}/ods'):
    if fn.startswith('_') or not fn.endswith('.json'): continue
    d = json.load(open(f'{U1}/ods/{fn}'))
    b = d.get('body') or {}
    if 'Organisation' in b:
        o = b['Organisation']
        rs = o.get('Roles', {}).get('Role', [])
        prim = [r['id'] for r in rs if r.get('primaryRole')]
        ods[fn[:-5]] = dict(name=o['Name'], status=o['Status'], rc=o.get('orgRecordClass'), primary=prim[0] if prim else None)
print('per-code ODS records:', len(ods))
three = collections.defaultdict(list)
for role, orgs in lists.items():
    for o in orgs:
        if len(o['OrgId']) == 3:
            three[o['OrgId']].append((role, o['Status'], o['OrgRecordClass'], o['Name']))
print('distinct 3-char codes across lists:', len(three))
for role in lists:
    c = collections.Counter(o['Status'] for o in lists[role] if len(o['OrgId']) == 3)
    print(' ', role, dict(c))
# RO261 3-char: lead letters
print('RO261 3-char leads:', collections.Counter(o['OrgId'][0] for o in lists['RO261'] if len(o['OrgId'])==3))
print('RO261 3-char non-Q:', [(o['OrgId'], o['Name'], o['Status']) for o in lists['RO261'] if len(o['OrgId'])==3 and o['OrgId'][0]!='Q'])
print('RO213 3-char leads:', collections.Counter(o['OrgId'][0] for o in lists['RO213'] if len(o['OrgId'])==3))
print('RO197 3-char leads:', collections.Counter((o['OrgId'][0],o['Status']) for o in lists['RO197'] if len(o['OrgId'])==3))
# digit-rule family among 3-char: letter-only or zero-only
def family(c):
    ds = [ch for ch in c if ch.isdigit()]
    return (not ds) or all(ch == '0' for ch in ds)
for role in ['RO197','RO261','RO213','RO98']:
    fam = [o for o in lists[role] if len(o['OrgId'])==3 and family(o['OrgId'])]
    print(role, 'digit-rule-family 3-char:', len(fam), collections.Counter(o['Status'] for o in fam))
# O/0 twin check across all 3-char codes in lists
allc = set(three)
def variants(c):
    out = {c}
    idx = [i for i,ch in enumerate(c) if ch in 'O0']
    for mask in range(1 << len(idx)):
        s = list(c)
        for k,i in enumerate(idx):
            if mask >> k & 1: s[i] = '0' if c[i]=='O' else 'O'
        out.add(''.join(s))
    return out
twins = [(a, sorted(variants(a) & allc - {a})) for a in sorted(allc) if (variants(a) & allc) - {a}]
print('O/0 twin pairs among all 3-char list codes (any status/role):', twins)
# candidate allowlist variants
def allow(active_only=True, roles=('RO197','RO261','RO213')):
    s = {}
    for role in roles:
        for o in lists[role]:
            c = o['OrgId']
            if len(c)!=3 or not family(c): continue
            if role=='RO261' and c[0]!='Q': continue
            if active_only and o['Status']!='Active': continue
            if o['OrgRecordClass']!='RC1': continue
            s[c] = (role, o['Status'], o['Name'])
    return s
A = allow(True); AI = allow(False)
print('allowlist Active RC1 RO197+RO261(Q)+RO213, digit-rule family:', len(A), collections.Counter(v[0] for v in A.values()))
print('  incl inactive:', len(AI), collections.Counter((v[0],v[1]) for v in AI.values()))
# listed 129 codes: in allowlist?
J = json.load(open(f'{U1}/nhs-ods-joined.json'))
codes129 = sorted({r['code'] for r in J if len(r['code'])==3 and family(r['code']) and r['ods'].get('found')})
print('listed digit-family 3-char codes:', len(codes129))
missing = [c for c in codes129 if c not in A]
print('  not in Active allowlist:', [(c, ods.get(c, {}).get('primary'), roles.get(ods.get(c, {}).get('primary')), ods.get(c,{}).get('status')) for c in missing])
json.dump({'A': A, 'AI': AI, 'ods': ods, 'three': three}, open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'allow.json'), 'w'))
