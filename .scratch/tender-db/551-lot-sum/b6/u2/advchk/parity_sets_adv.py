"""Adversarial recount of B6 U2's v2 parity and guard sets, independent of run_u2.py's comparison code.

python3 -I -B parity_sets_adv.py DECISIONS.jsonl.gz OUT.json
Reads: served_lsa_rows.json, the decisions file (the port's per-id output), the b6 tender bodies (to recompute the
guard flags from the served lot_results), and run_u2's P.json / R_single.json / R_excess.json / u2-result.json /
parity_v2.json (to compare with). Writes OUT.json only.
"""
import collections
import gzip
import json
import os
import sys

U2 = '/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/u2'
ROOT = os.path.dirname(os.path.dirname(U2))
TEN = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/b6/tenders'
NAMES = ('v2', 'v2+single', 'v2+cover', 'v2+both')


def key(c):
    return (c['field'], int(c['notice_id']), int(c['cents']))


def main(dec_path, out):
    rows = json.load(open(os.path.join(ROOT, 'b5/investigate/served_lsa_rows.json')))
    served = {int(r[0]): r for r in rows}
    ids = sorted(t for t in served if t != 179732)
    assert len(ids) == 3502 and len(served) == 3503
    recs = {}
    for line in gzip.open(dec_path, 'rt', encoding='utf-8'):
        r = json.loads(line)
        assert r['id'] not in recs
        recs[r['id']] = r
    missing = sorted(set(ids) - set(recs))
    extra = sorted(set(recs) - set(ids))
    # ---- parity, recounted
    dis = collections.defaultdict(list)
    not_first = []
    for t in ids:
        row = served[t]
        assert row[3] == 'lot-sum-awards'
        det = json.loads(row[7])
        v2 = recs[t]['decisions']['v2']
        if not v2['ship']:
            dis[t].append('ship')
            continue
        if not (isinstance(row[4], int) and row[4] == v2['S']):
            dis[t].append(('S', row[4], v2['S']))
        if row[5] != v2['currency']:
            dis[t].append(('cur', row[5], v2['currency']))
        if row[6] != row[4]:
            dis[t].append(('row6!=row4', row[6], row[4]))     # informative: served value vs S
        if det.get('lots') != v2['n']:
            dis[t].append(('n', det.get('lots'), v2['n']))
        if det.get('summed') != 'awards':
            dis[t].append(('summed', det.get('summed')))
        vb = det.get('vouched_by')
        vk = (vb['field'], int(vb['notice_id']), int(vb['cents'])) if vb else None
        vouches = [tuple(v) for v in v2['vouches']]
        if vk not in vouches:
            dis[t].append(('vouched_by', vk, vouches))
        first = v2['vouched_by']
        if not first or vk != (first['field'], int(first['notice_id']), int(first['cents'])):
            not_first.append(t)
        if set(det) - {'summed', 'lots', 'vouched_by'}:
            dis[t].append(('extra detail keys', sorted(det)))
    # ---- guard sets, recounted from ship flags
    ships = {nm: {t for t in ids if recs[t]['decisions'][nm]['ship']} for nm in NAMES}
    subset = {nm: ships[nm] <= ships['v2'] for nm in NAMES}
    R = {nm: sorted(ships['v2'] - ships[nm]) for nm in NAMES[1:]}
    R_single = R['v2+single']
    R_excess_cover = sorted(set(R['v2+cover']) - set(R_single))
    R_excess_both = sorted(set(R['v2+both']) - set(R_single))
    # ---- guard flags recomputed from the served lot_results (independent of the port's cand_flags)
    flag_mism = collections.Counter()
    flag_mism_ex = collections.defaultdict(list)
    dec_mism = collections.Counter()
    dec_mism_ex = collections.defaultdict(list)
    kinds_by_field = collections.defaultdict(collections.Counter)
    for t in ids:
        rec = recs[t]
        with gzip.open(os.path.join(TEN, f'{t}.json.gz'), 'rt', encoding='utf-8') as fh:
            x = json.load(fh)
        lrs = x.get('lot_results') or []
        keys = {s[0] for s in rec['summed']}
        cands = rec['vouch_candidates'] or []
        v2 = rec['decisions']['v2']
        if [key(c) for c in cands] != [tuple(v) for v in v2['vouches']] and v2['ship']:
            dec_mism['v2_vouches_ne_candidates'] += 1
            dec_mism_ex['v2_vouches_ne_candidates'].append(t)
        my = {}
        for c in cands:
            kinds_by_field[c['field']][c['kind']] += 1
            n = int(c['notice_id'])
            rr = sum(1 for lr in lrs if lr.get('notice_id') == n)
            names = sorted({lr['lot'] for lr in lrs if lr.get('notice_id') == n and lr.get('lot') not in (None, '')
                            and lr['lot'] in keys})
            kind = 'range' if c['field'] in ('TED-LOW', 'TED-HIGH') else c['kind']
            g_single = rr == 1
            g_cover = kind in ('award', 'range') and not names
            for nm, mine, theirs in (('round_results', rr, c['round_results']), ('names', names, sorted(c['names_summed_keys'])),
                                     ('g_single', g_single, c['g_single']), ('g_cover', g_cover, c['g_cover']),
                                     ('kind_range', kind, c['kind'])):
                if mine != theirs:
                    flag_mism[nm] += 1
                    if len(flag_mism_ex[nm]) < 10:
                        flag_mism_ex[nm].append([t, key(c), mine, theirs])
            my[key(c)] = (g_single, g_cover)
        for nm, gs in (('v2+single', ('single',)), ('v2+cover', ('cover',)), ('v2+both', ('single', 'cover'))):
            d = rec['decisions'][nm]
            if not v2['ship']:
                continue
            kept = [key(c) for c in cands if not (('single' in gs and my[key(c)][0]) or ('cover' in gs and my[key(c)][1]))]
            exp_ship = bool(kept)
            if exp_ship != d['ship']:
                dec_mism[f'{nm}:ship'] += 1
                dec_mism_ex[f'{nm}:ship'].append(t)
            if [tuple(v) for v in d['vouches']] != kept:
                dec_mism[f'{nm}:vouches'] += 1
                dec_mism_ex[f'{nm}:vouches'].append(t)
            if d['ship']:
                fvb = d['vouched_by']
                if not fvb or (fvb['field'], int(fvb['notice_id']), int(fvb['cents'])) != kept[0]:
                    dec_mism[f'{nm}:vouched_by_not_first_kept'] += 1
                    dec_mism_ex[f'{nm}:vouched_by_not_first_kept'].append(t)
                for k in ('S', 'currency', 'n'):
                    if d[k] != v2[k]:
                        dec_mism[f'{nm}:{k}_moved'] += 1
            else:
                if d['first'] != 'R10' or d['refusals'] != ['R10']:
                    dec_mism[f'{nm}:refusal_not_R10'] += 1
                    dec_mism_ex[f'{nm}:refusal_not_R10'].append([t, d['first'], d['refusals']])
    # ---- compare with run_u2's files
    P = json.load(open(os.path.join(U2, 'P.json')))['guard_sets']
    Rs = json.load(open(os.path.join(U2, 'R_single.json')))
    Rx = json.load(open(os.path.join(U2, 'R_excess.json')))
    res_u2 = json.load(open(os.path.join(U2, 'u2-result.json')))
    par = json.load(open(os.path.join(U2, 'parity_v2.json')))
    cmp_ = {
        'P_ids_equal': {nm: sorted(e['id'] for e in P[nm]['ships']) == sorted(ships[nm]) for nm in NAMES},
        'P_n': {nm: P[nm]['n'] for nm in NAMES},
        'R_single_equal': Rs['ids'] == R_single, 'R_excess_equal': Rx['ids'] == R_excess_cover,
        'R_excess_alt_equal': Rx['alt_R_both_minus_R_single'] == R_excess_both,
        'u2_counts': {k: res_u2['counts'][k] for k in ('v2_ships', 'P_v2+single', 'P_v2+cover', 'P_v2+both', 'R_single',
                                                        'R_excess', 'R_cover_all')},
        'u2_parity': {k: par[k] for k in ('n_compared', 'n_agree', 'n_disagree', 'n_exempt')},
    }
    # P entries carry S/currency/n/vouched_by equal to the decisions
    p_ent_bad = 0
    for nm in NAMES:
        for e in P[nm]['ships']:
            d = recs[e['id']]['decisions'][nm]
            if (e['S'], e['currency'], e['n'], e['vouched_by'], e['vouches']) != (d['S'], d['currency'], d['n'],
                                                                                d['vouched_by'], d['vouches']):
                p_ent_bad += 1
    known = {'6465510_in_R_single': 6465510 in R_single, '6465510_in_R_cover': 6465510 in R['v2+cover'],
             '6595330_in_R_single': 6595330 in R_single, '6595330_in_R_cover': 6595330 in R['v2+cover']}
    res = {
        'n_ids': len(ids), 'n_decided': len(recs), 'missing': missing, 'extra': extra,
        'n_disagree': len(dis), 'disagreements': {str(k): v for k, v in list(dis.items())[:50]},
        'disagree_kinds': dict(collections.Counter((v[0] if isinstance(v, tuple) else v) for vs in dis.values() for v in vs)),
        'n_vouched_by_not_first': len(not_first), 'not_first': not_first[:50],
        'ships': {nm: len(ships[nm]) for nm in NAMES}, 'subset_of_v2': subset,
        'R': {nm: len(v) for nm, v in R.items()}, 'R_single': R_single, 'R_cover': R['v2+cover'], 'R_both': R['v2+both'],
        'R_excess_cover_minus_single': R_excess_cover, 'R_excess_both_minus_single': R_excess_both,
        'R_cover_not_in_single': sorted(set(R['v2+cover']) - set(R_single)),
        'R_single_not_in_cover': len(set(R_single) - set(R['v2+cover'])),
        'R_both_eq_union': set(R['v2+both']) == set(R_single) | set(R['v2+cover']),
        'flag_mismatch': dict(flag_mism), 'flag_mismatch_examples': dict(flag_mism_ex),
        'decision_mismatch': dict(dec_mism), 'decision_mismatch_examples': {k: v[:10] for k, v in dec_mism_ex.items()},
        'kinds_by_field': {f: dict(c) for f, c in kinds_by_field.items()},
        'compare_with_run_u2': cmp_, 'P_entries_mismatch': p_ent_bad, 'known': known,
    }
    with open(out, 'w') as fh:
        json.dump(res, fh, indent=1, sort_keys=True, default=list)
    print(json.dumps({k: v for k, v in res.items() if k not in ('R_single',)}, indent=1, default=list)[:9000])


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
