import json, sys, collections
D=sys.argv[1]
adj=json.load(open(D+'/adjudication-rows.json'))
for o in (6384818, 13335820, 30916196):
    for r in adj:
        if r['org_id']==o: print(o, r['code'], r['side'], r['final'], r['name'], r['coded'], r['uncoded'], r['other'])
long=[r for r in adj if len(r['code'])>3]
for r in long: print('LONG', r['code'], r['side'], r['final'], r['org_id'], r['name'], r['coded'], r['uncoded'])
