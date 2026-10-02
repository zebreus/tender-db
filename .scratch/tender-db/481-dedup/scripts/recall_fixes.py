import pickle, collections
from rf import wilson
pos, neg, unm = pickle.load(open('F.pkl', 'rb'))
tw = pickle.load(open('twins_ef.pkl', 'rb'))
DAY = 86400
def grain_ok(t): return t is not None and t % 3600 != 0          # sub-hour grain (not on a full hour)
def add_extra(f, A, Bs, ted_pub, doe_pub):
    # value across scopes: any equality among {procedure, lots sum} on each side
    if Bs is None: return
    sa = {A['ev'].get('procedure_cents'), A['ev'].get('lots_cents_sum')} - {None}
    sb = {Bs['ev'].get('procedure_cents'), Bs['ev'].get('lots_cents_sum')} - {None}
    f['val_sc'] = 'na' if not sa or not sb else ('eq' if sa & sb else 'ne')
    # time gate using the DOE dispatch as well as the DOE publication date
    dd = A['dispatched_at']
    alt = abs(ted_pub - dd) / DAY if dd is not None and ted_pub is not None else 1e9
    f['dt_e'] = min(f['dt'], alt)
    f['dp'] = (f['disp'] == 'eq' and grain_ok(A['dispatched_at']))
    f['dp_known'] = f['disp'] != 'na'
for f in pos: add_extra(f, f['A'], f['Bs'], f['rec']['ted']['published_at'], f['rec']['doe']['published_at'])
for f in neg: add_extra(f, f['A'], f['Bs'], f['rec']['doe']['published_at'] + f['rec']['delta_days'] * DAY, f['rec']['doe']['published_at'])
for u in unm:
    tp0 = u['t']['doe']['published_at']
    for f in u['cands']:
        tedpub = tp0 + f['c']['delta_days'] * DAY
        if f['has_signals']: add_extra(f, u['A'], f['Bs'], tedpub, tp0)
        else:
            dd = u['A']['dispatched_at']; f['dt_e'] = min(f['dt'], abs(tedpub - dd) / DAY if dd else 1e9)
            f['dp'] = (f['disp'] == 'eq' and grain_ok(dd)); f['dp_known'] = f['disp'] != 'na'

CONTRA = ('cpv', 'dl', 'val', 'iref', 'lots')
def nc(f, v):
    keys = [k for k in CONTRA if not (k == 'val' and v in ('scoped', 'drop'))]
    if any(f[k] == 'ne' for k in keys): return False
    if v == 'scoped' and f.get('val_sc') == 'ne': return False
    return True
def make(corr=('dl',), val='strict', time='pub', org='id', dp=False, title=True, sub=True):
    def r(f):
        if f.get('t_norm') is None: return False          # candidate without signals
        if not (f['org'] if org == 'id' else (f['org'] or f['name'])): return False
        if title and not f['t_norm']: return False
        if sub and not f['sub']: return False
        if (f['dt'] if time == 'pub' else f['dt_e']) > 7: return False
        if not nc(f, val): return False
        if corr is None: return True
        ok = any(f[k] == 'eq' for k in corr) or (dp and f['dp'])
        return ok
    return r

def score(r, label):
    tp = sum(r(f) for f in pos)
    fps = [f for f in neg if r(f)]
    fp_unknown_dp = 0
    # islands: correct unique join to the BT-701 twin
    isl = collections.Counter()
    for u in unm:
        t = tw.get(u['t']['tender_id'])
        if not t: continue
        hits = [f for f in u['cands'] if r(f)]
        ids = {f['c']['tender_id'] for f in hits}
        if ids == {t[0]}: isl['correct'] += 1
        elif t[0] in ids: isl['amb_incl_twin'] += 1
        elif ids: isl['WRONG'] += 1
        else: isl['none'] += 1
    # all unmerged: unique joins
    j = collections.Counter()
    for u in unm:
        hits = {f['c']['tender_id'] for f in u['cands'] if r(f)}
        g = 'sdk01' if u['t']['doe_profile'].startswith('eforms-sdk-0.1') else 'other'
        if len(hits) == 1: j[g + '_join'] += 1
        elif hits: j[g + '_amb'] += 1
    return dict(rule=label, tp=tp, recall=round(tp / 458, 4), recall_lb=round(wilson(tp, 458)[0], 4),
                fp=len(fps), fp_g1=sum(not f['ted_has_doe'] for f in fps),
                fp_ids=[(f['rec']['doe_tender_id'], f['rec']['ted_tender_id']) for f in fps],
                islands=dict(isl), unmerged=dict(j))

def show(s):
    print('%-62s TP %3d (%.1f%%, lb %.1f%%) FP %d (G1 %d) islands %s unmerged %s' % (
        s['rule'], s['tp'], 100 * s['recall'], 100 * s['recall_lb'], s['fp'], s['fp_g1'], s['islands'], s['unmerged']))
    if s['fp']: print('      FP pairs (doe_tender, ted_tender):', s['fp_ids'])

V = [
 ('L2  O&T&S&D7&NC&DL', make(('dl',))),
 ('L2+Fv  value scoped', make(('dl',), val='scoped')),
 ('L2+Fv2 value dropped from NC', make(('dl',), val='drop')),
 ('L2+Ft  time gate min(pub,doe-disp)', make(('dl',), time='e')),
 ('L2+Fd  corroborator DL|DP', make(('dl',), dp=True)),
 ('L2+Fd+Fv2+Ft', make(('dl',), dp=True, val='drop', time='e')),
 ('L4  O&T&S&D7&NC&(IR|DL|VL)', make(('iref', 'dl', 'val'))),
 ('L4+Fv2', make(('iref', 'dl', 'val'), val='drop')),
 ('L4+Ft', make(('iref', 'dl', 'val'), time='e')),
 ('L4+Fd', make(('iref', 'dl', 'val'), dp=True)),
 ('L4+Fd+Fv2+Ft', make(('iref', 'dl', 'val'), dp=True, val='drop', time='e')),
 ('L4+Fo  org id OR name_norm', make(('iref', 'dl', 'val'), org='name')),
 ('L5  O&T&S&D7&NC', make(None)),
 ('L5+Fv2', make(None, val='drop')),
 ('L5+Ft', make(None, time='e')),
 ('L5+Fv2+Ft', make(None, val='drop', time='e')),
 ('L5+Fo', make(None, org='name')),
 ('L5-S  (subtype-free)', make(None, sub=False)),
]
if __name__ == '__main__':
    out = []
    for lab, r in V:
        s = score(r, lab); out.append(s); show(s)
    import json; json.dump(out, open('fixes.json', 'w'), indent=1)
