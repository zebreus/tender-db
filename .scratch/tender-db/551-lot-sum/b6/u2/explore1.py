import json, gzip, os, sys, importlib.util, collections
B6='/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6'
spec=importlib.util.spec_from_file_location('rule', B6+'/rule_v2_g.py'); R=importlib.util.module_from_spec(spec); spec.loader.exec_module(R)
served=json.load(open(B6+'/../b5/investigate/served_lsa_rows.json'))
ids=sorted(int(r[0]) for r in served if int(r[0])!=179732)
print('ids',len(ids))
ft=[]; miss=collections.Counter(); allN=set(); maxn=0; layer=collections.Counter()
missing_ids={}
for t in ids:
    m=json.load(open(f'{R.B6_TENDERS}/{t}.meta.json'))
    ft.append((m['fetched_at'],t))
    x=R.tender_detail(t)
    for n in R.version_notice_ids(x):
        allN.add(n)
        p=R.find_cached('notices',n)
        if p is None: missing_ids.setdefault(t,[]).append(n)
        else: layer[os.path.dirname(p)]+=1
ft.sort()
print('fetch range',ft[0],ft[-1])
print('distinct notices',len(allN),'max',max(allN))
print('missing',sum(len(v) for v in missing_ids.values()), list(missing_ids.items())[:10])
for k,v in layer.items(): print(k,v)
