"""568 u8-oos group o2: run the unit-7 rule (u7/g3_port.py: replay -> features -> apply_final) on the cached
o2 reads, and join the hand labels of o2_labels.py (written before this driver was run).

Input:  cache_o2/tenders_<id>.json (GET /v1/tenders/{id}) and cache_o2/notices_<id>_content.json
        (GET /v1/notices/{id}/content) for every chain notice; draw.json group "o2"; o2_labels.py.
Output: o2_measured.json (per matched block: label, served lot, 568 replay lot, guards fired, features).
Usage:  python3 -I -B o2_run_rule.py          (-B: never write bytecode next to the read-only u7/ modules)
"""
import collections, json, os, sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
U7 = os.path.join(os.path.dirname(HERE), 'u7')
sys.path.insert(0, U7)
sys.path.insert(0, HERE)
import g3_port as G  # noqa: E402  (also puts ../cross_notice_port and u7/lot_designators on the path)
import o2_labels  # noqa: E402

CACHE = os.path.join(HERE, 'cache_o2')
WRONG = ('multi_lot_contract', 'wrong_other')


def ld(p):
    with open(p, encoding='utf-8') as fh:
        return json.load(fh)


def load_group():
    draw = ld(os.path.join(HERE, 'draw.json'))
    ids = next(g['ids'] for g in draw['groups'] if g['key'] == 'o2')
    tenders, notices = {}, {}
    for tid in ids:
        t = ld(os.path.join(CACHE, f'tenders_{tid}.json'))
        tenders[tid] = t
        for v in t['versions']:
            n = v['caused_by_notice_id']
            notices[n] = ld(os.path.join(CACHE, f'notices_{n}_content.json'))
    return ids, tenders, notices


def label_of(tid, can, res):
    e = o2_labels.LABELS.get(tid)
    if e is None:
        return 'unjudged', None
    cls, why = e.get('blocks', {}).get(f'{can}/{res}', e['default'])
    return cls, why


