"""guard_sim.py -- offline model of the 584 name-agreement guard over org 49's 30,567 mention names
(584-u1/org49_mentions.json; no box reads). Prints, per test variant, how many mentions the guard
would REFUSE (no significant shared token with the identifier org's names), and the refused names.
The model reads mention HEAD names only (variants are not in the u1 file); the 5 bilingual
"National Appeals Chamber" mentions carry a POL variant "Krajowa Izba Odwoławcza" (584-u2 S6), so
they pass under any variant-aware test -- reported separately."""
import json, re, sys, unicodedata
from collections import Counter

rows = json.load(open(sys.argv[1]))
ORG_NAMES = ['Krajowa Izba Odwoławcza', 'National Appeals Chamber']  # org 49 head + ENG satellite (S6)
STOP = set('''w we i z ze o oo na do przy pod od dla ul al pl sp spółka spolka s a sa spk sk k j jawna akcyjna
ograniczoną ograniczona odpowiedzialnością odpowiedzialnoscia komandytowa cywilna sc z.o.o the of and for
oddział oddzial siedzibą siedziba'''.split())

def tokens(name):
    s = unicodedata.normalize('NFC', name).replace('\xa0', ' ')
    raw = re.findall(r"[^\W\d_]+", s, re.UNICODE)
    return raw

def sig(name):
    return [t.lower() for t in tokens(name) if t.lower() not in STOP and len(t) >= 3]

def stem_eq(a, b, k=5):
    if a == b:
        return True
    n = min(len(a), len(b))
    if n < 4:
        return False
    p = 0
    while p < n and a[p] == b[p]:
        p += 1
    return p >= min(k, n - 1)

def initialism(name):
    """initial letters of the significant tokens: Krajowa Izba Odwoławcza -> kio"""
    return ''.join(t[0] for t in sig(name))

ORG_SIG = [sig(n) for n in ORG_NAMES]
ORG_INIT = {initialism(n) for n in ORG_NAMES}

def agree_exact(m, org_sigs):
    ms = set(sig(m))
    return any(ms & set(o) for o in org_sigs)

def agree_stem(m, org_sigs):
    ms = sig(m)
    return any(stem_eq(a, b) for o in org_sigs for a in ms for b in o)

def agree_init(m, org_sigs, inits):
    if agree_stem(m, org_sigs):
        return True
    toks = tokens(m)
    return any(t.isupper() and 2 <= len(t) <= 6 and t.lower() in inits for t in toks)

# vocabulary: name families (stemmed sig-token sets) carried by >= 1% of the org's mentions
names = Counter(r[3] for r in rows)
total = sum(names.values())
def fam(n):
    return tuple(sorted({t[:5] for t in sig(n)}))
famc = Counter()
for n, c in names.items():
    famc[fam(n)] += c
vocab = [list(f) for f, c in famc.items() if c >= 0.01 * total and f]
VOCAB_SIGS = ORG_SIG + vocab
VOCAB_INITS = ORG_INIT | {''.join(t[0] for t in f) for f in vocab}

tests = {
    'A exact token, head+satellites': lambda m: agree_exact(m, ORG_SIG),
    'B stem(5), head+satellites': lambda m: agree_stem(m, ORG_SIG),
    'C stem(5)+initialism, head+satellites': lambda m: agree_init(m, ORG_SIG, ORG_INIT),
    'D C + vocabulary (families >= 1% of mentions)': lambda m: agree_init(m, VOCAB_SIGS, VOCAB_INITS),
}
BILINGUAL = {'National Appeals Chamber'}
print(f'org 49 mentions: {total}, distinct names: {len(names)}')
print('vocabulary families >= 1%:', [(f, famc[tuple(f)]) for f in vocab])
for label, f in tests.items():
    refused = Counter({n: c for n, c in names.items() if not f(n)})
    print(f'\n== {label}: refuses {sum(refused.values())} mentions / {len(refused)} names')
    for n, c in refused.most_common():
        print(f'   {c:6d}  {n!r}{"  [bilingual: POL variant agrees]" if n in BILINGUAL else ""}')
