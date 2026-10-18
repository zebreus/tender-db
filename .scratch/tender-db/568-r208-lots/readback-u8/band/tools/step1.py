"""Band stage step 1: page /v1/changes (tender) 957854566 -> 958764334, the >= EUR 1 bn band, and the 17 path-V heads."""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from fetch import get

RAW = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASELINE = '/home/user/tender-db/.scratch/tender-db/568-r208-lots/drain/baseline-pre568u8-570.json'
LO, HI = 957854566, 958764334
BAND_MIN = 100_000_000_000

# changes
since, n = LO, 0
while True:
    d = get(f'/v1/changes?entity=tender&since={since}&limit=1000', os.path.join(RAW, 'changes', f'p{n:04d}.json'))
    print('changes', n, since, len(d['events']), d.get('last_cursor'), d.get('more'), d.get('generation'),
          d.get('ignored_filters'), d.get('reset'), flush=True)
    if d.get('reset'):
        sys.exit(f'reset {d.get("reset")}')
    n += 1
    if d.get('last_cursor') is None or int(d['last_cursor']) >= HI or not d.get('more'):
        break
    since = int(d['last_cursor'])

# band
cursor, n = '', 0
while True:
    d = get(f'/v1/tenders?min_value={BAND_MIN}&limit=1000' + (f'&cursor={cursor}' if cursor else ''),
            os.path.join(RAW, 'bandpages', f'b{n:04d}.json'))
    print('band', n, len(d['items']), d.get('more'), d.get('next_cursor'), d.get('ignored_filters'), flush=True)
    n += 1
    if not d.get('more'):
        break
    cursor = d['next_cursor']

# path-V heads
base = json.load(open(BASELINE))
for i in sorted(int(k) for k in base['path_v']):
    t = get(f'/v1/tenders/{i}', os.path.join(RAW, 'pathv', f't{i}.json'))
    print('pathv', i, t.get('version'), t.get('value'), flush=True)
print('STEP1 DONE')
