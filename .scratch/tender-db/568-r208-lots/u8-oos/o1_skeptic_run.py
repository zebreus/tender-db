"""568 u8-oos group o1, SKEPTIC: re-run the unit-7 rule on the skeptic's labels and write o1_final.json.

Order: o1_skeptic_labels.json was written (o1_skeptic_labels.py) from the re-read before this driver ran.
Input:  cache_o1/tenders_<id>.json, cache_o1/notices_<id>_content.json (the measurer's raw reads, unchanged)
        o1_skeptic_labels.json
Rule:   ../u7/g3_port.py replay() -> apply_final() per award notice, unchanged (imported, python3 -I).
Checks: the served set (lot != null on an award notice without own lot sections) = the 568 replay set;
        served lot = replay lot per block; an exact-value cross-check (block estimate/award equal to a CN lot
        value: which lots); the 12 random single_right blocks re-read (seeded sample, outside the guarded CAN).
Output: o1_final.json (meta, summary, blocks in the StructuredOutput shape + features).
Usage:  python3 -I o1_skeptic_run.py
"""
import collections, json, os, random, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
U7 = os.path.join(ROOT, 'u7')
sys.path.insert(0, ROOT)
sys.path.insert(0, U7)
import g3_port as G  # noqa: E402
import cross_notice_port as P  # noqa: E402

CACHE = os.path.join(HERE, 'cache_o1')


def ld(name):
    with open(os.path.join(CACHE, name), encoding='utf-8') as fh:
        return json.load(fh)


def amounts(section):
    return {v['field_id']: (v.get('cents'), v.get('currency')) for v in section.get('values', []) if v.get('type') == 'amount'}


