"""568 u8-oos group o1: the labelling view (no guard is computed here; label first, rule second).

For each Tender: the served lots and lot_results, every chain notice's lot sections (number, title,
description, value), and every award block of every award notice with its printed numbers, its own
title(s), its amounts, its winners, the served lot and the 568 replay lot (../cross_notice_port.py
steps 1-9 + G1/G2 -- not the unit-7 guards).
Usage: python3 -I o1_view.py [tender_id ...]   (default: every o1 tender)
"""
import json, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, ROOT)
import cross_notice_port as P  # noqa: E402

CACHE = os.path.join(HERE, 'cache_o1')


def ld(name):
    p = os.path.join(CACHE, name)
    return json.load(open(p, encoding='utf-8')) if os.path.exists(p) else None


def amt(v):
    c = v.get('cents')
    return f"{v['field_id'].replace('TED-', '')}={c / 100 if c is not None else None:,} {v.get('currency')}" if c is not None \
        else f"{v['field_id'].replace('TED-', '')}=?"


def vals(sec, fields=None, types=None, lang_first=None):
    out = []
    for v in sorted(sec.get('values', []), key=lambda v: (v['field_id'], v.get('ordinal', 0))):
        if fields and v['field_id'] not in fields:
            continue
        if types and v.get('type') not in types:
            continue
        out.append(v)
    return out


def short(s, n):
    s = (s or '').replace('\n', ' ')
    return s if len(s) <= n else s[:n] + '…'


def texts(sec, fields, n=200, langs=None):
    r = []
    for v in vals(sec, fields, ('text',)):
        if langs and v.get('lang') not in langs:
            continue
        r.append(f"[{v['field_id'].replace('TED-', '')}/{v.get('lang')}] {short(v.get('value'), n)}")
    return r


def show(tid, full=False):
    t = ld(f'tenders_{tid}.json')
    if t is None:
        print('MISSING tender', tid)
        return
    chain = [v['caused_by_notice_id'] for v in sorted(t['versions'], key=lambda v: v['seq'])]
    print('=' * 110)
    print(f"TENDER {tid}  {short(t.get('title'), 150)}  versions={chain}  lots={t.get('lots')}")
    for l in t.get('lot_details', []):
        print(f"   served lot {l['lot_key']}: {short(l.get('title'), 120)}  value={l.get('value')}")
    served = {}
    for r in t.get('lot_results', []):
        served[(r['notice_id'], r['key'])] = r
    carried, renumbered, lotmeta = {}, False, {}
    for n in chain:
        c = ld(f'notices_{n}_content.json')
        if c is None:
            print('   MISSING notice', n)
            continue
        secs = P.sections_of(c)
        proc = secs.get('PROCEDURE', {})
        dt = [v.get('code') for v in vals(proc, ('TED-TD_DOCUMENT_TYPE',))]
        form = [v.get('code') for v in vals(proc, ('TED-FORM',))]
        fref = [v.get('value') for v in vals(proc, ('TED-FILE_REFERENCE_NUMBER',))]
        refs = P.award_refs(secs)
        own, ren = P.carry(carried, secs)
        renumbered |= ren
        print(f"-- notice {n} doctype={dt} form={form} fileref={[short(x, 60) for x in fref]} own_lot_sections={len(own)}"
              f" result_blocks={len(refs)} renumbered_so_far={renumbered}")
        ptitle = texts(proc, ('TED-TITLE_CONTRACT', 'TED-TITLE', 'TED-CONTRACT_TITLE'), 160)
        if ptitle:
            print('     proc title:', ptitle[0] if not full else ptitle)
        for v in vals(proc, None, ('amount',)):
            print('     proc amount', amt(v))
        for s in sorted(own, key=lambda s: P.res_order(s['section_id']) if '-' in s['section_id'] else 0):
            num = P.published_number(s)
            tt = texts(s, ('TED-LOT_TITLE', 'TED-TITLE'), 160)
            dd = texts(s, ('TED-LOT_DESCRIPTION', 'TED-SHORT_DESCR', 'TED-SHORT_CONTRACT_DESCRIPTION'), 220 if not full else 2000)
            aa = [amt(v) for v in vals(s, None, ('amount',))]
            lotmeta[s['section_id']] = (num, tt, aa)
            print(f"     LOTSEC {s['section_id']} kind={s['kind']} number={num!r} {tt[:3]} {aa}")
            for d in dd[: (3 if not full else 20)]:
                print('            ', d)
        if not refs:
            continue
        matched = P.match_notice(carried, renumbered, own, refs)
        orgs = {}
        for s in secs.values():
            if s['kind'] == 'Organization':
                orgs.setdefault(s.get('parent_section_id'), []).append(s)
        for res in sorted(refs, key=P.res_order):
            s = secs[res]
            lot, why = matched[res]
            sv = served.get((n, res))
            slot = sv['lot'] if sv else 'NOT-SERVED'
            nums = [(f.replace('TED-', ''), v) for f, v in refs[res]]
            cn = [v.get('value') for v in vals(s, ('TED-CONTRACT_NUMBER', 'TED-CONTRACT_NO'))]
            tt = texts(s, ('TED-CONTRACT_TITLE', 'TED-TITLE', 'TED-TITLE_CONTRACT'), 220 if not full else 2000)
            aa = [amt(v) for v in vals(s, None, ('amount',))]
            win = []
            for o in orgs.get(res, []):
                nm = [v.get('value') for v in vals(o, ('TED-OFFICIALNAME', 'TED-ORGANISATION'))]
                win.append(short(nm[0] if nm else '?', 90))
            flag = ('  <== MATCHED' if not own else '  (own-lot CAN: 567 scope, not 568)') if (slot not in (None, 'NOT-SERVED')) else ''
            agree = '' if (lot == slot or slot == 'NOT-SERVED' or own) else f'  !! replay={lot}({why}) served={slot}'
            print(f"   BLOCK {n}/{res} nums={nums} contract_no={cn} served={slot} replay={lot}({why}){flag}{agree}")
            for x in tt[: (4 if not full else 50)]:
                print('        ', x)
            if aa:
                print('         amounts:', aa)
            if win:
                print('         winners:', win)
            if sv:
                print(f"         served awarded={sv.get('awarded')} winners={[short(w.get('organization_name'), 60) for w in sv.get('winners', [])]}")
            other = [v for v in vals(s) if v['field_id'] not in ('TED-CONTRACT_TITLE', 'TED-TITLE', 'TED-TITLE_CONTRACT', 'TED-LOT_NO', 'TED-LOT_NUMBER', 'TED-ITEM', 'TED-CONTRACT_NUMBER', 'TED-CONTRACT_NO')
                     and v.get('type') == 'text']
            for v in other[: (3 if not full else 20)]:
                print(f"         other [{v['field_id'].replace('TED-', '')}] {short(v.get('value'), 200)}")


if __name__ == '__main__':
    args = sys.argv[1:]
    full = '--full' in args
    args = [a for a in args if a != '--full']
    if args:
        ids = [int(a) for a in args]
    else:
        draw = json.load(open(os.path.join(HERE, 'draw.json'), encoding='utf-8'))
        ids = next(g['ids'] for g in draw['groups'] if g['key'] == 'o1')
    for tid in ids:
        show(tid, full)
