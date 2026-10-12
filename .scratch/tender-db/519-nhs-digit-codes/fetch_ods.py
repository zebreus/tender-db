#!/usr/bin/env python3
"""519 unit 1: fetch the ODS ORD record of every GB-NHS identity code of ODS shape
(3, 5 or 6 alphanumerics) that the 513 frozen table does not carry. Cached per code in ods/."""
import json, os, re, subprocess, sys, time
HERE = os.path.dirname(os.path.abspath(__file__))
BASE = 'https://directory.spineservices.nhs.uk/ORD/2-0-0/organisations/'
tsv = set()
for l in open(os.path.join(HERE, '../../../crates/ingest/sdk/nhs-ods.tsv')):
    if not l.startswith('#') and l.strip():
        tsv.add(l.split('\t')[0])
orgs = json.load(open(os.path.join(HERE, 'gbnhs-orgs-2026-10-12.json')))
codes = sorted({o['identifier'][5:] for o in orgs} - tsv)
codes = [c for c in codes if re.fullmatch(r'[A-Z0-9]{3}|[A-Z0-9]{5}|[A-Z0-9]{6}', c)]
print(len(codes), 'codes to look up', file=sys.stderr)
n = 0
for c in codes:
    path = os.path.join(HERE, 'ods', c + '.json')
    if os.path.exists(path):
        continue
    for attempt in range(5):
        p = subprocess.run(['curl', '-sS', '-m', '60', '-H', 'Accept: application/json', '-w', '\n%{http_code}', BASE + c],
                           capture_output=True, text=True)
        if p.returncode == 0:
            break
        time.sleep(3 * (attempt + 1))
    if p.returncode != 0:
        print('FAIL', c, p.stderr.strip(), file=sys.stderr)
        continue
    body, _, status = p.stdout.rpartition('\n')
    try:
        parsed = json.loads(body)
    except ValueError:
        parsed = None
    with open(path + '.tmp', 'w') as f:
        json.dump({'code': c, 'http': status, 'body': parsed}, f)
    os.replace(path + '.tmp', path)
    n += 1
    if n % 100 == 0:
        print(n, 'fetched', file=sys.stderr)
    time.sleep(0.5)
print('done', n, file=sys.stderr)
