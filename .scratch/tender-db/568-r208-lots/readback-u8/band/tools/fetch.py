"""Rate-limited GET for the band stage: >= 1.6 s between GETs across processes (stamp file), a 408 retried once, nothing else retried."""
import json, os, subprocess, sys, time

BASE = 'https://tenders.zebreus.click'
GAP = 1.6
STAMP = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), '.last_get')
LOG = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), 'get.log')


def _wait():
    try:
        last = os.path.getmtime(STAMP)
    except FileNotFoundError:
        last = 0.0
    w = last + GAP - time.time()
    if w > 0:
        time.sleep(w)
    with open(STAMP, 'w') as f:
        f.write(str(time.time()))


def get(path, out):
    """GET BASE+path into out (atomic). Returns parsed JSON. Raises on non-200 (408 retried once)."""
    if os.path.exists(out):
        return json.load(open(out))
    for attempt in (1, 2):
        _wait()
        r = subprocess.run(['curl', '-sS', '--max-time', '90', '-o', out + '.part', '-w', '%{http_code}', BASE + path],
                           capture_output=True, text=True)
        code = r.stdout.strip()
        with open(LOG, 'a') as lg:
            lg.write(f'{time.strftime("%H:%M:%S", time.gmtime())} {code} {path}\n')
        if code == '200':
            os.replace(out + '.part', out)
            return json.load(open(out))
        if os.path.exists(out + '.part'):
            os.remove(out + '.part')
        if code == '408' and attempt == 1:
            continue
        raise RuntimeError(f'GET {path}: HTTP {code} {r.stderr.strip()}')
