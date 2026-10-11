import json, os, sys, collections
U1 = sys.argv[1]; HERE = os.path.dirname(os.path.abspath(__file__))
exec(open(f'{HERE}/a3_effect.py').read().split("print('reproduce")[0])
moved, remaining, mbc, pcm, kept = plan({'DIFFERENT','NOT_ORG'})
worse = []; better = []
for code in codes + ['DQF']:
    R = {r['org_id'] for r in reg if r['code']==code and r['final']=='SAME'}
    P = {r['org_id'] for r in pp if r['code']==code and r['final']=='SAME'}
    Pall = [(r['org_id'], r['final']) for r in pp if r['code']==code]
    Rrem = {o for o in R if remaining[o] > 0}
    ident = 1 if pcm.get(code) else 0
    a = len(Rrem) + ident + len(P - R)
    ok = ident and len(P)==1 and all(f=='SAME' for _, f in Pall)
    u = a - (1 if ok else 0); t = len(R | P)
    if u > t: worse.append((code, t, a, u, len(R), len(Rrem), Pall))
print('worse after U7:', len(worse))
for w in worse: print('  ', w)
pins = sorted({(r['code'], r['org_id'], r['name'], r['final'], r['coded']) for r in reg if r['final'] in ('DIFFERENT','NOT_ORG')})
print('pin entries (code, holder):', len(pins), 'codes', len({p[0] for p in pins}), 'coded mentions', sum(p[4] for p in pins))
# per-code stray share of coded mentions (registry side), top
pc = collections.defaultdict(collections.Counter)
for r in reg: pc[r['code']][r['final']] += r['coded']
top = sorted(((c['DIFFERENT']+c['NOT_ORG'])/max(1,sum(c.values())), k, dict(c)) for k, c in pc.items())[-8:]
print('highest per-code stray shares:', [(k, round(s,2), c) for s, k, c in reversed(top)])
# capped codes (listing truncated holders)
Lst = json.load(open(f'{U1}/nhs-listing-2236.json'))['listing']
capped = [(x['value'][5:], x['registry_holder_count'], len(x['registry_holders'])) for x in Lst if x['registry_holder_count'] > len(x['registry_holders'])]
print('capped codes:', capped, 'unlisted holders', sum(a-b for _, a, b in capped))
# emptied clusters by class (design A) and which
emp = [(r['code'], r['org_id'], r['final'], r['name'][:50]) for r in reg if remaining[r['org_id']]==0 and moved[r['org_id']]>0]
print('emptied under design A:', len(emp), collections.Counter(e[2] for e in emp))
print('  hosted emptied:', [e for e in emp if e[2]!='SAME'])
# clusters partially moved
part = {r['org_id'] for r in reg if remaining[r['org_id']]>0 and moved[r['org_id']]>0}
print('partially moved clusters:', len(part))
