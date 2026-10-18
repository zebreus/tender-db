"""568 u8-oos group o1: run the unit-7 rule (G3 | G4 notice | G5 | G6) on the labelled out-of-sample blocks.

Order of work: the labels in o1_labels.json were written from o1_view.py (which computes no guard) BEFORE
this driver first ran; this driver only joins them.

Input:  cache_o1/tenders_<id>.json      (GET /v1/tenders/{id})
        cache_o1/notices_<id>_content.json (GET /v1/notices/{id}/content, every chain notice)
        o1_labels.json                   (per matched block: cls + why)
Rule:   ../u7/g3_port.py replay() -> apply_final() per award notice, unchanged (imported, python3 -I).
Output: o1_measured.json (per block: printed, served lot, 568 replay lot, class, guards fired, u7 lot,
        the guard features) + a summary.
Usage:  python3 -I o1_run_rule.py
"""
import collections, json, os, sys

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


def main():
    draw = json.load(open(os.path.join(HERE, 'draw.json'), encoding='utf-8'))
    ids = next(g['ids'] for g in draw['groups'] if g['key'] == 'o1')
    labels = {(b['tender_id'], b['can_notice_id'], b['result_key']): b
              for b in json.load(open(os.path.join(HERE, 'o1_labels.json'), encoding='utf-8'))}

    rows, problems, read = [], [], 0
    tenders_without = []
    for tid in ids:
        t = ld(f'tenders_{tid}.json')
        read += 1
        chain = [v['caused_by_notice_id'] for v in sorted(t['versions'], key=lambda v: v['seq'])]
        notices = {n: ld(f'notices_{n}_content.json') for n in chain}
        own_lots = {n: any(s['kind'] in P.LOT_KINDS for s in notices[n].get('sections') or []) for n in chain}
        served = {(r['notice_id'], r['key']): r for r in t.get('lot_results', [])}

        blocks = G.replay(tid, t, notices)
        by_notice = collections.defaultdict(list)
        for b in blocks:
            by_notice[b['can_notice_id']].append(b)
        for bl in by_notice.values():
            G.apply_final(bl)

        # The 568 scope from the served side: lot != null on a block of an award notice without own lot sections.
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
                problems.append({'served_vs_replay': list(key), 'served': None if sv is None else sv.get('lot'),
                                 'replay': b['lot_568']})
            f = b['f']
            rows.append({
                'tender_id': tid, 'can_notice_id': b['can_notice_id'], 'result_key': b['result_key'],
                'printed': b['printed'], 'number_values': b['number_values'],
                'served_lot': None if sv is None else sv.get('lot'), 'lot_568': b['lot_568'], 'why_568': b['why_568'],
                'cls': lab['cls'] if lab else 'unlabelled', 'why': lab['why'] if lab else None,
                'title': (b['titles'] or [''])[0][:200],
                'guards_fired': b['guards'], 'lot_u7': b['lot_u7'],
                'features': {'N': f['N'], 'one_lot': f['one_lot'], 'Df': f['Df'], 'contradicts': f['contradicts'],
                             'g5': f['g5'], 'several_numbers': f['several_numbers'],
                             'ratio': None if f['ratio'] is None else round(f['ratio'], 4)},
            })
        if n_matched == 0:
            tenders_without.append(tid)
    missing_labels = sorted(set(labels) - {(r['tender_id'], r['can_notice_id'], r['result_key']) for r in rows})
    if missing_labels:
        problems.append({'labels_without_a_matched_block': [list(k) for k in missing_labels]})

    wrong = ('multi_lot_contract', 'wrong_other')
    cnt = collections.Counter(r['cls'] for r in rows)
    caught = collections.Counter(r['cls'] for r in rows if r['guards_fired'])
    per_guard = {g: collections.Counter(r['cls'] for r in rows if g in r['guards_fired']) for g in ('G3', 'G4', 'G5', 'G6')}
    t_before = sorted({r['tender_id'] for r in rows if r['cls'] in wrong})
    t_after = sorted({r['tender_id'] for r in rows if r['cls'] in wrong and r['lot_u7'] is not None})
    touched = sorted({r['tender_id'] for r in rows if r['guards_fired']})
    summary = {
        'tenders_read': read,
        'tenders_with_matched_blocks': read - len(tenders_without),
        'tenders_without_matched_blocks': tenders_without,
        'matched_blocks': len(rows),
        'by_class': dict(cnt),
        'refused_by_class': dict(caught),
        'refused_by_guard_and_class': {g: dict(c) for g, c in per_guard.items()},
        'false_refusals': [f"{r['tender_id']}/{r['can_notice_id']}/{r['result_key']}" for r in rows
                           if r['cls'] == 'single_right' and r['guards_fired']],
        'wrong_blocks_kept': [f"{r['tender_id']}/{r['can_notice_id']}/{r['result_key']} ({r['cls']})" for r in rows
                              if r['cls'] in wrong and not r['guards_fired']],
        'tenders_with_a_wrong_block_before': {'k': len(t_before), 'n': read, 'ci95': G.cp(len(t_before), read), 'ids': t_before},
        'tenders_with_a_wrong_block_after': {'k': len(t_after), 'n': read, 'ci95': G.cp(len(t_after), read), 'ids': t_after},
        'tenders_touched_by_the_guards': touched,
        'consistency_problems': problems,
    }
    out = {'meta': {'issue': 568, 'unit': '8-oos', 'group': 'o1',
                    'what': 'Out-of-sample check of the unit-7 refusal guards on draw.json group o1 (25 Tenders): '
                            'every block the 568 rule matched, labelled first (o1_labels.json, from o1_view.py, '
                            'no guard computed), then the decided rule run unchanged via u7/g3_port.py.',
                    'classes': {'single_right': 'one lot, correctly placed',
                                'multi_lot_contract': 'one award for several lots',
                                'wrong_other': 'on a lot that is not its lot for another reason',
                                'unclear': 'cannot tell'},
                    'reads': 'GET /v1/tenders/{id} + /v1/notices/{id}/content for every chain notice, '
                             '1 GET per 1.6 s, raw in cache_o1/ (o1_fetch.py, log o1_fetch.log)',
                    'notes': [
                        '8286472 (CN 12587380, CAN 16994904) is a contract-counter notice, the 5051007 shape: LOT_NUMBER = '
                        'CONTRACT_NO 1..17 while the titles say "zadanie 1..8, 10..18" (lot 9 and 19 unawarded). RES-1..8 '
                        'are right by coincidence, RES-9..17 sit one lot low. G4 (notice) refuses all 17: 9/9 wrong caught, '
                        '8 coincidental rights lost (decision risk 3).',
                        'New G3 keyword false-positive shape (no net effect here, G4 already refuses the notice): the '
                        'count phrase "(pakiet N pozycji/pozycje)" = "a package of N items" reads as designator Pakiet N. '
                        'With "zadanie k" it gives |Dn| = 2 on RES-4 (pakiet 8, zadanie 4), RES-5 (6, 5) and RES-8 '
                        '(3, 8), all right blocks. In each case the extra number is also in the matched lot\'s own CN '
                        'title "(pakiet N pozycji)", which G4 already credits but G3 does not.',
                        '5439807 F06 award notices 19223648/19225856/19377201/19483641 carry their own lot sections '
                        '(567 same-notice keys) and are out of the 568 scope; not labelled.'],
                    },
           'summary': summary, 'blocks': rows}
    with open(os.path.join(HERE, 'o1_measured.json'), 'w', encoding='utf-8') as fh:
        fh.write('{\n"meta": ' + json.dumps(out['meta'], ensure_ascii=False, indent=1))
        fh.write(',\n"summary": ' + json.dumps(summary, ensure_ascii=False, indent=1))
        fh.write(',\n"blocks": [\n' + ',\n'.join(json.dumps(r, ensure_ascii=False) for r in rows) + '\n]\n}\n')
    print(json.dumps(summary, ensure_ascii=False, indent=1))


if __name__ == '__main__':
    main()
