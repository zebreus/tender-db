import json, sys, collections
D=sys.argv[1]
adj=json.load(open(D+'/adjudication-rows.json'))
print(len(adj), adj[0].keys())
c=collections.Counter((r['side'],r['final']) for r in adj)
print(c)
s=collections.defaultdict(lambda:[0,0,0,0])
for r in adj:
    k=(r['side'],r['final']); s[k][0]+=1; s[k][1]+=r.get('coded') or 0; s[k][2]+=r.get('uncoded') or 0; s[k][3]+=r.get('other') or 0
for k,v in sorted(s.items()): print(k,v)
codes=set(r['code'] for r in adj); print('codes',len(codes))
# distinct org ids per class
same_reg=[r for r in adj if r['side']=='registry' and r['final']=='SAME']
print('same reg rows',len(same_reg),'distinct orgs',len(set(r['org_id'] for r in same_reg)))
dup=collections.Counter(r['org_id'] for r in same_reg); print([ (o,n) for o,n in dup.items() if n>1])
print('distinct same reg coded/uncoded (dedup by org):')
seen={}
for r in same_reg: seen[r['org_id']]=r
print(sum(r['coded'] for r in seen.values()), sum(r['uncoded'] for r in seen.values()))
print(set(r['final'] for r in adj))
print([r for r in adj if r['final'] not in ('SAME','DIFFERENT','HOSTED_OR_SITE','NOT_ORG')][:3])
