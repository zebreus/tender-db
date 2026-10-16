import json, collections, sys, re
src = open(__import__('os').path.join(__import__('os').path.dirname(__file__), 'guard_552.py')).read().split('c = collections.Counter()')[0]
exec(src)
CONNECT = set('und and de del della dei degli di du des der den of for für fur i z w ze we e ed et y και και του της των & + - – en van voor u a o na do od'.split())
def kept_lines(old):
    out = []
    for l in lines_of(old):
        if out and ADDR_LINE.search(l): break
        out.append(l)
    return out
LEGAL = set('''gmbh ag kg ohg ug mbh mbb co ev eg gbr srl spa sa sas sarl ltd limited plc llp inc corp bv nv oy ab as aps sro spol
zoo sp ooo kft zrt doo dd ad ae epe ike oe sl slu scrl sc scarl scpa coop lda'''.split())
def ends_legal(a):
    last = a.rstrip(' ,;').split()[-1] if a.split() else ''
    t = toks(last)
    return bool(t) and (''.join(t) in LEGAL or all(x in LEGAL for x in t))
def wrap(a, b):
    w = toks(a)
    if ends_legal(a): return None
    raw_last = a.rstrip().split()[-1] if a.split() else ''
    if '.' in raw_last: w = w[:-1] + ['.']
    if a.rstrip().endswith(('-', '–')) and not a.rstrip().endswith((' -', ' –')): return 'hyphen'
    if w and w[-1] in CONNECT and not a.rstrip().endswith((',', ';', ':')): return 'connector'
    if b[:1].islower(): return 'lower'
    return None
d = json.load(open('rejudge2543_result.json'))
v = {}
for g in d['groups']:
    for it in g['items']: v.setdefault(it['org_id'], it['verdict'])
c = collections.Counter()
for i, o in enumerate(rows):
    if len(sys.argv) > 1 and sys.argv[1] == 'withheld' and not (o['rel'] == 'unfit' and guard(o['old'], o['new'])): continue
    k = kept_lines(o['old'])
    ws = [wrap(a, b) for a, b in zip(k, k[1:])]
    allw = bool(ws) and all(ws)
    g = guard(o['old'], o['new']) or 'serve'
    c[(o['list'], o['rel'] or '-', g if g == 'serve' else 'withheld', 'R0w' if allw else '-')] += 1
    if allw:
        print(f"[{i}] {o['org']} {o['list']} {o['rel']} {g} judge={v.get(o['org'],'-')} {ws}\n   R0w {' '.join(' '.join(k).split())[:150]!r}\n   NEW {o['new'][:120]!r}")
for kk, vv in sorted(c.items()): print(kk, vv)
