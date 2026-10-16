"""Issue 552 unit 2, revised after the adversarial check (wf_0744b445-0ee).
Guard on VOTE KEYS (verbatim crosswalk head_vote_key, via the reviewers' Rust binary `vk`), set equality,
control-only population, R0w = whitespace collapse (no cut) on wrap-only heads inside the withheld set."""
import json, re, sys, os, collections
VK = sys.argv[1]            # dir holding rk.py + vk binary
sys.path.insert(0, VK)
from rk import rust, vkey
rows = json.load(open(sys.argv[2]))
judge = json.load(open(sys.argv[3]))
v = {}
for g in judge['groups']:
    for it in g['items']: v.setdefault(it['org_id'], it['verdict'])
# Rust's char::is_control && char::is_whitespace: \t \n \x0b \x0c \r \x85
CW = re.compile('[\t\n\x0b\x0c\r\x85]')
def lines_of(old): return [l.strip() for l in CW.split(old) if l.strip()]
def control_only(old):
    r = rust([old])[0]
    return r['ctrl'] and not r['cctrl'] and not r['mojibake'] and r['clen'] <= 200
def guard(old, new):
    LS = [set(vkey(l).split(' ')) for l in lines_of(old)]
    ek = vkey(new)
    if not ek: return 'empty'
    n = set(ek.split(' '))
    if any(n <= l for l in LS): return None
    for i in range(len(LS)):
        acc = set()
        for j in range(i, len(LS)):
            acc |= LS[j]
            if n == acc: return None
    return 'splice' if n <= set().union(*LS) else 'foreign'
CONNECT = set('und and de del della dei degli di du des der den of for für fur i z w ze we e ed et y και του της των en van voor u na do od a o'.split())
LEGAL_TAIL = re.compile(r'(\b(gmbh|mbh|ag|kg|ohg|ug|e\.?\s?v|e\.?\s?g|gbr|s\.?\s?r\.?\s?l|s\.?\s?p\.?\s?a|s\.?\s?a|s\.?\s?a\.?\s?s|s\.?\s?a\.?\s?r\.?\s?l|ltd|limited|plc|llp|inc|corp|b\.?\s?v|n\.?\s?v|oy|ab|a/?s|aps|s\.?\s?r\.?\s?o|sp\.?\s?z\s?o\.?\s?o|d\.?\s?o\.?\s?o|d\.?\s?d|kft|zrt|a\.?\s?e|s\.?\s?l|s\.?\s?c\.?\s?p\.?\s?a|s\.?\s?c\.?\s?a\.?\s?r\.?\s?l|lda|co\.?\s?kg)\.?)\s*[,;]?$', re.I)
def wrap(a, b):
    a = a.rstrip()
    if LEGAL_TAIL.search(a): return None                       # a name that ends in its legal form is complete
    if a.endswith((',', ';', ':')): return None                # a field separator, not a wrap
    last = a.split()[-1] if a.split() else ''
    if last.endswith(('-', '–')) and len(last) > 1: return 'hyphen'   # `Planungs-` (a spaced ` -` is not)
    if '.' not in last and last.casefold() in CONNECT: return 'connector'
    if b[:1].islower(): return 'lower'
    return None
c = collections.Counter(); r0w = []; pins = {}
for o in rows:
    if o['rel'] != 'unfit': continue
    co = control_only(o['old'])
    g = guard(o['old'], o['new']) if co else 'not-asked'
    L = lines_of(o['old'])
    ws = [wrap(a, b) for a, b in zip(L, L[1:])]
    is_r0w = co and g not in (None, 'not-asked') and bool(ws) and all(ws)
    out = 'not-asked' if not co else ('pass' if g is None else ('R0w' if is_r0w else 'lines'))
    c[(out, v.get(o['org'], 'unjudged'))] += 1
    pins[o['org']] = out
    if is_r0w: r0w.append((o['org'], v.get(o['org']), ' '.join(o['old'].split()), o['new']))
for k, n in sorted(c.items()): print(k, n)
tot = collections.Counter(k[0] for k in c.elements())
print('totals', dict(tot))
for x in r0w: print('R0w', x[0], x[1], repr(x[2][:110]), '<-elected', repr(x[3][:80]))
for org in (22289866, 22661109, 22737299, 14770, 22216826, 22925490, 4040, 22354266, 22650988, 22299001, 22862316, 22545991, 22718921, 22571984, 23294406, 22393047, 22648679):
    print('pin', org, pins.get(org))
