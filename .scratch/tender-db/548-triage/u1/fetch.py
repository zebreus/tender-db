# 548 U1: public GETs at <= 1 per 2.2 s, a 408 retried once. usage: fetch.py <name> <path> [<name> <path> ...]
import sys, time, urllib.request, urllib.error, datetime, os
D = '/home/user/tender-db/.scratch/tender-db/548-triage/'
B = 'https://tenders.zebreus.click'
args = sys.argv[1:]
last = 0.0
def get(path):
    global last
    wait = 2.2 - (time.time() - last)
    if wait > 0: time.sleep(wait)
    last = time.time()
    with open(D + 'api_requests.log', 'a') as log: log.write(f"{datetime.datetime.utcnow():%Y-%m-%dT%H:%M:%SZ} GET {path}\n")
    try:
        with urllib.request.urlopen(urllib.request.Request(B + path, headers={'User-Agent': '548-u1'}), timeout=60) as r: return r.status, r.read()
    except urllib.error.HTTPError as e: return e.code, e.read()
for name, path in zip(args[::2], args[1::2]):
    st, body = get(path)
    if st == 408: st, body = get(path)
    open(D + 'u1/api/' + name + '.json', 'wb').write(body)
    print(st, name, len(body))
