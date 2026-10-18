"""O1 extension: join the pre-guard labels with the combined replay (guards on) and the served lot of the 30-id bar read.

Labels: the 27 kept Tenders take fresh568's labels (../fresh568/labels_fresh568.py, judged 112/112 by the verifier and
accepted by the judge); the 3 extension Tenders take ext/labels_ext.py, written from view.py (pre-guard replay lot
only) BEFORE this join was run. A 568 block of the 30 with no label is reported in `unlabelled` and fails the join.

Also the stability check of the 27: the served lot_results of each kept Tender in this bar read against fresh568's
bar read (same blocks, same served lot, same version).

Usage: python3 -I -B join_ext.py RAW_EXT_DIR   (RAW_EXT_DIR holds draw_ext.json and bar_cache/)
Writes RAW_EXT_DIR/labels_joined_ext.json and prints the summary.
"""
import collections
import importlib.util
import json
import os
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))       # readback-u8/ext
RB = os.path.dirname(HERE)                              # readback-u8
sys.path.insert(0, RB)
import combined_port as C  # noqa: E402


def load_labels(path):
    spec = importlib.util.spec_from_file_location(os.path.basename(path)[:-3], path)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m.LABELS


S = sys.argv[1]
OLD = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/fresh568'
L27 = load_labels(os.path.join(RB, 'fresh568', 'labels_fresh568.py'))
LEXT = load_labels(os.path.join(HERE, 'labels_ext.py'))
assert not set(L27) & set(LEXT), 'label sets overlap'
LABELS = {**L27, **LEXT}
cache = C.Cache([os.path.join(S, 'bar_cache')])
old_cache = C.Cache([os.path.join(OLD, 'bar_cache')])
draw = json.load(open(os.path.join(S, 'draw_ext.json')))
new_ids = [x['id'] for x in draw['draw'][27:]]
rows, missing, stability = [], [], []
for x in draw['draw']:
    tid = x['id']
    t = cache.tender(tid)
    c = cache.contents(t)
    pre = C.expected(t, c, True, False)
    att = C.attribute(t, c)
    served = {(r['notice_id'], r['key']): r.get('lot') for r in t.get('lot_results', [])}
    if tid not in new_ids:
        o = old_cache.tender(tid)
        oserved = {(r['notice_id'], r['key']): r.get('lot') for r in o.get('lot_results', [])}
        diff = sorted(f'{k[0]}/{k[1]}: {oserved.get(k, "ABSENT")} -> {served.get(k, "ABSENT")}'
                      for k in set(served) | set(oserved) if served.get(k, 'ABSENT') != oserved.get(k, 'ABSENT'))
        stability.append({'tender_id': tid, 'version_then': o.get('version'), 'version_now': t.get('version'),
                          'blocks_then': len(oserved), 'blocks_now': len(served), 'served_diffs': diff})
    for k, r in sorted(pre.items(), key=lambda kv: (kv[0][0], C.P.res_order(kv[0][1]))):
        if not r['reason'].startswith('568 match'):
            continue
        L = LABELS.get(tid, {})
        lab = L.get('blocks', {}).get(f'{k[0]}/{k[1]}') or L.get('default')
        if lab is None:
            missing.append(f'{tid}/{k[0]}/{k[1]}')
            lab = ('UNLABELLED', '')
        a = att[k]
        rows.append({'tender_id': tid, 'extension': tid in new_ids, 'g2': x['g2'], 'out': x['out'],
                     'notice_id': k[0], 'key': k[1], 'printed': r['printed'], 'tentative_lot': r['expected_lot'],
                     'label': lab[0], 'evidence': lab[1], 'port_lot': a['expected_lot'], 'port_reason': a['reason'],
                     'cause': a['cause'], 'base_lot': a['base_lot'], 'base_reason': a['base_reason'],
                     'served_lot': served.get(k, 'ABSENT')})
WRONG = ('wrong_other', 'multi_lot_contract')


def summ_of(rs):
    wrong_kept = [r for r in rs if r['label'] in WRONG and r['port_lot'] is not None]
    return {
        'blocks_labelled': len(rs),
        'by_label': dict(collections.Counter(r['label'] for r in rs)),
        'kept_by_label': dict(collections.Counter(r['label'] for r in rs if r['port_lot'] is not None)),
        'refused_by_label': dict(collections.Counter(r['label'] for r in rs if r['port_lot'] is None)),
        'served_ne_port': [r for r in rs if r['served_lot'] != r['port_lot']],
        'tenders_with_wrong_kept': sorted({r['tender_id'] for r in wrong_kept}),
        'wrong_kept': wrong_kept,
        'wrong_caught': [r for r in rs if r['label'] in WRONG and r['port_lot'] is None],
        'right_refused': [r for r in rs if r['label'] == 'single_right' and r['port_lot'] is None],
        'unclear': [r for r in rs if r['label'] == 'unclear'],
        'tenders_with_any_568_block': sorted({r['tender_id'] for r in rs}),
        'tenders_with_a_wrong_block_before_guards': sorted({r['tender_id'] for r in rs if r['label'] in WRONG}),
    }


out = {'unlabelled': missing, 'extension_ids': new_ids,
       'all30': summ_of(rows), 'extension3': summ_of([r for r in rows if r['extension']]),
       'kept27': summ_of([r for r in rows if not r['extension']]),
       'stability27': {'tenders': len(stability),
                       'tenders_with_served_diffs': [s['tender_id'] for s in stability if s['served_diffs']],
                       'tenders_with_version_change': [s['tender_id'] for s in stability
                                                       if s['version_then'] != s['version_now']],
                       'per_tender': stability},
       'blocks': rows}
json.dump(out, open(os.path.join(S, 'labels_joined_ext.json'), 'w'), ensure_ascii=False, indent=1)
for part in ('extension3', 'kept27', 'all30'):
    sm = out[part]
    print(part, json.dumps({k: (v if not isinstance(v, list) or not v or not isinstance(v[0], dict) else len(v))
                            for k, v in sm.items()}, ensure_ascii=False))
print('unlabelled', missing)
print('stability27', json.dumps({k: v for k, v in out['stability27'].items() if k != 'per_tender'}))
for name in ('wrong_kept', 'wrong_caught', 'right_refused', 'served_ne_port', 'unclear'):
    print(name, len(out['all30'][name]))
    for r in out['all30'][name]:
        print('   ', r['tender_id'], r['notice_id'], r['key'], 'printed', r['printed'], r['label'], 'tentative',
              r['tentative_lot'], 'port', r['port_lot'], '|', r['port_reason'], '| served', r['served_lot'], '| cause',
              r['cause'])
sys.exit(1 if missing else 0)
