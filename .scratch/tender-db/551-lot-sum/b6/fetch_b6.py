"""Issue 551 B6 U1: the fetcher for the B6 port (b6/rule_v2_g.py). U2 runs it over the 3,502; U1 runs it only on the
self-test ids.

    python3 -I -B fetch_b6.py [--ids FILE | --id N[,N...] ...] [--notices-only] [--refresh] [--dry]
                              [--pause-utc HH:MM-HH:MM] [--manifest FILE] [--limit K]

  ids      default: every served_lsa_rows.json id whose derivation is lot-sum-awards (b5/investigate, 3,502 ids:
           the 3,503 rows minus 179732). --ids reads a JSON list, or an object with an "ids" list; --id takes ids
           inline (repeatable, comma-separated).
  tenders  GET /v1/tenders/{id} into scratchpad b6/tenders/{id}.json.gz (the exact response body, gzip), with a
           sidecar {id}.meta.json: fetched_at (UTC, the instant the GET was sent), path, status, bytes, sha256 of the
           body. Resumable: an id whose .json.gz exists is skipped (--refresh re-reads it, replacing the file).
  notices  then, for every fetched tender, every version notice rule_v2_g.version_notice_ids() reads (the versions'
           caused_by_notice_id plus every notice a lot result / bid / contract names) that NO notice layer of
           rule_v2_g holds: GET /v1/notices/{id}/content into scratchpad b6/notices/{id}.json.gz. Notice contents are
           immutable, so a hit in any layer is never re-read.
  GETs     through b5/cohort_parity.Http, with cohort_parity's shared stamp file (scratchpad b5/.last_get, shared with
           confirm_draw.py): at most one GET per 1.5 s across processes, a 408 retried ONCE and never more, a transport
           error retried twice. No other network path, no ssh.
  manifest scratchpad b6/fetch_manifest.json (rewritten every 25 GETs and at the end): per tender its fetch time and
           status, per tender its version-notice count and the notices missing from every layer, the notice GETs,
           every 404 and every other miss (408 twice, transport, 5xx). A re-run merges into it.
  --pause-utc HH:MM-HH:MM  stop cleanly (exit 3, manifest written, resumable) when the UTC clock is inside the window
           before a GET (the decision pauses U2 from 07:30Z until the chain shows done on /health).
  --dry    no GET: report what a run would fetch (tenders not cached; notices missing from every layer, for the
           tenders already cached).

Exit: 0 all done; 2 any 404 or miss; 3 paused by --pause-utc.
"""
import argparse
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


