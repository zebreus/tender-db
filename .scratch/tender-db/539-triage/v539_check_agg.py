import json, collections
D = '/home/user/tender-db/.scratch/tender-db/539-triage/'
F = {r[0]: dict(kind=r[1], period=r[2]) for r in json.load(open(D+'q01_fetches.out.json'))['rows']}
rows=[]
for q in ['q03_agg','q04_agg','q05_agg','q06_agg','q07_day','q08_day','q09_day']:
    j=json.load(open(D+q+'.out.json')); assert 'rows' in j and 'columns' in j and not j.get('truncated')
    rows += [{k:(0 if v is None and k in ('inv','fut','nooff','timed') else v) for k,v in zip(j['columns'], r)} for r in j['rows']]
fs=set(r['f'] for r in rows)
print('fetches with rows', len(fs), 'missing', sorted(set(F)-fs))
dup=collections.Counter((r['f'],r['profile'],r['pb'],r['db']) for r in rows); print('dup groups', sum(1 for v in dup.values() if v>1))
tot=sum(r['n'] for r in rows); print('total', tot)
prof=collections.Counter()
for r in rows: prof[r['profile']]+=r['n']
print(prof.most_common())
def era(f):
    if f==444: return '2022-12'
    return F[f]['kind']
E=collections.defaultdict(collections.Counter)
for r in rows:
    c=E[era(r['f'])]; c['n']+=r['n']; c['fetches_'+str(r['f'])]=1
    c[r['pb'] if r['pb'] in ('in','after','null') else 'stale']+=r['n']
for k,c in E.items(): print(k, 'fetches',sum(1 for x in c if str(x).startswith('fetches_')), {x:c[x] for x in ('n','in','after','null','stale')})
# regular table
B=['b001','b002','b030','b180','b365']
R=collections.defaultdict(collections.Counter)
for r in rows:
    if r['f']==444: continue
    c=R[r['profile']]; d=r['db']!='none'
    c['n']+=r['n']; c['disp']+=r['n']*d
    if r['pb'] in B: c[(r['pb'],d)]+=r['n']
    c['inv']+=r['inv']; c['after']+= r['n'] if r['pb']=='after' else 0
    if r['db']=='after': c['dafter']+=r['n']
for p,c in sorted(R.items(), key=lambda x:-x[1]['n']):
    print(p.replace('eforms:eforms-',''), c['n'], 'disp',c['disp'], 'g1 D/N', c[('b001',True)], c[('b001',False)],
          '2-29', c[('b002',True)]+c[('b002',False)], 'N', c[('b002',False)], '30-179', c[('b030',True)]+c[('b030',False)], '180-364', c[('b180',True)]+c[('b180',False)],
          '365+', c[('b365',True)]+c[('b365',False)], 'ge2', sum(c[(b,x)] for b in B[1:] for x in (True,False)),
          'ge2 N', sum(c[(b,False)] for b in B[1:]), 'inv', c['inv'], 'after', c['after'], 'dafter', c['dafter'])
# all-corpus inversions and dispatch after window
inv=collections.Counter(); da=0; dtot=0; after=collections.Counter(); fut=0; nooff=0
for r in rows:
    inv[r['profile']]+=r['inv']; fut+=r['fut']; nooff+=r['nooff']
    if r['db']!='none': dtot+=r['n']
    if r['db']=='after': da+=r['n']
    if r['pb']=='after': after[r['profile']]+=r['n']
print('inv', sum(inv.values()), inv.most_common()); print('dispatch after window', da, 'of', dtot, 'fut', fut, 'nooff', nooff)
print('pub after window', sum(after.values()), after)
# 444
c=collections.Counter()
for r in rows:
    if r['f']!=444: continue
    c[(r['profile'], r['pb'] in B, r['db'])]+=r['n']; c['inv']+=r['inv']
print('444', c)
# daily era
c=collections.Counter()
for r in rows:
    if F[r['f']]['kind']!='daily': continue
    c[r['pb']]+=r['n']; c['n']+=r['n']
print('daily', c, 'g1 %', c['b001']/c['n'], '2-29 %', c['b002']/c['n'], '30+', c['b030']+c['b180']+c['b365'])
c=collections.Counter()
for r in rows:
    if F[r['f']]['kind']=='monthly' and r['f']!=444: c[r['pb']]+=r['n']
print('monthly regular', c)
c=collections.Counter()
for r in rows:
    if F[r['f']]['kind']=='monthly': c[r['pb']]+=r['n']
print('monthly all', c)
# regular stale classes
c=collections.Counter()
for r in rows:
    if r['f']==444: continue
    if r['pb']=='b001': c['1']+=r['n']
    elif r['pb']=='b002': c['2-29']+=r['n']
    elif r['pb'] in B: c['30+']+=r['n']
print('regular stale', c, sum(c.values()))
c=collections.Counter()
for r in rows:
    if r['pb'] in B: c['all']+=r['n']; c[r['pb']]+=r['n']
print('all stale', c)
