import pickle, collections
from bands import BANDS, pos, neg
def atoms(f):
    out = []
    if not f['org']: out.append('org_split')
    if not f['t_norm']: out.append('title_differs')
    if not f['sub']: out.append('subtype_differs')
    if f['dt'] > 7: out.append('dt>7')
    for k in f['contra']: out.append('contra_' + k)
    if not ('eq' in (f['iref'], f['dl'], f['val'])): out.append('no_corroborator')
    if f['dl'] != 'eq': out.append('dl_' + f['dl'])
    if f['iref'] != 'eq': out.append('iref_' + f['iref'])
    return out
if __name__ == "__main__":
  for name in ['L2 org&tnorm&sub&dt7&nocontra&dl', 'L4 org&tnorm&sub&dt7&nocontra&(iref|dl|val)', 'L5 org&tnorm&sub&dt7&nocontra']:
    r = BANDS[name]; miss = [f for f in pos if not r(f)]
    c = collections.Counter(); combo = collections.Counter()
    for f in miss:
        a = atoms(f); c.update(a)
        core = [x for x in a if not x.startswith(('dl_', 'iref_'))]
        combo[tuple(core)] += 1
    print('==', name, 'missed', len(miss))
    print('  atom counts:', dict(c.most_common()))
    print('  core combos:', combo.most_common())
    # subtype of missed
    print('  doe subtype of missed:', collections.Counter(f['rec']['doe']['subtype'] for f in miss).most_common())
