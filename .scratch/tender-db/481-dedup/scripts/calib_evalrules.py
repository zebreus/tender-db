import pickle, collections, json, sys
from feat import wilson
from rules import RULES

pos, neg, unm = pickle.load(open('feats.pkl', 'rb'))
# conservative: every negative shares a buyer org id by construction (shared_org_ids), so treat org/name as True
for f in neg:
    f['org'] = True
    f['name'] = True

NP = len(pos); NN = len(neg)
clean = [f for f in pos if f['clean_pairing']]
negby = collections.defaultdict(list)
for f in neg:
    if f['pool'] == 'twin_known':
        negby[f['doe_notice']].append(f)

out = []
for name, r in RULES:
    tp = sum(1 for f in pos if r(f))
    tpc = sum(1 for f in clean if r(f))
    fps = [f for f in neg if r(f)]
    fp = len(fps)
    fp_tk = sum(1 for f in fps if f['pool'] == 'twin_known')
    prec = tp / (tp + fp) if tp + fp else float('nan')
    plo, _ = wilson(tp, tp + fp)
    rlo, rhi = wilson(tp, NP)
    _, fprhi = wilson(fp, NN)
    # per-query (positive DOE notice with its <=3 nearest same-buyer TED competitors from twin_known)
    q = collections.Counter()
    qa = collections.Counter()
    for f in pos:
        comps = negby.get(f['doe_notice'], [])
        hit_t = r(f)
        hit_c = sum(1 for c in comps if r(c))
        if hit_t and hit_c == 0: q['correct'] += 1
        elif hit_t and hit_c > 0: q['ambiguous'] += 1
        elif not hit_t and hit_c == 1: q['wrong'] += 1
        elif not hit_t and hit_c > 1: q['ambig_wrong'] += 1
        else: q['none'] += 1
        # twin absent: only competitors (queries that have >=1 competitor)
        if comps:
            qa['n'] += 1
            if hit_c == 1: qa['wrong_join'] += 1
            elif hit_c > 1: qa['ambiguous'] += 1
    out.append(dict(rule=name, tp=tp, recall=tp / NP, recall_lo=rlo, tp_clean=tpc, recall_clean=tpc / len(clean),
                    fp=fp, fp_twin_known=fp_tk, fp_doe_only=fp - fp_tk, fpr=fp / NN, fpr_hi=fprhi,
                    precision=prec, precision_lo=plo,
                    q_correct=q['correct'], q_ambig=q['ambiguous'], q_wrong=q['wrong'] + q['ambig_wrong'],
                    q_wrong_single=q['wrong'], q_none=q['none'],
                    qa_n=qa['n'], qa_wrong=qa['wrong_join'], qa_ambig=qa['ambiguous'],
                    fp_ids=[(f['pool'], f['doe_tender'], f['ted_tender']) for f in fps][:40]))

json.dump(out, open('rule_eval.json', 'w'), indent=1)
hdr = '%-78s %4s %6s %6s %4s %6s %7s %6s %7s | %4s %4s %4s | %4s %4s %4s' % (
    'rule', 'TP', 'rec', 'recCl', 'FP', 'FPR', 'FPRub', 'prec', 'precLB', 'qOK', 'qAmb', 'qBad', 'aN', 'aBad', 'aAmb')
print(hdr)
for o in out:
    print('%-78s %4d %6.3f %6.3f %4d %6.4f %7.4f %6.4f %7.4f | %4d %4d %4d | %4d %4d %4d' % (
        o['rule'][:78], o['tp'], o['recall'], o['recall_clean'], o['fp'], o['fpr'], o['fpr_hi'], o['precision'], o['precision_lo'],
        o['q_correct'], o['q_ambig'], o['q_wrong'], o['qa_n'], o['qa_wrong'], o['qa_ambig']))
print('NP', NP, 'clean', len(clean), 'NN', NN, 'queries with competitors', sum(1 for f in pos if negby.get(f['doe_notice'])))
