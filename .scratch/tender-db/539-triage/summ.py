# usage: python3 -I summ.py <out.json>...  — per-fetch totals, ingest time, and per-profile bucket table
import json, sys, datetime, collections
D = '/home/user/tender-db/.scratch/tender-db/539-triage/'
F = {r[0]: r for r in json.load(open(D + 'q01_fetches.out.json'))['rows']}
rows = []
for p in sys.argv[1:]:
    j = json.load(open(p))
    assert 'rows' in j and 'columns' in j and not j.get('truncated'), p
    cols = j['columns']
    rows += [dict(zip(cols, r)) for r in j['rows']]
per_f = collections.defaultdict(lambda: collections.Counter())
ing = {}
for r in rows:
    per_f[r['f']]['n'] += r['n']
    per_f[r['f']]['stale' if r['pb'].startswith('b') else r['pb']] += r['n']
    a, b = ing.get(r['f'], (1e18, 0)); ing[r['f']] = (min(a, r['min_ing']), max(b, r['max_ing']))
for f in sorted(per_f, key=lambda f: F[f][2]):
    c = per_f[f]
    print(f, F[f][1], F[f][2], dict(c), 'stale%%=%.2f' % (100 * c['stale'] / c['n']),
          'ing', datetime.datetime.utcfromtimestamp(ing[f][0]).strftime('%m-%dT%H:%M'),
          datetime.datetime.utcfromtimestamp(ing[f][1]).strftime('%m-%dT%H:%M'))
print('total notices', sum(c['n'] for c in per_f.values()), 'fetches', len(per_f))
