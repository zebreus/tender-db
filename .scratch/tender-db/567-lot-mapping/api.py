import json, os, sys, time, urllib.request, urllib.error
BASE = "https://tenders.zebreus.click"
DIR = os.path.dirname(os.path.abspath(__file__))
_last = [0.0]
def get(path, cache=None):
    if cache:
        p = os.path.join(DIR, cache)
        if os.path.exists(p):
            with open(p) as f:
                return json.load(f)
    tries = 0
    while True:
        wait = 1.6 - (time.time() - _last[0])
        if wait > 0:
            time.sleep(wait)
        _last[0] = time.time()
        req = urllib.request.Request(BASE + path, headers={"Accept": "application/json", "User-Agent": "tender-db-567-measure"})
        try:
            with urllib.request.urlopen(req, timeout=90) as r:
                data = json.load(r)
            break
        except urllib.error.HTTPError as e:
            if e.code == 408 and tries == 0:
                tries += 1
                continue
            if e.code in (429, 503) and tries < 3:
                tries += 1
                time.sleep(10)
                continue
            raise
    if cache:
        p = os.path.join(DIR, cache)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w") as f:
            json.dump(data, f)
    return data
