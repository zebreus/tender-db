"""Issue 551 B6 U1: offline self-tests of b6/rule_v2_g.py on the small fetch (b6/selftest_ids.json: the 61 + 8).

    python3 -I -B selftest_g.py [--out selftest_g.json]

Every network path is disabled for the run (socket connect and urllib's urlopen raise), so a pass proves the rule
decides from the caches alone. The tests (issue 551 "B6 -- decision"; the U1 task):
  T0  guard mechanics. (a) guarded_vouches on hand-set flags: an estimate is never removed by G-cover; a candidate
      both guards remove is listed under both; guards=frozenset() removes nothing. (b) the rule's own per-candidate
      derivation, cand_flags(), on real-shaped inputs (decision A2): a TED-HIGH (r3_kind None) whose notice holds no
      row on a summed key is kind 'range' and G-cover removes it; with one keyed row on a summed key it is kept; a
      keyless row (lot None or '') never names a key; an estimate total naming no key is kept by G-cover; exactly one
      lot_results row on the notice, keyed or not, trips G-single. (c) the two guards disagree, end to end through
      decide_facts on 6465510's real facts with its candidates replaced: a TED-HIGH from a notice with two lot=null
      rows ('cover' refuses at R10, 'single' ships) and a total from a notice with exactly one keyed LotResult naming
      a summed key ('single' refuses at R10, 'cover' ships).
  T1  6465510 refuses at R10 under 'single', under 'cover' and under both, from its real contents (v2 ships it);
      its candidate on 20772908 is kind 'award', round_results 1, names no summed key, and is the one listed in
      g_single_removed under 'single' and in g_cover_removed under 'cover'.
  T2  7749755, 6779279, 6850401, 7698260 (identity vouches) still ship, with v2's S, under 'single', 'cover', both.
  T3  every other head of the 61 is unchanged vs v2 under 'single', 'cover', both: (ship, first, S, currency, n).
      A moved vouched_by on an id that still ships is listed (G can shift the first vouch), not a failure.
  T4  port v2 (no guards) reproduces served_lsa_rows.json for the 61: ship, S, currency, n, and the served vouched_by
      is a MEMBER of the port's vouch list.
  T5  the pins 585917 (EUR 250,000.00), 5995030 (PLN 1,549,511.60), 8426318 (GBP 3,000,000,000.00) ship with their
      S under v2, 'single', 'cover' and both, and port v2 matches served_lsa_rows for them (T4's comparison);
      every TED-LOW / TED-HIGH candidate of 8426318 is kind 'range' (and there is at least one).
  T6  the partial sums 6679375, 7153588, 7398935, 7722197 (decision U1: "so do the partial sums") ship under v2 and
      under 'single', 'cover' and both with v2's S, and port v2 matches served_lsa_rows for them.
  T7  a real fixture that separates the guards: 6595330 is refused at R10 by 'single' and by both, and ships under
      v2 and 'cover' (its TED-VAL_TOTAL notice has one lot_results row and names a summed key).
  X   recorded, not a bar: 5816379 (the superseded-vouch residual G is not expected to catch), with its T4 comparison.
"""
import argparse
import hashlib
import importlib.util
import json
import os
import socket
import sys
import time
import traceback
import urllib.request

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
RULE = os.path.join(HERE, 'rule_v2_g.py')
FETCH = os.path.join(HERE, 'fetch_b6.py')
SERVED_LSA = os.path.join(ROOT, 'b5', 'investigate', 'served_lsa_rows.json')
SERVED_SNAP = os.path.join(ROOT, 'b5', 'investigate', 'served')
IDS = os.path.join(HERE, 'selftest_ids.json')
TARGET = 6465510
IDENTITY = [7749755, 6779279, 6850401, 7698260]
PINS = {585917: (25000000, 'EUR'), 5995030: (154951160, 'PLN'), 8426318: (300000000000, 'GBP')}
PARTIAL = [6679375, 7153588, 7398935, 7722197]
RESIDUAL = [5816379]
SEPARATES = 6595330
T1_NOTICE = 20772908
TESTS = ('T0', 'T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7')
GUARDED = ['v2+single', 'v2+cover', 'v2+both']


def sha(p):
    with open(p, 'rb') as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def no_network():
    def refuse(*a, **k):
        raise RuntimeError('selftest_g: network disabled (the B6 rule must decide from caches only)')
    socket.socket.connect = refuse
    socket.create_connection = refuse
    urllib.request.urlopen = refuse


