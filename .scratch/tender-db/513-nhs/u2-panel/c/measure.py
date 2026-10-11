import json, sys, collections, statistics, os
D=sys.argv[1]
adj=json.load(open(D+'/adjudication-rows.json'))
lst=json.load(open(D+'/nhs-listing-2236.json'))['listing']
joined={j['code']:j for j in json.load(open(D+'/nhs-ods-joined.json'))}
# mention counts by org
mc=collections.defaultdict(dict)
for line in open(D+'/mention-counts-by-org.jsonl'):
    d=json.loads(line)
    for o,raw,n in d['rows']:
        mc[o][raw]=mc[o].get(raw,0)+n
print('orgs in mention-counts', len(mc))
TYPED={'WESTMIDLANDSAMBULANCESERVICE':'RYA','NHSCHESHIREANDMERSEYSIDEICB':'QYG'}
def body(code): return TYPED.get(code, code)
# per body
B=collections.defaultdict(lambda: {'same_reg':set(),'same_ppon':set(),'other_reg':set(),'other_ppon':set(),'rows':[]})
for r in adj:
    b=B[body(r['code'])]; b['rows'].append(r)
    if r['side']=='registry':
        (b['same_reg'] if r['final']=='SAME' else b['other_reg']).add(r['org_id'])
    else:
        (b['same_ppon'] if r['final']=='SAME' else b['other_ppon']).add(r['org_id'])
print('bodies', len(B))
# org -> bodies where SAME
same_of=collections.defaultdict(set)
for code,b in B.items():
    for o in b['same_reg']|b['same_ppon']: same_of[o].add(code)
multi=[(o,c) for o,c in same_of.items() if len(c)>1]
print('orgs SAME for 2+ bodies:', multi)
# org classed SAME in one body and non-SAME in another
nonsame_of=collections.defaultdict(set)
for code,b in B.items():
    for o in b['other_reg']|b['other_ppon']: nonsame_of[o].add(code)
both=[(o,same_of[o],nonsame_of[o]) for o in same_of if o in nonsame_of]
print('orgs SAME somewhere and non-SAME elsewhere:', both)
def total(o):
    return sum(mc.get(o,{}).values())
def coded_for(o, code):
    # coded mentions with raw GB-NHS-<code> (case-insensitive), as adjudication counted
    return None
# today count
today=sum(len(b['same_reg']|b['same_ppon']) for b in B.values())
print('today SAME orgs summed per body:', today)
# distinct
allsame=set().union(*[b['same_reg']|b['same_ppon'] for b in B.values()])
print('distinct SAME orgs:', len(allsame))
# after C: per body one org if any SAME
after=sum(1 for b in B.values() if b['same_reg']|b['same_ppon'])
print('after C (one per body with SAME holders):', after)
# bodies by PPON situation
cat=collections.Counter()
ppon_same_count=collections.Counter()
for code,b in B.items():
    ns=len(b['same_ppon']); ppon_same_count[ns]+=1
    if not (b['same_reg']|b['same_ppon']): cat['no SAME']+=1
    elif ns>=1 and b['same_reg']: cat['reg+ppon SAME']+=1
    elif ns>=1: cat['ppon only SAME']+=1
    else: cat['reg only SAME (no SAME ppon)']+=1
print(cat, 'same ppon per body', ppon_same_count)
# members per body and losers
mem=[len(b['same_reg']|b['same_ppon']) for b in B.values() if (b['same_reg']|b['same_ppon'])]
print('members total', sum(mem), 'losers', sum(m-1 for m in mem), 'bodies with >=2 members', sum(1 for m in mem if m>=2))
print('member size dist', collections.Counter(mem))
# mentions unified (adjudication counts)
def adjcount(b):
    reg=0; seen=set(); coded=0; unc=0; oth=0; pp=0
    for r in b['rows']:
        if r['final']!='SAME' or r['org_id'] in seen: continue
        seen.add(r['org_id'])
        if r['side']=='registry':
            coded+=r['coded']; unc+=r['uncoded']; oth+=r['other']
        else:
            pp+=r['coded']+r['uncoded']+r['other']
    return coded,unc,oth,pp
tc=tu=to=tp=0
for code,b in B.items():
    c,u,o,p=adjcount(b); tc+=c; tu+=u; to+=o; tp+=p
print('SAME (dedup per body): coded',tc,'uncoded',tu,'other',to,'ppon',tp,'sum',tc+tu+to+tp)
# check vs mention-counts totals
tot_mc=sum(total(o) for o in allsame)
print('mention-counts total over distinct SAME orgs', tot_mc, 'orgs missing from mc', [o for o in allsame if o not in mc][:10], len([o for o in allsame if o not in mc]))
