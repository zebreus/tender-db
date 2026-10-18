"""Issue 551 B5, step 3: cohort parity, the SERVED state after the id-cohort drain against the port of record.

docs/operations.md, "Path W v2", B5's runbook steps 2 and 3. The port of record is b3/rule_v2_b3.py, sha256 pinned
below and loaded unmodified. Only its cache paths are pointed at this session's scratchpad (module globals, set after
import), and its reader is replaced by a layered one, so B3's raw tender reads are never written.

    python3 -I -B cohort_parity.py ids          build the 563-id cohort, write b5/cohort_ids.json, print it
    python3 -I -B cohort_parity.py aside        copy B3's raw tender reads aside, with a sha256 manifest. A file
                                                already there is checked, never overwritten. The source is read only.
    python3 -I -B cohort_parity.py fetch [--missing]
                                                run `aside` first. Then re-read /v1/tenders/{id} for the cohort into
                                                B5's OWN directory (scratchpad b5/raw/tenders), then fetch every notice
                                                content no cache holds into b5/raw/notices. At most 1 GET per 1.5 s,
                                                one stamp file shared with confirm_draw.py; a 408 is retried once.
    python3 -I -B cohort_parity.py compare [--out FILE] [--tenders DIR ...] [--b3-tenders DIR ...]
                                         [--b3-outcomes FILE] [--ids FILE]
                                                the comparison. It is offline: it reads only the caches.
    python3 -I -B cohort_parity.py selftest     offline test: B3's cached reads, a simulated served state, planted
                                                disagreements

Exit codes for `compare`:
  0  0 disagreements on the whole cohort.
  1  any disagreement. Its kinds are listed in the output: ship/refuse, S, currency, n, vouched_by, moved-since-B3.
  2  incomplete, with no disagreement: an id unread, or the port raised on an id.

THE COMPARISON. This implements operations.md step 3 exactly. A served `value_derivation.method = lot-sum-awards` is a
ship. Anything else is a refusal.
  - Head served lot-sum-awards. The port ships when all of these hold:
      * v1's reason is `W` or `W/G0-served`. After the drain the served value IS the sum, so the port's G0-against-
        served check refuses every real ship. That is why `W/G0-served` counts here.
      * `v2_refusals` is empty (no R1-R10).
    The ship then agrees only when the served value {cents, currency} and `value_derivation.lots` equal the port's
    S, currency and n.
    `vouched_by`:
      * the port's own `vouched_by` agrees;
      * a total the port also counts (`facts()['vouches']`) is B2's recorded name-level difference. It is listed,
        and it is not a stop;
      * anything else is a `vouched_by` disagreement (exit 1).
  - Any other head: `v2_decision` as it stands. Its served head is still the max election, so `W/G0-served` there is
    a real refusal. A port ship here is a ship/refuse disagreement: the port ships and the Rust refuses.
  - Cross-check with B3's outcomes (b3/results/api_port.jsonl.gz, decided on B3's reads; 7511890 from B3's
    verifier). An id whose ship/refuse moved since B3 is explained only when the B5 read names a notice the B3 read
    did not (a notice published since). Otherwise it counts as a disagreement (`moved-since-B3`).
"""
import argparse
import copy
import gzip
import hashlib
import importlib.util
import json
import os
import shutil
import ssl
import subprocess
import sys
import time
import traceback
import urllib.error
import urllib.request

sys.dont_write_bytecode = True

HERE = os.path.dirname(os.path.abspath(__file__))           # .../551-lot-sum/b5
ROOT = os.path.dirname(HERE)                                 # .../551-lot-sum
B3 = os.path.join(ROOT, 'b3')
U4B = os.path.join(ROOT, 'unit4b')
PORT = os.path.join(B3, 'rule_v2_b3.py')
PORT_SHA = '5400bc555f15ba09e852afe132009a900a2e5a3634e7546ff703fc9a8a89ade5'   # issue 551 "B3 -- port parity PASSED"
GEN_B3 = os.path.join(B3, 'gen_b3.py')
GEN_B3_SHA = '2e9635bae0a49b7924780d7829d549ec9edf5180a69c37d9b6062aa83cc6d9de'  # recorded only (fixtures, not decisions)
LABELS = os.path.join(ROOT, 'postdrain', 'read_result.json')
LABELS_SHA = '048a62c1981b5c6271a34b471807a8fbc754c6a38225cb32c91d5bdd471a4306'
WALKS = [os.path.join(U4B, 'walk.jsonl'), os.path.join(U4B, 'walk_fallback.jsonl')]
B3_OUTCOMES = os.path.join(B3, 'results', 'api_port.jsonl.gz')
COHORT_FILE = os.path.join(HERE, 'cohort_ids.json')
PX2_OOS = [585917, 5995030, 467176, 294968, 7256284, 5977060]     # B3's oos-6 (B3-parity.json), operations.md step 2
EXTRA = [7511890]                                                 # b3/B3-result.md, the 570 check
COHORT_SIZE = 563
# 7511890 is not in api_port.jsonl.gz. B3's verifier decided it separately (b3/B3-result.md; scratchpad
# b3v/run7511890/port.json): W/T' (not a v1 fire), S PLN 339,546.48, n = 4.
B3_EXTRA_OUTCOMES = {7511890: {'ship': False, 'first': "W/T'", 'S': 33954648, 'cur': 'PLN', 'n': 4,
                               'source': 'b3/B3-result.md (verifier, scratchpad b3v/run7511890/port.json)'}}

