import json, collections, statistics, sys, re
exec(open('common.py').read())
# Design A model: identity per body takes coded (exact or single O/0 fold) mentions on SAME + HOSTED registry holders of that body.
FOLDS={'GBNHSROB':'GBNHSR0B','GBNHSQ0X':'GBNHSQOX'}
def key(raw):
    v=norm(raw)
    if v is None: return None
    return FOLDS.get(v,v)
res=collections.Counter(); canon=[]; largest_before=[]; largest_after=[]; is_largest=0; shrink=0; worse=better=same=0
moved_total=0; emptied=0
per_body_after_u7={}
for b,v in sorted(bodies.items()):
    orgs = v['reg']|v['ppon']
    T = sum(total(o) for o in orgs)
    hosted = {r['org_id'] for r in rows if r['body']==b and r['side']=='registry' and r['final']=='HOSTED_OR_SITE'}
    movers = v['reg']|hosted
    moved = {o: sum(n for raw,n in mc[o].items() if key(raw)=='GBNHS'+b) for o in movers}
    ident = sum(moved.values())
    moved_total += ident
    remain = {o: total(o)-moved[o] for o in v['reg']}
    nonempty = [o for o in v['reg'] if remain[o]>0]
    emptied += sum(1 for o in v['reg'] if remain[o]==0 and moved[o]>0)
    has_ident = 1 if ident>0 else 0
    before = len(orgs)
    after_rebind = len(nonempty) + has_ident + len(v['ppon'])
    single_ppon = len(v['ppon'])==1
    after_u7 = len(nonempty) + (has_ident + len(v['ppon']) - (1 if (has_ident and single_ppon) else 0))
    res['before']+=before; res['rebind']+=after_rebind; res['u7']+=after_u7
    if after_u7<before: better+=1
    elif after_u7>before: worse+=1
    else: same+=1
    canon_m = ident + (total(next(iter(v['ppon']))) if (single_ppon and has_ident) else 0)
    if T and v['reg']:
        canon.append(canon_m/T)
        lb = max(total(o) for o in orgs)/T
        sizes = [remain[o] for o in nonempty] + ([canon_m] if has_ident else []) + ([total(p) for p in v['ppon']] if not (single_ppon and has_ident) else [])
        la = max(sizes)/T
        largest_before.append(lb); largest_after.append(la)
        if has_ident and canon_m == max(sizes): is_largest+=1
        if la<lb-1e-12: shrink+=1
print(dict(res), 'better/same/worse', better, same, worse)
print('moved coded (SAME+HOSTED) total', moved_total, 'emptied SAME clusters', emptied)
print('bodies w/ registry', len(canon), 'median canonical share after U7', statistics.median(canon))
print('canonical is largest', is_largest, 'largest shrinks', shrink)
print('median largest before/after', statistics.median(largest_before), statistics.median(largest_after))
