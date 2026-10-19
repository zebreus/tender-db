import json, gzip, collections, importlib.util, sys
sys.argv=['x']
spec=importlib.util.spec_from_file_location('run_u2','run_u2.py'); U=importlib.util.module_from_spec(spec); spec.loader.exec_module(U)
R=U.load_rule()
recs={}
for line in gzip.open('decisions.jsonl.gz','rt'):
    d=json.loads(line); recs[d['id']]=d
br=collections.Counter(); ex=collections.defaultdict(list)
vb_kind=collections.Counter()
for t,rec in recs.items():
    v2=rec['decisions']['v2']; vb=v2['vouched_by']
    c0=next(c for c in rec['vouch_candidates'] if [c['field'],c['notice_id'],c['cents']]==[vb['field'],vb['notice_id'],vb['cents']])
    vb_kind[(c0['kind'], c0['field'])]+=1
    ests=[c for c in rec['vouch_candidates'] if c['kind']=='estimate']
    if not ests: continue
    x=R.tender_detail(t); contents={n:R.notice_content(n) for n in R.version_notice_ids(x)}
    inv,_=R.remap_inverse(x,contents); keys={k for k,_n,_c in rec['summed']}
    for c in ests:
        content=contents[c['notice_id']]
        is_cn,why=U.notice_cn(R,content)
        lots=[s['section_id'] for s in content['sections'] if s.get('kind') in U.SINGLE_LOT_KINDS]
        sk=None
        if len(lots)==1:
            back=[k for k,o in (inv.get(c['notice_id']) or {}).items() if o==lots[0]]
            sk=back[0] if back else lots[0]
        k=(is_cn, min(len(lots),3), None if len(lots)!=1 else (sk in keys))
        br[k]+=1
        if len(lots)==1: ex[k].append((t,c['notice_id'],c['field'],why,lots[0],sk,sorted(keys)[:4]))
print(vb_kind.most_common())
print(br.most_common())
for k,v in ex.items(): print(k, v[:4])
for t in (7698260,):
    rec=recs[t]; print(t, rec['summed'], [(c['field'],c['notice_id'],c['kind']) for c in rec['vouch_candidates']])