SP = os.environ.get('B5_SCRATCH', '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad')
B3_TENDERS = os.path.join(SP, 'b3', 'raw', 'tenders')               # B3's reads: NEVER written by this script
B3_VERIFY_TENDERS = os.path.join(SP, 'b3', 'verify', 'raw', 'tenders')  # 7511890's B3-era read
B5_DIR = os.path.join(SP, 'b5')
ASIDE = os.path.join(B5_DIR, 'b3_tenders_aside')
B5_TENDERS = os.path.join(B5_DIR, 'raw', 'tenders')
B5_NOTICES = os.path.join(B5_DIR, 'raw', 'notices')
STAMP = os.path.join(B5_DIR, '.last_get')                           # shared with confirm_draw.py
# notice contents: B3's order (b3/raw, the M1 cache, B2's bundle), then 7511890's u570 notices, then B5's fetches.
NOTICE_LAYERS = [os.path.join(SP, 'b3', 'raw', 'notices'), os.path.join(SP, 'm551', 'cache', 'notices'),
                 os.path.join(SP, 'b3', 'bundle', 'cache', 'notices'), os.path.join(SP, 'b3', 'verify', 'raw', 'notices'),
                 B5_NOTICES]

API = 'https://tenders.zebreus.click'
GAP = 1.5
RETRY_408_SLEEP = 6.0
NET_RETRIES, NET_SLEEP = 2, 5.0


def sha256_file(p):
    with open(p, 'rb') as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def ld(p):
    with open(p, encoding='utf-8') as fh:
        return json.load(fh)


def dump(p, obj):
    os.makedirs(os.path.dirname(os.path.abspath(p)), exist_ok=True)
    tmp = p + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as fh:
        json.dump(obj, fh, indent=1, ensure_ascii=False, default=str)
        fh.write('\n')
    os.replace(tmp, p)


def now():
    return time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())


# ------------------------------------------------------------------------------------------------ the port of record
def load_port():
    got = sha256_file(PORT)
    if got != PORT_SHA:
        raise SystemExit(f'cohort_parity: {PORT} sha256 {got} is not the port of record {PORT_SHA}')
    spec = importlib.util.spec_from_file_location('rule_v2_b3_b5', PORT)
    R = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(R)
    # The port names this session's scratchpad. Point it at SP (same paths by default), so a fresh session moves
    # only B5_SCRATCH and the port file stays byte-identical.
    R.SCRATCH = os.path.join(SP, 'm551')
    R.CACHE = os.path.join(R.SCRATCH, 'cache')
    R.RATES_DIR = os.path.join(R.SCRATCH, 'rates')
    R.B3_RAW = os.path.join(SP, 'b3', 'raw')
    R.B3_BUNDLE = os.path.join(SP, 'b3', 'bundle', 'cache')
    return R


def install_reader(R, tender_dirs, notice_dirs=None):
    """Replace the port's `_cached`: tenders from `tender_dirs` (first hit wins), notices from `notice_dirs`. No GET.
    `tender_detail` / `notice_content` / `facts` look `_cached` up at call time, so they read through this."""
    notice_dirs = NOTICE_LAYERS if notice_dirs is None else notice_dirs

    def _cached(kind, key, path, refresh=False):
        for base in (tender_dirs if kind == 'tenders' else notice_dirs):
            p = os.path.join(base, f'{int(key)}.json')
            if os.path.exists(p):
                return ld(p)
        raise R.FetchError(path, 'not cached (b5 cohort_parity reads caches only; run `cohort_parity.py fetch`)')
    R._cached = _cached


# ------------------------------------------------------------------------------------------------ the cohort
def cohort_sources():
    if sha256_file(LABELS) != LABELS_SHA:
        raise SystemExit(f'cohort_parity: {LABELS} changed (sha256 pinned {LABELS_SHA})')
    lab = sorted(int(k) for k in ld(LABELS)['final'])
    walked = []
    for f in WALKS:
        with open(f, encoding='utf-8') as fh:
            for line in fh:
                if not line.strip():
                    continue
                r = json.loads(line)
                if 'id' in r:                     # meta / end rows carry no id (b3/fetch.py ids())
                    walked.append(int(r['id']))
    return lab, walked


def build_cohort():
    lab, walked = cohort_sources()
    assert len(lab) == 279 and len(set(walked)) == len(walked) == 277, (len(lab), len(walked))
    ids = sorted(set(lab) | set(walked) | set(PX2_OOS) | set(EXTRA))
    assert len(ids) == COHORT_SIZE, len(ids)
    b3_ids = set()
    with gzip.open(B3_OUTCOMES, 'rt', encoding='utf-8') as fh:
        for line in fh:
            b3_ids.add(json.loads(line)['id'])
    assert set(ids) == b3_ids | set(EXTRA), 'the cohort is not B3\'s 562 plus 7511890'
    oos_b3 = sorted(r for r in b3_ids if r in PX2_OOS)
    assert oos_b3 == sorted(PX2_OOS)
    return ids, lab, walked


def cohort_ids(path=None):
    """The cohort: a JSON list or a cohort_ids.json-shaped object at `path`, else the built one (checked against
    b5/cohort_ids.json when that exists)."""
    if path:
        d = ld(path)
        return sorted(int(i) for i in (d['ids'] if isinstance(d, dict) else d))
    ids, _, _ = build_cohort()
    if os.path.exists(COHORT_FILE) and ld(COHORT_FILE)['ids'] != ids:
        raise SystemExit(f'cohort_parity: {COHORT_FILE} differs from the cohort its sources build')
    return ids


def cmd_ids(args):
    ids, lab, walked = build_cohort()
    dry = json.dumps({'kind': 'refold-lot-sum', 'tender_ids': ids}, separators=(',', ':'))
    wet = json.dumps({'kind': 'refold-lot-sum', 'dry_run': False, 'expect': 'N', 'tender_ids': ids},
                     separators=(',', ':')).replace('"expect":"N"', '"expect":N')
    out = {
        'what': 'issue 551 B5 step 2: the id cohort for refold-lot-sum tender_ids (docs/operations.md "Path W v2")',
        'built_at': now(),
        'count': len(ids),
        'sources': {
            'labelled_279': {'file': 'postdrain/read_result.json keys of "final"', 'sha256': LABELS_SHA, 'count': len(lab)},
            'walked_277': {'files': {os.path.relpath(f, ROOT): sha256_file(f) for f in WALKS},
                           'rule': 'id of every row that has one (meta / end rows skipped)', 'count': len(walked)},
            'px2_oos_6': PX2_OOS,
            'extra': EXTRA,
        },
        'equals_b3_562_plus_7511890': True,
        'ids_sha256': hashlib.sha256(json.dumps(ids).encode()).hexdigest(),
        'ids': ids,
        'dry_job': dry,
        'wet_job_template': wet,
    }
    dump(args.out or COHORT_FILE, out)
    print(f'cohort: {len(ids)} ids -> {args.out or COHORT_FILE}')
    print(','.join(str(i) for i in ids))
    return 0


