"""Assemble the verifier's result (ext-verify/verify.json) from its own outputs."""
import json, os, collections
VD = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/ext-verify'
OUT = '/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8/ext-verify'
ld = lambda p: json.load(open(p, encoding='utf-8'))
walk = ld(f'{VD}/walk_result.json'); labels = ld(f'{VD}/labels_verify.json'); served = ld(f'{VD}/served.json')
chk = ld(f'{VD}/check3_summary.json')
rows = walk['rows']
acts = collections.Counter(r['act'] for r in rows)
held_new = [r for r in rows if r['act'] == 'held']
g2_moves = {}
for k, v in served.items():
    tid = k.split('/')[0]
    if v['base_lot'] is None and v['expected'] is not None and v['cause'] == '570':
        g2_moves.setdefault(tid, []).append(k.split('/', 1)[1] + ' -> ' + v['expected'])
res = {
    'stage': 'adversarial verify of the O1 extension (568 read-back, readback-u8/readback.md O1)',
    'changed_set': {'pages': 47, 'events_in_range': walk['changed'], 'sha256': '8062ccd12fbeeea78a794b5ea9ce85126588066c882b9d0782261f3f73e6bd79',
                    'source': 're-paged fresh by the verifier, /v1/changes?entity=tender&since=957854566, to last_cursor 958764334, more=false'},
    'walk': {'excluded_hit': walk['excluded_hit'], 'candidates': walk['candidates'],
             'prefix_0_770_equals_fresh568_walk': walk['prefix_771_equals_fresh568_walk'],
             'seed': walk['seed'], 'walked_from': 771, 'walked_to': walk['walked_to'], 'acts': dict(acts),
             'gets': walk['gets'], 'stop': walk['stop'],
             'held_new': [{'index': r['index'], 'id': r['id'], 'year': r['year'], 'qualifying_notice': r['ev'].get('qualifying'),
                           'qualifying_placed': r['ev'].get('qualifying_placed')} for r in held_new],
             'held_30': walk['held'], 'n_out': walk['n_out'], 'n_g2': walk['n_g2']},
    'parity_3_fresh': {k: chk[k] for k in ('read_started', 'read_finished', 'tenders', 'read', 'blocks', 'parity_ok', 'parity_breaks',
                                           'computed_not_served', 'served_outside_chain', 'non_legacy_mismatches',
                                           'legacy_test_disagrees', 'incomplete', 'by_cause', 'by_reason', 'gets', 'fetch_errors',
                                           'parity_verdict')},
    'g2_570_moves': {t: {'n': len(v), 'blocks': v} for t, v in g2_moves.items()},
    'labels': labels,
    'label_counts': dict(collections.Counter(l['label'] for l in labels)),
}
os.makedirs(OUT, exist_ok=True)
json.dump(res, open(f'{OUT}/verify.json', 'w'), indent=1, ensure_ascii=False)
print(json.dumps({k: v for k, v in res.items() if k not in ('labels',)}, ensure_ascii=False)[:4000])
