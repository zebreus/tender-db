"""Issue 568 unit 7: the multi-lot / contradicted-number guards (G3-G6) replayed on the cached parsed values.

Replays the deployed 568 rule (`../cross_notice_port.py`: steps 1-9 + G1/G2) on every award notice of
every Tender in three sets, then the unit-7 guards, and joins the judged classes:
  s150 -- the salted 150 of the 156,123 corrected (readback/u7_sample.json; skeptic-final per block:
          skeptic_u*/u*_skeptic_final*.json);
  m77  -- unit 1's 77 measured Tenders (measurement.json + extra strata; readback/measured77.json);
  f30  -- the read-back's fresh 30 (readback/fresh30.json; readback_result.json: 8327340's 7 multi-lot
          blocks wrong, every other block right).
Input: u7-cache.jsonl.gz (every tender's versions and every chain notice's /v1/notices/{id}/content).

Usage: python3 -I g3_port.py [out.json]   (default: measurement-u7.json next to this file)
"""
import collections, gzip, json, math, os, re, sys, unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, ROOT)
sys.path.insert(0, HERE)
import cross_notice_port as P  # noqa: E402
import lot_designators as L  # noqa: E402

nl = P.normalise_lot_number
TITLE_FIELDS = ('TED-CONTRACT_TITLE', 'TED-TITLE', 'TED-TITLE_CONTRACT')
LOT_TEXT_FIELDS = ('TED-LOT_TITLE', 'TED-TITLE', 'TED-LOT_DESCRIPTION', 'TED-SHORT_DESCR', 'TED-SHORT_CONTRACT_DESCRIPTION')
LOT_TITLE_FIELDS = ('TED-LOT_TITLE', 'TED-TITLE')
PROC_TITLE_FIELDS = ('TED-TITLE_CONTRACT', 'TED-TITLE', 'TED-CONTRACT_TITLE')


# ---------------------------------------------------------------- inputs
def load_bundle():
    tenders, notices, sets = {}, {}, collections.defaultdict(list)
    with gzip.open(os.path.join(HERE, 'u7-cache.jsonl.gz'), 'rt', encoding='utf-8') as fh:
        for line in fh:
            r = json.loads(line)
            if r['kind'] == 'tender':
                tenders[r['id']] = r['data']
                sets[r['set']].append(r['id'])
            else:
                notices[r['id']] = r['data']
    return tenders, notices, sets


def load_labels():
    lab = {}
    files = ['skeptic_u1/u1_skeptic_final.short.json', 'skeptic_u2/u2_skeptic_final.json',
             'skeptic_u3/u3_skeptic_final.min.json', 'skeptic_u4/u4_skeptic_final.min.json',
             'skeptic_u5/u5_skeptic_final.min.json', 'skeptic_u6/u6_skeptic_final.short.json']
    for f in files:
        for b in json.load(open(os.path.join(HERE, f), encoding='utf-8'))['blocks']:
            lab[('s150', b['tender_id'], b['can_notice_id'], b['result_key'])] = b['cls']
    m = json.load(open(os.path.join(ROOT, 'measurement.json'), encoding='utf-8'))
    for t in m['tenders']:
        for b in t['blocks']:
            lab[('m77', t['tender_id'], t['can_notice_id'], b['result_key'])] = b['cls']
    for b in m['extra_strata_judge_read']['blocks']:
        lab[('m77', b['tender_id'], b['notice_id'], b['result_key'])] = b.get('cls_with_G1') or b['cls_without_guards']
    return lab


F30_MULTI = {(17821481, 'RES-1'), (17821481, 'RES-2'), (17821481, 'RES-5'), (17821481, 'RES-6'),
             (17821481, 'RES-7'), (17830774, 'RES-1'), (17830774, 'RES-2')}
ONE_LOT_PROCEDURE = {4514627}  # the Tender's only lot is the whole 182-lot procedure (skeptic note)


def label(lab, b):
    if b['set'] == 'f30':
        return 'multi' if (b['can_notice_id'], b['result_key']) in F30_MULTI else 'right'
    c = lab.get((b['set'], b['tender_id'], b['can_notice_id'], b['result_key']))
    if c is None:
        return 'unjudged'
    if c in ('single_right', 'unique_right', 'unique_unverifiable'):
        return 'right'
    if c == 'multi_lot_contract':
        return 'multi_one_lot_tender' if b['tender_id'] in ONE_LOT_PROCEDURE else 'multi'
    return c


