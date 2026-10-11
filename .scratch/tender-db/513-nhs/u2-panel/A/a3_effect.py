import json, os, sys, collections, statistics
U1 = sys.argv[1]
HERE = os.path.dirname(os.path.abspath(__file__))
al = json.load(open(f'{HERE}/allow.json')); A = set(al['A']) | {'DQF','TAD','TAJ'}
T = json.load(open(f'{HERE}/tails.json')); mc = {int(k): v for k, v in T['mc'].items()}
rows = json.load(open(f'{U1}/adjudication-rows.json'))
reg = [r for r in rows if r['side']=='registry']; pp = [r for r in rows if r['side']=='ppon']
def norm(raw): return ''.join(c.upper() for c in raw if c.isascii() and c.isalnum())
def variants(c):
    out=set(); idx=[i for i,ch in enumerate(c) if ch in 'O0']
    for mask in range(1,1<<len(idx)):
        s=list(c)
        for k,i in enumerate(idx):
            if mask>>k&1: s[i]='0' if c[i]=='O' else 'O'
        out.add(''.join(s))
    return out
def code_of(raw, fold=True):
    if raw is None: return None
    v = norm(raw)
    if not v.startswith('GBNHS') or len(v)!=8: return None
    t = v[5:]
    if t in A: return t
    if fold:
        h=[x for x in variants(t) if x in A]
        if len(h)==1: return h[0]
    return None
cls = {(r['code'], r['org_id']): r['final'] for r in reg}
# (code, org) pairs present in mention data but not adjudicated
unadj = collections.Counter()
for org, d in mc.items():
    for raw, n in d.items():
        c = code_of(raw)
        if c and (c, org) not in cls and not any(r['org_id']==org and r['side']=='ppon' for r in pp):
            unadj[(c, org, raw)] += n
print('coded (code,org) pairs present but not adjudicated:', dict(unadj))
def plan(pin_classes, fold=True):
    moved = collections.Counter(); kept_coded = collections.Counter(); remaining = {}
    moved_by_class = collections.Counter(); per_code_moved = collections.defaultdict(collections.Counter)
    for org, d in mc.items():
        rem = 0
        for raw, n in d.items():
            c = code_of(raw, fold)
            k = cls.get((c, org), 'SAME' if c else None)  # unadjudicated pairs: RWK/fold hits, all SAME per review
            if c and k not in pin_classes:
                moved[org] += n; moved_by_class[k] += n; per_code_moved[c][k] += n
            else:
                rem += n
                if c: kept_coded[org] += n
        remaining[org] = rem
    return moved, remaining, moved_by_class, per_code_moved, kept_coded
codes = sorted({r['code'] for r in reg})
def counts(pin_classes, u7_mode):
    moved, remaining, mbc, pcm, kept = plan(pin_classes)
    today = after = afteru7 = 0; per = collections.Counter(); emptied = collections.Counter()
    for code in codes:
        R = {r['org_id'] for r in reg if r['code']==code and r['final']=='SAME'}
        P = {r['org_id'] for r in pp if r['code']==code and r['final']=='SAME'}
        t = len(R | P)
        Rrem = {o for o in R if remaining[o] > 0}
        ident = 1 if pcm.get(code) else 0
        a = len(Rrem) + ident + len(P - R)
        # U7: identity merges with the single SAME PPON org (non-SAME PPON holder: hard-denied)
        Pall = [r for r in pp if r['code']==code]
        ok = ident and len(P)==1 and all(r['final']=='SAME' for r in Pall)
        if u7_mode == 'naive': ok = ident and len(P)>=1
        u = a - (1 if ok else 0)
        today += t; after += a; afteru7 += u
        per['better' if u < t else 'worse' if u > t else 'same'] += 1
    for r in reg:
        if remaining[r['org_id']] == 0 and moved[r['org_id']] > 0: emptied[r['final']] += 1
    return today, after, afteru7, dict(per), dict(emptied), dict(mbc)
print('reproduce withdrawn D3 (no pins, no fold-distinction):', counts(set(), 'naive'))
print('design A (pins on DIFFERENT+NOT_ORG), U7 strict:', counts({'DIFFERENT','NOT_ORG'}, 'strict'))
print('design A (pins on DIFFERENT+NOT_ORG), U7 naive :', counts({'DIFFERENT','NOT_ORG'}, 'naive'))
print('design A (pins on DIFFERENT+NOT_ORG+HOSTED), U7 strict:', counts({'DIFFERENT','NOT_ORG','HOSTED_OR_SITE'}, 'strict'))

# --- body coverage: largest single org's share of the body's mentions, today vs after U7 (design A pins) ---
moved, remaining, mbc, pcm, kept = plan({'DIFFERENT','NOT_ORG'})
ppon_m = {r['org_id']: r['uncoded']+r['coded']+r['other'] for r in pp}
shares_today = []; shares_after = []; canon_share = []; coded_share = []
worse_largest = 0; n = 0
for code in codes:
    if code not in A: continue
    R = sorted({r['org_id'] for r in reg if r['code']==code and r['final']=='SAME'})
    P = sorted({r['org_id'] for r in pp if r['code']==code and r['final']=='SAME'})
    if not R: continue
    tot_R = {o: sum(mc[o].values()) for o in R}
    body = sum(tot_R.values()) + sum(ppon_m[o] for o in P if o not in R)
    if body == 0: continue
    largest_today = max(list(tot_R.values()) + [ppon_m[o] for o in P])
    mv = sum(moved[o] for o in R)  # SAME clusters' moved coded (all codes on that org; RET/REN overlaps negligible)
    canon = mv + (ppon_m[P[0]] if len(P)==1 and all(r['final']=='SAME' for r in pp if r['code']==code) else 0)
    rest = [remaining[o] for o in R] + [ppon_m[o] for o in P if canon == mv]
    largest_after = max([canon] + rest)
    shares_today.append(largest_today / body); shares_after.append(largest_after / body); canon_share.append(canon / body)
    coded_share.append(mv / sum(tot_R.values()))
    n += 1
    if largest_after < largest_today: worse_largest += 1
print('\nbodies:', n)
print('median coded share of SAME registry mentions: %.1f%%' % (100*statistics.median(coded_share)), 'below 10%%: %d' % sum(1 for s in coded_share if s < .10))
print('median largest-org share today: %.1f%%, after U7: %.1f%%' % (100*statistics.median(shares_today), 100*statistics.median(shares_after)))
print('median canonical (identity+PPON after U7) share: %.1f%%; canonical is the largest org for %d of %d bodies' % (100*statistics.median(canon_share), sum(1 for c, l in zip(canon_share, shares_after) if abs(c-l) < 1e-12), n))
print('bodies whose largest single org shrinks:', worse_largest)
tot_coded = sum(r['coded'] for r in reg if r['final']=='SAME')
uncoded_distinct = sum(sum(v for k, v in mc[o].items() if k in (None, 'null')) for o in {r['org_id'] for r in reg if r['final']=='SAME'})
other_distinct = sum(sum(v for k, v in mc[o].items() if k not in (None, 'null') and code_of(k) is None) for o in {r['org_id'] for r in reg if r['final']=='SAME'})
print('SAME clusters (distinct orgs): coded rows', tot_coded, 'uncoded mentions', uncoded_distinct, 'other-identifier mentions', other_distinct, 'ppon-org mentions', sum(ppon_m[o] for o in {r['org_id'] for r in pp if r['final']=='SAME'}))
