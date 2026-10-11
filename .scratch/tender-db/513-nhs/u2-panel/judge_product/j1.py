import json, collections, statistics, sys
U1 = sys.argv[1]
rows = json.load(open(f'{U1}/adjudication-rows.json'))
mc = collections.defaultdict(dict)
for line in open(f'{U1}/mention-counts-by-org.jsonl'):
    line=line.strip()
    if not line: continue
    d = json.loads(line)
    for org, raw, n in d['rows']:
        mc[org][raw] = mc[org].get(raw,0)+n
print('orgs with mention counts', len(mc))
codes = sorted({r['code'] for r in rows})
print('codes', len(codes), [c for c in codes if len(c)!=3])
cls = collections.Counter((r['side'], r['final']) for r in rows)
print(cls)
# org -> set of (code, final)
orgcodes = collections.defaultdict(set)
for r in rows: orgcodes[r['org_id']].add((r['code'], r['side'], r['final']))
multi_same = {o:v for o,v in orgcodes.items() if sum(1 for c,s,f in v if f=='SAME')>1}
print('orgs SAME for >1 code:', len(multi_same))
for o,v in list(multi_same.items())[:20]: print(' ', o, sorted(v))
cross = {o:v for o,v in orgcodes.items() if len({f for c,s,f in v})>1}
print('orgs with mixed classes:', len(cross))
for o,v in cross.items(): print(' ', o, sorted(v))
missing = [r['org_id'] for r in rows if r['org_id'] not in mc]
print('rows with org missing from mention counts', len(missing), missing[:10])
