"""568 u8-oos group o2: fetch every Tender of the group and every chain notice's parsed content.

Read-only public API, at most one GET per 1.5 s (1.7 s used), a 408 retried once, never more.
Raw responses land in cache_o2/ as tenders_<id>.json and notices_<id>_content.json; an existing
file is never re-fetched.
Usage: python3 -I o2_fetch.py
"""
import json, os, subprocess, time

HERE = os.path.dirname(os.path.abspath(__file__))
CACHE = os.path.join(HERE, 'cache_o2')
BASE = 'https://tenders.zebreus.click'
GAP = 1.7
_last = [0.0]


def get(path, out):
    if os.path.exists(out):
        return 'cached'
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


def main():
    draw = json.load(open(os.path.join(HERE, 'draw.json'), encoding='utf-8'))
    ids = next(g['ids'] for g in draw['groups'] if g['key'] == 'o2')
    os.makedirs(CACHE, exist_ok=True)
    for tid in ids:
        tp = os.path.join(CACHE, f'tenders_{tid}.json')
        st = get(f'/v1/tenders/{tid}', tp)
        print('tender', tid, st, flush=True)
        if not os.path.exists(tp):
            continue
        t = json.load(open(tp, encoding='utf-8'))
        chain = [v['caused_by_notice_id'] for v in sorted(t['versions'], key=lambda v: v['seq'])]
        extra = sorted({r['notice_id'] for r in t.get('lot_results', []) if r.get('notice_id')} - set(chain))
        for n in chain + extra:
            st = get(f'/v1/notices/{n}/content', os.path.join(CACHE, f'notices_{n}_content.json'))
            print('  notice', n, st, flush=True)


if __name__ == '__main__':
    main()
