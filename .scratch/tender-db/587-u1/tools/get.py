"""get.py NAME PATH -- one public GET to https://tenders.zebreus.click, logged to requests.log.
Budget 20 GETs total, >= 3 s between, never retried (408/429/any non-200 recorded and left)."""
import json, os, subprocess, sys, time, fcntl
HERE = '/home/user/tender-db/.scratch/tender-db/587-u1'
LOG = os.path.join(HERE, 'requests.log')
STATE = os.path.join(HERE, 'raw', 'get_state.json')
BUDGET, GAP = 20, 3.0
def main():
    name, path = sys.argv[1], sys.argv[2]
    lock = open(os.path.join(HERE, 'raw', 'get.lock'), 'a'); fcntl.flock(lock, fcntl.LOCK_EX)
    st = json.load(open(STATE)) if os.path.exists(STATE) else {'used': 0, 'last': 0, 'names': []}
    if st['used'] >= BUDGET:
        sys.exit('REFUSED: GET budget used')
    if name in st['names']:
        sys.exit(f'REFUSED: {name} already fetched (never retried)')
    wait = GAP - (time.time() - st['last'])
    if wait > 0:
        time.sleep(wait)
    out = os.path.join(HERE, 'raw', 'get', f'{name}.json')
    url = 'https://tenders.zebreus.click' + path
    t0 = time.time()
    p = subprocess.run(['curl', '-sS', '-o', out, '-w', '%{http_code} %{size_download}', '--max-time', '60', url],
                       capture_output=True, text=True)
    t1 = time.time()
    st['used'] += 1; st['last'] = t1; st['names'].append(name)
    json.dump(st, open(STATE, 'w'), indent=1)
    code, size = (p.stdout.split() + ['?', '?'])[:2]
    line = (f"{time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(t0))} GET {path} -> {code} rc={p.returncode} bytes={size} "
            f"secs={t1 - t0:.2f} n={st['used']}/{BUDGET} name={name}")
    open(LOG, 'a').write(line + '\n')
    print(line)
main()
