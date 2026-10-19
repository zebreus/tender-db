# usage: python3 -I srcsum.py — for stale non-sdk-0.1 notices: was the served date a stamped publication date (hp=1)
# and where does the requested date fall (rb), per era x profile x served gap (1 / 2-29 / 30+).
import json, collections
D = '/home/user/tender-db/.scratch/tender-db/539-triage/'
F = {r[0]: r for r in json.load(open(D + 'q01_fetches.out.json'))['rows']}
rows = []
for n in ['q24_src', 'q25_src', 'q26_src', 'q27_src']:
    j = json.load(open(D + n + '.out.json')); assert 'rows' in j and not j.get('truncated')
    rows += [dict(zip(j['columns'], r)) for r in j['rows']]
T = collections.defaultdict(collections.Counter)
for r in rows:
    era = 'monthly' if F[r['f']][1] == 'monthly' else 'daily'
    g = {'b001': '1', 'b002': '2-29', 'b030': '30+'}[r['pb']]
    for key in [(era, r['profile'], g), (era, r['profile'], 'ALL'), ('ALL', r['profile'], g), ('ALL', 'ALL', g)]:
        c = T[key]; c['n'] += r['n']; c[f"hp{r['hp']}"] += r['n']; c[f"hp{r['hp']}_{r['rb']}"] += r['n']
print('era | profile | gap | stale | stamped-pub-date served (hp1) | hp1: requested in/after window | hp1: requested 1d | hp1: requested 2-29 | hp1: req 30+ | hp1: no requested | hp0 (requested served) ')
for k in sorted(T):
    c = T[k]
    print(' | '.join(k), '|', c['n'], '|', c['hp1'], '|', c['hp1_in'] + c['hp1_after'], '|', c['hp1_r001'], '|', c['hp1_r002'], '|', c['hp1_r030'], '|', c['hp1_none'], '|', c['hp0'], f"(hp0 rb: in {c['hp0_in']}, 1d {c['hp0_r001']}, 2-29 {c['hp0_r002']}, 30+ {c['hp0_r030']}, none {c['hp0_none']})")
