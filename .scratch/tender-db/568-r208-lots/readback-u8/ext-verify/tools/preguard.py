"""Pre-guard replay lot (570 remap on, guards off) per block of the 3 Tenders from the verifier's fresh cache, compared
with the verifier's own right-lot judgement made from the notices (nview). No served lot, no guard shown."""
import json, os, sys
sys.dont_write_bytecode = True
sys.path.insert(0, '/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8')
import combined_port as C
CACHE = sys.argv[1]
ld = lambda p: json.load(open(p, encoding='utf-8'))
# verifier's judgement: printed lot number -> the Tender key of the lot the block awards
RIGHT = {5048894: {'1': 'LOT-2', '2': 'LOT-1', '3': 'LOT-3'}}
out = {}
for tid in (5048894, 4836560, 5226033):
    t = ld(os.path.join(CACHE, f'tenders_{tid}.json'))
    chain = C.chain_of(t)
    contents = {n: ld(os.path.join(CACHE, f'notices_{n}_content.json')) for n in chain}
    pre = C.expected(t, contents, True, False)
    for k, r in sorted(pre.items()):
        printed = r.get('printed')
        right = RIGHT.get(tid, {}).get(str(printed), f'LOT-{printed}')
        out[f'{tid}/{k[0]}/{k[1]}'] = {'printed': printed, 'preguard_lot': r['expected_lot'], 'verifier_right_lot': right,
                                       'agree': r['expected_lot'] == right}
    print(tid, 'chain', chain, 'blocks', len(pre), 'disagree', [x for x, v in out.items() if x.startswith(str(tid)) and not v['agree']])
json.dump(out, open(sys.argv[2], 'w'), indent=1)
