"""568 unit 8: the AMENDED rule (decision-u7.md "Amendment (unit 8, 2026-10-18)") on the out-of-sample 50.

Input:  draw.json (the 50 ids, groups o1 / o2); cache_o1/, cache_o2/ (the measurers' raw reads: tenders_<id>.json,
        notices_<id>_content.json; else extract oos-cache.tar.gz here); the skeptic-final labels, o1_final.json and
        o2_final.json (`cls` per matched block); ../u7/measurement-u8.json for the pooled figures.
Rule:   ../u7/g3_port.py, imported: replay() -> per award notice apply_final() (FINAL, as decided, frozen unit-7
        reader) and apply_amended() (FINAL_AMENDED: since unit 8 round 2 with R1 + R2, the function's defaults),
        plus apply_amended(sides='port') for attribution, and the round-1 rule FINAL_AMENDED_R0
        (agree='designator', confirm='heads') with R0+R1 / R0+R2 alone as attribution rows.
Checks: FINAL here equals the skeptic runs' `guards_fired` on every block (port parity with o1/o2_final.json).
Also:   5416534's counterfactual (o2_skeptic_run.py): CAN 18634900's blocks re-keyed to the CN lot their own titles
        name, under FINAL_AMENDED and FINAL_AMENDED_R0 (the zad.-compound shape the critic flagged).
Output: amended_oos.json.  Usage: python3 -I -B amended_oos.py
"""
import collections, copy, json, math, os, re, sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
U7 = os.path.join(os.path.dirname(HERE), 'u7')
sys.path.insert(0, U7)
import g3_port as G  # noqa: E402

WRONG = ('multi_lot_contract', 'wrong_other')
RIGHT = ('single_right', 'unique_right', 'unique_unverifiable')
# Unit 8 round 2: FINAL_AMENDED (tag 8) now includes R1 + R2. The round-1 rule and the two refinements alone stay
# measured beside it: (tag, row name, agree, confirm).
VARIANTS = (('8r0', 'FINAL_AMENDED_R0', 'designator', 'heads'),
            ('8r0h', 'R0_plus_R1_only', 'head', 'heads'),
            ('8r0s', 'R0_plus_R2_only', 'designator', 'simple'))


def ld(p):
    with open(p, encoding='utf-8') as fh:
        return json.load(fh)


def norm_label(c):
    if c in RIGHT:
        return 'right'
    return {'multi_lot_contract': 'multi'}.get(c, c)


def run_group(key, ids):
    cache = os.path.join(HERE, 'cache_' + key)
    final = {(b['tender_id'], b['can_notice_id'], b['result_key']): b for b in ld(os.path.join(HERE, key + '_final.json'))['blocks']}
    rows, problems = [], []
    for tid in ids:
        t = ld(os.path.join(cache, f'tenders_{tid}.json'))
        notices = {v['caused_by_notice_id']: ld(os.path.join(cache, f"notices_{v['caused_by_notice_id']}_content.json"))
                   for v in t['versions']}
        blocks = G.replay(tid, t, notices)
        by = collections.defaultdict(list)
        for b in blocks:
            b['set'] = 'oos-' + key
            by[b['can_notice_id']].append(b)
        for bl in by.values():
            G.apply_final(bl)
            G.apply_amended(bl)  # FINAL_AMENDED (R1 + R2)
            G.apply_amended(bl, sides='port', tag='8p')
            for tag, _, agree, confirm in VARIANTS:
                G.apply_amended(bl, tag=tag, agree=agree, confirm=confirm)
        for b in blocks:
            if b['lot_568'] is None:
                continue
            k = (tid, b['can_notice_id'], b['result_key'])
            fb = final.get(k)
            if fb is None:
                problems.append({'unlabelled': list(k)})
                continue
            if sorted(fb['guards_fired']) != sorted(b['guards']):
                problems.append({'final_guards_differ': list(k), 'skeptic_run': fb['guards_fired'], 'here': b['guards']})
            b['label'] = norm_label(fb['cls'])
            b['cls'] = fb['cls']
            rows.append(b)
    missing = set(final) - {(b['tender_id'], b['can_notice_id'], b['result_key']) for b in rows}
    if missing:
        problems.append({'labelled_but_not_matched': sorted(map(list, missing))})
    return rows, problems


