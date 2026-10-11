import json, sys, statistics, collections, re
d = sys.argv[1]
rows = json.load(open(d + '/adjudication-rows.json'))
mc = collections.defaultdict(dict)
for line in open(d + '/mention-counts-by-org.jsonl'):
    line=line.strip()
    if not line: continue
    for org, raw, n in json.loads(line)['rows']:
        mc[org][raw] = mc[org].get(raw, 0) + n
codes = sorted({r['code'] for r in rows})
print('codes', len(codes))
same_reg = collections.defaultdict(set); same_ppon = collections.defaultdict(set)
for r in rows:
    if r['final'] != 'SAME': continue
    (same_reg if r['side']=='registry' else same_ppon)[r['code']].add(r['org_id'])
allsame = set()
for c in codes: allsame |= same_reg[c] | same_ppon[c]
print('distinct SAME orgs', len(allsame), 'per-code sum', sum(len(same_reg[c]|same_ppon[c]) for c in codes))
# total mentions on distinct SAME orgs
tot = sum(sum(mc[o].values()) for o in allsame if o in mc)
print('mentions on distinct SAME orgs', tot, 'missing counts', [o for o in allsame if o not in mc])
# coded share per code on registry side
def is_coded(raw, code):
    if raw is None: return False
    v = re.sub(r'[^A-Za-z0-9]','',raw).upper()
    return v == 'GBNHS'+code
shares=[]
for c in codes:
    if not same_reg[c]: continue
    coded = sum(n for o in same_reg[c] for raw,n in mc.get(o,{}).items() if is_coded(raw,c))
    total = sum(n for o in same_reg[c] for raw,n in mc.get(o,{}).items())
    if total: shares.append(coded/total)
print('registry codes', len(shares), 'median coded share %.3f' % statistics.median(shares), 'below10', sum(s<0.1 for s in shares))
# largest org share today per body (union reg+ppon)
lg=[]; canon=[]
for c in codes:
    orgs = same_reg[c]|same_ppon[c]
    tots = {o: sum(mc.get(o,{}).values()) for o in orgs}
    T = sum(tots.values())
    if T==0: continue
    lg.append(max(tots.values())/T)
    if same_ppon[c]:
        canon.append(sum(tots[o] for o in same_ppon[c])/T)
print('median largest share %.3f' % statistics.median(lg), 'median ppon share %.3f' % statistics.median(canon), len(canon))
