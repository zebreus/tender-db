"""570 read-back (e)/(g): page /v1/changes?entity=<E>&since=<A>&limit=1000 up to cursor B into OUT/pNNNN.json.

One GET per 1.6 s (>= 1.5 s), a 408 retried once, nothing else retried. Resumes from the pages already in OUT.
Stops when a page's last_cursor >= B or `more` is false. Usage:
  python3 -I -B page_changes.py ENTITY OUT [A B]      (default A=957854566 B=958764334)
"""
import json
import os
import subprocess
import sys
import time

ENTITY, OUT = sys.argv[1], sys.argv[2]
SINCE0, HI = (int(sys.argv[3]), int(sys.argv[4])) if len(sys.argv) > 4 else (957854566, 958764334)
BASE = 'https://tenders.zebreus.click'
GAP = 1.6
os.makedirs(OUT, exist_ok=True)
files = sorted(f for f in os.listdir(OUT) if f.startswith('p') and f.endswith('.json'))
since, n, last = SINCE0, 0, 0.0
for f in files:
    d = json.load(open(os.path.join(OUT, f)))
    since = int(d['last_cursor'])
    n += 1
    if since >= HI or not d.get('more'):
        print('already complete', f, d['last_cursor'], d.get('more'))
        sys.exit(0)
log = open(os.path.join(OUT, 'page.log'), 'a')
while True:
    out = os.path.join(OUT, f'p{n:04d}.json')
    for attempt in (1, 2):
        w = last + GAP - time.time()
        if w > 0:
            time.sleep(w)
        last = time.time()
        r = subprocess.run(['curl', '-sS', '--max-time', '60', '-o', out + '.part', '-w', '%{http_code}',
                            f'{BASE}/v1/changes?entity={ENTITY}&since={since}&limit=1000'],
                           capture_output=True, text=True)
        code = r.stdout.strip()
        if code == '200':
            os.replace(out + '.part', out)
            break
        if os.path.exists(out + '.part'):
            os.remove(out + '.part')
        if code == '408' and attempt == 1:
            continue
        sys.exit(f'page since={since}: HTTP {code} {r.stderr}')
    d = json.load(open(out))
    ev = d['events']
    line = f"{n} since={since} events={len(ev)} last_cursor={d['last_cursor']} more={d.get('more')} gen={d.get('generation')} ignored={d.get('ignored_filters')} reset={d.get('reset')}"
    print(line, flush=True)
    log.write(line + '\n')
    log.flush()
    if d.get('reset'):
        sys.exit(f'reset: {d.get("reset")}')
    n += 1
    if d['last_cursor'] is None or int(d['last_cursor']) >= HI or not d.get('more'):
        break
    since = int(d['last_cursor'])
