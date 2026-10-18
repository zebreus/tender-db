"""568 u8-oos group o2: one line per award block, beside the served lot's CN text (labelling aid; no guard).

Usage: python3 -I o2_compact.py <tender_id> [width]
"""
import json, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, ROOT)
import cross_notice_port as P  # noqa: E402

CACHE = os.path.join(HERE, 'cache_o2')


def ld(name):
    p = os.path.join(CACHE, name)
    return json.load(open(p, encoding='utf-8')) if os.path.exists(p) else None


def vals(sec, fields=None, types=None):
    return [v for v in sorted(sec.get('values', []), key=lambda v: (v['field_id'], v.get('ordinal', 0)))
            if (not fields or v['field_id'] in fields) and (not types or v.get('type') in types)]


def first_text(sec, fields):
    t = [v.get('value') or '' for v in vals(sec, fields, ('text',))]
    return ' / '.join(x.replace('\n', ' ') for x in t)


def amt(sec, fields):
    for v in vals(sec, fields, ('amount',)):
        if v.get('cents') is not None:
            return f"{v['cents'] / 100:,.2f}{v.get('currency') or ''}"
    return '-'


def main():
    tid = int(sys.argv[1])
    w = int(sys.argv[2]) if len(sys.argv) > 2 else 70
    t = ld(f'tenders_{tid}.json')
    chain = [v['caused_by_notice_id'] for v in sorted(t['versions'], key=lambda v: v['seq'])]
    served = {(r['notice_id'], r['key']): r for r in t.get('lot_results', [])}
    carried, renumbered, lotsec = {}, False, {}
    for n in chain:
        c = ld(f'notices_{n}_content.json')
        secs = P.sections_of(c)
        own, ren = P.carry(carried, secs)
        renumbered |= ren
        for s in own:
            lotsec[s['section_id']] = s
        refs = P.award_refs(secs)
        if not refs:
            continue
        matched = P.match_notice(carried, renumbered, own, refs)
        print(f'-- CAN {n} own_lots={len(own)} blocks={len(refs)}')
        for res in sorted(refs, key=P.res_order):
            s = secs[res]
            lot, why = matched[res]
            sv = served.get((n, res))
            sl = sv['lot'] if sv else 'ABSENT'
            cno = [v.get('value') for v in vals(s, ('TED-CONTRACT_NUMBER', 'TED-CONTRACT_NO'), ('id',))]
            title = first_text(s, ('TED-CONTRACT_TITLE', 'TED-TITLE', 'TED-TITLE_CONTRACT'))
            ls = lotsec.get(sl) if sl not in (None, 'ABSENT') else None
            ltxt = (first_text(ls, ('TED-LOT_TITLE', 'TED-TITLE')) + ' | ' +
                    first_text(ls, ('TED-LOT_DESCRIPTION', 'TED-SHORT_DESCR'))) if ls else ''
            lval = amt(ls, ('TED-VALUE_COST', 'TED-VAL_ESTIMATED_TOTAL', 'TED-VAL_OBJECT')) if ls else '-'
            win = [v.get('value') for x in secs.values() if x.get('parent_section_id') == res and x['kind'] == 'Organization'
                   for v in vals(x, ('TED-OFFICIALNAME',), ('text',))]
            flag = '' if lot == sl or (sv is None and lot is None) else ' !!'
            print(f"{res:6s} p={P.award_block_lot_number(refs[res])!s:5s} lnv={[v for f, v in refs[res]]} cno={cno} -> {sl}/{lot}{flag} "
                  f"| T: {title[:w]} | L: {ltxt[:w]} | v={amt(s, ('TED-VALUE_COST', 'TED-VAL_TOTAL', 'TED-LOW_VALUE'))} "
                  f"est={amt(s, ('TED-INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT.VALUE_COST',))} lv={lval} w={'; '.join(x[:30] for x in win)}")


if __name__ == '__main__':
    main()
