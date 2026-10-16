exec(open('/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/v11/judge/t1.py').read().split("def A_hold")[0])
import collections
def cell(s):
    x,e,f=s
    st='stale' if f['ta']>=365 else 'fresh'
    c='C2' if e['clusters']==2 else 'C3+'
    q='2nd>H' if f['sh'] else '2nd<=H'
    return (st,c,q)
cells=collections.defaultdict(list)
for s in ship: cells[cell(s)].append(s)
for k in sorted(cells): 
    S=cells[k]; print(k,len(S),rnw(S),'wrongs:',[s[0]['org_id'] for s in S if s[0]['final']=='wrong'])
# planwide
R1=[e for e in P['planned'] if e['rule']=='R1' and e['recent_released'] is None]
pc=collections.Counter()
pm=collections.Counter()
for e in R1:
    f=feats(e); k=cell((None,e,f)); pc[k]+=1; pm[k]+=e['n']
for k in sorted(pc): print('plan',k,pc[k],pm[k])
