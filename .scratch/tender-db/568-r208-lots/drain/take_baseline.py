"""Baseline before the combined 568-u8 + 570 refold (decision-u7.md "Units" 10; 570 decision unit 5).

Reads the public API only (about 1 request per second, a 408 retried once):
  band     -- every Tender at or above EUR 1 bn (value, derivation, version), as baseline-pre568.json;
  path_v   -- the 16 path-V heads of baseline-pre568.json plus 5617904;
  tenders  -- lots[] and lot_results[] of every Tender a read-back names: 570's 211 port Tenders, the salted 150,
              the OOS 50, the fresh 30 and unit 1's measured Tenders.

Usage: python3 -I take_baseline.py OUT.json
"""
import json, os, sys, time, urllib.request, urllib.error

API = 'https://tenders.zebreus.click'
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)          # 568-r208-lots
SCR = os.path.dirname(ROOT)           # .scratch/tender-db
BAND_MIN = 100_000_000_000


def get(path):
    for attempt in (0, 1):
        time.sleep(1.0)
        try:
            with urllib.request.urlopen(API + path, timeout=60) as r:
                return json.load(r)
        except urllib.error.HTTPError as e:
            if e.code == 408 and attempt == 0:
                continue
            raise


def main():
    out = sys.argv[1]
    old = json.load(open(os.path.join(HERE, 'baseline-pre568.json')))
    heads = sorted({int(k) for k in old['path_v']} | {5617904})

    band, cursor, pages = {}, '', 0
    while True:
        d = get(f'/v1/tenders?min_value={BAND_MIN}&limit=1000' + (f'&cursor={cursor}' if cursor else ''))
        pages += 1
        for t in d['items']:
            band[str(t['id'])] = {'value': t['value'], 'derivation': t.get('value_derivation'), 'version': t['version']}
        if not d.get('more'):
            break
        cursor = d['next_cursor']

    path_v = {}
    for i in heads:
        t = get(f'/v1/tenders/{i}')
        path_v[str(i)] = {'value': t.get('value'), 'derivation': t.get('value_derivation'), 'version': t.get('version')}

    ids = set()
    p570 = json.load(open(os.path.join(SCR, '570-lot-keys', 'port_result.json')))
    ids |= {s['tender_id'] for s in p570['sample'] + p570['exhibits']}
    ids |= {i for g in json.load(open(os.path.join(ROOT, 'readback', 'u7_sample.json')))['groups'] for i in g['ids']}
    ids |= {i for g in json.load(open(os.path.join(ROOT, 'u8-oos', 'draw.json')))['groups'] for i in g['ids']}
    ids |= {x['id'] for x in json.load(open(os.path.join(ROOT, 'readback', 'fresh30.json')))['draw']}
    ids |= {t['tender_id'] for t in json.load(open(os.path.join(ROOT, 'measurement.json')))['tenders']}
    ids.add(8327340)

    tenders = {}
    for i in sorted(ids):
        t = get(f'/v1/tenders/{i}')
        tenders[str(i)] = {
            'version': t.get('version'),
            'value': t.get('value'),
            'lots': [{'key': l.get('key'), 'title': l.get('title')} for l in t.get('lots') or []],
            'lot_results': [{'key': r.get('key'), 'lot': r.get('lot'), 'notice_id': r.get('notice_id'),
                             'awarded': r.get('awarded')} for r in t.get('lot_results') or []],
        }

    json.dump({'taken_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), 'band_min_eur_cents': BAND_MIN,
               'pages': pages, 'band': band, 'path_v': path_v, 'tenders': tenders},
              open(out, 'w'), indent=0, sort_keys=True)
    print(f'band {len(band)} ({pages} pages), path_v {len(path_v)}, tenders {len(tenders)} -> {out}')


if __name__ == '__main__':
    main()
