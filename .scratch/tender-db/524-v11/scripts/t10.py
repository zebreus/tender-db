exec(open('/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/v11/judge/t9.py').read().split("c=collections.Counter(); m=")[0])
import collections,re
# labelled breakdown by v11 reason
def lab_reason(x,e):
    return v11('planned',e)
for name,S in [('pooled',[s for s in ship if s[0]['plan']!='2555']),('capped',[s for s in ship if s[0]['plan']=='2555'])]:
    c=collections.defaultdict(list)
    for x,e,f in S: c[lab_reason(x,e)].append((x,e,f))
    for k in sorted(c): print(name,k,len(c[k]),rnw(c[k]),[s[0]['org_id'] for s in c[k] if s[0]['final']!='right'] if k[0]=='hold' else '')
dig=[]
for x in L:
    st,e=status(x)
    if st=='planned' and e['recent_released']=='digits': dig.append((x,e,None))
c=collections.defaultdict(list)
for s in dig: c[v11('planned',s[1])].append(s)
for k in sorted(c): print('digits',k,len(c[k]),rnw(c[k]))
# junk-ish heads among top_stale holds
R1=[e for e in P['planned'] if e['rule']=='R1' and e['recent_released'] is None]
ts=[e for e in R1 if v11('planned',e)==('hold','top_stale')]
ctrl=sum(1 for e in ts if re.search(r'[\x00-\x1f]',e['old_name']))
digs=sum(1 for e in ts if set(t for t in T(e['old_name']) if t.isdigit())-set(T(e['new_name'])))
print('top_stale holds',len(ts),'ctrl heads',ctrl,'heads w/ digit token absent from new',digs)