# ------------------------------------------------------------------------------------------------ reads (online)
def _urlopen(path):
    """One GET of API+path -> (status, body). Transport errors raise OSError-family exceptions."""
    ca = os.environ.get('SSL_CERT_FILE')
    ctx = ssl.create_default_context(cafile=ca) if ca and os.path.exists(ca) else ssl.create_default_context()
    req = urllib.request.Request(API + path, headers={'Accept': 'application/json', 'User-Agent': 'tender-db issue-551 B5'})
    try:
        with urllib.request.urlopen(req, timeout=90, context=ctx) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()


class Http:
    """At most one GET per `gap` seconds across processes (a stamp file), a 408 retried ONCE, a transport error
    retried twice. `transport(path) -> (status, body)` is replaceable (selftest)."""

    def __init__(self, stamp=STAMP, gap=GAP, transport=None, retry_408_sleep=None, log=None):
        self.stamp, self.gap = stamp, gap
        self.transport = transport or _urlopen
        self.retry_408_sleep = RETRY_408_SLEEP if retry_408_sleep is None else retry_408_sleep
        self.calls = []                     # (path, monotonic-ish wall time, status)
        self.log = log

    def _wait(self):
        os.makedirs(os.path.dirname(self.stamp), exist_ok=True)
        try:
            with open(self.stamp) as fh:
                last = float(fh.read().strip() or 0)
        except (OSError, ValueError):
            last = 0.0
        d = time.time() - last
        if d < self.gap:
            time.sleep(self.gap - d)
        with open(self.stamp, 'w') as fh:
            fh.write(repr(time.time()))

    def get(self, path):
        t408 = tnet = 0
        while True:
            self._wait()
            t = time.time()
            try:
                code, body = self.transport(path)
            except (urllib.error.URLError, TimeoutError, ConnectionError, OSError) as e:
                self.calls.append((path, t, 'net'))
                tnet += 1
                if tnet <= NET_RETRIES:
                    time.sleep(NET_SLEEP)
                    continue
                return 'net', repr(e).encode()
            self.calls.append((path, t, code))
            if code == 408 and t408 == 0:
                t408 += 1
                time.sleep(self.retry_408_sleep)
                continue
            return code, body


def _protected():
    return {os.path.realpath(p) for p in (B3_TENDERS, B3_VERIFY_TENDERS, ASIDE)} | \
           {os.path.realpath(p) for p in NOTICE_LAYERS if p != B5_NOTICES}


def save_read(http, dest_dir, key, path):
    """GET `path` into dest_dir/<key>.json (+ .meta). Refuses to write into B3's caches or the aside copy."""
    if os.path.realpath(dest_dir) in _protected():
        raise SystemExit(f'cohort_parity: refusing to write into {dest_dir} (a B3 cache or the aside copy)')
    code, body = http.get(path)
    if code != 200:
        return code, body[:300]
    json.loads(body)
    os.makedirs(dest_dir, exist_ok=True)
    p = os.path.join(dest_dir, f'{key}.json')
    with open(p + '.tmp', 'wb') as fh:
        fh.write(body)
    os.replace(p + '.tmp', p)
    with open(os.path.join(dest_dir, f'{key}.meta'), 'w') as fh:
        fh.write(json.dumps({'fetched_at': now(), 'path': path}))
    return 200, None


def aside(dst=ASIDE, srcs=(B3_TENDERS, B3_VERIFY_TENDERS)):
    """Copy B3's raw tender reads aside with a sha256 manifest. Never overwrites a file; a re-run re-checks."""
    man_p = os.path.join(dst, 'MANIFEST.json')
    files = {}
    for src in srcs:
        for f in sorted(os.listdir(src)):
            if f.endswith('.json') or f.endswith('.meta'):
                if f in files:
                    raise SystemExit(f'aside: {f} in two sources')
                files[f] = os.path.join(src, f)
    if os.path.exists(man_p):
        man = ld(man_p)
        bad, src_moved = [], []
        for f, rec in man['files'].items():
            if sha256_file(os.path.join(dst, f)) != rec['sha256']:
                bad.append(f)
            s = files.get(f)
            if s is not None and sha256_file(s) != rec['sha256']:
                src_moved.append(f)
        if bad:
            raise SystemExit(f'aside: {len(bad)} aside file(s) differ from the manifest: {bad[:5]}')
        return {'status': 'present', 'files': len(man['files']), 'source_changed_since_aside': src_moved}
    os.makedirs(dst, exist_ok=True)
    man = {'what': "B3's raw /v1/tenders reads, copied aside before B5's re-read (operations.md step 2)",
           'copied_at': now(), 'files': {}}
    for f, s in files.items():
        d = os.path.join(dst, f)
        if os.path.exists(d):
            raise SystemExit(f'aside: {d} exists without a manifest; refusing to overwrite')
        shutil.copy2(s, d)
        h = sha256_file(d)
        if h != sha256_file(s):
            raise SystemExit(f'aside: copy of {s} differs')
        man['files'][f] = {'sha256': h, 'src': s}
    dump(man_p, man)
    return {'status': 'copied', 'files': len(man['files'])}


def cmd_aside(args):
    r = aside()
    print(json.dumps(r))
    return 0 if not r.get('source_changed_since_aside') else 1


def needed_notices(R, ids, tender_dirs):
    install_reader(R, tender_dirs)
    need = set()
    for t in ids:
        try:
            x = R.tender_detail(t)
        except R.FetchError:
            continue
        need |= set(R.version_notice_ids(x))
    have = set()
    for base in NOTICE_LAYERS:
        if os.path.isdir(base):
            have |= {int(f[:-5]) for f in os.listdir(base) if f.endswith('.json')}
    return sorted(need), sorted(need - have)


