import json, sys, os
D=sys.argv[1]
out=[]
for fn in sorted(os.listdir(D+'/ods')):
    if not fn.endswith('.json') or len(fn)!=8: continue
    d=json.load(open(D+'/ods/'+fn)); b=(d.get('body') or {}).get('Organisation')
    if not b: continue
    ends=[x for x in b.get('Date',[]) if x.get('Type')=='Legal' and x.get('End')]
    succ=[s.get('Target',{}).get('OrgId',{}).get('extension') for s in (b.get('Succs') or {}).get('Succ',[]) if s.get('Type')=='Successor']
    if ends: out.append((fn[:3], ends[0]['End'], b.get('Status'), succ))
print(len(out)); [print(o) for o in out]
