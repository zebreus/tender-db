import json, os, sys, collections
U1 = sys.argv[1]; HERE = os.path.dirname(os.path.abspath(__file__))
exec(open(f'{HERE}/a3_effect.py').read().split("print('reproduce")[0])
moved, remaining, mbc, pcm, kept = plan({'DIFFERENT','NOT_ORG'})
full = [o for o in moved if remaining[o] == 0]
part = [o for o in moved if remaining[o] > 0]
print('moved total', sum(moved.values()), 'from', len(moved), 'orgs; via retirement', sum(moved[o] for o in full), 'from', len(full), '; partial', sum(moved[o] for o in part), 'from', len(part))
# split-empty: emptied clusters whose moved mentions go to >1 identity
def targets(o):
    s = set()
    for raw, n in mc[o].items():
        c = code_of(raw)
        k = cls.get((c, o), 'SAME' if c else None)
        if c and k not in ('DIFFERENT','NOT_ORG'): s.add(c)
    return s
print('emptied clusters with >1 target identity:', [(o, targets(o)) for o in full if len(targets(o)) > 1])
print('partial clusters with >1 target identity:', [(o, targets(o)) for o in part if len(targets(o)) > 1])
# without pins, wrong-body moves and wrong retirements
m0, r0, *_ = plan(set())
wrong = [(r['code'], r['org_id'], r['name'][:45], r['coded']) for r in reg if r['final'] in ('DIFFERENT','NOT_ORG') and r0[r['org_id']] == 0 and m0[r['org_id']] > 0]
print('without pins: non-SAME clusters retired into another body:', wrong)
# coded mentions per code: share pinned
pin_m = sum(r['coded'] for r in reg if r['final'] in ('DIFFERENT','NOT_ORG'))
print('pinned coded mentions', pin_m, 'of', sum(mbc.values()) + pin_m, '= %.2f%%' % (100*pin_m/(sum(mbc.values())+pin_m)))
