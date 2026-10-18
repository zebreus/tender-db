"""fresh568 labelling view. NO guard is computed or shown, and no served lot of an award block is shown:
the block's lot is the PRE-GUARD combined replay (570 remap + 568 match, guards off). Label first, rule second.
Usage: python3 -I view.py CACHE_DIR tid [tid...] [--long]"""
import json, os, sys
sys.dont_write_bytecode = True
RB = '/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8'
sys.path.insert(0, RB)
import combined_port as C  # noqa
P = C.P
args = sys.argv[1:]
LONG = '--long' in args
args = [a for a in args if a != '--long']
CACHE = args[0]


def ld(name):
    p = os.path.join(CACHE, name)
    return json.load(open(p, encoding='utf-8')) if os.path.exists(p) else None


def short(s, n):
    s = (s or '').replace('\n', ' ')
    if LONG:
        n *= 4
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
    chain = C.chain_of(t)
    contents = {n: ld(f'notices_{n}_content.json') for n in chain}
    contents = {n: c for n, c in contents.items() if c is not None}
    maps = C.remap_chain(chain, contents, True)
    pre = C.expected(t, contents, True, False)
    print('=' * 120)
    print(f"TENDER {tid} pub={t.get('published_at')} {short(t.get('title'), 160)}  chain={chain}  value={t.get('value')}")
    for l in t.get('lot_details', []):
        print(f"   lot {l['lot_key']:8s} no={l.get('published_lot_no')!r} {short(l.get('title'), 110)}  value={l.get('value')}")
    for n in chain:
        c = contents.get(n)
        if c is None:
            print('   MISSING notice', n); continue
        secs = P.sections_of(c)
        proc = secs.get('PROCEDURE', {})
        td = [v.get('code') for v in vals(proc, ('TED-TD_DOCUMENT_TYPE',))]
        form = [v.get('code') for v in vals(proc, ('TED-FORM',))]
        fref = [v.get('value') for v in vals(proc, ('TED-FILE_REFERENCE_NUMBER',))]
        ojs = [v.get('value') for v in vals(proc, ('TED-NO_DOC_OJS',))]
        refs = P.award_refs(secs)
        lsecs = [s for s in secs.values() if s.get('kind') in P.LOT_KINDS]
        target, how = maps[n]
        moved = {k: v for k, v in target.items() if k != v}
        print(f"-- notice {n} {ojs} TD={td} FORM={form} file={fref} lot_sections={len(lsecs)} results={len(refs)} remap={how} {moved if moved else ''}")
        for line in texts(proc, ('TED-TITLE_CONTRACT', 'TED-TITLE', 'TED-CONTRACT_TITLE'), 140, 2):
            print('     proc', line)
        for s in sorted(lsecs, key=lambda s: P.res_order(s['section_id']) if '-' in s['section_id'] else 0):
            print(f"     {s['kind']} {s['section_id']}->{target.get(s['section_id'], s['section_id'])} no={P.published_number(s)!r} {ids(s, ('TED-LOT_NO', 'TED-LOT_NUMBER'))}")
            for line in texts(s, ('TED-LOT_TITLE', 'TED-TITLE'), 150, 2):
                print('         ', line)
            for line in texts(s, ('TED-LOT_DESCRIPTION', 'TED-SHORT_DESCR', 'TED-SHORT_CONTRACT_DESCRIPTION'), 220, 2):
                print('         ', line)
            a = amts(s)
            if a:
                print('          $', '; '.join(a))
        if not refs:
            continue
        for res in sorted(refs, key=P.res_order):
            s = secs[res]
            row = pre.get((n, res), {})
            win = [v.get('value') for x in secs.values() if x.get('parent_section_id') == res and x['kind'] == 'Organization'
                   for v in vals(x, ('TED-OFFICIALNAME',), ('text',))]
            print(f"   {res:7s} printed={P.award_block_lot_number(refs[res])!r} refs={refs[res]} "
                  f"{ids(s, ('TED-CONTRACT_NUMBER', 'TED-CONTRACT_NO'))} PRE-GUARD lot={row.get('expected_lot')} ({row.get('reason')})")
            for line in texts(s, ('TED-CONTRACT_TITLE', 'TED-TITLE', 'TED-TITLE_CONTRACT'), 160, 3):
                print('         ', line)
            if LONG:
                for line in texts(s, ('TED-ADDITIONAL_INFORMATION',), 200, 2):
                    print('         ', line)
            a = amts(s)
            print('          $', '; '.join(a), '| winners:', '; '.join(short(w, 50) for w in win))


for a in args[1:]:
    show(int(a))
