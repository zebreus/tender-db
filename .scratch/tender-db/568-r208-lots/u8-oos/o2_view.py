"""568 u8-oos group o2: the labelling view. No unit-7 guard is computed here (label first, rule second).

Per Tender: the served lots (lot_details) and lot_results, every chain notice's lot sections (number, title,
description, value), and every award block of every award notice with its printed numbers, its own title(s),
its amounts, its winners, the served lot and the 568 replay lot (../cross_notice_port.py steps 1-9 + G1/G2).
Usage: python3 -I o2_view.py [--long] [tender_id ...]   (default: every o2 tender)
"""
import json, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, ROOT)
import cross_notice_port as P  # noqa: E402

CACHE = os.path.join(HERE, 'cache_o2')
LONG = False


def ld(name):
    p = os.path.join(CACHE, name)
    return json.load(open(p, encoding='utf-8')) if os.path.exists(p) else None


def short(s, n):
    s = (s or '').replace('\n', ' ')
    if LONG:
        n = n * 4
    return s if len(s) <= n else s[:n] + '…'


def vals(sec, fields=None, types=None):
    return [v for v in sorted(sec.get('values', []), key=lambda v: (v['field_id'], v.get('ordinal', 0)))
            if (not fields or v['field_id'] in fields) and (not types or v.get('type') in types)]


def amts(sec):
    out = []
    for v in vals(sec, types=('amount',)):
        c = v.get('cents')
        out.append(f"{v['field_id'].replace('TED-', '')}={'?' if c is None else format(c / 100, ',.2f')} {v.get('currency')}")
    return out


def texts(sec, fields, n, maxn=3):
    r = [f"[{v['field_id'].replace('TED-', '')}/{v.get('lang')}] {short(v.get('value'), n)}" for v in vals(sec, fields, ('text',))]
    return r[:maxn] + ([f'(+{len(r) - maxn} more)'] if len(r) > maxn else [])


def ids(sec, fields):
    return {f.replace('TED-', ''): [v.get('value') for v in vals(sec, (f,), ('id',))] for f in fields if vals(sec, (f,), ('id',))}


def show(tid):
    t = ld(f'tenders_{tid}.json')
    if t is None:
        print('MISSING tender', tid)
        return
    chain = [v['caused_by_notice_id'] for v in sorted(t['versions'], key=lambda v: v['seq'])]
    print('=' * 120)
    print(f"TENDER {tid}  {short(t.get('title'), 160)}  chain={chain}  served lots={t.get('lots')}  value={t.get('value')}")
    for l in t.get('lot_details', []):
        print(f"   served {l['lot_key']:8s} {short(l.get('title'), 110)}  value={l.get('value')}")
    served = {(r['notice_id'], r['key']): r for r in t.get('lot_results', [])}
    carried, renumbered = {}, False
    for n in chain:
        c = ld(f'notices_{n}_content.json')
        if c is None:
            print('   MISSING notice', n)
            continue
        secs = P.sections_of(c)
        proc = secs.get('PROCEDURE', {})
        td = [v.get('code') for v in vals(proc, ('TED-TD_DOCUMENT_TYPE',))]
        form = [v.get('code') for v in vals(proc, ('TED-FORM',))]
        fref = [v.get('value') for v in vals(proc, ('TED-FILE_REFERENCE_NUMBER',))]
        ojs = [v.get('value') for v in vals(proc, ('TED-NO_DOC_OJS',))]
        refs = P.award_refs(secs)
        own, ren = P.carry(carried, secs)
        renumbered |= ren
        print(f"-- notice {n} {ojs} TD={td} FORM={form} file={fref} own_lot_sections={len(own)} results={len(refs)} renumbered={renumbered}")
        for line in texts(proc, ('TED-TITLE_CONTRACT', 'TED-TITLE', 'TED-CONTRACT_TITLE'), 140, 2):
            print('     proc', line)
        if own or LONG:
            for line in texts(proc, ('TED-SHORT_CONTRACT_DESCRIPTION', 'TED-SHORT_DESCR'), 200, 3):
                print('     proc', line)
        for s in sorted(own, key=lambda s: P.res_order(s['section_id']) if '-' in s['section_id'] else 0):
            print(f"     {s['kind']} {s['section_id']} no={P.published_number(s)!r} {ids(s, ('TED-LOT_NO', 'TED-LOT_NUMBER'))}")
            for line in texts(s, ('TED-LOT_TITLE', 'TED-TITLE'), 150, 2):
                print('         ', line)
            for line in texts(s, ('TED-LOT_DESCRIPTION', 'TED-SHORT_DESCR', 'TED-SHORT_CONTRACT_DESCRIPTION'), 220, 2):
                print('         ', line)
            a = amts(s)
            if a:
                print('          $', '; '.join(a))
        if not refs:
            continue
        matched = P.match_notice(carried, renumbered, own, refs)
        for res in sorted(refs, key=P.res_order):
            s = secs[res]
            lot, why = matched[res]
            sv = served.get((n, res))
            sl = sv['lot'] if sv else 'ABSENT'
            win = [v.get('value') for x in secs.values() if x.get('parent_section_id') == res and x['kind'] == 'Organization'
                   for v in vals(x, ('TED-OFFICIALNAME',), ('text',))]
            flag = '' if (lot == sl or (sv is None and lot is None)) else '  <<< REPLAY!=SERVED'
            print(f"   {res:7s} printed={P.award_block_lot_number(refs[res])!r} refs={refs[res]} "
                  f"{ids(s, ('TED-CONTRACT_NUMBER', 'TED-CONTRACT_NO'))} served={sl} replay={lot}({why}){flag}")
            for line in texts(s, ('TED-CONTRACT_TITLE', 'TED-TITLE', 'TED-TITLE_CONTRACT'), 160, 3):
                print('         ', line)
            if LONG:
                for line in texts(s, ('TED-ADDITIONAL_INFORMATION',), 200, 2):
                    print('         ', line)
            a = amts(s)
            print('          $', '; '.join(a), '| winners:', '; '.join(short(w, 50) for w in win))


def main():
    global LONG
    args = sys.argv[1:]
    if '--long' in args:
        LONG = True
        args.remove('--long')
    if args:
        tids = [int(a) for a in args]
    else:
        draw = json.load(open(os.path.join(HERE, 'draw.json'), encoding='utf-8'))
        tids = next(g['ids'] for g in draw['groups'] if g['key'] == 'o2')
    for tid in tids:
        show(tid)


if __name__ == '__main__':
    main()