def cmd_fetch(args, http=None):
    ids = cohort_ids(args.ids)
    print('aside:', json.dumps(aside()), flush=True)
    http = http or Http()
    fails = []
    if not args.notices_only:
        todo = [t for t in ids if not (args.missing and os.path.exists(os.path.join(B5_TENDERS, f'{t}.json')))]
        print('tenders', len(todo), flush=True)
        for i, t in enumerate(todo):
            code, err = save_read(http, B5_TENDERS, t, f'/v1/tenders/{t}')
            if code != 200:
                fails.append({'kind': 'tender', 'id': t, 'code': code, 'body': (err or b'').decode('utf-8', 'replace')})
                print('FAIL tender', t, code, flush=True)
            if i % 50 == 0:
                print(i, t, flush=True)
    R = load_port()
    need, missing = needed_notices(R, ids, [B5_TENDERS])
    print('notices needed', len(need), 'missing from every cache', len(missing), flush=True)
    for i, n in enumerate(missing):
        code, err = save_read(http, B5_NOTICES, n, f'/v1/notices/{n}/content')
        if code != 200:
            fails.append({'kind': 'notice', 'id': n, 'code': code, 'body': (err or b'').decode('utf-8', 'replace')})
            print('FAIL notice', n, code, flush=True)
        if i % 50 == 0:
            print(i, n, flush=True)
    dump(os.path.join(B5_DIR, 'raw', 'fetch_log.json'), {'at': now(), 'tenders': len(ids), 'notices_fetched': len(missing),
                                                         'fails': fails})
    print('done; fails', len(fails), flush=True)
    return 0 if not fails else 2


# ------------------------------------------------------------------------------------------------ the comparison
def served_of(x):
    v = x.get('value') or None
    d = x.get('value_derivation') or None
    return {'value': v, 'derivation': d, 'method': (d or {}).get('method')}


def port_decision(R, f, served_ship):
    """The port's decision, served-aware per operations.md step 3."""
    why = f['v1']['why']
    v1r = 'W' if why == 'W' else why.split(':')[0]
    refusals = R.v2_refusals(f)
    internal = None
    if served_ship:
        fires = v1r in ('W', 'W/G0-served')
        ship = fires and not refusals
        first = None if ship else (refusals[0] if fires else v1r)
        if fires:
            d = f['v1']['derived'] or {}
            if (d.get('cents'), d.get('currency'), d.get('lots')) != (f['S'], f['currency'], f['n']):
                internal = f"port derived {d} is not its S/currency/n {f['S']}/{f['currency']}/{f['n']}"
        rule = 'served-ship: v1 W or W/G0-served, no R1-R10'
    else:
        ship, first, _, _, _ = R.v2_decision(f)       # asserts derived == S/currency/n where v1 fires
        rule = 'v2_decision'
    return {'ship': bool(ship), 'first': first, 'S': f.get('S'), 'currency': f.get('currency'), 'n': f.get('n'),
            'v1_reason': v1r, 'refusals': refusals, 'vouched_by': f.get('vouched_by'),
            'vouches': [list(v) for v in (f.get('vouches') or [])], 'S_eur': f.get('S_eur'), 'rule': rule,
            'internal': internal}


def compare_one(served, port):
    """-> (disagreement kinds, name-level vouched_by note or None)."""
    kinds, note = [], None
    served_ship = served['method'] == 'lot-sum-awards'
    if served_ship != port['ship']:
        kinds.append('ship/refuse')
    elif served_ship:
        sv = served['value'] or {}
        dv = served['derivation'] or {}
        if sv.get('cents') != port['S']:
            kinds.append('S')
        if sv.get('currency') != port['currency']:
            kinds.append('currency')
        if dv.get('lots') != port['n']:
            kinds.append('n')
        vb, pv = dv.get('vouched_by'), port['vouched_by']
        if vb != pv:
            key = None if not isinstance(vb, dict) else [vb.get('field'), vb.get('notice_id'), vb.get('cents')]
            if key is not None and key in port['vouches']:
                note = {'served': vb, 'port': pv, 'why': "B2's name-level difference: the served total is one the port "
                                                         "also counts (facts()['vouches'])"}
            else:
                kinds.append('vouched_by')
    if port.get('internal'):
        kinds.append('port-internal')
    return kinds, note


def new_notices(R, x5, x3):
    """Notices the B5 read names that the B3 read did not, with their version publication date when one is known."""
    if x3 is None:
        return None
    pub = {v.get('caused_by_notice_id'): v.get('published_at') for v in x5.get('versions', [])}
    return [{'notice_id': n, 'published_at': pub.get(n)}
            for n in sorted(set(R.version_notice_ids(x5)) - set(R.version_notice_ids(x3)))]


def detail_diff(x5, x3):
    if x3 is None:
        return None
    drop = ('value', 'value_derivation')
    return sorted(k for k in set(x5) | set(x3) if k not in drop and x5.get(k) != x3.get(k))


def crosscheck(R, b3row, port, x5, x3):
    """-> (kind, record): kind in agree | moved-explained | moved-unexplained | no-b3-outcome, plus info moves."""
    rec = {'b3': b3row and {k: b3row.get(k) for k in ('ship', 'first', 'S', 'cur', 'n')}}
    if b3row is None:
        return 'no-b3-outcome', rec
    info = []
    if (b3row.get('S'), b3row.get('cur'), b3row.get('n')) != (port['S'], port['currency'], port['n']):
        info.append('S/currency/n')
    if b3row.get('first') != port['first']:
        info.append('first')
    rec['info_moved'] = info
    if bool(b3row['ship']) == port['ship']:
        return 'agree', rec
    nn = new_notices(R, x5, x3)
    rec['new_notices'] = nn
    rec['detail_keys_differing'] = detail_diff(x5, x3)
    return ('moved-explained' if nn else 'moved-unexplained'), rec