def load_rule():
    spec = importlib.util.spec_from_file_location('rule_v2_g_selftest', RULE)
    R = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(R)
    return R


def tup(d):
    return (d['ship'], d['first'], d['S'], d['currency'], d['n'])


def t0(R):
    c_est = {'field': 'TED-VAL_ESTIMATED_TOTAL', 'notice_id': 1, 'cents': 100, 'kind': 'estimate',
             'g_single': False, 'g_cover': False}
    c_both = {'field': 'TED-VAL_TOTAL', 'notice_id': 2, 'cents': 100, 'kind': 'award', 'g_single': True, 'g_cover': True}
    c_rng = {'field': 'TED-HIGH', 'notice_id': 3, 'cents': 100, 'kind': 'range', 'g_single': False, 'g_cover': True}
    f = {'vouch_candidates': [c_est, c_both, c_rng]}
    out = {}
    k, rs, rc = R.guarded_vouches(f, frozenset())
    out['none_removes_nothing'] = (len(k) == 3 and not rs and not rc)
    k, rs, rc = R.guarded_vouches(f, frozenset({'single', 'cover'}))
    out['both_lists_both'] = (k == [c_est] and rs == [c_both] and rc == [c_both, c_rng])
    k, rs, rc = R.guarded_vouches(f, frozenset({'cover'}))
    out['estimate_kept_by_cover'] = (c_est in k and c_rng not in k)
    # the kind mapping the port uses (decision A2): Range = field in TED-LOW/HIGH (r3_kind None), Award = r3_kind award
    out['range_kind_is_by_field'] = (R.stated_kind('TED-LOW', 'Notice', '3') == (None, True)
                                     and R.stated_kind('TED-HIGH', 'Notice', '3') == (None, True))
    out['can_val_total_is_award'] = R.stated_kind('TED-VAL_TOTAL', 'Notice', '3') == ('award', True)
    out['cn_val_total_is_estimate'] = R.stated_kind('TED-VAL_TOTAL', 'Notice', '2') == ('estimate', True)

    # (b) cand_flags on real-shaped inputs: results_by_lot is {lot: [(i, lot_results row)]} as facts() builds it
    def rbl(rows):
        d = {}
        for i, lr in enumerate(rows):
            d.setdefault(lr.get('lot'), []).append((i, lr))
        return d

    def cf(field, r3_kind, nid, rows, summed_keys=('LOT-1', 'LOT-2')):
        return R.cand_flags({'field': field, 'notice_id': nid, 'cents': 100, 'r3_kind': r3_kind, 'form': None},
                            R.summed_keys_of([{'key': k} for k in summed_keys]), rbl(rows), R.round_counts(rows))

    other = [{'lot': 'LOT-1', 'notice_id': 9}, {'lot': 'LOT-2', 'notice_id': 9}]
    hi_none = cf('TED-HIGH', None, 7, other + [{'lot': 'LOT-9', 'notice_id': 7}, {'lot': 'LOT-8', 'notice_id': 7}])
    out['b1_ted_high_no_summed_key_is_range_and_g_cover'] = (hi_none['kind'] == 'range' and hi_none['g_cover'] is True
                                                            and hi_none['names_summed_keys'] == [])
    lo_none = cf('TED-LOW', None, 7, other)
    out['b1_ted_low_no_row_is_range_and_g_cover'] = lo_none['kind'] == 'range' and lo_none['g_cover'] is True
    hi_one = cf('TED-HIGH', None, 7, other + [{'lot': 'LOT-2', 'notice_id': 7}, {'lot': 'LOT-9', 'notice_id': 7}])
    out['b2_ted_high_one_keyed_row_on_summed_key_kept'] = (hi_one['g_cover'] is False
                                                          and hi_one['names_summed_keys'] == ['LOT-2'])
    for blank in (None, ''):
        kl = cf('TED-HIGH', None, 7, other + [{'lot': blank, 'notice_id': 7}, {'lot': blank, 'notice_id': 7}],
                summed_keys=('LOT-1', 'LOT-2', blank))
        out[f'b3_keyless_row_{blank!r}_names_no_key'] = kl['names_summed_keys'] == [] and kl['g_cover'] is True
    for fld in ('BT-27', 'TED-VAL_ESTIMATED_TOTAL'):
        est = cf(fld, 'estimate', 7, other + [{'lot': 'LOT-9', 'notice_id': 7}, {'lot': 'LOT-9', 'notice_id': 7}])
        out[f'b4_estimate_{fld}_no_key_kept_by_cover'] = est['kind'] == 'estimate' and est['g_cover'] is False
    aw = cf('BT-161', 'award', 7, other + [{'lot': 'LOT-9', 'notice_id': 7}, {'lot': 'LOT-9', 'notice_id': 7}])
    out['b4_award_no_key_g_cover'] = aw['kind'] == 'award' and aw['g_cover'] is True and aw['g_single'] is False
    one_keyless = cf('TED-VAL_TOTAL', 'award', 7, other + [{'lot': None, 'notice_id': 7}])
    one_keyed = cf('TED-VAL_TOTAL', 'award', 7, other + [{'lot': 'LOT-1', 'notice_id': 7}])
    two = cf('TED-VAL_TOTAL', 'award', 7, other + [{'lot': None, 'notice_id': 7}, {'lot': None, 'notice_id': 7}])
    out['b5_exactly_one_row_keyless_g_single'] = one_keyless['round_results'] == 1 and one_keyless['g_single'] is True
    out['b5_exactly_one_row_keyed_g_single'] = (one_keyed['round_results'] == 1 and one_keyed['g_single'] is True
                                               and one_keyed['g_cover'] is False)
    out['b5_two_rows_not_g_single'] = two['round_results'] == 2 and two['g_single'] is False
    out['b5_no_row_not_g_single'] = cf('TED-VAL_TOTAL', 'award', 8, other)['g_single'] is False
    return all(out.values()), out


