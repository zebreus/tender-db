"""Bands (rule x guard), calibration numbers, and application to unmerged.json."""
import pickle, collections, json
from feat import wilson
from rules import RULES

pos, neg, unm = pickle.load(open('feats.pkl', 'rb'))
for f in neg:
    f['org'] = True; f['name'] = True
R = dict(RULES)
NP, NN = len(pos), len(neg)
negby = collections.defaultdict(list)
for f in neg:
    if f['pool'] == 'twin_known':
        negby[f['doe_notice']].append(f)

# guard G1: the candidate TED Tender has no DOE version (TED-only target). Positives: the twin Tender was
# TED-only before the key merged it, so G1 passes for every positive.
G = {
    'none': (lambda f: True, lambda f: True),
    'G1': (lambda f: True, lambda f: not f['ted_has_doe']),
}

BANDS = [
    'C13 org & title norm & sub & dt<=7 & no-contra & iref eq & deadline eq',
    'C11 org & title norm & sub & dt<=7 & no-contra & deadline eq',
    'C15 org & deadline exact & sub & dt<=7 & no-contra',
    'C2  org & cpv eq & deadline exact & value exact',
    'C10 org & title norm & sub & dt<=7 & no-contra & iref eq',
    'C4  org & internal_ref eq & title norm',
    'C12 org & title norm & sub & dt<=7 & no-contra & (iref|deadline|value eq)',
    'C18 org & title jacc>=0.8 & sub & dt<=7 & no-contra & (iref|deadline eq)',
    'C8  org & title norm & sub & dt<=7 & no-contra',
    'C17 org & title jacc>=0.8 & sub & dt<=7 & no-contra',
    'C6  org & title norm & same_subtype & dt<=7',
    'S8  org & title exact_ci',
    'S10 org & title jacc>=0.80',
    'S7  org & internal_ref eq',
    'S5  org & deadline exact',
    'C1  org & deadline exact & cpv eq',
    'C3  org & internal_ref eq & cpv eq',
    'S3  org & cpv_main eq',
    'N1  name & title norm & sub & dt<=7 & no-contra & (iref|deadline|value eq)',
    'N2  name & title norm & dt<=7 & no-contra & (iref|deadline eq)  [subtype-free]',
]


def calib(rule, gname):
    _, gt = G[gname]
    r = lambda f: rule(f) and gt(f)
    tp = sum(1 for f in pos if rule(f))          # G1 passes all positives
    fps = [f for f in neg if r(f)]
    fp = len(fps)
    plo, _ = wilson(tp, tp + fp)
    _, fprhi = wilson(fp, NN)
    qok = qamb = qbad = 0; qa_bad = qa_amb = 0
    for f in pos:
        comps = negby.get(f['doe_notice'], [])
        ht = rule(f); hc = sum(1 for c in comps if r(c))
        if ht and hc == 0: qok += 1
        elif ht and hc: qamb += 1
        elif hc: qbad += 1
        if comps:
            if hc == 1: qa_bad += 1
            elif hc > 1: qa_amb += 1
    return dict(tp=tp, recall=tp / NP, fp=fp, fpr=fp / NN, fpr_ub=fprhi,
                precision=tp / (tp + fp) if tp + fp else None, precision_lb=plo,
                q_ok=qok, q_amb=qamb, q_bad=qbad, qa_bad=qa_bad, qa_amb=qa_amb, qa_bad_ub=wilson(qa_bad, 380)[1])


def group(u):
    p = u['profile']
    if p.startswith('eforms-de'):
        return 'eForms-DE'
    if p == 'eforms-sdk-0.1:num':
        return 'sdk-0.1 num ' + ('EU' if u['eu'] else 'nat')
    if p == 'eforms-sdk-0.1:uuid':
        return 'sdk-0.1 uuid'
    if p == 'eforms-sdk-1.0':
        return 'EU-SDK 1.0 (E-forms)'
    return 'EU-SDK 1.12/1.13 (T01)'


def apply(rule, gname):
    _, gt = G[gname]
    r = lambda f: rule(f) and gt(f)
    res = collections.defaultdict(collections.Counter)
    detail = []
    for u in unm:
        g = group(u)
        hits = [c for c in u['cands'] if r(c)]
        res[g]['n'] += 1
        hidden = u['n_cand'] > u['n_cand_sig']
        if len(hits) == 1:
            res[g]['join'] += 1
            if hidden: res[g]['join_unsig_cands'] += 1
            if hits[0]['ted_has_doe']: res[g]['join_target_has_doe'] += 1
            if hits[0]['cand_island']: res[g]['join_target_island'] += 1
            detail.append((u['tender_id'], hits[0]['cand_tender'], g, u['subtype'], hits[0]['cand_subtype']))
        elif len(hits) > 1:
            res[g]['ambiguous'] += 1
    tot = collections.Counter()
    for g in res:
        tot.update(res[g])
    res['ALL'] = tot
    return res, detail


if __name__ == '__main__':
    out = {}
    print('%-80s %-4s %4s %6s %3s %7s %7s %7s | %3s %3s %3s | %3s %3s %6s' % (
        'band', 'grd', 'TP', 'recall', 'FP', 'FPRub', 'prec', 'precLB', 'qOK', 'qAm', 'qBd', 'aBd', 'aAm', 'aBdUB'))
    for b in BANDS:
        for gname in ('none', 'G1'):
            c = calib(R[b], gname)
            out[(b, gname)] = c
            print('%-80s %-4s %4d %6.3f %3d %7.4f %7.4f %7.4f | %3d %3d %3d | %3d %3d %6.4f' % (
                b[:80], gname, c['tp'], c['recall'], c['fp'], c['fpr_ub'], c['precision'] or 0, c['precision_lb'],
                c['q_ok'], c['q_amb'], c['q_bad'], c['qa_bad'], c['qa_amb'], c['qa_bad_ub']))
    print()
    groups = ['eForms-DE', 'sdk-0.1 num EU', 'sdk-0.1 num nat', 'sdk-0.1 uuid', 'EU-SDK 1.0 (E-forms)', 'EU-SDK 1.12/1.13 (T01)', 'ALL']
    apps = {}
    for b in BANDS:
        for gname in ('none', 'G1'):
            res, det = apply(R[b], gname)
            apps[(b, gname)] = (res, det)
            print('%-80s %-4s ' % (b[:80], gname) + '  '.join(
                '%s:%d/%d/%d(u%d,d%d)' % (g.split()[0] + (g.split()[1] if len(g.split()) > 1 else ''), res[g]['join'], res[g]['ambiguous'], res[g]['n'],
                                         res[g]['join_unsig_cands'], res[g]['join_target_has_doe']) for g in groups if g in res))
    pickle.dump((out, apps), open('bands.pkl', 'wb'))
