exec(open('/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/v11/judge/t1.py').read().split("def A_hold")[0])
import collections
# digits releases labelled
dig=[]
for x in L:
    st,e=status(x)
    if st=='planned' and e['recent_released']=='digits': dig.append((x,e,feats(e)))
print('digits labels',len(dig),rnw(dig),'orgs',len({x['org_id'] for x,_,_ in dig}))
for x,e,f in dig:
    if f['ta']>=365 or x['final']=='wrong':
        print(' ',x['plan'],x['org_id'],x['final'],'ta',f['ta'],'C',e['clusters'],'H',e['head'],'2nd',e['second'],e['old_name'][:40],'->',e['new_name'][:40])
DIG=[e for e in P['planned'] if e['recent_released']=='digits']
print('plan digits stale',sum(feats(e)['ta']>=365 for e in DIG),'stale C3+',sum(feats(e)['ta']>=365 and e['clusters']>=3 for e in DIG))
UNF=[e for e in P['planned'] if e['recent_released']=='unfit']
print('plan unfit stale',sum(feats(e)['ta']>=365 for e in UNF),'stale C3+',sum(feats(e)['ta']>=365 and e['clusters']>=3 for e in UNF))
