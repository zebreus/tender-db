#!/usr/bin/env python3
# Tally a window body (<name>.out.json) into the per-window table; prints JSON.
import json, sys
d = '/home/user/tender-db/.scratch/tender-db/539-triage/post-drain-chains'
out = {}
for name in sys.argv[1:]:
    j = json.load(open(f'{d}/{name}.out.json'))
    cols = j['columns']; rows = [dict(zip(cols, r)) for r in j['rows']]
    s = {
        'rows': len(rows), 'min_tid': min(r['tid'] for r in rows), 'max_tid': max(r['tid'] for r in rows),
        'tenders': len(rows),
        'versions': sum(r['nv'] for r in rows),
        'tenders_2plus': sum(1 for r in rows if r['nv'] >= 2),
        'tenders_with_doe': sum(1 for r in rows if r['n_doe'] > 0),
        'tenders_2plus_with_doe': sum(1 for r in rows if r['nv'] >= 2 and r['n_doe'] > 0),
        'tenders_doe_and_ted': sum(1 for r in rows if r['n_doe'] > 0 and r['n_ted'] > 0),
        'pairs': sum(r['pairs'] for r in rows),
        'pairs_doe': sum(r['pairs_doe'] for r in rows),
        'pairs_non_doe': sum(r['pairs'] - r['pairs_doe'] for r in rows),
        'pairs_same_instant': sum(r['pairs_same_instant'] for r in rows),
        'pairs_same_instant_doe_tenders': sum(r['pairs_same_instant'] for r in rows if r['n_doe'] > 0),
        'pairs_cross_source': sum(r['pairs_cross_source'] for r in rows),
        'vd_doe': sum(r['vd_doe'] for r in rows), 'vt_doe': sum(r['vt_doe'] for r in rows),
        'vd_non': sum(r['vd_non'] for r in rows), 'vt_non': sum(r['vt_non'] for r in rows),
        'n_nonotice': sum(r['n_nonotice'] for r in rows),
        'seq_gaps': [r['tid'] for r in rows if r['min_seq'] != 1 or r['max_seq'] != r['nv']],
        'pairs_vs_nv_mismatch': [r['tid'] for r in rows if r['pairs'] != r['nv'] - 1],
        'head_mismatch': [(r['tid'], r['cur_seq'], r['max_seq'], r['head_pub_delta']) for r in rows
                          if r['cur_seq'] != r['max_seq'] or r['head_pub_delta'] != 0],
        'max_nv': max(r['nv'] for r in rows),
        'max_nv_tid': max(rows, key=lambda r: r['nv'])['tid'],
        'violating': [r for r in rows if r['first_viol_seq'] is not None],
        'examples_2plus_doe': [r['tid'] for r in rows if r['nv'] >= 2 and r['n_doe'] > 0][:8],
        'examples_doe_ted_same_instant': [r['tid'] for r in rows if r['n_doe'] > 0 and r['n_ted'] > 0 and r['pairs_same_instant'] > 0][:8],
    }
    out[name] = s
print(json.dumps(out, indent=1))
