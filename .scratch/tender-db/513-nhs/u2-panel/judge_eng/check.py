import json, sys, statistics, collections
d = sys.argv[1]
rows = json.load(open(d + '/adjudication-rows.json'))
# mention counts
mc = collections.defaultdict(dict)
for line in open(d + '/mention-counts-by-org.jsonl'):
    line=line.strip()
    if not line: continue
    j = json.loads(line)
    for org, raw, n in j['rows']:
        mc[org][raw] = mc[org].get(raw, 0) + n
print('orgs with counts', len(mc))
# TAJ check
for r in rows:
    if r['code'] in ('TAJ','QMM') and r['side']=='ppon':
        print('PPON row', r['code'], r['org_id'], r['name'], r['final'])
lst = json.load(open(d + '/nhs-listing-2236.json'))
print(type(lst), (list(lst.keys())[:10] if isinstance(lst, dict) else len(lst)))
