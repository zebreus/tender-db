import json, sys, collections
D=sys.argv[1]
adj=json.load(open(D+'/adjudication-rows.json'))
L=json.load(open(D+'/nhs-listing-2236.json'))['listing']
by={}
for e in L:
    code=e['literal'][len('GB-NHS-'):] if e['literal'].upper().startswith('GB-NHS-') else e['literal']
    by[e['value'][5:]]=e
bad=[]
for r in adj:
    e=by.get(r['code'])
    if not e: bad.append(('nocode',r['code'])); continue
    hs = e['registry_holders'] if r['side']=='registry' else e['ppon_holders']
    ids={h['org_id']:h['name'] for h in hs}
    if r['org_id'] not in ids: bad.append((r['code'],r['side'],r['org_id'],r['name'],ids))
    elif ids[r['org_id']]!=r['name']: bad.append(('namediff',r['code'],r['side'],r['org_id'],r['name'],ids[r['org_id']]))
print(len(bad)); [print(b) for b in bad]
# identity-null codes in listing
nulls=[e for e in L if e['identity'] is None]
print('identity-null listing codes', len(nulls))
print('registry holder totals listed/count', sum(len(e['registry_holders']) for e in nulls), sum(e['registry_holder_count'] for e in nulls))
print('ppon holder totals listed/count', sum(len(e['ppon_holders']) for e in nulls), sum(e['ppon_holder_count'] for e in nulls))
capped=[(e['value'][5:], e['registry_holder_count']) for e in nulls if e['registry_holder_count']>len(e['registry_holders'])]
print('capped', capped, sum(c-5 for _,c in capped))
