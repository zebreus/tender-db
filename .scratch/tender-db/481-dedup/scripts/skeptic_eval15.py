import sys, json; sys.path.insert(0,'.')
from lib import *
D=json.load(open('disp_neg_ef.json')); TW=json.load(open('uuid_twins_ef.json'))
def bucket(x):
    if x is None: return 'missing'
    x=abs(x)
    return 'eq' if x==0 else ('<=60s' if x<=60 else ('<=1h' if x<=3600 else ('<=1d' if x<=86400 else '>1d')))
for pool in ('twin_known','doe_only_key_disjoint'):
    c=collections.Counter(); ex=[]
    for n in N:
        if n['pool']!=pool: continue
        a=n['doe'].get('dispatched_at'); b=D.get(str(n['ted']['notice_id']))
        k=bucket(None if a is None or b is None else b-a); c[k]+=1
        if k in ('eq','<=60s'): ex.append((n['doe_tender_id'],n['ted_tender_id'],b-a,title_of(n['doe'])[:50],title_of(n['ted'],'canonical')[:50]))
    print(pool,sorted(c.items())); [print('   ',e) for e in ex]
# eForms islands: twin vs non-twin candidates
c=collections.Counter(); ex=[]
for t in U:
    if t['doe_profile'].startswith('eforms-sdk-0.1'): continue
    tw=set(TW.get(str(t['tender_id']),[])); a=t['doe'].get('dispatched_at')
    for x in t['candidates']:
        if not x['has_signals']: continue
        b=D.get(str(x['notice_id']))
        k=bucket(None if a is None or b is None else b-a)
        lab='twin' if x['notice_id'] in tw else 'nontwin'
        c[(lab,k)]+=1
        if lab=='nontwin' and k in ('eq','<=60s'): ex.append((t['tender_id'],x['tender_id'],b-a,(title_of(t['doe'],'canonical') or '')[:50],(title_of(x,'canonical') or '')[:50]))
print('eforms islands',sorted(c.items())); [print('   ',e) for e in ex]
