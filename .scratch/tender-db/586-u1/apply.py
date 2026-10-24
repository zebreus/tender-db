"""Issue 586 unit 1 (a): apply fold.py's tiers to the saved populations (no box reads).

usage: python3 -I apply.py <audit-576-fanout dir> [--dump]

Population P1: the 3,751 new orgs (second-look/raw/t2_classified.json: the org's name, the PRINTED name of its
first mention's parent ORG in the same notice `pp`, and its touchpoint mentions `tpo`). refute/raw/04's parent
is the same mention's (checked: 3,751/3,751 same parent id and name).
Population P2: the two notice windows (second-look/raw/03-*, 05-*): every touchpoint mention, grouped by
(bound org, parent org), with min(tp printed) and min(parent printed).
Salted 40 per tier: the tier's population ordered by sha256("<salt>:<org id>") hex, first 40.
"""
import hashlib
import json
import os
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fold  # noqa: E402

D = sys.argv[1]
t2 = json.load(open(os.path.join(D, 'second-look/raw/t2_classified.json')))


def salted(rows, salt, n=40):
    return sorted(rows, key=lambda r: hashlib.sha256(f"{salt}:{r['id']}".encode()).hexdigest())[:n]


res = defaultdict(list)
for r in t2:
    t = fold.tier(r['name'], r['pp'])
    r['tier'] = t
    res[t].append(r)

print('P1: 3,751 new orgs, by tier (orgs / their touchpoint mentions)')
tot_o, tot_m = len(t2), sum(r['tpo'] for r in t2)
for k in ['A', 'C0', 'C1', 'C2', 'C3', 'C3r', None]:
    rows = res.get(k, [])
    print(f'  {str(k):5} {len(rows):5} orgs {sum(r["tpo"] for r in rows):6} mentions')
a = res['A']
c = res['C0'] + res['C1'] + res['C2'] + res['C3'] + res['C3r']
print(f'  tier A  {len(a)} orgs / {sum(r["tpo"] for r in a)} mentions; tier C {len(c)} / {sum(r["tpo"] for r in c)}')
print(f'  of {tot_o} orgs / {tot_m} mentions')

# second-look's classes against ours
x = Counter((r['cls'], r['tier'] and r['tier'][0]) for r in t2)
print('second-look class x tier:')
for cls in sorted({r['cls'] for r in t2}):
    print(f'  {cls:15}', {str(k): v for (c2, k), v in x.items() if c2 == cls})

# windows
for f in ['03-t3_window_a.json', '05-t5_window_b.json']:
    w = json.load(open(os.path.join(D, 'second-look/raw', f)))
    G = [dict(zip(w['columns'], row)) for row in w['rows']]
    cnt = Counter()
    tot = 0
    for g in G:
        tot += g['n']
        if g['porg'] is None or g['org'] == g['porg']:
            continue
        t = fold.tier(g['tp_printed'] or '', g['p_printed'] or '')
        side = 'new' if g['org'] >= 31974972 else 'old'
        if t:
            cnt[(side, t[0])] += g['n']
    print(f'P2 {f}: {tot} touchpoint mentions; would alias (mentions):', dict(cnt))

if '--vectors' in sys.argv:
    # The golden vectors for the Rust port: every pair of P1 with both keys and the tier.
    out = sys.argv[sys.argv.index('--vectors') + 1]
    vec = [dict(id=r['id'], tp=r['name'], parent=r['pp'], key_tp=fold.key_a(r['name'], 'tp', r['pp']),
                key_parent=fold.key_a(r['pp']), tier=r['tier']) for r in sorted(t2, key=lambda r: r['id'])]
    json.dump(vec, open(out, 'w'), ensure_ascii=False, indent=0)
    print(f'wrote {len(vec)} vectors to {out}')

if '--dump' in sys.argv:
    for k in ['A', 'C']:
        rows = a if k == 'A' else c
        print(f'\n== all tier {k} ({len(rows)})')
        for r in sorted(rows, key=lambda r: r['id']):
            print(f"  {r['id']} {r['tier']:3} {r['tpo']:4} {r['name'][:80]!r} <- {r['pp'][:90]!r}")

if '--salted' in sys.argv:
    for k, salt in [('A', '586-u1-tierA'), ('C', '586-u1-tierC')]:
        rows = a if k == 'A' else c
        print(f'\n== salted {k} ({salt}), {min(40, len(rows))} of {len(rows)}')
        for i, r in enumerate(salted(rows, salt), 1):
            print(f"  {i:2} {r['id']} {r['tier']:3} {r['tpo']:4} parent {r['pp'][:100]!r} -> tp {r['name'][:100]!r}")
