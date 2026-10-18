"""Independent re-check of fresh570 (e) on fresh reads. Own lot-section parse + own remap (570 decision text, steps
3-5 + K1-K4) cross-checked against combined_port.remap_chain; E1/E2/E3 recomputed; E4 via check_live.check
(the verified parity tool). Usage: python3 -I -B verify570.py CACHE STAGE_CACHE OUT.json id,id,..."""
import json, os, sys, unicodedata, difflib, collections
sys.dont_write_bytecode = True
U8 = '/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8'
sys.path.insert(0, U8)
import combined_port as C
import check_live as CL
R = C.R
FIELDS = ["TED-LOT_NO", "TED-LOT_NUMBER", "BT-137-Lot", "BT-137-Part", "BT-137-LotsGroup", "DE1-ProcurementProjectLot-ID"]


def ld(p):
    return json.load(open(p, encoding='utf-8'))


def my_sections(c):
    out = []
    for s in sorted(c['sections'], key=lambda s: s['section_id']):
        if s['kind'] not in ('Lot', 'LotsGroup', 'Part'):
            continue
        vals = sorted(s['values'], key=lambda v: (v['field_id'], v.get('ordinal', 0)))
        nums = [(FIELDS.index(v['field_id']), v['value']) for v in vals if v['type'] == 'id' and v['field_id'] in FIELDS]
        num = min(nums, key=lambda x: x[0])[1] if nums else None
        titles = [v['value'] for v in vals if v['type'] == 'text' and v['field_id'] == 'TED-TITLE']
        out.append({'key': s['section_id'], 'kind': s['kind'], 'num': num, 'title': titles[0] if titles else None})
    return out


def legacy(c):
    return any(v['field_id'].startswith(('TED-', 'TXT-')) for s in c['sections'] for v in s['values'])


def norm(p):
    return R.norm(p)  # normalise_lot_number port (verified, shared with 567/568)


def lookup(carried, p):
    exact = collections.defaultdict(list); nd = collections.defaultdict(list)
    for k, l in carried.items():
        if l['kind'] != 'Lot':
            continue
        n = (l['num'] or '').strip()
        if not n:
            continue
        exact[n].append(k)
        if norm(n) is not None:
            nd[norm(n)].append(k)
    p = p.strip()
    if p in exact:
        return exact[p][0] if len(exact[p]) == 1 else None
    n = norm(p)
    if n is None or n not in nd:
        return None
    return nd[n][0] if len(nd[n]) == 1 else None


def my_remap(secs, carried, is_leg, nonleg_seen):
    own = {s['key']: s['key'] for s in secs}
    if not secs:
        return own, 'none'
    if not is_leg:
        return own, 'non-legacy'
    if nonleg_seen:
        return own, 'K3'
    if not carried:
        return own, 'anchor'
    if not any(l['kind'] == 'Lot' and (l['num'] or '').strip() for l in carried.values()):
        return own, 'K4'
    tg, matched = {}, {}
    for s in secs:
        p = (s['num'] or '').strip()
        t = lookup(carried, p) if p and s['kind'] == 'Lot' else None
        tg[s['key']] = t if t is not None else s['key']
        matched[s['key']] = t is not None
    if len(set(tg.values())) != len(tg):
        return own, 'K1'
    for s in secs:
        p = (s['num'] or '').strip()
        if matched[s['key']] or not p:
            continue
        c = carried.get(s['key'])
        if c and (c['num'] or '').strip() and norm(c['num']) != norm(p):
            return own, 'K2'
    return tg, ('identity' if all(k == v for k, v in tg.items()) else 'remapped')


def replay(chain, contents, remap):
    carried, nonleg, maps, versions = {}, False, {}, []
    for n in chain:
        c = contents[n]
        secs = my_sections(c)
        if remap:
            tg, how = my_remap(secs, carried, legacy(c), nonleg)
        else:
            tg, how = {s['key']: s['key'] for s in secs}, 'off'
        before = {k: dict(v, hist=list(v['hist'])) for k, v in carried.items()}
        nonleg |= (not legacy(c)) and bool(secs)
        for s in secs:
            k = tg[s['key']]
            l = carried.setdefault(k, {'kind': s['kind'], 'num': s['num'], 'title': s['title'], 'hist': []})
            l['kind'], l['num'] = s['kind'], s['num']
            if s['title']:
                l['title'] = s['title']
            l['hist'].append((n, s['key'], s['num'], s['title']))
        maps[n] = (tg, how, before, secs)
        versions.append((n, {k: (v['num'], v['hist'][-1][3]) for k, v in carried.items()}))
    return maps, carried, versions


