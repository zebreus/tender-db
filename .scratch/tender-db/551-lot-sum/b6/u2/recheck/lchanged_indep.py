"""Adversarial re-derivation of U2's provisional L_changed from the saved change pages (independent of run_u2.py)."""
import collections, glob, gzip, hashlib, json, os, sys
U2 = '/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/u2'
B6 = os.path.dirname(U2)
ROOT = os.path.dirname(B6)
SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'
TEN = SP + '/b6/tenders'
rows = json.load(open(ROOT + '/b5/investigate/served_lsa_rows.json'))
ids = sorted({int(r[0]) for r in rows} - {179732})
print('served rows', len(rows), 'distinct', len({int(r[0]) for r in rows}), 'ids', len(ids))
S = set(ids)
man = json.load(open(U2 + '/api/manifest.json'))
ev = []
since = 958787061
for rec in sorted(man['changes'], key=lambda r: r['i']):
    p = U2 + '/api/' + rec['file']
    b = open(p, 'rb').read()
    assert hashlib.sha256(b).hexdigest() == rec['sha256']
    d = json.loads(b)
    assert rec['path'] == f'/v1/changes?since={since}&entity=tender&limit=1000', rec['path']
    assert d['ignored_filters'] == [], d['ignored_filters']
    ev += d['events']
    since = int(d['last_cursor'])
    last_more = d['more']
assert last_more is False
print('events', len(ev), 'head', since)
types = collections.Counter(type(e['id']).__name__ for e in ev); print('id types', types)
ents = collections.Counter(e['entity'] for e in ev); print('entities', ents)
cur = [int(e['cursor']) for e in ev]
assert cur == sorted(cur) and len(set(cur)) == len(cur) and cur[0] > 958787061
# group by changed_at
g = collections.OrderedDict()
for e in ev:
    g.setdefault(e['changed_at'], []).append(e)
for k, v in g.items():
    vi = {e['id'] for e in v}
    print(k, 'rows', len(v), 'ids', len(vi), 'in3502', len(vi & S), 'cursor', v[0]['cursor'], v[-1]['cursor'],
          dict(collections.Counter((e['op'], e['version'] is None) for e in v)))
# 2630
r2630 = [e for e in ev if 958787061 < int(e['cursor']) <= 958809778]
print('2630 range rows', len(r2630), 'ids', len({e['id'] for e in r2630}), 'eq3502', {e['id'] for e in r2630} == S,
      'stamps', collections.Counter(e['changed_at'] for e in r2630), 'ops', collections.Counter((e['op'], e['version']) for e in r2630))
after = [e for e in ev if int(e['cursor']) > 958809778]
print('rows after 2630 range', len(after), 'touching 3502:', sorted({e['id'] for e in after} & S)[:20], len({e['id'] for e in after} & S))
# any 03:34 stamp after the range?
print('03:3x rows after range', sum(1 for e in after if e['changed_at'].startswith('2026-10-19T03:3')))
r541 = sorted({e['id'] for e in ev if 958809778 < int(e['cursor']) <= 958809905})
r571 = sorted({e['id'] for e in ev if 958809905 < int(e['cursor']) <= 958810037})
f571 = sorted(set(json.load(open(B6 + '/l_changed_571_ids.json'))))
f545 = sorted(set(json.load(open(B6 + '/l_changed_545_ids.json'))))
print('541 range', len(r541), 'rows', sum(1 for e in ev if 958809778 < int(e['cursor']) <= 958809905))
print('571 range', len(r571), 'file', len(f571), 'equal', r571 == f571)
s1106 = sorted({e['id'] for e in ev if e['changed_at'] == '2026-10-19T11:06:21Z'})
print('11:06:21 ids', len(s1106), '545 file', len(f545), 'equal', s1106 == f545, 'file-only', sorted(set(f545) - set(s1106)), 'feed-only', len(set(s1106) - set(f545)))
# rows after 958848408 (probe cursor)
late = [e for e in ev if int(e['cursor']) > 958848408]
print('rows after 958848408', len(late), collections.Counter(e['changed_at'] for e in late))
chain = [e for e in ev if 958810037 < int(e['cursor']) <= 958848408]
print('chain range rows', len(chain), 'ids', len({e['id'] for e in chain}), collections.Counter(e['changed_at'] for e in chain))
# fetch times
fts = {}
for t in ids:
    m = json.load(open(f'{TEN}/{t}.meta.json'))
    fts[t] = m['fetched_at']
sec = [t for t, v in fts.items() if '.' not in v]
print('fetch range', min(fts.values()), max(fts.values()), 'second-precision', len(sec))
# fetch time vs 2630 stamp: any fetched before 03:34:07?
print('fetched before 2026-10-19T03:34:07Z', sum(1 for v in fts.values() if v < '2026-10-19T03:34:07'))
print('fetched before 07:35:55Z chain stamp', sum(1 for v in fts.values() if v < '2026-10-19T07:35:55'),
      'before 11:06:21Z', sum(1 for v in fts.values() if v < '2026-10-19T11:06:21'))
# per-id (a) literal and operative
lit = {t for t in ids if any(e['id'] == t and e['changed_at'] <= fts[t].replace('Z','') + 'Z' for e in ev)}
union = sorted(set(r541) | set(f571) | set(f545))
print('union 541|571|545', len(union), 'pairwise overlaps', len(set(r541)&set(f571)), len(set(r541)&set(f545)), len(set(f571)&set(f545)), 'in3502', len(set(union)&S))
prov = json.load(open(U2 + '/l_changed_provisional.json'))
print('U2 provisional n', prov['n'], 'equal to my union', prov['l_changed_provisional'] == union)
print('all non-2630 tender ids in feed', len({e['id'] for e in after}))
json.dump({'fetch': fts}, open(U2 + '/recheck/fetch_times.json', 'w'))
