"""Stage "sets" of the 568-u8 + 570 read-back: named items over the live re-reads in rb8/sets/live.

Offline only (no GET): reads the live tender files check_live.py wrote, the check_live summaries, expected.json,
the pre-drain baseline, measurement-u8.json, combined_run.json and port_result.json.

Usage: python3 -I -B analyze.py OUT.json
"""
import collections
import json
import os
import sys
import unicodedata

sys.dont_write_bytecode = True
S = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8'
H = '/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8'
R = '/home/user/tender-db/.scratch/tender-db/568-r208-lots'
K570 = '/home/user/tender-db/.scratch/tender-db/570-lot-keys'
sys.path.insert(0, H)
import combined_port as C  # noqa: E402

LIVE = os.path.join(S, 'sets', 'live')
OUT = os.path.join(S, 'sets', 'out')


def ld(p):
    with open(p, encoding='utf-8') as fh:
        return json.load(fh)


ids = ld(os.path.join(S, 'sets', 'idsets.json'))
expected = {int(t): {(r['notice_id'], r['key']): r for r in rows} for t, rows in ld(os.path.join(H, 'expected.json')).items()}
baseline = {int(t): v for t, v in ld(os.path.join(R, 'drain', 'baseline-pre568u8-570.json'))['tenders'].items()}
pr = ld(os.path.join(K570, 'port_result.json'))
m8 = ld(os.path.join(R, 'u7', 'measurement-u8.json'))
crun = ld(os.path.join(H, 'combined_run.json'))

# reuse dirs, in check_live's order per set, to rebuild per-block rows (printed, reason) offline
reuse_u7 = [os.path.join(S, 'sets', 'u7notices'), os.path.join(S, 'fresh')]
reuse_oos = [os.path.join(R, 'u8-oos', 'cache_o1'), os.path.join(R, 'u8-oos', 'cache_o2'),
             os.path.join(S, 'oos', 'cache_o1'), os.path.join(S, 'oos', 'cache_o2')]
reuse_570 = sorted(os.path.join(S, 'c570', d, 'live') for d in os.listdir(os.path.join(S, 'c570')) if d.endswith('_skeptic')) \
    + [os.path.join(S, 'c570', f'cache_k{i}') for i in range(1, 13)] + [os.path.join(S, 'c567')]
live_cache = C.Cache([LIVE])
caches = {'u7': C.Cache(reuse_u7), 'oos': C.Cache(reuse_oos + reuse_u7), '570': C.Cache(reuse_570 + reuse_u7)}
SET_CACHE = {'x': 'u7', 's150': 'u7', 'f30': 'u7', 'm77': 'u7', 'oos': 'oos', 'p570': '570'}


def live(tid):
    p = os.path.join(LIVE, f'tenders_{tid}.json')
    return ld(p) if os.path.exists(p) else None


def contents_for(t, which):
    out = {}
    for n in C.chain_of(t):
        if n in live_cache.notices:
            out[n] = ld(live_cache.notices[n])
        elif n in caches[which].notices:
            out[n] = ld(caches[which].notices[n])
    return out


def proj(t):
    return {'lots': [{'key': l.get('lot_key'), 'title': l.get('title'), 'value': l.get('value')} for l in t.get('lot_details') or []],
            'lot_results': [{'key': r.get('key'), 'lot': r.get('lot'), 'notice_id': r.get('notice_id'), 'awarded': r.get('awarded')}
                            for r in t.get('lot_results') or []]}


def served(t):
    return {(r['notice_id'], r['key']): r.get('lot') for r in t.get('lot_results') or []}


def strip(s):
    return ''.join(ch for ch in unicodedata.normalize('NFKD', s or '') if not unicodedata.combining(ch)).lower()


open_blocks = []
items = []


def ob(tid, key, label, why, notice=None, exp='<na>', srv='<na>'):
    d = {'tender_id': tid, 'result_key': key, 'label': label, 'why': why}
    if notice is not None:
        d['notice_id'] = notice
    if exp != '<na>':
        d['expected_lot'] = exp
    if srv != '<na>':
        d['served_lot'] = srv
    open_blocks.append(d)


