import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import vfetch as V
OUT = sys.argv[1]
SINCE0, HI = 957854566, 958764334
since, n = SINCE0, 0
while True:
    out = os.path.join(OUT, f'p{n:04d}.json')
    ok, code = V.get(f'/v1/changes?entity=tender&since={since}&limit=1000', out)
    if not ok:
        sys.exit(f'page {n} since={since}: HTTP {code}')
    d = V.ld(out)
    print(n, since, len(d['events']), d['last_cursor'], d.get('more'), d.get('generation'), flush=True)
    n += 1
    if d['last_cursor'] is None or int(d['last_cursor']) >= HI or not d.get('more'):
        break
    since = int(d['last_cursor'])