def stats(rows, lot_key, guard_key, ids):
    right = [b for b in rows if b['label'] == 'right']
    fr = [b for b in right if b[lot_key] is None]
    before = sorted({b['tender_id'] for b in rows if b['label'] in ('multi', 'wrong_other')}, key=ids.index)
    after = sorted({b['tender_id'] for b in rows if b['label'] in ('multi', 'wrong_other') and b[lot_key] is not None},
                   key=ids.index)

    def caught(lbl):
        pool = [b for b in rows if b['label'] == lbl]
        return {'k': sum(1 for b in pool if b[guard_key]), 'n': len(pool)}
    return {
        'matched_blocks': len(rows), 'multi_caught': caught('multi'), 'other_wrong_caught': caught('wrong_other'),
        'false_refusals': {'k': len(fr), 'n': len(right), 'ci95': G.cp(len(fr), len(right)),
                           'ids': [f"{b['tender_id']}/{b['can_notice_id']}/{b['result_key']}" for b in fr]},
        'tenders_with_a_wrong_block_before': {'k': len(before), 'n': len(ids), 'ci95': G.cp(len(before), len(ids)),
                                              'ids': before},
        'tenders_with_a_wrong_block_after': {'k': len(after), 'n': len(ids), 'ci95': G.cp(len(after), len(ids)),
                                             'ids': after},
        'residual_wrong_blocks': [f"{b['tender_id']}/{b['can_notice_id']}/{b['result_key']} ({b['label']})"
                                  for b in rows if b['label'] in ('multi', 'wrong_other') and b[lot_key] is not None],
        'blocks_refused': sum(1 for b in rows if b[guard_key]),
        'refused_by_guard': dict(collections.Counter(g for b in rows for g in b[guard_key])),
        'tenders_touched': sorted({b['tender_id'] for b in rows if b[guard_key]}, key=ids.index),
    }


def counterfactual_5416534(tender, notices):
    """o2_skeptic_run.py's counterfactual under FINAL_AMENDED: every CAN 18634900 block re-keyed to the CN lot its
    own title names (printed := that lot's number), i.e. the same award texts on a CAN printing the CN's numbering."""
    blocks = [b for b in G.replay(5416534, tender, notices) if b['can_notice_id'] == 18634900]
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
        c['set'] = 'cf'
        out.append(c)
    G.apply_final(out)
    G.apply_amended(out)
    for tag, _, agree, confirm in VARIANTS:
        G.apply_amended(out, tag=tag, agree=agree, confirm=confirm)
    return {
        'what': "CAN 18634900 with every block re-keyed to the CN lot its own title names and that lot's number printed",
        'blocks': len(out), 'unmapped': unmapped,
        'FINAL_refused': sum(1 for c in out if c['guards']),
        'FINAL_guards': dict(collections.Counter(g for c in out for g in c['guards'])),
        'FINAL_AMENDED_refused': sum(1 for c in out if c['guards8']),
        'FINAL_AMENDED_guards': dict(collections.Counter(g for c in out for g in c['guards8'])),
        'FINAL_AMENDED_refusals': [f"{c['result_key']} '{c['titles'][0]}' -> {c['lot_568']} {c['guards8']} Dn={c['f8']['Dn']}"
                                   for c in out if c['guards8']],
        'variants_refused': {nm: sum(1 for c in out if c['guards' + tag]) for tag, nm, _, _ in VARIANTS},
        'variants_guards': {nm: dict(collections.Counter(g for c in out for g in c['guards' + tag]))
                            for tag, nm, _, _ in VARIANTS},
    }


