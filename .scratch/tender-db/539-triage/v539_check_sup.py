import json, collections
D = '/home/user/tender-db/.scratch/tender-db/539-triage/'
F = {r[0]: dict(kind=r[1], period=r[2]) for r in json.load(open(D+'q01_fetches.out.json'))['rows']}
rows=[]
for q in ['q10_sup','q11_sup','q12_sup','q13_sup','q14_sup','q15_sup','q16_sup444a','q17_sup444b']:
    j=json.load(open(D+q+'.out.json')); assert 'rows' in j and not j.get('truncated')
    rows += [dict(zip(j['columns'], r)) for r in j['rows']]
print('fetches', len(set(r['f'] for r in rows)))
K=['stale','with_v','vers','multi','r_doe','r_ted','r_pub']
def tot(pred):
    c=collections.Counter()
    for r in rows:
        if pred(r):
            for k in K: c[k]+=r[k] or 0
    return dict(c)
print('all', tot(lambda r: True))
print('ge2', tot(lambda r: r['pb']!='b001'))
print('ge30', tot(lambda r: r['pb'] in ('b030','b180','b365')))
print('ge30 444 sdk0.1 nodisp', tot(lambda r: r['pb'] in ('b030','b180','b365') and r['f']==444 and r['d']==0 and r['profile'].endswith('sdk-0.1')))
print('sdk0.1 D regular', tot(lambda r: r['profile'].endswith('sdk-0.1') and r['d']==1 and r['f']!=444))
print('sdk0.1 N all', tot(lambda r: r['profile'].endswith('sdk-0.1') and r['d']==0))
for p in ['de-1.1','de-1.2','de-2.0','de-2.1','sdk-1.0']:
    print(p, tot(lambda r: r['profile'].endswith(p)))
print('444', tot(lambda r: r['f']==444))
# compare with step2 stale counts per fetch
st=collections.Counter()
for r in rows: st[r['f']]+=r['stale']
cj=json.load(open(D+'counts.json'))['st']
print('per-fetch stale mismatch vs counts.json', [(f, st[int(f)], v) for f,v in cj.items() if st[int(f)]!=v])
# r_doe by era
print('r_doe daily', tot(lambda r: F[r['f']]['kind']=='daily'), 'monthly', tot(lambda r: F[r['f']]['kind']=='monthly'))
