exec(open('/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/v11/judge/t1.py').read().split("def A_hold")[0])
import collections, re
def dbl(s):
    t=T(s); n=len(t)
    return n>0 and n%2==0 and t[:n//2]==t[n//2:] and any(len(x)>=3 for x in t)
def doubled(old,new): return dbl(new) and not dbl(old)
def rival(e): return e['clusters'] > 1 + (1 if e['head']>0 else 0)
def stale(e,Td): 
    ta=age(e['top_newest'],PLAN_ASOF); return ta is None or ta>=Td
def M_reason(e,Td=365,seat=True):
    if doubled(e['old_name'],e['new_name']): return 'doubled'
    if seat and seat_dropped(e['old_name'],e['new_name']): return 'seat_dropped'
    if e['recent_released']: return None
    if stale(e,Td) and rival(e): return 'top_stale'
    return None
R1=[e for e in P['planned'] if e['rule']=='R1' and e['recent_released'] is None]
print('H=0 R1 entries', sum(1 for e in R1 if e['head']==0), 'H=0 & C2', sum(1 for e in R1 if e['head']==0 and e['clusters']==2))
for Td in [180,270,365,450,540,600,625,730]:
    out=[]
    for lab,S in [('pooled',[s for s in ship if s[0]['plan']!='2555']),('capped',[s for s in ship if s[0]['plan']=='2555'])]:
        h=[s for s in S if M_reason(s[1],Td)]; k=[s for s in S if not M_reason(s[1],Td)]
        out.append('%s held %d %s kept %d %s'%(lab,len(h),rnw(h),len(k),rnw(k)))
    c=collections.Counter(M_reason(e,Td) for e in R1); m=collections.Counter()
    for e in R1: m[M_reason(e,Td)]+=e['n']
    print('M T=%d'%Td,' | '.join(out),'| plan',{k:(c[k],m[k]) for k in c if k})
# M at 365 details
Td=365
c=collections.Counter(M_reason(e) for e in R1); m=collections.Counter()
for e in R1: m[M_reason(e)]+=e['n']
tot_h=sum(c[k] for k in c if k); tot_m=sum(m[k] for k in m if k)
print('M365 R1 holds',tot_h,'%.1f%%'%(100*tot_h/len(R1)),'mentions',tot_m,'%.1f%%'%(100*tot_m/sum(e['n'] for e in R1)))
print('R1 ships after M:',len(R1)-tot_h, sum(e['n'] for e in R1)-tot_m)
scope=sorted([e for e in P['planned'] if e['rule']!='R2' and e['recent_released']!='unfit'], key=lambda e:e['org_id'])
low=[e for e in scope[:510]]
lc=collections.Counter(M_reason(e) for e in low)
print('low510',lc)
# next capped run population: lowest 500 not held
kept=[e for e in scope if not M_reason(e)]
print('next capped: lowest 500 kept span ids',kept[0]['org_id'],kept[499]['org_id'], 'rank of 500th in scope', scope.index(kept[499]))
DIG=[e for e in P['planned'] if e['recent_released']=='digits']
print('digits held by doubled/seat:',[(e['org_id']) for e in DIG if M_reason(e)])
R2=[e for e in P['planned'] if e['rule']=='R2']
print('R2 doubled',sum(doubled(e['old_name'],e['new_name']) for e in R2),'seat',sum(seat_dropped(e['old_name'],e['new_name']) for e in R2))
# by decile
R1s=sorted(R1,key=lambda e:e['org_id']); n=len(R1s)
print('decile hold%:',[round(100*sum(1 for e in R1s[i*n//10:(i+1)*n//10] if M_reason(e))/(n//10),1) for i in range(10)])
