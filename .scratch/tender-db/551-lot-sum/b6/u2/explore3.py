import json, gzip, os, sys, importlib.util, collections
B6='/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6'
spec=importlib.util.spec_from_file_location('rule', B6+'/rule_v2_g.py'); R=importlib.util.module_from_spec(spec); spec.loader.exec_module(R)
served=json.load(open(B6+'/../b5/investigate/served_lsa_rows.json'))
ids=sorted(int(r[0]) for r in served if int(r[0])!=179732)
hits=collections.defaultdict(list); forms=collections.Counter(); kinds=collections.Counter()
for t in ids:
    x=R.tender_detail(t)
    for n in R.version_notice_ids(x):
        c=R.notice_content(n)
        for s in c.get('sections',[]):
            kinds[s.get('kind')]+=1
            for v in s.get('values',[]):
                f=v['field_id']
                if f=='TED-COST_PARAMETERS' or f.startswith('TED-AWARD_CONTRACT_MOVE'):
                    hits[t].append((n,f))
                if f=='TED-FORM':
                    fv=str(R._val(v)).strip().upper()
                    forms[fv]+=1
                    if fv in ('T01','T02'): hits[t].append((n,'TED-FORM '+fv))
print('T01/T02/COST_PARAMETERS hits', dict(hits))
print('forms', forms.most_common(40))
print('section kinds', kinds.most_common())
