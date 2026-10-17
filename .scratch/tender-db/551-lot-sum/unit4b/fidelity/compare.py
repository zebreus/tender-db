"""Compare the independent derivation (fid/derived.json) with the port's walk facts (walk_facts/{id}.json)."""
import json
import sys

import os
BASE = os.environ.get('M551', '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/m551')
D = json.load(open(BASE + '/fid/derived.json'))
out = {}
for t, F in D.items():
    P = json.load(open(f'{BASE}/walk_facts/{t}.json'))
    mm = []

    def cmp(name, mine, port):
        if mine != port:
            mm.append({'fact': name, 'expected': mine, 'port': port})
    v1 = P['v1']
    cmp('v1 fires (T\', K, G0 all pass)', bool(F['T_prime_ok'] and F['K_ok'] and F['G0_ok']), v1['fires'])
    cmp('currency', F['currency'], P['currency'])
    cmp('S (cents)', F['S'], P['S'])
    cmp('n', F['n'], P['n'])
    cmp('S_eur (cents)', F['S_eur_cents'], P['S_eur'])
    cmp('served head', F['served'], v1['served'])
    ps = {s['key']: s for s in P['summed']}
    ms = {s['key']: s for s in F['summed']}
    cmp('summed keys', sorted(ms), sorted(ps))
    for k in sorted(set(ms) & set(ps)):
        a, b = ms[k], ps[k]
        cmp(f'{k} award notice', a['notice'], b['notice_id'])
        cmp(f'{k} result key', a['result_key'], b['result_key'])
        cmp(f'{k} cents', a['cents'], b['cents'])
        cmp(f'{k} published lot no', a['pn_frozen'], b['published_lot_no'])
        api_ids = [w[0] for w in a['api_winners']]
        cmp(f'{k} winner ids', api_ids, b['contractor_ids'])
        nwin_notice = len({o for w in (a['winners_notice'] or []) for o in w})
        if nwin_notice != len(b['contractor_ids']):
            mm.append({'fact': f'{k} winner count (notice vs port/API)', 'expected': nwin_notice, 'port': len(b['contractor_ids']),
                       'note': a['win_src']})
    pr = P['pred']
    cmp('R1 keys', sorted(s['key'] for s in F['summed'] if s['R1']), sorted(k for k, _ in pr['R1']))
    cmp('R2a', [list(x) for x in F['R2a']], [list(x) for x in pr['R2a']])
    cmp('R2b', F['R2b'], pr['R2b'])
    cmp('R2c keys', [k for k, _ in F['R2c']], pr['R2c'])
    cmp('R2d', F['R2d'], pr['R2d'])
    cmp('R3', [[r[0], r[1], r[2], r[3]] for r in F['R3']], [list(r) for r in pr['R3']])
    cmp('R10', F['R10'], pr['R10'])
    cmp('R6', [list(x) for x in F['R6']], [list(x) for x in pr['R6']])
    cmp('R7', [[r[0], r[1], r[2]] for r in F['R7']], [list(r) for r in pr['R7']])
    cmp('R8', F['R8'], pr['R8'])
    mt = sorted((t_['nid'], t_['field'], t_['cents'], t_['cur'], t_['kind3']) for t_ in F['stated_totals'])
    pt = sorted((t_['notice_id'], t_['field'], t_['cents'], t_['currency'], t_['r3_kind'] or 'range') for t_ in P['stated_totals'])
    cmp('stated totals (nid, field, cents, cur, kind)', mt, pt)
    cmp('vouches', sorted(map(list, F['vouches'])), sorted(map(list, P['vouches'])))
    out[t] = mm
    print(f'== {t}: {len(mm)} mismatch(es)')
    for m in mm:
        print('   ', json.dumps(m, default=str, ensure_ascii=False)[:600])
