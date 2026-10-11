import json, os, sys, collections, re
U1 = sys.argv[1]
HERE = os.path.dirname(os.path.abspath(__file__))
al = json.load(open(f'{HERE}/allow.json'))
A = set(al['A']) | {'DQF', 'TAD', 'TAJ'}   # measured care trusts + IS provider (per-code ORD records)
three = al['three']
ods = al['ods']
mc = collections.defaultdict(dict)
for line in open(f'{U1}/mention-counts-by-org.jsonl'):
    d = json.loads(line)
    for org, raw, n in d['rows']:
        mc[org][raw] = mc[org].get(raw, 0) + n
print('orgs in mention-counts:', len(mc), 'mentions', sum(sum(v.values()) for v in mc.values()))
rows = json.load(open(f'{U1}/adjudication-rows.json'))
reg = [r for r in rows if r['side'] == 'registry']
org_codes = collections.defaultdict(set)
for r in rows: org_codes[r['org_id']].add((r['code'], r['final'], r['side']))
print('adjudicated orgs (both sides):', len(org_codes), 'in mc:', len(set(org_codes) & set(mc)))
def norm(raw):
    return ''.join(c.upper() for c in raw if c.isascii() and c.isalnum())
def family(c):
    ds = [ch for ch in c if ch.isdigit()]
    return (not ds) or all(ch == '0' for ch in ds)
def variants(c):
    out = set()
    idx = [i for i,ch in enumerate(c) if ch in 'O0']
    for mask in range(1, 1 << len(idx)):
        s = list(c)
        for k,i in enumerate(idx):
            if mask >> k & 1: s[i] = '0' if c[i]=='O' else 'O'
        out.add(''.join(s))
    return out
def arm(tail):
    if tail in A: return ('exact', tail)
    hits = [v for v in variants(tail) if v in A]
    if len(hits) == 1: return ('fold', hits[0])
    return (None, None)
tails = collections.defaultdict(lambda: collections.Counter())
bare = collections.Counter(); other_nhs = collections.Counter()
for org, d in mc.items():
    for raw, n in d.items():
        if raw is None: continue
        v = norm(raw)
        if v.startswith('GBNHS'):
            t = v[5:]
            if len(t) == 3:
                tails[t][org] += n
            else:
                other_nhs[v] += n
        elif len(v) == 3 and v.isalpha():
            bare[v] += n
fam_tails = {t: c for t, c in tails.items() if family(t)}
dig_tails = {t: c for t, c in tails.items() if not family(t)}
print('distinct GBNHS+3 tails:', len(tails), 'digit-rule family:', len(fam_tails), 'digit-bearing (mint today):', len(dig_tails))
listed = {r['code'] for r in rows}
out = []
for t, c in sorted(fam_tails.items()):
    kind, target = arm(t)
    holders = sorted(c.items())
    cls = sorted({(cd, f) for o in c for (cd, f, s) in org_codes.get(o, ()) if s == 'registry'})
    ods3 = three.get(t) or ([('ods', ods[t]['status'], ods[t]['rc'], ods[t]['name'])] if t in ods else None)
    out.append((t, kind, target, sum(c.values()), len(c), t in listed, ods3[0][:2] if ods3 else '404/none', ods3[0][3] if ods3 else ''))
mint = [o for o in out if o[1]]
nomint = [o for o in out if not o[1]]
print('\nfamily tails that the allowlist arm MINTS:', len(mint), 'mentions', sum(o[3] for o in mint))
print('  of which not among the 129 listed codes:', [(o[0], o[1], o[2], o[3], o[6], o[7]) for o in mint if not o[5]])
print('  fold hits:', [(o[0], o[2], o[3]) for o in mint if o[1]=='fold'])
print('\nfamily tails REFUSED by the allowlist:', len(nomint), 'mentions', sum(o[3] for o in nomint))
for o in nomint: print('  ', o[0], 'mentions', o[3], 'orgs', o[4], o[6], o[7])
print('\nbare 3-letter raw values (no GB-NHS prefix; length<4, never mint):', sum(bare.values()), bare.most_common(12))
print('other GBNHS values (not 3-char tail):', len(other_nhs), sum(other_nhs.values()), other_nhs.most_common(8))
# shape-rule (withdrawn D1) would mint: every family tail minus denylist
deny = set('TBC TBA TBD NHS NIL NON UNK XXX ZZZ AAA'.split())
shape = [o for o in out if o[0] not in deny and not (o[0][0]==o[0][1]==o[0][2])]
print('\nwithdrawn shape rule would mint', len(shape), 'tails; junk among them (not allowlisted, mints under shape):', [o[0] for o in shape if not o[1]])
json.dump({'tails': {t: dict(c) for t, c in tails.items()}, 'mc': {str(k): v for k, v in mc.items()}}, open(f'{HERE}/tails.json', 'w'))
