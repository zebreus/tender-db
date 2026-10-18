"""AFTER labelling: served lot vs combined replay per block (fresh cache), with guard reason, cause and base."""
import json, os, sys
sys.dont_write_bytecode = True
sys.path.insert(0, '/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8')
import combined_port as C
CACHE = sys.argv[1]
ld = lambda p: json.load(open(p, encoding='utf-8'))
out = {}
for tid in (5048894, 4836560, 5226033):
    t = ld(os.path.join(CACHE, f'tenders_{tid}.json'))
    chain = C.chain_of(t)
    contents = {n: ld(os.path.join(CACHE, f'notices_{n}_content.json')) for n in chain}
    rows = C.attribute(t, contents)
    for r in t['lot_results']:
        k = (r['notice_id'], r['key'])
        x = rows[k]
        out[f"{tid}/{k[0]}/{k[1]}"] = {'served': r.get('lot'), 'expected': x['expected_lot'], 'reason': x['reason'],
                                        'cause': x['cause'], 'base_lot': x['base_lot'], 'base_reason': x['base_reason'],
                                        'remap_only': x['remap_only_lot'], 'guards_only': x['guards_only_lot'],
                                        'winners': [w.get('organization_name') for w in r.get('winners') or []],
                                        'decision': r.get('decision')}
json.dump(out, open(sys.argv[2], 'w'), indent=1, ensure_ascii=False)
import collections
print(collections.Counter((v['served'] == v['expected'], v['cause'], v['base_lot'] is None, v['base_reason'][:14]) for v in out.values()))
for k, v in out.items():
    if v['served'] is None or v['cause'] != '570' or 'G' in v['reason'][:20]:
        print(k, v)
