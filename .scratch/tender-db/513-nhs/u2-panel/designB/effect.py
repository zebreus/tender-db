import sys, os, collections
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gate_b as G
for c in list(G.preds):
    G.preds[c] = [p for p in G.preds[c] if p in G.table and G.table[p]['status'] == 'Inactive']
import gate_b3 as G3
rows = G.rows
by = collections.defaultdict(list)
for r in rows: by[r['code']].append(r)
def effect(label, admit):
    today = after = u7 = 0; per = collections.Counter(); moved = collections.Counter(); emptied = collections.Counter(); idents = 0
    for code, rs in by.items():
        reg = [r for r in rs if r['side'] == 'registry']
        same = [r for r in reg if r['final'] == 'SAME']
        pp = [r for r in rs if r['side'] == 'ppon' and r['final'] == 'SAME']
        t = len(same) + len(pp)
        adm = [r for r in reg if code in G.table and admit(r)]
        for r in adm: moved[r['final']] += r['coded']
        ident = 1 if any(r['coded'] for r in adm) else 0
        idents += ident
        admset = {id(r) for r in adm}
        for r in reg:
            if id(r) in admset and r['uncoded'] == 0 and r['other'] == 0: emptied[r['final']] += 1
        kept = [r for r in same if not (id(r) in admset and r['uncoded'] == 0 and r['other'] == 0)]
        a = ident + len(kept) + len(pp)
        b = ident + len(kept) + (0 if (ident and len(pp) == 1) else len(pp))
        today += t; after += a; u7 += b
        per['unchanged' if b == t else ('better' if b < t else 'worse')] += 1
    print(f'{label}: identities {idents}; coded mentions moved {dict(moved)}; clusters emptied {dict(emptied)}; '
          f'orgs today {today} -> after mint+rebind {after} -> after U7 {u7}; per code after U7 {dict(per)}')
effect('A literal, name-blind', lambda r: True)
effect('B3 gated', lambda r: G3.admit(r['name'], r['code'])[0] is not None)
# coverage: share of each SAME body's registry mentions that are coded
import statistics
sh = []
for code, rs in by.items():
    s = [r for r in rs if r['side'] == 'registry' and r['final'] == 'SAME']
    tot = sum(r['coded'] + r['uncoded'] + r['other'] for r in s)
    if tot: sh.append(sum(r['coded'] for r in s) / tot)
print('median coded share of a SAME body (registry):', round(100 * statistics.median(sh), 1), '% over', len(sh), 'codes; below 10%:', sum(1 for x in sh if x < 0.10))