def main():
    ids, tenders, notices = load_group()
    rows, problems = [], []
    for tid in ids:
        t = tenders[tid]
        served = {(r['notice_id'], r['key']): r for r in t.get('lot_results', [])}
        blocks = G.replay(tid, t, notices)
        by_notice = collections.defaultdict(list)
        for b in blocks:
            by_notice[b['can_notice_id']].append(b)
        for bl in by_notice.values():
            G.apply_final(bl)
        for b in blocks:
            sv = served.get((b['can_notice_id'], b['result_key']))
            served_lot = sv['lot'] if sv else None
            if served_lot != b['lot_568']:
                problems.append({'tender_id': tid, 'can': b['can_notice_id'], 'res': b['result_key'],
                                 'served': served_lot, 'replay_568': b['lot_568']})
            if b['lot_568'] is None:
                continue
            cls, why = label_of(tid, b['can_notice_id'], b['result_key'])
            f = b['f']
            rows.append({
                'tender_id': tid, 'can_notice_id': b['can_notice_id'], 'result_key': b['result_key'],
                'printed': b['printed'], 'number_values': b['number_values'], 'served_lot': served_lot,
                'lot_568': b['lot_568'], 'cls': cls, 'why': why,
                'guards_fired': b['guards'], 'lot_after_u7': b['lot_u7'],
                'title': (b['titles'] or [''])[0][:200], 'titles_n': len(b['titles']),
                'features': {'N_size': len(f['N']), 'one_lot': f['one_lot'], 'D': f['D'], 'Df': f['Df'],
                             'contradicts': f['contradicts'], 'g5': f['g5'],
                             'several_numbers': f['several_numbers'],
                             'ratio': None if f['ratio'] is None else round(f['ratio'], 4)},
            })
    rows.sort(key=lambda r: (ids.index(r['tender_id']), r['can_notice_id'], G.P.res_order(r['result_key'])))

    cnt = collections.Counter(r['cls'] for r in rows)
    wrong = [r for r in rows if r['cls'] in WRONG]
    right = [r for r in rows if r['cls'] == 'single_right']
    caught = [r for r in wrong if r['guards_fired']]
    residual = [r for r in wrong if not r['guards_fired']]
    false_ref = [r for r in right if r['guards_fired']]
    t_before = sorted({r['tender_id'] for r in wrong}, key=ids.index)
    t_after = sorted({r['tender_id'] for r in residual}, key=ids.index)
    t_touched = sorted({r['tender_id'] for r in rows if r['guards_fired']}, key=ids.index)
    per_guard = {}
    for g in ('G3', 'G4', 'G5', 'G6'):
        per_guard[g] = {
            'fired_blocks': sum(1 for r in rows if g in r['guards_fired']),
            'on_wrong': sum(1 for r in wrong if g in r['guards_fired']),
            'on_multi': sum(1 for r in rows if r['cls'] == 'multi_lot_contract' and g in r['guards_fired']),
            'on_right': sum(1 for r in right if g in r['guards_fired']),
            'tenders': sorted({r['tender_id'] for r in rows if g in r['guards_fired']}, key=ids.index),
        }
    by_tender = []
    for tid in ids:
        tr = [r for r in rows if r['tender_id'] == tid]
        by_tender.append({'tender_id': tid, 'matched': len(tr),
                          'cls': dict(collections.Counter(r['cls'] for r in tr)),
                          'guards': dict(collections.Counter(g for r in tr for g in r['guards_fired'])),
                          'refused': sum(1 for r in tr if r['guards_fired'])})
    summary = {
        'tenders_read': len(ids),
        'tenders_without_matched_blocks': [tid for tid in ids if not any(r['tender_id'] == tid for r in rows)],
        'matched_blocks': len(rows),
        'replay_vs_served_disagreements': problems,
        'labels': dict(cnt),
        'tenders_with_a_wrong_block_before': {'k': len(t_before), 'n': len(ids), 'ci95': G.cp(len(t_before), len(ids)),
                                              'ids': t_before},
        'tenders_with_a_wrong_block_after_u7': {'k': len(t_after), 'n': len(ids), 'ci95': G.cp(len(t_after), len(ids)),
                                                'ids': t_after},
        'wrong_blocks_caught': {'k': len(caught), 'n': len(wrong), 'ci95': G.cp(len(caught), len(wrong))},
        'right_blocks_refused': {'k': len(false_ref), 'n': len(right), 'ci95': G.cp(len(false_ref), len(right)),
                                 'ids': [f"{r['tender_id']}/{r['can_notice_id']}/{r['result_key']}" for r in false_ref]},
        'residual_wrong_blocks': [f"{r['tender_id']}/{r['can_notice_id']}/{r['result_key']} ({r['cls']})" for r in residual],
        'tenders_touched_by_guards': t_touched,
        'per_guard': per_guard,
        'by_tender': by_tender,
    }
    meta = {
        'issue': 568, 'unit': '8-oos', 'group': 'o2',
        'what': 'Out-of-sample check of the unit-7 refusal guards (G3 | G4 notice | G5 | G6, |N| < 2 exempt) on the 25 o2 '
                'Tenders of the fresh salted 50 (draw.json). Every block the 568 rule matched (served lot != null on an '
                'R2.0.8 award block of a CAN without lot sections of its own) was labelled by hand from the cached reads '
                '(o2_view.py / o2_compact.py; labels in o2_labels.py) BEFORE the rule was run; this file is the join.',
        'labels': {'single_right': 'one lot, correctly placed (title/description agree with the lot, or value/winner agree)',
                   'multi_lot_contract': 'one award for several lots (text, or value = a sum of several CN lots)',
                   'wrong_other': 'on a lot that is not its lot for another reason (contract counter, CAN-only '
                                  'renumbering, mis-chained CAN)',
                   'unclear': 'not decidable from the reads'},
        'reads': 'GET /v1/tenders/{id} + GET /v1/notices/{id}/content for every chain notice, 1 GET per 1.7 s, no 408 '
                 'seen (o2_fetch.py, o2_fetch.log; 25 tenders, 67 notices)',
        'driver': 'o2_run_rule.py imports u7/g3_port.py (replay, features via apply_final) unchanged',
        # Written AFTER the run (annotation of the result, not a label; the labels are o2_labels.py).
        'observations': [
            'The only wrong Tender is 5416534 (CAN 18634900, 105 blocks): LN = CONTRACT_NUMBER = 1..105 is a contract '
            'counter, while the titles name the real pakiet and the CN numbers its 111 lots sequentially (Pakiet 16 '
            'poz 1-6 = LOT-16..21, ..., Pakiet 92 = LOT-111). Pakiet 15 is unawarded, so RES-1..14 are right by '
            'coincidence and RES-15..105 (91) sit on an earlier lot. This is the recorded 5051007 / 5267810 shape.',
            'G4 (notice) fires on the whole CAN: 91/91 wrong caught, and the 14 coincidental rights refused - the '
            'deliberate cost the decision names (risk 3). No other guard fires anywhere else in o2: 0 of the other '
            '146 right blocks on 24 Tenders are refused.',
            'Latent G3 false positive seen (no effect here): "Pakiet 16 zad.1", "Pakiet 59 zad. 2", "Pakiet 88 '
            'zad.13" read as TWO designators {16, 1} because zad is a lot keyword and not a compound second half '
            '(POSITION_WORDS). Each names ONE CN lot (the CN describes LOT-16 as "Pakiet 16 - pozycja nr 1"). G3 '
            'fires on these 22 blocks; they are wrong anyway and G4 already refuses them, but on a CAN that printed '
            "the CN's own numbers (16..21) G3 would refuse 22 right blocks (G4 would not: the lot's own text carries "
            'both designators).',
            'G5 and G6 fire nowhere in o2. No multi-lot contract and no residual wrong block in o2.',
        ],
    }
    out = os.path.join(HERE, 'o2_measured.json')
    with open(out, 'w', encoding='utf-8') as fh:
        fh.write('{\n"meta": ' + json.dumps(meta, ensure_ascii=False, indent=1))
        fh.write(',\n"summary": ' + json.dumps(summary, ensure_ascii=False, indent=1))
        fh.write(',\n"blocks": [\n' + ',\n'.join(json.dumps(r, ensure_ascii=False) for r in rows) + '\n]\n}\n')
    print(json.dumps({k: v for k, v in summary.items() if k != 'by_tender'}, ensure_ascii=False, indent=1))
    for bt in by_tender:
        print(bt)


if __name__ == '__main__':
    main()
