import pickle, collections, json
from feat import wilson
from rules import RULES
R = dict(RULES)
pos, neg, unm = pickle.load(open('feats.pkl', 'rb'))
for f in neg: f['org'] = True; f['name'] = True
negby = collections.defaultdict(list)
for f in neg:
    if f['pool'] == 'twin_known': negby[f['doe_notice']].append(f)
gate = lambda f: f['org'] and f['sub'] and f['dt'] <= 7
ng = [f for f in neg if gate(f)]
ladder = [('L1', 'C13 org & title norm & sub & dt<=7 & no-contra & iref eq & deadline eq'),
          ('L2', 'C11 org & title norm & sub & dt<=7 & no-contra & deadline eq'),
          ('L2b', 'C15 org & deadline exact & sub & dt<=7 & no-contra'),
          ('Lv', 'C2  org & cpv eq & deadline exact & value exact'),
          ('L3', 'C10 org & title norm & sub & dt<=7 & no-contra & iref eq'),
          ('L4', 'C12 org & title norm & sub & dt<=7 & no-contra & (iref|deadline|value eq)'),
          ('L5', 'C8  org & title norm & sub & dt<=7 & no-contra'),
          ('L6', 'C6  org & title norm & same_subtype & dt<=7'),
          ('L7', 'S9  org & title norm eq'),
          ('L8', 'S10 org & title jacc>=0.80'),
          ('Li', 'S7  org & internal_ref eq'),
          ('Ld', 'S5  org & deadline exact'),
          ('L9', 'S3  org & cpv_main eq')]
def grp(u):
    p = u['profile']
    if p.startswith('eforms-de'): return 'eForms-DE'
    if p in ('eforms-sdk-1.12', 'eforms-sdk-1.13'): return 'T01'
    if p == 'eforms-sdk-1.0': return 'SDK1.0'
    if p == 'eforms-sdk-0.1:uuid': return 'sdk01uuid'
    return 'sdk01num-' + ('EU' if u['eu'] else 'nat')
res = []
for L, rn in ladder:
    for gname in ('none', 'G1'):
        r = (lambda r0: (lambda f: r0(f)))(R[rn]) if gname == 'none' else (lambda r0: (lambda f: r0(f) and not f['ted_has_doe']))(R[rn])
        tp = sum(1 for f in pos if R[rn](f)); tpc = sum(1 for f in pos if f['clean_pairing'] and R[rn](f))
        fp = sum(1 for f in neg if r(f)); fpg = sum(1 for f in ng if r(f))
        q = collections.Counter(); qa = collections.Counter()
        for f in pos:
            comps = negby.get(f['doe_notice'], []); ht = R[rn](f); hc = sum(1 for c in comps if r(c))
            q['ok' if ht and not hc else 'amb' if ht else 'bad' if hc == 1 else 'badamb' if hc else 'none'] += 1
            if comps: qa['bad' if hc == 1 else 'amb' if hc > 1 else 'none'] += 1
        app = collections.defaultdict(collections.Counter); targets = collections.Counter(); subt = collections.Counter()
        for u in unm:
            h = [c for c in u['cands'] if r(c)]
            g = grp(u); app[g]['n'] += 1
            if len(h) == 1:
                app[g]['join'] += 1; targets[h[0]['cand_tender']] += 1; subt[u['subtype']] += 1
            elif h: app[g]['amb'] += 1
        tot = collections.Counter()
        for g in list(app): tot.update(app[g])
        app['ALL'] = tot
        res.append(dict(band=L, rule=rn, guard=gname, tp=tp, recall=round(tp / 458, 4), recall_lb=round(wilson(tp, 458)[0], 4),
                        tp_clean=tpc, recall_clean=round(tpc / 367, 4), fp=fp, fpr=round(fp / 1289, 5), fpr_ub=round(wilson(fp, 1289)[1], 5),
                        fp_gated=fpg, n_gated=len(ng), fpr_gated_ub=round(wilson(fpg, len(ng))[1], 5),
                        precision=round(tp / (tp + fp), 4), precision_lb=round(wilson(tp, tp + fp)[0], 4),
                        query=dict(q), twin_absent=dict(qa), twin_absent_bad_ub=round(wilson(qa['bad'], 380)[1], 4),
                        unmerged={g: dict(v) for g, v in app.items()}, join_subtypes=dict(subt),
                        ted_targets_joined_twice=sum(1 for v in targets.values() if v > 1)))
json.dump(res, open('results.json', 'w'), indent=1)
for x in res:
    u = x['unmerged']
    print('%-3s %-4s TP %3d rec %.3f (lb %.3f, clean %.3f) FP %3d fprUB %.4f gFP %2d/%d gUB %.4f prec %.4f lb %.4f | q %s | absent bad %d ub %.4f | joins eDE %d/%d T01 %d/%d SDK1.0 %d sdk01 %d | amb ALL %d | tgt2x %d | sub %s' % (
        x['band'], x['guard'], x['tp'], x['recall'], x['recall_lb'], x['recall_clean'], x['fp'], x['fpr_ub'], x['fp_gated'], x['n_gated'], x['fpr_gated_ub'],
        x['precision'], x['precision_lb'], x['query'], x['twin_absent'].get('bad', 0), x['twin_absent_bad_ub'],
        u['eForms-DE'].get('join', 0), u['eForms-DE'].get('amb', 0), u['T01'].get('join', 0), u['T01'].get('amb', 0), u['SDK1.0'].get('join', 0),
        sum(u[g].get('join', 0) for g in u if g.startswith('sdk01')), u['ALL'].get('amb', 0), x['ted_targets_joined_twice'], x['join_subtypes']))
