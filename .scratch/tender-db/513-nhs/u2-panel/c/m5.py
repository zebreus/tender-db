import json, sys, collections, re
D=sys.argv[1]
adj=json.load(open(D+'/adjudication-rows.json'))
TYPED={'WESTMIDLANDSAMBULANCESERVICE':'RYA','NHSCHESHIREANDMERSEYSIDEICB':'QYG'}
def norm(n):
    c=' '.join(n.lower().split())
    s=c.rstrip('.,; ')
    return s or c
names=collections.defaultdict(set)
for r in adj:
    if r['final']=='SAME':
        names[norm(r['name'])].add(TYPED.get(r['code'],r['code']))
coll={n:b for n,b in names.items() if len(b)>1}
print('SAME name_norms claimed by 2+ bodies:', coll)
# SAME name equal to a non-SAME holder's name in another body
non=collections.defaultdict(set)
for r in adj:
    if r['final']!='SAME': non[norm(r['name'])].add((r['code'],r['final']))
x={n:(names[n],non[n]) for n in names if n in non}
print('SAME names that are also non-SAME holder names elsewhere:', len(x)); [print(' ',k,v) for k,v in list(x.items())[:20]]
# distinct names per body (alias set size)
per=collections.defaultdict(set)
for r in adj:
    if r['final']=='SAME': per[TYPED.get(r['code'],r['code'])].add(norm(r['name']))
print('alias names total', sum(len(v) for v in per.values()), 'bodies', len(per))
# generic-looking names: no token left after the gate_sim GENERIC list
GENERIC=set('nhs the and of trust foundation integrated care board ft icb limited ltd university universities hospital hospitals teaching healthcare health services service partnership community mental group commissioning support unit csu ccg for in t as trading c o sub location strategic local'.split())
gen=[(n,b) for n,b in names.items() if not [t for t in re.findall(r'[a-z0-9]+', n.replace('&',' and ')) if t not in GENERIC]]
print('SAME names with no distinctive token:', gen)
