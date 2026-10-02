import pickle, collections
pos, neg, unm = pickle.load(open('F.pkl', 'rb'))
nc = lambda f: not f['contra']
corr = lambda f: 'eq' in (f['iref'], f['dl'], f['val'])
BANDS = {
 'L1 org&tnorm&sub&dt7&nocontra&iref&dl': lambda f: f['org'] and f['t_norm'] and f['sub'] and f['dt'] <= 7 and nc(f) and f['iref'] == 'eq' and f['dl'] == 'eq',
 'L2 org&tnorm&sub&dt7&nocontra&dl':      lambda f: f['org'] and f['t_norm'] and f['sub'] and f['dt'] <= 7 and nc(f) and f['dl'] == 'eq',
 'L3 org&tnorm&sub&dt7&nocontra&iref':    lambda f: f['org'] and f['t_norm'] and f['sub'] and f['dt'] <= 7 and nc(f) and f['iref'] == 'eq',
 'L4 org&tnorm&sub&dt7&nocontra&(iref|dl|val)': lambda f: f['org'] and f['t_norm'] and f['sub'] and f['dt'] <= 7 and nc(f) and corr(f),
 'L5 org&tnorm&sub&dt7&nocontra':         lambda f: f['org'] and f['t_norm'] and f['sub'] and f['dt'] <= 7 and nc(f),
}
if __name__ == '__main__':
    for name, r in BANDS.items():
        tp = sum(r(f) for f in pos); fp = sum(r(f) for f in neg); fpg = sum(r(f) and not f['ted_has_doe'] for f in neg)
        fpp = collections.Counter(f['pool'] for f in neg if r(f))
        print('%-48s TP %3d/458 (%.1f%%) miss %3d  FP %2d %s  FP|G1 %d' % (name, tp, 100*tp/458, 458-tp, fp, dict(fpp), fpg))
    print('negatives G1-eligible (TED side has no DOE version):', sum(not f['ted_has_doe'] for f in neg), 'of', len(neg),
          collections.Counter((f['pool'], f['ted_has_doe']) for f in neg))