def t0_decide(R, base):
    """T0 (c): the guards disagree end to end. `base` is 6465510's real facts (v2 ships it); its candidates are
    replaced by one cand_flags() candidate at a time, with v2's vouched_by / vouches set to match."""
    import copy
    out = {}
    if base is None:
        return False, {'error': 'no base facts (6465510 not decidable offline)'}
    summed_keys = ['LOT-1', 'LOT-2']
    other = [{'lot': 'LOT-1', 'notice_id': 9}, {'lot': 'LOT-2', 'notice_id': 9}]
    cases = {
        'cover_only': ({'field': 'TED-HIGH', 'r3_kind': None},
                       other + [{'lot': None, 'notice_id': 7}, {'lot': None, 'notice_id': 7}],
                       {'v2': True, 'v2+single': True, 'v2+cover': False, 'v2+both': False}),
        'single_only': ({'field': 'BT-161', 'r3_kind': 'award'},
                        other + [{'lot': 'LOT-2', 'notice_id': 7}],
                        {'v2': True, 'v2+single': False, 'v2+cover': True, 'v2+both': False}),
    }
    for name, (tt, rows, want) in cases.items():
        d = {}
        for i, lr in enumerate(rows):
            d.setdefault(lr.get('lot'), []).append((i, lr))
        S = base['S']
        c = R.cand_flags(dict(tt, notice_id=7, cents=S, form=None), summed_keys, d, R.round_counts(rows))
        f = copy.deepcopy(base)
        f['vouch_candidates'] = [c]
        f['vouched_by'] = {'field': c['field'], 'notice_id': 7, 'cents': S}
        f['vouches'] = [(c['field'], 7, S)]
        dec = R.decide_all(f)
        got = {g: dec[g]['ship'] for g in want}
        firsts = {g: dec[g]['first'] for g in want}
        good = got == want and all(firsts[g] == 'R10' for g in want if not want[g]) \
            and all(dec[g]['S'] == S for g in want if want[g])
        out[name] = {'pass': bool(good), 'cand': c, 'ship': got, 'first': firsts}
    return all(v['pass'] for v in out.values()), out


def served_rows():
    with open(SERVED_LSA, encoding='utf-8') as fh:
        return {int(r[0]): r for r in json.load(fh)}


def parity(row, v2):
    """T4's comparison of port v2 with a served_lsa_rows row (id, source, published, derivation, cents, currency,
    eur_cents, detail JSON)."""
    detail = json.loads(row[7]) if row[7] else {}
    vb = detail.get('vouched_by')
    served_ship = row[3] == 'lot-sum-awards'
    kinds = []
    if served_ship != v2['ship']:
        kinds.append('ship/refuse')
    if served_ship:
        if row[4] != v2['S']:
            kinds.append('S')
        if row[5] != v2['currency']:
            kinds.append('currency')
        if detail.get('lots') != v2['n']:
            kinds.append('n')
        key = None if not isinstance(vb, dict) else [vb.get('field'), vb.get('notice_id'), vb.get('cents')]
        member = key is not None and key in v2['vouches']
        if not member:
            kinds.append('vouched_by')
    else:
        member = None
    return {'ok': not kinds, 'kinds': kinds, 'served': {'ship': served_ship, 'S': row[4], 'currency': row[5],
                                                       'n': detail.get('lots'), 'vouched_by': vb},
            'vouched_by_member': member, 'vouched_by_equal_first': vb == v2['vouched_by']}


