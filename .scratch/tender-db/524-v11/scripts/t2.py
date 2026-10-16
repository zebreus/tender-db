exec(open('/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/v11/judge/feat.py').read())
import collections
R1=[e for e in P['planned'] if e['rule']=='R1' and e['recent_released'] is None]
DIG=[e for e in P['planned'] if e['rule']=='R1' and e['recent_released']=='digits']
UNF=[e for e in P['planned'] if e['rule']=='R1' and e['recent_released']=='unfit']
R2=[e for e in P['planned'] if e['rule']=='R2']
F={e['org_id']:feats(e) for e in P['planned']}
def tot(es): return len(es), sum(e['n'] for e in es)
print('R1 nonrel',tot(R1),'digits',tot(DIG),'unfit',tot(UNF),'R2',tot(R2))
print('missing top_newest R1:',sum(1 for e in R1 if e['top_newest'] is None))
scope=sorted([e for e in P['planned'] if e['rule']!='R2' and e['recent_released']!='unfit'], key=lambda e:e['org_id'])
low=scope[:510]
lowR1=[e for e in low if e['rule']=='R1' and e['recent_released'] is None]
print('low510',len(low),'lowR1',len(lowR1))
def report(name,fn,pop=R1):
    h=[e for e in pop if fn(F[e['org_id']],e)]
    hl=[e for e in low if e['rule']=='R1' and e['recent_released'] is None and fn(F[e['org_id']],e)]
    n,m=tot(h); N,M=tot(pop)
    print('%-28s holds %5d (%.1f%%) mentions %7d (%.1f%%) | low510 %d (%.1f%%)'%(name,n,100*n/N,m,100*m/M,len(hl),100*len(hl)/len(low)))
for T_ in [180,270,365,450,540,600,625,730,1095]:
    report('stale>=%d'%T_, lambda f,e,T_=T_: f['ta'] is None or f['ta']>=T_)
    report('stale>=%d & 2nd>H'%T_, lambda f,e,T_=T_: (f['ta'] is None or f['ta']>=T_) and f['sh'])
    report('stale>=%d & C>=3'%T_, lambda f,e,T_=T_: (f['ta'] is None or f['ta']>=T_) and e['clusters']>=3)
report('dA',lambda f,e:f['dA']); report('dB',lambda f,e:f['dB']); report('seat',lambda f,e:f['seat'])
report('A full', lambda f,e:(f['ta']>=365) or f['dA'])
report('B full', lambda f,e:(f['ta']>=365 and f['sh']) or f['dB'] or f['seat'])
print('placeholder ids in planned:',[(e['org_id'],e['identifier'],e['recent_released']) for e in P['planned'] if F[e['org_id']]['ph']])
print('placeholder ids in held:',[(e['org_id'],e['identifier'],e['reason']) for e in P['held'] if placeholder_id(e.get('identifier'))])
print('R2 dA',sum(F[e['org_id']]['dA'] for e in R2),'dB',sum(F[e['org_id']]['dB'] for e in R2))
print('digits: stale365',sum(F[e['org_id']]['ta']>=365 for e in DIG),'stale&2nd>H',sum(F[e['org_id']]['ta']>=365 and F[e['org_id']]['sh'] for e in DIG),'dA',sum(F[e['org_id']]['dA'] for e in DIG),'dB',sum(F[e['org_id']]['dB'] for e in DIG),'seat',sum(F[e['org_id']]['seat'] for e in DIG))
