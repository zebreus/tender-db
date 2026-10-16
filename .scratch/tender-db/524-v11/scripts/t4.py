exec(open('/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/v11/judge/t1.py').read().split("def A_hold")[0])
from math import comb
def fisher_one(a,b,c,d):
    # P(X>=a) hypergeometric, a wrongs in group1 of size a+b, total wrong a+c
    n1=a+b; K=a+c; N=a+b+c+d
    return sum(comb(K,k)*comb(N-K,n1-k) for k in range(a,min(K,n1)+1))/comb(N,n1)
print('stale 2nd>H 3/10 vs stale 2nd<=H 1/29: p=%.4f'%fisher_one(3,7,1,28))
print('stale 2nd>H 3/10 vs stale 2nd<=H 0/28: p=%.4f'%fisher_one(3,7,0,28))
print('stale C3+ 3/17 vs stale C2 1/22: p=%.4f'%fisher_one(3,14,1,21))
print('stale 4/39 vs fresh 1/81: p=%.4f'%fisher_one(4,35,1,80))
print('2nd>H all 3/24 vs 2nd<=H 2/96: p=%.4f'%fisher_one(3,21,2,94))
# 2nd>H items sorted by top age
for x,e,f in sorted(ship,key=lambda s:s[2]['ta']):
    if f['sh']: print(f['ta'],x['org_id'],x['final'],e['head'],e['second'],e['top'],e['clusters'],e['old_name'][:50],'->',e['new_name'][:50])
