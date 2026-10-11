import json, sys, collections
D=sys.argv[1]
adj=json.load(open(D+'/adjudication-rows.json'))
def show(codes=None, orgs=None):
    for r in adj:
        if (codes and r['code'] in codes) or (orgs and r['org_id'] in orgs):
            print(r['code'], r['side'], r['org_id'], r['final'], repr(r['name']), r['coded'], r['uncoded'], r['other'], '|', (r.get('real_body') or ''), (r.get('real_ods_code') or ''))
show(codes={'TAJ','QMM','RYA','WESTMIDLANDSAMBULANCESERVICE','QYG','NHSCHESHIREANDMERSEYSIDEICB'})
print('---')
show(orgs={1371484,4275910,3793356,30914670,1167736,1240944,1432862,4474166})
# per-code count the review's way
per=collections.defaultdict(set)
for r in adj:
    if r['final']=='SAME': per[r['code']].add(r['org_id'])
print('per-code SAME sum', sum(len(v) for v in per.values()), 'codes', len(per))
