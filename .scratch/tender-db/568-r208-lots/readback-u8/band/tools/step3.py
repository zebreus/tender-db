"""Band stage step 3: read /v1/tenders/{id} for every band / path-V Tender in the changed set (no mover exists, so
these are the corrected Tenders the band holds), and fetch the chain notices' contents no cache holds."""
import gzip, json, os, re, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from fetch import get

RAW = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RB8 = os.path.dirname(RAW)
d = json.load(open(os.path.join(RAW, 'diff.json')))
ids = sorted(set(d['band_in_changed_set']) | {p['id'] for p in d['path_v'] if p['in_changed_set']})

N = [re.compile(r'(?:v1_)?notices?_(\d+)(?:_content)?\.json'), re.compile(r'n(\d+)\.json')]
idx = {}
for sub in ('c570', 'fresh', 'oos', 'c567', 'sets/live', 'fresh568', 'fresh570', 'u8fix', 'vx', 'band/notices'):
    for root, subdirs, files in os.walk(os.path.join(RB8, sub)):
        subdirs.sort()
        for f in sorted(files):
            m = next((m for m in (rx.fullmatch(f) for rx in N) if m), None)
            if m:
                idx.setdefault(int(m.group(1)), []).append(os.path.join(root, f))
bundle = set()
with gzip.open('/home/user/tender-db/.scratch/tender-db/568-r208-lots/u7/u7-cache.jsonl.gz', 'rt') as fh:
    for line in fh:
        r = json.loads(line)
        if r['kind'] != 'tender':
            bundle.add(r['id'])


def cached(n):
    for p in idx.get(n, []):
        try:
            c = json.load(open(p))
        except Exception:
            continue
        if isinstance(c, dict) and 'sections' in c:
            return p
    return 'bundle' if n in bundle else None


plan = {}
for i in ids:
    src = os.path.join(RAW, 'pathv', f't{i}.json')
    t = json.load(open(src)) if os.path.exists(src) else get(f'/v1/tenders/{i}', os.path.join(RAW, 'tenders', f't{i}.json'))
    chain = [v['caused_by_notice_id'] for v in sorted(t['versions'], key=lambda v: v['seq'])]
    plan[i] = {n: cached(n) for n in chain}
    print('tender', i, 'v', t['version'], 'chain', chain, 'cached', sum(1 for v in plan[i].values() if v), flush=True)
for i in ids:
    for n, p in plan[i].items():
        if p is None:
            c = get(f'/v1/notices/{n}/content', os.path.join(RAW, 'notices', f'notices_{n}_content.json'))
            plan[i][n] = 'fetched'
            print('notice', n, 'sections', len(c.get('sections', [])), flush=True)
json.dump({str(k): v for k, v in plan.items()}, open(os.path.join(RAW, 'plan.json'), 'w'), indent=1)
print('STEP3 DONE', len(ids))