# ---------------------------------------------------------------- per set: live vs replay on the live chain, vs expected.json
rows_by = {}
set_stats = {}
for name, tids in ids.items():
    st = collections.Counter()
    for tid in tids:
        t = live(tid)
        if t is None:
            st['tender_not_read'] += 1
            ob(tid, '', f'set:{name}', 'tender not read live')
            continue
        st['tenders_read'] += 1
        if tid not in rows_by:
            contents = contents_for(t, SET_CACHE[name])
            missing = [n for n in C.chain_of(t) if n not in contents]
            rows_by[tid] = (C.attribute(t, contents), missing)
        rows, missing = rows_by[tid]
        if missing:
            st['tenders_missing_chain_notices'] += 1
            ob(tid, '', f'set:{name}', f'chain notices not available offline: {missing}')
        sv = served(t)
        ex = expected.get(tid, {})
        if len(sv) != len(t.get('lot_results') or []):
            ob(tid, '', f'set:{name}', 'duplicate (notice_id, key) in lot_results')
        for k, lot in sv.items():
            st['blocks_served'] += 1
            row = rows.get(k)
            if row is None:
                st['served_outside_replay'] += 1
                ob(tid, k[1], f'set:{name}', 'served block not produced by the replay on the live chain', k[0], None, lot)
            elif lot == row['expected_lot']:
                st['equal_replay_on_live_chain'] += 1
            else:
                st['differ_replay_on_live_chain'] += 1
                ob(tid, k[1], f'set:{name}', f"served != replay on the live chain ({row['reason']}; cause {row['cause']})",
                   k[0], row['expected_lot'], lot)
            e = ex.get(k)
            if e is None:
                st['served_not_in_expected_json'] += 1
                if row is not None:
                    ob(tid, k[1], f'set:{name}', 'served block absent from expected.json', k[0], None, lot)
            elif e['expected_lot'] == lot:
                st['equal_expected_json'] += 1
            else:
                st['differ_expected_json'] += 1
                if row is not None and row['expected_lot'] == lot:
                    ob(tid, k[1], f'set:{name}', f"served equals the live-chain replay but not expected.json ({e['reason']})",
                       k[0], e['expected_lot'], lot)
        for k in ex:
            if k not in sv:
                st['expected_json_not_served'] += 1
                ob(tid, k[1], f'set:{name}', 'expected.json block not served', k[0], ex[k]['expected_lot'], None)
        for k in rows:
            if k not in sv:
                st['replay_not_served'] += 1
        st['guard_refused_expected'] += sum(1 for k in sv if rows.get(k, {}).get('reason', '').startswith('568 refused'))
    cl = os.path.join(OUT, f'{name}.json')
    summ = ld(cl) if os.path.exists(cl) else None
    set_stats[name] = {'stats': dict(st), 'check_live': None if summ is None else {
        k: summ[k] for k in ('read_started', 'read_finished', 'tenders', 'read', 'blocks', 'parity_ok', 'parity_breaks',
                             'non_legacy_ok', 'non_legacy_mismatches', 'served_outside_chain', 'computed_not_served',
                             'legacy_test_disagrees', 'profile_from_prefix', 'incomplete', 'fetch_errors', 'gets',
                             'by_cause', 'by_reason', 'parity_verdict', 'verdict_reasons', 'breaks')}}
    if summ is not None:
        for b in summ['breaks']:
            if not any(o['tender_id'] == b['tender_id'] and o.get('notice_id') == b['notice_id'] and o['result_key'] == b['key']
                       for o in open_blocks):
                ob(b['tender_id'], b['key'], f'set:{name}', f"check_live parity break ({b['reason']})", b['notice_id'],
                   b['expected'], b['served'])


def item(name, ok, evidence, adjud=False):
    items.append({'item': name, 'result': 'pass' if ok else ('adjudicate' if adjud else 'fail'), 'evidence': evidence})


def cl_ok(name):
    c = set_stats[name]['check_live']
    return c is not None and c['parity_verdict'] == 'pass'


