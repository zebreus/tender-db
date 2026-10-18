"""Join the pre-guard labels with the combined replay (guards on) and the served lot of the bar read."""
import json, os, sys, collections
sys.dont_write_bytecode = True
RB = '/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8'
sys.path.insert(0, RB)
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import combined_port as C  # noqa
from labels_fresh568 import LABELS  # noqa
S = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/fresh568'
cache = C.Cache([os.path.join(S, 'bar_cache')])
draw = json.load(open(os.path.join(S, 'draw.json')))
rows, missing = [], []
for x in draw['draw']:
    tid = x['id']
    t = cache.tender(tid)
    c = cache.contents(t)
    pre = C.expected(t, c, True, False)
    att = C.attribute(t, c)
    served = {(r['notice_id'], r['key']): r.get('lot') for r in t.get('lot_results', [])}
    for k, r in sorted(pre.items(), key=lambda kv: (kv[0][0], C.P.res_order(kv[0][1]))):
        if not r['reason'].startswith('568 match'):
            continue
        L = LABELS.get(tid, {})
        lab = L.get('blocks', {}).get(f'{k[0]}/{k[1]}') or L.get('default')
        if lab is None:
            missing.append(f'{tid}/{k[0]}/{k[1]}')
            lab = ('UNLABELLED', '')
        a = att[k]
        rows.append({'tender_id': tid, 'g2': x['g2'], 'out': x['out'], 'notice_id': k[0], 'key': k[1],
                     'printed': r['printed'], 'tentative_lot': r['expected_lot'], 'label': lab[0], 'evidence': lab[1],
                     'port_lot': a['expected_lot'], 'port_reason': a['reason'], 'cause': a['cause'],
                     'base_lot': a['base_lot'], 'served_lot': served.get(k, 'ABSENT')})
WRONG = ('wrong_other', 'multi_lot_contract')
summ = {'blocks_labelled': len(rows), 'unlabelled': missing,
        'by_label': dict(collections.Counter(r['label'] for r in rows)),
        'kept_by_label': dict(collections.Counter(r['label'] for r in rows if r['port_lot'] is not None)),
        'refused_by_label': dict(collections.Counter(r['label'] for r in rows if r['port_lot'] is None)),
        'served_ne_port': [r for r in rows if r['served_lot'] != r['port_lot']]}
wrong_kept = [r for r in rows if r['label'] in WRONG and r['port_lot'] is not None]
unclear = [r for r in rows if r['label'] == 'unclear']
summ['tenders_with_wrong_kept'] = sorted({r['tender_id'] for r in wrong_kept})
summ['wrong_kept'] = wrong_kept
summ['wrong_caught'] = [r for r in rows if r['label'] in WRONG and r['port_lot'] is None]
summ['right_refused'] = [r for r in rows if r['label'] == 'single_right' and r['port_lot'] is None]
summ['unclear'] = unclear
summ['tenders_with_any_568_block'] = sorted({r['tender_id'] for r in rows})
summ['tenders_with_a_wrong_block_before_guards'] = sorted({r['tender_id'] for r in rows if r['label'] in WRONG})
json.dump({'summary': summ, 'blocks': rows}, open(os.path.join(S, 'labels_joined.json'), 'w'), ensure_ascii=False, indent=1)
print(json.dumps({k: v for k, v in summ.items() if k not in ('wrong_caught', 'right_refused', 'wrong_kept', 'served_ne_port')}, ensure_ascii=False))
for name in ('wrong_kept', 'wrong_caught', 'right_refused', 'served_ne_port'):
    print(name, len(summ[name]))
    for r in summ[name]:
        print('   ', r['tender_id'], r['notice_id'], r['key'], 'printed', r['printed'], r['label'], 'tentative', r['tentative_lot'],
              'port', r['port_lot'], '|', r['port_reason'], '| served', r['served_lot'], '| cause', r['cause'])
