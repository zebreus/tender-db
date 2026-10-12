#!/usr/bin/env python3
"""519 M4 reads (bounded, indexed; through /root/sq.sh on the box).

1. organizations PK: the 52 sub-ICB + 25 RO198 site holders still stand (id, identifier, name).
2. organization_mentions_org: every mention of those holders (IN lists of <= 20 org ids, LIMIT 5000).
3. notices PK: published_at / published_offset per mentioned notice (IN lists of <= 100, LIMIT 100).

Each read's raw JSON answer is cached under m4-raw/; a re-run reads nothing already cached.
Stops at the first non-JSON / error answer (never retries a 408).
"""
import json, os, subprocess, sys
HERE = os.path.dirname(os.path.abspath(__file__))
U1 = os.path.dirname(HERE)
RAW = os.path.join(HERE, 'm4-raw')


def sql(tag, q):
    path = os.path.join(RAW, tag + '.json')
    if os.path.exists(path):
        return json.load(open(path))
    r = subprocess.run(['ssh', '-o', 'BatchMode=yes', 'root@zebreus.click', '/root/sq.sh'],
                       input=q.encode(), capture_output=True)
    try:
        d = json.loads(r.stdout)
    except ValueError:
        sys.exit(f'{tag}: non-JSON answer: {r.stdout[:300]!r} {r.stderr[:300]!r}')
    if 'rows' not in d:
        sys.exit(f'{tag}: error answer {str(d)[:300]}')
    if d.get('truncated'):
        sys.exit(f'{tag}: truncated')
    d['_sql'] = q
    with open(path + '.tmp', 'w') as f:
        json.dump(d, f)
    os.replace(path + '.tmp', path)
    return d


def chunks(xs, n):
    for i in range(0, len(xs), n):
        yield xs[i:i + n]


def main():
    os.makedirs(RAW, exist_ok=True)
    cls = json.load(open(os.path.join(U1, 'gbnhs-orgs-classified.json')))
    holders = [o for o in cls if o['ods'].get('primary') in ('RO98', 'RO198')]
    ids = sorted(o['id'] for o in holders)
    assert len(ids) == 77, len(ids)
    orgs = []
    for i, ch in enumerate(chunks(ids, 100)):
        d = sql(f'orgs_{i}', 'SELECT id, identifier, name, provisional FROM organizations WHERE id IN ('
                + ', '.join(map(str, ch)) + f') LIMIT {len(ch)}')
        orgs += d['rows']
    ments = []
    for i, ch in enumerate(chunks(ids, 20)):
        d = sql(f'ments_{i}', 'SELECT organization_id, notice_id, section_id, name, country, raw_identifier, scheme '
                'FROM organization_mentions WHERE organization_id IN (' + ', '.join(map(str, ch)) + ') LIMIT 5000')
        ments += d['rows']
    nids = sorted({m[1] for m in ments})
    notices = []
    for i, ch in enumerate(chunks(nids, 100)):
        d = sql(f'notices_{i}', 'SELECT id, published_at, published_offset, published_has_time, source, profile '
                'FROM notices WHERE id IN (' + ', '.join(map(str, ch)) + f') LIMIT {len(ch)}')
        notices += d['rows']
    json.dump({'orgs': orgs, 'mentions': ments, 'notices': notices},
              open(os.path.join(HERE, 'm4-reads.json'), 'w'))
    print(f'orgs {len(orgs)}/77; mentions {len(ments)}; notices {len(notices)}/{len(nids)}')


if __name__ == '__main__':
    main()
