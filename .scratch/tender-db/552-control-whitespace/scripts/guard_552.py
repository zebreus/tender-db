import json, re, sys, unicodedata, collections
rows = json.load(open('cw_rows.json'))
def fold(s):
    s = unicodedata.normalize('NFKD', s or '')
    s = ''.join(c for c in s if not unicodedata.combining(c)).casefold()
    s = s.replace('’', '').replace("'", '').replace('`', '')
    return s
def toks(s):
    return [t for t in re.sub(r'[^\w]+', ' ', fold(s)).split() if t]
CW = re.compile(r'[\n\r\t]')
ADDR_LINE = re.compile(r'^(adres|ul\.|sede|tel|nip|regon|p\.?\s?iva|c\.f\.|postfach|d-\d{5}|\d{2}-\d{3}\b|\d{4,5}\s+\w)|\b\w*(stra(ss|ß)e|str\.|allee|platz|weg)\s*\d', re.I)
def lines_of(old):
    return [l.strip() for l in CW.split(old) if l.strip()]
def guard(old, new):
    """None when the elected spelling is servable for a control-whitespace head, else a reason."""
    L = [set(toks(l)) for l in lines_of(old)]
    n = set(toks(new))
    if not n: return 'empty'
    if any(n <= l for l in L): return None            # inside one line (a member, the first line, ...)
    for i in range(len(L)):                            # a contiguous run of whole lines, as a word set
        acc = set()
        for j in range(i, len(L)):
            acc |= L[j]
            if n == acc: return None
    return 'splice' if n <= set().union(*L) else 'foreign'
def cut_address(old):
    out = []
    for l in lines_of(old):
        if out and ADDR_LINE.search(l): break
        out.append(l)
    return ' '.join(' '.join(out).split())
c = collections.Counter()
for i, o in enumerate(rows):
    g = guard(o['old'], o['new'])
    o['guard'] = g
    c[(o['list'], o['rel'] or '-', g or 'serve')] += 1
    if g: print(f"[{i}] {o['list']} {o['org']} {o['rel']} {g}\n   NEW {o['new'][:110]!r}\n   FALLBACK {cut_address(o['old'])[:160]!r}")
for k, v in sorted(c.items()): print(k, v)