def cl_line(name):
    c = set_stats[name]['check_live']
    if c is None:
        return f'{name}: check_live not run'
    return (f"{name}: check_live read {c['read']}/{c['tenders']}, {c['blocks']} blocks, parity_ok {c['parity_ok']}, "
            f"breaks {c['parity_breaks']}, verdict {c['parity_verdict']} {c['verdict_reasons']}, gets {c['gets']}, "
            f"read {c['read_started']}..{c['read_finished']}")


# ---------------------------------------------------------------- 568 (b) 8327340
t = live(8327340)
sv = served(t) if t else {}
want = {(17821481, f'RES-{i}'): None for i in (1, 2, 5, 6, 7)}
want.update({(17830774, 'RES-1'): None, (17830774, 'RES-2'): None,
             (17821481, 'RES-3'): 'LOT-24', (17821481, 'RES-4'): 'LOT-32', (17840150, 'RES-1'): 'LOT-2'})
bad = []
for k, v in want.items():
    got = sv.get(k, '<absent>')
    if got != v:
        bad.append(f'{k[0]}/{k[1]} served {got} want {v}')
        ob(8327340, k[1], '568(b)', 'named block not as required', k[0], v, None if got == '<absent>' else got)
extra = sorted(set(sv) - set(want))
item('568 (b) 8327340: 7 multi-lot blocks null; 17821481 RES-3 LOT-24, RES-4 LOT-32; 17840150 RES-1 LOT-2',
     t is not None and not bad and not extra,
     f"live v{t and t.get('version')}: served {json.dumps({f'{k[0]}/{k[1]}': v for k, v in sorted(sv.items())})}; "
     f"mismatches {bad}; extra blocks {extra}; {cl_line('x')}")

# ---------------------------------------------------------------- 568 (c) the s150
refused = [(b['tender_id'], b['can_notice_id'], b['result_key']) for b in m8['blocks'] if b['set'] == 's150' and b['guards8']]
named = collections.Counter(t_ for t_, _, _ in refused)
not_null = []
for tid, n, k in refused:
    t = live(tid)
    got = served(t).get((n, k), '<absent>') if t else '<tender not read>'
    if got is not None:
        not_null.append(f'{tid}/{n}/{k}: {got}')
        ob(tid, k, '568(c) refused', 'refused block not served null', n, None, None if got == '<absent>' else got)
keep = {(5051007, 17689000, 'RES-1'): 'LOT-1', (5051007, 17689000, 'RES-2'): 'LOT-2', (5051007, 17689000, 'RES-3'): 'LOT-3',
        (5267810, 18199880, 'RES-1'): 'LOT-1', (8313642, 19187804, 'RES-14'): 'LOT-15', (8313642, 19187804, 'RES-15'): 'LOT-18'}
keep_bad = []
for (tid, n, k), v in keep.items():
    t = live(tid)
    got = served(t).get((n, k), '<absent>') if t else '<tender not read>'
    if got != v:
        keep_bad.append(f'{tid}/{n}/{k}: {got} want {v}')
        ob(tid, k, '568(c) named kept', 'named block not on its lot', n, v, got)
s150_other = set_stats['s150']['stats']
item('568 (c) s150: 57 refused blocks null; 5051007 RES-1..3 / 5267810 RES-1 kept; 8313642 RES-14 LOT-15, RES-15 LOT-18; '
     'every other block equals expected',
     len(refused) == 57 and not not_null and not keep_bad and cl_ok('s150')
     and s150_other.get('differ_expected_json', 0) == 0 and s150_other.get('expected_json_not_served', 0) == 0,
     f"refused list (measurement-u8 s150 guards8): {len(refused)} = {dict(named)}; not null: {not_null}; kept/movers "
     f"mismatches: {keep_bad}; s150 live vs expected.json: {s150_other}; {cl_line('s150')}")

# ---------------------------------------------------------------- 568 (d) the m77
st = set_stats['m77']['stats']
m77_ok = (cl_ok('m77') and st.get('blocks_served') == 692 and st.get('equal_expected_json') == 692
          and st.get('guard_refused_expected', 0) == 0 and set_stats['m77']['check_live']['blocks'] == 692
          and set_stats['m77']['check_live']['parity_ok'] == 692)
