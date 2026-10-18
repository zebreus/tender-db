"""O1 extension: assemble ext/result.json from the raw stage files (no GET). Also copies the draw, the bar summary and
log, the joined labels and the offline G2-test check into ext/.

Usage: python3 -I -B build_result.py RAW_EXT_DIR
"""
import collections
import json
import os
import shutil
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
S = sys.argv[1]


def ld(p):
    return json.load(open(p, encoding='utf-8'))


for src, dst in (('draw_ext.json', 'draw_ext.json'), ('bar_summary.json', 'bar_summary.json'), ('bar.log', 'bar_log.txt'),
                 ('labels_joined_ext.json', 'labels_joined_ext.json'), ('offline_g2_check.json', 'g2_test_on_fresh568_30.json'),
                 ('draw_ext.log', 'draw_log.txt'), ('join.log', 'join_log.txt')):
    shutil.copyfile(os.path.join(S, src), os.path.join(HERE, dst))

draw = ld(os.path.join(S, 'draw_ext.json'))
bar = ld(os.path.join(S, 'bar_summary.json'))
lab = ld(os.path.join(S, 'labels_joined_ext.json'))
g2chk = ld(os.path.join(S, 'offline_g2_check.json'))
w = draw['walked']
read = [r for r in w if 'g2_evidence' in r]
new = [r for r in w if r.get('held')]
ext_ids = [r['id'] for r in new]
blocks = lab['blocks']
WRONG = ('wrong_other', 'multi_lot_contract')

per_new = {}
for r in new:
    tid = r['id']
    bs = [b for b in blocks if b['tender_id'] == tid]
    g = bar['g2_stratum']['per_tender'][str(tid)]
    per_new[tid] = {
        'walk_index': r['index'], 'published_year': r['year'], 'out': r['out'],
        'g2_test': {k: r['g2_evidence'][k] for k in ('qualifying_notice', 'why', 'lot_placed_by_profile',
                                                      'r208_lot_placed_notices') if k in r['g2_evidence']},
        'old_signal_hits': r['old_signal_hits'],
        'blocks_568_scope': len(bs),
        'labels': dict(collections.Counter(b['label'] for b in bs)),
        'kept': sum(1 for b in bs if b['port_lot'] is not None),
        'refused': [f"{b['notice_id']}/{b['key']} ({b['port_reason']}; label {b['label']})" for b in bs if b['port_lot'] is None],
        'served_eq_port': all(b['served_lot'] == b['port_lot'] for b in bs),
        'wrong_kept': [f"{b['notice_id']}/{b['key']}" for b in bs if b['label'] in WRONG and b['port_lot'] is not None],
        'g2_report_570': {'exercises_570': g['exercises_570'], 'null_to_lot_by_570': len(g['null_to_lot_by_570']),
                          'of_which_base_was_g2_refusal': len(g['of_which_base_was_g2_refusal'])},
        'parity_blocks_served': None,
    }
log = open(os.path.join(S, 'bar.log'), encoding='utf-8').read().splitlines()
for line in log:
    if line.startswith('TENDER '):
        tid = int(line.split()[1].rstrip(':'))
        if tid in per_new:
            per_new[tid]['parity_blocks_served'] = line