def main():
    draw = json.load(open(os.path.join(HERE, 'draw.json'), encoding='utf-8'))
    ids = next(g['ids'] for g in draw['groups'] if g['key'] == 'o1')
    labels = {(b['tender_id'], b['can_notice_id'], b['result_key']): b
              for b in json.load(open(os.path.join(HERE, 'o1_skeptic_labels.json'), encoding='utf-8'))}
    measurer = {(b['tender_id'], b['can_notice_id'], b['result_key']): b
                for b in json.load(open(os.path.join(HERE, 'o1_labels.json'), encoding='utf-8'))}

    rows, problems, read, without = [], [], 0, []
    for tid in ids:
        t = ld(f'tenders_{tid}.json')
        read += 1
        chain = [v['caused_by_notice_id'] for v in sorted(t['versions'], key=lambda v: v['seq'])]
        notices = {n: ld(f'notices_{n}_content.json') for n in chain}
        own_lots = {n: any(s['kind'] in P.LOT_KINDS for s in notices[n].get('sections') or []) for n in chain}
        served = {(r['notice_id'], r['key']): r for r in t.get('lot_results', [])}
        lot_values = {}
        for n in chain:
            for s in notices[n].get('sections') or []:
                if s['kind'] in P.LOT_KINDS:
                    for f in ('TED-VALUE_COST', 'TED-VAL_ESTIMATED_TOTAL', 'TED-VAL_OBJECT'):
                        a = amounts(s).get(f)
                        if a and a[0]:
                            lot_values[s['section_id']] = a
                            break
        blocks = G.replay(tid, t, notices)
        by_notice = collections.defaultdict(list)
        for b in blocks:
            by_notice[b['can_notice_id']].append(b)
        for bl in by_notice.values():
            G.apply_final(bl)
        served_scope = {(n, k) for (n, k), r in served.items() if r.get('lot') and n in own_lots and not own_lots[n]}
        replay_scope = {(b['can_notice_id'], b['result_key']) for b in blocks if b['lot_568'] is not None}
        if served_scope != replay_scope:
            problems.append({'tender_id': tid, 'served_only': sorted(served_scope - replay_scope),
                             'replay_only': sorted(replay_scope - served_scope)})
        n_matched = 0
        for b in blocks:
            if b['lot_568'] is None:
                continue
            n_matched += 1
            key = (tid, b['can_notice_id'], b['result_key'])
            sv = served.get((b['can_notice_id'], b['result_key']))
            lab = labels.get(key)
            if lab is None:
                problems.append({'unlabelled': list(key)})
            if sv is None or sv.get('lot') != b['lot_568']:
                problems.append({'served_vs_replay': list(key), 'served': None if sv is None else sv.get('lot'), 'replay': b['lot_568']})
            sec = P.sections_of(notices[b['can_notice_id']])[b['result_key']]
            am = amounts(sec)
            eq = {}
            for f in ('TED-INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT.VALUE_COST', 'TED-VALUE_COST', 'TED-VAL_TOTAL'):
                if f in am and am[f][0]:
                    hit = sorted(k for k, v in lot_values.items() if v == am[f])
                    if hit:
                        eq[f.replace('TED-', '')] = hit
            f = b['f']
            m = measurer.get(key, {})
            rows.append({
                'tender_id': tid, 'can_notice_id': b['can_notice_id'], 'result_key': b['result_key'],
                'printed': b['printed'], 'served_lot': None if sv is None else sv.get('lot'),
                'cls': lab['cls'] if lab else 'unlabelled', 'guards_fired': b['guards'],
                'title': (b['titles'] or [''])[0][:200], 'why': lab['why'] if lab else None,
                'measurer_cls': m.get('cls'), 'lot_568': b['lot_568'], 'lot_u7': b['lot_u7'],
                'value_equal_lots': eq,
                'features': {'N': len(f['N']), 'one_lot': f['one_lot'], 'Df': f['Df'], 'contradicts': f['contradicts'],
                             'g5': f['g5'], 'several_numbers': f['several_numbers'],
                             'ratio': None if f['ratio'] is None else round(f['ratio'], 4)},
            })
        if n_matched == 0:
            without.append(tid)
    missing = sorted(set(labels) - {(r['tender_id'], r['can_notice_id'], r['result_key']) for r in rows})
    if missing:
        problems.append({'labels_without_a_matched_block': [list(k) for k in missing]})

    # value cross-check: a block whose estimate/award equals another lot's CN value and not its own
    value_flags = [f"{r['tender_id']}/{r['result_key']} served {r['served_lot']} value= {r['value_equal_lots']}"
                   for r in rows if r['value_equal_lots'] and not any(r['served_lot'] in v for v in r['value_equal_lots'].values())]

    wrong = ('multi_lot_contract', 'wrong_other')
    cnt = collections.Counter(r['cls'] for r in rows)
    per_guard = {g: dict(collections.Counter(r['cls'] for r in rows if g in r['guards_fired'])) for g in ('G3', 'G4', 'G5', 'G6')}
    t_before = sorted({r['tender_id'] for r in rows if r['cls'] in wrong})
    t_after = sorted({r['tender_id'] for r in rows if r['cls'] in wrong and r['lot_u7'] is not None})
    right = [r for r in rows if r['cls'] == 'single_right']
    fr = [r for r in right if r['guards_fired']]
    fr_out = [r for r in right if r['guards_fired'] and r['tender_id'] != 8286472]
    g3_alone_right = [r for r in right if 'G3' in r['guards_fired']]
    rng = random.Random('568u8-o1-skeptic')
    pool = sorted((r for r in right if not r['guards_fired']), key=lambda r: (r['tender_id'], r['can_notice_id'], P.res_order(r['result_key'])))
    sample = rng.sample(pool, 12)
    summary = {
        'tenders_read': read, 'tenders_without_matched_blocks': without, 'matched_blocks': len(rows),
        'by_class': dict(cnt), 'class_changes_vs_measurer': [f"{r['tender_id']}/{r['result_key']}: {r['measurer_cls']} -> {r['cls']}"
                                                             for r in rows if r['measurer_cls'] != r['cls']],
        'refused_by_guard_and_class': per_guard,
        'refused_by_class': dict(collections.Counter(r['cls'] for r in rows if r['guards_fired'])),
        'false_refusals': {'k': len(fr), 'n': len(right), 'ci95': G.cp(len(fr), len(right)),
                           'ids': [f"{r['tender_id']}/{r['can_notice_id']}/{r['result_key']}" for r in fr]},
        'false_refusals_outside_the_counter_notice': {'k': len(fr_out), 'n': len([r for r in right if r['tender_id'] != 8286472]),
                                                      'ci95': G.cp(len(fr_out), len([r for r in right if r['tender_id'] != 8286472]))},
        'g3_on_right_blocks': {'k': len(g3_alone_right), 'n': len(right), 'ci95': G.cp(len(g3_alone_right), len(right)),
                               'ids': [f"{r['tender_id']}/{r['result_key']}" for r in g3_alone_right]},
        'wrong_caught': {'k': sum(1 for r in rows if r['cls'] in wrong and r['guards_fired']),
                         'n': sum(1 for r in rows if r['cls'] in wrong)},
        'wrong_blocks_kept': [f"{r['tender_id']}/{r['can_notice_id']}/{r['result_key']} ({r['cls']})" for r in rows
                              if r['cls'] in wrong and not r['guards_fired']],
        'tenders_with_a_wrong_block_before': {'k': len(t_before), 'n': read, 'ci95': G.cp(len(t_before), read), 'ids': t_before},
        'tenders_with_a_wrong_block_after': {'k': len(t_after), 'n': read, 'ci95': G.cp(len(t_after), read), 'ids': t_after},
        'tenders_touched_by_the_guards': sorted({r['tender_id'] for r in rows if r['guards_fired']}),
        'value_cross_check_flags': value_flags,
        'random_12_single_right_reread': [f"{r['tender_id']}/{r['can_notice_id']}/{r['result_key']}" for r in sample],
        'consistency_problems': problems,
    }
    meta = {'issue': 568, 'unit': '8-oos', 'group': 'o1', 'role': 'skeptic-final',
            'what': 'Skeptic re-read of the o1 measurer (o1_labels.json / o1_measured.json): every guarded block, every '
                    'wrong/multi/unclear block, 12 seeded-random single_right blocks, and in fact every one of the 168 matched '
                    'blocks; labels in o1_skeptic_labels.json written before this driver ran; the unit-7 rule re-run unchanged.',
            'driver': 'o1_skeptic_run.py', 'labels': 'o1_skeptic_labels.json (o1_skeptic_labels.py)',
            'notes': [
                'No class changed. 159 single_right, 9 wrong_other, 0 multi_lot_contract, 0 unclear. The rule re-run reproduces '
                'the measurer exactly: G4 on 17 blocks (9 wrong + 8 right), G3 on 10 (7 wrong + 3 right), G5 0, G6 0. The served set '
                'equals the replay set, and every served lot equals the replay lot.',
                '8286472 / CAN 16994904 is confirmed as a contract counter. LOT_NUMBER = CONTRACT_NO 01..17. Lots 9 (423,000) and 19 '
                '(366,642.54) are missing from the CAN value list. The value cross-check finds that RES-9..17 each have the estimate '
                'of LOT-(k+1) exactly. G4 (notice) catches 9/9. The 8 refused right blocks are the known G4 cost (unit-7 risk 3, the '
                '5051007 shape). Outside this notice there are 0 of 151 false refusals.',
                'G3 false positive confirmed: lot_designators.designators() reads "(pakiet 8 pozycji) ... - zadanie 4" as '
                '[8, 4]. The count phrase "pakiet N pozycji/pozycje" (a package of N items) counts as a designator, so G3 alone '
                'refuses 3 of 159 right blocks (RES-4, 5, 8). G4 masks this here. In a notice not numbered by a counter, it '
                'would be a real G3 false refusal whenever N is a number one of the inherited lots prints and differs from k. '
                'Without G4, G3 catches only 7 of the 9 wrong blocks: RES-9 "pakiet 36" (36 not in N) and RES-16 "pakiet 17 / '
                'zadanie 17" leave |Dn| = 1.',
                '8338322 is no longer label-by-numbering-only. The CN III.2.3 prints a per-task experience threshold '
                '"dla zadania nr k". Each block\'s estimate / 2 (24-month contract) sits just above the threshold for its '
                'printed k, on all 13 blocks. That includes 18 vs 19, whose descriptions are identical.',
                'The measurer wrote 5084145\'s ratios as 4.5x..197x; they actually run 4.5x..763x (RES-8, Вариант 12). The titles '
                'name exactly one lot each and no lot permutation fits the values, so these are value anomalies. 5179195 CAN '
                'estimates are 1/4 of the CN lot values (the CN\'s 480,000 for lot 3 is a typo for 48,000). 5251601 RES-19\'s '
                'estimate is copied from LOT-22 / RES-20, while its title and award fit LOT-21.',
                'Cache spot-check: live GET /v1/tenders/8286472 and /v1/notices/16994904/content (2 GETs, both 200) equal '
                'cache_o1 as parsed JSON (lot_results, versions, full notice content).'],
            }
    with open(os.path.join(HERE, 'o1_final.json'), 'w', encoding='utf-8') as fh:
        fh.write('{\n"meta": ' + json.dumps(meta, ensure_ascii=False, indent=1))
        fh.write(',\n"summary": ' + json.dumps(summary, ensure_ascii=False, indent=1))
        fh.write(',\n"blocks": [\n' + ',\n'.join(json.dumps(r, ensure_ascii=False) for r in rows) + '\n]\n}\n')
    print(json.dumps(summary, ensure_ascii=False, indent=1))


if __name__ == '__main__':
    main()