def main():
    draw = ld(os.path.join(HERE, 'draw.json'))
    groups = {g['key']: g['ids'] for g in draw['groups']}
    allrows, problems, per_group = [], [], {}
    for key, ids in groups.items():
        rows, prob = run_group(key, ids)
        problems += prob
        allrows += rows
        per_group[key] = {'FINAL': stats(rows, 'lot_u7', 'guards', ids),
                          'FINAL_AMENDED': stats(rows, 'lot_u8', 'guards8', ids),
                          'FINAL_AMENDED_on_port_text_sides': stats(rows, 'lot_u8p', 'guards8p', ids)}
        for tag, nm, _, _ in VARIANTS:
            per_group[key][nm] = stats(rows, 'lot_u' + tag, 'guards' + tag, ids)
    ids_all = groups['o1'] + groups['o2']
    combined = {'FINAL': stats(allrows, 'lot_u7', 'guards', ids_all),
                'FINAL_AMENDED': stats(allrows, 'lot_u8', 'guards8', ids_all),
                'FINAL_AMENDED_on_port_text_sides': stats(allrows, 'lot_u8p', 'guards8p', ids_all)}
    for tag, nm, _, _ in VARIANTS:
        combined[nm] = stats(allrows, 'lot_u' + tag, 'guards' + tag, ids_all)
    changed = []
    for b in allrows:
        if b['lot_u7'] != b['lot_u8']:
            changed.append({'block': f"{b['set']}:{b['tender_id']}/{b['can_notice_id']}/{b['result_key']}",
                            'label': b['label'], 'printed': b['printed'], 'lot_u7': b['lot_u7'], 'lot_u8': b['lot_u8'],
                            'guards_u7': b['guards'], 'guards_u8': b['guards8'], 'why': G.why_changed(b),
                            'title': (b['titles'] or [''])[0][:160]})
    changed_r0 = [{'block': f"{b['set']}:{b['tender_id']}/{b['can_notice_id']}/{b['result_key']}",
                   'label': b['label'], 'printed': b['printed'], 'lot_u8r0': b['lot_u8r0'], 'lot_u8': b['lot_u8'],
                   'guards_u8r0': b['guards8r0'], 'guards_u8': b['guards8'], 'why': G.why_r0_changed(b),
                   'title': (b['titles'] or [''])[0][:160]}
                  for b in allrows if b['lot_u8r0'] != b['lot_u8'] or b['guards8r0'] != b['guards8']]
    port_vs_fold = [f"{b['tender_id']}/{b['can_notice_id']}/{b['result_key']}" for b in allrows if b['lot_u8'] != b['lot_u8p']]
    multi_lang = sum(1 for b in allrows if len({lg for lg, _ in b['titles_lang']}) > 1)
    lang_split = [(f"{b['tender_id']}/{b['result_key']}", b['f8']['Dn']) for b in allrows
                  if len(b['f8']['Dn']) > 1 and len({tuple(v) for v in b['f8']['Dn'].values()}) > 1]

    # pooled with the in-sample uniform draws (s150 + f30) of measurement-u8.json
    pooled = None
    m8p = os.path.join(U7, 'measurement-u8.json')
    if os.path.exists(m8p):
        m8 = ld(m8p)
        uni = m8['amended']['uniform_tenders_with_a_wrong_block']
        k_in, n_in = uni['FINAL_AMENDED']['k'], uni['FINAL_AMENDED']['n']
        k_oos, n_oos = combined['FINAL_AMENDED']['tenders_with_a_wrong_block_after']['k'], len(ids_all)
        k, n = k_in + k_oos, n_in + n_oos
        lo, hi = G.cp(k, n)

        def pmf(i, nn, p):
            return math.comb(nn, i) * p ** i * (1 - p) ** (nn - i)
        bar = {tag: {'p_tender': round(p, 4), 'P(<=2 of 30)': round(sum(pmf(i, 30, p) for i in range(3)), 3),
                     'P(0 of 30)': round(pmf(0, 30, p), 3)}
               for tag, p in (('point', k / n), ('ci95_low', lo), ('ci95_high', hi))}
        fr_in = sum(m8['amended']['per_set'][s]['FINAL_AMENDED']['false_refusals']['k'] for s in ('s150', 'f30', 'm77'))
        nr_in = sum(m8['amended']['per_set'][s]['FINAL_AMENDED']['false_refusals']['n'] for s in ('s150', 'f30', 'm77'))
        fr_o, nr_o = combined['FINAL_AMENDED']['false_refusals']['k'], combined['FINAL_AMENDED']['false_refusals']['n']
        pooled = {
            'uniform_tenders_with_a_wrong_block_after_FINAL_AMENDED': {
                'in_sample': [k_in, n_in], 'oos': [k_oos, n_oos], 'k': k, 'n': n, 'ci95': [lo, hi]},
            'fresh30_bar_ii_P(pass)': bar,
            'false_refusals_FINAL_AMENDED': {'in_sample': [fr_in, nr_in], 'oos': [fr_o, nr_o], 'k': fr_in + fr_o,
                                             'n': nr_in + nr_o, 'ci95': G.cp(fr_in + fr_o, nr_in + nr_o)},
        }
    t5416534 = ld(os.path.join(HERE, 'cache_o2', 'tenders_5416534.json'))
    n5416534 = {v['caused_by_notice_id']: ld(os.path.join(HERE, 'cache_o2', f"notices_{v['caused_by_notice_id']}_content.json"))
                for v in t5416534['versions']}
    out = {
        'issue': 568, 'unit': '8 (amended rule on the OOS 50)',
        'driver': 'amended_oos.py (python3 -I -B; imports ../u7/g3_port.py: replay -> apply_final / apply_amended)',
        'labels': 'skeptic-final: o1_final.json / o2_final.json `cls`',
        'consistency_problems': problems,
        'per_group': per_group, 'combined': combined,
        'changed_outcomes': changed,
        'changed_vs_FINAL_AMENDED_R0': changed_r0,
        'fold_vs_port_text_sides_differ': port_vs_fold,
        'blocks_with_titles_in_2+_languages': multi_lang, 'blocks_whose_languages_name_different_Dn': lang_split,
        'counterfactual_5416534': counterfactual_5416534(t5416534, n5416534),
        'pooled_with_in_sample': pooled,
    }
    with open(os.path.join(HERE, 'amended_oos.json'), 'w', encoding='utf-8') as fh:
        json.dump(out, fh, ensure_ascii=False, indent=1)
        fh.write('\n')
    brief = {k: out[k] for k in ('consistency_problems', 'fold_vs_port_text_sides_differ', 'blocks_with_titles_in_2+_languages',
                                 'blocks_whose_languages_name_different_Dn', 'pooled_with_in_sample')}
    print(json.dumps(brief, ensure_ascii=False, indent=1))
    for rule in ('FINAL', 'FINAL_AMENDED') + tuple(nm for _, nm, _, _ in VARIANTS):
        c = combined[rule]
        print(rule, 'multi', c['multi_caught'], 'other', c['other_wrong_caught'], 'FR', c['false_refusals']['k'], '/',
              c['false_refusals']['n'], c['false_refusals']['ci95'], 'T before', c['tenders_with_a_wrong_block_before']['k'],
              'after', c['tenders_with_a_wrong_block_after']['k'], 'residual', c['residual_wrong_blocks'],
              'by guard', c['refused_by_guard'])
    for c in changed:
        print('CHANGED', c['block'], c['label'], c['lot_u7'], '->', c['lot_u8'], '|', c['why'])
    for c in changed_r0:
        print('VS_R0', c['block'], c['label'], c['lot_u8r0'], '->', c['lot_u8'], '|', c['why'])
    print(json.dumps(out['counterfactual_5416534'], ensure_ascii=False, indent=1)[:2500])


if __name__ == '__main__':
    main()
