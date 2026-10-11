import json, collections, statistics, sys, re
U1 = sys.argv[1]
rows = json.load(open(f'{U1}/adjudication-rows.json'))
# fix TAJ ppon
for r in rows:
    if r['code']=='TAJ' and r['side']=='ppon': r['org_id']=30917612; r['_fixed']=True
FOLD={'WESTMIDLANDSAMBULANCESERVICE':'RYA','NHSCHESHIREANDMERSEYSIDEICB':'QYG'}
for r in rows: r['body']=FOLD.get(r['code'], r['code'])
mc = collections.defaultdict(dict)
for line in open(f'{U1}/mention-counts-by-org.jsonl'):
    line=line.strip()
    if not line: continue
    for org, raw, n in json.loads(line)['rows']:
        mc[org][raw] = mc[org].get(raw,0)+n
def norm(raw):
    if raw is None: return None
    v = re.sub(r'[^A-Z0-9]','', raw.upper())
    return v
def coded_for(org, code):
    return sum(n for raw,n in mc[org].items() if raw is not None and norm(raw)=='GBNHS'+code)
def total(org): return sum(mc[org].values())
bodies = collections.defaultdict(lambda: {'reg':set(),'ppon':set()})
for r in rows:
    if r['final']=='SAME':
        bodies[r['body']]['reg' if r['side']=='registry' else 'ppon'].add(r['org_id'])
print('bodies', len(bodies))
allsame = set()
for b,v in bodies.items(): allsame |= v['reg']|v['ppon']
print('distinct SAME orgs', len(allsame))
# orgs SAME in >1 body?
cnt = collections.Counter()
for b,v in bodies.items():
    for o in v['reg']|v['ppon']: cnt[o]+=1
print('orgs in >1 body', [o for o,c in cnt.items() if c>1])
tot = sum(total(o) for o in allsame)
print('SAME mentions total (distinct orgs)', tot)
# per-body shares
largest=[]; ppon_share=[]; coded_share=[]; coded_share_reg=[]; norgs=[]
for b,v in sorted(bodies.items()):
    orgs = v['reg']|v['ppon']
    T = sum(total(o) for o in orgs)
    if T==0: continue
    largest.append(max(total(o) for o in orgs)/T)
    if v['ppon']:
        ppon_share.append(max(total(o) for o in v['ppon'])/T)
    c = sum(coded_for(o,b) for o in v['reg'])
    Treg = sum(total(o) for o in v['reg'])
    coded_share.append(c/T)
    if Treg: coded_share_reg.append(c/Treg)
    norgs.append(len(orgs))
print('median largest share', statistics.median(largest))
print('median PPON org share (bodies with ppon)', statistics.median(ppon_share), len(ppon_share))
print('median coded share of all SAME', statistics.median(coded_share))
print('median coded share of registry SAME', statistics.median(coded_share_reg), len(coded_share_reg), sum(1 for x in coded_share_reg if x<0.10))
print('orgs today', sum(norgs), 'bodies', len(norgs))
