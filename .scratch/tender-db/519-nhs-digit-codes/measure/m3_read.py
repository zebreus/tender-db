#!/usr/bin/env python3
"""519 M3 reads (read-only; every answer cached under m3-raw/, a re-run reads nothing already cached).

1. The public org detail, GET https://tenders.zebreus.click/v1/organizations/<id> (no token; one PK seek plus
   one `organization_merged_identifiers_org` seek), for each of the 77 child holders (52 sub-ICB + 25 RO198
   sites, unit 1): identifier, country, provisional, mention count, `merged_identifiers`, `identifier_status`
   (the 452 flag). Paced 0.5 s.
2. The verdict-store readbacks through the box helper, GET /admin/case-reviews?table=<t>&limit=5000
   (`Db::verdict_rows`, admin-gated, read-only) for t in identifier, merge, rehoming, case, country, name.

Stops at the first non-JSON / non-2xx answer (never retries).
"""
import json, os, subprocess, sys, time, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, 'm3-raw')
API = os.path.join(RAW, 'api')
TABLES = ['identifier', 'merge', 'rehoming', 'case', 'country', 'name']


def get_api(oid):
    path = os.path.join(API, f'{oid}.json')
    if os.path.exists(path):
        return json.load(open(path))
    req = urllib.request.Request(f'https://tenders.zebreus.click/v1/organizations/{oid}',
                                 headers={'accept': 'application/json'})
    with urllib.request.urlopen(req, timeout=60) as r:
        status = r.status
        body = r.read()
    d = json.loads(body)
    d['_http'] = status
    with open(path + '.tmp', 'w') as f:
        json.dump(d, f)
    os.replace(path + '.tmp', path)
    time.sleep(0.5)
    return d


def get_admin(table):
    path = os.path.join(RAW, f'admin_{table}.json')
    if os.path.exists(path):
        return json.load(open(path))
    r = subprocess.run(['ssh', '-o', 'BatchMode=yes', 'root@zebreus.click',
                        f'/root/aj.sh "/admin/case-reviews?table={table}&limit=5000"'],
                       capture_output=True)
    if r.returncode != 0:
        sys.exit(f'admin {table}: exit {r.returncode} {r.stderr[:300]!r} {r.stdout[:300]!r}')
    try:
        d = json.loads(r.stdout)
    except ValueError:
        sys.exit(f'admin {table}: non-JSON answer {r.stdout[:300]!r}')
    if 'rows' not in d:
        sys.exit(f'admin {table}: error answer {str(d)[:300]}')
    with open(path + '.tmp', 'w') as f:
        json.dump(d, f)
    os.replace(path + '.tmp', path)
    return d


def main():
    os.makedirs(API, exist_ok=True)
    holders = json.load(open(os.path.join(RAW, 'holders.json')))
    assert len(holders) == 77
    for h in holders:
        d = get_api(h['org_id'])
        if d.get('id') != h['org_id']:
            print(f"note: {h['org_id']} answered id {d.get('id')}", file=sys.stderr)
    print(f'api: {len(holders)} holders read')
    for t in TABLES:
        d = get_admin(t)
        print(f"admin {t}: {d['count']} rows (limit {d['limit']})")


if __name__ == '__main__':
    main()
