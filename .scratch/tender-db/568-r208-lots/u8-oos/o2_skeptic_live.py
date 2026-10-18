"""568 u8-oos group o2 SKEPTIC: re-read a few Tenders live and diff their lot_results against cache_o2/.

Read-only public API, at most one GET per 1.5 s (1.7 s used), a 408 retried once, never more.
Live responses land in cache_o2/ as skeptic_live_tenders_<id>.json (the measurer's files are never touched).
Usage: python3 -I -B o2_skeptic_live.py <tender_id> ...
"""
import json, os, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
CACHE = os.path.join(HERE, 'cache_o2')
BASE = 'https://tenders.zebreus.click'
GAP = 1.7
_last = [0.0]


def get(path, out):
    for attempt in (1, 2):
        wait = _last[0] + GAP - time.time()
        if wait > 0:
            time.sleep(wait)
        _last[0] = time.time()
        tmp = out + '.part'
        r = subprocess.run(['curl', '-sS', '--max-time', '90', '-o', tmp, '-w', '%{http_code}', BASE + path],
                           capture_output=True, text=True)
        code = r.stdout.strip()
        if code == '200':
            os.replace(tmp, out)
            return code
        if os.path.exists(tmp):
            os.remove(tmp)
        if code == '408' and attempt == 1:
            continue
        return f'{code} {r.stderr.strip()}'
    return 'failed'


def lr(t):
    return sorted((r['notice_id'], r['key'], r.get('lot'), tuple(w.get('organization_name') or '' for w in r.get('winners') or []))
                  for r in t.get('lot_results', []))


def main():
    for tid in [int(a) for a in sys.argv[1:]]:
        out = os.path.join(CACHE, f'skeptic_live_tenders_{tid}.json')
        st = get(f'/v1/tenders/{tid}', out)
        print('tender', tid, st, flush=True)
        if st != '200':
            continue
        live = json.load(open(out, encoding='utf-8'))
        cached = json.load(open(os.path.join(CACHE, f'tenders_{tid}.json'), encoding='utf-8'))
        a, b = lr(live), lr(cached)
        print('  lot_results live', len(a), 'cached', len(b), 'identical' if a == b else 'DIFFER')
        if a != b:
            sa, sb = set(a), set(b)
            print('  only live  :', sorted(sa - sb)[:10])
            print('  only cached:', sorted(sb - sa)[:10])
        print('  versions live', [v['caused_by_notice_id'] for v in live['versions']],
              'cached', [v['caused_by_notice_id'] for v in cached['versions']])


if __name__ == '__main__':
    main()
