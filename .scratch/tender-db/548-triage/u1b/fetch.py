# 548 U1b: public GETs, one at a time, >= 2.2 s apart (across hosts), a 408 retried once.
# usage: fetch.py <name> <url-or-/v1-path> [<name> <url> ...]   bodies -> u1b/api/<name>.json, log -> ../api_requests.log
import sys, time, urllib.request, urllib.error, datetime, os
D = '/home/user/tender-db/.scratch/tender-db/548-triage/'
B = 'https://tenders.zebreus.click'
os.makedirs(D + 'u1b/api', exist_ok=True)
args = sys.argv[1:]
last = 0.0


def get(url):
    global last
    wait = 2.2 - (time.time() - last)
    if wait > 0: time.sleep(wait)
    last = time.time()
    with open(D + 'api_requests.log', 'a') as log: log.write(f"{datetime.datetime.utcnow():%Y-%m-%dT%H:%M:%SZ} GET {url} (u1b)\n")
    try:
        with urllib.request.urlopen(urllib.request.Request(url, headers={'User-Agent': '548-u1b', 'Accept': 'application/json'}), timeout=60) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()
    except Exception as e:
        return 0, repr(e).encode()


for name, path in zip(args[::2], args[1::2]):
    url = path if path.startswith('http') else B + path
    if os.path.exists(D + 'u1b/api/' + name + '.json'):
        print('cached', name); continue
    st, body = get(url)
    if st == 408: st, body = get(url)
    if st == 200: open(D + 'u1b/api/' + name + '.json', 'wb').write(body)
    print(st, name, len(body), '' if st == 200 else body[:200])