kept_wrong_tenders = lab['all30']['tenders_with_wrong_kept']
result = {
    'stage': 'ext (568 O1: the 3-Tender G2 extension of the fresh 30)',
    'date': '2026-10-18',
    'disposition': 'readback-u8/readback.md "Open items and dispositions" O1 (and O7: the G2 test replaced)',
    'drain': {'refold': 2616, 'project': 2617, 'finished': '2026-10-18T15:19:48Z'},
    'draw': {
        'tool': 'ext/draw_fresh30_g2fix.py (a copy of ../draw_fresh30.py; the original is unedited): corrected G2 test '
                '+ resume',
        'changed_file': draw['changed_file'], 'changed_sha256_of_pages': '8062ccd12fbeeea78a794b5ea9ce85126588066c882b9d0782261f3f73e6bd79',
        'cursor_range': draw['cursor_range'], 'excluded': draw['excluded'], 'candidates': draw['candidates'],
        'resumed': draw['resumed'],
        'corrected_g2_test': 'outside readback/population.json AND a lot_result with a non-null lot from a notice whose '
                             '/v1/notices?tender={id} profile is ted-export-r208 and whose /v1/notices/{id}/content holds '
                             'no section of kind Lot / LotsGroup / Part',
        'corrected_test_on_the_original_30_offline': {
            'source': 'fresh568 bar_cache (no GET); ext/g2_test_on_fresh568_30.json',
            'true_g2': [x['id'] for x in g2chk if x['g2_corrected']],
            'false_positives_of_the_old_tool': [x['id'] for x in g2chk if x['g2_original_tool'] and not x['g2_corrected']],
            'agrees_with_judge': sorted(x['id'] for x in g2chk if x['g2_corrected']) == sorted([4888356, 5864590, 5362479, 4569314, 5351872])},
        'walk': {'from_index': draw['walked_from_index'], 'to_index': draw['walked_to_index'], 'rows': len(w),
                 'skipped_unread_population': sum(1 for r in w if r['why'].startswith('skipped unread')),
                 'read': len(read), 'read_inside_population': sum(1 for r in read if r['in_population']),
                 'not_found_404': draw['not_found_404'],
                 'g2_outcomes_of_read_rows': dict(collections.Counter(
                     'qualifies' if r['g2_evidence'].get('qualifying_notice') else r['g2_evidence'].get('why') for r in read)),
                 'profile_listings_read': sum(1 for r in read if 'lot_placed_by_profile' in r['g2_evidence']),
                 'lot_placed_notices_missing_from_listing': sum(1 for r in read if r['g2_evidence'].get('lot_placed_notices_not_in_listing')),
                 'r208_lot_placed_notices_with_own_lot_sections': sum(1 for r in read if r['g2_evidence'].get('r208_notices_with_own_lot_sections')),
                 'old_signal_false_positives_skipped': [(r['index'], r['id']) for r in read if r['old_signal_hits'] and not r['held']],
                 'gets': draw['gets'], 'fetch_errors': draw['fetch_errors'],
                 'started': open(os.path.join(S, 'draw_started.txt')).read().strip(), 'written': '2026-10-18T18:07:28Z'},
        'held_new': [{'index': r['index'], 'id': r['id'], 'year': r['year'], 'out': r['out'], 'g2': r['g2']} for r in new],
        'draw_30': draw['draw'], 'held': draw['held'], 'out': draw['out'], 'g2': draw['g2'], 'complete': draw['complete'],
        'raw': os.path.join(S, 'draw_ext.json'), 'raw_cache': os.path.join(S, 'draw_cache'),
    },
    'bar_i': {k: bar[k] for k in ('mode', 'drain', 'read_started', 'read_finished', 'cache', 'tenders', 'read', 'blocks',
                                  'parity_ok', 'parity_breaks', 'tenders_with_breaks', 'by_cause', 'by_reason', 'gets',
                                  'fetch_errors', 'incomplete', 'computed_not_served', 'served_outside_chain',
                                  'non_legacy_mismatches', 'legacy_test_disagrees', 'profile_from_prefix',
                                  'parity_verdict', 'pass_bar_i')},
    'stability_27': {k: v for k, v in lab['stability27'].items() if k != 'per_tender'} | {
        'against': 'fresh568 bar read 2026-10-18T15:43-15:53Z (rb8/fresh568/bar_cache): served lot of every block and '
                   'the Tender version'},
    'labels': {
        'order': 'labels_ext.py (3 new Tenders) written 18:15:31Z from view.py over the extension bar cache (pre-guard '
                 'replay lot only), before join_ext.py first ran (18:15:39Z+); of the bar read only tenders/read/'
                 'incomplete/fetch_errors/gets had been looked at. The 27 kept Tenders reuse fresh568 labels_fresh568.py '
                 '(112 blocks, verifier-agreed 112/112). Disclosure: the draw walk necessarily read served lot_results; '
                 'its printed G2 evidence gave per-Tender counts of lot-placed r208 results (24 / 19 / 22), which beside '
                 'the views\' 24 / 20 / 22 blocks implied that one 4836560 block was served null, not which one',
        'extension3': {k: v for k, v in lab['extension3'].items() if not (isinstance(v, list) and v and isinstance(v[0], dict))},
        'all30': {k: v for k, v in lab['all30'].items() if not (isinstance(v, list) and v and isinstance(v[0], dict))},
        'wrong_caught_all30': [f"{b['tender_id']} {b['notice_id']}/{b['key']} {b['label']} ({b['port_reason']})" for b in lab['all30']['wrong_caught']],
        'right_refused_all30': [f"{b['tender_id']} {b['notice_id']}/{b['key']} ({b['port_reason']}; cause {b['cause']})" for b in lab['all30']['right_refused']],
        'unlabelled': lab['unlabelled'],
    },
    'per_new_tender': per_new,
    'bar_ii': {'tenders_with_wrong_kept': kept_wrong_tenders, 'count': len(kept_wrong_tenders), 'limit': 2,
               'pass': len(kept_wrong_tenders) <= 2},
    'bar_iii': {'wrong_kept': [f"{b['tender_id']} {b['notice_id']}/{b['key']}" for b in lab['all30']['wrong_kept']],
                'new_kept_wrong_shapes': [], 'reopen': bool(bar['parity_breaks']) or len(kept_wrong_tenders) >= 3},
    'recorded_shapes': [
        {'kind': 'false refusal (information; not a bar criterion; a refusal is pre-568 behaviour)',
         'block': '4836560 17518149/RES-3', 'guard': 'G5', 'cause': 'unchanged (base null by G2; null now by G5)',
         'shape': "inherited carried-title collision: relaunch CN 17244355 ('Travaux Wallon lots 4,6,7 et 14') prints "
                  "LOT_NUMBER 1..4 positionally, so its lot-14 section ('lot 14 : Equipements de cuisine') is carried "
                  "onto key LOT-4 (title never superseded later), while the real LOT-14 has only the anchor's "
                  "description, no title. The right lot-14 block 'Lot 14 : Equipements de cuisine' then shares no title "
                  "key with LOT-14 and equals exactly one OTHER lot's title (LOT-4): G5 fires"},
        {'kind': 'kept block on a lot whose carried text is another lot\'s (labelled single_right, recorded caveat)',
         'block': '4836560 17518149/RES-1 -> LOT-4',
         'shape': "the same 17244355 carry defect: LOT-4 (the anchor's lot 4, 'étanchéité - zinguerie') now serves lot "
                  "14's title/description; the block 'Lot 04 : Etanchéité - zinguerie' (contract 13003, MJP Bardage) is "
                  "on lot 4's only key by the procedure's numbering. Inherited (pre-570 the section id LOT-4 lands on "
                  "LOT-4 too), 570's printed-number remap cannot see a positional relaunch. If a reviewer judged this "
                  "block wrong_other, bar (ii) would read 1/30, still a pass, and (iii) would record it as an inherited "
                  "kept shape"},
        {'kind': 'inherited weld (information)', 'tender': 5048894,
         'shape': "anchor CN 17545429 is file CNWL/T025/CS (2013), the CAN's CN 18025780 file CNWL/T034 (2014): a "
                  "re-tender in one Tender (issue 364's class); both name the same three lots and 570 maps the 2014 "
                  "numbering (1 Hardware, 2 Special Seating) onto the 2013 keys, so all 24 blocks land right"},
        {'kind': 'party data (information, outside lot placement)', 'tender': 5226033,
         'shape': "the INFRUCTUEUX blocks' winner is the word 'infructueux' parsed as an organization"},
    ],
    'files': {
        'summaries': HERE,
        'raw': S,
        'views': [os.path.join(S, f'view_{t}.txt') for t in ext_ids],
    },
}
json.dump(result, open(os.path.join(HERE, 'result.json'), 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
print(json.dumps({k: result[k] for k in ('bar_ii', 'bar_iii')}, ensure_ascii=False))
print(json.dumps(result['bar_i'], ensure_ascii=False)[:600])
print(json.dumps(result['per_new_tender'], ensure_ascii=False, indent=1))
