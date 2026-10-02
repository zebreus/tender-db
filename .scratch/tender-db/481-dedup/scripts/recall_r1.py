import collections
from fixes import pos, neg, unm, tw, nc, score, show
from rf import wilson
def dps(f): return f['dp']
def dph(f): return f['disp'] == 'eq' and not f['dp']
def core(f):
    return (f.get('t_norm') is not None and f['org'] and f['t_norm'] and f['sub'] and f['dt_e'] <= 7 and nc(f, 'drop'))
def R1(f): return core(f) and (f['dl'] == 'eq' or dps(f) or (dph(f) and f['iref'] == 'eq'))
def R2(f): return core(f) and (f['dl'] == 'eq' or f['disp'] == 'eq' or f['iref'] == 'eq' or f['val'] == 'eq')
if __name__ == '__main__':
    for lab, r in (('R1 core & (DL | DPs | DPh&IR)', R1), ('R2 core & (DL | DP | IR | VL)', R2)):
        s = score(r, lab); show(s)
        tt = [f for f in pos if f['disp'] == 'eq']
        k = sum(r(f) for f in tt)
        print('   true twins (dispatch-identical) recalled: %d/%d  Wilson LB %.4f' % (k, len(tt), wilson(k, len(tt))[0]))
    # which corroborator admits each R1 positive
    c = collections.Counter()
    for f in pos:
        if R1(f):
            c[('DL' if f['dl'] == 'eq' else '') + ('+IR' if f['iref'] == 'eq' else '') + ('+DPs' if dps(f) else '') + ('+DPh' if dph(f) else '')] += 1
    print('R1 admits by corroborator set:', c.most_common())
    print('R1 admits relying on dispatch without DL/IR:', sum(1 for f in pos if R1(f) and f['dl'] != 'eq' and f['iref'] != 'eq'))
    # negatives reaching the corroborator test (pass core)
    cn = [f for f in neg if core(f)]
    print('negatives passing core:', len(cn), [(f['rec']['doe_tender_id'], f['rec']['ted_tender_id'], f['disp'], f['dl'], f['iref']) for f in cn])
    # negatives one atom away from core
    def miss1(f):
        atoms = {'org': f['org'], 'title': f['t_norm'], 'sub': f['sub'], 'dt': f['dt_e'] <= 7, 'nc': nc(f, 'drop')}
        bad = [k for k, v in atoms.items() if not v]
        return bad
    m1 = collections.Counter(tuple(miss1(f)) for f in neg if len(miss1(f)) == 1)
    print('negatives failing exactly one core atom:', dict(m1))
    print('  title-equal negatives that fail only nc:', [(f['rec']['doe_tender_id'], f['rec']['ted_tender_id'], [k for k in f['contra'] if k != 'val']) for f in neg if miss1(f) == ['nc']][:20])