def load_b3_outcomes(path):
    out = {}
    with gzip.open(path, 'rt', encoding='utf-8') as fh:
        for line in fh:
            r = json.loads(line)
            out[r['id']] = {k: r[k] for k in ('ship', 'first', 'S', 'cur', 'n')}
    for k, v in B3_EXTRA_OUTCOMES.items():
        out.setdefault(k, v)
    return out


def default_b3_tender_dirs():
    if os.path.exists(os.path.join(ASIDE, 'MANIFEST.json')):
        return [ASIDE]
    return [B3_TENDERS, B3_VERIFY_TENDERS]


def population_of():
    lab, walked = cohort_sources()
    tag = {i: 'labelled' for i in lab}
    tag.update({i: 'walked' for i in walked})
    tag.update({i: 'px2' for i in PX2_OOS})
    tag.update({i: 'extra' for i in EXTRA})
    return tag


def reads_window(ids, tender_dirs):
    """The fetched_at range of the served reads (their .meta files), to check they follow the drain's project."""
    at = []
    for t in ids:
        for base in tender_dirs:
            if os.path.exists(os.path.join(base, f'{t}.json')):
                m = os.path.join(base, f'{t}.meta')
                if os.path.exists(m):
                    at.append(ld(m).get('fetched_at'))
                break
    at = [a for a in at if a]
    return {'with_meta': len(at), 'first': min(at, default=None), 'last': max(at, default=None)}


def run_compare(R, ids, tender_dirs, b3_dirs, b3_outcomes_path):
    b3 = load_b3_outcomes(b3_outcomes_path)
    pop = population_of()
    rows, disagreements, names, unread, errors = [], [], [], [], []
    xc = {'agree': 0, 'moved-explained': [], 'moved-unexplained': [], 'no-b3-outcome': [], 'info_moved': []}
    notes = {'served_lot_sum_vouched': [], 'path_v_mismatch': [], 'refused_head_not_the_port_election': []}
    for tid in ids:
        install_reader(R, tender_dirs)
        try:
            x = R.tender_detail(tid)
        except R.FetchError as e:
            unread.append({'id': tid, 'error': str(e)})
            continue
        served = served_of(x)
        try:
            f = R.facts(tid)
            port = port_decision(R, f, served['method'] == 'lot-sum-awards')
        except Exception as e:  # noqa: BLE001 - recorded, and the run is incomplete
            errors.append({'id': tid, 'error': repr(e), 'trace': traceback.format_exc()[-1500:]})
            continue
        kinds, note = compare_one(served, port)
        # the B3 cross-check (B3's read, kept aside, for the new-notice explanation)
        x3 = None
        for base in b3_dirs:
            p = os.path.join(base, f'{tid}.json')
            if os.path.exists(p):
                x3 = ld(p)
                break
        install_reader(R, tender_dirs)
        xkind, xrec = crosscheck(R, b3.get(tid), port, x, x3)
        if xkind == 'agree':
            xc['agree'] += 1
        else:
            xc[xkind].append(dict(id=tid, **xrec))
        if xkind == 'moved-unexplained':
            kinds.append('moved-since-B3')
        if xrec.get('info_moved'):
            xc['info_moved'].append({'id': tid, 'moved': xrec['info_moved'], 'b3': xrec['b3'],
                                     'now': {k: port[k] for k in ('ship', 'first', 'S', 'currency', 'n')}})
        if served['method'] == 'lot-sum-vouched':
            notes['served_lot_sum_vouched'].append(tid)
        if (served['method'] == 'lot-sum-vouched') != (f['v1']['why'] == 'V'):      # merge_rule: why 'V' = path V serves
            notes['path_v_mismatch'].append({'id': tid, 'served_method': served['method'], 'port_v1': f['v1']['why']})
        if served['method'] != 'lot-sum-awards' and not f['v1']['served_matches_election']:
            notes['refused_head_not_the_port_election'].append({'id': tid, 'served': served['value'],
                                                                'port_today': f['v1']['today']})
        row = {'id': tid, 'served_ship': served['method'] == 'lot-sum-awards', 'port_ship': port['ship'],
               'agree': not kinds, 'population': pop.get(tid, 'other')}
        rows.append(row)
        if note:
            names.append(dict(id=tid, **note))
        if kinds:
            disagreements.append({'id': tid, 'kinds': kinds,
                                  'served': {'value': served['value'], 'value_derivation': served['derivation']},
                                  'port': port, 'b3_crosscheck': {'kind': xkind, **xrec}})
    kinds_count = {}
    for d in disagreements:
        for k in d['kinds']:
            kinds_count[k] = kinds_count.get(k, 0) + 1
    exit_code = 1 if disagreements else (2 if (unread or errors) else 0)
    summary = {
        'what': 'issue 551 B5 step 3: cohort parity, served state vs the port of record (docs/operations.md "Path W v2")',
        'generated_at': now(),
        'port': {'path': os.path.relpath(PORT, ROOT), 'sha256': PORT_SHA,
                 'gen_b3_sha256_recorded': GEN_B3_SHA},
        'reads': {'tenders': tender_dirs, 'b3_tenders': b3_dirs, 'notices': NOTICE_LAYERS,
                  'b3_outcomes': b3_outcomes_path},
        'cohort': {'count': len(ids), 'ids_sha256': hashlib.sha256(json.dumps(sorted(ids)).encode()).hexdigest()},
        'counts': {'compared': len(rows), 'agree': sum(r['agree'] for r in rows), 'disagree': len(disagreements),
                   'served_ships': sum(r['served_ship'] for r in rows), 'port_ships': sum(r['port_ship'] for r in rows),
                   'both_ship': sum(r['served_ship'] and r['port_ship'] for r in rows),
                   'both_refuse': sum((not r['served_ship']) and (not r['port_ship']) for r in rows),
                   'unread': len(unread), 'port_error': len(errors), 'name_level_vouched_by': len(names)},
        'served_ships_by_population': {k: sum(1 for r in rows if r['served_ship'] and r['population'] == k)
                                       for k in ('labelled', 'walked', 'px2', 'extra', 'other')},
        'served_ship_ids': [r['id'] for r in rows if r['served_ship']],
        'reads_window': reads_window(ids, tender_dirs),
        'disagreement_kinds': kinds_count,
        'disagreements': disagreements,
        'name_level_vouched_by': names,
        'b3_crosscheck': {'agree': xc['agree'], 'moved_explained': xc['moved-explained'],
                          'moved_unexplained': xc['moved-unexplained'], 'no_b3_outcome': xc['no-b3-outcome'],
                          'info_moved': xc['info_moved']},
        'notes': notes,
        'unread': unread,
        'port_errors': errors,
        'bar': ('PASS: 0 ship/refuse disagreements; S, currency and n equal on every ship' if exit_code == 0 else
                'FAIL: see disagreements (operations.md step 3: stop, switch off by step 7)' if exit_code == 1 else
                'INCOMPLETE: ids unread or the port raised; no verdict'),
        'exit': exit_code,
        'rows': [[r['id'], int(r['served_ship']), int(r['port_ship']), int(r['agree'])] for r in rows],
        'rows_legend': ['id', 'served_ship', 'port_ship', 'agree'],
    }
    return summary


