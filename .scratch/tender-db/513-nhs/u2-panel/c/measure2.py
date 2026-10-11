import json, sys, collections, statistics, re
D=sys.argv[1]
adj=json.load(open(D+'/adjudication-rows.json'))
L=json.load(open(D+'/nhs-listing-2236.json'))['listing']
# correction: TAJ ppon row carries QMM's PPON org id; the listing says 30917612
for r in adj:
    if r['code']=='TAJ' and r['side']=='ppon' and r['org_id']==31542073:
        r['org_id']=30917612; r['other']=None  # count unknown: 30917612 not in mention-counts
mc=collections.defaultdict(dict)
for line in open(D+'/mention-counts-by-org.jsonl'):
    for o,raw,n in json.loads(line)['rows']:
        mc[o][raw]=mc[o].get(raw,0)+n
TYPED={'WESTMIDLANDSAMBULANCESERVICE':'RYA','NHSCHESHIREANDMERSEYSIDEICB':'QYG'}
body=lambda c: TYPED.get(c,c)
capped={'QHM','QKK','QMF','QMJ','QOQ','QRL','QYG','RBL','RBN','RJZ','RWW'}
B=collections.defaultdict(lambda: {'reg':{}, 'ppon':{}, 'xreg':{}, 'xppon':{}})
for r in adj:
    b=B[body(r['code'])]
    key=('reg' if r['side']=='registry' else 'ppon')
    if r['final']!='SAME': key='x'+key
    b[key].setdefault(r['org_id'], r)
def tot(o):
    return sum(mc[o].values()) if o in mc else None
def coded_tail(raw):
    if not raw: return None
    v=''.join(ch for ch in raw if ch.isalnum()).upper()
    return v[5:] if v.startswith('GBNHS') else None
rows=[]
for code,b in sorted(B.items()):
    members=list(b['reg'])+list(b['ppon'])
    if not members: continue
    mt={o:tot(o) for o in members}
    known=[o for o in members if mt[o] is not None]
    T=sum(mt[o] for o in known)
    coded=sum(n for o in b['reg'] for raw,n in mc.get(o,{}).items() if coded_tail(raw)==code)
    ppons=list(b['ppon'])
    if len(ppons)==1: surv=ppons[0]; kind='ppon'
    elif len(ppons)>1:
        # the PPON paired with the real code in the listing
        e=[e for e in L if e['value']=='GBNHS'+code][0]
        surv=e['ppon_holders'][0]['org_id']; kind='ppon(2)'
    else:
        surv=max(b['reg'], key=lambda o:(mt[o] or 0, -o)); kind='anchor'
    largest=max((mt[o] or 0) for o in members)
    rows.append(dict(code=code, n=len(members), nreg=len(b['reg']), nppon=len(ppons), kind=kind, surv=surv, T=T,
        surv_before=(mt[surv] or 0), largest=largest, coded=coded, ppon_m=sum(mt[o] or 0 for o in ppons),
        moved=T-(mt[surv] or 0), capped=code in capped, unknown=[o for o in members if mt[o] is None]))
print('bodies with SAME holders', len(rows))
print('today orgs (distinct SAME per body):', sum(r['n'] for r in rows))
allm=set()
for code,b in B.items(): allm|=set(b['reg'])|set(b['ppon'])
print('distinct SAME orgs:', len(allm))
print('after C, all bodies:', len(rows))
held=[r for r in rows if r['capped']]
print('capped bodies', len(held), 'their listed SAME orgs', sum(r['n'] for r in held))
print('after C now (capped held):', (len(rows)-len(held)) + sum(r['n'] for r in held))
print('survivor kinds', collections.Counter(r['kind'] for r in rows))
print('losers / ledger rows (all):', sum(r['n']-1 for r in rows), ' now:', sum(r['n']-1 for r in rows if not r['capped']))
T=sum(r['T'] for r in rows); print('listed SAME mentions total', T, ' unknown-count members', [u for r in rows for u in r['unknown']])
print('mentions moved onto survivors (all):', sum(r['moved'] for r in rows), ' now:', sum(r['moved'] for r in rows if not r['capped']))
print('mentions on survivors after (all):', T, ' now:', sum(r['T'] for r in rows if not r['capped']))
sh_largest=[r['largest']/r['T'] for r in rows if r['T']]
sh_coded=[r['coded']/r['T'] for r in rows if r['T']]
sh_ppon=[r['ppon_m']/r['T'] for r in rows if r['T'] and r['nppon']]
med=lambda x: round(100*statistics.median(x),1)
print('median share today: largest single org %s%%, PPON org %s%% (bodies with a SAME PPON), coded mentions %s%%' % (med(sh_largest), med(sh_ppon), med(sh_coded)))
print('bodies where largest org < 50%:', sum(1 for s in sh_largest if s<0.5), 'of', len(sh_largest))
print('sum largest-org mentions today', sum(r['largest'] for r in rows), 'of', T, '=', round(100*sum(r['largest'] for r in rows)/T,1),'%')
print('sum coded', sum(r['coded'] for r in rows), 'sum ppon', sum(r['ppon_m'] for r in rows))
# non-SAME holders left alone
for side in ('xreg','xppon'):
    c=collections.Counter(); m=0; cod=0
    seen=set()
    for code,b in B.items():
        for o,r in b[side].items():
            if (o,code) in seen: continue
            seen.add((o,code)); c[r['final']]+=1
    print(side, dict(c))
dif=[r for r in adj if r['final']!='SAME' and r['side']=='registry']
print('non-SAME registry rows coded/uncoded/other', sum(r['coded'] for r in dif), sum(r['uncoded'] for r in dif), sum(r['other'] for r in dif))
# junk tails on SAME clusters
junk=collections.Counter(); junkm=0
for code,b in B.items():
    for o in b['reg']:
        for raw,n in mc.get(o,{}).items():
            t=coded_tail(raw)
            if t and len(t)==3 and t!=code and not any(ch.isdigit() and ch!='0' for ch in t):
                junk[t]+=n
print('3-char digit-rule-family tails on SAME clusters other than the body code:', dict(junk), sum(junk.values()))
# PPON holders excluded
for code,b in sorted(B.items()):
    for o,r in b['xppon'].items(): print('excluded PPON', code, o, r['final'], r['name'])
for r in rows:
    if r['kind']!='ppon': print(r['code'], r['kind'], 'members', r['n'], 'T', r['T'])
pp=[r for r in rows if r['kind'].startswith('ppon')]; an=[r for r in rows if r['kind']=='anchor']
print('PPON-survivor bodies', len(pp), 'mentions on canonical PPON orgs before', sum(r['ppon_m'] for r in pp), 'after', sum(r['T'] for r in pp), 'moved', sum(r['moved'] for r in pp))
print('anchor bodies', len(an), 'mentions', sum(r['T'] for r in an), 'moved', sum(r['moved'] for r in an))
ppn=[r for r in pp if not r['capped']]; ann=[r for r in an if not r['capped']]
print('now: PPON bodies', len(ppn), 'after', sum(r['T'] for r in ppn), '; anchor bodies', len(ann), sum(r['T'] for r in ann))
print('capped bodies:', [(r['code'], r['kind'], r['n']) for r in rows if r['capped']])
