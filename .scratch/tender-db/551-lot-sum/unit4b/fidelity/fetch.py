"""Independent fetch for the M1 fidelity check: /v1/tenders/{id} and every notice it names, into fid/raw.
One GET per 1.6 s (shares the port's stamp file so no two processes overlap); a 408 retried once."""
import json, os, ssl, sys, time, urllib.request, urllib.error
API = 'https://tenders.zebreus.click'
BASE = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/m551'
RAW = BASE + '/fid/raw'
STAMP = BASE + '/cache/.last_get'
CTX = ssl.create_default_context(cafile=os.environ.get('SSL_CERT_FILE') or '/root/.ccr/ca-bundle.crt')
NGET = 0
def wait():
    try: last = float(open(STAMP).read().strip() or 0)
    except Exception: last = 0
    g = time.time() - last
    if g < 1.6: time.sleep(1.6 - g)
    open(STAMP, 'w').write(repr(time.time()))
def get(path):
    global NGET
    t408 = 0
    while True:
        wait(); NGET += 1
        req = urllib.request.Request(API + path, headers={'Accept': 'application/json', 'User-Agent': 'tender-db issue-551 M1 fidelity'})
        try:
            with urllib.request.urlopen(req, timeout=90, context=CTX) as r:
                return json.loads(r.read())
        except urllib.error.HTTPError as e:
            if e.code == 408 and t408 == 0:
                t408 = 1; time.sleep(6); continue
            raise
def save(p, x):
    json.dump(x, open(p, 'w'), ensure_ascii=False)
for tid in sys.argv[1:]:
    tid = int(tid)
    x = get(f'/v1/tenders/{tid}')
    save(f'{RAW}/tenders/{tid}.json', x)
    nids = set(v['caused_by_notice_id'] for v in x.get('versions', []) if v.get('caused_by_notice_id'))
    for k in ('lot_results', 'bids', 'contracts'):
        nids |= {r['notice_id'] for r in x.get(k, []) if r.get('notice_id')}
    for nid in sorted(nids):
        p = f'{RAW}/notices/{nid}.json'
        if os.path.exists(p): continue
        save(p, get(f'/v1/notices/{nid}/content'))
    print(tid, 'notices', sorted(nids), 'gets so far', NGET, time.strftime('%H:%M:%SZ', time.gmtime()), flush=True)
