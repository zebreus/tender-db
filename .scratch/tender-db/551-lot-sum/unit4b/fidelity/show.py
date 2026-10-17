import json, sys
import os
D = json.load(open(os.environ.get('M551', '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/m551') + '/fid/derived.json'))
for t in sys.argv[1:]:
    F = D[t]
    print('=====', t, 'date', F['date'], 'cur', F['currency'], 'rate', F['rate'], 'S', F['S'], 'n', F['n'], 'S_eur', F['S_eur_cents'], 'served', F['served'], F['served_eur_cents'])
    print('T\'', F['T_prime_ok'], F['procedure_figures'], '| K', F['K_ok'], '| G0', F['G0_ok'], '| G1\'', F['G1p_repeated'], '| other cur awards', F['other_currency_awards'])
    print('forms', F['notice_forms'])
    for s in F['summed']:
        print('  ', s['key'], 'n', s['notice'], s['result_key'], s['cents'], 'blk', s['block_amounts'], '| pn sec', s['pn_section'], 'blk', s['pn_block'], 'frozen', s['pn_frozen'], 'item', s['item'],
              '|', s['win_src'], 'tenders', s['n_tenders'], 'group', s['group'], 'notice winners', s['winners_notice'], 'api', [w[0] for w in s['api_winners']], 'R1', s['R1'], s['decision'])
    print('stated totals:')
    for r in F['stated_totals']:
        print('    ', r['nid'], r['field'], r['section'], r['cur'], r['cents'], r['kind3'], 'form', r['form'], 'single-own' if r['single_lot_own'] else '')
    print('R3 award totals', F['R3_award_totals'], 'R3', F['R3'], 'estimates', F['estimates'])
    print('vouches', F['vouches'], 'R10', F['R10'], 'R2', F['R2a'], F['R2b'], F['R2c'], F['R2d'], 'R6', F['R6'], 'R7', F['R7'], 'R8', F['R8'])
    print('all award blocks (nid, key, LOT_NO, BT-13713, ITEM, amounts, n contractors/tenders):')
    for b in F['all_award_blocks']:
        print('    ', b)
