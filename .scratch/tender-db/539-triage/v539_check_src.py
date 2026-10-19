import json, collections
D = '/home/user/tender-db/.scratch/tender-db/539-triage/'
rows=[]
for q in ['q24_src','q25_src','q26_src','q27_src']:
    j=json.load(open(D+q+'.out.json')); assert 'rows' in j and not j.get('truncated')
    rows += [dict(zip(j['columns'], r)) for r in j['rows']]
print('fetches', len(set(r['f'] for r in rows)), 'n', sum(r['n'] for r in rows))
c=collections.Counter()
for r in rows:
    de = r['profile'].startswith('eforms:eforms-de')
    if r['pb']=='b030' and de:
        c['de30']+=r['n']; c[('hp',r['hp'])]+=r['n']
        if r['hp']: c[('rb',r['rb'])]+=r['n']
    if r['pb']=='b001': c['g1']+=r['n']; c['g1_hp0']+=r['n']*(r['hp']==0)
    if r['pb']=='b002': c['g2']+=r['n']; c['g2_hp0']+=r['n']*(r['hp']==0)
print(c)
rel=[]
for q in ['q28_rel','q29_rel']:
    j=json.load(open(D+q+'.out.json')); rel += [dict(zip(j['columns'], r)) for r in j['rows']]
c=collections.Counter(); byp=collections.defaultdict(collections.Counter)
for r in rel:
    c['n']+=r['n']; c[r['rel']]+=r['n']; byp[r['profile']]['n']+=r['n']; byp[r['profile']][r['rel']]+=r['n']
    if r['pb']=='b030' and r['profile'].startswith('eforms:eforms-de'): c['de30_'+r['rel']]+=r['n']
print(c)
for p,v in byp.items(): print(p, dict(v), 'stamped share %.4f' % ((v['n']-v['nopub'])/v['n']))
print(sorted(set(r['f'] for r in rel)))