# ---------------------------------------------------------------- replay
def texts_of(section, fields):
    return [v.get('value') or '' for v in sorted(section.get('values', []), key=lambda v: (v['field_id'], v.get('ordinal', 0)))
            if v.get('type') == 'text' and v['field_id'] in fields]


def first_amount(section, fields):
    for f in fields:
        for v in sorted(section.get('values', []), key=lambda v: v.get('ordinal', 0)):
            if v['field_id'] == f and v.get('type') == 'amount':
                return (v.get('cents'), v.get('currency'))
    return None


def replay(tid, tender, notices):
    chain = [v['caused_by_notice_id'] for v in sorted(tender['versions'], key=lambda v: v['seq'])]
    carried, lotmeta, renumbered, tender_titles = {}, {}, False, []
    blocks = []
    for n in chain:
        secs = P.sections_of(notices[n])
        refs = P.award_refs(secs)
        own, ren = P.carry(carried, secs)
        renumbered |= ren
        for s in own:
            lotmeta[s['section_id']] = {'title': texts_of(s, LOT_TITLE_FIELDS), 'texts': texts_of(s, LOT_TEXT_FIELDS),
                                        'value': first_amount(s, ('TED-VALUE_COST', 'TED-VAL_ESTIMATED_TOTAL', 'TED-VAL_OBJECT'))}
        proc = secs.get('PROCEDURE')
        if proc is not None:
            t = texts_of(proc, PROC_TITLE_FIELDS)
            if t:  # per-field supersede: the latest notice that publishes a Tender title replaces it
                tender_titles = t
        if not refs:
            continue
        matched = P.match_notice(carried, renumbered, own, refs)
        inherited = {k: {'kind': kd, 'printed': num, **lotmeta.get(k, {'title': [], 'texts': [], 'value': None})}
                     for k, (kd, num) in carried.items()}
        for res in sorted(refs, key=P.res_order):
            s = secs[res]
            lot, why = matched[res]
            field = next((f for f in ('TED-LOT_NO', 'TED-LOT_NUMBER') if any(x == f for x, _ in refs[res])), None)
            blocks.append({
                'tender_id': tid, 'can_notice_id': n, 'result_key': res,
                'printed': P.award_block_lot_number(refs[res]), 'lot_568': lot, 'why_568': why,
                'number_values': [v for f, v in refs[res] if f == field],
                'titles': texts_of(s, TITLE_FIELDS),
                'award': first_amount(s, ('TED-VAL_TOTAL', 'TED-VALUE_COST')),
                'inherited': inherited, 'tender_titles': list(tender_titles),
            })
    return blocks


# ---------------------------------------------------------------- guards
def title_key(s):
    """G5's comparable title: NFKD, accents and case folded, one leading designator dropped
    ('5. Snagging', 'Lot 3: ...'), runs of non-word characters collapsed."""
    if not s:
        return ''
    s = unicodedata.normalize('NFKD', s.lower())
    s = ''.join(ch for ch in s if not unicodedata.combining(ch))
    s = re.sub(r'^\s*(?:(?:lot|lots|los|lote|lotto|pakiet|czesc|zadanie|part|partie|perceel|cast)\s*'
               r'(?:n[or°º]\.?\s*)?)?[0-9ivxlc]{1,4}\s*[.:)\-–—]+\s*', '', s)
    return re.sub(r'[^\w]+', ' ', s).strip()


def features(b):
    lots = {k: v for k, v in b['inherited'].items() if v['kind'] == 'Lot' and v['printed'] and v['printed'].strip()}
    N = {nl(v['printed']) for v in lots.values()} - {None}
    p = nl(b['printed']) if b['printed'] else None
    D = set()
    for t in b['titles']:
        D |= set(L.designators(t))
    Df = {d for d in D if d[0] in N}
    lot = b['inherited'].get(b['lot_568']) if b['lot_568'] else None
    Dlot = set()
    for t in (lot or {}).get('texts', []):
        Dlot |= set(L.designators(t))
    raw = set()
    for t in b['titles']:
        raw |= set(L.raw_numbers(t))
    ratio = None
    if lot and lot.get('value') and b['award'] and lot['value'][0] and b['award'][0] is not None \
            and lot['value'][1] == b['award'][1]:
        ratio = b['award'][0] / lot['value'][0]
    keys = {title_key(t) for t in b['titles']} - {''}
    mine = {title_key(t) for t in (lot or {}).get('title', [])} - {''}
    tender_keys = {title_key(t) for t in b['tender_titles']} - {''}
    others = [{title_key(t) for t in v.get('title', [])} - {''} for k, v in lots.items() if k != b['lot_568']]
    # one language agreeing with the matched lot's own title is agreement (8303947 RES-2: its Maltese
    # variant copies lot 1's, its other 22 languages are lot 2's)
    g5 = not (keys & mine) and any(k not in tender_keys and sum(1 for o in others if k in o) == 1 for k in keys)
    return {
        'N': sorted(N), 'p': p, 'one_lot': len(N) < 2,
        'D': sorted(D), 'Df': sorted(Df),
        'contradicts': bool(Df) and p not in {d[0] for d in Df} and not (Df & Dlot),
        'raw_in_N': sorted(raw & N), 'ratio': ratio,
        'several_numbers': len({nl(v) for v in b['number_values'] if v and v.strip()} - {None}) >= 2,
        'g5': g5,
    }


