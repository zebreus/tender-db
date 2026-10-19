"""Issue 551 B6 U1: the fetcher for the B6 port (b6/rule_v2_g.py). U2 runs it over the 3,502; U1 runs it only on the
self-test ids.

    python3 -I -B fetch_b6.py [--ids FILE | --id N[,N...] ...] [--notices-only] [--refresh] [--dry]
                              [--pause-utc HH:MM-HH:MM] [--manifest FILE] [--limit K]

  ids      default: every served_lsa_rows.json id whose derivation is lot-sum-awards (b5/investigate, 3,502 ids:
           the 3,503 rows minus 179732). --ids reads a JSON list, or an object with an "ids" list; --id takes ids
           inline (repeatable, comma-separated).
  tenders  GET /v1/tenders/{id} into scratchpad b6/tenders/{id}.json.gz (the exact response body, gzip), with a
           sidecar {id}.meta.json: fetched_at (UTC, millisecond precision, the instant the attempt that returned the
           saved body was sent -- after the stamp wait and any retry), sent_at (before the stamp wait), path, status,
           bytes, sha256 of the body. The sidecar is written BEFORE the body. Resumable: an id counts as cached only
           when its .json.gz AND its .meta.json exist and the sidecar's sha256 matches the body, so a kill between
           the two writes re-fetches that id. --refresh re-reads a cached id; the earlier sidecar is kept as
           {id}.meta.<its fetched_at>.json and listed in the manifest entry's `history` (U2's parity exemption for an
           id is fixed by the fetch time it was compared on; a refresh never erases it).
  notices  then, for every fetched tender, every version notice rule_v2_g.version_notice_ids() reads (the versions'
           caused_by_notice_id plus every notice a lot result / bid / contract names) that NO notice layer of
           rule_v2_g holds: GET /v1/notices/{id}/content into scratchpad b6/notices/{id}.json.gz. Notice contents are
           immutable, so a hit in any layer is never re-read.
  GETs     through b5/cohort_parity.Http, with cohort_parity's shared stamp file (scratchpad b5/.last_get, shared with
           confirm_draw.py): at most one GET per 1.5 s across processes, a 408 retried ONCE and never more, a transport
           error retried twice. The stamp read / sleep / write runs under an flock (scratchpad b6/.http.lock), so two
           fetch_b6 processes can never both read the same stamp. No other network path, no ssh.
  lock     a run holds an exclusive flock on scratchpad b6/fetch.lock for its whole life; a second fetch_b6 run exits 4
           at once (one writer of the manifest and the layers).
  manifest scratchpad b6/fetch_manifest.json (rewritten every 25 GETs and at the end): per tender its fetch time and
           status, per tender its version-notice count and the notices missing from every layer, the notice GETs,
           every 404 and every other miss (408 twice, transport, 5xx). A re-run merges into it; a body on disk with no
           manifest entry (a kill between dumps) is backfilled from its sidecar / file. missing_now is kept current at
           every dump (null = not computed yet, never 0 by default); summary.notice_phase_complete says whether the
           last run finished its notice phase. The run's id list is written once to b6/fetch_ids_<sha256[:16]>.json
           and summary.progress counts its done / failed / not-yet-attempted tenders.
  --pause-utc HH:MM-HH:MM  stop cleanly (exit 3, manifest written, resumable) when the UTC clock is inside the window
           before a GET (the decision pauses U2 from 07:30Z until the chain shows done on /health).
  --dry    no GET: report what a run would fetch (tenders not cached; notices missing from every layer, for the
           tenders already cached).

Exit: 0 all done; 2 any 404 or miss; 3 paused by --pause-utc; 4 another fetch_b6 run holds the lock.
"""
import argparse
import fcntl
import gzip
import hashlib
import importlib.util
import json
import os
import sys
import time

sys.dont_write_bytecode = True

HERE = os.path.dirname(os.path.abspath(__file__))                      # .../551-lot-sum/b6
ROOT = os.path.dirname(HERE)
SERVED_LSA = os.path.join(ROOT, 'b5', 'investigate', 'served_lsa_rows.json')


def _load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


CP = _load_module('cohort_parity_b6', os.path.join(ROOT, 'b5', 'cohort_parity.py'))
R = _load_module('rule_v2_g_fetch', os.path.join(HERE, 'rule_v2_g.py'))
MANIFEST = os.path.join(R.B6, 'fetch_manifest.json')
RUN_LOCK = os.path.join(R.B6, 'fetch.lock')
HTTP_LOCK = os.path.join(R.B6, '.http.lock')