def sq(t):
    t = unicodedata.normalize('NFKD', (t or '').lower())
    t = ''.join(ch for ch in t if not unicodedata.combining(ch))
    return ' '.join(''.join(ch if ch.isalnum() else ' ' for ch in t).split())


def sim(a, b):
    return difflib.SequenceMatcher(None, sq(a), sq(b)).ratio()


def tverdict(title, tgt, own):
    if not title:
        return 'untitled'
    if not tgt:
        return 'target-untitled'
    if sq(title) == sq(tgt):
        return 'equal'
    if sq(tgt) and (sq(tgt) in sq(title) or sq(title) in sq(tgt)) and not (own and sq(own) in sq(title)):
        return 'contains'
    a, b = sim(title, tgt), sim(title, own) if own else 0.0
    return 'target' if a > b + 0.02 else ('own' if b > a + 0.02 else 'tie')


def check(tid, cache, stage_cache):
    t = ld(os.path.join(cache, f'tenders_{tid}.json'))
    chain = C.chain_of(t)
    contents = {n: ld(os.path.join(cache, f'notices_{n}_content.json')) for n in chain}
    out = {'id': tid, 'chain': len(chain), 'source': t.get('source')}
    # stage read vs fresh read
    st = os.path.join(stage_cache, f'tenders_{tid}.json')
    if os.path.exists(st):
        s = ld(st)
        out['fresh_eq_stage'] = {k: s.get(k) == t.get(k) for k in ('lot_results', 'lot_details', 'versions', 'version')}
        out['notice_contents_eq_stage'] = all(
            not os.path.exists(os.path.join(stage_cache, f'notices_{n}_content.json')) or
            ld(os.path.join(stage_cache, f'notices_{n}_content.json')) == contents[n] for n in chain)
    on, carry_on, ver_on = replay(chain, contents, True)
    off, carry_off, ver_off = replay(chain, contents, False)
    cp = C.remap_chain(chain, contents, True)
    mine = {n: {k: v for k, v in on[n][0].items() if k != v} for n in chain}
    theirs = {n: {k: v for k, v in cp[n][0].items() if k != v} for n in chain}
    out['moved_sections_agree_with_combined_port'] = mine == theirs
    out['how'] = {str(n): on[n][1] for n in chain}
    moved = {n: m for n, m in mine.items() if m}
    out['remap_touched'] = bool(moved)
    out['moved_sections'] = sum(len(m) for m in moved.values())
    anchor = next((n for n in chain if my_sections(contents[n])), None)
    anchor_t = {s['key']: s['title'] for s in my_sections(contents[anchor])} if anchor else {}
    # E2
    e2 = collections.Counter(); e2a = collections.Counter(); e2_rows = []
    for n, m in moved.items():
        tg, how, before, secs = on[n]
        sd = {s['key']: s for s in secs}
        for own, tk in sorted(m.items()):
            s = sd[own]
            ct = before.get(tk, {}).get('title'); co = before.get(own, {}).get('title')
            v = tverdict(s['title'], ct, co)
            va = tverdict(s['title'], anchor_t.get(tk), anchor_t.get(own))
            numok = norm(before[tk]['num']) == norm(s['num'])
            e2[v] += 1; e2a[va] += 1
            e2_rows.append({'notice': n, 'sec': own, 'to': tk, 'num': s['num'], 'carried_num': before[tk]['num'],
                            'num_ok': numok, 'title': s['title'], 'carried_title': ct, 'own_key_carried_title': co,
                            'anchor_at_target': anchor_t.get(tk), 'verdict_carried': v, 'verdict_anchor': va})
    out['E2_carried'] = dict(e2); out['E2_anchor'] = dict(e2a); out['E2_rows'] = e2_rows
    # E3
    same_sets = all(set(a[1]) == set(b[1]) for a, b in zip(ver_on, ver_off))
    served_keys = sorted(l['lot_key'] for l in t['lot_details'])
    served_title = {l['lot_key']: l.get('title') for l in t['lot_details']}
    out['E3'] = {'same_keys_every_version': same_sets, 'served_eq_head_on': served_keys == sorted(carry_on),
                 'n_served': len(served_keys),
                 'served_title_eq_on_latest': sum((served_title[k] or None) == (carry_on[k]['hist'][-1][3] or None) for k in served_keys if k in carry_on),
                 'served_title_ne_on_latest': [k for k in served_keys if k in carry_on and (served_title[k] or None) != (carry_on[k]['hist'][-1][3] or None)]}
    # per-key number churn across versions
    def churn(vers):
        prev, n = {}, 0
        for _, st_ in vers:
            for k, (num, _) in st_.items():
                if k in prev and norm(prev[k] or '') != norm(num or ''):
                    n += 1
            prev = {k: v[0] for k, v in st_.items()}
        return n
    out['key_number_changes'] = {'on': churn(ver_on), 'off': churn(ver_off)}
    # E1: blocks in remapped notices
    e1 = []
    for r in t['lot_results']:
        n = r['notice_id']
        if n not in moved:
            continue
        sm = R.same_notice_map(contents[n]).get(r['key'])
        own_sec, printed = sm if sm else (None, None)
        if own_sec is None:
            e1.append({'block': f"{n}/{r['key']}", 'served': r['lot'], 'own_section': None, 'note': 'no own section'})
            continue
        exp = on[n][0].get(own_sec, own_sec)
        base = own_sec
        if exp == base:
            continue
        sec_num = next(s['num'] for s in on[n][3] if s['key'] == own_sec)
        own_no = printed if (printed or '').strip() else sec_num
        head_no = carry_on.get(r['lot'], {}).get('num') if r['lot'] else None
        e1.append({'block': f"{n}/{r['key']}", 'served': r['lot'], 'expected': exp, 'base': base, 'own_no': own_no,
                   'head_no': head_no, 'ok': r['lot'] == exp and head_no is not None and norm(head_no) == norm(own_no)})
    out['E1'] = {'moved_blocks': len([x for x in e1 if 'ok' in x]), 'ok': sum(x.get('ok', False) for x in e1),
                 'bad': [x for x in e1 if not x.get('ok')]}
    # E4 parity (check_live.check: combined_port expected vs served)
    pairs, extra, rows = CL.check(t, contents)
    breaks = [f'{k}: served {s} expected {r["expected_lot"]}' for s, r, k in pairs if r is not None and s != r['expected_lot']]
    outside = [k for s, r, k in pairs if r is None]
    out['E4'] = {'served': len(pairs), 'breaks': breaks, 'outside': outside, 'computed_not_served': [list(k) for k in extra],
                 'causes': dict(collections.Counter(r['cause'] for r in rows.values()))}
    prof_p = os.path.join(cache, f'notices_tender_{tid}.json')
    if os.path.exists(prof_p):
        prof = {int(x['id']): x.get('profile') for x in ld(prof_p).get('items', [])}
        out['profiles'] = dict(collections.Counter(prof.get(n) for n in chain))
        out['legacy_profile_vs_prefix_disagree'] = [n for n in chain if CL.is_legacy_profile(prof.get(n) or '') != legacy(contents[n])]
    return out, (on, off, ver_on, ver_off, contents, chain, t)


