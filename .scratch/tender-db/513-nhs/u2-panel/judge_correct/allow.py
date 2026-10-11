import json, sys, os, re, collections
base = sys.argv[1]
L = os.path.join(base, 'ods-lists')
codes = {}
for f in ['list_RO197.json','list_RO261.json','list_RO213.json','list_RO98.json','list_RO108.json']:
    p = os.path.join(L, f)
    try:
        d = json.load(open(p))
    except Exception as e:
        print('skip', f, e); continue
    for o in d.get('Organisations', []):
        codes.setdefault(o['OrgId'], []).append((f, o['Status'], o['OrgRecordClass'], o['PrimaryRoleId'], o['Name']))
three = {c: v for c, v in codes.items() if len(c) == 3}
print('3-char codes total', len(three))
def fam(c):
    digs = [ch for ch in c if ch.isdigit()]
    return (not digs) or all(ch == '0' for ch in digs)
def norm_o0(c): return c.replace('0', 'O')
groups = collections.defaultdict(set)
for c in three: groups[norm_o0(c)].add(c)
tw = [g for g in groups.values() if len(g) > 1]
print('O/0 twin groups among 3-char', len(tw), tw[:10])
allow = set()
by = collections.Counter()
for c, v in three.items():
    if not fam(c): continue
    for (f, st, rc, role, name) in v:
        if st == 'Active' and rc == 'RC1' and f in ('list_RO197.json','list_RO261.json','list_RO213.json') and role in ('RO197','RO261','RO213'):
            if role == 'RO261' and not c.startswith('Q'): continue
            allow.add(c); by[role] += 1; break
print('allow (fam, active RC1, RO197/261Q/213, primary role)', len(allow), by)
# family codes in RO98
r98 = [c for c, v in three.items() if fam(c) and any(f == 'list_RO98.json' for f, *_ in v)]
print('RO98 fam 3-char', len(r98), sorted(r98))
# check listed 129 codes
lst = json.load(open(os.path.join(base, 'nhs-listing-2236.json')))
txt = json.dumps(lst)
adj = json.load(open(os.path.join(base, 'adjudication-rows.json')))
c129 = sorted({x['code'] for x in adj if len(x['code']) == 3})
print('adj codes', len(c129))
missing = [c for c in c129 if c not in allow]
print('adj codes not in allow', missing, [three.get(c) for c in missing])
# junk tails
junk = 'ICB NHS TBC OYG ROB Q0X ODX QYC QKJ QLG 0YD OYD UHS KGH RGY RCN RMB 00N 00P NGH RWK AAV'.split()
for j in junk:
    print(j, 'allow' if j in allow else '-', three.get(j))
