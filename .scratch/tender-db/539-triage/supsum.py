# usage: python3 -I supsum.py — step-3 table from q10..q17 sup reads: per era x profile x gap bucket x IssueDate present:
# stale notices, with a version, in a multi-version tender, reorder flags.
import json, collections
D = '/home/user/tender-db/.scratch/tender-db/539-triage/'
F = {r[0]: r for r in json.load(open(D + 'q01_fetches.out.json'))['rows']}
rows = []
for n in ['q10_sup', 'q11_sup', 'q12_sup', 'q13_sup', 'q14_sup', 'q15_sup', 'q16_sup444a', 'q17_sup444b']:
    j = json.load(open(D + n + '.out.json')); assert 'rows' in j and not j.get('truncated')
    rows += [dict(zip(j['columns'], r)) for r in j['rows']]
def era(f): return '2022-12 initial' if f == 444 else ('monthly 2023-01..2026-07' if F[f][1] == 'monthly' else 'daily 2026-07-22..10-18')
K = ['stale', 'with_v', 'vers', 'multi', 'r_doe', 'r_ted', 'r_pub']
T = collections.defaultdict(collections.Counter)
for r in rows:
    gap = {'b001': '1', 'b002': '2-29', 'b030': '30-179', 'b180': '180-364', 'b365': '365+'}[r['pb']]
    for key in [(era(r['f']), r['profile'], gap, r['d']), (era(r['f']), r['profile'], 'ALL', r['d']), (era(r['f']), r['profile'], '>=2', r['d']) if gap != '1' else None,
                ('ALL', r['profile'], 'ALL', r['d']), ('ALL', r['profile'], '>=2', r['d']) if gap != '1' else None, ('ALL', r['profile'], '>=30', r['d']) if gap not in ('1', '2-29') else None]:
        if key is None: continue
        for k in K: T[key][k] += r[k] or 0
print('era | profile | gap | IssueDate | ' + ' | '.join(K))
for key in sorted(T):
    c = T[key]
    print(' | '.join(map(str, key)) + ' | ' + ' | '.join(str(c[k]) for k in K))
json.dump({' / '.join(map(str, k)): v for k, v in T.items()}, open(D + 'sup_table.json', 'w'), indent=0)
