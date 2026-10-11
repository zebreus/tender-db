U1 = sys.argv[1]
rows = json.load(open(f'{U1}/adjudication-rows.json'))
for r in rows:
    if r['code']=='TAJ' and r['side']=='ppon': r['org_id']=30917612
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
    return re.sub(r'[^A-Z0-9]','', raw.upper())
def total(org): return sum(mc[org].values())
bodies = collections.defaultdict(lambda: {'reg':set(),'ppon':set()})
for r in rows:
    if r['final']=='SAME':
        bodies[r['body']]['reg' if r['side']=='registry' else 'ppon'].add(r['org_id'])