item('568 (d) m77: 692 blocks equal expected, no guard refusal', m77_ok, f"m77 live vs expected.json: {st}; {cl_line('m77')}")

# ---------------------------------------------------------------- OOS 50
oos_ref = [(r['tender_id'], b['notice_id'], b['key']) for r in crun['changed_blocks']['oos'] for b in r['blocks']
           if b['reason'].startswith('568 refused')]
oos_nn = []
for tid, n, k in oos_ref:
    t = live(tid)
    got = served(t).get((n, k), '<absent>') if t else '<tender not read>'
    if got is not None:
        oos_nn.append(f'{tid}/{n}/{k}: {got}')
        ob(tid, k, 'OOS refused', 'refused block not served null', n, None, got)
st = set_stats['oos']['stats']
item('OOS 50: 100 refused blocks null, all others equal expected',
     len(oos_ref) == 100 and not oos_nn and cl_ok('oos') and st.get('differ_expected_json', 0) == 0
     and st.get('expected_json_not_served', 0) == 0,
     f"refused (combined_run oos, '568 refused'): {len(oos_ref)} = {dict(collections.Counter(x[0] for x in oos_ref))}; "
     f"not null: {oos_nn}; oos live vs expected.json: {st}; {cl_line('oos')}")

# ---------------------------------------------------------------- 570
rows570 = {r['tender_id']: r for r in pr['sample'] + pr['exhibits']}
changed_sample = [r['tender_id'] for r in pr['sample'] if r['B_moved_sections'] or r['results_moved_by_B']]
changed_exh = [r['tender_id'] for r in pr['exhibits'] if r['B_moved_sections'] or r['results_moved_by_B']]
equal_base_named = [8433088, 6016286, 7465548, 7112657, 5939039, 5317587, 5540787, 6120045, 7025760]


def base_cmp(tid, allow_moves):
    """lots and lot_results of the live Tender against the baseline; allow_moves: blocks expected.json moves may
    differ in `lot` only, and must then serve the expected lot."""
    t = live(tid)
    if t is None:
        return ['tender not read'], 0
    b = baseline.get(tid)
    if b is None:
        return ['no baseline'], 0
    p = proj(t)
    probs, moved = [], 0
    if sorted(p['lots'], key=lambda x: x['key'] or '') != sorted(b['lots'], key=lambda x: x['key'] or ''):
        lk, bk = {x['key']: x for x in p['lots']}, {x['key']: x for x in b['lots']}
        for k in sorted(set(lk) | set(bk)):
            if lk.get(k) != bk.get(k):
                probs.append(f'lot {k}: live {lk.get(k)} vs baseline {bk.get(k)}')
    lr = {(x['notice_id'], x['key']): x for x in p['lot_results']}
    br = {(x['notice_id'], x['key']): x for x in b['lot_results']}
    ex = expected.get(tid, {})
    for k in sorted(set(lr) | set(br), key=str):
        if k not in lr or k not in br:
            probs.append(f'block {k}: live {lr.get(k)} vs baseline {br.get(k)}')
            continue
        if lr[k] == br[k]:
            continue
        e = ex.get(k)
        diff_fields = [f for f in lr[k] if lr[k][f] != br[k].get(f)]
        if allow_moves and diff_fields == ['lot'] and e is not None and e['expected_lot'] != br[k]['lot'] \
                and lr[k]['lot'] == e['expected_lot']:
            moved += 1
            continue
        probs.append(f"block {k[0]}/{k[1]}: live {lr[k]} vs baseline {br[k]} (expected.json {e and e['expected_lot']})")
    # an expected move the live Tender does not make
    if allow_moves:
        for k, e in ex.items():
            if k in br and e['expected_lot'] != br[k]['lot'] and lr.get(k, {}).get('lot') != e['expected_lot']:
                probs.append(f"block {k[0]}/{k[1]}: expected.json moves {br[k]['lot']} -> {e['expected_lot']}, live "
                             f"{lr.get(k, {}).get('lot')}")
    return probs, moved


