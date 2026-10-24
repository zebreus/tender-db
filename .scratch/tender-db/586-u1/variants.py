"""Issue 586 unit 1: the rules as the issue literally reads them (V0), and what each tightening that leads to
fold.py's final rules removes. usage: python3 -I variants.py <audit-576-fanout dir> [--salted]"""
import hashlib, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fold

D = sys.argv[1]
t2 = json.load(open(os.path.join(D, 'second-look/raw/t2_classified.json')))
byid = {r['id']: r for r in t2}


def tier_v(tp, parent, tp_brackets='allow', min_e='final', tp_organ=False, place='muni', places=True):
    """fold.tier with each tightening switchable: tp_brackets 'all' (cut every touchpoint bracket) or 'allow';
    min_e 1 / 2 / 'final'; tp_organ True also cuts an organ phrase off the touchpoint; place 'suffix' (the bare
    place = any tail of the parent key) or 'muni' (the tail of a town-hall entity only)."""
    pb = fold.parent_brackets(parent)
    saved = fold.TP_BRACKET_PLACES
    if not places:
        fold.TP_BRACKET_PLACES = set()
    t = fold.key_tokens(tp, 'parent' if tp_brackets == 'all' else 'tp', pb)
    fold.TP_BRACKET_PLACES = saved
    if tp_brackets == 'all':      # side 'parent' skips the touchpoint's head-label cut; apply it here
        for h in fold._TP_HEAD:
            if len(t) > len(h) and t[:len(h)] == h:
                t = t[len(h):]
                break
    p = fold.key_tokens(parent, 'parent')
    if not t or not p:
        return None
    if t == p:
        return 'A'
    if fold._organ_syn(t) == fold._organ_syn(p):
        return 'C0'
    organ, e = fold.organ_split(p)
    ok = {1: lambda e: len(e) >= 1, 2: lambda e: len(e) >= 2, 'final': fold._entity_ok}[min_e]
    if organ and ok(e) and t == e:
        return 'C1'
    if tp_organ:
        to, te = fold.organ_split(t)
        if to and te == e and ok(e):
            return 'C1t'
    em, tm = fold._muni(e), fold._muni(t)
    if em[0] == 'ayuntamiento' and len(em) >= 2:
        if tm == em:
            return 'C2'
        if t == em[1:]:
            return 'C3'
    if tm[0] == 'ayuntamiento' and len(tm) >= 2 and p[0] not in fold.MUNI and p == tm[1:]:
        return 'C3r'
    if place == 'suffix' and len(t) < len(p) and p[-len(t):] == t:
        return 'C3s'
    return None


STEPS = [
    ('V0 literal', dict(tp_brackets='all', min_e=1, tp_organ=True, place='suffix')),
    ('T1 bare place only under a town hall', dict(tp_brackets='all', min_e=1, tp_organ=True)),
    ('T2 no organ cut on the touchpoint', dict(tp_brackets='all', min_e=1)),
    ('T3 touchpoint brackets: allow-list only', dict(min_e=1, places=False)),
    ('T3b + a bracket naming a province (relax)', dict(min_e=1)),
    ('T4 one-token entity >= 4 chars (final)', dict()),
]
res = {}
for name, kw in STEPS:
    res[name] = {i: tier_v(r['name'], r['pp'], **kw) for i, r in byid.items()}
assert all(res[STEPS[-1][0]][i] == fold.tier(r['name'], r['pp']) for i, r in byid.items()), 'final != fold.tier'


def cnt(name, k):
    ids = [i for i, t in res[name].items() if t and t[0] == k]
    return len(ids), sum(byid[i]['tpo'] for i in ids)


for name, _ in STEPS:
    a, c = cnt(name, 'A'), cnt(name, 'C')
    print(f'{name:42} A {a[0]:4} orgs {a[1]:5} m | C {c[0]:4} orgs {c[1]:5} m')
for (n1, _), (n2, _) in zip(STEPS, STEPS[1:]):
    for k in 'AC':
        d = sorted(i for i in byid if (res[n1][i] or ' ')[0] == k and (res[n2][i] or ' ')[0] != k)
        g = sorted(i for i in byid if (res[n1][i] or ' ')[0] != k and (res[n2][i] or ' ')[0] == k)
        print(f'\n-- {n2}: tier {k} drops {len(d)} orgs / {sum(byid[i]["tpo"] for i in d)} m, gains {len(g)} / '
              f'{sum(byid[i]["tpo"] for i in g)} m')
        for i in d + g:
            r = byid[i]
            print(f"   {'-' if i in d else '+'} {i} {str(res[n1][i] if i in d else res[n2][i]):3} {r['tpo']:4} "
                  f"parent {r['pp'][:95]!r} -> tp {r['name'][:95]!r}")

if '--salted' in sys.argv:
    for name in [STEPS[0][0]]:
        for k, salt in [('A', '586-u1-tierA'), ('C', '586-u1-tierC')]:
            pop = [i for i, t in res[name].items() if t and t[0] == k]
            s = sorted(pop, key=lambda i: hashlib.sha256(f'{salt}:{i}'.encode()).hexdigest())[:40]
            print(f'\n== {name} salted {k} ({salt}) 40 of {len(pop)}')
            for j, i in enumerate(s, 1):
                r = byid[i]
                print(f"  {j:2} {i} {res[name][i]:3} {r['tpo']:4} parent {r['pp'][:90]!r} -> tp {r['name'][:90]!r}")
