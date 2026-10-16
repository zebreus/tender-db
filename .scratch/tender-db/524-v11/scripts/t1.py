exec(open('/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/v11/judge/feat.py').read())
import collections
ship=[]
for x in L:
    st,e=status(x)
    if st=='planned' and e['rule']=='R1' and x['group'] in ('change:R1','applied'):
        ship.append((x,e,feats(e)))
def rnw(items):
    c=collections.Counter(x['final'] for x,_,_ in items)
    return '%d/%d/%d'%(c['right'],c['neutral'],c['wrong'])
pooled=[s for s in ship if s[0]['plan']!='2555']
capped=[s for s in ship if s[0]['plan']=='2555']
cappedNR=[s for s in capped if s[1]['recent_released'] is None]
print('pooled',len(pooled),rnw(pooled),'capped',len(capped),rnw(capped),'cappedNR',len(cappedNR),rnw(cappedNR))
# dated by own plan as_of vs 2543
diff=0
for x,e,f in ship:
    own=age(e['top_newest'],d(ASOF[x['plan']]))
    if (own>=365)!=(f['ta']>=365): diff+=1
print('own-asof flips at 365:',diff)
def A_hold(f,T=365,D=True): return (f['ta'] is None or f['ta']>=T) or (D and f['dA'])
def B_hold(f,T=365): return ((f['ta'] is None or f['ta']>=T) and f['sh']) or f['dB'] or f['seat']
for name,fn in [('v10',lambda f,e:False),('A365',lambda f,e:A_hold(f)),('B365',lambda f,e:B_hold(f)),
                ('A540',lambda f,e:A_hold(f,540)),('B540',lambda f,e:B_hold(f,540)),
                ('stale365 only',lambda f,e:f['ta']>=365),('D only A',lambda f,e:f['dA']),('D only B',lambda f,e:f['dB'])]:
    out=[]
    for lab,S in [('pooled',pooled),('capped',capped)]:
        rel=lambda s: s[1]['recent_released'] is not None
        h=[s for s in S if fn(s[2],s[1]) and not (name.startswith('A') and rel(s))]
        k=[s for s in S if s not in h]
        out.append('%s held %d %s kept %d %s'%(lab,len(h),rnw(h),len(k),rnw(k)))
    print(name,' | '.join(out))
