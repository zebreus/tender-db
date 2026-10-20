import json,glob,os,collections as C
D='/home/user/tender-db/.scratch/tender-db/562-u5/second-half/out'
def L(n): return json.load(open(os.path.join(D,n+'.out')))
LO,HI,PLO,PHI=958848784,958904203,958853608,958904203
# sums
tot=0; byat=C.Counter(); seg=C.Counter(); maxrows=0
d5=C.Counter(); proj=C.Counter(); cur_minmax={}
for i in range(12):
    j=L(f'sum_{i}'); assert not j['truncated']; maxrows=max(maxrows,j['row_count'])
    for at,k,sl,op,n,lo,hi in j['rows']:
        tot+=n; byat[at]+=n
        if hi<=PLO: d5[(k,sl,op)]+=n
        elif lo>PLO: proj[(k,sl,op,at)]+=n
        else: print('STRADDLE',at,k,sl,op,lo,hi)
print('total',tot,'expect',HI-LO,'max sum rows',maxrows)
print('by changed_at',sorted(byat.items()))
print('d5',dict(d5),sum(d5.values()))
print('proj total',sum(proj.values()))
pk=C.Counter()
for (k,sl,op,at),n in proj.items(): pk[(k,sl,op)]+=n
for k in sorted(pk): print('  proj',k,pk[k])
# seqless
S=[];
for i in range(11):
    j=L(f'seqless_{i}'); assert not j['truncated'] and j['row_count']<1000,(i,j['row_count'])
    cols=j['columns']; S+= [dict(zip(cols,r)) for r in j['rows']]
T=[]
for i in range(11):
    j=L(f'ted_{i}'); assert not j['truncated'] and j['row_count']<2000
    cols=j['columns']; T+= [dict(zip(cols,r)) for r in j['rows']]
print('seqless rows',len(S),'distinct',len({r['tender_id'] for r in S}),'ats',{r['changed_at'] for r in S})
print('ted rows',len(T),'distinct',len({r['tender_id'] for r in T}))
flips=[r for r in T if r['created_at']<r['n_ingested_at'] and r['doe_before'] and not r['ted_before']]
print('flips',len(flips),len({r['tender_id'] for r in flips}),'seqs',C.Counter(r['seq'] for r in flips),'d5',C.Counter(r['d5_rows'] for r in flips))
ann=[r for r in flips if r['seqless_cursor']]
print('announced',len(ann),'gaps',min(r['seqless_cursor']-r['cursor'] for r in ann),max(r['seqless_cursor']-r['cursor'] for r in ann))
flipT={r['tender_id'] for r in flips}
Sd={r['tender_id']:r for r in S}
# check seqless_cursor actually exists in S
print('ann cursor in S', sum(1 for r in ann if r['seqless_cursor'] in {s['cursor'] for s in S}))
tedfirst=[r for r in T if r['ted_before']]
comint=[r for r in T if r['created_at']>=r['n_ingested_at']]
print('TED-first rows',len(tedfirst),'tenders',len({r['tender_id'] for r in tedfirst}),'with seqless in S', len({r['tender_id'] for r in tedfirst}&set(Sd)))
print('co-minted rows',len(comint),'with seqless in S', len({r['tender_id'] for r in comint}&set(Sd)), [(r['tender_id'],r['doe_before'],r['ted_before']) for r in comint])
B=[s for s in S if s['d5_rows']==0]; D3=[s for s in S if s['d5_rows']>0]
print('B',len(B),'D3',len(D3),'B==flips',{s['tender_id'] for s in B}==flipT)
print('B paired',C.Counter((s['paired_seq_rows'],s['paired_min_seq'],s['current_seq']) for s in B))
print('B head lots',sum(s['head_lots'] for s in B),'D3 head lots',sum(s['head_lots'] for s in D3))
print('D3 paired',C.Counter(s['paired_seq_rows']>0 for s in D3), 'd5_rows dist',C.Counter(s['d5_rows'] for s in D3))
print('lot check', sum(s['head_lots'] for s in B)+ (4609-897), pk[('lot',1,'changed')])
print('B kinds',C.Counter((s['source'],s['kind']) for s in B))
# non-flip T rows that have a seqless row on non-D5 tender
nf=[r for r in T if r['tender_id'] not in flipT and r['seqless_cursor']]
print('non-flip TED rows with seqless pairing',len(nf),C.Counter(r['d5_rows']>0 for r in nf))
# TED rows on D5 tenders
print('TED rows with d5>0',sum(1 for r in T if r['d5_rows']>0))
