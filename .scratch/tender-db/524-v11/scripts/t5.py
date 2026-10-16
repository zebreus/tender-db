exec(open('/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/v11/judge/t1.py').read().split("def A_hold")[0])
R1=sorted([e for e in P['planned'] if e['rule']=='R1' and e['recent_released'] is None],key=lambda e:e['org_id'])
F={e['org_id']:feats(e) for e in R1}
n=len(R1)
print('id range', R1[0]['org_id'], R1[-1]['org_id'])
for i in range(10):
    seg=R1[i*n//10:(i+1)*n//10]
    st=sum(F[e['org_id']]['ta']>=365 for e in seg)
    b=sum((F[e['org_id']]['ta']>=365 and F[e['org_id']]['sh']) or F[e['org_id']]['dB'] for e in seg)
    c3=sum(F[e['org_id']]['ta']>=365 and e['clusters']>=3 for e in seg)
    labs=[s for s in ship if seg[0]['org_id']<=s[0]['org_id']<=seg[-1]['org_id'] and s[1]['recent_released'] is None]
    print('decile %d ids %8d-%8d stale %.1f%% B %.1f%% C3stale %.1f%% | labels %d %s'%(i,seg[0]['org_id'],seg[-1]['org_id'],100*st/len(seg),100*b/len(seg),100*c3/len(seg),len(labs),rnw(labs)))
import statistics
print('median top age', statistics.median(F[e['org_id']]['ta'] for e in R1), 'low510 median', statistics.median(F[e['org_id']]['ta'] for e in R1[:498]))