def cmd_compare(args):
    R = load_port()
    ids = cohort_ids(args.ids)
    tender_dirs = args.tenders or [B5_TENDERS]
    b3_dirs = args.b3_tenders or default_b3_tender_dirs()
    s = run_compare(R, ids, tender_dirs, b3_dirs, args.b3_outcomes or B3_OUTCOMES)
    out = args.out or os.path.join(B5_DIR, 'cohort_parity.json')
    dump(out, s)
    c = s['counts']
    print(f"compared {c['compared']}/{s['cohort']['count']}: agree {c['agree']}, disagree {c['disagree']} "
          f"{s['disagreement_kinds']}; served ships {c['served_ships']}, port ships {c['port_ships']}; "
          f"name-level vouched_by {c['name_level_vouched_by']}; unread {c['unread']}, port errors {c['port_error']}; "
          f"B3 moved explained {len(s['b3_crosscheck']['moved_explained'])}, "
          f"unexplained {len(s['b3_crosscheck']['moved_unexplained'])}")
    for d in s['disagreements'][:20]:
        p = d['port']
        print(f"  DISAGREE {d['id']} {d['kinds']}: served {d['served']['value']} {d['served']['value_derivation']} | "
              f"port ship={p['ship']} first={p['first']} S={p['S']} {p['currency']} n={p['n']} v1={p['v1_reason']}")
    print(s['bar'], '->', out)
    print(f'GATE-EXIT={s["exit"]}')
    return s['exit']


