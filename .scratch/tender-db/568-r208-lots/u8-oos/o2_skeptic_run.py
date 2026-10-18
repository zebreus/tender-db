"""568 u8-oos group o2 SKEPTIC: re-run the unit-7 rule on the cached o2 reads with the skeptic's labels.

It imports u7/g3_port.py unchanged and runs replay(), then apply_final() per award notice; apply_final calls
features(). It joins o2_skeptic_labels.py and diffs every block against the measurer's o2_measured.json
(label, guards, served lot). It also runs two checks:
  * random12: twelve single_right blocks with no guard fired, drawn with random.Random('568u8-o2-skeptic-12'),
    each printed beside its served lot's CN text and value.
  * counterfactual_5416534: the counter CAN 18634900 with every block re-keyed to the CN lot its own title
    names, and the printed number set to that lot's number. This is what a CAN printing the CN's own numbering
    would look like, so it tests the measurer's "latent G3 false positive" (zad is not a compound half).
Output: o2_final.json
Usage:  python3 -I -B o2_skeptic_run.py
"""
import collections, copy, json, os, random, re, sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
U7 = os.path.join(os.path.dirname(HERE), 'u7')
sys.path.insert(0, U7)
sys.path.insert(0, HERE)
import g3_port as G  # noqa: E402
import o2_skeptic_labels as SL  # noqa: E402

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
    e = SL.LABELS[tid]
    return e.get('blocks', {}).get(f'{can}/{res}', e['default'])


def run_guards(blocks):
    by_notice = collections.defaultdict(list)
    for b in blocks:
        by_notice[b['can_notice_id']].append(b)
    for bl in by_notice.values():
        G.apply_final(bl)


def counterfactual_5416534(tid, tender, notices):
    """Re-key every 18634900 block to the CN lot its title names (printed := that lot's own number)."""
    blocks = [b for b in G.replay(tid, tender, notices) if b['can_notice_id'] == 18634900]
    inherited = blocks[0]['inherited']
    by_title = {}
    for key, v in inherited.items():
        for t in v['texts']:
            m = re.match(r'\s*pakiet\s+(\d+)\s*(?:[-–]\s*pozycja\s+(?:nr\s+)?(\d+))?', t, re.IGNORECASE)
            if m:
                by_title[(int(m.group(1)), int(m.group(2)) if m.group(2) else None)] = (key, v['printed'])
    out, unmapped = [], []
    for b in blocks:
        t = b['titles'][0]
        m = re.match(r'\s*pakiet\s+(\d+)\s*(?:zad\.?\s*(\d+))?\s*$', t, re.IGNORECASE)
        k = (int(m.group(1)), int(m.group(2)) if m.group(2) else None) if m else None
        if k not in by_title:
            unmapped.append((b['result_key'], t))
            continue
        key, printed = by_title[k]
        c = copy.deepcopy(b)
        c['printed'], c['number_values'], c['lot_568'] = printed, [printed], key
        out.append(c)
    G.apply_final(out)
    fired = collections.Counter(g for c in out for g in c['guards'])
    return {
        'what': "CAN 18634900 with every block re-keyed to the CN lot its own title names and the CN's number printed "
                "(a counterfactual: the same award texts on a CAN that used the CN's numbering)",
        'blocks': len(out), 'unmapped': unmapped,
        'guards_fired_blocks': dict(fired),
        'refused': [f"{c['result_key']} '{c['titles'][0]}' -> {c['lot_568']} {c['guards']} Df={c['f']['Df']}"
                    for c in out if c['guards']],
        'g4_contradicting_blocks': [c['result_key'] for c in out if c['f']['contradicts']],
    }


def lot_text(b):
    lot = b['inherited'].get(b['lot_568']) or {}
    return {'lot_texts': [t[:160] for t in lot.get('texts', [])[:2]], 'lot_value': lot.get('value'),
            'award': b['award']}


