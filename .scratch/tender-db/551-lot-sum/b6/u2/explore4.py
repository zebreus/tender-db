import json, glob, collections
B6='/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6'
served=json.load(open(B6+'/../b5/investigate/served_lsa_rows.json'))
ids=set(int(r[0]) for r in served if int(r[0])!=179732)
ev=[]
for p in sorted(glob.glob('api/changes/*.json')):
    ev+=json.load(open(p))['events']
print(len(ev), ev[0], ev[-1])
cur=[int(e['cursor']) for e in ev]; assert cur==sorted(cur) and len(set(cur))==len(cur)
by=collections.OrderedDict()
for e in ev:
    k=(e['changed_at'], e['op'], e['version'] is None)
    by.setdefault(e['changed_at'],[]).append(e)
for t,es in by.items():
    tids=set(e['id'] for e in es)
    ops=collections.Counter((e['op'], e['version'] is None) for e in es)
    print(t, len(es), 'distinct',len(tids), 'in3502',len(tids&ids), 'cur',es[0]['cursor'],es[-1]['cursor'], dict(ops))