def now():
    return time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())


def iso_ms(t):
    return time.strftime('%Y-%m-%dT%H:%M:%S', time.gmtime(t)) + f'.{int((t % 1) * 1000):03d}Z'


class LockedHttp(CP.Http):
    """cohort_parity.Http with its stamp read / sleep / write under an exclusive flock, so concurrent fetch_b6
    processes serialise on the stamp (the stamp is written before the GET is sent, as in cohort_parity)."""

    def __init__(self, *a, lock_path=HTTP_LOCK, **k):
        super().__init__(*a, **k)
        self.lock_path = lock_path

    def _wait(self):
        os.makedirs(os.path.dirname(self.lock_path), exist_ok=True)
        with open(self.lock_path, 'a') as lk:
            fcntl.flock(lk, fcntl.LOCK_EX)
            try:
                super()._wait()
            finally:
                fcntl.flock(lk, fcntl.LOCK_UN)


class Locked(Exception):
    pass


def take_run_lock(path=RUN_LOCK):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    fh = open(path, 'a+')
    try:
        fcntl.flock(fh, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        fh.close()
        raise Locked(path)
    fh.seek(0)
    fh.truncate()
    fh.write(f'{os.getpid()} {now()}\n')
    fh.flush()
    return fh


def sha256_bytes(b):
    return hashlib.sha256(b).hexdigest()


def default_ids():
    with open(SERVED_LSA, encoding='utf-8') as fh:
        rows = json.load(fh)
    return sorted(int(r[0]) for r in rows if r[3] == 'lot-sum-awards')


def parse_ids(args):
    ids = []
    if args.ids:
        with open(args.ids, encoding='utf-8') as fh:
            d = json.load(fh)
        ids += [int(i) for i in (d['ids'] if isinstance(d, dict) else d)]
    for chunk in args.id or []:
        ids += [int(i) for i in chunk.split(',') if i.strip()]
    if not args.ids and not args.id:
        ids = default_ids()
    return list(dict.fromkeys(ids))


def in_window(win):
    if not win:
        return False
    a, b = win.split('-')
    t = time.strftime('%H:%M', time.gmtime())
    return (a <= t < b) if a <= b else (t >= a or t < b)


def write_gz(path, body):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + '.tmp'
    with gzip.open(tmp, 'wb', compresslevel=6) as fh:
        fh.write(body)
    os.replace(tmp, path)


def dump(path, obj):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as fh:
        json.dump(obj, fh, indent=1, ensure_ascii=False, default=str)
        fh.write('\n')
    os.replace(tmp, path)


class Paused(Exception):
    pass


class Fetcher:
    def __init__(self, http, manifest_path, pause=None):
        self.http, self.manifest_path, self.pause = http, manifest_path, pause
        self.m = self._load_manifest()
        self.gets_since_dump = 0
        self.pending = {}               # tid -> notices missing from every layer at the start of the notice phase
        self.got_notices = set()        # notices fetched (200) in this run
        self.ids = []                   # this run's id list (summary.progress)

    def _load_manifest(self):
        if os.path.exists(self.manifest_path):
            with open(self.manifest_path, encoding='utf-8') as fh:
                m = json.load(fh)
        else:
            m = {'what': 'issue 551 B6 fetch manifest (b6/fetch_b6.py)', 'created_at': now(), 'runs': [],
                 'tenders': {}, 'notices': {}, 'tender_notices': {}, 'not_found_404': [], 'misses': []}
        return m

    def save(self):
        self.m['updated_at'] = now()
        for tid, miss in self.pending.items():          # keep missing_now current at every dump
            e = self.m['tender_notices'].setdefault(str(tid), {})
            e['missing_now'] = [n for n in miss if n not in self.got_notices]
        t = self.m['tenders']
        tn = self.m['tender_notices']
        bad = {(r['kind'], str(r['id'])) for r in self.m['not_found_404'] + self.m['misses']}
        have = {i for i in self.ids if all(os.path.exists(q) for q in self._paths(i))}   # cheap; is_cached on resume
        done = len(have)
        failed = sum(1 for i in self.ids if i not in have and ('tender', str(i)) in bad)
        self.m['summary'] = {
            'tenders_ok': sum(1 for v in t.values() if v.get('status') == 200),
            'tenders_404': sum(1 for v in t.values() if v.get('status') == 404),
            'tenders_other': sum(1 for v in t.values() if v.get('status') not in (200, 404)),
            'notices_fetched_ok': sum(1 for v in self.m['notices'].values() if v.get('status') == 200),
            'notices_404': sum(1 for v in self.m['notices'].values() if v.get('status') == 404),
            'notices_other': sum(1 for v in self.m['notices'].values() if v.get('status') not in (200, 404)),
            'tenders_with_missing_notices_now': sum(1 for v in tn.values() if v.get('missing_now')),
            'tenders_missing_now_not_computed': sum(1 for v in tn.values() if v.get('missing_now') is None),
            'notices_missing_now': len({n for v in tn.values() for n in (v.get('missing_now') or [])}),
            'notice_phase_complete': bool(self.m['runs'] and self.m['runs'][-1].get('notice_phase_complete')),
            'progress': {'run_ids': len(self.ids), 'tenders_done': done, 'tenders_failed': failed,
                         'tenders_not_attempted': len(self.ids) - done - failed,
                         'ids_file': self.m['runs'][-1].get('ids_file') if self.m['runs'] else None},
        }
        dump(self.manifest_path, self.m)
        self.gets_since_dump = 0

    def _get(self, path):
        """-> (fetched_at, sent_at, code, body). fetched_at is the send time of the attempt that returned `body`
        (after the stamp wait and any retry), millisecond precision; sent_at is before the stamp wait."""
        if in_window(self.pause):
            raise Paused(path)
        t0 = time.time()
        n0 = len(self.http.calls)
        code, body = self.http.get(path)
        self.gets_since_dump += 1
        t = self.http.calls[-1][1] if len(self.http.calls) > n0 else t0
        return iso_ms(t), iso_ms(t0), code, body

    def _record_fail(self, kind, key, code, body):
        rec = {'kind': kind, 'id': key, 'code': code, 'at': now(),
               'body': (body or b'')[:300].decode('utf-8', 'replace') if isinstance(body, (bytes, bytearray)) else str(body)}
        lst = self.m['not_found_404'] if code == 404 else self.m['misses']
        lst[:] = [r for r in lst if not (r['kind'] == kind and r['id'] == key)]
        lst.append(rec)

    def _clear_fail(self, kind, key):
        for lst in (self.m['not_found_404'], self.m['misses']):
            lst[:] = [r for r in lst if not (r['kind'] == kind and r['id'] == key)]

    def _paths(self, tid):
        return (os.path.join(R.B6_TENDERS, f'{tid}.json.gz'), os.path.join(R.B6_TENDERS, f'{tid}.meta.json'))

    def is_cached(self, tid):
        """Body and sidecar both present and the sidecar's sha256 matches the body (a kill between the writes, or
        a sidecar from another read, is not cached)."""
        p, mp = self._paths(tid)
        if not (os.path.exists(p) and os.path.exists(mp)):
            return False
        try:
            with open(mp, encoding='utf-8') as fh:
                meta = json.load(fh)
            with gzip.open(p, 'rb') as fh:
                return meta.get('sha256') == sha256_bytes(fh.read())
        except (OSError, ValueError, EOFError):
            return False

    def _backfill_tender(self, tid):
        e = self.m['tenders'].get(str(tid))
        if e and e.get('status') == 200 and e.get('fetched_at'):
            return
        with open(self._paths(tid)[1], encoding='utf-8') as fh:
            meta = json.load(fh)
        self.m['tenders'][str(tid)] = {'status': 200, 'fetched_at': meta.get('fetched_at'), 'version': meta.get('version'),
                                       'sha256': meta.get('sha256'), 'backfilled_from_sidecar': True}

    def tender(self, tid, refresh=False):
        p, mp = self._paths(tid)
        cached = self.is_cached(tid)
        if cached and not refresh:
            self._backfill_tender(tid)
            return 'cached'
        path = f'/v1/tenders/{tid}'
        fetched, sent, code, body = self._get(path)
        if code != 200:
            prev = self.m['tenders'].get(str(tid))
            self.m['tenders'][str(tid)] = {'status': code, 'attempted_at': fetched}
            if cached and prev:                        # a failed refresh keeps the cached read on record
                self.m['tenders'][str(tid)] = dict(prev, refresh_failed={'status': code, 'attempted_at': fetched})
            self._record_fail('tender', tid, code, body)
            return code
        x = json.loads(body)
        if int(x.get('id', tid)) != tid:
            self.m['tenders'][str(tid)] = {'status': 'id-mismatch', 'attempted_at': fetched, 'got': x.get('id')}
            self._record_fail('tender', tid, 'id-mismatch', body)
            return 'id-mismatch'
        history = list((self.m['tenders'].get(str(tid)) or {}).get('history') or [])
        if os.path.exists(mp):                         # --refresh (or a stale sidecar): keep the earlier read's record
            with open(mp, encoding='utf-8') as fh:
                old = json.load(fh)
            stamp = str(old.get('fetched_at') or 'unknown').replace(':', '')
            keep = os.path.join(R.B6_TENDERS, f'{tid}.meta.{stamp}.json')
            os.replace(mp, keep)
            history.append(dict(old, kept_as=os.path.basename(keep), body_matched=cached))
        meta = {'fetched_at': fetched, 'sent_at': sent, 'path': path, 'status': 200, 'bytes': len(body),
                'sha256': sha256_bytes(body), 'version': x.get('version'), 'n_versions': len(x.get('versions') or [])}
        dump(mp, meta)                                 # the sidecar first, then the body (is_cached checks the pair)
        write_gz(p, body)
        e = {'status': 200, 'fetched_at': fetched, 'sent_at': sent, 'version': x.get('version'), 'sha256': meta['sha256']}
        if history:
            e['history'] = history
        self.m['tenders'][str(tid)] = e
        self._clear_fail('tender', tid)
        return 200

    def notice(self, nid):
        if R.find_cached('notices', nid) is not None:
            return 'cached'
        path = f'/v1/notices/{nid}/content'
        fetched, sent, code, body = self._get(path)
        if code != 200:
            self.m['notices'][str(nid)] = {'status': code, 'attempted_at': fetched}
            self._record_fail('notice', nid, code, body)
            return code
        c = json.loads(body)
        if c.get('notice_id') is not None and int(c['notice_id']) != nid:
            self.m['notices'][str(nid)] = {'status': 'id-mismatch', 'attempted_at': fetched}
            self._record_fail('notice', nid, 'id-mismatch', body)
            return 'id-mismatch'
        write_gz(os.path.join(R.B6_NOTICES, f'{nid}.json.gz'), body)
        self.m['notices'][str(nid)] = {'status': 200, 'fetched_at': fetched, 'bytes': len(body), 'sha256': sha256_bytes(body)}
        self.got_notices.add(nid)
        self._clear_fail('notice', nid)
        return 200

    def backfill_notices(self):
        """A b6 notice file with no manifest entry (written after the last dump of a killed run) gets one."""
        n = 0
        if not os.path.isdir(R.B6_NOTICES):
            return 0
        for name in os.listdir(R.B6_NOTICES):
            if not name.endswith('.json.gz'):
                continue
            nid = name[:-len('.json.gz')]
            e = self.m['notices'].get(nid)
            if e and e.get('status') == 200:
                continue
            fp = os.path.join(R.B6_NOTICES, name)
            with gzip.open(fp, 'rb') as fh:
                body = fh.read()
            self.m['notices'][nid] = {'status': 200, 'fetched_at': None, 'file_mtime': iso_ms(os.path.getmtime(fp)),
                                      'bytes': len(body), 'sha256': sha256_bytes(body), 'backfilled_from_file': True}
            self._clear_fail('notice', int(nid))
            n += 1
        return n

    def needed(self, tid):
        """(version notice ids, those no layer holds) for a cached tender, or None when the tender is not cached."""
        if not self.is_cached(tid):
            return None
        x = R.tender_detail(tid)
        nids = R.version_notice_ids(x)
        return nids, [n for n in nids if R.find_cached('notices', n) is None]

    def maybe_save(self):
        if self.gets_since_dump >= 25:
            self.save()


def write_ids(ids):
    """The run's id list, once, to b6/fetch_ids_<sha256[:16]>.json -> (sha256, path)."""
    body = json.dumps(ids).encode()
    h = sha256_bytes(body)
    path = os.path.join(R.B6, f'fetch_ids_{h[:16]}.json')
    if not os.path.exists(path):
        dump(path, {'sha256_of_json_ids': h, 'n': len(ids), 'ids': ids})
    return h, path


def run(ids, http=None, manifest=MANIFEST, notices_only=False, refresh=False, dry=False, pause=None, log=print,
        lock_path=RUN_LOCK):
    try:
        lock = take_run_lock(lock_path)
    except Locked as e:
        log(f'fetch_b6: another run holds {e}; not starting (exit 4)')
        return 4
    try:
        return _run(ids, http, manifest, notices_only, refresh, dry, pause, log)
    finally:
        lock.close()


def _run(ids, http, manifest, notices_only, refresh, dry, pause, log):
    f = Fetcher(http or LockedHttp(stamp=CP.STAMP), manifest, pause)
    ids_sha, ids_file = write_ids(ids)
    f.ids = list(ids)
    run_rec = {'started_at': now(), 'ids': len(ids), 'ids_sha256': ids_sha, 'ids_file': ids_file,
               'stamp': CP.STAMP, 'gap_s': CP.GAP, 'dry': dry, 'refresh': refresh, 'notices_only': notices_only,
               'notice_phase_complete': False}
    f.m['runs'].append(run_rec)
    run_rec['notices_backfilled'] = f.backfill_notices()
    status = 0
    try:
        if dry:
            todo_t = [t for t in ids if not f.is_cached(t) or refresh]
            miss = set()
            for t in ids:
                nd = f.needed(t)
                if nd:
                    miss |= set(nd[1])
            run_rec.update(dry_tenders_to_fetch=len(todo_t), dry_notices_missing=len(miss))
            log(f'dry: {len(todo_t)} tenders to fetch; {len(miss)} notices missing from every layer '
                f'(for the {sum(1 for t in ids if f.is_cached(t))} tenders cached)')
            return 0
        if not notices_only:
            for i, t in enumerate(ids):
                r = f.tender(t, refresh=refresh)
                if r not in ('cached', 200):
                    log(f'FAIL tender {t} {r}')
                if i % 50 == 0:
                    log(f'tenders {i}/{len(ids)} {t} {r}')
                f.maybe_save()
        else:
            for t in ids:
                if f.is_cached(t):
                    f._backfill_tender(t)
        all_missing = []
        for t in ids:
            nd = f.needed(t)
            if nd is None:
                continue
            nids, missing = nd
            prev = f.m['tender_notices'].get(str(t)) or {}
            f.m['tender_notices'][str(t)] = {'version_notices': len(nids), 'missing_before': len(missing),
                                             'missing_now': None}
            if prev.get('missing_before_first_run') is not None or prev.get('missing_before') is not None:
                f.m['tender_notices'][str(t)]['missing_before_first_run'] = prev.get(
                    'missing_before_first_run', prev.get('missing_before'))
            f.pending[t] = missing
            all_missing += missing
        all_missing = list(dict.fromkeys(all_missing))
        run_rec['notices_missing_before'] = len(all_missing)
        log(f'notices missing from every layer: {len(all_missing)}')
        for i, n in enumerate(all_missing):
            r = f.notice(n)
            if r not in ('cached', 200):
                log(f'FAIL notice {n} {r}')
            if i % 50 == 0:
                log(f'notices {i}/{len(all_missing)} {n} {r}')
            f.maybe_save()
        f.pending = {}
        for t in ids:                                  # the exact recompute, from the layers
            nd = f.needed(t)
            if nd is not None:
                f.m['tender_notices'][str(t)]['missing_now'] = nd[1]
        run_rec['notice_phase_complete'] = True
    except Paused as e:
        run_rec['paused_at'] = now()
        run_rec['paused_before'] = str(e)
        log(f'paused by --pause-utc {pause} before {e}; re-run to resume')
        status = 3
    finally:
        run_rec['ended_at'] = now()
        run_rec['http_calls'] = len(f.http.calls)
        run_rec['http_statuses'] = {}
        for _p, _t, c in f.http.calls:
            run_rec['http_statuses'][str(c)] = run_rec['http_statuses'].get(str(c), 0) + 1
        f.save()
    if status:
        return status
    s = f.m['summary']
    ids_set = {str(t) for t in ids}
    bad = [r for r in f.m['not_found_404'] + f.m['misses'] if r['kind'] == 'notice' or str(r['id']) in ids_set]
    log(json.dumps(s))
    return 2 if bad else 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--ids')
    ap.add_argument('--id', action='append')
    ap.add_argument('--notices-only', action='store_true')
    ap.add_argument('--refresh', action='store_true')
    ap.add_argument('--dry', action='store_true')
    ap.add_argument('--pause-utc')
    ap.add_argument('--manifest', default=MANIFEST)
    ap.add_argument('--limit', type=int)
    a = ap.parse_args(argv)
    ids = parse_ids(a)
    if a.limit:
        ids = ids[:a.limit]
    return run(ids, manifest=a.manifest, notices_only=a.notices_only, refresh=a.refresh, dry=a.dry, pause=a.pause_utc)


if __name__ == '__main__':
    sys.exit(main())
