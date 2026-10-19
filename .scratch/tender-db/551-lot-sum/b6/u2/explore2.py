import json, gzip, os, sys, importlib.util, collections
B6='/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6'
spec=importlib.util.spec_from_file_location('rule', B6+'/rule_v2_g.py'); R=importlib.util.module_from_spec(spec); spec.loader.exec_module(R)
served=json.load(open(B6+'/../b5/investigate/served_lsa_rows.json'))
ids=sorted(int(r[0]) for r in served if int(r[0])!=179732)
byN=collections.defaultdict(set)
for t in ids:
    x=R.tender_detail(t)
    for n in R.version_notice_ids(x): byN[n].add(t)
ns=sorted(byN)
print('top 40:', [(n, sorted(byN[n])[:3]) for n in ns[-40:]])
import bisect
for thr in (26000000,27000000,27500000,28000000,30000000,40000000):
    print(thr, len(ns)-bisect.bisect_left(ns,thr))
print(json.load(open(f'{R.B6_TENDERS}/42392.meta.json')))