# (b) 8375276
t = live(8375276)
rows, _ = rows_by.get(8375276, ({}, []))
lots = {l['lot_key']: l.get('title') for l in (t or {}).get('lot_details') or []}
b_probs = []
if not strip(lots.get('LOT-1')).startswith('lahve'):
    b_probs.append(f"LOT-1 title {lots.get('LOT-1')!r}")
if strip(lots.get('LOT-2')).strip() != 'kojenecka voda':
    b_probs.append(f"LOT-2 title {lots.get('LOT-2')!r}")
sv = served(t) if t else {}
per_printed = collections.Counter()
for k, lot in sorted(sv.items(), key=str):
    row = rows.get(k)
    pnum = row and row.get('printed')
    digits = ''.join(ch for ch in (pnum or '') if ch.isdigit())
    want_lot = f'LOT-{int(digits)}' if digits else None
    per_printed[(pnum, lot)] += 1
    if want_lot is None:
        # printed no number: outside the claim; must still equal the replay
        if row is None or lot != row['expected_lot']:
            b_probs.append(f'{k[0]}/{k[1]} printed nothing, served {lot}, replay {row and row["expected_lot"]}')
            ob(8375276, k[1], '570(b)', 'unnumbered block not on the replay lot', k[0], row and row['expected_lot'], lot)
        continue
    if lot != want_lot:
        b_probs.append(f'{k[0]}/{k[1]} printed {pnum!r} served {lot}')
        ob(8375276, k[1], '570(b)', f'block printed {pnum!r} not on LOT-{digits}', k[0], want_lot, lot)
bl = {(x['notice_id'], x['key']): x['lot'] for x in baseline[8375276]['lot_results']}
mv = collections.Counter((bl[k], v) for k, v in sv.items() if k in bl and bl[k] != v)
moved_ids = {(m['notice'], m['block']) for m in rows570[8375276]['results_moved_by_B']}
moved_live = {k for k, v in sv.items() if k in bl and bl[k] != v}
if moved_live != moved_ids:
    b_probs.append(f'moved vs baseline {len(moved_live)} != port moved {len(moved_ids)}: '
                   f'{sorted(moved_live ^ moved_ids, key=str)[:10]}')
item('570 (b) 8375276: LOT-1 "Lahve...", LOT-2 "Kojenecka voda"; printed 1 on LOT-1, 2 on LOT-2; 147 moved (120 + 27)',
     t is not None and not b_probs and sum(mv.values()) == 147 and sorted(mv.values()) == [27, 120],
     f"live v{t and t.get('version')} lots {json.dumps(lots, ensure_ascii=False)}; {len(sv)} blocks by (printed, served): "
     f"{ {f'{a}->{b}': c for (a, b), c in sorted(per_printed.items(), key=str)} }; moved vs baseline: "
     f"{ {f'{a}->{b}': c for (a, b), c in mv.items()} } = {sum(mv.values())}; moved set == port_result results_moved_by_B: "
     f"{moved_live == moved_ids}; problems {b_probs}")

# (c) 13 changed Tenders + 9 equal-baseline
c_probs, n_moved, n_ok = [], 0, 0
for tid in changed_sample + changed_exh:
    t = live(tid)
    if t is None:
        c_probs.append(f'{tid}: not read')
        continue
    sv = served(t)
    for m in rows570[tid]['results_moved_by_B']:
        n_moved += 1
        got = sv.get((m['notice'], m['block']), '<absent>')
        if got == m['B']:
            n_ok += 1
        else:
            c_probs.append(f"{tid}/{m['notice']}/{m['block']}: served {got} B {m['B']}")
            ob(tid, m['block'], '570(c) moved', "block not on port_result.json's B lot", m['notice'], m['B'], got)
    lk = sorted(l['lot_key'] for l in t.get('lot_details') or [])
    bk = sorted(l['key'] for l in baseline[tid]['lots'])
    if lk != bk:
        c_probs.append(f'{tid}: lot keys {lk} vs baseline {bk}')
        ob(tid, '', '570(c) lot keys', f'lot key set changed: live {lk} vs baseline {bk}')
