# usage: python3 -I optsum.py — option (a) residual table from q20..q23 (per era x profile), cross-checked against step 2 totals.
import json, collections
D = '/home/user/tender-db/.scratch/tender-db/539-triage/'
F = {r[0]: r for r in json.load(open(D + 'q01_fetches.out.json'))['rows']}
def era(f): return '2022-12' if f == 444 else ('monthly' if F[f][1] == 'monthly' else 'daily')
rows = []
for n in ['q20_opta', 'q21_opta', 'q22_opta', 'q23_opta']:
    j = json.load(open(D + n + '.out.json')); assert 'rows' in j and not j.get('truncated')
    rows += [dict(zip(j['columns'], r)) for r in j['rows']]
# cross-check vs step 2
s2 = collections.Counter()
for n in ['q03_agg', 'q04_agg', 'q05_agg', 'q06_agg', 'q07_day', 'q08_day', 'q09_day']:
    j = json.load(open(D + n + '.out.json'))
    for r in j['rows']:
        r = dict(zip(j['columns'], r)); s2[(r['f'], r['profile'], r['pb'])] += r['n']
s4 = collections.Counter()
for r in rows: s4[(r['f'], r['profile'], r['pb'])] += r['n']
print('cross-check vs step 2:', 'IDENTICAL' if s2 == s4 else 'DIFF', sum(s2.values()), sum(s4.values()))
T = collections.defaultdict(collections.Counter)
for r in rows:
    if not r['pb'].startswith('b'): 
        T[(era(r['f']), r['profile'])]['nonstale'] += r['n']
        if r['ra'] == 'after': T[(era(r['f']), r['profile'])]['nonstale_pushed_after'] += r['n']
        continue
    k = (era(r['f']), r['profile'])
    g2 = r['pb'] != 'b001'
    T[k]['stale'] += r['n']; T[k]['stale>=2'] += r['n'] if g2 else 0
    T[k]['ra_' + r['ra']] += r['n']
    if g2: T[k]['g2_ra_' + r['ra']] += r['n']
order = ['in', 'after', 'a001', 'a002', 'a030', 'a180', 'a365']
for k in sorted(T, key=lambda k: (k[0], -T[k]['stale'])):
    c = T[k]
    if not c['stale']: continue
    fixed = c['ra_in'] + c['ra_after']
    print(f"{k[0]:8s} {k[1]:24s} stale={c['stale']:7d} fixed_by_a={fixed:6d} ({100*fixed/c['stale']:5.1f}%) residual: " +
          ' '.join(f"{o}={c['ra_'+o]}" for o in order) +
          f" | gap>=2: {c['stale>=2']} -> residual " + ' '.join(f"{o}={c['g2_ra_'+o]}" for o in order) +
          f" | nonstale pushed past window end by (a): {c['nonstale_pushed_after']}")
json.dump({' / '.join(k): v for k, v in T.items()}, open(D + 'opta_table.json', 'w'), indent=0)
