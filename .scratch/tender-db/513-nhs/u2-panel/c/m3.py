import json, sys, collections, statistics
D=sys.argv[1]
adj=json.load(open(D+'/adjudication-rows.json'))
L={e['value'][5:]:e for e in json.load(open(D+'/nhs-listing-2236.json'))['listing']}
for c in ['0CX','QJG','QKK','QMF','QOX','QWE','QYG','RAL','RGT','RJN','RTF','RXM','RXX','DQF']:
    e=L[c]; print(c, e['class'], e['ppon'], [(h['org_id'],h['name']) for h in e['ppon_holders']], 'reg', e['registry_holder_count'])
# review-style coded share: coded / SAME registry mentions per code
per=collections.defaultdict(lambda:[0,0])
seen=set()
for r in adj:
    if r['side']=='registry' and r['final']=='SAME' and len(r['code'])==3:
        per[r['code']][0]+=r['coded']; per[r['code']][1]+=r['coded']+r['uncoded']+r['other']
sh=[a/b for a,b in per.values() if b]
print('review-style coded share median', round(100*statistics.median(sh),1), 'codes', len(sh), 'below 10%', sum(1 for s in sh if s<0.1))
