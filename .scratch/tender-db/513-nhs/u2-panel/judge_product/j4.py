import json, collections, statistics, sys, re
exec(open('common.py').read())
capped = {'QHM','QYG','QMF','QKK','QRL','QOQ','QMJ','RBL','RBN','RJZ','RWW'}
rep=0; rep_now=0; merges=0; merges_now=0; anchors=[]; orgs_now=0
ppon_before=0; ppon_after=0
for b,v in bodies.items():
    orgs = v['reg']|v['ppon']
    if v['ppon']:
        # survivor: PPON (RYA: the PPON paired with real code 30913621)
        surv = 30913621 if b=='RYA' and 30913621 in v['ppon'] else max(v['ppon'], key=total)
    else:
        surv = max(v['reg'], key=total); anchors.append(b)
    losers = orgs - {surv}
    m = sum(total(o) for o in losers)
    rep += m; merges += len(losers)
    if b in capped: orgs_now += len(orgs)
    else: rep_now += m; merges_now += len(losers); orgs_now += 1
    if v['ppon']:
        ppon_before += total(surv); ppon_after += sum(total(o) for o in orgs)
print('repointed', rep, 'now', rep_now, 'merges', merges, 'now', merges_now, 'orgs now', orgs_now)
print('anchors', len(anchors), sorted(anchors))
print('on PPON rows before/after', ppon_before, ppon_after)
