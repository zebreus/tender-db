import json, glob, collections, importlib.util
B6='/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6'
spec=importlib.util.spec_from_file_location('rule', B6+'/rule_v2_g.py'); R=importlib.util.module_from_spec(spec); spec.loader.exec_module(R)
served=json.load(open(B6+'/../b5/investigate/served_lsa_rows.json'))
ids=sorted(int(r[0]) for r in served if int(r[0])!=179732)
ev=[]
for p in sorted(glob.glob('api/changes/*.json')): ev+=json.load(open(p))['events']
s571=set(json.load(open(B6+'/l_changed_571_ids.json'))); s545=set(json.load(open(B6+'/l_changed_545_ids.json')))
g=collections.defaultdict(set)
for e in ev: g[e['changed_at']].add(e['id'])
print('571 eq', g['2026-10-19T05:53:39Z']==s571, '545 eq 11:06:21', g['2026-10-19T11:06:21Z']==s545, '11:06:08 vs 545', len(g['2026-10-19T11:06:08Z']&s545))
vals=collections.Counter(); sub=collections.Counter()
for t in ids:
    x=R.tender_detail(t)
    for v in x['versions']: sub[v.get('notice_subtype')]+=1
    for n in R.version_notice_ids(x):
        c=R.notice_content(n)
        for s in c['sections']:
            for v in s.get('values',[]):
                if v['field_id'] in ('BT-02-notice','OPP-070-notice','DE1-NoticeSubType-SubTypeCode'):
                    vals[(v['field_id'], str(R._val(v)))]+=1
print(sorted(vals.items()))
print(sub.most_common())
