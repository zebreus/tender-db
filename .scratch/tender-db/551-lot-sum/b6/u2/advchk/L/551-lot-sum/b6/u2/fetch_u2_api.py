"""Issue 551 B6 U2: the public-API reads the provisional L_changed needs (run once; run_u2.py then works offline).

    python3 -I -B fetch_u2_api.py changes      page GET /v1/changes?since=958787061&entity=tender&limit=1000 to the head
    python3 -I -B fetch_u2_api.py notices      find, among the notices the 3,502 fresh tender reads name, the first one
                                               whose ingested_at >= 2026-10-18T20:30:00Z (binary search; ingested_at is
                                               assigned at insert in id order, store lib.rs import_lag), plus a margin of
                                               probes either side of the boundary

Rules (the U2 task): public API only (https://tenders.zebreus.click), GET only, at most one GET every 2 s (this
process's own stamp, and never sooner than 2 s after the shared B5 stamp other fetchers write), a 408 retried ONCE and
never more, a transport error retried once; every attempt logged to api_requests.log as
'<UTC iso> GET <path> rc=<code>'. Bodies are saved verbatim under api/ with a manifest (path, status, sent_at, sha256).
No /v1/sql, no POST.
"""
import gzip
import hashlib
import importlib.util
import json
import os
import ssl
import sys
import time
import urllib.error
import urllib.request

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
B6 = os.path.dirname(HERE)
ROOT = os.path.dirname(B6)
API = 'https://tenders.zebreus.click'
LOG = os.path.join(HERE, 'api_requests.log')
OUT = os.path.join(HERE, 'api')
MANIFEST = os.path.join(OUT, 'manifest.json')
STAMP = os.path.join(HERE, '.last_get')
SHARED_STAMP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/b5/.last_get'
GAP = 2.0
RETRY_408_SLEEP = 6.0
SINCE = 958787061
CUTOFF = '2026-10-18T20:30:00Z'
SERVED_LSA = os.path.join(ROOT, 'b5', 'investigate', 'served_lsa_rows.json')


def iso(t):
    return time.strftime('%Y-%m-%dT%H:%M:%S', time.gmtime(t)) + f'.{int((t % 1) * 1000):03d}Z'


def _last(p):
    try:
        with open(p) as fh:
            return float(fh.read().strip() or 0)
    except (OSError, ValueError):
        return 0.0


def _wait():
    last = max(_last(STAMP), _last(SHARED_STAMP))
    d = time.time() - last
    if d < GAP:
        time.sleep(GAP - d)
    with open(STAMP, 'w') as fh:
        fh.write(repr(time.time()))


def _urlopen(path):
    ca = os.environ.get('SSL_CERT_FILE')
    ctx = ssl.create_default_context(cafile=ca) if ca and os.path.exists(ca) else ssl.create_default_context()
    req = urllib.request.Request(API + path, headers={'Accept': 'application/json',
                                                      'User-Agent': 'tender-db issue-551 B6 U2'})
    try:
        with urllib.request.urlopen(req, timeout=90, context=ctx) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()


def get(path):
    """-> (status, body, sent_at). A 408 is retried once and never more; a transport error once."""
    n408 = nnet = 0
    while True:
        _wait()
        t = time.time()
        try:
            code, body = _urlopen(path)
        except (urllib.error.URLError, TimeoutError, ConnectionError, OSError) as e:
            with open(LOG, 'a') as fh:
                fh.write(f'{iso(t)} GET {path} rc=net\n')
            nnet += 1
            if nnet <= 1:
                time.sleep(5)
                continue
            return 'net', repr(e).encode(), iso(t)
        with open(LOG, 'a') as fh:
            fh.write(f'{iso(t)} GET {path} rc={code}\n')
        if code == 408 and n408 == 0:
            n408 += 1
            time.sleep(RETRY_408_SLEEP)
            continue
        return code, body, iso(t)


def load_manifest():
    if os.path.exists(MANIFEST):
        with open(MANIFEST) as fh:
            return json.load(fh)
    return {'what': 'issue 551 B6 U2 public-API reads (fetch_u2_api.py)', 'changes': [], 'notices': {}}


