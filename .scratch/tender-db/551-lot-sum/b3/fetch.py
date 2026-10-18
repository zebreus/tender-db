"""Issue 551 B3: re-read /v1/tenders/{id} (post-567/568/570 drain) for the 285 port-check ids (279 labelled + 6 out
of sample) and the 277 walked ids, then fetch every version / round notice's /v1/notices/{id}/content that no
earlier cache holds. Public API, at most one GET per 1.5 s (a stamp file), a 408 retried once, no ssh.

    python3 -I fetch.py tenders        # every id, fresh (writes RAW/tenders/<id>.json + .meta)
    python3 -I fetch.py notices        # notice contents missing from RAW/notices, the M1 cache and the B2 bundle

Notice contents are not changed by a refold (the parsed layer is untouched), so the M1 port cache
(scratchpad m551/cache/notices) and fixtures_data/inputs.tar.gz are reused read-only; only missing ones are fetched.
"""
import json, os, ssl, sys, tarfile, time, urllib.error, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
U4B = os.path.join(HERE, '..', 'unit4b')
SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'
RAW = SP + '/b3/raw'
M1_NOTICES = SP + '/m551/cache/notices'
API = 'https://tenders.zebreus.click'
GAP = 1.5
_ctx = ssl.create_default_context(cafile=os.environ.get('SSL_CERT_FILE') or None)


def ids():
    pc = json.load(open(os.path.join(U4B, 'M1-port-check.json')))
    out = {int(k) for k in pc}
    for f in ('walk.jsonl', 'walk_fallback.jsonl'):
        for line in open(os.path.join(U4B, f)):
            r = json.loads(line)
            if 'id' in r:
                out.add(int(r['id']))
    return sorted(out)


def wait():
    stamp = RAW + '/.last_get'
    try:
        last = float(open(stamp).read())
    except (OSError, ValueError):
        last = 0.0
    d = time.time() - last
    if d < GAP:
        time.sleep(GAP - d)
    open(stamp, 'w').write(repr(time.time()))


def get(path):
    t408 = tnet = 0
    while True:
        wait()
        req = urllib.request.Request(API + path, headers={'Accept': 'application/json', 'User-Agent': 'tender-db issue-551 B3'})
        try:
            with urllib.request.urlopen(req, timeout=90, context=_ctx) as r:
                code, body = r.status, r.read()
        except urllib.error.HTTPError as e:
            code, body = e.code, e.read()
        except (urllib.error.URLError, TimeoutError, ConnectionError, OSError) as e:
            tnet += 1
            if tnet <= 2:
                time.sleep(5)
                continue
            return 'net', repr(e).encode()
        if code == 408 and t408 == 0:
            t408 += 1
            time.sleep(6)
            continue
        return code, body


def save(kind, key, path):
    code, body = get(path)
    if code != 200:
        print('FAIL', kind, key, code, body[:200], flush=True)
        return False
    p = f'{RAW}/{kind}/{key}.json'
    json.loads(body)
    open(p + '.tmp', 'wb').write(body)
    os.replace(p + '.tmp', p)
    open(f'{RAW}/{kind}/{key}.meta', 'w').write(json.dumps({'fetched_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), 'path': path}))
    return True


def bundle_notices():
    with tarfile.open(os.path.join(U4B, 'fixtures_data', 'inputs.tar.gz')) as t:
        return {int(m.name.split('/')[-1][:-5]) for m in t.getmembers() if m.name.startswith('cache/notices/')}


def needed_notices(x):
    seq = [v['caused_by_notice_id'] for v in sorted(x.get('versions', []), key=lambda v: v.get('seq') or 0) if v.get('caused_by_notice_id')]
    extra = set()
    for k in ('lot_results', 'bids', 'contracts'):
        extra |= {r['notice_id'] for r in x.get(k, []) if r.get('notice_id')}
    return set(seq) | extra


def main():
    what = sys.argv[1]
    if what == 'tenders':
        todo = ids()
        if len(sys.argv) > 2 and sys.argv[2] == '--missing':
            todo = [t for t in todo if not os.path.exists(f'{RAW}/tenders/{t}.json')]
        print('tenders', len(todo), flush=True)
        for i, t in enumerate(todo):
            save('tenders', t, f'/v1/tenders/{t}')
            if i % 50 == 0:
                print(i, t, flush=True)
    elif what == 'notices':
        have = bundle_notices() | {int(f[:-5]) for f in os.listdir(M1_NOTICES) if f.endswith('.json')}
        have |= {int(f[:-5]) for f in os.listdir(RAW + '/notices') if f.endswith('.json')}
        need = set()
        for t in ids():
            p = f'{RAW}/tenders/{t}.json'
            if os.path.exists(p):
                need |= needed_notices(json.load(open(p)))
        todo = sorted(need - have)
        print('notices needed', len(need), 'missing', len(todo), flush=True)
        for i, n in enumerate(todo):
            save('notices', n, f'/v1/notices/{n}/content')
            if i % 50 == 0:
                print(i, n, flush=True)
    print('done', flush=True)


if __name__ == '__main__':
    main()