if __name__ == '__main__':
    cache, stage, outp, ids = sys.argv[1], sys.argv[2], sys.argv[3], [int(x) for x in sys.argv[4].split(',')]
    res = []
    for tid in ids:
        o, _ = check(tid, cache, stage)
        res.append(o)
        print(tid, 'touched', o['remap_touched'], 'agreeCP', o['moved_sections_agree_with_combined_port'], 'msec', o['moved_sections'],
              'E1', o['E1']['ok'], '/', o['E1']['moved_blocks'], 'bad', len(o['E1']['bad']), 'E2c', o['E2_carried'], 'E2a', o['E2_anchor'],
              'E3', o['E3']['same_keys_every_version'], o['E3']['served_eq_head_on'], o['E3']['served_title_eq_on_latest'], '/', o['E3']['n_served'],
              'E4 served', o['E4']['served'], 'breaks', len(o['E4']['breaks']), 'outside', len(o['E4']['outside']), 'cns', len(o['E4']['computed_not_served']),
              'fresh=stage', o.get('fresh_eq_stage'), o.get('notice_contents_eq_stage'), 'churn', o['key_number_changes'], 'prof', o.get('profiles'), o.get('legacy_profile_vs_prefix_disagree'), flush=True)
    json.dump(res, open(outp, 'w'), indent=1, ensure_ascii=False, default=str)