def save_manifest(m):
    tmp = MANIFEST + '.tmp'
    with open(tmp, 'w') as fh:
        json.dump(m, fh, indent=1, sort_keys=True)
        fh.write('\n')
    os.replace(tmp, MANIFEST)


def save_body(rel, body):
    p = os.path.join(OUT, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'wb') as fh:
        fh.write(body)
    return hashlib.sha256(body).hexdigest()


def cmd_changes():
    m = load_manifest()
    if m['changes']:
        raise SystemExit('changes already fetched (api/manifest.json); refusing to re-page over a recorded run')
    since = SINCE
    i = 0
    while True:
        path = f'/v1/changes?since={since}&entity=tender&limit=1000'
        code, body, sent = get(path)
        rel = f'changes/{i:03d}.json'
        rec = {'i': i, 'path': path, 'status': code, 'sent_at': sent}
        if code != 200:
            rec['body'] = body[:300].decode('utf-8', 'replace')
            m['changes'].append(rec)
            save_manifest(m)
            raise SystemExit(f'changes page {i} {path}: {code}')
        d = json.loads(body)
        rec.update(file=rel, sha256=save_body(rel, body), n=len(d['events']), last_cursor=d['last_cursor'],
                   more=d['more'], generation=d.get('generation'), reset=d.get('reset'),
                   first_changed_at=(d['events'][0]['changed_at'] if d['events'] else None),
                   last_changed_at=(d['events'][-1]['changed_at'] if d['events'] else None))
        m['changes'].append(rec)
        save_manifest(m)
        print(i, rec['n'], rec['last_cursor'], rec['last_changed_at'], rec['more'], flush=True)
        if d.get('reset'):
            raise SystemExit(f'reset {d["reset"]}')
        if not d['more']:
            break
        since = int(d['last_cursor'])
        i += 1


def named_notices():
    spec = importlib.util.spec_from_file_location('rule_v2_g_u2fetch', os.path.join(B6, 'rule_v2_g.py'))
    R = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(R)
    with open(SERVED_LSA) as fh:
        ids = sorted(int(r[0]) for r in json.load(fh) if int(r[0]) != 179732)
    out = set()
    for t in ids:
        out |= set(R.version_notice_ids(R.tender_detail(t)))
    return sorted(out)


def cmd_notices(margin=6):
    m = load_manifest()
    L = named_notices()

    def ingested(nid):
        rec = m['notices'].get(str(nid))
        if rec is None:
            path = f'/v1/notices/{nid}'
            code, body, sent = get(path)
            rec = {'path': path, 'status': code, 'sent_at': sent}
            if code == 200:
                rel = f'notices/{nid}.json'
                rec.update(file=rel, sha256=save_body(rel, body), ingested_at=json.loads(body).get('ingested_at'))
            else:
                rec['body'] = body[:300].decode('utf-8', 'replace')
            m['notices'][str(nid)] = rec
            save_manifest(m)
            print(nid, code, rec.get('ingested_at'), flush=True)
        if rec['status'] != 200:
            raise SystemExit(f'notice {nid}: {rec["status"]}')
        return rec['ingested_at']

    def after(nid):
        return ingested(nid) >= CUTOFF          # ISO 'YYYY-MM-DDTHH:MM:SSZ' strings compare in time order

    lo, hi = 0, len(L)                          # first index whose notice was ingested at/after the cutoff
    if not after(L[-1]):
        lo = len(L)
    else:
        hi = len(L) - 1
        while lo < hi:
            mid = (lo + hi) // 2
            if after(L[mid]):
                hi = mid
            else:
                lo = mid + 1
    for j in range(max(0, lo - margin), min(len(L), lo + margin)):
        ingested(L[j])
    m['boundary'] = {'cutoff': CUTOFF, 'n_named': len(L), 'first_index_at_or_after': lo,
                     'first_id_at_or_after': L[lo] if lo < len(L) else None,
                     'last_id_before': L[lo - 1] if lo > 0 else None, 'margin': margin}
    save_manifest(m)
    print(json.dumps(m['boundary']))


if __name__ == '__main__':
    a = sys.argv[1:]
    if a == ['changes']:
        cmd_changes()
    elif a == ['notices']:
        cmd_notices()
    else:
        raise SystemExit(__doc__)