def notices_since_snapshot(R, tid):
    """Diagnostic for a parity miss: version notices the fresh read names that the B5 served snapshot did not."""
    p = os.path.join(SERVED_SNAP, f'{tid}.json')
    if not os.path.exists(p):
        return None
    with open(p, encoding='utf-8') as fh:
        old = json.load(fh)
    new = R.tender_detail(tid)
    return {'new_notices': sorted(set(R.version_notice_ids(new)) - set(R.version_notice_ids(old))),
            'version_then': old.get('version'), 'version_now': new.get('version')}


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('--out', default=os.path.join(HERE, 'selftest_g.json'))
    a = ap.parse_args(argv)
    no_network()
    R = load_rule()
    with open(IDS, encoding='utf-8') as fh:
        idsj = json.load(fh)
    ids61, extra = idsj['ids61'], idsj['extra']
    assert len(ids61) == 61 and TARGET in ids61 and all(i in ids61 for i in IDENTITY)
    assert all(p in extra for p in PINS) and all(p in extra for p in PARTIAL) and SEPARATES in extra
    assert len(extra) <= 15, len(extra)
    served = served_rows()

    per, errors, facts_of = {}, {}, {}
    for tid in ids61 + extra:
        try:
            f = R.facts(tid)
            facts_of[tid] = f
            dec = R.decide_all(f)
            per[tid] = {'decisions': dec, 'vouch_candidates': f.get('vouch_candidates'), 'flags': f['flags'],
                        'S_eur': f.get('S_eur'), 'v1_why': f['v1']['why'], 'tender_read': R.tender_read_meta(tid),
                        'parity_v2': parity(served[tid], dec['v2'])}
        except Exception as e:                   # noqa: BLE001 - recorded; the test then fails
            errors[tid] = f'{type(e).__name__}: {e}'
            traceback.print_exc()

    res = {}
    ok0, d0 = t0(R)
    ok0c, d0c = t0_decide(R, facts_of.get(TARGET))
    res['T0'] = {'pass': ok0 and ok0c, 'detail': d0, 'decide_cases': d0c}

    # T1
    d = per.get(TARGET, {}).get('decisions')
    t1 = {'v2': d and tup(d['v2'])}
    if d:
        for g in GUARDED:
            t1[g] = {'decision': tup(d[g]), 'g_single_removed': d[g]['g_single_removed'],
                     'g_cover_removed': d[g]['g_cover_removed']}
        cands = per[TARGET]['vouch_candidates'] or []
        c1 = [c for c in cands if c['notice_id'] == T1_NOTICE]
        t1['candidates'] = cands
        facts_ok = (len(c1) >= 1 and all(c['kind'] == 'award' and c['round_results'] == 1
                                         and c['names_summed_keys'] == [] and c['g_single'] and c['g_cover'] for c in c1))
        k1 = [[c['field'], c['notice_id'], c['cents']] for c in c1]
        removed_ok = (bool(d['v2+single']['g_single_removed']) and bool(d['v2+cover']['g_cover_removed'])
                      and all(k in d['v2+single']['g_single_removed'] for k in k1)
                      and all(k in d['v2+cover']['g_cover_removed'] for k in k1)
                      and all(k in d['v2+both']['g_single_removed'] and k in d['v2+both']['g_cover_removed'] for k in k1))
        outcome_ok = d['v2']['ship'] and all((not d[g]['ship']) and d[g]['first'] == 'R10' for g in GUARDED)
        t1['checks'] = {'outcome': bool(outcome_ok), 'candidate_facts_20772908': bool(facts_ok),
                        'removed_lists': bool(removed_ok)}
        ok1 = outcome_ok and facts_ok and removed_ok
    else:
        ok1 = False
    res['T1'] = {'pass': bool(ok1), 'id': TARGET, 'detail': t1}

    # T2
    t2, ok2 = {}, True
    for tid in IDENTITY:
        d = per.get(tid, {}).get('decisions')
        if not d:
            ok2 = False
            t2[tid] = 'missing'
            continue
        good = d['v2']['ship'] and all(d[g]['ship'] and d[g]['S'] == d['v2']['S'] for g in GUARDED)
        ok2 &= bool(good)
        t2[tid] = {'pass': bool(good), 'S': d['v2']['S'], 'currency': d['v2']['currency'],
                   **{g: tup(d[g]) for g in ['v2'] + GUARDED}}
    res['T2'] = {'pass': ok2, 'detail': t2}

    # T3
    changed, vb_moved, missing = {}, {}, []
    for tid in ids61:
        if tid == TARGET:
            continue
        d = per.get(tid, {}).get('decisions')
        if not d:
            missing.append(tid)
            continue
        for g in GUARDED:
            if tup(d[g]) != tup(d['v2']):
                changed.setdefault(tid, {})[g] = {'v2': tup(d['v2']), 'guarded': tup(d[g]), 'g_removed': d[g]['g_removed']}
            elif d[g]['ship'] and d[g]['vouched_by'] != d['v2']['vouched_by']:
                vb_moved.setdefault(tid, {})[g] = {'v2': d['v2']['vouched_by'], 'guarded': d[g]['vouched_by']}
    res['T3'] = {'pass': not changed and not missing, 'checked': 60 - len(missing), 'changed': changed,
                 'missing': missing, 'vouched_by_moved_still_ships': vb_moved}

    # T4
    miss4 = {tid: dict(per[tid]['parity_v2'], since_snapshot=notices_since_snapshot(R, tid))
             for tid in ids61 if tid in per and not per[tid]['parity_v2']['ok']}
    not_first = sorted(tid for tid in ids61 if tid in per and per[tid]['parity_v2']['ok']
                       and not per[tid]['parity_v2']['vouched_by_equal_first'])
    unread = [tid for tid in ids61 if tid not in per]
    res['T4'] = {'pass': not miss4 and not unread, 'checked': len(ids61) - len(unread),
                 'agree': len(ids61) - len(unread) - len(miss4), 'disagreements': miss4, 'unread': unread,
                 'vouched_by_member_not_first': not_first}

    # T5
    t5, ok5 = {}, True
    for tid, (cents, cur) in PINS.items():
        e = per.get(tid)
        if not e:
            ok5 = False
            t5[tid] = 'missing'
            continue
        d = e['decisions']
        good = all(d[g]['ship'] and d[g]['S'] == cents and d[g]['currency'] == cur for g in ['v2'] + GUARDED) \
            and e['parity_v2']['ok']
        rng = None
        if tid == 8426318:
            lh = [c for c in (e['vouch_candidates'] or []) if c['field'] in ('TED-LOW', 'TED-HIGH')]
            rng = {'n': len(lh), 'all_range': bool(lh) and all(c['kind'] == 'range' for c in lh),
                   'names_summed_keys': [len(c['names_summed_keys']) for c in lh]}
            good = good and rng['all_range']
        ok5 &= bool(good)
        t5[tid] = {'pass': bool(good), 'pin': [cents, cur], 'parity_v2': e['parity_v2']['kinds'], 'range_cands': rng,
                   **{g: {'decision': tup(d[g]), 'vouched_by': d[g]['vouched_by']} for g in ['v2'] + GUARDED}}
    res['T5'] = {'pass': ok5, 'detail': t5}

    # T6 (barred): the partial sums
    t6, ok6 = {}, True
    for tid in PARTIAL:
        e = per.get(tid)
        if not e:
            ok6 = False
            t6[tid] = {'error': errors.get(tid, 'missing')}
            continue
        d = e['decisions']
        good = d['v2']['ship'] and all(d[g]['ship'] and d[g]['S'] == d['v2']['S'] for g in GUARDED) \
            and e['parity_v2']['ok']
        ok6 &= bool(good)
        t6[tid] = {'pass': bool(good), 'parity_v2': e['parity_v2']['kinds'], **{g: tup(d[g]) for g in ['v2'] + GUARDED}}
    res['T6'] = {'pass': ok6, 'detail': t6}

    # T7 (barred): a real fixture where the guards disagree
    e = per.get(SEPARATES)
    if e:
        d = e['decisions']
        want = {'v2': True, 'v2+single': False, 'v2+cover': True, 'v2+both': False}
        got = {g: d[g]['ship'] for g in want}
        ok7 = got == want and d['v2+single']['first'] == 'R10' and d['v2+both']['first'] == 'R10' \
            and d['v2+cover']['S'] == d['v2']['S']
        res['T7'] = {'pass': bool(ok7), 'id': SEPARATES, 'ship': got,
                     'first': {g: d[g]['first'] for g in want}, 'candidates': e['vouch_candidates'],
                     'parity_v2': e['parity_v2']['kinds']}
    else:
        res['T7'] = {'pass': False, 'id': SEPARATES, 'error': errors.get(SEPARATES, 'missing')}

    # X (recorded)
    xr = {}
    for tid in RESIDUAL:
        e = per.get(tid)
        if not e:
            xr[tid] = {'error': errors.get(tid, 'missing')}
            continue
        d = e['decisions']
        xr[tid] = {'role': 'residual-5816379-shape',
                   'parity_v2': e['parity_v2']['kinds'],
                   'still_ships_same_S_under_all': all(d[g]['ship'] and d[g]['S'] == d['v2']['S'] for g in ['v2'] + GUARDED),
                   **{g: tup(d[g]) for g in ['v2'] + GUARDED}}
    res['X'] = {'bar': False, 'detail': xr}

    # sizes on the 69, recorded
    sizes = {g: sorted(t for t, e in per.items() if e['decisions']['v2']['ship'] and not e['decisions'][g]['ship'])
             for g in GUARDED}
    passed = all(res[k]['pass'] for k in TESTS) and not errors
    summary = ' '.join(f"{k}={'PASS' if res[k]['pass'] else 'FAIL'}" for k in TESTS)
    t1d = res['T1']['detail']
    t1s = (f"T1 6465510 v2 {'ship' if t1d.get('v2') and t1d['v2'][0] else 'NOT ship'} -> "
           + '/'.join(f"{g[3:]}:{t1d[g]['decision'][1] if g in t1d else '?'}" for g in GUARDED)
           + f" (checks {t1d.get('checks')})")
    n2 = sum(1 for v in t2.values() if isinstance(v, dict) and v['pass'])
    n6 = sum(1 for v in t6.values() if v.get('pass'))
    summary = (f"selftest_g {'PASS' if passed else 'FAIL'}: {summary}; T0 {sum(map(bool, d0.values()))}/{len(d0)} "
               f"mechanics + {sum(1 for v in d0c.values() if isinstance(v, dict) and v.get('pass'))}/2 guard-disagreement "
               f"cases; {t1s}; T2 {n2}/4 identity vouches keep S; T3 {res['T3']['checked']}/60 other heads of the 61 "
               f"checked ({len(res['T3']['changed'])} changed, {len(vb_moved)} with a moved vouched_by); "
               f"T4 port v2 = served_lsa_rows on {res['T4']['agree']}/61 (vouched_by by membership; "
               f"{len(not_first)} member-not-first); T5 pins {sum(1 for v in t5.values() if isinstance(v, dict) and v['pass'])}/3 "
               f"keep S; T6 partial sums {n6}/4 keep S; T7 {SEPARATES} ship {res['T7'].get('ship')}; "
               f"refused-vs-v2 among the {len(per)}: single {sizes['v2+single']}, cover {sizes['v2+cover']}, "
               f"both {sizes['v2+both']}; errors {len(errors)}")
    out = {'what': 'issue 551 B6 U1 self-tests of rule_v2_g.py (offline; network disabled during the run)',
           'at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), 'pass': passed, 'summary': summary,
           'sha256': {'rule_v2_g.py': sha(RULE), 'fetch_b6.py': sha(FETCH), 'selftest_g.py': sha(__file__),
                      'source b3/rule_v2_b3.py': sha(os.path.join(ROOT, 'b3', 'rule_v2_b3.py')),
                      'served_lsa_rows.json': sha(SERVED_LSA)},
           'rates': R.rates_sha256(), 'ids': {'61': ids61, 'extra': extra}, 'tests': res,
           'refused_vs_v2': sizes, 'errors': errors,
           'tender_reads': {str(t): (e['tender_read'] or {}).get('fetched_at') for t, e in per.items()},
           'per_id': {str(t): {'decisions': {g: {k: v for k, v in dd.items()} for g, dd in e['decisions'].items()},
                               'vouch_candidates': e['vouch_candidates'], 'S_eur': e['S_eur'], 'v1_why': e['v1_why'],
                               'flags': e['flags'], 'parity_v2': e['parity_v2']} for t, e in per.items()}}
    with open(a.out + '.tmp', 'w', encoding='utf-8') as fh:
        json.dump(out, fh, indent=1, default=str)
        fh.write('\n')
    os.replace(a.out + '.tmp', a.out)
    print(summary)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