def apply_final(blocks):
    """Unit 7's rule over one award notice's blocks (all blocks of one CAN of one Tender)."""
    for b in blocks:
        b['f'] = features(b)
    # G4 reads only the blocks the 568 rule matched: a contradiction on a block that found no lot
    # (5218711's CAN-only 5/6/7 for sub-lots 3.B-3.D) says nothing about the numbers that did match.
    g4 = any(b['lot_568'] is not None and not b['f']['one_lot'] and b['f']['contradicts'] for b in blocks)
    for b in blocks:
        f = b['f']
        fired = []
        if b['lot_568'] is not None and not f['one_lot']:
            if g4:
                fired.append('G4')
            if f['several_numbers']:
                fired.append('G6')
            if len(f['Df']) >= 2:
                fired.append('G3')
            if f['g5']:
                fired.append('G5')
        b['guards'] = fired
        b['lot_u7'] = None if fired else b['lot_568']


# ---------------------------------------------------------------- stats
def cp(k, n, a=0.05):
    if n == 0:
        return [None, None]
    def cdf(x, p):
        return sum(math.comb(n, i) * p ** i * (1 - p) ** (n - i) for i in range(0, x + 1))
    def bis(f):
        lo, hi = 0.0, 1.0
        for _ in range(60):
            m = (lo + hi) / 2
            lo, hi = (lo, m) if f(m) else (m, hi)
        return (lo + hi) / 2
    lo = 0.0 if k == 0 else bis(lambda p: 1 - cdf(k - 1, p) >= a / 2)
    hi = 1.0 if k == n else bis(lambda p: cdf(k, p) <= a / 2)
    return [round(lo, 4), round(hi, 4)]