eq_probs = {}
for tid in equal_base_named:
    p, _ = base_cmp(tid, False)
    if p:
        eq_probs[tid] = p
        ob(tid, '', '570(c) equal-baseline', '; '.join(p[:5]))
item('570 (c) 8 changed sample + 5 changed exhibits on B lots, lot key sets unchanged; 9 named Tenders equal the baseline',
     not c_probs and not eq_probs and n_moved == 239 and n_ok == 239,
     f"changed sample {changed_sample}, changed exhibits {changed_exh}; results_moved_by_B on B: {n_ok}/{n_moved}; "
     f"lot key sets equal baseline for all 13: {not any('lot keys' in x for x in c_probs)}; problems {c_probs}; "
     f"9 equal-baseline (lots + lot_results): {len(equal_base_named) - len(eq_probs)}/9, differ {eq_probs}; {cl_line('p570')}")

# (d) the other 196 sample Tenders
others = [r['tender_id'] for r in pr['sample'] if r['tender_id'] not in changed_sample]
d_probs, d_moved, d_moved_list = {}, 0, []
for tid in others:
    p, mvd = base_cmp(tid, True)
    d_moved += mvd
    if mvd:
        d_moved_list.append(f'{tid}: {mvd}')
    if p:
        d_probs[tid] = p
        ob(tid, '', '570(d) equal-baseline', '; '.join(p[:5]))
item('570 (d) the other 196 sample Tenders equal the baseline (lots, lot_results) except blocks expected.json moves',
     len(others) == 196 and not d_probs,
     f"{len(others)} Tenders; equal (allowing expected moves): {len(others) - len(d_probs)}; blocks moved as expected.json "
     f"says: {d_moved} ({d_moved_list}); differ: {d_probs}")

# ---------------------------------------------------------------- all baseline Tenders: what moved vs the baseline
moves = collections.Counter()
unexpected_moves, lot_changes = [], []
for tid, b in baseline.items():
    t = live(tid)
    if t is None:
        continue
    sv = served(t)
    br = {(x['notice_id'], x['key']): x['lot'] for x in b['lot_results']}
    ex = expected.get(tid, {})
    for k, lot in sv.items():
        if k in br and br[k] != lot:
            e = ex.get(k)
            if e is not None and e['expected_lot'] == lot:
                moves['expected'] += 1
            else:
                moves['unexpected'] += 1
                unexpected_moves.append(f'{tid}/{k[0]}/{k[1]}: {br[k]} -> {lot} (expected {e and e["expected_lot"]})')
    p = proj(t)
    if sorted(p['lots'], key=lambda x: x['key'] or '') != sorted(b['lots'], key=lambda x: x['key'] or ''):
        lot_changes.append(tid)
global_info = {'baseline_tenders_read': sum(1 for tid in baseline if live(tid) is not None), 'moves_vs_baseline': dict(moves),
               'unexpected_moves': unexpected_moves, 'tenders_whose_lots_differ_from_baseline': lot_changes}

verdict = 'pass' if all(i['result'] == 'pass' for i in items) and not open_blocks else (
    'fail' if any(i['result'] == 'fail' for i in items) else 'adjudicate')
out = {'stage': 'sets', 'verdict': verdict, 'items': items, 'open_blocks': open_blocks, 'set_stats': set_stats,
       'global_vs_baseline': global_info,
       'inputs': {'live_cache': LIVE, 'expected': os.path.join(H, 'expected.json'),
                  'baseline': os.path.join(R, 'drain', 'baseline-pre568u8-570.json')}}
with open(sys.argv[1], 'w', encoding='utf-8') as fh:
    json.dump(out, fh, ensure_ascii=False, indent=1, default=str)
print(json.dumps({'verdict': verdict, 'items': [(i['item'][:60], i['result']) for i in items], 'open_blocks': len(open_blocks),
                  'global': {k: (v if not isinstance(v, list) else len(v)) for k, v in global_info.items()}}, ensure_ascii=False, indent=1))
