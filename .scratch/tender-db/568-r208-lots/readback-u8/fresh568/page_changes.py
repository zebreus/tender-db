import json, os, subprocess, sys, time
OUT = sys.argv[1]
SINCE0, HI = 957854566, 958764334
BASE = 'https://tenders.zebreus.click'
files = sorted(f for f in os.listdir(OUT) if f.startswith('p') and f.endswith('.json'))
since = SINCE0
n = 0
last = 0.0
# resume from existing pages
for f in files:
    d = json.load(open(os.path.join(OUT, f)))
    since = int(d['last_cursor']); n += 1
    if int(d['last_cursor']) >= HI or not d.get('more'):
        print('already complete', f, d['last_cursor'], d.get('more')); sys.exit(0)
while True:
    out = os.path.join(OUT, f'p{n:04d}.json')
    for attempt in (1, 2):
        w = last + 1.6 - time.time()
        if w > 0: time.sleep(w)
        last = time.time()
        r = subprocess.run(['curl', '-sS', '--max-time', '60', '-o', out + '.part', '-w', '%{http_code}',
                            f'{BASE}/v1/changes?entity=tender&since={since}&limit=1000'], capture_output=True, text=True)
        code = r.stdout.strip()
        if code == '200':
            os.replace(out + '.part', out); break
        if os.path.exists(out + '.part'): os.remove(out + '.part')
        if code == '408' and attempt == 1: continue
        sys.exit(f'page since={since}: HTTP {code} {r.stderr}')
    d = json.load(open(out))
    ev = d['events']
    print(n, since, len(ev), d['last_cursor'], d.get('more'), d.get('generation'), flush=True)
    n += 1
    if d['last_cursor'] is None or int(d['last_cursor']) >= HI or not d.get('more'):
        break
    since = int(d['last_cursor'])