def now():
    return time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())


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
        t = self.m['tenders']
        self.m['summary'] = {
            'tenders_ok': sum(1 for v in t.values() if v.get('status') == 200),
            'tenders_404': sum(1 for v in t.values() if v.get('status') == 404),
            'tenders_other': sum(1 for v in t.values() if v.get('status') not in (200, 404)),
            'notices_fetched_ok': sum(1 for v in self.m['notices'].values() if v.get('status') == 200),
            'notices_404': sum(1 for v in self.m['notices'].values() if v.get('status') == 404),
            'notices_other': sum(1 for v in self.m['notices'].values() if v.get('status') not in (200, 404)),
            'tenders_with_missing_notices_now': sum(1 for v in self.m['tender_notices'].values() if v.get('missing_now')),
        }
        dump(self.manifest_path, self.m)
        self.gets_since_dump = 0

    def _get(self, path):
        if in_window(self.pause):
            raise Paused(path)
        sent = now()
        code, body = self.http.get(path)
        self.gets_since_dump += 1
        return sent, code, body

    def _record_fail(self, kind, key, code, body):
        rec = {'kind': kind, 'id': key, 'code': code, 'at': now(),
               'body': (body or b'')[:300].decode('utf-8', 'replace') if isinstance(body, (bytes, bytearray)) else str(body)}
        lst = self.m['not_found_404'] if code == 404 else self.m['misses']
        lst[:] = [r for r in lst if not (r['kind'] == kind and r['id'] == key)]
        lst.append(rec)

    def _clear_fail(self, kind, key):
        for lst in (self.m['not_found_404'], self.m['misses']):
            lst[:] = [r for r in lst if not (r['kind'] == kind and r['id'] == key)]

    def tender(self, tid, refresh=False):
        p = os.path.join(R.B6_TENDERS, f'{tid}.json.gz')
        if os.path.exists(p) and not refresh:
            return 'cached'
        path = f'/v1/tenders/{tid}'
        sent, code, body = self._get(path)
        if code != 200:
            self.m['tenders'][str(tid)] = {'status': code, 'attempted_at': sent}
            self._record_fail('tender', tid, code, body)
            return code
        x = json.loads(body)
        if int(x.get('id', tid)) != tid:
            self.m['tenders'][str(tid)] = {'status': 'id-mismatch', 'attempted_at': sent, 'got': x.get('id')}
            self._record_fail('tender', tid, 'id-mismatch', body)
            return 'id-mismatch'
        write_gz(p, body)
        meta = {'fetched_at': sent, 'path': path, 'status': 200, 'bytes': len(body), 'sha256': sha256_bytes(body),
                'version': x.get('version'), 'n_versions': len(x.get('versions') or [])}
        dump(os.path.join(R.B6_TENDERS, f'{tid}.meta.json'), meta)
        self.m['tenders'][str(tid)] = {'status': 200, 'fetched_at': sent, 'version': x.get('version')}
        self._clear_fail('tender', tid)
        return 200

    def notice(self, nid):
        if R.find_cached('notices', nid) is not None:
            return 'cached'
        path = f'/v1/notices/{nid}/content'
        sent, code, body = self._get(path)
        if code != 200:
            self.m['notices'][str(nid)] = {'status': code, 'attempted_at': sent}
            self._record_fail('notice', nid, code, body)
            return code
        c = json.loads(body)
        if c.get('notice_id') is not None and int(c['notice_id']) != nid:
            self.m['notices'][str(nid)] = {'status': 'id-mismatch', 'attempted_at': sent}
            self._record_fail('notice', nid, 'id-mismatch', body)
            return 'id-mismatch'
        write_gz(os.path.join(R.B6_NOTICES, f'{nid}.json.gz'), body)
        self.m['notices'][str(nid)] = {'status': 200, 'fetched_at': sent, 'bytes': len(body)}
        self._clear_fail('notice', nid)
        return 200

    def needed(self, tid):
        """(version notice ids, those no layer holds) for a cached tender, or None when the tender is not cached."""
        if R.find_cached('tenders', tid) is None:
            return None
        x = R.tender_detail(tid)
        nids = R.version_notice_ids(x)
        return nids, [n for n in nids if R.find_cached('notices', n) is None]

    def maybe_save(self):
        if self.gets_since_dump >= 25:
            self.save()


def run(ids, http=None, manifest=MANIFEST, notices_only=False, refresh=False, dry=False, pause=None, log=print):
    f = Fetcher(http or CP.Http(stamp=CP.STAMP), manifest, pause)
    run_rec = {'started_at': now(), 'ids': len(ids), 'ids_sha256': sha256_bytes(json.dumps(ids).encode()),
               'stamp': CP.STAMP, 'gap_s': CP.GAP, 'dry': dry, 'refresh': refresh, 'notices_only': notices_only}
    f.m['runs'].append(run_rec)
    status = 0
    try:
        if dry:
            todo_t = [t for t in ids if R.find_cached('tenders', t) is None or refresh]
            miss = set()
            for t in ids:
                nd = f.needed(t)
                if nd:
                    miss |= set(nd[1])
            run_rec.update(dry_tenders_to_fetch=len(todo_t), dry_notices_missing=len(miss))
            log(f'dry: {len(todo_t)} tenders to fetch; {len(miss)} notices missing from every layer '
                f'(for the {sum(1 for t in ids if R.find_cached("tenders", t))} tenders cached)')
            return 0
        if not notices_only:
            for i, t in enumerate(ids):
                r = f.tender(t, refresh=refresh)
                if r not in ('cached', 200):
                    log(f'FAIL tender {t} {r}')
                if i % 50 == 0:
                    log(f'tenders {i}/{len(ids)} {t} {r}')
                f.maybe_save()
        all_missing = []
        for t in ids:
            nd = f.needed(t)
            if nd is None:
                continue
            nids, missing = nd
            f.m['tender_notices'][str(t)] = {'version_notices': len(nids), 'missing_before': len(missing)}
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
        for t in ids:
            nd = f.needed(t)
            if nd is not None:
                f.m['tender_notices'][str(t)]['missing_now'] = nd[1]
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
