import json, sys, collections
D=sys.argv[1]
adj=json.load(open(D+'/adjudication-rows.json'))
codes={r['code'] for r in adj}
mc=collections.defaultdict(dict)
for line in open(D+'/mention-counts-by-org.jsonl'):
    for o,raw,n in json.loads(line)['rows']:
        mc[o][raw]=mc[o].get(raw,0)+n
TYPED={'WESTMIDLANDSAMBULANCESERVICE':'RYA','NHSCHESHIREANDMERSEYSIDEICB':'QYG'}
same=collections.defaultdict(set)
for r in adj:
    if r['final']=='SAME' and r['side']=='registry': same[TYPED.get(r['code'],r['code'])].add(r['org_id'])
def tail(raw):
    v=''.join(ch for ch in (raw or '') if ch.isalnum()).upper()
    return v[5:] if v.startswith('GBNHS') else None
junk=collections.Counter(); neigh=collections.Counter()
for code,orgs in same.items():
    for o in orgs:
        for raw,n in mc[o].items():
            t=tail(raw)
            if not t or len(t)!=3 or t==code: continue
            if any(ch.isdigit() and ch!='0' for ch in t): continue
            (neigh if t in codes else junk)[t]+=n
print('junk (not a listed code):', dict(junk), sum(junk.values()))
print('another listed code (stray code on the right-name cluster):', dict(neigh), sum(neigh.values()))
# per-body member count distribution & bodies better/unchanged
cnt=collections.Counter()
for r in adj: pass