# ------------------------------------------------------------------------------------------------ selftest (offline)
def cmd_selftest(args):
    """Offline. B3's post-drain cached reads with a simulated served state: the port's own ships are marked
    lot-sum-awards with S, currency, n and vouched_by, as the Rust serves them. Then compare must report 0
    disagreements, and each planted disagreement must make it exit non-zero with exactly that kind on that id."""
    base = os.path.join(B5_DIR, 'selftest_cohort')
    if os.path.exists(base):
        shutil.rmtree(base)
    os.makedirs(base)
    me = os.path.abspath(__file__)
    ev = {'started_at': now(), 'checks': []}

    def check(name, ok, **kw):
        ev['checks'].append(dict(name=name, ok=bool(ok), **kw))
        print(('ok   ' if ok else 'FAIL ') + name, json.dumps(kw, default=str)[:400], flush=True)

    R = load_port()
    ids = cohort_ids()
    b3_dirs = [B3_TENDERS, B3_VERIFY_TENDERS]
    src_sha_before = {f: sha256_file(os.path.join(B3_TENDERS, f)) for f in sorted(os.listdir(B3_TENDERS))}

    # 0. the aside copy (into the selftest dir, not the real ASIDE)
    n_src = sum(1 for d in b3_dirs for f in os.listdir(d) if f.endswith(('.json', '.meta')))
    a1 = aside(dst=os.path.join(base, 'aside'))
    a2 = aside(dst=os.path.join(base, 'aside'))
    check('aside copies every B3 read with a manifest, a re-run only re-checks',
          a1['status'] == 'copied' and a2['status'] == 'present' and a1['files'] == a2['files'] == n_src
          and not a2['source_changed_since_aside'], first=a1, second=a2, source_files=n_src)
    try:
        save_read(Http(transport=lambda p: (200, b'{}'), gap=0, stamp=os.path.join(base, '.stamp')), B3_TENDERS, 1, '/x')
        refused = False
    except SystemExit:
        refused = True
    check('a read is never written into B3\'s tender cache', refused)

    # 1. the simulated served state
    sim = os.path.join(base, 'sim_tenders')
    os.makedirs(sim)
    install_reader(R, b3_dirs)
    dec, ships, facts_by = {}, [], {}
    for t in ids:
        x = R.tender_detail(t)
        f = R.facts(t)
        ship, first, S, cur, n = R.v2_decision(f)
        dec[t] = (ship, first, S, cur, n)
        facts_by[t] = f
        y = copy.deepcopy(x)
        if ship:
            ships.append(t)
            y['value'] = {'cents': S, 'currency': cur}
            y['value_derivation'] = {'method': 'lot-sum-awards', 'summed': 'awards', 'lots': n, 'vouched_by': f['vouched_by']}
        dump(os.path.join(sim, f'{t}.json'), y)
    b3 = load_b3_outcomes(B3_OUTCOMES)
    same_as_b3 = [t for t in ids if (dec[t][0], dec[t][1], dec[t][2], dec[t][3], dec[t][4]) ==
                  (b3[t]['ship'], b3[t]['first'], b3[t]['S'], b3[t]['cur'], b3[t]['n'])]
    check('the port on B3\'s reads reproduces B3\'s outcomes on all 563 (api_port.jsonl.gz + 7511890)',
          len(same_as_b3) == len(ids) and len(ships) == 73, same=len(same_as_b3), ships=len(ships))
    # a name-level vouched_by: a ship whose port counts a second, differently named total
    def vkey(t):
        vb = facts_by[t]['vouched_by']
        return None if vb is None else [vb['field'], vb['notice_id'], vb['cents']]
    nl = next(t for t in ships if vkey(t) and any(list(v) != vkey(t) for v in facts_by[t]['vouches']))
    alt = next(list(v) for v in facts_by[nl]['vouches'] if list(v) != vkey(nl))
    y = ld(os.path.join(sim, f'{nl}.json'))
    y['value_derivation']['vouched_by'] = {'field': alt[0], 'notice_id': alt[1], 'cents': alt[2]}
    dump(os.path.join(sim, f'{nl}.json'), y)

    # 2. the G0 trap the runbook names: v2_decision as it stands refuses every simulated ship (W/G0-served)
    install_reader(R, [sim])
    trap = {}
    for t in ships:
        s_ = R.v2_decision(R.facts(t))
        trap[s_[1]] = trap.get(s_[1], 0) + 1
    check('v2_decision as it stands refuses every served ship with W/G0-served (why step 3 is served-aware)',
          trap == {'W/G0-served': len(ships)}, first_checks=trap)

    def run(tag, extra_dirs=(), b3o=None, ids_file=None):
        out = os.path.join(base, f'{tag}.json')
        cmd = [sys.executable, '-I', '-B', me, 'compare', '--out', out, '--b3-tenders', B3_TENDERS,
               '--b3-tenders', B3_VERIFY_TENDERS]
        for d in list(extra_dirs) + [sim]:
            cmd += ['--tenders', d]
        if b3o:
            cmd += ['--b3-outcomes', b3o]
        if ids_file:
            cmd += ['--ids', ids_file]
        p = subprocess.run(cmd, capture_output=True, text=True)
        return p.returncode, ld(out), p.stdout[-1500:] + p.stderr[-1500:]

    # 3. the clean run
    code, s, log = run('clean')
    check('clean: 0 disagreements, exit 0, 73 served ships = 73 port ships, the planted name-level difference listed',
          code == 0 and s['counts']['disagree'] == 0 and s['counts']['compared'] == 563 and s['counts']['agree'] == 563
          and s['counts']['served_ships'] == 73 and s['counts']['port_ships'] == 73
          and [n_['id'] for n_ in s['name_level_vouched_by']] == [nl] and not s['b3_crosscheck']['moved_unexplained']
          and not s['b3_crosscheck']['moved_explained'] and s['b3_crosscheck']['agree'] == 563
          and s['served_ships_by_population'] == {'labelled': 30, 'walked': 41, 'px2': 2, 'extra': 0, 'other': 0},
          exit=code, counts=s['counts'], kinds=s['disagreement_kinds'], name_level=s['name_level_vouched_by'],
          b3_agree=s['b3_crosscheck']['agree'], by_population=s['served_ships_by_population'])
    ev['clean_summary'] = {k: s[k] for k in ('counts', 'served_ships_by_population', 'disagreement_kinds',
                                             'name_level_vouched_by', 'b3_crosscheck', 'notes', 'bar', 'exit')}

    # 4. planted disagreements, one per run (an overlay directory holding only the planted read)
    ship_ids = [t for t in ships if t != nl and vkey(t)]
    refuse_v1 = next(t for t in ids if not dec[t][0] and facts_by[t]['v1']['why'] == 'W')   # v1 fires, an R refuses
    t_a, t_c, t_d, t_e, t_f = ship_ids[0], ship_ids[1], ship_ids[2], ship_ids[3], ship_ids[4]
    plants = []

    def plant(tag, tid, mutate, kind):
        d = os.path.join(base, f'plant_{tag}')
        os.makedirs(d)
        y = ld(os.path.join(sim, f'{tid}.json'))
        mutate(y, tid)
        dump(os.path.join(d, f'{tid}.json'), y)
        plants.append((tag, tid, kind, d))

    def unserve(y, tid):                       # the Rust kept the max election: the B3 read itself
        y.clear()
        y.update(ld(os.path.join(B3_TENDERS, f'{tid}.json')))

    def serve(y, tid):                         # the Rust ships what the port refuses
        _, _, S, cur, n = dec[tid]
        y['value'] = {'cents': S, 'currency': cur}
        y['value_derivation'] = {'method': 'lot-sum-awards', 'summed': 'awards', 'lots': n,
                                 'vouched_by': facts_by[tid]['vouched_by']}

    def cents(y, tid):
        y['value']['cents'] += 1

    def currency(y, tid):
        y['value']['currency'] = 'CHF' if y['value']['currency'] != 'CHF' else 'EUR'

    def lots(y, tid):
        y['value_derivation']['lots'] += 1

    def vouch(y, tid):
        y['value_derivation']['vouched_by'] = dict(y['value_derivation']['vouched_by'],
                                                   cents=y['value_derivation']['vouched_by']['cents'] + 1)
    plant('rust_refuses_a_port_ship', t_a, unserve, 'ship/refuse')
    plant('rust_ships_a_port_refusal', refuse_v1, serve, 'ship/refuse')
    plant('S_off_by_one_cent', t_c, cents, 'S')
    plant('currency', t_d, currency, 'currency')
    plant('lots', t_e, lots, 'n')
    plant('vouched_by_not_counted_by_the_port', t_f, vouch, 'vouched_by')
    for tag, tid, kind, d in plants:
        code, s, log = run(tag, [d])
        got = [(x_['id'], x_['kinds']) for x_ in s['disagreements']]
        check(f'plant {tag}: exit 1, exactly [{tid} {kind}]', code == 1 and got == [(tid, [kind])], exit=code, got=got,
              bar=s['bar'])
    # all six at once
    code, s, log = run('plant_all', [d for _, _, _, d in plants])
    got = sorted((x_['id'], tuple(x_['kinds'])) for x_ in s['disagreements'])
    want = sorted((tid, (kind,)) for _, tid, kind, _ in plants)
    check('all six plants at once: exit 1, the six planted (id, kind) exactly', code == 1 and got == want,
          exit=code, got=got, kinds=s['disagreement_kinds'])
    ev['plant_all_disagreements'] = s['disagreements']

    # 5. a move since B3 with no new notice counts as a disagreement (a flipped B3 outcome)
    flip = os.path.join(base, 'b3_outcomes_flipped.jsonl.gz')
    flipped = refuse_v1
    with gzip.open(B3_OUTCOMES, 'rt', encoding='utf-8') as fi, gzip.open(flip, 'wt', encoding='utf-8') as fo:
        for line in fi:
            r = json.loads(line)
            if r['id'] == flipped:
                r['ship'] = True
            fo.write(json.dumps(r) + '\n')
    code, s, log = run('moved_since_b3', b3o=flip)
    got = [(x_['id'], x_['kinds']) for x_ in s['disagreements']]
    check(f'a ship/refuse move since B3 with no new notice: exit 1, exactly [{flipped} moved-since-B3]',
          code == 1 and got == [(flipped, ['moved-since-B3'])], exit=code, got=got)
    # ... and a move explained by a notice the B5 read names (the classifier itself)
    x3 = ld(os.path.join(B3_TENDERS, f'{flipped}.json'))
    x5 = copy.deepcopy(x3)
    x5['versions'] = x5['versions'] + [{'caused_by_notice_id': 99999999, 'published_at': '2026-10-19', 'seq': 99}]
    install_reader(R, b3_dirs)
    k1, r1 = crosscheck(R, {'ship': True, 'first': None, 'S': 1, 'cur': 'EUR', 'n': 2},
                        {'ship': False, 'first': 'R3', 'S': 1, 'currency': 'EUR', 'n': 2}, x5, x3)
    k2, _ = crosscheck(R, {'ship': True, 'first': None, 'S': 1, 'cur': 'EUR', 'n': 2},
                       {'ship': False, 'first': 'R3', 'S': 1, 'currency': 'EUR', 'n': 2}, x3, x3)
    check('a move with a notice the B3 read lacked is explained; without one it is not',
          k1 == 'moved-explained' and r1['new_notices'] == [{'notice_id': 99999999, 'published_at': '2026-10-19'}]
          and k2 == 'moved-unexplained', explained=k1, new=r1['new_notices'], without=k2)

    # 6. an unread id is incomplete (exit 2)
    ids_x = os.path.join(base, 'ids_plus_unread.json')
    dump(ids_x, ids + [1])
    code, s, log = run('unread', ids_file=ids_x)
    check('an id with no read: exit 2 (incomplete), listed as unread', code == 2 and [u['id'] for u in s['unread']] == [1]
          and s['counts']['disagree'] == 0, exit=code, unread=[u['id'] for u in s['unread']])

    # 7. the reader: one GET per 1.5 s, a 408 retried once, nothing else retried
    script = {'/a': [408, 200], '/b': [408, 408], '/c': [500]}
    seen = []

    def fake(path):
        seen.append(path)
        return script[path].pop(0), b'{}'
    h = Http(stamp=os.path.join(base, '.stamp_http'), transport=fake, retry_408_sleep=0)
    codes = [h.get(p)[0] for p in ('/a', '/b', '/c')]
    gaps = [round(b[1] - a[1], 3) for a, b in zip(h.calls, h.calls[1:])]
    check('Http: 408 retried once (then 200 / then given up), a 500 not retried, every GET >= 1.5 s apart',
          codes == [200, 408, 500] and seen == ['/a', '/a', '/b', '/b', '/c'] and min(gaps) >= 1.5,
          codes=codes, calls=seen, gaps_s=gaps)

    # 8. the notices B5 would fetch: none missing for these reads (every one cached)
    need, missing = needed_notices(R, ids, [sim])
    check('every notice the simulated reads name is cached (fetch would GET 0 notices for them)', not missing,
          needed=len(need), missing=len(missing))

    src_sha_after = {f: sha256_file(os.path.join(B3_TENDERS, f)) for f in sorted(os.listdir(B3_TENDERS))}
    check("B3's raw tender cache is byte-identical after the selftest", src_sha_before == src_sha_after,
          files=len(src_sha_after))
    ev['finished_at'] = now()
    ev['passed'] = all(c['ok'] for c in ev['checks'])
    ev['simulated'] = {'ships': ships, 'name_level_id': nl, 'name_level_alt': alt,
                       'plants': [{'tag': t, 'id': i, 'kind': k} for t, i, k, _ in plants], 'flipped_b3': flipped}
    out = args.out or os.path.join(base, 'selftest.json')
    dump(out, ev)
    print('SELFTEST', 'PASSED' if ev['passed'] else 'FAILED', '->', out)
    return 0 if ev['passed'] else 1


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    sub = ap.add_subparsers(dest='cmd', required=True)
    p = sub.add_parser('ids')
    p.add_argument('--out')
    sub.add_parser('aside')
    p = sub.add_parser('fetch')
    p.add_argument('--missing', action='store_true', help='only ids with no B5 read yet')
    p.add_argument('--notices-only', action='store_true')
    p.add_argument('--ids')
    p = sub.add_parser('compare')
    p.add_argument('--out')
    p.add_argument('--tenders', action='append', help='served reads, first hit wins (default: scratchpad b5/raw/tenders)')
    p.add_argument('--b3-tenders', action='append', help="B3's reads (default: the aside copy, else B3's raw caches)")
    p.add_argument('--b3-outcomes', help='default b3/results/api_port.jsonl.gz')
    p.add_argument('--ids')
    p = sub.add_parser('selftest')
    p.add_argument('--out')
    args = ap.parse_args(argv)
    return {'ids': cmd_ids, 'aside': cmd_aside, 'fetch': cmd_fetch, 'compare': cmd_compare,
            'selftest': cmd_selftest}[args.cmd](args)


if __name__ == '__main__':
    sys.exit(main())
