# Reproduce u1-results §6 org counts for design A, then compute design B.
import json, sys, collections
sys.path.insert(0, sys.argv[2])
import gate_b as G
rows = G.rows
by = collections.defaultdict(list)
for r in rows: by[r['code']].append(r)
def model(admit):   # admit(r) -> True if r's coded mentions move to the identity org
    today = after = u7 = 0; per = collections.Counter(); share = []
    for code, rs in by.items():
        reg = [r for r in rs if r['side'] == 'registry' and r['final'] == 'SAME']
        pp = [r for r in rs if r['side'] == 'ppon' and r['final'] == 'SAME']
        t = len(reg) + len(pp)
        mints = code in G.table   # typed names never mint
        moved = [r for r in rs if r['side'] == 'registry' and mints and admit(r)]
        ident = 1 if any(r['coded'] for r in moved) else 0
        movedids = {id(r) for r in moved}
        kept = [r for r in reg if not (id(r) in movedids and r['uncoded'] == 0 and r['other'] == 0)]
        a = ident + len(kept) + len(pp)
        b = ident + len(kept) + (0 if (ident and pp) else len(pp)) if len(pp) <= 1 else ident + len(kept) + len(pp)
        today += t; after += a; u7 += b
        per['unchanged' if b == t else ('better' if b < t else 'worse')] += 1
    return today, after, u7, dict(per)
print('design A (name-blind):', model(lambda r: True))
cfg = G.CFGS['G3 +allowlist +O/0 fold (DESIGN B)']
def admit_b(r):
    tc, _ = G.resolve_tail(r['code'], True)
    return tc is not None and G.gate(r['name'], tc, cfg)[0]
print('design B (gated):', model(admit_b))
