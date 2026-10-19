import json, collections
D = '/home/user/tender-db/.scratch/tender-db/539-triage/'
F = {r[0]: dict(kind=r[1], period=r[2]) for r in json.load(open(D+'q01_fetches.out.json'))['rows']}
rows=[]
for q in ['q20_opta','q21_opta','q22_opta','q23_opta']:
    j=json.load(open(D+q+'.out.json')); assert 'rows' in j and 'columns' in j and not j.get('truncated')
    rows += [dict(zip(j['columns'], r)) for r in j['rows']]
fs=set(r['f'] for r in rows); print('fetches', len(fs), 'total', sum(r['n'] for r in rows))
B={'b001':'1','b002':'2-29','b030':'30+','b180':'30+','b365':'30+'}
A={'a001':'1','a002':'2-29','a030':'30+','a180':'30+','a365':'30+','in':'in','after':'after','null':'null'}
reg=[r for r in rows if r['f']!=444]
T=collections.Counter()
for r in reg:
    if r['pb'] in B: T[(B[r['pb']], A[r['ra']])]+=r['n']
for k in sorted(T): print(k, T[k])
# changed = ra != pb (bucket) or moved; count rows where (a) re-dates: ra bucket differs OR pb in/after but A>p? can't see within bucket
# inversions under a: re-dated = rows where A>p => equals inv count; check pb in/after with ra change
c=collections.Counter()
for r in rows:
    if r['pb'] in ('in',) and r['ra']!='in': c['in->'+r['ra']]+=r['n']
    if r['pb']=='after' and r['ra']!='after': c['after->'+r['ra']]+=r['n']
print('in-window rows moved out', c)
# sdk-0.1 >=2
s=collections.Counter()
for r in reg:
    if r['profile']=='eforms:eforms-sdk-0.1' and r['pb'] in ('b002','b030','b180','b365'):
        s['n']+=r['n']; s['fixed' if r['ra']=='in' else 'res_'+r['ra']]+=r['n']
print('sdk-0.1 >=2', s)
# 30+ residual by profile
s=collections.Counter()
for r in reg:
    if r['pb'] in ('b030','b180','b365') and A[r['ra']]=='30+': s[r['profile']]+=r['n']
print('30+ left', s)
# 2-29 fixed by profile
s=collections.Counter()
for r in reg:
    if r['pb']=='b002' and r['ra']=='in': s[r['profile']]+=r['n']
print('2-29 fixed', s, sum(s.values()))
s=collections.Counter()
for r in reg:
    if r['pb']=='b002' and r['ra']!='in': s[(F[r['f']]['kind'], r['ra'])]+=r['n']
print('2-29 not fixed', s)
# 444 under a
s=collections.Counter()
for r in rows:
    if r['f']==444: s[(r['pb'], r['ra'])]+=r['n']
print('444', s)