def main():
    out_path = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, 'measurement-u7.json')
    tenders, notices, sets = load_bundle()
    lab = load_labels()
    allb = []
    for name in ('s150', 'm77', 'f30'):
        for tid in sets[name]:
            blocks = replay(tid, tenders[tid], notices)
            by_notice = collections.defaultdict(list)
            for b in blocks:
                b['set'] = name
                by_notice[b['can_notice_id']].append(b)
            for bl in by_notice.values():
                apply_final(bl)
            allb.extend(blocks)
    matched = [b for b in allb if b['lot_568'] is not None]
    for b in matched:
        b['label'] = label(lab, b)

    # candidate guards (block predicates; notice-level ones look at the notice's blocks)
    notice_of = collections.defaultdict(list)
    for b in matched:
        notice_of[(b['set'], b['tender_id'], b['can_notice_id'])].append(b)
    def nk(b):
        return (b['set'], b['tender_id'], b['can_notice_id'])
    def g3a(b): return not b['f']['one_lot'] and len(b['f']['raw_in_N']) >= 2
    def g3c(b): return not b['f']['one_lot'] and len(b['f']['Df']) >= 2
    def val(k): return lambda b: b['f']['ratio'] is not None and b['f']['ratio'] >= k
    def g4blk(b): return not b['f']['one_lot'] and b['f']['contradicts']
    def g4n(b): return any(g4blk(x) for x in notice_of[nk(b)])
    def g5(b): return not b['f']['one_lot'] and b['f']['g5']
    def g5n(b): return any(g5(x) for x in notice_of[nk(b)])
    def g6(b): return not b['f']['one_lot'] and b['f']['several_numbers']
    cands = {
        'G3a: title names >=2 distinct integers that inherited lots print': g3a,
        'G3c: title names >=2 distinct lot designators (lot/part/position phrase, any language) that inherited lots print': g3c,
    }
    for k in (1.5, 1.8, 2.0, 2.5, 3.0):
        cands[f'G3d: award >= {k}x the matched lot\'s own CN value (same currency)'] = val(k)
        cands[f'G3b: G3a or award >= {k}x lot value'] = (lambda k: lambda b: g3a(b) or val(k)(b))(k)
        cands[f'G3c or award >= {k}x lot value'] = (lambda k: lambda b: g3c(b) or val(k)(b))(k)
    cands['G4 (block): a title designator an inherited lot prints, not the printed number nor the matched lot\'s own'] = g4blk
    cands['G4 (notice): any block of the notice contradicts -> none matched'] = g4n
    cands['G5 (block): title is exactly one OTHER inherited lot\'s title (not the matched lot\'s, not the Tender\'s)'] = g5
    cands['G5 (notice)'] = g5n
    cands['G6 (block): the block prints 2+ distinct lot numbers'] = g6
    cands['FINAL = G3c | G4 notice | G5 block | G6'] = lambda b: bool(b['guards'])
    labels = ['multi', 'wrong_other', 'multi_one_lot_tender', 'right']
    table = {}
    for name, g in cands.items():
        row = {}
        for labl in labels:
            for s in ('s150', 'm77', 'f30', 'all'):
                pool = [b for b in matched if b['label'] == labl and (s == 'all' or b['set'] == s)]
                if not pool:
                    continue
                k = sum(1 for b in pool if g(b))
                row[f'{labl}/{s}'] = {'k': k, 'n': len(pool), 'ci95': cp(k, len(pool))}
        hits = collections.defaultdict(list)
        for b in matched:
            if g(b) and b['label'] != 'right' or (g(b) and b['label'] == 'right'):
                hits[b['label']].append(f"{b['set']}:{b['tender_id']}/{b['can_notice_id']}/{b['result_key']}")
        row['hits'] = {k: v for k, v in hits.items()}
        table[name] = row

    # tender level over the uniform draws (s150 + f30)
    def tender_wrong(tid, setname, use_final):
        bl = [b for b in matched if b['tender_id'] == tid and b['set'] == setname]
        return any(b['label'] in ('multi', 'wrong_other') and (not use_final or b['lot_u7'] is not None) for b in bl)
    uni = [(t, s) for s in ('s150', 'f30') for t in sets[s]]
    before = [t for t, s in uni if tender_wrong(t, s, False)]
    after = [t for t, s in uni if tender_wrong(t, s, True)]
    refused_right = [b for b in matched if b['label'] == 'right' and b['lot_u7'] is None]
    s150_touched = sorted({b['tender_id'] for b in matched if b['set'] == 's150' and b['guards']})
    s150_all_refused = sorted(t for t in s150_touched
                              if all(b['lot_u7'] is None for b in matched if b['tender_id'] == t and b['set'] == 's150'))
    residual = [b for b in matched if b['label'] in ('multi', 'wrong_other') and b['lot_u7'] is not None]

    per_block = [{k: b[k] for k in ('set', 'tender_id', 'can_notice_id', 'result_key', 'printed', 'lot_568', 'lot_u7',
                                    'guards', 'label')}
                 | {'Df': b['f']['Df'], 'contradicts': b['f']['contradicts'], 'g5': b['f']['g5'],
                    'several_numbers': b['f']['several_numbers'],
                    'ratio': None if b['f']['ratio'] is None else round(b['f']['ratio'], 4),
                    'title': (b['titles'] or [''])[0][:160]}
                 for b in matched]
    summary = {
        'matched_blocks': collections.Counter(f"{b['set']}/{b['label']}" for b in matched),
        'replay_matches_served': 'see extract check in decision-u7.md (s150 1173/1173 incl. nulls, m77 692/692, f30 212/212)',
        'uniform_tenders': len(uni),
        'tenders_with_a_wrong_block_before': {'k': len(before), 'n': len(uni), 'ci95': cp(len(before), len(uni)), 'ids': before},
        'tenders_with_a_wrong_block_after': {'k': len(after), 'n': len(uni), 'ci95': cp(len(after), len(uni)), 'ids': after},
        'right_blocks_refused': [f"{b['set']}:{b['tender_id']}/{b['can_notice_id']}/{b['result_key']}" for b in refused_right],
        's150_tenders_touched': {'k': len(s150_touched), 'n': 150, 'ci95': cp(len(s150_touched), 150), 'ids': s150_touched},
        's150_tenders_fully_reverted': {'k': len(s150_all_refused), 'n': 150, 'ci95': cp(len(s150_all_refused), 150),
                                        'ids': s150_all_refused},
        's150_blocks_refused': sum(1 for b in matched if b['set'] == 's150' and b['guards']),
        'residual_wrong_blocks': [f"{b['set']}:{b['tender_id']}/{b['can_notice_id']}/{b['result_key']} ({b['label']})"
                                  for b in residual],
        'm77_blocks_refused': sum(1 for b in matched if b['set'] == 'm77' and b['guards']),
        'f30_8327340': {f"{b['can_notice_id']}/{b['result_key']}": [b['lot_568'], b['lot_u7'], b['guards']]
                        for b in matched if b['tender_id'] == 8327340},
    }
    pop = 156123
    def scale(k, n):
        lo, hi = cp(k, n)
        return {'point': round(pop * k / n), 'ci95': [round(pop * lo), round(pop * hi)]}
    def pmf(k, n, p):
        return math.comb(n, k) * p ** k * (1 - p) ** (n - k)
    res_k, res_n = len(after), len(uni)
    bar = {}
    for tag, p in (('point', res_k / res_n), ('ci95_low', cp(res_k, res_n)[0]), ('ci95_high', cp(res_k, res_n)[1])):
        bar[tag] = {'p_tender': round(p, 4), 'P(0 of 30)': round(pmf(0, 30, p), 3),
                    'P(<=2 of 30)': round(sum(pmf(k, 30, p) for k in range(3)), 3)}
    meta = {
        'issue': 568, 'unit': 7,
        'what': 'Refusal guards for the 568 cross-notice match: candidate catches and false refusals over the salted 150 '
                '(skeptic-final), the 77 measured (unit 1) and the read-back fresh 30 (8327340 included); the decided '
                'rule (FINAL) replayed on the cached parsed values (u7-cache.jsonl.gz) by g3_port.py.',
        'labels': {'right': 'single_right / unique_right / unique_unverifiable',
                   'multi': 'one award block for several lots, printing one of them (multi_lot_contract)',
                   'wrong_other': 'on a lot that is not its lot for another reason',
                   'multi_one_lot_tender': '4514627: a 17-lot contract on the Tender whose only lot is the whole '
                                           '182-lot procedure; same scope as the Tender, not counted wrong'},
        'decided_rule': 'G3 | G4(notice) | G5 | G6, none on a Tender whose inherited lots print < 2 distinct numbers; '
                        'see decision-u7.md',
        'scale_to_156123_corrected': {
            'tenders_touched': scale(len(s150_touched), 150),
            'tenders_fully_reverted': scale(len(s150_all_refused), 150),
            'blocks_refused_point': round(pop * summary['s150_blocks_refused'] / 150),
        },
        'fresh30_bar_probabilities_at_measured_residual': bar,
        'unit1_frame_vs_corrected_draws': {
            'm77_cans_with_a_wrong_or_multi_block': '0 of 70 CANs (656 matched blocks)',
            'uniform_draws': f'{len(before)} of {len(uni)} Tenders',
            'P(0 of 70 | rate of the uniform draws)': round((1 - len(before) / len(uni)) ** 70, 4)},
    }
    with open(out_path, 'w', encoding='utf-8') as fh:
        fh.write('{\n"meta": ' + json.dumps(meta, ensure_ascii=False, indent=1))
        fh.write(',\n"summary": ' + json.dumps(summary, ensure_ascii=False, indent=1, default=list))
        fh.write(',\n"candidates": ' + json.dumps(table, ensure_ascii=False, indent=1, default=list))
        fh.write(',\n"blocks": [\n' + ',\n'.join(json.dumps(b, ensure_ascii=False, default=list) for b in per_block) + '\n]\n}\n')
    for name, row in table.items():
        print(f"{name[:100]:100s} multi {row['multi/all']['k']}/{row['multi/all']['n']}  wrong {row['wrong_other/all']['k']}/{row['wrong_other/all']['n']}"
              f"  FR s150 {row['right/s150']['k']} m77 {row['right/m77']['k']} f30 {row['right/f30']['k']} all {row['right/all']['k']}/{row['right/all']['n']} {row['right/all']['ci95']}")
    print(json.dumps({k: v for k, v in summary.items() if k != 'f30_8327340'}, default=list)[:3000])
    print(summary['f30_8327340'])


if __name__ == '__main__':
    main()