def main():
    ids, tenders, notices = load_group()
    measured = {(r['tender_id'], r['can_notice_id'], r['result_key']): r
                for r in ld(os.path.join(HERE, 'o2_measured.json'))['blocks']}
    rows, problems, all_blocks = [], [], {}
    for tid in ids:
        t = tenders[tid]
        served = {(r['notice_id'], r['key']): r for r in t.get('lot_results', [])}
        blocks = G.replay(tid, t, notices)
        run_guards(blocks)
        replayed = {(b['can_notice_id'], b['result_key']) for b in blocks}
        for k, r in served.items():
            if r.get('lot') and k not in replayed:
                problems.append({'tender_id': tid, 'unreplayed_served': list(k), 'lot': r['lot']})
        for b in blocks:
            sv = served.get((b['can_notice_id'], b['result_key']))
            served_lot = sv['lot'] if sv else None
            if served_lot != b['lot_568']:
                problems.append({'tender_id': tid, 'can': b['can_notice_id'], 'res': b['result_key'],
                                 'served': served_lot, 'replay_568': b['lot_568']})
            if b['lot_568'] is None:
                continue
            cls, why = label_of(tid, b['can_notice_id'], b['result_key'])
            all_blocks[(tid, b['can_notice_id'], b['result_key'])] = b
            rows.append({
                'tender_id': tid, 'can_notice_id': b['can_notice_id'], 'result_key': b['result_key'],
                'printed': b['printed'], 'served_lot': served_lot, 'cls': cls,
                'guards_fired': b['guards'], 'title': (b['titles'] or [''])[0][:120], 'why': why,
                'features': {'N_size': len(b['f']['N']), 'one_lot': b['f']['one_lot'], 'Df': b['f']['Df'],
                             'contradicts': b['f']['contradicts'], 'g5': b['f']['g5'],
                             'several_numbers': b['f']['several_numbers']},
            })
    rows.sort(key=lambda r: (ids.index(r['tender_id']), r['can_notice_id'], G.P.res_order(r['result_key'])))

    # diff against the measurer
    diffs = []
    for r in rows:
        m = measured.get((r['tender_id'], r['can_notice_id'], r['result_key']))
        if m is None:
            diffs.append({'block': f"{r['tender_id']}/{r['can_notice_id']}/{r['result_key']}", 'issue': 'not in o2_measured'})
            continue
        for fld in ('cls', 'served_lot', 'printed'):
            if m[fld] != r[fld]:
                diffs.append({'block': f"{r['tender_id']}/{r['can_notice_id']}/{r['result_key']}", 'field': fld,
                              'measured': m[fld], 'skeptic': r[fld]})
        if sorted(m['guards_fired']) != sorted(r['guards_fired']):
            diffs.append({'block': f"{r['tender_id']}/{r['can_notice_id']}/{r['result_key']}", 'field': 'guards',
                          'measured': m['guards_fired'], 'skeptic': r['guards_fired']})
    extra = set(measured) - {(r['tender_id'], r['can_notice_id'], r['result_key']) for r in rows}

    wrong = [r for r in rows if r['cls'] in WRONG]
    right = [r for r in rows if r['cls'] == 'single_right']
    caught = [r for r in wrong if r['guards_fired']]
    residual = [r for r in wrong if not r['guards_fired']]
    false_ref = [r for r in right if r['guards_fired']]
    t_before = sorted({r['tender_id'] for r in wrong}, key=ids.index)
    t_after = sorted({r['tender_id'] for r in residual}, key=ids.index)
    per_guard = {g: {'fired_blocks': sum(1 for r in rows if g in r['guards_fired']),
                     'on_wrong': sum(1 for r in wrong if g in r['guards_fired']),
                     'on_right': sum(1 for r in right if g in r['guards_fired']),
                     'tenders': sorted({r['tender_id'] for r in rows if g in r['guards_fired']}, key=ids.index)}
                 for g in ('G3', 'G4', 'G5', 'G6')}

    sr_free = [r for r in rows if r['cls'] == 'single_right' and not r['guards_fired']]
    pick = sorted(random.Random('568u8-o2-skeptic-12').sample(sr_free, 12),
                  key=lambda r: (r['tender_id'], r['can_notice_id'], G.P.res_order(r['result_key'])))
    random12 = []
    for r in pick:
        b = all_blocks[(r['tender_id'], r['can_notice_id'], r['result_key'])]
        random12.append({'block': f"{r['tender_id']}/{r['can_notice_id']}/{r['result_key']}", 'printed': r['printed'],
                         'served_lot': r['served_lot'], 'titles': [x[:160] for x in b['titles']], **lot_text(b)})

    cf = counterfactual_5416534(5416534, tenders[5416534], notices)

    summary = {
        'tenders_read': len(ids), 'matched_blocks': len(rows),
        'tenders_without_matched_blocks': [tid for tid in ids if not any(r['tender_id'] == tid for r in rows)],
        'replay_vs_served_problems': problems,
        'labels': dict(collections.Counter(r['cls'] for r in rows)),
        'tenders_with_a_wrong_block_before': {'k': len(t_before), 'n': len(ids), 'ci95': G.cp(len(t_before), len(ids)), 'ids': t_before},
        'tenders_with_a_wrong_block_after_u7': {'k': len(t_after), 'n': len(ids), 'ci95': G.cp(len(t_after), len(ids)), 'ids': t_after},
        'wrong_blocks_caught': {'k': len(caught), 'n': len(wrong), 'ci95': G.cp(len(caught), len(wrong))},
        'right_blocks_refused': {'k': len(false_ref), 'n': len(right), 'ci95': G.cp(len(false_ref), len(right)),
                                 'all_in': sorted({f"{r['tender_id']}/{r['can_notice_id']}" for r in false_ref})},
        'right_blocks_refused_outside_counter_notices': sum(1 for r in false_ref if r['tender_id'] != 5416534),
        'residual_wrong_blocks': [f"{r['tender_id']}/{r['can_notice_id']}/{r['result_key']}" for r in residual],
        'per_guard': per_guard,
        'diff_vs_measurer': diffs, 'blocks_only_in_measurer': sorted(map(list, extra)),
        'labels_changed_by_skeptic': SL.CHANGED,
    }
    out = {
        'group': 'o2', 'issue': 568, 'unit': '8-oos skeptic',
        'tenders_read': len(ids),
        'tenders_without_matched_blocks': summary['tenders_without_matched_blocks'],
        'port_driver': os.path.join(HERE, 'o2_skeptic_run.py') + ' (python3 -I -B; imports u7/g3_port.py unchanged: '
                       'replay -> apply_final per CAN; labels o2_skeptic_labels.py)',
        'summary': summary,
        'random12_single_right': random12,
        'counterfactual_5416534': cf,
        'blocks': rows,
    }
    with open(os.path.join(HERE, 'o2_final.json'), 'w', encoding='utf-8') as fh:
        fh.write('{\n' + ',\n'.join(f'"{k}": ' + json.dumps(v, ensure_ascii=False, indent=None if k == 'blocks' else 1)
                                     for k, v in out.items() if k != 'blocks'))
        fh.write(',\n"blocks": [\n' + ',\n'.join(json.dumps(r, ensure_ascii=False) for r in rows) + '\n]\n}\n')
    print(json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(cf, ensure_ascii=False, indent=1))
    for x in random12:
        print(json.dumps(x, ensure_ascii=False))


if __name__ == '__main__':
    main()
