"""Verifier's own rate-limited fetcher: one GET per >= 1.6 s across processes (a lock + stamp file), a 408 retried
once, nothing else retried. Existing files are never re-fetched."""
import fcntl, json, os, subprocess, time
BASE = 'https://tenders.zebreus.click'
GAP = 1.6
STAMP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/ext-verify/tools/.last_get'
LOG = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/ext-verify/tools/gets.log'
gets = 0
def get(path, out):
    """True when out holds a 200 body; (False, code) otherwise."""
    global gets
    if os.path.exists(out):
        return True, 'cached'
    code = None
    for attempt in (1, 2):
        with open(STAMP + '.lock', 'w') as lk:
            fcntl.flock(lk, fcntl.LOCK_EX)
            try:
                last = float(open(STAMP).read().strip())
            except Exception:
                last = 0.0
            w = last + GAP - time.time()
            if w > 0:
                time.sleep(w)
            t0 = time.time()
            with open(STAMP, 'w') as fh:
                fh.write(str(t0))
            r = subprocess.run(['curl', '-sS', '--max-time', '60', '-o', out + '.part', '-w', '%{http_code}',
                                BASE + path], capture_output=True, text=True)
        gets += 1
        code = r.stdout.strip()
        with open(LOG, 'a') as fh:
            fh.write(f"{time.strftime('%H:%M:%S', time.gmtime(t0))} {code} {path}\n")
        if code == '200':
            os.replace(out + '.part', out)
            return True, code
        if os.path.exists(out + '.part'):
            os.remove(out + '.part')
        if code == '408' and attempt == 1:
            continue
        return False, code
    return False, code
def ld(p):
    with open(p, encoding='utf-8') as fh:
        return json.load(fh)
