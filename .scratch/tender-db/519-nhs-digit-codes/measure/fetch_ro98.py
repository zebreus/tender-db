#!/usr/bin/env python3
"""519 M2: fetch (cached) the ODS ORD record of every RO98 row of crates/ingest/sdk/nhs-ods.tsv.

    python3 -I fetch_ro98.py [--offline]

Cache: measure/ods-ro98/<code>.json, wrapped {"code","http","body"} like ../ods/.
Seeded from ../ods-table-codes/ when that record is an HTTP 200 (03K's failed fetch,
http "000", is never seeded; it is fetched first). Exits non-zero on any status other
than 200 or 404. Paced 0.5 s between requests. Responses are external data.
"""
import json, os, subprocess, sys, time
HERE = os.path.dirname(os.path.abspath(__file__))
U1 = os.path.dirname(HERE)
REPO = os.path.abspath(os.path.join(U1, '..', '..', '..'))
TSV = os.path.join(REPO, 'crates', 'ingest', 'sdk', 'nhs-ods.tsv')
CACHE = os.path.join(HERE, 'ods-ro98')
SEED = os.path.join(U1, 'ods-table-codes')
BASE = 'https://directory.spineservices.nhs.uk/ORD/2-0-0/organisations/'
OFFLINE = '--offline' in sys.argv[1:]
fetched = 0


def fetch(code):
    global fetched
    if OFFLINE:
        sys.exit(f'--offline: cache miss for {code}')
    if fetched:
        time.sleep(0.5)
    fetched += 1
    for attempt in range(5):
        p = subprocess.run(['curl', '-sS', '-m', '60', '-H', 'Accept: application/json',
                            '-w', '\n%{http_code}', BASE + code], capture_output=True, text=True)
        if p.returncode == 0:
            break
        print(f'  curl {p.returncode} for {code} (attempt {attempt + 1}): {p.stderr.strip()}', file=sys.stderr)
        time.sleep(3 * (attempt + 1))
    if p.returncode != 0:
        sys.exit(f'curl failed for {code}: {p.stderr.strip()}')
    body, _, status = p.stdout.rpartition('\n')
    try:
        parsed = json.loads(body)
    except ValueError:
        parsed = None
    if status not in ('200', '404'):
        sys.exit(f'{code}: HTTP {status} {body[:200]}')
    if status == '200' and not (isinstance(parsed, dict) and 'Organisation' in parsed):
        sys.exit(f'{code}: HTTP 200 without an Organisation: {body[:200]}')
    return {'code': code, 'http': status, 'body': parsed}


def save(path, d):
    with open(path + '.tmp', 'w', encoding='utf-8') as f:
        json.dump(d, f, ensure_ascii=False)
    os.replace(path + '.tmp', path)


def main():
    os.makedirs(CACHE, exist_ok=True)
    codes = [l.split('\t')[0] for l in open(TSV, encoding='utf-8')
             if not l.startswith('#') and l.strip() and l.split('\t')[3] == 'RO98']
    print(len(codes), 'RO98 codes', file=sys.stderr)
    # 03K first, always a fresh fetch (unit 1's record is a failed fetch).
    order = ['03K'] + [c for c in codes if c != '03K']
    seeded = 0
    for c in order:
        path = os.path.join(CACHE, c + '.json')
        if os.path.exists(path):
            d = json.load(open(path, encoding='utf-8'))
            if d.get('http') in ('200', '404'):
                continue
        s = os.path.join(SEED, c + '.json')
        if c != '03K' and os.path.exists(s):
            d = json.load(open(s, encoding='utf-8'))
            if d.get('http') == '200' and isinstance(d.get('body'), dict) and 'Organisation' in d['body']:
                save(path, d)
                seeded += 1
                continue
        save(path, fetch(c))
        if fetched % 50 == 0:
            print(fetched, 'fetched', file=sys.stderr)
    print(f'done: fetched {fetched}, seeded {seeded}', file=sys.stderr)


if __name__ == '__main__':
    main()
