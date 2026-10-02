import pickle, collections, json
from bands import BANDS
from miss import atoms
_, _, unm = pickle.load(open('F.pkl', 'rb'))
tw = pickle.load(open('twins_ef.pkl', 'rb'))
lab = [u for u in unm if tw.get(u['t']['tender_id'])]
def evalband(r, guard=False, units=lab):
    out = collections.Counter(); misses = []
    for u in units:
        twin = tw[u['t']['tender_id']][0]
        hits = [f for f in u['cands'] if f['has_signals'] and r(f) and (not guard or not f['ted_has_doe'])]
        ht = [f for f in hits if f['c']['tender_id'] == twin]
        wrong = [f for f in hits if f['c']['tender_id'] != twin]
        if ht and not wrong: out['join_correct'] += 1
        elif ht and wrong: out['ambiguous_incl_twin'] += 1
        elif wrong: out['WRONG_join' if len(wrong) == 1 else 'ambiguous_wrong'] += 1
        else: out['none'] += 1; misses.append(u)
    return out, misses
if __name__ == '__main__':
    print('labelled islands', len(lab))
    for name, r in BANDS.items():
        for g in (False, True):
            o, m = evalband(r, g)
            print('%-48s G1=%d %s' % (name, g, dict(o)))
    # reasons for misses of L4 / L5 on islands
    for name in ['L2 org&tnorm&sub&dt7&nocontra&dl', 'L4 org&tnorm&sub&dt7&nocontra&(iref|dl|val)', 'L5 org&tnorm&sub&dt7&nocontra']:
        o, m = evalband(BANDS[name])
        c = collections.Counter(); combos = collections.Counter()
        for u in m:
            twin = tw[u['t']['tender_id']][0]
            f = [f for f in u['cands'] if f['c']['tender_id'] == twin][0]
            a = atoms(f); c.update(a); combos[tuple(x for x in a if not x.startswith(('dl_', 'iref_')))] += 1
        print('==', name, 'misses', len(m)); print('  ', dict(c.most_common())); print('  ', combos.most_common())
